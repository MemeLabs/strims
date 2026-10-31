// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A CEA-608 decoder of caption channel 1 (CC1) to plain text.
//!
//! It takes field 1's byte pairs ([`CcTuple`]s of `cc_type` 0) a frame at
//! a time, in presentation order, and says when the caption on screen
//! changes, as the text of its rows from top to bottom, joined by LF,
//! each without the spaces around it.
//!
//! - **Channels.** A control code of data channel 2 (CC2) selects it, and
//!   its characters are dropped until a code of channel 1. Text mode
//!   (`TR`, `RTD`) and its characters are dropped.
//! - **Modes.** Pop-on (`RCL`) loads a hidden memory, which end of
//!   caption (`EOC`) swaps onto the screen. Roll-up (`RU2` to `RU4`) types
//!   on the base row, and carriage return (`CR`) scrolls a window of 2 to 4
//!   rows up. Paint-on (`RDC`) types on the screen. A change typed on the
//!   screen shows at the next `CR` or [`TYPED_SHOW`] after it; erasing
//!   the screen (`EDM`) shows at once.
//! - **Codes.** Preamble address codes set the row and indent; tab offsets
//!   move right; a mid-row code is a space; backspace, delete to end of
//!   row and erasing the hidden memory edit as they say. Special and
//!   extended characters are read, an extended one in place of the
//!   character before it. Styles, colours and attributes are dropped.
//! - **Repeats.** Control codes, special and extended characters among
//!   them, come twice: a code that repeats the one in the frame before, or
//!   earlier in its own frame, with nothing between, is dropped.
//! - **Errors.** A pair whose byte fails its odd parity is dropped.
//!
//! It works in time linear in its input and holds a screen and a hidden
//! memory of [`ROWS`] by [`COLUMNS`] characters.

use strims_base::time::Duration;

use crate::captions::CcTuple;

/// A caption screen's rows and columns.
pub const ROWS: usize = 15;
pub const COLUMNS: usize = 32;
/// How long a change typed on the screen in roll-up or paint-on waits
/// for a carriage return before it shows.
pub const TYPED_SHOW: Duration = Duration::from_millis(500);

/// `cc_type` of a 608 pair of field 1, in a tuple's first byte.
const FIELD_1: u8 = 0;

/// What the decoder writes into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    PopOn,
    /// The roll-up window's rows.
    RollUp(usize),
    PaintOn,
    /// The text service: dropped.
    Text,
}

/// A caption memory: rows of characters, `' '` where empty.
type Memory = [[char; COLUMNS]; ROWS];

const BLANK: Memory = [[' '; COLUMNS]; ROWS];

/// Decodes CC1 ([module docs](self)).
#[derive(Debug, Clone)]
pub struct Cc1Decoder {
    screen: Memory,
    hidden: Memory,
    mode: Mode,
    row: usize,
    column: usize,
    /// The data channel the last control code selected.
    channel: u8,
    /// The last control code, until a pair or a frame without pairs
    /// comes between.
    last_control: Option<[u8; 2]>,
    /// When the screen was typed on, since it last showed.
    typed: Option<Duration>,
    /// A change shows at the frame's end.
    show: bool,
}

impl Default for Cc1Decoder {
    fn default() -> Self {
        Cc1Decoder::new()
    }
}

impl Cc1Decoder {
    pub fn new() -> Cc1Decoder {
        Cc1Decoder {
            screen: BLANK,
            hidden: BLANK,
            mode: Mode::PopOn,
            row: ROWS - 1,
            column: 0,
            channel: 1,
            last_control: None,
            typed: None,
            show: false,
        }
    }

    /// Takes a frame's tuples, the frame presented at `at`: the caption
    /// on screen when it changed, an empty one when it went.
    pub fn frame(&mut self, at: Duration, tuples: &[CcTuple]) -> Option<String> {
        let mut pairs = 0;
        for &[head, a, b] in tuples {
            if head & 0x03 != FIELD_1 {
                continue;
            }
            pairs += 1;
            self.pair(at, a, b);
        }
        if pairs == 0 {
            self.last_control = None;
        }
        if self.typed.is_some_and(|t| at - t >= TYPED_SHOW) {
            self.show = true;
        }
        if !std::mem::take(&mut self.show) {
            return None;
        }
        self.typed = None;
        Some(text(&self.screen))
    }

