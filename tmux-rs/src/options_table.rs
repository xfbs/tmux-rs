use super::*;

unsafe extern "C" {
    // TODO I don't know the actual length, so fix this
    // pub static options_table: [options_table_entry; 191];
    // pub static options_other_names: [options_name_map; 6];
}

//This file has a tables with all the server, session and window
//options. These tables are the master copy of the options with their real
//(user-visible) types, range limits and default values. At start these are
//copied into the runtime global options trees (which only has number and
//string types). These tables are then used to look up the real type when the
//user sets an option or its value needs to be shown.

// Choice option type lists.
static mut options_table_mode_keys_list: [*const c_char; 3] = [c"emacs".as_ptr(), c"vi".as_ptr(), null()];
static mut options_table_clock_mode_style_list: [*const c_char; 3] = [c"12".as_ptr(), c"24".as_ptr(), null()];
static mut options_table_status_list: [*const c_char; 7] = [
    c"off".as_ptr(),
    c"on".as_ptr(),
    c"2".as_ptr(),
    c"3".as_ptr(),
    c"4".as_ptr(),
    c"5".as_ptr(),
    null(),
];
static mut options_table_message_line_list: [*const c_char; 6] = [
    c"0".as_ptr(),
    c"1".as_ptr(),
    c"2".as_ptr(),
    c"3".as_ptr(),
    c"4".as_ptr(),
    null(),
];
static mut options_table_status_keys_list: [*const c_char; 3] = [c"emacs".as_ptr(), c"vi".as_ptr(), null()];
static mut options_table_status_justify_list: [*const c_char; 5] = [
    c"left".as_ptr(),
    c"centre".as_ptr(),
    c"right".as_ptr(),
    c"absolute-centre".as_ptr(),
    null(),
];
static mut options_table_status_position_list: [*const c_char; 3] = [c"top".as_ptr(), c"bottom".as_ptr(), null()];
static mut options_table_bell_action_list: [*const c_char; 5] = [
    c"none".as_ptr(),
    c"any".as_ptr(),
    c"current".as_ptr(),
    c"other".as_ptr(),
    null(),
];
static mut options_table_visual_bell_list: [*const c_char; 4] =
    [c"off".as_ptr(), c"on".as_ptr(), c"both".as_ptr(), null()];
static mut options_table_cursor_style_list: [*const c_char; 8] = [
    c"default".as_ptr(),
    c"blinking-block".as_ptr(),
    c"block".as_ptr(),
    c"blinking-underline".as_ptr(),
    c"underline".as_ptr(),
    c"blinking-bar".as_ptr(),
    c"bar".as_ptr(),
    null(),
];
static mut options_table_pane_status_list: [*const c_char; 4] =
    [c"off".as_ptr(), c"top".as_ptr(), c"bottom".as_ptr(), null()];
static mut options_table_pane_border_indicators_list: [*const c_char; 5] = [
    c"off".as_ptr(),
    c"colour".as_ptr(),
    c"arrows".as_ptr(),
    c"both".as_ptr(),
    null(),
];
static mut options_table_pane_border_lines_list: [*const c_char; 6] = [
    c"single".as_ptr(),
    c"double".as_ptr(),
    c"heavy".as_ptr(),
    c"simple".as_ptr(),
    c"number".as_ptr(),
    null(),
];
static mut options_table_popup_border_lines_list: [*const c_char; 8] = [
    c"single".as_ptr(),
    c"double".as_ptr(),
    c"heavy".as_ptr(),
    c"simple".as_ptr(),
    c"rounded".as_ptr(),
    c"padded".as_ptr(),
    c"none".as_ptr(),
    null(),
];
static mut options_table_set_clipboard_list: [*const c_char; 4] =
    [c"off".as_ptr(), c"external".as_ptr(), c"on".as_ptr(), null()];
static mut options_table_window_size_list: [*const c_char; 5] = [
    c"largest".as_ptr(),
    c"smallest".as_ptr(),
    c"manual".as_ptr(),
    c"latest".as_ptr(),
    null(),
];
static mut options_table_remain_on_exit_list: [*const c_char; 4] =
    [c"off".as_ptr(), c"on".as_ptr(), c"failed".as_ptr(), null()];
