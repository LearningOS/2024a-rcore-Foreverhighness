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
