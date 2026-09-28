//! Carmina Gade: `ROOTTOWN03` (MUT gcmn town03.cpp, constructor 0x0043b980,
//! `Draw` 0x0043e400 through the vtable at 0x00388f60), the third Root
//! Town (`game.town` 2), new in Mutation. Infection's executable carries an
//! earlier `ROOTTOWN03` no disc reaches; this is Mutation's.
//!
//! ```text
//! ROOTTOWN03()  the map's layers: WORLD_MAN +0x490 (50), +0x1b4 (40) over
//!   0x0043b980  (340, 36) 160 x 160, +0x1b8 (45) the screen; town03
//!               (town03d in crisis); the AIRSHIP (town03Ship); SetFog(3000,
//!               15000, 0, 85, 0x0014140c) and ccSys.bgColor the same; 34
//!               STATICMODELs (RT_MODELTABLE03), no STATICOBJECTs; the
//!               clumps CMP_sr3bac1, _bac2, _mou1, _mou2 (+0x1c8..+0x1d4),
//!               in crisis CMP_sr3dat1_1-3 (+0x74..+0x7c), each SetFogSw(0);
//!               the map's four sprites (TEX_sr3map1 three times, xallow0);
//!               the lights from ANM_sr3bac1a (town03Light: LGT_sr3lig1,
//!               LGT_omni01-11); BLT_bg, _obj, _obj2, _floor; three ccAnms
//!               of ANM_sr3wat1a, SetFogSw(0), OBJ_sr3wat00 duplicated
//!               (8200, 8192, 8192), rooted at the origin and stepped once;
//!               a 128 x 128 ccTexChunk (+0x1f4) swapped into water 0's
//!               model
//! Draw()        cameraGetPos; on effLayer waterUVModifi2 of water 0 (reach
//!   0x0043e400  32000); on objLayer water 0 stepped and drawn, then
//!               MakePacketDrawBuffTrans into its texture; the airship's
//!               Move and its ccAnm's Draw; DrawBG; on obj2Layer DrawObj2,
//!               DrawObj, DrawFloor, DrawWithOutFog of rows 29-33 and of the
//!               type-0 rows below 29 (24-28), DrawMap; u$1875 += 0.005
//!               (back to 0 at 1, read by nothing)
//! DrawBG()      v$1474 += (0.001, 0.002), each back to 0 past 1; the four
//!   0x0043cb30  clumps at the origin on bgLayer[0]-[3] (-100 .. -70); in
//!               crisis MAT_sr3dat1_2's U and _3's U and V from vftoi12 of
//!               the second, CMP_sr3dat1_1-3 on bgLayer[4]-[6] (-60, -50,
//!               -45)
//! DrawObj2, DrawObj  0x0043ce60, 0x0043cdc0: the rows of type 3, 2
//! DrawFloor     0x0043cf00: rows 0-7
//! ```
//!
//! Water 1 and 2 are made and stepped once but never drawn. The BLT chunks
//! (`ccBltGrpChunk::LockBlt`, `MakePacketLoadData`) load the textures into
//! the GS, which the port's renderer does not need.
//!
//! The airship (`AIRSHIP`, 0x0043af10, `Move` 0x0043b0c0) flies a round of
//! six legs between the dummies `DMY_marker69`-`72`: 69 to 70, 70 to 71,
//! 71 to 72, then back. A leg takes 2000 frames (its `t` += 0.0005), then
//! the ship turns for 300 frames (`dirc.z` by pi/300 or pi/600 a frame, to
//! the leg's heading), then waits 300 frames. It puffs smoke from its two
//! chimneys while it flies and while it waits, and sounds (SE 257) as each
//! puff comes while it waits. The puffs draw from `fieldrand`.

use std::collections::HashMap;
use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::statics::DrawPass;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::draw::{OBJ_LAYER, OBJ2_LAYER};
use crate::ee::{self, F, ONE, V4};
use crate::pose::Play;
use crate::town::{
    Base, FogParams, RootTown, Spec, Town, TownEvent, TownSprite, TownView, WaterZero, anim_draw_fog, clump_models,
    crisis_rows, pos_rot_zyx, water0_draw,
};

/// `ChangeScene`'s town number.
pub const NO: i32 = 2;
pub const FILE: &str = "town03";
pub const CRISIS_FILE: &str = "town03d";
/// `DrawBG`'s clumps and their layers: the sky's two and the mountains'
/// two, on `bgLayer[0]`-`[3]`.
pub const BACK: [(&str, i16); 4] =
    [("CMP_sr3bac1", -100), ("CMP_sr3bac2", -90), ("CMP_sr3mou1", -80), ("CMP_sr3mou2", -70)];
