// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A fake node's host: the network, transfer and directory hosts over the
//! world.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use strims_kademlia::ID;
use strims_ppspp::integrity::ProtectionMethod;
use strims_ppspp::{SwarmId, SwarmOptions, WriterOptions};
use strims_proto::types::Key;
use strims_vpn::HostAddr;

use crate::exec::TokioTimer;
use crate::pipe::{Closed, Reader, Writer};
use crate::world::{FakeReader, FakeWriter, Log, VpnReader, VpnWriter, World, wake_all};

pub struct SwarmEntry {
    pub id: SwarmId,
    pub label: String,
    /// Whether this node created (writes) the swarm.
    pub created: bool,
    log: Log,
    closed: Rc<Cell<bool>>,
}

#[derive(Default)]
pub struct NodeState {
    /// Network key -> handle.
    pub networks: BTreeMap<Vec<u8>, u32>,
    pub next_net: u32,
    pub next_port: u16,
    /// (peer, network) -> (local port, peer port).
    pub bound: BTreeMap<(ID, u32), (u16, u16)>,
    /// Each peer's broker channel, until taken.
    pub brokers: BTreeMap<ID, (Reader, Writer)>,
    pub swarms: BTreeMap<u32, SwarmEntry>,
    pub next_swarm: u32,
    /// Ports opened by number under another number.
    pub port_remap: BTreeMap<u16, u16>,
    /// Readers opened from now start at the newest chunk ([`FakeHost::open_readers_live`]).
    pub live_readers: bool,
}

#[derive(Clone)]
pub struct FakeHost {
    pub id: ID,
    pub state: Rc<RefCell<NodeState>>,
    pub world: World,
    pub timer: TokioTimer,
}

impl FakeHost {
    pub fn network_key(&self, net: u32) -> Option<Vec<u8>> {
        let s = self.state.borrow();
        s.networks
            .iter()
            .find(|(_, n)| **n == net)
            .map(|(k, _)| k.clone())
    }

    pub fn net(&self, key: &[u8]) -> Option<u32> {
        self.state.borrow().networks.get(key).copied()
    }

    /// With `live`, the readers this node opens from now start at the start
    /// of the newest chunk, once the log holds a byte, as a low-latency
    /// swarm reader joins a live stream (and can move back with
    /// [`SwarmRead::rewind`](strims_transfer::SwarmRead::rewind)); without,
    /// at the window's tail.
    pub fn open_readers_live(&self, live: bool) {
        self.state.borrow_mut().live_readers = live;
    }

    /// Opens `port` on the network, whatever `port_remap` says.
    pub fn open_port(&self, net: u32, port: u16) -> Result<(VpnReader, VpnWriter), Closed> {
        let network = self.network_key(net).ok_or(Closed)?;
        self.world
            .open_port(self.id, network, port, self.timer.clone())
    }

    /// The counts of this node's swarms with id `id` it created and added.
    pub fn swarms_of(&self, id: &SwarmId) -> (usize, usize) {
        let s = self.state.borrow();
        let of = s.swarms.values().filter(|e| e.id == *id);
        of.fold(
            (0, 0),
            |(c, a), e| {
                if e.created { (c + 1, a) } else { (c, a + 1) }
            },
        )
    }

    fn add_entry(
        &self,
        id: SwarmId,
        options: &SwarmOptions,
        created: bool,
    ) -> (u32, Log, Rc<Cell<bool>>) {
        let window = options.live_window * options.chunk_size;
        let chunk_size = options
            .clone()
            .assign_defaults(&SwarmOptions::default_options())
            .chunk_size as u64;
        let log = self.world.log(&id, &options.label, window, chunk_size);
        let closed = Rc::new(Cell::new(false));
        let mut s = self.state.borrow_mut();
        s.next_swarm += 1;
        let h = s.next_swarm;
        s.swarms.insert(
            h,
            SwarmEntry {
                id,
                label: options.label.clone(),
                created,
                log: log.clone(),
                closed: closed.clone(),
            },
        );
        (h, log, closed)
    }
}

impl strims_network::Host for FakeHost {
    type Network = u32;
    type BrokerReader = Reader;
    type BrokerWriter = Writer;
    type VpnReader = VpnReader;
    type VpnWriter = VpnWriter;
    type Publication = (Vec<u8>, Vec<u8>);
    type Error = Closed;

    fn host_id(&self) -> ID {
        self.id
    }

    async fn add_network(&self, key: &[u8]) -> Result<u32, Closed> {
        let mut s = self.state.borrow_mut();
        s.next_net += 1;
        let net = s.next_net;
        s.networks.insert(key.to_vec(), net);
        Ok(net)
    }

    async fn remove_network(&self, net: u32) -> Result<(), Closed> {
        let mut s = self.state.borrow_mut();
        s.networks.retain(|_, n| *n != net);
        s.bound.retain(|(_, n), _| *n != net);
        Ok(())
    }

    async fn take_broker(&self, host: ID) -> Result<(Reader, Writer), Closed> {
        self.state.borrow_mut().brokers.remove(&host).ok_or(Closed)
    }

