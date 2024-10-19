# 吐槽时间

## 我看到就会死的十六进制数

```rust
/// page size : 4KB
pub const PAGE_SIZE: usize = 0x1000;
/// page size bits: 12
pub const PAGE_SIZE_BITS: usize = 0xc;
```

把 12 写作 0xc 是不是很 cool?  
但是我看到意味不明的十六进制数就会暴毙。  
MD忍不了了，一拳把 rcore 打爆！

## Magic number が多すぎる！

本小节代码截取自 [`ch4`](https://github.com/LearningOS/template-2024a-rcore/tree/ch4) 分支。  
commit hash: 1b46614b80a0ecf41320f57a2eb7f8ccdb5947eb

[`512`](https://github.com/LearningOS/template-2024a-rcore/blob/ch4/os/src/mm/address.rs#L182) 实际是 `PAGE_SIZE / sizeof::<PageTableEntry>` 或者 `1 << PPN_WIDTH`.  
[`4096`](https://github.com/LearningOS/template-2024a-rcore/blob/ch4/os/src/mm/address.rs#L187) 实际是 `PAGE_SIZE`.

```rust
impl PhysPageNum {
    pub fn get_pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512) }
    }
    pub fn get_bytes_array(&self) -> &'static mut [u8] {
        let pa: PhysAddr = (*self).into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096) }
    }
}
```

[`3`](https://github.com/LearningOS/template-2024a-rcore/blob/ch4/os/src/mm/address.rs#L164) 实际上是 `PAGE_TABLE_LEVELS`.  
[`511`](https://github.com/LearningOS/template-2024a-rcore/blob/ch4/os/src/mm/address.rs#L164) 实际是 `(1 << VPN_WIDTH) - 1`.  
[`9`](https://github.com/LearningOS/template-2024a-rcore/blob/ch4/os/src/mm/address.rs#L165) 实际是 `VPN_WIDTH` 或者 `(PAGE_SIZE / sizeof::<PageTableEntry>)。log2()`.

```rust
impl VirtPageNum {
    /// Get the indexes of the page table entry
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        for i in (0..3).rev() {
            idx[i] = vpn & 511;
            vpn >>= 9;
        }
        idx
    }
}
```

[`[0x7f, 0x45, 0x4c, 0x46]`](https://github.com/LearningOS/template-2024a-rcore/blob/ch4/os/src/mm/memory_set.rs#L156) 可以换为 `xmas_elf::header::MAGIC`.

```rust
impl MemorySet {
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        ...
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        ...
    }
}
```

## 实现了 Copy trait 标量类型的方法可以不必传引用

参考 [`i32::abs()`](https://doc.rust-lang.org/std/primitive.i32.html#method.abs), [`usize::saturating_add()`](https://doc.rust-lang.org/std/primitive.usize.html#method.saturating_add) 等方法可以发现，对于实现了 Copy trait 的类型而言，其方法可以直接进行值传递而无需传递引用。  
更改后的代码:

```rust
impl PhysPageNum {
    /// Get the reference of page table(array of ptes)
    pub fn get_pte_array(self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = self.into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, 512) }
    }
    /// Get the reference of page(array of bytes)
    pub fn get_bytes_array(self) -> &'static mut [u8] {
        let pa: PhysAddr = self.into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, 4096) }
    }
    /// Get the mutable reference of physical address
    pub fn get_mut<T>(self) -> &'static mut T {
        let pa: PhysAddr = self.into();
        pa.get_mut()
    }
}
```

## 没有价值的 Arc

[`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html) 的实现需要原子操作的支持，在 ch4 出现有点不合时宜。  
更重要的是，它在这里完全没有起到一点作用。  
更改后的代码:

```rust
lazy_static! {
    /// The kernel's initial memory mapping(kernel address space)
    pub static ref KERNEL_SPACE: UPSafeCell<MemorySet> =
        unsafe { UPSafeCell::new(MemorySet::new_kernel()) };
}
```

## 使用库接口

`PageTableEntry` 里的辅助方法使用库函数更容易理解。

```rust
impl PageTableEntry {
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::V)
    }
    pub fn readable(&self) -> bool {
        self.flags().contains(PTEFlags::R)
    }
    pub fn writable(&self) -> bool {
        self.flags().contains(PTEFlags::W)
    }
    pub fn executable(&self) -> bool {
        self.flags().contains(PTEFlags::X)
    }
}
```

`MemorySet::from_elf()` 里的 [`check` 部分](https://github.com/LearningOS/template-2024a-rcore/blob/ch4/os/src/mm/memory_set.rs#L156) 完全可以替换为 [`xmas_elf::header::sanity_check()`](https://docs.rs/xmas-elf/latest/src/xmas_elf/header.rs.html#428-453)

## StackFrameAllocator

StackFrameAllocator 实际上拥有着从 current 到 end 这一段内存的所有权，但是代码和文档里均没有体现。  
StackFrameAllocator 中 current 和 end 实际是应该是 PhysPageNum 类型。  
FrameTracker 并不是一个很恰当的命名，我个人觉得可以改为 PageAllocated 或者 PageOwned.  
更改后的代码:

```rust
pub struct StackFrameAllocator {
    current: PhysPageNum,       // usize -> PhysPageNum
    end: PhysPageNum,           // usize -> PhysPageNum
    recycled: Vec<PhysPageNum>, // usize -> PhysPageNum
}
```

## MemorySet with more strict constrain

代码中实际上有两种 MemorySet, 一种是 Kernel MemorySet, 一种是 User MemorySet.  
User MemorySet 实际上只能接收 User Space 里的虚拟地址。  
这样还有一个好处是可以自动添加 User 权限。  
更改后的代码:

```rust
pub struct MemorySet<T: VAddr> {
    page_table: PageTable<T>,
    areas: Vec<MapArea<T>>,
}
impl<T: VAddr> MemorySet<T> {
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }
}
impl MemorySet<KernelVAddr> {
    pub fn insert_area(
        &mut self,
        start_va: KernelVAddr,
        end_va: KernelVAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Identical, permission),
            None,
        );
    }
}
impl MemorySet<UserVAddr> {
    pub fn insert_area(
        &mut self,
        start_va: UserVAddr,
        end_va: UserVAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission | MapPermission::U),
            None,
        );
    }
}
```