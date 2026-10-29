// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The state the world's nodes share: vpn ports, the hash table and swarm
//! logs.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::future::poll_fn;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use futures_sink::Sink;
use strims_kademlia::ID;
use strims_ppspp::SwarmId;
use strims_rpc::Timer;
use strims_transfer::{ReadFailure, SwarmRead, SwarmWrite};
use strims_vpn::{Delivery, HostAddr, M_ENCRYPT, NetworkId};
use tokio::time::Instant;

use crate::exec::TokioTimer;
use crate::pipe::Closed;

#[derive(Default)]
pub struct PortState {
    queue: VecDeque<Delivery<u16>>,
    reader: Option<Waker>,
}

pub type Port = Rc<RefCell<PortState>>;

/// A swarm's stream, shared by every node's swarm with its id and label.
#[derive(Default)]
pub struct LogState {
    pub epoch: u64,
    /// The last `window` bytes of the stream, from offset `tail`.
    pub data: VecDeque<u8>,
    pub tail: u64,
    pub window: usize,
    /// Bytes written in all epochs.
    pub written: u64,
    wakers: Vec<Waker>,
}

impl LogState {
    pub(crate) fn take_wakers(&mut self) -> Vec<Waker> {
        std::mem::take(&mut self.wakers)
    }

    /// Starts a new epoch at offset 0, returning the waiting readers' wakers.
    pub(crate) fn new_epoch(&mut self) -> Vec<Waker> {
        self.epoch += 1;
        self.data.clear();
        self.tail = 0;
        self.take_wakers()
    }
}

pub type Log = Rc<RefCell<LogState>>;

pub(crate) fn wake_all(wakers: Vec<Waker>) {
    for w in wakers {
        w.wake();
    }
}

/// Every node's vpn ports, by (host, network key, port); the hash table's
/// service addresses, by (key, salt), one table for all networks; and the
/// swarm logs, by (swarm id, label).
#[derive(Default)]
pub struct WorldState {
    pub ports: BTreeMap<(ID, Vec<u8>, u16), Port>,
    pub next_port: u16,
    pub addresses: BTreeMap<(Vec<u8>, Vec<u8>), HostAddr>,
    pub logs: BTreeMap<(SwarmId, String), Log>,
    /// The instant every node's clock reads as [`T0`](crate::T0).
    base: Option<Instant>,
}

#[derive(Clone, Default)]
pub struct World(pub Rc<RefCell<WorldState>>);

impl World {
    /// The world's clock: nodes started later read the same time.
    pub fn timer(&self) -> TokioTimer {
        let base = *self.0.borrow_mut().base.get_or_insert_with(Instant::now);
        TokioTimer { base }
    }

    pub(crate) fn log(&self, id: &SwarmId, label: &str, window: usize) -> Log {
        let mut w = self.0.borrow_mut();
        w.logs
            .entry((id.clone(), label.to_owned()))
            .or_insert_with(|| {
                Rc::new(RefCell::new(LogState {
                    window,
                    ..LogState::default()
                }))
            })
            .clone()
    }

    /// The address the hash table holds under (`key`, `salt`).
    pub fn address(&self, key: &[u8], salt: &[u8]) -> Option<HostAddr> {
        let k = (key.to_vec(), salt.to_vec());
        self.0.borrow().addresses.get(&k).copied()
    }

    /// The directory address published under `key`.
    pub fn directory_address(&self, key: &[u8]) -> Option<HostAddr> {
        self.address(key, strims_directory::ADDRESS_SALT)
    }

    /// Bytes written to the log of `id` under `label` in all epochs.
    pub fn written(&self, id: &SwarmId, label: &str) -> u64 {
        let w = self.0.borrow();
        w.logs
            .get(&(id.clone(), label.to_owned()))
            .map_or(0, |l| l.borrow().written)
    }

    /// Opens `port` of `host` on the network with key `network`, failing
    /// while it is open.
    pub(crate) fn open_port(
        &self,
        host: ID,
        network: Vec<u8>,
        port: u16,
        timer: TokioTimer,
    ) -> Result<(VpnReader, VpnWriter), Closed> {
        let key = (host, network.clone(), port);
        let state = Port::default();
        {
            let mut w = self.0.borrow_mut();
            if w.ports.contains_key(&key) {
                return Err(Closed);
            }
            w.ports.insert(key.clone(), state.clone());
        }
        let writer = VpnWriter {
            world: self.clone(),
            src: host,
            src_port: port,
            network,
            timer,
        };
        let reader = VpnReader {
            port: state,
            world: self.clone(),
            key,
        };
        Ok((reader, writer))
    }
}

/// A port's deliveries; dropping it closes the port.
pub struct VpnReader {
    port: Port,
    world: World,
    key: (ID, Vec<u8>, u16),
}