static mut options_table_destroy_unattached_list: [*const c_char; 5] = [
    c"off".as_ptr(),
    c"on".as_ptr(),
    c"keep-last".as_ptr(),
    c"keep-group".as_ptr(),
    null(),
];
static mut options_table_detach_on_destroy_list: [*const c_char; 6] = [
    c"off".as_ptr(),
    c"on".as_ptr(),
    c"no-detached".as_ptr(),
    c"previous".as_ptr(),
    c"next".as_ptr(),
    null(),
];
static mut options_table_extended_keys_list: [*const c_char; 4] =
    [c"off".as_ptr(), c"on".as_ptr(), c"always".as_ptr(), null()];
static mut options_table_extended_keys_format_list: [*const c_char; 3] = [c"csi-u".as_ptr(), c"xterm".as_ptr(), null()];
static mut options_table_allow_passthrough_list: [*const c_char; 4] =
    [c"off".as_ptr(), c"on".as_ptr(), c"all".as_ptr(), null()];

/// Map of name conversions.
#[unsafe(no_mangle)]
pub static mut options_other_names: [options_name_map; 6] = [
    options_name_map::new(c"display-panes-color".as_ptr(), c"display-panes-colour".as_ptr()),
    options_name_map::new(
        c"display-panes-active-color".as_ptr(),
        c"display-panes-active-colour".as_ptr(),
    ),
    options_name_map::new(c"clock-mode-color".as_ptr(), c"clock-mode-colour".as_ptr()),
    options_name_map::new(c"cursor-color".as_ptr(), c"cursor-colour".as_ptr()),
    options_name_map::new(c"pane-colors".as_ptr(), c"pane-colours".as_ptr()),
    options_name_map::new(null(), null()),
];

/// Status line format.
pub const OPTIONS_TABLE_STATUS_FORMAT1: *const c_char = concat!(
    "#[align=left range=left #{E:status-left-style}]",
    "#[push-default]",
    "#{T;=/#{status-left-length}:status-left}",
    "#[pop-default]",
    "#[norange default]",
    "#[list=on align=#{status-justify}]",
    "#[list=left-marker]<#[list=right-marker]>#[list=on]",
    "#{W:",
    "#[range=window|#{window_index} ",
    "#{E:window-status-style}",
    "#{?#{&&:#{window_last_flag},",
    "#{!=:#{E:window-status-last-style},default}}, ",
    "#{E:window-status-last-style},",
    "}",
    "#{?#{&&:#{window_bell_flag},",
    "#{!=:#{E:window-status-bell-style},default}}, ",
    "#{E:window-status-bell-style},",
    "#{?#{&&:#{||:#{window_activity_flag},",
    "#{window_silence_flag}},",
    "#{!=:",
    "#{E:window-status-activity-style},",
    "default}}, ",
    "#{E:window-status-activity-style},",
    "}",
    "}",
    "]",
    "#[push-default]",
    "#{T:window-status-format}",
    "#[pop-default]",
    "#[norange default]",
    "#{?window_end_flag,,#{window-status-separator}}",
    ",",
    "#[range=window|#{window_index} list=focus ",
    "#{?#{!=:#{E:window-status-current-style},default},",
    "#{E:window-status-current-style},",
    "#{E:window-status-style}",
    "}",
    "#{?#{&&:#{window_last_flag},",
    "#{!=:#{E:window-status-last-style},default}}, ",
    "#{E:window-status-last-style},",
    "}",
    "#{?#{&&:#{window_bell_flag},",
    "#{!=:#{E:window-status-bell-style},default}}, ",
    "#{E:window-status-bell-style},",
    "#{?#{&&:#{||:#{window_activity_flag},",
    "#{window_silence_flag}},",
    "#{!=:",
    "#{E:window-status-activity-style},",
    "default}}, ",
    "#{E:window-status-activity-style},",
    "}",
    "}",
    "]",
    "#[push-default]",
    "#{T:window-status-current-format}",
    "#[pop-default]",
    "#[norange list=on default]",
    "#{?window_end_flag,,#{window-status-separator}}",
    "}",
    "#[nolist align=right range=right #{E:status-right-style}]",
    "#[push-default]",
    "#{T;=/#{status-right-length}:status-right}",
    "#[pop-default]",
    "#[norange default]\0"
)
.as_ptr()
.cast();

