//! Types related to task management

use super::TaskContext;
use crate::syscall::{SYSCALL_EXIT, SYSCALL_GET_TIME, SYSCALL_TRACE,
SYSCALL_WRITE, SYSCALL_YIELD};

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// the syscall counter
    pub syscall_cnt: CountSyscall,
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
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

/// the syscall count of a task
#[derive(Copy, Clone)]
pub struct CountSyscall {
    cnt_write: isize,
    cnt_exit: isize,
    cnt_yield: isize,
    cnt_gettime: isize,
    cnt_trace: isize,
}

impl CountSyscall {
    /// moidify the syscall_count
    pub fn modify_cnt(&mut self, syscall_id: usize) {
        match syscall_id {
            SYSCALL_WRITE => self.cnt_write += 1,
            SYSCALL_EXIT => self.cnt_exit += 1,
            SYSCALL_YIELD => self.cnt_yield += 1,
            SYSCALL_GET_TIME => self.cnt_gettime += 1,
            SYSCALL_TRACE => self.cnt_trace += 1,
            _ => panic!("Unsupported syscall_id: {}", syscall_id),
        };
    }

    /// get the syscall_count
    pub fn get_cnt(&self, syscall_id: usize) -> isize {
        match syscall_id {
            SYSCALL_WRITE => return self.cnt_write,
            SYSCALL_EXIT => return self.cnt_exit,
            SYSCALL_YIELD => return self.cnt_yield,
            SYSCALL_GET_TIME => return self.cnt_gettime,
            SYSCALL_TRACE => return self.cnt_trace,
            _ => {return -1 as isize;}
        };

    }
    /// initialize the count 
    /// of the syscall to be zero
    pub fn zero_init() -> Self {
        Self {
            cnt_exit: 0,
            cnt_gettime: 0,
            cnt_trace: 0,
            cnt_write: 0,
            cnt_yield: 0,
        }
    }   
}