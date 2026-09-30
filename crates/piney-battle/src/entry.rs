//! How enemies appear in a field and a dungeon: the entry control
//! (`entctrl.cpp`, gcmn 0x0042df10-0x00431d04), the magic portal
//! (`gmcircle.cpp`, `ccMagicCircle`, 0x00454840-0x0045621c) and where the
//! portals come from (`WORLD_MAN::EntryGimmick` main 0x001a1f20,
//! `WORLD::SetMagicCircle` 0x005ab610, `DUNGEON::SetMagicCircle` 0x005bf590,
//! `ccEntryEventMng` main 0x001b62e0). [`EntryCtrl`] keeps the four lists; the
//! objects' `main`s go through [`Seam`], an enemy's through [`EnemySeam`]. The
//! rules are in docs/engine/battle.md ("Enemies appearing").

use piney_data::save::SaveData;
use piney_data::tables::types::EntryFunc;
use piney_data::volume::Volume;

use crate::affect::AffectCtx;
use crate::blocks::InfoRef;
use crate::chara::{AffectFunc, Char, Env};
use crate::enemy_ai::{self, Ai, Enemy, EntryParam, Frame};
use crate::enemy_motion::{At, Call, Motion, MotionData, MotionWorld};
use crate::geom::{self, F, V4, add, atan2f, deg2rad, div, from_int, le, lt, mul, rad2deg, set_dirc, sinf, sub};
use crate::param::{Base, Elm};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::tables::{self, Tables};
use crate::world::{AnmSlot, CharHit, Note, World};

const ONE: F = geom::ONE;
const PI: F = geom::PI;
const TWO_PI: F = geom::TWO_PI;
const HALF_PI: F = geom::HALF_PI;
const NEG_PI: F = 0xc049_0fdb;
/// `FZeroPosition` (main 0x002f7390) and `vf0`: (0, 0, 0, 1).
pub const ZERO_POS: V4 = geom::VF0;

/// `enemyTbl` rows (`ccEnemyTable[303]`).
pub const ENEMY_ROWS: usize = tables::ENEMIES;
/// `gimmickTbl`'s rows (INF gcmn 0x0061e0f0: `ccGimmickTable[45]`, 0x70
/// bytes each, the `ccGimmickParam` base, then a `ccEntry` at +0x28).
pub const GIMMICKS: usize = 45;
/// `npcTbl`'s rows (INF gcmn 0x00619460: 0x70 bytes, a `ccEntry` at +0x28).
pub const NPCS: usize = 175;
/// The gimmick id of the magic circle.
pub const MAGIC_CIRCLE: i32 = 15;
/// `ccMagicCircle`'s particles.
pub const PARTS: usize = 128;

/// `ccEntryParamClear(ep)` (gcmn 0x0042e1d0): every word -1, then type
/// and id 0 and position and heading (0, 0, 0, 1).
pub fn entry_param_clear() -> EntryParam {
    EntryParam {
        pos: ZERO_POS,
        dirc: ZERO_POS,
        ty: 0,
        id: 0,
        area: -1,
        area_num: -1,
        floor: -1,
        block: -1,
        x: -1,
        y: -1,
        ent_root: -1,
        land: -1,
        param: [-1; 4],
    }
}

// tables ------------------------------------------------------------------------------

/// A `gimmickTbl` or `npcTbl` row: the base parameters `SetBaseParam`
/// points its `ccChar.base` at, and its `ccEntry`'s constructor and `esize`
/// (how many `entryObject(ep)` makes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpawnRow {
    pub base: Base,
    pub func: Option<EntryFunc>,
    pub esize: i32,
}

/// The tables the entry control reads besides [`Tables`]: `gimmickTbl`,
/// `npcTbl` and `ccEnemyListInfo` (5 servers x 7 types of `{int *list; int
/// num}`), the volume's (`battle`).
#[derive(Clone)]
pub struct SpawnTables {
    pub volume: Volume,
    pub gimmicks: Vec<SpawnRow>,
    pub npcs: Vec<SpawnRow>,
    /// `ccEnemyListInfo[server][type]`: the `enemyTbl` rows by rank.
    pub enemy_lists: Vec<Vec<Vec<i32>>>,
}

impl SpawnTables {
    /// The volume's.
    pub fn of(volume: Volume) -> SpawnTables {
        let t = piney_data::tables::battle::of(volume);
        let row = |base, entry: &piney_data::tables::types::Entry| SpawnRow {
            base: crate::param::base_of(base),
            func: entry.func,
            esize: entry.esize,
        };
        SpawnTables {
            volume,
            gimmicks: t.gimmicks().iter().map(|g| row(&g.param.base, &g.entry)).collect(),
            npcs: t.npcs().iter().map(|n| row(&n.param.base, &n.entry)).collect(),
            enemy_lists: t
                .enemy_lists()
                .iter()
                .map(|server| server.iter().map(|l| l.list.map_or_else(Vec::new, <[i32]>::to_vec)).collect())
                .collect(),
        }
    }
}

// the registered rows ---------------------------------------------------------------

/// The rows an area may spawn: `ccRegisterEnemyTbl` (gcmn 0x006fa900,
/// `int[8]`, count main 0x00378194), their drained forms `ccRegisterDrainTbl`
/// (0x006fa920, `int[16]`, count main 0x00378198), `ccRegisterEnemyRange`
/// (main 0x00378190) and `enemyTbl[i].entry.exist`. The registration does
/// not check the counts: `cells` holds both tables, so a ninth registered row
/// lands in the drain table as it does in the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Register {
    /// `ccRegisterEnemyTbl[0..8]` then `ccRegisterDrainTbl[0..16]`.
    pub cells: [i32; 24],
    pub enemy_num: i32,
    pub drain_num: i32,
    pub range: i32,
    pub exist: Vec<i32>,
}

impl Default for Register {
    /// As `ccInitRegisterEnemy` leaves it.
    fn default() -> Self {
        Register { cells: [-1; 24], enemy_num: 0, drain_num: 0, range: 3, exist: vec![0; ENEMY_ROWS] }
    }
}

impl Register {
    /// `ccRegisterEnemyTbl[i]`.
    pub fn enemy(&self, i: i32) -> i32 {
        usize::try_from(i).ok().and_then(|i| self.cells.get(i)).copied().unwrap_or(-1)
    }

    /// `ccRegisterDrainTbl[i]`.
    pub fn drain(&self, i: i32) -> i32 {
        usize::try_from(i + 8).ok().and_then(|i| self.cells.get(i)).copied().unwrap_or(-1)
    }

    fn set_exist(&mut self, id: i32) {
        if let Some(e) = usize::try_from(id).ok().and_then(|i| self.exist.get_mut(i)) {
            *e = 1;
        }
    }

    fn push_enemy(&mut self, id: i32) {
        if let Some(c) = usize::try_from(self.enemy_num).ok().and_then(|i| self.cells.get_mut(i)) {
            *c = id;
        }
        self.enemy_num = self.enemy_num.wrapping_add(1);
    }

    /// The drained form registered once.
    fn add_drain(&mut self, t: &Tables, d: i32) {
        self.set_exist(d);
        if (0..self.drain_num).any(|k| self.drain(k) == d) {
            return;
        }
        if let Some(c) = usize::try_from(self.drain_num + 8).ok().and_then(|i| self.cells.get_mut(i)) {
            *c = d;
        }
        self.drain_num = self.drain_num.wrapping_add(1);
        let _ = t;
    }

    /// `ccInitRegisterEnemy` (gcmn 0x0042e750): no row registered, the
    /// range 3. (It also points every row's `entry.func` at its race's
    /// constructor in `ccEntryRaceTbl`, which [`crate::races::construct`]
    /// dispatches on.)
    pub fn init(&mut self) {
        *self = Register::default();
    }

    /// One row as `ccRegisterEnemyList` and `ccRegisterEnemyOne` register
    /// it: `exist`, into the table, and its drained form (a middle boss's
    /// base form and that form's drained form, `ccGetDrainId`) into the
    /// drain table.
    fn register(&mut self, t: &Tables, id: i32) {
        self.set_exist(id);
        self.push_enemy(id);
        let d2 = if crate::enemy_ai::middle_boss(t, id) >= 0 {
            let d = crate::enemy_ai::drain_id(t, id);
            self.add_drain(t, d);
            crate::enemy_ai::drain_id(t, d)
        } else {
            crate::enemy_ai::drain_id(t, id)
        };
        self.add_drain(t, d2);
    }

    /// `ccRegisterEnemyOne(id)` (gcmn 0x0042e8d0).
    pub fn register_one(&mut self, t: &Tables, id: i32) {
        self.register(t, id);
    }

    /// `ccRegisterEnemyList(server, type, rank)` (gcmn 0x0042ed70):
    /// `range` rows of `ccEnemyListInfo[server][type]` from `rank`, staying
    /// on the last past the end. From Mutation on (0x004446f0) a start too
    /// near the end moves back so that all `range` rows fit.
    pub fn register_list(&mut self, t: &Tables, st: &SpawnTables, server: i32, ty: i32, rank: i32) {
        let list = &st.enemy_lists[server as usize][ty as usize];
        let rows = list_rows(t, list, rank, self.range);
        for id in rows {
            self.register(t, id);
        }
    }
}

/// The rows `ccRegisterEnemyList` takes from `list` at `rank`: `range` of
/// them, staying on the last past the end; from Mutation on starting back
/// far enough for all to fit.
fn list_rows(t: &Tables, list: &[i32], rank: i32, range: i32) -> Vec<i32> {
    let num = list.len() as i32;
    let mut rank = rank;
    let later = t.volume != Volume::Inf;
    if later && num < rank + range {
        rank = num - range;
    }
    let mut out = Vec::new();
    for _ in 0..range {
        out.push(list[rank as usize]);
        rank += 1;
        if !later && rank == num {
            rank -= 1;
        }
    }
    out
}

/// `ccAnalyzeEnemyList(server, type, rank)` (gcmn 0x0042f290): the highest
/// level among the three rows `ccRegisterEnemyList` would register (at
/// least 0). From Mutation on (0x00444c20) the rows are the `range` it
/// would register (`ccRegisterEnemyRange`), the first three compared.
pub fn analyze_enemy_list(t: &Tables, st: &SpawnTables, server: i32, ty: i32, rank: i32, range: i32) -> i32 {
    let list = &st.enemy_lists[server as usize][ty as usize];
    let later = t.volume != Volume::Inf;
    let rows = list_rows(t, list, rank, if later { range } else { 3 });
    rows.iter()
        .take(3)
        .map(|&id| i32::from(t.enemies[id as usize].param.base.level))
        .fold(0, |m, x| if m < x { x } else { m })
}

// the save --------------------------------------------------------------------------

/// `saveData.eventEntry` (+0x40cc + 24 * field): six words of bits per story
/// field, 1 to 160.
pub const SAVE_EVENT_ENTRY: usize = 0x40cc;
/// `saveData.fountain` (+0x65dc): `int[100]`, the fountains used.
pub const SAVE_FOUNTAIN: usize = 0x65dc;
/// The circles opened (+0x6864), and the areas cleared of circles: fields
/// and the dungeons of field type 4 (+0x6866), other dungeons (+0x6868);
/// shorts held at 10000.
pub const SAVE_CIRCLES: usize = 0x6864;
pub const SAVE_FIELDS_CLEARED: usize = 0x6866;
pub const SAVE_DUNGEONS_CLEARED: usize = 0x6868;

/// The word and bit of event entry `n` (C's `/` and `%` by 32).
fn event_entry_at(field: i32, n: i32) -> Option<(usize, u32)> {
    if field == 0 || field >= 161 {
        return None;
    }
    let word = n / 32;
    if word >= 6 {
        return None;
    }
    let at = (SAVE_EVENT_ENTRY as i64 + 24 * i64::from(field) + 4 * i64::from(word)) as usize;
    Some((at, (n % 32) as u32))
}

/// `ccSaveData::CheckEventEntry(n)` (main 0x00178030): bit `n` of the story
/// field's event entries (`game.field` 1-160; 0 elsewhere).
pub fn check_event_entry(save: &SaveData, field: i32, n: i32) -> bool {
    match event_entry_at(field, n) {
        Some((at, bit)) if at + 4 <= piney_data::save::SIZE => (save.i32(at) as u32) & (1u32 << (bit & 31)) != 0,
        _ => false,
    }
}

/// `ccSaveData::ClearEventEntry(n)` (main 0x001780d0): the bit cleared.
pub fn clear_event_entry(save: &mut SaveData, field: i32, n: i32) {
    if let Some((at, bit)) = event_entry_at(field, n)
        && at + 4 <= piney_data::save::SIZE
    {
        let v = save.i32(at) as u32 & !(1u32 << (bit & 31));
        save.set_i32(at, v as i32);
    }
}

/// `ccEntryCtrl::deleteEventEntry(ep)` (gcmn 0x00430200): an object an
/// event placed (`entRoot` 0, `param[0]` an event entry) is used up: its
/// event entry is cleared.
pub fn delete_event_entry(save: &mut SaveData, field: i32, ep: &EntryParam) {
    if ep.ent_root == 0 && ep.param[0] != -1 {
        clear_event_entry(save, field, ep.param[0]);
    }
}

/// `ccSaveData::CheckFountain(code)` (main 0x001782f0): whether `code |
/// server << 29` is among the save's 100 fountains used.
pub fn check_fountain(save: &SaveData, server: i32, code: i32) -> bool {
    let key = code | server.wrapping_shl(29);
    (0..100).any(|k| save.i32(SAVE_FOUNTAIN + 4 * k) == key)
}

/// A short of the save raised by one and held at 10000.
fn bump_count(save: &mut SaveData, at: usize) {
    let v = save.i16(at).wrapping_add(1);
    save.set_i16(at, v);
    if save.i16(at) >= 10001 {
        save.set_i16(at, 10000);
    }
}

// the objects -----------------------------------------------------------------------

/// Where the game is: `game` (`ccGame`) and `worldman` as the entry control
/// reads them, and Kite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Game {
    /// `game.area` (+0x14): 0 a Root Town, 1 a field, 2 a dungeon.
    pub area: i32,
    /// `game.town`, `field`, `dungeon`, `floor`, `block` (+0x20 - +0x30).
    pub town: i32,
    pub field: i32,
    pub dungeon: i32,
    pub floor: i32,
    pub block: i32,
    /// `game.server` (+0x1c).
    pub server: i32,
    /// `WORLD_MAN::GetFieldType()` (main 0x0019f0c0).
    pub field_type: i32,
    /// `worldman->wordparam->ID`: the area's code (`CheckFountain`).
    pub area_code: i32,
    /// `worldman->wordparam->areaLevel` (+0x20): the spring's kind.
    pub area_level: i32,
    /// `plw.pw` (main 0x00730300): Kite's scene index.
    pub player: Option<usize>,
}

