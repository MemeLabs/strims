// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use strims_base::rng::det_rng;

use super::*;

fn packet(b: &[u8]) -> &Packet {
    b.try_into().unwrap()
}

/// The C2 bytes a server skips.
const C2: [u8; PACKET_LEN] = [0x5a; PACKET_LEN];

/// Runs a server handshake over C0 C1 and C2; returns the reply.
fn serve(hello: &[u8], seed: u64) -> (ServerHandshake, Vec<u8>) {
    let mut rng = det_rng(seed);
    let mut server = ServerHandshake::new();
    let (n, reply) = server.read(hello, &mut rng).unwrap();
    assert_eq!(n, hello.len());
    let reply = reply.unwrap();
    assert!(!server.is_done());
    let (n, again) = server.read(&C2, &mut rng).unwrap();
    assert_eq!((n, again), (PACKET_LEN, None));
    assert!(server.is_done());
    (server, reply)
}

#[test]
fn keys_are_the_flash_keys() {
    assert_eq!(CLIENT_PARTIAL_KEY, b"Genuine Adobe Flash Player 001");
    assert_eq!(SERVER_PARTIAL_KEY, b"Genuine Adobe Flash Media Server 001");
    let tail =
        hex::decode("f0eec24a8068bee82e00d0d1029e7e576eec5d2d29806fab93b8e636cfeb31ae").unwrap();
    assert_eq!(&CLIENT_FULL_KEY[30..], tail);
    assert_eq!(&SERVER_FULL_KEY[36..], tail);
}

#[test]
fn digest_handshake_validates_both_ways() {
    let mut rng = det_rng(1);
    let (mut client, hello) = ClientHandshake::new(true, &mut rng);
    assert_eq!(hello.len(), 1 + PACKET_LEN);
    let c1 = packet(&hello[1..]);
    let c1_digest = find_digest(c1, CLIENT_PARTIAL_KEY).unwrap();

    let (server, reply) = serve(&hello, 2);
    assert!(server.digest());
    assert_eq!(reply.len(), 1 + 2 * PACKET_LEN);
    assert_eq!(reply[0], VERSION);
    let s1 = packet(&reply[1..=PACKET_LEN]);
    let s2 = packet(&reply[1 + PACKET_LEN..]);
    assert_eq!(&s1[..4], [0; 4]);
    assert_eq!(&s1[4..8], SERVER_VERSION);
    let s1_digest = find_digest(s1, SERVER_PARTIAL_KEY).unwrap();
    // S1's digest sits at base 8.
    let offset = digest_offset(s1, S1_DIGEST_BASE);
    assert_eq!(&s1[offset..offset + DIGEST_LEN], s1_digest);
    assert!(response_valid(&SERVER_FULL_KEY, &c1_digest, s2));

    let (n, c2) = client.read(&reply, &mut rng).unwrap();
    assert_eq!(n, reply.len());
    assert!(client.is_done());
    assert_eq!(client.server_digest(), Some(true));
    assert!(response_valid(
        &CLIENT_FULL_KEY,
        &s1_digest,
        packet(&c2.unwrap())
    ));
}

#[test]
fn plain_handshake_echoes_c1() {
    let mut rng = det_rng(3);
    let (mut client, hello) = ClientHandshake::new(false, &mut rng);
    assert_eq!(&hello[5..9], [0; 4]);
    let (server, reply) = serve(&hello, 4);
    assert!(!server.digest());
    let s1 = &reply[1..=PACKET_LEN];
    assert_eq!(&s1[..8], [0; 8]);
    assert_eq!(&reply[1 + PACKET_LEN..], &hello[1..]);
    let (_, c2) = client.read(&reply, &mut rng).unwrap();
    assert_eq!(client.server_digest(), None);
    assert_eq!(c2.unwrap(), s1);
}

