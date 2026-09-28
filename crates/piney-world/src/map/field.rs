//! A field's map: `WORLD::DrawMiniMap` (gcmn 0x005a3940, from `WORLD::Draw`
//! 0x005a97b0 unless `ccGame.inBattle`) and its helpers, with the sprites
//! `WORLD::Init` (0x005a4cd0) makes and the texture `WORLD::Generate`
//! (0x005a6da0) paints: a 80 x 80 height map in the texture's corner that
//! scrolls and wraps under the arrow (the field is a torus), the objects'
//! icons, the dungeon entrance, the fountain and the magic portals. On the
//! map x runs right to left (column 79 - x) and y top to bottom
//! (docs/engine/map.md).

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::field::{Field, Kind};
use piney_data::tables::world::Fieldmapicon;
use piney_desktop::anm::Ctx;
use piney_desktop::assets::SceneFile;
use piney_desktop::kanji::Fonts;
use piney_desktop::message::WindowTexture;
use piney_desktop::view::LayerView;
use piney_draw::{Rgba, TexRef, Upload, UploadFormat};

use super::sprite::{FLIP, Out, Place, Spr, Text};
use super::{LABEL_COLOUR, MAP_LAYER, colour, pulse};
use crate::ee::{self, F, V4, k};

/// The sprites' ids.
pub const MINIMAP: u8 = 0;
pub const ARROW: u8 = 1;
pub const KANJI: u8 = 2;

/// `alpharate$3075`: 0.7.
pub const ALPHA_RATE: F = 0x3f33_3333;
/// `Generate`'s height scale, 0.1240234375.
pub const HEIGHT_SCALE: F = 0x3dfe_0000;
/// `DrawMiniMap`'s story areas with no dungeon on the map.
pub const NO_DUNGEON: [i32; 20] = [113, 112, 111, 110, 109, 82, 81, 80, 79, 78, 57, 56, 55, 54, 53, 42, 41, 40, 39, 28];
/// The eight neighbours of the whole-map copy, then itself first: @2619,
/// @2683 and @2774 (gcmn 0x00658760, 0x006587b0, 0x00658800), all alike.
pub const NINE: [(i32, i32); 9] =
    [(0, 0), (-160, 0), (160, 0), (0, -160), (0, 160), (-160, -160), (160, -160), (-160, 160), (160, 160)];

/// A `FIELDMAPICON` (0x18 bytes): the cell (texels) and where it hangs from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Icon {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub ofsx: i32,
    pub ofsy: i32,
}

impl Icon {
    fn of(i: &Fieldmapicon) -> Icon {
        Icon { x: i.x, y: i.y, w: i.w, h: i.h, ofsx: i.ofsx, ofsy: i.ofsy }
    }
}

/// A `FOBJECT` or `FOBJECT2` as the map reads it: `type`, `idx`, the
/// height cell `mx`, `my`, `wp`, and its icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Obj {
    pub kind: u32,
    pub idx: i32,
    pub mx: i32,
    pub my: i32,
    pub wp: [F; 2],
    pub icon: Option<Icon>,
}

/// A magic portal on the entry control's list (`g_entCtrl+0x20`): its
/// `ccEntryObj` +0x138, +0x13c (the height cell), +0xe0 bit 5 (opened,
/// fading) and +0xec (its alpha, which the map fades).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Circle {
    pub mx: i32,
    pub my: i32,
    pub fading: bool,
    pub alpha: i32,
}

/// What `DrawMiniMap` reads of the world.
#[derive(Debug, PartialEq, Eq)]
pub struct Input<'a> {
    /// `WORLD_MAN.fieldMapMode` (+0x12c): 0 the default map, 1 the whole
    /// map, 2 none.
    pub mode: i32,
    pub alpha: F,
    pub pos: V4,
    pub dirc: V4,
    /// `WORLD_MAN.eventAreaNumber` (+0x120).
    pub event_area: i32,
    /// `ccCheckFountain()`.
    pub fountain: bool,
    pub circles: &'a mut [Circle],
}

