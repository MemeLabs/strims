// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The node's and the controls' counters as the Debug service's snapshot
//! (`strims.debug.v1.Metrics`).
//!
//! [`ProfileMetrics`] is the profile's [`MetricsSource`]: one node query,
//! the controls' counters and the platform's heap counters per snapshot.
//! Each function below takes a struct apart field by field, so a counter
//! the snapshot lacks fails the build. Durations become nanoseconds, bins
//! their `u64`, and swarm, peer and network keys their slotmap bits.

use strims_chat::{ChatControl, WhisperControl};
use strims_debug::MetricsSource;
use strims_directory::DirectoryControl;
use strims_kv::Kv;
use strims_network::NetworkControl;
use strims_ppspp::{
    ChannelMetrics, ChannelSchedulerStats, ChannelStats, MetricsSnapshot, PeerStats, PushMode,
    PushStats, RequestControllerKind, SwarmSchedulerStats, TransferMetricsSnapshot,
};
use strims_proto::debug::v1;
use strims_proto::service::Error as RpcError;
use strims_replication::ReplicationControl;
use strims_rpc::Timer;
use strims_transfer::TransferControl;
use strims_video::{VideoControl, VideoHost};
use strims_vnic::{HostMetrics, LinkKind, LinkMetrics};

use super::{MemoryMetrics, NodeHost};
use crate::{
    ChannelInfo, DriverMetrics, LinkInfo, NetworkInfo, Node, NodeMetrics, RunnerMetrics,
    RunnerPeerInfo, StoreInfo, SwarmInfo, WriterInfo,
};

/// A running profile's node, controls and heap counters, as the Debug
/// service's snapshots report them.
pub(crate) struct ProfileMetrics<K: Kv, T: Timer, V: VideoHost> {
    pub(crate) node: Node,
    pub(crate) network: NetworkControl<K, NodeHost, T>,
    pub(crate) transfers: TransferControl<NodeHost, T>,
    pub(crate) replication: ReplicationControl<K, NodeHost, T>,
    pub(crate) directory: DirectoryControl<K, NodeHost, T>,
    pub(crate) chat: ChatControl<K, NodeHost, T>,
    pub(crate) whispers: WhisperControl<K, NodeHost, T>,
    pub(crate) video: VideoControl<K, NodeHost, V, T>,
    pub(crate) memory: Option<fn() -> MemoryMetrics>,
}

impl<K: Kv, T: Timer, V: VideoHost> MetricsSource for ProfileMetrics<K, T, V> {
    /// Fails when the node has stopped.
    async fn metrics(&self) -> Result<v1::Metrics, RpcError> {
        let n = self
            .node
            .metrics()
            .await
            .map_err(|e| RpcError::new(0, e.to_string()))?;
        Ok(v1::Metrics {
            memory: self.memory.map(|read| memory(read())),
            node: Some(node(n)),
            network: Some(network(self.network.metrics())),
            transfer: Some(transfer(self.transfers.metrics())),
            replication: Some(replication(self.replication.metrics())),
            directory: self
                .directory
                .network_metrics()
                .into_iter()
                .map(|(network_key, m)| v1::DirectoryNetworkMetrics {
                    network_key,
                    metrics: Some(directory(m)),
                })
                .collect(),
            chat: Some(chat(self.chat.metrics())),
            whispers: Some(whispers(self.whispers.metrics())),
            video: Some(video(self.video.metrics())),
            ..v1::Metrics::default()
        })
    }
}

fn count(n: usize) -> u64 {
    n as u64
}

fn key(k: impl slotmap::Key) -> u64 {
    k.data().as_ffi()
}

fn memory(m: MemoryMetrics) -> v1::MemoryMetrics {
    let MemoryMetrics {
        live_bytes,
        peak_bytes,
        allocated_bytes,
        allocations,
        deallocations,
        reallocations,
        linear_memory_bytes,
    } = m;
    v1::MemoryMetrics {
        live_bytes,
        peak_bytes,
        allocated_bytes,
        allocations,
        deallocations,
        reallocations,
        linear_memory_bytes,
    }
}

