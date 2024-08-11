use std::io::{self, Result};
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite};
use futures_boring::ssl::{
    SslAcceptor, SslAlert, SslConnector, SslMethod, SslVerifyError, SslVerifyMode, SslVersion,
};
use futures_boring::x509::X509;
use futures_boring::{accept, connect, ec, pkey};
use multistream_select::{dialer_select_proto, listener_select_proto, Version};
use rasi::net::{TcpListener, TcpStream};
use rep2p::identity::PublicKey;
use rep2p::multiaddr::{Multiaddr, Protocol};
use rep2p::transport::syscall::{DriverConnection, DriverListener, DriverStream, DriverTransport};
use rep2p::transport::{Connection, Listener, Stream};
use rep2p::Switch;
use rep2p_mux::{Reason, YamuxConn, YamuxStream, INIT_WINDOW_SIZE};
use uuid::Uuid;

fn to_sockaddr(addr: &Multiaddr) -> Option<SocketAddr> {
    let mut iter = addr.iter();

    let ip = match iter.next()? {
        Protocol::Ip4(ip) => IpAddr::from(ip),
        Protocol::Ip6(ip) => IpAddr::from(ip),
        _ => return None,
    };

    let next = iter.next()?;

    match next {
        Protocol::Tcp(port) | Protocol::Udp(port) => {
            return Some(SocketAddr::new(ip, port));
        }
        _ => {}
    }

    None
}

/// The libp2p tcp transport implementation.
pub struct TcpTransport;

#[async_trait]
impl DriverTransport for TcpTransport {
    async fn bind(&self, laddr: &Multiaddr, switch: Switch) -> Result<Listener> {
        let (cert, pk) = rep2p_x509::generate(switch.keystore()).await?;

        let cert = X509::from_der(&cert)?;

        let pk = pkey::PKey::from_ec_key(ec::EcKey::private_key_from_der(&pk)?)?;

        let mut ssl_acceptor_builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;

        ssl_acceptor_builder.set_max_proto_version(Some(SslVersion::TLS1_3))?;
        ssl_acceptor_builder.set_min_proto_version(Some(SslVersion::TLS1_3))?;

        ssl_acceptor_builder.set_certificate(&cert)?;

        ssl_acceptor_builder.set_private_key(&pk)?;

        ssl_acceptor_builder.check_private_key()?;

        ssl_acceptor_builder.set_custom_verify_callback(
            SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT,
            |ssl| {
                let cert = ssl
                    .peer_certificate()
                    .ok_or(SslVerifyError::Invalid(SslAlert::CERTIFICATE_REQUIRED))?;

                let cert = cert
                    .to_der()
                    .map_err(|_| SslVerifyError::Invalid(SslAlert::BAD_CERTIFICATE))?;

                let peer_id = rep2p_x509::verify(cert)
                    .map_err(|_| SslVerifyError::Invalid(SslAlert::BAD_CERTIFICATE))?
                    .to_peer_id();

                log::trace!("ssl_server: verified peer={}", peer_id);

                Ok(())
            },
        );

        let ssl_acceptor = ssl_acceptor_builder.build();

        let addr = to_sockaddr(laddr).ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Invalid laddr",
        ))?;

        let listener = TcpListener::bind(addr).await?;

        let laddr = listener.local_addr()?;

        Ok(P2pTcpListener::new(ssl_acceptor, listener, laddr).into())
    }

    /// Connect to peer with remote peer [`raddr`](Multiaddr).
    async fn connect(&self, raddr: &Multiaddr, switch: Switch) -> Result<Connection> {
        let (cert, pk) = rep2p_x509::generate(switch.keystore()).await?;

        let cert = X509::from_der(&cert)?;

        let pk = pkey::PKey::from_ec_key(ec::EcKey::private_key_from_der(&pk)?)?;

        let mut config = SslConnector::builder(SslMethod::tls_client())?;

        config.set_certificate(&cert)?;

        config.set_private_key(&pk)?;

        config.set_max_proto_version(Some(SslVersion::TLS1_3))?;
        config.set_min_proto_version(Some(SslVersion::TLS1_3))?;

        config.set_custom_verify_callback(SslVerifyMode::PEER, |ssl| {
            let cert = ssl
                .peer_certificate()
                .ok_or(SslVerifyError::Invalid(SslAlert::CERTIFICATE_REQUIRED))?;

            let cert = cert
                .to_der()
                .map_err(|_| SslVerifyError::Invalid(SslAlert::BAD_CERTIFICATE))?;

            let peer_id = rep2p_x509::verify(cert)
                .map_err(|_| SslVerifyError::Invalid(SslAlert::BAD_CERTIFICATE))?
                .to_peer_id();

            log::trace!("ssl_client: verified peer={}", peer_id);

            Ok(())
        });

        let config = config.build().configure()?;

        let addr =
            to_sockaddr(raddr).ok_or(io::Error::new(io::ErrorKind::Other, "Invalid laddr"))?;

        let mut stream = TcpStream::connect(addr).await?;

        let laddr = stream.local_addr()?;

        // dynamic select the secure protocol.
        let (_, _) = dialer_select_proto(&mut stream, ["/tls/1.0.0"], Version::V1).await?;

        let mut stream = connect(config, &addr.ip().to_string(), stream)
            .await
            .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;

        let cert = stream
            .ssl()
            .peer_certificate()
            .ok_or(io::Error::new(io::ErrorKind::Other, "Handshaking"))?;

        let public_key = rep2p_x509::verify(cert.to_der()?)?;

        let (_, _) = dialer_select_proto(&mut stream, ["/yamux/1.0.0"], Version::V1).await?;

        let conn = P2pTcpConn::new(laddr, addr, public_key, stream, false)?;

        Ok(conn.into())
    }

    /// Check if this transport support the protocol stack represented by the `addr`.
    fn multiaddr_hit(&self, addr: &Multiaddr) -> bool {
        let stack = addr.protocol_stack().collect::<Vec<_>>();

        if stack.len() > 1 {
            if stack[1] == "tcp" {
                return true;
            }
        }

        return false;
    }
}

