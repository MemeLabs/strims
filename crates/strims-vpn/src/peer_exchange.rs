// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Linking to hosts known only through the network, on port 12.
//!
//! Two hosts trade `LinkDescription`s (listener URIs, WebRTC session
//! descriptions) in encrypted messages. vpn runs the exchange; the driver
//! owns the link candidates (a `LinkCandidatePool`, see
//! `strims_vnic::interface`) and does the dialing, keyed by [`ExchangeId`]:
//!
//! - **Connect** ([`Host::connect`]):
//!   [`Event::GatherLocalDescriptions`] → the driver answers with
//!   [`Host::set_local_descriptions`] → a `LinkOffer` goes out → the answer
//!   either reports success, an error, or carries the remote's
//!   descriptions → [`Event::DialRemoteDescriptions`] → the driver answers
//!   with [`Host::set_remote_descriptions_result`] →
//!   [`Event::ExchangeClosed`].
//! - **Offer**: refused with an error answer when the
//!   host is already a peer or the vnic peer budget is spent; otherwise
//!   [`Event::DialRemoteDescriptions`] with the offer's descriptions. If that
//!   connects, an empty answer reports it; if not,
//!   [`Event::GatherLocalDescriptions`] and the answer carries ours, so the
//!   offerer can dial us. The exchange then stays open until its timeout,
//!   for links that complete in the background (a WebRTC answerer).
//!
//! Every exchange ends [`LINK_TIMEOUT`] after it started, so a lost answer
//! can't block later connects to that host. Only an answer carrying the
//! exchange's id counts.

use std::collections::{BTreeMap, VecDeque};

use slotmap::SlotMap;
use strims_base::time::{Duration, Time};
use strims_kademlia::ID;
use strims_proto::vpn::v1::PeerExchangeMessage;
use strims_proto::vpn::v1::peer_exchange_message::{Body, LinkAnswer, LinkOffer};
use strims_vnic::ports::PEER_EXCHANGE_PORT;

use crate::host::{Event, Host, Inbound, random_u64};
use crate::{Error, ExchangeId, LinkDescription, NetworkId, VnicHost};

/// How long an exchange lives.
pub const LINK_TIMEOUT: Duration = Duration::from_secs(20);

/// Error texts sent in `LinkAnswer.error_message`.
pub const ERR_PEER_LINK_OPEN: &str = "peer link already open";
pub const ERR_NO_NEW_PEERS: &str = "unable to add new peers";

/// How an exchange ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExchangeOutcome {
    /// Our candidates connected with the remote's descriptions.
    Connected,
    /// Our candidates tried the remote's descriptions without connecting.
    NotConnected,
    /// The answerer reported it connected using our descriptions (an
    /// answer with no descriptions and no error).
    RemoteConnected,
    /// The answer carried an error (for example [`ERR_NO_NEW_PEERS`]).
    Rejected(String),
    /// We answered an offer with our descriptions; the offerer may have
    /// connected.
    Answered,
    /// A local step failed (gathering, dialing, or sending).
    Failed(String),
    /// No answer in time.
    TimedOut,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// Outgoing: waiting for our descriptions to offer.
    GatherOffer,
    /// Outgoing: offer sent.
    AwaitAnswer,
    /// Outgoing: dialing the answer's descriptions.
    DialAnswer,
    /// Incoming: dialing the offer's descriptions.
    DialOffer,
    /// Incoming: waiting for our descriptions to answer with.
    GatherAnswer,
    /// Incoming: answered with our descriptions; open until the timeout.
    Answered,
}

#[derive(Debug)]
pub(crate) struct Exchange {
    network: NetworkId,
    host_id: ID,
    /// `exchange_id` on the wire: ours for outgoing, the offerer's for
    /// incoming.
    wire_id: u64,
    state: State,
    deadline: Time,
}

/// A link offer waiting for `flush`, which can see vnic's peer table.
#[derive(Debug)]
pub(crate) struct PendingOffer {
    network: NetworkId,
    src: ID,
    wire_id: u64,
    descriptions: Vec<LinkDescription>,
}

