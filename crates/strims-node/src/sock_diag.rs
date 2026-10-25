// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A TCP socket's state as the kernel reports it to `ss`: one exact
//! `NETLINK_SOCK_DIAG` query by the socket's addresses, answered with its
//! `inet_diag_msg` and `tcp_info` (Linux's `uapi/linux/inet_diag.h` and
//! `uapi/linux/tcp.h`). The layouts are fixed by the kernel's ABI and
//! read by offset, so a reply from an older kernel just has fewer fields.

use std::io;
use std::net::SocketAddr;
use std::os::fd::OwnedFd;

use rustix::net::netlink::{self, SocketAddrNetlink};
use rustix::net::{
    AddressFamily, RecvFlags, SendFlags, SocketFlags, SocketType, recv, sendto, socket_with,
};
use strims_base::time::{Duration, Time};
use strims_node_core::{CaptureSample, LinkProbe};

const NLMSG_HDR: usize = 16;
const NLMSG_ERROR: u16 = 2;
const NLMSG_DONE: u16 = 3;
const NLM_F_REQUEST: u16 = 1;
const SOCK_DIAG_BY_FAMILY: u16 = 20;
const IPPROTO_TCP: u8 = 6;
const AF_INET: u8 = 2;
const AF_INET6: u8 = 10;
/// `INET_DIAG_INFO`: the attribute holding `tcp_info`.
const INET_DIAG_INFO: u16 = 2;
/// `inet_diag_req_v2`: family, protocol, ext, pad, states, then the socket
/// id (ports, addresses, interface, cookie).
const REQ_LEN: usize = 8 + INET_DIAG_SOCKID;
const INET_DIAG_SOCKID: usize = 48;
/// `inet_diag_msg`: family, state, timer, retrans, the socket id,
/// expires, rqueue, wqueue, uid, inode.
const MSG_LEN: usize = 4 + INET_DIAG_SOCKID + 20;
const MSG_WQUEUE: usize = 4 + INET_DIAG_SOCKID + 8;

/// What a socket holds and how its connection runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TcpState {
    /// Bytes written and not yet sent (`tcpi_notsent_bytes`).
    pub(crate) unsent: u64,
    /// Bytes sent and not yet acknowledged.
    pub(crate) unacked: u64,
    /// The congestion window, in bytes (`tcpi_snd_cwnd` segments).
    pub(crate) cwnd: u64,
    /// The smoothed round trip and its mean deviation, in microseconds.
    pub(crate) srtt_us: u64,
    pub(crate) rttvar_us: u64,
    /// Segments retransmitted over the connection's life.
    pub(crate) retrans: u64,
    /// The delivery rate estimate, in bytes per second.
    pub(crate) delivery_rate: u64,
}

/// A netlink socket for queries about one TCP connection.
#[derive(Debug)]
pub(crate) struct SockDiag {
    fd: OwnedFd,
    local: SocketAddr,
    peer: SocketAddr,
    seq: u32,
    buf: Vec<u8>,
}

impl SockDiag {
    /// A query socket for the connection from `local` to `peer`.
    pub(crate) fn new(local: SocketAddr, peer: SocketAddr) -> io::Result<SockDiag> {
        let fd = socket_with(
            AddressFamily::NETLINK,
            SocketType::DGRAM,
            SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
            Some(netlink::SOCK_DIAG),
        )?;
        Ok(SockDiag {
            fd,
            local,
            peer,
            seq: 0,
            buf: vec![0; 4096],
        })
    }

    /// The connection's state now; `None` once it is gone, or if the
    /// kernel didn't answer at once.
    pub(crate) fn query(&mut self) -> Option<TcpState> {
        self.seq = self.seq.wrapping_add(1);
        let req = request(self.local, self.peer, self.seq);
        sendto(
            &self.fd,
            &req,
            SendFlags::empty(),
            &SocketAddrNetlink::new(0, 0),
        )
        .ok()?;
        // Drain the replies to this query and any late ones to earlier
        // queries; the kernel answers within the send.
        let mut state = None;
        while let Ok((n, _)) = recv(&self.fd, &mut self.buf[..], RecvFlags::empty()) {
            let reply = self.buf.get(..n)?;
            if let Some(s) = parse(reply, self.seq) {
                state = Some(s);
            }
        }
        state
    }
}

/// A link's TCP socket, read for the capture.
#[derive(Debug)]
pub(crate) struct TcpProbe {
    diag: SockDiag,
    /// The socket carries a WebSocket.
    ws: bool,
}

impl TcpProbe {
    /// A probe of the connected socket from `local` to `peer`.
    pub(crate) fn new(local: SocketAddr, peer: SocketAddr, ws: bool) -> io::Result<TcpProbe> {
        Ok(TcpProbe {
            diag: SockDiag::new(local, peer)?,
            ws,
        })
    }
}

impl LinkProbe for TcpProbe {
    fn sample(&mut self, _: Time) -> Option<CaptureSample> {
        let s = self.diag.query()?;
        let us = |v: u64| Duration::from_micros(i64::try_from(v).unwrap_or(i64::MAX));
        Some(CaptureSample::Tcp {
            ws: self.ws,
            unsent: s.unsent,
            unacked: s.unacked,
            cwnd: s.cwnd,
            srtt: us(s.srtt_us),
            rttvar: us(s.rttvar_us),
            retrans: s.retrans,
            delivery_rate: s.delivery_rate,
        })
    }
}

