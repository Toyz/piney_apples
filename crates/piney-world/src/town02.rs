//! Dun Loireag: `ROOTTOWN02` (gcmn town02.cpp, constructor 0x004240c0,
//! `Draw` 0x004266a0 through the vtable at 0x00375ff0), the second Root
//! Town (`game.town` 1, the server Theta), and what it adds to the
//! [`Base`] every town builds.
//!
//! ```text
//! ROOTTOWN02()  mapLayer; fog SetFog(1500, 10000, 0, 75, 0xf0c080) and
//!   0x004240c0  ccSys.bgColor 0xf0c080; town02 (town02d in crisis); 20
//!               STATICMODELs (RT_MODELTABLE02) and 9 STATICOBJECTs
//!               (RT_OBJTABLE02); the clumps CMP_sr2bac1 (+0x244),
//!               CMP_sr2clo_1_1, _1_2 (+0x248, +0x24c), CMP_sr2sun1 (+0x250),
//!               each SetFogSw(0); in crisis CMP_sr2dat1_1-3 (+0x70..+0x78);
//!               the map's sprites (TEX_sr2map1, xallow0); the lights from
//!               ANM_sr2bac1a (town02Light: LGT_sr2lig1, LGT_sr2omn01-05);
//!               town_z (+0x240), its LENSFLARE (+0x23c); 25 CLOUDs
//!               (+0x1d8); three ccAnms of ANM_sr2wat1a (+0x1b0..+0x1b8),
//!               SetFogSw(0), OBJ_sr2wat00 duplicated (8200, 8192, 8192),
//!               rooted at (0, 4200, 0) and stepped once; a 128 x 128
//!               ccTexChunk (+0x1d4) swapped into water 0's model
//! Draw()        the clouds' Init on the first call (+0x04); cameraGetPos;
//!   0x004266a0  SetUV of water 1 (U) and 2 (V); waterUVModifi2 of water 0
//!               (reach 32000); on effLayer water 2, water 1; on objLayer
//!               water 0, then MakePacketDrawBuffTrans into its texture; the
//!               scroll u$1777 += 0.005, back to 0 at 1; DrawBG; on objLayer
//!               DrawObj2, DrawObj, DrawFloor, DrawMap, the type-0 rows; on
//!               effLayer LENSFLARE::Draw(town02, DMY_sr2lig_1point, 0), then
//!               each cloud's Move and Draw
//! DrawBG()      v$1346 += (0.01, 0.02), v2$1347 += 0.001, each back to 0
//!   0x00425130  past 1; MAT_sr2clo_1's and _2's V offsets vftoi12 of the
//!               first two less their crops; the sky on bgLayer[0] (-100),
//!               the sun at DMY_sr2lig_1point on bgLayer[1] (-90); on
//!               effLayer the cloud layers; in crisis MAT_sr2dat1_2's U and
//!               _3's U and V from v2, CMP_sr2dat1_1-3 on bgLayer[2]-[4]
//!               (-80, -70, -60)
//! DrawObj2, DrawObj, DrawFloor  0x004255e0, 0x00425500, 0x004256c0: the
//!               rows of type 3, 2, 1 (and the static objects of types 3
//!               and 2), without the clip rules Mac Anu has
//! ```
//!
//! The scrolls are the game's statics: `u$1777` starts at 0 on the first
//! `Draw` after power-on and `v$1346` in the data, so they carry across
//! visits; here they start at 0 with each town. The clouds draw from the
//! area generator's `fieldrand`, whose state is the game's own global
//! `seed`: whatever the last area left, 13 in the executable. The town
//! starts from 13 unless the host sets [`DunLoireag::rng`].

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::statics::DrawPass;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::cloud::{self, Cloud};
use crate::draw::{EFF_LAYER, OBJ_LAYER};
use crate::ee::{self, F, ONE, V4};
use crate::lensflare;
use crate::pose::Play;
use crate::town::{
    Base, FogParams, RootTown, Spec, Town, TownSprite, TownView, WaterZero, anim_draw_on, clump_models, crisis_rows,
    pos_rot_zyx, water_rows, water0_draw,
};

