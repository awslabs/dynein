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
async fn test_admin_delete_non_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let tm = util::setup_with_lock().await?;
    let mut c = tm.command()?;
    let cmd = c.args(&[
        "--region",
        "local",
        "admin",
        "delete",
        "table",
        "dummy-table",
        "--yes",
    ]);

    cmd.assert().failure().stderr(predicate::str::contains(
        // The error message is different between DynamoDB local and real service.
        // Is should be "Requested resource not found: Table: dummy-table not found" actually.
        "Cannot do operations on a non-existent table",
    ));

    Ok(())
}

#[tokio::test]
async fn test_admin_delete_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup_with_lock().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;
    let mut c = tm.command()?;

    let cmd = c.args(&[
        "--region",
        "local",
        "admin",
        "delete",
        "table",
        &table_name,
        "--yes",
    ]);
    cmd.assert().success().stdout(format!(
        "Delete operation for the table '{}' has been started.\n",
        table_name
    ));

    // Verify that the table was successfully removed
    let mut c = tm.command()?;
    let cmd = c.args(["--region", "local", "admin", "list"]);
    cmd.assert().success().stdout(
        predicate::str::is_match(
            "DynamoDB tables in region: local
  No table in this region.",
        )
        .unwrap(),
    );

    // To prevent double deletion in the Drop trait, exclude the table here
    tm.remove_temporary_table(&table_name);

    Ok(())
}
