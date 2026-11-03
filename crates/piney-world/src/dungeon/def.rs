//! What a dungeon type is: a [`DungeonDef`] each, the `DUNGEON` systems
//! its constructor makes, what `SetRoom` stands in a room and in what order
//! ([`RoomStep`]), and `DUNGEON::Draw`'s order awake and through a frame the
//! tasks sleep (docs/engine/dungeon.md, "Dungeon types").

use piney_data::classes::GameClass;

use crate::story_map::layer;
use crate::systems::fireflies::{FirefliesDef, Made};
use crate::systems::lake_sky::LakeSkyDef;
use crate::systems::pieces::PiecesDef;
use crate::systems::room_lights::{Glow, RoomLightsDef, glow};
use crate::systems::room_water::RoomWaterDef;
use crate::systems::sparks::SparksDef;
use crate::systems::{Hook, Kind, RoomPart, Step, SystemDef};

/// One step of `SetRoom` after the room's anm and its hits, in the game's
/// order: a system's part, or `SetDoor` (the doors and the ban block).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomStep {
    Run(Kind, RoomPart),
    Doors,
}

/// A dungeon type: the lakes (types 8 and 9, a field type 4 area's,
/// open to the sky: one floor, no event rooms, statues), `GetFood()`'s
/// gimmick, `EntryBreakObject`'s rows (none in the lakes), the story
/// rooms' `ROOM_INFO` family (`sroom_infoE`, types t and t + 4 share it),
/// `MoveDoor`'s sound (its jump tables @8659 and @8664) and whether an
/// open door's leaf leaves the hit list; its systems; `SetRoom`'s steps;
/// `Draw`'s steps awake and asleep. One `static` a type.
#[derive(Debug)]
pub struct DungeonDef {
    pub class: GameClass,
    pub ty: u8,
    pub lake: bool,
    pub food: i32,
    pub breakables: Option<[i32; 4]>,
    pub family: usize,
    pub door_se: i32,
    pub leaf_off: bool,
    pub systems: &'static [SystemDef],
    pub room: &'static [RoomStep],
    pub order: &'static [Step],
    pub asleep: &'static [Step],
}

