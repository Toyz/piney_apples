//! Area 16's story map, Hideous Someone's Giant: `EVENTAREA07` (gcmn
//! 0x00405a00-0x00406cdc), which `WORLD_MAN::GO(1)` makes for
//! `eventAreaNumber` 16. Infection reaches it after the ending (side event
//! 62). Block 0 is the giant's plain under a bobbing sky with 25 `CLOUD`s and
//! a lens flare; block 1 (`se1_7_2`'s names, read through `se1_7_1`'s handle
//! in the game) is one no Infection script reaches. The tables by block are
//! in docs/engine/evarea.md.

use piney_data::classes::StoryMapGameClass;
use piney_data::statics::DrawPass;
use piney_data::volume::Volume;

use crate::story_map::{
    Asleep, BASE_BLOCK, BASE_STORY_MAP, BlockDef, Door, LightsDef, Spread, Start, StartSpread, StoryMapDef, layer,
    move_tex,
};
use crate::systems::bob::BobDef;
use crate::systems::clouds::{CloudsDef, Made};
use crate::systems::flare::{FlareDef, FlareDraw};
use crate::systems::scrolls::{MatScroll, ScrollsDef};
use crate::systems::sky::{At, SkyClump, SkyDef};
use crate::systems::statics::{ModelsDef, ObjectsDef, PassLayers};
use crate::systems::{Hook, Kind, Step, SystemDef};
use crate::town::FogParams;

/// `eventAreaNumber` 16.
pub const AREA: i32 = 16;
/// Block 0's scene file (+0x1a4) and block 1's.
pub const FILE: &str = "se1_7_1";
pub const FILE_2: &str = "se1_7_2";

/// Block 0's `ccSys.bgColor`, 0xdcfae6 (block 1 keeps it).
pub const BG_COLOR: [u8; 3] = [0xe6, 0xfa, 0xdc];

/// The blocks' `SetFog`.
pub const FOG: [FogParams; 2] = [
    FogParams { near: 15000.0, far: 35000.0, near_rate: 0.0, far_rate: 80.0, colour: BG_COLOR },
    FogParams { near: 500.0, far: 5000.0, near_rate: 0.0, far_rate: 80.0, colour: [0, 0, 0] },
];

/// `EA_BGNAME07` (gcmn 0x005d2230): `bg[k]`, each drawn on `bgLayer[k]`
/// at (0, 0, bob), fog off.
static SKY: SkyDef = SkyDef {
    clumps: &[
        SkyClump { at: At::Bob, ..SkyClump::on("CMP_se1_7bg1", layer::BG0) },
        SkyClump { at: At::Bob, ..SkyClump::on("CMP_se1_7cl3", layer::BG1) },
        SkyClump { at: At::Bob, ..SkyClump::on("CMP_se1_7cl1", layer::BG2) },
        SkyClump { at: At::Bob, ..SkyClump::on("CMP_se1_7cl2", layer::BG3) },
    ],
};

/// `DrawBG`'s bob: by `fieldrand(25)`, turning at 350 and 0.
static BOB: BobDef = BobDef { rand: 25, top: 0x43af_0000 };

/// Each block's `modelArray` (`ChangeBlock` calls no `SetPos` but on
/// block 0's rows 0 and 4, at the bob: the rows are postype 0, at the
/// origin). Block 0 draws every pass on objLayer, block 1 its floor on
/// floorLayer.
static MODELS: [ModelsDef; 2] = [
    ModelsDef { ride: &[0, 4], ..ModelsDef::on("EA_MODELTABLE07", layer::OBJ) },
    ModelsDef {
        layers: PassLayers { floor: layer::FLOOR, ..PassLayers::all(layer::OBJ) },
        ..ModelsDef::on("EA_MODELTABLE0702", layer::OBJ)
    },
];

/// Block 0's `anmArray`, its row 1 at the bob; block 1 builds none.
static OBJECTS: ObjectsDef = ObjectsDef { ride: &[1], ..ObjectsDef::on("EA_OBJTABLE07", layer::OBJ) };

/// `EA_moveTex07` (gcmn 0x005d2240): the clouds' materials, V by 0.1 a
/// frame.
static SCROLLS: ScrollsDef = ScrollsDef::move_tex(
    &[0, 0x3dcc_cccd, 0, 0x3dcc_cccd],
    move_tex::EA07,
    &[MatScroll::move_tex("MAT_se1_7clo1", 0), MatScroll::move_tex("MAT_se1_7cl3", 1)],
);

/// `CLOUD`s of type 1, made by `ChangeBlock(0)` about the origin.
static CLOUDS: CloudsDef = CloudsDef { count: 25, kind: 1, made: Made::Build };
/// `LENSFLARE::Draw(se1_7_1, DMY_se1_7lig1point, 0)`.
static FLARE: FlareDef = FlareDef::at("DMY_se1_7lig1point", FlareDraw::LensFlare);

/// Block 0's `Draw` (gcmn 0x00406b80): `DrawBG` (the bob, the scrolls, the
/// clumps), on objLayer `DrawObj2`, `DrawObj`, `DrawFloor` (each its rows,
/// then its objects within their clip of the eye), on effLayer each
/// cloud's `Move` and `Draw`, then the flare.
const BLOCK_0: &[Step] = &[
    Step::Run(Kind::Bob, Hook::Draw),
    Step::Run(Kind::Scrolls, Hook::Draw),
    Step::Run(Kind::Sky, Hook::Draw),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj2)),
    Step::Run(Kind::Objects, Hook::Pass(DrawPass::Obj2)),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Objects, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Floor)),
    Step::Run(Kind::Objects, Hook::Pass(DrawPass::Floor)),
    Step::Run(Kind::Clouds, Hook::Draw),
    Step::Run(Kind::LensFlare, Hook::Draw),
];
/// Block 1's: `DrawObj`, then `DrawFloor`.
const BLOCK_1: &[Step] =
    &[Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj)), Step::Run(Kind::Models, Hook::Pass(DrawPass::Floor))];

