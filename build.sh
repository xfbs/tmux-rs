#!/bin/bash
set -xe

# sh autogen.sh
# ./configure && make

# see Makefile.am
# for list of files
# dist_tmux_SOURCES
# just modify libs in Makefile
# LIBS = -ltinfo  -levent_core  -lm  -lresolv -llog -lxmalloc
cargo build
make
# run with LD_LIBRARY_PATH=./target/release
