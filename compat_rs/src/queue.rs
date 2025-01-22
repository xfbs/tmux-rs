use core::ptr::null_mut;
use std::ops::ControlFlow;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tailq_head<T> {
    pub tqh_first: *mut T,
    pub tqh_last: *mut *mut T,
}

impl<T> tailq_head<T> {
    pub const fn new() -> Self {
        Self {
            tqh_first: null_mut(),
            tqh_last: null_mut(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tailq_entry<T> {
    pub tqe_next: *mut T,
    pub tqe_prev: *mut *mut T,
}

pub trait Entry<T, Discriminant = ()> {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<T>;
}

pub trait Head<T> {
    unsafe fn head(this: *mut Self) -> *mut tailq_head<T>;
}

pub unsafe fn tailq_init<T>(head: *mut tailq_head<T>) {
    (*head).tqh_first = core::ptr::null_mut();
    (*head).tqh_last = &raw mut (*head).tqh_first;
}

pub unsafe fn tailq_first<T>(head: *mut tailq_head<T>) -> *mut T {
    (*head).tqh_first
}
pub unsafe fn tailq_end<T>(_head: *mut tailq_head<T>) -> *mut T {
    core::ptr::null_mut()
}

pub unsafe fn tailq_next<T, Q, D>(elm: *mut T) -> *mut Q
where
    T: Entry<Q, D>,
{
    (*Entry::entry(elm)).tqe_next
}

#[macro_export]
macro_rules! tailq_last {
    ($head:expr, $headname:ty) => {{
        let head: *mut $headname = (*$head).tqh_last.cast();
        *(*head).tqh_last
    }};
}
pub use tailq_last;

#[macro_export]
macro_rules! tailq_prev {
    ($elm:expr, $headname:ty, $field:ident) => {{
        let head: *mut $headname = (*$elm).$field.tqe_prev.cast();
        *(*head).tqh_last
    }};
}
pub use tailq_prev;

pub unsafe fn tailq_empty<T>(head: *mut tailq_head<T>) -> bool {
    unsafe { tailq_first(head) == tailq_end(head) }
}

#[macro_export]
macro_rules! tailq_insert_head {
    ($head:expr, $elm:expr, $field:ident) => {
        ((*$elm).$field.tqe_next = (*$head).tqh_first);
        if !(*$elm).$field.tqe_next.is_null() {
            (*(*$head).tqh_first).$field.tqe_prev = &raw mut (*$elm).$field.tqe_next;
        } else {
            (*$head).tqh_last = &raw mut (*$elm).$field.tqe_next;
        }

        (*$head).tqh_first = $elm;
        (*$elm).$field.tqe_prev = &raw mut (*$head).tqh_first;
    };
}
pub use tailq_insert_head;

#[macro_export]
macro_rules! tailq_insert_tail {
    ($head:expr, $elm:ident, $field:ident) => {
        (*$elm).$field.tqe_next = null_mut();
        (*$elm).$field.tqe_prev = (*$head).tqh_last;
        *(*$head).tqh_last = $elm;
        (*$head).tqh_last = &raw mut (*$elm).$field.tqe_next;
    };
}
pub use tailq_insert_tail;

#[macro_export]
macro_rules! tailq_insert_after {
    ($head:expr, $listelm:ident, $elm:ident, $field:ident) => {
        (*$elm).$field.tqe_next = (*$listelm).$field.tqe_next;

        if !(*$elm).$field.tqe_next.is_null() {
            (*(*$elm).$field.tqe_next).$field.tqe_prev = &raw mut (*$elm).$field.tqe_next;
        } else {
            (*$head).tqh_last = &raw mut (*$elm).$field.tqe_next;
        }

        (*$listelm).$field.tqe_next = $elm;
        (*$elm).$field.tqe_prev = &raw mut (*$listelm).$field.tqe_next;
    };
}
pub use tailq_insert_after;

#[macro_export]
macro_rules! tailq_insert_before {
    ($listelm:ident, $elm:ident, $field:ident) => {
        (*$elm).$field.tqe_prev = (*$listelm).$field.tqe_prev;
        (*$elm).$field.tqe_next = $listelm;
        *(*$listelm).$field.tqe_prev = $elm;
        (*$listelm).$field.tqe_prev = &raw mut (*$elm).$field.tqe_next;
    };
}
pub use tailq_insert_before;

#[macro_export]
macro_rules! tailq_remove {
    ($head:expr, $elm:ident, $field:ident) => {
        if !((*$elm).$field.tqe_next).is_null() {
            (*(*$elm).$field.tqe_next).$field.tqe_prev = (*$elm).$field.tqe_prev;
        } else {
            (*$head).tqh_last = (*$elm).$field.tqe_prev;
        }
        *(*$elm).$field.tqe_prev = (*$elm).$field.tqe_next;
    };
}
pub use tailq_remove;

#[inline]
pub unsafe fn tailq_foreach<F, T, B, D>(head: *mut tailq_head<T>, mut f: F) -> std::ops::ControlFlow<B>
where
    F: FnMut(*mut T) -> std::ops::ControlFlow<B>,
    T: Entry<T, D>,
{
    let mut curr = tailq_first(head);

    while !curr.is_null() {
        if let ControlFlow::Break(break_value) = f(curr) {
            return ControlFlow::Break(break_value);
        }
        curr = tailq_next(curr);
    }

    ControlFlow::Continue(())
}

// need to store next before calling func so can be used for deallocation
#[inline]
pub unsafe fn tailq_foreach_safe<F, T, B, D>(head: *mut tailq_head<T>, mut f: F) -> std::ops::ControlFlow<B>
where
    F: FnMut(*mut T) -> std::ops::ControlFlow<B>,
    T: Entry<T, D>,
{
    let mut curr = tailq_first(head);

    while !curr.is_null() {
        let tmp = tailq_next(curr);
        if let ControlFlow::Break(break_value) = f(curr) {
            return ControlFlow::Break(break_value);
        }
        curr = tmp;
    }

    ControlFlow::Continue(())
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct list_head<T> {
    pub lh_first: *mut T,
}

impl<T> list_head<T> {
    pub const fn const_default() -> Self {
        Self { lh_first: null_mut() }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct list_entry<T> {
    pub le_next: *mut T,
    pub le_prev: *mut *mut T,
}
