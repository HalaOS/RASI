//! Filesystem manipulation operations.
//!
//! This module is an async version of std::fs.

use std::{
    fs::{FileType, Metadata, Permissions},
    io,
    path::{Path, PathBuf},
    task::Poll,
};

use futures::{AsyncRead, AsyncSeek, AsyncWrite, Stream};
use rasi_syscall::{global_filesystem, FileOpenMode, Handle};

use crate::utils::cancelable_would_block;

/// A wrap type for filesystem [`syscall`](rasi_syscall::FileSystem).
pub struct FileSystem {
    syscall: &'static dyn rasi_syscall::FileSystem,
}

impl FileSystem {
    /// Create new `FileSystem` with [`global_filesystem`].
    pub fn new() -> Self {
        FileSystem {
            syscall: global_filesystem(),
        }
    }
    /// Create new `FileSystem` with custom [`syscall`](rasi_syscall::FileSystem).
    pub fn new_with(syscall: &'static dyn rasi_syscall::FileSystem) -> Self {
        FileSystem { syscall }
    }

    /// Returns the canonical form of a path.
    /// The returned path is in absolute form with all intermediate components normalized and symbolic links resolved.
    /// This function is an async version of std::fs::canonicalize.
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` does not point to an existing file or directory.
    /// * A non-final component in `path` is not a directory.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// let path = fs::canonicalize(".").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn canonicalize<P: AsRef<Path>>(&self, path: P) -> io::Result<PathBuf> {
        cancelable_would_block(|cx| self.syscall.canonicalize(cx.waker().clone(), path.as_ref()))
            .await
    }

    /// Copies the contents and permissions of a file to a new location.
    ///
    /// On success, the total number of bytes copied is returned and equals the length of the `to` file
    /// after this operation.
    ///
    /// The old contents of `to` will be overwritten. If `from` and `to` both point to the same file,
    /// then the file will likely get truncated as a result of this operation.
    ///
    /// If you're working with open [`File`]s and want to copy contents through those types, use the
    /// [`io::copy`] function.
    ///
    /// This function is an async version of [`std::fs::copy`].
    ///
    /// [`File`]: struct.File.html
    /// [`io::copy`]: ../io/fn.copy.html
    /// [`std::fs::copy`]: https://doc.rust-lang.org/std/fs/fn.copy.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `from` does not point to an existing file.
    /// * The current process lacks permissions to read `from` or write `to`.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// let num_bytes = fs::copy("a.txt", "b.txt").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn copy<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> io::Result<u64> {
        cancelable_would_block(|cx| {
            self.syscall
                .copy(cx.waker().clone(), from.as_ref(), to.as_ref())
        })
        .await
    }

    /// Creates a new directory.
    ///
    /// Note that this function will only create the final directory in `path`. If you want to create
    /// all of its missing parent directories too, use the [`create_dir_all`] function instead.
    ///
    /// This function is an async version of [`std::fs::create_dir`].
    ///
    /// [`create_dir_all`]: fn.create_dir_all.html
    /// [`std::fs::create_dir`]: https://doc.rust-lang.org/std/fs/fn.create_dir.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` already points to an existing file or directory.
    /// * A parent directory in `path` does not exist.
    /// * The current process lacks permissions to create the directory.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// fs::create_dir("./some/directory").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn create_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        cancelable_would_block(|cx| self.syscall.create_dir(cx.waker().clone(), path.as_ref()))
            .await
    }

    /// Creates a new directory and all of its parents if they are missing.
    ///
    /// This function is an async version of [`std::fs::create_dir_all`].
    ///
    /// [`std::fs::create_dir_all`]: https://doc.rust-lang.org/std/fs/fn.create_dir_all.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` already points to an existing file or directory.
    /// * The current process lacks permissions to create the directory or its missing parents.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// fs::create_dir_all("./some/directory").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        cancelable_would_block(|cx| {
            self.syscall
                .create_dir_all(cx.waker().clone(), path.as_ref())
        })
        .await
    }

    /// Creates a hard link on the filesystem.
    ///
    /// The `dst` path will be a link pointing to the `src` path. Note that operating systems often
    /// require these two paths to be located on the same filesystem.
    ///
    /// This function is an async version of [`std::fs::hard_link`].
    ///
    /// [`std::fs::hard_link`]: https://doc.rust-lang.org/std/fs/fn.hard_link.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `src` does not point to an existing file.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// fs::hard_link("a.txt", "b.txt").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn hard_link<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        from: P,
        to: Q,
    ) -> io::Result<()> {
        cancelable_would_block(|cx| {
            self.syscall
                .hard_link(cx.waker().clone(), from.as_ref(), to.as_ref())
        })
        .await
    }

    /// Reads metadata for a path.
    ///
    /// This function will traverse symbolic links to read metadata for the target file or directory.
    /// If you want to read metadata without following symbolic links, use [`symlink_metadata`]
    /// instead.
    ///
    /// This function is an async version of [`std::fs::metadata`].
    ///
    /// [`symlink_metadata`]: fn.symlink_metadata.html
    /// [`std::fs::metadata`]: https://doc.rust-lang.org/std/fs/fn.metadata.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` does not point to an existing file or directory.
    /// * The current process lacks permissions to read metadata for the path.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// let perm = fs::metadata("a.txt").await?.permissions();
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn metadata<P: AsRef<Path>>(&self, path: P) -> io::Result<Metadata> {
        cancelable_would_block(|cx| self.syscall.metadata(cx.waker().clone(), path.as_ref())).await
    }

    /// Reads a symbolic link and returns the path it points to.
    ///
    /// This function is an async version of [`std::fs::read_link`].
    ///
    /// [`std::fs::read_link`]: https://doc.rust-lang.org/std/fs/fn.read_link.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` does not point to an existing link.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// let path = fs::read_link("a.txt").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn read_link<P: AsRef<Path>>(&self, path: P) -> io::Result<PathBuf> {
        cancelable_would_block(|cx| self.syscall.read_link(cx.waker().clone(), path.as_ref())).await
    }

    /// Removes an empty directory.
    ///
    /// This function is an async version of [`std::fs::remove_dir`].
    ///
    /// [`std::fs::remove_dir`]: https://doc.rust-lang.org/std/fs/fn.remove_dir.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` is not an existing and empty directory.
    /// * The current process lacks permissions to remove the directory.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// fs::remove_dir("./some/directory").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn remove_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        cancelable_would_block(|cx| self.syscall.remove_dir(cx.waker().clone(), path.as_ref()))
            .await
    }

    /// Removes a directory and all of its contents.
    ///
    /// This function is an async version of [`std::fs::remove_dir_all`].
    ///
    /// [`std::fs::remove_dir_all`]: https://doc.rust-lang.org/std/fs/fn.remove_dir_all.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` is not an existing and empty directory.
    /// * The current process lacks permissions to remove the directory.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// fs::remove_dir_all("./some/directory").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        cancelable_would_block(|cx| {
            self.syscall
                .remove_dir_all(cx.waker().clone(), path.as_ref())
        })
        .await
    }

    /// Removes a file.
    ///
    /// This function is an async version of [`std::fs::remove_file`].
    ///
    /// [`std::fs::remove_file`]: https://doc.rust-lang.org/std/fs/fn.remove_file.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` does not point to an existing file.
    /// * The current process lacks permissions to remove the file.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// fs::remove_file("a.txt").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        cancelable_would_block(|cx| self.syscall.remove_file(cx.waker().clone(), path.as_ref()))
            .await
    }

    /// Renames a file or directory to a new location.
    ///
    /// If a file or directory already exists at the target location, it will be overwritten by this
    /// operation.
    ///
    /// This function is an async version of [`std::fs::rename`].
    ///
    /// [`std::fs::rename`]: https://doc.rust-lang.org/std/fs/fn.rename.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `from` does not point to an existing file or directory.
    /// * `from` and `to` are on different filesystems.
    /// * The current process lacks permissions to do the rename operation.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// fs::rename("a.txt", "b.txt").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn rename<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> io::Result<()> {
        cancelable_would_block(|cx| {
            self.syscall
                .rename(cx.waker().clone(), from.as_ref(), to.as_ref())
        })
        .await
    }

    /// Changes the permissions of a file or directory.
    ///
    /// This function is an async version of [`std::fs::set_permissions`].
    ///
    /// [`std::fs::set_permissions`]: https://doc.rust-lang.org/std/fs/fn.set_permissions.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` does not point to an existing file or directory.
    /// * The current process lacks permissions to change attributes on the file or directory.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// let mut perm = fs::metadata("a.txt").await?.permissions();
    /// perm.set_readonly(true);
    /// fs::set_permissions("a.txt", perm).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn set_permissions<P: AsRef<Path>>(
        &self,
        path: P,
        perm: Permissions,
    ) -> io::Result<()> {
        cancelable_would_block(|cx| {
            self.syscall
                .set_permissions(cx.waker().clone(), path.as_ref(), &perm)
        })
        .await
    }

    /// Reads metadata for a path without following symbolic links.
    ///
    /// If you want to follow symbolic links before reading metadata of the target file or directory,
    /// use [`metadata`] instead.
    ///
    /// This function is an async version of [`std::fs::symlink_metadata`].
    ///
    /// [`metadata`]: fn.metadata.html
    /// [`std::fs::symlink_metadata`]: https://doc.rust-lang.org/std/fs/fn.symlink_metadata.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` does not point to an existing file or directory.
    /// * The current process lacks permissions to read metadata for the path.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    ///
    /// let perm = fs::symlink_metadata("a.txt").await?.permissions();
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn symlink_metadata<P: AsRef<Path>>(&self, path: P) -> io::Result<Metadata> {
        cancelable_would_block(|cx| {
            self.syscall
                .symlink_metadata(cx.waker().clone(), path.as_ref())
        })
        .await
    }

    /// Returns a stream of entries in a directory.
    ///
    /// The stream yields items of type [`io::Result`]`<`[`DirEntry`]`>`. Note that I/O errors can
    /// occur while reading from the stream.
    ///
    /// This function is an async version of [`std::fs::read_dir`].
    ///
    /// [`io::Result`]: ../io/type.Result.html
    /// [`DirEntry`]: struct.DirEntry.html
    /// [`std::fs::read_dir`]: https://doc.rust-lang.org/std/fs/fn.read_dir.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * `path` does not point to an existing directory.
    /// * The current process lacks permissions to read the contents of the directory.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    /// use rasi::prelude::*;
    ///
    /// let mut entries = fs::read_dir(".").await?;
    ///
    /// while let Some(res) = entries.next().await {
    ///     let entry = res?;
    ///     println!("{}", entry.file_name());
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    #[inline]
    pub async fn read_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<ReadDir> {
        let handle =
            cancelable_would_block(|cx| self.syscall.read_dir(cx.waker().clone(), path.as_ref()))
                .await?;

        Ok(ReadDir {
            handle,
            syscall: self.syscall,
            cancel_handle: None,
        })
    }

    /// Opens a file with provided [`mode`](FileOpenMode)
    pub async fn open_file<P: AsRef<Path>>(
        &self,
        path: P,
        open_mode: FileOpenMode,
    ) -> io::Result<File> {
        let handle = cancelable_would_block(|cx| {
            self.syscall
                .open_file(cx.waker().clone(), path.as_ref(), &open_mode)
        })
        .await?;

        Ok(File::new(handle, self.syscall))
    }

    /// Returns true if the path points at an existing entity.
    /// This function will traverse symbolic links to query information about the destination file. In case of broken symbolic links this will return false.
    ///
    /// If you cannot access the directory containing the file, e.g., because of a permission error, this will return false.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use std::path::Path;
    /// assert_eq!(rasi::fs::exists(Path::new("does_not_exist.txt")).await, false);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.metadata(path).await.is_ok()
    }
}

