// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Each host's software: the node, its event policy, link supervision,
//! the swarm writer and readers, and the transfers that bind the swarms'
//! channels.
//!
//! turmoil re-runs [`host_main`] when a host restarts, so everything here
//! starts from the spec and the shared registry: a restarted host joins the
//! same networks and swarms with a new host key, its seed continues the
//! stream from the current time, and its viewers keep accumulating into the
//! same statistics.
//!
//! Every node's stores keep each chunk of a stream as its frames' sequence
//! number and write time ([`FrameCodec`]), or a media chunk's
//! ([`MediaCodec`]), not its bytes.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use futures_util::stream::{Empty, empty};
use strims_node::app::{NodeTimer, PeerHandler, PeerServices};
use strims_node::{
    HostId, NetworkId, NetworkOptions, Node, NodeError, NodeEvent, NodeEvents, PeerInfo, ReadError,
    SimClock, SwarmId, SwarmKey, SwarmReader, SwarmWriter, WriterOptions,
};
use strims_ppspp::SwarmOptions;
use strims_ppspp::store::ChunkBytes;
use strims_rpc::{Client, Router};
use strims_transfer::TransferControl;
use tokio::task::spawn_local;

use crate::APP_PORT;
use crate::net::SimNet;
use crate::spec::{Binding, Fault, HostIx, LinkSpec, MediaSpec, SwarmSpec};
use crate::stream::{
    FRAME_LEN, FrameCodec, FrameParser, JoinRewind, MEDIA_HEADER, MediaCodec, MediaParser,
    encode_frame, fill_media, media_header,
};
use crate::world::{HASH_TABLE_SALT, PEER_INDEX_KEY, World, epoch, sim_now, sim_now_ms};
use strims_wan::live::Signer;

/// How long a dial may take before the supervisor gives up on it.
const DIAL_TIMEOUT: Duration = Duration::from_secs(30);

/// Swarm `s`'s options at every node: its stores keep [`FrameCodec`]
/// state, or a media swarm's [`MediaCodec`] state, unless the scenario
/// keeps the bytes
/// ([`Scenario::stored_chunks`](crate::Scenario::stored_chunks)) or the
/// chunks aren't whole frames.
fn swarm_options(w: &World, s: usize, spec: &SwarmSpec) -> SwarmOptions {
    let mut o = spec.options.clone();
    let swarm = u32::try_from(s).unwrap_or(u32::MAX);
    if w.spec.stored_chunks {
        return o;
    }
    if spec.media.is_some() {
        o.chunk_bytes = ChunkBytes::Codec(Arc::new(MediaCodec::new(w.seed, swarm)));
    } else if FrameCodec::fits(o.chunk_size) {
        o.chunk_bytes = ChunkBytes::Codec(Arc::new(FrameCodec::new(w.seed, swarm)));
    }
    o
}