pub const OPTIONS_TABLE_STATUS_FORMAT2: *const c_char = concat!(
    "#[align=centre]#{P:#{?pane_active,#[reverse],}",
    "#{pane_index}[#{pane_width}x#{pane_height}]#[default] }\0"
)
.as_ptr()
.cast();

#[unsafe(no_mangle)]
pub static mut options_table_status_format_default: [*const c_char; 3] =
    [OPTIONS_TABLE_STATUS_FORMAT1, OPTIONS_TABLE_STATUS_FORMAT2, null()];

/* Helpers for hook options. */
macro_rules! options_table_hook {
    ($hook_name:expr, $default_value:expr) => {
        options_table_entry {
            name: $hook_name.as_ptr(),
            type_: options_table_type::OPTIONS_TABLE_COMMAND,
            scope: OPTIONS_TABLE_SESSION,
            flags: OPTIONS_TABLE_IS_ARRAY | OPTIONS_TABLE_IS_HOOK,
            default_str: $default_value.as_ptr(),
            separator: c"".as_ptr(),
            ..unsafe { zeroed() }
        }
    };
}

macro_rules! options_table_pane_hook {
    ($hook_name:expr, $default_value:expr) => {
        options_table_entry {
            name: $hook_name.as_ptr(),
            type_: options_table_type::OPTIONS_TABLE_COMMAND,
            scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
            flags: OPTIONS_TABLE_IS_ARRAY | OPTIONS_TABLE_IS_HOOK,
            default_str: $default_value.as_ptr(),
            separator: c"".as_ptr(),
            ..unsafe { zeroed() }
        }
    };
}

macro_rules! options_table_window_hook {
    ($hook_name:expr, $default_value:expr) => {
        options_table_entry {
            name: $hook_name.as_ptr(),
            type_: options_table_type::OPTIONS_TABLE_COMMAND,
            scope: OPTIONS_TABLE_WINDOW,
            flags: OPTIONS_TABLE_IS_ARRAY | OPTIONS_TABLE_IS_HOOK,
            default_str: $default_value.as_ptr(),
            separator: c"".as_ptr(),
            ..unsafe { zeroed() }
        }
    };
}

