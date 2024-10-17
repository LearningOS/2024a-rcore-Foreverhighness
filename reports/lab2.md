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
