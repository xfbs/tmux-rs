---
layout: post
title: Introducing tmux-rs (draft)
author: Collin Richards
date: 2025-06-25
---
For the 6 months or so I've been quietly porting tmux from C to Rust. I've recently reached a big milestone: the code base is now 100% (unsafe) Rust.

I'd like to share the process of porting the original codebase from ~67,000 lines of C code to ~81,000 lines of Rust (excluding comments and empty lines).

You might be asking: why did you rewrite tmux in Rust? And yeah, I don't really have a good reason. It's a hobby project. Like gardening, but with more segfaults.

{% toc %}

- [Starting with C2Rust](#starting-with-c2rust)
- [Build process](#build-process)
- [Interesting Bugs](#interesting-bugs)
- C vs Rust semantics
  - pointers
  - intrusive data structures
  - goto
- Development Process
    - vim
    - ai (llms)
    - debuggers
- interesting inflection points
- Misc. translation bugs


## Starting with C2Rust

I started this project as a way of trying out [C2Rust](https://github.com/immunant/c2rust), a C to Rust transpiler. The tool was a little tricky to set up, but once it was running the generated output was a successful port of the tmux codebase in Rust.

Despite the generated code working, it was basically unmaintainable and 3x larger than the original C. You wouldn't want to touch it with a 10 foot pole. Here's an example of the input and output:

```c
// original C code
int colour_palette_get(struct colour_palette *p, int c) {
  if (p == NULL)
    return (-1);

  if (c >= 90 && c <= 97)
    c = 8 + c - 90;
  else if (c & y)
    c &= ~COLOUR_FLAG_256;
  else if (c >= 8)
    return (-1);

  if (p->palette != NULL && p->palette[c] != -1)
    return (p->palette[c]);
  if (p->default_palette != NULL && p->default_palette[c] != -1)
    return (p->default_palette[c]);
  return (-1);
}
```

```rust
// generated Rust code
#[no_mangle]
pub unsafe extern "C" fn colour_palette_get(
    mut p: *mut colour_palette,
    mut c: libc::c_int,
) -> libc::c_int {
    if p.is_null() {
        return -(1 as libc::c_int);
    }
    if c >= 90 as libc::c_int && c <= 97 as libc::c_int {
        c = 8 as libc::c_int + c - 90 as libc::c_int;
    } else if c & 0x1000000 as libc::c_int != 0 {
        c &= !(0x1000000 as libc::c_int);
    } else if c >= 8 as libc::c_int {
        return -(1 as libc::c_int)
    }
    if !((*p).palette).is_null()
        && *((*p).palette).offset(c as isize) != -(1 as libc::c_int)
    {
        return *((*p).palette).offset(c as isize);
    }
    if !((*p).default_palette).is_null()
        && *((*p).default_palette).offset(c as isize) != -(1 as libc::c_int)
    {
        return *((*p).default_palette).offset(c as isize);
    }
    return -(1 as libc::c_int);
}
```

This is a pretty simple example, but things can get a lot worse. My main concern was losing information from named constants like `COLOUR_FLAG_256`. Having that translated to `0x1000000` is an example of the kind of information loss that frequently happens during the transpilation process.

There are a lot of casts to `libc::c_int` polluting the code as well. I suspect this is to handle [C's integer promotion rules](https://stackoverflow.com/a/46073296). Most of them are completely unnecessary when operating on literals in Rust.

I spent quite a lot of time manually refactoring the shitty Rust code to less shitty Rust code, but I kept finding myself having to look at the original C code to understand the program's intent. Here's a cleaned up Rust version of that function. Still not perfect, but much better in my opinion:

```rust
/// Get a colour from a palette.
pub unsafe fn colour_palette_get(p: *const colour_palette, mut c: i32) -> i32 {
    unsafe {
        if p.is_null() {
            return -1;
        } else if (90..=97).contains(&c) {
            c = 8 + c - 90;
        } else if c & COLOUR_FLAG_256 != 0 {
            c &= !COLOUR_FLAG_256;
        } else if c >= 8 {
            return -1;
        }

        let c = c as usize;

        if !(*p).palette.is_null() && *(*p).palette.add(c) != -1 {
            *(*p).palette.add(c)
        } else if !(*p).default_palette.is_null() && *(*p).default_palette.add(c) != -1 {
            *(*p).default_palette.add(c)
        } else {
            -1
        }
    }
}
```

After manually refactoring many files this way I gave up with this approach. I threw away all of the C2Rust output and decided I would translate all of the files into Rust manually from C.

Despite not using C2Rust for this project I still think it's a great tool. It was very important for me to actually be able to compile and run the project from the start. It made me realize this endeavour was achievable. It's like starting at the finish line and going backwards.

## Build process

```
┌─────────────┐    ┌────────────┐     ┌──────────────┐    ┌──────────┐         ┌───────┐    
│ Makefile.am │───►│ autogen.sh ├────►│ configure.sh │───►│ Makefile │         │ cargo │    
└─────────────┘    └────────────┘     └──────────────┘    └──────────┘         └───┬───┘    
                                                                                   │        
                                                                                   │        
                                ┌──────┐       ┌──────┐                            │        
                           ┌───►│tmux.c├──────►│tmux.o├───────┐                    │        
               ┌──────┐    │    └──────┘       └──────┘       │                    │        
               │tmux.h├────┤                                  │                    │        
               └──────┘    │  ┌────────┐     ┌────────┐       │                    │        
                           ├─►│window.c├────►│window.o├───────┤                    │        
              ┌────────┐   │  └────────┘     └────────┘       │                    │        
              │compat.h├───┤                                  │                    │        
              └────────┘   │    ┌──────┐       ┌──────┐       │                    │        
                           └───►│pane.c├──────►│pane.o├───────┤                    ▼        
                                └──────┘       └──────┘       │             ┌──────────────┐
                                          ┌───────────┐       │    ┌────┐   │              │
                                          │           │       ├───►│tmux│◄──┤ libtmux_rs.a │
                                          │ libc.so.6 ├───────┤    └────┘   │              │
                                          │           │       │             └──────────────┘
                                          └───────────┘       │                             
                                      ┌───────────────┐       │                             
                                      │               │       │                             
                                      │ libtinfo.so.6 ├───────┘                             
                                      │               │                                     
                                      └───────────────┘                                     
```

I think the most important part of this rewrite was developing a solid understanding of how the project was built. This meant doing a bit of research on the bespoke build setup that every C project has. For tmux this is `autotools`. I figured out where I could remove files in `autogen.sh` and manually modified the generated `Makefile` to link in a shared library generated by my rust crate using the `crate-type = "staticlib"` option.

This did mean my build process wasn't as simple as just running cargo build. I wrote a small `build.sh` script which would invoke cargo, then run `make` using the modified `Makefile`. This worked for a while but any time I completed translating a file and removed a C file I had to reconfigure and re-modify the makefile.

The one of the first files I translated was `colour.c`. It's quite small, most of the functions are pure and self contained. Files which are leaves in the build graph are good candidates to rewrite first, such as `xmalloc.c` (a wrapper around malloc which aborts on failure).

Early on I would try to break things up into mini-crates. It ends up being easier to put everything in the same crate for two reasons. Crates can't have circular dependencies and you can run into linking issues when linking multiple Rust libraries into the same binary.

At first I would translate one file at a time, with no way to validate the changes when halfway through a file. After translating a rather large file and spending a long time debugging the issue down to a flipped conditional I changed the development process to translate function by function. This did mean adding headers in the C code for functions which were originally static. The new process looked like this:

```c
// copy the header of the C function
// and comment out the C function body

int colour_palette_get(struct colour_palette *p, int c);
// int colour_palette_get(struct colour_palette *p, int c) {
// ...
//
```

Then the code could be translated one function at a time. The C code
would link against the Rust implementation as long as the function had
the `#[unsafe(no_mangle)]` attribute `extern "C"` annotation and importantly the correct signature.

After translating about half of the C files I started thinking this build process was a bit silly. Most of the code was now in Rust. Maybe instead of building a C binary and linking in a Rust library I should be building a Rust binary and linking in a C library. Well that's exactly what you can use the cc crate to do.

I set up a build.rs like so:

```rust
// simplified version from: git show 4b82e3709029a6b9d3bd572db16aa136ed5992e2 -- tmux-rs/build.rs
fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-link-lib=bsd");
    println!("cargo::rustc-link-lib=tinfo");
    println!("cargo::rustc-link-lib=event_core");
    println!("cargo::rustc-link-lib=m");
    println!("cargo::rustc-link-lib=resolv");

    let mut builder = &mut cc::Build::new();

    static FILES: &[&str] = &[
        "osdep-linux.c",
        "cmd-new-session.c",
        "cmd-queue.c",
        // ...
        "window-customize.c",
        "window-tree.c",
    ];
    for f in FILES {
        builder = builder.file(std::path::PathBuf::from("..").join(f))
    }

    builder.compile("foo");
}

```

## Interesting Bugs

### Bug 1 (symbol resolution and linker shenanigans)

Eventually I can eliminate using the `cc` crate after all of the C files are translated.


linking issues with static variables.

```c
// imsg.c
int imsg_fd_overhead = 0;
```

```rust
// imsg.rs
static mut imsg_fd_overhead: i32 = 0;
```

Oh no! because we aren't using no_mangle there's two copies of the same variable (rust mangles the symbol so there isn't a conflict).

if we properly annotated the rust symbol:
```rust
// imsg.rs
#[unsafe(no_mangle)]
static mut imsg_fd_overhead: i32 = 0;
```

Then we would have gotten a linker error and would have found out we should
also modify the C code to be a declaration instead of a definition like so:

```c
// imsg.c
extern int imsg_fd_overhead;
```

Why this mattered when I changed the build system. At this point in development I was still working using a couple crates. The main project crate tmux-rs and a support crate called `compat_rs`. imsg.rs lived in `compat_rs`.

When switching the build process to use cargo, rust resolves the symbols in compat_rs using the rust library and not as a static library.

I built the code as a staticlib and a rust lib.


### Bug 2 (Mismatched struct definition)

I noticed this bug when translating the simplest function that shouldn't have caused a bug. It was something like this:


```c
void set_value(client* c) {
  c->foo = 5;
}
```

```rust
unsafe extern "C" fn set_value(c: *mut client) {
  unsafe {
    (*c).foo = 5;
  }
}
```

I was shocked that after translating this simple function the program started segfaulting. By inspecting it in the debugger showed that the segfault in the rust code was happening on that line, which should be identical to the C. So what's the issue. Well it just so happens that when I manually translated the type declaration of the client struct I missed an `*` on one of the types. This type was just above the data field. Meaning the C and Rust code had different views of the type after that mismatched field.


### Bug 3 (Mismatched function declaration)

One other interesting case. I translated a simple function which returned a pointer.


```c
void* get_addr(client* c) {
  return c->bar;
}
```

```rust
unsafe extern "C" fn get_addr(c: *mut client) -> *mut c_void {
  unsafe {
    (*c).bar
  }
}
```

This time after doing the translation I hastily commented out `get_addr`
in C and did my hacky `build.sh` compilation process with the usual compiler warnings.

This time I got another segmentation fault. Read at address `0x2764` something like that.
I walked through the code again in the debugger. Inside of the function `get_addr` `(*c).bar`
had a valid address, but it was something like `0x60302764`. But value received from the calling C code
was `0x2764`. Do you know the problem yet? Need another hint. If I looked more closely at the compilation
warnings I would have seen:

```
warning: implicit declaration of function ‘get_addr’ [-Wimplicit-function-declaration]
```

That's right, the C code was using the implicit declaration which is something like:

```c
int get_addr();
```

That explains why the value was incorrect, the C compiler was thinking a 4 byte value was returned not an 8 byte pointer.
So the top 4 bytes were being truncated or ignored. The fix was as simple as adding the correct prototype to the C code
and the compiler would generate the correct code for getting the return value.


I haven't yet talked much about actual Rust code. Only build processes. I'll start with what I think is a fundamental difference between Rust and C. Pointers vs. References.

Rust has two reference types: &T a shared reference or &mut T an exclusive or mutable reference. For anyone familiar with C++ or Java Rust reference have different semantics from these languages. A Rust reference is just an address with several other invariants. (An invariant is something that is always true. If you break one of these contracts then your code would be invoking undefined behavior and your computer will explode). One of the invariants is that references cannot be null and the value pointed to is fully initialized and valid.

The natural mapping of pointers in a C program would be a reference in rust, either exclusive or shared depending if it's modified in the code. The problem is, often times some of the invariants required by references in Rust cannot always be upheld if we do a straight one-to-one mapping of the source from C to Rust. That means we can't use rust references. We have to use another type provided by rust called raw pointers. Semantically raw pointers are the same as C pointers, but because you don't really use them outside of unsafe Rust they are extremely unergonomic to use. For example:

Again, I don't like this code. I much prefer C to this flavor of Rust, also known as cRust. Link up tsoding video.


There's two other interesting C language features which I want to discuss. The first is goto, the second isn't a language feature, but a macro header library for intrusive data structures.

### Goto

C has goto. People like to hate on goto, but actually even though it's used throughout the tmux codebase, only one or two of the usages actually cause implementation difficulties.

The c2rust transpiler uses an algorithm called relooper to emulate goto logic. However most cases don't actually require using this algorithm and can instead use a much simpler method.

For forward jumps a labeled block with a break statement is sufficient.

Backward jumps can be implemented using a labeled loop with continue and break statements at the end of the loop.

These are the most common usages in the form of goto error; and goto try_again; it's only when the location being jumped to is in the middle of a loop or oddly nested do things get tricky.

I think these are called irreducible blocks and research was done on them a while ago.

It's always fun to read a paper from the 70's to help with a problem you have now.

I spent a couple hours drawing out the control flow to figure out how to map it to only forward and back jumps.

I stumbled into a solution for my specific case before learning weather or not the case I had was truly irreducible. If I have a bit more time I'd like to study that a bit more, but I've already solved the task at hand so it's hard to justify spending more time on that detour.

### Macro header library

Tmux makes heavy use of two data structures defined in tree.h and queue.h. They are an intrusion red black tree and a linked list defined by macros. If you're not familiar, an intrusive data structure is one where the pieces of the data structure live within your struct. This is apposed to how most container data structures are implemented today where the container holds the unmodified struct and doesn't require support from the struct to hold data critical to a correct implementation.

For example:

A normal linked list might look like this:

An intrusive one might look like this:

I actually went through many iterations of implementing a good rust interface for mimicking the c code. This is what I ended up with and why:

## Development process

Throughout working on this project I used many different text editors and ides. I did start trying out cursor for part of the development process.

My typical workflow was using vim to do the translation heavily relying on custom macros to speed up the translation process.

I also found that making some edits while the file is still a C file also helped. For example I used clang format to add braces for all if statements (they can be implicit in C).

The things that I made vim macros were for things like converting == NULL to .is_null()

As stated earlier one of the more annoying things to fix was changing -> to .*

Most of these mechanical changes are very easy to make, but are hard to do all at once with a find and replace. This means doing it by hand thousands of times.

AI tooling really helps. I used it for translating functions at a time. It would occasionally insert bugs, just like me, so as much time needed to be spent reviewing the generated code as it would take to write it. The only thing it saved was my hands.

Doing this large amount of refactoring is really hard on your fingers. So even though I quit using cursor my feeling is that I'd still reach for it if my hands are really physically hurting, but I need to keep working. Usually when I reach that point I think it's better to just take a break though.

I'd love to talk more about refactoring tricks with different editors and IDEs, but I think it's actually more compelling to see that in action so I'll plan to make a video for that instead.

In this porting project there were many key inflection points. For example right now I'm trying to improve the code in general, fix clippy lints, reduce unsafe code, and usage of external c libraries.

A big portion of time was spent thinking how to remove the yacc dependency. Lalrpop really helped with that.

Well today I'm going to release version 0.0.1. I am aware of many bugs. For example just passing prefix + ? Will cause a crash / hang. Don't use it for anything important yet.

## Bugs

During the development process I like to make list of the bugs that I find. Whenever I find an issue due to something silly I'd add it to my list below. Part of this is so that I can avoid making the same mistakes again. These are some of the bugs which were introduced during the translation process. Feel free to skim the list, most of them are written in such a way which is only meaningful to myself. By far the most common was accidentally flipping a conditional during the translation.

- Incorrect translation of != null check (wrote x.is_null() instead of !x.is_null())
- Incorrect translation of do while
- Incorrect translation of self-referential struct (just used null to init because lazyness when translating)
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
- typo fps, fsp, variable unused null, cmd-queue.c ( causing crash when C-b t for clock)
- missing C prototype :struct cmd_parse_commands \* cmd_parse_do_buffer(const char \*buf, size_t len, struct cmd_parse_input \*pi, char \*\*cause)
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
- incorrect terminal behavior ; bad flag check should be value == 0, but i did !value != 0)
