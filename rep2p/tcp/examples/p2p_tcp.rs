use std::time::Duration;

use clap::Parser;
use futures::executor::block_on;
use rasi::timer::sleep;
use rasi_mio::{net::register_mio_network, timer::register_mio_timer};
use rep2p::{multiaddr::Multiaddr, Switch, PROTOCOL_IPFS_PING};
use rep2p_tcp::TcpTransport;

fn clap_parse_multiaddr(s: &str) -> Result<Vec<Multiaddr>, String> {
    let addrs = s
        .split(";")
        .map(|v| Multiaddr::try_from(v))
        .collect::<Result<Vec<Multiaddr>, rep2p::multiaddr::Error>>()
        .map_err(|err| err.to_string())?;

    Ok(addrs)
}

type Multiaddrs = Vec<Multiaddr>;

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = "This is a rp2p-based program to sniff the topology of a libp2p network"
)]
struct Client {
    /// The boostrap route table.
    #[arg(short, long, value_parser = clap_parse_multiaddr, default_value="/ip4/127.0.0.1/tcp/4001")]
    bootstrap: Multiaddrs,

    /// Use verbose output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() {
    register_mio_network();
    register_mio_timer();

    if let Err(err) = block_on(run_client()) {
        log::error!("Sniffier exit with error: {:#?}", err);
    }
}

async fn run_client() -> rep2p::Result<()> {
    let config = Client::parse();

    let level = if config.verbose {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    };

    pretty_env_logger::formatted_timed_builder()
        .filter_level(level)
        .init();

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    let switch = Switch::new(format!("rp2p-{}", VERSION))
        .transport(TcpTransport)
        .create()
        .await?;

    log::info!("Start switch: {}", switch.local_id());

    for raddr in config.bootstrap {
        log::info!("connect to peer: {}", raddr);

        switch.connect_to(&raddr, [PROTOCOL_IPFS_PING]).await?;

        log::info!("connect to peer: {} -- ok", raddr);
    }

    loop {
        sleep(Duration::from_secs(1)).await;
    }
}
