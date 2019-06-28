use crate::copy_in_place;
use crate::encode::{encode_chunk, encode_full_chunks_without_padding, encode_partial_chunk};
use crate::Config;
use std::{fmt, fmt::Debug, io};

/// Encode base64 data as writing to a io::Write. Base64 encoding requires some
/// amount of buffering. EncodeWriter behaves a lot like BufWriter. It will only
/// write bytes to the underlying writer when the internal buffer is at capacity
/// or when an explicit `flush()` is called. Additionally, only whole chunks will
/// be encoded until `finish` is invoked to indicate that no more data will be
/// written. `finish()` will automatically be invoked on Drop if not done explicitly,
/// though if done in Drop it will ignore any errors from the underyling writer.
pub struct EncodeWriter<C, W>
where
    C: Config,
    W: io::Write,
{
    config: C,
    inner: Option<W>,
    // already encoded input, waiting to be written.
    pending_output: [u8; 1024],
    // number of bytes in pending_output.
    bytes_in_pending_output: usize,
    // This is unencoded input that couldn't be encoded due to being a partial chunk.
    partial_input: [u8; 3],
    // number of bytes in partial_input.
    bytes_in_partial_input: usize,
    // A flag used to indicate that a panic was encountered when writing to the
    // inner writer. Used in the Drop impl to not attempt writing to the inner
    // writer again.
    panicked: bool,
}

impl<C, W> EncodeWriter<C, W>
where
    C: Config,
    W: io::Write,
{
    /// Create a new EncodeWriter that wraps the provided writer.
    pub fn new(config: C, writer: W) -> Self {
        EncodeWriter {
            config,
            inner: Some(writer),
            pending_output: [0; 1024],
            bytes_in_pending_output: 0,
            partial_input: [0; 3],
            bytes_in_partial_input: 0,
            panicked: false,
        }
    }

    /// Indicate that we are finished writing. Any partial chunks will be written
    /// to the underyling writer. On error from the underlying write a
    /// FinishError is returned that allows recovering the EncodedWriter if
    /// needed for retries.
    pub fn finish(mut self) -> Result<W, FinishError<Self>> {
        match self.do_finish() {
            Ok(()) => Ok(self.inner.take().unwrap()),
            Err(err) => Err(FinishError(self, err)),
        }
    }

    fn do_finish(&mut self) -> io::Result<()> {
        while self.bytes_in_pending_output > 0 || self.bytes_in_partial_input > 0 {
            let bytes_remaining_in_pending_output =
                self.pending_output.len() - self.bytes_in_pending_output;
            if self.bytes_in_partial_input > 0
                && self.config.encoded_output_len(self.bytes_in_partial_input)
                    < bytes_remaining_in_pending_output
            {
                let partial_chunk = &self.partial_input[..self.bytes_in_partial_input];
                self.bytes_in_pending_output += encode_partial_chunk(
                    self.config,
                    partial_chunk,
                    &mut self.pending_output[self.bytes_in_pending_output..],
                );
                self.bytes_in_partial_input = 0;
            }
            self.write_atleast(self.bytes_in_pending_output)?;
        }
        Ok(())
    }

    fn write_to_inner<R>(&mut self, range: R) -> io::Result<usize>
    where
        R: std::slice::SliceIndex<[u8], Output = [u8]>,
    {
        self.panicked = true;
        let input = &self.pending_output[range];
        let res = self.inner.as_mut().unwrap().write(input);
        self.panicked = false;
        res
    }

    fn write_atleast(&mut self, num_bytes: usize) -> io::Result<usize> {
        debug_assert!(num_bytes <= self.bytes_in_pending_output);
        let mut bytes_written = 0;
        while bytes_written < num_bytes {
            match self.write_to_inner(bytes_written..self.bytes_in_pending_output) {
                Ok(n) => bytes_written += n,
                Err(err) => {
                    self.consume_pending_output(bytes_written);
                    return Err(err);
                }
            }
        }
        self.consume_pending_output(bytes_written);
        Ok(bytes_written)
    }

    fn consume_pending_output(&mut self, num_bytes: usize) {
        debug_assert!(num_bytes <= self.bytes_in_pending_output);
        copy_in_place(
            &mut self.pending_output[..self.bytes_in_pending_output],
            num_bytes..,
            0,
        );
        self.bytes_in_pending_output -= num_bytes;
    }
}

