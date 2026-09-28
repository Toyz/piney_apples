//! Dun Loireag's Grunties: `ccSetChibiGuso` (gcmn 0x0050f0b0), which places
//! the town's Grunties from the save's growth record (`saveData +0x2194`),
//! and `ccPGuso` (pgbreed.cpp, 0x0050ac00 on, 0x330 bytes over `ccGimmick`):
//! the young Grunty (`npcTbl` 154-157) that walks its route, sits to be
//! spoken to, eats what Kite feeds it and grows through four stages, and the
//! grown ones (145-153) at the town's pens. The menus (`InuMenu` 46,
//! `OtonainuMenu` 45, `BreedingMenu` 56) reach it through its affect
//! functions (docs/engine/town02.md, "The Grunties").

use std::rc::Rc;
use std::sync::Arc;

use glam::Mat4;
use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::save::SaveData;
use piney_data::tables::world;
use piney_data::volume::Volume;
use piney_desktop::assets::SceneFile;

use crate::body::{Body, TRALL};
use crate::char::{Char, View, p2w, w2p};
use crate::ee::{self, F, ONE, V4, add, cosf, from_int, le, lt, mul, sinf, sub};
use crate::entry::Npc;
use crate::hit::{self, UNIT, WALL_MASK};
use crate::merchant::EntryObj;
use crate::mt::Mt;
use crate::npc::NpcRow;
use crate::pose::Play;
use crate::rtownpc::{get_dirc, get_dirc_chg, get_dirc0, get_dist3d, set_dirc};

/// `npcTbl`'s grown Grunties (145-153) and young ones (154-157).
pub const ROW_ADULT: i32 = 145;
pub const ROW_YOUNG: i32 = 154;
/// `saveData.growth[5]` (+0x2194): a `GROWTH_PARAM` (0x18 bytes) a town.
pub const SAVE_GROWTH: usize = 0x2194;
/// `saveData.inuCount[9]` (+0x7474): how many of each grown kind.
pub const SAVE_INU_COUNT: usize = 0x7474;
/// `saveData.impItemList` (+0xcfc): the key items; 49 the Grunty Flute.
const IMP_ITEM_LIST: usize = 0x0cfc;
const FLUTE: usize = 49;

/// `pgAnmPtr[4]` (gcmn 0x005ee300): levels 0 and 1 play the `cdga` table
/// (0x005ee230) on the main anm, 2 and 3 the `cdgb` one (0x005ee250,
/// also `pgAnmTblB`) on anmB; entries 8-11 are the next body's.
pub const ANM_A: [Option<&str>; 12] = [
    Some("ANM_cdganut0"),
    Some("ANM_cdganut1"),
    Some("ANM_cdgaeat0"),
    Some("ANM_cdgawal0"),
    Some("ANM_cdgaevo0"),
    None,
    None,
    None,
    Some("ANM_cdgbnut0"),
    Some("ANM_cdgbnut1"),
    Some("ANM_cdgbeat0"),
    Some("ANM_cdgbwal0"),
];
pub const ANM_B: [Option<&str>; 12] = [
    Some("ANM_cdgbnut0"),
    Some("ANM_cdgbnut1"),
    Some("ANM_cdgbeat0"),
    Some("ANM_cdgbwal0"),
    Some("ANM_cdgbevo0"),
    None,
    None,
    None,
    Some("ANM_cdg0nut0"),
    Some("ANM_cdg0nut1"),
    Some("ANM_cdg0run0"),
    Some("ANM_cdg0wal0"),
];

/// `pgAnmPtrAdult[9]` (gcmn 0x005ee310): each grown kind's first four
/// (`cdgN` nut0, nut1, run0, wal0; kind 0 is `cdg0`, 1-8 `cdg2`-`cdg9`).
pub fn adult_anm(local: usize, n: usize) -> Option<String> {
    // Each row holds its own kind's four, then the next two kinds'; a
    // name past the kinds' end reads the table after it (never played).
    let kind = local + n / 4;
    if n >= 12 || kind > 8 {
        return None;
    }
    let k = if kind == 0 { 0 } else { kind + 1 };
    let what = ["nut0", "nut1", "run0", "wal0"][n % 4];
    Some(format!("ANM_cdg{k}{what}"))
}

/// `ccsTblPG[9]` (gcmn 0x005ee340): each grown kind's file.
pub const CCS_ADULT: [&str; 9] =
    ["cdogbod0", "cdogbod2", "cdogbod3", "cdogbod4", "cdogbod5", "cdogbod6", "cdogbod7", "cdogbod8", "cdogbod9"];
/// The young one's second body, always loaded (`ccscB`).
pub const CCS_B: &str = "cdogbodb";

/// `markPosTbl[8]` (gcmn 0x005ee3c0, pgbreed's own): by town 1-4 the young
/// one's route (NULL-ended) and the grown ones' three places.
pub fn route(town: i32) -> &'static [&'static str] {
    match town {
        1 => &["DMY_marker34", "DMY_marker35", "DMY_marker36"],
        2 => &["DMY_marker27", "DMY_marker26", "DMY_marker30"],
        3 => &["DMY_marker118", "DMY_marker119", "DMY_marker120"],
        _ => &["DMY_marker01", "DMY_marker02", "DMY_marker03"],
    }
}
pub const ADULT_SPOTS: [&str; 3] = ["DMY_cdog0", "DMY_cdog1", "DMY_cdog2"];

/// `adultGusoParam[4]` (gcmn 0x005ee3e0, `GROWTH_PARAM` rows by town 1-4):
/// the smell .. pure a grown one of the town's first kind should have.
pub const ADULT_PARAM: [[i16; 7]; 4] =
    [[0, 30, 10, 5, 20, 15, 5], [0, 40, 20, 10, 15, 15, 5], [0, 40, 15, 15, 5, 20, 10], [0, 40, 10, 15, 5, 15, 20]];
/// `pgSizeParam[16]` (gcmn 0x005ee440): by town the sizes that start
/// levels 1, 2, 3 and grown.
pub const SIZE_PARAM: [[i16; 4]; 4] = [[5, 10, 20, 30], [7, 15, 25, 40], [7, 15, 25, 40], [7, 15, 25, 40]];
/// `foodTbl[16]` (gcmn 0x005ee450, `FOOD_PARAM`): what each food adds to
/// size, smell, crooked, cruel, iq and pure.
pub const FOOD_TBL: [[i16; 6]; 16] = [
    [2, 0, 0, 0, 0, 0],
    [1, 0, 4, -4, -2, -1],
    [1, 4, 3, -3, 0, 1],
    [1, 1, 5, -2, -1, 2],
    [1, 3, 1, -1, 1, 0],
    [1, 2, 2, 0, 2, 4],
    [1, -1, 0, 1, 3, 5],
    [1, -2, -1, 2, 0, 3],
    [1, -3, -2, 3, 5, 0],
    [1, -4, -3, 0, -3, -3],
    [1, 5, 0, 4, -4, -4],
    [1, 0, -4, 5, 4, -2],
    [2, -3, -1, 3, 2, 1],
    [2, -1, -3, 1, 2, 3],
    [2, 3, 1, 0, -1, -3],
    [2, 1, 3, 0, -3, -1],
];
/// `needFoodTbl1`, `needFoodTbl2` (gcmn 0x005ee510, 0x005ee530): by the
/// stat furthest from a grown one's (smell .. pure; the first four rows),
/// four foods it asks for when it is short of (1) or over (2) it; the
/// line is 3 + 2 (food - 1).
pub const NEED_FOOD_1: [[i8; 4]; 5] = [[2, 4, 10, 14], [1, 2, 3, 15], [8, 10, 11, 12], [6, 8, 11, 5], [5, 6, 7, 13]];
pub const NEED_FOOD_2: [[i8; 4]; 5] = [[7, 8, 9, 12], [8, 9, 11, 13], [1, 2, 3, 1], [1, 9, 10, 15], [9, 10, 11, 14]];
/// The message tables `dogAction2` (gcmn 0x005ee6f0, by row 145-153) and
/// `dogActionAdult` (0x005ee780, by kind) draw a line from, `ccRand() & 3`.
pub const ADULT_LINES: [[i8; 4]; 9] = [
    [7, 9, 10, 12],
    [7, 8, 9, 10],
    [7, 8, 10, 12],
    [7, 8, 9, 10],
    [7, 8, 9, 10],
    [7, 8, 9, 10],
    [7, 8, 9, 10],
    [7, 8, 9, 10],
    [7, 8, 10, 14],
];
pub const GROWN_LINES: [[i8; 4]; 9] = [
    [7, 9, 10, 12],
    [7, 8, 9, 10],
    [7, 8, 10, 12],
    [7, 8, 9, 10],
    [7, 8, 9, 10],
    [7, 8, 9, 10],
    [7, 8, 9, 10],
    [7, 8, 9, 10],
    [7, 8, 10, 14],
];
/// `@1899` (gcmn 0x005ee6b0): a new grown kind's line when Kite has the
/// flute already.
pub const FLUTE_LINES: [i8; 9] = [14, 11, 13, 11, 12, 13, 11, 11, 16];
/// The voices of evoActAdult (`ccCheckVoiceGrp(145 + kind)`, n): the
/// change, the look back, the run.
pub const VOICE_GROW: [i32; 9] = [23, 20, 23, 21, 25, 23, 21, 21, 26];
pub const VOICE_TURN: [i32; 9] = [20, 17, 19, 17, 18, 19, 17, 17, 22];
pub const VOICE_RUN: [i32; 9] = [26, 23, 25, 23, 27, 25, 23, 23, 28];