    async fn reserve_peer_port(&self, _: ID) -> Result<u16, Closed> {
        let mut s = self.state.borrow_mut();
        s.next_port += 1;
        Ok(1000 + s.next_port)
    }

    async fn release_peer_port(&self, _: ID, _: u16) -> Result<(), Closed> {
        Ok(())
    }

    async fn bind_peer(
        &self,
        host: ID,
        net: u32,
        local_port: u16,
        peer_port: u16,
    ) -> Result<(), Closed> {
        self.state
            .borrow_mut()
            .bound
            .insert((host, net), (local_port, peer_port));
        Ok(())
    }

    async fn unbind_peer(&self, host: ID, net: u32) -> Result<(), Closed> {
        self.state.borrow_mut().bound.remove(&(host, net));
        Ok(())
    }

    async fn open_vpn_port(&self, net: u32) -> Result<(u16, VpnReader, VpnWriter), Closed> {
        let port = {
            let mut w = self.world.0.borrow_mut();
            w.next_port += 1;
            1000 + w.next_port
        };
        let (reader, writer) = self.open_port(net, port)?;
        Ok((port, reader, writer))
    }

    async fn publish_address(
        &self,
        _: u32,
        key: Key,
        salt: Vec<u8>,
        port: u16,
    ) -> Result<(Vec<u8>, Vec<u8>), Closed> {
        let k = (key.public, salt);
        self.world
            .0
            .borrow_mut()
            .addresses
            .insert(k.clone(), HostAddr::new(self.id, port));
        Ok(k)
    }

    async fn unpublish_address(&self, k: (Vec<u8>, Vec<u8>)) -> Result<(), Closed> {
        let mut w = self.world.0.borrow_mut();
        if w.addresses.get(&k).is_some_and(|a| a.host == self.id) {
            w.addresses.remove(&k);
        }
        Ok(())
    }

    async fn resolve_address(
        &self,
        _: u32,
        key: Vec<u8>,
        salt: Vec<u8>,
    ) -> Result<Option<HostAddr>, Closed> {
        Ok(self.world.address(&key, &salt))
    }
}

impl strims_transfer::Host for FakeHost {
    type Network = u32;
    type Swarm = u32;
    type Publication = ();
    type Search = futures_util::stream::Empty<ID>;
    type Error = Closed;

    fn host_id(&self) -> ID {
        self.id
    }

    async fn run_swarm(&self, _: ID, _: u32, _: u64, _: u64) -> Result<(), Closed> {
        Ok(())
    }

    async fn stop_swarm(&self, _: ID, _: u32) -> Result<(), Closed> {
        Ok(())
    }

    async fn index_publish(&self, _: u32, _: Vec<u8>, _: Vec<u8>) -> Result<(), Closed> {
        Ok(())
    }

    async fn index_unpublish(&self, _: ()) -> Result<(), Closed> {
        Ok(())
    }

    async fn index_search(
        &self,
        _: u32,
        _: Vec<u8>,
        _: Vec<u8>,
    ) -> Result<futures_util::stream::Empty<ID>, Closed> {
        Ok(futures_util::stream::empty())
    }

    async fn connect_peer(&self, _: u32, _: ID) -> Result<bool, Closed> {
        Ok(false)
    }
}

impl strims_transfer::SwarmHost for FakeHost {
    type Writer = FakeWriter;
    type Reader = FakeReader;

    async fn create_swarm(
        &self,
        options: WriterOptions,
    ) -> Result<(u32, SwarmId, FakeWriter), Closed> {
        let id = SwarmId::new(&options.key.public);
        let (h, log, closed) = self.add_entry(id.clone(), &options.swarm_options, true);
        let wakers = log.borrow_mut().new_epoch();
        wake_all(wakers);
        let o = options
            .swarm_options
            .assign_defaults(&SwarmOptions::default_options());
        let unit = match o.integrity.protection_method {
            ProtectionMethod::MERKLE_TREE => o.chunk_size * o.chunks_per_signature,
            _ => o.chunk_size,
        };
        Ok((
            h,
            id,
            FakeWriter {
                log,
                closed,
                unit: unit as u64,
            },
        ))
    }

    async fn add_swarm(&self, id: SwarmId, options: SwarmOptions) -> Result<u32, Closed> {
        Ok(self.add_entry(id, &options, false).0)
    }

    async fn remove_swarm(&self, swarm: u32) -> Result<(), Closed> {
        let e = self
            .state
            .borrow_mut()
            .swarms
            .remove(&swarm)
            .ok_or(Closed)?;
        e.closed.set(true);
        let wakers = e.log.borrow_mut().take_wakers();
        wake_all(wakers);
        Ok(())
    }

    async fn open_reader(&self, swarm: u32) -> Result<FakeReader, Closed> {
        let s = self.state.borrow();
        let e = s.swarms.get(&swarm).ok_or(Closed)?;
        let l = e.log.borrow();
        Ok(FakeReader {
            log: e.log.clone(),
            closed: e.closed.clone(),
            epoch: l.epoch,
            pos: l.tail,
            joining: s.live_readers,
        })
    }
}

