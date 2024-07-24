//! Future-based filesystem manipulation operations.
//!
//! This module contains basic methods to manipulate the contents of the local filesystem.
//! All methods in this module represent cross-platform filesystem operations.
//! Extra platform-specific functionality can be found in the extension traits of
//! rasi::fs::$platform.
//!
//! # Driver configuration
//! Before using these filesystem api, we need to use function [`register_fs_driver`] to
//! inject one filesystem driver. here is an example of how to inject the filesystem
//! [`driver`](syscall::Driver) with the [`rasi-mio`] library.
//!
//! ```no_run
//! # futures::executor::block_on(async {
//! use rasi::fs;
//!
//! /// Here, we use `rasi_mio` crate as the filesystem implementation,
//! /// injected into the context of the current application.
//!
//! /// rasi_mio::fs::register_mio_filesystem();
//!
//! let current_path = fs::canonicalize("./").await;
//!
//! # });
//! ```
//!
//! # Custom filesystem
//!
//! Check out the [`rasi-mio`] to learn how to implement a Filesystem driver  from the ground up!
//!
//! [`rasi-mio`]: https://github.com/HalaOS/RASI/tree/main/crates/mio

use bitmask_enum::bitmask;
use futures::{future::poll_fn, AsyncRead, AsyncSeek, AsyncWrite, Stream};
use std::{
    io::{Result, SeekFrom},
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    task::{Context, Poll},
};

pub use std::fs::{FileType, Metadata, Permissions};

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

#[cfg(windows)]
pub mod windows {

    use std::io::ErrorKind;

    use super::*;

    pub struct NamedPipeListener(Box<dyn syscall::windows::FSDNamedPipeListener>);

    impl Deref for NamedPipeListener {
        type Target = dyn syscall::windows::FSDNamedPipeListener;
        fn deref(&self) -> &Self::Target {
            &*self.0
        }
    }

    impl<F: syscall::windows::FSDNamedPipeListener + 'static> From<F> for NamedPipeListener {
        fn from(value: F) -> Self {
            Self(Box::new(value))
        }
    }

    impl NamedPipeListener {
        /// Returns internal `FSDNamedPipeListener` object.
        pub fn as_raw_ptr(&self) -> &dyn syscall::windows::FSDNamedPipeListener {
            &*self.0
        }

        pub async fn accept(&self) -> Result<NamedPipeStream> {
            poll_fn(|cx| self.as_raw_ptr().poll_next(cx)).await
        }
    }

    impl Stream for NamedPipeListener {
        type Item = Result<NamedPipeStream>;

        fn poll_next(
            self: std::pin::Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Self::Item>> {
            match self.as_raw_ptr().poll_next(cx) {
                Poll::Ready(Ok(stream)) => Poll::Ready(Some(Ok(stream))),
                Poll::Ready(Err(err)) => {
                    if err.kind() == ErrorKind::BrokenPipe {
                        Poll::Ready(None)
                    } else {
                        Poll::Ready(Some(Err(err)))
                    }
                }
                Poll::Pending => Poll::Pending,
            }
        }
    }

    pub struct NamedPipeStream(Arc<Box<dyn syscall::windows::FSDNamedPipeStream>>);

    impl Deref for NamedPipeStream {
        type Target = dyn syscall::windows::FSDNamedPipeStream;
        fn deref(&self) -> &Self::Target {
            &**self.0
        }
    }

    impl<F: syscall::windows::FSDNamedPipeStream + 'static> From<F> for NamedPipeStream {
        fn from(value: F) -> Self {
            Self(Arc::new(Box::new(value)))
        }
    }

    impl NamedPipeStream {
        /// Returns internal `FSDNamedPipeStream` object.
        pub fn as_raw_ptr(&self) -> &dyn syscall::windows::FSDNamedPipeStream {
            &**self.0
        }
    }

    impl AsyncRead for NamedPipeStream {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<Result<usize>> {
            self.as_raw_ptr().poll_read(cx, buf)
        }
    }

    impl AsyncWrite for NamedPipeStream {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize>> {
            self.as_raw_ptr().poll_write(cx, buf)
        }

        fn poll_flush(self: std::pin::Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
            self.as_raw_ptr().poll_close(cx)
        }
    }
}

/// A filesystem driver must implement the `Driver-*` traits in this module.
pub mod syscall {
    use super::*;

    #[cfg(windows)]
    pub mod windows {