/// `WORLD`'s map.
#[derive(Clone, Debug)]
pub struct FieldMap {
    pub spr: [Spr; 2],
    /// `WORLD.mapFlag` (+0x6170): `WORLD::ShowMap` has run.
    pub map_flag: bool,
    /// `r$3072`: the arrow's pulse.
    pub r: F,
    pub field_type: i32,
    /// `fobj2[0 .. KeyObjNum]`.
    pub fobj2: Vec<Obj>,
    /// `fobj[x][y]`, x-major, the chips with an object.
    pub fobj: Vec<Obj>,
    /// `WORLD.water` (+0x30): the lake's index in `fobj2`; `Generate`
    /// starts it at 0 (the dungeon entrance), which a field with no lake
    /// keeps.
    pub water: usize,
    /// `WORLD.dx[0]`, `dy[0]`: the dungeon entrance's height cell.
    pub dungeon: [i32; 2],
    pub fp_dungeon: Icon,
    /// `mapmsg[0]`, `[1]`.
    pub labels: [Vec<u8>; 2],
    /// The texture as `Generate` leaves it, and its palette.
    pub pixels: Vec<u8>,
    pub clut: Vec<Rgba>,
    places: Vec<Place>,
}

/// `Generate`'s texel for a height: `(char) fptoui(h * 0.124) + 128`.
pub fn height_texel(h: F) -> u8 {
    let p = ee::mul(h, HEIGHT_SCALE);
    let v = if ee::lt(p, 0) || ee::eq(p, 0) { 0 } else { ee::to_int(p) as u32 };
    ((v as u8 as i8) as i32 + 128) as u8
}

/// `Generate`'s painting of the height map into `pixels` (256 a row).
pub fn paint_heights(pixels: &mut [u8], field: &Field) {
    for y in 0..80usize {
        for x in 0..80usize {
            pixels[(255 - y) * 256 + 79 - x] = height_texel(field.map[x * 80 + y]);
        }
    }
}

