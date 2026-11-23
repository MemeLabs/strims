// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! WebRTC on a net that models data channels ([`Net::models_channels`]:
//! the sim's window model). The node's endpoint is a [`ProxyRtc`], as a
//! browser worker's is, and [`run`] plays the window: each connection is
//! set up as the node's stack and its peer's set one up ([`LinkSetup`],
//! [`Net::link_setup`]), over streams of the net, and the answering side's
//! stream, carried as a modelled channel ([`Net::open_channel`], of
//! `WebRtcConfig::max_buffered_amount`), is attached as the connection's
//! link (`Input::Connected` with `LinkConfig::webrtc`). The driver writes
//! it as a browser's link, or, where the channels stand for a native
//! node's ([`Net::channels_in_process`]), as the node task writes an
//! in-process channel: an MTU of messages at once.
//!
//! - **Descriptions.** An offer is `strims-model <address> <token>
//!   <stack>` and an answer `strims-model answer <address> <token> <stack>
//!   <checks>`: the side's channel listener (TCP at its first WebRTC
//!   listen address and the public address), its connection's id, its
//!   stack ([`LinkSetup::name`]), and the answering side's socket for
//!   checks (UDP, at the same addresses). An offer leaves its stack's
//!   [`LinkSetup::offer_ready`] and [`LinkSetup::offer_post`] after it is
//!   asked for, an answer its [`LinkSetup::answer_ready`], one of a stack that ends its gathering at its check
//!   [`LinkSetup::answer_after_check`] after its check succeeds if that
//!   comes first. A host with draws ([`Net::setup_draws`]) draws its
//!   pages' parts of each, and ends a Chromium offer's gathering on its
//!   metronome's tick ([`LinkSetup::gather_after_set`]).
//! - **ICE.** Each side's connectivity check is a dial of the other's
//!   listener, whose connect is the check's round trip: the answering
//!   side's its stack's [`LinkSetup::check_ready`] after the offer comes,
//!   the offering side's once it has applied the answer. The answering
//!   side's connection carries the rest; the offering side's names the
//!   answer and ends. The offering side's check also sends the answering
//!   side's socket a datagram as its request, which tells the answering
//!   side, a one-way delay later, that the check reached it (or the dial's
//!   name, should the path lose the datagram).
//! - **DTLS and SCTP.** Over that connection, a modelled channel from its
//!   first byte, each side sends what its stack sends ([`Handshake`]): the
//!   answering side the offer's token first, as its check named the
//!   offering side's credentials, then the DTLS handshake's flights in the
//!   roles the answer took and the SCTP association's chunks, each flight
//!   one message ([`frame`]), read to its end and no further. A side whose
//!   association is up reports the connection up (`Signal(Connected)`) and
//!   attaches the stream, whose next message is its link's first: the
//!   page's writer paces the link's messages alone ([`Net::channel_up`]).
//! - **Failures.** A dial that fails, a stream the net can't model, and a
//!   stream that ends or breaks the handshake close the connection
//!   (`Signal(Closed)`). A stream whose token names no live offer is
//!   dropped, and so is an offer's on `Close`.

use std::collections::BTreeMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use strims_node_core::{
    Input, Inputs, NodeConfig, NodeError, ProxyRtc, RtcCommand, RtcConn, RtcFactory, RtcInput,
    RtcSignal,
};
use strims_vnic::LinkConfig;
use strims_vnic::webrtc::{SdpType, SessionDescription};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::Instant;

use crate::link::Transport;
use crate::net::Net;

const PREFIX: &str = "strims-model";

/// How a WebRTC stack sets a link up, as the model runs it: when its
/// session descriptions are ready, the roles it takes in DTLS and SCTP,
/// when its DTLS client sends its ClientHello and resends it, and the
/// bytes of what it sends. Trips are the paths': each flight waits for the
/// one it answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkSetup {
    /// The stack's name in the model's descriptions.
    pub name: &'static str,
    /// From the node's connect to its page's gathering end for an offer
    /// (the testbed harness's `rtcTiming` `gathered` of the offer less the
    /// node's `connecting`), and from an offer's coming to the answer
    /// leaving: the node's path to the page, making the description and
    /// gathering its candidates, as non-trickle ICE waits for them, and the
    /// answer's way back. The testbed's harness asks its browser nodes to
    /// connect, which takes them 2-3 ms more; a deployed node connects of
    /// itself, so the harness's share stays out. Each is an unloaded
    /// host's. A stack whose answer leaves at its check
    /// ([`LinkSetup::answer_after_check`]) takes `answer_ready` only where
    /// no check of its has succeeded by then.
    pub offer_ready: Duration,
    pub answer_ready: Duration,
    /// From an offer's gathering end to its leaving for the signalling
    /// relay: the page's post to its node's worker and the worker's write
    /// to the relay's socket (a capture's first payload to the relay after
    /// the harness's `gathered`). From there to its peer's check the
    /// model's paths and hops take what the testbed's offers took.
    pub offer_post: Duration,
    /// From an offer's coming to the answering side's first ICE check: the
    /// page setting the offer and its answer, whose setting starts the
    /// gathering of the host candidate the check goes from.
    pub check_ready: Duration,
    /// Answering, its gathering ends as its first ICE check succeeds, and
    /// its answer leaves this long after; else at `answer_ready`.
    pub answer_after_check: Option<Duration>,
    /// Answering, the stack takes DTLS's client role (`a=setup:active`);
    /// else the server's.
    pub answers_active: bool,
    /// DTLS 1.3 with a peer that runs it; else DTLS 1.2.
    pub dtls13: bool,
    /// Its ClientHello offers a hybrid post-quantum key share
    /// (X25519MLKEM768), and its server takes one: a server of such a
    /// stack answers a ClientHello of such a stack with the scheme's
    /// ciphertext in its flight ([`Flights::kem`]).
    pub hybrid: bool,
    /// As DTLS server, it drops a ClientHello that comes before its own
    /// ICE check succeeds, which its client then resends; else it keeps the
    /// ClientHello until then.
    pub drops_early_hello: bool,
    /// As DTLS client, it sends its ClientHello once its own ICE check has
    /// succeeded and its peer's check has reached it too; else at its own
    /// check alone.
    pub hello_after_peer_check: bool,
    /// As DTLS server its SCTP sends an INIT too, as a client's always
    /// does.
    pub inits_as_server: bool,
    /// As DTLS client, it resends its ClientHello while no server flight
    /// has come: first `resend_rtts` of its ICE check's round trip after
    /// sending it, within `resend_min` and `resend_max`, then twice the
    /// last wait each time, to a minute at most. With the host's draws the
    /// round trip is the one the net timed ([`Net::connect_rtt`]), to the
    /// millisecond, as the stack reads its check's response as it comes;
    /// without, the check's connect's.
    pub resend_min: Duration,
    pub resend_rtts: u32,
    pub resend_max: Duration,
    /// The period of the stack's metronome, whose ticks its low-precision
    /// timers fire on: a resend's timer fires at the first tick at or after
    /// its deadline, and with the host's draws ([`SetupDraws`]) so does its
    /// offer's gathering's last phase. Ticks fall on the host's grid
    /// ([`SetupDraws::tick_origin`]), without draws on one counted from the
    /// model's start.
    pub tick: Option<Duration>,
    /// With the host's draws, an offer's gathering ends at the first tick
    /// at or after its own setting plus this, [`GATHER_TAIL`] past that
    /// tick, in place of `offer_ready`. An offer the node asks for while
    /// none of its page's is still to be set opens a batch, whose first
    /// setting comes `set_ready` after the ask at the median, `set_lead`
    /// its spread about that; each offer asked before that setting comes
    /// joins the batch, drawing nothing, and moves the setting on by
    /// `make_per_offer`, as the page makes every offer of the batch before
    /// it sets one. The batch's offers are then set in turn,
    /// `set_per_offer` apart. A run's measured first setting (the
    /// harness's `rtcTiming` `set` of a node's first offer less its first
    /// `connecting`, strims-sim's `PageTiming`) holds its page's making of
    /// every offer its node asked for at its join, so a page that takes it
    /// takes no `make_per_offer`.
    pub gather_after_set: Option<Duration>,
    pub set_ready: Duration,
    pub set_lead: Spread,
    pub make_per_offer: Duration,
    pub set_per_offer: Duration,
    /// With the host's draws, an offer's post to the signalling relay comes
    /// no sooner than this after the page's offer post before, as its page
    /// sends them in turn.
    pub post_per_offer: Duration,
    /// With the host's draws, `check_ready` drawn, from the asking of its
    /// batch's last offer to the batch's first check (`check_per_offer`).
    pub check_spread: Spread,
    /// With the host's draws and a `check_spread`, the answering page's
    /// offers check in batches: one it is asked to set before the latest
    /// batch's first check goes joins that batch, whose first check then
    /// goes once the joining offer's is ready, the others in the order
    /// their offers were asked, each this after the one before.
    pub check_per_offer: Duration,
    pub flights: Flights,
    pub turns: Turns,
}

/// A duration a host draws, as its inverse distribution: knots of
/// (quantile in permille, µs), the first at 0 and the last at 1000, linear
/// between them; none ([`Spread::NONE`]) draws nothing. A knot's µs may be
/// negative, for a spread about a part the model holds fixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spread(pub &'static [(u16, i32)]);

impl Spread {
    pub const NONE: Spread = Spread(&[]);

    /// Its value at quantile `u` in `[0, 1)`, in µs.
    pub fn at(self, u: f64) -> f64 {
        let q = u.clamp(0.0, 1.0) * 1000.0;
        let mut knots = self.0.iter().map(|&(p, us)| (f64::from(p), f64::from(us)));
        let Some(mut lo) = knots.next() else {
            return 0.0;
        };
        for hi in knots {
            if q <= hi.0 {
                let w = hi.0 - lo.0;
                return if w > 0.0 {
                    lo.1 + (hi.1 - lo.1) * (q - lo.0) / w
                } else {
                    hi.1
                };
            }
            lo = hi;
        }
        lo.1
    }
}

/// A host's draws for its links' setups ([`Net::setup_draws`]): where its
/// stack's metronome ticks, and the stream each of its links draws its
/// parts from, in a fixed order as the node's commands come.
#[derive(Clone)]
pub struct SetupDraws {
    /// An instant on the host's tick grid: its ticks fall a whole number of
    /// its stack's [`LinkSetup::tick`] from it. Chromium's metronome ticks
    /// on the host's monotonic clock's multiples of its period
    /// (`TimerBasedTickProvider::TimeSnappedToNextTick`, snapped to
    /// `base::TimeTicks()`), so every Chromium on one host shares a phase,
    /// and hosts apart have their own.
    pub tick_origin: Instant,
    /// The host's stream, uniform in `[0, 1)`.
    pub uniform: Arc<dyn Fn() -> f64 + Send + Sync>,
}

impl SetupDraws {
    /// `spread` drawn, nothing for none.
    fn draw(&self, spread: Spread) -> Option<f64> {
        (!spread.0.is_empty()).then(|| spread.at((self.uniform)()))
    }
}

/// `base` moved by `us` µs, no sooner than `floor`.
fn shifted(base: Instant, us: f64, floor: Instant) -> Instant {
    let d = Duration::from_secs_f64(us.abs() / 1e6);
    let at = if us < 0.0 {
        base.checked_sub(d).unwrap_or(floor)
    } else {
        base + d
    };
    at.max(floor)
}

