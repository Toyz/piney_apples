//! Area 13's story map: `EVENTAREA01` (gcmn area01.cpp; MUT 0x00414110 the
//! constructor, 0x00414c00 `Draw`), which `GO(1)` makes for field 13
//! (Mutation's event 115). The scene `se1_1`: seven static models, one
//! object, a background, eleven lights, and seven `EFF_se1_1ef1` glows that
//! play their animation and now and then flicker (docs/engine/evarea.md).

use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::statics::{DrawPass, Position};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::layers::Layers;

use crate::area::AreaCore;
use crate::draw::{self, Draw};
use crate::ee::{ONE, V4};
use crate::hit::HitModel;
use crate::sprite::{EffSprite, EffState};
use crate::story_map::StoryMap;
use crate::story_map::holy_ground::layer;
use crate::story_map::{Made, MapDef};
use crate::story_map::{event_hits, model_hits};
use crate::systems::statics::{ModelsDef, Names, ObjectsDef, StaticModels, StaticObjects};
use crate::systems::{Request, paint};
use crate::town::{FogParams, TownView};

/// `game.field` of this map.
pub const AREA: i32 = 13;
const FILE: &str = "se1_1";
const BG: &str = "CMP_se1_1bg1";
const START: &str = "DMY_marker_ev01";
const EFF: &str = "EFF_se1_1ef1";
/// Where the glows stand (MUT 0x005ffeb0).
const GLOWS: [&str; 7] = [
    "DMY_effpoint01",
    "DMY_effpoint02",
    "DMY_effpoint03",
    "DMY_effpoint04",
    "DMY_effpoint05",
    "DMY_effpoint06",
    "DMY_effpoint07",
];

/// `eventarea01Light`: the distant light, then ten omni lights.
const LIGHTS: [(i32, &str); 11] = [
    (0, "LGT_se1_1lig1"),
    (1, "LGT_se1_1omn01"),
    (1, "LGT_se1_1omn02"),
    (1, "LGT_se1_1omn03"),
    (1, "LGT_se1_1omn04"),
    (1, "LGT_se1_1omn05"),
    (1, "LGT_se1_1omn06"),
    (1, "LGT_se1_1omn07"),
    (1, "LGT_se1_1omn08"),
    (1, "LGT_se1_1omn09"),
    (1, "LGT_se1_1omn10"),
];

/// `SetFog(150, 4000, 0, 80, 0x1e1e1e)`, and `ccSys.bgColor` 0x1e1e1e.
const FOG: FogParams = FogParams { near: 150.0, far: 4000.0, near_rate: 0.0, far_rate: 80.0, colour: [0x1e; 3] };
pub const BG_COLOR: [u8; 3] = [0x1e; 3];

const LIGHT_ANIM: &str = "ANM_se1_1bac1a";
static MODELS: ModelsDef = ModelsDef { table: "EA_MODELTABLE01", layer: layer::OBJ, names: Names::Table };
static OBJECTS: ObjectsDef = ObjectsDef { table: "EA_OBJTABLE01", anm_num: None };

/// One glow (`eff[k]` +0x1c4, `pos` +0x1e0, `frame` +0x250, `wait`
/// +0x26c, `count` +0x288).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glow {
    pub pos: V4,
    pub frame: u32,
    pub wait: u32,
    pub count: u32,
}

/// `EVENTAREA01` as `WORLD_MAN.eventmap` holds it.
pub struct Area13 {
    pub core: AreaCore,
    pub models: StaticModels,
    pub objects: StaticObjects,
    /// `bg[0]`: the background clump's models.
    pub bg: Option<(u32, Vec<u32>)>,
    pub glows: [Glow; 7],
    /// The glow's pattern count (`ccEff` +0x30).
    pub pat_num: u16,
}

impl Area13 {
    /// `EVENTAREA01::EVENTAREA01`, `fieldrand` the game's (`rng`).
    pub fn new(archive: &Arc<Archive>, volume: Volume, def_se: u32, rng: Rng) -> Result<Area13> {
        let mut core = AreaCore::read(archive, volume, FILE, LIGHT_ANIM, &LIGHTS, FOG)?;
        let models = StaticModels::read(&MODELS, &core.file, volume)?;
        let objects = StaticObjects::read(&OBJECTS, &core.file, volume)?;
        let (c, sc) = (&core.file.ccs, &core.file.scene);
        // The collision: the models' Hit chunks, in the order the
        // STATICMODELs were built, over GO's bounds.
        let hit_models = HitModel::read(volume, c)?;
        let hits = event_hits(volume, def_se, model_hits(models.iter().map(|m| m.model), &hit_models));
        let bg = crate::story_map::holy_ground::clump_models(&core.file, BG);
        let (pos, rot) = crate::town::dummy_bits(&Position::DummyRot(START), c, sc);
        if c.find_object(START).is_none() {
            return Err(Error::NotFound(format!("{FILE}: {START}")));
        }
        let pat_num =
            crate::sky::cloud::eff_pat_num(c, EFF).ok_or_else(|| Error::NotFound(format!("{FILE}: {EFF}")))?;
        // Each glow: its frame, its wait and its flicker drawn from
        // fieldrand in that order; then the places.
        let mut rng = rng;
        let mut glows = [Glow { pos: [0, 0, 0, ONE], frame: 0, wait: 0, count: 0 }; 7];
        for g in &mut glows {
            g.frame = rng.below(u32::from(pat_num));
            g.wait = rng.below(100) + 30;
            g.count = rng.below(15) + 1;
        }
        for (g, name) in glows.iter_mut().zip(GLOWS) {
            if c.find_object(name).is_none() {
                return Err(Error::NotFound(format!("{FILE}: {name}")));
            }
            g.pos = crate::town::dummy_bits(&Position::Dummy(name), c, sc).0;
        }
        core.hits = hits;
        // The start: `DMY_marker_ev01`'s position, w its turn about z.
        // fieldrand: `GO(1)` sets no seed for a story map, so the map draws
        // on from where the scene before left it.
        core.start = [pos[0], pos[1], pos[2], rot[2]];
        core.field_rand = Some(rng);
        Ok(Area13 { core, models, objects, bg, glows, pat_num })
    }