impl<C, W> io::Write for EncodeWriter<C, W>
where
    C: Config,
    W: io::Write,
{
    fn write(&mut self, mut input: &[u8]) -> io::Result<usize> {
        let mut input_bytes_consumed = 0;
        let mut bytes_in_partial_input_checkpoint = 0;
        let mut bytes_in_pending_output_checkpoint = 0;
        // Loop, but at most we'll return halfway through the second iteration.
        loop {
            {
                let bytes_remaining_in_pending_output =
                    self.pending_output.len() - self.bytes_in_pending_output;
                // if the output buffer is full, write atleast enough to make room for
                // one chunk. This may write to the inner writer multiple times, but
                // it's okay because what it's writing is not part of the current input.

                if input_bytes_consumed > 0 {
                    // This is the second iteration of the loop. We've consumed
                    // all the input bytes we can, we will always return out of
                    // this condition.
                    if bytes_remaining_in_pending_output < 4 {
                        // The buffer is at capacity. Attempt a single write.
                        // Restoring bytes_in_pending_output and
                        // bytes_in_partial_chunk on failure.
                        match self.write_to_inner(..self.bytes_in_pending_output) {
                            Ok(bytes_written) => {
                                self.consume_pending_output(bytes_written);
                                return Ok(input_bytes_consumed);
                            }
                            Err(err) => {
                                self.bytes_in_pending_output = bytes_in_pending_output_checkpoint;
                                self.bytes_in_partial_input = bytes_in_partial_input_checkpoint;
                                return Err(err);
                            }
                        }
                    } else {
                        return Ok(input_bytes_consumed);
                    }
                }
                debug_assert!(input_bytes_consumed == 0);

                if bytes_remaining_in_pending_output < 4 {
                    // The output buffer is full only containing data encoded on a
                    // previous invocation of write. Write atleast a full chunks
                    // worth of output to the inner writer. This may invoke write on
                    // the inner writer multiple times, but that's okay because
                    // what's being written did not come from the current input.
                    self.write_atleast(4 - bytes_remaining_in_pending_output)?;
                }
            }

            // We now have atleast 1 full chunk available in pending output and
            // we have not consumed any of this write's input. Save
            // bytes_in_partial_input and bytes_in_pending_output. If we
            // encounter a write error when attempting to write to inner we can
            // restore these values to effectively not consume any input.
            debug_assert!(self.pending_output.len() - self.bytes_in_pending_output >= 4);
            bytes_in_partial_input_checkpoint = self.bytes_in_partial_input;
            bytes_in_pending_output_checkpoint = self.bytes_in_pending_output;

            if self.bytes_in_partial_input > 0 {
                // We have a partial chunk from a previous write. Complete the
                // chunk if possible. Returning if input was too small to
                // complete the chunk.
                let bytes_to_copy = std::cmp::min(input.len(), 3 - self.bytes_in_partial_input);
                self.partial_input
                    [self.bytes_in_partial_input..self.bytes_in_partial_input + bytes_to_copy]
                    .clone_from_slice(&input[..bytes_to_copy]);
                self.bytes_in_partial_input += bytes_to_copy;
                input_bytes_consumed += bytes_to_copy;
                input = &input[bytes_to_copy..];

                if self.bytes_in_partial_input == 3 {
                    encode_chunk(
                        self.config,
                        self.partial_input,
                        arrayref::array_mut_ref!(
                            self.pending_output,
                            self.bytes_in_pending_output,
                            4
                        ),
                    );
                    self.bytes_in_pending_output += 4;
                    self.bytes_in_partial_input = 0;
                } else {
                    // All the input was consumed without completing a chunk.
                    debug_assert!(input.is_empty());
                    debug_assert!(input_bytes_consumed == 1);
                    return Ok(input_bytes_consumed);
                }
            }

            let (full_chunk_bytes_consumed, pending_output_bytes_written) =
                encode_full_chunks_without_padding(
                    self.config,
                    input,
                    &mut self.pending_output[self.bytes_in_pending_output..],
                );
            input_bytes_consumed += full_chunk_bytes_consumed;
            self.bytes_in_pending_output += pending_output_bytes_written;

            input = &input[full_chunk_bytes_consumed..];
            if input.len() < 3 {
                debug_assert!(self.bytes_in_partial_input == 0);
                self.partial_input[..input.len()].clone_from_slice(input);
                self.bytes_in_partial_input = input.len();
                input_bytes_consumed += input.len();
            }
        }
    }

    /// This will only flush full chunks of base64 data. Partial chunks cannot be written until we're done writing completely.
    fn flush(&mut self) -> io::Result<()> {
        let bytes_written = self.write_to_inner(..self.bytes_in_pending_output)?;
        self.consume_pending_output(bytes_written);
        Ok(())
    }
}

impl<C, W> Drop for EncodeWriter<C, W>
where
    C: Config,
    W: io::Write,
{
    fn drop(&mut self) {
        if self.inner.is_some() && !self.panicked {
            let _ = self.do_finish();
        }
    }
}

impl<C, W> Debug for EncodeWriter<C, W>
where
    C: Config,
    W: io::Write,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("EncodeWriter")
            .field("config", &self.config)
            //           .field("inner", &self.inner)
            .field("pending_output", &&self.pending_output[..])
            .field("bytes_in_pending_output", &self.bytes_in_pending_output)
            .field("partial_input", &&self.partial_input[..])
            .field("bytes_in_partial_input", &self.bytes_in_partial_input)
            .field("panicked", &self.panicked)
            .finish()
    }
}

#[derive(Debug)]
/// FinishError is returned from `EncodeWriter::finish` it indicates that the
/// underlying writer returned an error when attempting to write the final chunk.
/// It's possible to recover the EncodeWriter from this error if retrying the
/// finish call is desired.
pub struct FinishError<T>(T, io::Error);

impl<T> FinishError<T> {
    pub fn error(&self) -> &io::Error {
        &self.1
    }

    pub fn into_encode_writer(self) -> T {
        self.0
    }
}

impl<T: Send + fmt::Debug> std::error::Error for FinishError<T> {
    fn description(&self) -> &str {
        std::error::Error::description(self.error())
    }
}

impl<T> fmt::Display for FinishError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <io::Error as fmt::Display>::fmt(self.error(), f)
    }
}
