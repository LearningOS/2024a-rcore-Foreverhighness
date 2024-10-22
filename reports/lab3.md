# 吐槽时间

## Implicit Drop

解引用赋值会触发 Drop, 对裸指针进行操作是 unsafe 行为，需要慎重考虑。

```rust
impl KernelStack {
    pub fn push_on_top<T: Sized>(&self, value: T) -> *mut T {
        let kernel_stack_top = self.get_top();
        let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
        unsafe { ptr_mut.write(value); } // before: *ptr_mut = value;
        ptr_mut
    }
}
```

## Too many drop when calling `__switch`

drop 太多啦！  
我认为这里更应该用 `clippy` 里推荐的方法。

```rust
extern "C" {
    /// `__switch` is effectively an await point, so we should avoid holding `RefCell` across the await point.
    /// There are references in clippy lint.
    /// [clippy::await_holding_refcell_ref](https://rust-lang.github.io/rust-clippy/master/index.html#await_holding_refcell_ref)
    /// [clippy::await_holding_lock](https://rust-lang.github.io/rust-clippy/master/index.html#await_holding_lock)
    /// [clippy::await_holding_invalid_type](https://rust-lang.github.io/rust-clippy/master/index.html#await_holding_invalid_type)
    ///
    /// There is an idiomatic way to remove explicit `drop`s when calling `__switch`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// fn foo() {
    ///     let (current_task_cx_ptr, next_task_cx_ptr) = {
    ///         // logic before __switch
    ///         (current_task_cx_ptr, next_task_cx_ptr)
    ///     };
    ///     unsafe {
    ///         __switch(current_task_cx_ptr, next_task_cx_ptr);
    ///     }
    ///     {
    ///         // logic return from __switch
    ///     }
    /// }
    /// ```
    /// Switch to the context of `next_task_cx_ptr`, saving the current context
    /// in `current_task_cx_ptr`.
    pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);
}
```

## Set priority to isize::MAX, really?

在 `ch5_setprio` 里要求 `set_priority` 可以传入的最大值是 `isize::MAX`, 可以比 `BIG_STRIDE` 还大。  
这样算出来的步长 `BIG_STRIDE / isize::MAX == 0`, 这种行为真的是合理的吗？  
还是说要打补丁，当 `BIG_STRIDE / priority == 0` 时，通过设置步长为 1 来避免这种行为。  
问答作业中使用 8 bits 存储 stride, 于是我最开始在代码实现里就是 u8 来存，但是后来因为不想处理 `u8 / isize` 的情况就改成了 `usize`.

```rust
pub fn main() -> i32 {
    assert_eq!(set_priority(10), 10);
    assert_eq!(set_priority(isize::MAX), isize::MAX);
    assert_eq!(set_priority(0), -1);
    assert_eq!(set_priority(1), -1);
    assert_eq!(set_priority(-10), -1);
    println!("Test set_priority OK!");
    0
}
```

## Incorrect explanation of stride algorithm

尽管不影响理解与实现，但我仍旧要指出的是：文档中对于 Stride 算法中关键变量的定义存在错误。  
文档中弄反了 pass 和 stride 的定义，实际上 pass 才是累加器, stride 是步长。  
这点从这两个单词的意思中也能体现。
