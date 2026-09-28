//! The numbers over characters (gcmn gamectrl.cpp): `flyFontCtrl` (gcmn
//! 0x0072ebd0), the sixteen rising numbers `ccEntryFlyFontNum` starts and
//! `ccCtrlFlyFont` moves (1.5 px a frame for 40 frames), and the `ccFont` both
//! they and [`crate::damupr`]'s stacked numbers draw with: `ccMenuCtrl` +0xb4
//! `flyFont`, 160 packets on `xasc00::TEX_xasc00` on the menu layer (242).
//! What the calls queue is recorded as [`Quad`]s exactly as
//! `ccSprite::MakePacketStr` (main 0x0015aed0) builds them, and turned into
//! primitives by [`quad_prim`]. The rules are in docs/engine/effects.md.

use piney_data::Result;
use piney_data::tables::kanji::SPRITE_COLOR_TABLE;
use piney_data::volume::Volume;
use piney_desktop::layers::Layers;
use piney_desktop::message::MENU_LAYER;
use piney_desktop::view::{CULL_X1, CULL_Y1, LayerView, XYOFFSET_X, XYOFFSET_Y};
use piney_draw::{Blend, Cmd, Prim, Rgba, TexRef};

use crate::ee::{self, F, ONE, V4};

pub const ENTRIES: usize = 16;
/// An entry's life in frames.
pub const LIFE: i32 = 40;
/// `ccEntryFlyFontNum`: 140 added to the point's z.
const RISE: F = 0x430c_0000;
/// `ccMenuCtrl` +0xb4 `flyFont`: `SetPrim(160, 0)`.
pub const PACKET_MAX: usize = 160;
/// The shadow's colour (ctrl 0x10).
const SHADOW: [u32; 3] = [16, 16, 16];

/// The strings and the table the fly fonts read from the executable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Strings {
    pub miss: Vec<u8>,
    pub ff_miss: Vec<u8>,
    pub ff_exp: Vec<u8>,
    pub ff_level_down: Vec<u8>,
    pub pow10: [i32; 9],
}

impl Strings {
    /// From the executable with GCMN.PRG over it.
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> Strings {
        use piney_data::tables::sjis::encode;
        let t = piney_data::tables::effect::of(volume);
        Strings {
            miss: encode(t.miss()),
            ff_miss: encode(t.ff_miss()),
            ff_exp: encode(t.ff_exp()),
            ff_level_down: encode(t.ff_level_down()),
            pow10: crate::spell::first(t.pow10()),
        }
    }
}

/// `ccFont::SetType(t)` (main 0x0015cc20): sx, sy, su, sv, wu, wv (1/16
/// texels), wi.
pub const FONT_TYPES: [(F, F, i32, i32, i32, i32, i32); 4] = [
    (0x4100_0000, 0x4140_0000, 8, 12, 2048, 2688, 16),
    (0x4140_0000, 0x4180_0000, 12, 16, 0, 0, 16),
    (0x4120_0000, 0x4140_0000, 10, 12, 2048, 1536, 12),
    (0x4160_0000, 0x4180_0000, 14, 16, 3072, 0, 4),
];

/// One glyph's SPRITE as `MakePacketStr` writes it: RGBAQ's four words,
/// then UV and XYZ2 of the two corners (12.4 screen units with the
/// XYOFFSET, 1/16 texels).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quad {
    pub rgba: [u32; 4],
    pub uv0: (i32, i32),
    pub xy0: (i32, i32),
    pub uv1: (i32, i32),
    pub xy1: (i32, i32),
    /// The shadow's quad (ctrl 0x10), drawn before the glyph.
    pub shadow: bool,
}

/// The `ccSprite` fields of `flyFont` that the fly fonts set and
/// `MakePacketStr` reads, and its queue.
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    /// +0x04: 1 textured, 0x10 the shadow, 0x20 / 0x40 flipped.
    pub ctrl: i32,
    /// +0x30.
    pub transp: F,
    /// +0x34 cx, +0x38 cy, +0x40 dx, +0x44 dy, +0x48 sx, +0x4c sy.
    pub cx: F,
    pub cy: F,
    pub dx: F,
    pub dy: F,
    pub sx: F,
    pub sy: F,
    /// +0x50 su, +0x54 sv (texels), +0x58 wu, +0x5c wv (1/16 texels), +0x60
    /// wi.
    pub su: i32,
    pub sv: i32,
    pub wu: i32,
    pub wv: i32,
    pub wi: i32,
    /// +0x68 `color[0]`: r, g (the first u64), b, a (the second), a word
    /// each.
    pub rgba: [u32; 4],
    /// +0x20 packetNum, +0x24 packetMax.
    pub packet_num: usize,
    pub packet_max: usize,
    /// The texture's height in texels (`1 << TEX0.TH`): 256 for
    /// `TEX_xasc00`.
    pub tex_h: i32,
    /// The glyphs queued since the last send.
    pub queue: Vec<Quad>,
}