/// In crisis (`town03d`) the clumps over them, on `bgLayer[4]`-`[6]`, and
/// the materials that scroll (the first in U, the second in U and V).
pub const CRISIS_SKY: [(&str, i16); 3] = [("CMP_sr3dat1_1", -60), ("CMP_sr3dat1_2", -50), ("CMP_sr3dat1_3", -45)];
pub const CRISIS_MATS: [&str; 2] = ["MAT_sr3dat1_2", "MAT_sr3dat1_3"];
/// The light animation and `town03Light` (MUT gcmn 0x00603500).
pub const LIGHT_ANIM: &str = "ANM_sr3bac1a";
pub const LIGHTS: [(i32, &str); 12] = [
    (0, "LGT_sr3lig1"),
    (1, "LGT_omni01"),
    (1, "LGT_omni02"),
    (1, "LGT_omni03"),
    (1, "LGT_omni04"),
    (1, "LGT_omni05"),
    (1, "LGT_omni06"),
    (1, "LGT_omni07"),
    (1, "LGT_omni08"),
    (1, "LGT_omni09"),
    (1, "LGT_omni10"),
    (1, "LGT_omni11"),
];
/// The water, rooted at the origin (`@1139`, `@1140`).
pub const WATER_ANIM: &str = "ANM_sr3wat1a";
pub const WATER_MODEL: &str = "MDL_sr3wat00";
/// `waterUVModifi2`'s reach: 32000.
const WATER_REACH: F = 0x46fa_0000;

/// `SetFog(3000, 15000, 0, 85, 0x0014140c)` and the same colour as
/// `ccSys.bgColor`: a dark teal.
pub const FOG: FogParams =
    FogParams { near: 3000.0, far: 15000.0, near_rate: 0.0, far_rate: 85.0, colour: [0x0c, 0x14, 0x14] };
pub const BG_COLOR: [u8; 3] = [0x0c, 0x14, 0x14];

const SPEC: Spec = Spec {
    no: NO,
    models: "RT_MODELTABLE03",
    objects: None,
    light_anim: LIGHT_ANIM,
    lights: &LIGHTS,
    fog: FOG,
    clear: Some(BG_COLOR),
};

/// `DrawBG`'s scrolls a frame, `v$1474` (MUT gcmn gp 0x0038b118): 0.001 and
/// 0.002.
const BG_STEP: [F; 2] = [0x3a83_126f, 0x3b03_126f];
/// The water's scroll a frame, `u$1875`: 0.005.
const WATER_STEP: F = 0x3ba3_d70a;
const K4096: F = 0x4580_0000;
/// The rows `Draw` draws without fog first (29-33), and below which it
/// draws the type-0 rows after them.
const NO_FOG_ROWS: std::ops::Range<usize> = 29..34;
/// `DrawFloor`'s rows.
const FLOOR_ROWS: std::ops::Range<usize> = 0..8;
/// The area generator's `seed` as the executable holds it: the airship's
/// `fieldrand`.
pub const FIELD_SEED: u32 = 13;

/// The airship's dummies, its clip and its numbers.
pub mod ship {
    use crate::ee::F;

