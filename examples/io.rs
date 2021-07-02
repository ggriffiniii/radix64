/// Very simple example that can either encode or decode stdin and print to stdout.
/// $ echo foo | cargo run --example io
/// Zm9vCg==
///
/// $ echo foo | cargo run --example io | cargo run --example io -- -d
/// foo
///
use radix64::{
    io::{DecodeReader, EncodeWriter},
    STD as MY_CONFIG,
};
use std::{env, error::Error, io, iter::FromIterator};

enum Mode {
    Encode,
    Decode,
}

// for really terrible argument parsing.
impl FromIterator<String> for Mode {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut mode = Mode::Encode;
        for arg in iter {
            match arg.as_str() {
                "-d" => mode = Mode::Decode,
                "-e" => mode = Mode::Encode,
                _ => {}
            }
        }
        mode
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mode: Mode = env::args().collect();

    match mode {
        Mode::Encode => {
            let mut dst = EncodeWriter::new(MY_CONFIG, io::stdout());
            let mut src = io::stdin();
            io::copy(&mut src, &mut dst)?;
            dst.finish()?;
        }
        Mode::Decode => {
            let mut dst = io::stdout();
            let mut src = DecodeReader::new(MY_CONFIG, io::stdin());
            io::copy(&mut src, &mut dst)?;
        }
    }
    Ok(())
}