impl Font {
    /// `flyFont` as `ccMenuCtrl` makes it: `new ccSprite` (grey 128, alpha
    /// 128, transp 1, wi 1), `SetPrim(160, 0)` (ctrl 1), `CopyTex(fontTex)`.
    pub fn fly() -> Font {
        Font {
            ctrl: 1,
            transp: ONE,
            cx: 0,
            cy: 0,
            dx: 0,
            dy: 0,
            sx: 0,
            sy: 0,
            su: 0,
            sv: 0,
            wu: 0,
            wv: 0,
            wi: 1,
            rgba: [128; 4],
            packet_num: 0,
            packet_max: PACKET_MAX,
            tex_h: 256,
            queue: Vec::new(),
        }
    }

    /// `ccFont::SetType(t)`; other types change nothing.
    pub fn set_type(&mut self, t: i32) {
        if let Some(&(sx, sy, su, sv, wu, wv, wi)) = usize::try_from(t).ok().and_then(|t| FONT_TYPES.get(t)) {
            (self.sx, self.sy, self.su, self.sv, self.wu, self.wv, self.wi) = (sx, sy, su, sv, wu, wv, wi);
        }
    }

    /// sx, sy, cx and cy times a scale, as both fly-font draws do.
    pub fn scale(&mut self, sx: F, sy: F) {
        self.sx = ee::mul(self.sx, sx);
        self.sy = ee::mul(self.sy, sy);
        self.cx = ee::mul(self.cx, sx);
        self.cy = ee::mul(self.cy, sy);
    }

    /// `color[0]`'s r, g, b from `ccSpriteColorTable[i]` (the alpha word
    /// kept).
    pub fn set_colour(&mut self, i: i32) {
        let c = SPRITE_COLOR_TABLE[i.clamp(0, 23) as usize];
        self.rgba[0] = u32::from(c[0]);
        self.rgba[1] = u32::from(c[1]);
        self.rgba[2] = u32::from(c[2]);
    }

    /// `color[0][1] = (b & 0xff) | a << 32`: the alpha word, and b cut to
    /// its low byte (`lbu`).
    pub fn set_alpha(&mut self, a: i32) {
        self.rgba[2] &= 0xff;
        self.rgba[3] = a as u32;
    }

    /// `ccSprite::MakePacketStr(str, 0)` (main 0x0015aed0) through the
    /// layer's view: a glyph a byte from 0x21 to 0x7f (cell `c - 0x20` of
    /// the grid), the shadow first with ctrl 0x10; any other byte only
    /// moves on; a quad wholly off the screen is dropped; nothing past
    /// `packetMax`. dx moves on by sx a byte.
    pub fn make_packet_str(&mut self, s: &[u8], view: &LayerView) {
        let (mut x, mut y) = apply(view, ee::add(self.dx, self.cx), ee::add(self.dy, self.cy));
        let w = ee::to_int(ee::mul(self.sx, view.sx.to_bits()));
        let h = ee::to_int(ee::mul(self.sy, view.sy.to_bits()));
        let a = ee::to_int(ee::mul(ee::from_int(self.rgba[3] as i32), self.transp)) as u32;
        let main = [self.rgba[0], self.rgba[1], self.rgba[2], a];
        let shade = [SHADOW[0], SHADOW[1], SHADOW[2], a];
        let (su16, sv16, th16) = (self.su * 16, self.sv * 16, self.tex_h * 16);
        let wi = self.wi.max(1);
        for &c in s {
            if c == 0 {
                break;
            }
            if !(33..128).contains(&c) {
                x = x.wrapping_add(w);
                self.dx = ee::add(self.dx, self.sx);
                continue;
            }
            let cell = i32::from(c) - 32;
            for pass in 0..2 {
                if self.packet_num >= self.packet_max {
                    return;
                }
                let rgba = if pass == 0 {
                    if self.ctrl & 0x10 == 0 {
                        continue;
                    }
                    x = ee::to_int(ee::add(ee::from_int(x), view.sx.to_bits()));
                    y = ee::to_int(ee::add(ee::from_int(y), view.sy.to_bits()));
                    shade
                } else {
                    if self.ctrl & 0x10 != 0 {
                        x = ee::to_int(ee::sub(ee::from_int(x), view.sx.to_bits()));
                        y = ee::to_int(ee::sub(ee::from_int(y), view.sy.to_bits()));
                    }
                    main
                };
                if x > CULL_X1 || x + w < XYOFFSET_X || y > CULL_Y1 || y + h < XYOFFSET_Y {
                    continue;
                }
                let mut u0 = (cell % wi) * su16 + self.wu;
                let mut v0 = th16 - self.wv - (cell / wi) * sv16 - 1;
                let mut u1 = u0 + su16;
                let mut v1 = v0 - (sv16 - 1);
                if self.ctrl & 0x20 != 0 {
                    (u0, u1) = (u1 - 1, u0 - 1);
                }
                if self.ctrl & 0x40 != 0 {
                    (v0, v1) = (v1, v0);
                }
                self.queue.push(Quad {
                    rgba,
                    uv0: (u0, v0),
                    xy0: (x, y),
                    uv1: (u1, v1),
                    xy1: (x + w, y + h),
                    shadow: pass == 0,
                });
                self.packet_num += 1;
            }
            x = x.wrapping_add(w);
            self.dx = ee::add(self.dx, self.sx);
        }
    }