pub(crate) async fn host_main(w: Rc<World>, h: HostIx) -> turmoil::Result {
    let inc = w.begin_incarnation(h);
    let config = w.node_config(h, inc).ok_or("unknown host")?;
    let net = SimNet::new(w.send_buffers.clone())
        .with_wan(w.wan.clone())
        .with_segment(w.spec.tcp_segment);
    let (node, events) = Node::spawn(config, net, SimClock::turmoil(epoch())).await?;
    // Peers may link before the setup below finishes; they must know us.
    w.register_id(h, node.host_id());

    let mut nets: Vec<Option<NetworkId>> = vec![None; w.spec.networks.len()];
    for (n, spec) in w.spec.networks.iter().enumerate() {
        if !spec.members.contains(&h) {
            continue;
        }
        let key = w.network_keys.get(n).ok_or("network key")?;
        let auto_bind_port = (spec.binding == Binding::Auto).then_some(spec.port);
        let net = node
            .add_network(&key.public, NetworkOptions { auto_bind_port })
            .await?;
        node.set_vpn_handler(net, APP_PORT).await?;
        if let Some(slot) = nets.get_mut(n) {
            *slot = Some(net);
        }
    }
    if let Some(p) = &w.spec.probes
        && let Some(Some(net)) = nets.get(p.network)
    {
        spawn_local(publish(w.clone(), h, node.clone(), *net));
    }

    let transfers = TransferControl::new(
        SimTransferHost(node.clone()),
        NodeTimer(SimClock::turmoil(epoch())),
        strims_transfer::Config {
            discovery: false,
            ..strims_transfer::Config::default()
        },
    );
    spawn_local({
        let transfers = transfers.clone();
        async move { transfers.run().await }
    });
    let mut swarms: Vec<Option<SwarmKey>> = vec![None; w.spec.swarms.len()];
    for (s, spec) in w.spec.swarms.iter().enumerate() {
        let key = w.swarm_keys.get(s).ok_or("swarm key")?;
        let swarm = if spec.seed == h {
            let (swarm, _, writer) = node
                .create_swarm(WriterOptions {
                    swarm_options: swarm_options(&w, s, spec),
                    key: key.clone(),
                })
                .await?;
            match spec.media {
                Some(m) => spawn_local(media_writer_task(w.clone(), h, s, writer, m)),
                None => spawn_local(writer_task(w.clone(), h, s, writer)),
            };
            swarm
        } else if spec.member(h) {
            let swarm = node
                .add_swarm(SwarmId::new(&key.public), swarm_options(&w, s, spec))
                .await?;
            if spec.viewers.contains(&h) {
                spawn_local(reader_task(w.clone(), h, s, node.clone(), swarm));
            }
            swarm
        } else {
            continue;
        };
        let t = transfers.add(swarm, &SwarmId::new(&key.public), Vec::new());
        transfers.publish(t, ());
        if let Some(slot) = swarms.get_mut(s) {
            *slot = Some(swarm);
        }
    }

    w.register(h, &node, nets.clone(), swarms);
    let services = PeerServices::new(node.clone()).with(Rc::new(SimTransfers(transfers)));
    spawn_local(event_task(
        w.clone(),
        h,
        node.clone(),
        events,
        nets,
        services,
    ));
    if w.spec.faults.iter().any(|f| match f.fault {
        Fault::Uplink(x, _) | Fault::Connect { from: x, .. } => x == h,
        _ => false,
    }) {
        spawn_local(control(w.clone(), h, node.clone()));
    }
    spawn_local(supervisor(w.clone(), h, node));
    std::future::pending::<()>().await;
    Ok(())
}

/// Publishes the host's hash table record and peer index entry at
/// startup, before it has links. Publishes repeat only every 10 minutes,
/// so lookups find these records because vpn gives them to links that
/// come up later (see [`strims_vpn::RECORD_PUSH_DELAY`]).
async fn publish(w: Rc<World>, h: HostIx, node: Node, net: NetworkId) {
    let me = w.name(h).to_owned();
    let key = w.hash_table_key(h);
    if let Err(e) = node
        .hash_table_set(net, key, HASH_TABLE_SALT.to_vec(), w.hash_table_value(h))
        .await
    {
        w.log(&me, format!("hash table set failed: {e}"));
    }
    if let Err(e) = node
        .peer_index_publish(net, PEER_INDEX_KEY.to_vec(), w.peer_index_salt(h), APP_PORT)
        .await
    {
        w.log(&me, format!("peer index publish failed: {e}"));
    }
    w.log(&me, "published records");
}

/// The node as the transfers' host. Transfers are published on one
/// network, `()`: every linked peer, so every peer that adds a swarm runs
/// it with every other linked peer that has it. Scenario topologies are
/// fixed, so nothing is discovered.
struct SimTransferHost(Node);

impl strims_transfer::Host for SimTransferHost {
    type Network = ();
    type Swarm = SwarmKey;
    type Publication = ();
    type Search = Empty<HostId>;
    type Error = NodeError;

    fn host_id(&self) -> HostId {
        self.0.host_id()
    }

    async fn run_swarm(
        &self,
        host: HostId,
        swarm: SwarmKey,
        channel: u64,
        peer_channel: u64,
    ) -> Result<(), NodeError> {
        self.0.run_swarm(host, swarm, channel, peer_channel).await
    }

    async fn stop_swarm(&self, host: HostId, swarm: SwarmKey) -> Result<(), NodeError> {
        match self.0.stop_swarm(host, swarm).await {
            Err(NodeError::UnknownPeer) => Ok(()),
            r => r,
        }
    }

    async fn index_publish(&self, _: (), _: Vec<u8>, _: Vec<u8>) -> Result<(), NodeError> {
        Ok(())
    }

    async fn index_unpublish(&self, _: ()) -> Result<(), NodeError> {
        Ok(())
    }

