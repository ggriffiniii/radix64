//! Base64 encoding and decoding.
//!
//! # Quick Examples
//!
//! Encode a message using standard base64 alphabet
//! ```
//! use radix64::STD;
//! assert_eq!(STD.encode("my message"), "bXkgbWVzc2FnZQ==");
//! ```
//!
//! Encode multiple messages while reusing a single buffer. This can be much more efficient when encoding many messages.
//! ```
//! use radix64::STD;
//! let mut buffer = Vec::new();
//! assert_eq!(STD.encode_with_buffer("my message", &mut buffer), "bXkgbWVzc2FnZQ==");
//! assert_eq!(STD.encode_with_buffer("my message2", &mut buffer), "bXkgbWVzc2FnZTI=");
//! assert_eq!(STD.encode_with_buffer("my message3", &mut buffer), "bXkgbWVzc2FnZTM=");
//! ```
//!
//! Decode a message using URL safe alphabet
//! ```
//! use radix64::URL_SAFE;
//! assert_eq!(URL_SAFE.decode("ABCD").unwrap(), &[0, 16, 131]);
//! ```
//!
//! Decode multiple messages while reusing a single buffer. This can be much more efficient when decoding many messages.
//! ```
//! use radix64::URL_SAFE;
//! let mut buffer = Vec::new();
//! assert_eq!(URL_SAFE.decode_with_buffer("ABCD", &mut buffer).unwrap(), &[0, 16, 131]);
//! assert_eq!(URL_SAFE.decode_with_buffer("ABCE", &mut buffer).unwrap(), &[0, 16, 132]);
//! assert_eq!(URL_SAFE.decode_with_buffer("ABCF", &mut buffer).unwrap(), &[0, 16, 133]);
//! ```
//!
//! Decode data from stdin.
//! ```
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # use std::io::Read;
//! use radix64::{STD, io::DecodeReader};
//! let mut reader = DecodeReader::new(STD, std::io::stdin());
//! let mut decoded = Vec::new();
//! reader.read_to_end(&mut decoded)?;
//! # Ok(())
//! # }
//! ```
//!
//! Encode data to stdout.
//! ```
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # use std::io::Write;
//! use radix64::{STD, io::EncodeWriter};
//! let mut writer = EncodeWriter::new(STD, std::io::stdout());
//! writer.write_all("my message".as_bytes())?;
//! # Ok(())
//! # }
//! ```
//!
//! # Configs
//!
//! There are a variety of base64 configurations. There are constants defined
//! representing the most common varieties and the ability to define a custom
//! configuration using [CustomConfig](struct.CustomConfig.html). Each
//! configuration has a set of methods for encoding and decoding. The methods
//! are as follows:
//!
//! #### Encoding
//! | Function             | Output                             | Allocates                        |
//! | -------------------- | ---------------------------------- | -------------------------------- |
//! | `encode`             | Returns a new `String`             | Always                           |
//! | `encode_with_buffer` | Returns a `&str` within the buffer | Only if the buffer needs to grow |
//! | `encode_slice`       | Writes to provided `&mut [u8]`     | Never                            |
//!
//! #### Decoding
//! | Function             | Output                              | Allocates                        |
//! | -------------------- | ----------------------------------- | -------------------------------- |
//! | `decode`             | Returns a new `Vec<u8>`             | Always                           |
//! | `decode_with_buffer` | Returns a `&[u8]` within the buffer | Only if the buffer needs to grow |
//! | `decode_slice`       | Writes to provided `&mut [u8]`      | Never                            |
//!
//! # Performance
//!
//! The provided configurations `STD`, `URL_SAFE`, and `CRYPT` (along with the
//! `NO_PAD` alternatives) each provide an AVX2 optimized implementation. When
//! running on an AVX2 enabled CPU this can be dramatically faster. This library
//! also strives to perform efficiently when not using AVX2. Here is a summary of
//! results compared with the `base64` (v0.10.1) crate. These results were run
//! on an AVX2 enabled workstation and are only meant to serve as a reference.
//! Performance measurements can be very fickle, always measure a representative
//! workload on your system for the most accurate comparisons.
//!
//! ## With AVX2 enabled
//! #### Encoding
//! | Input Byte Size | radix64 Throughput | base64 Throughput |
//! | --------------- | ------------------ | ----------------- |
//! | 3 bytes         | 508 MiB/s          | 344 MiB/s         |
//! | 32 bytes        | 2.09 GiB/s         | 1.30 GiB/s        |
//! | 128 bytes       | 4.16 GiB/s         | 1.92 GiB/s        |
//! | 8192 bytes      | 5.94 GiB/s         | 2.25 GiB/s        |
//!
//! #### Decoding
//! | Input Byte Size | radix64 Throughput | base64 Throughput |
//! | --------------- | ------------------ | ----------------- |
//! | 3 bytes         | 293 MiB/s          | 178 MiB/s         |
//! | 32 bytes        | 1.47 GiB/s         | 973 MiB/s         |
//! | 128 bytes       | 3.85 GiB/s         | 1.55 GiB/s        |
//! | 8192 bytes      | 8.21 GiB/s         | 1.93 GiB/s        |
//!
//! ## Without any SIMD optimizations (--no-default-features)
//! #### Encoding
//! | Input Byte Size | radix64 Throughput | base64 Throughput |
//! | --------------- | ------------------ | ----------------- |
//! | 3 bytes         | 530 MiB/s          | 347 MiB/s         |
//! | 32 bytes        | 1.54 GiB/s         | 1.29 GiB/s        |
//! | 128 bytes       | 2.01 GiB/s         | 1.91 GiB/s        |
//! | 8192 bytes      | 2.25 GiB/s         | 2.20 GiB/s        |
//!
//! #### Decoding
//! | Input Byte Size | radix64 Throughput | base64 Throughput |
//! | --------------- | ------------------ | ----------------- |
//! | 3 bytes         | 302 MiB/s          | 178 MiB/s         |
//! | 32 bytes        | 969 MiB/s          | 976 MiB/s         |
//! | 128 bytes       | 1.59 GiB/s         | 1.55 GiB/s        |
//! | 8192 bytes      | 2.04 GiB/s         | 1.98 GiB/s        |

