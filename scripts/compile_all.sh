#!/usr/bin/env bash
set -euo pipefail

build() {
    local name=$1
    local flags="-C target-feature=$2"
    local dir=target/$name

    env -u RUSTFLAGS \
        CARGO_TARGET_DIR="$dir" \
        RUSTFLAGS="$flags" \
        EVALFILE="$(pwd)/nn-$(cat net.txt).bin" \
        cargo build --release --features embed --bin cli

    cp "$dir"/release/cli test-"$name"
}

build avx2 "+avx2,+fma,-avx512f"
build avx512 "+avx2,+fma,+avx512f,+avx512bw"
build vbmi2 "+avx2,+fma,+avx512f,+avx512bw,+avx512vbmi,+avx512vbmi2"
