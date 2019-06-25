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
//! # Configs
//!
//! There are a variety of base64 configurations. There are constants defined
//! representing the most common varieties and the ability to define a custom
//! configuration using [ConfigBuilder](struct.ConfigBuilder). Each
//! configuration has a set of methods for encoding and decoding. The methods
//! are as follows:
//!
//! #### Encoding
//! | Function             | Output                           | Allocates                        |
//! | -------------------- | -------------------------------- | -------------------------------- |
//! | `encode`             | Returns a new String             | Always                           |
//! | `encode_with_buffer` | Returns a &str within the buffer | Only if the buffer needs to grow |
//! | `encode_slice`       | Writes to provided &mut [u8]     | Never                            |
//!
//! #### Decoding
//! | Function             | Output                            | Allocates                        |
//! | -------------------- | --------------------------------- | -------------------------------- |
//! | `decode`             | Returns a new Vec<u8>             | Always                           |
//! | `decode_with_buffer` | Returns a &[u8] within the buffer | Only if the buffer needs to grow |
//! | `decode_slice`       | Writes to provided &mut [u8]      | Never                            |
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
//! | 3 bytes         | 401 MiB/s          | 344 MiB/s         |
//! | 32 bytes        | 1.92 GiB/s         | 1.31 GiB/s        |
//! | 128 bytes       | 4.15 GiB/s         | 1.92 GiB/s        |
//! | 8192 bytes      | 6.42 GiB/s         | 2.23 GiB/s        |
//!
//! #### Decoding
//! | Input Byte Size | radix64 Throughput | base64 Throughput |
//! | --------------- | ------------------ | ----------------- |
//! | 3 bytes         | 310 MiB/s          | 178 MiB/s         |
//! | 32 bytes        | 1.09 GiB/s         | 966 MiB/s         |
//! | 128 bytes       | 3.09 GiB/s         | 1.53 GiB/s        |
//! | 8192 bytes      | 8.37 GiB/s         | 1.99 GiB/s        |
//!
//! ## Without any SIMD optimizations (--no-default-features)
//! #### Encoding
//! | Input Byte Size | radix64 Throughput | base64 Throughput |
//! | --------------- | ------------------ | ----------------- |
//! | 3 bytes         | 499 MiB/s          | 346 MiB/s         |
//! | 32 bytes        | 1.39 GiB/s         | 1.31 GiB/s        |
//! | 128 bytes       | 2.00 GiB/s         | 1.92 GiB/s        |
//! | 8192 bytes      | 2.32 GiB/s         | 2.25 GiB/s        |
//!
//! #### Decoding
//! | Input Byte Size | radix64 Throughput | base64 Throughput |
//! | --------------- | ------------------ | ----------------- |
//! | 3 bytes         | 311 MiB/s          | 176 MiB/s         |
//! | 32 bytes        | 1.02 GiB/s         | 970 MiB/s         |
//! | 128 bytes       | 1.69 GiB/s         | 1.54 GiB/s        |
//! | 8192 bytes      | 2.14 GiB/s         | 1.98 GiB/s        |

#![deny(missing_docs)]

pub use config::{
    Config, ConfigBuilder, Crypt, CustomConfig, CustomConfigError, Std, StdNoPad, UrlSafe,
    UrlSafeNoPad,
};
pub use decode::DecodeError;

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

