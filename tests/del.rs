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
async fn test_del_non_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let tm = util::setup().await?;
    let mut c = tm.command()?;
    let cmd = c.args([
        "--region",
        "local",
        "--table",
        "dummy-table-doesnt-exist",
        "del",
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
async fn test_del_non_existent_item() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;

    let mut c = tm.command()?;
    let cmd = c.args(["--region", "local", "--table", &table_name, "del", "42"]);
    cmd.assert().success().stdout(format!(
        "Successfully deleted an item from the table '{}'.\n",
        table_name
    ));
    Ok(())
}

#[tokio::test]
async fn test_del_existent_item() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;

    for action in ["d", "del", "delete"] {
        let table_name = tm
            .create_temporary_table_with_items(
                "pk",
                None,
                vec![
                    util::TemporaryItem::new("a", None, None),
                    util::TemporaryItem::new("b", None, None),
                ],
            )
            .await?;

        let mut c = tm.command()?;
        let cmd = c.args(["--region", "local", "--table", &table_name, action, "a"]);
        cmd.assert().success().stdout(format!(
            "Successfully deleted an item from the table '{}'.\n",
            table_name
        ));

        let mut c = tm.command()?;
        let scan_cmd = c.args(["--region", "local", "--table", &table_name, "scan"]);
        scan_cmd
            .assert()
            .success()
            .stdout(predicate::str::diff("pk  attributes\nb\n"));
    }

    Ok(())
}

#[tokio::test]
async fn test_del_existent_item_with_sk() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk,S",
            Some("sk,N"),
            vec![
                util::TemporaryItem::new("abc", Some("1"), None),
                util::TemporaryItem::new("abc", Some("2"), None),
                util::TemporaryItem::new("abc", Some("3"), None),
            ],
        )
        .await?;

    let mut c = tm.command()?;
    let cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "del",
        "abc",
        "2",
    ]);
    cmd.assert().success().stdout(format!(
        "Successfully deleted an item from the table '{}'.\n",
        table_name
    ));

    let mut c = tm.command()?;
    let scan_cmd = c.args(["--region", "local", "--table", &table_name, "scan"]);
    scan_cmd.assert().success().stdout(predicate::str::diff(
        "pk   sk  attributes\nabc  1\nabc  3\n",
    ));

    Ok(())
}
