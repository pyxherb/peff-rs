use std::{
    alloc::{GlobalAlloc, Layout},
    ptr::copy,
};

use crate::{ptr::null_ptr_mut, rcobj::RcObject};

///
/// A polymorphic allocator, dedicated on reducing size of the generated codes.
///
pub unsafe trait Alloc: RcObject {
    unsafe fn alloc(&mut self, size: usize, alignment: usize) -> *mut u8;
    unsafe fn release(&mut self, ptr: *mut u8, size: usize, alignment: usize);
    unsafe fn realloc(
        &mut self,
        ptr: *mut u8,
        old_size: usize,
        old_alignment: usize,
        new_size: usize,
        new_alignment: usize,
    ) -> *mut u8;
}

#[cfg(feature = "std")]
pub struct StdAlloc {
    allocator: std::alloc::System,
}

impl StdAlloc {
    pub fn new() -> StdAlloc {
        return StdAlloc {
            allocator: std::alloc::System {},
        };
    }

    pub fn into_ptr(&self) -> *const StdAlloc {
        core::ptr::from_ref(self)
    }
    
    pub fn into_ptr_mut(&mut self) -> *mut StdAlloc {
        core::ptr::from_mut(self)
    }
}

impl RcObject for StdAlloc {
    fn inc_ref(&mut self) {}
    fn dec_ref(&mut self) {}
}

unsafe impl Alloc for StdAlloc {
    unsafe fn alloc(&mut self, size: usize, alignment: usize) -> *mut u8 {
        unsafe {
            self.allocator
                .alloc(Layout::from_size_align(size, alignment).unwrap())
        }
    }

    unsafe fn release(&mut self, ptr: *mut u8, size: usize, alignment: usize) {
        unsafe {
            self.allocator
                .dealloc(ptr, Layout::from_size_align(size, alignment).unwrap());
        }
    }

    unsafe fn realloc(
        &mut self,
        ptr: *mut u8,
        size: usize,
        alignment: usize,
        new_size: usize,
        new_alignment: usize,
    ) -> *mut u8 {
        if alignment == new_alignment {
            unsafe {
                return self.allocator.realloc(
                    ptr,
                    Layout::from_size_align(size, alignment).unwrap(),
                    new_size,
                );
            }
        } else {
            let copy_size;

            if new_size > size {
                copy_size = size;
            } else {
                copy_size = new_size;
            }

            unsafe {
                let p = self.alloc(new_size, new_alignment);
                if p.is_null() {
                    return null_ptr_mut();
                }
                copy(ptr, p, copy_size);
                self.release(ptr, size, alignment);
                return p;
            }
        }
    }
}
