use core::ptr::null_mut;
use std::ops::ControlFlow;

pub trait ListEntry<T, Discriminant = ()> {
    unsafe fn field(this: *mut Self) -> *mut list_entry<T>;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct list_head<T> {
    pub lh_first: *mut T,
}
pub const fn list_head_initializer<T>() -> list_head<T> { list_head { lh_first: null_mut() } }

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct list_entry<T> {
    pub le_next: *mut T,
    pub le_prev: *mut *mut T,
}

pub unsafe fn list_first<T>(head: *mut list_head<T>) -> *mut T { unsafe { (*head).lh_first } }

pub fn list_end<T>() -> *mut T { null_mut() }

pub unsafe fn list_empty<T>(head: *mut list_head<T>) -> bool { unsafe { list_first(head).is_null() } }

pub unsafe fn list_next<T, Discriminant>(elm: *mut T) -> *mut T
where
    T: ListEntry<T, Discriminant>,
{
    unsafe { (*ListEntry::field(elm)).le_next }
}

pub unsafe fn list_foreach<F, T, B, D>(head: *mut list_head<T>, mut f: F) -> std::ops::ControlFlow<B>
where
    F: FnMut(*mut T) -> std::ops::ControlFlow<B>,
    T: ListEntry<T, D>,
{
    let mut var = unsafe { list_first(head) };
    while !var.is_null() {
        if let ControlFlow::Break(break_value) = f(var) {
            return ControlFlow::Break(break_value);
        }
        var = list_next::<T, D>(var);
    }
    ControlFlow::Continue(())
}

pub unsafe fn list_foreach_safe<F, T, B, D>(head: *mut list_head<T>, mut f: F) -> std::ops::ControlFlow<B>
where
    F: FnMut(*mut T) -> std::ops::ControlFlow<B>,
    T: ListEntry<T, D>,
{
    let mut var = unsafe { list_first(head) };
    while !var.is_null() {
        let tmp = unsafe { list_next::<T, D>(var) };
        if let ControlFlow::Break(break_value) = f(var) {
            return ControlFlow::Break(break_value);
        }
        var = tmp;
    }
    ControlFlow::Continue(())
}

pub unsafe fn list_init<T>(head: *mut list_head<T>) {
    unsafe {
        (*head).lh_first = list_end();
    }
}

pub unsafe fn list_insert_after<T, D>(listelm: *mut T, elm: *mut T)
where
    T: ListEntry<T, D>,
{
    unsafe {
        (*ListEntry::field(elm)).le_next = (*ListEntry::field(listelm)).le_next;
        if !(*ListEntry::field(elm)).le_next.is_null() {
            (*ListEntry::field((*ListEntry::field(listelm)).le_next)).le_prev =
                &raw mut (*ListEntry::field(elm)).le_next;
        }
        (*ListEntry::field(listelm)).le_next = elm;
        (*ListEntry::field(elm)).le_prev = &raw mut (*ListEntry::field(listelm)).le_next;
    }
}

pub unsafe fn list_insert_before<T, D>(listelm: *mut T, elm: *mut T)
where
    T: ListEntry<T, D>,
{
    unsafe {
        (*ListEntry::field(elm)).le_prev = (*ListEntry::field(listelm)).le_prev;
        (*ListEntry::field(elm)).le_next = listelm;
        *(*ListEntry::field(listelm)).le_prev = elm;
        (*ListEntry::field(listelm)).le_prev = &raw mut (*ListEntry::field(elm)).le_next;
    }
}

pub unsafe fn list_insert_head<T, D>(head: *mut list_head<T>, elm: *mut T)
where
    T: ListEntry<T, D>,
{
    unsafe {
        (*ListEntry::field(elm)).le_next = (*head).lh_first;
        if !(*ListEntry::field(elm)).le_next.is_null() {
            (*ListEntry::field((*head).lh_first)).le_prev = &raw mut (*ListEntry::field(elm)).le_next;
        }
        (*head).lh_first = elm;
        (*ListEntry::field(elm)).le_prev = &raw mut (*head).lh_first;
    }
}

pub unsafe fn list_remove<T, D>(elm: *mut T)
where
    T: ListEntry<T, D>,
{
    unsafe {
        if !(*ListEntry::field(elm)).le_next.is_null() {
            (*ListEntry::field((*ListEntry::field(elm)).le_next)).le_prev = (*ListEntry::field(elm)).le_prev;
        }
        *(*ListEntry::field(elm)).le_prev = (*ListEntry::field(elm)).le_next;
    }
}

pub unsafe fn list_replace<T, D>(elm: *mut T, elm2: *mut T)
where
    T: ListEntry<T, D>,
{
    unsafe {
        (*ListEntry::field(elm2)).le_next = (*ListEntry::field(elm)).le_next;
        if !(*ListEntry::field(elm2)).le_next.is_null() {
            (*ListEntry::field((*ListEntry::field(elm2)).le_next)).le_prev = &raw mut (*ListEntry::field(elm2)).le_next;
        }
        (*ListEntry::field(elm2)).le_prev = (*ListEntry::field(elm)).le_prev;
        *(*ListEntry::field(elm2)).le_prev = elm2;
    }
}

// tailq

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tailq_head<T> {
    pub tqh_first: *mut T,
    pub tqh_last: *mut *mut T,
}

pub const unsafe fn tailq_head_initializer<T>(head: *mut tailq_head<T>) {
    unsafe {
        (*head).tqh_first = null_mut();
        (*head).tqh_last = &raw mut (*head).tqh_first;
    }
}

#[macro_export]
macro_rules! TAILQ_HEAD_INITIALIZER {
    ($ident:ident) => {
        compat_rs::queue::tailq_head {
            tqh_first: null_mut(),
            tqh_last: unsafe { &raw mut $ident.tqh_first },
        }
    };
}
pub use TAILQ_HEAD_INITIALIZER;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct tailq_entry<T> {
    pub tqe_next: *mut T,
    pub tqe_prev: *mut *mut T,
}

impl<T> std::fmt::Debug for tailq_entry<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("tailq_entry")
            .field("tqe_next", &self.tqe_next)
            .field("tqe_prev", &self.tqe_prev)
            .finish()
    }
}

