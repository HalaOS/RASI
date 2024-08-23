#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    SwitchError(#[from] rep2p::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

/// Result type returns by this module functionss.
pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for std::io::Error {
    fn from(value: Error) -> Self {
        match value {
            Error::IoError(err) => err,
            Error::SwitchError(rep2p::Error::IoError(err)) => err,
            err => std::io::Error::new(std::io::ErrorKind::Other, err),
        }
    }
}
