use rasi::net::syscall::Driver;

use self::{tcp::run_tcp_spec, udp::run_udp_spec};

pub mod tcp;
pub mod udp;
#[cfg(unix)]
pub mod unix;

pub async fn run_network_spec(syscall: &dyn Driver) {
    run_udp_spec(syscall).await;
    run_tcp_spec(syscall).await;

    #[cfg(unix)]
    unix::run_unix_spec(syscall).await;
}
