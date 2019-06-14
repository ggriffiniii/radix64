//! This module is included whenever running on an architecture that doesn't have a specialized module.

use crate::decode::block::{IntoBlockDecoder, ScalarBlockDecoder};
use crate::{Crypt, Std, StdNoPad, UrlSafe, UrlSafeNoPad};

macro_rules! impl_into_block_decoder {
    ($( $cfg:ident ),+) => {$(
        impl IntoBlockDecoder for $cfg {
            type BlockDecoder = ScalarBlockDecoder<Self>;

            #[inline]
            fn into_block_decoder(self) -> Self::BlockDecoder {
                ScalarBlockDecoder::new(self)
            }
        }
    )+}
}
impl_into_block_decoder!(Std, StdNoPad, UrlSafe, UrlSafeNoPad, Crypt);
