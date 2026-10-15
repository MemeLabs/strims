// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Replays `golden/vectors.txt`, which `golden/main.go` writes with the Go
//! implementation: frames, `FrameWriter` batching, `TCPMuxInit`, the AES link
//! handshake and ciphertext, `PeerInit` from real Go hosts, listener URIs
//! and the WebRTC session description JSON.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;

use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_crypto::{
    certificate_parent, key_from_seed, key_to_curve25519, public_key_to_curve25519,
    verify_certificate,
};
use strims_kademlia::ID;
use strims_proto::vnic::v1::PeerInit;
use strims_vnic::stream::{MessageReader, decode_stream_message, encode_stream_message};
use strims_vnic::tcp::{encode_tcp_mux_init, format_tcp_uri};
use strims_vnic::webrtc::{SdpType, SessionDescription};
use strims_vnic::ws::{WsListenAddress, format_ws_uri};
use strims_vnic::{
    AesLinkHandshake, Event, FRAME_HEADER_LEN, Frame, FrameDecoder, FrameWriter, Host, HostConfig,
    LinkConfig, LinkKind, TcpMuxAcceptor,
};

struct Vector {
    kind: String,
    fields: BTreeMap<String, String>,
}

impl Vector {
    fn s(&self, k: &str) -> &str {
        self.fields
            .get(k)
            .unwrap_or_else(|| panic!("{} missing {k}", self.kind))
    }
    fn hex(&self, k: &str) -> Vec<u8> {
        hex::decode(self.s(k)).unwrap()
    }
    fn text(&self, k: &str) -> String {
        String::from_utf8(self.hex(k)).unwrap()
    }
    fn list(&self, k: &str) -> Vec<Vec<u8>> {
        let s = self.s(k);
        if s.is_empty() {
            return vec![];
        }
        s.split(',').map(|x| hex::decode(x).unwrap()).collect()
    }
    fn num<T: std::str::FromStr>(&self, k: &str) -> T
    where
        T::Err: std::fmt::Debug,
    {
        self.s(k).parse().unwrap()
    }
}

fn vectors(kind: &str) -> Vec<Vector> {
    let text = include_str!("../golden/vectors.txt");
    let vs: Vec<Vector> = text
        .lines()
        .filter(|l| l.split(' ').next() == Some(kind))
        .map(|l| {
            let mut parts = l.split(' ');
            let kind = parts.next().unwrap().to_owned();
            let fields = parts
                .map(|p| {
                    let (k, v) = p.split_once('=').unwrap();
                    (k.to_owned(), v.to_owned())
                })
                .collect();
            Vector { kind, fields }
        })
        .collect();
    assert!(!vs.is_empty(), "no {kind} vectors");
    vs
}

#[test]
fn frames() {
    for v in vectors("frame") {
        let port: u16 = v.num("port");
        let body = v.hex("body");
        let wire = v.hex("wire");
        let f = Frame { port, body: &body };
        let mut buf = vec![0u8; f.encoded_len()];
        assert_eq!(f.encode(&mut buf).unwrap(), wire.len());
        assert_eq!(buf, wire);
        assert_eq!(Frame::decode(&wire).unwrap(), (f, wire.len()));
        for split in 0..=wire.len() {
            let mut d = FrameDecoder::new();
            let mut got = vec![];
            d.decode(&wire[..split], &mut |f| got.push((f.port, f.body.to_vec())));
            d.decode(&wire[split..], &mut |f| got.push((f.port, f.body.to_vec())));
            assert_eq!(got, vec![(port, body.clone())]);
        }
    }
}

#[test]
fn frame_writer_batching() {
    for v in vectors("framewriter") {
        let mtu: usize = v.num("mtu");
        let port: u16 = v.num("port");
        let mut w = FrameWriter::new(mtu - FRAME_HEADER_LEN, 1 << 20);
        for op in v.s("ops").split(',') {
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
        // Each recorded link write is one frame.
        let mut got = vec![];
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
            got.push(frame);
        }
        assert_eq!(got, v.list("writes"), "mtu {mtu}");
    }
}

#[test]
fn tcp_mux_init() {
    for v in vectors("tcpmuxinit") {
        let key: [u8; 32] = v.hex("peer_key").try_into().unwrap();
        let wire = v.hex("wire");
        let mut out = vec![];
        encode_tcp_mux_init(&key, &mut out).unwrap();
        assert_eq!(out, wire);
        assert_eq!(
            TcpMuxAcceptor::new().handle_input(&wire).unwrap(),
            (wire.len(), Some(key))
        );
    }
}

