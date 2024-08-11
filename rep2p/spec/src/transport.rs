//! test specs for transport layer driver.

use std::net::SocketAddr;

use async_trait::async_trait;
use futures::TryStreamExt;
use rasi::task::spawn_ok;
use rep2p::{multiaddr::Multiaddr, Result, Switch};

use crate::setup;

/// A trait to access context data of the spec test.
#[async_trait]
pub trait TransportSpecContext {
    fn to_multaddr(&self, addr: SocketAddr) -> Multiaddr;

    async fn create_switch(&self, protos: &[&str]) -> Result<Switch>;
}

/// entry point for transport layer tests.
pub async fn transport_specs<C>(cx: C) -> Result<()>
where
    C: TransportSpecContext,
{
    setup();

    open_stream(&cx).await?;

    Ok(())
}

static TRANSPORT_SPEC_PROTOS: &[&str] = ["transport_spec/1.0.0"].as_slice();

async fn open_stream(cx: &dyn TransportSpecContext) -> Result<()> {
    let client = cx.create_switch(TRANSPORT_SPEC_PROTOS).await?;

    let server = cx.create_switch(TRANSPORT_SPEC_PROTOS).await?;

    let peer_addrs = server.local_addrs().await;

    spawn_ok(async move {
        let mut incoming = server.into_incoming();

        while let Some((_stream, _)) = incoming.try_next().await.unwrap() {}
    });

    for raddr in peer_addrs {
        let (_stream, _) = client.connect_to(&raddr, TRANSPORT_SPEC_PROTOS).await?;
    }

    Ok(())
}
