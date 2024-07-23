# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for compatibility with GitHub comment style markdown rendering.
-->

## [0.2.2] - 2024-07-24

- First release of the rasi rewrite.

## [0.1.11] - 2024-03-15

- [Add http client / server support](crates/ext/src/http).

## [0.1.10] - 2024-03-12

- Quic: QuicListener replace futures::lock::Mutex as [`AsyncSpinMutex`](crates/ext/src/utils/sync/).

## [0.1.9] - 2024-03-12

- Fixed: ext doc generation bugs.

## [0.1.8] - 2024-03-12

- Fixed: some quic bugs.

## [0.1.6] - 2024-03-05

- add utility type **OpenOptions**: a builder for opening files with configurable options.

## [0.1.5] - 2024-03-05

- fixed: error in generating crate document of [`rasi-default`](/crates/default/)

## [0.1.4] - 2024-03-05

- Add official specification test suite.
- Add [`path`](/crates/syscall/src/path/) mod which is an async version of [`std::path`].
- Replace type [`std::path::Path`](https://doc.rust-lang.org/std/path/struct.Path.html) of trait [`FileSystem`](/crates/syscall/src/fs.rs) with type [`rasi::path::Path`](/crates/syscall/src/path/mod.rs)
