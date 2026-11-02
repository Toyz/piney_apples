//! The boss arenas: `EVENTAREAB0` (gcmn areab0.cpp, constructor 0x004079f0,
//! `Draw` 0x004095e0), the story map `WORLD_MAN::GO(1)` makes for fields 1-8,
//! each a stage of its own scene file (Infection's Skeith on field 1,
//! `se1_5`). Nine static models, three sky clumps with a scrolling cloud
//! layer, 54 `FIREFLY2`s ([`crate::systems::fireflies`]), a reflection model
//! on the layer `SwitchLayer` flips (Skeith's magic) and three bobbing
//! models. The collision is the floor model's Hit chunk, with no height map
//! (docs/engine/evarea.md).

use std::borrow::Cow;
use std::rc::Rc;

use glam::Mat4;
use piney_data::classes::StoryMapGameClass;
use piney_data::statics::DrawPass;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use crate::ee::{self, F};
use crate::hit::HitModel;
use crate::story_map::{Asleep, BASE_BLOCK, BASE_STORY_MAP, BlockDef, LightsDef, StageByte, Start, StoryMapDef, layer};
use crate::systems::fireflies::Firefly2Def;
use crate::systems::scrolls::{MatScroll, ScrollsDef, Shown, Wrap};
use crate::systems::sky::{SkyClump, SkyDef};
use crate::systems::statics::{ModelsDef, PassLayers, StaticModels};
use crate::systems::{
    BuildCx, DrawCx, Fog, Hook, Kind, MapSystem, Others, Piece, PlaceCx, Request, Signal, Step, StepCx, SystemDef,
};
use crate::town::{FogParams, clump_models};

/// Each field's stage (`game.field` 1-8; the constructor's jump table at
/// 0x006a3d60). Field 8 also loads `se4_8`, `NextStage`'s last stage.
pub const STAGES: [&str; 9] = ["", "se1_5", "se2_3", "se2_4", "se3_2", "se3_4", "se4_3", "se4_5", "se4_7"];
/// The stage the class's names are written for (read as each stage's).
const NAMES: &str = "se1_5";

/// `bossFireFlyEff` (gcmn 0x005d26e0): each field's firefly sprite, in
/// its stage's file.
pub const FIREFLY_EFF: [&str; 9] = [
    "",
    "EFF_se1_5fir1",
    "EFF_se2_3fir1",
    "EFF_se2_4fir1",
    "EFF_se3_2fir1",
    "EFF_se3_4fir1",
    "EFF_se4_3fir1",
    "EFF_se4_5fir1",
    "EFF_se4_7fir1",
];

/// The dummies the fireflies start at (`@1153`), the k-th firefly at
/// `k % 18`.
const MARKERS: [&str; 18] = [
    "DMY_marker01",
    "DMY_marker02",
    "DMY_marker03",
    "DMY_marker04",
    "DMY_marker05",
    "DMY_marker06",
    "DMY_marker07",
    "DMY_marker08",
    "DMY_marker09",
    "DMY_marker10",
    "DMY_marker11",
    "DMY_marker12",
    "DMY_marker13",
    "DMY_marker14",
    "DMY_marker15",
    "DMY_marker16",
    "DMY_marker17",
    "DMY_marker18",
];

/// How many fireflies the constructor makes.
pub const FIREFLIES: usize = 54;
static FIREFLIES_DEF: Firefly2Def = Firefly2Def { count: FIREFLIES, markers: &MARKERS, effs: &FIREFLY_EFF };

/// The constructor's `ccSys.bgColor`.
pub const BG_COLOR: [u8; 3] = [0x1e, 0x1e, 0x1e];

/// `SetFog(1500, 12000, 0, 70, 0x1e1e1e)`.
pub const FOG: FogParams = FogParams { near: 1500.0, far: 12000.0, near_rate: 0.0, far_rate: 70.0, colour: [0x1e; 3] };

