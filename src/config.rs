use crate::decode::block::IntoBlockDecoder;
use crate::encode::block::IntoBlockEncoder;
use crate::u6::U6;
use crate::DecodeError;
use std::fmt;

pub(crate) const INVALID_VALUE: u8 = 255;

mod private {
    use crate::u6::U6;
    pub trait SealedConfig: std::fmt::Debug {
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

use private::SealedConfig;

/// Config represents a base64 configuration.
///
/// Each Config provides methods to encode and decode according to the
/// configuration. This trait is sealed and not intended to be implemented
/// outside of this crate. Custom configurations can be defined using
/// [ConfigBuilder](struct.ConfigBuilder.html).
pub trait Config: Copy + SealedConfig + IntoBlockEncoder + IntoBlockDecoder {
    /// Encode the provided input into a String.
    fn encode<I>(self, input: &I) -> String
    where
        I: AsRef<[u8]> + ?Sized,
    {
        let input = input.as_ref();
        let mut output = vec![0; self.encoded_output_len(input.len())];
        self.encode_slice(input, output.as_mut_slice());
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
    fn encode_with_buffer<'i, 'b, I>(self, input: &'i I, buffer: &'b mut Vec<u8>) -> &'b str
    where
        I: AsRef<[u8]> + ?Sized,
    {
        let input = input.as_ref();
        let output_size = self.encoded_output_len(input.len());
        if output_size > buffer.len() {
            buffer.resize(output_size, 0);
        }
        let output = &mut buffer[..output_size];
        self.encode_slice(input, output);
        // The builtin alphabets are all ascii and the CustomConfigBuilder
        // ensures any custom alphabets only contain ascii characters as well.
        // Therefore we can bypass the utf8 check on the encoded output.
        debug_assert!(output.iter().all(u8::is_ascii));
        unsafe { std::str::from_utf8_unchecked(output) }
    }

    /// Encode the provided input into the provided output slice. The slice must
    /// be large enough to contain the encoded output. Use `encoded_output_len`
    /// to determine how large the output slice needs to be and how much of the
    /// slice was written to. This method allows for the most control over memory
    /// placement, but `encode_with_buffer` is typically more ergonomic and just
    /// as performant.
    #[inline]
    fn encode_slice<I>(self, input: &I, output: &mut [u8])
    where
        I: AsRef<[u8]> + ?Sized,
    {
        crate::encode::encode_slice(self, input, output)
    }

    /// Decode the provided input.
    fn decode<I>(self, input: &I) -> Result<Vec<u8>, DecodeError>
    where
        I: AsRef<[u8]> + ?Sized,
    {
        let input = input.as_ref();
        let mut output = vec![0; self.maximum_decoded_output_len(input.len())];
        let decoded_len = self.decode_slice(input, output.as_mut_slice())?.len();
        debug_assert!(decoded_len <= output.len());
        output.truncate(decoded_len);
        Ok(output)
    }

    /// Decode the provided input into the provided buffer. The returned &[u8] is a view into the beginning of the provided buffer that contains the decoded output.
    /// Decode the provided input into the provided buffer, returning a &[u8] of
    /// the decoded input. The returned &[u8] is a view into the beginning of the
    /// provided buffer that contains the decoded data. This method *overwrites*
    /// the data in the buffer, it *does not* append to the buffer. This method
    /// exists to provide an efficient way to amortize allocations when
    /// repeatedly decoding different inputs. The same buffer can be provided for
    /// each invocation and will only be resized when necessary. Any data in the
    /// buffer outside the range of the returned &[u8] is not part of the decoded
    /// output and should be ignored.
    fn decode_with_buffer<'i, 'b, I>(
        self,
        input: &'i I,
        buffer: &'b mut Vec<u8>,
    ) -> Result<&'b [u8], DecodeError>
    where
        I: AsRef<[u8]> + ?Sized,
    {
        let input = input.as_ref();
        let output_size = self.maximum_decoded_output_len(input.len());
        if output_size > buffer.len() {
            buffer.resize(output_size, 0);
        }
        self.decode_slice(input, buffer.as_mut_slice())
    }

    /// Decode the provided input into the provided output slice. The slice must
    /// be large enough to contain the decoded output. Use `maximum_decoded_output_len`
    /// to determine how large the output slice needs to be. The returned &[u8]
    /// is a view into the beginning of the output slice that indicates the
    /// length of the decoded output. This method allows for the most control
    /// over memory placement, but `encode_with_buffer` is typically more
    /// ergonomic and just as performant.
    #[inline]
    fn decode_slice<'a, 'b, I>(
        self,
        input: &'a I,
        output: &'b mut [u8],
    ) -> Result<&'b [u8], DecodeError>
    where
        I: AsRef<[u8]> + ?Sized,
    {
        crate::decode::decode_slice(self, input, output)
    }

    /// Determine the size of encoded output for the given input length.
    fn encoded_output_len(self, input_len: usize) -> usize {
        let complete_chunks = input_len / 3;
        let input_remaining = input_len % 3;

        if input_remaining == 0 {
            return complete_chunks * 4;
        }

        if self.padding_byte().is_some() {
            (complete_chunks + 1) * 4
        } else {
            let encoded_remaining = match input_remaining {
                1 => 2,
                2 => 3,
                _ => unreachable!("impossible remainder"),
            };
            complete_chunks * 4 + encoded_remaining
        }
    }

    /// Determine the maximum size of decoded output for the given input length.
    /// Note that this does not necessarily match the actual decoded size, but
    /// represents the upper bound.
    fn maximum_decoded_output_len(self, input_len: usize) -> usize {
        const BITS_PER_ENCODED_BYTE: usize = 6;
        let encoded_bits = input_len * BITS_PER_ENCODED_BYTE;
        (encoded_bits / 8) + 1
    }
}

macro_rules! impl_config_from_table {
    ($cfg:ty, $encode_table:ident, $decode_table:ident, $padding:expr) => {
        impl SealedConfig for $cfg {
            #[inline]
            fn encode_u6(self, input: U6) -> u8 {
                crate::encode::encode_using_table(crate::tables::$encode_table, input)
            }

            #[inline]
            fn decode_u8(self, input: u8) -> u8 {
                crate::decode::decode_using_table(crate::tables::$decode_table, input)
            }

            #[inline]
            fn padding_byte(self) -> Option<u8> {
                $padding
            }
        }

        impl Config for $cfg {}
    };
}

macro_rules! define_inherent_impl {
    ($cfg:ty) => {
        impl $cfg {
            /// See [Config::encode](trait.Config.html#method.encode).
            #[inline]
            pub fn encode<I>(self, input: &I) -> String
            where
                I: AsRef<[u8]> + ?Sized,
            {
                <Self as Config>::encode(self, input)
            }

            /// See [Config::encode_with_buffer](trait.Config.html#method.encode_with_buffer).
            #[inline]
            pub fn encode_with_buffer<'i, 'b, I>(
                self,
                input: &'i I,
                buffer: &'b mut Vec<u8>,
            ) -> &'b str
            where
                I: AsRef<[u8]> + ?Sized,
            {
                <Self as Config>::encode_with_buffer(self, input, buffer)
            }

            /// See [Config::encode_slice](trait.Config.html#method.encode_slice).
            #[inline]
            pub fn encode_slice<I>(self, input: &I, output: &mut [u8])
            where
                I: AsRef<[u8]> + ?Sized,
            {
                <Self as Config>::encode_slice(self, input, output)
            }

            /// See [Config::decode](trait.Config.html#method.decode).
            #[inline]
            pub fn decode<I>(self, input: &I) -> Result<Vec<u8>, DecodeError>
            where
                I: AsRef<[u8]> + ?Sized,
            {
                <Self as Config>::decode(self, input)
            }

            /// See [Config::decode_with_buffer](trait.Config.html#method.decode_with_buffer).
            #[inline]
            pub fn decode_with_buffer<'i, 'b, I>(
                self,
                input: &'i I,
                buffer: &'b mut Vec<u8>,
            ) -> Result<&'b [u8], DecodeError>
            where
                I: AsRef<[u8]> + ?Sized,
            {
                <Self as Config>::decode_with_buffer(self, input, buffer)
            }

            /// See [Config::decode_slice](trait.Config.html#method.decode_slice).
            #[inline]
            pub fn decode_slice<'a, 'b, I>(
                self,
                input: &'a I,
                output: &'b mut [u8],
            ) -> Result<&'b [u8], DecodeError>
            where
                I: AsRef<[u8]> + ?Sized,
            {
                <Self as Config>::decode_slice(self, input, output)
            }

            /// See [Config::encoded_output_len](trait.Config.html#method.encoded_output_len).
            #[inline]
            pub fn encoded_output_len(self, input_len: usize) -> usize {
                <Self as Config>::encoded_output_len(self, input_len)
            }

            /// See [Config::maximum_decoded_output_len](trait.Config.html#method.maximum_decoded_output_len).
            #[inline]
            pub fn maximum_decoded_output_len(self, input_len: usize) -> usize {
                <Self as Config>::maximum_decoded_output_len(self, input_len)
            }
        }
    };
}

/// The standard character set (uses `+` and `/`) with `=` padding.
///
/// See [RFC 4648](https://tools.ietf.org/html/rfc4648#section-4).
#[derive(Debug, Clone, Copy)]
pub struct Std;
impl_config_from_table!(Std, STD_ENCODE, STD_DECODE, Some(b'='));
define_inherent_impl!(Std);

/// The standard character set (uses `+` and `/`) *without* padding.
///
/// See [RFC 4648](https://tools.ietf.org/html/rfc4648#section-4).
#[derive(Debug, Clone, Copy)]
pub struct StdNoPad;
impl_config_from_table!(StdNoPad, STD_ENCODE, STD_DECODE, None);
define_inherent_impl!(StdNoPad);

/// The URL safe character set (uses `-` and `_`) with `=` padding.
///
/// See [RFC 4648](https://tools.ietf.org/html/rfc4648#section-5).
#[derive(Debug, Clone, Copy)]
pub struct UrlSafe;
impl_config_from_table!(UrlSafe, URL_SAFE_ENCODE, URL_SAFE_DECODE, Some(b'='));
define_inherent_impl!(UrlSafe);

/// The URL safe character set (uses `-` and `_`) *without* padding.
///
/// See [RFC 4648](https://tools.ietf.org/html/rfc4648#section-5).
#[derive(Debug, Clone, Copy)]
pub struct UrlSafeNoPad;
impl_config_from_table!(UrlSafeNoPad, URL_SAFE_ENCODE, URL_SAFE_DECODE, None);
define_inherent_impl!(UrlSafeNoPad);

/// The crypt(3) character set
///
/// (uses `./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz`)
/// *without* padding.
#[derive(Debug, Clone, Copy)]
pub struct Crypt;
impl_config_from_table!(Crypt, CRYPT_ENCODE, CRYPT_DECODE, None);
define_inherent_impl!(Crypt);

/// A custom defined alphabet and padding.
///
/// All characters of the alphabet, as well as the padding character (if any),
/// must be ascii characters.
///
/// A CustomConfig is relatively expensive to create. You would typically want to
/// create a CustomConfig once on startup (perhaps using lazy_static) and pass
/// around a reference. Note that Config is only implemented for shared
/// references to CustomConfig, not for CustomConfig itself. Method calls
/// will implicitly take a reference with the `.` operator, but when passing a
/// CustomConfig into a function that expects a Config you will need to pass by
/// reference explicitly.
///
/// ```
/// use radix64::{Config, ConfigBuilder};
/// let my_cfg = ConfigBuilder::with_alphabet("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/").with_padding(b'=').build().unwrap();
///
/// // This works
/// my_cfg.encode("my message");
///
/// // So does this:
///
/// fn do_some_base64<C: Config>(config: C) {
///     config.encode("my message");
/// }
/// do_some_base64(&my_cfg);
///
/// // But this would not
/// // do_some_base64(my_cfg);
/// ```
#[derive(Clone)]
pub struct CustomConfig {
    encode_table: [u8; 64],
    decode_table: [u8; 256],
    padding_byte: Option<u8>,
}

impl SealedConfig for &CustomConfig {
    fn encode_u6(self, input: U6) -> u8 {
        crate::encode::encode_using_table(&self.encode_table, input)
    }

