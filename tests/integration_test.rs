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
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs
                           // use assert_cmd::cmd::Command; // Run programs - it seems to be equal to "use assert_cmd::prelude::* + use std::process::Command"

use once_cell::sync::Lazy;
use regex::bytes::Regex;
use rusoto_core::Region;
use rusoto_dynamodb::{DynamoDb, DynamoDbClient};
use std::fs::File;
use std::io::{self, Write}; // Used when check results by printing to stdout
use std::sync::Mutex;
use std::time::Duration;
use tokio::time::delay_for;

use tempfile::Builder;

/// Integration tests would go with DynamoDB Local, so before running them setup() starts up DynamoDB Local with Docker.
/// FYI: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/DynamoDBLocal.html
///      https://hub.docker.com/r/amazon/dynamodb-local
async fn setup() -> Result</* std::process::Command */ Command, Box<dyn std::error::Error>> {
    setup_with_port(8000).await
}

// We use std::sync::Mutex instead of tokio::sync::Mutex, because mutex must be poisoned after setup failure.
static SETUP_MUTEX: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(0));

/// Check existence of docker process for dynamodb-local
async fn check_dynamodb_local_running(port: u16) -> bool {
    let mut docker_for_check = Command::new("docker");

    let check_cmd = docker_for_check.args(&[
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
    if port_re.is_match(&check_out.stdout) {
        true
    } else {
        false
    }
}

async fn setup_with_port(port: i32) -> Result<Command, Box<dyn std::error::Error>> {
    // Check the current process at first to allow multiple threads to run tests concurrently
    if check_dynamodb_local_running(port as u16).await {
        return Ok(Command::cargo_bin("dy")?);
    };

    // To avoid unnecessary docker container creation, setup docker sequentially
    let _lock = SETUP_MUTEX.lock();

    // Recheck whether another thread already started the dynamodb-local
    if check_dynamodb_local_running(port as u16).await {
        return Ok(Command::cargo_bin("dy")?);
    }

    let mut docker_for_run = Command::new("docker");
    let docker_run = docker_for_run.args(&[
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
    loop {
        if let Ok(_result) = ddb.list_tables(Default::default()).await {
            println!("ListTables API succeeded.");
            break;
        } else {
            println!("Couldn't connect. Retry after 3 seconds.");
            delay_for(Duration::from_secs(3)).await;
        }
    }

    Ok(Command::cargo_bin("dy")?)
}

async fn cleanup(tables: Vec<&str>) -> Result<(), Box<dyn std::error::Error>> {
    for table in tables {
        let mut dynein_cmd = setup().await?;
        let cmd = dynein_cmd.args(&[
            "--region", "local", "admin", "delete", "table", "--yes", table,
        ]);
        cmd.assert().success();
    }
    Ok(())
}

async fn cleanup_with_port(tables: Vec<&str>, port: i32) -> Result<(), Box<dyn std::error::Error>> {
    for table in tables {
        let mut dynein_cmd = setup().await?;
        let cmd = dynein_cmd.args(&[
            "--region",
            "local",
            "--port",
            &format!("{}", port),
            "admin",
            "delete",
            "table",
            "--yes",
            table,
        ]);
        cmd.assert().success();
    }
    Ok(())
}

#[tokio::test]
async fn test_help() -> Result<(), Box<dyn std::error::Error>> {
    setup().await?;
    let mut dynein_cmd = Command::cargo_bin("dy")?;
    let cmd = dynein_cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("dynein is a command line tool"));
    Ok(())
}

#[tokio::test]
async fn test_create_table() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_create_table";

    // $ dy admin create table <table_name> --keys pk
    let mut c = setup().await?;
    let create_cmd = c.args(&[
        "--region", "local", "admin", "create", "table", table_name, "--keys", "pk",
    ]);
    create_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            &table_name
        )));

    // $ dy admin desc <table_name>
    let mut c = setup().await?;
    let desc_cmd = c.args(&["--region", "local", "desc", table_name]);
    desc_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            &table_name
        )));

    Ok(cleanup(vec![table_name]).await?)
}

