use std::sync::Once;

use rasi_default::{
    executor::register_futures_executor, net::register_mio_network, time::register_mio_timer,
};

#[allow(unused)]
pub(crate) fn init() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        #[cfg(windows)]
        rasi_default::fs::register_mio_named_pipe();
        register_mio_network();
        register_mio_timer();
        register_futures_executor().unwrap();
    })
}