impl FieldMap {
    /// `WORLD::Init`'s map sprites and `Generate`'s texture for `field`
    /// (its objects as `Generate` placed them), its scene file `file` (the
    /// field's `fieldccs`).
    pub fn new(
        archive: &Arc<Archive>,
        volume: piney_data::volume::Volume,
        field: &Field,
        file: &SceneFile,
    ) -> piney_data::Result<FieldMap> {
        let ft = field.params.field_type as i32;
        // KeyIconTBL, SubIconTBL, BaseIconTBL, TreeIconTBL: per field type a
        // FIELDMAPICON array by the object's row.
        let w = piney_data::tables::world::of(volume);
        let table = |t: &[Option<&[Fieldmapicon]>], idx: usize| -> piney_data::Result<Option<Icon>> {
            let Some(icons) = t.get(ft as usize).copied().flatten() else { return Ok(None) };
            let icon = icons.get(idx).ok_or_else(|| piney_data::Error::NotFound(format!("field icon {ft}/{idx}")))?;
            Ok(Some(Icon::of(icon)))
        };
        let mut fobj2 = Vec::new();
        let mut water = 0;
        let mut dungeon = [0; 2];
        let mut grid: Vec<Option<Obj>> = vec![None; 40 * 40];
        for o in &field.objects {
            let icon = match o.kind {
                Kind::Key => table(w.key_icons(), o.index)?,
                Kind::Sub => table(w.sub_icons(), o.index)?,
                Kind::Base => table(w.base_icons(), o.index)?,
                Kind::Tree => table(w.tree_icons(), o.index)?,
                Kind::Entrance | Kind::Lake => None,
            };
            let obj = Obj { kind: o.kind.number(), idx: o.index as i32, mx: o.cell[0], my: o.cell[1], wp: o.pos, icon };
            match o.kind {
                Kind::Base | Kind::Tree => grid[(o.x as usize % 40) * 40 + o.y as usize % 40] = Some(obj),
                _ => {
                    if o.kind == Kind::Entrance {
                        dungeon = [obj.mx, obj.my];
                    }
                    if o.kind == Kind::Lake {
                        water = fobj2.len();
                    }
                    fobj2.push(obj);
                }
            }
        }
        // fieldminimap: the texture of each field type.
        let name = w.field_minimap().get(ft as usize).copied().flatten().unwrap_or_default().to_owned();
        let (textures, cluts) = piney_data::texture::read(&file.ccs)?;
        let obj = file.ccs.find_object(&name).ok_or_else(|| piney_data::Error::NotFound(name.clone()))?;
        let t = textures.iter().find(|t| t.object == obj).ok_or_else(|| piney_data::Error::NotFound(name.clone()))?;
        let mut pixels = t.levels.first().map(|l| l.pixels.clone()).unwrap_or_default();
        pixels.resize(256 * 256, 0);
        paint_heights(&mut pixels, field);
        let clut = cluts.get(&t.clut).map(|c| c.colours.iter().map(|&c| Rgba(c)).collect()).unwrap_or_default();
        let arrow = WindowTexture::read_named(archive, "xallow0", "TEX_xallow0", None)
            .ok_or_else(|| piney_data::Error::NotFound("xallow0::TEX_xallow0".into()))?;
        let view = LayerView::frame(340.0, 28.0, 160.0, 160.0, 1.0, 1.0);
        let places = vec![
            // The minimap's texture is the frame's upload, set at render.
            Place { layer: MAP_LAYER, view, tex: TexRef::Upload(0), tex_h: 256 },
            Place { layer: MAP_LAYER, view, tex: arrow.tex, tex_h: arrow.tex_h },
        ];
        let l = piney_data::tables::world::of(volume).map_labels();
        let labels = [l[0], l[1]].map(piney_data::tables::sjis::encode);
        Ok(FieldMap {
            spr: [Spr::mask(MINIMAP, 512, 64), Spr::mask(ARROW, 16, 16)],
            map_flag: false,
            r: 0,
            field_type: ft,
            fobj2,
            fobj: grid.into_iter().flatten().collect(),
            water,
            dungeon,
            fp_dungeon: Icon::of(&w.fp_dungeon()),
            labels,
            pixels,
            clut,
            places,
        })
    }

    /// Where sprite `spr` draws (the texture of the painted one is the
    /// frame's upload, set by `render`).
    pub fn place(&self, spr: u8) -> Option<&Place> {
        self.places.get(spr as usize)
    }

    /// `WORLD::DrawMiniMap` for one frame: what it sent, in order.
    pub fn draw(&mut self, inp: &mut Input) -> Vec<Out> {
        if inp.mode == 2 {
            return Vec::new();
        }
        let mut out = Vec::new();
        for s in &mut self.spr {
            s.transp = inp.alpha;
        }
        // The kanji: its colour ccSpriteColorTable[7], its alpha word as
        // the constructor left it (128).
        let text = self.labels[if inp.mode == 1 { 0 } else { 1 }].clone();
        out.push(Out::Text(Text {
            spr: KANJI,
            text,
            dx: k(10.0),
            dy: k(6.0),
            rgba: colour(LABEL_COLOUR, 128),
            transp: inp.alpha,
        }));
        let pulse = pulse(&mut self.r);
        let px = ee::to_int(ee::div(inp.pos[0], k(300.0)));
        let py = ee::to_int(ee::div(inp.pos[1], k(300.0)));
        let s0 = 80 - px;
        let mut xpos = [-80, 80];
        let mut ypos = [-80, 80];
        let d = xpos[0] - (s0 << 1);
        if d >= 0 {
            xpos[1] = -240;
        } else if d < -159 {
            xpos[0] = 240;
        }
        let d = ypos[0] - ((py - 80) << 1);
        if d >= 0 {
            ypos[1] = -240;
        } else if d < -159 {
            ypos[0] = 240;
        }
        let (subx, suby) = (s0 << 1, (py - 80) << 1);
        let g = Grid { mode: inp.mode, subx, suby, xpos, ypos, pos: inp.pos };
        self.heights(&g);
        if inp.mode == 0 {
            self.key_objects(&g);
            self.sub_objects(&g);
        }
        if !NO_DUNGEON.contains(&inp.event_area) {
            self.dungeon_icon(&g);
        }
        if inp.fountain {
            self.lake(&g);
        }
        if self.map_flag {
            self.circles(&g, inp.circles);
        }
        let [minimap, arrow] = &mut self.spr;
        arrow.cell(0, 1280, 16, 16, k(16.0), k(16.0));
        arrow.dx = k(80.0);
        arrow.dy = k(80.0);
        arrow.rot = inp.dirc[2];
        arrow.cx = k(-8.0);
        arrow.cy = k(-4.0);
        arrow.set_alpha(pulse);
        arrow.ctrl |= FLIP;
        arrow.make_packet_s(0, 1);
        arrow.ctrl &= !FLIP;
        minimap.cell(1536, 1536, 160, 160, k(160.0), k(160.0));
        minimap.dx = 0;
        minimap.dy = 0;
        minimap.cx = 0;
        minimap.cy = 0;
        minimap.set_alpha(112);
        minimap.make_packet(0, 1);
        out.extend(arrow.send().into_iter().map(Out::Send));
        out.extend(minimap.send().into_iter().map(Out::Send));
        out
    }

