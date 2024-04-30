/*
 * Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License").
 * You may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use assert_cmd::prelude::*; // Add methods on commands
use std::env;
use std::process::Command; // Run programs
                           // use assert_cmd::cmd::Command; // Run programs - it seems to be equal to "use assert_cmd::prelude::* + use std::process::Command"

use once_cell::sync::Lazy;
use rand::{distributions::Alphanumeric, Rng};
use regex::bytes::Regex;
use rusoto_core::Region;
use rusoto_dynamodb::{DynamoDb, DynamoDbClient};
use serde_json::Value;
use std::io::{self, Write}; // Used when check results by printing to stdout
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;
use tokio::time::sleep;

// We use std::sync::Mutex instead of tokio::sync::Mutex, because mutex must be poisoned after setup failure.
static SETUP_LOCK: Lazy<RwLock<()>> = Lazy::new(|| RwLock::new(()));
static SETUP_DOCKER_RUN_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub struct TestManager<'a> {
    port: i32,
    temporary_tables: Vec<String>,
    config_dirs: Vec<PathBuf>,
    default_config_dir: PathBuf,
    _read_lock: Option<RwLockReadGuard<'a, ()>>,
    _write_lock: Option<RwLockWriteGuard<'a, ()>>,
}

impl<'a> TestManager<'a> {
    fn create_config_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let dir_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        println!("create config folder: {}", dir_name);

        let mut conf_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        conf_dir.push(dir_name);

        // Create .dynein directory
        let mut dir = conf_dir.clone();
        dir.push(".dynein");
        std::fs::create_dir_all(&dir)?;

        Ok(conf_dir)
    }

    pub fn default_config_dir(&self) -> PathBuf {
        let mut path = self.default_config_dir.to_path_buf();
        path.push(".dynein");
        path
    }

    pub fn command(&self) -> Result<Command, Box<dyn std::error::Error>> {
        let mut c = Command::cargo_bin("dy")?;
        c.env("DYNEIN_CONFIG_DIR", &self.default_config_dir);
        Ok(c)
    }

    pub fn command_with_envs<S>(&mut self, config: S) -> Result<Command, Box<dyn std::error::Error>>
    where
        S: AsRef<[u8]>,
    {
        let conf_dir = Self::create_config_dir()?;

        let mut conf_path = conf_dir.clone();
        conf_path.push(".dynein");
        conf_path.push("config.yml");
        std::fs::write(conf_path, config)?;

        self.config_dirs.push(conf_dir.to_owned());

        let cmd = self.command().map(|mut x| {
            x.env("DYNEIN_CONFIG_DIR", conf_dir);
            x
        });

        cmd
    }

    /// Create temporary table which is deleted when the struct is dropped.
    /// You don't need to delete the table manually.
    pub async fn create_temporary_table(
        &mut self,
        pk: &'static str,
        sk: Option<&'static str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let table_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        println!("create temporary table: {}", table_name);

        let mut c = self.command()?;
        let port = self.port.to_string();
        let mut args = vec![
            "--region",
            "local",
            "--port",
            &port,
            "admin",
            "create",
            "table",
            &table_name,
            "--keys",
        ];
        let mut keys = vec![pk];
        if let Some(sk) = sk {
            keys.push(sk);
        }

        args.extend(keys);
        c.args(args).assert().success();

        self.temporary_tables.push(table_name.clone());

        Ok(table_name)
    }

    /// Create temporary table with items via `create_temporary_table`.
    pub async fn create_temporary_table_with_items<ItemIter>(
        &mut self,
        pk: &'static str,
        sk: Option<&'static str>,
        items: ItemIter,
    ) -> Result<String, Box<dyn std::error::Error>>
    where
        ItemIter: IntoIterator<Item = TemporaryItem>,
    {
        let table_name = self.create_temporary_table(pk, sk).await?;

        for ti in items {
            let mut c = self.command()?;
            let mut args = vec!["--region", "local", "--table", &table_name, "put"];
            args.extend(ti.keys());
            if let Some(item) = ti.item {
                args.extend(vec!["--item", item]);
            }

            c.args(args).assert().success();
        }

        Ok(table_name)
    }

    /// Delete table manually.
    pub fn cleanup<I, S>(&self, tables: I) -> Result<(), Box<dyn std::error::Error>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for table in tables {
            let mut c = self.command()?;
            let cmd = c.args([
                "--region",
                "local",
                "--port",
                &self.port.to_string(),
                "admin",
                "delete",
                "table",
                "--yes",
                table.as_ref(),
            ]);
            cmd.assert().success();
        }

        Ok(())
    }

    pub fn cleanup_config<I, P>(&self, config_dirs: I) -> Result<(), Box<dyn std::error::Error>>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        for dir in config_dirs {
            std::fs::remove_dir_all(dir)?;
        }

        Ok(())
    }
}

impl<'a> Drop for TestManager<'a> {
    fn drop(&mut self) {
        println!("delete temporary tables: {:?}", self.temporary_tables);
        let _ = self.cleanup(&self.temporary_tables);
        println!(
            "delete config directories: {:?} and {:?}",
            self.config_dirs, self.default_config_dir
        );
        let _ = self.cleanup_config([&self.default_config_dir]);
        let _ = self.cleanup_config(self.config_dirs.iter());
    }
}

pub async fn setup() -> Result<TestManager<'static>, Box<dyn std::error::Error>> {
    setup_with_port(8000).await
}

pub async fn setup_with_port(
    port: i32,
) -> Result<TestManager<'static>, Box<dyn std::error::Error>> {
    let lock = SETUP_LOCK.read().unwrap();
    setup_container(port).await?;

    Ok(TestManager {
        port,
        temporary_tables: vec![],
        config_dirs: vec![],
        default_config_dir: TestManager::create_config_dir()?,
        _read_lock: Some(lock),
        _write_lock: None,
    })
}

pub async fn setup_with_lock() -> Result<TestManager<'static>, Box<dyn std::error::Error>> {
    let lock = SETUP_LOCK.write().unwrap();
    setup_container(8000).await?;

    Ok(TestManager {
        port: 8000,
        temporary_tables: vec![],
        config_dirs: vec![],
        default_config_dir: TestManager::create_config_dir()?,
        _read_lock: None,
        _write_lock: Some(lock),
    })
}

/// Check existence of docker process for dynamodb-local
fn check_dynamodb_local_running(port: u16) -> bool {
    let mut docker_for_check = Command::new("docker");

    let check_cmd = docker_for_check.args([
        "ps",
        "--format",
        "{{.Ports}}",
        "--filter",
        "ancestor=amazon/dynamodb-local",
    ]);
    let check_out = check_cmd.output().expect("failed to execute check cmd");
    let reg_str = format!(r"(?m):{}->\d+/tcp$", port);
    let port_re = Regex::new(&reg_str).unwrap();
    if !check_out.status.success() {
        panic!("failed to execute docker ps command")
    }
    port_re.is_match(&check_out.stdout)
}

/// Integration tests would go with DynamoDB Local, so before running them setup() starts up DynamoDB Local with Docker.
/// FYI: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/DynamoDBLocal.html
///      https://hub.docker.com/r/amazon/dynamodb-local
async fn setup_container(port: i32) -> Result<(), Box<dyn std::error::Error>> {
    // Stop docker setup if DYNEIN_TEST_NO_DOCKER_SETUP=true.
    // This configuration is useful for skipping the docker setup in the GitHub CI environment.
    // Also, it reduces test time because of skipping of docker checks.
    // If you use this, you must ensure that docker containers are running for tests.
    // See https://github.com/awslabs/dynein/pull/59 for detail.
    let stop_setup: bool = env::var("DYNEIN_TEST_NO_DOCKER_SETUP")
        .unwrap_or("false".to_string())
        .to_lowercase()
        .parse()
        .expect("DYNEIN_TEST_NO_DOCKER_SETUP expects true or false");
    if stop_setup {
        return Ok(());
    }

    // Check the current process at first to allow multiple threads to run tests concurrently.
    // This is for performance optimization on Windows and Mac OS.
    // See https://github.com/awslabs/dynein/pull/28#issuecomment-972880324 for detail.
    if check_dynamodb_local_running(port as u16) {
        return Ok(());
    };

    // To avoid unnecessary docker container creation, setup docker sequentially
    let _lock = SETUP_DOCKER_RUN_MUTEX.lock();

    // Recheck whether another thread already started the dynamodb-local
    if check_dynamodb_local_running(port as u16) {
        return Ok(());
    }

    let mut docker_for_run = Command::new("docker");
    let docker_run = docker_for_run.args([
        "run",
        "-p",
        &format!("{}:8000", port),
        "-d",
        "amazon/dynamodb-local",
    ]);
    let output = docker_run
        .output()
        .expect("failed to running Docker image amazon/dynamodb-local in setup().");
    if !output.status.success() {
        panic!("failed to execute docker run command")
    }
    print!("DynamoDB Local is up as a container: ");
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    // Wait dynamodb-local
    let health_check_url = format!("http://localhost:{}", port);
    let ddb = DynamoDbClient::new(Region::Custom {
        name: "local".to_owned(),
        endpoint: health_check_url,
    });
    let max_retries = 5;
    let mut attempts = 0;
    loop {
        match ddb.list_tables(Default::default()).await {
            Ok(_result) => {
                println!("ListTables API succeeded.");
                break;
            }
            Err(e) => {
                println!("Couldn't connect: {} \n Retry after 3 seconds.", e);
                sleep(Duration::from_secs(3)).await;

                attempts += 1;
                if attempts >= max_retries {
                    panic!("Failed to connect after {} attempts.", max_retries);
                }
            }
        }
    }

    Ok(())
}

pub struct TemporaryItem {
    pval: &'static str,
    sval: Option<&'static str>,
    item: Option<&'static str>,
}

impl TemporaryItem {
    pub fn new(
        pval: &'static str,
        sval: Option<&'static str>,
        item: Option<&'static str>,
    ) -> TemporaryItem {
        TemporaryItem { pval, sval, item }
    }

    pub fn keys(&self) -> Vec<&'static str> {
        let mut result = vec![self.pval];
        if let Some(sval) = self.sval {
            result.push(sval);
        }

        result
    }
}

pub fn check_dynein_files_existence(dir: &str, exist: bool) {
    // assert_eq!(Path::new(&format!("{}/config.yml", dir)).exists(), exist);
    let mut config = PathBuf::from(dir);
    config.push("config.yml");
    assert_eq!(config.exists(), exist);
    // assert_eq!(Path::new(&format!("{}/cache.yml", dir)).exists(), exist);
    let mut cache = PathBuf::from(dir);
    cache.push("cache.yml");
    assert_eq!(cache.exists(), exist);
}

pub async fn cleanup_config(dummy_dir: &str) -> io::Result<()> {
    use std::fs::remove_dir_all;

    remove_dir_all(dummy_dir)
}

pub fn assert_eq_cmd_json(cmd: &mut Command, expected: &str) {
    cmd.assert().success();
    let stdout = cmd.output().unwrap().stdout;
    let output = String::from_utf8(stdout).unwrap();

    assert_eq_json(&output, expected)
}

pub fn assert_eq_json(content: &str, expected: &str) {
    assert_eq!(
        content.parse::<serde_json::Value>().unwrap(),
        expected.parse::<serde_json::Value>().unwrap(),
    )
}

/// Similar to assert_eq_json, but this function ignores both array order and map key order.
/// For instance, it considers `{"a": [2, 1], "b": "value"}` as equal to `{"b": "value", "a": [1, 2]}`.
pub fn assert_eq_json_ignore_order(cmd: &mut Command, expected: &str) {
    cmd.assert().success();
    let stdout = cmd.output().unwrap().stdout;
    let output = String::from_utf8(stdout).unwrap();

    let mut output_json = output.parse::<serde_json::Value>().unwrap();
    let mut expected_json = expected.parse::<serde_json::Value>().unwrap();

    sort_json_array(&mut output_json);
    sort_json_array(&mut expected_json);

    assert_eq!(output_json, expected_json);
}

fn sort_json_array(value: &mut Value) {
    match value {
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                sort_json_array(v);
            }
            arr.sort_by_key(|a| a.to_string());
        }
        Value::Object(obj) => {
            for v in obj.values_mut() {
                sort_json_array(v);
            }
        }
        _ => {}
    }
}

pub fn assert_eq_yaml(cmd: &mut Command, expected: &str) {
    cmd.assert().success();
    let stdout = cmd.output().unwrap().stdout;
    let output = String::from_utf8(stdout).unwrap();

    assert_eq!(
        serde_yaml::from_str::<serde_yaml::Value>(&output).unwrap(),
        serde_yaml::from_str::<serde_yaml::Value>(expected).unwrap(),
    )
}
