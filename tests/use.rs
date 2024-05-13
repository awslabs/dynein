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

use crate::util::{assert_eq_cmd_json, assert_eq_yaml};
use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*;

/// Checks if entries exist in a table.
///
/// # Arguments
///
/// * `tm` - A mutable reference to a `TestManager` instance.
/// * `tbl` - The name of the table to use.
/// * `pk` - The primary key of the table.
/// * `entries` - A slice of primary key strings representing the items to check.
/// * `exist` - A boolean indicating whether the entries should exist or not.
/// * `with_table` - A boolean that indicates whether the '--table' argument should be used in the command or not.
///   If true, '--table' is included in the command; otherwise, it is not.
///
/// # Returns
///
/// Returns `Ok(())` if the check completes successfully, otherwise returns an error.
///
/// # Errors
///
/// Returns an error if any command fails or if the expected output does not match.
fn check_table_entries_existence(
    tm: &mut util::TestManager<'_>,
    tbl: &str,
    pk: &str,
    entries: &[&str],
    exist: bool,
    with_table: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut c = tm.command()?;
    if with_table {
        c.args(["--region", "local", "use", "--table", tbl]);
    } else {
        c.args(["--region", "local", "use", tbl]);
    }
    c.assert().success();
    for &entry in entries {
        let mut c = tm.command()?;
        let cmd = c.args(["get", entry]);
        if exist {
            assert_eq_cmd_json(cmd, &format!(r#"{{"{}":"{}"}}"#, pk, entry));
        } else {
            cmd.assert()
                .success()
                .stdout(predicate::str::contains("No item found."));
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_use() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;

    // We will check both cases `use --table <table name>` and `use <table name>`
    for with_table_arg in [true, false] {
        let tbl = tm
            .create_temporary_table_with_items(
                "pk",
                None,
                [util::TemporaryItem::new("pk1", None, None)],
            )
            .await?;
        let mut c = tm.command()?;
        if with_table_arg {
            c.args(["-r", "local", "use", "--table", &tbl]);
        } else {
            c.args(["-r", "local", "use", &tbl]);
        }
        c.assert().success();

        // Derive config file path
        let mut config_path = tm.default_config_dir();
        config_path.push("config.yml");

        // Check config file contents
        let config_contents = std::fs::read_to_string(&config_path)?;
        assert_eq_yaml(
            config_contents,
            format!(
                r#"
                using_region: local
                using_table: {tbl}
                using_port: 8000
                query:
                    strict_mode: false
                retry: null
                "#
            ),
        );

        assert_eq_cmd_json(tm.command()?.args(["get", "pk1"]), r#"{"pk":"pk1"}"#);
    }

    Ok(())
}

#[tokio::test]
async fn test_use_switch() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;

    // We will check both cases `use --table <table name>` and `use <table name>`
    for with_table_arg in [true, false] {
        // Generate two tables to use in a test
        let tbl1 = tm
            .create_temporary_table_with_items(
                "pk1",
                None,
                vec![
                    util::TemporaryItem::new("v1", None, None),
                    util::TemporaryItem::new("v2", None, None),
                ],
            )
            .await?;
        let tbl2 = tm
            .create_temporary_table_with_items(
                "pk2",
                None,
                vec![
                    util::TemporaryItem::new("v3", None, None),
                    util::TemporaryItem::new("v4", None, None),
                ],
            )
            .await?;

        check_table_entries_existence(&mut tm, &tbl1, "pk1", &["v1", "v2"], true, with_table_arg)?;
        check_table_entries_existence(&mut tm, &tbl1, "pk2", &["v3", "v4"], false, with_table_arg)?;
        check_table_entries_existence(&mut tm, &tbl2, "pk1", &["v1", "v2"], false, with_table_arg)?;
        check_table_entries_existence(&mut tm, &tbl2, "pk2", &["v3", "v4"], true, with_table_arg)?;
    }

    Ok(())
}

#[tokio::test]
async fn test_use_non_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let tm = util::setup().await?;

    // Generate a table name that (hopefully) does not exist
    let non_existent_table = "NonExistentTable";

    // Attempt to use the non-existent table
    tm.command()?
        .args(["--region", "local", "use", non_existent_table])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            // The error message is different between DynamoDB local and real service.
            // It should be "Requested resource not found: Table: NonExistentTable not found" actually.
            "Cannot do operations on a non-existent table",
        ));

    Ok(())
}
