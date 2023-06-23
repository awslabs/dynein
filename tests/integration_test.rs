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

mod util;

use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs
                           // use assert_cmd::cmd::Command; // Run programs - it seems to be equal to "use assert_cmd::prelude::* + use std::process::Command"

use std::fs::File;
use std::io::Write; // Used when check results by printing to stdout

use tempfile::Builder;

#[tokio::test]
async fn test_help() -> Result<(), Box<dyn std::error::Error>> {
    let mut dynein_cmd = Command::cargo_bin("dy")?;
    let cmd = dynein_cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("dynein is a command line tool"));
    Ok(())
}

#[tokio::test]
async fn test_custom_config_location() -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;

    let dummy_dir = "./tests/dummy_dir";
    let config_dir = dummy_dir.to_string() + "/.dynein";

    // cleanup config folder in case it was already there
    util::cleanup_config(dummy_dir).await.ok();

    // dy config clear
    Command::cargo_bin("dy")?
        .args(&["config", "clear"])
        .env("DYNEIN_CONFIG_DIR", dummy_dir)
        .output()?;

    // check config folder created at our desired location
    assert!(Path::new(&config_dir).exists());

    // cleanup config folder
    util::cleanup_config(dummy_dir).await.ok();

    Ok(())
}

#[tokio::test]
async fn test_create_table() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_create_table";

    // $ dy admin create table <table_name> --keys pk
    let mut c = util::setup().await?;
    let create_cmd = c.args(&[
        "--region", "local", "admin", "create", "table", table_name, "--keys", "pk",
    ]);
    create_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            &table_name
        )));

    // $ dy admin desc <table_name>
    let mut c = util::setup().await?;
    let desc_cmd = c.args(&["--region", "local", "desc", table_name]);
    desc_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            &table_name
        )));

    util::cleanup(vec![table_name]).await
}

#[tokio::test]
async fn test_create_table_with_region_local_and_port_number_options(
) -> Result<(), Box<dyn std::error::Error>> {
    let port = 8001;
    let table_name = "table--test_create_table_with_region_local_and_port_number_options";

    // $ dy admin create table <table_name> --keys pk
    let mut c = util::setup_with_port(port).await?;
    let create_cmd = c.args(&[
        "--region",
        "local",
        "--port",
        &format!("{}", port),
        "admin",
        "create",
        "table",
        table_name,
        "--keys",
        "pk",
    ]);
    create_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            &table_name
        )));

    // $ dy admin desc <table_name>
    let mut c = util::setup_with_port(port).await?;
    let desc_cmd = c.args(&[
        "--region",
        "local",
        "--port",
        &format!("{}", port),
        "desc",
        table_name,
    ]);
    desc_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            &table_name
        )));

    util::cleanup_with_port(vec![table_name], port).await
}

#[tokio::test]
async fn test_scan_non_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut c = util::setup().await?;
    let cmd = c.args(&[
        "--region",
        "local",
        "--table",
        "dummy-table-doent-exist",
        "scan",
    ]);
    cmd.assert().failure().stderr(predicate::str::contains(
        "Cannot do operations on a non-existent table",
    ));
    Ok(())
}

#[tokio::test]
async fn test_scan_blank_table() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_scan_blank_table";

    let mut c = util::setup().await?;
    c.args(&[
        "--region", "local", "admin", "create", "table", table_name, "--keys", "pk",
    ])
    .output()?;
    let mut c = util::setup().await?;
    let scan_cmd = c.args(&["--region", "local", "--table", table_name, "scan"]);
    scan_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("No item to show"));

    util::cleanup(vec![table_name]).await
}

#[tokio::test]
async fn test_simple_scan() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_simple_scan";

    let mut c = util::setup().await?;
    c.args(&[
        "--region", "local", "admin", "create", "table", table_name, "--keys", "pk",
    ])
    .output()?;
    let mut c = util::setup().await?;
    c.args(&["--region", "local", "--table", table_name, "put", "abc"])
        .output()?;

    let mut c = util::setup().await?;
    let scan_cmd = c.args(&["--region", "local", "--table", table_name, "scan"]);
    scan_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("pk  attributes\nabc"));

    util::cleanup(vec![table_name]).await
}

async fn prepare_pk_sk_table(table_name: &&str) -> Result<(), Box<dyn std::error::Error>> {
    let mut c = util::setup().await?;
    c.args(&[
        "--region", "local", "admin", "create", "table", table_name, "--keys", "pk,S", "sk,N",
    ])
    .output()?;
    let mut c = util::setup().await?;
    c.args(&[
        "--region", "local", "--table", table_name, "put", "abc", "1",
    ])
    .output()?;
    let mut c = util::setup().await?;
    c.args(&[
        "--region", "local", "--table", table_name, "put", "abc", "2",
    ])
    .output()?;
    Ok(())
}

