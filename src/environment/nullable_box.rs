use std::ops::{Deref, DerefMut};

/// A [`Box<T>`] that can be nulled out in unsafe code.
///
/// In effect, this is basically just a [`Option<Box<T>>`] where derefrencing
/// None is unsafe.
///
/// This is needed for when threads yield and need to give back access to
/// environment data to the executor (for other threads). To do this, the inner
/// fields are moved out, become null and the environment is sent back to the
/// executor. There should be no reason to use this outside of that, though.
pub struct NullableBox<T> {
    inner: Option<Box<T>>,
}

impl<T> NullableBox<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: Some(Box::new(inner)),
        }
    }

    /// Make a new null (empty) NullableBox.
    ///
    /// SAFETY: You **MUST NEVER** allow this to be derefrenced, which means
    /// that this **MUST NOT** escape into safe code.
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
