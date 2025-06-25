fn main() {
    println!("cargo::rerun-if-changed=src/grammar.lalrpop");
    lalrpop::process_root().unwrap();

    println!("cargo::rerun-if-changed=build.rs");

    println!("cargo::rustc-link-lib=bsd");
    // symbols used by libbsd:
    // - [ ] setproctitle
    // - [ ] setproctitle_init
    // - [ ] recallocarray
    // - [ ] freezero
    // - [ ] strunvis
    // - [ ] vis
    // - [ ] strnvis
    // - [ ] stravis
    // - [ ] fgetln
    // - [x] strlcat
    // - [x] strlcpy

    println!("cargo::rustc-link-lib=tinfo");
    // symbols used from tinfo:
    // - cur_term
    // - del_curterm
    // - setupterm
    // - tigetflag
    // - tigetnum
    // - tigetstr
    // - tparm

    println!("cargo::rustc-link-lib=event_core");
    // -ltmux_rs -ltinfo  -levent_core  -lm  -lresolv
}
