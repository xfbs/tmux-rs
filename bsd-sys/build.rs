// man 7 libbsd
// https://rust-lang.github.io/rust-bindgen/tutorial-3.html

const ITEMS: &[&str] = &[
    "VIS_CSTYLE",
    "VIS_DQ",
    "VIS_GLOB",
    "VIS_NL",
    "VIS_OCTAL",
    "VIS_TAB",
    // ...
];
const VARS: &[&str] = &["optarg", "optind", "optreset"];
const FUNCS: &[&str] = &[
    "bsd_getopt",
    "closefrom",
    "fgetln",
    "freezero",
    "getopt",
    "getpeereid",
    "getprogname",
    "recallocarray",
    "setproctitle",
    "strlcat",
    "strlcpy",
    "strtonum",
    "vis",
    "stravis",
];

fn main() {
    println!("cargo:rustc-link-lib=bsd");

    let mut builder = bindgen::Builder::default().header("wrapper.h");

    for func in FUNCS {
        builder = builder.allowlist_function(func);
    }

    for item in ITEMS {
        builder = builder.allowlist_item(item);
    }

    for var in VARS {
        builder = builder.allowlist_var(var);
    }

    let bindings = builder
        .merge_extern_blocks(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