    /// Each glow's `ccEff::Draw` this frame: waiting, its animation's next
    /// frame (wrapping at the last); its wait out, a random frame, `count`
    /// times, then a new wait (30-129) and flicker (1-15).
    pub fn glow_frames(&mut self) -> [u32; 7] {
        let n = u32::from(self.pat_num).max(1);
        let mut out = [0; 7];
        let Some(rng) = &mut self.core.field_rand else { return out };
        for (k, g) in self.glows.iter_mut().enumerate() {
            if g.wait == 0 {
                out[k] = rng.below(n) & 0xffff;
                g.count = g.count.wrapping_sub(1);
                if g.count == 0 {
                    g.wait = rng.below(100) + 30;
                    g.count = rng.below(15) + 1;
                }
            } else {
                g.wait -= 1;
                out[k] = g.frame;
                g.frame += 1;
                if g.frame == u32::from(self.pat_num) {
                    g.frame = 0;
                }
            }
        }
        out
    }

    fn clump_draw(&self, layers: &mut Layers, layer: i16, to_screen: Mat4, model: u32) {
        let rows = Default::default();
        let d = Draw {
            file: &self.core.file,
            model,
            world: Mat4::IDENTITY,
            alpha: 1.0,
            rows: &rows,
            lights: None,
            nodes: &[],
            morph: Vec::new(),
            clut_swaps: Vec::new(),
        };
        draw::model_edited(layers, layer, to_screen, &d, None, draw::Fogging::None);
    }
}

impl StoryMap for Area13 {
    fn core(&self) -> &AreaCore {
        &self.core
    }
    fn core_mut(&mut self) -> &mut AreaCore {
        &mut self.core
    }
    fn clear(&self) -> [u8; 3] {
        BG_COLOR
    }
    /// `Draw`: `DrawBG` (the clump at the identity, fog off), then on
    /// `objLayer` `DrawObj2`, `DrawObj`, `DrawFloor` and the type-0 rows,
    /// then the glows on `effLayer`.
    fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, v: &TownView, step: bool) -> Vec<EffSprite> {
        if let Some((_, models)) = &self.bg {
            for &m in models {
                self.clump_draw(layers, layer::BG0, to_screen, m);
            }
        }
        let fog = draw::Fogging::Depth(FOG.depth());
        let file = &*self.core.file;
        // The rows and objects draw no ccEff.
        let mut no_sprites = Vec::new();
        let mut put = |layers: &mut Layers, r: Option<Request>| {
            if let Some(r) = r {
                paint(layers, to_screen, &r, &mut no_sprites);
            }
        };
        for pass in [DrawPass::Obj2, DrawPass::Obj] {
            for row in self.models.rows(pass) {
                put(layers, self.models.request(file, row, layer::OBJ, fog));
            }
            let volume = self.core.hits.volume;
            let rows = if step { self.objects.step_pass(pass, v.eye, volume, file) } else { Vec::new() };
            for row in rows {
                put(layers, self.objects.request(file, row, layer::OBJ, fog));
            }
        }
        for pass in [DrawPass::Floor, DrawPass::Other] {
            for row in self.models.rows(pass) {
                put(layers, self.models.request(file, row, layer::OBJ, fog));
            }
        }
        let frames = if step {
            self.glow_frames()
        } else {
            let saved = (self.glows, self.core.field_rand);
            let f = self.glow_frames();
            (self.glows, self.core.field_rand) = saved;
            f
        };
        self.glows
            .iter()
            .zip(frames)
            .map(|(g, pat)| EffSprite::new(FILE, EFF, g.pos, pat as u16, layer::EFF, EffState::NO_FOG))
            .collect()
    }
}

/// `GO(1)`'s area 13.
pub const DEF: MapDef = MapDef {
    class: "EVENTAREA01",
    claims: |e| e.field == AREA,
    make: |e| Ok(Made::New(Box::new(Area13::new(e.archive, e.volume, e.def_se, e.field_rand)?))),
};

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

    /// Mutation's area 13: the seven models, the object, the eleven
    /// lights, the background, the collision, the start, and the glows'
    /// first frames from fieldrand.
    #[test]
    fn area_13() {
        let Some(archive) = archive() else { return };
        let mut a = Area13::new(&archive, Volume::Mut, 0, Rng::new(0)).unwrap();
        assert_eq!((0..7).filter(|&r| a.models.model(r).is_some()).count(), 7);
        assert!(a.objects.object(0).is_some());
        assert_eq!(a.core.lights.lights.len(), 11);
        assert!(a.bg.is_some());
        assert!(!a.core.hits.models.is_empty());
        assert!(a.pat_num > 0);
        assert!(a.glows.iter().all(|g| g.frame < u32::from(a.pat_num) && (30..130).contains(&g.wait)));
        let mut layers = Layers::default();
        let sprites = a.draw(&mut layers, Mat4::IDENTITY, &TownView::default(), true);
        assert_eq!(sprites.len(), 7);
        assert!(sprites.iter().all(|s| s.state == EffState::NO_FOG && s.layer == layer::EFF));
        assert!(!layers.flatten().is_empty());
    }
}
