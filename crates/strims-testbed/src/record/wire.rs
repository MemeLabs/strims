// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A line's parts as bytes: how a browser node's worker hands them to the
//! worker that writes its line ([`crate::node::LineParts`]), one
//! transferred buffer a line, where JSON would cost the node's worker the
//! line's every digit.
//!
//! [`Wire::to_wire`] appends a value's bytes; [`Wire::from_wire`] reads one from the
//! front of a slice and advances it, `None` for bytes that aren't one.
//! Integers are little-endian at their width (`usize` as a `u64`), times
//! and durations their nanoseconds as an `i64`, a `bool` a byte, an
//! `Option` a byte (0 none, 1 some) and then its value, a `Vec` or a
//! `String` its length as a `u32` and then its items or its UTF-8 bytes, a
//! tuple or an array its items in order. Both ends are one build's code
//! (the page's bundle), so the bytes carry no version.

use strims_base::stats::{WAIT_BUCKETS, WaitHistogram};
use strims_base::time::{Duration, Time};
use strims_node_core::{HostId, TimerRecord, TimerSource};

use super::TimerContext;

/// A value as bytes (see the [module docs](self)).
pub trait Wire: Sized {
    fn to_wire(&self, out: &mut Vec<u8>);
    fn from_wire(r: &mut &[u8]) -> Option<Self>;
}

/// The first `N` bytes of `r`, taken.
fn bytes<const N: usize>(r: &mut &[u8]) -> Option<[u8; N]> {
    let (head, rest) = r.split_first_chunk::<N>()?;
    *r = rest;
    Some(*head)
}

macro_rules! ints {
    ($($t:ty),*) => {$(
        impl Wire for $t {
            fn to_wire(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_le_bytes());
            }
            fn from_wire(r: &mut &[u8]) -> Option<Self> {
                Some(<$t>::from_le_bytes(bytes(r)?))
            }
        }
    )*};
}
ints!(u8, u16, u32, u64, i64);

