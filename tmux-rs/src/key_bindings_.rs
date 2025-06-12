// Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
// IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
// OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

use crate::*;

use libc::strcmp;

use crate::compat::{
    RB_GENERATE_STATIC,
    queue::tailq_foreach,
    tree::{
        rb_empty, rb_find, rb_foreach, rb_init, rb_initializer, rb_insert, rb_min, rb_next,
        rb_remove,
    },
};
use crate::log::fatalx_c;

macro_rules! DEFAULT_SESSION_MENU {
    () => {
        concat!(
            " 'Next' 'n' {switch-client -n}",
            " 'Previous' 'p' {switch-client -p}",
            " ''",
            " 'Renumber' 'N' {move-window -r}",
            " 'Rename' 'n' {command-prompt -I \"#S\" {rename-session -- '%%'}}",
            " ''",
            " 'New Session' 's' {new-session}",
            " 'New Window' 'w' {new-window}"
        )
    };
}

macro_rules! DEFAULT_WINDOW_MENU {
    () => {
        concat!(
            " '#{?#{>:#{session_windows},1},,-}Swap Left' 'l' {swap-window -t:-1}",
            " '#{?#{>:#{session_windows},1},,-}Swap Right' 'r' {swap-window -t:+1}",
            " '#{?pane_marked_set,,-}Swap Marked' 's' {swap-window}",
            " ''",
            " 'Kill' 'X' {kill-window}",
            " 'Respawn' 'R' {respawn-window -k}",
            " '#{?pane_marked,Unmark,Mark}' 'm' {select-pane -m}",
            " 'Rename' 'n' {command-prompt -FI \"#W\" {rename-window -t '#{window_id}' -- '%%'}}",
            " ''",
            " 'New After' 'w' {new-window -a}",
            " 'New At End' 'W' {new-window}"
        )
    };
}

macro_rules! DEFAULT_PANE_MENU {
    () => {
        concat!(
            " '#{?#{m/r:(copy|view)-mode,#{pane_mode}},Go To Top,}' '<' {send -X history-top}",
            " '#{?#{m/r:(copy|view)-mode,#{pane_mode}},Go To Bottom,}' '>' {send -X history-bottom}",
            " ''",
            " '#{?mouse_word,Search For #[underscore]#{=/9/...:mouse_word},}' 'C-r' {if -F '#{?#{m/r:(copy|view)-mode,#{pane_mode}},0,1}' 'copy-mode -t='; send -Xt= search-backward \"#{q:mouse_word}\"}",
            " '#{?mouse_word,Type #[underscore]#{=/9/...:mouse_word},}' 'C-y' {copy-mode -q; send-keys -l -- \"#{q:mouse_word}\"}",
            " '#{?mouse_word,Copy #[underscore]#{=/9/...:mouse_word},}' 'c' {copy-mode -q; set-buffer -- \"#{q:mouse_word}\"}",
            " '#{?mouse_line,Copy Line,}' 'l' {copy-mode -q; set-buffer -- \"#{q:mouse_line}\"}",
            " ''",
            " '#{?mouse_hyperlink,Type #[underscore]#{=/9/...:mouse_hyperlink},}' 'C-h' {copy-mode -q; send-keys -l -- \"#{q:mouse_hyperlink}\"}",
            " '#{?mouse_hyperlink,Copy #[underscore]#{=/9/...:mouse_hyperlink},}' 'h' {copy-mode -q; set-buffer -- \"#{q:mouse_hyperlink}\"}",
            " ''",
            " 'Horizontal Split' 'h' {split-window -h}",
            " 'Vertical Split' 'v' {split-window -v}",
            " ''",
            " '#{?#{>:#{window_panes},1},,-}Swap Up' 'u' {swap-pane -U}",
            " '#{?#{>:#{window_panes},1},,-}Swap Down' 'd' {swap-pane -D}",
            " '#{?pane_marked_set,,-}Swap Marked' 's' {swap-pane}",
            " ''",
            " 'Kill' 'X' {kill-pane}",
            " 'Respawn' 'R' {respawn-pane -k}",
            " '#{?pane_marked,Unmark,Mark}' 'm' {select-pane -m}",
            " '#{?#{>:#{window_panes},1},,-}#{?window_zoomed_flag,Unzoom,Zoom}' 'z' {resize-pane -Z}",
        )
    };
}

