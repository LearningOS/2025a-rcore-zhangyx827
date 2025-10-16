//! Semaphore

use crate::sync::{check_deadlock_ok, UPSafeCell};
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::vec::Vec;
use alloc::{collections::VecDeque, sync::Arc};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
    pub unused_cnt: usize,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                    unused_cnt: 0,
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                wakeup_task(task);
            }
        }
    }
    /// up operation with sem_id when detecting lock
    pub fn up_semid(&self, sem_id: usize) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        let task = current_task().unwrap();
        let mut task_inner = task.inner_exclusive_access();
        for (i, id) in task_inner.res_holding.iter().enumerate() {
            if *id == sem_id {
                task_inner.res_holding.remove(i);
                break;
            }
        }

        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                inner.unused_cnt += 1;
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self) {
        trace!("kernel: Semaphore::down");
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        }
    }
    /// try to down
    pub fn try_down(&self, sem_id: usize) -> isize {   
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        let cur_task = current_task().unwrap();
        let mut task_inner = cur_task.inner_exclusive_access();
        if inner.count < 0 {
            task_inner.res_earning = Some(sem_id);
            let process = cur_task.process.upgrade().unwrap();
            let process_inner = process.inner_exclusive_access();
            let mut living_tasks = Vec::new();
            for task_opt in process_inner.tasks.iter() {
                if let Some(task) = task_opt {
                    living_tasks.push(task);
                }
            }
            // drop(process_inner);
            drop(task_inner);
            // drop(process_inner);
            drop(inner);
            if check_deadlock_ok(living_tasks, &process_inner, false) {
                drop(process_inner);
                let mut inner = self.inner.exclusive_access();
                inner.wait_queue.push_back(current_task().unwrap());
                drop(inner);
                block_current_and_run_next();
                let mut inner = self.inner.exclusive_access();
                let mut task_inner = cur_task.inner_exclusive_access();
                task_inner.res_holding.push(sem_id);
                task_inner.res_earning = None;
                inner.unused_cnt -= 1;
            } else {
                drop(process_inner);
                // drop(inner);
                return -0xDEAD;
            }
        } else {
            task_inner.res_holding.push(sem_id);
        }
        0
    }
}
