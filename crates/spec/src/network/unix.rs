use std::{env::temp_dir, fs::remove_file};

use futures::{executor::ThreadPool, task::SpawnExt, AsyncReadExt, AsyncWriteExt};
use rasi::net::syscall::Driver;
use rasi::net::unix::{UnixListener, UnixStream};

use crate::async_spec;

pub async fn test_unix_echo(syscall: &dyn Driver) {
    let thread_pool = ThreadPool::new().unwrap();

    let dir = temp_dir();
    let bind_path = dir.join("echo");

    if bind_path.exists() {
        remove_file(bind_path.clone()).unwrap();
    }

    let server = UnixListener::bind_with(bind_path.clone(), syscall)
        .await
        .unwrap();

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

    let mut client = UnixStream::connect_with(bind_path, syscall).await.unwrap();

    client.write(message).await.unwrap();

    let mut buf = vec![0; 32];

    let read_size = client.read(&mut buf).await.unwrap();

    assert_eq!(&buf[..read_size], message);
}

pub async fn run_unix_spec(syscall: &dyn Driver) {
    async_spec!(test_unix_echo, syscall);
}
