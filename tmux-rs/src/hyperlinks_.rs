use compat_rs::{
    TAILQ_HEAD_INITIALIZER, VIS_CSTYLE, VIS_OCTAL,
    queue::{tailq_first, tailq_insert_tail, tailq_remove},
    tree::{rb_find, rb_foreach, rb_init, rb_insert, rb_remove},
};
use libc::strcmp;

use crate::{xmalloc::Zeroable, *};

unsafe extern "C" {
    // pub unsafe fn hyperlinks_put(_: *mut hyperlinks, _: *const c_char, _: *const c_char) -> c_uint;
    // pub unsafe fn hyperlinks_get( _: *mut hyperlinks, _: c_uint, _: *mut *const c_char, _: *mut *const c_char, _: *mut *const c_char,) -> c_int;
    // pub unsafe fn hyperlinks_init() -> *mut hyperlinks;
    // pub unsafe fn hyperlinks_copy(_: *mut hyperlinks) -> *mut hyperlinks;
    // pub unsafe fn hyperlinks_reset(_: *mut hyperlinks);
    // pub unsafe fn hyperlinks_free(_: *mut hyperlinks);
}

const MAX_HYPERLINKS: u32 = 5000;

static mut hyperlinks_next_external_id: c_longlong = 1;
static mut global_hyperlinks_count: u32 = 0;

unsafe impl Zeroable for hyperlinks_uri {}
compat_rs::impl_tailq_entry!(hyperlinks_uri, list_entry, tailq_entry<hyperlinks_uri>);
#[repr(C)]
// #[derive(compat_rs::TailQEntry)]
pub struct hyperlinks_uri {
    pub tree: *mut hyperlinks,

    pub inner: u32,
    pub internal_id: *mut c_char,
    pub external_id: *mut c_char,
    pub uri: *mut c_char,

    // #[entry]
    pub list_entry: tailq_entry<hyperlinks_uri>,

    pub by_inner_entry: rb_entry<hyperlinks_uri>,
    pub by_uri_entry: rb_entry<hyperlinks_uri>,
}

pub type hyperlinks_by_uri_tree = rb_head<hyperlinks_uri>;
pub type hyperlinks_by_inner_tree = rb_head<hyperlinks_uri>;

pub type hyperlinks_list = tailq_head<hyperlinks_uri>;

static mut global_hyperlinks: hyperlinks_list = TAILQ_HEAD_INITIALIZER!(global_hyperlinks);

#[repr(C)]
pub struct hyperlinks {
    pub next_inner: u32,
    pub by_inner: hyperlinks_by_inner_tree,
    pub by_uri: hyperlinks_by_uri_tree,
    pub references: u32,
}

#[unsafe(no_mangle)]
unsafe extern "C" fn hyperlinks_by_uri_cmp(left: *const hyperlinks_uri, right: *const hyperlinks_uri) -> i32 {
    unsafe {
        if (*(*left).internal_id == b'\0' as _ || *(*right).internal_id == b'\0' as _) {
            if (*(*left).internal_id != b'\0' as _) {
                return (-1);
            }
            if (*(*right).internal_id != b'\0' as _) {
                return (1);
            }
            return ((*left).inner as i32 - (*right).inner as i32);
        }

        let r = strcmp((*left).internal_id, (*right).internal_id);
        if (r != 0) {
            return (r);
        }
        strcmp((*left).uri, (*right).uri)
    }
}

// RB_PROTOTYPE_STATIC(hyperlinks_by_uri_tree, hyperlinks_uri, by_uri_entry, hyperlinks_by_uri_cmp);
RB_GENERATE!(
    hyperlinks_by_uri_tree,
    hyperlinks_uri,
    by_uri_entry,
    hyperlinks_by_uri_cmp
);

#[unsafe(no_mangle)]
unsafe extern "C" fn hyperlinks_by_inner_cmp(left: *const hyperlinks_uri, right: *const hyperlinks_uri) -> i32 {
    unsafe { (*left).inner.wrapping_sub((*right).inner) as i32 }
}

// RB_PROTOTYPE_STATIC(hyperlinks_by_inner_tree, hyperlinks_uri, by_inner_entry, hyperlinks_by_inner_cmp);
RB_GENERATE!(
    hyperlinks_by_inner_tree,
    hyperlinks_uri,
    by_inner_entry,
    hyperlinks_by_inner_cmp
);

