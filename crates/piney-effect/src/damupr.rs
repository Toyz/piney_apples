//! The damage and recovery numbers that stack over a character (GCMN.PRG
//! gamectrl.cpp): `ccDamUprStr` (a list from `root`, main .sbss
//! 0x00378c80, a node per character) and its `ccUprollStr` (sixteen lines
//! rolling up), and the `ccEntryFlyFontNew*` functions the battle code
//! calls.
//!
//! ```text
//! ccEntryFlyFontNew(kind, v, pos, ch, sx, sy)  (gcmn 0x0051afc0; pos, sx,
//!                                   sy unused)
//!   v -1:  AddStr(ffstrMISS "+,--", 0, 3, ch, 1, 1)             MISS
//!   else:  AddStr(Int2StrFF(v, 5), kind, 3, ch, s, s), s 1.6 from 1000,
//!          1.3 from 100, else 1
//! ccEntryFlyFontNewExp(kind, v, pos, ch)   Int2StrFF(v, 5) + "./0" (EXP)
//! ccEntryFlyFontNewLevelDown(kind, pos, ch)  "1.2.1 3456" (LEVEL DOWN)
//! ccEntryFlyFontNewMiss(pos, ch)           "+,--", colour 0
//!   (font type 3, the big digits: byte 0x20 + n is cell n of
//!    " 0123456789MISEXPLVDOWN-")
//! ccDamUprStr::AddStr(str, col, ftype, ch, sx, sy)  (0x0051bb80)
//!   the node of ch (a new one at the head of the list when none), its
//!   ccUprollStr::AddStr (0x0051b140), dispSw 0 (drawn once CtrlAll has
//!   placed it)
//! ccUprollStr::AddStr: lineTop on (mod 16), lineNum up to 16; the new
//!   line: str (15 bytes at most), alphaCnt 24, alpha 128, colour, ftype;
//!   the global font's SetType(ftype & 15) for the cell size:
//!   w = fptosi(sx * (su << 4)), h = fptosi(sy * (sv << 4)) (1/16 px);
//!   lx = -(len w) / 2, ly 0, addly 288; the line that was on top gets
//!   addly (h + 16) - ly when its ly is less
//! CtrlAll (0x0051b780), each node from the head:
//!   no ch: nothing
//!   ccCheckTarget(ch) fails: Ctrl while it has lines, else the node is
//!     deleted
//!   ch->base->type & 2 (a party member):
//!     camera camID 1 of type 1 (the party view): x = slot 170 + 60,
//!       y = 360 - ((ccConditionIconNum(ch) + 4) / 5) 32, shown
//!     else: the point is ch's pos, no lift
//!   else the point is ch's pos with z + 0.9 height, 16 px lower
//!   ccCalcTagPosChar(ch, (0, 0, 160), 1) not 1 (off the screen or behind):
//!     hidden; else the point through sceVu0RotTransPers (sysLayer's
//!     view), converted as the fly fonts do when the depth is in range
//!     (the raw fixed point when not), x clamped to 26..486, y to 90..436,
//!     plus the lift; shown
//!   then ccUprollStr::Ctrl (0x0051b530): each line from the top: ly on by
//!   addly / 4 (at least 1); the lines' ly summed; alphaCnt down one: alpha
//!   128, then (alphaCnt + 8) 128 / 8 over 8 frames, then 0; a line whose
//!   sum reaches 2561 (160 px) and those under it stop, from 1281 the
//!   alpha loses (sum - 1280) / 10; lineNum becomes the lines still seen
//! DrawAll (0x0051bb20) each node's ccUprollStr::Draw (0x0051b320), when
//!   dispSw and lines: from the top line down, SetType(ftype & 15), scaled
//!   by the line's sx, sy, shadowed, its colour and alpha, at
//!   ((x << 4) + lx) >> 4, ((y << 4) - (the lines' ly so far)) >> 4 - sy
//! ```

use piney_desktop::view::LayerView;

use crate::ee::{self, F, ONE, V4};
use crate::flyfont::{self, Font, Strings};
use crate::{CharRef, Host};

/// `ccUprollStrLine`'s lines.
pub const LINES: usize = 16;
/// A new line's rise, 18 px in 1/16 px.
const ADD_LY: i16 = 288;
/// `ccEntryFlyFontNew`'s scales: 1.6 from 1000, 1.3 from 100.
const SCALE_1000: F = 0x3fcc_cccd;
const SCALE_100: F = 0x3fa6_6666;
/// 0.9 of the height, where a non-member's numbers start.
const LIFT: F = 0x3f66_6666;
/// `ccCalcTagPosChar`'s offset: 160 over the feet.
const TAG_HEIGHT: F = 0x4320_0000;