impl strims_directory::Host for FakeHost {
    async fn open_vpn_port_at(
        &self,
        net: u32,
        port: u16,
    ) -> Result<(VpnReader, VpnWriter), Closed> {
        let port = self
            .state
            .borrow()
            .port_remap
            .get(&port)
            .copied()
            .unwrap_or(port);
        self.open_port(net, port)
    }
}

#[cfg(test)]
mod tests {
    //! The fake swarm writer's flush, and the fake reader's live join and
    //! rewind.
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use strims_ppspp::{RewindError, store};
    use strims_transfer::{ReadFailure, RewindFailure, SwarmHost, SwarmRead, SwarmWrite};

    use super::*;

    fn host() -> FakeHost {
        let world = World::default();
        FakeHost {
            id: ID::from_bytes(&[1; 32]),
            state: Rc::default(),
            world: world.clone(),
            timer: world.timer(),
        }
    }

    /// A live reader starts at the newest chunk once there is one, and
    /// moves back to the start of any chunk the log holds, at or before
    /// its position; a discontinuity stays for the next read.
    #[test]
    fn a_live_reader_joins_at_the_newest_chunk_and_rewinds() {
        crate::exec::run(async {
            let host = host();
            let options = WriterOptions {
                swarm_options: SwarmOptions {
                    chunk_size: 64,
                    chunks_per_signature: 1,
                    live_window: 8,
                    ..Default::default()
                },
                key: strims_crypto::key_from_seed(&[3; 32]),
            };
            let (h, _, mut w) = host.create_swarm(options).await.unwrap();
            host.open_readers_live(true);
            let mut r = host.open_reader(h).await.unwrap();
            let stream: Vec<u8> = (0..1024u32).map(|i| (i % 251) as u8).collect();
            w.write(&stream[..200]).await.unwrap();
            let mut buf = [0u8; 1024];
            let (off, n) = r.read_at(&mut buf).await.unwrap();
            assert_eq!((off, n), (192, 8), "the newest chunk's start");
            let refused = |r: Result<u64, RewindFailure<Closed>>| match r {
                Err(RewindFailure::Refused(e)) => Some(e),
                _ => None,
            };
            assert_eq!(refused(r.rewind(201).await), Some(RewindError::Ahead));
            assert_eq!(r.rewind(70).await.ok(), Some(64));
            let (off, n) = r.read_at(&mut buf).await.unwrap();
            assert_eq!((off, &buf[..n]), (64, &stream[64..200]));

            // past the window: the reader skipped to the tail
            w.write(&stream[200..]).await.unwrap();
            assert!(matches!(
                r.read_at(&mut buf).await,
                Err(ReadFailure::Discontinuity)
            ));
            let (off, _) = r.read_at(&mut buf).await.unwrap();
            assert_eq!(off, 1024 - 512);
            assert_eq!(
                refused(r.rewind(511).await),
                Some(RewindError::OutsideWindow)
            );
            assert_eq!(r.rewind(512).await.ok(), Some(512));

            // a new epoch: the read reports it, not the rewind
            w.reset().await.unwrap();
            assert_eq!(
                refused(r.rewind(512).await),
                Some(RewindError::Read(store::Error::StreamReset))
            );
            assert!(matches!(
                r.read_at(&mut buf).await,
                Err(ReadFailure::Discontinuity)
            ));
            host.remove_swarm(h).await.unwrap();
            assert!(matches!(r.rewind(0).await, Err(RewindFailure::Closed)));
        });
    }

    /// A flush pads the stream with zeros to the swarm's integrity unit and
    /// returns the offset where the next unit starts, where the next write
    /// lands; with nothing staged it returns the same offset.
    #[test]
    fn flush_pads_to_the_unit() {
        crate::exec::run(async {
            let host = host();
            let options = WriterOptions {
                swarm_options: SwarmOptions {
                    chunk_size: 64,
                    chunks_per_signature: 8,
                    live_window: 64,
                    ..Default::default()
                },
                key: strims_crypto::key_from_seed(&[2; 32]),
            };
            let (h, _, mut w) = host.create_swarm(options).await.unwrap();
            let mut r = host.open_reader(h).await.unwrap();
            w.write(&[1; 100]).await.unwrap();
            assert_eq!(w.flush().await.ok(), Some(512));
            assert_eq!(w.flush().await.ok(), Some(512));
            w.write(&[2; 10]).await.unwrap();
            assert_eq!(w.flush().await.ok(), Some(1024));

            let mut got = Vec::new();
            let mut buf = [0u8; 2048];
            while got.len() < 1024 {
                let (off, n) = r.read_at(&mut buf).await.unwrap();
                assert_eq!(off, got.len() as u64);
                got.extend_from_slice(&buf[..n]);
            }
            let mut want = vec![1; 100];
            want.resize(512, 0);
            want.extend([2; 10]);
            want.resize(1024, 0);
            assert_eq!(got, want);
        });
    }
}
