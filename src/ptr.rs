use core::mem::MaybeUninit;

pub fn null_ptr<T: ?Sized>() -> *const T {
    unsafe { MaybeUninit::<*const T>::zeroed().assume_init() }
}

pub fn null_ptr_mut<T: ?Sized>() -> *mut T {
    unsafe { MaybeUninit::<*mut T>::zeroed().assume_init() }
}
