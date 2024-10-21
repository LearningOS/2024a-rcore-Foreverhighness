//! File status definitions

bitflags! {
    /// The mode of a inode
    /// whether a directory or a file
    pub struct StatMode: u32 {
        /// null
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;
        /// ordinary regular file
        const FILE  = 0o100000;
    }
}

/// The stat of a inode
#[repr(C)]
#[derive(Debug)]
pub struct Stat {
    /// ID of device containing file
    pub dev: u64,
    /// inode number
    pub ino: u64,
    /// file type and mode
    pub mode: StatMode,
    /// number of hard links
    pub nlink: u32,
    /// unused pad
    pad: [u64; 7],
}

/// File Status
#[derive(Debug)]
pub struct FileStatus {
    /// inode number
    pub inumber: u64,
    /// file type and mode
    pub mode: StatMode,
    /// number of hard links
    pub num_links: u32,
}

impl From<FileStatus> for Stat {
    fn from(
        FileStatus {
            inumber: ino,
            mode,
            num_links: nlink,
        }: FileStatus,
    ) -> Stat {
        Stat {
            dev: 0,
            ino,
            mode,
            nlink,
            pad: [0; 7],
        }
    }
}
