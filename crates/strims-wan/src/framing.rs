// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! What an access link charges a packet past its IP bytes.
//!
//! A shaper counts the frames its link carries, not IP packets: each
//! packet's link headers and trailers, a floor on what a short one costs,
//! and on DSL the line code, ATM's 53-byte cells of 48 (the frame padded to
//! whole cells) or VDSL2's PTM, a byte in 65 (tc-cake(8)'s overhead
//! keywords give each encapsulation's figures). Small packets pay the most:
//! a 64-byte IP packet costs 159 bytes of ADSL cells under PPPoE over LLC,
//! a 1500-byte one 1749.

use core::fmt;

use crate::wire::{MSS, TCP_HEADER};

/// How a link carries its frames' bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    /// As they are.
    Bytes,
    /// In ATM cells: 53 bytes for each 48 or part of 48 (ADSL).
    Atm,
    /// In PTM's 64/65 code: a byte more for each 64 or part (VDSL2).
    Ptm,
}

/// A link's per-packet charge (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Framing {
    /// Bytes past the IP packet that the link counts in each.
    pub overhead: u16,
    /// The least a packet costs, its overhead included, before the line
    /// code.
    pub mpu: u16,
    pub line: Line,
}

/// IP bytes in a full TCP packet.
const FULL: usize = MSS + TCP_HEADER;

impl Framing {
    /// IP bytes alone.
    pub const NONE: Framing = Framing::bytes(0, 0);
    /// An Ethernet header, 14 bytes: what Linux's qdiscs count on an
    /// Ethernet device's egress (its `skb->len`), as the Docker testbed's
    /// tbf does.
    pub const ETHERNET: Framing = Framing::bytes(14, 0);
    /// An Ethernet frame on the wire: header, FCS, preamble and gap, 38
    /// bytes, at least 84 (tc-cake(8) `ethernet`).
    pub const ETHERNET_WIRE: Framing = Framing::bytes(38, 84);
    /// DOCSIS: a service flow counts the Ethernet frame through its CRC,
    /// 18 bytes, at least 64 (tc-cake(8) `docsis`).
    pub const DOCSIS: Framing = Framing::bytes(18, 64);
    /// ADSL with PPPoE over LLC/SNAP, 40 bytes in ATM cells (tc-cake(8)
    /// `pppoe-llcsnap`).
    pub const ADSL_PPPOE: Framing = Framing {
        overhead: 40,
        mpu: 0,
        line: Line::Atm,
    };
    /// VDSL2 with PPPoE and a VLAN, 30 bytes in PTM (tc-cake(8)
    /// `pppoe-ptm`).
    pub const VDSL_PPPOE: Framing = Framing {
        overhead: 30,
        mpu: 0,
        line: Line::Ptm,
    };

    pub const fn bytes(overhead: u16, mpu: u16) -> Framing {
        Framing {
            overhead,
            mpu,
            line: Line::Bytes,
        }
    }

    /// What a packet of `ip` bytes costs the link.
    pub fn packet(self, ip: usize) -> usize {
        let n = ip
            .saturating_add(usize::from(self.overhead))
            .max(usize::from(self.mpu));
        match self.line {
            Line::Bytes => n,
            Line::Atm => n.div_ceil(48).saturating_mul(53),
            Line::Ptm => n.saturating_add(n.div_ceil(64)),
        }
    }

    /// What a message of `wire` IP bytes in `packets` packets costs, all
    /// but the last a full TCP packet ([`crate::wire::tcp_wire`]'s; a
    /// datagram is one packet).
    pub fn message(self, wire: usize, packets: u32) -> usize {
        let (full, each, small, extra) = split(wire, packets);
        full.saturating_mul(self.packet(FULL))
            .saturating_add(small.saturating_mul(self.packet(each)))
            .saturating_add(extra.saturating_mul(self.packet(each + 1)))
    }

