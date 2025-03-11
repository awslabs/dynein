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

// This module interact with DynamoDB Data Plane APIs
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    io::{self, Write},
    vec::Vec,
};

use crate::parser::{AttributeDefinition, AttributeType, DyneinParser, ParseError};
use aws_sdk_dynamodb::{
    operation::scan::ScanOutput,
    types::{AttributeValue, ReturnValue},
    Client as DynamoDbSdkClient,
};
use log::{debug, error};
use serde::{ser::SerializeStruct, Serialize, Serializer};
use serde_json::Value as JsonValue;
use tabwriter::TabWriter;
// use bytes::Bytes;

use super::app;
use super::ddb::key;

/* =================================================
struct / enum / const
================================================= */

#[derive(Debug)]
struct GeneratedQueryParams {
    exp: Option<String>,
    names: Option<HashMap<String, String>>,
    vals: Option<HashMap<String, AttributeValue>>,
}

#[derive(Debug)]
struct GeneratedScanParams {
    exp: Option<String>,
    names: Option<HashMap<String, String>>,
}

#[derive(Debug)]
struct GeneratedUpdateParams {
    exp: Option<String>,
    names: Option<HashMap<String, String>>,
    vals: Option<HashMap<String, AttributeValue>>,
}

enum UpdateActionType {
    Set,
    Remove,
}

#[derive(Debug)]
pub enum DyneinQueryParamsError {
    NoSuchIndex(String /* index name */, String /* table name */),
    NoSortKeyDefined,
    InvalidSortKeyOption(ParseError),
}

impl From<ParseError> for DyneinQueryParamsError {
    fn from(err: ParseError) -> Self {
        DyneinQueryParamsError::InvalidSortKeyOption(err)
    }
}

impl fmt::Display for DyneinQueryParamsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            DyneinQueryParamsError::NoSuchIndex(idx, t) => {
                write!(
                    f,
                    "No index named '{}' found on the target table '{}'. \
                    Please execute 'dy desc' command to see indexes the table has.",
                    idx, t
                )
            }
            DyneinQueryParamsError::NoSortKeyDefined => {
                write!(
                    f,
                    "You've passed --sort-key (-s) option, \
                    however the target table (or index) doesn't have sort key. \
                    Please execute 'dy desc' command to see key schema."
                )
            }
            DyneinQueryParamsError::InvalidSortKeyOption(err) => {
                write!(
                    f,
                    "{}\n--sort-key syntax is invalid. \
                    This option accepts one of the following styles: \
                    '= 123', '> 123', '>= 123', '< 123', '<= 123', \
                    'between 10 and 99', or 'begins_with \"prefix\"'. \
                    For more information, please visit \
                    https://github.com/awslabs/dynein/blob/main/docs/query.md.",
                    err
                )
            }
        }
    }
}
impl Error for DyneinQueryParamsError {}

/* =================================================
Public functions
================================================= */

/// This function calls Scan API and return mutiple items. By default it uses 'table' output format.
/// Scan API retrieves all items in a given table, something like `SELECT * FROM mytable` in SQL world.
pub async fn scan(
    cx: &app::Context,
    index: Option<String>,
    consistent_read: bool,
    attributes: &Option<String>,
    keys_only: bool,
    limit: i32,
) {
    let ts: app::TableSchema = app::table_schema(cx).await;

    let items = scan_api(
        cx,
        index,
        consistent_read,
        attributes,
        keys_only,
        Some(limit),
        None,
    )
    .await
    .items
    .expect("items should be 'Some' even if there's no item in the table.");
    match cx.output.as_deref() {
        None | Some("table") => display_items_table(items, &ts, attributes, keys_only),
        Some("json") => println!(
            "{}",
            serde_json::to_string_pretty(&convert_to_json_vec(&items)).unwrap()
        ),
        Some("raw") => println!(
            "{}",
            serde_json::to_string_pretty(&strip_items(&items)).unwrap()
        ),
        Some(o) => {
            println!("ERROR: unsupported output type '{}'.", o);
            std::process::exit(1);
        }
    }
}

pub async fn scan_api(
    cx: &app::Context,
    index: Option<String>,
    consistent_read: bool,
    attributes: &Option<String>,
    keys_only: bool,
    limit: Option<i32>,
    esk: Option<HashMap<String, AttributeValue>>,
) -> ScanOutput {
    debug!("context: {:#?}", &cx);
    let ts: app::TableSchema = app::table_schema(cx).await;

    let scan_params: GeneratedScanParams = generate_scan_expressions(&ts, attributes, keys_only);

    let ddb = &cx.ddb_client;

    ddb.scan()
        .table_name(ts.name)
        .set_index_name(index)
        .set_limit(limit)
        .set_projection_expression(scan_params.exp)
        .set_expression_attribute_names(scan_params.names)
        .consistent_read(consistent_read)
        .set_exclusive_start_key(esk)
        .send()
        .await
        .unwrap_or_else(|e| {
            debug!("Scan API call got an error -- {:?}", e);
            app::bye_with_sdk_error(1, e);
        })
}

pub struct QueryParams {
    pub pval: String,
    pub sort_key_expression: Option<String>,
    pub index: Option<String>,
    pub limit: Option<i32>,
    pub consistent_read: bool,
    pub descending: bool,
    pub attributes: Option<String>,
    pub keys_only: bool,
}

/// This function calls Query API and return mutiple items. By default it uses 'table' output format.
/// Partition key is required. Optionally you can pass key condition expression to search more specific set of items using sort key.
/// References:
/// - https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Query.html#Query.KeyConditionExpressions
/// - https://aws.amazon.com/blogs/database/using-sort-keys-to-organize-data-in-amazon-dynamodb/
pub async fn query(cx: &app::Context, params: QueryParams) {
    debug!("context: {:#?}", &cx);
    let ts: app::TableSchema = app::table_schema(cx).await;

    debug!("For table '{}' (index '{:?}'), generating KeyConditionExpression using sort_key_expression: '{:?}'", &ts.name, &params.index, &params.sort_key_expression);
    let query_params: GeneratedQueryParams = match generate_query_expressions(
        &ts,
        &params.pval,
        &params.sort_key_expression,
        &params.index,
        cx.should_strict_for_query(),
    ) {
        Ok(qp) => qp,
        Err(e) => {
            error!("{}", e.to_string());
            std::process::exit(1);
        }
    };
    debug!(
        "Generated QueryParams for the table '{}' is: {:#?}",
        &ts.name, &query_params
    );

    let ddb = &cx.ddb_client;

    let req = ddb
        .query()
        .table_name(ts.name.to_string())
        .set_index_name(params.index)
        .set_limit(params.limit)
        .set_key_condition_expression(query_params.exp)
        .set_expression_attribute_names(query_params.names)
        .set_expression_attribute_values(query_params.vals)
        .consistent_read(params.consistent_read)
        .set_scan_index_forward(params.descending.then_some(false));
    debug!("Request: {:#?}", req);

    match req.send().await {
        Ok(res) => {
            match res.items {
                None => panic!("This message should not be shown"), // as Query returns 'Some([])' if there's no item to return.
                Some(items) => match cx.output.as_deref() {
                    None | Some("table") => {
                        display_items_table(items, &ts, &params.attributes, params.keys_only)
                    }
                    Some("json") => println!(
                        "{}",
                        serde_json::to_string_pretty(&convert_to_json_vec(&items)).unwrap()
                    ),
                    Some("raw") => println!(
                        "{}",
                        serde_json::to_string_pretty(&strip_items(&items)).unwrap()
                    ),
                    Some(o) => {
                        println!("ERROR: unsupported output type '{}'.", o);
                        std::process::exit(1);
                    }
                },
            }
        }
        Err(e) => {
            debug!("Query API call got an error -- {:?}", e);
            app::bye_with_sdk_error(1, e);
        }
    }
}