/// Peer exchange state for every network.
#[derive(Debug, Default)]
pub(crate) struct PeerExchangeService {
    /// Outgoing ones are bounded by the owner's calls, incoming ones by
    /// `Config::max_incoming_exchanges`.
    exchanges: SlotMap<ExchangeId, Exchange>,
    /// Outgoing exchanges awaiting an answer, per (network, host).
    outgoing: BTreeMap<(NetworkId, ID), ExchangeId>,
    /// Bounded by `Config::max_pending_offers`.
    pub(crate) pending: VecDeque<PendingOffer>,
    incoming: usize,
}

impl<H: Copy> Host<H> {
    /// Starts an exchange to link with `host_id`. Returns `None` when it is
    /// this host, already a vnic peer, or has an exchange in progress.
    pub fn connect<T: Copy>(
        &mut self,
        vnic: &VnicHost<T>,
        now: Time,
        net: NetworkId,
        host_id: ID,
    ) -> Result<Option<ExchangeId>, Error> {
        if !self.networks.contains_key(net) {
            return Err(Error::NetworkNotFound);
        }
        if vnic.has_peer(&host_id) || host_id == self.id {
            return Ok(None);
        }
        if self.peer_exchange.outgoing.contains_key(&(net, host_id)) {
            return Ok(None);
        }
        let wire_id = random_u64(&mut self.rng);
        let x = self.peer_exchange.exchanges.insert(Exchange {
            network: net,
            host_id,
            wire_id,
            state: State::GatherOffer,
            deadline: now + LINK_TIMEOUT,
        });
        self.peer_exchange.outgoing.insert((net, host_id), x);
        self.push_event(Event::GatherLocalDescriptions {
            network: net,
            exchange: x,
            host_id,
        });
        Ok(Some(x))
    }

    /// The driver's answer to [`Event::GatherLocalDescriptions`].
    pub fn set_local_descriptions(
        &mut self,
        now: Time,
        exchange: ExchangeId,
        result: Result<Vec<LinkDescription>, String>,
    ) -> Result<(), Error> {
        let x = self
            .peer_exchange
            .exchanges
            .get(exchange)
            .ok_or(Error::UnknownRequest)?;
        let (net, host, wire_id, state) = (x.network, x.host_id, x.wire_id, x.state);
        match (state, result) {
            (State::GatherOffer, Ok(descriptions)) => {
                let msg = Body::LinkOffer(LinkOffer {
                    exchange_id: wire_id,
                    descriptions,
                });
                match self.send_exchange(now, net, host, msg) {
                    Ok(()) => self.set_state(exchange, State::AwaitAnswer),
                    Err(e) => self.close_exchange(exchange, ExchangeOutcome::Failed(e.to_string())),
                }
            }
            (State::GatherOffer, Err(e)) => {
                self.close_exchange(exchange, ExchangeOutcome::Failed(e))
            }
            (State::GatherAnswer, Ok(descriptions)) => {
                match self.send_answer(now, net, host, wire_id, descriptions, None) {
                    Ok(()) => self.set_state(exchange, State::Answered),
                    Err(e) => self.close_exchange(exchange, ExchangeOutcome::Failed(e.to_string())),
                }
            }
            (State::GatherAnswer, Err(e)) => {
                let _ = self.send_answer(now, net, host, wire_id, vec![], Some(&e));
                self.close_exchange(exchange, ExchangeOutcome::Failed(e));
            }
            _ => return Err(Error::UnknownRequest),
        }
        Ok(())
    }

    /// The driver's answer to [`Event::DialRemoteDescriptions`]: whether a
    /// link connected.
    pub fn set_remote_descriptions_result(
        &mut self,
        now: Time,
        exchange: ExchangeId,
        result: Result<bool, String>,
    ) -> Result<(), Error> {
        let x = self
            .peer_exchange
            .exchanges
            .get(exchange)
            .ok_or(Error::UnknownRequest)?;
        let (net, host, wire_id, state) = (x.network, x.host_id, x.wire_id, x.state);
        match (state, result) {
            (State::DialAnswer, Ok(true)) => {
                self.close_exchange(exchange, ExchangeOutcome::Connected)
            }
            (State::DialAnswer, Ok(false)) => {
                self.close_exchange(exchange, ExchangeOutcome::NotConnected)
            }
            (State::DialAnswer, Err(e)) => {
                self.close_exchange(exchange, ExchangeOutcome::Failed(e))
            }
            (State::DialOffer, Ok(true)) => {
                let _ = self.send_answer(now, net, host, wire_id, vec![], None);
                self.close_exchange(exchange, ExchangeOutcome::Connected);
            }
            // The offerer may still reach our descriptions.
            (State::DialOffer, Ok(false) | Err(_)) => self.gather_answer(exchange),
            _ => return Err(Error::UnknownRequest),
        }
        Ok(())
    }