/// `ChangeScene`'s town number.
pub const NO: i32 = 1;
pub const FILE: &str = "town02";
pub const CRISIS_FILE: &str = "town02d";
/// `town_z`: the lens flare's and the clouds' sprites.
pub const EFF_FILE: &str = "town_z";
/// `DrawBG`'s clumps: the sky, the two cloud layers and the sun.
pub const SKY: &str = "CMP_sr2bac1";
pub const CLOUD_SKY: [&str; 2] = ["CMP_sr2clo_1_1", "CMP_sr2clo_1_2"];
pub const CLOUD_MATS: [&str; 2] = ["MAT_sr2clo_1", "MAT_sr2clo_2"];
pub const SUN: &str = "CMP_sr2sun1";
/// The sun's place, and `LENSFLARE::Draw`'s sun.
pub const SUN_POINT: &str = "DMY_sr2lig_1point";
/// In crisis (`town02d`) the clumps over the sky, on `WORLD_MAN`'s
/// `bgLayer[2]`-`[4]`, and the materials that scroll.
pub const CRISIS_SKY: [(&str, i16); 3] = [("CMP_sr2dat1_1", -80), ("CMP_sr2dat1_2", -70), ("CMP_sr2dat1_3", -60)];
pub const CRISIS_MATS: [&str; 2] = ["MAT_sr2dat1_2", "MAT_sr2dat1_3"];
/// The light animation and `town02Light` (gcmn 0x005d4e20).
pub const LIGHT_ANIM: &str = "ANM_sr2bac1a";
pub const LIGHTS: [(i32, &str); 6] = [
    (0, "LGT_sr2lig1"),
    (1, "LGT_sr2omn01"),
    (1, "LGT_sr2omn02"),
    (1, "LGT_sr2omn03"),
    (1, "LGT_sr2omn04"),
    (1, "LGT_sr2omn05"),
];
/// The water: `@1318`, `@1319`, and its root `@988` (0, 4200, 0) turned
/// by `@989` (none).
pub const WATER_ANIM: &str = "ANM_sr2wat1a";
pub const WATER_OBJ: &str = "OBJ_sr2wat00";
pub const WATER_MODEL: &str = "MDL_sr2wat00";
const WATER_POS: V4 = [0, 0x4583_4000, 0, ONE];
/// `waterUVModifi2`'s reach: 32000 (0x0042682c).
const WATER_REACH: F = 0x46fa_0000;

/// `SetFog(1500, 10000, 0, 75, 0x00f0c080)` (0x00424278) and the same
/// colour as `ccSys.bgColor`: a pale blue.
pub const FOG: FogParams =
    FogParams { near: 1500.0, far: 10000.0, near_rate: 0.0, far_rate: 75.0, colour: [0x80, 0xc0, 0xf0] };
pub const BG_COLOR: [u8; 3] = [0x80, 0xc0, 0xf0];

const SPEC: Spec = Spec {
    no: NO,
    models: "RT_MODELTABLE02",
    objects: Some("RT_OBJTABLE02"),
    light_anim: LIGHT_ANIM,
    lights: &LIGHTS,
    fog: FOG,
    clear: Some(BG_COLOR),
};

/// `DrawBG`'s scrolls a frame: `v$1346` (gcmn gp 0x00378170) 0.01 and
/// 0.02, `v2$1347` (0x00378b5c) 0.001.
const BG_STEP: [F; 2] = [0x3c23_d70a, 0x3ca3_d70a];
const BG_STEP2: F = 0x3a83_126f;
/// The water's scroll a frame, `u$1777` (0x00378b6c): 0.005.
const WATER_STEP: F = 0x3ba3_d70a;
const K4096: F = 0x4580_0000;
/// The `CLOUD`s `Draw` makes on its first call (+0x1d8, type 0).
pub const CLOUDS: usize = 25;
/// The area generator's `seed` (main 0x00377d20) as the executable holds
/// it.
pub const FIELD_SEED: u32 = 13;