/// This function calls GetItem API - get an item with given primary key(s). By default it uses 'json' output format.
pub async fn get_item(
    cx: &app::Context,
    pval: String,
    sval: Option<String>,
    consistent_read: bool,
) {
    debug!("context: {:#?}", &cx);
    // Use table if explicitly specified by `--table/-t` option. Otherwise, load table name from config file.
    let ts: app::TableSchema = app::table_schema(cx).await;
    let primary_keys = identify_target(&ts, &pval, sval.as_deref());

    debug!(
        "Calling GetItem API for the table '{}' with key(s): {:?}",
        &ts.name, &primary_keys
    );

    let ddb = &cx.ddb_client;

    match ddb
        .get_item()
        .table_name(ts.name)
        .set_key(Some(primary_keys))
        .consistent_read(consistent_read)
        .send()
        .await
    {
        Ok(res) => match res.item {
            None => println!("No item found."),
            Some(item) => match cx.output.as_deref() {
                None | Some("json") => println!(
                    "{}",
                    serde_json::to_string_pretty(&convert_to_json(&item)).unwrap()
                ),
                Some("yaml") => println!(
                    "{}",
                    serde_yaml::to_string(&convert_to_json(&item)).unwrap()
                ),
                Some("raw") => println!(
                    "{}",
                    serde_json::to_string_pretty(&strip_item(&item)).unwrap()
                ),
                Some(o) => {
                    println!("ERROR: unsupported output type '{}'.", o);
                    std::process::exit(1);
                }
            },
        },
        Err(e) => {
            debug!("GetItem API call got an error -- {:?}", e);
            app::bye_with_sdk_error(1, e);
        }
    }
}

// put_item function saves an item with given primary key(s). You can pass other attributes with --item/-i option in JSON format.
// As per DynamoDB PutItem API behavior, if the item already exists it'd be replaced.
pub async fn put_item(cx: &app::Context, pval: String, sval: Option<String>, item: Option<String>) {
    debug!("context: {:#?}", &cx);
    let ts: app::TableSchema = app::table_schema(cx).await;
    let mut full_item_image = identify_target(&ts, &pval, sval.as_deref()); // Firstly, ideitify primary key(s) to ideitnfy an item to put.

    debug!(
        "Inserting (or replacing) an item identified by the primary key(s): {:?}",
        &full_item_image
    );

    // merge additional items passed by `--item/-i` option.
    match item {
        None => (),
        Some(_i) => {
            let parser = DyneinParser::new();
            let result = parser.parse_dynein_format(Some(full_item_image), &_i);
            match result {
                Ok(attrs) => {
                    full_item_image = attrs;
                }
                Err(e) => {
                    error!("ERROR: failed to load item. {:?}", e);
                    std::process::exit(1);
                }
            };
        }
    };

    debug!("Calling PutItem API to insert: {:?}", &full_item_image);

    let ddb = &cx.ddb_client;

    match ddb
        .put_item()
        .table_name(ts.name.to_string())
        .set_item(Some(full_item_image))
        .send()
        .await
    {
        Ok(_) => {
            println!("Successfully put an item to the table '{}'.", &ts.name);
        }
        Err(e) => {
            debug!("PutItem API call got an error -- {:?}", e);
            app::bye_with_sdk_error(1, e);
        }
    }
}

// delete_item functions calls DeleteItem API - delete an item with given primary key(s).
pub async fn delete_item(cx: &app::Context, pval: String, sval: Option<String>) {
    debug!("context: {:#?}", &cx);
    let ts: app::TableSchema = app::table_schema(cx).await;
    let primary_keys = identify_target(&ts, &pval, sval.as_deref());

    debug!(
        "Calling DeleteItem API for the table '{}' with key(s): {:?}",
        &ts.name, &primary_keys
    );

    let ddb = &cx.ddb_client;

    match ddb
        .delete_item()
        .table_name(ts.name.to_string())
        .set_key(Some(primary_keys))
        .send()
        .await
    {
        // NOTE: DynamoDB DeleteItem API is idempotent and returns "OK" even if an item trying to delete doesn't exist.
        Ok(_) => {
            println!(
                "Successfully deleted an item from the table '{}'.",
                &ts.name
            );
        }
        Err(e) => {
            debug!("Deletetem API call got an error -- {:?}", e);
            app::bye_with_sdk_error(1, e);
        }
    }
}

// UpdateItem API https://docs.aws.amazon.com/amazondynamodb/latest/APIReference/API_UpdateItem.html
pub async fn update_item(
    cx: &app::Context,
    pval: String,
    sval: Option<String>,
    set_expression: Option<String>,
    remove_expression: Option<String>,
) {
    debug!("context: {:#?}", &cx);
    if set_expression.is_none() && remove_expression.is_none() {
        // setting both --set and --remove is prohibited by conflicts_with of clap
        error!("One of --set or --remove option is required. Passing both options is invalid.");
        std::process::exit(1);
    };

    let ts: app::TableSchema = app::table_schema(cx).await;
    let primary_keys = identify_target(&ts, &pval, sval.as_deref());

    debug!(
        "Calling UpdateItem API for the table '{}' with key(s): {:?}",
        &ts.name, &primary_keys
    );

    // above logic has checked "only either one of `--set` or `--remove` exist".
    let update_params: GeneratedUpdateParams = if let Some(sx) = set_expression {
        generate_update_expressions(UpdateActionType::Set, &sx)
    } else if let Some(rx) = remove_expression {
        generate_update_expressions(UpdateActionType::Remove, &rx)
    } else {
        panic!("Neither --set nor --remove is not specified, but this should not be catched here.");
    };

    let ddb = &cx.ddb_client;

    match ddb
        .update_item()
        .table_name(ts.name.to_string())
        .set_key(Some(primary_keys))
        .set_update_expression(update_params.exp)
        .set_expression_attribute_names(update_params.names)
        .set_expression_attribute_values(update_params.vals)
        .return_values(ReturnValue::AllNew) // ask DynamoDB to return updated item.
        .send()
        .await
    {
        Ok(res) => {
            println!("Successfully updated an item in the table '{}'.", &ts.name);
            println!(
                "Updated item: {}",
                serde_json::to_string(&convert_to_json(&res.attributes.unwrap())).unwrap()
            );
        }
        Err(e) => {
            debug!("UpdateItem API call got an error -- {:?}", e);
            app::bye_with_sdk_error(1, e);
        }
    }
}

