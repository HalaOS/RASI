use std::net::SocketAddr;

use futures::{SinkExt, TryStreamExt};
use rand::seq::IteratorRandom;
use rasi::net::UdpSocket;
use rasi_ext::{bytes::Bytes, net::udp_group::UdpGroup};

use crate::async_spec;

pub async fn test_udp_echo(syscall: &'static dyn rasi_syscall::Network) {
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

pub async fn test_udp_group_echo(syscall: &'static dyn rasi_syscall::Network) {
    let addrs: Vec<SocketAddr> = ["127.0.0.1:0".parse().unwrap()].repeat(4);
    let (mut client_sender, mut client_receiver) = UdpGroup::bind_with(addrs.as_slice(), syscall)
        .await
        .unwrap()
        .split();

    let server = UdpGroup::bind_with(addrs.as_slice(), syscall)
        .await
        .unwrap();

    let raddrs = server.local_addrs().cloned().collect::<Vec<_>>();

    let (mut server_sender, mut server_receiver) = server.split();

    let random_raddr = raddrs
        .iter()
        .choose(&mut rand::thread_rng())
        .cloned()
        .unwrap();

    client_sender
        .send((Bytes::from_static(b"hello world"), None, random_raddr))
        .await
        .unwrap();

    let (buf, path_info) = server_receiver.try_next().await.unwrap().unwrap();

    let buf = buf.freeze();

    assert_eq!(buf, Bytes::from_static(b"hello world"));

    server_sender
        .send((
            Bytes::from_static(b"hello world"),
            Some(path_info.to),
            path_info.from,
        ))
        .await
        .unwrap();

    let (buf, _) = client_receiver.try_next().await.unwrap().unwrap();

    let buf = buf.freeze();

    assert_eq!(buf, Bytes::from_static(b"hello world"));
}

pub async fn run_udp_spec(syscall: &'static dyn rasi_syscall::Network) {
    async_spec!(test_udp_echo, syscall);
    async_spec!(test_udp_group_echo, syscall);
}
