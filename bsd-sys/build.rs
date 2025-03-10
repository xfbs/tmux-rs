// man 7 libbsd
// https://rust-lang.github.io/rust-bindgen/tutorial-3.html

const ITEMS: &[&str] = &[
    "VIS_CSTYLE",
    "VIS_DQ",
    "VIS_GLOB",
    "VIS_NL",
    "VIS_OCTAL",
    "VIS_TAB",
    "bsd_getopt",
    "closefrom",
    "fgetln",
    "freezero",
    "getopt",
    "getpeereid",
    "getprogname",
    "optarg",
    "optind",
    "optreset",
    "recallocarray",
    "setproctitle",
    "stravis",
    "strlcat",
    "strlcpy",
    "strnvis",
    "strtonum",
    "strunvis",
    "vis",
];

fn main() {
    println!("cargo:rustc-link-lib=bsd");

    let mut builder = bindgen::Builder::default();
    for item in ITEMS {
        builder = builder.allowlist_item(item);
    }

    let bindings = builder
        .header("wrapper.h")
        .layout_tests(false)
        .merge_extern_blocks(true)
        .rust_edition(bindgen::RustEdition::Edition2024)
        .rust_target(bindgen::RustTarget::nightly())
        .use_core()
        .wrap_unsafe_ops(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