    pub const MARKERS: [&str; 4] = ["DMY_marker69", "DMY_marker70", "DMY_marker71", "DMY_marker72"];
    pub const ANIM: &str = "ANM_sr3shi01_a";
    /// Each leg's (from, to) markers, by `+0x04`.
    pub const LEGS: [(usize, usize); 6] = [(0, 1), (1, 2), (2, 3), (3, 2), (2, 1), (1, 0)];
    /// The model's height below the ship's place: z -500 while it is set.
    pub const DRAW_Z: F = 0xc3fa_0000;
    /// A leg's `t` a frame: 0.0005.
    pub const T_STEP: F = 0x3a03_126f;
    /// The turn's and the wait's frames.
    pub const TURN: i32 = 300;
    pub const WAIT: i32 = 300;
    /// By the leg just begun: the turn a frame (`dirc.z` less it for
    /// legs 0, 4 and 5, plus it for the others) and the heading the turn
    /// ends on. pi/300 (0x3c2b92a6) and -pi/600 (0xbbab92a6).
    pub const TURNS: [(F, bool, F); 6] = [
        (0x3c2b_92a6, false, 0),
        (0xbbab_92a6, true, 0xbfc9_0fdb),
        (0xbbab_92a6, true, 0x4049_0fdb),
        (0xbc2b_92a6, true, 0),
        (0xbbab_92a6, false, 0x3fc9_0fdb),
        (0xbbab_92a6, false, 0x4049_0fdb),
    ];
    /// The chimneys (in the ship's frame, -180 and 180 across, 500 on, 900
    /// up), the puffs' velocity while it flies and while it waits (x -2 and
    /// 2), and their texture and fades.
    pub const CHIMNEYS: [F; 2] = [0xc334_0000, 0x4334_0000];
    pub const CHIMNEY_Y: F = 0x43fa_0000;
    pub const CHIMNEY_Z: F = 0x4461_0000;
    pub const PUFF_X: [F; 2] = [0xc000_0000, 0x4000_0000];
    pub const FLY_V: [F; 2] = [0x4040_0000, 0x40a0_0000];
    pub const WAIT_V: [F; 2] = [0x3f80_0000, 0xc100_0000];
    pub const PUFF_TEX: i32 = 109;
    pub const PUFF_FADE: (i16, i16) = (512, 32);
    /// `ccSeOn3D(257, pos)` with each pair of puffs while it waits.
    pub const SE: i32 = 257;
}

/// The `AIRSHIP` (0xd0 bytes).
#[derive(Clone, Debug)]
pub struct Airship {
    /// +0x04 the leg (0-5), +0x0c the turn's frames left, +0x10 the wait's,
    /// +0xb0 the leg's `t`, +0xb8 the frames to the next puffs.
    pub leg: usize,
    pub turn: i32,
    pub wait: i32,
    pub t: F,
    pub puff: i32,
    /// +0x20 its place (z 0 once it has moved), +0x30 its heading (only z
    /// turns), +0x40 the leg's way (to less the place it began from).
    pub pos: V4,
    pub dirc: V4,
    pub way: V4,
    /// +0x50..+0x80 the four markers, +0x90 the leg's start, +0xa0 its end.
    pub markers: [V4; 4],
    pub from: V4,
    pub to: V4,
    /// +0xbc its ccAnm, and the root `Move` set it at before it moved.
    pub play: Play,
    pub root: [V4; 4],
}

