use std::{
    ops::{Index, IndexMut},
    ptr::{copy_nonoverlapping, drop_in_place},
};

use crate::{alloc::Alloc, ptr::null_ptr_mut, rcobj::RcObjectPtr};

pub struct DynArray<T>
where
    T: Sized,
{
    alloc: RcObjectPtr<dyn Alloc>,
    data: *mut T,
    size: usize,
    capacity: usize,
}

impl<T> DynArray<T>
where
    T: Sized,
{
    pub fn new(alloc: *mut dyn Alloc) -> DynArray<T> {
        DynArray::<T> {
            alloc: RcObjectPtr::from_raw(alloc),
            data: null_ptr_mut(),
            size: 0,
            capacity: 0,
        }
    }

    #[must_use]
    fn _shrink_to_fit(&mut self) -> bool {
        if self.capacity == self.size {
            return true;
        }
        unsafe {
            let ptr = self.alloc.borrow_mut().realloc(
                self.data as *mut u8,
                self.capacity * size_of::<T>(),
                align_of::<T>(),
                self.size * size_of::<T>(),
                align_of::<T>(),
            );
            if ptr.is_null() {
                return false;
            }
            self.data = ptr as *mut T;
        }

        self.capacity = self.size;

        true
    }

    #[must_use]
    fn _shrink_capacity(&mut self, new_capacity: usize) -> bool {
        assert!(new_capacity < self.capacity);
        // The operation must be atomic, which means we have to keep old
        // capacity to save the old data which will be discarded for rolling
        // back if the operation is failed.
        let new_data: *mut u8;
        unsafe {
            new_data =
                (*self.alloc.into_raw_mut()).alloc(size_of::<T>() * new_capacity, align_of::<T>());

            if new_data.is_null() {
                return false;
            }

            copy_nonoverlapping(self.data, new_data as *mut T, self.size);
        };

        {
            let mut i = self.size;

            while i > new_capacity {
                unsafe {
                    std::ptr::drop_in_place(self.data.wrapping_add(i));
                }
                i += 1;
            }
        }

        self.capacity = new_capacity;
        self.data = new_data as *mut T;

        true
    }

    #[must_use]
    fn _grow_capacity(&mut self, new_capacity: usize) -> bool {
        assert!(new_capacity > self.capacity);
        unsafe {
            let new_data = (*self.alloc.into_raw_mut()).realloc(
                self.data as *mut u8,
                self.capacity * size_of::<T>(),
                align_of::<T>(),
                new_capacity * size_of::<T>(),
                align_of::<T>(),
            );

            if new_data.is_null() {
                return false;
            }

            self.data = new_data as *mut T;
        }
        self.capacity = new_capacity;
        true
    }

    /// Get grown capacity if an automatic growth of capacity is needed.
    fn _get_grown_capacity(&self, new_size: usize) -> usize {
        if self.capacity == 0 {
            return new_size;
        }
        let new_capacity = self.capacity + (self.capacity >> 1);
        if new_capacity < new_size {
            return new_size;
        }
        new_capacity
    }

    /// Grow the capacity automatically if needed.
    #[must_use]
    fn _auto_grow(&mut self, new_size: usize) -> bool {
        if self.capacity < new_size {
            if !self._grow_capacity(self._get_grown_capacity(new_size)) {
                return false;
            }
        }
        self.size = new_size;
        true
    }

    unsafe fn _move_data(new_data: *mut T, old_data: *mut T, size: usize) {
        if new_data.wrapping_add(size) <= old_data {
            unsafe {
                for i in 0..size + 1 {
                    (*new_data.wrapping_add(i)) = std::ptr::read(old_data.wrapping_add(i));
                }
            }
        } else {
            unsafe {
                for i in size..0 {
                    (*new_data.wrapping_add(i - 1)) = std::ptr::read(old_data.wrapping_add(i - 1));
                }
            }
        }
    }

    unsafe fn _move_data_uninit(new_data: *mut T, old_data: *mut T, size: usize) {
        unsafe {
            std::ptr::copy(old_data, new_data, size);
        }
    }

    #[must_use]
    pub fn reserve(&mut self, capacity: usize) -> bool {
        if self.capacity < capacity {
            return self._grow_capacity(capacity);
        }
        true
    }

    #[must_use]
    pub fn push_back(&mut self, data: T) -> Option<&mut T> {
        let size = self.size;
        let new_size = self.size + 1;
        if !self._auto_grow(new_size) {
            return None;
        }
        self[size] = data;
        Some(&mut self[size])
    }

    #[must_use]
    pub fn push_front(&mut self, data: T) -> Option<&mut T> {
        let size = self.size;
        let new_size = self.size + 1;
        if !self._auto_grow(new_size) {
            return None;
        }
        unsafe {
            Self::_move_data_uninit(self.data.wrapping_add(1), self.data, size);
        }
        self[0] = data;
        Some(&mut self[0])
    }

    #[must_use]
    pub fn shrink_to_fit(&mut self) -> bool {
        return self._shrink_to_fit();
    }

    #[must_use]
    pub fn resize(&mut self, size: usize, data: T) -> bool
    where
        T: Copy,
    {
        if size > self.size {
            if size > self.capacity {
                if !self._auto_grow(size) {
                    return false;
                }
            }
            {
                let mut i = self.size;
                while i < size {
                    unsafe {
                        (*self.data.wrapping_add(i)) = data.clone();
                    }
                    i += 1;
                }
            }
        } else if size < self.size {
            if !self._shrink_capacity(size) {
                return false;
            }
        }
        self.size = size;
        true
    }

    pub fn pop_front(&mut self) {
        unsafe {
            Self::_move_data(self.data, self.data.wrapping_add(1), self.size);
        }
        self.size -= 1;
    }

    pub fn pop_back(&mut self) {
        unsafe {
            drop_in_place(self.data.wrapping_add(self.size - 1));
        }
        self.size -= 1;
    }

    #[must_use]
    pub fn insert(&mut self, index: usize, data: T) -> Option<&mut T> {
        let new_size = self.size + 1;
        if !self._auto_grow(new_size) {
            return None;
        }
        unsafe {
            Self::_move_data_uninit(
                self.data.wrapping_add(index + 1),
                self.data.wrapping_add(index),
                self.size - index,
            );
        }
        self[index] = data;
        Some(&mut self[index])
    }

    pub fn remove(&mut self, iter: MutIter<'_, T>) {
        assert!(
            core::ptr::from_mut(iter.vec) == core::ptr::from_mut(self),
            "Vec does not match!"
        );
    }

    pub fn front_mut(&mut self) -> &mut T {
        assert!(!self.data.is_null(), "The Vec is empty");
        return &mut self[0];
    }

    pub fn back_mut(&mut self) -> &mut T {
        assert!(!self.data.is_null(), "The Vec is empty");
        let index = self.size;
        return &mut self[index - 1];
    }

    pub fn front(&self) -> &T {
        assert!(!self.data.is_null(), "The Vec is empty");
        return &self[0];
    }

    pub fn back(&self) -> &T {
        assert!(!self.data.is_null(), "The Vec is empty");
        let index = self.size;
        return &self[index - 1];
    }

    pub fn begin(&self) -> Iter<'_, T> {
        Iter::new(&self, 0)
    }

    pub fn begin_mut(&mut self) -> MutIter<'_, T> {
        MutIter::new(self, 0)
    }

    pub fn end(&self) -> Iter<'_, T> {
        Iter::new(&self, self.size)
    }

    pub fn end_mut(&mut self) -> MutIter<'_, T> {
        MutIter::new(self, self.size)
    }

    pub fn size(&self) -> usize {
        return self.size;
    }

    pub fn data(&self) -> *const T {
        self.data
    }

    pub fn data_mut(&self) -> *mut T {
        self.data
    }
}

