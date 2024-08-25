//! This module provides a asynchronously DNS client implementation.

use std::{
    collections::VecDeque,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::{
        atomic::{AtomicBool, AtomicU16, Ordering},
        Arc,
    },
};

use dns_protocol::{Flags, Message, Question, ResourceRecord};
use futures::lock::Mutex;
use futures_map::KeyWaitMap;

use crate::errors::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LookupEvent {
    Send,
    Response(u16),
}

enum LookupEventArg {
    Send,
    Response(Vec<u8>),
}

#[derive(Default)]
pub(crate) struct RawDnsLookup {
    pub(crate) send_closed: AtomicBool,
    pub(crate) recv_closed: AtomicBool,
    is_closed: AtomicBool,
    idgen: AtomicU16,
    sending: Mutex<VecDeque<Vec<u8>>>,
    waiters: KeyWaitMap<LookupEvent, LookupEventArg>,
}

/// A DNS client type without [`Drop`] support.
/// you should manually call the [`close`](DnsLookupWithoutDrop::close) function to cleanup resources.
///
/// Usually this type is used by background io tasks, the end-users should use [`DnsLookup`] instead.
#[derive(Default, Clone)]
pub struct DnsLookupWithoutDrop(pub(crate) Arc<RawDnsLookup>);

impl DnsLookupWithoutDrop {
    /// Returns true if this client is closed.
    pub fn is_closed(&self) -> bool {
        self.0.is_closed.load(Ordering::SeqCst)
    }

    /// Writes a single DNS packet to be sent to the server.
    pub async fn send(&self) -> Result<Vec<u8>> {
        loop {
            if let Some(buf) = self.0.sending.lock().await.pop_front() {
                return Ok(buf);
            }

            if self.0.is_closed.load(Ordering::SeqCst) {
                return Err(Error::InvalidState);
            }

            self.0.waiters.wait(&LookupEvent::Send, ()).await;
        }
    }

    /// Processes DNS packet received from the peer.
    pub async fn recv<Buf>(&self, buf: Buf) -> Result<()>
    where
        Buf: AsRef<[u8]>,
    {
        if self.0.is_closed.load(Ordering::SeqCst) {
            return Err(Error::InvalidState);
        }

        // Incomplete packet.
        if buf.as_ref().len() < 12 {
            return Err(Error::TooShort);
        }

        let mut id_buf = [0; 2];

        id_buf.copy_from_slice(&buf.as_ref()[..2]);

        let id = u16::from_be_bytes(id_buf);

        self.0.waiters.insert(
            LookupEvent::Response(id),
            LookupEventArg::Response(buf.as_ref().to_vec()),
        );

        Ok(())
    }

    /// Close this client
    pub fn close(&self) {
        self.0.is_closed.store(true, Ordering::SeqCst);

        self.0
            .waiters
            .insert(LookupEvent::Send, LookupEventArg::Send);
    }

    pub(crate) fn close_send(&self) {
        self.close();
        self.0.send_closed.store(true, Ordering::SeqCst);

        self.0
            .waiters
            .insert(LookupEvent::Send, LookupEventArg::Send);
    }

    pub(crate) fn close_recv(&self) {
        self.close();
        self.0.recv_closed.store(true, Ordering::SeqCst);

        self.0
            .waiters
            .insert(LookupEvent::Send, LookupEventArg::Send);
    }
}

/// A asynchronous DNs client.
#[derive(Default)]
pub struct DnsLookup(DnsLookupWithoutDrop);

impl Drop for DnsLookup {
    fn drop(&mut self) {
        self.0.close();
    }
}

impl DnsLookup {
    /// Get the innner [`DnsLookupWithoutDrop`] instance.
    pub fn to_inner(&self) -> DnsLookupWithoutDrop {
        self.0.clone()
    }
    /// Lookup ipv6 records.
    pub async fn lookup_ipv6<'a, N>(&self, label: N) -> Result<Vec<Ipv6Addr>>
    where
        N: AsRef<str>,
    {
        self.lookup_ip(label).await.map(|addrs| {
            addrs
                .into_iter()
                .filter_map(|addr| match addr {
                    IpAddr::V6(addr) => Some(addr),
                    IpAddr::V4(_) => None,
                })
                .collect()
        })
    }

    /// Lookup ipv4 records.
    pub async fn lookup_ipv4<'a, N>(&self, label: N) -> Result<Vec<Ipv4Addr>>
    where
        N: AsRef<str>,
    {
        self.lookup_ip(label).await.map(|addrs| {
            addrs
                .into_iter()
                .filter_map(|addr| match addr {
                    IpAddr::V4(addr) => Some(addr),
                    IpAddr::V6(_) => None,
                })
                .collect()
        })
    }
    /// Lookup ipv4/ipv6 records.
    pub async fn lookup_ip<'a, N>(&self, label: N) -> Result<Vec<IpAddr>>
    where
        N: AsRef<str>,
    {
        let id = self.0 .0.idgen.fetch_add(1, Ordering::Relaxed);

        let mut questions = [
            Question::new(label.as_ref(), dns_protocol::ResourceType::A, 1),
            Question::new(label.as_ref(), dns_protocol::ResourceType::AAAA, 1),
        ];

        let message = Message::new(
            id,
            Flags::standard_query(),
            &mut questions,
            &mut [],
            &mut [],
            &mut [],
        );

        let mut buf = vec![0; message.space_needed()];

        message.write(&mut buf)?;

        self.0 .0.sending.lock().await.push_back(buf);

        self.0
             .0
            .waiters
            .insert(LookupEvent::Send, LookupEventArg::Send);

        if let Some(LookupEventArg::Response(buf)) =
            self.0 .0.waiters.wait(&LookupEvent::Response(id), ()).await
        {
            let mut answers = [ResourceRecord::default(); 16];
            let mut authority = [ResourceRecord::default(); 16];
            let mut additional = [ResourceRecord::default(); 16];

            let message = Message::read(
                &buf,
                &mut questions,
                &mut answers,
                &mut authority,
                &mut additional,
            )?;

            let mut group = vec![];

            for answer in message.answers() {
                // Determine the IP address.
                match answer.data().len() {
                    4 => {
                        let mut ip = [0u8; 4];
                        ip.copy_from_slice(answer.data());
                        let ip = Ipv4Addr::from(ip);
                        log::trace!("{} has address {}", answer.name(), ip);
                        group.push(ip.into());
                    }
                    16 => {
                        let mut ip = [0u8; 16];
                        ip.copy_from_slice(answer.data());
                        let ip = IpAddr::from(ip);
                        log::trace!("{} has address {}", answer.name(), ip);
                        group.push(ip.into());
                    }
                    _ => {
                        log::trace!("{} has unknown address type", answer.name());
                    }
                }
            }

            Ok(group)
        } else {
            Err(Error::LookupCanceled(id))
        }
    }
}
