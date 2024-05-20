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

use crate::pest::Parser;
use aws_sdk_dynamodb::{primitives::Blob, types::AttributeValue};
use base64::engine::{general_purpose, DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::{DecodeError, Engine};
use bytes::Bytes;
use itertools::Itertools;
use pest::iterators::Pair;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Write};
use std::iter::Enumerate;
use std::str::Chars;
use std::sync::OnceLock;

#[derive(Parser)]
#[grammar = "expression.pest"]
struct GeneratedParser;

type SetAction = Vec<AtomicSet>;
type RemoveAction = Vec<AtomicRemove>;

pub struct AttributeDefinition {
    attribute_name: String,
    attribute_type: AttributeType,
}

impl AttributeDefinition {
    pub fn new(
        attribute_name: impl Into<String>,
        key_type: impl Into<AttributeType>,
    ) -> AttributeDefinition {
        AttributeDefinition {
            attribute_name: attribute_name.into(),
            attribute_type: key_type.into(),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum AttributeType {
    S,
    N,
    B,
    Bool,
    Null,
    L,
    M,
    NS,
    SS,
    BS,
}

impl Display for AttributeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeType::S => {
                write!(f, "string (S)")
            }
            AttributeType::N => {
                write!(f, "number (N)")
            }
            AttributeType::B => {
                write!(f, "binary (B)")
            }
            AttributeType::Bool => {
                write!(f, "boolean (BOOL)")
            }
            AttributeType::Null => {
                write!(f, "null (NULL)")
            }
            AttributeType::L => {
                write!(f, "list (L)")
            }
            AttributeType::M => {
                write!(f, "map (M)")
            }
            AttributeType::NS => {
                write!(f, "nummber set (NS)")
            }
            AttributeType::SS => {
                write!(f, "string set (SS)")
            }
            AttributeType::BS => {
                write!(f, "binary set (BS)")
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum SortKeyCondition {
    Eq(AttrVal),
    // =, ==
    Le(AttrVal),
    // <=
    Lt(AttrVal),
    // <
    Ge(AttrVal),
    // >=
    Gt(AttrVal),
    // >
    Between(AttrVal, AttrVal),
    // between A and B, between A B
    BeginsWith(AttrVal), // begins_with
}

impl Display for SortKeyCondition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SortKeyCondition::Eq(val) => {
                write!(f, "= {}", val)?;
            }
            SortKeyCondition::Le(val) => {
                write!(f, "<= {}", val)?;
            }
            SortKeyCondition::Lt(val) => {
                write!(f, "< {}", val)?;
            }
            SortKeyCondition::Ge(val) => {
                write!(f, ">= {}", val)?;
            }
            SortKeyCondition::Gt(val) => {
                write!(f, "> {}", val)?;
            }
            SortKeyCondition::Between(begin, end) => {
                write!(f, "between {} and {}", begin, end)?;
            }
            SortKeyCondition::BeginsWith(prefix) => {
                write!(f, "begins_with {}", prefix)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct AtomicSet {
    path: Path,
    value: Value,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct AtomicRemove {
    path: Path,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct Path {
    elements: Vec<PathElement>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum PathElement {
    Attribute(String),
    Index(String),
}

impl Path {
    fn new() -> Path {
        Path {
            elements: Vec::new(),
        }
    }

    fn add_attr(&mut self, attr: String) {
        self.elements.push(PathElement::Attribute(attr));
    }

    fn add_index(&mut self, idx: String) {
        self.elements.push(PathElement::Index(idx));
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Value {
    PlusExpression(Operand, Operand),
    MinusExpression(Operand, Operand),
    Operand(Operand),
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Operand {
    Function(Function),
    Literal(AttrVal),
    Path(Path),
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum Function {
    ListAppendFunction(ListAppendParameter, ListAppendParameter),
    IfNotExistsFunction(Path, Box<Value>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum ListAppendParameter {
    Path(Path),
    ListLiteral(AttrVal),
}

/// The result of parsing expression
#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionResult {
    exp: String,
    names: HashMap<String, String>,
    values: HashMap<String, AttributeValue>,
}

impl ExpressionResult {
    pub fn get_expression(&self) -> String {
        self.exp.clone()
    }

    pub fn get_names(&self) -> HashMap<String, String> {
        self.names.clone()
    }

    pub fn get_values(&self) -> HashMap<String, AttributeValue> {
        self.values.clone()
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ParsingErrorWithSuggestError {
    pub parse_error: Box<pest::error::Error<Rule>>,
    pub suggest: String,
}

impl Display for ParsingErrorWithSuggestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n\nFailed to parse as the strict format. Did you intend '{}'?",
            *self.parse_error, self.suggest
        )
    }
}

/// The error context of an unexpected end of a string
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct EscapeCharUnexpectedEndOfSequenceError {
    pub handling_target: String,
    pub escape_pos: usize,
}

impl Display for EscapeCharUnexpectedEndOfSequenceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Unexpected end of escape sequences at {} for the string '{}'",
            self.escape_pos, self.handling_target
        )
    }
}

/// The error context of an invalid unicode character
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct InvalidUnicodeCharError {
    pub handling_target: String,
    pub escape_pos: usize,
}

impl Display for InvalidUnicodeCharError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid unicode character at {} for the string '{}'",
            self.escape_pos, self.handling_target
        )
    }
}

/// The error context of an unexpected character a string
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct EscapeCharError {
    pub handling_target: String,
    pub invalid_char: char,
    pub escape_pos: usize,
}

impl Display for EscapeCharError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Unexpected escaped character at handling char '{}' at {} for the string '{}'",
            self.invalid_char, self.escape_pos, self.handling_target
        )
    }
}

/// The error context of an unexpected escape byte
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct EscapeByteError {
    pub handling_target: String,
    pub escape_byte: u8,
    pub escape_pos: usize,
}

impl Display for EscapeByteError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Unexpected escaped byte {}({:x}) at {} parsing '{}'",
            char::from(self.escape_byte),
            self.escape_byte,
            self.escape_pos,
            self.handling_target
        )
    }
}

/// The error context of invalid type
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct InvalidTypesError {
    pub expected_type: AttributeType,
    pub actual_type: AttributeType,
}

impl Display for InvalidTypesError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid type detected. Expected type is {}, but actual type is {}.",
            self.expected_type, self.actual_type
        )
    }
}

/// The error context of invalid type
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct InvalidTypesWithSuggestError {
    pub expected_type: AttributeType,
    pub actual_type: AttributeType,
    pub suggest: String,
}

impl Display for InvalidTypesWithSuggestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid type detected. Expected type is {}, but actual type is {}.\nDid you intend '{}'?",
            self.expected_type, self.actual_type, self.suggest
        )
    }
}
/// The error context of a parsing error
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ParseError {
    ParsingError(Box<pest::error::Error<Rule>>),
    ParsingErrorWithSuggest(ParsingErrorWithSuggestError),
    UnexpectedEndOfSequence(EscapeCharUnexpectedEndOfSequenceError),
    InvalidUnicodeChar(InvalidUnicodeCharError),
    InvalidEscapeChar(EscapeCharError),
    InvalidEscapeByte(EscapeByteError),
    InvalidBeginsWith(String),
    InvalidTypes(InvalidTypesError),
    InvalidTypesWithSuggest(InvalidTypesWithSuggestError),
    Base64DecodeError(DecodeError),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::ParsingError(err) => {
                write!(f, "{}", err)
            }
            ParseError::ParsingErrorWithSuggest(err) => {
                write!(f, "{}", err)
            }
            ParseError::UnexpectedEndOfSequence(err) => {
                write!(f, "{}", err)
            }
            ParseError::InvalidUnicodeChar(err) => {
                write!(f, "{}", err)
            }
            ParseError::InvalidEscapeChar(err) => {
                write!(f, "{}", err)
            }
            ParseError::InvalidEscapeByte(err) => {
                write!(f, "{}", err)
            }
            ParseError::InvalidBeginsWith(input) => {
                write!(f, "the argument of begins_with is invalid: '{}'", input)
            }
            ParseError::InvalidTypes(err) => {
                write!(f, "{}", err)
            }
            ParseError::InvalidTypesWithSuggest(err) => {
                write!(f, "{}", err)
            }
            ParseError::Base64DecodeError(err) => {
                write!(f, "failed to decode base64 string: {}", err)
            }
        }
    }
}

impl From<DecodeError> for ParseError {
    fn from(err: DecodeError) -> Self {
        ParseError::Base64DecodeError(err)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum AttrVal {
    N(String),
    S(String),
    Bool(bool),
    Null(bool),
    B(Bytes),
    L(Vec<AttrVal>),
    M(HashMap<String, AttrVal>),
    NS(Vec<String>),
    SS(Vec<String>),
    BS(Vec<Bytes>),
}

impl AttrVal {
    fn is_type(&self, t: AttributeType) -> bool {
        self.attribute_type() == t
    }

    fn attribute_type(&self) -> AttributeType {
        match self {
            AttrVal::N(_) => AttributeType::N,
            AttrVal::S(_) => AttributeType::S,
            AttrVal::B(_) => AttributeType::B,
            AttrVal::Bool(_) => AttributeType::Bool,
            AttrVal::Null(_) => AttributeType::Null,
            AttrVal::L(_) => AttributeType::L,
            AttrVal::M(_) => AttributeType::M,
            AttrVal::NS(_) => AttributeType::NS,
            AttrVal::SS(_) => AttributeType::SS,
            AttrVal::BS(_) => AttributeType::BS,
        }
    }

    fn convert_attribute_value(self) -> AttributeValue {
        match self {
            AttrVal::N(number) => AttributeValue::N(number),
            AttrVal::S(str) => AttributeValue::S(str),
            AttrVal::Bool(boolean) => AttributeValue::Bool(boolean),
            AttrVal::Null(isnull) => AttributeValue::Null(isnull),
            AttrVal::B(binary) => AttributeValue::B(Blob::new(binary)),
            AttrVal::L(list) => AttributeValue::L(
                list.into_iter()
                    .map(|x| x.convert_attribute_value())
                    .collect(),
            ),
            AttrVal::M(map) => AttributeValue::M(
                map.into_iter()
                    .map(|(key, val)| (key, val.convert_attribute_value()))
                    .collect(),
            ),
            AttrVal::NS(list) => AttributeValue::Ns(list),
            AttrVal::SS(list) => AttributeValue::Ss(list),
            AttrVal::BS(list) => AttributeValue::Bs(list.into_iter().map(Blob::new).collect()),
        }
    }
}

fn format_array_elements<T: Display>(f: &mut Formatter<'_>, vals: &[T]) -> std::fmt::Result {
    if let Some((last, rest)) = vals.split_last() {
        for val in rest {
            write!(f, "{},", val)?;
        }
        write!(f, "{}", last)?;
    }
    Ok(())
}

impl Display for AttrVal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AttrVal::N(val) => {
                f.write_str(val)?;
            }
            AttrVal::S(val) => {
                write!(f, "\"{}\"", convert_to_escaped_json_string(val))?;
            }
            AttrVal::Bool(val) => match val {
                false => f.write_str("false")?,
                true => f.write_str("true")?,
            },
            AttrVal::Null(_) => {
                f.write_str("null")?;
            }
            AttrVal::B(val) => {
                write!(f, "b64\"{}\"", general_purpose::STANDARD.encode(val))?;
            }
            AttrVal::L(vals) => {
                f.write_char('[')?;
                format_array_elements(f, vals)?;
                f.write_char(']')?;
            }
            AttrVal::M(map) => {
                f.write_char('{')?;
                let mut first = true;
                for (k, v) in map.iter() {
                    if first {
                        first = false;
                    } else {
                        f.write_char(',')?;
                    }
                    write!(f, "\"{}\":{}", convert_to_escaped_json_string(k), v)?;
                }
                f.write_char('}')?;
            }
            AttrVal::NS(vals) => {
                assert_ne!(vals.len(), 0);
                f.write_str("<<")?;
                format_array_elements(f, vals)?;
                f.write_str(">>")?;
            }
            AttrVal::SS(vals) => {
                assert_ne!(vals.len(), 0);
                f.write_str("<<")?;
                if let Some((last, rest)) = vals.split_last() {
                    for val in rest {
                        write!(f, "\"{}\",", val)?;
                    }
                    write!(f, "\"{}\"", last)?;
                }
                f.write_str(">>")?;
            }
            AttrVal::BS(vals) => {
                assert_ne!(vals.len(), 0);
                f.write_str("<<")?;
                if let Some((last, rest)) = vals.split_last() {
                    for val in rest {
                        write!(f, "b64\"{}\",", general_purpose::STANDARD.encode(val))?;
                    }
                    write!(f, "b64\"{}\"", general_purpose::STANDARD.encode(last))?;
                }
                f.write_str(">>")?;
            }
        }
        Ok(())
    }
}

