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

use crate::parser::DyneinParser;
use aws_sdk_dynamodb::{
    operation::batch_write_item::BatchWriteItemError,
    types::{AttributeValue, DeleteRequest, PutRequest, WriteRequest},
    Client as DynamoDbSdkClient,
};
use backon::Retryable;
use base64::{engine::general_purpose, Engine as _};
use bytes::Bytes;
use log::{debug, error, warn};
use serde_json::Value as JsonValue;
use std::{collections::HashMap, error, fmt, fs, future::Future, io::Error as IOError, pin::Pin};

use super::app;
use super::data;
use super::key;

/* =================================================
struct / enum / const
================================================= */

#[derive(Debug)]
pub enum DyneinBatchError {
    LoadData(IOError),
    PraseJSON(serde_json::Error),
    BatchWriteError(aws_sdk_dynamodb::error::SdkError<BatchWriteItemError>),
    InvalidInput(String),
    ParseError(crate::parser::ParseError),
}
impl fmt::Display for DyneinBatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            DyneinBatchError::LoadData(ref e) => e.fmt(f),
            DyneinBatchError::PraseJSON(ref e) => e.fmt(f),
            DyneinBatchError::BatchWriteError(ref e) => e.fmt(f),
            DyneinBatchError::InvalidInput(ref msg) => write!(f, "{}", msg),
            DyneinBatchError::ParseError(ref e) => e.fmt(f),
        }
    }
}
impl error::Error for DyneinBatchError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            DyneinBatchError::LoadData(ref e) => Some(e),
            DyneinBatchError::PraseJSON(ref e) => Some(e),
            DyneinBatchError::BatchWriteError(ref e) => Some(e),
            DyneinBatchError::InvalidInput(_) => None,
            DyneinBatchError::ParseError(_) => None,
        }
    }
}
impl From<IOError> for DyneinBatchError {
    fn from(e: IOError) -> Self {
        Self::LoadData(e)
    }
}
impl From<serde_json::Error> for DyneinBatchError {
    fn from(e: serde_json::Error) -> Self {
        Self::PraseJSON(e)
    }
}
impl From<aws_sdk_dynamodb::error::SdkError<BatchWriteItemError>> for DyneinBatchError {
    fn from(e: aws_sdk_dynamodb::error::SdkError<BatchWriteItemError>) -> Self {
        Self::BatchWriteError(e)
    }
}

impl From<crate::parser::ParseError> for DyneinBatchError {
    fn from(e: crate::parser::ParseError) -> Self {
        Self::ParseError(e)
    }
}

impl From<dialoguer::Error> for DyneinBatchError {
    fn from(e: dialoguer::Error) -> Self {
        match e {
            dialoguer::Error::IO(e) => Self::LoadData(e),
        }
    }
}

/* =================================================
Public functions
================================================= */

