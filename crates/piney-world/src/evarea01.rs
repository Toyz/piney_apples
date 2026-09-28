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

use crate::draw::{self, Draw};
use crate::ee::{F, ONE, V4};
use crate::evarea::{BOUNDS, StorySprite, layer};
use crate::hit::{HitModel, Hits};
use crate::story_map::StoryMap;
use crate::town::{Base, FogParams, Spec, TownLights, TownView};

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

const SPEC: Spec = Spec {
    no: -1,
    models: "EA_MODELTABLE01",
    objects: Some("EA_OBJTABLE01"),
    light_anim: "ANM_se1_1bac1a",
    lights: &LIGHTS,
    fog: FOG,
    clear: Some(BG_COLOR),
};

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
    pub base: Base,
    /// `bg[0]`: the background clump's models.
    pub bg: Option<(u32, Vec<u32>)>,
    /// `DMY_marker_ev01`'s position, w its turn about z.
    pub start: V4,
    pub glows: [Glow; 7],
    /// The glow's pattern count (`ccEff` +0x30).
    pub pat_num: u16,
    /// `fieldrand`: `GO(1)` sets no seed for a story map; the map keeps its
    /// own, from 0.
    pub rng: Rng,
}

impl Area13 {
    /// `EVENTAREA01::EVENTAREA01`.
    pub fn new(archive: &Arc<Archive>, volume: Volume, def_se: u32) -> Result<Area13> {
        let mut base = Base::read_volume(archive, volume, FILE, &SPEC)?;
        let (c, sc) = (&base.file.ccs, &base.file.scene);
        // The collision: the models' Hit chunks, in the order the
        // STATICMODELs were built, over GO's bounds.
        let hit_models = HitModel::read(c)?;
        let mut hits =
            Hits { area: 1, bounds: Some(BOUNDS), heights: None, def_se, event_area: true, ..Hits::default() };
        hits.models = base
            .models
            .iter()
            .flatten()
            .filter_map(|m| hit_models.iter().find(|h| h.parent == m.model).cloned())
            .collect();
        let bg = crate::evarea::clump_models(&base.file, BG);
        let (pos, rot) = crate::town::dummy_bits(&Position::DummyRot(START), c, sc);
        if c.find_object(START).is_none() {
            return Err(Error::NotFound(format!("{FILE}: {START}")));
        }
        let pat_num = crate::cloud::eff_pat_num(c, EFF).ok_or_else(|| Error::NotFound(format!("{FILE}: {EFF}")))?;
        // Each glow: its frame, its wait and its flicker drawn from
        // fieldrand in that order; then the places.
        let mut rng = Rng::new(0);
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
        base.hits = hits;
        Ok(Area13 { base, bg, start: [pos[0], pos[1], pos[2], rot[2]], glows, pat_num, rng })
    }

