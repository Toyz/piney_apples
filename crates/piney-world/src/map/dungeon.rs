//! A dungeon's map: `DUNGEON::DrawMap` (gcmn 0x005cc160) with the rooms
//! `DUNGEON::MakeMiniMap` (0x005cd850) puts on it as the party enters them,
//! `DUNGEON::ShowMap` (0x005cf260) for the Fairy's Orb, and the sprites
//! the constructor (0x005b7c00) makes.
//!
//! ```text
//! DUNGEON()     mapLayer: priority 50, SetFrame(340, 28, 160, 160, 80, 80, 1, 1)
//!               map  ccMask(2, 0)  DungeonName[type]::TEX_xdsmap01 (the
//!                                  backdrop, and map1's cells)
//!               map1 ccMask(50, 5) TEX_xdsmap01: the dots and signs
//!               map2 ccMask(2, 0)  TEX_xdsmap02: the rooms seen
//!               map3 ccMask(50, 5) xallow0::TEX_xallow0: the arrow
//!               kanji ccInitKanji(16, 0), ctrl 0x10, on fontLayer
//! smallmap      [10][256][256], a byte a 300-unit square: 0 nothing, 1
//!               floor, 2 a door (ground 0x80000), 3 stairs (0x20000), 4 a
//!               ground of kind 0xc0c0
//! MakeMiniMap   (from Draw, for the room under the player; 0 when seen or
//!  (here)       no room) the room seen; a grid of vertical rays from 1500
//!               to -500 every 300 units over the room (12, 22 or 42 a
//!               side by its size; 82 for a story room of type 16 or more)
//!               through ccHitCheckLM, each hit on a square still 0 marking
//!               it by the ground's attribute; the ray's end becomes the
//!               hit point and walks on from there. Then Draw paints
//!               TEX_xdsmap02: square (x, y) is texel (211 - x, 243 - y),
//!               1 -> 255, 2 -> 254, 3 -> 253, 4 -> 251
//! DrawMap       nothing when dungeonMapMode (WORLD_MAN +0x130) is 1 or in
//!               a special room (WORLD_MAN.specialRoom not -1)
//!               mapHideFlag: the texture painted again (0 -> 1) and
//!               ccMenu.mapStatus back to 1 (unless it was 2 or an event
//!               holds the map)
//!               the magic portals (u 239) and the gimmicks (u 243) in
//!               rooms seen on this floor, 4 x 4
//!               the up stairs (u 233, v 45) and the down stairs (u 225,
//!               v 19) at startpos[0], [1] when their room is seen,
//!               pulsing; not in types 8 and 9, where the first fountain
//!               (u 225, v 70) is drawn instead
//!               the arrow (map3, turning, upside down) at (80, 80)
//!               map (224 x 224 texels drawn 448 x 448, alpha 64 x 0.7)
//!               and map2 over it, both at (-24, -24) less the player's
//!               place: 2 px a 300-unit square, x mirrored
//!               SendPacketS map3, map1; SendPacket map2, map
//!               the floor, "B 1" .. (levelstr), at (350, 34) on the font
//!               layer but in types 8 and 9
//! ```

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::volume::Volume;
use piney_desktop::anm::Ctx;
use piney_desktop::assets::SceneFile;
use piney_desktop::kanji::Fonts;
use piney_desktop::message::WindowTexture;
use piney_desktop::view::LayerView;
use piney_draw::{Rgba, TexRef, Upload, UploadFormat};

use super::sprite::{FLIP, Out, Place, Spr, Text};
use super::{FONT_LAYER, LABEL_COLOUR, MAP_LAYER, colour, pulse};
use crate::ee::{self, F, ONE, V4, k};
use crate::hit::Hits;

/// The sprites' ids.
pub const MAP: u8 = 0;
pub const MAP1: u8 = 1;
pub const MAP2: u8 = 2;
pub const MAP3: u8 = 3;
pub const KANJI: u8 = 4;
/// `alpharate$8241`: 0.7.
pub const ALPHA_RATE: F = 0x3f33_3333;
/// No room (`UpRoom`, `DownRoom`, a room index).
pub const NO_ROOM: usize = 15;
pub const FLOORS: usize = 10;
/// `smallmap`'s used side.
pub const SIDE: usize = 200;