    /// `ccFont::MakeSignedNum(clm, num)` (main 0x0015cde0):
    /// `sdec2str(min(clm, 126), num)` drawn by `MakePacketStr`.
    pub fn make_signed_num(&mut self, clm: i32, num: i32, view: &LayerView) {
        let clm = if clm < 127 { clm } else { 126 };
        let s = sdec2str(clm, i64::from(num), 0);
        self.make_packet_str(&s, view);
    }

    /// `SendPacket`: the queue, emptied.
    pub fn take(&mut self) -> Vec<Quad> {
        self.packet_num = 0;
        std::mem::take(&mut self.queue)
    }
}

/// `ccView::ApplyLayerScreenMatrix` on (x, y) and `sceVu0FTOI0Vector`:
/// `trunc(x sx + ox)`, `trunc(y sy + oy)` in EE arithmetic.
fn apply(view: &LayerView, x: F, y: F) -> (i32, i32) {
    let f = |v: F, s: f32, o: f32| ee::to_int(ee::add(ee::mul(v, s.to_bits()), o.to_bits()));
    (f(x, view.sx, view.ox), f(y, view.sy, view.oy))
}

/// `sdec2str(n, v, buf, neg)` (main 0x0015cfc0): a sign column and `n`
/// digits (the low ones when `v` has more): leading zeros but the last
/// blank, the sign ('-', or ' ') just before the first digit shown.
pub fn sdec2str(n: i32, v: i64, neg: i32) -> Vec<u8> {
    let n = n.max(0) as usize;
    let (sign, mut v) = if v < 0 || neg < 0 { (b'-', v.wrapping_neg()) } else { (b' ', v) };
    let mut buf = vec![b'0'; n + 1];
    for i in (1..=n).rev() {
        buf[i] = (b'0' as i8).wrapping_add((v % 10) as i8) as u8;
        v /= 10;
    }
    if v != 0 {
        buf[0] = sign;
    } else {
        let mut s = 0;
        while s < n {
            if buf[s] != b'0' {
                break;
            }
            buf[s] = b' ';
            s += 1;
        }
        if s > 0 {
            buf[s - 1] = sign;
        }
    }
    buf
}

/// `Int2StrFF(buf, v, n)` (gcmn 0x0051add0): `v` in the big digits of
/// font type 3 (digit d is byte 0x21 + d), at most `n` places, no leading
/// zeros; a negative number starts with 0x37 ('-').
pub fn int2str_ff(pow10: &[i32; 9], v: i32, n: i32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut v = v;
    if v < 0 {
        v = v.wrapping_neg();
        out.push(0x37);
    }
    let mut started = false;
    let mut k = n;
    while k != 0 {
        let p = pow10[k.clamp(0, 8) as usize];
        let q = if p == 0 { 0 } else { v.wrapping_div(p) };
        if started || q != 0 || k == 1 {
            out.push((q + 33) as u8);
            started = true;
        }
        v = v.wrapping_sub(q.wrapping_mul(p));
        k -= 1;
    }
    out
}

/// A glyph's quad as a GS SPRITE on the menu layer, through the view's
/// scissor, blended as `alphaBlendTbl[0]`.
pub fn quad_prim(q: &Quad, tex: TexRef, view: &LayerView) -> Prim {
    let rgba = Rgba(q.rgba.map(|c| c as u8));
    piney_desktop::sprite::sprite_prim(
        (q.xy0.0, q.xy0.1, q.uv0.0, q.uv0.1),
        (q.xy1.0, q.xy1.1, q.uv1.0, q.uv1.1),
        rgba,
        Some(tex),
        Blend::TABLE[0],
        view,
    )
}

