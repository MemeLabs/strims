// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Duration strings, as the UI writes mute and ignore durations.
//!
//! A duration is an optional sign, then one or more decimal numbers, each
//! with an optional fraction and a unit: "300ms", "-1.5h", "+5m",
//! "2h45m". The units are "ns", "us" (also "µs" and "μs"), "ms", "s", "m"
//! and "h". "0" alone, signed or not, needs no unit. The total must fit
//! in an `i64` of nanoseconds, from `-2^63` to `2^63 - 1`. A fraction
//! keeps at most the digits that fit in 63 bits, and the nanoseconds it
//! adds to each number are computed in `f64` and truncated.

use strims_base::time::Duration;

/// The longest duration string [`parse`] reads.
pub const MAX_DURATION_BYTES: usize = 64;

/// Why a duration string didn't parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DurationError {
    /// It's empty, malformed, or out of range.
    #[error("invalid duration")]
    Invalid,
    /// A number has no unit.
    #[error("missing unit in duration")]
    MissingUnit,
    #[error("unknown unit in duration")]
    UnknownUnit,
    /// It's longer than [`MAX_DURATION_BYTES`].
    #[error("duration too long")]
    TooLong,
}

/// `2^63`: the magnitude of the most negative duration.
const LIMIT: u64 = 1 << 63;

/// Parses the duration string `s`; see the module docs.
pub fn parse(s: &str) -> Result<Duration, DurationError> {
    if s.len() > MAX_DURATION_BYTES {
        return Err(DurationError::TooLong);
    }

    let (neg, mut s) = match s.as_bytes().first() {
        Some(b'-') => (true, s.get(1..).unwrap_or_default()),
        Some(b'+') => (false, s.get(1..).unwrap_or_default()),
        _ => (false, s),
    };
    if s == "0" {
        return Ok(Duration::ZERO);
    }
    if s.is_empty() {
        return Err(DurationError::Invalid);
    }

    let mut total: u64 = 0;
    while let Some(&first) = s.as_bytes().first() {
        if first != b'.' && !first.is_ascii_digit() {
            return Err(DurationError::Invalid);
        }
        let (whole, digits, rest) = leading_int(s)?;
        s = rest;

        let (mut fraction, mut scale, mut fraction_digits) = (0, 1.0, 0);
        if let Some(rest) = s.strip_prefix('.') {
            (fraction, scale, fraction_digits, s) = leading_fraction(rest);
        }
        if digits == 0 && fraction_digits == 0 {
            return Err(DurationError::Invalid);
        }

        // The split falls on an ASCII byte or the end, so on a char
        // boundary.
        let unit_len = s
            .bytes()
            .position(|c| c == b'.' || c.is_ascii_digit())
            .unwrap_or(s.len());
        if unit_len == 0 {
            return Err(DurationError::MissingUnit);
        }
        let (unit, rest) = s.split_at_checked(unit_len).ok_or(DurationError::Invalid)?;
        s = rest;
        let unit = unit_nanos(unit).ok_or(DurationError::UnknownUnit)?;

        if whole > LIMIT / unit {
            return Err(DurationError::Invalid);
        }
        let mut v = whole * unit;
        if fraction > 0 {
            // At most one unit (3.6e12 ns for an hour), as the fraction is
            // below 1.
            v = v.saturating_add((fraction as f64 * (unit as f64 / scale)) as u64);
            if v > LIMIT {
                return Err(DurationError::Invalid);
            }
        }
        total = total
            .checked_add(v)
            .filter(|&t| t <= LIMIT)
            .ok_or(DurationError::Invalid)?;
    }

    if neg {
        Ok(Duration::from_nanos(0i64.wrapping_sub_unsigned(total)))
    } else {
        i64::try_from(total)
            .map(Duration::from_nanos)
            .map_err(|_| DurationError::Invalid)
    }
}

/// The nanoseconds of `unit`.
fn unit_nanos(unit: &str) -> Option<u64> {
    Some(match unit {
        "ns" => 1,
        "us" | "\u{b5}s" | "\u{3bc}s" => 1_000,
        "ms" => 1_000_000,
        "s" => 1_000_000_000,
        "m" => 60_000_000_000,
        "h" => 3_600_000_000_000,
        _ => return None,
    })
}

/// The leading decimal digits of `s` as an integer of at most `2^63`, the
/// number of digits, and the rest of `s`.
fn leading_int(s: &str) -> Result<(u64, usize, &str), DurationError> {
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    let mut x: u64 = 0;
    for c in s.bytes().take(digits) {
        if x > LIMIT / 10 {
            return Err(DurationError::Invalid);
        }
        x = x * 10 + u64::from(c - b'0');
        if x > LIMIT {
            return Err(DurationError::Invalid);
        }
    }
    Ok((x, digits, s.get(digits..).unwrap_or_default()))
}