RB_GENERATE_STATIC!(key_bindings, key_binding, entry, key_bindings_cmp);
RB_GENERATE_STATIC!(key_tables, key_table, entry, key_table_cmp);
static mut key_tables: key_tables = rb_initializer();

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_table_cmp(table1: *const key_table, table2: *const key_table) -> i32 {
    unsafe { strcmp((*table1).name, (*table2).name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_cmp(bd1: *const key_binding, bd2: *const key_binding) -> i32 {
    unsafe {
        if (*bd1).key < (*bd2).key {
            return (-1);
        }
        if (*bd1).key > (*bd2).key {
            return (1);
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_free(bd: *mut key_binding) {
    unsafe {
        cmd_list_free((*bd).cmdlist);
        free_((*bd).note);
        free_(bd);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_get_table(
    name: *const c_char,
    create: i32,
) -> *mut key_table {
    unsafe {
        let mut table_find = MaybeUninit::<key_table>::uninit();
        let table_find = table_find.as_mut_ptr();
        // struct key_table	table_find, *table;

        (*table_find).name = name.cast_mut();
        let table = rb_find(&raw mut key_tables, table_find);
        if !table.is_null() || create == 0 {
            return table;
        }

        let table = xmalloc_::<key_table>().as_ptr();
        (*table).name = xstrdup(name).as_ptr();
        rb_init(&raw mut (*table).key_bindings);
        rb_init(&raw mut (*table).default_key_bindings);

        (*table).references = 1; /* one reference in key_tables */
        rb_insert(&raw mut key_tables, table);

        table
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_first_table() -> *mut key_table {
    unsafe { rb_min(&raw mut key_tables) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_next_table(table: *mut key_table) -> *mut key_table {
    unsafe { rb_next(table) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_unref_table(table: *mut key_table) {
    unsafe {
        (*table).references -= 1;
        if (*table).references != 0 {
            return;
        }

        for bd in rb_foreach(&raw mut (*table).key_bindings).map(NonNull::as_ptr) {
            rb_remove(&raw mut (*table).key_bindings, bd);
            key_bindings_free(bd);
        }
        for bd in rb_foreach(&raw mut (*table).default_key_bindings).map(NonNull::as_ptr) {
            rb_remove(&raw mut (*table).default_key_bindings, bd);
            key_bindings_free(bd);
        }

        free_((*table).name);
        free_(table);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_get(
    table: NonNull<key_table>,
    key: key_code,
) -> *mut key_binding {
    unsafe {
        let mut bd = MaybeUninit::<key_binding>::uninit();
        let bd = bd.as_mut_ptr();

        (*bd).key = key;
        rb_find(&raw mut (*table.as_ptr()).key_bindings, bd)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_get_default(
    table: *mut key_table,
    key: key_code,
) -> *mut key_binding {
    unsafe {
        let mut bd = MaybeUninit::<key_binding>::uninit();
        let bd = bd.as_mut_ptr();

        (*bd).key = key;
        rb_find(&raw mut (*table).default_key_bindings, bd)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_first(table: *mut key_table) -> *mut key_binding {
    unsafe { rb_min(&raw mut (*table).key_bindings) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_next(
    _table: *mut key_table,
    bd: *mut key_binding,
) -> *mut key_binding {
    unsafe { rb_next(bd) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_add(
    name: *const c_char,
    key: key_code,
    note: *const c_char,
    repeat: i32,
    cmdlist: *mut cmd_list,
) {
    unsafe {
        let table = key_bindings_get_table(name, 1);

        let mut bd = key_bindings_get(NonNull::new(table).unwrap(), key & !KEYC_MASK_FLAGS);
        if (cmdlist.is_null()) {
            if (!bd.is_null()) {
                free_((*bd).note);
                if (!note.is_null()) {
                    (*bd).note = xstrdup(note).as_ptr();
                } else {
                    (*bd).note = null_mut();
                }
            }
            return;
        }
        if (!bd.is_null()) {
            rb_remove(&raw mut (*table).key_bindings, bd);
            key_bindings_free(bd);
        }

        bd = xcalloc1::<key_binding>();
        (*bd).key = (key & !KEYC_MASK_FLAGS);
        if !note.is_null() {
            (*bd).note = xstrdup(note).as_ptr();
        }
        rb_insert(&raw mut (*table).key_bindings, bd);

        if repeat != 0 {
            (*bd).flags |= KEY_BINDING_REPEAT;
        }
        (*bd).cmdlist = cmdlist;

        let s = cmd_list_print((*bd).cmdlist, 0);
        log_debug!(
            "{}: {:#x} {} = {}",
            "key_bindings_add",
            (*bd).key,
            _s(key_string_lookup_key((*bd).key, 1)),
            _s(s),
        );
        free_(s);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_remove(name: *const c_char, key: key_code) {
    unsafe {
        let Some(table) = NonNull::new(key_bindings_get_table(name, 0)) else {
            return;
        };

        let bd = key_bindings_get(table, key & !KEYC_MASK_FLAGS);
        if bd.is_null() {
            return;
        }

        log_debug!(
            "{}: {:#x} {}",
            "key_bindings_remove",
            (*bd).key,
            _s(key_string_lookup_key((*bd).key, 1)),
        );

        rb_remove(&raw mut (*table.as_ptr()).key_bindings, bd);
        key_bindings_free(bd);

        if (rb_empty(&raw mut (*table.as_ptr()).key_bindings)
            && rb_empty(&raw mut (*table.as_ptr()).default_key_bindings))
        {
            rb_remove(&raw mut key_tables, table.as_ptr());
            key_bindings_unref_table(table.as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_reset(name: *const c_char, key: key_code) {
    unsafe {
        let Some(table) = NonNull::new(key_bindings_get_table(name, 0)) else {
            return;
        };

        let bd = key_bindings_get(table, key & !KEYC_MASK_FLAGS);
        if bd.is_null() {
            return;
        }

        let dd = key_bindings_get_default(table.as_ptr(), (*bd).key);
        if (dd.is_null()) {
            key_bindings_remove(name, (*bd).key);
            return;
        }

        cmd_list_free((*bd).cmdlist);
        (*bd).cmdlist = (*dd).cmdlist;
        (*(*bd).cmdlist).references += 1;

        free_((*bd).note);
        if !(*dd).note.is_null() {
            (*bd).note = xstrdup((*dd).note).as_ptr();
        } else {
            (*bd).note = null_mut();
        }
        (*bd).flags = (*dd).flags;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_remove_table(name: *const c_char) {
    unsafe {
        let table = key_bindings_get_table(name, 0);
        if (!table.is_null()) {
            rb_remove(&raw mut key_tables, table);
            key_bindings_unref_table(table);
        }
        for c in crate::compat::queue::tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if (*c).keytable == table {
                server_client_set_key_table(c, null_mut());
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_reset_table(name: *const c_char) {
    unsafe {
        let table = key_bindings_get_table(name, 0);
        if table.is_null() {
            return;
        }
        if (rb_empty(&raw mut (*table).default_key_bindings)) {
            key_bindings_remove_table(name);
            return;
        }
        for bd in rb_foreach(&raw mut (*table).key_bindings).map(NonNull::as_ptr) {
            key_bindings_reset(name, (*bd).key);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_init_done(
    _item: *mut cmdq_item,
    data: *mut c_void,
) -> cmd_retval {
    unsafe {
        for table in rb_foreach(&raw mut key_tables).map(NonNull::as_ptr) {
            for bd in rb_foreach(&raw mut (*table).key_bindings).map(NonNull::as_ptr) {
                let new_bd = xcalloc1::<key_binding>();
                new_bd.key = (*bd).key;
                if !(*bd).note.is_null() {
                    new_bd.note = xstrdup((*bd).note).as_ptr();
                }
                new_bd.flags = (*bd).flags;
                new_bd.cmdlist = (*bd).cmdlist;
                (*new_bd.cmdlist).references += 1;
                rb_insert(&raw mut (*table).default_key_bindings, new_bd);
            }
        }
    }

    cmd_retval::CMD_RETURN_NORMAL
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_init() {
    #[rustfmt::skip]
    static mut defaults: [*const c_char; 262] = [
        /* Prefix keys. */
        c"bind -N 'Send the prefix key' C-b { send-prefix }".as_ptr(),
        c"bind -N 'Rotate through the panes' C-o { rotate-window }".as_ptr(),
        c"bind -N 'Suspend the current client' C-z { suspend-client }".as_ptr(),
        c"bind -N 'Select next layout' Space { next-layout }".as_ptr(),
        c"bind -N 'Break pane to a new window' ! { break-pane }".as_ptr(),
        c"bind -N 'Split window vertically' '\"' { split-window }".as_ptr(),
        c"bind -N 'List all paste buffers' '#' { list-buffers }".as_ptr(),
        c"bind -N 'Rename current session' '$' { command-prompt -I'#S' { rename-session -- '%%' } }".as_ptr(),
        c"bind -N 'Split window horizontally' % { split-window -h }".as_ptr(),
        c"bind -N 'Kill current window' & { confirm-before -p\"kill-window #W? (y/n)\" kill-window }".as_ptr(),
        c"bind -N 'Prompt for window index to select' \"'\" { command-prompt -T window-target -pindex { select-window -t ':%%' } }".as_ptr(),
        c"bind -N 'Switch to previous client' ( { switch-client -p }".as_ptr(),
        c"bind -N 'Switch to next client' ) { switch-client -n }".as_ptr(),
        c"bind -N 'Rename current window' , { command-prompt -I'#W' { rename-window -- '%%' } }".as_ptr(),
        c"bind -N 'Delete the most recent paste buffer' - { delete-buffer }".as_ptr(),
        c"bind -N 'Move the current window' . { command-prompt -T target { move-window -t '%%' } }".as_ptr(),
        c"bind -N 'Describe key binding' '/' { command-prompt -kpkey  { list-keys -1N '%%' } }".as_ptr(),
        c"bind -N 'Select window 0' 0 { select-window -t:=0 }".as_ptr(),
        c"bind -N 'Select window 1' 1 { select-window -t:=1 }".as_ptr(),
        c"bind -N 'Select window 2' 2 { select-window -t:=2 }".as_ptr(),
        c"bind -N 'Select window 3' 3 { select-window -t:=3 }".as_ptr(),
        c"bind -N 'Select window 4' 4 { select-window -t:=4 }".as_ptr(),
        c"bind -N 'Select window 5' 5 { select-window -t:=5 }".as_ptr(),
        c"bind -N 'Select window 6' 6 { select-window -t:=6 }".as_ptr(),
        c"bind -N 'Select window 7' 7 { select-window -t:=7 }".as_ptr(),
        c"bind -N 'Select window 8' 8 { select-window -t:=8 }".as_ptr(),
        c"bind -N 'Select window 9' 9 { select-window -t:=9 }".as_ptr(),
        c"bind -N 'Prompt for a command' : { command-prompt }".as_ptr(),
        c"bind -N 'Move to the previously active pane' \\; { last-pane }".as_ptr(),
        c"bind -N 'Choose a paste buffer from a list' = { choose-buffer -Z }".as_ptr(),
        c"bind -N 'List key bindings' ? { list-keys -N }".as_ptr(),
        c"bind -N 'Choose and detach a client from a list' D { choose-client -Z }".as_ptr(),
        c"bind -N 'Spread panes out evenly' E { select-layout -E }".as_ptr(),
        c"bind -N 'Switch to the last client' L { switch-client -l }".as_ptr(),
        c"bind -N 'Clear the marked pane' M { select-pane -M }".as_ptr(),
        c"bind -N 'Enter copy mode' [ { copy-mode }".as_ptr(),
        c"bind -N 'Paste the most recent paste buffer' ] { paste-buffer -p }".as_ptr(),
        c"bind -N 'Create a new window' c { new-window }".as_ptr(),
        c"bind -N 'Detach the current client' d { detach-client }".as_ptr(),
        c"bind -N 'Search for a pane' f { command-prompt { find-window -Z -- '%%' } }".as_ptr(),
        c"bind -N 'Display window information' i { display-message }".as_ptr(),
        c"bind -N 'Select the previously current window' l { last-window }".as_ptr(),
        c"bind -N 'Toggle the marked pane' m { select-pane -m }".as_ptr(),
        c"bind -N 'Select the next window' n { next-window }".as_ptr(),
        c"bind -N 'Select the next pane' o { select-pane -t:.+ }".as_ptr(),
        c"bind -N 'Customize options' C { customize-mode -Z }".as_ptr(),
        c"bind -N 'Select the previous window' p { previous-window }".as_ptr(),
        c"bind -N 'Display pane numbers' q { display-panes }".as_ptr(),
        c"bind -N 'Redraw the current client' r { refresh-client }".as_ptr(),
        c"bind -N 'Choose a session from a list' s { choose-tree -Zs }".as_ptr(),
        c"bind -N 'Show a clock' t { clock-mode }".as_ptr(),
        c"bind -N 'Choose a window from a list' w { choose-tree -Zw }".as_ptr(),
        c"bind -N 'Kill the active pane' x { confirm-before -p\"kill-pane #P? (y/n)\" kill-pane }".as_ptr(),
        c"bind -N 'Zoom the active pane' z { resize-pane -Z }".as_ptr(),
        c"bind -N 'Swap the active pane with the pane above' '{' { swap-pane -U }".as_ptr(),
        c"bind -N 'Swap the active pane with the pane below' '}' { swap-pane -D }".as_ptr(),
        c"bind -N 'Show messages' '~' { show-messages }".as_ptr(),
        c"bind -N 'Enter copy mode and scroll up' PPage { copy-mode -u }".as_ptr(),
        c"bind -N 'Select the pane above the active pane' -r Up { select-pane -U }".as_ptr(),
        c"bind -N 'Select the pane below the active pane' -r Down { select-pane -D }".as_ptr(),
        c"bind -N 'Select the pane to the left of the active pane' -r Left { select-pane -L }".as_ptr(),
        c"bind -N 'Select the pane to the right of the active pane' -r Right { select-pane -R }".as_ptr(),
        c"bind -N 'Set the even-horizontal layout' M-1 { select-layout even-horizontal }".as_ptr(),
        c"bind -N 'Set the even-vertical layout' M-2 { select-layout even-vertical }".as_ptr(),
        c"bind -N 'Set the main-horizontal layout' M-3 { select-layout main-horizontal }".as_ptr(),
        c"bind -N 'Set the main-vertical layout' M-4 { select-layout main-vertical }".as_ptr(),
        c"bind -N 'Select the tiled layout' M-5 { select-layout tiled }".as_ptr(),
        c"bind -N 'Set the main-horizontal-mirrored layout' M-6 { select-layout main-horizontal-mirrored }".as_ptr(),
        c"bind -N 'Set the main-vertical-mirrored layout' M-7 { select-layout main-vertical-mirrored }".as_ptr(),
        c"bind -N 'Select the next window with an alert' M-n { next-window -a }".as_ptr(),
        c"bind -N 'Rotate through the panes in reverse' M-o { rotate-window -D }".as_ptr(),
        c"bind -N 'Select the previous window with an alert' M-p { previous-window -a }".as_ptr(),
        c"bind -N 'Move the visible part of the window up' -r S-Up { refresh-client -U 10 }".as_ptr(),
        c"bind -N 'Move the visible part of the window down' -r S-Down { refresh-client -D 10 }".as_ptr(),
        c"bind -N 'Move the visible part of the window left' -r S-Left { refresh-client -L 10 }".as_ptr(),
        c"bind -N 'Move the visible part of the window right' -r S-Right { refresh-client -R 10 }".as_ptr(),
        c"bind -N 'Reset so the visible part of the window follows the cursor' -r DC { refresh-client -c }".as_ptr(),
        c"bind -N 'Resize the pane up by 5' -r M-Up { resize-pane -U 5 }".as_ptr(),
        c"bind -N 'Resize the pane down by 5' -r M-Down { resize-pane -D 5 }".as_ptr(),
        c"bind -N 'Resize the pane left by 5' -r M-Left { resize-pane -L 5 }".as_ptr(),
        c"bind -N 'Resize the pane right by 5' -r M-Right { resize-pane -R 5 }".as_ptr(),
        c"bind -N 'Resize the pane up' -r C-Up { resize-pane -U }".as_ptr(),
        c"bind -N 'Resize the pane down' -r C-Down { resize-pane -D }".as_ptr(),
        c"bind -N 'Resize the pane left' -r C-Left { resize-pane -L }".as_ptr(),
        c"bind -N 'Resize the pane right' -r C-Right { resize-pane -R }".as_ptr(),
        /* Menu keys */
        concat!( "bind < { display-menu -xW -yW -T '#[align=centre]#{window_index}:#{window_name}' ", DEFAULT_WINDOW_MENU!(), " }\0").as_ptr().cast(),
        concat!( "bind > { display-menu -xP -yP -T '#[align=centre]#{pane_index} ", "(#{pane_id})' ", DEFAULT_PANE_MENU!(), " }\0").as_ptr().cast(),
        // Mouse button 1 down on pane.
        c"bind -n MouseDown1Pane { select-pane -t=; send -M }".as_ptr(),
        /* Mouse button 1 drag on pane. */
        c"bind -n MouseDrag1Pane { if -F '#{||:#{pane_in_mode},#{mouse_any_flag}}' { send -M } { copy-mode -M } }".as_ptr(),
        /* Mouse wheel up on pane. */
        c"bind -n WheelUpPane { if -F '#{||:#{pane_in_mode},#{mouse_any_flag}}' { send -M } { copy-mode -e } }".as_ptr(),
        /* Mouse button 2 down on pane. */
        c"bind -n MouseDown2Pane { select-pane -t=; if -F '#{||:#{pane_in_mode},#{mouse_any_flag}}' { send -M } { paste -p } }".as_ptr(),
        /* Mouse button 1 double click on pane. */
        c"bind -n DoubleClick1Pane { select-pane -t=; if -F '#{||:#{pane_in_mode},#{mouse_any_flag}}' { send -M } { copy-mode -H; send -X select-word; run -d0.3; send -X copy-pipe-and-cancel } }".as_ptr(),
        /* Mouse button 1 triple click on pane. */
        c"bind -n TripleClick1Pane { select-pane -t=; if -F '#{||:#{pane_in_mode},#{mouse_any_flag}}' { send -M } { copy-mode -H; send -X select-line; run -d0.3; send -X copy-pipe-and-cancel } }".as_ptr(),
        /* Mouse button 1 drag on border. */
        c"bind -n MouseDrag1Border { resize-pane -M }".as_ptr(),
        /* Mouse button 1 down on status line. */
        c"bind -n MouseDown1Status { select-window -t= }".as_ptr(),
        /* Mouse wheel down on status line. */
        c"bind -n WheelDownStatus { next-window }".as_ptr(),
        /* Mouse wheel up on status line. */
        c"bind -n WheelUpStatus { previous-window }".as_ptr(),
        /* Mouse button 3 down on status left. */
        concat!("bind -n MouseDown3StatusLeft { display-menu -t= -xM -yW -T '#[align=centre]#{session_name}' ", DEFAULT_SESSION_MENU!(), " }\0").as_ptr().cast(),
        concat!("bind -n M-MouseDown3StatusLeft { display-menu -t= -xM -yW -T '#[align=centre]#{session_name}' ", DEFAULT_SESSION_MENU!(), " }\0").as_ptr().cast(),
        /* Mouse button 3 down on status line. */
        concat!( "bind -n MouseDown3Status { display-menu -t= -xW -yW -T '#[align=centre]#{window_index}:#{window_name}' ", DEFAULT_WINDOW_MENU!(), "}\0").as_ptr().cast(),
        concat!( "bind -n M-MouseDown3Status { display-menu -t= -xW -yW -T '#[align=centre]#{window_index}:#{window_name}' ", DEFAULT_WINDOW_MENU!(), "}\0").as_ptr().cast(),
        /* Mouse button 3 down on pane. */
        concat!( "bind -n MouseDown3Pane { if -Ft= '#{||:#{mouse_any_flag},#{&&:#{pane_in_mode},#{?#{m/r:(copy|view)-mode,#{pane_mode}},0,1}}}' { select-pane -t=; send -M } { display-menu -t= -xM -yM -T '#[align=centre]#{pane_index} (#{pane_id})' ", DEFAULT_PANE_MENU!(), " } }\0").as_ptr().cast(),
        concat!( "bind -n M-MouseDown3Pane { display-menu -t= -xM -yM -T '#[align=centre]#{pane_index} (#{pane_id})' ", DEFAULT_PANE_MENU!(), " }\0").as_ptr().cast(),
        /* Copy mode (emacs) keys. */
        c"bind -Tcopy-mode C-Space { send -X begin-selection }".as_ptr(),
        c"bind -Tcopy-mode C-a { send -X start-of-line }".as_ptr(),
        c"bind -Tcopy-mode C-c { send -X cancel }".as_ptr(),
        c"bind -Tcopy-mode C-e { send -X end-of-line }".as_ptr(),
        c"bind -Tcopy-mode C-f { send -X cursor-right }".as_ptr(),
        c"bind -Tcopy-mode C-b { send -X cursor-left }".as_ptr(),
        c"bind -Tcopy-mode C-g { send -X clear-selection }".as_ptr(),
        c"bind -Tcopy-mode C-k { send -X copy-pipe-end-of-line-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode C-n { send -X cursor-down }".as_ptr(),
        c"bind -Tcopy-mode C-p { send -X cursor-up }".as_ptr(),
        c"bind -Tcopy-mode C-r { command-prompt -T search -ip'(search up)' -I'#{pane_search_string}' { send -X search-backward-incremental '%%' } }".as_ptr(),
        c"bind -Tcopy-mode C-s { command-prompt -T search -ip'(search down)' -I'#{pane_search_string}' { send -X search-forward-incremental '%%' } }".as_ptr(),
        c"bind -Tcopy-mode C-v { send -X page-down }".as_ptr(),
        c"bind -Tcopy-mode C-w { send -X copy-pipe-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode Escape { send -X cancel }".as_ptr(),
        c"bind -Tcopy-mode Space { send -X page-down }".as_ptr(),
        c"bind -Tcopy-mode , { send -X jump-reverse }".as_ptr(),
        c"bind -Tcopy-mode \\; { send -X jump-again }".as_ptr(),
        c"bind -Tcopy-mode F { command-prompt -1p'(jump backward)' { send -X jump-backward '%%' } }".as_ptr(),
        c"bind -Tcopy-mode N { send -X search-reverse }".as_ptr(),
        c"bind -Tcopy-mode P { send -X toggle-position }".as_ptr(),
        c"bind -Tcopy-mode R { send -X rectangle-toggle }".as_ptr(),
        c"bind -Tcopy-mode T { command-prompt -1p'(jump to backward)' { send -X jump-to-backward '%%' } }".as_ptr(),
        c"bind -Tcopy-mode X { send -X set-mark }".as_ptr(),
        c"bind -Tcopy-mode f { command-prompt -1p'(jump forward)' { send -X jump-forward '%%' } }".as_ptr(),
        c"bind -Tcopy-mode g { command-prompt -p'(goto line)' { send -X goto-line '%%' } }".as_ptr(),
        c"bind -Tcopy-mode n { send -X search-again }".as_ptr(),
        c"bind -Tcopy-mode q { send -X cancel }".as_ptr(),
        c"bind -Tcopy-mode r { send -X refresh-from-pane }".as_ptr(),
        c"bind -Tcopy-mode t { command-prompt -1p'(jump to forward)' { send -X jump-to-forward '%%' } }".as_ptr(),
        c"bind -Tcopy-mode Home { send -X start-of-line }".as_ptr(),
        c"bind -Tcopy-mode End { send -X end-of-line }".as_ptr(),
        c"bind -Tcopy-mode MouseDown1Pane select-pane".as_ptr(),
        c"bind -Tcopy-mode MouseDrag1Pane { select-pane; send -X begin-selection }".as_ptr(),
        c"bind -Tcopy-mode MouseDragEnd1Pane { send -X copy-pipe-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode WheelUpPane { select-pane; send -N5 -X scroll-up }".as_ptr(),
        c"bind -Tcopy-mode WheelDownPane { select-pane; send -N5 -X scroll-down }".as_ptr(),
        c"bind -Tcopy-mode DoubleClick1Pane { select-pane; send -X select-word; run -d0.3; send -X copy-pipe-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode TripleClick1Pane { select-pane; send -X select-line; run -d0.3; send -X copy-pipe-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode NPage { send -X page-down }".as_ptr(),
        c"bind -Tcopy-mode PPage { send -X page-up }".as_ptr(),
        c"bind -Tcopy-mode Up { send -X cursor-up }".as_ptr(),
        c"bind -Tcopy-mode Down { send -X cursor-down }".as_ptr(),
        c"bind -Tcopy-mode Left { send -X cursor-left }".as_ptr(),
        c"bind -Tcopy-mode Right { send -X cursor-right }".as_ptr(),
        c"bind -Tcopy-mode M-1 { command-prompt -Np'(repeat)' -I1 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode M-2 { command-prompt -Np'(repeat)' -I2 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode M-3 { command-prompt -Np'(repeat)' -I3 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode M-4 { command-prompt -Np'(repeat)' -I4 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode M-5 { command-prompt -Np'(repeat)' -I5 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode M-6 { command-prompt -Np'(repeat)' -I6 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode M-7 { command-prompt -Np'(repeat)' -I7 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode M-8 { command-prompt -Np'(repeat)' -I8 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode M-9 { command-prompt -Np'(repeat)' -I9 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode M-< { send -X history-top }".as_ptr(),
        c"bind -Tcopy-mode M-> { send -X history-bottom }".as_ptr(),
        c"bind -Tcopy-mode M-R { send -X top-line }".as_ptr(),
        c"bind -Tcopy-mode M-b { send -X previous-word }".as_ptr(),
        c"bind -Tcopy-mode C-M-b { send -X previous-matching-bracket }".as_ptr(),
        c"bind -Tcopy-mode M-f { send -X next-word-end }".as_ptr(),
        c"bind -Tcopy-mode C-M-f { send -X next-matching-bracket }".as_ptr(),
        c"bind -Tcopy-mode M-m { send -X back-to-indentation }".as_ptr(),
        c"bind -Tcopy-mode M-r { send -X middle-line }".as_ptr(),
        c"bind -Tcopy-mode M-v { send -X page-up }".as_ptr(),
        c"bind -Tcopy-mode M-w { send -X copy-pipe-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode M-x { send -X jump-to-mark }".as_ptr(),
        c"bind -Tcopy-mode 'M-{' { send -X previous-paragraph }".as_ptr(),
        c"bind -Tcopy-mode 'M-}' { send -X next-paragraph }".as_ptr(),
        c"bind -Tcopy-mode M-Up { send -X halfpage-up }".as_ptr(),
        c"bind -Tcopy-mode M-Down { send -X halfpage-down }".as_ptr(),
        c"bind -Tcopy-mode C-Up { send -X scroll-up }".as_ptr(),
        c"bind -Tcopy-mode C-Down { send -X scroll-down }".as_ptr(),
        /* Copy mode (vi) keys. */
        c"bind -Tcopy-mode-vi '#' { send -FX search-backward '#{copy_cursor_word}' }".as_ptr(),
        c"bind -Tcopy-mode-vi * { send -FX search-forward '#{copy_cursor_word}' }".as_ptr(),
        c"bind -Tcopy-mode-vi C-c { send -X cancel }".as_ptr(),
        c"bind -Tcopy-mode-vi C-d { send -X halfpage-down }".as_ptr(),
        c"bind -Tcopy-mode-vi C-e { send -X scroll-down }".as_ptr(),
        c"bind -Tcopy-mode-vi C-b { send -X page-up }".as_ptr(),
        c"bind -Tcopy-mode-vi C-f { send -X page-down }".as_ptr(),
        c"bind -Tcopy-mode-vi C-h { send -X cursor-left }".as_ptr(),
        c"bind -Tcopy-mode-vi C-j { send -X copy-pipe-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode-vi Enter { send -X copy-pipe-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode-vi C-u { send -X halfpage-up }".as_ptr(),
        c"bind -Tcopy-mode-vi C-v { send -X rectangle-toggle }".as_ptr(),
        c"bind -Tcopy-mode-vi C-y { send -X scroll-up }".as_ptr(),
        c"bind -Tcopy-mode-vi Escape { send -X clear-selection }".as_ptr(),
        c"bind -Tcopy-mode-vi Space { send -X begin-selection }".as_ptr(),
        c"bind -Tcopy-mode-vi '$' { send -X end-of-line }".as_ptr(),
        c"bind -Tcopy-mode-vi , { send -X jump-reverse }".as_ptr(),
        c"bind -Tcopy-mode-vi / { command-prompt -T search -p'(search down)' { send -X search-forward '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi 0 { send -X start-of-line }".as_ptr(),
        c"bind -Tcopy-mode-vi 1 { command-prompt -Np'(repeat)' -I1 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi 2 { command-prompt -Np'(repeat)' -I2 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi 3 { command-prompt -Np'(repeat)' -I3 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi 4 { command-prompt -Np'(repeat)' -I4 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi 5 { command-prompt -Np'(repeat)' -I5 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi 6 { command-prompt -Np'(repeat)' -I6 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi 7 { command-prompt -Np'(repeat)' -I7 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi 8 { command-prompt -Np'(repeat)' -I8 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi 9 { command-prompt -Np'(repeat)' -I9 { send -N '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi : { command-prompt -p'(goto line)' { send -X goto-line '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi \\; { send -X jump-again }".as_ptr(),
        c"bind -Tcopy-mode-vi ? { command-prompt -T search -p'(search up)' { send -X search-backward '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi A { send -X append-selection-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode-vi B { send -X previous-space }".as_ptr(),
        c"bind -Tcopy-mode-vi D { send -X copy-pipe-end-of-line-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode-vi E { send -X next-space-end }".as_ptr(),
        c"bind -Tcopy-mode-vi F { command-prompt -1p'(jump backward)' { send -X jump-backward '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi G { send -X history-bottom }".as_ptr(),
        c"bind -Tcopy-mode-vi H { send -X top-line }".as_ptr(),
        c"bind -Tcopy-mode-vi J { send -X scroll-down }".as_ptr(),
        c"bind -Tcopy-mode-vi K { send -X scroll-up }".as_ptr(),
        c"bind -Tcopy-mode-vi L { send -X bottom-line }".as_ptr(),
        c"bind -Tcopy-mode-vi M { send -X middle-line }".as_ptr(),
        c"bind -Tcopy-mode-vi N { send -X search-reverse }".as_ptr(),
        c"bind -Tcopy-mode-vi P { send -X toggle-position }".as_ptr(),
        c"bind -Tcopy-mode-vi T { command-prompt -1p'(jump to backward)' { send -X jump-to-backward '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi V { send -X select-line }".as_ptr(),
        c"bind -Tcopy-mode-vi W { send -X next-space }".as_ptr(),
        c"bind -Tcopy-mode-vi X { send -X set-mark }".as_ptr(),
        c"bind -Tcopy-mode-vi ^ { send -X back-to-indentation }".as_ptr(),
        c"bind -Tcopy-mode-vi b { send -X previous-word }".as_ptr(),
        c"bind -Tcopy-mode-vi e { send -X next-word-end }".as_ptr(),
        c"bind -Tcopy-mode-vi f { command-prompt -1p'(jump forward)' { send -X jump-forward '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi g { send -X history-top }".as_ptr(),
        c"bind -Tcopy-mode-vi h { send -X cursor-left }".as_ptr(),
        c"bind -Tcopy-mode-vi j { send -X cursor-down }".as_ptr(),
        c"bind -Tcopy-mode-vi k { send -X cursor-up }".as_ptr(),
        c"bind -Tcopy-mode-vi z { send -X scroll-middle }".as_ptr(),
        c"bind -Tcopy-mode-vi l { send -X cursor-right }".as_ptr(),
        c"bind -Tcopy-mode-vi n { send -X search-again }".as_ptr(),
        c"bind -Tcopy-mode-vi o { send -X other-end }".as_ptr(),
        c"bind -Tcopy-mode-vi q { send -X cancel }".as_ptr(),
        c"bind -Tcopy-mode-vi r { send -X refresh-from-pane }".as_ptr(),
        c"bind -Tcopy-mode-vi t { command-prompt -1p'(jump to forward)' { send -X jump-to-forward '%%' } }".as_ptr(),
        c"bind -Tcopy-mode-vi v { send -X rectangle-toggle }".as_ptr(),
        c"bind -Tcopy-mode-vi w { send -X next-word }".as_ptr(),
        c"bind -Tcopy-mode-vi '{' { send -X previous-paragraph }".as_ptr(),
        c"bind -Tcopy-mode-vi '}' { send -X next-paragraph }".as_ptr(),
        c"bind -Tcopy-mode-vi % { send -X next-matching-bracket }".as_ptr(),
        c"bind -Tcopy-mode-vi Home { send -X start-of-line }".as_ptr(),
        c"bind -Tcopy-mode-vi End { send -X end-of-line }".as_ptr(),
        c"bind -Tcopy-mode-vi MouseDown1Pane { select-pane }".as_ptr(),
        c"bind -Tcopy-mode-vi MouseDrag1Pane { select-pane; send -X begin-selection }".as_ptr(),
        c"bind -Tcopy-mode-vi MouseDragEnd1Pane { send -X copy-pipe-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode-vi WheelUpPane { select-pane; send -N5 -X scroll-up }".as_ptr(),
        c"bind -Tcopy-mode-vi WheelDownPane { select-pane; send -N5 -X scroll-down }".as_ptr(),
        c"bind -Tcopy-mode-vi DoubleClick1Pane { select-pane; send -X select-word; run -d0.3; send -X copy-pipe-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode-vi TripleClick1Pane { select-pane; send -X select-line; run -d0.3; send -X copy-pipe-and-cancel }".as_ptr(),
        c"bind -Tcopy-mode-vi BSpace { send -X cursor-left }".as_ptr(),
        c"bind -Tcopy-mode-vi NPage { send -X page-down }".as_ptr(),
        c"bind -Tcopy-mode-vi PPage { send -X page-up }".as_ptr(),
        c"bind -Tcopy-mode-vi Up { send -X cursor-up }".as_ptr(),
        c"bind -Tcopy-mode-vi Down { send -X cursor-down }".as_ptr(),
        c"bind -Tcopy-mode-vi Left { send -X cursor-left }".as_ptr(),
        c"bind -Tcopy-mode-vi Right { send -X cursor-right }".as_ptr(),
        c"bind -Tcopy-mode-vi M-x { send -X jump-to-mark }".as_ptr(),
        c"bind -Tcopy-mode-vi C-Up { send -X scroll-up }".as_ptr(),
        c"bind -Tcopy-mode-vi C-Down { send -X scroll-down }".as_ptr(),
    ];

    unsafe {
        for default in defaults {
            let pr = cmd_parse_from_string(default, null_mut());
            if (*pr).status != cmd_parse_status::CMD_PARSE_SUCCESS {
                log_debug!("{}", _s((*pr).error));
                fatalx_c(c"bad default key: %s".as_ptr(), default);
            }
            cmdq_append(null_mut(), cmdq_get_command((*pr).cmdlist, null_mut()));
            cmd_list_free((*pr).cmdlist);
        }
        cmdq_append(
            null_mut(),
            cmdq_get_callback!(key_bindings_init_done, null_mut()).as_ptr(),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_read_only(
    item: *mut cmdq_item,
    data: *mut c_void,
) -> cmd_retval {
    unsafe {
        cmdq_error(item, c"client is read-only".as_ptr());
    }
    cmd_retval::CMD_RETURN_ERROR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn key_bindings_dispatch(
    bd: *mut key_binding,
    item: *mut cmdq_item,
    c: *mut client,
    event: *mut key_event,
    fs: *mut cmd_find_state,
) -> *mut cmdq_item {
    unsafe {
        let mut flags = 0;

        let readonly = if c.is_null() || !(*c).flags.intersects(client_flag::READONLY) {
            true
        } else {
            cmd_list_all_have((*bd).cmdlist, cmd_flag::CMD_READONLY).as_bool()
        };

        let mut new_item = null_mut();
        if !readonly {
            new_item = cmdq_get_callback!(key_bindings_read_only, null_mut()).as_ptr();
        } else {
            if (*bd).flags & KEY_BINDING_REPEAT != 0 {
                flags |= CMDQ_STATE_REPEAT;
            }
            let new_state = cmdq_new_state(fs, event, flags);
            new_item = cmdq_get_command((*bd).cmdlist, new_state);
            cmdq_free_state(new_state);
        }
        if (!item.is_null()) {
            new_item = cmdq_insert_after(item, new_item);
        } else {
            new_item = cmdq_append(c, new_item);
        }
        new_item
    }
}
