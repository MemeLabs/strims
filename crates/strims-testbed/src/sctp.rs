// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A data channel's SCTP association as the tables' `sctp_events` take
//! it ([`SctpEvent`]): each packet it sent or received and each timer it
//! fired. strims-sim's window model writes its own associations' events;
//! a testbed run's come from a Chromium node's dcSCTP, which logs every
//! packet its associations send or receive, decrypted.
//!
//! dcSCTP's packet observer (`TextPcapPacketObserver`,
//! net/dcsctp/public/text_pcap_packet_observer.cc) is on in each
//! association `DcSctpTransport::Start` makes while WebRTC's verbose
//! logging is on for media/sctp/dcsctp_transport.cc, which Chromium maps
//! to vlog 1 of that file; [`CHROMIUM_FLAGS`] turn it on there and in
//! the observer's file alone. Each packet is then a log message of three
//! lines on the browser's stderr (`--enable-logging=stderr`):
//!
//! ```text
//! [<pid>:<tid>:<MMDD>/<HHMMSS>.<µs>:VERBOSE1:third_party/webrtc/net/dcsctp/public/text_pcap_packet_observer.cc:<line>]
//!
//! O 00:00:36.475 0000 13 88 13 88 ... # SCTP_PACKET DcSctpTransport1
//! ```
//!
//! the prefix's time (Chromium's local time, to the µs, which the harness
//! runs at `TZ=UTC`: the message's, on the network thread that sends and
//! receives the association's packets), an empty
//! line, and `O` (sent) or `I` (received) with dcSCTP's own ms, the
//! packet's bytes in hex and the association's name, one per peer
//! connection in the order the page made them. A sent packet is logged
//! as dcSCTP hands it to DTLS, whether or not the socket below took it,
//! and a received one as dcSCTP takes it. Formatting each byte costs the
//! network thread about 0.17 ms a full packet, and so slows the
//! association's sending: a run with the log on is timed otherwise and
//! counts otherwise (its uplink's drops, its packets' trains) than one with
//! it off. The testbed turns it on only for the nodes `TRACE_SCTP` names,
//! and a bench reads its timing, drops and wire from runs with it off and
//! only what the log alone holds from runs with it on.
//!
//! [`Observer`] reads the browser's lines as they come and makes each
//! packet a [`Packet`]: its time, direction, association and its chunks'
//! headers, which the harness writes a JSON line each
//! ([`Packet::json`]). strims-sim's testbed module reads them back
//! ([`Packet::parse`]), names each association's link by the node's
//! writes on it ([`name_assocs`]), and makes its rows ([`Rows`]).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::Value;
use strims_base::time::Duration;

/// Chromium's flags that turn dcSCTP's packet observer on, besides
/// `--enable-logging=stderr`.
pub const CHROMIUM_FLAGS: &[&str] = &["--vmodule=dcsctp_transport=1,text_pcap_packet_observer=1"];

/// A DATA chunk's header, an I-DATA chunk's, and SCTP's common header.
const DATA_HEADER: u32 = 16;
const IDATA_HEADER: u32 = 20;
const COMMON_HEADER: usize = 12;
const DAY_US: u64 = 86_400_000_000;

/// A chunk of a logged packet, its header's fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chunk {
    /// DATA (or I-DATA, `idata`): its TSN, stream, the message's sequence
    /// number (I-DATA's message id, cut to 16 bits), payload bytes and
    /// flags (I 8, U 4, B 2, E 1).
    Data {
        tsn: u32,
        sid: u16,
        ssn: u16,
        len: u32,
        flags: u8,
        idata: bool,
    },
    /// SACK: its cumulative TSN, the window it advertises, its gap blocks
    /// (offsets from the cumulative TSN) and duplicate TSNs.
    Sack {
        cum: u32,
        rwnd: u32,
        gaps: Vec<(u16, u16)>,
        dups: u16,
    },
    /// INIT or INIT ACK (`ack`): the tag and initial TSN it announces.
    Init { ack: bool, tag: u32, tsn: u32 },
    /// Any other chunk, by type.
    Other(u8),
}

impl Chunk {
    /// The bytes a DATA chunk puts in its sender's flight, as dcSCTP
    /// counts them: its header and padded payload.
    fn size(&self) -> u32 {
        match *self {
            Chunk::Data { len, idata, .. } => {
                (if idata { IDATA_HEADER } else { DATA_HEADER }) + len.next_multiple_of(4)
            }
            _ => 0,
        }
    }
}

