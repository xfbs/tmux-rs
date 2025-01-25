// https://man.openbsd.org/tree.3
// probably best way define a generic struct
// make the macros call the generic struct
use core::{ops::ControlFlow, ptr::null_mut};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct rb_head<T> {
    pub rbh_root: *mut T,
}

impl<T> Default for rb_head<T> {
    fn default() -> Self {
        Self { rbh_root: null_mut() }
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
enum rb_color {
    #[default]
    RB_BLACK = 0,
    RB_RED = 1,
}

#[repr(C)]
#[derive(Copy)]
pub struct rb_entry<T> {
    pub rbe_left: *mut T,
    pub rbe_right: *mut T,
    pub rbe_parent: *mut T,
    pub rbe_color: rb_color,
}

impl<T> std::fmt::Debug for rb_entry<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("rb_entry")
            .field("rbe_left", &self.rbe_left)
            .field("rbe_right", &self.rbe_right)
            .field("rbe_parent", &self.rbe_parent)
            .field("rbe_color", &self.rbe_color)
            .finish()
    }
}

impl<T> Default for rb_entry<T> {
    fn default() -> Self {
        Self {
            rbe_left: null_mut(),
            rbe_right: null_mut(),
            rbe_parent: null_mut(),
            rbe_color: Default::default(),
        }
    }
}
impl<T> Clone for rb_entry<T> {
    fn clone(&self) -> Self {
        Self {
            rbe_left: self.rbe_left,
            rbe_right: self.rbe_right,
            rbe_parent: self.rbe_parent,
            rbe_color: self.rbe_color,
        }
    }
}

pub trait GetEntry<T> {
    unsafe fn entry_mut(this: *mut Self) -> *mut rb_entry<T>;
    unsafe fn cmp(this: *const Self, other: *const Self) -> i32;
}

pub const unsafe fn rb_init<T>(head: *mut rb_head<T>) {
    unsafe {
        (*head).rbh_root = null_mut();
    }
}

pub const fn rb_initializer<T>() -> rb_head<T> {
    rb_head { rbh_root: null_mut() }
}

macro_rules! rb_left {
    ($elm:expr) => {
        (*GetEntry::entry_mut($elm)).rbe_left
    };
}
pub unsafe fn rb_left<T>(this: *mut T) -> *mut T
where
    T: GetEntry<T>,
{
    (*T::entry_mut(this)).rbe_left
}

macro_rules! rb_right {
    ($elm:expr) => {
        (*GetEntry::entry_mut($elm)).rbe_right
    };
}
pub unsafe fn rb_right<T>(this: *mut T) -> *mut T
where
    T: GetEntry<T>,
{
    (*T::entry_mut(this)).rbe_left
}

macro_rules! rb_parent {
    ($elm:expr) => {
        (*GetEntry::entry_mut($elm)).rbe_parent
    };
}
pub unsafe fn rb_parent<T>(this: *mut T) -> *mut T
where
    T: GetEntry<T>,
{
    (*T::entry_mut(this)).rbe_parent
}

macro_rules! rb_color {
    ($elm:expr) => {
        (*GetEntry::entry_mut($elm)).rbe_color
    };
}
pub unsafe fn rb_color<T>(elm: *mut T) -> rb_color
where
    T: GetEntry<T>,
{
    (*T::entry_mut(elm)).rbe_color
}

macro_rules! rb_root {
    ($head:expr) => {
        (*$head).rbh_root
    };
}
pub unsafe fn rb_root<T>(head: *mut rb_head<T>) -> *mut T {
    (*head).rbh_root
}

pub unsafe fn rb_empty<T>(head: *mut rb_head<T>) -> bool {
    (*head).rbh_root.is_null()
}

pub unsafe fn rb_set<T>(elm: *mut T, parent: *mut T)
where
    T: GetEntry<T>,
{
    (*T::entry_mut(elm)).rbe_parent = parent;
    (*T::entry_mut(elm)).rbe_right = null_mut();
    (*T::entry_mut(elm)).rbe_left = null_mut();
    (*T::entry_mut(elm)).rbe_color = rb_color::RB_RED;
}

pub unsafe fn rb_set_blackred<T>(black: *mut T, red: *mut T)
where
    T: GetEntry<T>,
{
    (*T::entry_mut(black)).rbe_color = rb_color::RB_BLACK;
    (*T::entry_mut(red)).rbe_color = rb_color::RB_RED;
}