/// What the constructors of the objects that are not enemies leave in
/// `ccEntryObj` (and the `ccChar` and `ccGimmick` members the entry control
/// and the circle use): a gimmick's, an NPC's, a magic circle's base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryObj {
    /// `ccChar.dirc`, `transparency`, `setTransparency`.
    pub dirc: V4,
    pub transparency: F,
    pub set_transparency: F,
    /// `ccEntryObj` +0xe0: `objFlag` ... `ccs2Flag`.
    pub obj_flag: bool,
    pub init_flag: bool,
    pub freeze_flag: bool,
    pub affect_flag: bool,
    pub disp_sw: bool,
    pub dest_flag: bool,
    pub cmnd_flag: bool,
    pub ccs2_flag: bool,
    pub pl_dist: F,
    pub pl_dirc: F,
    pub alpha: i32,
    pub fade_flag: i16,
    pub fade_cnt: i16,
    pub grot_deg: i16,
    pub grot_spd: i16,
    pub ent: EntryParam,
    pub hit: CharHit,
    pub anm_tbl: u32,
    /// `destFunc`'s gcmn address, 0 for none.
    pub dest_func: u32,
    /// `ccGimmick`: `gimId`, `actNum`, `actCnt`, `anmFlag`.
    pub gim_id: i32,
    pub act_num: i32,
    pub act_cnt: i32,
    pub anm_flag: i32,
    /// What the gimmick's own class keeps (a box's or idol's rays).
    pub class: crate::gimmick::Class,
}

impl Default for EntryObj {
    /// `ccChar::ccChar`, `ccEntryObj::ccEntryObj` (0x0042f7d0) and
    /// `ccGimmick::ccGimmick` (0x004533a0): display on, alpha 128, fully
    /// opaque, no fade, the body out of the list.
    fn default() -> Self {
        EntryObj {
            dirc: [0; 4],
            transparency: ONE,
            set_transparency: ONE,
            obj_flag: false,
            init_flag: false,
            freeze_flag: false,
            affect_flag: false,
            disp_sw: true,
            dest_flag: false,
            cmnd_flag: false,
            ccs2_flag: false,
            pl_dist: 0,
            pl_dirc: 0,
            alpha: 128,
            fade_flag: 0,
            fade_cnt: 0,
            grot_deg: 0,
            grot_spd: 0,
            ent: EntryParam::default(),
            hit: CharHit { mask2: u32::MAX, ..CharHit::default() },
            anm_tbl: 0,
            dest_func: 0,
            gim_id: 0,
            act_num: 0,
            act_cnt: 0,
            anm_flag: 0,
            class: crate::gimmick::Class::None,
        }
    }
}

/// `ccEff` (0x70 bytes) as a circle's particle uses it: its position,
/// scale, rotation, colour, pattern count (`patNum`, from the effect's
/// chunk, `EFF_xmagpat1`) and transparency.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Eff {
    pub pos: V4,
    pub scale_x: F,
    pub scale_y: F,
    pub rotate: F,
    pub color: u32,
    pub pat_num: u16,
    pub transparency: F,
}

/// `ccMcPart` (0x70 bytes): one of a magic circle's particles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct McPart {
    /// `anmFlag`: its life is over.
    pub anm_flag: bool,
    /// 0 free; 1-4 the kind `createPart` made it.
    pub status: i32,
    /// `mat`: columns.
    pub mat: [V4; 4],
    pub speed: F,
    pub rad_cnt: F,
    /// The circle's `actCnt` when it was made.
    pub mc_act_cnt: i16,
    pub eff_anm_pat: u16,
    pub cnt: i16,
    pub life: i16,
    pub transparency: F,
    pub rnd: F,
    /// Its `ccEff` (made with the circle's particles).
    pub eff: Eff,
}

/// `ccMagicCircle` (0x39f0 bytes): a magic portal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MagicCircle {
    pub obj: EntryObj,
    /// `partFlag`: the 128 particles' effects exist.
    pub part_flag: bool,
    /// `partTop`: the particle `setPart` overwrites when none is free.
    pub part_top: i32,
    pub parts: Vec<McPart>,
}

/// Which of the entry control's lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Enemy = 0,
    Circle = 1,
    Gimmick = 2,
    Npc = 3,
}

/// A list: `enNum`/`enHead`/`enFoot` and the others.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct List {
    pub num: i32,
    pub head: Option<usize>,
    pub foot: Option<usize>,
}

/// What `g_entryList` keeps between areas: each list's objects in order,
/// with their characters and an enemy's state (see
/// [`EntryCtrl::keep`]).
#[derive(Clone, Debug, Default)]
pub struct KeptEntries {
    pub lists: [Vec<(Char, Obj, Option<Enemy>)>; 4],
    /// Each kept object's scene index in the area it left.
    pub olds: [Vec<usize>; 4],
}

impl KeptEntries {
    /// How many objects each list keeps.
    pub fn counts(&self) -> [usize; 4] {
        std::array::from_fn(|k| self.lists[k].len())
    }
}

/// `ccEntryObj.pre`, `next` (+0x1c0, +0x1c4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Link {
    pub pre: Option<usize>,
    pub next: Option<usize>,
}

/// The entry control's own record of an object (by scene index).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Obj {
    /// Not an entry object (Kite, the party), or deleted.
    #[default]
    None,
    /// An enemy: its state is `foes[i]`.
    Enemy,
    Circle(Box<MagicCircle>),
    Gimmick(Box<EntryObj>),
    Npc(Box<EntryObj>),
}

/// `ccEntryCtrl` (0x40 bytes): where it runs and its four lists.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryCtrl {
    /// `area`, `areaNum`, `floor`, `block`: `game.area`, the town, field or
    /// dungeon number by area, `game.floor`, `game.block` when the task
    /// started; objects of other places stay off (`initObject`).
    pub area: i32,
    pub area_num: i32,
    pub floor: i32,
    pub block: i32,
    /// Enemies, magic circles, gimmicks, NPCs.
    pub lists: [List; 4],
    /// By scene index.
    pub links: Vec<Link>,
    pub objs: Vec<Obj>,
}

/// What the entry control asks of the rest of the game, in call order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Out {
    /// `ccSeOn3D(se, pos)`: a sound at a place.
    Se { se: i32, pos: V4 },
    /// `effRemoveTrap(pos, 103, 121)`: a dead enemy's treasure box appears.
    RemoveTrap { pos: V4 },
    /// `ccStartThread(ccThDfComp, 33, 4096)`: the area's last circle opened.
    AreaCleared,
    /// `ccDeleteCmnd` on a character that was on a command list; the menu's
    /// target moves on if it was this one (`ccChangeCmndTarget`).
    DeleteCmnd(usize),
    /// `~ccEntryObj`: the class's `destFunc` (gcmn address) ran and the
    /// models were freed.
    Destroyed { who: usize, dest_func: u32 },
    /// A race constructor's `ccEnemyWeaponCtrl(n, info, this)`.
    Weapon { who: usize, info: InfoRef, n: i32 },
    /// A race constructor's `ccEnemyDustCtrl(n, info, this)`.
    Dust { who: usize, info: InfoRef, n: i32 },
    /// `ccEnemyH`'s constructor, types 3 and 4: breath `slot` (0 or 1)
    /// made (`new ccEnemyBreath`, the info's node looked up in the clump,
    /// `initBreath(info)`).
    Breath { who: usize, slot: usize, info: InfoRef },
    /// `ccEnemyG` rows 154-157: the clump duplicated and recoloured
    /// (`MAT_clut2`, `CLT_egmfrod1c1`).
    Clut { who: usize },
    /// `ccEnemyG::checkGold`: the row's `area`, `territory` and `viewRange`
    /// set to 50000 in `enemyTbl` (the runtime's table).
    GoldRanges { ene_id: i32 },
    /// The circle's 128 particle effects made (`EFF_xmagpat1`) or freed.
    CircleParts { who: usize, made: bool },
    /// `WORLD_MAN::SetActiveLayer(layer)`.
    Layer(i32),
    /// The circle's model drawn (`ccCoord::SetMatrix_PosRotZYX(pos, dirc)`,
    /// `ccAnm::Draw`) at `transparency`.
    CircleDraw { who: usize, transparency: F },
    /// `ccEff::Draw(effAnmPat)` for a particle.
    PartDraw { who: usize, part: usize, transparency: F, pat: u16 },
    /// `effAfterDrain(obj, -1)`: the drained form's effect, when
    /// [`EnemySeam`] has spawned it.
    AfterDrain { who: usize },
    /// `ccSeOn(se)`: a sound with no place.
    Se2d(i32),
    /// `ccSeOn3DNote(se, pos, note)`: a sound at a place on its note (a
    /// Grunty food taken: 179, 70).
    SeNote { se: i32, pos: V4, note: i32 },
    /// `ccVoicePgFood(kind)` (main 0x001800e0): a Grunty food calls out as
    /// Kite comes near - out of battle, row `kind` of `voiceFoodTbl`
    /// (`voiceFoodTblE` with the English voices) from `FOOD.BIN`
    /// (`voiceFile` 18).
    FoodVoice { kind: i32 },
    /// A Grunty food drawn: `SetMatrix_PosRotZYXScale(pos, dirc, scale)`,
    /// `ccChar::Draw()`.
    FoodDraw { who: usize, dirc: V4, scale: V4 },
    /// `effOpenBox(pos)`: a box or an idol opens.
    OpenBox { pos: V4 },
    /// `effRemoveTrap(pos, -1, -1)`: a trap taken off with the Fortune
    /// Wire, or the Zeit statue opened.
    TrapRemoved { pos: V4 },
    /// `effOpenTrapBox(pos, kind, trap)`: a trap goes off (kind 0 a
    /// treasure box, 1 a wooden box or barrel; trap 0 the hit, 3 and 4 the
    /// skills).
    OpenTrapBox { pos: V4, kind: i32, trap: i32 },
    /// `effCrushBarrel`, `effCrushPot`, `effCrushCorpse`, `effCrushEgg`
    /// (`what` 0-3) at `pos`, second argument 0.
    Crush { what: i32, pos: V4 },
    /// `effVirusCrystal(pos)`.
    VirusCrystal { pos: V4 },
    /// `effStatueOfGod(pos, &effsw)`: an idol's glow, which runs while the
    /// idol's `effsw` ([`crate::gimmick::Idol::effsw`]) is not 0.
    StatueOfGod { who: usize, pos: V4 },
    /// `ccEnemyEffDustRing(idol, ofs, 4.0, 100.0, 8, 30, 132)`.
    DustRing { who: usize, ofs: V4 },
    /// `ccAISysMsgSend(name, -1, Kite's AI id, 0xffff, 0, 30)`: a message
    /// to the party's AI (`ccSpcMessageOpenTrapBox`).
    SpcMessage(i32),
    /// A box's or an idol's rays ([`crate::prim`]).
    Radiate { who: usize, out: crate::prim::RadOut },
    /// A box drawn: `drawEnv` alpha `(256 t + 1) / 2` (a byte) and height (three
    /// times the base's), `WORLD_MAN::SetActiveLayer(layer)`, its model at
    /// `SetMatrix_PosRotZYX(pos, dirc)` with transparency `t`, the layer
    /// back to 0.
    GimDraw { who: usize, layer: i32, transparency: F, alpha: i32, height: F, dirc: V4 },
    /// An idol or a symbol drawn: `SetMatrix_PosRotZYX(pos, dirc)`,
    /// `ccChar::Draw()`.
    IdolDraw { who: usize, dirc: V4 },
    /// A symbol's omni light (`ccGimSymbol` +0x704) in the draw
    /// environment's group this frame at `pos` with `intensity`
    /// (`ccLightGrp::AddGrp`); None: taken out (`DelGrp`).
    SymbolLight { who: usize, pos: Option<V4>, intensity: F },
    /// One of a symbol's fires drawn: `EFF_x008` of its file,
    /// `ccEff::Draw(pattern)` at `pos` with that scale and colour, on layer
    /// 3.
    SymbolFire { pos: V4, pattern: u16, scale: [F; 2], colour: u32 },
    /// `effUseSymbol(pos)` (main 0x001d0d70): the symbol cast.
    UseSymbol { pos: V4 },
    /// `ccParticleExplode(pos, v, 1.5, 1)`: a symbol's spark while it
    /// casts.
    SymbolSpark { pos: V4, v: V4 },
    /// `effSmoke(pos, v, 1.0, 8, 204, 512, 32)`: a lake symbol's puff,
    /// every other frame.
    SymbolSmoke { pos: V4, v: V4 },
    /// A `ccGimEtc`'s particles at `pos`, running while its `effsw`
    /// (+0x1e0) is not 0: row 19 `effBossRoomEntrance(pos, &effsw)`
    /// (main 0x001d0be0), 20 `effFountain` (0x001d0ae0), 21
    /// `effDungeonEntrance(pos, &effsw, 2)` (0x001d0c90).
    EtcEffect { who: usize, row: i32, pos: V4 },
    /// The spring drawn (`ccAnm::Draw` on layer 6 at
    /// `SetMatrix_PosRotZYXScale(pos, dirc, scale)`).
    EtcDraw { who: usize, pos: V4, dirc: V4, scale: V4 },
    /// The spring's two omni lights in the group as they now stand
    /// (`ccLightGrp::AddGrp`); None: taken out (`DelGrp`).
    EtcLights { who: usize, lights: Option<[crate::prim::OmniLight; 2]> },
    /// `ccScFade::EntryFlash2(scFadeDef, t0, t1, colour, 0, 0, 512, 384)`,
    /// or with no `t1` `EntryFlash(scFadeDef, t0, colour, ...)`: the
    /// screen flashed.
    Flash { t0: i16, t1: Option<i16>, colour: u32 },
    /// `ccEnemyEffDustRing(pos, s, r, n, life, tex)` (gcmn 0x0043a590): a
    /// ring of dust at a place.
    DustRingAt { pos: V4, s: F, r: F, n: i32, life: i32, tex: i32 },
}

/// The objects' own code the entry control calls and does not hold: each
/// object's `main` (vtable +8) and the constructors of objects that are
/// not enemies of a ported race or magic circles. Each is called exactly
/// where the game calls it.
pub trait Seam {
    /// `ccEnemy::main` (0x00432cd0) of enemy `who`, after its `routine`:
    /// true to delete it (`deleteEnemy`). It may call back into `ctrl`
    /// (`entryEnemyObject` when a corpse is taken away, `entryObject` for
    /// a drained form).
    fn enemy_main(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx, who: usize) -> bool;
    /// A gimmick's `main` (its class's; `ccEntryObj::main`, 0x00431360,
    /// returns 0).
    fn gimmick_main(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx, who: usize) -> bool;
    /// An NPC's `main`.
    fn npc_main(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx, who: usize) -> bool;
    /// `ccEntryRaceTbl[race].func(entry)` for a race [`crate::races`] does
    /// not construct: the enemy of `ent` as its constructor leaves it.
    fn make_enemy(&mut self, cx: &mut Cx, race: i32, ent: &EntryParam, who: usize) -> (Char, Enemy);
    /// `gimmickTbl[id].entry.func(entry)` for a gimmick other than the
    /// magic circle (`ccEntryGimBox` ...).
    fn make_gimmick(&mut self, cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, EntryObj);
    /// `npcTbl[id].entry.func(entry)`.
    fn make_npc(&mut self, cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, EntryObj);
}