fn node(m: NodeMetrics) -> v1::NodeMetrics {
    let NodeMetrics {
        host: h,
        vpn: v,
        driver: d,
        links,
        runner: r,
        networks,
    } = m;
    v1::NodeMetrics {
        host: Some(host(h)),
        vpn: Some(vpn(v)),
        driver: Some(driver(d)),
        links: links.into_iter().map(link).collect(),
        runner: Some(runner(r)),
        networks: networks.into_iter().map(network_info).collect(),
    }
}

fn host(m: HostMetrics) -> v1::HostMetrics {
    let HostMetrics {
        links_active,
        frame_read_count,
        frame_read_bytes,
        frame_handler_not_found_count,
        frame_write_count,
        frame_write_bytes,
        keepalive_write_count,
        keepalive_read_count,
        keepalive_write_bytes: _,
        probe_write_count: _,
        probe_read_count: _,
        echo_write_count: _,
        echo_hold_ms: _,
        echo_read_count: _,
        echo_unknown_count: _,
        path_negative_rtt_count: _,
        handshake_timeout_count,
        handshake_evict_count,
    } = m;
    v1::HostMetrics {
        links_active,
        frame_read_count,
        frame_read_bytes,
        frame_handler_not_found_count,
        frame_write_count,
        frame_write_bytes,
        keepalive_write_count,
        keepalive_read_count,
        handshake_timeout_count,
        handshake_evict_count,
    }
}

fn vpn(m: strims_vpn::Metrics) -> v1::VpnMetrics {
    let strims_vpn::Metrics {
        messages_sent,
        messages_relayed,
        frames_received,
        frames_queued,
        frames_sent,
        messages_delivered,
        duplicates,
        malformed,
        trailer_mismatch,
        compressed_dropped,
        verify_failures,
        unverified_dropped,
        hop_limit,
        link_queue_full,
        too_large,
        no_handler,
        decrypt_failures,
        handler_errors,
        deliveries_dropped,
        dedup_evictions,
        shorter_relays,
        learned_routes,
        fallback_routes,
        link_announcements_sent,
        link_announcements_received,
        records_pushed,
    } = m;
    v1::VpnMetrics {
        messages_sent,
        messages_relayed,
        frames_received,
        frames_queued,
        frames_sent,
        messages_delivered,
        duplicates,
        malformed,
        trailer_mismatch,
        compressed_dropped,
        verify_failures,
        unverified_dropped,
        hop_limit,
        link_queue_full,
        too_large,
        no_handler,
        decrypt_failures,
        handler_errors,
        deliveries_dropped,
        dedup_evictions,
        shorter_relays,
        learned_routes,
        fallback_routes,
        link_announcements_sent,
        link_announcements_received,
        records_pushed,
    }
}

fn driver(m: DriverMetrics) -> v1::DriverMetrics {
    let DriverMetrics {
        tcp_dials,
        tcp_dial_errors,
        ws_dials,
        ws_dial_errors,
        accepted,
        accept_errors,
        transport_failures,
        links_closed,
        bytes_read,
        bytes_written,
        events_dropped,
        results_dropped,
        vpn_frame_errors,
        transfer_errors,
        rpc_bytes_read,
        rpc_bytes_written,
        rpc_overflows,
        broker_bytes_read,
        broker_bytes_written,
        broker_overflows,
        vpn_port_drops,
        vpn_send_errors,
        commands,
        queries,
        query_wakes,
        loop_iterations,
        webrtc_connections,
        webrtc_opened,
        webrtc_closed,
        udp_datagrams_read,
        udp_bytes_read,
        udp_datagrams_written,
        udp_bytes_written,
        udp_send_dropped,
        udp_unrouted,
    } = m;
    v1::DriverMetrics {
        tcp_dials,
        tcp_dial_errors,
        ws_dials,
        ws_dial_errors,
        accepted,
        accept_errors,
        transport_failures,
        links_closed,
        bytes_read,
        bytes_written,
        events_dropped,
        results_dropped,
        vpn_frame_errors,
        transfer_errors,
        rpc_bytes_read,
        rpc_bytes_written,
        rpc_overflows,
        broker_bytes_read,
        broker_bytes_written,
        broker_overflows,
        vpn_port_drops,
        vpn_send_errors,
        commands,
        queries,
        query_wakes,
        loop_iterations,
        webrtc_connections,
        webrtc_opened,
        webrtc_closed,
        udp_datagrams_read,
        udp_bytes_read,
        udp_datagrams_written,
        udp_bytes_written,
        udp_send_dropped,
        udp_unrouted,
    }
}

