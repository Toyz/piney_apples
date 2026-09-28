//! The minimap's sprites: `ccSprite` and `ccMask` as the map code drives
//! them, the packets they queue, and the GS primitives `MakePacketStr` (main
//! 0x0015aed0) makes of them. A `ccMask` is a TF sprite holding a TRF one
//! that `MakePacketS` queues turned cells on. [`Spr`] keeps the fields the map
//! writes and [`Packet`] a copy at each call (floats as bits), compared call
//! for call with the game by `tools/test_map_rs.py`; [`prims`] draws them as
//! `MakePacketStr` does (docs/engine/map.md, "The sprites").

use piney_desktop::view::{CULL_X1, CULL_Y1, LayerView, XYOFFSET_X, XYOFFSET_Y};
use piney_draw::{Blend, Cmd, DrawState, Prim, PrimKind, Rgba, TexRef, TexState, Vertex};

use crate::ee::{self, F, ONE, V4};

/// `ccSprite.ctrl`'s bits the map sets: a drop shadow, mirrored left to
/// right, upside down.
pub const SHADOW: u32 = 0x10;
pub const MIRROR: u32 = 0x20;
pub const FLIP: u32 = 0x40;
const FLIPS: u32 = SHADOW | MIRROR | FLIP;

/// One queued cell, with the sprite's fields at the call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Packet {
    /// Which of the map's sprites (the owner's numbering).
    pub spr: u8,
    /// Through `ccMask::MakePacketS`: the mask's second, turning sprite.
    pub sub: bool,
    pub code: u8,
    /// +0x30 `transp`, +0x34 `cx`, +0x38 `cy`, +0x3c `rot`, +0x40 `dx`,
    /// +0x44 `dy`, +0x48 `sx`, +0x4c `sy`.
    pub transp: F,
    pub cx: F,
    pub cy: F,
    pub rot: F,
    pub dx: F,
    pub dy: F,
    pub sx: F,
    pub sy: F,
    /// +0x50 `su`, +0x54 `sv`, +0x58 `wu`, +0x5c `wv`, +0x60 `wi`.
    pub su: i32,
    pub sv: i32,
    pub wu: i32,
    pub wv: i32,
    pub wi: i32,
    /// `color[0]`: the low bytes of its r, g, b and alpha words.
    pub rgba: [u8; 4],
    /// `ctrl & 0x70`.
    pub ctrl: u32,
}

/// A `ccSprite` or `ccMask` as the map drives it.
#[derive(Clone, Debug)]
pub struct Spr {
    pub id: u8,
    /// `ctrl`'s 0x10, 0x20 and 0x40.
    pub ctrl: u32,
    pub transp: F,
    pub cx: F,
    pub cy: F,
    pub rot: F,
    pub dx: F,
    pub dy: F,
    pub sx: F,
    pub sy: F,
    pub su: i32,
    pub sv: i32,
    pub wu: i32,
    pub wv: i32,
    pub wi: i32,
    pub rgba: [u8; 4],
    /// `packetMax` of the sprite and of a mask's second sprite (0: none).
    pub max: usize,
    pub sub_max: usize,
    pub queue: Vec<Packet>,
    pub sub_queue: Vec<Packet>,
}

impl Spr {
    /// `new ccMask(m, ms)`: `ccSprite::ccSprite` leaves the colour (128,
    /// 128, 128), alpha 128, `transp` 1 and `wi` 1.
    pub fn mask(id: u8, m: usize, ms: usize) -> Spr {
        Spr {
            id,
            ctrl: 0,
            transp: ONE,
            cx: 0,
            cy: 0,
            rot: 0,
            dx: 0,
            dy: 0,
            sx: 0,
            sy: 0,
            su: 0,
            sv: 0,
            wu: 0,
            wv: 0,
            wi: 1,
            rgba: [0x80; 4],
            max: m,
            sub_max: ms,
            queue: Vec::new(),
            sub_queue: Vec::new(),
        }
    }

    /// `color[0][1] = b | a << 32`: the alpha word.
    pub fn set_alpha(&mut self, a: i32) {
        self.rgba[3] = a as u8;
    }

    /// The cell grid: `wu`, `wv` in 1/16 texels, one cell a row, `su` x `sv`
    /// texels drawn `sx` x `sy`.
    pub fn cell(&mut self, wu: i32, wv: i32, su: i32, sv: i32, sx: F, sy: F) {
        self.wu = wu;
        self.wv = wv;
        self.wi = 1;
        self.su = su;
        self.sv = sv;
        self.sx = sx;
        self.sy = sy;
    }

