// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Seed sweeps, replay commands and the determinism check.
//!
//! - `STRIMS_SIM_SEEDS=<n>` runs seeds `1..=n` (default [`DEFAULT_SEEDS`],
//!   or [`DEFAULT_SEEDS_DEBUG`] in a debug build, so the plain
//!   `cargo test` stays short), and with `STRIMS_SIM_SEED_FROM=<k>` the
//!   `n` seeds from `k` (a [`sweep_from`] sweep starts at its own seed
//!   unless that's set).
//! - `STRIMS_SIM_SEED=<s>` runs only seed `s` (the replay of a failure).
//! - `STRIMS_SIM_TRACE=1` prints every run's trace.
//! - `STRIMS_SIM_CAPTURE=<dir>` captures every node's ppspp traffic
//!   ([`Scenario::capture`]) into `<dir>/<test>/seed-<s>/`, and the replay's
//!   into `seed-<s>-replay/`.
//! - `STRIMS_SIM_SEND_BUFFER=<ms>` gives every WAN scenario send buffers
//!   ([`Scenario::send_buffer`]) with an uplink queue of `ms` milliseconds,
//!   replacing the scenario's own, for comparisons.
//! - `STRIMS_SIM_TCP=1` gives the send buffers congestion windows and
//!   bounded access queues ([`SendBuffer::window`]).
//! - `STRIMS_SIM_TCP_SEGMENT=<bytes>` splits TCP writes into segments of at
//!   most that many bytes ([`Scenario::tcp_segment`]) in place of the
//!   window model's two packets
//!   ([`WINDOW_SEGMENT`](crate::spec::WINDOW_SEGMENT)).
//! - `STRIMS_SIM_UPLINK_BURST=<bucket>` gives every WAN host's uplink a
//!   shaper's token bucket ([`Bucket`]; the network's model of one is in
//!   `strims_wan::network`): `<bytes>`, at least a full frame
//!   ([`MIN_BURST`], and at least what the host's framing charges a full
//!   packet: with `STRIMS_SIM_FRAMING`, a bucket under a DOCSIS or ADSL
//!   frame fails the run), a bucket that size whose bytes leave at once, as
//!   `hack/testbed/run.sh`'s tbf has; `profiles`, each host its access
//!   kind's ([`Access::bucket`](strims_wan::Access::bucket)); `off`, none,
//!   the scenario's own included. Unset or blank, each scenario keeps its
//!   own: none, but `calibration_star`'s given `STRIMS_SIM_CAL_SEED_BURST`
//!   or `STRIMS_SIM_CAL_VIEWER_BURST`.
//! - `STRIMS_SIM_FRAMING=<framing>` charges every WAN host's uplink
//!   packets their link's framing past their IP bytes
//!   (`strims_wan::framing`): `ethernet`, Ethernet's 14 bytes (what the
//!   testbed's tbf counts); `profiles`, each host its access kind's
//!   ([`Access::framing`](strims_wan::Access::framing): DOCSIS's 18, ADSL's
//!   cells, ...); `off`, IP bytes alone, the scenario's own included.
//!   Unset or blank, each scenario keeps its own (IP bytes alone, but a
//!   testbed link's, `STRIMS_SIM_CAL_SEED_LINK`).
//! - `STRIMS_SIM_UPLINK_SLOTS=<slots>` hands every WAN host's uplink its
//!   messages in slots after its shaper (`strims_wan::slots`):
//!   `<min>-<max>` ms; `profiles`, each host its access kind's
//!   ([`Access::slots`](strims_wan::Access::slots): cable's grants); `off`,
//!   none, the scenario's own included. Unset or blank, each scenario's.
//! - `STRIMS_SIM_KNOWN_UPLINK=1` tells every WAN host the meter's truth
//!   as it boots and ahead of each of its stats samples, which reads it
//!   ([`HostSpec::upload_capacity_live`](crate::HostSpec::upload_capacity_live):
//!   [`Report::upload_cap_at`](crate::Report::upload_cap_at) for the second
//!   before, from the inputs the report takes it from), the oracle the
//!   upload meter is measured against. `=seed` tells only the hosts that
//!   seed a swarm, `=viewers` only the others. With `-static` (`=1-static`,
//!   `=seed-static`, `=viewers-static`) each is told once, before it sends,
//!   what its uplink carries (its rate, or its capacity schedule's mean) in
//!   IP bytes at full-size packets
//!   ([`HostSpec::upload_capacity`](crate::HostSpec::upload_capacity)).
//! - `STRIMS_SIM_MINUTES=<m>` sets the stream length of the scenarios that
//!   take one ([`stream_minutes`]).
//! - `STRIMS_SIM_CHUNK=<bytes>` and `STRIMS_SIM_MUNRO=<bytes>` give every
//!   swarm chunks and signatures of those sizes
//!   ([`with_chunks`](crate::SwarmSpec::with_chunks)): a media swarm's 4096
//!   and 32768 against the scenarios' 1 KiB frames. Either alone keeps the
//!   other's bytes.
//! - `STRIMS_SIM_MEDIA=<encoder>` and `STRIMS_SIM_SIGN=<policy>` make
//!   every swarm a live encoder's stream signed early
//!   ([`with_media`](crate::SwarmSpec::with_media); the forms of
//!   `strims_wan::live::Encoder::parse` and `Sign::parse`, `1500v` and
//!   `u8,t200`), in chunks of `STRIMS_SIM_CHUNK` bytes ([`MEDIA_CHUNK`]
//!   without it), the policy setting the chunks per signature.
//!   `STRIMS_SIM_SIGN=off`, or either alone, changes nothing.
//! - `STRIMS_SIM_PUSH_MODE=<mode>` runs every swarm in that push mode
//!   ([`with_push_mode`](crate::SwarmSpec::with_push_mode)): `pull` (pull
//!   alone, HAVEs and the requests they bring at the tick), `eager-have`
//!   (pull, HAVEs and requests as soon as they can go) or `hybrid` (push
//!   subscriptions over eager HAVEs).
//! - `STRIMS_SIM_STREAMS=<n>` gives every swarm `n` streams, at least 1
//!   ([`with_stream_count`](crate::SwarmSpec::with_stream_count)): chunk
//!   `i` is pushed in stream `i % n`.
//! - `STRIMS_SIM_STREAM_SECS=<s>` streams every swarm for `s` seconds from
//!   its start, the run ending 10 s after the last stops: the WAN cases'
//!   minute, longer. A scenario's faults, steps and competing uploads keep
//!   their times.
//! - `STRIMS_SIM_RTC=<stack>,...` links every host as a browser does
//!   ([`Scenario::over_webrtc`]): over WebRTC, through a signalling relay,
//!   host `i` on the `i`-th stack of the list, round: `chromium`,
//!   `firefox` or `str0m` (the window model's data channels, under
//!   `STRIMS_SIM_TCP=1`), or `udp` (str0m over UDP). `off` changes
//!   nothing.
//! - `STRIMS_SIM_DISCOVERY=1` lets the hosts make their swarms' links
//!   ([`Scenario::organic`]): each keeps only its link to the signalling
//!   relay and finds the others through transfer discovery.
//! - `STRIMS_SIM_HOSTILE=<kind>:<share>[,<kind>:<share>...]` makes each
//!   share of the first swarm's viewers hostile hosts of its kind
//!   ([`Scenario::hostile`]; the kinds are `hostile.rs`'s: `free-rider`,
//!   `liar`, `whitewasher`, `hog`, `grabber`, `withholder`, `colluder`,
//!   `free-rider+echo`, `liar+echo`), the kinds interleaved; `mix` takes
//!   the dishonest mix ([`Scenario::mix`]), which the run draws: 10-20 %
//!   of the viewers, free riders and liars in turn; `off` none.
//! - `STRIMS_SIM_CHURN=<viewers/min>:<away s>` makes the first swarm's
//!   viewers leave and rejoin in turn ([`Scenario::churn`]); `off` none.
//! - `STRIMS_SIM_ARRIVALS=<profile>` sets when the first swarm's viewers
//!   arrive ([`Scenario::arrivals`], [`crate::arrivals`]), times in
//!   seconds from the stream's start:
//!   `t0` (the default) with the run; `trickle` within the stream's first
//!   third, in random order, or `trickle:<per s>` as a Poisson stream at
//!   that rate; `event:<warm %>:<at s>:<window s>` that percentage of them
//!   over the stream's first `at`, the rest within `window` from `at`.
//! - `STRIMS_SIM_DEPART=<share %>:<at s>:<window s>[:abrupt]` makes that
//!   percentage of them, among those there by `at`, leave for good within
//!   `window` from `at` ([`Scenario::departures`]): gracefully, closing
//!   their links, or with `abrupt` going silent; `off` none.
//! - `STRIMS_SIM_SEED_PIN=<case>[-qos][:<slots>]` sets the first swarm's
//!   seed's allocation ([`Scenario::seed_pin`], [`crate::seed_pin`]): a
//!   pin of `random`, `whole`, `nearest`, `oracle` or `dc-only` viewers,
//!   the slots the seed's upload pays for unless `<slots>` names them; or
//!   with `admission` or `adaptive` the policy alone; `-qos` gives the
//!   seed's uplink shares by allocation; `off` the scenario's own.
//! - `STRIMS_SIM_SEED_LINKS=all` links the first swarm's seed to every
//!   viewer it has no link with ([`Scenario::link_seed_to_viewers`]), the
//!   topology a pin's bounds take: a pin names viewers the seed links to;
//!   `off` keeps the scenario's links.
//! - `STRIMS_SIM_LOOPS=1` feeds every node's parent and decision logs to
//!   the loop analysis ([`Scenario::loops`]), which the row's `loops*`,
//!   `grafts*` and `switches*` report; it takes the tap a capture would,
//!   so it fails with `STRIMS_SIM_CAPTURE`. Changes no trace.
//! - `STRIMS_SIM_EXP=<flag>,...` sets experiment flags
//!   ([`Scenario::exp`]); `off` sets none.
//! - `STRIMS_SIM_CLOCK_SKEW=<skew>` puts every host's clock off true time
//!   ([`Scenario::clock_skew`]): `real` draws each host's offset from web
//!   telemetry's distribution ([`SkewClass`](crate::skew::SkewClass)), and
//!   `<ms>` puts every host that many milliseconds ahead or behind, up to
//!   [`MAX_SKEW`](crate::skew::MAX_SKEW) (more fails the test); `0` draws
//!   the signs alone; `off` skews and draws nothing. Unset, a scenario
//!   keeps its own, `real` unless it says otherwise.
//! - `STRIMS_SIM_SERIES=1` samples every node's transfers
//!   ([`Scenario::transfer_samples`]), which the run second by second
//!   (`Report::series_data`) and the tables' `seconds` take.
//! - `STRIMS_SIM_OUT=<dir>` writes each run's tables ([`crate::tables`],
//!   [`Report::write_tables`]) into `<dir>`, or into `<dir>/seed-<s>/`
//!   when the sweep runs more than one seed; it samples transfers as
//!   `STRIMS_SIM_SERIES=1` does. `STRIMS_SIM_BIN_MS=<ms>` (default 100)
//!   sets the width of the `bins` table's bins ([`Scenario::wire_bins`]),
//!   `STRIMS_SIM_BIN_NODES=1` adds each node's bins (`node_bins`), and
//!   `STRIMS_SIM_FORECAST=1` the nodes' forecast logs (`forecast`, large:
//!   [`Scenario::forecast_log`]), `STRIMS_SIM_SPEC=1` their request logs
//!   (`arrivals`, `haves`, `asks`, large: [`Scenario::request_logs`]).
//!   None of them changes the trace.
//! - `STRIMS_SIM_ROWS=1` prints each run's measures as one line,
//!   `ROW {json}`, on stdout ([`Report::row`]).
//! - `STRIMS_SIM_JOB=1` runs the sweep as one sim farm job
//!   (`hack/sim-farm`): the one seed `STRIMS_SIM_SEED` names, once, and its
//!   row. The farm checks that runs reproduce, so the seed isn't replayed
//!   here. A failed check goes into the row (`passed`, `failure`) and to
//!   stderr, and doesn't fail the test: a trial counts it.
//!
//! Outside a farm job, the first seed runs a second time and must give the
//! same trace hash.
//!
//! A sweep starts every run at once, the second run of the first seed
//! included, each on a thread of its own. Runs on separate threads are
//! independent: turmoil keeps a simulation's world in thread-local state
//! and runs all of its hosts on the thread that steps it, and [`run`] seeds
//! str0m's thread-local rng and counts the heap of that thread only. Once
//! every run has ended, the reports are printed and checked in seed order,
//! so a failing sweep reports its lowest failing seed.

