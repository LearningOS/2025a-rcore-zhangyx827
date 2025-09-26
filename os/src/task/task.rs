//! Types related to task management

use super::TaskContext;
use crate::config::TRAP_CONTEXT_BASE;
use crate::mm::{
    kernel_stack_position, MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE
};
use crate::trap::{trap_handler, TrapContext};
use crate::syscall::{SYSCALL_EXIT, SYSCALL_GET_TIME, SYSCALL_MMAP, SYSCALL_MUNMAP, SYSCALL_SBRK, SYSCALL_TRACE,
SYSCALL_WRITE, SYSCALL_YIELD};
/// The task control block (TCB) of a task.
pub struct TaskControlBlock {
    /// Save task context
    pub task_cx: TaskContext,

    /// Maintain the execution status of the current process
    pub task_status: TaskStatus,

    /// Application address space
    pub memory_set: MemorySet,

    /// The phys page number of trap context
    pub trap_cx_ppn: PhysPageNum,

    /// The size(top addr) of program which is loaded from elf file
    pub base_size: usize,

    /// Heap bottom
    pub heap_bottom: usize,

    /// Program break
    pub program_brk: usize,

    /// Syscall counter
    pub syscall_cnt: CountSyscall,
}

impl TaskControlBlock {
    /// get the trap context
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    /// get the user token
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    /// Based on the elf info in program, build the contents of task in a new address space
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        // map a kernel-stack in kernel space
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let new_syscall_cnt = CountSyscall::zero_init();
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
            syscall_cnt: new_syscall_cnt,
        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }
    /// change the location of the program break. return None if failed.
    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        let old_break = self.program_brk;
        let new_brk = self.program_brk as isize + size as isize;
        if new_brk < self.heap_bottom as isize {
            return None;
        }
        let result = if size < 0 {
            self.memory_set
                .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        } else {
            self.memory_set
                .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        };
        if result {
            self.program_brk = new_brk as usize;
            Some(old_break)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
/// task status: UnInit, Ready, Running, Exited
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


#[derive(Copy, Clone)]
pub struct CountSyscall {
    cnt_write: isize,
    cnt_exit: isize,
    cnt_yield: isize,
    cnt_gettime: isize,
    cnt_trace: isize,
    cnt_mmap: isize,
    cnt_munmap: isize,
    cnt_sbrk: isize,
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
            SYSCALL_MMAP => self.cnt_mmap += 1,
            SYSCALL_MUNMAP => self.cnt_munmap += 1,
            SYSCALL_SBRK => self.cnt_sbrk += 1,
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
            SYSCALL_MMAP => return self.cnt_mmap,
            SYSCALL_MUNMAP => return self.cnt_munmap,
            SYSCALL_SBRK => return self.cnt_sbrk,
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
            cnt_mmap: 0,
            cnt_munmap: 0,
            cnt_sbrk: 0,
        }
    }   
}