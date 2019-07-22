radix64
=======

[![](http://meritbadge.herokuapp.com/radix64)](https://crates.io/crates/radix64)
[![Docs](https://docs.rs/radix64/badge.svg)](https://docs.rs/radix64)
[![build status](https://api.travis-ci.org/ggriffiniii/radix64.svg)](https://travis-ci.org/ggriffiniii/radix64)

Fast and easy base64 encoding and decoding.

### Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
radix64 = "0.3"
```

### Examples

Encode and decode with the standard alphabet

```rust
use radix64::STD;
let encoded = STD.encode("my message");
let decoded = STD.decode(&encoded).unwrap();
assert_eq!("my message".as_bytes(), &decoded);
```

Encode and decode with the url safe alphabet

```rust
use radix64::URL_SAFE;
let encoded = URL_SAFE.encode("my message");
let decoded = URL_SAFE.decode(&encoded).unwrap();
assert_eq!("my message".as_bytes(), &decoded);
```

Encode multiple messages reusing the same allocated buffer for each message.
```rust
use radix64::STD;
let mut buf = Vec::new();
let encoded = STD.encode_with_buffer("my message", &buf);
let encoded = STD.encode_with_buffer("my second message", &buf);
```

Decode multiple messages reusing the same allocated buffer for each message.
```rust
use radix64::STD;
let mut buf = Vec::new();
let decoded = STD.decode_with_buffer("AABB", &buf);
let decoded = STD.decode_with_buffer("AA==", &buf);
```

Define and use a custom alphabet
```rust
use radix64::ConfigBuilder;
let my_cfg =
    ConfigBuilder::with_alphabet("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz#&")
        .no_padding()
        .build()
        .unwrap();
let encoded = my_cfg.encode("my message");
let decoded = my_cfg.decode(&encoded).unwrap();
```

### Performance

The standard alphabets (STD, URL_SAFE, and CRYPT) along with the NO_PAD variants
all have an AVX2 optimized encoder and decoder. This provides a huge performance
boost if running on an AVX2 enabled CPU. A runtime check will be performed by
default to see if AVX2 is available. If you specify compiling for an AVX2
enabled platform the runtime check will be avoided. If you want to avoid using
the AVX2 implementation you can disable the "simd" feature when compiling the
crate.

See a sample of benchmark runs [here](https://ggriffiniii.github.io/radix64/bench_results)
