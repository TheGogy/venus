#!/usr/bin/env bash
set -euo pipefail

build() {
    local name=$1
    local flags="-C target-feature=$2"
    local dir=target/$name

    env -u RUSTFLAGS \
        CARGO_TARGET_DIR="$dir" \
        RUSTFLAGS="$flags" \
        EVALFILE="$(pwd)/net.bin" \
        cargo build --release --features embed --bin cli

    cp "$dir"/release/cli test-"$name"
}

build base "-avx2,-avx512f"
build avx2 "+avx2,-avx512f"
build avx512 "+avx2,+avx512f"
