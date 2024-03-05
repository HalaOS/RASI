# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for compatibility with GitHub comment style markdown rendering.
-->

## [0.1.3] - 2024-03-05

- Add official specification test suite.
- Add [`path`](/crates/syscall/src/path/) mod which is an async version of [`std::path`].
- Replace type [`std::path::Path`](https://doc.rust-lang.org/std/path/struct.Path.html) of trait [`FileSystem`](/crates/syscall/src/fs.rs) with type [`rasi::path::Path`](/crates/syscall/src/path/mod.rs)
