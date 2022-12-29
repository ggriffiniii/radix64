use crate::u6::U6;
use crate::{Config, CustomConfig};

mod arch;

pub trait IntoBlockEncoder: Copy {
    type BlockEncoder: BlockEncoder;

    fn into_block_encoder(self) -> Self::BlockEncoder;
}

pub trait BlockEncoder: Copy {
    fn encode_blocks(self, input: &[u8], output: &mut [u8]) -> (usize, usize);
}

#[derive(Debug, Clone, Copy)]
pub struct ScalarBlockEncoder<C>(C);

impl<C> ScalarBlockEncoder<C>
where
    C: Config,
{
    #[inline]
    pub(crate) fn new(config: C) -> Self {
        ScalarBlockEncoder(config)
    }

    fn encode_chunk(self, input: u64, output: &mut [u8; 8]) {
        for (idx, out) in output.iter_mut().enumerate() {
            let shift_amount = 64 - (idx as u64 + 1) * 6;
            let shifted_input = input >> shift_amount;
            *out = self.0.encode_u6(U6::from_low_six_bits(shifted_input as u8));
        }
    }
}

impl<C> BlockEncoder for ScalarBlockEncoder<C>
where
    C: Config,
{
    #[inline]
    fn encode_blocks(self, input: &[u8], output: &mut [u8]) -> (usize, usize) {
        let mut iter = crate::BlockIter::<26, 24, 32, 32>::new(input, output);
        while let Some((input_block, output_block)) = iter.next_chunk() {
            for i in 0..4 {
                self.encode_chunk(
                    from_be_bytes((&input_block[i * 6..][..8]).try_into().unwrap()),
                    (&mut output_block[i * 8..][..8]).try_into().unwrap(),
                );
            }
        }
        iter.remaining()
    }
}

#[inline]
fn from_be_bytes(input: [u8; 8]) -> u64 {
    let mut output: u64 = 0;
    unsafe {
        std::ptr::copy_nonoverlapping(input.as_ptr(), &mut output as *mut u64 as *mut u8, 8);
    }
    output.to_be()
}

impl IntoBlockEncoder for &CustomConfig {
    type BlockEncoder = ScalarBlockEncoder<Self>;

    #[inline]
    fn into_block_encoder(self) -> Self::BlockEncoder {
        ScalarBlockEncoder::new(self)
    }
}
