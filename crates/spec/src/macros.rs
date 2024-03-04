#[macro_export]
macro_rules! async_spec {
    ($f: ident, $syscall: expr) => {
        print!("spec {} ...", stringify!($f));

        $f($syscall).await;
        println!(" ok");
    };
}
