use crate::cmd;
use atty;
use log::debug;
use std::error::Error;
use std::io::{stdout, BufRead, Stdin, StdinLock, Write};

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

        match self.input.read_line(&mut self.line) {
            Ok(0) => {
                // // append newline after '> '
                // println!("");
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