/// `WORLD_MAN`'s layers (`GO`, `ccLayer::Init` priorities) the town draws
/// on besides `objLayer`.
pub mod layer {
    /// `bgLayer[0]` (+0x494) and `[1]` (+0x498).
    pub const BG0: i16 = -100;
    pub const BG1: i16 = -90;
    /// `SetActiveLayer(3)`: effLayer (+0x4c8).
    pub const EFF: i16 = crate::draw::EFF_LAYER;
}

/// One piece `ROOTTOWN02::Draw` draws, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// `waterAnm[k]`, stepped and drawn (2 and 1 on effLayer, 0 on
    /// objLayer).
    Water(usize),
    /// `ccLayer::MakePacketDrawBuffTrans` on objLayer: the picture so far
    /// copied into water 0's texture.
    Copy,
    /// `DrawBG`: the sky clump at the origin.
    Sky,
    /// `DrawBG`: the sun's clump at `DMY_sr2lig_1point`.
    Sun,
    /// `DrawBG`: the cloud layer `k` at the origin.
    CloudSky(usize),
    /// `DrawBG` in crisis: the crisis clump `k` at the origin on its own
    /// layer.
    CrisisSky(usize),
    /// A `STATICMODEL` (its `RT_MODELTABLE` row).
    Model(usize),
    /// A `STATICOBJECT` (its `RT_OBJTABLE` row) within its clip: its
    /// animation stepped, then drawn.
    Object(usize),
    /// `DrawMap`: the minimap (the host draws it).
    Map,
    /// `LENSFLARE::Draw`: flare `k` at the place given.
    Flare(usize, V4),
    /// `CLOUD::Draw` of cloud `k` (as its `Move` left it).
    Cloud(usize),
}

/// What `ROOTTOWN02` holds besides the [`Base`].
pub struct DunLoireag {
    /// The clumps' models: the sky, the cloud layers, the sun.
    sky: Vec<u32>,
    cloud_sky: [Vec<u32>; 2],
    sun: Vec<u32>,
    /// `DMY_sr2lig_1point`'s position (w 1).
    pub sun_pos: V4,
    crisis_sky: Option<[Vec<u32>; 3]>,
    /// `v$1346` and `v2$1347`, and what `DrawBG` wrote from them: the
    /// cloud layers' V (`vftoi12` of the first two) and the crisis
    /// materials' U and V (of the third).
    pub bg_scroll: [F; 2],
    pub bg_scroll2: F,
    pub cloud_uv: [u16; 2],
    pub bg_uv: u16,
    /// `waterAnm[0..2]` (+0x1b0..+0x1b8).
    water: [Play; 3],
    /// `u$1777`, and what the last draw passed `ccAnm::SetUV`.
    pub water_u: F,
    pub water_uv: u16,
    /// Water 0's texture coordinates and texture.
    pub water0: WaterZero,
    /// +0x04: set by the first `Draw`, which makes the clouds.
    pub started: bool,
    pub clouds: Vec<Cloud>,
    /// `EFF_srzsmo1`'s `patNum`.
    pat_num: u16,
    /// `fieldrand` as the clouds draw from it.
    pub rng: Rng,
    /// Where `LENSFLARE::Draw` put its flares last (None: not drawn).
    pub flares: Option<[V4; 6]>,
}

/// `vftoi12(x)` as the 16 bits the materials keep.
fn vftoi12(x: F) -> u16 {
    (ee::to_int(ee::mul(x, K4096)) & 0xffff) as u16
}