    async fn index_search(
        &self,
        _: (),
        _: Vec<u8>,
        _: Vec<u8>,
    ) -> Result<Empty<HostId>, NodeError> {
        Ok(empty())
    }

    async fn connect_peer(&self, _: (), _: HostId) -> Result<bool, NodeError> {
        Ok(false)
    }
}

/// The transfers as a peer handler: every linked peer is on `()`.
struct SimTransfers(TransferControl<SimTransferHost, NodeTimer<SimClock>>);

impl PeerHandler for SimTransfers {
    fn add_peer(&self, peer: &PeerInfo, router: &Router, client: Client) {
        self.0.add_peer(peer.host_id, router, client);
        self.0.add_network_peer((), peer.host_id);
    }

    fn remove_peer(&self, host: HostId) {
        self.0.remove_peer(host);
    }

    fn handle_event(&self, e: &NodeEvent) {
        if let NodeEvent::TransferError { host_id, .. } = e {
            self.0.transfer_failed(*host_id);
        }
    }
}

/// The host's event policy: serve transfers on every peer's RPC link,
/// bind explicit networks, record vpn probes and exchanges, and log it
/// all.
async fn event_task(
    w: Rc<World>,
    h: HostIx,
    node: Node,
    mut events: NodeEvents,
    nets: Vec<Option<NetworkId>>,
    services: PeerServices,
) {
    let me = w.name(h).to_owned();
    while let Some(e) = events.recv().await {
        services.handle_event(&e).await;
        match e {
            NodeEvent::PeerAdded(p) => {
                let peer = w.ix_of(&p.host_id);
                let dir = if p.dialed { "to" } else { "from" };
                w.log(&me, format!("peer+ {dir} {} {:?}", w.label(peer), p.kind));
                let Some(px) = peer else {
                    continue;
                };
                for (n, net) in nets.iter().enumerate() {
                    let (Some(net), Some(spec)) = (net, w.spec.networks.get(n)) else {
                        continue;
                    };
                    if spec.binding == Binding::Explicit
                        && spec.members.contains(&px)
                        && let Err(e) = node
                            .bind_peer_network(p.host_id, *net, spec.port, spec.port)
                            .await
                    {
                        w.log(&me, format!("bind {} failed: {e}", w.name(px)));
                    }
                }
            }
            NodeEvent::PeerRemoved { host_id } => {
                w.log(&me, format!("peer- {}", w.label(w.ix_of(&host_id))));
            }
            NodeEvent::Message(d) => {
                let body = String::from_utf8_lossy(&d.body);
                if let Some(id) = body.strip_prefix("probe ").and_then(|s| s.parse().ok())
                    && w.probe_delivered(id, d.hops)
                {
                    w.log(
                        &me,
                        format!(
                            "probe {id} from {} hops {}",
                            w.label(w.ix_of(&d.src)),
                            d.hops
                        ),
                    );
                }
            }
            NodeEvent::ExchangeClosed {
                host_id, outcome, ..
            } => {
                w.exchange_closed(&outcome);
                w.log(
                    &me,
                    format!("exchange {} {outcome:?}", w.label(w.ix_of(&host_id))),
                );
            }
            NodeEvent::TransferError { host_id, error } => {
                w.log(
                    &me,
                    format!("transfer error {}: {error}", w.label(w.ix_of(&host_id))),
                );
            }
            NodeEvent::NetworkPeerAdded { .. } | NodeEvent::NetworkPeerRemoved { .. } => {}
        }
    }
}

/// Dials this host's links that are down, now and every
/// `Scenario::redial_every`. It is a run's only reconnect: links heal
/// after partitions and restarts through it.
async fn supervisor(w: Rc<World>, h: HostIx, node: Node) {
    let links: Vec<(usize, LinkSpec)> = w
        .spec
        .links
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, l)| l.from == h)
        .collect();
    if links.is_empty() {
        return;
    }
    loop {
        let Ok(peers) = node.peers().await else {
            return;
        };
        for &(l, spec) in &links {
            let Some(key) = w.profile_key(spec.to) else {
                continue;
            };
            let up = peers.iter().any(|p| p.profile_key == key.public);
            if !up && w.start_dial(h, l) {
                spawn_local(dial(w.clone(), h, l, spec, node.clone()));
            }
        }
        tokio::time::sleep(w.spec.redial_every).await;
    }
}