    fn pair(&mut self, at: Duration, a: u8, b: u8) {
        if !odd(a) || !odd(b) {
            self.last_control = None;
            return;
        }
        let (a, b) = (a & 0x7f, b & 0x7f);
        match a {
            0x00 => {}
            0x01..=0x0f => self.last_control = None,
            0x10..=0x1f => {
                if self.last_control.take() == Some([a, b]) {
                    return;
                }
                self.last_control = Some([a, b]);
                self.channel = if a & 0x08 == 0 { 1 } else { 2 };
                if self.channel == 1 {
                    self.control(at, a, b);
                }
            }
            _ => {
                self.last_control = None;
                if self.channel == 1 {
                    self.put(at, standard(a));
                    if b >= 0x20 {
                        self.put(at, standard(b));
                    }
                }
            }
        }
    }

    fn control(&mut self, at: Duration, a: u8, b: u8) {
        match (a, b) {
            (0x11, 0x20..=0x2f) => self.put(at, ' '),
            (0x11, 0x30..=0x3f) => {
                if let Some(&c) = SPECIAL.get(usize::from(b - 0x30)) {
                    self.put(at, c);
                }
            }
            (0x12 | 0x13, 0x20..=0x3f) => {
                let table = if a == 0x12 {
                    &EXTENDED_12
                } else {
                    &EXTENDED_13
                };
                if let Some(&c) = table.get(usize::from(b - 0x20)) {
                    self.column = self.column.saturating_sub(1);
                    self.put(at, c);
                }
            }
            (0x14 | 0x15, 0x20..=0x2f) => self.command(at, b),
            (0x17, 0x21..=0x23) => {
                self.column = (self.column + usize::from(b - 0x20)).min(COLUMNS - 1);
            }
            (0x10..=0x17, 0x40..=0x7f) => self.preamble(a, b),
            _ => {}
        }
    }

    /// A miscellaneous control code.
    fn command(&mut self, at: Duration, b: u8) {
        match b {
            // RCL
            0x20 => self.mode = Mode::PopOn,
            // BS
            0x21 => {
                if self.column > 0 && self.mode != Mode::Text {
                    self.column -= 1;
                    let (row, column) = (self.row, self.column);
                    if let Some(c) = self.target().get_mut(row).and_then(|r| r.get_mut(column)) {
                        *c = ' ';
                    }
                    self.typed_on(at);
                }
            }
            // DER
            0x24 if self.mode != Mode::Text => {
                let (row, column) = (self.row, self.column);
                if let Some(r) = self.target().get_mut(row) {
                    for c in r.iter_mut().skip(column) {
                        *c = ' ';
                    }
                }
                self.typed_on(at);
            }
            // RU2, RU3, RU4
            0x25..=0x27 => {
                let rows = usize::from(b - 0x23);
                if !matches!(self.mode, Mode::RollUp(_)) {
                    self.screen = BLANK;
                    self.hidden = BLANK;
                    self.show = true;
                    self.row = ROWS - 1;
                }
                self.mode = Mode::RollUp(rows);
                self.column = 0;
            }
            // RDC
            0x29 => self.mode = Mode::PaintOn,
            // TR, RTD
            0x2a | 0x2b => self.mode = Mode::Text,
            // EDM
            0x2c => {
                self.screen = BLANK;
                self.show = true;
            }
            // CR
            0x2d => {
                if let Mode::RollUp(rows) = self.mode {
                    self.roll(rows);
                    self.show = true;
                }
                self.column = 0;
            }
            // ENM
            0x2e => self.hidden = BLANK,
            // EOC
            0x2f => {
                std::mem::swap(&mut self.screen, &mut self.hidden);
                self.mode = Mode::PopOn;
                self.show = true;
            }
            _ => {}
        }
    }

