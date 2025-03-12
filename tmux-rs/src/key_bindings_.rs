use super::*;

unsafe extern "C" {
    pub fn key_bindings_get_table(_: *const c_char, _: c_int) -> *mut key_table;
    pub fn key_bindings_first_table() -> *mut key_table;
    pub fn key_bindings_next_table(_: *mut key_table) -> *mut key_table;
    pub fn key_bindings_unref_table(_: *mut key_table);
    pub fn key_bindings_get(_: *mut key_table, _: key_code) -> *mut key_binding;
    pub fn key_bindings_get_default(_: *mut key_table, _: key_code) -> *mut key_binding;
    pub fn key_bindings_first(_: *mut key_table) -> *mut key_binding;
    pub fn key_bindings_next(_: *mut key_table, _: *mut key_binding) -> *mut key_binding;
    pub fn key_bindings_add(_: *const c_char, _: key_code, _: *const c_char, _: c_int, _: *mut cmd_list);
    pub fn key_bindings_remove(_: *const c_char, _: key_code);
    pub fn key_bindings_reset(_: *const c_char, _: key_code);
    pub fn key_bindings_remove_table(_: *const c_char);
    pub fn key_bindings_reset_table(_: *const c_char);
    pub fn key_bindings_init();
    pub fn key_bindings_dispatch(
        _: *mut key_binding,
        _: *mut cmdq_item,
        _: *mut client,
        _: *mut key_event,
        _: *mut cmd_find_state,
    ) -> *mut cmdq_item;
}