// https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/WorkingWithItems.html#WorkingWithItems.AtomicCounters
pub async fn atomic_counter(
    cx: &app::Context,
    pval: String,
    sval: Option<String>,
    set_expression: Option<String>,
    remove_expression: Option<String>,
    target_attr: String,
) {
    debug!("context: {:#?}", &cx);
    if set_expression.is_some() || remove_expression.is_some() {
        error!("--atomic-counter option cannot be used with --set or --remove.");
        std::process::exit(1);
    };
    let atomic_counter_expression = format!("{} = {} + 1", target_attr, target_attr);
    update_item(cx, pval, sval, Some(atomic_counter_expression), None).await;
}

/* =================================================
Private functions
================================================= */

/*
Basically what this function does is to replace attribute names and values into DynamoDB style placeholders, i.e. "#ATTRNAME" and ":VALUE".
And return UpdateExpression [1] string and supplementary names/values that are saved as HashMaps.
For better UX, dynein automatically replace all tokens into placeholders as it's hard to be aware of which keywords are reserved words [2].

[1]: https://docs.amazonaws.cn/en_us/amazondynamodb/latest/developerguide/Expressions.UpdateExpressions.html
[2]: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/ReservedWords.html

As dynein prefer simple UX over minor use-cases, currently dynein doesn't support all actions in UpdateExpression:
- SET ... Modify/add attribute(s) to/of an item. dynein's `--set` option would generate an expression begins with `SET`.
    - list_append function: `You can add elements to the end of a list`
    - if_not_exists function: `If you want to avoid overwriting an existing attribute`
- REMOVE   ... Remove attribute(s) from an item, or remove element(s) from a list attribute of an item. dynein's `--remove` option would generate an expression begins with `REMOVE`.
- (DELETE) ... dynein doesn't support `DELETE`. Remove element(s) from a set attribute of an item. DELETE supports only Set data types (SS,NS,BS).
- (ADD)    ... dynein doesn't support `ADD`. Per the doc above `In general, we recommend using SET rather than ADD.`

Support status of various examples ([x] = not available for now, [o] = supported):
- [o] "SET Price = :newval" => in dynein: `$ dy update <keys> --set 'Price = 123'`.
- [o] "SET LastPostedBy = :lastpostedby" => in dynein: `$ dy update <keys> --set 'LastPostedBy = "2020-02-24T22:22:22Z"'`.
- [o] "SET Replies = :zero, Status = :stat" => in dynein: `$ dy update <keys> --set 'Replies = 0, Status = "OPEN"'`.
- [o] "SET Replies = :zero, LastPostedBy = :lastpostedby" => in dynein: `$ dy update <keys> --set 'Replies = 0, LastPostedBy = "2020-02-24T22:22:22Z"'`.
- [o] "SET #cls = :val" => in dynein you can pass reserved words normally: `$ dy update <keys> --set 'class = "Math"'`.
- [o] "SET Price = Price + :incr" => --set 'Price = Price + 1' works. If :incr is 1, you can consider using --atomic-counter.
- [o] "SET RelatedItems[1] = :ri" => --set 'RelatedItems[1] = "item1"'
- [o] "SET #pr.#5star[1] = :r5, #pr.#3star = :r3" => --set 'pr.`5star`[1] = 7, pr.`3star` = 3'
- [o] "SET #ri = list_append(#ri, :vals)" => --set 'RelatedItems = list_append(RelatedItems, ["item2"])'
- [o] "SET #ri = list_append(:vals, #ri)" => --set 'RelatedItems = list_append(["item2"], RelatedItems)'
- [o] "SET Price = if_not_exists(Price, :p)" => --set 'Price = if_not_exists(Price, 123)'
- [o] "REMOVE Brand, InStock, QuantityOnHand" => in dynein: `$ dy update <keys> --remove 'Brand, InStock, QuantityOnHand'`.
- [o] "REMOVE RelatedItems[1], RelatedItems[2]" => --remove 'RelatedItems[1], RelatedItems[2]'
*/
fn generate_update_expressions(
    action_type: UpdateActionType,
    given_expression: &str,
) -> GeneratedUpdateParams {
    let mut expression: String = String::from("");
    let names;
    let vals;

    match action_type {
        UpdateActionType::Set => {
            expression.push_str("SET ");
            let mut parser = DyneinParser::new();

            // TODO: the error should bubble up for better error handling.
            let result = parser
                .parse_set_action(given_expression)
                .expect("Failed to parse given expression");
            expression.push_str(&result.get_expression());
            names = result.get_names();
            vals = result.get_values();
        }
        UpdateActionType::Remove => {
            expression.push_str("REMOVE ");
            let mut parser = DyneinParser::new();

            // TODO: the error should bubble up for better error handling.
            let result = parser
                .parse_remove_action(given_expression)
                .expect("Failed to parse given expression");
            expression.push_str(&result.get_expression());
            names = result.get_names();
            vals = result.get_values();
        }
    }; // match action_type

    debug!("generated UpdateExpression: {:?}", expression);
    debug!("generated ExpressionAttributeNames: {:?}", names);
    debug!("generated ExpressionAttributeValues: {:?}", vals);

    GeneratedUpdateParams {
        exp: Some(expression),
        names: if names.is_empty() { None } else { Some(names) },
        vals: if vals.is_empty() { None } else { Some(vals) },
    }
}

// Without `--table/-t` option, `identify_target` utilizes table info stored in config file which is saved via `dy use` command.
// With `--table/-t` option, `identify_target` retrieves primary key(s) info by calling DescribeTable API each time which would consumre additional time.
fn identify_target(
    ts: &app::TableSchema,
    pval: &str,
    optional_sval: Option<&str>,
) -> HashMap<String, AttributeValue> {
    let mut target = HashMap::<String, AttributeValue>::new();
    target.insert(
        ts.pk.name.to_string(),
        build_attrval_scalar(&ts.pk.kind.to_string(), pval),
    );

    // if sort key value is given from command line, add sort key to target HashMap to identify an item.
    if let Some(sval) = optional_sval {
        match ts.sk.as_ref() {
            Some(sk) => target.insert(
                sk.name.to_string(),
                build_attrval_scalar(&sk.kind.to_string(), sval),
            ),
            None => {
                error!("Partition and Sort keys are given to identify an item, but table '{t}' uses Partition key only. Check `dy desc {t}`", t = &ts.name);
                std::process::exit(1);
            }
        };
    }
    debug!(
        "Generated primary key(s) to identify an item: {:?}",
        &target
    );
    target
}

// top 3 scalar types that can be used for primary keys.
//   ref: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.CoreComponents.html
//        https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.NamingRulesDataTypes.html#HowItWorks.DataTypes
fn build_attrval_scalar(_ktype: &str, _kval: &str) -> AttributeValue {
    debug!(
        "Constructing an AttributeValue for (type: {:?}, val: {:?})",
        _ktype, _kval
    );

    match _ktype {
        "S" => AttributeValue::S(String::from(_kval)),
        "N" => AttributeValue::N(String::from(_kval)), // NOTE: pass string, not number
        // "B" => { attrval.b = Some(Bytes::from(_kval.clone().as_str())) },
        _ => panic!("ERROR: Unknown DynamoDB Data Type: {}", _ktype),
    }
}