    fn decode_u8(self, input: u8) -> u8 {
        crate::decode::decode_using_table(&self.decode_table, input)
    }

    fn padding_byte(self) -> Option<u8> {
        self.padding_byte
    }
}

impl Config for &CustomConfig {}

impl CustomConfig {
    /// See [Config::encode](trait.Config.html#method.encode).
    #[inline]
    pub fn encode<I>(&self, input: &I) -> String
    where
        I: AsRef<[u8]> + ?Sized,
    {
        <&Self as Config>::encode(self, input)
    }

    /// See [Config::encode_with_buffer](trait.Config.html#method.encode_with_buffer).
    #[inline]
    pub fn encode_with_buffer<'i, 'b, I>(&self, input: &'i I, buffer: &'b mut Vec<u8>) -> &'b str
    where
        I: AsRef<[u8]> + ?Sized,
    {
        <&Self as Config>::encode_with_buffer(self, input, buffer)
    }

    /// See [Config::encode_slice](trait.Config.html#method.encode_slice).
    #[inline]
    pub fn encode_slice<I>(&self, input: &I, output: &mut [u8])
    where
        I: AsRef<[u8]> + ?Sized,
    {
        <&Self as Config>::encode_slice(self, input, output)
    }

    /// See [Config::decode](trait.Config.html#method.decode).
    #[inline]
    pub fn decode<I>(&self, input: &I) -> Result<Vec<u8>, DecodeError>
    where
        I: AsRef<[u8]> + ?Sized,
    {
        <&Self as Config>::decode(self, input)
    }