/// A `ccUprollStrLine` (0x2c bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// +0x00 str[16]: up to 15 bytes.
    pub text: Vec<u8>,
    /// +0x10 color (a `ccSpriteColorTable` row), +0x14 alphaCnt.
    pub color: i32,
    pub alpha_cnt: i32,
    /// +0x18 alpha, +0x1a lx, +0x1c ly, +0x1e addly, +0x20 ftype.
    pub alpha: i16,
    pub lx: i16,
    pub ly: i16,
    pub addly: i16,
    pub ftype: i16,
    /// +0x24 sx, +0x28 sy.
    pub sx: F,
    pub sy: F,
}

impl Default for Line {
    /// As `ccUprollStr`'s constructor (gcmn 0x0051b0b0) leaves it.
    fn default() -> Line {
        Line { text: Vec::new(), color: 2, alpha_cnt: 0, alpha: 0, lx: 0, ly: 0, addly: 0, ftype: 0, sx: ONE, sy: ONE }
    }
}

/// A `ccUprollStr` (0x2cc bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Uproll {
    pub lines: [Line; LINES],
    /// +0x2c0 lineNum, +0x2c2 lineTop, +0x2c4 x, +0x2c6 y, +0x2c8 dispSw.
    pub line_num: i16,
    pub line_top: i16,
    pub x: i16,
    pub y: i16,
    pub disp_sw: bool,
}

impl Default for Uproll {
    fn default() -> Uproll {
        Uproll { lines: Default::default(), line_num: 0, line_top: 0, x: 0, y: 0, disp_sw: true }
    }
}

fn prev(i: usize) -> usize {
    if i == 0 { LINES - 1 } else { i - 1 }
}

impl Uproll {
    /// `ccUprollStr::AddStr(str, col, ftype, sx, sy)` (gcmn 0x0051b140);
    /// `font` is the global `font` (its type changes).
    pub fn add_str(&mut self, s: &[u8], color: i32, ftype: i32, sx: F, sy: F, font: &mut Font) {
        let old = self.line_top as usize % LINES;
        self.line_top = if self.line_top < 15 { self.line_top + 1 } else { 0 };
        self.line_num = if self.line_num < 16 { self.line_num + 1 } else { 16 };
        let top = self.line_top as usize % LINES;
        let text: Vec<u8> = s.iter().copied().take_while(|&c| c != 0).take(15).collect();
        let len = text.len() as i32;
        font.set_type(ftype & 15);
        let w = ee::to_int(ee::mul(sx, ee::from_int(font.su << 4)));
        let h = ee::to_int(ee::mul(sy, ee::from_int(font.sv << 4)));
        let l = &mut self.lines[top];
        l.text = text;
        l.alpha_cnt = 24;
        l.alpha = 128;
        l.color = color;
        l.ftype = ftype as i16;
        l.lx = ((-len.wrapping_mul(w)) / 2) as i16;
        l.ly = 0;
        l.sx = sx;
        l.sy = sy;
        let o = &mut self.lines[old];
        if i32::from(o.ly) < h + 16 {
            o.addly = (h + 16 - i32::from(o.ly)) as i16;
        }
        self.lines[top].addly = ADD_LY;
    }

    /// `ccUprollStr::Ctrl()` (gcmn 0x0051b530).
    pub fn ctrl(&mut self) {
        if self.line_num == 0 {
            return;
        }
        let mut i = self.line_top as usize % LINES;
        let mut sum: i32 = 0;
        let mut seen: i16 = 0;
        for k in 0..self.line_num {
            let l = &mut self.lines[i];
            if l.addly != 0 {
                let mut step = l.addly >> 2;
                if step == 0 {
                    step = 1;
                }
                l.ly = l.ly.wrapping_add(step);
                l.addly = l.addly.wrapping_sub(step);
            }
            sum = sum.wrapping_add(i32::from(l.ly));
            l.alpha_cnt = l.alpha_cnt.wrapping_sub(1);
            let mut a = if l.alpha_cnt < -8 {
                0
            } else if l.alpha_cnt < 0 {
                seen = k + 1;
                ((l.alpha_cnt + 8) << 7) / 8
            } else {
                seen = k + 1;
                128
            };
            if sum >= 2561 {
                break;
            }
            if sum >= 1281 {
                a -= ((sum - 1280) << 3) / 80;
                if a < 0 {
                    a = 0;
                } else {
                    seen = k + 1;
                }
            }
            l.alpha = a as i16;
            i = prev(i);
        }
        self.line_num = seen;
    }

