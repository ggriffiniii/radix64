//! Utilities for encoding and decoding from std::io::Read and std::io::Write.
//!
//! ### Received base64 encoded data from stdin, decode it, and print it to stdout.
//! ```
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use radix64::{STD, io::DecodeReader};
//! use std::io;
//!
//! let mut dst = io::stdout();
//! let mut src = DecodeReader::new(STD, io::stdin());
//! io::copy(&mut src, &mut dst)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Received data from stdin, encode it, and print it to stdout.
//! ```
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use radix64::{STD, io::EncodeWriter};
//! use std::io;
//! let mut dst = EncodeWriter::new(STD, io::stdout());
//! let mut src = io::stdin();
//! io::copy(&mut src, &mut dst)?;
//! dst.finish()?;
//! # Ok(())
//! # }
//! ```

pub use crate::decode::io::DecodeReader;
pub use crate::encode::io::{EncodeWriter, FinishError};
