use crate::encode::{encode_full_chunks_without_padding, encode_partial_chunk};
use crate::Config;
use std::fmt;

/// Display is a convenience wrapper that provides a Display impl for the passed
/// in data.
pub struct Display<'a, C> {
    config: C,
    data: &'a [u8],
}

impl<'a, C> Display<'a, C> {
    /// Wrap the data, providing a Display implementation that will base64 encode
    /// the data according to the configuration specified.
    pub fn new<T>(config: C, data: &'a T) -> Self
    where
        C: Config,
        T: AsRef<[u8]>,
    {
        Display {
            config,
            data: data.as_ref(),
        }
    }
}

impl<'a, C> fmt::Display for Display<'a, C>
where
    C: Config,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut input = self.data;
        let mut buffer = [0; 1024];
        while input.len() >= 3 {
            let (input_idx, output_idx) =
                encode_full_chunks_without_padding(self.config, input, &mut buffer);
            // Encoded output is always ascii and therefore valid utf8.
            debug_assert!(&buffer[..output_idx].iter().all(u8::is_ascii));
            let output_str = unsafe { std::str::from_utf8_unchecked(&buffer[..output_idx]) };
            f.write_str(&output_str)?;
            input = &input[input_idx..];
        }
        let output_idx = encode_partial_chunk(self.config, input, &mut buffer);
        // Encoded output is always ascii and therefore valid utf8.
        debug_assert!(&buffer[..output_idx].iter().all(u8::is_ascii));
        let output_str = unsafe { std::str::from_utf8_unchecked(&buffer[..output_idx]) };
        f.write_str(&output_str)?;
        Ok(())
    }
}
