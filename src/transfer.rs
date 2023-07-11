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
    fs,
    io::{Error as IOError, Write},
    path::Path,
};

use dialoguer::Confirm;
use log::{debug, error};
use serde_json::{de::StrRead, Deserializer, StreamDeserializer, Value as JsonValue};

use rusoto_dynamodb::{AttributeValue, ScanOutput, WriteRequest};

use super::app;
use super::batch;
use super::control;
use super::data;

#[derive(Debug)]
struct SuggestedAttribute {
    name: String,
    type_str: String,
}

/* =================================================
Public functions
================================================= */

/// Export items in a DynamoDB table into specified format (JSON, JSONL, JSON compact, or CSV. default is JSON).
/// As CSV is a kind of "structured" format, you cannot export DynamoDB's NoSQL-ish "unstructured" data into CSV without any instruction from users.
/// Thus as an "instruction" this function takes --attributes or --keys-only options. If neither of them are given, dynein "guesses" attributes to export from the first item.
pub async fn export(
    cx: app::Context,
    given_attributes: Option<String>,
    keys_only: bool,
    output_file: String,
    format: Option<String>,
) -> Result<(), IOError> {
    // TODO: Parallel scan to make it faster https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Scan.html#Scan.ParallelScan
    // TODO: Show rough progress bar (sum(scan_output.scanned_item)/item_size_of_the_table(6hr)) to track progress.
    let ts: app::TableSchema = app::table_schema(&cx).await;
    let format_str: Option<&str> = format.as_deref();

    if ts.mode == control::Mode::Provisioned {
        let msg = "WARN: For the best performance on import/export, dynein recommends OnDemand mode. However the target table is Provisioned mode now. Proceed anyway?";
        if !Confirm::new().with_prompt(msg).interact()? {
            app::bye(0, "Operation has been cancelled.");
        }
    }

    // Basically given_attributes would be used, but on CSV format, it can be overwritten by suggested attributes
    let mut attributes: Option<String> = given_attributes.clone();
    match format_str {
        Some("csv") => {
            if !keys_only && given_attributes.is_none() {
                attributes = overwrite_attributes_or_exit(&cx, &ts)
                    .await
                    .expect("failed to overwrite attributes based on a scanned item");
            }
        }
        None | Some(_) => {
            if keys_only || given_attributes.is_some() {
                app::bye(
                    1,
                    "You can use --keys-only and --attributes only with CSV format.",
                )
            }
        }
    }

    // Create output file. If target file already exists, ask users if it's ok to delete contents of the file.
    // Though final output file is created here, it would be blank until scan all items. You can see progress in temporary output file.
    let f: fs::File = if Path::new(&output_file).exists() {
        let msg = "Specified output file already exists. Is it OK to truncate contents?";
        if !Confirm::new().with_prompt(msg).interact()? {
            app::bye(0, "Operation has been cancelled.");
        }
        debug!("truncating existing output file.");
        let _f = fs::OpenOptions::new().append(true).open(&output_file)?;
        _f.set_len(0)?;
        _f
    } else {
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&output_file)?
    };

    // These temporary file is used to store data "body" and finally merged into output file.
    let tmp_output_filename: &str = &format!("{}_tmp", output_file);
    let mut tmp_output_file: fs::File = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(tmp_output_filename)?;
    tmp_output_file.set_len(0)?;

    let mut last_evaluated_key: Option<HashMap<String, rusoto_dynamodb::AttributeValue>> = None;
    loop {
        // Invoke Scan API here. At the 1st iteration exclusive_start_key would be "None" as defined above, outside of the loop.
        // On 2nd iteration and later, passing last_evaluated_key from the previous loop as an exclusive_start_key.
        let scan_output: ScanOutput = data::scan_api(
            cx.clone(),
            None,  /* index */
            false, /* consistent_read */
            &attributes,
            keys_only,
            None,               /* limit */
            last_evaluated_key, /* exclusive_start_key */
        )
        .await;

        let items = scan_output
            .items
            .expect("Scan result items should be 'Some' even if no item returned.");
        match format_str {
            None | Some("json") => {
                let s = serde_json::to_string_pretty(&data::convert_to_json_vec(&items))?;
                tmp_output_file.write_all(connectable_json(s, false).as_bytes())?;
            }
            Some("jsonl") => {
                let mut s: String = String::new();
                for item in &items {
                    s.push_str(&serde_json::to_string(&data::convert_to_json(item))?);
                    s.push('\n');
                }
                tmp_output_file.write_all(s.as_bytes())?;
            }
            Some("json-compact") => {
                let s = serde_json::to_string(&data::convert_to_json_vec(&items))?;
                tmp_output_file.write_all(connectable_json(s, true).as_bytes())?;
            }
            Some("csv") => {
                let s = data::convert_items_to_csv_lines(
                    &items,
                    &ts,
                    &attrs_to_append(&ts, &attributes),
                    keys_only,
                );
                tmp_output_file.write_all(s.as_bytes())?;
            }
            Some(o) => panic!("Invalid output format is given: {}", o),
        }

        // update last_evaluated_key for the next iteration.
        // If there's no more item in the table, last_evaluated_key would be "None" and it means it's ok to break the loop.
        debug!(
            "scan_output.last_evaluated_key is: {:?}",
            &scan_output.last_evaluated_key
        );
        match scan_output.last_evaluated_key {
            None => break,
            Some(lek) => last_evaluated_key = Some(lek),
        }
    }

    match format_str {
        None | Some("json") => json_finish(f, tmp_output_filename)?.write_all(b"\n]")?,
        Some("json-compact") => json_finish(f, tmp_output_filename)?.write_all(b"]")?,
        Some("jsonl") => jsonl_finish(f, tmp_output_filename)?,
        Some("csv") => csv_finish(
            f,
            tmp_output_filename,
            &ts,
            attrs_to_append(&ts, &attributes),
            keys_only,
        )?
        .write_all(b"\n")?,
        Some(o) => panic!("Invalid output format is given: {}", o),
    };

    // As mentioned earlier, deleting temporary file here in all formats.
    fs::remove_file(tmp_output_filename)?;

    Ok(())
}

