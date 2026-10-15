// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A text dump of a map's cells, and a parser for it, for building tests
//! from log output. The golden trace hashes the dump, so its format pins the
//! cell layout.

use core::fmt::Write;

use crate::bin::Bin;
use crate::cell::{Cell, Ref, is_map_ref};
use crate::map::Map;

/// Error returned by [`Map::debug_parse`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DebugParseError {
    #[error("malformed header line")]
    Header,
    #[error("malformed cell on line {0}")]
    Cell(usize),
    #[error("inconsistent map: {0}")]
    Inconsistent(&'static str),
}

impl Map {
    /// Dumps the map's cells: one line per cell with the bitmaps in binary
    /// and refs in decimal, tagged `M` (map cell), `F` (free), `R` (ref) or
    /// `B` (bitmap).
    pub fn debug_string(&self) -> String {
        let (free_top, alloc_count, cell_count, root_bin, cells) = self.raw_parts();
        let mut s = String::new();

        // Writing to a String can't fail.
        let _ = writeln!(
            s,
            "freeTop: {free_top}, allocCount: {alloc_count}, cellCount: {cell_count}, rootBin: {}",
            root_bin.0
        );

        let mut free = vec![false; cells.len()];
        let mut r = free_top as usize;
        if let Some(f) = free.get_mut(r) {
            *f = true;
        }
        // Bounded so a corrupt free list can't loop forever.
        for _ in 0..cells.len() {
            let next = cells.get(r).map_or(0, |c| c.left as usize);
            if next == 0 || next >= cells.len() {
                break;
            }
            if let Some(f) = free.get_mut(next) {
                *f = true;
            }
            r = next;
        }

        for (i, (c, &is_free)) in cells.iter().zip(&free).enumerate() {
            let _ = write!(s, "{i:<7}");
            let r = i as Ref;
            if is_map_ref(r) {
                let _ = write!(s, "{:032b} M {:032b} M", c.left, c.right);
            } else if is_free {
                let _ = write!(s, "{:>32} F", c.left);
            } else {
                let (left_is_ref, right_is_ref) = self.ref_flags(r);
                write_half(&mut s, c.left, left_is_ref);
                s.push(' ');
                write_half(&mut s, c.right, right_is_ref);
            }
            s.push('\n');
        }

        s
    }

    /// Parses [`Map::debug_string`] output.
    ///
    /// Rejects malformed input and maps whose refs or free list are
    /// inconsistent, so a parsed map can't index out of bounds later.
    pub fn debug_parse(s: &str) -> Result<Map, DebugParseError> {
        let mut lines = s.lines();
        let header = lines.next().ok_or(DebugParseError::Header)?;
        let (free_top, alloc_count, cell_count, root_bin) =
            parse_header(header).ok_or(DebugParseError::Header)?;

        let mut cells = Vec::new();
        for (n, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            cells.push(parse_cell(line).ok_or(DebugParseError::Cell(n + 1))?);
        }

        let m = Map::from_raw_parts(free_top, alloc_count, cell_count, Bin(root_bin), cells);
        validate(&m)?;
        Ok(m)
    }
}

fn write_half(s: &mut String, v: u32, is_ref: bool) {
    if is_ref {
        let _ = write!(s, "{v:>32} R");
    } else {
        let _ = write!(s, "{v:032b} B");
    }
}

fn parse_header(line: &str) -> Option<(Ref, usize, usize, u64)> {
    let mut fields = line.split(", ");
    let mut field = |name: &str| {
        fields
            .next()?
            .strip_prefix(name)?
            .strip_prefix(": ")
            .map(str::trim)
    };
    let free_top = field("freeTop")?.parse().ok()?;
    let alloc_count = field("allocCount")?.parse().ok()?;
    let cell_count = field("cellCount")?.parse().ok()?;
    let root_bin = field("rootBin")?.parse().ok()?;
    Some((free_top, alloc_count, cell_count, root_bin))
}

/// Parses `index value tag [value tag]`. `F` and `R` values are decimal,
/// everything else binary, and a missing right half is 0.
fn parse_cell(line: &str) -> Option<Cell> {
    let mut tokens = line.split_whitespace();
    tokens.next()?.parse::<u64>().ok()?;

    let value = |v: &str, tag: &str| {
        if tag == "F" || tag == "R" {
            v.parse::<u32>().ok()
        } else {
            u32::from_str_radix(v, 2).ok()
        }
    };

    let (v, tag) = (tokens.next()?, tokens.next()?);
    let left = value(v, tag)?;
    let right = match (tokens.next(), tokens.next()) {
        (Some(v), Some(tag)) if tag != "F" => value(v, tag)?,
        (None, None) => 0,
        _ => return None,
    };
    tokens.next().is_none().then_some(Cell { left, right })
}

