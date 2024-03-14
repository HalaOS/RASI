//! Filesystem manipulation operations.
//!
//! This module is an async version of std::fs.

use std::{
    fs::{FileType, Metadata, Permissions},
    io,
    task::Poll,
};

use futures::{AsyncRead, AsyncSeek, AsyncWrite, Stream};
use rasi_syscall::{global_filesystem, FileOpenMode, Handle};

use crate::utils::cancelable_would_block;
use rasi_syscall::path::{Path, PathBuf};

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
    ///     log::trace!("{}", entry.file_name());
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
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs;
    /// use rasi::prelude::*;
    /// use rasi::syscall::FileOpenMode;
    ///
    /// fs::open_file(
    ///    "does_not_exist.txt",
    ///    FileOpenMode::CreateNew | FileOpenMode::Writable,
    /// ).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
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
    /// use rasi::path::Path;
    /// assert_eq!(rasi::fs::exists(Path::new("does_not_exist.txt")).await, false);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.metadata(path).await.is_ok()
    }

    /// Returns `true` if the path exists on disk and is pointing at a directory.
    ///
    /// This function will traverse symbolic links to query information about the
    /// destination file. In case of broken symbolic links this will return `false`.
    ///
    /// If you cannot access the directory containing the file, e.g., because of a
    /// permission error, this will return `false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::path::Path;
    ///
    /// assert_eq!(rasi::fs::is_dir(Path::new("./is_a_directory/")).await, true);
    /// assert_eq!(rasi::fs::is_dir(Path::new("a_file.txt")).await, false);
    /// #
    /// # Ok(()) }) }
    /// ```
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
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::path::Path;
    /// assert_eq!(rasi::fs::is_file(Path::new("./is_a_directory/")).await, false);
    /// assert_eq!(rasi::fs::is_file(Path::new("a_file.txt")).await, true);
    /// #
    /// # Ok(()) }) }
    /// ```
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
    /// use rasi::syscall::FileOpenMode;
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
    /// use rasi::syscall::FileOpenMode;
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
    /// use rasi::syscall::FileOpenMode;
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
    ///     log::trace!("{:?}", entry.path());
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
    ///     log::trace!("{:?}", entry.metadata().await?);
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
    ///     log::trace!("{:?}", entry.file_type().await?);
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
    ///     log::trace!("{}", entry.file_name());
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn file_name(&self) -> String {
        self.syscall.dir_entry_file_name(&self.handle)
    }
}

/// Invoke `canonicalize` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::canonicalize`] for more details.
#[inline]
pub async fn canonicalize<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    FileSystem::new().canonicalize(path).await
}

/// Invoke `copy` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::copy`] for more details.
#[inline]
pub async fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<u64> {
    FileSystem::new().copy(from, to).await
}

/// Invoke `create_dir` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::create_dir`] for more details.
#[inline]
pub async fn create_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    FileSystem::new().create_dir(path).await
}

/// Invoke `create_dir_all` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::create_dir_all`] for more details.
#[inline]
pub async fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    FileSystem::new().create_dir_all(path).await
}

/// Invoke `hard_link` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::hard_link`] for more details.
#[inline]
pub async fn hard_link<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    FileSystem::new().hard_link(from, to).await
}

/// Invoke `metadata` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::metadata`] for more details.
#[inline]
pub async fn metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    FileSystem::new().metadata(path).await
}

/// Invoke `read_link` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::read_link`] for more details.
#[inline]
pub async fn read_link<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    FileSystem::new().read_link(path).await
}

/// Invoke `remove_dir` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::remove_dir`] for more details.
#[inline]
pub async fn remove_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    FileSystem::new().remove_dir(path).await
}

/// Invoke `remove_dir_all` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::remove_dir_all`] for more details.
#[inline]
pub async fn remove_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    FileSystem::new().remove_dir_all(path).await
}

/// Invoke `remove_file` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::remove_file`] for more details.
#[inline]
pub async fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    FileSystem::new().remove_file(path).await
}

/// Invoke `rename` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::rename`] for more details.
#[inline]
pub async fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    FileSystem::new().rename(from, to).await
}