fn link_kind(k: LinkKind) -> v1::LinkKind {
    match k {
        LinkKind::Tcp => v1::LinkKind::Tcp,
        LinkKind::Ws => v1::LinkKind::Ws,
        LinkKind::WebRtc => v1::LinkKind::Webrtc,
        LinkKind::Other => v1::LinkKind::Other,
    }
}

fn link_metrics(m: LinkMetrics) -> v1::LinkMetrics {
    let LinkMetrics {
        read_bytes,
        read_count,
        write_bytes,
        write_count,
    } = m;
    v1::LinkMetrics {
        read_bytes,
        read_count,
        write_bytes,
        write_count,
    }
}

fn link(l: LinkInfo) -> v1::LinkInfo {
    let LinkInfo {
        host_id,
        kind,
        metrics,
        dialed,
        mtu,
        transmit_pending,
        input_buffered,
        write_blocked,
        buffered_amount,
        path: _,
    } = l;
    v1::LinkInfo {
        host_id: host_id.map(|id| id.to_bytes().to_vec()).unwrap_or_default(),
        kind: link_kind(kind).into(),
        metrics: Some(link_metrics(metrics)),
        dialed,
        mtu: count(mtu),
        transmit_pending: count(transmit_pending),
        input_buffered: count(input_buffered),
        write_blocked,
        buffered_amount: count(buffered_amount),
    }
}

fn runner(m: RunnerMetrics) -> v1::RunnerMetrics {
    let RunnerMetrics { swarms, peers } = m;
    v1::RunnerMetrics {
        swarms: swarms.into_iter().map(swarm).collect(),
        peers: peers.into_iter().map(peer).collect(),
    }
}

fn swarm(s: SwarmInfo) -> v1::SwarmInfo {
    let SwarmInfo {
        swarm,
        id,
        label,
        channels,
        store: st,
        scheduler,
        push: p,
        parent_hosts: _,
        readers,
        readers_waiting,
        publishers,
    } = s;
    v1::SwarmInfo {
        swarm: key(swarm),
        id: id.as_bytes().to_vec(),
        label,
        channels: count(channels),
        store: Some(store(st)),
        scheduler: scheduler.map(swarm_scheduler),
        push: p.map(push),
        readers: count(readers),
        readers_waiting: count(readers_waiting),
        publishers: count(publishers),
    }
}

fn store(s: StoreInfo) -> v1::StoreInfo {
    let StoreInfo {
        ready,
        tail,
        next,
        window,
        readable_seq,
    } = s;
    v1::StoreInfo {
        ready,
        tail: tail.into(),
        next: next.into(),
        window,
        readable_seq,
    }
}

fn swarm_scheduler(s: SwarmSchedulerStats) -> v1::SwarmSchedulerStats {
    let SwarmSchedulerStats {
        first_chunk_set,
        have_bin_max,
        peer_max_have_bin,
        wasted_chunks,
        requests_timed_out: _,
        bin_time_nodes,
        have_map_size,
        request_map_size,
        stream_sources,
        subscribers,
        chunks_sent,
        pushed_chunks,
        distinct_chunks_sent,
        answers_queued: _,
        answers_refused: _,
        push_wait: _,
        answer_wait: _,
    } = s;
    v1::SwarmSchedulerStats {
        first_chunk_set,
        have_bin_max: have_bin_max.into(),
        peer_max_have_bin: peer_max_have_bin.into(),
        wasted_chunks,
        bin_time_nodes: count(bin_time_nodes),
        have_map_size: count(have_map_size),
        request_map_size: count(request_map_size),
        stream_sources: count(stream_sources),
        subscribers: count(subscribers),
        chunks_sent,
        pushed_chunks,
        distinct_chunks_sent,
    }
}

fn push_mode(m: PushMode) -> v1::PushMode {
    match m {
        PushMode::Default => push_mode(PushMode::DEFAULT),
        PushMode::Off => v1::PushMode::Off,
        PushMode::EagerHave => v1::PushMode::EagerHave,
        PushMode::Hybrid => v1::PushMode::Hybrid,
    }
}

