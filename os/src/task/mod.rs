//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::loader::{get_app_data, get_num_app};
use crate::sync::UPSafeCell;
use crate::timer::get_time_us;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
use task::TaskInfoBlock;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// The task manager inner in 'UPSafeCell'
struct TaskManagerInner {
    /// task list
    tasks: Vec<TaskControlBlock>,
    /// id of current `Running` task
    current_task: usize,
    /// user timer
    user_timer_us: usize,
    /// kernel timer
    kernel_timer_us: usize,
}

lazy_static! {
    /// a `TaskManager` global instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                    user_timer_us: 0,
                    kernel_timer_us: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch4, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();

        trace!("Spawn first task");
        self.update_task_first_run_time();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            self.kernel_timer_stop();

            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);

            self.update_task_first_run_time();
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }

            self.kernel_timer_start();
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }

    /// Get current task info
    fn current_task_info(&self) -> (TaskStatus, TaskInfoBlock) {
        let inner = self.inner.exclusive_access();
        let current_task_no = inner.current_task;
        let task_status = inner.tasks[current_task_no].task_status;
        let task_info_block = inner.tasks[current_task_no].infos.clone();
        (task_status, task_info_block)
    }

    /// Update syscall times
    fn update_syscall_times(&self, syscall_id: usize) {
        let mut inner = self.inner.exclusive_access();
        let current_task_no = inner.current_task;
        let syscall_times = &mut inner.tasks[current_task_no].infos.syscall_times;
        *syscall_times.entry(syscall_id).or_default() += 1;
    }

    /// Start user timer
    fn user_timer_start(&self) {
        let inner = &mut *self.inner.exclusive_access();
        let current_task_no = inner.current_task;
        let now_us = get_time_us();

        trace!("T[{current_task_no}] user timer start at {now_us}");

        let timer_us = &mut inner.user_timer_us;

        assert_eq!(*timer_us, 0, "timer start without reset.");

        *timer_us = now_us;
    }

    /// Stop user timer
    fn user_timer_stop(&self) {
        let inner = &mut *self.inner.exclusive_access();
        let current_task_no = inner.current_task;
        let now_us = get_time_us();

        trace!("T[{current_task_no}] user timer stop at {now_us}us");

        let timer_us = &mut inner.user_timer_us;
        let task_timer = &mut inner.tasks[current_task_no]
            .infos
            .running_times
            .user_time_us;

        assert_ne!(*timer_us, 0, "timer stop without set.");

        let elapsed_us = now_us - *timer_us;
        *task_timer += elapsed_us;
        *timer_us = 0;
    }

    /// Start kernel timer
    fn kernel_timer_start(&self) {
        let inner = &mut *self.inner.exclusive_access();
        let current_task_no = inner.current_task;
        let now_us = get_time_us();

        trace!("T[{current_task_no}] kernel timer start at {now_us}us");

        let timer_us = &mut inner.kernel_timer_us;

        assert_eq!(*timer_us, 0, "timer start without reset.");

        *timer_us = now_us;
    }

    /// Stop kernel timer
    fn kernel_timer_stop(&self) {
        let inner = &mut *self.inner.exclusive_access();
        let current_task_no = inner.current_task;
        let now_us = get_time_us();

        trace!("T[{current_task_no}] kernel timer stop at {now_us}us");

        let timer_us = &mut inner.kernel_timer_us;
        let task_timer = &mut inner.tasks[current_task_no]
            .infos
            .running_times
            .kernel_time_us;

        assert_ne!(*timer_us, 0, "timer stop without set.");

        let elapsed_us = now_us - *timer_us;
        *task_timer += elapsed_us;
        *timer_us = 0;
    }

    /// Update task first run time info
    fn update_task_first_run_time(&self) {
        let mut inner = self.inner.exclusive_access();
        let current_task_no = inner.current_task;

        let first_run_time_us = &mut inner.tasks[current_task_no]
            .infos
            .running_times
            .first_run_time_us;

        if *first_run_time_us == 0 {
            let now_us = get_time_us();
            trace!("T[{current_task_no}] first run at {now_us}us");
            *first_run_time_us = now_us;

            drop(inner);
            self.user_timer_start();
        }
    }

    /// mmap
    fn mmap(&self, addr: usize, len: usize, prot: usize) -> isize {
        todo!()
    }

    /// munmap
    fn munmap(&self, addr: usize, len: usize) -> isize {
        todo!()
    }
}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}

/// Get current task info
pub fn current_task_info() -> (TaskStatus, TaskInfoBlock) {
    TASK_MANAGER.current_task_info()
}

/// Update syscall_times
pub fn update_syscall_times(syscall_id: usize) {
    TASK_MANAGER.update_syscall_times(syscall_id);
}

/// Start user timer
pub fn user_timer_start() {
    TASK_MANAGER.user_timer_start();
}

/// Stop user timer
pub fn user_timer_stop() {
    TASK_MANAGER.user_timer_stop();
}

/// Start kernel timer
pub fn kernel_timer_start() {
    TASK_MANAGER.kernel_timer_start();
}

/// Stop kernel timer
pub fn kernel_timer_stop() {
    TASK_MANAGER.kernel_timer_stop();
}

/// Create memory map for user space
pub fn mmap(addr: usize, len: usize, prot: usize) -> isize {
    TASK_MANAGER.mmap(addr, len, prot)
}

/// Remove memory map for user space
pub fn munmap(addr: usize, len: usize) -> isize {
    TASK_MANAGER.munmap(addr, len)
}