/// An open file on the filesystem.
///
/// Depending on what options the file was opened with, this type can be used for reading and/or writing.
/// Files are automatically closed when they get dropped and any errors detected on closing are ignored.
/// Use the sync_all method before dropping a file if such errors need to be handled.
///
/// This type is an async version of std::fs::File.
pub struct File {
    /// file handle.
    handle: Handle,
    /// the syscall interface for filesystem.
    syscall: &'static dyn rasi_syscall::FileSystem,
    /// The cancel handle reference to latest pending write ops.
    write_cancel_handle: Option<Handle>,
    /// The cancel handle reference to latest pending read ops.
    read_cancel_handle: Option<Handle>,
}

impl File {
    fn new(handle: Handle, syscall: &'static dyn rasi_syscall::FileSystem) -> Self {
        Self {
            handle,
            syscall,
            write_cancel_handle: None,
            read_cancel_handle: None,
        }
    }

    /// Reads the file's metadata.
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    /// use rasi::prelude::*;
    ///
    /// let file = fs::open_file("a.txt", FileOpenMode::Create).await?;
    /// let metadata = file.metadata().await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn metadata(&self) -> io::Result<Metadata> {
        cancelable_would_block(|cx| self.syscall.file_meta(cx.waker().clone(), &self.handle)).await
    }

    // Changes the permissions on the file.
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * The current process lacks permissions to change attributes on the file.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    /// use rasi::prelude::*;
    ///
    /// let file = fs::open_file("a.txt", FileOpenMode::Create).await?;
    ///
    /// let mut perms = file.metadata().await?.permissions();
    /// perms.set_readonly(true);
    /// file.set_permissions(perms).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn set_permissions(&self, perm: Permissions) -> io::Result<()> {
        cancelable_would_block(|cx| {
            self.syscall
                .file_set_permissions(cx.waker().clone(), &self.handle, &perm)
        })
        .await
    }

    /// Truncates or extends the file.
    ///
    /// If `size` is less than the current file size, then the file will be truncated. If it is
    /// greater than the current file size, then the file will be extended to `size` and have all
    /// intermediate data filled with zeros.
    ///
    /// The file's cursor stays at the same position, even if the cursor ends up being past the end
    /// of the file after this operation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    /// use rasi::prelude::*;
    ///
    /// let file = fs::open_file("a.txt",FileOpenMode::Create).await?;
    /// file.set_len(10).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn set_len(&self, size: u64) -> io::Result<()> {
        cancelable_would_block(|cx| {
            self.syscall
                .file_set_len(cx.waker().clone(), &self.handle, size)
        })
        .await
    }
}

