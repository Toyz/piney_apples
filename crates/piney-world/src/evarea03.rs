//! Area 43's story map: `EVENTAREA03` (gcmn area03.cpp; MUT 0x00416590 the
//! constructor, 0x00416bf0 `Draw`; INF 0x00403590, 0x00403bf0), which
//! `GO(1)` makes for field 43. The scene `se2_1`: two models, one distant
//! light, a fixed start. Unless area 43 is marked on the server (event 102's
//! `gate_mark`), `Draw` flies the camera over it, Kite speaks to himself and
//! the party goes back to town ([`Area43::fly_over`]). docs/engine/evarea.md.

use std::rc::Rc;
use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_data::save::SaveData;
use piney_data::statics::{self, DrawPass, Position};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::camera::{Camera, id};
use crate::draw::{self, Draw};
use crate::ee::{self, F, ONE, V4};
use crate::evarea::{BOUNDS, StaticModel, StorySprite, layer};
use crate::hit::{HitModel, Hits};
use crate::story_map::{StoryFrame, StoryMap, StoryRequest};
use crate::town::{FogParams, Light, TownLights, TownView};

/// `game.field` of this map.
pub const AREA: i32 = 43;
const FILE: &str = "se2_1";
const MODEL_TABLE: &str = "EA_MODELTABLE03";
/// `ANM_se2_1_1a`, and `eventarea03Light`'s one row: a distant light.
const ANIM: &str = "ANM_se2_1_1a";
const LIGHT: &str = "LGT_se2_1lig1";

/// `SetFog(150, 4000, 0, 80, 0x1e1e1e)`, and `ccSys.bgColor` 0x1e1e1e.
pub const FOG: FogParams = FogParams { near: 150.0, far: 4000.0, near_rate: 0.0, far_rate: 80.0, colour: [0x1e; 3] };
pub const BG_COLOR: [u8; 3] = [0x1e; 3];

/// `SetStartPos` (MUT 0x00600140): (0, 0, 0), w 1.0.
const START: V4 = [0, 0, 0, ONE];

/// The fly-over's camera (MUT 0x00600170-0x006001a0): view and position
/// from, then to.
const VIEW0: V4 = [0x44a5_4000, 0xc48d_0000, 0x447d_8000, 0];
const POS0: V4 = [0x44d1_c000, 0xc4b3_2000, 0x4494_8000, 0];
const VIEW1: V4 = [0xc352_0000, 0x4327_0000, 0x4210_0000, 0];
const POS1: V4 = [0x4322_0000, 0xc305_0000, 0x4332_0000, 0];
/// The frames of the move (t = cnt / 120, at most 1), and when Kite speaks.
const MOVE: F = 0x42f0_0000;
const SPEAKS: i32 = 140;

/// Area 43 marked on a server: `gateListMark[server][1]` bit 11.
const MARK_WORD: usize = 1;
const MARK_BIT: u32 = 0x800;

/// `EVENTAREA03` as `WORLD_MAN.eventmap` holds it.
pub struct Area43 {
    pub file: Rc<SceneFile>,
    /// `modelArray`: `EA_MODELTABLE03`'s rows.
    pub models: Vec<StaticModel>,
    pub lights: TownLights,
    /// +0x1c4 `cnt`: the fly-over's frame.
    pub cnt: i32,
    /// +0x1c8 `flag`: area 43 marked on the server as the map was made.
    pub flag: bool,
    /// Where `kiteSelfTalk` is in the volume.
    self_talk: u32,
    pub hits: Hits,
}

impl Area43 {
    /// `EVENTAREA03::EVENTAREA03` on `server`, with `save`'s marks.
    pub fn new(archive: &Arc<Archive>, volume: Volume, save: &SaveData, server: i32, def_se: u32) -> Result<Area43> {
        let file = Rc::new(SceneFile::read(archive, FILE)?);
        let (c, sc) = (&file.ccs, &file.scene);
        let table = statics::of(volume)
            .0
            .iter()
            .find(|t| t.name == MODEL_TABLE)
            .ok_or_else(|| Error::NotFound(MODEL_TABLE.into()))?;
        let mut models = Vec::new();
        for (row, r) in table.rows.iter().enumerate() {
            let model = c.find_object(r.model).ok_or_else(|| Error::NotFound(format!("{FILE}: {}", r.model)))?;
            let pos = match r.position {
                Position::None => [0; 4],
                _ => crate::town::dummy_bits(&r.position, c, sc).0,
            };
            models.push(StaticModel { row, pass: r.pass, model, pos, clip: r.clip.to_bits() });
        }
        // The light animation at frame 1, its ambient and the one light.
        let ai = file.anim(ANIM).ok_or_else(|| Error::NotFound(ANIM.into()))?;
        let (ambient, records) = crate::town::anim_lights(&file, ai)?;
        let light = c.find_object(LIGHT).ok_or_else(|| Error::NotFound(LIGHT.into()))?;
        let group: Vec<Light> = records.iter().filter(|l| l.object == light).map(|l| l.at(256)).collect();
        let lights = TownLights { ambient: ambient.unwrap_or(glam::Vec3::ZERO), lights: group, fog: Some(FOG.depth()) };
        let hit_models = HitModel::read(c)?;
        let mut hits =
            Hits { area: 1, bounds: Some(BOUNDS), heights: None, def_se, event_area: true, ..Hits::default() };
        hits.models = models.iter().filter_map(|m| hit_models.iter().find(|h| h.parent == m.model).cloned()).collect();
        let at = piney_data::save::offset::GATE_LIST_MARK + 20 * server.clamp(0, 4) as usize + 4 * MARK_WORD;
        let flag = save.i32(at) as u32 & MARK_BIT != 0;
        let self_talk = piney_data::tables::fieldui::of(volume).kite_self_talk_va();
        Ok(Area43 { file, models, lights, cnt: 0, flag, self_talk, hits })
    }