impl Wire for usize {
    fn to_wire(&self, out: &mut Vec<u8>) {
        (*self as u64).to_wire(out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        usize::try_from(u64::from_wire(r)?).ok()
    }
}

impl Wire for bool {
    fn to_wire(&self, out: &mut Vec<u8>) {
        out.push(u8::from(*self));
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        match u8::from_wire(r)? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
}

impl Wire for Time {
    fn to_wire(&self, out: &mut Vec<u8>) {
        self.unix_nanos().to_wire(out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        Some(Time::from_unix_nanos(i64::from_wire(r)?))
    }
}

impl Wire for Duration {
    fn to_wire(&self, out: &mut Vec<u8>) {
        self.as_nanos().to_wire(out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        Some(Duration::from_nanos(i64::from_wire(r)?))
    }
}

impl Wire for HostId {
    fn to_wire(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_bytes());
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        Some(HostId::from_bytes(&bytes(r)?))
    }
}

impl<T: Wire> Wire for Option<T> {
    fn to_wire(&self, out: &mut Vec<u8>) {
        match self {
            None => out.push(0),
            Some(x) => {
                out.push(1);
                x.to_wire(out);
            }
        }
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        match u8::from_wire(r)? {
            0 => Some(None),
            1 => Some(Some(T::from_wire(r)?)),
            _ => None,
        }
    }
}

/// Puts a length as the bytes carry it, a `u32` (saturating).
pub fn put_len(n: usize, out: &mut Vec<u8>) {
    u32::try_from(n).unwrap_or(u32::MAX).to_wire(out);
}

/// A length the bytes carry, at most what `r` holds of items at least a
/// byte long, so bytes that aren't a vector allocate nothing much.
pub fn take_len(r: &mut &[u8]) -> Option<usize> {
    let n = usize::try_from(u32::from_wire(r)?).ok()?;
    (n <= r.len()).then_some(n)
}

impl<T: Wire> Wire for Vec<T> {
    fn to_wire(&self, out: &mut Vec<u8>) {
        put_len(self.len(), out);
        for x in self {
            x.to_wire(out);
        }
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        let n = take_len(r)?;
        (0..n).map(|_| T::from_wire(r)).collect()
    }
}

/// Puts a name (a variant's, a source's), as a `String` goes.
pub fn put_name(name: &str, out: &mut Vec<u8>) {
    put_len(name.len(), out);
    out.extend_from_slice(name.as_bytes());
}

/// The name [`put_name`] put at the front of `r`, taken, borrowed from it.
pub fn take_name<'a>(r: &mut &'a [u8]) -> Option<&'a str> {
    let n = take_len(r)?;
    let (s, rest) = r.split_at_checked(n)?;
    *r = rest;
    std::str::from_utf8(s).ok()
}

impl Wire for String {
    fn to_wire(&self, out: &mut Vec<u8>) {
        put_len(self.len(), out);
        out.extend_from_slice(self.as_bytes());
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        let n = take_len(r)?;
        let (s, rest) = r.split_at_checked(n)?;
        *r = rest;
        String::from_utf8(s.to_vec()).ok()
    }
}

impl<T: Wire + Copy + Default, const N: usize> Wire for [T; N] {
    fn to_wire(&self, out: &mut Vec<u8>) {
        for x in self {
            x.to_wire(out);
        }
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        let mut a = [T::default(); N];
        for x in &mut a {
            *x = T::from_wire(r)?;
        }
        Some(a)
    }
}

macro_rules! tuple {
    ($($t:ident $x:ident),*) => {
        impl<$($t: Wire),*> Wire for ($($t,)*) {
            fn to_wire(&self, out: &mut Vec<u8>) {
                let ($($x,)*) = self;
                $($x.to_wire(out);)*
            }
            fn from_wire(r: &mut &[u8]) -> Option<Self> {
                Some(($($t::from_wire(r)?,)*))
            }
        }
    };
}
tuple!(A a, B b);
tuple!(A a, B b, C c);
tuple!(A a, B b, C c, D d);

impl Wire for WaitHistogram {
    fn to_wire(&self, out: &mut Vec<u8>) {
        self.buckets.to_wire(out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        Some(WaitHistogram {
            buckets: <[u64; WAIT_BUCKETS]>::from_wire(r)?,
        })
    }
}

impl Wire for TimerContext {
    fn to_wire(&self, out: &mut Vec<u8>) {
        put_name(self.name(), out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        TimerContext::parse(take_name(r)?)
    }
}

impl Wire for TimerRecord {
    fn to_wire(&self, out: &mut Vec<u8>) {
        put_name(self.source.name(), out);
        self.armed.to_wire(out);
        self.deadline.to_wire(out);
        self.fired.to_wire(out);
        self.requested.to_wire(out);
    }
    fn from_wire(r: &mut &[u8]) -> Option<Self> {
        Some(TimerRecord {
            source: TimerSource::parse(take_name(r)?)?,
            armed: Wire::from_wire(r)?,
            deadline: Wire::from_wire(r)?,
            fired: Wire::from_wire(r)?,
            requested: Wire::from_wire(r)?,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    fn round<T: Wire + PartialEq + std::fmt::Debug>(x: T) {
        let mut b = Vec::new();
        x.to_wire(&mut b);
        let mut r = b.as_slice();
        assert_eq!(T::from_wire(&mut r), Some(x));
        assert!(r.is_empty());
    }

    /// Each kind of value reads back as it was put, and bytes cut short or
    /// of another kind read as none.
    #[test]
    fn values_read_back_as_put() {
        round(7u8);
        round(u64::MAX);
        round(-5i64);
        round(usize::MAX >> 1);
        round(true);
        round(Time::from_unix_nanos(-3));
        round(Duration::from_micros(250));
        round(HostId([1, 2, 3, u64::MAX]));
        round(Some(3u16));
        round(None::<u32>);
        round(vec![(1u64, Some(true)), (2, None)]);
        round("π ok".to_owned());
        round([9u64; 4]);
        round((1u8, 2u16, 3u32, 4u64));
        let mut h = WaitHistogram::default();
        h.buckets[5] = 8;
        round(h);
        round(TimerRecord {
            source: TimerSource::Pacer,
            armed: Time::from_unix_millis(1),
            deadline: Time::from_unix_millis(2),
            fired: Time::from_unix_millis(3),
            requested: Duration::from_millis(1),
        });
        round(TimerContext::Worker);
        let mut b = Vec::new();
        vec![1u64, 2].to_wire(&mut b);
        let mut short = &b[..b.len() - 1];
        assert_eq!(Vec::<u64>::from_wire(&mut short), None);
        let mut huge = &[0xff, 0xff, 0xff, 0x7f][..];
        assert_eq!(Vec::<u8>::from_wire(&mut huge), None);
        assert_eq!(bool::from_wire(&mut &[2u8][..]), None);
    }
}
