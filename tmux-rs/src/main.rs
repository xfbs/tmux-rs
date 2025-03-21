#![no_main]

use ::std::alloc::System;
#[global_allocator]
static A: System = System;

use ::tmux_rs::main;