    /// Each glow's `ccEff::Draw` this frame: waiting, its animation's next
    /// frame (wrapping at the last); its wait out, a random frame, `count`
    /// times, then a new wait (30-129) and flicker (1-15).
    pub fn glow_frames(&mut self) -> [u32; 7] {
        let n = u32::from(self.pat_num).max(1);
        let mut out = [0; 7];
        for (k, g) in self.glows.iter_mut().enumerate() {
            if g.wait == 0 {
                out[k] = self.rng.below(n) & 0xffff;
                g.count = g.count.wrapping_sub(1);
                if g.count == 0 {
                    g.wait = self.rng.below(100) + 30;
                    g.count = self.rng.below(15) + 1;
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
            file: &self.base.file,
            model,
            world: Mat4::IDENTITY,
            alpha: 1.0,
            rows: &rows,
            lights: None,
            nodes: &[],
            morph: Vec::new(),
            clut_swaps: Vec::new(),
        };
        draw::model_edited(layers, layer, to_screen, d, None, draw::Fogging::None);
    }
}

impl StoryMap for Area13 {
    fn hits(&self) -> &Hits {
        &self.base.hits
    }
    fn file(&self) -> Option<&piney_desktop::assets::SceneFile> {
        Some(&self.base.file)
    }
    fn hits_mut(&mut self) -> &mut Hits {
        &mut self.base.hits
    }
    fn lights(&self) -> &TownLights {
        &self.base.lights
    }
    /// `WORLD_MAN::SetCharPosition` in a story map: the leader at the
    /// start, the others at (+300, +150), (-300, +150), (0, +300).
    fn start_positions(&self) -> (V4, F, [V4; 3]) {
        use crate::ee::{add, sub};
        let [x, y, z, dirc] = self.start;
        let starts = [
            [add(0x4396_0000, x), add(0x4316_0000, y), z, ONE],
            [sub(x, 0x4396_0000), add(0x4316_0000, y), z, ONE],
            [x, add(0x4396_0000, y), z, ONE],
        ];
        ([x, y, z, ONE], dirc, starts)
    }
    fn clear(&self) -> [u8; 3] {
        BG_COLOR
    }
    /// `Draw`: `DrawBG` (the clump at the identity, fog off), then on
    /// `objLayer` `DrawObj2`, `DrawObj`, `DrawFloor` and the type-0 rows,
    /// then the glows on `effLayer`.
    fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, v: &TownView, step: bool) -> Vec<StorySprite> {
        if let Some((_, models)) = &self.bg {
            for &m in models {
                self.clump_draw(layers, layer::BG0, to_screen, m);
            }
        }
        for pass in [DrawPass::Obj2, DrawPass::Obj] {
            for row in self.base.rows(pass).collect::<Vec<_>>() {
                self.base.row_draw(layers, layer::OBJ, to_screen, row);
            }
            let rows = if step { self.base.step_objects(pass, v.eye) } else { Vec::new() };
            for row in rows {
                self.base.object_draw(layers, layer::OBJ, to_screen, row);
            }
        }
        for pass in [DrawPass::Floor, DrawPass::Other] {
            for row in self.base.rows(pass).collect::<Vec<_>>() {
                self.base.row_draw(layers, layer::OBJ, to_screen, row);
            }
        }
        let frames = if step {
            self.glow_frames()
        } else {
            let saved = (self.glows, self.rng);
            let f = self.glow_frames();
            (self.glows, self.rng) = saved;
            f
        };
        self.glows
            .iter()
            .zip(frames)
            .map(|(g, pat)| StorySprite {
                file: FILE,
                eff: EFF,
                pos: g.pos,
                layer: layer::EFF,
                pat: pat as u16,
                scale: ONE,
                rotate: 0,
                transparency: ONE,
                flare: false,
                fog: false,
            })
            .collect()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
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

    /// Mutation's area 13: the seven models, the object, the eleven
    /// lights, the background, the collision, the start, and the glows'
    /// first frames from fieldrand.
    #[test]
    fn area_13() {
        let Some(archive) = archive() else { return };
        let mut a = Area13::new(&archive, Volume::Mut, 0).unwrap();
        assert_eq!((0..7).filter(|&r| a.base.model(r).is_some()).count(), 7);
        assert!(a.base.object(0).is_some());
        assert_eq!(a.base.lights.lights.len(), 11);
        assert!(a.bg.is_some());
        assert!(!a.base.hits.models.is_empty());
        assert!(a.pat_num > 0);
        assert!(a.glows.iter().all(|g| g.frame < u32::from(a.pat_num) && (30..130).contains(&g.wait)));
        let mut layers = Layers::default();
        let sprites = a.draw(&mut layers, Mat4::IDENTITY, &TownView::default(), true);
        assert_eq!(sprites.len(), 7);
        assert!(sprites.iter().all(|s| !s.fog && !s.flare && s.layer == layer::EFF));
        assert!(!layers.flatten().is_empty());
    }
}
