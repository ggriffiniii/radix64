#!/bin/bash

set -ex

cargo doc

cargo build --no-default-features

if [[ ${TRAVIS_RUST_VERSION} == "1.31.0" ]]; then
  cargo test --no-default-features
fi

cargo build

if [[ ${TRAVIS_RUST_VERSION} == "1.31.0" ]]; then
  cargo test
fi
