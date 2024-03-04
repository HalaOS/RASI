# rasi-default

This `crate` is a reference implementation of the [`rasi syscalls`](https://docs.rs/rasi-syscall/latest/rasi_syscall).

* The implementations of [`Network`](https://docs.rs/rasi-syscall/latest/rasi_syscall/trait.Network.html) ***syscall*** and [`Timer`](https://docs.rs/rasi-syscall/latest/rasi_syscall/trait.Timer.html) ***syscall*** are based on [`mio`](https://github.com/tokio-rs/mio);

* The implementation of [`Executor`](https://docs.rs/rasi-syscall/latest/rasi_syscall/trait.Executor.html) ***syscall*** is based on [`futures thread pool`](https://docs.rs/futures/latest/futures/executor/struct.ThreadPool.html#);

* The implementation of [`FileSystem`](https://docs.rs/rasi-syscall/latest/rasi_syscall/trait.FileSystem.html) ***syscall*** is based on [`std library`](https://doc.rust-lang.org/std/fs/).
