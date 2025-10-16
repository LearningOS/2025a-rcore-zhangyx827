//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::sync::check_deadlock_ok;
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::vec::Vec;
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
    /// whether lock is locked
    fn is_used(&self) -> bool;
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
    used: UPSafeCell<bool>,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
            used: unsafe {UPSafeCell::new(false)},
        }
    }
}

impl Mutex for MutexSpin {
    fn is_used(&self) -> bool {
        let used = self.used.exclusive_access();
        return *used;
    }
    /// try to lock if detect deadlock is enabled
    fn try_lock(&self, mutex_id: usize) -> isize {
        loop {
            let mut locked = self.locked.exclusive_access();
            let cur_task = current_task().unwrap();
            let mut current_inner = cur_task.inner_exclusive_access();
            if *locked {
                current_inner.res_earning = Some(mutex_id);
                let process = cur_task.process.upgrade().unwrap();
                let process_inner = process.inner_exclusive_access();
                let mut living_tasks = Vec::new();
                for task_opt in process_inner.tasks.iter() {
                    if let Some(task) = task_opt {
                        living_tasks.push(task);
                    }
                }
                drop(current_inner);
                drop(locked);
                if check_deadlock_ok(living_tasks, &process_inner, true) {
                    let _locked = self.locked.exclusive_access();
                    drop(process_inner);
                    drop(_locked);
                    suspend_current_and_run_next();
                    // return 0;
                } else {
                    drop(process_inner);
                    // drop(locked);
                    return -0xDEAD;
                }
            } else {
                *locked = true;
                let mut used = self.used.exclusive_access();
                *used = true;
                current_inner.res_holding.push(mutex_id);
                if current_inner.res_earning.is_some() {
                    current_inner.res_earning = None;
                }   
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
        let mut used = self.used.exclusive_access();
        let cur_task = current_task().unwrap();
        let mut task_inner = cur_task.inner_exclusive_access();
        for (i, mid) in task_inner.res_holding.iter().enumerate() {
            if *mid == mutex_id {
                task_inner.res_holding.remove(i);
                break;
            }
        }
        *locked = false;
        *used = false;
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
    used: bool,
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
                    used: false,
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    fn is_used(&self) -> bool {
        let mutex_inner = self.inner.exclusive_access();
        return mutex_inner.used;
    }
    /// try to lock if detect deadlock is enabled
    fn try_lock(&self, mutex_id: usize) -> isize {
        let mut mutex_inner = self.inner.exclusive_access();
        let cur_task = current_task().unwrap();
        let mut current_inner = cur_task.inner_exclusive_access();
        if mutex_inner.locked {
            current_inner.res_earning = Some(mutex_id);
            let process = cur_task.process.upgrade().unwrap();
            let process_inner = process.inner_exclusive_access();
            let mut living_tasks = Vec::new();
            for task_opt in process_inner.tasks.iter() {
                if let Some(task) = task_opt {
                    living_tasks.push(task);
                }
            }
            let _id = current_inner.res.as_ref().unwrap().tid;
            // drop(process_inner);
            drop(current_inner);
            drop(mutex_inner);
            if check_deadlock_ok(living_tasks, &process_inner, true) {
                // let mut current_inner = cur_task.inner_exclusive_access();
                let mut mutex_inner = self.inner.exclusive_access();
                mutex_inner.wait_queue.push_back(current_task().unwrap());
                drop(process_inner);
                drop(mutex_inner);
                block_current_and_run_next();
                let mut mutex_inner = self.inner.exclusive_access();
                let mut current_inner = cur_task.inner_exclusive_access();
                current_inner.res_holding.push(mutex_id);
                current_inner.res_earning = None;
                mutex_inner.used = true;
                let _id = current_inner.res.as_ref().unwrap().tid;
                0
            } else {
                drop(process_inner);
                // drop(mutex_inner);
                return -0xDEAD;
            }
        } else {
            mutex_inner.locked = true;
            mutex_inner.used = true;
            current_inner.res_holding.push(mutex_id);
            let _id = current_inner.res.as_ref().unwrap().tid;
            // debug!("task {} acquired lock{}  ", current_inner.res.as_ref().unwrap().tid, mutex_id);
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
            // mutex_inner.locked = false;
        } else {
            mutex_inner.locked = false;
        }
    }
    /// unlock the blocking mutex with mutex id
    fn unlock_mid(&self, mutex_id: usize) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        mutex_inner.used = false;
        let cur_task = current_task().unwrap();
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
        let mut task_inner = cur_task.inner_exclusive_access();
        let _id = task_inner.res.as_ref().unwrap().tid;
        // debug!("task {} released lock{}  ", task_inner.res.as_ref().unwrap().tid, mutex_id);
        for (i, mid) in task_inner.res_holding.iter().enumerate() {
            if *mid == mutex_id {
                task_inner.res_holding.remove(i);
                break;
            }
        }
    }
}
