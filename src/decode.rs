use crate::Config;
use std::{error, fmt};

pub(crate) mod block;

/// Errors that can occur during decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// An invalid byte was found in the input. The offending byte is provided.
    InvalidByte(u8),
    /// The length of the input is invalid.
    InvalidLength,
    /// The last non-padding byte of input has discarded bits and those bits are
    /// not zero. While this could be decoded it likely represents a corrupted or
    /// invalid encoding.
    InvalidTrailingBits,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DecodeError::InvalidByte(byte) => write!(f, "invalid byte {}", byte),
            DecodeError::InvalidLength => write!(f, "encoded text cannot have a 6-bit remainder"),
            DecodeError::InvalidTrailingBits => {
                write!(f, "last byte has unnecessary trailing bits")
            }
        }
    }
}

impl error::Error for DecodeError {
    fn description(&self) -> &str {
        match *self {
            DecodeError::InvalidByte(_) => "invalid byte",
            DecodeError::InvalidLength => "invalid length",
            DecodeError::InvalidTrailingBits => "invalid trailing bits",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
}

/// Decode the provided input slice writing the output to the output slice. It returns a
#[inline]
pub fn decode_slice<'a, 'b, C, I>(
    config: C,
    input: &'a I,
    output: &'b mut [u8],
) -> Result<&'b [u8], DecodeError>
where
    C: Config,
    I: AsRef<[u8]> + ?Sized,
{
    let output_bytes_remaining = _decode_slice(config, input.as_ref(), output)?;
    let output_len = output.len();
    Ok(&output[..output_len - output_bytes_remaining])
}

// _decode_slice on success will return the length of the output buffer
// remaining. i.e. The length of the output buffer that has *not* been written
// to.
fn _decode_slice<C>(config: C, mut input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError>
where
    C: Config,
{
    use block::BlockDecoder;
    if let Some(padding) = config.padding_byte() {
        let num_padding_bytes = input
            .iter()
            .rev()
            .cloned()
            .take_while(|&b| b == padding)
            .take(2)
            .count();
        match num_padding_bytes {
            0 => {}
            1 => input = &input[..input.len() - 1],
            2 => input = &input[..input.len() - 2],
            _ => unreachable!("impossible number of padding bytes"),
        }
    }

    let (input, output) = if input.len() < 32 {
        (input, output)
    } else {
        // If input is suitably large use an architecture optimized encoder.
        // The magic value of 27 was chosen because the avx2 encoder works with
        // 28 byte chunks of input at a time. Benchmarks show that bypassing
        // creating the block encoder when the input is small is up to 33%
        // faster (50% throughput improvement).
        let block_encoder = config.into_block_decoder();
        block_encoder.decode_blocks(input, output)?
    };

    let mut iter = DecodeIter::new(input, output);
    for (input, output) in iter.by_ref() {
        decode_chunk(config, *input, output).map_err(DecodeError::InvalidByte)?;
    }
    let (input, output) = iter.remaining();
    // Deal with the remaining partial chunk. The padding characters have already been removed.
    let output_remaining_len = output.len()
        - match input.len() {
            0 => 0,
            1 => return Err(DecodeError::InvalidLength),
            2 => {
                let first = config.decode_u8(input[0]);
                if first == crate::config::INVALID_VALUE {
                    return Err(DecodeError::InvalidByte(input[0]));
                }
                let second = config.decode_u8(input[1]);
                if second == crate::config::INVALID_VALUE {
                    return Err(DecodeError::InvalidByte(input[1]));
                }
                output[0] = (first << 2) | (second >> 4);
                if second & 0b0000_1111 != 0 {
                    return Err(DecodeError::InvalidTrailingBits);
                }
                1
            }
            3 => {
                let first = config.decode_u8(input[0]);
                if first == crate::config::INVALID_VALUE {
                    return Err(DecodeError::InvalidByte(input[0]));
                }
                let second = config.decode_u8(input[1]);
                if second == crate::config::INVALID_VALUE {
                    return Err(DecodeError::InvalidByte(input[1]));
                }
                let third = config.decode_u8(input[2]);
                if third == crate::config::INVALID_VALUE {
                    return Err(DecodeError::InvalidByte(input[2]));
                }
                output[0] = (first << 2) | (second >> 4);
                output[1] = (second << 4) | (third >> 2);
                if third & 0b0000_0011 != 0 {
                    return Err(DecodeError::InvalidTrailingBits);
                }
                2
            }
            x => unreachable!("impossible remainder: {}", x),
        };
    Ok(output_remaining_len)
}

/// Decode a chunk. The chunk cannot contain any padding.
#[inline]
fn decode_chunk<C: Config>(config: C, input: [u8; 4], output: &mut [u8; 3]) -> Result<(), u8> {
    let mut chunk_output: u32 = 0;
    for (idx, input) in input.iter().cloned().enumerate() {
        let decoded = config.decode_u8(input);
        if decoded == crate::config::INVALID_VALUE {
            return Err(input);
        }
        let shift_amount = 32 - (idx as u32 + 1) * 6;
        chunk_output |= u32::from(decoded) << shift_amount;
    }
    debug_assert!(chunk_output.trailing_zeros() >= 8);
    write_be_u24(chunk_output, output);
    Ok(())
}

/// Copy the 24 most significant bits into the provided buffer.
#[inline]
fn write_be_u24(n: u32, buf: &mut [u8; 3]) {
    unsafe {
        let n = (&n.to_be_bytes()) as *const u8;
        std::ptr::copy_nonoverlapping(n, buf.as_mut_ptr(), 3);
    }
}

#[inline]
pub(crate) fn decode_using_table(table: &[u8; 256], input: u8) -> u8 {
    table[input as usize]
}

define_block_iter!(
    name = DecodeIter,
    input_chunk_size = 4,
    input_stride = 4,
    output_chunk_size = 3,
    output_stride = 3
);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]

    fn detect_trailing_bits() {
        use crate::STD;
        assert!(STD.decode("iYU=").is_ok());
        assert_eq!(Err(DecodeError::InvalidTrailingBits), STD.decode("iYV="));
        assert_eq!(Err(DecodeError::InvalidTrailingBits), STD.decode("iYW="));
        assert_eq!(Err(DecodeError::InvalidTrailingBits), STD.decode("iYX="));
        assert_eq!(
            Err(DecodeError::InvalidTrailingBits),
            STD.decode("AAAAiYX=")
        );
    }

}
