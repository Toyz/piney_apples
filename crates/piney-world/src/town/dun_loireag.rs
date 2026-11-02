//! Dun Loireag: `ROOTTOWN02` (gcmn town02.cpp, constructor 0x004240c0, `Draw`
//! 0x004266a0 through the vtable at 0x00375ff0), the second Root Town
//! (`game.town` 1, the server Theta): the sky, cloud layers and sun of its
//! `DrawBG`, 25 `CLOUD`s, the lens flare, and three waters, one drawn through
//! a frame-buffer copy (docs/engine/town02.md). The scroll statics carry
//! across visits (`town::scroll_statics`). The clouds draw from `fieldrand`,
//! the game's one stream, which the town takes from the scene before
//! ([`crate::town::Town::open_with`]).

use piney_data::classes::TownGameClass;
use piney_data::statics::DrawPass;

use crate::draw::OBJ_LAYER;
use crate::ee::{F, ONE, V4};
use crate::map::town::{ArrowPlace, Balloons, MarkY, MinimapDef};
use crate::systems::clouds::{CloudsDef, Made};
use crate::systems::flare::{FlareDef, FlareDraw};
use crate::systems::scrolls::{MatScroll, ScrollsDef, Wrap};
use crate::systems::sky::{At, SkyClump, SkyDef, SkyLayer};
use crate::systems::statics::{ModelsDef, ObjectsDef};
use crate::systems::water::{WaterDef, WaterFile, WaterSpec};
use crate::systems::{DRAW_BG, Hook, Kind, PASSES, Step, SystemDef};
use crate::town::scroll_statics as statics;
use crate::town::{BASE_TOWN, FogParams, TownDef};

/// `DrawBG`'s clumps: the sky, the sun, the two cloud layers.
pub const SKY: &str = "CMP_sr2bac1";
pub const SUN: &str = "CMP_sr2sun1";
pub const CLOUD_SKY: [&str; 2] = ["CMP_sr2clo_1_1", "CMP_sr2clo_1_2"];
/// The sun's place, and `LENSFLARE::Draw`'s sun.
pub const SUN_POINT: &str = "DMY_sr2lig_1point";
/// The light animation and `town02Light` (gcmn 0x005d4e20).
pub const LIGHT_ANIM: &str = "ANM_sr2bac1a";
pub const LIGHTS: [(i32, &str); 6] = [
    (0, "LGT_sr2lig1"),
    (1, "LGT_sr2omn01"),
    (1, "LGT_sr2omn02"),
    (1, "LGT_sr2omn03"),
    (1, "LGT_sr2omn04"),
    (1, "LGT_sr2omn05"),
];
/// The water: `@1318`, `@1319`, and its root `@988` (0, 4200, 0) turned
/// by `@989` (none).
pub const WATER_ANIM: &str = "ANM_sr2wat1a";
pub const WATER_MODEL: &str = "MDL_sr2wat00";
pub const WATER_POS: V4 = [0, 0x4583_4000, 0, ONE];
/// `waterUVModifi2`'s reach: 32000 (0x0042682c).
const WATER_REACH: F = 0x46fa_0000;

/// `SetFog(1500, 10000, 0, 75, 0x00f0c080)` (0x00424278) and the same
/// colour as `ccSys.bgColor`: a pale blue.
pub const FOG: FogParams =
    FogParams { near: 1500.0, far: 10000.0, near_rate: 0.0, far_rate: 75.0, colour: [0x80, 0xc0, 0xf0] };
pub const BG_COLOR: [u8; 3] = [0x80, 0xc0, 0xf0];

/// `DrawBG`'s scrolls a frame: `v$1346` (gcmn gp 0x00378170) 0.01 and
/// 0.02, `v2$1347` (0x00378b5c) 0.001, each back to 0 past 1.
pub const BG_STEP: [F; 3] = [0x3c23_d70a, 0x3ca3_d70a, 0x3a83_126f];
/// The water's scroll a frame, `u$1777` (0x00378b6c): 0.005, back to 0 at 1.
pub const WATER: WaterSpec = WaterSpec { step: 0x3ba3_d70a, wrap: Wrap::AtOrPast };
/// The `CLOUD`s `Draw` makes on its first call (+0x1d8, type 0).
pub const CLOUDS: usize = 25;

