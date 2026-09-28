//! `ccSprite` / `ccMask` as the desktop uses them: a textured SPRITE per
//! cell (`SetPrim(m, 0)`, type TF), queued by `MakePacket` and sent to the
//! sprite's layer by `SendPacket` (`ccSprite::MakePacketStr` 0x0015aed0,
//! `SendPacket` 0x0015abb0, `SetTag` 0x0015c0c0).
//!
//! Every ccMask the desktop builds passes `ms = 0`, so it is a plain TF
//! sprite (`ccMask::ccMask` 0x0015c4b0).

use piney_draw::{Blend, DrawState, Prim, PrimKind, Rgba, TexRef, TexState, Vertex};

use crate::layers::Layers;
use crate::view::{CULL_X1, CULL_Y1, LayerView, XYOFFSET_X, XYOFFSET_Y};

/// A strip's four corners in 12.4 (top left, bottom left, top right,
/// bottom right).
type Corners = [(i32, i32); 4];

/// A TF sprite: its ccSprite fields.
#[derive(Clone, Debug)]
pub struct Sprite {
    /// `ccSprite.layer`: the layer it sends to.
    pub layer: i16,
    pub tex: Option<TexRef>,
    /// `1 << TEX0.TH`: the texture's height in texels.
    pub tex_h: i32,
    /// `color[0]`, alpha at 0x80.
    pub colour: [u8; 4],
    pub transp: f32,
    pub cx: f32,
    pub cy: f32,
    pub dx: f32,
    pub dy: f32,
    pub sx: f32,
    pub sy: f32,
    /// Cell size in texels.
    pub su: i32,
    pub sv: i32,
    /// Grid origin in 1/16 texels, and cells a row.
    pub wu: i32,
    pub wv: i32,
    pub wi: i32,
    /// `ccSprite.alphaBlendType`.
    pub alpha_blend: usize,
    /// ctrl 0x10: a (16, 16, 16) shadow one unit right and down.
    pub shadow: bool,
    /// ctrl 0x20: the cell mirrored left to right (U runs from `u1 - 1`
    /// down to `u0 - 1`).
    pub flip_u: bool,
    /// ctrl 0x40: the cell upside down (`MakePacketStr` swaps the quad's
    /// two V rows; the Controller page's arrows).
    pub flip_v: bool,
    /// The rotated types (`SetPrim` TRF, TRG: PRIM bit 0x2): the turn
    /// (`+0x3c`, radians). Each cell is then a strip of four corners:
    /// `RotZ(-rot)` of (cx, cy) plus (dx, dy), and that plus the turned
    /// edges (sx, 0) and (0, sy); `dx`, `dy` move on by the first edge.
    pub rot: Option<f32>,
    /// The Gouraud types (TG, TRG): each corner's colour (`color[0..7]`:
    /// top left, bottom left, top right, bottom right), the alpha times
    /// `transp` as the main colour's.
    pub vcol: Option<[[u8; 4]; 4]>,
    pub packet_max: usize,
    queued: Vec<Prim>,
}

impl Sprite {
    /// `ccMask(m, 0)` with `SetTex`: a TF sprite of at most `m` packets.
    pub fn mask(layer: i16, tex: TexRef, tex_h: i32, packet_max: usize) -> Self {
        Sprite {
            layer,
            tex: Some(tex),
            tex_h,
            colour: [0x80; 4],
            transp: 1.0,
            cx: 0.0,
            cy: 0.0,
            dx: 0.0,
            dy: 0.0,
            sx: 0.0,
            sy: 0.0,
            su: 0,
            sv: 0,
            wu: 0,
            wv: 0,
            wi: 1,
            alpha_blend: 0,
            shadow: false,
            flip_u: false,
            flip_v: false,
            rot: None,
            vcol: None,
            packet_max,
            queued: Vec::new(),
        }
    }