/// An answering page's batch ([`LinkSetup::check_per_offer`]): when its
/// first check goes, and how many offers it holds.
type Batch = watch::Sender<(Instant, u32)>;

/// An offer the page is asked to set at `now`, its check ready `ready`
/// later, joins `batch` if that batch's first check is still to go, which
/// then goes `ready` from now, and else opens the next: its batch and its
/// place in it.
fn join_batch(
    batch: &mut Option<Batch>,
    now: Instant,
    ready: Duration,
) -> (watch::Receiver<(Instant, u32)>, u32) {
    if let Some(b) = batch.as_ref() {
        let (first, offers) = *b.borrow();
        if now < first {
            b.send_replace((now + ready, offers + 1));
            return (b.subscribe(), offers);
        }
    }
    let (b, rx) = watch::channel((now + ready, 1));
    *batch = Some(b);
    (rx, 0)
}

/// With the host's draws, the offers a page is asked for while the first
/// of them is still to be set ([`LinkSetup::make_per_offer`]): when that
/// first setting comes, which each joining offer moves on, and how many
/// it holds.
type OfferBatch = (watch::Sender<Instant>, u32);

/// An offer the page is asked for at `now` joins `batch` if its first
/// setting is still to come, moving it on by `make`, and else opens the
/// next, set `lead()` from now: its batch and its place in it.
fn join_offers(
    batch: &mut Option<OfferBatch>,
    now: Instant,
    make: Duration,
    lead: impl FnOnce() -> Duration,
) -> (watch::Receiver<Instant>, u32) {
    if let Some((b, offers)) = batch.as_mut()
        && *b.borrow() > now
    {
        b.send_modify(|first| *first += make);
        *offers += 1;
        return (b.subscribe(), *offers - 1);
    }
    let (b, rx) = watch::channel(now + lead());
    *batch = Some((b, 1));
    (rx, 0)
}

/// Waits out its batch's first setting, which moves on as offers join it
/// until it comes: that setting.
async fn offers_set(mut first: watch::Receiver<Instant>) -> Instant {
    loop {
        let at = *first.borrow_and_update();
        if Instant::now() >= at {
            return at;
        }
        tokio::select! {
            () = tokio::time::sleep_until(at) => {}
            changed = first.changed() => {
                if changed.is_err() {
                    tokio::time::sleep_until(at).await;
                }
            }
        }
    }
}

/// When stack `me`'s page posts the offer at `place` of its batch, the
/// batch's first setting at `first` and its metronome's grid at `origin`:
/// each of its offers set `set_per_offer` after the one before, its
/// gathering ending on its tick ([`LinkSetup::gather_after_set`]), and
/// posted `offer_post` after that, no sooner than `post_per_offer` after
/// the batch's offer before.
fn offer_post_at(me: &LinkSetup, first: Instant, place: u32, origin: Instant) -> Instant {
    let after = me.gather_after_set.unwrap_or_default();
    (0..=place)
        .map(|j| {
            let set = first + me.set_per_offer * j;
            me.fires_at(set + after, origin)
                + GATHER_TAIL
                + me.offer_post
                + me.post_per_offer * (place - j)
        })
        .max()
        .unwrap_or(first)
}

/// When the check of the offer at `place` of a closed batch goes, its
/// first going at `first`: each later one `step` after the one before.
fn batch_check_at(first: Instant, place: u32, step: Duration) -> Instant {
    first + step * place
}

/// Waits out the check of the offer at `place` in its batch, whose first
/// check moves, sooner or later, as each offer joins it.
async fn batch_check(mut batch: watch::Receiver<(Instant, u32)>, place: u32, step: Duration) {
    loop {
        let (first, _) = *batch.borrow_and_update();
        if Instant::now() >= first {
            tokio::time::sleep_until(batch_check_at(first, place, step)).await;
            return;
        }
        tokio::select! {
            () = tokio::time::sleep_until(first) => {}
            // The page opens the next batch only once this one's first
            // check has gone, so a closed channel's wait is over too.
            changed = batch.changed() => {
                if changed.is_err() {
                    tokio::time::sleep_until(first).await;
                }
            }
        }
    }
}

/// How long past its tick a Chromium offer's gathering ends: 0.25 ms in 76
/// of the testbed's 80 (the halve-7 cells' PCAP=1 runs).
pub const GATHER_TAIL: Duration = Duration::from_micros(250);

/// With the hosts' draws, an offer's way to its peer's page about its
/// median, which the stacks' [`LinkSetup::offer_ready`] (or tick) and
/// `offer_post` hold with the model's paths and hops, and an answer's
/// through the signalling relay past those paths about its median, which
/// `answer_after_check` holds: the page's post to its node's worker, the
/// worker's pass, the relay's and the peer's worker's and page's. Drawn
/// at the sender, the offer's as its page's gathering ends and the
/// answer's on its leaving, neither sooner than the last wait it ended.
/// The testbed's halve-7 runs' 112 seed links read each leg's excess over
/// its paths' delays at 3, 5 and 9 ms (p10, p50, p90), the answer's at 0,
/// 3 and 7.
pub const OFFER_TRANSIT: Spread = Spread(&[
    (0, -2_500),
    (100, -2_000),
    (500, 0),
    (900, 4_000),
    (1000, 6_000),
]);
pub const ANSWER_TRANSIT: Spread = Spread(&[
    (0, -3_000),
    (100, -3_000),
    (500, 0),
    (900, 4_000),
    (1000, 6_000),
]);

/// The UDP payload bytes of what a stack sends in a link's setup: DTLS
/// flights with their records, and SCTP chunks in their DTLS record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flights {
    pub hello: usize,
    /// A DTLS 1.3 server's first flight (its ServerHello to its Finished),
    /// and a hybrid key share's ciphertext in it.
    pub server13: usize,
    pub kem: usize,
    /// A DTLS 1.3 client's last flight (its Certificate to its Finished),
    /// and the ACK its server sends for it, whose record numbers are the
    /// flight's records: the client's stack's, whichever stack the server
    /// runs.
    pub finish13: usize,
    pub ack13: usize,
    /// DTLS 1.2: the server's first flight (its ServerHello to its
    /// ServerHelloDone), the client's second (its Certificate to its
    /// Finished) and the server's last (ChangeCipherSpec and Finished).
    pub server12: usize,
    pub finish12: usize,
    pub server_finish12: usize,
    pub init: usize,
    pub init_ack: usize,
    pub cookie_echo: usize,
    pub cookie_ack: usize,
}

/// How long a stack takes to answer what reaches it during a link's
/// setup, from the packet's arrival to its reply's send: the browsers
/// handle each setup packet on one thread as it is read, ICE's check
/// response starting the DTLS client and each DTLS and SCTP packet's
/// handling sending its reply, so a turn is that thread's work. The net
/// hands a host what reached it at its next step, a millisecond's at
/// most later, and no sleep can send within the step, so the model has
/// the net send each reply at the flight's arrival plus its turn
/// ([`Net::delivered_ago`], [`Net::send_ago`]).
///
/// The sources (Chromium branch-heads/8078, its webrtc 19576e37870b and
/// boringssl 2ea3200c3a3e; Firefox FIREFOX_158_0_1_RELEASE). Chromium
/// reads a setup packet on the renderer's WebRTC network thread, a Mojo
/// hop from the network service's socket each way
/// (services/network/p2p/socket_udp.cc, blink's
/// p2p/socket_client_impl.cc), and answers it there in the same task: its
/// ClientHello one same-thread PostTask after the check's response
/// (wrapping_active_ice_controller.cc:140-148 to dtls_transport.cc's
/// MaybeStartDtls), BoringSSL's ACK of the client's Finished at once
/// (tls13_server.cc:1464-1466, d1_both.cc:1085-1092), the INIT in the
/// turn that reads the Finished (dtls_transport.cc:989-990 to
/// dcsctp_transport.cc:715, 787), dcSCTP's INIT ACK, COOKIE ECHO and
/// COOKIE ACK as it handles what they answer (dcsctp_socket.cc:1331,
/// 1390, 1459) and the DCEP OPEN on the same thread
/// (dcsctp_transport.cc:576, data_channel_controller.cc:218,
/// sctp_data_channel.cc:747). Firefox runs ICE and DTLS on the socket
/// process's socket thread, starting the handshake on a 0 ms timer
/// (transportlayerdtls.cpp:958-970, 1138), and dcSCTP in the content
/// process, so each SCTP packet crosses two IPC hops and their task
/// queues (MediaTransportParent.cpp:45, MediaTransportHandlerIPC.cpp:
/// 313-320, 433, DataChannel.cpp:322, 376, DataChannelDcSctp.cpp:166,
/// 198), its INIT among them: its SCTP turns are longer than Chromium's.
///
/// From the h7c8 halve-7-20 runs' captures (8 runs, every node's
/// packets), each reply's send less its peer's packet's arrival, medians:
/// a Chromium DTLS client's ClientHello 0.48 ms after its check's response
/// (0.39 to Firefox servers); a Chromium server's flight 0.27 ms after its
/// own check's response, a Chromium client's Finished 0.30-0.38 after the
/// server's flight, the server's ACK, INIT and INIT ACK 0.14-0.21 after
/// that Finished; dcSCTP's INIT ACK, COOKIE ECHO and COOKIE ACK 0.02-0.05
/// after the chunk they answer; and the link's first data 0.40 ms after
/// the association's last chunk. Firefox: its server flight 0.28-0.50
/// after the ClientHello it answers, its ACK 0.31-0.61 after the client's
/// Finished, dcSCTP's chunks 0.07-0.10, the first data 0.48. The
/// profiles' values are fits to these, one a kind of turn, with two
/// gaps: Firefox's `hello` is assumed, as the runs captured no Firefox
/// ClientHello, and no turn is fitted to a DTLS 1.3 server's ACK and INIT
/// after the client's Finished (0.14-0.21 ms on Chromium), which carry no
/// DTLS item and take the SCTP turn (Chromium's `dtls`, 300 us, is its
/// flights').
///
/// Approximations: a reply whose turn would end past the step's start
/// goes at the step's start, past its arrival still, and none goes before
/// what its host's uplink was offered last; only a data
/// channel's writes take the stamp (its association's sender sends them),
/// so the net's other writes go at the step's start; a later stamp in a
/// step moves all that step's writes on the link, to the step's start
/// where its turn ends past it (the COOKIE ACK's turn and the first
/// data's are one, and a Chromium viewer's COOKIE ECHO goes with its
/// first data at the step's start, 0.23 ms after the INIT ACK on h7c8,
/// where the testbed's went 0.04-0.05 ms after it); and the uplink
/// queues a backdated reply behind
/// what it was handed since, as it does any packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Turns {
    /// A DTLS client's ClientHello after its check's response.
    pub hello: Duration,
    /// A DTLS flight after the one it answers, or a server's after its
    /// check's response.
    pub dtls: Duration,
    /// An SCTP chunk of the association's setup after the one it answers,
    /// and a write that carries only the link's token.
    pub sctp: Duration,
    /// The link's first write after the association's last setup chunk.
    pub open: Duration,
}

impl Turns {
    /// None: a reply goes as the net's step sends it (str0m, which runs
    /// on the node's own runtime).
    pub const NONE: Turns = Turns {
        hello: Duration::ZERO,
        dtls: Duration::ZERO,
        sctp: Duration::ZERO,
        open: Duration::ZERO,
    };

