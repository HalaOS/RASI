//! syscall for filesystem.

use std::{
    fs::{FileType, Metadata, Permissions},
    io::{self, SeekFrom},
    sync::OnceLock,
    task::Waker,
};

use bitmask_enum::bitmask;

use crate::path::{Path, PathBuf};
use crate::{CancelablePoll, Handle};

/// A bitmask for open file.
///
/// See [`open_file`](FileSystem::open_file) for more information.
#[bitmask(u8)]
pub enum FileOpenMode {
    /// Configures the option for append mode.
    ///
    /// When set to true, this option means the file will be writable after opening
    /// and the file cursor will be moved to the end of file before every write operaiton.
    Append,
    /// Configures the option for write mode.
    /// If the file already exists, write calls on it will overwrite the previous contents without truncating it.
    Writable,
    /// Configures the option for read mode.
    /// When set to true, this option means the file will be readable after opening.
    Readable,

    /// Configures the option for creating a new file if it doesn’t exist.
    /// When set to true, this option means a new file will be created if it doesn’t exist.
    /// The file must be opened in [`Writable`](FileOpenMode::Writable)
    /// or [`Append`](FileOpenMode::Append) mode for file creation to work.
    Create,

    /// Configures the option for creating a new file or failing if it already exists.
    /// When set to true, this option means a new file will be created, or the open
    /// operation will fail if the file already exists.
    /// The file must be opened in [`Writable`](FileOpenMode::Writable)
    /// or [`Append`](FileOpenMode::Append) mode for file creation to work.
    CreateNew,

    /// Configures the option for truncating the previous file.
    /// When set to true, the file will be truncated to the length of 0 bytes.
    /// The file must be opened in [`Writable`](FileOpenMode::Writable)
    /// or [`Append`](FileOpenMode::Append) mode for file creation to work.
    Truncate,
}

/// Filesystem-related system call interface
pub trait FileSystem: Sync + Send {
    /// Opens a file with [`FileOpenMode`]
    fn open_file(
        &self,
        waker: Waker,
        path: &Path,
        open_mode: &FileOpenMode,
    ) -> CancelablePoll<io::Result<Handle>>;

    /// Write a buffer into this writer, returning how many bytes were written
    fn file_write(
        &self,
        waker: Waker,
        file: &Handle,
        buf: &[u8],
    ) -> CancelablePoll<io::Result<usize>>;

    /// Pull some bytes from this source into the specified buffer, returning how many bytes were read.
    fn file_read(
        &self,
        waker: Waker,
        file: &Handle,
        buf: &mut [u8],
    ) -> CancelablePoll<io::Result<usize>>;

    /// Attempts to sync all OS-internal metadata to disk.
    ///
    /// This function will attempt to ensure that all in-memory data reaches the filesystem before returning.
    ///
    /// This can be used to handle errors that would otherwise only be caught when the File is closed.
    /// Dropping a file will ignore errors in synchronizing this in-memory data.
    fn file_flush(&self, waker: Waker, file: &Handle) -> CancelablePoll<io::Result<()>>;

    /// Seek to an offset, in bytes, in a stream.
    ///
    /// A seek beyond the end of a stream is allowed, but behavior is defined by the implementation.
    ///
    /// If the seek operation completed successfully, this method returns the new position from the
    /// start of the stream. That position can be used later with [`SeekFrom::Start`].
    ///
    /// # Errors
    /// Seeking can fail, for example because it might involve flushing a buffer.
    ///
    /// Seeking to a negative offset is considered an error.
    fn file_seek(
        &self,
        waker: Waker,
        file: &Handle,
        pos: SeekFrom,
    ) -> CancelablePoll<io::Result<u64>>;

    ///  Reads the file's metadata.
    fn file_meta(&self, waker: Waker, file: &Handle) -> CancelablePoll<io::Result<Metadata>>;

    /// Changes the permissions on the file.
    fn file_set_permissions(
        &self,
        waker: Waker,
        file: &Handle,
        perm: &Permissions,
    ) -> CancelablePoll<io::Result<()>>;

    /// Truncates or extends the file.
    ///
    /// If `size` is less than the current file size, then the file will be truncated. If it is
    /// greater than the current file size, then the file will be extended to `size` and have all
    /// intermediate data filled with zeros.
    ///
    /// The file's cursor stays at the same position, even if the cursor ends up being past the end
    /// of the file after this operation.
    ///
    fn file_set_len(
        &self,
        waker: Waker,
        file: &Handle,
        size: u64,
    ) -> CancelablePoll<io::Result<()>>;

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

    /// Returns a iterator handle of entries in a directory.
    ///
    /// See [`dir_entry_next`](FileSystem::dir_entry_next) for more information about iteration.
    fn read_dir(&self, waker: Waker, path: &Path) -> CancelablePoll<io::Result<Handle>>;

    /// Advances the directory entry iterator and returns the next value.
    ///
    /// Returns None when iteration is finished.
    ///
    /// You can create a directory entry iterator by call function [`read_dir`](FileSystem::read_dir)
    fn dir_entry_next(
        &self,
        waker: Waker,
        read_dir_handle: &Handle,
    ) -> CancelablePoll<io::Result<Option<Handle>>>;

    /// Returns the bare name of this entry without the leading path.
    fn dir_entry_file_name(&self, entry: &Handle) -> String;

    /// Returns the full path to this entry.
    /// The full path is created by joining the original path passed to [`read_dir`](FileSystem::read_dir) with the name of this entry.
    fn dir_entry_path(&self, entry: &Handle) -> PathBuf;

    /// Reads the metadata for this entry.
    ///
    /// This function will traverse symbolic links to read the metadata.
    fn dir_entry_metadata(
        &self,
        waker: Waker,
        entry: &Handle,
    ) -> CancelablePoll<io::Result<Metadata>>;

    /// eads the file type for this entry.
    /// This function will not traverse symbolic links if this entry points at one.
    /// If you want to read metadata with following symbolic links, use [`dir_entry_metadata`](FileSystem::dir_entry_metadata) instead.
    fn dir_entry_file_type(
        &self,
        waker: Waker,
        entry: &Handle,
    ) -> CancelablePoll<io::Result<FileType>>;

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
    fn rename(&self, waker: Waker, from: &Path, to: &Path) -> CancelablePoll<io::Result<()>>;

    /// Changes the permissions of a file or directory.
    /// This function is an async version of [`std::fs::set_permissions`].
    fn set_permissions(
        &self,
        waker: Waker,
        path: &Path,
        perm: &Permissions,
    ) -> CancelablePoll<io::Result<()>>;

    /// Reads metadata for a path without following symbolic links.
    /// If you want to follow symbolic links before reading metadata of the target file or directory,
    /// use [`metadata`](FileSystem::metadata) instead.
    ///
    /// This function is an async version of [`std::fs::symlink_metadata`].
    fn symlink_metadata(&self, waker: Waker, path: &Path) -> CancelablePoll<io::Result<Metadata>>;
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

/// Get the globally registered instance of [`FileSystem`].
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
