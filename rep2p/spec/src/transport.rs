//! test specs for transport layer driver.

use async_trait::async_trait;
use futures::{AsyncReadExt, AsyncWriteExt, TryStreamExt};
use rasi::task::spawn_ok;
use rep2p::{
    serve::{
        syscall::{DriverProtocol, DriverProtocolHandler},
        ProtocolHandler, ServeMux,
    },
    transport::Stream,
    Result, Switch, SwitchBuilder,
};

use crate::setup;

/// A trait to access context data of the spec test.
#[async_trait]
pub trait TransportSpecContext {
    async fn create_switch(&self) -> Result<SwitchBuilder>;
}

/// entry point for transport layer tests.
pub async fn transport_specs<C>(cx: C) -> Result<()>
where
    C: TransportSpecContext,
{
    setup();

    // stream_ping_pong(&cx).await?;

    serve_mux(&cx).await?;

    Ok(())
}

static TRANSPORT_SPEC_PROTOS: &[&str] = ["/transport_spec/1.0.0"].as_slice();

#[allow(unused)]
async fn stream_ping_pong(cx: &dyn TransportSpecContext) -> Result<()> {
    let client = cx
        .create_switch()
        .await?
        .protos(TRANSPORT_SPEC_PROTOS)
        .create()
        .await?;

    let server = cx
        .create_switch()
        .await?
        .protos(TRANSPORT_SPEC_PROTOS)
        .create()
        .await?;

    let peer_addrs = server.local_addrs().await;

    spawn_ok(async move {
        let mut incoming = server.into_incoming();

        while let Some((mut stream, _)) = incoming.try_next().await.unwrap() {
            let mut buf = vec![0; 256];

            let read_size = stream.read(&mut buf).await.unwrap();

            stream.write(&buf[..read_size]).await.unwrap();
        }
    });

    for raddr in peer_addrs {
        for _ in 0..100 {
            let (mut stream, _) = client.connect(&raddr, TRANSPORT_SPEC_PROTOS).await?;

            stream.write_all(b"hello libp2p").await.unwrap();

            let mut buf = vec![0; 256];

            let read_size = stream.read(&mut buf).await.unwrap();

            assert_eq!(&buf[..read_size], b"hello libp2p");
        }
    }

    Ok(())
}

struct PingPong;

#[async_trait]
impl DriverProtocol for PingPong {
    /// Returns protocol display name.
    fn protos(&self) -> &'static [&'static str] {
        TRANSPORT_SPEC_PROTOS
    }

    async fn create(&self, _switch: &Switch) -> std::io::Result<ProtocolHandler> {
        Ok(PingPongHandler.into())
    }
}

struct PingPongHandler;

#[async_trait]
impl DriverProtocolHandler for PingPongHandler {
    async fn dispatch(&self, _: &str, mut stream: Stream) -> std::io::Result<()> {
        let mut buf = vec![0; 256];

        let read_size = stream.read(&mut buf).await?;

        stream.write(&buf[..read_size]).await?;

        Ok(())
    }
}

async fn serve_mux(cx: &dyn TransportSpecContext) -> Result<()> {
    let client = cx
        .create_switch()
        .await?
        .protos(TRANSPORT_SPEC_PROTOS)
        .create()
        .await?;

    let server = cx.create_switch().await?;

    let server = ServeMux::new().handle(PingPong).create(server).await?;

    let peer_addrs = server.local_addrs().await;

    for raddr in peer_addrs {
        for _ in 0..200 {
            let (mut stream, _) = client.connect(&raddr, TRANSPORT_SPEC_PROTOS).await?;

            stream.write_all(b"hello libp2p").await.unwrap();

            let mut buf = vec![0; 256];

            let read_size = stream.read(&mut buf).await.unwrap();

            assert_eq!(&buf[..read_size], b"hello libp2p");
        }

        break;
    }

    Ok(())
}
