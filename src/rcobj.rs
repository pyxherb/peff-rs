use crate::ptr::null_ptr_mut_generic;

pub trait RcObject {
    fn inc_ref(&mut self);
    fn dec_ref(&mut self);
}

pub struct RcObjectPtr<T>
where
    T: RcObject + ?Sized,
{
    ptr: *mut T,
}

impl<T> RcObjectPtr<T>
where
    T: RcObject + ?Sized,
{
    pub fn new(ptr: &mut T) -> RcObjectPtr<T> {
        (*ptr).inc_ref();
        RcObjectPtr { ptr: ptr }
    }

    pub fn from_raw(ptr: *mut T) -> RcObjectPtr<T> {
        unsafe {
            (*ptr).inc_ref();
        }
        RcObjectPtr { ptr: ptr }
    }

    pub fn into_raw(&self) -> *const T {
        self.ptr
    }

    pub fn into_raw_mut(&self) -> *mut T {
        self.ptr
    }

    pub fn borrow(&self) -> &T {
        unsafe { self.ptr.as_ref().unwrap() }
    }

    pub fn borrow_mut(&self) -> &mut T {
        unsafe { self.ptr.as_mut().unwrap() }
    }

    pub fn reset(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                (*self.ptr).dec_ref();
            };
            self.ptr = null_ptr_mut_generic();
        }
    }

    pub fn release(&mut self) -> *mut T {
        let tmp = self.ptr;
        self.ptr = null_ptr_mut_generic();
        tmp
    }
}

impl<T> Clone for RcObjectPtr<T>
where
    T: RcObject + ?Sized,
{
    fn clone(&self) -> RcObjectPtr<T> {
        if !self.ptr.is_null() {
            unsafe {
                (*self.ptr).inc_ref();
            }
        }
        RcObjectPtr { ptr: self.ptr }
    }
}

impl<T> Drop for RcObjectPtr<T>
where
    T: RcObject + ?Sized,
{
    fn drop(&mut self) {
        self.reset();
    }
}
