use std::future::Future;

use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use identity::PeerId;
use protobuf::Message;

use rep2p::multiaddr::Multiaddr;

use crate::{
    errors::{Error, Result},
    primitives::PeerInfo,
    proto::rpc,
};

#[allow(unused)]
/// An extension to add kad rpc functions to [`AsyncWrite`] + [`AsyncRead`]
pub(crate) trait KadRpc: AsyncWrite + AsyncRead + Unpin {
    fn find_node(
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

            if message.type_ != rpc::message::MessageType::FIND_NODE.into() {
                return Err(Error::InvalidFindNodeResponse(format!(
                    "{:?}",
                    message.type_
                )));
            }

            let mut peers = vec![];

            for peer in message.closerPeers {
                let mut addrs = vec![];

                for addr in peer.addrs {
                    addrs.push(Multiaddr::try_from(addr)?);
                }

                peers.push(PeerInfo {
                    id: PeerId::from_bytes(&peer.id)?,
                    addrs,
                    conn_type: peer.connection.enum_value_or_default(),
                });
            }

            Ok(peers)
        }
    }
}

impl<T> KadRpc for T where T: AsyncWrite + AsyncRead + Unpin {}
