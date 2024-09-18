use std::ops::{Deref, DerefMut};

// BEFOREMERGE: Document the reason that this needs to exist
pub struct NullableBox<T> {
    inner: Option<Box<T>>,
}

impl<T> NullableBox<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: Some(Box::new(inner)),
        }
    }

    // BEFOREMERGE: Document safety conditions properly.
    pub unsafe fn null() -> Self {
        Self { inner: None }
    }

    pub fn into_inner(self) -> T {
        debug_assert!(self.inner.is_some(), "NullableBox derefed on None!");
        unsafe { *self.inner.unwrap_unchecked() }
    }
}

impl<T> Default for NullableBox<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Deref for NullableBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        debug_assert!(self.inner.is_some(), "NullableBox derefed on None!");
        unsafe { self.inner.as_ref().unwrap_unchecked() }
    }
}

impl<T> DerefMut for NullableBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(self.inner.is_some(), "NullableBox derefed on None!");
        unsafe { self.inner.as_mut().unwrap_unchecked() }
    }
}

impl<T> AsRef<T> for NullableBox<T> {
    fn as_ref(&self) -> &T {
        debug_assert!(self.inner.is_some(), "NullableBox derefed on None!");
        unsafe { self.inner.as_ref().unwrap_unchecked() }
    }
}

impl<T> AsMut<T> for NullableBox<T> {
    fn as_mut(&mut self) -> &mut T {
        debug_assert!(self.inner.is_some(), "NullableBox derefed on None!");
        unsafe { self.inner.as_mut().unwrap_unchecked() }
    }
}
