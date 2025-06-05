#![feature(c_variadic)]
#![warn(static_mut_refs)]
// #![warn(clippy::shadow_reuse)]
// #![warn(clippy::shadow_same)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(private_interfaces)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(unused)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::deref_addrof, reason = "many false positive, required for unsafe code")]
#![allow(clippy::manual_clamp)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::needless_return)]
#![allow(clippy::new_without_default)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::zero_ptr)]
#![warn(clippy::shadow_reuse)]
#![warn(clippy::shadow_same)]
#![warn(clippy::shadow_unrelated)]

pub mod compat;

pub mod ncurses_;
pub use ncurses_::*;

pub mod libc_;
pub use libc_::*;
use xmalloc::Zeroable; // want to rexport everything from here

#[cfg(feature = "sixel")]
pub mod image_;
#[cfg(feature = "sixel")]
pub mod image_sixel;
#[cfg(feature = "sixel")]
use image_sixel::sixel_image;

#[cfg(feature = "utempter")]
pub mod utempter;

pub use core::{
    ffi::{
        CStr, c_char, c_int, c_long, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort, c_void,
        va_list::{VaList, VaListImpl},
    },
    mem::{ManuallyDrop, MaybeUninit, size_of, zeroed},
    ops::ControlFlow,
    ptr::{NonNull, null, null_mut},
};

pub use libc::{FILE, REG_EXTENDED, REG_ICASE, SEEK_END, SEEK_SET, SIGHUP, WEXITSTATUS, WIFEXITED, WIFSIGNALED, WTERMSIG, fclose, fdopen, fopen, fread, free, fseeko, ftello, fwrite, malloc, memcmp, mkstemp, pid_t, strcpy, strerror, strlen, termios, time_t, timeval, uid_t, unlink};

// libevent2
mod event_;
pub use event_::*;

use crate::compat::{
    RB_GENERATE, impl_tailq_entry,
    queue::{Entry, ListEntry, list_entry, list_head, tailq_entry, tailq_first, tailq_foreach, tailq_head, tailq_next},
    tree::{GetEntry, rb_entry, rb_head},
};

unsafe extern "C" {
    pub static mut environ: *mut *mut c_char;
    fn strsep(_: *mut *mut c_char, _delim: *const c_char) -> *mut c_char;
    // fn strsep(_: *mut *mut c_char, _delim: *const c_char) -> *mut c_char;
}

#[inline]
pub fn transmute_ptr<T>(value: Option<NonNull<T>>) -> *mut T {
    // unsafe { core::mem::transmute::<Option<NonNull<T>>, *mut T>(value) }
    // unsafe { core::mem::transmute::<Option<NonNull<T>>, *mut T>(value) }
    match value {
        Some(ptr) => ptr.as_ptr(),
        None => null_mut(),
    }
}

pub use compat::imsg::imsg; // TODO move

// #define S_ISDIR(mode)  (((mode) & S_IFMT) == S_IFDIR)
// TODO move this to a better spot
#[inline]
pub fn S_ISDIR(mode: u32) -> bool { mode & libc::S_IFMT == libc::S_IFDIR }

pub type wchar_t = core::ffi::c_int;
unsafe extern "C" {
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    static mut stderr: *mut FILE;
}

// TODO move to compat
pub unsafe fn strchr_(cs: *const c_char, c: char) -> *mut c_char { unsafe { libc::strchr(cs, c as i32) } }

// use crate::tmux_protocol_h::*;

pub type bitstr_t = u8;

pub unsafe fn bit_alloc(nbits: u32) -> *mut u8 { unsafe { libc::calloc(((nbits + 7) / 8) as usize, 1).cast() } }
pub unsafe fn bit_set(bits: *mut u8, i: u32) {
    unsafe {
        let byte_index = i / 8;
        let bit_index = i % 8;
        *bits.add(byte_index as usize) |= 1 << bit_index;
    }
}

#[inline]
pub unsafe fn bit_clear(bits: *mut u8, i: u32) {
    unsafe {
        let byte_index = i / 8;
        let bit_index = i % 8;
        *bits.add(byte_index as usize) &= !(1 << bit_index);
    }
}

/// clear bits start..=stop in bitstring
pub unsafe fn bit_nclear(bits: *mut u8, start: u32, stop: u32) {
    unsafe {
        // TODO this is written inefficiently, assuming the compiler will optimize it. if it doesn't rewrite it
        for i in start..=stop {
            bit_clear(bits, i);
        }
    }
}

pub unsafe fn bit_test(bits: *const u8, i: u32) -> bool {
    unsafe {
        let byte_index = i / 8;
        let bit_index = i % 8;
        (*bits.add(byte_index as usize) & (1 << bit_index)) != 0
    }
}

const TTY_NAME_MAX: usize = 32;

// discriminant structs
pub struct discr_alerts_entry;
pub struct discr_all_entry;
pub struct discr_by_uri_entry;
pub struct discr_by_inner_entry;
pub struct discr_data_entry;
pub struct discr_entry;
pub struct discr_gentry;
pub struct discr_index_entry;
pub struct discr_name_entry;
pub struct discr_pending_entry;
pub struct discr_sentry;
pub struct discr_time_entry;
pub struct discr_tree_entry;
pub struct discr_wentry;

unsafe extern "C" {
    unsafe fn basename(_: *mut c_char) -> *mut c_char;
}

pub const _PATH_BSHELL: *const c_char = c"/bin/sh".as_ptr();
pub const _PATH_DEFPATH: *const c_char = c"/usr/bin:/bin".as_ptr();
pub const _PATH_DEV: *const c_char = c"/dev/".as_ptr();
pub const _PATH_DEVNULL: *const c_char = c"/dev/null".as_ptr();
pub const _PATH_VI: *const c_char = c"/usr/bin/vi".as_ptr();

pub const SIZEOF_PATH_DEV: usize = 6;

pub const TMUX_CONF: &CStr = c"/etc/tmux.conf:~/.tmux.conf";
pub const TMUX_SOCK: &CStr = c"$TMUX_TMPDIR:/tmp/";
pub const TMUX_TERM: &CStr = c"screen";
pub const TMUX_LOCK_CMD: &CStr = c"lock -np";

/// Minimum layout cell size, NOT including border lines.
pub const PANE_MINIMUM: u32 = 1;

/// Automatic name refresh interval, in microseconds. Must be < 1 second.
pub const NAME_INTERVAL: i32 = 500000;

/// Default pixel cell sizes.
pub const DEFAULT_XPIXEL: u32 = 16;
pub const DEFAULT_YPIXEL: u32 = 32;

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

/* C0 control characters */
#[repr(u64)]
#[derive(Copy, Clone)]
enum c0 {
    C0_NUL,
    C0_SOH,
    C0_STX,
    C0_ETX,
    C0_EOT,
    C0_ENQ,
    C0_ASC,
    C0_BEL,
    C0_BS,
    C0_HT,
    C0_LF,
    C0_VT,
    C0_FF,
    C0_CR,
    C0_SO,
    C0_SI,
    C0_DLE,
    C0_DC1,
    C0_DC2,
    C0_DC3,
    C0_DC4,
    C0_NAK,
    C0_SYN,
    C0_ETB,
    C0_CAN,
    C0_EM,
    C0_SUB,
    C0_ESC,
    C0_FS,
    C0_GS,
    C0_RS,
    C0_US,
}

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
#[derive(Copy, Clone)]
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
pub const MOUSE_PARAM_MAX: u32 = 0xff;
pub const MOUSE_PARAM_UTF8_MAX: u32 = 0x7ff;
pub const MOUSE_PARAM_BTN_OFF: u32 = 0x20;
pub const MOUSE_PARAM_POS_OFF: u32 = 0x21;

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

impl utf8_data {
    pub const fn new<const N: usize>(data: [u8; N], have: c_uchar, size: c_uchar, width: c_uchar) -> Self {
        if N >= UTF8_SIZE {
            panic!("invalid size");
        }

        let mut padded_data = [0u8; 21];
        let mut i = 0usize;
        while i < N {
            padded_data[i] = data[i];
            i += 1;
        }

        Self { data: padded_data, have, size, width }
    }
}

pub use utf8_state::*;
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
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
pub fn COLOUR_DEFAULT(c: i32) -> bool { c == 8 || c == 9 }

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
pub const GRID_ATTR_BRIGHT: u16 = 0x1;
pub const GRID_ATTR_DIM: u16 = 0x2;
pub const GRID_ATTR_UNDERSCORE: u16 = 0x4;
pub const GRID_ATTR_BLINK: u16 = 0x8;
pub const GRID_ATTR_REVERSE: u16 = 0x10;
pub const GRID_ATTR_HIDDEN: u16 = 0x20;
pub const GRID_ATTR_ITALICS: u16 = 0x40;
pub const GRID_ATTR_CHARSET: u16 = 0x80; // alternative character set
pub const GRID_ATTR_STRIKETHROUGH: u16 = 0x100;
pub const GRID_ATTR_UNDERSCORE_2: u16 = 0x200;
pub const GRID_ATTR_UNDERSCORE_3: u16 = 0x400;
pub const GRID_ATTR_UNDERSCORE_4: u16 = 0x800;
pub const GRID_ATTR_UNDERSCORE_5: u16 = 0x1000;
pub const GRID_ATTR_OVERLINE: u16 = 0x2000;

