#![feature(c_variadic)]
#![allow(private_interfaces)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::new_without_default)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

pub mod alerts;
pub mod cmd_kill_server;
pub mod event_;
pub mod log;
pub mod server;
pub mod window_;
pub mod window_copy;
pub mod xmalloc;

#[cfg(feature = "utempter")]
pub mod utempter;

pub use core::{
    ffi::{
        CStr, c_char, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void,
        va_list::{VaList, VaListImpl},
    },
    mem::{ManuallyDrop, zeroed},
    ops::ControlFlow,
    ptr::{NonNull, null_mut},
};

pub type wchar_t = core::ffi::c_int;
unsafe extern "C" {
    #[expect(dead_code)]
    static mut stdin: *mut FILE;
    #[expect(dead_code)]
    static mut stdout: *mut FILE;
    static mut stderr: *mut FILE;
}

pub use libc::{FILE, REG_EXTENDED, REG_ICASE, pid_t, termios, time_t, timeval, uid_t};

pub use crate::event_::{EVBUFFER_DATA, EVBUFFER_LENGTH, evtimer_add, evtimer_set};
pub use libevent_sys::{bufferevent, evbuffer, evbuffer_get_length, evbuffer_pullup, event, event_base};

use compat_rs::queue::{Entry, list_entry, list_head, tailq_entry, tailq_head};
use compat_rs::tree::{rb_entry, rb_head};

// use crate::tmux_protocol_h::*;

pub type bitstr_t = c_uchar;

const TTY_NAME_MAX: usize = 32;

// TODO remove once options.c is ported
#[repr(C)]
#[derive(Copy, Clone)]
pub struct options_array_item {
    _opaque: [u8; 0],
}

// opaque types
macro_rules! opaque_types {
    ( $($ident:ident),* ) => {
        $(
          #[repr(C)]
          pub struct $ident { _opaque: [u8; 0] }
        )*
    };
}

opaque_types! {
    args,
    args_command_state,
    cmd,
    cmdq_item,
    cmdq_list,
    cmdq_state,
    cmds,
    control_state,
    environ,
    format_job_tree,
    format_tree,
    hyperlinks,
    hyperlinks_uri,
    input_ctx,
    job,
    menu_data,
    mode_tree_data,
    msgtype,
    options,
    options_entry,
    screen_write_citem,
    screen_write_cline
}

#[cfg(feature = "sixel")]
opaque_types! {
    sixel_image
}

opaque_types! {
    tty_code,
    tty_key,
    tmuxpeer,
    tmuxproc
}

pub const TMUX_CONF: &CStr = c"/etc/tmux.conf:~/.tmux.conf";
pub const TMUX_SOCK: &CStr = c"$TMUX_TMPDIR:/tmp/";
pub const TMUX_TERM: &CStr = c"screen";
pub const TMUX_LOCK_CMD: &CStr = c"lock -np";

/// Minimum layout cell size, NOT including border lines.
pub const PANE_MINIMUM: i32 = 1;

/// Automatic name refresh interval, in microseconds. Must be < 1 second.
pub const NAME_INTERVAL: i32 = 500000;

/// Default pixel cell sizes.
pub const DEFAULT_XPIXEL: i32 = 16;
pub const DEFAULT_YPIXEL: i32 = 32;

// Alert option values
pub const ALERT_NONE: i32 = 0;
pub const ALERT_ANY: i32 = 1;
pub const ALERT_CURRENT: i32 = 2;
pub const ALERT_OTHER: i32 = 3;

// Visual option values
pub const VISUAL_OFF: i32 = 0;
pub const VISUAL_ON: i32 = 1;
pub const VISUAL_BOTH: i32 = 2;

// No key or unknown key.
pub const KEYC_NONE: c_ulonglong = 0x000ff000000000;
pub const KEYC_UNKNOWN: c_ulonglong = 0x000fe000000000;

// Base for special (that is, not Unicode) keys. An enum must be at most a
// signed int, so these are based in the highest Unicode PUA.
pub const KEYC_BASE: c_ulonglong = 0x0000000010e000;
pub const KEYC_USER: c_ulonglong = 0x0000000010f000;
pub const KEYC_USER_END: c_ulonglong = KEYC_USER + KEYC_NUSER;

// Key modifier bits
pub const KEYC_META: c_ulonglong = 0x00100000000000;
pub const KEYC_CTRL: c_ulonglong = 0x00200000000000;
pub const KEYC_SHIFT: c_ulonglong = 0x00400000000000;

// Key flag bits.
pub const KEYC_LITERAL: c_ulonglong = 0x01000000000000;
pub const KEYC_KEYPAD: c_ulonglong = 0x02000000000000;
pub const KEYC_CURSOR: c_ulonglong = 0x04000000000000;
pub const KEYC_IMPLIED_META: c_ulonglong = 0x08000000000000;
pub const KEYC_BUILD_MODIFIERS: c_ulonglong = 0x10000000000000;
pub const KEYC_VI: c_ulonglong = 0x20000000000000;
pub const KEYC_SENT: c_ulonglong = 0x40000000000000;

// Masks for key bits.
pub const KEYC_MASK_MODIFIERS: c_ulonglong = 0x00f00000000000;
pub const KEYC_MASK_FLAGS: c_ulonglong = 0xff000000000000;
pub const KEYC_MASK_KEY: c_ulonglong = 0x000fffffffffff;

pub const KEYC_NUSER: c_ulonglong = 1000;

#[inline(always)]
pub fn KEYC_IS_MOUSE(key: key_code) -> bool {
    const KEYC_MOUSE: c_ulonglong = keyc::KEYC_MOUSE as c_ulonglong;
    const KEYC_BSPACE: c_ulonglong = keyc::KEYC_BSPACE as c_ulonglong;

    (key & KEYC_MASK_KEY) >= KEYC_MOUSE && (key & KEYC_MASK_KEY) < KEYC_BSPACE
}

#[inline(always)]
pub fn KEYC_IS_UNICODE(key: key_code) -> bool {
    let masked = key & KEYC_MASK_KEY;

    const KEYC_BASE_END: c_ulonglong = keyc::KEYC_BASE_END as c_ulonglong;
    masked > 0x7f && (masked < KEYC_BASE || masked >= KEYC_BASE_END) && (masked < KEYC_USER || masked >= KEYC_USER_END)
}

pub const KEYC_CLICK_TIMEOUT: i32 = 300;

/// A single key. This can be ASCII or Unicode or one of the keys between
/// KEYC_BASE and KEYC_BASE_END.
pub type key_code = core::ffi::c_ulonglong;

// skipped C0 control characters

// idea write a custom top level macro
// which allows me to annotate a variant
// that should be converted to mouse key
/*
enum mouse_keys {
  KEYC_MOUSE,

  #[keyc_mouse_key]
  MOUSEMOVE,
}
*/
include!("keyc_mouse_key.rs");

/// Termcap codes.
#[repr(i32)]
pub enum tty_code_code {
    TTYC_ACSC,
    TTYC_AM,
    TTYC_AX,
    TTYC_BCE,
    TTYC_BEL,
    TTYC_BIDI,
    TTYC_BLINK,
    TTYC_BOLD,
    TTYC_CIVIS,
    TTYC_CLEAR,
    TTYC_CLMG,
    TTYC_CMG,
    TTYC_CNORM,
    TTYC_COLORS,
    TTYC_CR,
    TTYC_CS,
    TTYC_CSR,
    TTYC_CUB,
    TTYC_CUB1,
    TTYC_CUD,
    TTYC_CUD1,
    TTYC_CUF,
    TTYC_CUF1,
    TTYC_CUP,
    TTYC_CUU,
    TTYC_CUU1,
    TTYC_CVVIS,
    TTYC_DCH,
    TTYC_DCH1,
    TTYC_DIM,
    TTYC_DL,
    TTYC_DL1,
    TTYC_DSBP,
    TTYC_DSEKS,
    TTYC_DSFCS,
    TTYC_DSMG,
    TTYC_E3,
    TTYC_ECH,
    TTYC_ED,
    TTYC_EL,
    TTYC_EL1,
    TTYC_ENACS,
    TTYC_ENBP,
    TTYC_ENEKS,
    TTYC_ENFCS,
    TTYC_ENMG,
    TTYC_FSL,
    TTYC_HLS,
    TTYC_HOME,
    TTYC_HPA,
    TTYC_ICH,
    TTYC_ICH1,
    TTYC_IL,
    TTYC_IL1,
    TTYC_INDN,
    TTYC_INVIS,
    TTYC_KCBT,
    TTYC_KCUB1,
    TTYC_KCUD1,
    TTYC_KCUF1,
    TTYC_KCUU1,
    TTYC_KDC2,
    TTYC_KDC3,
    TTYC_KDC4,
    TTYC_KDC5,
    TTYC_KDC6,
    TTYC_KDC7,
    TTYC_KDCH1,
    TTYC_KDN2,
    TTYC_KDN3,
    TTYC_KDN4,
    TTYC_KDN5,
    TTYC_KDN6,
    TTYC_KDN7,
    TTYC_KEND,
    TTYC_KEND2,
    TTYC_KEND3,
    TTYC_KEND4,
    TTYC_KEND5,
    TTYC_KEND6,
    TTYC_KEND7,
    TTYC_KF1,
    TTYC_KF10,
    TTYC_KF11,
    TTYC_KF12,
    TTYC_KF13,
    TTYC_KF14,
    TTYC_KF15,
    TTYC_KF16,
    TTYC_KF17,
    TTYC_KF18,
    TTYC_KF19,
    TTYC_KF2,
    TTYC_KF20,
    TTYC_KF21,
    TTYC_KF22,
    TTYC_KF23,
    TTYC_KF24,
    TTYC_KF25,
    TTYC_KF26,
    TTYC_KF27,
    TTYC_KF28,
    TTYC_KF29,
    TTYC_KF3,
    TTYC_KF30,
    TTYC_KF31,
    TTYC_KF32,
    TTYC_KF33,
    TTYC_KF34,
    TTYC_KF35,
    TTYC_KF36,
    TTYC_KF37,
    TTYC_KF38,
    TTYC_KF39,
    TTYC_KF4,
    TTYC_KF40,
    TTYC_KF41,
    TTYC_KF42,
    TTYC_KF43,
    TTYC_KF44,
    TTYC_KF45,
    TTYC_KF46,
    TTYC_KF47,
    TTYC_KF48,
    TTYC_KF49,
    TTYC_KF5,
    TTYC_KF50,
    TTYC_KF51,
    TTYC_KF52,
    TTYC_KF53,
    TTYC_KF54,
    TTYC_KF55,
    TTYC_KF56,
    TTYC_KF57,
    TTYC_KF58,
    TTYC_KF59,
    TTYC_KF6,
    TTYC_KF60,
    TTYC_KF61,
    TTYC_KF62,
    TTYC_KF63,
    TTYC_KF7,
    TTYC_KF8,
    TTYC_KF9,
    TTYC_KHOM2,
    TTYC_KHOM3,
    TTYC_KHOM4,
    TTYC_KHOM5,
    TTYC_KHOM6,
    TTYC_KHOM7,
    TTYC_KHOME,
    TTYC_KIC2,
    TTYC_KIC3,
    TTYC_KIC4,
    TTYC_KIC5,
    TTYC_KIC6,
    TTYC_KIC7,
    TTYC_KICH1,
    TTYC_KIND,
    TTYC_KLFT2,
    TTYC_KLFT3,
    TTYC_KLFT4,
    TTYC_KLFT5,
    TTYC_KLFT6,
    TTYC_KLFT7,
    TTYC_KMOUS,
    TTYC_KNP,
    TTYC_KNXT2,
    TTYC_KNXT3,
    TTYC_KNXT4,
    TTYC_KNXT5,
    TTYC_KNXT6,
    TTYC_KNXT7,
    TTYC_KPP,
    TTYC_KPRV2,
    TTYC_KPRV3,
    TTYC_KPRV4,
    TTYC_KPRV5,
    TTYC_KPRV6,
    TTYC_KPRV7,
    TTYC_KRI,
    TTYC_KRIT2,
    TTYC_KRIT3,
    TTYC_KRIT4,
    TTYC_KRIT5,
    TTYC_KRIT6,
    TTYC_KRIT7,
    TTYC_KUP2,
    TTYC_KUP3,
    TTYC_KUP4,
    TTYC_KUP5,
    TTYC_KUP6,
    TTYC_KUP7,
    TTYC_MS,
    TTYC_NOBR,
    TTYC_OL,
    TTYC_OP,
    TTYC_RECT,
    TTYC_REV,
    TTYC_RGB,
    TTYC_RI,
    TTYC_RIN,
    TTYC_RMACS,
    TTYC_RMCUP,
    TTYC_RMKX,
    TTYC_SE,
    TTYC_SETAB,
    TTYC_SETAF,
    TTYC_SETAL,
    TTYC_SETRGBB,
    TTYC_SETRGBF,
    TTYC_SETULC,
    TTYC_SETULC1,
    TTYC_SGR0,
    TTYC_SITM,
    TTYC_SMACS,
    TTYC_SMCUP,
    TTYC_SMKX,
    TTYC_SMOL,
    TTYC_SMSO,
    TTYC_SMUL,
    TTYC_SMULX,
    TTYC_SMXX,
    TTYC_SXL,
    TTYC_SS,
    TTYC_SWD,
    TTYC_SYNC,
    TTYC_TC,
    TTYC_TSL,
    TTYC_U8,
    TTYC_VPA,
    TTYC_XT,
}

pub const WHITESPACE: &CStr = c" ";

// Mode Keys. TODO convert to enum
pub const MODEKEY_EMACS: i32 = 0;
pub const MODEKEY_VI: i32 = 1;

// Modes.
pub const MODE_CURSOR: i32 = 0x1;
pub const MODE_INSERT: i32 = 0x2;
pub const MODE_KCURSOR: i32 = 0x4;
pub const MODE_KKEYPAD: i32 = 0x8;
pub const MODE_WRAP: i32 = 0x10;
pub const MODE_MOUSE_STANDARD: i32 = 0x20;
pub const MODE_MOUSE_BUTTON: i32 = 0x40;
pub const MODE_CURSOR_BLINKING: i32 = 0x80;
pub const MODE_MOUSE_UTF8: i32 = 0x100;
pub const MODE_MOUSE_SGR: i32 = 0x200;
pub const MODE_BRACKETPASTE: i32 = 0x400;
pub const MODE_FOCUSON: i32 = 0x800;
pub const MODE_MOUSE_ALL: i32 = 0x1000;
pub const MODE_ORIGIN: i32 = 0x2000;
pub const MODE_CRLF: i32 = 0x4000;
pub const MODE_KEYS_EXTENDED: i32 = 0x8000;
pub const MODE_CURSOR_VERY_VISIBLE: i32 = 0x10000;
pub const MODE_CURSOR_BLINKING_SET: i32 = 0x20000;
pub const MODE_KEYS_EXTENDED_2: i32 = 0x40000;

pub const ALL_MODES: i32 = 0xffffff;
pub const ALL_MOUSE_MODES: i32 = MODE_MOUSE_STANDARD | MODE_MOUSE_BUTTON | MODE_MOUSE_ALL;
pub const MOTION_MOUSE_MODES: i32 = MODE_MOUSE_BUTTON | MODE_MOUSE_ALL;
pub const CURSOR_MODES: i32 = MODE_CURSOR | MODE_CURSOR_BLINKING | MODE_CURSOR_VERY_VISIBLE;
pub const EXTENDED_KEY_MODES: i32 = MODE_KEYS_EXTENDED | MODE_KEYS_EXTENDED_2;

// Mouse protocol constants.
pub const MOUSE_PARAM_MAX: i32 = 0xff;
pub const MOUSE_PARAM_UTF8_MAX: i32 = 0x7ff;
pub const MOUSE_PARAM_BTN_OFF: i32 = 0x20;
pub const MOUSE_PARAM_POS_OFF: i32 = 0x21;

/* A single UTF-8 character. */
pub type utf8_char = c_uint;

// An expanded UTF-8 character. UTF8_SIZE must be big enough to hold combining
// characters as well. It can't be more than 32 bytes without changes to how
// characters are stored.
const UTF8_SIZE: usize = 21;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct utf8_data {
    pub data: [c_uchar; UTF8_SIZE],

    pub have: c_uchar,
    pub size: c_uchar,

    /// 0xff if invalid
    pub width: c_uchar,
}

pub use utf8_state::*;
#[repr(i32)]
pub enum utf8_state {
    UTF8_MORE,
    UTF8_DONE,
    UTF8_ERROR,
}

// Colour flags.
pub const COLOUR_FLAG_256: i32 = 0x01000000;
pub const COLOUR_FLAG_RGB: i32 = 0x02000000;

/// Special colours.
#[inline]
pub fn COLOR_DEFAULT(c: i32) -> bool {
    c == 8 || c == 9
}

// Replacement palette.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct colour_palette {
    pub fg: i32,
    pub bg: i32,

    pub palette: *mut i32,
    pub default_palette: *mut i32,
}

// Grid attributes. Anything above 0xff is stored in an extended cell.
pub const GRID_ATTR_BRIGHT: i32 = 0x1;
pub const GRID_ATTR_DIM: i32 = 0x2;
pub const GRID_ATTR_UNDERSCORE: i32 = 0x4;
pub const GRID_ATTR_BLINK: i32 = 0x8;
pub const GRID_ATTR_REVERSE: i32 = 0x10;
pub const GRID_ATTR_HIDDEN: i32 = 0x20;
pub const GRID_ATTR_ITALICS: i32 = 0x40;
pub const GRID_ATTR_CHARSET: i32 = 0x80; // alternative character set
pub const GRID_ATTR_STRIKETHROUGH: i32 = 0x100;
pub const GRID_ATTR_UNDERSCORE_2: i32 = 0x200;
pub const GRID_ATTR_UNDERSCORE_3: i32 = 0x400;
pub const GRID_ATTR_UNDERSCORE_4: i32 = 0x800;
pub const GRID_ATTR_UNDERSCORE_5: i32 = 0x1000;
pub const GRID_ATTR_OVERLINE: i32 = 0x2000;

/// All underscore attributes.
pub const GRID_ATTR_ALL_UNDERSCORE: i32 = GRID_ATTR_UNDERSCORE
    | GRID_ATTR_UNDERSCORE_2
    | GRID_ATTR_UNDERSCORE_3
    | GRID_ATTR_UNDERSCORE_4
    | GRID_ATTR_UNDERSCORE_5;

// Grid flags.
pub const GRID_FLAG_FG256: i32 = 0x1;
pub const GRID_FLAG_BG256: i32 = 0x2;
pub const GRID_FLAG_PADDING: i32 = 0x4;
pub const GRID_FLAG_EXTENDED: i32 = 0x8;
pub const GRID_FLAG_SELECTED: i32 = 0x10;
pub const GRID_FLAG_NOPALETTE: i32 = 0x20;
pub const GRID_FLAG_CLEARED: i32 = 0x40;

// Grid line flags.
pub const GRID_LINE_WRAPPED: i32 = 0x1;
pub const GRID_LINE_EXTENDED: i32 = 0x2;
pub const GRID_LINE_DEAD: i32 = 0x4;
pub const GRID_LINE_START_PROMPT: i32 = 0x8;
pub const GRID_LINE_START_OUTPUT: i32 = 0x10;