impl AsyncWrite for File {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self
            .syscall
            .file_write(cx.waker().clone(), &self.handle, buf)
        {
            rasi_syscall::CancelablePoll::Ready(r) => Poll::Ready(r),
            rasi_syscall::CancelablePoll::Pending(write_cancel_handle) => {
                self.write_cancel_handle = Some(write_cancel_handle);
                Poll::Pending
            }
        }
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<()>> {
        match self.syscall.file_flush(cx.waker().clone(), &self.handle) {
            rasi_syscall::CancelablePoll::Ready(r) => Poll::Ready(r),
            rasi_syscall::CancelablePoll::Pending(write_cancel_handle) => {
                self.write_cancel_handle = Some(write_cancel_handle);
                Poll::Pending
            }
        }
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

impl AsyncRead for File {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        match self
            .syscall
            .file_read(cx.waker().clone(), &self.handle, buf)
        {
            rasi_syscall::CancelablePoll::Ready(r) => Poll::Ready(r),
            rasi_syscall::CancelablePoll::Pending(write_cancel_handle) => {
                self.write_cancel_handle = Some(write_cancel_handle);
                Poll::Pending
            }
        }
    }
}

impl AsyncSeek for File {
    fn poll_seek(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        pos: io::SeekFrom,
    ) -> Poll<io::Result<u64>> {
        match self
            .syscall
            .file_seek(cx.waker().clone(), &self.handle, pos)
        {
            rasi_syscall::CancelablePoll::Ready(r) => Poll::Ready(r),
            rasi_syscall::CancelablePoll::Pending(read_cancel_handle) => {
                self.read_cancel_handle = Some(read_cancel_handle);
                Poll::Pending
            }
        }
    }
}

/// A stream of entries in a directory.
///
/// This stream is returned by [`read_dir`](FileSystem::read_dir) and yields items of type
/// [`io::Result`]`<`[`DirEntry`]`>`. Each [`DirEntry`] can then retrieve information like entry's
/// path or metadata.
///
/// This type is an async version of [`std::fs::ReadDir`].
///
/// [`DirEntry`]: struct.DirEntry.html
/// [`std::fs::ReadDir`]: https://doc.rust-lang.org/std/fs/struct.ReadDir.html
pub struct ReadDir {
    handle: Handle,
    syscall: &'static dyn rasi_syscall::FileSystem,
    cancel_handle: Option<Handle>,
}

impl Stream for ReadDir {
    type Item = io::Result<DirEntry>;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self
            .syscall
            .dir_entry_next(cx.waker().clone(), &self.handle)
        {
            rasi_syscall::CancelablePoll::Ready(Ok(r)) => Poll::Ready(r.map(|h| {
                Ok(DirEntry {
                    handle: h,
                    syscall: self.syscall,
                })
            })),
            rasi_syscall::CancelablePoll::Ready(Err(err)) => Poll::Ready(Some(Err(err))),
            rasi_syscall::CancelablePoll::Pending(cancel_handle) => {
                self.cancel_handle = Some(cancel_handle);
                Poll::Pending
            }
        }
    }
}

