//! This mode is a wrapper of [`std::fs`] that implement [`FileSystem`] trait.
//!

use std::{
    fs::{DirEntry, File, OpenOptions, ReadDir},
    io::{Read, Seek, Write},
};

use rasi_syscall::{
    path::*, ready, register_global_filesystem, CancelablePoll, FileOpenMode, FileSystem, Handle,
};

/// The wrapper of [`std::fs`] that implement [`FileSystem`] trait.
#[derive(Default)]
pub struct StdFileSystem;

impl FileSystem for StdFileSystem {
    fn open_file(
        &self,
        _waker: std::task::Waker,
        path: &Path,
        open_mode: &FileOpenMode,
    ) -> CancelablePoll<std::io::Result<Handle>> {
        ready(|| {
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

            Ok(Handle::new(file))
        })
    }

    fn file_write(
        &self,
        _waker: std::task::Waker,
        file: &Handle,
        buf: &[u8],
    ) -> CancelablePoll<std::io::Result<usize>> {
        let mut file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.write(buf))
    }

    fn file_read(
        &self,
        _waker: std::task::Waker,
        file: &Handle,
        buf: &mut [u8],
    ) -> CancelablePoll<std::io::Result<usize>> {
        let mut file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.read(buf))
    }

    fn file_flush(
        &self,
        _waker: std::task::Waker,
        file: &Handle,
    ) -> CancelablePoll<std::io::Result<()>> {
        let mut file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.flush())
    }

    fn file_seek(
        &self,
        _waker: std::task::Waker,
        file: &Handle,
        pos: std::io::SeekFrom,
    ) -> CancelablePoll<std::io::Result<u64>> {
        let mut file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.seek(pos))
    }

    fn file_meta(
        &self,
        _waker: std::task::Waker,
        file: &Handle,
    ) -> CancelablePoll<std::io::Result<std::fs::Metadata>> {
        let file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.metadata())
    }

    fn file_set_permissions(
        &self,
        _waker: std::task::Waker,
        file: &Handle,
        perm: &std::fs::Permissions,
    ) -> CancelablePoll<std::io::Result<()>> {
        let file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.set_permissions(perm.clone()))
    }

    fn file_set_len(
        &self,
        _waker: std::task::Waker,
        file: &Handle,
        size: u64,
    ) -> CancelablePoll<std::io::Result<()>> {
        let file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.set_len(size))
    }

    fn canonicalize(
        &self,
        _waker: std::task::Waker,
        path: &Path,
    ) -> CancelablePoll<std::io::Result<PathBuf>> {
        let path: &std::path::Path = path.as_ref();

        ready(|| std::fs::canonicalize(path).map(Into::into))
    }

    fn copy(
        &self,
        _waker: std::task::Waker,
        from: &Path,
        to: &Path,
    ) -> CancelablePoll<std::io::Result<u64>> {
        ready(|| std::fs::copy(from, to))
    }

    fn create_dir(
        &self,
        _waker: std::task::Waker,
        path: &Path,
    ) -> CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::create_dir(path))
    }

    fn create_dir_all(
        &self,
        _waker: std::task::Waker,
        path: &Path,
    ) -> CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::create_dir_all(path))
    }

    fn hard_link(
        &self,
        _waker: std::task::Waker,
        from: &Path,
        to: &Path,
    ) -> CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::hard_link(from, to))
    }

    fn metadata(
        &self,
        _waker: std::task::Waker,
        path: &Path,
    ) -> CancelablePoll<std::io::Result<std::fs::Metadata>> {
        ready(|| std::fs::metadata(path))
    }

    fn read_dir(
        &self,
        _waker: std::task::Waker,
        path: &Path,
    ) -> CancelablePoll<std::io::Result<Handle>> {
        ready(|| {
            let read_dir = std::fs::read_dir(path)?;

            Ok(Handle::new(parking_lot::Mutex::new(read_dir)))
        })
    }

    fn dir_entry_next(
        &self,
        _waker: std::task::Waker,
        read_dir_handle: &Handle,
    ) -> CancelablePoll<std::io::Result<Option<Handle>>> {
        let read_dir = read_dir_handle
            .downcast::<parking_lot::Mutex<ReadDir>>()
            .expect("Expect ReadDir");

        ready(|| {
            if let Some(next) = read_dir.lock().next() {
                match next {
                    Ok(next) => Ok(Some(Handle::new(next))),
                    Err(err) => Err(err),
                }
            } else {
                Ok(None)
            }
        })
    }

    fn dir_entry_file_name(&self, entry: &Handle) -> String {
        let dir_entry = entry.downcast::<DirEntry>().expect("Expect ReadDir");

        dir_entry.file_name().to_string_lossy().to_string()
    }

    fn dir_entry_path(&self, entry: &Handle) -> PathBuf {
        let dir_entry = entry.downcast::<DirEntry>().expect("Expect ReadDir");

        dir_entry.path().into()
    }

    fn dir_entry_metadata(
        &self,
        _waker: std::task::Waker,
        entry: &Handle,
    ) -> CancelablePoll<std::io::Result<std::fs::Metadata>> {
        let dir_entry = entry.downcast::<DirEntry>().expect("Expect ReadDir");

        ready(|| dir_entry.metadata())
    }

    fn dir_entry_file_type(
        &self,
        _waker: std::task::Waker,
        entry: &Handle,
    ) -> CancelablePoll<std::io::Result<std::fs::FileType>> {
        let dir_entry = entry.downcast::<DirEntry>().expect("Expect ReadDir");

        ready(|| dir_entry.file_type())
    }

    fn read_link(
        &self,
        _waker: std::task::Waker,
        path: &Path,
    ) -> CancelablePoll<std::io::Result<PathBuf>> {
        let path: &std::path::Path = path.as_ref();
        ready(|| std::fs::read_link(path).map(Into::into))
    }

    fn remove_dir(
        &self,
        _waker: std::task::Waker,
        path: &Path,
    ) -> CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::remove_dir(path))
    }

    fn remove_dir_all(
        &self,
        _waker: std::task::Waker,
        path: &Path,
    ) -> CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::remove_dir_all(path))
    }

    fn remove_file(
        &self,
        _waker: std::task::Waker,
        path: &Path,
    ) -> CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::remove_file(path))
    }

    fn rename(
        &self,
        _waker: std::task::Waker,
        from: &Path,
        to: &Path,
    ) -> CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::rename(from, to))
    }

    fn set_permissions(
        &self,
        _waker: std::task::Waker,
        path: &Path,
        perm: &std::fs::Permissions,
    ) -> CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::set_permissions(path, perm.clone()))
    }

    fn symlink_metadata(
        &self,
        _waker: std::task::Waker,
        path: &Path,
    ) -> CancelablePoll<std::io::Result<std::fs::Metadata>> {
        ready(|| std::fs::symlink_metadata(path))
    }
}