/// Go ran `handshakeAESLink` on both ends (A dialing B with B's key) with
/// fixed IVs, then wrote the plaintext chunks each way.
#[test]
fn aes_link() {
    for v in vectors("aes") {
        let a = key_from_seed(&v.hex("a_seed").try_into().unwrap());
        let b = key_from_seed(&v.hex("b_seed").try_into().unwrap());
        let b_pub: [u8; 32] = b.public.as_slice().try_into().unwrap();
        let (a_wire, b_wire) = (v.hex("a_wire"), v.hex("b_wire"));
        let (a_plain, b_plain) = (v.list("a_plain"), v.list("b_plain"));

        // A's side: same init bytes, same ciphertext.
        let hs = AesLinkHandshake::with_iv(
            key_to_curve25519(&a).unwrap(),
            Some(public_key_to_curve25519(&b_pub)),
            v.hex("a_iv").try_into().unwrap(),
        );
        let mut init = vec![];
        hs.write_init(&mut init).unwrap();
        assert_eq!(init, a_wire[..init.len()]);
        let mut hs = hs;
        let (n, link) = hs.handle_input(&b_wire).unwrap();
        let mut link = link.unwrap();
        let mut ct = vec![];
        for chunk in &a_plain {
            let mut c = chunk.clone();
            link.encrypt(&mut c);
            ct.extend(c);
        }
        assert_eq!(ct, a_wire[init.len()..]);
        // Decrypt B's ciphertext in odd pieces.
        let mut pt = b_wire[n..].to_vec();
        for c in pt.chunks_mut(3) {
            link.decrypt(c);
        }
        assert_eq!(pt, b_plain.concat());

        // B's side (the listener, accepting any key).
        let mut hs = AesLinkHandshake::with_iv(
            key_to_curve25519(&b).unwrap(),
            None,
            v.hex("b_iv").try_into().unwrap(),
        );
        let mut init = vec![];
        hs.write_init(&mut init).unwrap();
        assert_eq!(init, b_wire[..n]);
        let (m, link) = hs.handle_input(&a_wire).unwrap();
        let mut link = link.unwrap();
        let mut pt = a_wire[m..].to_vec();
        link.decrypt(&mut pt);
        assert_eq!(pt, a_plain.concat());
    }
}

/// `PeerInit`s written by real Go hosts.
#[test]
fn peer_init() {
    for v in vectors("peerinit") {
        let wire = v.hex("wire");
        let now = Time::from_unix_nanos(v.num("now")).truncate(Time::PRECISION);
        let mut r = MessageReader::new();
        let pushed = r.push(&wire);
        assert_eq!(pushed.consumed, wire.len());
        let init: PeerInit = decode_stream_message(pushed.message.unwrap()).unwrap();
        // Re-encoding is byte-identical.
        let mut again = vec![];
        encode_stream_message(&init, &mut again).unwrap();
        assert_eq!(again, wire);

        assert_eq!(init.protocol_version, 1);
        assert_eq!(init.node_platform, "");
        assert_eq!(init.node_version, "dev");
        let cert = init.certificate.as_ref().unwrap();
        verify_certificate(cert, now).unwrap();
        assert_eq!(cert.key, v.hex("host_id"));
        assert_eq!(cert.subject, v.text("label"));
        let parent = certificate_parent(cert).unwrap();
        assert_eq!(parent.key, v.hex("profile_key"));
        assert_eq!(parent.subject, "");
        assert_eq!(cert.not_after - cert.not_before, 2 * 3600);

        // A Rust host completes the handshake on it.
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
        let host_id = ID::from_bytes(&v.hex("host_id").try_into().unwrap());
        assert_eq!(
            h.poll_event(),
            Some(Event::PeerAdded {
                peer: link,
                host_id
            })
        );
        assert_eq!(
            h.peer(link).unwrap().certificate().key,
            v.hex("profile_key")
        );

        // Rust's own PeerInit has the same shape.
        let ours = h.peer_init(now).unwrap();
        let c = ours.certificate.as_ref().unwrap();
        assert_eq!(c.key, h.key().public);
        assert_eq!(c.not_after - c.not_before, 2 * 3600);
        assert_eq!(certificate_parent(c).unwrap().key, h.profile_key().public);
        verify_certificate(c, now).unwrap();
    }
}

#[test]
fn tcp_uris() {
    for v in vectors("tcpuri") {
        let got = format_tcp_uri(&v.text("address"), &v.text("host_ip"), &v.hex("key"));
        let want = v.text("out");
        if want.is_empty() {
            assert!(got.is_err(), "{} -> {got:?}", v.text("address"));
        } else {
            assert_eq!(got.unwrap(), want);
        }
    }
}

#[test]
fn ws_uris() {
    for v in vectors("wsuri") {
        let addr = WsListenAddress {
            address: v.text("address"),
            public_hostname: v.text("public_hostname"),
            public_port: v.num("public_port"),
            secure: v.num("secure"),
        };
        let got = format_ws_uri(&addr, &v.hex("key"));
        let want = v.text("out");
        if want.is_empty() {
            assert!(got.is_err(), "{addr:?} -> {got:?}");
        } else {
            assert_eq!(got.unwrap(), want);
        }
    }
}

#[test]
fn sdp_json() {
    for v in vectors("sdp") {
        let d = SessionDescription {
            sdp_type: SdpType::parse(v.s("type")).unwrap(),
            sdp: v.text("sdp"),
        };
        let json = v.text("json");
        assert_eq!(d.to_json(), json);
        assert_eq!(SessionDescription::from_json(&json).unwrap(), d);
    }
    for v in vectors("sdpdecode") {
        let json = v.text("json");
        let got = SessionDescription::from_json(&json);
        // Parsing succeeds exactly when the Go decoder accepted the vector
        // and its type is a known name.
        match SdpType::parse(v.s("type")) {
            Some(t) if v.s("ok") == "true" => {
                let d = got.unwrap();
                assert_eq!(d.sdp_type, t, "{json}");
                assert_eq!(d.sdp, v.text("sdp"));
            }
            _ => assert!(got.is_err(), "{json} -> {got:?}"),
        }
    }
}
