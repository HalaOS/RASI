/// protocol name of libp2p identity
pub const PROTOCOL_IPFS_ID: &str = "/ipfs/id/1.0.0";

/// protocol name of libp2p identity push
pub const PROTOCOL_IPFS_PUSH_ID: &str = "/ipfs/id/push/1.0.0";

/// protocol name of libp2p ping
pub const PROTOCOL_IPFS_PING: &str = "/ipfs/ping/1.0.0";

/// Variant type used by [`connect`](Switch::connect) function.
pub enum ConnectTo<'a> {
    PeerIdRef(&'a PeerId),
    MultiaddrRef(&'a Multiaddr),
    PeerId(PeerId),
    Multiaddr(Multiaddr),
}

impl<'a> From<&'a PeerId> for ConnectTo<'a> {
    fn from(value: &'a PeerId) -> Self {
        Self::PeerIdRef(value)
    }
}

impl<'a> From<&'a Multiaddr> for ConnectTo<'a> {
    fn from(value: &'a Multiaddr) -> Self {
        Self::MultiaddrRef(value)
    }
}

impl From<PeerId> for ConnectTo<'static> {
    fn from(value: PeerId) -> Self {
        Self::PeerId(value)
    }
}

impl From<Multiaddr> for ConnectTo<'static> {
    fn from(value: Multiaddr) -> Self {
        Self::Multiaddr(value)
    }
}

impl TryFrom<&str> for ConnectTo<'static> {
    type Error = Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        if let Ok(peer_id) = value.parse::<PeerId>() {
            return Ok(Self::PeerId(peer_id));
        }

        return Ok(Self::Multiaddr(value.parse::<Multiaddr>()?));
    }
}
