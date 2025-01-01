#!/bin/bash
set -xe

# sh autogen.sh
# ./configure && make

# see Makefile.am
# for list of files
# dist_tmux_SOURCES
# just modify libs in Makefile
# LIBS = -ltinfo  -levent_core  -lm  -lresolv -L/home/collin/Git/tmux/tmux-3.5a/target/release -llog -lxmalloc
cargo build --release
make
# run with LD_LIBRARY_PATH=./target/release
