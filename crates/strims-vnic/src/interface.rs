// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Link candidates and their signalling.
//!
//! Peer exchange (vpn) connects two hosts that aren't linked yet by trading
//! `LinkDescription`s over the network: the dialer offers the descriptions
//! of its candidates, the other side tries them and answers with its own.
//! A candidate is one interface's attempt at a link: a TCP or WebSocket
//! listener URI to dial, or a WebRTC peer connection.
//!
//! The driver implements [`LinkCandidate`] (it dials and runs ICE) and adds
//! the resulting transport to the [`crate::Host`] with `add_link`; vpn
//! drives a [`LinkCandidatePool`]. Local descriptions wait for ICE gathering
//! and remote ones for a dial or a data channel, so the pool reports
//! [`Poll::Pending`] until its candidates finish. The driver tells vpn to
//! poll again when a candidate makes progress (ICE gathered, dial finished,
//! data channel open and [`crate::Event::PeerAdded`] seen).
//!
//! A candidate names its interface ([`LinkCandidate::interface`]); the known
//! names are [`LinkInterface`]. Descriptions for interfaces no candidate has
//! are skipped.

use std::fmt;
use std::task::Poll;

pub use strims_proto::vnic::v1::LinkDescription;

/// The link interfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LinkInterface {
    Tcp,
    Ws,
    WebRtc,
}

impl LinkInterface {
    pub const ALL: [LinkInterface; 3] =
        [LinkInterface::Tcp, LinkInterface::Ws, LinkInterface::WebRtc];

    /// The `LinkDescription.interface` value.
    pub fn name(self) -> &'static str {
        match self {
            LinkInterface::Tcp => crate::tcp::TCP_SCHEME,
            LinkInterface::Ws => "ws",
            LinkInterface::WebRtc => crate::webrtc::WEBRTC_INTERFACE,
        }
    }

    /// The interface with this `LinkDescription.interface` value.
    pub fn from_name(name: &str) -> Option<LinkInterface> {
        LinkInterface::ALL.into_iter().find(|i| i.name() == name)
    }
}

/// What to dial for a URI, chosen by its scheme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialTarget {
    /// `tcp://host:port/<hex profile key>`: dial with
    /// [`crate::LinkConfig::tcp_dial`].
    Tcp(crate::tcp::TcpUri),
    /// `wss://...` (or `ws://...` when allowed): dial with
    /// [`crate::LinkConfig::ws`].
    Ws(crate::ws::WsDialTarget),
}

impl DialTarget {
    /// Parses a dial URI. `allow_insecure_ws` admits `ws`; the browser
    /// build passes `false`. Other schemes fail with "unsupported scheme".
    pub fn parse(uri: &str, allow_insecure_ws: bool) -> Result<DialTarget, crate::tcp::TcpError> {
        let scheme = uri
            .split_once("://")
            .map_or("", |(s, _)| s)
            .to_ascii_lowercase();
        if scheme == crate::tcp::TCP_SCHEME {
            return Ok(DialTarget::Tcp(crate::tcp::TcpUri::parse(uri)?));
        }
        if crate::ws::ws_valid_scheme(&scheme, allow_insecure_ws) {
            return Ok(DialTarget::Ws(crate::ws::WsDialTarget::parse(uri)?));
        }
        Err(crate::tcp::TcpError::InvalidUri(
            "unsupported scheme".to_owned(),
        ))
    }
}

/// A candidate's failure.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CandidateError {
    /// A transport connected but the vnic handshake failed. Stops
    /// [`LinkCandidatePool::poll_remote_descriptions`].
    #[error("peer init: {0}")]
    PeerInit(String),
    #[error("{0}")]
    Failed(String),
}

/// Several failures.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidateErrors(pub Vec<CandidateError>);

impl fmt::Display for CandidateErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, e) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{e}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CandidateErrors {}

/// One interface's link attempt, implemented by the driver.
pub trait LinkCandidate {
    /// The interface name this candidate handles (see [`LinkInterface`]).
    fn interface(&self) -> &str;

    /// This side's description. `Ready(Ok(None))` when the candidate has
    /// nothing to advertise (a TCP/WS interface without a listener).
    /// `Pending` while, for example, ICE gathering runs.
    fn poll_local_description(&mut self) -> Poll<Result<Option<LinkDescription>, CandidateError>>;

    /// Starts linking to a remote description: dial the URI, or apply the
    /// session description.
    fn set_remote_description(&mut self, d: &LinkDescription) -> Result<(), CandidateError>;

    /// The outcome of [`LinkCandidate::set_remote_description`]:
    /// `Ready(Ok(true))` once a link made through this candidate completed
    /// its handshake; `Ready(Ok(false))` if this candidate won't connect now
    /// (a WebRTC answerer connects in the background after sending its
    /// answer).
    fn poll_remote_description(&mut self) -> Poll<Result<bool, CandidateError>>;
}