    /// The turn of a reply carrying `items`, or none for no stamp: a reply
    /// of no DTLS item takes the SCTP turn, a token-only write among them.
    fn of(&self, items: u16) -> Option<Duration> {
        let t = if items & item::HELLO != 0 {
            self.hello
        } else if items & (item::FLIGHT | item::FINISH | item::SERVER_FINISH) != 0 {
            self.dtls
        } else {
            self.sctp
        };
        (*self != Turns::NONE).then_some(t)
    }
}

/// The longest wait between a DTLS client's resends of its ClientHello.
const MAX_RESEND: Duration = Duration::from_secs(60);

/// What dcSCTP's association sends over DTLS 1.3, as Chromium and Firefox
/// run it (the testbed's captures): INIT, INIT ACK, COOKIE ECHO and COOKIE
/// ACK.
const DCSCTP_INIT: usize = 66;
const DCSCTP_INIT_ACK: usize = 118;
const DCSCTP_COOKIE_ECHO: usize = 86;
const DCSCTP_COOKIE_ACK: usize = 38;
/// DTLS 1.2's flights, as str0m's dimpl sends them (strims-webrtc's sim):
/// a browser's in DTLS 1.2 are its peer's, str0m's, alone.
const DTLS12_SERVER: usize = 598;
const DTLS12_FINISH: usize = 553;
const DTLS12_SERVER_FINISH: usize = 75;

impl LinkSetup {
    /// Chromium, from the testbed (hack/testbed/run.sh, PCAP=1: the
    /// captures, the node's `connecting` and the page's `rtcTiming`) and
    /// its libwebrtc. Its offer leaves 134 ms after the node connects, at
    /// the median, with no ICE servers: its gathering runs every phase of
    /// the port allocator, 123-125 ms from the offer's setting. Answering,
    /// it has set the offer and its answer 2.1-2.5 ms after the page is
    /// asked to, and checks 0.5-0.6 ms after that (the C6 bench's runs'
    /// medians). Its gathering then ends as that check succeeds, libwebrtc
    /// stopping its port allocator once a connection is strongly connected
    /// (`stop_gather_on_strongly_connected`): in the runs' 72 answers it
    /// ended 0.6 ms after the check's response came at the median
    /// (0.2-3.4 ms), and the answer left 0.6 ms after that. So an answer's
    /// gathering takes its check's round trip, 22 ms on the testbed's
    /// 20-ms viewer paths and 32 on its seed's 30-ms ones, longer behind a
    /// full uplink queue; one none of whose checks succeeds gathers every
    /// phase, as an offer does, and leaves 127 ms after the offer came. Its
    /// DTLS (BoringSSL's 1.3) client sends its ClientHello as its
    /// own check succeeds, 0.1-0.9 ms after it on the testbed, and its
    /// server starts then too, replaying the ClientHello it kept. Its
    /// client resends at twice its check's round trip, at least 50 ms and
    /// at most 3 s (libwebrtc's `ComputeRetransmissionTimeout`, p2p/dtls/
    /// dtls_transport.cc, of `GetRttEstimate`: the selected connection's
    /// `Rtt().ms()`, whose first sample is its check's response's
    /// `request->Elapsed()` whole, p2p/base/connection.cc), doubling,
    /// on a timer of low precision, which Chromium fires at the next tick
    /// of its 64-Hz metronome (`webrtc_overrides`' `WebRtcTaskQueue` and
    /// `TimerBasedTickProvider`): on the testbed's 20-ms paths its resends
    /// to a Firefox server came 47-65 ms after its ClientHello, 57 at the
    /// median of 21. Its ClientHello carries X25519MLKEM768's share. Both
    /// its ends send an INIT (dcSCTP).
    ///
    /// With the host's draws, from the halve-7 cells' PCAP=1 runs (7
    /// viewers each joining at once, linking to the seed and the viewers
    /// before it, a Chromium seed answering), each offer read from its
    /// node's `rtcTiming` and its offered capture, every one a node made:
    /// in the halve-7-20 cell's 8 runs on c8a (40 Chromium viewers making
    /// 1 to 6 offers each, 144 in all), an offer's gathering ended at the
    /// first tick of the metronome at or after its own setting plus 113.0
    /// ms in 142 (`gather_after_set`), the run's every Chromium on one grid
    /// (their containers share the host's clock), [`GATHER_TAIL`] past its
    /// tick (0.2 ms past the run's earliest at the median, 0.1-0.5 at
    /// p10-p90, an earliest itself past its tick). A node's offers
    /// were set together, once its page had made those its node asked for
    /// by then, the node asking for them about a millisecond apart (its
    /// worker's cost of each): its first setting came 7.1 ms after its
    /// connect for a lone offer and 0.96 ms later for each further one
    /// (least squares over the 40 nodes; the h7spec halve-7 cells' 32 on
    /// c7a's 9R14, 8.9 and 1.79; `make_per_offer`, their mean, which a
    /// page of no measured setting takes), each later setting 0.1 ms after
    /// the one before at the median of 104 (0.0-0.5; the same on c7a;
    /// `set_per_offer`), and in 38 of the 40 nodes every offer gathered on
    /// one tick. A lone offer was set 9.8 ms after the ask at the median,
    /// 7.2-11.8 (p10-p90; `set_lead`), and a twin takes its run's first
    /// settings, a node each (`set_ready`, strims-sim's
    /// `testbed::offer_timing`): on c8a 6.2 ms for the viewer of one offer
    /// and 11.1 for that of six, 10.2 for every node's, 13.9 on c7a. The
    /// gathering's 113.0 ms is c7a's too (284 of 288). Its first offer left
    /// for the relay 0.86 ms after its gathering's end (`offer_post`: the
    /// first relay payload in its offered capture less its `gathered`
    /// stamp, to about 0.1 ms on Chromium) at the median of the halve-7-20
    /// cell's 8 runs' medians on c8a (0.83-0.93; their 40 viewers'
    /// 0.69-2.03), each later one of its tick 0.31 ms after the one before
    /// at the median of 102 (0.27-0.54; `post_per_offer`; c7a's 0.49), and
    /// the seed's check came 18.3 ms after the first at the median of those
    /// runs, where the model's paths and hops, the relay's and the seed's
    /// batch, take 18.0.
    /// The seed's checks
    /// went in batches: an offer it was asked to set before a batch's first
    /// check went joined it, and the batch's
    /// checks went in the order their offers were asked, the first 2.5 ms
    /// after its last offer's asking at the median of the runs' 42 batches
    /// of 1 to 5 offers (1.0-3.8 at p10-p90; `check_spread`, its median the
    /// C6 bench's `check_ready`), each later one 0.37 ms after the one
    /// before at the median of 68 (0.20-0.90; `check_per_offer`), in all
    /// 33 batches of two or more. That joins each of the seed's
    /// connections to its viewer by the offer the relay forwarded it,
    /// which the seed's page reads in turn; no offer came while a batch's
    /// checks were going, and no check went before its own answer's
    /// setting, 0.45 ms after it at the median (0.28-1.08). The strims
    /// page gives each connection its own chain of calls, so the batching
    /// is below it, likely in the signalling and network threads a
    /// renderer's connections share (unverified), and part of its stall
    /// may be the testbed's host, its 8 browsers joining at once. Whether
    /// it carries what a run's seed links share is open: of their opens'
    /// variance, an ANOVA's between-run component is 0.0 ms² in the
    /// twin's 32 runs a cell, 6.7 and 0.1 in the testbed's 8 (halve-7-10,
    /// -20), too few runs to tell.
    pub const CHROMIUM: LinkSetup = LinkSetup {
        name: "chromium",
        offer_ready: Duration::from_millis(134),
        answer_ready: Duration::from_millis(127),
        offer_post: Duration::from_micros(860),
        check_ready: Duration::from_micros(2_800),
        answer_after_check: Some(Duration::from_micros(1_200)),
        answers_active: true,
        dtls13: true,
        hybrid: true,
        drops_early_hello: false,
        hello_after_peer_check: false,
        inits_as_server: true,
        resend_min: Duration::from_millis(50),
        resend_rtts: 2,
        resend_max: Duration::from_secs(3),
        tick: Some(Duration::from_micros(15_625)),
        gather_after_set: Some(Duration::from_micros(113_000)),
        set_ready: Duration::from_micros(9_800),
        set_lead: Spread(&[
            (0, -3_990),
            (100, -2_600),
            (500, 0),
            (900, 2_000),
            (1000, 3_010),
        ]),
        make_per_offer: Duration::from_micros(1_850),
        set_per_offer: Duration::from_micros(100),
        post_per_offer: Duration::from_micros(310),
        check_spread: Spread(&[
            (0, 600),
            (100, 1_000),
            (250, 1_500),
            (500, 2_800),
            (750, 3_350),
            (900, 3_750),
            (1000, 5_000),
        ]),
        check_per_offer: Duration::from_micros(370),
        turns: Turns {
            hello: Duration::from_micros(450),
            dtls: Duration::from_micros(300),
            sctp: Duration::from_micros(40),
            open: Duration::from_micros(400),
        },
        flights: Flights {
            hello: 1463,
            server13: 629,
            kem: 1086,
            finish13: 455,
            ack13: 40,
            server12: DTLS12_SERVER,
            finish12: DTLS12_FINISH,
            server_finish12: DTLS12_SERVER_FINISH,
            init: DCSCTP_INIT,
            init_ack: DCSCTP_INIT_ACK,
            cookie_echo: DCSCTP_COOKIE_ECHO,
            cookie_ack: DCSCTP_COOKIE_ACK,
        },
    };