/// `ROOTTOWN02::ROOTTOWN02`: `town02d` when the save's crisis byte is set,
/// else `town02`, and `town_z`.
pub fn new(archive: &Arc<Archive>, crisis: bool) -> Result<Town> {
    let stem = if crisis { CRISIS_FILE } else { FILE };
    let base = Base::read(archive, stem, &SPEC)?;
    let file = base.file.clone();
    // town_z (+0x240): the lens flare's and the clouds' sprites, which the
    // host's effects draw by name.
    let eff = SceneFile::read(archive, EFF_FILE)?;
    let pat_num = cloud::eff_pat_num(&eff.ccs, cloud::EFF_SMOKE)
        .ok_or_else(|| Error::NotFound(format!("{EFF_FILE}: {}", cloud::EFF_SMOKE)))?;
    for name in lensflare::FLARES {
        eff.ccs.find_object(name).ok_or_else(|| Error::NotFound(format!("{EFF_FILE}: {name}")))?;
    }
    let sun_pos = file
        .ccs
        .find_object(SUN_POINT)
        .and_then(|o| file.scene.dummies.get(&o))
        .map(|d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE])
        .ok_or_else(|| Error::NotFound(SUN_POINT.into()))?;
    let wplay = Play::new(&file, WATER_ANIM).ok_or_else(|| Error::NotFound(WATER_ANIM.into()))?;
    let mut water = [wplay.clone(), wplay.clone(), wplay];
    for w in &mut water {
        // SetMatrix_PosRotZYX, then one _AnimateForward.
        w.forward(&file);
    }
    let d = DunLoireag {
        sky: clump_models(&file, SKY),
        cloud_sky: CLOUD_SKY.map(|n| clump_models(&file, n)),
        sun: clump_models(&file, SUN),
        sun_pos,
        crisis_sky: crisis.then(|| CRISIS_SKY.map(|(n, _)| clump_models(&file, n))),
        bg_scroll: [0; 2],
        bg_scroll2: 0,
        cloud_uv: [0; 2],
        bg_uv: 0,
        water,
        water_u: 0,
        water_uv: 0,
        water0: WaterZero::new(&file, WATER_MODEL, WATER_REACH)?,
        started: false,
        clouds: Vec::new(),
        pat_num,
        rng: Rng::new(FIELD_SEED),
        flares: None,
    };
    Ok(Town::new(base, d))
}

/// The water's root: `SetMatrix_PosRotZYX(@988, @989)`.
pub fn water_root() -> [V4; 4] {
    pos_rot_zyx(WATER_POS, [0; 3])
}

impl DunLoireag {
    /// `ROOTTOWN02::Draw` for this frame: the pieces in its order, with
    /// what it steps on the way - the clouds made on the first call, the
    /// water's animations and scroll, `DrawBG`'s scrolls, the static
    /// objects' animations, the clouds' `Move`.
    pub fn select(&mut self, base: &mut Base, v: &TownView) -> Vec<Piece> {
        let mut out = Vec::new();
        if !self.started {
            self.started = true;
            for _ in 0..CLOUDS {
                let c = Cloud::init(0, self.pat_num, v.player, &mut self.rng);
                self.clouds.push(c);
            }
        }
        // ccAnm::SetUV of vftoi12(u) into water 1's U and water 2's V;
        // waterUVModifi2 of water 0 (its UVs, for the draw); 2 and 1 on
        // effLayer, 0 on objLayer and the copy after it.
        self.water_uv = vftoi12(self.water_u);
        self.water0.modify(&water_root(), [0, 0, 0, ONE], v.eye, &v.world_screen);
        for k in [2usize, 1, 0] {
            self.water[k].forward(&base.file);
            out.push(Piece::Water(k));
        }
        out.push(Piece::Copy);
        self.water_u = ee::add(self.water_u, WATER_STEP);
        if !ee::lt(self.water_u, ONE) {
            self.water_u = 0;
        }
        // DrawBG (0x00425130).
        for (x, step) in self.bg_scroll.iter_mut().zip(BG_STEP) {
            *x = ee::add(*x, step);
        }
        self.bg_scroll2 = ee::add(self.bg_scroll2, BG_STEP2);
        for x in self.bg_scroll.iter_mut().chain(std::iter::once(&mut self.bg_scroll2)) {
            if !ee::le(*x, ONE) {
                *x = 0;
            }
        }
        self.cloud_uv = self.bg_scroll.map(vftoi12);
        out.extend([Piece::Sky, Piece::Sun, Piece::CloudSky(0), Piece::CloudSky(1)]);
        if self.crisis_sky.is_some() {
            self.bg_uv = vftoi12(self.bg_scroll2);
            out.extend((0..3).map(Piece::CrisisSky));
        }
        // DrawObj2, DrawObj, DrawFloor: each walks the 20 rows for its
        // type, the first two then the 9 static objects.
        for (pass, objects) in [(DrawPass::Obj2, true), (DrawPass::Obj, true), (DrawPass::Floor, false)] {
            out.extend(base.rows(pass).map(Piece::Model));
            if objects {
                out.extend(base.step_objects(pass, v.eye).into_iter().map(Piece::Object));
            }
        }
        out.push(Piece::Map);
        out.extend(base.rows(DrawPass::Other).map(Piece::Model));
        // LENSFLARE::Draw(town02, DMY_sr2lig_1point, 0).
        let sun_in_view = crate::rtownpc::check_camera_deg(self.sun_pos, lensflare::VIEW_DEG, &v.cam, v.player);
        self.flares = lensflare::draw(self.sun_pos, &v.flare, v.puppet_show, sun_in_view);
        if let Some(f) = self.flares {
            out.extend(f.into_iter().enumerate().map(|(k, p)| Piece::Flare(k, p)));
        }
        // The clouds: Move, then Draw.
        for k in 0..self.clouds.len() {
            self.clouds[k].step(v.player, &mut self.rng, &|_, _| 0);
            let c = &self.clouds[k];
            let in_view = crate::rtownpc::check_camera_deg(c.pos, cloud::VIEW_DEG, &v.cam, v.player);
            if c.drawn(v.player, in_view) {
                out.push(Piece::Cloud(k));
            }
        }
        out
    }