/// Invoke `set_permissions` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::set_permissions`] for more details.
#[inline]
pub async fn set_permissions<P: AsRef<Path>>(path: P, perm: Permissions) -> io::Result<()> {
    FileSystem::new().set_permissions(path, perm).await
}

/// Invoke `symlink_metadata` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::symlink_metadata`] for more details.
#[inline]
pub async fn symlink_metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    FileSystem::new().symlink_metadata(path).await
}

/// Invoke `read_dir` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::read_dir`] for more details.
#[inline]
pub async fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<ReadDir> {
    FileSystem::new().read_dir(path).await
}

/// Invoke `open_file` via global registered [`syscall`](rasi_syscall::FileSystem)
///
/// See [`FileSystem::open_file`] for more details.
/// Opens a file with [`open_mode`](FileOpenMode)
pub async fn open_file<P: AsRef<Path>>(path: P, open_mode: FileOpenMode) -> io::Result<File> {
    FileSystem::new().open_file(path, open_mode).await
}

/// Invoke `exists` via global registered [`syscall`](rasi_syscall::FileSystem) to check if provided path is exists.
///
/// See [`FileSystem::exists`] for more details.
pub async fn exists<P: AsRef<Path>>(path: P) -> bool {
    FileSystem::new().exists(path).await
}

/// Invoke `is_dir` via global registered [`syscall`](rasi_syscall::FileSystem) to check if provided path is a directory.
///
/// See [`FileSystem::exists`] for more details.
pub async fn is_dir<P: AsRef<Path>>(path: P) -> bool {
    FileSystem::new().is_dir(path).await
}

/// Invoke `is_file` via global registered [`syscall`](rasi_syscall::FileSystem) to check if provided path is a file.
///
/// See [`FileSystem::exists`] for more details.
pub async fn is_file<P: AsRef<Path>>(path: P) -> bool {
    FileSystem::new().is_file(path).await
}

/// A builder for opening files with configurable options.
///
/// Files can be opened in [`read`] and/or [`write`] mode.
///
/// The [`append`] option opens files in a special writing mode that moves the file cursor to the
/// end of file before every write operation.
///
/// It is also possible to [`truncate`] the file right after opening, to [`create`] a file if it
/// doesn't exist yet, or to always create a new file with [`create_new`].
///
/// This type is an async version of [`std::fs::OpenOptions`].
///
/// [`read`]: #method.read
/// [`write`]: #method.write
/// [`append`]: #method.append
/// [`truncate`]: #method.truncate
/// [`create`]: #method.create
/// [`create_new`]: #method.create_new
/// [`std::fs::OpenOptions`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html
///
/// # Examples
///
/// Open a file for reading:
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
/// #
/// use rasi::fs::OpenOptions;
///
/// let file = OpenOptions::new()
///     .read(true)
///     .open("a.txt")
///     .await?;
/// #
/// # Ok(()) }) }
/// ```
///
/// Open a file for both reading and writing, and create it if it doesn't exist yet:
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
/// #
/// use rasi::fs::OpenOptions;
///
/// let file = OpenOptions::new()
///     .read(true)
///     .write(true)
///     .create(true)
///     .open("a.txt")
///     .await?;
/// #
/// # Ok(()) }) }
/// ```
#[derive(Clone, Debug)]
pub struct OpenOptions(FileOpenMode);