impl<T> Index<usize> for DynArray<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        unsafe { self.data.wrapping_add(index).as_ref().unwrap() }
    }
}

impl<T> IndexMut<usize> for DynArray<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        unsafe { self.data.wrapping_add(index).as_mut().unwrap() }
    }
}

pub struct Iter<'a, T> {
    vec: &'a DynArray<T>,
    index: usize,
}

impl<'a, T> Iter<'a, T> {
    pub fn new(vec: &'a DynArray<T>, index: usize) -> Self {
        Iter {
            vec: vec,
            index: index,
        }
    }

    pub fn index_of(&self) -> usize {
        self.index
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        assert!(self.vec.size() <= self.vec.size());
        if self.index >= self.vec.size() {
            return None;
        }
        self.index += 1;
        Some(&self.vec[self.index])
    }
}

impl<'a, T> PartialEq for Iter<'a, T> {
    fn eq(&self, rhs: &Iter<'a, T>) -> bool {
        (self.vec as *const DynArray<T> == rhs.vec as *const DynArray<T>)
            && (self.index == rhs.index)
    }
}

impl<'a, T> PartialOrd for Iter<'a, T> {
    fn partial_cmp(&self, rhs: &Iter<'a, T>) -> Option<core::cmp::Ordering> {
        if self.vec as *const DynArray<T> < rhs.vec as *const DynArray<T> {
            return Some(core::cmp::Ordering::Less);
        }
        if self.vec as *const DynArray<T> > rhs.vec as *const DynArray<T> {
            return Some(core::cmp::Ordering::Greater);
        }
        if self.index < rhs.index {
            return Some(core::cmp::Ordering::Less);
        }
        if self.index > rhs.index {
            return Some(core::cmp::Ordering::Greater);
        }
        Some(core::cmp::Ordering::Equal)
    }
}

pub struct MutIter<'a, T> {
    vec: &'a mut DynArray<T>,
    index: usize,
}

impl<'a, T> MutIter<'a, T> {
    pub fn new(vec: &'a mut DynArray<T>, index: usize) -> Self {
        MutIter {
            vec: vec,
            index: index,
        }
    }

    pub fn index_of(&self) -> usize {
        self.index
    }
}

impl<'a, T> Iterator for MutIter<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        assert!(self.vec.size() <= self.vec.size());
        if self.index >= self.vec.size() {
            return None;
        }
        let ptr = self.vec.data_mut().wrapping_add(self.index);
        self.index += 1;
        unsafe { Some(&mut *ptr) }
    }
}

impl<'a, T> PartialEq for MutIter<'a, T> {
    fn eq(&self, rhs: &MutIter<'a, T>) -> bool {
        (self.vec as *const DynArray<T> == rhs.vec as *const DynArray<T>)
            && (self.index == rhs.index)
    }
}

impl<'a, T> PartialOrd for MutIter<'a, T> {
    fn partial_cmp(&self, rhs: &MutIter<'a, T>) -> Option<core::cmp::Ordering> {
        if self.vec as *const DynArray<T> < rhs.vec as *const DynArray<T> {
            return Some(core::cmp::Ordering::Less);
        }
        if self.vec as *const DynArray<T> > rhs.vec as *const DynArray<T> {
            return Some(core::cmp::Ordering::Greater);
        }
        if self.index < rhs.index {
            return Some(core::cmp::Ordering::Less);
        }
        if self.index > rhs.index {
            return Some(core::cmp::Ordering::Greater);
        }
        Some(core::cmp::Ordering::Equal)
    }
}
