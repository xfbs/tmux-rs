#!/bin/bash
set -xe
make clean
sh autogen.sh && ./configure
cargo build --release
make
