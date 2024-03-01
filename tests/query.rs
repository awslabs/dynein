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
async fn test_simple_query() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,N"),
            vec![
                util::TemporaryItem::new("abc", Some("1"), None),
                util::TemporaryItem::new("abc", Some("2"), None),
            ],
        )
        .await?;

    let mut c = tm.command()?;
    let query_cmd = c.args(&["--region", "local", "--table", &table_name, "query", "abc"]);
    query_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "pk   sk  attributes\nabc  1\nabc  2",
        ));

    Ok(())
}

#[tokio::test]
async fn test_simple_desc_query() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,N"),
            vec![
                util::TemporaryItem::new("abc", Some("1"), None),
                util::TemporaryItem::new("abc", Some("2"), None),
            ],
        )
        .await?;

    let mut c = tm.command()?;
    let query_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "query",
        "abc",
        "-d",
    ]);
    query_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "pk   sk  attributes\nabc  2\nabc  1",
        ));

    Ok(())
}

#[tokio::test]
async fn test_query_limit() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,N"),
            vec![
                util::TemporaryItem::new("abc", Some("1"), None),
                util::TemporaryItem::new("abc", Some("2"), None),
            ],
        )
        .await?;

    let mut c = tm.command()?;
    let query_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "query",
        "abc",
        "-l",
        "1",
    ]);
    query_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("pk   sk  attributes\nabc  1"));

    Ok(())
}

#[tokio::test]
async fn test_query_with_sort_key() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,N"),
            vec![
                util::TemporaryItem::new("abc", Some("1"), None),
                util::TemporaryItem::new("abc", Some("2"), None),
                util::TemporaryItem::new("abc", Some("3"), None),
            ],
        )
        .await?;

    let mut c = tm.command()?;
    let query_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "query",
        "abc",
        "-s",
        "2",
    ]);
    query_cmd.assert().success().stdout(
        predicate::str::is_match("pk +sk +attributes\n")
            .unwrap()
            .and(predicate::str::is_match("abc +1\n").unwrap().not())
            .and(predicate::str::is_match("abc +2\n").unwrap())
            .and(predicate::str::is_match("abc +3\n").unwrap().not()),
    );

    Ok(())
}

#[tokio::test]
async fn test_query_with_sort_key_order() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,N"),
            vec![
                util::TemporaryItem::new("abc", Some("1"), None),
                util::TemporaryItem::new("abc", Some("2"), None),
                util::TemporaryItem::new("abc", Some("3"), None),
            ],
        )
        .await?;

    for _ in 1..10 {
        let mut c = tm.command()?;
        let query_cmd = c.args(&[
            "--region",
            "local",
            "--table",
            &table_name,
            "query",
            "abc",
            "-s",
            "< 5",
        ]);
        query_cmd.assert().success().stdout(
            predicate::str::is_match("pk +sk +attributes\nabc +1\nabc +2\nabc +3\n").unwrap(),
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_query_with_sort_key_le() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,N"),
            vec![
                util::TemporaryItem::new("abc", Some("1"), None),
                util::TemporaryItem::new("abc", Some("2"), None),
                util::TemporaryItem::new("abc", Some("3"), None),
            ],
        )
        .await?;

    let mut c = tm.command()?;
    let query_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "query",
        "abc",
        "-s",
        "<=2",
    ]);
    query_cmd.assert().success().stdout(
        predicate::str::is_match("pk +sk +attributes\n")
            .unwrap()
            .and(predicate::str::is_match("abc +1\n").unwrap())
            .and(predicate::str::is_match("abc +2\n").unwrap())
            .and(predicate::str::is_match("abc +3\n").unwrap().not()),
    );

    Ok(())
}

#[tokio::test]
async fn test_query_using_between_string() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,S"),
            vec![
                util::TemporaryItem::new("abc", Some("1"), None),
                util::TemporaryItem::new("abc", Some("11"), None),
                util::TemporaryItem::new("abc", Some("2"), None),
                util::TemporaryItem::new("abc", Some("21"), None),
                util::TemporaryItem::new("abc", Some("22"), None),
            ],
        )
        .await?;

    let mut c = tm.command()?;
    let query_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "query",
        "abc",
        "-s",
        "between 11 21",
    ]);
    query_cmd.assert().success().stdout(
        predicate::str::is_match("pk +sk +attributes\n")
            .unwrap()
            .and(predicate::str::is_match("abc +1\n").unwrap().not())
            .and(predicate::str::is_match("abc +11\n").unwrap())
            .and(predicate::str::is_match("abc +2\n").unwrap())
            .and(predicate::str::is_match("abc +21\n").unwrap())
            .and(predicate::str::is_match("abc +22\n").unwrap().not()),
    );
    Ok(())
}