struct P2pTcpListener {
    laddr: SocketAddr,
    ssl_acceptor: SslAcceptor,
    listener: TcpListener,
}

impl P2pTcpListener {
    fn new(ssl_acceptor: SslAcceptor, listener: TcpListener, laddr: SocketAddr) -> Self {
        Self {
            laddr,
            ssl_acceptor,
            listener,
        }
    }
}

#[async_trait]
impl DriverListener for P2pTcpListener {
    /// Accept next incoming connection between local and peer.
    async fn accept(&mut self) -> Result<Connection> {
        let (mut stream, raddr) = self.listener.accept().await?;

        let (_, _) = listener_select_proto(&mut stream, ["/tls/1.0.0"]).await?;

        let mut stream = accept(&self.ssl_acceptor, stream)
            .await
            .map_err(|err| io::Error::new(io::ErrorKind::BrokenPipe, err))?;

        let cert = stream
            .ssl()
            .peer_certificate()
            .ok_or(io::Error::new(io::ErrorKind::Other, "Handshaking"))?;

        let public_key = rep2p_x509::verify(cert.to_der()?)?;

        let (_, _) = listener_select_proto(&mut stream, ["/yamux/1.0.0"]).await?;

        let conn = P2pTcpConn::new(self.laddr, raddr, public_key, stream, true)?;

        Ok(conn.into())
    }

    /// Returns the local address that this listener is bound to.
    fn local_addr(&self) -> Result<Multiaddr> {
        let mut addr = Multiaddr::from(self.laddr.ip());
        addr.push(Protocol::Tcp(self.laddr.port()));

        Ok(addr)
    }
}

#[derive(Clone)]
struct P2pTcpConn {
    public_key: PublicKey,
    laddr: Multiaddr,
    raddr: Multiaddr,
    conn: YamuxConn,
    is_closed: Arc<AtomicBool>,
    id: String,
}

