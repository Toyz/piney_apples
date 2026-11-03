//! What a field type is: a [`FieldDef`] each, `WORLD`'s systems as
//! `WORLD::Init` and `WORLD::Generate` make them for the type, `DrawBG`'s
//! for a plain and a hacked field, and `WORLD::Draw`'s and `DrawEffect`'s
//! order ([`FIELD_ORDER`]). Weather, time and the hack flag are the area's
//! (`WORLD_MAN`), which each system tests as the game's code does
//! (docs/engine/field.md, "Field types").

use piney_data::classes::GameClass;

use crate::ee::F;
use crate::story_map::layer;
use crate::systems::fireflies::{FirefliesDef, Firefly2Def, Firefly2Eff, Firefly2Made};
use crate::systems::flare::{FlareDef, FlareDraw, Sun};
use crate::systems::haze::HazeDef;
use crate::systems::scrolls::{Carry, Mat, MatScroll, ScrollsDef, Shown, Turns, Wrap};
use crate::systems::sky::{ClumpName, SkyClump, SkyDef};
use crate::systems::snow::SnowDef;
use crate::systems::{DRAW_BG, Hook, Kind, Step, SystemDef};
use crate::town::scroll_statics as statics;

/// Where `WORLD_MAN::SetCharPosition` stands the party back from the
/// field's dungeon: from the entrance (`dungeonPos[0]`) each of x and y
/// on by its offset or kept, facing `facing`.
#[derive(Clone, Copy, Debug)]
pub struct Back {
    pub x: Option<F>,
    pub y: Option<F>,
    pub facing: F,
}

/// `WORLD_MAN::GetWeather` (main 0x0019f190) and `GetTime` (0x0019ef50)
/// for the type: the weather of each background row its switch lists (0
/// fine, 1 cloudy, 2 rain, 3 storm, 4 light snow, 5 heavy snow) and of any
/// other, and each listed row's time (0 day, 1 evening; 2 night, as any
/// other).
#[derive(Clone, Copy, Debug)]
pub struct Skies {
    pub weather: &'static [u32],
    pub weather_else: u32,
    pub time: &'static [u32],
}

impl Skies {
    pub fn weather(&self, b: u32) -> u32 {
        self.weather.get(b as usize).copied().unwrap_or(self.weather_else)
    }

    pub fn time(&self, b: u32) -> u32 {
        self.time.get(b as usize).copied().unwrap_or(2)
    }
}

/// Types 0-3: cloudy in row 3.
const DRY: Skies = Skies { weather: &[0, 0, 0, 1], weather_else: 0, time: &[0, 1, 2, 0] };
/// Types 4 and 7-10: rain in rows 4 and 5, storms in 6 and 7.
const WET: Skies = Skies { weather: &[0, 0, 0, 1, 2, 2, 3, 3], weather_else: 0, time: &[0, 1, 2, 0, 0, 2, 0] };
/// Types 5 and 6: light snow in rows 0-3, heavy past them.
const SNOWY: Skies = Skies { weather: &[4, 4, 4, 4], weather_else: 5, time: &[0, 1, 2, 0, 0] };

/// A field type: its number, whether it has a field at all (type 4's
/// areas are a lake's dungeons, the first standing for the field),
/// `defSE` (`WORLD_MAN` +0x15c: the ground-type bits `ccLandHitCheck` gives
/// the height map, `GO`'s jump table at main 0x00355760), the sparks its
/// `FIREFLY`s throw (`FIREFLY::Move` by `WORLD_MAN::GetFieldType`), its
/// systems by who makes them, `DrawBG`'s plain and hacked, and the frame's
/// steps awake and asleep. One `static` a type.
#[derive(Debug)]
pub struct FieldDef {
    pub class: GameClass,
    pub ty: u32,
    pub field: bool,
    pub def_se: u32,
    pub skies: Skies,
    pub back: Back,
    pub spark: &'static str,
    /// `WORLD::Init`'s, from the seed's `fieldrand`.
    pub init: &'static [SystemDef],
    /// `WORLD::Generate`'s after `SetStartPos`, from the layout's.
    pub generate: &'static [SystemDef],
    /// `DrawBG`'s, `isHacked` 2 and any other.
    pub plain: &'static [SystemDef],
    pub hacked: &'static [SystemDef],
    pub order: &'static [Step],
    pub asleep: &'static [Step],
}

