#!/bin/bash
set -xe
# make clean
# sh autogen.sh && ./configure
# cargo build --release
# make

cargo clean
make clean
sh autogen.sh && ./configure --enable-debug
cargo build
make