/// One packet an association sent or received.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    /// When it was logged, in µs of Unix time.
    pub t_us: u64,
    pub out: bool,
    /// The association: its `DcSctpTransport` number.
    pub assoc: u32,
    /// Its common header's verification tag.
    pub vtag: u32,
    /// Its SCTP bytes: the common header and its chunks.
    pub bytes: u32,
    pub chunks: Vec<Chunk>,
}

fn be16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(at)?, *b.get(at + 1)?]))
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes([
        *b.get(at)?,
        *b.get(at + 1)?,
        *b.get(at + 2)?,
        *b.get(at + 3)?,
    ]))
}

impl Packet {
    /// The packet of SCTP bytes `b`: its common header and each chunk's
    /// header; `None` for one too short for its common header or a chunk
    /// that runs past its end.
    pub fn of_bytes(t_us: u64, out: bool, assoc: u32, b: &[u8]) -> Option<Packet> {
        let vtag = be32(b, 4)?;
        let mut chunks = Vec::new();
        let mut at = COMMON_HEADER;
        while at + 4 <= b.len() {
            let kind = *b.get(at)?;
            let flags = *b.get(at + 1)?;
            let len = usize::from(be16(b, at + 2)?);
            if len < 4 || at + len > b.len() {
                return None;
            }
            let c = |off: usize| be32(b, at + off);
            chunks.push(match kind {
                0 | 64 => {
                    let idata = kind == 64;
                    let header = if idata { IDATA_HEADER } else { DATA_HEADER };
                    // I-DATA: its TSN, stream, two reserved bytes, the
                    // message id and the PPID or fragment number.
                    let ssn = if idata {
                        c(12)? as u16
                    } else {
                        be16(b, at + 10)?
                    };
                    Chunk::Data {
                        tsn: c(4)?,
                        sid: be16(b, at + 8)?,
                        ssn,
                        len: u32::try_from(len).ok()?.checked_sub(header)?,
                        flags,
                        idata,
                    }
                }
                3 => {
                    let n = usize::from(be16(b, at + 12)?);
                    let gaps = (0..n)
                        .map(|i| Some((be16(b, at + 16 + 4 * i)?, be16(b, at + 18 + 4 * i)?)))
                        .collect::<Option<Vec<_>>>()?;
                    Chunk::Sack {
                        cum: c(4)?,
                        rwnd: c(8)?,
                        gaps,
                        dups: be16(b, at + 14)?,
                    }
                }
                1 | 2 => Chunk::Init {
                    ack: kind == 2,
                    tag: c(4)?,
                    tsn: c(16)?,
                },
                k => Chunk::Other(k),
            });
            at += len.next_multiple_of(4);
        }
        Some(Packet {
            t_us,
            out,
            assoc,
            vtag,
            bytes: u32::try_from(b.len()).ok()?,
            chunks,
        })
    }

