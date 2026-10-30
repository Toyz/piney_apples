//! The sky maps `WORLD_MAN::GO(1)` makes by `eventAreaNumber` on every
//! volume: `EVENTAREA04` (field 66, Bitter Fantasy Mirror World),
//! `EVENTAREA05` (field 67, its arena) and `EVENTAREA06` (field 91,
//! Pulsating Sea of Cloud Whale); MUT gcmn 0x00417050, 0x00417e10,
//! 0x00419e30 (docs/engine/evarea.md). Each is its tables' static models and
//! objects (all type 0), sky clumps, one scrolling material, 25 `CLOUD`s
//! made at the first `Draw`, and for 04 and 05 a `LENSFLARE`.

use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::statics::{DrawPass, Position};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::MOVE_TEX_ROWS;
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::area::AreaCore;
use crate::draw::{self, Draw};
use crate::ee::{F, ONE, V4};
use crate::hit::HitModel;
use crate::sky::cloud::{self, Cloud};
use crate::sky::lensflare::{self, FLARES};
use crate::story_map::holy_ground::{MoveTex, StorySprite, layer};
use crate::story_map::{Enter, StartSpread, StoryMap};
use crate::story_map::{Made, MapDef};
use crate::story_map::{event_hits, model_hits};
use crate::town::{Base, FogParams, Spec, TownView};

/// `effccs` (+0x254, 06's +0x268): the clouds' and the flares' file.
pub const EFF_FILE: &str = "town_z";
/// `CLOUD`s (+0x1f0, 06's +0x204).
pub const CLOUDS: usize = 25;
/// `LENSFLARE::Draw(ccs, "DMY_dummy5", 0)`'s sun (04 and 05).
const SUN: &str = "DMY_dummy5";

/// The class `GO(1)` makes, by `eventAreaNumber`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// `EVENTAREA04`, field 66: the oasis whose door is the dungeon.
    Mirror,
    /// `EVENTAREA05`, field 67: the arena that dungeon's last door leads to.
    Arena,
    /// `EVENTAREA06`, field 91.
    Whale,
}

impl Class {
    /// The class of field `field`, if it is one of these.
    pub fn of(field: i32) -> Option<Class> {
        match field {
            66 => Some(Class::Mirror),
            67 => Some(Class::Arena),
            91 => Some(Class::Whale),
            _ => None,
        }
    }
}

/// `SetFog(9000, 14000, 0, 70, 0xc8b9bd)` and `ccSys.bgColor` (04, 05).
const SAND: [u8; 3] = [0xbd, 0xb9, 0xc8];
const SAND_FOG: FogParams = FogParams { near: 9000.0, far: 14000.0, near_rate: 0.0, far_rate: 70.0, colour: SAND };
/// `SetFog(15000, 35000, 0, 80, 0xa77a6d)` and `ccSys.bgColor` (06).
const SEA: [u8; 3] = [0x6d, 0x7a, 0xa7];
const SEA_FOG: FogParams = FogParams { near: 15000.0, far: 35000.0, near_rate: 0.0, far_rate: 80.0, colour: SEA };

/// What a class's constructor and `Draw` name: its file and tables, the
/// `EA_BGNAME` clumps with their `SetFogSw` (the first `drawn` of them
/// drawn, `bg[k]` on `bgLayer[k]`), `EA_moveTex`'s row, the layer the
/// models go on, the clouds' type and whether a flare is made. The tables'
/// rows are the same on all four volumes.
struct Kind {
    file: &'static str,
    spec: Spec,
    bg: &'static [(&'static str, bool)],
    drawn: usize,
    move_tex: MoveTex,
    model_layer: i16,
    cloud_type: i32,
    flare: bool,
}

/// `eventarea04Light`; `eventarea05Light`'s first row, the one row its
/// loop reads (`s0 == 0` once).
const SE2_6_LIGHTS: [(i32, &str); 1] = [(0, "LGT_se2_6lig1")];
const SE3_5_LIGHTS: [(i32, &str); 1] = [(0, "LGT_se3_5lig1")];