/// Receives String with the complete "request_items" JSON strcture and converts it into corresponding HashMap data.
/// "request_items" is intended to be used for BatchWriteItem and has following structure:
/// HashMap<
///   String, ... table name. Batch requests can contain multiple tables as targets.
///   Vec<    ... requests for one table should be gathered.
///     WriteRequest ... https://docs.rs/rusoto_dynamodb/0.42.0/rusoto_dynamodb/struct.WriteRequest.html
///       either:
///         - put_request (Option<PutRequest>), where PutRequest { item: HashMap<String, AttributeValue> }
///             ... it should be same as "item" parameter used in PutItem.
///         - delete_request (Option<DeleteRequest>), where DeleteRequest { key: HashMap<String, AttributeValue> }
///             ... the only thing DeleteRequest should do is specify delete target via key (i.e. pk(+sk)).
pub fn build_batch_request_items_from_json(
    raw_json_content: String,
) -> Result<HashMap<String, Vec<WriteRequest>>, serde_json::Error> {
    let mut results = HashMap::<String, Vec<WriteRequest>>::new();
    debug!(
        "Trying to convert given string into Batch Request Items: {}",
        raw_json_content
    );

    let hashmap: HashMap<String, JsonValue> = serde_json::from_str(&raw_json_content)?;

    // for each table name as a key, multiple operations are included.
    for (tbl /* String */, operations /* JsonValue */) in hashmap {
        let mut write_requests = Vec::<WriteRequest>::new();
        let ops: &Vec<JsonValue> = operations
            .as_array()
            .expect("should be array of put/delete operations");

        // each "operation" is PutRequest or DeleteRequest. convert them into DynamoDB AttributeValue and push into WriteRequest vector.
        for op in ops {
            if let Some(wrapped_item /* JsonValue */) = op.get("PutRequest") {
                debug!("Building an item for PutRequest in BatchWriteItem");
                /*
                  JSON syntax for PutRequest would look like:
                    { "Thread": [
                      { "PutRequest": {
                        "Item": {
                          "ForumName": { "S": "Amazon DynamoDB" },
                          "Subject": { "S": "DynamoDB Thread 1" },
                          "Message": { "S": "DynamoDB thread 1 message" },
                          "LastPostedBy": { ...
                */
                if let Some(raw_item /* JsonValue */) = wrapped_item.get("Item") {
                    debug!("PutRequest content item is: {:#?}", &raw_item);
                    /*
                      PutRequest content item is:
                        Object({
                            "Category": Object( { "S": String( "Amazon Web Services",), },),
                            "Messages": Object( { "N": String( "4",), },),
                            "Name": Object( { "S": String( "Amazon DynamoDB",), },),
                            "Threads": Object( { "N": String( "2",), },),
                            "Views": Object( { "N": String( "1000",), },),
                        },)
                    */
                    let item: HashMap<String, AttributeValue> =
                        ddbjson_attributes_to_attrvals(raw_item);
                    write_requests.push(
                        WriteRequest::builder()
                            .put_request(
                                PutRequest::builder().set_item(Some(item)).build().unwrap(),
                            )
                            .build(),
                    );
                } else {
                    error!("[skip] no field named 'Item' under PutRequest");
                }
            } else if let Some(wrapped_key) = op.get("DeleteRequest") {
                debug!("Building an item for DeleteRequest in BatchWriteItem");
                /*
                  JSON syntax for DeleteRequest would look like:
                    { "Thread": [
                      { "DeleteRequest": {
                          "Key": {
                            "ForumName": { "S": "Amazon DynamoDB" },
                            "Subject": { "S": "DynamoDB Thread 1" }
                          }
                        }
                      },
                */

                if let Some(raw_key) = wrapped_key.get("Key") {
                    debug!("DeleteRequest target key is: {:#?}", &raw_key);
                    /*
                      DeleteRequest content item is:
                        Object( {
                          "ForumName": Object( { "S": String( "Amazon DynamoDB",), },),
                          "Subject": Object( { "S": String( "DynamoDB Thread 1",), },),
                        },)
                    */
                    let key: HashMap<String, AttributeValue> =
                        ddbjson_attributes_to_attrvals(raw_key);
                    write_requests.push(
                        WriteRequest::builder()
                            .delete_request(
                                DeleteRequest::builder().set_key(Some(key)).build().unwrap(),
                            )
                            .build(),
                    );
                } else {
                    error!("[skip] no field named 'Key' under DeleteRequest");
                }
            } else {
                error!("[skip] In the given batch data, unknown field (neither PutRequest nor DeleteRequest) found: {:?}", op);
            }
        }
        // finally build BatchWriteItem request items which cinsists of table name key and a vector of write requests (put/delete).
        results.insert(tbl.to_string(), write_requests);
    } // end loop over a "table" key. will take a look at next table if any.

    Ok(results)
}

/// this function calls BatchWriteItem API and returns UnprocessedItems.
/// Though the type of res.unprocessed_items is `Option`, when all items are written, `Some({})` would be returned instead of `None`.
/// ref: https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_BatchWriteItem.html
/// > If any requested operations fail because the table's provisioned throughput is exceeded or an internal processing failure occurs,
/// > the failed operations are returned in the UnprocessedItems response parameter.
/// > You can investigate and optionally resend the requests. Typically, you would call BatchWriteItem in a loop. Each iteration would
/// > check for unprocessed items and submit a new BatchWriteItem request with those unprocessed items until all items have been processed.
async fn batch_write_item_api(
    cx: app::Context,
    request_items: HashMap<String, Vec<WriteRequest>>,
) -> Result<
    Option<HashMap<String, Vec<WriteRequest>>>,
    aws_sdk_dynamodb::error::SdkError<BatchWriteItemError>,