// for SS and NS DynamoDB Attributes.
// :( serde_json::value::string -- to_string() --> "\"a\""
// :) serde_json::value::string -- as_str() --> some("a") -- unwrap() --> "a"
fn build_attrval_set(ktype: &str, kval: &[JsonValue]) -> AttributeValue {
    debug!(
        "Constructing an AttributeValue for (type: {:?}, val: {:#?})",
        ktype, kval
    );

    match ktype {
        "SS" => AttributeValue::Ss(
            kval.iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect(),
        ),
        "NS" => AttributeValue::Ns(
            kval.iter()
                .map(|x| x.as_i64().unwrap().to_string())
                .collect(),
        ),
        // NOTE: Currently BS is not supported.
        _ => panic!("ERROR: Unknown DynamoDB Data Type: {}", ktype),
    }
}

/// for "L" DynamoDB Attributes
/// used only for 'simplified JSON' format. Not compatible with DynamoDB JSON.
fn build_attrval_list(vec: &[JsonValue], enable_set_inference: bool) -> AttributeValue {
    let mut inside_attrvals = Vec::<AttributeValue>::new();
    for v in vec {
        debug!("this is an element of vec: {:?}", v);
        inside_attrvals.push(dispatch_jsonvalue_to_attrval(v, enable_set_inference));
    }
    AttributeValue::L(inside_attrvals)
}

/// for "M" DynamoDB Attributes
/// used only for 'simplified JSON' format. Not compatible with DynamoDB JSON.
fn build_attrval_map(
    json_map: &serde_json::Map<std::string::String, JsonValue>,
    enable_set_inference: bool,
) -> AttributeValue {
    let mut mapval = HashMap::<String, AttributeValue>::new();
    for (k, v) in json_map {
        debug!("working on key '{}', and value '{:?}'", k, v);
        mapval.insert(
            k.to_string(),
            dispatch_jsonvalue_to_attrval(v, enable_set_inference),
        );
    }
    AttributeValue::M(mapval)
}

/// Convert from serde_json::Value (standard JSON values) into DynamoDB style AttributeValue
pub fn dispatch_jsonvalue_to_attrval(jv: &JsonValue, enable_set_inference: bool) -> AttributeValue {
    match jv {
        // scalar types
        JsonValue::String(val) => AttributeValue::S(val.to_string()),
        JsonValue::Number(val) => AttributeValue::N(val.to_string()),
        JsonValue::Bool(val) => AttributeValue::Bool(*val),
        JsonValue::Null => AttributeValue::Null(true),

        // document types. they can be recursive.
        JsonValue::Object(obj) => build_attrval_map(obj, enable_set_inference),
        JsonValue::Array(vec) => {
            if enable_set_inference && vec.iter().all(|v| v.is_string()) {
                debug!(
                    "All elements in this attribute are String - treat it as 'SS': {:?}",
                    vec
                );
                build_attrval_set(&String::from("SS"), vec)
            } else if enable_set_inference && vec.iter().all(|v| v.is_number()) {
                debug!(
                    "All elements in this attribute are Number - treat it as 'NS': {:?}",
                    vec
                );
                build_attrval_set(&String::from("NS"), vec)
            } else {
                debug!("Elements are not uniform - treat it as 'L': {:?}", vec);
                build_attrval_list(vec, enable_set_inference)
            }
        }
    }
}

struct AttributeValueWrapper(AttributeValue);

impl Serialize for AttributeValueWrapper {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("Object", 1)?;

        match &self.0 {
            AttributeValue::S(v) => state.serialize_field("S", v)?,
            AttributeValue::N(v) => state.serialize_field("N", v)?,
            AttributeValue::Bool(v) => state.serialize_field("BOOL", v)?,
            AttributeValue::Null(_) => state.serialize_field("NULL", &true)?,
            AttributeValue::Ss(v) => state.serialize_field("SS", v)?,
            AttributeValue::Ns(v) => state.serialize_field("NS", v)?,
            AttributeValue::B(v) => {
                state.serialize_field("B", &aws_smithy_types::base64::encode(v))?
            }
            AttributeValue::Bs(v) => state.serialize_field(
                "BS",
                &v.iter()
                    .map(aws_smithy_types::base64::encode)
                    .collect::<Vec<_>>(),
            )?,
            AttributeValue::M(v) => {
                state.serialize_field(
                    "M",
                    &v.iter()
                        .map(|(k, v)| (k, AttributeValueWrapper(v.clone())))
                        .collect::<HashMap<_, _>>(),
                )?;
            }
            AttributeValue::L(v) => state.serialize_field(
                "L",
                &v.iter()
                    .map(|item| AttributeValueWrapper(item.clone()))
                    .collect::<Vec<_>>(),
            )?,
            _ => panic!(
                "DynamoDB AttributeValue is not in valid status: {:#?}",
                &self.0
            ),
        };
        state.end()
    }
}

/// `strip_items` calls `strip_item` for each item.
fn strip_items(
    items: &[HashMap<String, AttributeValue>],
) -> Vec<HashMap<String, serde_json::Value>> {
    items.iter().map(strip_item).collect()
}

/// `strip_item` function strips non-existing data types in AttributeValue struct:
///
///     { "pkA": AttributeValue {
///         b: None,
///         bool: None,
///         bs: None,
///         l: None,
///         m: None,
///         n: None,
///         ns: None,
///         null: None,
///         s: Some("e0a170d9-5ce3-443b-bbce-d0d49c71d151"),
///         ss: None
///     }}
///
/// to something like this:
///
///     { "pkA": { "S": "e0a170d9-5ce3-443b-bbce-d0d49c71d151" }
fn strip_item(item: &HashMap<String, AttributeValue>) -> HashMap<String, serde_json::Value> {
    item.iter()
        .map(|attr| {
            (
                attr.0.to_string(),
                serde_json::to_value(AttributeValueWrapper(attr.1.to_owned())).unwrap(),
            )
        })
        .collect()
}

impl From<key::Key> for AttributeDefinition {
    fn from(value: key::Key) -> Self {
        AttributeDefinition::new(value.name, value.kind)
    }
}

