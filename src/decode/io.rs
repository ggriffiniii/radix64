use crate::decode::DecodeError;
use crate::Config;
use std::io::Read;

/// Decode base64 data from a std::io::Read.
pub struct DecodeReader<C, R> {
    config: C,
    rdr: R,

    data: [u8; 1024],
    pos: usize,
    cap: usize,
    eof_seen: bool,

    decoded_partial_chunk: [u8; 3],
    // if bytes_contained_in_partial_chunk is zero then decoded_partial_chunk
    // does not contain any data. If it's non-zero then indexes
    // 4-bytes_contained_in_partial_chunk are valid and should be the next bytes
    // returned to the read output buffer.
    bytes_contained_in_partial_chunk: usize,
}

impl<C, R> DecodeReader<C, R>
where
    C: Config,
    R: Read,
{
    /// Create a new DecodeReader that wraps the provided reader.
    pub fn new(config: C, rdr: R) -> Self {
        DecodeReader {
            config,
            rdr,
            data: [0; 1024],
            pos: 0,
            cap: 0,
            eof_seen: false,
            decoded_partial_chunk: [0; 3],
            bytes_contained_in_partial_chunk: 0,
        }
    }

    fn write_partial_chunk(&mut self, output: &mut [u8]) -> usize {
        let bytes_to_copy = std::cmp::min(self.bytes_contained_in_partial_chunk, output.len());
        output[..bytes_to_copy].copy_from_slice(&self.decoded_partial_chunk[..bytes_to_copy]);
        self.bytes_contained_in_partial_chunk -= bytes_to_copy;
        // if bytes remain in the partial chunk move them to the beginning of the array.
        // An alternative to copying the bytes would be to maintain a
        // current position within the decoded_partial_chunk, but that seems
        // like unnecessary complexity to save copying at most 2 bytes.
        for idx in 0..self.bytes_contained_in_partial_chunk {
            self.decoded_partial_chunk[idx] = self.decoded_partial_chunk[idx + bytes_to_copy];
        }
        bytes_to_copy
    }

    fn fill(&mut self) -> std::io::Result<()> {
        crate::copy_in_place(&mut self.data, self.pos..self.cap, 0);
        self.cap -= self.pos;
        self.pos = 0;
        let n = self.rdr.read(&mut self.data[self.cap..])?;
        if n == 0 {
            self.eof_seen = true;
        }
        self.cap += n;

        Ok(())
    }

    fn end_of_decodable_data(&self) -> usize {
        if self.eof_seen {
            self.cap
        } else {
            self.cap.saturating_sub(2)
        }
    }
}

fn into_io_err(err: DecodeError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err)
}