// Grid string flags.
pub const GRID_STRING_WITH_SEQUENCES: i32 = 0x1;
pub const GRID_STRING_ESCAPE_SEQUENCES: i32 = 0x2;
pub const GRID_STRING_TRIM_SPACES: i32 = 0x4;
pub const GRID_STRING_USED_ONLY: i32 = 0x8;
pub const GRID_STRING_EMPTY_CELLS: i32 = 0x10;

// Cell positions.
pub const CELL_INSIDE: i32 = 0;
pub const CELL_TOPBOTTOM: i32 = 1;
pub const CELL_LEFTRIGHT: i32 = 2;
pub const CELL_TOPLEFT: i32 = 3;
pub const CELL_TOPRIGHT: i32 = 4;
pub const CELL_BOTTOMLEFT: i32 = 5;
pub const CELL_BOTTOMRIGHT: i32 = 6;
pub const CELL_TOPJOIN: i32 = 7;
pub const CELL_BOTTOMJOIN: i32 = 8;
pub const CELL_LEFTJOIN: i32 = 9;
pub const CELL_RIGHTJOIN: i32 = 10;
pub const CELL_JOIN: i32 = 11;
pub const CELL_OUTSIDE: i32 = 12;

// Cell borders.
pub const CELL_BORDERS: &CStr = c" xqlkmjwvtun~";
pub const SIMPLE_BORDERS: &CStr = c" |-+++++++++.";
pub const PADDED_BORDERS: &CStr = c"             ";

/// Grid cell data.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct grid_cell {
    pub data: utf8_data,
    pub attr: c_ushort,
    pub flags: c_uchar,
    pub fg: i32,
    pub bg: i32,
    pub us: i32,
    pub link: u32,
}

/// Grid extended cell entry.
pub type grid_extd_entry = grid_cell;

#[repr(C, align(4))]
pub struct grid_cell_entry_data {
    pub attr: c_uchar,
    pub fg: c_uchar,
    pub bg: c_uchar,
    pub data: c_uchar,
}
#[repr(C)]
pub struct grid_cell_entry {
    pub data: grid_cell_entry_data,
    pub flags: c_uchar,
}

/// Grid line.
#[repr(C)]
pub struct grid_line {
    pub celldata: *mut grid_cell_entry,
    pub cellused: u32,
    pub cellsize: u32,

    pub extddata: *mut grid_extd_entry,
    pub extdsize: u32,

    pub flags: i32,
    pub time: time_t,
}

pub const GRID_HISTORY: i32 = 0x1; // scroll lines into history

/// Entire grid of cells.
#[repr(C)]
pub struct grid {
    pub flags: i32,

    pub sx: u32,
    pub sy: u32,

    pub hscrolled: u32,
    pub hsize: u32,
    pub hlimit: u32,

    pub linedata: *mut grid_line,
}

/// Virtual cursor in a grid.
#[repr(C)]
pub struct grid_reader {
    pub gd: *mut grid,
    pub cx: u32,
    pub cy: u32,
}

/// Style alignment.
#[repr(i32)]
pub enum style_align {
    STYLE_ALIGN_DEFAULT,
    STYLE_ALIGN_LEFT,
    STYLE_ALIGN_CENTRE,
    STYLE_ALIGN_RIGHT,
    STYLE_ALIGN_ABSOLUTE_CENTRE,
}

/// Style list.
#[repr(i32)]
pub enum style_list {
    STYLE_LIST_OFF,
    STYLE_LIST_ON,
    STYLE_LIST_FOCUS,
    STYLE_LIST_LEFT_MARKER,
    STYLE_LIST_RIGHT_MARKER,
}

/// Style range.
#[repr(i32)]
pub enum style_range_type {
    STYLE_RANGE_NONE,
    STYLE_RANGE_LEFT,
    STYLE_RANGE_RIGHT,
    STYLE_RANGE_PANE,
    STYLE_RANGE_WINDOW,
    STYLE_RANGE_SESSION,
    STYLE_RANGE_USER,
}

#[repr(C)]
pub struct style_range {
    pub type_: style_range_type,
    pub argument: u32,
    pub string: [c_char; 16],
    pub start: u32,
    /// not included
    pub end: u32,

    pub entry: tailq_entry<style_range>,
}
pub type style_ranges = tailq_head<style_range>;

/// Style default.
#[repr(i32)]
pub enum style_default_type {
    STYLE_DEFAULT_BASE,
    STYLE_DEFAULT_PUSH,
    STYLE_DEFAULT_POP,
}

/// Style option.
#[repr(C)]
pub struct style {
    pub gc: grid_cell,
    pub ignore: i32,

    pub fill: i32,
    pub align: style_align,
    pub list: style_list,

    pub range_type: style_range_type,
    pub range_argument: u32,
    pub range_string: [c_char; 16],

    pub default_type: style_default_type,
}

#[cfg(feature = "sixel")]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct image {
    s: *mut screen,
    data: *mut sixel_image,
    fallback: *mut c_char,
    px: u32,
    py: u32,
    sx: u32,
    sy: u32,

    all_entry: tailq_entry<image>,
    entry: tailq_entry<image>,
}

#[cfg(feature = "sixel")]
pub type images = tailq_head<image>;

/// Cursor style.
#[repr(C)]
#[derive(Copy, Clone)]
pub enum screen_cursor_style {
    SCREEN_CURSOR_DEFAULT,
    SCREEN_CURSOR_BLOCK,
    SCREEN_CURSOR_UNDERLINE,
    SCREEN_CURSOR_BAR,
}

opaque_types! {
    screen_sel,
    screen_titles
}

/// Virtual screen.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct screen {
    pub title: *mut c_char,
    pub path: *mut c_char,
    pub titles: *mut screen_titles,

    /// grid data
    pub grid: *mut grid,

    /// cursor x
    pub cx: u32,
    /// cursor y
    pub cy: u32,

    /// cursor style
    pub cstyle: screen_cursor_style,
    pub default_cstyle: screen_cursor_style,
    /// cursor colour
    pub ccolour: i32,
    /// default cursor colour
    pub default_ccolour: i32,

    /// scroll region top
    pub rupper: u32,
    /// scroll region bottom
    pub rlower: u32,

    pub mode: i32,
    pub default_mode: i32,

    pub saved_cx: u32,
    pub saved_cy: u32,
    pub saved_grid: *mut grid,
    pub saved_cell: grid_cell,
    pub saved_flags: i32,

    pub tabs: *mut bitstr_t,
    pub sel: *mut screen_sel,

    #[cfg(feature = "sixel")]
    images: images,

    pub write_list: *mut screen_write_cline,

    pub hyperlinks: *mut hyperlinks,
}

pub const SCREEN_WRITE_SYNC: i32 = 0x1;

// Screen write context.
pub type screen_write_init_ctx_cb = Option<unsafe extern "C" fn(*mut screen_write_ctx, *mut tty_ctx)>;
#[repr(C)]
pub struct screen_write_ctx {
    pub wp: *mut window_pane,
    pub s: *mut screen,

    pub flags: i32,

    pub init_ctx_cb: screen_write_init_ctx_cb,

    pub arg: *mut c_void,

    pub item: *mut screen_write_citem,
    pub scrolled: u32,
    pub bg: u32,
}

/// Box border lines option.
#[repr(i32)]
pub enum box_lines {
    BOX_LINES_DEFAULT = -1,
    BOX_LINES_SINGLE,
    BOX_LINES_DOUBLE,
    BOX_LINES_HEAVY,
    BOX_LINES_SIMPLE,
    BOX_LINES_ROUNDED,
    BOX_LINES_PADDED,
    BOX_LINES_NONE,
}

/// Pane border lines option.
#[repr(i32)]
pub enum pane_lines {
    PANE_LINES_SINGLE,
    PANE_LINES_DOUBLE,
    PANE_LINES_HEAVY,
    PANE_LINES_SIMPLE,
    PANE_LINES_NUMBER,
}

// Pane border indicator option.
pub const PANE_BORDER_OFF: i32 = 0;
pub const PANE_BORDER_COLOUR: i32 = 1;
pub const PANE_BORDER_ARROWS: i32 = 2;
pub const PANE_BORDER_BOTH: i32 = 3;

// Mode returned by window_pane_mode function.
pub const WINDOW_PANE_NO_MODE: i32 = 0;
pub const WINDOW_PANE_COPY_MODE: i32 = 1;
pub const WINDOW_PANE_VIEW_MODE: i32 = 2;

// Screen redraw context.
#[repr(C)]
pub struct screen_redraw_ctx {
    pub c: *mut client,

    pub statuslines: u32,
    pub statustop: i32,

    pub pane_status: i32,
    pub pane_lines: pane_lines,

    pub no_pane_gc: grid_cell,
    pub no_pane_gc_set: i32,

    pub sx: u32,
    pub sy: u32,
    pub ox: u32,
    pub oy: u32,
}

pub unsafe fn screen_size_x(s: *const screen) -> u32 {
    unsafe { (*(*s).grid).sx }
}
pub unsafe fn screen_size_y(s: *const screen) -> u32 {
    unsafe { (*(*s).grid).sx }
}
pub unsafe fn screen_hsize(s: *const screen) -> u32 {
    unsafe { (*(*s).grid).hsize }
}
pub unsafe fn screen_hlimit(s: *const screen) -> u32 {
    unsafe { (*(*s).grid).hlimit }
}

// Menu.
#[repr(C)]
pub struct menu_item {
    pub name: *const c_char,
    pub key: key_code,
    pub command: *const c_char,
}
#[repr(C)]
pub struct menu {
    pub title: *const c_char,
    pub items: *mut menu_item,
    pub count: u32,
    pub width: u32,
}
pub type menu_choice_cb = Option<unsafe extern "C" fn(*mut menu, u32, key_code, *mut c_void)>;

// Window mode. Windows can be in several modes and this is used to call the
// right function to handle input and output.
#[repr(C)]
pub struct window_mode {
    pub name: *const c_char,
    pub default_format: *const c_char,

    pub init: Option<unsafe extern "C" fn(*mut window_mode_entry, *mut cmd_find_state, *mut args) -> *mut screen>,
    pub free: Option<unsafe extern "C" fn(*mut window_mode_entry)>,
    pub resize: Option<unsafe extern "C" fn(*mut window_mode_entry, u32, u32)>,
    pub update: Option<unsafe extern "C" fn(*mut window_mode_entry)>,
    pub key: Option<
        unsafe extern "C" fn(
            *mut window_mode_entry,
            *mut client,
            *mut session,
            *mut winlink,
            key_code,
            *mut mouse_event,
        ),
    >,

    pub key_table: Option<unsafe extern "C" fn(*mut window_mode_entry) -> *const c_char>,
    pub command: Option<
        unsafe extern "C" fn(
            *mut window_mode_entry,
            *mut client,
            *mut session,
            *mut winlink,
            *mut args,
            *mut mouse_event,
        ),
    >,
    pub formats: Option<unsafe extern "C" fn(*mut window_mode_entry, *mut format_tree)>,
}

// Active window mode.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct window_mode_entry {
    pub wp: *mut window_pane,
    pub swp: *mut window_pane,

    pub mode: *mut window_mode,
    pub data: *mut (),

    pub screen: *mut screen,
    pub prefix: u32,

    pub entry: tailq_entry<window_mode_entry>,
}
impl Entry<window_mode_entry> for window_mode_entry {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<window_mode_entry> {
        unsafe { &raw mut (*this).entry }
    }
}

/// Offsets into pane buffer.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct window_pane_offset {
    pub used: usize,
}

/// Queued pane resize.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct window_pane_resize {
    pub sx: u32,
    pub sy: u32,

    pub osx: u32,
    pub osy: u32,

    pub entry: tailq_entry<window_pane_resize>,
}
pub type window_pane_resizes = tailq_head<window_pane_resize>;
impl Entry<window_pane_resize> for window_pane_resize {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<window_pane_resize> {
        unsafe { &raw mut (*this).entry }
    }
}

pub const PANE_REDRAW: i32 = 0x1;
pub const PANE_DROP: i32 = 0x2;
pub const PANE_FOCUSED: i32 = 0x4;
pub const PANE_VISITED: i32 = 0x8;
/* 0x10 unused */
/* 0x20 unused */
pub const PANE_INPUTOFF: i32 = 0x40;
pub const PANE_CHANGED: i32 = 0x80;
pub const PANE_EXITED: i32 = 0x100;
pub const PANE_STATUSREADY: i32 = 0x200;
pub const PANE_STATUSDRAWN: i32 = 0x400;
pub const PANE_EMPTY: i32 = 0x800;
pub const PANE_STYLECHANGED: i32 = 0x1000;
pub const PANE_UNSEENCHANGES: i32 = 0x2000;

/// Child window structure.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct window_pane {
    pub id: u32,
    pub active_point: u32,

    pub window: *mut window,
    pub options: *mut options,

    pub layout_cell: *mut layout_cell,
    pub saved_layout_cell: *mut layout_cell,

    pub sx: u32,
    pub sy: u32,

    pub xoff: u32,
    pub yoff: u32,

    pub flags: i32,

    pub argc: i32,
    pub argv: *mut *mut c_char,
    pub shell: *mut c_char,
    pub cwd: *mut c_char,

    pub pid: pid_t,
    pub tty: [c_char; TTY_NAME_MAX],
    pub status: i32,
    pub dead_time: timeval,

    pub fd: i32,
    pub event: *mut bufferevent,

    pub offset: window_pane_offset,
    pub base_offset: usize,

    pub resize_queue: window_pane_resizes,
    pub resize_timer: event,

    pub ictx: *mut input_ctx,

    pub cached_gc: grid_cell,
    pub cached_active_gc: grid_cell,
    pub palette: colour_palette,

    pub pipe_fd: i32,
    pub pipe_event: *mut bufferevent,
    pub pipe_offset: window_pane_offset,

    pub screen: *mut screen,
    pub base: screen,

    pub status_screen: screen,
    pub status_size: usize,

    pub modes: tailq_head<window_mode_entry>,

    pub searchstr: *mut c_char,
    pub searchregex: i32,

    pub border_gc_set: i32,
    pub border_gc: grid_cell,

    pub control_bg: i32,
    pub control_fg: i32,

    /// link in list of all panes
    pub entry: tailq_entry<window_pane>,
    /// link in list of last visited
    pub sentry: tailq_entry<window_pane>,
    pub tree_entry: rb_entry<window_pane>,
}

impl Entry<window_pane> for window_pane {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<window_pane> {
        unsafe { &raw mut (*this).entry }
    }
}
impl compat_rs::tree::GetEntry<window_pane> for window_pane {
    fn entry_mut(this: *mut Self) -> *mut rb_entry<window_pane> {
        // <https://github.com/rust-lang/rust/pull/129248#issue-2472094687>
        #![expect(
            clippy::not_unsafe_ptr_arg_deref,
            reason = "false positive. no load occurs. see: https://www.ralfj.de/blog/2024/08/14/places.html"
        )]
        unsafe { &raw mut (*this).tree_entry }
    }

    fn entry(this: *const Self) -> *const rb_entry<window_pane> {
        #![expect(
            clippy::not_unsafe_ptr_arg_deref,
            reason = "false positive. no load occurs. see: https://www.ralfj.de/blog/2024/08/14/places.html"
        )]
        unsafe { &raw const (*this).tree_entry }
    }

    unsafe fn cmp(this: *const Self, other: *const Self) -> i32 {
        unsafe { (*this).id.wrapping_sub((*other).id) as i32 }
    }
}

pub type window_panes = tailq_head<window_pane>;
pub type window_pane_tree = rb_head<window_pane>;
compat_rs::impl_rb_tree_protos!(window_pane_tree, window_pane);

pub const WINDOW_BELL: i32 = 0x1;
pub const WINDOW_ACTIVITY: i32 = 0x2;
pub const WINDOW_SILENCE: i32 = 0x4;
pub const WINDOW_ZOOMED: i32 = 0x8;
pub const WINDOW_WASZOOMED: i32 = 0x10;
pub const WINDOW_RESIZE: i32 = 0x20;
pub const WINDOW_ALERTFLAGS: i32 = WINDOW_BELL | WINDOW_ACTIVITY | WINDOW_SILENCE;

/// Window structure.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct window {
    pub id: u32,
    pub latest: *mut (),

    pub name: *mut c_char,
    pub name_event: event,
    pub name_time: timeval,

    pub alerts_timer: event,
    pub offset_timer: event,

    pub activity_time: timeval,

    pub active: *mut window_pane,
    pub last_panes: window_panes,
    pub panes: window_panes,

    pub lastlayout: i32,
    pub layout_root: *mut layout_cell,
    pub saved_layout_root: *mut layout_cell,
    pub old_layout: *mut c_char,

    pub sx: u32,
    pub sy: u32,
    pub manual_sx: u32,
    pub manual_sy: u32,
    pub xpixel: u32,
    pub ypixel: u32,

    pub new_sx: u32,
    pub new_sy: u32,
    pub new_xpixel: u32,
    pub new_ypixel: u32,

    pub fill_character: *mut utf8_data,
    pub flags: i32,

    pub alerts_queued: i32,
    pub alerts_entry: tailq_entry<window>,

    pub options: *mut options,

    pub references: u32,
    pub winlinks: tailq_head<winlink>,
    pub entry: rb_entry<window>,
}
pub type windows = rb_head<window>;
compat_rs::impl_rb_tree_protos!(windows, window);

impl compat_rs::tree::GetEntry<window> for window {
    fn entry_mut(this: *mut Self) -> *mut rb_entry<window> {
        #![expect(
            clippy::not_unsafe_ptr_arg_deref,
            reason = "false positive. no load occurs. see: https://www.ralfj.de/blog/2024/08/14/places.html"
        )]
        unsafe { &raw mut (*this).entry }
    }

    fn entry(this: *const Self) -> *const rb_entry<window> {
        #![expect(
            clippy::not_unsafe_ptr_arg_deref,
            reason = "false positive. no load occurs. see: https://www.ralfj.de/blog/2024/08/14/places.html"
        )]
        unsafe { &raw const (*this).entry }
    }

    unsafe fn cmp(this: *const Self, other: *const Self) -> i32 {
        unsafe { (*this).id.wrapping_sub((*other).id) as i32 }
    }
}

pub const WINLINK_BELL: i32 = 0x1;
pub const WINLINK_ACTIVITY: i32 = 0x2;
pub const WINLINK_SILENCE: i32 = 0x4;
pub const WINLINK_ALERTFLAGS: i32 = WINLINK_BELL | WINLINK_ACTIVITY | WINLINK_SILENCE;
pub const WINLINK_VISITED: i32 = 0x8;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct winlink {
    pub idx: i32,
    pub session: *mut session,
    pub window: *mut window,

    pub flags: i32,

    pub entry: rb_entry<winlink>,

    pub wentry: tailq_entry<winlink>,
    pub sentry: tailq_entry<winlink>,
}

impl compat_rs::queue::Entry<winlink> for winlink {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<winlink> {
        unsafe { &raw mut (*this).wentry }
    }
}

