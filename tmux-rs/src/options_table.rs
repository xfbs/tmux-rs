use super::*;

unsafe extern "C" {
    // TODO I don't know the actual length, so fix this
    pub static options_table: [options_table_entry; 0usize];
    pub static options_other_names: [options_name_map; 0usize];
}