    /// `DrawHeightOnMiniMap` (0x005a3170).
    fn heights(&mut self, g: &Grid) {
        let s = &mut self.spr[0];
        let a = ee::to_int(ee::mul(k(112.0), ALPHA_RATE));
        if g.mode == 1 {
            let x = ee::to_int(ee::div(g.pos[0], k(300.0)));
            let y = 80 - ee::to_int(ee::div(g.pos[1], k(300.0)));
            for (ox, oy) in NINE {
                s.cell(0, 0, 80, 80, k(160.0), k(160.0));
                s.dx = ee::from_int(x - 80 + ox);
                s.dy = ee::from_int(y + oy);
                s.cx = 0;
                s.cy = 0;
                s.set_alpha(a);
                s.make_packet(0, 1);
            }
            return;
        }
        for (wu, wv, xi, yi) in [(0, 0, 0, 0), (640, 0, 1, 0), (0, 640, 0, 1), (640, 640, 1, 1)] {
            s.cell(wu, wv, 40, 40, k(160.0), k(160.0));
            s.dx = ee::from_int(g.xpos[xi] - g.subx);
            s.dy = ee::from_int(g.ypos[yi] - g.suby);
            s.cx = 0;
            s.cy = 0;
            if (wu, wv) == (0, 0) {
                s.set_alpha(a);
            }
            s.make_packet(0, 1);
        }
    }

    /// An object's icon at its height cell, `DrawKeyObjectOnMiniMap`'s and
    /// `DrawSubObjectOnMiniMap`'s shared body.
    fn icon(s: &mut Spr, g: &Grid, o: &Obj, icon: &Icon) {
        s.cell(icon.x << 4, icon.y << 4, icon.w, icon.h, ee::from_int(icon.w), ee::from_int(icon.h));
        s.set_alpha(ee::to_int(ee::mul(k(64.0), ALPHA_RATE)));
        let (xb, yb) = g.at(o.mx, o.my);
        s.dx = ee::from_int(xb - g.subx - icon.ofsx);
        s.dy = ee::from_int(yb - g.suby - icon.ofsy);
        s.cx = 0;
        s.cy = 0;
        s.make_packet(0, 1);
    }

    /// `DrawKeyObjectOnMiniMap` (0x005a1b60): `fobj2[]`'s key (type 1) and
    /// sub (type 2) objects.
    fn key_objects(&mut self, g: &Grid) {
        for o in &self.fobj2 {
            if let (1 | 2, Some(icon)) = (o.kind, o.icon) {
                Self::icon(&mut self.spr[0], g, o, &icon);
            }
        }
    }