use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::thread;
use std::time::Duration;

use strims_ppspp::PushMode;
use strims_wan::access::{Bucket, Site};
use strims_wan::live::{Encoder, Sign};

use crate::report::Report;
use crate::run::run;
use crate::skew::ClockSkew;
use crate::spec::{Arrivals, Departures, HostIx, Scenario, SendBuffer};
use crate::wan::SctpProfile;

/// A media swarm's chunk size without `STRIMS_SIM_CHUNK`: a video swarm's.
pub const MEDIA_CHUNK: usize = 4096;

/// The least bucket `STRIMS_SIM_UPLINK_BURST` takes: a full Ethernet
/// frame, which tbf needs to send one at all (`hack/testbed/run.sh`'s
/// `SEED_BURST` floor).
pub const MIN_BURST: u64 = 1514;

/// What is wrong with `site`'s bucket, if it holds less than its link's
/// largest frame (a full packet's charge, `Framing::packet(1500)`): a
/// shaper drops a packet its bucket can't cover whole, so the testbed
/// refuses such a bucket ([`strims_wan::testbed::Spec::check_bucket`]), and the
/// model would pass what the testbed drops.
fn small_bucket(site: &Site) -> Option<String> {
    let b = site.profile.bucket?;
    let frame = site.profile.framing.packet(1500) as u64;
    (b.bytes < frame).then(|| {
        format!(
            "a bucket of {} bytes is less than its link's largest frame, {frame} bytes",
            b.bytes
        )
    })
}

