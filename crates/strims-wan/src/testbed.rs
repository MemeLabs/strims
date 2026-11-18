// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The Docker testbed's access link classes (`run.sh`'s `SEED_LINK`,
//! `VIEWER_LINK` and `LINKS`), parsed once for both sides: `run.sh`
//! validates every node's spec with this crate's `testbed_link` example
//! ([`cli`]), built from its own tree, before any container starts and
//! builds its qdiscs from what that prints (`hack/testbed/links.sh`), and
//! the sims' twins take the same [`Spec`] as a site (strims-sim's
//! `link::TestbedLink`), refusing what the WAN model has no form of.
//!
//! A spec is `<class>[,<key>=<value>...]`, each key at most once:
//!
//! - `tbf`: the run's rate, bucket and peak (`SEED_BURST`,
//!   `SEED_PEAK_MBIT`), Ethernet's 14 bytes a packet
//!   ([`Framing::ETHERNET`], what a qdisc counts).
//! - `cable`: a bucket of `burst=` bytes (125 ms of the starting rate
//!   without) sent at `peak=` Mbit/s (100, a line's, which a link whose
//!   rate reaches it at its start or a step goes without), `aqm=pie`
//!   (DOCSIS-PIE, the default; the testbed's alone) or `aqm=fifo`,
//!   DOCSIS's framing, and the upstream's slots ([`CABLE_GRANTS`] unless
//!   `slot=` says).
//! - `dsl`: a bucket of 3500 bytes, `line=adsl`'s ATM cells (the default)
//!   or `line=vdsl`'s PTM ([`Framing::ADSL_PPPOE`],
//!   [`Framing::VDSL_PPPOE`]).
//! - `fiber`: the run's bucket and peak, an 18-byte frame.
//! - `trace`: delivery opportunities, `file=` in Mahimahi's format
//!   (`format=us` for tracelink's), `scale=`, `offset=` seconds into it
//!   from the stream's start ([`crate::capacity`]), Ethernet's 14
//!   bytes; on the testbed alone, `delay=<file>@<step ms>` (a one-way
//!   delay trace) and `limit=` bytes held at most.
//!
//! On any class, `slot=<min>-<max>` (or `<ms>`, or `off`) gives in
//! milliseconds the slots that send what its shaper passed (cable's grants;
//! the queue's limit counts what waits for the shaper, not for a slot), and
//! on any but a trace, `router=fq_codel|cake` (the
//! testbed's alone) an SQM home router at `pct=` percent of the rate
//! (default 95). A file's path takes letters, digits and `._/+-_` alone,
//! which go inside single quotes in `links.sh`'s shell as they are.
//! A peak given (`SEED_PEAK_MBIT` under tbf or fiber, cable's `peak=`) is
//! over the link's rate at its start and at every step, as tbf takes a
//! peakrate over its rate alone: both sides refuse one at or under it.
//! The twins refuse a router, DOCSIS-PIE, a delay trace and a byte limit,
//! and a trace link takes no rate steps on either side.

use strims_base::time::Duration;

use crate::access::{Bucket, CABLE_GRANTS, Site};
use crate::framing::{Framing, Line};
use crate::slots::Slots;

/// A link class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Tbf,
    Cable,
    Dsl,
    Fiber,
    Trace,
}

impl Class {
    pub const fn name(self) -> &'static str {
        match self {
            Class::Tbf => "tbf",
            Class::Cable => "cable",
            Class::Dsl => "dsl",
            Class::Fiber => "fiber",
            Class::Trace => "trace",
        }
    }
}

/// A home router's AQM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Router {
    FqCodel,
    Cake,
}

/// A trace link's schedule and the testbed's extras.
#[derive(Clone, Debug, PartialEq)]
pub struct Trace {
    /// The trace file, in Mahimahi's format or (`us`) tracelink's.
    pub file: String,
    pub us: bool,
    /// What every opportunity's bytes are multiplied by.
    pub scale: f64,
    /// Seconds into the trace at the stream's start.
    pub offset: f64,
    /// A one-way delay trace and its step in ms.
    pub delay: Option<(String, f64)>,
    /// The bytes the link holds at most.
    pub limit: Option<u64>,
}