pub async fn import(
    cx: app::Context,
    input_file: String,
    format: Option<String>,
    enable_set_inference: bool,
) -> Result<(), batch::DyneinBatchError> {
    let format_str: Option<&str> = format.as_deref();

    let ts: app::TableSchema = app::table_schema(&cx).await;
    if ts.mode == control::Mode::Provisioned {
        let msg = "WARN: For the best performance on import/export, dynein recommends OnDemand mode. However the target table is Provisioned mode now. Proceed anyway?";
        if !Confirm::new().with_prompt(msg).interact()? {
            println!("Operation has been cancelled.");
            return Ok(());
        }
    }

    let input_string: String = if Path::new(&input_file).exists() {
        fs::read_to_string(&input_file)?
    } else {
        error!("Couldn't find the input file '{}'.", &input_file);
        std::process::exit(1);
    };

    match format_str {
        None | Some("json") | Some("json-compact") => {
            let array_of_json_obj: Vec<JsonValue> = serde_json::from_str(&input_string)?;
            write_array_of_jsons_with_chunked_25(cx, array_of_json_obj, enable_set_inference)
                .await?;
        }
        Some("jsonl") => {
            // JSON Lines can be deserialized with into_iter() as below.
            let array_of_json_obj: StreamDeserializer<'_, StrRead<'_>, JsonValue> =
                Deserializer::from_str(&input_string).into_iter::<JsonValue>();
            // list_of_jsons contains deserialize results. Filter them and get only valid items.
            let array_of_valid_json_obj: Vec<JsonValue> =
                array_of_json_obj.filter_map(Result::ok).collect();
            write_array_of_jsons_with_chunked_25(cx, array_of_valid_json_obj, enable_set_inference)
                .await?;
        }
        Some("csv") => {
            let lines: Vec<&str> = input_string
                .split('\n')
                .collect::<Vec<&str>>() // split by "\n" and get lines
                .iter()
                .filter(|&x| !x.is_empty())
                .cloned()
                .collect::<Vec<&str>>(); // remove blank line (e.g. last line)
            let headers: Vec<&str> = lines[0].split(',').collect::<Vec<&str>>();
            let mut matrix: Vec<Vec<&str>> = vec![];
            // Iterate over lines (from index = 1, as index = 0 is the header line)
            for (i, line) in lines.iter().enumerate().skip(1) {
                let cells: Vec<&str> = line.split(',').collect::<Vec<&str>>();
                debug!("splitted line => {:?}", cells);
                matrix.push(cells);
                if i % 25 == 0 {
                    write_csv_matrix(&cx, matrix.clone(), &headers, enable_set_inference).await?;
                    matrix.clear();
                }
            }
            debug!("rest of matrix => {:?}", matrix);
            if !matrix.is_empty() {
                write_csv_matrix(&cx, matrix.clone(), &headers, enable_set_inference).await?;
            }
        }
        Some(o) => panic!("Invalid input format is given: {}", o),
    }
    Ok(())
}