/// Fails on a WAN host of `spec` whose bucket is smaller than its link's
/// largest frame ([`small_bucket`]).
#[allow(clippy::panic)] // a knob's bucket the testbed refuses fails its test
fn check_buckets(spec: &Scenario) {
    for h in &spec.hosts {
        if let Some(e) = h.site.and_then(|s| small_bucket(&s)) {
            panic!(
                "{}: {e} (STRIMS_SIM_UPLINK_BURST, STRIMS_SIM_FRAMING)",
                h.name
            );
        }
    }
}

/// Seeds per sweep when `STRIMS_SIM_SEEDS` isn't set.
pub const DEFAULT_SEEDS: u64 = 8;
/// The same in a debug build.
pub const DEFAULT_SEEDS_DEBUG: u64 = 2;

/// Reads a run setting from the environment. Only the test run's
/// configuration comes from here, never the simulated nodes'.
#[allow(clippy::disallowed_methods)]
fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// The seeds a sweep runs (see the module docs).
pub fn seeds() -> Vec<u64> {
    seeds_from(1)
}

/// [`seeds`], from `from` when `STRIMS_SIM_SEED_FROM` isn't set.
fn seeds_from(from: u64) -> Vec<u64> {
    let n = if cfg!(debug_assertions) {
        DEFAULT_SEEDS_DEBUG
    } else {
        DEFAULT_SEEDS
    };
    seeds_or(n, from)
}

/// [`seeds`], with `n` seeds from `from` when `STRIMS_SIM_SEEDS` and
/// `STRIMS_SIM_SEED_FROM` aren't set.
fn seeds_or(n: u64, from: u64) -> Vec<u64> {
    if let Some(s) = env("STRIMS_SIM_SEED").and_then(|s| s.trim().parse().ok()) {
        return vec![s];
    }
    let n = env("STRIMS_SIM_SEEDS")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(n)
        .max(1);
    let from: u64 = env("STRIMS_SIM_SEED_FROM")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(from);
    (from..from.saturating_add(n)).collect()
}

fn replay(test: &str, seed: u64) -> String {
    format!(
        "replay: STRIMS_SIM_SEED={seed} cargo test -p strims-sim --release --test scenarios {test} -- --exact --nocapture --include-ignored"
    )
}

#[allow(clippy::panic)] // a failed sweep fails its test
fn fail(test: &str, seed: u64, msg: &str, trace: &[String]) -> ! {
    let tail = trace.len().saturating_sub(60);
    for l in trace.get(tail..).unwrap_or_default() {
        eprintln!("  | {l}");
    }
    panic!(
        "{test} failed with seed {seed}: {msg}\n{}",
        replay(test, seed)
    );
}

/// How a run ended: its report, turmoil's error, or its panic.
type Outcome = thread::Result<Result<Report, String>>;

/// The report of a run that ended well. A run that failed fails the sweep
/// with its seed; a panic resumes on the calling thread.
fn report_of(test: &str, seed: u64, outcome: Outcome) -> Report {
    match outcome {
        Ok(Ok(r)) => {
            if env("STRIMS_SIM_TRACE").is_some_and(|v| v != "0") {
                for l in &r.trace {
                    eprintln!("  | {l}");
                }
            }
            r
        }
        Ok(Err(e)) => fail(test, seed, &format!("simulation failed: {e}"), &[]),
        Err(p) => {
            eprintln!("{test} panicked with seed {seed}\n{}", replay(test, seed));
            resume_unwind(p)
        }
    }
}

/// A run on a thread of its own, or one whose thread couldn't start, which
/// then runs on the calling thread when joined.
enum Run<'scope> {
    Started(thread::ScopedJoinHandle<'scope, Result<Report, String>>),
    Unstarted(u64),
}

impl Run<'_> {
    /// The run's outcome once it has ended. Every run is joined, so a run's
    /// panic reaches its outcome and never fails the scope.
    fn join(self, build: &impl Fn(u64) -> Scenario) -> Outcome {
        match self {
            Run::Started(handle) => handle.join(),
            Run::Unstarted(seed) => catch_unwind(AssertUnwindSafe(|| run(build(seed), seed))),
        }
    }
}

