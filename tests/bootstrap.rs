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
async fn test_bootstrap() -> Result<(), Box<dyn std::error::Error>> {
    let mut c = util::setup().await?;
    let cmd = c.args(&["--region", "local", "bootstrap"]);
    cmd.assert().success().stdout(predicate::str::contains(
        "Now all tables have sample data. Try following commands to play with dynein. Enjoy!",
    ));

    let test_cases = vec![
        (
            vec!["--region", "local", "--table", "Forum", "get", "Amazon S3"],
            r#"{
                "Category": "Amazon Web Services",
                "Name": "Amazon S3"
             }"#,
        ),
        (
            vec![
                "--region",
                "local",
                "--table",
                "ProductCatalog",
                "get",
                "205",
            ],
            r#"{
                "Id": 205,
                "ProductCategory": "Bicycle",
                "Description": "205 Description",
                "Title": "18-Bike-204",
                "BicycleType": "Hybrid",
                "Brand": "Brand-Company C",
                "Price": 500,
                "Color": [
                  "Red",
                  "Black"
                ]
              }"#,
        ),
        (
            vec![
                "--region",
                "local",
                "--table",
                "Reply",
                "get",
                "Amazon DynamoDB#DynamoDB Thread 1",
                "2015-09-15T19:58:22.947Z",
            ],
            r#"{
                "PostedBy": "User A",
                "Id": "Amazon DynamoDB#DynamoDB Thread 1",
                "Message": "DynamoDB Thread 1 Reply 1 text",
                "ReplyDateTime": "2015-09-15T19:58:22.947Z"
              }"#,
        ),
        (
            vec![
                "--region",
                "local",
                "--table",
                "Thread",
                "get",
                "Amazon S3",
                "S3 Thread 1",
            ],
            r#"{
                "Views": 0,
                "Message": "S3 thread 1 message",
                "Answered": 0,
                "ForumName": "Amazon S3",
                "LastPostedDateTime": "2015-09-29T19:58:22.514Z",
                "Tags": [
                  "largeobjects",
                  "multipart upload"
                ],
                "Replies": 0,
                "LastPostedBy": "User A",
                "Subject": "S3 Thread 1"
              }
              "#,
        ),
    ];

    for (args, expected_json) in test_cases {
        let mut c = util::setup().await?;

        let cmd = c.args(&args);
        util::assert_eq_json(cmd, expected_json);
    }

    util::cleanup(vec!["Forum", "ProductCatalog", "Reply", "Thread"]).await
}

#[tokio::test]
async fn test_bootstrap_movie() -> Result<(), Box<dyn std::error::Error>> {
    let mut c = util::setup().await?;
    let cmd = c.args(&["--region", "local", "bootstrap", "--sample", "movie"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("All tables are in ACTIVE."));

    let mut c = util::setup().await?;
    let cmd = c.args(&[
        "--region",
        "local",
        "--table",
        "Movie",
        "get",
        "1933",
        "King Kong",
    ]);
    util::assert_eq_json(
        cmd,
        r#"
    {
        "year": 1933,
        "info": {
          "actors": [
            "Bruce Cabot",
            "Fay Wray",
            "Robert Armstrong"
          ],
          "directors": [
            "Ernest B. Schoedsack",
            "Merian C. Cooper"
          ],
          "genres": [
            "Adventure",
            "Fantasy",
            "Horror"
          ],
          "image_url": "http://ia.media-imdb.com/images/M/MV5BMTkxOTIxMDU2OV5BMl5BanBnXkFtZTcwNjM5NjQyMg@@._V1_SX400_.jpg",
          "plot": "A film crew goes to a tropical island for an exotic location shoot and discovers a colossal giant gorilla who takes a shine to their female blonde star.",
          "rank": 3551,
          "rating": 8,
          "release_date": "1933-03-07T00:00:00Z",
          "running_time_secs": 6000
        },
        "title": "King Kong"
      }
      "#,
    );

    util::cleanup(vec!["Movie"]).await
}