#![deny(missing_docs)]

#[doc(inline)]
pub use crate::configs::CustomConfig;
pub use crate::decode::DecodeError;
pub use crate::display::Display;

use crate::configs::{Crypt, Fast, Std, StdNoPad, UrlSafe, UrlSafeNoPad};

/// Encode and Decode using the standard characer set with padding.
///
/// See [RFC 4648](https://tools.ietf.org/html/rfc4648#section-4)
pub const STD: Std = Std;

/// Encode and Decode using the standard characer set *without* padding.
///
/// See [RFC 4648](https://tools.ietf.org/html/rfc4648#section-4)
pub const STD_NO_PAD: StdNoPad = StdNoPad;

/// Encode and Decode using the URL safe characer set with padding.
///
/// See [RFC 4648](https://tools.ietf.org/html/rfc4648#section-5)
pub const URL_SAFE: UrlSafe = UrlSafe;

/// Encode and Decode using the URL safe characer set *without* padding.
///
/// See [RFC 4648](https://tools.ietf.org/html/rfc4648#section-5)
pub const URL_SAFE_NO_PAD: UrlSafeNoPad = UrlSafeNoPad;

/// Encode and Decode using the `crypt(3)` character set.
pub const CRYPT: Crypt = Crypt;

/// Encode and Decode using a fast alphabet with no padding.
///
/// This is not part of any official specification and should only be used when
/// interoperability is not a concern. The alphabet used is\
/// ``:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]^_`abcdefghijklmnopqrstuvwxyz``\
/// It's specifically tailored for fast encoding and decoding when AVX2 is in
/// use.
pub const FAST: Fast = Fast;

mod private {
    use crate::decode::block::IntoBlockDecoder;
    use crate::encode::block::IntoBlockEncoder;
    use crate::u6::U6;
    pub trait SealedConfig: IntoBlockEncoder + IntoBlockDecoder {
        /// Encodes the six bits of input into the 8 bits of output.
        fn encode_u6(self, input: U6) -> u8;

        /// Decodes the encoded byte into six bits matching the original input.
        /// config::INVALID_VALUE is returned on invalid input.
        fn decode_u8(self, input: u8) -> u8;

        /// Indicates whether this configuration uses padding and if so, which
        /// character to use.
        fn padding_byte(self) -> Option<u8>;
    }
}

