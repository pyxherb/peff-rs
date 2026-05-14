use std::{
    alloc::{GlobalAlloc, Layout}, ptr::copy
};

use core::ptr::copy_nonoverlapping;

use crate::{
    rcobj::{RcObject, RcObjectPtr},
    scope_guard::ScopeGuard,
};

use core::ptr::null_mut;

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
        assert!(!ptr.is_null(), "Releasing a null block");

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
        assert!(!ptr.is_null(), "Trying realloc with null pointer");

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
                    return null_mut();
                }
                copy(ptr, p, copy_size);
                self.release(ptr, size, alignment);
                return p;
            }
        }
    }
}

struct LDAllocRecord {
    prev: *mut LDAllocRecord,
    next: *mut LDAllocRecord,
    ptr: *mut u8,
    size: usize,
}

pub struct LDAlloc {
    allocator: RcObjectPtr<dyn Alloc>,
    alloc_records: *mut LDAllocRecord,
}

impl LDAlloc {
    pub fn new(allocator: *mut dyn Alloc) -> LDAlloc {
        return LDAlloc {
            allocator: RcObjectPtr::from_raw(allocator),
            alloc_records: null_mut(),
        };
    }

    pub fn into_ptr(&self) -> *const LDAlloc {
        core::ptr::from_ref(self)
    }

    pub fn into_ptr_mut(&mut self) -> *mut LDAlloc {
        core::ptr::from_mut(self)
    }

    pub fn dump_allocated_blocks(&self) {
        let mut i: *const LDAllocRecord = self.alloc_records;

        while !i.is_null() {
            unsafe {
                println!("Block at {:?}, size = {:?}", (*i).ptr, (*i).size);
            }
            i = unsafe { (*i).next };
        }
    }

    pub fn validate_blocks(&self) {
        let mut i: *const LDAllocRecord = self.alloc_records;

        while !i.is_null() {
            let ptr = unsafe { (*i).ptr };
            let size = unsafe { (*i).size };
            if ptr.is_null() {
                panic!("Allocation records damaged");
            }
            let record_ptr = unsafe {
                (ptr.add(size + size_of::<usize>()) as *mut *mut LDAllocRecord).read_unaligned()
            };
            let canary = unsafe { (ptr.add(size) as *mut usize).read_unaligned() };

            if (ptr as usize) ^ (record_ptr as usize) != canary {
                panic!("Block at {:?} damaged", ptr);
            }

            i = unsafe { (*i).next };
        }
    }
}

impl RcObject for LDAlloc {
    fn inc_ref(&mut self) {}
    fn dec_ref(&mut self) {}
}

unsafe impl Alloc for LDAlloc {
    unsafe fn alloc(&mut self, size: usize, alignment: usize) -> *mut u8 {
        let blk: *mut u8;
        unsafe {
            blk = self.allocator.borrow_mut().alloc(
                size + size_of::<usize>() + size_of::<*mut LDAllocRecord>(),
                alignment,
            );
        }
        if blk.is_null() {
            return null_mut();
        }

        let mut release_blk_guard = ScopeGuard::new(|| unsafe {
            self.allocator.borrow_mut().release(blk, size, alignment);
        });

        let record = LDAllocRecord {
            prev: null_mut(),
            next: self.alloc_records,
            ptr: blk,
            size: size,
        };

        let record_ptr: *mut LDAllocRecord;
        unsafe {
            record_ptr = self
                .allocator
                .borrow_mut()
                .alloc(size_of::<LDAllocRecord>(), align_of::<LDAllocRecord>())
                as *mut LDAllocRecord;
            if record_ptr.is_null() {
                return null_mut();
            }
            record_ptr.write(record);
        }
        if !self.alloc_records.is_null() {
            unsafe {
                (*self.alloc_records).prev = record_ptr;
            }
        }
        self.alloc_records = record_ptr;

        let canary_ptr_in_blk = blk.wrapping_add(size) as *mut usize;
        let record_ptr_in_blk =
            blk.wrapping_add(size + size_of::<usize>()) as *mut *mut LDAllocRecord;

        unsafe {
            canary_ptr_in_blk.write_unaligned((blk as usize) ^ (record_ptr as usize));
            record_ptr_in_blk.write_unaligned(record_ptr);
        };

        if (blk as usize) ^ (record_ptr as usize) == 1858626043582usize {
            println!("Test");
        }

        release_blk_guard.release();
        blk
    }

    unsafe fn release(&mut self, ptr: *mut u8, size: usize, alignment: usize) {
        if ptr.is_null() {
            panic!("Releasing a null block");
        }
        let record_ptr = unsafe {
            (ptr.wrapping_add(size + size_of::<usize>()) as *mut *mut LDAllocRecord).read_unaligned()
        };
        let canary = unsafe { (ptr.wrapping_add(size) as *mut usize).read_unaligned() };

        unsafe {
            if (*record_ptr).ptr != ptr {
                panic!(
                    "Releasing at {:?} which is not a previously allocated block",
                    ptr
                );
            }
            if (*record_ptr).size != size {
                panic!(
                    "Releasing at {:?} where size {} does not match the previously allocated size {}",
                    ptr,
                    size,
                    (*record_ptr).size
                );
            }

            if (((*record_ptr).ptr as usize) ^ (record_ptr as usize)) != canary {
                panic!("Block at {:?} damaged", ptr);
            }

            if self.alloc_records == record_ptr {
                self.alloc_records = (*record_ptr).next;
            }

            if !(*record_ptr).prev.is_null() {
                (*(*record_ptr).prev).next = (*record_ptr).next;
            }

            if !(*record_ptr).next.is_null() {
                (*(*record_ptr).next).prev = (*record_ptr).prev;
            }
        };

        unsafe {
            self.allocator.borrow_mut().release(
                ptr,
                size + size_of::<usize>() + size_of::<*mut LDAllocRecord>(),
                alignment,
            );
            self.allocator.borrow_mut().release(
                record_ptr as *mut u8,
                size_of::<LDAllocRecord>(),
                align_of::<LDAllocRecord>(),
            );
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
        if ptr.is_null() {
            panic!("Trying realloc with null pointer");
        }

        let copy_size;

        if new_size > size {
            copy_size = size;
        } else {
            copy_size = new_size;
        }

        unsafe {
            let p = self.alloc(new_size, new_alignment);
            if p.is_null() {
                return null_mut();
            }
            copy_nonoverlapping::<u8>(ptr, p, copy_size);
            self.release(ptr, size, alignment);
            self.validate_blocks();
            return p;
        }
    }
}