/// All underscore attributes.
pub const GRID_ATTR_ALL_UNDERSCORE: u16 = GRID_ATTR_UNDERSCORE | GRID_ATTR_UNDERSCORE_2 | GRID_ATTR_UNDERSCORE_3 | GRID_ATTR_UNDERSCORE_4 | GRID_ATTR_UNDERSCORE_5;

bitflags::bitflags! {
    /// Grid flags.
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct grid_flag : u8 {
        const FG256 = 0x1;
        const BG256 = 0x2;
        const PADDING = 0x4;
        const EXTENDED = 0x8;
        const SELECTED = 0x10;
        const NOPALETTE = 0x20;
        const CLEARED = 0x40;
    }
}

/// Grid line flags.
bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct grid_line_flag: i32 {
        const WRAPPED      = 1 << 0; // 0x1
        const EXTENDED     = 1 << 1; // 0x2
        const DEAD         = 1 << 2; // 0x4
        const START_PROMPT = 1 << 3; // 0x8
        const START_OUTPUT = 1 << 4; // 0x10
    }
}

// Grid string flags.
pub const GRID_STRING_WITH_SEQUENCES: i32 = 0x1;
pub const GRID_STRING_ESCAPE_SEQUENCES: i32 = 0x2;
pub const GRID_STRING_TRIM_SPACES: i32 = 0x4;
pub const GRID_STRING_USED_ONLY: i32 = 0x8;
pub const GRID_STRING_EMPTY_CELLS: i32 = 0x10;

/// Cell positions.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum cell_type {
    CELL_INSIDE = 0,
    CELL_TOPBOTTOM = 1,
    CELL_LEFTRIGHT = 2,
    CELL_TOPLEFT = 3,
    CELL_TOPRIGHT = 4,
    CELL_BOTTOMLEFT = 5,
    CELL_BOTTOMRIGHT = 6,
    CELL_TOPJOIN = 7,
    CELL_BOTTOMJOIN = 8,
    CELL_LEFTJOIN = 9,
    CELL_RIGHTJOIN = 10,
    CELL_JOIN = 11,
    CELL_OUTSIDE = 12,
}
use cell_type::*; // TODO remove

// Cell borders.
pub const CELL_BORDERS: [u8; 13] = [b' ', b'x', b'q', b'l', b'k', b'm', b'j', b'w', b'v', b't', b'u', b'n', b'~'];
pub const SIMPLE_BORDERS: [u8; 13] = [b' ', b'|', b'-', b'+', b'+', b'+', b'+', b'+', b'+', b'+', b'+', b'+', b'.'];
pub const PADDED_BORDERS: [u8; 13] = [b' '; 13];

/// Grid cell data.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct grid_cell {
    pub data: utf8_data,
    pub attr: c_ushort,
    pub flags: grid_flag,
    pub fg: i32,
    pub bg: i32,
    pub us: i32,
    pub link: u32,
}

impl grid_cell {
    pub const fn new(data: utf8_data, attr: c_ushort, flags: grid_flag, fg: i32, bg: i32, us: i32, link: u32) -> Self { Self { data, attr, flags, fg, bg, us, link } }
}

/// Grid extended cell entry.
#[repr(C)]
pub struct grid_extd_entry {
    pub data: utf8_char,
    pub attr: u16,
    pub flags: u8,
    pub fg: i32,
    pub bg: i32,
    pub us: i32,
    pub link: u32,
}

#[derive(Copy, Clone)]
#[repr(C, align(4))]
pub struct grid_cell_entry_data {
    pub attr: c_uchar,
    pub fg: c_uchar,
    pub bg: c_uchar,
    pub data: c_uchar,
}

#[repr(C)]
pub union grid_cell_entry_union {
    pub offset: u32,
    pub data: grid_cell_entry_data,
}

#[repr(C)]
pub struct grid_cell_entry {
    pub union_: grid_cell_entry_union,
    pub flags: grid_flag,
}

/// Grid line.
#[repr(C)]
pub struct grid_line {
    pub celldata: *mut grid_cell_entry,
    pub cellused: u32,
    pub cellsize: u32,

    pub extddata: *mut grid_extd_entry,
    pub extdsize: u32,

    pub flags: grid_line_flag,
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
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum style_align {
    STYLE_ALIGN_DEFAULT,
    STYLE_ALIGN_LEFT,
    STYLE_ALIGN_CENTRE,
    STYLE_ALIGN_RIGHT,
    STYLE_ALIGN_ABSOLUTE_CENTRE,
}

/// Style list.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum style_list {
    STYLE_LIST_OFF,
    STYLE_LIST_ON,
    STYLE_LIST_FOCUS,
    STYLE_LIST_LEFT_MARKER,
    STYLE_LIST_RIGHT_MARKER,
}

/// Style range.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum style_range_type {
    STYLE_RANGE_NONE,
    STYLE_RANGE_LEFT,
    STYLE_RANGE_RIGHT,
    STYLE_RANGE_PANE,
    STYLE_RANGE_WINDOW,
    STYLE_RANGE_SESSION,
    STYLE_RANGE_USER,
}

crate::compat::impl_tailq_entry!(style_range, entry, tailq_entry<style_range>);
// #[derive(crate::compat::TailQEntry)]
#[repr(C)]
pub struct style_range {
    pub type_: style_range_type,
    pub argument: u32,
    pub string: [c_char; 16],
    pub start: u32,
    /// not included
    pub end: u32,

    // #[entry]
    pub entry: tailq_entry<style_range>,
}
pub type style_ranges = tailq_head<style_range>;

/// Style default.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum style_default_type {
    STYLE_DEFAULT_BASE,
    STYLE_DEFAULT_PUSH,
    STYLE_DEFAULT_POP,
}

/// Style option.
#[repr(C)]
#[derive(Copy, Clone)]
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
crate::compat::impl_tailq_entry!(image, all_entry, tailq_entry<image>);
#[cfg(feature = "sixel")]
crate::compat::impl_tailq_entry!(image, entry, tailq_entry<image>);
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
#[repr(i32)]
#[derive(Copy, Clone)]
pub enum screen_cursor_style {
    SCREEN_CURSOR_DEFAULT,
    SCREEN_CURSOR_BLOCK,
    SCREEN_CURSOR_UNDERLINE,
    SCREEN_CURSOR_BAR,
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
    pub images: images,

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
#[derive(Copy, Clone, Default, Eq, PartialEq, num_enum::TryFromPrimitive)]
pub enum box_lines {
    #[default]
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
#[derive(Copy, Clone, Default, Eq, PartialEq, num_enum::TryFromPrimitive)]
pub enum pane_lines {
    #[default]
    PANE_LINES_SINGLE,
    PANE_LINES_DOUBLE,
    PANE_LINES_HEAVY,
    PANE_LINES_SIMPLE,
    PANE_LINES_NUMBER,
}

macro_rules! define_error_unit {
    ($error_type:ident) => {
        #[derive(Debug)]
        pub struct $error_type;
        impl ::std::error::Error for $error_type {}
        impl ::std::fmt::Display for $error_type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
        }
    };
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

    pub pane_status: pane_status,
    pub pane_lines: pane_lines,

    pub no_pane_gc: grid_cell,
    pub no_pane_gc_set: i32,

    pub sx: u32,
    pub sy: u32,
    pub ox: u32,
    pub oy: u32,
}

pub unsafe fn screen_size_x(s: *const screen) -> u32 { unsafe { (*(*s).grid).sx } }
pub unsafe fn screen_size_y(s: *const screen) -> u32 { unsafe { (*(*s).grid).sy } }
pub unsafe fn screen_hsize(s: *const screen) -> u32 { unsafe { (*(*s).grid).hsize } }
pub unsafe fn screen_hlimit(s: *const screen) -> u32 { unsafe { (*(*s).grid).hlimit } }

// Menu.
#[repr(C)]
pub struct menu_item {
    pub name: SyncCharPtr,
    pub key: key_code,
    pub command: SyncCharPtr,
}
impl menu_item {
    const fn new(name: Option<&'static CStr>, key: key_code, command: *const c_char) -> Self {
        Self {
            name: match name {
                Some(n) => SyncCharPtr::new(n),
                None => SyncCharPtr::null(),
            },
            key,
            command: SyncCharPtr(command),
        }
    }
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
    pub name: SyncCharPtr,
    pub default_format: SyncCharPtr,

    pub init: Option<unsafe extern "C" fn(NonNull<window_mode_entry>, *mut cmd_find_state, *mut args) -> *mut screen>,
    pub free: Option<unsafe extern "C" fn(NonNull<window_mode_entry>)>,
    pub resize: Option<unsafe extern "C" fn(NonNull<window_mode_entry>, u32, u32)>,
    pub update: Option<unsafe extern "C" fn(NonNull<window_mode_entry>)>,
    pub key: Option<unsafe extern "C" fn(NonNull<window_mode_entry>, *mut client, *mut session, *mut winlink, key_code, *mut mouse_event)>,

