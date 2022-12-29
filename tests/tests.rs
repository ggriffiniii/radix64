use proptest::prelude::Strategy;
use radix64::io::EncodeWriter;
use radix64::{Config, CRYPT, FAST, STD, STD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use std::io;

// Create a custom config that should match each of the builtin configs.
mod custom_configs {
    use radix64::CustomConfig;

    pub static STD: CustomConfig = CustomConfig::with_alphabet(
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
    )
    .with_padding(b'=')
    .build_or_die();

    pub static STD_NO_PAD: CustomConfig = CustomConfig::with_alphabet(
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
    )
    .no_padding()
    .build_or_die();

    pub static URL_SAFE: CustomConfig = CustomConfig::with_alphabet(
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
    )
    .with_padding(b'=')
    .build_or_die();

    pub static URL_SAFE_NO_PAD: CustomConfig = CustomConfig::with_alphabet(
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
    )
    .no_padding()
    .build_or_die();

    pub static CRYPT: CustomConfig = CustomConfig::with_alphabet(
        b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
    )
    .no_padding()
    .build_or_die();

    pub static FAST: CustomConfig = CustomConfig::with_alphabet(
        br#">?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]^_`abcdefghijklmnopqrstuvwxyz{|}"#,
    )
    .no_padding()
    .build_or_die();
}

macro_rules! tests_for_configs {
    ($( $cfg:ident ),+) => {
        #[cfg(test)]
        mod property_tests {
            $(
            #[allow(non_snake_case)]
            mod $cfg {
                use crate::*;
                use proptest::prelude::{any, proptest};
                use proptest::collection::vec;
                proptest! {
                    #[test]
                    fn roundtrip(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        let decoded = $cfg.decode(&encoded).expect("decode failed");
                        assert_eq!(input, decoded);
                    }

                    #[test]
                    fn custom_can_be_decoded_by_builtin(input in any::<Vec<u8>>()) {
                        let encoded = custom_configs::$cfg.encode(&input);
                        let decoded = $cfg.decode(&encoded).expect("decode failed");
                        assert_eq!(input, decoded);
                    }

                    #[test]
                    fn custom_can_decode_builtin(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        let decoded = custom_configs::$cfg.decode(&encoded).expect("decode failed");
                        assert_eq!(input, decoded);
                    }

                    #[test]
                    fn encode_with_buffer_matches_encode(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        let mut buf = Vec::new();
                        let encoded_buf = $cfg.encode_with_buffer(&input, &mut buf);
                        assert_eq!(encoded, encoded_buf);
                    }

                    #[test]
                    fn encode_slice_matches_encode(input in any::<Vec<u8>>()) {
                        let mut encoded_vec = vec![0; input.len() * 4 / 3 + 3];
                        let bytes_written = $cfg.encode_slice(&input, encoded_vec.as_mut_slice());
                        encoded_vec.truncate(bytes_written);
                        let encoded_string = $cfg.encode(&input);
                        assert_eq!(encoded_vec.as_slice(), encoded_string.as_bytes())
                    }

                    #[test]
                    fn display_matches_encode(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        let display = radix64::Display::new($cfg, &input).to_string();
                        assert_eq!(encoded, display);
                    }

                    #[test]
                    fn decode_with_buffer_matches_decode(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        let mut buf = Vec::new();
                        let decoded_buf = $cfg.decode_with_buffer(&encoded, &mut buf).expect("decode failed");
                        let decoded_vec = $cfg.decode(&encoded).expect("decode failed");
                        assert_eq!(decoded_buf, decoded_vec.as_slice());
                    }

                    #[test]
                    fn decode_slice_matches_decode(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        let mut decoded_slice = vec![0; input.len()];
                        let bytes_written = $cfg.decode_slice(&encoded, decoded_slice.as_mut_slice()).expect("decode failed");
                        decoded_slice.truncate(bytes_written);
                        let decoded_vec = $cfg.decode(&encoded).expect("decode failed");
                        assert_eq!(decoded_slice, decoded_vec.as_slice());
                    }

                    #[test]
                    fn encode_slice_always_panics_or_returns_ascii(input in any::<Vec<u8>>(), output_len in 0..1000usize) {
                        let res = std::panic::catch_unwind(|| {
                            let mut encoded = vec![255; output_len];
                            let bytes_written = $cfg.encode_slice(&input, encoded.as_mut_slice());
                            encoded.truncate(bytes_written);
                            encoded
                        });
                        match res {
                            Ok(encoded) => assert!(encoded.iter().all(u8::is_ascii)),
                            Err(_) => {}, // Panic is expected when output len is too short.
                        }
                    }

                    // encode_with_buffer does an unchecked conversion from a
                    // slice of bytes to a &str. This is just a sanity test to
                    // verify the string returned is valid UTF-8.
                    #[test]
                    fn encode_buffer_returns_valid_str(input in any::<Vec<u8>>()) {
                        let mut buffer = Vec::new();
                        let encoded = $cfg.encode_with_buffer(&input, &mut buffer);
                        std::str::from_utf8(encoded.as_bytes()).expect("invalid UTF-8 returned from encode_with_buffer");
                    }

                    // encode does an unchecked conversion from a slice of bytes
                    // to a &str. This is just a sanity test to verify the
                    // string returned is valid UTF-8.
                    #[test]
                    fn encode_returns_valid_str(input in any::<Vec<u8>>()) {
                        let encoded = $cfg.encode(&input);
                        std::str::from_utf8(encoded.as_bytes()).expect("invalid UTF-8 returned from encode_with_buffer");
                    }

                    // Write input through an EncodeWriter ensuring that the output matches.
                    // The reads are done with varying buffer sizes to try and
                    // catch edge cases around chunking.
                    #[test]
                    fn encode_writer_matches((input, flaky_behavior) in vec_and_flaky_writer_behavior()) {
                        use radix64::io::EncodeWriter;
                        let encoded = $cfg.encode(&input);
                        let mut writer_encoded = Vec::new();
                        {
                            let flaky_writer = FlakyWriter::new(&mut writer_encoded, flaky_behavior.into_iter());
                            let mut writer = EncodeWriter::new($cfg, flaky_writer);
                            write_all_with_retries(&mut writer, &input);
                            finish_encode_writer_with_retries(writer);
                        }
                        assert_eq!(encoded.as_bytes(), writer_encoded.as_slice());
                    }

                    #[test]
                    fn encode_writer_one_byte_writes(input in any::<Vec<u8>>()) {
                        use radix64::io::EncodeWriter;
                        use std::io::{Cursor, Write};
                        let encoded = $cfg.encode(&input);
                        let mut writer_encoded = Vec::new();
                        {
                            let mut writer = EncodeWriter::new($cfg, Cursor::new(&mut writer_encoded));
                            for b in input {
                                writer.write(&[b][..]).expect("write failed");
                                // invoking flush is not necessary, but nice to
                                // exercise that codepath somewhere.
                                writer.flush().expect("flush failed");
                            }
                            writer.finish().expect("finish failed");
                        }
                        assert_eq!(encoded.as_bytes(), writer_encoded.as_slice());
                    }

                    // Ensure that EncodeWriter writes the final partial chunk on Drop.
                    #[test]
                    fn encode_writer_writes_final_chunk_on_drop(input in any::<Vec<u8>>()) {
                        use std::io::Write;
                        use radix64::io::EncodeWriter;
                        let encoded = $cfg.encode(&input);
                        let mut writer_encoded = Vec::new();
                        {
                            let mut writer = EncodeWriter::new($cfg, &mut writer_encoded);
                            writer.write_all(&input).expect("failed to write all input");
                            // do not call finish explicitly.
                        }
                        assert_eq!(encoded.as_bytes(), writer_encoded.as_slice());
                    }

                    // read a vector from a DecodeReader, ensuring that it matches the encoded input.
                    // The reads are done with varying buffer sizes to try and
                    // catch edge cases around chunking.
                    #[test]
                    fn decode_reader_roundtrip((input, buffer_sizes) in vec_and_buffer_sizes()) {
                        use radix64::io::DecodeReader;
                        use std::io::Cursor;
                        let encoded = $cfg.encode(&input);
                        let reader = DecodeReader::new($cfg, Cursor::new(encoded));
                        let decoded = read_to_end_using_varying_buffer_sizes(reader, buffer_sizes.iter().cloned()).expect("failed to read to the end of input");
                        assert_eq!(input, decoded);
                    }

                    // ensure that padding in the middle of the input stream is not silently accepted.
                    // The buffer sizes to use are randomly chosen between 1 and 5.
                    #[test]
                    fn decode_reader_reject_middle_padding(buffer_sizes in vec(1 as usize ..5, 1..3)) {
                        use radix64::io::DecodeReader;
                        use std::io::Cursor;
                        let mut input = $cfg.encode("A");
                        input.push_str(&$cfg.encode("BBB"));
                        let reader = DecodeReader::new($cfg, Cursor::new(&input));
                        match read_to_end_using_varying_buffer_sizes(reader, buffer_sizes.iter().cloned()) {
                            Ok(_) => panic!("incorrect padding accepted"),
                            Err(_) => {}, // this is good
                        }
                    }

                    // ensure that reading from a DecodeReader and decoding from
                    // a vector result in the same response.
                    #[test]
                    fn decode_reader_error_matches_decode(input in any::<String>()) {
                        use radix64::io::DecodeReader;
                        use std::io::{Cursor, Read};
                        let mut reader = DecodeReader::new($cfg, Cursor::new(&input));
                        let mut buffer = Vec::new();
                        let reader_res = match reader.read_to_end(&mut buffer) {
                            Ok(_) => Ok(buffer),
                            Err(_) => Err(()),
                        };
                        let res = $cfg.decode(&input).map_err(|_| ());
                        assert_eq!(res, reader_res);
                    }
                }
            })+
        }
    }
}

// define a proptest strategy that returns a random buffer and an additional
// vector that contains usize values of buffer sizes to read from the buffer
// with. The buffer sizes are kept significantly smaller than the size of the
// random buffer to try and catch edge cases around chunked reads.
fn vec_and_buffer_sizes() -> impl Strategy<Value = (Vec<u8>, Vec<usize>)> {
    use proptest::collection::vec;
    use proptest::prelude::{any, Just};
    vec(any::<u8>(), 1..100).prop_flat_map(|v| {
        let len = v.len();
        let max_buffer_size = std::cmp::max(2, len / 3);
        (Just(v), vec(1..max_buffer_size, 1..10))
    })
}

// define a proptest strategy that returns a random buffer and an additional
// vector that contains a series of writer behaviors (max number of bytes the
// writer should consume, return an error, etc.). The max number of bytes
// consumed are kept significantly smaller than the size of the random buffer to
// try and catch edge cases around chunking.
fn vec_and_flaky_writer_behavior() -> impl Strategy<Value = (Vec<u8>, Vec<FlakyWriterBehavior>)> {
    use proptest::collection::vec;
    use proptest::prelude::{any, Just};
    vec(any::<u8>(), 1..1024).prop_flat_map(|v| {
        let len = v.len();
        let max_write_size = std::cmp::max(2, len / 3);
        // Flaky behavior cycles between 9 random conditions, and one condition
        // that consumes 1 bytes. This ensures that all flaky writers makes some
        // amount of progress.
        let flaky_behavior = vec![
            flaky_writer_behavior_strategy(max_write_size).boxed(),
            flaky_writer_behavior_strategy(max_write_size).boxed(),
            flaky_writer_behavior_strategy(max_write_size).boxed(),
            flaky_writer_behavior_strategy(max_write_size).boxed(),
            flaky_writer_behavior_strategy(max_write_size).boxed(),
            flaky_writer_behavior_strategy(max_write_size).boxed(),
            flaky_writer_behavior_strategy(max_write_size).boxed(),
            flaky_writer_behavior_strategy(max_write_size).boxed(),
            flaky_writer_behavior_strategy(max_write_size).boxed(),
            Just(FlakyWriterBehavior::ConsumeBytes(1)).boxed(),
        ];
        (Just(v), flaky_behavior)
    })
}

// A proptest strategry to return a FlakyWriterBehavior that never consumes more
// than max_write_size bytes.
fn flaky_writer_behavior_strategy(
    max_write_size: usize,
) -> impl Strategy<Value = FlakyWriterBehavior> {
    use proptest::prelude::{prop_oneof, Just};
    prop_oneof![
        // For cases without data, `Just` is all you need
        Just(FlakyWriterBehavior::Err(std::io::ErrorKind::Other)),
        // For cases with data, write a strategy for the interior data, then
        // map into the actual enum case.
        (0..max_write_size).prop_map(FlakyWriterBehavior::ConsumeBytes)
    ]
}

// read to the end of the provided reader collecting the results into a vector.
// The read calls to the reader are done in buffer sizes according to the passed
// in iterator.
// For example if the passed in iterator returns [1, 10, 5]. It will first issue
// a read of 1 byte in length, then 10 bytes, then 5 bytes, then 1 byte, rinse
// and repeat until EOF is reached.
fn read_to_end_using_varying_buffer_sizes<R, I>(
    mut rdr: R,
    buffer_sizes: I,
) -> std::io::Result<Vec<u8>>
where
    R: io::Read,
    I: Iterator<Item = usize> + Clone,
{
    let mut v = Vec::new();
    for buffer_size in buffer_sizes.cycle() {
        let prev_len = v.len();
        v.resize(prev_len + buffer_size, 0);
        let n = rdr.read(&mut v[prev_len..])?;
        v.truncate(prev_len + n);
        if n == 0 {
            return Ok(v);
        }
    }
    unreachable!();
}

// Not a generally useful utility. You wouldn't want to retry indefinitely, but
// in our case the errors are known to be intermittent and will resolve in a
// timely fashion.
fn write_all_with_retries<W>(mut writer: W, mut input: &[u8])
where
    W: io::Write,
{
    while !input.is_empty() {
        match writer.write(input) {
            Ok(n) => input = &input[n..],
            Err(_) => {}
        }
    }
}

// Again, not generally useful. Continue retrying EncodeWriter::finish until it
// eventually succeeds.
fn finish_encode_writer_with_retries<C, W>(mut writer: EncodeWriter<C, W>)
where
    C: Config,
    W: io::Write,
{
    loop {
        writer = match writer.finish() {
            Ok(_) => break,
            Err(finish_err) => finish_err.into_encode_writer(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum FlakyWriterBehavior {
    ConsumeBytes(usize),
    Err(io::ErrorKind),
}

struct FlakyWriter<W, I> {
    writer: W,
    behavior_iter: std::iter::Cycle<I>,
}

impl<W, I> FlakyWriter<W, I>
where
    W: io::Write,
    I: Iterator<Item = FlakyWriterBehavior> + Clone,
{
    fn new(writer: W, behavior: I) -> Self {
        FlakyWriter {
            writer,
            behavior_iter: behavior.cycle(),
        }
    }
}

impl<W, I> io::Write for FlakyWriter<W, I>
where
    W: io::Write,
    std::iter::Cycle<I>: Iterator<Item = FlakyWriterBehavior>,
{
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        let behavior = self.behavior_iter.next().unwrap();
        match behavior {
            FlakyWriterBehavior::ConsumeBytes(num_bytes) => {
                let num_bytes = std::cmp::min(input.len(), num_bytes);
                self.writer.write(&input[..num_bytes])
            }
            FlakyWriterBehavior::Err(kind) => {
                Err(io::Error::new(kind, "flaky writer error".to_owned()))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

tests_for_configs!(STD, STD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD, CRYPT, FAST);