impl From<AttrVal> for AttributeValue {
    fn from(value: AttrVal) -> Self {
        value.convert_attribute_value()
    }
}

/// Parse internal of double quoted string.
///
/// It accepts escape characters as the following.
///
/// | Escape Sequence | Character Represented by Sequence |
/// |-----------------|-----------------------------------|
/// |       \0        | An ASCII NUL (X'00') character    |
/// |       \b        | A backspace character             |
/// |       \f        | A form feed character             |
/// |       \n        | A newline (linefeed) character    |
/// |       \r        | A carriage return character       |
/// |       \t        | A tab character                   |
/// |       \\\"      | A double quote (") character      |
/// |       \\\'      | A single quote (') character      |
/// |       \\\\      | A backslash (\\) character        |
/// |       \\/       | A slash (/) character             |
/// |     \\uXXXX     | An arbitrary unicode character    |
fn parse_internal_double_quote_string(str: &str) -> Result<String, ParseError> {
    let mut result = String::with_capacity(str.len());
    let mut iter = str.chars().enumerate();

    while let Some((pos, ch)) = iter.next() {
        if ch != '\\' {
            result.push(ch);
        } else {
            let escaping_pos = pos;
            let consume = |iter: &mut Enumerate<Chars>| -> Result<(usize, char), ParseError> {
                iter.next().ok_or_else(|| {
                    ParseError::UnexpectedEndOfSequence(EscapeCharUnexpectedEndOfSequenceError {
                        escape_pos: escaping_pos,
                        handling_target: str.to_owned(),
                    })
                })
            };
            let parse_u16 = |iter: &mut Enumerate<Chars>| -> Result<u16, ParseError> {
                let mut result = 0u16;
                for _ in 0..4 {
                    let (_pos, ch) = consume(iter)?;
                    if let Some(b) = ch.to_digit(16) {
                        result = (result << 4) + b as u16
                    } else {
                        return Err(ParseError::InvalidEscapeChar(EscapeCharError {
                            handling_target: str.to_owned(),
                            invalid_char: ch,
                            escape_pos: escaping_pos,
                        }));
                    }
                }
                Ok(result)
            };
            let (pos, ch) = consume(&mut iter)?;
            match ch {
                '0' => result.push('\0'),
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                'b' => result.push('\x08'),
                'f' => result.push('\x0c'),
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                'u' => {
                    let u1 = parse_u16(&mut iter)?;
                    if let Some(c) = char::from_u32(u1 as u32) {
                        // This char is in Basic Multilingual Plane.
                        result.push(c);
                    } else {
                        let (_, ch) = consume(&mut iter)?;
                        if ch != '\\' {
                            return Err(ParseError::InvalidUnicodeChar(InvalidUnicodeCharError {
                                handling_target: str.to_owned(),
                                escape_pos: pos,
                            }));
                        }
                        let (_, ch) = consume(&mut iter)?;
                        if ch != 'u' {
                            return Err(ParseError::InvalidUnicodeChar(InvalidUnicodeCharError {
                                handling_target: str.to_owned(),
                                escape_pos: pos,
                            }));
                        }
                        let u2 = parse_u16(&mut iter)?;
                        // All escape sequences have been processed. Try to decode as utf16.
                        // This `unwrap` is always safe.
                        result.push(char::decode_utf16([u1, u2]).next().unwrap().map_err(
                            |_| {
                                ParseError::InvalidUnicodeChar(InvalidUnicodeCharError {
                                    handling_target: str.to_owned(),
                                    escape_pos: pos,
                                })
                            },
                        )?);
                    }
                }
                _ => result.push(ch),
            }
        }
    }
    Ok(result)
}

/// Convert a provided string into escaped string which can be used in JSON string.
///
/// This function escapes the following characters:
///
/// * Unicode control characters
/// * a double quote character (\")
/// * a backslash character (\\)
/// * a backspace character (\b)
/// * a form feed character (\f)
/// * a newline (linefeed) character (\n)
/// * a carriage return character (\r)
/// * a tab character (\t)
fn convert_to_escaped_json_string(str: &str) -> String {
    let mut result = String::with_capacity(str.len());

    for ch in str.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\x08' => result.push_str("\\b"),
            '\x0C' => result.push_str("\\f"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => {
                if ch.is_control() {
                    // The following code logic is verbose for control characters because control
                    // characters are in the Basic Multilingual Plane. However, we would add
                    // categories other than control characters based on the user's feedback or
                    // further understanding of Unicode categories unsuitable for displaying
                    // characters. Therefore, we retain this logic to facilitate future improvements.
                    let mut b = [0; 2];
                    for c in ch.encode_utf16(&mut b) {
                        const HEX: &[u8; 16] = b"0123456789abcdef";
                        let b0 = ((*c >> 12) as u8 & 0xf) as usize;
                        let b1 = ((*c >> 8) as u8 & 0xf) as usize;
                        let b2 = ((*c >> 4) as u8 & 0xf) as usize;
                        let b3 = (*c as u8 & 0xf) as usize;
                        result.push_str("\\u");
                        unsafe {
                            // Following code is safe because HEX is created from valid string.
                            result.push(char::from_u32_unchecked(HEX[b0] as u32));
                            result.push(char::from_u32_unchecked(HEX[b1] as u32));
                            result.push(char::from_u32_unchecked(HEX[b2] as u32));
                            result.push(char::from_u32_unchecked(HEX[b3] as u32));
                        }
                    }
                } else {
                    result.push(ch);
                }
            }
        }
    }

    result
}

/// Parse double quoted string which accepts escape sequence.
fn parse_double_quote_literal(str: &str) -> Result<String, ParseError> {
    parse_internal_double_quote_string(&str[1..str.len() - 1])
}

/// Parse single quoted string as is.
fn parse_single_quote_literal(str: &str) -> String {
    str[1..str.len() - 1].to_owned()
}

/// Parse an internal of binary_literal.
///
/// We use the same semantics as rust byte literals, except that we accept multiple bytes.
/// See: https://doc.rust-lang.org/reference/tokens.html#byte-literals
fn parse_internal_binary_literal(str: &str) -> Result<Bytes, ParseError> {
    let mut result = Vec::with_capacity(str.len());
    enum State {
        Normal,
        StartEscape,
        ByteEscapeFirstChar,
        ByteEscapeSecondChar,
    }
    let mut state = State::Normal;
    let mut byte = 0u8;
    for (idx, ch) in str.bytes().enumerate() {
        match state {
            State::Normal => {
                if ch == b'\\' {
                    state = State::StartEscape;
                } else {
                    result.push(ch);
                }
            }
            State::StartEscape => match ch {
                b'n' => {
                    result.push(b'\n');
                    state = State::Normal;
                }
                b'r' => {
                    result.push(b'\r');
                    state = State::Normal;
                }
                b't' => {
                    result.push(b'\t');
                    state = State::Normal;
                }
                b'0' => {
                    result.push(b'\0');
                    state = State::Normal;
                }
                b'\\' | b'\'' | b'"' => {
                    result.push(ch);
                    state = State::Normal;
                }
                b'x' => state = State::ByteEscapeFirstChar,
                _ => {
                    return Err(ParseError::InvalidEscapeByte(EscapeByteError {
                        handling_target: str.to_owned(),
                        escape_byte: ch,
                        escape_pos: idx,
                    }));
                }
            },
            State::ByteEscapeFirstChar => {
                byte = hex_as_byte(str, idx, ch)?;
                state = State::ByteEscapeSecondChar;
            }
            State::ByteEscapeSecondChar => {
                let byte = byte << 4 | hex_as_byte(str, idx, ch)?;
                result.push(byte);
                state = State::Normal;
            }
        }
    }
    Ok(Bytes::from(result))
}

/// Parse a hex character as a byte.
///
/// `parsing_str` and `idx` are used to create an error.
fn hex_as_byte(parsing_str: &str, idx: usize, ch: u8) -> Result<u8, ParseError> {
    if ch.is_ascii_digit() {
        Ok(ch - b'0')
    } else if (b'A'..=b'F').contains(&ch) {
        Ok(ch - b'A' + 10)
    } else if (b'a'..=b'f').contains(&ch) {
        Ok(ch - b'a' + 10)
    } else {
        Err(ParseError::InvalidEscapeByte(EscapeByteError {
            handling_target: parsing_str.to_owned(),
            escape_byte: ch,
            escape_pos: idx,
        }))
    }
}

/// Parse binary literal.
fn parse_binary_literal(str: &str) -> Result<Bytes, ParseError> {
    parse_internal_binary_literal(&str[1..str.len() - 1])
}

