//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

/// unlinkat syscall
const SYSCALL_UNLINKAT: usize = 35;
/// linkat syscall
const SYSCALL_LINKAT: usize = 37;
/// open syscall
const SYSCALL_OPEN: usize = 56;
/// close syscall
const SYSCALL_CLOSE: usize = 57;
/// read syscall
const SYSCALL_READ: usize = 63;
/// write syscall
const SYSCALL_WRITE: usize = 64;
/// fstat syscall
const SYSCALL_FSTAT: usize = 80;
/// exit syscall
const SYSCALL_EXIT: usize = 93;
/// yield syscall
const SYSCALL_YIELD: usize = 124;
/// setpriority syscall
const SYSCALL_SET_PRIORITY: usize = 140;
/// gettime syscall
const SYSCALL_GET_TIME: usize = 169;
/// getpid syscall
const SYSCALL_GETPID: usize = 172;
/// sbrk syscall
const SYSCALL_SBRK: usize = 214;
/// munmap syscall
const SYSCALL_MUNMAP: usize = 215;
/// fork syscall
const SYSCALL_FORK: usize = 220;
/// exec syscall
const SYSCALL_EXEC: usize = 221;
/// mmap syscall
const SYSCALL_MMAP: usize = 222;
/// waitpid syscall
const SYSCALL_WAITPID: usize = 260;
/// spawn syscall
const SYSCALL_SPAWN: usize = 400;
/// taskinfo syscall
const SYSCALL_TASK_INFO: usize = 410;

mod fs;
mod process;

use fs::*;
use process::*;

use crate::fs::Stat;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 4]) -> isize {
    let _guard = SyscallChecker::new(syscall_id, args);

    update_syscall_times(syscall_id);
    match syscall_id {
        SYSCALL_OPEN => sys_open(args[1] as *const u8, args[2] as u32),
        SYSCALL_CLOSE => sys_close(args[0]),
        SYSCALL_LINKAT => sys_linkat(args[1] as *const u8, args[3] as *const u8),
        SYSCALL_UNLINKAT => sys_unlinkat(args[1] as *const u8),
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_FSTAT => sys_fstat(args[0], args[1] as *mut Stat),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TASK_INFO => sys_task_info(args[0] as *mut TaskInfo),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_SBRK => sys_sbrk(args[0] as i32),
        SYSCALL_SPAWN => sys_spawn(args[0] as *const u8),
        SYSCALL_SET_PRIORITY => sys_set_priority(args[0] as isize),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}

use crate::{sync::UPSafeCell, task::update_syscall_times};

/// Checker state in syscall
struct SyscallCheckerState {
    // assert all sys_func will return
    counter: alloc::collections::BTreeMap<usize, i32>,
}

impl SyscallCheckerState {
    /// Construct a syscall checker
    const fn new() -> Self {
        Self {
            counter: alloc::collections::BTreeMap::new(),
        }
    }
}

static CHECKER_STATE: UPSafeCell<SyscallCheckerState> =
    unsafe { UPSafeCell::const_new(SyscallCheckerState::new()) };

/// Syscall checker
#[allow(dead_code)]
#[derive(Debug)]
struct SyscallChecker<Args> {
    syscall_id: usize,
    args: Args,
}

impl<Args> SyscallChecker<Args> {
    fn new(syscall_id: usize, args: Args) -> SyscallChecker<Args> {
        let checker = SyscallChecker { syscall_id, args };
        checker.start();
        checker
    }

    fn start(&self) {
        let mut state = CHECKER_STATE.exclusive_access();

        // counter
        assert!(
            state
                .counter
                .iter()
                .filter(|(&id, _)| id != SYSCALL_YIELD && id != SYSCALL_EXIT)
                .all(|(_, &x)| x == 0),
            "counter: {:?}",
            state.counter
        );
        *state.counter.entry(self.syscall_id).or_default() += 1;
    }

    fn finalize(&self) {
        let mut state = CHECKER_STATE.exclusive_access();

        // counter
        *state.counter.entry(self.syscall_id).or_default() -= 1;
        assert!(
            state
                .counter
                .iter()
                .filter(|(&id, _)| id != SYSCALL_YIELD && id != SYSCALL_EXIT)
                .all(|(_, &x)| x == 0),
            "counter: {:?}",
            state.counter
        );
    }
}

impl<Args> Drop for SyscallChecker<Args> {
    fn drop(&mut self) {
        self.finalize();
    }
}