/// bodyHit: radius 65, height 60, kind 2.
const RADIUS: F = 0x4282_0000;
const HEIGHT: F = 0x4270_0000;
const KIND: u32 = 2;
/// The body ids of the port's character list: [`BODY_ID`] plus the
/// Grunty's number.
pub const BODY_ID: u32 = 0x280;
const ARRIVE: F = 0x42a0_0000; // 80
const WALK: F = 0x3fc0_0000; // 1.5
const RUN: F = 0x4140_0000; // 12
const HALF: F = 0x3f00_0000;
const MOVED: F = 0x4248_0000; // 50
const LAND_MASK: u32 = 0x2000_0002;
const TENTH: F = 0x3dcc_cccd; // 0.1
const K001: F = 0x3c23_d70a; // 0.01
const K002: F = 0x3ca3_d70a; // 0.02
const K004: F = 0x3d23_d70a; // 0.04
const K005: F = 0x3d4c_cccd; // 0.05

/// `GROWTH_PARAM`: a town's Grunty record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Growth {
    /// 0-3 young, 4 grown.
    pub level: i16,
    pub size: i16,
    pub smell: i16,
    pub crooked: i16,
    pub cruel: i16,
    pub iq: i16,
    pub pure: i16,
    /// The grown kinds this town has (1 each).
    pub ty: [i16; 3],
    /// The food it asks for, as a line (3, 5, ... 33; -1 fresh).
    pub food_num: i32,
}

impl Growth {
    pub fn read(save: &SaveData, town: i32) -> Growth {
        let a = SAVE_GROWTH + 0x18 * town.clamp(0, 4) as usize;
        let s = |k: usize| save.i16(a + 2 * k);
        Growth {
            level: s(0),
            size: s(1),
            smell: s(2),
            crooked: s(3),
            cruel: s(4),
            iq: s(5),
            pure: s(6),
            ty: [s(7), s(8), s(9)],
            food_num: save.i32(a + 0x14),
        }
    }

    pub fn write(&self, save: &mut SaveData, town: i32) {
        let a = SAVE_GROWTH + 0x18 * town.clamp(0, 4) as usize;
        for (k, v) in [self.level, self.size, self.smell, self.crooked, self.cruel, self.iq, self.pure]
            .into_iter()
            .chain(self.ty)
            .enumerate()
        {
            save.set_i16(a + 2 * k, v);
        }
        save.set_i32(a + 0x14, self.food_num);
    }

    /// smell .. pure.
    fn stats(&self) -> [i16; 5] {
        [self.smell, self.crooked, self.cruel, self.iq, self.pure]
    }
}

/// `ccSetChibiGuso` (gcmn 0x0050f0b0) in `town` (area 0): the rows it
/// places (`setDog`), in order, and the save's record as it leaves it (a
/// grown record with a kind free starts over).
pub fn set_chibi_guso(save: &mut SaveData, town: i32) -> Vec<i32> {
    let mut out = Vec::new();
    if town <= 0 {
        return out;
    }
    let mut g = Growth::read(save, town);
    for s in 0..3 {
        if g.ty[s] == 0 {
            continue;
        }
        let row = match (s, town) {
            (0, _) => 145,
            (1, 1..=4) => 144 + 2 * town,
            (2, 1..=4) => 145 + 2 * town,
            _ => continue,
        };
        out.push(row);
    }
    if g.level == 4 {
        if g.ty.iter().all(|&t| t != 0) {
            return out;
        }
        g.level = 0;
        g.size = 0;
        g.smell = 0;
        g.crooked = 0;
        g.cruel = 0;
        g.iq = 0;
        g.pure = 0;
        g.write(save, town);
    }
    if (0..4).contains(&g.level) {
        out.push(ROW_YOUNG + i32::from(g.level));
    }
    out
}

/// Which table and body an anm plays from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnmTbl {
    /// `pgAnmPtr[level]`: the `cdga` table (levels 0, 1) or `cdgb` (2, 3).
    Young(i16),
    /// `pgAnmPtrAdult[kind]`.
    Adult(usize),
}

impl AnmTbl {
    fn name(&self, n: usize) -> Option<String> {
        match *self {
            AnmTbl::Young(l) => {
                let t = if l < 2 { &ANM_A } else { &ANM_B };
                t.get(n).copied().flatten().map(str::to_string)
            }
            AnmTbl::Adult(k) => adult_anm(k, n),
        }
    }
}

/// What a Grunty's frame asks of the rest of the game, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum GruntyEvent {
    /// `ccSeSetParamInu(param, pgPtr)`: a note's step or cry at `pos` (the
    /// last Grunty made; `inuCheckNote` reads `pgPtr`), on ground
    /// `attribute`.
    Note {
        param: u32,
        pos: V4,
        attribute: u32,
    },
    /// `ccSeOn3D(n, pos)`.
    Se3d {
        n: i32,
        pos: V4,
    },
    /// `effSmoke(pos, v, scale, life, kind, a, b)`.
    Smoke {
        pos: V4,
        v: V4,
        scale: F,
        life: i32,
        kind: i32,
    },
    /// `effEvolvePG(ch)`, `effGrowPG(ch)` at the Grunty: its place and
    /// its base's height (`base->height`) then.
    Evolve {
        pos: V4,
        height: F,
    },
    Grow {
        pos: V4,
        height: F,
    },
    /// `ccVoiceRequest(grp, n)`; `ccEvVoiceStop()`.
    Voice {
        grp: i32,
        n: i32,
    },
    VoiceStop,
    /// `ccChatMsg::OpenChat(ch, text)` (pgChatTbl's line), `CloseChat()`.
    Chat(usize),
    ChatClose,
    /// `changeCamera(n)`; `cameraSetPos(v, camID)`, `cameraSetView(v,
    /// camID)`.
    Camera(i16),
    CamPos(V4),
    CamView(V4),
    /// Kite put at `pos` facing `dirc_z` (`plw->pos`, `plw->dirc.z`).
    Player {
        pos: V4,
        dirc_z: F,
    },
    /// The Grunty off the command lists for good (`deleteCmnd(1)`).
    Dropped,
    /// The debug line `evoActPon1`'s dead camera turn prints (`DEG :`).
    Debug(i32),
}

/// One Grunty.
#[derive(Clone)]
pub struct Grunty {
    /// The row it was made from: the port's name for it on the entry
    /// control's list (its base changes as it grows).
    pub code: i32,
    /// Its base: `npcTbl[inuID]` (SetBaseParam as it grows).
    pub row: NpcRow,
    /// Main body (the entry's stream: cdogboda, cdogbodb or a grown kind's)
    /// and its anm (`ch.play`).
    pub ch: Char,
    pub entry: EntryObj,
    /// `ccscB`/`anmB`: cdogbodb; `ccscC`/`anmC`: the grown kind's, from
    /// `adultSetup`.
    pub body_b: Rc<Body>,
    pub play_b: Option<Play>,
    /// anmB's +0x88, the alpha it draws at (1 from `ccCoord::ccCoord`;
    /// the growing's cross-fade writes it).
    pub alpha_b: F,
    pub body_c: Option<Rc<Body>>,
    pub play_c: Option<Play>,
    /// The main anm's play when it has one (`ch.play` otherwise holds its
    /// first animation).
    pub has_a: bool,
    archive: Option<Arc<Archive>>,
    /// +0x9c `affectType`; +0x94 which `affectFunc`.
    pub affect_type: i16,
    pub affect_fn: AffectFn,
    /// +0x200 nextPos, +0x210 oldPos, +0x220 scale, +0x250 camPos, +0x260
    /// camView.
    pub next_pos: V4,
    pub old_pos: V4,
    pub scale: V4,
    pub cam_pos: V4,
    pub cam_view: V4,
    /// +0x270 inuID, +0x274 localID (the grown kind).
    pub inu_id: i32,
    pub local_id: i32,
    /// +0x278 actNum, +0x27c actNumOld, +0x280 actChange, +0x284
    /// actProcess, +0x288 actCnt, +0x28c anmNum, +0x290 discNum.
    pub act: i32,
    pub act_old: i32,
    pub act_change: i32,
    pub act_process: i32,
    pub act_cnt: i32,
    pub anm_num: i32,
    pub disc_num: i32,
    /// +0x294 bodyHitFlag, +0x296 bodyHitCnt, +0x298 posCnt.
    pub body_hit_flag: i16,
    pub body_hit_cnt: i16,
    pub pos_cnt: i16,
    /// +0x29a foodKind, +0x29c foodNum, +0x29e gpSub[5].
    pub food_kind: i16,
    pub food_num: i16,
    pub gp_sub: [i16; 5],
    /// +0x2a8 adultType, +0x2aa dmylevel, +0x2ac spdeg.
    pub adult_type: i16,
    pub dmylevel: i16,
    pub spdeg: u16,
    /// +0x2ae growthNum (0 idle, 1 eating, 2 growing, 3 speaking, 4 the
    /// menu's done, 5 gone), +0x2af msgNum.
    pub growth_num: i8,
    pub msg_num: i8,
    /// +0x2b4 speed (the run's, 10 from `effect`), +0x2b8 pgScale, +0x2bc
    /// camdist, +0x2c0 camdeg.
    pub speed: F,
    pub pg_scale: F,
    pub camdist: F,
    pub camdeg: F,
    /// +0x2c4 markPos (an index into the route), +0x2cc anmTbl.
    pub mark: usize,
    pub route: Vec<V4>,
    pub anm_tbl: AnmTbl,
    /// +0x2d0 grow; the town whose record +0x2e8 points at.
    pub grow: Growth,
    pub town: i32,
    /// +0x2ec camFlag, +0x2f0 delFlag, +0x2f4 evolevel, +0x2f8 evonum,
    /// +0x2fc evocnt, +0x300 chatcnt.
    pub cam_flag: i32,
    pub del_flag: i32,
    pub evolevel: i32,
    pub evonum: i32,
    pub evocnt: i32,
    pub chatcnt: i32,
    /// +0x304 evoflag, +0x305 exist, +0x306 chatFlag, +0x307 foodMode.
    pub evoflag: u8,
    pub exist: u8,
    pub chat_flag: u8,
    pub food_mode: u8,
    /// +0x308 transrate[2], +0x310 selFood.
    pub transrate: [F; 2],
    pub sel_food: i32,
    /// Its number on the port's character list.
    pub num: u32,
    /// What this frame asked for.
    pub events: Vec<GruntyEvent>,
    /// The notes `NoteProcess` handed on this frame (event, param), in
    /// order, for the caller to give `inuCheckNote` with `pgPtr`.
    pub notes: Vec<(u32, u32)>,
    /// Each anm's note list as its last `_AnimateForward` left it
    /// (`SetAnm` and `NoteProcess` leave it alone).
    pending_a: Vec<(u32, u32)>,
    pending_b: Vec<(u32, u32)>,
    pending_c: Vec<(u32, u32)>,
    /// What the last frame drew: the anm (0 main, 1 anmB, 2 anmC), its
    /// alpha, and whether its matrix had the scale. (evoActPon1/2 also set
    /// the draw environment's +0xa4 byte to the second body's alpha, half
    /// of 256 of it, before drawing it.)
    pub drawn: Vec<(u8, F, bool)>,
    /// Tables read from the executable.
    tables: Rc<Tables>,
}