/// One def is one field type.
impl PartialEq for FieldDef {
    fn eq(&self, other: &FieldDef) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for FieldDef {}

/// `WORLD::Draw`'s pictures after the ground, then `DrawEffect`'s, then
/// `DrawBG`: the snow's first ten falls, the heat haze, the lake, the rain
/// and its storm, the fires' smoke, the snow or embers, the 2D smoke, the
/// fireflies, TOBJ, BIRD, the lens flare, the steam and the background.
pub const FIELD_ORDER: &[Step] = &[
    Step::Run(Kind::Snow, Hook::Start),
    Step::Run(Kind::Haze, Hook::Draw),
    Step::Run(Kind::Lake, Hook::Draw),
    Step::Run(Kind::Rain, Hook::Draw),
    Step::Run(Kind::Thunder, Hook::Draw),
    Step::Run(Kind::Fires, Hook::Draw),
    Step::Run(Kind::Snow, Hook::Draw),
    Step::Run(Kind::Smoke, Hook::Draw),
    Step::Run(Kind::Fireflies, Hook::Draw),
    Step::Run(Kind::Firefly2, Hook::Draw),
    Step::Run(Kind::Tobj, Hook::Draw),
    Step::Run(Kind::Bird, Hook::Draw),
    Step::Run(Kind::LensFlare, Hook::Draw),
    Step::Run(Kind::Steam, Hook::Draw),
    DRAW_BG,
];

/// A frame the tasks sleep through: the last frame's pictures again, the
/// haze's and the lake's sampling of the picture under them brought up to
/// the camera.
pub const FIELD_ASLEEP: &[Step] = &[Step::Run(Kind::Haze, Hook::Again), Step::Run(Kind::Lake, Hook::Again)];

/// `DrawBG`'s steps: the scroll 0.003 a frame; hacked, the turns 0.001,
/// wrapped at 3.14.
const BG_STEP: F = 0x3b44_9ba6;
const TURN_STEP: F = 0x3a83_126f;
const TURN_MAX: F = 0x4048_f5c3;

/// `DrawBG`'s scroll `v$4519` and turns, statics of the executable: plain,
/// the sky's V takes it and it steps on, past 1 less 1; hacked, the turns
/// step first, the sky's V takes it and it steps back, below 0 on by 1. The
/// aurora's U takes it as stepped.
static PLAIN_SCROLLS: ScrollsDef = scrolls(Wrap::Less, false);
static HACKED_SCROLLS: ScrollsDef = scrolls(Wrap::Below, true);

const fn scrolls(wrap: Wrap, stepped: bool) -> ScrollsDef {
    ScrollsDef {
        steps: &[BG_STEP],
        wrap,
        carry: Some(Carry::Statics(statics::WORLD_BG)),
        first: None,
        mats: &[
            MatScroll { mat: Mat::Sky, u: None, v: Some(0), shown: Shown::Before },
            MatScroll { mat: Mat::Aurora, u: Some(0), v: None, shown: Shown::After },
        ],
        turns: Some(Turns { step: TURN_STEP, max: TURN_MAX, slot: statics::WORLD_BG_TURN, stepped }),
    }
}

/// `DrawBG`'s clumps about the centre (`WORLD` +0x60b0 on, priority -100 to
/// -60): hacked, the mountains first, unturned; the sky and the first
/// layer unturned, the second by the first turn, the aurora (type 6's) and
/// the last by the second.
static PLAIN_SKY: SkyDef = SkyDef { clumps: &[SKY, FIRST, SECOND, LAST] };
static PLAIN_AURORA_SKY: SkyDef = SkyDef { clumps: &[SKY, FIRST, SECOND, AURORA, LAST] };
static HACKED_SKY: SkyDef = SkyDef { clumps: &[MOUNTAINS, SKY, FIRST, SECOND, LAST] };
static HACKED_AURORA_SKY: SkyDef = SkyDef { clumps: &[MOUNTAINS, SKY, FIRST, SECOND, AURORA, LAST] };
const SKY: SkyClump = SkyClump::field(ClumpName::Row(0), layer::BG0, None);
const FIRST: SkyClump = SkyClump::field(ClumpName::Row(1), layer::BG1, None);
const SECOND: SkyClump = SkyClump::field(ClumpName::Row(2), layer::BG2, Some(0));
const LAST: SkyClump = SkyClump::field(ClumpName::Row(3), layer::BG3, Some(1));
const MOUNTAINS: SkyClump = SkyClump::field(ClumpName::Row(4), layer::bg(4), None);
const AURORA: SkyClump = SkyClump::field(ClumpName::Aurora, layer::BG2, Some(1));

const PLAIN: &[SystemDef] = &[SystemDef::Scrolls(&PLAIN_SCROLLS), SystemDef::Sky(&PLAIN_SKY)];
const HACKED: &[SystemDef] = &[SystemDef::Scrolls(&HACKED_SCROLLS), SystemDef::Sky(&HACKED_SKY)];

/// The lens flare's sun: `DMY_<x>lig_<b + 1>point` of the background, the
/// table at gcmn 0x006588f0 by type (its rows, four for types 0-3, seven
/// for the others), drawn by `LENSFLARE::Draw(bgstream, name, 1)` with
/// `field_eff`'s six while `WORLD_MAN::CheckLensFlare` holds.
const fn flare(name: &'static str, rows: u32) -> FlareDef {
    FlareDef { sun: Sun::Row { name, rows }, file: crate::field::ambient::EFF_FILE, draw: FlareDraw::Field }
}

static FLARE_0: FlareDef = flare("sfalig", 4);
static FLARE_1: FlareDef = flare("sfblig", 4);
static FLARE_2: FlareDef = flare("sfclig", 4);
static FLARE_3: FlareDef = flare("sfdlig", 4);
static FLARE_5: FlareDef = flare("sfglig", 7);
static FLARE_6: FlareDef = flare("sfhlig", 7);
static FLARE_7: FlareDef = flare("sfilig", 7);
static FLARE_8: FlareDef = flare("sfklig", 7);
static FLARE_9: FlareDef = flare("sfmlig", 7);
static FLARE_10: FlareDef = flare("sfp7lig", 7);

/// The heat haze of types 0-3: 3000 ahead of the player for 0 and 1, 6000
/// for 2 and 3.
static HAZE_NEAR: HazeDef = HazeDef { ahead: 0xc53b_8000 };
static HAZE_FAR: HazeDef = HazeDef { ahead: 0xc5bb_8000 };

/// `SNOW`s: type 0's sixteen embers, or by weather the flakes (100 light,
/// 200 heavy), of the field's own file: `sfg8` in type 5's, `sfh8` in 6's.
static EMBERS: SnowDef = SnowDef::Embers(16);
static FLAKES_5: SnowDef =
    SnowDef::Flakes { light: 100, heavy: 200, effs: ["EFF_sfg8sno1", "EFF_sfg8sno2", "EFF_sfg8sno3", "EFF_sfg8sno4"] };
static FLAKES_6: SnowDef =
    SnowDef::Flakes { light: 100, heavy: 200, effs: ["EFF_sfh8sno1", "EFF_sfh8sno2", "EFF_sfh8sno3", "EFF_sfh8sno4"] };

/// `Generate`'s five `FIREFLY`s by night or hacked, and field type 9's
/// fifteen `FIREFLY2`s roaming about the camera.
static FIREFLIES: FirefliesDef = FirefliesDef { count: 5 };
static ROAMERS: Firefly2Def =
    Firefly2Def { count: 15, made: Firefly2Made::Roam, eff: Firefly2Eff::Field("EFF_sfzfir1") };

/// What `Generate` makes on every field: TOBJ, by its roll, the rain with
/// its storm and the fireflies, by the weather and time, and the lake, by
/// the layout.
const GENERATE: &[SystemDef] = &[
    SystemDef::Tobj(&()),
    SystemDef::Rain(&()),
    SystemDef::Thunder(&()),
    SystemDef::Fireflies(&FIREFLIES),
    SystemDef::Lake(&()),
];

/// What every field type takes unless it says otherwise.
pub const BASE_FIELD: FieldDef = FieldDef {
    class: GameClass::World,
    ty: 0,
    field: true,
    def_se: 0xc000,
    skies: DRY,
    back: Back { x: None, y: Some(0xc461_0000), facing: 0 },
    spark: "EFF_sfpfir_1",
    init: &[],
    generate: GENERATE,
    plain: PLAIN,
    hacked: HACKED,
    order: FIELD_ORDER,
    asleep: FIELD_ASLEEP,
};

pub static FIELD_0: FieldDef = FieldDef {
    ty: 0,
    def_se: 0x20f0,
    init: &[
        SystemDef::Snow(&EMBERS),
        SystemDef::Steam(&()),
        SystemDef::Haze(&HAZE_NEAR),
        SystemDef::LensFlare(&FLARE_0),
    ],
    ..BASE_FIELD
};

pub static FIELD_1: FieldDef = FieldDef {
    ty: 1,
    def_se: 0x20f0,
    init: &[SystemDef::Haze(&HAZE_NEAR), SystemDef::LensFlare(&FLARE_1)],
    ..BASE_FIELD
};

pub static FIELD_2: FieldDef = FieldDef {
    ty: 2,
    def_se: 0x0090_b0c0,
    init: &[SystemDef::Smoke(&()), SystemDef::Haze(&HAZE_FAR), SystemDef::LensFlare(&FLARE_2)],
    ..BASE_FIELD
};

pub static FIELD_3: FieldDef = FieldDef {
    ty: 3,
    def_se: 0x0090_b0c0,
    init: &[SystemDef::Smoke(&()), SystemDef::Haze(&HAZE_FAR), SystemDef::LensFlare(&FLARE_3)],
    ..BASE_FIELD
};

/// Type 4: no field; its first dungeon is a lake's and stands for it, and
/// its `FIREFLY`s stand on the ground at 0.
pub static FIELD_4: FieldDef =
    FieldDef { ty: 4, field: false, def_se: 0xc000, skies: WET, generate: &[], plain: &[], hacked: &[], ..BASE_FIELD };

pub static FIELD_5: FieldDef = FieldDef {
    ty: 5,
    def_se: 0x00e0_e0e0,
    skies: SNOWY,
    init: &[SystemDef::Snow(&FLAKES_5), SystemDef::Smoke(&()), SystemDef::LensFlare(&FLARE_5)],
    ..BASE_FIELD
};

/// Type 6 adds an aurora to `DrawBG`; the party comes back from its
/// dungeon 900 past the entrance, facing pi.
pub static FIELD_6: FieldDef = FieldDef {
    ty: 6,
    def_se: 0x00d0_d0d0,
    skies: SNOWY,
    back: Back { x: None, y: Some(0x4461_0000), facing: crate::ee::PI },
    init: &[SystemDef::Snow(&FLAKES_6), SystemDef::Smoke(&()), SystemDef::LensFlare(&FLARE_6)],
    plain: &[SystemDef::Scrolls(&PLAIN_SCROLLS), SystemDef::Sky(&PLAIN_AURORA_SKY)],
    hacked: &[SystemDef::Scrolls(&HACKED_SCROLLS), SystemDef::Sky(&HACKED_AURORA_SKY)],
    ..BASE_FIELD
};

/// Type 7's fires smoke (`SetSpecialObj`'s places).
pub static FIELD_7: FieldDef = FieldDef {
    ty: 7,
    def_se: 0x0040_5060,
    skies: WET,
    init: &[SystemDef::LensFlare(&FLARE_7)],
    generate: &[
        SystemDef::Tobj(&()),
        SystemDef::Rain(&()),
        SystemDef::Thunder(&()),
        SystemDef::Fireflies(&FIREFLIES),
        SystemDef::Lake(&()),
        SystemDef::Fires(&()),
    ],
    ..BASE_FIELD
};

pub static FIELD_8: FieldDef =
    FieldDef { ty: 8, def_se: 0x6040, skies: WET, init: &[SystemDef::LensFlare(&FLARE_8)], ..BASE_FIELD };

/// Type 9: `FIREFLY2`s roam in place of the `FIREFLY`s, whose sparks are
/// `EFF_sfzfir1` here.
pub static FIELD_9: FieldDef = FieldDef {
    ty: 9,
    def_se: 0x6040,
    skies: WET,
    spark: "EFF_sfzfir1",
    init: &[SystemDef::LensFlare(&FLARE_9)],
    generate: &[
        SystemDef::Tobj(&()),
        SystemDef::Rain(&()),
        SystemDef::Thunder(&()),
        SystemDef::Firefly2(&ROAMERS),
        SystemDef::Lake(&()),
    ],
    ..BASE_FIELD
};

/// Type 10's hawk, BIRD, after the others; the party comes back from its
/// dungeon 900 to the entrance's side, facing 1.5675.
pub static FIELD_10: FieldDef = FieldDef {
    ty: 10,
    def_se: 0xc000,
    skies: WET,
    back: Back { x: Some(0x4461_0000), y: Some(0), facing: 0x3fc8_f5c3 },
    init: &[SystemDef::LensFlare(&FLARE_10)],
    generate: &[
        SystemDef::Tobj(&()),
        SystemDef::Rain(&()),
        SystemDef::Thunder(&()),
        SystemDef::Fireflies(&FIREFLIES),
        SystemDef::Bird(&()),
        SystemDef::Lake(&()),
    ],
    ..BASE_FIELD
};

/// The field types by number (`GO(1)`'s by `WORLD_MAN`'s field type).
pub static FIELDS: [&FieldDef; 11] =
    [&FIELD_0, &FIELD_1, &FIELD_2, &FIELD_3, &FIELD_4, &FIELD_5, &FIELD_6, &FIELD_7, &FIELD_8, &FIELD_9, &FIELD_10];

impl FieldDef {
    /// Field type `ty`'s def, None past the table.
    pub fn of(ty: u32) -> Option<&'static FieldDef> {
        FIELDS.get(ty as usize).copied()
    }

    /// `GetWeather` and `GetTime` of type `ty` with background row `b`
    /// (past the table, the switches' last arms: fine, and as types 0-3).
    pub fn skies_of(ty: u32) -> Skies {
        FieldDef::of(ty).map_or(Skies { weather_else: 0, ..DRY }, |d| d.skies)
    }
}
