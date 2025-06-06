> [!WARNING]
> This project is alpha quality and has known memory bugs.

# tmux-rs

A rust port of [tmux](https://github.com/tmux/tmux).

## Why?

Why not? This a fun hobby project for me. It's been my gardening for the past year.

I started this project as a way of getting first hand experience using C2Rust.
It's simultaneously a great and a terrible tool. I was amazed when I used it that it
was able to produce Rust code which compiled to a binary which was effectively equivalent to the original C binary.
Unfortunately, the quality of the resulting rust code ... leaves a lot to be desired.
It doesn't always retain the intent of the original code, despite being equivalent.
After discovering this I opted to complete the porting process by hand. The bulk of the work
was done without AI (llm) assistance. When I was about 70% complete I started integrating using
Cursor into my workflow for porting some files.

## Why not just use [zellij](https://zellij.dev/)

I like tmux. I want tmux, not something else. Also I tried out using it before and the compilation time was 8 minutes on my machine.
That's a bit to long for me.

### Files Remaining

I'm in the home stretch of the first part of this project. After finishing translating these files I will begin
refactoring the Rust code to make it more idiomatic and use less unsafe.

- [ ] 3186 tty
- [ ] 3392 server-client
- [ ] 5786 window-copy
- [ ]  159 cmd-parse.y (partially translated), need to figure out an approach to get rid of yacc/bison

## Tips

- Use clang-format or other to reformat the C code quickly
- Picking a C file: Start with root files in the project. (files with no or few dependencies on the rest of the project)
- You cannot link multiple static rust libraries (.a) into a single compilation artifact. There will be duplicate symbols.
- Seems a common source of bugs is stubbing something and intending to come back to it later, but not. Avoid this.
- SEGFAULTS, crashes, panics are much easier to debug then infinite loops and other types of bugs because you get a stack trace
- export ASAN_OPTIONS=log_path=asan , log asan issues to file because tmux owns stdout,stderr

## Debugging Tips
- reduce problem to single function. set a breakpoint on that function. walk through it in old and new version and notice differences
- if crashing on user action, start, get pid of second process; attach gdb on second process without follow child mode; continue; trigger action, hopefully gdb will be at the point where the crash occurred

# Progress

need to be able to get some more useful information when.
more then just server exited unexpectedly.
- figure out abort / panic logs
- print a stacktrace on server process segfault

# NEXT
- [ ] verify all uses of tailq and rbq, any structs with multiple "entry" fields we use correctly
- [ ] LICENSE stuff
- [ ] use base64 crate instead of libresolv
- [ ] improve interface on variadics
    - args_print_add
    - cfg_add_cause
    - cmd_log_argv
    - cmdq_add_format
    - cmdq_error
    - cmdq_insert_hook
    - cmdq_print
    - control_write
    - environ_log
    - environ_set
    - fatal
    - fatal
    - fatalx
    - fatalx_c
    - file_error
    - file_print
    - format_add
    - format_log1
    - format_printf
    - fprintf
    - input_reply
    - log_debug
    - log_debug
    - log_debug_c
    - options_set_string
    - screen_write_nputs
    - screen_write_puts
    - screen_write_strlen
    - screen_write_text
    - server_add_message
    - status_message_set
    - xasprintf
    - xasprintf
    - xasprintf_
    - xsnprintf

# TODO
- review cmd_rotate_window.rs cmd_rotate_window_exec tailq_foreach calls
- memory sanitizer
- dump backtrace on abort
  - gdb break
    - client_main
    - client_connect
    - server_start
    - proc_fork_and_daemon
    - proc loop server loop
- better rust format string style logging functions
- tailq and rbtree
  - recheck all tailq, and rbtree structs for multiple links.
  - tailq support new generic type discriminant
  - fully complete library / crate implementation with documentation
- use bitflags instead of manually 
- implement fatal and fatalx which accept static rust string
- consider enum usage
- fix commented out debug lines because I was too lazy to translate properly

## After 100% Rust

- migrate to stable rust
  - remove usages of c_variadics or help stabilize
- coverage
- convert to references instead of pointers
  - requirements to convert pointer to reference <https://doc.rust-lang.org/core/ptr/index.html#pointer-to-reference-conversion>
    - interleaving accesses between refs and ptrs seems to be not allowed
      - <https://doc.rust-lang.org/nightly/core/ptr/index.html#safety>
      - <https://github.com/rust-lang/unsafe-code-guidelines/issues/463>
    - when converting from ptr to ref need to ensure types are initialized and valid when passed into a function
    - read or writes through a ptr will invalidate a reference
    - also need to ensure no pointers are created and stored from the references
    - NonNull use as_uninit_mut
- get rid of paste crate, won't need to join symbols any more for C code
- implement cmd-parse.y parser in pest or nom to remove yacc as a build dependency
- performance: perf command like: perf record -F 99 -i -p 696418 -p 696420
- lints
- miri (too many libc functions, maybe possible for tests)
- eliminate libbsd and other libc functions (use rust equivalent)
- write compatibility tests to validate structs with existing C structs
- misc rust refactoring:
  - keyc should just be u64
- refactor to get rid of:
  - xasprintf ?
  - any function which accepts *mut c_void pointers to use generics
  - xreallocarray (use rust vec)

# Thoughts & Ideas
- emulate rust scoped enums with modules, structs and constants
- better rust-analyzer integration with C code

# Interesting Patterns

- goto labeled block translation
- bitflags

# C Stuff
- integer promotion rules
- rust literal value inference
- prototypes
- variadics
- function pointer equality comparison

# Notes

## Compat

tmux is a *bsd project.
I'm not sure which bsd exactly, but it's clear from reading the source code there's many libc functions
used which don't exist on linux, and are provided by bsd. The tmux project makes use of code in the compat
directory and autotools to shim these functions on OS's which they aren't provided. The first area to port
is this. Many linux distro's provide some of these functions already implemented through a library called
libbsd. I made a libbsd-sys library that provides auto-generated rust bindings to this C library. The surface
area of these functions is quite small and could easily be reimplemented later to remove this dependency.

## queue.h and tree.h

The tmux project makes extensive use of macros in the `compat/queue.h` and `compat/tree.h` headers which
implement an intrusive linked list and intrusive red black tree. For the most part, I've been able
to mirror the implementations at the source level using Rust generics. This is a key area to get right.
The auto-generated expanded C macros generated a mess from this code. This code needs to be hand crafted
properly to make use of rust generics which is abi compatible with the original C code. Maybe in the future
it would make sense to instead make use of a crate which provides the same functionality such as [intrusive_collections](https://docs.rs/intrusive-collections/latest/intrusive_collections/).

## C pointer field access operator `->`

Once annoyance of porting C code which makes heavy use of pointers is having to convert uses of the `->` operator.
Rust has no such operator and pointers don't implement deref, so they must be translated to something like `(*w).field`.

For a bit, I thought I could implement by own smart pointer type which wrapped a `*mut T` or `NonNull` and also
implemented DerefMut. Unfortunately doing this requires that you can create a `&mut T` which would likely invoke
undefined behaviour in this context.

## Translation of goto in irreducible control flow

see tty-keys.c tty_key_next

## BUGS
- exiting an opened window (not the original one ) causes server exit when asan is enabled
- multiple issues due to untype safe logging translation
- sendmsg in client to server causes SIGPIPE to be handled and exit control loop
- TODO, noticed I flipped translation order of fields of args_parse struct. need to double check that all translations which use the initialization is correct
- Window borders are incorrect
- status command; junk after completed text; probably improperly terminated string

- redraw is broken with vim

## BUGS (found)

- Incorrect translation of do while
- Incorrect translation of != null check
- incorrect translation of self-referential struct (just used null to init because lazyness when translating)
- missing init for tailq in struct // the big one causing crash on init
- missing break at end of loop emulating goto in rb_remove: hangs on Ctrl-D
- missing field in struct translation
- incorrect field in struct. used struct instead of struct pointer
- flipped == args_type::ARGS_NONE instead of flipped != args_type::ARGS_NONE
- flipped != 0 instead of == 0 for coverting from !int_like_value in conditional
- incorrect translation of for loop with continue to while with continue and increment at end; increment isn't applied (cmd_find)
- incorrect translation of cmd_entry args_parse cb None, when should have been Some(cb): after translating cmd-display-menu immediately aborts on start
- typo in rb_right macro, expanded to access left field
- crashes when config file is completely commented out: missing early exit in cmdq_get_command, no return in function
- Due to use after shadowing in client_.rs client_connect xasprintf usage found by using LSAN_OPTIONS=report_objects=1 leak on exit:
- memcpy_(&raw mut tmp as *mut i8, in_, end); should have been: memcpy_(tmp, in_, end)
  -  because I switched to a pointer instead of buffer,but didn't change memcpy code
- typo fps, fsp, variable unused null , cmd-queue.c ( causing crash when C-b t for clock)
- missing C prototype :struct cmd_parse_commands * cmd_parse_do_buffer(const char *buf, size_t len, struct cmd_parse_input *pi, char **cause)
  - return address value truncated to int
- for loop never entered didn't init variable needed for side effect after for loop ended (arguments)
- incorrect for loop translation. used 1..count, but should have used while loop
- extra copy paste: duplicate value += 1; value +=1;
- flipped null check
- flipped : char		 acs[UCHAR_MAX + 1][2]; -> pub acs: [[c_char; c_uchar::MAX as usize + 1]; 2], should be [[c_char; 2]; c_uchar::MAX as usize + 1],
- crashes when typing 'c' because of one of my bindings (seems something fixed this)
- linking problem
  - found many bugs in imsg and imsg-buffer implementation, seem to have not been caught when implemented due to how symbols are resolved
    - completely retranslate imsg and imsg-buffer
  - I suspect that linking is shadowing some broken rust implementations, and maybe when compiling with rust the rust implementation is preferred
    - it's interesting I notice differences in behavior between debug and release for this
- incorrect terminal behavior ; typed keys not displayed properly; lowercased ascii value in table when should be upper (likely caused by vim mistype u in visual)
- incorrect terminal behavior ; bad flag check should be valu == 0, but i did !value != 0)


# References

- [tmux](https://github.com/tmux/tmux)
- [C2Rust](https://github.com/immunant/c2rust)
- [rust-bindgen](https://rust-lang.github.io/rust-bindgen/)
- [Compiling C to Safe Rust, Formalized](https://arxiv.org/abs/2412.15042)
- [Porting C to Rust for a Fast and Safe AV1 Media Decoder](https://www.memorysafety.org/blog/porting-c-to-rust-for-av1/)
- [Fish 4.0: The Fish Of Theseus](https://fishshell.com/blog/rustport/)
- [Immunant's C2Rust tmux](https://github.com/immunant/tmux-rs)
- [Improved C Variadics in Rust and C2Rust](https://immunant.com/blog/2019/09/variadics/)
- [$20,000 rav1d AV1 Decoder Performance Bounty](https://www.memorysafety.org/blog/rav1d-perf-bounty/)
- [Making the rav1d Video Decoder 1% Faster](https://ohadravid.github.io/posts/2025-05-rav1d-faster/)
- [Multiple Security Issues in Screen](https://www.openwall.com/lists/oss-security/2025/05/12/1)
- [A 10x Faster TypeScript](https://devblogs.microsoft.com/typescript/typescript-native-port/)

