//! Process management syscalls
use crate::{
    config::{MAX_SYSCALL_NUM,CLOCK_FREQ},
    task::{exit_current_and_run_next,get_current_start_time,get_current_syscall_times,suspend_current_and_run_next, TaskStatus},
    timer::{get_time, get_time_us,MSEC_PER_SEC}
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
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let start_time = match get_current_start_time(){
        Some(time) => time,
        None =>{return -1;}

    };
    let now = get_time() as usize ;
    //let interval = (now-start_time)* MICRO_PER_SEC / CLOCK_FREQ;
    let interval = (now-start_time)/ (CLOCK_FREQ / MSEC_PER_SEC);
    unsafe{
        *_ti = TaskInfo{
            status: TaskStatus::Running,
            syscall_times: get_current_syscall_times(),
            time: interval,
        };
        println!("THE TASKINFO SYSCALL NUM IS {}",(*_ti).syscall_times[410]);
    }
    
    0
}