impl Airship {
    /// `AIRSHIP::AIRSHIP(stream)`: at marker 69 bound for 70, the first
    /// puffs next frame.
    pub fn new(file: &SceneFile) -> Result<Airship> {
        let dummy = |name: &str| {
            file.ccs
                .find_object(name)
                .and_then(|o| file.scene.dummies.get(&o))
                .map(|d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE])
                .ok_or_else(|| Error::NotFound(name.into()))
        };
        let markers =
            [dummy(ship::MARKERS[0])?, dummy(ship::MARKERS[1])?, dummy(ship::MARKERS[2])?, dummy(ship::MARKERS[3])?];
        let play = Play::new(file, ship::ANIM).ok_or_else(|| Error::NotFound(ship::ANIM.into()))?;
        let (pos, to) = (markers[0], markers[1]);
        Ok(Airship {
            leg: 0,
            turn: 0,
            wait: 0,
            t: 0,
            puff: 1,
            pos,
            dirc: [0; 4],
            way: sub(to, pos),
            markers,
            from: pos,
            to,
            play,
            root: unit(),
        })
    }

    /// The root `Move` sets the ccAnm at: `SetMatrix_PosRotXYZ` of the place
    /// 500 lower and the heading.
    fn set_root(&self) -> [V4; 4] {
        let mut p = self.pos;
        p[2] = ship::DRAW_Z;
        let r = self.dirc;
        let m = piney_data::anim::rot_z_bits_of(
            piney_data::anim::rot_y_bits_of(piney_data::anim::rot_x_bits_of(unit(), r[0]), r[1]),
            r[2],
        );
        let mut m = m;
        for (k, &v) in p.iter().take(3).enumerate() {
            m[3][k] = ee::add(m[3][k], v);
        }
        m
    }

    /// `AIRSHIP::Move`: the ccAnm set and stepped, then the turn, the wait
    /// or the leg; the puffs and sounds it makes go to `events`.
    pub fn step(&mut self, file: &SceneFile, rng: &mut Rng, events: &mut Vec<TownEvent>) {
        self.root = self.set_root();
        self.play.forward(file);
        self.pos[2] = 0;
        if self.turn != 0 {
            self.turn -= 1;
            if let Some(&(d, add, end)) = ship::TURNS.get(self.leg) {
                self.dirc[2] = if add { ee::add(self.dirc[2], d) } else { ee::sub(self.dirc[2], d) };
                if self.turn == 0 {
                    self.dirc[2] = end;
                }
            }
            if self.turn == 0 {
                self.wait = ship::WAIT;
            }
            return;
        }
        if self.wait != 0 {
            self.wait -= 1;
            if self.puff == 0 {
                self.puffs(ship::WAIT_V, rng, events);
                events.push(TownEvent::Se3d { n: ship::SE, pos: self.pos });
                self.puff = rng.below(8) as i32 + 6;
            } else {
                self.puff -= 1;
            }
            return;
        }
        self.t = ee::add(ship::T_STEP, self.t);
        if !ee::lt(self.t, ONE) {
            self.t = 0;
            self.pos = self.to;
            self.turn = ship::TURN;
            self.leg = (self.leg + 1) % 6;
            let (a, b) = ship::LEGS[self.leg];
            self.from = self.markers[a];
            self.to = self.markers[b];
            self.way = sub(self.to, self.pos);
            return;
        }
        let s = scale(self.way, self.t);
        self.pos = add(self.from, s);
        if self.puff == 0 {
            self.puffs(ship::FLY_V, rng, events);
            self.puff = rng.below(8) as i32 + 6;
        } else {
            self.puff -= 1;
        }
    }

    /// The two chimneys' puffs: each place and velocity turned by the
    /// heading (`sceVu0RotMatrix`), the place from the ship's; the size 3
    /// to 5 and the life 40 to 79 from `fieldrand`.
    fn puffs(&self, v: [F; 2], rng: &mut Rng, events: &mut Vec<TownEvent>) {
        let m = piney_data::anim::rot_bits([self.dirc[0], self.dirc[1], self.dirc[2]]);
        for k in 0..2 {
            let at = apply(&m, [ship::CHIMNEYS[k], ship::CHIMNEY_Y, ship::CHIMNEY_Z, ONE]);
            let vel = apply(&m, [ship::PUFF_X[k], v[0], v[1], ONE]);
            let pos = add(at, self.pos);
            let scale = ee::from_int(rng.below(3) as i32 + 3);
            let life = rng.below(40) as i32 + 40;
            events.push(TownEvent::SmokeN { pos, v: vel, scale, life, kind: ship::PUFF_TEX, fade: ship::PUFF_FADE });
        }
    }
}

fn unit() -> [V4; 4] {
    [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]]
}

/// `sceVu0AddVector` (all four).
fn add(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| ee::add(a[i], b[i]))
}

/// `sceVu0SubVector` (all four).
fn sub(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| ee::sub(a[i], b[i]))
}

/// `sceVu0ScaleVector` (all four).
fn scale(a: V4, t: F) -> V4 {
    a.map(|x| ee::mul(x, t))
}

/// `sceVu0ApplyMatrix`.
fn apply(m: &[V4; 4], v: V4) -> V4 {
    let mut out = [0; 4];
    for (i, o) in out.iter_mut().enumerate() {
        let mut s = ee::mul(m[0][i], v[0]);
        for c in 1..4 {
            s = ee::add(s, ee::mul(m[c][i], v[c]));
        }
        *o = s;
    }
    out
}

/// One piece `ROOTTOWN03::Draw` draws, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// `waterAnm[k]`, stepped and drawn (only 0 is).
    Water(usize),
    /// `ccLayer::MakePacketDrawBuffTrans` on objLayer: the picture so far
    /// copied into water 0's texture.
    Copy,
    /// The airship: its ccAnm drawn where its `Move` set it.
    Ship,
    /// `DrawBG`: background clump `k` at the origin on its own layer.
    Back(usize),
    /// `DrawBG` in crisis: the crisis clump `k` at the origin on its own
    /// layer.
    CrisisSky(usize),
    /// A `STATICMODEL` (its `RT_MODELTABLE` row).
    Model(usize),
    /// `STATICMODEL::DrawWithOutFog` of a row.
    ModelNoFog(usize),
    /// `DrawMap`: the minimap (the host draws it).
    Map,
}

/// What `ROOTTOWN03` holds besides the [`Base`].
pub struct CarminaGade {
    /// `DrawBG`'s clumps' models.
    back: [Vec<u32>; 4],
    crisis_sky: Option<[Vec<u32>; 3]>,
    /// `v$1474`, and the crisis materials' offset `DrawBG` wrote from the
    /// second.
    pub bg_scroll: [F; 2],
    pub bg_uv: u16,
    /// `waterAnm[0..2]` (+0x1bc..+0x1c4); only the first is drawn.
    water: [Play; 3],
    /// `u$1875`.
    pub water_u: F,
    /// Water 0's texture coordinates and texture.
    pub water0: WaterZero,
    /// `town03Ship`.
    pub ship: Airship,
    /// `fieldrand` as the airship draws from it.
    pub rng: Rng,
}