/// Config represents a base64 configuration.
///
/// Each Config provides methods to encode and decode according to the
/// configuration. This trait is sealed and not intended to be implemented
/// outside of this crate. Custom configurations can be defined using
/// [CustomConfig](struct.CustomConfig.html).
pub trait Config: Copy + private::SealedConfig {
    /// Encode the provided input into a String.
    #[inline]
    fn encode<I>(self, input: &I) -> String
    where
        I: AsRef<[u8]> + ?Sized,
    {
        let input = input.as_ref();
        let mut output = vec![0; input.len() * 4 / 3 + 3];
        let bytes_written = crate::encode::encode_slice(self, input, output.as_mut_slice());
        output.truncate(bytes_written);
        // The builtin alphabets are all ascii and the CustomConfigBuilder
        // ensures any custom alphabets only contain ascii characters as well.
        // Therefore we can bypass the utf8 check on the encoded output.
        debug_assert!(output.iter().all(u8::is_ascii));
        unsafe { String::from_utf8_unchecked(output) }
    }

    /// Encode the provided input into the provided buffer, returning a &str of
    /// the encoded input. The returned &str is a view into the beginning of the
    /// provided buffer that contains the encoded data. This method *overwrites*
    /// the data in the buffer, it *does not* append to the buffer. This method
    /// exists to provide an efficient way to amortize allocations when
    /// repeatedly encoding different inputs. The same buffer can be provided for
    /// each invocation and will only be resized when necessary. Any data in the
    /// buffer outside the range of the returned &str is not part of the encoded
    /// output and should be ignored.
    #[inline]
    fn encode_with_buffer<'i, 'b, I>(self, input: &'i I, buffer: &'b mut Vec<u8>) -> &'b str
    where
        I: AsRef<[u8]> + ?Sized,
    {
        let input = input.as_ref();
        let output_size = input.len() * 4 / 3 + 3;
        if output_size > buffer.len() {
            buffer.resize(output_size, 0);
        }
        let num_encoded_bytes = crate::encode::encode_slice(self, input, buffer.as_mut_slice());
        let encoded = &buffer[..num_encoded_bytes];
        // The builtin alphabets are all ascii and the CustomConfigBuilder
        // ensures any custom alphabets only contain ascii characters as well.
        // Therefore we can bypass the utf8 check on the encoded output.
        debug_assert!(encoded.iter().all(u8::is_ascii));
        unsafe { std::str::from_utf8_unchecked(encoded) }
    }

    /// Encode the provided input into the provided output slice. The slice must
    /// be large enough to contain the encoded output and panics if it's not.
    /// Use `input.len() * 4 / 3 + 3` as a conservative estimate. It returns the
    /// number of bytes of encoded output written to the output slice. This
    /// method allows for the most control over memory placement, but
    /// `encode_with_buffer` is typically more ergonomic and just as performant.
    #[inline]
    fn encode_slice<I>(self, input: &I, output: &mut [u8]) -> usize
    where
        I: AsRef<[u8]> + ?Sized,
    {
        crate::encode::encode_slice(self, input.as_ref(), output)
    }

    /// Decode the provided input.
    #[inline]
    fn decode<I>(self, input: &I) -> Result<Vec<u8>, DecodeError>
    where
        I: AsRef<[u8]> + ?Sized,
    {
        let mut output = Vec::new();
        let decoded_len = self.decode_with_buffer(input, &mut output)?.len();
        output.truncate(decoded_len);
        Ok(output)
    }

    /// Decode the provided input into the provided buffer, returning a &[u8] of
    /// the decoded input. The returned &[u8] is a view into the beginning of the
    /// provided buffer that contains the decoded data. This method *overwrites*
    /// the data in the buffer, it *does not* append to the buffer. This method
    /// exists to provide an efficient way to amortize allocations when
    /// repeatedly decoding different inputs. The same buffer can be provided for
    /// each invocation and will only be resized when necessary. Any data in the
    /// buffer outside the range of the returned &[u8] is not part of the decoded
    /// output and should be ignored.
    #[inline]
    fn decode_with_buffer<'i, 'b, I>(
        self,
        input: &'i I,
        buffer: &'b mut Vec<u8>,
    ) -> Result<&'b [u8], DecodeError>
    where
        I: AsRef<[u8]> + ?Sized,
    {
        let input = input.as_ref();
        let output_size = input.len() * 3 / 4 + 1;
        if output_size > buffer.len() {
            buffer.resize(output_size, 0);
        }
        let num_decoded_bytes = crate::decode::decode_slice(self, input, buffer.as_mut_slice())?;
        Ok(&buffer[..num_decoded_bytes])
    }