    /// `cameraSetView`, `cameraSetPos`, then `cameraGetRot` into
    /// `cameraSetRot`, on the active camera (the scratch `GetRot` reads is
    /// unset in the game; 0 here).
    fn aim(camera: &mut Camera, view: V4, pos: V4) {
        let n = camera.cam_id;
        let c = camera.cam_mut(n);
        c.view = view;
        c.pos = pos;
        let rot = camera.get_rot(n, [0; 4]);
        camera.cam_mut(n).rot = rot;
    }

    /// `Draw`'s scene while area 43 is not marked: camera 3 from
    /// (`VIEW0`, `POS0`) to (`VIEW1`, `POS1`) over 120 of 140 frames with
    /// the menus banned; at 140 `kiteSelfTalk`; once it closes the menus
    /// back, the field camera, and `ChangeArea(0, 1)`.
    pub fn fly_over(&mut self, x: &mut StoryFrame) {
        if self.flag {
            return;
        }
        if self.cnt == 0 {
            x.camera.change_camera(id::EVENT);
            Self::aim(x.camera, VIEW0, POS0);
            x.out.push(StoryRequest::MenuBan(true));
        }
        if self.cnt < SPEAKS {
            let mut t = ee::div(ee::from_int(self.cnt), MOVE);
            if !ee::le(t, ONE) {
                t = ONE;
            }
            let lerp = |a: V4, b: V4| {
                let l = |k: usize| ee::add(a[k], ee::mul(t, ee::sub(b[k], a[k])));
                [l(0), l(1), l(2), 0]
            };
            Self::aim(x.camera, lerp(VIEW0, VIEW1), lerp(POS0, POS1));
            self.cnt += 1;
        }
        if self.cnt == SPEAKS {
            x.out.push(StoryRequest::Message { rec: self.self_talk });
            self.cnt += 1;
        }
        if self.cnt == SPEAKS + 1 && x.message_closed {
            x.out.push(StoryRequest::MenuBan(false));
            x.camera.change_camera(id::FIELD);
            x.out.push(StoryRequest::ChangeArea(crate::area::kind::TOWN, 1));
            self.cnt += 1;
        }
    }

    fn model_draw(&self, layers: &mut Layers, to_screen: Mat4, m: &StaticModel) {
        let world = Mat4::from_translation(glam::Vec3::new(ee::f(m.pos[0]), ee::f(m.pos[1]), ee::f(m.pos[2])));
        let d = Draw {
            file: &self.file,
            model: m.model,
            world,
            alpha: 1.0,
            rows: &Default::default(),
            lights: None,
            nodes: &[],
            morph: Vec::new(),
            clut_swaps: Vec::new(),
        };
        draw::model(layers, layer::OBJ, to_screen, d);
    }
}

impl StoryMap for Area43 {
    fn hits(&self) -> &Hits {
        &self.hits
    }
    fn hits_mut(&mut self) -> &mut Hits {
        &mut self.hits
    }
    fn lights(&self) -> &TownLights {
        &self.lights
    }
    /// `WORLD_MAN::SetCharPosition` in a story map: the leader at the
    /// start, the others at (+300, +150), (-300, +150), (0, +300).
    fn start_positions(&self) -> (V4, F, [V4; 3]) {
        let [x, y, z, dirc] = START;
        let starts = [
            [ee::add(0x4396_0000, x), ee::add(0x4316_0000, y), z, ONE],
            [ee::sub(x, 0x4396_0000), ee::add(0x4316_0000, y), z, ONE],
            [x, ee::add(0x4396_0000, y), z, ONE],
        ];
        ([x, y, z, ONE], dirc, starts)
    }
    fn clear(&self) -> [u8; 3] {
        BG_COLOR
    }
    fn frame(&mut self, x: &mut StoryFrame) {
        self.fly_over(x);
    }
    /// `Draw`'s models: `DrawFloor` (the floor models inside `BLT_floor`:
    /// none, both rows are pass 0), then the pass-0 models on `objLayer`.
    fn draw(&mut self, layers: &mut Layers, to_screen: Mat4, _v: &TownView, _step: bool) -> Vec<StorySprite> {
        for m in self.models.iter().filter(|m| m.pass == DrawPass::Floor) {
            self.model_draw(layers, to_screen, m);
        }
        for m in self.models.iter().filter(|m| m.pass == DrawPass::Obj) {
            self.model_draw(layers, to_screen, m);
        }
        Vec::new()
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

    /// Mutation's area 43: `se2_1`'s two models, the floor's collision,
    /// the one light, and the mark read from the save.
    #[test]
    fn area_43() {
        let Some(archive) = archive() else { return };
        let mut save = SaveData::default();
        let a = Area43::new(&archive, Volume::Mut, &save, 1, 0).unwrap();
        let name = |o: u32| a.file.ccs.object_name(o).unwrap().to_string();
        assert_eq!(a.models.iter().map(|m| name(m.model)).collect::<Vec<_>>(), ["MDL_se2_1fl1", "MDL_se2_1ba1"]);
        assert!(!a.hits.models.is_empty());
        assert_eq!(a.lights.lights.len(), 1);
        assert!(!a.flag);
        let at = piney_data::save::offset::GATE_LIST_MARK + 20 + 4;
        save.set_i32(at, MARK_BIT as i32);
        assert!(Area43::new(&archive, Volume::Mut, &save, 1, 0).unwrap().flag);
    }
}