/// `vftoi12(x)` as the 16 bits the materials keep.
fn vftoi12(x: F) -> u16 {
    (ee::to_int(ee::mul(x, K4096)) & 0xffff) as u16
}

/// `ROOTTOWN03::ROOTTOWN03`: `town03d` when the save's crisis byte is set,
/// else `town03`.
pub fn new(archive: &Arc<Archive>, crisis: bool) -> Result<Town> {
    let stem = if crisis { CRISIS_FILE } else { FILE };
    let base = Base::read(archive, stem, &SPEC)?;
    let file = base.file.clone();
    let wplay = Play::new(&file, WATER_ANIM).ok_or_else(|| Error::NotFound(WATER_ANIM.into()))?;
    let mut water = [wplay.clone(), wplay.clone(), wplay];
    for w in &mut water {
        // SetMatrix_PosRotZYX, then one _AnimateForward.
        w.forward(&file);
    }
    let d = CarminaGade {
        back: BACK.map(|(n, _)| clump_models(&file, n)),
        crisis_sky: crisis.then(|| CRISIS_SKY.map(|(n, _)| clump_models(&file, n))),
        bg_scroll: [0; 2],
        bg_uv: 0,
        water,
        water_u: 0,
        water0: WaterZero::new(&file, WATER_MODEL, WATER_REACH)?,
        ship: Airship::new(&file)?,
        rng: Rng::new(FIELD_SEED),
    };
    Ok(Town::new(base, d))
}

/// The water's root: `SetMatrix_PosRotZYX(@1139, @1140)`, the origin.
pub fn water_root() -> [V4; 4] {
    pos_rot_zyx([0, 0, 0, ONE], [0; 3])
}

impl CarminaGade {
    /// `ROOTTOWN03::Draw` for this frame: the pieces in its order, with
    /// what it steps on the way - water 0, the airship, `DrawBG`'s
    /// scrolls, the water's scroll.
    pub fn select(&mut self, base: &mut Base, v: &TownView) -> Vec<Piece> {
        let mut out = Vec::new();
        self.water0.modify(&water_root(), [0, 0, 0, ONE], v.eye, &v.world_screen);
        self.water[0].forward(&base.file);
        out.extend([Piece::Water(0), Piece::Copy]);
        self.ship.step(&base.file, &mut self.rng, &mut base.events);
        out.push(Piece::Ship);
        // DrawBG (0x0043cb30).
        for (x, step) in self.bg_scroll.iter_mut().zip(BG_STEP) {
            *x = ee::add(*x, step);
            if !ee::le(*x, ONE) {
                *x = 0;
            }
        }
        out.extend((0..4).map(Piece::Back));
        if self.crisis_sky.is_some() {
            self.bg_uv = vftoi12(self.bg_scroll[1]);
            out.extend((0..3).map(Piece::CrisisSky));
        }
        // DrawObj2, DrawObj (the rows of type 3, 2), DrawFloor (rows 0-7).
        for pass in [DrawPass::Obj2, DrawPass::Obj] {
            out.extend(base.rows(pass).map(Piece::Model));
        }
        out.extend(FLOOR_ROWS.map(Piece::Model));
        // Without fog: rows 29-33, then the type-0 rows below 29.
        out.extend(NO_FOG_ROWS.map(Piece::ModelNoFog));
        out.extend(base.rows(DrawPass::Other).filter(|&row| row < NO_FOG_ROWS.start).map(Piece::ModelNoFog));
        out.push(Piece::Map);
        self.water_u = ee::add(self.water_u, WATER_STEP);
        if !ee::lt(self.water_u, ONE) {
            self.water_u = 0;
        }
        out
    }

