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
use std::io::Write;
use std::process::Stdio;

#[tokio::test]
async fn test_admin_delete_non_existento_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut c = util::setup().await?;
    let cmd = c.args(&[
        "--region",
        "local",
        "admin",
        "delete",
        "table",
        "dummy-table",
    ]);

    cmd.assert().failure().stderr(predicate::str::contains(
        // The error message is different between DynamoDB local and real service.
        // Is should be "Requested resource not found: Table: dummy-table not found" actually.
        "Cannot do operations on a non-existent table",
    ));

    Ok(())
}

#[tokio::test]
async fn test_admin_delete_non_existent_item() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = util::create_temporary_table("pk", None).await?;

    let mut c = util::setup().await?;
    let mut cmd = c
        .args(&["--region", "local", "admin", "delete", "table", &table_name])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start child process");

    {
        let stdin = cmd.stdin.as_mut().expect("failed to get stdin");
        stdin.write_all(b"y\n").expect("failed to write to stdin");
    }

    let out = cmd.wait_with_output().expect("failet to waito on child");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    println!("stdout {}", stdout);
    println!("sterr {}", stderr);

    Ok(())
}

#[tokio::test]
async fn test_admin_delete_existent_item() -> Result<(), Box<dyn std::error::Error>> {
    // TODO:

    Ok(())
}

#[tokio::test]
async fn test_admin_delete_existent_item_with_sk() -> Result<(), Box<dyn std::error::Error>> {
    // TODO:

    Ok(())
}
