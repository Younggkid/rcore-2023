//! Types related to task management
use super::TaskContext;
use crate::config::{TRAP_CONTEXT_BASE,MAX_SYSCALL_NUM};
use crate::mm::{
    kernel_stack_position, VirtPageNum,MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE,
};
use crate::trap::{trap_handler, TrapContext};

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

    /// The time of this task start
    pub task_start_time: Option<usize>,
    
    /// The system call info of this task
    pub task_syscall_times: [u32;MAX_SYSCALL_NUM],
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
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
            task_start_time: None,
            task_syscall_times: [0; MAX_SYSCALL_NUM],
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
    ///map a vitual memory region starting from [start],length [len] to a physical memory
    pub fn mmap(&mut self, start:usize, len:usize, prot:usize) -> Option<(VirtPageNum,VirtPageNum)>{
        if prot & (!0x7) !=0 || prot & 0x7 == 0 {return None;}
        for viradd in start..start+len {
            let vpn = VirtAddr::from(viradd).floor();
            if let Some(pte) = self.memory_set.translate(vpn){
                if pte.is_valid() {return None;}
            }
        }
        let st_add = VirtAddr(start);
        let end_add = VirtAddr(start+len);
        let mut map_perm = MapPermission::U;
        if prot & 1 == 1 {
            map_perm |= MapPermission::R;
        }
        if prot>>1 & 1 == 1 {
            map_perm |= MapPermission::W;
        }
        if prot>>2 & 1 == 1 {
            map_perm |= MapPermission::X;
        }
        self.memory_set.insert_framed_area(st_add, end_add,map_perm);

        let start_vpn = st_add.floor();
        let end_vpn = end_add.ceil();
        Some((start_vpn,end_vpn))

    }
    /// opposite to mmap
    pub fn munmap(&mut self, start:usize, len:usize) -> bool{
        let st_add = VirtAddr(start);
        let end_add = VirtAddr(start+len);

        self.memory_set.remove_framed_area(st_add,end_add)

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