/// One def is one dungeon type.
impl PartialEq for DungeonDef {
    fn eq(&self, other: &DungeonDef) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for DungeonDef {}

/// `SetRoom`'s tail: an event room of type 32's tree and leaves, then, the
/// room built, `SetWater` (the water, the sparks), `SetLight`, `SetObject`,
/// `SetDoor`, `SetAnmObject`, and a lake's fireflies.
pub const ROOM: &[RoomStep] = &[
    RoomStep::Run(Kind::Leaves, RoomPart::Leaves),
    RoomStep::Run(Kind::RoomWater, RoomPart::Water),
    RoomStep::Run(Kind::Sparks, RoomPart::Water),
    RoomStep::Run(Kind::RoomLights, RoomPart::Light),
    RoomStep::Run(Kind::Pieces, RoomPart::Object),
    RoomStep::Doors,
    RoomStep::Run(Kind::Pieces, RoomPart::AnmObject),
    RoomStep::Run(Kind::Fireflies, RoomPart::Fireflies),
];

/// `DUNGEON::Draw` before the room and its doors: a lake's `DrawBG`,
/// `DrawWater`, `DrawEff` (the sparks, the room lights' glows, a lake's
/// fireflies, type 32's tree and leaves), then for k under 10 `object[k]`,
/// `anmobj[k]` and `anmobj2[k]`.
pub const ORDER: &[Step] = &[
    Step::Run(Kind::LakeSky, Hook::Draw),
    Step::Run(Kind::RoomWater, Hook::Draw),
    Step::Run(Kind::Sparks, Hook::Draw),
    Step::Run(Kind::RoomLights, Hook::Draw),
    Step::Run(Kind::Fireflies, Hook::Draw),
    Step::Run(Kind::Leaves, Hook::Draw),
    Step::Run(Kind::Pieces, Hook::Draw),
];

/// A frame the tasks sleep through: each as it stands, the water's
/// sampling brought up to the camera and the fireflies drawn again.
pub const ASLEEP: &[Step] = &[
    Step::Run(Kind::LakeSky, Hook::Again),
    Step::Run(Kind::RoomWater, Hook::Again),
    Step::Run(Kind::Sparks, Hook::Again),
    Step::Run(Kind::RoomLights, Hook::Again),
    Step::Run(Kind::Fireflies, Hook::Again),
    Step::Run(Kind::Leaves, Hook::Again),
    Step::Run(Kind::Pieces, Hook::Again),
];

/// `waterAnmName[type]` (@5657, 0x00696af0).
static WATER_1: RoomWaterDef = RoomWaterDef { anim: "ANM_sd1af0_a" };
static WATER_2: RoomWaterDef = RoomWaterDef { anim: "ANM_sd2af0_a" };
static WATER_3: RoomWaterDef = RoomWaterDef { anim: "ANM_sd3af0_a" };
static WATER_5: RoomWaterDef = RoomWaterDef { anim: "ANM_sd5af0_a" };
static WATER_4: RoomWaterDef = RoomWaterDef { anim: "ANM_sd4af0_a" };

/// `DUNGEON::ChangeClut(eff, add)`'s palettes (gp 0x003783a0): type 0's
/// and type 1's, each with its "c1" and "c2" twins; the others change
/// nothing.
static SPARKS_0: SparksDef = SparksDef { clut: Some(["CLT_sd100o11", "CLT_sd100o11c1", "CLT_sd100o11c2"]) };
static SPARKS_1: SparksDef = SparksDef { clut: Some(["CLT_sd200o19", "CLT_sd200o19c1", "CLT_sd200o19c2"]) };
static SPARKS: SparksDef = SparksDef { clut: None };

/// `SetLight`'s glows for each dummy of its three searches, by `type & 3`
/// (jump tables @6868 and @6869): `OBJ_o_light_m0*`'s, `OBJ_o_light_s0*`'s,
/// and the altar's (`OBJ_o_altar_l0*`, types 2 and 6 only).
const WHITE: u32 = 0x00e6_fafa;
const BLUE: u32 = 0x0096_b4fa;
const SKY: u32 = 0x006e_c3e1;
const LEL0: &str = "EFF_0lel0_";
const LEC0: &str = "EFF_0lec0_";
const LEC1: Option<&str> = Some("EFF_0lec1_");
const LEB0: &str = "EFF_0leb0_";
const ALTAR: &[Glow] = &[
    glow(LEC0, [0x4348_0000, 0x439b_0000, 0x4357_0000], BLUE, LEC1),
    glow(LEC0, [0xc348_0000, 0x439b_0000, 0x4357_0000], BLUE, LEC1),
];
static LIGHTS_0: RoomLightsDef = RoomLightsDef {
    m0: &[glow(LEL0, [0, 0xc016_6666, 0x439b_0000], WHITE, None)],
    s0: &[glow("EFF_0let0_", [0, 0, 0x42b4_0000], BLUE, Some("EFF_0let1_"))],
    altar: &[],
};
static LIGHTS_1: RoomLightsDef = RoomLightsDef {
    m0: &[glow(LEL0, [0, 0xbeb3_3333, 0x439b_0000], WHITE, None)],
    s0: &[glow(LEC0, [0x4234_0000, 0, 0x434d_0000], BLUE, LEC1), glow(LEC0, [0xc234_0000, 0, 0x434d_0000], BLUE, LEC1)],
    altar: &[],
};
static LIGHTS_2: RoomLightsDef = RoomLightsDef {
    m0: &[glow(LEC0, [0, 0x432f_0000, 0x4357_0000], BLUE, LEC1)],
    s0: &[glow(LEC0, [0, 0x4120_0000, 0x4357_0000], BLUE, LEC1)],
    altar: ALTAR,
};
static LIGHTS_3: RoomLightsDef = RoomLightsDef {
    m0: &[glow(LEB0, [0, 0x4280_0000, 0x43d0_0000], SKY, None)],
    s0: &[
        glow(LEB0, [0, 0, 0x43c8_0000], WHITE, None),
        glow(LEB0, [0x42c8_0000, 0, 0x4389_8000], WHITE, None),
        glow(LEB0, [0xc2c8_0000, 0, 0x4348_0000], WHITE, None),
    ],
    altar: &[],
};
static LIGHTS_6: RoomLightsDef = RoomLightsDef { altar: ALTAR, ..LIGHTS_2 };
static LIGHTS_4: RoomLightsDef = RoomLightsDef { ..LIGHTS_0 };
static LIGHTS_5: RoomLightsDef = RoomLightsDef { ..LIGHTS_1 };
static LIGHTS_7: RoomLightsDef = RoomLightsDef { ..LIGHTS_3 };

/// `SetObject`'s and `SetAnmObject`'s pieces: types 0-3's animated
/// objects (@5566 0x00696ad0, @5571 0x00696ae0), types 3 and 7's walls
/// (@7087-7090), the lakes' statues and flowers.
const WALLS: [(&str, &str); 2] = [("OBJ_0paw0*", "ANM_sd5aw0_a"), ("OBJ_0paw1*", "ANM_sd5aw1_a")];
static PIECES_0: PiecesDef = PiecesDef { anm: Some(("OBJ_0pag0_*", "ANM_sd1ag0_a")), walls: None, statues: false };
static PIECES_1: PiecesDef = PiecesDef { anm: Some(("OBJ_0pac0_*", "ANM_sd2ag0_a")), walls: None, statues: false };
static PIECES_2: PiecesDef = PiecesDef { anm: Some(("OBJ_0par0_*", "ANM_sd3ag0_a")), walls: None, statues: false };
static PIECES_3: PiecesDef =
    PiecesDef { anm: Some(("OBJ_0pab0_*", "ANM_sd5ag0_a")), walls: Some(WALLS), statues: false };
static PIECES: PiecesDef = PiecesDef { anm: None, walls: None, statues: false };
static PIECES_7: PiecesDef = PiecesDef { anm: None, walls: Some(WALLS), statues: false };
static PIECES_LAKE: PiecesDef = PiecesDef { anm: None, walls: None, statues: true };

/// The lakes' `DrawBG` clumps (`CMP_o_<name><bg>_`, by `WORLD_MAN::GetBG`)
/// and the material its scroll writes.
static SKY_8: LakeSkyDef =
    LakeSkyDef { clumps: &["bac_l", "bac_m", "clo_l", "clo_m"], material: ("MAT_sfp7bac", ""), layers: LAKE_LAYERS };
static SKY_9: LakeSkyDef = LakeSkyDef {
    clumps: &["bac_l", "bac_m", "ero_l", "ero_m", "ero_s"],
    material: ("MAT_sfp9dat", "_3"),
    layers: LAKE_LAYERS,
};
/// `WORLD_MAN` +0x494 .. +0x4a4: the layers a lake's `DrawBG` draws on.
const LAKE_LAYERS: [i16; 5] = [layer::BG0, layer::BG1, layer::BG2, layer::BG3, layer::bg(4)];

/// `SetRoom`'s five `FIREFLY`s by the room's centre, by night or hacked.
static LAKE_FIREFLIES: FirefliesDef = FirefliesDef { count: 5, made: Made::Room };

/// What every dungeon type takes unless it says otherwise.
pub const BASE_DUNGEON: DungeonDef = DungeonDef {
    class: GameClass::Dungeon,
    ty: 0,
    lake: false,
    food: 23,
    breakables: None,
    family: 4,
    door_se: 45,
    leaf_off: false,
    systems: &[],
    room: ROOM,
    order: ORDER,
    asleep: ASLEEP,
};

/// `DUNGEON::EntryBreakObject`'s gimmick rows (its jump table @4166 and
/// `EntryBreakObjectMain`'s @4116, types 0-3 and again 4-7): `OBJ_0pr4*` ..
/// `OBJ_0pr7*` take them in turn, `OBJ_0pr2*` one of the four at random
/// (`fieldrand(4)`, drawn for every breakable).
const BREAK_0: [i32; 4] = [8, 10, 7, 12];
const BREAK_1: [i32; 4] = [8, 9, 7, 12];
const BREAK_2: [i32; 4] = [10, 9, 11, 12];
const BREAK_3: [i32; 4] = [13, 14, 11, 12];

pub static DUNGEON_0: DungeonDef = DungeonDef {
    ty: 0,
    food: 34,
    breakables: Some(BREAK_0),
    family: 4,
    door_se: 45,
    systems: &[
        SystemDef::RoomWater(&WATER_1),
        SystemDef::Sparks(&SPARKS_0),
        SystemDef::RoomLights(&LIGHTS_0),
        SystemDef::Pieces(&PIECES_0),
        SystemDef::Leaves(&()),
    ],
    ..BASE_DUNGEON
};

pub static DUNGEON_1: DungeonDef = DungeonDef {
    ty: 1,
    food: 35,
    breakables: Some(BREAK_1),
    family: 5,
    door_se: 46,
    systems: &[
        SystemDef::RoomWater(&WATER_2),
        SystemDef::Sparks(&SPARKS_1),
        SystemDef::RoomLights(&LIGHTS_1),
        SystemDef::Pieces(&PIECES_1),
        SystemDef::Leaves(&()),
    ],
    ..BASE_DUNGEON
};

pub static DUNGEON_2: DungeonDef = DungeonDef {
    ty: 2,
    food: 36,
    breakables: Some(BREAK_2),
    family: 6,
    door_se: 47,
    systems: &[
        SystemDef::RoomWater(&WATER_3),
        SystemDef::Sparks(&SPARKS),
        SystemDef::RoomLights(&LIGHTS_2),
        SystemDef::Pieces(&PIECES_2),
        SystemDef::Leaves(&()),
    ],
    ..BASE_DUNGEON
};

pub static DUNGEON_3: DungeonDef = DungeonDef {
    ty: 3,
    food: 37,
    breakables: Some(BREAK_3),
    family: 7,
    door_se: 51,
    leaf_off: true,
    systems: &[
        SystemDef::RoomWater(&WATER_5),
        SystemDef::Sparks(&SPARKS),
        SystemDef::RoomLights(&LIGHTS_3),
        SystemDef::Pieces(&PIECES_3),
        SystemDef::Leaves(&()),
    ],
    ..BASE_DUNGEON
};

pub static DUNGEON_4: DungeonDef = DungeonDef {
    ty: 4,
    food: 34,
    breakables: Some(BREAK_0),
    family: 4,
    door_se: 45,
    systems: &[
        SystemDef::RoomWater(&WATER_1),
        SystemDef::Sparks(&SPARKS),
        SystemDef::RoomLights(&LIGHTS_4),
        SystemDef::Pieces(&PIECES),
        SystemDef::Leaves(&()),
    ],
    ..BASE_DUNGEON
};

pub static DUNGEON_5: DungeonDef = DungeonDef {
    ty: 5,
    food: 35,
    breakables: Some(BREAK_1),
    family: 5,
    door_se: 46,
    systems: &[
        SystemDef::RoomWater(&WATER_2),
        SystemDef::Sparks(&SPARKS),
        SystemDef::RoomLights(&LIGHTS_5),
        SystemDef::Pieces(&PIECES),
        SystemDef::Leaves(&()),
    ],
    ..BASE_DUNGEON
};

pub static DUNGEON_6: DungeonDef = DungeonDef {
    ty: 6,
    food: 36,
    breakables: Some(BREAK_2),
    family: 6,
    door_se: 47,
    systems: &[
        SystemDef::RoomWater(&WATER_3),
        SystemDef::Sparks(&SPARKS),
        SystemDef::RoomLights(&LIGHTS_6),
        SystemDef::Pieces(&PIECES),
        SystemDef::Leaves(&()),
    ],
    ..BASE_DUNGEON
};

pub static DUNGEON_7: DungeonDef = DungeonDef {
    ty: 7,
    food: 37,
    breakables: Some(BREAK_3),
    family: 7,
    door_se: 51,
    leaf_off: true,
    systems: &[
        SystemDef::RoomWater(&WATER_5),
        SystemDef::Sparks(&SPARKS),
        SystemDef::RoomLights(&LIGHTS_7),
        SystemDef::Pieces(&PIECES_7),
        SystemDef::Leaves(&()),
    ],
    ..BASE_DUNGEON
};

/// The lakes: their sky, the scroll it carries, the statues and the
/// fireflies by night; no room lights.
pub static DUNGEON_8: DungeonDef = DungeonDef {
    ty: 8,
    lake: true,
    food: 33,
    family: 4,
    door_se: 48,
    leaf_off: true,
    systems: &[
        SystemDef::LakeSky(&SKY_8),
        SystemDef::RoomWater(&WATER_4),
        SystemDef::Sparks(&SPARKS),
        SystemDef::Pieces(&PIECES_LAKE),
        SystemDef::Fireflies(&LAKE_FIREFLIES),
        SystemDef::Leaves(&()),
    ],
    ..BASE_DUNGEON
};

pub static DUNGEON_9: DungeonDef = DungeonDef {
    ty: 9,
    lake: true,
    food: 33,
    family: 5,
    door_se: 48,
    leaf_off: true,
    systems: &[
        SystemDef::LakeSky(&SKY_9),
        SystemDef::RoomWater(&WATER_4),
        SystemDef::Sparks(&SPARKS),
        SystemDef::Pieces(&PIECES_LAKE),
        SystemDef::Fireflies(&LAKE_FIREFLIES),
        SystemDef::Leaves(&()),
    ],
    ..BASE_DUNGEON
};

/// The dungeon types by number (`DUNGEON.type`).
pub static DUNGEONS: [&DungeonDef; 10] = [
    &DUNGEON_0, &DUNGEON_1, &DUNGEON_2, &DUNGEON_3, &DUNGEON_4, &DUNGEON_5, &DUNGEON_6, &DUNGEON_7, &DUNGEON_8,
    &DUNGEON_9,
];

impl DungeonDef {
    /// Dungeon type `ty`'s def, None past the table.
    pub fn of(ty: u8) -> Option<&'static DungeonDef> {
        DUNGEONS.get(usize::from(ty)).copied()
    }
}