/// An exact `SOCK_DIAG_BY_FAMILY` request for the connection, asking for
/// `tcp_info`.
fn request(local: SocketAddr, peer: SocketAddr, seq: u32) -> [u8; NLMSG_HDR + REQ_LEN] {
    let mut b = [0u8; NLMSG_HDR + REQ_LEN];
    let (hdr, req) = b.split_at_mut(NLMSG_HDR);
    put(hdr, 0, &((NLMSG_HDR + REQ_LEN) as u32).to_ne_bytes());
    put(hdr, 4, &SOCK_DIAG_BY_FAMILY.to_ne_bytes());
    put(hdr, 6, &NLM_F_REQUEST.to_ne_bytes());
    put(hdr, 8, &seq.to_ne_bytes());
    let family = if local.is_ipv4() { AF_INET } else { AF_INET6 };
    put(req, 0, &[family, IPPROTO_TCP, 1 << (INET_DIAG_INFO - 1), 0]);
    put(req, 4, &u32::MAX.to_ne_bytes());
    let (_, id) = req.split_at_mut(8);
    put(id, 0, &local.port().to_be_bytes());
    put(id, 2, &peer.port().to_be_bytes());
    put(id, 4, &ip_bytes(local));
    put(id, 20, &ip_bytes(peer));
    // No interface; INET_DIAG_NOCOOKIE.
    put(id, 40, &[0xff; 8]);
    b
}

/// An address as `inet_diag_sockid` holds it: 4 bytes for IPv4, 16 for
/// IPv6, in network order.
fn ip_bytes(a: SocketAddr) -> [u8; 16] {
    let mut out = [0u8; 16];
    match a {
        SocketAddr::V4(v) => put(&mut out, 0, &v.ip().octets()),
        SocketAddr::V6(v) => put(&mut out, 0, &v.ip().octets()),
    }
    out
}

fn put(dst: &mut [u8], at: usize, src: &[u8]) {
    if let Some(d) = dst.get_mut(at..at + src.len()) {
        d.copy_from_slice(src);
    }
}

fn u32_at(b: &[u8], at: usize) -> Option<u64> {
    let a: [u8; 4] = b.get(at..at + 4)?.try_into().ok()?;
    Some(u32::from_ne_bytes(a).into())
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    let a: [u8; 8] = b.get(at..at + 8)?.try_into().ok()?;
    Some(u64::from_ne_bytes(a))
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    let a: [u8; 2] = b.get(at..at + 2)?.try_into().ok()?;
    Some(u16::from_ne_bytes(a))
}

/// The state in a reply datagram to query `seq`, if it holds one.
fn parse(mut reply: &[u8], seq: u32) -> Option<TcpState> {
    while reply.len() >= NLMSG_HDR {
        let len = u32_at(reply, 0)? as usize;
        let kind = u16_at(reply, 4)?;
        let msg_seq = u32_at(reply, 8)? as u32;
        let msg = reply.get(NLMSG_HDR..len)?;
        if kind == SOCK_DIAG_BY_FAMILY && msg_seq == seq {
            return state(msg);
        }
        if kind == NLMSG_ERROR || kind == NLMSG_DONE || len < NLMSG_HDR {
            return None;
        }
        reply = reply.get(len.next_multiple_of(4)..)?;
    }
    None
}

/// The state in an `inet_diag_msg` and its attributes.
fn state(msg: &[u8]) -> Option<TcpState> {
    let wqueue = u32_at(msg, MSG_WQUEUE)?;
    let mut attrs = msg.get(MSG_LEN..)?;
    while attrs.len() >= 4 {
        let len = usize::from(u16_at(attrs, 0)?);
        let kind = u16_at(attrs, 2)?;
        if len < 4 {
            return None;
        }
        if kind == INET_DIAG_INFO {
            return tcp_info(attrs.get(4..len)?, wqueue);
        }
        attrs = attrs.get(len.next_multiple_of(4)..)?;
    }
    None
}

/// `struct tcp_info`, by offset.
fn tcp_info(i: &[u8], wqueue: u64) -> Option<TcpState> {
    let mss = u32_at(i, 16)?;
    let unsent = u32_at(i, 144).unwrap_or(0);
    Some(TcpState {
        unsent,
        unacked: wqueue.saturating_sub(unsent),
        cwnd: u32_at(i, 80)?.saturating_mul(mss),
        srtt_us: u32_at(i, 68)?,
        rttvar_us: u32_at(i, 72)?,
        retrans: u32_at(i, 100)?,
        delivery_rate: u64_at(i, 160).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    use super::*;

    #[test]
    fn a_connected_socket_reads_its_state() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut a = TcpStream::connect(l.local_addr().unwrap()).unwrap();
        let (mut b, _) = l.accept().unwrap();
        a.write_all(&[7; 100_000]).unwrap();
        let mut got = vec![0; 100_000];
        b.read_exact(&mut got).unwrap();
        let mut d = SockDiag::new(a.local_addr().unwrap(), a.peer_addr().unwrap()).unwrap();
        let s = d.query().unwrap();
        assert!(s.cwnd > 0 && s.srtt_us > 0, "{s:?}");
        assert_eq!((s.unsent, s.unacked), (0, 0), "{s:?}");
        // Another query on the same socket answers too.
        assert!(d.query().is_some());
        drop((a, b));
        let mut gone = SockDiag::new(
            "127.0.0.1:1".parse().unwrap(),
            "127.0.0.1:2".parse().unwrap(),
        )
        .unwrap();
        assert_eq!(gone.query(), None);
    }
}
