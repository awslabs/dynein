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
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::Builder;

#[tokio::test]
async fn test_batch_write_json_put() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;

    for action in ["bwrite", "batch-write-item", "bw"] {
        let table_name = tm.create_temporary_table("pk", None).await?;

        let tmpdir = Builder::new().tempdir()?;
        let batch_input_file_path = create_test_json_file(
            "tests/resources/test_batch_write_put.json",
            vec![&table_name],
            &tmpdir,
        );

        let mut c = tm.command()?;
        c.args([
            "--region",
            "local",
            action,
            "--input",
            &batch_input_file_path,
        ])
        .assert()
        .success();

        let mut c = tm.command()?;
        let scan_cmd = c.args([
            "--region",
            "local",
            "--table",
            &table_name,
            "scan",
            "-o",
            "raw",
        ]);

        let expected_json =
            std::fs::read_to_string("tests/resources/test_batch_write_put_output.json")?;

        util::assert_eq_json_ignore_order(scan_cmd, expected_json.as_str());
    }

    Ok(())
}

#[tokio::test]
async fn test_batch_write_json_delete() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            None,
            [util::TemporaryItem::new(
                "ichi",
                None,
                Some(r#"{"null-field": null}"#),
            )],
        )
        .await?;

    let table_name_sk = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk"),
            [util::TemporaryItem::new(
                "ichi",
                Some("sortkey"),
                Some(r#"{"null-field": null}"#),
            )],
        )
        .await?;

    let tmpdir = Builder::new().tempdir()?;
    let batch_input_file_path = create_test_json_file(
        "tests/resources/test_batch_write_delete.json",
        vec![&table_name, &table_name_sk],
        &tmpdir,
    );

    let mut c = tm.command()?;
    c.args([
        "--region",
        "local",
        "bwrite",
        "--input",
        &batch_input_file_path,
    ])
    .assert()
    .success();

    let mut c = tm.command()?;
    let scan_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "raw",
    ]);
    scan_cmd.assert().success().stdout("[]\n");

    let mut c = tm.command()?;
    let scan_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name_sk,
        "scan",
        "-o",
        "raw",
    ]);
    scan_cmd.assert().success().stdout("[]\n");

    Ok(())
}

#[tokio::test]
async fn test_batch_write_json_put_delete() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            None,
            [util::TemporaryItem::new(
                "ichi",
                None,
                Some(r#"{"null-field": null}"#),
            )],
        )
        .await?;

    let tmpdir = Builder::new().tempdir()?;
    let batch_input_file_path = create_test_json_file(
        "tests/resources/test_batch_write_put_delete.json",
        vec![&table_name],
        &tmpdir,
    );

    let mut c = tm.command()?;
    c.args([
        "--region",
        "local",
        "bwrite",
        "--input",
        &batch_input_file_path,
    ])
    .assert()
    .success();

    let mut c = tm.command()?;
    let scan_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "json",
    ]);
    scan_cmd.assert().success().stdout(
        predicate::str::is_match(r#""pk": "ni""#)?.and(predicate::str::is_match(r#""pk": "san""#)?),
    );

    Ok(())
}

#[tokio::test]
async fn test_batch_write_json_put_delete_multiple_tables() -> Result<(), Box<dyn std::error::Error>>
{
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            None,
            [util::TemporaryItem::new(
                "ichi",
                None,
                Some(r#"{"null-field": null}"#),
            )],
        )
        .await?;

    let table_name2 = tm
        .create_temporary_table_with_items(
            "pk",
            None,
            [util::TemporaryItem::new(
                "ichi",
                None,
                Some(r#"{"null-field": null}"#),
            )],
        )
        .await?;

    let tmpdir = Builder::new().tempdir()?;
    let batch_input_file_path = create_test_json_file(
        "tests/resources/test_batch_write_put_delete_multiple_tables.json",
        vec![&table_name, &table_name2],
        &tmpdir,
    );

    let mut c = tm.command()?;
    c.args([
        "--region",
        "local",
        "bwrite",
        "--input",
        &batch_input_file_path,
    ])
    .assert()
    .success();

    for table in [&table_name, &table_name2] {
        let mut c = tm.command()?;
        let scan_cmd = c.args(["--region", "local", "--table", table, "scan", "-o", "json"]);
        scan_cmd.assert().success().stdout(
            predicate::str::is_match(r#""pk": "ni""#)?
                .and(predicate::str::is_match(r#""pk": "san""#)?),
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_batch_write_put() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;

    let mut c = tm.command()?;
    c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "bwrite",
        "--put",
        r#"{"pk": "11",
        "null-field": null,
        "list-field": [1, 2, 3, "str"],
        "map-field": {"l0": <<1, 2>>, "l1": <<"str1", "str2">>, "l2": true},
        "binary-field": b"\x00",
        "binary-set-field": <<b"\x01", b"\x02">>}"#,
    ])
    .assert()
    .success();

    let mut c = tm.command()?;
    let get_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "get",
        "11",
        "-o",
        "raw",
    ]);

    let expected = r#"
        {
            "pk": { "S": "11" },
            "binary-set-field": { "BS": ["AQ==", "Ag=="] },
            "list-field": {
                "L": [
                    { "N": "1" },
                    { "N": "2" },
                    { "N": "3" },
                    { "S": "str" }
                ]
            },
            "map-field": {
                "M": {
                    "l0": { "NS": ["1", "2"] },
                    "l1": { "SS": ["str1", "str2"] },
                    "l2": { "BOOL": true }
                }
            },
            "null-field": { "NULL": true },
            "binary-field": { "B": "AA==" }
        }
        "#;

    util::assert_eq_json_ignore_order(get_cmd, expected);

    Ok(())
}

#[tokio::test]
async fn test_batch_write_put_sk() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", Some("sk")).await?;

    let mut c = tm.command()?;
    c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "bwrite",
        "--put",
        r#"{"pk": "11", "sk": "111"}"#,
    ])
    .assert()
    .success();

    let mut c = tm.command()?;
    let get_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "get",
        "11",
        "111",
        "-o",
        "json",
    ]);
    get_cmd.assert().success().stdout(
        predicate::str::is_match(r#""pk": "11""#)?.and(predicate::str::is_match(r#""sk": "111""#)?),
    );

    Ok(())
}

