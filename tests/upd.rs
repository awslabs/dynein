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

use crate::util::assert_eq_cmd_json;
use assert_cmd::assert::OutputAssertExt;
use predicates::prelude::*;

pub mod util;

#[tokio::test]
async fn test_upd() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;

    for action in ["upd", "update", "u"] {
        let tbl = tm.create_temporary_table("pk", None).await?;
        tm.command()?
            .args([
                "--region",
                "local",
                "--table",
                &tbl,
                action,
                "pk1",
                "--set",
                "attr1=123",
            ])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("pk1")
                    .and(predicate::str::contains("attr1"))
                    .and(predicate::str::contains("123")),
            );

        let mut cmd = tm.command()?;
        cmd.args(["--region", "local", "--table", &tbl, "get", "pk1"]);
        assert_eq_cmd_json(&mut cmd, r#"{"pk":"pk1","attr1":123}"#);

        tm.command()?
            .args([
                "--region",
                "local",
                "--table",
                &tbl,
                action,
                "pk1",
                "--set",
                "attr2='str'",
            ])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("pk1")
                    .and(predicate::str::contains("attr1"))
                    .and(predicate::str::contains("attr2"))
                    .and(predicate::str::contains("str")),
            );

        let mut cmd = tm.command()?;
        cmd.args(["--region", "local", "--table", &tbl, "get", "pk1"]);
        assert_eq_cmd_json(&mut cmd, r#"{"pk":"pk1","attr1":123,"attr2":"str"}"#);

        tm.command()?
            .args([
                "--region",
                "local",
                "--table",
                &tbl,
                action,
                "pk1",
                "--remove",
                "attr1,attr2",
            ])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("pk1")
                    .and(predicate::str::contains("attr1").not())
                    .and(predicate::str::contains("attr2").not()),
            );

        let mut cmd = tm.command()?;
        cmd.args(["--region", "local", "--table", &tbl, "get", "pk1"]);
        assert_eq_cmd_json(&mut cmd, r#"{"pk":"pk1"}"#);
    }

    Ok(())
}

#[tokio::test]
async fn test_upd_with_sk() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let tbl = tm.create_temporary_table("pk", Some("sk")).await?;

    tm.command()?
        .args([
            "--region",
            "local",
            "--table",
            &tbl,
            "upd",
            "pk1",
            "sk1",
            "--set",
            "attr1=123",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("pk1")
                .and(predicate::str::contains("sk1"))
                .and(predicate::str::contains("attr1"))
                .and(predicate::str::contains("123")),
        );

    let mut cmd = tm.command()?;
    cmd.args(["--region", "local", "--table", &tbl, "get", "pk1", "sk1"]);
    assert_eq_cmd_json(&mut cmd, r#"{"pk":"pk1","sk":"sk1","attr1":123}"#);

    tm.command()?
        .args([
            "--region",
            "local",
            "--table",
            &tbl,
            "upd",
            "pk1",
            "sk1",
            "--set",
            "attr2='str'",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("pk1")
                .and(predicate::str::contains("sk1"))
                .and(predicate::str::contains("attr1"))
                .and(predicate::str::contains("attr2"))
                .and(predicate::str::contains("str")),
        );

    let mut cmd = tm.command()?;
    cmd.args(["--region", "local", "--table", &tbl, "get", "pk1", "sk1"]);
    assert_eq_cmd_json(
        &mut cmd,
        r#"{"pk":"pk1","sk":"sk1","attr1":123,"attr2":"str"}"#,
    );

    tm.command()?
        .args([
            "--region",
            "local",
            "--table",
            &tbl,
            "upd",
            "pk1",
            "sk1",
            "--remove",
            "attr1,attr2",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("pk1")
                .and(predicate::str::contains("sk1"))
                .and(predicate::str::contains("attr1").not())
                .and(predicate::str::contains("attr2").not()),
        );

    let mut cmd = tm.command()?;
    cmd.args(["--region", "local", "--table", &tbl, "get", "pk1", "sk1"]);
    assert_eq_cmd_json(&mut cmd, r#"{"pk":"pk1","sk":"sk1"}"#);

    Ok(())
}

#[tokio::test]
async fn test_upd_fibonacci() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let tbl = tm.create_temporary_table("pk", None).await?;

    // Initial setting for Fibonacci sequence
    tm.command()?
        .args([
            "--region",
            "local",
            "--table",
            &tbl,
            "upd",
            "pk1",
            "--set",
            "n1=0,n2=1",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("pk1")
                .and(predicate::str::contains("n1"))
                .and(predicate::str::contains("n2")),
        );

    // Fibonacci sequence: 0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, ...
    let fib_sequence = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89];
    for i in 0..10 {
        tm.command()?
            .args([
                "--region",
                "local",
                "--table",
                &tbl,
                "upd",
                "pk1",
                "--set",
                "n1=n2,n2=n1+n2",
            ])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("pk1")
                    .and(predicate::str::contains("n1"))
                    .and(predicate::str::contains(&format!(
                        "{}",
                        fib_sequence[i + 2]
                    ))), // +2 because we start from third element
            );
    }

    let mut cmd = tm.command()?;
    cmd.args(["--region", "local", "--table", &tbl, "get", "pk1"]);
    assert_eq_cmd_json(&mut cmd, r#"{"pk":"pk1","n1":55,"n2":89}"#);

    Ok(())
}

#[tokio::test]
async fn test_upd_atomic_counter() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let tbl = tm.create_temporary_table("pk", None).await?;

    // Set initial counter value to 0
    tm.command()?
        .args([
            "--region",
            "local",
            "--table",
            &tbl,
            "upd",
            "pk1",
            "--set",
            "counter=0",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("pk1").and(predicate::str::contains("counter")));

    // Increment the counter 5 times
    for i in 1..=5 {
        tm.command()?
            .args([
                "--region",
                "local",
                "--table",
                &tbl,
                "upd",
                "pk1",
                "--atomic-counter",
                "counter",
            ])
            .assert()
            .success()
            .stdout(
                predicate::str::contains("pk1")
                    .and(predicate::str::contains("counter"))
                    .and(predicate::str::contains(format!("{}", i))),
            );
    }

    let mut cmd = tm.command()?;
    cmd.args(["--region", "local", "--table", &tbl, "get", "pk1"]);
    assert_eq_cmd_json(&mut cmd, r#"{"pk":"pk1","counter":5}"#);

    Ok(())
}
