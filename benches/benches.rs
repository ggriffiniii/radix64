use criterion::{
    black_box, criterion_group, criterion_main, Bencher, Criterion, ParameterizedBenchmark,
    Throughput,
};
use radix64::Config;

use base64::STANDARD as B64_CONFIG;
use radix64::STD as RADIX_CONFIG;

mod radix {
    use super::*;
    use rand::Rng;

    pub(crate) fn encode<C: Config>(config: C, b: &mut Bencher, &size: &usize) {
        let mut input = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        b.iter(|| {
            let encoded = config.encode(&input);
            black_box(&encoded);
        })
    }

    pub(crate) fn decode<C: Config>(config: C, b: &mut Bencher, &size: &usize) {
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let encoded = config.encode(&input);
        b.iter(|| {
            let decoded = config.decode(&encoded).expect("decode failed");
            black_box(&decoded);
        })
    }

    pub(crate) fn encode_with_buffer<C: Config>(config: C, b: &mut Bencher, &size: &usize) {
        let mut input = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let mut buffer = Vec::new();
        b.iter(|| {
            let encoded = config.encode_with_buffer(&input, &mut buffer);
            black_box(&encoded);
        })
    }

    pub(crate) fn decode_with_buffer<C: Config>(config: C, b: &mut Bencher, &size: &usize) {
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let encoded = config.encode(&input);
        let mut buffer = Vec::new();
        b.iter(|| {
            let decoded = config
                .decode_with_buffer(&encoded, &mut buffer)
                .expect("decode failed");
            black_box(&decoded);
        })
    }

    pub(crate) fn encode_slice<C: Config>(config: C, b: &mut Bencher, &size: &usize) {
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let mut output = vec![0; config.encoded_output_len(size)];
        b.iter(|| {
            config.encode_slice(&input, output.as_mut_slice());
            black_box(&output);
        })
    }

    pub(crate) fn decode_slice<C: Config>(config: C, b: &mut Bencher, &size: &usize) {
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let encoded = config.encode(&input);
        let mut decoded = vec![0; size];
        b.iter(|| {
            config
                .decode_slice(&encoded, decoded.as_mut_slice())
                .unwrap();
            black_box(&decoded);
        })
    }

    pub(crate) fn encode_writer<C: Config>(config: C, b: &mut Bencher, &size: &usize) {
        use std::io::Write;
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let mut output = Vec::with_capacity(config.encoded_output_len(size));
        b.iter(|| {
            output.clear();
            let mut writer = radix64::io::EncodeWriter::new(config, &mut output);
            writer.write_all(&input).expect("failed to write all input");
            writer.finish().expect("failed to finish write");
            black_box(&output);
        })
    }

    pub(crate) fn decode_reader<C: Config>(config: C, b: &mut Bencher, &size: &usize) {
        use std::io::Read;
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let encoded = config.encode(&input);
        let mut decoded = Vec::with_capacity(input.len());
        b.iter(|| {
            decoded.clear();
            let mut rdr = radix64::io::DecodeReader::new(config, std::io::Cursor::new(&encoded));
            rdr.read_to_end(&mut decoded)
                .expect("failed to read to the end");
            black_box(&decoded);
        })
    }

    pub(crate) fn display<C: Config>(config: C, b: &mut Bencher, &size: &usize) {
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        b.iter(|| {
            let display = radix64::Display::new(config, &input).to_string();
            black_box(&display);
        })
    }
}

mod b64 {
    use super::*;
    use rand::Rng;

    pub(crate) fn encode(config: base64::Config, b: &mut Bencher, &size: &usize) {
        let mut input = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        b.iter(|| {
            let encoded = base64::encode_config(&input, config);
            black_box(&encoded);
        })
    }

    pub(crate) fn decode(config: base64::Config, b: &mut Bencher, &size: &usize) {
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let encoded = base64::encode_config(&input, config);
        b.iter(|| {
            let decoded = base64::decode_config(&encoded, config).expect("decode failed");
            black_box(&decoded);
        })
    }

    pub(crate) fn encode_with_buffer(config: base64::Config, b: &mut Bencher, &size: &usize) {
        let mut input = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let mut buffer = String::new();
        b.iter(|| {
            buffer.clear();
            let encoded = base64::encode_config_buf(&input, config, &mut buffer);
            black_box(&encoded);
        })
    }

    pub(crate) fn decode_with_buffer(config: base64::Config, b: &mut Bencher, &size: &usize) {
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let encoded = base64::encode(&input);
        let mut buffer = Vec::new();
        b.iter(|| {
            buffer.clear();
            base64::decode_config_buf(&encoded, config, &mut buffer).expect("decode failed");
            black_box(&buffer);
        })
    }

    pub(crate) fn encode_slice(config: base64::Config, b: &mut Bencher, &size: &usize) {
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let mut output = vec![0; size * 4 / 3 + 4];
        b.iter(|| {
            base64::encode_config_slice(&input, config, output.as_mut_slice());
            black_box(&output);
        })
    }

    pub(crate) fn decode_slice(config: base64::Config, b: &mut Bencher, &size: &usize) {
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let encoded = base64::encode(&input);
        let mut decoded = vec![0; size];
        b.iter(|| {
            base64::decode_config_slice(&encoded, config, decoded.as_mut_slice()).unwrap();
            black_box(&decoded);
        })
    }

    pub(crate) fn encode_writer(config: base64::Config, b: &mut Bencher, &size: &usize) {
        use std::io::Write;
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        let mut output = Vec::with_capacity(size * 4 / 3 + 4);
        b.iter(|| {
            output.clear();
            {
                let mut writer = base64::write::EncoderWriter::new(&mut output, config);
                writer.write_all(&input).expect("failed to write all input");
                writer.finish().expect("failed to finish write");
            }
            black_box(&output);
        })
    }