pub unsafe fn rb_rotate_left<T>(head: *mut rb_head<T>, elm: *mut T)
where
    T: GetEntry<T>,
{
    let tmp = rb_right(elm);
    rb_right!(elm) = rb_left(tmp);
    if !rb_right!(elm).is_null() {
        rb_parent!(rb_left(tmp)) = elm;
    }
    rb_parent!(tmp) = rb_parent(elm);
    if !rb_parent(tmp).is_null() {
        if elm == rb_left(rb_parent(elm)) {
            rb_left!(rb_parent(elm)) = tmp;
        } else {
            rb_right!(rb_parent(elm)) = tmp;
        }
    } else {
        (*head).rbh_root = tmp;
    }

    rb_left!(tmp) = elm;
    rb_parent!(elm) = tmp;
}

pub unsafe fn rb_rotate_right<T>(head: *mut rb_head<T>, elm: *mut T)
where
    T: GetEntry<T>,
{
    let tmp = rb_left(elm);
    rb_left!(elm) = rb_right(tmp);
    if !rb_left(elm).is_null() {
        rb_parent!(rb_right(tmp)) = elm;
    }
    rb_parent!(tmp) = rb_parent(elm);
    if !rb_parent(tmp).is_null() {
        if elm == rb_left(rb_parent(elm)) {
            rb_left!(rb_parent(elm)) = tmp;
        } else {
            rb_right!(rb_parent(elm)) = tmp;
        }
    } else {
        (*head).rbh_root = tmp;
    }
    rb_right!(tmp) = elm;
    rb_parent!(elm) = tmp;
}

#[macro_export]
macro_rules! impl_rb_tree_protos {
    ($head_ty:ty, $ty:ty) => {
        ::paste::paste! {
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn [<$head_ty _RB_MINMAX>](head: *mut rb_head<$ty>, val: i32) -> *mut $ty {
                unsafe { $crate::tree::rb_minmax(head, val) }
            }

            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn [<$head_ty _RB_NEXT>](elm: *mut $ty) -> *mut $ty {
                unsafe { $crate::tree::rb_next(elm) }
            }

            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn [<$head_ty _RB_PREV>](elm: *mut $ty) -> *mut $ty {
                unsafe { $crate::tree::rb_prev(elm) }
            }
        }
    };
}
pub use impl_rb_tree_protos;

pub unsafe fn rb_minmax<T>(head: *mut rb_head<T>, val: i32) -> *mut T
where
    T: GetEntry<T>,
{
    let mut tmp: *mut T = (*head).rbh_root;
    let mut parent: *mut T = null_mut();

    while !tmp.is_null() {
        parent = tmp;
        if val < 0 {
            tmp = rb_left(tmp);
        } else {
            tmp = rb_right(tmp);
        }
    }

    parent
}

pub unsafe fn rb_insert_color<T>(head: *mut rb_head<T>, mut elm: *mut T)
where
    T: GetEntry<T>,
{
    let mut parent;

    while ({
        parent = rb_parent(elm);
        !parent.is_null() && rb_color(parent) == rb_color::RB_RED
    }) {
        let mut gparent = rb_parent(parent);
        if parent == rb_left(gparent) {
            let mut tmp = rb_right(gparent);
            if !tmp.is_null() && rb_color(tmp) == rb_color::RB_RED {
                rb_color!(tmp) = rb_color::RB_BLACK;
                rb_set_blackred(parent, gparent);
                elm = gparent;
                continue;
            }
            if rb_right(parent) == elm {
                rb_rotate_left(head, parent);
                tmp = parent;
                parent = elm;
                elm = tmp;
            }
            rb_set_blackred(parent, gparent);
            rb_rotate_right(head, gparent);
        } else {
            let mut tmp = rb_left(gparent);
            if !tmp.is_null() && rb_color(tmp) == rb_color::RB_RED {
                rb_color!(tmp) = rb_color::RB_BLACK;
                rb_set_blackred(parent, gparent);
                elm = gparent;
                continue;
            }
            if rb_left(parent) == elm {
                rb_rotate_right(head, parent);
                tmp = parent;
                parent = elm;
                elm = tmp;
            }
            rb_set_blackred(parent, gparent);
            rb_rotate_left(head, gparent);
        }
    }
    (*T::entry_mut((*head).rbh_root)).rbe_color = rb_color::RB_BLACK;
}