/// Parse internal of binary_string.
///
/// We use same semantics as rust byte string literals.
/// See: https://doc.rust-lang.org/reference/tokens.html#byte-string-literals
fn parse_internal_binary_string(str: &str) -> Result<Bytes, ParseError> {
    let mut result = Vec::with_capacity(str.len());
    enum State {
        Normal,
        StartEscape,
        ByteEscapeFirstChar,
        ByteEscapeSecondChar,
        SkipSpaces,
    }
    let mut state = State::Normal;
    let mut byte = 0u8;
    for (idx, ch) in str.bytes().enumerate() {
        match state {
            State::Normal => {
                if ch == b'\\' {
                    state = State::StartEscape;
                } else {
                    result.push(ch);
                }
            }
            State::StartEscape => match ch {
                b'n' => {
                    result.push(b'\n');
                    state = State::Normal;
                }
                b'r' => {
                    result.push(b'\r');
                    state = State::Normal;
                }
                b't' => {
                    result.push(b'\t');
                    state = State::Normal;
                }
                b'0' => {
                    result.push(b'\0');
                    state = State::Normal;
                }
                b'\\' | b'\'' | b'"' => {
                    result.push(ch);
                    state = State::Normal;
                }
                b'\r' | b'\n' => {
                    state = State::SkipSpaces;
                }
                b'x' => state = State::ByteEscapeFirstChar,
                _ => {
                    return Err(ParseError::InvalidEscapeByte(EscapeByteError {
                        handling_target: str.to_owned(),
                        escape_byte: ch,
                        escape_pos: idx,
                    }));
                }
            },
            State::SkipSpaces => match ch {
                b' ' | b'\t' | b'\n' | b'\r' => {}
                b'\\' => {
                    state = State::StartEscape;
                }
                _ => {
                    state = State::Normal;
                    result.push(ch);
                }
            },
            State::ByteEscapeFirstChar => {
                byte = hex_as_byte(str, idx, ch)?;
                state = State::ByteEscapeSecondChar;
            }
            State::ByteEscapeSecondChar => {
                let byte = byte << 4 | hex_as_byte(str, idx, ch)?;
                result.push(byte);
                state = State::Normal;
            }
        }
    }
    Ok(Bytes::from(result))
}

/// Parse binary string literal.
fn parse_binary_string_literal(str: &str) -> Result<Bytes, ParseError> {
    parse_internal_binary_string(&str[1..str.len() - 1])
}

/// Parse b64 literal.
static B64_ENGINE: OnceLock<GeneralPurpose> = OnceLock::new();
fn parse_b64_literal(str: &str) -> Result<Bytes, ParseError> {
    let engine = B64_ENGINE.get_or_init(|| {
        GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
        )
    });
    Ok(engine.decode(&str[4..str.len() - 1]).map(Bytes::from)?)
}

fn parse_string_literal(pair: Pair<Rule>) -> Result<String, ParseError> {
    match pair.as_rule() {
        Rule::double_quote_literal => Ok(parse_double_quote_literal(pair.as_str())?),
        Rule::single_quote_literal => Ok(parse_single_quote_literal(pair.as_str())),
        _ => {
            // this must not happen
            unreachable!("Expect string literal, but another token found");
        }
    }
}

fn parse_list_literal(pair: Pair<Rule>) -> Result<Vec<AttrVal>, ParseError> {
    assert_eq!(pair.as_rule(), Rule::list_literal);
    pair.into_inner().map(parse_literal).collect()
}

/// Parse a literal part
fn parse_literal(pair: Pair<Rule>) -> Result<AttrVal, ParseError> {
    match pair.as_rule() {
        Rule::true_literal => Ok(AttrVal::Bool(true)),
        Rule::false_literal => Ok(AttrVal::Bool(false)),
        Rule::null_literal => Ok(AttrVal::Null(true)),
        Rule::double_quote_literal | Rule::single_quote_literal => {
            Ok(AttrVal::S(parse_string_literal(pair)?))
        }
        Rule::number_literal => Ok(AttrVal::N(pair.as_str().to_owned())),
        Rule::binary_literal => Ok(AttrVal::B(parse_binary_literal(pair.as_str())?)),
        Rule::binary_string_literal => Ok(AttrVal::B(parse_binary_string_literal(pair.as_str())?)),
        Rule::b64_literal => Ok(AttrVal::B(parse_b64_literal(pair.as_str())?)),
        Rule::list_literal => Ok(AttrVal::L(parse_list_literal(pair)?)),
        Rule::map_literal => {
            let map: Result<HashMap<_, _>, _> = pair
                .into_inner()
                .map(|p| {
                    assert_eq!(p.as_rule(), Rule::map_pair);
                    let it = p.into_inner();
                    if let Some((p_key, p_val)) = it.collect_tuple() {
                        assert_eq!(p_key.as_rule(), Rule::map_key);
                        assert_eq!(p_val.as_rule(), Rule::map_value);
                        // this unwrap is safe because map_key has always one string literal
                        let key = parse_string_literal(p_key.into_inner().next().unwrap());
                        key.and_then(|key| {
                            // this unwrap is safe because map_value has always one literal
                            let value = p_val.into_inner().next().unwrap();
                            parse_literal(value).map(|x| (key.to_string(), x))
                        })
                    } else {
                        // this must not happen
                        unreachable!("Unexpected non-paired map element")
                    }
                })
                .collect();
            Ok(AttrVal::M(map?))
        }
        Rule::string_set_literal => {
            let list: Result<Vec<_>, _> = pair
                .into_inner()
                .map(|p| {
                    match p.as_rule() {
                        Rule::double_quote_literal => parse_double_quote_literal(p.as_str()),
                        Rule::single_quote_literal => Ok(parse_single_quote_literal(p.as_str())),
                        _ => {
                            // this must not happen
                            unreachable!("Unexpected string set element")
                        }
                    }
                })
                .collect();
            Ok(AttrVal::SS(list?))
        }
        Rule::number_set_literal => {
            let list: Vec<_> = pair
                .into_inner()
                .map(|p| {
                    match p.as_rule() {
                        Rule::number_literal => p.as_str().to_string(),
                        _ => {
                            // this must not happen
                            unreachable!("Unexpected number set element")
                        }
                    }
                })
                .collect();
            Ok(AttrVal::NS(list))
        }
        Rule::binary_set_literal => {
            let list: Result<Vec<_>, _> = pair
                .into_inner()
                .map(|p| {
                    match p.as_rule() {
                        Rule::binary_literal => parse_binary_literal(p.as_str()),
                        Rule::binary_string_literal => parse_binary_string_literal(p.as_str()),
                        Rule::b64_literal => parse_b64_literal(p.as_str()),
                        _ => {
                            // this must not happen
                            unreachable!("Unexpected binary set element")
                        }
                    }
                })
                .collect();
            Ok(AttrVal::BS(list?))
        }
        _ => {
            // this must not happen
            unreachable!("Unexpected element on literal")
        }
    }
}

fn parse_path(pair: Pair<Rule>) -> Path {
    assert_eq!(pair.as_rule(), Rule::path);
    let mut path = Path::new();
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::non_quoted_identifier => {
                path.add_attr(p.as_str().to_owned());
            }
            Rule::quoted_identifier => {
                path.add_attr(p.as_str().to_owned().replace("``", "`"));
            }
            Rule::list_index_number => path.add_index(p.as_str().to_owned()),
            _ => {
                // this must not happen
                unreachable!("Unexpected element on path")
            }
        }
    }
    path
}

fn parse_list_append_parameter(pair: Pair<Rule>) -> Result<ListAppendParameter, ParseError> {
    assert_eq!(pair.as_rule(), Rule::list_append_parameter);
    let pair = pair.into_inner().next().unwrap();
    match pair.as_rule() {
        Rule::path => Ok(ListAppendParameter::Path(parse_path(pair))),
        Rule::list_literal => Ok(ListAppendParameter::ListLiteral(AttrVal::L(
            parse_list_literal(pair)?,
        ))),
        _ => {
            // this must not happen
            unreachable!("Invalid parameter of list_append")
        }
    }
}

fn parse_function(pair: Pair<Rule>) -> Result<Function, ParseError> {
    assert_eq!(pair.as_rule(), Rule::function);
    // this unwrap is safe because function has exactly one children
    let pair = pair.into_inner().next().unwrap();
    match pair.as_rule() {
        Rule::list_append_function => {
            let mut pair = pair.into_inner();
            let lhs = pair.next().unwrap();
            let rhs = pair.next().unwrap();
            let lhs_list = parse_list_append_parameter(lhs)?;
            let rhs_list = parse_list_append_parameter(rhs)?;
            Ok(Function::ListAppendFunction(lhs_list, rhs_list))
        }
        Rule::if_not_exists_function => {
            let mut pair = pair.into_inner();
            let path = pair.next().unwrap();
            let value = pair.next().unwrap();
            let path_expression = parse_path(path);
            let value_expression = parse_value(value)?;
            Ok(Function::IfNotExistsFunction(
                path_expression,
                Box::new(value_expression),
            ))
        }
        _ => {
            // this must not happen
            unreachable!("Invalid function expression")
        }
    }
}

fn parse_operand(pair: Pair<Rule>) -> Result<Operand, ParseError> {
    assert_eq!(pair.as_rule(), Rule::operand);
    // this unwrap is safe because function has exactly one children
    let pair = pair.into_inner().next().unwrap();
    match pair.as_rule() {
        Rule::function => Ok(Operand::Function(parse_function(pair)?)),
        Rule::path => Ok(Operand::Path(parse_path(pair))),
        _ => Ok(Operand::Literal(parse_literal(pair)?)),
    }
}

fn parse_value(pair: Pair<Rule>) -> Result<Value, ParseError> {
    assert_eq!(pair.as_rule(), Rule::value);
    // this unwrap is safe because value has exactly one children
    let pair = pair.into_inner().next().unwrap();
    match pair.as_rule() {
        Rule::plus_expression => {
            if let Some((lhs, rhs)) = pair.into_inner().collect_tuple() {
                let lhs = parse_operand(lhs)?;
                let rhs = parse_operand(rhs)?;
                Ok(Value::PlusExpression(lhs, rhs))
            } else {
                // this must not happen
                unreachable!("Invalid plus expression is detected");
            }
        }
        Rule::minus_expression => {
            if let Some((lhs, rhs)) = pair.into_inner().collect_tuple() {
                let lhs = parse_operand(lhs)?;
                let rhs = parse_operand(rhs)?;
                Ok(Value::MinusExpression(lhs, rhs))
            } else {
                // this must not happen
                unreachable!("Invalid plus expression is detected");
            }
        }
        Rule::operand => Ok(Value::Operand(parse_operand(pair)?)),
        _ => {
            // this must not happen
            unreachable!("Unexpected expression is detected");
        }
    }
}

fn parse_sort_key_condition(pair: Pair<Rule>) -> Result<SortKeyCondition, ParseError> {
    assert_eq!(pair.as_rule(), Rule::sort_key);
    // this unwrap is safe because sort_key exactly one children
    let pair = pair.into_inner().next().unwrap();
    match pair.as_rule() {
        Rule::sort_eq => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            let literal = parse_literal(literal_pair)?;
            Ok(SortKeyCondition::Eq(literal))
        }
        Rule::sort_le => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            let literal = parse_literal(literal_pair)?;
            Ok(SortKeyCondition::Le(literal))
        }
        Rule::sort_lt => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            let literal = parse_literal(literal_pair)?;
            Ok(SortKeyCondition::Lt(literal))
        }
        Rule::sort_ge => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            let literal = parse_literal(literal_pair)?;
            Ok(SortKeyCondition::Ge(literal))
        }
        Rule::sort_gt => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            let literal = parse_literal(literal_pair)?;
            Ok(SortKeyCondition::Gt(literal))
        }
        Rule::sort_between => {
            let mut it = pair.into_inner();
            // this unwrap is always safe
            let start_pair = it.next().unwrap();
            let start = parse_literal(start_pair)?;
            // this unwrap is always safe
            let end_pair = it.next().unwrap();
            let end = parse_literal(end_pair)?;
            Ok(SortKeyCondition::Between(start, end))
        }
        Rule::sort_begins_with => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            let input_str = literal_pair.to_string();
            let literal = parse_literal(literal_pair)?;
            if let AttrVal::S(prefix) = literal {
                Ok(SortKeyCondition::BeginsWith(AttrVal::S(prefix)))
            } else {
                Err(ParseError::InvalidBeginsWith(input_str))
            }
        }
        _ => {
            // this must not happen
            unreachable!("Unexpected sort condition is detected");
        }
    }
}