fn push(s: PushStats) -> v1::PushStats {
    let PushStats {
        mode,
        parents,
        parent_lag,
        upload_rate,
        upload_busy_permille,
        interior,
        children,
        expected,
        hits,
        misses,
        late,
        unsolicited,
        requests_sent,
        opens,
        declined,
        open_timeouts,
        shed,
        pruned,
        switched,
        accepted,
        rejected,
        sheds_sent,
        stale_drops,
        suppressed,
        upload_capacity: _,
        upload_queue_delay: _,
        seed_budget: _,
        seed_window: _,
        seed_copies: _,
        seed_first_wave: _,
        seed_first_wave_capacity: _,
    } = s;
    let streams = parents.len().max(parent_lag.len());
    let parents = (0..streams)
        .map(|i| {
            let lag = parent_lag.get(i).copied().flatten();
            v1::PushParent {
                peer: parents.get(i).copied().flatten().map(key),
                lag: lag.map(|(l, _)| l.as_nanos()),
                deadline: lag.map(|(_, d)| d.as_nanos()),
            }
        })
        .collect();
    v1::PushStats {
        mode: push_mode(mode).into(),
        parents,
        upload_rate,
        upload_busy_permille,
        interior: interior.map(u32::from),
        children: count(children),
        expected,
        hits,
        misses,
        late,
        unsolicited,
        requests_sent,
        opens,
        declined,
        open_timeouts,
        shed,
        pruned,
        switched,
        accepted,
        rejected,
        sheds_sent,
        stale_drops,
        suppressed,
    }
}

fn peer(p: RunnerPeerInfo) -> v1::RunnerPeerInfo {
    let RunnerPeerInfo {
        host_id,
        qos_weight,
        queues,
        metrics,
        writer: w,
        channels,
    } = p;
    v1::RunnerPeerInfo {
        host_id: host_id.to_bytes().to_vec(),
        qos_weight,
        queues: Some(peer_stats(queues)),
        metrics: Some(snapshot(metrics)),
        writer: w.map(writer),
        channels: channels.into_iter().map(channel).collect(),
    }
}

fn peer_stats(s: PeerStats) -> v1::PeerStats {
    let PeerStats {
        wants_transmit,
        queued_channels,
        queued_data,
        queue_ring,
        dropped,
        path: _,
    } = s;
    v1::PeerStats {
        wants_transmit,
        queued_channels,
        queued_data: queued_data.into_iter().map(count).collect(),
        queue_ring: queue_ring.into_iter().map(count).collect(),
        dropped,
    }
}

fn transfer_snapshot(s: TransferMetricsSnapshot) -> v1::TransferMetricsSnapshot {
    let TransferMetricsSnapshot { count, rate } = s;
    v1::TransferMetricsSnapshot { count, rate }
}

fn snapshot(s: MetricsSnapshot) -> v1::MetricsSnapshot {
    let MetricsSnapshot { read, write } = s;
    v1::MetricsSnapshot {
        read: Some(transfer_snapshot(read)),
        write: Some(transfer_snapshot(write)),
    }
}

fn writer(w: WriterInfo) -> v1::WriterInfo {
    let WriterInfo {
        weight,
        backlogged,
        blocked,
    } = w;
    v1::WriterInfo {
        weight,
        backlogged,
        blocked,
    }
}

fn channel(c: ChannelInfo) -> v1::ChannelInfo {
    let ChannelInfo {
        swarm,
        channel,
        peer_channel,
        metrics,
        stats,
    } = c;
    v1::ChannelInfo {
        swarm: key(swarm),
        channel,
        peer_channel,
        metrics: Some(snapshot(metrics)),
        stats: stats.map(channel_stats),
    }
}

fn channel_stats(s: ChannelStats) -> v1::ChannelStats {
    let ChannelStats {
        scheduler,
        inbound,
        outbound,
    } = s;
    v1::ChannelStats {
        scheduler: Some(channel_scheduler(scheduler)),
        inbound: Some(channel_metrics(inbound)),
        outbound: Some(channel_metrics(outbound)),
    }
}