/// An entry in a directory.
///
/// A stream of entries in a directory is returned by [`read_dir`].
///
/// This type is an async version of [`std::fs::DirEntry`].
pub struct DirEntry {
    handle: Handle,
    syscall: &'static dyn rasi_syscall::FileSystem,
}

impl DirEntry {
    /// Returns the full path to this entry.
    ///
    /// The full path is created by joining the original path passed to [`read_dir`] with the name
    /// of this entry.
    ///
    /// [`read_dir`]: fn.read_dir.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    /// use rasi::prelude::*;
    ///
    /// let mut dir = fs::read_dir(".").await?;
    ///
    /// while let Some(res) = dir.next().await {
    ///     let entry = res?;
    ///     println!("{:?}", entry.path());
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn path(&self) -> PathBuf {
        self.syscall.dir_entry_path(&self.handle)
    }

    /// Reads the metadata for this entry.
    ///
    /// This function will traverse symbolic links to read the metadata.
    ///
    /// If you want to read metadata without following symbolic links, use [`symlink_metadata`]
    /// instead.
    ///
    /// [`symlink_metadata`]: fn.symlink_metadata.html
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * This entry does not point to an existing file or directory anymore.
    /// * The current process lacks permissions to read the metadata.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    /// use rasi::prelude::*;
    ///
    /// let mut dir = fs::read_dir(".").await?;
    ///
    /// while let Some(res) = dir.next().await {
    ///     let entry = res?;
    ///     println!("{:?}", entry.metadata().await?);
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn metadata(&self) -> io::Result<Metadata> {
        cancelable_would_block(|cx| {
            self.syscall
                .dir_entry_metadata(cx.waker().clone(), &self.handle)
        })
        .await
    }