/// `modelArray`: `EA_MODELTABLEB0` of Infection's tables, its `se1_5`
/// names the stage's (the constructor writes the field's two digits over
/// characters 6 and 8: `MDL_se1_5fl0`); `DrawObj` on objLayer, `DrawFloor`
/// on floorLayer, no fog.
static MODELS: ModelsDef = ModelsDef {
    layers: PassLayers { floor: layer::FLOOR, ..PassLayers::all(layer::OBJ) },
    fog: Fog::None,
    ..ModelsDef::on("EA_MODELTABLEB0", layer::OBJ)
};

/// `bg[0..2]` (+0x1a8) on `bgLayer[0..2]`.
static SKY: SkyDef = SkyDef {
    clumps: &[
        SkyClump::on("CMP_se1_5bac1", layer::BG0),
        SkyClump::on("CMP_se1_5clo1", layer::BG1),
        SkyClump::on("CMP_se1_5clo2", layer::BG2),
    ],
};

/// `DrawBG`'s `v$1597` (a static of the executable): the cloud layer's V
/// takes it, then it steps by 0.0015 (0x3ac49ba6), past 1 less 1.
static SCROLLS: ScrollsDef = ScrollsDef {
    steps: &[0x3ac4_9ba6],
    wrap: Wrap::Less,
    from: 0,
    shown: Shown::Before,
    carry: Some(crate::systems::scrolls::Carry::Statics(crate::town::scroll_statics::ARENA_BG)),
    mats: &[MatScroll::v("MAT_se1_5clo1", 0)],
};

/// `eventareaB0Light`: the distant light, then the 19 omni lights, those
/// the stage has.
const LIGHTS: LightsDef = LightsDef {
    anim: "ANM_se1_5bac1a",
    table: &[
        "LGT_se1_5lig1",
        "LGT_se1_5omn01",
        "LGT_se1_5omn02",
        "LGT_se1_5omn03",
        "LGT_se1_5omn04",
        "LGT_se1_5omn05",
        "LGT_se1_5omn06",
        "LGT_se1_5omn07",
        "LGT_se1_5omn08",
        "LGT_se1_5omn09",
        "LGT_se1_5omn10",
        "LGT_se1_5omn11",
        "LGT_se1_5omn12",
        "LGT_se1_5omn13",
        "LGT_se1_5omn14",
        "LGT_se1_5omn15",
        "LGT_se1_5omn16",
        "LGT_se1_5omn17",
        "LGT_se1_5omn18",
        "LGT_se1_5omn19",
    ],
    skip_missing: true,
};

/// `Draw`: `DrawBG` (the material takes `v`, then `v` steps; the
/// clumps), the type-2 models on objLayer, the type-1 on floorLayer,
/// `modelArray[5]` on its layer, each firefly's `Draw` then its `Move` on
/// effLayer, then the three models bob.
const ORDER: &[Step] = &[
    Step::Run(Kind::Scrolls, Hook::Draw),
    Step::Run(Kind::Sky, Hook::Draw),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Floor)),
    Step::Run(Kind::Reflection, Hook::Draw),
    Step::Run(Kind::Firefly2, Hook::Draw),
    Step::Run(Kind::Bobbing, Hook::Draw),
];

/// The arena's stage: its lights, fog and centre, its models, sky, the
/// reflection and the bobbing.
const STAGE: BlockDef = BlockDef {
    lights: Some(LIGHTS),
    fog: Some(FOG),
    start: Start::marker("DMY_center01"),
    systems: &[SystemDef::Models(&MODELS), SystemDef::Sky(&SKY), SystemDef::Reflection(&()), SystemDef::Bobbing(&())],
    order: ORDER,
    ..BASE_BLOCK
};

/// What every stage keeps across `NextStage`: the scroll, the fireflies.
const ARENA_SYSTEMS: &[SystemDef] = &[SystemDef::Scrolls(&SCROLLS), SystemDef::Firefly2(&FIREFLIES_DEF)];