/// A scenario setting from the run's environment, if set and not blank.
/// A value that doesn't parse fails the test, so a mistyped setting can't
/// run the default instead.
#[allow(clippy::panic)] // a mistyped setting fails its test
pub fn knob<T: std::str::FromStr>(name: &str) -> Option<T> {
    let v = env(name)?;
    let v = v.trim();
    if v.is_empty() {
        return None;
    }
    match v.parse() {
        Ok(x) => Some(x),
        Err(_) => panic!("{name}={v} doesn't parse"),
    }
}

/// `STRIMS_SIM_MEDIA` and `STRIMS_SIM_SIGN`, when both are set and the
/// policy isn't `off`.
#[allow(clippy::panic)] // a mistyped setting fails its test
fn media_knobs() -> Option<(Encoder, Sign)> {
    let encoder = env("STRIMS_SIM_MEDIA")
        .map(|s| Encoder::parse(s.trim()).unwrap_or_else(|e| panic!("STRIMS_SIM_MEDIA={e}")));
    let sign = env("STRIMS_SIM_SIGN")
        .and_then(|s| Sign::parse(s.trim()).unwrap_or_else(|e| panic!("STRIMS_SIM_SIGN={e}")));
    encoder.zip(sign)
}

/// `STRIMS_SIM_PUSH_MODE`'s modes: `pull`, `eager-have` or `hybrid`.
fn parse_push_mode(v: &str) -> Result<PushMode, String> {
    match v.trim() {
        "pull" => Ok(PushMode::Off),
        "eager-have" => Ok(PushMode::EagerHave),
        "hybrid" => Ok(PushMode::Hybrid),
        other => Err(format!("{other} is no mode: pull, eager-have or hybrid")),
    }
}

/// `STRIMS_SIM_PUSH_MODE` ([`parse_push_mode`]), if set and not blank.
#[allow(clippy::panic)] // a mistyped setting fails its test
fn push_mode_knob() -> Option<PushMode> {
    let v = env("STRIMS_SIM_PUSH_MODE").filter(|v| !v.trim().is_empty())?;
    Some(parse_push_mode(&v).unwrap_or_else(|e| panic!("STRIMS_SIM_PUSH_MODE={e}")))
}

/// `STRIMS_SIM_STREAMS`, if set and not blank: at least 1.
#[allow(clippy::panic)] // a mistyped setting fails its test
fn streams_knob() -> Option<usize> {
    let n: usize = knob("STRIMS_SIM_STREAMS")?;
    if n == 0 {
        panic!("STRIMS_SIM_STREAMS=0: at least 1");
    }
    Some(n)
}

/// `STRIMS_SIM_EXP`'s flags, comma-separated; `off` is none.
#[allow(clippy::panic)] // a mistyped flag fails its test
fn exp_knob() -> Vec<&'static str> {
    let Some(v) = env("STRIMS_SIM_EXP").filter(|v| v.trim() != "off") else {
        return Vec::new();
    };
    v.split(',')
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .map(|f| {
            crate::spec::EXP_FLAGS
                .iter()
                .copied()
                .find(|&k| k == f)
                .unwrap_or_else(|| panic!("STRIMS_SIM_EXP: {f} is no flag"))
        })
        .collect()
}

/// What `STRIMS_SIM_HOSTILE` makes of the first swarm's viewers.
#[derive(Clone, Debug, PartialEq)]
enum Hostile {
    /// Each kind's share ([`Scenario::hostile`]).
    Shares(Vec<(&'static str, f64)>),
    /// The dishonest mix ([`Scenario::mix`]).
    Mix,
}

/// `STRIMS_SIM_HOSTILE`'s forms: `<kind>:<share>[,<kind>:<share>...]`,
/// each kind once with a share from 0 to 1, or `mix`; `off` or blank,
/// none.
fn parse_hostile(v: &str) -> Result<Option<Hostile>, String> {
    let v = v.trim();
    match v {
        "off" | "" => return Ok(None),
        "mix" => return Ok(Some(Hostile::Mix)),
        _ => {}
    }
    let mut shares: Vec<(&'static str, f64)> = Vec::new();
    for part in v.split(',') {
        let (kind, share) = part
            .split_once(':')
            .ok_or_else(|| format!("{part} is no <kind>:<share>"))?;
        let kind = crate::hostile::KINDS
            .iter()
            .copied()
            .find(|&k| k == kind.trim())
            .ok_or_else(|| format!("{kind} is no kind"))?;
        if shares.iter().any(|&(k, _)| k == kind) {
            return Err(format!("{kind} is named twice"));
        }
        let share = share
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|x| (0.0..=1.0).contains(x))
            .ok_or_else(|| format!("{share} is no share from 0 to 1"))?;
        shares.push((kind, share));
    }
    Ok(Some(Hostile::Shares(shares)))
}

/// `STRIMS_SIM_HOSTILE` ([`parse_hostile`]), if set.
#[allow(clippy::panic)] // a mistyped setting fails its test
fn hostile_knob() -> Option<Hostile> {
    let v = env("STRIMS_SIM_HOSTILE")?;
    parse_hostile(&v).unwrap_or_else(|e| panic!("STRIMS_SIM_HOSTILE={v}: {e}"))
}

/// `STRIMS_SIM_CHURN=<viewers/min>:<away s>` ([`Scenario::churn`]); `off`
/// none.
#[allow(clippy::panic)] // a mistyped setting fails its test
fn churn_knob() -> Option<(f64, Duration)> {
    let v = env("STRIMS_SIM_CHURN").filter(|v| v.trim() != "off")?;
    let parse = |x: &str| {
        x.trim()
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("STRIMS_SIM_CHURN: {v} is no <viewers/min>:<away s>"))
    };
    let (rate, away) = v
        .split_once(':')
        .unwrap_or_else(|| panic!("STRIMS_SIM_CHURN: {v} is no <viewers/min>:<away s>"));
    Some((parse(rate), Duration::from_secs_f64(parse(away))))
}

/// `STRIMS_SIM_DEPART=<share>:<at>:<window>[:abrupt]`
/// ([`Scenario::departures`]); `off` none.
#[allow(clippy::panic)] // a mistyped setting fails its test
fn depart_knob() -> Option<Departures> {
    let v = env("STRIMS_SIM_DEPART").filter(|v| !matches!(v.trim(), "off" | ""))?;
    Some(
        v.parse()
            .unwrap_or_else(|e| panic!("STRIMS_SIM_DEPART={v}: {e}")),
    )
}

/// `STRIMS_SIM_CLOCK_SKEW` ([`Scenario::clock_skew`]), if set and not
/// blank.
#[allow(clippy::panic)] // a mistyped setting, or one past the cap, fails its test
fn clock_skew_knob() -> Option<ClockSkew> {
    let v = env("STRIMS_SIM_CLOCK_SKEW").filter(|v| !v.trim().is_empty())?;
    Some(
        v.parse()
            .unwrap_or_else(|e| panic!("STRIMS_SIM_CLOCK_SKEW={v}: {e}")),
    )
}

/// `STRIMS_SIM_SEED_PIN` ([`crate::seed_pin::SeedPin::parse`]), if set
/// and not `off`.
#[allow(clippy::panic)] // a mistyped setting fails its test
fn seed_pin_knob() -> Option<crate::seed_pin::SeedPin> {
    let v = env("STRIMS_SIM_SEED_PIN")?;
    crate::seed_pin::SeedPin::parse(&v).unwrap_or_else(|e| panic!("STRIMS_SIM_SEED_PIN={v}: {e}"))
}

/// Whether `STRIMS_SIM_SEED_LINKS` is `all`; unset, blank or `off` is
/// not.
#[allow(clippy::panic)] // a mistyped setting fails its test
fn seed_links_knob() -> bool {
    match env("STRIMS_SIM_SEED_LINKS").as_deref().map(str::trim) {
        None | Some("" | "off") => false,
        Some("all") => true,
        Some(v) => panic!("STRIMS_SIM_SEED_LINKS={v}: all or off"),
    }
}

/// What `STRIMS_SIM_UPLINK_BURST` gives every WAN host's uplink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UplinkBurst {
    /// No bucket, the scenario's own included.
    Off,
    /// A bucket of that many bytes, leaving at once.
    Bytes(u64),
    /// Each host its access kind's bucket ([`strims_wan::Access::bucket`]).
    Profiles,
}