/// The texture `flyFont` draws with: `xasc00`'s `TEX_xasc00` through its
/// own palette, as [`send`] takes it.
pub fn font_tex(archive: &piney_data::archive::Archive) -> Result<TexRef> {
    let ccs = piney_data::ccs::Ccs::parse(archive.inflate_named("xasc00")?)?;
    let object = ccs.find_object("TEX_xasc00").ok_or_else(|| piney_data::Error::NotFound("TEX_xasc00".into()))?;
    let (textures, _) = piney_data::texture::read(&ccs)?;
    let t =
        textures.iter().find(|t| t.object == object).ok_or_else(|| piney_data::Error::NotFound("TEX_xasc00".into()))?;
    Ok(TexRef::Ccs { file: "xasc00".into(), texture: object, clut: t.clut })
}

/// `flyFont`'s `SendPacket` of `quads`: one group at the front of the menu
/// layer (242), in queue order. In the game the enemy bars' labels
/// (queued on `flyFont` earlier in `Disp`) are in the same group, before
/// these: a runtime that draws them apart appends [`quad_prim`]s of these
/// to their group instead.
pub fn send(quads: &[Quad], tex: &TexRef, layers: &mut Layers) {
    let view = piney_desktop::message::menu_view();
    let group = quads.iter().map(|q| Cmd::Prim(quad_prim(q, tex.clone(), &view))).collect();
    layers.prepend(MENU_LAYER, group);
}

/// One `flyFontCtrl` entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    pub color: i32,
    pub timer: i32,
    pub num: i32,
    pub pos: V4,
    /// Up to 15 bytes; empty draws `num`.
    pub text: Vec<u8>,
    pub ofs_x: i32,
    pub sx: F,
    pub sy: F,
}

/// `flyFontCtrl`: the rising numbers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FlyFonts {
    pub entries: [Entry; ENTRIES],
}

impl FlyFonts {
    /// `ccInitFlyFont()` (gcmn 0x0051a8c0, from `ccThGameCtrl`): every
    /// timer 0.
    pub fn init(&mut self) {
        for e in &mut self.entries {
            e.timer = 0;
        }
    }

    /// `ccEntryFlyFontNum(color, num, pos, sx, sy, ofsX)` (gcmn 0x0051a900):
    /// the first free entry (none: nothing); `num` -1 draws "MISS"
    /// (`ffMissStr`, `num` left as it was).
    #[allow(clippy::too_many_arguments)]
    pub fn entry_num(&mut self, s: &Strings, color: i32, num: i32, pos: V4, sx: F, sy: F, ofs_x: i32) -> Option<usize> {
        let (i, e) = self.entries.iter_mut().enumerate().find(|(_, e)| e.timer == 0)?;
        e.timer = LIFE;
        e.color = color;
        if num == -1 {
            e.text = s.miss.iter().copied().take(15).collect();
        } else {
            e.num = num;
            e.text.clear();
        }
        e.pos = pos;
        e.pos[2] = ee::add(e.pos[2], RISE);
        e.sx = sx;
        e.sy = sy;
        e.ofs_x = ofs_x;
        Some(i)
    }

    /// `ccCtrlFlyFont(pause)` (gcmn 0x0051aa10) with `font` = flyFont:
    /// `world_screen` is sysLayer's view's world-to-screen matrix (+0xd0),
    /// `view` the menu layer's.
    pub fn ctrl(&mut self, pause: bool, font: &mut Font, world_screen: &[V4; 4], view: &LayerView) {
        for e in &mut self.entries {
            if e.timer == 0 {
                continue;
            }
            if !pause {
                e.timer -= 1;
            }
            e.pos = ee::vadd(e.pos, [0; 4]);
            e.pos[3] = ONE;
            let Some((x, y)) = project(world_screen, e.pos) else { continue };
            let x = x.wrapping_add(e.ofs_x);
            let y = y.wrapping_add(-(3 * (LIFE - e.timer)) / 2);
            if !(-63..576).contains(&x) {
                continue;
            }
            font.set_type(2);
            font.scale(e.sx, e.sy);
            font.set_colour(e.color);
            font.dx = ee::from_int(x);
            font.dy = ee::from_int(y);
            font.set_alpha(if e.timer < 20 { (e.timer * 128) / 20 } else { 128 });
            font.ctrl |= 0x10;
            if e.text.is_empty() {
                let n = if e.num < 10 {
                    1
                } else if e.num < 100 {
                    2
                } else if e.num < 1000 {
                    3
                } else {
                    4
                };
                font.make_signed_num(n, e.num, view);
            } else {
                font.make_packet_str(&e.text, view);
            }
        }
    }
}