#[test]
fn c1_digest_at_base_772_takes_the_digest_handshake() {
    let mut rng = det_rng(5);
    let mut c1 = [0; PACKET_LEN];
    rand_core::Rng::fill_bytes(&mut rng, &mut c1[8..]);
    c1[4..8].copy_from_slice(&[9, 0, 124, 2]);
    let c1_digest = write_digest(&mut c1, 772, CLIENT_PARTIAL_KEY);
    assert!(digest_offset(&c1, 772) >= 776);
    assert_eq!(find_digest(&c1, CLIENT_PARTIAL_KEY), Some(c1_digest));
    let mut hello = vec![VERSION];
    hello.extend_from_slice(&c1);
    let (server, reply) = serve(&hello, 6);
    assert!(server.digest());
    assert!(response_valid(
        &SERVER_FULL_KEY,
        &c1_digest,
        packet(&reply[1 + PACKET_LEN..])
    ));
}

#[test]
fn corrupted_c1_digest_takes_the_plain_handshake() {
    let mut rng = det_rng(7);
    let (_, hello) = ClientHandshake::new(true, &mut rng);
    let offset = digest_offset(packet(&hello[1..]), 8);
    for at in [offset, offset + DIGEST_LEN - 1, 1000, PACKET_LEN - 1] {
        let mut bad = hello.clone();
        bad[1 + at] ^= 1;
        let (server, reply) = serve(&bad, 8);
        assert!(!server.digest(), "flipped byte {at}");
        assert_eq!(&reply[1 + PACKET_LEN..], &bad[1..]);
    }
}

#[test]
fn client_reports_a_bad_server_digest() {
    let mut rng = det_rng(9);
    let (client, hello) = ClientHandshake::new(true, &mut rng);
    let (_, reply) = serve(&hello, 10);
    // A bad S2 digest; S1's still signs C2.
    let mut bad = reply.clone();
    *bad.last_mut().unwrap() ^= 1;
    let mut c = clone(&client);
    let (_, c2) = c.read(&bad, &mut rng).unwrap();
    assert_eq!(c.server_digest(), Some(false));
    assert_ne!(c2.unwrap(), &reply[1..=PACKET_LEN]);
    // A bad S1 digest: C2 echoes S1.
    let mut bad = reply.clone();
    bad[1 + 900] ^= 1;
    let mut c = clone(&client);
    let (_, c2) = c.read(&bad, &mut rng).unwrap();
    assert_eq!(c.server_digest(), Some(false));
    assert_eq!(c2.unwrap(), &bad[1..=PACKET_LEN]);
}

fn clone(c: &ClientHandshake) -> ClientHandshake {
    ClientHandshake {
        c1_digest: c.c1_digest,
        s: c.s.clone(),
        done: c.done,
        server_digest: c.server_digest,
    }
}

#[test]
fn bad_versions_are_refused() {
    let mut rng = det_rng(11);
    for version in [0, 6, 0xff] {
        let mut server = ServerHandshake::new();
        let e = server.read(&[version], &mut rng).unwrap_err();
        assert_eq!(e, RtmpError::Version(version));
        assert_eq!(
            e.to_string(),
            format!(
                "RTMP version {version} is not supported; the handshake must start with version 3"
            )
        );
    }
    let (mut client, _) = ClientHandshake::new(true, &mut rng);
    assert_eq!(client.read(&[6], &mut rng), Err(RtmpError::Version(6)));
}

#[test]
fn split_input_gives_the_same_reply() {
    let mut rng = det_rng(12);
    let (_, hello) = ClientHandshake::new(true, &mut rng);
    let (_, whole) = serve(&hello, 13);
    let mut input = hello.clone();
    input.extend_from_slice(&C2);
    input.extend_from_slice(b"chunks");
    for step in [1, 7, 1536] {
        let mut rng = det_rng(13);
        let mut server = ServerHandshake::new();
        let mut reply = None;
        let mut pos = 0;
        while !server.is_done() {
            let end = (pos + step).min(input.len());
            let (n, r) = server.read(&input[pos..end], &mut rng).unwrap();
            pos += n;
            if r.is_some() {
                reply = r;
            }
        }
        // The handshake stops at the end of C2.
        assert_eq!(pos, 1 + 2 * PACKET_LEN, "step {step}");
        assert_eq!(reply.unwrap(), whole, "step {step}");
    }
}
