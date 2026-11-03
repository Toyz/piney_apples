//! Area 43's story map: `EVENTAREA03` (gcmn area03.cpp; MUT 0x00416590 the
//! constructor, 0x00416bf0 `Draw`; INF 0x00403590, 0x00403bf0), which
//! `GO(1)` makes for field 43. The scene `se2_1`: two models, one distant
//! light, a fixed start. Unless area 43 is marked on the server (event 102's
//! `gate_mark`), `Draw` flies the camera over it, Kite speaks to himself and
//! the party goes back to town ([`FlyOver`]). docs/engine/evarea.md.

use piney_data::Result;
use piney_data::classes::StoryMapGameClass;
use piney_data::statics::DrawPass;

use crate::camera::{Camera, id};
use crate::ee::{self, F, ONE, V4};
use crate::story_map::{BASE_BLOCK, BASE_STORY_MAP, BlockDef, LightsDef, Start, StoryMapDef, layer};
use crate::story_map::{StoryFrame, StoryRequest};
use crate::systems::statics::ModelsDef;
use crate::systems::{BuildCx, Fog, Hook, Kind, MapSystem, Step, SystemDef};
use crate::town::FogParams;

/// `game.field` of this map.
pub const AREA: i32 = 43;
const FILE: &str = "se2_1";
/// `modelArray`: `EA_MODELTABLE03`'s two rows, on objLayer, no fog.
static MODELS: ModelsDef = ModelsDef { fog: Fog::None, ..ModelsDef::on("EA_MODELTABLE03", layer::OBJ) };
/// `ANM_se2_1_1a`, and `eventarea03Light`'s one row: a distant light.
const LIGHTS: LightsDef = LightsDef { anim: "ANM_se2_1_1a", table: &["LGT_se2_1lig1"], skip_missing: false };

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

/// `EVENTAREA03`'s own: the fly-over `Draw` runs while area 43 is not
/// marked on the server.
#[derive(Clone, Debug)]
pub struct FlyOver {
    /// +0x1c4 `cnt`: the fly-over's frame.
    pub cnt: i32,
    /// +0x1c8 `flag`: area 43 marked on the server as the map was made.
    pub flag: bool,
    /// Where `kiteSelfTalk` is in the volume.
    self_talk: u32,
}

impl FlyOver {
    /// The constructor's: `gateListMark[server][1]` bit 11 of the save.
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<FlyOver> {
        let at = |n: i32| piney_data::save::offset::GATE_LIST_MARK + 20 * n.clamp(0, 4) as usize + 4 * MARK_WORD;
        let marks = cx.map.and_then(|m| Some((m.save?, m.server)));
        let flag = marks.is_some_and(|(s, n)| s.i32(at(n)) as u32 & MARK_BIT != 0);
        let self_talk = piney_data::tables::fieldui::of(cx.volume).kite_self_talk_va;
        Ok(FlyOver { cnt: 0, flag, self_talk })
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
}

impl MapSystem for FlyOver {
    /// `Draw`'s scene while area 43 is not marked: camera 3 from
    /// (`VIEW0`, `POS0`) to (`VIEW1`, `POS1`) over 120 of 140 frames with
    /// the menus banned; at 140 `kiteSelfTalk`; once it closes the menus
    /// back, the field camera, and `ChangeArea(0, 1)`.
    fn scene(&mut self, x: &mut StoryFrame) {
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
}

/// `EVENTAREA03`, field 43: the disc's own tables, the fixed start. Its
/// `Draw` draws `DrawFloor` (the floor models inside `BLT_floor`: none,
/// both rows are pass 0), then the pass-0 models on objLayer.
pub static AREA_43: StoryMapDef = StoryMapDef {
    class: StoryMapGameClass::Eventarea03,
    fields: &[AREA],
    file: FILE,
    clear: BG_COLOR,
    blocks: &[BlockDef {
        lights: Some(LIGHTS),
        fog: Some(FOG),
        start: Start::At(START),
        systems: &[SystemDef::Models(&MODELS), SystemDef::FlyOver(&())],
        order: &[
            Step::Run(Kind::Models, Hook::Pass(DrawPass::Floor)),
            Step::Run(Kind::Models, Hook::Pass(DrawPass::Other)),
        ],
        ..BASE_BLOCK
    }],
    ..BASE_STORY_MAP
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story_map::tests::{made, made_with};
    use crate::systems::statics::StaticModels;
    use piney_data::save::SaveData;
    use piney_data::volume::Volume;

    /// Mutation's area 43: `se2_1`'s two models, the floor's collision,
    /// the one light, and the mark read from the save.
    #[test]
    fn area_43() {
        let Some(archive) = crate::town::tests::archive_of("mutation") else { return };
        let mut save = SaveData::default();
        let mut a = made(&archive, Volume::Mut, &AREA_43, 1, 0);
        let models = a.systems().get::<StaticModels>().unwrap();
        let name = |o: u32| models.file().ccs.object_name(o).unwrap().to_string();
        assert_eq!(models.iter().map(|m| name(m.model)).collect::<Vec<_>>(), ["MDL_se2_1fl1", "MDL_se2_1ba1"]);
        assert!(!a.core.hits.models.is_empty());
        assert_eq!(a.core.lights.lights.len(), 1);
        assert!(!a.systems().get::<FlyOver>().unwrap().flag);
        // Draw: both pass-0 models go out on objLayer.
        let mut layers = piney_desktop::layers::Layers::default();
        let mut cc = piney_battle::rand::Genrand::default();
        a.draw(&mut layers, glam::Mat4::IDENTITY, &crate::systems::MapView::default(), true, &mut cc);
        assert!(!layers.flatten().is_empty(), "nothing drawn");
        let at = piney_data::save::offset::GATE_LIST_MARK + 20 + 4;
        save.set_i32(at, MARK_BIT as i32);
        let marked = made_with(&archive, Volume::Mut, &AREA_43, 1, 0, &save, 1);
        assert!(marked.systems().get::<FlyOver>().unwrap().flag);
    }
}