#[tokio::test]
async fn test_create_table_with_region_local_and_port_number_options(
) -> Result<(), Box<dyn std::error::Error>> {
    let port = 8001;
    let table_name = "table--test_create_table_with_region_local_and_port_number_options";

    // $ dy admin create table <table_name> --keys pk
    let mut c = setup_with_port(port).await?;
    let create_cmd = c.args(&[
        "--region",
        "local",
        "--port",
        &format!("{}", port),
        "admin",
        "create",
        "table",
        table_name,
        "--keys",
        "pk",
    ]);
    create_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            &table_name
        )));

    // $ dy admin desc <table_name>
    let mut c = setup_with_port(port).await?;
    let desc_cmd = c.args(&[
        "--region",
        "local",
        "--port",
        &format!("{}", port),
        "desc",
        table_name,
    ]);
    desc_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            &table_name
        )));

    Ok(cleanup_with_port(vec![table_name], port).await?)
}

#[tokio::test]
async fn test_scan_non_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut c = setup().await?;
    let cmd = c.args(&[
        "--region",
        "local",
        "--table",
        "dummy-table-doent-exist",
        "scan",
    ]);
    cmd.assert().failure().stderr(predicate::str::contains(
        "Cannot do operations on a non-existent table",
    ));
    Ok(())
}

#[tokio::test]
async fn test_scan_blank_table() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_scan_blank_table";

    let mut c = setup().await?;
    c.args(&[
        "--region", "local", "admin", "create", "table", table_name, "--keys", "pk",
    ])
    .output()?;
    let mut c = setup().await?;
    let scan_cmd = c.args(&["--region", "local", "--table", table_name, "scan"]);
    scan_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("No item to show"));

    Ok(cleanup(vec![table_name]).await?)
}

#[tokio::test]
async fn test_simple_scan() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_simple_scan";

    let mut c = setup().await?;
    c.args(&[
        "--region", "local", "admin", "create", "table", table_name, "--keys", "pk",
    ])
    .output()?;
    let mut c = setup().await?;
    c.args(&["--region", "local", "--table", table_name, "put", "abc"])
        .output()?;

    let mut c = setup().await?;
    let scan_cmd = c.args(&["--region", "local", "--table", table_name, "scan"]);
    scan_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("pk  attributes\nabc"));

    Ok(cleanup(vec![table_name]).await?)
}

async fn prepare_pk_sk_table(table_name: &&str) -> Result<(), Box<dyn std::error::Error>> {
    let mut c = setup().await?;
    c.args(&[
        "--region", "local", "admin", "create", "table", table_name, "--keys", "pk,S", "sk,N",
    ])
    .output()?;
    let mut c = setup().await?;
    c.args(&[
        "--region", "local", "--table", table_name, "put", "abc", "1",
    ])
    .output()?;
    let mut c = setup().await?;
    c.args(&[
        "--region", "local", "--table", table_name, "put", "abc", "2",
    ])
    .output()?;
    Ok(())
}

#[tokio::test]
async fn test_simple_query() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_simple_query";

    prepare_pk_sk_table(&table_name).await?;
    let mut c = setup().await?;
    let query_cmd = c.args(&["--region", "local", "--table", table_name, "query", "abc"]);
    query_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "pk   sk  attributes\nabc  1\nabc  2",
        ));

    Ok(cleanup(vec![table_name]).await?)
}

#[tokio::test]
async fn test_simple_desc_query() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_desc_simple_query";

    prepare_pk_sk_table(&table_name).await?;
    let mut c = setup().await?;
    let query_cmd = c.args(&[
        "--region", "local", "--table", table_name, "query", "abc", "-d",
    ]);
    query_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "pk   sk  attributes\nabc  2\nabc  1",
        ));

    Ok(cleanup(vec![table_name]).await?)
}

