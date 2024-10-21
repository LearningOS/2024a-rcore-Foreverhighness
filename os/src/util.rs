//! util.rs - A Utility Rust Library For rcore
//!
//! This  defines a set of utility functions that can be used in rcore
//!

use core::{mem::size_of, ptr::addr_of};

use alloc::vec::Vec;

use crate::{mm::translated_byte_buffer, task::current_user_token};

/// User space ptr wrapper but correctly supply `write` and `read` interface.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserSpacePtr<T>(*mut T);

impl<T: Sized> UserSpacePtr<T> {
    /// Overwrites a memory location with the given value without reading or
    /// dropping the old value.
    ///
    /// See [`core::ptr::write`] for safety concerns and examples.
    ///
    /// [`core::ptr::write`]: core::ptr::write()
    pub unsafe fn write(self, val: T)
    where
        T: Sized,
    {
        let buffers = self.into_buffers();
        let mut src = unsafe { core::slice::from_raw_parts(addr_of!(val) as _, size_of::<T>()) };

        assert_eq!(src.len(), buffers.iter().map(|v| v.len()).sum());
        for buffer in buffers {
            let nbytes = buffer.len();
            buffer.copy_from_slice(&src[..nbytes]);
            src = &src[nbytes..];
        }
        assert_eq!(src.len(), 0)
    }

    /// Reads the value from `self` without moving it. This leaves the
    /// memory in `self` unchanged.
    ///
    /// See [`core::ptr::read`] for safety concerns and examples.
    ///
    /// [`core::ptr::read`]: core::ptr::read()
    pub unsafe fn read(self) -> T
    where
        T: Sized,
    {
        todo!()
    }

    fn into_buffers(self) -> Vec<&'static mut [u8]> {
        translated_byte_buffer(current_user_token(), self.0 as _, size_of::<T>())
    }
}

impl<T> From<*mut T> for UserSpacePtr<T> {
    fn from(value: *mut T) -> Self {
        Self(value)
    }
}