/// A character on the entry control's lists as the map reads it: a magic
/// portal (`mcHead`, +0x20) or a gimmick (`gimHead`, +0x2c): its
/// `entParam` id (+0x124), floor (+0x130), block (+0x134), position
/// (+0x100) and `param[2]` (+0x150).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ent {
    pub id: i32,
    pub floor: i32,
    pub block: i32,
    pub pos: [F; 2],
    pub param2: i32,
}

/// What `DrawMap` reads of the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input<'a> {
    /// `WORLD_MAN.dungeonMapMode` (+0x130): 0 on, 1 off.
    pub mode: i32,
    /// `WORLD_MAN.specialRoom` (+0x160): -1 none.
    pub special_room: i32,
    pub alpha: F,
    pub pos: V4,
    pub dirc: V4,
    /// `eventMng` +0x78c: an event holds the map hidden.
    pub event_hold: bool,
    pub circles: &'a [Ent],
    pub gims: &'a [Ent],
}

/// What a room is to the map (`minimap[f][i]` and what `MakeMiniMap`
/// reads).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Room {
    /// `animIdx[f][i].info` set: the room exists.
    pub made: bool,
    /// `pos[f][i]`: its centre.
    pub pos: [F; 2],
    /// `minimap[f][i].size` (0-2) and `.flag` (seen).
    pub size: u8,
    pub seen: bool,
    /// A story room of type 16 or more (`edit->roomname[]`).
    pub special: bool,
}

/// A floor as the map reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Floor {
    pub rooms: Vec<Room>,
    /// `UpRoom[f]`, `DownRoom[f]` (15 none) and `startpos[0][f]`,
    /// `startpos[1][f]`.
    pub up: usize,
    pub down: usize,
    pub start: [V4; 2],
}

/// `DUNGEON`'s map.
#[derive(Clone, Debug)]
pub struct DungeonMap {
    pub spr: [Spr; 4],
    pub r: F,
    /// `DUNGEON.type` (+0x10).
    pub dtype: i32,
    pub floors: Vec<Floor>,
    /// `smallmap[10][200][200]` (the game's rows are 256 wide).
    pub small: Vec<u8>,
    /// `mapHideFlag` (+0x42c).
    pub map_hide: i32,
    /// `TEX_xdsmap02` as painted, and its palette.
    pub pixels: Vec<u8>,
    pub clut: Vec<Rgba>,
    pub labels: Vec<Vec<u8>>,
    places: Vec<Place>,
}

/// What a frame of `DrawMap` asks of the rest of the game.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Drawn {
    pub outs: Vec<Out>,
    /// `ccMenu.mapStatus = 1`.
    pub map_status: bool,
}

/// `MakeMiniMap`'s square for a ground attribute: 3 stairs, 2 a door, 4 a
/// ground of kind 0xc0c0, 0 kind 0x303030, else 1.
pub fn square_of(att: u32) -> u8 {
    if att & 0x2_0000 != 0 {
        3
    } else if att & 0x8_0000 != 0 {
        2
    } else if att & 0x00f0_f0f0 == 0xc0c0 {
        4
    } else if att & 0x00f0_f0f0 == 0x0030_3030 {
        0
    } else {
        1
    }
}

/// The texel index a square paints: `Draw`'s and `ShowMap`'s 255, 254, 253,
/// 251 for 1-4 (else none), `DrawMap`'s repaint 1 for the rest.
pub fn texel_of(square: u8, repaint: bool) -> Option<u8> {
    match square {
        1 => Some(255),
        2 => Some(254),
        3 => Some(253),
        4 => Some(251),
        _ if repaint => Some(1),
        _ => None,
    }
}

/// A position's map place for a dot: `2 (200 - x / 300) - 1` (2 less when
/// `x % 300` is over 200) and `2 (y / 300)` (2 less when `y % 300` is 200
/// or under), in C's integer division.
pub fn dot_of(pos: [F; 2]) -> (i32, i32) {
    let (x, y) = (ee::to_int(pos[0]), ee::to_int(pos[1]));
    let mut sx = ((200 - x / 300) << 1) - 1;
    if x % 300 >= 201 {
        sx -= 2;
    }
    let mut sy = (y / 300) << 1;
    if y % 300 < 201 {
        sy -= 2;
    }
    (sx, sy)
}