    /// `DrawSubObjectOnMiniMap` (0x005a1ff0): `fobj[x][y]`'s base (type 3)
    /// and tree (type 4) objects.
    fn sub_objects(&mut self, g: &Grid) {
        for o in &self.fobj {
            if let (3 | 4, Some(icon)) = (o.kind, o.icon) {
                Self::icon(&mut self.spr[0], g, o, &icon);
            }
        }
    }

    /// `DrawLakeOnMiniMap` (0x005a2510): the fountain.
    fn lake(&mut self, g: &Grid) {
        let Some(lake) = self.fobj2.get(self.water).copied() else { return };
        let s = &mut self.spr[0];
        s.cell(2960, 208, 18, 18, k(18.0), k(18.0));
        s.set_alpha(128);
        if g.mode == 1 {
            let x = ee::to_int(ee::div(g.pos[0], k(300.0)));
            let y = 80 - ee::to_int(ee::div(g.pos[1], k(300.0)));
            let lx = 160 - ee::to_int(ee::div(lake.wp[0], k(300.0)));
            let ly = ee::to_int(ee::div(lake.wp[1], k(300.0)));
            for (ox, oy) in NINE {
                s.dx = ee::from_int(lx + (x - 80) - 9 + ox);
                s.dy = ee::from_int(ly + y - 9 + oy);
                s.cx = 0;
                s.cy = 0;
                s.make_packet(0, 1);
            }
            return;
        }
        let (xb, yb) = g.at(lake.mx, lake.my);
        s.dx = ee::from_int(xb - g.subx - 9);
        s.dy = ee::from_int(yb - g.suby - 9);
        s.cx = 0;
        s.cy = 0;
        s.make_packet(0, 1);
    }

    /// `DrawCircleOnMiniMap` (0x005a2840): the magic portals, an opened one
    /// fading by 1 a packet.
    fn circles(&mut self, g: &Grid, circles: &mut [Circle]) {
        let s = &mut self.spr[0];
        s.cell(1280, 0, 16, 16, k(16.0), k(16.0));
        let (mut bx, mut by) = (0, 0);
        if g.mode == 1 {
            bx = ee::to_int(ee::div(g.pos[0], k(300.0))) - 80;
            by = 80 - ee::to_int(ee::div(g.pos[1], k(300.0)));
            s.cell(1408, 128, 4, 4, k(4.0), k(4.0));
        }
        let alpha = |c: &mut Circle| {
            if c.fading {
                if c.alpha != 0 {
                    c.alpha -= 1;
                }
            } else {
                c.alpha = 128;
            }
            c.alpha
        };
        for c in circles.iter_mut() {
            if g.mode == 1 {
                let (cx, cy) = (160 - (c.mx << 1), c.my << 1);
                for (ox, oy) in NINE {
                    s.dx = ee::from_int(cx + bx - 1 + ox);
                    s.dy = ee::from_int(cy + by - 1 + oy);
                    s.cx = 0;
                    s.cy = 0;
                    s.set_alpha(alpha(c));
                    s.make_packet(0, 1);
                }
            } else {
                let (xb, yb) = g.at(c.mx, c.my);
                s.dx = ee::from_int(xb - g.subx - 8);
                s.dy = ee::from_int(yb - g.suby - 8);
                s.cx = 0;
                s.cy = 0;
                s.set_alpha(alpha(c));
                s.make_packet(0, 1);
            }
        }
    }

