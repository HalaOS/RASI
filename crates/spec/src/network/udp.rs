use rasi::net::NetworkDriver;
use rasi::net::UdpSocket;

use crate::async_spec;

pub async fn test_udp_echo(syscall: &dyn NetworkDriver) {
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

pub async fn run_udp_spec(syscall: &dyn NetworkDriver) {
    async_spec!(test_udp_echo, syscall);
}
