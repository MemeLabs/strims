// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The video control: the profile's channels, the device's ingress and
//! HLS configs, the records' events, and the node's video swarms.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::future::poll_fn;
use std::rc::Rc;
use std::task::{Poll, Waker};

use futures_util::future::LocalBoxFuture;
use futures_util::stream::FuturesUnordered;
use futures_util::{FutureExt, StreamExt};
use strims_base::rng::DetRng;
use strims_dao::events::Record;
use strims_dao::schema::network::network_key;
use strims_dao::schema::video::{HlsConfig, IngressConfig, VideoChannels};
use strims_dao::{Broadcast, Recv, ReplicatedStore, Singleton, StoreEvent, Subscription, Table};
use strims_directory::{DirectoryControl, DirectoryEvent, Host, ListingKeeper};
use strims_kv::Kv;
use strims_network::{NetworkControl, NetworkEvent};
use strims_proto::profile::v1::Profile;
use strims_proto::types::Key;
use strims_proto::video::v1::{
    HlsEgressConfig, VideoChannel, VideoIngressConfig, VideoIngressStream,
};
use strims_rpc::Timer;
use strims_transfer::{TransferControl, TransferId};

use crate::capture::Capture;
use crate::config::{Config, Metrics, check_hls_config, check_ingress_config};
use crate::hls::HlsRoutes;
use crate::hls_egress::{HlsEgress, HlsSweep};
use crate::ingress::Ingress;
use crate::registry::{LocalSwarm, Registry, VideoSwarm};
use crate::{VideoError, VideoEvent, VideoHost};

/// A swarm of the node.
pub(crate) type Swarm<H> = <H as strims_transfer::Host>::Swarm;

pub(crate) struct Shared<K: Kv, H: Host, V: VideoHost, T: Timer> {
    pub(crate) host: H,
    pub(crate) video: V,
    pub(crate) store: ReplicatedStore<K>,
    pub(crate) profile: Profile,
    pub(crate) network: NetworkControl<K, H, T>,
    pub(crate) transfers: TransferControl<H, T>,
    pub(crate) directory: DirectoryControl<K, H, T>,
    pub(crate) timer: T,
    pub(crate) rng: RefCell<DetRng>,
    pub(crate) config: Config,
    records: RefCell<Records>,
    pub(crate) events: Broadcast<VideoEvent>,
    pub(crate) metrics: RefCell<Metrics>,
    /// The node's video swarms. Reached through
    /// [`with_swarms`](Self::with_swarms), which wakes their waiters.
    swarms: RefCell<Registry<Swarm<H>>>,
    /// The captures, by transfer id. Bound:
    /// [`CaptureSettings::max_captures`](crate::CaptureSettings::max_captures).
    pub(crate) captures: RefCell<BTreeMap<TransferId, Capture<H>>>,
    /// The HLS streams, and the profile's registration on the HLS routes.
    pub(crate) hls: RefCell<HlsEgress>,
    /// The RTMP ingress: its config, connections and sessions.
    pub(crate) ingress: RefCell<Ingress>,
    /// Tasks for `run` to start. Bound: a reload per lag of the store's
    /// events, each ending after one read of the records; per capture, its
    /// creation, its writer and a listing keeper per network; an add per
    /// egress slot; a removal per removed swarm, each one node call; the
    /// RTMP listener, a task per RTMP connection and a listing keeper per
    /// ingest session.
    spawned: RefCell<Vec<LocalBoxFuture<'static, ()>>>,
    /// Wakes `run`.
    waker: RefCell<Option<Waker>>,
}

/// A value one task sets and another takes.
pub(crate) struct Slot<T> {
    value: RefCell<Option<T>>,
    waker: RefCell<Option<Waker>>,
}

impl<T> Slot<T> {
    pub(crate) fn new() -> Rc<Slot<T>> {
        Rc::new(Slot {
            value: RefCell::new(None),
            waker: RefCell::new(None),
        })
    }

    /// Sets the value and wakes the taker.
    pub(crate) fn set(&self, v: T) {
        let prev = self.value.replace(Some(v));
        drop(prev);
        let w = self.waker.take();
        if let Some(w) = w {
            w.wake();
        }
    }

