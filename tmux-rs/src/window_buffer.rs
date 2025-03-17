use compat_rs::{VIS_CSTYLE, VIS_OCTAL, VIS_TAB, queue::tailq_first};
use libc::{memmem, qsort, strcmp, strstr};

use crate::xmalloc::xreallocarray;

use super::*;
unsafe extern "C" {
    // pub static mut window_buffer_mode: window_mode;
}

const WINDOW_BUFFER_DEFAULT_COMMAND: *const i8 = c"paste-buffer -p -b '%%'".as_ptr();
const WINDOW_BUFFER_DEFAULT_FORMAT: *const i8 = c"#{t/p:buffer_created}: #{buffer_sample}".as_ptr();

const WINDOW_BUFFER_DEFAULT_KEY_FORMAT: *const i8 = concat!(
    "#{?#{e|<:#{line},10},",
    "#{line}",
    ",",
    "#{?#{e|<:#{line},36},",
    "M-#{a:#{e|+:97,#{e|-:#{line},10}}}",
    ",",
    "",
    "}",
    "}\0"
)
.as_ptr()
.cast();

static mut window_buffer_menu_items: [menu_item; 12] = [
    menu_item::new(c"Paste".as_ptr(), 'p' as u64, null_mut()),
    menu_item::new(c"Paste Tagged".as_ptr(), 'P' as u64, null_mut()),
    menu_item::new(c"".as_ptr(), KEYC_NONE, null_mut()),
    menu_item::new(c"Tag".as_ptr(), 't' as u64, null_mut()),
    menu_item::new(c"Tag All".as_ptr(), '\x14' as u64, null_mut()),
    menu_item::new(c"Tag None".as_ptr(), 'T' as u64, null_mut()),
    menu_item::new(c"".as_ptr(), KEYC_NONE, null_mut()),
    menu_item::new(c"Delete".as_ptr(), 'd' as u64, null_mut()),
    menu_item::new(c"Delete Tagged".as_ptr(), 'D' as u64, null_mut()),
    menu_item::new(c"".as_ptr(), KEYC_NONE, null_mut()),
    menu_item::new(c"Cancel".as_ptr(), 'q' as u64, null_mut()),
    menu_item::new(null_mut(), KEYC_NONE, null_mut()),
];

#[unsafe(no_mangle)]
pub static mut window_buffer_mode: window_mode = window_mode {
    name: c"buffer-mode".as_ptr(),
    default_format: WINDOW_BUFFER_DEFAULT_FORMAT,

    init: Some(window_buffer_init),
    free: Some(window_buffer_free),
    resize: Some(window_buffer_resize),
    update: Some(window_buffer_update),
    key: Some(window_buffer_key),
    ..unsafe { zeroed() }
};

#[repr(u32)]
enum window_buffer_sort_type {
    WINDOW_BUFFER_BY_TIME,
    WINDOW_BUFFER_BY_NAME,
    WINDOW_BUFFER_BY_SIZE,
}

const window_buffer_sort_list_len: u32 = 3;
static mut window_buffer_sort_list: [SyncCharPtr; 3] = [
    SyncCharPtr::new(c"time"),
    SyncCharPtr::new(c"name"),
    SyncCharPtr::new(c"size"),
];

static mut window_buffer_sort: *mut mode_tree_sort_criteria = null_mut();

pub struct window_buffer_itemdata {
    pub name: *mut c_char,
    pub order: u32,
    pub size: usize,
}

pub struct window_buffer_modedata {
    pub wp: *mut window_pane,
    pub fs: cmd_find_state,

    pub data: *mut mode_tree_data,
    pub command: *mut c_char,
    pub format: *mut c_char,
    pub key_format: *mut c_char,

    pub item_list: *mut *mut window_buffer_itemdata,
    pub item_size: u32,
}

