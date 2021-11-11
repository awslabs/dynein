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
    fs::{create_dir_all, read_to_string, File},
    io::{copy, Error as IOError, Write},
    path::PathBuf,
    thread, time,
};

use futures::future::join_all;
use log::{debug, error};
use rusoto_core::{Region, RusotoError};
use rusoto_dynamodb::*;
use tempfile::Builder;

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
    BatchError(RusotoError<BatchWriteItemError>),
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
    fn from(e: IOError) -> DyneinBootstrapError {
        DyneinBootstrapError::LoadData(e)
    }
}
impl From<serde_json::Error> for DyneinBootstrapError {
    fn from(e: serde_json::Error) -> DyneinBootstrapError {
        DyneinBootstrapError::PraseJSON(e)
    }
}
impl From<reqwest::Error> for DyneinBootstrapError {
    fn from(e: reqwest::Error) -> DyneinBootstrapError {
        DyneinBootstrapError::ReqwestError(e)
    }
}
impl From<zip::result::ZipError> for DyneinBootstrapError {
    fn from(e: zip::result::ZipError) -> DyneinBootstrapError {
        DyneinBootstrapError::ZipError(e)
    }
}
impl From<RusotoError<BatchWriteItemError>> for DyneinBootstrapError {
    fn from(e: RusotoError<BatchWriteItemError>) -> DyneinBootstrapError {
        DyneinBootstrapError::BatchError(e)
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
    cx: app::Context,
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

async fn launch_movie_sample(cx: app::Context) -> Result<(), DyneinBootstrapError> {
    println!(
        "\
Bootstrapping - dynein will creates 'Movie' table used in public tutorials:
e.g. https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/GettingStarted.NodeJs.02.html
     https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/GettingStarted.Ruby.02.html

'Movie' - composite primary key table
    year (N)
    title (S)
"
    );

    // Step 1. Create tables
    prepare_table(&cx, "Movie", vec!["year,N", "title,S"].as_ref()).await;

    // Step 2. Download & unzip data. The sampledata.zip contains 4 files.
    let url =
        "https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/samples/moviedata.zip";
    let download_dir: tempfile::TempDir = download_and_extract_zip(url).await?;
    let content = read_to_string(download_dir.path().join("moviedata.json"))?;
    /*
    moviedata.json (103494 lines, 3.5M bytes)
    The JSON file is not a DynamoDB style JSON, but standard JSON format like below:
    [
        {
            "year": 2013,
            "title": "Rush",
            "info": {
                "directors": ["Ron Howard"],
                "release_date": "2013-09-02T00:00:00Z",
                "rating": 8.3,
                "genres": [
                    "Action",
    */

    // Step 3. wait tables to be created and in ACTIVE status
    wait_table_creation(&cx, vec!["Movie"]).await;

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
                        .map(|(k, v)| (String::from(k), data::dispatch_jsonvalue_to_attrval(v)))
                        .collect();
                    write_requests.push(WriteRequest {
                        put_request: Some(PutRequest { item: item_attrval }),
                        delete_request: None,
                    });
                    if write_requests.len() == 25 {
                        break 'batch;
                    };
                }
            }
        } // 'batch loop
        request_items.insert("Movie".to_string(), write_requests);
        batch::batch_write_untill_processed(cx.clone(), request_items).await?;
    } // 'whole loop
    request_items.insert("Movie".to_string(), write_requests);
    batch::batch_write_untill_processed(cx.clone(), request_items).await?;

    Ok(())
}