impl compat_rs::tree::GetEntry<winlink> for winlink {
    fn entry_mut(this: *mut Self) -> *mut rb_entry<winlink> {
        #![expect(
            clippy::not_unsafe_ptr_arg_deref,
            reason = "false positive. no load occurs. see: https://www.ralfj.de/blog/2024/08/14/places.html"
        )]
        unsafe { &raw mut (*this).entry }
    }

    fn entry(this: *const Self) -> *const rb_entry<winlink> {
        #![expect(
            clippy::not_unsafe_ptr_arg_deref,
            reason = "false positive. no load occurs. see: https://www.ralfj.de/blog/2024/08/14/places.html"
        )]
        unsafe { &raw const (*this).entry }
    }

    unsafe fn cmp(this: *const Self, other: *const Self) -> i32 {
        unsafe { (*this).idx.wrapping_sub((*other).idx) }
    }
}

pub type winlinks = rb_head<winlink>;
compat_rs::impl_rb_tree_protos!(winlinks, winlink);
pub type winlink_stack = tailq_head<winlink>;
compat_rs::impl_rb_tree_protos!(winlink_stack, winlink);

// Window size option.
pub const WINDOW_SIZE_LARGEST: i32 = 0;
pub const WINDOW_SIZE_SMALLEST: i32 = 1;
pub const WINDOW_SIZE_MANUAL: i32 = 2;
pub const WINDOW_SIZE_LATEST: i32 = 3;

// Pane border status option.
pub const PANE_STATUS_OFF: i32 = 0;
pub const PANE_STATUS_TOP: i32 = 1;
pub const PANE_STATUS_BOTTOM: i32 = 2;

/// Layout direction.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum layout_type {
    LAYOUT_LEFTRIGHT,
    LAYOUT_TOPBOTTOM,
    LAYOUT_WINDOWPANE,
}

/// Layout cells queue.
pub type layout_cells = tailq_head<layout_cell>;

/// Layout cell.
#[repr(C)]
pub struct layout_cell {
    pub type_: layout_type,

    pub parent: *mut layout_cell,

    pub sx: u32,
    pub sy: u32,

    pub xoff: u32,
    pub yoff: u32,

    pub wp: *mut window_pane,
    pub cells: layout_cells,

    pub entry: tailq_entry<layout_cell>,
}

pub const ENVIRON_HIDDEN: i32 = 0x1;

/// Environment variable.
#[repr(C)]
pub struct environ_entry {
    pub name: *mut c_char,
    pub value: *mut c_char,

    pub flags: i32,
    pub entry: rb_entry<environ_entry>,
}

/// Client session.
#[repr(C)]
pub struct session_group {
    pub name: *const c_char,
    pub sessions: tailq_head<session>,

    pub entry: rb_entry<session_group>,
}
pub type session_groups = rb_head<session_group>;

pub const SESSION_PASTING: i32 = 0x1;
pub const SESSION_ALERTED: i32 = 0x2;

#[repr(C)]
pub struct session {
    pub id: u32,
    pub name: *mut c_char,
    pub cwd: *mut c_char,

    pub creation_time: timeval,
    pub last_attached_time: timeval,
    pub activity_time: timeval,
    pub last_activity_time: timeval,

    pub lock_timer: event,

    pub curw: *mut winlink,
    pub lastw: winlink_stack,
    pub windows: winlinks,

    pub statusat: i32,
    pub statuslines: u32,

    pub options: *mut options,

    pub flags: i32,

    pub attached: u32,

    pub tio: *mut termios,

    pub environ: *mut environ,

    pub references: i32,

    pub gentry: tailq_entry<session>,
    pub entry: rb_entry<session>,
}
pub type sessions = rb_head<session>;

pub const MOUSE_MASK_BUTTONS: i32 = 195;
pub const MOUSE_MASK_SHIFT: i32 = 4;
pub const MOUSE_MASK_META: i32 = 8;
pub const MOUSE_MASK_CTRL: i32 = 16;
pub const MOUSE_MASK_DRAG: i32 = 32;
pub const MOUSE_MASK_MODIFIERS: i32 = MOUSE_MASK_SHIFT | MOUSE_MASK_META | MOUSE_MASK_CTRL;

/* Mouse wheel type. */
pub const MOUSE_WHEEL_UP: i32 = 64;
pub const MOUSE_WHEEL_DOWN: i32 = 65;

/* Mouse button type. */
pub const MOUSE_BUTTON_1: i32 = 0;
pub const MOUSE_BUTTON_2: i32 = 1;
pub const MOUSE_BUTTON_3: i32 = 2;
pub const MOUSE_BUTTON_6: i32 = 66;
pub const MOUSE_BUTTON_7: i32 = 67;
pub const MOUSE_BUTTON_8: i32 = 128;
pub const MOUSE_BUTTON_9: i32 = 129;
pub const MOUSE_BUTTON_10: i32 = 130;
pub const MOUSE_BUTTON_11: i32 = 131;

// Mouse helpers.
#[inline]
pub fn MOUSE_BUTTONS(b: i32) -> bool {
    b & MOUSE_MASK_BUTTONS != 0
}
#[inline]
pub fn MOUSE_WHEEL(b: i32) -> bool {
    ((b) & MOUSE_MASK_BUTTONS) == MOUSE_WHEEL_UP || ((b) & MOUSE_MASK_BUTTONS) == MOUSE_WHEEL_DOWN
}
#[inline]
pub fn MOUSE_DRAG(b: i32) -> bool {
    b & MOUSE_MASK_DRAG != 0
}
#[inline]
pub fn MOUSE_RELEASE(b: i32) -> bool {
    b & MOUSE_MASK_BUTTONS == 3
}

// Mouse input.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct mouse_event {
    pub valid: i32,
    pub ignore: i32,

    pub key: key_code,

    pub statusat: i32,
    pub statuslines: u32,

    pub x: u32,
    pub y: u32,
    pub b: u32,

    pub lx: u32,
    pub ly: u32,
    pub lb: u32,

    pub ox: u32,
    pub oy: u32,

    pub s: i32,
    pub w: i32,
    pub wp: i32,

    pub sgr_type: u32,
    pub sgr_b: u32,
}

/// Key event.
#[repr(C)]
pub struct key_event {
    pub key: key_code,
    pub m: mouse_event,
}

pub const TERM_256COLOURS: i32 = 0x1;
pub const TERM_NOAM: i32 = 0x2;
pub const TERM_DECSLRM: i32 = 0x4;
pub const TERM_DECFRA: i32 = 0x8;
pub const TERM_RGBCOLOURS: i32 = 0x10;
pub const TERM_VT100LIKE: i32 = 0x20;
pub const TERM_SIXEL: i32 = 0x40;

/// Terminal definition.
#[repr(C)]
pub struct tty_term {
    pub name: *mut c_char,
    pub tty: *mut tty,
    pub features: i32,

    pub acs: [[c_char; c_uchar::MAX as usize + 1]; 2],

    pub codes: *mut tty_code,

    pub flags: i32,

    pub entry: list_entry<tty_term>,
}
pub type tty_terms = list_head<tty_term>;

pub const TTY_NOCURSOR: i32 = 0x1;
pub const TTY_FREEZE: i32 = 0x2;
pub const TTY_TIMER: i32 = 0x4;
pub const TTY_NOBLOCK: i32 = 0x8;
pub const TTY_STARTED: i32 = 0x10;
pub const TTY_OPENED: i32 = 0x20;
pub const TTY_OSC52QUERY: i32 = 0x40;
pub const TTY_BLOCK: i32 = 0x80;
pub const TTY_HAVEDA: i32 = 0x100; // Primary DA.
pub const TTY_HAVEXDA: i32 = 0x200;
pub const TTY_SYNCING: i32 = 0x400;
pub const TTY_HAVEDA2: i32 = 0x800; // Secondary DA.
pub const TTY_ALL_REQUEST_FLAGS: i32 = TTY_HAVEDA | TTY_HAVEDA2 | TTY_HAVEXDA;

/// Client terminal.
#[repr(C)]
pub struct tty {
    pub client: *mut client,
    pub start_timer: event,
    pub clipboard_timer: event,
    pub last_requests: time_t,

    pub sx: u32,
    pub sy: u32,

    pub xpixel: u32,
    pub ypixel: u32,

    pub cx: u32,
    pub cy: u32,
    pub cstyle: screen_cursor_style,
    pub ccolour: i32,

    pub oflag: i32,
    pub oox: u32,
    pub ooy: u32,
    pub osx: u32,
    pub osy: u32,

    pub mode: i32,
    pub fg: i32,
    pub bg: i32,

    pub rlower: u32,
    pub rupper: u32,

    pub rleft: u32,
    pub rright: u32,

    pub event_in: event,
    pub in_: *mut evbuffer,
    pub event_out: event,
    pub out: *mut evbuffer,
    pub timer: event,
    pub discarded: usize,

    pub tio: termios,

    pub cell: grid_cell,
    pub last_cell: grid_cell,

    pub flags: i32,

    pub term: *mut tty_term,

    pub mouse_last_x: u32,
    pub mouse_last_y: u32,
    pub mouse_last_b: u32,
    pub mouse_drag_flag: i32,
    pub mouse_drag_update: Option<unsafe extern "C" fn(*mut client, *mut mouse_event)>,
    pub mouse_drag_release: Option<unsafe extern "C" fn(*mut client, *mut mouse_event)>,

    pub key_timer: event,
    pub key_tree: tty_key,
}

pub type tty_ctx_redraw_cb = Option<unsafe extern "C" fn(*const tty_ctx)>;
pub type tty_ctx_set_client_cb = Option<unsafe extern "C" fn(*mut tty_ctx, *mut client)>;

#[repr(C)]
pub struct tty_ctx {
    pub s: *mut screen,

    pub redraw_cb: tty_ctx_redraw_cb,
    pub set_client_cb: tty_ctx_set_client_cb,
    pub arg: *mut (),

    pub cell: *const grid_cell,
    pub wrapped: i32,

    pub num: u32,
    pub ptr: *mut (),
    pub ptr2: *mut (),

    pub allow_invisible_panes: i32,

    /*
     * Cursor and region position before the screen was updated - this is
     * where the command should be applied; the values in the screen have
     * already been updated.
     */
    pub ocx: u32,
    pub oxy: u32,

    pub orupper: u32,
    pub orlower: u32,

    /* Target region (usually pane) offset and size. */
    pub xoff: u32,
    pub yoff: u32,
    pub rxoff: u32,
    pub ryoff: u32,
    pub sx: u32,
    pub sy: u32,

    // The background colour used for clearing (erasing).
    pub bg: u32,

    // The default colours and palette.
    pub defaults: grid_cell,
    pub palette: colour_palette,

    // Containing region (usually window) offset and size.
    pub bigger: i32,
    pub wox: u32,
    pub woy: u32,
    pub wsx: u32,
    pub wsy: u32,
}

// Saved message entry.
#[repr(C)]
pub struct message_entry {
    pub msg: *mut c_char,
    pub msg_num: u32,
    pub msg_time: timeval,

    pub entry: tailq_entry<message_entry>,
}
pub type message_list = tailq_head<message_entry>;

/// Argument type.
#[repr(i32)]
pub enum args_type {
    ARGS_NONE,
    ARGS_STRING,
    ARGS_COMMANDS,
}

#[repr(C)]
pub union args_value_union {
    pub string: *mut c_char,
    pub cmdlist: *mut cmd_list,
}

/// Argument value.
#[repr(C)]
pub struct args_value {
    pub type_: args_type,
    pub args_value_union: args_value_union,
    pub cached: *mut c_char,
    pub entry: tailq_entry<args_value>,
}

opaque_types! {
    args_entry
}
/// Arguments set.
pub type args_tree = rb_head<args_entry>;

/// Arguments parsing type.
#[repr(C)]
pub enum args_parse_type {
    ARGS_PARSE_INVALID,
    ARGS_PARSE_STRING,
    ARGS_PARSE_COMMANDS_OR_STRING,
    ARGS_PARSE_COMMANDS,
}

pub type args_parse_cb = Option<unsafe extern "C" fn(*mut args, u32, *mut *mut c_char) -> args_parse_type>;
#[repr(C)]
pub struct args_parse {
    pub template: *const c_char,
    pub lower: i32,
    pub upper: i32,
    pub cb: args_parse_cb,
}

/// Command find structures.
#[repr(C)]
#[derive(Copy, Clone)]
pub enum cmd_find_type {
    CMD_FIND_PANE,
    CMD_FIND_WINDOW,
    CMD_FIND_SESSION,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cmd_find_state {
    pub flags: i32,
    pub current: *mut cmd_find_state,

    pub s: *mut session,
    pub wl: *mut winlink,
    pub w: *mut window,
    pub wp: *mut window_pane,
    pub idx: i32,
}

// Command find flags.
pub const CMD_FIND_PREFER_UNATTACHED: i32 = 0x1;
pub const CMD_FIND_QUIET: i32 = 0x2;
pub const CMD_FIND_WINDOW_INDEX: i32 = 0x4;
pub const CMD_FIND_DEFAULT_MARKED: i32 = 0x8;
pub const CMD_FIND_EXACT_SESSION: i32 = 0x10;
pub const CMD_FIND_EXACT_WINDOW: i32 = 0x20;
pub const CMD_FIND_CANFAIL: i32 = 0x40;

/// List of commands.
#[repr(C)]
pub struct cmd_list {
    pub references: i32,
    pub group: u32,
    pub list: *mut cmds,
}

/* Command return values. */
#[repr(i32)]
#[derive(Copy, Clone)]
pub enum cmd_retval {
    CMD_RETURN_ERROR = -1,
    CMD_RETURN_NORMAL = 0,
    CMD_RETURN_WAIT,
    CMD_RETURN_STOP,
}

// Command parse result.
#[repr(i32)]
pub enum cmd_parse_status {
    CMD_PARSE_ERROR,
    CMD_PARSE_SUCCESS,
}
#[repr(C)]
pub struct cmd_parse_result {
    pub status: cmd_parse_status,
    pub cmdlist: *mut cmd_list,
    pub error: *mut c_char,
}

pub const CMD_PARSE_QUIET: i32 = 0x1;
pub const CMD_PARSE_PARSEONLY: i32 = 0x2;
pub const CMD_PARSE_NOALIAS: i32 = 0x4;
pub const CMD_PARSE_VERBOSE: i32 = 0x8;
pub const CMD_PARSE_ONEGROUP: i32 = 0x10;

#[repr(C)]
pub struct cmd_parse_input {
    pub flags: i32,

    pub file: *const c_char,
    pub line: u32,

    pub item: *mut cmdq_item,
    pub c: *mut client,
    pub fs: cmd_find_state,
}

/* Command queue flags. */
pub const CMDQ_STATE_REPEAT: i32 = 0x1;
pub const CMDQ_STATE_CONTROL: i32 = 0x2;
pub const CMDQ_STATE_NOHOOKS: i32 = 0x4;

// Command queue callback.
pub type cmdq_cb = Option<unsafe extern "C" fn(*mut cmdq_item, *mut c_void) -> cmd_retval>;

// Command definition flag.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct cmd_entry_flag {
    pub flag: c_char,
    pub type_: cmd_find_type,
    pub flags: i32,
}

pub const CMD_STARTSERVER: i32 = 0x1;
pub const CMD_READONLY: i32 = 0x2;
pub const CMD_AFTERHOOK: i32 = 0x4;
pub const CMD_CLIENT_CFLAG: i32 = 0x8;
pub const CMD_CLIENT_TFLAG: i32 = 0x10;
pub const CMD_CLIENT_CANFAIL: i32 = 0x20;

// Command definition.
#[repr(C)]
pub struct cmd_entry {
    pub name: *const c_char,
    pub alias: *const c_char,

    pub args: args_parse,
    pub usage: *const c_char,

    pub source: cmd_entry_flag,
    pub target: cmd_entry_flag,

    pub flags: i32,

    pub exec: Option<unsafe extern "C" fn(*mut cmd, *mut cmdq_item) -> cmd_retval>,
}

/* Status line. */
pub const STATUS_LINES_LIMIT: usize = 5;
#[repr(C)]
pub struct status_line_entry {
    pub expanded: *mut c_char,
    pub ranges: style_ranges,
}
#[repr(C)]
pub struct status_line {
    pub timer: event,

    pub screen: screen,
    pub active: *mut screen,
    pub references: c_int,

    pub style: grid_cell,
    pub entries: [status_line_entry; STATUS_LINES_LIMIT],
}

/* Prompt type. */
pub const PROMPT_NTYPES: usize = 4;
#[repr(i32)]
pub enum prompt_type {
    PROMPT_TYPE_COMMAND,
    PROMPT_TYPE_SEARCH,
    PROMPT_TYPE_TARGET,
    PROMPT_TYPE_WINDOW_TARGET,
    PROMPT_TYPE_INVALID = 0xff,
}

/* File in client. */
pub type client_file_cb = Option<unsafe extern "C" fn(*mut client, *mut c_char, i32, i32, *mut evbuffer, *mut c_void)>;
#[repr(C)]
pub struct client_file {
    pub c: *mut client,
    pub peer: *mut tmuxpeer,
    pub tree: *mut client_files,

    pub references: i32,
    pub stream: i32,

    pub path: *mut c_char,
    pub buffer: *mut evbuffer,
    pub event: *mut bufferevent,

    pub fd: i32,
    pub error: i32,
    pub closed: i32,

    pub cb: client_file_cb,
    pub data: *mut c_void,

    pub entry: rb_entry<client_file>,
}
pub type client_files = rb_head<client_file>;

// Client window.
#[repr(C)]
pub struct client_window {
    pub window: u32,
    pub pane: *mut window_pane,

    pub sx: u32,
    pub sy: u32,

    pub entry: rb_entry<client_window>,
}
pub type client_windows = rb_head<client_window>;

/* Visible areas not obstructed by overlays. */
pub const OVERLAY_MAX_RANGES: usize = 3;
#[repr(C)]
pub struct overlay_ranges {
    pub px: [u32; OVERLAY_MAX_RANGES],
    pub nx: [u32; OVERLAY_MAX_RANGES],
}

pub type prompt_input_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void, *const c_char, i32) -> i32>;
pub type prompt_free_cb = Option<unsafe extern "C" fn(*mut c_void)>;
pub type overlay_check_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void, u32, u32, u32, *mut overlay_ranges)>;
pub type overlay_mode_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void, *mut u32, *mut u32) -> *mut screen>;
pub type overlay_draw_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void, *mut screen_redraw_ctx)>;
pub type overlay_key_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void, *mut key_event) -> i32>;
pub type overlay_free_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void)>;
pub type overlay_resize_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void)>;