    fn packet(&self, code: u8, sub: bool) -> Packet {
        Packet {
            spr: self.id,
            sub,
            code,
            transp: self.transp,
            cx: self.cx,
            cy: self.cy,
            rot: self.rot,
            dx: self.dx,
            dy: self.dy,
            sx: self.sx,
            sy: self.sy,
            su: self.su,
            sv: self.sv,
            wu: self.wu,
            wv: self.wv,
            wi: self.wi,
            rgba: self.rgba,
            ctrl: self.ctrl & FLIPS,
        }
    }

    /// `ccSprite::MakePacket(code, t)`: `MakePacketStr` of `{code, t ? 0xff
    /// : 0}`. The map always passes `t` 1 (raw cells, 0xff the end); with
    /// `t` 0 a code of 0 is the string's end and nothing is queued. Past
    /// `packetMax` nothing is queued. `dx` moves on by `sx`.
    pub fn make_packet(&mut self, code: u8, t: i32) {
        if (t == 0 && code == 0) || (t != 0 && code >= 0xfe) {
            return;
        }
        if self.queue.len() < self.max {
            let p = self.packet(code, false);
            self.queue.push(p);
        }
        self.dx = ee::add(self.dx, self.sx);
    }

    /// `ccMask::MakePacketS(code, t)`: the mask's layer, texture, grid
    /// (`wu`, `wv` to whole texels; `sv` from `fptosi(sy)`), size, place,
    /// colour, `transp`, `rot`, centre and ctrl 0x10, 0x20, 0x40 into the
    /// second sprite, then its `MakePacketStr`.
    pub fn make_packet_s(&mut self, code: u8, t: i32) {
        if self.sub_max == 0 || (t == 0 && code == 0) || (t != 0 && code >= 0xfe) {
            return;
        }
        let mut p = self.packet(code, true);
        p.wu = (self.wu >> 4) << 4;
        p.wv = (self.wv >> 4) << 4;
        p.sv = ee::to_int(self.sy);
        if self.sub_queue.len() < self.sub_max {
            self.sub_queue.push(p);
        }
    }

    /// `SendPacketS` (or `SendPacket`): the second sprite's queue, then the
    /// mask's, each emptied.
    pub fn send(&mut self) -> Vec<Send> {
        let mut out = Vec::new();
        if self.sub_max != 0 {
            out.push(Send { spr: self.id, sub: true, packets: std::mem::take(&mut self.sub_queue) });
        }
        out.push(Send { spr: self.id, sub: false, packets: std::mem::take(&mut self.queue) });
        out
    }

    /// `ccSprite::SendPacket` alone (the mask's own queue).
    pub fn send_main(&mut self) -> Send {
        Send { spr: self.id, sub: false, packets: std::mem::take(&mut self.queue) }
    }
}

/// One `SendPacket`: a sprite's queue as it went to its layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Send {
    pub spr: u8,
    pub sub: bool,
    pub packets: Vec<Packet>,
}

/// A `ccKanji::Disp(text, -1, 1, 1)` of the map (the field's "Overall Map"
/// and "Default Map", the dungeon's floor): a `ccInitKanji(16, 0)` kanji
/// (kt 0, 64 texture rows, 16 packets, ctrl 0x10), its colour
/// `ccSpriteColorTable[7]`, sent at once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    /// The map's own numbering of its kanji.
    pub spr: u8,
    pub text: Vec<u8>,
    pub dx: F,
    pub dy: F,
    pub rgba: [u8; 4],
    pub transp: F,
}

/// What a frame of the map sent, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Out {
    Send(Send),
    Text(Text),
}

/// Where a sprite draws: its layer, the layer's view and its texture
/// (`1 << TEX0.TH` rows).
#[derive(Clone, Debug)]
pub struct Place {
    pub layer: i16,
    pub view: LayerView,
    pub tex: TexRef,
    pub tex_h: i32,
}

/// `_sceVu0ecossin` (main 0x001109b8) as `sceVu0RotMatrixZ` uses it: cos an
/// odd polynomial in pi/2 - |angle|, sin +-sqrt(1 - cos^2). (sin, cos).
fn vu_cossin(angle: F) -> (F, F) {
    const S: [F; 4] = [0x362e_9c14, 0xb94f_b21f, 0x3c08_873e, 0xbe2a_aaa4];
    const HALF_PI: F = 0x3fc9_0fdb;
    let negative = ee::lt(angle, 0);
    let x = if negative { ee::add(HALF_PI, angle) } else { ee::sub(HALF_PI, angle) };
    let x2 = ee::mul(x, x);
    let mut p = S.map(|c| ee::mul(ee::mul(c, x), x2));
    for v in &mut p[..3] {
        *v = ee::mul(*v, x2);
    }
    let mut r = ee::add(x, p[3]);
    for v in &mut p[..2] {
        *v = ee::mul(*v, x2);
    }
    r = ee::add(r, p[2]);
    p[0] = ee::mul(p[0], x2);
    r = ee::add(ee::add(r, p[1]), p[0]);
    let q = ee::sqrt(ee::sub(ONE, ee::mul(r, r)));
    (if negative { ee::sub(0, q) } else { ee::add(0, q) }, r)
}

