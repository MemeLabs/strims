// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The testbed's topology file and who links to whom.
//!
//! ```text
//! node <i> <ip:port> [<kind>]
//! link <a> <b>
//! relay <ip:port> <ip:port>
//! browser-links ws|webrtc
//! ```
//!
//! Node 0 seeds. A node's kind is `native` (the default), `firefox` or
//! `chromium`. A `node` line gives a native node's TCP listener; browsers
//! can't listen, so theirs is only their address. A `relay` line gives the
//! relay's TCP and WebSocket listeners: a native node that joins the
//! testbed network and runs no swarm, through which browsers make their
//! WebRTC links (peer exchange). With a relay, every node dials it (TCP
//! from a native node, WebSocket from a browser) and joins the network.
//!
//! The links ([`Topology::plan`]):
//!
//! | link | made by | how |
//! |---|---|---|
//! | native–native | the higher index | a TCP dial |
//! | browser–native, `browser-links ws` (the default) | the browser | a WebSocket dial of the native's listener, on its TCP port + 1 |
//! | browser–native, `browser-links webrtc` | the browser | a peer exchange: the native runs WebRTC on UDP, its TCP port + 1, and no WebSocket listener, so the exchange can't settle on a WebSocket |
//! | browser–browser | the higher index | a peer exchange |

use std::net::SocketAddr;

use strims_crypto::key_from_seed;
use strims_proto::types::Key;

/// A node's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Native,
    Firefox,
    Chromium,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        match s {
            "native" => Some(Kind::Native),
            "firefox" => Some(Kind::Firefox),
            "chromium" => Some(Kind::Chromium),
            _ => None,
        }
    }

    pub fn is_browser(self) -> bool {
        self != Kind::Native
    }
}

/// How browsers link to native nodes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrowserLinks {
    #[default]
    Ws,
    WebRtc,
}

/// One `node` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeLine {
    pub index: usize,
    pub addr: SocketAddr,
    pub kind: Kind,
}

impl NodeLine {
    /// The native node's WebSocket listener (`browser-links ws`) or WebRTC
    /// UDP address (`browser-links webrtc`): its TCP port + 1.
    pub fn browser_addr(&self) -> SocketAddr {
        SocketAddr::new(self.addr.ip(), self.addr.port().wrapping_add(1))
    }
}

/// The relay's listeners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Relay {
    pub tcp: SocketAddr,
    pub ws: SocketAddr,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Topology {
    pub nodes: Vec<NodeLine>,
    pub links: Vec<(usize, usize)>,
    pub relay: Option<Relay>,
    pub browser_links: BrowserLinks,
}

/// A link a node makes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Link {
    /// Dial this URI.
    Dial(String),
    /// Exchange with this node through the relay, once its host id is
    /// known.
    Exchange(usize),
}

impl Topology {
    pub fn parse(text: &str) -> Result<Topology, String> {
        let mut t = Topology::default();
        for line in text.lines() {
            let w: Vec<&str> = line.split_whitespace().collect();
            let n = |s: &str| s.parse::<usize>().map_err(|e| format!("{line:?}: {e}"));
            let addr = |s: &str| {
                s.parse::<SocketAddr>()
                    .map_err(|e| format!("{line:?}: {e}"))
            };
            let kind = |s: &str| Kind::parse(s).ok_or_else(|| format!("{line:?}: bad kind"));
            match w.as_slice() {
                ["node", i, a] => t.nodes.push(NodeLine {
                    index: n(i)?,
                    addr: addr(a)?,
                    kind: Kind::Native,
                }),
                ["node", i, a, k] => t.nodes.push(NodeLine {
                    index: n(i)?,
                    addr: addr(a)?,
                    kind: kind(k)?,
                }),
                ["link", a, b] => t.links.push((n(a)?, n(b)?)),
                ["relay", tcp, ws] => {
                    t.relay = Some(Relay {
                        tcp: addr(tcp)?,
                        ws: addr(ws)?,
                    })
                }
                ["browser-links", "ws"] => t.browser_links = BrowserLinks::Ws,
                ["browser-links", "webrtc"] => t.browser_links = BrowserLinks::WebRtc,
                [] => {}
                _ => return Err(format!("bad topology line {line:?}")),
            }
        }
        if t.relay.is_none() && t.nodes.iter().any(|n| n.kind.is_browser()) {
            return Err("browser nodes need a relay".to_owned());
        }
        Ok(t)
    }

    pub fn node(&self, i: usize) -> Option<&NodeLine> {
        self.nodes.iter().find(|n| n.index == i)
    }

    /// The URI node `me` dials the relay at, if there is one.
    pub fn relay_uri(&self, me: usize) -> Option<String> {
        let relay = self.relay?;
        let key = hex::encode(relay_key().public);
        Some(match self.node(me)?.kind {
            Kind::Native => format!("tcp://{}/{key}", relay.tcp),
            _ => format!("ws://{}/{key}", relay.ws),
        })
    }