    /// See [Config::decode_with_buffer](trait.Config.html#method.decode_with_buffer).
    #[inline]
    pub fn decode_with_buffer<'i, 'b, I>(
        &self,
        input: &'i I,
        buffer: &'b mut Vec<u8>,
    ) -> Result<&'b [u8], DecodeError>
    where
        I: AsRef<[u8]> + ?Sized,
    {
        <&Self as Config>::decode_with_buffer(self, input, buffer)
    }

    /// See [Config::decode_slice](trait.Config.html#method.decode_slice).
    #[inline]
    pub fn decode_slice<'a, 'b, I>(
        &self,
        input: &'a I,
        output: &'b mut [u8],
    ) -> Result<&'b [u8], DecodeError>
    where
        I: AsRef<[u8]> + ?Sized,
    {
        <&Self as Config>::decode_slice(self, input, output)
    }

    /// See [Config::encoded_output_len](trait.Config.html#method.encoded_output_len).
    #[inline]
    pub fn encoded_output_len(&self, input_len: usize) -> usize {
        <&Self as Config>::encoded_output_len(self, input_len)
    }

    /// See [Config::maximum_decoded_output_len](trait.Config.html#method.maximum_decoded_output_len).
    #[inline]
    pub fn maximum_decoded_output_len(&self, input_len: usize) -> usize {
        <&Self as Config>::maximum_decoded_output_len(self, input_len)
    }
}

impl fmt::Debug for CustomConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CustomConfig")
            .field("encode_table", &&self.encode_table[..])
            .field("decode_table", &&self.decode_table[..])
            .field("padding_byte", &self.padding_byte)
            .finish()
    }
}

/// A constructor for custom configurations.
#[derive(Debug, Clone)]
pub struct ConfigBuilder<'a> {
    alphabet: &'a [u8],
    padding_byte: Option<u8>,
}