impl P2pTcpConn {
    fn new<S>(
        laddr: SocketAddr,
        raddr: SocketAddr,
        public_key: PublicKey,
        stream: S,
        is_server: bool,
    ) -> io::Result<Self>
    where
        S: AsyncWrite + AsyncRead + 'static + Sync + Send,
    {
        let mut m_laddr = Multiaddr::from(laddr.ip());
        m_laddr.push(Protocol::Tcp(laddr.port()));
        m_laddr.push(Protocol::Tls);

        let mut m_raddr = Multiaddr::from(raddr.ip());
        m_raddr.push(Protocol::Tcp(raddr.port()));
        m_raddr.push(Protocol::Tls);

        let (read, write) = stream.split();
        let conn = rep2p_mux::YamuxConn::new_with(INIT_WINDOW_SIZE, is_server, read, write);

        Ok(Self {
            laddr: m_laddr,
            raddr: m_raddr,
            conn,
            public_key,
            is_closed: Default::default(),
            id: Uuid::new_v4().to_string(),
        })
    }
}

#[async_trait]
impl DriverConnection for P2pTcpConn {
    fn id(&self) -> &str {
        &self.id
    }
    /// Returns local bind address.
    ///
    /// This can be useful, for example, when binding to port 0 to figure out which port was
    /// actually bound.
    fn local_addr(&self) -> &Multiaddr {
        &self.laddr
    }

    /// Returns the remote address that this connection is connected to.
    fn peer_addr(&self) -> &Multiaddr {
        &self.raddr
    }

    /// Accept newly incoming stream for reading/writing.
    ///
    /// If the connection is dropping or has been dropped, this function will returns `None`.
    async fn accept(&mut self) -> io::Result<Stream> {
        let stream = self.conn.stream_accept().await?;

        Ok(P2pTcpStream::new(
            stream,
            self.public_key.clone(),
            self.laddr.clone(),
            self.raddr.clone(),
        )
        .into())
    }

    async fn connect(&mut self) -> Result<Stream> {
        let stream = self.conn.stream_open().await?;

        Ok(P2pTcpStream::new(
            stream,
            self.public_key.clone(),
            self.laddr.clone(),
            self.raddr.clone(),
        )
        .into())
    }

    async fn close(&mut self) -> io::Result<()> {
        self.conn.close(Reason::Normal).await?;

        self.is_closed.store(true, Ordering::Relaxed);

        Ok(())
    }

    /// Returns true if this connection is closed or is closing.
    fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Relaxed)
    }

    /// Creates a new independently owned handle to the underlying socket.
    fn clone(&self) -> Connection {
        Clone::clone(self).into()
    }

    /// Return the remote peer's public key.
    fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
}

struct P2pTcpStream {
    id: String,
    stream: YamuxStream,
    public_key: PublicKey,
    laddr: Multiaddr,
    raddr: Multiaddr,
}

impl P2pTcpStream {
    fn new(stream: YamuxStream, public_key: PublicKey, laddr: Multiaddr, raddr: Multiaddr) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            stream,
            public_key,
            laddr,
            raddr,
        }
    }
}

#[async_trait]
impl DriverStream for P2pTcpStream {
    fn id(&self) -> &str {
        &self.id
    }

    /// Return the remote peer's public key.
    fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Returns the local address that this stream is bound to.
    fn local_addr(&self) -> &Multiaddr {
        &self.laddr
    }

    /// Returns the remote address that this stream is connected to.
    fn peer_addr(&self) -> &Multiaddr {
        &self.raddr
    }
    /// Attempt to read data via this stream.
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }

    /// Attempt to write data via this stream.
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }

    /// Attempt to flush the write data.
    fn poll_flush(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    /// Close this connection.
    fn poll_close(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.stream).poll_close(cx)
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use rep2p::{Result, Switch};
    use rep2p_spec::transport::{transport_specs, TransportSpecContext};

    use super::*;

    struct TcpMock;

    #[async_trait]
    impl TransportSpecContext for TcpMock {
        async fn create_switch(&self, protos: &[&str]) -> Result<Switch> {
            Switch::new("test")
                .protos(protos)
                .bind("/ip4/127.0.0.1/tcp/0".parse()?)
                .transport(TcpTransport)
                .create()
                .await
        }
    }

    #[futures_test::test]
    async fn test_specs() {
        // pretty_env_logger::init();
        transport_specs(TcpMock).await.unwrap();
    }
}
