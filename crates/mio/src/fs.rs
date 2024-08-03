use std::{
    fs::OpenOptions,
    io::{Read, Seek, Write},
    sync::Mutex,
    task::Poll,
};

use rasi::fs::{register_fs_driver, FileOpenMode};

use crate::utils::ready;

pub struct MioFileSystemDriver;

impl rasi::fs::syscall::Driver for MioFileSystemDriver {
    fn open_file(
        &self,
        path: &std::path::Path,
        open_mode: rasi::fs::FileOpenMode,
    ) -> std::io::Result<rasi::fs::File> {
        let mut ops = OpenOptions::new();

        if open_mode.contains(FileOpenMode::Create) {
            ops.create(true);
        }

        if open_mode.contains(FileOpenMode::CreateNew) {
            ops.create_new(true);
        }

        if open_mode.contains(FileOpenMode::Append) {
            ops.append(true);
        }

        if open_mode.contains(FileOpenMode::Readable) {
            ops.read(true);
        }

        if open_mode.contains(FileOpenMode::Truncate) {
            ops.truncate(true);
        }

        if open_mode.contains(FileOpenMode::Writable) {
            ops.write(true);
        }

        let file = ops.open(path)?;

        Ok(MioFile(file).into())
    }

    fn canonicalize(&self, path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        path.canonicalize()
    }

    fn poll_copy(
        &self,
        _cx: &mut std::task::Context<'_>,
        from: &std::path::Path,
        to: &std::path::Path,
    ) -> std::task::Poll<std::io::Result<u64>> {
        ready(|| std::fs::copy(from, to))
    }

    fn poll_create_dir(
        &self,
        _cx: &mut std::task::Context<'_>,
        path: &std::path::Path,
    ) -> std::task::Poll<std::io::Result<()>> {
        ready(|| std::fs::create_dir(path))
    }

    fn poll_create_dir_all(
        &self,
        _cx: &mut std::task::Context<'_>,
        path: &std::path::Path,
    ) -> std::task::Poll<std::io::Result<()>> {
        ready(|| std::fs::create_dir_all(path))
    }

    fn poll_hard_link(
        &self,
        _cx: &mut std::task::Context<'_>,
        from: &std::path::Path,
        to: &std::path::Path,
    ) -> std::task::Poll<std::io::Result<()>> {
        ready(|| std::fs::hard_link(from, to))
    }

    fn poll_metadata(
        &self,
        _cx: &mut std::task::Context<'_>,
        path: &std::path::Path,
    ) -> std::task::Poll<std::io::Result<std::fs::Metadata>> {
        ready(|| std::fs::metadata(path))
    }

    fn poll_read_link(
        &self,
        _cx: &mut std::task::Context<'_>,
        path: &std::path::Path,
    ) -> std::task::Poll<std::io::Result<std::path::PathBuf>> {
        ready(|| std::fs::read_link(path))
    }

    fn poll_remove_dir(
        &self,
        _cx: &mut std::task::Context<'_>,
        path: &std::path::Path,
    ) -> std::task::Poll<std::io::Result<()>> {
        ready(|| std::fs::remove_dir(path))
    }

    fn poll_remove_dir_all(
        &self,
        _cx: &mut std::task::Context<'_>,
        path: &std::path::Path,
    ) -> std::task::Poll<std::io::Result<()>> {
        ready(|| std::fs::remove_dir_all(path))
    }

    fn poll_remove_file(
        &self,
        _cx: &mut std::task::Context<'_>,
        path: &std::path::Path,
    ) -> std::task::Poll<std::io::Result<()>> {
        ready(|| std::fs::remove_file(path))
    }

    fn poll_rename(
        &self,
        _cx: &mut std::task::Context<'_>,
        from: &std::path::Path,
        to: &std::path::Path,
    ) -> std::task::Poll<std::io::Result<()>> {
        ready(|| std::fs::rename(from, to))
    }

    fn poll_set_permissions(
        &self,
        _cx: &mut std::task::Context<'_>,
        path: &std::path::Path,
        perm: &std::fs::Permissions,
    ) -> std::task::Poll<std::io::Result<()>> {
        ready(|| std::fs::set_permissions(path, perm.clone()))
    }

    fn poll_symlink_metadata(
        &self,
        _cx: &mut std::task::Context<'_>,
        path: &std::path::Path,
    ) -> std::task::Poll<std::io::Result<std::fs::Metadata>> {
        ready(|| std::fs::symlink_metadata(path))
    }

    fn read_dir(&self, path: &std::path::Path) -> std::io::Result<rasi::fs::ReadDir> {
        Ok(MioReadDir(Mutex::new(std::fs::read_dir(path)?)).into())
    }

