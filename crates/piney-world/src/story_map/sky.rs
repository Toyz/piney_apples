//! The sky maps `WORLD_MAN::GO(1)` makes by `eventAreaNumber` on every
//! volume: `EVENTAREA04` (field 66, Bitter Fantasy Mirror World),
//! `EVENTAREA05` (field 67, its arena) and `EVENTAREA06` (field 91,
//! Pulsating Sea of Cloud Whale); MUT gcmn 0x00417050, 0x00417e10,
//! 0x00419e30 (docs/engine/evarea.md). Each is its tables' static models and
//! objects (all type 0), the `EA_BGNAME` clumps, one `EA_moveTex` row, 25
//! `CLOUD`s made at the first `Draw`, and for 04 and 05 a `LENSFLARE`.

use piney_data::classes::StoryMapGameClass;
use piney_data::statics::DrawPass;

use crate::ee::F;
use crate::story_map::{
    BASE_BLOCK, BASE_STORY_MAP, BlockDef, Door, LightsDef, Spread, Start, StartSpread, StoryMapDef, layer, move_tex,
};
use crate::systems::clouds::{CloudsDef, Made};
use crate::systems::flare::{FlareDef, FlareDraw};
use crate::systems::scrolls::{MatScroll, ScrollsDef};
use crate::systems::sky::{SkyClump, SkyDef};
use crate::systems::statics::{ModelsDef, ObjectsDef};
use crate::systems::{Hook, Kind, Step, SystemDef};
use crate::town::FogParams;

/// `SetFog(9000, 14000, 0, 70, 0xc8b9bd)` and `ccSys.bgColor` (04, 05).
const SAND: [u8; 3] = [0xbd, 0xb9, 0xc8];
const SAND_FOG: FogParams = FogParams { near: 9000.0, far: 14000.0, near_rate: 0.0, far_rate: 70.0, colour: SAND };
/// `SetFog(15000, 35000, 0, 80, 0xa77a6d)` and `ccSys.bgColor` (06).
const SEA: [u8; 3] = [0x6d, 0x7a, 0xa7];
const SEA_FOG: FogParams = FogParams { near: 15000.0, far: 35000.0, near_rate: 0.0, far_rate: 80.0, colour: SEA };

/// `eventarea04Light`; `eventarea05Light`'s first row, the one row its
/// loop reads (`s0 == 0` once); `eventarea06Light`.
const SE2_6_LIGHTS: LightsDef = LightsDef { anim: "ANM_se2_6a", table: &["LGT_se2_6lig1"], skip_missing: false };
const SE3_5_LIGHTS: LightsDef = LightsDef { anim: "ANM_se3_5_1a", table: &["LGT_se3_5lig1"], skip_missing: false };

/// `CLOUD`s (+0x1f0, 06's +0x204), made by the first `Draw` about the
/// player: type 0, the whale's type 3.
static CLOUDS: CloudsDef = CloudsDef { count: 25, kind: 0, made: Made::FirstDraw };
static WHALE_CLOUDS: CloudsDef = CloudsDef { count: 25, kind: 3, made: Made::FirstDraw };
/// `LENSFLARE::Draw(ccs, "DMY_dummy5", 0)` (04 and 05).
static FLARE: FlareDef = FlareDef { sun: "DMY_dummy5", draw: FlareDraw::LensFlare };

/// `bg[k]` on `bgLayer[k]` (`EA_BGNAME`), its fog switched on or not.
const fn bg(name: &'static str, k: i16, fog: bool) -> SkyClump {
    SkyClump { fog, ..SkyClump::on(name, layer::bg(k)) }
}

/// `EA_moveTex`'s one row: (u_step, v_step) and its material.
const fn scrolls(steps: &'static [F; 2], row: usize, mats: &'static [MatScroll; 1]) -> ScrollsDef {
    ScrollsDef::move_tex(steps, row, mats)
}

/// `Draw`: the clouds made on the first call, `MoveTexture`, `DrawBG`'s
/// clumps, the type-0 models, then the type-0 objects within their clip;
/// on effLayer the flare and each cloud's `Move` and `Draw`.
const ORDER: &[Step] = &[
    Step::Run(Kind::Clouds, Hook::Start),
    Step::Run(Kind::Scrolls, Hook::Draw),
    Step::Run(Kind::Sky, Hook::Draw),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Other)),
    Step::Run(Kind::Objects, Hook::Pass(DrawPass::Other)),
    Step::Run(Kind::LensFlare, Hook::Draw),
    Step::Run(Kind::Clouds, Hook::Draw),
];

static MIRROR_MODELS: ModelsDef = ModelsDef::on("EA_MODELTABLE04", layer::OBJ);
static MIRROR_OBJECTS: ObjectsDef = ObjectsDef::on("EA_OBJTABLE04", layer::OBJ);
/// bg[2..4] have their fog switched back on.
static MIRROR_SKY: SkyDef = SkyDef {
    clumps: &[
        bg("CMP_se2_6bg2", 0, false),
        bg("CMP_se2_6bg1", 1, false),
        bg("CMP_se2_6cl3", 2, true),
        bg("CMP_se2_6cl2", 3, true),
        bg("CMP_se2_6cl1", 4, true),
        bg("CMP_se2_6su1", 5, false),
    ],
};
static MIRROR_SCROLLS: ScrollsDef =
    scrolls(&[0, 0x3ba3_d70a], move_tex::EA04, &[MatScroll::move_tex("MAT_se1_7clo1", 0)]);

