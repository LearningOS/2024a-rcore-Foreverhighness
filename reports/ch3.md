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
/// clock frequency (ticks per second)
pub const CLOCK_FREQ: usize = 12_500_000;
/// the physical memory end
pub const MEMORY_END: Address = 0x88000000;
```

经过单位调整可以发现 `timer.rs` 中存在单位出错的情况。

```rust
const TICKS_PER_SEC: usize = 100; // The number of ticks per second
pub fn get_time() -> usize; // Get the current time in ticks
set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
```

`get_time` 返回的是 `tick`, `CLOCK_FREQ` 的单位是 `tick/s`,  `TICKS_PER_SEC` 的单位也是 `tick/s`.  
后面的式子运算后就没有单位了，实际上是不合法的运算。  
而这个式子的实际意义是得到 10ms 后的 tick, 这一点在代码里完全没有体现，也没有注释指出这一点，仅在文档中提到。  
可以说 `TICKS_PER_SEC` 完全是一个 magic number.  
个人推荐删了或者添加正确的 `TICKS_PER_MSEC` 常量。

```rust
/// Timer Tick
pub type Tick = usize;
/// Get the current time in ticks
pub fn get_time_tick() -> Tick {
    time::read()
}
/// Set the next timer interrupt
pub fn set_next_trigger() {
    set_timer(get_time_tick() + CLOCK_FREQ * 10 / MSEC_PER_SEC); // set timer after 10ms
}
```
