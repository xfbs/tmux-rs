use super::*;

unsafe extern "C" {
    pub fn file_cmp(_: *mut client_file, _: *mut client_file) -> c_int;
    pub fn client_files_RB_INSERT_COLOR(_: *mut client_files, _: *mut client_file);
    pub fn client_files_RB_REMOVE_COLOR(_: *mut client_files, _: *mut client_file, _: *mut client_file);
    pub fn client_files_RB_REMOVE(_: *mut client_files, _: *mut client_file) -> *mut client_file;
    pub fn client_files_RB_INSERT(_: *mut client_files, _: *mut client_file) -> *mut client_file;
    pub fn client_files_RB_FIND(_: *mut client_files, _: *mut client_file) -> *mut client_file;
    pub fn client_files_RB_NFIND(_: *mut client_files, _: *mut client_file) -> *mut client_file;
    pub fn file_create_with_peer(
        _: *mut tmuxpeer,
        _: *mut client_files,
        _: c_int,
        _: client_file_cb,
        _: *mut c_void,
    ) -> *mut client_file;
    pub fn file_create_with_client(_: *mut client, _: c_int, _: client_file_cb, _: *mut c_void) -> *mut client_file;
    pub fn file_free(_: *mut client_file);
    pub fn file_fire_done(_: *mut client_file);
    pub fn file_fire_read(_: *mut client_file);
    pub fn file_can_print(_: *mut client) -> c_int;
    pub fn file_print(_: *mut client, _: *const c_char, ...);
    pub fn file_vprint(_: *mut client, _: *const c_char, _: *mut VaList);
    pub fn file_print_buffer(_: *mut client, _: *mut c_void, _: usize);
    pub fn file_error(_: *mut client, _: *const c_char, ...);
    pub fn file_write(
        _: *mut client,
        _: *const c_char,
        _: c_int,
        _: *const c_void,
        _: usize,
        _: client_file_cb,
        _: *mut c_void,
    );
    pub fn file_read(_: *mut client, _: *const c_char, _: client_file_cb, _: *mut c_void) -> *mut client_file;
    pub fn file_cancel(_: *mut client_file);
    pub fn file_push(_: *mut client_file);
    pub fn file_write_left(_: *mut client_files) -> c_int;
    pub fn file_write_open(
        _: *mut client_files,
        _: *mut tmuxpeer,
        _: *mut imsg,
        _: c_int,
        _: c_int,
        _: client_file_cb,
        _: *mut c_void,
    );
    pub fn file_write_data(_: *mut client_files, _: *mut imsg);
    pub fn file_write_close(_: *mut client_files, _: *mut imsg);
    pub fn file_read_open(
        _: *mut client_files,
        _: *mut tmuxpeer,
        _: *mut imsg,
        _: c_int,
        _: c_int,
        _: client_file_cb,
        _: *mut c_void,
    );
    pub fn file_write_ready(_: *mut client_files, _: *mut imsg);
    pub fn file_read_data(_: *mut client_files, _: *mut imsg);
    pub fn file_read_done(_: *mut client_files, _: *mut imsg);
    pub fn file_read_cancel(_: *mut client_files, _: *mut imsg);
}
