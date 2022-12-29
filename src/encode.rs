use crate::u6::U6;
use crate::Config;

pub(crate) mod block;
pub(crate) mod io;

pub(crate) fn encode_slice<C>(config: C, mut input: &[u8], mut output: &mut [u8]) -> usize
where
    C: Config,
{
    let (input_idx, output_idx) = encode_full_chunks_without_padding(config, input, output);
    input = &input[input_idx..];
    output = &mut output[output_idx..];

    output_idx + encode_partial_chunk(config, input, output)
}

#[inline]
pub(crate) fn encode_full_chunks_without_padding<C>(
    config: C,
    mut input: &[u8],
    mut output: &mut [u8],
) -> (usize, usize)
where
    C: Config,
{
    use crate::encode::block::BlockEncoder;
    let (full_block_input_idx, full_block_output_idx) = if input.len() < 32 {
        (0, 0)
    } else {
        // If input is suitably large use an architecture optimized encoder.
        let block_encoder = config.into_block_encoder();
        block_encoder.encode_blocks(input, output)
    };
    input = &input[full_block_input_idx..];
    output = &mut output[full_block_output_idx..];

    // Encode the remaining non-padding 3 byte chunks of input.
    let mut iter = crate::BlockIter::<3, 3, 4, 4>::new(input, output);
    while let Some((input, output)) = iter.next_chunk() {
        encode_chunk(config, *input, output);
    }
    let (chunk_input_idx, chunk_output_idx) = iter.remaining();
    (
        full_block_input_idx + chunk_input_idx,
        full_block_output_idx + chunk_output_idx,
    )
}

#[inline]
pub(crate) fn encode_partial_chunk<C>(config: C, input: &[u8], output: &mut [u8]) -> usize
where
    C: Config,
{
    match input.len() {
        0 => 0,
        1 => {
            output[0] = config.encode_u6(U6::from_low_six_bits(input[0] >> 2));
            output[1] = config.encode_u6(U6::from_low_six_bits(input[0] << 4));
            if let Some(padding) = config.padding_byte() {
                output[2] = padding;
                output[3] = padding;
                4
            } else {
                2
            }
        }
        2 => {
            output[0] = config.encode_u6(U6::from_low_six_bits(input[0] >> 2));
            output[1] = config.encode_u6(U6::from_low_six_bits(input[0] << 4 | input[1] >> 4));
            output[2] = config.encode_u6(U6::from_low_six_bits(input[1] << 2));
            if let Some(padding) = config.padding_byte() {
                output[3] = padding;
                4
            } else {
                3
            }
        }
        _ => panic!("invalid input remaining. Is the output buffer too small?"),
    }
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
    // No need to do bounds checking because a U6 is guaranteed to only contain 0-63
    let encoded = unsafe { table.get_unchecked(idx) };
    *encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_slice_panics_on_short_output_slice() {
        let did_panic = std::panic::catch_unwind(|| {
            let mut output = vec![0; 1];
            encode_slice(crate::STD, "aaaa".as_ref(), output.as_mut_slice());
        })
        .is_err();
        assert!(did_panic);
    }
}