/// `EVENTAREAB0` for field `n`: Infection's tables, its stage's file, the
/// boss centred on `DMY_center01`; `fieldrand` (the fireflies, the
/// bobbing) the game's one stream, which `GO(1)` does not seed.
const fn arena(fields: &'static [i32], file: &'static str) -> StoryMapDef {
    StoryMapDef {
        class: StoryMapGameClass::Eventareab0,
        fields,
        file,
        tables: Some(Volume::Inf),
        stage: Some(NAMES),
        clear: BG_COLOR,
        blocks: &[STAGE],
        systems: ARENA_SYSTEMS,
        boss_at_start: true,
        asleep: Asleep::Standing,
        ..BASE_STORY_MAP
    }
}

pub static ARENA_1: StoryMapDef = arena(&[1], STAGES[1]);
pub static ARENA_2: StoryMapDef = arena(&[2], STAGES[2]);
pub static ARENA_3: StoryMapDef = arena(&[3], STAGES[3]);
pub static ARENA_4: StoryMapDef = arena(&[4], STAGES[4]);
pub static ARENA_5: StoryMapDef = arena(&[5], STAGES[5]);
pub static ARENA_6: StoryMapDef = arena(&[6], STAGES[6]);
pub static ARENA_7: StoryMapDef = arena(&[7], STAGES[7]);

/// The stage `NextStage` leaves, and the field it does it on.
pub const LAST_STAGE: &str = "se4_8";
pub const LAST_FIELD: i32 = 8;
/// `saveData` byte the constructor reads: set, the arena starts at its
/// last stage (Corbenik's third form, `ccThBoss08`'s test likewise).
pub const LAST_STAGE_BYTE: usize = 0x652f;

/// Field 8's: `NextStage` (QUA gcmn 0x0030aa70) deletes the stage's models
/// and puts its omni lights out of the group (the distant light kept);
/// the last stage stands in its place. Quarantine's constructor starts
/// there once the save says so (QUA gcmn 0x0030a170).
pub static ARENA_8: StoryMapDef = StoryMapDef {
    blocks: &[
        STAGE,
        BlockDef {
            lights: Some(LightsDef { table: &["LGT_se1_5lig1"], ..LIGHTS }),
            fog: Some(FOG),
            systems: &[SystemDef::LastStage(&())],
            order: &[Step::Run(Kind::LastStage, Hook::Draw)],
            ..BASE_BLOCK
        },
    ],
    stage_byte: Some(StageByte { volume: Volume::Qua, byte: LAST_STAGE_BYTE, block: 1 }),
    ..arena(&[LAST_FIELD], STAGES[8])
};

/// `EVENTAREAB0`'s own: `modelArray[5]` on refLayer, or objLayer while
/// `layerSw` (+0x2b4) is set (`SwitchLayer`, gcmn 0x004095b0).
#[derive(Clone, Debug, Default)]
pub struct Reflection {
    pub layer_sw: bool,
}

/// The reflection's row.
const REFLECTION: usize = 5;

impl Reflection {
    pub fn new(_: &'static (), _: &mut BuildCx) -> Result<Reflection> {
        Ok(Reflection::default())
    }
}

impl MapSystem for Reflection {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook == Hook::Draw {
            let layer = if self.layer_sw { layer::OBJ } else { layer::REF };
            cx.put(Kind::Models, Piece::ModelOn(REFLECTION, layer));
        }
    }

    fn signal(&mut self, s: Signal, _: &mut PlaceCx) {
        if s == Signal::SwitchLayer {
            self.layer_sw = !self.layer_sw;
        }
    }
}

/// `EVENTAREAB0`'s own: `modelArray[6..8]` bob, each by `sw[k]` (+0x29c, 0
/// rising): up by `fieldrand(5)` until past 50, down until under -50.
#[derive(Clone, Debug)]
pub struct Bobbing {
    pub sw: [i32; 3],
}