    /// The packet as a JSON line's object, which [`Packet::parse`] reads
    /// back: `t` (Unix µs), `d` (`o` sent, `i` received), `a`, `v`, `b`,
    /// and `c` its chunks, each an array: `["d", tsn, sid, ssn, len,
    /// flags, idata]`, `["s", cum, rwnd, [[start, end], ...], dups]`,
    /// `["i", ack, tag, tsn]`, `["x", type]`.
    pub fn json(&self) -> String {
        let mut s = String::with_capacity(96);
        let d = if self.out { 'o' } else { 'i' };
        let _ = write!(
            s,
            r#"{{"t":{},"d":"{d}","a":{},"v":{},"b":{},"c":["#,
            self.t_us, self.assoc, self.vtag, self.bytes
        );
        for (i, c) in self.chunks.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let _ = match c {
                Chunk::Data {
                    tsn,
                    sid,
                    ssn,
                    len,
                    flags,
                    idata,
                } => write!(
                    s,
                    r#"["d",{tsn},{sid},{ssn},{len},{flags},{}]"#,
                    u8::from(*idata)
                ),
                Chunk::Sack {
                    cum,
                    rwnd,
                    gaps,
                    dups,
                } => {
                    let g: Vec<String> = gaps.iter().map(|(a, b)| format!("[{a},{b}]")).collect();
                    write!(s, r#"["s",{cum},{rwnd},[{}],{dups}]"#, g.join(","))
                }
                Chunk::Init { ack, tag, tsn } => {
                    write!(s, r#"["i",{},{tag},{tsn}]"#, u8::from(*ack))
                }
                Chunk::Other(k) => write!(s, r#"["x",{k}]"#),
            };
        }
        s.push_str("]}");
        s
    }

    /// A packet [`Packet::json`] wrote; `None` for a line it didn't.
    pub fn parse(line: &str) -> Option<Packet> {
        let v: Value = serde_json::from_str(line).ok()?;
        let n = |v: &Value| v.as_u64();
        let n32 = |v: &Value| v.as_u64().and_then(|x| u32::try_from(x).ok());
        let n16 = |v: &Value| v.as_u64().and_then(|x| u16::try_from(x).ok());
        let chunks = v
            .get("c")?
            .as_array()?
            .iter()
            .map(|c| {
                let c = c.as_array()?;
                let at = |i: usize| c.get(i);
                Some(match at(0)?.as_str()? {
                    "d" => Chunk::Data {
                        tsn: n32(at(1)?)?,
                        sid: n16(at(2)?)?,
                        ssn: n16(at(3)?)?,
                        len: n32(at(4)?)?,
                        flags: u8::try_from(n(at(5)?)?).ok()?,
                        idata: n(at(6)?)? != 0,
                    },
                    "s" => Chunk::Sack {
                        cum: n32(at(1)?)?,
                        rwnd: n32(at(2)?)?,
                        gaps: at(3)?
                            .as_array()?
                            .iter()
                            .map(|g| Some((n16(g.get(0)?)?, n16(g.get(1)?)?)))
                            .collect::<Option<Vec<_>>>()?,
                        dups: n16(at(4)?)?,
                    },
                    "i" => Chunk::Init {
                        ack: n(at(1)?)? != 0,
                        tag: n32(at(2)?)?,
                        tsn: n32(at(3)?)?,
                    },
                    "x" => Chunk::Other(u8::try_from(n(at(1)?)?).ok()?),
                    _ => return None,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Packet {
            t_us: n(v.get("t")?)?,
            out: v.get("d")?.as_str()? == "o",
            assoc: n32(v.get("a")?)?,
            vtag: n32(v.get("v")?)?,
            bytes: n32(v.get("b")?)?,
            chunks,
        })
    }

    fn data(&self) -> impl Iterator<Item = &Chunk> {
        self.chunks
            .iter()
            .filter(|c| matches!(c, Chunk::Data { .. }))
    }

    fn sack(&self) -> Option<(u32, &[(u16, u16)])> {
        self.chunks.iter().find_map(|c| match c {
            Chunk::Sack { cum, gaps, .. } => Some((*cum, &gaps[..])),
            _ => None,
        })
    }
}

/// What a browser's line was to the [`Observer`].
#[derive(Debug, PartialEq, Eq)]
pub enum Line {
    /// A packet's last line: the packet.
    Packet(Packet),
    /// Another line of a packet's log message, or one the observer
    /// couldn't read (counted in [`Observer::unread`]).
    Part,
    /// A line of the browser's own.
    Other,
}

/// Reads a Chromium browser's stderr lines into the packets its dcSCTP's
/// observer logged (see the [module docs](self)).
#[derive(Debug, Default)]
pub struct Observer {
    /// The time of the last observer message's prefix, in µs of the UTC
    /// day.
    prefix: Option<u64>,
    /// The packet lines it couldn't read.
    pub unread: u64,
}

impl Observer {
    /// Takes the browser's next line, `now_us` (Unix µs) as it reads it,
    /// which dates the prefix's time of day.
    pub fn take(&mut self, line: &str, now_us: u64) -> Line {
        if line.starts_with('[') && line.contains("text_pcap_packet_observer.cc") {
            self.prefix = prefix_time(line);
            return Line::Part;
        }
        if line.is_empty() && self.prefix.is_some() {
            return Line::Part;
        }
        let Some(tod) = self.prefix.take() else {
            return Line::Other;
        };
        let packet = line
            .split_once(" # SCTP_PACKET DcSctpTransport")
            .and_then(|(dump, name)| {
                let assoc = name.trim().parse::<u32>().ok()?;
                let mut words = dump.split(' ');
                let out = match words.next()? {
                    "O" => true,
                    "I" => false,
                    _ => return None,
                };
                // dcSCTP's own time and the dump's offset.
                words.next()?;
                words.next()?;
                let bytes = words
                    .map(|w| u8::from_str_radix(w, 16).ok())
                    .collect::<Option<Vec<u8>>>()?;
                Packet::of_bytes(dated(tod, now_us), out, assoc, &bytes)
            });
        match packet {
            Some(p) => Line::Packet(p),
            None => {
                self.unread += 1;
                Line::Part
            }
        }
    }
}

/// The µs of the UTC day a Chromium log prefix
/// (`[pid:tid:MMDD/HHMMSS.uuuuuu:...`) names.
fn prefix_time(line: &str) -> Option<u64> {
    let stamp = line.split(':').nth(2)?;
    let (_, clock) = stamp.split_once('/')?;
    let (hms, us) = clock.split_once('.')?;
    let hms: u64 = hms.parse().ok()?;
    let us: u64 = us.parse().ok()?;
    let (h, m, s) = (hms / 10_000, hms / 100 % 100, hms % 100);
    (h < 24 && m < 60 && s < 60 && us < 1_000_000)
        .then_some(((h * 60 + m) * 60 + s) * 1_000_000 + us)
}

/// The Unix µs of the time of day `tod` nearest `now_us`.
fn dated(tod: u64, now_us: u64) -> u64 {
    let t = now_us - now_us % DAY_US + tod;
    if t > now_us + DAY_US / 2 {
        t - DAY_US
    } else if t + DAY_US / 2 < now_us {
        t + DAY_US
    } else {
        t
    }
}

/// The sizes of the messages an association's packets show it sent, in
/// order, as many as `max`, and when its first DATA went out.
fn sent_messages(packets: &[Packet], max: usize) -> (Vec<u64>, Option<u64>) {
    let (mut sizes, mut first, mut open, mut top) = (Vec::new(), None, 0u64, None::<u32>);
    for p in packets.iter().filter(|p| p.out) {
        for c in p.data() {
            let &Chunk::Data {
                tsn, len, flags, ..
            } = c
            else {
                continue;
            };
            if top.is_some_and(|t| (tsn.wrapping_sub(t) as i32) <= 0) {
                continue;
            }
            top = Some(tsn);
            first.get_or_insert(p.t_us);
            if flags & 2 != 0 {
                open = 0;
            }
            open += u64::from(len);
            if flags & 1 != 0 {
                sizes.push(open);
                open = 0;
                if sizes.len() >= max {
                    return (sizes, first);
                }
            }
        }
    }
    (sizes, first)
}

/// How many of the messages an association sends [`name_assocs`] matches
/// to a link's writes at most.
const MATCH: usize = 32;

/// Each association of `packets` (by number) named by the link whose
/// writes it carried: of `links`, each a link's key and its writes'
/// sizes in order, the one whose writes begin as the messages the
/// association sent begin, over the most messages (one at least, and every
/// message of the shorter), and only it: an association two links fit as
/// well is left out, and so is a link two associations fit.
pub fn name_assocs<K: Clone + Ord>(
    packets: &BTreeMap<u32, Vec<Packet>>,
    links: &[(K, Vec<u64>)],
) -> BTreeMap<u32, K> {
    let mut best: BTreeMap<u32, K> = BTreeMap::new();
    let mut taken: BTreeMap<K, usize> = BTreeMap::new();
    for (&a, ps) in packets {
        let (sent, _) = sent_messages(ps, MATCH);
        if sent.is_empty() {
            continue;
        }
        let fits: Vec<(&K, usize)> = links
            .iter()
            .filter_map(|(k, writes)| {
                let n = sent.iter().zip(writes).take_while(|(x, y)| x == y).count();
                (n > 0 && n >= sent.len().min(writes.len())).then_some((k, n))
            })
            .collect();
        let Some(&(k, n)) = fits.iter().max_by_key(|(_, n)| *n) else {
            continue;
        };
        if fits.iter().filter(|(_, m)| *m == n).count() == 1 {
            best.insert(a, k.clone());
            *taken.entry(k.clone()).or_default() += 1;
        }
    }
    best.retain(|_, k| taken.get(k).copied() == Some(1));
    best
}

/// Which way an event's packet went: `out` (the association sent it),
/// `in` (it received it); a timer has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Out,
    In,
}

impl Dir {
    pub fn name(self) -> &'static str {
        match self {
            Dir::Out => "out",
            Dir::In => "in",
        }
    }
}

/// What an event was (`sctp_events.what`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum What {
    /// A packet of DATA its sender sent for the first time.
    Data,
    /// A SACK alone.
    Sack,
    /// First-sent DATA with a SACK bundled.
    DataSack,
    /// A packet that resent DATA.
    Rtx,
    /// A fast retransmission: resent DATA alone, past the window, at the
    /// SACK that found it missing.
    FastRtx,
    /// The retransmission timer fired.
    T3,
    /// Any other packet: the handshake's, a heartbeat, a stream reset.
    Control,
}

impl What {
    pub fn name(self) -> &'static str {
        match self {
            What::Data => "data",
            What::Sack => "sack",
            What::DataSack => "data_sack",
            What::Rtx => "rtx",
            What::FastRtx => "fast_rtx",
            What::T3 => "t3",
            What::Control => "control",
        }
    }
}

/// An event of an association (see the [module docs](self)), one row of
/// the tables' `sctp_events`, whose writer adds its node, peer, link and
/// ordinal. TSNs count from the initial TSN of their direction (0 its
/// first chunk, -1 a SACK before any).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SctpEvent {
    /// Its time, from the run's start.
    pub at: Duration,
    pub dir: Option<Dir>,
    pub what: What,
    /// The first and last TSN of the DATA it carried.
    pub tsn: Option<(i64, i64)>,
    /// Its DATA chunks, their payload bytes, and its SCTP bytes (the
    /// common header and every chunk, padded).
    pub chunks: u32,
    pub data_bytes: u64,
    pub packet_bytes: u64,
    /// A SACK's cumulative TSN and gap blocks.
    pub cum_tsn: Option<i64>,
    pub gaps: Option<u32>,
    /// The association's window and flight as the event left them: on a
    /// packet it sent, on a SACK it took and on a timer; none on DATA it
    /// received.
    pub cwnd_bytes: Option<u64>,
    pub flight_bytes: Option<u64>,
    /// Whether its chunks were read from its length.
    pub estimated: bool,
}

