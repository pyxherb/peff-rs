pub struct ScopeGuard<T>
where
    T: FnOnce() -> (),
{
    callback: Option<T>,
}

impl<T> ScopeGuard<T>
where
    T: FnOnce() -> (),
{
    #[inline]
    pub fn new(callback: T) -> ScopeGuard<T> {
        return ScopeGuard::<T> {
            callback: Some(callback),
        };
    }

    #[inline]
    pub fn release(&mut self) {
        self.callback = None;
    }
}

impl<T> Drop for ScopeGuard<T>
where
    T: FnOnce() -> (),
{
    #[inline]
    fn drop(&mut self) {
        match self.callback.take() {
            Some(callback) => callback(),
            None => (),
        }
    }
}
