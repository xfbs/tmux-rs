// man 7 libbsd
fn main() {
    println!("cargo:rustc-link-lib=bsd");

    // https://rust-lang.github.io/rust-bindgen/tutorial-3.html
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .allowlist_function("bsd_getopt")
        .allowlist_function("closefrom")
        .allowlist_function("fgetln")
        .allowlist_function("freezero")
        .allowlist_function("getopt")
        .allowlist_function("getpeereid")
        .allowlist_function("getprogname")
        .allowlist_function("recallocarray")
        .allowlist_function("setproctitle")
        .allowlist_function("strlcat")
        .allowlist_function("strlcpy")
        .allowlist_function("strtonum")
        .allowlist_var("optarg")
        .allowlist_var("optind")
        .allowlist_var("optreset")
        .merge_extern_blocks(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
