use std::{net::SocketAddr, time::Duration};

use rasi::{net::UdpSocket, task::spawn_ok, timer::TimeoutExt};

use crate::{
    client::{DnsLookup, DnsLookupWithoutDrop},
    Result,
};

impl DnsLookup {
    /// Create a DNS lookup over udp socket.
    pub async fn with_udp_server(nameserver: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind(if nameserver.is_ipv4() {
            "0.0.0.0:0".parse::<SocketAddr>()?
        } else {
            "[::]:0".parse::<SocketAddr>()?
        })
        .await?;

        let this = Self::default();

        let lookup = this.to_inner();

        let lookup_cloned = lookup.clone();
        let socket_cloned = socket.clone();
        let server_cloned = nameserver.clone();

        spawn_ok(async move {
            if let Err(err) =
                Self::udp_send_loop(&lookup_cloned, &socket_cloned, server_cloned).await
            {
                log::error!("DnsLookup, stop send loop with error: {}", err);
            } else {
                log::trace!("DnsLookup, stop send loop.",);
            }

            lookup_cloned.close_send();
        });

        spawn_ok(async move {
            if let Err(err) = Self::udp_recv_loop(&lookup, &socket, nameserver).await {
                log::error!("DnsLookup, stop recv loop with error: {}", err);
            } else {
                log::trace!("DnsLookup, stop recv loop.",);
            }

            lookup.close_recv();
        });

        Ok(this)
    }

    async fn udp_send_loop(
        lookup: &DnsLookupWithoutDrop,
        socket: &UdpSocket,
        server: SocketAddr,
    ) -> Result<()> {
        loop {
            let buf = lookup.send().await?;

            let send_size = socket.send_to(buf, server).await?;

            log::trace!("DnsLookup, send len={} raddr={}", send_size, server);
        }
    }

    async fn udp_recv_loop(
        lookup: &DnsLookupWithoutDrop,
        socket: &UdpSocket,
        server: SocketAddr,
    ) -> Result<()> {
        let mut buf = vec![0; 1024 * 1024];

        log::trace!("DnsLookup, udp listener on {}", socket.local_addr()?);

        loop {
            let (read_size, from) = match socket
                .recv_from(&mut buf)
                .timeout(Duration::from_millis(200))
                .await
            {
                Some(Ok(r)) => r,
                Some(Err(err)) => return Err(err.into()),
                None => {
                    if lookup.is_closed() {
                        return Ok(());
                    }

                    continue;
                }
            };

            if from != server {
                log::warn!("DnsLookup, recv packet from unknown peer={}", from);
            } else {
                log::trace!("DnsLookup, recv response len={}", read_size);
            }

            lookup.recv(&buf[..read_size]).await?;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{atomic::Ordering, Once},
        time::Duration,
    };

    use rasi::timer::sleep;
    use rasi_mio::{net::register_mio_network, timer::register_mio_timer};

    use crate::client::DnsLookup;

    fn init() {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            _ = pretty_env_logger::try_init_timed();

            register_mio_network();
            register_mio_timer();
        });
    }

    #[futures_test::test]
    async fn test_udp_lookup() {
        init();

        let innner = {
            let lookup = DnsLookup::with_udp_server("8.8.8.8:53".parse().unwrap())
                .await
                .unwrap();

            let group = lookup.lookup_ip("docs.rs").await.unwrap();

            log::trace!("{:?}", group);

            lookup.to_inner()
        };

        sleep(Duration::from_secs(1)).await;

        assert_eq!(innner.0.send_closed.load(Ordering::Relaxed), true);
        assert_eq!(innner.0.recv_closed.load(Ordering::Relaxed), true);
    }
}