#[tokio::test]
async fn test_batch_write() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_batch_write";

    let mut c = setup().await?;
    c.args(&[
        "--region", "local", "admin", "create", "table", table_name, "--keys", "pk",
    ])
    .output()?;

    let tmpdir = Builder::new().tempdir()?; // defining stand alone variable here as tempfile::tempdir creates directory and deletes it when the destructor is run.
    let batch_input_file_path = tmpdir.path().join("test_batch_write.json");
    let mut f = File::create(tmpdir.path().join("test_batch_write.json"))?;
    f.write_all(b"
    {
        \"table--test_batch_write\": [
            {
                \"PutRequest\": {
                    \"Item\": {
                        \"pk\": { \"S\": \"ichi\" },
                        \"ISBN\": { \"S\": \"111-1111111111\" },
                        \"Price\": { \"N\": \"2\" },
                        \"Dimensions\": { \"SS\": [\"Giraffe\", \"Hippo\" ,\"Zebra\"] },
                        \"PageCount\": { \"NS\": [\"42.2\", \"-19\", \"7.5\", \"3.14\"] },
                        \"InPublication\": { \"BOOL\": false },
                        \"Nothing\": { \"NULL\": true },
                        \"Authors\": {
                            \"L\": [
                                { \"S\": \"Author1\" },
                                { \"S\": \"Author2\" },
                                { \"N\": \"42\" }
                            ]
                        },
                        \"Details\": {
                            \"M\": {
                                \"Name\": { \"S\": \"Joe\" },
                                \"Age\":  { \"N\": \"35\" },
                                \"Misc\": {
                                    \"M\": {
                                        \"hope\": { \"BOOL\": true },
                                        \"dream\": { \"L\": [ { \"N\": \"35\" }, { \"NULL\": true } ] }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        ]
    }
    ")?;
    let mut c = setup().await?;
    c.args(&[
        "--region",
        "local",
        "--table",
        table_name,
        "bwrite",
        "--input",
        &batch_input_file_path.to_str().unwrap(),
    ])
    .output()?;

    let mut c = setup().await?;
    let scan_cmd = c.args(&["--region", "local", "--table", table_name, "scan"]);
    scan_cmd
        .assert()
        .success()
        .stdout(predicate::str::is_match("pk *attributes\nichi").unwrap());

    /*
    get output should looks like:
        $ dy --region local -t table--test_batch_write get ichi
        {
          "Dimensions": [
            "Giraffe",
            "Hippo",
            "Zebra"
          ],
          "PageCount": [
            -19.0,
            3.14,
            7.5,
            42.2
          ],
          "Authors": [
            "Author1",
            "Author2",
            42
          ],
          "InPublication": false,
          "Nothing": null,
          "Price": 2,
          "pk": "ichi",
          "Details": {
            "Age": 35,
            "Misc": {
              "dream": [
                35,
                null
              ],
              "hope": true
            },
            "Name": "Joe"
          },
          "ISBN": "111-1111111111"
        }
    */
    let mut c = setup().await?;
    let get_cmd = c.args(&["--region", "local", "--table", table_name, "get", "ichi"]);
    let output = get_cmd.output()?.stdout;

    // more verification would be nice
    assert_eq!(
        true,
        predicate::str::is_match("\"Dimensions\":")?.eval(String::from_utf8(output)?.as_str())
    );

    Ok(cleanup(vec![table_name]).await?)
}

#[tokio::test]
async fn test_shell_mode() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{Seek, SeekFrom};

    let table_name = "table--test_shell_mode";

    // $ dy admin create table <table_name> --keys pk
    let mut c = setup().await?;
    let shell_session = c.args(&["--region", "local", "--shell"]);
    let mut tmpfile = Builder::new().tempfile()?.into_file();
    writeln!(tmpfile, "admin create table {} --keys pk", table_name)?;
    writeln!(tmpfile, "use {}", table_name)?;
    writeln!(tmpfile, "desc")?;
    tmpfile.seek(SeekFrom::Start(0))?;
    shell_session
        .stdin(tmpfile)
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            &table_name
        )));

    Ok(cleanup(vec![table_name]).await?)
}