/// A parsed spec (see the module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub class: Class,
    burst: Option<u64>,
    peak: Option<f64>,
    pie: bool,
    vdsl: bool,
    /// `None`: the class's own slots.
    slot: Option<Option<Slots>>,
    router: Option<Router>,
    pct: f64,
    trace: Option<Trace>,
}

fn path(spec: &str, key: &str, v: &str) -> Result<String, String> {
    let ok = !v.is_empty()
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._/+-_".contains(c));
    if ok {
        Ok(v.to_owned())
    } else {
        Err(format!(
            "link {spec}: {key}={v}: a path of letters, digits and ._/+-_"
        ))
    }
}

fn slots(v: &str) -> Result<Option<Slots>, String> {
    if v == "off" {
        return Ok(None);
    }
    let ms = |x: &str| {
        x.parse::<f64>()
            .ok()
            .filter(|m| m.is_finite() && *m >= 0.0)
            .map(|m| Duration::from_nanos((m * 1e6).round() as i64))
            .ok_or_else(|| format!("slot={v}: <min>-<max> or <ms>, in ms, or off"))
    };
    let (min, max) = match v.split_once('-') {
        Some((a, b)) => (ms(a)?, ms(b)?),
        None => (ms(v)?, ms(v)?),
    };
    if max < min {
        return Err(format!("slot={v}: max under min"));
    }
    Ok(Some(Slots { min, max }))
}

impl Spec {
    /// Parses `spec`, refusing what neither side would build.
    pub fn parse(spec: &str) -> Result<Spec, String> {
        let mut parts = spec.split(',');
        let class = match parts.next().unwrap_or_default() {
            "tbf" => Class::Tbf,
            "cable" => Class::Cable,
            "dsl" => Class::Dsl,
            "fiber" => Class::Fiber,
            "trace" => Class::Trace,
            c => return Err(format!("link {spec}: {c}: tbf, cable, dsl, fiber or trace")),
        };
        let mut kv: Vec<(&str, &str)> = Vec::new();
        for p in parts {
            let (k, v) = p
                .split_once('=')
                .ok_or_else(|| format!("link {spec}: {p} is no key=value"))?;
            if kv.iter().any(|(x, _)| *x == k) {
                return Err(format!("link {spec}: {k} twice"));
            }
            let ok = match k {
                "slot" => true,
                "router" | "pct" => class != Class::Trace,
                "burst" | "peak" | "aqm" => class == Class::Cable,
                "line" => class == Class::Dsl,
                "file" | "format" | "scale" | "offset" | "delay" | "limit" => class == Class::Trace,
                _ => return Err(format!("link {spec}: no key {k}")),
            };
            if !ok {
                return Err(format!("link {spec}: {k} isn't {}'s", class.name()));
            }
            kv.push((k, v));
        }
        let get = |key: &str| kv.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
        let num = |key: &str, ok: fn(f64) -> bool| -> Result<Option<f64>, String> {
            get(key)
                .map(|v| {
                    v.parse::<f64>()
                        .ok()
                        .filter(|x| x.is_finite() && ok(*x))
                        .ok_or_else(|| format!("link {spec}: {key}={v}"))
                })
                .transpose()
        };
        let whole = |key: &str| -> Result<Option<u64>, String> {
            get(key)
                .map(|v| {
                    v.parse::<u64>()
                        .ok()
                        .filter(|&x| x > 0)
                        .ok_or_else(|| format!("link {spec}: {key}={v}: a byte count"))
                })
                .transpose()
        };
        let router = match get("router") {
            None => None,
            Some("fq_codel") => Some(Router::FqCodel),
            Some("cake") => Some(Router::Cake),
            Some(r) => return Err(format!("link {spec}: router={r}: fq_codel or cake")),
        };
        if get("pct").is_some() && router.is_none() {
            return Err(format!("link {spec}: pct= needs router="));
        }
        let trace = if class == Class::Trace {
            let file = path(
                spec,
                "file",
                get("file").ok_or_else(|| format!("link {spec}: trace needs file="))?,
            )?;
            let delay = get("delay")
                .map(|d| {
                    let (f, step) = d
                        .rsplit_once('@')
                        .ok_or_else(|| format!("link {spec}: delay=<file>@<step ms>"))?;
                    let step = step
                        .parse::<f64>()
                        .ok()
                        .filter(|s| s.is_finite() && *s > 0.0)
                        .ok_or_else(|| format!("link {spec}: delay={d}: a step in ms"))?;
                    Ok::<_, String>((path(spec, "delay", f)?, step))
                })
                .transpose()?;
            Some(Trace {
                file,
                us: match get("format").unwrap_or("mm") {
                    "mm" => false,
                    "us" => true,
                    f => return Err(format!("link {spec}: format={f}: mm or us")),
                },
                scale: num("scale", |x| x > 0.0)?.unwrap_or(1.0),
                offset: num("offset", |x| x >= 0.0)?.unwrap_or(0.0),
                delay,
                limit: whole("limit")?,
            })
        } else {
            None
        };
        Ok(Spec {
            class,
            burst: whole("burst")?,
            peak: num("peak", |x| x > 0.0)?,
            pie: match get("aqm").unwrap_or("pie") {
                "pie" => true,
                "fifo" => false,
                a => return Err(format!("link {spec}: aqm={a}: pie or fifo")),
            },
            vdsl: match get("line").unwrap_or("adsl") {
                "adsl" => false,
                "vdsl" => true,
                l => return Err(format!("link {spec}: line={l}: adsl or vdsl")),
            },
            slot: get("slot")
                .map(slots)
                .transpose()
                .map_err(|e| format!("link {spec}: {e}"))?,
            router,
            pct: num("pct", |x| x > 0.0 && x <= 100.0)?.unwrap_or(95.0),
            trace,
        })
    }

