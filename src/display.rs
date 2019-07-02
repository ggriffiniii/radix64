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
        let mut buffer = [0; 1024];
        let mut input = self.data;
        while !input.is_empty() {
            let (input_idx, mut output_idx) =
                encode_full_chunks_without_padding(self.config, input, &mut buffer);
            input = &input[input_idx..];
            let output_remaining = buffer.len() - output_idx;
            if output_remaining > 3 {
                debug_assert!(input.len() < 3);
                // We must have either consumed the entire input, or there is a partial chunk remaining with enough room in the buffer to encode it.
                output_idx += encode_partial_chunk(self.config, &input, &mut buffer[output_idx..]);
                input = &input[0..0];
            }
            // Encoded output is always ascii and therefore valid utf8.
            debug_assert!(&buffer[..output_idx].iter().all(u8::is_ascii));
            let output_str = unsafe { std::str::from_utf8_unchecked(&buffer[..output_idx]) };
            f.write_str(output_str)?;
        }
        Ok(())
    }
}
