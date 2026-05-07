use core::mem::MaybeUninit;

///
/// Create a null pointer even if the type is a dynamic trait.
///  
pub fn null_ptr_generic<T: ?Sized>() -> *const T {
    unsafe { MaybeUninit::<*const T>::zeroed().assume_init() }
}

///
/// Create a mutable null pointer even if the type is a dynamic trait.
/// 
pub fn null_ptr_mut_generic<T: ?Sized>() -> *mut T {
    unsafe { MaybeUninit::<*mut T>::zeroed().assume_init() }
}
