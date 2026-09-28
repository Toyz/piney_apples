//! The small helpers of toppage.cpp and bbs.cpp: the key repeat the menu
//! and the board share, the board's scroll bar, and its text measuring.

use piney_input::Pad;

/// `g_TP_pushFlag` and `g_TP_pushCntr` (main .sdata 0x00378e1c,
/// 0x00378e20) with `ccIsKeyRepeat(key, n)` (toppage.prg 0x004016a0).
///
/// The counter lives in the main executable: it is not reset when the
/// top page is entered, so a count left by the last visit carries over
/// (it starts at 0 at boot).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyRepeat {
    /// Set by a push, cleared by a release; nothing reads it.
    pub push_flag: i32,
    /// Counts the repeat bits of every key tested; `n` of them is a move.
    pub push_cntr: i32,
}

impl KeyRepeat {
    /// `ccIsKeyRepeat(key, n)`: a push moves at once; a release clears the
    /// flag; each frame the repeat bit is up counts, and the `n`th count
    /// moves and starts again.
    pub fn is_key_repeat(&mut self, pad: &Pad, key: u32, n: i32) -> bool {
        if pad.push.bits() & key != 0 {
            self.push_flag = 1;
            return true;
        }
        if pad.unpush.bits() & key != 0 {
            self.push_flag = 0;
            return false;
        }
        if pad.repeat.bits() & key != 0 {
            self.push_cntr = self.push_cntr.wrapping_add(1);
            if self.push_cntr == n {
                self.push_cntr = 0;
                return true;
            }
        }
        false
    }
}

/// The colour escapes `ccStrRangeCopy` and `GetLineLengthNoColor` step
/// over: `#W`, `#Y`, `#B`, `#G`, `#R`.
fn colour_code(c: u8) -> bool {
    matches!(c, b'W' | b'Y' | b'B' | b'G' | b'R')
}

fn at(s: &[u8], i: i32) -> u8 {
    if i < 0 { 0 } else { s.get(i as usize).copied().unwrap_or(0) }
}

/// `ccStrRangeCopy(dst, src, start, count)` (toppage.prg 0x00401730): bytes
/// `2 start .. 2 (start + count)` of `src`, stopping at its end; a colour
/// escape is copied whole and does not count.
pub fn str_range_copy(src: &[u8], start: i32, count: i32) -> Vec<u8> {
    let mut t = start.wrapping_mul(2);
    let mut end = t.wrapping_add(count.wrapping_mul(2));
    let mut out = Vec::new();
    while t < end {
        let c = at(src, t);
        if c == 0 {
            break;
        }
        out.push(c);
        if c == b'#' && t < end - 1 {
            let d = at(src, t + 1);
            if colour_code(d) {
                out.push(d);
                t += 2;
                end += 2;
                continue;
            }
        }
        t += 1;
    }
    out
}

/// `ccBBSMsgObj::GetLineLengthNoColor` (0x00404610) on the line's text:
/// its bytes, a colour escape counting none.
pub fn line_length_no_color(s: &[u8]) -> i32 {
    let len = s.iter().position(|&c| c == 0).unwrap_or(s.len()) as i32;
    let (mut i, mut n) = (0, 0);
    while i < len {
        if at(s, i) == b'#' && i < len - 1 && colour_code(at(s, i + 1)) {
            i += 2;
        } else {
            i += 1;
            n += 1;
        }
    }
    n
}

/// The EE's `div`: truncating, and by zero the quotient the EE leaves in
/// LO, -1 for a non-negative dividend and 1 for a negative one (as PCSX2
/// has it from hardware). The board's scroll bars divide by zero when a
/// list holds exactly a page.
pub fn ee_div(a: i32, b: i32) -> i32 {
    if b == 0 { if a < 0 { 1 } else { -1 } } else { a.wrapping_div(b) }
}

