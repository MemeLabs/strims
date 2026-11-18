---
name: protocol-design
description: "Design rules for the strims swarm protocol, its links and the sims' model, and how to test a change to them. Use when designing or changing the ppspp protocol (crates/strims-ppspp): its messages and wire format, or what a peer's claims may decide; transport selection or links (strims-vnic, strims-vpn, strims-network); the sims' model (clocks, rng, hostile kinds); or when planning how to test a protocol or scheduler change. The schedulers' behaviour is crates/PLAN.md's open work, not this skill's. For sim results use sim-lab; for what the sims record, sim-metrics."
---

# Designing the swarm protocol

`crates/CONVENTIONS.md` holds the rules every crate keeps; these are the
design rules above them. Dropped ideas are in `crates/TRIED.md`, open
work in `crates/PLAN.md`. The schedulers' behaviour is PLAN.md's: its
Swarm and Network construction items.

## Trust

CONVENTIONS' "Hostile peers" governs. A peer's claim about itself is
never a signal: one that claims spare capacity draws the seed's copies
and withholds them. Node-wide path readings take a low quantile across
links (`strims_base::path::aggregate`), so a minority that delays its
echoes can't move them.

## Wire

New ppspp messages assume every peer supports them: no
`SupportedMessages` option (it has no encoding; the decoder refuses it),
no version gate, no capability handshake. RFC 7574 inspires and doesn't
bind (`MessageType::Announce` is ours). What an old peer would do goes in
the commit message, not a gate. CONVENTIONS' add-only rule, whose
unignorable fields need an agreed version, governs protobuf messages
between peers; the ppspp codec's format is pinned by
`crates/strims-ppspp/golden/vectors.txt`, updated in the same commit.

## Links and transports

- **One link per neighbour.** vpn refuses an offer from a host that is
  already a peer, starts no exchange with a host while one with it is
  open, offered or answered, and of two crossed exchanges the lower host
  id's wins (`crates/strims-vpn/src/peer_exchange.rs`).
  vnic keeps a host's newest link at each end, so two links between a
  pair can leave none. Node-core gives an exchange
  one candidate per interface its driver has, TCP, WebSocket, WebRTC
  (`State::exchange`), and vnic's `LinkCandidatePool` keeps the first
  that links, in that order. No fix may need a second association (most
  peers can't open one): later fixes belong to the swarm (more
  neighbours, framing, fewer packets).
- **Worst case first.** Tune against the all-browser case, the `browsers`
  tier (`suite.browsers_tier`: every host a browser, the seed included).
  Mixed audiences (a native seed) wait until swarms stop collapsing.
- **Uniform shapes.** An API across transports carries one comparable
  number per concept. Decisions read vnic's path measure
  (`strims_base::path`, `Runner::set_peer_path`), whose probes ride the
  link's byte stream above every transport. TCP-only detail (unacked,
  cwnd) stays in capture samples (`crates/strims-ppspp-capture/src/sample.rs`).

## The sims' model

- **One rng.** A sim feature draws from the run's one rng at a fixed
  point, and nothing when off, so off-runs trace as before; never a
  stream of its own (`det_rng(seed ^ TAG)`): any change moves the
  timeline anyway. strims-sim's order is `World::new`'s (lib.rs,
  Determinism); the harness forks its runners' from `Sim::rng`. The
  seed-derived streams (WAN paths; the harness's sites and greedy draw)
  are always on in their scenarios, and no precedent.
- **Clocks.** A clock off by more than an hour is user error for the UI
  to surface; the service layer needn't tolerate it. Both sims cap every
  offset at `MAX_SKEW`, 10 minutes (`crates/strims-sim/src/skew.rs`),
  inside the tightest cross-clock tolerances: a record's 30-minute age
  (`HASH_TABLE_MAX_RECORD_AGE`) and a link certificate's hour either side
  (`CERT_PREDATE_DURATION`, `HOST_CERT_VALID_DURATION`). The harness's
  asserting tests run real clocks (`ClockSkew::Real` in `Config::new`).
- **Hostile peers are always there.** strims-sim's measuring tiers run
  `STRIMS_SIM_HOSTILE=mix` (`Scenario::mix`) unless a cell names its own;
  honest-only is an explicit `off` arm. Checks and results count honest
  viewers only (`HostClass::Hostile` skipped), as in the harness, whose
  greedy viewers (`Greedy`) are in every run.

## Testing a protocol change

1. **Correctness:** `cargo nextest run -p strims-ppspp --test sim`, with
   the unit tests. The harness asserts only what holds however a
   swarm is timed: honest viewers' delivery (`check_delivery`), the
   bytes, bounded runner state under attack, the wire invariants, each
   attacker's behaviour, exact replay and trace invariance. `Config::new`
   hardwires its defaults; no knob reaches a test. It models no socket
   queue or acknowledgement and runs no node layer, so no timing bound or
   efficiency ratio goes back in, and its timing is never evidence.
2. **Neutrality:** a change meant to move nothing proves it. In the
   harness, dump each `Sim::run`'s trace, frames and bytes by test,
   parent against change, through an uncommitted patch (as 072d0fd6 and
   a941bf5f did); in strims-sim, `suite.py neutral --base PARENT`
   (`.claude/skills/sim-metrics/references/checks.md`).
3. **Measurement:** strims-sim's tiers with the mix, `suite.py run --base
   PARENT --tier robust` (`browsers` for transports); then the sim-lab
   skill judges the evidence.
