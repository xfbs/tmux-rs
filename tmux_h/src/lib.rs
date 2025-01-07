#![allow(non_camel_case_types)]
use core::{ffi::*, mem::ManuallyDrop};

use libc::{pid_t, termios, time_t, timeval};
use libevent_sys::{bufferevent, evbuffer, event};

use compat_rs::queue::{Entry, list_entry, list_head, tailq_entry, tailq_head};
use compat_rs::tree::{rb_entry, rb_head};

// use crate::tmux_protocol_h::*;

pub type bitstr_t = c_uchar;

const TTY_NAME_MAX: usize = 32;

// forward defs
pub struct cmds;
pub struct control_state;
pub struct environ;
pub struct format_job_tree;
pub struct format_tree;
pub struct hyperlinks_uri;
pub struct hyperlinks;
pub struct input_ctx;
pub struct job;
pub struct menu_data;
pub struct mode_tree_data;
pub struct options_array_item;
pub struct options_entry;
pub struct screen_write_citem;
pub struct screen_write_cline;
pub struct prompt_free_cb;

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
    cmd,
    cmdq_item,
    cmdq_list,
    options,
    msgtype
}

#[cfg(feature = "sixel")]
struct sixel_image;

pub struct tty_code;
pub struct tty_key;
pub struct tmuxpeer;
pub struct tmuxproc;

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
    masked > 0x7f
        && (masked < KEYC_BASE || masked >= KEYC_BASE_END)
        && (masked < KEYC_USER || masked >= KEYC_USER_END)
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
type utf8_char = c_uint;

