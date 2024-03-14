#[cfg(all(windows, feature = "windows_named_pipe"))]
mod windows {
    use std::{
        io,
        ops::{Deref, DerefMut},
    };

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
        pub async fn accept(&mut self) -> io::Result<IpcStream> {
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

#[cfg(all(windows, feature = "windows_named_pipe"))]
pub use windows::*;

#[cfg(all(unix, feature = "unix_socket"))]
mod unix {
    use std::{
        env::temp_dir,
        fs::{create_dir_all, remove_file},
        io,
        ops::{Deref, DerefMut},
    };

    use rasi_syscall::{global_network, Network};

    use crate::net::{UnixListener, UnixStream};

    /// Interprocess communication server socket.
    pub struct IpcListener {
        named_pipe_listener: UnixListener,
    }

    impl IpcListener {
        /// Create new ipc server lsitener with custom [`syscall`](NamedPipe) and bind to `addr`
        pub async fn bind_with<A: AsRef<str>>(
            name: A,
            syscall: &'static dyn Network,
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

        /// Create new ipc server lsitener with global registered [`syscall`](NamedPipe) and bind to `addr`
        pub async fn bind<A: AsRef<str>>(name: A) -> io::Result<Self> {
            Self::bind_with(name, global_network()).await
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

    pub struct IpcStream(UnixStream);

    impl IpcStream {
        /// Create new client named pipe stream and connect to `addr`
        pub async fn connect_with<A: AsRef<str>>(
            name: A,
            syscall: &'static dyn Network,
        ) -> io::Result<Self> {
            let dir = temp_dir();
            let bind_path = dir.join("inter_process").join(name.as_ref());

            UnixStream::connect_with(bind_path, syscall)
                .await
                .map(|stream| IpcStream(stream))
        }

        /// Create new client named pipe stream and connect to `addr`
        pub async fn connect<A: AsRef<str>>(name: A) -> io::Result<Self> {
            Self::connect_with(name, global_network()).await
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

#[cfg(all(unix, feature = "unix_socket"))]
pub use unix::*;