/// Errors that can occur when building a `CustomConfig`.
#[derive(Debug, Clone)]
pub enum CustomConfigError {
    /// The alphabet is not 64 characters long.
    AlphabetNot64Bytes,
    /// The alphabet contains non-ascii characters.
    NonAscii(u8),
    /// The alphabet contains duplicate values.
    DuplicateValue(u8),
}

impl<'a> ConfigBuilder<'a> {
    /// Set the alphabet to use.
    /// The provided alphabet needs to be 64 non-repeating ascii bytes.
    pub fn with_alphabet<A: AsRef<[u8]> + ?Sized>(alphabet: &'a A) -> Self {
        ConfigBuilder {
            alphabet: alphabet.as_ref(),
            padding_byte: Some(b'='),
        }
    }

    /// Set which character to use for padding.
    pub fn with_padding(mut self, padding_byte: u8) -> Self {
        self.padding_byte = Some(padding_byte);
        self
    }

    /// Do not use any padding.
    pub fn no_padding(mut self) -> Self {
        self.padding_byte = None;
        self
    }

    /// Validate and build the `CustomConfig`.
    pub fn build(self) -> Result<CustomConfig, CustomConfigError> {
        if self.alphabet.len() != 64 {
            return Err(CustomConfigError::AlphabetNot64Bytes);
        }
        if let Some(&b) = self.alphabet.iter().find(|b| !b.is_ascii()) {
            return Err(CustomConfigError::NonAscii(b));
        }
        if let Some(b) = self.padding_byte {
            if !b.is_ascii() {
                return Err(CustomConfigError::NonAscii(b));
            }
            // Verify the padding character is not part of the alphabet.
            if self.alphabet.iter().cloned().any(|c| c == b) {
                return Err(CustomConfigError::DuplicateValue(b));
            }
        }
        let mut decode_scratch: Vec<u8> = vec![INVALID_VALUE; 256];
        for (i, b) in self.alphabet.iter().cloned().enumerate() {
            if decode_scratch[b as usize] != INVALID_VALUE {
                return Err(CustomConfigError::DuplicateValue(b));
            }
            decode_scratch[b as usize] = i as u8;
        }
        let mut encode_table = [0; 64];
        let mut decode_table = [0; 256];
        encode_table.copy_from_slice(self.alphabet);
        decode_table.copy_from_slice(&decode_scratch);
        Ok(CustomConfig {
            encode_table,
            decode_table,
            padding_byte: self.padding_byte,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        assert_eq!(Std.encoded_output_len(1), 4);
        assert_eq!(Std.encoded_output_len(2), 4);
        assert_eq!(Std.encoded_output_len(3), 4);
        assert_eq!(Std.encoded_output_len(4), 8);
        assert_eq!(Std.encoded_output_len(5), 8);

        assert_eq!(StdNoPad.encoded_output_len(1), 2);
        assert_eq!(StdNoPad.encoded_output_len(2), 3);
        assert_eq!(StdNoPad.encoded_output_len(3), 4);
        assert_eq!(StdNoPad.encoded_output_len(4), 6);
        assert_eq!(StdNoPad.encoded_output_len(5), 7);
    }
}