fn parse_sort_key_str_pair(pair: Pair<Rule>) -> Result<SortKeyCondition, ParseError> {
    assert_eq!(pair.as_rule(), Rule::sort_key_str);
    // this unwrap is safe because sort_key exactly one children
    let pair = pair.into_inner().next().unwrap();
    match pair.as_rule() {
        Rule::bare_str => Ok(SortKeyCondition::Eq(AttrVal::S(pair.as_str().to_owned()))),
        Rule::sort_eq_str => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            Ok(SortKeyCondition::Eq(AttrVal::S(
                literal_pair.as_str().to_owned(),
            )))
        }
        Rule::sort_le_str => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            Ok(SortKeyCondition::Le(AttrVal::S(
                literal_pair.as_str().to_owned(),
            )))
        }
        Rule::sort_lt_str => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            Ok(SortKeyCondition::Lt(AttrVal::S(
                literal_pair.as_str().to_owned(),
            )))
        }
        Rule::sort_ge_str => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            Ok(SortKeyCondition::Ge(AttrVal::S(
                literal_pair.as_str().to_owned(),
            )))
        }
        Rule::sort_gt_str => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            Ok(SortKeyCondition::Gt(AttrVal::S(
                literal_pair.as_str().to_owned(),
            )))
        }
        Rule::sort_between_str => {
            let mut it = pair.into_inner();
            // this unwrap is always safe
            let start_pair = it.next().unwrap();
            let start = AttrVal::S(start_pair.as_str().to_owned());
            // this unwrap is always safe
            let end_pair = it.next().unwrap();
            let end = AttrVal::S(end_pair.as_str().to_owned());
            Ok(SortKeyCondition::Between(start, end))
        }
        Rule::sort_begins_with_str => {
            // this unwrap is always safe
            let prefix_pair = pair.into_inner().next().unwrap();
            let prefix = prefix_pair.as_str().to_owned();
            Ok(SortKeyCondition::BeginsWith(AttrVal::S(prefix)))
        }
        _ => {
            // this must not happen
            unreachable!("Unexpected sort condition is detected");
        }
    }
}

fn parse_sort_key_number_pair(pair: Pair<Rule>) -> Result<SortKeyCondition, ParseError> {
    assert_eq!(pair.as_rule(), Rule::sort_key_number);

    // this unwrap is safe because sort_key exactly one children
    let pair = pair.into_inner().next().unwrap();

    // In the current implementation, only the code path of `number_literal` is used because the fallback behavior is not needed in other patterns.
    // However, for the sake of the completeness of the implementation, the rest of the codes also is retained.
    match pair.as_rule() {
        Rule::number_literal => Ok(SortKeyCondition::Eq(AttrVal::N(pair.as_str().to_owned()))),
        Rule::sort_eq_num => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            Ok(SortKeyCondition::Eq(AttrVal::S(
                literal_pair.as_str().to_owned(),
            )))
        }
        Rule::sort_le_num => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            Ok(SortKeyCondition::Le(AttrVal::S(
                literal_pair.as_str().to_owned(),
            )))
        }
        Rule::sort_lt_num => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            Ok(SortKeyCondition::Lt(AttrVal::S(
                literal_pair.as_str().to_owned(),
            )))
        }
        Rule::sort_ge_num => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            Ok(SortKeyCondition::Ge(AttrVal::S(
                literal_pair.as_str().to_owned(),
            )))
        }
        Rule::sort_gt_num => {
            // this unwrap is always safe
            let literal_pair = pair.into_inner().next().unwrap();
            Ok(SortKeyCondition::Gt(AttrVal::S(
                literal_pair.as_str().to_owned(),
            )))
        }
        Rule::sort_between_num => {
            let mut it = pair.into_inner();
            // this unwrap is always safe
            let start_pair = it.next().unwrap();
            let start = AttrVal::S(start_pair.as_str().to_owned());
            // this unwrap is always safe
            let end_pair = it.next().unwrap();
            let end = AttrVal::S(end_pair.as_str().to_owned());
            Ok(SortKeyCondition::Between(start, end))
        }
        _ => {
            // this must not happen
            unreachable!("Unexpected sort condition is detected");
        }
    }
}

fn parse_remove_action_pair(pair: Pair<Rule>) -> RemoveAction {
    assert_eq!(pair.as_rule(), Rule::remove_action);
    let mut remove_actions = Vec::new();
    for pair in pair.into_inner() {
        let path = parse_path(pair);
        remove_actions.push(AtomicRemove { path })
    }
    remove_actions
}

fn parse_set_action_pair(pair: Pair<Rule>) -> Result<SetAction, ParseError> {
    assert_eq!(pair.as_rule(), Rule::set_action);
    let mut set_actions = Vec::new();
    for chunk in pair.into_inner().chunks(2).into_iter() {
        if let Some((path, value)) = chunk.collect_tuple() {
            let path = parse_path(path);
            let value = parse_value(value)?;
            set_actions.push(AtomicSet { path, value });
        } else {
            // this must not happen
            unreachable!("Unpaired set action is detected")
        }
    }
    Ok(set_actions)
}

fn attr_name_ref(idx: usize) -> String {
    format!("#DYNEIN_ATTRNAME{}", idx)
}

fn attr_val_ref(idx: usize) -> String {
    format!(":DYNEIN_ATTRVAL{}", idx)
}

/// The parser for dynein.
#[derive(Debug, Clone, PartialEq)]
pub struct DyneinParser {
    names: HashMap<String, String>,
    names_inv: HashMap<String, String>,
    values: HashMap<String, AttributeValue>,
}

impl DyneinParser {
    /// Create a new parser.
    ///
    /// The created parser has a context for API calls, `ExpressionAttributeNames` and `ExpressionAttributeValues`.
    /// These contexts can be shared by multiple actions.
    pub fn new() -> DyneinParser {
        DyneinParser {
            names: HashMap::new(),
            names_inv: HashMap::new(),
            values: HashMap::new(),
        }
    }

    /// Clear the parser internal state for `ExpressionAttributeNames` and `ExpressionAttributeValues`.
    ///
    /// Currently, this function is used for testing purposes only.
    #[cfg(test)]
    fn clear(&mut self) {
        self.names.clear();
        self.names_inv.clear();
        self.values.clear();
    }

    /// Parse sort key condition in non-strict mode.
    pub fn parse_sort_key_with_fallback(
        &mut self,
        exp: &str,
        sort_attr: &AttributeDefinition,
    ) -> Result<ExpressionResult, ParseError> {
        self.parse_sort_key_without_fallback(exp, sort_attr)
            .or_else(|err| match sort_attr.attribute_type {
                AttributeType::S => self.parse_and_process_sort_key_for_string(exp, sort_attr),
                AttributeType::N => self.parse_and_process_sort_key_for_number(exp, sort_attr),
                _ => Err(err),
            })
    }

    /// Parse sort key condition in strict mode.
    pub fn parse_sort_key_with_suggest(
        &mut self,
        exp: &str,
        sort_attr: &AttributeDefinition,
    ) -> Result<ExpressionResult, ParseError> {
        let mut pair = GeneratedParser::parse(Rule::sort_key, exp).map_err(|err| {
            let fallback_result = self.try_sort_key_parse(exp, sort_attr);
            match fallback_result {
                Some(exp) => ParseError::ParsingErrorWithSuggest(ParsingErrorWithSuggestError {
                    parse_error: Box::new(err),
                    suggest: format!("{}", exp),
                }),
                None => ParseError::ParsingError(Box::new(err)),
            }
        })?;
        let condition = parse_sort_key_condition(pair.next().unwrap())?;
        self.process_sort_key(exp, sort_attr, condition)
    }

    /// Parse sort key condition in strict mode.
    fn parse_sort_key_without_fallback(
        &mut self,
        exp: &str,
        sort_attr: &AttributeDefinition,
    ) -> Result<ExpressionResult, ParseError> {
        let mut pair = GeneratedParser::parse(Rule::sort_key, exp)
            .map_err(|err| ParseError::ParsingError(Box::new(err)))?;
        let condition = parse_sort_key_condition(pair.next().unwrap())?;
        self.process_sort_key(exp, sort_attr, condition)
    }

    pub fn parse_dynein_format(
        &self,
        initial_item: Option<HashMap<String, AttributeValue>>,
        exp: &str,
    ) -> Result<HashMap<String, AttributeValue>, ParseError> {
        let result = GeneratedParser::parse(Rule::map_literal, exp);
        match result {
            Ok(mut pair) => {
                let item = parse_literal(pair.next().unwrap())?
                    .convert_attribute_value()
                    .as_m()
                    .unwrap()
                    .to_owned();
                // content must be map literal
                let mut image = match initial_item {
                    Some(init_item) => init_item,
                    None => HashMap::new(),
                };
                image.extend(item);
                Ok(image)
            }
            Err(err) => Err(ParseError::ParsingError(Box::new(err))),
        }
    }

    /// Parse set actions.
    ///
    /// You can call this more than once.
    /// In this case, you have a responsibility to merge the `exp` of [`ExpressionResult`].
    pub fn parse_set_action(&mut self, exp: &str) -> Result<ExpressionResult, ParseError> {
        let result = GeneratedParser::parse(Rule::set_action, exp);
        match result {
            Ok(mut pair) => {
                let set_action = parse_set_action_pair(pair.next().unwrap())?;
                self.process_set_action(set_action)
            }
            Err(err) => Err(ParseError::ParsingError(Box::new(err))),
        }
    }

    /// Parse remove actions.
    ///
    /// You can call this more than once.
    /// In this case, you have a responsibility to merge the `exp` of [`ExpressionResult`].
    pub fn parse_remove_action(&mut self, exp: &str) -> Result<ExpressionResult, ParseError> {
        let result = GeneratedParser::parse(Rule::remove_action, exp);
        match result {
            Ok(mut pair) => {
                let remove_action = parse_remove_action_pair(pair.next().unwrap());
                self.process_remove_action(remove_action)
            }
            Err(err) => Err(ParseError::ParsingError(Box::new(err))),
        }
    }

    fn try_sort_key_parse(
        &self,
        exp: &str,
        sort_attr: &AttributeDefinition,
    ) -> Option<SortKeyCondition> {
        match sort_attr.attribute_type {
            AttributeType::S => self.try_parse_sort_key_for_string(exp).ok(),
            AttributeType::N => self.try_parse_sort_key_for_number(exp).ok(),
            _ => None,
        }
    }

    fn try_parse_sort_key_for_string(&self, exp: &str) -> Result<SortKeyCondition, ParseError> {
        let result = GeneratedParser::parse(Rule::sort_key_str, exp);
        match result {
            Ok(mut pair) => Ok(parse_sort_key_str_pair(pair.next().unwrap())?),
            Err(err) => Err(ParseError::ParsingError(Box::new(err))),
        }
    }

