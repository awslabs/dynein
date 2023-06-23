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
use tempfile::Builder;

#[tokio::test]
async fn test_shell_mode() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{Seek, SeekFrom};

    let table_name = "table--test_shell_mode";

    // $ dy admin create table <table_name> --keys pk
    let mut c = util::setup().await?;
    let shell_session = c.args(&["--region", "local", "--shell"]);
    let mut tmpfile = Builder::new().tempfile()?.into_file();
    writeln!(tmpfile, "admin create table {} --keys pk", table_name)?;
    writeln!(tmpfile, "use {}", table_name)?;
    writeln!(tmpfile, "desc")?;
    tmpfile.seek(SeekFrom::Start(0))?;
    shell_session
        .stdin(tmpfile)
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            &table_name
        )));

    util::cleanup(vec![table_name]).await
}