#[derive(Debug)]
struct RemoteState {
    descriptions: Vec<LinkDescription>,
    /// The description being tried.
    di: usize,
    /// The candidate being tried for it.
    ci: usize,
    started: bool,
    errors: Vec<CandidateError>,
}

/// One candidate per interface for one exchange.
#[derive(Debug)]
pub struct LinkCandidatePool<C> {
    candidates: Vec<C>,
    /// Each candidate's local description, once ready.
    local: Vec<Option<Result<Option<LinkDescription>, CandidateError>>>,
    remote: Option<RemoteState>,
}

impl<C: LinkCandidate> LinkCandidatePool<C> {
    pub fn new(candidates: Vec<C>) -> LinkCandidatePool<C> {
        let local = candidates.iter().map(|_| None).collect();
        LinkCandidatePool {
            candidates,
            local,
            remote: None,
        }
    }

    /// The candidates that were created, or the combined errors if none
    /// was.
    pub fn from_results(
        results: impl IntoIterator<Item = Result<C, CandidateError>>,
    ) -> Result<LinkCandidatePool<C>, CandidateErrors> {
        let mut cs = Vec::new();
        let mut errs = Vec::new();
        for r in results {
            match r {
                Ok(c) => cs.push(c),
                Err(e) => errs.push(e),
            }
        }
        if cs.is_empty() {
            return Err(CandidateErrors(errs));
        }
        Ok(LinkCandidatePool::new(cs))
    }

    pub fn candidates(&self) -> &[C] {
        &self.candidates
    }

    pub fn candidates_mut(&mut self) -> &mut [C] {
        &mut self.candidates
    }

    /// Ready once every candidate answered: the descriptions in candidate
    /// order, or, if there are none, the combined errors (`Ok(vec![])` when
    /// there were no errors either).
    pub fn poll_local_descriptions(
        &mut self,
    ) -> Poll<Result<Vec<LinkDescription>, CandidateErrors>> {
        let mut pending = false;
        for (c, slot) in self.candidates.iter_mut().zip(self.local.iter_mut()) {
            if slot.is_none() {
                match c.poll_local_description() {
                    Poll::Ready(r) => *slot = Some(r),
                    Poll::Pending => pending = true,
                }
            }
        }
        if pending {
            return Poll::Pending;
        }
        let mut ds = Vec::new();
        let mut errs = Vec::new();
        for r in self.local.iter().flatten() {
            match r {
                Ok(Some(d)) => ds.push(d.clone()),
                Ok(None) => {}
                Err(e) => errs.push(e.clone()),
            }
        }
        if ds.is_empty() && !errs.is_empty() {
            return Poll::Ready(Err(CandidateErrors(errs)));
        }
        Poll::Ready(Ok(ds))
    }

    /// Starts trying `descriptions`. Poll the outcome with
    /// [`LinkCandidatePool::poll_remote_descriptions`].
    pub fn set_remote_descriptions(&mut self, descriptions: Vec<LinkDescription>) {
        self.remote = Some(RemoteState {
            descriptions,
            di: 0,
            ci: 0,
            started: false,
            errors: Vec::new(),
        });
    }