/// The tables the Grunties read at run time, the volume's
/// (`tables::world`): `camPosDef`, `camViewDef`, `inuPos`, `playerPos` by
/// `discNum` (town - 1), the fixed camera, the Grunty's place and Kite's
/// while it eats; `pgChatTbl` ("I wanna eat ... oink!").
#[derive(Clone, Debug)]
pub struct Tables {
    pub cam_pos: [V4; 4],
    pub cam_view: [V4; 4],
    pub inu_pos: [V4; 4],
    pub player_pos: [V4; 4],
    pub chat: Vec<Vec<u8>>,
    /// The rows 145-157.
    pub rows: Vec<NpcRow>,
}

impl Tables {
    pub fn of(volume: Volume) -> Result<Tables> {
        let w = world::of(volume);
        let v4s = |t: [[f32; 4]; 4]| t.map(|v| v.map(f32::to_bits));
        let chat = w.pg_chat().iter().map(|s| piney_data::tables::sjis::encode(s.unwrap_or_default())).collect();
        let rows = (ROW_ADULT..=ROW_YOUNG + 3).map(|r| NpcRow::of(volume, r as usize)).collect::<Result<_>>()?;
        Ok(Tables {
            cam_pos: v4s(w.pg_cam_pos()),
            cam_view: v4s(w.pg_cam_view()),
            inu_pos: v4s(w.pg_inu_pos()),
            player_pos: v4s(w.pg_player_pos()),
            chat,
            rows,
        })
    }

    pub fn row(&self, id: i32) -> Option<&NpcRow> {
        self.rows.get(usize::try_from(id - ROW_ADULT).ok()?)
    }
}

/// The position of dummy `name` in `file` (w 1), and its rotation.
fn dummy(file: &SceneFile, name: &str) -> Option<(V4, V4)> {
    let d = file.ccs.find_object(name).and_then(|o| file.scene.dummies.get(&o))?;
    // Decode_DummyPosRot: the rotation's degrees as pi * deg / 180.
    let rad = |x: f32| ee::div(ee::mul(ee::PI, x.to_bits()), 0x4334_0000);
    let rot = d.rot.map(|r| [rad(r.x), rad(r.y), rad(r.z), 0]).unwrap_or([0; 4]);
    Some(([d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE], rot))
}

/// The files the Grunties need, loaded once for a town.
#[derive(Clone)]
pub struct Bodies {
    pub a: Option<Rc<Body>>,
    pub b: Rc<Body>,
    /// The grown kinds' by kind.
    pub adults: [Option<Rc<Body>>; 9],
}

impl Bodies {
    pub fn load(archive: &Arc<Archive>) -> Result<Bodies> {
        Ok(Bodies { a: None, b: Rc::new(Body::read(archive, CCS_B, TRALL)?), adults: Default::default() })
    }

    /// The body of `stem`, loaded and kept.
    fn stem(&mut self, archive: &Arc<Archive>, stem: &str) -> Result<Rc<Body>> {
        if stem == CCS_B {
            return Ok(self.b.clone());
        }
        if stem == "cdogboda" {
            if self.a.is_none() {
                self.a = Some(Rc::new(Body::read(archive, stem, TRALL)?));
            }
            return Ok(self.a.clone().expect("loaded"));
        }
        let k = CCS_ADULT.iter().position(|&s| s == stem).unwrap_or(0);
        if self.adults[k].is_none() {
            self.adults[k] = Some(Rc::new(Body::read(archive, CCS_ADULT[k], TRALL)?));
        }
        Ok(self.adults[k].clone().expect("loaded"))
    }
}

/// `ccSetChibiGuso`'s `setDog(n)` (gcmn 0x0050ef10) and
/// `ccPGuso::ccPGuso`: row `id` in `town`, its place from the town's
/// dummies; `num` its number on the character list.
#[allow(clippy::too_many_arguments)]
pub fn set_dog(
    archive: &Arc<Archive>,
    tables: &Rc<Tables>,
    bodies: &mut Bodies,
    town_file: &SceneFile,
    hits: &mut hit::Hits,
    save: &SaveData,
    town: i32,
    id: i32,
    num: u32,
) -> Result<Option<Grunty>> {
    // setDog: the young one at its route's first dummy, no turn; a grown
    // one at DMY_cdog0 (145), cdog1 (even rows) or cdog2, with the dummy's
    // rotation.
    let (pos, rot) = if id >= ROW_YOUNG {
        match dummy(town_file, route(town)[0]) {
            Some((p, _)) => (p, [0, 0, 0, ONE]),
            None => return Ok(None),
        }
    } else {
        let k = if id < 146 {
            0
        } else if (id - 146) % 2 == 0 {
            1
        } else {
            2
        };
        match dummy(town_file, ADULT_SPOTS[k]) {
            Some(d) => d,
            None => return Ok(None),
        }
    };
    let Some(row) = tables.row(id).cloned() else { return Ok(None) };
    let body = bodies.stem(archive, &row.stem())?;
    Ok(Grunty::new(archive, tables, bodies, town_file, hits, save, town, &row, body, pos, rot, num))
}