/// What the entry control works on: the tables, the characters and the
/// enemies' state by scene index, the world, both generators the spawning
/// draws from, the save, where the game is, the registered rows, and the
/// outputs.
pub struct Cx<'a> {
    pub t: &'a Tables,
    pub st: &'a SpawnTables,
    pub scene: &'a mut Scene,
    pub foes: &'a mut Vec<Option<Enemy>>,
    /// The world: the queries of the entry control ([`World`]) and, for
    /// the enemies' own frame ([`EnemySeam`]), the motion's calls.
    pub world: &'a mut dyn MotionWorld,
    /// `ccRand()` (main 0x001d9a10).
    pub cc: &'a mut dyn Rng,
    /// `lastRnd` (main 0x00378aec), `ccRandS`'s state.
    pub rnds: &'a mut u16,
    pub save: &'a mut SaveData,
    pub game: Game,
    pub reg: &'a Register,
    /// `patNum` of the magic circle's particle effect (`EFF_xmagpat1` of
    /// its scene file), which `ccEff::Init` reads from the chunk.
    pub part_pats: u16,
    pub out: &'a mut Vec<Out>,
}

/// `ccRandS()` (main 0x001d9c10): `lastRnd = rotl2(((lastRnd ^ 0x1100) -
/// 25939) & 0xffff)`, returned as a `short`.
pub fn rand_s(s: &mut u16) -> i16 {
    let v = u32::from((*s ^ 0x1100).wrapping_sub(25939));
    let r = ((v << 2) | (v >> 14)) as u16;
    *s = r;
    r as i16
}

/// `ccRegisterEnemyTbl[abs(ccRand() % ccRegisterEnemyNum)]`: a registered
/// row at random. With no row registered the game divides by zero (its
/// result is unpredictable); the port takes the first cell then.
fn random_row(cx: &mut Cx) -> i32 {
    let r = cx.cc.rand();
    let n = cx.reg.enemy_num;
    let k = if n == 0 { 0 } else { r.wrapping_rem(n).wrapping_abs() };
    cx.reg.enemy(k)
}

/// The members `ccEntryObj::routine` reads and writes, wherever an object
/// keeps them.
pub struct RoutineRef<'r> {
    pub dirc: &'r mut V4,
    pub grot_deg: i16,
    pub grot_spd: &'r mut i16,
    pub pl_dist: &'r mut F,
    pub pl_dirc: &'r mut F,
    pub disp_sw: &'r mut bool,
    pub freeze_flag: &'r mut bool,
    pub cmnd_flag: &'r mut bool,
    pub ent_root: i32,
    pub fade_flag: &'r mut i16,
    pub fade_cnt: i16,
    pub transparency: &'r mut F,
    pub set_transparency: &'r mut F,
}

impl EntryObj {
    pub fn routine_ref(&mut self) -> RoutineRef<'_> {
        RoutineRef {
            dirc: &mut self.dirc,
            grot_deg: self.grot_deg,
            grot_spd: &mut self.grot_spd,
            pl_dist: &mut self.pl_dist,
            pl_dirc: &mut self.pl_dirc,
            disp_sw: &mut self.disp_sw,
            freeze_flag: &mut self.freeze_flag,
            cmnd_flag: &mut self.cmnd_flag,
            ent_root: self.ent.ent_root,
            fade_flag: &mut self.fade_flag,
            fade_cnt: self.fade_cnt,
            transparency: &mut self.transparency,
            set_transparency: &mut self.set_transparency,
        }
    }
}

/// An enemy's [`RoutineRef`].
pub fn enemy_routine_ref(e: &mut Enemy) -> RoutineRef<'_> {
    RoutineRef {
        dirc: &mut e.dirc,
        grot_deg: e.grot_deg,
        grot_spd: &mut e.grot_spd,
        pl_dist: &mut e.pl_dist,
        pl_dirc: &mut e.pl_dirc,
        disp_sw: &mut e.disp_sw,
        freeze_flag: &mut e.freeze_flag,
        cmnd_flag: &mut e.cmnd_flag,
        ent_root: e.ent.ent_root,
        fade_flag: &mut e.fade_flag,
        fade_cnt: e.fade_cnt,
        transparency: &mut e.transparency,
        set_transparency: &mut e.set_transparency,
    }
}

/// `ccEntryCmnd(ch)` (gcmn 0x00519630): onto the end of the command list
/// its type names (party 0x7, foes 0xe0, objects), unless already listed.
pub fn entry_cmnd(scene: &mut Scene, who: usize) {
    if scene.listed(who) {
        return;
    }
    let ty = scene.chars[who].ty();
    if ty & 7 != 0 {
        scene.pc_list.push(who);
    } else if ty & 0xe0 != 0 {
        scene.ene_list.push(who);
    } else {
        scene.obj_list.push(who);
    }
}

/// `ccDeleteCmnd(ch)` (gcmn 0x00519700): off its command list.
pub fn delete_cmnd_char(scene: &mut Scene, out: &mut Vec<Out>, who: usize) {
    if scene.listed(who) {
        crate::enemy_ai::delete_from_lists(scene, who);
        out.push(Out::DeleteCmnd(who));
    }
}

/// `ccEntryObj::deleteCmnd(flg)` (gcmn 0x0042fe40): off the command list
/// unless `cmndFlag` already keeps it off, which `flg` then sets.
pub(crate) fn delete_cmnd(cmnd_flag: &mut bool, scene: &mut Scene, out: &mut Vec<Out>, who: usize, flg: bool) {
    if *cmnd_flag {
        return;
    }
    delete_cmnd_char(scene, out, who);
    *cmnd_flag = flg;
}

/// `ccEntryObj::routine` (gcmn 0x0042fa60), every frame before the
/// object's `main`: an event's turn (`grotSpd`, toward `grotDeg`, through
/// `ccSetDirc`, ending within 0.003 rad), `plDist` and `plDirc` from Kite
/// (the position through the player's frame, negated, on the ground),
/// `dispSW` within 7000, beyond 10000 frozen and off the command list
/// (`cmndFlag` kept set for an entry with an `entRoot`), else on it
/// (`ccEntryCmnd`) unless `cmndFlag`; then a fade in (`fadeFlag` 1) or out
/// (2) of `1 / fadeCnt` a frame.
pub fn routine(
    volume: Volume,
    o: RoutineRef,
    pos: V4,
    who: usize,
    scene: &mut Scene,
    world: &mut dyn World,
    out: &mut Vec<Out>,
) {
    if *o.grot_spd != 0 {
        let mut tmp = *o.dirc;
        let r = deg2rad(o.grot_deg);
        tmp[2] = set_dirc(tmp[2], r, i32::from(*o.grot_spd));
        *o.dirc = tmp;
        // fabs((double) (dirc.z - r)) < (double) 0.003f
        if f64::from(f32::from_bits(sub(tmp[2], r))).abs() < f64::from_bits(0x3f68_9374_c000_0000) {
            *o.grot_spd = 0;
        }
    }
    let q = world.w2p(pos);
    let p = [mul(q[0], geom::MINUS_ONE), mul(q[1], geom::MINUS_ONE), 0, ONE];
    *o.pl_dist = geom::length_on(volume, p);
    let mut d = add(HALF_PI, atan2f(p[1], p[0]));
    if !le(d, PI) {
        d = sub(d, TWO_PI);
    }
    if lt(d, NEG_PI) {
        d = add(d, TWO_PI);
    }
    *o.pl_dirc = d;
    *o.disp_sw = le(*o.pl_dist, geom::k(7000.0));
    if !le(*o.pl_dist, geom::k(10000.0)) {
        *o.freeze_flag = true;
        let keep = o.ent_root != -1 && o.ent_root != 0;
        delete_cmnd(o.cmnd_flag, scene, out, who, keep);
    } else {
        *o.freeze_flag = false;
        if !*o.cmnd_flag {
            entry_cmnd(scene, who);
        }
    }
    match *o.fade_flag {
        1 => {
            let mut t = add(*o.set_transparency, div(ONE, from_int(i32::from(o.fade_cnt))));
            if !le(t, ONE) {
                t = ONE;
                *o.fade_flag = 0;
            }
            *o.transparency = t;
            *o.set_transparency = t;
        }
        2 => {
            let mut t = sub(*o.set_transparency, div(ONE, from_int(i32::from(o.fade_cnt))));
            if lt(t, 0) {
                t = 0;
                *o.fade_flag = 0;
            }
            *o.transparency = t;
            *o.set_transparency = t;
        }
        _ => {}
    }
}

// VU0 macro routines (sceVu0*, main), in the FPU's rules ------------------------------

type Mat = [V4; 4];

const UNIT: Mat = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];

/// `_sceVu0ecossin`'s polynomial (S5432, main 0x002f7520).
const SIN: [F; 4] = [0x362e_9c14, 0xb94f_b21f, 0x3c08_873e, 0xbe2a_aaa4];

/// `_sceVu0ecossin` (main 0x001109b8) as `sceVu0RotMatrixX/Y/Z` use it:
/// (sin, cos), cos an odd polynomial in pi/2 - |angle|, sin +-sqrt(1 -
/// cos^2).
fn vu_cossin(angle: F) -> (F, F) {
    let neg = lt(angle, 0);
    let x = if neg { add(HALF_PI, angle) } else { sub(HALF_PI, angle) };
    let x2 = mul(x, x);
    let mut p = SIN.map(|k| mul(mul(k, x), x2));
    for v in &mut p[..3] {
        *v = mul(*v, x2);
    }
    let mut r = add(x, p[3]);
    for v in &mut p[..2] {
        *v = mul(*v, x2);
    }
    r = add(r, p[2]);
    p[0] = mul(p[0], x2);
    r = add(add(r, p[1]), p[0]);
    let q = geom::sqrt(sub(ONE, mul(r, r)));
    (if neg { sub(0, q) } else { add(0, q) }, r)
}

/// The rotation's columns times each column of `m`.
fn vu_mul(cols: &Mat, m: &Mat) -> Mat {
    m.map(|col| {
        std::array::from_fn(|k| {
            let acc = mul(cols[0][k], col[0]);
            let acc = add(acc, mul(cols[1][k], col[1]));
            let acc = add(acc, mul(cols[2][k], col[2]));
            add(acc, mul(cols[3][k], col[3]))
        })
    })
}

/// `sceVu0RotMatrixX/Y/Z(out, m, angle)` for axis 0, 1, 2: the axis
/// rotation times `m`.
fn vu_rot(m: &Mat, axis: usize, angle: F) -> Mat {
    let (s, c) = vu_cossin(angle);
    let (cs, sn, ns) = (add(0, c), add(0, s), sub(0, s));
    let rot = match axis {
        0 => [[ONE, 0, 0, 0], [0, cs, sn, 0], [0, ns, cs, 0], [0, 0, 0, ONE]],
        1 => [[cs, 0, ns, 0], [0, ONE, 0, 0], [sn, 0, cs, 0], [0, 0, 0, ONE]],
        _ => [[cs, sn, 0, 0], [ns, cs, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]],
    };
    vu_mul(&rot, m)
}

/// `sceVu0ApplyMatrix(out, m, v)`: ((c0 x + c1 y) + c2 z) + c3 w.
fn vu_apply(m: &Mat, v: V4) -> V4 {
    std::array::from_fn(|k| {
        let acc = mul(m[0][k], v[0]);
        let acc = add(acc, mul(m[1][k], v[1]));
        let acc = add(acc, mul(m[2][k], v[2]));
        add(acc, mul(m[3][k], v[3]))
    })
}

/// `DEG2RAD((short) ccRand())`: a random angle.
fn rand_angle(cc: &mut dyn Rng) -> F {
    deg2rad(cc.rand() as i16)
}

/// Unit, then X, Y and Z by random angles (the particles' way of turning
/// at random).
fn random_turn(cc: &mut dyn Rng) -> Mat {
    let m = vu_rot(&UNIT, 0, rand_angle(cc));
    let m = vu_rot(&m, 1, rand_angle(cc));
    vu_rot(&m, 2, rand_angle(cc))
}

/// `DEG2RAD((short)(RAD2DEG(r) + step))`.
fn turn(r: F, step: i32) -> F {
    deg2rad((i32::from(rad2deg(r)) + step) as i16)
}

/// `x -= step` while `x > floor` (the particles' shrinking).
fn shrink(x: &mut F, floor: F, step: F) {
    if !le(*x, floor) {
        *x = sub(*x, step);
    }
}

impl McPart {
    /// `ccMcPart::fade` (gcmn 0x00454840): every fourth frame the colour's
    /// red, green and blue fall by one (not below 0); its alpha rises by
    /// one every frame (from 255 it wraps to 0 in the stored word).
    pub fn fade(&mut self) {
        let c = self.eff.color;
        let (mut r, mut g, mut b, a) = (c & 0xff, (c >> 8) & 0xff, (c >> 16) & 0xff, c >> 24);
        if self.cnt & 3 == 0 {
            r = r.saturating_sub(1);
            g = g.saturating_sub(1);
            b = b.saturating_sub(1);
        }
        let a = a + 1;
        self.eff.color = r | (g << 8) | (b << 16) | a.wrapping_shl(24);
    }

