// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Wall-clock time and signed durations.
//!
//! [`Time`] is nanoseconds since the Unix epoch, kept at millisecond
//! precision so it round-trips through the wire format. [`Duration`] is
//! signed because peer timestamps come from other clocks and differences
//! between them can be negative.
//!
//! All arithmetic saturates.

use core::fmt;
use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Signed duration in nanoseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration(i64);

impl Duration {
    pub const ZERO: Duration = Duration(0);
    pub const NANOSECOND: Duration = Duration(1);
    pub const MICROSECOND: Duration = Duration(1_000);
    pub const MILLISECOND: Duration = Duration(1_000_000);
    pub const SECOND: Duration = Duration(1_000_000_000);
    pub const MINUTE: Duration = Duration(60 * 1_000_000_000);
    pub const HOUR: Duration = Duration(60 * 60 * 1_000_000_000);
    pub const MAX: Duration = Duration(i64::MAX);
    pub const MIN: Duration = Duration(i64::MIN);

    pub const fn from_nanos(n: i64) -> Duration {
        Duration(n)
    }

    pub const fn from_micros(n: i64) -> Duration {
        Duration(n.saturating_mul(1_000))
    }

    pub const fn from_millis(n: i64) -> Duration {
        Duration(n.saturating_mul(1_000_000))
    }

    pub const fn from_secs(n: i64) -> Duration {
        Duration(n.saturating_mul(1_000_000_000))
    }

    /// Converts from fractional seconds, saturating at the bounds.
    pub fn from_secs_f64(s: f64) -> Duration {
        Duration((s * 1e9) as i64)
    }

    pub const fn as_nanos(self) -> i64 {
        self.0
    }

    pub const fn as_micros(self) -> i64 {
        self.0 / 1_000
    }

    pub const fn as_millis(self) -> i64 {
        self.0 / 1_000_000
    }

    pub const fn as_secs(self) -> i64 {
        self.0 / 1_000_000_000
    }

    pub fn as_secs_f64(self) -> f64 {
        self.0 as f64 / 1e9
    }

    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    pub const fn abs(self) -> Duration {
        Duration(self.0.saturating_abs())
    }

    /// Rounds toward zero to a multiple of `m`. A non-positive `m` leaves
    /// the duration unchanged.
    pub const fn truncate(self, m: Duration) -> Duration {
        if m.0 <= 0 {
            return self;
        }
        Duration(self.0 - self.0 % m.0)
    }

    /// Converts to `std::time::Duration`, or `None` if negative.
    pub fn to_std(self) -> Option<std::time::Duration> {
        u64::try_from(self.0)
            .ok()
            .map(std::time::Duration::from_nanos)
    }
}

impl From<std::time::Duration> for Duration {
    fn from(d: std::time::Duration) -> Duration {
        Duration(i64::try_from(d.as_nanos()).unwrap_or(i64::MAX))
    }
}

impl Add for Duration {
    type Output = Duration;
    fn add(self, o: Duration) -> Duration {
        Duration(self.0.saturating_add(o.0))
    }
}

impl AddAssign for Duration {
    fn add_assign(&mut self, o: Duration) {
        *self = *self + o;
    }
}

impl Sub for Duration {
    type Output = Duration;
    fn sub(self, o: Duration) -> Duration {
        Duration(self.0.saturating_sub(o.0))
    }
}

impl SubAssign for Duration {
    fn sub_assign(&mut self, o: Duration) {
        *self = *self - o;
    }
}

impl Neg for Duration {
    type Output = Duration;
    fn neg(self) -> Duration {
        Duration(self.0.saturating_neg())
    }
}

impl Mul<i64> for Duration {
    type Output = Duration;
    fn mul(self, n: i64) -> Duration {
        Duration(self.0.saturating_mul(n))
    }
}

impl Div<i64> for Duration {
    type Output = Duration;
    /// Division by zero yields [`Duration::MAX`] or [`Duration::MIN`] by sign.
    fn div(self, n: i64) -> Duration {
        match self.0.checked_div(n) {
            Some(v) => Duration(v),
            None if self.0 < 0 => Duration::MIN,
            None => Duration::MAX,
        }
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}ms", self.0 as f64 / 1e6)
    }
}

/// Error returned when an encode or decode buffer is too short.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferTooShort;

impl fmt::Display for BufferTooShort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("buffer too short")
    }
}

impl std::error::Error for BufferTooShort {}

/// Nanoseconds since the Unix epoch at millisecond precision.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time(i64);

impl Time {
    /// Sentinel for "no time".
    pub const NIL: Time = Time(i64::MIN);
    /// The latest representable time.
    pub const MAX: Time = Time(i64::MAX);
    /// The Unix epoch.
    pub const EPOCH: Time = Time(0);
    /// Wire size in bytes.
    pub const BYTE_LEN: usize = 8;
    /// Clock precision.
    pub const PRECISION: Duration = Duration::MILLISECOND;

