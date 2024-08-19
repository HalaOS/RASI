use rep2p::multiaddr::{self, Multiaddr};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Multiaddr without p2p node, {0}")]
    InvalidSeedMultAddr(Multiaddr),

    #[error(transparent)]
    SwitchError(#[from] rep2p::Error),

    #[error(transparent)]
    IoErr(#[from] std::io::Error),

    #[error(transparent)]
    ProtocolError(#[from] protobuf::Error),

    #[error(transparent)]
    ReadError(#[from] unsigned_varint::io::ReadError),

    #[error("The rpc response length is greater than {0}")]
    ResponeLength(usize),

    #[error("Invalid find_node response type: {0}")]
    InvalidFindNodeResponse(String),

    #[error(transparent)]
    ParseError(#[from] identity::ParseError),

    #[error(transparent)]
    MultiaddrError(#[from] multiaddr::Error),

    #[error("Rpc timeout.")]
    Timeout,

    #[error("{0}")]
    Other(String),
}

/// Result type returns by this module functionss.
pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for std::io::Error {
    fn from(value: Error) -> Self {
        match value {
            Error::IoErr(err) => err,
            err => std::io::Error::new(std::io::ErrorKind::Other, err),
        }
    }
}