    /// `ccUprollStr::Draw()` (gcmn 0x0051b320) into `font`'s queue.
    pub fn draw(&self, font: &mut Font, view: &LayerView) {
        if !self.disp_sw || self.line_num == 0 {
            return;
        }
        let mut i = self.line_top as usize % LINES;
        let x16 = i32::from(self.x) << 4;
        let mut y16 = i32::from(self.y) << 4;
        for _ in 0..self.line_num {
            let l = &self.lines[i];
            font.set_type(i32::from(l.ftype) & 15);
            font.scale(l.sx, l.sy);
            font.ctrl |= 0x10;
            font.set_colour(l.color);
            y16 = y16.wrapping_sub(i32::from(l.ly));
            let y = (y16 >> 4).wrapping_sub(ee::to_int(font.sy));
            let x = x16.wrapping_add(i32::from(l.lx)) >> 4;
            font.dx = ee::from_int(x);
            font.dy = ee::from_int(y);
            font.set_alpha(i32::from(l.alpha));
            font.make_packet_str(&l.text, view);
            i = prev(i);
        }
    }
}

/// A `ccDamUprStr` node (0x2d8 bytes): +0x08 ch, +0x0c its lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub ch: Option<CharRef>,
    pub uproll: Uproll,
}

/// `ccDamUprStr::root`'s list, the head (the newest node) first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DamUprStr {
    pub nodes: Vec<Node>,
}

impl DamUprStr {
    /// `ccDamUprStr::ResetAll()` (gcmn 0x0051b710, from `ccMenuCtrl`'s
    /// destructor): every node deleted.
    pub fn reset_all(&mut self) {
        self.nodes.clear();
    }

    /// `ccDamUprStr::AddStr(str, col, ftype, ch, sx, sy)` (gcmn 0x0051bb80):
    /// the node's index.
    #[allow(clippy::too_many_arguments)]
    pub fn add_str(
        &mut self,
        s: &[u8],
        color: i32,
        ftype: i32,
        ch: Option<CharRef>,
        sx: F,
        sy: F,
        font: &mut Font,
    ) -> usize {
        let i = match self.nodes.iter().position(|n| n.ch == ch) {
            Some(i) => i,
            None => {
                self.nodes.insert(0, Node { ch, uproll: Uproll::default() });
                0
            }
        };
        let n = &mut self.nodes[i];
        n.uproll.add_str(s, color, ftype, sx, sy, font);
        n.uproll.disp_sw = false;
        i
    }

    /// `ccDamUprStr::CtrlAll()` (gcmn 0x0051b780); `world_screen` is
    /// sysLayer's view's.
    pub fn ctrl_all(&mut self, host: &dyn Host, world_screen: &[V4; 4]) {
        let mut i = 0;
        while i < self.nodes.len() {
            let n = &mut self.nodes[i];
            let Some(ch) = n.ch else {
                i += 1;
                continue;
            };
            if !host.check_target(ch) {
                if n.uproll.line_num == 0 {
                    self.nodes.remove(i);
                    continue;
                }
                n.uproll.ctrl();
                i += 1;
                continue;
            }
            place(&mut n.uproll, host, ch, world_screen);
            n.uproll.ctrl();
            i += 1;
        }
    }

    /// `ccDamUprStr::DrawAll()` (gcmn 0x0051bb20) into `font`'s queue.
    pub fn draw_all(&self, font: &mut Font, view: &LayerView) {
        for n in &self.nodes {
            n.uproll.draw(font, view);
        }
    }
}

