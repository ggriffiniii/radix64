#!/bin/bash

set -ex

cargo doc

cargo build --no-default-features
cargo test --no-default-features

cargo build
cargo test
