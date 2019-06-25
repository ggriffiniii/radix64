/// Verify that we can encode and decode data from a separate implementation
/// (base64 crate).
use proptest::prelude::{any, proptest};

macro_rules! test_cfg {
    ($name:ident, radix_cfg = $radix_cfg:ident, base64_cfg = $base64_cfg:ident) => {
        mod $name {
            use super::*;
            const BASE64_CFG: base64::Config = base64::$base64_cfg;
            use radix64::$radix_cfg as RADIX_CFG;

            proptest! {
                #[test]
                fn encode_identically(input in any::<Vec<u8>>()) {
                    let base64_encoded = base64::encode_config(&input, BASE64_CFG);
                    let radix64_encoded = RADIX_CFG.encode(&input);
                    assert_eq!(base64_encoded, radix64_encoded);
                }

                #[test]
                fn radix64_can_decode_from_base64(input in any::<Vec<u8>>()) {
                    let encoded = base64::encode_config(&input, BASE64_CFG);
                    let decoded = RADIX_CFG.decode(&encoded).expect("failed to decode");
                    assert_eq!(input, decoded);
                }

                #[test]
                fn base64_can_decode_from_radix64(input in any::<Vec<u8>>()) {
                    let encoded = RADIX_CFG.encode(&input);
                    let decoded = base64::decode_config(&encoded, BASE64_CFG).expect("failed to decode");
                    assert_eq!(input, decoded);
                }
            }
        }
    }
}

test_cfg!(standard, radix_cfg = STD, base64_cfg = STANDARD);
test_cfg!(
    standard_no_pad,
    radix_cfg = STD_NO_PAD,
    base64_cfg = STANDARD_NO_PAD
);
test_cfg!(url_safe, radix_cfg = URL_SAFE, base64_cfg = URL_SAFE);
test_cfg!(
    url_safe_no_pad,
    radix_cfg = URL_SAFE_NO_PAD,
    base64_cfg = URL_SAFE_NO_PAD
);
test_cfg!(crypt, radix_cfg = CRYPT, base64_cfg = CRYPT);