    pub const fn from_unix_nanos(n: i64) -> Time {
        Time(n)
    }

    pub const fn from_unix(sec: i64, nsec: i64) -> Time {
        Time(sec.saturating_mul(1_000_000_000).saturating_add(nsec))
    }

    pub const fn from_unix_millis(ms: i64) -> Time {
        Time(ms.saturating_mul(1_000_000))
    }

    /// Converts a wall-clock reading, truncated to [`Time::PRECISION`].
    /// Only drivers obtain `SystemTime`; core crates receive `Time`.
    pub fn from_system(t: std::time::SystemTime) -> Time {
        let n = match t.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => i64::try_from(d.as_nanos()).unwrap_or(i64::MAX),
            Err(e) => i64::try_from(e.duration().as_nanos()).map_or(i64::MIN, |n| -n),
        };
        Time(n).truncate(Time::PRECISION)
    }

    pub const fn unix_nanos(self) -> i64 {
        self.0
    }

    pub const fn unix_millis(self) -> i64 {
        self.0 / 1_000_000
    }

    pub const fn unix(self) -> i64 {
        self.0 / 1_000_000_000
    }

    pub const fn is_nil(self) -> bool {
        self.0 == i64::MIN
    }

    pub const fn after(self, o: Time) -> bool {
        self.0 > o.0
    }

    pub const fn before(self, o: Time) -> bool {
        self.0 < o.0
    }

    /// Rounds toward zero to a multiple of `d`, so times before the epoch
    /// round up. A non-positive `d` leaves the time unchanged.
    pub const fn truncate(self, d: Duration) -> Time {
        if d.0 <= 0 {
            return self;
        }
        Time(self.0 - self.0 % d.0)
    }

    /// Writes the 8 byte big-endian wire form.
    pub fn encode(self, buf: &mut [u8]) -> Result<usize, BufferTooShort> {
        let dst = buf.get_mut(..Self::BYTE_LEN).ok_or(BufferTooShort)?;
        dst.copy_from_slice(&self.0.to_be_bytes());
        Ok(Self::BYTE_LEN)
    }

    /// Reads the 8 byte big-endian wire form.
    pub fn decode(buf: &[u8]) -> Result<(Time, usize), BufferTooShort> {
        let src: [u8; Self::BYTE_LEN] = buf
            .get(..Self::BYTE_LEN)
            .and_then(|b| b.try_into().ok())
            .ok_or(BufferTooShort)?;
        Ok((Time(i64::from_be_bytes(src)), Self::BYTE_LEN))
    }
}

impl Add<Duration> for Time {
    type Output = Time;
    fn add(self, d: Duration) -> Time {
        Time(self.0.saturating_add(d.0))
    }
}

impl AddAssign<Duration> for Time {
    fn add_assign(&mut self, d: Duration) {
        *self = *self + d;
    }
}

impl Sub<Duration> for Time {
    type Output = Time;
    fn sub(self, d: Duration) -> Time {
        Time(self.0.saturating_sub(d.0))
    }
}

impl SubAssign<Duration> for Time {
    fn sub_assign(&mut self, d: Duration) {
        *self = *self - d;
    }
}

impl Sub for Time {
    type Output = Duration;
    fn sub(self, o: Time) -> Duration {
        Duration(self.0.saturating_sub(o.0))
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nil() {
            return f.write_str("nil");
        }
        let ms = self.0.div_euclid(1_000_000);
        write!(f, "{}.{:03}", ms.div_euclid(1_000), ms.rem_euclid(1_000))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_round_trip() {
        let t = Time::from_unix_millis(1_700_000_000_123);
        let mut buf = [0u8; 8];
        assert_eq!(t.encode(&mut buf), Ok(8));
        assert_eq!(Time::decode(&buf), Ok((t, 8)));
        assert_eq!(Time::decode(&buf[..7]), Err(BufferTooShort));
    }

    #[test]
    fn truncate_rounds_toward_zero() {
        let t = Time::from_unix_nanos(1_234_567_891);
        assert_eq!(
            t.truncate(Time::PRECISION),
            Time::from_unix_nanos(1_234_000_000)
        );
        // Times before the epoch round up.
        let n = Time::from_unix_nanos(-1_234_567_891);
        assert_eq!(
            n.truncate(Time::PRECISION),
            Time::from_unix_nanos(-1_234_000_000)
        );
    }

    #[test]
    fn signed_differences() {
        let a = Time::from_unix_millis(1_000);
        let b = Time::from_unix_millis(1_250);
        assert_eq!(b - a, Duration::from_millis(250));
        assert_eq!(a - b, Duration::from_millis(-250));
        assert_eq!(Time::MAX + Duration::SECOND, Time::MAX);
        assert_eq!(Time::NIL - Duration::SECOND, Time::NIL);
    }
}
