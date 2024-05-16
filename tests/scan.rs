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

use crate::util::TemporaryItem;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_scan_non_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let tm = util::setup().await?;
    let mut c = tm.command()?;
    let cmd = c.args([
        "--region",
        "local",
        "--table",
        "dummy-table-doesnt-exist",
        "scan",
    ]);
    cmd.assert().failure().stderr(predicate::str::contains(
        // The error message is different between DynamoDB local and real service.
        // It should be "Requested resource not found: Table: table not found" actually.
        "Cannot do operations on a non-existent table",
    ));
    Ok(())
}

#[tokio::test]
async fn test_scan_blank_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;

    let mut c = tm.command()?;
    let scan_cmd = c.args(["--region", "local", "--table", &table_name, "scan"]);
    scan_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("No item to show"));

    Ok(())
}

#[tokio::test]
async fn test_simple_scan() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;

    for action in ["scan", "s"] {
        let table_name = tm.create_temporary_table("pk", None).await?;
        let mut c = tm.command()?;
        c.args(["--region", "local", "--table", &table_name, "put", "abc"])
            .output()?;

        let mut c = tm.command()?;
        let scan_cmd = c.args(["--region", "local", "--table", &table_name, action]);
        scan_cmd
            .assert()
            .success()
            .stdout(predicate::str::contains("pk  attributes\nabc"));
    }

    Ok(())
}

#[tokio::test]
async fn test_consistent_scan() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items("pk,S", None, [TemporaryItem::new("1", None, None)])
        .await?;

    let mut c = tm.command()?;
    let scan_cmd = c.args(["--region", "local", "--table", &table_name, "scan"]);
    let scan_exec = scan_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));

    let scan_consistent_cmd = scan_cmd.args(["--consistent-read"]);
    scan_consistent_cmd
        .assert()
        .success()
        .stdout(scan_exec.get_output().stdout.to_owned());

    Ok(())
}

#[tokio::test]
async fn test_index_scan() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk,S",
            None,
            [TemporaryItem::new("1", None, Some("{'sk':'1'}"))],
        )
        .await?;

    let mut create_idx_cmd = tm.command()?;
    create_idx_cmd
        .args([
            "--region",
            "local",
            "--table",
            &table_name,
            "admin",
            "create",
            "index",
            "idx",
            "--keys",
            "sk,S",
        ])
        .assert()
        .success();

    // This sleep is required to prevent InternalFailure
    sleep(Duration::from_secs(1)).await;

    let mut scan_cmd = tm.command()?;
    let scan_exec = scan_cmd
        .args(["--region", "local", "--table", &table_name, "scan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sk"));

    let mut scan_idx_cmd = tm.command()?;
    scan_idx_cmd
        .args([
            "--region",
            "local",
            "--table",
            &table_name,
            "scan",
            "--index",
            "idx",
        ])
        .assert()
        .success()
        .stdout(scan_exec.get_output().stdout.to_owned());

    Ok(())
}

#[tokio::test]
async fn test_scan_with_attributes() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk,S",
            None,
            [TemporaryItem::new(
                "1",
                None,
                Some("{'opt1':'1','opt2':'2'}"),
            )],
        )
        .await?;

    let mut scan_cmd = tm.command()?;
    scan_cmd
        .args([
            "--region",
            "local",
            "--table",
            &table_name,
            "scan",
            "-a",
            "opt1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("opt1").and(predicate::str::contains("opt2").not()));

    Ok(())
}

#[tokio::test]
async fn test_scan_with_limits() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk,S",
            None,
            [
                TemporaryItem::new("opt1", None, None),
                TemporaryItem::new("opt2", None, None),
            ],
        )
        .await?;

    let mut scan_cmd = tm.command()?;
    scan_cmd
        .args([
            "--region",
            "local",
            "--table",
            &table_name,
            "scan",
            "--limit",
            "1",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("opt1")
                .and(predicate::str::contains("opt2").not())
                .or(predicate::str::contains("opt1")
                    .not()
                    .and(predicate::str::contains("opt2"))),
        );

    Ok(())
}