impl DungeonMap {
    /// The constructor's map sprites over the dungeon's scene file `file`
    /// (`DungeonName[type]`, the texture set `SetDungeonTexClut` picked).
    pub fn new(
        archive: &Arc<Archive>,
        volume: Volume,
        file: &SceneFile,
        dtype: i32,
        floors: Vec<Floor>,
    ) -> piney_data::Result<DungeonMap> {
        let (textures, cluts) = piney_data::texture::read(&file.ccs)?;
        let find = |name: &str| {
            let obj = file.ccs.find_object(name).ok_or_else(|| piney_data::Error::NotFound(name.into()))?;
            textures.iter().find(|t| t.object == obj).ok_or_else(|| piney_data::Error::NotFound(name.into()))
        };
        let t1 = find("TEX_xdsmap01")?;
        let t2 = find("TEX_xdsmap02")?;
        let mut pixels = t2.levels.first().map(|l| l.pixels.clone()).unwrap_or_default();
        pixels.resize(256 * 256, 0);
        let clut = cluts.get(&t2.clut).map(|c| c.colours.iter().map(|&c| Rgba(c)).collect()).unwrap_or_default();
        let back = TexRef::Ccs { file: file.stem.clone(), texture: t1.object, clut: t1.clut };
        let arrow = WindowTexture::read_named(archive, "xallow0", "TEX_xallow0", None)
            .ok_or_else(|| piney_data::Error::NotFound("xallow0::TEX_xallow0".into()))?;
        let view = LayerView::frame(340.0, 28.0, 160.0, 160.0, 1.0, 1.0);
        let places = vec![
            Place { layer: MAP_LAYER, view, tex: back.clone(), tex_h: 1 << t1.th },
            Place { layer: MAP_LAYER, view, tex: back, tex_h: 1 << t1.th },
            Place { layer: MAP_LAYER, view, tex: TexRef::Upload(0), tex_h: 256 },
            Place { layer: MAP_LAYER, view, tex: arrow.tex, tex_h: arrow.tex_h },
        ];
        // levelstr: "B 1" .. "B 10".
        let level_str = piney_data::tables::world::of(volume).level_str();
        let labels = (0..FLOORS)
            .map(|i| piney_data::tables::sjis::encode(level_str.get(i).copied().flatten().unwrap_or_default()))
            .collect();
        Ok(DungeonMap {
            spr: [Spr::mask(MAP, 2, 0), Spr::mask(MAP1, 50, 5), Spr::mask(MAP2, 2, 0), Spr::mask(MAP3, 50, 5)],
            r: 0,
            dtype,
            floors,
            small: vec![0; FLOORS * SIDE * SIDE],
            map_hide: 0,
            pixels,
            clut,
            labels,
            places,
        })
    }

    /// Where sprite `spr` draws (the texture of the painted one is the
    /// frame's upload, set by `render`).
    pub fn place(&self, spr: u8) -> Option<&Place> {
        self.places.get(spr as usize)
    }

    fn room(&self, f: usize, i: usize) -> Option<&Room> {
        self.floors.get(f).and_then(|fl| fl.rooms.get(i))
    }

    fn seen(&self, f: i32, i: i32) -> bool {
        f >= 0 && i >= 0 && self.room(f as usize, i as usize).is_some_and(|r| r.seen)
    }

    /// `smallmap[f][x][y]`.
    pub fn square(&self, f: usize, x: usize, y: usize) -> u8 {
        self.small[(f * SIDE + x) * SIDE + y]
    }

    /// The texture painted from floor `f`'s squares: `Draw` and `ShowMap`
    /// only over squares 1-4, `DrawMap`'s repaint over all.
    pub fn paint(&mut self, f: usize, repaint: bool) {
        for y in 0..SIDE {
            for x in 0..SIDE {
                if let Some(t) = texel_of(self.square(f, x, y), repaint) {
                    self.pixels[(255 - (y + 12)) * 256 + (199 - x) + 12] = t;
                }
            }
        }
    }