    fn parse_and_process_sort_key_for_string(
        &mut self,
        exp: &str,
        sort_attr: &AttributeDefinition,
    ) -> Result<ExpressionResult, ParseError> {
        let condition = self.try_parse_sort_key_for_string(exp)?;
        self.process_sort_key(exp, sort_attr, condition)
    }

    fn try_parse_sort_key_for_number(&self, exp: &str) -> Result<SortKeyCondition, ParseError> {
        let result = GeneratedParser::parse(Rule::sort_key_number, exp);
        match result {
            Ok(mut pair) => Ok(parse_sort_key_number_pair(pair.next().unwrap())?),
            Err(err) => Err(ParseError::ParsingError(Box::new(err))),
        }
    }

    fn parse_and_process_sort_key_for_number(
        &mut self,
        exp: &str,
        sort_attr: &AttributeDefinition,
    ) -> Result<ExpressionResult, ParseError> {
        let condition = self.try_parse_sort_key_for_number(exp)?;
        self.process_sort_key(exp, sort_attr, condition)
    }

    fn get_or_create_attr_name_ref(&mut self, attr_name: String) -> String {
        match self.names_inv.entry(attr_name.to_owned()) {
            Entry::Occupied(o) => o.get().to_owned(),
            Entry::Vacant(v) => {
                let ref_name = attr_name_ref(self.names.len());
                v.insert(ref_name.to_owned());
                self.names.insert(ref_name.to_owned(), attr_name);
                ref_name
            }
        }
    }

    fn add_value_and_return_ref(&mut self, value: AttrVal) -> String {
        let idx = self.values.len();
        let ref_name = attr_val_ref(idx);
        let value = value.convert_attribute_value();
        self.values.insert(ref_name.to_owned(), value);
        ref_name
    }

    fn process_path(&mut self, input: Path) -> String {
        let mut expression = String::new();
        let mut is_first = true;
        for elem in input.elements {
            match elem {
                PathElement::Attribute(name) => {
                    let name_ref = self.get_or_create_attr_name_ref(name);
                    if is_first {
                        expression.push_str(&name_ref);
                        is_first = false;
                    } else {
                        expression.push('.');
                        expression.push_str(&name_ref)
                    }
                }
                PathElement::Index(idx) => {
                    expression.push('[');
                    expression.push_str(&idx);
                    expression.push(']');
                }
            }
        }
        expression
    }

    fn process_literal(&mut self, input: AttrVal) -> Result<String, ParseError> {
        Ok(self.add_value_and_return_ref(input))
    }

    fn process_list_append_parameter(
        &mut self,
        input: ListAppendParameter,
    ) -> Result<String, ParseError> {
        match input {
            ListAppendParameter::Path(path) => Ok(self.process_path(path)),
            ListAppendParameter::ListLiteral(literal) => self.process_literal(literal),
        }
    }

    fn process_function(&mut self, input: Function) -> Result<String, ParseError> {
        match input {
            Function::ListAppendFunction(lhs, rhs) => {
                let mut expression = "list_append(".to_owned();
                let lhs = self.process_list_append_parameter(lhs)?;
                let rhs = self.process_list_append_parameter(rhs)?;
                expression.push_str(&lhs);
                expression.push(',');
                expression.push_str(&rhs);
                expression.push(')');
                Ok(expression)
            }
            Function::IfNotExistsFunction(path, value) => {
                let mut expression = "if_not_exists(".to_owned();
                let path_expression = self.process_path(path);
                let value_expression = self.process_value(*value)?;
                expression.push_str(&path_expression);
                expression.push(',');
                expression.push_str(&value_expression);
                expression.push(')');
                Ok(expression)
            }
        }
    }

    fn process_operand(&mut self, input: Operand) -> Result<String, ParseError> {
        match input {
            Operand::Function(function) => self.process_function(function),
            Operand::Literal(literal) => self.process_literal(literal),
            Operand::Path(path) => Ok(self.process_path(path)),
        }
    }

    fn process_value(&mut self, input: Value) -> Result<String, ParseError> {
        match input {
            Value::PlusExpression(lhs, rhs) => {
                let mut lhs = self.process_operand(lhs)?;
                let rhs = self.process_operand(rhs)?;
                lhs.push('+');
                lhs.push_str(&rhs);
                Ok(lhs)
            }
            Value::MinusExpression(lhs, rhs) => {
                let mut lhs = self.process_operand(lhs)?;
                let rhs = self.process_operand(rhs)?;
                lhs.push('-');
                lhs.push_str(&rhs);
                Ok(lhs)
            }
            Value::Operand(op) => self.process_operand(op),
        }
    }

    fn process_sort_key(
        &mut self,
        exp: &str,
        sort_attr: &AttributeDefinition,
        condition: SortKeyCondition,
    ) -> Result<ExpressionResult, ParseError> {
        let mut expression = String::new();
        let attr_type = sort_attr.attribute_type;
        let attr_name = &sort_attr.attribute_name;
        let mut process_op = |val: AttrVal, op: &str| -> Result<ExpressionResult, ParseError> {
            if !val.is_type(attr_type) {
                let fallback_result = self.try_sort_key_parse(exp, sort_attr);
                let err = match fallback_result {
                    Some(exp) => {
                        ParseError::InvalidTypesWithSuggest(InvalidTypesWithSuggestError {
                            expected_type: attr_type.to_owned(),
                            actual_type: val.attribute_type(),
                            suggest: format!("{}", exp),
                        })
                    }
                    None => ParseError::InvalidTypes(InvalidTypesError {
                        expected_type: attr_type.to_owned(),
                        actual_type: val.attribute_type(),
                    }),
                };
                Err(err)?;
            }
            let path = self.get_or_create_attr_name_ref(attr_name.to_owned());
            let value = self.process_literal(val)?;
            expression.push_str(&path);
            expression.push_str(op);
            expression.push_str(&value);
            Ok(ExpressionResult {
                exp: expression.to_owned(),
                names: self.names.clone(),
                values: self.values.clone(),
            })
        };
        match condition {
            SortKeyCondition::Eq(val) => process_op(val, "="),
            SortKeyCondition::Le(val) => process_op(val, "<="),
            SortKeyCondition::Lt(val) => process_op(val, "<"),
            SortKeyCondition::Ge(val) => process_op(val, ">="),
            SortKeyCondition::Gt(val) => process_op(val, ">"),
            SortKeyCondition::Between(start, end) => {
                if !start.is_type(attr_type) || !end.is_type(attr_type) {
                    if !start.is_type(attr_type) {
                        Err(ParseError::InvalidTypes(InvalidTypesError {
                            expected_type: attr_type.to_owned(),
                            actual_type: start.attribute_type(),
                        }))?;
                    } else {
                        Err(ParseError::InvalidTypes(InvalidTypesError {
                            expected_type: attr_type.to_owned(),
                            actual_type: end.attribute_type(),
                        }))?;
                    }
                }
                let path = self.get_or_create_attr_name_ref(attr_name.to_owned());
                let start = self.process_literal(start)?;
                let end = self.process_literal(end)?;
                expression.push_str(&path);
                expression.push_str(" BETWEEN ");
                expression.push_str(&start);
                expression.push_str(" AND ");
                expression.push_str(&end);
                Ok(ExpressionResult {
                    exp: expression.to_owned(),
                    names: self.names.clone(),
                    values: self.values.clone(),
                })
            }
            SortKeyCondition::BeginsWith(prefix) => {
                if !prefix.is_type(attr_type) {
                    Err(ParseError::InvalidTypes(InvalidTypesError {
                        expected_type: attr_type.to_owned(),
                        actual_type: prefix.attribute_type(),
                    }))?;
                }
                let path = self.get_or_create_attr_name_ref(attr_name.to_owned());
                let prefix = self.process_literal(prefix)?;
                expression.push_str("begins_with(");
                expression.push_str(&path);
                expression.push(',');
                expression.push_str(&prefix);
                expression.push(')');
                Ok(ExpressionResult {
                    exp: expression.to_owned(),
                    names: self.names.clone(),
                    values: self.values.clone(),
                })
            }
        }
    }

    fn process_set_action(&mut self, input: SetAction) -> Result<ExpressionResult, ParseError> {
        let mut expression = String::new();
        for set in input {
            let path = self.process_path(set.path);
            let value = self.process_value(set.value)?;
            if !expression.is_empty() {
                expression.push(',');
            }
            expression.push_str(&path);
            expression.push('=');
            expression.push_str(&value);
        }
        Ok(ExpressionResult {
            exp: expression.to_owned(),
            names: self.names.clone(),
            values: self.values.clone(),
        })
    }