/// This function using [`register_global_filesystem`] to register the [`StdFileSystem`] to global registry.
///
/// So you may not call this function twice, otherwise will cause a panic. [`read more`](`register_global_filesystem`)
pub fn register_std_filesystem() {
    register_global_filesystem(StdFileSystem)
}

#[cfg(all(windows, feature = "windows_named_pipe"))]
mod windows {
    use crate::{
        reactor::{global_reactor, would_block, MioSocket},
        TokenSequence,
    };

    use super::ready;

    use std::{
        ffi::OsStr,
        io::{self, Read, Write},
        ops::Deref,
        os::windows::{ffi::OsStrExt, io::FromRawHandle},
        ptr::null,
    };

    use mio::{Interest, Token};
    use rasi_syscall::{register_global_named_pipe, Handle, NamedPipe};
    use windows_sys::Win32::{
        Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CreateFileW, FILE_FLAG_OVERLAPPED, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
        },
        System::Pipes::{
            CreateNamedPipeW, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
        },
    };

    pub struct MioNamedPipe {
        buffer_size: u32,
    }

    impl Default for MioNamedPipe {
        fn default() -> Self {
            Self { buffer_size: 512 }
        }
    }

    impl MioNamedPipe {
        /// Create default `MioNamedPipe` configuration.
        pub fn new() -> Self {
            Default::default()
        }

        pub fn register(self) {
            register_global_named_pipe(self);
        }
    }

    fn encode_addr(addr: &OsStr) -> Box<[u16]> {
        let len = addr.encode_wide().count();
        let mut vec = Vec::with_capacity(len + 1);
        vec.extend(addr.encode_wide());
        vec.push(0);
        vec.into_boxed_slice()
    }

    impl NamedPipe for MioNamedPipe {
        fn client_open(
            &self,
            _waker: std::task::Waker,
            addr: &std::ffi::OsStr,
        ) -> rasi_syscall::CancelablePoll<std::io::Result<rasi_syscall::Handle>> {
            let addr = encode_addr(addr);

            let desired_access = GENERIC_READ | GENERIC_WRITE;

            let flag = FILE_FLAG_OVERLAPPED;

            ready(|| unsafe {
                let handle = CreateFileW(
                    addr.as_ptr(),
                    desired_access,
                    0,
                    null(),
                    OPEN_EXISTING,
                    flag,
                    0,
                );

                if handle == INVALID_HANDLE_VALUE {
                    return Err(io::Error::last_os_error());
                }

                let mut socket = mio::windows::NamedPipe::from_raw_handle(handle as _);

                let token = Token::next();

                global_reactor().register(
                    &mut socket,
                    token,
                    Interest::READABLE.add(Interest::WRITABLE),
                )?;

                Ok(Handle::new(MioSocket::from((token, socket))))
            })
        }

        fn server_create(
            &self,
            _waker: std::task::Waker,
            addr: &std::ffi::OsStr,
        ) -> rasi_syscall::CancelablePoll<std::io::Result<rasi_syscall::Handle>> {
            let addr = encode_addr(addr);

            let pipe_mode = PIPE_TYPE_BYTE | PIPE_REJECT_REMOTE_CLIENTS;

            let open_mode = FILE_FLAG_OVERLAPPED | PIPE_ACCESS_DUPLEX;

            ready(|| unsafe {
                let handle = CreateNamedPipeW(
                    addr.as_ptr(),
                    open_mode,
                    pipe_mode,
                    PIPE_UNLIMITED_INSTANCES,
                    self.buffer_size,
                    self.buffer_size,
                    0,
                    null(),
                );

                if handle == INVALID_HANDLE_VALUE {
                    return Err(io::Error::last_os_error());
                }

                let mut socket = mio::windows::NamedPipe::from_raw_handle(handle as _);

                let token = Token::next();

                global_reactor().register(
                    &mut socket,
                    token,
                    Interest::READABLE.add(Interest::WRITABLE),
                )?;

                Ok(Handle::new(MioSocket::from((token, socket))))
            })
        }

        fn server_accept(
            &self,
            waker: std::task::Waker,
            socket: &rasi_syscall::Handle,
        ) -> rasi_syscall::CancelablePoll<std::io::Result<()>> {
            let socket = socket
                .downcast::<MioSocket<mio::windows::NamedPipe>>()
                .expect("Expect NamedPipe.");

            would_block(socket.token, waker, Interest::WRITABLE, || socket.connect())
        }

        fn server_disconnect(&self, socket: &rasi_syscall::Handle) -> std::io::Result<()> {
            let socket = socket
                .downcast::<MioSocket<mio::windows::NamedPipe>>()
                .expect("Expect NamedPipe.");

            socket.disconnect()
        }

        fn write(
            &self,
            waker: std::task::Waker,
            socket: &rasi_syscall::Handle,
            buf: &[u8],
        ) -> rasi_syscall::CancelablePoll<std::io::Result<usize>> {
            let socket = socket
                .downcast::<MioSocket<mio::windows::NamedPipe>>()
                .expect("Expect NamedPipe.");

            would_block(socket.token, waker, Interest::WRITABLE, || {
                socket.deref().write(buf)
            })
        }

        fn read(
            &self,
            waker: std::task::Waker,
            socket: &rasi_syscall::Handle,
            buf: &mut [u8],
        ) -> rasi_syscall::CancelablePoll<std::io::Result<usize>> {
            let socket = socket
                .downcast::<MioSocket<mio::windows::NamedPipe>>()
                .expect("Expect NamedPipe.");

            would_block(socket.token, waker, Interest::READABLE, || {
                socket.deref().read(buf)
            })
        }
    }
}

#[cfg(windows)]
pub use windows::*;

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use rasi_spec::fs::run_fs_spec;
    use rasi_syscall::NamedPipe;

    use super::*;

    static INIT: OnceLock<Box<dyn FileSystem>> = OnceLock::new();

    fn get_syscall() -> &'static dyn FileSystem {
        INIT.get_or_init(|| Box::new(StdFileSystem::default()))
            .as_ref()
    }

    #[futures_test::test]
    async fn test_std_fs() {
        run_fs_spec(get_syscall()).await;
    }

    static INIT_NAMED_PIPE: OnceLock<Box<dyn NamedPipe>> = OnceLock::new();

    fn get_named_pipe_syscall() -> &'static dyn NamedPipe {
        INIT_NAMED_PIPE
            .get_or_init(|| Box::new(MioNamedPipe::default()))
            .as_ref()
    }

    #[cfg(all(windows, feature = "windows_named_pipe"))]
    #[futures_test::test]
    async fn test_named_pipe() {
        use rasi_spec::windows::run_windows_spec;

        run_windows_spec(get_named_pipe_syscall()).await;
    }
}