        use super::*;
        pub trait FSDNamedPipeListener: Sync + Send {
            fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<()>>;

            fn poll_next(
                &self,
                cx: &mut Context<'_>,
            ) -> Poll<Result<crate::fs::windows::NamedPipeStream>>;
        }

        pub trait FSDNamedPipeStream: Sync + Send {
            fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<()>>;

            /// Write a buffer into this writer, returning how many bytes were written
            fn poll_write(&self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>>;

            /// Pull some bytes from this source into the specified buffer, returning how many bytes were read.
            fn poll_read(&self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>>;

            fn poll_close(&self, cx: &mut Context<'_>) -> Poll<Result<()>>;
        }
    }

    /// A driver is the main entry to access asynchronously filesystem functions
    pub trait Driver: Sync + Send {
        /// Open new file with provided `mode`.
        fn open_file(&self, path: &Path, mode: FileOpenMode) -> Result<File>;

        /// Returns the canonical form of a path.
        /// The returned path is in absolute form with all intermediate components
        /// normalized and symbolic links resolved.
        /// This function is an async version of [`std::fs::canonicalize`].
        fn canonicalize(&self, path: &Path) -> Result<PathBuf>;

        /// Copies the contents and permissions of a file to a new location.
        /// On success, the total number of bytes copied is returned and equals
        /// the length of the to file after this operation.
        /// The old contents of to will be overwritten. If from and to both point
        /// to the same file, then the file will likely get truncated as a result of this operation.
        fn poll_copy(&self, cx: &mut Context<'_>, from: &Path, to: &Path) -> Poll<Result<u64>>;

        /// Creates a new directory.
        /// Note that this function will only create the final directory in path.
        /// If you want to create all of its missing parent directories too, use
        /// the [`create_dir_all`](FileSystem::create_dir_all) function instead.
        ///
        /// This function is an async version of [`std::fs::create_dir`].
        fn poll_create_dir(&self, cx: &mut Context<'_>, path: &Path) -> Poll<Result<()>>;

        /// Creates a new directory and all of its parents if they are missing.
        /// This function is an async version of [`std::fs::create_dir_all`].
        fn poll_create_dir_all(&self, cx: &mut Context<'_>, path: &Path) -> Poll<Result<()>>;

        /// Creates a hard link on the filesystem.
        /// The dst path will be a link pointing to the src path. Note that operating
        /// systems often require these two paths to be located on the same filesystem.
        ///
        /// This function is an async version of [`std::fs::hard_link`].
        fn poll_hard_link(&self, cx: &mut Context<'_>, from: &Path, to: &Path) -> Poll<Result<()>>;

        /// Reads metadata for a path.
        /// This function will traverse symbolic links to read metadata for the target
        /// file or directory. If you want to read metadata without following symbolic
        /// links, use symlink_metadata instead.
        ///
        /// This function is an async version of [`std::fs::metadata`].
        fn poll_metadata(&self, cx: &mut Context<'_>, path: &Path) -> Poll<Result<Metadata>>;

        /// Reads a symbolic link and returns the path it points to.
        ///
        /// This function is an async version of [`std::fs::read_link`].
        fn poll_read_link(&self, cx: &mut Context<'_>, path: &Path) -> Poll<Result<PathBuf>>;

        /// Removes an empty directory,
        /// if the `path` is not an empty directory, use the function
        /// [`remove_dir_all`](FileSystem::remove_dir_all) instead.
        ///
        /// This function is an async version of std::fs::remove_dir.
        fn poll_remove_dir(&self, cx: &mut Context<'_>, path: &Path) -> Poll<Result<()>>;

        /// Removes a directory and all of its contents.
        ///
        /// This function is an async version of [`std::fs::remove_dir_all`].
        fn poll_remove_dir_all(&self, cx: &mut Context<'_>, path: &Path) -> Poll<Result<()>>;

        /// Removes a file.
        /// This function is an async version of [`std::fs::remove_file`].
        fn poll_remove_file(&self, cx: &mut Context<'_>, path: &Path) -> Poll<Result<()>>;

        /// Renames a file or directory to a new location.
        /// If a file or directory already exists at the target location, it will be overwritten by this operation.
        /// This function is an async version of std::fs::rename.
        fn poll_rename(&self, cx: &mut Context<'_>, from: &Path, to: &Path) -> Poll<Result<()>>;

        /// Changes the permissions of a file or directory.
        /// This function is an async version of [`std::fs::set_permissions`].
        fn poll_set_permissions(
            &self,
            cx: &mut Context<'_>,
            path: &Path,
            perm: &Permissions,
        ) -> Poll<Result<()>>;

        /// Reads metadata for a path without following symbolic links.
        /// If you want to follow symbolic links before reading metadata of the target file or directory,
        /// use [`metadata`](FileSystem::metadata) instead.
        ///
        /// This function is an async version of [`std::fs::symlink_metadata`].
        fn poll_symlink_metadata(
            &self,
            cx: &mut Context<'_>,
            path: &Path,
        ) -> Poll<Result<Metadata>>;

        /// Returns a iterator handle of entries in a directory.
        ///
        /// See [`dir_entry_next`](FileSystem::dir_entry_next) for more information about iteration.
        fn read_dir(&self, path: &Path) -> Result<ReadDir>;

        #[cfg(any(windows))]
        /// Opens the named pipe identified by `addr`.
        fn named_pipe_client_open(
            &self,
            addr: &std::ffi::OsStr,
        ) -> Result<crate::fs::windows::NamedPipeStream>;

        #[cfg(any(windows))]
        /// Creates the named pipe identified by `addr` for use as a server.
        ///
        /// This uses the [`CreateNamedPipe`] function.
        fn named_pipe_server_create(
            &self,
            addr: &std::ffi::OsStr,
        ) -> Result<crate::fs::windows::NamedPipeListener>;
    }

    pub trait DriverDirEntry: Sync + Send {
        /// Returns the bare name of this entry without the leading path.
        fn name(&self) -> String;

        /// Returns the full path to this entry.
        /// The full path is created by joining the original path passed to [`read_dir`](FileSystemDriver::read_dir) with the name of this entry.
        fn path(&self) -> PathBuf;

        /// Reads the metadata for this entry.
        ///
        /// This function will traverse symbolic links to read the metadata.
        fn meta(&self) -> Result<Metadata>;

        /// eads the file type for this entry.
        /// This function will not traverse symbolic links if this entry points at one.
        /// If you want to read metadata with following symbolic links, use [`meta`](FSDDirEntry::meta) instead.
        fn file_type(&self) -> Result<FileType>;
    }

    /// Driver-specific `File` object.
    pub trait DriverFile: Sync + Send {
        /// Poll if the file object is actually opened.
        fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<()>>;

        /// Write a buffer into this writer, returning how many bytes were written
        fn poll_write(&self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>>;

        /// Pull some bytes from this source into the specified buffer, returning how many bytes were read.
        fn poll_read(&self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>>;

        /// Attempts to sync all OS-internal metadata to disk.
        ///
        /// This function will attempt to ensure that all in-memory data reaches the filesystem before returning.
        ///
        /// This can be used to handle errors that would otherwise only be caught when the File is closed.
        /// Dropping a file will ignore errors in synchronizing this in-memory data.
        fn poll_flush(&self, cx: &mut Context<'_>) -> Poll<Result<()>>;

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
        fn poll_seek(&self, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<Result<u64>>;

        ///  Reads the file's metadata.
        fn poll_meta(&self, cx: &mut Context<'_>) -> Poll<Result<Metadata>>;

        /// Changes the permissions on the file.
        fn poll_set_permissions(
            &self,
            cx: &mut Context<'_>,
            perm: &Permissions,
        ) -> Poll<Result<()>>;

        /// Truncates or extends the file.
        ///
        /// If `size` is less than the current file size, then the file will be truncated. If it is
        /// greater than the current file size, then the file will be extended to `size` and have all
        /// intermediate data filled with zeros.
        ///
        /// The file's cursor stays at the same position, even if the cursor ends up being past the end
        /// of the file after this operation.
        ///
        fn poll_set_len(&self, cx: &mut Context<'_>, size: u64) -> Poll<Result<()>>;
    }

    pub trait DriverReadDir: Sync + Send {
        /// Get asynchronously opened event.
        fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<()>>;

        fn poll_next(&self, cx: &mut Context<'_>) -> Poll<Option<Result<DirEntry>>>;
    }
}

/// Entries returned by the [`ReadDir`] iterator.
pub struct DirEntry(Box<dyn syscall::DriverDirEntry>);

impl Deref for DirEntry {
    type Target = dyn syscall::DriverDirEntry;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<F: syscall::DriverDirEntry + 'static> From<F> for DirEntry {
    fn from(value: F) -> Self {
        Self(Box::new(value))
    }
}

impl DirEntry {
    /// Returns internal `FSDDirEntry` object.
    pub fn as_raw_ptr(&self) -> &dyn syscall::DriverDirEntry {
        &*self.0
    }
}

/// Iterator over the entries in a directory.
pub struct ReadDir(Box<dyn syscall::DriverReadDir>);

impl Deref for ReadDir {
    type Target = dyn syscall::DriverReadDir;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<F: syscall::DriverReadDir + 'static> From<F> for ReadDir {
    fn from(value: F) -> Self {
        Self(Box::new(value))
    }
}

impl ReadDir {
    /// Returns internal `FSDReadDir` object.
    pub fn as_raw_ptr(&self) -> &dyn syscall::DriverReadDir {
        &*self.0
    }

    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::new_with(path, get_fs_driver()).await
    }

    pub async fn new_with<P: AsRef<Path>>(path: P, driver: &dyn syscall::Driver) -> Result<Self> {
        let readdir = driver.read_dir(path.as_ref())?;

        poll_fn(|cx| readdir.as_raw_ptr().poll_ready(cx))
            .await
            .map(|_| readdir)
    }
}

impl Stream for ReadDir {
    type Item = Result<DirEntry>;
    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.as_raw_ptr().poll_next(cx)
    }
}

/// An open file on the filesystem.
///
/// Depending on what options the file was opened with, this type can be used for reading and/or writing.
/// Files are automatically closed when they get dropped and any errors detected on closing are ignored.
/// Use the sync_all method before dropping a file if such errors need to be handled.
///
/// This type is an async version of std::fs::File.
pub struct File(Arc<Box<dyn syscall::DriverFile>>);

impl Deref for File {
    type Target = dyn syscall::DriverFile;
    fn deref(&self) -> &Self::Target {
        &**self.0
    }
}

impl<F: syscall::DriverFile + 'static> From<F> for File {
    fn from(value: F) -> Self {
        Self(Arc::new(Box::new(value)))
    }
}

impl File {
    /// Returns internal `FSDFile` object.
    pub fn as_raw_ptr(&self) -> &dyn syscall::DriverFile {
        &**self.0
    }

    /// Open a file with provided `FileOpenMode`.
    pub async fn open<P: AsRef<Path>>(path: P, mode: FileOpenMode) -> Result<Self> {
        Self::open_with(path, mode, get_fs_driver()).await
    }

    /// Use custom `FileSystemDriver` to open file.
    pub async fn open_with<P: AsRef<Path>>(
        path: P,
        mode: FileOpenMode,
        driver: &dyn syscall::Driver,
    ) -> Result<Self> {
        let file = driver.open_file(path.as_ref(), mode)?;

        // Wait for file opened event.
        poll_fn(|cx| file.as_raw_ptr().poll_ready(cx)).await?;

        Ok(file)
    }

    pub async fn meta(&self) -> Result<Metadata> {
        poll_fn(|cx| self.as_raw_ptr().poll_meta(cx)).await
    }

    pub async fn set_permissions(&self, perm: &Permissions) -> Result<()> {
        poll_fn(|cx| self.as_raw_ptr().poll_set_permissions(cx, perm)).await
    }

    pub async fn set_len(&self, len: u64) -> Result<()> {
        poll_fn(|cx| self.as_raw_ptr().poll_set_len(cx, len)).await
    }
}

impl AsyncRead for File {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        self.as_raw_ptr().poll_read(cx, buf)
    }
}

impl AsyncWrite for File {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        self.as_raw_ptr().poll_write(cx, buf)
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.as_raw_ptr().poll_flush(cx)
    }

    fn poll_close(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.poll_flush(cx)
    }
}

impl AsyncSeek for File {
    fn poll_seek(
        self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<Result<u64>> {
        self.as_raw_ptr().poll_seek(cx, pos)
    }
}

/// Returns the canonical form of a path.
/// The returned path is in absolute form with all intermediate components
/// normalized and symbolic links resolved.
/// This function is an async version of [`std::fs::canonicalize`].
pub async fn canonicalize<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    get_fs_driver().canonicalize(path.as_ref())
}

/// Copies the contents and permissions of a file to a new location.
/// On success, the total number of bytes copied is returned and equals
/// the length of the to file after this operation.
/// The old contents of to will be overwritten. If from and to both point
/// to the same file, then the file will likely get truncated as a result of this operation.
pub async fn copy<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<u64> {
    poll_fn(|cx| get_fs_driver().poll_copy(cx, from.as_ref(), to.as_ref())).await
}

/// Creates a new directory.
/// Note that this function will only create the final directory in path.
/// If you want to create all of its missing parent directories too, use
/// the [`create_dir_all`](FileSystem::create_dir_all) function instead.
///
/// This function is an async version of [`std::fs::create_dir`].
pub async fn create_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    poll_fn(|cx| get_fs_driver().poll_create_dir(cx, path.as_ref())).await
}

/// Creates a new directory and all of its parents if they are missing.
/// This function is an async version of [`std::fs::create_dir_all`].
pub async fn create_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    poll_fn(|cx| get_fs_driver().poll_create_dir_all(cx, path.as_ref())).await
}

/// Creates a hard link on the filesystem.
/// The dst path will be a link pointing to the src path. Note that operating
/// systems often require these two paths to be located on the same filesystem.
///
/// This function is an async version of [`std::fs::hard_link`].
pub async fn hard_link<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<()> {
    poll_fn(|cx| get_fs_driver().poll_hard_link(cx, from.as_ref(), to.as_ref())).await
}

/// Reads metadata for a path.
/// This function will traverse symbolic links to read metadata for the target
/// file or directory. If you want to read metadata without following symbolic
/// links, use symlink_metadata instead.
///
/// This function is an async version of [`std::fs::metadata`].
pub async fn metadata<P: AsRef<Path>>(path: P) -> Result<Metadata> {
    poll_fn(|cx| get_fs_driver().poll_metadata(cx, path.as_ref())).await
}

/// Reads a symbolic link and returns the path it points to.
///
/// This function is an async version of [`std::fs::read_link`].
pub async fn read_link<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    poll_fn(|cx| get_fs_driver().poll_read_link(cx, path.as_ref())).await
}

/// Removes an empty directory,
/// if the `path` is not an empty directory, use the function
/// [`remove_dir_all`](FileSystem::remove_dir_all) instead.
///
/// This function is an async version of std::fs::remove_dir.
pub async fn remove_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    poll_fn(|cx| get_fs_driver().poll_remove_dir(cx, path.as_ref())).await
}

/// Removes a directory and all of its contents.
///
/// This function is an async version of [`std::fs::remove_dir_all`].
pub async fn remove_dir_all<P: AsRef<Path>>(path: P) -> Result<()> {
    poll_fn(|cx| get_fs_driver().poll_remove_dir_all(cx, path.as_ref())).await
}

/// Removes a file.
/// This function is an async version of [`std::fs::remove_file`].
pub async fn remove_file<P: AsRef<Path>>(path: P) -> Result<()> {
    poll_fn(|cx| get_fs_driver().poll_remove_file(cx, path.as_ref())).await
}

/// Renames a file or directory to a new location.
/// If a file or directory already exists at the target location, it will be overwritten by this operation.
/// This function is an async version of std::fs::rename.
pub async fn rename<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<()> {
    poll_fn(|cx| get_fs_driver().poll_rename(cx, from.as_ref(), to.as_ref())).await
}

/// Changes the permissions of a file or directory.
/// This function is an async version of [`std::fs::set_permissions`].
pub async fn set_permissions<P: AsRef<Path>>(path: P, perm: &Permissions) -> Result<()> {
    poll_fn(|cx| get_fs_driver().poll_set_permissions(cx, path.as_ref(), perm)).await
}

/// Reads metadata for a path without following symbolic links.
/// If you want to follow symbolic links before reading metadata of the target file or directory,
/// use [`metadata`](FileSystem::metadata) instead.
///
/// This function is an async version of [`std::fs::symlink_metadata`].
pub async fn symlink_metadata<P: AsRef<Path>>(path: P) -> Result<Metadata> {
    poll_fn(|cx| get_fs_driver().poll_symlink_metadata(cx, path.as_ref())).await
}

/// Returns `true` if the path exists on disk and is pointing at a directory.
///
/// This function will traverse symbolic links to query information about the
/// destination file. In case of broken symbolic links this will return `false`.
///
/// If you cannot access the directory containing the file, e.g., because of a
/// permission error, this will return `false`.
///
/// # See Also
///
/// This is a convenience function that coerces errors to false. If you want to
/// check errors, call [fs::metadata] and handle its Result. Then call
/// [fs::Metadata::is_dir] if it was Ok.
///
/// [fs::metadata]: ../fs/fn.metadata.html
/// [fs::Metadata::is_dir]: ../fs/struct.Metadata.html#method.is_dir
pub async fn is_dir<P: AsRef<Path>>(path: P) -> bool {
    metadata(path).await.map(|m| m.is_dir()).unwrap_or(false)
}

pub struct FileSystem<'a> {
    driver: &'a dyn syscall::Driver,
}

impl<'a> From<&'a dyn syscall::Driver> for FileSystem<'a> {
    fn from(value: &'a dyn syscall::Driver) -> Self {
        Self { driver: value }
    }
}

impl<'a> FileSystem<'a> {
    /// Returns `true` if the path exists on disk and is pointing at a directory.
    ///
    /// This function will traverse symbolic links to query information about the
    /// destination file. In case of broken symbolic links this will return `false`.
    ///
    /// If you cannot access the directory containing the file, e.g., because of a
    /// permission error, this will return `false`.
    ///
    ///
    /// # See Also
    ///
    /// This is a convenience function that coerces errors to false. If you want to
    /// check errors, call [fs::metadata] and handle its Result. Then call
    /// [fs::Metadata::is_dir] if it was Ok.
    ///
    /// [fs::metadata]: ../fs/fn.metadata.html
    /// [fs::Metadata::is_dir]: ../fs/struct.Metadata.html#method.is_dir
    pub async fn is_dir<P: AsRef<Path>>(&self, path: P) -> bool {
        self.metadata(path)
            .await
            .map(|m| m.is_dir())
            .unwrap_or(false)
    }

    /// Returns `true` if the path exists on disk and is pointing at a regular file.
    ///
    /// This function will traverse symbolic links to query information about the
    /// destination file. In case of broken symbolic links this will return `false`.
    ///
    /// If you cannot access the directory containing the file, e.g., because of a
    /// permission error, this will return `false`.
    ///
    /// # See Also
    ///
    /// This is a convenience function that coerces errors to false. If you want to
    /// check errors, call [fs::metadata] and handle its Result. Then call
    /// [fs::Metadata::is_file] if it was Ok.
    ///
    /// [fs::metadata]: ../fs/fn.metadata.html
    /// [fs::Metadata::is_file]: ../fs/struct.Metadata.html#method.is_file

    pub async fn is_file<P: AsRef<Path>>(&self, path: P) -> bool {
        self.metadata(path)
            .await
            .map(|m| m.is_file())
            .unwrap_or(false)
    }

    pub async fn open_file<P: AsRef<Path>>(&self, path: P, mode: FileOpenMode) -> Result<File> {
        File::open_with(path, mode, self.driver).await
    }

    /// Returns the canonical form of a path.
    /// The returned path is in absolute form with all intermediate components
    /// normalized and symbolic links resolved.
    /// This function is an async version of [`std::fs::canonicalize`].
    pub async fn canonicalize<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        self.driver.canonicalize(path.as_ref())
    }

    /// Copies the contents and permissions of a file to a new location.
    /// On success, the total number of bytes copied is returned and equals
    /// the length of the to file after this operation.
    /// The old contents of to will be overwritten. If from and to both point
    /// to the same file, then the file will likely get truncated as a result of this operation.
    pub async fn copy<F: AsRef<Path>, T: AsRef<Path>>(&self, from: F, to: T) -> Result<u64> {
        poll_fn(|cx| self.driver.poll_copy(cx, from.as_ref(), to.as_ref())).await
    }

    /// Creates a new directory.
    /// Note that this function will only create the final directory in path.
    /// If you want to create all of its missing parent directories too, use
    /// the [`create_dir_all`](FileSystem::create_dir_all) function instead.
    ///
    /// This function is an async version of [`std::fs::create_dir`].
    pub async fn create_dir<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        poll_fn(|cx| self.driver.poll_create_dir(cx, path.as_ref())).await
    }

    /// Creates a new directory and all of its parents if they are missing.
    /// This function is an async version of [`std::fs::create_dir_all`].
    pub async fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        poll_fn(|cx| self.driver.poll_create_dir_all(cx, path.as_ref())).await
    }

    /// Creates a hard link on the filesystem.
    /// The dst path will be a link pointing to the src path. Note that operating
    /// systems often require these two paths to be located on the same filesystem.
    ///
    /// This function is an async version of [`std::fs::hard_link`].
    pub async fn hard_link<F: AsRef<Path>, T: AsRef<Path>>(&self, from: F, to: T) -> Result<()> {
        poll_fn(|cx| self.driver.poll_hard_link(cx, from.as_ref(), to.as_ref())).await
    }

    /// Reads metadata for a path.
    /// This function will traverse symbolic links to read metadata for the target
    /// file or directory. If you want to read metadata without following symbolic
    /// links, use symlink_metadata instead.
    ///
    /// This function is an async version of [`std::fs::metadata`].
    pub async fn metadata<P: AsRef<Path>>(&self, path: P) -> Result<Metadata> {
        poll_fn(|cx| self.driver.poll_metadata(cx, path.as_ref())).await
    }

    /// Reads a symbolic link and returns the path it points to.
    ///
    /// This function is an async version of [`std::fs::read_link`].
    pub async fn read_link<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf> {
        poll_fn(|cx| self.driver.poll_read_link(cx, path.as_ref())).await
    }

    /// Removes an empty directory,
    /// if the `path` is not an empty directory, use the function
    /// [`remove_dir_all`](FileSystem::remove_dir_all) instead.
    ///
    /// This function is an async version of std::fs::remove_dir.
    pub async fn remove_dir<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        poll_fn(|cx| self.driver.poll_remove_dir(cx, path.as_ref())).await
    }

    /// Removes a directory and all of its contents.
    ///
    /// This function is an async version of [`std::fs::remove_dir_all`].
    pub async fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        poll_fn(|cx| self.driver.poll_remove_dir_all(cx, path.as_ref())).await
    }

    /// Removes a file.
    /// This function is an async version of [`std::fs::remove_file`].
    pub async fn remove_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        poll_fn(|cx| self.driver.poll_remove_file(cx, path.as_ref())).await
    }

    /// Renames a file or directory to a new location.
    /// If a file or directory already exists at the target location, it will be overwritten by this operation.
    /// This function is an async version of std::fs::rename.
    pub async fn rename<F: AsRef<Path>, T: AsRef<Path>>(&self, from: F, to: T) -> Result<()> {
        poll_fn(|cx| self.driver.poll_rename(cx, from.as_ref(), to.as_ref())).await
    }

    /// Changes the permissions of a file or directory.
    /// This function is an async version of [`std::fs::set_permissions`].
    pub async fn set_permissions<P: AsRef<Path>>(&self, path: P, perm: &Permissions) -> Result<()> {
        poll_fn(|cx| self.driver.poll_set_permissions(cx, path.as_ref(), perm)).await
    }

    /// Reads metadata for a path without following symbolic links.
    /// If you want to follow symbolic links before reading metadata of the target file or directory,
    /// use [`metadata`](FileSystem::metadata) instead.
    ///
    /// This function is an async version of [`std::fs::symlink_metadata`].
    pub async fn symlink_metadata<P: AsRef<Path>>(&self, path: P) -> Result<Metadata> {
        poll_fn(|cx| self.driver.poll_symlink_metadata(cx, path.as_ref())).await
    }
}

static FIFLE_SYSTEM_DRIVER: OnceLock<Box<dyn syscall::Driver>> = OnceLock::new();

/// Get the filesystem driver(Implementation) from the global context of this application.
///
/// # Panic
///
/// Before calling this function, [`get_fs_driver`] should be called to register an implementation.
/// otherwise panics the current thread with message `Call register_network_driver first.`.
pub fn get_fs_driver() -> &'static dyn syscall::Driver {
    FIFLE_SYSTEM_DRIVER
        .get()
        .expect("Call register_network_driver first.")
        .as_ref()
}

/// Inject a filesystem driver(Implementation) into the global context of this application.
///
/// # Examples
/// ```no_run
///
/// fn main() {
///   // The `register_mio_filesystem` function indirectly calls the `register_fs_driver` function.
///   // rasi_mio::fs::register_mio_filesystem();
/// }
///
/// ```
///
/// # Panic
///
/// Multiple calls to this function are not permitted!!!
pub fn register_fs_driver<E: syscall::Driver + 'static>(driver: E) {
    if FIFLE_SYSTEM_DRIVER.set(Box::new(driver)).is_err() {
        panic!("Multiple calls to register_network_driver are not permitted!!!");
    }
}
