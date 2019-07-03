/// SSE implementation of base64 encoding.
use crate::Config;
use crate::encode::block::{BlockEncoder, IntoBlockEncoder, ScalarBlockEncoder};
use crate::{Std, StdNoPad, UrlSafe, UrlSafeNoPad, Crypt, Fast};

#[derive(Debug,Clone,Copy)]
pub struct Encoder<C>(C);

impl<C> BlockEncoder for Encoder<C> where C: Config + avx2::Translate256i {
    #[inline]
    fn encode_blocks(self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        if let Ok(encoder) = avx2::Encoder::new(self.0) {
            encoder.encode_blocks(input, output)
        } else {
            ScalarBlockEncoder::new(self.0).encode_blocks(input, output)
        }
    }
}

macro_rules! define_into_block_encoder {
    ($( $cfg:ident ),+) => {$(
        impl IntoBlockEncoder for $cfg {
            type BlockEncoder = Encoder<Self>;

            #[inline]
            fn into_block_encoder(self) -> Self::BlockEncoder {
                Encoder(self)
            }
        }
    )+}
}
define_into_block_encoder!(Std,StdNoPad,UrlSafe,UrlSafeNoPad,Crypt,Fast);

mod avx2 {
     #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;
    use crate::{Std, StdNoPad, UrlSafe, UrlSafeNoPad, Crypt, Fast};

    pub trait Translate256i: Copy {
        unsafe fn translate_m256i(input: __m256i) -> __m256i;
    }

    #[derive(Debug,Clone,Copy)]
    pub(crate) struct Encoder<C>(C);

    impl<C> Encoder<C> where C: Translate256i {
        #[inline]
        pub(crate) fn new(config: C) -> Result<Self, ()> {
            if is_x86_feature_detected!("avx2") {
                Ok(Encoder(config))
            } else {
                Err(())
            }
        }

        pub(crate) fn encode_blocks(self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
            // The unsafe block is required because _encode_blocks relies on AVX2
            // intrinsics. This is safe because Encoder::new() ensures that an
            // encoder is only successfully created when the CPU supports AVX2.
            unsafe { self._encode_blocks(input, output) }
        }

        #[target_feature(enable = "avx2")]
        unsafe fn _encode_blocks(self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
            let mut iter = BlockIter::new(input, output);
            for (input, output) in iter.by_ref() {
                #[allow(clippy::cast_ptr_alignment)]
                let lo_data = _mm_loadu_si128(input.as_ptr() as *const __m128i);
                #[allow(clippy::cast_ptr_alignment)]
                let hi_data = _mm_loadu_si128(input.as_ptr().add(12) as *const __m128i);
                let input = _mm256_set_m128i(hi_data, lo_data);
                #[allow(clippy::cast_ptr_alignment)]
                _mm256_storeu_si256(output.as_mut_ptr() as *mut __m256i, self.encode_block(input));
            }
            iter.remaining()
        }

        #[target_feature(enable = "avx2")]
        unsafe fn encode_block(self, input: __m256i) -> __m256i {
            #[rustfmt::skip]
            let input = _mm256_shuffle_epi8(
                input,
                _mm256_setr_epi8(
                    2,  2,  1,  0,  // The trailing comments fix a bug in tarpaulin
                    5,  5,  4,  3,  // causing the args to be lines not covered.
                    8,  8,  7,  6,  //
                    11, 11, 10, 9,  //
                    2,  2,  1,  0,  //
                    5,  5,  4,  3,  //
                    8,  8,  7,  6,  //
                    11, 11, 10, 9,  //
                ),
            );
            let mask = _mm256_set1_epi32(0x3F00_0000);
            let res = _mm256_and_si256(_mm256_srli_epi32(input, 2), mask);
            let mask = _mm256_srli_epi32(mask, 8);
            let res = _mm256_or_si256(res, _mm256_and_si256(_mm256_srli_epi32(input, 4), mask));
            let mask = _mm256_srli_epi32(mask, 8);
            let res = _mm256_or_si256(res, _mm256_and_si256(_mm256_srli_epi32(input, 6), mask));
            let mask = _mm256_srli_epi32(mask, 8);
            let res = _mm256_or_si256(res, _mm256_and_si256(input, mask));
            #[rustfmt::skip]
            let res = _mm256_shuffle_epi8(
                res,
                _mm256_setr_epi8(
                    3,  2,  1,  0,  // The trailing comments fix a bug in tarpaulin
                    7,  6,  5,  4,  // causing the args to be lines not covered.
                    11, 10, 9,  8,  //
                    15, 14, 13, 12, //
                    19, 18, 17, 16, //
                    23, 22, 21, 20, //
                    27, 26, 25, 24, //
                    31, 30, 29, 28, //
                ),
            );
            C::translate_m256i(res)
        }

    }

