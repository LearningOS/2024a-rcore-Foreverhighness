//! File trait & inode(dir, file, pipe, stdin, stdout)

mod file_status;
mod inode;
mod stdio;

use crate::mm::UserBuffer;

/// trait File for all file types
pub trait File: Send + Sync {
    /// the file readable?
    fn readable(&self) -> bool;
    /// the file writable?
    fn writable(&self) -> bool;
    /// read from the file to buf, return the number of bytes read
    fn read(&self, buf: UserBuffer) -> usize;
    /// write to the file from buf, return the number of bytes written
    fn write(&self, buf: UserBuffer) -> usize;
    /// return file status
    fn status(&self) -> FileStatus {
        unimplemented!()
    }
}

pub use file_status::{FileStatus, Stat, StatMode};
pub use inode::{link_at, list_apps, open_file, unlink_at, OSInode, OpenFlags};
pub use stdio::{Stdin, Stdout};
