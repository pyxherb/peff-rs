use crate::base::{alloc::Alloc, rcobj::RcObjectPtr};
use core::ptr::null_mut;

pub struct Box<T> {
    ptr: *mut T,
    alloc: RcObjectPtr<dyn Alloc>,
}

impl<T> Box<T> {
    pub fn new(alloc: *mut dyn Alloc, data: T) -> Box<T> {
        unsafe {
            let ptr = (*alloc).alloc(size_of::<T>(), align_of::<T>()) as *mut T;

            ptr.write(data);

            Box::<T> {
                ptr,
                alloc: RcObjectPtr::<dyn Alloc>::from_raw(alloc),
            }
        }
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
                self.ptr.drop_in_place();
                self.alloc.borrow_mut().release(
                    self.ptr as *mut u8,
                    size_of::<T>(),
                    align_of::<T>(),
                );
            };
            self.ptr = null_mut();
        }
    }

    pub fn release(&mut self) -> *mut T {
        let tmp = self.ptr;
        self.ptr = null_mut();
        tmp
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        self.reset();
    }
}
