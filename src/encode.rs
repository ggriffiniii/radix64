use crate::config::Config;
use crate::u6::U6;

pub(crate) mod block;

pub fn encode_slice<C, I>(config: C, input: &I, output: &mut [u8])
where
    C: Config,
    I: AsRef<[u8]> + ?Sized,
{
    use block::BlockEncoder;
    let input = input.as_ref();

    let (input, output) = if input.len() < 28 {
        (input, output)
    } else {
        // If input is suitably large use an architecture optimized encoder.
        // The magic value of 28 was chosen because the avx2 encoder works with
        // 28 byte chunks of input at a time. Benchmarks show that bypassing
        // creating the block encoder when the input is small is up to 33%
        // faster (50% throughput improvement).
        let block_encoder = config.into_block_encoder();
        block_encoder.encode_blocks(input, output)
    };

    // Encode the remaining non-padding 3 byte chunks of input.
    let mut iter = EncodeIter::new(input, output);
    for (input, output) in iter.by_ref() {
        encode_chunk(config, *input, output);
    }

    // Deal with the remaining partial chunk that possibly requires padding.
    let (input, output) = iter.remaining();
    assert!(output.len() >= config.encoded_output_len(input.len()));
    match input.len() {
        0 => {}
        1 => {
            output[0] = config.encode_u6(U6::from_low_six_bits(input[0] >> 2));
            output[1] = config.encode_u6(U6::from_low_six_bits(input[0] << 4));
            if let Some(padding) = config.padding_byte() {
                output[2] = padding;
                output[3] = padding;
            }
        }
        2 => {
            output[0] = config.encode_u6(U6::from_low_six_bits(input[0] >> 2));
            output[1] = config.encode_u6(U6::from_low_six_bits(input[0] << 4 | input[1] >> 4));
            output[2] = config.encode_u6(U6::from_low_six_bits(input[1] << 2));
            if let Some(padding) = config.padding_byte() {
                output[3] = padding;
            }
        }
        x => unreachable!("invalid remaining length: {}", x),
    };
}

fn encode_chunk<C: Config>(config: C, input: [u8; 3], output: &mut [u8; 4]) {
    output[0] = config.encode_u6(U6::from_low_six_bits(input[0] >> 2));
    output[1] = config.encode_u6(U6::from_low_six_bits(input[0] << 4 | input[1] >> 4));
    output[2] = config.encode_u6(U6::from_low_six_bits(input[1] << 2 | input[2] >> 6));
    output[3] = config.encode_u6(U6::from_low_six_bits(input[2]));
}

#[inline]
pub(crate) fn encode_using_table(table: &[u8; 64], input: U6) -> u8 {
    let idx: usize = input.into();
    let encoded = unsafe { table.get_unchecked(idx) };
    *encoded
}

define_block_iter!(
    name = EncodeIter,
    input_chunk_size = 3,
    input_stride = 3,
    output_chunk_size = 4,
    output_stride = 4
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_slice_panics_on_short_output_slice() {
        let did_panic = std::panic::catch_unwind(|| {
            let mut output = vec![0; 1];
            encode_slice(crate::STD, "aaaa", output.as_mut_slice());
        })
        .is_err();
        assert!(did_panic);
    }
}
