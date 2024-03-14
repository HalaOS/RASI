use futures::{AsyncReadExt, AsyncWriteExt};
use rasi::{
    executor::spawn,
    inter_process::{IpcListener, IpcStream},
};

mod init;

#[futures_test::test]
async fn test_echo() {
    init::init();

    let server = IpcListener::bind("echo").await.unwrap();

    let message = b"hello world";

    spawn(async move {
        let mut stream = server.accept().await.unwrap();

        let mut buf = vec![0; 32];

        let read_size = stream.read(&mut buf).await.unwrap();

        assert_eq!(&buf[..read_size], message);

        stream.write(&buf[..read_size]).await.unwrap();
    });

    let mut client = IpcStream::connect("echo").await.unwrap();

    client.write(message).await.unwrap();

    let mut buf = vec![0; 32];

    let read_size = client.read(&mut buf).await.unwrap();

    assert_eq!(&buf[..read_size], message);
}
