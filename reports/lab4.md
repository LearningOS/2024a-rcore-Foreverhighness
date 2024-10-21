# 吐槽时间

## `drain` is not cool

这里用 `drain` 是不是比用 `remove` 要 cool?  
Bro, 这并不好笑，用 `remove` 才对。

```rust
impl BlockCacheManager {
    pub fn get_block_cache(&mut self) -> Arc<Mutex<BlockCache>> {
        // ...
        // substitute
        if self.queue.len() == BLOCK_CACHE_SIZE {
            // from front to tail
            if let Some((idx, _)) = self
                .queue
                .iter()
                .enumerate()
                .find(|(_, pair)| Arc::strong_count(&pair.1) == 1)
            {
                self.remove(idx); // self.queue.drain(idx..=idx);
            } else {
                panic!("Run out of BlockCache!");
            }
        }
        // ...
    }
}
```

## No `pair` please!

我受够了 `pair` 了！这里不是 `C++`, 我们不用带着 `pair` 到处跑。  
`Rust` 语法的表达力很足，可以直接解包，这样代码更容易理解。

```rust
impl BlockCacheManager {
    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some((_, block_cache)) = self.queue.iter().find(|&&(id, _)| id == block_id) {
            Arc::clone(block_cache)
        } else {
            // substitute
            if self.queue.len() == BLOCK_CACHE_SIZE {
                // from front to tail
                if let Some((idx, _)) = self
                    .queue
                    .iter()
                    .enumerate()
                    .find(|(_, (_, block_cache))| Arc::strong_count(block_cache) == 1)
                {
                    self.queue.drain(idx..=idx);
                } else {
                    panic!("Run out of BlockCache!");
                }
            }
            // load block into mem and push back
            let block_cache = Arc::new(Mutex::new(BlockCache::new(
                block_id,
                Arc::clone(&block_device),
            )));
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

impl MemorySet {
    pub fn new_kernel() -> Self {
        ...
        for &(mmio_start_va, mmio_len) in MMIO {
            memory_set.push(MapArea::new(
                mmio_start_va.into(),
                (mmio_start_va + mmio_len).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ), None);
        }
        memory_set
    }
}
```

## static assertion

虽然 Rust 中并没有自带的 static_assert 函数，但是可以通过小技巧实现等价的效果。

```rust
pub struct DiskInode {
    pub size: u32,
    pub direct: [u32; INODE_DIRECT_COUNT],
    pub indirect1: u32,
    pub indirect2: u32,
    type_: DiskInodeType,
}
const _: () = assert!(core::mem::size_of::<DiskInode>() == 128);
```


## Pointer to reference conversion

文档中可以添加和 unsafe 相关的的这个文档链接 [Pointer to reference conversion](https://doc.rust-lang.org/std/ptr/index.html#pointer-to-reference-conversion).

```rust
impl BlockCache {
    pub fn get_ref<T>(&self, offset: usize) -> &T where T: Sized {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) }
    }
    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T where T: Sized {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        self.modified = true;
        let addr = self.addr_of_offset(offset);
        unsafe { &mut *(addr as *mut T) }
    }
}
```
