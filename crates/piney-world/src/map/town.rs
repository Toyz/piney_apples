//! A Root Town's map. Mac Anu's is `ROOTTOWN01::DrawMap` (gcmn 0x00422d00,
//! through the vtable at 0x00375f90 +0x1c from `ROOTTOWN01::Draw`), with the
//! three sprites the constructor (0x00421470) makes: the map scrolled under
//! the arrow, the arrow turned by the player's heading, and the signs fading
//! in and out. Dun Loireag's `DrawMap` (0x00425760) puts the arrow in screen
//! pixels and dims a sign under it. From Mutation on, Dun Loireag and
//! Carmina Gade draw on two layers of their own and show the flag race's
//! racers (docs/engine/map.md).

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::volume::Volume;
use piney_desktop::message::WindowTexture;
use piney_desktop::view::LayerView;

use super::sprite::{FLIP, MIRROR, Out, Place, Spr};
use super::{FONT_LAYER, MAP_LAYER, pulse};
use crate::ee::{self, F, V4, k};

/// The map sprites' ids.
pub const MAP: u8 = 0;
pub const ICONS: u8 = 1;
pub const ARROW: u8 = 2;
/// From Mutation on: the flag race's racers.
pub const MARKS: u8 = 3;

/// The layers of a later volume's Dun Loireag and Carmina Gade (`+0x1b4`,
/// `+0x1b8`).
pub const OWN_MAP_LAYER: i16 = 40;
pub const MARK_LAYER: i16 = 45;

/// A flag race's racer as the town map reads it: its `pos` (+0x40) and
/// its fade (+0x1ec, 0 to 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Racer {
    pub pos: V4,
    pub fade: F,
}

/// An `ICONPOS` (0x24 bytes): a sign's place on the map texture, its label
/// cell, where it was last drawn and its fade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IconPos {
    pub x: i32,
    pub y: i32,
    /// The label's cell (texels).
    pub u: i32,
    pub v: i32,
    /// Where the balloon and the label were last drawn (-1 never).
    pub tmp: [i32; 2],
    pub tmp2: [i32; 2],
    pub alpha: i32,
}

/// What `DrawMap` reads of the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    /// `WORLD_MAN.townMapMode` (+0x128): 0 on, 1 off.
    pub mode: i32,
    /// `WORLD_MAN.mapAlpha` (+0x148).
    pub alpha: F,
    /// The player's `pos` (+0x40) and `dirc` (+0x60).
    pub pos: V4,
    pub dirc: V4,
    /// From Mutation on, while the flag race runs in the town: its three
    /// racers (`0x00774a90`), each while there is one.
    pub race: Option<[Option<Racer>; 3]>,
}

/// A town's map as its `ROOTTOWN` class keeps it.
#[derive(Clone, Debug)]
pub struct TownMap {
    /// `game.town`: 0 Mac Anu's `DrawMap`, 1 Dun Loireag's, 2 Carmina
    /// Gade's.
    pub town: i32,
    /// Mutation's and the later volumes' `DrawMap` (Infection's otherwise).
    pub later: bool,
    /// map[0] - map[3]; map[3] only from Mutation on, outside Mac Anu.
    pub spr: [Spr; 4],
    /// `RT01ICONPOS` (`RT02ICONPOS`, `RT03ICONPOS`) as the draws leave it (a
    /// global: the game keeps it across visits).
    pub icons: [IconPos; 6],
    /// `r$1560` (`r$1488`, ..): the arrow's pulse angle.
    pub r: F,
    pub mapw: i32,
    pub maph: i32,
    pub stepw: F,
    pub steph: F,
    places: Vec<Place>,
}

impl TownMap {
    /// The constructor's map part for Mac Anu (`town01`, or `town01d` in
    /// the crisis: `stem`), `RT01ICONPOS` as the volume holds it.
    pub fn new(archive: &Arc<Archive>, volume: Volume, stem: &str) -> piney_data::Result<TownMap> {
        TownMap::open(archive, volume, 0, stem)
    }