    /// The pieces into the field's layers.
    fn draw_pieces(&self, base: &Base, layers: &mut Layers, to_screen: Mat4, pieces: &[Piece]) {
        let none = HashMap::new();
        let fm = (&*base.file, &base.morphers);
        for &piece in pieces {
            match piece {
                // Water 0: the picture so far (the copy made after it on
                // objLayer, drawn just before it) at waterUVModifi2's UVs.
                Piece::Water(0) => {
                    let root = crate::draw::mat(&water_root());
                    water0_draw(layers, OBJ_LAYER, to_screen, fm, &self.water[0], root, &self.water0);
                }
                // Water 1 and 2 are never drawn.
                Piece::Water(_) | Piece::Copy | Piece::Map => {}
                Piece::Ship => {
                    let root = crate::draw::mat(&self.ship.root);
                    let fog = base.depth_fog();
                    anim_draw_fog(layers, OBJ_LAYER, to_screen, fm, &self.ship.play, root, &[], 1.0, None, None, fog);
                }
                Piece::Back(k) => base.clump_draw(layers, BACK[k].1, to_screen, &self.back[k], Mat4::IDENTITY, &none),
                Piece::CrisisSky(k) => {
                    let Some(models) = &self.crisis_sky else { continue };
                    let rows = crisis_rows(&base.file, &CRISIS_MATS, self.bg_uv);
                    base.clump_draw(layers, CRISIS_SKY[k].1, to_screen, &models[k], Mat4::IDENTITY, &rows);
                }
                Piece::Model(row) => base.row_draw(layers, OBJ2_LAYER, to_screen, row),
                Piece::ModelNoFog(row) => {
                    base.row_draw_with(layers, OBJ2_LAYER, to_screen, row, crate::draw::Fogging::None)
                }
            }
        }
    }
}

impl RootTown for CarminaGade {
    fn draw(&mut self, base: &mut Base, layers: &mut Layers, to_screen: Mat4, v: &TownView) -> Vec<TownSprite> {
        let pieces = self.select(base, v);
        self.draw_pieces(base, layers, to_screen, &pieces);
        Vec::new()
    }

    fn water_time(&self, k: usize) -> u32 {
        self.water[k].time
    }

    fn water0(&self) -> Option<&WaterZero> {
        Some(&self.water0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive() -> Option<Arc<Archive>> {
        let p = std::path::Path::new("../../work/mutation/mutation.iso");
        if !p.exists() {
            eprintln!("skipped: no {}", p.display());
            return None;
        }
        let mut iso = piney_data::iso::Iso::open(p).ok()?;
        Some(Arc::new(Archive::new(iso.read_path("DATA/DATA.BIN").ok()?).ok()?))
    }

    /// The constructor's pieces: 34 model rows, no static objects, the four
    /// background clumps, the lights, the fog and background colour, and
    /// the airship at marker 69 bound for 70.
    #[test]
    fn constructor() {
        let Some(archive) = archive() else { return };
        let t = new(&archive, false).unwrap();
        let b = &t.base;
        assert_eq!(b.no, NO);
        assert_eq!((0..34).filter(|&r| b.model(r).is_some()).count(), 34);
        let d = t.class::<CarminaGade>().unwrap();
        assert!(d.back.iter().all(|c| !c.is_empty()));
        assert!(d.crisis_sky.is_none());
        assert_eq!(b.lights.lights.len(), 12);
        assert_eq!(b.clear, Some(BG_COLOR));
        assert_eq!((d.ship.pos, d.ship.to), (d.ship.markers[0], d.ship.markers[1]));
        assert!(!b.hits.models.is_empty());
    }

    /// Every frame's pieces come in `Draw`'s order, and the airship flies
    /// its first leg: 2000 frames, then the turn.
    #[test]
    fn the_airship_flies_its_first_leg() {
        let Some(archive) = archive() else { return };
        let mut t = new(&archive, false).unwrap();
        let v = TownView::default();
        let (base, d) = t.parts_mut::<CarminaGade>().unwrap();
        let p = d.select(base, &v);
        assert_eq!(
            &p[..7],
            &[
                Piece::Water(0),
                Piece::Copy,
                Piece::Ship,
                Piece::Back(0),
                Piece::Back(1),
                Piece::Back(2),
                Piece::Back(3)
            ]
        );
        assert_eq!(p.last(), Some(&Piece::Map));
        let mut frames = 1;
        while d.ship.turn == 0 {
            d.select(base, &v);
            frames += 1;
            assert!(frames < 2100, "the first leg did not end");
        }
        // t += 0.0005 in single floats: just short of 1 after 2000 steps.
        assert!((2000..2010).contains(&frames), "{frames}");
        assert_eq!((d.ship.leg, d.ship.turn, d.ship.pos), (1, ship::TURN, d.ship.markers[1]));
        assert!(base.take_events().iter().any(|e| matches!(e, TownEvent::SmokeN { .. })));
    }
}