    /// `DUNGEON::MakeMiniMap(here)` on floor `f`, rays through `hits`:
    /// true when the room was new to the map (the caller paints).
    pub fn make_mini_map(&mut self, hits: &mut Hits, f: usize, here: usize) -> bool {
        if here >= NO_ROOM {
            return false;
        }
        let Some(room) = self.floors.get_mut(f).and_then(|fl| fl.rooms.get_mut(here)) else { return false };
        if room.seen {
            return false;
        }
        room.seen = true;
        let room = *room;
        let (half, n) = if room.special {
            (k(6300.0), 82)
        } else {
            match room.size {
                0 => (k(1800.0), 12),
                1 => (k(3300.0), 22),
                _ => (k(6300.0), 42),
            }
        };
        let x0 = ee::add(k(150.0), ee::sub(room.pos[0], half));
        let y0 = ee::add(k(15.0), ee::add(k(150.0), ee::sub(room.pos[1], half)));
        let mut from: V4 = [room.pos[0], y0, k(1500.0), ONE];
        let mut to: V4 = [room.pos[0], y0, k(-500.0), ONE];
        for _ in 0..n {
            from[0] = x0;
            to[0] = x0;
            for _ in 0..n {
                let sx = ee::to_int(ee::div(ee::sub(from[0], k(150.0)), k(300.0)));
                let sy = ee::to_int(ee::div(ee::sub(from[1], k(165.0)), k(300.0)));
                if sx < 200
                    && sy < 200
                    && let Some((cp, _)) = hits.line(from, to, 0xffff_ffff, 0)
                {
                    to = cp;
                    if sx >= 0 && sy >= 0 {
                        let at = (f * SIDE + sx as usize) * SIDE + sy as usize;
                        if self.small[at] == 0 {
                            self.small[at] = square_of(hits.nearest.att);
                        }
                    }
                }
                from[0] = ee::add(from[0], k(300.0));
                to[0] = ee::add(to[0], k(300.0));
                from[2] = k(1500.0);
                to[2] = k(-500.0);
            }
            from[1] = ee::add(from[1], k(300.0));
            to[1] = ee::add(to[1], k(300.0));
        }
        true
    }

    /// `DUNGEON::Draw`'s part: the room under the player to the map, and
    /// the texture painted when it was new.
    pub fn enter_room(&mut self, hits: &mut Hits, f: usize, here: Option<usize>) {
        if self.make_mini_map(hits, f, here.unwrap_or(NO_ROOM)) {
            self.paint(f, false);
        }
    }

    /// The next room of floor `f` that `DUNGEON::ShowMap` would scan: made
    /// and not yet seen.
    pub fn unseen(&self, f: usize) -> Option<usize> {
        self.floors.get(f)?.rooms.iter().position(|r| r.made && !r.seen)
    }