/// CtrlAll's placing of a node over its character.
fn place(u: &mut Uproll, host: &dyn Host, ch: CharRef, world_screen: &[V4; 4]) {
    let mut lift = 0;
    let mut p = host.char_pos(ch);
    if host.char_type(ch) & 2 != 0 {
        if host.camera_id() == 1 && host.camera_type() == 1 {
            let n = host.party_slot(ch);
            let k = host.condition_icon_num(ch).wrapping_add(4);
            u.x = n.wrapping_mul(170).wrapping_add(60) as i16;
            u.y = 360i32.wrapping_sub((k / 5).wrapping_mul(32)) as i16;
            u.disp_sw = true;
            return;
        }
    } else {
        p[2] = ee::add(p[2], ee::mul(LIFT, host.char_height(ch)));
        lift = 16;
    }
    let tag = piney_world::char::calc_tag_pos(world_screen, host.char_pos(ch), [0, 0, TAG_HEIGHT, ONE], 1);
    if !matches!(tag, Some((_, _, 1))) {
        u.disp_sw = false;
        return;
    }
    p = ee::vadd(p, [0; 4]);
    p[3] = ONE;
    let v = ee::rot_trans_pers(world_screen, p);
    let (x, y) = if v[2] > 0 && v[2] < 0x0fff_ffff { flyfont::screen(v) } else { (v[0], v[1]) };
    u.x = x.clamp(26, 486) as i16;
    u.y = y.clamp(90, 436).wrapping_add(lift) as i16;
    u.disp_sw = true;
}

/// The `ccEntryFlyFontNew*` functions: what the battle code calls.
impl DamUprStr {
    /// `ccEntryFlyFontNew(kind, value, pos, ch, sx, sy)` (gcmn 0x0051afc0):
    /// the number (in the big digits, scaled by its size) or MISS for -1.
    pub fn entry_new(&mut self, s: &Strings, kind: i32, value: i32, ch: Option<CharRef>, font: &mut Font) -> usize {
        if value == -1 {
            return self.add_str(&s.ff_miss, 0, 3, ch, ONE, ONE, font);
        }
        let text = flyfont::int2str_ff(&s.pow10, value, 5);
        let k = if value >= 1000 {
            SCALE_1000
        } else if value >= 100 {
            SCALE_100
        } else {
            ONE
        };
        self.add_str(&text, kind, 3, ch, k, k, font)
    }

    /// `ccEntryFlyFontNewExp(kind, value, pos, ch)` (gcmn 0x0051aea0):
    /// the number and EXP.
    pub fn entry_exp(&mut self, s: &Strings, kind: i32, value: i32, ch: Option<CharRef>, font: &mut Font) -> usize {
        let mut text = flyfont::int2str_ff(&s.pow10, value, 5);
        text.extend_from_slice(&s.ff_exp);
        self.add_str(&text, kind, 3, ch, ONE, ONE, font)
    }

    /// `ccEntryFlyFontNewLevelDown(kind, pos, ch)` (gcmn 0x0051af20).
    pub fn entry_level_down(&mut self, s: &Strings, kind: i32, ch: Option<CharRef>, font: &mut Font) -> usize {
        self.add_str(&s.ff_level_down, kind, 3, ch, ONE, ONE, font)
    }

    /// `ccEntryFlyFontNewMiss(pos, ch)` (gcmn 0x0051af70).
    pub fn entry_miss(&mut self, s: &Strings, ch: Option<CharRef>, font: &mut Font) -> usize {
        self.add_str(&s.ff_miss, 0, 3, ch, ONE, ONE, font)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_roll_up_and_fade() {
        let s = Strings {
            miss: b"MISS".to_vec(),
            ff_miss: b"+,--".to_vec(),
            ff_exp: b"./0".to_vec(),
            ff_level_down: b"1.2.1 3456".to_vec(),
            pow10: [0, 1, 10, 100, 1000, 10000, 100000, 1000000, 10000000],
        };
        let mut d = DamUprStr::default();
        let mut font = Font::fly();
        d.entry_new(&s, 2, 57, Some(7), &mut font);
        d.entry_new(&s, 2, 1234, Some(7), &mut font);
        d.entry_miss(&s, Some(8), &mut font);
        assert_eq!(d.nodes.len(), 2);
        assert_eq!(d.nodes[0].ch, Some(8));
        let u = &mut d.nodes[1].uproll;
        assert_eq!((u.line_num, u.line_top), (2, 2));
        // 1234 at 1.6: four cells of 14 x 1.6 px, centred.
        assert_eq!(u.lines[2].lx, -(4 * ee::to_int(ee::mul(SCALE_1000, ee::from_int(224)))) as i16 / 2);
        for _ in 0..40 {
            u.ctrl();
        }
        assert_eq!(u.line_num, 0);
        assert_eq!(u.lines[2].ly, 288);
    }
}