/// `sceVu0UnitMatrix` then `sceVu0RotMatrixZ(m, m, a)` (0x00110a30): Rz(a),
/// as stored columns.
fn rot_z(a: F) -> [V4; 4] {
    let (s, c) = vu_cossin(a);
    let (c, sn, ns) = (ee::add(0, c), ee::add(0, s), ee::sub(0, s));
    let rot = [[c, sn, 0, 0], [ns, c, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];
    let unit = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];
    unit.map(|col| {
        std::array::from_fn(|i| {
            let acc = ee::mul(rot[0][i], col[0]);
            let acc = ee::add(acc, ee::mul(rot[1][i], col[1]));
            let acc = ee::add(acc, ee::mul(rot[2][i], col[2]));
            ee::add(acc, ee::mul(rot[3][i], col[3]))
        })
    })
}

/// `ccView::ApplyLayerScreenMatrix(int *, float *)` of (x, y, 0, 1):
/// `layer_screen` times the point, each lane truncated to 12.4.
pub fn layer_screen(view: &LayerView, x: F, y: F) -> (i32, i32) {
    let (sx, sy, ox, oy) = (view.sx.to_bits(), view.sy.to_bits(), view.ox.to_bits(), view.oy.to_bits());
    (ee::to_int(ee::add(ee::mul(sx, x), ox)), ee::to_int(ee::add(ee::mul(sy, y), oy)))
}

/// A corner in 12.4 and its texel coordinates in 1/16 texels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Corner {
    pub x: i32,
    pub y: i32,
    pub u: i32,
    pub v: i32,
}

/// What `MakePacketStr` writes for one packet: the corners (two for a
/// SPRITE, four for a strip) and RGBA, or None when it is culled.
pub fn corners(p: &Packet, view: &LayerView, tex_h: i32) -> Option<(Vec<Corner>, [u8; 4])> {
    let (su16, sv16) = (p.su * 16, p.sv * 16);
    let code = i32::from(p.code);
    let wi = p.wi.max(1);
    let u0 = (code % wi) * su16 + p.wu;
    let v0 = tex_h * 16 - p.wv - (code / wi) * sv16 - 1;
    let alpha = ee::to_int(ee::mul(ee::from_int(i32::from(p.rgba[3])), p.transp)) as u8;
    let rgba = [p.rgba[0], p.rgba[1], p.rgba[2], alpha];
    if !p.sub {
        let (x, y) = layer_screen(view, ee::add(p.dx, p.cx), ee::add(p.dy, p.cy));
        let w = ee::to_int(ee::mul(p.sx, view.sx.to_bits()));
        let h = ee::to_int(ee::mul(p.sy, view.sy.to_bits()));
        if x > CULL_X1 || x + w < XYOFFSET_X || y > CULL_Y1 || y + h < XYOFFSET_Y {
            return None;
        }
        let (mut a, mut b) = ((u0, v0), (u0 + su16, v0 - (sv16 - 1)));
        if p.ctrl & MIRROR != 0 {
            (a.0, b.0) = (b.0 - 1, a.0 - 1);
        }
        if p.ctrl & FLIP != 0 {
            (a.1, b.1) = (b.1, a.1);
        }
        return Some((vec![Corner { x, y, u: a.0, v: a.1 }, Corner { x: x + w, y: y + h, u: b.0, v: b.1 }], rgba));
    }
    let m = rot_z(ee::neg(p.rot));
    let ex = ee::apply(&m, [p.sx, 0, 0, ONE]);
    let ey = ee::apply(&m, [0, p.sy, 0, ONE]);
    let c = ee::apply(&m, [p.cx, p.cy, 0, ONE]);
    let o = [ee::add(c[0], p.dx), ee::add(c[1], p.dy)];
    let pts = [
        o,
        [ee::add(o[0], ey[0]), ee::add(o[1], ey[1])],
        [ee::add(o[0], ex[0]), ee::add(o[1], ex[1])],
        [ee::add(ee::add(o[0], ex[0]), ey[0]), ee::add(ee::add(o[1], ex[1]), ey[1])],
    ];
    // The cull tests each corner against the screen with the width and
    // height the unrotated path would have left in the scratch area (not
    // set on this path); a corner on the screen passes whatever they are.
    let xy: Vec<(i32, i32)> = pts.iter().map(|q| layer_screen(view, q[0], q[1])).collect();
    let off = xy
        .iter()
        .filter(|&&(x, y)| !(XYOFFSET_X..=CULL_X1).contains(&x) || !(XYOFFSET_Y..=CULL_Y1).contains(&y))
        .count();
    if off >= 4 {
        return None;
    }
    let mut uv = [(u0, v0), (u0, v0 - (sv16 - 1)), (u0 + su16, v0), (u0 + su16, v0 - (sv16 - 1))];
    if p.ctrl & MIRROR != 0 {
        uv = [(uv[2].0 - 1, uv[2].1), (uv[3].0 - 1, uv[3].1), (uv[0].0 - 1, uv[0].1), (uv[1].0 - 1, uv[1].1)];
    }
    if p.ctrl & FLIP != 0 {
        uv = [uv[1], uv[0], uv[3], uv[2]];
    }
    let out = xy.iter().zip(uv).map(|(&(x, y), (u, v))| Corner { x, y, u, v }).collect();
    Some((out, rgba))
}