    define_block_iter!(name=BlockIter, input_chunk_size=28, input_stride=24, output_chunk_size=32, output_stride=32);

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn translate_std(input: __m256i) -> __m256i {
        let s1mask = _mm256_cmpgt_epi8(_mm256_set1_epi8(26), input);
        let mut blockmask = s1mask;
        let s2mask = _mm256_andnot_si256(blockmask, _mm256_cmpgt_epi8(_mm256_set1_epi8(52), input));
        blockmask = _mm256_or_si256(blockmask, s2mask);
        let s3mask = _mm256_andnot_si256(blockmask, _mm256_cmpgt_epi8(_mm256_set1_epi8(62), input));
        blockmask = _mm256_or_si256(blockmask, s3mask);
        let s4mask = _mm256_andnot_si256(blockmask, _mm256_cmpgt_epi8(_mm256_set1_epi8(63), input));
        blockmask = _mm256_or_si256(blockmask, s4mask);
        let s1 = _mm256_and_si256(s1mask, _mm256_add_epi8(input, _mm256_set1_epi8(b'A' as i8)));
        let s2 = _mm256_and_si256(
            s2mask,
            _mm256_add_epi8(input, _mm256_set1_epi8(b'a' as i8 - 26)),
        );
        let s3 = _mm256_and_si256(
            s3mask,
            _mm256_add_epi8(input, _mm256_set1_epi8(b'0' as i8 - 52)),
        );
        let s4 = _mm256_and_si256(s4mask, _mm256_set1_epi8(b'+' as i8));
        let s5 = _mm256_andnot_si256(blockmask, _mm256_set1_epi8(b'/' as i8));
        _mm256_or_si256(
            s1,
            _mm256_or_si256(s2, _mm256_or_si256(s3, _mm256_or_si256(s4, s5))),
        )
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn translate_url_safe(input: __m256i) -> __m256i {
        let s1mask = _mm256_cmpgt_epi8(_mm256_set1_epi8(26), input);
        let mut blockmask = s1mask;
        let s2mask = _mm256_andnot_si256(blockmask, _mm256_cmpgt_epi8(_mm256_set1_epi8(52), input));
        blockmask = _mm256_or_si256(blockmask, s2mask);
        let s3mask = _mm256_andnot_si256(blockmask, _mm256_cmpgt_epi8(_mm256_set1_epi8(62), input));
        blockmask = _mm256_or_si256(blockmask, s3mask);
        let s4mask = _mm256_andnot_si256(blockmask, _mm256_cmpgt_epi8(_mm256_set1_epi8(63), input));
        blockmask = _mm256_or_si256(blockmask, s4mask);
        let s1 = _mm256_and_si256(s1mask, _mm256_add_epi8(input, _mm256_set1_epi8(b'A' as i8)));
        let s2 = _mm256_and_si256(
            s2mask,
            _mm256_add_epi8(input, _mm256_set1_epi8(b'a' as i8 - 26)),
        );
        let s3 = _mm256_and_si256(
            s3mask,
            _mm256_add_epi8(input, _mm256_set1_epi8(b'0' as i8 - 52)),
        );
        let s4 = _mm256_and_si256(s4mask, _mm256_set1_epi8(b'-' as i8));
        let s5 = _mm256_andnot_si256(blockmask, _mm256_set1_epi8(b'_' as i8));
        _mm256_or_si256(
            s1,
            _mm256_or_si256(s2, _mm256_or_si256(s3, _mm256_or_si256(s4, s5))),
        )
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn translate_crypt(input: __m256i) -> __m256i {
        let s1mask = _mm256_cmpgt_epi8(_mm256_set1_epi8(12), input);
        let mut blockmask = s1mask;
        let s2mask = _mm256_andnot_si256(blockmask, _mm256_cmpgt_epi8(_mm256_set1_epi8(38), input));
        blockmask = _mm256_or_si256(blockmask, s2mask);
        let s1 = _mm256_and_si256(s1mask, _mm256_add_epi8(input, _mm256_set1_epi8(b'.' as i8)));
        let s2 = _mm256_and_si256(
            s2mask,
            _mm256_add_epi8(input, _mm256_set1_epi8(b'A' as i8 - 12)),
        );
        let s3 = _mm256_andnot_si256(
            blockmask,
            _mm256_add_epi8(input, _mm256_set1_epi8(b'a' as i8 - 38)),
        );
        _mm256_or_si256(s1, _mm256_or_si256(s2, s3))
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn translate_fast(input: __m256i) -> __m256i {
        _mm256_add_epi8(input, _mm256_set1_epi8(62))
    }

    impl Translate256i for Std {
        #[inline]
        unsafe fn translate_m256i(input: __m256i) -> __m256i {
            translate_std(input)
        }
    }

    impl Translate256i for StdNoPad {
        #[inline]
        unsafe fn translate_m256i(input: __m256i) -> __m256i {
            translate_std(input)
        }
    }

    impl Translate256i for UrlSafe {
        #[inline]
        unsafe fn translate_m256i(input: __m256i) -> __m256i {
            translate_url_safe(input)
        }
    }

    impl Translate256i for UrlSafeNoPad {
        #[inline]
        unsafe fn translate_m256i(input: __m256i) -> __m256i {
            translate_url_safe(input)
        }
    }

    impl Translate256i for Crypt {
        #[inline]
        unsafe fn translate_m256i(input: __m256i) -> __m256i {
            translate_crypt(input)
        }
    }

    impl Translate256i for Fast {
        #[inline]
        unsafe fn translate_m256i(input: __m256i) -> __m256i {
            translate_fast(input)
        }
    }
}