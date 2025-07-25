use std::ops::{Deref, DerefMut};

/// Allocate a zero initialized boxed value over a generic type.
pub fn box_array<T>() -> Box<T> {
    unsafe {
        let layout = std::alloc::Layout::new::<T>();
        let ptr = std::alloc::alloc_zeroed(layout);
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Box::from_raw(ptr.cast())
    }
}

/// Wrapper to align the contained value to a 64 byte boundary.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C, align(64))]
pub struct Align64<T>(pub T);

impl<T, const SIZE: usize> Deref for Align64<[T; SIZE]> {
    type Target = [T; SIZE];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, const SIZE: usize> DerefMut for Align64<[T; SIZE]> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Align a slice of data to the given size.
#[inline(always)]
pub fn aligned<T, const SIZE: usize>(data: &[T]) -> &Align64<[T; SIZE]> {
    unsafe {
        debug_assert_eq!(data.len(), SIZE);
        let ptr = data.as_ptr();
        #[allow(clippy::cast_ptr_alignment)]
        &*ptr.cast()
    }
}