fn generate_query_expressions(
    ts: &app::TableSchema,
    pval: &str,
    sort_key_expression: &Option<String>,
    index: &Option<String>,
    strict: bool,
) -> Result<GeneratedQueryParams, DyneinQueryParamsError> {
    let expression: String = String::from("#DYNEIN_PKNAME = :DYNEIN_PKVAL");
    let mut names = HashMap::<String, String>::new();
    let mut vals = HashMap::<String, AttributeValue>::new();
    let mut sort_key_of_target_table_or_index: Option<key::Key> = None;

    match index {
        None =>
        /* Query for base table */
        {
            debug!("Assigning PK name/value and sort key (if any)");
            names.insert("#DYNEIN_PKNAME".to_owned(), ts.pk.name.to_owned());
            vals.insert(
                ":DYNEIN_PKVAL".to_owned(),
                build_attrval_scalar(&ts.pk.kind.to_string(), pval),
            );
            sort_key_of_target_table_or_index = ts.sk.clone();
        }
        Some(idx) =>
        /* Query for Secondary Index */
        {
            debug!("Specified Query target index name: {:?}", &idx);
            if let Some(table_indexes) = &ts.indexes {
                debug!("indexes attached to the table: {:?}", &table_indexes);
                for existing_idx in table_indexes {
                    // index name should be unique in a table. Even LSI and GSI don't have the same name.
                    if idx == &existing_idx.name {
                        names.insert(
                            String::from("#DYNEIN_PKNAME"),
                            String::from(&existing_idx.pk.name),
                        );
                        vals.insert(
                            String::from(":DYNEIN_PKVAL"),
                            build_attrval_scalar(&existing_idx.pk.kind.to_string(), pval),
                        );
                        sort_key_of_target_table_or_index = existing_idx.sk.clone();
                        break;
                    }
                }
            };

            // Exit with error if no effective secondary index found. Here "names" can be blank if:
            //   (1). no index is defined for the table, or
            //   (2). there're some index(es) but couldn't find specified name index
            if names.is_empty() {
                return Err(DyneinQueryParamsError::NoSuchIndex(
                    idx.to_string(),
                    ts.name.clone(),
                ));
            }
        }
    }

    debug!(
        "Before appending sort key expression ... exp='{}', names='{:?}', vals={:?}",
        &expression, &names, &vals
    );
    match sort_key_expression {
        None =>
        /* No --sort-key option given. proceed with partition key condition only. */
        {
            Ok(GeneratedQueryParams {
                exp: Some(expression),
                names: if names.is_empty() { None } else { Some(names) },
                vals: Some(vals),
            })
        }
        Some(ske) =>
        /* As --sort-key option is given, parse it and append the built SK related condition to required PK expression. */
        {
            append_sort_key_expression(
                sort_key_of_target_table_or_index,
                &expression,
                ske,
                names,
                vals,
                strict,
            )
        }
    }
}

impl From<key::KeyType> for AttributeType {
    fn from(value: key::KeyType) -> Self {
        match value {
            key::KeyType::S => AttributeType::S,
            key::KeyType::N => AttributeType::N,
            key::KeyType::B => AttributeType::B,
        }
    }
}

/// Using existing key condition expr (e.g. "myId <= :idVal") and supplementary mappings (expression_attribute_names, expression_attribute_values),
/// this method returns GeneratedQueryParams struct. Note that it's called only when sort key expression (ske) exists.
fn append_sort_key_expression(
    sort_key: Option<key::Key>,
    partition_key_expression: &str,
    sort_key_expression: &str,
    mut names: HashMap<String, String>,
    mut vals: HashMap<String, AttributeValue>,
    strict: bool,
) -> Result<GeneratedQueryParams, DyneinQueryParamsError> {
    // Check if the target table/index key schema has sort key. If there's no sort key definition, return with Err immediately.
    let (sk_name, sk_type) = match sort_key {
        Some(sk) => (sk.name.clone(), sk.kind),
        None => return Err(DyneinQueryParamsError::NoSortKeyDefined),
    };

    // Start building KeyConditionExpression. dynein automatically set placeholders, so currently it would be:
    //   "#DYNEIN_PKNAME = :DYNEIN_PKVAL AND "
    let mut built = format!("{} AND ", partition_key_expression);
    debug!(
        "Start building KeyConditionExpression. Currently built: '{}'",
        &built
    );

    let mut parser = DyneinParser::new();
    let result = if strict {
        parser.parse_sort_key_with_suggest(
            sort_key_expression,
            &AttributeDefinition::new(sk_name, sk_type),
        )
    } else {
        parser.parse_sort_key_with_fallback(
            sort_key_expression,
            &AttributeDefinition::new(sk_name, sk_type),
        )
    }
    .map_err(DyneinQueryParamsError::InvalidSortKeyOption)?;

    built.push_str(&result.get_expression());
    names.extend(result.get_names());
    vals.extend(result.get_values());

    debug!(
        "Finished to build KeyConditionExpression. Currently built: '{}'",
        &built
    );

    Ok(GeneratedQueryParams {
        exp: Some(built),
        names: if names.is_empty() { None } else { Some(names) },
        vals: Some(vals),
    })
}

/// Display items as a readable table format:
///   $ dy scan --output table
///   userName    registeredAt
///   thash       1582050565
///   tayoyo      1582000111
///   osaka       1583020931
fn display_items_table(
    items: Vec<HashMap<String, AttributeValue>>,
    ts: &app::TableSchema,
    selected_attributes: &Option<String>,
    keys_only: bool,
) {
    // Print no item message and return if items length is 0.
    if items.is_empty() {
        println!("No item to show in the table '{}'", ts.name);
        return;
    };

    // build header - first, primary key(s). Even index, key(s) are always projected.
    // ref: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/GSI.html#GSI.Projections
    let mut header: Vec<&str> = vec![ts.pk.name.as_str()];
    if let Some(sk) = &ts.sk {
        header.push(sk.name.as_str())
    };

    // build header - next, attribute names or aggregated "attributes" header, unless --keys-only flag is set.
    if !keys_only {
        if let Some(attrs) = selected_attributes {
            header.extend(attrs.split(',').collect::<Vec<&str>>());
        } else {
            header.push("attributes")
        };
    };
    debug!("built header elements: {:?}", header);

    let mut tw = TabWriter::new(io::stdout());
    tw.write_all((header.join("\t") + "\n").as_bytes()).unwrap();

    // `cells` is sth like: ["item1-pk\titem1-attr1\titem1-attr2", "item2-pk\titem2-attr1\titem2-attr2"]
    let mut cells: Vec<String> = vec![]; // may be able to use with_capacity to initialize the vec.
    for mut item in items {
        let mut item_attributes = vec![];
        // First, take primary key(s) of each item.
        let x: Option<AttributeValue> = item.remove(&ts.pk.name);
        if let Some(sk) = &ts.sk {
            let y: Option<AttributeValue> = item.remove(&sk.name);
            item_attributes.extend(vec![attrval_to_cell_print(x), attrval_to_cell_print(y)]);
        } else {
            item_attributes.extend(vec![attrval_to_cell_print(x)]);
        };

        if !item.is_empty() {
            if let Some(_attributes) = selected_attributes {
                let attrs: Vec<&str> = _attributes.split(',').map(|x| x.trim()).collect();
                for attr in attrs {
                    let attrval: Option<AttributeValue> = item.get(attr).cloned();
                    item_attributes.push(attrval_to_cell_print(attrval));
                }
            } else if !keys_only {
                // print rest aggreated "attributes" column in JSON format.
                let full = serde_json::to_string(&convert_to_json(&item)).unwrap();
                let threshold: usize = 50;
                if full.chars().count() > threshold {
                    // NOTE: counting bytes slice doesn't work for multi-bytes strings
                    let st: &String = &full.chars().take(threshold).collect();
                    item_attributes.push(String::from(st) + "...");
                } else {
                    item_attributes.push(full);
                }
            }
        }
        cells.push(item_attributes.join("\t"));
    }

    tw.write_all((cells.join("\n") + "\n").as_bytes()).unwrap();
    tw.flush().unwrap();
}

