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
use aws_sdk_dynamodb::types::{AttributeDefinition, KeySchemaElement, TableDescription};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Key {
    pub name: String,
    /// Data type of the primary key. i.e. "S" (String), "N" (Number), or "B" (Binary).
    /// Use 'kind' as 'type' is a keyword in Rust.
    pub kind: KeyType,
}

impl Key {
    /// return String with "<pk name> (<pk type>)", e.g. "myPk (S)". Used in desc command outputs.
    pub fn display(&self) -> String {
        format!("{} ({})", self.name, self.kind)
    }
}

/// Restrict acceptable DynamoDB data types for primary keys.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum KeyType {
    S,
    N,
    B,
}

/// implement Display for KeyType to simply print a single letter "S", "N", or "B".
impl std::fmt::Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                KeyType::S => "S",
                KeyType::N => "N",
                KeyType::B => "B",
            }
        )
    }
}

#[derive(Debug, PartialEq)]
pub struct ParseKeyTypeError {
    message: String,
}

impl std::error::Error for ParseKeyTypeError {
    fn description(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for ParseKeyTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.message)
    }
}

impl ParseKeyTypeError {
    /// Parses a region given as a string literal into a type `KeyType'
    pub fn new(input: &str) -> Self {
        Self {
            message: format!("Not a valid DynamoDB primary key type: {}", input),
        }
    }
}

impl FromStr for KeyType {
    type Err = ParseKeyTypeError;

    fn from_str(s: &str) -> Result<Self, ParseKeyTypeError> {
        match s {
            "S" => Ok(Self::S),
            "N" => Ok(Self::N),
            "B" => Ok(Self::B),
            x => Err(ParseKeyTypeError::new(x)),
        }
    }
}

/// returns Option of a tuple (attribute_name, attribute_type (S/N/B)).
/// Used when you want to know "what is the Partition Key name and its data type of this table".
pub fn typed_key(pk_or_sk: &str, desc: &TableDescription) -> Option<Key> {
    // extracting key schema of "base table" here
    let ks = desc.key_schema.as_ref().unwrap();
    typed_key_for_schema(pk_or_sk, ks, desc.attribute_definitions.as_ref().unwrap())
}

/// Receives key data type (HASH or RANGE), KeySchemaElement(s), and AttributeDefinition(s),
/// In many cases it's called by typed_key, but when retrieving index schema, this method can be used directly so put it as public.
pub fn typed_key_for_schema(
    pk_or_sk: &str,
    ks: &[KeySchemaElement],
    attrs: &[AttributeDefinition],
) -> Option<Key> {
    // Fetch Partition Key ("HASH") or Sort Key ("RANGE") from given Key Schema. pk should always exists, but sk may not.
    let target_key = ks.iter().find(|x| x.key_type == pk_or_sk.into());
    target_key.map(|key| Key {
        name: key.attribute_name.to_owned(),
        // kind should be one of S/N/B, Which can be retrieved from AttributeDefinition's attribute_type.
        kind: KeyType::from_str(
            attrs
                .iter()
                .find(|at| at.attribute_name == key.attribute_name)
                .expect("primary key should be in AttributeDefinition.")
                .attribute_type
                .as_str(),
        )
        .unwrap(),
    })
}
