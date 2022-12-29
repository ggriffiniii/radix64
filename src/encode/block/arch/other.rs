//! This module is included whenever running on an architecture that doesn't have a specialized module.

use crate::encode::block::{IntoBlockEncoder, ScalarBlockEncoder};
use crate::{Crypt, Fast, Std, StdNoPad, UrlSafe, UrlSafeNoPad};

macro_rules! impl_into_block_encoder {
    ($( $cfg:ident ),+) => {$(
        impl IntoBlockEncoder for $cfg {
            type BlockEncoder = ScalarBlockEncoder<Self>;

            #[inline]
            fn into_block_encoder(self) -> Self::BlockEncoder {
                ScalarBlockEncoder::new(self)
            }
        }
    )+}
}
impl_into_block_encoder!(Std, StdNoPad, UrlSafe, UrlSafeNoPad, Crypt, Fast);
