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
async fn test_apply() -> Result<(), Box<dyn std::error::Error>> {
    let tm = util::setup().await?;
    let mut c = tm.command()?;
    let cmd = c.args(["--region", "local", "admin", "apply"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("not yet implemented"));
    Ok(())
}

#[tokio::test]
#[should_panic(expected = "not yet implemented")]
async fn test_apply_with_dev() {
    let tm = util::setup().await.unwrap();
    let mut c = tm.command().unwrap();
    let cmd = c.args(["--region", "local", "admin", "apply", "--dev"]);
    cmd.unwrap();
}