    pub key_table: Option<unsafe extern "C" fn(*mut window_mode_entry) -> *const c_char>,
    pub command: Option<unsafe extern "C" fn(NonNull<window_mode_entry>, *mut client, *mut session, *mut winlink, *mut args, *mut mouse_event)>,
    pub formats: Option<unsafe extern "C" fn(*mut window_mode_entry, *mut format_tree)>,
}

// Active window mode.
crate::compat::impl_tailq_entry!(window_mode_entry, entry, tailq_entry<window_mode_entry>);
// #[derive(Copy, Clone, crate::compat::TailQEntry)]
#[repr(C)]
pub struct window_mode_entry {
    pub wp: *mut window_pane,
    pub swp: *mut window_pane,

    pub mode: *mut window_mode,
    pub data: *mut c_void,

    pub screen: *mut screen,
    pub prefix: u32,

    // #[entry]
    pub entry: tailq_entry<window_mode_entry>,
}

/// Offsets into pane buffer.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct window_pane_offset {
    pub used: usize,
}

/// Queued pane resize.
crate::compat::impl_tailq_entry!(window_pane_resize, entry, tailq_entry<window_pane_resize>);
// #[derive(Copy, Clone, crate::compat::TailQEntry)]
#[repr(C)]
pub struct window_pane_resize {
    pub sx: u32,
    pub sy: u32,

    pub osx: u32,
    pub osy: u32,

    // #[entry]
    pub entry: tailq_entry<window_pane_resize>,
}
pub type window_pane_resizes = tailq_head<window_pane_resize>;

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct window_pane_flags : i32 {
        const PANE_REDRAW = 0x1;
        const PANE_DROP = 0x2;
        const PANE_FOCUSED = 0x4;
        const PANE_VISITED = 0x8;
        /* 0x10 unused */
        /* 0x20 unused */
        const PANE_INPUTOFF = 0x40;
        const PANE_CHANGED = 0x80;
        const PANE_EXITED = 0x100;
        const PANE_STATUSREADY = 0x200;
        const PANE_STATUSDRAWN = 0x400;
        const PANE_EMPTY = 0x800;
        const PANE_STYLECHANGED = 0x1000;
        const PANE_UNSEENCHANGES = 0x2000;
    }
}

/// Child window structure.
// #[derive(Copy, Clone)]
#[repr(C)]
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

    pub flags: window_pane_flags,

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
pub type window_panes = tailq_head<window_pane>;
pub type window_pane_tree = rb_head<window_pane>;

impl Entry<window_pane, discr_entry> for window_pane {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<window_pane> { unsafe { &raw mut (*this).entry } }
}
impl Entry<window_pane, discr_sentry> for window_pane {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<window_pane> { unsafe { &raw mut (*this).sentry } }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct window_flag: i32 {
        const BELL = 0x1;
        const ACTIVITY = 0x2;
        const SILENCE = 0x4;
        const ZOOMED = 0x8;
        const WASZOOMED = 0x10;
        const RESIZE = 0x20;
    }
}
pub const WINDOW_ALERTFLAGS: window_flag = window_flag::BELL.union(window_flag::ACTIVITY).union(window_flag::SILENCE);

/// Window structure.
#[repr(C)]
// #[derive(Copy, Clone)]
pub struct window {
    pub id: u32,
    pub latest: *mut c_void,

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
    pub flags: window_flag,

    pub alerts_queued: i32,
    pub alerts_entry: tailq_entry<window>,

    pub options: *mut options,

    pub references: u32,
    pub winlinks: tailq_head<winlink>,
    pub entry: rb_entry<window>,
}
pub type windows = rb_head<window>;
// crate::compat::impl_rb_tree_protos!(windows, window);

impl crate::compat::queue::Entry<window, discr_alerts_entry> for window {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<window> { unsafe { &raw mut (*this).alerts_entry } }
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

impl crate::compat::queue::Entry<winlink, discr_wentry> for winlink {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<winlink> { unsafe { &raw mut (*this).wentry } }
}

impl crate::compat::queue::Entry<winlink, discr_sentry> for winlink {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<winlink> { unsafe { &raw mut (*this).sentry } }
}

pub type winlinks = rb_head<winlink>;
// crate::compat::impl_rb_tree_protos!(winlinks, winlink);
pub type winlink_stack = tailq_head<winlink>;
// crate::compat::impl_rb_tree_protos!(winlink_stack, winlink);

// Window size option.
pub const WINDOW_SIZE_LARGEST: i32 = 0;
pub const WINDOW_SIZE_SMALLEST: i32 = 1;
pub const WINDOW_SIZE_MANUAL: i32 = 2;
pub const WINDOW_SIZE_LATEST: i32 = 3;

/// Pane border status option.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq, num_enum::TryFromPrimitive)]
pub enum pane_status {
    PANE_STATUS_OFF,
    PANE_STATUS_TOP,
    PANE_STATUS_BOTTOM,
}

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
crate::compat::impl_tailq_entry!(layout_cell, entry, tailq_entry<layout_cell>);
// #[derive(crate::compat::TailQEntry)]
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

    // #[entry]
    pub entry: tailq_entry<layout_cell>,
}

pub const ENVIRON_HIDDEN: i32 = 0x1;

/// Environment variable.
#[repr(C)]
pub struct environ_entry {
    pub name: Option<NonNull<c_char>>,
    pub value: Option<NonNull<c_char>>,

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
crate::compat::impl_tailq_entry!(session, gentry, tailq_entry<session>);

pub const MOUSE_MASK_BUTTONS: u32 = 195;
pub const MOUSE_MASK_SHIFT: u32 = 4;
pub const MOUSE_MASK_META: u32 = 8;
pub const MOUSE_MASK_CTRL: u32 = 16;
pub const MOUSE_MASK_DRAG: u32 = 32;
pub const MOUSE_MASK_MODIFIERS: u32 = MOUSE_MASK_SHIFT | MOUSE_MASK_META | MOUSE_MASK_CTRL;

/* Mouse wheel type. */
pub const MOUSE_WHEEL_UP: u32 = 64;
pub const MOUSE_WHEEL_DOWN: u32 = 65;

/* Mouse button type. */
pub const MOUSE_BUTTON_1: u32 = 0;
pub const MOUSE_BUTTON_2: u32 = 1;
pub const MOUSE_BUTTON_3: u32 = 2;
pub const MOUSE_BUTTON_6: u32 = 66;
pub const MOUSE_BUTTON_7: u32 = 67;
pub const MOUSE_BUTTON_8: u32 = 128;
pub const MOUSE_BUTTON_9: u32 = 129;
pub const MOUSE_BUTTON_10: u32 = 130;
pub const MOUSE_BUTTON_11: u32 = 131;

// Mouse helpers.
#[inline]
pub fn MOUSE_BUTTONS(b: u32) -> u32 { b & MOUSE_MASK_BUTTONS }
#[inline]
pub fn MOUSE_WHEEL(b: u32) -> bool { ((b) & MOUSE_MASK_BUTTONS) == MOUSE_WHEEL_UP || ((b) & MOUSE_MASK_BUTTONS) == MOUSE_WHEEL_DOWN }
#[inline]
pub fn MOUSE_DRAG(b: u32) -> bool { b & MOUSE_MASK_DRAG != 0 }
#[inline]
pub fn MOUSE_RELEASE(b: u32) -> bool { b & MOUSE_MASK_BUTTONS == 3 }

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

unsafe impl Zeroable for tty_term {}
/// Terminal definition.
#[repr(C)]
pub struct tty_term {
    pub name: *mut c_char,
    pub tty: *mut tty,
    pub features: i32,

    pub acs: [[c_char; 2]; c_uchar::MAX as usize + 1],

    pub codes: *mut tty_code,

    pub flags: i32,

    pub entry: list_entry<tty_term>,
}
pub type tty_terms = list_head<tty_term>;
impl ListEntry<tty_term, discr_entry> for tty_term {
    unsafe fn field(this: *mut Self) -> *mut list_entry<tty_term> { unsafe { &raw mut (*this).entry } }
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct tty_flags: i32 {
        const TTY_NOCURSOR = 0x1;
        const TTY_FREEZE = 0x2;
        const TTY_TIMER = 0x4;
        const TTY_NOBLOCK = 0x8;
        const TTY_STARTED = 0x10;
        const TTY_OPENED = 0x20;
        const TTY_OSC52QUERY = 0x40;
        const TTY_BLOCK = 0x80;
        const TTY_HAVEDA = 0x100; // Primary DA.
        const TTY_HAVEXDA = 0x200;
        const TTY_SYNCING = 0x400;
        const TTY_HAVEDA2 = 0x800; // Secondary DA.
    }
}
pub const TTY_ALL_REQUEST_FLAGS: tty_flags = tty_flags::TTY_HAVEDA.union(tty_flags::TTY_HAVEDA2).union(tty_flags::TTY_HAVEXDA);

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

    pub flags: tty_flags,

    pub term: *mut tty_term,

    pub mouse_last_x: u32,
    pub mouse_last_y: u32,
    pub mouse_last_b: u32,
    pub mouse_drag_flag: i32,
    pub mouse_drag_update: Option<unsafe extern "C" fn(*mut client, *mut mouse_event)>,
    pub mouse_drag_release: Option<unsafe extern "C" fn(*mut client, *mut mouse_event)>,

    pub key_timer: event,
    pub key_tree: *mut tty_key,
}

