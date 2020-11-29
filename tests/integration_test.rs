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

use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs
                           // use assert_cmd::cmd::Command; // Run programs - it seems to be equal to "use assert_cmd::prelude::* + use std::process::Command"

use std::fs::File;
use std::io::{self, Write}; // Used when check results by printing to stdout

use tempfile::Builder;

/// Integration tests would go with DynamoDB Local, so before running them setup() starts up DynamoDB Local with Docker.
/// FYI: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/DynamoDBLocal.html
///      https://hub.docker.com/r/amazon/dynamodb-local
fn setup() -> Result</* std::process::Command */ Command, Box<dyn std::error::Error>> {
    // NOTE: setup() spins up DynamoDB Local in 8000 port as "local" region assumes it. Better to make it configurable.
    let port = 8000;
    let mut docker = Command::new("docker");

    let check_cmd = docker.args(&[
        "ps",
        "-q",
        "--filter",
        &format!("expose={}", port),
        "--filter",
        "ancestor=amazon/dynamodb-local",
    ]);
    let check_out = check_cmd.output().expect("failed to execut check cmd");
    if check_out.stdout.len() != 0 {
        println!("DynamoDB Local is already running.");
        return Ok(Command::cargo_bin("dy")?);
    };

    // As docker run wouldn't wait for DynamoDB Local process to launch & accept API, first test would fail.
    // possible workwround would be checking if a process is running and skip `docker run` if needed.
    // To avoid this issue, I'd sleep for a while in buildspec.yml (CodeBuild configuration).
    let docker_run = docker.args(&["run",
                                   "-p", &format!("{}:{}", port, port),
                                   "-d", "amazon/dynamodb-local" /*, "-jar", "DynamoDBLocal.jar", "-inMemory", "-port", &format!("{}", port) */]);
    let output = docker_run
        .output()
        .expect("failed to running Docker image amazon/dynamodb-local in setup().");
    print!("DynamoDB Local is up as a container: ");
    io::stdout().write_all(&output.stdout).unwrap();

    Ok(Command::cargo_bin("dy")?)
}

fn cleanup(tables: Vec<&str>) -> Result<(), Box<dyn std::error::Error>> {
    for table in tables {
        let mut dynein_cmd = setup()?;
        let cmd = dynein_cmd.args(&[
            "--region", "local", "admin", "delete", "table", "--yes", table,
        ]);
        cmd.assert().success();
    }
    Ok(())
}

#[test]
fn test_help() -> Result<(), Box<dyn std::error::Error>> {
    setup()?;
    let mut dynein_cmd = Command::cargo_bin("dy")?;
    let cmd = dynein_cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("dynein is a command line tool"));
    Ok(())
}

#[test]
fn test_create_table() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_create_table";

    // $ dy admin create table <table_name> --keys pk
    let mut c = setup()?;
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
    let mut c = setup()?;
    let desc_cmd = c.args(&["--region", "local", "desc", table_name]);
    desc_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "name: {}\nregion: local",
            &table_name
        )));

    Ok(cleanup(vec![table_name])?)
}

#[test]
fn test_scan_non_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let mut c = setup()?;
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

#[test]
fn test_scan_blank_table() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_scan_blank_table";

    let mut c = setup()?;
    c.args(&[
        "--region", "local", "admin", "create", "table", table_name, "--keys", "pk",
    ])
    .output()?;
    let mut c = setup()?;
    let scan_cmd = c.args(&["--region", "local", "--table", table_name, "scan"]);
    scan_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("No item to show"));

    Ok(cleanup(vec![table_name])?)
}

#[test]
fn test_simple_scan() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_simple_scan";

    let mut c = setup()?;
    c.args(&[
        "--region", "local", "admin", "create", "table", table_name, "--keys", "pk",
    ])
    .output()?;
    let mut c = setup()?;
    c.args(&["--region", "local", "--table", table_name, "put", "abc"])
        .output()?;

    let mut c = setup()?;
    let scan_cmd = c.args(&["--region", "local", "--table", table_name, "scan"]);
    scan_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("pk  attributes\nabc"));

    Ok(cleanup(vec![table_name])?)
}

#[test]
fn test_batch_write() -> Result<(), Box<dyn std::error::Error>> {
    let table_name = "table--test_batch_write";

    let mut c = setup()?;
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
    let mut c = setup()?;
    c.args(&[
        "--region",
        "local",
        "--table",
        table_name,
        "bwrite",
        "--input",
        &batch_input_file_path.to_str().unwrap(),
    ])
    .output()?;

    let mut c = setup()?;
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
    let mut c = setup()?;
    let get_cmd = c.args(&["--region", "local", "--table", table_name, "get", "ichi"]);
    let output = get_cmd.output()?.stdout;

    // more verification would be nice
    assert_eq!(
        true,
        predicate::str::is_match("\"Dimensions\":")?.eval(String::from_utf8(output)?.as_str())
    );

    Ok(cleanup(vec![table_name])?)
}

#[test]
fn test_shell_mode() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{Seek, SeekFrom};

    let table_name = "table--test_shell_mode";

    // $ dy admin create table <table_name> --keys pk
    let mut c = setup()?;
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

    Ok(cleanup(vec![table_name])?)
}