/// `sceVu0RotTransPers` of a point, and with the depth in (0, 0x0fffffff)
/// the screen pixel the fly fonts use: `fptosi(float(X) - 28672) / 16`,
/// `(Y - 32768 + 3584) / 16`.
pub fn project(world_screen: &[V4; 4], pos: V4) -> Option<(i32, i32)> {
    let v = ee::rot_trans_pers(world_screen, pos);
    if !(v[2] > 0 && v[2] < 0x0fff_ffff) {
        return None;
    }
    Some(screen(v))
}

/// The fixed-point screen point to pixels (both conversions round toward
/// zero).
pub fn screen(v: [i32; 4]) -> (i32, i32) {
    let x = ee::to_int(ee::sub(ee::from_int(v[0]), 0x46e0_0000)) / 16;
    let y = v[1].wrapping_sub(0x8000).wrapping_add(0xe00) / 16;
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_numbers_have_a_sign_column() {
        assert_eq!(sdec2str(2, 57, 0), b" 57");
        assert_eq!(sdec2str(1, -5, 0), b"-5");
        assert_eq!(sdec2str(4, 7, 0), b"    7");
        assert_eq!(sdec2str(3, -42, 0), b" -42");
        assert_eq!(sdec2str(4, 12345, 0), b" 2345");
        assert_eq!(sdec2str(1, 0, 0), b" 0");
    }

    #[test]
    fn big_digits() {
        let p = [0, 1, 10, 100, 1000, 10000, 100000, 1000000, 10000000];
        assert_eq!(int2str_ff(&p, 305, 5), [0x24, 0x21, 0x26]);
        assert_eq!(int2str_ff(&p, 0, 5), [0x21]);
        assert_eq!(int2str_ff(&p, -7, 5), [0x37, 0x28]);
    }

    #[test]
    fn the_menu_view_and_a_glyph() {
        // The harness builds the menu layer's view from these.
        let v = piney_desktop::message::menu_view();
        assert_eq!((v.sx, v.sy, v.ox, v.oy), (16.0, 16.0, 28672.5, 29184.5));
        // '5' in type 2 at (100, 50), 10 x 12: cell 21 of 12 a row.
        let mut f = Font::fly();
        f.set_type(2);
        f.dx = ee::k(100.0);
        f.dy = ee::k(50.0);
        f.make_packet_str(b"5", &v);
        let q = f.queue[0];
        assert_eq!(q.xy0, (0x7000 + 1600, 0x7200 + 800));
        assert_eq!(q.xy1, (0x7000 + 1760, 0x7200 + 992));
        assert_eq!(q.uv0, (2048 + 9 * 160, 4096 - 1536 - 192 - 1));
        let p = quad_prim(&q, TexRef::Upload(0), &v);
        assert_eq!((p.verts[0].x, p.verts[0].y, p.verts[1].x, p.verts[1].y), (100.0, 50.0, 110.0, 62.0));
        assert_eq!(ee::f(f.dx), 110.0);
    }

    #[test]
    fn a_number_rises_and_fades() {
        // A view straight down the y axis: the point's screen position is
        // fixed, so only the rise and the fade move it.
        let m = [[ONE, 0, 0, 0], [0, 0, 0, ONE], [0, ONE, 0, 0], [0x44f0_0000, 0x4500_0000, ONE, 0]];
        let view = piney_desktop::message::menu_view();
        let mut ff = FlyFonts::default();
        let s =
            Strings { miss: b"MISS".to_vec(), ff_miss: vec![], ff_exp: vec![], ff_level_down: vec![], pow10: [0; 9] };
        ff.entry_num(&s, 2, 57, [0, ONE, 0, ONE], ONE, ONE, 0).unwrap();
        let mut font = Font::fly();
        ff.ctrl(false, &mut font, &m, &view);
        let q = font.take();
        assert_eq!(q.len(), 4);
        assert!(q[0].shadow && !q[1].shadow);
        assert_eq!(ff.entries[0].timer, 39);
        for _ in 0..39 {
            ff.ctrl(false, &mut font, &m, &view);
        }
        assert_eq!(ff.entries[0].timer, 0);
        assert_eq!(font.rgba[3], 0);
    }
}
