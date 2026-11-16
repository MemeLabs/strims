// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The sims' twins of the Docker testbed's access link classes: the
//! testbed's own [`Spec`] ([`strims_wan::testbed`], whose docs give the
//! classes, their keys and defaults) as a site ([`TestbedLink`]), refusing
//! what the WAN model has no form of: a home router's AQM, DOCSIS-PIE, a
//! delay trace and a byte limit.

use std::sync::Arc;
use std::time::Duration as StdDuration;

use strims_base::time::Duration;
use strims_wan::capacity::Capacity;
use strims_wan::framing::Framing;
use strims_wan::slots::Slots;
use strims_wan::{Bucket, Site};

pub use strims_wan::testbed::{Class, Router, Spec};

/// A testbed link as the twin builds it: a [`Spec`] the WAN model can
/// shape.
#[derive(Clone, Debug, PartialEq)]
pub struct TestbedLink {
    spec: Spec,
}

impl TestbedLink {
    /// Parses `spec`, refusing what the model has no form of: a home
    /// router's AQM, DOCSIS-PIE (cable's default: give `aqm=fifo`), a
    /// delay trace, and a byte limit (the model's queue is the run's
    /// `QUEUE_MS` at the node's rate, as `run.sh`'s default limit is).
    pub fn parse(spec: &str) -> Result<TestbedLink, String> {
        let s = Spec::parse(spec)?;
        if s.router().is_some() {
            return Err(format!("link {spec}: the model has no home router AQM"));
        }
        if s.docsis_pie() {
            return Err(format!(
                "link {spec}: the model has no DOCSIS-PIE (give aqm=fifo)"
            ));
        }
        if let Some(t) = s.trace() {
            if t.delay.is_some() {
                return Err(format!("link {spec}: the model has no delay trace"));
            }
            if t.limit.is_some() {
                return Err(format!(
                    "link {spec}: the model's queue is the run's at the node's rate; leave limit= out"
                ));
            }
        }
        Ok(TestbedLink { spec: s })
    }

    pub fn class(&self) -> Class {
        self.spec.class
    }

    pub fn framing(&self) -> Framing {
        self.spec.framing()
    }

    pub fn slots(&self) -> Option<Slots> {
        self.spec.slots()
    }

    /// `site` shaped as the link is: its framing, slots and bucket, the
    /// run's `bucket` where the class takes it (`None`: none); fails on a
    /// bucket smaller than the link's largest frame, as the testbed does.
    pub fn site(&self, site: Site, bucket: Option<Bucket>) -> Result<Site, String> {
        let bucket = match self.spec.class {
            Class::Tbf | Class::Fiber | Class::Trace => bucket,
            Class::Cable | Class::Dsl => Some(self.spec.bucket(site.profile.up, Bucket::of(0))),
        };
        if let Some(b) = bucket
            && self.spec.class != Class::Trace
        {
            self.spec.check_bucket(b.bytes)?;
        }
        Ok(site
            .with_framing(self.framing())
            .with_slots(self.slots())
            .with_bucket(bucket))
    }

    /// A trace link's schedule, its time 0 (less `offset=`) at `start`,
    /// the stream's start in the run.
    pub fn capacity(&self, start: StdDuration) -> Result<Option<Arc<Capacity>>, String> {
        let Some(t) = self.spec.trace() else {
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
    use strims_wan::access::CABLE_GRANTS;
    use strims_wan::{Access, City};

    fn mbit(m: f64) -> u64 {
        (m * 125_000.0) as u64
    }

    #[test]
    fn classes_shape_their_sites() {
        let up = mbit(20.0);
        let base = Site::new(City::Chicago, Access::Cable).with_up(up);
        let run = Some(Bucket::of(3028));
        let tbf = TestbedLink::parse("tbf").unwrap();
        assert_eq!(tbf.site(base, run).unwrap().profile.bucket, run);
        assert_eq!(
            tbf.site(base, run).unwrap().profile.framing,
            Framing::ETHERNET
        );
        let cable = TestbedLink::parse("cable,aqm=fifo")
            .unwrap()
            .site(base, run)
            .unwrap();
        assert_eq!(
            cable.profile.bucket,
            Some(Bucket {
                bytes: 312_500,
                peak: Some(mbit(100.0))
            })
        );
        assert_eq!(cable.profile.slots, Some(CABLE_GRANTS));
        assert_eq!(cable.profile.framing, Framing::DOCSIS);
        let cable = TestbedLink::parse("cable,aqm=fifo,burst=10000,peak=10,slot=off")
            .unwrap()
            .site(base, run)
            .unwrap();
        assert_eq!(cable.profile.bucket, Some(Bucket::of(10_000)));
        assert_eq!(cable.profile.slots, None);
        let dsl = TestbedLink::parse("dsl,line=vdsl")
            .unwrap()
            .site(base, run)
            .unwrap();
        assert_eq!(dsl.profile.framing, Framing::VDSL_PPPOE);
        assert_eq!(dsl.profile.bucket, Some(Bucket::of(3500)));
    }

    #[test]
    fn a_bucket_holds_the_largest_frame_on_both_sides() {
        let up = mbit(20.0);
        let base = Site::new(City::Chicago, Access::Cable).with_up(up);
        // Fibre's and cable's frames are 1518 bytes, as the testbed's
        // parser takes them (strims_wan::testbed's tests).
        let fiber = TestbedLink::parse("fiber").unwrap();
        assert!(fiber.site(base, Some(Bucket::of(1517))).is_err());
        assert!(fiber.site(base, Some(Bucket::of(1518))).is_ok());
        let cable = TestbedLink::parse("cable,aqm=fifo,burst=1517").unwrap();
        assert!(cable.site(base, None).is_err());
    }

    #[test]
    fn the_twin_refuses_what_the_model_lacks() {
        for spec in [
            "cable",
            "cable,router=cake",
            "tbf,router=fq_codel",
            "trace,file=x,delay=y@10",
            "trace,file=x,limit=300000",
        ] {
            assert!(Spec::parse(spec).is_ok(), "{spec}");
            assert!(TestbedLink::parse(spec).is_err(), "{spec}");
        }
    }

    #[test]
    fn a_trace_starts_at_the_stream() {
        let dir = std::env::temp_dir().join(format!("strims-link-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("t.up");
        std::fs::write(&f, "1\n2\n").unwrap();
        let spec = format!("trace,file={},offset=0.001", f.display());
        let link = TestbedLink::parse(&spec).unwrap();
        let cap = link.capacity(StdDuration::from_secs(5)).unwrap().unwrap();
        let start = crate::wan::sim_time(StdDuration::from_secs(5));
        // 1 ms into the trace at the start: its 2 ms opportunity is 1 ms on.
        assert_eq!(cap.carries(2 * 1514 + 1), start + Duration::from_millis(1));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
