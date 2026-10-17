// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A fake node's host: the network, transfer and directory hosts over the
//! world.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use strims_kademlia::ID;
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
        let log = self.world.log(&id, &options.label, window);
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
        Ok((h, id, FakeWriter { log, closed }))
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
