// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The Docker testbed's access link classes (`hack/testbed/links.sh`, its
//! `run.sh`'s `SEED_LINK`, `VIEWER_LINK` and `LINKS`) as the WAN model's
//! sites take them, so a testbed run and its twin shape their uplinks
//! alike.
//!
//! A spec is `<class>[,<key>=<value>...]`:
//!
//! - `tbf`: the uplink's rate, its bucket and peak as the run gives them,
//!   Ethernet's 14 bytes a packet ([`Framing::ETHERNET`], what a qdisc
//!   counts).
//! - `cable`: a bucket of `burst=` bytes (125 ms of the starting rate
//!   without) at `peak=` Mbit/s (100), DOCSIS's framing, and the upstream's
//!   slots, `slot=<min>-<max>` ms ([`CABLE_GRANTS`] without), ahead of the
//!   bucket.
//! - `dsl`: a bucket of 3500 bytes, `line=adsl`'s ATM cells or
//!   `line=vdsl`'s PTM ([`Framing::ADSL_PPPOE`], [`Framing::VDSL_PPPOE`]).
//! - `fiber`: the run's bucket, an 18-byte frame.
//! - `trace`: delivery opportunities, `file=` in Mahimahi's format
//!   (`format=us` for tracelink's), `scale=`, `offset=` seconds into it,
//!   from the stream's start ([`strims_wan::capacity`]), Ethernet's 14
//!   bytes.
//!
//! `slot=` gives any class slots. What the model has no form of, it
//! refuses: a home router's AQM (`router=`), DOCSIS-PIE (`aqm=pie`, cable's
//! default on the testbed: give `aqm=fifo`), a trace's delay trace
//! (`delay=`); `limit=` is the queue the run gives.

use std::sync::Arc;
use std::time::Duration as StdDuration;

use strims_base::time::Duration;
use strims_wan::access::CABLE_GRANTS;
use strims_wan::capacity::Capacity;
use strims_wan::framing::Framing;
use strims_wan::slots::Slots;
use strims_wan::{Bucket, Site};

/// A testbed link spec, parsed (see the module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct TestbedLink {
    pub class: String,
    pub framing: Framing,
    /// The bucket a class sets, `None` where the run's own applies.
    bucket: Option<Bucket>,
    /// Whether the class takes the run's bucket (tbf, fiber).
    runs_bucket: bool,
    pub slots: Option<Slots>,
    trace: Option<Trace>,
}

#[derive(Clone, Debug, PartialEq)]
struct Trace {
    file: String,
    us: bool,
    scale: f64,
    offset: f64,
}

fn mbit(m: f64) -> u64 {
    (m * 125_000.0) as u64
}

fn slots(v: &str) -> Result<Option<Slots>, String> {
    if v == "off" {
        return Ok(None);
    }
    let (a, b) = v
        .split_once('-')
        .ok_or_else(|| format!("slot={v}: <min>-<max> ms"))?;
    let ms = |x: &str| {
        x.parse::<f64>()
            .ok()
            .filter(|m| m.is_finite() && *m >= 0.0)
            .map(|m| Duration::from_nanos((m * 1e6) as i64))
            .ok_or_else(|| format!("slot={v}: <min>-<max> ms"))
    };
    let (min, max) = (ms(a)?, ms(b)?);
    if max < min {
        return Err(format!("slot={v}: max under min"));
    }
    Ok(Some(Slots { min, max }))
}

impl TestbedLink {
    /// Parses `spec` for an uplink of `up` bytes per second.
    pub fn parse(spec: &str, up: u64) -> Result<TestbedLink, String> {
        let mut parts = spec.split(',');
        let class = parts.next().unwrap_or_default().trim().to_owned();
        let mut kv = Vec::new();
        for p in parts {
            let (k, v) = p
                .split_once('=')
                .ok_or_else(|| format!("link {spec}: {p} is no key=value"))?;
            kv.push((k.trim(), v.trim()));
        }
        let get = |key: &str| kv.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
        let num = |key: &str| -> Result<Option<f64>, String> {
            get(key)
                .map(|v| {
                    v.parse::<f64>()
                        .ok()
                        .filter(|x| x.is_finite() && *x >= 0.0)
                        .ok_or_else(|| format!("link {spec}: {key}={v}"))
                })
                .transpose()
        };
        for (k, _) in &kv {
            let ok = match *k {
                "slot" | "limit" => true,
                "burst" | "peak" | "aqm" => class == "cable",
                "line" => class == "dsl",
                "file" | "format" | "scale" | "offset" => class == "trace",
                "router" => return Err(format!("link {spec}: the model has no home router AQM")),
                "delay" => return Err(format!("link {spec}: the model has no delay trace")),
                _ => false,
            };
            if !ok {
                return Err(format!("link {spec}: {k} isn't {class}'s"));
            }
        }
        let mut link = TestbedLink {
            class: class.clone(),
            framing: Framing::ETHERNET,
            bucket: None,
            runs_bucket: false,
            slots: None,
            trace: None,
        };
        match class.as_str() {
            "tbf" => link.runs_bucket = true,
            "fiber" => {
                link.runs_bucket = true;
                link.framing = Framing::bytes(18, 0);
            }
            "cable" => {
                if get("aqm").unwrap_or("pie") != "fifo" {
                    return Err(format!(
                        "link {spec}: the model has no DOCSIS-PIE (give aqm=fifo)"
                    ));
                }
                let bytes = num("burst")?.map_or(up / 8, |b| b as u64);
                let peak = mbit(num("peak")?.unwrap_or(100.0));
                link.framing = Framing::DOCSIS;
                link.bucket = Some(Bucket {
                    bytes,
                    peak: (peak > up).then_some(peak),
                });
                link.slots = Some(CABLE_GRANTS);
            }
            "dsl" => {
                link.framing = match get("line").unwrap_or("adsl") {
                    "adsl" => Framing::ADSL_PPPOE,
                    "vdsl" => Framing::VDSL_PPPOE,
                    l => return Err(format!("link {spec}: line={l}: adsl or vdsl")),
                };
                link.bucket = Some(Bucket::of(3500));
            }
            "trace" => {
                let file = get("file").ok_or_else(|| format!("link {spec}: no file="))?;
                link.trace = Some(Trace {
                    file: file.to_owned(),
                    us: match get("format").unwrap_or("mm") {
                        "mm" => false,
                        "us" => true,
                        f => return Err(format!("link {spec}: format={f}: mm or us")),
                    },
                    scale: num("scale")?.unwrap_or(1.0),
                    offset: num("offset")?.unwrap_or(0.0),
                });
            }
            c => return Err(format!("link {spec}: {c}: tbf, cable, dsl, fiber or trace")),
        }
        if let Some(v) = get("slot") {
            link.slots = slots(v).map_err(|e| format!("link {spec}: {e}"))?;
        }
        Ok(link)
    }

