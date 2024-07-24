use std::time::Duration;

use futures::{executor::ThreadPool, task::SpawnExt, AsyncReadExt, AsyncWriteExt, TryStreamExt};
use rasi::ipc::{IpcListener, IpcStream};

#[cfg(windows)]
use rasi::fs::syscall::Driver;
#[cfg(unix)]
use rasi::net::syscall::Driver;

use crate::async_spec;

pub async fn test_named_pipe(
    #[cfg(windows)] syscall: &dyn Driver,
    #[cfg(unix)] syscall: &dyn Driver,
) {
    let thread_pool = ThreadPool::new().unwrap();

    let mut server = IpcListener::bind_with("rasi-named-pipe-create", syscall)
        .await
        .unwrap();

    let message = b"hello world";

    thread_pool
        .spawn(async move {
            let mut stream = server.try_next().await.unwrap().unwrap();

            let mut buf = vec![0; 32];

            let read_size = stream.read(&mut buf).await.unwrap();

            assert_eq!(&buf[..read_size], message);

            stream.write(&buf[..read_size]).await.unwrap();
        })
        .unwrap();

    std::thread::sleep(Duration::from_secs(1));

    let mut client = IpcStream::connect_with("rasi-named-pipe-create", syscall)
        .await
        .unwrap();

    client.write(message).await.unwrap();

    let mut buf = vec![0; 32];

    let read_size = client.read(&mut buf).await.unwrap();

    assert_eq!(&buf[..read_size], message);
}

pub async fn run_ipc_spec(#[cfg(windows)] syscall: &dyn Driver, #[cfg(unix)] syscall: &dyn Driver) {
    async_spec!(test_named_pipe, syscall);
}