const MIRROR: Kind = Kind {
    file: "se2_6",
    spec: Spec {
        no: -1,
        models: "EA_MODELTABLE04",
        objects: Some("EA_OBJTABLE04"),
        light_anim: "ANM_se2_6a",
        lights: &SE2_6_LIGHTS,
        fog: SAND_FOG,
        clear: Some(SAND),
    },
    // bg[2..4] have their fog switched back on.
    bg: &[
        ("CMP_se2_6bg2", false),
        ("CMP_se2_6bg1", false),
        ("CMP_se2_6cl3", true),
        ("CMP_se2_6cl2", true),
        ("CMP_se2_6cl1", true),
        ("CMP_se2_6su1", false),
    ],
    drawn: 6,
    move_tex: MoveTex { u: 0, v: 0, u_step: 0, v_step: 0x3ba3_d70a, mat: "MAT_se1_7clo1" },
    model_layer: layer::OBJ,
    cloud_type: 0,
    flare: true,
};

const ARENA: Kind = Kind {
    file: "se2_6",
    spec: Spec {
        no: -1,
        models: "EA_MODELTABLE05",
        objects: Some("EA_OBJTABLE05"),
        light_anim: "ANM_se2_6a",
        lights: &SE2_6_LIGHTS,
        fog: SAND_FOG,
        clear: Some(SAND),
    },
    bg: &[
        ("CMP_se2_6bg1", false),
        ("CMP_se2_6bg2", false),
        ("CMP_se2_6bg05", false),
        ("CMP_se2_6cl1", false),
        ("CMP_se2_6cl2", false),
        ("CMP_se2_6cl3", false),
    ],
    drawn: 5,
    move_tex: MoveTex { u: 0, v: 0, u_step: 0, v_step: 0x3c23_d70a, mat: "MAT_se1_7clo1" },
    model_layer: draw::FLOOR_LAYER,
    cloud_type: 0,
    flare: true,
};

const WHALE: Kind = Kind {
    file: "se3_5_1",
    spec: Spec {
        no: -1,
        models: "EA_MODELTABLE06",
        objects: Some("EA_OBJTABLE06"),
        light_anim: "ANM_se3_5_1a",
        lights: &SE3_5_LIGHTS,
        fog: SEA_FOG,
        clear: Some(SEA),
    },
    bg: &[
        ("CMP_se3_5bac1", false),
        ("CMP_se3_5clo1_1", false),
        ("CMP_se3_5clo1_2", false),
        ("CMP_se3_5clo2", false),
        ("CMP_se3_5su1", false),
    ],
    drawn: 5,
    move_tex: MoveTex { u: 0, v: 0, u_step: 0xbcf5_c28f, v_step: 0x3b23_d70a, mat: "MAT_se3_5clo2" },
    model_layer: draw::FLOOR_LAYER,
    cloud_type: 3,
    flare: false,
};

impl Class {
    fn kind(self) -> &'static Kind {
        match self {
            Class::Mirror => &MIRROR,
            Class::Arena => &ARENA,
            Class::Whale => &WHALE,
        }
    }
}

/// `bgLayer[k]`'s priority: -100, -90, .. -50.
fn bg_layer(k: usize) -> i16 {
    -100 + 10 * k as i16
}

/// One piece the class's `Draw` draws, in its order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Piece {
    /// `DrawBG`: `bg[k]` at the origin on `bgLayer[k]`.
    Bg(usize),
    /// `STATICMODEL::Draw` of a type-0 row.
    Model(usize),
    /// `STATICOBJECT::Draw` of a type-0 row within its clip, stepped.
    Object(usize),
    /// `LENSFLARE::Draw`'s flare `k`.
    Flare(usize, V4),
    /// `CLOUD::Draw` of cloud `k`, as its `Move` left it.
    Cloud(usize),
}

