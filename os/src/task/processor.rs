//!Implementation of [`Processor`] and Intersection of control flow
//!
//! Here, the continuous operation of user apps in CPU is maintained,
//! the current running state of CPU is recorded,
//! and the replacement and transfer of control flow of different applications are executed.

use super::__switch;
use super::task::TaskInfoBlock;
use super::{fetch_task, TaskStatus};
use super::{TaskContext, TaskControlBlock};
use crate::sync::UPSafeCell;
use crate::timer::get_time_us;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::*;

/// Processor management structure
#[derive(Debug)]
pub struct Processor {
    ///The task currently executing on the current processor
    current: Option<Arc<TaskControlBlock>>,

    ///The basic control flow of each core, helping to select and switch process
    idle_task_cx: TaskContext,

    /// kernel timer
    kernel_timer_us: usize,

    /// user timer
    user_timer_us: usize,
}

impl Processor {
    ///Create an empty Processor
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
            kernel_timer_us: 0,
            user_timer_us: 0,
        }
    }

    ///Get mutable reference to `idle_task_cx`
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }

    ///Get current task in moving semanteme
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.kernel_timer_stop();

        self.current.take()
    }

    ///Get current task in cloning semanteme
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

///The main part of process execution and scheduling
///Loop `fetch_task` to get the process that needs to run, and switch the process through `__switch`
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();

        assert!(processor.current.is_none());

        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            // release coming task_inner manually
            drop(task_inner);
            // release coming task TCB manually
            processor.current = Some(task);

            processor.update_task_first_run_time();

            // release processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            warn!("no tasks available in run_tasks");
        }
    }
}

/// Get current task through take, leaving a None in its place
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// Get a copy of the current task
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// Get the current user token(addr of page table)
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    task.get_user_token()
}

///Get the mutable reference to trap context of current task
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

///Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }

    kernel_timer_start();
}

impl Processor {
    /// Get current without Arc::clone
    fn current_ref(&self) -> Option<&Arc<TaskControlBlock>> {
        self.current.as_ref()
    }

    /// current pid
    fn current_pid(&self) -> Option<usize> {
        self.current_ref().map(|tcb| tcb.getpid())
    }

    /// Start user timer
    fn user_timer_start(&mut self) {
        let now_us = get_time_us();
        trace!(
            "T[{}] user timer start at {now_us}us",
            self.current_pid().unwrap()
        );

        let timer_us = &mut self.user_timer_us;

        assert_eq!(*timer_us, 0, "user timer start without reset.");

        *timer_us = now_us;
    }

    /// Stop user timer
    fn user_timer_stop(&mut self) {
        let now_us = get_time_us();
        trace!(
            "T[{}] user timer stop at {now_us}us",
            self.current_pid().unwrap()
        );

        let timer_us = &mut self.user_timer_us;
        let task_timer = &mut self
            .current
            .as_ref()
            .unwrap()
            .inner_exclusive_access()
            .infos
            .running_times
            .user_time_us;

        assert_ne!(*timer_us, 0, "user timer stop without set.");

        let elapsed_us = now_us - *timer_us;
        *task_timer += elapsed_us;
        *timer_us = 0;
    }

    /// Start kernel timer
    fn kernel_timer_start(&mut self) {
        let now_us = get_time_us();
        trace!(
            "T[{}] kernel timer start at {now_us}us",
            self.current_pid().unwrap()
        );

        let timer_us = &mut self.kernel_timer_us;

        assert_eq!(*timer_us, 0, "kernel timer start without reset.");

        *timer_us = now_us;
    }

    /// Stop kernel timer
    fn kernel_timer_stop(&mut self) {
        let now_us = get_time_us();
        trace!(
            "T[{}] kernel timer stop at {now_us}us",
            self.current_pid().unwrap()
        );

        let timer_us = &mut self.kernel_timer_us;
        let task_timer = &mut self
            .current
            .as_ref()
            .unwrap()
            .inner_exclusive_access()
            .infos
            .running_times
            .kernel_time_us;

        assert_ne!(*timer_us, 0, "kernel timer stop without set.");

        let elapsed_us = now_us - *timer_us;
        *task_timer += elapsed_us;
        *timer_us = 0;
    }

    /// Update task first run time info
    fn update_task_first_run_time(&mut self) {
        let mut state = self.current.as_ref().unwrap().inner_exclusive_access();
        let first_run_time_us = &mut state.infos.running_times.first_run_time_us;

        if *first_run_time_us != 0 {
            return;
        }

        let now_us = get_time_us();
        trace!("T[{}] first run at {now_us}us", self.current_pid().unwrap());
        *first_run_time_us = now_us;

        drop(state);
        self.user_timer_start();
    }
}

/// Start user timer
pub fn user_timer_start() {
    PROCESSOR.exclusive_access().user_timer_start();
}

/// Stop user timer
pub fn user_timer_stop() {
    PROCESSOR.exclusive_access().user_timer_stop();
}

/// Start kernel timer
pub fn kernel_timer_start() {
    PROCESSOR.exclusive_access().kernel_timer_start();
}

/// Stop kernel timer
pub fn kernel_timer_stop() {
    PROCESSOR.exclusive_access().kernel_timer_stop();
}

/// Get current task info
pub fn current_task_info() -> (TaskStatus, TaskInfoBlock) {
    PROCESSOR
        .exclusive_access()
        .current_ref()
        .unwrap()
        .inner_exclusive_access()
        .task_info()
}

/// Update syscall_times
pub fn update_syscall_times(syscall_id: usize) {
    PROCESSOR
        .exclusive_access()
        .current_ref()
        .unwrap()
        .inner_exclusive_access()
        .update_syscall_times(syscall_id);
}

/// Create memory map for user space
pub fn mmap(addr: usize, len: usize, prot: usize) -> isize {
    PROCESSOR
        .exclusive_access()
        .current_ref()
        .unwrap()
        .inner_exclusive_access()
        .mmap(addr, len, prot)
}

/// Remove memory map for user space
pub fn munmap(addr: usize, len: usize) -> isize {
    PROCESSOR
        .exclusive_access()
        .current_ref()
        .unwrap()
        .inner_exclusive_access()
        .munmap(addr, len)
}