/// The rows that bob, and their turn at +-50.
const BOBBING: usize = 6;
const BOB: F = 0x4248_0000;
const NEG_BOB: F = 0xc248_0000;

impl Bobbing {
    pub fn new(_: &'static (), _: &mut BuildCx) -> Result<Bobbing> {
        Ok(Bobbing { sw: [0, 1, 0] })
    }
}

impl MapSystem for Bobbing {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, others: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let Some(models) = others.get_mut::<StaticModels>() else { return };
        for (k, sw) in self.sw.iter_mut().enumerate() {
            let r = cx.core.field_rand.as_mut().map_or(0, |rng| ee::from_int(rng.below(5) as i32));
            let Some(m) = models.model_mut(BOBBING + k) else { continue };
            let z = &mut m.pos[2];
            if *sw == 0 {
                *z = ee::add(*z, r);
                if !ee::le(*z, BOB) {
                    *sw = 1;
                }
            } else {
                *z = ee::sub(*z, r);
                if ee::lt(*z, NEG_BOB) {
                    *sw = 0;
                }
            }
        }
    }
}

/// `EVENTAREAB0`'s own, field 8's last stage, which Quarantine's
/// constructor loads beside `se4_7` (QUA gcmn 0x00308b90): `se4_8`
/// (+0x2a8), its `ANM_se4_8bac1a` (+0x2ac) stepped and drawn on objLayer,
/// its floor `CMP_se4_8f01` (+0x2b0) on floorLayer while +0x2bc holds
/// (Corbenik's lasers switch it: QUA gcmn 0x0030aa40), and the file's hits
/// for the collision.
#[derive(Clone)]
pub struct LastStage {
    pub file: Rc<SceneFile>,
    pub anm: piney_desktop::anm::Anm,
    pub clump: Vec<u32>,
    pub clump_on: bool,
    hit_models: Vec<HitModel>,
    /// The clip's models and where the last `Draw` put them.
    posed: Vec<(u32, Mat4)>,
}

impl LastStage {
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<LastStage> {
        let file = Rc::new(SceneFile::read(cx.archive, LAST_STAGE)?);
        let mut anm = piney_desktop::anm::Anm::new();
        anm.set(&file, "ANM_se4_8bac1a");
        if file.ccs.find_object("CMP_se4_8f01").is_none() {
            return Err(Error::NotFound(format!("{LAST_STAGE}: CMP_se4_8f01")));
        }
        let clump = clump_models(&file, "CMP_se4_8f01");
        let hit_models = HitModel::read(cx.volume, &file.ccs)?;
        Ok(LastStage { file, anm, clump, clump_on: true, hit_models, posed: Vec::new() })
    }
}

impl MapSystem for LastStage {
    /// The clip forward, drawn; the clump while it is on.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        self.anm.forward();
        self.posed = self.anm.instances().into_iter().map(|i| (i.model, i.world)).collect();
        (0..self.posed.len()).for_each(|k| cx.put(Kind::LastStage, Piece::Item(k)));
        if self.clump_on {
            cx.put(Kind::LastStage, Piece::Clump(0));
        }
    }

    fn draw<'a>(&'a self, piece: Piece, cx: &DrawCx<'a>) -> Option<Request<'a>> {
        let (layer, models, world) = match piece {
            Piece::Item(k) => {
                let (model, world) = self.posed.get(k)?;
                (layer::OBJ, Cow::Borrowed(std::slice::from_ref(model)), *world)
            }
            Piece::Clump(_) => (layer::FLOOR, Cow::Borrowed(&self.clump[..]), Mat4::IDENTITY),
            _ => return None,
        };
        Some(self.models(layer, models, world, cx))
    }

    /// `NextStage`'s clip hit in place of the stage's.
    fn place(&mut self, cx: &mut PlaceCx, _: &mut Others) {
        cx.core.hits.models = self.hit_models.clone();
    }

    fn signal(&mut self, s: Signal, _: &mut PlaceCx) {
        if let Signal::StageClump(on) = s {
            self.clump_on = on;
        }
    }
}

