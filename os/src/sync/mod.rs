//! Synchronization and interior mutability primitives

mod condvar;
mod mutex;
mod semaphore;
mod up;

use core::cell::RefMut;

use alloc::{sync::Arc, vec::Vec, vec};
pub use condvar::Condvar;
pub use mutex::{Mutex, MutexBlocking, MutexSpin};
pub use semaphore::Semaphore;
pub use up::UPSafeCell;

use crate::task::{ProcessControlBlockInner, TaskControlBlock};
/// Check whether there is deadlock
pub fn check_deadlock_ok(living_tasks: Vec<&Arc<TaskControlBlock>>, process_inner: &RefMut<'_, ProcessControlBlockInner>, is_lock: bool) -> bool {
    // let task = current_task();
    let size = living_tasks.len();
    let mut available_res = Vec::new();
    let mut done = vec![false; size];
    let mut seen = vec![0; size];
    if is_lock {
        for (i, lock) in process_inner.mutex_list.iter().enumerate() {
            if lock.is_some() && !lock.as_ref().unwrap().is_used() {
                available_res.push(i);
            }
        }
    } else {
        for (i, sem) in process_inner.semaphore_list.iter().enumerate() {
            if sem.is_some() {
                let inner = sem.as_ref().unwrap().inner.exclusive_access();
                if inner.count > 0 || (inner.count <= 0 && inner.unused_cnt > 0) {
                    available_res.push(i);
                }
            }
        }
    }
    for (i, task) in living_tasks.iter().enumerate() {
        let task_inner = task.inner_exclusive_access();
        if task_inner.res_earning.is_none() {
            for res_id in task_inner.res_holding.iter() {
                available_res.push(*res_id);
            }
            done[i] = true;
        }
    }

    let mut i = 0;
    loop {
        if !done[i] {
            let task_inner = living_tasks[i].inner_exclusive_access();
            let earning_res = task_inner.res_earning.unwrap();
            if let Some(_mid) = available_res.iter().find(|&&res_id| res_id == earning_res) {
                // Ok
                for res_id in task_inner.res_holding.iter() {
                    available_res.push(*res_id);
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