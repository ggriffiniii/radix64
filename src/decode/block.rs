use crate::decode::INVALID_VALUE;
use crate::DecodeError;
use crate::{Config, CustomConfig};

mod arch;

pub trait IntoBlockDecoder: Copy {
    type BlockDecoder: BlockDecoder;

    fn into_block_decoder(self) -> Self::BlockDecoder;
}

pub trait BlockDecoder: Copy {
    fn decode_blocks(self, input: &[u8], output: &mut [u8]) -> Result<(usize, usize), DecodeError>;
}

#[derive(Debug, Clone, Copy)]
pub struct ScalarBlockDecoder<C>(C);

impl<C> ScalarBlockDecoder<C>
where
    C: Config,
{
    #[inline]
    pub(crate) fn new(config: C) -> Self {
        ScalarBlockDecoder(config)
    }
    fn decode_block(self, input: &[u8; 32], output: &mut [u8; 24]) -> Result<(), u8> {
        for i in 0..4 {
            self.decode_chunk(
                (&input[i * 8..][..8]).try_into().unwrap(),
                (&mut output[i * 6..][..6]).try_into().unwrap(),
            )?;
        }
        Ok(())
    }

    // Padding input as a reference rather than by value improves performance
    // according to the benchmarks on my machine. Ignore the clippy warning.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn decode_chunk(self, input: &[u8; 8], output: &mut [u8; 6]) -> Result<(), u8> {
        let mut chunk_output: u64 = 0;
        for (idx, input) in input.iter().cloned().enumerate() {
            let decoded = self.0.decode_u8(input);
            if decoded == INVALID_VALUE {
                return Err(input);
            }
            let shift_amount = 64 - (idx as u64 + 1) * 6;
            chunk_output |= u64::from(decoded) << shift_amount;
        }
        debug_assert!(chunk_output.trailing_zeros() >= 16);
        write_be_u48(chunk_output, output);
        Ok(())
    }
}

impl<C> BlockDecoder for ScalarBlockDecoder<C>
where
    C: Config,
{
    fn decode_blocks(self, input: &[u8], output: &mut [u8]) -> Result<(usize, usize), DecodeError> {
        let mut iter = BlockIter::new(input, output);
        while let Some((input_block, output_block)) = iter.next_chunk() {
            self.decode_block(input_block, output_block)
                .map_err(DecodeError::InvalidByte)?;
        }
        Ok(iter.remaining())
    }
}

define_block_iter!(
    name = BlockIter,
    input_chunk_size = 32,
    input_stride = 32,
    output_chunk_size = 24,
    output_stride = 24
);

impl IntoBlockDecoder for &CustomConfig {
    type BlockDecoder = ScalarBlockDecoder<Self>;

    #[inline]
    fn into_block_decoder(self) -> Self::BlockDecoder {
        ScalarBlockDecoder::new(self)
    }
}

/// Copy the 48 most significant bits into the provided buffer.
#[inline]
fn write_be_u48(n: u64, buf: &mut [u8; 6]) {
    unsafe {
        let n: [u8; 8] = *(&n.to_be() as *const u64 as *const [u8; 8]);
        std::ptr::copy_nonoverlapping(n.as_ptr(), buf.as_mut_ptr(), 6);
    }
}
