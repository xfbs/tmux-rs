#!/bin/bash
set -xe
# export RUSTFLAGS=-Zsanitizer=address
# export RUSTFLAGS="-Zsanitizer=address -C link-arg=-fsanitize=address"

export CC=clang
rm -f tmux
cargo build
make