pub const CLIENT_TERMINAL: u64 = 0x1;
pub const CLIENT_LOGIN: u64 = 0x2;
pub const CLIENT_EXIT: u64 = 0x4;
pub const CLIENT_REDRAWWINDOW: u64 = 0x8;
pub const CLIENT_REDRAWSTATUS: u64 = 0x10;
pub const CLIENT_REPEAT: u64 = 0x20;
pub const CLIENT_SUSPENDED: u64 = 0x40;
pub const CLIENT_ATTACHED: u64 = 0x80;
pub const CLIENT_EXITED: u64 = 0x100;
pub const CLIENT_DEAD: u64 = 0x200;
pub const CLIENT_REDRAWBORDERS: u64 = 0x400;
pub const CLIENT_READONLY: u64 = 0x800;
pub const CLIENT_NOSTARTSERVER: u64 = 0x1000;
pub const CLIENT_CONTROL: u64 = 0x2000;
pub const CLIENT_CONTROLCONTROL: u64 = 0x4000;
pub const CLIENT_FOCUSED: u64 = 0x8000;
pub const CLIENT_UTF8: u64 = 0x10000;
pub const CLIENT_IGNORESIZE: u64 = 0x20000;
pub const CLIENT_IDENTIFIED: u64 = 0x40000;
pub const CLIENT_STATUSFORCE: u64 = 0x80000;
pub const CLIENT_DOUBLECLICK: u64 = 0x100000;
pub const CLIENT_TRIPLECLICK: u64 = 0x200000;
pub const CLIENT_SIZECHANGED: u64 = 0x400000;
pub const CLIENT_STATUSOFF: u64 = 0x800000;
pub const CLIENT_REDRAWSTATUSALWAYS: u64 = 0x1000000;
pub const CLIENT_REDRAWOVERLAY: u64 = 0x2000000;
pub const CLIENT_CONTROL_NOOUTPUT: u64 = 0x4000000;
pub const CLIENT_DEFAULTSOCKET: u64 = 0x8000000;
pub const CLIENT_STARTSERVER: u64 = 0x10000000;
pub const CLIENT_REDRAWPANES: u64 = 0x20000000;
pub const CLIENT_NOFORK: u64 = 0x40000000;
pub const CLIENT_ACTIVEPANE: u64 = 0x80000000u64;
pub const CLIENT_CONTROL_PAUSEAFTER: u64 = 0x100000000u64;
pub const CLIENT_CONTROL_WAITEXIT: u64 = 0x200000000u64;
pub const CLIENT_WINDOWSIZECHANGED: u64 = 0x400000000u64;
pub const CLIENT_CLIPBOARDBUFFER: u64 = 0x800000000u64;
pub const CLIENT_BRACKETPASTING: u64 = 0x1000000000u64;
pub const CLIENT_ALLREDRAWFLAGS: u64 = CLIENT_REDRAWWINDOW
    | CLIENT_REDRAWSTATUS
    | CLIENT_REDRAWSTATUSALWAYS
    | CLIENT_REDRAWBORDERS
    | CLIENT_REDRAWOVERLAY
    | CLIENT_REDRAWPANES;
pub const CLIENT_UNATTACHEDFLAGS: u64 = CLIENT_DEAD | CLIENT_SUSPENDED | CLIENT_EXIT;
pub const CLIENT_NODETACHFLAGS: u64 = CLIENT_DEAD | CLIENT_EXIT;
pub const CLIENT_NOSIZEFLAGS: u64 = CLIENT_DEAD | CLIENT_SUSPENDED | CLIENT_EXIT;

//#[derive(Copy, Clone)]
#[repr(C)]
pub struct client {
    pub name: *const c_char,
    pub peer: *mut tmuxpeer,
    pub queue: *mut cmdq_list,

    pub windows: client_windows,

    pub control_state: *mut control_state,
    pub pause_age: c_uint,

    pub pid: pid_t,
    pub fd: c_int,
    pub out_fd: c_int,
    pub event: event,
    pub retval: c_int,

    pub creation_time: timeval,
    pub activity_time: timeval,

    pub environ: *mut environ,
    pub jobs: *mut format_job_tree,

    pub title: *mut c_char,
    pub path: *mut c_char,
    pub cwd: *const c_char,

    pub term_name: *mut c_char,
    pub term_features: c_int,
    pub term_type: *mut c_char,
    pub term_caps: *mut *mut c_char,
    pub term_ncaps: c_uint,

    pub ttyname: *mut c_char,
    pub tty: tty,

    pub written: usize,
    pub discarded: usize,
    pub redraw: usize,

    pub repeat_timer: event,

    pub click_timer: event,
    pub click_button: c_uint,
    pub click_event: mouse_event,

    pub status: status_line,

    pub flags: u64,

    pub exit_type: exit_type,
    pub exit_msgtype: msgtype,
    pub exit_session: *mut c_char,
    pub exit_message: *mut c_char,

    pub keytable: *mut key_table,

    pub redraw_panes: u64,

    pub message_ignore_keys: c_int,
    pub message_ignore_styles: c_int,
    pub message_string: *mut c_char,
    pub message_timer: event,

    pub prompt_string: *mut c_char,
    pub prompt_buffer: *mut utf8_data,
    pub prompt_last: *mut c_char,
    pub prompt_index: usize,
    pub prompt_inputcb: prompt_input_cb,
    pub prompt_freecb: prompt_free_cb,
    pub prompt_data: *mut libc::c_void,
    pub prompt_hindex: [c_uint; 4],
    pub prompt_mode: prompt_mode,
    pub prompt_saved: *mut utf8_data,
    pub prompt_flags: c_int,
    pub prompt_type: prompt_type,
    pub prompt_cursor: c_int,

    pub session: *mut session,
    pub last_session: *mut session,

    pub references: c_int,

    pub pan_window: *mut libc::c_void,
    pub pan_ox: c_uint,
    pub pan_oy: c_uint,

    pub overlay_check: overlay_check_cb,
    pub overlay_mode: overlay_mode_cb,
    pub overlay_draw: overlay_draw_cb,
    pub overlay_key: overlay_key_cb,
    pub overlay_free: overlay_free_cb,
    pub overlay_resize: overlay_resize_cb,
    pub overlay_data: *mut libc::c_void,
    pub overlay_timer: event,

    pub files: client_files,

    pub clipboard_panes: *mut c_uint,
    pub clipboard_npanes: c_uint,

    pub entry: tailq_entry<client>,
}
pub type clients = tailq_head<client>;
impl Entry<client> for client {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<client> {
        unsafe { &raw mut (*this).entry }
    }
}

/// Control mode subscription type.
#[repr(i32)]
pub enum control_sub_type {
    CONTROL_SUB_SESSION,
    CONTROL_SUB_PANE,
    CONTROL_SUB_ALL_PANES,
    CONTROL_SUB_WINDOW,
    CONTROL_SUB_ALL_WINDOWS,
}

pub const KEY_BINDING_REPEAT: i32 = 0x1;

/// Key binding and key table.
#[repr(C)]
pub struct key_binding {
    pub key: key_code,
    pub cmdlist: cmd_list,
    pub note: *const c_char,

    pub flags: i32,

    pub entry: rb_entry<key_binding>,
}
pub type key_bindings = rb_head<key_binding>;

#[repr(C)]
pub struct key_table {
    pub name: *mut c_char,
    pub key_bindings: key_bindings,
    pub default_key_bindings: key_bindings,

    pub references: u32,

    pub entry: rb_entry<key_table>,
}
pub type key_tables = rb_head<key_table>;

// Option data.
pub type options_array = rb_head<options_array_item>;
#[repr(C)]
pub union options_value {
    pub string: *mut c_char,
    pub number: c_longlong,
    pub style: ManuallyDrop<style>,
    pub array: options_array,
    pub cmdlist: *mut cmd_list,
}

// Option table entries.
#[repr(i32)]
pub enum options_table_type {
    OPTIONS_TABLE_STRING,
    OPTIONS_TABLE_NUMBER,
    OPTIONS_TABLE_KEY,
    OPTIONS_TABLE_COLOUR,
    OPTIONS_TABLE_FLAG,
    OPTIONS_TABLE_CHOICE,
    OPTIONS_TABLE_COMMAND,
}

pub const OPTIONS_TABLE_NONE: i32 = 0;
pub const OPTIONS_TABLE_SERVER: i32 = 0x1;
pub const OPTIONS_TABLE_SESSION: i32 = 0x2;
pub const OPTIONS_TABLE_WINDOW: i32 = 0x4;
pub const OPTIONS_TABLE_PANE: i32 = 0x8;

pub const OPTIONS_TABLE_IS_ARRAY: i32 = 0x1;
pub const OPTIONS_TABLE_IS_HOOK: i32 = 0x2;
pub const OPTIONS_TABLE_IS_STYLE: i32 = 0x4;

#[repr(C)]
pub struct options_table_entry {
    pub name: *mut c_char,
    pub alternative_name: *mut c_char,
    pub type_: options_table_type,
    pub scope: i32,
    pub flags: i32,
    pub minimum: u32,
    pub maximum: u32,

    pub choices: *mut *mut c_char,

    pub default_str: *mut c_char,
    pub default_num: c_longlong,
    pub default_arr: *mut *mut c_char,

    pub separator: *mut c_char,
    pub pattern: *mut c_char,

    pub text: *mut c_char,
    pub unit: *mut c_char,
}

#[repr(C)]
pub struct options_name_map {
    pub from: *mut c_char,
    pub to: *mut c_char,
}

/* Common command usages. */
pub const CMD_TARGET_PANE_USAGE: &CStr = c"[-t target-pane]";
pub const CMD_TARGET_WINDOW_USAGE: &CStr = c"[-t target-window]";
pub const CMD_TARGET_SESSION_USAGE: &CStr = c"[-t target-session]";
pub const CMD_TARGET_CLIENT_USAGE: &CStr = c"[-t target-client]";
pub const CMD_SRCDST_PANE_USAGE: &CStr = c"[-s src-pane] [-t dst-pane]";
pub const CMD_SRCDST_WINDOW_USAGE: &CStr = c"[-s src-window] [-t dst-window]";
pub const CMD_SRCDST_SESSION_USAGE: &CStr = c"[-s src-session] [-t dst-session]";
pub const CMD_SRCDST_CLIENT_USAGE: &CStr = c"[-s src-client] [-t dst-client]";
pub const CMD_BUFFER_USAGE: &CStr = c"[-b buffer-name]";

pub const SPAWN_KILL: i32 = 0x1;
pub const SPAWN_DETACHED: i32 = 0x2;
pub const SPAWN_RESPAWN: i32 = 0x4;
pub const SPAWN_BEFORE: i32 = 0x8;
pub const SPAWN_NONOTIFY: i32 = 0x10;
pub const SPAWN_FULLSIZE: i32 = 0x20;
pub const SPAWN_EMPTY: i32 = 0x40;
pub const SPAWN_ZOOM: i32 = 0x80;

/// Spawn common context.
#[repr(C)]
pub struct spawn_context {
    pub item: *mut cmdq_item,

    pub s: *mut session,
    pub wl: *mut winlink,
    pub tc: *mut client,

    pub wp0: *mut window_pane,
    pub lc: *mut layout_cell,

    pub name: *mut c_char,
    pub argv: *mut *mut c_char,
    pub argc: i32,
    pub environ: *mut environ,

    pub idx: i32,
    pub cwd: *mut c_char,

    pub flags: i32,
}

/// Mode tree sort order.
#[repr(C)]
pub struct mode_tree_sort_criteria {
    pub field: u32,
    pub reversed: i32,
}

// panic!();
//

pub const WINDOW_MINIMUM: i32 = PANE_MINIMUM;
pub const WINDOW_MAXIMUM: i32 = 10_000;

#[repr(i32)]
pub enum exit_type {
    CLIENT_EXIT_RETURN,
    CLIENT_EXIT_SHUTDOWN,
    CLIENT_EXIT_DETACH,
}

#[repr(i32)]
pub enum prompt_mode {
    PROMPT_ENTRY,
    PROMPT_COMMAND,
}

// tmux.c
unsafe extern "C" {
    pub static mut global_options: *mut options;
    pub static mut global_s_options: *mut options;
    pub static mut global_w_options: *mut options;
    pub static mut global_environ: *mut environ;
    pub static mut start_time: timeval;
    pub static mut socket_path: *mut c_char;
    pub static mut ptm_fd: c_int;
    pub static mut shell_command: *mut c_char;

    pub fn checkshell(_: *mut c_char) -> c_int;
    pub fn setblocking(_: c_int, _: c_int);
    pub fn shell_argv0(_: *mut c_char, _: c_int) -> *mut c_char;
    pub fn get_timer() -> u64;
    pub fn sig2name(_: i32) -> *mut c_char;
    pub fn find_cwd() -> *mut c_char;
    pub fn find_home() -> *mut c_char;
    pub fn getversion() -> *mut c_char;
}

// proc.c
#[repr(C)]
struct imsg {
    _opaque: [u8; 0],
}
unsafe extern "C" {
    pub fn proc_send(_: *mut tmuxpeer, _: msgtype, _: c_int, _: *const c_void, _: usize) -> c_int;
    pub fn proc_start(_: *const c_char) -> *mut tmuxproc;
    pub fn proc_loop(_: *mut tmuxproc, _: Option<unsafe extern "C" fn() -> c_int>);
    pub fn proc_exit(_: *mut tmuxproc);
    pub fn proc_set_signals(_: *mut tmuxproc, _: Option<unsafe extern "C" fn(_: c_int)>);
    pub fn proc_clear_signals(_: *mut tmuxproc, _: c_int);
    #[expect(private_interfaces)]
    pub fn proc_add_peer(
        _: *mut tmuxproc,
        _: c_int,
        _: Option<unsafe extern "C" fn(_: *mut imsg, _: *mut c_void)>,
        _: *mut c_void,
    ) -> *mut tmuxpeer;
    pub fn proc_remove_peer(_: *mut tmuxpeer);
    pub fn proc_kill_peer(_: *mut tmuxpeer);
    pub fn proc_flush_peer(_: *mut tmuxpeer);
    pub fn proc_toggle_log(_: *mut tmuxproc);
    pub fn proc_fork_and_daemon(_: *mut c_int) -> pid_t;
    pub fn proc_get_peer_uid(_: *mut tmuxpeer) -> uid_t;
}

// cfg.c
unsafe extern "C" {
    pub static mut cfg_finished: c_int;
    pub static mut cfg_client: *mut client;
    pub static mut cfg_files: *mut *mut c_char;
    pub static mut cfg_nfiles: c_uint;
    pub static mut cfg_quiet: c_int;
    pub fn start_cfg();
    pub fn load_cfg(
        _: *const c_char,
        _: *mut client,
        _: *mut cmdq_item,
        _: *mut cmd_find_state,
        _: c_int,
        _: *mut *mut cmdq_item,
    ) -> c_int;
    pub fn load_cfg_from_buffer(
        _: *const c_void,
        _: usize,
        _: *const c_char,
        _: *mut client,
        _: *mut cmdq_item,
        _: *mut cmd_find_state,
        _: c_int,
        _: *mut *mut cmdq_item,
    ) -> c_int;
    pub fn cfg_add_cause(_: *const c_char, ...);
    pub fn cfg_print_causes(_: *mut cmdq_item);
    pub fn cfg_show_causes(_: *mut session);
}

// paste.c
#[repr(C)]
struct paste_buffer {
    _opaque: [u8; 0],
}
#[expect(private_interfaces)]
unsafe extern "C" {
    pub fn paste_buffer_name(_: *mut paste_buffer) -> *const c_char;
    pub fn paste_buffer_order(_: *mut paste_buffer) -> c_uint;
    pub fn paste_buffer_created(_: *mut paste_buffer) -> time_t;
    pub fn paste_buffer_data(_: *mut paste_buffer, _: *mut usize) -> *const c_char;
    pub fn paste_walk(_: *mut paste_buffer) -> *mut paste_buffer;
    pub fn paste_is_empty() -> c_int;
    pub fn paste_get_top(_: *mut *const c_char) -> *mut paste_buffer;
    pub fn paste_get_name(_: *const c_char) -> *mut paste_buffer;
    pub fn paste_free(_: *mut paste_buffer);
    pub fn paste_add(_: *const c_char, _: *mut c_char, _: usize);
    pub fn paste_rename(_: *const c_char, _: *const c_char, _: *mut *mut c_char) -> c_int;
    pub fn paste_set(_: *mut c_char, _: usize, _: *const c_char, _: *mut *mut c_char) -> c_int;
    pub fn paste_replace(_: *mut paste_buffer, _: *mut c_char, _: usize);
    pub fn paste_make_sample(_: *mut paste_buffer) -> *mut c_char;
}

// format.c
pub const FORMAT_STATUS: u32 = 1;
pub const FORMAT_FORCE: u32 = 2;
pub const FORMAT_NOJOBS: u32 = 4;
pub const FORMAT_VERBOSE: u32 = 8;
pub const FORMAT_NONE: u32 = 0;
pub const FORMAT_PANE: u32 = 0x80000000;
pub const FORMAT_WINDOW: u32 = 0x40000000;
pub type format_cb = Option<unsafe extern "C" fn(_: *mut format_tree) -> *mut c_void>;
#[expect(private_interfaces)]
unsafe extern "C" {
    pub fn format_tidy_jobs();
    pub fn format_skip(_: *const c_char, _: *const c_char) -> *const c_char;
    pub fn format_true(_: *const c_char) -> c_int;
    pub fn format_create(_: *mut client, _: *mut cmdq_item, _: c_int, _: c_int) -> *mut format_tree;
    pub fn format_free(_: *mut format_tree);
    pub fn format_merge(_: *mut format_tree, _: *mut format_tree);
    pub fn format_get_pane(_: *mut format_tree) -> *mut window_pane;
    pub fn format_add(_: *mut format_tree, _: *const c_char, _: *const c_char, ...);
    pub fn format_add_tv(_: *mut format_tree, _: *const c_char, _: *mut timeval);
    pub fn format_add_cb(_: *mut format_tree, _: *const c_char, _: format_cb);
    pub fn format_log_debug(_: *mut format_tree, _: *const c_char);
    pub fn format_each(
        _: *mut format_tree,
        _: Option<unsafe extern "C" fn(_: *const c_char, _: *const c_char, _: *mut c_void)>,
        _: *mut c_void,
    );
    pub fn format_pretty_time(_: time_t, _: c_int) -> *mut c_char;
    pub fn format_expand_time(_: *mut format_tree, _: *const c_char) -> *mut c_char;
    pub fn format_expand(_: *mut format_tree, _: *const c_char) -> *mut c_char;
    pub fn format_single(
        _: *mut cmdq_item,
        _: *const c_char,
        _: *mut client,
        _: *mut session,
        _: *mut winlink,
        _: *mut window_pane,
    ) -> *mut c_char;
    pub fn format_single_from_state(
        _: *mut cmdq_item,
        _: *const c_char,
        _: *mut client,
        _: *mut cmd_find_state,
    ) -> *mut c_char;
    pub fn format_single_from_target(_: *mut cmdq_item, _: *const c_char) -> *mut c_char;
    pub fn format_create_defaults(
        _: *mut cmdq_item,
        _: *mut client,
        _: *mut session,
        _: *mut winlink,
        _: *mut window_pane,
    ) -> *mut format_tree;
    pub fn format_create_from_state(_: *mut cmdq_item, _: *mut client, _: *mut cmd_find_state) -> *mut format_tree;
    pub fn format_create_from_target(_: *mut cmdq_item) -> *mut format_tree;
    pub fn format_defaults(_: *mut format_tree, _: *mut client, _: *mut session, _: *mut winlink, _: *mut window_pane);
    pub fn format_defaults_window(_: *mut format_tree, _: *mut window);
    pub fn format_defaults_pane(_: *mut format_tree, _: *mut window_pane);
    pub fn format_defaults_paste_buffer(_: *mut format_tree, _: *mut paste_buffer);
    pub fn format_lost_client(_: *mut client);
    pub fn format_grid_word(_: *mut grid, _: c_uint, _: c_uint) -> *mut c_char;
    pub fn format_grid_hyperlink(_: *mut grid, _: c_uint, _: c_uint, _: *mut screen) -> *mut c_char;
    pub fn format_grid_line(_: *mut grid, _: c_uint) -> *mut c_char;
}