/// A send's packets as GS primitives: a SPRITE a cell, or a triangle strip
/// for the turning sprite; MODULATE over the sprite's texture, blended
/// `alphaBlendType` 0, clipped to the layer's scissor.
pub fn prims(send: &Send, place: &Place) -> Vec<Cmd> {
    let mut out = Vec::new();
    for p in &send.packets {
        let Some((cs, rgba)) = corners(p, &place.view, place.tex_h) else { continue };
        let verts = cs
            .iter()
            .map(|c| Vertex {
                x: (c.x - XYOFFSET_X) as f32 / 16.0,
                y: (c.y - XYOFFSET_Y) as f32 / 16.0,
                z: 0,
                u: c.u as f32 / 16.0,
                v: c.v as f32 / 16.0,
                rgba: Rgba(rgba),
            })
            .collect();
        let mut state = DrawState::sprite(Blend::TABLE[0], Some(TexState::modulate(place.tex.clone())));
        state.scissor = place.view.scissor;
        let kind = if p.sub { PrimKind::Strip } else { PrimKind::Sprite };
        out.push(Cmd::Prim(Prim { kind, gouraud: false, state, verts }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mask_queues_on_itself_and_its_second_sprite() {
        let mut s = Spr::mask(3, 4, 2);
        s.cell(0x80, 0x40, 16, 16, ee::k(16.0), ee::k(12.0));
        s.make_packet(0, 1);
        s.make_packet(0, 0);
        s.make_packet_s(0, 1);
        assert_eq!(s.queue.len(), 1);
        assert_eq!(s.sub_queue.len(), 1);
        // MakePacketS: sv from fptosi(sy), wu/wv whole texels.
        assert_eq!(s.sub_queue[0].sv, 12);
        let sent = s.send();
        assert_eq!(sent.iter().map(|x| (x.sub, x.packets.len())).collect::<Vec<_>>(), [(true, 1), (false, 1)]);
        assert!(s.queue.is_empty() && s.sub_queue.is_empty());
    }

    #[test]
    fn an_unturned_strip_is_the_sprite() {
        let view = LayerView::frame(340.0, 36.0, 160.0, 160.0, 1.0, 1.0);
        let mut s = Spr::mask(0, 4, 4);
        s.cell(0, 0, 16, 16, ee::k(16.0), ee::k(16.0));
        s.dx = ee::k(40.0);
        s.dy = ee::k(30.0);
        s.make_packet(0, 1);
        s.dx = ee::k(40.0);
        s.make_packet_s(0, 1);
        let (a, _) = corners(&s.queue[0], &view, 16).unwrap();
        let (b, _) = corners(&s.sub_queue[0], &view, 16).unwrap();
        assert_eq!((a[0].x, a[0].y), (b[0].x, b[0].y));
        // The sprite adds its height truncated on its own; the strip
        // truncates its far corner whole: a sixteenth apart at most.
        assert_eq!(a[1].x, b[3].x);
        assert!((b[3].y - a[1].y).abs() <= 1, "{a:?} {b:?}");
        assert_eq!((a[0].u, a[0].v, a[1].u, a[1].v), (b[0].u, b[0].v, b[3].u, b[3].v));
    }
}
