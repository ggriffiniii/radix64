//! This module is only included on x86 and x86_64.
use crate::Config;
use crate::decode::block::{BlockDecoder, IntoBlockDecoder, ScalarBlockDecoder};
use crate::decode::DecodeError;
use crate::{Std, StdNoPad, UrlSafe, UrlSafeNoPad, Crypt};
#[derive(Debug, Clone, Copy)]
pub struct Decoder<C>(C);

impl<C> BlockDecoder for Decoder<C> where C: Config + avx2::Translate256i {
    #[inline]
    fn decode_blocks(
        self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<(usize, usize), DecodeError> {
        if let Ok(decoder) = avx2::Decoder::new(self.0) {
            Ok(decoder.decode_blocks(input, output))
        } else {
            ScalarBlockDecoder::new(self.0).decode_blocks(input, output)
        }
    }
}

macro_rules! define_into_block_decoder {
    ($( $cfg:ident ),+) => {$(
        impl IntoBlockDecoder for $cfg {
            type BlockDecoder = Decoder<Self>;

            #[inline]
            fn into_block_decoder(self) -> Self::BlockDecoder {
                Decoder(self)
            }
        }
    )+}
}
define_into_block_decoder!(Std,StdNoPad,UrlSafe,UrlSafeNoPad,Crypt);

mod avx2 {
     #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;
    use crate::{Std, StdNoPad, UrlSafe, UrlSafeNoPad, Crypt};

    pub trait Translate256i: Copy {
        unsafe fn translate_m256i(input: __m256i) -> Result<__m256i, ()>;
    }

    #[derive(Debug,Clone,Copy)]
    pub(crate) struct Decoder<C>(C);

    impl<C> Decoder<C> where C: Translate256i {
        #[inline]
        pub(crate) fn new(config: C) -> Result<Self, ()> {
            if is_x86_feature_detected!("avx2") {
                Ok(Decoder(config))
            } else {
                Err(())
            }
        }

        pub(crate) fn decode_blocks(
            self,
            input: &[u8],
            output: &mut [u8],
        ) -> (usize, usize) {
            // The unsafe block is required because _encode_blocks relies on AVX2
            // intrinsics. This is safe because Encoder::new() ensures that an
            // encoder is only successfully created when the CPU supports AVX2.
            unsafe { self._decode_blocks(input, output) }
        }

        #[target_feature(enable = "avx2")]
        unsafe fn _decode_blocks(
            self,
            input: &[u8],
            output: &mut [u8],
        ) -> (usize, usize) {
            let mut iter = BlockIter::new(input, output);
            for (input_block, output_block) in iter.by_ref() {
                #[allow(clippy::cast_ptr_alignment)]
                let mut data = _mm256_loadu_si256(input_block.as_ptr() as *const __m256i);
                data = match self.decode_block(data) {
                    Ok(data) => data,
                    Err(_) => {
                        // Move back to the beginning of the chunk that failed
                        // and return the remaining slice to the non-optimized
                        // decoder for better error reporting.
                        iter.next_back();
                        return iter.remaining();
                    }
                };
                #[allow(clippy::cast_ptr_alignment)]
                _mm256_storeu_si256(output_block.as_mut_ptr() as *mut __m256i, data);
            }
            iter.remaining()
        }

        #[target_feature(enable = "avx2")]
        unsafe fn decode_block(self, mut input: __m256i) -> Result<__m256i, ()> {
            input = C::translate_m256i(input)?;
            input = _mm256_maddubs_epi16(input, _mm256_set1_epi32(0x0140_0140));
            input = _mm256_madd_epi16(input, _mm256_set1_epi32(0x0001_1000));
            input = _mm256_shuffle_epi8(
                input,
                #[cfg_attr(rustfmt, rustfmt_skip)]
                _mm256_setr_epi8(
                    2, 1, 0,
                    6, 5, 4,
                    10, 9, 8,
                    14, 13, 12,
                    -1, -1, -1, -1,

                    2, 1, 0,
                    6, 5, 4,
                    10, 9, 8,
                    14, 13, 12,
                    -1, -1, -1, -1,
                ),
            );
            Ok(_mm256_permutevar8x32_epi32(input, _mm256_setr_epi32(0, 1, 2, 4, 5, 6, -1, -1)))
        }
    }