    /// Tries each remote description, in order, with each candidate of its
    /// interface, in order, one at a time. `Ready(Ok(true))` at the first
    /// connection; a [`CandidateError::PeerInit`] stops at once; otherwise
    /// `Ready(Ok(false))`, or the combined errors.
    pub fn poll_remote_descriptions(&mut self) -> Poll<Result<bool, CandidateErrors>> {
        let Some(st) = &mut self.remote else {
            return Poll::Ready(Ok(false));
        };
        loop {
            let Some(d) = st.descriptions.get(st.di) else {
                let errors = std::mem::take(&mut st.errors);
                self.remote = None;
                return Poll::Ready(if errors.is_empty() {
                    Ok(false)
                } else {
                    Err(CandidateErrors(errors))
                });
            };
            let next = self
                .candidates
                .iter()
                .enumerate()
                .skip(st.ci)
                .find(|(_, c)| c.interface() == d.interface)
                .map(|(i, _)| i);
            let Some(ci) = next else {
                st.di += 1;
                st.ci = 0;
                st.started = false;
                continue;
            };
            st.ci = ci;
            let Some(c) = self.candidates.get_mut(ci) else {
                continue;
            };
            let outcome = if st.started {
                c.poll_remote_description()
            } else {
                st.started = true;
                match c.set_remote_description(d) {
                    Ok(()) => c.poll_remote_description(),
                    Err(e) => Poll::Ready(Err(e)),
                }
            };
            match outcome {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(true)) => {
                    self.remote = None;
                    return Poll::Ready(Ok(true));
                }
                Poll::Ready(Err(e @ CandidateError::PeerInit(_))) => {
                    self.remote = None;
                    return Poll::Ready(Err(CandidateErrors(vec![e])));
                }
                Poll::Ready(Err(e)) => st.errors.push(e),
                Poll::Ready(Ok(false)) => {}
            }
            st.ci += 1;
            st.started = false;
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

    use super::*;

    /// A scripted candidate.
    #[derive(Debug)]
    struct Fake {
        iface: &'static str,
        local: Vec<Poll<Result<Option<LinkDescription>, CandidateError>>>,
        remote: Vec<Poll<Result<bool, CandidateError>>>,
        applied: Vec<String>,
    }

    impl LinkCandidate for Fake {
        fn interface(&self) -> &str {
            self.iface
        }
        fn poll_local_description(
            &mut self,
        ) -> Poll<Result<Option<LinkDescription>, CandidateError>> {
            self.local.remove(0)
        }
        fn set_remote_description(&mut self, d: &LinkDescription) -> Result<(), CandidateError> {
            self.applied.push(d.description.clone());
            Ok(())
        }
        fn poll_remote_description(&mut self) -> Poll<Result<bool, CandidateError>> {
            self.remote.remove(0)
        }
    }

    fn desc(iface: &str, d: &str) -> LinkDescription {
        LinkDescription {
            interface: iface.to_owned(),
            description: d.to_owned(),
        }
    }

    fn fake(iface: &'static str) -> Fake {
        Fake {
            iface,
            local: vec![],
            remote: vec![],
            applied: vec![],
        }
    }

    #[test]
    fn local_descriptions_wait_for_all() {
        let mut a = fake("tcp");
        a.local = vec![Poll::Ready(Ok(Some(desc("tcp", "tcp://x"))))];
        let mut b = fake("webrtc");
        b.local = vec![Poll::Pending, Poll::Ready(Ok(Some(desc("webrtc", "{}"))))];
        let mut c = fake("ws");
        c.local = vec![Poll::Ready(Ok(None))];
        let mut p = LinkCandidatePool::new(vec![a, b, c]);
        assert!(p.poll_local_descriptions().is_pending());
        let ds = match p.poll_local_descriptions() {
            Poll::Ready(Ok(ds)) => ds,
            other => panic!("{other:?}"),
        };
        assert_eq!(ds, vec![desc("tcp", "tcp://x"), desc("webrtc", "{}")]);

        let mut e = fake("tcp");
        e.local = vec![Poll::Ready(Err(CandidateError::Failed("x".into())))];
        let mut p = LinkCandidatePool::new(vec![e]);
        assert!(matches!(p.poll_local_descriptions(), Poll::Ready(Err(_))));
    }

    #[test]
    fn remote_descriptions_are_tried_in_order() {
        let mut ws = fake("ws");
        ws.remote = vec![Poll::Ready(Err(CandidateError::Failed("dial".into())))];
        let mut rtc = fake("webrtc");
        rtc.remote = vec![Poll::Pending, Poll::Ready(Ok(true))];
        let mut p = LinkCandidatePool::new(vec![ws, rtc]);
        p.set_remote_descriptions(vec![
            desc("unknown", "?"),
            desc("ws", "wss://a"),
            desc("webrtc", "{sdp}"),
        ]);
        assert!(p.poll_remote_descriptions().is_pending());
        assert_eq!(p.poll_remote_descriptions(), Poll::Ready(Ok(true)));
        assert_eq!(p.candidates()[0].applied, vec!["wss://a"]);
        assert_eq!(p.candidates()[1].applied, vec!["{sdp}"]);

        // A peer init failure stops at once.
        let mut a = fake("tcp");
        a.remote = vec![Poll::Ready(Err(CandidateError::PeerInit("cert".into())))];
        let b = fake("tcp");
        let mut p = LinkCandidatePool::new(vec![a, b]);
        p.set_remote_descriptions(vec![desc("tcp", "1"), desc("tcp", "2")]);
        assert!(matches!(p.poll_remote_descriptions(), Poll::Ready(Err(e)) if e.0.len() == 1));
        assert!(p.candidates()[1].applied.is_empty());

        // Nothing connects: false, or the errors.
        let mut a = fake("tcp");
        a.remote = vec![Poll::Ready(Ok(false))];
        let mut p = LinkCandidatePool::new(vec![a]);
        p.set_remote_descriptions(vec![desc("tcp", "1")]);
        assert_eq!(p.poll_remote_descriptions(), Poll::Ready(Ok(false)));
        assert_eq!(LinkInterface::from_name("ws"), Some(LinkInterface::Ws));
        assert!(matches!(
            DialTarget::parse(&format!("tcp://h:1/{}", "ab".repeat(32)), false),
            Ok(DialTarget::Tcp(_))
        ));
        assert!(matches!(
            DialTarget::parse("wss://h/x", false),
            Ok(DialTarget::Ws(_))
        ));
        assert!(DialTarget::parse("ws://h/x", false).is_err());
        assert!(matches!(
            DialTarget::parse("ws://h/x", true),
            Ok(DialTarget::Ws(_))
        ));
        assert!(DialTarget::parse("webrtc://h/x", true).is_err());
        assert!(
            LinkCandidatePool::<Fake>::from_results(vec![Err(CandidateError::Failed("no".into()))])
                .is_err()
        );
    }
}
