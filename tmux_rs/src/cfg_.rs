use super::*;

unsafe extern "C" {
    pub unsafe static mut cfg_finished: c_int;
    pub unsafe static mut cfg_client: *mut client;
    pub unsafe static mut cfg_files: *mut *mut c_char;
    pub unsafe static mut cfg_nfiles: c_uint;
    pub unsafe static mut cfg_quiet: c_int;
    pub unsafe fn start_cfg();
    pub unsafe fn load_cfg(
        _: *const c_char,
        _: *mut client,
        _: *mut cmdq_item,
        _: *mut cmd_find_state,
        _: c_int,
        _: *mut *mut cmdq_item,
    ) -> c_int;
    pub unsafe fn load_cfg_from_buffer(
        _: *const c_void,
        _: usize,
        _: *const c_char,
        _: *mut client,
        _: *mut cmdq_item,
        _: *mut cmd_find_state,
        _: c_int,
        _: *mut *mut cmdq_item,
    ) -> c_int;
    pub unsafe fn cfg_add_cause(_: *const c_char, ...);
    pub unsafe fn cfg_print_causes(_: *mut cmdq_item);
    pub unsafe fn cfg_show_causes(_: *mut session);
}
