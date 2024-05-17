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

use crate::util::{assert_eq_json_ignore_order, setup, TemporaryItem};
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_admin_create_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = setup().await?;
    const TBL: &str = "table--test_admin_create_table";
    tm.command()?
        .args([
            "-r", "local", "admin", "create", "table", TBL, "--keys", "pk",
        ])
        .assert()
        .success();
    tm.add_tables_to_delete([TBL]);

    tm.command()?
        .args(["-r", "local", "ls"])
        .assert()
        .success()
        .stdout(predicate::str::contains(TBL));

    tm.command()?
        .args(["-r", "local", "desc", "--table", TBL])
        .assert()
        .success()
        .stdout(predicate::str::is_match(format!(
            "name: {TBL}
region: local
status: ACTIVE
schema:
  pk: pk \\(S\\)
  sk: null
mode: OnDemand
capacity: null
gsi: null
lsi: null
stream: null
count: 0
size_bytes: 0
created_at: .*"
        ))?);

    Ok(())
}

#[tokio::test]
async fn test_create_table_with_region_local_and_port_number_options(
) -> Result<(), Box<dyn std::error::Error>> {
    const PORT: i32 = 8001;
    const TBL: &str = "table--test_create_table_with_region_local_and_port_number_options";
    let mut tm = util::setup_with_port(PORT).await?;

    let mut c = tm.command()?;
    let create_cmd = c.args([
        "--region",
        "local",
        "--port",
        &format!("{}", PORT),
        "admin",
        "create",
        "table",
        TBL,
        "--keys",
        "pk",
    ]);
    create_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            TBL
        )));
    tm.add_tables_to_delete([TBL]);

    // $ dy admin desc <table_name>
    let mut c = tm.command()?;
    let desc_cmd = c.args([
        "--region",
        "local",
        "--port",
        &format!("{}", PORT),
        "desc",
        TBL,
    ]);
    desc_cmd
        .assert()
        .success()
        .stdout(predicate::str::is_match(format!(
            "name: {TBL}
region: local
status: ACTIVE
schema:
  pk: pk \\(S\\)
  sk: null
mode: OnDemand
capacity: null
gsi: null
lsi: null
stream: null
count: 0
size_bytes: 0
created_at: .*"
        ))?);

    Ok(())
}

#[tokio::test]
async fn test_admin_create_table_with_type() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = setup().await?;
    const TBL: &str = "table--test_admin_create_table_with_type";
    tm.command()?
        .args([
            "-r", "local", "admin", "create", "table", TBL, "--keys", "pk,N",
        ])
        .assert()
        .success();
    tm.add_tables_to_delete([TBL]);

    tm.command()?
        .args(["-r", "local", "describe", "--table", TBL])
        .assert()
        .success()
        .stdout(predicate::str::is_match(format!(
            "name: {TBL}
region: local
status: ACTIVE
schema:
  pk: pk \\(N\\)
  sk: null
mode: OnDemand
capacity: null
gsi: null
lsi: null
stream: null
count: 0
size_bytes: 0
created_at: .*"
        ))?);

    Ok(())
}

#[tokio::test]
async fn test_admin_create_table_with_sk() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = setup().await?;
    const TBL: &str = "table-test_admin_create_table_with_sk";
    tm.command()?
        .args([
            "-r", "local", "admin", "create", "table", TBL, "--keys", "pk", "sk",
        ])
        .assert()
        .success();
    tm.add_tables_to_delete([TBL]);

    tm.command()?
        .args(["-r", "local", "desc", "--table", TBL])
        .assert()
        .success()
        .stdout(predicate::str::is_match(format!(
            "name: {TBL}
region: local
status: ACTIVE
schema:
  pk: pk \\(S\\)
  sk: sk \\(S\\)
mode: OnDemand
capacity: null
gsi: null
lsi: null
stream: null
count: 0
size_bytes: 0
created_at: .*"
        ))?);

    Ok(())
}