    /// Firefox, from the testbed as Chromium is. Its page's gathering for
    /// an offer ends 12.8 ms after the node connects and the offer leaves
    /// for the relay 1.7 ms after that, at the medians of the halve-7-20
    /// cell's 8 runs on c8a's EPYC 9R45 (the harness's `gathered` less its
    /// `connecting`, and the capture's post less `gathered`), where c7a's
    /// 9R14 took 17.0 and 2.6 and 16 browsers on one host
    /// later still: a twin takes its run's (strims-sim's
    /// `testbed::offer_timing`). Its page posts a node's offers in turn,
    /// as Chromium's does: each later one 0.30 ms after the one before at
    /// the median of those runs' 64 (0.15-0.72 at p10-p90;
    /// `post_per_offer`). The seed's check came 18.7 ms after the
    /// post at the median of the c8a runs, where the model's paths and
    /// hops take 18.2. Its answer leaves 6 ms after an offer comes, at the
    /// median, its gathering a millisecond of it. Answering, it has
    /// set the offer and its answer 3.0 ms after the page is asked to, and
    /// checks 0.7-1.0 ms after that. Its DTLS (NSS's 1.3) client sends its
    /// ClientHello once the offering side's check has reached it as well
    /// as its own succeeded: on the testbed, in each of its 12 answers,
    /// 0.03-0.13 ms after its response to the offering side's check,
    /// 2.3-4.7 ms after its own check's response came. So the ClientHello
    /// comes to the offering side as that side's check succeeds. Its server
    /// sees nothing before its own check succeeds, so a ClientHello that
    /// comes earlier is lost to it, and its client resends after 50 ms,
    /// doubling. Both its ends send an INIT.
    pub const FIREFOX: LinkSetup = LinkSetup {
        name: "firefox",
        offer_ready: Duration::from_micros(12_800),
        answer_ready: Duration::from_millis(6),
        offer_post: Duration::from_micros(1_700),
        check_ready: Duration::from_micros(3_800),
        answer_after_check: None,
        answers_active: true,
        dtls13: true,
        hybrid: false,
        drops_early_hello: true,
        hello_after_peer_check: true,
        inits_as_server: true,
        resend_min: Duration::from_millis(50),
        resend_rtts: 0,
        resend_max: Duration::from_millis(50),
        tick: None,
        gather_after_set: None,
        set_ready: Duration::ZERO,
        set_lead: Spread::NONE,
        make_per_offer: Duration::ZERO,
        set_per_offer: Duration::ZERO,
        post_per_offer: Duration::from_micros(300),
        check_spread: Spread::NONE,
        check_per_offer: Duration::ZERO,
        turns: Turns {
            hello: Duration::from_micros(400),
            dtls: Duration::from_micros(400),
            sctp: Duration::from_micros(80),
            open: Duration::from_micros(480),
        },
        flights: Flights {
            hello: 264,
            server13: 783,
            kem: 0,
            finish13: 548,
            ack13: 72,
            server12: DTLS12_SERVER,
            finish12: DTLS12_FINISH,
            server_finish12: DTLS12_SERVER_FINISH,
            init: DCSCTP_INIT,
            init_ack: DCSCTP_INIT_ACK,
            cookie_echo: DCSCTP_COOKIE_ECHO,
            cookie_ack: DCSCTP_COOKIE_ACK,
        },
    };

    /// str0m 0.25 as strims-webrtc runs it (its sim's handshake): its
    /// descriptions are ready at once with no STUN servers; it answers
    /// passive (`a=setup:passive`), runs dimpl's DTLS 1.2, whose client
    /// resends after 1 s, doubling, and whose server keeps an early
    /// ClientHello; and its SCTP (sctp-proto) sends an INIT as DTLS client
    /// alone.
    pub const STR0M: LinkSetup = LinkSetup {
        name: "str0m",
        offer_ready: Duration::ZERO,
        answer_ready: Duration::ZERO,
        offer_post: Duration::ZERO,
        check_ready: Duration::ZERO,
        answer_after_check: None,
        answers_active: false,
        dtls13: false,
        hybrid: false,
        drops_early_hello: false,
        hello_after_peer_check: false,
        inits_as_server: false,
        resend_min: Duration::from_secs(1),
        resend_rtts: 0,
        resend_max: Duration::from_secs(1),
        tick: None,
        gather_after_set: None,
        set_ready: Duration::ZERO,
        set_lead: Spread::NONE,
        make_per_offer: Duration::ZERO,
        set_per_offer: Duration::ZERO,
        post_per_offer: Duration::ZERO,
        check_spread: Spread::NONE,
        check_per_offer: Duration::ZERO,
        turns: Turns::NONE,
        flights: Flights {
            hello: 118,
            server13: 0,
            kem: 0,
            finish13: 0,
            ack13: 0,
            server12: DTLS12_SERVER,
            finish12: DTLS12_FINISH,
            server_finish12: DTLS12_SERVER_FINISH,
            init: 77,
            init_ack: 113,
            cookie_echo: 85,
            cookie_ack: 53,
        },
    };

    const ALL: [LinkSetup; 3] = [LinkSetup::CHROMIUM, LinkSetup::FIREFOX, LinkSetup::STR0M];

    /// The stack named `name` in a description.
    fn named(name: &str) -> Option<LinkSetup> {
        LinkSetup::ALL.into_iter().find(|s| s.name == name)
    }

    /// The DTLS client's first resend of its ClientHello after a check of
    /// round trip `rtt`.
    fn resend_after(&self, rtt: Duration) -> Duration {
        (rtt * self.resend_rtts).clamp(self.resend_min, self.resend_max)
    }

    /// When a low-precision timer of `deadline` fires, the stack's ticks a
    /// whole number of [`LinkSetup::tick`] from `origin`.
    fn fires_at(&self, deadline: Instant, origin: Instant) -> Instant {
        let Some(tick) = self.tick.filter(|t| !t.is_zero()) else {
            return deadline;
        };
        let since = deadline.saturating_duration_since(origin).as_nanos();
        let ticks = since.div_ceil(tick.as_nanos());
        let at = u64::try_from(ticks.saturating_mul(tick.as_nanos())).unwrap_or(u64::MAX);
        origin + Duration::from_nanos(at)
    }
}

/// What a flight carries.
mod item {
    /// The offer's token.
    pub(super) const TOKEN: u16 = 1;
    pub(super) const HELLO: u16 = 1 << 1;
    /// The DTLS server's first flight.
    pub(super) const FLIGHT: u16 = 1 << 2;
    /// The DTLS client's second flight.
    pub(super) const FINISH: u16 = 1 << 3;
    /// A DTLS 1.2 server's last flight.
    pub(super) const SERVER_FINISH: u16 = 1 << 4;
    pub(super) const INIT: u16 = 1 << 5;
    pub(super) const INIT_ACK: u16 = 1 << 6;
    pub(super) const COOKIE_ECHO: u16 = 1 << 7;
    pub(super) const COOKIE_ACK: u16 = 1 << 8;
    /// The offering side's check, on its dial: its token is the answer's.
    pub(super) const CHECK: u16 = 1 << 9;
}

/// One flight of a link's setup: what it carries and its bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Flight {
    pub(crate) items: u16,
    pub(crate) bytes: usize,
}

impl Flight {
    fn add(&mut self, item: u16, bytes: usize) {
        self.items |= item;
        self.bytes += bytes;
    }

    fn is_empty(&self) -> bool {
        self.items == 0
    }
}

/// One side's DTLS handshake and SCTP association, past its ICE check's
/// start: each call takes what happened and returns the flight the side
/// sends for it, empty for none.
///
/// The DTLS client sends its ClientHello once its check succeeds
/// ([`Handshake::writable`]), and for a stack that waits for it, once its
/// peer's check has reached it too ([`Handshake::peer_checked`]); the
/// server answers it once its own check has succeeded too, keeping or
/// dropping one that comes earlier as its stack does. In DTLS 1.3 the
/// client's second flight ends its handshake and
/// carries its INIT; in 1.2 the server's Finished answers it, and the
/// client's INIT follows. A server whose stack sends an INIT sends it once
/// its handshake ends. A side answers an INIT with an INIT ACK and an INIT
/// ACK with a COOKIE ECHO; its association is up once a COOKIE ECHO or a
/// COOKIE ACK comes, and it sends nothing more but the COOKIE ACK a COOKIE
/// ECHO asks for, so the stream past a side's last flight is its peer's
/// link.
#[derive(Debug)]
pub(crate) struct Handshake {
    me: LinkSetup,
    peer: LinkSetup,
    /// The DTLS client.
    client: bool,
    /// Its ICE check succeeded, and its peer's reached it.
    writable: bool,
    peer_checked: bool,
    /// A client's ClientHello went.
    hello: bool,
    /// A server's ClientHello, kept until it is writable.
    held: bool,
    /// The server's first flight went (server) or came (client).
    flight: bool,
    /// Its DTLS handshake ended.
    dtls: bool,
    /// Its association is up.
    pub(crate) up: bool,
}

impl Handshake {
    /// `peer_checked`: the peer's check reached this side already, as the
    /// offering side's stream is the answering side's check.
    pub(crate) fn new(
        me: LinkSetup,
        peer: LinkSetup,
        client: bool,
        peer_checked: bool,
    ) -> Handshake {
        Handshake {
            me,
            peer,
            client,
            writable: false,
            peer_checked,
            hello: false,
            held: false,
            flight: false,
            dtls: false,
            up: false,
        }
    }

    fn dtls13(&self) -> bool {
        self.me.dtls13 && self.peer.dtls13
    }

    fn hello(&self) -> Flight {
        Flight {
            items: item::HELLO,
            bytes: self.me.flights.hello,
        }
    }

    /// A client's ClientHello, once its stack sends it.
    fn hello_due(&mut self) -> Flight {
        let due = self.writable && (self.peer_checked || !self.me.hello_after_peer_check);
        if !self.client || self.hello || !due {
            return Flight::default();
        }
        self.hello = true;
        self.hello()
    }

    fn server_flight(&mut self) -> Flight {
        self.flight = true;
        let f = &self.me.flights;
        let bytes = if self.dtls13() {
            f.server13
                + if self.me.hybrid && self.peer.hybrid {
                    f.kem
                } else {
                    0
                }
        } else {
            f.server12
        };
        Flight {
            items: item::FLIGHT,
            bytes,
        }
    }

    /// Its ICE check succeeded.
    pub(crate) fn writable(&mut self) -> Flight {
        if self.writable {
            return Flight::default();
        }
        self.writable = true;
        if self.client {
            self.hello_due()
        } else if std::mem::take(&mut self.held) {
            self.server_flight()
        } else {
            Flight::default()
        }
    }

    /// Its peer's ICE check reached it.
    pub(crate) fn peer_checked(&mut self) -> Flight {
        self.peer_checked = true;
        self.hello_due()
    }

    /// The ClientHello again, while a client's is unanswered.
    pub(crate) fn resend(&self) -> Option<Flight> {
        (self.client && self.hello && !self.flight).then(|| self.hello())
    }

    /// The peer's flight of `items` came.
    pub(crate) fn receive(&mut self, items: u16) -> Flight {
        let mut out = Flight::default();
        if self.up {
            return out;
        }
        let f = self.me.flights;
        let has = |i: u16| items & i != 0;
        if has(item::HELLO) && !self.client && !self.flight {
            if self.writable {
                out = self.server_flight();
            } else if !self.me.drops_early_hello {
                self.held = true;
            }
        }
        if has(item::FLIGHT) && self.client && !self.flight {
            self.flight = true;
            if self.dtls13() {
                self.dtls = true;
                out.add(item::FINISH, f.finish13);
                out.add(item::INIT, f.init);
            } else {
                out.add(item::FINISH, f.finish12);
            }
        }
        if has(item::FINISH) && !self.client && self.flight && !self.dtls {
            self.dtls = true;
            if self.dtls13() {
                out.bytes += self.peer.flights.ack13;
            } else {
                out.add(item::SERVER_FINISH, f.server_finish12);
            }
            if self.me.inits_as_server {
                out.add(item::INIT, f.init);
            }
        }
        if has(item::SERVER_FINISH) && self.client && self.flight && !self.dtls {
            self.dtls = true;
            out.add(item::INIT, f.init);
        }
        if has(item::INIT) {
            out.add(item::INIT_ACK, f.init_ack);
        }
        if has(item::INIT_ACK) {
            out.add(item::COOKIE_ECHO, f.cookie_echo);
        }
        if has(item::COOKIE_ECHO) {
            // Up: the COOKIE ECHO its INIT ACK asked for goes with the
            // COOKIE ACK, as its stack sends both, but asks for nothing.
            self.up = true;
            out.items &= !item::COOKIE_ECHO;
            out.add(item::COOKIE_ACK, f.cookie_ack);
        }
        if has(item::COOKIE_ACK) {
            self.up = true;
        }
        out
    }
}

