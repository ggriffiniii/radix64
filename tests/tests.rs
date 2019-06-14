use radix64::{CRYPT, STD, STD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};

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
                use crate::{$cfg, custom_configs};
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
                }
            })+
        }
    }
}

tests_for_configs!(STD, STD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD, CRYPT);