pub unsafe fn rb_remove_color<T>(head: *mut rb_head<T>, mut parent: *mut T, mut elm: *mut T)
where
    T: GetEntry<T>,
{
    let mut tmp: *mut T;
    while (elm.is_null() || rb_color(elm) == rb_color::RB_BLACK) && elm != rb_root(head) {
        if rb_left(parent) == elm {
            tmp = rb_right(parent);
            if rb_color(tmp) == rb_color::RB_RED {
                rb_set_blackred(tmp, parent);
                rb_rotate_left(head, parent);
                tmp = rb_right(parent);
            }
            if ((rb_left(tmp).is_null() || rb_color(rb_left(tmp)) == rb_color::RB_BLACK)
                && (rb_right(tmp).is_null() || rb_color(rb_right(tmp)) == rb_color::RB_BLACK))
            {
                rb_color!(tmp) = rb_color::RB_RED;
                elm = parent;
                parent = rb_parent(elm);
            } else {
                if rb_right(tmp).is_null() || rb_color(rb_right(tmp)) == rb_color::RB_BLACK {
                    let mut oleft = rb_left(tmp);
                    if !oleft.is_null() {
                        rb_color!(oleft) = rb_color::RB_BLACK;
                    }
                    rb_color!(elm) = rb_color::RB_RED;
                    rb_rotate_right(head, oleft);
                    tmp = rb_right(parent);
                }
                rb_color!(tmp) = rb_color(parent);
                rb_color!(parent) = rb_color::RB_BLACK;
                if !rb_right(tmp).is_null() {
                    rb_color!(rb_right!(tmp)) = rb_color::RB_BLACK;
                }
                rb_rotate_left(head, parent);
                elm = rb_root(head);
                break;
            }
        } else {
            tmp = rb_left(parent);
            if rb_color(tmp) == rb_color::RB_RED {
                rb_set_blackred(tmp, parent);
                rb_rotate_right(head, parent);
                tmp = rb_left(parent);
            }
            if (rb_left(tmp).is_null() || rb_color(rb_left(tmp)) == rb_color::RB_BLACK)
                && (rb_right(tmp).is_null() || rb_color(rb_right(tmp)) == rb_color::RB_BLACK)
            {
                rb_color!(tmp) = rb_color::RB_RED;
                elm = parent;
                parent = rb_parent(elm);
            } else {
                if rb_left(tmp).is_null() || rb_color(rb_left(tmp)) == rb_color::RB_BLACK {
                    let oright = rb_right(tmp);
                    if !oright.is_null() {
                        rb_color!(oright) = rb_color::RB_BLACK;
                    }
                    rb_color!(tmp) = rb_color::RB_RED;
                    rb_rotate_left(head, oright);
                    tmp = rb_left(parent);
                }
                rb_color!(tmp) = rb_color(parent);
                rb_color!(parent) = rb_color::RB_BLACK;
                if !rb_left(tmp).is_null() {
                    rb_color!(rb_left(tmp)) = rb_color::RB_BLACK;
                }
                rb_rotate_right(head, parent);
                elm = rb_root(head);
                break;
            }
        }
    }

    if !elm.is_null() {
        rb_color!(elm) = rb_color::RB_BLACK;
    }
}

pub unsafe fn rb_remove<T>(head: *mut rb_head<T>, mut elm: *mut T) -> *mut T
where
    T: GetEntry<T>,
{
    let mut old: *mut T = elm;
    let mut child: *mut T;
    let mut parent: *mut T;
    let mut color: rb_color;

    'color: loop {
        if rb_left(elm).is_null() {
            child = rb_right(elm);
        } else if rb_right(elm).is_null() {
            child = rb_left(elm);
        } else {
            elm = rb_right(elm);
            let mut left: *mut T;
            while ({
                left = rb_left(elm);
                !left.is_null()
            }) {
                elm = left;
            }
            child = rb_right(elm);
            parent = rb_parent(elm);
            color = rb_color(elm);
            if !child.is_null() {
                rb_parent!(child) = parent;
            }
            if !parent.is_null() {
                if rb_left(parent) == elm {
                    rb_left!(parent) = child;
                } else {
                    rb_right!(parent) = child;
                }
            } else {
                rb_root!(head) = child
            }
            if rb_parent(elm) == old {
                parent = elm;
            }
            *GetEntry::entry_mut(elm) = (*GetEntry::entry_mut(old)).clone();
            if !rb_parent(old).is_null() {
                if rb_left(rb_parent(old)) == old {
                    rb_left!(rb_parent(old)) = elm;
                } else {
                    rb_right!(rb_parent(old)) = elm;
                }
            } else {
                rb_root!(head) = elm;
            }
            rb_parent!(rb_left(old)) = elm;
            if !rb_right(old).is_null() {
                rb_parent!(rb_right(old)) = elm;
            }
            if !parent.is_null() {
                left = parent;
            }
            // goto color;
            break 'color;
        }
        parent = rb_parent(elm);
        color = rb_color(elm);
        if !child.is_null() {
            rb_parent!(child) = parent;
        }
        if !parent.is_null() {
            if rb_left(parent) == elm {
                rb_left!(parent) = child;
            } else {
                rb_right!(parent) = child;
            }
        } else {
            rb_root!(head) = child;
        }
        break;
    }
    // color:
    if color == rb_color::RB_BLACK {
        rb_remove_color(head, parent, child);
    }
    return old;
}