impl<C, R> Read for DecodeReader<C, R>
where
    C: Config,
    R: Read,
{
    fn read(&mut self, mut output: &mut [u8]) -> std::io::Result<usize> {
        // If we've previously partially returned a decoded chunk, return the
        // remaining bytes of the partial result before anything else.
        let mut bytes_written = 0;
        if self.bytes_contained_in_partial_chunk > 0 {
            bytes_written += self.write_partial_chunk(output);
            if self.bytes_contained_in_partial_chunk > 0 {
                return Ok(bytes_written);
            }
        }
        output = &mut output[bytes_written..];

        // Read until we get atleast one full chunk or see EOF.
        while self.end_of_decodable_data() - self.pos < 4 && !self.eof_seen {
            self.fill()?;
        }

        let mut decodable_data = &self.data[self.pos..self.end_of_decodable_data()];

        if decodable_data.is_empty() && self.eof_seen {
            // If we've seen EOF and don't have any decodable data we're done.
            return Ok(bytes_written);
        }

        if self.eof_seen {
            let start_len = decodable_data.len();
            decodable_data = crate::decode::remove_padding(self.config, decodable_data).map_err(into_io_err)?;
            self.cap -= start_len - decodable_data.len();
        }

        let (decodable_data_idx, output_idx) =
            crate::decode::decode_full_chunks_without_padding(self.config, decodable_data, output)
                .map_err(into_io_err)?;
        self.pos += decodable_data_idx;
        bytes_written += output_idx;
        let some_bytes_already_written = decodable_data_idx > 0;

        decodable_data = &decodable_data[decodable_data_idx..];
        output = &mut output[output_idx..];

        match (some_bytes_already_written, self.eof_seen) {
            (some_bytes_already_written, true) => {
                // EOF has been reached. We've already decoded as many full
                // chunks as possible into the output buffer. Either the
                // output buffer is too small to hold the next full chunk or
                // we have a partial chunk of decodable data remaining that
                // may or may not fit into the output buffer.
                if decodable_data.len() < 4
                    && output.len()
                        >= output_bytes_needed_to_decode_partial_chunk(decodable_data.len())?
                {
                    // This is a partial chunk that fits within the output buffer. Decode it.
                    let output_idx =
                        crate::decode::decode_partial_chunk(self.config, decodable_data, output)
                            .map_err(into_io_err)?;
                    self.pos += decodable_data.len();
                    bytes_written += output_idx;
                } else if decodable_data.len() < 4 {
                    // This is a partial chunk that does not fit within the output buffer.
                    // Decode to partial chunk.
                    let output_idx = crate::decode::decode_partial_chunk(
                        self.config,
                        decodable_data,
                        &mut self.decoded_partial_chunk[..],
                    )
                    .map_err(into_io_err)?;
                    self.pos += decodable_data.len();
                    self.bytes_contained_in_partial_chunk = output_idx;
                    bytes_written += self.write_partial_chunk(output);
                } else {
                    // We have atleast one full chunk of decodable data, but
                    // the output buffer is not large enough to hold another
                    // full chunk. If we've already written some bytes, just
                    // return those (maybe we'll get lucky and the next read
                    // will provide a large enough output buffer), otherwise
                    // decode into a partial chunk and copy what we can fit.
                    if some_bytes_already_written {
                        return Ok(bytes_written);
                    }
                    let (bytes_decoded, output_idx) =
                        crate::decode::decode_full_chunks_without_padding(
                            self.config,
                            decodable_data,
                            &mut self.decoded_partial_chunk,
                        )
                        .map_err(into_io_err)?;
                    debug_assert!(output_idx == self.decoded_partial_chunk.len());
                    debug_assert!(bytes_decoded == 4);
                    self.pos += 4;
                    self.bytes_contained_in_partial_chunk = 3;
                    bytes_written += self.write_partial_chunk(output);
                }
            }
            (true, false) => {
                // As many full chunks were written as possible and we
                // haven't yet seen EOF. No more writing is possible until
                // we get more data or see EOF.
            }
            (false, false) => {
                // We have a full chunks worth of decodable data, but none
                // were written. This must mean that the output buffer was
                // too small to hold a full chunk. Decode into a partial
                // chunk.
                assert!(output.len() < 3);
                let (bytes_decoded, output_idx) =
                    crate::decode::decode_full_chunks_without_padding(
                        self.config,
                        decodable_data,
                        &mut self.decoded_partial_chunk,
                    )
                    .map_err(into_io_err)?;
                debug_assert!(output_idx == self.decoded_partial_chunk.len());
                debug_assert!(bytes_decoded == 4);
                self.pos += 4;
                self.bytes_contained_in_partial_chunk = 3;
                bytes_written += self.write_partial_chunk(output);
            }
        }
        Ok(bytes_written)
    }
}

fn output_bytes_needed_to_decode_partial_chunk(
    partial_chunk_len: usize,
) -> Result<usize, std::io::Error> {
    Ok(match partial_chunk_len {
        0 => 0,
        1 => return Err(into_io_err(DecodeError::InvalidLength)),
        2 => 1,
        3 => 2,
        _ => unreachable!("not a valid partial chunk length: {}", partial_chunk_len),
    })
}
