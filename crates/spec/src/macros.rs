#[macro_export]
macro_rules! async_spec {
    ($f: ident, $syscall: expr) => {
        print!("spec {} ...", stringify!($f));

        let result = $f($syscall).await;

        if result.is_ok() {
            println!(" ok");
        } else {
            println!(" panic");
            println!("{:?}", result);
        }
    };
}