// format-draw.c
unsafe extern "C" {
    pub fn format_draw(
        _: *mut screen_write_ctx,
        _: *const grid_cell,
        _: c_uint,
        _: *const c_char,
        _: *mut style_ranges,
        _: c_int,
    );
    pub fn format_width(_: *const c_char) -> c_uint;
    pub fn format_trim_left(_: *const c_char, _: c_uint) -> *mut c_char;
    pub fn format_trim_right(_: *const c_char, _: c_uint) -> *mut c_char;
}

// notify.c
unsafe extern "C" {
    pub fn notify_hook(_: *mut cmdq_item, _: *const c_char);
    pub fn notify_client(_: *const c_char, _: *mut client);
    pub fn notify_session(_: *const c_char, _: *mut session);
    pub fn notify_winlink(_: *const c_char, _: *mut winlink);
    pub fn notify_session_window(_: *const c_char, _: *mut session, _: *mut window);
    pub fn notify_window(_: *const c_char, _: *mut window);
    pub fn notify_pane(_: *const c_char, _: *mut window_pane);
    pub fn notify_paste_buffer(_: *const c_char, _: c_int);
}

// options.c
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

// options-table.c
unsafe extern "C" {
    pub static options_table: [options_table_entry; 0usize];
    pub static options_other_names: [options_name_map; 0usize];
}

// job.c
pub type job_update_cb = Option<unsafe extern "C" fn(_: *mut job)>;
pub type job_complete_cb = Option<unsafe extern "C" fn(_: *mut job)>;
pub type job_free_cb = Option<unsafe extern "C" fn(_: *mut c_void)>;
unsafe extern "C" {
    pub fn job_run(
        _: *const c_char,
        _: c_int,
        _: *mut *mut c_char,
        _: *mut environ,
        _: *mut session,
        _: *const c_char,
        _: job_update_cb,
        _: job_complete_cb,
        _: job_free_cb,
        _: *mut c_void,
        _: c_int,
        _: c_int,
        _: c_int,
    ) -> *mut job;
    pub fn job_free(_: *mut job);
    pub fn job_transfer(_: *mut job, _: *mut pid_t, _: *mut c_char, _: usize) -> c_int;
    pub fn job_resize(_: *mut job, _: c_uint, _: c_uint);
    pub fn job_check_died(_: pid_t, _: c_int);
    pub fn job_get_status(_: *mut job) -> c_int;
    pub fn job_get_data(_: *mut job) -> *mut c_void;
    pub fn job_get_event(_: *mut job) -> *mut bufferevent;
    pub fn job_kill_all();
    pub fn job_still_running() -> c_int;
    pub fn job_print_summary(_: *mut cmdq_item, _: c_int);
}

// environ.c
unsafe extern "C" {
    pub fn environ_create() -> *mut environ;
    pub fn environ_free(_: *mut environ);
    pub fn environ_first(_: *mut environ) -> *mut environ_entry;
    pub fn environ_next(_: *mut environ_entry) -> *mut environ_entry;
    pub fn environ_copy(_: *mut environ, _: *mut environ);
    pub fn environ_find(_: *mut environ, _: *const c_char) -> *mut environ_entry;
    pub fn environ_set(_: *mut environ, _: *const c_char, _: c_int, _: *const c_char, ...);
    pub fn environ_clear(_: *mut environ, _: *const c_char);
    pub fn environ_put(_: *mut environ, _: *const c_char, _: c_int);
    pub fn environ_unset(_: *mut environ, _: *const c_char);
    pub fn environ_update(_: *mut options, _: *mut environ, _: *mut environ);
    pub fn environ_push(_: *mut environ);
    pub fn environ_log(_: *mut environ, _: *const c_char, ...);
    pub fn environ_for_session(_: *mut session, _: c_int) -> *mut environ;
}