    /// `DUNGEON::DrawMap` on floor `f` for one frame.
    pub fn draw(&mut self, f: usize, inp: &Input) -> Drawn {
        let mut d = Drawn::default();
        if inp.mode == 1 || inp.special_room != -1 {
            return d;
        }
        if self.map_hide != 0 {
            self.paint(f, true);
            if self.map_hide != 2 && !inp.event_hold {
                d.map_status = true;
            }
            self.map_hide = 0;
        }
        for s in &mut self.spr {
            s.transp = inp.alpha;
        }
        let pulse = pulse(&mut self.r);
        let px = 200 - ee::to_int(ee::div(inp.pos[0], k(300.0)));
        let py = ee::to_int(ee::div(inp.pos[1], k(300.0)));
        let s1 = ee::to_int(ee::sub(ee::from_int(px << 1), k(80.0)));
        let s5 = ee::to_int(ee::sub(ee::from_int(py << 1), k(80.0)));
        let fi = f as i32;
        let floor = self.floors.get(f).cloned().unwrap_or_default();
        let seen = |fl: i32, b: i32| self.seen(fl, b);
        let circles: Vec<(i32, i32)> =
            inp.circles.iter().filter(|e| e.floor == fi && seen(e.floor, e.block)).map(|e| dot_of(e.pos)).collect();
        let gims: Vec<(Ent, (i32, i32))> = inp
            .gims
            .iter()
            .filter(|e| e.floor == fi && seen(e.floor, e.block))
            .filter(|e| e.id < 6 || e.id == 17 || (38..45).contains(&e.id))
            .map(|e| (*e, dot_of(e.pos)))
            .collect();
        // The fountain's search tests the block's room on this floor, not
        // the gimmick's floor.
        let fountain = inp.gims.iter().find(|e| e.id == 20 && seen(fi, e.block)).map(|e| {
            let x = 200 - ee::to_int(ee::div(e.pos[0], k(300.0)));
            (x, ee::to_int(ee::div(e.pos[1], k(300.0))))
        });
        let up = (floor.up != NO_ROOM && seen(fi, floor.up as i32)).then_some(floor.start[0]);
        let down = (floor.down != NO_ROOM && seen(fi, floor.down as i32)).then_some(floor.start[1]);
        let special = self.dtype == 8 || self.dtype == 9;
        let [map, map1, map2, map3] = &mut self.spr;
        // The magic portals.
        map1.rot = 0;
        map1.cx = 0;
        map1.cy = 0;
        map1.cell(3824, 16, 4, 4, k(4.0), k(4.0));
        for (sx, sy) in circles {
            map1.dx = ee::from_int(sx - s1);
            map1.dy = ee::from_int(sy - s5);
            map1.set_alpha(128);
            map1.make_packet(0, 1);
        }
        // The gimmicks: a type 17 or 38-44 only while its param[2] is set.
        map1.rot = 0;
        map1.cx = 0;
        map1.cy = 0;
        map1.cell(3888, 16, 4, 4, k(4.0), k(4.0));
        for (e, (sx, sy)) in gims {
            map1.dx = ee::from_int(sx - s1);
            map1.dy = ee::from_int(sy - s5);
            map1.set_alpha(128);
            if (e.id == 17 || (38..45).contains(&e.id)) && e.param2 == 0 {
                continue;
            }
            map1.make_packet(0, 1);
        }
        // The stairs.
        let stairs = |s: &mut Spr, at: V4, wu: i32, wv: i32, su: i32, cx: F| {
            let x = 200 - ee::to_int(ee::div(at[0], k(300.0)));
            let y = ee::to_int(ee::div(at[1], k(300.0)));
            s.cell(wu, wv, su, 10, ee::from_int(su), k(10.0));
            s.dx = ee::from_int((x << 1) - s1);
            s.dy = ee::from_int((y << 1) - s5);
            s.rot = 0;
            s.cx = cx;
            s.cy = k(-5.0);
            s.set_alpha(pulse);
        };
        if let Some(at) = up {
            stairs(map1, at, 3728, 720, 14, k(-6.0));
            if !special {
                map1.make_packet(0, 1);
            }
        }
        if let Some(at) = down {
            stairs(map1, at, 3600, 304, 29, k(-15.0));
            map1.make_packet(0, 1);
        }
        if special && let Some((x, y)) = fountain {
            map1.rot = 0;
            map1.cx = 0;
            map1.cy = 0;
            map1.cell(3600, 1120, 18, 18, k(18.0), k(18.0));
            map1.dx = ee::from_int((x << 1) - 9 - s1);
            map1.dy = ee::from_int((y << 1) - 9 - s5);
            map1.set_alpha(128);
            map1.make_packet(0, 1);
        }
        // The arrow.
        map3.cell(0, 0, 16, 16, k(16.0), k(16.0));
        map3.dx = k(80.0);
        map3.dy = k(80.0);
        map3.rot = inp.dirc[2];
        map3.cx = k(-8.0);
        map3.cy = k(-4.0);
        map3.set_alpha(pulse);
        map3.ctrl |= FLIP;
        map3.make_packet_s(0, 1);
        map3.ctrl &= !FLIP;
        // The backdrop and the rooms seen.
        let (mx, my) = (ee::from_int(-24 - s1), ee::from_int(-24 - s5));
        map.cell(0, 0, 224, 224, k(448.0), k(448.0));
        map.dx = mx;
        map.dy = my;
        map.set_alpha(ee::to_int(ee::mul(k(64.0), ALPHA_RATE)));
        map.make_packet(0, 1);
        map2.cell(0, 0, 224, 224, k(448.0), k(448.0));
        map2.dx = mx;
        map2.dy = my;
        map2.set_alpha(128);
        map2.make_packet(0, 1);
        d.outs.extend(map3.send().into_iter().map(Out::Send));
        d.outs.extend(map1.send().into_iter().map(Out::Send));
        d.outs.push(Out::Send(map2.send_main()));
        d.outs.push(Out::Send(map.send_main()));
        if !special {
            d.outs.push(Out::Text(Text {
                spr: KANJI,
                text: self.labels.get(f).cloned().unwrap_or_default(),
                dx: k(350.0),
                dy: k(34.0),
                rgba: colour(LABEL_COLOUR, 128),
                transp: inp.alpha,
            }));
        }
        d
    }