#[tokio::test]
async fn test_batch_write_del() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            None,
            [util::TemporaryItem::new(
                "11",
                None,
                Some(r#"{"null-field": null}"#),
            )],
        )
        .await?;

    let mut c = tm.command()?;
    c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "bwrite",
        "--del",
        r#"{"pk": "11"}"#,
    ])
    .assert()
    .success();

    let mut c = tm.command()?;
    let scan_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "raw",
    ]);
    scan_cmd.assert().success().stdout("[]\n");

    Ok(())
}

#[tokio::test]
async fn test_batch_write_del_sk() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk"),
            [util::TemporaryItem::new(
                "11",
                Some("111"),
                Some(r#"{"null-field": null}"#),
            )],
        )
        .await?;

    let mut c = tm.command()?;
    c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "bwrite",
        "--del",
        r#"{"pk": "11", "sk": "111"}"#,
    ])
    .assert()
    .success();

    let mut c = tm.command()?;
    let scan_cmd = c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "raw",
    ]);
    scan_cmd.assert().success().stdout("[]\n");

    Ok(())
}

#[tokio::test]
async fn test_batch_write_all_options() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            None,
            [
                util::TemporaryItem::new("11", None, Some(r#"{"null-field": null}"#)),
                util::TemporaryItem::new("ichi", None, Some(r#"{"null-field": null}"#)),
            ],
        )
        .await?;

    let tmpdir = Builder::new().tempdir()?;
    let batch_input_file_path = create_test_json_file(
        "tests/resources/test_batch_write_put_delete.json",
        vec![&table_name],
        &tmpdir,
    );
    let mut c = tm.command()?;
    c.args([
        "--region",
        "local",
        "--table",
        &table_name,
        "bwrite",
        "--del",
        r#"{"pk": "11"}"#,
        "--input",
        &batch_input_file_path,
        "--put",
        r#"{"pk": "12", "null-field": null}"#,
    ])
    .assert()
    .success();

    let mut c = tm.command()?;
    let scan_cmd = c
        .args([
            "--region",
            "local",
            "--table",
            &table_name,
            "scan",
            "-o",
            "json",
        ])
        .assert()
        .success();
    let output = scan_cmd.get_output().stdout.to_owned();
    let output_str = String::from_utf8(output)?;

    // Check if the first item has been deleted
    assert!(!predicate::str::is_match(r#""pk": "11""#)?.eval(&output_str));
    assert!(!predicate::str::is_match(r#""pk": "ichi""#)?.eval(&output_str));
    // Check if the json item put exists
    assert!(predicate::str::is_match(r#""pk": "12""#)?.eval(&output_str));
    // Check if the command inputs exists
    assert!(predicate::str::is_match(r#""pk": "ni""#)?.eval(&output_str));
    assert!(predicate::str::is_match(r#""pk": "san""#)?.eval(&output_str));

    Ok(())
}

fn create_test_json_file(
    json_path: &str,
    table_names: Vec<&String>,
    tmpdir: &tempfile::TempDir,
) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(json_path);

    let mut test_json_content = std::fs::read_to_string(&path).unwrap();
    let file_name = path.file_name().unwrap();

    let batch_input_file_path = tmpdir.path().join(file_name);
    let mut f = File::create(&batch_input_file_path).unwrap();
    for (i, tbn) in table_names.iter().enumerate() {
        test_json_content = test_json_content.replace(&format!("__TABLE_NAME__{}", i + 1), tbn);
    }

    f.write_all(test_json_content.as_bytes()).unwrap();

    batch_input_file_path.to_str().unwrap().to_owned()
}