/// Both encoding and decoding iterate work on chunks of input and output slices.
/// This macro allows creating an efficient iterator to break the slices into
/// defined chunks (possibly differents sizes for input and output) and advance
/// by a defined stride (again possibly different for input and output). It uses
/// unsafe mechanisms for efficiency, but the exposed api should be sound.
macro_rules! define_block_iter {
    (name = $name:ident, input_chunk_size = $input_chunk_size:expr, input_stride = $input_stride:expr, output_chunk_size = $output_chunk_size:expr, output_stride = $output_stride:expr) => {
        /// An iterator that accepts an input slice and output slice. It yields (&[u8; $input_chunk_size], &mut [u8; $output_chunk_size]).
        /// Each yield advances the input $input_stride bytes and the output $output_stride bytes.
        struct $name<'a, 'b> {
            input: &'a [u8],
            output: &'b mut [u8],
            input_index: usize,
            output_index: usize,
            /// Do not yield a value unless input_index is less than this value. The
            /// comparison to input_index is only going to happen on indexes evenly
            /// divisible by $input_chunk_size.
            /// i.e. The first comparison will be at index 0, which will yield
            /// $input_chunk_size elements. The second comparison will be at
            /// index $input_stride yielding another $input_chunk_size elements.
            input_end: usize,
        }

        impl<'a, 'b> $name<'a, 'b> {
            #[inline]
            fn new(input: &'a [u8], output: &'b mut [u8]) -> Self {
                let input_chunks = if input.len() >= $input_chunk_size {
                    (input.len() - $input_chunk_size) / $input_stride + 1
                } else {
                    0
                };
                let output_chunks = if output.len() >= $output_chunk_size {
                    (output.len() - $output_chunk_size) / $output_stride + 1
                } else {
                    0
                };
                let num_chunks = std::cmp::min(input_chunks, output_chunks);
                $name {
                    input,
                    output,
                    input_index: 0,
                    output_index: 0,
                    input_end: num_chunks * $input_stride,
                }
            }

            #[inline]
            fn remaining(self) -> (&'a [u8], &'b mut [u8]) {
                (
                    self.input.split_at(self.input_index).1,
                    self.output.split_at_mut(self.output_index).1,
                )
            }
        }

        impl<'a, 'b> Iterator for $name<'a, 'b> {
            type Item = (
                &'a [u8; $input_chunk_size],
                &'b mut [u8; $output_chunk_size],
            );

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                if self.input_index < self.input_end {
                    let input = unsafe {
                        &*(self.input.as_ptr().add(self.input_index)
                            as *const [u8; $input_chunk_size])
                    };
                    let output = unsafe {
                        &mut *(self.output.as_mut_ptr().add(self.output_index)
                            as *mut [u8; $output_chunk_size])
                    };
                    self.input_index += $input_stride;
                    self.output_index += $output_stride;
                    Some((input, output))
                } else {
                    None
                }
            }
        }

        impl<'a, 'b> DoubleEndedIterator for $name<'a, 'b> {
            fn next_back(&mut self) -> Option<Self::Item> {
                if self.input_index > 0 {
                    self.input_index -= $input_stride;
                    self.output_index -= $output_stride;
                    let input = unsafe {
                        &*(self.input.as_ptr().add(self.input_index)
                            as *const [u8; $input_chunk_size])
                    };
                    let output = unsafe {
                        &mut *(self.output.as_mut_ptr().add(self.output_index)
                            as *mut [u8; $output_chunk_size])
                    };
                    Some((input, output))
                } else {
                    None
                }
            }
        }

        // Hardcoding this module name means that we can't define more than one
        // block iter per module. This is fine for now, but would be nice to
        // come up with a better solution.
        /// Property based tests to ensure
        #[cfg(test)]
        mod block_iter_tests {
            use super::$name;
            use proptest::prelude::{any, proptest};

            proptest! {
                #[test]
                fn stay_within_bounds(input in any::<Vec<u8>>(), mut output in any::<Vec<u8>>()) {
                    unsafe {
                        let input_len = input.len();
                        let output_len = output.len();
                        let input_start = input.as_ptr();
                        let input_end = input_start.add(input_len);
                        let output_start = output.as_ptr();
                        let output_end = output_start.add(output_len);
                        let mut iter = $name::new(&input, output.as_mut_slice());
                        let mut chunk_count = 0;
                        for (input_chunk, output_chunk) in iter.by_ref() {
                            chunk_count += 1;
                            assert!(input_chunk.as_ptr() >= input_start);
                            assert!(input_chunk.as_ptr().add($input_chunk_size) <= input_end);
                            assert!(output_chunk.as_ptr() >= output_start);
                            assert!(output_chunk.as_ptr().add($output_chunk_size) <= output_end);
                        }
                        let (input_remaining, output_remaining) = iter.remaining();
                        let input_bytes_remaining = input_len - (chunk_count * $input_stride);
                        assert_eq!(input_bytes_remaining, input_remaining.len());
                        assert_eq!(input_remaining.as_ptr().add(input_remaining.len()), input_end);
                        let output_bytes_remaining = output_len - (chunk_count * $output_stride);
                        assert_eq!(output_bytes_remaining, output_remaining.len());
                        assert_eq!(output_remaining.as_ptr().add(output_remaining.len()), output_end);
                    }
                }

                #[test]
                fn remaining_size_less_than_chunk_size(input in any::<Vec<u8>>(), mut output in any::<Vec<u8>>()) {
                    let mut iter = $name::new(&input, output.as_mut_slice());
                    for _ in iter.by_ref() {}
                    let (input_remaining, output_remaining) = iter.remaining();
                    if $input_chunk_size < input_remaining.len() && $output_chunk_size < output_remaining.len() {
                        panic!("iteration terminated early, leaving a larger than expected remaining slice");
                    }
                }
            }
        }
    };

}

// mod definitions need to appear after the macro definition.
pub(crate) mod config;
pub(crate) mod decode;
pub(crate) mod encode;
pub(crate) mod tables;
pub(crate) mod u6;