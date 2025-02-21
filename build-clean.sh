#!/bin/bash
set -xe
cargo clean
make clean
rm -f tmux

# export RUSTFLAGS=-Zsanitizer=address
export RUSTFLAGS="-Zsanitizer=address -C link-arg=-fsanitize=address"
export CC=clang

sh autogen.sh && ./configure --enable-debug
cargo build
make