/// `STRIMS_SIM_UPLINK_BURST`'s forms: `off`, `profiles` or a byte count
/// of at least [`MIN_BURST`]; blank, none.
fn parse_uplink_burst(v: &str) -> Result<Option<UplinkBurst>, String> {
    match v.trim() {
        "" => Ok(None),
        "off" => Ok(Some(UplinkBurst::Off)),
        "profiles" => Ok(Some(UplinkBurst::Profiles)),
        bytes => match bytes.parse::<u64>() {
            Ok(b) if b >= MIN_BURST => Ok(Some(UplinkBurst::Bytes(b))),
            _ => Err(format!(
                "{bytes} is no bucket: off, profiles or at least {MIN_BURST} bytes"
            )),
        },
    }
}

/// What `STRIMS_SIM_FRAMING` gives every WAN host's uplink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FramingKnob {
    Off,
    Ethernet,
    Profiles,
}

/// `STRIMS_SIM_FRAMING`, if set.
#[allow(clippy::panic)] // a mistyped setting fails its test
fn framing_knob() -> Option<FramingKnob> {
    match env("STRIMS_SIM_FRAMING").as_deref().map(str::trim) {
        None | Some("") => None,
        Some("off") => Some(FramingKnob::Off),
        Some("ethernet") => Some(FramingKnob::Ethernet),
        Some("profiles") => Some(FramingKnob::Profiles),
        Some(v) => panic!("STRIMS_SIM_FRAMING={v}: off, ethernet or profiles"),
    }
}

/// What `STRIMS_SIM_UPLINK_SLOTS` gives every WAN host's uplink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlotsKnob {
    Off,
    Profiles,
    Fixed(strims_wan::slots::Slots),
}

/// `STRIMS_SIM_UPLINK_SLOTS`'s forms: `off`, `profiles` or
/// `<min>-<max>` ms; blank, none.
fn parse_uplink_slots(v: &str) -> Result<Option<SlotsKnob>, String> {
    let ms = |x: &str| {
        x.trim()
            .parse::<f64>()
            .ok()
            .filter(|m| m.is_finite() && *m >= 0.0)
            .map(|m| strims_base::time::Duration::from_nanos((m * 1e6) as i64))
    };
    match v.trim() {
        "" => Ok(None),
        "off" => Ok(Some(SlotsKnob::Off)),
        "profiles" => Ok(Some(SlotsKnob::Profiles)),
        r => match r.split_once('-').and_then(|(a, b)| Some((ms(a)?, ms(b)?))) {
            Some((min, max)) if min <= max => {
                Ok(Some(SlotsKnob::Fixed(strims_wan::slots::Slots {
                    min,
                    max,
                })))
            }
            _ => Err(format!("{r}: off, profiles or <min>-<max> ms")),
        },
    }
}

/// `STRIMS_SIM_UPLINK_SLOTS` ([`parse_uplink_slots`]), if set.
#[allow(clippy::panic)] // a mistyped setting fails its test
fn uplink_slots_knob() -> Option<SlotsKnob> {
    let v = env("STRIMS_SIM_UPLINK_SLOTS")?;
    parse_uplink_slots(&v).unwrap_or_else(|e| panic!("STRIMS_SIM_UPLINK_SLOTS={v}: {e}"))
}

/// `STRIMS_SIM_UPLINK_BURST` ([`parse_uplink_burst`]), if set.
#[allow(clippy::panic)] // a mistyped setting fails its test
fn uplink_burst_knob() -> Option<UplinkBurst> {
    let v = env("STRIMS_SIM_UPLINK_BURST")?;
    parse_uplink_burst(&v).unwrap_or_else(|e| panic!("STRIMS_SIM_UPLINK_BURST={v}: {e}"))
}