    #[cfg(windows)]
    /// Opens the named pipe identified by `addr`.
    fn named_pipe_client_open(
        &self,
        addr: &std::ffi::OsStr,
    ) -> std::io::Result<rasi::fs::windows::NamedPipeStream> {
        use std::{
            os::windows::io::FromRawHandle,
            ptr::{null, null_mut},
        };

        use mio::{Interest, Token};
        use windows_sys::Win32::{
            Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE},
            Storage::FileSystem::{CreateFileW, FILE_FLAG_OVERLAPPED, OPEN_EXISTING},
        };

        use crate::{net::MioSocket, reactor::global_reactor, token::TokenSequence};

        let addr = windows::encode_addr(addr);

        let desired_access = GENERIC_READ | GENERIC_WRITE;

        let flag = FILE_FLAG_OVERLAPPED;

        unsafe {
            let handle = CreateFileW(
                addr.as_ptr(),
                desired_access,
                0,
                null(),
                OPEN_EXISTING,
                flag,
                std::ptr::null_mut(),
            );

            if handle == INVALID_HANDLE_VALUE {
                return Err(std::io::Error::last_os_error());
            }

            let mut socket = mio::windows::NamedPipe::from_raw_handle(handle as _);

            let token = Token::next();

            global_reactor().register(
                &mut socket,
                token,
                Interest::READABLE.add(Interest::WRITABLE),
            )?;

            Ok(windows::MioNamedPipeStream(MioSocket { token, socket }, false).into())
        }
    }

    #[cfg(windows)]
    /// Creates the named pipe identified by `addr` for use as a server.
    ///
    /// This uses the [`CreateNamedPipe`] function.
    fn named_pipe_server_create(
        &self,
        addr: &std::ffi::OsStr,
    ) -> std::io::Result<rasi::fs::windows::NamedPipeListener> {
        Ok(windows::MioNamedPipeListener::new(addr).into())
    }
}

#[cfg(windows)]
mod windows {
    use std::{
        ffi::OsStr,
        io::{Error, Read, Write},
        os::windows::{ffi::OsStrExt, io::FromRawHandle},
        ptr::null,
        sync::Mutex,
        task::Poll,
    };

    use mio::{Interest, Token};
    use windows_sys::Win32::{
        Foundation::INVALID_HANDLE_VALUE,
        Storage::FileSystem::{FILE_FLAG_OVERLAPPED, PIPE_ACCESS_DUPLEX},
        System::Pipes::{
            CreateNamedPipeW, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
        },
    };

    use crate::{
        net::MioSocket, reactor::global_reactor, token::TokenSequence, utils::would_block,
    };

    pub fn encode_addr(addr: &OsStr) -> Box<[u16]> {
        let len = addr.encode_wide().count();
        let mut vec = Vec::with_capacity(len + 1);
        vec.extend(addr.encode_wide());
        vec.push(0);
        vec.into_boxed_slice()
    }

    pub struct MioNamedPipeListener {
        addr: Box<[u16]>,
        buffer_size: u32,
        next: Mutex<Option<MioSocket<mio::windows::NamedPipe>>>,
    }

    impl MioNamedPipeListener {
        pub fn new(addr: &std::ffi::OsStr) -> Self {
            MioNamedPipeListener {
                addr: encode_addr(addr),
                buffer_size: 512,
                next: Default::default(),
            }
        }

        fn create_stream(&self) -> std::io::Result<()> {
            let mut next = self.next.lock().unwrap();

            if next.is_some() {
                return Ok(());
            }

            let pipe_mode = PIPE_TYPE_BYTE | PIPE_REJECT_REMOTE_CLIENTS;

            let open_mode = FILE_FLAG_OVERLAPPED | PIPE_ACCESS_DUPLEX;

            unsafe {
                let handle = CreateNamedPipeW(
                    self.addr.as_ptr(),
                    open_mode,
                    pipe_mode,
                    PIPE_UNLIMITED_INSTANCES,
                    self.buffer_size,
                    self.buffer_size,
                    0,
                    null(),
                );

                if handle == INVALID_HANDLE_VALUE {
                    return Err(Error::last_os_error());
                }

                let mut socket = mio::windows::NamedPipe::from_raw_handle(handle as _);

                let token = Token::next();

                global_reactor().register(
                    &mut socket,
                    token,
                    Interest::READABLE.add(Interest::WRITABLE),
                )?;

                *next = Some(MioSocket { token, socket });

                return Ok(());
            }
        }
    }

