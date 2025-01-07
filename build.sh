#!/bin/bash
set -xe

# sh autogen.sh
# ./configure && make

# see Makefile.am
# for list of files
# dist_tmux_SOURCES
# dist_tmux_OBJECTS
# just modify LIBS in Makefile
# LIBS = -ltinfo  -levent_core  -lm  -lresolv -L/home/collin/Git/tmux/tmux-3.5a/target/release -llog -lxmalloc -lcmd_kill_server
# consider also adding -Wl,path so that we don't need to set LD_LIBRARY_PATH when running
cargo build --release
make
# run with LD_LIBRARY_PATH=./target/release