impl Grunty {
    /// `ccPGuso::ccPGuso` (gcmn 0x0050ac00) on the entry `setDog` made.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        archive: &Arc<Archive>,
        tables: &Rc<Tables>,
        bodies: &Bodies,
        town_file: &SceneFile,
        hits: &mut hit::Hits,
        save: &SaveData,
        town: i32,
        row: &NpcRow,
        body: Rc<Body>,
        pos: V4,
        rot: V4,
        num: u32,
    ) -> Option<Grunty> {
        let id = i32::from(row.id);
        let grow = Growth::read(save, town);
        let anm_tbl =
            if id >= ROW_YOUNG { AnmTbl::Young(grow.level) } else { AnmTbl::Adult((id - ROW_ADULT) as usize) };
        let first = anm_tbl.name(0);
        let mut ch = match first.as_deref().and_then(|n| body.play(n)) {
            Some(_) => Char::new(body.clone(), first.as_deref()?, pos, rot, row.height, row.width)?,
            None => {
                // No such animation in the file: the anm plays nothing.
                let any = body.file.anims.first().map(|a| body.file.ccs.object_name(a.object).unwrap_or(""));
                Char::new(body.clone(), any.unwrap_or(""), pos, rot, row.height, row.width)?
            }
        };
        let has_a = first.as_deref().and_then(|n| body.play(n)).is_some();
        // posP is first written by main.
        ch.pos_p = [0; 4];
        ch.hit = hit::Body {
            pos,
            radius: RADIUS,
            height: HEIGHT,
            mask2: WALL_MASK,
            kind: KIND,
            id: BODY_ID + num,
            ..hit::Body::default()
        };
        hits.hit_enable(&mut ch.hit);
        let play_b = ANM_B[0].and_then(|n| bodies.b.play(n));
        let mut g = Grunty {
            code: id,
            row: row.clone(),
            ch,
            entry: EntryObj::default(),
            body_b: bodies.b.clone(),
            play_b,
            alpha_b: ONE,
            body_c: None,
            play_c: None,
            has_a,
            archive: Some(archive.clone()),
            affect_type: 0,
            affect_fn: if id >= ROW_YOUNG { AffectFn::Young } else { AffectFn::Grown },
            next_pos: [0; 4],
            old_pos: [0; 4],
            scale: [0; 4],
            cam_pos: [0; 4],
            cam_view: [0; 4],
            inu_id: id,
            local_id: 0,
            act: 9,
            act_old: 9,
            act_change: -1,
            act_process: 0,
            act_cnt: 0,
            anm_num: 0,
            disc_num: 0,
            body_hit_flag: 0,
            body_hit_cnt: 0,
            pos_cnt: 0,
            food_kind: 0,
            food_num: 0,
            gp_sub: [0; 5],
            adult_type: -1,
            dmylevel: grow.level,
            spdeg: 0,
            growth_num: 0,
            msg_num: 0,
            speed: 0,
            pg_scale: ONE,
            camdist: 0,
            camdeg: 0,
            mark: 0,
            route: Vec::new(),
            anm_tbl,
            grow,
            town,
            cam_flag: 0,
            del_flag: 0,
            evolevel: 0,
            evonum: 0,
            evocnt: 0,
            chatcnt: 0,
            evoflag: 0,
            exist: 0,
            chat_flag: 0,
            food_mode: 0,
            transrate: [0; 2],
            sel_food: 0,
            num,
            events: Vec::new(),
            notes: Vec::new(),
            pending_a: Vec::new(),
            pending_b: Vec::new(),
            pending_c: Vec::new(),
            drawn: Vec::new(),
            tables: tables.clone(),
        };
        if id >= ROW_YOUNG {
            g.route = route(town).iter().map(|n| dummy(town_file, n).map_or(ee::VF0, |d| d.0)).collect();
            g.mark = 1;
            g.next_pos = g.route.get(1).copied().unwrap_or(pos);
            g.ch.dirc[2] = get_dirc(g.ch.pos, g.next_pos);
            g.act = 3;
            g.act_old = 3;
            g.pg_scale = match grow.level {
                1 => 0x3fc0_0000,
                3 => 0x3f99_999a,
                _ => ONE,
            };
        }
        g.scale = [g.pg_scale, g.pg_scale, g.pg_scale, ONE];
        g.disc_num = match town {
            1..=4 => town - 1,
            _ => 0,
        };
        let a = ADULT_PARAM[0];
        let s = grow.stats();
        for k in 0..5 {
            g.gp_sub[k] = a[2 + k].wrapping_sub(s[k]);
        }
        Some(g)
    }

    fn save_grow(&self, save: &mut SaveData) {
        self.grow.write(save, self.town);
    }

    /// `ccChar::SetBaseParam(ccGetNpcParam(id))`.
    fn set_base(&mut self, id: i32) {
        if let Some(r) = self.tables.row(id) {
            self.row = r.clone();
            self.ch.height = r.height;
            self.ch.width = r.width;
        }
    }

    /// `ccPGuso::changeAnmPG(n, level)` (gcmn 0x0050eda0): the anm of
    /// `level` (0-1 the main one, 2-3 anmB, 4 anmC) set to entry `n` of
    /// its table; `anmNum = n`.
    pub fn change_anm(&mut self, n: i32, level: i16) {
        let k = n.max(0) as usize;
        match level {
            0 | 1 => {
                self.anm_tbl = AnmTbl::Young(level);
                if let Some(name) = self.anm_tbl.name(k) {
                    self.has_a = self.ch.set_anim(&name);
                } else {
                    self.has_a = false;
                }
            }
            2 | 3 => {
                self.anm_tbl = AnmTbl::Young(level);
                self.play_b = self.anm_tbl.name(k).and_then(|n| self.body_b.play(&n));
            }
            4 => {
                self.anm_tbl = AnmTbl::Adult(self.local_id.clamp(0, 8) as usize);
                self.play_c = match &self.body_c {
                    Some(b) => self.anm_tbl.name(k).and_then(|n| b.play(&n)),
                    None => None,
                };
            }
            _ => {}
        }
        self.anm_num = n;
    }

    /// The main anm's `frameNow` (+0x98).
    fn frame_a(&self) -> u32 {
        self.ch.play.time >> 8
    }

    fn frame_b(&self) -> u32 {
        self.play_b.as_ref().map_or(0, |p| p.time >> 8)
    }

    /// `_AnimateForward(frameSpd)` of the main anm, its notes kept.
    fn forward_a(&mut self) {
        if self.has_a {
            let (_, notes) = self.ch.play.forward_notes(&self.ch.body.file);
            self.pending_a = notes;
        }
    }

    fn forward_b(&mut self) {
        if let Some(p) = self.play_b.as_mut() {
            let (_, notes) = p.forward_notes(&self.body_b.file);
            self.pending_b = notes;
        }
    }

    fn forward_c(&mut self) {
        if let (Some(p), Some(b)) = (self.play_c.as_mut(), self.body_c.as_ref()) {
            let (_, notes) = p.forward_notes(&b.file);
            self.pending_c = notes;
        }
    }

    /// `ccAnm::NoteProcess`: the anm's notes handed on (again, if it has
    /// not stepped since).
    fn note_process(&mut self, which: u8) {
        let n = match which {
            0 => &self.pending_a,
            1 => &self.pending_b,
            _ => &self.pending_c,
        };
        self.notes.extend(n.iter().copied());
    }

    /// `ccPGuso::main` (gcmn 0x0050b3c0) after `ccEntryObj::routine`.
    pub fn main(&mut self, ctx: &mut GruntyCtx) {
        self.ch.pos_p = w2p(self.ch.pos, ctx.player);
        self.ch.pos = p2w(self.ch.pos_p, ctx.player);
        match self.act {
            0 => {
                if self.act_process == 0 {
                    self.change_anm(0, self.grow.level);
                    self.act_process += 1;
                }
                self.face();
            }
            1 => {
                if self.act_process == 0 {
                    if self.act_old != self.act {
                        self.change_anm(0, self.grow.level);
                    }
                    self.act_process += 1;
                    self.act_old = self.act;
                }
                if self.act_cnt >= 151 {
                    self.act = 3;
                    self.act_process = 0;
                    self.act_cnt = 0;
                } else {
                    self.act_cnt += 1;
                }
            }
            2 => {
                if self.act_process == 0 {
                    if self.anm_num != 0 {
                        self.change_anm(0, self.grow.level);
                    }
                    self.act_process += 1;
                }
                self.face();
            }
            3 => {
                self.walk(ctx);
                if self.act == 3 && self.act_process == 0 {
                    self.change_anm(3, self.grow.level);
                    self.act_process += 1;
                    self.act_old = self.act;
                }
            }
            6 => self.eat(ctx),
            7 => self.evo_adult(ctx),
            9 => {
                self.adult_main(ctx);
                return;
            }
            _ => {}
        }
        self.ch.hit.pos = self.ch.pos;
        if ctx.hits.collision_detection(&mut self.ch.hit) != 0 {
            let off = self.ch.hit.offset;
            self.ch.pos = ee::vadd(self.ch.pos, off);
            self.ch.pos_p = ee::vadd(self.ch.pos_p, off);
            self.ch.hit.pos = self.ch.pos;
            ctx.hits.sync(&self.ch.hit);
            self.body_hit_flag = 1;
            self.body_hit_cnt = self.body_hit_cnt.wrapping_add(1);
        } else {
            self.body_hit_flag = 0;
            self.body_hit_cnt = 0;
        }
        if self.evoflag != 0 {
            return;
        }
        match self.dmylevel {
            0 | 1 => {
                self.forward_a();
                self.note_process(0);
                // ccChar::Draw.
                if self.ch.fade(&ctx.view) && self.has_a {
                    self.drawn.push((0, self.ch.transparency, true));
                }
            }
            2 | 3 => {
                self.forward_b();
                self.note_process(1);
                self.drawn.push((1, self.alpha_b, true));
                if self.food_mode != 0 && self.chat_flag != 0 {
                    if self.chatcnt == 0 && self.msg_num >= 3 {
                        let k = ((i32::from(self.msg_num) - 3) / 2) as usize;
                        self.events.push(GruntyEvent::Chat(k));
                    }
                    self.chatcnt += 1;
                    if self.chatcnt >= 211 {
                        self.chatcnt = 0;
                    }
                }
            }
            4 => {
                self.note_process(2);
                self.forward_c();
                self.drawn.push((2, ONE, true));
            }
            _ => {}
        }
    }

    /// `ccSetDirc(&dirc.z, plDirc, 64)`: turning to face Kite.
    fn face(&mut self) {
        let mut z = self.ch.dirc[2];
        set_dirc(&mut z, self.entry.pl_dirc, 64);
        self.ch.dirc[2] = z;
    }

    /// The heading turned a sixteenth of the way toward `target`
    /// (`ccGetDircChg(.., 256)`); true when it faces within 4096 of it.
    fn turn_to(&mut self, dir: F) -> bool {
        let s1 = ee::rad2deg(self.ch.dirc[2]);
        let s2 = ee::rad2deg(dir);
        let s1 = (i32::from(s1) + get_dirc_chg(s1, s2, 256)) as i16;
        self.ch.dirc[2] = ee::deg2rad(s1);
        ((i32::from(s2 as u16) - i32::from(s1 as u16) + 4096) & 0xffff) < 8192
    }

    /// `ccPGuso::move` (gcmn 0x0050bca0): along the route; at a dummy (or
    /// stuck) `rand() & 3`: 0 or 3 rests (act 1) and heads for the same
    /// one after, 1 or 2 moves on that many.
    fn walk(&mut self, ctx: &mut GruntyCtx) {
        let pos = self.ch.pos;
        let dist = get_dist3d(pos, self.next_pos);
        let mut dir = get_dirc(pos, self.next_pos);
        if self.turn_to(dir) {
            if !le(dist, ARRIVE) && self.pos_cnt < 121 {
                if self.body_hit_flag != 0 {
                    let c = self.body_hit_cnt;
                    if c < 40 {
                        dir = sub(dir, HALF);
                    } else if (60..100).contains(&c) {
                        dir = add(dir, HALF);
                    } else if c >= 120 {
                        self.body_hit_cnt = 0;
                    }
                }
                let v = if self.act == 3 { WALK } else { RUN };
                self.ch.pos[0] = add(self.ch.pos[0], mul(v, sinf(dir)));
                self.ch.pos[1] = sub(self.ch.pos[1], mul(v, cosf(dir)));
            } else {
                let r = ctx.rand.rand() & 3;
                if r == 0 || r == 3 {
                    self.act = 1;
                    self.act_process = 0;
                    self.act_cnt = 0;
                } else {
                    for _ in 0..r {
                        self.mark += 1;
                        if self.mark >= self.route.len() {
                            self.mark = 0;
                        }
                    }
                }
                self.next_pos = self.route.get(self.mark).copied().unwrap_or(self.next_pos);
                self.pos_cnt = 0;
            }
        }
        self.land(ctx);
        if self.pos_cnt == 120 {
            let far = |a: F, b: F| {
                let d = sub(a, b);
                !le(d, MOVED) || lt(d, ee::neg(MOVED))
            };
            for k in 0..3 {
                if far(self.ch.pos[k], self.old_pos[k]) {
                    self.pos_cnt = 0;
                }
            }
        }
        if self.pos_cnt == 0 {
            self.old_pos = self.ch.pos;
        }
        self.pos_cnt = self.pos_cnt.wrapping_add(1);
    }

    /// `ccTransPosW2M`, `ccLandHitCheck(0x20000002)` and the attribute.
    fn land(&mut self, ctx: &mut GruntyCtx) {
        let mut p = self.ch.pos;
        p[3] = ONE;
        self.ch.pos[2] = ctx.hits.land(p, LAND_MASK);
        self.ch.hit_attribute = ctx.hits.attribute();
    }

    /// act 6 (main 0x0050b5c8): eating, then growing or waiting.
    fn eat(&mut self, ctx: &mut GruntyCtx) {
        match self.act_process {
            0 => {
                self.change_anm(2, self.dmylevel);
                self.act_process += 1;
                self.act_old = self.act;
            }
            1 => {
                self.move_cam();
                let t = if self.grow.level < 2 { self.frame_a() } else { self.frame_b() };
                if matches!(t, 110 | 115 | 120) {
                    self.burp_eff();
                }
                if t >= 130 {
                    self.act_process = 3;
                    self.param_calc(ctx);
                }
            }
            2 => self.evo_act(ctx),
            3 => {
                self.growth_num = 0;
                let d = self.tables.cam_view[self.disc_num.clamp(0, 3) as usize];
                for (c, &t) in self.cam_view.iter_mut().zip(d.iter()).take(3) {
                    *c = set_dist(*c, t, TENTH);
                }
                self.events.push(GruntyEvent::CamView(self.cam_view));
                self.chat_flag = 1;
            }
            _ => {}
        }
    }

    /// `ccPGuso::moveCam` (gcmn 0x00510490): the fixed camera's view eased
    /// toward the Grunty (level 0 once more from its feet +50).
    fn move_cam(&mut self) {
        let p = self.ch.pos;
        match self.grow.level {
            0 | 1 => {
                if self.grow.level == 0 {
                    self.cam_view[0] = set_dist(self.cam_view[0], p[0], TENTH);
                    self.cam_view[1] = set_dist(self.cam_view[1], p[1], TENTH);
                    self.cam_view[2] = set_dist(self.cam_view[2], add(0x4248_0000, p[2]), TENTH);
                }
                self.cam_view[0] = set_dist(self.cam_view[0], sub(p[0], 0x4120_0000), TENTH);
                self.cam_view[1] = set_dist(self.cam_view[1], p[1], TENTH);
                self.cam_view[2] = set_dist(self.cam_view[2], add(0x4248_0000, p[2]), TENTH);
            }
            2 | 3 => {
                self.cam_view[0] = set_dist(self.cam_view[0], sub(p[0], 0x41a0_0000), TENTH);
                self.cam_view[1] = set_dist(self.cam_view[1], p[1], TENTH);
                self.cam_view[2] = set_dist(self.cam_view[2], add(0x42a0_0000, p[2]), TENTH);
            }
            _ => {}
        }
        self.events.push(GruntyEvent::CamView(self.cam_view));
    }

    /// `ccPGuso::paramCalc` (gcmn 0x0050c0f0): the food's effect on the
    /// record, the level it has reached (growing up: evolevel, act 6's
    /// proccess 2; grown: act 7), the food it asks for next.
    fn param_calc(&mut self, ctx: &mut GruntyCtx) {
        let k = self.food_kind.clamp(0, 15) as usize;
        let n = self.food_num;
        let f = FOOD_TBL[k];
        let g = &mut self.grow;
        g.size = g.size.wrapping_add(f[0].wrapping_mul(n));
        g.smell = g.smell.wrapping_add(f[1].wrapping_mul(n));
        g.crooked = g.crooked.wrapping_add(f[2].wrapping_mul(n));
        g.cruel = g.cruel.wrapping_add(f[3].wrapping_mul(n));
        g.iq = g.iq.wrapping_add(f[4].wrapping_mul(n));
        g.pure = g.pure.wrapping_add(f[5].wrapping_mul(n));
        // The town's row and sizes (towns 1-4).
        let t = (self.town.clamp(1, 4) - 1) as usize;
        let adult = ADULT_PARAM[t];
        let sizes = SIZE_PARAM[t];
        let stats = self.grow.stats();
        for k in 0..5 {
            self.gp_sub[k] = adult[2 + k].wrapping_sub(stats[k]);
        }
        self.change_anm(0, self.dmylevel);
        let size = self.grow.size;
        let level = self.grow.level;
        let mut grown = false;
        if size >= sizes[0] && size < sizes[1] {
            if level <= 0 {
                self.evolve(1);
            }
        } else if size >= sizes[1] && size < sizes[2] {
            if level < 2 {
                self.evolve(2);
            }
        } else if size >= sizes[2] && size < sizes[3] {
            if level < 3 {
                self.evolve(3);
            }
        } else if size >= sizes[3] {
            grown = true;
        }
        if grown {
            let near = self.gp_sub.iter().filter(|&&d| (-7..8).contains(&d)).count();
            if near == 5 {
                self.adult_type = 1;
            } else {
                let far = self.gp_sub.iter().any(|&d| !(-10..11).contains(&d));
                self.adult_type = if far { 0 } else { 2 };
            }
            self.adult_setup(ctx);
            self.act_process = 0;
            self.act = 7;
            self.evonum = 0;
        }
        // The food it asks for: the stat furthest from the grown one's.
        let d: [i16; 5] = std::array::from_fn(|k| adult[2 + k].wrapping_sub(stats[k]));
        let mut best = d[0].wrapping_abs();
        let mut s1 = 0usize;
        for (v, &x) in d.iter().enumerate().skip(1) {
            let a = x.wrapping_abs();
            if best < a {
                best = a;
                s1 = v;
            }
        }
        if best < 7 {
            self.msg_num = 33;
        } else {
            self.msg_num = 3;
            let r = (ctx.rand.rand() & 3) as usize;
            let food = if d[s1] >= 0 { NEED_FOOD_1[s1][r] } else { NEED_FOOD_2[s1][r] };
            self.msg_num = self.msg_num.wrapping_add((food - 1).wrapping_mul(2));
        }
        self.grow.food_num = i32::from(self.msg_num);
        self.save_grow(ctx.save);
    }

    /// paramCalc's growing up: evolevel `n`, act 6's proccess 2.
    fn evolve(&mut self, n: i32) {
        self.evolevel = n;
        self.evonum = 0;
        self.evocnt = 0;
        self.act_process = 2;
    }
}