    /// The constructor's map part for town `town` (0 Mac Anu, 1 Dun
    /// Loireag, from Mutation on 2 Carmina Gade) of the file `stem`.
    pub fn open(archive: &Arc<Archive>, volume: Volume, town: i32, stem: &str) -> piney_data::Result<TownMap> {
        let later = volume != Volume::Inf;
        // stepw, steph: each constructor's (+0x88, +0x8c); Fort Ouph's 0.8
        // (OUT gcmn 0x00439e54), Lia Fail's 2.0 (0x0043c7ec).
        let (iconpos, tex_name, stepw, steph) = match town {
            0 => ("RT01ICONPOS", "TEX_sr1map1", 0x3fc6_6666, 0x3fcc_cccd),
            1 => ("RT02ICONPOS", "TEX_sr2map1", 0x3fa3_d70a, 0x3fa5_e354),
            2 => ("RT03ICONPOS", "TEX_sr3map1", 0x3f82_8f5c, 0x3f81_a9fc),
            3 => ("RT04ICONPOS", "TEX_sr4map1", 0x3f4c_cccd, 0x3f4c_cccd),
            4 => ("RT05ICONPOS", "TEX_sr5map1", 0x4000_0000, 0x4000_0000),
            n => return Err(piney_data::Error::NotFound(format!("town {n}'s map"))),
        };
        // Infection's Dun Loireag puts its arrow on the font layer; a later
        // volume's puts its map and racers on layers of the class's own, as
        // do the towns the port takes from the later volumes (2-4) on any
        // disc.
        let own = (later && town != 0) || town >= 2;
        let (map_layer, arrow_layer) = match (own, town) {
            (true, _) => (OWN_MAP_LAYER, MAP_LAYER),
            (false, 1) => (MAP_LAYER, FONT_LAYER),
            _ => (MAP_LAYER, MAP_LAYER),
        };
        let rows = piney_data::tables::world::of(volume)
            .town_icons()
            .iter()
            .find(|b| b.name == iconpos)
            .map(|b| b.rows)
            .ok_or_else(|| piney_data::Error::NotFound(iconpos.into()))?;
        let mut icons = [IconPos { x: 0, y: 0, u: 0, v: 0, tmp: [0; 2], tmp2: [0; 2], alpha: 0 }; 6];
        for (ic, r) in icons.iter_mut().zip(rows) {
            *ic = IconPos {
                x: r.x,
                y: r.y,
                u: r.u,
                v: r.v,
                tmp: [r.tmpx, r.tmpy],
                tmp2: [r.tmpx2, r.tmpy2],
                alpha: r.alpha,
            };
        }
        let tex = |file: &str, name: &str| {
            WindowTexture::read_named(archive, file, name, None)
                .ok_or_else(|| piney_data::Error::NotFound(format!("{file}::{name}")))
        };
        let map = tex(stem, tex_name)?;
        let arrow = tex("xallow0", "TEX_xallow0")?;
        let view = LayerView::frame(340.0, 36.0, 160.0, 160.0, 1.0, 1.0);
        let arrow_view = if arrow_layer == MAP_LAYER { view } else { LayerView::default_layer() };
        let mut places = vec![
            Place { layer: map_layer, view, tex: map.tex.clone(), tex_h: map.tex_h },
            Place { layer: FONT_LAYER, view: LayerView::default_layer(), tex: map.tex.clone(), tex_h: map.tex_h },
            Place { layer: arrow_layer, view: arrow_view, tex: arrow.tex, tex_h: arrow.tex_h },
        ];
        if own {
            places.push(Place { layer: MARK_LAYER, view: LayerView::default_layer(), tex: map.tex, tex_h: map.tex_h });
        }
        Ok(TownMap {
            town,
            later,
            spr: [
                Spr::mask(MAP, 128, 32),
                Spr::mask(ICONS, 128, 32),
                Spr::mask(ARROW, 6, 6),
                Spr::mask(MARKS, 128, 32),
            ],
            icons,
            r: 0,
            mapw: 140,
            maph: 256,
            stepw,
            steph,
            places,
        })
    }

    /// Where sprite `spr` draws.
    pub fn place(&self, spr: u8) -> Option<&Place> {
        self.places.get(spr as usize)
    }

