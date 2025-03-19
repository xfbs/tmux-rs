fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-link-lib=bsd");
    println!("cargo::rustc-link-lib=tinfo");
    println!("cargo::rustc-link-lib=event_core");
    println!("cargo::rustc-link-lib=m");
    println!("cargo::rustc-link-lib=resolv");
    // -ltmux_rs -ltinfo  -levent_core  -lm  -lresolv
    //

    let mut builder = &mut cc::Build::new();

    // let out_dir = std::env::var("OUT_DIR").unwrap();
    // let command = format!( "/bin/bash ./etc/ylwrap cmd-parse.y y.tab.c {out_dir}/cmd-parse.c y.tab.h `echo {out_dir}/cmd-parse.c | sed -e s/cc$/hh/ -e s/cpp$/hpp/ -e s/cxx$/hxx/ -e s/c++$/h++/ -e s/c$/h/` y.output cmd-parse.output -- byacc",);
    // builder = builder.file(std::path::PathBuf::from(out_dir).join("cmd-parse.c"));

    for f in FILES {
        println!("cargo::rerun-if-changed=../{f}");
        builder = builder.file(std::path::PathBuf::from("..").join(f))
    }
    builder = builder.file("../cmd-parse.c");

    // clang -DPACKAGE_NAME=\"tmux\" -DPACKAGE_TARNAME=\"tmux\" -DPACKAGE_VERSION=\"3.5a\" -DPACKAGE_STRING=\"tmux\ 3.5a\" -DPACKAGE_BUGREPORT=\"\" -DPACKAGE_URL=\"\" -DPACKAGE=\"tmux\" -DVERSION=\"3.5a\"
    // -DHAVE_STDIO_H=1 -DHAVE_STDLIB_H=1 -DHAVE_STRING_H=1 -DHAVE_INTTYPES_H=1 -DHAVE_STDINT_H=1 -DHAVE_STRINGS_H=1 -DHAVE_SYS_STAT_H=1 -DHAVE_SYS_TYPES_H=1
    // -DHAVE_UNISTD_H=1 -DHAVE_WCHAR_H=1 -DSTDC_HEADERS=1 -D_ALL_SOURCE=1 -D_DARWIN_C_SOURCE=1 -D_GNU_SOURCE=1 -D_HPUX_ALT_XOPEN_SOCKET_API=1 -D_NETBSD_SOURCE=1 -D_OPENBSD_SOURCE=1 -D_POSIX_PTHREAD_SEMANTICS=1 -D__STDC_WANT_IEC_60559_ATTRIBS_EXT__=1 -D__STDC_WANT_IEC_60559_BFP_EXT__=1 -D__STDC_WANT_IEC_60559_DFP_EXT__=1 -D__STDC_WANT_IEC_60559_FUNCS_EXT__=1 -D__STDC_WANT_IEC_60559_TYPES_EXT__=1 -D__STDC_WANT_LIB_EXT2__=1 -D__STDC_WANT_MATH_SPEC_FUNCS__=1 -D_TANDEM_SOURCE=1 -D__EXTENSIONS__=1 -DHAVE_DIRENT_H=1 -DHAVE_FCNTL_H=1 -DHAVE_INTTYPES_H=1 -DHAVE_PATHS_H=1 -DHAVE_PTY_H=1 -DHAVE_STDINT_H=1 -DHAVE_SYS_DIR_H=1 -DHAVE_LIBM=1 -DHAVE_DIRFD=1 -DHAVE_FLOCK=1 -DHAVE_PRCTL=1 -DHAVE_SYSCONF=1 -DHAVE_ASPRINTF=1 -DHAVE_CFMAKERAW=1 -DHAVE_CLOCK_GETTIME=1 -DHAVE_CLOSEFROM=1 -DHAVE_EXPLICIT_BZERO=1 -DHAVE_GETDTABLESIZE=1 -DHAVE_GETLINE=1 -DHAVE_MEMMEM=1 -DHAVE_SETENV=1 -DHAVE_STRCASESTR=1 -DHAVE_STRNDUP=1 -DHAVE_STRSEP=1 -DHAVE_EVENT2_EVENT_H=1 -DHAVE_NCURSES_H=1 -DHAVE_TIPARM=1 -DHAVE_B64_NTOP=1 -DHAVE_MALLOC_TRIM=1 -DHAVE_DAEMON=1 -DHAVE_FORKPTY=1 -DHAVE___PROGNAME=1 -DHAVE_PROGRAM_INVOCATION_SHORT_NAME=1 -DHAVE_PR_SET_NAME=1 -DHAVE_SO_PEERCRED=1 -DHAVE_PROC_PID=1 -I.  -D_DEFAULT_SOURCE -D_XOPEN_SOURCE=600 -DTMUX_VERSION='"3.5a"' -DTMUX_CONF='"/etc/tmux.conf:~/.tmux.conf:$XDG_CONFIG_HOME/tmux/tmux.conf:~/.config/tmux/tmux.conf"' -DTMUX_LOCK_CMD='"lock -np"' -DTMUX_TERM='"tmux-256color"' -DDEBUG -iquote.       -fsanitize=address -fno-omit-frame-pointer -O0 -std=gnu99 -g -Wno-long-long -Wall -W -Wformat=2 -Wmissing-prototypes -Wstrict-prototypes -Wmissing-declarations -Wwrite-strings -Wshadow -Wpointer-arith -Wsign-compare -Wundef -Wbad-function-cast -Winline -Wcast-align -Wdeclaration-after-statement -Wno-pointer-sign -Wno-attributes -Wno-unused-result -Wno-format-y2k    -MT compat/fdforkpty.o -MD -MP -MF $depbase.Tpo -c -o compat/fdforkpty.o compat/fdforkpty.c &&\

    builder
        .define("HAVE_STDIO_H", "1")
        .define("HAVE_STDLIB_H", "1")
        .define("HAVE_STRING_H", "1")
        .define("HAVE_INTTYPES_H", "1")
        .define("HAVE_STDINT_H", "1")
        .define("HAVE_STRINGS_H", "1")
        .define("HAVE_SYS_STAT_H", "1")
        .define("HAVE_SYS_TYPES_H", "1")
        .define("HAVE_UNISTD_H", "1")
        .define("HAVE_WCHAR_H", "1")
        .define("STDC_HEADERS", "1")
        .define("_ALL_SOURCE", "1")
        .define("_DARWIN_C_SOURCE", "1")
        .define("_GNU_SOURCE", "1")
        .define("_HPUX_ALT_XOPEN_SOCKET_API", "1")
        .define("_NETBSD_SOURCE", "1")
        .define("_OPENBSD_SOURCE", "1")
        .define("_POSIX_PTHREAD_SEMANTICS", "1")
        .define("__STDC_WANT_IEC_60559_ATTRIBS_EXT__", "1")
        .define("__STDC_WANT_IEC_60559_BFP_EXT__", "1")
        .define("__STDC_WANT_IEC_60559_DFP_EXT__", "1")
        .define("__STDC_WANT_IEC_60559_FUNCS_EXT__", "1")
        .define("__STDC_WANT_IEC_60559_TYPES_EXT__", "1")
        .define("__STDC_WANT_LIB_EXT2__", "1")
        .define("__STDC_WANT_MATH_SPEC_FUNCS__", "1")
        .define("_TANDEM_SOURCE", "1")
        .define("__EXTENSIONS__", "1")
        .define("HAVE_DIRENT_H", "1")
        .define("HAVE_FCNTL_H", "1")
        .define("HAVE_INTTYPES_H", "1")
        .define("HAVE_PATHS_H", "1")
        .define("HAVE_PTY_H", "1")
        .define("HAVE_STDINT_H", "1")
        .define("HAVE_SYS_DIR_H", "1")
        .define("HAVE_LIBM", "1")
        .define("HAVE_DIRFD", "1")
        .define("HAVE_FLOCK", "1")
        .define("HAVE_PRCTL", "1")
        .define("HAVE_SYSCONF", "1")
        .define("HAVE_ASPRINTF", "1")
        .define("HAVE_CFMAKERAW", "1")
        .define("HAVE_CLOCK_GETTIME", "1")
        .define("HAVE_CLOSEFROM", "1")
        .define("HAVE_EXPLICIT_BZERO", "1")
        .define("HAVE_GETDTABLESIZE", "1")
        .define("HAVE_GETLINE", "1")
        .define("HAVE_MEMMEM", "1")
        .define("HAVE_SETENV", "1")
        .define("HAVE_STRCASESTR", "1")
        .define("HAVE_STRNDUP", "1")
        .define("HAVE_STRSEP", "1")
        .define("HAVE_EVENT2_EVENT_H", "1")
        .define("HAVE_NCURSES_H", "1")
        .define("HAVE_TIPARM", "1")
        .define("HAVE_B64_NTOP", "1")
        .define("HAVE_MALLOC_TRIM", "1")
        .define("HAVE_DAEMON", "1")
        .define("HAVE_FORKPTY", "1")
        .define("HAVE___PROGNAME", "1")
        .define("HAVE_PROGRAM_INVOCATION_SHORT_NAME", "1")
        .define("HAVE_PR_SET_NAME", "1")
        .define("HAVE_SO_PEERCRED", "1")
        .define("DEBUG", None)
        .define("HAVE_PROC_PID", "1")
        .define(
            "TMUX_CONF",
            "\"/etc/tmux.conf:~/.tmux.conf:$XDG_CONFIG_HOME/tmux/tmux.conf:~/.config/tmux/tmux.conf\"",
        )
        .define("TMUX_TERM", "\"tmux-256color\"")
        //
        .define("b64_ntop", "__b64_ntop")
        .define("b64_pton", "__b64_pton")
        //
        .flag("-fsanitize=address")
        .flag("-fno-omit-frame-pointer")
        .flag("-O0")
        .flag("-std=gnu99")
        .flag("-g")
        .flag("-Wno-long-long")
        .flag("-Wall")
        .flag("-W")
        .flag("-Wformat=2")
        .flag("-Wmissing-prototypes")
        .flag("-Wstrict-prototypes")
        .flag("-Wmissing-declarations")
        .flag("-Wwrite-strings")
        .flag("-Wshadow")
        .flag("-Wpointer-arith")
        .flag("-Wsign-compare")
        .flag("-Wundef")
        .flag("-Wbad-function-cast")
        .flag("-Winline")
        .flag("-Wcast-align")
        .flag("-Wdeclaration-after-statement")
        .flag("-Wno-pointer-sign")
        .flag("-Wno-attributes")
        .flag("-Wno-unused-result")
        .flag("-Wno-format-y2k")
        .compile("foo");
}

static FILES: &[&str] = &[
    "format-draw.c",
    "format.c",
    "grid.c",
    "input.c",
    "layout.c",
    "mode-tree.c",
    // "options-table.c",
    "options.c",
    "screen-write.c",
    "server-client.c",
    "status.c",
    "tty-keys.c",
    "tty.c",
    "window-copy.c",
    "window-customize.c",
    "window-tree.c",
];

// removed:
// "session.c",