/// Applies `Fault::Uplink` and `Fault::Connect` to the node.
async fn control(w: Rc<World>, h: HostIx, node: Node) {
    let me = w.name(h).to_owned();
    loop {
        if let Some(rate) = w.take_uplink(h)
            && node.set_rate_limit(rate).await.is_err()
        {
            return;
        }
        for (to, n) in w.take_connects(h) {
            let (Some(net), Some(id)) = (w.net(h, n), w.host_id(to)) else {
                w.log(&me, format!("connect {} skipped", w.name(to)));
                continue;
            };
            match node.connect_peer(net, id).await {
                Ok(started) => w.log(&me, format!("connect {} started {started}", w.name(to))),
                Err(e) => w.log(&me, format!("connect {} failed: {e}", w.name(to))),
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn dial(w: Rc<World>, h: HostIx, l: usize, spec: LinkSpec, node: Node) {
    let Some(uri) = w.uri(spec.to, spec.transport) else {
        return;
    };
    let result = match tokio::time::timeout(DIAL_TIMEOUT, node.dial(&uri)).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("timed out".to_owned()),
    };
    w.end_dial(h, l, result);
}

/// Writes `frames_per_write` frames every `write_interval` from `start` to
/// `stop`. The write index comes from the simulated time, so a restarted
/// seed continues the numbering.
async fn writer_task(w: Rc<World>, h: HostIx, s: usize, mut writer: SwarmWriter) {
    let Some(spec) = w.spec.swarms.get(s).cloned() else {
        return;
    };
    let me = w.name(h).to_owned();
    let fpw = spec.frames_per_write as u64;
    let step = spec.write_interval.as_millis().max(1);
    let now = sim_now();
    let mut k = if now <= spec.start {
        0
    } else {
        u64::try_from((now - spec.start).as_millis().div_ceil(step)).unwrap_or(u64::MAX)
    };
    let mut buf = vec![0u8; spec.frames_per_write * FRAME_LEN];
    loop {
        let at = spec.start + spec.write_interval * u32::try_from(k).unwrap_or(u32::MAX);
        if at >= spec.stop {
            break;
        }
        tokio::time::sleep(at.saturating_sub(sim_now())).await;
        if w.take_writer_reset(s) {
            match writer.reset().await {
                Ok(()) => w.log(&me, format!("swarm {s} writer reset")),
                Err(e) => w.log(&me, format!("swarm {s} writer reset failed: {e}")),
            }
        }
        let t = sim_now_ms();
        for (j, f) in buf.chunks_mut(FRAME_LEN).enumerate() {
            encode_frame(w.seed, s as u32, k * fpw + j as u64, t, f);
        }
        if let Err(e) = writer.write(&buf).await {
            w.log(&me, format!("swarm {s} write failed: {e}"));
            return;
        }
        w.mark_written(s, k * fpw..(k + 1) * fpw);
        if k % 40 == 0 {
            w.log(&me, format!("swarm {s} wrote {}", (k + 1) * fpw));
        }
        k += 1;
    }
    w.log(&me, format!("swarm {s} writer done"));
    // Keep the publisher: dropping the writer closes it.
    std::future::pending::<()>().await;
}

/// Writes media swarm `s`'s access units ([`World::media_traces`]) from
/// `start` to `stop`, each record as its access unit arrives, a
/// [`MEDIA_HEADER`] first in each chunk it opens, and signs a unit in
/// progress at its deadline ([`Signer`], `SwarmWriter::flush`). A
/// restarted seed, or a writer reset, starts the units over at the stream's
/// start with the next access unit; the chunks' sequence numbers go on.
async fn media_writer_task(
    w: Rc<World>,
    h: HostIx,
    s: usize,
    mut writer: SwarmWriter,
    m: MediaSpec,
) {
    let (Some(spec), Some(Some(trace))) = (w.spec.swarms.get(s).cloned(), w.media_traces.get(s))
    else {
        return;
    };
    let me = w.name(h).to_owned();
    let chunk = spec.options.chunk_size;
    let cps = spec.options.chunks_per_signature;
    let new_signer = || Signer::new(m.sign.deadline, cps, chunk, MEDIA_HEADER);
    w.media_log(s, |l| {
        l.unit_chunks = cps as u64;
        l.chunk = chunk as u64;
    });
    let mut st = MediaWriter {
        signer: new_signer(),
        chunk: chunk as u64,
        open: None,
        pending: Vec::new(),
    };
    let at = |d: strims_base::time::Duration| {
        spec.start + Duration::from_millis(u64::try_from(d.as_millis()).unwrap_or(0))
    };
    let now = sim_now();
    let mut buf = Vec::new();
    for au in trace.iter().filter(|a| at(a.at) >= now) {
        let when = at(au.at);
        if when >= spec.stop {
            break;
        }
        while let Some(due) = st.signer.due().map(at).filter(|&d| d <= when) {
            tokio::time::sleep(due.saturating_sub(sim_now())).await;
            if !st.flush(&w, s, &mut writer, &me).await {
                return;
            }
        }
        tokio::time::sleep(when.saturating_sub(sim_now())).await;
        if w.take_writer_reset(s) {
            match writer.reset().await {
                Ok(()) => w.log(&me, format!("swarm {s} writer reset")),
                Err(e) => w.log(&me, format!("swarm {s} writer reset failed: {e}")),
            }
            st = MediaWriter {
                signer: new_signer(),
                chunk: chunk as u64,
                open: None,
                pending: Vec::new(),
            };
        }
        let placed = st.signer.record(au);
        if au.key && au.video {
            w.media_log(s, |l| l.gops.push(placed.offset));
        }
        st.lay_out(&w, s, chunk, placed.offset, placed.len, &mut buf);
        if let Err(e) = writer.write(&buf).await {
            w.log(&me, format!("swarm {s} write failed: {e}"));
            return;
        }
        let unit = st.signer.unit_bytes();
        st.signed(
            &w,
            s,
            (placed.offset + placed.len) / unit * unit / chunk as u64,
        );
        w.media_log(s, |l| {
            l.media_bytes += u64::from(au.len);
            l.stream_bytes += placed.len;
            l.units += placed.signed;
        });
    }
    if let Some(due) = st.signer.due() {
        tokio::time::sleep(at(due).saturating_sub(sim_now())).await;
        st.flush(&w, s, &mut writer, &me).await;
    }
    w.log(&me, format!("swarm {s} writer done"));
    // Keep the publisher: dropping the writer closes it.
    std::future::pending::<()>().await;
}

/// A media writer's state: the units, the chunk in progress (its sequence
/// number and the stream offset where it starts), and the chunks not yet
/// signed (index and first byte's time, ms).
struct MediaWriter {
    signer: Signer,
    chunk: u64,
    open: Option<(u64, u64)>,
    pending: Vec<(u64, u64)>,
}

impl MediaWriter {
    /// The bytes of the record at `offset`, `len` long, into `buf`: a
    /// header at each chunk it opens, its chunks' payload elsewhere.
    fn lay_out(
        &mut self,
        w: &World,
        s: usize,
        chunk: usize,
        offset: u64,
        len: u64,
        buf: &mut Vec<u8>,
    ) {
        let size = chunk as u64;
        let now = sim_now_ms();
        buf.clear();
        let mut p = offset;
        let end = offset + len;
        while p < end {
            let at = usize::try_from(p % size).unwrap_or(0);
            if at == 0 {
                let seq = w.media_chunk(s, now);
                self.open = Some((seq, p));
                self.pending.push((p / size, now));
                let h = media_header(seq, now);
                let n = h.len().min(usize::try_from(end - p).unwrap_or(0));
                buf.extend_from_slice(h.get(..n).unwrap_or_default());
                p += n as u64;
                continue;
            }
            let take = (size - p % size).min(end - p);
            let take = usize::try_from(take).unwrap_or(0);
            let from = buf.len();
            buf.resize(from + take, 0);
            let seq = self.open.map_or(0, |(seq, _)| seq);
            if let Some(out) = buf.get_mut(from..) {
                fill_media(w.seed, s as u32, seq, at, out);
            }
            p += take as u64;
        }
    }

    /// The chunks before index `through` were signed now.
    fn signed(&mut self, w: &World, s: usize, through: u64) {
        let now = sim_now_ms();
        let waits: Vec<u32> = self
            .pending
            .iter()
            .filter(|&&(c, _)| c < through)
            .map(|&(_, t)| u32::try_from(now.saturating_sub(t)).unwrap_or(u32::MAX))
            .collect();
        if !waits.is_empty() {
            self.pending.retain(|&(c, _)| c >= through);
            w.media_log(s, |l| l.sign_waits.extend(waits));
        }
    }

    /// Signs the unit in progress through the writer; false once the
    /// writer fails or pads where the signer doesn't.
    async fn flush(&mut self, w: &World, s: usize, writer: &mut SwarmWriter, me: &str) -> bool {
        let pad = self.signer.flush();
        match writer.flush().await {
            Ok(next) if next == pad.end => {}
            Ok(next) => {
                w.log(me, format!("swarm {s} flush to {next}, not {}", pad.end));
                return false;
            }
            Err(e) => {
                w.log(me, format!("swarm {s} flush failed: {e}"));
                return false;
            }
        }
        let size = self.chunk.max(1);
        self.signed(w, s, pad.end / size);
        let alone = pad.end / size - pad.start.div_ceil(size);
        let now = sim_now_ms();
        w.media_log(s, |l| {
            l.units += 1;
            l.stream_bytes += pad.end - pad.start;
            if alone > 0 {
                l.pad_chunks.push((now, alone));
            }
        });
        true
    }
}

async fn reader_task(w: Rc<World>, h: HostIx, s: usize, node: Node, swarm: SwarmKey) {
    let me = w.name(h).to_owned();
    let mut reader = match node.open_reader(swarm).await {
        Ok(r) => r,
        Err(e) => {
            w.log(&me, format!("swarm {s} open reader failed: {e}"));
            return;
        }
    };
    w.viewer_start(s, h);
    let mut buf = vec![0u8; 64 * 1024];
    if w.spec.swarms.get(s).is_some_and(|x| x.media.is_some()) {
        media_reader(&w, h, s, &mut reader, &mut buf).await;
        return;
    }
    let mut parser = FrameParser::default();
    loop {
        while w.reading_paused(h) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        match reader.read(&mut buf).await {
            Ok(n) => w.viewer_data(s, h, &mut parser, buf.get(..n).unwrap_or_default()),
            Err(ReadError::Underrun { skipped }) => {
                parser.resync();
                w.viewer_underrun(s, h, skipped);
            }
            Err(ReadError::Reset) => {
                parser.resync();
                w.viewer_reset(s, h);
            }
            Err(e) => {
                w.log(&me, format!("swarm {s} reader ended: {e}"));
                return;
            }
        }
    }
}

/// [`reader_task`]'s loop for a media swarm: reads at offsets, checked
/// chunk by chunk ([`MediaParser`]), and rewinds to its GOP's start after
/// its first read ([`SwarmSpec::join_rewind`]).
async fn media_reader(w: &World, h: HostIx, s: usize, reader: &mut SwarmReader, buf: &mut [u8]) {
    let me = w.name(h).to_owned();
    let mut parser = MediaParser::default();
    let mut rewind = w.spec.swarms.get(s).and_then(|x| x.join_rewind);
    loop {
        while w.reading_paused(h) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        match reader.read_at(buf).await {
            Ok((offset, n)) => {
                w.viewer_media(s, h, &mut parser, offset, buf.get(..n).unwrap_or_default());
                if let Some(deadline) = rewind.take() {
                    join_rewind(w, h, s, reader, offset, deadline).await;
                }
            }
            Err(ReadError::Underrun { skipped }) => {
                parser.resync();
                w.viewer_underrun(s, h, skipped);
            }
            Err(ReadError::Reset) => {
                parser.resync();
                w.viewer_reset(s, h);
            }
            Err(e) => {
                w.log(&me, format!("swarm {s} reader ended: {e}"));
                return;
            }
        }
    }
}

/// Moves `reader` back to the start of the GOP that stream offset `at`
/// falls in, within `deadline`, and notes how it went.
async fn join_rewind(
    w: &World,
    h: HostIx,
    s: usize,
    reader: &mut SwarmReader,
    at: u64,
    deadline: Duration,
) {
    let Some(to) = w.gop_start(s, at).filter(|&g| g < at) else {
        return;
    };
    let start = sim_now_ms();
    let landed = matches!(
        tokio::time::timeout(deadline, reader.rewind(to)).await,
        Ok(Ok(_))
    );
    let took = sim_now_ms().saturating_sub(start);
    let back = at - to;
    let how = if landed { "landed" } else { "dropped" };
    w.log(
        w.name(h),
        format!("swarm {s} join rewind of {back} bytes {how} after {took} ms"),
    );
    w.viewer_rewound(
        s,
        h,
        JoinRewind {
            start_ms: start,
            took_ms: took,
            landed,
            back,
        },
    );
}
