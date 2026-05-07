use crate::{alloc::Alloc, rcobj::RcObjectPtr};
use core::ptr::null_mut;

pub struct ListNode<T>
where
    T: Sized,
{
    data: T,
    prev: *mut ListNode<T>,
    next: *mut ListNode<T>,
}

pub struct List<T>
where
    T: Sized,
{
    alloc: RcObjectPtr<dyn Alloc>,
    first: *mut ListNode<T>,
    last: *mut ListNode<T>,
    size: usize,
}

impl<T> List<T>
where
    T: Sized,
{
    pub fn new(alloc: *mut dyn Alloc) -> List<T> {
        List::<T> {
            alloc: RcObjectPtr::from_raw(alloc),
            first: null_mut(),
            last: null_mut(),
            size: 0,
        }
    }

    fn _dealloc_node(&mut self, node: *mut ListNode<T>) {
        unsafe {
            node.drop_in_place();
            self.alloc.borrow_mut().release(
                node as *mut u8,
                size_of::<ListNode<T>>(),
                align_of::<ListNode<T>>(),
            );
        }
    }

    fn _alloc_node(&mut self, data: T) -> *mut ListNode<T> {
        let ptr: *mut ListNode<T>;

        unsafe {
            ptr = self
                .alloc
                .borrow_mut()
                .alloc(size_of::<ListNode<T>>(), align_of::<ListNode<T>>())
                as *mut ListNode<T>;
            if ptr.is_null() {
                return null_mut();
            }
            ptr.write(ListNode::<T> {
                data: data,
                prev: null_mut(),
                next: null_mut(),
            });
        };

        ptr
    }

    #[must_use]
    pub fn push_back(&mut self, data: T) -> Option<&mut T> {
        let ptr: *mut ListNode<T> = self._alloc_node(data);

        if ptr.is_null() {
            return None;
        }

        if self.first.is_null() {
            self.first = ptr;
        }
        unsafe {
            if !self.last.is_null() {
                (*self.last).next = ptr;
            }
            (*ptr).prev = self.last;
        }
        self.last = ptr;

        self.size += 1;

        Some(unsafe { &mut (*ptr).data })
    }

    #[must_use]
    pub fn push_front(&mut self, data: T) -> Option<&mut T> {
        let ptr: *mut ListNode<T> = self._alloc_node(data);

        if self.first.is_null() {
            self.last = ptr;
        }
        unsafe {
            if !self.first.is_null() {
                (*self.first).prev = ptr;
            }
            (*ptr).next = self.first;
        }
        self.first = ptr;

        self.size += 1;

        Some(unsafe { &mut (*ptr).data })
    }

    pub fn pop_front(&mut self) {
        assert!(!self.first.is_null(), "The list is empty");

        let next = unsafe { (*self.first).next };

        self._dealloc_node(self.first);

        if next.is_null() {
            self.last = null_mut();
        } else {
            unsafe {
                (*next).prev = null_mut();
            }
        }
        self.first = next;

        self.size -= 1;
    }

    pub fn pop_back(&mut self) {
        assert!(!self.last.is_null(), "The list is empty");

        let prev = unsafe { (*self.last).prev };

        self._dealloc_node(self.last);

        if prev.is_null() {
            self.first = null_mut();
        } else {
            unsafe {
                (*prev).next = null_mut();
            }
        }
        self.last = prev;

        self.size -= 1;
    }

    #[must_use]
    pub fn insert_front(&mut self, iter: MutIter<'_, T>, data: T) -> Option<&mut T> {
        assert!(
            core::ptr::from_mut(iter.list) == core::ptr::from_mut(self),
            "List does not match!"
        );

        let ptr = self._alloc_node(data);

        if ptr.is_null() {
            return None;
        }

        unsafe {
            if iter.node.is_null() {
                self.last = ptr;
            } else {
                if iter.node == self.first {
                    self.first = ptr;
                }
                (*iter.node).prev = ptr;
            }
        }

        Some(unsafe { &mut (*ptr).data })
    }

    pub fn remove(&mut self, iter: MutIter<'_, T>) {
        assert!(
            core::ptr::from_mut(iter.list) == core::ptr::from_mut(self),
            "List does not match!"
        );
        let node = iter.node;
        if node == self.first {
            self.first = unsafe { (*node).next }
        }
        if node == self.last {
            self.last = unsafe { (*node).prev }
        }
        unsafe {
            if !(*node).next.is_null() {
                (*(*node).next).prev = (*node).prev;
            }
            if !(*node).prev.is_null() {
                (*(*node).prev).next = (*node).next;
            }
        }
        self._dealloc_node(node);

        self.size -= 1;
    }

    pub fn front_mut(&mut self) -> &mut T {
        assert!(!self.first.is_null(), "The list is empty");
        unsafe { &mut (*self.first).data }
    }

    pub fn back_mut(&mut self) -> &mut T {
        assert!(!self.last.is_null(), "The list is empty");
        unsafe { &mut (*self.last).data }
    }

    pub fn front(&self) -> &T {
        assert!(!self.first.is_null(), "The list is empty");
        unsafe { &(*self.first).data }
    }

    pub fn back(&self) -> &T {
        assert!(!self.last.is_null(), "The list is empty");
        unsafe { &(*self.last).data }
    }

    pub fn begin(&self) -> Iter<'_, T> {
        Iter::new(&self, self.first)
    }

    pub fn begin_mut(&mut self) -> MutIter<'_, T> {
        MutIter::new(self, self.first)
    }

    pub fn end(&self) -> Iter<'_, T> {
        Iter::new(&self, null_mut())
    }

    pub fn end_mut(&mut self) -> MutIter<'_, T> {
        MutIter::new(self, null_mut())
    }

    pub fn size(&self) -> usize {
        return self.size;
    }
}

