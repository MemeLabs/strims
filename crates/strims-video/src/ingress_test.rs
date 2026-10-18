// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::config::IngressSettings;

#[test]
fn an_ingest_uri_carries_its_options() {
    let o = ingress_options();
    let id = SwarmId::new(&[7; 32]);
    let uri = Uri::new(id.clone(), o.uri_options()).to_string();
    assert_eq!(
        uri,
        format!("magnet:?xt=urn:ppspp:{id}&x.im=2&x.hf=4&x.sa=2&x.cs=1024&x.ps=32&x.sc=16")
    );
    assert_eq!(o.live_window, 16 * 1024);
}

#[test]
fn the_per_address_cap_counts_hosts() {
    for (peer, host) in [
        ("10.0.0.1:1935", "10.0.0.1"),
        ("[::1]:5000", "::1"),
        ("[fe80::1%eth0]:1", "fe80::1%eth0"),
        ("host.example:80", "host.example"),
        ("::1", "::1"),
        ("10.0.0.1", "10.0.0.1"),
        ("10.0.0.1:", "10.0.0.1:"),
        ("", ""),
    ] {
        assert_eq!(peer_host(peer), host, "{peer}");
    }
}

#[test]
fn the_rtmp_server_takes_the_ingress_bounds() {
    let s = IngressSettings::default().rtmp;
    assert_eq!(rtmp_config(&s), RtmpConfig::default());
    let c = rtmp_config(&RtmpSettings {
        max_queued_output: 1234,
        chunk_size: 512,
        ..s
    });
    assert_eq!((c.max_output, c.chunk_size), (1234, 512));
}

#[test]
fn the_publisher_hears_why_the_server_ended_its_stream() {
    let idle = End::Deadline(Deadline::Idle(Duration::from_secs(30)));
    assert_eq!(idle.to_string(), "no media for 30 s");
    assert!(idle.tells_peer() && idle.flushes());
    let media = End::Media("HEVC (hvc1) is not supported; send H.264 with AAC".into());
    assert_eq!(
        media.to_string(),
        "HEVC (hvc1) is not supported; send H.264 with AAC"
    );
    assert!(media.tells_peer());
    for quiet in [End::Unpublished, End::ServerClosed, End::PeerClosed] {
        assert!(!quiet.tells_peer(), "{quiet}");
    }
    assert!(!End::PeerClosed.flushes());
    assert!(!End::Io("reset".into()).flushes());
}

#[test]
fn a_live_snippet_keeps_the_channels_fields() {
    let c = VideoChannel {
        id: 3,
        directory_listing_snippet: Some(ListingSnippet {
            title: "title".into(),
            channel_name: "name".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let s = live_snippet(&c, 1_700_000_000);
    assert_eq!(
        (s.title.as_str(), s.channel_name.as_str()),
        ("title", "name")
    );
    assert_eq!((s.start_time, s.live), (1_700_000_000, true));
}
