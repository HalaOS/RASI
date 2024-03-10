use std::{ops, time::Duration};

/// An wrapper for quiche [`Config`](quiche::Config).
pub struct Config {
    quiche_config: quiche::Config,
    /// The time interval for sending ping packets.
    pub ping_packet_send_interval: Option<Duration>,
    /// `max_udp_payload_size transport` parameter.
    pub max_recv_udp_payload_size: usize,
    /// `max_udp_payload_size transport` parameter.
    pub max_send_udp_payload_size: usize,
}

impl ops::Deref for Config {
    type Target = quiche::Config;

    fn deref(&self) -> &Self::Target {
        &self.quiche_config
    }
}

impl ops::DerefMut for Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.quiche_config
    }
}

impl Config {
    /// Create new quic config instance with default config.
    pub fn new() -> Self {
        let mut config = Config {
            quiche_config: quiche::Config::new(quiche::PROTOCOL_VERSION).unwrap(),
            ping_packet_send_interval: None,
            max_recv_udp_payload_size: 65527,
            max_send_udp_payload_size: 1200,
        };

        config.set_initial_max_stream_data_bidi_local(1024 * 1024);
        config.set_initial_max_stream_data_bidi_remote(1024 * 1024);
        config.set_initial_max_streams_bidi(100);
        config.set_initial_max_streams_uni(100);

        config
    }

    /// Sets the `max_idle_timeout` transport parameter, in milliseconds.
    ///
    /// The default value is infinite, that is, no timeout is used.
    ///
    /// This function also set the `ping_packet_send_interval` as `max_idle_timeout` / 2
    pub fn set_max_idle_timeout(&mut self, v: u64) {
        self.ping_packet_send_interval = Some(Duration::from_millis(v / 2));
        self.quiche_config.set_max_idle_timeout(v)
    }

    /// Sets the `max_udp_payload_size transport` parameter.
    ///
    /// The default value is `65527`.
    pub fn set_max_recv_udp_payload_size(&mut self, v: usize) {
        self.max_recv_udp_payload_size = v;
        self.quiche_config.set_max_recv_udp_payload_size(v)
    }

    /// Sets the maximum outgoing UDP payload size.
    ///
    /// The default and minimum value is `1200`.
    pub fn set_max_send_udp_payload_size(&mut self, v: usize) {
        self.max_send_udp_payload_size = v;
        self.quiche_config.set_max_send_udp_payload_size(v)
    }
}