/// One direction's TSNs: the initial TSNs its INITs and INIT ACKs
/// announced, the first, and the highest sent.
#[derive(Debug, Default)]
struct Tsns {
    announced: Vec<u32>,
    base: Option<u32>,
    top: Option<i64>,
}

/// How far past an announced initial TSN a direction's first DATA seen may
/// be and still count from it (its first chunks lost).
const ANNOUNCED_WITHIN: u32 = 1 << 16;

impl Tsns {
    fn rel(&self, tsn: u32) -> Option<i64> {
        Some(i64::from(tsn.wrapping_sub(self.base?) as i32))
    }

    /// Takes the DATA chunk `tsn`: whether it went before. The first
    /// counts from the nearest initial TSN announced at or before it, else
    /// from itself.
    fn take(&mut self, tsn: u32) -> bool {
        let announced = self
            .announced
            .iter()
            .map(|&a| tsn.wrapping_sub(a))
            .filter(|&d| d < ANNOUNCED_WITHIN)
            .min()
            .map(|d| tsn.wrapping_sub(d));
        let base = *self.base.get_or_insert(announced.unwrap_or(tsn));
        let rel = i64::from(tsn.wrapping_sub(base) as i32);
        let again = self.top.is_some_and(|t| rel <= t);
        self.top = Some(self.top.map_or(rel, |t| t.max(rel)));
        again
    }
}

