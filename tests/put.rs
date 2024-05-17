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
use serde_json::json;

#[tokio::test]
async fn test_put_non_existent_table() -> Result<(), Box<dyn std::error::Error>> {
    let tm = util::setup().await?;
    let mut c = tm.command()?;

    let cmd = c.args([
        "--region",
        "local",
        "--table",
        "dummy-table-doesnt-exist",
        "put",
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
async fn test_put() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;

    let expected = r#"
    [{
        "pk": {
            "S": "42"
        }
    }]
    "#;
    for aciton in ["put", "p"] {
        let table_name = tm.create_temporary_table("pk", None).await?;

        let mut c = tm.command()?;
        let cmd = c.args(["--region", "local", "--table", &table_name, aciton, "42"]);
        cmd.assert().success().stdout(format!(
            "Successfully put an item to the table '{}'.\n",
            table_name
        ));

        let mut c = tm.command()?;
        let get_cmd = c.args([
            "--region",
            "local",
            "--table",
            &table_name,
            "scan",
            "-o",
            "raw",
        ]);

        util::assert_eq_cmd_json(get_cmd, expected);
    }

    Ok(())
}

#[tokio::test]
async fn test_put_sk() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", Some("sk")).await?;

    let mut c = tm.command()?;
    let cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "put",
        "42",
        "abc",
    ]);
    cmd.assert().success().stdout(format!(
        "Successfully put an item to the table '{}'.\n",
        table_name
    ));

    let mut c = tm.command()?;
    let get_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "raw",
    ]);

    let expected = r#"
      [{
          "sk": {
              "S": "abc"
          },
          "pk": {
              "S": "42"
          }
      }]
      "#;

    util::assert_eq_json_ignore_order(get_cmd, expected);
    Ok(())
}

#[tokio::test]
async fn test_put_missing_sk() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", Some("sk")).await?;

    let mut c = tm.command()?;
    let cmd = c.args(["--region", "local", "--table", &table_name, "put", "42"]);
    cmd.assert().failure().stderr(predicate::str::contains(
        "One of the required keys was not given a value",
    ));
    Ok(())
}

#[tokio::test]
async fn test_put_item() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;

    let mut c = tm.command()?;
    let cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "put",
        "42",
        "--item",
        r#"{"a": 9, "b": "str"}"#,
    ]);
    cmd.assert().success().stdout(format!(
        "Successfully put an item to the table '{}'.\n",
        table_name
    ));

    let mut c = tm.command()?;
    let get_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "raw",
    ]);

    let expected = r#"
      [{
          "pk": {
              "S": "42"
          },
          "a": {
              "N": "9"
          },
          "b": {
              "S": "str"
          }
      }]
      "#;

    util::assert_eq_json_ignore_order(get_cmd, expected);
    Ok(())
}

#[tokio::test]
async fn test_put_sk_item() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", Some("sk")).await?;

    let mut c = tm.command()?;
    let cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "put",
        "42",
        "abc",
        "--item",
        r#"{"a": 9, "b": "str"}"#,
    ]);
    cmd.assert().success().stdout(format!(
        "Successfully put an item to the table '{}'.\n",
        table_name
    ));

    let mut c = tm.command()?;
    let get_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "raw",
    ]);

    let expected = r#"
      [{
          "pk": {
              "S": "42"
          },
          "sk": {
              "S": "abc"
          },
          "a": {
              "N": "9"
          },
          "b": {
              "S": "str"
          }
      }]
      "#;

    util::assert_eq_json_ignore_order(get_cmd, expected);
    Ok(())
}

#[tokio::test]
async fn test_put_complex_item() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;

    let mut c = tm.command()?;
    let cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "put",
        "42",
        "--item",
        r#"{"myfield": "is", "nested": {"can": true, "go": false, "deep": [1,2,{"this_is_set": <<"x","y","z">>}]}}"#,
    ]);
    cmd.assert().success().stdout(format!(
        "Successfully put an item to the table '{}'.\n",
        table_name
    ));

    let mut c = tm.command()?;
    let get_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "raw",
    ]);

    let expected = r#"
        [{
          "pk": {
              "S": "42"
          },
          "myfield": {
              "S": "is"
          },
          "nested": {
              "M": {
                  "can": {
                      "BOOL": true
                  },
                  "deep": {
                      "L": [{
                          "N": "1"
                      }, {
                          "N": "2"
                      }, {
                          "M": {
                              "this_is_set": {
                                  "SS": ["x", "y", "z"]
                              }
                          }
                      }]
                  },
                  "go": {
                      "BOOL": false
                  }
              }
          }
      }]
      "#;

    util::assert_eq_json_ignore_order(get_cmd, expected);
    Ok(())
}

#[tokio::test]
async fn test_put_same_pk() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            None,
            vec![util::TemporaryItem::new(
                "42",
                None,
                Some(r#"{"null-field": null}"#),
            )],
        )
        .await?;

    let mut c = tm.command()?;
    let cmd = c.args(["--region", "local", "--table", &table_name, "put", "42"]);
    cmd.assert().success().stdout(format!(
        "Successfully put an item to the table '{}'.\n",
        table_name
    ));

    let mut c = tm.command()?;
    let get_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "raw",
    ]);

    let expected = r#"
    [{
        "pk": {
            "S": "42"
        }
    }]
    "#;

    util::assert_eq_cmd_json(get_cmd, &expected);
    Ok(())
}

#[tokio::test]
async fn test_multiple_put_same_pk() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;

    for _ in 1..10 {
        let mut c = tm.command()?;
        let cmd = c.args(["--region", "local", "--table", &table_name, "put", "42"]);
        cmd.assert().success().stdout(format!(
            "Successfully put an item to the table '{}'.\n",
            table_name
        ));
    }

    let mut c = tm.command()?;
    let get_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "raw",
    ]);

    let expected = r#"
    [{
        "pk": {
            "S": "42"
        }
    }]
    "#;

    util::assert_eq_cmd_json(get_cmd, &expected);
    Ok(())
}

#[tokio::test]
async fn test_multiple_put_different_pk() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;

    let mut items = Vec::new();
    for pk in 1..10 {
        let mut c = tm.command()?;
        let cmd = c.args([
            "--region",
            "local",
            "--table",
            &table_name,
            "put",
            &pk.to_string(),
        ]);
        cmd.assert().success().stdout(format!(
            "Successfully put an item to the table '{}'.\n",
            table_name
        ));

        items.push(json!({
            "pk": {
                "S": pk.to_string()
            }
        }));
    }

    let mut c = tm.command()?;
    let get_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "raw",
    ]);

    let expected = json!(items).to_string();
    util::assert_eq_json_ignore_order(get_cmd, &expected);
    Ok(())
}

#[tokio::test]
async fn test_multiple_put_same_pk_different_sk() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", Some("sk")).await?;

    let mut items = Vec::new();
    for sk in 1..10 {
        let mut c = tm.command()?;
        let cmd = c.args([
            "--region",
            "local",
            "--table",
            &table_name,
            "put",
            "42",
            &sk.to_string(),
        ]);
        cmd.assert().success().stdout(format!(
            "Successfully put an item to the table '{}'.\n",
            table_name
        ));

        items.push(json!({
            "pk": {
                "S": "42"
            },
            "sk": {
                "S":sk.to_string()
            }
        }));
    }

    let mut c = tm.command()?;
    let get_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "raw",
    ]);

    let expected = json!(items).to_string();
    util::assert_eq_json_ignore_order(get_cmd, &expected);
    Ok(())
}
