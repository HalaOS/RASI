//! This mode is a wrapper of [`std::fs`] that implement [`FileSystem`] trait.
//!

use std::{
    fs::{DirEntry, File, OpenOptions, ReadDir},
    io::{Read, Seek, Write},
};

use rasi_syscall::{ready, register_global_filesystem, FileOpenMode, FileSystem, Handle};

/// The wrapper of [`std::fs`] that implement [`FileSystem`] trait.
#[derive(Default)]
pub struct StdFileSystem;

impl FileSystem for StdFileSystem {
    fn open_file(
        &self,
        _waker: std::task::Waker,
        path: &std::path::Path,
        open_mode: &rasi_syscall::FileOpenMode,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<rasi_syscall::Handle>> {
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
        file: &rasi_syscall::Handle,
        buf: &[u8],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<usize>> {
        let mut file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.write(buf))
    }

    fn file_read(
        &self,
        _waker: std::task::Waker,
        file: &rasi_syscall::Handle,
        buf: &mut [u8],
    ) -> rasi_syscall::CancelablePoll<std::io::Result<usize>> {
        let mut file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.read(buf))
    }

    fn file_flush(
        &self,
        _waker: std::task::Waker,
        file: &rasi_syscall::Handle,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<()>> {
        let mut file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.flush())
    }

    fn file_seek(
        &self,
        _waker: std::task::Waker,
        file: &rasi_syscall::Handle,
        pos: std::io::SeekFrom,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<u64>> {
        let mut file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.seek(pos))
    }

    fn file_meta(
        &self,
        _waker: std::task::Waker,
        file: &rasi_syscall::Handle,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<std::fs::Metadata>> {
        let file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.metadata())
    }

    fn file_set_permissions(
        &self,
        _waker: std::task::Waker,
        file: &rasi_syscall::Handle,
        perm: &std::fs::Permissions,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<()>> {
        let file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.set_permissions(perm.clone()))
    }

    fn file_set_len(
        &self,
        _waker: std::task::Waker,
        file: &rasi_syscall::Handle,
        size: u64,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<()>> {
        let file = file.downcast::<File>().expect("Expect std::fs::File");

        ready(|| file.set_len(size))
    }

    fn canonicalize(
        &self,
        _waker: std::task::Waker,
        path: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<std::path::PathBuf>> {
        ready(|| std::fs::canonicalize(path))
    }

    fn copy(
        &self,
        _waker: std::task::Waker,
        from: &std::path::Path,
        to: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<u64>> {
        ready(|| std::fs::copy(from, to))
    }

    fn create_dir(
        &self,
        _waker: std::task::Waker,
        path: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::create_dir(path))
    }

    fn create_dir_all(
        &self,
        _waker: std::task::Waker,
        path: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::create_dir_all(path))
    }

    fn hard_link(
        &self,
        _waker: std::task::Waker,
        from: &std::path::Path,
        to: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::hard_link(from, to))
    }

    fn metadata(
        &self,
        _waker: std::task::Waker,
        path: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<std::fs::Metadata>> {
        ready(|| std::fs::metadata(path))
    }

    fn read_dir(
        &self,
        _waker: std::task::Waker,
        path: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<rasi_syscall::Handle>> {
        ready(|| {
            let read_dir = std::fs::read_dir(path)?;

            Ok(Handle::new(parking_lot::Mutex::new(read_dir)))
        })
    }

    fn dir_entry_next(
        &self,
        _waker: std::task::Waker,
        read_dir_handle: &rasi_syscall::Handle,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<Option<rasi_syscall::Handle>>> {
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

    fn dir_entry_file_name(&self, entry: &rasi_syscall::Handle) -> String {
        let dir_entry = entry.downcast::<DirEntry>().expect("Expect ReadDir");

        dir_entry.file_name().to_string_lossy().to_string()
    }

    fn dir_entry_path(&self, entry: &rasi_syscall::Handle) -> std::path::PathBuf {
        let dir_entry = entry.downcast::<DirEntry>().expect("Expect ReadDir");

        dir_entry.path()
    }

    fn dir_entry_metadata(
        &self,
        _waker: std::task::Waker,
        entry: &rasi_syscall::Handle,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<std::fs::Metadata>> {
        let dir_entry = entry.downcast::<DirEntry>().expect("Expect ReadDir");

        ready(|| dir_entry.metadata())
    }

    fn dir_entry_file_type(
        &self,
        _waker: std::task::Waker,
        entry: &rasi_syscall::Handle,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<std::fs::FileType>> {
        let dir_entry = entry.downcast::<DirEntry>().expect("Expect ReadDir");

        ready(|| dir_entry.file_type())
    }

    fn read_link(
        &self,
        _waker: std::task::Waker,
        path: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<std::path::PathBuf>> {
        ready(|| std::fs::read_link(path))
    }

    fn remove_dir(
        &self,
        _waker: std::task::Waker,
        path: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::remove_dir(path))
    }

    fn remove_dir_all(
        &self,
        _waker: std::task::Waker,
        path: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::remove_dir_all(path))
    }

    fn remove_file(
        &self,
        _waker: std::task::Waker,
        path: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::remove_file(path))
    }

    fn rename(
        &self,
        _waker: std::task::Waker,
        from: &std::path::Path,
        to: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::rename(from, to))
    }

    fn set_permissions(
        &self,
        _waker: std::task::Waker,
        path: &std::path::Path,
        perm: &std::fs::Permissions,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<()>> {
        ready(|| std::fs::set_permissions(path, perm.clone()))
    }

    fn symlink_metadata(
        &self,
        _waker: std::task::Waker,
        path: &std::path::Path,
    ) -> rasi_syscall::CancelablePoll<std::io::Result<std::fs::Metadata>> {
        ready(|| std::fs::symlink_metadata(path))
    }
}

/// This function using [`register_global_filesystem`] to register the [`StdFileSystem`] to global registry.
///
/// So you may not call this function twice, otherwise will cause a panic. [`read more`](`register_global_filesystem`)
pub fn register_std_filesystem() {
    register_global_filesystem(StdFileSystem)
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use rasi_spec::fs::run_fs_spec;

    use super::*;

    static INIT: OnceLock<Box<dyn rasi_syscall::FileSystem>> = OnceLock::new();

    fn get_syscall() -> &'static dyn rasi_syscall::FileSystem {
        INIT.get_or_init(|| Box::new(StdFileSystem::default()))
            .as_ref()
    }

    #[futures_test::test]
    async fn test_std_fs() {
        run_fs_spec(get_syscall()).await;
    }
}
