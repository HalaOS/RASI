use std::string::FromUtf8Error;

use dns_parser::ResponseCode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // #[error(transparent)]
    // ProtocolError(#[from] dns_protocol::Error),
    #[error("DNS lookup canceled, id={0}")]
    LookupCanceled(u16),

    #[error("The DNS packet length is too short.")]
    TooShort,

    #[error("The DNS packet is truncated.")]
    Truncated,

    #[error("The DNS lookup client is in an invalid state.")]
    InvalidState,

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    AddrParseError(#[from] std::net::AddrParseError),

    #[error(transparent)]
    FromUtf8Error(#[from] FromUtf8Error),

    #[error(transparent)]
    DnsParse(#[from] dns_parser::Error),

    #[error("DNS server report, err={0}")]
    ServerError(ResponseCode),

    #[cfg(all(unix, feature = "sysconf"))]
    #[error(transparent)]
    ResolvConf(#[from] resolv_conf::ParseError),

    #[error("Unable load sys-wide nameserver")]
    SysWideNameServer,

    #[cfg(all(windows, feature = "sysconf"))]
    #[error(transparent)]
    IpConfigError(#[from] ipconfig::error::Error),
}

/// Result type returns by APIs in this crate.
pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for std::io::Error {
    fn from(value: Error) -> Self {
        match value {
            Error::IoError(error) => error,
            _ => std::io::Error::new(std::io::ErrorKind::Other, value),
        }
    }
}
