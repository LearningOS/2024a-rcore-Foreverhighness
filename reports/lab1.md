# 吐槽时间

## config.rs

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

## timer.rs

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

## lazy_static

自 Rust 1.80 起，`lazy_static` 的功能可以被 `core::cell:LazyCell` 所代替，可以少一项依赖。

更重要的是在我本地 `lazy_static` 包裹的部分无法被正确格式化，很不爽。

我在本地自测了一下，可以通过编译。

```rust
// sync/up.rs
pub struct UPSafeWrapper<T>(T);
unsafe impl<T> Sync for UPSafeWrapper<T> {}
impl<T> Deref for UPSafeWrapper<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> UPSafeWrapper<T> {
    /// mark it const
    pub const unsafe fn new(value: T) -> Self {
        Self(value)
    }
}

// task/mod.rs
pub struct TaskManager {
    num_app: usize,
    inner: RefCell<TaskManagerInner>, // UPSafeCell -> RefCell
}
pub static TASK_MANAGER: UPSafeWrapper<LazyCell<TaskManager>> = unsafe {
    UPSafeWrapper::new(LazyCell::new(|| {
        let num_app = get_num_app();
        trace!("Init task manager num_app: {num_app}");
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: RefCell::new(TaskManagerInner {
                tasks,
                current_task: 0,
            }),
        }
    }))
};
```

## Clippy

`cargo clippy` 无法通过检查, `clippy` 是很重要很有用的工具，应当予以重视。

## Assembly/Rust interaction

OS 课程中与汇编打交道是不得不品尝的一环，但是现在已经有一些工具可以让我们提高汇编的可读性。

比如用 `gcc` 编译汇编，就能在汇编代码中使用 `#include`, `#define` 等宏定义增强汇编的表达力。

事实上[大写的 `.S` 后缀就通常代表这个汇编代码需要被 `gcc` 进行预处理][1]。

我在看到实验里汇编代码里居然没有使用 `gcc` 拓展时还挺惊讶的。


使用 [`naked function`][2] 特性可以代替 [`global_asm!`][4], 也容易与 Rust 代码进行交互，其 [RFC][3] 中提到了相较于 `global_asm!` 的优缺点。

缺点是纯汇编更容易与现有的绝大部分教材相联系，并且 `naked function` 特性到目前还没有稳定。

我个人认为 `__switch` 非常适合写成 `naked function`.

