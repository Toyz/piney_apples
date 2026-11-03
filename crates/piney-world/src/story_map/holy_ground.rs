//! Story area 15, Hidden Forbidden Holy Ground: `EVENTAREA02` (gcmn
//! area02.cpp, constructor 0x00401e00, `ChangeBlock` 0x00402300, `Draw`
//! 0x004033e0), the hand-built story map `WORLD_MAN::GO(1)` makes in place of
//! a generated field. Its scene `se1_2` holds two blocks: 0 the holy ground
//! outside, 1 the church, swapped by `WORLD_MAN::Enter` on the door. Each
//! block's floor model carries its whole collision; the map is not a torus
//! and has no height map (docs/engine/evarea.md).

use piney_data::classes::StoryMapGameClass;
use piney_data::statics::DrawPass;
use piney_data::volume::Volume;

use crate::ee::{F, V4};
use crate::story_map::{
    Asleep, BASE_BLOCK, BASE_STORY_MAP, BlockDef, Door, LightsDef, Start, StoryMapDef, layer, move_tex,
};
use crate::systems::anim::AnimDef;
use crate::systems::flare::{FlareDef, FlareDraw};
use crate::systems::scrolls::{MatScroll, ScrollsDef};
use crate::systems::sky::{SkyClump, SkyDef};
use crate::systems::statics::{ModelsDef, ObjectsDef, PassLayers};
use crate::systems::{Hook, Kind, Step, SystemDef};
use crate::town::FogParams;

/// The story area `EVENTAREA02` is, and its scene file.
pub const AREA: i32 = 15;
pub const FILE: &str = "se1_2";

/// The constructor's `ccSys.bgColor` (+0x18): the frame's clear colour.
pub const BG_COLOR: [u8; 3] = [0x1e, 0x1e, 0x1e];

/// The blocks' `SetFog(near, far, 0, 90, 0x323232)`.
pub const FOG: [FogParams; 2] = [
    FogParams { near: 250.0, far: 7800.0, near_rate: 0.0, far_rate: 90.0, colour: [0x32, 0x32, 0x32] },
    FogParams { near: 10000.0, far: 15000.0, near_rate: 0.0, far_rate: 90.0, colour: [0x32, 0x32, 0x32] },
];

/// Each block's `modelArray`, the whole of its table (`modelNum` 4, 7):
/// `DrawObj2` and `DrawObj` put theirs on obj2Layer, `DrawFloor` and the
/// type-0 rows on objLayer; `DrawObj2`'s third (row 2) by
/// `DrawWithOutFog`.
const MODEL_LAYERS: PassLayers =
    PassLayers { other: layer::OBJ, floor: layer::OBJ, obj: layer::OBJ2, obj2: layer::OBJ2 };
static MODELS: [ModelsDef; 2] = [
    ModelsDef { without_fog: &[2], ..models("EA_MODELTABLE02") },
    ModelsDef { without_fog: &[2], ..models("EA_MODELTABLE0202") },
];

const fn models(table: &'static str) -> ModelsDef {
    ModelsDef { layers: MODEL_LAYERS, ..ModelsDef::on(table, layer::OBJ) }
}

/// Each block's `anmArray`: `anmNum` 0 (the first table's one placeholder
/// row is never built) and 1 (`ANM_se1_2la1a`), on obj2Layer.
static OBJECTS: [ObjectsDef; 2] = [
    ObjectsDef { anm_num: Some(0), ..ObjectsDef::on("EA_OBJTABLE02", layer::OBJ2) },
    ObjectsDef { anm_num: Some(1), ..ObjectsDef::on("EA_OBJTABLE0202", layer::OBJ2) },
];

/// `DrawBG`: block 0's `bg[3]` on bgLayer[3], `[2]` on bgLayer[2], `[1]`
/// and `[0]` on effLayer; block 1's `bg[0]` on bgLayer[0].
static SKY: [SkyDef; 2] = [
    SkyDef {
        clumps: &[
            SkyClump::on("CMP_se1_2bg1_1", layer::BG3),
            SkyClump::on("CMP_se1_2wa1_1", layer::BG2),
            SkyClump::on("CMP_se1_2cl1_1", layer::EFF),
            SkyClump::on("CMP_se1_2cl2_1", layer::EFF),
        ],
    },
    SkyDef { clumps: &[SkyClump::on("CMP_se1_2bg1_2", layer::BG0)] },
];

/// `EA_moveTex02` (gcmn 0x005d1d50): block 0's two scrolling materials,
/// `MOVETEX {u, v, u_step, v_step, matname}`, V by 0.01 a frame. The
/// table is gcmn's own data: its scroll outlives the map.
static SCROLLS: ScrollsDef = ScrollsDef::move_tex(
    &[0, 0x3c23_d70a, 0, 0x3c23_d70a],
    move_tex::EA02,
    &[MatScroll::move_tex("MAT_se1_2cl1", 0), MatScroll::move_tex("MAT_se1_2wa1", 1)],
);

/// `revAnm`: `SetMatrix_PosRotZYX(@1365, @1364)` each frame, (0, 5400, 0)
/// turned by (0, 3.14, -3.14): the church's lanterns again, upside down
/// under its floor.
pub const REV_POS: V4 = [0, 0x45a8_c000, 0, 0];
pub const REV_ROT: [F; 3] = [0, 0x4048_f5c3, 0xc048_f5c3];
static REV: AnimDef = AnimDef { anim: "ANM_se1_2la1a", file: None, pos: REV_POS, rot: REV_ROT, layer: layer::OBJ2 };