    impl rasi::fs::syscall::windows::DriverNamedPipeListener for MioNamedPipeListener {
        fn poll_ready(
            &self,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_next(
            &self,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<rasi::fs::windows::NamedPipeStream>> {
            self.create_stream()?;

            let stream = self.next.lock().unwrap().take().unwrap();

            global_reactor().once(stream.token, Interest::WRITABLE, cx.waker().clone());

            loop {
                match stream.connect() {
                    Ok(_) => {
                        return {
                            global_reactor().remove_listeners(stream.token, Interest::WRITABLE);

                            Poll::Ready(Ok(MioNamedPipeStream(stream, true).into()))
                        }
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        *self.next.lock().unwrap() = Some(stream);
                        return Poll::Pending;
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {
                        continue;
                    }
                    Err(err) => {
                        global_reactor().remove_listeners(stream.token, Interest::WRITABLE);
                        return Poll::Ready(Err(err));
                    }
                }
            }
        }
    }

    pub struct MioNamedPipeStream(pub MioSocket<mio::windows::NamedPipe>, pub bool);

    impl rasi::fs::syscall::windows::DriverNamedPipeStream for MioNamedPipeStream {
        fn poll_ready(
            &self,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_write(
            &self,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            would_block(self.0.token, cx.waker().clone(), Interest::WRITABLE, || {
                (&self.0.socket).write(buf)
            })
        }

        fn poll_read(
            &self,
            cx: &mut std::task::Context<'_>,
            buf: &mut [u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            would_block(self.0.token, cx.waker().clone(), Interest::READABLE, || {
                (&self.0.socket).read(buf)
            })
        }

        fn poll_close(
            &self,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            if self.1 {
                self.0.socket.disconnect()?;
            }

            Poll::Ready(Ok(()))
        }
    }
}

struct MioFile(std::fs::File);

impl rasi::fs::syscall::DriverFile for MioFile {
    fn poll_ready(&self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_write(
        &self,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        ready(|| (&self.0).write(buf))
    }

    fn poll_read(
        &self,
        _cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        ready(|| (&self.0).read(buf))
    }

    fn poll_flush(&self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        ready(|| (&self.0).flush())
    }

    fn poll_seek(
        &self,
        _cx: &mut std::task::Context<'_>,
        pos: std::io::SeekFrom,
    ) -> std::task::Poll<std::io::Result<u64>> {
        ready(|| (&self.0).seek(pos))
    }

    fn poll_meta(
        &self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<std::fs::Metadata>> {
        ready(|| (&self.0).metadata())
    }

    fn poll_set_permissions(
        &self,
        _cx: &mut std::task::Context<'_>,
        perm: &std::fs::Permissions,
    ) -> std::task::Poll<std::io::Result<()>> {
        ready(|| (&self.0).set_permissions(perm.clone()))
    }

    fn poll_set_len(
        &self,
        _cx: &mut std::task::Context<'_>,
        size: u64,
    ) -> std::task::Poll<std::io::Result<()>> {
        ready(|| (&self.0).set_len(size))
    }
}

struct MioReadDir(Mutex<std::fs::ReadDir>);

impl rasi::fs::syscall::DriverReadDir for MioReadDir {
    fn poll_ready(&self, _cx: &mut std::task::Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_next(
        &self,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<std::io::Result<rasi::fs::DirEntry>>> {
        ready(|| {
            self.0
                .lock()
                .unwrap()
                .next()
                .map(|r| r.map(|e| MioDirEntry(e).into()))
        })
    }
}

struct MioDirEntry(std::fs::DirEntry);

impl rasi::fs::syscall::DriverDirEntry for MioDirEntry {
    fn name(&self) -> String {
        self.0.file_name().to_string_lossy().into_owned()
    }

    fn path(&self) -> std::path::PathBuf {
        self.0.path()
    }

    fn meta(&self) -> std::io::Result<std::fs::Metadata> {
        self.0.metadata()
    }

    fn file_type(&self) -> std::io::Result<std::fs::FileType> {
        self.0.file_type()
    }
}

/// This function using [`register_fs_driver`] to register the `MioFileSystemDriver` to global registry.
///
/// So you may not call this function twice, otherwise will cause a panic. [`read more`](`register_fs_driver`)
pub fn register_mio_filesystem() {
    register_fs_driver(MioFileSystemDriver)
}

#[cfg(test)]
mod tests {

    use rasi_spec::fs::run_fs_spec;

    use super::*;

    #[futures_test::test]
    async fn test_mio_fs() {
        static DRIVER: MioFileSystemDriver = MioFileSystemDriver;

        run_fs_spec(&DRIVER).await;

        #[cfg(windows)]
        rasi_spec::ipc::run_ipc_spec(&DRIVER).await;
    }
}