> {
    debug!(
        "Calling BatchWriteItem API with request_items: {:?}",
        &request_items
    );

    let config = cx.effective_sdk_config().await;
    let ddb = DynamoDbSdkClient::new(&config);

    let retry_setting = cx
        .retry
        .map(|v| v.batch_write_item.to_owned().unwrap_or(v.default));
    let res = match retry_setting {
        Some(backoff) => {
            let f = || async {
                ddb.batch_write_item()
                    .set_request_items(Some(request_items.clone()))
                    .send()
                    .await
            };
            f.retry(&backoff)
                .when(|err| match err.as_service_error() {
                    Some(BatchWriteItemError::ProvisionedThroughputExceededException(e)) => {
                        warn!("Retry batch_write_item : {}", e);
                        true
                    }
                    Some(BatchWriteItemError::InternalServerError(e)) => {
                        warn!("Retry batch_write_item : {}", e);
                        true
                    }
                    Some(BatchWriteItemError::RequestLimitExceeded(e)) => {
                        warn!("Retry batch_write_item : {}", e);
                        true
                    }
                    // aws_sdk_dynamodb::error::SdkError::DispatchFailure(e) => {
                    //     warn!("Retry batch_write_item : {}", &e);
                    //     true
                    // }
                    // aws_sdk_dynamodb::error::SdkError::a(response) => {
                    //     if response.body_as_str().contains("ThrottlingException") {
                    //         warn!("Retry batch_write_item : {}", err);
                    //         true
                    //     } else {
                    //         false
                    //     }
                    // }
                    _ => false,
                })
                .await
        }
        None => {
            ddb.batch_write_item()
                .set_request_items(Some(request_items))
                .send()
                .await
        }
    };
    match res {
        Ok(res) => Ok(res.unprocessed_items),
        Err(e) => Err(e),
    }
}

// Basically this function is intended to be defined as `pub async fn`.
// However, to recursively use async function, you have to return a future wrapped by pinned box. For more details: `rustc --explain E0733`.
pub fn batch_write_untill_processed(
    cx: app::Context,
    request_items: HashMap<String, Vec<WriteRequest>>,
) -> Pin<Box<dyn Future<Output = Result<(), aws_sdk_dynamodb::error::SdkError<BatchWriteItemError>>>>>
{
    Box::pin(async move {
        match batch_write_item_api(cx.clone(), request_items).await {
            Ok(result) => {
                let unprocessed_items: HashMap<String, Vec<WriteRequest>> =
                    result.expect("alwasy wrapped by Some");
                // if there's any unprocessed items, recursively call this function itself.
                if !unprocessed_items.is_empty() {
                    debug!("UnprocessedItems: {:?}", &unprocessed_items);
                    batch_write_untill_processed(cx, unprocessed_items).await
                }
                // untill it processes items completely.
                else {
                    Ok(())
                }
            }
            Err(e) => Err(e),
        }
    })
}

