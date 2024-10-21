use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};

pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,

    inode_id: u32,
}

impl Inode {
    /// We should not acquire efs lock here.
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
        inode_id: u32,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
            inode_id,
        }
    }

    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }

    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }

    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_id() as u32);
            }
        }
        None
    }

    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                    inode_id,
                ))
            })
        })
    }

    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        let op = |root_inode: &mut DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.modify_disk_inode(op).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
            new_inode_id,
        )))
        // release efs lock automatically by compiler
    }

    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }

    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }

    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
}

impl Inode {
    /// Whether this inode is a directory
    pub fn is_dir(&self) -> bool {
        self.read_disk_inode(|disk_inode| disk_inode.is_dir())
    }
    /// Whether this inode is a file
    pub fn is_file(&self) -> bool {
        self.read_disk_inode(|disk_inode| disk_inode.is_file())
    }
    /// Get hard link count
    pub fn links_count(&self) -> u32 {
        self.read_disk_inode(|disk_inode| disk_inode.links_count())
    }
    /// Get inode id
    pub fn inode_id(&self) -> u32 {
        self.inode_id
    }

    /// Return Some(inode_id, index in dir_disk_inode)
    fn find_entry_inode_id_and_index(&self, name: &str) -> Option<(u32, usize)> {
        self.read_disk_inode(|dir_disk_inode| {
            assert!(dir_disk_inode.is_dir());

            let file_count = (dir_disk_inode.size as usize) / DIRENT_SZ;
            let mut entry = DirEntry::empty();
            for idx in 0..file_count {
                let offset = idx * DIRENT_SZ;
                dir_disk_inode.read_at(offset, entry.as_bytes_mut(), &self.block_device);

                if entry.name() == name {
                    return Some((entry.inode_id(), idx));
                }
            }
            None
        })
    }

    // Append file in current directory
    fn append_dirent(&self, name: &str, inode_id: u32, fs: &mut MutexGuard<EasyFileSystem>) {
        self.modify_disk_inode(|dir_disk_inode| {
            assert!(dir_disk_inode.is_dir());

            // append file in the dirent
            let file_count = (dir_disk_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, dir_disk_inode, fs);
            // write dirent
            let dirent = DirEntry::new(name, inode_id);
            dir_disk_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });
    }

    // Swap remove file in current directory
    fn swap_remove_dirent(&self, idx: usize) {
        self.modify_disk_inode(|dir_disk_inode| {
            assert!(dir_disk_inode.is_dir());

            let file_count = (dir_disk_inode.size as usize) / DIRENT_SZ;
            let last_dir_offset = (file_count - 1) * DIRENT_SZ;
            let new_size = last_dir_offset as u32;

            assert!(idx < file_count);

            // write last dir entry to index
            let mut last_dir = DirEntry::empty();
            dir_disk_inode.read_at(last_dir_offset, last_dir.as_bytes_mut(), &self.block_device);
            dir_disk_inode.write_at(idx * DIRENT_SZ, last_dir.as_bytes(), &self.block_device);

            // decrease size
            dir_disk_inode.decrease_size_to(new_size);
        });
    }

    /// Create new link
    pub fn link_at(&self, old_path: &str, new_path: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();

        let (inode_id, _) = self.find_entry_inode_id_and_index(old_path)?;

        let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(block_offset, DiskInode::new_link);

        self.append_dirent(new_path, inode_id, &mut fs);

        block_cache_sync_all();

        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
            inode_id,
        )))
    }

    /// Remove inode under current inode by name
    pub fn unlink(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();

        let (inode_id, idx) = self.find_entry_inode_id_and_index(name)?;

        let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(block_offset, DiskInode::unlink);

        self.swap_remove_dirent(idx);

        block_cache_sync_all();

        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
            inode_id,
        )))
        // release efs lock automatically by compiler
    }

    /// Deallocate inode itself
    pub fn free(&self) {
        let mut fs = self.fs.lock();

        // Deallocate data
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });

        // Deallocate inode
        fs.dealloc_inode(self.inode_id);

        block_cache_sync_all();
    }
}