pub unsafe fn rb_insert<T>(head: *mut rb_head<T>, mut elm: *mut T) -> *mut T
where
    T: GetEntry<T>,
{
    let mut parent = null_mut();
    let mut comp = 0;

    let mut tmp = rb_root(head);
    while !tmp.is_null() {
        parent = tmp;

        comp = T::cmp(elm, parent);
        tmp = match comp {
            ..0 => rb_left(tmp),
            1.. => rb_right(tmp),
            0 => return tmp,
        };
    }
    rb_set(elm, parent);
    if !parent.is_null() {
        match comp {
            ..0 => rb_left!(parent) = elm,
            0.. => rb_right!(parent) = elm,
        }
    } else {
        rb_root!(head) = elm;
    }
    rb_insert_color(head, elm);
    null_mut()
}

pub unsafe fn rb_find<T>(head: *mut rb_head<T>, elm: *mut T) -> *mut T
where
    T: GetEntry<T>,
{
    let mut tmp: *mut T = (*head).rbh_root;

    while !tmp.is_null() {
        tmp = match T::cmp(elm, tmp) {
            ..0 => rb_left(tmp),
            1.. => rb_right(tmp),
            0 => return tmp,
        };
    }

    null_mut()
}

pub unsafe fn rb_nfind<T>(head: *mut rb_head<T>, elm: *mut T) -> *mut T
where
    T: GetEntry<T>,
{
    let mut tmp = rb_root(head);
    let mut res = null_mut();
    let mut comp = 0;
    while !tmp.is_null() {
        tmp = match T::cmp(elm, tmp) {
            ..0 => {
                res = tmp;
                rb_left(tmp)
            }
            1.. => rb_right(tmp),
            0 => return tmp,
        };
    }
    res
}

pub unsafe fn rb_min<T>(head: *mut rb_head<T>) -> *mut T
where
    T: GetEntry<T>,
{
    rb_minmax(head, -1)
}

pub unsafe fn rb_max<T>(head: *mut rb_head<T>) -> *mut T
where
    T: GetEntry<T>,
{
    rb_minmax(head, 1)
}

pub unsafe fn rb_foreach<F, T, C>(head: *mut rb_head<T>, mut f: F) -> Option<C>
where
    F: FnMut(*mut T) -> ControlFlow<C>,
    T: GetEntry<T>,
{
    let mut x = rb_min(head);

    while !x.is_null() {
        match f(x) {
            ControlFlow::Continue(cont) => {
                x = rb_next(x);
            }
            ControlFlow::Break(brk) => return Some(brk),
        }
    }

    None
}

pub unsafe fn rb_foreach_safe<F, T, C>(head: *mut rb_head<T>, mut f: F) -> Option<C>
where
    F: FnMut(*mut T) -> ControlFlow<C>,
    T: GetEntry<T>,
{
    let mut x = rb_min(head);

    while !x.is_null() {
        let tmp = rb_next(x);
        match f(x) {
            ControlFlow::Continue(cont) => {
                x = tmp;
            }
            ControlFlow::Break(brk) => return Some(brk),
        }
    }

    None
}

#[allow(clippy::collapsible_else_if)]
pub unsafe fn rb_next<T>(mut elm: *mut T) -> *mut T
where
    T: GetEntry<T>,
{
    if !rb_right(elm).is_null() {
        elm = rb_right(elm);
        while !rb_left(elm).is_null() {
            elm = rb_left(elm);
        }
    } else {
        if !rb_parent(elm).is_null() && elm == rb_left(rb_parent(elm)) {
            elm = rb_parent(elm);
        } else {
            while !rb_parent(elm).is_null() && elm == rb_right(rb_parent(elm)) {
                elm = rb_parent(elm);
            }
            elm = rb_parent(elm);
        }
    }

    elm
}

#[allow(clippy::collapsible_else_if)]
pub unsafe fn rb_prev<T>(mut elm: *mut T) -> *mut T
where
    T: GetEntry<T>,
{
    if !rb_left(elm).is_null() {
        elm = rb_left(elm);
        while !rb_right(elm).is_null() {
            elm = rb_right(elm);
        }
    } else {
        if !rb_parent(elm).is_null() && elm == rb_right(rb_parent(elm)) {
            elm = rb_parent(elm);
        } else {
            while !rb_parent(elm).is_null() && elm == rb_left(rb_parent(elm)) {
                elm = rb_parent(elm);
            }
            elm = rb_parent(elm);
        }
    }

    elm
}