/// An `EVENTAREA04`, `05` or `06` as `WORLD_MAN.eventmap` holds it.
pub struct SkyMap {
    pub class: Class,
    pub base: Base,
    /// `bg[k]` (+0x1a8): each clump's models, None where the file lacks it.
    pub bg: Vec<Option<(u32, Vec<u32>)>>,
    pub move_tex: MoveTex,
    /// +0x1e4: set by the first `Draw`, which `Init`s the clouds.
    pub started: bool,
    pub clouds: Vec<Cloud>,
    pat_num: u16,
    /// The flare's sun (`DMY_dummy5`, w 1), and where the flares were
    /// drawn last.
    pub sun: Option<V4>,
    pub flares: Option<[V4; 6]>,
}

/// A dummy's position and its turn about z (`Decode_DummyPosRot`'s
/// +0x28), as the constructors copy it into +0x1c.
fn marker(file: &SceneFile, name: &'static str) -> Result<(V4, F)> {
    if file.ccs.find_object(name).is_none() {
        return Err(Error::NotFound(format!("{}: {name}", file.stem)));
    }
    let (pos, rot) = crate::town::dummy_bits(&Position::DummyRot(name), &file.ccs, &file.scene);
    Ok((pos, rot[2]))
}

/// The constructor's `SetStartPos` (`game.areaPrev` being `area_prev`).
/// 04 writes the whole of the dummy's +0x10: from a dungeon (2)
/// `DMY_dummy2` at (-8000, 5800, 920) facing 0, else `DMY_dummy1` at
/// (4600, -4700, 910) facing pi. 05 keeps `DMY_dummy3`'s x, puts it at y
/// 9000 and z 800, facing its turn. 06 stands at `DMY_marker02` from a
/// dungeon, else `DMY_marker01`, facing its turn.
fn start(class: Class, file: &SceneFile, area_prev: i32) -> Result<V4> {
    Ok(match (class, area_prev) {
        (Class::Mirror, 2) => {
            marker(file, "DMY_dummy2")?;
            [0xc5fa_0000, 0x45b5_4000, 0x4466_0000, 0]
        }
        (Class::Mirror, _) => {
            marker(file, "DMY_dummy1")?;
            [0x458f_c000, 0xc592_e000, 0x4463_8000, 0x4049_0fdb]
        }
        (Class::Arena, _) => {
            let (p, w) = marker(file, "DMY_dummy3")?;
            [p[0], 0x460c_a000, 0x4448_0000, w]
        }
        (Class::Whale, a) => {
            let (p, w) = marker(file, if a == 2 { "DMY_marker02" } else { "DMY_marker01" })?;
            [p[0], p[1], p[2], w]
        }
    })
}

impl SkyMap {
    /// The class's constructor; `game.areaPrev` is `area_prev`, `rng` the
    /// game's `fieldrand`.
    pub fn new(
        archive: &Arc<Archive>,
        volume: Volume,
        class: Class,
        def_se: u32,
        area_prev: i32,
        rng: Rng,
    ) -> Result<SkyMap> {
        let k = class.kind();
        let mut base = Base::read_volume(archive, volume, k.file, &k.spec)?;
        let file = base.core.file.clone();
        // STATICMODEL enables its own model's Hit chunk: 04 and 05 share
        // se2_6, so only this class's models collide.
        let hit_models = HitModel::read(volume, &file.ccs)?;
        base.core.hits =
            event_hits(volume, def_se, model_hits(base.models.iter().flatten().map(|m| m.model), &hit_models));
        let bg = k.bg.iter().map(|(name, _)| crate::story_map::holy_ground::clump_models(&file, name)).collect();
        let eff = SceneFile::read(archive, EFF_FILE)?;
        let pat_num = cloud::eff_pat_num(&eff.ccs, cloud::EFF_SMOKE)
            .ok_or_else(|| Error::NotFound(format!("{EFF_FILE}: {}", cloud::EFF_SMOKE)))?;
        let sun = if k.flare {
            for name in FLARES {
                eff.ccs.find_object(name).ok_or_else(|| Error::NotFound(format!("{EFF_FILE}: {name}")))?;
            }
            Some(marker(&file, SUN)?.0)
        } else {
            None
        };
        // The start as the constructor's `SetStartPos` left it. fieldrand:
        // `GO(1)` sets no seed for a story map, so the clouds' `Init` draws
        // on from where the scene before left it (a gate's
        // `SimGenerateCode`, the dungeon).
        base.core.start = start(class, &file, area_prev)?;
        base.core.field_rand = Some(rng);
        Ok(SkyMap {
            class,
            base,
            bg,
            move_tex: k.move_tex,
            started: false,
            clouds: Vec::new(),
            pat_num,
            sun,
            flares: None,
        })
    }