/// An association's events from its logged packets: each packet a row,
/// its TSNs counted from its direction's initial TSN (as its INIT or INIT
/// ACK announced it; from its first DATA without one),
/// its flight as the log shows it (the chunks it sent past its SACKs'
/// cumulative TSN and gap blocks; dcSCTP also leaves out chunks it marked
/// lost), and no window (dcSCTP's isn't in its packets).
#[derive(Debug, Default)]
pub struct Rows {
    sent: Tsns,
    got: Tsns,
    /// The chunks sent and not yet acknowledged, by TSN from the first,
    /// and their bytes.
    outstanding: BTreeMap<i64, u32>,
    flight: u64,
    /// Whether the last packet was a SACK it took with gap blocks.
    gapped: bool,
    /// Whether it last sent a SACK with gap blocks.
    sent_gapped: bool,
}

impl Rows {
    /// The event of packet `p`, at `at`.
    pub fn event(&mut self, p: &Packet, at: Duration) -> SctpEvent {
        let tsns = if p.out { &mut self.sent } else { &mut self.got };
        for c in &p.chunks {
            if let &Chunk::Init { tsn, .. } = c {
                tsns.announced.push(tsn);
            }
        }
        let (mut chunks, mut data_bytes, mut resent, mut range) = (0, 0, 0, None::<(i64, i64)>);
        let mut sizes = Vec::new();
        for c in p.data() {
            let &Chunk::Data { tsn, len, .. } = c else {
                continue;
            };
            let again = tsns.take(tsn);
            let rel = tsns.rel(tsn).unwrap_or(0);
            chunks += 1;
            data_bytes += u64::from(len);
            resent += u32::from(again);
            range = Some(range.map_or((rel, rel), |(lo, hi)| (lo.min(rel), hi.max(rel))));
            if p.out && !again {
                sizes.push((rel, c.size()));
            }
        }
        let sack = p.sack();
        // A SACK acknowledges the other direction's TSNs.
        let acked = if p.out { &self.got } else { &self.sent };
        let cum_tsn = sack.and_then(|(cum, _)| acked.rel(cum).or(Some(-1)));
        let gaps = sack.map(|(_, g)| u32::try_from(g.len()).unwrap_or(u32::MAX));
        for (rel, size) in sizes {
            self.outstanding.insert(rel, size);
            self.flight += u64::from(size);
        }
        if !p.out
            && let (Some((_, blocks)), Some(cum)) = (sack, cum_tsn)
        {
            let mut gone: Vec<i64> = self.outstanding.range(..=cum).map(|(&t, _)| t).collect();
            for &(s, e) in blocks {
                let (s, e) = (cum + i64::from(s), cum + i64::from(e));
                gone.extend(self.outstanding.range(s..=e).map(|(&t, _)| t));
            }
            for t in gone {
                if let Some(size) = self.outstanding.remove(&t) {
                    self.flight -= u64::from(size);
                }
            }
        }
        let after_gap = if p.out { self.gapped } else { self.sent_gapped };
        let what = if chunks > 0 {
            if resent == chunks && after_gap && sack.is_none() {
                What::FastRtx
            } else if resent > 0 {
                What::Rtx
            } else if sack.is_some() {
                What::DataSack
            } else {
                What::Data
            }
        } else if sack.is_some() {
            What::Sack
        } else {
            What::Control
        };
        let has_gaps = gaps.is_some_and(|g| g > 0);
        if p.out {
            self.gapped = false;
            if sack.is_some() {
                self.sent_gapped = has_gaps;
            }
        } else {
            self.gapped = has_gaps;
        }
        let state = p.out || sack.is_some();
        SctpEvent {
            at,
            dir: Some(if p.out { Dir::Out } else { Dir::In }),
            what,
            tsn: range,
            chunks,
            data_bytes,
            packet_bytes: u64::from(p.bytes),
            cum_tsn,
            gaps,
            cwnd_bytes: None,
            flight_bytes: state.then_some(self.flight),
            estimated: false,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

    use super::*;

    const PREFIX: &str = "[121:145:1118/142803.899030:VERBOSE1:third_party/webrtc/net/dcsctp/public/text_pcap_packet_observer.cc:23] ";
    /// An INIT, as Chromium's observer logged it.
    const INIT: &str = "O 00:00:36.475 0000 13 88 13 88 00 00 00 00 63 69 fe e8 01 00 00 1e a1 d1 9c c2 00 50 00 00 ff ff ff ff 40 08 bb 51 c0 00 00 04 80 08 00 06 82 c0 00 00 # SCTP_PACKET DcSctpTransport1";

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!(" {x:02x}")).collect()
    }

