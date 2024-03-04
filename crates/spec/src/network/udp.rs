use std::io;

use rasi::net::UdpSocket;

use crate::async_spec;

pub async fn test_udp_echo(syscall: &'static dyn rasi_syscall::Network) -> io::Result<()> {
    let client = UdpSocket::bind_with("127.0.0.1:0", syscall).await?;
    let server = UdpSocket::bind_with("127.0.0.1:0", syscall).await?;

    let message = b"hello world";

    client.send_to(message, server.local_addr()?).await?;

    let mut buf = vec![0; 32];

    let (read_size, raddr) = server.recv_from(&mut buf).await?;

    assert_eq!(&buf[..read_size], message);
    assert_eq!(raddr, client.local_addr()?);

    server.send_to(&buf[..read_size], raddr).await?;

    let (read_size, raddr) = client.recv_from(&mut buf).await?;

    assert_eq!(&buf[..read_size], message);
    assert_eq!(raddr, server.local_addr()?);

    Ok(())
}

pub async fn run_udp_spec(syscall: &'static dyn rasi_syscall::Network) {
    async_spec!(test_udp_echo, syscall);
}
