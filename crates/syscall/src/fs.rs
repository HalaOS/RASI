//! syscall for filesystem.

use std::{
    fs::{FileType, Metadata, Permissions},
    io,
    path::{Path, PathBuf},
    sync::OnceLock,
    task::Waker,
};

use futures::stream::BoxStream;

use crate::{Cancelable, CancelablePoll};

/// Filesystem-related system call interface
pub trait FileSystem: Cancelable + Sync + Send {
    /// Returns the canonical form of a path.
    /// The returned path is in absolute form with all intermediate components
    /// normalized and symbolic links resolved.
    /// This function is an async version of [`std::fs::canonicalize`].
    fn canonicalize(&self, waker: Waker, path: &Path) -> CancelablePoll<io::Result<PathBuf>>;

    /// Copies the contents and permissions of a file to a new location.
    /// On success, the total number of bytes copied is returned and equals
    /// the length of the to file after this operation.
    /// The old contents of to will be overwritten. If from and to both point
    /// to the same file, then the file will likely get truncated as a result of this operation.
    fn copy(&self, waker: Waker, from: &Path, to: &Path) -> CancelablePoll<io::Result<u64>>;

    /// Creates a new directory.
    /// Note that this function will only create the final directory in path.
    /// If you want to create all of its missing parent directories too, use
    /// the [`create_dir_all`](FileSystem::create_dir_all) function instead.
    ///
    /// This function is an async version of [`std::fs::create_dir`].
    fn create_dir(&self, waker: Waker, path: &Path) -> CancelablePoll<io::Result<()>>;

    /// Creates a new directory and all of its parents if they are missing.
    /// This function is an async version of [`std::fs::create_dir_all`].
    fn create_dir_all(&self, waker: Waker, path: &Path) -> CancelablePoll<io::Result<()>>;

    /// Creates a hard link on the filesystem.
    /// The dst path will be a link pointing to the src path. Note that operating
    /// systems often require these two paths to be located on the same filesystem.
    ///
    /// This function is an async version of [`std::fs::hard_link`].
    fn hard_link(&self, waker: Waker, from: &Path, to: &Path) -> CancelablePoll<io::Result<()>>;

    /// Reads metadata for a path.
    /// This function will traverse symbolic links to read metadata for the target
    /// file or directory. If you want to read metadata without following symbolic
    /// links, use symlink_metadata instead.
    ///
    /// This function is an async version of [`std::fs::metadata`].
    fn metadata(&self, waker: Waker, path: &Path) -> CancelablePoll<io::Result<Metadata>>;

    /// Returns a stream of entries in a directory.
    fn read_dir(
        &self,
        waker: Waker,
        path: &Path,
    ) -> CancelablePoll<io::Result<BoxStream<'static, Box<dyn DirEntry>>>>;

    /// Reads a symbolic link and returns the path it points to.
    ///
    /// This function is an async version of [`std::fs::read_link`].
    fn read_link(&self, waker: Waker, path: &Path) -> CancelablePoll<io::Result<PathBuf>>;

    /// Removes an empty directory,
    /// if the `path` is not an empty directory, use the function
    /// [`remove_dir_all`](FileSystem::remove_dir_all) instead.
    ///
    /// This function is an async version of std::fs::remove_dir.
    fn remove_dir(&self, waker: Waker, path: &Path) -> CancelablePoll<io::Result<()>>;

    /// Removes a directory and all of its contents.
    ///
    /// This function is an async version of [`std::fs::remove_dir_all`].
    fn remove_dir_all(&self, waker: Waker, path: &Path) -> CancelablePoll<io::Result<()>>;

    /// Removes a file.
    /// This function is an async version of [`std::fs::remove_file`].
    fn remove_file(&self, waker: Waker, path: &Path) -> CancelablePoll<io::Result<()>>;

    /// Renames a file or directory to a new location.
    /// If a file or directory already exists at the target location, it will be overwritten by this operation.
    /// This function is an async version of std::fs::rename.
    fn rename(&self, waker: Waker, frmo: &Path, to: &Path) -> CancelablePoll<io::Result<()>>;

    /// Changes the permissions of a file or directory.
    /// This function is an async version of [`std::fs::set_permissions`].
    fn set_permissions(
        &self,
        waker: Waker,
        path: &Path,
        perm: Permissions,
    ) -> CancelablePoll<io::Result<()>>;

    /// Reads metadata for a path without following symbolic links.
    /// If you want to follow symbolic links before reading metadata of the target file or directory,
    /// use [`metadata`](FileSystem::metadata) instead.
    ///
    /// This function is an async version of [`std::fs::symlink_metadata`].
    fn symlink_metadata(&self, waker: Waker, path: &Path) -> CancelablePoll<io::Result<Metadata>>;
}

/// DirEntry system call interface
pub trait DirEntry {
    /// Returns the bare name of this entry without the leading path.
    fn file_name(&self) -> String;

    /// Returns the full path to this entry.
    /// The full path is created by joining the original path passed to [`read_dir`](FileSystem::read_dir) with the name of this entry.
    fn path(&self) -> PathBuf;

    /// Reads the metadata for this entry.
    ///
    /// This function will traverse symbolic links to read the metadata.
    fn metadata(&self, waker: Waker) -> CancelablePoll<io::Result<Metadata>>;

    /// eads the file type for this entry.
    /// This function will not traverse symbolic links if this entry points at one.
    /// If you want to read metadata with following symbolic links, use [`metadata`](DirEntry::metadata) instead.
    fn file_type(&self, waker: Waker) -> CancelablePoll<io::Result<FileType>>;
}

static GLOBAL_FILESYSTEM: OnceLock<Box<dyn FileSystem>> = OnceLock::new();

/// Register provided [`FileSystem`] as global filesystem implementation.
///
/// # Panic
///
/// Multiple calls to this function are not permitted!!!
pub fn register_global_filesystem<FS: FileSystem + 'static>(fs: FS) {
    if GLOBAL_FILESYSTEM.set(Box::new(fs)).is_err() {
        panic!("Multiple calls to register_global_filesystem are not permitted!!!");
    }
}

/// Get global register [`FileSystem`] syscall interface.
///
/// # Panic
///
/// You should call [`register_global_filesystem`] first to register implementation,
/// otherwise this function will cause a panic with `Call register_global_filesystem first`
pub fn global_filesystem() -> &'static dyn FileSystem {
    GLOBAL_FILESYSTEM
        .get()
        .expect("Call register_global_filesystem first")
        .as_ref()
}