/// This function takes Option<AttributeValue> and return string,
/// so that it can be shown in a "cell" of table format, which has only single-line, small area.
fn attrval_to_cell_print(optional_attrval: Option<AttributeValue>) -> String {
    match optional_attrval {
        None => String::from(""),
        Some(attrval) => {
            match attrval {
                AttributeValue::S(v) => v,
                AttributeValue::N(v) => v,
                AttributeValue::Bool(v) => v.to_string(),
                AttributeValue::Ss(v) => serde_json::to_string(&v).unwrap(),
                AttributeValue::Ns(v) => serde_json::to_string(
                    &v.iter()
                        .map(|v| str_to_json_num(v))
                        .collect::<Vec<JsonValue>>(),
                )
                .unwrap(),
                AttributeValue::Null(_) => String::from("null"),
                _ => String::from("(snip)"), // B, BS, L, and M are not shown.
            }
        }
    }
}

/// https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/HowItWorks.NamingRulesDataTypes.html
pub fn attrval_to_type(attrval: &AttributeValue) -> Option<String> {
    match attrval {
        AttributeValue::S(_) => Some(String::from("String")),
        AttributeValue::N(_) => Some(String::from("Number")),
        AttributeValue::B(_) => Some(String::from("Binary")),
        AttributeValue::Bool(_) => Some(String::from("Boolian")),
        AttributeValue::Null(_) => Some(String::from("Null")),
        AttributeValue::Ss(_) => Some(String::from("Set (String)")),
        AttributeValue::Ns(_) => Some(String::from("Set (Number)")),
        AttributeValue::Bs(_) => Some(String::from("Set (Binary)")),
        AttributeValue::M(_) => Some(String::from("Map")),
        AttributeValue::L(_) => Some(String::from("List")),
        _ => None,
    }
}

/// This function takes items and returns values in multiple lines - one line for one item.
pub fn convert_items_to_csv_lines(
    items: &[HashMap<String, AttributeValue>],
    ts: &app::TableSchema,
    attributes_to_append: &Option<Vec<String>>,
    keys_only: bool,
) -> String {
    items
        .iter()
        .map(|item| convert_item_to_csv_line(item, ts, attributes_to_append, keys_only))
        .collect::<Vec<String>>()
        .join("\n")
}

/// This function convert from a DynamoDB item: { "abc": "val", "def": 123 }
/// into comma separated line: "val",123
fn convert_item_to_csv_line(
    item: &HashMap<String, AttributeValue>,
    ts: &app::TableSchema,
    attributes_to_append: &Option<Vec<String>>,
    keys_only: bool,
) -> String {
    let mut line = String::new();

    // push pk value to the line
    let pk_attrval: &AttributeValue = item
        .iter()
        .find(|x| x.0 == &ts.pk.name)
        .expect("pk should exist")
        .1;
    // NOTE: Another possible implementation to generate string from attrval would be: `&attrval_to_cell_print(Some(pk_attrval.to_owned())))`.
    //       However, `attrval_to_cell_print` doesn't surround String value with double-quotes (""), so I prefer using attrval_to_jsonval here.
    line.push_str(&attrval_to_jsonval(pk_attrval).to_string());

    // push sk value to the line, if needed.
    if let Some(sk) = &ts.sk {
        let sk_attrval: &AttributeValue = item
            .iter()
            .find(|x| x.0 == &sk.name)
            .expect("sk should exist in an item")
            .1;
        line.push(',');
        line.push_str(&attrval_to_jsonval(sk_attrval).to_string());
    }

    if keys_only {
    } else if let Some(attrs) = attributes_to_append {
        for attr /* String */ in attrs {
            let attrval: &AttributeValue = item.iter().find(|x| x.0 == attr).expect("Specified attribute not found in the item.").1;
            line.push(',');
            // NOTE: If special handling for complex data type is needed: `if let Some(_) = attrval.m {...`
            line.push_str(&attrval_to_jsonval(attrval).to_string());
        }
    }

    line
}

pub fn convert_to_json_vec(
    items: &[HashMap<String, AttributeValue>],
) -> Vec<HashMap<String, serde_json::Value>> {
    items.iter().map(convert_to_json).collect()
}

pub fn convert_to_json(
    item: &HashMap<String, AttributeValue>,
) -> HashMap<String, serde_json::Value> {
    item.iter()
        .map(|attr| (attr.0.to_string(), attrval_to_jsonval(attr.1)))
        .collect()
}

fn str_to_json_num(s: &str) -> JsonValue {
    match s.parse::<u64>() {
        Ok(i) => serde_json::to_value(i).unwrap(),
        Err(_) => match s.parse::<f64>() {
            Ok(f) => serde_json::to_value(f).unwrap(),
            Err(e) => panic!(
                "Failed to parse DynamoDB 'N' typed value: {:#?}\n{:#?}",
                s, e
            ),
        },
    }
}

fn attrval_to_jsonval(attrval: &AttributeValue) -> JsonValue {
    let unsupported: &str = "<<<JSON output doesn't support this type attributes>>>";
    //  following list of if-else statements would be return value of this function.
    match attrval {
        AttributeValue::S(v) => serde_json::to_value(v).unwrap(),
        AttributeValue::N(v) => str_to_json_num(v),
        AttributeValue::Bool(v) => serde_json::to_value(v).unwrap(),
        AttributeValue::Null(_) => serde_json::to_value(()).unwrap(),
        AttributeValue::Ss(v) => serde_json::to_value(v).unwrap(),
        AttributeValue::Ns(v) => v.iter().map(|v| str_to_json_num(v)).collect(),
        AttributeValue::B(_) | AttributeValue::Bs(_) => serde_json::to_value(unsupported).unwrap(),
        AttributeValue::M(v) => attrval_to_json_map(v),
        AttributeValue::L(v) => v.iter().map(attrval_to_jsonval).collect(),
        _ => panic!(
            "DynamoDB AttributeValue is not in valid status: {:#?}",
            &attrval
        ),
    }
}

/// inverse of `build_attrval_map`
fn attrval_to_json_map(attrval_map: &HashMap<String, AttributeValue>) -> JsonValue {
    let mut result = HashMap::<String, JsonValue>::new();
    for (k, v) in attrval_map {
        debug!("working on key '{}', and value '{:?}'", k, v);
        result.insert(k.to_string(), attrval_to_jsonval(v));
    }
    serde_json::to_value(result).unwrap()
}