/// `DrawLensFlare` (gcmn 0x00402dc0) at `DMY_se1_2lig1point`: in the
/// church, while no puppet show runs.
static FLARE: FlareDef = FlareDef::at("DMY_se1_2lig1point", FlareDraw::Own);

/// `Draw`: `DrawBG`, `DrawObj2` (its rows), `DrawObj` (its rows, then its
/// objects within their clip of the eye), `revAnm`, `DrawFloor`, the
/// type-0 rows; in the church the flare.
const BLOCK_0: &[Step] = &[
    Step::Run(Kind::Scrolls, Hook::Draw),
    Step::Run(Kind::Sky, Hook::Draw),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj2)),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Objects, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Floor)),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Other)),
];
const BLOCK_1: &[Step] = &[
    Step::Run(Kind::Sky, Hook::Draw),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj2)),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Objects, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Anim, Hook::Draw),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Floor)),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Other)),
    Step::Run(Kind::LensFlare, Hook::Draw),
];

/// `EVENTAREA02`: Infection's tables on every volume; its constructor makes
/// block 0, then `SetStartPos(DMY_marker01)`; `ChangeBlock` (gcmn
/// 0x00402300) makes 0 the holy ground, any other the church, each with
/// its light animation's lights (`eventarea02Light`, `eventarea0202Light`)
/// and start. The door (main 0x0019dec4) goes into the church from block 0
/// (or -1), else out. `WORLD_MAN::Quit` spares it for a field.
pub static HOLY_GROUND: StoryMapDef = StoryMapDef {
    class: StoryMapGameClass::Eventarea02,
    fields: &[AREA],
    file: FILE,
    tables: Some(Volume::Inf),
    clear: BG_COLOR,
    doors: [Door::Block(1), Door::Block(0)],
    blocks: &[
        BlockDef {
            lights: Some(LightsDef { anim: "ANM_se1_2bac1a", table: &["LGT_se1_2lig1"], skip_missing: false }),
            fog: Some(FOG[0]),
            start: Start::marker("DMY_marker02"),
            systems: &[SystemDef::Models(&MODELS[0]), SystemDef::Objects(&OBJECTS[0]), SystemDef::Sky(&SKY[0])],
            order: BLOCK_0,
            ..BASE_BLOCK
        },
        BlockDef {
            lights: Some(LightsDef {
                anim: "ANM_se1_2bac2a",
                table: &["LGT_se1_2lig2", "LGT_omni01", "LGT_omni02", "LGT_omni03", "LGT_omni13"],
                skip_missing: false,
            }),
            fog: Some(FOG[1]),
            start: Start::marker("DMY_marker01_2"),
            systems: &[
                SystemDef::Models(&MODELS[1]),
                SystemDef::Objects(&OBJECTS[1]),
                SystemDef::Sky(&SKY[1]),
                SystemDef::Anim(&REV),
                SystemDef::LensFlare(&FLARE),
            ],
            order: BLOCK_1,
            ..BASE_BLOCK
        },
    ],
    start: Some(Start::marker("DMY_marker01")),
    systems: &[SystemDef::Scrolls(&SCROLLS)],
    kept: true,
    asleep: Asleep::Standing,
    ..BASE_STORY_MAP
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ee::{self, ONE};
    use crate::story_map::tests::made;
    use crate::systems::anim::Anim;
    use crate::systems::statics::{StaticModels, StaticObjects};

    /// The constructor's block 0 and the church: their models, the floor's
    /// collision, the start places and the door's way back and forth.
    #[test]
    fn blocks_and_the_door() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut a = made(&archive, Volume::Inf, &HOLY_GROUND, 1, 0);
        let name = |a: &crate::story_map::StoryArea, o: u32| a.core.file.ccs.object_name(o).unwrap().to_string();
        let counts = |a: &crate::story_map::StoryArea| {
            let s = a.systems();
            (s.get::<StaticModels>().unwrap().len(), s.get::<StaticObjects>().unwrap().len(), s.get::<Anim>().is_some())
        };
        assert_eq!(counts(&a), (4, 0, false));
        assert_eq!(a.core.hits.models.len(), 1);
        assert_eq!(name(&a, a.core.hits.models[0].object), "HIT_se1_2fl1_1hit");
        // DMY_marker01: (0, -3600, 0), turned -179.99998 degrees.
        assert_eq!(a.core.start[..3], [0, ee::k(-3600.0), 0]);
        assert!((ee::f(a.core.start[3]) + std::f32::consts::PI).abs() < 1e-5);
        assert_eq!(a.core.lights.lights.len(), 1);
        // Into the church and out again.
        assert!(matches!(a.enter(-1, 1).unwrap(), crate::story_map::Enter::Block(1)));
        assert_eq!(counts(&a), (7, 1, true));
        assert_eq!(name(&a, a.core.hits.models[0].object), "HIT_se1_2fl1_2hit");
        assert_eq!(a.core.start[..3], [0, ee::k(900.0), 0]);
        assert_eq!(a.core.lights.lights.len(), 5);
        assert!(matches!(a.enter(1, 1).unwrap(), crate::story_map::Enter::Block(0)));
        assert_eq!(a.core.start[..3], [0, ee::k(2700.0), ee::k(200.0)]);
        // Standing on the door's floor: its ground has the Enter bit.
        let z = a.core.hits.land([ee::k(95.0), ee::k(3700.0), ee::k(300.0), ONE], crate::hit::LAND_MASK);
        assert!((ee::f(z) - 200.0).abs() < 1e-3);
        assert_ne!(a.core.hits.nearest.att & 0x8_0000, 0);
    }
}