static TO_DOOR: Start = Start::marker("DMY_marker02");
static FROM_GATE: Start = Start::marker("DMY_marker01");

/// `EVENTAREA07`: Infection's tables on every volume, `fieldrand` (the
/// clouds and the bob) the game's one stream. `ChangeBlock` (gcmn
/// 0x00405c80): block 0 starts at its door from a dungeon, else at the
/// gate; block 1 at `DMY_marker03`, which is in neither of Infection's
/// files (the start stays). `Enter` (main 0x0019dda0): from block 0 (or
/// -1) the dungeon, else back to block 0. The others stand as main
/// 0x001a1190's (+-200, -150).
pub static GIANT: StoryMapDef = StoryMapDef {
    class: StoryMapGameClass::Eventarea07,
    fields: &[AREA],
    file: FILE,
    tables: Some(Volume::Inf),
    clear: BG_COLOR,
    spread: Spread::About(StartSpread::AHEAD),
    doors: [Door::Dungeon, Door::Block(0)],
    blocks: &[
        BlockDef {
            lights: Some(LightsDef { anim: "ANM_se1_7bg1a", table: &["LGT_se1_7lig1"], skip_missing: false }),
            fog: Some(FOG[0]),
            start: Start::FromDungeon(&TO_DOOR, &FROM_GATE),
            systems: &[
                SystemDef::Models(&MODELS[0]),
                SystemDef::Objects(&OBJECTS),
                SystemDef::Sky(&SKY),
                SystemDef::Clouds(&CLOUDS),
                SystemDef::LensFlare(&FLARE),
            ],
            order: BLOCK_0,
            ..BASE_BLOCK
        },
        BlockDef {
            file: Some(FILE_2),
            lights: Some(LightsDef {
                anim: "ANM_se1_7bg2a",
                table: &["LGT_se1_7lig2", "LGT_omn01", "LGT_omn02"],
                skip_missing: false,
            }),
            fog: Some(FOG[1]),
            start: Start::Marker { name: "DMY_marker03", lanes: [None; 4], or_keep: true },
            systems: &[SystemDef::Models(&MODELS[1])],
            order: BLOCK_1,
            ..BASE_BLOCK
        },
    ],
    systems: &[SystemDef::Scrolls(&SCROLLS), SystemDef::Bob(&BOB)],
    asleep: Asleep::Standing,
    ..BASE_STORY_MAP
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ee;
    use crate::story_map::tests::made;
    use crate::story_map::{Enter, StoryArea};
    use crate::systems::MapView;
    use crate::systems::bob::Bob;
    use crate::systems::clouds::Clouds;
    use crate::systems::statics::{StaticModels, StaticObjects};

    fn counts(g: &StoryArea) -> (usize, usize, usize) {
        let s = g.systems();
        (
            s.get::<StaticModels>().map_or(0, StaticModels::len),
            s.get::<StaticObjects>().map_or(0, StaticObjects::len),
            s.get::<Clouds>().map_or(0, |c| c.clouds.len()),
        )
    }

    /// Both blocks: the tables' pieces, one Hit chunk each, the start
    /// markers, the lights, the clouds of block 0.
    #[test]
    fn the_giants_blocks() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut g = made(&archive, Volume::Inf, &GIANT, 0, 0);
        let name = |g: &StoryArea, o: u32| {
            let f = g.systems().get::<StaticModels>().unwrap().file();
            f.ccs.object_name(o).unwrap().to_string()
        };
        assert_eq!(counts(&g), (8, 2, 25));
        assert_eq!(g.core.hits.models.len(), 1);
        assert_eq!(name(&g, g.core.hits.models[0].object), "HIT_se1_7ob1hit");
        assert_eq!(name(&g, g.core.hits.models[0].parent), "MDL_se1_7ob1_1");
        assert_eq!(g.core.lights.lights.len(), 1);
        let arrive = g.core.start;
        let back = made(&archive, Volume::Inf, &GIANT, 2, 0).core.start;
        assert_ne!(arrive, back, "DMY_marker01 and 02 are different places");
        g.change_block(1, 1).unwrap();
        assert_eq!(counts(&g), (2, 0, 0));
        assert_eq!(g.core.hits.models.len(), 1);
        assert_eq!(name(&g, g.core.hits.models[0].object), "HIT_se1_7ob1_5hit");
        assert_eq!(g.core.lights.lights.len(), 3);
        assert!(matches!(g.enter(1, 1).unwrap(), Enter::Block(0)));
        assert_eq!(g.block, 0);
        assert!(matches!(g.enter(0, 1).unwrap(), Enter::Dungeon));
    }

    /// The bob stays within 0..350 and turns at both ends.
    #[test]
    fn the_background_bobs() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut g = made(&archive, Volume::Inf, &GIANT, 0, 7);
        let v = MapView::default();
        let bob = |g: &StoryArea| g.systems().get::<Bob>().map(|b| (b.z, b.rising)).unwrap();
        let (mut top, mut turns, mut last) = (0.0f32, 0, bob(&g).1);
        for _ in 0..600 {
            g.step(&v, true, None);
            let (z, rising) = bob(&g);
            let z = ee::f(z);
            assert!((0.0..=350.0).contains(&z), "{z}");
            top = top.max(z);
            if rising != last {
                turns += 1;
                last = rising;
            }
        }
        assert!(top == 350.0 && turns >= 2, "top {top}, turns {turns}");
    }
}