pub struct window_buffer_editdata {
    pub wp_id: u32,
    pub name: *mut c_char,
    pub pb: *mut paste_buffer,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_add_item(data: *mut window_buffer_modedata) -> *mut window_buffer_itemdata {
    unsafe {
        (*data).item_list = xreallocarray_((*data).item_list, (*data).item_size as usize + 1).as_ptr();
        let item = xcalloc1::<window_buffer_itemdata>();
        *(*data).item_list.add((*data).item_size as usize) = item;
        (*data).item_size += 1;
        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_free_item(item: *mut window_buffer_itemdata) {
    unsafe {
        free_((*item).name);
        free_(item);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_cmp(a0: *const c_void, b0: *const c_void) -> i32 {
    unsafe {
        let mut a = a0 as *const *const window_buffer_itemdata;
        let mut b = b0 as *const *const window_buffer_itemdata;
        let mut result = 0i32;

        if ((*window_buffer_sort).field == window_buffer_sort_type::WINDOW_BUFFER_BY_TIME as u32) {
            result = (*(*b)).order as i32 - (*(*a)).order as i32;
        } else if ((*window_buffer_sort).field == window_buffer_sort_type::WINDOW_BUFFER_BY_SIZE as u32) {
            result = ((*(*b)).size as isize - (*(*a)).size as isize) as i32;
        }

        /* Use WINDOW_BUFFER_BY_NAME as default order and tie breaker. */
        if (result == 0) {
            result = strcmp((*(*a)).name, (*(*b)).name);
        }

        if ((*window_buffer_sort).reversed != 0) {
            result = -result;
        }

        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_build(
    modedata: *mut c_void,
    sort_crit: *mut mode_tree_sort_criteria,
    tag: *mut u64,
    filter: *const c_char,
) {
    unsafe {
        let mut data = modedata as *mut window_buffer_modedata;
        let mut item: *mut window_buffer_itemdata = null_mut();
        // char *text, *cp;
        // struct format_tree *ft;
        let mut s = null_mut();
        let mut wl = null_mut();
        let mut wp = null_mut();

        for i in 0..(*data).item_size {
            window_buffer_free_item(*(*data).item_list.add(i as usize));
        }
        free_((*data).item_list);
        (*data).item_list = null_mut();
        (*data).item_size = 0;

        let mut pb = paste_walk(null_mut());
        while let Some(pb_non_null) = NonNull::new(pb) {
            let mut item = window_buffer_add_item(data);
            (*item).name = xstrdup(paste_buffer_name(pb_non_null)).as_ptr();
            paste_buffer_data(pb, &raw mut (*item).size); // I'm sure if we follow alias rules on item.size here, so keep using older function
            (*item).order = paste_buffer_order(pb_non_null);
            pb = paste_walk(pb);
        }

        window_buffer_sort = sort_crit;
        qsort(
            (*data).item_list.cast(),
            (*data).item_size as usize,
            size_of::<*const window_buffer_itemdata>(),
            Some(window_buffer_cmp),
        );

        if cmd_find_valid_state(&raw mut (*data).fs).as_bool() {
            s = (*data).fs.s;
            wl = (*data).fs.wl;
            wp = (*data).fs.wp;
        }

        for i in 0..(*data).item_size {
            item = *(*data).item_list.add(i as usize);

            pb = paste_get_name((*item).name);
            if (pb.is_null()) {
                continue;
            }
            let ft = format_create(null_mut(), null_mut(), FORMAT_NONE, 0);
            format_defaults(ft, null_mut(), s, wl, wp);
            format_defaults_paste_buffer(ft, pb);

            if (!filter.is_null()) {
                let cp = format_expand(ft, filter);
                if (format_true(cp) == 0) {
                    free_(cp);
                    format_free(ft);
                    continue;
                }
                free_(cp);
            }

            let text = format_expand(ft, (*data).format);
            mode_tree_add(
                (*data).data.cast(),
                null_mut(),
                item.cast(),
                (*item).order as u64,
                (*item).name,
                text,
                -1,
            );
            free_(text);

            format_free(ft);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_draw(
    modedata: *mut c_void,
    itemdata: *mut c_void,
    ctx: *mut screen_write_ctx,
    sx: u32,
    sy: u32,
) {
    unsafe {
        let item = itemdata as *mut window_buffer_itemdata;
        let mut cx = (*(*ctx).s).cx;
        let mut cy = (*(*ctx).s).cy;

        let Some(pb) = NonNull::new(paste_get_name((*item).name)) else {
            return;
        };

        let mut psize: usize = 0;
        let mut buf: *mut c_char = null_mut();
        let mut end = paste_buffer_data_(pb, &mut psize);
        let pdata = end;
        for i in 0..sy {
            let start = end;
            while (end != pdata.add(psize) && *end != b'\n' as c_char) {
                end = end.add(1);
            }
            buf = xreallocarray(buf.cast(), 4, end.offset_from(start) as usize + 1)
                .as_ptr()
                .cast();
            utf8_strvis(
                buf,
                start,
                end.offset_from(start) as usize,
                VIS_OCTAL | VIS_CSTYLE | VIS_TAB,
            );
            if (*buf != b'\0' as c_char) {
                screen_write_cursormove(ctx, cx as i32, (cy + i) as i32, 0);
                screen_write_nputs(ctx, sx as isize, &raw const grid_default_cell, c"%s".as_ptr(), buf);
            }

            if end == pdata.add(psize) {
                break;
            }
            end = end.add(1);
        }
        free_(buf);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_search(modedata: *mut c_void, itemdata: *mut c_void, ss: *const c_char) -> i32 {
    unsafe {
        let item = itemdata as *mut window_buffer_itemdata;
        let Some(pb) = NonNull::new(paste_get_name((*item).name)) else {
            return 0;
        };
        if !strstr((*item).name, ss).is_null() {
            return 1;
        }
        let mut bufsize = 0;
        let bufdata = paste_buffer_data_(pb, &mut bufsize);
        !memmem(bufdata.cast(), bufsize, ss.cast(), strlen(ss)).is_null() as i32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_menu(modedata: *mut c_void, c: *mut client, key: key_code) {
    unsafe {
        let mut data = modedata as *mut window_buffer_modedata;
        let mut wp = (*data).wp as *mut window_pane;
        // window_mode_entry *wme;

        let mut wme = tailq_first(&raw mut (*wp).modes);
        if wme.is_null() || (*wme).data != modedata {
            return;
        }
        window_buffer_key(wme, c, null_mut(), null_mut(), key, null_mut())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_get_key(modedata: *mut c_void, itemdata: *mut c_void, line: u32) -> key_code {
    unsafe {
        let mut data = modedata as *mut window_buffer_modedata;
        let mut item = itemdata as *mut window_buffer_itemdata;
        // struct format_tree *ft;
        // struct session *s = NULL;
        // struct winlink *wl = NULL;
        // struct window_pane *wp = NULL;
        // struct paste_buffer *pb;
        // char *expanded;
        // key_code key;

        let mut s = null_mut::<session>();
        let mut wl = null_mut::<winlink>();
        let mut wp = null_mut::<window_pane>();

        if cmd_find_valid_state(&raw mut (*data).fs).as_bool() {
            s = (*data).fs.s;
            wl = (*data).fs.wl;
            wp = (*data).fs.wp;
        }
        let Some(pb) = NonNull::new(paste_get_name((*item).name)) else {
            return KEYC_NONE;
        };

        let ft = format_create(null_mut(), null_mut(), FORMAT_NONE, 0);
        format_defaults(ft, null_mut(), null_mut(), null_mut(), null_mut());
        format_defaults(ft, null_mut(), s, wl, wp);
        format_defaults_paste_buffer(ft, pb.as_ptr());
        format_add(ft, c"line".as_ptr(), c"%u".as_ptr(), line);

        let expanded = format_expand(ft, (*data).key_format);
        let key = key_string_lookup_string(expanded);
        free_(expanded);
        format_free(ft);
        key
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_init(
    wme: *mut window_mode_entry,
    fs: *mut cmd_find_state,
    args: *mut args,
) -> *mut screen {
    unsafe {
        let mut s = null_mut();
        let mut wp = (*wme).wp;
        let data = xcalloc1::<window_buffer_modedata>();
        (*wme).data = data as *mut window_buffer_modedata as *mut c_void;
        (*data).wp = wp;
        cmd_find_copy_state(&raw mut (*data).fs, fs);

        if (args.is_null() || !args_has_(args, 'F')) {
            (*data).format = xstrdup(WINDOW_BUFFER_DEFAULT_FORMAT).as_ptr();
        } else {
            (*data).format = xstrdup(args_get_(args, 'F')).as_ptr();
        }
        if (args.is_null() || !args_has_(args, 'K')) {
            (*data).key_format = xstrdup(WINDOW_BUFFER_DEFAULT_KEY_FORMAT).as_ptr();
        } else {
            (*data).key_format = xstrdup(args_get_(args, 'K')).as_ptr();
        }
        if (args.is_null() || args_count(args) == 0) {
            (*data).command = xstrdup(WINDOW_BUFFER_DEFAULT_COMMAND).as_ptr();
        } else {
            (*data).command = xstrdup(args_string(args, 0)).as_ptr();
        }

        (*data).data = mode_tree_start(
            wp,
            args,
            Some(window_buffer_build),
            Some(window_buffer_draw),
            Some(window_buffer_search),
            Some(window_buffer_menu),
            None,
            Some(window_buffer_get_key),
            data as *mut window_buffer_modedata as *mut c_void,
            &raw const window_buffer_menu_items as *const menu_item,
            &raw mut window_buffer_sort_list as *mut *const c_char,
            window_buffer_sort_list_len,
            &raw mut s,
        );
        mode_tree_zoom((*data).data, args);

        mode_tree_build((*data).data);
        mode_tree_draw((*data).data);

        s
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_free(wme: *mut window_mode_entry) {
    unsafe {
        let mut data = (*wme).data as *mut window_buffer_modedata;

        if (data.is_null()) {
            return;
        }

        mode_tree_free((*data).data);

        for i in 0..(*data).item_size {
            window_buffer_free_item(*(*data).item_list.add(i as usize));
        }
        free_((*data).item_list);

        free_((*data).format);
        free_((*data).key_format);
        free_((*data).command);

        free_(data);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_resize(wme: *mut window_mode_entry, sx: u32, sy: u32) {
    unsafe {
        let mut data = (*wme).data as *mut window_buffer_modedata;
        mode_tree_resize((*data).data, sx, sy);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_update(wme: *mut window_mode_entry) {
    unsafe {
        let mut data = (*wme).data as *mut window_buffer_modedata;

        mode_tree_build((*data).data);
        mode_tree_draw((*data).data);
        (*(*data).wp).flags |= window_pane_flags::PANE_REDRAW;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_do_delete(
    modedata: *mut c_void,
    itemdata: *mut c_void,
    c: *mut client,
    key: key_code,
) {
    unsafe {
        let mut data = modedata as *mut window_buffer_modedata;
        let mut item = itemdata as *mut window_buffer_itemdata;

        if item == mode_tree_get_current((*data).data).cast() && mode_tree_down((*data).data, 0) == 0 {
            /*
             *If we were unable to select the item further down we are at
             * the end of the list. Move one element up instead, to make
             * sure that we preserve a valid selection or we risk having
             * the tree build logic reset it to the first item.
             */
            mode_tree_up((*data).data, 0);
        }

        if let Some(pb) = NonNull::new(paste_get_name((*item).name)) {
            paste_free(pb);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_do_paste(
    modedata: *mut c_void,
    itemdata: *mut c_void,
    c: *mut client,
    key: key_code,
) {
    unsafe {
        let mut data = modedata as *mut window_buffer_modedata;
        let item = itemdata as *mut window_buffer_itemdata;

        if (!paste_get_name((*item).name).is_null()) {
            mode_tree_run_command(c, null_mut(), (*data).command, (*item).name);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_finish_edit(ed: *mut window_buffer_editdata) {
    unsafe {
        free_((*ed).name);
        free_(ed);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_edit_close_cb(buf: *mut c_char, mut len: usize, arg: *mut c_void) {
    unsafe {
        let ed = arg as *mut window_buffer_editdata;

        if (buf.is_null() || len == 0) {
            window_buffer_finish_edit(ed);
            return;
        }

        let pb = paste_get_name((*ed).name);
        if (pb.is_null() || pb != (*ed).pb) {
            window_buffer_finish_edit(ed);
            return;
        }
        let pb = NonNull::new(pb).expect("just checked");

        let mut oldlen = 0;
        let oldbuf = paste_buffer_data_(pb, &mut oldlen);
        if (oldlen != 0 && *oldbuf.add(oldlen - 1) != b'\n' as c_char && *buf.add(len - 1) == b'\n' as c_char) {
            len -= 1;
        }
        if (len != 0) {
            paste_replace(pb, buf, len);
        }

        let wp = window_pane_find_by_id((*ed).wp_id);
        if (!wp.is_null()) {
            let wme = tailq_first(&raw mut (*wp).modes);
            if ((*wme).mode == &raw mut window_buffer_mode) {
                let data = (*wme).data as *mut window_buffer_modedata;
                mode_tree_build((*data).data);
                mode_tree_draw((*data).data);
            }
            (*wp).flags |= window_pane_flags::PANE_REDRAW;
        }
        window_buffer_finish_edit(ed);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_start_edit(
    data: *mut window_buffer_modedata,
    item: *mut window_buffer_itemdata,
    c: *mut client,
) {
    unsafe {
        // struct paste_buffer *pb;
        // const char *buf;
        // size_t len;
        // struct window_buffer_editdata *ed;

        let Some(pb) = NonNull::new(paste_get_name((*item).name)) else {
            return;
        };
        let mut len = 0;
        let buf = paste_buffer_data_(pb, &mut len);

        let ed = xcalloc1::<window_buffer_editdata>();
        ed.wp_id = (*(*data).wp).id;
        ed.name = xstrdup(paste_buffer_name(pb)).as_ptr();
        ed.pb = pb.as_ptr();
        let ed = ed as *mut window_buffer_editdata;

        if popup_editor(c, buf, len, Some(window_buffer_edit_close_cb), ed.cast()) != 0 {
            window_buffer_finish_edit(ed);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_buffer_key(
    wme: *mut window_mode_entry,
    c: *mut client,
    s: *mut session,
    wl: *mut winlink,
    mut key: key_code,
    m: *mut mouse_event,
) {
    unsafe {
        let mut wp = (*wme).wp;
        let mut data = (*wme).data as *mut window_buffer_modedata;
        let mut mtd = (*data).data as *mut mode_tree_data;
        let mut finished = false;

        'out: {
            if paste_is_empty() != 0 {
                finished = true;
                break 'out;
            }

            finished = mode_tree_key(mtd, c, &raw mut key, m, null_mut(), null_mut()) != 0;
            match key as u8 {
                b'e' => {
                    let item = mode_tree_get_current(mtd) as *mut window_buffer_itemdata;
                    window_buffer_start_edit(data, item, c);
                }
                b'd' => {
                    let item = mode_tree_get_current(mtd);
                    window_buffer_do_delete(data.cast(), item, c, key);
                    mode_tree_build(mtd);
                }
                b'D' => {
                    mode_tree_each_tagged(mtd, Some(window_buffer_do_delete), c, key, 0);
                    mode_tree_build(mtd);
                }
                b'P' => {
                    mode_tree_each_tagged(mtd, Some(window_buffer_do_paste), c, key, 0);
                    finished = true;
                }
                b'p' | b'\r' => {
                    let item = mode_tree_get_current(mtd);
                    window_buffer_do_paste(data.cast(), item, c, key);
                    finished = true;
                }
                _ => (),
            }
        }
        // out:
        if (finished || paste_is_empty() != 0) {
            window_pane_reset_mode(wp);
        } else {
            mode_tree_draw(mtd);
            (*wp).flags |= window_pane_flags::PANE_REDRAW;
        }
    }
}