#[tokio::test]
async fn test_query_using_between_number() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,N"),
            vec![
                util::TemporaryItem::new("abc", Some("1"), None),
                util::TemporaryItem::new("abc", Some("11"), None),
                util::TemporaryItem::new("abc", Some("2"), None),
                util::TemporaryItem::new("abc", Some("21"), None),
                util::TemporaryItem::new("abc", Some("22"), None),
            ],
        )
        .await?;

    let mut c = tm.command()?;
    let query_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "query",
        "abc",
        "-s",
        "between 11 21",
    ]);
    query_cmd.assert().success().stdout(
        predicate::str::is_match("pk +sk +attributes\n")
            .unwrap()
            .and(predicate::str::is_match("abc +1\n").unwrap().not())
            .and(predicate::str::is_match("abc +11\n").unwrap())
            .and(predicate::str::is_match("abc +2\n").unwrap().not())
            .and(predicate::str::is_match("abc +21\n").unwrap())
            .and(predicate::str::is_match("abc +22\n").unwrap().not()),
    );
    Ok(())
}

#[tokio::test]
async fn test_query_using_begins_with() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,S"),
            vec![
                util::TemporaryItem::new("abc", Some("1"), None),
                util::TemporaryItem::new("abc", Some("11"), None),
                util::TemporaryItem::new("abc", Some("21"), None),
                util::TemporaryItem::new("abc", Some("22"), None),
            ],
        )
        .await?;

    let mut c = tm.command()?;
    let query_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "query",
        "abc",
        "-s",
        "begins_with 1",
    ]);
    query_cmd.assert().success().stdout(
        predicate::str::is_match("pk +sk +attributes\n")
            .unwrap()
            .and(predicate::str::is_match("abc +1\n").unwrap())
            .and(predicate::str::is_match("abc +11\n").unwrap())
            .and(predicate::str::is_match("abc +21\n").unwrap().not())
            .and(predicate::str::is_match("abc +22\n").unwrap().not()),
    );
    Ok(())
}

#[tokio::test]
async fn test_query_invalid_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,S"),
            vec![util::TemporaryItem::new("abc", Some("2"), None)],
        )
        .await?;

    let mut c = tm.command()?;
    let query_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "query",
        "abc",
        "-s",
        r#"= 3*"2""#,
    ]);
    query_cmd
        .assert()
        .failure()
        .stderr(predicate::str::contains("= expected sort_key_str"));
    Ok(())
}

#[tokio::test]
async fn test_query_with_strict_mode_with_suggestion() -> Result<(), Box<dyn std::error::Error>> {
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,S"),
            vec![util::TemporaryItem::new("abc", Some("2"), None)],
        )
        .await?;

    let mut c = tm.command()?;
    let query_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "query",
        "--strict",
        "abc",
        "-s",
        "=2",
    ]);
    query_cmd.assert().failure().stderr(
        predicate::str::contains(
            "Invalid type detected. Expected type is string (S), but actual type is number (N).",
        )
        .and(predicate::str::contains(r#"Did you intend '= "2"'?"#)),
    );
    Ok(())
}

#[tokio::test]
async fn test_query_with_strict_mode_without_suggestion() -> Result<(), Box<dyn std::error::Error>>
{
    let mut tm = util::setup().await?;
    let table_name = tm
        .create_temporary_table_with_items(
            "pk",
            Some("sk,N"),
            vec![util::TemporaryItem::new("abc", Some("8"), None)],
        )
        .await?;

    let mut c = tm.command()?;
    let query_cmd = c.args(&[
        "--region",
        "local",
        "--table",
        &table_name,
        "query",
        "--strict",
        "abc",
        "-s",
        r#"= "2*4""#,
    ]);
    query_cmd.assert().failure().stderr(
        predicate::str::contains(
            "Invalid type detected. Expected type is number (N), but actual type is string (S).",
        )
        .and(predicate::str::contains(r#"Did you intend"#).not()),
    );
    Ok(())
}
