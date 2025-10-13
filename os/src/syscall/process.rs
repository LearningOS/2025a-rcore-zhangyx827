//! Process management syscalls
//!
use alloc::sync::Arc;

use crate::{
    config::PAGE_SIZE, fs::{open_file, OpenFlags}, mm::{translated_byte_buffer, translated_refmut, translated_str, MapArea, MapPermission, MapType, VirtAddr}, task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskControlBlock,
    }, timer::get_time_us
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
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

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
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

/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let sec = us / 1_000_000;
    let usec = us % 1_000_000;
    let token = current_user_token();
    let mut va_start = ts as *const _ as usize;
    let mut va = va_start;
    let mut page_start = (va / PAGE_SIZE) * PAGE_SIZE;
    let mut bytes_arrays = translated_byte_buffer(token, va as *const u8,
        core::mem::size_of::<usize>() * 2);
    let end_va_sec = va + core::mem::size_of::<usize>();
    let mut mask = 0b11111111;
    let mut i = 0;
    let mut cur = 0;
    while va < end_va_sec {
        if va / PAGE_SIZE != page_start / PAGE_SIZE {
            page_start += PAGE_SIZE;
            cur = 1;
            va_start = page_start;
        }
        bytes_arrays[cur][va - va_start] = ((sec & mask) >> (i * 8)) as u8;
        mask <<= 8;
        va += 1;
        i += 1;
    }

    mask = 0b11111111;
    i = 0;
    let end_va_usec = end_va_sec + core::mem::size_of::<usize>();
    while va < end_va_usec {
        if va / PAGE_SIZE != page_start / PAGE_SIZE {
            page_start += PAGE_SIZE;
            cur = 1;
            va_start = page_start;
        }
        bytes_arrays[cur][va - va_start] = ((usec & mask) >> (i * 8)) as u8;
        mask <<= 8;
        va += 1;
        i += 1;
    }
    0
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    if start % PAGE_SIZE != 0 
    || port & !0x7 != 0 
    || port & 0x7 == 0 {
        return -1;
    }
    let length = ((len + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;
    let end = start + (length - PAGE_SIZE);
    let mut perm = MapPermission::empty();
    if port & (0b1 as usize) != 0 {
        perm |= MapPermission::R;
    }
    if port & (0b10 as usize) != 0 {
        perm |= MapPermission::W;
    }
    if port & (0b100 as usize) != 0 {
        perm |= MapPermission::X;
    }
    map_current_page(MapArea::new(VirtAddr::from(start), 
    VirtAddr::from(end), MapType::Framed, perm)) 
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    let length = ((len + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;
    let start_va = VirtAddr::from(start);
    unmap_current_page(start_va.into(), length / PAGE_SIZE)
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

/// spawn a new task
pub fn sys_spawn(path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let path = translated_str(token, path);
    // 这部分直接参考了sys_exec 的实现
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = Arc::new(TaskControlBlock::new(data));
        let mut task_inner = task.inner_exclusive_access();
        let current_task = current_task().unwrap();
        task_inner.parent = Some(Arc::downgrade(&current_task));
        let mut cur_inner = current_task.inner_exclusive_access();
        cur_inner.children.push(task.clone());
        let ans = task.pid.0;
        drop(task_inner);
        add_task(task);
        return ans as isize;
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}
