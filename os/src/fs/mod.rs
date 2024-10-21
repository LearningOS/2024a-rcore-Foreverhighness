//! File trait & inode(dir, file, pipe, stdin, stdout)

mod inode;
mod pipe;
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

pub use inode::{list_apps, open_file, OSInode, OpenFlags};
pub use pipe::{make_pipe, Pipe};
pub use stdio::{Stdin, Stdout};

mod file_status;
pub use file_status::{FileStatus, Stat, StatMode};
pub use inode::{link_at, unlink_at};
