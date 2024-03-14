use std::{thread::sleep, time::Duration};

use futures::{executor::ThreadPool, task::SpawnExt, AsyncReadExt, AsyncWriteExt};
use rasi::fs::{NamedPipeListener, NamedPipeStream};

use crate::async_spec;

pub async fn test_named_pipe(syscall: &'static dyn rasi_syscall::NamedPipe) {
    let thread_pool = ThreadPool::new().unwrap();

    let bind_path = r"\\.\pipe\rasi-named-pipe-create";

    let server = NamedPipeListener::bind_with(bind_path, syscall)
        .await
        .unwrap();

    let message = b"hello world";

    thread_pool
        .spawn(async move {
            let mut stream = server.accept().await.unwrap();

            let mut buf = vec![0; 32];

            let read_size = stream.read(&mut buf).await.unwrap();

            assert_eq!(&buf[..read_size], message);

            stream.write(&buf[..read_size]).await.unwrap();
        })
        .unwrap();

    sleep(Duration::from_secs(1));

    let mut client = NamedPipeStream::connect_with(bind_path, syscall)
        .await
        .unwrap();

    client.write(message).await.unwrap();

    let mut buf = vec![0; 32];

    let read_size = client.read(&mut buf).await.unwrap();

    assert_eq!(&buf[..read_size], message);
}

pub async fn run_windows_spec(syscall: &'static dyn rasi_syscall::NamedPipe) {
    async_spec!(test_named_pipe, syscall);
}