pub type tty_ctx_redraw_cb = Option<unsafe extern "C" fn(*const tty_ctx)>;
pub type tty_ctx_set_client_cb = Option<unsafe extern "C" fn(*mut tty_ctx, *mut client) -> i32>;

#[repr(C)]
pub struct tty_ctx {
    pub s: *mut screen,

    pub redraw_cb: tty_ctx_redraw_cb,
    pub set_client_cb: tty_ctx_set_client_cb,
    pub arg: *mut c_void,

    pub cell: *const grid_cell,
    pub wrapped: i32,

    pub num: u32,
    pub ptr: *mut c_void,
    pub ptr2: *mut c_void,

    pub allow_invisible_panes: i32,

    /*
     * Cursor and region position before the screen was updated - this is
     * where the command should be applied; the values in the screen have
     * already been updated.
     */
    pub ocx: u32,
    pub ocy: u32,

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
    pub palette: *const colour_palette,

    // Containing region (usually window) offset and size.
    pub bigger: i32,
    pub wox: u32,
    pub woy: u32,
    pub wsx: u32,
    pub wsy: u32,
}

// Saved message entry.
crate::compat::impl_tailq_entry!(message_entry, entry, tailq_entry<message_entry>);
// #[derive(Copy, Clone, crate::compat::TailQEntry)]
#[repr(C)]
pub struct message_entry {
    pub msg: *mut c_char,
    pub msg_num: u32,
    pub msg_time: timeval,

    // #[entry]
    pub entry: tailq_entry<message_entry>,
}
pub type message_list = tailq_head<message_entry>;

/// Argument type.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
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

unsafe impl Zeroable for args_value {}
/// Argument value.
crate::compat::impl_tailq_entry!(args_value, entry, tailq_entry<args_value>);
// #[derive(crate::compat::TailQEntry)]
#[repr(C)]
pub struct args_value {
    pub type_: args_type,
    pub union_: args_value_union,
    pub cached: *mut c_char,
    // #[entry]
    pub entry: tailq_entry<args_value>,
}
pub type args_tree = rb_head<args_entry>;

/// Arguments parsing type.
#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq)]
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

impl args_parse {
    pub const fn new(template: &CStr, lower: i32, upper: i32, cb: args_parse_cb) -> Self { Self { template: template.as_ptr(), lower, upper, cb } }
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
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum cmd_retval {
    CMD_RETURN_ERROR = -1,
    CMD_RETURN_NORMAL = 0,
    CMD_RETURN_WAIT,
    CMD_RETURN_STOP,
}

// Command parse result.
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
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

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct cmd_parse_input_flags: i32 {
        const CMD_PARSE_QUIET = 0x1;
        const CMD_PARSE_PARSEONLY = 0x2;
        const CMD_PARSE_NOALIAS = 0x4;
        const CMD_PARSE_VERBOSE = 0x8;
        const CMD_PARSE_ONEGROUP = 0x10;
    }
}

#[repr(C)]
pub struct cmd_parse_input {
    pub flags: cmd_parse_input_flags,

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

impl cmd_entry_flag {
    pub const fn new(flag: u8, type_: cmd_find_type, flags: i32) -> Self { Self { flag: flag as c_char, type_, flags } }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct cmd_flag: i32 {
        const CMD_STARTSERVER = 0x1;
        const CMD_READONLY = 0x2;
        const CMD_AFTERHOOK = 0x4;
        const CMD_CLIENT_CFLAG = 0x8;
        const CMD_CLIENT_TFLAG = 0x10;
        const CMD_CLIENT_CANFAIL = 0x20;
    }
}

// Command definition.
#[repr(C)]
pub struct cmd_entry {
    pub name: *const c_char,
    pub alias: *const c_char,

    pub args: args_parse,
    pub usage: *const c_char,

    pub source: cmd_entry_flag,
    pub target: cmd_entry_flag,

    pub flags: cmd_flag,

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
pub const PROMPT_NTYPES: u32 = 4;
#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq, num_enum::TryFromPrimitive)]
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
RB_GENERATE!(client_files, client_file, entry, file_cmp);

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

pub type prompt_input_cb = Option<unsafe extern "C" fn(*mut client, NonNull<c_void>, *const c_char, i32) -> i32>;
pub type prompt_free_cb = Option<unsafe extern "C" fn(NonNull<c_void>)>;
pub type overlay_check_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void, u32, u32, u32, *mut overlay_ranges)>;
pub type overlay_mode_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void, *mut u32, *mut u32) -> *mut screen>;
pub type overlay_draw_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void, *mut screen_redraw_ctx)>;
pub type overlay_key_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void, *mut key_event) -> i32>;
pub type overlay_free_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void)>;
pub type overlay_resize_cb = Option<unsafe extern "C" fn(*mut client, *mut c_void)>;

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    struct client_flag: u64 {
        const TERMINAL           = 0x0000000001u64;
        const LOGIN              = 0x0000000002u64;
        const EXIT               = 0x0000000004u64;
        const REDRAWWINDOW       = 0x0000000008u64;
        const REDRAWSTATUS       = 0x0000000010u64;
        const REPEAT             = 0x0000000020u64;
        const SUSPENDED          = 0x0000000040u64;
        const ATTACHED           = 0x0000000080u64;
        const EXITED             = 0x0000000100u64;
        const DEAD               = 0x0000000200u64;
        const REDRAWBORDERS      = 0x0000000400u64;
        const READONLY           = 0x0000000800u64;
        const NOSTARTSERVER      = 0x0000001000u64;
        const CONTROL            = 0x0000002000u64;
        const CONTROLCONTROL     = 0x0000004000u64;
        const FOCUSED            = 0x0000008000u64;
        const UTF8               = 0x0000010000u64;
        const IGNORESIZE         = 0x0000020000u64;
        const IDENTIFIED         = 0x0000040000u64;
        const STATUSFORCE        = 0x0000080000u64;
        const DOUBLECLICK        = 0x0000100000u64;
        const TRIPLECLICK        = 0x0000200000u64;
        const SIZECHANGED        = 0x0000400000u64;
        const STATUSOFF          = 0x0000800000u64;
        const REDRAWSTATUSALWAYS = 0x0001000000u64;
        const REDRAWOVERLAY      = 0x0002000000u64;
        const CONTROL_NOOUTPUT   = 0x0004000000u64;
        const DEFAULTSOCKET      = 0x0008000000u64;
        const STARTSERVER        = 0x0010000000u64;
        const REDRAWPANES        = 0x0020000000u64;
        const NOFORK             = 0x0040000000u64;
        const ACTIVEPANE         = 0x0080000000u64;
        const CONTROL_PAUSEAFTER = 0x0100000000u64;
        const CONTROL_WAITEXIT   = 0x0200000000u64;
        const WINDOWSIZECHANGED  = 0x0400000000u64;
        const CLIPBOARDBUFFER    = 0x0800000000u64;
        const BRACKETPASTING     = 0x1000000000u64;
    }
}

pub const CLIENT_ALLREDRAWFLAGS: client_flag = client_flag::REDRAWWINDOW
    .union(client_flag::REDRAWSTATUS)
    .union(client_flag::REDRAWSTATUSALWAYS)
    .union(client_flag::REDRAWBORDERS)
    .union(client_flag::REDRAWOVERLAY)
    .union(client_flag::REDRAWPANES);
pub const CLIENT_UNATTACHEDFLAGS: client_flag = client_flag::DEAD.union(client_flag::SUSPENDED).union(client_flag::EXIT);
pub const CLIENT_NODETACHFLAGS: client_flag = client_flag::DEAD.union(client_flag::EXIT);
pub const CLIENT_NOSIZEFLAGS: client_flag = client_flag::DEAD.union(client_flag::SUSPENDED).union(client_flag::EXIT);

pub const PROMPT_SINGLE: i32 = 0x1;
pub const PROMPT_NUMERIC: i32 = 0x2;
pub const PROMPT_INCREMENTAL: i32 = 0x4;
pub const PROMPT_NOFORMAT: i32 = 0x8;
pub const PROMPT_KEY: i32 = 0x8;

//#[derive(Copy, Clone)]
crate::compat::impl_tailq_entry!(client, entry, tailq_entry<client>);
// #[derive(crate::compat::TailQEntry)]
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

    pub flags: client_flag,

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
    pub prompt_data: *mut c_void,
    pub prompt_hindex: [c_uint; 4],
    pub prompt_mode: prompt_mode,
    pub prompt_saved: *mut utf8_data,

    pub prompt_flags: c_int,
    pub prompt_type: prompt_type,
    pub prompt_cursor: c_int,

    pub session: *mut session,
    pub last_session: *mut session,

    pub references: c_int,

    pub pan_window: *mut c_void,
    pub pan_ox: c_uint,
    pub pan_oy: c_uint,

    pub overlay_check: overlay_check_cb,
    pub overlay_mode: overlay_mode_cb,
    pub overlay_draw: overlay_draw_cb,
    pub overlay_key: overlay_key_cb,
    pub overlay_free: overlay_free_cb,
    pub overlay_resize: overlay_resize_cb,
    pub overlay_data: *mut c_void,
    pub overlay_timer: event,

    pub files: client_files,

    pub clipboard_panes: *mut c_uint,
    pub clipboard_npanes: c_uint,

    // #[entry]
    pub entry: tailq_entry<client>,
}
pub type clients = tailq_head<client>;

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
    pub cmdlist: *mut cmd_list,
    pub note: *mut c_char,

    pub flags: i32,

    pub entry: rb_entry<key_binding>,
}
pub type key_bindings = rb_head<key_binding>;

