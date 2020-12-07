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

use crate::cmd;
use log::debug;
use std::io::{stdout, BufRead, Stdin, StdinLock, Write};
use std::{error::Error, io};

/* =================================================
struct / enum / const
================================================= */

pub enum ShellInput {
    Builtin(BuiltinCommands),
    Command(cmd::Sub),
    Eof,
    ParseError(Box<dyn Error>),
}

pub enum BuiltinCommands {
    Exit,
}

pub struct ShellReader<'a> {
    line: String,
    input: StdinLock<'a>,
}

/* =================================================
inherent methods
================================================= */

impl<'a> ShellReader<'a> {
    pub fn new(input: &'a Stdin) -> Self {
        Self {
            line: String::new(),
            input: input.lock(),
        }
    }

    pub fn read_line(&mut self) -> Result<ShellInput, Box<dyn Error>> {
        if atty::is(atty::Stream::Stdin) {
            print!("> ");
            stdout().flush().expect("failed to flush output");
        }
        self.line.clear();
        match self.input.read_line(&mut self.line) {
            Ok(0) => {
                return Ok(ShellInput::Eof);
            }
            Ok(_) => (),
            Err(e) => return Err(Box::new(e)),
        }

        let line = self.line.trim_end();

        debug!("Line read: {:?}", line);

        match line {
            // build-in shell command(s)
            "exit" => Ok(ShellInput::Builtin(BuiltinCommands::Exit)),
            // dy commands
            line => {
                // TODO: better handling of whitespaces
                let args = match parse(line) {
                    Ok(args) => args,
                    Err(e) => {
                        eprintln!("Error while parsing input: {}", e);
                        return Ok(ShellInput::ParseError(e));
                    }
                };
                debug!("Args: {:?}", args);
                let child = match cmd::parse_args(args) {
                    Ok(child) => child,
                    Err(e) => {
                        eprintln!("Invalid argument: {}", e);
                        return Ok(ShellInput::ParseError(e));
                    }
                };
                Ok(ShellInput::Command(child))
            }
        }
    }
}

fn parse(line: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut ret = vec![];
    let mut input = line.trim_start();
    while 0 < input.len() {
        if input.starts_with("'") {
            let mut tok = String::new();
            let mut iter = input.chars();
            // discard first "'"
            iter.next();
            loop {
                match iter.next() {
                    Some('\'') => break,
                    Some('\\') => match iter.next() {
                        Some(c) => tok.push(c),
                        None => {
                            return Err(Box::new(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "escape('\\') is incomplete",
                            )));
                        }
                    },
                    Some(c) => tok.push(c),
                    None => {
                        return Err(Box::new(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "quote isn't closed",
                        )));
                    }
                }
            }
            input = iter.as_str().trim_start();
            ret.push(tok);
        } else {
            let pos = input.find(' ').unwrap_or_else(|| input.len());
            let (tok, rest) = input.split_at(pos);
            ret.push(tok.into());
            input = rest.trim_start();
        }
    }
    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ok() {
        let input = r#"query --sort-key '= 12' 'pk\\is\'escaped'"#;
        let result = parse(input);
        assert_eq!(
            result.unwrap(),
            vec!["query", "--sort-key", "= 12", r#"pk\is'escaped"#]
        )
    }

    #[test]
    fn test_parse_ng() {
        let input = r#"quote is 'broken"#;
        let result = parse(input);
        assert!(result.is_err());

        let input = r#"escape is 'broken\"#;
        let result = parse(input);
        assert!(result.is_err());

        let input = r#"quote is 'broken by escape\'"#;
        let result = parse(input);
        assert!(result.is_err());
    }
}
