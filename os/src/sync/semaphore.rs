//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::vec::Vec;
use alloc::vec;
use alloc::{collections::VecDeque, sync::Arc};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Check whether there is deadlock
    pub fn check_deadlock_ok(living_tasks: Vec<&Arc<TaskControlBlock>>) -> bool {
        let size = living_tasks.len();
        let mut available_sems = Vec::new();
        let mut done = vec![false; size];
        let mut seen = vec![0; size];

        for (i, task) in living_tasks.iter().enumerate() {
            let task_inner = task.inner_exclusive_access();
            if task_inner.earning_sem.is_none() {
                for sem_id in task_inner.holding_sem.iter() {
                    available_sems.push(*sem_id);
                }
                done[i] = true;
            }
        }

        let mut i = 0;
        loop {
            if !done[i] {
                let task_inner = living_tasks[i].inner_exclusive_access();
                let earning_sem = task_inner.earning_sem.unwrap();
                if let Some(_mid) = available_sems.iter().find(|&&sem_id| sem_id == earning_sem) {
                    // Ok
                    for sem_id in task_inner.holding_sem.iter() {
                        available_sems.push(*sem_id);
                    }
                    done[i] = true;
                } 
            }
            seen[i] += 1;
            if seen[i] == 3 {
                break;
            }
            i = (i + 1) % size;
        }
        if let Some(_false) = done.iter().find(|ok| **ok == false) {
            return false;
        } else {
            return true;
        }
    }
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
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
            if Self::check_deadlock_ok(living_tasks) {
                let mut task_inner = cur_task.inner_exclusive_access();
                task_inner.earning_sem = Some(sem_id);
                drop(task_inner);
                drop(process_inner);
                inner.wait_queue.push_back(current_task().unwrap());
                drop(inner);
                block_current_and_run_next();
            } else {
                return -0xDEAD;
            }
        } else {
            task_inner.holding_sem.push(sem_id);
        }
        0
    }
}
