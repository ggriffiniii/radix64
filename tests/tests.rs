use proptest::prelude::Strategy;
use radix64::{CRYPT, STD, STD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use std::io::Read;

// Create a custom config that should match each of the builtin configs.
mod custom_configs {
    use radix64::{ConfigBuilder, CustomConfig};
    lazy_static::lazy_static! {

        pub static ref STD: CustomConfig = ConfigBuilder::with_alphabet(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
        )
        .with_padding(b'=')
        .build()
        .expect("failed to build custom base64 config");

        pub static ref STD_NO_PAD: CustomConfig = ConfigBuilder::with_alphabet(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
        )
        .no_padding()
        .build()
        .expect("failed to build custom base64 config");

        pub static ref URL_SAFE: CustomConfig = ConfigBuilder::with_alphabet(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
        )
        .with_padding(b'=')
        .build()
        .expect("failed to build custom base64 config");

        pub static ref URL_SAFE_NO_PAD: CustomConfig = ConfigBuilder::with_alphabet(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
        )
        .no_padding()
        .build()
        .expect("failed to build custom base64 config");

        pub static ref CRYPT: CustomConfig = ConfigBuilder::with_alphabet(
            "./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
        )
        .no_padding()
        .build()
        .expect("failed to build custom base64 config");

    }
}

macro_rules! tests_for_configs {
    ($( $cfg:ident ),+) => {
        #[cfg(test)]
        mod property_tests {
            $(
            #[allow(non_snake_case)]
            mod $cfg {
                use proptest::prelude::{any, proptest};
                use proptest::collection::vec;
                use crate::{$cfg, custom_configs, read_to_end_using_varying_buffer_sizes, vec_and_buffer_sizes};
                proptest! {
                    #[test]
                    fn roundtrip(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        let decoded = $cfg.decode(&encoded).expect("decode failed");
                        assert_eq!(input, decoded);
                    }

                    #[test]
                    fn input_encodes_to_expected_length(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        assert_eq!(encoded.len(), $cfg.encoded_output_len(input.len()));
                    }

                    #[test]
                    fn custom_can_be_decoded_by_builtin(input in any::<Vec<u8>>()) {
                        let encoded = custom_configs::$cfg.encode(&input);
                        let decoded = $cfg.decode(&encoded).expect("decode failed");
                        assert_eq!(input, decoded);
                    }

                    #[test]
                    fn custom_can_decode_builtin(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        let decoded = custom_configs::$cfg.decode(&encoded).expect("decode failed");
                        assert_eq!(input, decoded);
                    }

                    #[test]
                    fn encode_with_buffer_matches_encode(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        let mut buf = Vec::new();
                        let encoded_buf = $cfg.encode_with_buffer(&input, &mut buf);
                        assert_eq!(encoded, encoded_buf);
                    }

                    #[test]
                    fn encode_slice_matches_encode(input in any::<Vec<u8>>()) {
                        let mut encoded_vec = vec![0; $cfg.encoded_output_len(input.len())];
                        $cfg.encode_slice(&input, encoded_vec.as_mut_slice());
                        let encoded_string = $cfg.encode(&input);
                        assert_eq!(encoded_vec.as_slice(), encoded_string.as_bytes())
                    }

                    #[test]
                    fn decode_with_buffer_matches_decode(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        let mut buf = Vec::new();
                        let decoded_buf = $cfg.decode_with_buffer(&encoded, &mut buf).expect("decode failed");
                        let decoded_vec = $cfg.decode(&encoded).expect("decode failed");
                        assert_eq!(decoded_buf, decoded_vec.as_slice());
                    }

                    #[test]
                    fn decode_slice_matches_decode(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        let mut decoded_slice = vec![0; input.len()];
                        let decoded_slice = $cfg.decode_slice(&encoded, decoded_slice.as_mut_slice()).expect("decode failed");
                        let decoded_vec = $cfg.decode(&encoded).expect("decode failed");
                        assert_eq!(decoded_slice, decoded_vec.as_slice());
                    }

                    #[test]
                    fn encode_slice_always_panics_or_returns_ascii(input in any::<Vec<u8>>(), output_len in 0..1000usize) {
                        let res = std::panic::catch_unwind(|| {
                            let mut encoded = vec![255; output_len];
                            $cfg.encode_slice(&input, encoded.as_mut_slice());
                            encoded
                        });
                        match res {
                            Ok(encoded) => assert!(&encoded[..$cfg.encoded_output_len(input.len())].iter().all(u8::is_ascii)),
                            Err(_) => {}, // Panic is expected when output len is too short.
                        }
                    }

                    // encode_with_buffer does an unchecked conversion from a
                    // slice of bytes to a &str. This is just a sanity test to
                    // verify the string returned is valid UTF-8.
                    #[test]
                    fn encode_buffer_returns_valid_str(input in any::<Vec<u8>>()) {
                        let mut buffer = Vec::new();
                        let encoded = $cfg.encode_with_buffer(&input, &mut buffer);
                        std::str::from_utf8(encoded.as_bytes()).expect("invalid UTF-8 returned from encode_with_buffer");
                    }

                    // encode does an unchecked conversion from a slice of bytes
                    // to a &str. This is just a sanity test to verify the
                    // string returned is valid UTF-8.
                    #[test]
                    fn encode_returns_valid_str(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        std::str::from_utf8(encoded.as_bytes()).expect("invalid UTF-8 returned from encode_with_buffer");
                    }

                    // read a vector from a DecodeReader, ensuring that it matches the encoded input.
                    // The reads are done with varying buffer sizes to try and
                    // catch edge cases around chunking.
                    #[test]
                    fn decode_reader_roundtrip((input, buffer_sizes) in vec_and_buffer_sizes()) {
                        use radix64::io::DecodeReader;
                        use std::io::Cursor;
                        let encoded = $cfg.encode(&input);
                        let reader = DecodeReader::new($cfg, Cursor::new(encoded));
                        let decoded = read_to_end_using_varying_buffer_sizes(reader, buffer_sizes.iter().cloned()).expect("failed to read to the end of input");
                        assert_eq!(input, decoded);
                    }

                    // ensure that padding in the middle of the input stream is not silently accepted.
                    // The buffer sizes to use are randomly chosen between 1 and 5.
                    #[test]
                    fn decode_reader_(buffer_sizes in vec(1 as usize ..5, 1..3)) {
                        use radix64::io::DecodeReader;
                        use std::io::Cursor;
                        let reader = DecodeReader::new($cfg, Cursor::new("AA==BBQQ"));
                        match read_to_end_using_varying_buffer_sizes(reader, buffer_sizes.iter().cloned()) {
                            Ok(_) => panic!("incorrect padding accepted"),
                            Err(_) => {}, // this is good
                        }
                    }
                }
            })+
        }
    }
}

// define a proptest strategy that returns a random buffer, and an additional
// vector that contains usize values of buffer sizes to read from the buffer
// with. The buffer sizes are kept significantly smaller than the size of the
// random buffer to try and catch edge cases around chunked reads.
fn vec_and_buffer_sizes() -> impl Strategy<Value = (Vec<u8>, Vec<usize>)> {
    use proptest::collection::vec;
    use proptest::prelude::{any, Just};
    vec(any::<u8>(), 1..100).prop_flat_map(|v| {
        let len = v.len();
        let max_buffer_size = std::cmp::max(2, len / 3);
        (Just(v), vec(1..max_buffer_size, 1..5))
    })
}

// read to the end of the provided reader collecting the results into a vector.
// The read calls to the reader are done in buffer sizes according to the passed
// in iterator.
// For example if the passed in iterator returns [1, 10, 5]. It will first issue
// a read of 1 byte in length, then 10 bytes, then 5 bytes, then 1 byte, rinse
// and repeat until EOF is reached.
fn read_to_end_using_varying_buffer_sizes<R, I>(
    mut rdr: R,
    buffer_sizes: I,
) -> std::io::Result<Vec<u8>>
where
    R: Read,
    I: Iterator<Item = usize> + Clone,
{
    let mut v = Vec::new();
    for buffer_size in buffer_sizes.cycle() {
        let prev_len = v.len();
        v.resize(prev_len + buffer_size, 0);
        let n = rdr.read(&mut v[prev_len..])?;
        v.truncate(prev_len + n);
        if n == 0 {
            return Ok(v);
        }
    }
    unreachable!();
}

tests_for_configs!(STD, STD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD, CRYPT);
