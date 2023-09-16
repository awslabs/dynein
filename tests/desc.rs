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
async fn test_desc_non_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let tm = util::setup().await?;
    let mut c = tm.command()?;
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
async fn test_desc_table_from_options() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk,S", Some("sk,N")).await?;

    let mut c = tm.command()?;
    let cmd = c.args(&["--region", "local", "--table", &table_name, "desc"]);
    cmd.assert().success().stdout(
        predicate::str::is_match(format!(
            "name: {}
region: local
status: ACTIVE
schema:
  pk: pk \\(S\\)
  sk: sk \\(N\\)
mode: OnDemand
capacity: ~
gsi: ~
lsi: ~
stream: ~
count: 0
size_bytes: 0
created_at: \".*\"",
            table_name
        ))
        .unwrap(),
    );

    Ok(())
}

#[tokio::test]
async fn test_desc_table_from_args() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk,S", Some("sk,N")).await?;

    let mut c = tm.command()?;
    let cmd = c.args(&["--region", "local", "desc", &table_name]);
    cmd.assert().success().stdout(
        predicate::str::is_match(format!(
            "name: {}
region: local
status: ACTIVE
schema:
  pk: pk \\(S\\)
  sk: sk \\(N\\)
mode: OnDemand
capacity: ~
gsi: ~
lsi: ~
stream: ~
count: 0
size_bytes: 0
created_at: \".*\"",
            table_name
        ))
        .unwrap(),
    );

    Ok(())
}

#[tokio::test]
async fn test_desc_all_tables() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup_with_lock().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;

    let mut c = tm.command()?;
    let cmd = c.args(&["--region", "local", "desc", "--all-tables"]);
    cmd.assert().success().stdout(
        predicate::str::is_match(format!(
            "name: {}
region: local
status: ACTIVE
schema:
  pk: pk \\(S\\)
  sk: ~
mode: OnDemand
capacity: ~
gsi: ~
lsi: ~
stream: ~
count: 0
size_bytes: 0
created_at: \".*\"",
            table_name
        ))
        .unwrap(),
    );

    Ok(())
}