/// `ccScrollBar` (bbs.cpp): a bar `sLength` long at `sPos`, `nPage` rows a
/// page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollBar {
    pub s_pos: i32,
    pub s_length: i32,
    /// The box: its offset down the bar and its length.
    pub b_pos: i32,
    pub b_length: i32,
    pub l_pos: i32,
    pub lmin: i32,
    pub lmax: i32,
    pub n_page: i32,
}

impl ScrollBar {
    /// `Init(sPos, sLength, nPage)` (0x00404880).
    pub fn init(&mut self, s_pos: i32, s_length: i32, n_page: i32) {
        self.s_pos = s_pos;
        self.s_length = s_length;
        self.n_page = n_page;
    }

    /// `SetRange(lmin, lmax)` (0x004048a0): the box is the whole bar for a
    /// list shorter than a page, else `nPage sLength / range`.
    pub fn set_range(&mut self, lmin: i32, lmax: i32) {
        self.lmin = lmin;
        self.lmax = lmax;
        let range = lmax.wrapping_sub(lmin);
        self.b_length = if range <= 0 || range < self.n_page {
            self.s_length
        } else {
            ee_div(self.n_page.wrapping_mul(self.s_length), range)
        };
    }

    /// `SetPos(lpos)` (0x00404900): `(sLength - bLength) lpos / (range -
    /// nPage)`.
    pub fn set_pos(&mut self, lpos: i32) {
        self.l_pos = lpos;
        let range = self.lmax.wrapping_sub(self.lmin);
        self.b_pos = if range <= 0 {
            0
        } else {
            ee_div(self.s_length.wrapping_sub(self.b_length).wrapping_mul(lpos), range.wrapping_sub(self.n_page))
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piney_input::Buttons;

    #[test]
    fn range_copy_skips_colours() {
        assert_eq!(str_range_copy(b"abcdef", 0, 1), b"ab");
        assert_eq!(str_range_copy(b"#Gabcd", 0, 1), b"#Gab");
        assert_eq!(str_range_copy(b"ab#Gcd", 0, 2), b"ab#Gcd");
        assert_eq!(str_range_copy(b"a#xbcd", 0, 2), b"a#xb");
        assert_eq!(str_range_copy(b"abc", 0, 5), b"abc");
        assert_eq!(str_range_copy(b"abcdef", 1, 1), b"cd");
    }

    #[test]
    fn length_without_colours() {
        assert_eq!(line_length_no_color(b"abc"), 3);
        assert_eq!(line_length_no_color(b"#Rab#W"), 2);
        assert_eq!(line_length_no_color(b"a#"), 2);
        assert_eq!(line_length_no_color(b"#q"), 2);
    }

    #[test]
    fn scroll_bar_page_edge() {
        let mut s = ScrollBar::default();
        s.init(62, 329, 18);
        s.set_range(0, 10);
        assert_eq!(s.b_length, 329);
        s.set_pos(3);
        assert_eq!(s.b_pos, 0);
        s.set_range(0, 36);
        assert_eq!(s.b_length, 164);
        s.set_pos(18);
        assert_eq!(s.b_pos, 165);
        // Exactly a page: 0 / 0 on the EE.
        s.set_range(0, 18);
        s.set_pos(0);
        assert_eq!((s.b_length, s.b_pos), (329, -1));
    }

    #[test]
    fn key_repeat_counts_every_key() {
        let mut k = KeyRepeat::default();
        let held = Pad { direct: Buttons::UP, repeat: Buttons::UP, ..Pad::default() };
        let push = Pad { push: Buttons::UP, ..held };
        assert!(k.is_key_repeat(&push, Buttons::UP.bits(), 6));
        let moves: Vec<bool> = (0..12).map(|_| k.is_key_repeat(&held, Buttons::UP.bits(), 6)).collect();
        assert_eq!(moves.iter().filter(|&&m| m).count(), 2);
        assert!(moves[5] && moves[11]);
        let up = Pad { unpush: Buttons::UP, ..Pad::default() };
        assert!(!k.is_key_repeat(&up, Buttons::UP.bits(), 6));
        assert_eq!(k.push_flag, 0);
    }
}