#[unsafe(no_mangle)]
unsafe extern "C" fn hyperlinks_remove(hlu: *mut hyperlinks_uri) {
    unsafe {
        let mut hl = (*hlu).tree;

        tailq_remove::<_, _>(&raw mut global_hyperlinks, hlu);
        global_hyperlinks_count -= 1;

        rb_remove::<_, discr_by_inner_entry>(&raw mut (*hl).by_inner, hlu);
        rb_remove::<_, discr_by_uri_entry>(&raw mut (*hl).by_uri, hlu);

        free_((*hlu).internal_id);
        free_((*hlu).external_id);
        free_((*hlu).uri);
        free_(hlu);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyperlinks_put(
    hl: *mut hyperlinks,
    uri_in: *const c_char,
    mut internal_id_in: *const c_char,
) -> u32 {
    unsafe {
        // struct hyperlinks_uri	 find, *hlu;
        // char			*uri, *internal_id, *external_id;
        let mut uri = null_mut();
        let mut internal_id = null_mut();
        let mut external_id = null_mut();

        /*
         * Anonymous URI are stored with an empty internal ID and the tree
         * comparator will make sure they never match each other (so each
         * anonymous URI is unique).
         */
        if (internal_id_in.is_null()) {
            internal_id_in = c"".as_ptr();
        }

        utf8_stravis(&raw mut uri, uri_in, VIS_OCTAL | VIS_CSTYLE);
        utf8_stravis(&raw mut internal_id, internal_id_in, VIS_OCTAL | VIS_CSTYLE);

        if (*internal_id_in != b'\0' as _) {
            let mut find = MaybeUninit::<hyperlinks_uri>::uninit();
            let mut find = find.as_mut_ptr();
            (*find).uri = uri;
            (*find).internal_id = internal_id;

            let hlu = rb_find::<_, discr_by_uri_entry>(&raw mut (*hl).by_uri, find);
            if (!hlu.is_null()) {
                free_(uri);
                free_(internal_id);
                return ((*hlu).inner);
            }
        }
        xasprintf(&raw mut external_id, c"tmux%llX".as_ptr(), hyperlinks_next_external_id);
        hyperlinks_next_external_id += 1;

        let hlu = xcalloc1::<hyperlinks_uri>() as *mut hyperlinks_uri;
        (*hlu).inner = (*hl).next_inner;
        (*hl).next_inner += 1;
        (*hlu).internal_id = internal_id;
        (*hlu).external_id = external_id;
        (*hlu).uri = uri;
        (*hlu).tree = hl;
        rb_insert::<_, discr_by_uri_entry>(&raw mut (*hl).by_uri, hlu);
        rb_insert::<_, discr_by_inner_entry>(&raw mut (*hl).by_inner, hlu);

        tailq_insert_tail(&raw mut global_hyperlinks, hlu);
        global_hyperlinks_count += 1;
        if (global_hyperlinks_count == MAX_HYPERLINKS) {
            hyperlinks_remove(tailq_first(&raw mut global_hyperlinks));
        }

        (*hlu).inner
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyperlinks_get(
    hl: *mut hyperlinks,
    inner: u32,
    uri_out: *mut *const c_char,
    internal_id_out: *mut *const c_char,
    external_id_out: *mut *const c_char,
) -> i32 {
    unsafe {
        let mut find = MaybeUninit::<hyperlinks_uri>::uninit();
        let mut find = find.as_mut_ptr();
        (*find).inner = inner;

        let hlu = rb_find::<_, discr_by_inner_entry>(&raw mut (*hl).by_inner, find);
        if (hlu.is_null()) {
            return 0;
        }
        if (!internal_id_out.is_null()) {
            *internal_id_out = (*hlu).internal_id;
        }
        if (!external_id_out.is_null()) {
            *external_id_out = (*hlu).external_id;
        }
        *uri_out = (*hlu).uri as _;
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyperlinks_init() -> *mut hyperlinks {
    unsafe {
        let mut hl = xcalloc_::<hyperlinks>(1).as_ptr();
        (*hl).next_inner = 1;
        rb_init(&raw mut (*hl).by_uri);
        rb_init(&raw mut (*hl).by_inner);
        (*hl).references = 1;
        hl
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyperlinks_copy(hl: *mut hyperlinks) -> *mut hyperlinks {
    unsafe {
        (*hl).references += 1;
    }
    hl
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyperlinks_reset(hl: *mut hyperlinks) {
    unsafe {
        for hlu in rb_foreach::<_, discr_by_inner_entry>(&raw mut (*hl).by_inner) {
            hyperlinks_remove(hlu.as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hyperlinks_free(hl: *mut hyperlinks) {
    unsafe {
        (*hl).references -= 1;
        if ((*hl).references == 0) {
            hyperlinks_reset(hl);
            free_(hl);
        }
    }
}