    /// `ccMcPart::main` (gcmn 0x004548e0), a live particle's frame: its
    /// effect's position through the player's frame (`ccTransPosFW2LW`); on
    /// its first frame its life, a scale of 5 and a random turn (three
    /// `ccRand`s, then a `ccRandF` for kinds 1, 2 and 4), later its motion by
    /// kind (docs/engine/effects.md, "The magic portal"); then `fade`, `cnt` up
    /// and the pattern round `patNum`. From Mutation on `ccTransPosFW2LW`
    /// leaves the position as it is without a player (`frame` false).
    pub fn main(&mut self, world: &mut dyn World, cc: &mut dyn Rng, frame: bool) {
        if frame {
            let p = world.w2p(self.eff.pos);
            self.eff.pos = world.p2w(p);
        }
        let first = self.cnt == 0;
        match self.status {
            1 | 3 | 4 if first => {
                self.life = match self.status {
                    1 | 4 => 120,
                    _ => 30,
                };
                if self.status == 4 {
                    self.eff.scale_x = geom::k(5.0);
                    self.eff.scale_y = geom::k(5.0);
                } else {
                    self.eff.scale_y = geom::k(5.0);
                    self.eff.scale_x = geom::k(5.0);
                }
                if self.status == 3 {
                    self.speed = geom::k(30.0);
                }
                let t = self.mat[3];
                self.eff.pos = [t[0], t[1], t[2], ONE];
                self.mat = random_turn(cc);
                if self.status == 1 {
                    let mut r = crate::enemy_ai::rand_f(cc, 0x3e4c_cccd);
                    self.rnd = r;
                    if lt(r, 0) {
                        r = mul(r, geom::MINUS_ONE);
                        self.rnd = r;
                    }
                }
            }
            2 if first => {
                self.life = 10;
                self.eff.scale_y = geom::k(5.0);
                self.eff.scale_x = geom::k(5.0);
                let t = self.mat[3];
                let mut at = [t[0], t[1], t[2], ONE];
                let m = random_turn(cc);
                let r = crate::enemy_ai::rand_f(cc, geom::k(20.0));
                let base = sub(geom::k(400.0), mul(geom::k(6.0), from_int(i32::from(self.mc_act_cnt))));
                let v = vu_apply(&m, [add(base, r), 0, 0, ONE]);
                at = geom::vadd(at, v);
                self.eff.pos = at;
                self.mat = m;
            }
            1 => {
                self.rad_cnt = turn(self.rad_cnt, 838);
                self.speed = mul(geom::k(6.0), sinf(self.rad_cnt));
                let v = vu_apply(&self.mat, [self.speed, 0, 0, ONE]);
                self.eff.pos = geom::vadd(self.eff.pos, v);
                let z = mul(geom::k(6.0), sinf(deg2rad(i32::from(self.cnt).wrapping_mul(1383) as i16)));
                self.eff.pos = geom::vadd(self.eff.pos, [0, 0, z, 0]);
                shrink(&mut self.eff.scale_x, ONE, 0x3d23_d70a);
                shrink(&mut self.eff.scale_y, ONE, 0x3d23_d70a);
            }
            2 => {
                self.rad_cnt = turn(self.rad_cnt, 1638);
                self.speed = mul(geom::k(20.0), mul(0x3f66_6666, sinf(self.rad_cnt)));
                let v = [geom::neg(self.speed), 0, 0, ONE];
                let mut s = mul(0x3d93_74bc, geom::cosf(self.rad_cnt));
                self.speed = s;
                if !le(s, PI) {
                    s = sub(s, TWO_PI);
                    self.speed = s;
                }
                if lt(self.speed, NEG_PI) {
                    self.speed = add(self.speed, TWO_PI);
                }
                let m = vu_rot(&self.mat, 0, self.speed);
                let m = vu_rot(&m, 1, self.speed);
                let m = vu_rot(&m, 2, self.speed);
                self.mat = m;
                let v = vu_apply(&m, v);
                self.eff.pos = geom::vadd(self.eff.pos, v);
                shrink(&mut self.eff.scale_x, ONE, 0x3e8f_5c29);
                shrink(&mut self.eff.scale_y, ONE, 0x3e8f_5c29);
            }
            3 => {
                self.rad_cnt = turn(self.rad_cnt, 582);
                self.speed = sub(self.speed, sinf(self.rad_cnt));
                let v = vu_apply(&self.mat, [self.speed, 0, 0, 0]);
                self.eff.pos = geom::vadd(self.eff.pos, v);
                self.eff.pos = geom::vadd(self.eff.pos, [0, 0, 0xc080_0000, 0]);
                shrink(&mut self.eff.scale_x, geom::k(2.0), 0x3d75_c28f);
                shrink(&mut self.eff.scale_y, geom::k(2.0), 0x3d75_c28f);
            }
            4 => {
                self.rad_cnt = turn(self.rad_cnt, 758);
                self.speed = mul(geom::k(8.0), sinf(self.rad_cnt));
                let z = crate::enemy_ai::rand_f(cc, 0x3f99_999a);
                let v = vu_apply(&self.mat, [self.speed, 0, z, ONE]);
                self.eff.pos = geom::vadd(self.eff.pos, v);
                shrink(&mut self.eff.scale_x, ONE, 0x3cf5_c28f);
                shrink(&mut self.eff.scale_y, ONE, 0x3cf5_c28f);
            }
            _ => {}
        }
        self.fade();
        let old = self.cnt;
        self.cnt = self.cnt.wrapping_add(1);
        if self.life < old {
            self.anm_flag = true;
        }
        self.eff_anm_pat = self.eff_anm_pat.wrapping_add(1);
        if i32::from(self.eff_anm_pat) >= i32::from(self.eff.pat_num) {
            self.eff_anm_pat = 0;
        }
        if self.anm_flag {
            self.status = 0;
        }
    }
}

impl MagicCircle {
    /// `ccMagicCircle::setPart(kind, mat, num)` (gcmn 0x00455510): `num`
    /// particles of `kind` at `mat`, each in the first free one from
    /// `partTop`, else over `partTop` (which then moves on).
    pub fn set_part(&mut self, kind: i32, mat: &Mat, num: i32) {
        let act = self.obj.act_cnt as i16;
        let init = |p: &mut McPart| {
            p.status = kind;
            p.anm_flag = false;
            p.rad_cnt = 0;
            p.cnt = 0;
            p.eff_anm_pat = 0;
            p.transparency = ONE;
            p.mat = *mat;
            p.mc_act_cnt = act;
            p.eff.pos = ZERO_POS;
            p.eff.rotate = 0;
            p.eff.color = 0x0080_8080;
        };
        for _ in 0..num {
            let free = (0..PARTS as i32).find_map(|j| {
                let mut k = self.part_top + j;
                if k >= 128 {
                    k -= 128;
                }
                (self.parts[k as usize].status == 0).then_some(k as usize)
            });
            match free {
                Some(k) => init(&mut self.parts[k]),
                None => {
                    init(&mut self.parts[self.part_top as usize]);
                    self.part_top += 1;
                    if self.part_top >= 128 {
                        self.part_top -= 128;
                    }
                }
            }
        }
    }

    /// `ccMagicCircle::createPart` (gcmn 0x00455710): this frame's new
    /// particles at the circle (its position in the matrix's translation):
    /// waiting, 3 of kind 1; opening, `actCnt / 4 + 1` of kind 2; giving,
    /// 8 of kind 4; the first 16 frames after, 2 of kind 3.
    pub fn create_part(&mut self, pos: V4) {
        let mut m = UNIT;
        m[3] = pos;
        match self.obj.act_num {
            0 => self.set_part(1, &m, 3),
            1 => self.set_part(2, &m, (self.obj.act_cnt >> 2) + 1),
            2 => self.set_part(4, &m, 8),
            3 if self.obj.act_cnt < 16 => self.set_part(3, &m, 2),
            _ => {}
        }
    }
}

/// `ccMagicCircle::ccMagicCircle(entry)` (gcmn 0x00455900): a
/// `ccGimmick` with every particle free, the entry copied, placed at its
/// position and heading, `gimId` the entry's id and the gimmick table's
/// base, the model `CMP_xmagcir0` playing `ANM_xmagcir1`
/// ([`World::anim_set`]), `ccDestMagicCircle` for clean-up, and off the
/// command lists for good (`deleteCmnd(1)`).
pub fn magic_circle_new(cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, MagicCircle) {
    let mut o = EntryObj { ent: *ent, dirc: ent.dirc, ..EntryObj::default() };
    let mut ch = gimmick_char(cx.st, ent.id);
    ch.ent_root = ent.ent_root as u32;
    ch.pos = ent.pos;
    ch.pos_p = cx.world.w2p(ent.pos);
    o.act_num = 0;
    o.act_cnt = 0;
    o.gim_id = ent.id;
    cx.world.anim_set(who, AnmSlot::Main, "ANM_xmagcir1");
    o.dest_func = 0x0045_5870;
    // deleteCmnd(1): not listed yet, so only cmndFlag.
    o.cmnd_flag = true;
    let mc = MagicCircle { obj: o, part_flag: false, part_top: 0, parts: vec![McPart::default(); PARTS] };
    (ch, mc)
}

/// A gimmick's `ccChar` as `ccChar::ccChar` and `SetBaseParam(ccGimmickParam
/// *)` leave it: the table row's base, no conditions, `skillStatus` 0 and
/// the `affectFunc` `ccGimmickAffect` (the `ccGimmick` constructor's).
pub fn gimmick_char(st: &SpawnTables, id: i32) -> Char {
    let base = usize::try_from(id).ok().and_then(|i| st.gimmicks.get(i)).map(|g| g.base).unwrap_or_default();
    let mut ch = Char::other(base, Elm::default());
    ch.condition_num = -1;
    ch.skill_status = 0;
    ch.affect.func = AffectFunc::Gimmick;
    ch
}

/// An NPC's character as `ccMerchan`'s and `ccRtownPC`'s constructors
/// start it over `ccGimmick`: `SetBaseParam(ccGetNpcParam(id))` (the
/// `npcTbl` row's base), no condition, no affect.
pub fn npc_char(st: &SpawnTables, id: i32) -> Char {
    let base = usize::try_from(id).ok().and_then(|i| st.npcs.get(i)).map(|n| n.base).unwrap_or_default();
    let mut ch = Char::other(base, Elm::default());
    ch.condition_num = -1;
    ch.skill_status = 0;
    ch.affect.func = AffectFunc::None;
    ch
}

impl EntryCtrl {
    /// The control as `ccThEntryCtrl` makes it: where the game is, empty
    /// lists.
    pub fn new(game: &Game) -> EntryCtrl {
        let area_num = match game.area {
            0 => game.town,
            1 => game.field,
            2 => game.dungeon,
            _ => 0,
        };
        EntryCtrl { area: game.area, area_num, floor: game.floor, block: game.block, ..EntryCtrl::default() }
    }

    fn grow(&mut self, n: usize) {
        if self.links.len() < n {
            self.links.resize(n, Link::default());
        }
        if self.objs.len() < n {
            self.objs.resize(n, Obj::None);
        }
    }

    /// The objects of a list, head to foot.
    pub fn list(&self, k: Kind) -> Vec<usize> {
        let mut v = Vec::new();
        let mut cur = self.lists[k as usize].head;
        while let Some(c) = cur {
            if v.len() > self.links.len() {
                break;
            }
            v.push(c);
            cur = self.links.get(c).and_then(|l| l.next);
        }
        v
    }

    /// The object's `ccEntryObj` members, when it is not an enemy.
    pub fn entry_obj(&self, who: usize) -> Option<&EntryObj> {
        match self.objs.get(who)? {
            Obj::Circle(c) => Some(&c.obj),
            Obj::Gimmick(o) | Obj::Npc(o) => Some(o),
            _ => None,
        }
    }

    pub fn entry_obj_mut(&mut self, who: usize) -> Option<&mut EntryObj> {
        match self.objs.get_mut(who)? {
            Obj::Circle(c) => Some(&mut c.obj),
            Obj::Gimmick(o) | Obj::Npc(o) => Some(o),
            _ => None,
        }
    }

    fn obj_flag(&self, cx: &Cx, who: usize) -> bool {
        match self.objs.get(who) {
            Some(Obj::Enemy) => cx.foes.get(who).and_then(|f| f.as_ref()).is_some_and(|e| e.obj_flag),
            _ => self.entry_obj(who).is_some_and(|o| o.obj_flag),
        }
    }

    fn append(&mut self, k: Kind, who: usize) {
        self.grow(who + 1);
        let l = &mut self.lists[k as usize];
        match l.foot {
            None => l.head = Some(who),
            Some(f) => self.links[f].next = Some(who),
        }
        let foot = l.foot;
        l.foot = Some(who);
        l.num = l.num.wrapping_add(1);
        self.links[who] = Link { pre: foot, next: None };
    }

    fn unlink(&mut self, k: Kind, who: usize) {
        let Link { pre, next } = self.links[who];
        let l = &mut self.lists[k as usize];
        match pre {
            None => {
                if let Some(n) = next {
                    self.links[n].pre = None;
                }
                l.head = next;
            }
            Some(p) => self.links[p].next = next,
        }
        match next {
            None => {
                if let Some(p) = pre {
                    self.links[p].next = None;
                }
                l.foot = pre;
            }
            Some(n) => self.links[n].pre = pre,
        }
        l.num = l.num.wrapping_sub(1);
    }

    /// `ccEntryObj::~ccEntryObj` with `delete`: `destFunc`, the models
    /// freed (a circle's `ccDestMagicCircle` frees its particles), the body
    /// out of the collision list (`~ccCharHit`: `HitDisable` while in it),
    /// off the command lists (`~ccChar`: `ccDeleteCmnd`).
    fn destroy(&mut self, cx: &mut Cx, who: usize) {
        let obj = std::mem::take(&mut self.objs[who]);
        let (dest_func, mut hit) = match &obj {
            Obj::Enemy => {
                let e = cx.foes[who].take().expect("an enemy's state");
                (e.dest_func, e.hit)
            }
            // ccDestMagicCircle frees the particles' effects.
            Obj::Circle(c) => (c.obj.dest_func, c.obj.hit),
            Obj::Gimmick(o) | Obj::Npc(o) => (o.dest_func, o.hit),
            Obj::None => return,
        };
        cx.out.push(Out::Destroyed { who, dest_func });
        if hit.sw {
            cx.world.hit_switch(who, &mut hit, false);
        }
        delete_cmnd_char(cx.scene, cx.out, who);
    }

    fn delete(&mut self, k: Kind, cx: &mut Cx, who: usize) {
        self.unlink(k, who);
        self.destroy(cx, who);
    }

    /// `ccEntryCtrl::deleteEnemy(obj)` (gcmn 0x004312d0): unlinked and
    /// destroyed.
    pub fn delete_enemy(&mut self, cx: &mut Cx, who: usize) {
        self.delete(Kind::Enemy, cx, who);
    }

    /// `ccEntryCtrl::deleteMagicCircle(obj)` (gcmn 0x004314e0).
    pub fn delete_magic_circle(&mut self, cx: &mut Cx, who: usize) {
        self.delete(Kind::Circle, cx, who);
    }

    /// `ccEntryCtrl::deleteGimmick(obj)` (gcmn 0x004316a0).
    pub fn delete_gimmick(&mut self, cx: &mut Cx, who: usize) {
        self.delete(Kind::Gimmick, cx, who);
    }

    /// `ccEntryCtrl::deleteNpc(obj)` (gcmn 0x00431860).
    pub fn delete_npc(&mut self, cx: &mut Cx, who: usize) {
        self.delete(Kind::Npc, cx, who);
    }

    /// `ccCheckActiveEnemy()` (gcmn 0x0042df10): the enemies on the list
    /// with `objFlag` set and `freezeFlag` clear.
    pub fn check_active_enemy(&self, cx: &Cx) -> i32 {
        let mut n = 0;
        let mut cur = self.lists[Kind::Enemy as usize].head;
        for _ in 0..self.lists[Kind::Enemy as usize].num {
            let Some(c) = cur else { break };
            if let Some(e) = cx.foes.get(c).and_then(|f| f.as_ref())
                && e.obj_flag
                && !e.freeze_flag
            {
                n += 1;
            }
            cur = self.links[c].next;
        }
        n
    }

    /// `ccCheckActiveObject()` (gcmn 0x0042df70): true when no enemy and no
    /// magic circle has `objFlag` set (a room is cleared).
    pub fn check_active_object(&self, cx: &Cx) -> bool {
        for k in [Kind::Enemy, Kind::Circle] {
            let mut cur = self.lists[k as usize].head;
            for _ in 0..self.lists[k as usize].num {
                let Some(c) = cur else { break };
                if self.obj_flag(cx, c) {
                    return false;
                }
                cur = self.links[c].next;
            }
        }
        true
    }