/// A flight's message header: its items and its payload's length.
const HEADER: usize = 4;
/// What the channel adds to a message of one packet: its DTLS 1.3 record,
/// SCTP's common header and a DATA chunk's header.
const CHANNEL_HEADERS: usize = 22 + 12 + 16;

/// `flight` as a message on the channel: its items, its payload's length,
/// and the payload, `token` first, padded to the flight's bytes less the
/// headers the channel adds to them.
pub(crate) fn frame(flight: Flight, token: Option<u64>) -> Vec<u8> {
    let mut items = flight.items;
    let mut payload = Vec::new();
    if let Some(t) = token {
        items |= item::TOKEN;
        payload.extend_from_slice(&t.to_be_bytes());
    }
    let len = flight
        .bytes
        .saturating_sub(CHANNEL_HEADERS + HEADER)
        .clamp(payload.len(), usize::from(u16::MAX));
    payload.resize(len, 0);
    let mut out = Vec::with_capacity(HEADER + len);
    out.extend_from_slice(&items.to_be_bytes());
    out.extend_from_slice(&(len as u16).to_be_bytes());
    out.extend_from_slice(&payload);
    out
}

/// The next message in `buf`, taken out of it: its items and its token,
/// if it carries one; `None` while it is incomplete.
fn parse(buf: &mut Vec<u8>) -> Option<(u16, Option<u64>)> {
    let header: [u8; HEADER] = buf.get(..HEADER)?.try_into().ok()?;
    let [i0, i1, l0, l1] = header;
    let (items, len) = (
        u16::from_be_bytes([i0, i1]),
        usize::from(u16::from_be_bytes([l0, l1])),
    );
    let payload = buf.get(HEADER..HEADER + len)?;
    let token = (items & item::TOKEN != 0)
        .then(|| payload.get(..8)?.try_into().ok().map(u64::from_be_bytes))
        .flatten();
    buf.drain(..HEADER + len);
    Some((items, token))
}

/// Reads `s` into `buf` until a whole message is there and takes it,
/// reading no byte past it: what follows a side's last flight is its
/// link's, which the link reads as a channel's messages come. Its reads
/// are cancel safe: what one read stays in `buf`.
async fn next_flight<S: AsyncRead + Unpin>(
    s: &mut S,
    buf: &mut Vec<u8>,
) -> io::Result<(u16, Option<u64>)> {
    let mut chunk = [0u8; 2048];
    loop {
        if let Some(m) = parse(buf) {
            return Ok(m);
        }
        let want = match buf.get(2..HEADER) {
            Some(&[l0, l1]) => HEADER + usize::from(u16::from_be_bytes([l0, l1])),
            _ => HEADER,
        };
        let room = want.saturating_sub(buf.len()).clamp(1, chunk.len());
        let n = s.read(chunk.get_mut(..room).unwrap_or_default()).await?;
        if n == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        buf.extend_from_slice(chunk.get(..n).unwrap_or_default());
    }
}

/// A check's request as the offering side's datagram carries it: the
/// answer's token in a STUN binding request's bytes, as the testbed's
/// offering sides send theirs.
const CHECK_BYTES: usize = 100;
/// Its first byte, which no STUN, DTLS or SCTP message starts with: the
/// net carries it as it carries any datagram.
const CHECK_TAG: u8 = b'm';

/// The datagram of a check of the answer of `token`.
fn check_datagram(token: u64) -> Vec<u8> {
    let mut d = Vec::with_capacity(CHECK_BYTES);
    d.push(CHECK_TAG);
    d.extend_from_slice(&token.to_be_bytes());
    d.resize(CHECK_BYTES, 0);
    d
}

/// The answer's token, if `d` is a check's datagram.
fn parse_check(d: &[u8]) -> Option<u64> {
    (*d.first()? == CHECK_TAG).then_some(())?;
    d.get(1..9)?.try_into().ok().map(u64::from_be_bytes)
}

/// The driver's listener, its socket for checks, and the proxy's commands,
/// until [`run`] takes them.
pub(crate) struct ModelRtc<N: Net> {
    listener: N::TcpListener,
    /// Where peers reach `listener`.
    pub(crate) advertised: SocketAddr,
    /// Where the peers' checks of our answers reach us, and where they
    /// reach it.
    checks: N::UdpSocket,
    checks_at: SocketAddr,
    commands: mpsc::UnboundedReceiver<RtcCommand>,
    /// Each channel's most bytes buffered.
    cap: usize,
}

/// Binds the channel listener and the socket for checks: the node's
/// endpoint and its driver.
pub(crate) async fn bind<N: Net>(
    net: &N,
    config: &NodeConfig,
) -> Result<(RtcFactory, ModelRtc<N>), NodeError> {
    let w = &config.webrtc;
    let Some(listen) = w.listen.first() else {
        return Err(NodeError::Config(
            "webrtc is enabled without a listen address".to_owned(),
        ));
    };
    let listener = net.tcp_bind(listen.clone()).await?;
    let local = net.tcp_listener_addr(&listener)?;
    let mut advertised = local;
    if advertised.ip().is_unspecified() {
        let Some(ip) = w.endpoint.host_ip.or(config.public_ip) else {
            return Err(NodeError::Config(format!(
                "webrtc listen address {advertised} is unspecified: bind a concrete ip, or set \
                 webrtc.endpoint.host_ip or public_ip"
            )));
        };
        advertised.set_ip(ip);
    }
    let checks = net
        .udp_bind(SocketAddr::new(local.ip(), 0).to_string())
        .await?;
    let checks_at = SocketAddr::new(advertised.ip(), net.udp_local_addr(&checks)?.port());
    let (tx, commands) = mpsc::unbounded_channel();
    let model = ModelRtc {
        listener,
        advertised,
        checks,
        checks_at,
        commands,
        cap: w.max_buffered_amount,
    };
    Ok((ProxyRtc::factory(w, 0, tx), model))
}

/// An offer's address, token and stack, if `desc` is one of this module's.
fn parse_offer(desc: &SessionDescription) -> Option<(String, u64, LinkSetup)> {
    let mut w = desc.sdp.split_whitespace();
    (w.next()? == PREFIX).then_some(())?;
    let addr = w.next()?.to_owned();
    let token = w.next()?.parse().ok()?;
    let stack = LinkSetup::named(w.next()?)?;
    Some((addr, token, stack))
}

/// An answer's address, token, stack and socket for checks, if `desc` is
/// one of this module's.
fn parse_answer(desc: &SessionDescription) -> Option<(String, u64, LinkSetup, SocketAddr)> {
    let mut w = desc.sdp.split_whitespace();
    (w.next()? == PREFIX && w.next()? == "answer").then_some(())?;
    let addr = w.next()?.to_owned();
    let token = w.next()?.parse().ok()?;
    let stack = LinkSetup::named(w.next()?)?;
    let checks = w.next()?.parse().ok()?;
    Some((addr, token, stack, checks))
}

async fn signal<T>(inputs: &Inputs<T>, conn: RtcConn, signal: RtcSignal) {
    let _ = inputs.send(Input::Signal(RtcInput { conn, signal })).await;
}

async fn closed<T>(inputs: &Inputs<T>, conn: RtcConn, e: &io::Error) {
    signal(
        inputs,
        conn,
        RtcSignal::Closed(format!("model channel: {e}")),
    )
    .await;
}

/// Reports `conn` up and hands `stream` to the node as its link, buffered
/// in vnic as the stack it stands for buffers ([`Net::channels_in_process`]),
/// its messages the link's from now on ([`Net::channel_up`]).
async fn attach<N: Net>(
    net: &N,
    stream: N::TcpStream,
    conn: RtcConn,
    inputs: &Inputs<Transport<N>>,
) {
    net.channel_up(&stream);
    signal(inputs, conn, RtcSignal::Connected).await;
    let _ = inputs
        .send(Input::Connected {
            dial: None,
            transport: Transport::Channel(stream),
            config: LinkConfig::webrtc(),
            leftover: Vec::new(),
            notsent_lowat: None,
            rtc: Some(conn),
            in_process: net.channels_in_process(),
        })
        .await;
}

/// The round trip a stack reads from its check, whose connect `s` returned
/// `elapsed` after its dial: with the host's draws (`timed`), the one the
/// net timed ([`Net::connect_rtt`]), to the nearest millisecond, as
/// libwebrtc reads its connection's (`Rtt().ms()`); else `elapsed`. The
/// net's is taken either way, as it gives it once.
fn check_rtt<N: Net>(net: &N, s: &N::TcpStream, elapsed: Duration, timed: bool) -> Duration {
    net.connect_rtt(s)
        .filter(|_| timed)
        .map_or(elapsed, nearest_ms)
}

/// `d` to the nearest millisecond, as libwebrtc's `TimeDelta::ms()`.
fn nearest_ms(d: Duration) -> Duration {
    let ms = (d.as_micros() + 500) / 1000;
    Duration::from_millis(u64::try_from(ms).unwrap_or(u64::MAX))
}

/// The offering side's check of the answer of `token`: its request reaches
/// the answering side's socket for checks, `checks`, as a datagram, and its
/// round trip is a dial of `addr`, sent to `done` as it connects
/// ([`check_rtt`], `timed` with the host's draws). The dial names the
/// answer too, for a datagram the path lost.
async fn check<N: Net>(
    net: N,
    socket: Arc<N::UdpSocket>,
    checks: SocketAddr,
    addr: String,
    token: u64,
    timed: bool,
    done: oneshot::Sender<Checked>,
) {
    let _ = net
        .udp_send_to(&socket, &check_datagram(token), checks)
        .await;
    let start = Instant::now();
    match net.tcp_connect(addr).await {
        Ok(mut s) => {
            let rtt = check_rtt(&net, &s, start.elapsed(), timed);
            let _ = done.send(Ok((rtt, arrived(&net, &s))));
            let named = Flight {
                items: item::CHECK,
                bytes: 0,
            };
            let _ = s.write_all(&frame(named, Some(token))).await;
        }
        Err(e) => {
            let _ = done.send(Err(e));
        }
    }
}

/// A check's outcome: its round trip and when its response came.
type Checked = io::Result<(Duration, Option<Instant>)>;

/// When what `s` last delivered reached the host, as the net timed it.
fn arrived<N: Net>(net: &N, s: &N::TcpStream) -> Option<Instant> {
    Instant::now().checked_sub(net.delivered_ago(s)?)
}

/// Has `s`'s writes of this step leave `turn` after `at`, what they
/// answer's arrival, where that is before the step: as the stack's
/// reaction to it ([`Turns`]). One whose turn ends later goes at the
/// step's start, past its arrival still, and so do the step's writes on
/// `s` stamped before it.
fn reply_after<N: Net>(net: &N, s: &N::TcpStream, at: Option<Instant>, turn: Option<Duration>) {
    let (Some(at), Some(turn)) = (at, turn) else {
        return;
    };
    net.send_ago(s, Instant::now().saturating_duration_since(at + turn));
}

/// What a flight in [`shake`] answers.
enum Answers {
    Nothing,
    /// The check's response, which came then.
    Check(Option<Instant>),
    /// The flight read last.
    Read,
}

