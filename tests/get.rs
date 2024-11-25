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
    let tm = util::setup().await?;
    let mut c = tm.command()?;
    let cmd = c.args([
        "--region",
        "local",
        "--table",
        "dummy-table-doesnt-exist",
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
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;

    let mut c = tm.command()?;
    let cmd = c.args(["--region", "local", "--table", &table_name, "get", "42"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No item found."));
    Ok(())
}

#[tokio::test]
async fn test_get_item() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;

    for action in ["get", "g"] {
        let table_name = prepare_table_with_item(&mut tm).await?;
        let mut c = tm.command()?;
        let cmd = c.args(["--region", "local", "--table", &table_name, action, "42"]);
        util::assert_eq_cmd_json(
            cmd,
            r#"{
          "flag": true,
          "pk": "42"
        }"#,
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_get_item_output_json() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = prepare_table_with_item(&mut tm).await?;

    let mut c = tm.command()?;
    let cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "get",
        "42",
        "-o",
        "json",
    ]);
    util::assert_eq_cmd_json(
        cmd,
        r#"{
          "flag": true,
          "pk": "42"
        }"#,
    );

    Ok(())
}

#[tokio::test]
async fn test_get_item_output_yaml() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = prepare_table_with_item(&mut tm).await?;

    let mut c = tm.command()?;
    let cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "get",
        "42",
        "-o",
        "yaml",
    ]);
    util::assert_eq_cmd_yaml(
        cmd,
        r#"---
flag: true
pk: "42"
"#,
    );

    Ok(())
}

#[tokio::test]
async fn test_get_item_output_raw() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = prepare_table_with_item(&mut tm).await?;

    let mut c = tm.command()?;
    let cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "get",
        "42",
        "-o",
        "raw",
    ]);
    util::assert_eq_cmd_json(
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

    Ok(())
}

async fn prepare_table_with_item<'a>(
    tm: &mut util::TestManager<'a>,
) -> Result<String, Box<dyn std::error::Error>> {
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            None,
            [util::TemporaryItem::new(
                "42",
                None,
                Some("{\"flag\": true}"),
            )],
        )
        .await?;

    Ok(table_name)
}
