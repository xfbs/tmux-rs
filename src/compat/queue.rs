use core::ptr::null_mut;
use std::ptr::NonNull;

// tailq

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tailq_head<T> {
    pub tqh_first: *mut T,
    pub tqh_last: *mut *mut T,
}


#[repr(C)]
#[derive(Copy, Clone)]
pub struct tailq_entry<T> {
    pub tqe_next: *mut T,
    pub tqe_prev: *mut *mut T,
}

impl<T> Default for tailq_entry<T> {
    fn default() -> Self {
        Self {
            tqe_next: null_mut(),
            tqe_prev: null_mut(),
        }
    }
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

pub unsafe fn tailq_init<T>(head: *mut tailq_head<T>) {
    unsafe {
        (*head).tqh_first = core::ptr::null_mut();
        (*head).tqh_last = &raw mut (*head).tqh_first;
    }
}


pub unsafe fn tailq_first<T>(head: *mut tailq_head<T>) -> *mut T {
    unsafe { (*head).tqh_first }
}

pub unsafe fn tailq_next<T, Q, D>(elm: *mut T) -> *mut Q
where
    T: Entry<Q, D>,
{
    unsafe { (*Entry::entry(elm)).tqe_next }
}

pub unsafe fn tailq_last<T>(head: *mut tailq_head<T>) -> *mut T {
    unsafe { *(*(*head).tqh_last.cast::<tailq_head<T>>()).tqh_last }
}

pub unsafe fn tailq_prev<T, Q, D>(elm: *mut T) -> *mut Q
where
    T: Entry<Q, D>,
{
    unsafe {
        let head: *mut tailq_head<Q> = (*Entry::entry(elm)).tqe_prev.cast();
        *(*head).tqh_last
    }
}

pub unsafe fn tailq_empty<T>(head: *const tailq_head<T>) -> bool {
    unsafe { (*head).tqh_first.is_null() }
}

pub unsafe fn tailq_insert_head<T, D>(head: *mut tailq_head<T>, elm: *mut T)
where
    T: Entry<T, D>,
{
    unsafe {
        (*T::entry(elm)).tqe_next = (*head).tqh_first;

        if !(*T::entry(elm)).tqe_next.is_null() {
            (*T::entry((*head).tqh_first)).tqe_prev = &raw mut (*T::entry(elm)).tqe_next;
        } else {
            (*head).tqh_last = &raw mut (*T::entry(elm)).tqe_next;
        }

        (*head).tqh_first = elm;
        (*T::entry(elm)).tqe_prev = &raw mut (*head).tqh_first;
    }
}

pub unsafe fn tailq_insert_tail<T, D>(head: *mut tailq_head<T>, elm: *mut T)
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

pub unsafe fn tailq_insert_after<T, D>(head: *mut tailq_head<T>, listelm: *mut T, elm: *mut T)
where
    T: Entry<T, D>,
{
    unsafe {
        (*T::entry(elm)).tqe_next = (*T::entry(listelm)).tqe_next;

        if !(*T::entry(elm)).tqe_next.is_null() {
            (*T::entry((*T::entry(elm)).tqe_next)).tqe_prev = &raw mut (*T::entry(elm)).tqe_next;
        } else {
            (*head).tqh_last = &raw mut (*T::entry(elm)).tqe_next;
        }

        (*T::entry(listelm)).tqe_next = elm;
        (*T::entry(elm)).tqe_prev = &raw mut (*T::entry(listelm)).tqe_next;
    }
}

pub unsafe fn tailq_insert_before<T, D>(listelm: *mut T, elm: *mut T)
where
    T: Entry<T, D>,
{
    unsafe {
        (*T::entry(elm)).tqe_prev = (*T::entry(listelm)).tqe_prev;
        (*T::entry(elm)).tqe_next = listelm;
        *(*T::entry(listelm)).tqe_prev = elm;
        (*T::entry(listelm)).tqe_prev = &raw mut (*T::entry(elm)).tqe_next;
    }
}

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

pub unsafe fn tailq_foreach_const<T, D>(
    head: *const tailq_head<T>,
) -> ConstTailqForwardIterator<T, D>
where
    T: Entry<T, D>,
{
    unsafe {
        ConstTailqForwardIterator {
            curr: NonNull::new((*head).tqh_first),
            _phantom: std::marker::PhantomData,
        }
    }
}
// this implementation can be used in place of safe and non-safe
pub struct ConstTailqForwardIterator<T, D> {
    curr: Option<NonNull<T>>,
    _phantom: std::marker::PhantomData<D>,
}
impl<T, D> Iterator for ConstTailqForwardIterator<T, D>
where
    T: Entry<T, D>,
{
    type Item = NonNull<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr?.as_ptr();
        std::mem::replace(&mut self.curr, NonNull::new(unsafe { tailq_next(curr) }))
    }
}

pub unsafe fn tailq_foreach<T, D>(head: *mut tailq_head<T>) -> TailqForwardIterator<T, D>
where
    T: Entry<T, D>,
{
    unsafe {
        TailqForwardIterator {
            curr: NonNull::new(tailq_first(head)),
            _phantom: std::marker::PhantomData,
        }
    }
}

// this implementation can be used in place of safe and non-safe
pub struct TailqForwardIterator<T, D> {
    curr: Option<NonNull<T>>,
    _phantom: std::marker::PhantomData<D>,
}
impl<T, D> Iterator for TailqForwardIterator<T, D>
where
    T: Entry<T, D>,
{
    type Item = NonNull<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr?.as_ptr();
        std::mem::replace(&mut self.curr, NonNull::new(unsafe { tailq_next(curr) }))
    }
}

pub unsafe fn tailq_foreach_reverse<T, D>(head: *mut tailq_head<T>) -> TailqReverseIterator<T, D>
where
    T: Entry<T, D>,
{
    unsafe {
        TailqReverseIterator {
            curr: NonNull::new(tailq_last(head)),
            _phantom: std::marker::PhantomData,
        }
    }
}

// this implementation can be used in place of safe and non-safe
pub struct TailqReverseIterator<T, D> {
    curr: Option<NonNull<T>>,
    _phantom: std::marker::PhantomData<D>,
}
impl<T, D> Iterator for TailqReverseIterator<T, D>
where
    T: Entry<T, D>,
{
    type Item = NonNull<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr?.as_ptr();
        std::mem::replace(&mut self.curr, NonNull::new(unsafe { tailq_prev(curr) }))
    }
}