/// Generate `ProjectionExpression` expression string and supplementary ExpressionAttributeNames.
/// If attributes = None and keys_only is false, returns GeneratedScanParams with Nones and Scan behaves as default.
/// If you set keys_only to true, the expression contains only primary key(s).
/// If you specify attributes to show, they'd be added to primary key(s). dynein's scan assumes always shows primary key(s).
fn generate_scan_expressions(
    ts: &app::TableSchema,
    attributes: &Option<String>,
    keys_only: bool,
) -> GeneratedScanParams {
    // Early return for the default condition. no --keys-only, no --attributes.
    if !keys_only && attributes.is_none() {
        return GeneratedScanParams {
            exp: None,
            names: None,
        };
    }

    // dynein always shows primary key(s) i.e. pk and sk (if any).
    let mut names = HashMap::<String, String>::new();
    names.insert(String::from("#DYNEIN_PKNAME"), ts.pk.name.clone());
    let mut returning_attributes: Vec<String> = vec![String::from("#DYNEIN_PKNAME")];
    if let Some(sk) = &ts.sk {
        returning_attributes.push(String::from("#DYNEIN_SKNAME"));
        names.insert(String::from("#DYNEIN_SKNAME"), sk.name.clone());
    };

    // if keys_only flag is true, no more attribute would be added.
    if keys_only {
    } else if let Some(_attributes) = attributes {
        let mut i: usize = 0;
        let attrs: Vec<&str> = _attributes.split(',').map(|x| x.trim()).collect();
        for attr in attrs {
            // skip if attributes contain primary key(s) as they're already included in the expression.
            if attr == ts.pk.name
                || (ts.sk.is_some() && attr == ts.sk.as_ref().unwrap().name.clone())
            {
                continue;
            }

            let placeholder = String::from("#DYNEIN_ATTRNAME") + &i.to_string();
            returning_attributes.push(placeholder.clone());
            names.insert(placeholder, String::from(attr));
            i += 1;
        }
    };

    let expression: String = returning_attributes.join(",");
    debug!("generated ProjectionExpression: {}", &expression);
    debug!("generated ExpressionAttributeNames: {:?}", &names);

    GeneratedScanParams {
        exp: Some(expression),
        names: Some(names),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::HashMap;

    #[test]
    fn test_generate_update_expressions_set_int() {
        let actual = generate_update_expressions(UpdateActionType::Set, "Price = 123");
        assert_eq!(
            actual.exp,
            Some("SET #DYNEIN_ATTRNAME0=:DYNEIN_ATTRVAL0".to_owned())
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([(
                "#DYNEIN_ATTRNAME0".to_owned(),
                "Price".to_owned(),
            )]))
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([(
                ":DYNEIN_ATTRVAL0".to_owned(),
                AttributeValue::N("123".to_owned()),
            )]))
        );
    }

    #[test]
    fn test_generate_update_expressions_set_int_str() {
        let actual =
            generate_update_expressions(UpdateActionType::Set, "Replies = 0, Status = \"OPEN\"");
        assert_eq!(
            actual.exp,
            Some(
                "SET #DYNEIN_ATTRNAME0=:DYNEIN_ATTRVAL0,#DYNEIN_ATTRNAME1=:DYNEIN_ATTRVAL1"
                    .to_owned()
            )
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([
                ("#DYNEIN_ATTRNAME0".to_owned(), "Replies".to_owned()),
                ("#DYNEIN_ATTRNAME1".to_owned(), "Status".to_owned()),
            ])),
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([
                (
                    ":DYNEIN_ATTRVAL0".to_owned(),
                    AttributeValue::N("0".to_owned()),
                ),
                (
                    ":DYNEIN_ATTRVAL1".to_owned(),
                    AttributeValue::S("OPEN".to_owned()),
                ),
            ])),
        );
    }

    #[test]
    fn test_generate_update_expressions_set_str() {
        let actual = generate_update_expressions(UpdateActionType::Set, "class = \"Math\"");
        assert_eq!(
            actual.exp,
            Some("SET #DYNEIN_ATTRNAME0=:DYNEIN_ATTRVAL0".to_owned())
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([(
                "#DYNEIN_ATTRNAME0".to_owned(),
                "class".to_owned(),
            )]))
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([(
                ":DYNEIN_ATTRVAL0".to_owned(),
                AttributeValue::S("Math".to_owned()),
            )])),
        );
    }

    #[test]
    fn test_generate_update_expressions_set_plus() {
        let actual = generate_update_expressions(UpdateActionType::Set, "Price = Price + 1");
        assert_eq!(
            actual.exp,
            Some("SET #DYNEIN_ATTRNAME0=#DYNEIN_ATTRNAME0+:DYNEIN_ATTRVAL0".to_owned())
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([(
                "#DYNEIN_ATTRNAME0".to_owned(),
                "Price".to_owned(),
            )])),
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([(
                ":DYNEIN_ATTRVAL0".to_owned(),
                AttributeValue::N("1".to_owned()),
            )])),
        );
    }

    #[test]
    fn test_generate_update_expressions_set_minus() {
        let actual = generate_update_expressions(UpdateActionType::Set, "Price = Price - 1");
        assert_eq!(
            actual.exp,
            Some("SET #DYNEIN_ATTRNAME0=#DYNEIN_ATTRNAME0-:DYNEIN_ATTRVAL0".to_owned())
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([(
                "#DYNEIN_ATTRNAME0".to_owned(),
                "Price".to_owned(),
            )])),
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([(
                ":DYNEIN_ATTRVAL0".to_owned(),
                AttributeValue::N("1".to_owned()),
            )])),
        );
    }

    #[test]
    fn test_generate_update_expressions_set_hyphen() {
        let actual = generate_update_expressions(
            UpdateActionType::Set,
            "LastPostedBy = \"2020-02-24T22:22:22Z\"",
        );
        assert_eq!(
            actual.exp,
            Some("SET #DYNEIN_ATTRNAME0=:DYNEIN_ATTRVAL0".to_owned())
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([(
                "#DYNEIN_ATTRNAME0".to_owned(),
                "LastPostedBy".to_owned(),
            )]))
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([(
                ":DYNEIN_ATTRVAL0".to_owned(),
                AttributeValue::S("2020-02-24T22:22:22Z".to_owned()),
            )]))
        );
    }

    #[test]
    fn test_generate_multi_update_expressions_include_hyphen() {
        let actual = generate_update_expressions(
            UpdateActionType::Set,
            "Replies = 0, LastPostedBy = \"2020-02-24T22:22:22Z\"",
        );
        assert_eq!(
            actual.exp,
            Some(
                "SET #DYNEIN_ATTRNAME0=:DYNEIN_ATTRVAL0,#DYNEIN_ATTRNAME1=:DYNEIN_ATTRVAL1"
                    .to_owned()
            )
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([
                ("#DYNEIN_ATTRNAME0".to_owned(), "Replies".to_owned()),
                ("#DYNEIN_ATTRNAME1".to_owned(), "LastPostedBy".to_owned()),
            ]))
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([
                (
                    ":DYNEIN_ATTRVAL0".to_owned(),
                    AttributeValue::N("0".to_owned())
                ),
                (
                    ":DYNEIN_ATTRVAL1".to_owned(),
                    AttributeValue::S("2020-02-24T22:22:22Z".to_owned())
                ),
            ]))
        );
    }

    #[test]
    fn test_generate_update_expressions_set_single_quote() {
        // To use single quote is not supported yet
        let actual = generate_update_expressions(UpdateActionType::Set, "key = 'value'");
        assert_eq!(
            actual.exp,
            Some("SET #DYNEIN_ATTRNAME0=:DYNEIN_ATTRVAL0".to_owned())
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([(
                "#DYNEIN_ATTRNAME0".to_owned(),
                "key".to_owned(),
            )]))
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([(
                ":DYNEIN_ATTRVAL0".to_owned(),
                AttributeValue::S("value".to_owned()),
            )]))
        );
    }

    // --set 'RelatedItems[1] = "item1"'
    #[test]
    fn test_generate_update_expressions_set_array_element() {
        let actual =
            generate_update_expressions(UpdateActionType::Set, "RelatedItems[1] = \"item1\"");
        assert_eq!(
            actual.exp,
            Some("SET #DYNEIN_ATTRNAME0[1]=:DYNEIN_ATTRVAL0".to_owned())
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([(
                "#DYNEIN_ATTRNAME0".to_owned(),
                "RelatedItems".to_owned()
            ),]))
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([(
                ":DYNEIN_ATTRVAL0".to_owned(),
                AttributeValue::S("item1".to_owned()),
            )]))
        );
    }

    // --set 'pr.5star[1] = 7, pr.3star = 3'
    #[test]
    fn test_generate_update_expressions_set_array_element_nested() {
        let actual =
            generate_update_expressions(UpdateActionType::Set, "pr.`5star`[1] = 7, pr.`3star` = 3");
        assert_eq!(
            actual.exp,
            Some("SET #DYNEIN_ATTRNAME0.#DYNEIN_ATTRNAME1[1]=:DYNEIN_ATTRVAL0,#DYNEIN_ATTRNAME0.#DYNEIN_ATTRNAME2=:DYNEIN_ATTRVAL1".to_owned())
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([
                ("#DYNEIN_ATTRNAME0".to_owned(), "pr".to_owned()),
                ("#DYNEIN_ATTRNAME1".to_owned(), "5star".to_owned()),
                ("#DYNEIN_ATTRNAME2".to_owned(), "3star".to_owned()),
            ]))
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([
                (
                    ":DYNEIN_ATTRVAL0".to_owned(),
                    AttributeValue::N("7".to_owned()),
                ),
                (
                    ":DYNEIN_ATTRVAL1".to_owned(),
                    AttributeValue::N("3".to_owned()),
                ),
            ]))
        )
    }

    // --set 'RelatedItems = list_append(RelatedItems, ["item2"])'
    #[test]
    fn test_generate_update_expressions_list_append() {
        let actual = generate_update_expressions(
            UpdateActionType::Set,
            "RelatedItems = list_append(RelatedItems, [\"item2\"])",
        );
        assert_eq!(
            actual.exp,
            Some(
                "SET #DYNEIN_ATTRNAME0=list_append(#DYNEIN_ATTRNAME0,:DYNEIN_ATTRVAL0)".to_owned()
            )
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([(
                "#DYNEIN_ATTRNAME0".to_owned(),
                "RelatedItems".to_owned()
            ),]))
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([(
                ":DYNEIN_ATTRVAL0".to_owned(),
                AttributeValue::L(vec![AttributeValue::S("item2".to_owned())]),
            )]))
        );
    }

    // --set 'RelatedItems = list_append(["item2"], RelatedItems)'
    #[test]
    fn test_generate_update_expressions_list_prepend() {
        let actual = generate_update_expressions(
            UpdateActionType::Set,
            "RelatedItems = list_append([\"item2\"], RelatedItems)",
        );
        assert_eq!(
            actual.exp,
            Some(
                "SET #DYNEIN_ATTRNAME0=list_append(:DYNEIN_ATTRVAL0,#DYNEIN_ATTRNAME0)".to_owned()
            )
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([(
                "#DYNEIN_ATTRNAME0".to_owned(),
                "RelatedItems".to_owned()
            ),]))
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([(
                ":DYNEIN_ATTRVAL0".to_owned(),
                AttributeValue::L(vec![AttributeValue::S("item2".to_owned())])
            )]))
        );
    }

    // --set 'Price = if_not_exists(Price, 123)'
    #[test]
    fn test_generate_update_expressions_if_not_exists() {
        let actual =
            generate_update_expressions(UpdateActionType::Set, "Price = if_not_exists(Price, 123)");
        assert_eq!(
            actual.exp,
            Some(
                "SET #DYNEIN_ATTRNAME0=if_not_exists(#DYNEIN_ATTRNAME0,:DYNEIN_ATTRVAL0)"
                    .to_owned()
            )
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([(
                "#DYNEIN_ATTRNAME0".to_owned(),
                "Price".to_owned()
            ),]))
        );
        assert_eq!(
            actual.vals,
            Some(HashMap::from([(
                ":DYNEIN_ATTRVAL0".to_owned(),
                AttributeValue::N("123".to_owned()),
            ),]))
        )
    }

    #[test]
    fn test_generate_update_expressions_remove() {
        let actual =
            generate_update_expressions(UpdateActionType::Remove, "Brand, InStock, QuantityOnHand");
        assert_eq!(
            actual.exp,
            Some("REMOVE #DYNEIN_ATTRNAME0,#DYNEIN_ATTRNAME1,#DYNEIN_ATTRNAME2".to_owned())
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([
                ("#DYNEIN_ATTRNAME0".to_owned(), "Brand".to_owned()),
                ("#DYNEIN_ATTRNAME1".to_owned(), "InStock".to_owned()),
                ("#DYNEIN_ATTRNAME2".to_owned(), "QuantityOnHand".to_owned()),
            ])),
        );
        assert_eq!(actual.vals, None);
    }

    // --remove "RelatedItems[1], RelatedItems[2]"
    #[test]
    fn test_generate_update_expressions_array_element() {
        let actual = generate_update_expressions(
            UpdateActionType::Remove,
            "RelatedItems[1], RelatedItems[2]",
        );
        assert_eq!(
            actual.exp,
            Some("REMOVE #DYNEIN_ATTRNAME0[1],#DYNEIN_ATTRNAME0[2]".to_owned())
        );
        assert_eq!(
            actual.names,
            Some(HashMap::from([(
                "#DYNEIN_ATTRNAME0".to_owned(),
                "RelatedItems".to_owned()
            )]))
        );
        assert_eq!(actual.vals, None);
    }

    #[test]
    fn test_dispatch_jsonvalue_to_attrval() {
        let string_list = r#"
        [
            "+44 1234567",
            "+44 2345678"
        ]"#;
        let string_list: Value = serde_json::from_str(string_list).unwrap();
        let actual = dispatch_jsonvalue_to_attrval(&string_list, false);
        assert_eq!(
            actual,
            AttributeValue::L(vec![
                AttributeValue::S("+44 1234567".to_owned()),
                AttributeValue::S("+44 2345678".to_owned()),
            ]),
        );
        let actual = dispatch_jsonvalue_to_attrval(&string_list, true);
        assert_eq!(
            actual,
            AttributeValue::Ss(vec!("+44 1234567".to_owned(), "+44 2345678".to_owned())),
        );

        let number_list = r#"
        [
            12345,
            67890
        ]"#;
        let number_list: Value = serde_json::from_str(number_list).unwrap();
        let actual = dispatch_jsonvalue_to_attrval(&number_list, false);
        assert_eq!(
            actual,
            AttributeValue::L(vec![
                AttributeValue::N("12345".to_owned()),
                AttributeValue::N("67890".to_owned()),
            ])
        );
        let actual = dispatch_jsonvalue_to_attrval(&number_list, true);
        assert_eq!(
            actual,
            AttributeValue::Ns(vec!["12345".to_owned(), "67890".to_owned()]),
        );

        let mix_list = r#"
        [
            "text",
            1234
        ]"#;
        let mix_list: Value = serde_json::from_str(mix_list).unwrap();
        for flag in [true, false] {
            let actual = dispatch_jsonvalue_to_attrval(&mix_list, flag);
            assert_eq!(
                actual,
                AttributeValue::L(vec![
                    AttributeValue::S("text".to_owned()),
                    AttributeValue::N("1234".to_owned()),
                ])
            );
        }
    }
}
