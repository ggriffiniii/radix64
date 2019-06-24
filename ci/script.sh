#!/bin/bash

set -ex

cargo build
cargo doc
cargo test
