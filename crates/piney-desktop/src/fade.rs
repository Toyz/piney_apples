//! `ccScFade` (fade.cpp): the screen fader the World icon uses (`EntryFade`
//! 0x00160400, `SendPacket` 0x0015fb80, `CheckFade` 0x001604d0). A fading
//! element is one flat TRISTRIP over its rectangle on the font layer (240),
//! blended `(Cs - Cd) As + Cd`, its colour `c0 + trunc((c1 - c0) * cnt /
//! tcnt)` per channel, `cnt` counting the frames drawn and holding at `tcnt`.

use piney_draw::{Blend, Cmd, DrawState, Prim, PrimKind, Rgba, Vertex};

use crate::anm::Ctx;
use crate::layers::FONT_LAYER;
use crate::view::{LOGICAL_H, LOGICAL_W, LayerView, XYOFFSET_X, XYOFFSET_Y};

/// `ChooseMode`'s `EntryFade(20, 0, 0x80000000, 0, 0, 512, 384)`: to
/// black. Colours are RGBA with R in the low byte.
pub const WORLD_FROM: u32 = 0x0000_0000;
pub const WORLD_TO: u32 = 0x8000_0000;

/// The colour at frame `cnt` of `tcnt`.
pub fn colour(c0: u32, c1: u32, cnt: u32, tcnt: u32) -> [u8; 4] {
    let mut out = [0u8; 4];
    for (i, o) in out.iter_mut().enumerate() {
        let a = i64::from((c0 >> (8 * i)) & 0xff);
        let b = i64::from((c1 >> (8 * i)) & 0xff);
        *o = ((a + (b - a) * i64::from(cnt.min(tcnt)) / i64::from(tcnt.max(1))) & 0xff) as u8;
    }
    out
}

/// The World's fade at frame `cnt`, on the font layer.
pub fn draw(ctx: &mut Ctx, cnt: u32, tcnt: u32) {
    draw_colours(ctx, WORLD_FROM, WORLD_TO, cnt, tcnt);
}

/// A fading element from `c0` to `c1` at frame `cnt` of `tcnt`.
pub fn draw_colours(ctx: &mut Ctx, c0: u32, c1: u32, cnt: u32, tcnt: u32) {
    draw_on(ctx, FONT_LAYER, c0, c1, cnt, tcnt);
}

/// As [`draw_colours`] on layer `layer` (the menu's fader is on 242).
pub fn draw_on(ctx: &mut Ctx, layer: i16, c0: u32, c1: u32, cnt: u32, tcnt: u32) {
    draw_rect(ctx, layer, FULL, c0, c1, cnt, tcnt);
}

/// `EntryFade`'s rectangle over the whole frame, in logical units.
pub const FULL: [f32; 4] = [0.0, 0.0, LOGICAL_W, LOGICAL_H];

/// The element's corners in 12.4 for the rectangle `rect` (x, y, w, h in
/// logical units) on a layer with the default view: the corner through
/// `ApplyLayerScreenMatrix`, the size `w ls00`, `h ls11` truncated.
pub fn corners(rect: [f32; 4]) -> [(i32, i32); 4] {
    use crate::eef::{mul, to_int};
    let view = LayerView::default_layer();
    let (x0, y0) = view.apply(rect[0], rect[1]);
    let (w, h) = (to_int(mul(rect[2], view.sx)), to_int(mul(rect[3], view.sy)));
    [(x0, y0), (x0, y0 + h), (x0 + w, y0), (x0 + w, y0 + h)]
}

/// As [`draw_on`] over `rect` (x, y, w, h in logical units).
#[allow(clippy::too_many_arguments)]
pub fn draw_rect(ctx: &mut Ctx, layer: i16, rect: [f32; 4], c0: u32, c1: u32, cnt: u32, tcnt: u32) {
    let rgba = Rgba(colour(c0, c1, cnt, tcnt));
    let v = |(x, y): (i32, i32)| Vertex {
        x: (x - XYOFFSET_X) as f32 / 16.0,
        y: (y - XYOFFSET_Y) as f32 / 16.0,
        z: 0,
        u: 0.0,
        v: 0.0,
        rgba,
    };
    let prim = Prim {
        kind: PrimKind::Strip,
        gouraud: false,
        state: DrawState::sprite(Blend::MIX, None),
        verts: corners(rect).map(v).to_vec(),
    };
    ctx.layers.prepend(layer, vec![Cmd::Prim(prim)]);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Agent-checked in eemu: 0, 6, 12, 19, 25, 32 ... 121, then 128.
    #[test]
    fn world_fade_alpha() {
        let a: Vec<u8> = (0..22).map(|k| colour(WORLD_FROM, WORLD_TO, k, 20)[3]).collect();
        assert_eq!(&a[..6], &[0, 6, 12, 19, 25, 32]);
        assert_eq!(a[19], 121);
        assert_eq!(a[20], 128);
        assert_eq!(a[21], 128);
    }

    /// The whole frame: (0x7000, 0x7200)-(0x9000, 0x8dff).
    #[test]
    fn full_frame_corners() {
        assert_eq!(corners(FULL), [(0x7000, 0x7200), (0x7000, 0x8dff), (0x9000, 0x7200), (0x9000, 0x8dff)]);
    }
}
