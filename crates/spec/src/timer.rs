use std::time::Duration;

use futures::future::pending;
use rasi::timer::{sleep_with, syscall, TimeoutExt};

use crate::async_spec;

pub async fn test_sleep(syscall: &dyn syscall::Driver) {
    sleep_with(Duration::from_micros(10), syscall).await;

    sleep_with(Duration::from_millis(20), syscall).await;

    sleep_with(Duration::from_secs(1), syscall).await;
}

pub async fn test_timeout(syscall: &dyn syscall::Driver) {
    let never = pending::<()>();
    let dur = Duration::from_millis(5);

    assert!(never.timeout_with(dur, syscall).await.is_none());
}

pub async fn run_timer_spec(syscall: &'static dyn syscall::Driver) {
    println!("Run timer spec testsuite");
    println!("");

    async_spec!(test_sleep, syscall);
    async_spec!(test_timeout, syscall);

    println!("");
}