/// This function is intended to be called from main.rs, as a destination of bwrite command.
/// It executes batch write operations based on the provided `puts`, `dels`, and `input_file` arguments.
/// At least one argument `puts`, `dels` or `input_file` is required, and all arguments can be specified simultaneously.
pub async fn batch_write_item(
    cx: app::Context,
    puts: Option<Vec<String>>,
    dels: Option<Vec<String>>,
    input_file: Option<String>,
) -> Result<(), DyneinBatchError> {
    // validate the input arguments
    if puts.is_none() && dels.is_none() && input_file.is_none() {
        return Err(DyneinBatchError::InvalidInput(String::from(
            "must provide at least one argument for 'bwrite' command",
        )));
    }

    let mut bwrite_items = HashMap::<String, Vec<WriteRequest>>::new();

    // Only use write_requests, parser and ts if `--puts` or `--dels` option is provided.
    if puts.is_some() || dels.is_some() {
        let mut write_requests = Vec::<WriteRequest>::new();
        let parser = DyneinParser::new();
        let ts: app::TableSchema = app::table_schema(&cx).await;

        if let Some(items) = puts {
            for item in items.iter() {
                let attrs = parser.parse_dynein_format(None, item)?;
                validate_item_keys(&attrs, &ts)?;
                write_requests.push(
                    WriteRequest::builder()
                        .put_request(PutRequest::builder().set_item(Some(attrs)).build().unwrap())
                        .build(),
                );
            }
        }

        if let Some(keys) = dels {
            for key in keys.iter() {
                let attrs = parser.parse_dynein_format(None, key)?;
                validate_item_keys(&attrs, &ts)?;
                write_requests.push(
                    WriteRequest::builder()
                        .delete_request(
                            DeleteRequest::builder()
                                .set_key(Some(attrs))
                                .build()
                                .unwrap(),
                        )
                        .build(),
                );
            }
        }

        bwrite_items.insert(ts.name, write_requests);
    }

    if let Some(file_path) = input_file {
        let content = fs::read_to_string(file_path)?;
        debug!("string content: {}", content);
        let items_from_json = build_batch_request_items_from_json(content)?;
        debug!("built items for batch from json: {:?}", items_from_json);

        // merge file items passed by `--input` option.
        for (tbl, mut ops) in items_from_json {
            bwrite_items
                .entry(tbl)
                .and_modify(|e| e.append(&mut ops))
                .or_insert(ops);
        }
    }

    debug!("built items for batch: {:?}", bwrite_items);
    batch_write_item_api(cx, bwrite_items).await?;
    Ok(())
}

/// This function takes cx (just for table name) and Vec<JsonValue>, where this JsonValue consists of multiple items as a standard JSON format,
///   then returns a HashMap from table name to Vec<WriteRequest>.
///   The returned HashMap can be used for a value of "RequestItems" parameter in BatchWriteItem API. https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_BatchWriteItem.html
/// Note that this function assumes that target table is only one table.
pub async fn convert_jsonvals_to_request_items(
    cx: &app::Context,
    items_jsonval: Vec<JsonValue>,
    enable_set_inference: bool,
) -> Result<HashMap<String, Vec<WriteRequest>>, DyneinBatchError> {
    let mut results = HashMap::<String, Vec<WriteRequest>>::new();
    let mut write_requests = Vec::<WriteRequest>::new();

    for item_jsonval in items_jsonval {
        // Focusing on an item - iterate over attributes in an item.
        let mut item = HashMap::<String, AttributeValue>::new();
        for (attr_name, body) in item_jsonval
            .as_object()
            .expect("should be valid JSON object")
            .iter()
        {
            item.insert(
                attr_name.to_string(),
                data::dispatch_jsonvalue_to_attrval(body, enable_set_inference),
            );
        }

        // Fill meaningful put_request here, then push it to the write_requests. Then go to the next item.
        write_requests.push(
            WriteRequest::builder()
                .put_request(PutRequest::builder().set_item(Some(item)).build().unwrap())
                .build(),
        );
    }

    // A single table name as a key, and insert all (up to 25) write_requests under the single table.
    results.insert(cx.effective_table_name(), write_requests);

    Ok(results)
}

