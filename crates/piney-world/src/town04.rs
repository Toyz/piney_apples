//! Fort Ouph: `ROOTTOWN04` (OUT gcmn town04.cpp, constructor 0x00439c50,
//! `Draw` 0x0043c430 through the vtable at 0x003842d0), the fourth Root Town
//! (`game.town` 3, the server Sigma), new in Outbreak (Quarantine's is the
//! same). Dun Loireag's clouds and lens flare, but no water, no type-0 rows,
//! and cloud layers on `bgLayer[5]` scrolled in U (docs/engine/root-towns.md).

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
use crate::town::{Base, FogParams, RootTown, Spec, Town, TownSprite, TownView, clump_models, crisis_rows};

/// `ChangeScene`'s town number.
pub const NO: i32 = 3;
pub const FILE: &str = "town04";
pub const CRISIS_FILE: &str = "town04d";
/// `town_z`: the lens flare's and the clouds' sprites.
pub const EFF_FILE: &str = "town_z";
/// `DrawBG`'s clumps: the sky, the sun, the three cloud layers.
pub const SKY: &str = "CMP_sr4bac1";
pub const SUN: &str = "CMP_sr4sun1";
pub const CLOUD_SKY: [&str; 3] = ["CMP_sr4clo1_1", "CMP_sr4clo1_2", "CMP_sr4clo2"];
/// The cloud layers' materials, scrolled in U.
pub const CLOUD_MATS: [&str; 2] = ["MAT_sr4clo1", "MAT_sr4clo2"];
/// The sun's place, and `LENSFLARE::Draw`'s sun.
pub const SUN_POINT: &str = "DMY_sr4lig1point";
/// In crisis (`town04d`) the clumps over the sky, on `bgLayer[2]`-`[4]`,
/// and the materials that scroll.
pub const CRISIS_SKY: [(&str, i16); 3] = [("CMP_sr4dat1_1", -80), ("CMP_sr4dat1_2", -70), ("CMP_sr4dat1_3", -60)];
pub const CRISIS_MATS: [&str; 2] = ["MAT_sr4dat1_2", "MAT_sr4dat1_3"];
/// The light animation and `town04Light` (OUT gcmn 0x00601210).
pub const LIGHT_ANIM: &str = "ANM_sr4bac1a";
pub const LIGHTS: [(i32, &str); 6] = [
    (0, "LGT_sr4lig1"),
    (1, "LGT_sr4omn1"),
    (1, "LGT_sr4omn2"),
    (1, "LGT_sr4omn3"),
    (1, "LGT_sr4omn4"),
    (1, "LGT_sr4omn5"),
];

/// `SetFog(1500, 10000, 0, 30, 0x00c8f0fb)` (0x00439ed8) and the same
/// colour as `ccSys.bgColor`: a pale warm haze.
pub const FOG: FogParams =
    FogParams { near: 1500.0, far: 10000.0, near_rate: 0.0, far_rate: 30.0, colour: [0xfb, 0xf0, 0xc8] };
pub const BG_COLOR: [u8; 3] = [0xfb, 0xf0, 0xc8];

const SPEC: Spec = Spec {
    no: NO,
    models: "RT_MODELTABLE04",
    objects: Some("RT_OBJTABLE04"),
    light_anim: LIGHT_ANIM,
    lights: &LIGHTS,
    fog: FOG,
    clear: Some(BG_COLOR),
};

/// `DrawBG`'s scrolls a frame (OUT gp 0x00386ed0, 0x00386ed4): 0.01 for
/// the clouds, 0.002 for the crisis sky.
const BG_STEP: [F; 2] = [0x3c23_d70a, 0x3b03_126f];
const K4096: F = 0x4580_0000;
/// The `CLOUD`s `Draw` makes on its first call (+0x1c4, type 0).
pub const CLOUDS: usize = 25;
/// The area generator's `seed` as the executable holds it.
pub const FIELD_SEED: u32 = 13;

/// `WORLD_MAN`'s layers the town draws on besides `objLayer`.
pub mod layer {
    /// `bgLayer[0]` (+0x494), `[1]` (+0x498) and `[5]` (+0x4a8).
    pub const BG0: i16 = -100;
    pub const BG1: i16 = -90;
    pub const BG5: i16 = -50;
    /// `SetActiveLayer(3)`: effLayer.
    pub const EFF: i16 = crate::draw::EFF_LAYER;
}