/// Runs `hs` over `s` until its association is up, from the flight of
/// `first` that came already, its ICE check `check`, which resolves to the
/// check's round trip, and its peer's check reaching it, `peer_check`;
/// `token` goes ahead of its first flight once its check succeeds. A check
/// that has succeeded counts before `first`. The stack's timer ticks count
/// from `origin`. Each reply to a check's response or a flight leaves its
/// stack's turn after what it answers came ([`Turns`]), and so does the
/// link's first write once the association is up.
#[allow(clippy::too_many_arguments)]
async fn shake<N: Net, F, P>(
    net: &N,
    s: &mut N::TcpStream,
    hs: &mut Handshake,
    mut first: Option<u16>,
    check: F,
    peer_check: P,
    mut token: Option<u64>,
    origin: Instant,
) -> io::Result<()>
where
    F: std::future::Future<Output = Checked>,
    P: std::future::Future<Output = ()>,
{
    tokio::pin!(check);
    tokio::pin!(peer_check);
    let mut buf = Vec::new();
    let (mut checked, mut peer) = (false, false);
    let mut rtt = Duration::ZERO;
    // The deadline of the client's next resend of its ClientHello, and the
    // wait it doubles.
    let mut resend: Option<(Instant, Duration)> = None;
    let mut last = None;
    while !hs.up {
        let fires = resend.map(|(at, _)| hs.me.fires_at(at, origin));
        let (out, answers) = tokio::select! {
            biased;
            got = &mut check, if !checked => {
                let (r, at) = got?;
                rtt = r;
                checked = true;
                (hs.writable(), Answers::Check(at))
            }
            () = &mut peer_check, if !peer => {
                peer = true;
                (hs.peer_checked(), Answers::Nothing)
            }
            () = std::future::ready(()), if first.is_some() => {
                let out = first.take().map_or_else(Flight::default, |items| hs.receive(items));
                (out, Answers::Read)
            }
            () = tokio::time::sleep_until(fires.unwrap_or_else(Instant::now)), if fires.is_some() => {
                let out = match (hs.resend(), resend) {
                    (Some(hello), Some((_, wait))) => {
                        let wait = (wait * 2).min(MAX_RESEND);
                        resend = Some((Instant::now() + wait, wait));
                        hello
                    }
                    _ => {
                        resend = None;
                        Flight::default()
                    }
                };
                (out, Answers::Nothing)
            }
            got = next_flight(s, &mut buf) => {
                let (items, _) = got?;
                let out = hs.receive(items);
                if hs.resend().is_none() {
                    resend = None;
                }
                (out, Answers::Read)
            }
        };
        let at = match answers {
            Answers::Nothing => None,
            Answers::Check(at) => at,
            Answers::Read => arrived(net, s),
        };
        last = at.or(last);
        if out.items & item::HELLO != 0 && resend.is_none() {
            let wait = hs.me.resend_after(rtt);
            resend = Some((Instant::now() + wait, wait));
        }
        if !out.is_empty() || (token.is_some() && checked) {
            reply_after(net, s, at, hs.me.turns.of(out.items));
            s.write_all(&frame(out, token.take())).await?;
        }
    }
    let open = (hs.me.turns != Turns::NONE).then_some(hs.me.turns.open);
    reply_after(net, s, last, open);
    Ok(())
}

/// What an answer waits out before its check.
type CheckWait = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

/// The answering side: checks the offer's address once `check_wait` is
/// out (its stack's `check_ready` after the offer came, or with the host's
/// draws its batch's turn), with the dial that
/// carries the handshake, its token first, tells `checked` once that check
/// succeeds (its round trip [`check_rtt`]'s, `timed` with the draws),
/// hears of the offering side's check from `peer_check`, and
/// attaches the stream once its association is up.
#[allow(clippy::too_many_arguments)]
async fn answer<N: Net>(
    net: N,
    addr: String,
    token: u64,
    me: LinkSetup,
    peer: LinkSetup,
    conn: RtcConn,
    cap: usize,
    check_wait: CheckWait,
    timed: bool,
    checked: oneshot::Sender<()>,
    peer_check: oneshot::Receiver<()>,
    origin: Instant,
    inputs: Inputs<Transport<N>>,
) {
    let linked = async {
        check_wait.await;
        let start = Instant::now();
        let mut s = net.tcp_connect(addr).await?;
        let rtt = check_rtt(&net, &s, start.elapsed(), timed);
        let at = arrived(&net, &s);
        let _ = checked.send(());
        if !net.open_channel(&s, cap) {
            return Err(io::Error::other("the net models no data channels"));
        }
        let mut hs = Handshake::new(me, peer, me.answers_active, false);
        let check = std::future::ready(Ok((rtt, at)));
        // A dropped sender is an answer closed, not a check.
        let peer_check = async {
            if peer_check.await.is_err() {
                std::future::pending::<()>().await;
            }
        };
        shake(
            &net,
            &mut s,
            &mut hs,
            None,
            check,
            peer_check,
            Some(token),
            origin,
        )
        .await?;
        Ok(s)
    };
    match linked.await {
        Ok(s) => attach(&net, s, conn, &inputs).await,
        Err(e) => closed(&inputs, conn, &e).await,
    }
}

/// The offering side, once it has both the answering side's stream (the
/// first flight on it, `items`) and its answer: runs the handshake over
/// the stream with its own check, and attaches the stream once its
/// association is up.
#[allow(clippy::too_many_arguments)]
async fn offer<N: Net>(
    net: N,
    mut s: N::TcpStream,
    items: u16,
    checked: oneshot::Receiver<Checked>,
    me: LinkSetup,
    peer: LinkSetup,
    conn: RtcConn,
    origin: Instant,
    inputs: Inputs<Transport<N>>,
) {
    let linked = async {
        let mut hs = Handshake::new(me, peer, !peer.answers_active, true);
        let check = async {
            checked
                .await
                .unwrap_or_else(|_| Err(io::Error::other("the check went")))
        };
        let peer_check = std::future::pending();
        shake(
            &net,
            &mut s,
            &mut hs,
            Some(items),
            check,
            peer_check,
            None,
            origin,
        )
        .await?;
        Ok(s)
    };
    match linked.await {
        Ok(s) => attach(&net, s, conn, &inputs).await,
        Err(e) => closed(&inputs, conn, &e).await,
    }
}

/// One of our offers: the answer's stack and our check of its address,
/// once applied, and the answering side's stream once its first flight
/// has come.
struct Offer<S> {
    conn: RtcConn,
    answer: Option<(LinkSetup, oneshot::Receiver<Checked>)>,
    stream: Option<(S, u16)>,
}

