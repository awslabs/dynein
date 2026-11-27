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

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::time::Duration;
#[tokio::test]
async fn test_admin_update_table_mode() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let tbl = tm.create_temporary_table("pk", None).await?;

    tm.command()?
        .args([
            "--region",
            "local",
            "admin",
            "update",
            "table",
            &tbl,
            "--mode",
            "provisioned",
            "--rcu",
            "5",
            "--wcu",
            "10",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("mode: Provisioned")
                .and(predicate::str::contains("rcu: 5"))
                .and(predicate::str::contains("wcu: 10")),
        );

    tm.command()?
        .args([
            "--region", "local", "admin", "update", "table", &tbl, "--mode", "ondemand",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("mode: OnDemand")
                .and(predicate::str::contains("capacity: null")),
        );

    Ok(())
}

#[tokio::test]
async fn test_admin_update_table_mode_with_index_requires_inherit_flag(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let tbl = tm.create_temporary_table("pk", None).await?;

    tm.command()?
        .args([
            "--region", "local", "admin", "create", "index", "idx", "--table", &tbl, "--keys",
            "gsipk,S", "gsisk,S",
        ])
        .assert()
        .success();

    tokio::time::sleep(Duration::from_secs(1)).await;

    tm.command()?
        .args([
            "--region",
            "local",
            "admin",
            "update",
            "table",
            &tbl,
            "--mode",
            "provisioned",
            "--rcu",
            "5",
            "--wcu",
            "5",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--inherit-index-capacity"));

    tm.command()?
        .args(["--region", "local", "admin", "desc", &tbl])
        .assert()
        .success()
        .stdout(predicate::str::contains("mode: OnDemand"));

    Ok(())
}

#[tokio::test]
async fn test_admin_update_table_mode_with_index() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let tbl = tm.create_temporary_table("pk", None).await?;

    tm.command()?
        .args([
            "--region", "local", "admin", "create", "index", "idx", "--table", &tbl, "--keys",
            "gsipk,S", "gsisk,S",
        ])
        .assert()
        .success();

    tokio::time::sleep(Duration::from_secs(1)).await;

    tm.command()?
        .args([
            "--region",
            "local",
            "admin",
            "update",
            "table",
            &tbl,
            "--mode",
            "provisioned",
            "--rcu",
            "100",
            "--wcu",
            "10",
            "--inherit-index-capacity",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("mode: Provisioned")
                .and(predicate::str::contains("rcu: 100"))
                .and(predicate::str::contains("wcu: 10"))
                .and(predicate::str::contains("idx: rcu 100, wcu 10"))
                // Index capacity must be the inherited units, not the zero values
                // DynamoDB reports when a GSI is omitted from the billing-mode switch.
                // See: https://github.com/awslabs/dynein/issues/228
                .and(predicate::str::contains("rcu: 0").not())
                .and(predicate::str::contains("wcu: 0").not()),
        );

    // Query the table again so this checks the GSI's persisted throughput,
    // rather than only the UpdateTable response.
    tm.command()?
        .args(["--region", "local", "admin", "desc", &tbl])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("mode: Provisioned")
                .and(predicate::str::contains("gsi:\n- name: idx"))
                .and(predicate::str::contains("    wcu: 10\n    rcu: 100")),
        );

    tm.command()?
        .args([
            "--region", "local", "admin", "update", "table", &tbl, "--mode", "ondemand",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("mode: OnDemand")
                .and(predicate::str::contains("capacity: null")),
        );

    tm.command()?
        .args([
            "--region",
            "local",
            "admin",
            "update",
            "table",
            &tbl,
            "--mode",
            "provisioned",
            "--inherit-index-capacity",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("mode: Provisioned")
                .and(predicate::str::contains("idx: rcu 5, wcu 5"))
                .and(predicate::str::contains("rcu: 5"))
                .and(predicate::str::contains("wcu: 5"))
                .and(predicate::str::contains("rcu: 0").not())
                .and(predicate::str::contains("wcu: 0").not()),
        );

    tm.command()?
        .args(["--region", "local", "admin", "desc", &tbl])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("mode: Provisioned")
                .and(predicate::str::contains("gsi:\n- name: idx"))
                .and(predicate::str::contains("    wcu: 5\n    rcu: 5")),
        );

    Ok(())
}