impl Grunty {
    /// `dogAction` (gcmn 0x0050f390), a young one's `affectFunc`: 14 the
    /// menu opens (it sits facing Kite), 15 a line (it sits up; its line),
    /// 0 the menu shuts (it walks on), 11 the fixed camera for eating
    /// (Kite and it put at their places) or back, 19 it eats food `arg1`,
    /// `arg2` of it. Nothing while act 8.
    pub fn dog_action(&mut self, cmd: i16, arg1: i16, arg2: i16, player_dirc: V4) {
        self.affect_type = cmd;
        match cmd {
            15 if self.act != 8 => {
                self.act = 2;
                self.act_process = 0;
                self.msg_num = if self.grow.level == 4 {
                    7
                } else if self.grow.size == 0 {
                    2
                } else {
                    self.grow.food_num as i8
                };
                self.chat_flag = 0;
                self.events.push(GruntyEvent::ChatClose);
            }
            14 if self.act != 8 => {
                self.act = 0;
                self.act_process = 0;
                self.sel_food = 0;
                if self.grow.level == 4 {
                    self.msg_num = 0;
                }
                self.chat_flag = 0;
            }
            0 if self.act != 8 => {
                self.act = 3;
                self.act_process = 0;
                self.events.push(GruntyEvent::ChatClose);
                self.msg_num = 0;
            }
            11 => self.fixed_camera(player_dirc),
            19 if self.act != 8 => {
                self.act = 6;
                self.act_process = 0;
                self.food_kind = arg1;
                self.food_num = arg2;
                self.growth_num = 1;
                self.chat_flag = 0;
                self.events.push(GruntyEvent::ChatClose);
                self.chatcnt = 0;
            }
            _ => {}
        }
    }

    /// Affect 11 of `dogAction` and `dogActionAdult`: the first time the
    /// fixed camera (`changeCamera(3)`, camPosDef and camViewDef by
    /// discNum), act 4, the Grunty at inuPos and Kite at playerPos facing
    /// each other; the second time the field camera back, and a grown one
    /// that ran off (evonum 7, level 4) off the command lists.
    fn fixed_camera(&mut self, player_dirc: V4) {
        let d = self.disc_num.clamp(0, 3) as usize;
        if self.cam_flag == 0 {
            self.cam_pos = self.tables.cam_pos[d];
            self.cam_view = self.tables.cam_view[d];
            self.events.push(GruntyEvent::Camera(3));
            self.events.push(GruntyEvent::CamPos(self.cam_pos));
            self.events.push(GruntyEvent::CamView(self.cam_view));
            self.cam_flag = 1;
            self.act = 4;
            let inu = self.tables.inu_pos[d];
            let pl = self.tables.player_pos[d];
            self.ch.pos = inu;
            self.ch.hit.pos = inu;
            let _ = player_dirc;
            self.events.push(GruntyEvent::Player { pos: pl, dirc_z: get_dirc0(pl, inu) });
            self.ch.dirc[2] = get_dirc0(inu, pl);
        } else {
            self.events.push(GruntyEvent::Camera(1));
            self.cam_flag = 0;
            if self.evonum == 7 && self.grow.level == 4 {
                self.entry.delete_cmnd(true);
                self.events.push(GruntyEvent::Dropped);
            }
        }
    }

    /// `dogAction2` (gcmn 0x0050f720), a grown one's: 15 a line from its
    /// row's four (`ccRand() & 3`), 14 line 7; the rest nothing.
    pub fn dog_action2(&mut self, cmd: i16, mt: &mut Mt) {
        self.affect_type = cmd;
        match cmd {
            15 => {
                let k = self.inu_id - ROW_ADULT;
                self.msg_num = if (0..9).contains(&k) { ADULT_LINES[k as usize][(mt.rand() & 3) as usize] } else { 7 };
            }
            14 => self.msg_num = 7,
            _ => {}
        }
    }