/* =================================================
Private functions
================================================= */

async fn overwrite_attributes_or_exit(
    cx: &app::Context,
    ts: &app::TableSchema,
) -> Result<Option<String>, IOError> {
    println!("As neither --keys-only nor --attributes options are given, fetching an item to understand attributes to export...");
    let suggested_attributes: Vec<SuggestedAttribute> = suggest_attributes(cx, ts).await;

    // if at least one attribute found
    println!("Found following attributes in the first item in the table:");
    for preview_attribute in &suggested_attributes {
        println!(
            "  - {} ({})",
            preview_attribute.name, preview_attribute.type_str
        );
    }
    let msg = "Are you OK to export items in CSV with columns(attributes) above?";
    if !Confirm::new().with_prompt(msg).interact()? {
        app::bye(0, "Operation has been cancelled. You can use --keys-only or --attributes option to specify columns explicitly.");
    }

    // Overwrite given attributes with suggested attributes beased on a sampled item
    Ok(Some(
        suggested_attributes
            .into_iter()
            .map(|sa| sa.name)
            .collect::<Vec<String>>()
            .join(","),
    ))
}

/// This function scan the fisrt item from the target table and use it as a source of attributes.
async fn suggest_attributes(cx: &app::Context, ts: &app::TableSchema) -> Vec<SuggestedAttribute> {
    let mut attributes_suggestion = vec![];

    // items: Vec<HashMap<String, AttributeValue>>
    let items = data::scan_api(
        cx.clone(),
        None,    /* index */
        false,   /* consistent_read */
        &None,   /* attributes */
        false,   /* keys_only */
        Some(1), /* limit */
        None,    /* esk */
    )
    .await
    .items
    .expect("items should be 'Some' even if there's no item in the table.");

    if items.is_empty() {
        app::bye(0, "No item to export in this table. Quit the operation.");
    }

    // Filter out primary keys. i.e. select attributes that aren't required by the table's keyschema.
    let primary_keys = vec![
        Some(ts.pk.name.to_owned()),
        ts.sk.to_owned().map(|x| x.name),
    ];
    let non_key_attributes = items[0]
        .iter()
        .filter(
            |(attr, _)| {
                !primary_keys
                    .iter()
                    .any(|key| Some(attr.to_owned()) == key.as_ref())
            }, // ).map(|(k, _)| k).collect::<Vec<&String>>();
        )
        .collect::<Vec<(&String, &AttributeValue)>>();

    for (attr, attrval) in non_key_attributes {
        attributes_suggestion.push(SuggestedAttribute {
            name: attr.to_owned(),
            type_str: data::attrval_to_type(attrval).expect("attrval should be mapped"),
        });
    }

    debug!("Suggested attributes to use: {:?}", attributes_suggestion);
    attributes_suggestion
}

fn attrs_to_append(ts: &app::TableSchema, attributes: &Option<String>) -> Option<Vec<String>> {
    attributes
        .clone()
        .map(|ats| filter_attributes_to_append(ts, ats))
}

/// This function takes list of attributes separated by comma (e.g. "name,age,address")
/// and return vec of these strings, filtering pk/sk.
fn filter_attributes_to_append(ts: &app::TableSchema, ats: String) -> Vec<String> {
    let mut attributes_to_append: Vec<String> = vec![];
    let splitted_attributes: Vec<String> = ats.split(',').map(|x| x.trim().to_owned()).collect();
    for attr in splitted_attributes {
        // skip if attributes contain primary key(s)
        if attr == ts.pk.name || (ts.sk.is_some() && attr == ts.clone().sk.unwrap().name) {
            println!("NOTE: primary keys are included by default and you don't need to give them as a part of --attributes.");
            continue;
        }
        attributes_to_append.push(attr);
    }
    attributes_to_append
}

