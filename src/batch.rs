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
    error,
    fmt,
    fs,
    future::Future,
    io::Error as IOError,
    pin::Pin,
};
use log::{debug,error};
use rusoto_core::RusotoError;
use rusoto_dynamodb::*;
use serde_json::Value as JsonValue;

use super::app;
use super::data;


/* =================================================
   struct / enum / const
   ================================================= */

#[derive(Debug)]
pub enum DyneinBatchError {
    LoadData(IOError),
    PraseJSON(serde_json::Error),
    BatchWriteError(RusotoError<BatchWriteItemError>),
}
impl fmt::Display for DyneinBatchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { match *self {
        DyneinBatchError::LoadData(ref e)  => e.fmt(f),
        DyneinBatchError::PraseJSON(ref e) => e.fmt(f),
        DyneinBatchError::BatchWriteError(ref e) => e.fmt(f),
    } }
}
impl error::Error for DyneinBatchError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> { match *self {
        DyneinBatchError::LoadData(ref e)  => Some(e),
        DyneinBatchError::PraseJSON(ref e) => Some(e),
        DyneinBatchError::BatchWriteError(ref e) => Some(e),
    } }
}
impl From<IOError> for DyneinBatchError { fn from(e: IOError) -> DyneinBatchError { DyneinBatchError::LoadData(e) } }
impl From<serde_json::Error> for DyneinBatchError { fn from(e: serde_json::Error) -> DyneinBatchError { DyneinBatchError::PraseJSON(e) } }
impl From<RusotoError<BatchWriteItemError>> for DyneinBatchError { fn from(e: RusotoError<BatchWriteItemError>) -> DyneinBatchError { DyneinBatchError::BatchWriteError(e) } }


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
pub fn build_batch_request_items(raw_json_content: String) -> Result<HashMap<String, Vec<WriteRequest>>, serde_json::Error> {
    let mut results = HashMap::<String, Vec<WriteRequest>>::new();
    debug!("Trying to convert given string into Batch Request Items: {}", raw_json_content);

    let hashmap: HashMap::<String, JsonValue> = serde_json::from_str(&raw_json_content)?;

    // for each table name as a key, multiple operations are included.
    for (tbl /* String */, operations /* JsonValue */ ) in hashmap.clone() {
        let mut write_requests = Vec::<WriteRequest>::new();
        let ops: &Vec<JsonValue> = operations.as_array().expect("should be array of put/delete operations");

        // each "operation" is PutRequest or DeleteRequest. convert them into DynamoDB AttributeValue and push into WriteRequest vector.
        for op in ops {
            if let Some(wrapped_item /* JsonValue */ ) = op.get("PutRequest") {
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
                    let item: HashMap<String, AttributeValue> = ddbjson_attributes_to_attrvals(raw_item);
                    write_requests.push(WriteRequest { put_request: Some(PutRequest { item: item }), delete_request: None });
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
                    let key: HashMap<String, AttributeValue> = ddbjson_attributes_to_attrvals(raw_key);
                    write_requests.push(WriteRequest { put_request: None, delete_request: Some(DeleteRequest { key: key }) });
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
async fn batch_write_item_api(cx: app::Context, request_items: HashMap<String, Vec<WriteRequest>>)
                                 -> Result<Option<HashMap<String, Vec<WriteRequest>>>, RusotoError<BatchWriteItemError>> {
    debug!("Calling BatchWriteItem API with request_items: {:?}", &request_items);

    let ddb = DynamoDbClient::new(cx.effective_region());
    let req: BatchWriteItemInput = BatchWriteItemInput {
        request_items: request_items,
        ..Default::default()
    };

    match ddb.batch_write_item(req).await {
        Ok(res) => Ok(res.unprocessed_items),
        Err(e) => Err(e),
    }
}


// Basically this function is intended to be defined as `pub async fn`.
// However, to recursively use async function, you have to return a future wrapped by pinned box. For more details: `rustc --explain E0733`.
pub fn batch_write_untill_processed(cx: app::Context, request_items: HashMap<String, Vec<WriteRequest>>)
                                 -> Pin<Box<dyn Future<Output = Result<(), RusotoError<BatchWriteItemError>>>>> {
    Box::pin(async move {
        match batch_write_item_api(cx.clone(), request_items).await {
            Ok(result) => {
                let unprocessed_items: HashMap<String, Vec<WriteRequest>> = result.expect("alwasy wrapped by Some");
                // if there's any unprocessed items, recursively call this function itself.
                if unprocessed_items.len() > 0 {
                    debug!("UnprocessedItems: {:?}", &unprocessed_items);
                    batch_write_untill_processed(cx, unprocessed_items).await
                }
                // untill it processes items completely.
                else { Ok(()) }
            },
            Err(e) => Err(e),
        }
    })
}


/// This function is intended to be called from main.rs, as a destination of bwrite command.
pub async fn batch_write_item(cx: app::Context, input_file: String) -> Result<(), DyneinBatchError> {
    let content = fs::read_to_string(input_file)?;
    debug!("string content: {}", content);
    let items = build_batch_request_items(content)?;
    debug!("built items for batch: {:?}", items);
    batch_write_item_api(cx, items).await?;
    Ok(())
}


/// This function takes cx (just for table name) and Vec<JsonValue>, where this JsonValue consists of multiple items as a standard JSON format,
///   then returns a HashMap from table name to Vec<WriteRequest>.
///   The returned HashMap can be used for a value of "RequestItems" parameter in BatchWriteItem API. https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_BatchWriteItem.html
/// Note that this function assumes that target table is only one table.
pub async fn convert_jsonvals_to_request_items(cx: &app::Context, items_jsonval: Vec<JsonValue>)
                                                -> Result<HashMap<String, Vec<WriteRequest>>, DyneinBatchError> {
    let mut results = HashMap::<String, Vec<WriteRequest>>::new();
    let mut write_requests = Vec::<WriteRequest>::new();

    for item_jsonval in items_jsonval {
        // Initialize a WriteRequest, which consists of a put_request for a single item.
        let mut write_request = WriteRequest { delete_request: None, put_request: None };

        // Focusing on an item - iterate over attributes in an item.
        let mut item = HashMap::<String, AttributeValue>::new();
        for (attr_name, body) in item_jsonval.as_object().expect("should be valid JSON object").iter() {
            item.insert(attr_name.to_string(), data::dispatch_jsonvalue_to_attrval(&body));
        };

        // Fill meaningful put_request here, then push it to the write_requests. Then go to the next item.
        write_request.put_request = Some(PutRequest { item: item });
        write_requests.push(write_request);
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
pub async fn csv_matrix_to_request_items(cx: &app::Context, matrix: &Vec<Vec<&str>>, headers: &Vec<&str>)
                  -> Result<HashMap<String, Vec<WriteRequest>>, DyneinBatchError> {
    let total_elements_in_matrix: usize = matrix.iter().map(|x| x.len()).collect::<Vec<usize>>().iter().sum::<usize>();
    if !(headers.len() * matrix.len() == total_elements_in_matrix) {
        error!("cells in the 'matrix' should have exact the same number of elements of 'headers'"); std::process::exit(1);
    }

    let mut results = HashMap::<String, Vec<WriteRequest>>::new();
    let mut write_requests = Vec::<WriteRequest>::new();

    for cells in matrix {
        // Initialize a WriteRequest, which consists of a put_request for a single item.
        let mut write_request = WriteRequest { delete_request: None, put_request: None };

        // Build an item. Note that DynamoDB data type of attributes are left to how serde_json::from_str parse the value in the cell.
        let mut item = HashMap::<String, AttributeValue>::new();
        for i in 0..headers.len() {
            let jsonval = serde_json::from_str(cells[i])?;
            debug!("CSV cell '{:?}' --serde_json::from_str--> JsonValue: {:?}", cells[i], jsonval);
            item.insert(headers[i].to_string(), data::dispatch_jsonvalue_to_attrval(&jsonval));
        }

        // Fill meaningful put_request here, then push it to the write_requests. Then go to the next item.
        write_request.put_request = Some(PutRequest { item: item });
        write_requests.push(write_request);
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
fn ddbjson_attributes_to_attrvals(ddbjson_attributes: &JsonValue) -> HashMap<String, AttributeValue> {
    let mut built_attributes = HashMap::<String, AttributeValue>::new();
    for (attribute_name, body) in ddbjson_attributes.as_object().expect("should be valid JSON object").iter() {
        debug!("attribute name is: {}, body is: {:?}", attribute_name, body);

        let attr_val: Option<AttributeValue> = ddbjson_val_to_attrval(body);

        match attr_val {
            Some(v) => { built_attributes.insert(attribute_name.to_string(), v); },
            None => { error!("[skip] invalid/unsupported DynamoDB JSON format: {:?}", body) },
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
        val.as_array().expect("should be valid JSON array").iter()
                      .map(|el| el.as_str().expect("should -> str").to_string()).collect::<Vec<String>>()
    };

    // following list of if-else statements would be return value of this function.
         if let Some(x) = ddb_jsonval.get("S") { Some(AttributeValue { s: Some(x.as_str().unwrap().to_string()), ..Default::default() }) }
    else if let Some(x) = ddb_jsonval.get("N") { Some(AttributeValue { n: Some(x.as_str().unwrap().to_string()), ..Default::default() }) }
    // else if let Some(x) = ddb_jsonval.get("B") { Some(AttributeValue { b: Some(Bytes::from(x.as_str().unwrap())), ..Default::default() }) }
    else if let Some(x) = ddb_jsonval.get("BOOL") { Some(AttributeValue { bool: Some(x.as_bool().unwrap()), ..Default::default() }) }
    else if let Some(x) = ddb_jsonval.get("SS") { Some(AttributeValue { ss: Some(set_logic(x)), ..Default::default() }) }
    else if let Some(x) = ddb_jsonval.get("NS") { Some(AttributeValue { ns: Some(set_logic(x)), ..Default::default() }) }
    else if let Some(x) = ddb_jsonval.get("L") {
        let list_element = x.as_array().unwrap().iter()
                            .map(|el| ddbjson_val_to_attrval(el).expect("failed to digest a list element"))
                            .collect::<Vec<AttributeValue>>();
        debug!("List Element: {:?}", list_element);
        Some(AttributeValue { l: Some(list_element), ..Default::default() })
    }
    else if let Some(x) = ddb_jsonval.get("M") {
        let inner_map: HashMap<String, AttributeValue> = ddbjson_attributes_to_attrvals(x);
        Some(AttributeValue { m: Some(inner_map), ..Default::default() })
    }
    else if ddb_jsonval.get("NULL").is_some() { Some(AttributeValue { null: Some(true), ..Default::default() }) }
    else { None }
}