    /// `ccCheckActiveObject(floor, block)` (gcmn 0x0042e010): true when no
    /// enemy and no magic circle belongs to that floor and block
    /// (`DUNGEON::SetRoom` opens a room's doors then).
    pub fn check_active_object_at(&self, cx: &Cx, floor: i32, block: i32) -> bool {
        for k in [Kind::Enemy, Kind::Circle] {
            let mut cur = self.lists[k as usize].head;
            for _ in 0..self.lists[k as usize].num {
                let Some(c) = cur else { break };
                let ent = match self.objs.get(c) {
                    Some(Obj::Enemy) => cx.foes[c].as_ref().map(|e| e.ent),
                    _ => self.entry_obj(c).map(|o| o.ent),
                };
                if ent.is_some_and(|e| e.floor == floor && e.block == block) {
                    return false;
                }
                cur = self.links[c].next;
            }
        }
        true
    }

    /// `ccEntryCtrl::initObject(obj)` (gcmn 0x00430ec0): an object of the
    /// control's area, area number, floor and block is switched on
    /// (`objFlag`, `initFlag`, its body into the collision list,
    /// `ccCharHit::SetHitSW(1)`) and, with `land` -1, set on the ground
    /// (`ccLandHitCheck(pos, 0x20000002)`, a magic circle 250 above it),
    /// its entry's and its own position and `posP` following; any other is
    /// switched off.
    pub fn init_object(&mut self, cx: &mut Cx, who: usize) {
        let enemy = matches!(self.objs.get(who), Some(Obj::Enemy));
        let ent = if enemy {
            cx.foes[who].as_ref().expect("an enemy's state").ent
        } else {
            self.entry_obj(who).expect("an entry object").ent
        };
        let here = ent.area == self.area
            && ent.area_num == self.area_num
            && ent.floor == self.floor
            && ent.block == self.block;
        let (flags, hit): ((&mut bool, &mut bool), &mut CharHit) = if enemy {
            let e = cx.foes[who].as_mut().expect("an enemy's state");
            ((&mut e.obj_flag, &mut e.init_flag), &mut e.hit)
        } else {
            let o = match &mut self.objs[who] {
                Obj::Circle(c) => &mut c.obj,
                Obj::Gimmick(o) | Obj::Npc(o) => o,
                _ => unreachable!(),
            };
            ((&mut o.obj_flag, &mut o.init_flag), &mut o.hit)
        };
        *flags.0 = here;
        *flags.1 = here;
        set_hit_sw(cx.world, who, hit, here);
        if !here || ent.land != -1 {
            return;
        }
        let mut t = cx.scene.chars[who].pos;
        t[2] = cx.world.land(t, 0x2000_0002);
        if ent.ty == 1 && ent.id == MAGIC_CIRCLE {
            t[2] = add(t[2], geom::k(250.0));
        }
        match &mut self.objs[who] {
            Obj::Enemy => cx.foes[who].as_mut().expect("an enemy's state").ent.pos = t,
            Obj::Circle(c) => c.obj.ent.pos = t,
            Obj::Gimmick(o) | Obj::Npc(o) => o.ent.pos = t,
            Obj::None => {}
        }
        let ch = &mut cx.scene.chars[who];
        ch.pos = t;
        ch.pos_p = cx.world.w2p(t);
    }

    /// A new scene index for an object the control makes.
    fn new_index(cx: &mut Cx) -> usize {
        cx.scene.chars.len()
    }

    fn put_char(cx: &mut Cx, who: usize, ch: Char) {
        if who == cx.scene.chars.len() {
            cx.scene.chars.push(ch);
        } else {
            cx.scene.chars[who] = ch;
        }
        if cx.foes.len() < cx.scene.chars.len() {
            cx.foes.resize(cx.scene.chars.len(), None);
        }
    }

    /// `ccEntryCtrl::entryEnemy(ep)` (gcmn 0x00431220): the row's race
    /// constructor (`enemyTbl[id].entry.func`, from `ccEntryRaceTbl`;
    /// [`crate::races::construct`], or [`Seam::make_enemy`] for a race it
    /// does not make), `initObject`, and onto the enemy list's end.
    pub fn entry_enemy(&mut self, cx: &mut Cx, seam: &mut dyn Seam, ep: &EntryParam) -> usize {
        let who = Self::new_index(cx);
        self.grow(who + 1);
        let race = crate::enemy_ai::enemy_race(cx.t, ep.id).map_or(-1, |r| r.0);
        let made = crate::races::construct(cx, race, ep, who);
        let (ch, e) = match made {
            Some(x) => x,
            None => seam.make_enemy(cx, race, ep, who),
        };
        Self::put_char(cx, who, ch);
        cx.foes[who] = Some(e);
        self.objs[who] = Obj::Enemy;
        self.init_object(cx, who);
        self.append(Kind::Enemy, who);
        who
    }

    /// `ccEntryCtrl::entryMagicCircle(ep)` (gcmn 0x004313f0): the entry set
    /// 250 above the ground (`ccLandHitCheck(pos, 0x20000002)`), then
    /// `gimmickTbl[15]`'s constructor ([`magic_circle_new`]), `initObject`,
    /// and onto the circle list's end.
    pub fn entry_magic_circle(&mut self, cx: &mut Cx, ep: &mut EntryParam) -> usize {
        let mut t = ep.pos;
        t[2] = cx.world.land(t, 0x2000_0002);
        t[2] = add(t[2], geom::k(250.0));
        ep.pos = t;
        let who = Self::new_index(cx);
        self.grow(who + 1);
        let (ch, mc) = magic_circle_new(cx, ep, who);
        Self::put_char(cx, who, ch);
        self.objs[who] = Obj::Circle(Box::new(mc));
        self.init_object(cx, who);
        self.append(Kind::Circle, who);
        who
    }

    /// `ccEntryCtrl::entryGimmick(ep)` (gcmn 0x004315f0): the gimmick's
    /// constructor ([`Seam::make_gimmick`]), `initObject`, onto the
    /// gimmick list's end.
    pub fn entry_gimmick(&mut self, cx: &mut Cx, seam: &mut dyn Seam, ep: &EntryParam) -> usize {
        let who = Self::new_index(cx);
        self.grow(who + 1);
        let (ch, o) = match crate::gimmick::construct(cx, ep, who) {
            Some(made) => made,
            None => seam.make_gimmick(cx, ep, who),
        };
        Self::put_char(cx, who, ch);
        self.objs[who] = Obj::Gimmick(Box::new(o));
        self.init_object(cx, who);
        self.append(Kind::Gimmick, who);
        who
    }

    /// `ccEntryCtrl::entryNpc(ep)` (gcmn 0x004317b0).
    pub fn entry_npc(&mut self, cx: &mut Cx, seam: &mut dyn Seam, ep: &EntryParam) -> usize {
        let who = Self::new_index(cx);
        self.grow(who + 1);
        let (ch, o) = seam.make_npc(cx, ep, who);
        Self::put_char(cx, who, ch);
        self.objs[who] = Obj::Npc(Box::new(o));
        self.init_object(cx, who);
        self.append(Kind::Npc, who);
        who
    }

    /// `ccEntryCtrl::entryObjectCheck(ep)` (gcmn 0x00430530): true when the
    /// entry is handled here and nothing more is made. An enemy of row -1
    /// takes a registered row at random. A gimmick an event placed
    /// (`entRoot` 0, `param[0]` an event entry) whose entry the save no
    /// longer holds: ids 0-5 (boxes) lose `param[1]` (-1), 38-44 `param[2]`
    /// (0), id 6 is not made. Id 15 becomes a magic circle
    /// ([`EntryCtrl::entry_magic_circle`]). A fountain (20) already used
    /// in this area (`CheckFountain`) is not made.
    pub fn entry_object_check(&mut self, cx: &mut Cx, ep: &mut EntryParam) -> bool {
        let still = |cx: &Cx, ep: &EntryParam| {
            ep.ent_root != 0 || ep.param[0] == -1 || check_event_entry(cx.save, cx.game.field, ep.param[0])
        };
        match ep.ty {
            0 => {
                if ep.id == -1 {
                    ep.id = random_row(cx);
                }
                false
            }
            1 => match ep.id {
                6 => !still(cx, ep),
                0..=5 => {
                    if !still(cx, ep) {
                        ep.param[1] = -1;
                    }
                    false
                }
                38..=44 => {
                    if !still(cx, ep) {
                        ep.param[2] = 0;
                    }
                    false
                }
                MAGIC_CIRCLE => {
                    self.entry_magic_circle(cx, ep);
                    true
                }
                20 => check_fountain(cx.save, cx.game.server, cx.game.area_code),
                _ => false,
            },
            _ => false,
        }
    }

    /// `ccEntryCtrl::entryObject(ep, n)` (gcmn 0x004307f0): after
    /// `entryObjectCheck`, one object on the spot for `n` 1, else `n` objects
    /// 300 away round it, each facing 90 degrees on with `param[3]` its number;
    /// after each enemy of an `entRoot` other than 0 and 3 a registered row may
    /// take the entry's row (docs/engine/battle.md, "Spawning"). The entry is
    /// changed in place. Returns the last object made.
    pub fn entry_object_n(&mut self, cx: &mut Cx, seam: &mut dyn Seam, ep: &mut EntryParam, n: i32) -> Option<usize> {
        if self.entry_object_check(cx, ep) {
            return None;
        }
        let mut obj = None;
        if n == 1 {
            if ep.land == -1 {
                let mut t = ep.pos;
                t[2] = cx.world.land(t, 0x2000_0002);
                ep.pos = t;
            }
            obj = match ep.ty {
                0 => {
                    ep.param[3] = 0;
                    Some(self.entry_enemy(cx, seam, ep))
                }
                1 => Some(self.entry_gimmick(cx, seam, ep)),
                2 => Some(self.entry_npc(cx, seam, ep)),
                _ => None,
            };
            return obj;
        }
        let centre = ep.pos;
        let heading = ep.dirc[2];
        let step = deg2rad(rad2deg(div(TWO_PI, from_int(n))));
        let start = deg2rad((i32::from(rad2deg(heading)) - 16384) as i16);
        for i in 0..n {
            let mut a = add(div(step, geom::k(2.0)), add(start, mul(step, from_int(i))));
            if !le(a, PI) {
                a = sub(a, TWO_PI);
            }
            if lt(a, NEG_PI) {
                a = add(a, TWO_PI);
            }
            let m = vu_rot(&UNIT, 2, a);
            let mut v = vu_apply(&m, [geom::k(300.0), 0, 0, ONE]);
            v = geom::vadd(v, centre);
            if ep.land == -1 {
                v[2] = cx.world.land(v, 0x2000_0002);
            }
            let d = [0, 0, deg2rad((i32::from(rad2deg(a)) + 16384) as i16), ONE];
            ep.pos = v;
            ep.dirc = d;
            match ep.ty {
                0 => {
                    ep.param[3] = i;
                    obj = Some(self.entry_enemy(cx, seam, ep));
                    if ep.ent_root != 0 && ep.ent_root != 3 {
                        let r = random_row(cx);
                        let esize = cx.t.enemies.get(r as usize).map_or(0, |row| row.esize);
                        if esize == 3 || esize == 4 {
                            ep.id = r;
                        }
                    }
                }
                1 => obj = Some(self.entry_gimmick(cx, seam, ep)),
                2 => obj = Some(self.entry_npc(cx, seam, ep)),
                _ => {}
            }
        }
        obj
    }

    /// `ccEntryCtrl::entryObject(ep)` (gcmn 0x00430c90): how many objects
    /// the entry makes, then [`EntryCtrl::entry_object_n`]. An enemy (a
    /// registered row at random for row -1) makes one more than
    /// `abs(ccRand() % esize)` by its row's `esize`: 1 always one; 3 one to three, a one
    /// becoming two when `(ccRand() >> 2) & 1`; 4 one to four, a one
    /// becoming two. A gimmick or an NPC makes its table row's `esize`. (Of
    /// another type the game passes an unset register; the port makes
    /// none.)
    pub fn entry_object(&mut self, cx: &mut Cx, seam: &mut dyn Seam, ep: &mut EntryParam) -> Option<usize> {
        let n = match ep.ty {
            0 => {
                if ep.id == -1 {
                    ep.id = random_row(cx);
                }
                let esize = cx.t.enemies.get(ep.id as usize).map_or(0, |row| row.esize);
                let r = cx.cc.rand();
                let mut n = if esize == 0 { 1 } else { r.wrapping_rem(esize).wrapping_abs() + 1 };
                match esize {
                    3 if n == 1 && (cx.cc.rand() >> 2) & 1 != 0 => n += 1,
                    4 if n == 1 => n += 1,
                    _ => {}
                }
                n
            }
            1 => cx.st.gimmicks.get(ep.id as usize).map_or(0, |g| g.esize),
            2 => cx.st.npcs.get(ep.id as usize).map_or(0, |n| n.esize),
            _ => 0,
        };
        self.entry_object_n(cx, seam, ep, n)
    }

    /// `ccEntryCtrl::entryCircleObject(mc)` (gcmn 0x00430360): what an
    /// opened circle gives, by its entry's `param[1]` (-1: enemies, or 1 in
    /// 8 (`(ccRand() & 7) == 3`) a gimmick) and `param[2]` (-1: a
    /// registered row, or gimmick `ccRand() & 1`), made from a copy of its
    /// entry (`entRoot` 2, `land` -1, heading `plDirc`, `param` all -1; an
    /// enemy of a circle an event placed keeps `entRoot` 0 and `param[2]`
    /// 0), `param[3]` of them (-1: `entryObject(ep)`'s count).
    pub fn entry_circle_object(&mut self, cx: &mut Cx, seam: &mut dyn Seam, ent: &EntryParam, pl_dirc: F) {
        let mut kind = ent.param[1];
        if kind == -1 {
            kind = i32::from(cx.cc.rand() & 7 == 3);
        }
        let mut id = ent.param[2];
        if id == -1 {
            match kind {
                0 => id = random_row(cx),
                1 => id = cx.cc.rand() & 1,
                _ => {}
            }
        }
        let mut ep = *ent;
        ep.ty = kind;
        ep.id = id;
        ep.ent_root = 2;
        ep.land = -1;
        ep.dirc = [0, 0, pl_dirc, ONE];
        ep.param = [-1; 4];
        if ent.ent_root == 0 && kind == 0 {
            ep.param[2] = 0;
            ep.ent_root = 0;
        }
        if ent.param[3] == -1 {
            self.entry_object(cx, seam, &mut ep);
        } else {
            self.entry_object_n(cx, seam, &mut ep, ent.param[3]);
        }
    }

    /// `ccEntryCtrl::entryEnemyObject(obj)` (gcmn 0x00430250): a dead
    /// enemy taken away (`ccEnemy::main`'s removal) leaves, when its
    /// entry's `param[2]` is not 0 and `(ccRand() & 3) == 0`, a treasure
    /// box (gimmick `ccRand() & 1`) where it stood (`entRoot` 1, `land`
    /// -1, `param` -1), with the trap-removal effect and sound 215.
    pub fn entry_enemy_object(&mut self, cx: &mut Cx, seam: &mut dyn Seam, who: usize) {
        let e = cx.foes[who].as_ref().expect("an enemy's state");
        if e.ent.param[2] == 0 {
            return;
        }
        let (ent, dirc) = (e.ent, e.dirc);
        if cx.cc.rand() & 3 != 0 {
            return;
        }
        let id = cx.cc.rand() & 1;
        let mut ep = ent;
        ep.pos = cx.scene.chars[who].pos;
        ep.dirc = dirc;
        ep.ty = 1;
        ep.id = id;
        ep.ent_root = 1;
        ep.land = -1;
        ep.param = [-1; 4];
        self.entry_object(cx, seam, &mut ep);
        cx.out.push(Out::RemoveTrap { pos: ep.pos });
        cx.out.push(Out::Se { se: 215, pos: ep.pos });
    }