    /// A home router's AQM, in place of the modem's shaper and queue.
    pub fn router(&self) -> Option<Router> {
        self.router
    }

    /// Whether a cable link's queue is DOCSIS-PIE (`aqm=pie`).
    pub fn docsis_pie(&self) -> bool {
        self.class == Class::Cable && self.pie
    }

    /// A trace link's schedule and extras.
    pub fn trace(&self) -> Option<&Trace> {
        self.trace.as_ref()
    }

    /// What the link charges a packet past its IP bytes.
    pub fn framing(&self) -> Framing {
        match self.class {
            Class::Tbf | Class::Trace => Framing::ETHERNET,
            Class::Cable => Framing::DOCSIS,
            Class::Dsl if self.vdsl => Framing::VDSL_PPPOE,
            Class::Dsl => Framing::ADSL_PPPOE,
            Class::Fiber => Framing::bytes(18, 0),
        }
    }

    /// The link's slots.
    pub fn slots(&self) -> Option<Slots> {
        match self.slot {
            Some(s) => s,
            None if self.class == Class::Cable => Some(CABLE_GRANTS),
            None => None,
        }
    }

    /// The device whose root is the link's modem on the testbed: ifbm for
    /// a link with slots, eth0 otherwise (`LINK_DEV`).
    pub fn modem_dev(&self) -> &'static str {
        if self.slots().is_some() {
            "ifbm"
        } else {
            "eth0"
        }
    }

    /// The link's largest frame, what it charges a full packet (1500 IP
    /// bytes): the least a bucket may hold, since a shaper drops a packet
    /// it can't cover whole.
    pub fn max_frame(&self) -> u64 {
        self.framing().packet(1500) as u64
    }

    /// Fails on a bucket smaller than the link's largest frame
    /// ([`Spec::max_frame`]).
    pub fn check_bucket(&self, bytes: u64) -> Result<(), String> {
        if bytes < self.max_frame() {
            return Err(format!(
                "a {} link's bucket holds at least its largest frame, {} bytes, not {bytes}",
                self.class.name(),
                self.max_frame()
            ));
        }
        Ok(())
    }

    /// The bucket the link's shaper has, its rate having started at
    /// `base` bytes per second and reaching `top` at its highest (its
    /// start's or a step's), given the run's `run` bucket (tbf's and
    /// fiber's); fails on a peak given at or under `top` (the run's, or
    /// cable's `peak=`). Cable's default peak is a line's, which a link
    /// whose rate reaches it goes without, its bucket leaving at once.
    pub fn bucket(&self, base: u64, top: u64, run: Bucket) -> Result<Bucket, String> {
        let given = |b: Bucket| match b.peak {
            Some(p) => check_peak(p, top).map(|()| b),
            None => Ok(b),
        };
        match self.class {
            Class::Cable => {
                let bytes = self.burst.unwrap_or((base as f64 / 8.0).round() as u64);
                match self.peak {
                    Some(p) => given(Bucket {
                        bytes,
                        peak: Some(mbit(p)),
                    }),
                    None => Ok(Bucket {
                        bytes,
                        peak: Some(CABLE_LINE).filter(|&p| p > top),
                    }),
                }
            }
            Class::Dsl => Ok(Bucket::of(3500)),
            Class::Tbf | Class::Fiber => given(run),
            Class::Trace => Ok(run),
        }
    }

    /// `site` shaped as the link is: its framing, slots and bucket, the
    /// run's `bucket` where the class takes it (`None`: none), its rate
    /// reaching `top` bytes per second at its highest (its start's or a
    /// step's); fails as the testbed does on a bucket smaller than the
    /// link's largest frame or a peak at or under `top` ([`Spec::bucket`],
    /// which also drops cable's default peak at or under it). A trace's
    /// schedule isn't the site's: its uplink takes it apart
    /// ([`crate::Network::set_up_capacity`]).
    pub fn shape(&self, site: Site, bucket: Option<Bucket>, top: u64) -> Result<Site, String> {
        let up = site.profile.up;
        let bucket = match self.class {
            Class::Tbf | Class::Fiber => bucket.map(|b| self.bucket(up, top, b)).transpose()?,
            Class::Trace => bucket,
            Class::Cable | Class::Dsl => Some(self.bucket(up, top, Bucket::of(0))?),
        };
        if let Some(b) = bucket
            && self.class != Class::Trace
        {
            self.check_bucket(b.bytes)?;
        }
        Ok(site
            .with_framing(self.framing())
            .with_slots(self.slots())
            .with_bucket(bucket))
    }
}