/// `WORLD_MAN`'s layers (`GO`, `ccLayer::Init` priorities) the town draws
/// on besides `objLayer`: `bgLayer[0]` (+0x494) and `[1]` (+0x498).
const BG0: i16 = -100;
const BG1: i16 = -90;

/// The materials `DrawBG` offsets: the cloud layers' V by the first two
/// scrolls, one each; in crisis (`town02d`) the clumps over the sky, on
/// `bgLayer[2]`-`[4]`, `MAT_sr2dat1_2` by the third in U, `_3` in U and V.
const MATS: [MatScroll; 4] = [
    MatScroll::v("MAT_sr2clo_1", 0),
    MatScroll::v("MAT_sr2clo_2", 1),
    MatScroll::u("MAT_sr2dat1_2", 2),
    MatScroll::uv("MAT_sr2dat1_3", 2),
];

static MODELS: ModelsDef = ModelsDef::on("RT_MODELTABLE02", OBJ_LAYER);
static OBJECTS: ObjectsDef = ObjectsDef::on("RT_OBJTABLE02", OBJ_LAYER);
static SCROLLS: ScrollsDef = ScrollsDef::statics(&BG_STEP, statics::TOWN02_BG, &MATS);
static SKY_CLUMPS: SkyDef = SkyDef {
    clumps: &[
        SkyClump::on(SKY, BG0),
        SkyClump { name: SUN, layer: SkyLayer::On(BG1), at: At::Dummy(SUN_POINT), crisis: false, fog: false },
        SkyClump { name: CLOUD_SKY[0], layer: SkyLayer::CloudSky, at: At::Origin, crisis: false, fog: false },
        SkyClump { name: CLOUD_SKY[1], layer: SkyLayer::CloudSky, at: At::Origin, crisis: false, fog: false },
        SkyClump::crisis("CMP_sr2dat1_1", -80),
        SkyClump::crisis("CMP_sr2dat1_2", -70),
        SkyClump::crisis("CMP_sr2dat1_3", -60),
    ],
};
/// The town's own water at (0, 4200, 0): 2 and 1 on effLayer, 0 on
/// objLayer and the copy after it.
static WATER_DEF: WaterDef = WaterDef {
    file: WaterFile::Area,
    anim: WATER_ANIM,
    model: WATER_MODEL,
    reach: WATER_REACH,
    root: WATER_POS,
    spec: WATER,
    slot: statics::TOWN02_WATER,
    drawn: &[2, 1, 0],
    layer: crate::draw::EFF_LAYER,
    copy: true,
};
static CLOUDS_DEF: CloudsDef = CloudsDef { count: CLOUDS, kind: 0, made: Made::FirstDraw };
static FLARE: FlareDef = FlareDef { sun: SUN_POINT, draw: FlareDraw::LensFlare };

/// `ROOTTOWN02::DrawMap` (gcmn 0x00425760): Infection's arrow in screen
/// pixels on the font layer; from Mutation on its own layers and the flag
/// race's racers, 12 up; the arrow's point dims a sign within (0, 54) and
/// (10, 49) of it.
static MINIMAP: MinimapDef = MinimapDef {
    iconpos: "RT02ICONPOS",
    tex: "TEX_sr2map1",
    step: (0x3fa3_d70a, 0x3fa5_e354),
    race: true,
    later_only: false,
    offset: (0, 0),
    racer_offset: (0, 0),
    mark_mask: (128, 32),
    mark_x: 0x4140_0000,
    mark_y: MarkY::Up,
    arrow_base: (0x4288_0000, 0x42a0_0000),
    arrow: ArrowPlace::ScreenOffOwnLayers,
    odd_sign: None,
    sign_window: false,
    balloons: Balloons::Never,
    dim: Some([0, 54, 10, 49]),
    hold_fade: false,
};

