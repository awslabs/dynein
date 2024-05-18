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

use ::serde::{Deserialize, Serialize};
use aws_sdk_dynamodb::types::{
    AttributeDefinition, BillingMode, BillingModeSummary, GlobalSecondaryIndexDescription,
    KeySchemaElement, KeyType, LocalSecondaryIndexDescription, ProvisionedThroughputDescription,
    ScalarAttributeType, StreamSpecification, TableDescription,
};
use chrono::DateTime;
use log::error;
use rusoto_signature::Region;

use super::key;

/* =================================================
struct / enum / const
================================================= */

// TableDescription doesn't implement Serialize
// https://docs.rs/rusoto_dynamodb/0.42.0/rusoto_dynamodb/struct.TableDescription.html
#[derive(Serialize, Deserialize, Debug)]
struct PrintDescribeTable {
    name: String,
    region: String,
    status: String,
    schema: PrintPrimaryKeys,

    mode: Mode,
    capacity: Option<PrintCapacityUnits>,

    gsi: Option<Vec<PrintSecondaryIndex>>,
    lsi: Option<Vec<PrintSecondaryIndex>>,

    stream: Option<String>,

    count: i64,
    size_bytes: i64,
    created_at: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Mode {
    Provisioned,
    OnDemand,
}

impl Into<BillingMode> for Mode {
    fn into(self) -> BillingMode {
        match self {
            Mode::Provisioned => BillingMode::Provisioned,
            Mode::OnDemand => BillingMode::PayPerRequest,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct PrintPrimaryKeys {
    pk: String,
    sk: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PrintCapacityUnits {
    wcu: i64,
    rcu: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct PrintSecondaryIndex {
    name: String,
    schema: PrintPrimaryKeys,
    capacity: Option<PrintCapacityUnits>,
}

/// Receives region (just to show in one line for reference) and TableDescription,
/// print them in readable YAML format. NOTE: '~' representes 'null' or 'no value' in YAML syntax.
pub fn print_table_description(region: Region, desc: TableDescription) {
    let attr_defs = desc.clone().attribute_definitions.unwrap();
    let mode = extract_mode(&desc.billing_mode_summary);

    let print_table: PrintDescribeTable = PrintDescribeTable {
        name: String::from(&desc.clone().table_name.unwrap()),
        region: String::from(region.name()),
        status: String::from(desc.clone().table_status.unwrap().as_str()),
        schema: PrintPrimaryKeys {
            pk: key::typed_key("HASH", &desc)
                .expect("pk should exist")
                .display(),
            sk: key::typed_key("RANGE", &desc).map(|k| k.display()),
        },

        mode: mode.clone(),
        capacity: extract_capacity(&mode, &desc.provisioned_throughput),

        gsi: extract_secondary_indexes(&mode, &attr_defs, desc.global_secondary_indexes),
        lsi: extract_secondary_indexes(&mode, &attr_defs, desc.local_secondary_indexes),
        stream: extract_stream(desc.latest_stream_arn, desc.stream_specification),

        size_bytes: desc.table_size_bytes.unwrap(),
        count: desc.item_count.unwrap(),
        created_at: epoch_to_rfc3339(desc.creation_date_time.unwrap().as_secs_f64()),
    };
    println!("{}", serde_yaml::to_string(&print_table).unwrap());
}

/// Using Vec of String which is passed via command line,
/// generate KeySchemaElement(s) & AttributeDefinition(s), that are essential information to create DynamoDB tables or GSIs.
pub fn generate_essential_key_definitions(
    given_keys: &[String],
) -> (Vec<KeySchemaElement>, Vec<AttributeDefinition>) {
    let mut key_schema: Vec<KeySchemaElement> = vec![];
    let mut attribute_definitions: Vec<AttributeDefinition> = vec![];
    for (key_id, key_str) in given_keys.iter().enumerate() {
        let key_and_type = key_str.split(',').collect::<Vec<&str>>();
        if key_and_type.len() >= 3 {
            error!(
                "Invalid format for --keys option: '{}'. Valid format is '--keys myPk,S mySk,N'",
                &key_str
            );
            std::process::exit(1);
        }

        // assumes first given key is Partition key, and second given key is Sort key (if any).
        key_schema.push(
            KeySchemaElement::builder()
                .attribute_name(String::from(key_and_type[0]))
                .key_type(if key_id == 0 {
                    KeyType::Hash
                } else {
                    KeyType::Range
                })
                .build().unwrap(),
        );

        // If data type of key is omitted, dynein assumes it as String (S).
        attribute_definitions.push(
            AttributeDefinition::builder()
                .attribute_name(String::from(key_and_type[0]))
                .attribute_type(if key_and_type.len() == 2 {
                    ScalarAttributeType::from(key_and_type[1].to_uppercase().as_ref())
                } else {
                    ScalarAttributeType::S
                })
                .build().unwrap(),
        )
    }
    (key_schema, attribute_definitions)
}

/// Map "BilingModeSummary" field in table description returned from DynamoDB API,
/// into convenient mode name ("Provisioned" or "OnDemand")
pub fn extract_mode(bs: &Option<BillingModeSummary>) -> Mode {
    let provisioned_mode = Mode::Provisioned;
    let ondemand_mode = Mode::OnDemand;
    match bs {
        // if BillingModeSummary field doesn't exist, the table is Provisioned Mode.
        None => provisioned_mode,
        Some(x) => {
            if x.clone().billing_mode.unwrap() == BillingMode::PayPerRequest {
                ondemand_mode
            } else {
                provisioned_mode
            }
        }
    }
}

// FYI: https://grammarist.com/usage/indexes-indices/
fn extract_secondary_indexes<T: IndexDesc>(
    mode: &Mode,
    attr_defs: &[AttributeDefinition],
    option_indexes: Option<Vec<T>>,
) -> Option<Vec<PrintSecondaryIndex>> {
    if let Some(indexes) = option_indexes {
        let mut xs = Vec::<PrintSecondaryIndex>::new();
        for idx in &indexes {
            let ks = &idx.retrieve_key_schema().as_ref().unwrap();
            let idx = PrintSecondaryIndex {
                name: String::from(idx.retrieve_index_name().as_ref().unwrap()),
                schema: PrintPrimaryKeys {
                    pk: key::typed_key_for_schema("HASH", ks, attr_defs)
                        .expect("pk should exist")
                        .display(),
                    sk: key::typed_key_for_schema("RANGE", ks, attr_defs).map(|k| k.display()),
                },
                capacity: idx.extract_index_capacity(mode),
            };
            xs.push(idx);
        }
        Some(xs)
    } else {
        None
    }
}

fn extract_stream(arn: Option<String>, spec: Option<StreamSpecification>) -> Option<String> {
    if arn.is_none() {
        None
    } else {
        Some(format!(
            "{} ({})",
            arn.unwrap(),
            spec.unwrap().stream_view_type.unwrap()
        ))
    }
}

pub fn epoch_to_rfc3339(epoch: f64) -> String {
    let utc_datetime = DateTime::from_timestamp(epoch as i64, 0).unwrap();
    utc_datetime.to_rfc3339()
}

fn extract_capacity(
    mode: &Mode,
    cap_desc: &Option<ProvisionedThroughputDescription>,
) -> Option<PrintCapacityUnits> {
    if mode == &Mode::OnDemand {
        None
    } else {
        let desc = cap_desc.as_ref().unwrap();
        Some(PrintCapacityUnits {
            wcu: desc.write_capacity_units.unwrap(),
            rcu: desc.read_capacity_units.unwrap(),
        })
    }
}

trait IndexDesc {
    fn retrieve_index_name(&self) -> &Option<String>;
    fn retrieve_key_schema(&self) -> &Option<Vec<KeySchemaElement>>;
    fn extract_index_capacity(&self, m: &Mode) -> Option<PrintCapacityUnits>;
}

impl IndexDesc for GlobalSecondaryIndexDescription {
    fn retrieve_index_name(&self) -> &Option<String> {
        &self.index_name
    }
    fn retrieve_key_schema(&self) -> &Option<Vec<KeySchemaElement>> {
        &self.key_schema
    }
    fn extract_index_capacity(&self, m: &Mode) -> Option<PrintCapacityUnits> {
        if m == &Mode::OnDemand {
            None
        } else {
            extract_capacity(m, &self.provisioned_throughput)
        }
    }
}

impl IndexDesc for LocalSecondaryIndexDescription {
    fn retrieve_index_name(&self) -> &Option<String> {
        &self.index_name
    }
    fn retrieve_key_schema(&self) -> &Option<Vec<KeySchemaElement>> {
        &self.key_schema
    }
    fn extract_index_capacity(&self, _: &Mode) -> Option<PrintCapacityUnits> {
        None // Unlike GSI, LSI doesn't have it's own capacity.
    }
}