    /// Exchanges in progress.
    pub fn exchanges(&self) -> impl Iterator<Item = (ExchangeId, NetworkId, ID)> + '_ {
        self.peer_exchange
            .exchanges
            .iter()
            .map(|(k, x)| (k, x.network, x.host_id))
    }

    fn set_state(&mut self, exchange: ExchangeId, state: State) {
        if let Some(x) = self.peer_exchange.exchanges.get_mut(exchange) {
            x.state = state;
        }
    }

    fn gather_answer(&mut self, exchange: ExchangeId) {
        let Some(x) = self.peer_exchange.exchanges.get_mut(exchange) else {
            return;
        };
        x.state = State::GatherAnswer;
        let (network, host_id) = (x.network, x.host_id);
        self.push_event(Event::GatherLocalDescriptions {
            network,
            exchange,
            host_id,
        });
    }

    fn close_exchange(&mut self, exchange: ExchangeId, outcome: ExchangeOutcome) {
        let Some(x) = self.peer_exchange.exchanges.remove(exchange) else {
            return;
        };
        let s = &mut self.peer_exchange;
        if s.outgoing.get(&(x.network, x.host_id)) == Some(&exchange) {
            s.outgoing.remove(&(x.network, x.host_id));
        }
        if matches!(
            x.state,
            State::DialOffer | State::GatherAnswer | State::Answered
        ) {
            s.incoming = s.incoming.saturating_sub(1);
        }
        self.push_event(Event::ExchangeClosed {
            network: x.network,
            exchange,
            host_id: x.host_id,
            outcome,
        });
    }

    fn send_exchange(
        &mut self,
        now: Time,
        net: NetworkId,
        host: ID,
        body: Body,
    ) -> Result<(), Error> {
        let msg = PeerExchangeMessage { body: Some(body) };
        self.send_proto(now, net, host, PEER_EXCHANGE_PORT, PEER_EXCHANGE_PORT, &msg)
    }

    /// Answers an offer with our descriptions or an error.
    fn send_answer(
        &mut self,
        now: Time,
        net: NetworkId,
        host: ID,
        wire_id: u64,
        descriptions: Vec<LinkDescription>,
        err: Option<&str>,
    ) -> Result<(), Error> {
        let body = Body::LinkAnswer(LinkAnswer {
            exchange_id: wire_id,
            descriptions,
            error_message: err.unwrap_or_default().to_owned(),
        });
        self.send_exchange(now, net, host, body)
    }

    /// Port 12.
    pub(crate) fn peer_exchange_handle(
        &mut self,
        _now: Time,
        net: NetworkId,
        msg: &Inbound<'_>,
    ) -> Result<(), Error> {
        let m = <PeerExchangeMessage as prost::Message>::decode(msg.body)
            .map_err(|e| Error::Decode(e.to_string()))?;
        match m.body {
            Some(Body::LinkOffer(offer)) => {
                // Answering needs vnic's peer table, so it waits for the
                // next flush.
                let s = &mut self.peer_exchange;
                if s.pending.len() < self.config.max_pending_offers {
                    s.pending.push_back(PendingOffer {
                        network: net,
                        src: msg.src,
                        wire_id: offer.exchange_id,
                        descriptions: offer.descriptions,
                    });
                }
                Ok(())
            }
            Some(Body::LinkAnswer(answer)) => {
                self.handle_link_answer(net, msg.src, answer);
                Ok(())
            }
            Some(Body::Rejection(_)) | None => Err(Error::UnexpectedMessageType),
        }
    }

    /// Only an answer carrying the id of an exchange awaiting one counts,
    /// so a stale or forged answer can't end the live exchange.
    fn handle_link_answer(&mut self, net: NetworkId, src: ID, answer: LinkAnswer) {
        let Some(&exchange) = self.peer_exchange.outgoing.get(&(net, src)) else {
            return;
        };
        let Some(x) = self.peer_exchange.exchanges.get(exchange) else {
            return;
        };
        if x.wire_id != answer.exchange_id || x.state != State::AwaitAnswer {
            return;
        }
        // A new connect may start while this one dials.
        self.peer_exchange.outgoing.remove(&(net, src));
        if answer.descriptions.is_empty() {
            let outcome = if answer.error_message.is_empty() {
                ExchangeOutcome::RemoteConnected
            } else {
                ExchangeOutcome::Rejected(answer.error_message)
            };
            self.close_exchange(exchange, outcome);
            return;
        }
        self.set_state(exchange, State::DialAnswer);
        self.push_event(Event::DialRemoteDescriptions {
            network: net,
            exchange,
            host_id: src,
            descriptions: answer.descriptions,
        });
    }

    /// Outgoing exchanges counted against the peer budget.
    fn mediator_count(&self, net: NetworkId) -> usize {
        self.peer_exchange
            .outgoing
            .range((net, ID::MIN)..=(net, ID::MAX))
            .count()
    }

    /// Answers queued link offers, or starts dialing their descriptions.
    pub(crate) fn peer_exchange_flush<T: Copy>(&mut self, vnic: &VnicHost<T>, now: Time) {
        while let Some(o) = self.peer_exchange.pending.pop_front() {
            if !self.networks.contains_key(o.network) {
                continue;
            }
            let allow = vnic.peer_count() + self.mediator_count(o.network) < vnic.max_peers()
                && self.peer_exchange.incoming < self.config.max_incoming_exchanges;
            let refuse = if vnic.has_peer(&o.src) {
                Some(ERR_PEER_LINK_OPEN)
            } else if !allow {
                Some(ERR_NO_NEW_PEERS)
            } else {
                None
            };
            if let Some(e) = refuse {
                let _ = self.send_answer(now, o.network, o.src, o.wire_id, vec![], Some(e));
                continue;
            }
            self.peer_exchange.incoming += 1;
            let exchange = self.peer_exchange.exchanges.insert(Exchange {
                network: o.network,
                host_id: o.src,
                wire_id: o.wire_id,
                state: State::DialOffer,
                deadline: now + LINK_TIMEOUT,
            });
            if o.descriptions.is_empty() {
                // Nothing to dial: answer with ours.
                self.gather_answer(exchange);
            } else {
                self.push_event(Event::DialRemoteDescriptions {
                    network: o.network,
                    exchange,
                    host_id: o.src,
                    descriptions: o.descriptions,
                });
            }
        }
    }

    pub(crate) fn peer_exchange_poll_timeout(&self) -> Option<Time> {
        self.peer_exchange
            .exchanges
            .values()
            .map(|x| x.deadline)
            .min()
    }

    pub(crate) fn peer_exchange_timeout(&mut self, now: Time) {
        let due: Vec<(ExchangeId, State)> = self
            .peer_exchange
            .exchanges
            .iter()
            .filter(|(_, x)| !now.before(x.deadline))
            .map(|(k, x)| (k, x.state))
            .collect();
        for (x, state) in due {
            let outcome = match state {
                State::Answered => ExchangeOutcome::Answered,
                _ => ExchangeOutcome::TimedOut,
            };
            self.close_exchange(x, outcome);
        }
    }

    pub(crate) fn peer_exchange_network_removed(&mut self, net: NetworkId) {
        self.peer_exchange.pending.retain(|o| o.network != net);
        let gone: Vec<ExchangeId> = self
            .peer_exchange
            .exchanges
            .iter()
            .filter(|(_, x)| x.network == net)
            .map(|(k, _)| k)
            .collect();
        for x in gone {
            self.close_exchange(x, ExchangeOutcome::Failed("network removed".to_owned()));
        }
    }
}