    /// The links node `me` makes, in topology order.
    pub fn plan(&self, me: usize) -> Vec<Link> {
        let Some(this) = self.node(me) else {
            return Vec::new();
        };
        self.links
            .iter()
            .filter_map(|&(a, b)| match (a == me, b == me) {
                (true, false) => self.node(b),
                (false, true) => self.node(a),
                _ => None,
            })
            .filter_map(|other| {
                let key = hex::encode(key_of(other.index).public);
                match (this.kind.is_browser(), other.kind.is_browser()) {
                    (false, false) if other.index < me => {
                        Some(Link::Dial(format!("tcp://{}/{key}", other.addr)))
                    }
                    (true, false) => Some(match self.browser_links {
                        BrowserLinks::Ws => {
                            Link::Dial(format!("ws://{}/{key}", other.browser_addr()))
                        }
                        BrowserLinks::WebRtc => Link::Exchange(other.index),
                    }),
                    (true, true) if other.index < me => Some(Link::Exchange(other.index)),
                    _ => None,
                }
            })
            .collect()
    }
}

/// Node `i`'s profile key.
pub fn key_of(i: usize) -> Key {
    key_from_seed(&[u8::try_from(i + 1).unwrap_or(u8::MAX); 32])
}

/// The relay's profile key, which is no node's.
pub fn relay_key() -> Key {
    key_from_seed(&[0xee; 32])
}

/// The key of the testbed's swarm.
pub fn swarm_key() -> Key {
    key_from_seed(&[0x55; 32])
}

/// The key of the network the relay and the nodes join.
pub fn network_key() -> Vec<u8> {
    key_from_seed(&[0xaa; 32]).public
}

/// The vpn port the nodes bind every peer to the network on.
pub const BIND_PORT: u16 = 10_000;

/// Node `i` among `n` nodes, by its profile key.
pub fn index_of(profile_key: &[u8], n: usize) -> Option<usize> {
    (0..n).find(|&i| key_of(i).public == profile_key)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    const MESH: &str = "node 0 10.77.0.10:4000\nnode 1 10.77.0.11:4000 firefox\n\
                        node 2 10.77.0.12:4000\nnode 3 10.77.0.13:4000 chromium\n\
                        relay 10.77.0.5:4000 10.77.0.5:4001\n\
                        link 0 1\nlink 0 2\nlink 0 3\nlink 1 2\nlink 1 3\nlink 2 3\n";

    fn dial_to(uri: &str) -> String {
        uri.split('/').nth(2).unwrap_or_default().to_owned()
    }

    #[test]
    fn natives_dial_lower_natives_and_browsers_make_their_links() {
        let t = Topology::parse(MESH).unwrap();
        assert_eq!(t.plan(0), vec![]);
        let two: Vec<String> = t
            .plan(2)
            .iter()
            .map(|l| match l {
                Link::Dial(u) => dial_to(u),
                Link::Exchange(i) => format!("x{i}"),
            })
            .collect();
        assert_eq!(two, vec!["10.77.0.10:4000"]);
        let one = t.plan(1);
        assert!(matches!(&one[..], [Link::Dial(a), Link::Dial(b)]
            if a.starts_with("ws://10.77.0.10:4001/") && b.starts_with("ws://10.77.0.12:4001/")));
        let three = t.plan(3);
        assert!(
            matches!(&three[..], [Link::Dial(a), Link::Exchange(1), Link::Dial(b)]
            if a.starts_with("ws://10.77.0.10:4001/") && b.starts_with("ws://10.77.0.12:4001/"))
        );
        assert!(t.relay_uri(0).unwrap().starts_with("tcp://10.77.0.5:4000/"));
        assert!(t.relay_uri(3).unwrap().starts_with("ws://10.77.0.5:4001/"));
    }

    #[test]
    fn webrtc_browser_links_exchange_with_natives() {
        let t = Topology::parse(&format!("{MESH}browser-links webrtc\n")).unwrap();
        assert_eq!(
            t.plan(3),
            vec![Link::Exchange(0), Link::Exchange(1), Link::Exchange(2)]
        );
    }

    #[test]
    fn all_native_topologies_parse_as_before() {
        let t =
            Topology::parse("node 0 10.77.0.10:4000\nnode 1 10.77.0.11:4000\nlink 0 1\n").unwrap();
        assert_eq!(t.relay, None);
        assert!(
            matches!(&t.plan(1)[..], [Link::Dial(u)] if u.starts_with("tcp://10.77.0.10:4000/"))
        );
        assert!(Topology::parse("node 0 10.77.0.10:4000 firefox\n").is_err());
    }
}