    /// The class's `Draw` for this frame: the pieces in its order, with
    /// what it steps (the clouds made on the first call, the scroll, the
    /// objects' animations, the clouds' `Move`).
    pub fn select(&mut self, v: &TownView) -> Vec<Piece> {
        let k = self.class.kind();
        let mut out = Vec::new();
        if !self.started {
            self.started = true;
            if let Some(rng) = &mut self.base.core.field_rand {
                self.clouds = (0..CLOUDS).map(|_| Cloud::init(k.cloud_type, self.pat_num, v.player, rng)).collect();
            }
        }
        // DrawBG: MoveTexture, then bg[k] on bgLayer[k].
        self.move_tex.step();
        out.extend((0..k.drawn).map(Piece::Bg));
        // The type-0 models (objLayer, or floorLayer for 05 and 06), then
        // the type-0 objects on objLayer.
        out.extend(self.base.rows(DrawPass::Other).map(Piece::Model));
        out.extend(self.base.step_objects(DrawPass::Other, v.eye).into_iter().map(Piece::Object));
        // effLayer: LENSFLARE::Draw(ccs, DMY_dummy5, 0), then the clouds.
        if let Some(sun) = self.sun {
            let in_view = crate::town::pc::check_camera_deg(sun, lensflare::VIEW_DEG, &v.cam, v.player);
            self.flares = lensflare::draw(sun, &v.flare, v.puppet_show, in_view);
            if let Some(f) = self.flares {
                out.extend(f.into_iter().enumerate().map(|(k, p)| Piece::Flare(k, p)));
            }
        }
        let volume = self.base.core.hits.volume;
        for i in 0..self.clouds.len() {
            if let Some(rng) = &mut self.base.core.field_rand {
                self.clouds[i].step(volume, v.player, rng, &|_, _| 0);
            }
            let c = &self.clouds[i];
            let in_view = crate::town::pc::check_camera_deg(c.pos, cloud::VIEW_DEG, &v.cam, v.player);
            if c.drawn(volume, v.player, in_view) {
                out.push(Piece::Cloud(i));
            }
        }
        out
    }

    /// `MoveTexture`'s material offset as the draws read it.
    fn scroll_rows(&self) -> std::collections::HashMap<u32, [u8; 2]> {
        let file = &self.base.core.file;
        let t = &self.move_tex;
        file.ccs
            .find_object(t.mat)
            .and_then(|mat| Some((mat, file.scene.materials.get(&mat)?)))
            .map(|(mat, m)| {
                let [u, v] = t.offsets();
                (mat, [(u.wrapping_sub(m.crop_u) >> 4) as u8, (v.wrapping_sub(m.crop_v) >> 4) as u8])
            })
            .into_iter()
            .collect()
    }

    fn models_draw(&self, layers: &mut Layers, layer: i16, to_screen: Mat4, models: &[u32], fog: draw::Fogging) {
        let rows = self.scroll_rows();
        for &model in models {
            let d = Draw {
                file: &self.base.core.file,
                model,
                world: Mat4::IDENTITY,
                alpha: 1.0,
                rows: &rows,
                lights: None,
                nodes: &[],
                morph: Vec::new(),
                clut_swaps: Vec::new(),
            };
            draw::model_edited(layers, layer, to_screen, &d, None, fog);
        }
    }