    /// `ROOTTOWN01::DrawMap` (or `ROOTTOWN02`'s, `ROOTTOWN03`'s) for one
    /// frame: what it sent, in order.
    pub fn draw(&mut self, inp: &Input) -> Vec<Out> {
        let mut addy = 0;
        let mut cy = (self.maph - 160) / 2;
        if inp.mode == 1 {
            return Vec::new();
        }
        let alpha = pulse(&mut self.r);
        if cy < 0 {
            cy = 0;
        }
        for s in &mut self.spr {
            s.transp = inp.alpha;
        }
        let own = self.later && self.town != 0;
        // Carmina Gade's map lies (16, 46) texels into its texture.
        let (ox, oy) = if self.town == 2 { (16, 46) } else { (0, 0) };
        let dirc = inp.dirc;
        let pos = inp.pos;
        let mut x = ee::mul(ee::div(pos[0], k(100.0)), self.stepw);
        let mut y = ee::mul(ee::div(ee::mul(k(-1.0), pos[1]), k(100.0)), self.steph);
        if self.town == 2 {
            x = ee::add(x, k(16.0));
            y = ee::add(y, k(46.0));
        }
        if cy != 0 {
            addy = ee::to_int(y);
            if cy < addy {
                addy = cy;
                y = ee::sub(y, ee::from_int(cy));
            } else if addy < -cy {
                addy = -cy;
                y = ee::add(y, ee::from_int(cy));
            } else {
                y = 0;
            }
        }
        let top = cy + addy;
        let (fcy, faddy) = (ee::from_int(cy), ee::from_int(addy));
        let [map, icons, arrow, marks] = &mut self.spr;
        // The map: 140 x 160 texels from row cy + addy.
        map.sx = k(140.0);
        map.sy = k(160.0);
        map.su = 140;
        map.sv = 160;
        map.wu = 0;
        map.wv = top << 4;
        map.wi = 1;
        map.dx = 0;
        map.dy = 0;
        map.cx = 0;
        map.cy = 0;
        map.set_alpha(128);
        map.make_packet(0, 1);
        // The flag race's racers, where they are inside the map's view.
        let race = own && inp.race.is_some();
        if own {
            marks.cx = 0;
            marks.cy = 0;
        }
        for (i, r) in inp.race.iter().flatten().enumerate().filter(|_| own) {
            let Some(r) = r else { continue };
            let px = ee::to_int(ee::mul(ee::from_int(ee::to_int(ee::div(r.pos[0], k(100.0)))), self.stepw)) + ox;
            let ry = ee::from_int(ee::to_int(ee::mul(k(-1.0), r.pos[1])));
            let py = ee::to_int(ee::mul(ee::from_int(ee::to_int(ee::div(ry, k(100.0)))), self.steph)) + oy;
            let at = ee::add(k(80.0), ee::add(k(36.0), ee::from_int(py)));
            let fy = ee::add(k(24.0), ee::sub(ee::sub(at, fcy), faddy));
            if ee::le(fy, k(36.0)) || !ee::lt(fy, k(196.0)) {
                continue;
            }
            marks.cell(3712, (24 * i as i32 + 144) << 4, 24, 24, k(24.0), k(24.0));
            marks.dx = ee::add(k(12.0), ee::sub(ee::add(k(420.0), ee::from_int(px)), k(24.0)));
            marks.dy = ee::sub(fy, k(12.0));
            marks.set_alpha(ee::to_int(ee::mul(k(128.0), r.fade)));
            // Faded within 11 pixels of the view's top and bottom.
            let near = ee::to_int(ee::sub(fy, k(36.0)));
            if near < 11 {
                marks.set_alpha(ee::to_int(ee::mul(ee::from_int(12 * near), r.fade)));
            }
            let near = ee::to_int(ee::sub(k(172.0), fy)).max(0);
            if near < 11 {
                marks.set_alpha(ee::to_int(ee::mul(ee::from_int(12 * near), r.fade)));
            }
            marks.make_packet(0, 1);
            marks.ctrl &= !MIRROR;
        }
        // The arrow: on the map's own view, except Infection's Dun
        // Loireag's in screen pixels.
        arrow.cell(0, 0, 16, 16, k(16.0), k(16.0));
        match (self.town, own) {
            (1, false) => {
                arrow.dx = ee::add(k(68.0), ee::add(k(340.0), x));
                arrow.dy = ee::sub(ee::add(k(116.0), y), k(14.0));
            }
            (0, _) => {
                arrow.dx = ee::add(k(68.0), x);
                arrow.dy = ee::add(k(4.0), ee::add(k(80.0), y));
            }
            (t, _) => {
                arrow.dx = ee::add(if t == 2 { k(70.0) } else { k(68.0) }, x);
                arrow.dy = ee::sub(ee::add(k(80.0), y), k(14.0));
            }
        }
        // Dun Loireag's arrow's point in screen pixels.
        let (ax, ay) =
            (ee::to_int(ee::add(k(68.0), ee::add(k(340.0), x))), ee::to_int(ee::sub(ee::add(k(116.0), y), k(14.0))));
        arrow.rot = dirc[2];
        arrow.cx = k(-8.0);
        arrow.cy = k(-10.0);
        arrow.set_alpha(alpha);
        arrow.make_packet_s(0, 1);
        // The signs; none draws while the race runs.
        icons.cx = 0;
        icons.cy = 0;
        let half = self.mapw / 2;
        for (i, row) in self.icons.iter_mut().enumerate() {
            let right = half < row.x;
            // Carmina Gade's sign 3.
            let odd = self.town == 2 && i == 3;
            if top < row.y {
                icons.cell(2624, 0, 54, 59, k(54.0), k(59.0));
                if self.town == 2 {
                    map.cx = k(-27.0);
                    map.cy = k(-29.0);
                    if !race {
                        if odd {
                            icons.ctrl |= MIRROR | FLIP;
                        } else {
                            icons.ctrl &= !(MIRROR | FLIP);
                        }
                    }
                }
                let fy = ee::sub(ee::sub(ee::sub(ee::add(k(36.0), ee::from_int(row.y)), fcy), faddy), k(59.0));
                let fx = ee::add(k(340.0), ee::from_int(row.x));
                // A sign right of the middle hangs its balloon to the
                // left; one left of it (or on it) mirrors the balloon.
                if right {
                    icons.dx = fx;
                    icons.dy = fy;
                    row.tmp = [ee::to_int(fx), ee::to_int(fy)];
                } else {
                    let dx = ee::sub(ee::add(k(6.0), fx), k(54.0));
                    icons.dx = dx;
                    icons.dy = fy;
                    row.tmp = [ee::to_int(dx), ee::to_int(fy)];
                    if !race {
                        icons.ctrl |= MIRROR;
                    }
                }
                // Dun Loireag's: the arrow's point inside the balloon's box
                // dims the sign.
                let (bx, by) = (row.tmp[0], row.tmp[1]);
                let under = self.town == 1 && ax >= bx && bx + 54 >= ax && ay >= by + 10 && by + 49 >= ay;
                let alpha = if under { 64 } else { row.alpha };
                if !race {
                    icons.set_alpha(alpha);
                    icons.make_packet(0, 1);
                    icons.ctrl &= !MIRROR;
                }
                icons.cell(row.u << 4, row.v << 4, 25, 24, k(25.0), k(24.0));
                icons.ctrl &= !(MIRROR | FLIP);
                let ly = ee::add(k(14.0), fy);
                let lx = ee::add(k(15.0), fx);
                let lx = if right { lx } else { ee::add(k(6.0), ee::sub(lx, k(54.0))) };
                icons.dx = lx;
                icons.dy = if odd && !right { ee::add(k(6.0), ly) } else { ly };
                row.tmp2 = [ee::to_int(lx), ee::to_int(ly)];
                if !race {
                    icons.set_alpha(alpha);
                    icons.make_packet(0, 1);
                }
                if row.alpha != 128 {
                    row.alpha = (row.alpha + 4).min(128);
                }
            } else if row.alpha != 0 && row.tmp[0] != -1 {
                // Fading out where it last stood.
                icons.cell(2624, 0, 54, 59, k(54.0), k(59.0));
                icons.dx = ee::from_int(row.tmp[0]);
                icons.dy = ee::from_int(row.tmp[1]);
                icons.cx = 0;
                icons.cy = 0;
                if !race {
                    icons.set_alpha(row.alpha);
                    if right {
                        icons.ctrl &= !MIRROR;
                    } else {
                        icons.ctrl |= MIRROR;
                    }
                    icons.make_packet(0, 1);
                    icons.ctrl &= !MIRROR;
                }
                icons.cell(row.u << 4, row.v << 4, 25, 24, k(25.0), k(24.0));
                icons.dx = ee::from_int(row.tmp2[0]);
                icons.dy = ee::from_int(row.tmp2[1]);
                icons.cx = 0;
                icons.cy = 0;
                if !race {
                    icons.set_alpha(row.alpha);
                    icons.make_packet(0, 1);
                }
                row.alpha = (row.alpha - 4).max(0);
            } else {
                row.alpha = 0;
            }
        }
        // The shades at the map's top and bottom, turned a quarter.
        for (dy, rot, a) in [
            (k(12.0), 0xbfc9_0fdb, if top < 10 { (top << 7) / 10 } else { 128 }),
            (k(148.0), 0x3fc9_0fdb, if addy >= 39 { ((cy - addy) << 7) / 10 } else { 128 }),
        ] {
            map.cell(2240, 0, 24, 140, k(24.0), k(140.0));
            map.dx = k(71.0);
            map.dy = dy;
            map.rot = rot;
            map.cx = k(-12.0);
            map.cy = k(-70.0);
            map.set_alpha(a);
            map.make_packet_s(0, 1);
        }
        let order: &[u8] = if own { &[ARROW, MARKS, MAP, ICONS] } else { &[ARROW, MAP, ICONS] };
        let mut out = Vec::new();
        for &s in order {
            out.extend(self.spr[s as usize].send().into_iter().map(Out::Send));
        }
        out
    }
}
