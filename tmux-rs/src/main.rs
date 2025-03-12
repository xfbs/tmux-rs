#![no_main]
use ::core::ffi::c_char;
use ::std::ffi::CString;

use std::alloc::System;
#[global_allocator]
static A: System = System;

use ::tmux_rs::main;

// fn main() {
//     let argv: Vec<String> = std::env::args().map(|arg| arg).collect();
//     let args: Vec<CString> = std::env::args().map(|arg| CString::new(arg).unwrap()).collect();
//     let mut c_args: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
//     c_args.push(core::ptr::null());
//
//     let env: Vec<CString> = std::env::vars()
//         .map(|(key, value)| CString::new(format!("{key}={value}")).unwrap())
//         .collect();
//
//     let mut c_env: Vec<*const c_char> = env.iter().map(|e| e.as_ptr()).collect();
//     c_env.push(std::ptr::null()); // Add a null pointer at the end, as is customary in C for environment variable lists.
//
//     let argc = args.len() as i32;
//     unsafe {
//         ::tmux_rs::main(argc, c_args.as_mut_ptr().cast());
//     }
//
//     // drop args, env
// }
