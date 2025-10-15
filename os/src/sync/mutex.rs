//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self);
    /// Unlock the mutex
    fn unlock(&self);
    /// Try lock the mutex
    fn try_lock(&self, mutex_id: usize) -> isize;
    /// Unlock with specific mutex id
    fn unlock_mid(&self, mutex_id: usize);
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
        }
    }
}

impl Mutex for MutexSpin {
    /// try to lock if detect deadlock is enabled
    fn try_lock(&self, mutex_id: usize) -> isize {
        loop {
            let mut locked = self.locked.exclusive_access();
            let cur_task = current_task().unwrap();
            let mut current_inner = cur_task.inner_exclusive_access();
            if *locked {
                let current_tid = current_inner.res.as_ref().unwrap().tid;
                drop(current_inner);
                // let task_inner = current_task
                let process = cur_task.process.upgrade().unwrap();
                let process_inner = process.inner_exclusive_access();
                for task_opt in process_inner.tasks.iter() {
                    if let Some(task) = task_opt {
                        let task_inner = task.inner_exclusive_access();
                        if task_inner.res.is_some() && task_inner.res.as_ref().unwrap().tid == current_tid {
                            if let Some(_mid) = task_inner.locks_holding.iter().find(|&&mid| mid == mutex_id) {
                                return -0xDEAD;
                                // can not acquire the lock already held
                            }
                        } else {
                            let current_inner = cur_task.inner_exclusive_access();
                            if let Some(earning_id) = task_inner.lock_earning {
                                if let Some(_cur_holding) = current_inner.locks_holding.iter().find(|&&mid| mid == earning_id) {
                                    return -0xDEAD;
                                }
                            }
                        }
                    }
                }
                let mut current_inner = cur_task.inner_exclusive_access();
                current_inner.lock_earning = Some(mutex_id);
                drop(locked);
                drop(current_inner);
                drop(process_inner);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                current_inner.locks_holding.push(mutex_id);
                return 0;
            }
        }
    }
    /// Lock the spinlock mutex
    fn lock(&self) {
        trace!("kernel: MutexSpin::lock");
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                *locked = true;
                return;
            }
        }
    }
    /// Unlock the spinlock mutex
    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
    /// Unlock the spinlock with mutex id
    fn unlock_mid(&self, mutex_id: usize) {
        trace!("kernel: MutexSpin::unlock");
        let mut locked = self.locked.exclusive_access();
        let cur_task = current_task().unwrap();
        let mut task_inner = cur_task.inner_exclusive_access();
        for (i, mid) in task_inner.locks_holding.iter().enumerate() {
            if *mid == mutex_id {
                task_inner.locks_holding.remove(i);
                break;
            }
        }
        *locked = false;
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new() -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// try to lock if detect deadlock is enabled
    fn try_lock(&self, mutex_id: usize) -> isize {
        let mut mutex_inner = self.inner.exclusive_access();
        let cur_task = current_task().unwrap();
        let mut current_inner = cur_task.inner_exclusive_access();
        if mutex_inner.locked {
            let current_tid = current_inner.res.as_ref().unwrap().tid;
            drop(current_inner);
            // let task_inner = current_task
            let process = cur_task.process.upgrade().unwrap();
            let process_inner = process.inner_exclusive_access();
            for task_opt in process_inner.tasks.iter() {
                if let Some(task) = task_opt {
                    let task_inner = task.inner_exclusive_access();
                    if task_inner.res.is_some() && task_inner.res.as_ref().unwrap().tid == current_tid {
                        if let Some(_mid) = task_inner.locks_holding.iter().find(|&&mid| mid == mutex_id) {
                            return -0xDEAD;
                            // can not acquire the lock already held
                        }
                    } else {
                        let current_inner = cur_task.inner_exclusive_access();
                        if let Some(earning_id) = task_inner.lock_earning {
                            if let Some(_cur_holding) = current_inner.locks_holding.iter().find(|&&mid| mid == earning_id) {
                                return -0xDEAD;
                            }
                        }
                    }
                }
            }
            let mut current_inner = cur_task.inner_exclusive_access();
            current_inner.lock_earning = Some(mutex_id);
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(current_inner);
            drop(process_inner);
            drop(mutex_inner);
            block_current_and_run_next();
            0
        } else {
            mutex_inner.locked = true;
            current_inner.locks_holding.push(mutex_id);
            0
        }
    }

    /// lock the blocking mutex
    fn lock(&self) {
        trace!("kernel: MutexBlocking::lock");
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
        }
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
    /// unlock the blocking mutex with mutex id
    fn unlock_mid(&self, mutex_id: usize) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        let cur_task = current_task().unwrap();
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
        let mut task_inner = cur_task.inner_exclusive_access();
        for (i, mid) in task_inner.locks_holding.iter().enumerate() {
            if *mid == mutex_id {
                task_inner.locks_holding.remove(i);
                break;
            }
        }
    }
}