/// `STRIMS_SIM_RTC`'s stacks; `off` is none.
#[allow(clippy::panic)] // a mistyped setting fails its test
fn rtc_knob() -> Option<Vec<Option<SctpProfile>>> {
    let v = env("STRIMS_SIM_RTC").filter(|v| v.trim() != "off")?;
    let stacks = v
        .split(',')
        .map(|s| match s.trim() {
            "chromium" => Some(SctpProfile::CHROMIUM),
            "firefox" => Some(SctpProfile::FIREFOX),
            "str0m" => Some(SctpProfile::STR0M),
            "udp" => None,
            other => panic!("STRIMS_SIM_RTC: {other} is no stack"),
        })
        .collect();
    Some(stacks)
}

/// The stream length of a scenario that takes it from the run's settings:
/// `STRIMS_SIM_MINUTES=<m>`, else `default` minutes.
pub fn stream_minutes(default: u64) -> u64 {
    env("STRIMS_SIM_MINUTES")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(default)
        .max(1)
}

/// Runs scenario `build(seed)` for every seed of [`seeds`] and the first
/// seed a second time, all at once, then checks each report with `check`
/// and compares the first seed's trace hashes. A failure panics with the
/// seed and a one-line replay command. `test` is the test function's name
/// (for the replay command and the runs' thread names).
pub fn sweep(
    test: &str,
    build: impl Fn(u64) -> Scenario + Sync,
    check: impl Fn(&Report) -> Result<(), String>,
) -> Vec<Report> {
    sweep_seeds(test, seeds(), build, check)
}

/// [`sweep`] over `n` seeds unless `STRIMS_SIM_SEEDS` says otherwise, for
/// scenarios too long for the default count.
pub fn sweep_n(
    test: &str,
    n: u64,
    build: impl Fn(u64) -> Scenario + Sync,
    check: impl Fn(&Report) -> Result<(), String>,
) -> Vec<Report> {
    sweep_seeds(test, seeds_or(n, 1), build, check)
}

/// [`sweep`] over the seeds from `from`: for a check that thresholds a
/// few samples per seed, where a seed's draw can fail it while the rate
/// it checks holds.
pub fn sweep_from(
    test: &str,
    from: u64,
    build: impl Fn(u64) -> Scenario + Sync,
    check: impl Fn(&Report) -> Result<(), String>,
) -> Vec<Report> {
    sweep_seeds(test, seeds_from(from), build, check)
}

/// [`sweep`] at `seed` alone unless `STRIMS_SIM_SEED` names another: a
/// case that reproduces a mode, at a seed it shows on.
pub fn sweep_at(
    test: &str,
    seed: u64,
    build: impl Fn(u64) -> Scenario + Sync,
    check: impl Fn(&Report) -> Result<(), String>,
) -> Vec<Report> {
    let seed = env("STRIMS_SIM_SEED")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(seed);
    sweep_seeds(test, vec![seed], build, check)
}