    /// `dogActionAdult` (gcmn 0x0050f9d0), a new grown one's: 15 (not act
    /// 8) sits up with a line of its kind's four, 14 sits (line 7), 0 walks
    /// on, 11 as `dogAction`'s.
    pub fn dog_action_adult(&mut self, cmd: i16, mt: &mut Mt, player_dirc: V4) {
        self.affect_type = cmd;
        match cmd {
            15 if self.act != 8 => {
                self.act = 2;
                self.act_process = 0;
                let k = self.local_id;
                self.msg_num = if (0..9).contains(&k) { GROWN_LINES[k as usize][(mt.rand() & 3) as usize] } else { 7 };
            }
            14 if self.act != 8 => {
                self.act = 0;
                self.act_process = 0;
                self.sel_food = 0;
                self.msg_num = 7;
            }
            0 if self.act != 8 => {
                self.act = 3;
                self.act_process = 0;
            }
            11 => self.fixed_camera(player_dirc),
            _ => {}
        }
    }

    /// The Grunty's `affectFunc` for the menu's command.
    pub fn affect(&mut self, cmd: i16, arg1: i16, arg2: i16, mt: &mut Mt, player_dirc: V4) {
        match self.affect_fn {
            AffectFn::Young => self.dog_action(cmd, arg1, arg2, player_dirc),
            AffectFn::Grown => self.dog_action2(cmd, mt),
            AffectFn::NewGrown => self.dog_action_adult(cmd, mt, player_dirc),
        }
    }

    /// `ccPGuso::burpEff` (gcmn 0x0050ff30): smoke from its mouth by level.
    fn burp_eff(&mut self) {
        let (y, z, sc): (F, F, F) = match self.grow.level {
            0 => (0xc248_0000, 0x4234_0000, ONE),
            1 => (0xc2a0_0000, 0x425c_0000, 0x3f99_999a),
            2 => (0xc2a0_0000, 0x4282_0000, 0x4000_0000),
            3 => (0xc2c8_0000, 0x42aa_0000, 0x4020_0000),
            _ => return self.burp_with(0, 0, 0),
        };
        self.burp_with(y, z, sc);
    }

    fn burp_with(&mut self, y: F, z: F, sc: F) {
        let m = crate::evcam::rot_z(&UNIT, self.ch.dirc[2]);
        let off = ee::apply(&m, [0, y, z, ONE]);
        let p = self.ch.pos;
        let pos = [add(p[0], off[0]), add(p[1], off[1]), add(p[2], off[2]), ONE];
        let v = ee::apply(&m, [0, 0xc000_0000, HALF, ONE]);
        self.events.push(GruntyEvent::Smoke { pos, v, scale: sc, life: 5, kind: 114 });
    }

    /// `ccPGuso::evoEff` (gcmn 0x005100e0): smoke round it as it grows.
    fn evo_eff(&mut self, mt: &mut Mt) {
        let m = crate::evcam::rot_z(&UNIT, self.ch.dirc[2]);
        let (y, z): (F, F) = match self.grow.level {
            0 => (0xc2c8_0000, 0x41c8_0000),
            1 => (0xc2c8_0000, 0x4248_0000),
            2 => (0xc2f0_0000, 0x428c_0000),
            3 => (0xc32a_0000, 0x428c_0000),
            _ => (0, 0),
        };
        let x = sub(rand_f(mt, 0x427c_0000, 0x41f8_0000), 0x4200_0000);
        let off = ee::apply(&m, [x, y, z, ONE]);
        let p = self.ch.pos;
        let pos = [add(p[0], off[0]), add(p[1], off[1]), add(p[2], off[2]), ONE];
        let v = ee::apply(&m, [0, HALF, 0x4040_0000, ONE]);
        self.events.push(GruntyEvent::Smoke { pos, v, scale: 0x4080_0000, life: 10, kind: 109 });
    }

    /// `ccPGuso::effect` (gcmn 0x005102a0), from `inuCheckNote` on a step
    /// (param 0 or 1) of `pgPtr`: a grown one running off kicks up dust.
    pub fn step_effect(&mut self, rand: &mut crate::Rand) {
        if le(self.ch.transparency, K005) || self.grow.level < 4 || self.evonum != 6 {
            return;
        }
        self.speed = 0x4120_0000;
        let speed = self.speed;
        let m = crate::evcam::rot_z(&UNIT, self.ch.dirc[2]);
        let off = ee::apply(&m, [0, 0x4348_0000, 0x4248_0000, ONE]);
        let p = self.ch.pos;
        let pos = [add(p[0], off[0]), add(p[1], off[1]), add(p[2], off[2]), ONE];
        let r = rand.rand() % 5;
        let y = mul(TENTH, mul(mul(0x3d99_999a, mul(0xbf80_0000, speed)), from_int(10 - r)));
        let v = ee::apply(&m, [0, y, 0, ONE]);
        self.events.push(GruntyEvent::Smoke { pos, v, scale: 0x4040_0000, life: 10, kind: 1 });
        self.events.push(GruntyEvent::Se3d { n: 53, pos: self.ch.pos });
    }

    /// `ccPGuso::pgDead` (gcmn 0x0050e990): a puff as a grown one that ran
    /// off flattens away.
    fn pg_dead(&mut self, rand: &mut crate::Rand) {
        let m = crate::evcam::rot_z(&UNIT, self.ch.dirc[2]);
        let p = self.ch.pos;
        let pos = [p[0], p[1], add(p[2], 0x4220_0000), ONE];
        let r = rand.rand() % 5;
        let y = mul(TENTH, mul(mul(0x3d99_999a, mul(0xbf80_0000, self.speed)), from_int(10 - r)));
        let v = ee::apply(&m, [0, y, 0, ONE]);
        self.events.push(GruntyEvent::Smoke { pos, v, scale: 0x4120_0000, life: 30, kind: 1 });
    }
}

impl Grunty {
    /// `ccPGuso::evoAct` (gcmn 0x0050c8f0): by evolevel.
    fn evo_act(&mut self, ctx: &mut GruntyCtx) {
        match self.evolevel {
            1 => self.evo_chibi2(ctx),
            2 => self.evo_pon1(ctx),
            3 => self.evo_pon2(ctx),
            _ => {}
        }
    }

    /// The scale's stretch while growing: y up by 0.01 and z by `up` for
    /// the first 30 frames, then y up by 0.01 and z down by `down`.
    fn stretch(&mut self, t: u32, up: F, down: F) {
        if t < 30 {
            self.scale[1] = add(self.scale[1], K001);
            self.scale[2] = add(self.scale[2], up);
        } else {
            self.scale[1] = add(self.scale[1], K001);
            self.scale[2] = sub(self.scale[2], down);
        }
    }

    /// pgScale for all three axes.
    fn set_scale(&mut self, s: F) {
        self.pg_scale = s;
        self.scale[0] = s;
        self.scale[1] = s;
        self.scale[2] = s;
    }

    fn evolve_pos(&self) -> V4 {
        self.ch.pos
    }

    /// `ccPGuso::evoActChibi2` (gcmn 0x0050c960): level 0 to 1 (Little
    /// Grunty, row 155), on the main anm.
    fn evo_chibi2(&mut self, ctx: &mut GruntyCtx) {
        match self.evonum {
            0 => {
                self.dmylevel = 1;
                self.set_base(155);
                self.inu_id = 155;
                // changeAnmPG(4, pgPtr->dmylevel): pgPtr is this young one.
                self.change_anm(4, self.dmylevel);
                self.evonum += 1;
                self.events.push(GruntyEvent::Voice { grp: -2, n: 37 });
            }
            1 => {
                let t = self.frame_a();
                if t >= 60 {
                    self.grow.level = self.dmylevel;
                    self.change_anm(0, self.dmylevel);
                    self.set_scale(0x3fc0_0000);
                    self.save_grow(ctx.save);
                    self.evonum += 1;
                    self.evocnt = 0;
                } else {
                    self.stretch(t, K004, K002);
                    if self.frame_a() == 30 {
                        self.events.push(GruntyEvent::Evolve { pos: self.evolve_pos(), height: self.ch.height });
                    }
                }
            }
            2 => {
                let c = self.evocnt;
                self.evocnt += 1;
                if c >= 91 {
                    self.act_process = 3;
                    self.evonum += 1;
                }
            }
            _ => {}
        }
    }

    /// The cross-fade `evoActPon1` and `evoActPon2` draw while the old body
    /// becomes the new: the main anm at transrate[0], every fifth count
    /// the draw environment's +0xa4 byte set and anmB at transrate[1].
    fn cross_fade(&mut self) {
        self.drawn.push((0, self.transrate[0], true));
        if self.evocnt % 5 == 0 {
            self.alpha_b = self.transrate[1];
            self.drawn.push((1, self.alpha_b, false));
        }
        self.evocnt = (self.evocnt + 1) & 0xff;
    }

    /// Both anms stepped as `evoActPon1`/`2` draw: the main one with the
    /// scale, anmB without.
    fn step_both(&mut self) {
        self.forward_a();
        self.note_process(0);
        self.forward_b();
        self.note_process(1);
    }