#[tokio::test]
async fn test_simple_query() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_simple_query";

    prepare_pk_sk_table(&table_name).await?;
    let mut c = util::setup().await?;
    let query_cmd = c.args(&["--region", "local", "--table", table_name, "query", "abc"]);
    query_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "pk   sk  attributes\nabc  1\nabc  2",
        ));

    util::cleanup(vec![table_name]).await
}

#[tokio::test]
async fn test_simple_desc_query() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_desc_simple_query";

    prepare_pk_sk_table(&table_name).await?;
    let mut c = util::setup().await?;
    let query_cmd = c.args(&[
        "--region", "local", "--table", table_name, "query", "abc", "-d",
    ]);
    query_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "pk   sk  attributes\nabc  2\nabc  1",
        ));

    util::cleanup(vec![table_name]).await
}

#[tokio::test]
async fn test_query_limit() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_query_limit";

    prepare_pk_sk_table(&table_name).await?;
    let mut c = util::setup().await?;
    let query_cmd = c.args(&[
        "--region", "local", "--table", table_name, "query", "abc", "-l", "1",
    ]);
    query_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("pk   sk  attributes\nabc  1"));

    util::cleanup(vec![table_name]).await
}

#[tokio::test]
async fn test_batch_write() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_batch_write";

    let mut c = util::setup().await?;
    c.args(&[
        "--region", "local", "admin", "create", "table", table_name, "--keys", "pk",
    ])
    .output()?;

    let tmpdir = Builder::new().tempdir()?; // defining stand alone variable here as tempfile::tempdir creates directory and deletes it when the destructor is run.
    let batch_input_file_path = tmpdir.path().join("test_batch_write.json");
    let mut f = File::create(tmpdir.path().join("test_batch_write.json"))?;
    f.write_all(b"
    {
        \"table--test_batch_write\": [
            {
                \"PutRequest\": {
                    \"Item\": {
                        \"pk\": { \"S\": \"ichi\" },
                        \"ISBN\": { \"S\": \"111-1111111111\" },
                        \"Price\": { \"N\": \"2\" },
                        \"Dimensions\": { \"SS\": [\"Giraffe\", \"Hippo\" ,\"Zebra\"] },
                        \"PageCount\": { \"NS\": [\"42.2\", \"-19\", \"7.5\", \"3.14\"] },
                        \"InPublication\": { \"BOOL\": false },
                        \"Nothing\": { \"NULL\": true },
                        \"Authors\": {
                            \"L\": [
                                { \"S\": \"Author1\" },
                                { \"S\": \"Author2\" },
                                { \"N\": \"42\" }
                            ]
                        },
                        \"Details\": {
                            \"M\": {
                                \"Name\": { \"S\": \"Joe\" },
                                \"Age\":  { \"N\": \"35\" },
                                \"Misc\": {
                                    \"M\": {
                                        \"hope\": { \"BOOL\": true },
                                        \"dream\": { \"L\": [ { \"N\": \"35\" }, { \"NULL\": true } ] }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        ]
    }
    ")?;
    let mut c = util::setup().await?;
    c.args(&[
        "--region",
        "local",
        "--table",
        table_name,
        "bwrite",
        "--input",
        batch_input_file_path.to_str().unwrap(),
    ])
    .output()?;

    let mut c = util::setup().await?;
    let scan_cmd = c.args(&["--region", "local", "--table", table_name, "scan"]);
    scan_cmd
        .assert()
        .success()
        .stdout(predicate::str::is_match("pk *attributes\nichi").unwrap());

    /*
    get output should looks like:
        $ dy --region local -t table--test_batch_write get ichi
        {
          "Dimensions": [
            "Giraffe",
            "Hippo",
            "Zebra"
          ],
          "PageCount": [
            -19.0,
            3.14,
            7.5,
            42.2
          ],
          "Authors": [
            "Author1",
            "Author2",
            42
          ],
          "InPublication": false,
          "Nothing": null,
          "Price": 2,
          "pk": "ichi",
          "Details": {
            "Age": 35,
            "Misc": {
              "dream": [
                35,
                null
              ],
              "hope": true
            },
            "Name": "Joe"
          },
          "ISBN": "111-1111111111"
        }
    */
    let mut c = util::setup().await?;
    let get_cmd = c.args(&["--region", "local", "--table", table_name, "get", "ichi"]);
    let output = get_cmd.output()?.stdout;

    // more verification would be nice
    assert_eq!(
        true,
        predicate::str::is_match("\"Dimensions\":")?.eval(String::from_utf8(output)?.as_str())
    );

    util::cleanup(vec![table_name]).await
}

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