#[tokio::test]
async fn test_admin_create_table_with_sk_type() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = setup().await?;
    const TBL: &str = "table-test_admin_create_table_with_sk_type";
    tm.command()?
        .args([
            "-r", "local", "admin", "create", "table", TBL, "--keys", "pk,B", "sk,N",
        ])
        .assert()
        .success();
    tm.add_tables_to_delete([TBL]);

    tm.command()?
        .args(["-r", "local", "show", "--table", TBL])
        .assert()
        .success()
        .stdout(predicate::str::is_match(format!(
            "name: {TBL}
region: local
status: ACTIVE
schema:
  pk: pk \\(B\\)
  sk: sk \\(N\\)
mode: OnDemand
capacity: null
gsi: null
lsi: null
stream: null
count: 0
size_bytes: 0
created_at: .*"
        ))?);

    Ok(())
}

#[tokio::test]
async fn test_admin_create_index() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = setup().await?;
    let tbl = tm
        .create_temporary_table_with_items(
            "pk",
            None,
            [
                TemporaryItem::new("pk1", None, Some(r#"{"gsi":1}"#)),
                TemporaryItem::new("pk2", None, None),
            ],
        )
        .await?;
    tm.command()?
        .args([
            "-r", "local", "admin", "create", "index", "--table", &tbl, "idx", "--keys", "gsi,N",
        ])
        .assert()
        .success();

    sleep(Duration::from_secs(1)).await;

    tm.command()?
        .args(["-r", "local", "info", "--table", &tbl])
        .assert()
        .success()
        .stdout(predicate::str::is_match(format!(
            "name: {tbl}
region: local
status: ACTIVE
schema:
  pk: pk \\(S\\)
  sk: null
mode: OnDemand
capacity: null
gsi:
- name: idx
  schema:
    pk: gsi \\(N\\)
    sk: null
  capacity: null
lsi: null
stream: null
count: 2
size_bytes: \\d+
created_at: .*"
        ))?);

    assert_eq_json_ignore_order(
        tm.command()?
            .args(["-r", "local", "--table", &tbl, "scan", "--output", "raw"]),
        r#"[
            {"pk":{"S":"pk1"}, "gsi":{"N":"1"}},
            {"pk":{"S":"pk2"}}
        ]"#,
    );

    assert_eq_json_ignore_order(
        tm.command()?.args([
            "-r", "local", "--table", &tbl, "scan", "--index", "idx", "--output", "raw",
        ]),
        r#"[
            {"pk":{"S":"pk1"}, "gsi":{"N":"1"}}
        ]"#,
    );

    Ok(())
}

#[tokio::test]
async fn test_admin_create_index_with_gsi() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = setup().await?;
    let tbl = tm
        .create_temporary_table_with_items(
            "pk",
            None,
            [
                TemporaryItem::new("pk1", None, Some(r#"{"gsi":"gsi1"}"#)),
                TemporaryItem::new("pk2", None, None),
            ],
        )
        .await?;
    tm.command()?
        .args([
            "-r", "local", "admin", "create", "index", "--table", &tbl, "idx", "--keys", "pk,S",
            "gsi,N",
        ])
        .assert()
        .success();

    sleep(Duration::from_secs(1)).await;

    tm.command()?
        .args(["-r", "local", "desc", "--table", &tbl])
        .assert()
        .success()
        .stdout(predicate::str::is_match(format!(
            "name: {tbl}
region: local
status: ACTIVE
schema:
  pk: pk \\(S\\)
  sk: null
mode: OnDemand
capacity: null
gsi:
- name: idx
  schema:
    pk: pk \\(S\\)
    sk: gsi \\(N\\)
  capacity: null
lsi: null
stream: null
count: 2
size_bytes: \\d+
created_at: .*"
        ))?);

    assert_eq_json_ignore_order(
        tm.command()?
            .args(["-r", "local", "--table", &tbl, "scan", "--output", "raw"]),
        r#"[
            {"pk":{"S":"pk1"}, "gsi":{"S":"gsi1"}},
            {"pk":{"S":"pk2"}}
        ]"#,
    );

    assert_eq_json_ignore_order(
        tm.command()?.args([
            "-r", "local", "--table", &tbl, "scan", "--index", "idx", "--output", "raw",
        ]),
        r#"[
            {"pk":{"S":"pk1"}, "gsi":{"S":"gsi1"}}
        ]"#,
    );

    Ok(())
}