    /// Reads the file type for this entry.
    ///
    /// This function will not traverse symbolic links if this entry points at one.
    ///
    /// If you want to read metadata with following symbolic links, use [`metadata`] instead.
    ///
    /// [`metadata`]: #method.metadata
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * This entry does not point to an existing file or directory anymore.
    /// * The current process lacks permissions to read this entry's metadata.
    /// * Some other I/O error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    /// use rasi::prelude::*;
    ///
    /// let mut dir = fs::read_dir(".").await?;
    ///
    /// while let Some(res) = dir.next().await {
    ///     let entry = res?;
    ///     println!("{:?}", entry.file_type().await?);
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn file_type(&self) -> io::Result<FileType> {
        cancelable_would_block(|cx| {
            self.syscall
                .dir_entry_file_type(cx.waker().clone(), &self.handle)
        })
        .await
    }

    /// Returns the bare name of this entry without the leading path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    /// use rasi::prelude::*;
    ///
    /// let mut dir = fs::read_dir(".").await?;
    ///
    /// while let Some(res) = dir.next().await {
    ///     let entry = res?;
    ///     println!("{}", entry.file_name());
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn file_name(&self) -> String {
        self.syscall.dir_entry_file_name(&self.handle)
    }
}

/// Invoke `canonicalize` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::canonicalize`] for more details.
#[inline]
pub async fn canonicalize<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    FileSystem::new().canonicalize(path).await
}

