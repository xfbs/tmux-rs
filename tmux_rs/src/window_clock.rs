use super::*;
unsafe extern "C" {
    pub static mut window_clock_mode: window_mode;
    pub static mut window_clock_table: [[[c_char; 5usize]; 5usize]; 14usize];
}