    /// `ccPGuso::evoActPon1` (gcmn 0x0050cbb0): level 1 to 2 (Grunty the
    /// Kid, row 156), from the main anm's body to anmB's. Its phase 2
    /// (the camera turning about the Grunty, printing "DEG :") is never
    /// reached: phase 1 goes on to 3.
    fn evo_pon1(&mut self, ctx: &mut GruntyCtx) {
        match self.evonum {
            0 => {
                self.set_base(156);
                self.inu_id = 156;
                self.anm_tbl = AnmTbl::Young(0);
                self.change_anm(4, 0);
                self.evonum += 1;
                self.transrate = [self.ch.transparency, 0];
                self.change_anm(4, 2);
                self.evocnt = 11;
                self.evoflag = 1;
                self.events.push(GruntyEvent::Voice { grp: -3, n: 37 });
            }
            1 => {
                let t = self.frame_a();
                if t >= 60 {
                    self.evonum = 3;
                    self.dmylevel = 2;
                    self.grow.level = 2;
                    self.change_anm(0, 2);
                    self.set_scale(ONE);
                    self.save_grow(ctx.save);
                    self.evocnt = 0;
                    self.transrate = [0, ONE];
                } else {
                    self.stretch(t, K002, K001);
                    let t = self.frame_a();
                    if t >= 36 && t.is_multiple_of(3) {
                        self.evo_eff(ctx.mt);
                    }
                    if self.frame_a() == 30 {
                        self.events.push(GruntyEvent::Evolve { pos: self.evolve_pos(), height: self.ch.height });
                    }
                }
            }
            3 => {
                let c = self.evocnt;
                self.evocnt += 1;
                if c >= 91 {
                    self.evonum += 1;
                }
            }
            4 => {
                self.ease_camera();
                self.evoflag = 0;
                self.growth_num = 0;
                self.act_process = 3;
                self.chat_flag = 1;
            }
            _ => {}
        }
        self.step_both();
        match self.evonum {
            0 => {
                if self.ch.fade(&ctx.view) && self.has_a {
                    self.drawn.push((0, self.ch.transparency, true));
                }
            }
            1 => self.cross_fade(),
            2..=4 => {
                self.alpha_b = self.transrate[1];
                self.drawn.push((1, self.alpha_b, false));
            }
            _ => {}
        }
    }

    /// The camera eased back to the town's fixed view and place.
    fn ease_camera(&mut self) {
        let d = self.disc_num.clamp(0, 3) as usize;
        let (v, p) = (self.tables.cam_view[d], self.tables.cam_pos[d]);
        for (c, &t) in self.cam_view.iter_mut().zip(v.iter()).take(3) {
            *c = set_dist(*c, t, TENTH);
        }
        self.events.push(GruntyEvent::CamView(self.cam_view));
        for (c, &t) in self.cam_pos.iter_mut().zip(p.iter()).take(3) {
            *c = set_dist(*c, t, TENTH);
        }
        self.events.push(GruntyEvent::CamPos(self.cam_pos));
    }

    /// `ccPGuso::evoActPon2` (gcmn 0x0050d5a0): to level 3 (row 157): from
    /// level 2 on anmB (phases 1, 2), or from level 0 or 1 by the main
    /// anm's cross-fade (phases 10, 20).
    fn evo_pon2(&mut self, ctx: &mut GruntyCtx) {
        match self.evonum {
            0 => {
                self.set_base(157);
                self.inu_id = 157;
                if self.grow.level < 2 {
                    self.anm_tbl = AnmTbl::Young(0);
                    self.change_anm(4, 0);
                    self.evoflag = 1;
                    self.evonum = 10;
                } else {
                    self.anm_tbl = AnmTbl::Young(2);
                    self.change_anm(4, 2);
                    self.evonum += 1;
                }
                self.transrate = [self.ch.transparency, 0];
                self.change_anm(4, 2);
                self.evocnt = 11;
                self.events.push(GruntyEvent::Voice { grp: -3, n: 37 });
            }
            1 => {
                let young = self.grow.level < 2;
                let t = if young { self.frame_a() } else { self.frame_b() };
                if t >= 60 {
                    self.dmylevel = 3;
                    self.grow.level = 3;
                    self.change_anm(0, 3);
                    self.set_scale(0x3f99_999a);
                    self.save_grow(ctx.save);
                    self.evonum += 1;
                    self.evocnt = 0;
                } else {
                    self.stretch(t, K004, K002);
                    let t = if young { self.frame_a() } else { self.frame_b() };
                    if t >= 31 && t & 1 != 0 {
                        self.evo_eff(ctx.mt);
                    }
                    let t = if young { self.frame_a() } else { self.frame_b() };
                    if t == 30 {
                        self.events.push(GruntyEvent::Evolve { pos: self.evolve_pos(), height: self.ch.height });
                    }
                }
            }
            2 => {
                let c = self.evocnt;
                self.evocnt += 1;
                if c >= 91 {
                    self.act_process = 3;
                    self.evonum += 1;
                }
            }
            10 => {
                let t = self.frame_a();
                if t >= 60 {
                    self.evonum = 20;
                    self.dmylevel = 3;
                    self.grow.level = 3;
                    self.change_anm(0, 3);
                    self.set_scale(0x3f99_999a);
                    self.save_grow(ctx.save);
                    self.evoflag = 0;
                    self.transrate = [0, ONE];
                    self.alpha_b = ONE;
                } else {
                    if t < 30 {
                        self.scale[1] = add(self.scale[1], K002);
                        self.scale[2] = add(self.scale[2], K005);
                    } else {
                        self.scale[1] = add(self.scale[1], K001);
                        self.scale[2] = sub(self.scale[2], K002);
                    }
                    let t = self.frame_a();
                    if t >= 31 && t & 1 != 0 {
                        self.evo_eff(ctx.mt);
                    }
                    if self.frame_a() == 30 {
                        self.events.push(GruntyEvent::Evolve { pos: self.evolve_pos(), height: self.ch.height });
                    }
                }
            }
            20 => {
                self.evoflag = 0;
                self.growth_num = 0;
                self.act_process = 3;
                self.chat_flag = 1;
                return;
            }
            _ => {}
        }
        if self.evoflag == 0 {
            return;
        }
        self.step_both();
        match self.evonum {
            0 => {
                if self.ch.fade(&ctx.view) && self.has_a {
                    self.drawn.push((0, self.ch.transparency, true));
                }
            }
            10 => self.cross_fade(),
            2 | 3 => {
                self.alpha_b = self.transrate[1];
                self.drawn.push((1, self.alpha_b, false));
            }
            _ => {}
        }
    }

    /// The anm of the young body the Grunty was (main below level 2, else
    /// anmB): its frame.
    fn young_frame(&self) -> u32 {
        if self.grow.level < 2 { self.frame_a() } else { self.frame_b() }
    }

    /// `ccPGuso::adultSetup` (gcmn 0x0050eac0): which grown kind (by
    /// adultType: 0 kind 0, 1 kind town * 2 - 1, 2 kind town * 2), its
    /// base, the record's kind set (exist when it already was), its body
    /// and anmC, the record saved and `inuCount[kind]` counted.
    fn adult_setup(&mut self, ctx: &mut GruntyCtx) {
        let slot = match self.adult_type {
            0 => {
                self.local_id = 0;
                Some(0)
            }
            1 => {
                self.local_id = self.town * 2 - 1;
                Some(1)
            }
            2 => {
                self.local_id = self.town * 2;
                Some(2)
            }
            _ => None,
        };
        if let Some(k) = slot {
            self.set_base(ROW_ADULT + self.local_id);
            self.grow.ty[k] = 1;
            if Growth::read(ctx.save, self.town).ty[k] == 1 {
                self.exist = 1;
            }
        }
        let local = self.local_id.clamp(0, 8) as usize;
        self.body_c = self.archive.as_ref().and_then(|a| Body::read(a, CCS_ADULT[local], TRALL).ok()).map(Rc::new);
        self.anm_tbl = AnmTbl::Adult(local);
        self.play_c = match (&self.body_c, self.anm_tbl.name(0)) {
            (Some(b), Some(n)) => b.play(&n),
            _ => None,
        };
        self.del_flag = 1;
        self.save_grow(ctx.save);
        let at = SAVE_INU_COUNT + 2 * local;
        let n = ctx.save.i16(at).wrapping_add(1).min(10000);
        ctx.save.set_i16(at, n);
    }