// An expanded UTF-8 character. UTF8_SIZE must be big enough to hold combining
// characters as well. It can't be more than 32 bytes without changes to how
// characters are stored.
const UTF8_SIZE: usize = 21;

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct utf8_data {
    pub(crate) data: [c_uchar; UTF8_SIZE],

    pub(crate) have: c_uchar,
    pub(crate) size: c_uchar,

    /// 0xff if invalid
    pub(crate) width: c_uchar,
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
#[derive(Copy, Clone)]
pub(crate) struct colour_palette {
    pub(crate) fg: i32,
    pub(crate) bg: i32,

    pub(crate) palette: *mut i32,
    pub(crate) default_palette: *mut i32,
}

// Grid attributes. Anything above 0xff is stored in an extended cell.
pub(crate) const GRID_ATTR_BRIGHT: i32 = 0x1;
pub(crate) const GRID_ATTR_DIM: i32 = 0x2;
pub(crate) const GRID_ATTR_UNDERSCORE: i32 = 0x4;
pub(crate) const GRID_ATTR_BLINK: i32 = 0x8;
pub(crate) const GRID_ATTR_REVERSE: i32 = 0x10;
pub(crate) const GRID_ATTR_HIDDEN: i32 = 0x20;
pub(crate) const GRID_ATTR_ITALICS: i32 = 0x40;
pub(crate) const GRID_ATTR_CHARSET: i32 = 0x80; // alternative character set
pub(crate) const GRID_ATTR_STRIKETHROUGH: i32 = 0x100;
pub(crate) const GRID_ATTR_UNDERSCORE_2: i32 = 0x200;
pub(crate) const GRID_ATTR_UNDERSCORE_3: i32 = 0x400;
pub(crate) const GRID_ATTR_UNDERSCORE_4: i32 = 0x800;
pub(crate) const GRID_ATTR_UNDERSCORE_5: i32 = 0x1000;
pub(crate) const GRID_ATTR_OVERLINE: i32 = 0x2000;

/// All underscore attributes.
pub(crate) const GRID_ATTR_ALL_UNDERSCORE: i32 = GRID_ATTR_UNDERSCORE
    | GRID_ATTR_UNDERSCORE_2
    | GRID_ATTR_UNDERSCORE_3
    | GRID_ATTR_UNDERSCORE_4
    | GRID_ATTR_UNDERSCORE_5;

// Grid flags.
pub(crate) const GRID_FLAG_FG256: i32 = 0x1;
pub(crate) const GRID_FLAG_BG256: i32 = 0x2;
pub(crate) const GRID_FLAG_PADDING: i32 = 0x4;
pub(crate) const GRID_FLAG_EXTENDED: i32 = 0x8;
pub(crate) const GRID_FLAG_SELECTED: i32 = 0x10;
pub(crate) const GRID_FLAG_NOPALETTE: i32 = 0x20;
pub(crate) const GRID_FLAG_CLEARED: i32 = 0x40;

// Grid line flags.
pub(crate) const GRID_LINE_WRAPPED: i32 = 0x1;
pub(crate) const GRID_LINE_EXTENDED: i32 = 0x2;
pub(crate) const GRID_LINE_DEAD: i32 = 0x4;
pub(crate) const GRID_LINE_START_PROMPT: i32 = 0x8;
pub(crate) const GRID_LINE_START_OUTPUT: i32 = 0x10;

// Grid string flags.
pub(crate) const GRID_STRING_WITH_SEQUENCES: i32 = 0x1;
pub(crate) const GRID_STRING_ESCAPE_SEQUENCES: i32 = 0x2;
pub(crate) const GRID_STRING_TRIM_SPACES: i32 = 0x4;
pub(crate) const GRID_STRING_USED_ONLY: i32 = 0x8;
pub(crate) const GRID_STRING_EMPTY_CELLS: i32 = 0x10;

// Cell positions.
pub(crate) const CELL_INSIDE: i32 = 0;
pub(crate) const CELL_TOPBOTTOM: i32 = 1;
pub(crate) const CELL_LEFTRIGHT: i32 = 2;
pub(crate) const CELL_TOPLEFT: i32 = 3;
pub(crate) const CELL_TOPRIGHT: i32 = 4;
pub(crate) const CELL_BOTTOMLEFT: i32 = 5;
pub(crate) const CELL_BOTTOMRIGHT: i32 = 6;
pub(crate) const CELL_TOPJOIN: i32 = 7;
pub(crate) const CELL_BOTTOMJOIN: i32 = 8;
pub(crate) const CELL_LEFTJOIN: i32 = 9;
pub(crate) const CELL_RIGHTJOIN: i32 = 10;
pub(crate) const CELL_JOIN: i32 = 11;
pub(crate) const CELL_OUTSIDE: i32 = 12;

// Cell borders.
pub(crate) const CELL_BORDERS: &CStr = c" xqlkmjwvtun~";
pub(crate) const SIMPLE_BORDERS: &CStr = c" |-+++++++++.";
pub(crate) const PADDED_BORDERS: &CStr = c"             ";

/// Grid cell data.
#[derive(Copy, Clone)]
#[repr(C)]
pub(crate) struct grid_cell {
    pub(crate) data: utf8_data,
    pub(crate) attr: c_ushort,
    pub(crate) flags: c_uchar,
    pub(crate) fg: i32,
    pub(crate) bg: i32,
    pub(crate) us: i32,
    pub(crate) link: u32,
}

/// Grid extended cell entry.
pub(crate) type grid_extd_entry = grid_cell;

#[repr(C, align(4))]
pub(crate) struct grid_cell_entry_data {
    pub(crate) attr: c_uchar,
    pub(crate) fg: c_uchar,
    pub(crate) bg: c_uchar,
    pub(crate) data: c_uchar,
}
#[repr(C)]
pub struct grid_cell_entry {
    pub data: grid_cell_entry_data,
    pub flags: c_uchar,
}

/// Grid line.
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
pub struct grid_reader {
    pub gd: *mut grid,
    pub cx: u32,
    pub cy: u32,
}

/// Style alignment.
pub enum style_align {
    STYLE_ALIGN_DEFAULT,
    STYLE_ALIGN_LEFT,
    STYLE_ALIGN_CENTRE,
    STYLE_ALIGN_RIGHT,
    STYLE_ALIGN_ABSOLUTE_CENTRE,
}

/// Style list.
pub enum style_list {
    STYLE_LIST_OFF,
    STYLE_LIST_ON,
    STYLE_LIST_FOCUS,
    STYLE_LIST_LEFT_MARKER,
    STYLE_LIST_RIGHT_MARKER,
}

/// Style range.
pub enum style_range_type {
    STYLE_RANGE_NONE,
    STYLE_RANGE_LEFT,
    STYLE_RANGE_RIGHT,
    STYLE_RANGE_PANE,
    STYLE_RANGE_WINDOW,
    STYLE_RANGE_SESSION,
    STYLE_RANGE_USER,
}

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
pub enum style_default_type {
    STYLE_DEFAULT_BASE,
    STYLE_DEFAULT_PUSH,
    STYLE_DEFAULT_POP,
}

/// Style option.
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
pub static mut images: tailq_head<image> = tailq_head::const_default();

/// Cursor style.
#[derive(Copy, Clone)]
pub enum screen_cursor_style {
    SCREEN_CURSOR_DEFAULT,
    SCREEN_CURSOR_BLOCK,
    SCREEN_CURSOR_UNDERLINE,
    SCREEN_CURSOR_BAR,
}

pub struct screen_sel;
pub struct screen_titles;
/// Virtual screen.
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
pub type screen_write_init_ctx_cb = fn(*mut screen_write_ctx, *mut tty_ctx);
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

// screen size macros skipped for now

// Menu.
pub struct menu_item {
    pub name: *const c_char,
    pub key: key_code,
    pub command: *const c_char,
}
pub struct menu {
    pub title: *const c_char,
    pub items: *mut menu_item,
    pub count: u32,
    pub width: u32,
}
pub type menu_choice_cb = fn(*mut menu, u32, key_code, *mut c_void);

// Window mode. Windows can be in several modes and this is used to call the
// right function to handle input and output.
pub struct window_mode {
    pub name: *const c_char,
    pub default_format: *const c_char,

    pub init: Option<unsafe extern "C" fn(*mut window_mode_entry, *mut cmd_find_state, *mut args)>,
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

/// Offsets into pane buffer.
#[derive(Copy, Clone)]
pub struct window_pane_offset {
    pub used: usize,
}

/// Queued pane resize.
#[derive(Copy, Clone)]
pub struct window_pane_resize {
    pub sx: u32,
    pub sy: u32,

    pub osx: u32,
    pub osy: u32,

    pub entry: tailq_entry<window_pane_resize>,
}
pub type window_pane_resizes = tailq_head<window_pane_resize>;

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
pub type window_panes = tailq_head<window_pane>;
pub type window_pane_tree = rb_head<window_pane>;

pub const WINDOW_BELL: i32 = 0x1;
pub const WINDOW_ACTIVITY: i32 = 0x2;
pub const WINDOW_SILENCE: i32 = 0x4;
pub const WINDOW_ZOOMED: i32 = 0x8;
pub const WINDOW_WASZOOMED: i32 = 0x10;
pub const WINDOW_RESIZE: i32 = 0x20;
pub const WINDOW_ALERTFLAGS: i32 = WINDOW_BELL | WINDOW_ACTIVITY | WINDOW_SILENCE;

/// Window structure.
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
pub type winlink_stack = tailq_head<winlink>;

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
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum layout_type {
    LAYOUT_LEFTRIGHT,
    LAYOUT_TOPBOTTOM,
    LAYOUT_WINDOWPANE,
}

/// Layout cells queue.
pub type layout_cells = tailq_head<layout_cell>;

/// Layout cell.
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
pub struct environ_entry {
    pub name: *mut c_char,
    pub value: *mut c_char,

    pub flags: i32,
    pub entry: rb_entry<environ_entry>,
}

/// Client session.
pub struct session_group {
    pub name: *const c_char,
    pub sessions: tailq_head<session>,

    pub entry: rb_entry<session_group>,
}
pub type session_groups = rb_head<session_group>;

pub const SESSION_PASTING: i32 = 0x1;
pub const SESSION_ALERTED: i32 = 0x2;

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
    pub mouse_drag_update: fn(*mut client, *mut mouse_event),
    pub mouse_drag_release: fn(*mut client, *mut mouse_event),

    pub key_timer: event,
    pub key_tree: tty_key,
}

pub type tty_ctx_redraw_cb = fn(*const tty_ctx);
pub type tty_ctx_set_client_cb = fn(*mut tty_ctx, *mut client);

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
pub struct message_entry {
    pub msg: *mut c_char,
    pub msg_num: u32,
    pub msg_time: timeval,

    pub entry: tailq_entry<message_entry>,
}
pub type message_list = tailq_head<message_entry>;

/// Argument type.
pub enum args_type {
    ARGS_NONE,
    ARGS_STRING,
    ARGS_COMMANDS,
}

pub union args_value_union {
    pub string: *mut c_char,
    pub cmdlist: *mut cmd_list,
}

/// Argument value.
pub struct args_value {
    pub type_: args_type,
    pub args_value_union: args_value_union,
    pub cached: *mut c_char,
    pub entry: tailq_entry<args_value>,
}

struct args_entry;
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

pub type args_parse_cb =
    Option<unsafe extern "C" fn(*mut args, u32, *mut *mut c_char) -> args_parse_type>;
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
pub enum cmd_parse_status {
    CMD_PARSE_ERROR,
    CMD_PARSE_SUCCESS,
}
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
pub type cmdq_cb = fn(*mut cmdq_item, *mut c_void) -> cmd_retval;

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
struct status_line_entry {
    expanded: *mut c_char,
    ranges: style_ranges,
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
const PROMPT_NTYPES: usize = 4;
pub enum prompt_type {
    PROMPT_TYPE_COMMAND,
    PROMPT_TYPE_SEARCH,
    PROMPT_TYPE_TARGET,
    PROMPT_TYPE_WINDOW_TARGET,
    PROMPT_TYPE_INVALID = 0xff,
}

/* File in client. */
pub type client_file_cb = fn(*mut client, *mut c_char, i32, i32, *mut evbuffer, *mut c_void);
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
pub struct overlay_ranges {
    pub px: [u32; OVERLAY_MAX_RANGES],
    pub nx: [u32; OVERLAY_MAX_RANGES],
}

pub type prompt_input_cb = fn(*mut client, *mut c_void, *const c_char, i32) -> i32;
pub type prompt_free_fb = fn(*mut c_void);
pub type overlay_check_cb = fn(*mut client, *mut c_void, u32, u32, u32, *mut overlay_ranges);
pub type overlay_mode_cb = fn(*mut client, *mut c_void, *mut u32, *mut u32) -> *mut screen;
pub type overlay_draw_cb = fn(*mut client, *mut c_void, *mut screen_redraw_ctx);
pub type overlay_key_cb = fn(*mut client, *mut c_void, *mut key_event) -> i32;
pub type overlay_free_cb = fn(*mut client, *mut c_void);
pub type overlay_resize_cb = fn(*mut client, *mut c_void);

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
pub enum control_sub_type {
    CONTROL_SUB_SESSION,
    CONTROL_SUB_PANE,
    CONTROL_SUB_ALL_PANES,
    CONTROL_SUB_WINDOW,
    CONTROL_SUB_ALL_WINDOWS,
}

pub const KEY_BINDING_REPEAT: i32 = 0x1;

/// Key binding and key table.
pub struct key_binding {
    pub key: key_code,
    pub cmdlist: cmd_list,
    pub note: *const c_char,

    pub flags: i32,

    pub entry: rb_entry<key_binding>,
}
pub type key_bindings = rb_head<key_binding>;

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
pub union options_value {
    pub string: *mut c_char,
    pub number: c_longlong,
    pub style: ManuallyDrop<style>,
    pub array: ManuallyDrop<options_array>,
    pub cmdlist: *mut cmd_list,
}

// Option table entries.
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
pub struct mode_tree_sort_criteria {
    pub field: u32,
    pub reversed: i32,
}

// panic!();

pub const WINDOW_MINIMUM: i32 = PANE_MINIMUM;
pub const WINDOW_MAXIMUM: i32 = 10_000;

pub enum exit_type {
    CLIENT_EXIT_RETURN,
    CLIENT_EXIT_SHUTDOWN,
    CLIENT_EXIT_DETACH,
}

pub enum prompt_mode {
    PROMPT_ENTRY,
    PROMPT_COMMAND,
}

pub const FORMAT_STATUS: i32 = 0x1;
pub const FORMAT_FORCE: i32 = 0x2;
pub const FORMAT_NOJOBS: i32 = 0x4;
pub const FORMAT_VERBOSE: i32 = 0x8;
pub const FORMAT_NONE: i32 = 0;
pub const FORMAT_PANE: u32 = 0x80000000;
pub const FORMAT_WINDOW: u32 = 0x40000000;
