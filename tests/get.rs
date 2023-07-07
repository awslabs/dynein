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
async fn test_get_non_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut c = util::setup().await?;
    let cmd = c.args(&[
        "--region",
        "local",
        "--table",
        "dummy-table-doent-exist",
        "get",
        "42",
    ]);
    cmd.assert().failure().stderr(predicate::str::contains(
        // The error message is different between DynamoDB local and real service.
        // It should be "Requested resource not found: Table: table not found" actually.
        "Cannot do operations on a non-existent table",
    ));
    Ok(())
}

#[tokio::test]
async fn test_get_non_existent_item() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = util::create_temporary_table(vec!["pk"]).await?;

    let mut c = util::setup().await?;
    let cmd = c.args(&["--region", "local", "--table", &table_name, "get", "42"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No item found."));
    util::cleanup(vec![&table_name]).await
}

#[tokio::test]
async fn test_get_item() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = prepare_table_with_item().await?;

    let mut c = util::setup().await?;
    let cmd = c.args(&["--region", "local", "--table", &table_name, "get", "42"]);
    util::assert_eq_json(
        cmd,
        r#"{
          "flag": true,
          "pk": "42"
        }"#,
    );

    util::cleanup(vec![&table_name]).await
}

#[tokio::test]
async fn test_get_item_output_json() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = prepare_table_with_item().await?;

    let mut c = util::setup().await?;
    let cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "get",
        "42",
        "-o",
        "json",
    ]);
    util::assert_eq_json(
        cmd,
        r#"{
          "flag": true,
          "pk": "42"
        }"#,
    );

    util::cleanup(vec![&table_name]).await
}

#[tokio::test]
async fn test_get_item_output_yaml() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = prepare_table_with_item().await?;

    let mut c = util::setup().await?;
    let cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "get",
        "42",
        "-o",
        "yaml",
    ]);
    util::assert_eq_yaml(
        cmd,
        r#"---
flag: true
pk: "42"
"#,
    );

    util::cleanup(vec![&table_name]).await
}

#[tokio::test]
async fn test_get_item_output_raw() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = prepare_table_with_item().await?;

    let mut c = util::setup().await?;
    let cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "get",
        "42",
        "-o",
        "raw",
    ]);
    util::assert_eq_json(
        cmd,
        r#"{
          "flag": {
            "BOOL": true
          },
          "pk": {
            "S": "42"
          }
        }"#,
    );

    util::cleanup(vec![&table_name]).await
}

async fn prepare_table_with_item() -> Result<String, Box<dyn std::error::Error>> {
    let table_name = util::create_temporary_table(vec!["pk"]).await?;

    let mut c = util::setup().await?;
    c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "put",
        "42",
        "-i",
        "{\"flag\": true}",
    ])
    .assert()
    .success();

    Ok(table_name)
}
