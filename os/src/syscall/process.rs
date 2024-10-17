//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, current_task_info, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
    timer::{get_time_us, MICRO_PER_SEC, MSEC_PER_SEC},
    util::copy_to_user_space,
};

pub use crate::task::update_syscall_times;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
#[derive(Debug)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts_ptr: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let now_us = get_time_us();
    let ts = TimeVal {
        sec: now_us / MICRO_PER_SEC,
        usec: now_us % MICRO_PER_SEC,
    };
    copy_to_user_space(&ts, ts_ptr);
    0
}

/// get current task info
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti_ptr: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let (status, info) = current_task_info();
    let syscall_times = core::array::from_fn(|syscall_id| {
        info.syscall_times
            .get(&syscall_id)
            .copied()
            .unwrap_or_default()
    });

    let time_ms = {
        let now_us = get_time_us();
        let elapsed = now_us - info.running_times.first_run_time_us;
        elapsed / (MICRO_PER_SEC / MSEC_PER_SEC)
    };
    let ti = TaskInfo {
        status,
        syscall_times,
        time: time_ms,
    };
    copy_to_user_space(&ti, ti_ptr);
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
