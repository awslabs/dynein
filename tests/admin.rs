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

#[tokio::test]
async fn test_create_table() -> Result<(), Box<dyn std::error::Error>> {
    let tm = util::setup().await?;
    let table_name = "table--test_create_table";

    let mut c = tm.command()?;
    let create_cmd = c.args([
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
    let mut c = tm.command()?;
    let desc_cmd = c.args(["--region", "local", "desc", table_name]);
    desc_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            &table_name
        )));

    tm.cleanup(vec![table_name])
}

#[tokio::test]
async fn test_create_table_with_region_local_and_port_number_options(
) -> Result<(), Box<dyn std::error::Error>> {
    let port = 8001;
    let table_name = "table--test_create_table_with_region_local_and_port_number_options";
    let tm = util::setup_with_port(port).await?;

    let mut c = tm.command()?;
    let create_cmd = c.args([
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
    let mut c = tm.command()?;
    let desc_cmd = c.args([
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

    tm.cleanup(vec![table_name])
}