    /// `ccEntryCtrl::restoreEntry()` (gcmn 0x00431070): the lists kept in
    /// `g_entryList` when the area was left, each object `initObject`ed
    /// again (only those of the current place come back on).
    pub fn restore_entry(&mut self, cx: &mut Cx, saved: [List; 4]) {
        self.lists = saved;
        for k in [Kind::Enemy, Kind::Circle, Kind::Gimmick, Kind::Npc] {
            let mut cur = self.lists[k as usize].head;
            for _ in 0..self.lists[k as usize].num {
                let Some(c) = cur else { break };
                self.init_object(cx, c);
                cur = self.links[c].next;
            }
        }
    }

    /// `ccThEntryCtrlDelete(ctrl)` (gcmn 0x00431d10), when the task ends: with
    /// `keep` (the area left for a moment) the enemies and gimmicks circles and
    /// corpses made (`entRoot` not -1 or 0), the gimmicks fading out and every
    /// NPC are deleted, the rest kept; otherwise every object is deleted.
    /// Returns the lists as `g_entryList` keeps them for
    /// [`EntryCtrl::restore_entry`].
    pub fn leave(&mut self, cx: &mut Cx, keep: bool) -> [List; 4] {
        let owned = |r: i32| r != -1 && r != 0;
        for k in [Kind::Enemy, Kind::Circle, Kind::Gimmick, Kind::Npc] {
            if keep && k == Kind::Circle {
                continue;
            }
            let mut cur = self.lists[k as usize].head;
            let n = self.lists[k as usize].num;
            for _ in 0..n {
                let Some(c) = cur else { break };
                let next = self.links[c].next;
                let go = !keep
                    || match k {
                        Kind::Enemy => cx.foes[c].as_ref().is_some_and(|e| owned(e.ent.ent_root)),
                        Kind::Gimmick => self.entry_obj(c).is_some_and(|o| owned(o.ent.ent_root) || o.fade_flag == 2),
                        _ => true,
                    };
                if go {
                    self.delete(k, cx, c);
                }
                cur = next;
            }
        }
        self.lists
    }

    /// [`EntryCtrl::leave`] with what survives taken out of the scene:
    /// `g_entryList` and the objects it keeps (their characters, their
    /// entry objects and an enemy's state), list by list in list order,
    /// for the next area's [`EntryCtrl::restore_kept`]. The runtime makes
    /// a new scene for each area, so the objects move with the lists.
    /// A kept object's body is marked off the collision list, which the
    /// next area builds anew.
    pub fn keep(&mut self, cx: &mut Cx, keep: bool) -> KeptEntries {
        let lists = self.leave(cx, keep);
        let mut out = KeptEntries::default();
        for (k, l) in lists.iter().enumerate() {
            let mut cur = l.head;
            for _ in 0..l.num {
                let Some(c) = cur else { break };
                let mut obj = self.objs.get(c).cloned().unwrap_or_default();
                match &mut obj {
                    Obj::Circle(m) => m.obj.hit.sw = false,
                    Obj::Gimmick(o) | Obj::Npc(o) => o.hit.sw = false,
                    _ => {}
                }
                let mut foe = cx.foes.get(c).cloned().flatten();
                if let Some(e) = foe.as_mut() {
                    e.hit.sw = false;
                }
                out.lists[k].push((cx.scene.chars[c].clone(), obj, foe));
                out.olds[k].push(c);
                cur = self.links[c].next;
            }
        }
        out
    }

    /// `ccEntryCtrl::restoreEntry()` over [`KeptEntries`]: each kept object
    /// back into this control's scene at the end (new scene indices), the
    /// four lists rebuilt in their order, then each object `initObject`ed
    /// (only those of the current place come back on).
    /// Returns each object's new scene index, list by list in order (as
    /// [`KeptEntries::olds`] has the old ones).
    pub fn restore_kept(&mut self, cx: &mut Cx, kept: KeptEntries) -> Vec<usize> {
        let mut made = Vec::new();
        for (k, list) in kept.lists.into_iter().enumerate() {
            let kind = [Kind::Enemy, Kind::Circle, Kind::Gimmick, Kind::Npc][k];
            for (ch, obj, foe) in list {
                let who = Self::new_index(cx);
                self.grow(who + 1);
                Self::put_char(cx, who, ch);
                cx.foes[who] = foe;
                self.objs[who] = obj;
                // The object's own class names it by its new index (a
                // box's or an idol's rays follow it).
                if let Obj::Gimmick(o) | Obj::Npc(o) = &mut self.objs[who] {
                    o.class.moved_to(who);
                }
                self.append(kind, who);
                made.push(who);
            }
        }
        for &who in &made {
            self.init_object(cx, who);
        }
        made
    }

    /// One frame of `ccThEntryCtrl` (gcmn 0x00431970, the loop after
    /// `ccTscb::Breath`): the enemies, the magic circles, the gimmicks and
    /// the NPCs, each list from its head for as many objects as it held when
    /// its turn began (the next object taken before each runs): with
    /// `objFlag`, [`routine`] then its `main` (an enemy's, a gimmick's and
    /// an NPC's through [`Seam`], a circle's [`EntryCtrl::circle_main`]);
    /// a true `main` deletes it.
    pub fn frame(&mut self, cx: &mut Cx, seam: &mut dyn Seam) {
        for k in [Kind::Enemy, Kind::Circle, Kind::Gimmick, Kind::Npc] {
            let mut cur = self.lists[k as usize].head;
            let n = self.lists[k as usize].num;
            for _ in 0..n {
                let Some(c) = cur else { break };
                let next = self.links[c].next;
                if self.obj_flag(cx, c) {
                    self.run_routine(cx, c);
                    let done = match k {
                        Kind::Enemy => seam.enemy_main(self, cx, c),
                        Kind::Circle => self.circle_main(cx, seam, c),
                        Kind::Gimmick => seam.gimmick_main(self, cx, c),
                        Kind::Npc => seam.npc_main(self, cx, c),
                    };
                    if done {
                        self.delete(k, cx, c);
                    }
                }
                cur = next;
            }
        }
    }

    /// [`routine`] on object `who`.
    pub fn run_routine(&mut self, cx: &mut Cx, who: usize) {
        let pos = cx.scene.chars[who].pos;
        match &mut self.objs[who] {
            Obj::Enemy => {
                let e = cx.foes[who].as_mut().expect("an enemy's state");
                routine(cx.t.volume, enemy_routine_ref(e), pos, who, cx.scene, cx.world, cx.out);
            }
            Obj::Circle(c) => routine(cx.t.volume, c.obj.routine_ref(), pos, who, cx.scene, cx.world, cx.out),
            Obj::Gimmick(o) | Obj::Npc(o) => {
                routine(cx.t.volume, o.routine_ref(), pos, who, cx.scene, cx.world, cx.out)
            }
            Obj::None => {}
        }
    }

    /// `ccMagicCircle::main` (gcmn 0x00455b60): a closed circle far from Kite
    /// frees its particles; otherwise acts 0-4 (Kite within 3000 opens it, the
    /// animation, what it gives through `entryCircleObject`, and after 65
    /// frames it goes, counted in the save), then `createPart`, the particles'
    /// `main`s, the animation and the draw. The acts are in
    /// docs/engine/battle.md ("The magic circle").
    pub fn circle_main(&mut self, cx: &mut Cx, seam: &mut dyn Seam, who: usize) -> bool {
        let Obj::Circle(mut mc) = std::mem::take(&mut self.objs[who]) else {
            panic!("not a magic circle");
        };
        let r = self.circle_main_inner(cx, seam, who, &mut mc);
        // The object may have been replaced meanwhile only by itself.
        self.objs[who] = Obj::Circle(mc);
        r
    }