/// Cable's default peak, a line's: four bonded DOCSIS 3.0 upstream
/// channels ([`crate::access::CABLE_PEAK`]).
const CABLE_LINE: u64 = crate::access::CABLE_PEAK;

/// Fails on a bucket's peak, in bytes per second, at or under `top`, the
/// highest rate its link runs at (its start's or a step's): tbf takes a
/// peakrate over its rate alone, and a line's own rate is over what its
/// shaper passes.
pub fn check_peak(peak: u64, top: u64) -> Result<(), String> {
    if peak <= top {
        return Err(format!(
            "a peak of {} Mbit/s isn't over the link's rate, {} Mbit/s at its highest (its start's or a step's)",
            peak as f64 / 125_000.0,
            top as f64 / 125_000.0
        ));
    }
    Ok(())
}

fn mbit(m: f64) -> u64 {
    (m * 125_000.0) as u64
}

/// A node's settings in a run, for [`Spec::shell`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Run {
    /// The link's rate now, the rate it started at, and its highest (its
    /// start's or a step's), in Mbit/s.
    pub up_mbit: f64,
    pub base_mbit: f64,
    pub top_mbit: f64,
    /// The run's bucket and peak (`SEED_BURST`, `SEED_PEAK_MBIT`).
    pub burst: u64,
    pub peak_mbit: Option<f64>,
    /// The queue's byte limit (`run.sh`'s rate over `QUEUE_MS`).
    pub limit: u64,
}

fn kbit(mbit: f64) -> String {
    format!("{:.0}", mbit * 1000.0)
}