    /// What each packet of such a message costs, in the order they leave:
    /// its full packets, then the rest ([`Framing::message`] is their sum).
    pub fn packets(self, wire: usize, packets: u32) -> impl Iterator<Item = usize> {
        let (full, each, small, extra) = split(wire, packets);
        std::iter::repeat_n(self.packet(FULL), full)
            .chain(std::iter::repeat_n(self.packet(each), small))
            .chain(std::iter::repeat_n(self.packet(each + 1), extra))
    }
}

/// A message of `wire` IP bytes in `packets` packets, all but the last a
/// full TCP packet: its full packets, and the rest's packets of `each`
/// bytes and of `each + 1`, which share what is left evenly.
fn split(wire: usize, packets: u32) -> (usize, usize, usize, usize) {
    let full = (packets.max(1) as usize - 1).min(wire / FULL);
    let rest = wire - full * FULL;
    let tail = packets.max(1) as usize - full;
    let each = rest / tail.max(1);
    let extra = rest - each * tail.max(1);
    (full, each, tail - extra, extra)
}

impl fmt::Display for Framing {
    /// `<overhead>`, ` mpu <mpu>` if it has one, and ` atm` or ` ptm` for
    /// a line code.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.overhead)?;
        if self.mpu > 0 {
            write!(f, " mpu {}", self.mpu)?;
        }
        match self.line {
            Line::Bytes => Ok(()),
            Line::Atm => f.write_str(" atm"),
            Line::Ptm => f.write_str(" ptm"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{tcp_wire, udp_wire};

    #[test]
    fn a_messages_packets_cost_what_it_does() {
        for f in [
            Framing::NONE,
            Framing::ETHERNET,
            Framing::DOCSIS,
            Framing::ADSL_PPPOE,
        ] {
            for (wire, packets) in [
                (40, 1),
                (1500, 1),
                (3000, 2),
                (2000, 3),
                (10_000, 7),
                (5, 4),
            ] {
                let each: Vec<usize> = f.packets(wire, packets).collect();
                assert_eq!(each.len(), packets as usize, "{wire} {packets}");
                assert_eq!(
                    each.iter().sum::<usize>(),
                    f.message(wire, packets),
                    "{wire} {packets}"
                );
            }
        }
    }

    #[test]
    fn links_charge_their_frames() {
        assert_eq!(Framing::NONE.packet(64), 64);
        assert_eq!(Framing::ETHERNET.packet(64), 78);
        assert_eq!(Framing::ETHERNET.packet(1500), 1514);
        assert_eq!(Framing::ETHERNET_WIRE.packet(40), 84);
        assert_eq!(Framing::DOCSIS.packet(40), 64);
        assert_eq!(Framing::DOCSIS.packet(1500), 1518);
        assert_eq!(Framing::VDSL_PPPOE.packet(1500), 1530 + 24);
    }

    #[test]
    fn atm_pads_to_whole_cells() {
        // The testbed's tbf under `stab linklayer atm overhead 26` charged
        // these (hack/testbed: 26 is the 40 less the 14 of skb->len).
        for (ip, cells) in [
            (64, 159),
            (128, 212),
            (228, 318),
            (500, 636),
            (828, 1007),
            (1028, 1219),
            (1500, 1749),
        ] {
            assert_eq!(Framing::ADSL_PPPOE.packet(ip), cells, "{ip}");
        }
    }

    #[test]
    fn a_message_is_its_packets() {
        let (w, p) = tcp_wire(3 * MSS + 100);
        assert_eq!(
            Framing::ETHERNET.message(w, p),
            3 * 1514 + Framing::ETHERNET.packet(152)
        );
        assert_eq!(
            Framing::ADSL_PPPOE.message(w, p),
            3 * 1749 + Framing::ADSL_PPPOE.packet(152)
        );
        let (w, p) = udp_wire(1200);
        assert_eq!(Framing::ETHERNET.message(w, p), 1242);
        assert_eq!(Framing::NONE.message(w, p), w);
        // Packets that aren't full TCP ones split what they carry.
        assert_eq!(Framing::ETHERNET.message(200, 2), 228);
        assert_eq!(Framing::ETHERNET.message(201, 2), 229);
    }
}
