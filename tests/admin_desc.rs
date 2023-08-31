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
async fn test_admin_desc_table_from_options() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = util::create_temporary_table("pk,S", Some("sk,N")).await?;
    let mut c = util::setup().await?;
    let cmd = c.args(&["--region", "local", "admin", "--table", &table_name, "desc"]);
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
    util::cleanup(vec![&table_name]).await
}

#[tokio::test]
async fn test_admin_desc_table_from_args() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = util::create_temporary_table("pk,S", Some("sk,N")).await?;

    let mut c = util::setup().await?;
    let cmd = c.args(&["--region", "local", "admin", "desc", &table_name]);
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
    util::cleanup(vec![&table_name]).await
}

#[tokio::test]
async fn test_admin_desc_all_tables() -> Result<(), Box<dyn std::error::Error>> {
    let table_name1 = util::create_temporary_table("pk", None).await?;
    let table_name2 = util::create_temporary_table("pk,S", Some("sk,N")).await?;

    let mut c = util::setup().await?;
    let cmd = c.args(&["--region", "local", "admin", "desc", "--all-tables"]);
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
            table_name1
        ))
        .unwrap()
        .and(
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
                table_name2
            ))
            .unwrap(),
        ),
    );
    util::cleanup(vec![&table_name1, &table_name2]).await
}
