use self::{tcp::run_tcp_spec, udp::run_udp_spec};

pub mod tcp;
pub mod udp;

pub async fn run_network_spec(syscall: &'static dyn rasi_syscall::Network) {
    run_udp_spec(syscall).await;
    run_tcp_spec(syscall).await;
}