fn controller(k: RequestControllerKind) -> v1::RequestControllerKind {
    match k {
        RequestControllerKind::Default => v1::RequestControllerKind::Default,
        RequestControllerKind::Ledbat => v1::RequestControllerKind::Ledbat,
        RequestControllerKind::BbrLike => v1::RequestControllerKind::BbrLike,
    }
}

fn channel_scheduler(s: ChannelSchedulerStats) -> v1::ChannelSchedulerStats {
    let ChannelSchedulerStats {
        handshake_received,
        choked,
        controller: kind,
        cwnd,
        flight_size,
        cto,
        rtt_mean,
        min_rtt,
        delivery_rate,
        pending_requests,
        request_time_nodes,
        extra_messages,
        peer_max_have_bin,
        peer_have_map_size,
        have_map_size,
        cancel_map_size,
    } = s;
    v1::ChannelSchedulerStats {
        handshake_received,
        choked,
        controller: controller(kind).into(),
        cwnd,
        flight_size,
        cto: cto.as_nanos(),
        rtt_mean: rtt_mean.as_nanos(),
        min_rtt: min_rtt.as_nanos(),
        delivery_rate,
        pending_requests: count(pending_requests),
        request_time_nodes: count(request_time_nodes),
        extra_messages: count(extra_messages),
        peer_max_have_bin: peer_max_have_bin.into(),
        peer_have_map_size: count(peer_have_map_size),
        have_map_size: count(have_map_size),
        cancel_map_size: count(cancel_map_size),
    }
}

fn channel_metrics(m: ChannelMetrics) -> v1::ChannelMetrics {
    let ChannelMetrics {
        handshake,
        restart,
        data,
        chunk,
        integrity,
        signed_integrity,
        ack,
        have,
        request,
        cancel,
        choke,
        unchoke,
        ping,
        pong,
        stream_request,
        stream_cancel,
        stream_open,
        stream_close,
        announce,
        data_bytes,
        overhead_bytes,
        invalid_data,
        invalid_chunk,
        invalid_bytes,
        rejected,
        stale_stream_open,
        stale_data,
    } = m;
    v1::ChannelMetrics {
        handshake,
        restart,
        data,
        chunk,
        integrity,
        signed_integrity,
        ack,
        have,
        request,
        cancel,
        choke,
        unchoke,
        ping,
        pong,
        stream_request,
        stream_cancel,
        stream_open,
        stream_close,
        data_bytes,
        overhead_bytes,
        invalid_data,
        invalid_chunk,
        invalid_bytes,
        rejected,
        stale_stream_open,
        stale_data,
        announce,
    }
}

fn network_info(n: NetworkInfo) -> v1::NetworkInfo {
    let NetworkInfo {
        network,
        links,
        routed_links,
        queued,
        topology,
    } = n;
    v1::NetworkInfo {
        network: key(network),
        links: count(links),
        routed_links: count(routed_links),
        queued: count(queued),
        topology: count(topology),
    }
}

fn network(m: strims_network::Metrics) -> v1::NetworkMetrics {
    let strims_network::Metrics {
        runs,
        runs_completed,
        runs_failed,
        cuckoo_retries,
        runs_paced,
        links_paired,
        links_opened,
        bindings_refused,
        broker_bytes_read,
        broker_bytes_written,
        call_errors,
        host_errors,
    } = m;
    v1::NetworkMetrics {
        runs,
        runs_completed,
        runs_failed,
        cuckoo_retries,
        runs_paced,
        links_paired,
        links_opened,
        bindings_refused,
        broker_bytes_read,
        broker_bytes_written,
        call_errors,
        host_errors,
    }
}

fn transfer(m: strims_transfer::Metrics) -> v1::TransferMetrics {
    let strims_transfer::Metrics {
        announces_sent,
        announces_accepted,
        announces_received,
        announces_answered,
        call_errors,
        runs,
        stops,
        run_errors,
        closes_sent,
        closes_received,
        index_publishes,
        searches,
        search_errors,
        connects,
        host_errors,
    } = m;
    v1::TransferMetrics {
        announces_sent,
        announces_accepted,
        announces_received,
        announces_answered,
        call_errors,
        runs,
        stops,
        run_errors,
        closes_sent,
        closes_received,
        index_publishes,
        searches,
        search_errors,
        connects,
        host_errors,
    }
}