    pub(crate) fn display(config: base64::Config, b: &mut Bencher, &size: &usize) {
        let mut input: Vec<u8> = vec![0; size];
        rand::thread_rng().fill(input.as_mut_slice());
        b.iter(|| {
            let display = base64::display::Base64Display::with_config(&input, config).to_string();
            black_box(&display);
        })
    }
}

const BYTE_SIZES: [usize; 7] = [3, 32, 64, 128, 512, 4096, 8192];

pub fn encode_benches(byte_sizes: &[usize]) -> ParameterizedBenchmark<usize> {
    ParameterizedBenchmark::new(
        "radix64",
        |b, s| radix::encode(RADIX_CONFIG, b, s),
        byte_sizes.iter().cloned(),
    )
    .with_function("base64", |b, s| b64::encode(B64_CONFIG, b, s))
}

pub fn encode_with_buffer_benches(byte_sizes: &[usize]) -> ParameterizedBenchmark<usize> {
    ParameterizedBenchmark::new(
        "radix64",
        |b, s| radix::encode_with_buffer(RADIX_CONFIG, b, s),
        byte_sizes.iter().cloned(),
    )
    .with_function("base64", |b, s| b64::encode_with_buffer(B64_CONFIG, b, s))
}

pub fn encode_slice_benches(byte_sizes: &[usize]) -> ParameterizedBenchmark<usize> {
    ParameterizedBenchmark::new(
        "radix64",
        |b, s| radix::encode_slice(RADIX_CONFIG, b, s),
        byte_sizes.iter().cloned(),
    )
    .with_function("base64", |b, s| b64::encode_slice(B64_CONFIG, b, s))
}

pub fn encode_writer_benches(byte_sizes: &[usize]) -> ParameterizedBenchmark<usize> {
    ParameterizedBenchmark::new(
        "radix64",
        |b, s| radix::encode_writer(RADIX_CONFIG, b, s),
        byte_sizes.iter().cloned(),
    )
    .with_function("base64", |b, s| b64::encode_writer(B64_CONFIG, b, s))
}

pub fn decode_benches(byte_sizes: &[usize]) -> ParameterizedBenchmark<usize> {
    ParameterizedBenchmark::new(
        "radix64",
        |b, s| radix::decode(RADIX_CONFIG, b, s),
        byte_sizes.iter().cloned(),
    )
    .with_function("base64", |b, s| b64::decode(B64_CONFIG, b, s))
}

pub fn decode_with_buffer_benches(byte_sizes: &[usize]) -> ParameterizedBenchmark<usize> {
    ParameterizedBenchmark::new(
        "radix64",
        |b, s| radix::decode_with_buffer(RADIX_CONFIG, b, s),
        byte_sizes.iter().cloned(),
    )
    .with_function("base64", |b, s| b64::decode_with_buffer(B64_CONFIG, b, s))
}

pub fn decode_slice_benches(byte_sizes: &[usize]) -> ParameterizedBenchmark<usize> {
    ParameterizedBenchmark::new(
        "radix64",
        |b, s| radix::decode_slice(RADIX_CONFIG, b, s),
        byte_sizes.iter().cloned(),
    )
    .with_function("base64", |b, s| b64::decode_slice(B64_CONFIG, b, s))
}

pub fn decode_reader_benches(byte_sizes: &[usize]) -> ParameterizedBenchmark<usize> {
    ParameterizedBenchmark::new(
        "radix64",
        |b, s| radix::decode_reader(RADIX_CONFIG, b, s),
        byte_sizes.iter().cloned(),
    )
}

pub fn display_benches(byte_sizes: &[usize]) -> ParameterizedBenchmark<usize> {
    ParameterizedBenchmark::new(
        "radix64",
        |b, s| radix::display(RADIX_CONFIG, b, s),
        byte_sizes.iter().cloned(),
    )
    .with_function("base64", |b, s| b64::display(B64_CONFIG, b, s))
}

fn customize_benchmark(benchmark: ParameterizedBenchmark<usize>) -> ParameterizedBenchmark<usize> {
    benchmark.throughput(|s| Throughput::Bytes(*s as u32))
}

fn bench(c: &mut Criterion) {
    c.bench(
        "encode_bench",
        customize_benchmark(encode_benches(&BYTE_SIZES[..])),
    );
    c.bench(
        "encode_slice_bench",
        customize_benchmark(encode_slice_benches(&BYTE_SIZES[..])),
    );
    c.bench(
        "encode_with_buffer_bench",
        customize_benchmark(encode_with_buffer_benches(&BYTE_SIZES[..])),
    );
    c.bench(
        "encode_writer_bench",
        customize_benchmark(encode_writer_benches(&BYTE_SIZES[..])),
    );
    c.bench(
        "decode_bench",
        customize_benchmark(decode_benches(&BYTE_SIZES[..])),
    );
    c.bench(
        "decode_slice_bench",
        customize_benchmark(decode_slice_benches(&BYTE_SIZES[..])),
    );
    c.bench(
        "decode_with_buffer_bench",
        customize_benchmark(decode_with_buffer_benches(&BYTE_SIZES[..])),
    );
    c.bench(
        "decode_reader",
        customize_benchmark(decode_reader_benches(&BYTE_SIZES[..])),
    );
    c.bench(
        "display",
        customize_benchmark(display_benches(&BYTE_SIZES[..])),
    );
}

criterion_group!(benches, bench);
criterion_main!(benches);