    /// The value, once set.
    pub(crate) async fn take(&self) -> T {
        poll_fn(|cx| {
            if let Some(v) = self.value.take() {
                return Poll::Ready(v);
            }
            let prev = self.waker.replace(Some(cx.waker().clone()));
            drop(prev);
            Poll::Pending
        })
        .await
    }

    /// The value, if set and not taken.
    pub(crate) fn take_now(&self) -> Option<T> {
        self.value.take()
    }
}

/// The channels and configs as the control last took them, from the
/// store's events or a read of every record.
#[derive(Default)]
struct Records {
    /// Bound: the profile's channels in the store.
    channels: BTreeMap<u64, VideoChannel>,
    ingress: Option<VideoIngressConfig>,
    hls: Option<HlsEgressConfig>,
    /// The current read of every record, while one runs.
    reload: Option<Reload>,
    next_reload: u64,
}

/// A read of every record, and the records store events wrote since it
/// began. The read leaves those as the events set them: a newer write the
/// read saw still has its event to come.
struct Reload {
    id: u64,
    /// Bound: the profile's channels in the store, and those deleted
    /// during the read.
    channels: BTreeSet<u64>,
    ingress: bool,
    hls: bool,
}

/// Every record, as one read found them.
struct Snapshot {
    channels: Vec<VideoChannel>,
    ingress: VideoIngressConfig,
    hls: HlsEgressConfig,
}

impl Records {
    fn channel_written(&mut self, c: &VideoChannel) {
        if let Some(r) = &mut self.reload {
            r.channels.insert(c.id);
        }
        self.channels.insert(c.id, c.clone());
    }

    fn channel_deleted(&mut self, c: &VideoChannel) {
        if let Some(r) = &mut self.reload {
            r.channels.insert(c.id);
        }
        self.channels.remove(&c.id);
    }

    fn ingress_written(&mut self, c: &VideoIngressConfig) {
        if let Some(r) = &mut self.reload {
            r.ingress = true;
        }
        self.ingress = Some(c.clone());
    }

    fn hls_written(&mut self, c: &HlsEgressConfig) {
        if let Some(r) = &mut self.reload {
            r.hls = true;
        }
        self.hls = Some(c.clone());
    }