    /// The pieces into the field's layers; the sprites (the lens flare,
    /// the clouds) are handed back.
    fn draw_pieces(&self, base: &Base, layers: &mut Layers, to_screen: Mat4, pieces: &[Piece]) -> Vec<TownSprite> {
        let none = HashMap::new();
        let fm = (&*base.file, &base.morphers);
        let root = crate::draw::mat(&water_root());
        let mut sprites = Vec::new();
        for &piece in pieces {
            match piece {
                // Water 0: the picture so far (the copy made after it on
                // objLayer, drawn just before it) at waterUVModifi2's UVs.
                Piece::Water(0) => water0_draw(layers, OBJ_LAYER, to_screen, fm, &self.water[0], root, &self.water0),
                Piece::Water(k) => {
                    let rows = water_rows(&base.file, k, self.water_uv);
                    anim_draw_on(layers, layer::EFF, to_screen, fm, &self.water[k], root, &[], 1.0, Some(&rows), None);
                }
                Piece::Copy | Piece::Map => {}
                Piece::Sky => base.clump_draw(layers, layer::BG0, to_screen, &self.sky, Mat4::IDENTITY, &none),
                Piece::Sun => {
                    let at = Vec3::new(ee::f(self.sun_pos[0]), ee::f(self.sun_pos[1]), ee::f(self.sun_pos[2]));
                    base.clump_draw(layers, layer::BG1, to_screen, &self.sun, Mat4::from_translation(at), &none);
                }
                Piece::CloudSky(k) => {
                    let rows = cloud_rows(&base.file, self.cloud_uv);
                    base.clump_draw(layers, layer::EFF, to_screen, &self.cloud_sky[k], Mat4::IDENTITY, &rows);
                }
                Piece::CrisisSky(k) => {
                    let Some(models) = &self.crisis_sky else { continue };
                    let rows = crisis_rows(&base.file, &CRISIS_MATS, self.bg_uv);
                    base.clump_draw(layers, CRISIS_SKY[k].1, to_screen, &models[k], Mat4::IDENTITY, &rows);
                }
                Piece::Model(row) => base.row_draw(layers, OBJ_LAYER, to_screen, row),
                Piece::Object(row) => base.object_draw(layers, OBJ_LAYER, to_screen, row),
                Piece::Flare(k, pos) => sprites.push(TownSprite {
                    file: EFF_FILE,
                    eff: lensflare::FLARES[k],
                    pos,
                    pattern: 0,
                    layer: EFF_LAYER,
                    scale: [ONE; 2],
                    rotate: 0,
                    transparency: ONE,
                    depth: false,
                }),
                Piece::Cloud(k) => {
                    let c = &self.clouds[k];
                    sprites.push(TownSprite {
                        file: EFF_FILE,
                        eff: cloud::EFF_SMOKE,
                        pos: c.pos,
                        pattern: c.pattern as u16,
                        layer: EFF_LAYER,
                        scale: [c.eff.scale_x, c.eff.scale_y],
                        rotate: c.eff.rotate,
                        transparency: c.eff.transparency,
                        depth: false,
                    });
                }
            }
        }
        sprites
    }
}

