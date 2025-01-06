#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate dylib;

use dylib::parser::*;

fuzz_target!(|input: &str| {
    let mut parser = DyneinParser::new();
    let _ = parser.parse_remove_action(input);
});