    fn circle_main_inner(&mut self, cx: &mut Cx, seam: &mut dyn Seam, who: usize, mc: &mut MagicCircle) -> bool {
        if mc.obj.act_num == 0 && mc.obj.freeze_flag {
            if mc.part_flag {
                cx.out.push(Out::CircleParts { who, made: false });
                mc.part_flag = false;
            }
            return false;
        }
        if !mc.part_flag {
            for p in mc.parts.iter_mut() {
                p.status = 0;
                p.eff = Eff { pat_num: cx.part_pats, ..Eff::default() };
            }
            cx.out.push(Out::CircleParts { who, made: true });
            mc.part_flag = true;
            mc.part_top = 0;
        }
        let ch = &mut cx.scene.chars[who];
        ch.pos_p = cx.world.w2p(ch.pos);
        ch.pos = cx.world.p2w(ch.pos_p);
        let pos = ch.pos;
        match mc.obj.act_num {
            0 => {
                let kite = cx.game.player.is_some_and(|p| cx.scene.listed(p));
                if kite && le(mc.obj.pl_dist, geom::k(3000.0)) {
                    cx.out.push(Out::Se { se: 216, pos });
                    cx.out.push(Out::Se { se: 217, pos });
                    mc.obj.act_num = 1;
                    mc.obj.act_cnt = 0;
                }
            }
            1 => {
                let c = mc.obj.act_cnt;
                mc.obj.act_cnt = c.wrapping_add(1);
                if c >= 31 {
                    cx.world.anim_set(who, AnmSlot::Main, "ANM_xmagcir2");
                    mc.obj.act_num = 2;
                    mc.obj.act_cnt = 0;
                    mc.obj.dest_flag = true;
                }
            }
            2 => {
                let c = mc.obj.act_cnt;
                mc.obj.act_cnt = c.wrapping_add(1);
                if c >= 21 {
                    cx.out.push(Out::Se { se: 215, pos });
                    let (ent, pl_dirc) = (mc.obj.ent, mc.obj.pl_dirc);
                    self.entry_circle_object(cx, seam, &ent, pl_dirc);
                    mc.obj.act_num = 3;
                    mc.obj.act_cnt = 0;
                }
            }
            3 => {
                mc.obj.act_cnt = mc.obj.act_cnt.wrapping_add(1);
                if mc.obj.anm_flag != 0 {
                    mc.obj.act_num = 4;
                    mc.obj.act_cnt = 0;
                }
            }
            4 => {
                let c = mc.obj.act_cnt;
                mc.obj.act_cnt = c.wrapping_add(1);
                if c >= 65 {
                    if mc.obj.ent.ent_root != 0 {
                        bump_count(cx.save, SAVE_CIRCLES);
                        if self.lists[Kind::Circle as usize].num == 1 {
                            cx.out.push(Out::AreaCleared);
                            match cx.game.area {
                                1 => bump_count(cx.save, SAVE_FIELDS_CLEARED),
                                2 => {
                                    if cx.game.field_type == 4 && cx.game.dungeon == 0 {
                                        bump_count(cx.save, SAVE_FIELDS_CLEARED);
                                    } else {
                                        bump_count(cx.save, SAVE_DUNGEONS_CLEARED);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    return true;
                }
            }
            _ => {}
        }
        mc.create_part(cx.scene.chars[who].pos);
        let frame = cx.t.volume == Volume::Inf || cx.game.player.is_some();
        for p in mc.parts.iter_mut() {
            if p.status != 0 {
                p.main(cx.world, cx.cc, frame);
            }
        }
        if mc.obj.act_num == 0 && !lt(mc.obj.pl_dist, geom::k(7000.0)) {
            return false;
        }
        mc.obj.anm_flag = i32::from(cx.world.anim_forward(who, AnmSlot::Main, 256));
        if !mc.obj.disp_sw {
            return false;
        }
        let pos = cx.scene.chars[who].pos;
        mc.obj.disp_sw = cx.world.camera_deg(pos, 12288);
        if !mc.obj.disp_sw {
            return false;
        }
        let t0 = cx.world.camera_transparency(pos, 0, 0, geom::k(7000.0), geom::k(600.0));
        let mut t = mul(t0, mc.obj.set_transparency);
        cx.out.push(Out::Layer(3));
        cx.out.push(Out::CircleDraw { who, transparency: t });
        for (k, p) in mc.parts.iter_mut().enumerate() {
            if p.status == 0 {
                continue;
            }
            t = mul(t, p.transparency);
            if !le(t, ONE) {
                t = ONE;
            }
            if lt(t, 0) {
                t = 0;
            }
            p.eff.transparency = t;
            cx.out.push(Out::PartDraw { who, part: k, transparency: t, pat: p.eff_anm_pat });
        }
        cx.out.push(Out::Layer(0));
        false
    }
}

/// `ccCharHit::SetHitSW(sw)` (main 0x001533f0): into the collision list or
/// out of it when that changes (`HitEnable`'s work, or `HitDisable`).
pub fn set_hit_sw(world: &mut dyn World, who: usize, hit: &mut CharHit, on: bool) {
    if hit.sw == on {
        return;
    }
    world.hit_switch(who, hit, on);
    hit.sw = on;
}

// where the entries come from ---------------------------------------------------------

/// What `WORLD::SetMagicCircle` reads of the field: `FIELD.check` and
/// `WORLD::GetHeight`. piney-data's generated field answers both.
pub trait FieldMap {
    /// `FIELD.check[cx][cy]` (FIELD +0xce430, a signed byte): the chip is
    /// taken.
    fn check(&self, cx: i32, cy: i32) -> i8;
    /// `WORLD::GetHeight(x, y)` (gcmn 0x005aa520).
    fn height(&self, x: F, y: F) -> F;
}

impl FieldMap for piney_data::field::Field {
    fn check(&self, cx: i32, cy: i32) -> i8 {
        self.check1()[(cx * 40 + cy) as usize] as i8
    }
    fn height(&self, x: F, y: F) -> F {
        self.get_height(x, y)
    }
}

/// `FIELD::CalcWorldMeshPosition(out, cx, cy)` (gcmn 0x005ae940): a chip's
/// centre, (600 + 1200 cx, 600 + 1200 cy, 0, 1).
pub fn chip_centre(cx: i32, cy: i32) -> V4 {
    let c = |v: i32| add(geom::k(600.0), mul(geom::k(1200.0), from_int(v)));
    [c(cx), c(cy), 0, ONE]
}

/// `ccGetDist(a, b)` (main 0x001d9dd0).
fn get_dist(a: V4, b: V4) -> F {
    crate::enemy_ai::get_dist(a, b)
}

/// `WORLD::SetMagicCircle()` (gcmn 0x005ab610): the field's 16 entries, 4, 8
/// or 12 of them magic circles by `circleOfs` (3: none; a story area: all),
/// one in each of the 4 x 4 blocks of 10 x 10 chips at a site `fieldrand`
/// finds; event area 14 draws the sites but makes no entry. Circles are
/// gimmick 15, the rest enemies of a registered row, all with `entRoot` and
/// `land` -1 (docs/engine/battle.md, "Where the entries come from").
#[allow(clippy::too_many_arguments)]
pub fn world_set_magic_circle(
    ctrl: &mut EntryCtrl,
    cx: &mut Cx,
    seam: &mut dyn Seam,
    field: &dyn FieldMap,
    rng: &mut piney_data::dungeon::Rng,
    circle_ofs: i32,
    event_area: i32,
    start: V4,
) {
    let mut is_circle = [false; 16];
    let n = match circle_ofs {
        0 => 4,
        1 => 8,
        2 => 12,
        3 => return,
        _ => 12,
    };
    for _ in 0..n {
        loop {
            let k = rng.below(16) as usize;
            if !is_circle[k] {
                is_circle[k] = true;
                break;
            }
        }
    }
    if cx.game.field != 0 {
        is_circle = [true; 16];
    }
    let mut placed: Vec<V4> = Vec::with_capacity(16);
    for gy in (0..40).step_by(10) {
        for gx in (0..40).step_by(10) {
            // The game counts taken chips toward 100 but clears the count at
            // the top of every try, so it tries until a site passes.
            loop {
                let a = rng.below(10) as i32;
                let b = rng.below(10) as i32;
                let (chx, chy) = (gx + a, gy + b);
                if field.check(chx, chy) != 0 {
                    continue;
                }
                let mut ep = entry_param_clear();
                ep.pos = chip_centre(chx, chy);
                ep.pos[2] = field.height(ep.pos[0], ep.pos[1]);
                let mut d = get_dist(start, ep.pos);
                for &p in &placed {
                    if lt(get_dist(p, ep.pos), geom::k(6000.0)) {
                        d = geom::k(4000.0);
                    }
                }
                if le(d, geom::k(3500.0)) {
                    continue;
                }
                placed.push(ep.pos);
                let k = placed.len() - 1;
                if event_area != 14 {
                    if is_circle[k] {
                        ep.ty = 1;
                        ep.id = MAGIC_CIRCLE;
                        ep.area = 1;
                        ep.land = -1;
                        ep.area_num = cx.game.field;
                        ep.x = geom::to_int(div(ep.pos[0], geom::k(600.0)));
                        ep.y = geom::to_int(div(ep.pos[1], geom::k(600.0)));
                    } else {
                        ep.ty = 0;
                        ep.id = -1;
                        ep.area = 1;
                        ep.land = -1;
                        ep.area_num = cx.game.field;
                    }
                    ctrl.entry_object(cx, seam, &mut ep);
                }
                if placed.len() == 16 {
                    return;
                }
                break;
            }
        }
    }
}

/// One field object as `WORLD::SetFood` and `WORLD::SetSpecialObj` read
/// it: its `wp` (FOBJECT +0x30, FOBJECT2 +0x40), whether it has an
/// animation (`anm`; else its clump is searched), and the places (the
/// world matrices' last rows, the object's own root at the identity) of
/// its nodes named `OBJ_xgfood0a*` and `OBJ_xgsymb*`, in the search's
/// order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldObj {
    pub wp: V4,
    pub anm: bool,
    pub food: Vec<V4>,
    pub symb: Vec<V4>,
}

/// What `WORLD::SetFood` and `WORLD::SetSpecialObj` read of the field:
/// `fobj2[0 .. KeyObjNum]` (the entrance, key, sub and lake objects),
/// `fobj[x][y]` in the setters' order (y the outer loop, the chips with an
/// object only), `water` (+0x30: the lake's index in `fobj2`, 0 without a
/// lake), `WORLD_MAN.inPoint[2]` (+0xd0, `GetDungeonMarkerPoint`) and
/// `eventAreaNumber` (+0x120: `game.field` in a field), and the field type.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldGims {
    pub fobj2: Vec<FieldObj>,
    pub fobj: Vec<FieldObj>,
    pub water: i32,
    pub in_point: [V4; 2],
    pub event_area: i32,
    pub field_type: i32,
}

/// `WORLD_MAN::GetFood()` (main 0x001a3960) in a field: the food row by
/// field type (23 past the table).
pub fn field_food(field_type: i32) -> i32 {
    const FOOD: [i32; 11] = [23, 24, 25, 26, 33, 27, 28, 29, 30, 31, 32];
    usize::try_from(field_type).ok().and_then(|t| FOOD.get(t)).copied().unwrap_or(23)
}

/// A field gimmick's entry: row `id` at `pos`, `area` 1, `entRoot` -1,
/// `game.field`'s, `land` 0.
fn field_entry(cx: &Cx, id: i32, pos: V4) -> EntryParam {
    let mut ep = entry_param_clear();
    ep.pos = pos;
    ep.ty = 1;
    ep.id = id;
    ep.area = 1;
    ep.ent_root = -1;
    ep.area_num = cx.game.field;
    ep.land = 0;
    ep
}

/// A node's place plus the object's `wp`, w 1.
fn node_place(node: V4, wp: V4) -> V4 {
    let p = geom::vadd(node, wp);
    [p[0], p[1], p[2], ONE]
}

/// `WORLD::SetFood()` (gcmn 0x005aa990). With a lake (`water` not 0), the
/// Spring of Myst (gimmick 20) at the lake's `wp`, z 0. Then for each
/// `fobj2` object, then each chip's `fobj`, with one to three
/// `OBJ_xgfood0a*` nodes: `fieldrand(n) + 1` foods (`GetFood()`), each at
/// a node not yet taken (`ccRand() % n`, signed, until a free one).
pub fn world_set_food(
    ctrl: &mut EntryCtrl,
    cx: &mut Cx,
    seam: &mut dyn Seam,
    g: &FieldGims,
    rng: &mut piney_data::dungeon::Rng,
) {
    if g.water != 0
        && let Some(o) = usize::try_from(g.water).ok().and_then(|k| g.fobj2.get(k))
    {
        let mut pos = o.wp;
        pos[2] = 0;
        let mut ep = field_entry(cx, 20, pos);
        ctrl.entry_object(cx, seam, &mut ep);
    }
    let food = field_food(g.field_type);
    for o in g.fobj2.iter().chain(g.fobj.iter()) {
        let n = o.food.len();
        if n == 0 || n >= 4 {
            continue;
        }
        let k = rng.below(n as u32) + 1;
        let mut taken = [false; 3];
        for _ in 0..k {
            // ccRand() is signed: a negative remainder reads the words
            // below `used[]` (the search's count and table pointer, never
            // 0), so the game draws again.
            let r = loop {
                let r = cx.cc.rand().wrapping_rem(n as i32);
                if r >= 0 && !taken[r as usize] {
                    break r as usize;
                }
            };
            taken[r] = true;
            let mut ep = field_entry(cx, food, node_place(o.food[r], o.wp));
            ctrl.entry_object(cx, seam, &mut ep);
        }
    }
}

/// `eventAreaNumber`s with no dungeon entrance's swirl
/// (`WORLD::SetSpecialObj`).
const NO_ENTRANCE: [i32; 20] = [28, 39, 40, 41, 42, 53, 54, 55, 56, 57, 78, 79, 80, 81, 82, 109, 110, 111, 112, 113];

/// `WORLD::SetSpecialObj()` (gcmn 0x005aae90): the dungeon entrance's two
/// in-points (`GetDungeonMarkerPoint`) each get gimmick 21 (`ENTRANCE`),
/// but in the areas of [`NO_ENTRANCE`]; then for each `fobj2` object with
/// `OBJ_xgsymb*` nodes (its anm's, else its clump's) and each chip's
/// `fobj` with them in its anm, when `fieldrand(100) >= 41` a symbol
/// (gimmick 18) at node `fieldrand(n)`. Type 7's fires (`firePos`) are
/// the runtime's ([`crate`]'s caller keeps them).
pub fn world_set_special_obj(
    ctrl: &mut EntryCtrl,
    cx: &mut Cx,
    seam: &mut dyn Seam,
    g: &FieldGims,
    rng: &mut piney_data::dungeon::Rng,
) {
    if !NO_ENTRANCE.contains(&g.event_area) {
        let mut ep = field_entry(cx, 21, g.in_point[0]);
        ctrl.entry_object(cx, seam, &mut ep);
        ep.pos = g.in_point[1];
        ctrl.entry_object(cx, seam, &mut ep);
    }
    let fobj = g.fobj.iter().filter(|o| o.anm);
    for o in g.fobj2.iter().chain(fobj) {
        let n = o.symb.len();
        if n == 0 || rng.below(100) < 41 {
            continue;
        }
        let r = rng.below(n as u32) as usize;
        let mut ep = field_entry(cx, 18, node_place(o.symb[r], o.wp));
        ctrl.entry_object(cx, seam, &mut ep);
    }
}

/// A `gimPos` slot (0x30 bytes) of the dungeon generator: position (+0),
/// floor (+0x20) and room (+0x21) as signed bytes, and what it holds
/// (+0x22: 0 item box, 1 magic circle, 2 fountain, 3-6 the special
/// objects; -1 used).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GimSlot {
    pub pos: V4,
    /// +0x10: its heading.
    pub dirc: V4,
    pub floor: i8,
    pub block: i8,
    pub kind: i8,
}

/// A story dungeon's `GIMMICKDATA` row (0x20 bytes) as
/// `DUNGEON::SetMagicCircle` reads it: floor, room, position in world
/// units, type (1 a magic circle). piney-data's `EditDungeon` rows
/// (`dungeon::GimmickData`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditGim {
    pub floor: i32,
    pub block: i32,
    pub x: i32,
    pub y: i32,
    pub ty: i32,
    /// +0x14 `kind`, +0x18 `flag` (a box's item word), +0x1c `direc`.
    pub kind: i32,
    pub flag: i32,
    pub direc: i32,
}

/// A story dungeon's `ROOMDATA` row as the gimmick setters read it:
/// floor, room, `type` (+0x10), `eventFlag` (+0x20) and `itemID` (+0x48,
/// the idol's item word).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EditRoom {
    pub floor: i32,
    pub block: i32,
    pub ty: i32,
    pub event: i32,
    pub item: i32,
}

/// What `DUNGEON::SetMagicCircle` reads and writes of `DUNGEON`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DungeonGims {
    /// `DUNGEON.type` (+0x10).
    pub dtype: i32,
    /// +0x14: a story dungeon.
    pub story: bool,
    /// `gimPos` (+0x424, 750 slots).
    pub slots: Vec<GimSlot>,
    /// `edit` (+0x3351c): the story dungeon's gimmick rows, if any.
    pub edit: Option<Vec<EditGim>>,
    /// +0x3c: the event entries numbered so far (`SetItemBox` counts
    /// first).
    pub counter: i32,
    /// The story dungeon's `ROOMDATA` rows.
    pub rooms: Vec<EditRoom>,
    /// `rotate[10][15]` (+0x2f488): each room's turn.
    pub rotate: Vec<[F; 15]>,
    /// +0x44 `lakeFlag`.
    pub lake: bool,
    /// `WORLD_MAN.timeSym` (+0x134).
    pub time_sym: i32,
    /// `WORLD_MAN::GetFieldAttrb()`.
    pub field_attr: i32,
    /// `WORLD_MAN::GetFood()`: the food row of the area.
    pub food: i32,
    /// For each `edit` row, where `DUNGEON::GetNearDoorPosition` (gcmn
    /// 0x005c9ba0) puts a boss room's warning (row 19): the room's gate
    /// dummy (`OBJ_w_0g10_*`, with none its door dummies, `OBJ_0pae0_*`)
    /// nearest the row's place within 12000; None where there is none (the
    /// game's stack words then).
    pub warn_pos: Vec<Option<V4>>,
    /// `DUNGEON::GetBanRoom` found a banned room: no warning is made.
    pub ban_room: bool,
}

/// The first slot holding `kind`, used (set to -1).
pub(crate) fn take_slot(d: &mut DungeonGims, kind: i8) -> Option<GimSlot> {
    let s = d.slots.iter_mut().take(750).find(|s| s.kind == kind)?;
    s.kind = -1;
    Some(*s)
}

/// `DUNGEON::SetMagicCircle()` (gcmn 0x005bf590): a magic circle (gimmick
/// 15, `entRoot` -1, z 0) for every `gimPos` slot of kind 1; in a story
/// dungeon one for every edit row of type 1 (its position as floats, the
/// `w` the caller's stack leaves, taken as 1 here; `param[0]` the next
/// event entry number); and in the lake dungeons (types 8, 9) the special
/// objects (gimmick 18, `land` 0) of the slots of kinds 3 to 6, in that
/// order. Every entry is made with `entryObject(ep)` on its own floor and
/// room of `game.dungeon`.
pub fn dungeon_set_magic_circle(ctrl: &mut EntryCtrl, cx: &mut Cx, seam: &mut dyn Seam, d: &mut DungeonGims) {
    while let Some(s) = take_slot(d, 1) {
        let mut ep = entry_param_clear();
        ep.pos = s.pos;
        ep.pos[2] = 0;
        ep.ty = 1;
        ep.id = MAGIC_CIRCLE;
        ep.area = 2;
        ep.floor = i32::from(s.floor);
        ep.block = i32::from(s.block);
        ep.ent_root = -1;
        ep.area_num = cx.game.dungeon;
        ctrl.entry_object(cx, seam, &mut ep);
    }
    if d.story
        && let Some(edit) = d.edit.clone()
    {
        for g in edit.iter() {
            if g.ty != 1 {
                continue;
            }
            let mut ep = entry_param_clear();
            ep.pos = [from_int(g.x), from_int(g.y), 0, ONE];
            ep.ty = 1;
            ep.id = MAGIC_CIRCLE;
            ep.area = 2;
            ep.floor = g.floor;
            ep.block = g.block;
            ep.ent_root = -1;
            ep.area_num = cx.game.dungeon;
            ep.param[0] = d.counter;
            d.counter = d.counter.wrapping_add(1);
            ctrl.entry_object(cx, seam, &mut ep);
        }
    }
    if d.dtype != 8 && d.dtype != 9 {
        return;
    }
    for kind in 3..=6 {
        while let Some(s) = take_slot(d, kind) {
            let mut ep = entry_param_clear();
            ep.pos = s.pos;
            ep.ty = 1;
            ep.id = 18;
            ep.area = 2;
            ep.floor = i32::from(s.floor);
            ep.block = i32::from(s.block);
            ep.ent_root = -1;
            ep.area_num = cx.game.dungeon;
            ep.land = 0;
            ctrl.entry_object(cx, seam, &mut ep);
        }
    }
}

/// The other setters `WORLD_MAN::EntryGimmick` calls, which place gimmicks
/// from the field's and the dungeon rooms' dummy nodes and draw `fieldrand`
/// and `ccRand`: `WORLD::SetFood` (gcmn 0x005aa990), `SetSpecialObj`
/// (0x005aae90), `DUNGEON::SetItemBox` (0x005beb30), `SetIDOL` (0x005be750),
/// `EntryBreakObject` (0x005bff10), and the two magic-circle setters. Each is
/// called where `EntryGimmick` calls it.
pub trait EntryGimmickSeam {
    fn set_food(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx);
    fn world_set_magic_circle(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx);
    fn set_special_obj(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx);
    fn set_item_box(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx);
    fn dungeon_set_magic_circle(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx);
    fn set_idol(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx);
    fn entry_break_object(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx);
}