/// Answers the proxy's commands and links its connections, until the
/// proxy or the listener goes away.
pub(crate) async fn run<N: Net>(net: N, model: ModelRtc<N>, inputs: Inputs<Transport<N>>) {
    let ModelRtc {
        listener,
        advertised,
        checks,
        checks_at,
        mut commands,
        cap,
    } = model;
    let me = net.link_setup();
    let draws = net.setup_draws();
    let origin = draws.as_ref().map_or_else(Instant::now, |d| d.tick_origin);
    // With the host's draws, the page's offers being made together, and its
    // latest offer post where it posts each as it is asked for.
    let mut offer_batch: Option<OfferBatch> = None;
    let mut last_post: Option<Instant> = None;
    // The offers the page is setting together, while their checks are to go.
    let mut batch: Option<Batch> = None;
    let checks = Arc::new(checks);
    // Our offers by token, and our answers waiting for their offering
    // side's check. Bound: the endpoint's live connections.
    let mut offers: BTreeMap<u64, Offer<N::TcpStream>> = BTreeMap::new();
    let mut answers: BTreeMap<u64, oneshot::Sender<()>> = BTreeMap::new();
    // Our answering checks' successes, until their answer is asked for.
    // Bound: the endpoint's live connections.
    let mut checked: BTreeMap<u64, oneshot::Receiver<()>> = BTreeMap::new();
    type Accepted<S> = (u64, u16, S);
    let (accepted_tx, mut accepted) = mpsc::unbounded_channel::<Accepted<N::TcpStream>>();
    // The answers whose offering side's check came, by token.
    let (heard_tx, mut heard) = mpsc::unbounded_channel::<u64>();
    let accepts = {
        let (net, heard_tx) = (net.clone(), heard_tx.clone());
        async move {
            while let Ok((mut s, _)) = net.tcp_accept(&listener).await {
                if !net.open_channel(&s, cap) {
                    continue;
                }
                let (tx, heard_tx) = (accepted_tx.clone(), heard_tx.clone());
                tokio::spawn(async move {
                    let mut buf = Vec::new();
                    match next_flight(&mut s, &mut buf).await {
                        Ok((items, Some(token))) if items & item::CHECK != 0 => {
                            let _ = heard_tx.send(token);
                        }
                        Ok((items, Some(token))) => {
                            let _ = tx.send((token, items, s));
                        }
                        _ => {}
                    }
                });
            }
        }
    };
    tokio::pin!(accepts);
    let requests = {
        let (net, checks) = (net.clone(), checks.clone());
        async move {
            let mut buf = [0u8; CHECK_BYTES];
            while let Ok((n, _)) = net.udp_recv_from(&checks, &mut buf).await {
                if let Some(token) = buf.get(..n).and_then(parse_check) {
                    let _ = heard_tx.send(token);
                }
            }
            std::future::pending::<()>().await;
        }
    };
    tokio::pin!(requests);
    loop {
        let ready = tokio::select! {
            biased;
            () = &mut accepts => return,
            () = &mut requests => return,
            cmd = commands.recv() => {
                let Some(cmd) = cmd else {
                    return;
                };
                match cmd {
                    RtcCommand::Create { .. } => None,
                    RtcCommand::Describe { conn, sdp_type } => {
                        let offer = sdp_type == SdpType::Offer;
                        // With the host's draws, the offer's batch and its
                        // place in it (a stack that ends its gathering on its
                        // tick), else when the page posts it.
                        let (mut gather, mut posted) = (None, None);
                        let (sdp, ready, check) = if offer {
                            offers.insert(conn.0, Offer { conn, answer: None, stream: None });
                            let sdp = format!("{PREFIX} {advertised} {} {}", conn.0, me.name);
                            if let Some(d) = &draws {
                                let now = Instant::now();
                                if me.gather_after_set.is_some() {
                                    let lead = || {
                                        let us = d.draw(me.set_lead).unwrap_or(0.0)
                                            + me.set_ready.as_secs_f64() * 1e6;
                                        Duration::from_secs_f64(us.max(0.0) / 1e6)
                                    };
                                    gather = Some(join_offers(&mut offer_batch, now, me.make_per_offer, lead));
                                } else {
                                    let at = now + me.offer_ready + me.offer_post;
                                    let at = last_post.map_or(at, |p| at.max(p + me.post_per_offer));
                                    last_post = Some(at);
                                    posted = Some(at);
                                }
                            }
                            (sdp, me.offer_ready, None)
                        } else {
                            let sdp = format!(
                                "{PREFIX} answer {advertised} {} {} {checks_at}",
                                conn.0, me.name
                            );
                            let check = me.answer_after_check.zip(checked.remove(&conn.0));
                            (sdp, me.answer_ready, check)
                        };
                        let transit = draws
                            .as_ref()
                            .and_then(|d| d.draw(if offer { OFFER_TRANSIT } else { ANSWER_TRANSIT }));
                        let post = if offer { me.offer_post } else { Duration::ZERO };
                        let inputs = inputs.clone();
                        tokio::spawn(async move {
                            let deadline = Instant::now() + ready;
                            let mut at = deadline;
                            if let Some((after, ok)) = check
                                && let Ok(Ok(())) = tokio::time::timeout_at(deadline, ok).await
                            {
                                at = at.min(Instant::now() + after);
                            }
                            at = match gather {
                                Some((first, place)) => offer_post_at(&me, offers_set(first).await, place, origin),
                                None => posted.unwrap_or(at + post),
                            };
                            if let Some(us) = transit {
                                at = shifted(at, us, Instant::now());
                            }
                            tokio::time::sleep_until(at).await;
                            let desc = SessionDescription { sdp_type, sdp };
                            signal(&inputs, conn, RtcSignal::LocalDescription(desc)).await;
                        });
                        None
                    }
                    RtcCommand::SetRemote { conn, desc } if desc.sdp_type == SdpType::Offer => {
                        match parse_offer(&desc) {
                            Some((addr, token, peer)) => {
                                let (tx, rx) = oneshot::channel();
                                answers.insert(conn.0, tx);
                                let (ok_tx, ok) = oneshot::channel();
                                checked.insert(conn.0, ok);
                                let drawn = draws.as_ref().and_then(|d| d.draw(me.check_spread));
                                let wait: CheckWait = match drawn {
                                    Some(us) => {
                                        let ready = Duration::from_secs_f64(us.max(0.0) / 1e6);
                                        let (b, place) =
                                            join_batch(&mut batch, Instant::now(), ready);
                                        Box::pin(batch_check(b, place, me.check_per_offer))
                                    }
                                    None => {
                                        let ready = me.check_ready;
                                        Box::pin(async move {
                                            if !ready.is_zero() {
                                                tokio::time::sleep(ready).await;
                                            }
                                        })
                                    }
                                };
                                let (net, inputs) = (net.clone(), inputs.clone());
                                tokio::spawn(answer(
                                    net,
                                    addr,
                                    token,
                                    me,
                                    peer,
                                    conn,
                                    cap,
                                    wait,
                                    draws.is_some(),
                                    ok_tx,
                                    rx,
                                    origin,
                                    inputs,
                                ));
                            }
                            None => {
                                let e = RtcSignal::Closed("not a model offer".to_owned());
                                signal(&inputs, conn, e).await;
                            }
                        }
                        None
                    }
                    RtcCommand::SetRemote { conn, desc } => {
                        let answer = parse_answer(&desc);
                        if let (Some(o), Some((addr, token, peer, at))) =
                            (offers.get_mut(&conn.0), answer)
                        {
                            let (tx, rx) = oneshot::channel();
                            let (net, socket) = (net.clone(), checks.clone());
                            let timed = draws.is_some();
                            tokio::spawn(check(net, socket, at, addr, token, timed, tx));
                            o.answer = Some((peer, rx));
                            Some(conn.0)
                        } else if offers.remove(&conn.0).is_some() {
                            let e = RtcSignal::Closed("not a model answer".to_owned());
                            signal(&inputs, conn, e).await;
                            None
                        } else {
                            None
                        }
                    }
                    RtcCommand::Close { conn } => {
                        offers.remove(&conn.0);
                        answers.remove(&conn.0);
                        checked.remove(&conn.0);
                        None
                    }
                }
            }
            got = accepted.recv() => got.and_then(|(token, items, s)| {
                offers.get_mut(&token).map(|o| {
                    o.stream = Some((s, items));
                    token
                })
            }),
            got = heard.recv() => {
                if let Some(tx) = got.and_then(|token| answers.remove(&token)) {
                    let _ = tx.send(());
                }
                None
            }
        };
        let Some(token) = ready else {
            continue;
        };
        if !offers
            .get(&token)
            .is_some_and(|o| o.answer.is_some() && o.stream.is_some())
        {
            continue;
        }
        let Some(Offer {
            conn,
            answer: Some((peer, checked)),
            stream: Some((s, items)),
        }) = offers.remove(&token)
        else {
            continue;
        };
        let (net, inputs) = (net.clone(), inputs.clone());
        tokio::spawn(offer(
            net, s, items, checked, me, peer, conn, origin, inputs,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two sides' handshakes over a path of one-way delay `d` each way, the
    /// offering side's check succeeding `check` after the answering side's,
    /// so reaching the answering side at `check - 1`, and the answering
    /// side's first flight (its token and, as a client that doesn't wait for
    /// that, its hello) leaving at 0: when each side's association is up,
    /// in multiples of `d`, and the flights' items in the order they went,
    /// sender first (`true` the answering side).
    fn run_pair(
        offering: LinkSetup,
        answering: LinkSetup,
        check: u32,
    ) -> (u32, u32, Vec<(bool, u16)>) {
        let mut o = Handshake::new(offering, answering, !answering.answers_active, true);
        let mut a = Handshake::new(answering, offering, answering.answers_active, false);
        // (arrival in d, to the answering side, items), in send order.
        let mut wire: Vec<(u32, bool, u16)> = Vec::new();
        let mut sent = Vec::new();
        let reached = check.checked_sub(1);
        if reached.is_none_or(|r| r == 0) {
            a.peer_checked();
        }
        let mut first = a.writable();
        first.items |= item::TOKEN;
        wire.push((1, false, first.items));
        sent.push((true, first.items));
        let (mut o_up, mut a_up) = (None, None);
        for t in 0..64u32 {
            if t == check {
                let f = o.writable();
                if !f.is_empty() {
                    wire.push((t + 1, true, f.items));
                    sent.push((false, f.items));
                }
            }
            if t > 0 && reached == Some(t) {
                let f = a.peer_checked();
                if !f.is_empty() {
                    wire.push((t + 1, false, f.items));
                    sent.push((true, f.items));
                }
            }
            if t > 0 && a.resend().is_some() && t % 8 == 0 {
                let f = a.hello();
                wire.push((t + 1, false, f.items));
                sent.push((true, f.items));
            }
            let due: Vec<(u32, bool, u16)> = wire.iter().copied().filter(|w| w.0 == t).collect();
            for (_, to_a, items) in due {
                let side = if to_a { &mut a } else { &mut o };
                let f = side.receive(items);
                if !f.is_empty() {
                    wire.push((t + 1, !to_a, f.items));
                    sent.push((to_a, f.items));
                }
            }
            if o.up && o_up.is_none() {
                o_up = Some(t);
            }
            if a.up && a_up.is_none() {
                a_up = Some(t);
            }
        }
        (o_up.unwrap_or(99), a_up.unwrap_or(99), sent)
    }

    use item::*;

    /// Between browsers the answering side is the DTLS 1.3 client: its
    /// hello waits at the offering side until that side's check succeeds,
    /// and the server's flight, the client's finished and INIT, the
    /// server's INIT and INIT ACK, and the client's INIT ACK and COOKIE ECHO
    /// follow a one-way delay apart: the offering side is up three trips
    /// after its check, the answering side a trip later.
    #[test]
    fn browsers_take_dtls13_then_both_inits() {
        let (o, a, sent) = run_pair(LinkSetup::CHROMIUM, LinkSetup::CHROMIUM, 2);
        assert_eq!(
            sent,
            vec![
                (true, TOKEN | HELLO),
                (false, FLIGHT),
                (true, FINISH | INIT),
                (false, INIT | INIT_ACK),
                (true, INIT_ACK | COOKIE_ECHO),
                (false, COOKIE_ACK),
            ]
        );
        assert_eq!((o, a), (2 + 4, 2 + 5));
    }

    /// A hello that reaches a Chromium server before its check succeeds is
    /// kept; a Firefox server drops it, and the setup waits for the
    /// client's resend.
    #[test]
    fn a_firefox_server_waits_for_the_resent_hello() {
        let (o, _, _) = run_pair(LinkSetup::CHROMIUM, LinkSetup::CHROMIUM, 2);
        let (fo, _, sent) = run_pair(LinkSetup::FIREFOX, LinkSetup::CHROMIUM, 2);
        assert_eq!(o, 6);
        // The resend at 8 reaches it at 9, its flight goes then.
        assert_eq!(fo, 9 + 4);
        assert_eq!(sent.iter().filter(|s| s.1 & HELLO != 0).count(), 2);
        // A check before the hello comes: no resend.
        let (fo, _, sent) = run_pair(LinkSetup::FIREFOX, LinkSetup::CHROMIUM, 0);
        assert_eq!(fo, 1 + 4);
        assert_eq!(sent.iter().filter(|s| s.1 & HELLO != 0).count(), 1);
    }

    /// A Firefox client sends its hello once the offering side's check
    /// reaches it, not at its own check: to a Firefox server it then comes
    /// with that side's check's success, and the setup takes no resend.
    #[test]
    fn a_firefox_client_sends_its_hello_as_the_offering_sides_check_comes() {
        let mut c = Handshake::new(LinkSetup::FIREFOX, LinkSetup::CHROMIUM, true, false);
        assert!(c.writable().is_empty());
        assert_eq!(c.resend(), None);
        assert_eq!(c.peer_checked().items, HELLO);
        assert!(c.resend().is_some());
        assert!(c.peer_checked().is_empty(), "one hello");
        let mut c = Handshake::new(LinkSetup::FIREFOX, LinkSetup::CHROMIUM, true, false);
        assert!(c.peer_checked().is_empty(), "its own check first");
        assert_eq!(c.writable().items, HELLO);
        let mut c = Handshake::new(LinkSetup::CHROMIUM, LinkSetup::FIREFOX, true, false);
        assert_eq!(c.writable().items, HELLO, "Chromium's at its own check");

        let (o, a, sent) = run_pair(LinkSetup::FIREFOX, LinkSetup::FIREFOX, 2);
        assert_eq!((o, a), (2 + 4, 2 + 5));
        assert_eq!(sent.iter().filter(|s| s.1 & HELLO != 0).count(), 1);
        // A later check: the token goes at its own check, the hello at the
        // offering side's.
        let (o, _, sent) = run_pair(LinkSetup::CHROMIUM, LinkSetup::FIREFOX, 5);
        assert_eq!(sent.get(..2), Some(&[(true, TOKEN), (true, HELLO)][..]));
        assert_eq!(o, 5 + 4);
    }

    /// A DTLS 1.3 server's ACK of the client's last flight is sized by the
    /// client's records, its stack's.
    #[test]
    fn the_servers_ack_follows_the_clients_flight() {
        let ack = |server: LinkSetup, client: LinkSetup| {
            let mut s = Handshake::new(server, client, false, true);
            s.writable();
            s.receive(HELLO);
            s.receive(FINISH | INIT).bytes
        };
        let (c, f) = (LinkSetup::CHROMIUM, LinkSetup::FIREFOX);
        let init_ack = c.flights.init_ack;
        assert_eq!(ack(c, f), 72 + c.flights.init + init_ack);
        assert_eq!(ack(f, c), 40 + f.flights.init + init_ack);
    }

    /// Chromium's resend timer fires on its metronome's next tick.
    #[test]
    fn chromiums_resends_fire_on_its_ticks() {
        let o = Instant::now();
        let ms = |m: u64| Duration::from_millis(m);
        let c = LinkSetup::CHROMIUM;
        assert_eq!(c.fires_at(o + ms(50), o), o + Duration::from_micros(62_500));
        assert_eq!(c.fires_at(o + ms(125), o), o + ms(125), "a tick");
        assert_eq!(c.fires_at(o, o), o);
        assert_eq!(LinkSetup::FIREFOX.fires_at(o + ms(50), o), o + ms(50));
    }

    /// A spread is linear between its knots, from the first quantile's to
    /// the last's; none reads 0. Every stack's runs from 0 to 1000 permille
    /// in order, the transits' and Chromium's setting's medians are 0 (the
    /// stacks' fixed parts hold them), its check delay keeps
    /// `check_ready` at its median, and a lone Chromium offer drawn at its
    /// medians is set 9.8 ms after its ask, as on the testbed, and half a
    /// tick past its gathering's start leaves within 2.5 ms of the
    /// testbed's lone offers' median, 132.4 ms.
    #[test]
    fn spreads_read_their_knots_and_keep_the_stacks_medians() {
        let s = Spread(&[(0, 1_000), (100, 2_000), (500, 4_000), (1000, 9_000)]);
        for (u, want) in [
            (0.0, 1_000.0),
            (0.05, 1_500.0),
            (0.5, 4_000.0),
            (0.75, 6_500.0),
        ] {
            assert!((s.at(u) - want).abs() < 1e-6, "{u}: {}", s.at(u));
        }
        assert!((s.at(1.0) - 9_000.0).abs() < 1e-6);
        assert!(Spread::NONE.at(0.3).abs() < 1e-12);
        let c = LinkSetup::CHROMIUM;
        for spread in [c.set_lead, c.check_spread, OFFER_TRANSIT, ANSWER_TRANSIT] {
            let ps: Vec<u16> = spread.0.iter().map(|k| k.0).collect();
            assert!(ps.first() == Some(&0) && ps.last() == Some(&1000), "{ps:?}");
            assert!(
                ps.iter().zip(ps.iter().skip(1)).all(|(a, b)| a < b),
                "{ps:?}"
            );
        }
        assert!(OFFER_TRANSIT.at(0.5).abs() < 1e-6 && ANSWER_TRANSIT.at(0.5).abs() < 1e-6);
        assert!(c.set_lead.at(0.5).abs() < 1e-6);
        let ms = |d: Duration| d.as_secs_f64() * 1e3;
        let half_tick = c.tick.map_or(0.0, ms) / 2.0;
        assert!((c.check_spread.at(0.5) / 1e3 - ms(c.check_ready)).abs() < 1e-6);
        let set = c.set_lead.at(0.5) / 1e3 + ms(c.set_ready);
        assert!((set - 9.8).abs() < 1e-6, "{set}");
        let lone = set + c.gather_after_set.map_or(0.0, ms) + half_tick + ms(GATHER_TAIL);
        assert!((lone - 132.4).abs() < 2.5, "{lone}");
    }

    /// The offers a node asks for while the first is still to be set join
    /// its batch, each moving that first setting on by `make_per_offer`
    /// and drawing nothing; they are set `set_per_offer` apart, gather on
    /// one tick and leave in turn. One asked once they are set opens the
    /// next batch.
    #[tokio::test(start_paused = true)]
    async fn a_pages_offers_asked_together_set_and_post_in_turn() {
        let me = LinkSetup::CHROMIUM;
        let o = Instant::now();
        let mut batch = None;
        let mut leads = 0;
        let mut lead = || {
            leads += 1;
            me.set_ready
        };
        let joined: [_; 3] =
            std::array::from_fn(|_| join_offers(&mut batch, o, me.make_per_offer, &mut lead));
        let [(rx, _), ..] = &joined;
        let first = offers_set(rx.clone()).await;
        assert_eq!(first, o + me.set_ready + me.make_per_offer * 2);
        // Set 13.5, 13.6 and 13.7 ms in, their gathering ends at the tick at
        // or after 126.5-126.7 ms: 140.625.
        let posts = joined.map(|(_, place)| offer_post_at(&me, first, place, o));
        let at = o + Duration::from_micros(140_625) + GATHER_TAIL + me.offer_post;
        let step = me.post_per_offer;
        assert_eq!(posts, [at, at + step, at + step * 2]);
        let now = Instant::now();
        assert!(now >= first);
        let (next, place) = join_offers(&mut batch, now, me.make_per_offer, &mut lead);
        assert_eq!((*next.borrow(), place), (now + me.set_ready, 0));
        assert_eq!(leads, 2);
    }

    /// The offers a page is asked to set before its batch's first check
    /// goes join the batch, whose first check goes once the last joiner's
    /// is ready, sooner or later than before, and the rest in the order
    /// their offers were asked; one asked as the first check goes opens the
    /// next. A waiting check follows its batch's first as it moves sooner.
    #[tokio::test(start_paused = true)]
    async fn an_answering_pages_offers_check_together() {
        let o = Instant::now();
        let ms = |m: u64| Duration::from_millis(m);
        let step = Duration::from_micros(370);
        let mut batch = None;
        let (a, pa) = join_batch(&mut batch, o, ms(3));
        let (_, pb) = join_batch(&mut batch, o + ms(1), ms(3));
        assert_eq!(*a.borrow(), (o + ms(4), 2));
        let waiting = tokio::spawn(batch_check(a.clone(), pa, step));
        tokio::time::sleep_until(o + ms(2)).await;
        let (_, pc) = join_batch(&mut batch, o + ms(2), ms(1));
        assert_eq!((pa, pb, pc), (0, 1, 2));
        let (first, offers) = *a.borrow();
        assert_eq!((first, offers), (o + ms(3), 3), "the last joiner's draw");
        let at: Vec<_> = [pa, pb, pc].map(|p| batch_check_at(first, p, step)).into();
        assert_eq!(at, [o + ms(3), o + ms(3) + step, o + ms(3) + step * 2]);
        assert!(waiting.await.is_ok(), "the waiting check returns");
        assert_eq!(Instant::now(), o + ms(3), "the first check went sooner");
        let (d, pd) = join_batch(&mut batch, o + ms(3), ms(2));
        assert_eq!((pd, *d.borrow()), (0, (o + ms(5), 1)));
        assert_eq!(*a.borrow(), (o + ms(3), 3), "the closed batch stays");
    }

    /// A shift moves an instant either way, never before its floor.
    #[test]
    fn shifts_stop_at_their_floor() {
        let o = Instant::now();
        let ms = |m: u64| Duration::from_millis(m);
        assert_eq!(
            shifted(o + ms(10), 2_500.0, o),
            o + Duration::from_micros(12_500)
        );
        assert_eq!(
            shifted(o + ms(10), -2_500.0, o),
            o + Duration::from_micros(7_500)
        );
        assert_eq!(shifted(o + ms(1), -2_500.0, o), o);
    }

    /// str0m answers passive: the offering side is the DTLS 1.2 client,
    /// whose hello goes at its check. 1.2 takes a second round trip, and
    /// str0m's server sends no INIT: the client's INIT follows the server's
    /// Finished.
    #[test]
    fn str0m_answers_as_a_dtls12_server_without_an_init() {
        let (o, a, sent) = run_pair(LinkSetup::CHROMIUM, LinkSetup::STR0M, 2);
        assert_eq!(
            sent,
            vec![
                (true, TOKEN),
                (false, HELLO),
                (true, FLIGHT),
                (false, FINISH),
                (true, SERVER_FINISH),
                (false, INIT),
                (true, INIT_ACK),
                (false, COOKIE_ECHO),
                (true, COOKIE_ACK),
            ]
        );
        assert_eq!((o, a), (2 + 8, 2 + 7));
    }

    /// A server of a hybrid stack sends the key share's ciphertext only to a
    /// hybrid client.
    #[test]
    fn the_server_flight_carries_the_hybrid_ciphertext_between_hybrid_stacks() {
        let flight = |client: LinkSetup| {
            let mut s = Handshake::new(LinkSetup::CHROMIUM, client, false, true);
            s.writable();
            s.receive(HELLO).bytes
        };
        assert_eq!(flight(LinkSetup::CHROMIUM), 629 + 1086);
        assert_eq!(flight(LinkSetup::FIREFOX), 629);
    }

    #[test]
    fn frames_carry_the_token_and_the_flights_bytes() {
        let mut buf = frame(
            Flight {
                items: HELLO,
                bytes: 1463,
            },
            Some(7),
        );
        assert_eq!(buf.len(), 1463 - CHANNEL_HEADERS);
        buf.extend_from_slice(b"link");
        assert_eq!(parse(&mut buf), Some((TOKEN | HELLO, Some(7))));
        assert_eq!(buf, b"link");
        let mut small = frame(
            Flight {
                items: COOKIE_ACK,
                bytes: 38,
            },
            None,
        );
        assert_eq!(small.len(), HEADER);
        assert_eq!(parse(&mut small), Some((COOKIE_ACK, None)));
        assert!(parse(&mut vec![0, 2, 0, 9, 1]).is_none());
    }

    #[test]
    fn chromium_resends_at_twice_its_check_within_its_bounds() {
        let c = LinkSetup::CHROMIUM;
        let us = Duration::from_micros;
        assert_eq!(nearest_ms(us(30_120)), Duration::from_millis(30));
        assert_eq!(nearest_ms(us(30_500)), Duration::from_millis(31));
        assert_eq!(
            c.resend_after(nearest_ms(us(30_120))),
            Duration::from_millis(60),
            "the halve-7 cells' seed checks"
        );
        assert_eq!(
            c.resend_after(Duration::from_millis(20)),
            Duration::from_millis(50)
        );
        assert_eq!(
            c.resend_after(Duration::from_millis(45)),
            Duration::from_millis(90)
        );
        assert_eq!(
            c.resend_after(Duration::from_secs(9)),
            Duration::from_secs(3)
        );
        assert_eq!(
            LinkSetup::FIREFOX.resend_after(Duration::from_millis(200)),
            Duration::from_millis(50)
        );
    }

    #[test]
    fn descriptions_name_the_stack() {
        let offer = SessionDescription {
            sdp_type: SdpType::Offer,
            sdp: format!("{PREFIX} 10.0.0.1:4000 9 firefox"),
        };
        assert_eq!(
            parse_offer(&offer),
            Some(("10.0.0.1:4000".to_owned(), 9, LinkSetup::FIREFOX))
        );
        let answer = SessionDescription {
            sdp_type: SdpType::Answer,
            sdp: format!("{PREFIX} answer 10.0.0.2:4000 3 str0m 10.0.0.2:49152"),
        };
        let checks = SocketAddr::from(([10, 0, 0, 2], 49152));
        assert_eq!(
            parse_answer(&answer),
            Some(("10.0.0.2:4000".to_owned(), 3, LinkSetup::STR0M, checks))
        );
        let bare = SessionDescription {
            sdp_type: SdpType::Answer,
            sdp: format!("{PREFIX} answer 10.0.0.2:4000 3 str0m"),
        };
        assert_eq!(parse_answer(&bare), None);
    }

    #[test]
    fn checks_name_the_answer() {
        let d = check_datagram(0x0102_0304_0506_0708);
        assert_eq!(d.len(), CHECK_BYTES);
        assert_eq!(parse_check(&d), Some(0x0102_0304_0506_0708));
        // Not a STUN, DTLS or SCTP message's first byte.
        assert!(!matches!(d.first(), Some(0..=3 | 20..=22 | 128..=191)));
        assert_eq!(parse_check(&[0; 9]), None);
        assert_eq!(parse_check(d.get(..5).unwrap_or_default()), None);
    }
}