/// Checks that every ref reachable from the root and every free list entry
/// is an in-bounds data cell, and that the counters agree with them.
fn validate(m: &Map) -> Result<(), DebugParseError> {
    use DebugParseError::Inconsistent;

    let (free_top, alloc_count, cell_count, root_bin, cells) = m.raw_parts();
    let len = cells.len();
    let is_data_cell = |r: Ref| (r as usize) < len && !is_map_ref(r);

    // Data cells stand for bins of layer 6 and up.
    if !is_data_cell(1) || root_bin.is_none() || root_bin.layer() < 6 {
        return Err(Inconsistent("bad root"));
    }
    if (0..len).filter(|&i| !is_map_ref(i as Ref)).count() != cell_count {
        return Err(Inconsistent("cellCount"));
    }

    // The refs must form a tree of in-bounds data cells no deeper than layer
    // 6, or traces could run off the end or loop.
    let mut seen = vec![false; len];
    let mut stack = vec![(1 as Ref, root_bin.layer())];
    let mut allocated = 0;
    while let Some((r, layer)) = stack.pop() {
        let first_visit = seen
            .get_mut(r as usize)
            .map(|s| !core::mem::replace(s, true));
        if !is_data_cell(r) || layer < 6 || first_visit != Some(true) {
            return Err(Inconsistent("bad ref"));
        }
        allocated += 1;
        let (left_is_ref, right_is_ref) = m.ref_flags(r);
        let c = cells.get(r as usize).copied().unwrap_or_default();
        if left_is_ref {
            stack.push((c.left, layer - 1));
        }
        if right_is_ref {
            stack.push((c.right, layer - 1));
        }
    }
    if allocated != alloc_count || alloc_count > cell_count {
        return Err(Inconsistent("allocCount"));
    }

    // The free list must hold exactly the unallocated cells.
    let mut r = free_top;
    for _ in 0..cell_count - alloc_count {
        let first_visit = seen
            .get_mut(r as usize)
            .map(|s| !core::mem::replace(s, true));
        if !is_data_cell(r) || first_visit != Some(true) {
            return Err(Inconsistent("bad free list"));
        }
        r = cells.get(r as usize).map_or(0, |c| c.left);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dump after `set(0)`, `set(200)`, `set(4000)` on a new map.
    const DUMP: &str = "\
freeTop: 13, allocCount: 12, cellCount: 15, rootBin: 2047
0      00000000000000000000000011110010 M 00000000000000000000111100010010 M
1                                     7 R                                8 R
2      00000000000000000000000000000001 B 00000000000000000000000000000000 B
3      00000000000000000000000000000000 B 00000000000000000000000000010000 B
4                                     2 R                                3 R
5                                     4 R 00000000000000000000000000000000 B
6                                     5 R 00000000000000000000000000000000 B
7                                     6 R 00000000000000000000000000000000 B
8      00000000000000000000000000000000 B                                9 R
9      00000000000000000000000000000000 B                               10 R
10     00000000000000000000000000000000 B                               11 R
11     00000000000000000000000000000000 B                               12 R
12     00000000000000010000000000000000 B 00000000000000000000000000000000 B
13                                   14 F
14                                   15 F
15                                    0 F
";

    #[test]
    fn dump_format() {
        let mut m = Map::new();
        m.set(Bin(0));
        m.set(Bin(200));
        m.set(Bin(4000));
        assert_eq!(m.debug_string(), DUMP);
    }

    #[test]
    fn parse_round_trip() -> Result<(), DebugParseError> {
        let m = Map::debug_parse(DUMP)?;
        assert_eq!(m.debug_string(), DUMP);
        assert!(m.filled_at(Bin(0)));
        assert!(m.filled_at(Bin(200)));
        assert!(m.filled_at(Bin(4000)));
        assert!(m.empty_at(Bin(2)));
        Ok(())
    }

    #[test]
    fn parse_rejects_bad_refs() {
        let bad = DUMP.replace(
            "1                                     7 R",
            "1                                    99 R",
        );
        assert!(matches!(
            Map::debug_parse(&bad),
            Err(DebugParseError::Inconsistent(_))
        ));
        assert_eq!(
            Map::debug_parse("nonsense").err(),
            Some(DebugParseError::Header)
        );
    }
}