/// `WORLD_MAN`'s members `EntryGimmick` reads and writes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldMan {
    /// `flag` (+0x8): 1 a field, 2 a dungeon.
    pub flag: i32,
    /// `entryFlag[4]` (+0x54): the field's (0) and each dungeon's (1-3)
    /// gimmicks placed.
    pub entry_flag: [i32; 4],
    /// `eventAreaFlag` (+0x124): a hand-made event map.
    pub event_area_flag: i32,
}

/// `volumeNum` (main 0x0034bbf8): 1 in Infection.
pub const VOLUME: i32 = 1;

/// `WORLD_MAN::EntryGimmick()` (main 0x001a1f20), from `ccThEntryCtrl`'s
/// set-up in a field or dungeon: a field's gimmicks once (`SetFood`,
/// `SetMagicCircle`, `SetSpecialObj`; not on a hand-made event map; the
/// game never sets `entryFlag[0]` here); a dungeon's once per dungeon
/// (`SetItemBox`, `SetMagicCircle` but in volume 2's field 27, `SetIDOL`,
/// then `entryFlag[1 + dungeon]`), and its breakable objects every time.
pub fn entry_gimmick(wm: &mut WorldMan, ctrl: &mut EntryCtrl, cx: &mut Cx, s: &mut dyn EntryGimmickSeam) {
    match wm.flag {
        1 => {
            if (cx.game.field == 0 || wm.event_area_flag == 0) && wm.entry_flag[0] == 0 {
                s.set_food(ctrl, cx);
                s.world_set_magic_circle(ctrl, cx);
                s.set_special_obj(ctrl, cx);
            }
        }
        2 => {
            let k = 1 + cx.game.dungeon as usize;
            if wm.entry_flag[k] == 0 {
                s.set_item_box(ctrl, cx);
                if !(VOLUME == 2 && cx.game.field == 27) {
                    s.dungeon_set_magic_circle(ctrl, cx);
                }
                s.set_idol(ctrl, cx);
                wm.entry_flag[k] = 1;
            }
            s.entry_break_object(ctrl, cx);
        }
        _ => {}
    }
}

/// An `evPos` of the event manager (`eventMng` +0x1c0, 0x20 bytes): floor,
/// block (9999 both: relative to Kite), number, heading, position; the
/// event VM's `set_pos` and `marker_pos` fill them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EvPos {
    pub floor: i16,
    pub block: i16,
    pub num: i32,
    pub dirc: F,
    pub pos: V4,
}

/// The position an event entry's marker names: the `evPos` of that number
/// (Kite's position added when its floor and block are 9999 or more), else
/// (0, 0, 0, 1).
pub(crate) fn event_pos(positions: &[EvPos], marker: i16, kite: V4) -> Option<EvPos> {
    let p = positions.iter().take(16).find(|p| p.num == i32::from(marker))?;
    let mut e = *p;
    if p.floor >= 9999 && p.block >= 9999 {
        e.pos = geom::vadd(e.pos, kite);
    }
    Some(e)
}

/// The entry `ccEntryEventMng` (main 0x001b62e0) makes for an event's
/// `entry_mc TYPE CODE MARKER PARAM` (type 0 or 1): a magic circle (gimmick
/// 15, `entRoot` 0) at the marker's position ([`EvPos`]) on the current area,
/// floor and block, giving `param[1]` TYPE, `param[2]` CODE and `param[3]`
/// PARAM (0 and 1 mean 1). The runtime hands it to [`EntryCtrl::entry_object`];
/// in a dungeon the game also closes the room's doors (`DUNGEON::CloseDoor`).
pub fn event_magic_circle(e: [i16; 4], positions: &[EvPos], kite: V4, game: &Game) -> Option<EntryParam> {
    if e[0] != 0 && e[0] != 1 {
        return None;
    }
    let mut ep = entry_param_clear();
    ep.pos = event_pos(positions, e[2], kite).map_or(ZERO_POS, |p| p.pos);
    ep.ty = 1;
    ep.id = MAGIC_CIRCLE;
    ep.area = game.area;
    ep.floor = game.floor;
    ep.block = game.block;
    ep.ent_root = 0;
    ep.x = geom::to_int(div(ep.pos[0], geom::k(600.0)));
    ep.y = geom::to_int(div(ep.pos[1], geom::k(600.0)));
    match game.area {
        1 => ep.area_num = game.field,
        2 => ep.area_num = game.dungeon,
        _ => {}
    }
    ep.param[0] = -1;
    ep.param[1] = i32::from(e[0]);
    ep.param[2] = i32::from(e[1]);
    ep.param[3] = if e[3] == 0 || e[3] == 1 { 1 } else { i32::from(e[3]) };
    Some(ep)
}

/// The entry `ccEntryEventMng` makes for an event's `entry TYPE CODE
/// MARKER PARAM` of type 5 or 6 (an enemy or object of the event): an
/// enemy of row CODE (`entRoot` 0, `param[2]` 0) at the marker's position
/// facing its heading, on the current area, floor and block; the runtime
/// makes PARAM of them (0 and 1 mean 1) with
/// [`EntryCtrl::entry_object_n`]. Returns the entry and the count.
pub fn event_enemy(e: [i16; 4], positions: &[EvPos], kite: V4, game: &Game) -> (EntryParam, i32) {
    let mut ep = entry_param_clear();
    match event_pos(positions, e[2], kite) {
        Some(p) => {
            ep.pos = p.pos;
            ep.dirc = [0, 0, p.dirc, ONE];
        }
        None => {
            ep.pos = ZERO_POS;
            ep.dirc = ZERO_POS;
        }
    }
    ep.ty = 0;
    ep.id = i32::from(e[1]);
    ep.area = game.area;
    ep.floor = game.floor;
    ep.block = game.block;
    ep.ent_root = 0;
    ep.param[2] = 0;
    match game.area {
        1 => ep.area_num = game.field,
        2 => ep.area_num = game.dungeon,
        _ => {}
    }
    let n = if e[3] == 0 || e[3] == 1 { 1 } else { i32::from(e[3]) };
    (ep, n)
}

// The enemies' own frame -------------------------------------------------------------

/// The enemies' `main` for [`EntryCtrl::frame`]: a [`Seam`] whose
/// [`Seam::enemy_main`] runs `ccEnemy::main` (0x00432cd0) as
/// [`Motion::enemy_main`] on the entry control's scene, with `cx.world` as the
/// motion's world. The enemy book's record of a Data Drain goes into the save
/// at once; a corpse taken away (`entryEnemyObject`) and a drain's spawn of
/// the drained form are done as `main` returns, as the game does them last.
/// The gimmicks' and NPCs' `main`s go to [`EnemySeam::inner`]; a race
/// [`crate::enemy_motion`] does not move should not be spawned.
pub struct EnemySeam<'s> {
    /// The animation names and sound categories.
    pub data: &'s MotionData,
    /// What the affects read: the party, `ccMenu`, `ccSkillCheck`.
    pub affect: &'s AffectCtx<'s>,
    /// The environment `CalcReal` and the damage read.
    pub env: &'s Env,
    /// newlib's `rand()` (the damage rolls, the skill requests).
    pub rand: &'s mut dyn Rng,
    /// The player's frame the rules read (`ccTransPosW2P` in the enemies'
    /// decisions), unrecorded.
    pub frame: &'s dyn Frame,
    /// `eventMng->puppetShow`.
    pub puppet_show: bool,
    /// `pgRideFlag`.
    pub ride: bool,
    /// `ccPartyManager.memberChar[0]`: the player.
    pub player: Option<usize>,
    /// `worldman`'s area words A, B, C (+0x14c): where the enemy book
    /// records a kill.
    pub book_area: [i32; 3],
    /// The gimmicks' and NPCs' mains and the other constructors.
    pub inner: &'s mut dyn Seam,
}

/// What the enemy's frame left for the entry control.
enum Todo {
    Remove,
    Drain(enemy_ai::DrainSpawn),
}

/// The world as the enemy's frame sees it: `cx.world`, with the calls
/// that are the entry control's or the save's taken out.
struct Carry<'c> {
    w: &'c mut dyn MotionWorld,
    save: &'c mut SaveData,
    server: i32,
    area: [i32; 3],
    todo: &'c mut Vec<Todo>,
}

impl World for Carry<'_> {
    fn w2p(&mut self, pos: V4) -> V4 {
        self.w.w2p(pos)
    }
    fn p2w(&mut self, pos: V4) -> V4 {
        self.w.p2w(pos)
    }
    fn land(&mut self, pos: V4, mask: u32) -> F {
        self.w.land(pos, mask)
    }
    fn hit_attribute(&mut self) -> u32 {
        self.w.hit_attribute()
    }
    fn line(&mut self, from: V4, to: V4, mask: u32, kind: i32) -> F {
        self.w.line(from, to, mask, kind)
    }
    fn collide(&mut self, who: usize, hit: &mut CharHit) -> i32 {
        self.w.collide(who, hit)
    }
    fn hit_char_type(&mut self) -> u32 {
        self.w.hit_char_type()
    }
    fn hit_switch(&mut self, who: usize, hit: &mut CharHit, on: bool) {
        self.w.hit_switch(who, hit, on)
    }
    fn camera_deg(&mut self, pos: V4, deg: i16) -> bool {
        self.w.camera_deg(pos, deg)
    }
    fn camera_transparency(&mut self, pos: V4, width: F, height: F, far: F, len: F) -> F {
        self.w.camera_transparency(pos, width, height, far, len)
    }
    fn anim_set(&mut self, who: usize, slot: AnmSlot, name: &str) {
        self.w.anim_set(who, slot, name)
    }
    fn anim_frame(&mut self, who: usize, slot: AnmSlot) -> u16 {
        self.w.anim_frame(who, slot)
    }
    fn anim_forward(&mut self, who: usize, slot: AnmSlot, step: u16) -> i16 {
        self.w.anim_forward(who, slot, step)
    }
    fn anim_notes(&mut self, who: usize, slot: AnmSlot) -> Vec<Note> {
        self.w.anim_notes(who, slot)
    }
}

impl MotionWorld for Carry<'_> {
    fn shake_range(&mut self, pos: V4) -> bool {
        self.w.shake_range(pos)
    }
    fn call(&mut self, who: usize, c: Call, at: &mut At) {
        match c {
            Call::Rule(enemy_ai::Out::KillRecord { ene_id }) => {
                enemy_ai::record_kill(self.save, ene_id, self.server, self.area);
            }
            Call::Rule(enemy_ai::Out::Remove) => self.todo.push(Todo::Remove),
            Call::Rule(enemy_ai::Out::DrainSpawn(sp)) => self.todo.push(Todo::Drain(sp)),
            c => self.w.call(who, c, at),
        }
    }
}

impl Seam for EnemySeam<'_> {
    fn enemy_main(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx, who: usize) -> bool {
        let active = ctrl.check_active_enemy(cx);
        let mut todo = Vec::new();
        let r = {
            let world = enemy_ai::World {
                puppet_show: self.puppet_show,
                ride: self.ride,
                active_enemies: active,
                player: self.player,
                frame: self.frame,
            };
            let mut ai = Ai {
                t: cx.t,
                scene: &mut *cx.scene,
                foes: &mut cx.foes[..],
                world,
                rand: &mut *self.rand,
                cc: &mut *cx.cc,
                out: Vec::new(),
            };
            let mut w = Carry {
                w: &mut *cx.world,
                save: &mut *cx.save,
                server: cx.game.server,
                area: self.book_area,
                todo: &mut todo,
            };
            let mut m = Motion { ai: &mut ai, w: &mut w, data: self.data, affect: self.affect, env: self.env };
            m.enemy_main(who)
        };
        for x in todo {
            match x {
                Todo::Remove => ctrl.entry_enemy_object(cx, self, who),
                Todo::Drain(sp) => {
                    let mut ep = sp.ent;
                    if let Some(n) = ctrl.entry_object_n(cx, self, &mut ep, 1) {
                        if let Some(e) = cx.foes.get_mut(n).and_then(Option::as_mut) {
                            e.transparency = ONE;
                            e.set_transparency = ONE;
                            enemy_ai::after_drain_spawn(e, &mut cx.scene.chars[n], &sp);
                        }
                        cx.out.push(Out::AfterDrain { who: n });
                    }
                }
            }
        }
        r == 1
    }
    fn gimmick_main(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx, who: usize) -> bool {
        let in_battle = self.env.in_battle;
        if let Some(r) = crate::gimmick::main(ctrl, cx, self, in_battle, who) {
            return r;
        }
        self.inner.gimmick_main(ctrl, cx, who)
    }
    fn npc_main(&mut self, ctrl: &mut EntryCtrl, cx: &mut Cx, who: usize) -> bool {
        self.inner.npc_main(ctrl, cx, who)
    }
    fn make_enemy(&mut self, cx: &mut Cx, race: i32, ent: &EntryParam, who: usize) -> (Char, Enemy) {
        self.inner.make_enemy(cx, race, ent, who)
    }
    fn make_gimmick(&mut self, cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, EntryObj) {
        self.inner.make_gimmick(cx, ent, who)
    }
    fn make_npc(&mut self, cx: &mut Cx, ent: &EntryParam, who: usize) -> (Char, EntryObj) {
        self.inner.make_npc(cx, ent, who)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rand_s_rotates() {
        let mut s = 0u16;
        let a = rand_s(&mut s);
        // (0 ^ 0x1100) - 25939 = 0x1100 - 0x6553 = 0xabad; rotl 2 = 0xaeb6.
        assert_eq!(s, 0xaeb6);
        assert_eq!(a, 0xaeb6u16 as i16);
    }

    #[test]
    fn event_entries_address_the_story_field() {
        let mut save = SaveData::new();
        save.set_i32(SAVE_EVENT_ENTRY + 24 * 14 + 4, 1 << 3);
        assert!(check_event_entry(&save, 14, 35));
        assert!(!check_event_entry(&save, 14, 34));
        assert!(!check_event_entry(&save, 0, 35));
        assert!(!check_event_entry(&save, 14, 6 * 32));
        clear_event_entry(&mut save, 14, 35);
        assert!(!check_event_entry(&save, 14, 35));
    }

    #[test]
    fn particles_take_the_first_free_slot() {
        let mut mc = MagicCircle {
            obj: EntryObj::default(),
            part_flag: true,
            part_top: 0,
            parts: vec![McPart::default(); PARTS],
        };
        mc.parts[0].status = 1;
        mc.set_part(2, &UNIT, 2);
        assert_eq!((mc.parts[1].status, mc.parts[2].status, mc.part_top), (2, 2, 0));
        for p in mc.parts.iter_mut() {
            p.status = 1;
        }
        mc.set_part(3, &UNIT, 1);
        assert_eq!((mc.parts[0].status, mc.part_top), (3, 1));
    }

    #[test]
    fn fade_wraps_alpha() {
        let mut p = McPart::default();
        p.eff.color = 0xff01_0203;
        p.cnt = 4;
        p.fade();
        assert_eq!(p.eff.color, 0x0000_0102);
    }

    #[test]
    fn chips_are_1200_apart() {
        assert_eq!(chip_centre(0, 1), [geom::k(600.0), geom::k(1800.0), 0, ONE]);
    }
}