    /// The frame's texture of the rooms seen, as an upload.
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

    /// A frame of the map into the field's layers.
    pub fn render(&self, outs: &[Out], fonts: Option<&Fonts>, ctx: &mut Ctx) {
        if outs.is_empty() {
            return;
        }
        let id = ctx.uploads.len() as u32;
        ctx.uploads.push(self.upload(id));
        let place = |spr: u8| -> Option<Place> {
            let mut p = self.places.get(spr as usize)?.clone();
            if spr == MAP2 {
                p.tex = TexRef::Upload(id);
            }
            Some(p)
        };
        super::render(outs, &place, &|_| Some((FONT_LAYER, LayerView::default_layer())), fonts, ctx);
    }
}

/// The floors as the map reads them from the dungeon `MakeFloor` built:
/// each slot's centre and size (`minimap[f][i].size` is the room's size,
/// `MakeRoom` 0x005ba8fc / 0x005bd800), and for a story dungeon (`edit`)
/// the rooms of type 16 or more, which get a centre but no model and scan
/// as a large area.
pub fn floors_of(
    area: &crate::dungeon_area::DungeonArea,
    edit: Option<&piney_data::dungeon::EditDungeon>,
) -> Vec<Floor> {
    const CELL: F = 0x443b_8000;
    let mut out: Vec<Floor> = area
        .floors
        .iter()
        .map(|fl| Floor {
            rooms: fl
                .rooms
                .iter()
                .map(|s| Room { made: s.made, pos: s.pos, size: s.size, seen: false, special: false })
                .collect(),
            up: fl.up,
            down: fl.down,
            start: fl.start,
        })
        .collect();
    for rd in edit.map(|e| e.rooms).unwrap_or(&[]) {
        let (Ok(f), Ok(i)) = (usize::try_from(rd.floor), usize::try_from(rd.index)) else { continue };
        let Some(room) = out.get_mut(f).and_then(|fl| fl.rooms.get_mut(i)) else { continue };
        room.size = rd.size.index();
        if rd.room_type >= 16 {
            room.special = true;
            room.pos = [ee::mul(CELL, ee::from_int(rd.x + 8)), ee::mul(CELL, ee::from_int(rd.y + 8))];
        }
    }
    out
}

/// `DUNGEON::ShowMap` (0x005cf260), one call (one frame of the events'
/// `show_map`): the room under the player deleted; the first room of the
/// floor built and not yet seen built, put on the map by `MakeMiniMap`,
/// painted and deleted (false: not done); with none left, the room under
/// the player built again (true: done).
pub fn show_map_step(area: &mut crate::dungeon_area::DungeonArea, player: V4) -> bool {
    let f = area.level;
    let here = area.here(player);
    area.delete_room();
    let Some(mut map) = area.map.take() else { return true };
    let done = match map.unseen(f) {
        Some(i) => {
            area.set_room(f, i);
            map.make_mini_map(&mut area.hits, f, i);
            map.paint(f, false);
            area.delete_room();
            false
        }
        None => {
            if let Some(h) = here {
                area.set_room(f, h);
            }
            true
        }
    };
    area.map = Some(map);
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dots_round_as_the_game_does() {
        // x 1000: 200 - 3 = 197, 393, rem 100: 393; y 1000: 6, rem 100 < 201: 4.
        assert_eq!(dot_of([k(1000.0), k(1000.0)]), (393, 4));
        // x 1450: 200 - 4 = 196, 391, rem 250 > 200: 389.
        assert_eq!(dot_of([k(1450.0), k(1450.0)]).0, 389);
        assert_eq!(dot_of([k(1450.0), k(1450.0)]).1, 8);
        assert_eq!(square_of(0x2_0000), 3);
        assert_eq!(square_of(0x8_0000), 2);
        assert_eq!(square_of(0xc0c0), 4);
        assert_eq!(square_of(0x30_3030), 0);
        assert_eq!(square_of(0x1), 1);
    }
}