impl Spec {
    /// What `links.sh` builds the link from, as shell assignments:
    /// `LINK_CLASS`; `LINK_STAB`, tc's size table for the class's
    /// framing over the 14 bytes of Ethernet header a qdisc counts;
    /// `LINK_SHAPER`, the shaper qdisc (tbf, a peak's bucket two of the
    /// class's largest frames, or the router's); `LINK_CHILD`,
    /// the qdisc that holds its queue, if not tbf's own; `LINK_SLOT`,
    /// netem's slot argument; `LINK_DEV`, the device whose root is the
    /// modem (eth0, or ifbm for a link with slots, ahead of which eth0's
    /// root hands the modem what the delay returns and holds what comes
    /// back for its slot); `LINK_TRACE`, tracelink's arguments (its trace
    /// at `/links/trace`, a delay trace at `/links/delay`: two files may
    /// share a name); and `LINK_FILES`, the host files to mount there, as
    /// `<host path>:<path>`. Fails on a bucket the shaper can't take: one under the
    /// class's largest frame ([`Spec::check_bucket`]), or a peak at or
    /// under the link's highest rate ([`check_peak`]).
    pub fn shell(&self, run: &Run) -> Result<String, String> {
        let f = self.framing();
        let stab = if f == Framing::ETHERNET {
            String::new()
        } else {
            let mut s = format!("stab overhead {}", i32::from(f.overhead) - 14);
            if f.mpu > 0 {
                s += &format!(" mpu {}", f.mpu);
            }
            if f.line == Line::Atm {
                s += " linklayer atm";
            }
            // A floor or cells need a table of sizes, which charges a byte
            // at a time only with 2048 entries over an mtu under 2048 (at
            // 2048 over 2048 it rounds up to 2 bytes, 499 to 500); an
            // overhead alone needs none.
            if f.mpu > 0 || f.line == Line::Atm {
                s += " mtu 2047 tsize 2048";
            }
            s
        };
        let line = if self.class == Class::Dsl && self.vdsl {
            64.0 / 65.0
        } else {
            1.0
        };
        // The modem's shaper, which a router or a trace replaces.
        let (bytes, peak) = if self.router.is_none() && self.class != Class::Trace {
            let bucket = self.bucket(
                mbit(run.base_mbit),
                mbit(run.top_mbit),
                Bucket {
                    bytes: run.burst,
                    peak: run.peak_mbit.map(mbit),
                },
            )?;
            self.check_bucket(bucket.bytes)?;
            let peak = match self.class {
                Class::Cable => bucket.peak.map(|p| p as f64 / 125_000.0),
                Class::Tbf | Class::Fiber => run.peak_mbit,
                Class::Dsl | Class::Trace => None,
            };
            (bucket.bytes, peak)
        } else {
            (run.burst, None)
        };
        let (shaper, child) = match self.router {
            None if self.class == Class::Trace => (String::new(), String::new()),
            Some(router) => {
                // cake's pppoe-ptm takes PTM's byte in 65 itself, a tbf's
                // size table doesn't.
                let code = if router == Router::Cake { 1.0 } else { line };
                let rate = format!("{:.0}", run.up_mbit * 1000.0 * code * self.pct / 100.0);
                match router {
                    Router::Cake => (
                        format!("cake bandwidth {rate}kbit {}", cake_overhead(self)),
                        String::new(),
                    ),
                    Router::FqCodel => {
                        let r: u64 = rate.parse().unwrap_or(0);
                        (
                            format!(
                                "tbf rate {rate}kbit burst {} limit {}",
                                (r / 8).max(1749),
                                run.limit
                            ),
                            "fq_codel".to_owned(),
                        )
                    }
                }
            }
            None => (
                format!(
                    "tbf rate {}kbit burst {} limit {}{}",
                    kbit(run.up_mbit * line),
                    bytes,
                    run.limit,
                    peak.map_or(String::new(), |p| format!(
                        " peakrate {p}mbit mtu {}",
                        2 * self.max_frame()
                    ))
                ),
                if self.class == Class::Cable && self.pie {
                    // The modem's buffer, which DOCSIS's Buffer Control leaves
                    // to the operator (CableLabs recommends 50 ms of the
                    // rate, vendors 250 at most), is the run's queue, as a
                    // FIFO's is: pie counts packets, so as many full frames
                    // as it holds.
                    format!(
                        "pie limit {} target 10ms tupdate 16ms",
                        run.limit.div_ceil(self.max_frame())
                    )
                } else {
                    String::new()
                },
            ),
        };
        let stab = if self.router == Some(Router::Cake) {
            String::new()
        } else {
            stab
        };
        let slot = self.slots().map_or(String::new(), |s| s.netem());
        let (trace, files) = match &self.trace {
            Some(t) => {
                let mut a = format!(
                    "--trace /links/trace --format {} --scale {} --offset-s {} --limit {}",
                    if t.us { "us" } else { "mm" },
                    t.scale,
                    t.offset,
                    t.limit.unwrap_or(run.limit)
                );
                let mut files = format!("{}:/links/trace", t.file);
                if let Some((d, step)) = &t.delay {
                    a += &format!(" --delay /links/delay --delay-step-ms {step}");
                    files += &format!(" {d}:/links/delay");
                }
                (a, files)
            }
            None => (String::new(), String::new()),
        };
        let sh = [
            ("LINK_CLASS", self.class.name().to_owned()),
            ("LINK_STAB", stab),
            ("LINK_SHAPER", shaper),
            ("LINK_CHILD", child),
            ("LINK_SLOT", slot),
            ("LINK_DEV", self.modem_dev().to_owned()),
            ("LINK_TRACE", trace),
            ("LINK_FILES", files),
        ]
        .iter()
        .map(|(k, v)| format!("{k}='{v}'\n"))
        .collect();
        Ok(sh)
    }
}