/// `ROOTTOWN02` (`town02`, `town02d` in crisis; `town_z` for the lens
/// flare's and the clouds' sprites): its `Draw` makes the clouds on its
/// first call, then the water and its copy, `DrawBG`, `DrawObj2`,
/// `DrawObj` and `DrawFloor` (each walking the 20 rows for its type, the
/// first two then the 9 static objects), `DrawMap`, the type-0 rows,
/// `LENSFLARE::Draw(town02, DMY_sr2lig_1point, 0)`, then each cloud's
/// `Move` and `Draw`.
pub static DUN_LOIREAG: TownDef = TownDef {
    no: 1,
    class: TownGameClass::Roottown02,
    name: "Dun Loireag",
    file: "town02",
    crisis_file: Some("town02d"),
    light_anim: LIGHT_ANIM,
    lights: &LIGHTS,
    fog: FOG,
    clear: Some(BG_COLOR),
    start: Some(([0, 0x455a_c000, 0, ONE], crate::START_DIRC)),
    minimap: Some(&MINIMAP),
    systems: &[
        SystemDef::Models(&MODELS),
        SystemDef::Objects(&OBJECTS),
        SystemDef::Scrolls(&SCROLLS),
        SystemDef::Sky(&SKY_CLUMPS),
        SystemDef::Water(&WATER_DEF),
        SystemDef::LensFlare(&FLARE),
        SystemDef::Clouds(&CLOUDS_DEF),
    ],
    order: &[
        Step::Run(Kind::Clouds, Hook::Start),
        Step::Run(Kind::Water, Hook::Draw),
        DRAW_BG,
        PASSES,
        Step::Map,
        Step::Run(Kind::Models, Hook::Pass(DrawPass::Other)),
        Step::Run(Kind::LensFlare, Hook::Draw),
        Step::Run(Kind::Clouds, Hook::Draw),
    ],
    ..BASE_TOWN
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ee;
    use crate::systems::clouds::Clouds;
    use crate::systems::sky::Sky;
    use crate::systems::statics::{StaticModels, StaticObjects};
    use crate::systems::water::Water;
    use crate::systems::{Drawn, Piece};
    use crate::town::{Town, TownView};
    use piney_data::volume::Volume;

    /// The constructor's pieces: 20 model rows (the table's all present),
    /// 9 static objects, the sky, the sun at its dummy, the lights, and the
    /// fog and background colour.
    #[test]
    fn constructor() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let t = Town::open(&archive, Volume::Inf, 1, false).unwrap();
        let s = t.systems();
        assert_eq!(t.def.no, 1);
        let models = s.get::<StaticModels>().unwrap();
        assert_eq!((0..20).filter(|&r| models.model(r).is_some()).count(), 20);
        let objects = s.get::<StaticObjects>().unwrap();
        assert_eq!((0..9).filter(|&r| objects.object(r).is_some()).count(), 9);
        let sky = s.get::<Sky>().unwrap();
        assert!((0..4).all(|k| sky.made(k)) && (4..7).all(|k| !sky.made(k)));
        assert_eq!(t.core.lights.lights.len(), 6);
        assert_eq!(t.def.clear, Some(BG_COLOR));
        assert!(!t.core.hits.models.is_empty());
        assert!(t.core.field_rand.is_some(), "the clouds draw from fieldrand");
    }

    /// The first `Draw` makes the 25 clouds about the player, and every
    /// frame's pieces come in `Draw`'s order: the water, the copy, `DrawBG`,
    /// the rows and objects by pass, the map.
    #[test]
    fn first_draw() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut t = Town::open(&archive, Volume::Inf, 1, false).unwrap();
        let player = [0, ee::k(3500.0), 0, ONE];
        t.step(&TownView { eye: [0, ee::k(2600.0), ee::k(400.0), ONE], player, ..TownView::default() });
        let (w, sky) = (Kind::Water, Kind::Sky);
        let p = |k: Kind, piece: Piece| Drawn::Piece(k, piece);
        assert_eq!(
            &t.frame()[..8],
            &[
                p(w, Piece::Anim(2)),
                p(w, Piece::Anim(1)),
                p(w, Piece::Anim(0)),
                p(w, Piece::Copy),
                p(sky, Piece::Clump(0)),
                p(sky, Piece::Clump(1)),
                p(sky, Piece::Clump(2)),
                p(sky, Piece::Clump(3)),
            ]
        );
        assert!(t.frame().contains(&Drawn::Map));
        assert_eq!(t.systems().get::<Clouds>().unwrap().clouds.len(), CLOUDS);
        let water = t.systems().get::<Water>().unwrap();
        assert_eq!((water.uv, water.u), (0, WATER.step));
    }
}