    /// The pieces into the field's layers; the flares and clouds come back
    /// as sprites.
    fn draw_pieces(&self, layers: &mut Layers, to_screen: Mat4, pieces: &[Piece]) -> Vec<StorySprite> {
        let k = self.class.kind();
        let mut sprites = Vec::new();
        for &piece in pieces {
            match piece {
                Piece::Bg(i) => {
                    let Some((_, models)) = &self.bg[i] else { continue };
                    let fog = if k.bg[i].1 { self.base.depth_fog() } else { draw::Fogging::None };
                    self.models_draw(layers, bg_layer(i), to_screen, models, fog);
                }
                Piece::Model(row) => {
                    let Some(p) = self.base.model(row) else { continue };
                    self.models_draw(layers, k.model_layer, to_screen, &[p.model], self.base.depth_fog());
                }
                Piece::Object(row) => self.base.object_draw(layers, layer::OBJ, to_screen, row),
                Piece::Flare(i, pos) => sprites.push(StorySprite {
                    file: EFF_FILE,
                    eff: FLARES[i],
                    pos,
                    layer: layer::EFF,
                    pat: 0,
                    scale: ONE,
                    rotate: 0,
                    transparency: ONE,
                    flare: true,
                    fog: true,
                }),
                Piece::Cloud(i) => {
                    let c = &self.clouds[i];
                    sprites.push(StorySprite {
                        file: EFF_FILE,
                        eff: cloud::EFF_SMOKE,
                        pos: c.pos,
                        layer: layer::EFF,
                        pat: c.pattern as u16,
                        scale: c.eff.scale_x,
                        rotate: c.eff.rotate,
                        transparency: c.eff.transparency,
                        flare: true,
                        fog: true,
                    });
                }
            }
        }
        sprites
    }
}

impl Class {
    /// The class's `EA_moveTex` row in gcmn's data.
    fn scroll_row(self) -> usize {
        match self {
            Class::Mirror => crate::story_map::holy_ground::scroll::EA04,
            Class::Arena => crate::story_map::holy_ground::scroll::EA05,
            Class::Whale => crate::story_map::holy_ground::scroll::EA06,
        }
    }
}

impl StoryMap for SkyMap {
    fn core(&self) -> &AreaCore {
        &self.base.core
    }
    fn core_mut(&mut self) -> &mut AreaCore {
        &mut self.base.core
    }
    fn put_scrolls(&self, gcmn: &mut [[u32; 2]; MOVE_TEX_ROWS]) {
        gcmn[self.class.scroll_row()] = self.move_tex.scroll();
    }
    fn take_scrolls(&mut self, gcmn: &[[u32; 2]; MOVE_TEX_ROWS]) {
        self.move_tex = self.move_tex.at(gcmn, self.class.scroll_row());
    }
    /// `WORLD_MAN::SetCharPosition` (MUT main 0x001b61e0): the leader at
    /// `eventStartPos`; field 66's others at (+200, -150), (-200, -150),
    /// (0, +300) as field 16's, 67's and 91's at (+300, +150), (-300,
    /// +150), (0, +300).
    fn spread(&self) -> StartSpread {
        match self.class {
            Class::Mirror => StartSpread::AHEAD,
            Class::Arena | Class::Whale => StartSpread::BEHIND,
        }
    }
    fn clear(&self) -> [u8; 3] {
        self.class.kind().spec.clear.unwrap_or_default()
    }
    /// `WORLD_MAN::Enter` (MUT main 0x001b2b80): 66 to its dungeon
    /// (`ChangeArea(2, 0)`); 67 nothing; 91 to its dungeon from block 0 or
    /// -1. 91's other branch calls `EVENTAREA07::ChangeBlock(0)` on this
    /// map, but no door or script gives field 91 another block (its
    /// dungeon's way out is `ChangeArea(1, 91)`, block -1), so it stays.
    fn enter(&mut self, block: i32, _area_prev: i32) -> Result<Enter> {
        Ok(match (self.class, block) {
            (Class::Mirror, _) | (Class::Whale, 0 | -1) => Enter::Dungeon,
            _ => Enter::Stay,
        })
    }
    fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, v: &TownView, step: bool) -> Vec<StorySprite> {
        if step {
            let pieces = self.select(v);
            return self.draw_pieces(layers, to_screen, &pieces);
        }
        // Drawn, not stepped: the pieces of a step put back after (the
        // clouds a first step would make are drawn from that step's).
        let saved = (
            self.move_tex,
            self.base.objects.clone(),
            self.started,
            self.clouds.clone(),
            self.base.core.field_rand,
            self.flares,
        );
        let pieces = self.select(v);
        let sprites = self.draw_pieces(layers, to_screen, &pieces);
        (self.move_tex, self.base.objects, self.started, self.clouds, self.base.core.field_rand, self.flares) = saved;
        sprites
    }
}

