use std::{net::SocketAddr, time::Duration};

use rasi::{net::UdpSocket, task::spawn_ok, timer::TimeoutExt};

use crate::{
    client::{DnsLookup, DnsLookupState},
    Result,
};

#[cfg(all(unix, feature = "sysconf"))]
mod sys {
    use std::net::{IpAddr, SocketAddr};

    use crate::{Error, Result};

    /// Get the system-wide DNS name server configuration.
    pub fn name_server() -> Result<SocketAddr> {
        let config = std::fs::read("/etc/resolv.conf")?;

        let config = resolv_conf::Config::parse(&config)?;

        for name_server in config.nameservers {
            let ip_addr: IpAddr = name_server.into();

            return Ok((ip_addr, 53).into());
        }

        return Err(Error::SysWideNameServer.into());
    }
}

#[cfg(all(windows, feature = "sysconf"))]
mod sys {
    use std::net::SocketAddr;

    use crate::{Error, Result};

    /// Get the system-wide DNS name server configuration.
    pub fn name_server() -> Result<SocketAddr> {
        for adapter in ipconfig::get_adapters()? {
            for ip_addr in adapter.dns_servers() {
                return Ok((ip_addr.clone(), 53).into());
            }
        }

        return Err(Error::SysWideNameServer.into());
    }
}

impl DnsLookup {
    /// Create a DNS lookup with sys-wide DNS name server configuration.
    #[cfg(feature = "sysconf")]
    pub async fn over_udp() -> Result<Self> {
        Self::with_udp_server(sys::name_server()?).await
    }

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

            lookup_cloned.close();
        });

        spawn_ok(async move {
            if let Err(err) = Self::udp_recv_loop(&lookup, &socket, nameserver).await {
                log::error!("DnsLookup, stop recv loop with error: {}", err);
            } else {
                log::trace!("DnsLookup, stop recv loop.",);
            }

            lookup.close();
        });

        Ok(this)
    }

    async fn udp_send_loop(
        lookup: &DnsLookupState,
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
        lookup: &DnsLookupState,
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
    use std::{sync::Once, time::Duration};

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

        {
            let lookup = DnsLookup::over_udp().await.unwrap();

            let group = lookup.lookup_ip("docs.rs").await.unwrap();

            log::trace!("{:?}", group);
        };

        sleep(Duration::from_secs(1)).await;
    }

    #[futures_test::test]
    async fn test_udp_lookup_txt() {
        init();

        {
            let lookup = DnsLookup::over_udp().await.unwrap();

            let group = lookup
                .lookup_txt("_dnsaddr.bootstrap.libp2p.io")
                .await
                .unwrap();

            log::trace!("{:?}", group);
        };

        sleep(Duration::from_secs(1)).await;
    }
}