fn sweep_seeds(
    test: &str,
    seeds: Vec<u64>,
    build: impl Fn(u64) -> Scenario + Sync,
    check: impl Fn(&Report) -> Result<(), String>,
) -> Vec<Report> {
    let capture = env("STRIMS_SIM_CAPTURE");
    let send_buffer = env("STRIMS_SIM_SEND_BUFFER").and_then(|s| s.trim().parse::<u64>().ok());
    let known_uplink = env("STRIMS_SIM_KNOWN_UPLINK").map(|s| s.trim().to_owned());
    let uplink_burst = uplink_burst_knob();
    let framing = framing_knob();
    let uplink_slots = uplink_slots_knob();
    let rows = env("STRIMS_SIM_ROWS").is_some_and(|s| s.trim() == "1");
    let job = env("STRIMS_SIM_JOB").is_some_and(|s| s.trim() == "1");
    assert!(
        !job || seeds.len() == 1,
        "STRIMS_SIM_JOB runs one seed: set STRIMS_SIM_SEED"
    );
    let window = env("STRIMS_SIM_TCP").is_some_and(|s| s.trim() == "1");
    let series = env("STRIMS_SIM_SERIES").is_some_and(|s| s.trim() == "1");
    let out = env("STRIMS_SIM_OUT")
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from);
    let bin = Duration::from_millis(knob::<u64>("STRIMS_SIM_BIN_MS").unwrap_or(100).max(1));
    let bin_nodes = knob::<u32>("STRIMS_SIM_BIN_NODES") == Some(1);
    let forecast = knob::<u32>("STRIMS_SIM_FORECAST") == Some(1);
    let request_logs = knob::<u32>("STRIMS_SIM_SPEC") == Some(1);
    let chunk: Option<usize> = knob("STRIMS_SIM_CHUNK");
    let munro: Option<usize> = knob("STRIMS_SIM_MUNRO");
    let stream_secs: Option<u64> = knob("STRIMS_SIM_STREAM_SECS");
    let segment: Option<usize> = knob("STRIMS_SIM_TCP_SEGMENT");
    let media = media_knobs();
    let push_mode = push_mode_knob();
    let streams = streams_knob();
    let rtc = rtc_knob();
    let organic = env("STRIMS_SIM_DISCOVERY").is_some_and(|s| s.trim() == "1");
    let exp = exp_knob();
    let hostile = hostile_knob();
    let churn = churn_knob();
    let arrivals: Option<Arrivals> = knob("STRIMS_SIM_ARRIVALS");
    let departures = depart_knob();
    let clock_skew = clock_skew_knob();
    let seed_pin = seed_pin_knob();
    let seed_links = seed_links_knob();
    let loops = knob::<u32>("STRIMS_SIM_LOOPS") == Some(1);
    let with_capture = |seed: u64, replay: &str| {
        let mut spec = build(seed);
        if let Some(ms) = send_buffer
            && spec.wan.is_some()
        {
            spec.send_buffer = Some(SendBuffer {
                uplink_queue: Duration::from_millis(ms),
                window: false,
            });
        }
        if window && let Some(b) = &mut spec.send_buffer {
            b.window = true;
        }
        if series || out.is_some() {
            spec.transfer_samples = true;
        }
        if out.is_some() {
            spec.wire_bins = Some(bin);
            spec.forecast_log = forecast;
            spec.request_logs = request_logs;
        }
        if segment.is_some() {
            spec.tcp_segment = segment;
        }
        if let Some(secs) = stream_secs {
            for s in &mut spec.swarms {
                s.stop = s.start + Duration::from_secs(secs);
            }
            if let Some(stop) = spec.swarms.iter().map(|s| s.stop).max() {
                spec.duration = stop + Duration::from_secs(10);
            }
        }
        if let Some((encoder, sign)) = media {
            let c = chunk.unwrap_or(MEDIA_CHUNK);
            spec.swarms = std::mem::take(&mut spec.swarms)
                .into_iter()
                .map(|s| s.with_chunk_size(c).with_media(encoder, sign))
                .collect();
        } else if chunk.is_some() || munro.is_some() {
            spec.swarms = std::mem::take(&mut spec.swarms)
                .into_iter()
                .map(|s| {
                    let c = chunk.unwrap_or(s.options.chunk_size);
                    let m = munro.unwrap_or(s.options.chunk_size * s.options.chunks_per_signature);
                    s.with_chunks(c, m)
                        .unwrap_or_else(|e| fail(test, seed, &e, &[]))
                })
                .collect();
        }
        for s in &mut spec.swarms {
            if let Some(mode) = push_mode {
                s.options.push_mode = mode;
            }
            if let Some(n) = streams {
                s.options.stream_count = n;
            }
        }
        if let Some(burst) = uplink_burst {
            for site in spec.hosts.iter_mut().filter_map(|h| h.site.as_mut()) {
                let bucket = match burst {
                    UplinkBurst::Off => None,
                    UplinkBurst::Bytes(b) => Some(Bucket::of(b)),
                    UplinkBurst::Profiles => site.access.bucket(site.profile.up),
                };
                *site = site.with_bucket(bucket);
            }
        }
        if let Some(framing) = framing {
            for site in spec.hosts.iter_mut().filter_map(|h| h.site.as_mut()) {
                *site = site.with_framing(match framing {
                    FramingKnob::Off => strims_wan::framing::Framing::NONE,
                    FramingKnob::Ethernet => strims_wan::framing::Framing::ETHERNET,
                    FramingKnob::Profiles => site.access.framing(),
                });
            }
        }
        if uplink_burst.is_some() || framing.is_some() {
            check_buckets(&spec);
        }
        if let Some(slots) = uplink_slots {
            for site in spec.hosts.iter_mut().filter_map(|h| h.site.as_mut()) {
                *site = site.with_slots(match slots {
                    SlotsKnob::Off => None,
                    SlotsKnob::Profiles => site.access.slots(),
                    SlotsKnob::Fixed(s) => Some(s),
                });
            }
        }
        if let Some(which) = known_uplink.as_deref() {
            let seeds: Vec<HostIx> = spec.swarms.iter().map(|s| s.seed).collect();
            let (which, live) = match which.strip_suffix("-static") {
                Some(w) => (w, false),
                None => (which, true),
            };
            for (i, h) in spec.hosts.iter_mut().enumerate() {
                let seed = seeds.contains(&i);
                let told = match which {
                    "1" => true,
                    "seed" => seed,
                    "viewers" => !seed,
                    _ => false,
                };
                if told && let Some(site) = h.site {
                    if live {
                        h.upload_capacity_live = true;
                    } else {
                        // Told before it sends: what the link carries in
                        // IP bytes, at full-size packets.
                        let f = site.profile.framing;
                        let link = h
                            .up_capacity
                            .as_ref()
                            .map_or(site.profile.up, |c| c.mean_rate());
                        h.upload_capacity = Some(if f == strims_wan::framing::Framing::NONE {
                            link
                        } else {
                            link * 1500 / f.packet(1500) as u64
                        });
                    }
                }
            }
        }
        if seed_links {
            spec.link_seed_to_viewers();
        }
        if loops {
            if capture.is_some() {
                fail(
                    test,
                    seed,
                    "STRIMS_SIM_LOOPS takes the tap STRIMS_SIM_CAPTURE would",
                    &[],
                );
            }
            spec.loops = true;
        }
        if let Some(stacks) = &rtc {
            spec.over_webrtc(stacks);
        }
        if organic {
            spec.organic();
        }
        spec.exp.extend(exp.iter().copied());
        match &hostile {
            Some(Hostile::Shares(kinds)) => spec.hostile(kinds),
            Some(Hostile::Mix) => spec.mix = true,
            None => {}
        }
        if let Some((per_min, away)) = churn {
            spec.churn(per_min, away);
        }
        if let Some(a) = arrivals {
            spec.arrivals = a;
        }
        if departures.is_some() {
            spec.departures = departures;
        }
        if let Some(skew) = clock_skew {
            spec.clock_skew = skew;
        }
        if let Some(pin) = &seed_pin {
            if let Err(e) = pin.check(&spec) {
                fail(test, seed, &format!("STRIMS_SIM_SEED_PIN={pin}: {e}"), &[]);
            }
            crate::seed_pin::apply(&mut spec, pin.clone());
        }
        if let Some(dir) = &capture {
            let run = format!("seed-{seed}{replay}");
            spec.capture = Some(std::path::Path::new(dir).join(test).join(run));
        }
        spec
    };
    let first = |seed: u64| with_capture(seed, "");
    let replay = |seed: u64| with_capture(seed, "-replay");
    let (first, replay) = (&first, &replay);
    let (outcomes, again) = thread::scope(|s| {
        let start = |seed: u64, name: String, again: bool| match thread::Builder::new()
            .name(name)
            .spawn_scoped(s, move || {
                let spec = if again { replay(seed) } else { first(seed) };
                run(spec, seed)
            }) {
            Ok(handle) => Run::Started(handle),
            Err(_) => Run::Unstarted(seed),
        };
        let runs: Vec<Run<'_>> = seeds
            .iter()
            .map(|&seed| start(seed, format!("{test} seed {seed}"), false))
            .collect();
        let again = seeds
            .first()
            .filter(|_| !job)
            .map(|&seed| start(seed, format!("{test} replay"), true));
        let outcomes: Vec<Outcome> = runs.into_iter().map(|r| r.join(first)).collect();
        (outcomes, again.map(|r| r.join(replay)))
    });

    let mut reports = Vec::new();
    let mut verdicts = Vec::new();
    for (&seed, outcome) in seeds.iter().zip(outcomes) {
        let r = report_of(test, seed, outcome);
        eprintln!("{}", r.summary());
        let verdict = check(&r);
        if rows || job {
            println!(
                "ROW {}",
                r.row(test, verdict.as_ref().err().map(String::as_str))
            );
        }
        if let Some(dir) = &out {
            let dir = match seeds.len() {
                1 => dir.clone(),
                _ => dir.join(format!("seed-{seed}")),
            };
            if let Err(e) = r.write_tables(&dir, bin, bin_nodes) {
                fail(
                    test,
                    seed,
                    &format!("STRIMS_SIM_OUT: writing {}: {e}", dir.display()),
                    &[],
                );
            }
        }
        reports.push(r);
        verdicts.push(verdict);
    }
    for (r, verdict) in reports.iter().zip(verdicts) {
        if let Err(e) = verdict {
            if job {
                eprintln!(
                    "{test} failed with seed {}: {e}\n{}",
                    r.seed,
                    self::replay(test, r.seed)
                );
            } else {
                fail(test, r.seed, &e, &r.trace);
            }
        }
    }
    if let (Some(first), Some(again)) = (reports.first(), again) {
        let again = report_of(test, first.seed, again);
        if again.trace_hash != first.trace_hash {
            let at = first
                .trace
                .iter()
                .zip(&again.trace)
                .position(|(a, b)| a != b)
                .unwrap_or(first.trace.len().min(again.trace.len()));
            let msg = format!(
                "not deterministic: trace hashes {} and {} differ at line {at}: {:?} vs {:?}",
                first.trace_hash,
                again.trace_hash,
                first.trace.get(at),
                again.trace.get(at)
            );
            fail(test, first.seed, &msg, &[]);
        }
        eprintln!(
            "{test}: seed {} replayed: same trace hash {} ({} lines)",
            first.seed,
            first.trace_hash,
            first.trace.len()
        );
    }
    reports
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn push_mode_knob_takes_the_three_modes() {
        assert_eq!(parse_push_mode("pull"), Ok(PushMode::Off));
        assert_eq!(parse_push_mode(" eager-have "), Ok(PushMode::EagerHave));
        assert_eq!(parse_push_mode("hybrid"), Ok(PushMode::Hybrid));
        for bad in ["off", "Hybrid", "eager", "push", "default"] {
            assert!(parse_push_mode(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn hostile_knob_takes_kinds_shares_and_the_mix() {
        assert_eq!(parse_hostile("off"), Ok(None));
        assert_eq!(parse_hostile(" "), Ok(None));
        assert_eq!(parse_hostile("mix"), Ok(Some(Hostile::Mix)));
        assert_eq!(
            parse_hostile("liar:0.2"),
            Ok(Some(Hostile::Shares(vec![("liar", 0.2)])))
        );
        assert_eq!(
            parse_hostile("free-rider:0.1, liar : 0.05,hog:0"),
            Ok(Some(Hostile::Shares(vec![
                ("free-rider", 0.1),
                ("liar", 0.05),
                ("hog", 0.0)
            ])))
        );
        for bad in [
            "liar",
            "liar:",
            "liar:x",
            "liar:1.5",
            "liar:-0.1",
            "liar:NaN",
            "honest:0.1",
            "liar:0.1,liar:0.1",
            "liar:0.1,",
            "mix,liar:0.1",
        ] {
            assert!(parse_hostile(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn uplink_burst_knob_takes_off_profiles_and_a_frame_or_more() {
        assert_eq!(parse_uplink_burst(" "), Ok(None));
        assert_eq!(parse_uplink_burst("off"), Ok(Some(UplinkBurst::Off)));
        assert_eq!(
            parse_uplink_burst("profiles"),
            Ok(Some(UplinkBurst::Profiles))
        );
        assert_eq!(
            parse_uplink_burst(" 3028 "),
            Ok(Some(UplinkBurst::Bytes(3028)))
        );
        assert_eq!(
            parse_uplink_burst("1514"),
            Ok(Some(UplinkBurst::Bytes(MIN_BURST)))
        );
        for bad in ["0", "1513", "-3028", "3k", "on", "profile"] {
            assert!(parse_uplink_burst(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_bucket_holds_its_links_largest_frame() {
        use strims_wan::framing::Framing;
        let site = |bytes: u64, f: Framing| {
            Site::new(strims_wan::City::Chicago, strims_wan::Access::Dsl)
                .with_bucket(Some(Bucket::of(bytes)))
                .with_framing(f)
        };
        assert_eq!(small_bucket(&site(MIN_BURST, Framing::ETHERNET)), None);
        assert!(small_bucket(&site(1517, Framing::DOCSIS)).is_some());
        assert_eq!(small_bucket(&site(1518, Framing::DOCSIS)), None);
        assert!(small_bucket(&site(MIN_BURST, Framing::ADSL_PPPOE)).is_some());
        assert_eq!(small_bucket(&site(1749, Framing::ADSL_PPPOE)), None);
    }

    #[test]
    fn uplink_slots_knob_takes_off_profiles_and_a_range() {
        use strims_base::time::Duration;
        assert_eq!(parse_uplink_slots(""), Ok(None));
        assert_eq!(parse_uplink_slots("off"), Ok(Some(SlotsKnob::Off)));
        assert_eq!(
            parse_uplink_slots("profiles"),
            Ok(Some(SlotsKnob::Profiles))
        );
        assert_eq!(
            parse_uplink_slots("2-8"),
            Ok(Some(SlotsKnob::Fixed(strims_wan::slots::Slots {
                min: Duration::from_millis(2),
                max: Duration::from_millis(8),
            })))
        );
        for bad in ["8-2", "2", "a-b", "-1-2"] {
            assert!(parse_uplink_slots(bad).is_err(), "{bad}");
        }
    }
}
