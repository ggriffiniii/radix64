//! The different varieties of base64.
use crate::u6::U6;
use crate::{private::SealedConfig, Config, DecodeError};
use std::fmt;

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
            /// See [Config::encode](../trait.Config.html#method.encode).
            #[inline]
            pub fn encode<I>(self, input: &I) -> String
            where
                I: AsRef<[u8]> + ?Sized,
            {
                <Self as Config>::encode(self, input)
            }

            /// See [Config::encode_with_buffer](../trait.Config.html#method.encode_with_buffer).
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

            /// See [Config::encode_slice](../trait.Config.html#method.encode_slice).
            #[inline]
            pub fn encode_slice<I>(self, input: &I, output: &mut [u8]) -> usize
            where
                I: AsRef<[u8]> + ?Sized,
            {
                <Self as Config>::encode_slice(self, input, output)
            }

            /// See [Config::decode](../trait.Config.html#method.decode).
            #[inline]
            pub fn decode<I>(self, input: &I) -> Result<Vec<u8>, DecodeError>
            where
                I: AsRef<[u8]> + ?Sized,
            {
                <Self as Config>::decode(self, input)
            }

            /// See [Config::decode_with_buffer](../trait.Config.html#method.decode_with_buffer).
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

            /// See [Config::decode_slice](../trait.Config.html#method.decode_slice).
            #[inline]
            pub fn decode_slice<I>(self, input: &I, output: &mut [u8]) -> Result<usize, DecodeError>
            where
                I: AsRef<[u8]> + ?Sized,
            {
                <Self as Config>::decode_slice(self, input, output)
            }
        }
    };
}

/// The standard character set (uses `+` and `/`) with `=` padding.
///
/// See [RFC 4648](https://tools.ietf.org/html/rfc4648#section-4).
#[derive(Debug, Default, Clone, Copy)]
pub struct Std;
impl_config_from_table!(Std, STD_ENCODE, STD_DECODE, Some(b'='));
define_inherent_impl!(Std);

/// The standard character set (uses `+` and `/`) *without* padding.
///
/// See [RFC 4648](https://tools.ietf.org/html/rfc4648#section-4).
#[derive(Debug, Default, Clone, Copy)]
pub struct StdNoPad;
impl_config_from_table!(StdNoPad, STD_ENCODE, STD_DECODE, None);
define_inherent_impl!(StdNoPad);

/// The URL safe character set (uses `-` and `_`) with `=` padding.
///
/// See [RFC 4648](https://tools.ietf.org/html/rfc4648#section-5).
#[derive(Debug, Default, Clone, Copy)]
pub struct UrlSafe;
impl_config_from_table!(UrlSafe, URL_SAFE_ENCODE, URL_SAFE_DECODE, Some(b'='));
define_inherent_impl!(UrlSafe);

/// The URL safe character set (uses `-` and `_`) *without* padding.
///
/// See [RFC 4648](https://tools.ietf.org/html/rfc4648#section-5).
#[derive(Debug, Default, Clone, Copy)]
pub struct UrlSafeNoPad;
impl_config_from_table!(UrlSafeNoPad, URL_SAFE_ENCODE, URL_SAFE_DECODE, None);
define_inherent_impl!(UrlSafeNoPad);

/// The crypt(3) character set
///
/// (uses `./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz`)
/// *without* padding.
#[derive(Debug, Default, Clone, Copy)]
pub struct Crypt;
impl_config_from_table!(Crypt, CRYPT_ENCODE, CRYPT_DECODE, None);
define_inherent_impl!(Crypt);

/// The Fast character set
///
/// (uses `:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]^_`abcdefghijklmnopqrstuvwxyz`)
/// *without* padding.
#[derive(Debug, Default, Clone, Copy)]
pub struct Fast;
impl_config_from_table!(Fast, FAST_ENCODE, FAST_DECODE, None);
define_inherent_impl!(Fast);