/// One piece `ROOTTOWN04::Draw` draws, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// `DrawBG`: the sky clump at the origin.
    Sky,
    /// `DrawBG`: the sun's clump at `DMY_sr4lig1point`.
    Sun,
    /// `DrawBG` in crisis: the crisis clump `k` at the origin on its own
    /// layer.
    CrisisSky(usize),
    /// `DrawBG`: the cloud layer `k` at the origin.
    CloudSky(usize),
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

/// What `ROOTTOWN04` holds besides the [`Base`].
pub struct FortOuph {
    /// The clumps' models: the sky, the sun, the cloud layers.
    sky: Vec<u32>,
    sun: Vec<u32>,
    cloud_sky: [Vec<u32>; 3],
    /// `DMY_sr4lig1point`'s position (w 1).
    pub sun_pos: V4,
    crisis_sky: Option<[Vec<u32>; 3]>,
    /// The two scrolls, and what `DrawBG` wrote from them: the cloud
    /// materials' U (`vftoi12` of the first) and the crisis materials' U
    /// and V (of the second).
    pub bg_scroll: [F; 2],
    pub cloud_u: u16,
    pub bg_uv: u16,
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

/// `ROOTTOWN04::ROOTTOWN04`: `town04d` when the save's crisis byte is set,
/// else `town04`, and `town_z`.
pub fn new(archive: &Arc<Archive>, crisis: bool) -> Result<Town> {
    let stem = if crisis { CRISIS_FILE } else { FILE };
    let base = Base::read(archive, stem, &SPEC)?;
    let file = base.file.clone();
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
    let d = FortOuph {
        sky: clump_models(&file, SKY),
        sun: clump_models(&file, SUN),
        cloud_sky: CLOUD_SKY.map(|n| clump_models(&file, n)),
        sun_pos,
        crisis_sky: crisis.then(|| CRISIS_SKY.map(|(n, _)| clump_models(&file, n))),
        bg_scroll: [0; 2],
        cloud_u: 0,
        bg_uv: 0,
        started: false,
        clouds: Vec::new(),
        pat_num,
        rng: Rng::new(FIELD_SEED),
        flares: None,
    };
    Ok(Town::new(base, d))
}

impl FortOuph {
    /// `ROOTTOWN04::Draw` for this frame: the pieces in its order, with
    /// what it steps on the way - the clouds made on the first call,
    /// `DrawBG`'s scrolls, the static object's animation, the clouds'
    /// `Move`.
    pub fn select(&mut self, base: &mut Base, v: &TownView) -> Vec<Piece> {
        let mut out = Vec::new();
        if !self.started {
            self.started = true;
            for _ in 0..CLOUDS {
                let c = Cloud::init(0, self.pat_num, v.player, &mut self.rng);
                self.clouds.push(c);
            }
        }
        // DrawBG (0x0043aa40).
        for (x, step) in self.bg_scroll.iter_mut().zip(BG_STEP) {
            *x = ee::add(*x, step);
        }
        for x in &mut self.bg_scroll {
            if !ee::le(*x, ONE) {
                *x = 0;
            }
        }
        self.cloud_u = vftoi12(self.bg_scroll[0]);
        out.extend([Piece::Sky, Piece::Sun]);
        if self.crisis_sky.is_some() {
            self.bg_uv = vftoi12(self.bg_scroll[1]);
            out.extend((0..3).map(Piece::CrisisSky));
        }
        out.extend((0..3).map(Piece::CloudSky));
        // DrawObj2, DrawObj, DrawFloor: each walks the 16 rows for its
        // type, the first two then the static object.
        for (pass, objects) in [(DrawPass::Obj2, true), (DrawPass::Obj, true), (DrawPass::Floor, false)] {
            out.extend(base.rows(pass).map(Piece::Model));
            if objects {
                out.extend(base.step_objects(pass, v.eye).into_iter().map(Piece::Object));
            }
        }
        out.push(Piece::Map);
        // LENSFLARE::Draw(town04, DMY_sr4lig1point, 0).
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
        let mut sprites = Vec::new();
        for &piece in pieces {
            match piece {
                Piece::Sky => base.clump_draw(layers, layer::BG0, to_screen, &self.sky, Mat4::IDENTITY, &none),
                Piece::Sun => {
                    let at = Vec3::new(ee::f(self.sun_pos[0]), ee::f(self.sun_pos[1]), ee::f(self.sun_pos[2]));
                    base.clump_draw(layers, layer::BG1, to_screen, &self.sun, Mat4::from_translation(at), &none);
                }
                Piece::CrisisSky(k) => {
                    let Some(models) = &self.crisis_sky else { continue };
                    let rows = crisis_rows(&base.file, &CRISIS_MATS, self.bg_uv);
                    base.clump_draw(layers, CRISIS_SKY[k].1, to_screen, &models[k], Mat4::IDENTITY, &rows);
                }
                Piece::CloudSky(k) => {
                    let rows = cloud_rows(&base.file, self.cloud_u);
                    base.clump_draw(layers, layer::BG5, to_screen, &self.cloud_sky[k], Mat4::IDENTITY, &rows);
                }
                Piece::Model(row) => base.row_draw(layers, OBJ_LAYER, to_screen, row),
                Piece::Object(row) => base.object_draw(layers, OBJ_LAYER, to_screen, row),
                Piece::Map => {}
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

impl RootTown for FortOuph {
    fn draw(&mut self, base: &mut Base, layers: &mut Layers, to_screen: Mat4, v: &TownView) -> Vec<TownSprite> {
        let pieces = self.select(base, v);
        self.draw_pieces(base, layers, to_screen, &pieces)
    }
}

/// The materials' offsets `DrawBG` wrote into the cloud layers: U only,
/// the scroll less each material's crop (STROW values).
fn cloud_rows(file: &SceneFile, u: u16) -> HashMap<u32, [u8; 2]> {
    let mut rows = HashMap::new();
    for name in CLOUD_MATS {
        let Some(mat) = file.ccs.find_object(name) else { continue };
        let Some(m) = file.scene.materials.get(&mat) else { continue };
        rows.insert(mat, [(u.wrapping_sub(m.crop_u) >> 4) as u8, 0]);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The constructor's pieces: 16 model rows (the table's all present),
    /// the static object, the sky, the sun at its dummy, the cloud layers,
    /// the six lights, and the fog and background colour.
    #[test]
    fn constructor() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let t = new(&archive, false).unwrap();
        let b = &t.base;
        assert_eq!(b.no, NO);
        assert_eq!((0..16).filter(|&r| b.model(r).is_some()).count(), 16);
        assert_eq!((0..1).filter(|&r| b.object(r).is_some()).count(), 1);
        let d = t.class::<FortOuph>().unwrap();
        assert!(!d.sky.is_empty() && !d.sun.is_empty() && d.cloud_sky.iter().all(|c| !c.is_empty()));
        assert!(d.crisis_sky.is_none());
        assert_eq!(b.lights.lights.len(), 6);
        assert_eq!(b.clear, Some(BG_COLOR));
        assert!(d.pat_num > 0);
        assert!(!b.hits.models.is_empty());
        let c = new(&archive, true).unwrap();
        assert!(c.class::<FortOuph>().unwrap().crisis_sky.as_ref().is_some_and(|s| s.iter().all(|m| !m.is_empty())));
    }

    /// The first `Draw` makes the 25 clouds, and the pieces come in
    /// `Draw`'s order: `DrawBG` (the sky, the sun, the three cloud layers),
    /// the rows and the object by pass, the map; the scrolls step.
    #[test]
    fn first_draw() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut t = new(&archive, false).unwrap();
        let player = [0, ee::k(6000.0), 0, ONE];
        let v = TownView { eye: [0, ee::k(5000.0), ee::k(400.0), ONE], player, ..TownView::default() };
        let (base, d) = t.parts_mut::<FortOuph>().unwrap();
        let p = d.select(base, &v);
        assert_eq!(&p[..5], &[Piece::Sky, Piece::Sun, Piece::CloudSky(0), Piece::CloudSky(1), Piece::CloudSky(2)]);
        assert!(p.contains(&Piece::Map));
        assert_eq!(d.clouds.len(), CLOUDS);
        assert_eq!(d.bg_scroll, BG_STEP);
        assert_eq!(d.cloud_u, vftoi12(BG_STEP[0]));
    }
}
