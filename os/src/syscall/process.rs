//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_task_info, current_user_token, exit_current_and_run_next,
        mmap, munmap, suspend_current_and_run_next, Priority, TaskStatus,
    },
    timer::{get_time_us, MICRO_PER_SEC, MSEC_PER_SEC},
    util::UserSpacePtr,
};

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
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time(ts: 0x{ts:X?})",
        current_task().unwrap().pid.0
    );
    let now_us = get_time_us();
    unsafe {
        UserSpacePtr::from(ts).write(TimeVal {
            sec: now_us / MICRO_PER_SEC,
            usec: now_us % MICRO_PER_SEC,
        });
    }
    0
}

/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
/// get current task info
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!(
        "kernel:pid[{}] sys_task_info(ti: 0x{ti:X?})",
        current_task().unwrap().pid.0
    );
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
    trace!(
        "kernel:pid[{}] sys_mmap(addr: 0x{addr:0X}, len: {len}, prot: 0x{prot:b})",
        current_task().unwrap().pid.0
    );
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
    trace!(
        "kernel:pid[{}] sys_munmap(addr: 0x{addr:0X}, len: {len})",
        current_task().unwrap().pid.0
    );

    let addr_aligned = addr % PAGE_SIZE == 0;

    if addr_aligned {
        return munmap(addr, len);
    }
    -1
}

/// spawn
pub fn sys_spawn(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    trace!(
        "kernel:pid[{}] sys_spawn(path: {path})",
        current_task().unwrap().pid.0
    );

    if let Some(elf_data) = get_app_data_by_name(path.as_str()) {
        let new_task = current_task().unwrap().spawn(elf_data);
        let new_pid = new_task.getpid();
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}

// Set task priority
pub fn sys_set_priority(pri: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority(priority: {pri})",
        current_task().unwrap().pid.0
    );

    if let Ok(priority) = Priority::try_from(pri) {
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .set_priority(priority);

        pri
    } else {
        -1
    }
}