/// `GO(1)`'s sky maps, by `eventAreaNumber`.
pub const DEF: MapDef = MapDef {
    class: "EVENTAREA04",
    claims: |e| Class::of(e.field).is_some(),
    make: |e| {
        let c = Class::of(e.field).ok_or_else(|| piney_data::Error::NotFound(format!("sky map {}", e.field)))?;
        Ok(Made::New(Box::new(SkyMap::new(e.archive, e.volume, c, e.def_se, e.area_prev, e.field_rand)?)))
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ee;

    /// Each class on each volume: every row of its tables in the file, its
    /// clumps, one light, the collision of its own models only, the
    /// scrolled material, the start, and a first draw with its clouds.
    #[test]
    fn the_sky_maps() {
        for (disc, volume) in [
            ("infection", Volume::Inf),
            ("mutation", Volume::Mut),
            ("outbreak", Volume::Out),
            ("quarantine", Volume::Qua),
        ] {
            let Some(archive) = crate::town::tests::archive_of(disc) else { continue };
            for (class, models, objects) in [(Class::Mirror, 11, 1), (Class::Arena, 9, 1), (Class::Whale, 3, 21)] {
                let m = SkyMap::new(&archive, volume, class, 0, 1, Rng::new(0)).unwrap();
                let k = class.kind();
                assert_eq!((0..models).filter(|&r| m.base.model(r).is_some()).count(), models, "{class:?}");
                assert_eq!((0..objects).filter(|&r| m.base.object(r).is_some()).count(), objects, "{class:?}");
                assert!(m.bg.iter().all(Option::is_some), "{class:?}: a sky clump missing");
                assert_eq!(m.base.core.lights.lights.len(), 1, "{class:?}");
                assert!(!m.base.core.hits.models.is_empty(), "{class:?}: no ground");
                let own: Vec<u32> = m.base.models.iter().flatten().map(|p| p.model).collect();
                assert!(m.base.core.hits.models.iter().all(|h| own.contains(&h.parent)), "{class:?}");
                assert_eq!(m.scroll_rows().len(), 1, "{class:?}: {}", k.move_tex.mat);
                assert_eq!(m.sun.is_some(), k.flare);
                let mut m = m;
                let mut layers = Layers::default();
                let v = TownView { player: m.base.core.start, ..TownView::default() };
                // A draw while the tasks sleep, before any step, puts the
                // clouds back as they were (none yet).
                m.draw(&mut layers, Mat4::IDENTITY, &v, false);
                assert!(m.clouds.is_empty() && !m.started);
                m.draw(&mut layers, Mat4::IDENTITY, &v, true);
                assert_eq!(m.clouds.len(), CLOUDS);
                assert!(!layers.flatten().is_empty(), "{class:?}: nothing drawn");
            }
        }
    }

    /// The starts: 04's by `game.areaPrev`, 05's at y 9000 and z 800.
    #[test]
    fn the_starts() {
        let Some(archive) = crate::town::tests::archive_of("mutation") else { return };
        let gate = SkyMap::new(&archive, Volume::Mut, Class::Mirror, 0, 0, Rng::new(0)).unwrap().base.core.start;
        let back = SkyMap::new(&archive, Volume::Mut, Class::Mirror, 0, 2, Rng::new(0)).unwrap().base.core.start;
        assert_eq!(gate, [ee::k(4600.0), ee::k(-4700.0), ee::k(910.0), 0x4049_0fdb]);
        assert_eq!(back, [ee::k(-8000.0), ee::k(5800.0), ee::k(920.0), 0]);
        let arena = SkyMap::new(&archive, Volume::Mut, Class::Arena, 0, 2, Rng::new(0)).unwrap().base.core.start;
        assert_eq!((arena[1], arena[2]), (ee::k(9000.0), ee::k(800.0)));
    }
}
