//! Fort Ouph: `ROOTTOWN04` (OUT gcmn town04.cpp, constructor 0x00439c50,
//! `Draw` 0x0043c430 through the vtable at 0x003842d0), the fourth Root Town
//! (`game.town` 3, the server Sigma), new in Outbreak (Quarantine's is the
//! same). Dun Loireag's clouds and lens flare, but no water, no type-0 rows,
//! and cloud layers on `bgLayer[5]` scrolled in U (docs/engine/root-towns.md).

use piney_data::classes::TownGameClass;

use crate::draw::OBJ_LAYER;
use crate::ee::{F, ONE};
use crate::map::town::{ArrowPlace, Balloons, MarkY, MinimapDef};
use crate::systems::clouds::CloudsDef;
use crate::systems::flare::FlareDef;
use crate::systems::scrolls::ScrollsDef;
use crate::systems::sky::{At, Axes, MatScroll, SkyClump, SkyDef, SkyLayer};
use crate::systems::statics::{ModelsDef, ObjectsDef};
use crate::systems::{DRAW_BG, Hook, Kind, PASSES, Step, SystemDef};
use crate::town::scroll_statics as statics;
use crate::town::{BASE_TOWN, FogParams, TownDef};

/// `DrawBG`'s clumps: the sky, the sun, the three cloud layers.
pub const SKY: &str = "CMP_sr4bac1";
pub const SUN: &str = "CMP_sr4sun1";
pub const CLOUD_SKY: [&str; 3] = ["CMP_sr4clo1_1", "CMP_sr4clo1_2", "CMP_sr4clo2"];
/// The sun's place, and `LENSFLARE::Draw`'s sun.
pub const SUN_POINT: &str = "DMY_sr4lig1point";
/// The light animation and `town04Light` (OUT gcmn 0x00601210).
pub const LIGHT_ANIM: &str = "ANM_sr4bac1a";
pub const LIGHTS: [(i32, &str); 6] = [
    (0, "LGT_sr4lig1"),
    (1, "LGT_sr4omn1"),
    (1, "LGT_sr4omn2"),
    (1, "LGT_sr4omn3"),
    (1, "LGT_sr4omn4"),
    (1, "LGT_sr4omn5"),
];

/// `SetFog(1500, 10000, 0, 30, 0x00c8f0fb)` (0x00439ed8) and the same
/// colour as `ccSys.bgColor`: a pale warm haze.
pub const FOG: FogParams =
    FogParams { near: 1500.0, far: 10000.0, near_rate: 0.0, far_rate: 30.0, colour: [0xfb, 0xf0, 0xc8] };
pub const BG_COLOR: [u8; 3] = [0xfb, 0xf0, 0xc8];

/// `DrawBG`'s scrolls a frame (OUT gp 0x00386ed0, 0x00386ed4): 0.01 for
/// the clouds, 0.002 for the crisis sky.
pub const BG_STEP: [F; 2] = [0x3c23_d70a, 0x3b03_126f];
/// The `CLOUD`s `Draw` makes on its first call (+0x1c4, type 0).
pub const CLOUDS: usize = 25;

/// `WORLD_MAN`'s layers: `bgLayer[0]` (+0x494), `[1]` (+0x498) and `[5]`
/// (+0x4a8).
const BG0: i16 = -100;
const BG1: i16 = -90;
const BG5: i16 = -50;

/// The cloud layers' materials, scrolled in U by the first scroll.
const CLOUD_MATS: [MatScroll; 2] =
    [MatScroll::new("MAT_sr4clo1", 0, Axes::U), MatScroll::new("MAT_sr4clo2", 0, Axes::U)];
/// In crisis (`town04d`) the clumps over the sky, on `bgLayer[2]`-`[4]`:
/// `MAT_sr4dat1_2` by the second scroll in U, `_3` in U and V.
const CRISIS_MATS: [MatScroll; 2] =
    [MatScroll::new("MAT_sr4dat1_2", 1, Axes::U), MatScroll::new("MAT_sr4dat1_3", 1, Axes::UV)];

const fn cloud_sky(name: &'static str) -> SkyClump {
    SkyClump { name, layer: SkyLayer::On(BG5), at: At::Origin, mats: &CLOUD_MATS, crisis: false }
}

static MODELS: ModelsDef = ModelsDef { table: "RT_MODELTABLE04", layer: OBJ_LAYER };
static OBJECTS: ObjectsDef = ObjectsDef { table: "RT_OBJTABLE04" };
static SCROLLS: ScrollsDef = ScrollsDef { steps: &BG_STEP, slot: statics::TOWN04_BG };
static SKY_CLUMPS: SkyDef = SkyDef {
    clumps: &[
        SkyClump::on(SKY, BG0),
        SkyClump { name: SUN, layer: SkyLayer::On(BG1), at: At::Dummy(SUN_POINT), mats: &[], crisis: false },
        SkyClump::crisis("CMP_sr4dat1_1", -80, &CRISIS_MATS),
        SkyClump::crisis("CMP_sr4dat1_2", -70, &CRISIS_MATS),
        SkyClump::crisis("CMP_sr4dat1_3", -60, &CRISIS_MATS),
        cloud_sky(CLOUD_SKY[0]),
        cloud_sky(CLOUD_SKY[1]),
        cloud_sky(CLOUD_SKY[2]),
    ],
};
static CLOUDS_DEF: CloudsDef = CloudsDef { count: CLOUDS, kind: 0 };
static FLARE: FlareDef = FlareDef { sun: SUN_POINT };