impl Stream for VpnReader {
    type Item = Result<Delivery<u16>, Closed>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut p = self.port.borrow_mut();
        if let Some(d) = p.queue.pop_front() {
            return Poll::Ready(Some(Ok(d)));
        }
        p.reader = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Drop for VpnReader {
    fn drop(&mut self) {
        let removed = {
            let mut w = self.world.0.borrow_mut();
            match w.ports.get(&self.key) {
                Some(p) if Rc::ptr_eq(p, &self.port) => w.ports.remove(&self.key),
                _ => None,
            }
        };
        drop(removed);
    }
}

/// A port's sends: each reaches the destination port at once if it is
/// open on the same network, and is dropped otherwise.
pub struct VpnWriter {
    world: World,
    src: ID,
    src_port: u16,
    network: Vec<u8>,
    timer: TokioTimer,
}

impl Sink<(HostAddr, Vec<u8>)> for VpnWriter {
    type Error = Closed;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, (to, body): (HostAddr, Vec<u8>)) -> Result<(), Closed> {
        let port = self
            .world
            .0
            .borrow()
            .ports
            .get(&(to.host, self.network.clone(), to.port))
            .cloned();
        let Some(port) = port else {
            return Ok(());
        };
        let w = {
            let mut p = port.borrow_mut();
            p.queue.push_back(Delivery {
                network: NetworkId::default(),
                handler: to.port,
                src: self.src,
                src_port: self.src_port,
                dst: to.host,
                dst_port: to.port,
                flags: M_ENCRYPT,
                hops: 0,
                verified: None,
                body,
                received: self.timer.now(),
            });
            p.reader.take()
        };
        if let Some(w) = w {
            w.wake();
        }
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Poll::Ready(Ok(()))
    }
}

/// A swarm's writer.
pub struct FakeWriter {
    pub(crate) log: Log,
    pub(crate) closed: Rc<Cell<bool>>,
    /// The swarm's integrity unit in bytes, which a flush pads to.
    pub(crate) unit: u64,
}

impl SwarmWrite for FakeWriter {
    type Error = Closed;

    async fn write(&mut self, data: &[u8]) -> Result<(), Closed> {
        if self.closed.get() {
            return Err(Closed);
        }
        let wakers = {
            let mut l = self.log.borrow_mut();
            l.data.extend(data);
            l.written += data.len() as u64;
            let excess = l.data.len().saturating_sub(l.window);
            l.data.drain(..excess);
            l.tail += excess as u64;
            l.take_wakers()
        };
        wake_all(wakers);
        Ok(())
    }

    async fn reset(&mut self) -> Result<(), Closed> {
        if self.closed.get() {
            return Err(Closed);
        }
        let wakers = self.log.borrow_mut().new_epoch();
        wake_all(wakers);
        Ok(())
    }

    /// Pads the stream with zeros to the next unit boundary, as a swarm
    /// writer's flush does, and returns its offset.
    async fn flush(&mut self) -> Result<u64, Closed> {
        if self.closed.get() {
            return Err(Closed);
        }
        let (off, wakers) = {
            let mut l = self.log.borrow_mut();
            let at = l.tail + l.data.len() as u64;
            let unit = self.unit.max(1);
            let pad = (unit - at % unit) % unit;
            l.data.extend(std::iter::repeat_n(0, pad as usize));
            l.written += pad;
            let excess = l.data.len().saturating_sub(l.window);
            l.data.drain(..excess);
            l.tail += excess as u64;
            (at + pad, l.take_wakers())
        };
        wake_all(wakers);
        Ok(off)
    }
}

/// A swarm's reader.
pub struct FakeReader {
    pub(crate) log: Log,
    pub(crate) closed: Rc<Cell<bool>>,
    pub(crate) epoch: u64,
    pub(crate) pos: u64,
}

impl SwarmRead for FakeReader {
    type Error = Closed;

    async fn read_at(&mut self, out: &mut [u8]) -> Result<(u64, usize), ReadFailure<Closed>> {
        poll_fn(|cx| {
            if self.closed.get() {
                return Poll::Ready(Err(ReadFailure::Closed));
            }
            let mut l = self.log.borrow_mut();
            if l.epoch != self.epoch || self.pos < l.tail {
                self.epoch = l.epoch;
                self.pos = l.tail;
                return Poll::Ready(Err(ReadFailure::Discontinuity));
            }
            let end = l.tail + l.data.len() as u64;
            if self.pos >= end {
                l.wakers.push(cx.waker().clone());
                return Poll::Pending;
            }
            let start = (self.pos - l.tail) as usize;
            let n = out.len().min((end - self.pos) as usize);
            for (o, b) in out.iter_mut().zip(l.data.range(start..start + n)) {
                *o = *b;
            }
            let at = self.pos;
            self.pos += n as u64;
            Poll::Ready(Ok((at, n)))
        })
        .await
    }
}