/// A custom defined alphabet and padding.
///
/// All characters of the alphabet, as well as the padding character (if any),
/// must be ascii characters.
///
/// # Examples
/// ```
/// // Create a custom base64 configuration that matches what `crypt(3)`
/// // produces. This is equivalent to using radix64::CRYPT except the builtin
/// // constant provides SIMD optimized encoding/decoding when available while a
/// // custom config cannot.
/// use radix64::CustomConfig;
///
/// let my_config = CustomConfig::with_alphabet(
///     b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
/// )
/// .no_padding()
/// .build()
/// .unwrap();
///
/// let my_encoded_msg = my_config.encode("my message");
/// assert_eq!("PLYUPKJnQq3bNE", my_encoded_msg.as_str());
/// assert_eq!("my message".as_bytes(), my_config.decode(&my_encoded_msg).unwrap().as_slice());
/// ```
///
/// Note that building a custom configuration is somewhat expensive. It needs to
/// iterate over the provided alphabet, sanity check it's contents, create an
/// inverted alphabet for decoding, and store the results. For this reason it's
/// encouraged to create a static instance of custom config and share that
/// instance throughout the code.
/// ```
/// use radix64::CustomConfig;
///
/// pub static MY_CONFIG: CustomConfig = CustomConfig::with_alphabet(
///     b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
/// )
/// .with_padding(b'=')
/// .build_or_die();
///
/// let my_encoded_msg = MY_CONFIG.encode("my message");
/// assert_eq!("bXkgbWVzc2FnZQ==", my_encoded_msg.as_str());
/// assert_eq!("my message".as_bytes(), MY_CONFIG.decode(&my_encoded_msg).unwrap().as_slice());
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
    /// Start creating a new CustomConfig with the provided alphabet.
    /// The provided alphabet needs to be 64 non-repeating ascii bytes.
    pub const fn with_alphabet(alphabet: &[u8; 64]) -> CustomConfigBuilder {
        CustomConfigBuilder::with_alphabet(*alphabet)
    }

    /// See [Config::encode](../trait.Config.html#method.encode).
    #[inline]
    pub fn encode<I>(&self, input: &I) -> String
    where
        I: AsRef<[u8]> + ?Sized,
    {
        <&Self as Config>::encode(self, input)
    }

    /// See [Config::encode_with_buffer](../trait.Config.html#method.encode_with_buffer).
    #[inline]
    pub fn encode_with_buffer<'i, 'b, I>(&self, input: &'i I, buffer: &'b mut Vec<u8>) -> &'b str
    where
        I: AsRef<[u8]> + ?Sized,
    {
        <&Self as Config>::encode_with_buffer(self, input, buffer)
    }

    /// See [Config::encode_slice](../trait.Config.html#method.encode_slice).
    #[inline]
    pub fn encode_slice<I>(&self, input: &I, output: &mut [u8]) -> usize
    where
        I: AsRef<[u8]> + ?Sized,
    {
        <&Self as Config>::encode_slice(self, input, output)
    }

    /// See [Config::decode](../trait.Config.html#method.decode).
    #[inline]
    pub fn decode<I>(&self, input: &I) -> Result<Vec<u8>, DecodeError>
    where
        I: AsRef<[u8]> + ?Sized,
    {
        <&Self as Config>::decode(self, input)
    }

    /// See [Config::decode_with_buffer](../trait.Config.html#method.decode_with_buffer).
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

    /// See [Config::decode_slice](../trait.Config.html#method.decode_slice).
    #[inline]
    pub fn decode_slice<'a, 'b, I>(
        &self,
        input: &'a I,
        output: &'b mut [u8],
    ) -> Result<usize, DecodeError>
    where
        I: AsRef<[u8]> + ?Sized,
    {
        <&Self as Config>::decode_slice(self, input, output)
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
///
/// See [CustomConfig](struct.CustomConfig.html)
#[derive(Debug, Clone)]
pub struct CustomConfigBuilder {
    alphabet: [u8; 64],
    padding_byte: Option<u8>,
}

/// Errors that can occur when building a `CustomConfig`.
#[derive(Debug, Clone)]
pub enum CustomConfigError {
    /// The alphabet contains non-ascii characters.
    NonAscii(u8),
    /// The alphabet contains duplicate values.
    DuplicateValue(u8),
}

impl CustomConfigBuilder {
    /// Set the alphabet to use.
    /// The provided alphabet needs to be 64 non-repeating ascii bytes.
    pub const fn with_alphabet(alphabet: [u8; 64]) -> Self {
        CustomConfigBuilder {
            alphabet: alphabet,
            padding_byte: Some(b'='),
        }
    }

    /// Set which character to use for padding.
    pub const fn with_padding(mut self, padding_byte: u8) -> Self {
        self.padding_byte = Some(padding_byte);
        self
    }

    /// Do not use any padding.
    pub const fn no_padding(mut self) -> Self {
        self.padding_byte = None;
        self
    }

    /// Validate and build the `CustomConfig`
    pub const fn build(self) -> Result<CustomConfig, CustomConfigError> {
        use crate::decode::INVALID_VALUE;

        let mut i = 0;
        while i < 64 {
            let b = self.alphabet[i];
            if !b.is_ascii() {
                panic!("non-ascii characater in alphabet");
            }

            match self.padding_byte {
                Some(pad) if pad == b => panic!("padding matches alphabet character"),
                _ => {}
            }
            i += 1;
        }
        if let Some(b) = self.padding_byte {
            if !b.is_ascii() {
                panic!("non-ascii character in padding");
            }
        }
        let mut decode_table = [INVALID_VALUE; 256];
        let mut i = 0;
        while i < 64 {
            let b = self.alphabet[i];
            if decode_table[b as usize] != INVALID_VALUE {
                panic!("duplicate character in alphabet");
            }
            decode_table[b as usize] = i as u8;

            i += 1;
        }
        Ok(CustomConfig {
            encode_table: self.alphabet,
            decode_table,
            padding_byte: self.padding_byte,
        })
    }

    /// Validate and build the `CustomConfig`. Panicking if validation failed.
    /// This is particularly useful within const contexts.
    pub const fn build_or_die(self) -> CustomConfig {
        match self.build() {
            Ok(cfg) => cfg,
            Err(CustomConfigError::NonAscii(_)) => panic!("{}", "non-ascsii character in alphabet"),
            Err(CustomConfigError::DuplicateValue(_)) => {
                panic!("{}", "duplicate character in alphabet")
            }
        }
    }
}