    /// Sets the cell: `wu`, `wv` in 1/16 texels, `sx`/`su` wide, `sy`/`sv`
    /// high, one cell a row.
    pub fn cell(&mut self, wu: i32, wv: i32, w: i32, h: i32) {
        self.wu = wu;
        self.wv = wv;
        self.wi = 1;
        self.sx = w as f32;
        self.su = w;
        self.sy = h as f32;
        self.sv = h;
    }

    /// `ccSprite::MakePacket(code, 1)`: cell `code` at (dx, dy) through
    /// `view`, the shadow first when on; moves dx on by sx.
    pub fn make_packet(&mut self, code: u8, view: &LayerView) {
        if self.rot.is_some() || self.vcol.is_some() {
            return self.make_strip(code, view);
        }
        let (mut x, mut y) = view.apply(self.dx + self.cx, self.dy + self.cy);
        let w = crate::eef::to_int(crate::eef::mul(self.sx, view.sx));
        let h = crate::eef::to_int(crate::eef::mul(self.sy, view.sy));
        let a = crate::eef::to_int(crate::eef::mul(crate::eef::from_int(i32::from(self.colour[3])), self.transp)) as u8;
        let main = [self.colour[0], self.colour[1], self.colour[2], a];
        let shade = [16, 16, 16, a];
        let cell = i32::from(code);
        let col = cell % self.wi.max(1);
        let row = cell / self.wi.max(1);
        let u0 = col * self.su * 16 + self.wu;
        let v0 = self.tex_h * 16 - self.wv - row * self.sv * 16 - 1;
        let (u1, v1) = (u0 + self.su * 16, v0 - (self.sv * 16 - 1));
        let (u0, u1) = if self.flip_u { (u1 - 1, u0 - 1) } else { (u0, u1) };
        let (v0, v1) = if self.flip_v { (v1, v0) } else { (v0, v1) };
        for pass in 0..2 {
            if self.queued.len() >= self.packet_max {
                return;
            }
            let rgba = if pass == 0 {
                if !self.shadow {
                    continue;
                }
                x = crate::eef::to_int(crate::eef::add(crate::eef::from_int(x), view.sx));
                y = crate::eef::to_int(crate::eef::add(crate::eef::from_int(y), view.sy));
                shade
            } else {
                if self.shadow {
                    x = crate::eef::to_int(crate::eef::sub(crate::eef::from_int(x), view.sx));
                    y = crate::eef::to_int(crate::eef::sub(crate::eef::from_int(y), view.sy));
                }
                main
            };
            if x > CULL_X1 || x + w < XYOFFSET_X || y > CULL_Y1 || y + h < XYOFFSET_Y {
                continue;
            }
            self.queued.push(sprite_prim(
                (x, y, u0, v0),
                (x + w, y + h, u1, v1),
                Rgba(rgba),
                self.tex.clone(),
                Blend::TABLE[self.alpha_blend.min(8)],
                view,
            ));
        }
        self.dx += self.sx;
    }

