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
async fn test_scan_non_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut c = util::setup().await?;
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
    let table_name = util::create_temporary_table(vec!["pk"]).await?;

    let mut c = util::setup().await?;
    let scan_cmd = c.args(&["--region", "local", "--table", &table_name, "scan"]);
    scan_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("No item to show"));

    util::cleanup(vec![&table_name]).await
}

#[tokio::test]
async fn test_simple_scan() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = util::create_temporary_table(vec!["pk"]).await?;

    let mut c = util::setup().await?;
    c.args(&["--region", "local", "--table", &table_name, "put", "abc"])
        .output()?;

    let mut c = util::setup().await?;
    let scan_cmd = c.args(&["--region", "local", "--table", &table_name, "scan"]);
    scan_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("pk  attributes\nabc"));

    util::cleanup(vec![&table_name]).await
}
