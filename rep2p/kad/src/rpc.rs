use std::future::Future;

use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use identity::PeerId;
use protobuf::Message;

use rep2p::{book::PeerInfo, multiaddr::Multiaddr};

use crate::{
    errors::{Error, Result},
    proto::{self, rpc},
};

#[allow(unused)]
/// An extension to add kad rpc functions to [`AsyncWrite`] + [`AsyncRead`]
pub(crate) trait KadRpc: AsyncWrite + AsyncRead + Unpin {
    /// Invoke a libp2p kad rpc call.
    fn kad_rpc_call(
        mut self,
        message: rpc::Message,
        max_recv_len: usize,
    ) -> impl Future<Output = Result<rpc::Message>>
    where
        Self: Sized,
    {
        async move {
            let buf = message.write_to_bytes()?;

            let mut payload_len = unsigned_varint::encode::usize_buffer();

            self.write_all(unsigned_varint::encode::usize(buf.len(), &mut payload_len))
                .await?;

            self.write_all(buf.as_slice()).await?;

            let body_len = unsigned_varint::aio::read_usize(&mut self).await?;

            if body_len > max_recv_len {
                return Err(Error::ResponeLength(max_recv_len));
            }

            let mut buf = vec![0u8; body_len];

            self.read_exact(&mut buf).await?;

            let message = rpc::Message::parse_from_bytes(&buf)?;

            Ok(message)
        }
    }

    /// invoke find_node call.
    fn kad_find_node(
        mut self,
        peer_id: &PeerId,
        max_recv_len: usize,
    ) -> impl Future<Output = Result<Vec<PeerInfo>>>
    where
        Self: Sized,
    {
        let mut message = rpc::Message::new();

        message.type_ = rpc::message::MessageType::FIND_NODE.into();
        message.key = peer_id.to_bytes();

        async move {
            let message = self.kad_rpc_call(message, max_recv_len).await?;

            let mut peers = vec![];

            for peer in message.closerPeers {
                let mut addrs = vec![];

                for addr in peer.addrs {
                    addrs.push(Multiaddr::try_from(addr)?);
                }

                peers.push(PeerInfo {
                    id: PeerId::from_bytes(&peer.id)?,
                    addrs,
                    conn_type: peer.connection.enum_value_or_default().into(),
                });
            }

            Ok(peers)
        }
    }
}

impl<T> KadRpc for T where T: AsyncWrite + AsyncRead + Unpin {}

impl From<proto::rpc::message::ConnectionType> for rep2p::book::ConnectionType {
    fn from(value: proto::rpc::message::ConnectionType) -> Self {
        match value {
            rpc::message::ConnectionType::NOT_CONNECTED => Self::NotConnected,
            rpc::message::ConnectionType::CONNECTED => Self::Connected,
            rpc::message::ConnectionType::CAN_CONNECT => Self::CanConnect,
            rpc::message::ConnectionType::CANNOT_CONNECT => Self::CannotConnect,
        }
    }
}

impl From<rep2p::book::ConnectionType> for proto::rpc::message::ConnectionType {
    fn from(value: rep2p::book::ConnectionType) -> Self {
        match value {
            rep2p::book::ConnectionType::NotConnected => Self::NOT_CONNECTED,
            rep2p::book::ConnectionType::Connected => Self::CONNECTED,
            rep2p::book::ConnectionType::CanConnect => Self::CAN_CONNECT,
            rep2p::book::ConnectionType::CannotConnect => Self::CANNOT_CONNECT,
        }
    }
}
