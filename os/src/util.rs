//! util.rs - A Utility Rust Library For rcore
//!
//! This  defines a set of utility functions that can be used in rcore
//!

use crate::{mm::translated_byte_buffer, task::current_user_token};

/// *mut u8 point to user space
pub type UserSpacePtr<T> = *mut T;
/// *const u8 point to kernel space
pub type KernelSpaceRef<'k, T> = &'k T;

/// Copy slice from src to dst
pub fn copy_to_user_space<T>(src: KernelSpaceRef<T>, dst: UserSpacePtr<T>) {
    assert!(!dst.is_null());

    let len = core::mem::size_of::<T>();
    let mut src = unsafe { core::slice::from_raw_parts(src as *const T as _, len) };
    let buffers = translated_byte_buffer(current_user_token(), dst as _, len);
    for buffer in buffers {
        let nbytes = buffer.len();
        buffer.copy_from_slice(&src[..nbytes]);
        src = &src[nbytes..];
    }

    assert_eq!(src.len(), 0);
}
