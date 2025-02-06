#!/bin/bash
set -xe
# make clean
# sh autogen.sh && ./configure
# cargo build --release
# make

# export RUSTFLAGS=-Zsanitizer=address
# export RUSTFLAGS="-Zsanitizer=address -C link-arg=-fsanitize=address"

export CC=clang
cargo clean
make clean
rm -f tmux
sh autogen.sh && ./configure --enable-debug
cargo build
make

# export CC=clang
# rm -f tmux
# cargo build
# make