fn replication(m: strims_replication::Metrics) -> v1::ReplicationMetrics {
    let strims_replication::Metrics {
        leases,
        opens,
        bootstraps_sent,
        bootstraps_received,
        events_sent,
        events_received,
        syncs_sent,
        syncs_received,
        logs_sent,
        logs_received,
        refused,
        peer_errors,
        store_errors,
        gc_runs,
        logs_collected,
        publishes,
        lookups,
        connects,
    } = m;
    v1::ReplicationMetrics {
        leases,
        opens,
        bootstraps_sent,
        bootstraps_received,
        events_sent,
        events_received,
        syncs_sent,
        syncs_received,
        logs_sent,
        logs_received,
        refused,
        peer_errors,
        store_errors,
        gc_runs,
        logs_collected,
        publishes,
        lookups,
        connects,
    }
}

fn directory(m: strims_directory::Metrics) -> v1::DirectoryMetrics {
    let strims_directory::Metrics {
        server,
        view,
        serves,
        pings,
        ping_errors,
        discontinuities,
        asset_bundles,
        snippets_served,
        snippets_refused,
        embed_batches,
        embed_batch_errors,
        host_errors,
        store_errors,
    } = m;
    v1::DirectoryMetrics {
        server: server.map(directory_server),
        view: Some(directory_view(view)),
        serves,
        pings,
        ping_errors,
        discontinuities,
        asset_bundles,
        snippets_served,
        snippets_refused,
        embed_batches,
        embed_batch_errors,
        host_errors,
        store_errors,
    }
}

fn directory_server(m: strims_directory::ServerMetrics) -> v1::DirectoryServerMetrics {
    let strims_directory::ServerMetrics {
        broadcasts,
        records,
        events,
        refreshes,
        refreshed,
        listings_created,
        listings_removed,
        users_created,
        users_removed,
        sessions_created,
        sessions_expired,
        sessions_evicted,
        sessions_displaced,
        refused,
        subscriptions,
        snippet_responses,
        stale_responses,
        snippets_shown,
        snippets_unverified,
        snippets_stale,
        snippets_ended,
        snippet_timeouts,
        snippets_rejected,
        snippet_floods,
        snippets_oversized,
        embed_loads,
        embed_snippets_shown,
        embed_snippets_rejected,
        stale_embed_batches,
    } = m;
    v1::DirectoryServerMetrics {
        broadcasts,
        records,
        events,
        refreshes,
        refreshed,
        listings_created,
        listings_removed,
        users_created,
        users_removed,
        sessions_created,
        sessions_expired,
        sessions_evicted,
        sessions_displaced,
        refused,
        subscriptions,
        snippet_responses,
        stale_responses,
        snippets_shown,
        snippets_unverified,
        snippets_stale,
        snippets_ended,
        snippet_timeouts,
        snippets_rejected,
        snippet_floods,
        snippets_oversized,
        embed_loads,
        embed_snippets_shown,
        embed_snippets_rejected,
        stale_embed_batches,
    }
}

fn directory_view(m: strims_directory::ViewMetrics) -> v1::DirectoryViewMetrics {
    let strims_directory::ViewMetrics {
        records,
        events,
        resets,
        malformed_records,
        malformed_events,
        unknown_listings,
        listings_dropped,
        users_dropped,
        ids_dropped,
        malformed_snippets,
        images_dropped,
        snippets_dropped,
        oversized_listings,
        events_dropped,
    } = m;
    v1::DirectoryViewMetrics {
        records,
        events,
        resets,
        malformed_records,
        malformed_events,
        unknown_listings,
        listings_dropped,
        users_dropped,
        ids_dropped,
        malformed_snippets,
        images_dropped,
        snippets_dropped,
        oversized_listings,
        events_dropped,
    }
}