    /// `MakePacketStr`'s four-corner types (0x0015b064-0x0015bb80,
    /// 0x0015c000): the rotated and the Gouraud cells, a textured
    /// TRIANGLE_STRIP of (top left, bottom left, top right, bottom right).
    /// A rotated cell is dropped when all four corners are off the screen
    /// (the flat ones test the top left as a SPRITE does); its shadow moves
    /// the corner by the view's scales.
    fn make_strip(&mut self, code: u8, view: &LayerView) {
        use crate::eef::{add, from_int, mul, sub, to_int};
        let alpha = |a: u8| to_int(mul(from_int(i32::from(a)), self.transp)) as u8;
        let main = [self.colour[0], self.colour[1], self.colour[2], alpha(self.colour[3])];
        let cols = match self.vcol {
            Some(v) => v.map(|c| [c[0], c[1], c[2], alpha(c[3])]),
            None => [main; 4],
        };
        let shade = [16, 16, 16, main[3]];
        let cell = i32::from(code);
        let col = cell % self.wi.max(1);
        let row = cell / self.wi.max(1);
        let u0 = col * self.su * 16 + self.wu;
        let v0 = self.tex_h * 16 - self.wv - row * self.sv * 16 - 1;
        let (u1, v1) = (u0 + self.su * 16, v0 - (self.sv * 16 - 1));
        let mut uv = [(u0, v0), (u0, v1), (u1, v0), (u1, v1)];
        if self.flip_u {
            uv = [uv[2], uv[3], uv[0], uv[1]].map(|(u, v)| (u - 1, v));
        }
        // The corners in 12.4 for each pass (the shadow first), and the
        // first edge `dx` moves by.
        let (passes, step): ([Corners; 2], (f32, f32)) = match self.rot {
            Some(r) => {
                let (sn, cs) = (-r).sin_cos();
                let turn = |x: f32, y: f32| (sub(mul(cs, x), mul(sn, y)), add(mul(sn, x), mul(cs, y)));
                let a = turn(self.sx, 0.0);
                let b = turn(0.0, self.sy);
                let c = turn(self.cx, self.cy);
                let c = (add(c.0, self.dx), add(c.1, self.dy));
                let at = |c: (f32, f32)| {
                    let p = |x: f32, y: f32| view.apply(x, y);
                    [
                        p(c.0, c.1),
                        p(add(c.0, b.0), add(c.1, b.1)),
                        p(add(c.0, a.0), add(c.1, a.1)),
                        p(add(b.0, add(c.0, a.0)), add(b.1, add(c.1, a.1))),
                    ]
                };
                // The shadow moves the corner by the view's scales, in the
                // sprite's own units.
                ([at((add(c.0, view.sx), add(c.1, view.sy))), at(c)], a)
            }
            None => {
                let (x, y) = view.apply(self.dx + self.cx, self.dy + self.cy);
                let w = to_int(mul(self.sx, view.sx));
                let h = to_int(mul(self.sy, view.sy));
                let at = |(x, y): (i32, i32)| [(x, y), (x, y + h), (x + w, y), (x + w, y + h)];
                let shadow = (to_int(add(from_int(x), view.sx)), to_int(add(from_int(y), view.sy)));
                ([at(shadow), at((x, y))], (self.sx, 0.0))
            }
        };
        let off =
            |(x, y): (i32, i32), w: i32, h: i32| x > CULL_X1 || x + w < XYOFFSET_X || y > CULL_Y1 || y + h < XYOFFSET_Y;
        for (pass, pts) in passes.into_iter().enumerate() {
            if self.queued.len() >= self.packet_max {
                break;
            }
            if pass == 0 && !self.shadow {
                continue;
            }
            let rgba = if pass == 0 { [shade; 4] } else { cols };
            let skip = if self.rot.is_some() {
                pts.iter().all(|&p| off(p, 0, 0))
            } else {
                off(pts[0], pts[3].0 - pts[0].0, pts[3].1 - pts[0].1)
            };
            if skip {
                continue;
            }
            let verts = (0..4)
                .map(|k| Vertex {
                    x: (pts[k].0 - XYOFFSET_X) as f32 / 16.0,
                    y: (pts[k].1 - XYOFFSET_Y) as f32 / 16.0,
                    z: 0,
                    u: uv[k].0 as f32 / 16.0,
                    v: uv[k].1 as f32 / 16.0,
                    rgba: Rgba(rgba[k]),
                })
                .collect();
            let mut state =
                DrawState::sprite(Blend::TABLE[self.alpha_blend.min(8)], self.tex.clone().map(TexState::modulate));
            state.scissor = view.scissor;
            self.queued.push(Prim { kind: PrimKind::Strip, gouraud: self.vcol.is_some(), state, verts });
        }
        self.dx = add(self.dx, step.0);
        self.dy = add(self.dy, step.1);
    }

    /// Packets queued and not yet sent.
    pub fn queued_len(&self) -> usize {
        self.queued.len()
    }

    /// `ccSprite::SendPacket`: everything queued, as one group at the front
    /// of the sprite's layer.
    pub fn send(&mut self, layers: &mut Layers) {
        let group = std::mem::take(&mut self.queued).into_iter().map(piney_draw::Cmd::Prim).collect();
        layers.prepend(self.layer, group);
    }
}