/// Invoke `copy` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::copy`] for more details.
#[inline]
pub async fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<u64> {
    FileSystem::new().copy(from, to).await
}

/// Invoke `create_dir` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::create_dir`] for more details.
#[inline]
pub async fn create_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    FileSystem::new().create_dir(path).await
}

/// Invoke `create_dir_all` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::create_dir_all`] for more details.
#[inline]
pub async fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    FileSystem::new().create_dir_all(path).await
}

/// Invoke `hard_link` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::hard_link`] for more details.
#[inline]
pub async fn hard_link<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    FileSystem::new().hard_link(from, to).await
}

/// Invoke `metadata` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::metadata`] for more details.
#[inline]
pub async fn metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    FileSystem::new().metadata(path).await
}

/// Invoke `read_link` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::read_link`] for more details.
#[inline]
pub async fn read_link<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    FileSystem::new().read_link(path).await
}

/// Invoke `remove_dir` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::remove_dir`] for more details.
#[inline]
pub async fn remove_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    FileSystem::new().remove_dir(path).await
}

/// Invoke `remove_dir_all` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::remove_dir_all`] for more details.
#[inline]
pub async fn remove_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    FileSystem::new().remove_dir_all(path).await
}

/// Invoke `remove_file` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::remove_file`] for more details.
#[inline]
pub async fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    FileSystem::new().remove_file(path).await
}

/// Invoke `rename` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::rename`] for more details.
#[inline]
pub async fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    FileSystem::new().rename(from, to).await
}

/// Invoke `set_permissions` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::set_permissions`] for more details.
#[inline]
pub async fn set_permissions<P: AsRef<Path>>(path: P, perm: Permissions) -> io::Result<()> {
    FileSystem::new().set_permissions(path, perm).await
}

/// Invoke `symlink_metadata` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::symlink_metadata`] for more details.
#[inline]
pub async fn symlink_metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    FileSystem::new().symlink_metadata(path).await
}

/// Invoke `read_dir` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::read_dir`] for more details.
#[inline]
pub async fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<ReadDir> {
    FileSystem::new().read_dir(path).await
}

/// Invoke `open_file` via global register [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::open_file`] for more details.
/// Opens a file with [`open_mode`](FileOpenMode)
pub async fn open_file<P: AsRef<Path>>(path: P, open_mode: FileOpenMode) -> io::Result<File> {
    FileSystem::new().open_file(path, open_mode).await
}

/// Invoke `exists` via global register [`syscall`](rasi_syscall::FileSystem) to check if provided path is exists.
///
/// See [`FileSystem::exists`] for more details.
pub async fn exists<P: AsRef<Path>>(path: P) -> bool {
    FileSystem::new().exists(path).await
}
