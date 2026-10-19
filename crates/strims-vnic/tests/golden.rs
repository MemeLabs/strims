// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vectors (`golden/vectors.txt`): frames, `FrameWriter` batching,
//! `TCPMuxInit`, the AES link handshake and ciphertext, a host's
//! `PeerInit`, listener URIs and the WebRTC session description JSON.
//! Values are hex, and lists are comma-separated. `STRIMS_GOLDEN=update`
//! rewrites the vectors from the code (see `strims-golden`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_crypto::{
    certificate_parent, key_from_seed, key_to_curve25519, public_key_to_curve25519,
    verify_certificate,
};
use strims_golden::Golden;
use strims_proto::vnic::v1::PeerInit;
use strims_vnic::stream::{MessageReader, decode_stream_message, encode_stream_message};
use strims_vnic::tcp::{encode_tcp_mux_init, format_tcp_uri};
use strims_vnic::webrtc::{SdpType, SessionDescription};
use strims_vnic::ws::{WsListenAddress, format_ws_uri};
use strims_vnic::{
    AesLinkHandshake, Event, FRAME_HEADER_LEN, Frame, FrameDecoder, FrameWriter, Host, HostConfig,
    LinkConfig, LinkKind, TcpMuxAcceptor,
};

fn open() -> Golden {
    Golden::open(concat!(env!("CARGO_MANIFEST_DIR"), "/golden/vectors.txt"))
}

/// The entries named `name`, of which there must be some.
fn entries(g: &Golden, name: &str) -> Vec<usize> {
    let es = g.all(name);
    assert!(!es.is_empty(), "no {name} vectors");
    es
}

fn num<T: std::str::FromStr>(g: &Golden, e: usize, k: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    g.str(e, k).parse().unwrap()
}

fn text(g: &Golden, e: usize, k: &str) -> String {
    String::from_utf8(g.bytes(e, k)).unwrap()
}

fn arr<const N: usize>(b: Vec<u8>) -> [u8; N] {
    b.try_into().unwrap()
}

fn list(g: &Golden, e: usize, k: &str) -> Vec<Vec<u8>> {
    let s = g.str(e, k);
    if s.is_empty() {
        return vec![];
    }
    s.split(',').map(|x| hex::decode(x).unwrap()).collect()
}

fn hex_list(items: &[Vec<u8>]) -> String {
    items.iter().map(hex::encode).collect::<Vec<_>>().join(",")
}

#[test]
fn frames() {
    let mut g = open();
    for e in entries(&g, "frame") {
        let port: u16 = num(&g, e, "port");
        let body = g.bytes(e, "body");
        let f = Frame { port, body: &body };
        let mut wire = vec![0u8; f.encoded_len()];
        assert_eq!(f.encode(&mut wire).unwrap(), wire.len());
        assert_eq!(Frame::decode(&wire).unwrap(), (f, wire.len()));
        for split in 0..=wire.len() {
            let mut d = FrameDecoder::new();
            let mut got = vec![];
            d.decode(&wire[..split], &mut |f| got.push((f.port, f.body.to_vec())));
            d.decode(&wire[split..], &mut |f| got.push((f.port, f.body.to_vec())));
            assert_eq!(got, vec![(port, body.clone())]);
        }
        g.check_hex(e, "wire", &wire);
    }
}

/// `ops` are `w:<data>` (write), `wf:<data>` (write a frame) and `f`
/// (flush); `writes` are the frames on a link with the given MTU.
#[test]
fn frame_writer_batching() {
    let mut g = open();
    for e in entries(&g, "framewriter") {
        let mtu: usize = num(&g, e, "mtu");
        let port: u16 = num(&g, e, "port");
        let mut w = FrameWriter::new(mtu - FRAME_HEADER_LEN, 1 << 20);
        for op in g.str(e, "ops").split(',') {
            match op.split_once(':') {
                Some(("w", d)) => {
                    let d = hex::decode(d).unwrap();
                    assert_eq!(w.write(&d).unwrap(), d.len());
                }
                Some(("wf", d)) => {
                    w.write_frame(&hex::decode(d).unwrap()).unwrap();
                }
                None if op == "f" => w.flush().unwrap(),
                _ => panic!("op {op}"),
            }
        }
        let mut writes = vec![];
        let mut body = vec![0u8; mtu - FRAME_HEADER_LEN];
        loop {
            let n = w.fill(&mut body);
            if n == 0 {
                break;
            }
            let mut frame = vec![0u8; FRAME_HEADER_LEN + n];
            Frame {
                port,
                body: &body[..n],
            }
            .encode(&mut frame)
            .unwrap();
            writes.push(frame);
        }
        g.check(e, "writes", hex_list(&writes));
    }
}

