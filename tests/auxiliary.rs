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

pub mod util;

use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs
                           // use assert_cmd::cmd::Command; // Run programs - it seems to be equal to "use assert_cmd::prelude::* + use std::process::Command"
use std::path::Path;

#[tokio::test]
async fn test_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut dynein_cmd = Command::cargo_bin("dy")?;
    let cmd = dynein_cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("dynein is a command line tool"));
    Ok(())
}

#[tokio::test]
async fn test_custom_config_location() -> Result<(), Box<dyn std::error::Error>> {
    let tm = util::setup().await?;

    let dummy_dir = "./tests/dummy_dir";
    let config_dir = dummy_dir.to_string() + "/.dynein";

    // cleanup config folder in case it was already there
    util::cleanup_config(dummy_dir).await.ok();
    check_file_exist(&config_dir, false);

    // run any dy command to generate default config
    let mut c = tm.command()?;
    c.env("DYNEIN_CONFIG_DIR", dummy_dir).assert();

    // check config folder created at our desired location
    check_file_exist(&config_dir, true);

    // cleanup config folder
    util::cleanup_config(dummy_dir).await.ok();

    Ok(())
}

fn check_file_exist(dir: &str, exist: bool) {
    assert_eq!(Path::new(&dir).exists(), exist);
    assert_eq!(Path::new(&format!("{}/config.yml", dir)).exists(), exist);
    assert_eq!(Path::new(&format!("{}/cache.yml", dir)).exists(), exist);
}
