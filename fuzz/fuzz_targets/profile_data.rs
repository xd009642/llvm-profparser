#![no_main]
use libfuzzer_sys::fuzz_target;
use llvm_profparser::parse_bytes;

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
    let _ = parse_bytes(data);
});