#[repr(C)]
pub struct key_table {
    pub name: *mut c_char,
    pub activity_time: timeval,
    pub key_bindings: key_bindings,
    pub default_key_bindings: key_bindings,

    pub references: u32,

    pub entry: rb_entry<key_table>,
}
pub type key_tables = rb_head<key_table>;

// Option data.
pub type options_array = rb_head<options_array_item>;

#[repr(C)]
#[derive(Copy, Clone)]
pub union options_value {
    pub string: *mut c_char,
    pub number: c_longlong,
    pub style: style,
    pub array: options_array,
    pub cmdlist: *mut cmd_list,
}

// Option table entries.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
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
    pub name: *const c_char,
    pub alternative_name: *mut c_char,
    pub type_: options_table_type,
    pub scope: i32,
    pub flags: i32,
    pub minimum: u32,
    pub maximum: u32,

    pub choices: *const *const c_char,

    pub default_str: *const c_char,
    pub default_num: c_longlong,
    pub default_arr: *const *const c_char,

    pub separator: *const c_char,
    pub pattern: *const c_char,

    pub text: *const c_char,
    pub unit: *const c_char,
}

#[repr(C)]
pub struct options_name_map {
    pub from: *const c_char,
    pub to: *const c_char,
}
impl options_name_map {
    const fn new(from: *const c_char, to: *const c_char) -> Self { Self { from, to } }
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

    pub name: *const c_char,
    pub argv: *mut *mut c_char,
    pub argc: i32,
    pub environ: *mut environ,

    pub idx: i32,
    pub cwd: *const c_char,

    pub flags: i32,
}

/// Mode tree sort order.
#[repr(C)]
pub struct mode_tree_sort_criteria {
    pub field: u32,
    pub reversed: i32,
}

pub const WINDOW_MINIMUM: u32 = PANE_MINIMUM;
pub const WINDOW_MAXIMUM: u32 = 10_000;

