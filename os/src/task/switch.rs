//!Wrap `switch.S` as a function
use super::TaskContext;
use core::arch::global_asm;

global_asm!(include_str!("switch.S"));

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
    ///         let (current_task_cx_ptr, next_task_cx_ptr);
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