impl OpenOptions {
    /// Creates a blank set of options.
    ///
    /// All options are initially set to `false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .read(true)
    ///     .open("a.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn new() -> OpenOptions {
        OpenOptions(FileOpenMode::none())
    }

    /// Configures the option for read mode.
    ///
    /// When set to `true`, this option means the file will be readable after opening.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .read(true)
    ///     .open("a.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn read(&mut self, read: bool) -> &mut OpenOptions {
        if read {
            self.0 = self.0.xor(FileOpenMode::Readable);
        } else {
            self.0 = self.0.and(FileOpenMode::Readable);
        }

        self
    }

    /// Configures the option for write mode.
    ///
    /// When set to `true`, this option means the file will be writable after opening.
    ///
    /// If the file already exists, write calls on it will overwrite the previous contents without
    /// truncating it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .write(true)
    ///     .open("a.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn write(&mut self, write: bool) -> &mut OpenOptions {
        if write {
            self.0 = self.0.xor(FileOpenMode::Writable);
        } else {
            self.0 = self.0.and(FileOpenMode::Writable);
        }
        self
    }

    /// Configures the option for append mode.
    ///
    /// When set to `true`, this option means the file will be writable after opening and the file
    /// cursor will be moved to the end of file before every write operaiton.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .append(true)
    ///     .open("a.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn append(&mut self, append: bool) -> &mut OpenOptions {
        if append {
            self.0 = self.0.xor(FileOpenMode::Append);
        } else {
            self.0 = self.0.and(FileOpenMode::Append);
        }

        self
    }

    /// Configures the option for truncating the previous file.
    ///
    /// When set to `true`, the file will be truncated to the length of 0 bytes.
    ///
    /// The file must be opened in [`write`] or [`append`] mode for truncation to work.
    ///
    /// [`write`]: #method.write
    /// [`append`]: #method.append
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .write(true)
    ///     .truncate(true)
    ///     .open("a.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn truncate(&mut self, truncate: bool) -> &mut OpenOptions {
        if truncate {
            self.0 = self.0.xor(FileOpenMode::Truncate);
        } else {
            self.0 = self.0.and(FileOpenMode::Truncate);
        }

        self
    }

    /// Configures the option for creating a new file if it doesn't exist.
    ///
    /// When set to `true`, this option means a new file will be created if it doesn't exist.
    ///
    /// The file must be opened in [`write`] or [`append`] mode for file creation to work.
    ///
    /// [`write`]: #method.write
    /// [`append`]: #method.append
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .write(true)
    ///     .create(true)
    ///     .open("a.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn create(&mut self, create: bool) -> &mut OpenOptions {
        if create {
            self.0 = self.0.xor(FileOpenMode::Create);
        } else {
            self.0 = self.0.and(FileOpenMode::Create);
        }

        self
    }

    /// Configures the option for creating a new file or failing if it already exists.
    ///
    /// When set to `true`, this option means a new file will be created, or the open operation
    /// will fail if the file already exists.
    ///
    /// The file must be opened in [`write`] or [`append`] mode for file creation to work.
    ///
    /// [`write`]: #method.write
    /// [`append`]: #method.append
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .write(true)
    ///     .create_new(true)
    ///     .open("a.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn create_new(&mut self, create_new: bool) -> &mut OpenOptions {
        if create_new {
            self.0 = self.0.xor(FileOpenMode::CreateNew);
        } else {
            self.0 = self.0.and(FileOpenMode::CreateNew);
        }
        self
    }

    /// Opens a file with the configured options.
    ///
    /// # Errors
    ///
    /// An error will be returned in the following situations:
    ///
    /// * The file does not exist and neither [`create`] nor [`create_new`] were set.
    /// * The file's parent directory does not exist.
    /// * The current process lacks permissions to open the file in the configured mode.
    /// * The file already exists and [`create_new`] was set.
    /// * Invalid combination of options was used, like [`truncate`] was set but [`write`] wasn't,
    ///   or none of [`read`], [`write`], and [`append`] modes was set.
    /// * An OS-level occurred, like too many files are open or the file name is too long.
    /// * Some other I/O error occurred.
    ///
    /// [`read`]: #method.read
    /// [`write`]: #method.write
    /// [`append`]: #method.append
    /// [`truncate`]: #method.truncate
    /// [`create`]: #method.create
    /// [`create_new`]: #method.create_new
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { futures::executor::block_on(async {
    /// #
    /// use rasi::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .read(true)
    ///     .open("a.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn open<P: AsRef<Path>>(&self, path: P) -> io::Result<File> {
        let path = path.as_ref().to_owned();

        open_file(path, self.0).await
    }

    /// Opens a file with the configured options and the custom [`syscall`](rasi_syscall::FileSystem).
    ///
    /// For more details, see the document of function [`open`](Self::open).
    pub async fn open_with<P: AsRef<Path>>(
        &self,
        path: P,
        syscall: &'static dyn rasi_syscall::FileSystem,
    ) -> io::Result<File> {
        let path = path.as_ref().to_owned();

        FileSystem::new_with(syscall).open_file(path, self.0).await
    }
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(windows, feature = "windows_named_pipe"))]
mod windows {
    use std::{
        ffi::{OsStr, OsString},
        io,
        task::Poll,
    };

