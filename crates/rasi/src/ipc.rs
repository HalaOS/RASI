#[cfg(windows)]
mod windows {
    use std::{
        ffi::OsString,
        io,
        ops::{Deref, DerefMut},
        str::FromStr,
    };

    use futures::{future::poll_fn, Stream, StreamExt};

    use crate::fs::{
        get_fs_driver,
        windows::{NamedPipeListener, NamedPipeStream},
        FileSystemDriver,
    };

    /// Interprocess communication server socket.
    pub struct IpcListener {
        listener: NamedPipeListener,
    }

    impl IpcListener {
        /// Create new ipc server lsitener with custom [`syscall`](NamedPipe) and bind to `addr`
        pub async fn bind_with<A: AsRef<str>>(
            name: A,
            driver: &dyn FileSystemDriver,
        ) -> io::Result<Self> {
            let addr = format!(r"\\.\pipe\{}", name.as_ref());

            let listener = driver.named_pipe_server_create(OsString::from(&addr).as_ref())?;

            Ok(Self { listener })
        }

        /// Create new ipc server lsitener with global registered [`syscall`](NamedPipe) and bind to `addr`
        pub async fn bind<A: AsRef<str>>(name: A) -> io::Result<Self> {
            Self::bind_with(name, get_fs_driver()).await
        }

        /// Accepts a new incoming connection to this listener.
        ///
        /// When a connection is established, the corresponding stream and address will be returned.
        pub async fn accept(&mut self) -> io::Result<IpcStream> {
            self.listener.accept().await.map(|stream| IpcStream(stream))
        }
    }

    impl Stream for IpcListener {
        type Item = io::Result<IpcStream>;

        fn poll_next(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            self.listener
                .poll_next_unpin(cx)
                .map_ok(|stream| IpcStream(stream))
        }
    }

    pub struct IpcStream(NamedPipeStream);

    impl IpcStream {
        /// Create new client named pipe stream and connect to `addr`
        pub async fn connect_with<A: AsRef<str>>(
            name: A,
            driver: &dyn FileSystemDriver,
        ) -> io::Result<Self> {
            let addr = format!(r"\\.\pipe\{}", name.as_ref());

            let stream =
                driver.named_pipe_client_open(OsString::from_str(&addr).as_ref().unwrap())?;

            poll_fn(|cx| stream.poll_ready(cx))
                .await
                .map(|_| Self(stream))
        }

        /// Create new client named pipe stream and connect to `addr`
        pub async fn connect<A: AsRef<str>>(name: A) -> io::Result<Self> {
            Self::connect_with(name, get_fs_driver()).await
        }
    }

    impl Deref for IpcStream {
        type Target = NamedPipeStream;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for IpcStream {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}

#[cfg(windows)]
pub use windows::*;

#[cfg(unix)]
mod unix {
    use std::{
        env::temp_dir,
        fs::{create_dir_all, remove_file},
        io,
        ops::{Deref, DerefMut},
        task::{Context, Poll},
    };

    use futures::{Stream, StreamExt};

    use crate::net::{
        get_network_driver,
        unix::{UnixListener, UnixStream},
        NetworkDriver,
    };

    /// Interprocess communication server socket.
    pub struct IpcListener {
        named_pipe_listener: UnixListener,
    }

    impl IpcListener {
        /// Create new ipc server lsitener with custom [`syscall`](Network) and bind to `addr`
        pub async fn bind_with<A: AsRef<str>>(
            name: A,
            syscall: &dyn NetworkDriver,
        ) -> io::Result<Self> {
            let dir = temp_dir().join("inter_process");

            if !dir.exists() {
                create_dir_all(dir.clone()).unwrap();
            }

            let bind_path = dir.join(name.as_ref());

            if bind_path.exists() {
                remove_file(bind_path.clone()).unwrap();
            }

            let named_pipe_listener = UnixListener::bind_with(bind_path, syscall).await?;

            Ok(Self {
                named_pipe_listener,
            })
        }

        /// Create new ipc server lsitener with global registered [`syscall`](Network) and bind to `addr`
        pub async fn bind<A: AsRef<str>>(name: A) -> io::Result<Self> {
            Self::bind_with(name, get_network_driver()).await
        }

        /// Accepts a new incoming connection to this listener.
        ///
        /// When a connection is established, the corresponding stream and address will be returned.
        pub async fn accept(&mut self) -> io::Result<IpcStream> {
            self.named_pipe_listener
                .accept()
                .await
                .map(|(stream, _)| IpcStream(stream))
        }
    }

    impl Stream for IpcListener {
        type Item = io::Result<IpcStream>;

        fn poll_next(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Self::Item>> {
            self.named_pipe_listener
                .poll_next_unpin(cx)
                .map_ok(|stream| IpcStream(stream))
        }
    }

    #[derive(Clone)]
    pub struct IpcStream(UnixStream);

    impl IpcStream {
        /// Create new client named pipe stream and connect to `addr`
        pub async fn connect_with<A: AsRef<str>>(
            name: A,
            syscall: &dyn NetworkDriver,
        ) -> io::Result<Self> {
            let dir = temp_dir();
            let bind_path = dir.join("inter_process").join(name.as_ref());

            UnixStream::connect_with(bind_path, syscall)
                .await
                .map(|stream| IpcStream(stream))
        }

        /// Create new client named pipe stream and connect to `addr`
        pub async fn connect<A: AsRef<str>>(name: A) -> io::Result<Self> {
            Self::connect_with(name, get_network_driver()).await
        }
    }

    impl Deref for IpcStream {
        type Target = UnixStream;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for IpcStream {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}

#[cfg(unix)]
pub use unix::*;