pub struct Iter<'a, T> {
    list: &'a List<T>,
    node: *const ListNode<T>,
}

impl<'a, T> Iter<'a, T> {
    pub fn new(list: &'a List<T>, node: *const ListNode<T>) -> Self {
        Iter {
            list: list,
            node: node,
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if self.node.is_null() {
            return None;
        }
        if self.node == self.list.last {
            self.node = null_mut();
            return None;
        }
        let data = unsafe { Some(&(*self.node).data) };
        self.node = unsafe { (*self.node).next };
        data
    }
}

impl<'a, T> PartialEq for Iter<'a, T> {
    fn eq(&self, rhs: &Iter<'a, T>) -> bool {
        self.node == rhs.node
    }
}

impl<'a, T> PartialOrd for Iter<'a, T> {
    fn partial_cmp(&self, rhs: &Iter<'a, T>) -> Option<core::cmp::Ordering> {
        if self.node < rhs.node {
            return Some(core::cmp::Ordering::Less);
        }
        if self.node > rhs.node {
            return Some(core::cmp::Ordering::Greater);
        }
        Some(core::cmp::Ordering::Equal)
    }
}

pub struct MutIter<'a, T> {
    list: &'a mut List<T>,
    node: *mut ListNode<T>,
}

impl<'a, T> MutIter<'a, T> {
    pub fn new(list: &'a mut List<T>, node: *mut ListNode<T>) -> Self {
        MutIter {
            list: list,
            node: node,
        }
    }
}

impl<'a, T> Iterator for MutIter<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        if self.node.is_null() {
            return None;
        }
        if self.node == self.list.last {
            self.node = null_mut();
            return None;
        }
        let data = unsafe { Some(&mut (*self.node).data) };
        self.node = unsafe { (*self.node).next };
        data
    }
}

impl<'a, T> PartialEq for MutIter<'a, T> {
    fn eq(&self, rhs: &MutIter<'a, T>) -> bool {
        self.node == rhs.node
    }
}

impl<'a, T> PartialOrd for MutIter<'a, T> {
    fn partial_cmp(&self, rhs: &MutIter<'a, T>) -> Option<core::cmp::Ordering> {
        if self.node < rhs.node {
            return Some(core::cmp::Ordering::Less);
        }
        if self.node > rhs.node {
            return Some(core::cmp::Ordering::Greater);
        }
        Some(core::cmp::Ordering::Equal)
    }
}