#[repr(i32)]
pub enum exit_type {
    CLIENT_EXIT_RETURN,
    CLIENT_EXIT_SHUTDOWN,
    CLIENT_EXIT_DETACH,
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum prompt_mode {
    PROMPT_ENTRY,
    PROMPT_COMMAND,
}

mod tmux;

#[cfg(not(test))]
pub use crate::tmux::main;

pub use crate::tmux::{checkshell, find_cwd, find_home, get_timer, getversion, global_environ, global_options, global_s_options, global_w_options, ptm_fd, setblocking, shell_argv0, shell_command, sig2name, socket_path, start_time};

mod proc;
pub use crate::proc::{proc_add_peer, proc_clear_signals, proc_exit, proc_flush_peer, proc_fork_and_daemon, proc_get_peer_uid, proc_kill_peer, proc_loop, proc_remove_peer, proc_send, proc_set_signals, proc_start, proc_toggle_log, tmuxpeer, tmuxproc};

mod cfg_;
pub use crate::cfg_::{cfg_add_cause, cfg_client, cfg_files, cfg_finished, cfg_nfiles, cfg_print_causes, cfg_quiet, cfg_show_causes, load_cfg, load_cfg_from_buffer, start_cfg};

mod paste;
pub use crate::paste::{paste_add, paste_buffer, paste_buffer_created, paste_buffer_data, paste_buffer_data_, paste_buffer_name, paste_buffer_order, paste_free, paste_get_name, paste_get_top, paste_is_empty, paste_make_sample, paste_rename, paste_replace, paste_set, paste_walk};

mod format;
pub use crate::format::{
    FORMAT_NONE, FORMAT_PANE, FORMAT_WINDOW, format_add, format_add_cb, format_add_tv, format_cb, format_create, format_create_defaults, format_create_from_state, format_create_from_target, format_defaults, format_defaults_pane, format_defaults_paste_buffer, format_defaults_window, format_each,
    format_expand, format_expand_time, format_flags, format_free, format_get_pane, format_grid_hyperlink, format_grid_line, format_grid_word, format_job_tree, format_log_debug, format_lost_client, format_merge, format_pretty_time, format_single, format_single_from_state, format_single_from_target,
    format_skip, format_tidy_jobs, format_tree, format_true,
};

mod format_draw_;
pub use crate::format_draw_::{format_draw, format_trim_left, format_trim_right, format_width};

mod notify;
pub use crate::notify::{notify_client, notify_hook, notify_pane, notify_paste_buffer, notify_session, notify_session_window, notify_window, notify_winlink};

mod options_;
pub use crate::options_::{
    options, options_array_assign, options_array_clear, options_array_first, options_array_get, options_array_item, options_array_item_index, options_array_item_value, options_array_next, options_array_set, options_create, options_default, options_default_to_string, options_empty, options_entry,
    options_first, options_free, options_from_string, options_get, options_get_number, options_get_only, options_get_parent, options_get_string, options_is_array, options_is_string, options_match, options_match_get, options_name, options_next, options_owner, options_parse, options_parse_get,
    options_push_changes, options_remove_or_default, options_scope_from_flags, options_scope_from_name, options_set_number, options_set_parent, options_set_string, options_string_to_style, options_table_entry, options_to_string,
};

mod options_table;
pub use crate::options_table::{options_other_names, options_table};

mod job_;
pub const JOB_NOWAIT: i32 = 1;
pub const JOB_KEEPWRITE: i32 = 2;
pub const JOB_PTY: i32 = 4;
pub const JOB_DEFAULTSHELL: i32 = 8;
pub use crate::job_::{job, job_check_died, job_complete_cb, job_free, job_free_cb, job_get_data, job_get_event, job_get_status, job_kill_all, job_print_summary, job_resize, job_run, job_still_running, job_transfer, job_update_cb};

mod environ_;
pub use crate::environ_::{environ, environ_clear, environ_copy, environ_create, environ_find, environ_first, environ_for_session, environ_free, environ_log, environ_next, environ_push, environ_put, environ_set, environ_unset, environ_update};

mod tty_;
pub use crate::tty_::{
    tty_attributes, tty_cell, tty_clipboard_query, tty_close, tty_cmd_alignmenttest, tty_cmd_cell, tty_cmd_cells, tty_cmd_clearcharacter, tty_cmd_clearendofline, tty_cmd_clearendofscreen, tty_cmd_clearline, tty_cmd_clearscreen, tty_cmd_clearstartofline, tty_cmd_clearstartofscreen,
    tty_cmd_deletecharacter, tty_cmd_deleteline, tty_cmd_insertcharacter, tty_cmd_insertline, tty_cmd_linefeed, tty_cmd_rawstring, tty_cmd_reverseindex, tty_cmd_scrolldown, tty_cmd_scrollup, tty_cmd_setselection, tty_cmd_syncstart, tty_create_log, tty_cursor, tty_default_colours, tty_draw_line,
    tty_free, tty_init, tty_m_in_off, tty_open, tty_putc, tty_putcode, tty_putcode_i, tty_putcode_ii, tty_putcode_iii, tty_putcode_s, tty_putcode_ss, tty_putn, tty_puts, tty_raw, tty_region_off, tty_repeat_requests, tty_reset, tty_resize, tty_send_requests, tty_set_path, tty_set_selection,
    tty_set_size, tty_set_title, tty_start_tty, tty_stop_tty, tty_sync_end, tty_sync_start, tty_update_client_offset, tty_update_features, tty_update_mode, tty_update_window_offset, tty_window_bigger, tty_window_offset, tty_write,
};

mod tty_term_;
pub use crate::tty_term_::{
    tty_code, tty_term_apply, tty_term_apply_overrides, tty_term_create, tty_term_describe, tty_term_flag, tty_term_free, tty_term_free_list, tty_term_has, tty_term_ncodes, tty_term_number, tty_term_read_list, tty_term_string, tty_term_string_i, tty_term_string_ii, tty_term_string_iii,
    tty_term_string_s, tty_term_string_ss, tty_terms,
};

mod tty_features;
pub use crate::tty_features::{tty_add_features, tty_apply_features, tty_default_features, tty_get_features};

mod tty_acs;
pub use crate::tty_acs::{tty_acs_double_borders, tty_acs_get, tty_acs_heavy_borders, tty_acs_needed, tty_acs_reverse_get, tty_acs_rounded_borders};

mod tty_keys;
pub use crate::tty_keys::{tty_key, tty_keys_build, tty_keys_colours, tty_keys_free, tty_keys_next};

mod arguments;

// TODO convert calls to args_has to args_has_
pub unsafe fn args_has_(args: *mut args, flag: char) -> bool {
    debug_assert!(flag.is_ascii());
    unsafe { args_has(args, flag as u8) != 0 }
}

// pub unsafe fn args_get(_: *mut args, _: c_uchar) -> *const c_char;
pub unsafe fn args_get_(args: *mut args, flag: char) -> *const c_char {
    debug_assert!(flag.is_ascii());
    unsafe { args_get(args, flag as u8) }
}

pub use crate::arguments::{
    args, args_command_state, args_copy, args_count, args_create, args_entry, args_escape, args_first, args_first_value, args_free, args_free_value, args_free_values, args_from_vector, args_get, args_has, args_make_commands, args_make_commands_free, args_make_commands_get_command,
    args_make_commands_now, args_make_commands_prepare, args_next, args_next_value, args_parse, args_percentage, args_percentage_and_expand, args_print, args_set, args_string, args_string_percentage, args_string_percentage_and_expand, args_strtonum, args_strtonum_and_expand, args_to_vector,
    args_value, args_values,
};

mod cmd_;
pub use crate::cmd_::{
    cmd, cmd_append_argv, cmd_copy, cmd_copy_argv, cmd_free, cmd_free_argv, cmd_get_alias, cmd_get_args, cmd_get_entry, cmd_get_group, cmd_get_source, cmd_list_all_have, cmd_list_any_have, cmd_list_append, cmd_list_append_all, cmd_list_copy, cmd_list_first, cmd_list_free, cmd_list_move,
    cmd_list_new, cmd_list_next, cmd_list_print, cmd_log_argv, cmd_mouse_at, cmd_mouse_pane, cmd_mouse_window, cmd_pack_argv, cmd_parse, cmd_prepend_argv, cmd_print, cmd_stringify_argv, cmd_table, cmd_template_replace, cmd_unpack_argv, cmds,
};

pub use crate::cmd_::cmd_attach_session::cmd_attach_session;

pub use crate::cmd_::cmd_find::{
    cmd_find_best_client, cmd_find_clear_state, cmd_find_client, cmd_find_copy_state, cmd_find_empty_state, cmd_find_from_client, cmd_find_from_mouse, cmd_find_from_nothing, cmd_find_from_pane, cmd_find_from_session, cmd_find_from_session_window, cmd_find_from_window, cmd_find_from_winlink,
    cmd_find_from_winlink_pane, cmd_find_target, cmd_find_valid_state,
};

pub mod cmd_parse;
pub use crate::cmd_parse::{cmd_parse_and_append, cmd_parse_and_insert, cmd_parse_command, cmd_parse_from_arguments, cmd_parse_from_buffer, cmd_parse_from_file, cmd_parse_from_string, cmd_parse_state, *};

pub use crate::cmd_::cmd_queue::{
    cmdq_add_format, cmdq_add_formats, cmdq_append, cmdq_continue, cmdq_copy_state, cmdq_error, cmdq_free, cmdq_free_state, cmdq_get_callback1, cmdq_get_client, cmdq_get_command, cmdq_get_current, cmdq_get_error, cmdq_get_event, cmdq_get_flags, cmdq_get_name, cmdq_get_source, cmdq_get_state,
    cmdq_get_target, cmdq_get_target_client, cmdq_guard, cmdq_insert_after, cmdq_insert_hook, cmdq_item, cmdq_link_state, cmdq_list, cmdq_merge_formats, cmdq_new, cmdq_new_state, cmdq_next, cmdq_print, cmdq_print_data, cmdq_running, cmdq_state,
};

pub use crate::cmd_::cmd_wait_for::cmd_wait_for_flush;

mod client_;
pub use crate::client_::client_main;

mod key_bindings_;
pub use crate::key_bindings_::{
    key_bindings_add, key_bindings_dispatch, key_bindings_first, key_bindings_first_table, key_bindings_get, key_bindings_get_default, key_bindings_get_table, key_bindings_init, key_bindings_next, key_bindings_next_table, key_bindings_remove, key_bindings_remove_table, key_bindings_reset,
    key_bindings_reset_table, key_bindings_unref_table,
};

mod key_string;
pub use crate::key_string::{key_string_lookup_key, key_string_lookup_string};

mod alerts;
pub use crate::alerts::{alerts_check_session, alerts_queue, alerts_reset_all};
/*
unsafe extern "C" {
    pub unsafe fn alerts_queue(w: *mut window, flags: c_int);
}
*/

mod file;
pub use crate::file::{
    client_files_RB_FIND, client_files_RB_INSERT, client_files_RB_INSERT_COLOR, client_files_RB_NFIND, client_files_RB_REMOVE, client_files_RB_REMOVE_COLOR, file_can_print, file_cancel, file_cmp, file_create_with_client, file_create_with_peer, file_error, file_fire_done, file_fire_read, file_free,
    file_print, file_print_buffer, file_push, file_read, file_read_cancel, file_read_data, file_read_done, file_read_open, file_vprint, file_write, file_write_close, file_write_data, file_write_left, file_write_open, file_write_ready,
};

mod server;
pub use crate::server::{clients, current_time, marked_pane, message_log, server_add_accept, server_add_message, server_check_marked, server_clear_marked, server_create_socket, server_is_marked, server_proc, server_set_marked, server_start, server_update_socket};

/*
unsafe extern "C" {
    pub unsafe static mut clients: clients;
    pub unsafe static mut marked_pane: cmd_find_state;
    pub unsafe static mut server_proc: *mut tmuxproc;


    #[unsafe(no_mangle)]
    pub fn server_start(
        client: *mut tmuxproc,
        flags: u64,
        base: *mut event_base,
        lockfd: c_int,
        lockfile: *mut c_char,
    ) -> c_int;

    #[unsafe(no_mangle)]
    pub unsafe fn server_add_message(fmt: *const c_char, ...);
    pub unsafe fn server_check_marked() -> c_int;
    pub unsafe fn server_clear_marked();
}
*/

mod server_client;
pub use crate::server_client::{
    client_windows_RB_FIND, client_windows_RB_INSERT, client_windows_RB_INSERT_COLOR, client_windows_RB_NFIND, client_windows_RB_REMOVE, client_windows_RB_REMOVE_COLOR, server_client_add_client_window, server_client_check_nested, server_client_clear_overlay, server_client_create,
    server_client_detach, server_client_exec, server_client_get_client_window, server_client_get_cwd, server_client_get_flags, server_client_get_key_table, server_client_get_pane, server_client_handle_key, server_client_how_many, server_client_loop, server_client_lost, server_client_open,
    server_client_overlay_range, server_client_print, server_client_remove_pane, server_client_set_flags, server_client_set_key_table, server_client_set_overlay, server_client_set_pane, server_client_set_session, server_client_suspend, server_client_unref,
};

mod server_fn;
pub use crate::server_fn::{
    server_check_unattached, server_destroy_pane, server_destroy_session, server_kill_pane, server_kill_window, server_link_window, server_lock, server_lock_client, server_lock_session, server_redraw_client, server_redraw_session, server_redraw_session_group, server_redraw_window,
    server_redraw_window_borders, server_renumber_all, server_renumber_session, server_status_client, server_status_session, server_status_session_group, server_status_window, server_unlink_window, server_unzoom_window,
};

mod status;
pub use crate::status::{
    status_at_line, status_free, status_get_range, status_init, status_line_size, status_message_clear, status_message_redraw, status_message_set, status_prompt_clear, status_prompt_hlist, status_prompt_hsize, status_prompt_key, status_prompt_load_history, status_prompt_redraw,
    status_prompt_save_history, status_prompt_set, status_prompt_type, status_prompt_type_string, status_prompt_update, status_redraw, status_timer_start, status_timer_start_all, status_update_cache,
};

mod resize;
pub use crate::resize::{default_window_size, recalculate_size, recalculate_sizes, recalculate_sizes_now, resize_window};

mod input;
pub use crate::input::{input_ctx, input_free, input_init, input_parse_buffer, input_parse_pane, input_parse_screen, input_pending, input_reply_clipboard, input_reset};

mod input_keys;
pub use crate::input_keys::{input_key, input_key_build, input_key_get_mouse, input_key_pane};

mod colour;
pub use crate::colour::{
    colour_256to16, colour_256toRGB, colour_byname, colour_find_rgb, colour_force_rgb, colour_fromstring, colour_join_rgb, colour_palette_clear, colour_palette_free, colour_palette_from_option, colour_palette_get, colour_palette_init, colour_palette_set, colour_parseX11, colour_split_rgb,
    colour_tostring,
};

mod attributes;
pub use crate::attributes::{attributes_fromstring, attributes_tostring};

mod grid_;
pub use crate::grid_::{
    grid_adjust_lines, grid_cells_equal, grid_cells_look_equal, grid_clear, grid_clear_history, grid_clear_lines, grid_collect_history, grid_compare, grid_create, grid_default_cell, grid_destroy, grid_duplicate_lines, grid_empty_line, grid_get_cell, grid_get_line, grid_line_length, grid_move_cells,
    grid_move_lines, grid_peek_line, grid_reflow, grid_remove_history, grid_scroll_history, grid_scroll_history_region, grid_set_cell, grid_set_cells, grid_set_padding, grid_string_cells, grid_unwrap_position, grid_wrap_position,
};

mod grid_reader_;
pub use crate::grid_reader_::{
    grid_reader_cursor_back_to_indentation, grid_reader_cursor_down, grid_reader_cursor_end_of_line, grid_reader_cursor_jump, grid_reader_cursor_jump_back, grid_reader_cursor_left, grid_reader_cursor_next_word, grid_reader_cursor_next_word_end, grid_reader_cursor_previous_word,
    grid_reader_cursor_right, grid_reader_cursor_start_of_line, grid_reader_cursor_up, grid_reader_get_cursor, grid_reader_in_set, grid_reader_line_length, grid_reader_start,
};

mod grid_view;
pub use crate::grid_view::{
    grid_view_clear, grid_view_clear_history, grid_view_delete_cells, grid_view_delete_lines, grid_view_delete_lines_region, grid_view_get_cell, grid_view_insert_cells, grid_view_insert_lines, grid_view_insert_lines_region, grid_view_scroll_region_down, grid_view_scroll_region_up,
    grid_view_set_cell, grid_view_set_cells, grid_view_set_padding, grid_view_string_cells,
};

mod screen_write;
pub use crate::screen_write::{
    screen_write_alignmenttest, screen_write_alternateoff, screen_write_alternateon, screen_write_backspace, screen_write_box, screen_write_carriagereturn, screen_write_cell, screen_write_citem, screen_write_clearcharacter, screen_write_clearendofline, screen_write_clearendofscreen,
    screen_write_clearhistory, screen_write_clearline, screen_write_clearscreen, screen_write_clearstartofline, screen_write_clearstartofscreen, screen_write_cline, screen_write_collect_add, screen_write_collect_end, screen_write_cursordown, screen_write_cursorleft, screen_write_cursormove,
    screen_write_cursorright, screen_write_cursorup, screen_write_deletecharacter, screen_write_deleteline, screen_write_fast_copy, screen_write_free_list, screen_write_fullredraw, screen_write_hline, screen_write_insertcharacter, screen_write_insertline, screen_write_linefeed,
    screen_write_make_list, screen_write_menu, screen_write_mode_clear, screen_write_mode_set, screen_write_nputs, screen_write_preview, screen_write_putc, screen_write_puts, screen_write_rawstring, screen_write_reset, screen_write_reverseindex, screen_write_scrolldown, screen_write_scrollregion,
    screen_write_scrollup, screen_write_setselection, screen_write_start, screen_write_start_callback, screen_write_start_pane, screen_write_stop, screen_write_strlen, screen_write_text, screen_write_vline, screen_write_vnputs,
};

mod screen_redraw;
pub use crate::screen_redraw::{screen_redraw_pane, screen_redraw_screen};

mod screen_;
pub use crate::screen_::{
    screen_alternate_off, screen_alternate_on, screen_check_selection, screen_clear_selection, screen_free, screen_hide_selection, screen_init, screen_mode_to_string, screen_pop_title, screen_push_title, screen_reinit, screen_reset_hyperlinks, screen_reset_tabs, screen_resize, screen_resize_cursor,
    screen_sel, screen_select_cell, screen_set_cursor_colour, screen_set_cursor_style, screen_set_path, screen_set_selection, screen_set_title, screen_titles,
};

mod window_;
pub use crate::window_::{
    all_window_panes, window_add_pane, window_add_ref, window_cmp, window_count_panes, window_create, window_destroy_panes, window_find_by_id, window_find_by_id_str, window_find_string, window_get_active_at, window_has_pane, window_lost_pane, window_pane_at_index, window_pane_cmp,
    window_pane_default_cursor, window_pane_destroy_ready, window_pane_exited, window_pane_find_by_id, window_pane_find_by_id_str, window_pane_find_down, window_pane_find_left, window_pane_find_right, window_pane_find_up, window_pane_get_new_data, window_pane_index, window_pane_key,
    window_pane_mode, window_pane_next_by_number, window_pane_previous_by_number, window_pane_reset_mode, window_pane_reset_mode_all, window_pane_resize, window_pane_search, window_pane_send_resize, window_pane_set_event, window_pane_set_mode, window_pane_stack_push, window_pane_stack_remove,
    window_pane_start_input, window_pane_update_focus, window_pane_update_used_data, window_pane_visible, window_pop_zoom, window_printable_flags, window_push_zoom, window_redraw_active_switch, window_remove_pane, window_remove_ref, window_resize, window_set_active_pane, window_set_fill_character,
    window_set_name, window_unzoom, window_update_activity, window_update_focus, window_zoom, windows, winlink_add, winlink_clear_flags, winlink_cmp, winlink_count, winlink_find_by_index, winlink_find_by_window, winlink_find_by_window_id, winlink_next, winlink_next_by_number, winlink_previous,
    winlink_previous_by_number, winlink_remove, winlink_set_window, winlink_shuffle_up, winlink_stack_push, winlink_stack_remove,
};
/*
unsafe extern "C" {
    pub unsafe static mut windows: windows;
    pub unsafe fn window_add_ref(w: *mut window, from: *const c_char);
    pub unsafe fn window_remove_ref(w: *mut window, from: *const c_char);
}
*/

unsafe extern "C" {
    // TODO remove these, generated by macro
    pub fn windows_RB_INSERT_COLOR(_: *mut windows, _: *mut window);
    pub fn windows_RB_REMOVE_COLOR(_: *mut windows, _: *mut window, _: *mut window);
    pub fn windows_RB_REMOVE(_: *mut windows, _: *mut window) -> *mut window;
    pub fn windows_RB_INSERT(_: *mut windows, _: *mut window) -> *mut window;
    pub fn windows_RB_FIND(_: *mut windows, _: *mut window) -> *mut window;
    pub fn windows_RB_NFIND(_: *mut windows, _: *mut window) -> *mut window;
    pub fn winlinks_RB_INSERT_COLOR(_: *mut winlinks, _: *mut winlink);
    pub fn winlinks_RB_REMOVE_COLOR(_: *mut winlinks, _: *mut winlink, _: *mut winlink);
    pub fn winlinks_RB_REMOVE(_: *mut winlinks, _: *mut winlink) -> *mut winlink;
    pub fn winlinks_RB_INSERT(_: *mut winlinks, _: *mut winlink) -> *mut winlink;
    pub fn winlinks_RB_FIND(_: *mut winlinks, _: *mut winlink) -> *mut winlink;
    pub fn winlinks_RB_NFIND(_: *mut winlinks, _: *mut winlink) -> *mut winlink;
    pub fn window_pane_tree_RB_INSERT_COLOR(_: *mut window_pane_tree, _: *mut window_pane);
    pub fn window_pane_tree_RB_REMOVE_COLOR(_: *mut window_pane_tree, _: *mut window_pane, _: *mut window_pane);
    pub fn window_pane_tree_RB_REMOVE(_: *mut window_pane_tree, _: *mut window_pane) -> *mut window_pane;
    pub fn window_pane_tree_RB_INSERT(_: *mut window_pane_tree, _: *mut window_pane) -> *mut window_pane;
    pub fn window_pane_tree_RB_FIND(_: *mut window_pane_tree, _: *mut window_pane) -> *mut window_pane;
    pub fn window_pane_tree_RB_NFIND(_: *mut window_pane_tree, _: *mut window_pane) -> *mut window_pane;
}

mod layout;
pub use crate::layout::{
    layout_assign_pane, layout_close_pane, layout_count_cells, layout_create_cell, layout_destroy_cell, layout_fix_offsets, layout_fix_panes, layout_free, layout_free_cell, layout_init, layout_make_leaf, layout_make_node, layout_print_cell, layout_resize, layout_resize_adjust, layout_resize_layout,
    layout_resize_pane, layout_resize_pane_to, layout_search_by_border, layout_set_size, layout_split_pane, layout_spread_cell, layout_spread_out,
};

mod layout_custom;
pub use crate::layout_custom::{layout_dump, layout_parse};

mod layout_set;
pub use crate::layout_set::{layout_set_lookup, layout_set_next, layout_set_previous, layout_set_select};

mod mode_tree;
pub use crate::mode_tree::{
    mode_tree_add, mode_tree_build, mode_tree_build_cb, mode_tree_collapse_current, mode_tree_count_tagged, mode_tree_data, mode_tree_down, mode_tree_draw, mode_tree_draw_as_parent, mode_tree_draw_cb, mode_tree_each_cb, mode_tree_each_tagged, mode_tree_expand, mode_tree_expand_current,
    mode_tree_free, mode_tree_get_current, mode_tree_get_current_name, mode_tree_height_cb, mode_tree_item, mode_tree_key, mode_tree_key_cb, mode_tree_menu_cb, mode_tree_no_tag, mode_tree_remove, mode_tree_resize, mode_tree_run_command, mode_tree_search_cb, mode_tree_set_current, mode_tree_start,
    mode_tree_up, mode_tree_zoom,
};

mod window_buffer;
pub use crate::window_buffer::window_buffer_mode;

mod window_tree;
pub use crate::window_tree::window_tree_mode;

mod window_clock;
pub use crate::window_clock::{window_clock_mode, window_clock_table};

mod window_client;
pub use crate::window_client::window_client_mode;

mod window_copy;
pub use crate::window_copy::{window_copy_add, window_copy_get_line, window_copy_get_word, window_copy_mode, window_copy_pagedown, window_copy_pageup, window_copy_start_drag, window_copy_vadd, window_view_mode};

mod window_customize;
pub use crate::window_customize::window_customize_mode;

mod names;
pub use crate::names::{check_window_name, default_window_name, parse_window_name};

mod control;
pub use crate::control::{
    control_add_sub, control_all_done, control_continue_pane, control_discard, control_pane_offset, control_pause_pane, control_ready, control_remove_sub, control_reset_offsets, control_set_pane_off, control_set_pane_on, control_start, control_state, control_stop, control_write,
    control_write_output,
};

mod control_notify;
pub use crate::control_notify::{
    control_notify_client_detached, control_notify_client_session_changed, control_notify_pane_mode_changed, control_notify_paste_buffer_changed, control_notify_paste_buffer_deleted, control_notify_session_closed, control_notify_session_created, control_notify_session_renamed,
    control_notify_session_window_changed, control_notify_window_layout_changed, control_notify_window_linked, control_notify_window_pane_changed, control_notify_window_renamed, control_notify_window_unlinked,
};

mod session_;
pub use crate::session_::{
    next_session_id, session_add_ref, session_alive, session_attach, session_check_name, session_cmp, session_create, session_destroy, session_detach, session_find, session_find_by_id, session_find_by_id_str, session_group_add, session_group_attached_count, session_group_contains,
    session_group_count, session_group_find, session_group_new, session_group_synchronize_from, session_group_synchronize_to, session_has, session_is_linked, session_last, session_next, session_next_session, session_previous, session_previous_session, session_remove_ref, session_renumber_windows,
    session_select, session_set_current, session_update_activity, sessions,
};
// sessions_RB_INSERT, sessions_RB_INSERT_COLOR, sessions_RB_NFIND, sessions_RB_REMOVE, sessions_RB_REMOVE_COLOR, sessions_RB_FIND

mod utf8;
pub use crate::utf8::{
    utf8_append, utf8_build_one, utf8_copy, utf8_cstrhas, utf8_cstrwidth, utf8_from_data, utf8_fromcstr, utf8_fromwc, utf8_in_table, utf8_isvalid, utf8_open, utf8_padcstr, utf8_rpadcstr, utf8_sanitize, utf8_set, utf8_stravis, utf8_stravisx, utf8_strlen, utf8_strvis, utf8_strwidth, utf8_to_data,
    utf8_tocstr, utf8_towc,
};

mod osdep;
pub use crate::osdep::{osdep_event_init, osdep_get_cwd, osdep_get_name};

mod utf8_combined;
pub use crate::utf8_combined::{utf8_has_zwj, utf8_is_modifier, utf8_is_vs, utf8_is_zwj};

// procname.c
unsafe extern "C" {
    pub unsafe fn get_proc_name(_: c_int, _: *mut c_char) -> *mut c_char;
    pub unsafe fn get_proc_cwd(_: c_int) -> *mut c_char;
}

#[macro_use] // log_debug
mod log;
use crate::log::log_debug;
pub use crate::log::{fatal, fatalx, log_add_level, log_close, log_get_level, log_open, log_toggle};
/*
unsafe extern "C" {
    pub unsafe fn fatal(msg: *const c_char, ap: ...) -> !;
    pub unsafe fn fatalx(msg: *const c_char, args: ...) -> !;
    pub unsafe fn log_add_level();
    pub fn log_close();
    pub unsafe fn log_debug(msg: *const c_char, args: ...);
    pub unsafe fn log_get_level() -> c_int;
    pub unsafe fn log_open(name: *const c_char);
    pub unsafe fn log_toggle(name: *const c_char);
}
*/

pub const MENU_NOMOUSE: i32 = 0x1;
pub const MENU_TAB: i32 = 0x2;
pub const MENU_STAYOPEN: i32 = 0x4;
mod menu_;
pub use crate::menu_::{menu_add_item, menu_add_items, menu_check_cb, menu_create, menu_data, menu_display, menu_draw_cb, menu_free, menu_free_cb, menu_key_cb, menu_mode_cb, menu_prepare};

pub const POPUP_CLOSEEXIT: i32 = 0x1;
pub const POPUP_CLOSEEXITZERO: i32 = 0x2;
pub const POPUP_INTERNAL: i32 = 0x4;
mod popup;
pub use crate::popup::{popup_close_cb, popup_display, popup_editor, popup_finish_edit_cb};

mod style_;
pub use crate::style_::{style_add, style_apply, style_copy, style_parse, style_set, style_tostring};

mod spawn;
pub use crate::spawn::{spawn_pane, spawn_window};

mod regsub;
pub use crate::regsub::regsub;

/* image.c */
unsafe extern "C" {}
/* image-sixel.c */
unsafe extern "C" {}

mod server_acl;
pub use crate::server_acl::{server_acl_display, server_acl_get_uid, server_acl_init, server_acl_join, server_acl_user, server_acl_user_allow, server_acl_user_allow_write, server_acl_user_deny, server_acl_user_deny_write, server_acl_user_find};

mod hyperlinks_;
pub use crate::hyperlinks_::{hyperlinks, hyperlinks_copy, hyperlinks_free, hyperlinks_get, hyperlinks_init, hyperlinks_put, hyperlinks_reset, hyperlinks_uri};

pub mod xmalloc;
pub use crate::xmalloc::{free_, memcpy_, memcpy__, xasprintf, xasprintf_, xcalloc, xcalloc_, xcalloc__, xcalloc1, xmalloc, xmalloc_, xrealloc, xrealloc_, xreallocarray_, xsnprintf, xstrdup, xstrdup_, xvasprintf};
/*
unsafe extern "C" {
    pub unsafe fn xasprintf(ret: *mut *mut c_char, fmt: *const c_char, args: ...) -> c_int;
    pub safe fn xcalloc(nmemb: usize, size: usize) -> NonNull<c_void>;
    pub safe fn xmalloc(size: usize) -> NonNull<c_void>;
    pub fn xreallocarray(ptr: *mut c_void, nmemb: usize, size: usize) -> NonNull<c_void>;
    pub unsafe fn xstrdup(str: *const c_char) -> NonNull<c_char>;
}
*/

pub mod tmux_protocol;
pub use crate::tmux_protocol::{PROTOCOL_VERSION, msg_command, msg_read_cancel, msg_read_data, msg_read_done, msg_read_open, msg_write_close, msg_write_data, msg_write_open, msg_write_ready, msgtype};

unsafe extern "C-unwind" {
    pub fn vsnprintf(_: *mut c_char, _: usize, _: *const c_char, _: VaList) -> c_int;
    pub fn vasprintf(_: *mut *mut c_char, _: *const c_char, _: VaList) -> c_int;
}

unsafe impl Sync for SyncCharPtr {}
#[repr(transparent)]
#[derive(Copy, Clone)]
struct SyncCharPtr(*const c_char);
impl SyncCharPtr {
    const fn new(value: &'static CStr) -> Self { Self(value.as_ptr()) }
    const fn from_ptr(value: *const c_char) -> Self { Self(value) }
    const fn null() -> Self { Self(null()) }
    const fn is_null(&self) -> bool { self.0.is_null() }
    const fn as_ptr(&self) -> *const c_char { self.0 }
}

// TODO this will eventually swap to be bool, but for now, while there is C code should be ffi compatible with i32
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct boolint(i32);
impl boolint {
    const fn true_() -> Self { Self(1) }
    const fn false_() -> Self { Self(0) }
    const fn as_bool(&self) -> bool { self.0 != 0 }
    const fn as_int(&self) -> i32 { self.0 }
}

impl From<boolint> for bool {
    fn from(value: boolint) -> Self { value.as_bool() }
}

impl From<bool> for boolint {
    fn from(value: bool) -> Self { Self(value as i32) }
}

impl std::ops::Not for boolint {
    type Output = bool;
    fn not(self) -> bool { self.0 == 0 }
}

// TODO struct should have some sort of lifetime
/// Display wrapper for a *c_char pointer
#[repr(transparent)]
pub struct _s(*const i8);
impl _s {
    unsafe fn from_raw(s: *const c_char) -> Self { _s(s) }
}
impl std::fmt::Display for _s {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_null() {
            f.write_str("(null)")
        } else {
            let len = unsafe { libc::strlen(self.0 as *const i8) };
            let s: &[u8] = unsafe { std::slice::from_raw_parts(self.0 as *const u8, len) };
            let s = std::str::from_utf8(s).unwrap_or("%s-invalid-utf8");
            f.write_str(s)
        }
    }
}

// TOOD make usable in const context
// https://stackoverflow.com/a/63904992
#[macro_export]
macro_rules! function_name {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str { std::any::type_name::<T>() }
        let name = type_name_of(f);

        // Find and cut the rest of the path
        match &name[..name.len() - 3].rfind(':') {
            Some(pos) => &name[pos + 1..name.len() - 3],
            None => &name[..name.len() - 3],
        }
    }};
}

pub const fn concat_array<const N: usize, const M: usize, const O: usize, T: Copy>(a1: [T; N], a2: [T; M]) -> [T; O] {
    let mut out: [MaybeUninit<T>; O] = [MaybeUninit::uninit(); O];

    let mut i: usize = 0;
    while i < a1.len() {
        out[i].write(a1[i]);
        i += 1;
    }
    while i < a1.len() + a2.len() {
        out[i].write(a2[i - a1.len()]);
        i += 1;
    }

    assert!(a1.len() + a2.len() == out.len());
    assert!(i == out.len());

    unsafe { std::mem::transmute_copy(&out) }
    // TODO once stabilized switch to:
    // unsafe { MaybeUninit::array_assume_init(out) }
}