    /// Decode the provided input into the provided output slice. The slice must
    /// be large enough to contain the decoded output and panics if it's not. Use
    /// `input.len() * 6 / 8 + 1` as a conservative estimate. It returns the
    /// number of bytes of decoded output written to the output slice. This
    /// method allows for the most control over memory placement, but
    /// `decode_with_buffer` is typically more ergonomic and just as performant.
    #[inline]
    fn decode_slice<I>(self, input: &I, output: &mut [u8]) -> Result<usize, DecodeError>
    where
        I: AsRef<[u8]> + ?Sized,
    {
        crate::decode::decode_slice(self, input.as_ref(), output)
    }
}

/// Both encoding and decoding iterate work on chunks of input and output slices.
/// This macro allows creating an efficient iterator to break the slices into
/// defined chunks (possibly differents sizes for input and output) and advance
/// by a defined stride (again possibly different for input and output).
macro_rules! define_block_iter {
    (
        name = $name:ident,
        input_chunk_size = $input_chunk_size:expr,
        input_stride = $input_stride:expr,
        output_chunk_size = $output_chunk_size:expr,
        output_stride = $output_stride:expr
    ) => {
        /// An iterator that accepts an input slice and output slice. It yields
        /// (&[u8; $input_chunk_size], &mut [u8; $output_chunk_size]). Each yield
        /// advances the input $input_stride bytes and the output $output_stride
        /// bytes.
        struct $name<'a, 'b> {
            input: &'a [u8],
            output: &'b mut [u8],
            input_index: usize,
            output_index: usize,
        }

        impl<'a, 'b> $name<'a, 'b> {
            #[inline]
            fn new(input: &'a [u8], output: &'b mut [u8]) -> Self {
                $name {
                    input,
                    output,
                    input_index: 0,
                    output_index: 0,
                }
            }

            #[inline]
            fn next_chunk(
                &mut self,
            ) -> Option<(&[u8; $input_chunk_size], &mut [u8; $output_chunk_size])> {
                if self.input_index + $input_chunk_size <= self.input.len()
                    && self.output_index + $output_chunk_size <= self.output.len()
                {
                    use std::convert::TryInto;
                    let input = (&self.input[self.input_index..][..$input_chunk_size])
                        .try_into()
                        .unwrap();
                    let output = (&mut self.output[self.output_index..][..$output_chunk_size])
                        .try_into()
                        .unwrap();
                    self.input_index += $input_stride;
                    self.output_index += $output_stride;
                    Some((input, output))
                } else {
                    None
                }
            }

            #[allow(dead_code)]
            fn step_back(&mut self) {
                if self.input_index > 0 {
                    self.input_index -= $input_stride;
                    self.output_index -= $output_stride;
                }
            }

            #[inline]
            fn remaining(self) -> (usize, usize) {
                (self.input_index, self.output_index)
            }
        }
    };
}

// mod definitions need to appear after the macro definition.
pub mod configs;
pub(crate) mod decode;
pub(crate) mod display;
pub(crate) mod encode;
pub mod io;
pub(crate) mod tables;
pub(crate) mod u6;

use std::ops::Bound;
use std::ops::RangeBounds;

// Copy the data in slice within the src range, to the index specified by dest.
// This is just a stop-gap until slice::copy_within is stabilized.
pub(crate) fn copy_in_place<T: Copy, R: RangeBounds<usize>>(slice: &mut [T], src: R, dest: usize) {
    let src_start = match src.start_bound() {
        Bound::Included(&n) => n,
        Bound::Excluded(&n) => n.checked_add(1).expect("range bound overflows usize"),
        Bound::Unbounded => 0,
    };
    let src_end = match src.end_bound() {
        Bound::Included(&n) => n.checked_add(1).expect("range bound overflows usize"),
        Bound::Excluded(&n) => n,
        Bound::Unbounded => slice.len(),
    };
    assert!(src_start <= src_end, "src end is before src start");
    assert!(src_end <= slice.len(), "src is out of bounds");
    let count = src_end - src_start;
    assert!(dest <= slice.len() - count, "dest is out of bounds");
    unsafe {
        core::ptr::copy(
            slice.get_unchecked(src_start),
            slice.get_unchecked_mut(dest),
            count,
        );
    }
}