/// cake's overhead keyword for the class's framing: cake takes it over
/// the IP packet, so no size table goes with it.
fn cake_overhead(spec: &Spec) -> &'static str {
    match spec.class {
        Class::Cable => "docsis",
        Class::Dsl if spec.vdsl => "pppoe-ptm",
        Class::Dsl => "pppoe-llcsnap",
        Class::Fiber => "overhead 18",
        Class::Tbf | Class::Trace => "overhead 14",
    }
}

/// The `testbed_link` example's arguments:
/// `--link <spec> --up-mbit <m> --burst <bytes> --limit <bytes>
/// [--peak-mbit <m>] [--base-mbit <m>] [--top-mbit <m>]` prints
/// [`Spec::shell`] for a node whose link runs at `--up-mbit`, having
/// started at `--base-mbit` (`--up-mbit` without) and reaching
/// `--top-mbit` at its highest, its start's or a step's (the higher of the
/// two without), or fails with what is wrong.
pub fn cli(args: &[String]) -> Result<String, String> {
    let mut it = args.iter();
    let (mut spec, mut up, mut base, mut top, mut burst, mut limit, mut peak) =
        (None, None, None, None, None, None, None);
    while let Some(a) = it.next() {
        let mut value = || {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{a} needs a value"))
        };
        let pos = |v: String, a: &str| {
            v.parse::<f64>()
                .ok()
                .filter(|x| x.is_finite() && *x > 0.0)
                .ok_or_else(|| format!("{a} {v}: a positive number"))
        };
        match a.as_str() {
            "--link" => spec = Some(value()?),
            "--up-mbit" => up = Some(pos(value()?, a)?),
            "--base-mbit" => base = Some(pos(value()?, a)?),
            "--top-mbit" => top = Some(pos(value()?, a)?),
            "--burst" => burst = Some(pos(value()?, a)? as u64),
            "--limit" => limit = Some(pos(value()?, a)? as u64),
            "--peak-mbit" => peak = Some(pos(value()?, a)?),
            _ => return Err(format!("--link: unknown {a}")),
        }
    }
    let (Some(spec), Some(up), Some(burst), Some(limit)) = (spec, up, burst, limit) else {
        return Err("--link <spec> --up-mbit <m> --burst <bytes> --limit <bytes> [--peak-mbit <m>] [--base-mbit <m>] [--top-mbit <m>]".to_owned());
    };
    let s = Spec::parse(&spec)?;
    let base = base.unwrap_or(up);
    let top = top.unwrap_or(up.max(base));
    if top < up.max(base) {
        return Err(format!("--top-mbit {top}: under --up-mbit or --base-mbit"));
    }
    s.shell(&Run {
        up_mbit: up,
        base_mbit: base,
        top_mbit: top,
        burst,
        peak_mbit: peak,
        limit,
    })
    .map_err(|e| format!("link {spec}: {e}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn arg(spec: &str, burst: &str) -> Result<String, String> {
        cli(&[
            "--link",
            spec,
            "--up-mbit",
            "20",
            "--burst",
            burst,
            "--limit",
            "1",
        ]
        .map(String::from))
    }

    #[test]
    fn slots_take_a_range_a_length_or_off() {
        let ms = Duration::from_millis;
        let s = |v: &str| Spec::parse(&format!("fiber,slot={v}")).map(|s| s.slots());
        assert_eq!(
            s("1-4"),
            Ok(Some(Slots {
                min: ms(1),
                max: ms(4)
            }))
        );
        assert_eq!(
            s("5"),
            Ok(Some(Slots {
                min: ms(5),
                max: ms(5)
            }))
        );
        assert_eq!(s("off"), Ok(None));
        assert_eq!(Spec::parse("cable").unwrap().slots(), Some(CABLE_GRANTS));
    }

    #[test]
    fn both_sides_refuse_one_set_of_specs() {
        for spec in [
            "token",
            "tbf,peak=10",
            "tbf,pct=90",
            "tbf,slot=",
            "fiber,slot=4-1",
            "fiber,slot=x",
            "cable,burst=abc",
            "cable,burst=0",
            "cable,burst=10,burst=20",
            "cable,aqm=red",
            "dsl,line=sdsl",
            "trace",
            "trace,file=a b",
            "trace,file=a:b",
            "trace,file=a'b",
            "trace,file=$x",
            "trace,file=x,offset=-5",
            "trace,file=x,scale=0",
            "trace,file=x,router=cake",
            "trace,file=x,delay=y",
            "tbf,router=red",
            "tbf,router=cake,pct=0",
            "tbf,nokey=1",
        ] {
            assert!(Spec::parse(spec).is_err(), "{spec}");
        }
    }

    #[test]
    fn a_bucket_holds_the_largest_frame() {
        // Fibre's and cable's frames are 1518 bytes, a tbf's 1514.
        assert!(arg("fiber", "1517").is_err());
        assert!(arg("fiber", "1518").is_ok());
        assert!(arg("tbf", "1514").is_ok());
        assert!(arg("cable,burst=1517", "3028").is_err());
        assert!(arg("cable,burst=x", "3028").is_err());
    }

    /// `cli` at `up` Mbit/s, started at 10 and reaching `top`.
    fn at(spec: &str, up: &str, top: &str, peak: Option<&str>) -> Result<String, String> {
        let mut a = vec![
            "--link",
            spec,
            "--up-mbit",
            up,
            "--base-mbit",
            "10",
            "--top-mbit",
            top,
            "--burst",
            "3028",
            "--limit",
            "100000",
        ];
        if let Some(p) = peak {
            a.extend(["--peak-mbit", p]);
        }
        cli(&a.into_iter().map(String::from).collect::<Vec<_>>())
    }

    #[test]
    fn a_peak_is_over_every_rate_the_link_runs_at() {
        // The run's peak, under tbf and fiber.
        for class in ["tbf", "fiber"] {
            assert!(at(class, "10", "10", Some("5")).is_err(), "{class}");
            assert!(at(class, "10", "10", Some("10")).is_err(), "{class}");
            assert!(at(class, "10", "10", Some("40")).is_ok(), "{class}");
            // A step to 40 reaches it; one to 30 doesn't.
            assert!(at(class, "10", "40", Some("40")).is_err(), "{class}");
            let step = at(class, "30", "30", Some("40")).unwrap();
            assert!(step.contains("rate 30000kbit"), "{step}");
            assert!(step.contains("peakrate 40mbit"), "{step}");
        }
        // Cable's own: refused at or under a rate the link runs at.
        assert!(at("cable,peak=15", "10", "20", None).is_err());
        assert!(at("cable,peak=10", "20", "20", None).is_err());
        assert!(
            at("cable,peak=15", "10", "10", None)
                .unwrap()
                .contains("peakrate 15mbit")
        );
        // Cable's default, a line's 100 Mbit/s: none once a rate reaches it,
        // at the start and at every step alike.
        assert!(
            at("cable", "50", "50", None)
                .unwrap()
                .contains("peakrate 100mbit")
        );
        for up in ["50", "120"] {
            let sh = at("cable", up, "120", None).unwrap();
            assert!(!sh.contains("peakrate"), "{sh}");
        }
        assert!(at("cable", "200", "200", None).is_ok());
        // A router or a trace takes no peak.
        assert!(at("tbf,router=cake", "10", "10", Some("5")).is_ok());
        assert!(at("tbf", "20", "10", None).is_err());
    }

    #[test]
    fn the_testbed_builds_the_class() {
        let run = Run {
            up_mbit: 10.0,
            base_mbit: 10.0,
            top_mbit: 10.0,
            burst: 3028,
            peak_mbit: None,
            limit: 125_000,
        };
        let sh = |spec: &str, run: &Run| Spec::parse(spec).unwrap().shell(run).unwrap();
        assert_eq!(
            sh("tbf", &run),
            "LINK_CLASS='tbf'\nLINK_STAB=''\nLINK_SHAPER='tbf rate 10000kbit burst 3028 limit 125000'\nLINK_CHILD=''\nLINK_SLOT=''\nLINK_DEV='eth0'\nLINK_TRACE=''\nLINK_FILES=''\n"
        );
        let cable = sh("cable", &run);
        assert!(
            cable.contains("LINK_STAB='stab overhead 4 mpu 64 mtu 2047 tsize 2048'"),
            "{cable}"
        );
        assert!(sh("fiber", &run).contains("LINK_STAB='stab overhead 4'"));
        assert!(sh("dsl,line=vdsl", &run).contains("LINK_STAB='stab overhead 16'"));
        assert!(
            sh("dsl", &run)
                .contains("LINK_STAB='stab overhead 26 linklayer atm mtu 2047 tsize 2048'")
        );
        assert!(cable.contains("LINK_SHAPER='tbf rate 10000kbit burst 156250 limit 125000 peakrate 100mbit mtu 3036'"), "{cable}");
        assert!(
            cable.contains("LINK_CHILD='pie limit 83 target 10ms tupdate 16ms'"),
            "{cable}"
        );
        assert!(cable.contains("LINK_SLOT='slot 2000us 8000us'"), "{cable}");
        // The modem's device: ifbm with slots, cable's by default.
        assert!(cable.contains("LINK_DEV='ifbm'"), "{cable}");
        assert!(sh("cable,slot=off", &run).contains("LINK_DEV='eth0'"));
        assert!(sh("fiber,slot=1-3", &run).contains("LINK_DEV='ifbm'"));
        assert!(sh("trace,file=x,slot=2", &run).contains("LINK_DEV='ifbm'"));
        assert!(
            sh("dsl,line=vdsl,router=cake", &run)
                .contains("LINK_SHAPER='cake bandwidth 9500kbit pppoe-ptm'")
        );
        assert!(sh("dsl,line=vdsl,router=fq_codel", &run).contains("tbf rate 9354kbit"));
        // A peak's bucket holds two of the class's frames.
        let peaked = Run {
            peak_mbit: Some(40.0),
            ..run
        };
        assert!(sh("tbf", &peaked).contains("peakrate 40mbit mtu 3028'"));
        assert!(sh("fiber", &peaked).contains("peakrate 40mbit mtu 3036'"));
        // A step keeps the bucket of the rate it started at.
        let step = sh(
            "cable",
            &Run {
                up_mbit: 5.0,
                ..run
            },
        );
        assert!(step.contains("burst 156250"), "{step}");
        let trace = sh("trace,file=/t/x.up,delay=/t/d@10,scale=0.5", &run);
        assert!(trace.contains("LINK_TRACE='--trace /links/trace --format mm --scale 0.5 --offset-s 0 --limit 125000 --delay /links/delay --delay-step-ms 10'"), "{trace}");
        assert!(
            trace.contains("LINK_FILES='/t/x.up:/links/trace /t/d:/links/delay'"),
            "{trace}"
        );
        assert!(sh("trace,file=/t/a_b.up", &run).contains("LINK_FILES='/t/a_b.up:/links/trace'"));
        // A trace and a delay trace of one name mount apart.
        let same = sh("trace,file=/a/x,delay=/b/x@10", &run);
        assert!(
            same.contains("LINK_FILES='/a/x:/links/trace /b/x:/links/delay'"),
            "{same}"
        );
    }
}