/// The constructor's `SetStartPos`, writing the whole of the dummy's
/// +0x10: from a dungeon `DMY_dummy2` at (-8000, 5800, 920) facing 0,
/// else `DMY_dummy1` at (4600, -4700, 910) facing pi.
static MIRROR_DOOR: Start = Start::Marker {
    name: "DMY_dummy2",
    lanes: [Some(0xc5fa_0000), Some(0x45b5_4000), Some(0x4466_0000), Some(0)],
    or_keep: false,
};
static MIRROR_GATE: Start = Start::Marker {
    name: "DMY_dummy1",
    lanes: [Some(0x458f_c000), Some(0xc592_e000), Some(0x4463_8000), Some(0x4049_0fdb)],
    or_keep: false,
};

/// `EVENTAREA04`, field 66: the oasis whose door is the dungeon. Its
/// others stand as field 16's (+-200, -150); `Enter` (MUT main
/// 0x001b2b80) takes every door to its dungeon (`ChangeArea(2, 0)`).
pub static MIRROR: StoryMapDef = StoryMapDef {
    class: StoryMapGameClass::Eventarea04,
    fields: &[66],
    file: "se2_6",
    clear: SAND,
    spread: Spread::About(StartSpread::AHEAD),
    doors: [Door::Dungeon; 2],
    blocks: &[BlockDef {
        lights: Some(SE2_6_LIGHTS),
        fog: Some(SAND_FOG),
        start: Start::FromDungeon(&MIRROR_DOOR, &MIRROR_GATE),
        systems: &[
            SystemDef::Models(&MIRROR_MODELS),
            SystemDef::Objects(&MIRROR_OBJECTS),
            SystemDef::Sky(&MIRROR_SKY),
            SystemDef::Scrolls(&MIRROR_SCROLLS),
            SystemDef::Clouds(&CLOUDS),
            SystemDef::LensFlare(&FLARE),
        ],
        order: ORDER,
        ..BASE_BLOCK
    }],
    ..BASE_STORY_MAP
};

static ARENA_MODELS: ModelsDef = ModelsDef::on("EA_MODELTABLE05", layer::FLOOR);
static ARENA_OBJECTS: ObjectsDef = ObjectsDef::on("EA_OBJTABLE05", layer::OBJ);
/// The first five of `EA_BGNAME05`'s six (`CMP_se2_6cl3` is made, never
/// drawn).
static ARENA_SKY: SkyDef = SkyDef {
    clumps: &[
        bg("CMP_se2_6bg1", 0, false),
        bg("CMP_se2_6bg2", 1, false),
        bg("CMP_se2_6bg05", 2, false),
        bg("CMP_se2_6cl1", 3, false),
        bg("CMP_se2_6cl2", 4, false),
    ],
};
static ARENA_SCROLLS: ScrollsDef =
    scrolls(&[0, 0x3c23_d70a], move_tex::EA05, &[MatScroll::move_tex("MAT_se1_7clo1", 0)]);

/// `EVENTAREA05`, field 67: the arena the mirror's dungeon's last door
/// leads to. `DMY_dummy3`'s x and turn, at y 9000 and z 800.
pub static MIRROR_ARENA: StoryMapDef = StoryMapDef {
    class: StoryMapGameClass::Eventarea05,
    fields: &[67],
    file: "se2_6",
    clear: SAND,
    blocks: &[BlockDef {
        lights: Some(SE2_6_LIGHTS),
        fog: Some(SAND_FOG),
        start: Start::Marker {
            name: "DMY_dummy3",
            lanes: [None, Some(0x460c_a000), Some(0x4448_0000), None],
            or_keep: false,
        },
        systems: &[
            SystemDef::Models(&ARENA_MODELS),
            SystemDef::Objects(&ARENA_OBJECTS),
            SystemDef::Sky(&ARENA_SKY),
            SystemDef::Scrolls(&ARENA_SCROLLS),
            SystemDef::Clouds(&CLOUDS),
            SystemDef::LensFlare(&FLARE),
        ],
        order: ORDER,
        ..BASE_BLOCK
    }],
    ..BASE_STORY_MAP
};

static WHALE_MODELS: ModelsDef = ModelsDef::on("EA_MODELTABLE06", layer::FLOOR);
static WHALE_OBJECTS: ObjectsDef = ObjectsDef::on("EA_OBJTABLE06", layer::OBJ);
static WHALE_SKY: SkyDef = SkyDef {
    clumps: &[
        bg("CMP_se3_5bac1", 0, false),
        bg("CMP_se3_5clo1_1", 1, false),
        bg("CMP_se3_5clo1_2", 2, false),
        bg("CMP_se3_5clo2", 3, false),
        bg("CMP_se3_5su1", 4, false),
    ],
};
static WHALE_SCROLLS: ScrollsDef =
    scrolls(&[0xbcf5_c28f, 0x3b23_d70a], move_tex::EA06, &[MatScroll::move_tex("MAT_se3_5clo2", 0)]);
