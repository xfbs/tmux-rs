use crate::window_mode;

unsafe extern "C" {
    pub static mut window_copy_mode: window_mode;
    pub static mut window_view_mode: window_mode;
}