impl LastStage {
    fn models<'a>(&'a self, layer: i16, models: Cow<'a, [u32]>, world: Mat4, cx: &DrawCx) -> Request<'a> {
        let fog = crate::draw::Fogging::None;
        Request::Models { file: &self.file, models, world, layer, rows: cx.rows.clone(), fog, alpha: 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ee::ONE;
    use crate::story_map::StoryArea;
    use crate::story_map::tests::made;
    use crate::systems::fireflies::Fireflies2;
    use crate::systems::scrolls::Scrolls;
    use crate::systems::{Carried, Drawn};
    use crate::town::TownView;

    /// Skeith's arena: the stage's models, the floor's collision, the
    /// start at the centre, the lights.
    #[test]
    fn skeith_arena() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let a = made(&archive, Volume::Inf, &ARENA_1, 1, 0);
        let models = a.systems().get::<StaticModels>().unwrap();
        let name = |o: u32| a.core.file.ccs.object_name(o).unwrap().to_string();
        assert_eq!(a.core.file.stem, "se1_5");
        assert_eq!(models.len(), 9);
        assert_eq!(name(models[5].model), "MDL_se1_5ob0");
        assert_eq!(a.core.hits.models.len(), 1);
        assert_eq!(name(a.core.hits.models[0].object), "HIT_se1_5fl0hit");
        assert_eq!(a.core.lights.lights.len(), 20);
        assert_eq!(a.systems().get::<Fireflies2>().unwrap().fireflies.len(), FIREFLIES);
    }

    fn kinds(a: &mut StoryArea) -> Vec<Kind> {
        let v = TownView { player: [0, 0, 0, ONE], ..TownView::default() };
        a.step(&v, true, None);
        a.frame().iter().filter_map(|d| if let Drawn::Piece(k, _) = d { Some(*k) } else { None }).collect()
    }

    /// Corbenik's arena's last stage: `se4_8`'s clip and floor drawn, its
    /// floor gone while the lasers switch it off.
    #[test]
    fn corbenik_last_stage() {
        let Some(archive) = crate::town::tests::archive_of("quarantine") else { return };
        let mut a = made(&archive, Volume::Qua, &ARENA_8, 1, 0);
        assert!(kinds(&mut a).iter().any(|k| matches!(k, Kind::Firefly2 | Kind::Models)));
        a.change_block(1, 1).unwrap();
        assert_eq!(a.core.lights.lights.len(), 1, "the distant light kept");
        assert!(!a.core.hits.models.is_empty(), "the last stage's hit");
        let k = kinds(&mut a);
        assert!(k.len() > 1 && k.iter().all(|&k| k == Kind::LastStage));
        a.signal(Signal::StageClump(false));
        assert_eq!(kinds(&mut a).len(), k.len() - 1, "the floor off");
    }

    /// `DrawBG`'s `v$1597` is the executable's: an arena takes it as the
    /// last one left it and hands it back.
    #[test]
    fn the_arenas_scroll_is_the_executables() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut a = made(&archive, Volume::Inf, &ARENA_1, 1, 0);
        let mut s = [0; piney_desktop::SCROLL_STATICS];
        s[crate::town::scroll_statics::ARENA_BG] = 0x3e80_0000;
        a.take(&Carried { statics: &s, move_tex: None });
        assert_eq!(a.systems().get::<Scrolls>().unwrap().v[0], 0x3e80_0000);
        a.step(&TownView::default(), true, None);
        let mut out = [0; piney_desktop::SCROLL_STATICS];
        a.put(&mut out, &mut [[0; 2]; piney_desktop::MOVE_TEX_ROWS]);
        assert_eq!(out[crate::town::scroll_statics::ARENA_BG], ee::add(0x3e80_0000, 0x3ac4_9ba6));
    }
}
