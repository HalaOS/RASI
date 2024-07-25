use std::io;

pub(crate) fn map_quic_error(error: quiche::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error)
}
