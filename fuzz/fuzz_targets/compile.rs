#![no_main]

use libfuzzer_sys::fuzz_target;
use rasn_compiler::{OutputMode, RasnCompiler};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        RasnCompiler::default()
            .set_output_mode(OutputMode::NoOutput)
            .add_asn_literal(s)
            .compile()
            .ok();
    }
});
