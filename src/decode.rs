use crate::Config;
use std::{error, fmt};

pub(crate) mod block;
pub(crate) mod io;

pub(crate) const INVALID_VALUE: u8 = 255;

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

// decode_slice on success will return the number of decoded bytes written.
pub(crate) fn decode_slice<C>(
    config: C,
    mut input: &[u8],
    mut output: &mut [u8],
) -> Result<usize, DecodeError>
where
    C: Config,
{
    input = remove_padding(config, input)?;
    let (input_idx, output_idx) = decode_full_chunks_without_padding(config, input, output)?;
    input = &input[input_idx..];
    output = &mut output[output_idx..];

    // Deal with the remaining partial chunk. The padding characters have already been removed.
    Ok(output_idx + decode_partial_chunk(config, input, output)?)
}

#[inline]
fn remove_padding<C>(config: C, input: &[u8]) -> Result<&[u8], DecodeError>
where
    C: Config,
{
    Ok(if let Some(padding) = config.padding_byte() {
        if input.len() % 4 != 0 {
            return Err(DecodeError::InvalidLength);
        }
        let num_padding_bytes = input
            .iter()
            .rev()
            .cloned()
            .take_while(|&b| b == padding)
            .take(2)
            .count();
        match num_padding_bytes {
            0 => input,
            1 => &input[..input.len() - 1],
            2 => &input[..input.len() - 2],
            _ => unreachable!("impossible number of padding bytes"),
        }
    } else {
        input
    })
}

#[inline]
fn decode_full_chunks_without_padding<C>(
    config: C,
    mut input: &[u8],
    mut output: &mut [u8],
) -> Result<(usize, usize), DecodeError>
where
    C: Config,
{
    use crate::decode::block::BlockDecoder;
    let (input_idx, output_idx) = if input.len() < 32 {
        (0, 0)
    } else {
        // If input is suitably large use an architecture optimized encoder.
        // The magic value of 27 was chosen because the avx2 encoder works with
        // 28 byte chunks of input at a time. Benchmarks show that bypassing
        // creating the block encoder when the input is small is up to 33%
        // faster (50% throughput improvement).
        let block_encoder = config.into_block_decoder();
        block_encoder.decode_blocks(input, output)?
    };

    input = &input[input_idx..];
    output = &mut output[output_idx..];

    let mut iter = DecodeIter::new(input, output);
    while let Some((input, output)) = iter.next_chunk() {
        decode_chunk(config, *input, output).map_err(DecodeError::InvalidByte)?;
    }

    let (input_idx2, output_idx2) = iter.remaining();
    Ok((input_idx + input_idx2, output_idx + output_idx2))
}

#[inline]
fn decode_partial_chunk<C>(config: C, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError>
where
    C: Config,
{
    // Deal with the remaining partial chunk. The padding characters have already been removed.
    match input.len() {
        0 => Ok(0),
        1 => Err(DecodeError::InvalidLength),
        2 => {
            let first = config.decode_u8(input[0]);
            if first == INVALID_VALUE {
                return Err(DecodeError::InvalidByte(input[0]));
            }
            let second = config.decode_u8(input[1]);
            if second == INVALID_VALUE {
                return Err(DecodeError::InvalidByte(input[1]));
            }
            output[0] = (first << 2) | (second >> 4);
            if second & 0b0000_1111 != 0 {
                return Err(DecodeError::InvalidTrailingBits);
            }
            Ok(1)
        }
        3 => {
            let first = config.decode_u8(input[0]);
            if first == INVALID_VALUE {
                return Err(DecodeError::InvalidByte(input[0]));
            }
            let second = config.decode_u8(input[1]);
            if second == INVALID_VALUE {
                return Err(DecodeError::InvalidByte(input[1]));
            }
            let third = config.decode_u8(input[2]);
            if third == INVALID_VALUE {
                return Err(DecodeError::InvalidByte(input[2]));
            }
            output[0] = (first << 2) | (second >> 4);
            output[1] = (second << 4) | (third >> 2);
            if third & 0b0000_0011 != 0 {
                return Err(DecodeError::InvalidTrailingBits);
            }
            Ok(2)
        }
        x => unreachable!("impossible remainder: {}", x),
    }
}

/// Decode a chunk. The chunk cannot contain any padding.
#[inline]
fn decode_chunk<C: Config>(config: C, input: [u8; 4], output: &mut [u8; 3]) -> Result<(), u8> {
    let mut chunk_output: u32 = 0;
    for (idx, input) in input.iter().cloned().enumerate() {
        let decoded = config.decode_u8(input);
        if decoded == INVALID_VALUE {
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
        let n: [u8; 4] = *(&n.to_be() as *const _ as *const [u8; 4]);
        std::ptr::copy_nonoverlapping(n.as_ptr(), buf.as_mut_ptr(), 3);
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
