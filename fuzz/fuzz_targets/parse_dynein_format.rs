#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate dylib;

use dylib::parser::*;

fuzz_target!(|input: &str| {
    let parser = DyneinParser::new();
    let _ = parser.parse_dynein_format(None, input);
});
