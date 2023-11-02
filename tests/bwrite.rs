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
use base64::{engine::general_purpose, Engine as _};
use predicates::prelude::*; // Used for writing assertions
use serde_json::Value;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::Builder;

#[tokio::test]
async fn test_batch_write() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;

    let tmpdir = Builder::new().tempdir().unwrap(); // defining stand alone variable here as tempfile::tempdir creates directory and deletes it when the destructor is run.
    let batch_input_file_path = create_test_json_file(
        "tests/resources/test_batch_write.json",
        &table_name,
        &tmpdir,
    );

    let mut c = tm.command()?;
    c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "bwrite",
        "--input",
        &batch_input_file_path,
    ])
    .output()?;

    let mut c = tm.command()?;
    let scan_cmd = c.args(&["--region", "local", "--table", &table_name, "scan"]);
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
    let mut c = tm.command()?;
    let get_cmd = c.args(&["--region", "local", "--table", &table_name, "get", "ichi"]);
    let output = get_cmd.output()?.stdout;

    // more verification would be nice
    assert_eq!(
        true,
        predicate::str::is_match("\"Dimensions\":")?.eval(String::from_utf8(output)?.as_str())
    );

    let mut c = tm.command()?;
    let get_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "get",
        "ichi",
        "-o",
        "raw",
    ]);

    let output = get_cmd.output()?.stdout;
    let data: Value = serde_json::from_str(&String::from_utf8(output)?)?;

    let binary = data["Binary"]["B"].as_str().unwrap();
    assert_eq!(binary, "dGhpcyB0ZXh0IGlzIGJhc2U2NC1lbmNvZGVk");

    // The order of the values within a set is not preserved, so I will use HashSet to compare them.
    let binary_set: HashSet<String> = data["BinarySet"]["BS"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .map(|s| s.to_string())
        .collect();
    let binary_set_expected: HashSet<String> =
        HashSet::from(["U3Vubnk=", "UmFpbnk=", "U25vd3k="].map(String::from));
    assert_eq!(binary_set, binary_set_expected);

    Ok(())
}

#[tokio::test]
async fn test_batch_write_put() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", None).await?;

    let mut c = tm.command()?;
    c.args(&[
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
    .output()?;

    let mut c = tm.command()?;
    let get_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "get",
        "11",
        "-o",
        "raw",
    ]);
    let output = get_cmd.output()?.stdout;
    let data: Value = serde_json::from_str(&String::from_utf8(output)?)?;
    assert_eq!(data["pk"]["S"], "11");
    assert_eq!(data["null-field"]["NULL"], true);
    assert_eq!(data["list-field"]["L"][0]["N"], "1");
    assert_eq!(data["list-field"]["L"][1]["N"], "2");
    assert_eq!(data["list-field"]["L"][2]["N"], "3");
    assert_eq!(data["list-field"]["L"][3]["S"], "str");
    assert_eq!(data["map-field"]["M"]["l0"]["NS"][0], "1");
    assert_eq!(data["map-field"]["M"]["l0"]["NS"][1], "2");
    assert_eq!(data["map-field"]["M"]["l1"]["SS"][0], "str1");
    assert_eq!(data["map-field"]["M"]["l1"]["SS"][1], "str2");
    assert_eq!(data["map-field"]["M"]["l2"]["BOOL"], true);
    assert_eq!(
        data["binary-field"]["B"],
        general_purpose::STANDARD.encode(b"\x00")
    );
    assert_eq!(
        data["binary-set-field"]["BS"][0],
        general_purpose::STANDARD.encode(b"\x01")
    );
    assert_eq!(
        data["binary-set-field"]["BS"][1],
        general_purpose::STANDARD.encode(b"\x02")
    );

    Ok(())
}

#[tokio::test]
async fn test_batch_write_put_sk() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm.create_temporary_table("pk", Some("sk")).await?;

    let mut c = tm.command()?;
    c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "bwrite",
        "--put",
        r#"{"pk": "11", "sk": "111"}"#,
    ])
    .output()?;

    let mut c = tm.command()?;
    let get_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "get",
        "11",
        "111",
        "-o",
        "raw",
    ]);
    let output = get_cmd.output()?.stdout;
    let data: Value = serde_json::from_str(&String::from_utf8(output)?)?;
    assert_eq!(data["pk"]["S"], "11");
    assert_eq!(data["sk"]["S"], "111");

    Ok(())
}

#[tokio::test]
async fn test_batch_write_del() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            None,
            vec![util::TemporaryItem::new(
                "11",
                None,
                Some(r#"{"null-field": null}"#),
            )],
        )
        .await?;

    let mut c = tm.command()?;
    c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "bwrite",
        "--del",
        r#"{"pk": "11"}"#,
    ])
    .output()?;

    let mut c = tm.command()?;
    let scan_cmd = c.args(&[
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
            vec![util::TemporaryItem::new(
                "11",
                Some("111"),
                Some(r#"{"null-field": null}"#),
            )],
        )
        .await?;

    let mut c = tm.command()?;
    c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "bwrite",
        "--del",
        r#"{"pk": "11", "sk": "111"}"#,
    ])
    .output()?;

    let mut c = tm.command()?;
    let scan_cmd = c.args(&[
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
            vec![util::TemporaryItem::new(
                "11",
                None,
                Some(r#"{"null-field": null}"#),
            )],
        )
        .await?;

    let tmpdir = Builder::new().tempdir().unwrap(); // defining stand alone variable here as tempfile::tempdir creates directory and deletes it when the destructor is run.
    let batch_input_file_path = create_test_json_file(
        "tests/resources/test_batch_write.json",
        &table_name,
        &tmpdir,
    );
    let mut c = tm.command()?;
    c.args(&[
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
    .output()?;

    let mut c = tm.command()?;
    let scan_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "scan",
        "-o",
        "json",
    ]);
    let output = scan_cmd.output()?.stdout;
    let output_str = String::from_utf8(output)?;
    // Check if the first item put has been deleted
    assert_eq!(
        false,
        predicate::str::is_match(r#""pk": "11""#)?.eval(&output_str)
    );
    // Check if the json file input exists
    assert_eq!(
        true,
        predicate::str::is_match(r#""pk": "ichi""#)?.eval(&output_str)
    );
    // Check if the second item put exists
    assert_eq!(
        true,
        predicate::str::is_match(r#"pk": "12""#)?.eval(&output_str)
    );

    Ok(())
}

fn create_test_json_file(
    json_path: &str,
    table_name: &String,
    tmpdir: &tempfile::TempDir,
) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(json_path);
    let test_json_content = std::fs::read_to_string(&path).unwrap();
    let file_name = path.file_name().unwrap();

    let batch_input_file_path = tmpdir.path().join(file_name);
    let mut f = File::create(&batch_input_file_path).unwrap();
    f.write_all(
        test_json_content
            .replace("__TABLE_NAME__", &table_name)
            .as_bytes(),
    )
    .unwrap();

    batch_input_file_path.to_str().unwrap().to_owned()
}
