use rasi::task::spawn_ok;

use crate::{transport::ProtocolStream, Result};

use super::{ListenerId, Switch, SwitchEvent};

/// A server-side socket that accept new inbound [`ProtocolStream`]
pub struct ProtocolListener {
    pub(super) id: ListenerId,
    pub(super) switch: Switch,
}

impl Drop for ProtocolListener {
    fn drop(&mut self) {
        let switch = self.switch.clone();
        let id = self.id;
        spawn_ok(async move {
            switch.mutable.lock().await.close_listener(&id);
        })
    }
}

impl ProtocolListener {
    /// Accept a new inbound [`ProtocolStream`].
    ///
    /// On success, returns the negotiated protocol id with the stream.
    pub async fn accept(&self) -> Result<(ProtocolStream, String)> {
        loop {
            if let Some(incoming) = self.switch.mutable.lock().await.incoming_next(&self.id)? {
                return Ok(incoming);
            }

            _ = self
                .switch
                .event_map
                .wait(&SwitchEvent::Accept(self.id), ());
        }
    }

    /// Conver the switch into a [`Stream`](futures::Stream) object.
    pub fn into_incoming(
        self,
    ) -> impl futures::Stream<Item = Result<(ProtocolStream, String)>> + Unpin {
        Box::pin(futures::stream::unfold(self, |listener| async move {
            let res = listener.accept().await;
            Some((res, listener))
        }))
    }
}