// tty.c
unsafe extern "C" {
    pub fn tty_create_log();
    pub fn tty_window_bigger(_: *mut tty) -> c_int;
    pub fn tty_window_offset(_: *mut tty, _: *mut c_uint, _: *mut c_uint, _: *mut c_uint, _: *mut c_uint) -> c_int;
    pub fn tty_update_window_offset(_: *mut window);
    pub fn tty_update_client_offset(_: *mut client);
    pub fn tty_raw(_: *mut tty, _: *const c_char);
    pub fn tty_attributes(
        _: *mut tty,
        _: *const grid_cell,
        _: *const grid_cell,
        _: *mut colour_palette,
        _: *mut hyperlinks,
    );
    pub fn tty_reset(_: *mut tty);
    pub fn tty_region_off(_: *mut tty);
    pub fn tty_m_in_off(_: *mut tty);
    pub fn tty_cursor(_: *mut tty, _: c_uint, _: c_uint);
    pub fn tty_clipboard_query(_: *mut tty);
    pub fn tty_putcode(_: *mut tty, _: tty_code_code);
    pub fn tty_putcode_i(_: *mut tty, _: tty_code_code, _: c_int);
    pub fn tty_putcode_ii(_: *mut tty, _: tty_code_code, _: c_int, _: c_int);
    pub fn tty_putcode_iii(_: *mut tty, _: tty_code_code, _: c_int, _: c_int, _: c_int);
    pub fn tty_putcode_s(_: *mut tty, _: tty_code_code, _: *const c_char);
    pub fn tty_putcode_ss(_: *mut tty, _: tty_code_code, _: *const c_char, _: *const c_char);
    pub fn tty_puts(_: *mut tty, _: *const c_char);
    pub fn tty_putc(_: *mut tty, _: c_uchar);
    pub fn tty_putn(_: *mut tty, _: *const c_void, _: usize, _: c_uint);
    pub fn tty_cell(_: *mut tty, _: *const grid_cell, _: *const grid_cell, _: *mut colour_palette, _: *mut hyperlinks);
    pub fn tty_init(_: *mut tty, _: *mut client) -> c_int;
    pub fn tty_resize(_: *mut tty);
    pub fn tty_set_size(_: *mut tty, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn tty_start_tty(_: *mut tty);
    pub fn tty_send_requests(_: *mut tty);
    pub fn tty_repeat_requests(_: *mut tty);
    pub fn tty_stop_tty(_: *mut tty);
    pub fn tty_set_title(_: *mut tty, _: *const c_char);
    pub fn tty_set_path(_: *mut tty, _: *const c_char);
    pub fn tty_update_mode(_: *mut tty, _: c_int, _: *mut screen);
    pub fn tty_draw_line(
        _: *mut tty,
        _: *mut screen,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: *const grid_cell,
        _: *mut colour_palette,
    );
    pub fn tty_sync_start(_: *mut tty);
    pub fn tty_sync_end(_: *mut tty);
    pub fn tty_open(_: *mut tty, _: *mut *mut c_char) -> c_int;
    pub fn tty_close(_: *mut tty);
    pub fn tty_free(_: *mut tty);
    pub fn tty_update_features(_: *mut tty);
    pub fn tty_set_selection(_: *mut tty, _: *const c_char, _: *const c_char, _: usize);
    pub fn tty_write(_: Option<unsafe extern "C" fn(_: *mut tty, _: *const tty_ctx)>, _: *mut tty_ctx);
    pub fn tty_cmd_alignmenttest(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_cell(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_cells(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearendofline(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearendofscreen(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearline(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearscreen(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearstartofline(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearstartofscreen(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_deletecharacter(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearcharacter(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_deleteline(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_insertcharacter(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_insertline(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_linefeed(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_scrollup(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_scrolldown(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_reverseindex(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_setselection(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_rawstring(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_syncstart(_: *mut tty, _: *const tty_ctx);
    pub fn tty_default_colours(_: *mut grid_cell, _: *mut window_pane);
}

// tty-term.c
unsafe extern "C" {
    pub static mut tty_terms: tty_terms;
    pub fn tty_term_ncodes() -> c_uint;
    pub fn tty_term_apply(_: *mut tty_term, _: *const c_char, _: c_int);
    pub fn tty_term_apply_overrides(_: *mut tty_term);
    pub fn tty_term_create(
        _: *mut tty,
        _: *mut c_char,
        _: *mut *mut c_char,
        _: c_uint,
        _: *mut c_int,
        _: *mut *mut c_char,
    ) -> *mut tty_term;
    pub fn tty_term_free(_: *mut tty_term);
    pub fn tty_term_read_list(
        _: *const c_char,
        _: c_int,
        _: *mut *mut *mut c_char,
        _: *mut c_uint,
        _: *mut *mut c_char,
    ) -> c_int;
    pub fn tty_term_free_list(_: *mut *mut c_char, _: c_uint);
    pub fn tty_term_has(_: *mut tty_term, _: tty_code_code) -> c_int;
    pub fn tty_term_string(_: *mut tty_term, _: tty_code_code) -> *const c_char;
    pub fn tty_term_string_i(_: *mut tty_term, _: tty_code_code, _: c_int) -> *const c_char;
    pub fn tty_term_string_ii(_: *mut tty_term, _: tty_code_code, _: c_int, _: c_int) -> *const c_char;
    pub fn tty_term_string_iii(_: *mut tty_term, _: tty_code_code, _: c_int, _: c_int, _: c_int) -> *const c_char;
    pub fn tty_term_string_s(_: *mut tty_term, _: tty_code_code, _: *const c_char) -> *const c_char;
    pub fn tty_term_string_ss(_: *mut tty_term, _: tty_code_code, _: *const c_char, _: *const c_char) -> *const c_char;
    pub fn tty_term_number(_: *mut tty_term, _: tty_code_code) -> c_int;
    pub fn tty_term_flag(_: *mut tty_term, _: tty_code_code) -> c_int;
    pub fn tty_term_describe(_: *mut tty_term, _: tty_code_code) -> *const c_char;
}

// tty-features.c
unsafe extern "C" {
    pub fn tty_add_features(_: *mut c_int, _: *const c_char, _: *const c_char);
    pub fn tty_get_features(_: c_int) -> *const c_char;
    pub fn tty_apply_features(_: *mut tty_term, _: c_int) -> c_int;
    pub fn tty_default_features(_: *mut c_int, _: *const c_char, _: c_uint);
}

/* tty-acs.c */
unsafe extern "C" {
    pub fn tty_acs_needed(_: *mut tty) -> c_int;
    pub fn tty_acs_get(_: *mut tty, _: c_uchar) -> *const c_char;
    pub fn tty_acs_reverse_get(_: *mut tty, _: *const c_char, _: usize) -> c_int;
    pub fn tty_acs_double_borders(_: c_int) -> *const utf8_data;
    pub fn tty_acs_heavy_borders(_: c_int) -> *const utf8_data;
    pub fn tty_acs_rounded_borders(_: c_int) -> *const utf8_data;
}
/* tty-keys.c */
unsafe extern "C" {

    pub fn tty_keys_build(_: *mut tty);
    pub fn tty_keys_free(_: *mut tty);
    pub fn tty_keys_next(_: *mut tty) -> c_int;
    pub fn tty_keys_colours(
        _: *mut tty,
        _: *const c_char,
        _: usize,
        _: *mut usize,
        _: *mut c_int,
        _: *mut c_int,
    ) -> c_int;
}
/* arguments.c */
unsafe extern "C" {
    pub fn args_set(_: *mut args, _: c_uchar, _: *mut args_value, _: c_int);
    pub fn args_create() -> *mut args;
    pub fn args_parse(_: *const args_parse, _: *mut args_value, _: c_uint, _: *mut *mut c_char) -> *mut args;
    pub fn args_copy(_: *mut args, _: c_int, _: *mut *mut c_char) -> *mut args;
    pub fn args_to_vector(_: *mut args, _: *mut c_int, _: *mut *mut *mut c_char);
    pub fn args_from_vector(_: c_int, _: *mut *mut c_char) -> *mut args_value;
    pub fn args_free_value(_: *mut args_value);
    pub fn args_free_values(_: *mut args_value, _: c_uint);
    pub fn args_free(_: *mut args);
    pub fn args_print(_: *mut args) -> *mut c_char;
    pub fn args_escape(_: *const c_char) -> *mut c_char;
    pub fn args_has(_: *mut args, _: c_uchar) -> c_int;
    pub fn args_get(_: *mut args, _: c_uchar) -> *const c_char;
    pub fn args_first(_: *mut args, _: *mut *mut args_entry) -> c_uchar;
    pub fn args_next(_: *mut *mut args_entry) -> c_uchar;
    pub fn args_count(_: *mut args) -> c_uint;
    pub fn args_values(_: *mut args) -> *mut args_value;
    pub fn args_value(_: *mut args, _: c_uint) -> *mut args_value;
    pub fn args_string(_: *mut args, _: c_uint) -> *const c_char;
    pub fn args_make_commands_now(_: *mut cmd, _: *mut cmdq_item, _: c_uint, _: c_int) -> *mut cmd_list;
    pub fn args_make_commands_prepare(
        _: *mut cmd,
        _: *mut cmdq_item,
        _: c_uint,
        _: *const c_char,
        _: c_int,
        _: c_int,
    ) -> *mut args_command_state;
    pub fn args_make_commands(
        _: *mut args_command_state,
        _: c_int,
        _: *mut *mut c_char,
        _: *mut *mut c_char,
    ) -> *mut cmd_list;
    pub fn args_make_commands_free(_: *mut args_command_state);
    pub fn args_make_commands_get_command(_: *mut args_command_state) -> *mut c_char;
    pub fn args_first_value(_: *mut args, _: c_uchar) -> *mut args_value;
    pub fn args_next_value(_: *mut args_value) -> *mut args_value;
    pub fn args_strtonum(_: *mut args, _: c_uchar, _: c_longlong, _: c_longlong, _: *mut *mut c_char) -> c_longlong;
    pub fn args_strtonum_and_expand(
        _: *mut args,
        _: c_uchar,
        _: c_longlong,
        _: c_longlong,
        _: *mut cmdq_item,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub fn args_percentage(
        _: *mut args,
        _: c_uchar,
        _: c_longlong,
        _: c_longlong,
        _: c_longlong,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub fn args_string_percentage(
        _: *const c_char,
        _: c_longlong,
        _: c_longlong,
        _: c_longlong,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub fn args_percentage_and_expand(
        _: *mut args,
        _: c_uchar,
        _: c_longlong,
        _: c_longlong,
        _: c_longlong,
        _: *mut cmdq_item,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub fn args_string_percentage_and_expand(
        _: *const c_char,
        _: c_longlong,
        _: c_longlong,
        _: c_longlong,
        _: *mut cmdq_item,
        _: *mut *mut c_char,
    ) -> c_longlong;
}
/* cmd-find.c */
unsafe extern "C" {
    pub fn cmd_find_target(
        _: *mut cmd_find_state,
        _: *mut cmdq_item,
        _: *const c_char,
        _: cmd_find_type,
        _: c_int,
    ) -> c_int;
    pub fn cmd_find_best_client(_: *mut session) -> *mut client;
    pub fn cmd_find_client(_: *mut cmdq_item, _: *const c_char, _: c_int) -> *mut client;
    pub fn cmd_find_clear_state(_: *mut cmd_find_state, _: c_int);
    pub fn cmd_find_empty_state(_: *mut cmd_find_state) -> c_int;
    pub fn cmd_find_valid_state(_: *mut cmd_find_state) -> c_int;
    pub fn cmd_find_copy_state(_: *mut cmd_find_state, _: *mut cmd_find_state);
    pub fn cmd_find_from_session(_: *mut cmd_find_state, _: *mut session, _: c_int);
    pub fn cmd_find_from_winlink(_: *mut cmd_find_state, _: *mut winlink, _: c_int);
    pub fn cmd_find_from_session_window(_: *mut cmd_find_state, _: *mut session, _: *mut window, _: c_int) -> c_int;
    pub fn cmd_find_from_window(_: *mut cmd_find_state, _: *mut window, _: c_int) -> c_int;
    pub fn cmd_find_from_winlink_pane(_: *mut cmd_find_state, _: *mut winlink, _: *mut window_pane, _: c_int);
    pub fn cmd_find_from_pane(_: *mut cmd_find_state, _: *mut window_pane, _: c_int) -> c_int;
    pub fn cmd_find_from_client(_: *mut cmd_find_state, _: *mut client, _: c_int) -> c_int;
    pub fn cmd_find_from_mouse(_: *mut cmd_find_state, _: *mut mouse_event, _: c_int) -> c_int;
    pub fn cmd_find_from_nothing(_: *mut cmd_find_state, _: c_int) -> c_int;
}
/* cmd.c */
unsafe extern "C" {
    pub static mut cmd_table: [*const cmd_entry; 0usize];
    pub fn cmd_log_argv(_: c_int, _: *mut *mut c_char, _: *const c_char, ...);
    pub fn cmd_prepend_argv(_: *mut c_int, _: *mut *mut *mut c_char, _: *const c_char);
    pub fn cmd_append_argv(_: *mut c_int, _: *mut *mut *mut c_char, _: *const c_char);
    pub fn cmd_pack_argv(_: c_int, _: *mut *mut c_char, _: *mut c_char, _: usize) -> c_int;
    pub fn cmd_unpack_argv(_: *mut c_char, _: usize, _: c_int, _: *mut *mut *mut c_char) -> c_int;
    pub fn cmd_copy_argv(_: c_int, _: *mut *mut c_char) -> *mut *mut c_char;
    pub fn cmd_free_argv(_: c_int, _: *mut *mut c_char);
    pub fn cmd_stringify_argv(_: c_int, _: *mut *mut c_char) -> *mut c_char;
    pub fn cmd_get_alias(_: *const c_char) -> *mut c_char;
    pub fn cmd_get_entry(_: *mut cmd) -> *const cmd_entry;
    pub fn cmd_get_args(_: *mut cmd) -> *mut args;
    pub fn cmd_get_group(_: *mut cmd) -> c_uint;
    pub fn cmd_get_source(_: *mut cmd, _: *mut *const c_char, _: *mut c_uint);
    pub fn cmd_parse(_: *mut args_value, _: c_uint, _: *const c_char, _: c_uint, _: *mut *mut c_char) -> *mut cmd;
    pub fn cmd_copy(_: *mut cmd, _: c_int, _: *mut *mut c_char) -> *mut cmd;
    pub fn cmd_free(_: *mut cmd);
    pub fn cmd_print(_: *mut cmd) -> *mut c_char;
    pub fn cmd_list_new() -> *mut cmd_list;
    pub fn cmd_list_copy(_: *mut cmd_list, _: c_int, _: *mut *mut c_char) -> *mut cmd_list;
    pub fn cmd_list_append(_: *mut cmd_list, _: *mut cmd);
    pub fn cmd_list_append_all(_: *mut cmd_list, _: *mut cmd_list);
    pub fn cmd_list_move(_: *mut cmd_list, _: *mut cmd_list);
    pub fn cmd_list_free(_: *mut cmd_list);
    pub fn cmd_list_print(_: *mut cmd_list, _: c_int) -> *mut c_char;
    pub fn cmd_list_first(_: *mut cmd_list) -> *mut cmd;
    pub fn cmd_list_next(_: *mut cmd) -> *mut cmd;
    pub fn cmd_list_all_have(_: *mut cmd_list, _: c_int) -> c_int;
    pub fn cmd_list_any_have(_: *mut cmd_list, _: c_int) -> c_int;
    pub fn cmd_mouse_at(_: *mut window_pane, _: *mut mouse_event, _: *mut c_uint, _: *mut c_uint, _: c_int) -> c_int;
    pub fn cmd_mouse_window(_: *mut mouse_event, _: *mut *mut session) -> *mut winlink;
    pub fn cmd_mouse_pane(_: *mut mouse_event, _: *mut *mut session, _: *mut *mut winlink) -> *mut window_pane;
    pub fn cmd_template_replace(_: *const c_char, _: *const c_char, _: c_int) -> *mut c_char;
}
/* cmd-attach-session.c */
unsafe extern "C" {
    pub fn cmd_attach_session(
        _: *mut cmdq_item,
        _: *const c_char,
        _: c_int,
        _: c_int,
        _: c_int,
        _: *const c_char,
        _: c_int,
        _: *const c_char,
    ) -> cmd_retval;
}
/* cmd-parse.c */
unsafe extern "C" {
    pub fn cmd_parse_from_file(_: *mut FILE, _: *mut cmd_parse_input) -> *mut cmd_parse_result;
    pub fn cmd_parse_from_string(_: *const c_char, _: *mut cmd_parse_input) -> *mut cmd_parse_result;
    pub fn cmd_parse_and_insert(
        _: *const c_char,
        _: *mut cmd_parse_input,
        _: *mut cmdq_item,
        _: *mut cmdq_state,
        _: *mut *mut c_char,
    ) -> cmd_parse_status;
    pub fn cmd_parse_and_append(
        _: *const c_char,
        _: *mut cmd_parse_input,
        _: *mut client,
        _: *mut cmdq_state,
        _: *mut *mut c_char,
    ) -> cmd_parse_status;
    pub fn cmd_parse_from_buffer(_: *const c_void, _: usize, _: *mut cmd_parse_input) -> *mut cmd_parse_result;
    pub fn cmd_parse_from_arguments(_: *mut args_value, _: c_uint, _: *mut cmd_parse_input) -> *mut cmd_parse_result;
}
/* cmd-queue.c */
unsafe extern "C" {
    pub fn cmdq_new_state(_: *mut cmd_find_state, _: *mut key_event, _: c_int) -> *mut cmdq_state;
    pub fn cmdq_link_state(_: *mut cmdq_state) -> *mut cmdq_state;
    pub fn cmdq_copy_state(_: *mut cmdq_state, _: *mut cmd_find_state) -> *mut cmdq_state;
    pub fn cmdq_free_state(_: *mut cmdq_state);
    pub fn cmdq_add_format(_: *mut cmdq_state, _: *const c_char, _: *const c_char, ...);
    pub fn cmdq_add_formats(_: *mut cmdq_state, _: *mut format_tree);
    pub fn cmdq_merge_formats(_: *mut cmdq_item, _: *mut format_tree);
    pub fn cmdq_new() -> *mut cmdq_list;
    pub fn cmdq_free(_: *mut cmdq_list);
    pub fn cmdq_get_name(_: *mut cmdq_item) -> *const c_char;
    pub fn cmdq_get_client(_: *mut cmdq_item) -> *mut client;
    pub fn cmdq_get_target_client(_: *mut cmdq_item) -> *mut client;
    pub fn cmdq_get_state(_: *mut cmdq_item) -> *mut cmdq_state;
    pub fn cmdq_get_target(_: *mut cmdq_item) -> *mut cmd_find_state;
    pub fn cmdq_get_source(_: *mut cmdq_item) -> *mut cmd_find_state;
    pub fn cmdq_get_event(_: *mut cmdq_item) -> *mut key_event;
    pub fn cmdq_get_current(_: *mut cmdq_item) -> *mut cmd_find_state;
    pub fn cmdq_get_flags(_: *mut cmdq_item) -> c_int;
    pub fn cmdq_get_command(_: *mut cmd_list, _: *mut cmdq_state) -> *mut cmdq_item;
    pub fn cmdq_get_callback1(_: *const c_char, _: cmdq_cb, _: *mut c_void) -> *mut cmdq_item;
    pub fn cmdq_get_error(_: *const c_char) -> *mut cmdq_item;
    pub fn cmdq_insert_after(_: *mut cmdq_item, _: *mut cmdq_item) -> *mut cmdq_item;
    pub fn cmdq_append(_: *mut client, _: *mut cmdq_item) -> *mut cmdq_item;
    pub fn cmdq_insert_hook(_: *mut session, _: *mut cmdq_item, _: *mut cmd_find_state, _: *const c_char, ...);
    pub fn cmdq_continue(_: *mut cmdq_item);
    pub fn cmdq_next(_: *mut client) -> c_uint;
    pub fn cmdq_running(_: *mut client) -> *mut cmdq_item;
    pub fn cmdq_guard(_: *mut cmdq_item, _: *const c_char, _: c_int);
    pub fn cmdq_print(_: *mut cmdq_item, _: *const c_char, ...);
    pub fn cmdq_print_data(_: *mut cmdq_item, _: c_int, _: *mut evbuffer);
    pub fn cmdq_error(_: *mut cmdq_item, _: *const c_char, ...);
}
/* cmd-wait-for.c */
unsafe extern "C" {
    pub fn cmd_wait_for_flush();
}
/* client.c */
unsafe extern "C" {
    pub fn client_main(_: *mut event_base, _: c_int, _: *mut *mut c_char, _: u64, _: c_int) -> c_int;
}
/* key-bindings.c */
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
/* key-string.c */
unsafe extern "C" {
    pub fn key_string_lookup_string(_: *const c_char) -> key_code;
    pub fn key_string_lookup_key(_: key_code, _: c_int) -> *const c_char;
}

// alerts.c
pub use crate::alerts::{alerts_check_session, alerts_queue, alerts_reset_all};

/* file.c */
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

// server.c
pub use crate::server::{
    clients, current_time, marked_pane, message_log, server_add_accept, server_add_message, server_check_marked,
    server_clear_marked, server_create_socket, server_is_marked, server_proc, server_set_marked, server_start,
    server_update_socket,
};

/* server-client.c */
unsafe extern "C" {
    pub fn client_windows_RB_INSERT_COLOR(_: *mut client_windows, _: *mut client_window);
    pub fn client_windows_RB_REMOVE_COLOR(_: *mut client_windows, _: *mut client_window, _: *mut client_window);
    pub fn client_windows_RB_REMOVE(_: *mut client_windows, _: *mut client_window) -> *mut client_window;
    pub fn client_windows_RB_INSERT(_: *mut client_windows, _: *mut client_window) -> *mut client_window;
    pub fn client_windows_RB_FIND(_: *mut client_windows, _: *mut client_window) -> *mut client_window;
    pub fn client_windows_RB_NFIND(_: *mut client_windows, _: *mut client_window) -> *mut client_window;
    pub fn server_client_how_many() -> c_uint;
    pub fn server_client_set_overlay(
        _: *mut client,
        _: c_uint,
        _: overlay_check_cb,
        _: overlay_mode_cb,
        _: overlay_draw_cb,
        _: overlay_key_cb,
        _: overlay_free_cb,
        _: overlay_resize_cb,
        _: *mut c_void,
    );
    pub fn server_client_clear_overlay(_: *mut client);
    pub fn server_client_overlay_range(
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: *mut overlay_ranges,
    );
    pub fn server_client_set_key_table(_: *mut client, _: *const c_char);
    pub fn server_client_get_key_table(_: *mut client) -> *const c_char;
    pub fn server_client_check_nested(_: *mut client) -> c_int;
    pub fn server_client_handle_key(_: *mut client, _: *mut key_event) -> c_int;
    pub fn server_client_create(_: c_int) -> *mut client;
    pub fn server_client_open(_: *mut client, _: *mut *mut c_char) -> c_int;
    pub fn server_client_unref(_: *mut client);
    pub fn server_client_set_session(_: *mut client, _: *mut session);
    pub fn server_client_lost(_: *mut client);
    pub fn server_client_suspend(_: *mut client);
    pub fn server_client_detach(_: *mut client, _: msgtype);
    pub fn server_client_exec(_: *mut client, _: *const c_char);
    pub fn server_client_loop();
    pub fn server_client_get_cwd(_: *mut client, _: *mut session) -> *const c_char;
    pub fn server_client_set_flags(_: *mut client, _: *const c_char);
    pub fn server_client_get_flags(_: *mut client) -> *const c_char;
    pub fn server_client_get_client_window(_: *mut client, _: c_uint) -> *mut client_window;
    pub fn server_client_add_client_window(_: *mut client, _: c_uint) -> *mut client_window;
    pub fn server_client_get_pane(_: *mut client) -> *mut window_pane;
    pub fn server_client_set_pane(_: *mut client, _: *mut window_pane);
    pub fn server_client_remove_pane(_: *mut window_pane);
    pub fn server_client_print(_: *mut client, _: c_int, _: *mut evbuffer);
}
/* server-fn.c */
unsafe extern "C" {
    pub fn server_redraw_client(_: *mut client);
    pub fn server_status_client(_: *mut client);
    pub fn server_redraw_session(_: *mut session);
    pub fn server_redraw_session_group(_: *mut session);
    pub fn server_status_session(_: *mut session);
    pub fn server_status_session_group(_: *mut session);
    pub fn server_redraw_window(_: *mut window);
    pub fn server_redraw_window_borders(_: *mut window);
    pub fn server_status_window(_: *mut window);
    pub fn server_lock();
    pub fn server_lock_session(_: *mut session);
    pub fn server_lock_client(_: *mut client);
    pub fn server_kill_pane(_: *mut window_pane);
    pub fn server_kill_window(_: *mut window, _: c_int);
    pub fn server_renumber_session(_: *mut session);
    pub fn server_renumber_all();
    pub fn server_link_window(
        _: *mut session,
        _: *mut winlink,
        _: *mut session,
        _: c_int,
        _: c_int,
        _: c_int,
        _: *mut *mut c_char,
    ) -> c_int;
    pub fn server_unlink_window(_: *mut session, _: *mut winlink);
    pub fn server_destroy_pane(_: *mut window_pane, _: c_int);
    pub fn server_destroy_session(_: *mut session);
    pub fn server_check_unattached();
    pub fn server_unzoom_window(_: *mut window);
}
/* status.c */
unsafe extern "C" {
    pub static mut status_prompt_hlist: [*mut *mut c_char; 0usize];
    pub static mut status_prompt_hsize: [c_uint; 0usize];
    pub fn status_timer_start(_: *mut client);
    pub fn status_timer_start_all();
    pub fn status_update_cache(_: *mut session);
    pub fn status_at_line(_: *mut client) -> c_int;
    pub fn status_line_size(_: *mut client) -> c_uint;
    pub fn status_get_range(_: *mut client, _: c_uint, _: c_uint) -> *mut style_range;
    pub fn status_init(_: *mut client);
    pub fn status_free(_: *mut client);
    pub fn status_redraw(_: *mut client) -> c_int;
    pub fn status_message_set(_: *mut client, _: c_int, _: c_int, _: c_int, _: *const c_char, ...);
    pub fn status_message_clear(_: *mut client);
    pub fn status_message_redraw(_: *mut client) -> c_int;
    pub fn status_prompt_set(
        _: *mut client,
        _: *mut cmd_find_state,
        _: *const c_char,
        _: *const c_char,
        _: prompt_input_cb,
        _: prompt_free_cb,
        _: *mut c_void,
        _: c_int,
        _: prompt_type,
    );
    pub fn status_prompt_clear(_: *mut client);
    pub fn status_prompt_redraw(_: *mut client) -> c_int;
    pub fn status_prompt_key(_: *mut client, _: key_code) -> c_int;
    pub fn status_prompt_update(_: *mut client, _: *const c_char, _: *const c_char);
    pub fn status_prompt_load_history();
    pub fn status_prompt_save_history();
    pub fn status_prompt_type_string(_: c_uint) -> *const c_char;
    pub fn status_prompt_type(type_: *const c_char) -> prompt_type;
}
/* resize.c */
unsafe extern "C" {
    pub fn resize_window(_: *mut window, _: c_uint, _: c_uint, _: c_int, _: c_int);
    pub fn default_window_size(
        _: *mut client,
        _: *mut session,
        _: *mut window,
        _: *mut c_uint,
        _: *mut c_uint,
        _: *mut c_uint,
        _: *mut c_uint,
        _: c_int,
    );
    pub fn recalculate_size(_: *mut window, _: c_int);
    pub fn recalculate_sizes();
    pub fn recalculate_sizes_now(_: c_int);
}
/* input.c */
unsafe extern "C" {
    pub fn input_init(_: *mut window_pane, _: *mut bufferevent, _: *mut colour_palette) -> *mut input_ctx;
    pub fn input_free(_: *mut input_ctx);
    pub fn input_reset(_: *mut input_ctx, _: c_int);
    pub fn input_pending(_: *mut input_ctx) -> *mut evbuffer;
    pub fn input_parse_pane(_: *mut window_pane);
    pub fn input_parse_buffer(_: *mut window_pane, _: *mut c_uchar, _: usize);
    pub fn input_parse_screen(
        _: *mut input_ctx,
        _: *mut screen,
        _: screen_write_init_ctx_cb,
        _: *mut c_void,
        _: *mut c_uchar,
        _: usize,
    );
    pub fn input_reply_clipboard(_: *mut bufferevent, _: *const c_char, _: usize, _: *const c_char);
}
/* input-key.c */
unsafe extern "C" {
    pub fn input_key_build();
    pub fn input_key_pane(_: *mut window_pane, _: key_code, _: *mut mouse_event) -> c_int;
    pub fn input_key(_: *mut screen, _: *mut bufferevent, _: key_code) -> c_int;
    pub fn input_key_get_mouse(
        _: *mut screen,
        _: *mut mouse_event,
        _: c_uint,
        _: c_uint,
        _: *mut *const c_char,
        _: *mut usize,
    ) -> c_int;
}
/* colour.c */
unsafe extern "C" {
    pub fn colour_find_rgb(_: c_uchar, _: c_uchar, _: c_uchar) -> c_int;
    pub fn colour_join_rgb(_: c_uchar, _: c_uchar, _: c_uchar) -> c_int;
    pub fn colour_split_rgb(_: c_int, _: *mut c_uchar, _: *mut c_uchar, _: *mut c_uchar);
    pub fn colour_force_rgb(_: c_int) -> c_int;
    pub fn colour_tostring(_: c_int) -> *const c_char;
    pub fn colour_fromstring(s: *const c_char) -> c_int;
    pub fn colour_256toRGB(_: c_int) -> c_int;
    pub fn colour_256to16(_: c_int) -> c_int;
    pub fn colour_byname(_: *const c_char) -> c_int;
    pub fn colour_parseX11(_: *const c_char) -> c_int;
    pub fn colour_palette_init(_: *mut colour_palette);
    pub fn colour_palette_clear(_: *mut colour_palette);
    pub fn colour_palette_free(_: *mut colour_palette);
    pub fn colour_palette_get(_: *mut colour_palette, _: c_int) -> c_int;
    pub fn colour_palette_set(_: *mut colour_palette, _: c_int, _: c_int) -> c_int;
    pub fn colour_palette_from_option(_: *mut colour_palette, _: *mut options);
}
/* attributes.c */
unsafe extern "C" {
    pub fn attributes_tostring(_: c_int) -> *const c_char;
    pub fn attributes_fromstring(_: *const c_char) -> c_int;
}
/* grid.c */
unsafe extern "C" {
    pub static grid_default_cell: grid_cell;
    pub fn grid_empty_line(_: *mut grid, _: c_uint, _: c_uint);
    pub fn grid_cells_equal(_: *const grid_cell, _: *const grid_cell) -> c_int;
    pub fn grid_cells_look_equal(_: *const grid_cell, _: *const grid_cell) -> c_int;
    pub fn grid_create(_: c_uint, _: c_uint, _: c_uint) -> *mut grid;
    pub fn grid_destroy(_: *mut grid);
    pub fn grid_compare(_: *mut grid, _: *mut grid) -> c_int;
    pub fn grid_collect_history(_: *mut grid);
    pub fn grid_remove_history(_: *mut grid, _: c_uint);
    pub fn grid_scroll_history(_: *mut grid, _: c_uint);
    pub fn grid_scroll_history_region(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_clear_history(_: *mut grid);
    pub fn grid_peek_line(_: *mut grid, _: c_uint) -> *const grid_line;
    pub fn grid_get_cell(_: *mut grid, _: c_uint, _: c_uint, _: *mut grid_cell);
    pub fn grid_set_cell(_: *mut grid, _: c_uint, _: c_uint, _: *const grid_cell);
    pub fn grid_set_padding(_: *mut grid, _: c_uint, _: c_uint);
    pub fn grid_set_cells(_: *mut grid, _: c_uint, _: c_uint, _: *const grid_cell, _: *const c_char, _: usize);
    pub fn grid_get_line(_: *mut grid, _: c_uint) -> *mut grid_line;
    pub fn grid_adjust_lines(_: *mut grid, _: c_uint);
    pub fn grid_clear(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_clear_lines(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_move_lines(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_move_cells(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_string_cells(
        _: *mut grid,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: *mut *mut grid_cell,
        _: c_int,
        _: *mut screen,
    ) -> *mut c_char;
    pub fn grid_duplicate_lines(_: *mut grid, _: c_uint, _: *mut grid, _: c_uint, _: c_uint);
    pub fn grid_reflow(_: *mut grid, _: c_uint);
    pub fn grid_wrap_position(_: *mut grid, _: c_uint, _: c_uint, _: *mut c_uint, _: *mut c_uint);
    pub fn grid_unwrap_position(_: *mut grid, _: *mut c_uint, _: *mut c_uint, _: c_uint, _: c_uint);
    pub fn grid_line_length(_: *mut grid, _: c_uint) -> c_uint;
}
/* grid-reader.c */
unsafe extern "C" {
    pub fn grid_reader_start(_: *mut grid_reader, _: *mut grid, _: c_uint, _: c_uint);
    pub fn grid_reader_get_cursor(_: *mut grid_reader, _: *mut c_uint, _: *mut c_uint);
    pub fn grid_reader_line_length(_: *mut grid_reader) -> c_uint;
    pub fn grid_reader_in_set(_: *mut grid_reader, _: *const c_char) -> c_int;
    pub fn grid_reader_cursor_right(_: *mut grid_reader, _: c_int, _: c_int);
    pub fn grid_reader_cursor_left(_: *mut grid_reader, _: c_int);
    pub fn grid_reader_cursor_down(_: *mut grid_reader);
    pub fn grid_reader_cursor_up(_: *mut grid_reader);
    pub fn grid_reader_cursor_start_of_line(_: *mut grid_reader, _: c_int);
    pub fn grid_reader_cursor_end_of_line(_: *mut grid_reader, _: c_int, _: c_int);
    pub fn grid_reader_cursor_next_word(_: *mut grid_reader, _: *const c_char);
    pub fn grid_reader_cursor_next_word_end(_: *mut grid_reader, _: *const c_char);
    pub fn grid_reader_cursor_previous_word(_: *mut grid_reader, _: *const c_char, _: c_int, _: c_int);
    pub fn grid_reader_cursor_jump(_: *mut grid_reader, _: *const utf8_data) -> c_int;
    pub fn grid_reader_cursor_jump_back(_: *mut grid_reader, _: *const utf8_data) -> c_int;
    pub fn grid_reader_cursor_back_to_indentation(_: *mut grid_reader);
}
/* grid-view.c */
unsafe extern "C" {
    pub fn grid_view_get_cell(_: *mut grid, _: c_uint, _: c_uint, _: *mut grid_cell);
    pub fn grid_view_set_cell(_: *mut grid, _: c_uint, _: c_uint, _: *const grid_cell);
    pub fn grid_view_set_padding(_: *mut grid, _: c_uint, _: c_uint);
    pub fn grid_view_set_cells(_: *mut grid, _: c_uint, _: c_uint, _: *const grid_cell, _: *const c_char, _: usize);
    pub fn grid_view_clear_history(_: *mut grid, _: c_uint);
    pub fn grid_view_clear(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_scroll_region_up(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_scroll_region_down(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_insert_lines(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_insert_lines_region(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_delete_lines(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_delete_lines_region(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_insert_cells(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_delete_cells(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_string_cells(_: *mut grid, _: c_uint, _: c_uint, _: c_uint) -> *mut c_char;
}
/* screen-write.c */
unsafe extern "C" {
    pub fn screen_write_make_list(_: *mut screen);
    pub fn screen_write_free_list(_: *mut screen);
    pub fn screen_write_start_pane(_: *mut screen_write_ctx, _: *mut window_pane, _: *mut screen);
    pub fn screen_write_start(_: *mut screen_write_ctx, _: *mut screen);
    pub fn screen_write_start_callback(
        _: *mut screen_write_ctx,
        _: *mut screen,
        _: screen_write_init_ctx_cb,
        _: *mut c_void,
    );
    pub fn screen_write_stop(_: *mut screen_write_ctx);
    pub fn screen_write_reset(_: *mut screen_write_ctx);
    pub fn screen_write_strlen(_: *const c_char, ...) -> usize;
    pub fn screen_write_text(
        _: *mut screen_write_ctx,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_int,
        _: *const grid_cell,
        _: *const c_char,
        ...
    ) -> c_int;
    pub fn screen_write_puts(_: *mut screen_write_ctx, _: *const grid_cell, _: *const c_char, ...);
    pub fn screen_write_nputs(_: *mut screen_write_ctx, _: isize, _: *const grid_cell, _: *const c_char, ...);
    pub fn screen_write_vnputs(
        _: *mut screen_write_ctx,
        _: isize,
        _: *const grid_cell,
        _: *const c_char,
        _: *mut VaList,
    );
    pub fn screen_write_putc(_: *mut screen_write_ctx, _: *const grid_cell, _: c_uchar);
    pub fn screen_write_fast_copy(_: *mut screen_write_ctx, _: *mut screen, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn screen_write_hline(
        _: *mut screen_write_ctx,
        _: c_uint,
        _: c_int,
        _: c_int,
        _: box_lines,
        _: *const grid_cell,
    );
    pub fn screen_write_vline(_: *mut screen_write_ctx, _: c_uint, _: c_int, _: c_int);
    pub fn screen_write_menu(
        _: *mut screen_write_ctx,
        _: *mut menu,
        _: c_int,
        _: box_lines,
        _: *const grid_cell,
        _: *const grid_cell,
        _: *const grid_cell,
    );
    pub fn screen_write_box(
        _: *mut screen_write_ctx,
        _: c_uint,
        _: c_uint,
        _: box_lines,
        _: *const grid_cell,
        _: *const c_char,
    );
    pub fn screen_write_preview(_: *mut screen_write_ctx, _: *mut screen, _: c_uint, _: c_uint);
    pub fn screen_write_backspace(_: *mut screen_write_ctx);
    pub fn screen_write_mode_set(_: *mut screen_write_ctx, _: c_int);
    pub fn screen_write_mode_clear(_: *mut screen_write_ctx, _: c_int);
    pub fn screen_write_cursorup(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_cursordown(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_cursorright(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_cursorleft(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_alignmenttest(_: *mut screen_write_ctx);
    pub fn screen_write_insertcharacter(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_deletecharacter(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_clearcharacter(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_insertline(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_deleteline(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_clearline(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_clearendofline(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_clearstartofline(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_cursormove(_: *mut screen_write_ctx, _: c_int, _: c_int, _: c_int);
    pub fn screen_write_reverseindex(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_scrollregion(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_linefeed(_: *mut screen_write_ctx, _: c_int, _: c_uint);
    pub fn screen_write_scrollup(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_scrolldown(_: *mut screen_write_ctx, _: c_uint, _: c_uint);
    pub fn screen_write_carriagereturn(_: *mut screen_write_ctx);
    pub fn screen_write_clearendofscreen(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_clearstartofscreen(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_clearscreen(_: *mut screen_write_ctx, _: c_uint);
    pub fn screen_write_clearhistory(_: *mut screen_write_ctx);
    pub fn screen_write_fullredraw(_: *mut screen_write_ctx);
    pub fn screen_write_collect_end(_: *mut screen_write_ctx);
    pub fn screen_write_collect_add(_: *mut screen_write_ctx, _: *const grid_cell);
    pub fn screen_write_cell(_: *mut screen_write_ctx, _: *const grid_cell);
    pub fn screen_write_setselection(_: *mut screen_write_ctx, _: *const c_char, _: *mut c_uchar, _: c_uint);
    pub fn screen_write_rawstring(_: *mut screen_write_ctx, _: *mut c_uchar, _: c_uint, _: c_int);
    pub fn screen_write_alternateon(_: *mut screen_write_ctx, _: *mut grid_cell, _: c_int);
    pub fn screen_write_alternateoff(_: *mut screen_write_ctx, _: *mut grid_cell, _: c_int);
}
/* screen-redraw.c */
unsafe extern "C" {
    pub fn screen_redraw_screen(_: *mut client);
    pub fn screen_redraw_pane(_: *mut client, _: *mut window_pane);
}
/* screen.c */
unsafe extern "C" {
    pub fn screen_init(_: *mut screen, _: c_uint, _: c_uint, _: c_uint);
    pub fn screen_reinit(_: *mut screen);
    pub fn screen_free(_: *mut screen);
    pub fn screen_reset_tabs(_: *mut screen);
    pub fn screen_reset_hyperlinks(_: *mut screen);
    pub fn screen_set_cursor_style(_: c_uint, _: *mut screen_cursor_style, _: *mut c_int);
    pub fn screen_set_cursor_colour(_: *mut screen, _: c_int);
    pub fn screen_set_title(_: *mut screen, _: *const c_char) -> c_int;
    pub fn screen_set_path(_: *mut screen, _: *const c_char);
    pub fn screen_push_title(_: *mut screen);
    pub fn screen_pop_title(_: *mut screen);
    pub fn screen_resize(_: *mut screen, _: c_uint, _: c_uint, _: c_int);
    pub fn screen_resize_cursor(_: *mut screen, _: c_uint, _: c_uint, _: c_int, _: c_int, _: c_int);
    pub fn screen_set_selection(
        _: *mut screen,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_int,
        _: *mut grid_cell,
    );
    pub fn screen_clear_selection(_: *mut screen);
    pub fn screen_hide_selection(_: *mut screen);
    pub fn screen_check_selection(_: *mut screen, _: c_uint, _: c_uint) -> c_int;
    pub fn screen_select_cell(_: *mut screen, _: *mut grid_cell, _: *const grid_cell);
    pub fn screen_alternate_on(_: *mut screen, _: *mut grid_cell, _: c_int);
    pub fn screen_alternate_off(_: *mut screen, _: *mut grid_cell, _: c_int);
    pub fn screen_mode_to_string(_: c_int) -> *const c_char;
}

// window.c
pub use crate::window_::*;
unsafe extern "C" {
    pub static mut windows: windows;
    pub static mut all_window_panes: window_pane_tree;
    pub fn window_cmp(_: *mut window, _: *mut window) -> c_int;
    pub fn windows_RB_INSERT_COLOR(_: *mut windows, _: *mut window);
    pub fn windows_RB_REMOVE_COLOR(_: *mut windows, _: *mut window, _: *mut window);
    pub fn windows_RB_REMOVE(_: *mut windows, _: *mut window) -> *mut window;
    pub fn windows_RB_INSERT(_: *mut windows, _: *mut window) -> *mut window;
    pub fn windows_RB_FIND(_: *mut windows, _: *mut window) -> *mut window;
    pub fn windows_RB_NFIND(_: *mut windows, _: *mut window) -> *mut window;
    pub fn winlink_cmp(_: *mut winlink, _: *mut winlink) -> c_int;
    pub fn winlinks_RB_INSERT_COLOR(_: *mut winlinks, _: *mut winlink);
    pub fn winlinks_RB_REMOVE_COLOR(_: *mut winlinks, _: *mut winlink, _: *mut winlink);
    pub fn winlinks_RB_REMOVE(_: *mut winlinks, _: *mut winlink) -> *mut winlink;
    pub fn winlinks_RB_INSERT(_: *mut winlinks, _: *mut winlink) -> *mut winlink;
    pub fn winlinks_RB_FIND(_: *mut winlinks, _: *mut winlink) -> *mut winlink;
    pub fn winlinks_RB_NFIND(_: *mut winlinks, _: *mut winlink) -> *mut winlink;
    pub fn window_pane_cmp(_: *mut window_pane, _: *mut window_pane) -> c_int;
    pub fn window_pane_tree_RB_INSERT_COLOR(_: *mut window_pane_tree, _: *mut window_pane);
    pub fn window_pane_tree_RB_REMOVE_COLOR(_: *mut window_pane_tree, _: *mut window_pane, _: *mut window_pane);
    pub fn window_pane_tree_RB_REMOVE(_: *mut window_pane_tree, _: *mut window_pane) -> *mut window_pane;
    pub fn window_pane_tree_RB_INSERT(_: *mut window_pane_tree, _: *mut window_pane) -> *mut window_pane;
    pub fn window_pane_tree_RB_FIND(_: *mut window_pane_tree, _: *mut window_pane) -> *mut window_pane;
    pub fn window_pane_tree_RB_NFIND(_: *mut window_pane_tree, _: *mut window_pane) -> *mut window_pane;
    pub fn winlink_find_by_index(_: *mut winlinks, _: c_int) -> *mut winlink;
    pub fn winlink_find_by_window(_: *mut winlinks, _: *mut window) -> *mut winlink;
    pub fn winlink_find_by_window_id(_: *mut winlinks, _: c_uint) -> *mut winlink;
    pub fn winlink_count(_: *mut winlinks) -> c_uint;
    pub fn winlink_add(_: *mut winlinks, _: c_int) -> *mut winlink;
    pub fn winlink_set_window(_: *mut winlink, _: *mut window);
    pub fn winlink_remove(_: *mut winlinks, _: *mut winlink);
    pub fn winlink_next(_: *mut winlink) -> *mut winlink;
    pub fn winlink_previous(_: *mut winlink) -> *mut winlink;
    pub fn winlink_next_by_number(_: *mut winlink, _: *mut session, _: c_int) -> *mut winlink;
    pub fn winlink_previous_by_number(_: *mut winlink, _: *mut session, _: c_int) -> *mut winlink;
    pub fn winlink_stack_push(_: *mut winlink_stack, _: *mut winlink);
    pub fn winlink_stack_remove(_: *mut winlink_stack, _: *mut winlink);
    pub fn window_find_by_id_str(_: *const c_char) -> *mut window;
    pub fn window_find_by_id(_: c_uint) -> *mut window;
    pub fn window_update_activity(_: *mut window);
    pub fn window_create(_: c_uint, _: c_uint, _: c_uint, _: c_uint) -> *mut window;
    pub fn window_pane_set_event(_: *mut window_pane);
    pub fn window_get_active_at(_: *mut window, _: c_uint, _: c_uint) -> *mut window_pane;
    pub fn window_find_string(_: *mut window, _: *const c_char) -> *mut window_pane;
    pub fn window_has_pane(_: *mut window, _: *mut window_pane) -> c_int;
    pub fn window_set_active_pane(_: *mut window, _: *mut window_pane, _: c_int) -> c_int;
    pub fn window_update_focus(_: *mut window);
    pub fn window_pane_update_focus(_: *mut window_pane);
    pub fn window_redraw_active_switch(_: *mut window, _: *mut window_pane);
    pub fn window_add_pane(_: *mut window, _: *mut window_pane, _: c_uint, _: c_int) -> *mut window_pane;
    pub fn window_resize(_: *mut window, _: c_uint, _: c_uint, _: c_int, _: c_int);
    pub fn window_pane_send_resize(_: *mut window_pane, _: c_uint, _: c_uint);
    pub fn window_zoom(_: *mut window_pane) -> c_int;
    pub fn window_unzoom(_: *mut window, _: c_int) -> c_int;
    pub fn window_push_zoom(_: *mut window, _: c_int, _: c_int) -> c_int;
    pub fn window_pop_zoom(_: *mut window) -> c_int;
    pub fn window_lost_pane(_: *mut window, _: *mut window_pane);
    pub fn window_remove_pane(_: *mut window, _: *mut window_pane);
    pub fn window_pane_at_index(_: *mut window, _: c_uint) -> *mut window_pane;
    pub fn window_pane_next_by_number(_: *mut window, _: *mut window_pane, _: c_uint) -> *mut window_pane;
    pub fn window_pane_previous_by_number(_: *mut window, _: *mut window_pane, _: c_uint) -> *mut window_pane;
    pub fn window_pane_index(_: *mut window_pane, _: *mut c_uint) -> c_int;
    pub fn window_count_panes(_: *mut window) -> c_uint;
    pub fn window_destroy_panes(_: *mut window);
    pub fn window_pane_find_by_id_str(_: *const c_char) -> *mut window_pane;
    pub fn window_pane_find_by_id(_: c_uint) -> *mut window_pane;
    pub fn window_pane_destroy_ready(_: *mut window_pane) -> c_int;
    pub fn window_pane_resize(_: *mut window_pane, _: c_uint, _: c_uint);
    pub fn window_pane_set_mode(
        _: *mut window_pane,
        _: *mut window_pane,
        _: *const window_mode,
        _: *mut cmd_find_state,
        _: *mut args,
    ) -> c_int;
    pub fn window_pane_reset_mode(_: *mut window_pane);
    pub fn window_pane_reset_mode_all(_: *mut window_pane);
    pub fn window_pane_key(
        _: *mut window_pane,
        _: *mut client,
        _: *mut session,
        _: *mut winlink,
        _: key_code,
        _: *mut mouse_event,
    ) -> c_int;
    pub fn window_pane_visible(_: *mut window_pane) -> c_int;
    pub fn window_pane_exited(_: *mut window_pane) -> c_int;
    pub fn window_pane_search(_: *mut window_pane, _: *const c_char, _: c_int, _: c_int) -> c_uint;
    pub fn window_printable_flags(_: *mut winlink, _: c_int) -> *const c_char;
    pub fn window_pane_find_up(_: *mut window_pane) -> *mut window_pane;
    pub fn window_pane_find_down(_: *mut window_pane) -> *mut window_pane;
    pub fn window_pane_find_left(_: *mut window_pane) -> *mut window_pane;
    pub fn window_pane_find_right(_: *mut window_pane) -> *mut window_pane;
    pub fn window_pane_stack_push(_: *mut window_panes, _: *mut window_pane);
    pub fn window_pane_stack_remove(_: *mut window_panes, _: *mut window_pane);
    pub fn window_set_name(_: *mut window, _: *const c_char);
    pub fn window_add_ref(_: *mut window, _: *const c_char);
    pub fn window_remove_ref(_: *mut window, _: *const c_char);
    pub fn winlink_clear_flags(_: *mut winlink);
    pub fn winlink_shuffle_up(_: *mut session, _: *mut winlink, _: c_int) -> c_int;
    pub fn window_pane_start_input(_: *mut window_pane, _: *mut cmdq_item, _: *mut *mut c_char) -> c_int;
    pub fn window_pane_get_new_data(_: *mut window_pane, _: *mut window_pane_offset, _: *mut usize) -> *mut c_void;
    pub fn window_pane_update_used_data(_: *mut window_pane, _: *mut window_pane_offset, _: usize);
    pub fn window_set_fill_character(_: *mut window);
    pub fn window_pane_default_cursor(_: *mut window_pane);
    pub fn window_pane_mode(_: *mut window_pane) -> c_int;
}
/* layout.c */
unsafe extern "C" {
    pub fn layout_count_cells(_: *mut layout_cell) -> c_uint;
    pub fn layout_create_cell(_: *mut layout_cell) -> *mut layout_cell;
    pub fn layout_free_cell(_: *mut layout_cell);
    pub fn layout_print_cell(_: *mut layout_cell, _: *const c_char, _: c_uint);
    pub fn layout_destroy_cell(_: *mut window, _: *mut layout_cell, _: *mut *mut layout_cell);
    pub fn layout_resize_layout(_: *mut window, _: *mut layout_cell, _: layout_type, _: c_int, _: c_int);
    pub fn layout_search_by_border(_: *mut layout_cell, _: c_uint, _: c_uint) -> *mut layout_cell;
    pub fn layout_set_size(_: *mut layout_cell, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn layout_make_leaf(_: *mut layout_cell, _: *mut window_pane);
    pub fn layout_make_node(_: *mut layout_cell, _: layout_type);
    pub fn layout_fix_offsets(_: *mut window);
    pub fn layout_fix_panes(_: *mut window, _: *mut window_pane);
    pub fn layout_resize_adjust(_: *mut window, _: *mut layout_cell, _: layout_type, _: c_int);
    pub fn layout_init(_: *mut window, _: *mut window_pane);
    pub fn layout_free(_: *mut window);
    pub fn layout_resize(_: *mut window, _: c_uint, _: c_uint);
    pub fn layout_resize_pane(_: *mut window_pane, _: layout_type, _: c_int, _: c_int);
    pub fn layout_resize_pane_to(_: *mut window_pane, _: layout_type, _: c_uint);
    pub fn layout_assign_pane(_: *mut layout_cell, _: *mut window_pane, _: c_int);
    pub fn layout_split_pane(_: *mut window_pane, _: layout_type, _: c_int, _: c_int) -> *mut layout_cell;
    pub fn layout_close_pane(_: *mut window_pane);
    pub fn layout_spread_cell(_: *mut window, _: *mut layout_cell) -> c_int;
    pub fn layout_spread_out(_: *mut window_pane);
}
/* layout-custom.c */
unsafe extern "C" {
    pub fn layout_dump(_: *mut layout_cell) -> *mut c_char;
    pub fn layout_parse(_: *mut window, _: *const c_char, _: *mut *mut c_char) -> c_int;
}
/* layout-set.c */
unsafe extern "C" {
    pub fn layout_set_lookup(_: *const c_char) -> c_int;
    pub fn layout_set_select(_: *mut window, _: c_uint) -> c_uint;
    pub fn layout_set_next(_: *mut window) -> c_uint;
    pub fn layout_set_previous(_: *mut window) -> c_uint;
}
/* mode-tree.c */
pub type mode_tree_build_cb = ::std::option::Option<
    unsafe extern "C" fn(_: *mut c_void, _: *mut mode_tree_sort_criteria, _: *mut u64, _: *const c_char),
>;
pub type mode_tree_draw_cb = ::std::option::Option<
    unsafe extern "C" fn(_: *mut c_void, _: *mut c_void, _: *mut screen_write_ctx, _: c_uint, _: c_uint),
>;
pub type mode_tree_search_cb =
    ::std::option::Option<unsafe extern "C" fn(_: *mut c_void, _: *mut c_void, _: *const c_char) -> c_int>;
pub type mode_tree_menu_cb = ::std::option::Option<unsafe extern "C" fn(_: *mut c_void, _: *mut client, _: key_code)>;
pub type mode_tree_height_cb = ::std::option::Option<unsafe extern "C" fn(_: *mut c_void, _: c_uint) -> c_uint>;
pub type mode_tree_key_cb =
    ::std::option::Option<unsafe extern "C" fn(_: *mut c_void, _: *mut c_void, _: c_uint) -> key_code>;
pub type mode_tree_each_cb =
    ::std::option::Option<unsafe extern "C" fn(_: *mut c_void, _: *mut c_void, _: *mut client, _: key_code)>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct mode_tree_item {
    _unused: [u8; 0],
}
unsafe extern "C" {
    pub fn mode_tree_count_tagged(_: *mut mode_tree_data) -> c_uint;
    pub fn mode_tree_get_current(_: *mut mode_tree_data) -> *mut c_void;
    pub fn mode_tree_get_current_name(_: *mut mode_tree_data) -> *const c_char;
    pub fn mode_tree_expand_current(_: *mut mode_tree_data);
    pub fn mode_tree_collapse_current(_: *mut mode_tree_data);
    pub fn mode_tree_expand(_: *mut mode_tree_data, _: u64);
    pub fn mode_tree_set_current(_: *mut mode_tree_data, _: u64) -> c_int;
    pub fn mode_tree_each_tagged(_: *mut mode_tree_data, _: mode_tree_each_cb, _: *mut client, _: key_code, _: c_int);
    pub fn mode_tree_up(_: *mut mode_tree_data, _: c_int);
    pub fn mode_tree_down(_: *mut mode_tree_data, _: c_int) -> c_int;
    pub fn mode_tree_start(
        _: *mut window_pane,
        _: *mut args,
        _: mode_tree_build_cb,
        _: mode_tree_draw_cb,
        _: mode_tree_search_cb,
        _: mode_tree_menu_cb,
        _: mode_tree_height_cb,
        _: mode_tree_key_cb,
        _: *mut c_void,
        _: *const menu_item,
        _: *mut *const c_char,
        _: c_uint,
        _: *mut *mut screen,
    ) -> *mut mode_tree_data;
    pub fn mode_tree_zoom(_: *mut mode_tree_data, _: *mut args);
    pub fn mode_tree_build(_: *mut mode_tree_data);
    pub fn mode_tree_free(_: *mut mode_tree_data);
    pub fn mode_tree_resize(_: *mut mode_tree_data, _: c_uint, _: c_uint);
    pub fn mode_tree_add(
        _: *mut mode_tree_data,
        _: *mut mode_tree_item,
        _: *mut c_void,
        _: u64,
        _: *const c_char,
        _: *const c_char,
        _: c_int,
    ) -> *mut mode_tree_item;
    pub fn mode_tree_draw_as_parent(_: *mut mode_tree_item);
    pub fn mode_tree_no_tag(_: *mut mode_tree_item);
    pub fn mode_tree_remove(_: *mut mode_tree_data, _: *mut mode_tree_item);
    pub fn mode_tree_draw(_: *mut mode_tree_data);
    pub fn mode_tree_key(
        _: *mut mode_tree_data,
        _: *mut client,
        _: *mut key_code,
        _: *mut mouse_event,
        _: *mut c_uint,
        _: *mut c_uint,
    ) -> c_int;
    pub fn mode_tree_run_command(_: *mut client, _: *mut cmd_find_state, _: *const c_char, _: *const c_char);
}
/* window-buffer.c */
unsafe extern "C" {
    pub static window_buffer_mode: window_mode;
}
/* window-tree.c */
unsafe extern "C" {
    pub static window_tree_mode: window_mode;
}
/* window-clock.c */
unsafe extern "C" {
    pub static window_clock_mode: window_mode;
    pub static mut window_clock_table: [[[c_char; 5usize]; 5usize]; 14usize];
}
/* window-client.c */
unsafe extern "C" {
    pub static window_client_mode: window_mode;
}

// window-copy.c
// pub use crate::window_copy::{window_copy_mode, window_view_mode};
/* window-copy.c */
unsafe extern "C" {
    pub static mut window_copy_mode: window_mode;
    pub static mut window_view_mode: window_mode;
    pub fn window_copy_add(_: *mut window_pane, _: c_int, _: *const c_char, ...);
    pub fn window_copy_vadd(_: *mut window_pane, _: c_int, _: *const c_char, _: *mut VaList);
    pub fn window_copy_pageup(_: *mut window_pane, _: c_int);
    pub fn window_copy_pagedown(_: *mut window_pane, _: c_int, _: c_int);
    pub fn window_copy_start_drag(_: *mut client, _: *mut mouse_event);
    pub fn window_copy_get_word(_: *mut window_pane, _: c_uint, _: c_uint) -> *mut c_char;
    pub fn window_copy_get_line(_: *mut window_pane, _: c_uint) -> *mut c_char;
}
/* window-option.c */
unsafe extern "C" {
    pub static window_customize_mode: window_mode;
}
/* names.c */
unsafe extern "C" {
    pub fn check_window_name(_: *mut window);
    pub fn default_window_name(_: *mut window) -> *mut c_char;
    pub fn parse_window_name(_: *const c_char) -> *mut c_char;
}
/* control.c */
unsafe extern "C" {
    pub fn control_discard(_: *mut client);
    pub fn control_start(_: *mut client);
    pub fn control_ready(_: *mut client);
    pub fn control_stop(_: *mut client);
    pub fn control_set_pane_on(_: *mut client, _: *mut window_pane);
    pub fn control_set_pane_off(_: *mut client, _: *mut window_pane);
    pub fn control_continue_pane(_: *mut client, _: *mut window_pane);
    pub fn control_pause_pane(_: *mut client, _: *mut window_pane);
    pub fn control_pane_offset(_: *mut client, _: *mut window_pane, _: *mut c_int) -> *mut window_pane_offset;
    pub fn control_reset_offsets(_: *mut client);
    pub fn control_write(_: *mut client, _: *const c_char, ...);
    pub fn control_write_output(_: *mut client, _: *mut window_pane);
    pub fn control_all_done(_: *mut client) -> c_int;
    pub fn control_add_sub(_: *mut client, _: *const c_char, _: control_sub_type, _: c_int, _: *const c_char);
    pub fn control_remove_sub(_: *mut client, _: *const c_char);
}
/* control-notify.c */
unsafe extern "C" {
    pub fn control_notify_pane_mode_changed(_: c_int);
    pub fn control_notify_window_layout_changed(_: *mut window);
    pub fn control_notify_window_pane_changed(_: *mut window);
    pub fn control_notify_window_unlinked(_: *mut session, _: *mut window);
    pub fn control_notify_window_linked(_: *mut session, _: *mut window);
    pub fn control_notify_window_renamed(_: *mut window);
    pub fn control_notify_client_session_changed(_: *mut client);
    pub fn control_notify_client_detached(_: *mut client);
    pub fn control_notify_session_renamed(_: *mut session);
    pub fn control_notify_session_created(_: *mut session);
    pub fn control_notify_session_closed(_: *mut session);
    pub fn control_notify_session_window_changed(_: *mut session);
    pub fn control_notify_paste_buffer_changed(_: *const c_char);
    pub fn control_notify_paste_buffer_deleted(_: *const c_char);
}
/* session.c */
unsafe extern "C" {
    pub static mut sessions: sessions;
    pub static mut next_session_id: c_uint;
    pub fn session_cmp(_: *mut session, _: *mut session) -> c_int;
    pub fn sessions_RB_INSERT_COLOR(_: *mut sessions, _: *mut session);
    pub fn sessions_RB_REMOVE_COLOR(_: *mut sessions, _: *mut session, _: *mut session);
    pub fn sessions_RB_REMOVE(_: *mut sessions, _: *mut session) -> *mut session;
    pub fn sessions_RB_INSERT(_: *mut sessions, _: *mut session) -> *mut session;
    pub fn sessions_RB_FIND(_: *mut sessions, _: *mut session) -> *mut session;
    pub fn sessions_RB_NFIND(_: *mut sessions, _: *mut session) -> *mut session;
    pub fn session_alive(_: *mut session) -> c_int;
    pub fn session_find(_: *const c_char) -> *mut session;
    pub fn session_find_by_id_str(_: *const c_char) -> *mut session;
    pub fn session_find_by_id(_: c_uint) -> *mut session;
    pub fn session_create(
        _: *const c_char,
        _: *const c_char,
        _: *const c_char,
        _: *mut environ,
        _: *mut options,
        _: *mut termios,
    ) -> *mut session;
    pub fn session_destroy(_: *mut session, _: c_int, _: *const c_char);
    pub fn session_add_ref(_: *mut session, _: *const c_char);
    pub fn session_remove_ref(_: *mut session, _: *const c_char);
    pub fn session_check_name(_: *const c_char) -> *mut c_char;
    pub fn session_update_activity(_: *mut session, _: *mut timeval);
    pub fn session_next_session(_: *mut session) -> *mut session;
    pub fn session_previous_session(_: *mut session) -> *mut session;
    pub fn session_attach(_: *mut session, _: *mut window, _: c_int, _: *mut *mut c_char) -> *mut winlink;
    pub fn session_detach(_: *mut session, _: *mut winlink) -> c_int;
    pub fn session_has(_: *mut session, _: *mut window) -> c_int;
    pub fn session_is_linked(_: *mut session, _: *mut window) -> c_int;
    pub fn session_next(_: *mut session, _: c_int) -> c_int;
    pub fn session_previous(_: *mut session, _: c_int) -> c_int;
    pub fn session_select(_: *mut session, _: c_int) -> c_int;
    pub fn session_last(_: *mut session) -> c_int;
    pub fn session_set_current(_: *mut session, _: *mut winlink) -> c_int;
    pub fn session_group_contains(_: *mut session) -> *mut session_group;
    pub fn session_group_find(_: *const c_char) -> *mut session_group;
    pub fn session_group_new(_: *const c_char) -> *mut session_group;
    pub fn session_group_add(_: *mut session_group, _: *mut session);
    pub fn session_group_synchronize_to(_: *mut session);
    pub fn session_group_synchronize_from(_: *mut session);
    pub fn session_group_count(_: *mut session_group) -> c_uint;
    pub fn session_group_attached_count(_: *mut session_group) -> c_uint;
    pub fn session_renumber_windows(_: *mut session);
}
/* utf8.c */
unsafe extern "C" {
    pub fn utf8_towc(_: *const utf8_data, _: *mut wchar_t) -> utf8_state;
    pub fn utf8_fromwc(wc: wchar_t, _: *mut utf8_data) -> utf8_state;
    pub fn utf8_in_table(_: wchar_t, _: *const wchar_t, _: c_uint) -> c_int;
    pub fn utf8_build_one(_: c_uchar) -> utf8_char;
    pub fn utf8_from_data(_: *const utf8_data, _: *mut utf8_char) -> utf8_state;
    pub fn utf8_to_data(_: utf8_char, _: *mut utf8_data);
    pub fn utf8_set(_: *mut utf8_data, _: c_uchar);
    pub fn utf8_copy(_: *mut utf8_data, _: *const utf8_data);
    pub fn utf8_open(_: *mut utf8_data, _: c_uchar) -> utf8_state;
    pub fn utf8_append(_: *mut utf8_data, _: c_uchar) -> utf8_state;
    pub fn utf8_isvalid(_: *const c_char) -> c_int;
    pub fn utf8_strvis(_: *mut c_char, _: *const c_char, _: usize, _: c_int) -> c_int;
    pub fn utf8_stravis(_: *mut *mut c_char, _: *const c_char, _: c_int) -> c_int;
    pub fn utf8_stravisx(_: *mut *mut c_char, _: *const c_char, _: usize, _: c_int) -> c_int;
    pub fn utf8_sanitize(_: *const c_char) -> *mut c_char;
    pub fn utf8_strlen(_: *const utf8_data) -> usize;
    pub fn utf8_strwidth(_: *const utf8_data, _: isize) -> c_uint;
    pub fn utf8_fromcstr(_: *const c_char) -> *mut utf8_data;
    pub fn utf8_tocstr(_: *mut utf8_data) -> *mut c_char;
    pub fn utf8_cstrwidth(_: *const c_char) -> c_uint;
    pub fn utf8_padcstr(_: *const c_char, _: c_uint) -> *mut c_char;
    pub fn utf8_rpadcstr(_: *const c_char, _: c_uint) -> *mut c_char;
    pub fn utf8_cstrhas(_: *const c_char, _: *const utf8_data) -> c_int;
}
/* osdep-*.c */
unsafe extern "C" {
    pub fn osdep_get_name(_: c_int, _: *mut c_char) -> *mut c_char;
    pub fn osdep_get_cwd(_: c_int) -> *mut c_char;
    pub fn osdep_event_init() -> *mut event_base;
}
/* utf8-combined.c */
unsafe extern "C" {
    pub fn utf8_has_zwj(_: *const utf8_data) -> c_int;
    pub fn utf8_is_zwj(_: *const utf8_data) -> c_int;
    pub fn utf8_is_vs(_: *const utf8_data) -> c_int;
    pub fn utf8_is_modifier(_: *const utf8_data) -> c_int;
}
/* procname.c */
unsafe extern "C" {
    pub fn get_proc_name(_: c_int, _: *mut c_char) -> *mut c_char;
    pub fn get_proc_cwd(_: c_int) -> *mut c_char;
}
/* log.c */
unsafe extern "C" {
    pub fn log_add_level();
    pub fn log_get_level() -> c_int;
    pub fn log_open(_: *const c_char);
    pub fn log_toggle(_: *const c_char);
    pub fn log_close();
    pub fn log_debug(_: *const c_char, ...);
    pub fn fatal(_: *const c_char, ...) -> !;
    pub fn fatalx(_: *const c_char, ...) -> !;
}
/* menu.c */
unsafe extern "C" {
    pub fn menu_create(_: *const c_char) -> *mut menu;
    pub fn menu_add_items(_: *mut menu, _: *const menu_item, _: *mut cmdq_item, _: *mut client, _: *mut cmd_find_state);
    pub fn menu_add_item(_: *mut menu, _: *const menu_item, _: *mut cmdq_item, _: *mut client, _: *mut cmd_find_state);
    pub fn menu_free(_: *mut menu);
    pub fn menu_prepare(
        _: *mut menu,
        _: c_int,
        _: c_int,
        _: *mut cmdq_item,
        _: c_uint,
        _: c_uint,
        _: *mut client,
        _: box_lines,
        _: *const c_char,
        _: *const c_char,
        _: *const c_char,
        _: *mut cmd_find_state,
        _: menu_choice_cb,
        _: *mut c_void,
    ) -> *mut menu_data;
    pub fn menu_display(
        _: *mut menu,
        _: c_int,
        _: c_int,
        _: *mut cmdq_item,
        _: c_uint,
        _: c_uint,
        _: *mut client,
        _: box_lines,
        _: *const c_char,
        _: *const c_char,
        _: *const c_char,
        _: *mut cmd_find_state,
        _: menu_choice_cb,
        _: *mut c_void,
    ) -> c_int;
    pub fn menu_mode_cb(_: *mut client, _: *mut c_void, _: *mut c_uint, _: *mut c_uint) -> *mut screen;
    pub fn menu_check_cb(_: *mut client, _: *mut c_void, _: c_uint, _: c_uint, _: c_uint, _: *mut overlay_ranges);
    pub fn menu_draw_cb(_: *mut client, _: *mut c_void, _: *mut screen_redraw_ctx);
    pub fn menu_free_cb(_: *mut client, _: *mut c_void);
    pub fn menu_key_cb(_: *mut client, _: *mut c_void, _: *mut key_event) -> c_int;
}
/* popup.c */
pub type popup_close_cb =
    ::std::option::Option<unsafe extern "C" fn(_: ::std::os::raw::c_int, _: *mut ::std::os::raw::c_void)>;
pub type popup_finish_edit_cb = ::std::option::Option<
    unsafe extern "C" fn(_: *mut ::std::os::raw::c_char, _: usize, _: *mut ::std::os::raw::c_void),
>;
unsafe extern "C" {
    pub fn popup_display(
        _: c_int,
        _: box_lines,
        _: *mut cmdq_item,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: *mut environ,
        _: *const c_char,
        _: c_int,
        _: *mut *mut c_char,
        _: *const c_char,
        _: *const c_char,
        _: *mut client,
        _: *mut session,
        _: *const c_char,
        _: *const c_char,
        _: popup_close_cb,
        _: *mut c_void,
    ) -> c_int;
    pub fn popup_editor(_: *mut client, _: *const c_char, _: usize, _: popup_finish_edit_cb, _: *mut c_void) -> c_int;
}
/* style.c */
unsafe extern "C" {
    pub fn style_parse(_: *mut style, _: *const grid_cell, _: *const c_char) -> c_int;
    pub fn style_tostring(_: *mut style) -> *const c_char;
    pub fn style_add(_: *mut grid_cell, _: *mut options, _: *const c_char, _: *mut format_tree);
    pub fn style_apply(_: *mut grid_cell, _: *mut options, _: *const c_char, _: *mut format_tree);
    pub fn style_set(_: *mut style, _: *const grid_cell);
    pub fn style_copy(_: *mut style, _: *mut style);
}
/* spawn.c */
unsafe extern "C" {
    pub fn spawn_window(_: *mut spawn_context, _: *mut *mut c_char) -> *mut winlink;
    pub fn spawn_pane(_: *mut spawn_context, _: *mut *mut c_char) -> *mut window_pane;
}
/* regsub.c */
unsafe extern "C" {
    pub fn regsub(_: *const c_char, _: *const c_char, _: *const c_char, _: c_int) -> *mut c_char;
}
/* image.c */
unsafe extern "C" {}
/* image-sixel.c */
unsafe extern "C" {}
/* server-acl.c */
opaque_types! {server_acl_user}
unsafe extern "C" {
    pub fn server_acl_init();
    pub fn server_acl_user_find(_: uid_t) -> *mut server_acl_user;
    pub fn server_acl_display(_: *mut cmdq_item);
    pub fn server_acl_user_allow(_: uid_t);
    pub fn server_acl_user_deny(_: uid_t);
    pub fn server_acl_user_allow_write(_: uid_t);
    pub fn server_acl_user_deny_write(_: uid_t);
    pub fn server_acl_join(_: *mut client) -> c_int;
    pub fn server_acl_get_uid(_: *mut server_acl_user) -> uid_t;
}
/* hyperlink.c */
unsafe extern "C" {
    pub fn hyperlinks_put(_: *mut hyperlinks, _: *const c_char, _: *const c_char) -> c_uint;
    pub fn hyperlinks_get(
        _: *mut hyperlinks,
        _: c_uint,
        _: *mut *const c_char,
        _: *mut *const c_char,
        _: *mut *const c_char,
    ) -> c_int;
    pub fn hyperlinks_init() -> *mut hyperlinks;
    pub fn hyperlinks_copy(_: *mut hyperlinks) -> *mut hyperlinks;
    pub fn hyperlinks_reset(_: *mut hyperlinks);
    pub fn hyperlinks_free(_: *mut hyperlinks);
}
