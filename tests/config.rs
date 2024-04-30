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

#[tokio::test]
async fn test_config_dump() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup_with_lock().await?;

    let mut c = tm.command()?;
    let cmd = c.args(["config", "clear"]);
    cmd.assert().success();

    let table_name = tm.create_temporary_table("pk", None).await?;
    assert_config_use_dump(&tm, table_name).await?;

    Ok(())
}

#[tokio::test]
async fn test_config_clear() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup_with_lock().await?;

    let mut c = tm.command()?;
    let cmd = c.args(["config", "clear"]);
    cmd.assert().success();

    let table_name = tm.create_temporary_table("pk", None).await?;
    // In order to check existence of config before clear
    assert_config_use_dump(&tm, table_name).await?;

    let home = dirs::home_dir().unwrap();
    let base = home.to_str().unwrap();
    let config_dir = format!("{base}/.dynein");
    util::check_dynein_files_existence(&config_dir, true);

    let mut c = tm.command()?;
    let cmd = c.args(["config", "clear"]);
    cmd.assert().success();
    util::check_dynein_files_existence(&config_dir, false);

    let mut c = tm.command()?;
    let cmd = c.args(["config", "dump"]);
    cmd.assert().success().stdout(
        "tables: null

using_region: null
using_table: null
using_port: null
query:
  strict_mode: false

",
    );

    Ok(())
}

async fn assert_config_use_dump(
    tm: &util::TestManager<'_>,
    table_name: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut c = tm.command()?;
    let cmd = c.args(["--region", "local", "use", &table_name]);
    cmd.assert().success().stdout(format!(
        "Now you're using the table '{table_name}' (local).\n"
    ));

    let mut c = tm.command()?;
    let cmd = c.args(["config", "dump"]);
    cmd.assert().success().stdout(format!(
        "tables:
  local/{table_name}:
    region: local
    name: {table_name}
    pk:
      name: pk
      kind: S
    sk: null
    indexes: null
    mode: OnDemand

using_region: local
using_table: {table_name}
using_port: 8000
query:
  strict_mode: false

"
    ));

    Ok(())
}