pub trait Entry<T, Discriminant = ()> {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<T>;
}

pub trait Head<T> {
    unsafe fn head(this: *mut Self) -> *mut tailq_head<T>;
}

pub unsafe fn tailq_init<T>(head: *mut tailq_head<T>) {
    unsafe {
        (*head).tqh_first = core::ptr::null_mut();
        (*head).tqh_last = &raw mut (*head).tqh_first;
    }
}

pub unsafe fn tailq_first<T>(head: *mut tailq_head<T>) -> *mut T { unsafe { (*head).tqh_first } }
pub fn tailq_end<T>(_head: *mut tailq_head<T>) -> *mut T { core::ptr::null_mut() }

pub unsafe fn tailq_next<T, Q, D>(elm: *mut T) -> *mut Q
where
    T: Entry<Q, D>,
{
    unsafe { (*Entry::entry(elm)).tqe_next }
}

/*
#[macro_export]
macro_rules! tailq_last {
    ($head:expr, $headname:ty) => {{
        let head: *mut $headname = (*$head).tqh_last.cast();
        *(*head).tqh_last
    }};
}
pub use tailq_last;
*/

pub unsafe fn tailq_last<T>(head: *mut tailq_head<T>) -> *mut T {
    unsafe {
        let head: *mut tailq_head<T> = (*head).tqh_last.cast();
        *(*head).tqh_last
    }
}

/*
#[macro_export]
macro_rules! tailq_prev {
    ($elm:expr, $headname:ty, $field:ident) => {{
        let head: *mut $headname = (*$elm).$field.tqe_prev.cast();
        *(*head).tqh_last
    }};
}
pub use tailq_prev;
*/

pub unsafe fn tailq_prev<T, Q, D>(elm: *mut T) -> *mut Q
where
    T: Entry<Q, D>,
{
    unsafe {
        let head: *mut tailq_head<Q> = (*Entry::entry(elm)).tqe_prev.cast();
        *(*head).tqh_last
    }
}

pub unsafe fn tailq_empty<T>(head: *const tailq_head<T>) -> bool { unsafe { (*head).tqh_first.is_null() } }

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

pub unsafe extern "C" fn tailq_insert_tail<T, D>(head: *mut tailq_head<T>, elm: *mut T)
where
    T: Entry<T, D>,
{
    unsafe {
        (*Entry::<_, D>::entry(elm)).tqe_next = null_mut();
        (*Entry::<_, D>::entry(elm)).tqe_prev = (*head).tqh_last;
        *(*head).tqh_last = elm;
        (*head).tqh_last = &raw mut (*Entry::<_, D>::entry(elm)).tqe_next;
    }
}

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