    /// A packet of `chunks` (each a chunk's bytes, unpadded) under tag 7.
    fn packet(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut b = vec![0x13, 0x88, 0x13, 0x88, 0, 0, 0, 7, 0, 0, 0, 0];
        for c in chunks {
            b.extend(c);
            b.resize(b.len().next_multiple_of(4), 0);
        }
        b
    }

    fn data(tsn: u32, ssn: u16, len: u16, flags: u8) -> Vec<u8> {
        let mut c = vec![0, flags];
        c.extend((len + 16).to_be_bytes());
        c.extend(tsn.to_be_bytes());
        c.extend(1u16.to_be_bytes());
        c.extend(ssn.to_be_bytes());
        c.extend(53u32.to_be_bytes());
        c.resize(16 + usize::from(len), 0xab);
        c
    }

    /// An I-DATA chunk: TSN, stream 1, reserved, message id `mid`, FSN 0.
    fn idata(tsn: u32, mid: u32, len: u16, flags: u8) -> Vec<u8> {
        let mut c = vec![64, flags];
        c.extend((len + 20).to_be_bytes());
        c.extend(tsn.to_be_bytes());
        c.extend(1u16.to_be_bytes());
        c.extend(0xffffu16.to_be_bytes());
        c.extend(mid.to_be_bytes());
        c.extend(0u32.to_be_bytes());
        c.resize(20 + usize::from(len), 0xab);
        c
    }