    use futures::{AsyncRead, AsyncWrite};
    use rasi_syscall::{global_named_pipe, Handle, NamedPipe};

    use crate::utils::cancelable_would_block;

    /// The named pipe server side socket.
    pub struct NamedPipeListener {
        /// named pipe bind address.
        addr: OsString,
        /// a reference to syscall interface .
        syscall: &'static dyn NamedPipe,
    }

    impl NamedPipeListener {
        /// Create new named pipe server with custom [`syscall`](NamedPipe) and bind to `addr`
        pub async fn bind_with<A: AsRef<OsStr>>(
            addr: A,
            syscall: &'static dyn NamedPipe,
        ) -> io::Result<Self> {
            Ok(Self {
                addr: addr.as_ref().to_os_string(),
                syscall,
            })
        }

        /// Create new named pipe server with global [`syscall`](NamedPipe) and bind to `addr`.
        pub async fn bind<A: AsRef<OsStr>>(addr: A) -> io::Result<Self> {
            Self::bind_with(addr, global_named_pipe()).await
        }

        /// Accepts a new incoming connection to this listener.
        ///
        /// When a connection is established, the corresponding stream and address will be returned.
        pub async fn accept(&self) -> io::Result<NamedPipeStream> {
            let socket = cancelable_would_block(|cx| {
                self.syscall.server_create(cx.waker().clone(), &self.addr)
            })
            .await?;

            let stream = NamedPipeStream::new(socket, self.syscall);

            cancelable_would_block(|cx| {
                self.syscall
                    .server_accept(cx.waker().clone(), &stream.socket)
            })
            .await?;

            Ok(stream)
        }
    }

    /// The stream object of named pipe.
    pub struct NamedPipeStream {
        /// Named pipe stream handle.
        socket: Handle,
        /// a reference to syscall interface .
        syscall: &'static dyn NamedPipe,

        /// The cancel handle reference to latest pending write ops.
        write_cancel_handle: Option<Handle>,

        /// The cancel handle reference to latest pending read ops.
        read_cancel_handle: Option<Handle>,
    }

    impl NamedPipeStream {
        fn new(socket: Handle, syscall: &'static dyn NamedPipe) -> Self {
            Self {
                socket,
                syscall,
                write_cancel_handle: None,
                read_cancel_handle: None,
            }
        }

        /// Create new client named pipe stream and connect to `addr`
        pub async fn connect_with<A: AsRef<OsStr>>(
            addr: A,
            syscall: &'static dyn NamedPipe,
        ) -> io::Result<Self> {
            let socket =
                cancelable_would_block(|cx| syscall.client_open(cx.waker().clone(), addr.as_ref()))
                    .await?;

            Ok(Self::new(socket, syscall))
        }
    }

    impl AsyncRead for NamedPipeStream {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut [u8],
        ) -> std::task::Poll<io::Result<usize>> {
            match self.syscall.read(cx.waker().clone(), &self.socket, buf) {
                rasi_syscall::CancelablePoll::Ready(r) => Poll::Ready(r),
                rasi_syscall::CancelablePoll::Pending(read_cancel_handle) => {
                    self.read_cancel_handle = Some(read_cancel_handle);
                    Poll::Pending
                }
            }
        }
    }

    impl AsyncWrite for NamedPipeStream {
        fn poll_write(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            match self.syscall.write(cx.waker().clone(), &self.socket, buf) {
                rasi_syscall::CancelablePoll::Ready(r) => Poll::Ready(r),
                rasi_syscall::CancelablePoll::Pending(write_cancel_handle) => {
                    self.write_cancel_handle = Some(write_cancel_handle);
                    Poll::Pending
                }
            }
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
}

#[cfg(all(windows, feature = "windows_named_pipe"))]
pub use windows::*;
