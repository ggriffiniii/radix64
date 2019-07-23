#!/bin/bash

set -ex

cargo doc

cargo build --no-default-features

if [[ -z "${MSRV}" ]]; then
  cargo test --no-default-features
fi

cargo build

if [[ -z "${MSRV}" ]]; then
  cargo test
fi