    #[test]
    fn an_idata_chunk_reads_its_message_id() {
        let p = Packet::of_bytes(0, true, 0, &packet(&[idata(7, 0x0001_0005, 30, 3)])).unwrap();
        assert_eq!(
            p.chunks,
            vec![Chunk::Data {
                tsn: 7,
                sid: 1,
                ssn: 5,
                len: 30,
                flags: 3,
                idata: true
            }]
        );
        assert_eq!(p.chunks[0].size(), 20 + 32);
    }

    fn sack(cum: u32, gaps: &[(u16, u16)]) -> Vec<u8> {
        let mut c = vec![3, 0];
        c.extend((16 + 4 * gaps.len() as u16).to_be_bytes());
        c.extend(cum.to_be_bytes());
        c.extend(5_242_880u32.to_be_bytes());
        c.extend((gaps.len() as u16).to_be_bytes());
        c.extend(0u16.to_be_bytes());
        for &(s, e) in gaps {
            c.extend(s.to_be_bytes());
            c.extend(e.to_be_bytes());
        }
        c
    }

    fn line(out: bool, assoc: u32, b: &[u8]) -> String {
        let d = if out { "O" } else { "I" };
        format!(
            "{d} 00:00:36.475 0000{} # SCTP_PACKET DcSctpTransport{assoc}",
            hex(b)
        )
    }

    /// 2026-11-18 14:28:03 UTC, a few ms after [`PREFIX`]'s time.
    const NOW: u64 = 1_795_012_083_905_000;

    #[test]
    fn the_observer_reads_a_packets_three_lines() {
        let mut o = Observer::default();
        assert_eq!(
            o.take(
                "[11:23:1118/142803.754136:ERROR:dbus/bus.cc:405] no bus",
                NOW
            ),
            Line::Other
        );
        assert_eq!(o.take(PREFIX, NOW), Line::Part);
        assert_eq!(o.take("", NOW), Line::Part);
        let Line::Packet(p) = o.take(INIT, NOW) else {
            panic!("no packet");
        };
        assert_eq!(p.t_us, NOW - 905_000 + 899_030);
        assert!(p.out);
        assert_eq!(p.assoc, 1);
        assert_eq!(p.vtag, 0);
        assert_eq!(p.bytes, 44);
        assert_eq!(
            p.chunks,
            vec![Chunk::Init {
                ack: false,
                tag: 0xa1d1_9cc2,
                tsn: 0x4008_bb51
            }]
        );
        // The empty line ends nothing without a prefix before it.
        assert_eq!(o.take("", NOW), Line::Other);
        assert_eq!(o.take("plain", NOW), Line::Other);
        assert_eq!(o.unread, 0);
    }

    #[test]
    fn a_prefix_dates_its_time_of_day_nearest_the_reading() {
        // Just past midnight, a prefix of the day before.
        let midnight = NOW - NOW % DAY_US;
        assert_eq!(dated(DAY_US - 1, midnight + 5), midnight - 1);
        assert_eq!(dated(3, midnight + DAY_US - 2), midnight + DAY_US + 3);
        assert_eq!(dated(7, midnight + 9), midnight + 7);
    }

    #[test]
    fn a_packets_chunks_round_trip_through_its_line() {
        let b = packet(&[
            sack(9, &[(2, 3), (5, 5)]),
            data(10, 4, 1160, 2),
            data(11, 4, 33, 9),
        ]);
        let mut o = Observer::default();
        o.take(PREFIX, NOW);
        o.take("", NOW);
        let Line::Packet(p) = o.take(&line(false, 3, &b), NOW) else {
            panic!("no packet");
        };
        assert!(!p.out);
        assert_eq!(p.bytes as usize, b.len());
        assert_eq!(
            p.chunks[0],
            Chunk::Sack {
                cum: 9,
                rwnd: 5_242_880,
                gaps: vec![(2, 3), (5, 5)],
                dups: 0
            }
        );
        assert_eq!(
            p.chunks[2],
            Chunk::Data {
                tsn: 11,
                sid: 1,
                ssn: 4,
                len: 33,
                flags: 9,
                idata: false
            }
        );
        assert_eq!(p.chunks[2].size(), 16 + 36);
        assert_eq!(Packet::parse(&p.json()), Some(p));
    }