/// "matrix" is a vector of vectors. These internal vectors has strs, each of them is an attribute for an item.
///
/// e.g.
///    name, age, fruit ... headers
/// [[John, 12, Apple],
///  [Ami, 23, Orange],
///  [Shu, 42, Banana]] ... matrix
pub async fn csv_matrix_to_request_items(
    cx: &app::Context,
    matrix: &[Vec<&str>],
    headers: &[&str],
    enable_set_inference: bool,
) -> Result<HashMap<String, Vec<WriteRequest>>, DyneinBatchError> {
    let total_elements_in_matrix: usize = matrix
        .iter()
        .map(|x| x.len())
        .collect::<Vec<usize>>()
        .iter()
        .sum::<usize>();
    if (headers.len() * matrix.len()) != total_elements_in_matrix {
        error!("cells in the 'matrix' should have exact the same number of elements of 'headers'");
        std::process::exit(1);
    }

    let mut results = HashMap::<String, Vec<WriteRequest>>::new();
    let mut write_requests = Vec::<WriteRequest>::new();

    for cells in matrix {
        // Build an item. Note that DynamoDB data type of attributes are left to how serde_json::from_str parse the value in the cell.
        let mut item = HashMap::<String, AttributeValue>::new();
        for i in 0..headers.len() {
            let jsonval = serde_json::from_str(cells[i])?;
            debug!(
                "CSV cell '{:?}' --serde_json::from_str--> JsonValue: {:?}",
                cells[i], jsonval
            );
            item.insert(
                headers[i].to_string(),
                data::dispatch_jsonvalue_to_attrval(&jsonval, enable_set_inference),
            );
        }

        // Fill meaningful put_request here, then push it to the write_requests. Then go to the next item.
        write_requests.push(
            WriteRequest::builder()
                .put_request(PutRequest::builder().set_item(Some(item)).build().unwrap())
                .build(),
        );
    }

    // A single table name as a key, and insert all (up to 25) write_requests under the single table.
    results.insert(cx.effective_table_name(), write_requests);

    Ok(results)
}

/* =================================================
Private functions
================================================= */

/// As input is DynamoDB JSON, all JsonValue would be 'Object', 'String', or maybe document types.
/// Input format (DynamoDB JSON) is where this function differs from `dispatch_jsonvalue_to_attrval`, which accepts from 'standard' human readable JSON.
/// Input example:
///     Object({
///         "Category": Object( { "S": String( "Amazon Web Services",), },),
///         "Messages": Object( { "N": String( "4",), },),
///         "Name": Object( { "S": String( "Amazon DynamoDB",), },),
///         "Threads": Object( { "N": String( "2",), },),
///         "Views": Object( { "N": String( "1000",), },),
///     },)
fn ddbjson_attributes_to_attrvals(
    ddbjson_attributes: &JsonValue,
) -> HashMap<String, AttributeValue> {
    let mut built_attributes = HashMap::<String, AttributeValue>::new();
    for (attribute_name, body) in ddbjson_attributes
        .as_object()
        .expect("should be valid JSON object")
        .iter()
    {
        debug!("attribute name is: {}, body is: {:?}", attribute_name, body);

        let attr_val: Option<AttributeValue> = ddbjson_val_to_attrval(body);

        match attr_val {
            Some(v) => {
                built_attributes.insert(attribute_name.to_string(), v);
            }
            None => error!(
                "[skip] invalid/unsupported DynamoDB JSON format: {:?}",
                body
            ),
        };
    }
    built_attributes
}

