//! Process management syscalls

use crate::{
    config::MAX_SYSCALL_NUM,
    fs::{open_file, OpenFlags},
    mm::{translated_ref, translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next, pid2task,
        suspend_current_and_run_next, SignalAction, SignalFlags, TaskStatus, MAX_SIG,
    },
};
use alloc::{string::String, sync::Arc, vec::Vec};

use crate::config::PAGE_SIZE;
use crate::task::{current_task_info, mmap, munmap, Priority};
use crate::timer::{get_time_us, MICRO_PER_SEC, MSEC_PER_SEC};
use crate::util::UserSpacePtr;

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

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
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

pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    let mut args_vec: Vec<String> = Vec::new();
    loop {
        let arg_str_ptr = *translated_ref(token, args);
        if arg_str_ptr == 0 {
            break;
        }
        args_vec.push(translated_str(token, arg_str_ptr as *const u8));
        unsafe {
            args = args.add(1);
        }
    }
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        let argc = args_vec.len();
        task.exec(all_data.as_slice(), args_vec);
        // return argc because cx.x[10] will be covered with it later
        argc as isize
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

pub fn sys_kill(pid: usize, signum: i32) -> isize {
    trace!("kernel:pid[{}] sys_kill", current_task().unwrap().pid.0);
    if let Some(task) = pid2task(pid) {
        if let Some(flag) = SignalFlags::from_bits(1 << signum) {
            // insert the signal if legal
            let mut task_ref = task.inner_exclusive_access();
            if task_ref.signals.contains(flag) {
                return -1;
            }
            task_ref.signals.insert(flag);
            0
        } else {
            -1
        }
    } else {
        -1
    }
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

pub fn sys_sigprocmask(mask: u32) -> isize {
    trace!(
        "kernel:pid[{}] sys_sigprocmask",
        current_task().unwrap().pid.0
    );
    if let Some(task) = current_task() {
        let mut inner = task.inner_exclusive_access();
        let old_mask = inner.signal_mask;
        if let Some(flag) = SignalFlags::from_bits(mask) {
            inner.signal_mask = flag;
            old_mask.bits() as isize
        } else {
            -1
        }
    } else {
        -1
    }
}

pub fn sys_sigreturn() -> isize {
    trace!(
        "kernel:pid[{}] sys_sigreturn",
        current_task().unwrap().pid.0
    );
    if let Some(task) = current_task() {
        let mut inner = task.inner_exclusive_access();
        inner.handling_sig = -1;
        // restore the trap context
        let trap_ctx = inner.get_trap_cx();
        *trap_ctx = inner.trap_ctx_backup.unwrap();
        // Here we return the value of a0 in the trap_ctx,
        // otherwise it will be overwritten after we trap
        // back to the original execution of the application.
        trap_ctx.x[10] as isize
    } else {
        -1
    }
}

fn check_sigaction_error(signal: SignalFlags, action: usize, old_action: usize) -> bool {
    if action == 0
        || old_action == 0
        || signal == SignalFlags::SIGKILL
        || signal == SignalFlags::SIGSTOP
    {
        true
    } else {
        false
    }
}

pub fn sys_sigaction(
    signum: i32,
    action: *const SignalAction,
    old_action: *mut SignalAction,
) -> isize {
    trace!(
        "kernel:pid[{}] sys_sigaction",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if signum as usize > MAX_SIG {
        return -1;
    }
    if let Some(flag) = SignalFlags::from_bits(1 << signum) {
        if check_sigaction_error(flag, action as usize, old_action as usize) {
            return -1;
        }
        let prev_action = inner.signal_actions.table[signum as usize];
        *translated_refmut(token, old_action) = prev_action;
        inner.signal_actions.table[signum as usize] = *translated_ref(token, action);
        0
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

    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let elf_data = app_inode.read_all();
        let new_task = current_task().unwrap().spawn(&elf_data);
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
