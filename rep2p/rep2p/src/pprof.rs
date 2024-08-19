use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use async_trait::async_trait;
use identity::PeerId;

use crate::driver_wrapper;

/// A libp2p pprof driver must implement the `Driver-*` traits in this module.
pub mod syscall {
    use std::time::Duration;

    use async_trait::async_trait;
    use identity::PeerId;

    #[async_trait]
    pub trait DriverProfiler: Sync + Send {
        /// Record connecting time spent.
        async fn connect(&self, id: &str, peer_id: &PeerId, addrs: usize, spent_time: Duration, inbound: bool);

        async fn connect_failed(&self, peer_id: &PeerId, addrs: usize, spent_time: Duration);

        /// Record one connection to `peer_id` is closed.
        async fn disconnected(&self, id: &str, peer_id: &PeerId);

        /// Record opening a stream succeed.
        async fn open_stream(&self, id: &str, peer_id: &PeerId, proto: &str);

        /// Record opening a stream succeed.
        async fn open_stream_failed(&self, peer_id: &PeerId, proto: &[String]);

        /// Record close one stream.
        async fn close_stream(&self, peer_id: &PeerId, proto: &str);
    }
}

driver_wrapper!(
    ["A type wrapper of [`DriverProfiler`](syscall::DriverProfiler)"]
   Profiler[syscall::DriverProfiler]
);

#[derive(Debug, Default)]
pub struct DefaultProfiler {
    connect_times: AtomicU64,
    connect_success: AtomicU64,
    connect_failed: AtomicU64,
    disconnected: AtomicU64,
    stream_open_success: AtomicU64,
    stream_open_failed: AtomicU64,
    stream_closed: AtomicU64,
}

impl DefaultProfiler {
    fn write_log(&self) {
        let conn_avg = self.connect_times.load(Ordering::Relaxed)
            / (self.connect_success.load(Ordering::Relaxed)
                + self.connect_failed.load(Ordering::Relaxed));

        log::trace!(
            "conn_avg={}s, conn_succ={}, conn_failed={}, conn_active={}, stream_succ={}, stream_failed={}, stream_active={}",
            conn_avg,
            self.connect_success.load(Ordering::Relaxed),
            self.connect_failed.load(Ordering::Relaxed),
            self.connect_success.load(Ordering::Relaxed)
                - self.disconnected.load(Ordering::Relaxed),                     
            self.stream_open_success.load(Ordering::Relaxed),
            self.stream_open_failed.load(Ordering::Relaxed),
            self.stream_open_success.load(Ordering::Relaxed)
                - self.stream_closed.load(Ordering::Relaxed)
        );
    }
}

#[async_trait]
impl syscall::DriverProfiler for DefaultProfiler {
    /// Record connecting time spent.
    async fn connect(&self, _id: &str, _peer_id: &PeerId, _addrs: usize, spent_time: Duration, _inbound: bool) {
        self.connect_times
            .fetch_add(spent_time.as_secs(), Ordering::Relaxed);
        self.connect_success.fetch_add(1, Ordering::Relaxed);

        self.write_log();
    }

    async fn connect_failed(&self, _peer_id: &PeerId, _addrs: usize, spent_time: Duration) {
        self.connect_times
            .fetch_add(spent_time.as_secs(), Ordering::Relaxed);
        self.connect_failed.fetch_add(1, Ordering::Relaxed);

        self.write_log();
    }

    /// Record one connection to `peer_id` is closed.
    async fn disconnected(&self, _id: &str, _peer_id: &PeerId) {
        self.disconnected.fetch_add(1, Ordering::Relaxed);

        self.write_log();
    }

    /// Record opening a stream succeed.
    async fn open_stream(&self, _id: &str, _peer_id: &PeerId, _proto: &str) {
        self.stream_open_success.fetch_add(1, Ordering::Relaxed);

        self.write_log();
    }

    /// Record opening a stream succeed.
    async fn open_stream_failed(&self, _peer_id: &PeerId, _proto: &[String]) {
        self.stream_open_failed.fetch_add(1, Ordering::Relaxed);

        self.write_log();
    }

    /// Record close one stream.
    async fn close_stream(&self, _peer_id: &PeerId, _proto: &str) {
        self.stream_closed.fetch_add(1, Ordering::Relaxed);

        self.write_log();
    }
}
