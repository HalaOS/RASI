use std::io;

use crate::future::event_map::EventStatus;

pub(crate) fn map_quic_error(error: quiche::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error)
}

pub(crate) fn map_event_map_error(error: EventStatus) -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, format!("{:?}", error))
}