    #[test]
    fn a_line_cut_short_is_unread() {
        let mut b = packet(&[data(10, 4, 100, 3)]);
        b.truncate(40);
        let mut o = Observer::default();
        o.take(PREFIX, NOW);
        assert_eq!(o.take(&line(true, 0, &b), NOW), Line::Part);
        assert_eq!(o.unread, 1);
    }

    fn pkt(out: bool, chunks: &[Vec<u8>]) -> Packet {
        Packet::of_bytes(0, out, 0, &packet(chunks)).unwrap()
    }

    #[test]
    fn rows_count_tsns_resends_and_the_flight_the_sacks_leave() {
        let mut r = Rows::default();
        let at = Duration::ZERO;
        let e = r.event(
            &pkt(true, &[data(100, 0, 1160, 2), data(101, 0, 40, 1)]),
            at,
        );
        assert_eq!(
            (e.what, e.tsn, e.chunks, e.data_bytes),
            (What::Data, Some((0, 1)), 2, 1200)
        );
        assert_eq!(e.flight_bytes, Some(1176 + 56));
        let e = r.event(&pkt(true, &[data(102, 1, 1160, 3)]), at);
        assert_eq!(e.flight_bytes, Some(1176 + 56 + 1176));
        let e = r.event(&pkt(true, &[data(103, 2, 1160, 3)]), at);
        assert_eq!(e.tsn, Some((3, 3)));
        // 100 acknowledged, 102 past a gap.
        let e = r.event(&pkt(false, &[sack(100, &[(2, 2)])]), at);
        assert_eq!((e.what, e.cum_tsn, e.gaps), (What::Sack, Some(0), Some(1)));
        assert_eq!(e.flight_bytes, Some(56 + 1176));
        // 101 resent at the SACK that found it missing.
        let e = r.event(&pkt(true, &[data(101, 0, 40, 1)]), at);
        assert_eq!((e.what, e.tsn), (What::FastRtx, Some((1, 1))));
        assert_eq!(e.flight_bytes, Some(56 + 1176), "a resend adds nothing");
        let e = r.event(&pkt(true, &[data(101, 0, 40, 1), data(104, 3, 8, 3)]), at);
        assert_eq!(e.what, What::Rtx);
        let e = r.event(&pkt(false, &[sack(104, &[])]), at);
        assert_eq!((e.cum_tsn, e.flight_bytes), (Some(4), Some(0)));
        // The peer's DATA counts from the initial TSN its INIT announced,
        // its first chunk lost.
        let mut init = pkt(false, &[]);
        init.chunks.push(Chunk::Init {
            ack: false,
            tag: 5,
            tsn: 8999,
        });
        let e = r.event(&init, at);
        assert_eq!((e.what, e.flight_bytes), (What::Control, None));
        let e = r.event(&pkt(false, &[data(9000, 0, 25, 3)]), at);
        assert_eq!(
            (e.dir, e.what, e.tsn, e.flight_bytes),
            (Some(Dir::In), What::Data, Some((1, 1)), None)
        );
        let e = r.event(&pkt(true, &[sack(9000, &[]), data(105, 4, 8, 3)]), at);
        assert_eq!(
            (e.what, e.cum_tsn, e.tsn),
            (What::DataSack, Some(1), Some((5, 5)))
        );
    }

    #[test]
    fn associations_are_named_by_the_writes_they_carried() {
        let sent = |assoc: u32, sizes: &[u16]| -> Vec<Packet> {
            sizes
                .iter()
                .enumerate()
                .map(|(i, &len)| {
                    let tsn = 1000 + i as u32;
                    let mut p = pkt(true, &[data(tsn, i as u16, len, 3)]);
                    p.assoc = assoc;
                    p
                })
                .collect()
        };
        let packets: BTreeMap<u32, Vec<Packet>> = [
            (0, sent(0, &[25, 300, 4000])),
            (1, sent(1, &[25, 310])),
            (2, sent(2, &[25])),
        ]
        .into_iter()
        .collect();
        let links = vec![
            ("a", vec![25, 300, 4000, 12]),
            ("b", vec![25, 310, 7]),
            ("c", vec![26]),
        ];
        let named = name_assocs(&packets, &links);
        assert_eq!(named.get(&0), Some(&"a"));
        assert_eq!(named.get(&1), Some(&"b"));
        // 25 fits a and b alike.
        assert_eq!(named.get(&2), None);
    }
}
