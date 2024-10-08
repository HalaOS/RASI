use futures::poll;
use rasi::net::syscall::Driver;
use rasi::net::UdpSocket;

use crate::async_spec;

pub async fn test_udp_echo(syscall: &dyn Driver) {
    let client = UdpSocket::bind_with("127.0.0.1:0", syscall).await.unwrap();
    let server = UdpSocket::bind_with("127.0.0.1:0", syscall).await.unwrap();

    let message = b"hello world";

    client
        .send_to(message, server.local_addr().unwrap())
        .await
        .unwrap();

    let mut buf = vec![0; 32];

    let (read_size, raddr) = server.recv_from(&mut buf).await.unwrap();

    assert_eq!(&buf[..read_size], message);
    assert_eq!(raddr, client.local_addr().unwrap());

    server.send_to(&buf[..read_size], raddr).await.unwrap();

    let (read_size, raddr) = client.recv_from(&mut buf).await.unwrap();

    assert_eq!(&buf[..read_size], message);
    assert_eq!(raddr, server.local_addr().unwrap());
}

pub async fn test_udp_shutdown(syscall: &dyn Driver) {
    let socket = UdpSocket::bind_with("127.0.0.1:0", syscall).await.unwrap();

    let mut buf = vec![0; 1024];

    let mut recv_from = Box::pin(socket.recv_from(&mut buf));

    assert!(poll!(&mut recv_from).is_pending());

    socket.shutdown(std::net::Shutdown::Read).unwrap();

    assert!(poll!(&mut recv_from).is_ready());

    socket
        .send_to(b"hello", socket.local_addr().unwrap())
        .await
        .unwrap();

    socket.shutdown(std::net::Shutdown::Write).unwrap();

    socket
        .send_to(b"hello", socket.local_addr().unwrap())
        .await
        .expect_err("shutdown write");
}

pub async fn run_udp_spec(syscall: &dyn Driver) {
    async_spec!(test_udp_echo, syscall);
    async_spec!(test_udp_shutdown, syscall);
}