#[test]
fn tcp_mux_init() {
    let mut g = open();
    for e in entries(&g, "tcpmuxinit") {
        let key: [u8; 32] = arr(g.bytes(e, "peer_key"));
        let mut wire = vec![];
        encode_tcp_mux_init(&key, &mut wire).unwrap();
        assert_eq!(
            TcpMuxAcceptor::new().handle_input(&wire).unwrap(),
            (wire.len(), Some(key))
        );
        g.check_hex(e, "wire", &wire);
    }
}

/// A dials B with B's key and B accepts any key, each with a fixed IV.
/// Each side's wire is its handshake init, then its plaintext chunks
/// encrypted in order.
#[test]
fn aes_link() {
    let mut g = open();
    for e in entries(&g, "aes") {
        let a = key_from_seed(&arr(g.bytes(e, "a_seed")));
        let b = key_from_seed(&arr(g.bytes(e, "b_seed")));
        let b_pub: [u8; 32] = arr(b.public.clone());
        let (a_plain, b_plain) = (list(&g, e, "a_plain"), list(&g, e, "b_plain"));

        let mut a_hs = AesLinkHandshake::with_iv(
            key_to_curve25519(&a).unwrap(),
            Some(public_key_to_curve25519(&b_pub)),
            arr(g.bytes(e, "a_iv")),
        );
        let mut b_hs = AesLinkHandshake::with_iv(
            key_to_curve25519(&b).unwrap(),
            None,
            arr(g.bytes(e, "b_iv")),
        );
        let (mut a_wire, mut b_wire) = (vec![], vec![]);
        a_hs.write_init(&mut a_wire).unwrap();
        b_hs.write_init(&mut b_wire).unwrap();
        let (a_init, b_init) = (a_wire.len(), b_wire.len());
        let (n, a_link) = a_hs.handle_input(&b_wire).unwrap();
        assert_eq!(n, b_init);
        let (n, b_link) = b_hs.handle_input(&a_wire).unwrap();
        assert_eq!(n, a_init);
        let (mut a_link, mut b_link) = (a_link.unwrap(), b_link.unwrap());
        for (link, plain, wire) in [
            (&mut a_link, &a_plain, &mut a_wire),
            (&mut b_link, &b_plain, &mut b_wire),
        ] {
            for chunk in plain {
                let mut c = chunk.clone();
                link.encrypt(&mut c);
                wire.extend(c);
            }
        }

        // A decrypts B's ciphertext in odd pieces, B decrypts A's at once.
        let mut pt = b_wire[b_init..].to_vec();
        for c in pt.chunks_mut(3) {
            a_link.decrypt(c);
        }
        assert_eq!(pt, b_plain.concat());
        let mut pt = a_wire[a_init..].to_vec();
        b_link.decrypt(&mut pt);
        assert_eq!(pt, a_plain.concat());

        g.check_hex(e, "a_wire", &a_wire);
        g.check_hex(e, "b_wire", &b_wire);
    }
}