    /// `ccPGuso::evoActAdult` (gcmn 0x0050dd30): the young one grows into
    /// the grown kind adultSetup chose, speaks (growthNum 3) until the menu
    /// is done with it (4); a kind the town has already runs off (exist:
    /// to a far place, flattening away) and is gone (growthNum 5), a new
    /// one walks the young one's route with `dogActionAdult`.
    fn evo_adult(&mut self, ctx: &mut GruntyCtx) {
        let local = self.local_id.clamp(0, 8) as usize;
        let voice = |n: i32| GruntyEvent::Voice { grp: ROW_ADULT + local as i32, n };
        match self.evonum {
            0 => {
                if self.grow.level < 2 {
                    self.change_anm(0, 0);
                } else {
                    self.change_anm(0, 2);
                }
                self.evonum += 1;
                self.evocnt = 0;
            }
            1 => {
                if self.evocnt == 30 {
                    self.growth_num = 2;
                }
                if self.evocnt >= 121 {
                    if self.grow.level < 2 {
                        self.change_anm(4, 0);
                    } else {
                        self.change_anm(4, 2);
                    }
                    self.evonum += 1;
                    if self.local_id < 9 && self.local_id >= 0 {
                        self.events.push(voice(VOICE_GROW[local]));
                    }
                }
                if self.evocnt == 20 {
                    self.events.push(GruntyEvent::Grow { pos: self.ch.pos, height: self.ch.height });
                }
                self.evocnt += 1;
            }
            2 => {
                let t = self.young_frame();
                if t >= 60 {
                    self.dmylevel = 4;
                    self.grow.level = 4;
                    self.change_anm(0, 4);
                    self.set_scale(ONE);
                    self.evonum += 1;
                    self.msg_num = if self.exist != 0 {
                        5
                    } else if ctx.save.u8(IMP_ITEM_LIST + FLUTE) != 0 {
                        FLUTE_LINES[local]
                    } else {
                        1
                    };
                    self.events.push(GruntyEvent::VoiceStop);
                    self.growth_num = 3;
                    self.evocnt = 0;
                    self.save_grow(ctx.save);
                } else {
                    self.stretch(t, K004, K002);
                    if self.young_frame() == 50 {
                        self.events.push(GruntyEvent::Evolve { pos: self.ch.pos, height: self.ch.height });
                    }
                }
            }
            3 => {
                let c = self.evocnt;
                self.evocnt += 1;
                if c >= 16 {
                    self.evonum += 1;
                }
            }
            4 => {
                if self.growth_num == 4 {
                    self.msg_num = 0;
                    if self.exist != 0 {
                        self.evonum += 1;
                        self.next_pos = if self.town == 3 {
                            [0x4553_d4a4, 0xc602_86ae, 0x4495_d052, self.next_pos[3]]
                        } else {
                            [0xc481_8d71, 0xc605_110a, 0x4495_d052, self.next_pos[3]]
                        };
                        self.evocnt = 0;
                        self.change_anm(3, self.grow.level);
                    } else {
                        self.act = 3;
                        self.affect_fn = AffectFn::NewGrown;
                    }
                }
            }
            5 => {
                let dir = get_dirc(self.ch.pos, self.next_pos);
                let _ = self.turn_to(dir);
                self.evocnt += 1;
                if self.evocnt == 60 {
                    self.change_anm(1, self.grow.level);
                    if (0..9).contains(&self.local_id) {
                        self.events.push(voice(VOICE_TURN[local]));
                    }
                }
                if self.evocnt >= 151 {
                    self.evonum += 1;
                    self.change_anm(2, self.grow.level);
                    self.evocnt = 0;
                    self.spdeg = 0;
                }
            }
            6 => {
                self.run_away(ctx);
                let p = self.ch.pos;
                for (c, &t) in self.cam_view.iter_mut().zip(p.iter()).take(3) {
                    *c = set_dist(*c, t, TENTH);
                }
                self.events.push(GruntyEvent::CamView(self.cam_view));
                if self.evocnt >= 81 {
                    self.scale[1] = sub(self.scale[1], TENTH);
                    if lt(self.scale[1], 0) {
                        self.scale[1] = 0;
                        if self.evocnt % 4 == 0 {
                            self.pg_dead(ctx.rand);
                        }
                    }
                }
                if self.evocnt == 50 && (0..9).contains(&self.local_id) {
                    self.events.push(voice(VOICE_RUN[local]));
                }
                let c = self.evocnt;
                self.evocnt += 1;
                if c >= 121 {
                    self.evonum += 1;
                    self.growth_num = 5;
                    self.evocnt = 0;
                }
            }
            _ => {}
        }
    }

    /// `ccPGuso::runAway` (gcmn 0x0050e820): turning toward nextPos and
    /// running at 12 along the way to it, bobbing 20 by `spdeg`.
    fn run_away(&mut self, ctx: &mut GruntyCtx) {
        let dir = get_dirc(self.ch.pos, self.next_pos);
        let _ = self.turn_to(dir);
        self.ch.pos[0] = add(self.ch.pos[0], mul(RUN, sinf(dir)));
        self.ch.pos[1] = sub(self.ch.pos[1], mul(RUN, cosf(dir)));
        let b = sinf(ee::deg2rad(self.spdeg as i16));
        self.ch.pos[2] = add(self.ch.pos[2], mul(0x41a0_0000, b));
        self.spdeg = self.spdeg.wrapping_add(64);
        if self.spdeg > 0x8000 {
            self.spdeg = 0x8000;
        }
        self.ch.hit_attribute = ctx.hits.attribute();
    }

    /// `ccPGuso::adultMain` (gcmn 0x0050bba0), act 9: a grown one at its
    /// pen: pushed out of others, its notes, its anm stepped, drawn.
    fn adult_main(&mut self, ctx: &mut GruntyCtx) {
        self.ch.hit.pos = self.ch.pos;
        if ctx.hits.collision_detection(&mut self.ch.hit) != 0 {
            let off = self.ch.hit.offset;
            self.ch.pos = ee::vadd(self.ch.pos, off);
            self.ch.pos_p = ee::vadd(self.ch.pos_p, off);
            self.ch.hit.pos = self.ch.pos;
            ctx.hits.sync(&self.ch.hit);
            self.body_hit_flag = 1;
            self.body_hit_cnt = self.body_hit_cnt.wrapping_add(1);
        } else {
            self.body_hit_flag = 0;
            self.body_hit_cnt = 0;
        }
        self.note_process(0);
        self.forward_a();
        if self.ch.fade(&ctx.view) && self.has_a {
            self.drawn.push((0, self.ch.transparency, false));
        }
    }

    /// The root matrix an anm draws at: `SetMatrix_PosRotZYXScale` (the
    /// main anm and anmC) or `SetMatrix_PosRotZYX` (anmB, and a grown
    /// one's).
    pub fn root(&self, scaled: bool) -> Mat4 {
        let r = crate::body::root(self.ch.pos, self.ch.dirc);
        if scaled {
            r * Mat4::from_scale(glam::Vec3::new(ee::f(self.scale[0]), ee::f(self.scale[1]), ee::f(self.scale[2])))
        } else {
            r
        }
    }
}

impl Grunty {
    /// One frame on the entry control's list: `ccEntryObj::routine`, then
    /// `ccPGuso::main`. The notes it passed are left in `notes` for
    /// [`inu_check_note`] with `pgPtr`.
    pub fn step(&mut self, ctx: &mut GruntyCtx) {
        self.events.clear();
        self.notes.clear();
        self.drawn.clear();
        self.entry.routine(&mut self.ch, ctx.player);
        self.main(ctx);
    }

    /// Draws what the last frame drew.
    pub fn draw(&self, layers: &mut piney_desktop::layers::Layers, to_screen: Mat4, lights: &crate::town::TownLights) {
        for &(which, alpha, scaled) in &self.drawn {
            let root = self.root(scaled);
            let shaded = self.ch.hit_attribute & crate::char::SHADED != 0;
            let a = ee::f(alpha);
            match which {
                // ccChar::Draw: its shadow alpha and length around the
                // draw.
                0 if self.has_a => crate::draw::char_shadow(layers, self.ch.shadow_t, self.ch.height, |layers| {
                    self.ch.body.draw_fog(
                        layers,
                        crate::draw::CHAR_LAYER,
                        to_screen,
                        &self.ch.play,
                        root,
                        a,
                        lights,
                        shaded,
                        &self.ch.clut_swaps,
                        self.ch.blend.into(),
                    )
                }),
                // anmB and anmC drawn by ccAnm::Draw: the environment as
                // the last ccChar::Draw left it.
                1 => {
                    if let Some(p) = &self.play_b {
                        self.body_b.draw_fog(
                            layers,
                            crate::draw::CHAR_LAYER,
                            to_screen,
                            p,
                            root,
                            a,
                            lights,
                            shaded,
                            &[],
                            crate::draw::Fogging::None,
                        );
                    }
                }
                2 => {
                    if let (Some(b), Some(p)) = (&self.body_c, &self.play_c) {
                        b.draw_fog(
                            layers,
                            crate::draw::CHAR_LAYER,
                            to_screen,
                            p,
                            root,
                            a,
                            lights,
                            shaded,
                            &[],
                            crate::draw::Fogging::None,
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

/// `inuCheckNote` (gcmn 0x0050fec0) for each note a Grunty's anms handed
/// on, on `pg` (`pgPtr`, the last Grunty made): a note of event 1 or 2
/// with param 0 or 1 is a step (`ccPGuso::effect`), a larger param a cry
/// (`ccSeSetParamInu`).
pub fn inu_check_note(notes: &[(u32, u32)], pg: &mut Grunty, rand: &mut crate::Rand) {
    for &(event, param) in notes {
        if !matches!(event, 1 | 2) {
            continue;
        }
        if param < 2 {
            pg.step_effect(rand);
        } else {
            pg.events.push(GruntyEvent::Note { param, pos: pg.ch.pos, attribute: pg.ch.hit_attribute });
        }
    }
}

impl Npc for Grunty {
    fn code(&self) -> i32 {
        self.code
    }

    fn flags(&self) -> u32 {
        self.row.flags
    }

    fn char(&self) -> &Char {
        &self.ch
    }

    fn char_mut(&mut self) -> &mut Char {
        &mut self.ch
    }

    /// The world steps Grunties itself ([`Grunty::step`]): they read
    /// `ccRand` and write the save.
    fn step(&mut self, _ctx: &mut crate::entry::NpcCtx) {}

    fn listed(&self) -> bool {
        self.entry.listed
    }
}

/// `ccRandF(x, clamp)` (main 0x001d9b20): `x * ccRand() / 2^31` in double
/// precision, kept within `clamp` either way.
pub fn rand_f(mt: &mut Mt, x: F, clamp: F) -> F {
    let r = f64::from(mt.rand());
    let v = ((f64::from(f32::from_bits(x)) * (r / 2_147_483_648.0)) as f32).to_bits();
    if lt(v, 0) {
        let c = ee::neg(clamp);
        if lt(v, c) { c } else { v }
    } else if le(v, clamp) {
        v
    } else {
        clamp
    }
}

/// Which affect function a Grunty has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AffectFn {
    /// `dogAction`: a young one.
    Young,
    /// `dogAction2`: a grown one placed by `ccSetChibiGuso`.
    Grown,
    /// `dogActionAdult`: a young one grown up in front of Kite.
    NewGrown,
}

/// `ccSetDist(&v, to, rate)` (main 0x001da5d0).
pub fn set_dist(v: F, to: F, rate: F) -> F {
    let mut x = v;
    piney_battle::geom::set_dist(&mut x, to, rate);
    x
}

/// What a Grunty's frame reads of the world.
pub struct GruntyCtx<'a> {
    /// Kite: `plw->pos`, `plw->dirc`.
    pub player: V4,
    pub player_dirc: V4,
    /// For `ccChar::Draw`'s fade.
    pub view: View,
    pub hits: &'a mut hit::Hits,
    /// newlib's `rand()` and the game's `ccRand`.
    pub rand: &'a mut crate::Rand,
    pub mt: &'a mut Mt,
    pub save: &'a mut SaveData,
}
