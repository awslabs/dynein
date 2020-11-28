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
use atty;
use log::debug;
use std::error::Error;
use std::io::{stdout, BufRead, Stdin, StdinLock, Write};

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
                let args = line.split(' ');
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
