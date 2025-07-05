fn main() {
    println!("cargo::rerun-if-changed=src/grammar.lalrpop");
    lalrpop::process_root().unwrap();

    // TODO consider conditionally change based on os
    println!("cargo::rustc-link-lib=ncurses"); // ncurses is packaged on homebrew, but not libtinfo
    // println!("cargo::rustc-link-lib=tinfo");

    println!("cargo::rustc-link-lib=event_core");
}