/// `ROOTTOWN04::DrawMap`: its own layers, the map 16 down for the player
/// (0.8 a step, OUT gcmn 0x00439e54), racers 12 down, the arrow at (70,
/// 80), a sign dimmed within (8, 46) and (10, 45), the signs' fade held
/// while the race runs.
static MINIMAP: MinimapDef = MinimapDef {
    iconpos: "RT04ICONPOS",
    tex: "TEX_sr4map1",
    step: (0x3f4c_cccd, 0x3f4c_cccd),
    race: true,
    later_only: true,
    offset: (0, 16),
    racer_offset: (0, 0),
    mark_mask: (128, 32),
    mark_x: 0x4140_0000,
    mark_y: MarkY::Down,
    arrow_base: (0x428c_0000, 0x42a0_0000),
    arrow: ArrowPlace::Base,
    odd_sign: None,
    sign_window: false,
    balloons: Balloons::OutsideRace,
    dim: Some([8, 46, 10, 45]),
    hold_fade: true,
};

/// `ROOTTOWN04` (`town04`, `town04d` in crisis; `town_z`): its `Draw` makes
/// the clouds on its first call, then `DrawBG`, `DrawObj2`, `DrawObj` and
/// `DrawFloor` (each walking the 16 rows for its type, the first two then
/// the static object), `DrawMap`, `LENSFLARE::Draw(town04,
/// DMY_sr4lig1point, 0)` and each cloud's `Move` and `Draw`.
pub static FORT_OUPH: TownDef = TownDef {
    no: 3,
    class: TownGameClass::Roottown04,
    name: "Fort Ouph",
    file: "town04",
    crisis_file: Some("town04d"),
    light_anim: LIGHT_ANIM,
    lights: &LIGHTS,
    fog: FOG,
    clear: Some(BG_COLOR),
    start: Some(([0, 0x45bb_8000, 0, ONE], crate::START_DIRC)),
    minimap: Some(&MINIMAP),
    systems: &[
        SystemDef::Models(&MODELS),
        SystemDef::Objects(&OBJECTS),
        SystemDef::Scrolls(&SCROLLS),
        SystemDef::Sky(&SKY_CLUMPS),
        SystemDef::LensFlare(&FLARE),
        SystemDef::Clouds(&CLOUDS_DEF),
    ],
    order: &[
        Step::Run(Kind::Clouds, Hook::Start),
        DRAW_BG,
        PASSES,
        Step::Map,
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
    use crate::systems::scrolls::Scrolls;
    use crate::systems::sky::Sky;
    use crate::systems::statics::{StaticModels, StaticObjects};
    use crate::systems::{Drawn, Piece};
    use crate::town::{Town, TownView};
    use piney_data::volume::Volume;

    /// The constructor's pieces: 16 model rows (the table's all present),
    /// the static object, the sky, the sun at its dummy, the cloud layers,
    /// the six lights, and the fog and background colour; in crisis the
    /// crisis clumps too.
    #[test]
    fn constructor() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let t = Town::open(&archive, Volume::Out, 3, false).unwrap();
        let s = t.systems();
        assert_eq!(t.def.no, 3);
        let models = s.get::<StaticModels>().unwrap();
        assert_eq!((0..16).filter(|&r| models.model(r).is_some()).count(), 16);
        assert!(s.get::<StaticObjects>().unwrap().object(0).is_some());
        let sky = s.get::<Sky>().unwrap();
        assert!([0, 1, 5, 6, 7].iter().all(|&k| sky.made(k)) && (2..5).all(|k| !sky.made(k)));
        assert_eq!(t.core.lights.lights.len(), 6);
        assert_eq!(t.def.clear, Some(BG_COLOR));
        assert!(!t.core.hits.models.is_empty());
        let c = Town::open(&archive, Volume::Out, 3, true).unwrap();
        assert!((0..8).all(|k| c.systems().get::<Sky>().unwrap().made(k)));
    }

    /// The first `Draw` makes the 25 clouds, and the pieces come in
    /// `Draw`'s order: `DrawBG` (the sky, the sun, the three cloud layers),
    /// the rows and the object by pass, the map; the scrolls step.
    #[test]
    fn first_draw() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut t = Town::open(&archive, Volume::Out, 3, false).unwrap();
        let player = [0, ee::k(6000.0), 0, ONE];
        t.step(&TownView { eye: [0, ee::k(5000.0), ee::k(400.0), ONE], player, ..TownView::default() });
        let clump = |k: usize| Drawn::Piece(Kind::Sky, Piece::Clump(k));
        assert_eq!(&t.frame()[..5], &[clump(0), clump(1), clump(5), clump(6), clump(7)]);
        assert!(t.frame().contains(&Drawn::Map));
        assert_eq!(t.systems().get::<Clouds>().unwrap().clouds.len(), CLOUDS);
        let scrolls = t.systems().get::<Scrolls>().unwrap();
        assert_eq!((&scrolls.v[..], scrolls.uv(0)), (&BG_STEP[..], ee::vftoi12(BG_STEP[0])));
    }
}