    /// Takes the records of `s`, read by `reload`, except those events
    /// wrote since it began: the events that report the difference.
    fn apply(&mut self, reload: Reload, s: Snapshot) -> Vec<VideoEvent> {
        let mut events = Vec::new();
        let read: BTreeMap<u64, VideoChannel> = s.channels.into_iter().map(|c| (c.id, c)).collect();
        let gone: Vec<u64> = self
            .channels
            .keys()
            .filter(|id| !read.contains_key(id) && !reload.channels.contains(id))
            .copied()
            .collect();
        for id in gone {
            if let Some(c) = self.channels.remove(&id) {
                events.push(VideoEvent::ChannelDeleted(c));
            }
        }
        for (id, c) in read {
            if reload.channels.contains(&id) || self.channels.get(&id) == Some(&c) {
                continue;
            }
            self.channels.insert(id, c.clone());
            events.push(VideoEvent::ChannelChanged(c));
        }
        if !reload.ingress && self.ingress.as_ref() != Some(&s.ingress) {
            self.ingress = Some(s.ingress.clone());
            events.push(VideoEvent::IngressConfigChanged(s.ingress));
        }
        if !reload.hls && self.hls.as_ref() != Some(&s.hls) {
            self.hls = Some(s.hls.clone());
            events.push(VideoEvent::HlsConfigChanged(s.hls));
        }
        events
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Shared<K, H, V, T> {
    /// Has `run` start a task.
    pub(crate) fn spawn(&self, task: LocalBoxFuture<'static, ()>) {
        self.spawned.borrow_mut().push(task);
        let w = self.waker.borrow_mut().take();
        if let Some(w) = w {
            w.wake();
        }
    }

    /// The profile's key, which signs its channels' stream keys.
    pub(crate) fn profile_key(&self) -> Result<&Key, VideoError> {
        self.profile.key.as_ref().ok_or(VideoError::NoProfileKey)
    }

    // --- swarms ---------------------------------------------------------------

    /// Runs `f` on the swarm registry, then wakes the tasks it woke.
    pub(crate) fn with_swarms<R>(&self, f: impl FnOnce(&mut Registry<Swarm<H>>) -> R) -> R {
        let (out, woken) = {
            let mut r = self.swarms.borrow_mut();
            let out = f(&mut r);
            (out, r.take_woken())
        };
        for w in woken {
            w.wake();
        }
        out
    }

    /// Counts a failed node call, and the error it answers with.
    pub(crate) fn host_failed(&self, what: &str, e: &impl fmt::Display) -> VideoError {
        self.metrics.borrow_mut().host_errors += 1;
        tracing::warn!(error = %e, "{what} failed");
        VideoError::Host(e.to_string())
    }

    /// Removes `swarm` from the node. Remove its transfer first.
    pub(crate) async fn remove_host_swarm(&self, swarm: Swarm<H>) {
        if let Err(e) = self.host.remove_swarm(swarm).await {
            self.host_failed("removing a video swarm", &e);
        }
    }

    /// Takes down a slot gone from the registry: its transfer now, and
    /// its swarm in a task.
    pub(crate) fn drop_swarm(self: &Rc<Self>, t: TransferId, e: LocalSwarm<Swarm<H>>) {
        if let Some(swarm) = e.swarm {
            self.transfers.remove(t);
            let s = self.clone();
            self.spawn(async move { s.remove_host_swarm(swarm).await }.boxed_local());
        }
    }

    /// Publishes the transfer of `t` on each of its networks the node has
    /// joined and it isn't published on.
    pub(crate) fn publish_joined(&self, t: &TransferId) {
        let networks: Vec<Vec<u8>> = self.with_swarms(|r| {
            r.get(t)
                .filter(|e| e.swarm.is_some())
                .map(|e| e.networks.iter().cloned().collect())
                .unwrap_or_default()
        });
        for key in networks {
            if let Some(net) = self.network.network(&key).and_then(|i| i.net)
                && !self.transfers.is_published(*t, net)
            {
                self.transfers.publish(*t, net);
            }
        }
    }

    /// [`publish_joined`](Self::publish_joined) for every video swarm.
    fn publish_all(&self) {
        for t in self.with_swarms(|r| r.ids()) {
            self.publish_joined(&t);
        }
    }

    fn handle_network_event(&self, e: &NetworkEvent) {
        match e {
            NetworkEvent::Joined { network } | NetworkEvent::Updated { network } => {
                let key = network_key(network);
                let ids: Vec<TransferId> = self.with_swarms(|r| {
                    r.ids()
                        .into_iter()
                        .filter(|t| r.get(t).is_some_and(|e| e.networks.contains(&key)))
                        .collect()
                });
                for t in ids {
                    self.publish_joined(&t);
                }
            }
            _ => {}
        }
    }

    /// The listing keepers of the captures and ingest sessions.
    fn keepers(&self) -> Vec<ListingKeeper> {
        let mut keepers: Vec<ListingKeeper> = self
            .captures
            .borrow()
            .values()
            .flat_map(|c| c.keepers().cloned())
            .collect();
        keepers.extend(self.ingest_keepers());
        keepers
    }

    fn handle_directory_event(&self, e: &DirectoryEvent) {
        for k in self.keepers() {
            k.handle_event(e);
        }
    }

    fn handle_store_event(&self, e: &StoreEvent) {
        let event = {
            let mut r = self.records.borrow_mut();
            match e {
                StoreEvent::Changed(Record::VideoChannel(c)) => {
                    r.channel_written(c);
                    VideoEvent::ChannelChanged(c.clone())
                }
                StoreEvent::Deleted(Record::VideoChannel(c)) => {
                    r.channel_deleted(c);
                    VideoEvent::ChannelDeleted(c.clone())
                }
                StoreEvent::Changed(Record::VideoIngressConfig(c)) => {
                    r.ingress_written(c);
                    VideoEvent::IngressConfigChanged(c.clone())
                }
                StoreEvent::Deleted(Record::VideoIngressConfig(_)) => {
                    let c = IngressConfig::default_value().unwrap_or_default();
                    r.ingress_written(&c);
                    VideoEvent::IngressConfigChanged(c)
                }
                StoreEvent::Changed(Record::HlsEgressConfig(c)) => {
                    r.hls_written(c);
                    VideoEvent::HlsConfigChanged(c.clone())
                }
                StoreEvent::Deleted(Record::HlsEgressConfig(_)) => {
                    let c = HlsConfig::default_value().unwrap_or_default();
                    r.hls_written(&c);
                    VideoEvent::HlsConfigChanged(c)
                }
                _ => return,
            }
        };
        if let VideoEvent::HlsConfigChanged(c) = &event {
            self.hls_config_written(c);
        }
        self.ingress_records_changed(std::slice::from_ref(&event));
        self.events.emit(vec![event]);
    }

    /// Starts reading every record again, in place of any read that
    /// runs. `quiet` takes what it reads without reporting it.
    fn start_reload(self: &Rc<Self>, quiet: bool) {
        let id = {
            let mut r = self.records.borrow_mut();
            let id = r.next_reload;
            r.next_reload += 1;
            r.reload = Some(Reload {
                id,
                channels: BTreeSet::new(),
                ingress: false,
                hls: false,
            });
            id
        };
        self.spawn(self.clone().reload(id, quiet).boxed_local());
    }

    async fn reload(self: Rc<Self>, id: u64, quiet: bool) {
        let read = async {
            let mut tx = self.store.read().await?;
            Ok::<_, strims_dao::Error>(Snapshot {
                channels: VideoChannels::get_all(&mut tx).await?,
                ingress: IngressConfig::get(&mut tx).await?.unwrap_or_default(),
                hls: HlsConfig::get(&mut tx).await?.unwrap_or_default(),
            })
        }
        .await;
        let events = {
            let mut r = self.records.borrow_mut();
            let Some(reload) = r.reload.take_if(|l| l.id == id) else {
                return;
            };
            match read {
                Ok(s) => r.apply(reload, s),
                Err(e) => {
                    self.metrics.borrow_mut().store_errors += 1;
                    tracing::warn!(error = %e, "reading the video records failed");
                    return;
                }
            }
        };
        for e in &events {
            if let VideoEvent::HlsConfigChanged(c) = e {
                self.hls_config_written(c);
            }
        }
        self.ingress_records_changed(&events);
        if !quiet {
            self.events.emit(events);
        }
    }
}

/// Runs the profile's video: its channels, the device's ingress and HLS
/// configs, its captures and streams; see the [crate docs](crate).
///
/// Clones share the control.
pub struct VideoControl<K: Kv, H: Host, V: VideoHost, T: Timer> {
    pub(crate) shared: Rc<Shared<K, H, V, T>>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Clone for VideoControl<K, H, V, T> {
    fn clone(&self) -> Self {
        VideoControl {
            shared: self.shared.clone(),
        }
    }
}

/// What woke `run`.
enum Wake {
    Task,
    Store(Option<Recv<StoreEvent>>),
    Network(Option<Recv<NetworkEvent>>),
    Directory(Option<Recv<DirectoryEvent>>),
    Work,
    HlsSweep,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> VideoControl<K, H, V, T> {
    /// A control for the node behind `host`, with the RTMP listener of
    /// `video`, keeping the video channels of `profile` and the device's
    /// configs in `store`. Streams go on the networks `network` joins, as
    /// transfers run by `transfers`, listed through `directory`, and HLS
    /// streams served on `hls_routes`. Timed on `timer`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: H,
        video: V,
        store: ReplicatedStore<K>,
        profile: Profile,
        network: NetworkControl<K, H, T>,
        transfers: TransferControl<H, T>,
        directory: DirectoryControl<K, H, T>,
        hls_routes: HlsRoutes,
        timer: T,
        rng: DetRng,
        config: Config,
    ) -> Self {
        let hls = RefCell::new(HlsEgress::new(&hls_routes, &profile));
        VideoControl {
            shared: Rc::new(Shared {
                host,
                video,
                store,
                profile,
                network,
                transfers,
                directory,
                timer,
                rng: RefCell::new(rng),
                config,
                records: RefCell::default(),
                events: Broadcast::new(),
                metrics: RefCell::default(),
                swarms: RefCell::default(),
                captures: RefCell::default(),
                hls,
                ingress: RefCell::default(),
                spawned: RefCell::new(Vec::new()),
                waker: RefCell::new(None),
            }),
        }
    }

    /// The changes the control sees from now on; see [`VideoEvent`].
    pub fn subscribe(&self) -> Subscription<VideoEvent> {
        self.shared.events.subscribe()
    }

    /// The control's counters.
    pub fn metrics(&self) -> Metrics {
        self.shared.metrics.borrow().clone()
    }

    /// The node's video swarms, in transfer id order; see the
    /// [crate docs](crate#video-swarms).
    pub fn video_swarms(&self) -> Vec<VideoSwarm> {
        self.shared.with_swarms(|r| r.list())
    }

    // --- ingress ------------------------------------------------------------

    /// Whether this node can run the RTMP ingress: its [`VideoHost`]
    /// listens for TCP connections.
    pub fn ingress_supported(&self) -> bool {
        self.shared.video.rtmp_supported()
    }

    /// The device's ingress config.
    pub async fn ingress_config(&self) -> Result<VideoIngressConfig, VideoError> {
        let mut tx = self.shared.store.read().await?;
        Ok(IngressConfig::get(&mut tx).await?.unwrap_or_default())
    }

    /// Stores `c` as the device's ingress config, once it passes
    /// [`check_ingress_config`]. Its service network keys are kept as
    /// given.
    pub async fn set_ingress_config(
        &self,
        c: VideoIngressConfig,
    ) -> Result<VideoIngressConfig, VideoError> {
        check_ingress_config(&c)?;
        let mut tx = self.shared.store.write(self.shared.timer.now()).await?;
        let c = IngressConfig::set(&mut tx, c).await?;
        tx.commit().await?;
        Ok(c)
    }

    /// The streams the ingress is taking now, by channel; see the
    /// [ingress docs](crate::ingress).
    pub fn ingress_streams(&self) -> Vec<VideoIngressStream> {
        self.shared.ingress_streams()
    }

    // --- HLS ----------------------------------------------------------------

    /// Whether the driver serves HLS ([`HlsSettings::serve`](crate::HlsSettings::serve))
    /// and the profile's streams are routed there; see
    /// [`hls_egress`](crate::hls_egress).
    pub fn hls_supported(&self) -> bool {
        self.shared.config.hls.serve && self.shared.hls.borrow().routed()
    }

    /// The device's HLS egress config.
    pub async fn hls_config(&self) -> Result<HlsEgressConfig, VideoError> {
        let mut tx = self.shared.store.read().await?;
        Ok(HlsConfig::get(&mut tx).await?.unwrap_or_default())
    }

    /// Stores `c` as the device's HLS egress config, once it passes
    /// [`check_hls_config`].
    pub async fn set_hls_config(&self, c: HlsEgressConfig) -> Result<HlsEgressConfig, VideoError> {
        check_hls_config(&c)?;
        let mut tx = self.shared.store.write(self.shared.timer.now()).await?;
        let c = HlsConfig::set(&mut tx, c).await?;
        tx.commit().await?;
        Ok(c)
    }

    // --- running ------------------------------------------------------------

    /// Runs the video: follows the store's channels and configs and
    /// reports their changes, publishes the video swarms on networks as
    /// the node joins them, feeds the listing keepers the directory's
    /// events, runs the RTMP ingress, and carries the control's tasks:
    /// swarm adds and removals, capture writers, RTMP connections,
    /// listing keepers and HLS stream readers. Closes idle HLS streams.
    /// Never resolves; drop it to stop. Run it once per
    /// control.
    pub async fn run(&self) {
        let s = &self.shared;
        let mut store_events = Some(s.store.events().subscribe());
        let mut network_events = Some(s.network.subscribe());
        let mut directory_events = Some(s.directory.subscribe());
        s.start_reload(true);
        s.publish_all();
        s.start_ingress();
        let mut tasks: FuturesUnordered<LocalBoxFuture<'static, ()>> = FuturesUnordered::new();
        let mut hls_sweep: HlsSweep<T> = None;
        loop {
            let spawned: Vec<_> = s.spawned.borrow_mut().drain(..).collect();
            tasks.extend(spawned);
            let wake = poll_fn(|cx| {
                *s.waker.borrow_mut() = Some(cx.waker().clone());
                if let Poll::Ready(Some(())) = tasks.poll_next_unpin(cx) {
                    return Poll::Ready(Wake::Task);
                }
                if let Some(ev) = &mut store_events
                    && let Poll::Ready(r) = ev.poll_next_unpin(cx)
                {
                    return Poll::Ready(Wake::Store(r));
                }
                if let Some(ev) = &mut network_events
                    && let Poll::Ready(r) = ev.poll_next_unpin(cx)
                {
                    return Poll::Ready(Wake::Network(r));
                }
                if let Some(ev) = &mut directory_events
                    && let Poll::Ready(r) = ev.poll_next_unpin(cx)
                {
                    return Poll::Ready(Wake::Directory(r));
                }
                if s.poll_hls_sweep(&mut hls_sweep, cx).is_ready() {
                    return Poll::Ready(Wake::HlsSweep);
                }
                if !s.spawned.borrow().is_empty() {
                    return Poll::Ready(Wake::Work);
                }
                Poll::Pending
            })
            .await;
            match wake {
                Wake::Store(Some(Recv::Event(e))) => s.handle_store_event(&e),
                Wake::Store(Some(Recv::Lagged)) => {
                    s.metrics.borrow_mut().store_lags += 1;
                    s.start_reload(false);
                }
                Wake::Store(None) => store_events = None,
                Wake::Network(Some(Recv::Event(e))) => s.handle_network_event(&e),
                Wake::Network(Some(Recv::Lagged)) => s.publish_all(),
                Wake::Network(None) => network_events = None,
                Wake::Directory(Some(Recv::Event(e))) => s.handle_directory_event(&e),
                Wake::Directory(Some(Recv::Lagged)) => {
                    for k in s.keepers() {
                        k.check();
                    }
                }
                Wake::Directory(None) => directory_events = None,
                Wake::HlsSweep => s.sweep_hls(),
                Wake::Task | Wake::Work => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]

    use super::*;

    fn channel(id: u64, title: &str) -> VideoChannel {
        VideoChannel {
            id,
            directory_listing_snippet: Some(strims_proto::network::v1::directory::ListingSnippet {
                title: title.into(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn reload(id: u64) -> Reload {
        Reload {
            id,
            channels: BTreeSet::new(),
            ingress: false,
            hls: false,
        }
    }

    fn snapshot(channels: Vec<VideoChannel>, enabled: bool) -> Snapshot {
        Snapshot {
            channels,
            ingress: VideoIngressConfig {
                enabled,
                ..Default::default()
            },
            hls: HlsEgressConfig::default(),
        }
    }

    #[test]
    fn a_reload_reports_what_differs_from_the_last_report() {
        let mut r = Records::default();
        r.channel_written(&channel(1, "a"));
        r.channel_written(&channel(2, "b"));
        r.ingress_written(&VideoIngressConfig::default());
        r.hls_written(&HlsEgressConfig::default());
        let events = r.apply(
            reload(0),
            snapshot(vec![channel(2, "b2"), channel(3, "c")], true),
        );
        assert_eq!(
            events,
            [
                VideoEvent::ChannelDeleted(channel(1, "a")),
                VideoEvent::ChannelChanged(channel(2, "b2")),
                VideoEvent::ChannelChanged(channel(3, "c")),
                VideoEvent::IngressConfigChanged(VideoIngressConfig {
                    enabled: true,
                    ..Default::default()
                }),
            ]
        );
        assert_eq!(r.channels.len(), 2);
    }

    #[test]
    fn a_reload_keeps_what_events_wrote_during_its_read() {
        let mut r = Records::default();
        r.channel_written(&channel(1, "a"));
        r.reload = Some(reload(0));
        r.channel_written(&channel(2, "new"));
        r.channel_deleted(&channel(1, "a"));
        r.ingress_written(&VideoIngressConfig {
            enabled: true,
            ..Default::default()
        });
        let touched = r.reload.take().unwrap_or_else(|| reload(0));
        let events = r.apply(
            touched,
            snapshot(vec![channel(1, "a"), channel(2, "old")], false),
        );
        assert_eq!(
            events,
            [VideoEvent::HlsConfigChanged(HlsEgressConfig::default())]
        );
        assert_eq!(r.channels.keys().copied().collect::<Vec<_>>(), [2]);
        assert_eq!(r.channels[&2], channel(2, "new"));
        assert!(r.ingress.as_ref().is_some_and(|c| c.enabled));
    }
}