    /// A preamble address code: its row, and its indent's column.
    fn preamble(&mut self, a: u8, b: u8) {
        let pair = match a {
            0x11 => 0,
            0x12 => 2,
            0x15 => 4,
            0x16 => 6,
            0x17 => 8,
            0x10 if b < 0x60 => 10,
            0x13 => 11,
            0x14 => 13,
            _ => return,
        };
        let row = pair + usize::from(b & 0x20 != 0);
        if let Mode::RollUp(rows) = self.mode
            && row != self.row
        {
            // The window moves to its new base row.
            let mut moved = BLANK;
            for k in 0..rows {
                if let (Some(from), Some(to)) = (self.row.checked_sub(k), row.checked_sub(k))
                    && let (Some(src), Some(dst)) = (self.screen.get(from), moved.get_mut(to))
                {
                    *dst = *src;
                }
            }
            self.screen = moved;
        }
        self.row = row.min(ROWS - 1);
        self.column = if b & 0x10 != 0 {
            usize::from((b & 0x0e) >> 1) * 4
        } else {
            0
        };
    }

    /// Scrolls the roll-up window of `rows` up a row, leaving the base row
    /// empty and nothing above the window.
    fn roll(&mut self, rows: usize) {
        let base = self.row.max(rows - 1);
        let top = base + 1 - rows;
        for r in 0..ROWS {
            let next = if r >= top && r < base {
                self.screen.get(r + 1).copied().unwrap_or([' '; COLUMNS])
            } else {
                [' '; COLUMNS]
            };
            if let Some(row) = self.screen.get_mut(r) {
                *row = next;
            }
        }
        self.row = base;
    }

    /// The memory characters go into: the hidden one in pop-on.
    fn target(&mut self) -> &mut Memory {
        match self.mode {
            Mode::PopOn => &mut self.hidden,
            _ => &mut self.screen,
        }
    }

    fn typed_on(&mut self, at: Duration) {
        if matches!(self.mode, Mode::RollUp(_) | Mode::PaintOn) {
            self.typed.get_or_insert(at);
        }
    }

    fn put(&mut self, at: Duration, c: char) {
        if self.mode == Mode::Text {
            return;
        }
        let (row, column) = (self.row, self.column);
        if let Some(cell) = self.target().get_mut(row).and_then(|r| r.get_mut(column)) {
            *cell = c;
        }
        self.column = (column + 1).min(COLUMNS - 1);
        self.typed_on(at);
    }
}

/// Whether `b` has odd parity.
fn odd(b: u8) -> bool {
    b.count_ones() % 2 == 1
}

/// The rows of `m` with characters, each trimmed, joined by LF.
fn text(m: &Memory) -> String {
    let mut out = String::new();
    for row in m {
        let line: String = row.iter().collect();
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
    }
    out
}

/// The standard character `b`, 0x20 to 0x7f: ASCII but for nine.
fn standard(b: u8) -> char {
    match b {
        0x2a => 'á',
        0x5c => 'é',
        0x5e => 'í',
        0x5f => 'ó',
        0x60 => 'ú',
        0x7b => 'ç',
        0x7c => '÷',
        0x7d => 'Ñ',
        0x7e => 'ñ',
        0x7f => '█',
        b => char::from(b),
    }
}

/// The special characters, `0x11 0x30` to `0x11 0x3f`; `0x39` is a
/// transparent space.
const SPECIAL: [char; 16] = [
    '®', '°', '½', '¿', '™', '¢', '£', '♪', 'à', ' ', 'è', 'â', 'ê', 'î', 'ô', 'û',
];

/// The extended characters, `0x12 0x20` to `0x12 0x3f`: Spanish, French
/// and miscellaneous.
const EXTENDED_12: [char; 32] = [
    'Á', 'É', 'Ó', 'Ú', 'Ü', 'ü', '‘', '¡', '*', '’', '—', '©', '℠', '•', '“', '”', 'À', 'Â', 'Ç',
    'È', 'Ê', 'Ë', 'ë', 'Î', 'Ï', 'ï', 'Ô', 'Ù', 'ù', 'Û', '«', '»',
];

/// The extended characters, `0x13 0x20` to `0x13 0x3f`: Portuguese,
/// German and Danish.
const EXTENDED_13: [char; 32] = [
    'Ã', 'ã', 'Í', 'Ì', 'ì', 'Ò', 'ò', 'Õ', 'õ', '{', '}', '\\', '^', '_', '|', '~', 'Ä', 'ä', 'Ö',
    'ö', 'ß', '¥', '¤', '¦', 'Å', 'å', 'Ø', 'ø', '┌', '┐', '└', '┘',
];

#[cfg(test)]
#[path = "cea608_test.rs"]
mod tests;