#[unsafe(no_mangle)]
pub static mut options_table: [options_table_entry; 191] = [options_table_entry {
    name: c"backspace".as_ptr(),
    type_: options_table_type::OPTIONS_TABLE_KEY,
    scope: OPTIONS_TABLE_SERVER,
    default_num: b'\x7f' as i64,
    text: c"The key to send for backspace.".as_ptr(),
    ..unsafe { zeroed() }
},

    options_table_entry {
        name: c"buffer-limit".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 1,
        maximum: i32::MAX as u32,
        default_num: 50,
        text: c"The maximum number of automatic buffers. When this is reached, the oldest buffer is deleted.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"command-alias".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: c"split-pane=split-window,splitp=split-window,server-info=show-messages -JT,info=show-messages -JT,choose-window=choose-tree -w,choose-session=choose-tree -s".as_ptr(),
        separator: c",".as_ptr(),
        text: c"Array of command aliases. Each entry is an alias and a command separated by '='.".as_ptr(),
        ..unsafe { zeroed() }
    },
    options_table_entry {
        name: c"copy-command".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        default_str: c"".as_ptr(),
        text: c"Shell command run when text is copied. If empty, no command is run.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"cursor-colour".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: -1,
        text: c"Colour of the cursor.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"cursor-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        choices: &raw const options_table_cursor_style_list as *const *const c_char,
        default_num: 0,
        text: c"Style of the cursor.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"default-terminal".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        default_str: TMUX_TERM.as_ptr(),
        text: c"Default for the 'TERM' environment variable.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"editor".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        default_str: _PATH_VI,
        text: c"Editor run to edit files.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"escape-time".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 10,
        unit: c"milliseconds".as_ptr(),
        text: c"Time to wait before assuming a key is Escape.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"exit-empty".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SERVER,
        default_num: 1,
        text: c"Whether the server should exit if there are no sessions.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"exit-unattached".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SERVER,
        default_num: 0,
        text: c"Whether the server should exit if there are no attached clients.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"extended-keys".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SERVER,
        choices: &raw const options_table_extended_keys_list as *const *const c_char,
        default_num: 0,
        text: c"Whether to request extended key sequences from terminals that support it.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"extended-keys-format".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SERVER,
        choices: &raw const options_table_extended_keys_format_list as *const *const c_char,
        default_num: 1,
        text: c"The format of emitted extended key sequences.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"focus-events".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SERVER,
        default_num: 0,
        text: c"Whether to send focus events to applications.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"history-file".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        default_str: c"".as_ptr(),
        text: c"Location of the command prompt history file. Empty does not write a history file.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"menu-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        flags: OPTIONS_TABLE_IS_STYLE,
        default_str: c"default".as_ptr(),
        separator: c",".as_ptr(),
        text: c"Default style of menu.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"menu-selected-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        flags: OPTIONS_TABLE_IS_STYLE,
        default_str: c"bg=yellow,fg=black".as_ptr(),
        separator: c",".as_ptr(),
        text: c"Default style of selected menu item.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"menu-border-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"default".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Default style of menu borders.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"menu-border-lines".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const options_table_popup_border_lines_list as *const *const c_char,
        default_num: box_lines::BOX_LINES_SINGLE as i64,
        text: c"Type of characters used to draw menu border lines. Some of these are only supported on terminals with UTF-8 support.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"message-limit".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 1000,
        text: c"Maximum number of server messages to keep.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"prefix-timeout".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 0,
        unit: c"milliseconds".as_ptr(),
        text: c"The timeout for the prefix key if no subsequent key is pressed. Zero means disabled.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"prompt-history-limit".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SERVER,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 100,
        text: c"Maximum number of commands to keep in history.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"set-clipboard".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SERVER,
        choices: &raw const options_table_set_clipboard_list as *const *const c_char,
        default_num: 1,
        text: c"Whether to attempt to set the system clipboard ('on' or 'external') and whether to allow applications to create paste buffers with an escape sequence ('on' only).".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"terminal-overrides".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: c"linux*:AX@".as_ptr(),
        separator: c",".as_ptr(),
        text: c"List of terminal capabilities overrides.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"terminal-features".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: c"xterm*:clipboard:ccolour:cstyle:focus:title,screen*:title,rxvt*:ignorefkeys".as_ptr(),
        separator: c",".as_ptr(),
        text: c"List of terminal features, used if they cannot be automatically detected.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"user-keys".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SERVER,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: c"".as_ptr(),
        separator: c",".as_ptr(),
        text: c"User key assignments. Each sequence in the list is translated into a key: 'User0', 'User1' and so on.".as_ptr(),
        ..unsafe { zeroed() }
    },

    /* Session options. */
    options_table_entry {
        name: c"activity-action".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_bell_action_list as *const *const c_char,
        default_num: ALERT_OTHER as i64,
        text: c"Action to take on an activity alert.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"assume-paste-time".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 1,
        unit: c"milliseconds".as_ptr(),
        text: c"Maximum time between input to assume it is pasting rather than typing.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"base-index".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 0,
        text: c"Default index of the first window in each session.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"bell-action".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_bell_action_list as *const *const c_char,
        default_num: ALERT_ANY as i64,
        text: c"Action to take on a bell alert.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"default-command".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: c"".as_ptr(),
        text: c"Default command to run in new panes. If empty, a shell is started.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"default-shell".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: _PATH_BSHELL,
        text: c"Location of default shell.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"default-size".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        pattern: c"[0-9]*x[0-9]*".as_ptr(),
        default_str: c"80x24".as_ptr(),
        text: c"Initial size of new sessions.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"destroy-unattached".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_destroy_unattached_list as *const *const c_char,
        default_num: 0,
        text: c"Whether to destroy sessions when they have no attached clients, or keep the last session whether in the group.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"detach-on-destroy".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_detach_on_destroy_list as *const *const c_char,
        default_num: 1,
        text: c"Whether to detach when a session is destroyed, or switch the client to another session if any exist.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"display-panes-active-colour".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 1,
        text: c"Colour of the active pane for 'display-panes'.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"display-panes-colour".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 4,
        text: c"Colour of not active panes for 'display-panes'.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"display-panes-time".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 1,
        maximum: i32::MAX as u32,
        default_num: 1000,
        unit: c"milliseconds".as_ptr(),
        text: c"Time for which 'display-panes' should show pane numbers.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"display-time".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 750,
        unit: c"milliseconds".as_ptr(),
        text: c"Time for which status line messages should appear.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"history-limit".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 2000,
        unit: c"lines".as_ptr(),
        text: c"Maximum number of lines to keep in the history for each pane. If changed, the new value applies only to new panes.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"key-table".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: c"root".as_ptr(),
        text: c"Default key table. Key presses are first looked up in this table.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"lock-after-time".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 0,
        unit: c"seconds".as_ptr(),
        text: c"Time after which a client is locked if not used.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"lock-command".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: TMUX_LOCK_CMD.as_ptr(),
        text: c"Shell command to run to lock a client.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"message-command-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: c"bg=black,fg=yellow".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of the command prompt when in command mode, if 'mode-keys' is set to 'vi'.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"message-line".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_message_line_list as *const *const c_char,
        default_num: 0,
        text: c"Position (line) of messages and the command prompt.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"message-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: c"bg=yellow,fg=black".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of messages and the command prompt.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"mouse".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 0,
        text: c"Whether the mouse is recognised and mouse key bindings are executed. Applications inside panes can use the mouse even when 'off'.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"prefix".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_KEY,
        scope: OPTIONS_TABLE_SESSION,
        default_num: b'b' as i64 | KEYC_CTRL as i64,
        text: c"The prefix key.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"prefix2".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_KEY,
        scope: OPTIONS_TABLE_SESSION,
        default_num: KEYC_NONE as i64,
        text: c"A second prefix key.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"renumber-windows".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 0,
        text: c"Whether windows are automatically renumbered rather than leaving gaps.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"repeat-time".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i16::MAX as u32,
        default_num: 500,
        unit: c"milliseconds".as_ptr(),
        text: c"Time to wait for a key binding to repeat, if it is bound with the '-r' flag.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"set-titles".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 0,
        text: c"Whether to set the terminal title, if supported.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"set-titles-string".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: c"#S:#I:#W - \"#T\" #{session_alerts}".as_ptr(),
        text: c"Format of the terminal title to set.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"silence-action".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_bell_action_list as *const *const c_char,
        default_num: ALERT_OTHER as i64,
        text: c"Action to take on a silence alert.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_status_list as *const *const c_char,
        default_num: 1,
        text: c"Number of lines in the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-bg".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 8,
        text: c"Background colour of the status line. This option is deprecated, use 'status-style' instead.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-fg".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_SESSION,
        default_num: 8,
        text: c"Foreground colour of the status line. This option is deprecated, use 'status-style' instead.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-format".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_arr: &raw const options_table_status_format_default as *const *const c_char,
        text: c"Formats for the status lines. Each array member is the format for one status line. The default status line is made up of several components which may be configured individually with other options such as 'status-left'.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-interval".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 15,
        unit: c"seconds".as_ptr(),
        text: c"Number of seconds between status line updates.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-justify".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_status_justify_list as *const *const c_char,
        default_num: 0,
        text: c"Position of the window list in the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-keys".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_status_keys_list as *const *const c_char,
        default_num: MODEKEY_EMACS as i64,
        text: c"Key set to use at the command prompt.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-left".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: c"[#{session_name}] ".as_ptr(),
        text: c"Contents of the left side of the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-left-length".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i16::MAX as u32,
        default_num: 10,
        text: c"Maximum width of the left side of the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-left-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: c"default".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of the left side of the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-position".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_status_position_list as *const *const c_char,
        default_num: 1,
        text: c"Position of the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-right".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: c"#{?window_bigger,[#{window_offset_x}#,#{window_offset_y}] ,}\"#{=21:pane_title}\" %H:%M %d-%b-%y".as_ptr(),
        text: c"Contents of the right side of the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-right-length".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_SESSION,
        minimum: 0,
        maximum: i16::MAX as u32,
        default_num: 40,
        text: c"Maximum width of the right side of the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-right-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: c"default".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of the right side of the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"status-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: c"bg=green,fg=black".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"update-environment".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        flags: OPTIONS_TABLE_IS_ARRAY,
        default_str: c"DISPLAY KRB5CCNAME SSH_ASKPASS SSH_AUTH_SOCK SSH_AGENT_PID SSH_CONNECTION WINDOWID XAUTHORITY".as_ptr(),
        text: c"List of environment variables to update in the session environment when a client is attached.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"visual-activity".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_visual_bell_list as *const *const c_char,
        default_num: VISUAL_OFF as i64,
        text: c"How activity alerts should be shown: a message ('on'), a message and a bell ('both') or nothing ('off').".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"visual-bell".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_visual_bell_list as *const *const c_char,
        default_num: VISUAL_OFF as i64,
        text: c"How bell alerts should be shown: a message ('on'), a message and a bell ('both') or nothing ('off').".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"visual-silence".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_SESSION,
        choices: &raw const options_table_visual_bell_list as *const *const c_char,
        default_num: VISUAL_OFF as i64,
        text: c"How silence alerts should be shown: a message ('on'), a message and a bell ('both') or nothing ('off').".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"word-separators".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_SESSION,
        default_str: c"!\"#$%&'()*+,-./:;<=>?@[\\]^`{|}~".as_ptr(),
        text: c"Characters considered to separate words.".as_ptr(),
        ..unsafe { zeroed() }
    },

    /* Window options */
    options_table_entry {
        name: c"aggressive-resize".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 0,
        text: c"When 'window-size' is 'smallest', whether the maximum size of a window is the smallest attached session where it is the current window ('on') or the smallest session it is linked to ('off').".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"allow-passthrough".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        choices: &raw const options_table_allow_passthrough_list as *const *const c_char,
        default_num: 0,
        text: c"Whether applications are allowed to use the escape sequence to bypass tmux. Can be 'off' (disallowed), 'on' (allowed if the pane is visible), or 'all' (allowed even if the pane is invisible).".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"allow-rename".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 0,
        text: c"Whether applications are allowed to use the escape sequence to rename windows.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"allow-set-title".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 1,
        text: c"Whether applications are allowed to use the escape sequence to set the pane title.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"alternate-screen".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 1,
        text: c"Whether applications are allowed to use the alternate screen.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"automatic-rename".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 1,
        text: c"Whether windows are automatically renamed.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"automatic-rename-format".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"#{?pane_in_mode,[tmux],#{pane_current_command}}#{?pane_dead,[dead],}".as_ptr(),
        text: c"Format used to automatically rename windows.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"clock-mode-colour".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 4,
        text: c"Colour of the clock in clock mode.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"clock-mode-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const options_table_clock_mode_style_list as *const *const c_char,
        default_num: 1,
        text: c"Time format of the clock in clock mode.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"copy-mode-match-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"bg=cyan,fg=black".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of search matches in copy mode.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"copy-mode-current-match-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"bg=magenta,fg=black".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of the current search match in copy mode.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"copy-mode-mark-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"bg=red,fg=black".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of the marked line in copy mode.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"fill-character".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"".as_ptr(),
        text: c"Character used to fill unused parts of window.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"main-pane-height".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"24".as_ptr(),
        text: c"Height of the main pane in the 'main-horizontal' layout. This may be a percentage, for example '10%'.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"main-pane-width".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"80".as_ptr(),
        text: c"Width of the main pane in the 'main-vertical' layout. This may be a percentage, for example '10%'.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"mode-keys".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const options_table_mode_keys_list as *const *const c_char,
        default_num: MODEKEY_EMACS as i64,
        text: c"Key set used in copy mode.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"mode-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        flags: OPTIONS_TABLE_IS_STYLE,
        default_str: c"bg=yellow,fg=black".as_ptr(),
        separator: c",".as_ptr(),
        text: c"Style of indicators and highlighting in modes.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"monitor-activity".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 0,
        text: c"Whether an alert is triggered by activity.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"monitor-bell".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 1,
        text: c"Whether an alert is triggered by a bell.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"monitor-silence".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_WINDOW,
        minimum: 0,
        maximum: i32::MAX as u32,
        default_num: 0,
        text: c"Time after which an alert is triggered by silence. Zero means no alert.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"other-pane-height".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"0".as_ptr(),
        text: c"Height of the other panes in the 'main-horizontal' layout. This may be a percentage, for example '10%'.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"other-pane-width".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"0".as_ptr(),
        text: c"Height of the other panes in the 'main-vertical' layout. This may be a percentage, for example '10%'.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"pane-active-border-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"#{?pane_in_mode,fg=yellow,#{?synchronize-panes,fg=red,fg=green}}".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of the active pane border.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"pane-base-index".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_NUMBER,
        scope: OPTIONS_TABLE_WINDOW,
        minimum: 0,
        maximum: u16::MAX as u32,
        default_num: 0,
        text: c"Index of the first pane in each window.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"pane-border-format".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: c"#{?pane_active,#[reverse],}#{pane_index}#[default] \"#{pane_title}\"".as_ptr(),
        text: c"Format of text in the pane status lines.".as_ptr(),
        ..unsafe { zeroed() }
    },


    options_table_entry {
        name: c"pane-border-indicators".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const options_table_pane_border_indicators_list as *const *const c_char,
        default_num: PANE_BORDER_COLOUR as i64,
        text: c"Whether to indicate the active pane by colouring border or displaying arrow markers.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"pane-border-lines".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const options_table_pane_border_lines_list as *const *const c_char,
        default_num: pane_lines::PANE_LINES_SINGLE as i64,
        text: c"Type of characters used to draw pane border lines. Some of these are only supported on terminals with UTF-8 support.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"pane-border-status".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const options_table_pane_status_list as *const *const c_char,
        default_num: PANE_STATUS_OFF as i64,
        text: c"Position of the pane status lines.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"pane-border-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"default".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of the pane status lines.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"pane-colours".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_COLOUR,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: c"".as_ptr(),
        flags: OPTIONS_TABLE_IS_ARRAY,
        text: c"The default colour palette for colours zero to 255.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"popup-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"default".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Default style of popups.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"popup-border-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"default".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Default style of popup borders.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"popup-border-lines".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const options_table_popup_border_lines_list as *const *const c_char,
        default_num: box_lines::BOX_LINES_SINGLE as i64,
        text: c"Type of characters used to draw popup border lines. Some of these are only supported on terminals with UTF-8 support.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"remain-on-exit".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        choices: &raw const options_table_remain_on_exit_list as *const *const c_char,
        default_num: 0,
        text: c"Whether panes should remain ('on') or be automatically killed ('off' or 'failed') when the program inside exits.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"remain-on-exit-format".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: c"Pane is dead (#{?#{!=:#{pane_dead_status},},status #{pane_dead_status},}#{?#{!=:#{pane_dead_signal},},signal #{pane_dead_signal},}, #{t:pane_dead_time})".as_ptr(),
        text: c"Message shown after the program in a pane has exited, if remain-on-exit is enabled.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"scroll-on-clear".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 1,
        text: c"Whether the contents of the screen should be scrolled into history when clearing the whole screen.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"synchronize-panes".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_num: 0,
        text: c"Whether typing should be sent to all panes simultaneously.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"window-active-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: c"default".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Default style of the active pane.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"window-size".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_CHOICE,
        scope: OPTIONS_TABLE_WINDOW,
        choices: &raw const options_table_window_size_list as *const *const c_char,
        default_num: WINDOW_SIZE_LATEST as i64,
        text: c"How window size is calculated. 'latest' uses the size of the most recently used client, 'largest' the largest client, 'smallest' the smallest client and 'manual' a size set by the 'resize-window' command.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"window-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE,
        default_str: c"default".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Default style of panes that are not the active pane.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"window-status-activity-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"reverse".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of windows in the status line with an activity alert.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"window-status-bell-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"reverse".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of windows in the status line with a bell alert.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"window-status-current-format".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"#I:#W#{?window_flags,#{window_flags}, }".as_ptr(),
        text: c"Format of the current window in the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"window-status-current-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"default".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of the current window in the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"window-status-format".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"#I:#W#{?window_flags,#{window_flags}, }".as_ptr(),
        text: c"Format of windows in the status line, except the current window.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"window-status-last-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"default".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of the last window in the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"window-status-separator".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c" ".as_ptr(),
        text: c"Separator between windows in the status line.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"window-status-style".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_STRING,
        scope: OPTIONS_TABLE_WINDOW,
        default_str: c"default".as_ptr(),
        flags: OPTIONS_TABLE_IS_STYLE,
        separator: c",".as_ptr(),
        text: c"Style of windows in the status line, except the current and last windows.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"wrap-search".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 1,
        text: c"Whether searching in copy mode should wrap at the top or bottom.".as_ptr(),
        ..unsafe { zeroed() }
    },

    options_table_entry {
        name: c"xterm-keys".as_ptr(),
        type_: options_table_type::OPTIONS_TABLE_FLAG,
        scope: OPTIONS_TABLE_WINDOW,
        default_num: 1,
        text: c"Whether xterm-style function key sequences should be sent. This option is no longer used.".as_ptr(),
        ..unsafe { zeroed() }
    },
    /* Hook options. */
    options_table_hook!(c"after-bind-key", c""),
    options_table_hook!(c"after-capture-pane", c""),
    options_table_hook!(c"after-copy-mode", c""),
    options_table_hook!(c"after-display-message", c""),
    options_table_hook!(c"after-display-panes", c""),
    options_table_hook!(c"after-kill-pane", c""),
    options_table_hook!(c"after-list-buffers", c""),
    options_table_hook!(c"after-list-clients", c""),
    options_table_hook!(c"after-list-keys", c""),
    options_table_hook!(c"after-list-panes", c""),
    options_table_hook!(c"after-list-sessions", c""),
    options_table_hook!(c"after-list-windows", c""),
    options_table_hook!(c"after-load-buffer", c""),
    options_table_hook!(c"after-lock-server", c""),
    options_table_hook!(c"after-new-session", c""),
    options_table_hook!(c"after-new-window", c""),
    options_table_hook!(c"after-paste-buffer", c""),
    options_table_hook!(c"after-pipe-pane", c""),
    options_table_hook!(c"after-queue", c""),
    options_table_hook!(c"after-refresh-client", c""),
    options_table_hook!(c"after-rename-session", c""),
    options_table_hook!(c"after-rename-window", c""),
    options_table_hook!(c"after-resize-pane", c""),
    options_table_hook!(c"after-resize-window", c""),
    options_table_hook!(c"after-save-buffer", c""),
    options_table_hook!(c"after-select-layout", c""),
    options_table_hook!(c"after-select-pane", c""),
    options_table_hook!(c"after-select-window", c""),
    options_table_hook!(c"after-send-keys", c""),
    options_table_hook!(c"after-set-buffer", c""),
    options_table_hook!(c"after-set-environment", c""),
    options_table_hook!(c"after-set-hook", c""),
    options_table_hook!(c"after-set-option", c""),
    options_table_hook!(c"after-show-environment", c""),
    options_table_hook!(c"after-show-messages", c""),
    options_table_hook!(c"after-show-options", c""),
    options_table_hook!(c"after-split-window", c""),
    options_table_hook!(c"after-unbind-key", c""),
    options_table_hook!(c"alert-activity", c""),
    options_table_hook!(c"alert-bell", c""),
    options_table_hook!(c"alert-silence", c""),
    options_table_hook!(c"client-active", c""),
    options_table_hook!(c"client-attached", c""),
    options_table_hook!(c"client-detached", c""),
    options_table_hook!(c"client-focus-in", c""),
    options_table_hook!(c"client-focus-out", c""),
    options_table_hook!(c"client-resized", c""),
    options_table_hook!(c"client-session-changed", c""),
    options_table_hook!(c"command-error", c""),
    options_table_pane_hook!(c"pane-died", c""),
    options_table_pane_hook!(c"pane-exited", c""),
    options_table_pane_hook!(c"pane-focus-in", c""),
    options_table_pane_hook!(c"pane-focus-out", c""),
    options_table_pane_hook!(c"pane-mode-changed", c""),
    options_table_pane_hook!(c"pane-set-clipboard", c""),
    options_table_pane_hook!(c"pane-title-changed", c""),
    options_table_hook!(c"session-closed", c""),
    options_table_hook!(c"session-created", c""),
    options_table_hook!(c"session-renamed", c""),
    options_table_hook!(c"session-window-changed", c""),
    options_table_window_hook!(c"window-layout-changed", c""),
    options_table_hook!(c"window-linked", c""),
    options_table_window_hook!(c"window-pane-changed", c""),
    options_table_window_hook!(c"window-renamed", c""),
    options_table_window_hook!(c"window-resized", c""),
    options_table_hook!(c"window-unlinked", c""),

    options_table_entry {
        name: null(),
        ..unsafe { zeroed() }
    },

];
