#[cfg(all(windows, feature = "windows_named_pipe"))]
mod windows {
    use std::io;

    use rasi_syscall::{global_named_pipe, NamedPipe};

    use crate::fs::{NamedPipeListener, NamedPipeStream};

    /// Interprocess communication server socket.
    pub struct IpcListener {
        named_pipe_listener: NamedPipeListener,
    }

    impl IpcListener {
        /// Create new ipc server lsitener with custom [`syscall`](NamedPipe) and bind to `addr`
        pub async fn bind_with<A: AsRef<str>>(
            name: A,
            syscall: &'static dyn NamedPipe,
        ) -> io::Result<Self> {
            let addr = format!(r"\\.\pipe\{}", name.as_ref());

            let named_pipe_listener = NamedPipeListener::bind_with(addr, syscall).await?;

            Ok(Self {
                named_pipe_listener,
            })
        }

        /// Create new ipc server lsitener with global registered [`syscall`](NamedPipe) and bind to `addr`
        pub async fn bind<A: AsRef<str>>(name: A) -> io::Result<Self> {
            Self::bind_with(name, global_named_pipe()).await
        }

        /// Accepts a new incoming connection to this listener.
        ///
        /// When a connection is established, the corresponding stream and address will be returned.
        pub async fn accept(&self) -> io::Result<IpcStream> {
            self.named_pipe_listener
                .accept()
                .await
                .map(|stream| IpcStream(stream))
        }
    }

    pub struct IpcStream(NamedPipeStream);

    impl IpcStream {
        /// Create new client named pipe stream and connect to `addr`
        pub async fn connect_with<A: AsRef<str>>(
            name: A,
            syscall: &'static dyn NamedPipe,
        ) -> io::Result<Self> {
            let addr = format!(r"\\.\pipe\{}", name.as_ref());

            NamedPipeStream::connect_with(addr, syscall)
                .await
                .map(|stream| IpcStream(stream))
        }

        /// Create new client named pipe stream and connect to `addr`
        pub async fn connect<A: AsRef<str>>(name: A) -> io::Result<Self> {
            Self::connect_with(name, global_named_pipe()).await
        }
    }
}

#[cfg(all(windows, feature = "windows_named_pipe"))]
pub use windows::*;

#[cfg(all(windows, feature = "unix_socket"))]
mod unix {}

#[cfg(all(windows, feature = "unix_socket"))]
pub use unix::*;
