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
use tempfile::tempdir;

#[tokio::test]
async fn test_export_non_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let tm = util::setup().await?;
    let mut c = tm.command()?;
    let cmd = c.args(&[
        "--region",
        "local",
        "--table",
        "dummy-table-doent-exist",
        "export",
        "--output-file",
        "a",
    ]);
    cmd.assert().failure().stderr(predicate::str::contains(
        // The error message is different between DynamoDB local and real service.
        // It should be "Requested resource not found: Table: table not found" actually.
        "Cannot do operations on a non-existent table",
    ));
    Ok(())
}

#[tokio::test]
async fn test_export_empty_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;

    let base_dir = tempdir()?;
    let temp_path = base_dir.path().join(&table_name);

    let mut c = tm.command()?;
    let cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "export",
        "--output-file",
        temp_path.to_str().unwrap(),
    ]);
    // TODO: this behavior should be fixed by the issue
    // https://github.com/awslabs/dynein/issues/152
    cmd.assert().failure().stderr(predicate::str::contains(
        "thread 'main' panicked at src/transfer.rs:481:20:\nattempt to subtract with overflow",
    ));
    Ok(())
}

#[tokio::test]
async fn test_export_with_items() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,N"),
            vec![
                util::TemporaryItem::new("abc", Some("1"), None),
                util::TemporaryItem::new("abc", Some("2"), Some(r#"{"a": 1, "b": 2}"#)),
                util::TemporaryItem::new("def", Some("3"), None),
            ],
        )
        .await?;

    let base_dir = tempdir()?;
    let temp_path = base_dir.path().join(&table_name);

    let mut c = tm.command()?;
    let cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "export",
        "--output-file",
        temp_path.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let export_content = std::fs::read_to_string(temp_path)?;
    util::assert_eq_json(
        &export_content,
        r#"[
        {
          "pk": "abc",
          "sk": 1
        },
        {
          "pk": "abc",
          "sk": 2,
          "a": 1,
          "b": 2
        },
        {
          "pk": "def",
          "sk": 3
        }
      ]"#,
    );

    Ok(())
}