/// The `PeerInit` a host sends at `now`: its profile key from
/// `profile_seed`, its host key from `det_rng(rng_seed)` and its `label`.
#[test]
fn peer_init() {
    let mut g = open();
    for e in entries(&g, "peerinit") {
        let now = Time::from_unix_nanos(num(&g, e, "now")).truncate(Time::PRECISION);
        let label = text(&g, e, "label");
        let config = HostConfig {
            label: label.clone(),
            ..HostConfig::default()
        };
        let profile = key_from_seed(&arr(g.bytes(e, "profile_seed")));
        let mut a: Host<()> = Host::new(profile, config, det_rng(num(&g, e, "rng_seed"))).unwrap();
        let init = a.peer_init(now).unwrap();
        let mut wire = vec![];
        encode_stream_message(&init, &mut wire).unwrap();

        assert_eq!(init.protocol_version, 1);
        assert_eq!(init.node_platform, "");
        assert_eq!(init.node_version, "dev");
        let cert = init.certificate.as_ref().unwrap();
        verify_certificate(cert, now).unwrap();
        assert_eq!(cert.key, a.key().public);
        assert_eq!(cert.subject, label);
        assert_eq!(cert.not_after - cert.not_before, 2 * 3600);
        let parent = certificate_parent(cert).unwrap();
        assert_eq!(parent.key, a.profile_key().public);
        assert_eq!(parent.subject, "");

        // Decoding and re-encoding is the identity.
        let mut r = MessageReader::new();
        let pushed = r.push(&wire);
        assert_eq!(pushed.consumed, wire.len());
        let decoded: PeerInit = decode_stream_message(pushed.message.unwrap()).unwrap();
        assert_eq!(decoded, init);
        let mut again = vec![];
        encode_stream_message(&decoded, &mut again).unwrap();
        assert_eq!(again, wire);

        // Another host completes the handshake on it.
        let mut h: Host<()> =
            Host::new(key_from_seed(&[9; 32]), HostConfig::default(), det_rng(9)).unwrap();
        let link = h
            .add_link(now, LinkConfig::new(LinkKind::Other, 1500))
            .unwrap();
        let mut input = wire.clone();
        assert_eq!(
            h.handle_link_input(link, now, &mut input, &mut |_| {})
                .unwrap(),
            wire.len()
        );
        assert_eq!(
            h.poll_event(),
            Some(Event::PeerAdded {
                peer: link,
                host_id: a.id()
            })
        );
        assert_eq!(
            h.peer(link).unwrap().certificate().key,
            a.profile_key().public
        );

        g.check_hex(e, "host_id", &a.key().public);
        g.check_hex(e, "profile_key", &a.profile_key().public);
        g.check_hex(e, "wire", &wire);
    }
}

/// `out` is empty for an address that has no URI.
#[test]
fn tcp_uris() {
    let mut g = open();
    for e in entries(&g, "tcpuri") {
        let address = text(&g, e, "address");
        let out = match format_tcp_uri(&address, &text(&g, e, "host_ip"), &g.bytes(e, "key")) {
            Ok(uri) => {
                assert!(!uri.is_empty(), "{address}");
                uri
            }
            Err(_) => String::new(),
        };
        g.check_hex(e, "out", out);
    }
}

/// `out` is empty for an address that has no URI.
#[test]
fn ws_uris() {
    let mut g = open();
    for e in entries(&g, "wsuri") {
        let addr = WsListenAddress {
            address: text(&g, e, "address"),
            public_hostname: text(&g, e, "public_hostname"),
            public_port: num(&g, e, "public_port"),
            secure: num(&g, e, "secure"),
        };
        let out = match format_ws_uri(&addr, &g.bytes(e, "key")) {
            Ok(uri) => {
                assert!(!uri.is_empty(), "{addr:?}");
                uri
            }
            Err(_) => String::new(),
        };
        g.check_hex(e, "out", out);
    }
}

#[test]
fn sdp_json() {
    let mut g = open();
    for e in entries(&g, "sdp") {
        let d = SessionDescription {
            sdp_type: SdpType::parse(g.str(e, "type")).unwrap(),
            sdp: text(&g, e, "sdp"),
        };
        let json = d.to_json();
        assert_eq!(SessionDescription::from_json(&json).unwrap(), d);
        g.check_hex(e, "json", json);
    }
    // An accepted `json` pins the `type` and `sdp` it parses to. A rejected
    // one must not be marked `ok` with a known type.
    for e in entries(&g, "sdpdecode") {
        let json = text(&g, e, "json");
        match SessionDescription::from_json(&json) {
            Ok(d) => {
                g.check(e, "ok", "true");
                g.check(e, "type", d.sdp_type.as_str());
                g.check_hex(e, "sdp", d.sdp);
            }
            Err(err) => assert!(
                g.str(e, "ok") != "true" || SdpType::parse(g.str(e, "type")).is_none(),
                "{json}: {err:?}"
            ),
        }
    }
}
