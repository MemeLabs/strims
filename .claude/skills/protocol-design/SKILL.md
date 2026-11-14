---
name: protocol-design
description: "Design rules for the strims swarm protocol, its links and the sims' model, and how to test a change to them. Use when designing or changing the ppspp protocol or runner (crates/strims-ppspp): its messages, schedulers, upload meter, request control or seed allocation; transport selection or links (strims-vnic, strims-vpn, strims-network); how many peers a node keeps; the sims' model (clocks, rng, hostile kinds); or when planning how to test such a change. For sim results use sim-lab; for what the sims record, sim-metrics."
---

# Designing the swarm protocol

`crates/CONVENTIONS.md` holds the rules every crate keeps; these are the
design rules above them. Dropped ideas are in `crates/TRIED.md`, open
work in `crates/PLAN.md`.

## Trust

CONVENTIONS' "Hostile peers" governs. A peer's claim about itself is
never a signal: one that claims spare capacity draws the seed's copies
and withholds them. The adaptive seed ranks holders by round trips it
times, and bars one only on a quorum of reporters (`REPORT_QUORUM`), each
checked against its own pushes (`crates/strims-ppspp/src/scheduler_seed.rs`).
Node-wide path readings take a low quantile across links
(`strims_base::path::aggregate`), so a minority that delays its echoes
can't move them.

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
  already a peer, and of two crossed exchanges the lower host id's wins
  (`crates/strims-vpn/src/peer_exchange.rs`). Node-core gives an exchange
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

## Peer pool

How many peers a node keeps is the equilibrium of its measured uplink,
hardware and network, never a tuned constant: a constant fits the config
it was tuned on. Fixed budgets (`capN` in
`crates/strims-sim/src/pool.rs`) are arms and references only.
`NodeConfig::max_peers` is the user's ceiling, not a pool size.

## Request control and the meter

- **Controllers keep pings.** Request windows time PING/PONG round trips
  (`RequestController::on_ping_rtt`): a pong waits in the peer's queues
  and arrives whether or not requests are answered. vnic's echoes skip
  the peer's queues
  (`crates/strims-ppspp/src/queue_delay.rs`), so timeouts from them run
  short; answered requests give no sample before the first answer or
  while the peer answers nothing; timed-out ages taken as round trips
  (censored samples) stretch timeouts on lossy paths. A ping-free
  controller needs an always-on round trip through the peer's queue.
- **The meter reads the uplink whatever the node sends.** A due probe
  leaves with the next frame QoS lets through (`PathMeter::probe_due`),
  so `UploadMeter` (`crates/strims-ppspp/src/runner.rs`) sees a burst's
  queue only where a probe rides into it. Timing probes behind the
  node's own bursts, per link or node-wide, is rejected: the reading then
  rests on the node's traffic pattern (HAVE frames carrying probes into
  push bursts), and fails when that pattern changes.

## HAVE removal and speculative requests

- **Summaries, not exact state.** Decisions that replace HAVEs come from
  a lightweight declared summary and an arrival model. When the model
  falls short, improve it (other statistics, censoring, calibration,
  trend breaks), judged by prediction error against true holds
  (`LagForecast`, `simdb.py forecast-report`). Every HAVE consumer gets
  a model-based replacement, measured with no HAVEs at all.
- **Cures wait.** Tuning cures for slow first waves and repair stalls
  waits on speculative requests that adapt to trend breaks (halvings,
  joins, parent changes).
- **The answer cap.** A channel's queued answers hold at most a window
  (`max_pending_requests`; `handle_request`, d27c8828): stale answers go,
  requests outside the window's span drop, covered ones queue nothing,
  and once the window is full its span ends at the newest chunk, so a
  request ahead of what the peer holds drops. A speculative or HAVE-less
  design must make room there.

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
   PARENT --tier robust` (`browsers` for transports, `net` and
   `net-hostile` for the pool); then the sim-lab skill judges the
   evidence.