/// This function tweaks scan output items.
/// Each scan iteration, converted string would be a single JSON array: e.g. [ {a:1}, {a:2} ]
/// When multiple scan is needed (i.e. when last_evaluated_key is Some), connected string would be: e.g. [ {a:1}, {a:2} ][ {a:3}, {a:4} ]
/// To avoid this invalid JSON from written to output file, this method remove the first "[" and the last "]", then add "," after the last item.
fn connectable_json(mut s: String, compact: bool) -> String {
    s.remove(0); // remove first char "["
    let len = s.len();
    if compact {
        s.truncate(len - 1); // remove last char "]"
    } else {
        s.truncate(len - 2); // remove last char "]" and newline
    }
    s.push(','); // add last "," so that continue to next iteration
    s
}

/// This function takes final output file and temporary filename which has incomplete JSON body, and write final output JSON file.
/// last "]" is not added in this function, as it depends on json or json-compact.
fn json_finish(mut f: fs::File, tmp_output_filename: &str) -> Result<fs::File, IOError> {
    f.write_all(b"[")?; // write initial "[" as the first letter of JSON array.
    let mut contents = fs::read_to_string(tmp_output_filename)?;
    let len = contents.len();
    contents.truncate(len - 1); // remove last ","
    f.write_all(contents.as_bytes())?;
    Ok(f)
}

/// This function takes final output file and temporary filename. For JSON"L", copying whole content is enough.
fn jsonl_finish(mut f: fs::File, tmp_output_filename: &str) -> Result<(), IOError> {
    let contents = fs::read_to_string(tmp_output_filename)?;
    f.write_all(contents.as_bytes())?;
    Ok(())
}

/// This function takes final output file and temporary filename, writing CSV header and then copying contents to the output file.
fn csv_finish(
    mut f: fs::File,
    tmp_output_filename: &str,
    ts: &app::TableSchema,
    attributes_to_append: Option<Vec<String>>,
    keys_only: bool,
) -> Result<fs::File, IOError> {
    f.write_all(build_csv_header(ts, attributes_to_append, keys_only).as_bytes())?;
    let contents = fs::read_to_string(tmp_output_filename)?;
    f.write_all(contents.as_bytes())?;
    Ok(f)
}

/// This function generate CSV headers for the output file to export.
fn build_csv_header(
    ts: &app::TableSchema,
    attributes_to_append: Option<Vec<String>>,
    keys_only: bool,
) -> String {
    // First of all put pk (and sk, if exists)
    let mut header_str: String = ts.pk.name.clone();
    if let Some(sk) = &ts.sk {
        header_str.push(',');
        header_str.push_str(&sk.name);
    };

    if keys_only {
    } else if let Some(attrs) = attributes_to_append {
        header_str.push(',');
        header_str.push_str(&attrs.join(","));
    }

    header_str.push('\n');
    header_str
}

async fn write_array_of_jsons_with_chunked_25(
    cx: app::Context,
    array_of_json_obj: Vec<JsonValue>,
    enable_set_inference: bool,
) -> Result<(), batch::DyneinBatchError> {
    for chunk /* Vec<JsonValue> */ in array_of_json_obj.chunks(25) { // As BatchWriteItem request can have up to 25 items.
        let request_items: HashMap<String, Vec<WriteRequest>> = batch::convert_jsonvals_to_request_items(&cx, chunk.to_vec(), enable_set_inference).await?;
        batch::batch_write_untill_processed(cx.clone(), request_items).await?;
    }
    Ok(())
}

/// This function takes "matrix" with "headers", builds a parameter for BatchWriteItem, then write it untill they've been processed all.
/// The "matrix" is a data built from CSV file and each "cell/column" is an attribute of a item.
///
/// e.g.
///    name, age, fruit ... headers
/// [[John, 12, Apple],
///  [Ami, 23, Orange],
///  [Shu, 42, Banana]] ... matrix
async fn write_csv_matrix(
    cx: &app::Context,
    matrix: Vec<Vec<&str>>,
    headers: &[&str],
    enable_set_inference: bool,
) -> Result<(), batch::DyneinBatchError> {
    let request_items: HashMap<String, Vec<WriteRequest>> =
        batch::csv_matrix_to_request_items(cx, &matrix, headers, enable_set_inference).await?;
    batch::batch_write_untill_processed(cx.clone(), request_items).await?;
    Ok(())
}