    /// `DrawDungeonOnMiniMap` (0x005a2c60): the entrance and the red arrow
    /// over it.
    fn dungeon_icon(&mut self, g: &Grid) {
        let fp = self.fp_dungeon;
        let s = &mut self.spr[0];
        let entrance = |s: &mut Spr| {
            s.cell(fp.x << 4, fp.y << 4, fp.w, fp.h, ee::from_int(fp.w), ee::from_int(fp.h));
        };
        let arrow = |s: &mut Spr| s.cell(1296, 1040, 12, 15, k(12.0), k(15.0));
        entrance(s);
        if g.mode == 1 {
            let x = ee::to_int(ee::div(g.pos[0], k(300.0)));
            let y = 80 - ee::to_int(ee::div(g.pos[1], k(300.0)));
            let dx0 = 160 - (self.dungeon[0] << 1);
            let dy0 = self.dungeon[1] << 1;
            for (ox, oy) in NINE {
                entrance(s);
                let sy = dy0 + y;
                let sx = dx0 + (x - 80);
                s.dx = ee::from_int(sx - 9 + ox);
                s.dy = ee::from_int(sy - 9 + oy);
                s.cx = 0;
                s.cy = 0;
                s.set_alpha(128);
                s.make_packet(0, 1);
                arrow(s);
                s.dx = ee::from_int(sx - 6 + ox);
                s.dy = ee::from_int(sy - 17 + oy);
                s.cx = 0;
                s.cy = 0;
                s.make_packet(0, 1);
            }
            return;
        }
        let (xb, yb) = g.at(self.dungeon[0], self.dungeon[1]);
        let (sy, sx) = (yb - g.suby, xb - g.subx);
        s.dx = ee::from_int(sx - fp.ofsx);
        s.dy = ee::from_int(sy - fp.ofsy);
        s.cx = 0;
        s.cy = 0;
        s.set_alpha(128);
        s.make_packet(0, 1);
        arrow(s);
        s.dx = ee::from_int(sx - fp.ofsx + 3);
        s.dy = ee::from_int(sy - fp.ofsy - 8);
        s.cx = 0;
        s.cy = 0;
        s.set_alpha(128);
        s.make_packet(0, 1);
    }

    /// The frame's texture: the painted field texture as an upload.
    pub fn upload(&self, id: u32) -> Upload {
        Upload {
            id,
            width: 256,
            height: 256,
            format: UploadFormat::Psmt8,
            pixels: self.pixels.clone(),
            clut: self.clut.clone(),
        }
    }

    /// A frame of the map into the field's layers: the texture uploaded,
    /// each send at the front of its layer, the label through `fonts`.
    pub fn render(&self, outs: &[Out], fonts: Option<&Fonts>, ctx: &mut Ctx) {
        if outs.is_empty() {
            return;
        }
        let id = ctx.uploads.len() as u32;
        ctx.uploads.push(self.upload(id));
        let place = |spr: u8| -> Option<Place> {
            let mut p = self.places.get(spr as usize)?.clone();
            if spr == MINIMAP {
                p.tex = TexRef::Upload(id);
            }
            Some(p)
        };
        let view = self.places[0].view;
        super::render(outs, &place, &|_| Some((MAP_LAYER, view)), fonts, ctx);
    }
}

/// `DrawMiniMap`'s arguments to its helpers: `subx`, `suby`, `xpos[2]`,
/// `ypos[2]` (the four quarters' places) and the player's position.
struct Grid {
    mode: i32,
    subx: i32,
    suby: i32,
    xpos: [i32; 2],
    ypos: [i32; 2],
    pos: V4,
}

impl Grid {
    /// A height cell's place on the default map (4 px a cell, x mirrored),
    /// before `subx`, `suby`.
    fn at(&self, mx: i32, my: i32) -> (i32, i32) {
        let xc = 79 - mx;
        let xb = if xc >= 40 { self.xpos[1] + ((xc - 40) << 2) } else { self.xpos[0] + (xc << 2) };
        let yb = if my >= 40 { self.ypos[1] + ((my - 40) << 2) } else { self.ypos[0] + (my << 2) };
        (xb, yb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_become_palette_indices() {
        assert_eq!(height_texel(0), 128);
        assert_eq!(height_texel(k(-50.0)), 128);
        assert_eq!(height_texel(k(100.0)), 128 + 12);
        // Past 127 the char wraps.
        assert_eq!(height_texel(k(1100.0)), (136i32 as u8 as i8 as i32 + 128) as u8);
    }
}
