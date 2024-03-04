use std::{io, time::Duration};

use rasi::time::sleep_with;

use crate::async_spec;

pub async fn test_sleep(syscall: &'static dyn rasi_syscall::Timer) -> io::Result<()> {
    sleep_with(Duration::from_micros(10), syscall).await;

    sleep_with(Duration::from_millis(20), syscall).await;

    sleep_with(Duration::from_secs(1), syscall).await;

    Ok(())
}

pub async fn run_timer_spec(syscall: &'static dyn rasi_syscall::Timer) {
    println!("Run timer spec testsuite");
    println!("");

    async_spec!(test_sleep, syscall);

    println!("");
}
