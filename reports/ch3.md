# 改进的点

`config.rs` 的常量基本都没有带单位，尽管联系上下文可以推测出单位，但在注释中添加单位更一致也更方便理解。  
`usize` 类型存在二义性，建议添加类型别名加以区分。

```rust
// mm/address.rs
/// Physical address
pub type Address = usize;

// config.rs
/// user app's stack size (Byte)
pub const USER_STACK_SIZE: usize = 4096;
/// kernel stack size (Byte)
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
/// kernel heap size (Byte)
pub const KERNEL_HEAP_SIZE: usize = 0x20000;
/// the max number of apps
pub const MAX_APP_NUM: usize = 16;
/// base_addr(changed) of app
pub const APP_BASE_ADDRESS: Address = 0x80400000;
/// size limit of app (Byte)
pub const APP_SIZE_LIMIT: usize = 0x20000;

/// the max number of syscall
pub const MAX_SYSCALL_NUM: usize = 500;
/// clock frequency (per second)
pub const CLOCK_FREQ: usize = 12_500_000;
/// the physical memory end
pub const MEMORY_END: Address = 0x88000000;
```