impl RootTown for DunLoireag {
    fn draw(&mut self, base: &mut Base, layers: &mut Layers, to_screen: Mat4, v: &TownView) -> Vec<TownSprite> {
        let pieces = self.select(base, v);
        self.draw_pieces(base, layers, to_screen, &pieces)
    }

    fn water_time(&self, k: usize) -> u32 {
        self.water[k].time
    }

    fn water0(&self) -> Option<&WaterZero> {
        Some(&self.water0)
    }
}

/// The materials' offsets `DrawBG` wrote into the cloud layers: V only,
/// the scroll less each material's crop (STROW values).
fn cloud_rows(file: &SceneFile, uv: [u16; 2]) -> HashMap<u32, [u8; 2]> {
    let mut rows = HashMap::new();
    for (name, v) in CLOUD_MATS.iter().zip(uv) {
        let Some(mat) = file.ccs.find_object(name) else { continue };
        let Some(m) = file.scene.materials.get(&mat) else { continue };
        rows.insert(mat, [0, (v.wrapping_sub(m.crop_v) >> 4) as u8]);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The constructor's pieces: 20 model rows (the table's all present),
    /// 9 static objects, the sky, the sun at its dummy, the lights, and the
    /// fog and background colour.
    #[test]
    fn constructor() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let t = new(&archive, false).unwrap();
        let b = &t.base;
        assert_eq!(b.no, NO);
        assert_eq!((0..20).filter(|&r| b.model(r).is_some()).count(), 20);
        assert_eq!((0..9).filter(|&r| b.object(r).is_some()).count(), 9);
        let d = t.class::<DunLoireag>().unwrap();
        assert!(!d.sky.is_empty() && !d.sun.is_empty() && d.cloud_sky.iter().all(|c| !c.is_empty()));
        assert!(d.crisis_sky.is_none());
        assert_eq!(b.lights.lights.len(), 6);
        assert_eq!(b.clear, Some(BG_COLOR));
        assert!(d.pat_num > 0);
        assert!(!b.hits.models.is_empty());
    }

    /// The first `Draw` makes the 25 clouds about the player, and every
    /// frame's pieces come in `Draw`'s order: the water, the copy, `DrawBG`,
    /// the rows and objects by pass, the map.
    #[test]
    fn first_draw() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut t = new(&archive, false).unwrap();
        let player = [0, ee::k(3500.0), 0, ONE];
        let v = TownView { eye: [0, ee::k(2600.0), ee::k(400.0), ONE], player, ..TownView::default() };
        let (base, d) = t.parts_mut::<DunLoireag>().unwrap();
        let p = d.select(base, &v);
        assert_eq!(
            &p[..8],
            &[
                Piece::Water(2),
                Piece::Water(1),
                Piece::Water(0),
                Piece::Copy,
                Piece::Sky,
                Piece::Sun,
                Piece::CloudSky(0),
                Piece::CloudSky(1)
            ]
        );
        assert_eq!(d.clouds.len(), CLOUDS);
        assert!(p.contains(&Piece::Map));
        assert_eq!(d.water_uv, 0);
        assert_eq!(d.water_u, WATER_STEP);
    }
}
