use crate::net::udp::UDP;
use crate::net::IPv4;
use crate::task::current_process;
use crate::task::current_task;
use alloc::sync::Arc;

// just support udp
pub fn sys_connect(raddr: u32, lport: u16, rport: u16) -> isize {
    trace!(
        "kernel:pid[{}] sys_connect",
        current_task().unwrap().process.upgrade().unwrap().getpid()
    );
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    let fd = inner.alloc_fd();
    let udp_node = UDP::new(IPv4::from_u32(raddr), lport, rport);
    inner.fd_table[fd] = Some(Arc::new(udp_node));
    fd as isize
}
