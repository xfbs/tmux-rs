const ALLOW_LIST: &[&str] = &[
    "EVLOOP_ONCE",
    "EV_PERSIST",
    "EV_READ",
    "EV_SIGNAL",
    "EV_TIMEOUT",
    "EV_WRITE",
    "SIZE_MAX",
    "bufferevent",
    "bufferevent_disable",
    "bufferevent_enable",
    "bufferevent_free",
    "bufferevent_get_output",
    "bufferevent_new",
    "bufferevent_setwatermark",
    "bufferevent_write",
    "bufferevent_write_buffer",
    "evbuffer",
    "evbuffer_add",
    "evbuffer_drain",
    "evbuffer_eol_style",
    "evbuffer_eol_style_EVBUFFER_EOL_LF",
    "evbuffer_free",
    "evbuffer_get_length",
    "evbuffer_new",
    "evbuffer_pullup",
    "evbuffer_readln",
    "event",
    "event_active",
    "event_add",
    "event_base",
    "event_del",
    "event_get_method",
    "event_get_version",
    "event_initialized",
    "event_loop",
    "event_once",
    "event_pending",
    "event_reinit",
    "event_set",
    "event_set_log_callback",
];

const BLOCK_LIST: &[&str] = &[
    "EVLOOP_NONBLOCK",
    "EVLOOP_NO_EXIT_ON_EMPTY",
    "EVLOOP_ONCE",
    "EV_CLOSED",
    "EV_ET",
    "EV_FINALIZE",
    "EV_PERSIST",
    "EV_READ",
    "EV_SIGNAL",
    "EV_TIMEOUT",
    "EV_WRITE",
    "evbuffer_add_printf",
    "evbuffer_add_vprintf",
    "timeval",
];

fn main() {
    println!("cargo:rustc-link-lib=event_core");
    println!("cargo:rerun-if-changed=wrapper.h");

    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        .blocklist_item("IPPORT_RESERVED");
    for allow in ALLOW_LIST {
        builder = builder.allowlist_item(allow);
    }
    for block in BLOCK_LIST {
        builder = builder.blocklist_item(block);
    }

    let bindings = builder
        .rust_target(bindgen::RustTarget::nightly()) // 2024 isn't supported in bindgen latest stable yet, and panics
        .rust_edition(bindgen::RustEdition::Edition2024)
        .layout_tests(false)
        .merge_extern_blocks(true)
        .wrap_unsafe_ops(true)
        .use_core()
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // bindings .write_to_file(std::path::PathBuf::from("out.rs")) .expect("Couldn't write bindings!");
}
