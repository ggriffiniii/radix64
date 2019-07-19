#![no_main]
use libfuzzer_sys::fuzz_target;
use radix64::STD;

// Attempt to decode random input. If successful, the decoded value should
// encode to exactly the input. Any deviation shows an inconsistent
// roundtripping.
fuzz_target!(|data: &[u8]| {
    if let Ok(decoded) = STD.decode(data) {
        assert_eq!(STD.encode(&decoded).as_bytes(), data);
    }
});
