#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ProtocolError(#[from] dns_protocol::Error),

    #[error("DNS lookup canceled, id={0}")]
    LookupCanceled(u16),

    #[error("The DNS packet length is too short.")]
    TooShort,

    #[error("The DNS lookup client is in an invalid state.")]
    InvalidState,

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    AddrParseError(#[from] std::net::AddrParseError),
}

/// Result type returns by APIs in this crate.
pub type Result<T> = std::result::Result<T, Error>;
