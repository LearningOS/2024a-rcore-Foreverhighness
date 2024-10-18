//! Process management syscalls
use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    task::{
        change_program_brk, current_task_info, exit_current_and_run_next, mmap, munmap,
        suspend_current_and_run_next, TaskStatus,
    },
    timer::{get_time_us, MICRO_PER_SEC, MSEC_PER_SEC},
    util::UserSpacePtr,
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
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let now_us = get_time_us();
    unsafe {
        UserSpacePtr::from(ts).write(TimeVal {
            sec: now_us / MICRO_PER_SEC,
            usec: now_us % MICRO_PER_SEC,
        });
    }
    0
}

/// get current task info
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
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
    unsafe {
        UserSpacePtr::from(ti).write(TaskInfo {
            status,
            syscall_times,
            time: time_ms,
        });
    }
    0
}

/// mmap
pub fn sys_mmap(addr: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap(addr: 0x{addr:0X}, len: {len}, prot: 0x{prot:b})");
    const PROT_MASK: usize = 0b111;

    let addr_aligned = addr % PAGE_SIZE == 0;
    let valid_prot = (prot & !PROT_MASK) == 0;
    let prot_none = (prot & PROT_MASK) == 0;

    if addr_aligned && valid_prot && !prot_none {
        return mmap(addr, len, prot);
    }
    -1
}

/// munmap
pub fn sys_munmap(addr: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap(addr: 0x{addr:0X}, len: {len})");

    let addr_aligned = addr % PAGE_SIZE == 0;

    if addr_aligned {
        return munmap(addr, len);
    }
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
