use std::{env::temp_dir, io::SeekFrom};

use futures::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use rasi::fs::{self, syscall, FileOpenMode};

use crate::async_spec;

async fn test_fs(syscall: &dyn syscall::Driver) {
    let fs: fs::FileSystem = syscall.into();

    fs.canonicalize("./").await.unwrap();

    let temp_dir = temp_dir();

    let mut test_dir = temp_dir.clone();
    test_dir.push("test");

    if fs.metadata(test_dir.clone()).await.is_ok() {
        fs.remove_dir_all(test_dir.clone()).await.unwrap();
    }

    fs.create_dir(test_dir.clone()).await.unwrap();

    test_dir.push("test1");
    test_dir.push("test2");

    fs.create_dir_all(test_dir.clone()).await.unwrap();

    assert!(fs.is_dir(temp_dir).await);

    let mut test_file = test_dir.clone();
    test_file.push("test");

    let mut file = fs
        .open_file(
            test_file.clone(),
            FileOpenMode::CreateNew | FileOpenMode::Writable,
        )
        .await
        .unwrap();

    let content = b"hellow rasi filesystem.";

    file.write_all(content).await.unwrap();

    assert!(fs.is_file(test_file.clone()).await);

    let mut test_file2 = test_dir.clone();
    test_file2.push("test1");

    file.close().await.unwrap();

    fs.copy(test_file.clone(), test_file2.clone())
        .await
        .unwrap();

    let mut file2 = fs
        .open_file(test_file2.clone(), FileOpenMode::Readable)
        .await
        .unwrap();

    let mut buf = Vec::with_capacity(1024);

    let _ = file2.read_to_end(&mut buf).await.unwrap();

    assert_eq!(&buf, content);

    file2
        .seek(SeekFrom::Current(-(buf.len() as i64)))
        .await
        .unwrap();

    buf.clear();

    let _ = file2.read_to_end(&mut buf).await.unwrap();

    assert_eq!(&buf, content);

    let mut link = test_dir.clone();
    link.push("link");

    fs.hard_link(test_file.clone(), link.clone()).await.unwrap();

    let mut link_file = fs
        .open_file(link.clone(), FileOpenMode::Readable)
        .await
        .unwrap();

    let mut buf = Vec::with_capacity(1024);

    let _ = link_file.read_to_end(&mut buf).await.unwrap();

    assert_eq!(&buf, content);

    fs.remove_file(link).await.unwrap();

    fs.remove_file(test_file2).await.unwrap();

    fs.remove_dir_all(test_dir).await.unwrap();
}

pub async fn run_fs_spec(syscall: &dyn syscall::Driver) {
    println!("Run filesystem spec testsuite");
    println!("");

    async_spec!(test_fs, syscall);

    println!("");
}
