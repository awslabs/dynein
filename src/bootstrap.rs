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

use std::{
    collections::HashMap,
    error, fmt,
    io::{Cursor, Error as IOError, Read},
    thread, time,
};

use aws_sdk_dynamodb::{
    operation::{batch_write_item::BatchWriteItemError, create_table::CreateTableError},
    types::{AttributeValue, PutRequest, WriteRequest},
};
use futures::future::join_all;
use log::debug;

use brotli::Decompressor;
use serde_json::Value as JsonValue;

use super::app;
use super::batch;
use super::control;
use super::data;

/* =================================================
struct / enum / const
================================================= */

#[derive(Debug)]
pub enum DyneinBootstrapError {
    LoadData(IOError),
    PraseJSON(serde_json::Error),
    ReqwestError(reqwest::Error),
    ZipError(zip::result::ZipError),
    BatchError(aws_sdk_dynamodb::error::SdkError<BatchWriteItemError>),
}
impl fmt::Display for DyneinBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            DyneinBootstrapError::LoadData(ref e) => e.fmt(f),
            DyneinBootstrapError::PraseJSON(ref e) => e.fmt(f),
            DyneinBootstrapError::ReqwestError(ref e) => e.fmt(f),
            DyneinBootstrapError::ZipError(ref e) => e.fmt(f),
            DyneinBootstrapError::BatchError(ref e) => e.fmt(f),
        }
    }
}
impl error::Error for DyneinBootstrapError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            DyneinBootstrapError::LoadData(ref e) => Some(e),
            DyneinBootstrapError::PraseJSON(ref e) => Some(e),
            DyneinBootstrapError::ReqwestError(ref e) => Some(e),
            DyneinBootstrapError::ZipError(ref e) => Some(e),
            DyneinBootstrapError::BatchError(ref e) => Some(e),
        }
    }
}
impl From<IOError> for DyneinBootstrapError {
    fn from(e: IOError) -> Self {
        Self::LoadData(e)
    }
}
impl From<serde_json::Error> for DyneinBootstrapError {
    fn from(e: serde_json::Error) -> Self {
        Self::PraseJSON(e)
    }
}
impl From<reqwest::Error> for DyneinBootstrapError {
    fn from(e: reqwest::Error) -> Self {
        Self::ReqwestError(e)
    }
}
impl From<zip::result::ZipError> for DyneinBootstrapError {
    fn from(e: zip::result::ZipError) -> Self {
        Self::ZipError(e)
    }
}
impl From<aws_sdk_dynamodb::error::SdkError<BatchWriteItemError>> for DyneinBootstrapError {
    fn from(e: aws_sdk_dynamodb::error::SdkError<BatchWriteItemError>) -> Self {
        Self::BatchError(e)
    }
}

/* =================================================
Public functions
================================================= */

pub fn list_samples() {
    let samples: Vec<&str> = vec![
        "public", // default
        "movie",
    ];
    for sample in samples {
        println!("{}", sample);
    }
}

pub async fn launch_sample(
    cx: &app::Context,
    sample: Option<String>,
) -> Result<(), DyneinBootstrapError> {
    match sample {
        None => launch_default_sample(cx).await,
        Some(s) => {
            if s == "public" {
                launch_default_sample(cx).await
            } else if s == "movie" {
                launch_movie_sample(cx).await
            } else {
                println!("Unknown sample name. Available samples are:");
                list_samples();
                std::process::exit(1);
            }
        }
    }
}

/* =================================================
Private functions
================================================= */

async fn launch_movie_sample(cx: &app::Context) -> Result<(), DyneinBootstrapError> {
    println!(
        "\
Bootstrapping - dynein will create 'Movie' table with official 'Movie' sample data:

'Movie' - composite primary key table
    year (N)
    title (S)

see https://github.com/awslabs/dynein#working-with-dynamodb-items for detail
"
    );

    // Step 1. create tables
    prepare_table(cx, "Movie", vec!["year,N", "title,S"].as_ref()).await;

    // Step 2. wait tables to be created and in ACTIVE status
    wait_table_creation(cx, vec!["Movie"]).await;

    // Step 3. decompress data
    let compressed_data = include_bytes!("./resources/bootstrap/moviedata.json.br");
    let cursor = Cursor::new(compressed_data);
    let mut decompressor = Decompressor::new(cursor, 4096);
    let mut content = String::new();
    decompressor.read_to_string(&mut content)?;

    // Step 4. load data into tables
    /*
    Array([
        Object({
            "rank": Number(4,),
            ...
    */
    let deserialized_json: JsonValue = serde_json::from_str(&content).unwrap();
    debug!("converted JSON: {:#?}", &deserialized_json);
    if !deserialized_json.is_array() {
        println!("target JSON should be an array.");
        std::process::exit(1);
    };
    let mut whole_items = deserialized_json.as_array().expect("is array").iter();

    let mut request_items;
    let mut write_requests;
    'whole: loop {
        request_items = HashMap::<String, Vec<WriteRequest>>::new();
        write_requests = Vec::<WriteRequest>::new();
        'batch: loop {
            match whole_items.next() {
                None => {
                    break 'whole;
                }
                Some(item) => {
                    let item_json = item
                        .as_object()
                        .expect("each item should be a valid JSON object.");
                    let item_attrval: HashMap<String, AttributeValue> = item_json
                        .iter()
                        .map(|(k, v)| {
                            (
                                String::from(k),
                                data::dispatch_jsonvalue_to_attrval(v, true),
                            )
                        })
                        .collect();
                    write_requests.push(
                        WriteRequest::builder()
                            .put_request(
                                PutRequest::builder()
                                    .set_item(Some(item_attrval))
                                    .build()
                                    .unwrap(),
                            )
                            .build(),
                    );
                    if write_requests.len() == 25 {
                        break 'batch;
                    };
                }
            }
        } // 'batch loop
        request_items.insert("Movie".to_string(), write_requests);
        batch::batch_write_until_processed(cx, request_items).await?;
    } // 'whole loop
    request_items.insert("Movie".to_string(), write_requests);
    batch::batch_write_until_processed(cx, request_items).await?;

    Ok(())
}

