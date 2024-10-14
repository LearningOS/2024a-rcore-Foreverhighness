//! Types related to task management

use super::TaskContext;
use crate::config::MAX_SYSCALL_NUM;

/// The task control block (TCB) of a task.
#[derive(Copy, Clone, Debug)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
}

/// The running time info of task
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RunningTimeInfo {
    pub user_time_us: usize,
    pub kernel_time_us: usize,
    pub real_time_us: usize,
}

/// The task information block of a task.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaskInfoBlock {
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub running_times: RunningTimeInfo,
}

impl TaskInfoBlock {
    /// New task info block
    pub fn new() -> Self {
        Self {
            syscall_times: [0; MAX_SYSCALL_NUM],
            ..Default::default()
        }
    }
}

impl Default for TaskInfoBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// The status of a task
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