async fn launch_default_sample(cx: app::Context) -> Result<(), DyneinBootstrapError> {
    println!(
        "\
Bootstrapping - dynein will creates 4 sample tables defined here:
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
        prepare_table(&cx, table_name, keys).await
    }

    /* Step 2. Download & unzip data. The sampledata.zip contains 4 files.
    https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/samples/sampledata.zip
    - Forum.json          (23 lines)
    - ProductCatalog.json (306 lines)
    - Reply.json          (75 lines)
    - Thread.json         (129 lines)

    These JSON files are already BatchWriteItem format. e.g. Forum.json
    { "Forum": [
        { "PutRequest":
            { "Item": {
                    "Name": {"S":"Amazon DynamoDB"},
                    "Category": {"S":"Amazon Web Services"},
                    "Threads": {"N":"2"},
                    "Messages": {"N":"4"}, ...
    */
    let url =
        "https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/samples/sampledata.zip";
    let download_dir: tempfile::TempDir = download_and_extract_zip(url).await?;

    // Step 3. wait tables to be created and in ACTIVE status
    let creating_table_names: Vec<&str> = tables.clone().iter().map(|pair| pair.0).collect();
    wait_table_creation(&cx, creating_table_names).await;

    // Step 4. load data into tables
    println!("Tables are ready and retrieved sample data locally. Now start writing data into samle tables...");
    for (table_name, _) in &tables {
        let content: String =
            read_to_string(download_dir.path().join(format!("{}.json", table_name)))?;
        let request_items = batch::build_batch_request_items(content)?;
        batch::batch_write_untill_processed(cx.clone(), request_items).await?;
    }
    println!(
        "\n\nNow all tables have sample data. Try following commands to play with dynein. Enjoy!"
    );
    println!("  $ dy --region {} ls", &cx.effective_region().name());
    println!(
        "  $ dy --region {} desc --table Thread",
        &cx.effective_region().name()
    );
    println!(
        "  $ dy --region {} scan --table Thread",
        &cx.effective_region().name()
    );
    println!(
        "  $ dy --region {} use --table Thread",
        &cx.effective_region().name()
    );
    println!("  $ dy scan");
    println!("\nAfter you 'use' a table like above, dynein assume you're using the same region & table, which info is stored at ~/.dynein/config.yml and ~/.dynein/cache.yml");
    println!(
        "Let's move on with the '{}' region you've just 'use'd...",
        &cx.effective_region().name()
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
        cx.clone(),
        table_name.to_string(),
        keys.iter().map(|k| (*k).to_string()).collect(),
    )
    .await
    {
        Ok(desc) => {
            println!(
                "Started to create table '{}' in {} region. status: {}",
                &table_name,
                &cx.effective_region().name(),
                desc.table_status.unwrap()
            );
        }
        Err(e) => match e {
            RusotoError::Service(CreateTableError::ResourceInUse(_)) => println!(
                "[skip] Table '{}' already exists, skipping to create new one.",
                &table_name
            ),
            _ => {
                debug!("CreateTable API call got an error -- {:#?}", e);
                error!("{}", e.to_string());
                std::process::exit(1);
            }
        },
    }
}

async fn download_and_extract_zip(target: &str) -> Result<tempfile::TempDir, DyneinBootstrapError> {
    let tmpdir: tempfile::TempDir = Builder::new().tempdir()?;
    debug!("temporary download & unzip directory: {:?}", &tmpdir);

    println!("Temporarily downloading sample data from {}", target);
    let res_bytes = reqwest::get(target).await?.bytes().await?;
    let fpath: PathBuf = tmpdir.path().join("downloaded_sampledata.zip");
    debug!("Downloading the file at: {}", &fpath.display());
    let mut zfile: File = File::create(fpath.clone())?;
    zfile.write_all(&res_bytes)?;
    debug!(
        "Finished writing content of the downloaded data into '{}'",
        &fpath.display()
    );

    let mut zarchive = zip::ZipArchive::new(File::open(fpath)?)?;
    debug!("Opened the zip archive File just written: {:?}", zarchive);

    for i in 0..zarchive.len() {
        let mut f: zip::read::ZipFile<'_> = zarchive.by_index(i)?;
        debug!("target ZipFile name: {}", f.name());
        let unzipped_fpath = tmpdir.path().join(f.name());
        debug!(
            "[file #{}] file in the archive is: {}",
            &i,
            unzipped_fpath.display()
        );

        // create a directory if target file is a directory (ends with '/').
        if (&*f.name()).ends_with('/') {
            create_dir_all(&unzipped_fpath)?
        } else {
            // create missing parent directory before diving into actual file
            if let Some(p) = unzipped_fpath.parent() {
                if !p.exists() {
                    create_dir_all(&p)?;
                }
            }

            // create unzipped file
            let mut out = File::create(&unzipped_fpath)?;
            copy(&mut f, &mut out)?;
            debug!("[file #{}] done extracting file.", &i);
        }
    }

    Ok(tmpdir)
}

async fn wait_table_creation(cx: &app::Context, mut processing_tables: Vec<&str>) {
    debug!("tables in progress: {:?}", processing_tables);
    loop {
        let r: &Region = &cx.effective_region();
        let create_table_results = join_all(
            processing_tables
                .iter()
                .map(|t| app::describe_table_api(r, (*t).to_string())),
        )
        .await;
        let statuses: Vec<String> = create_table_results
            .iter()
            .map(|desc| desc.table_status.to_owned().unwrap())
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
