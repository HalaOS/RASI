use futures::{executor::ThreadPool, task::SpawnExt, AsyncReadExt};
use rasi::net::{NetworkDriver, TcpListener, TcpStream};

use futures::AsyncWriteExt;

use crate::async_spec;

pub async fn test_tcp_ttl(syscall: &dyn NetworkDriver) {
    let server = TcpListener::bind_with("127.0.0.1:0", syscall)
        .await
        .unwrap();

    server.set_ttl(128).unwrap();

    assert_eq!(128, server.ttl().unwrap());

    let thread_pool = ThreadPool::new().unwrap();

    let raddr = server.local_addr().unwrap();

    thread_pool
        .spawn(async move {
            let (mut stream, _) = server.accept().await.unwrap();

            let mut buf = vec![0; 32];

            let _ = stream.read(&mut buf).await.unwrap();
        })
        .unwrap();

    let client = TcpStream::connect_with(raddr, syscall).await.unwrap();

    client.set_ttl(128).unwrap();

    assert_eq!(128, client.ttl().unwrap());

    client.set_nodelay(true).unwrap();

    assert!(client.nodelay().unwrap());
}

pub async fn test_tcp_echo(syscall: &dyn NetworkDriver) {
    let thread_pool = ThreadPool::new().unwrap();

    let server = TcpListener::bind_with("127.0.0.1:0", syscall)
        .await
        .unwrap();

    let raddr = server.local_addr().unwrap();

    let message = b"hello world";

    thread_pool
        .spawn(async move {
            let (mut stream, _) = server.accept().await.unwrap();

            let mut buf = vec![0; 32];

            let read_size = stream.read(&mut buf).await.unwrap();

            assert_eq!(&buf[..read_size], message);

            stream.write(&buf[..read_size]).await.unwrap();
        })
        .unwrap();

    let mut client = TcpStream::connect_with(raddr, syscall).await.unwrap();

    client.write(message).await.unwrap();

    let mut buf = vec![0; 32];

    let read_size = client.read(&mut buf).await.unwrap();

    assert_eq!(&buf[..read_size], message);
}

pub async fn run_tcp_spec(syscall: &dyn NetworkDriver) {
    async_spec!(test_tcp_echo, syscall);
    async_spec!(test_tcp_ttl, syscall);
}
