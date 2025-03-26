use compat_rs::tree::rb_initializer;

use crate::*;

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct format_flags: i32 {
        const FORMAT_STATUS  = 1;
        const FORMAT_FORCE   = 2;
        const FORMAT_NOJOBS  = 4;
        const FORMAT_VERBOSE = 8;
    }
}

pub const FORMAT_NONE: i32 = 0;
pub const FORMAT_PANE: u32 = 0x80000000u32;
pub const FORMAT_WINDOW: u32 = 0x40000000u32;

pub type format_cb = Option<unsafe extern "C" fn(_: *mut format_tree) -> *mut c_void>;

#[rustfmt::skip]
unsafe extern "C" {
    pub unsafe fn format_tidy_jobs();
    pub unsafe fn format_skip(_: *const c_char, _: *const c_char) -> *const c_char;
    pub unsafe fn format_true(_: *const c_char) -> c_int;
    pub unsafe fn format_create(_: *mut client, _: *mut cmdq_item, _: c_int, _: format_flags) -> *mut format_tree;
    pub unsafe fn format_free(_: *mut format_tree);
    pub unsafe fn format_merge(_: *mut format_tree, _: *mut format_tree);
    pub unsafe fn format_get_pane(_: *mut format_tree) -> *mut window_pane;
    pub unsafe fn format_add(_: *mut format_tree, _: *const c_char, _: *const c_char, ...);
    pub unsafe fn format_add_tv(_: *mut format_tree, _: *const c_char, _: *mut timeval);
    pub unsafe fn format_add_cb(_: *mut format_tree, _: *const c_char, _: format_cb);
    pub unsafe fn format_log_debug(_: *mut format_tree, _: *const c_char);
    pub unsafe fn format_each( _: *mut format_tree, _: Option<unsafe extern "C" fn(_: *const c_char, _: *const c_char, _: *mut c_void)>, _: *mut c_void,);
    pub unsafe fn format_pretty_time(_: time_t, _: c_int) -> *mut c_char;
    pub unsafe fn format_expand_time(_: *mut format_tree, _: *const c_char) -> *mut c_char;
    pub unsafe fn format_expand(_: *mut format_tree, _: *const c_char) -> *mut c_char;
    pub unsafe fn format_single( _: *mut cmdq_item, _: *const c_char, _: *mut client, _: *mut session, _: *mut winlink, _: *mut window_pane,) -> *mut c_char;
    pub unsafe fn format_single_from_state( _: *mut cmdq_item, _: *const c_char, _: *mut client, _: *mut cmd_find_state,) -> *mut c_char;
    pub unsafe fn format_single_from_target(_: *mut cmdq_item, _: *const c_char) -> *mut c_char;
    pub unsafe fn format_create_defaults( _: *mut cmdq_item, _: *mut client, _: *mut session, _: *mut winlink, _: *mut window_pane,) -> *mut format_tree;
    pub unsafe fn format_create_from_state( _: *mut cmdq_item, _: *mut client, _: *mut cmd_find_state,) -> *mut format_tree;
    pub unsafe fn format_create_from_target(_: *mut cmdq_item) -> *mut format_tree;
    pub unsafe fn format_defaults( _: *mut format_tree, _: *mut client, _: *mut session, _: *mut winlink, _: *mut window_pane,);
    pub unsafe fn format_defaults_window(_: *mut format_tree, _: *mut window);
    pub unsafe fn format_defaults_pane(_: *mut format_tree, _: *mut window_pane);
    pub unsafe fn format_defaults_paste_buffer(_: *mut format_tree, _: *mut paste_buffer);
    pub unsafe fn format_lost_client(_: *mut client);
    pub unsafe fn format_grid_word(_: *mut grid, _: c_uint, _: c_uint) -> *mut c_char;
    pub unsafe fn format_grid_hyperlink(_: *mut grid, _: c_uint, _: c_uint, _: *mut screen) -> *mut c_char;
    pub unsafe fn format_grid_line(_: *mut grid, _: c_uint) -> *mut c_char;
}

// Entry in format job tree.
#[repr(C)]
pub struct format_job {
    pub client: *mut client,
    pub tag: u32,
    pub cmd: *mut c_char,
    pub expanded: *mut c_char,

    pub last: time_t,
    pub out: *mut c_char,
    pub updated: i32,

    pub job: *mut job,
    pub status: i32,

    pub entry: rb_entry<format_job>,
}

pub type format_job_tree = rb_head<format_job>;
pub static mut format_jobs: format_job_tree = rb_initializer();