    define_block_iter!(name=BlockIter, input_chunk_size=32, input_stride=32, output_chunk_size=32, output_stride=24);

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn translate_std(input: __m256i) -> Result<__m256i, ()> {
        let hi_nibbles = _mm256_and_si256(_mm256_srli_epi32(input, 4), _mm256_set1_epi8(0x0f));
        let low_nibbles = _mm256_and_si256(input, _mm256_set1_epi8(0x0f));
        if !is_valid_std(hi_nibbles, low_nibbles) {
            return Err(());
        }

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let shift_lut = _mm256_setr_epi8(
            0,   0,  19,   4, -65, -65, -71, -71,
            0,   0,   0,   0,   0,   0,   0,   0,
            0,   0,  19,   4, -65, -65, -71, -71,
            0,   0,   0,   0,   0,   0,   0,   0,
        );

        let sh = _mm256_shuffle_epi8(shift_lut, hi_nibbles);
        let eq_underscore = _mm256_cmpeq_epi8(input, _mm256_set1_epi8(b'/' as i8));
        let shift = _mm256_blendv_epi8(sh, _mm256_set1_epi8(16), eq_underscore);
        Ok(_mm256_add_epi8(input, shift))
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    #[allow(overflowing_literals)]
    unsafe fn is_valid_std(hi_nibbles: __m256i, low_nibbles: __m256i) -> bool {
        #[cfg_attr(rustfmt, rustfmt_skip)]
        let mask_lut = _mm256_setr_epi8(
            0b1010_1000,                            // 0
            0b1111_1000, 0b1111_1000, 0b1111_1000,  // 1 .. 9
            0b1111_1000, 0b1111_1000, 0b1111_1000,
            0b1111_1000, 0b1111_1000, 0b1111_1000,
            0b1111_0000,                            // 10
            0b0101_0100,                            // 11
            0b0101_0000, 0b0101_0000, 0b0101_0000,  // 12 .. 14
            0b0101_0100,                            // 15

            0b1010_1000,                            // 0
            0b1111_1000, 0b1111_1000, 0b1111_1000,  // 1 .. 9
            0b1111_1000, 0b1111_1000, 0b1111_1000,
            0b1111_1000, 0b1111_1000, 0b1111_1000,
            0b1111_0000,                            // 10
            0b0101_0100,                            // 11
            0b0101_0000, 0b0101_0000, 0b0101_0000,  // 12 .. 14
            0b0101_0100,                            // 15
        );

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let bit_pos_lut = _mm256_setr_epi8(
            0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        );

        let m = _mm256_shuffle_epi8(mask_lut, low_nibbles);
        let bit = _mm256_shuffle_epi8(bit_pos_lut, hi_nibbles);
        let non_match = _mm256_cmpeq_epi8(_mm256_and_si256(m, bit), _mm256_setzero_si256());
        _mm256_movemask_epi8(non_match) == 0
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn translate_url_safe(input: __m256i) -> Result<__m256i, ()> {
        let hi_nibbles = _mm256_and_si256(_mm256_srli_epi32(input, 4), _mm256_set1_epi8(0x0f));
        let low_nibbles = _mm256_and_si256(input, _mm256_set1_epi8(0x0f));
        if !is_valid_url_safe(hi_nibbles, low_nibbles) {
            return Err(());
        }

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let shift_lut = _mm256_setr_epi8(
            0,   0,  17,   4, -65, -65, -71, -71,
            0,   0,   0,   0,   0,   0,   0,   0,
            0,   0,  17,   4, -65, -65, -71, -71,
            0,   0,   0,   0,   0,   0,   0,   0,
        );

        let sh = _mm256_shuffle_epi8(shift_lut, hi_nibbles);
        let eq_underscore = _mm256_cmpeq_epi8(input, _mm256_set1_epi8(b'_' as i8));
        let shift = _mm256_blendv_epi8(sh, _mm256_set1_epi8(-32), eq_underscore);
        Ok(_mm256_add_epi8(input, shift))
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    #[allow(overflowing_literals)]
    unsafe fn is_valid_url_safe(hi_nibbles: __m256i, low_nibbles: __m256i) -> bool {
        #[cfg_attr(rustfmt, rustfmt_skip)]
        let mask_lut = _mm256_setr_epi8(
            0b1010_1000,                            // 0
            0b1111_1000, 0b1111_1000, 0b1111_1000,  // 1 .. 9
            0b1111_1000, 0b1111_1000, 0b1111_1000,
            0b1111_1000, 0b1111_1000, 0b1111_1000,
            0b1111_0000,                            // 10
            0b0101_0000, 0b0101_0000,               // 11 .. 12
            0b0101_0100,                            // 13
            0b0101_0000,                            // 14
            0b0111_0000,                            // 15

            0b1010_1000,                            // 0
            0b1111_1000, 0b1111_1000, 0b1111_1000,  // 1 .. 9
            0b1111_1000, 0b1111_1000, 0b1111_1000,
            0b1111_1000, 0b1111_1000, 0b1111_1000,
            0b1111_0000,                            // 10
            0b0101_0000, 0b0101_0000,               // 11 .. 12
            0b0101_0100,                            // 13
            0b0101_0000,                            // 14
            0b0111_0000,                            // 15
        );

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let bit_pos_lut = _mm256_setr_epi8(
            0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        );

        let m = _mm256_shuffle_epi8(mask_lut, low_nibbles);
        let bit = _mm256_shuffle_epi8(bit_pos_lut, hi_nibbles);
        let non_match = _mm256_cmpeq_epi8(_mm256_and_si256(m, bit), _mm256_setzero_si256());
        _mm256_movemask_epi8(non_match) == 0
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn translate_crypt(input: __m256i) -> Result<__m256i, ()> {
        let hi_nibbles = _mm256_and_si256(_mm256_srli_epi32(input, 4), _mm256_set1_epi8(0x0f));
        let low_nibbles = _mm256_and_si256(input, _mm256_set1_epi8(0x0f));
        if !is_valid_crypt(hi_nibbles, low_nibbles) {
            return Err(());
        }

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let shift_lut = _mm256_setr_epi8(
            0,   0, -46, -46, -53, -53, -59, -59,
            0,   0,   0,   0,   0,   0,   0,   0,
            0,   0, -46, -46, -53, -53, -59, -59,
            0,   0,   0,   0,   0,   0,   0,   0,
        );
        let sh = _mm256_shuffle_epi8(shift_lut, hi_nibbles);
        Ok(_mm256_add_epi8(input, sh))
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    #[allow(overflowing_literals)]
    unsafe fn is_valid_crypt(hi_nibbles: __m256i, low_nibbles: __m256i) -> bool {
        #[cfg_attr(rustfmt, rustfmt_skip)]
        let mask_lut = _mm256_setr_epi8(
            0b1010_1000,                            // 0
            0b1111_1000, 0b1111_1000, 0b1111_1000,  // 1 .. 9 
            0b1111_1000, 0b1111_1000, 0b1111_1000,
            0b1111_1000, 0b1111_1000, 0b1111_1000,
            0b1111_0000,                            // 10
            0b0101_0000, 0b0101_0000, 0b0101_0000,  // 11 .. 13
            0b0101_0100, 0b0101_0100,               // 14 .. 15

            0b1010_1000,                            // 0
            0b1111_1000, 0b1111_1000, 0b1111_1000,  // 1 .. 9 
            0b1111_1000, 0b1111_1000, 0b1111_1000,
            0b1111_1000, 0b1111_1000, 0b1111_1000,
            0b1111_0000,                            // 10
            0b0101_0000, 0b0101_0000, 0b0101_0000,  // 11 .. 13
            0b0101_0100, 0b0101_0100,               // 14 .. 15
        );

        #[cfg_attr(rustfmt, rustfmt_skip)]
        let bit_pos_lut = _mm256_setr_epi8(
            0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        );

        let m = _mm256_shuffle_epi8(mask_lut, low_nibbles);
        let bit = _mm256_shuffle_epi8(bit_pos_lut, hi_nibbles);
        let non_match = _mm256_cmpeq_epi8(_mm256_and_si256(m, bit), _mm256_setzero_si256());
        _mm256_movemask_epi8(non_match) == 0
    }

    impl Translate256i for Std {
        #[inline]
        unsafe fn translate_m256i(input: __m256i) -> Result<__m256i, ()> {
            translate_std(input)
        }
    }

    impl Translate256i for StdNoPad {
        #[inline]
        unsafe fn translate_m256i(input: __m256i) -> Result<__m256i, ()> {
            translate_std(input)
        }
    }

    impl Translate256i for UrlSafe {
        #[inline]
        unsafe fn translate_m256i(input: __m256i) -> Result<__m256i, ()> {
            translate_url_safe(input)
        }
    }

    impl Translate256i for UrlSafeNoPad {
        #[inline]
        unsafe fn translate_m256i(input: __m256i) -> Result<__m256i, ()> {
            translate_url_safe(input)
        }
    }

    impl Translate256i for Crypt {
        #[inline]
        unsafe fn translate_m256i(input: __m256i) -> Result<__m256i, ()> {
            translate_crypt(input)
        }
    }
}