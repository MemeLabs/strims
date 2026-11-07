// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Packet framing and serialization times.

use strims_base::time::Duration;

/// TCP payload bytes per packet (a 1500-byte MTU less IPv4, TCP and
/// timestamp headers).
pub const MSS: usize = 1448;
/// IPv4, TCP and timestamp option header bytes per TCP packet. No link
/// framing: a write goes in packets of its own, where TCP fills segments
/// across writes, which charges about what Ethernet's 14 bytes would (the
/// SCTP model, whose messages are packets of their own, charges them).
pub const TCP_HEADER: usize = 52;
/// IPv4 and UDP header bytes per datagram.
pub const UDP_HEADER: usize = 28;
/// Packets of at most this many wire bytes count as small: a short message
/// alone, an ACK or a SACK, whose headers weigh as much as what it carries.
pub const SMALL_PACKET: usize = 256;

/// The wire bytes and packet count of `payload` bytes of TCP stream data.
/// An empty payload (a SYN, FIN or RST) is one bare packet.
pub fn tcp_wire(payload: usize) -> (usize, u32) {
    let packets = payload.div_ceil(MSS).max(1);
    (
        payload.saturating_add(packets.saturating_mul(TCP_HEADER)),
        u32::try_from(packets).unwrap_or(u32::MAX),
    )
}

/// The wire bytes and packet count of a `payload`-byte datagram.
pub fn udp_wire(payload: usize) -> (usize, u32) {
    (payload.saturating_add(UDP_HEADER), 1)
}

/// Of a message's `packets` packets and `wire` bytes, how many are small
/// ([`SMALL_PACKET`]): the last, short of a full TCP packet, or the only
/// one.
pub fn small_packets(wire: usize, packets: u32) -> u32 {
    let full = (packets.max(1) as usize - 1).saturating_mul(MSS + TCP_HEADER);
    u32::from(wire.saturating_sub(full) <= SMALL_PACKET)
}

/// How long `bytes` take to serialize at `rate` bytes per second.
pub fn tx_time(bytes: usize, rate: u64) -> Duration {
    let rate = i64::try_from(rate.max(1)).unwrap_or(i64::MAX);
    Duration::from_nanos(
        i64::try_from(bytes)
            .unwrap_or(i64::MAX)
            .saturating_mul(1_000_000_000)
            / rate,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing() {
        assert_eq!(tcp_wire(0), (TCP_HEADER, 1));
        assert_eq!(tcp_wire(MSS), (MSS + TCP_HEADER, 1));
        assert_eq!(tcp_wire(MSS + 1), (MSS + 1 + 2 * TCP_HEADER, 2));
        assert_eq!(tcp_wire(65_535).1, 46);
        assert_eq!(udp_wire(1200), (1228, 1));
    }

    #[test]
    fn small_packets_are_the_short_ones() {
        let (w, p) = tcp_wire(100);
        assert_eq!(small_packets(w, p), 1);
        let (w, p) = tcp_wire(MSS);
        assert_eq!(small_packets(w, p), 0);
        let (w, p) = tcp_wire(MSS + 100);
        assert_eq!(small_packets(w, p), 1);
        let (w, p) = tcp_wire(2 * MSS);
        assert_eq!(small_packets(w, p), 0);
        assert_eq!(small_packets(1228, 1), 0);
    }

    #[test]
    fn serialization() {
        assert_eq!(tx_time(125_000, 125_000), Duration::SECOND);
        assert_eq!(tx_time(1500, 1_000_000), Duration::from_micros(1500));
        assert_eq!(tx_time(10, 0), Duration::from_secs(10));
    }
}
