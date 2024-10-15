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

/// write syscall
const SYSCALL_WRITE: usize = 64;
/// exit syscall
const SYSCALL_EXIT: usize = 93;
/// yield syscall
const SYSCALL_YIELD: usize = 124;
/// gettime syscall
const SYSCALL_GET_TIME: usize = 169;
/// taskinfo syscall
const SYSCALL_TASK_INFO: usize = 410;

mod fs;
mod process;

use alloc::collections::btree_map::BTreeMap;
use fs::*;
use process::*;

use crate::sync::UPSafeCell;

/// Checker state in syscall
struct SyscallCheckerState {
    // assert all sys_func will return
    counter: BTreeMap<usize, i32>,
}

impl SyscallCheckerState {
    /// Construct a syscall checker
    const fn new() -> Self {
        Self {
            counter: BTreeMap::new(),
        }
    }
}

static CHECKER_STATE: UPSafeCell<SyscallCheckerState> =
    unsafe { UPSafeCell::const_new(SyscallCheckerState::new()) };

/// Syscall checker
#[allow(dead_code)]
struct SyscallChecker {
    syscall_id: usize,
    args: [usize; 3],
}

impl SyscallChecker {
    fn new(syscall_id: usize, args: [usize; 3]) -> SyscallChecker {
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

impl Drop for SyscallChecker {
    fn drop(&mut self) {
        self.finalize();
    }
}

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    let _guard = SyscallChecker::new(syscall_id, args);

    update_syscall_times(syscall_id);
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TASK_INFO => sys_task_info(args[0] as *mut TaskInfo),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
