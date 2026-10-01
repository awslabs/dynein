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

use console::Term;
use std::collections::VecDeque;
use std::time::Instant;
use std::{
    collections::HashMap,
    fs,
    io::{self, Error as IOError, Seek, Write},
    path::Path,
};

use dialoguer::Confirm;
use log::{debug, error};
use serde_json::{de::StrRead, Deserializer, StreamDeserializer, Value as JsonValue};

use aws_sdk_dynamodb::{
    operation::scan::ScanOutput,
    types::{AttributeValue, WriteRequest},
};
use thiserror::Error;

use super::app;
use super::batch;
use super::data;
use super::ddb::table;

#[derive(Error, Debug)]
pub enum DyneinExportError {
    #[error("io error")]
    IO(#[from] std::io::Error),
    #[error("serde error")]
    SerdeError(#[from] serde_json::Error),
}

impl From<dialoguer::Error> for DyneinExportError {
    fn from(e: dialoguer::Error) -> Self {
        match e {
            dialoguer::Error::IO(e) => DyneinExportError::IO(e),
        }
    }
}

#[derive(Debug)]
struct SuggestedAttribute {
    name: String,
    type_str: String,
}

#[derive(Clone, Debug, Hash, PartialOrd, PartialEq)]
struct ProgressState {
    processed_items: usize,
    recent_processed_items: VecDeque<(Instant, usize)>,
    max_recordable_observations: usize,
}

impl ProgressState {
    fn new(max_recordable_observations: usize) -> ProgressState {
        ProgressState {
            processed_items: 0,
            recent_processed_items: VecDeque::with_capacity(max_recordable_observations),
            max_recordable_observations,
        }
    }

    fn add_observation(&mut self, processed_items: usize) {
        self.add_observation_with_time(processed_items, Instant::now())
    }

    fn add_observation_with_time(&mut self, processed_items: usize, at: Instant) {
        self.processed_items += processed_items;

        if self.recent_processed_items.len() == self.max_recordable_observations {
            self.recent_processed_items.pop_back();
        }
        self.recent_processed_items
            .push_front((at, processed_items));
    }

    fn processed_items(&self) -> usize {
        self.processed_items
    }

    fn recent_average_processed_items_per_second(&self) -> f64 {
        self.recent_average_processed_items_per_second_with_time(Instant::now())
    }

    fn recent_average_processed_items_per_second_with_time(&self, at: Instant) -> f64 {
        let mut sum = 0.0;
        for v in &self.recent_processed_items {
            sum += v.1 as f64
        }
        if let Some((oldest_time, _)) = self.recent_processed_items.back() {
            if at == *oldest_time {
                f64::NAN
            } else {
                sum / at.duration_since(*oldest_time).as_secs_f64()
            }
        } else {
            0.0
        }
    }

    fn show(&self, to_stderr: bool) {
        let items = self.processed_items();
        let items_per_sec = self.recent_average_processed_items_per_second();
        let mut term = if to_stderr {
            Term::stderr()
        } else {
            Term::stdout()
        };
        term.clear_line().expect("Failed to clear line");
        write!(
            term,
            "{} items processed ({:.2} items/sec)",
            items, items_per_sec
        )
        .expect("Failed to update message");
        term.flush().expect("Failed to flush");
    }
}

const MAX_NUMBER_OF_OBSERVES: usize = 10;

/* =================================================
Public functions
================================================= */

/// Export items in a DynamoDB table into specified format (JSON, JSONL, JSON compact, or CSV. default is JSON).
/// As CSV is a kind of "structured" format, you cannot export DynamoDB's NoSQL-ish "unstructured" data into CSV without any instruction from users.
/// Thus as an "instruction" this function takes --attributes or --keys-only options. If neither of them are given, dynein "guesses" attributes to export from the first item.
pub async fn export(
    cx: &app::Context,
    given_attributes: Option<String>,
    keys_only: bool,
    output_file: String,
    format: Option<String>,
) -> Result<(), DyneinExportError> {
    // TODO: Parallel scan to make it faster https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Scan.html#Scan.ParallelScan
    // TODO: Show rough progress bar (sum(scan_output.scanned_item)/item_size_of_the_table(6hr)) to track progress.
    let ts: app::TableSchema = app::table_schema(cx).await;
    let format_str: Option<&str> = format.as_deref();
    let to_stdout = output_file == "-" || output_file == "/dev/stdout";

    if ts.mode == table::Mode::Provisioned {
        let msg = "WARN: For the best performance on import/export, dynein recommends OnDemand mode. However the target table is Provisioned mode now. Proceed anyway?";
        if !Confirm::new().with_prompt(msg).interact()? {
            export_bye(to_stdout, 0, "Operation has been cancelled.");
        }
    }

    // Basically given_attributes would be used, but on CSV format, it can be overwritten by suggested attributes
    let attributes: Option<String> = match format_str {
        Some("csv") => {
            if !keys_only && given_attributes.is_none() {
                overwrite_attributes_or_exit(cx, &ts, to_stdout)
                    .await
                    .expect("failed to overwrite attributes based on a scanned item")
            } else {
                given_attributes
            }
        }
        None | Some(_) => {
            if keys_only || given_attributes.is_some() {
                export_bye(
                    to_stdout,
                    1,
                    "You can use --keys-only and --attributes only with CSV format.",
                )
            }
            given_attributes
        }
    };

    // Create the output file, or use stdout for a pipe. Confirm before truncating an existing file.
    let mut f: Box<dyn Write> = if to_stdout {
        Box::new(io::stdout())
    } else if Path::new(&output_file).exists() {
        let msg = "Specified output file already exists. Is it OK to truncate contents?";
        if !Confirm::new().with_prompt(msg).interact()? {
            export_bye(to_stdout, 0, "Operation has been cancelled.");
        }
        debug!("truncating existing output file.");
        let _f = fs::OpenOptions::new().append(true).open(&output_file)?;
        _f.set_len(0)?;
        Box::new(_f)
    } else {
        Box::new(
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&output_file)?,
        )
    };

    // Keep file exports buffered, but stream directly when stdout is requested.
    let mut tmp_output_file = if to_stdout {
        None
    } else {
        Some(tempfile::tempfile()?)
    };
    let writer: &mut dyn Write = match tmp_output_file.as_mut() {
        Some(tmp) => tmp,
        None => &mut f,
    };

    let attributes_to_append = attrs_to_append(&ts, &attributes, to_stdout);
    match format_str {
        None | Some("json") | Some("json-compact") => writer.write_all(b"[")?,
        Some("csv") => writer
            .write_all(build_csv_header(&ts, attributes_to_append.clone(), keys_only).as_bytes())?,
        _ => {}
    }

    let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;
    let mut progress_status = ProgressState::new(MAX_NUMBER_OF_OBSERVES);
    let mut wrote_csv_rows = false;
    let mut wrote_json_items = false;
    loop {
        // Invoke Scan API here. At the 1st iteration exclusive_start_key would be "None" as defined above, outside of the loop.
        // On 2nd iteration and later, passing last_evaluated_key from the previous loop as an exclusive_start_key.
        let scan_output: ScanOutput = data::scan_api(
            cx,
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

        progress_status.add_observation(items.len());
        match format_str {
            None | Some("json") => {
                let s = serde_json::to_string_pretty(&data::convert_to_json_vec(&items))?;
                write_json_page(writer, s, false, &mut wrote_json_items)?;
            }
            Some("jsonl") => {
                for item in &items {
                    writer.write_all(
                        serde_json::to_string(&data::convert_to_json(item))?.as_bytes(),
                    )?;
                    writer.write_all(b"\n")?;
                }
            }
            Some("json-compact") => {
                let s = serde_json::to_string(&data::convert_to_json_vec(&items))?;
                write_json_page(writer, s, true, &mut wrote_json_items)?;
            }
            Some("csv") => {
                let s =
                    data::convert_items_to_csv_lines(&items, &ts, &attributes_to_append, keys_only);
                write_csv_page(writer, &s, &mut wrote_csv_rows)?;
            }
            Some(o) => panic!("Invalid output format is given: {}", o),
        }
        if to_stdout {
            writer.flush()?;
        }
        progress_status.show(to_stdout);

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
        None | Some("json") => writer.write_all(b"\n]")?,
        Some("json-compact") => writer.write_all(b"]")?,
        Some("jsonl") => {}
        Some("csv") => writer.write_all(b"\n")?,
        Some(o) => panic!("Invalid output format is given: {}", o),
    };
    writer.flush()?;
    if let Some(mut tmp) = tmp_output_file {
        tmp.rewind()?;
        io::copy(&mut tmp, &mut f)?;
    }
    f.flush()?;

    Ok(())
}

pub async fn import(
    cx: &app::Context,
    input_file: String,
    format: Option<String>,
    enable_set_inference: bool,
) -> Result<(), batch::DyneinBatchError> {
    let format_str: Option<&str> = format.as_deref();

    let ts: app::TableSchema = app::table_schema(cx).await;
    if ts.mode == table::Mode::Provisioned {
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
                .into_iter()
                .filter(|&x| !x.is_empty())
                .collect::<Vec<&str>>(); // remove blank line (e.g. last line)
            let headers: Vec<&str> = lines[0].split(',').collect::<Vec<&str>>();
            let mut matrix: Vec<Vec<&str>> = vec![];
            // Iterate over lines (from index = 1, as index = 0 is the header line)
            let mut progress_status = ProgressState::new(MAX_NUMBER_OF_OBSERVES);
            for (i, line) in lines.iter().enumerate().skip(1) {
                let cells: Vec<&str> = line.split(',').collect::<Vec<&str>>();
                debug!("splitted line => {:?}", cells);
                matrix.push(cells);
                if i % 25 == 0 {
                    write_csv_matrix(cx, &matrix, &headers, enable_set_inference).await?;
                    progress_status.add_observation(25);
                    progress_status.show(false);
                    matrix.clear();
                }
            }
            debug!("rest of matrix => {:?}", matrix);
            if !matrix.is_empty() {
                write_csv_matrix(cx, &matrix, &headers, enable_set_inference).await?;
                progress_status.add_observation(matrix.len());
                progress_status.show(false);
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
    to_stderr: bool,
) -> Result<Option<String>, dialoguer::Error> {
    print_export_info(to_stderr, "As neither --keys-only nor --attributes options are given, fetching an item to understand attributes to export...");
    let suggested_attributes: Vec<SuggestedAttribute> = suggest_attributes(cx, ts, to_stderr).await;

    // if at least one attribute found
    print_export_info(
        to_stderr,
        "Found following attributes in the first item in the table:",
    );
    for preview_attribute in &suggested_attributes {
        print_export_info(
            to_stderr,
            &format!(
                "  - {} ({})",
                preview_attribute.name, preview_attribute.type_str
            ),
        );
    }
    let msg = "Are you OK to export items in CSV with columns(attributes) above?";
    if !Confirm::new().with_prompt(msg).interact()? {
        export_bye(to_stderr, 0, "Operation has been cancelled. You can use --keys-only or --attributes option to specify columns explicitly.");
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
async fn suggest_attributes(
    cx: &app::Context,
    ts: &app::TableSchema,
    to_stderr: bool,
) -> Vec<SuggestedAttribute> {
    let mut attributes_suggestion = vec![];

    // items: Vec<HashMap<String, AttributeValue>>
    let items = data::scan_api(
        cx,
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
        export_bye(
            to_stderr,
            0,
            "No item to export in this table. Quit the operation.",
        );
    }

    // Filter out primary keys. i.e. select attributes that aren't required by the table's keyschema.
    let primary_keys = [
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

fn attrs_to_append(
    ts: &app::TableSchema,
    attributes: &Option<String>,
    to_stderr: bool,
) -> Option<Vec<String>> {
    attributes
        .as_ref()
        .map(|ats| filter_attributes_to_append(ts, ats, to_stderr))
}

/// This function takes list of attributes separated by comma (e.g. "name,age,address")
/// and return vec of these strings, filtering pk/sk.
fn filter_attributes_to_append(ts: &app::TableSchema, ats: &str, to_stderr: bool) -> Vec<String> {
    let mut attributes_to_append: Vec<String> = vec![];
    let splitted_attributes: Vec<String> = ats.split(',').map(|x| x.trim().to_owned()).collect();
    for attr in splitted_attributes {
        // skip if attributes contain primary key(s)
        if attr == ts.pk.name || (ts.sk.is_some() && attr == ts.sk.as_ref().unwrap().name) {
            print_export_info(to_stderr, "NOTE: primary keys are included by default and you don't need to give them as a part of --attributes.");
            continue;
        }
        attributes_to_append.push(attr);
    }
    attributes_to_append
}

fn print_export_info(to_stderr: bool, message: &str) {
    if to_stderr {
        eprintln!("{message}");
    } else {
        println!("{message}");
    }
}

fn export_bye(to_stderr: bool, code: i32, message: &str) -> ! {
    print_export_info(to_stderr, message);
    std::process::exit(code);
}

/// This function tweaks scan output items.
/// Each scan iteration, converted string would be a single JSON array: e.g. [ {a:1}, {a:2} ]
/// When multiple scan is needed (i.e. when last_evaluated_key is Some), connected string would be: e.g. [ {a:1}, {a:2} ][ {a:3}, {a:4} ]
/// To avoid this invalid JSON from written to output file, this method remove the first "[" and the last "]", then add "," after the last item.
fn connectable_json(mut s: String, compact: bool) -> String {
    s.remove(0); // remove first char "["
    let len = s.len();
    if compact || len == 1 {
        // empty array even if not compact is on one line
        s.truncate(len - 1); // remove last char "]"
    } else {
        s.truncate(len - 2); // remove last char "]" and newline
    }
    s.push(','); // add last "," so that continue to next iteration
    s
}

/// Append one JSON page without breaking the surrounding array.
fn write_json_page(
    writer: &mut dyn Write,
    contents: String,
    compact: bool,
    wrote_items: &mut bool,
) -> Result<(), IOError> {
    let mut body = connectable_json(contents, compact);
    body.pop(); // Remove the comma added for the next page.
    if body.is_empty() {
        return Ok(());
    }
    if *wrote_items {
        writer.write_all(b",")?;
    }
    writer.write_all(body.as_bytes())?;
    *wrote_items = true;
    Ok(())
}

/// Write one page of CSV rows, separating it from any rows written earlier.
fn write_csv_page(
    writer: &mut dyn Write,
    contents: &str,
    wrote_rows: &mut bool,
) -> Result<(), IOError> {
    if contents.is_empty() {
        return Ok(());
    }
    if *wrote_rows {
        writer.write_all(b"\n")?;
    }
    writer.write_all(contents.as_bytes())?;
    *wrote_rows = true;
    Ok(())
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
    cx: &app::Context,
    array_of_json_obj: Vec<JsonValue>,
    enable_set_inference: bool,
) -> Result<(), batch::DyneinBatchError> {
    let mut progress_status = ProgressState::new(MAX_NUMBER_OF_OBSERVES);
    for chunk /* Vec<JsonValue> */ in array_of_json_obj.chunks(25) { // As BatchWriteItem request can have up to 25 items.
        let items = chunk.to_vec();
        let count = items.len();
        let request_items: HashMap<String, Vec<WriteRequest>> = batch::convert_jsonvals_to_request_items(cx, items, enable_set_inference).await?;
        batch::batch_write_until_processed(cx, request_items).await?;
        progress_status.add_observation(count);
        progress_status.show(false);
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
    matrix: &[Vec<&str>],
    headers: &[&str],
    enable_set_inference: bool,
) -> Result<(), batch::DyneinBatchError> {
    let request_items: HashMap<String, Vec<WriteRequest>> =
        batch::csv_matrix_to_request_items(cx, matrix, headers, enable_set_inference).await?;
    batch::batch_write_until_processed(cx, request_items).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ddb::key::{Key, KeyType};
    use std::ops::Add;
    use std::time::Duration;

    #[test]
    fn csv_pages_are_separated_without_blank_rows() {
        let mut output = Vec::new();
        let mut wrote_rows = false;

        write_csv_page(&mut output, "first\nsecond", &mut wrote_rows).unwrap();
        write_csv_page(&mut output, "", &mut wrote_rows).unwrap();
        write_csv_page(&mut output, "third\nfourth", &mut wrote_rows).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "first\nsecond\nthird\nfourth"
        );
    }

    #[test]
    fn json_pages_form_one_valid_array() {
        let mut json = Vec::new();
        let mut wrote_items = false;
        json.extend_from_slice(b"[");
        write_json_page(&mut json, "[{\"pk\":1}]".to_owned(), true, &mut wrote_items).unwrap();
        write_json_page(&mut json, "[]".to_owned(), true, &mut wrote_items).unwrap();
        write_json_page(&mut json, "[{\"pk\":2}]".to_owned(), true, &mut wrote_items).unwrap();
        json.extend_from_slice(b"]");
        assert_eq!(String::from_utf8(json).unwrap(), r#"[{"pk":1},{"pk":2}]"#);

        let mut empty = Vec::new();
        let mut wrote_items = false;
        empty.extend_from_slice(b"[");
        write_json_page(&mut empty, "[]".to_owned(), true, &mut wrote_items).unwrap();
        empty.extend_from_slice(b"]");
        assert_eq!(String::from_utf8(empty).unwrap(), "[]");

        let mut pretty = Vec::new();
        let mut wrote_items = false;
        pretty.extend_from_slice(b"[");
        write_json_page(
            &mut pretty,
            "[\n  {\n    \"pk\": 1\n  }\n]".to_owned(),
            false,
            &mut wrote_items,
        )
        .unwrap();
        pretty.extend_from_slice(b"\n]");
        assert!(serde_json::from_slice::<serde_json::Value>(&pretty).is_ok());
    }

    #[test]
    fn csv_header_and_rows_are_separated() {
        let schema = app::TableSchema {
            region: "local".to_owned(),
            name: "test".to_owned(),
            pk: Key {
                name: "pk".to_owned(),
                kind: KeyType::S,
            },
            sk: None,
            indexes: None,
            mode: table::Mode::OnDemand,
        };
        let mut csv = Vec::new();
        csv.extend_from_slice(build_csv_header(&schema, None, true).as_bytes());
        let mut wrote_rows = false;
        write_csv_page(&mut csv, "one\ntwo", &mut wrote_rows).unwrap();
        csv.extend_from_slice(b"\n");
        assert_eq!(csv, b"pk\none\ntwo\n");
    }

    #[test]
    fn test_progress_status() {
        let mut progress = ProgressState::new(2);

        let first_observation = Instant::now();
        progress.add_observation_with_time(10, first_observation);
        assert_eq!(progress.processed_items(), 10);
        assert_eq!(progress.recent_processed_items.len(), 1);
        assert!(progress
            .recent_average_processed_items_per_second_with_time(first_observation)
            .is_nan());

        let second_observation = first_observation.add(Duration::from_millis(500));
        progress.add_observation_with_time(10, second_observation);
        assert_eq!(progress.processed_items(), 20);
        assert_eq!(progress.recent_processed_items.len(), 2);
        assert_eq!(
            progress.recent_average_processed_items_per_second_with_time(second_observation),
            (10.0 + 10.0) / 0.5
        );

        let third_observation = first_observation.add(Duration::from_millis(1000));
        progress.add_observation_with_time(12, third_observation);
        assert_eq!(progress.processed_items(), 32);
        assert_eq!(progress.recent_processed_items.len(), 2);
        assert_eq!(
            progress.recent_average_processed_items_per_second_with_time(third_observation),
            (10.0 + 12.0) / 0.5
        );
    }
}
