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
use bytes::Bytes;
use itertools::Itertools;
use pest::iterators::Pair;
use rusoto_dynamodb::AttributeValue;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::iter::Enumerate;
use std::str::Chars;

#[derive(Parser)]
#[grammar = "expression.pest"]
struct GeneratedParser;

type SetAction = Vec<AtomicSet>;
type RemoveAction = Vec<AtomicRemove>;

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

/// The error context of a parsing error
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ParseError {
    ParsingError(Box<pest::error::Error<Rule>>),
    UnexpectedEndOfSequence(EscapeCharUnexpectedEndOfSequenceError),
    InvalidUnicodeChar(InvalidUnicodeCharError),
    InvalidEscapeChar(EscapeCharError),
    InvalidEscapeByte(EscapeByteError),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            ParseError::ParsingError(ref e) => write!(f, "{}", e),
            ParseError::UnexpectedEndOfSequence(ref e) => write!(f, "{}", e),
            ParseError::InvalidUnicodeChar(ref e) => write!(f, "{}", e),
            ParseError::InvalidEscapeChar(ref e) => write!(f, "{}", e),
            ParseError::InvalidEscapeByte(ref e) => write!(f, "{}", e),
        }
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
    fn convert_attribute_value(self) -> AttributeValue {
        match self {
            AttrVal::N(number) => AttributeValue {
                n: Some(number),
                ..Default::default()
            },
            AttrVal::S(str) => AttributeValue {
                s: Some(str),
                ..Default::default()
            },
            AttrVal::Bool(boolean) => AttributeValue {
                bool: Some(boolean),
                ..Default::default()
            },
            AttrVal::Null(isnull) => AttributeValue {
                null: Some(isnull),
                ..Default::default()
            },
            AttrVal::B(binary) => AttributeValue {
                b: Some(binary),
                ..Default::default()
            },
            AttrVal::L(list) => AttributeValue {
                l: Some(
                    list.into_iter()
                        .map(|x| x.convert_attribute_value())
                        .collect(),
                ),
                ..Default::default()
            },
            AttrVal::M(map) => AttributeValue {
                m: Some(
                    map.into_iter()
                        .map(|(key, val)| (key, val.convert_attribute_value()))
                        .collect(),
                ),
                ..Default::default()
            },
            AttrVal::NS(list) => AttributeValue {
                ns: Some(list),
                ..Default::default()
            },
            AttrVal::SS(list) => AttributeValue {
                ss: Some(list),
                ..Default::default()
            },
            AttrVal::BS(list) => AttributeValue {
                bs: Some(list),
                ..Default::default()
            },
        }
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

    pub fn parse_dynein_format(
        &self,
        initial_item: Option<HashMap<String, AttributeValue>>,
        exp: &str,
    ) -> Result<HashMap<String, AttributeValue>, ParseError> {
        let result = GeneratedParser::parse(Rule::map_literal, exp);
        match result {
            Ok(mut pair) => {
                let item = parse_literal(pair.next().unwrap())?.convert_attribute_value();
                // content must be map literal
                let mut image = match initial_item {
                    Some(init_item) => init_item,
                    None => HashMap::new(),
                };
                image.extend(item.m.unwrap());
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
            "<<b'deadbeef',b'\\xde\\xad\\xbe\\xef',b\"wa\\\n\trp\",b\"no-\n\twarp\">>";
        let parsed_result = GeneratedParser::parse(Rule::literal, binary_list);
        let binary_set_literal = parse_literal(parsed_result.unwrap().next().unwrap()).unwrap();
        assert_eq!(
            binary_set_literal,
            AttrVal::BS(Vec::from([
                Bytes::from_static(b"deadbeef"),
                Bytes::from_static(b"\xde\xad\xbe\xef"),
                Bytes::from_static(b"warp"),
                Bytes::from_static(b"no-\n\twarp"),
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
            AttributeValue {
                n: Some("123".to_owned()),
                ..Default::default()
            }
        );
        do_test!(
            AttrVal::S("string".to_owned()),
            AttributeValue {
                s: Some("string".to_owned()),
                ..Default::default()
            }
        );
        do_test!(
            AttrVal::Bool(true),
            AttributeValue {
                bool: Some(true),
                ..Default::default()
            }
        );
        do_test!(
            AttrVal::Bool(false),
            AttributeValue {
                bool: Some(false),
                ..Default::default()
            }
        );
        do_test!(
            AttrVal::Null(true),
            AttributeValue {
                null: Some(true),
                ..Default::default()
            }
        );
        do_test!(
            AttrVal::B(Bytes::from_static(b"123")),
            AttributeValue {
                b: Some(Bytes::from_static(b"123")),
                ..Default::default()
            }
        );
        do_test!(
            AttrVal::L(vec![AttrVal::N("123".to_owned())]),
            AttributeValue {
                l: Some(vec![AttributeValue {
                    n: Some("123".to_owned()),
                    ..Default::default()
                }]),
                ..Default::default()
            }
        );
        do_test!(
            AttrVal::M(HashMap::from([(
                "m".to_owned(),
                AttrVal::N("123".to_owned()),
            )])),
            AttributeValue {
                m: Some(HashMap::from([(
                    "m".to_owned(),
                    AttributeValue {
                        n: Some("123".to_owned()),
                        ..Default::default()
                    }
                )])),
                ..Default::default()
            }
        );
        do_test!(
            AttrVal::NS(vec!["123".to_owned()]),
            AttributeValue {
                ns: Some(vec!["123".to_owned()]),
                ..Default::default()
            }
        );
        do_test!(
            AttrVal::SS(vec!["123".to_owned()]),
            AttributeValue {
                ss: Some(vec!["123".to_owned()]),
                ..Default::default()
            }
        );
        do_test!(
            AttrVal::BS(vec![Bytes::from_static(b"123")]),
            AttributeValue {
                bs: Some(vec![Bytes::from_static(b"123")]),
                ..Default::default()
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
                           "k5": <<b'This', b"bin">>
                         }"#
                )
                .unwrap(),
            HashMap::from([
                (
                    "k0".to_owned(),
                    AttributeValue {
                        null: Some(true),
                        ..Default::default()
                    }
                ),
                (
                    "k1".to_owned(),
                    AttributeValue {
                        l: Some(Vec::from([
                            AttributeValue {
                                n: Some("1".to_owned()),
                                ..Default::default()
                            },
                            AttributeValue {
                                n: Some("2".to_owned()),
                                ..Default::default()
                            },
                            AttributeValue {
                                n: Some("3".to_owned()),
                                ..Default::default()
                            },
                            AttributeValue {
                                s: Some("str".to_owned()),
                                ..Default::default()
                            },
                        ])),
                        ..Default::default()
                    }
                ),
                (
                    "k2".to_owned(),
                    AttributeValue {
                        s: Some("str".to_owned()),
                        ..Default::default()
                    }
                ),
                (
                    "k3".to_owned(),
                    AttributeValue {
                        m: Some(HashMap::from([
                            (
                                "l0".to_owned(),
                                AttributeValue {
                                    ns: Some(vec!["1".to_owned(), "2".to_owned()]),
                                    ..Default::default()
                                }
                            ),
                            (
                                "l1".to_owned(),
                                AttributeValue {
                                    ss: Some(vec!["str1".to_owned(), "str2".to_owned()]),
                                    ..Default::default()
                                }
                            ),
                            (
                                "l2".to_owned(),
                                AttributeValue {
                                    bool: Some(true),
                                    ..Default::default()
                                }
                            )
                        ])),
                        ..Default::default()
                    }
                ),
                (
                    "k4".to_owned(),
                    AttributeValue {
                        b: Some(Bytes::from_static(b"\x20")),
                        ..Default::default()
                    }
                ),
                (
                    "k5".to_owned(),
                    AttributeValue {
                        bs: Some(vec!(
                            Bytes::from_static(b"This"),
                            Bytes::from_static(b"bin"),
                        )),
                        ..Default::default()
                    }
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
                values: HashMap::from([(
                    attr_val_ref(0),
                    AttributeValue {
                        s: Some("string".to_owned()),
                        ..Default::default()
                    }
                )]),
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
        let values = HashMap::from([(
            attr_val_ref(0),
            AttributeValue {
                s: Some("string".to_owned()),
                ..Default::default()
            },
        )]);
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