/// Input is a single attribute value (i.e. a String attribute) in DynamoDB JSON format.
/// Input example (N):
///     Object( { "N": String( "4",), },)
///
/// Input example (L):
///     Array([
///         Object({"S": String("Red")}),
///         Object({"S": String("Black")})])
///
/// Input example (M):
///     Object({"M": Object({
///              "Name": Object({"S": String("Joe")})})}),
///              "Age": Object({"N": String("35")}),
///              "Misc": Object({
///                  "M": Object({
///                      "hope": Object({"BOOL": Bool(true)})})}),
///                      "dream": Object({
///                          "L": Array([
///                              Object({"N": String("35")}),
///                              Object({"NULL": Bool(true)})])}),
fn ddbjson_val_to_attrval(ddb_jsonval: &JsonValue) -> Option<AttributeValue> {
    // prepare shared logic that can be used for both SS and NS.
    let set_logic = |val: &JsonValue| -> Vec<String> {
        val.as_array()
            .expect("should be valid JSON array")
            .iter()
            .map(|el| el.as_str().expect("should -> str").to_string())
            .collect::<Vec<String>>()
    };

    // following list of if-else statements would be return value of this function.
    if let Some(x) = ddb_jsonval.get("S") {
        Some(AttributeValue::S(x.as_str().unwrap().to_string()))
    } else if let Some(x) = ddb_jsonval.get("N") {
        Some(AttributeValue::N(x.as_str().unwrap().to_string()))
    } else if let Some(x) = ddb_jsonval.get("B") {
        Some(AttributeValue::B(aws_sdk_dynamodb::primitives::Blob::new(
            json_binary_val_to_bytes(x),
        )))
    } else if let Some(x) = ddb_jsonval.get("BOOL") {
        Some(AttributeValue::Bool(x.as_bool().unwrap()))
    } else if let Some(x) = ddb_jsonval.get("SS") {
        Some(AttributeValue::Ss(set_logic(x)))
    } else if let Some(x) = ddb_jsonval.get("NS") {
        Some(AttributeValue::Ns(set_logic(x)))
    } else if let Some(x) = ddb_jsonval.get("BS") {
        let binary_set = x
            .as_array()
            .expect("should be valid JSON array")
            .iter()
            .map(json_binary_val_to_bytes)
            .map(aws_sdk_dynamodb::primitives::Blob::new)
            .collect::<Vec<aws_sdk_dynamodb::primitives::Blob>>();
        debug!("Binary Set: {:?}", binary_set);
        Some(AttributeValue::Bs(binary_set))
    } else if let Some(x) = ddb_jsonval.get("L") {
        let list_element = x
            .as_array()
            .unwrap()
            .iter()
            .map(|el| ddbjson_val_to_attrval(el).expect("failed to digest a list element"))
            .collect::<Vec<AttributeValue>>();
        debug!("List Element: {:?}", list_element);
        Some(AttributeValue::L(list_element))
    } else if let Some(x) = ddb_jsonval.get("M") {
        let inner_map: HashMap<String, AttributeValue> = ddbjson_attributes_to_attrvals(x);
        Some(AttributeValue::M(inner_map))
    } else if ddb_jsonval.get("NULL").is_some() {
        Some(AttributeValue::Null(true))
    } else {
        None
    }
}

//  Decodes a base64 encoded binary value to Bytes.
fn json_binary_val_to_bytes(v: &JsonValue) -> Bytes {
    Bytes::from(
        general_purpose::STANDARD
            .decode(v.as_str().expect("binary inputs should be string value"))
            .expect("binary inputs should be base64 with padding encoded"),
    )
}

// Check if the item has a partition key and sort key.
fn validate_item_keys(
    attrs: &HashMap<String, AttributeValue>,
    ts: &app::TableSchema,
) -> Result<(), DyneinBatchError> {
    if !attrs.contains_key(&ts.pk.name) {
        return Err(DyneinBatchError::InvalidInput(format!(
            "must provide the partition key attribute {}",
            ts.pk.name
        )));
    }
    validate_key_type(&ts.pk.name, &ts.pk.kind, attrs)?;

    if let Some(sk) = &ts.sk {
        if !attrs.contains_key(&sk.name) {
            return Err(DyneinBatchError::InvalidInput(format!(
                "must provide the sort key attribute {}",
                sk.name
            )));
        }
        validate_key_type(&sk.name, &sk.kind, attrs)?;
    }

    Ok(())
}

fn validate_key_type(
    key_name: &str,
    expected_key_type: &key::KeyType,
    attrs: &HashMap<String, AttributeValue>,
) -> Result<(), DyneinBatchError> {
    match expected_key_type {
        key::KeyType::S => {
            if attrs[key_name].as_s().is_err() {
                return Err(DyneinBatchError::InvalidInput(
                    generate_type_mismatch_error_message(key_name, "String"),
                ));
            }
        }
        key::KeyType::N => {
            if attrs[key_name].as_n().is_err() {
                return Err(DyneinBatchError::InvalidInput(
                    generate_type_mismatch_error_message(key_name, "Number"),
                ));
            }
        }
        key::KeyType::B => {
            if attrs[key_name].as_b().is_err() {
                return Err(DyneinBatchError::InvalidInput(
                    generate_type_mismatch_error_message(key_name, "Binary"),
                ));
            }
        }
    }

    Ok(())
}

fn generate_type_mismatch_error_message(attr_name: &str, expected_type: &str) -> String {
    format!(
        "type mismatch for the key {}, expected: {}",
        attr_name, expected_type
    )
}
