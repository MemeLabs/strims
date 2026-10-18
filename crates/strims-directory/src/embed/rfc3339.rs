// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! RFC 3339 date-times, as the embed APIs send them.

/// The Unix time, in whole seconds, of `s`:
/// `YYYY-MM-DDTHH:MM:SS[.fraction](Z|±HH:MM)`, with `T` in either case or
/// a space, and `Z` in either case. `None` if it doesn't parse or a field
/// is out of range.
pub fn parse(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    let num = |from: usize, len: usize| -> Option<i64> {
        let digits = b.get(from..from + len)?;
        digits.iter().try_fold(0i64, |n, d| {
            d.is_ascii_digit().then(|| n * 10 + i64::from(d - b'0'))
        })
    };
    let at = |i: usize, c: &[u8]| b.get(i).is_some_and(|x| c.contains(x));
    if !(at(4, b"-") && at(7, b"-") && at(10, b"Tt ") && at(13, b":") && at(16, b":")) {
        return None;
    }
    let (year, month, day) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    let (hour, minute, second) = (num(11, 2)?, num(14, 2)?, num(17, 2)?);
    if !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    let mut i = 19;
    if at(i, b".") {
        i += 1;
        let start = i;
        while at(i, b"0123456789") {
            i += 1;
        }
        if i == start {
            return None;
        }
    }
    let offset = if at(i, b"Zz") && b.len() == i + 1 {
        0
    } else if at(i, b"+-") && at(i + 3, b":") && b.len() == i + 6 {
        let (h, m) = (num(i + 1, 2)?, num(i + 4, 2)?);
        if h > 23 || m > 59 {
            return None;
        }
        let o = h * 3600 + m * 60;
        if at(i, b"-") { -o } else { o }
    } else {
        return None;
    };
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        2 if is_leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days from 1970-01-01 to the proleptic Gregorian date `y-m-d`.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_times_parse_to_unix_seconds() {
        for (s, want) in [
            ("1970-01-01T00:00:00Z", 0),
            ("2023-11-14T22:13:20Z", 1_700_000_000),
            ("2023-11-14t22:13:20z", 1_700_000_000),
            ("2023-11-14 22:13:20Z", 1_700_000_000),
            ("2023-11-14T22:13:20.999Z", 1_700_000_000),
            ("2023-11-14T22:13:20.000000000Z", 1_700_000_000),
            ("2023-11-15T00:13:20+02:00", 1_700_000_000),
            ("2023-11-14T20:43:20-01:30", 1_700_000_000),
            ("2000-02-29T00:00:00Z", 951_782_400),
            ("1969-12-31T23:59:59Z", -1),
            ("2038-01-19T03:14:08Z", 2_147_483_648),
        ] {
            assert_eq!(parse(s), Some(want), "{s}");
        }
    }

    #[test]
    fn malformed_date_times_are_refused() {
        for s in [
            "",
            "2023-11-14",
            "2023-11-14T22:13:20",
            "2023-11-14T22:13Z",
            "2023/11/14T22:13:20Z",
            "2023-13-14T22:13:20Z",
            "2023-00-14T22:13:20Z",
            "2023-02-29T22:13:20Z",
            "2023-04-31T00:00:00Z",
            "2023-11-14T24:00:00Z",
            "2023-11-14T22:60:00Z",
            "2023-11-14T22:13:61Z",
            "2023-11-14T22:13:20.Z",
            "2023-11-14T22:13:20+0200",
            "2023-11-14T22:13:20+24:00",
            "2023-11-14T22:13:20Zjunk",
            "2023-11-14T22:13:20+02:00x",
            "+023-11-14T22:13:20Z",
            "２023-11-14T22:13:20Z",
        ] {
            assert_eq!(parse(s), None, "{s}");
        }
    }
}
