//! Process management syscalls



use crate::{config::PAGE_SIZE, mm::{MapArea, MapPermission, MapType, PageTable, VirtAddr, VA_WIDTH_SV39}, task::{change_program_brk, current_user_token, exit_current_and_run_next, get_cnt, map_current_page, suspend_current_and_run_next, unmap_current_page}, timer::get_time_us};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let sec = us / 1_000_000;
    let usec = us % 1_000_000;
    let token = current_user_token();
    let page_table = PageTable::from_token(token);
    let mut va = ts as *const _ as usize;
    let mut page_start = (va / PAGE_SIZE) * PAGE_SIZE;
    let vpn = VirtAddr::from(page_start).floor();
    let mut bytes_array = page_table.translate(vpn)
        .unwrap()
        .ppn()
        .get_bytes_array();
    let end_va_sec = va + core::mem::size_of::<usize>();
    let mut mask = 0b11111111;
    let mut i = 0;
    while va < end_va_sec {
        if va / PAGE_SIZE != page_start / PAGE_SIZE {
            page_start += PAGE_SIZE;
            let vpn = VirtAddr::from(page_start).floor();
            bytes_array = page_table.translate(vpn)
                .unwrap()
                .ppn()
                .get_bytes_array();
        }
        bytes_array[va & (PAGE_SIZE - 1)] = ((sec & mask) >> (i * 8)) as u8;
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
            let vpn = VirtAddr::from(page_start).floor();
            bytes_array = page_table.translate(vpn)
                .unwrap()
                .ppn()
                .get_bytes_array();
        }
        bytes_array[va & (PAGE_SIZE - 1)] = ((usec & mask) >> (i * 8)) as u8;
        mask <<= 8;
        va += 1;
        i += 1;
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => {
            let token = current_user_token();
            let page_table = PageTable::from_token(token);
            let va = VirtAddr::from(id);
            let vpn = va.floor();
            let high_bit = id & (1 << (VA_WIDTH_SV39 - 1));
            if high_bit != 0 && (id &  (!((1 << VA_WIDTH_SV39 - 1) - 1))
            != !((1 << VA_WIDTH_SV39 - 1) - 1)) 
            || high_bit == 0 && (id &  (!((1 << VA_WIDTH_SV39 - 1) - 1))
            != 0) 
            {
                return -1;
            }
            if let Some(pte) = page_table.translate(vpn) {
                if !pte.is_valid() || !pte.readable() {
                    return -1;
                    // not valid or cannot be read
                }
                let mem = pte.ppn().get_bytes_array();
                return mem[va.page_offset()] as isize;
            } else {
                return -1;
                // there is no mapping of this 
                // virtual address.
            }
        },
        1 => {
            let token = current_user_token();
            let page_table = PageTable::from_token(token);
            let va = VirtAddr::from(id);
            let vpn = va.floor();
            if let Some(pte) = page_table.translate(vpn) {
                if !pte.is_valid() || !pte.writable() {
                    return -1;
                    // not valid or cannot be read
                }
                let mem = pte.ppn().get_bytes_array();
                mem[va.page_offset()] = data as u8;
                return 0;
            } else {
                return -1;
                // not mapped
            }
        },
        2 => {
            return get_cnt(id);
        },
        _ => return -1,
    }
}

// YOUR JOB: Implement mmap.
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

// YOUR JOB: Implement munmap.
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
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