async fn launch_default_sample(cx: &app::Context) -> Result<(), DyneinBootstrapError> {
    println!(
        "\
Bootstrapping - dynein will create 4 sample tables defined here:
https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/AppendixSampleTables.html

'ProductCatalog' - simple primary key table
    Id (N)

'Forum' - simple primary key table
    Name (S)

'Thread' - composite primary key table
    ForumName (S)
    Subject (S)

'Reply' - composite primary key table, with GSI named 'PostedBy-Message-Index'
    Id (S)
    ReplyDateTime (S)
"
    );

    let tables = vec![
        ("ProductCatalog", vec!["Id,N"]),
        ("Forum", vec!["Name,S"]),
        ("Thread", vec!["ForumName,S", "Subject,S"]),
        ("Reply", vec!["Id,S", "ReplyDateTime,S"]),
    ];

    // Step 1. Create tables
    for (table_name, keys) in &tables {
        prepare_table(cx, table_name, keys).await
    }

    // Step 2. wait tables to be created and in ACTIVE status
    let creating_table_names: Vec<&str> = tables.iter().map(|pair| pair.0).collect();
    wait_table_creation(cx, creating_table_names).await;

    println!("Tables are ready and retrieved sample data locally. Now start writing data into samle tables...");
    for (table_name, _) in &tables {
        // Step 3. decompress data
        let compressed_data = match *table_name {
            "ProductCatalog" => &include_bytes!("./resources/bootstrap/ProductCatalog.json.br")[..],
            "Forum" => &include_bytes!("./resources/bootstrap/Forum.json.br")[..],
            "Thread" => &include_bytes!("./resources/bootstrap/Thread.json.br")[..],
            "Reply" => &include_bytes!("./resources/bootstrap/Reply.json.br")[..],
            _ => panic!("No such table name: {}", table_name),
        };
        let cursor = Cursor::new(compressed_data);
        let mut decompressor = Decompressor::new(cursor, 4096);
        let mut content = String::new();
        decompressor.read_to_string(&mut content)?;
        // Step 4. load data into tables
        let request_items = batch::build_batch_request_items_from_json(content.to_string())?;
        batch::batch_write_until_processed(cx, request_items).await?;
    }

    let region = cx.effective_region().await.to_string();

    println!(
        "\n\nNow all tables have sample data. Try following commands to play with dynein. Enjoy!"
    );
    println!("  $ dy --region {} ls", region);
    println!("  $ dy --region {} desc --table Thread", region);
    println!("  $ dy --region {} scan --table Thread", region);
    println!("  $ dy --region {} use --table Thread", region);
    println!("  $ dy scan");
    println!("\nAfter you 'use' a table like above, dynein assume you're using the same region & table, which info is stored at ~/.dynein/config.yml and ~/.dynein/cache.yml");
    println!(
        "Let's move on with the '{}' region you've just 'use'd...",
        region
    );
    println!("  $ dy scan --table Forum");
    println!("  $ dy scan -t ProductCatalog");
    println!("  $ dy get -t ProductCatalog 101");
    println!("  $ dy query -t Reply \"Amazon DynamoDB#DynamoDB Thread 2\"");
    println!("  $ dy query -t Reply \"Amazon DynamoDB#DynamoDB Thread 2\"  --sort-key \"begins_with 2015-10\"");
    Ok(())
}

async fn prepare_table(cx: &app::Context, table_name: &str, keys: &[&str]) {
    match control::create_table_api(
        cx,
        table_name.to_string(),
        keys.iter().map(|k| (*k).to_string()).collect(),
    )
    .await
    {
        Ok(desc) => {
            println!(
                "Started to create table '{}' in {} region. status: {}",
                table_name,
                cx.effective_region().await.as_ref(),
                desc.table_status.unwrap()
            );
        }
        Err(e) => match e.as_service_error() {
            Some(CreateTableError::ResourceInUseException(_)) => println!(
                "[skip] Table '{}' already exists in {} region, skipping to create new one.",
                table_name,
                cx.effective_region().await.as_ref()
            ),
            _ => {
                debug!("CreateTable API call got an error -- {:#?}", e);
                app::bye_with_sdk_error(1, e);
            }
        },
    }
}

async fn wait_table_creation(cx: &app::Context, mut processing_tables: Vec<&str>) {
    debug!("tables in progress: {:?}", processing_tables);
    loop {
        let create_table_results = join_all(
            processing_tables
                .iter()
                .map(|t| control::describe_table_api(cx, (*t).to_string())),
        )
        .await;
        let statuses: Vec<String> = create_table_results
            .iter()
            .map(|desc| desc.table_status.to_owned().unwrap().to_string())
            .collect();
        debug!("Current table statues: {:?}", statuses);
        processing_tables = processing_tables
            .iter()
            .zip(statuses.iter())
            .filter(|(_, s)| s.as_str() != "ACTIVE")
            .map(|(t, _)| *t)
            .collect();
        println!("Still CREATING following tables: {:?}", processing_tables);
        if processing_tables.is_empty() {
            println!("All tables are in ACTIVE.");
            break;
        }
        println!("Waiting for tables to be ACTIVE status...");
        thread::sleep(time::Duration::from_millis(5000));
    }
}
