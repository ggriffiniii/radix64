#![no_main]
use libfuzzer_sys::fuzz_target;
use radix64::STD;

// Encode random input, and ensure that decoding the result matches the input.
fuzz_target!(|data: &[u8]| {
    let encoded = STD.encode(data);
    let decoded = STD.decode(encoded.as_bytes()).expect("decode failed");
    assert_eq!(data, decoded.as_slice());
});