static WHALE_DOOR: Start = Start::marker("DMY_marker02");
static WHALE_GATE: Start = Start::marker("DMY_marker01");

/// `EVENTAREA06`, field 91. `Enter` takes block 0 or -1 to its dungeon;
/// its other branch calls `EVENTAREA07::ChangeBlock(0)` on this map, but
/// no door or script gives field 91 another block (its dungeon's way out
/// is `ChangeArea(1, 91)`, block -1), so it stays.
pub static WHALE: StoryMapDef = StoryMapDef {
    class: StoryMapGameClass::Eventarea06,
    fields: &[91],
    file: "se3_5_1",
    clear: SEA,
    doors: [Door::Dungeon, Door::Stay],
    blocks: &[BlockDef {
        lights: Some(SE3_5_LIGHTS),
        fog: Some(SEA_FOG),
        start: Start::FromDungeon(&WHALE_DOOR, &WHALE_GATE),
        systems: &[
            SystemDef::Models(&WHALE_MODELS),
            SystemDef::Objects(&WHALE_OBJECTS),
            SystemDef::Sky(&WHALE_SKY),
            SystemDef::Scrolls(&WHALE_SCROLLS),
            SystemDef::Clouds(&WHALE_CLOUDS),
        ],
        order: ORDER,
        ..BASE_BLOCK
    }],
    ..BASE_STORY_MAP
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ee;
    use crate::story_map::tests::made;
    use crate::systems::clouds::Clouds;
    use crate::systems::statics::{StaticModels, StaticObjects};
    use crate::town::TownView;
    use piney_data::volume::Volume;

    /// Each class on each volume: every row of its tables in the file, one
    /// light, the collision of its own models only, the scrolled material,
    /// the start, and a first draw with its clouds.
    #[test]
    fn the_sky_maps() {
        for (disc, volume) in [
            ("infection", Volume::Inf),
            ("mutation", Volume::Mut),
            ("outbreak", Volume::Out),
            ("quarantine", Volume::Qua),
        ] {
            let Some(archive) = crate::town::tests::archive_of(disc) else { continue };
            for (def, models, objects) in [(&MIRROR, 11, 1), (&MIRROR_ARENA, 9, 1), (&WHALE, 3, 21)] {
                let mut m = made(&archive, volume, def, 1, 0);
                let s = m.systems();
                assert_eq!(s.get::<StaticModels>().unwrap().len(), models, "{:?}", def.class);
                assert_eq!(s.get::<StaticObjects>().unwrap().len(), objects, "{:?}", def.class);
                assert_eq!(m.core.lights.lights.len(), 1, "{:?}", def.class);
                assert!(!m.core.hits.models.is_empty(), "{:?}: no ground", def.class);
                let own: Vec<u32> = s.get::<StaticModels>().unwrap().iter().map(|p| p.model).collect();
                assert!(m.core.hits.models.iter().all(|h| own.contains(&h.parent)), "{:?}", def.class);
                assert_eq!(m.rows().len(), 1, "{:?}", def.class);
                let mut layers = piney_desktop::layers::Layers::default();
                let v = TownView { player: m.core.start, ..TownView::default() };
                let mut cc = piney_battle::rand::Genrand::default();
                // A draw while the tasks sleep, before any step, puts the
                // clouds back as they were (none yet).
                m.draw(&mut layers, glam::Mat4::IDENTITY, &v, false, &mut cc);
                assert!(m.systems().get::<Clouds>().unwrap().clouds.is_empty());
                m.draw(&mut layers, glam::Mat4::IDENTITY, &v, true, &mut cc);
                assert_eq!(m.systems().get::<Clouds>().unwrap().clouds.len(), 25);
                assert!(!layers.flatten().is_empty(), "{:?}: nothing drawn", def.class);
            }
        }
    }

    /// The starts: 04's by `game.areaPrev`, 05's at y 9000 and z 800.
    #[test]
    fn the_starts() {
        let Some(archive) = crate::town::tests::archive_of("mutation") else { return };
        let gate = made(&archive, Volume::Mut, &MIRROR, 0, 0).core.start;
        let back = made(&archive, Volume::Mut, &MIRROR, 2, 0).core.start;
        assert_eq!(gate, [ee::k(4600.0), ee::k(-4700.0), ee::k(910.0), 0x4049_0fdb]);
        assert_eq!(back, [ee::k(-8000.0), ee::k(5800.0), ee::k(920.0), 0]);
        let arena = made(&archive, Volume::Mut, &MIRROR_ARENA, 2, 0).core.start;
        assert_eq!((arena[1], arena[2]), (ee::k(9000.0), ee::k(800.0)));
    }
}