fn chat(m: strims_chat::Metrics) -> v1::ChatMetrics {
    let strims_chat::Metrics {
        serves,
        reads,
        events_written,
        asset_bundles,
        asset_resets,
        assets_too_large,
        listing_publishes,
        listing_errors,
        local_calls,
        remote_calls,
        servers_dropped,
        streams_refused,
        cache_loads,
        cache_refusals,
        cache_writes,
        write_errors,
        host_errors,
        store_errors,
    } = m;
    v1::ChatMetrics {
        serves,
        reads,
        events_written,
        asset_bundles,
        asset_resets,
        assets_too_large,
        listing_publishes,
        listing_errors,
        local_calls,
        remote_calls,
        servers_dropped,
        streams_refused,
        cache_loads,
        cache_refusals,
        cache_writes,
        write_errors,
        host_errors,
        store_errors,
    }
}

fn whispers(m: strims_chat::WhisperMetrics) -> v1::WhisperMetrics {
    let strims_chat::WhisperMetrics {
        received,
        duplicates,
        refused,
        rate_limited,
        enqueued,
        attempts,
        delivered,
        failed,
        leases,
        ports_opened,
        pending,
        in_flight,
        host_errors,
        store_errors,
    } = m;
    v1::WhisperMetrics {
        received,
        duplicates,
        refused,
        rate_limited,
        enqueued,
        attempts,
        delivered,
        failed,
        leases,
        ports_opened,
        pending: count(pending),
        in_flight: count(in_flight),
        host_errors,
        store_errors,
    }
}

fn video(m: strims_video::Metrics) -> v1::VideoMetrics {
    let strims_video::Metrics {
        store_lags,
        store_errors,
        host_errors,
        dropped_records,
        write_errors,
        reopens,
        hls_streams,
        hls_segments,
        hls_skipped_records,
        autoseed_skipped,
        ingress: i,
    } = m;
    v1::VideoMetrics {
        store_lags,
        store_errors,
        host_errors,
        dropped_records,
        write_errors,
        reopens,
        hls_streams,
        hls_segments,
        hls_skipped_records,
        autoseed_skipped,
        ingress: Some(ingress(i)),
    }
}