/// A GS SPRITE from two corners in 12.4 units (x, y include the XYOFFSET;
/// u, v are texels x 16).
pub fn sprite_prim(
    a: (i32, i32, i32, i32),
    b: (i32, i32, i32, i32),
    rgba: Rgba,
    tex: Option<TexRef>,
    blend: Blend,
    view: &LayerView,
) -> Prim {
    let v = |(x, y, u, vv): (i32, i32, i32, i32)| Vertex {
        x: (x - XYOFFSET_X) as f32 / 16.0,
        y: (y - XYOFFSET_Y) as f32 / 16.0,
        z: 0,
        u: u as f32 / 16.0,
        v: vv as f32 / 16.0,
        rgba,
    };
    let mut state = DrawState::sprite(blend, tex.map(TexState::modulate));
    state.scissor = view.scissor;
    Prim { kind: PrimKind::Sprite, gouraud: false, state, verts: vec![v(a), v(b)] }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cursor() -> Sprite {
        let mut s = Sprite::mask(0, TexRef::FrameBuffer { x: 0, y: 0, width: 64, height: 64 }, 64, 8);
        s.cell(0, 0, 32, 32);
        s.cx = -16.0;
        s.cy = -16.0;
        s.dx = 256.0;
        s.dy = 192.0;
        s
    }

    /// The target cursor (TRF, turned pi/4 about its centre): a diamond
    /// strip of four corners about (256, 192), the first edge's turn moving
    /// dx and dy on.
    #[test]
    fn a_rotated_cell_is_a_turned_strip() {
        let view = LayerView::default_layer();
        let mut s = cursor();
        s.rot = Some(std::f32::consts::FRAC_PI_4);
        s.make_packet(0, &view);
        let p = &s.queued[0];
        assert_eq!(p.kind, PrimKind::Strip);
        assert_eq!(p.verts.len(), 4);
        let (cx, cy) = view.apply(256.0, 192.0);
        let centre = LayerView::pixel(cx, cy);
        // Each corner 16 sqrt(2) from the centre in logical units, on the
        // axes (a diamond); the view stretches y by 448 / 384.
        let d: Vec<(f32, f32)> = p.verts.iter().map(|v| (v.x - centre.0, v.y - centre.1)).collect();
        let r = 16.0 * std::f32::consts::SQRT_2;
        for &(x, y) in &d {
            let on_axis = x.abs() < 0.2 || y.abs() < 0.2;
            assert!(on_axis, "{d:?}");
            assert!((x.abs() - r).abs() < 0.2 || (y.abs() / (448.0 / 384.0) - r).abs() < 0.3, "{d:?}");
        }
        assert!((s.dx - (256.0 + 16.0 * std::f32::consts::SQRT_2)).abs() < 0.01, "{}", s.dx);
        assert!((s.dy + 16.0 * std::f32::consts::SQRT_2 - 192.0).abs() < 0.01, "{}", s.dy);
    }

    /// A Gouraud cell (TG) keeps its corners' colours, top left, bottom
    /// left, top right, bottom right, on an axis-aligned strip.
    #[test]
    fn a_gouraud_cell_keeps_its_corners() {
        let view = LayerView::default_layer();
        let mut s = cursor();
        let c = [[10, 0, 0, 128], [0, 20, 0, 128], [0, 0, 30, 128], [40, 40, 40, 64]];
        s.vcol = Some(c);
        s.make_packet(0, &view);
        let p = &s.queued[0];
        assert!(p.gouraud);
        assert_eq!(p.verts.iter().map(|v| v.rgba.0).collect::<Vec<_>>(), c.to_vec());
        let (x0, y0) = (p.verts[0].x, p.verts[0].y);
        assert_eq!((p.verts[1].x, p.verts[2].y), (x0, y0));
        assert!(p.verts[3].x > x0 && p.verts[3].y > y0);
        assert_eq!(s.dx, 256.0 + 32.0);
    }
}