pub unsafe fn tailq_remove<T, D>(head: *mut tailq_head<T>, elm: *mut T)
where
    T: Entry<T, D>,
{
    unsafe {
        if !(*Entry::<_, D>::entry(elm)).tqe_next.is_null() {
            (*Entry::<_, D>::entry((*Entry::<_, D>::entry(elm)).tqe_next)).tqe_prev =
                (*Entry::<_, D>::entry(elm)).tqe_prev;
        } else {
            (*head).tqh_last = (*Entry::<_, D>::entry(elm)).tqe_prev;
        }
        *(*Entry::<_, D>::entry(elm)).tqe_prev = (*Entry::<_, D>::entry(elm)).tqe_next;
    }
}

pub unsafe fn tailq_replace<T, D>(head: *mut tailq_head<T>, elm: *mut T, elm2: *mut T)
where
    T: Entry<T, D>,
{
    unsafe {
        (*Entry::<_, D>::entry(elm2)).tqe_next = (*Entry::<_, D>::entry(elm)).tqe_next;
        if !(*Entry::<_, D>::entry(elm2)).tqe_next.is_null() {
            (*Entry::<_, D>::entry((*Entry::<_, D>::entry(elm2)).tqe_next)).tqe_prev =
                &raw mut (*Entry::<_, D>::entry(elm2)).tqe_next;
        } else {
            (*head).tqh_last = &raw mut (*Entry::<_, D>::entry(elm2)).tqe_next;
        }
        (*Entry::<_, D>::entry(elm2)).tqe_prev = (*Entry::<_, D>::entry(elm)).tqe_prev;
        *(*Entry::<_, D>::entry(elm2)).tqe_prev = elm2;
    }
}

#[inline]
pub unsafe fn tailq_foreach<F, T, B, D>(head: *mut tailq_head<T>, mut f: F) -> std::ops::ControlFlow<B>
where
    F: FnMut(*mut T) -> std::ops::ControlFlow<B>,
    T: Entry<T, D>,
{
    unsafe {
        let mut curr = tailq_first(head);

        while !curr.is_null() {
            if let ControlFlow::Break(break_value) = f(curr) {
                return ControlFlow::Break(break_value);
            }
            curr = tailq_next(curr);
        }

        ControlFlow::Continue(())
    }
}

pub unsafe fn tailq_foreach_<T, D>(head: *mut tailq_head<T>) -> TailQIterator<T, D>
where
    T: Entry<T, D>,
{
    unsafe {
        TailQIterator {
            curr: tailq_first(head),
            _phantom: std::marker::PhantomData,
        }
    }
}

// this implementation can be used in place of safe and non-safe
pub struct TailQIterator<T, D> {
    curr: *mut T,
    _phantom: std::marker::PhantomData<D>,
}
impl<T, D> Iterator for TailQIterator<T, D>
where
    T: Entry<T, D>,
{
    type Item = *mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr.is_null() {
            return None;
        }

        let tmp = unsafe { tailq_next(self.curr) };
        self.curr = tmp;
        Some(tmp)
    }
}

#[inline]
pub unsafe fn tailq_foreach_safe<F, T, B, D>(head: *mut tailq_head<T>, mut f: F) -> std::ops::ControlFlow<B>
where
    F: FnMut(*mut T) -> std::ops::ControlFlow<B>,
    T: Entry<T, D>,
{
    unsafe {
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
}

#[inline]
pub unsafe fn tailq_foreach_reverse<F, T, B, D>(head: *mut tailq_head<T>, mut f: F) -> std::ops::ControlFlow<B>
where
    F: FnMut(*mut T) -> std::ops::ControlFlow<B>,
    T: Entry<T, D>,
{
    unsafe {
        let mut curr = tailq_last(head);

        while !curr.is_null() {
            let tmp = tailq_prev(curr);
            if let ControlFlow::Break(break_value) = f(curr) {
                return ControlFlow::Break(break_value);
            }
            curr = tmp;
        }

        ControlFlow::Continue(())
    }
}

#[inline]
pub unsafe fn tailq_concat<T, D>(head1: *mut tailq_head<T>, head2: *mut tailq_head<T>)
where
    T: Entry<T, D>,
{
    unsafe {
        if !tailq_empty::<T>(head2) {
            *(*head1).tqh_last = (*head2).tqh_first;
            (*Entry::entry((*head2).tqh_first)).tqe_prev = (*head1).tqh_last;
            (*head1).tqh_last = (*head2).tqh_last;
            tailq_init(head2);
        }
    }
}

#[macro_export]
macro_rules! impl_tailq_entry {
    ($struct_name:ident, $attribute_field_name:ident, $attribute_field_ty:ty) => {
        impl ::compat_rs::queue::Entry<$struct_name> for $struct_name {
            unsafe fn entry(this: *mut Self) -> *mut $attribute_field_ty {
                unsafe { &raw mut (*this).$attribute_field_name }
            }
        }
    };
}
pub use impl_tailq_entry;