    fn process_remove_action(
        &mut self,
        input: RemoveAction,
    ) -> Result<ExpressionResult, ParseError> {
        let mut expression = String::new();
        for remove in input {
            let path = self.process_path(remove.path);
            if !expression.is_empty() {
                expression.push(',');
            }
            expression.push_str(&path);
        }
        Ok(ExpressionResult {
            exp: expression.to_owned(),
            names: self.names.clone(),
            values: self.values.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attr_val_display() {
        let item = AttrVal::M(HashMap::from([
            (
                "\"k\ne\0y\r\"".to_owned(),
                AttrVal::S("\tstr\x08\x0c🍣\u{009F}bfnrt".to_owned()),
            ),
            ("n".to_owned(), AttrVal::N("123".to_owned())),
            ("null".to_owned(), AttrVal::Null(true)),
            ("true".to_owned(), AttrVal::Bool(true)),
            ("false".to_owned(), AttrVal::Bool(false)),
            (
                "b".to_owned(),
                AttrVal::B(Bytes::from_static(b"\xf0\x9f\x8d\xa3\n\0")),
            ),
            (
                "l".to_owned(),
                AttrVal::L(vec![
                    AttrVal::N("1".to_owned()),
                    AttrVal::S("2".to_owned()),
                    AttrVal::B(Bytes::from_static(b"\x03")),
                ]),
            ),
            ("m0".to_owned(), AttrVal::M(HashMap::new())),
            (
                "m1".to_owned(),
                AttrVal::M(HashMap::from([(
                    "m2".to_owned(),
                    AttrVal::M(HashMap::from([(
                        "m3".to_owned(),
                        AttrVal::S("nested".to_owned()),
                    )])),
                )])),
            ),
            (
                "ss".to_owned(),
                AttrVal::SS(vec!["1".to_owned(), "2".to_owned()]),
            ),
            (
                "ns".to_owned(),
                AttrVal::NS(vec!["1".to_owned(), "2".to_owned()]),
            ),
            (
                "bs".to_owned(),
                AttrVal::BS(vec![
                    Bytes::from_static(b"\x01"),
                    Bytes::from_static(b"\x02"),
                ]),
            ),
        ]));
        let result = format!("{}", item);
        let chars: Vec<_> = result.chars().collect();
        assert_eq!(*chars.first().unwrap(), '{');
        assert_eq!(*chars.last().unwrap(), '}');
        assert_ne!(chars[chars.len() - 2], ',');
        assert!(result.contains(r#""\"k\ne\u0000y\r\"":"\tstr\b\f🍣\u009fbfnrt""#));
        assert!(result.contains(r#""n":123"#));
        assert!(result.contains(r#""null":null"#));
        assert!(result.contains(r#""true":true"#));
        assert!(result.contains(r#""false":false"#));
        assert!(result.contains(r#""b":b64"8J+NowoA""#));
        assert!(result.contains(r#""l":[1,"2",b64"Aw=="]"#));
        assert!(result.contains(r#""m0":{}"#));
        assert!(result.contains(r#""m1":{"m2":{"m3":"nested"}}"#));
        assert!(result.contains(r#""ss":<<"1","2">>"#));
        assert!(result.contains(r#""ns":<<1,2>>"#));
        assert!(result.contains(r#""bs":<<b64"AQ==",b64"Ag==">>"#));
    }

    #[test]
    fn test_sort_key_condition_display() {
        assert_eq!(
            format!("{}", SortKeyCondition::Eq(AttrVal::S("str".to_owned()))).as_str(),
            r#"= "str""#
        );
        assert_eq!(
            format!("{}", SortKeyCondition::Eq(AttrVal::N("12".to_owned()))).as_str(),
            r#"= 12"#
        );
        assert_eq!(
            format!(
                "{}",
                SortKeyCondition::Eq(AttrVal::B(Bytes::from_static(b"str")))
            )
            .as_str(),
            r#"= b64"c3Ry""#
        );
        assert_eq!(
            format!("{}", SortKeyCondition::Le(AttrVal::S("str".to_owned()))).as_str(),
            r#"<= "str""#
        );
        assert_eq!(
            format!("{}", SortKeyCondition::Lt(AttrVal::N("12".to_owned()))).as_str(),
            r#"< 12"#
        );
        assert_eq!(
            format!(
                "{}",
                SortKeyCondition::Ge(AttrVal::B(Bytes::from_static(b"str")))
            )
            .as_str(),
            r#">= b64"c3Ry""#
        );
        assert_eq!(
            format!("{}", SortKeyCondition::Gt(AttrVal::S("str".to_owned()))).as_str(),
            r#"> "str""#
        );
        assert_eq!(
            format!(
                "{}",
                SortKeyCondition::Between(AttrVal::N("1".to_owned()), AttrVal::N("2".to_owned()))
            )
            .as_str(),
            r#"between 1 and 2"#
        );
        assert_eq!(
            format!(
                "{}",
                SortKeyCondition::BeginsWith(AttrVal::S("str".to_owned()))
            )
            .as_str(),
            r#"begins_with "str""#
        );
    }

    #[test]
    fn test_parse_internal_double_quote_string() {
        // JSON based syntax
        assert_eq!(parse_internal_double_quote_string("a").unwrap(), "a");
        assert_eq!(parse_internal_double_quote_string("'").unwrap(), "'");
        assert_eq!(
            parse_internal_double_quote_string("\\r\\n").unwrap(),
            "\r\n"
        );

        assert_eq!(parse_internal_double_quote_string("\\\"").unwrap(), "\"");
        assert_eq!(parse_internal_double_quote_string("\\\\").unwrap(), "\\");
        assert_eq!(parse_internal_double_quote_string("\\/").unwrap(), "/");
        assert_eq!(parse_internal_double_quote_string("\\b").unwrap(), "\x08");
        assert_eq!(parse_internal_double_quote_string("\\f").unwrap(), "\x0c");
        assert_eq!(parse_internal_double_quote_string("\\n").unwrap(), "\n");
        assert_eq!(parse_internal_double_quote_string("\\r").unwrap(), "\r");
        assert_eq!(parse_internal_double_quote_string("\\t").unwrap(), "\t");

        assert_eq!(parse_internal_double_quote_string("\\u002F").unwrap(), "/");
        assert_eq!(parse_internal_double_quote_string("\\u002f").unwrap(), "/");
        assert_eq!(parse_internal_double_quote_string("/").unwrap(), "/");

        assert_eq!(
            parse_internal_double_quote_string("\\uD834\\uDD1E").unwrap(),
            "𝄞"
        );

        // Expanded syntax by dynein (some of which was inspired by Rust)
        assert_eq!(parse_internal_double_quote_string("\\0").unwrap(), "\0");
        assert_eq!(parse_internal_double_quote_string("\\'").unwrap(), "'");
        assert_eq!(parse_internal_double_quote_string("\\\"").unwrap(), "\"");
        assert_eq!(parse_internal_double_quote_string("\0").unwrap(), "\0");
        assert_eq!(parse_internal_double_quote_string("\r\n").unwrap(), "\r\n");
        assert_eq!(parse_internal_double_quote_string("\r").unwrap(), "\r");
        assert_eq!(parse_internal_double_quote_string("\n").unwrap(), "\n");
        assert_eq!(parse_internal_double_quote_string("\t").unwrap(), "\t");

        // The following cases should not happen typically, but we check them for sure of robustness.
        assert_eq!(parse_internal_double_quote_string("\"").unwrap(), "\"");

        // Invalid escape
        assert_eq!(
            parse_internal_double_quote_string("\\").expect_err("It must not Ok()"),
            ParseError::UnexpectedEndOfSequence(EscapeCharUnexpectedEndOfSequenceError {
                handling_target: "\\".to_owned(),
                escape_pos: 0,
            })
        );
        // g is not valid hex digit
        assert_eq!(
            parse_internal_double_quote_string("\\udefg").expect_err("It must not Ok()"),
            ParseError::InvalidEscapeChar(EscapeCharError {
                handling_target: "\\udefg".to_owned(),
                invalid_char: 'g',
                escape_pos: 0,
            })
        );
        // Incomplete surrogate pair
        assert_eq!(
            parse_internal_double_quote_string("\\uD834").expect_err("It must not Ok()"),
            ParseError::UnexpectedEndOfSequence(EscapeCharUnexpectedEndOfSequenceError {
                handling_target: "\\uD834".to_owned(),
                escape_pos: 0,
            })
        );

        // Multilingual checks
        assert_eq!(
            parse_internal_double_quote_string("This is a line.\\nこれは行です。").unwrap(),
            "This is a line.\nこれは行です。"
        );
    }

    #[test]
    fn test_parse_single_quote_literal() {
        assert_eq!(parse_single_quote_literal("'a'"), "a");
        assert_eq!(parse_single_quote_literal("'\\0'"), "\\0");
        assert_eq!(parse_single_quote_literal("'\\r\\n'"), "\\r\\n");
        assert_eq!(parse_single_quote_literal("'\\r'"), "\\r");
        assert_eq!(parse_single_quote_literal("'\\n'"), "\\n");
        assert_eq!(parse_single_quote_literal("'\\t'"), "\\t");
        assert_eq!(parse_single_quote_literal("'\\\\'"), "\\\\");
        assert_eq!(parse_single_quote_literal("'\\''"), "\\'");
        assert_eq!(parse_single_quote_literal("'\\\"'"), "\\\"");
    }

    #[test]
    fn test_hex_as_byte() {
        assert_eq!(hex_as_byte("0", 0, b'0').unwrap(), 0);
        assert_eq!(hex_as_byte("9", 0, b'9').unwrap(), 9);
        assert_eq!(hex_as_byte("a", 0, b'a').unwrap(), 10);
        assert_eq!(hex_as_byte("f", 0, b'f').unwrap(), 15);
        assert_eq!(
            hex_as_byte("g", 0, b'g').unwrap_err(),
            ParseError::InvalidEscapeByte(EscapeByteError {
                handling_target: "g".to_owned(),
                escape_pos: 0,
                escape_byte: b'g',
            })
        );
        assert_eq!(
            hex_as_byte("dummy", 0, b'\xff').unwrap_err(),
            ParseError::InvalidEscapeByte(EscapeByteError {
                handling_target: "dummy".to_owned(),
                escape_pos: 0,
                escape_byte: b'\xff',
            })
        );
    }

    #[test]
    fn test_parse_internal_binary_literal() {
        assert_eq!(
            parse_internal_binary_literal("\\xDE\\xAD\\xbe\\xef").unwrap(),
            Bytes::from_static(b"\xde\xad\xbe\xef")
        );
        assert_eq!(
            parse_internal_binary_literal("\\n\\r\\t\\\\\\0\\'\\\"").unwrap(),
            Bytes::from_static(b"\n\r\t\\\0\'\"")
        );
        assert_eq!(
            parse_internal_binary_literal("\\xZZ").unwrap_err(),
            ParseError::InvalidEscapeByte(EscapeByteError {
                handling_target: "\\xZZ".to_owned(),
                escape_pos: 2,
                escape_byte: b'Z',
            })
        );
    }

    #[test]
    fn test_parse_internal_binary_string() {
        let binary_string = "a\\\n\r\n\t b\\\r\r\n\t c\\xDE\\xAD\\xbe\\xef\\r\\n\\t\\\\\\0\\'\\\"";
        let expect_binary = b"abc\xde\xad\xbe\xef\r\n\t\\\0'\"";
        assert_eq!(
            parse_internal_binary_string(binary_string).unwrap(),
            Bytes::from_static(expect_binary)
        );
    }

    #[test]
    fn test_parse_b64_literal() {
        let literal = "b64'AA=='";
        assert_eq!(
            parse_b64_literal(literal).unwrap(),
            Bytes::from_static(b"\0")
        );
        let literal = "b64'AA'";
        assert_eq!(
            parse_b64_literal(literal).unwrap(),
            Bytes::from_static(b"\0")
        );
        let literal = "b64'SGk='";
        assert_eq!(
            parse_b64_literal(literal).unwrap(),
            Bytes::from_static(b"Hi")
        );
        let literal = r#"b64"QmluCg==""#;
        assert_eq!(
            parse_b64_literal(literal).unwrap(),
            Bytes::from_static(b"Bin\n")
        );
        let literal = r#"b64"RAEBog==""#;
        let expect_binary = b"\x44\x01\x01\xa2";
        assert_eq!(
            parse_b64_literal(literal).unwrap(),
            Bytes::from_static(expect_binary)
        );
    }

    #[test]
    fn test_parse_literal() {
        // boolean literal
        let parsed_result = GeneratedParser::parse(Rule::literal, "true")
            .unwrap()
            .next()
            .unwrap();
        let true_literal = parse_literal(parsed_result).unwrap();
        assert_eq!(true_literal, AttrVal::Bool(true));

        let parsed_result = GeneratedParser::parse(Rule::literal, "false")
            .unwrap()
            .next()
            .unwrap();
        let false_literal = parse_literal(parsed_result).unwrap();
        assert_eq!(false_literal, AttrVal::Bool(false));

        // null literal
        let parsed_result = GeneratedParser::parse(Rule::literal, "null")
            .unwrap()
            .next()
            .unwrap();
        let null_literal = parse_literal(parsed_result).unwrap();
        assert_eq!(null_literal, AttrVal::Null(true));

        // double quoted string literal
        let parsed_result = GeneratedParser::parse(Rule::literal, "\"🍣 is \\\"sushi\\\"!\"")
            .unwrap()
            .next()
            .unwrap();
        let sushi_string = parse_literal(parsed_result).unwrap();
        assert_eq!(sushi_string, AttrVal::S("🍣 is \"sushi\"!".to_owned()));

        let parsed_result = GeneratedParser::parse(Rule::literal, "\"\\0\\r\\n\\t\\\\\\\"\\'\"")
            .unwrap()
            .next()
            .unwrap();
        let all_escape_string = parse_literal(parsed_result).unwrap();
        assert_eq!(all_escape_string, AttrVal::S("\0\r\n\t\\\"\'".to_owned()));

        let parsed_result = GeneratedParser::parse(
            Rule::literal,
            r###""\"\\\/\b\f\n\r\t\u002F\u002f\uD834\uDD1E""###,
        )
        .unwrap()
        .next()
        .unwrap();
        let json_compatible_string = parse_literal(parsed_result).unwrap();
        assert_eq!(json_compatible_string, AttrVal::S("\u{0022}\u{005c}\u{002f}\u{0008}\u{000c}\u{000a}\u{000d}\u{0009}\u{002f}\u{002f}\u{1d11e}".to_owned()));

        // single quoted string literal
        let parsed_result = GeneratedParser::parse(Rule::literal, "'Escape must not work\\n'")
            .unwrap()
            .next()
            .unwrap();
        let raw_string = parse_literal(parsed_result).unwrap();
        assert_eq!(raw_string, AttrVal::S("Escape must not work\\n".to_owned()));

        // number literal
        let num_list = [
            "12345678901234567890",
            "0",
            "+1",
            "-1",
            "+0",
            "-0",
            "+0.0",
            "-0.0",
            "3.141592653589793238462643",
            "+1.1",
            "-1.1",
            ".1",
            "1.",
            "0.0",
            "0.",
            ".0",
            "-2.71828182846e-12",
            "1e1",
            "+1e+1",
            "-1e-1",
            "1e0",
            "0e1",
            "0e0", // 0e0 = 0 in DynamoDB
            "1E-130",
            "9.9999999999999999999999999999999999999E+125",
            "-9.9999999999999999999999999999999999999E+125",
            "-1E-130",
        ];
        for num in num_list {
            let parsed_result = GeneratedParser::parse(Rule::literal, num)
                .unwrap()
                .next()
                .unwrap();
            let pi_number = parse_literal(parsed_result).unwrap();
            assert_eq!(pi_number, AttrVal::N(num.to_owned()));
        }

        // list literal
        let parsed_result = GeneratedParser::parse(Rule::literal, "[1,'2', true]")
            .unwrap()
            .next()
            .unwrap();
        let list_literal = parse_literal(parsed_result).unwrap();
        assert_eq!(
            list_literal,
            AttrVal::L(Vec::from([
                AttrVal::N("1".to_owned()),
                AttrVal::S("2".to_owned()),
                AttrVal::Bool(true),
            ]))
        );

        // map literal
        let parsed_result = GeneratedParser::parse(
            Rule::literal,
            "{'1': \"id1\", \"2\": 4, '3': true, 's 1': null}",
        )
        .unwrap()
        .next()
        .unwrap();
        let map_literal = parse_literal(parsed_result).unwrap();
        assert_eq!(
            map_literal,
            AttrVal::M(HashMap::from([
                ("1".to_owned(), AttrVal::S("id1".to_owned())),
                ("2".to_owned(), AttrVal::N("4".to_owned())),
                ("3".to_owned(), AttrVal::Bool(true)),
                ("s 1".to_owned(), AttrVal::Null(true)),
            ]))
        );

        // binary literal
        let parsed_result = GeneratedParser::parse(
            Rule::literal,
            "b'\\xDE\\xAD\\xbe\\xef\\n\\r\\t\\\\\0\\'\\\"'",
        )
        .unwrap()
        .next()
        .unwrap();
        let binary_literal = parse_literal(parsed_result).unwrap();
        assert_eq!(
            binary_literal,
            AttrVal::B(Bytes::from_static(b"\xde\xad\xbe\xef\n\r\t\\\0\'\""))
        );

        // binary string literal
        let parsed_result = GeneratedParser::parse(
            Rule::literal,
            "b\"\\xDE\\xAD\\xbe\\xef\\\n\r\t\\\\\0\\'\\\"\"",
        )
        .unwrap()
        .next()
        .unwrap();
        let binary_string = parse_literal(parsed_result).unwrap();
        assert_eq!(
            binary_string,
            AttrVal::B(Bytes::from_static(b"\xde\xad\xbe\xef\\\0\'\""))
        );

        // b64 literal
        let parsed_result = GeneratedParser::parse(
            Rule::literal,
            // Generated by: echo -n "\\xDE\\xAD\\xbe\\xef\n\r\t\\\\\\0'\"" | base64
            // Hex: de ad be ef 0a 0d 09 5c 00 27 22
            "b64\"3q2+7woNCVwAJyI=\"",
        )
        .unwrap()
        .next()
        .unwrap();
        let binary_string = parse_literal(parsed_result).unwrap();
        assert_eq!(
            binary_string,
            AttrVal::B(Bytes::from_static(b"\xde\xad\xbe\xef\n\r\t\\\0\'\""))
        );

        // string set literal
        let parsed_result = GeneratedParser::parse(Rule::literal, "<<'S1',\"S 2\">>")
            .unwrap()
            .next()
            .unwrap();
        let string_set_literal = parse_literal(parsed_result).unwrap();
        assert_eq!(
            string_set_literal,
            AttrVal::SS(Vec::from(["S1".to_owned(), "S 2".to_owned(),]))
        );

        // number set literal
        let parsed_result = GeneratedParser::parse(Rule::literal, "<<0,-3,1.570,-1e3>>")
            .unwrap()
            .next()
            .unwrap();
        let string_set_literal = parse_literal(parsed_result).unwrap();
        assert_eq!(
            string_set_literal,
            AttrVal::NS(Vec::from([
                "0".to_owned(),
                "-3".to_owned(),
                "1.570".to_owned(),
                "-1e3".to_owned(),
            ]))
        );

        // binary set literal
        let binary_list =
            "<<b'deadbeef',b'\\xde\\xad\\xbe\\xef',b\"wa\\\n\trp\",b\"no-\n\twarp\",b64\"YjY0LWQ=\",b64'YjY0LXM='>>";
        let parsed_result = GeneratedParser::parse(Rule::literal, binary_list);
        let binary_set_literal = parse_literal(parsed_result.unwrap().next().unwrap()).unwrap();
        assert_eq!(
            binary_set_literal,
            AttrVal::BS(Vec::from([
                Bytes::from_static(b"deadbeef"),
                Bytes::from_static(b"\xde\xad\xbe\xef"),
                Bytes::from_static(b"warp"),
                Bytes::from_static(b"no-\n\twarp"),
                Bytes::from_static(b"b64-d"),
                Bytes::from_static(b"b64-s"),
            ]))
        );

        // nested literal
        let literal_input = "{'id': '123456', 'year': 2023, 'info': {'creators':['Alice', 'Bob']}}";
        let parsed_result = GeneratedParser::parse(Rule::literal, literal_input)
            .unwrap()
            .next()
            .unwrap();
        let map_literal = parse_literal(parsed_result).unwrap();
        assert_eq!(
            map_literal,
            AttrVal::M(HashMap::from([
                ("id".to_owned(), AttrVal::S("123456".to_owned())),
                ("year".to_owned(), AttrVal::N("2023".to_owned())),
                (
                    "info".to_owned(),
                    AttrVal::M(HashMap::from([(
                        "creators".to_owned(),
                        AttrVal::L(Vec::from([
                            AttrVal::S("Alice".to_owned()),
                            AttrVal::S("Bob".to_owned()),
                        ]))
                    )]))
                ),
            ]))
        );
    }

    #[test]
    fn test_process_path() {
        let path_parsed = GeneratedParser::parse(Rule::path, "a0.a1[1][2].`a 2`[2].a三.`a``4`.a0")
            .unwrap()
            .next()
            .unwrap();
        let result = parse_path(path_parsed);
        let mut expected = Path::new();
        expected.add_attr("a0".to_owned());
        expected.add_attr("a1".to_owned());
        expected.add_index("1".to_owned());
        expected.add_index("2".to_owned());
        expected.add_attr("a 2".to_owned());
        expected.add_index("2".to_owned());
        expected.add_attr("a三".to_owned());
        expected.add_attr("a`4".to_owned());
        expected.add_attr("a0".to_owned());
        assert_eq!(result, expected);

        let mut parser = DyneinParser::new();
        let result = parser.process_path(result);
        assert_eq!(
            result,
            format!(
                "{}.{}[1][2].{}[2].{}.{}.{}",
                attr_name_ref(0),
                attr_name_ref(1),
                attr_name_ref(2),
                attr_name_ref(3),
                attr_name_ref(4),
                attr_name_ref(0)
            )
        );
        assert_eq!(
            parser.names,
            HashMap::from([
                (attr_name_ref(0), "a0".to_owned()),
                (attr_name_ref(1), "a1".to_owned()),
                (attr_name_ref(2), "a 2".to_owned()),
                (attr_name_ref(3), "a三".to_owned()),
                (attr_name_ref(4), "a`4".to_owned()),
            ])
        );
        assert_eq!(parser.values, HashMap::new());
    }

    #[test]
    fn test_process_literal() {
        macro_rules! do_test {
            ($in:expr, $expected:expr) => {{
                let mut parser = DyneinParser::new();
                let result = parser.process_literal($in).unwrap();
                assert_eq!(result, attr_val_ref(0));
                assert_eq!(parser.names, HashMap::new());
                assert_eq!(parser.values, HashMap::from([(attr_val_ref(0), $expected)]));
            }};
        }

        do_test!(
            AttrVal::N("123".to_owned()),
            AttributeValue::N("123".to_owned())
        );
        do_test!(
            AttrVal::S("string".to_owned()),
            AttributeValue::S("string".to_owned())
        );
        do_test!(AttrVal::Bool(true), AttributeValue::Bool(true));
        do_test!(AttrVal::Bool(false), AttributeValue::Bool(false));
        do_test!(AttrVal::Null(true), AttributeValue::Null(true));
        do_test!(
            AttrVal::B(Bytes::from_static(b"123")),
            AttributeValue::B(Blob::new(Bytes::from_static(b"123")))
        );
        do_test!(
            AttrVal::L(vec![AttrVal::N("123".to_owned())]),
            AttributeValue::L(vec![AttributeValue::N("123".to_owned())])
        );
        do_test!(
            AttrVal::M(HashMap::from([(
                "m".to_owned(),
                AttrVal::N("123".to_owned()),
            )])),
            AttributeValue::M(HashMap::from([(
                "m".to_owned(),
                AttributeValue::N("123".to_owned())
            )]))
        );
        do_test!(
            AttrVal::NS(vec!["123".to_owned()]),
            AttributeValue::Ns(vec!["123".to_owned()])
        );
        do_test!(
            AttrVal::SS(vec!["123".to_owned()]),
            AttributeValue::Ss(vec!["123".to_owned()])
        );
        do_test!(
            AttrVal::BS(vec![Bytes::from_static(b"123")]),
            AttributeValue::Bs(vec![Blob::new(Bytes::from_static(b"123"))])
        );
    }

    #[test]
    fn test_parse_sort_key() {
        let mut parser = DyneinParser::new();

        // test = for number types
        assert_eq!(
            parser
                .parse_sort_key_with_fallback(
                    "= 1",
                    &AttributeDefinition::new("id", AttributeType::N),
                )
                .unwrap(),
            ExpressionResult {
                exp: format!("{}={}", attr_name_ref(0), attr_val_ref(0)),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([(attr_val_ref(0), AttributeValue::N("1".to_owned()))]),
            }
        );

        // test == for string types
        parser.clear();
        assert_eq!(
            parser
                .parse_sort_key_with_fallback(
                    "=='1'",
                    &AttributeDefinition::new("id", AttributeType::S),
                )
                .unwrap(),
            ExpressionResult {
                exp: format!("{}={}", attr_name_ref(0), attr_val_ref(0)),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([(attr_val_ref(0), AttributeValue::S("1".to_owned()))]),
            }
        );

        // test > for string types
        parser.clear();
        assert_eq!(
            parser
                .parse_sort_key_with_fallback(
                    "> '1'",
                    &AttributeDefinition::new("id", AttributeType::S),
                )
                .unwrap(),
            ExpressionResult {
                exp: format!("{}>{}", attr_name_ref(0), attr_val_ref(0)),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([(attr_val_ref(0), AttributeValue::S("1".to_owned()))]),
            }
        );

        // test >= for number types
        parser.clear();
        assert_eq!(
            parser
                .parse_sort_key_with_fallback(
                    ">=1",
                    &AttributeDefinition::new("id", AttributeType::N),
                )
                .unwrap(),
            ExpressionResult {
                exp: format!("{}>={}", attr_name_ref(0), attr_val_ref(0)),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([(attr_val_ref(0), AttributeValue::N("1".to_owned()))]),
            }
        );

        // test < for string types
        parser.clear();
        assert_eq!(
            parser
                .parse_sort_key_with_fallback(
                    "<\"1 2\"",
                    &AttributeDefinition::new("id", AttributeType::S),
                )
                .unwrap(),
            ExpressionResult {
                exp: format!("{}<{}", attr_name_ref(0), attr_val_ref(0)),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([(attr_val_ref(0), AttributeValue::S("1 2".to_owned()))]),
            }
        );

        // test <= for number types
        parser.clear();
        assert_eq!(
            parser
                .parse_sort_key_with_fallback(
                    "<=-1e5",
                    &AttributeDefinition::new("id", AttributeType::N),
                )
                .unwrap(),
            ExpressionResult {
                exp: format!("{}<={}", attr_name_ref(0), attr_val_ref(0)),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([(attr_val_ref(0), AttributeValue::N("-1e5".to_owned()))]),
            }
        );

        // test between for binary types
        parser.clear();
        assert_eq!(
            parser
                .parse_sort_key_with_fallback(
                    "BETWEEN b'1' AND b'2'",
                    &AttributeDefinition::new("id", AttributeType::B),
                )
                .unwrap(),
            ExpressionResult {
                exp: format!(
                    "{} BETWEEN {} AND {}",
                    attr_name_ref(0),
                    attr_val_ref(0),
                    attr_val_ref(1)
                ),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([
                    (
                        attr_val_ref(0),
                        AttributeValue::B(Blob::new(Bytes::from_static(b"1")))
                    ),
                    (
                        attr_val_ref(1),
                        AttributeValue::B(Blob::new(Bytes::from_static(b"2")))
                    )
                ]),
            }
        );

        // test begins_with for string types
        parser.clear();
        assert_eq!(
            parser
                .parse_sort_key_with_fallback(
                    "begins_with 'id1234#e1234'",
                    &AttributeDefinition::new("id", AttributeType::S),
                )
                .unwrap(),
            ExpressionResult {
                exp: format!("begins_with({},{})", attr_name_ref(0), attr_val_ref(0),),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([(
                    attr_val_ref(0),
                    AttributeValue::S("id1234#e1234".to_owned())
                )]),
            }
        );

        // test non-strict input for string types
        let op_in = ["= 1", "==1", "> 1", ">=1", "<1.2", "<=-1e5"];
        let expected_op = ["=", "=", ">", ">=", "<", "<="];
        let expected_val = ["1", "1", "1", "1", "1.2", "-1e5"];
        for i in 0..op_in.len() {
            parser.clear();
            assert_eq!(
                parser
                    .parse_sort_key_with_fallback(
                        op_in[i],
                        &AttributeDefinition::new("id", AttributeType::S),
                    )
                    .unwrap(),
                ExpressionResult {
                    exp: format!("{}{}{}", attr_name_ref(0), expected_op[i], attr_val_ref(0)),
                    names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                    values: HashMap::from([(
                        attr_val_ref(0),
                        AttributeValue::S(expected_val[i].to_owned())
                    )]),
                }
            );
            parser.clear();
            assert_eq!(
                parser.parse_sort_key_without_fallback(
                    op_in[i],
                    &AttributeDefinition::new("id", AttributeType::S),
                ),
                Err(ParseError::InvalidTypesWithSuggest(
                    InvalidTypesWithSuggestError {
                        expected_type: AttributeType::S,
                        actual_type: AttributeType::N,
                        suggest: format!("{} \"{}\"", expected_op[i], expected_val[i]),
                    }
                ))
            );
        }

        // test invalid type input
        parser.clear();
        assert_eq!(
            parser.parse_sort_key_without_fallback(
                r#"= "1*3""#,
                &AttributeDefinition::new("id", AttributeType::N),
            ),
            Err(ParseError::InvalidTypes(InvalidTypesError {
                expected_type: AttributeType::N,
                actual_type: AttributeType::S,
            }))
        );

        // test between in non-strict for string types
        parser.clear();
        assert_eq!(
            parser
                .parse_sort_key_with_fallback(
                    "BETWEEN 1 AND 2",
                    &AttributeDefinition::new("id", AttributeType::S),
                )
                .unwrap(),
            ExpressionResult {
                exp: format!(
                    "{} BETWEEN {} AND {}",
                    attr_name_ref(0),
                    attr_val_ref(0),
                    attr_val_ref(1)
                ),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([
                    (attr_val_ref(0), AttributeValue::S("1".to_owned())),
                    (attr_val_ref(1), AttributeValue::S("2".to_owned()))
                ]),
            }
        );

        // test begins_with in non-strict for string types
        parser.clear();
        assert_eq!(
            parser
                .parse_sort_key_with_fallback(
                    "begins_with id12#i-12@i-12/i-12",
                    &AttributeDefinition::new("id", AttributeType::S),
                )
                .unwrap(),
            ExpressionResult {
                exp: format!("begins_with({},{})", attr_name_ref(0), attr_val_ref(0),),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([(
                    attr_val_ref(0),
                    AttributeValue::S("id12#i-12@i-12/i-12".to_owned())
                )]),
            }
        );

        // test bare for string types
        parser.clear();
        assert_eq!(
            parser
                .parse_sort_key_with_fallback(
                    "123",
                    &AttributeDefinition::new("id", AttributeType::S),
                )
                .unwrap(),
            ExpressionResult {
                exp: format!("{}={}", attr_name_ref(0), attr_val_ref(0),),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([(attr_val_ref(0), AttributeValue::S("123".to_owned()))]),
            }
        );

        // test bare for number types
        parser.clear();
        assert_eq!(
            parser
                .parse_sort_key_with_fallback(
                    "123",
                    &AttributeDefinition::new("id", AttributeType::N),
                )
                .unwrap(),
            ExpressionResult {
                exp: format!("{}={}", attr_name_ref(0), attr_val_ref(0),),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([(attr_val_ref(0), AttributeValue::N("123".to_owned()))]),
            }
        );
    }

    #[test]
    fn test_parse_dynein_format() {
        let parser = DyneinParser::new();
        assert_eq!(
            parser
                .parse_dynein_format(
                    None,
                    r#"{
                           "k0": null,
                           "k1": [1, 2, 3, "str"],
                           "k2": "str",
                           "k3": {
                             "l0": <<1, 2>>,
                             "l1": <<'str1', "str2">>,
                             "l2": true
                           },
                           "k4": b"\x20",
                           "k5": <<b'This', b"bin", b64"ZmlsZQ==">>
                         }"#,
                )
                .unwrap(),
            HashMap::from([
                ("k0".to_owned(), AttributeValue::Null(true)),
                (
                    "k1".to_owned(),
                    AttributeValue::L(vec![
                        AttributeValue::N("1".to_owned()),
                        AttributeValue::N("2".to_owned()),
                        AttributeValue::N("3".to_owned()),
                        AttributeValue::S("str".to_owned()),
                    ])
                ),
                ("k2".to_owned(), AttributeValue::S("str".to_owned())),
                (
                    "k3".to_owned(),
                    AttributeValue::M(HashMap::from([
                        (
                            "l0".to_owned(),
                            AttributeValue::Ns(vec!["1".to_owned(), "2".to_owned()])
                        ),
                        (
                            "l1".to_owned(),
                            AttributeValue::Ss(vec!["str1".to_owned(), "str2".to_owned()])
                        ),
                        ("l2".to_owned(), AttributeValue::Bool(true))
                    ]))
                ),
                (
                    "k4".to_owned(),
                    AttributeValue::B(Blob::new(Bytes::from_static(b"\x20")))
                ),
                (
                    "k5".to_owned(),
                    AttributeValue::Bs(vec![
                        Blob::new(Bytes::from_static(b"This")),
                        Blob::new(Bytes::from_static(b"bin")),
                        Blob::new(Bytes::from_static(b"file"))
                    ])
                )
            ])
        )
    }

    #[test]
    fn test_parse_set_action() {
        let mut parser = DyneinParser::new();
        assert_eq!(
            parser.parse_set_action("id = \"string\"").unwrap(),
            ExpressionResult {
                exp: format!("{}={}", attr_name_ref(0), attr_val_ref(0)),
                names: HashMap::from([(attr_name_ref(0), "id".to_owned())]),
                values: HashMap::from([(attr_val_ref(0), AttributeValue::S("string".to_owned()))]),
            }
        );
    }

    #[test]
    fn test_remove_action() {
        let mut parser = DyneinParser::new();
        assert_eq!(
            parser
                .parse_remove_action("p0, p1[0], p2.p3[1].p0")
                .unwrap(),
            ExpressionResult {
                exp: format!(
                    "{},{}[0],{}.{}[1].{}",
                    attr_name_ref(0),
                    attr_name_ref(1),
                    attr_name_ref(2),
                    attr_name_ref(3),
                    attr_name_ref(0)
                ),
                names: HashMap::from([
                    (attr_name_ref(0), "p0".to_owned()),
                    (attr_name_ref(1), "p1".to_owned()),
                    (attr_name_ref(2), "p2".to_owned()),
                    (attr_name_ref(3), "p3".to_owned()),
                ]),
                values: HashMap::new(),
            }
        );
    }

    #[test]
    fn test_set_and_remove_action() {
        let mut parser = DyneinParser::new();
        let names = HashMap::from([(attr_name_ref(0), "p0".to_owned())]);
        let values = HashMap::from([(attr_val_ref(0), AttributeValue::S("string".to_owned()))]);
        assert_eq!(
            parser.parse_set_action("p0 = \"string\"").unwrap(),
            ExpressionResult {
                exp: format!("{}={}", attr_name_ref(0), attr_val_ref(0)),
                names: names.to_owned(),
                values: values.to_owned(),
            }
        );
        assert_eq!(
            parser.parse_remove_action("p0").unwrap(),
            ExpressionResult {
                exp: attr_name_ref(0),
                names,
                values,
            }
        );
    }
}
