use super::*;

unsafe extern "C" {
    pub fn options_create(_: *mut options) -> *mut options;
    pub fn options_free(_: *mut options);
    pub fn options_get_parent(_: *mut options) -> *mut options;
    pub fn options_set_parent(_: *mut options, _: *mut options);
    pub fn options_first(_: *mut options) -> *mut options_entry;
    pub fn options_next(_: *mut options_entry) -> *mut options_entry;
    pub fn options_empty(_: *mut options, _: *const options_table_entry) -> *mut options_entry;
    pub fn options_default(_: *mut options, _: *const options_table_entry) -> *mut options_entry;
    pub fn options_default_to_string(_: *const options_table_entry) -> *mut c_char;
    pub fn options_name(_: *mut options_entry) -> *const c_char;
    pub fn options_owner(_: *mut options_entry) -> *mut options;
    pub fn options_table_entry(_: *mut options_entry) -> *const options_table_entry;
    pub fn options_get_only(_: *mut options, _: *const c_char) -> *mut options_entry;
    pub fn options_get(_: *mut options, _: *const c_char) -> *mut options_entry;
    pub fn options_array_clear(_: *mut options_entry);
    pub fn options_array_get(_: *mut options_entry, _: c_uint) -> *mut options_value;
    pub fn options_array_set(
        _: *mut options_entry,
        _: c_uint,
        _: *const c_char,
        _: c_int,
        _: *mut *mut c_char,
    ) -> c_int;
    pub fn options_array_assign(_: *mut options_entry, _: *const c_char, _: *mut *mut c_char) -> c_int;
    pub fn options_array_first(_: *mut options_entry) -> *mut options_array_item;
    pub fn options_array_next(_: *mut options_array_item) -> *mut options_array_item;
    pub fn options_array_item_index(_: *mut options_array_item) -> c_uint;
    pub fn options_array_item_value(_: *mut options_array_item) -> *mut options_value;
    pub fn options_is_array(_: *mut options_entry) -> c_int;
    pub fn options_is_string(_: *mut options_entry) -> c_int;
    pub fn options_to_string(_: *mut options_entry, _: c_int, _: c_int) -> *mut c_char;
    pub fn options_parse(_: *const c_char, _: *mut c_int) -> *mut c_char;
    pub fn options_parse_get(_: *mut options, _: *const c_char, _: *mut c_int, _: c_int) -> *mut options_entry;
    pub fn options_match(_: *const c_char, _: *mut c_int, _: *mut c_int) -> *mut c_char;
    pub fn options_match_get(
        _: *mut options,
        _: *const c_char,
        _: *mut c_int,
        _: c_int,
        _: *mut c_int,
    ) -> *mut options_entry;
    pub fn options_get_string(_: *mut options, _: *const c_char) -> *const c_char;
    pub fn options_get_number(_: *mut options, _: *const c_char) -> c_longlong;
    pub fn options_set_string(_: *mut options, _: *const c_char, _: c_int, _: *const c_char, ...)
    -> *mut options_entry;
    pub fn options_set_number(_: *mut options, _: *const c_char, _: c_longlong) -> *mut options_entry;
    pub fn options_scope_from_name(
        _: *mut args,
        _: c_int,
        _: *const c_char,
        _: *mut cmd_find_state,
        _: *mut *mut options,
        _: *mut *mut c_char,
    ) -> c_int;
    pub fn options_scope_from_flags(
        _: *mut args,
        _: c_int,
        _: *mut cmd_find_state,
        _: *mut *mut options,
        _: *mut *mut c_char,
    ) -> c_int;
    pub fn options_string_to_style(_: *mut options, _: *const c_char, _: *mut format_tree) -> *mut style;
    pub fn options_from_string(
        _: *mut options,
        _: *const options_table_entry,
        _: *const c_char,
        _: *const c_char,
        _: c_int,
        _: *mut *mut c_char,
    ) -> c_int;
    pub fn options_find_choice(_: *const options_table_entry, _: *const c_char, _: *mut *mut c_char) -> c_int;
    pub fn options_push_changes(_: *const c_char);
    pub fn options_remove_or_default(_: *mut options_entry, _: c_int, _: *mut *mut c_char) -> c_int;
}
