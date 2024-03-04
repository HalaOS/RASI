use std::io;

use futures::{executor::ThreadPool, task::SpawnExt, AsyncReadExt};
use rasi::{
    futures::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};

use crate::async_spec;

pub async fn test_tcp_ttl(syscall: &'static dyn rasi_syscall::Network) -> io::Result<()> {
    let server = TcpListener::bind_with("127.0.0.1:0", syscall).await?;

    server.set_ttl(128)?;

    assert_eq!(128, server.ttl()?);

    let thread_pool = ThreadPool::new().unwrap();

    let raddr = server.local_addr()?;

    thread_pool
        .spawn(async move {
            let (mut stream, _) = server.accept().await.unwrap();

            let mut buf = vec![0; 32];

            let _ = stream.read(&mut buf).await.unwrap();
        })
        .unwrap();

    let client = TcpStream::connect_with(raddr, syscall).await?;

    client.set_ttl(128)?;

    assert_eq!(128, client.ttl()?);

    client.set_nodelay(true)?;

    assert!(client.nodelay()?);

    Ok(())
}

pub async fn test_tcp_echo(syscall: &'static dyn rasi_syscall::Network) -> io::Result<()> {
    let thread_pool = ThreadPool::new().unwrap();

    let server = TcpListener::bind_with("127.0.0.1:0", syscall).await?;

    let raddr = server.local_addr()?;

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

    let mut client = TcpStream::connect_with(raddr, syscall).await?;

    client.write(message).await?;

    let mut buf = vec![0; 32];

    let read_size = client.read(&mut buf).await?;

    assert_eq!(&buf[..read_size], message);

    Ok(())
}

pub async fn run_tcp_spec(syscall: &'static dyn rasi_syscall::Network) {
    async_spec!(test_tcp_echo, syscall);
    async_spec!(test_tcp_ttl, syscall);
}