    /// `site` shaped as the link is: its framing, slots and bucket, the
    /// run's `bucket` where the class takes it.
    pub fn site(&self, site: Site, bucket: Option<Bucket>) -> Site {
        let bucket = if self.runs_bucket {
            bucket
        } else {
            self.bucket
        };
        site.with_framing(self.framing)
            .with_slots(self.slots)
            .with_bucket(bucket)
    }

    /// A trace link's schedule, its time 0 (less `offset=`) at `start`,
    /// the stream's start in the run.
    pub fn capacity(&self, start: StdDuration) -> Result<Option<Arc<Capacity>>, String> {
        let Some(t) = &self.trace else {
            return Ok(None);
        };
        let text = std::fs::read_to_string(&t.file).map_err(|e| format!("{}: {e}", t.file))?;
        let cap = if t.us {
            Capacity::microseconds(&text, t.scale)
        } else {
            Capacity::mahimahi(&text, 1514, t.scale)
        }
        .map_err(|e| format!("{}: {e}", t.file))?;
        let origin = crate::wan::sim_time(start) - Duration::from_nanos((t.offset * 1e9) as i64);
        Ok(Some(Arc::new(cap.starting(origin))))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use strims_wan::{Access, City};

    #[test]
    fn classes_shape_their_sites() {
        let up = mbit(20.0);
        let base = Site::new(City::Chicago, Access::Cable).with_up(up);
        let run = Some(Bucket::of(3028));
        let tbf = TestbedLink::parse("tbf", up).unwrap();
        assert_eq!(tbf.site(base, run).profile.bucket, run);
        assert_eq!(tbf.site(base, run).profile.framing, Framing::ETHERNET);
        let cable = TestbedLink::parse("cable,aqm=fifo", up)
            .unwrap()
            .site(base, run);
        assert_eq!(
            cable.profile.bucket,
            Some(Bucket {
                bytes: 312_500,
                peak: Some(mbit(100.0))
            })
        );
        assert_eq!(cable.profile.slots, Some(CABLE_GRANTS));
        assert_eq!(cable.profile.framing, Framing::DOCSIS);
        let cable = TestbedLink::parse("cable,aqm=fifo,burst=10000,peak=10,slot=off", up)
            .unwrap()
            .site(base, run);
        assert_eq!(cable.profile.bucket, Some(Bucket::of(10_000)));
        assert_eq!(cable.profile.slots, None);
        let dsl = TestbedLink::parse("dsl,line=vdsl", up)
            .unwrap()
            .site(base, run);
        assert_eq!(dsl.profile.framing, Framing::VDSL_PPPOE);
        assert_eq!(dsl.profile.bucket, Some(Bucket::of(3500)));
        let wifi = TestbedLink::parse("fiber,slot=1-4", up).unwrap();
        assert_eq!(
            wifi.slots,
            Some(Slots {
                min: Duration::from_millis(1),
                max: Duration::from_millis(4)
            })
        );
    }

    #[test]
    fn what_the_model_lacks_is_refused() {
        for spec in [
            "cable",
            "cable,router=cake",
            "trace,file=x,delay=y@10",
            "tbf,peak=10",
            "dsl,line=sdsl",
            "token",
            "fiber,slot=4-1",
        ] {
            assert!(TestbedLink::parse(spec, 125_000).is_err(), "{spec}");
        }
    }

    #[test]
    fn a_trace_starts_at_the_stream() {
        let dir = std::env::temp_dir().join(format!("strims-link-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("t.up");
        std::fs::write(&f, "1\n2\n").unwrap();
        let spec = format!("trace,file={},offset=0.001", f.display());
        let link = TestbedLink::parse(&spec, 125_000).unwrap();
        let cap = link.capacity(StdDuration::from_secs(5)).unwrap().unwrap();
        let start = crate::wan::sim_time(StdDuration::from_secs(5));
        // 1 ms into the trace at the start: its 2 ms opportunity is 1 ms on.
        assert_eq!(cap.carries(2 * 1514 + 1), start + Duration::from_millis(1));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