/// The leading decimal digits of `s` as a fraction `x / scale`, the
/// number of digits, and the rest of `s`. Digits past those that fit in
/// `x` are read and dropped.
fn leading_fraction(s: &str) -> (u64, f64, usize, &str) {
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    let mut x: u64 = 0;
    let mut scale = 1.0;
    for c in s.bytes().take(digits) {
        if x > (LIMIT - 1) / 10 {
            break;
        }
        let y = x * 10 + u64::from(c - b'0');
        if y > LIMIT {
            break;
        }
        x = y;
        scale *= 10.0;
    }
    (x, scale, digits, s.get(digits..).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NS: i64 = 1;
    const US: i64 = 1_000;
    const MS: i64 = 1_000_000;
    const S: i64 = 1_000_000_000;
    const M: i64 = 60 * S;
    const H: i64 = 60 * M;

    #[test]
    fn valid_durations() {
        let cases: &[(&str, i64)] = &[
            // simple
            ("0", 0),
            ("5s", 5 * S),
            ("30s", 30 * S),
            ("1478s", 1478 * S),
            // sign
            ("-5s", -5 * S),
            ("+5s", 5 * S),
            ("-0", 0),
            ("+0", 0),
            ("-0s", 0),
            // decimal
            ("5.0s", 5 * S),
            ("5.6s", 5 * S + 600 * MS),
            ("5.s", 5 * S),
            (".5s", 500 * MS),
            ("1.0s", S),
            ("1.00s", S),
            ("1.004s", S + 4 * MS),
            ("1.0040s", S + 4 * MS),
            ("100.00100s", 100 * S + MS),
            ("0.0s", 0),
            // units
            ("10ns", 10 * NS),
            ("11us", 11 * US),
            ("12\u{b5}s", 12 * US),
            ("12\u{3bc}s", 12 * US),
            ("13ms", 13 * MS),
            ("14s", 14 * S),
            ("15m", 15 * M),
            ("16h", 16 * H),
            // composite
            ("3h30m", 3 * H + 30 * M),
            ("1h30m", H + 30 * M),
            ("10.5s4m", 4 * M + 10 * S + 500 * MS),
            ("-2m3.4s", -(2 * M + 3 * S + 400 * MS)),
            (
                "1h2m3s4ms5us6ns",
                H + 2 * M + 3 * S + 4 * MS + 5 * US + 6 * NS,
            ),
            ("39h9m14.425s", 39 * H + 9 * M + 14 * S + 425 * MS),
            ("300ms", 300 * MS),
            ("-1.5h", -(H + 30 * M)),
            ("+5m", 5 * M),
            ("1s1s", 2 * S),
            // large values
            ("52763797000ns", 52763797000),
            ("0.3333333333333333333h", 20 * M),
            ("9007199254740993ns", (1 << 53) + 1),
            ("9223372036854775807ns", i64::MAX),
            ("9223372036854775.807us", i64::MAX),
            ("9223372036s854ms775us807ns", i64::MAX),
            ("-9223372036854775808ns", i64::MIN),
            ("-9223372036854775.808us", i64::MIN),
            ("-9223372036s854ms775us808ns", i64::MIN),
            ("-2562047h47m16.854775808s", i64::MIN),
            ("0.100000000000000000000h", 6 * M),
            ("0.830103483285477580700h", 49 * M + 48 * S + 372539827),
        ];
        for &(input, want) in cases {
            assert_eq!(parse(input), Ok(Duration::from_nanos(want)), "{input:?}");
        }
    }

    #[test]
    fn invalid_durations() {
        use DurationError::*;
        let cases: &[(&str, DurationError)] = &[
            ("", Invalid),
            ("3", MissingUnit),
            ("-", Invalid),
            ("+", Invalid),
            ("s", Invalid),
            (".", Invalid),
            ("-.", Invalid),
            (".s", Invalid),
            ("+.s", Invalid),
            ("1d", UnknownUnit),
            ("1H", UnknownUnit),
            ("5 s", UnknownUnit),
            ("1e3s", UnknownUnit),
            ("\u{fffd}", Invalid),
            ("\u{fffd} hello \u{fffd} world", Invalid),
            ("00", MissingUnit),
            ("--5s", Invalid),
            ("+-5s", Invalid),
            (" 5s", Invalid),
            ("5s ", UnknownUnit),
            ("1.5.5s", MissingUnit),
            ("1s.", Invalid),
            ("1..5s", MissingUnit),
            ("1sm", UnknownUnit),
            ("µs", Invalid),
            // overflow
            ("9223372036854775810ns", Invalid),
            ("9223372036854775808ns", Invalid),
            ("-9223372036854775809ns", Invalid),
            ("9223372036854776us", Invalid),
            ("3000000h", Invalid),
            ("9223372036854775.808us", Invalid),
            ("9223372036854ms775us808ns", Invalid),
            ("99999999999999999999ns", Invalid),
            ("9223372036854775808ns9223372036854775808ns", Invalid),
        ];
        for &(input, want) in cases {
            assert_eq!(parse(input), Err(want), "{input:?}");
        }
    }

    #[test]
    fn the_input_length_is_bounded() {
        let long = format!("{}s", "0".repeat(MAX_DURATION_BYTES));
        assert_eq!(parse(&long), Err(DurationError::TooLong));
        let fits = format!("{}1s", "0".repeat(MAX_DURATION_BYTES - 2));
        assert_eq!(parse(&fits), Ok(Duration::SECOND));
    }
}
