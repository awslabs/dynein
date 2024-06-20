#![no_main]

use libfuzzer_sys::fuzz_target;

extern crate dylib;

use arbitrary::Arbitrary;
use dylib::parser::*;

#[derive(Arbitrary, Debug)]
struct Input {
    text: String,
    attribute_definition: AttributeDefinition,
}

fuzz_target!(|input: Input| {
    let mut parser = DyneinParser::new();
    let _ = parser.parse_sort_key_with_suggest(&input.text, &input.attribute_definition);
});
