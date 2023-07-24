//! Process management syscalls
use core::mem::size_of;

use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    task::{
        munmap,mmap,get_current_syscall_times,get_current_start_time,current_user_token, change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus,
    }, mm::{translated_byte_buffer, VirtAddr},
    timer::{get_time_us}
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
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
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let vir_addr: VirtAddr= (_ts as usize).into();
    let buffers = translated_byte_buffer(current_user_token(), 
                                        vir_addr.0 as *const u8, 
                                        size_of::<TimeVal>());
    let us = get_time_us();
    let time_val = TimeVal{
        sec : us/1000000,
        usec : us%1000000,
    };
    unsafe{
        let time_val_bytes = core::slice::from_raw_parts((&time_val as *const TimeVal) as *const u8 , size_of::<TimeVal>());
    
    let mut idx = 0;
    for buffer in buffers{
        for i in 0..buffer.len(){
            buffer[i] = time_val_bytes[idx];
            idx += 1;
        }
    }
}
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let vir_addr: VirtAddr = (_ti as usize).into();
    let buffers = translated_byte_buffer(current_user_token(), 
                            vir_addr.0 as *const u8, 
                        size_of::<TaskInfo>());
    let start_time = match get_current_start_time(){
        Some(time) => time,
        None => {return -1;}
    };
    let now = get_time_us() as usize;
    let interval = (now-start_time)/ 1000;
    let task_info = TaskInfo{
        status: TaskStatus::Running,
        syscall_times: get_current_syscall_times(),
        time: interval,
    };

    unsafe{
        let task_info_bytes = core::slice::from_raw_parts((&task_info as *const TaskInfo) as *const u8 , size_of::<TaskInfo>());
    
    let mut idx = 0;
    for buffer in buffers{
        for i in 0..buffer.len(){
            buffer[i] = task_info_bytes[idx];
            idx += 1;
        }
    }
    0
}

    
}
// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    if _start / PAGE_SIZE * PAGE_SIZE != _start  {return -1} 
    
    if mmap(_start,_len,_port).is_some() {return 0;}
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if _start / PAGE_SIZE * PAGE_SIZE != _start  {return -1} 
    if munmap(_start,_len) == true {return 0;}
    return -1;
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