fn ingress(m: strims_video::IngressMetrics) -> v1::IngressMetrics {
    let strims_video::IngressMetrics {
        listens,
        listen_errors,
        connections,
        rejected_connections,
        refused,
        sessions,
        replaced,
        segments,
        bytes,
        dropped_units,
        metadata,
        deadlines,
        media_errors,
        write_errors,
    } = m;
    v1::IngressMetrics {
        listens,
        listen_errors,
        connections,
        rejected_connections,
        refused,
        sessions,
        replaced,
        segments,
        bytes,
        dropped_units,
        metadata,
        deadlines,
        media_errors,
        write_errors,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use slotmap::KeyData;
    use strims_base::time::Duration;
    use strims_binmap::Bin;
    use strims_kademlia::ID;
    use strims_ppspp::{PeerKey, SwarmId, SwarmKey};
    use strims_vpn::NetworkId;

    use super::*;

    fn scheduler() -> ChannelSchedulerStats {
        ChannelSchedulerStats {
            handshake_received: true,
            choked: false,
            controller: RequestControllerKind::BbrLike,
            cwnd: -2,
            flight_size: 3,
            cto: Duration::from_millis(250),
            rtt_mean: Duration::from_millis(40),
            min_rtt: Duration::from_millis(30),
            delivery_rate: 1000,
            pending_requests: 4,
            request_time_nodes: 5,
            extra_messages: 6,
            peer_max_have_bin: Bin::NONE,
            peer_have_map_size: 7,
            have_map_size: 8,
            cancel_map_size: 9,
        }
    }

    #[test]
    fn a_snapshot_mirrors_the_node_metrics() {
        let swarm_key: SwarmKey = KeyData::from_ffi(0x1_0000_0003).into();
        let peer_key: PeerKey = KeyData::from_ffi(0x3_0000_0005).into();
        let network: NetworkId = KeyData::from_ffi(0x1_0000_0001).into();
        let host_id = ID::from_bytes(&[9; 32]);
        let m = NodeMetrics {
            links: vec![LinkInfo {
                host_id: None,
                kind: LinkKind::WebRtc,
                metrics: LinkMetrics {
                    read_bytes: 10,
                    ..LinkMetrics::default()
                },
                dialed: true,
                mtu: 1200,
                transmit_pending: 0,
                input_buffered: 0,
                write_blocked: false,
                buffered_amount: 64,
                path: None,
            }],
            runner: RunnerMetrics {
                swarms: vec![SwarmInfo {
                    parent_hosts: Vec::new(),
                    swarm: swarm_key,
                    id: SwarmId::new(&[1, 2, 3]),
                    label: "mock".into(),
                    channels: 1,
                    store: StoreInfo {
                        ready: false,
                        tail: Bin::NONE,
                        next: Bin(6),
                        window: 16,
                        readable_seq: 2,
                    },
                    scheduler: None,
                    push: Some(PushStats {
                        mode: PushMode::Hybrid,
                        parents: vec![Some(peer_key), None],
                        parent_lag: vec![
                            Some((Duration::from_millis(10), Duration::from_millis(50))),
                            None,
                        ],
                        interior: Some(3),
                        ..PushStats::default()
                    }),
                    readers: 1,
                    readers_waiting: 0,
                    publishers: 0,
                }],
                peers: vec![RunnerPeerInfo {
                    host_id,
                    qos_weight: Some(4),
                    queues: PeerStats {
                        queued_data: [1, 2],
                        ..PeerStats::default()
                    },
                    metrics: MetricsSnapshot::default(),
                    writer: None,
                    channels: vec![ChannelInfo {
                        swarm: swarm_key,
                        channel: 1,
                        peer_channel: 2,
                        metrics: MetricsSnapshot {
                            read: TransferMetricsSnapshot {
                                count: 100,
                                rate: 10,
                            },
                            write: TransferMetricsSnapshot::default(),
                        },
                        stats: Some(ChannelStats {
                            scheduler: scheduler(),
                            inbound: ChannelMetrics {
                                data_bytes: 100,
                                ..ChannelMetrics::default()
                            },
                            outbound: ChannelMetrics::default(),
                        }),
                    }],
                }],
            },
            networks: vec![NetworkInfo {
                network,
                links: 1,
                routed_links: 1,
                queued: 0,
                topology: 2,
            }],
            ..NodeMetrics::default()
        };

        let n = node(m);
        let link = &n.links[0];
        assert!(link.host_id.is_empty());
        assert_eq!(link.kind(), v1::LinkKind::Webrtc);
        assert_eq!((link.mtu, link.buffered_amount), (1200, 64));
        assert_eq!(link.metrics.unwrap().read_bytes, 10);

        let runner = n.runner.unwrap();
        let swarm = &runner.swarms[0];
        assert_eq!(swarm.swarm, 0x1_0000_0003);
        assert_eq!(
            (swarm.id.as_slice(), swarm.label.as_str()),
            (&[1, 2, 3][..], "mock")
        );
        let store = swarm.store.unwrap();
        assert_eq!((store.tail, store.next), (u64::MAX, 6));
        assert!(swarm.scheduler.is_none());
        let push = swarm.push.as_ref().unwrap();
        assert_eq!(push.mode(), v1::PushMode::Hybrid);
        assert_eq!(push.interior, Some(3));
        assert_eq!(
            push.parents,
            vec![
                v1::PushParent {
                    peer: Some(0x3_0000_0005),
                    lag: Some(10_000_000),
                    deadline: Some(50_000_000),
                },
                v1::PushParent::default(),
            ]
        );

        let peer = &runner.peers[0];
        assert_eq!(peer.host_id, vec![9; 32]);
        assert_eq!(peer.qos_weight, Some(4));
        assert!(peer.writer.is_none());
        assert_eq!(peer.queues.as_ref().unwrap().queued_data, vec![1, 2]);
        let channel = &peer.channels[0];
        assert_eq!(channel.swarm, swarm.swarm);
        assert_eq!(channel.metrics.unwrap().read.unwrap().count, 100);
        let stats = channel.stats.unwrap();
        assert_eq!(stats.inbound.unwrap().data_bytes, 100);
        let scheduler = stats.scheduler.unwrap();
        assert_eq!(scheduler.controller(), v1::RequestControllerKind::BbrLike);
        assert_eq!(
            (scheduler.cwnd, scheduler.cto, scheduler.rtt_mean),
            (-2, 250_000_000, 40_000_000)
        );
        assert_eq!(scheduler.peer_max_have_bin, u64::MAX);

        assert_eq!(n.networks[0].network, 0x1_0000_0001);
        assert_eq!(n.networks[0].topology, 2);
    }
}
