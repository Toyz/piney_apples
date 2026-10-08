//! The fights in a field and a dungeon: piney-battle's scene run in the
//! field's tasks. [`Combat`] keeps what the battle's code works on - the
//! characters ([`Scene`]), the party AI's bus ([`Crew`]), the party
//! ([`Party`]), the running skills ([`Skills`]), the entry control
//! ([`EntryCtrl`]), `ccThSpc`'s globals and both generators - and runs one
//! frame of the tasks in the game's order ([`Combat::frame`]): `ccThSpc`,
//! `ccThAISystem`, `ccThPlayer`, each `ccThFellowNN`, `ccThEntryCtrl`,
//! `ccThSkill` (docs/engine/battle.md, "How it plugs into piney-world").

pub mod boss;
pub mod breath;
pub mod cast;
pub mod chat;
pub mod dust;
pub mod gate_out;
pub mod ride;
pub mod spc;
pub mod stage;
pub mod town;
pub mod weapon;

use std::cell::RefCell;
use std::rc::Rc;

use piney_battle::Char as BChar;
use piney_battle::affect::{self, AffectCtx};
use piney_battle::chara::CondFx;
use piney_battle::chara::{AffectFunc, Env, spc_flag};
use piney_battle::enemy_ai::{self, Enemy, EntryParam};
use piney_battle::enemy_motion::MotionData;
use piney_battle::entry::{self, EntryCtrl, EntryObj, Register, SpawnTables};
use piney_battle::event::{Event, Events, Who};
use piney_battle::evparty::{self, EvParty, Roster};
use piney_battle::exp::{self, Party};
use piney_battle::fellow::{self, MotionTables};
use piney_battle::flow::{self, Skills};
use piney_battle::frame::{SpcOut, SpcThread};
use piney_battle::kite::{self, KiteTables, MapBounds};
use piney_battle::param::{SpcParam, cond};
use piney_battle::party_ai::{self, Ai as BAi, Crew, Parts, Runtime};
use piney_battle::party_motion::{Keep, Movement, Share};
use piney_battle::rand::{Genrand, Rand, Rng};
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_battle::world::{AnmSlot, CharHit};
use piney_data::iso::Iso;
use piney_data::save::SaveData;

use crate::body::Body;
use crate::camera::{CamPad, Camera};
use crate::ee::{self, F, ONE, V4};
use crate::hit::Hits;

pub use cast::{Actor, Cast, Look, Looks};
pub use stage::{Show, Stage};

/// What the battle's code reads from the disc: the rule tables, Kite's
/// and the members' clips, the entry control's tables, the enemies'
/// clips and sound categories.
pub struct BattleData {
    /// The disc's volume, whose tables these are.
    pub volume: piney_data::volume::Volume,
    pub t: Tables,
    pub kt: KiteTables,
    pub mt: MotionTables,
    pub st: SpawnTables,
    pub md: MotionData,
    /// Skeith's tables (`piney_battle::boss`), None if they did not read.
    pub skeith: Option<piney_battle::boss::SkeithData>,
    /// Every boss's tables (`piney_battle::boss::BossData`).
    pub bosses: Option<piney_battle::boss::BossData>,
}

impl BattleData {
    pub fn read(iso: &mut Iso) -> piney_data::Result<BattleData> {
        let volume = iso.volume()?;
        let t = Tables::read(iso)?;
        let st = SpawnTables::of(volume);
        let md = MotionData::read_iso(iso, &t)?;
        Ok(BattleData {
            volume,
            t,
            kt: KiteTables::of(volume),
            mt: MotionTables::of(volume),
            st,
            md,
            skeith: Some(piney_battle::boss::SkeithData::of(volume)),
            bosses: Some(piney_battle::boss::BossData::of(volume)),
        })
    }
}

/// What a frame of the battle's tasks reads of the field around it.
pub struct Tasks<'a> {
    pub hits: &'a mut Hits,
    pub camera: &'a mut Camera,
    pub pad: CamPad,
    pub save: &'a mut SaveData,
    /// `game.area`, `town`, `field`, `dungeon`, `floor`, `block`, `server`.
    pub scene: crate::area::Scene,
    pub wm: crate::area::WorldMan,
    /// `ccMenuCtrl::CheckMenuType()`, `forbid`.
    pub menu_type: i32,
    pub menu_forbid: bool,
    /// `eventMng->puppetShow` (+0x78c).
    pub puppet_show: bool,
    /// `cmndTarget != 0`.
    pub cmnd_target: bool,
    /// `ccSys.count`.
    pub count: u32,
    /// `WORLD_MAN::CheckEventArea()`.
    pub event_area: bool,
    /// `WORLD_MAN.dungeonPos[0]` (+0x460, `WORLD::SetDungeonEnter`): the
    /// field's entrance, z 0; zero elsewhere. Its w is never written; the
    /// port's is 1 (only x and y are read).
    pub dungeon: V4,
    /// The dungeon's floor map and the room's window.
    pub map2d: Vec<u8>,
    pub map2d_info: [i32; 3],
    /// `SetPathFindingMap()` (gcmn 0x00515aa0) is due before this frame's
    /// tasks: `ccThSpc`'s start, or the last frame's `DUNGEON::Draw` after
    /// a room change (`roomEnterFlag`), its `MakeMiniMap` done.
    pub path_map: bool,
    /// `spcConditionEffectFlag` (0x00378ce4): the condition tints pulse,
    /// off while an event's `menu_ban` holds.
    pub effect_sw: bool,
    /// The registry and the party (`ccSpcManager`, `ccPartyManager`, the
    /// game's globals): a member's remote command 5 leaves it here as in a
    /// town. None where no party is kept (the fights' harnesses).
    pub spcs: Option<&'a mut crate::party::Spcs>,
}

/// What the field's effects (`ccThEffect`, `ccThParticle`, which live
/// above this crate) read of the battle at their places in the frame.
pub struct FxWorld<'a> {
    pub t: &'a Tables,
    /// Mutable for the effects' damage calls ([`FxWorld::skill_damage`]).
    pub scene: &'a mut Scene,
    pub foes: &'a [Option<Enemy>],
    pub ctrl: &'a EntryCtrl,
    pub cast: &'a Cast,
    pub party: &'a Party,
    pub kite: Option<usize>,
    /// The party's characters: (registry id, scene index), Kite first.
    pub members: &'a [(i32, usize)],
    /// newlib's `rand()` and `genrand()`: the game's two generators,
    /// which the effects draw from too.
    pub rand: &'a mut Rand,
    pub cc: &'a mut Genrand,
    pub camera: &'a Camera,
    pub hits: &'a mut Hits,
    pub bounds: MapBounds,
    /// `game` +0x14 `area`, +0x28 `dungeon`, `WORLD_MAN::GetFieldType()`.
    pub area: (i32, i32, i32),
    /// `game` +0x24 `field`.
    pub field: i32,
    /// The frame's presentation the effects have not seen yet, in order.
    pub shows: &'a [Show],
    /// In `ccThEffect` and `ccThSkill`'s systems: what the effects' damage
    /// calls act on (None in the passes that make none).
    pub damage: Option<FxDamage<'a>>,
}

/// What the effects' damage calls act on where the game makes them, in
/// `ccThEffect` (a spell's element) or a run's system in `ccThSkill`: the
/// runs (their `acFlag`), the frame's env and its affects.
pub struct FxDamage<'a> {
    pub skills: &'a RefCell<Skills>,
    pub env: &'a Env,
    pub ev: &'a mut Events,
}

impl FxWorld<'_> {
    /// One of the effects' damage calls, made at once as the game makes it
    /// (it draws `rand()` between the effects' own draws). The calls with an
    /// `acFlag` take their run's, and do nothing once the run is gone
    /// (`ccSkillDamage(ccChar *, ccChar *, ccSkill *)`, gcmn 0x00573d40,
    /// checks `SkillEntryTop` first).
    pub fn skill_damage(&mut self, call: SpellDamage) {
        let Some(d) = self.damage.as_mut() else { return };
        let t = self.t;
        let player = self.kite.map(|k| self.scene.chars[k].pos);
        let bounds = self.bounds;
        let w2p = |p: V4| player.map_or(p, |pl| kite::w2p_pos(&bounds, pl, p).0);
        let ac_of = |key: u32| d.skills.borrow().runs.iter().find(|r| r.key == key).map(|r| r.ac_flag);
        let (spell, sid) = match call {
            SpellDamage::Target { spell, sid, .. } | SpellDamage::Area { spell, sid, .. } => (Some(spell), sid),
            SpellDamage::At { sid, .. } => (None, sid),
        };
        let Some(sk) = t.skill(sid) else { return };
        let mut ac = match spell.map(ac_of) {
            Some(None) => return,
            Some(Some(ac)) => ac,
            None => 0,
        };
        let (scene, rand) = (&mut *self.scene, &mut *self.rand);
        match call {
            SpellDamage::Target { attacker: Some(a), target: Some(tg), .. } => {
                piney_battle::damage::skill_damage(t, scene, a, tg, sk, &mut ac, sid, rand, d.env, d.ev);
            }
            SpellDamage::At { attacker: Some(a), pos, ttype, .. } => {
                piney_battle::damage::skill_damage_at(t, scene, a, w2p(pos), ttype, sk, sid, rand, d.env, d.ev);
            }
            SpellDamage::Area { attacker: Some(a), target, pos, ttype, .. } => {
                let tg = target.unwrap_or(a);
                let pos = w2p(pos);
                piney_battle::damage::skill_damage2(t, scene, a, tg, pos, ttype, sk, &mut ac, sid, rand, d.env, d.ev);
            }
            _ => {}
        }
        if let Some(key) = spell
            && let Some(r) = d.skills.borrow_mut().runs.iter_mut().find(|r| r.key == key)
        {
            r.ac_flag = ac;
        }
    }
}

/// The field's effect tasks, run where the game runs them: `ccThEffect`
/// (80) after the entry control and before `ccThSkill`, `ccThParticle`
/// (98) after it. Each first starts what the tasks before it asked for
/// ([`FxWorld::shows`]).
pub trait FxTasks {
    /// The starters for `w.shows`, then `ccThEffect`.
    fn effect(&mut self, _w: &mut FxWorld) {}
    /// The starters for `w.shows` (`ccThSkill`'s), then `ccThParticle`.
    fn particle(&mut self, _w: &mut FxWorld) {}
    /// `ccSkill::Main`'s call of an attack spell's element system for one
    /// run, in `ccThSkill`'s walk (the spell made at its first call, as
    /// `_ccSkillRequest` makes it): what the system did.
    fn spell(&mut self, _w: &mut FxWorld, _run: &SpellRun) -> SpellOut {
        SpellOut::default()
    }
    /// `ccThSkill` deleted the run.
    fn spell_remove(&mut self, _key: u32) {}
    /// `ccSkill::Main`'s step of a run's own animation (`ANM_<file>` of
    /// the skill's file, made at the run's first call as `_ccSkillRequest`
    /// makes it): `_AnimateForward(frameSpd)` and the notes `NoteProcess`
    /// hands on. None when there is no such animation.
    fn skill_anim(&mut self, _key: u32, _file: &str) -> Option<flow::AnimFrame> {
        None
    }
    /// `ccThBossEffect` (66), before the boss's task: the starters for
    /// `w.shows`, then `ccBossEffManager::Draw` over the boss's effects.
    fn boss_effects(&mut self, _w: &mut FxWorld) {}
    /// After the boss's task: the starters for `w.shows` - the
    /// `ccBossEff*Create`s its Main made (`Show::Boss`, `Out::Effect`).
    fn boss_shows(&mut self, _w: &mut FxWorld) {}
}

/// A running attack spell as `ccSkill::Main` hands it to its system.
#[derive(Clone, Copy, Debug)]
pub struct SpellRun {
    pub key: u32,
    pub sid: i32,
    pub stype: i8,
    /// `count` before Main's increment.
    pub count: i16,
    /// `holdFlag`: the targets held while the system runs.
    pub hold: bool,
    pub c_pos: V4,
    pub c_dirc: V4,
    pub t_pos: V4,
    pub t_type: i32,
    pub creator: Option<usize>,
    pub target: Option<usize>,
}

/// A damage call of a spell's system or element, in the game's words;
/// `spell` names the run whose `acFlag` it takes.
#[derive(Clone, Copy, Debug)]
pub enum SpellDamage {
    /// `ccSkillDamage(attacker, target, sk, &acFlag, sid)`.
    Target { spell: u32, attacker: Option<usize>, target: Option<usize>, sid: i32 },
    /// `ccSkillDamage(attacker, pos, tType, sk, sid)`, `pos` in the world.
    At { attacker: Option<usize>, pos: V4, ttype: i32, sid: i32 },
    /// `ccSkillDamage2(attacker, target, pos, tType, sk, &acFlag, sid)`.
    Area { spell: u32, attacker: Option<usize>, target: Option<usize>, pos: V4, ttype: i32, sid: i32 },
}

/// What a spell's system left: its `endFlag`, `holdFlag` and `level`, and
/// the casters it let act again (its damage calls are made at once,
/// [`FxWorld::skill_damage`]).
#[derive(Clone, Debug, Default)]
pub struct SpellOut {
    pub ran: bool,
    pub status: i8,
    pub hold: bool,
    pub level: i32,
    pub released: Vec<usize>,
}

/// What a dungeon hands the entry control's set-up: the objects kept from
/// the room before (`g_entryList`), and, the first time in the dungeon,
/// what `EntryGimmick`'s setters place with the dungeon's `fieldrand`.
pub struct DungeonEntries {
    pub kept: Option<entry::KeptEntries>,
    /// The kept objects' actors, in the kept lists' order.
    pub actors: Vec<Option<Actor>>,
    pub gims: Option<entry::DungeonGims>,
    pub rng: piney_data::dungeon::Rng,
    /// The dungeon's type and the built room's breakables' dummies
    /// ([`crate::dungeon_area::DungeonArea::breakables_here`]).
    pub dtype: u8,
    pub breakables: Vec<(u8, V4)>,
}

/// What a generated field hands the entry control's set-up for
/// `WORLD_MAN::EntryGimmick`'s field setters (`WORLD::SetFood`,
/// `SetMagicCircle`, `SetSpecialObj`, in that order), with the field's
/// `fieldrand` as `Generate` left it: the objects and in-points
/// ([`crate::field_area::FieldArea::field_gims`]), the map `SetMagicCircle`
/// reads, `circleOfs`, `eventAreaNumber` and `fieldStartPos`.
pub struct FieldEntries {
    pub gims: entry::FieldGims,
    pub map: std::rc::Rc<piney_data::field::Field>,
    pub rng: piney_data::dungeon::Rng,
    pub circle_ofs: i32,
    pub event_area: i32,
    pub start: V4,
}

/// `DUNGEON::EntryBreakObject`'s gimmick rows by dungeon type (its jump
/// table @4166 and `EntryBreakObjectMain`'s @4116, types 0-3 and again
/// 4-7): `OBJ_0pr4*` .. `OBJ_0pr7*` take them in turn, `OBJ_0pr2*` one of
/// the four at random (`fieldrand(4)`, drawn for every breakable).
const BREAK_ROWS: [[i32; 4]; 4] = [[8, 10, 7, 12], [8, 9, 7, 12], [10, 9, 11, 12], [13, 14, 11, 12]];

/// A box's draw transparency.
/// `ccCoord::SetMatrix_PosRotZYXScale(pos, rot, scale)` (main
/// 0x00138120): the scale, the turns (z, y, x), the position.
fn food_matrix(pos: V4, rot: V4, scale: V4) -> [V4; 4] {
    let m = crate::town::pos_rot_zyx(pos, [rot[0], rot[1], rot[2]]);
    let mut out = m;
    for (col, &s) in out.iter_mut().zip(scale.iter()).take(3) {
        for v in col.iter_mut().take(3) {
            *v = ee::mul(*v, s);
        }
    }
    out
}

fn transparency_of(o: &entry::Out) -> F {
    match *o {
        entry::Out::GimDraw { transparency, .. } => transparency,
        _ => ONE,
    }
}

/// No effects.
pub struct NoFx;

impl FxTasks for NoFx {}

/// An enemy's rule outputs that did not go through its frame (a
/// `selectTarget` run from outside it): its `ClearConditionEffect` ends its
/// condition effect at once (`deleteConditionEffect`, `conditionNum` -1).
fn enemy_cleared(
    fx: &mut std::collections::HashMap<usize, i32>,
    shows: &mut Vec<Show>,
    c: usize,
    out: &[enemy_ai::Out],
) {
    let cleared = out.iter().any(|o| matches!(o, enemy_ai::Out::ClearConditionEffect));
    if cleared && fx.remove(&c).is_some() {
        shows.push(Show::ConditionEffect { who: c, act: CondFx::Delete, num: -1 });
    }
}

/// The fights of one area.
pub struct Combat {
    pub data: Rc<BattleData>,
    pub scene: Scene,
    pub foes: Vec<Option<Enemy>>,
    pub crew: Crew,
    pub party: Party,
    pub keep: Keep,
    pub skills: RefCell<Skills>,
    pub ctrl: EntryCtrl,
    pub reg: Register,
    pub spc: SpcThread,
    pub player: kite::Player,
    /// newlib's `rand()`, `ccRand()` (MT19937), `ccRandS`'s state.
    pub rand: Rand,
    pub cc: Genrand,
    pub rnds: u16,
    pub kite: Option<usize>,
    /// The party's characters: (registry id, scene index), Kite first.
    pub members: Vec<(i32, usize)>,
    /// Each member's record as the last frame stored it in the save, by
    /// registry id.
    stored: Vec<(i32, SpcParam)>,
    pub cast: Cast,
    pub shows: Vec<Show>,
    /// The chat lines rules raised outside a member's frame (a hit's, an
    /// affect's), run by [`Combat::drain_chats`] where the frame has what
    /// they need ([`chat`]).
    pub chat_queue: Vec<Event>,
    /// How many of `shows` the effects have seen.
    pub fx_seen: usize,
    /// Each enemy's `ccEnemyDustCtrl`: the last frame bucket of each of
    /// its rows (the nodes' +8), by scene index.
    pub dust: std::collections::HashMap<usize, Vec<i16>>,
    /// The dust rings the controllers have raised (for tests).
    pub dust_rings: u64,
    /// For tests and shots: the next Data Drain side effect's two draws
    /// (`rand() & 15` into the infection's row of `dataDrainErosionTbl`,
    /// then the resistance roll `rand() % 1001`) instead of `rand`'s.
    pub force_side_draws: Option<[i32; 2]>,
    /// The members' `ccUseItemRequest` calls of the frame (their AI's
    /// `UseItem`, the ocarina command), for the runtime to carry out
    /// ([`crate::field_world::FieldWorld::take_member_items`]).
    pub member_items: Vec<MemberItem>,
    /// `ccGame`'s battle state (+0x58 `inBattle`, +0x5c `inBattleCnt`,
    /// +0x60 `inBattleDist`), which `ccThGameCtrl` keeps.
    pub battle: crate::talk::InBattle,
    /// `recoveryReq`: the element heals `ccThGameCtrl` hands out.
    pub recovery: crate::talk::RecoveryReqs<usize>,
    /// `WORLD_MAN::Enter` asked by Kite's frame.
    pub enter: bool,
    /// The entry control's set-up has run.
    pub started: bool,
    /// The magic portal's particle pattern count (`EFF_xmagpat1`).
    pub part_pats: u16,
    /// The event's `ccThEvHold` task (`eventMng.holdTscb`).
    pub ev_hold: Option<evparty::Hold>,
    /// The boxes' and idols' rays this frame (`ccPrimRadiate::disp`): each
    /// ray's four points in the world with their colours.
    pub rays: Vec<Vec<[piney_battle::prim::PrimVert; 4]>>,
    /// The enemies' weapon controllers ([`weapon`]) and their trails this
    /// frame.
    pub weapons: weapon::Weapons,
    /// The dogs', wyrms' and dragons' breaths ([`breath`]).
    pub breaths: breath::Breaths,
    pub trails: Vec<weapon::Trail>,
    /// Each character's condition effect (`ccChar` +0x2c) as the effects
    /// hold it: its own number (+0x1c), by scene index.
    cond_fx: std::collections::HashMap<usize, i32>,
    /// `ccSpcConditionEffectSW()` and the eye view, as the frame found them
    /// (`DispConditionEffect` reads both).
    cond_fx_shown: bool,
    eye_view: bool,
    /// How the party comes into the area, for the characters built next.
    pub entrance: Entrance,
    /// The boss's tasks (`ccBossEntryStart`), once an event made it.
    pub boss: Option<boss::BossRun>,
    /// What `ccRestoreSpcCondition` puts back into the characters built
    /// next, by `charTbl` row: the party's `storeCondition` when the scene
    /// restores it ([`crate::party::restores_condition`]), else none.
    pub restore: [Option<crate::party::StoredCondition>; 18],
    /// The event's NPCs `ccEntryEventMng` made (type 3 a town PC, type 4
    /// the Administrator): their stand-ins on the entry control's NPC list,
    /// which the world moves, draws and speaks for.
    pub npcs: Vec<EventNpc>,
    /// What the object menus ask of Kite from the menu task, played before
    /// his next frame (the menu task runs after the world's here).
    pub kite_menu: Vec<KiteMenuCall>,
    /// The symbols' omni lights in the group (`ccGimSymbol` +0x704), by
    /// scene index: place and intensity as their last frame left them.
    pub symbol_lights: std::collections::BTreeMap<usize, (V4, F)>,
    /// The spring's two omni lights in the group (`ccGimEtc` +0x260), by
    /// scene index, as its last frame left them.
    pub spring_lights: std::collections::BTreeMap<usize, [piney_battle::prim::OmniLight; 2]>,
    /// The rays' omni lights in the group (`ccPrimRadiate` +0x58: the
    /// boxes', the idols', the radiator's), by scene index, as their last
    /// `setLight` left them.
    pub rad_lights: std::collections::BTreeMap<usize, piney_battle::prim::OmniLight>,
    /// The symbols' fires drawn by the last frame (`ccSymFire::main`'s
    /// `ccEff::Draw`): place, pattern, scale, colour.
    pub symbol_fires: Vec<(V4, u16, [F; 2], u32)>,
    /// The riding Grunty ([`ride`]).
    pub ride: ride::Riding,
}

/// `plw`'s calls from the object menus (`ItemObjMenu`, `TrapObjMenu`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KiteMenuCall {
    /// `ccPlayer::BreakSomething(0)`: act 25, held.
    Break,
    /// `ccPlayer::AttackCancel()`.
    AttackCancel,
}

/// How the party comes into the area, as `ccPlayer::ccPlayer`,
/// `ccFellow::Initialize` and `ccAI::ccAI` read `game` and `gtHackFlag`
/// ([`gate_out`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entrance {
    /// `ccAI::ccAI`'s `arrivalChatCnt` (+0x80): 150 in a town or arriving
    /// in a field from one, -1 after a gate hack or otherwise.
    pub chat: i16,
    /// The gate-hacked arrival: Kite in act 24 (`ghoFlag`), the members
    /// hidden for 65 frames standing, no transfer in.
    pub hacked: bool,
    /// Whether the party members come in through the gate (act 13, the
    /// transfer's effect and sound) or stand where they are put (act 2).
    pub member_transfer: bool,
}

impl Default for Entrance {
    fn default() -> Self {
        Entrance { chat: -1, hacked: false, member_transfer: true }
    }
}

impl Entrance {
    /// `ccFellow::Initialize`'s (gcmn 0x0041ae80) choice for a member: in a
    /// town, or in a field (or an event area of no dungeon) come to from a
    /// town (`areaPrev` 0), through the gate (act 13) after a lag of
    /// `(rand() % 4) * 5` frames; in a dungeon only with `WORLD_MAN.warpFlag`
    /// (+0x168; the dungeons' warps are not ported, so never), without the
    /// lag; otherwise standing (act 2), the body on the collision list:
    /// into a dungeon, from room to room, back to a field.
    pub fn member_transfer(area: i32, area_prev: i32, field_type: i32, dungeon: i32, warp_flag: bool) -> bool {
        let field_rule = area_prev == 0;
        if field_type == 4 && dungeon == 0 {
            return field_rule;
        }
        match area {
            0 => true,
            1 => field_rule,
            2 => warp_flag,
            _ => false,
        }
    }
}

/// A member's `ccUseItemRequest(me, target, code, arg)` (gcmn 0x0057aa80)
/// from its AI: `me` uses item `code` on `target`. The AI has taken the
/// item from the member's list already.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemberItem {
    pub user: usize,
    pub target: usize,
    pub code: i32,
    pub arg: i32,
}

/// A [`Runtime`] handing each call to the world in the cell.
struct Own<'c, 'w, W: ?Sized>(&'c RefCell<&'w mut W>);

impl<W: Runtime + ?Sized> Runtime for Own<'_, '_, W> {
    fn call(&mut self, call: party_ai::Call, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) -> i32 {
        self.0.borrow_mut().call(call, scene, crew, rng)
    }
    fn call_ctx(&mut self, call: party_ai::Call, p: Parts<'_>) -> i32 {
        self.0.borrow_mut().call_ctx(call, p)
    }
    fn w2p(&mut self, pos: V4) -> V4 {
        Runtime::w2p(&mut **self.0.borrow_mut(), pos)
    }
}

/// The objects' own code the entry control calls that is not an enemy's
/// or a portal's: a gimmick or an NPC made with its table row's base and
/// doing nothing here (an event's NPCs are the world's to move and draw:
/// [`Combat::npcs`]).
struct Objects;

impl entry::Seam for Objects {
    fn enemy_main(&mut self, _: &mut EntryCtrl, _: &mut entry::Cx, _: usize) -> bool {
        false
    }
    fn gimmick_main(&mut self, _: &mut EntryCtrl, _: &mut entry::Cx, _: usize) -> bool {
        false
    }
    fn npc_main(&mut self, _: &mut EntryCtrl, _: &mut entry::Cx, _: usize) -> bool {
        false
    }
    fn make_enemy(&mut self, cx: &mut entry::Cx, _race: i32, ent: &EntryParam, who: usize) -> (BChar, Enemy) {
        piney_battle::races::init_enemy(cx, ent, who)
    }
    fn make_gimmick(&mut self, cx: &mut entry::Cx, ent: &EntryParam, _who: usize) -> (BChar, EntryObj) {
        let mut ch = entry::gimmick_char(cx.st, ent.id);
        ch.pos = ent.pos;
        (ch, EntryObj { ent: *ent, dirc: ent.dirc, gim_id: ent.id, ..EntryObj::default() })
    }
    fn make_npc(&mut self, cx: &mut entry::Cx, ent: &EntryParam, _who: usize) -> (BChar, EntryObj) {
        let mut ch = entry::npc_char(cx.st, ent.id);
        ch.pos = ent.pos;
        (ch, EntryObj { ent: *ent, dirc: ent.dirc, ..EntryObj::default() })
    }
}

/// An event's NPC in a field or dungeon: its stand-in's scene index, the
/// entry's type (3 a town PC, 4 a merchant: the Administrator) and its
/// `npcTbl` row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventNpc {
    pub who: usize,
    pub ty: i16,
    pub code: i16,
}

/// `ccEntryEventMng`'s `entry 3|4 CODE MARKER` outside the towns:
/// `ccSetRtownPC(code)` (gcmn 0x00506640) or `ccSetMerchant(code)`
/// (0x005057f0), an `entryObject` of type 2 (a PC with no landmark), put at
/// the marker's `evPos` facing its heading (Kite's position added for floor
/// and block 9999). A PC comes off the command list for good. The stand-in's
/// body stays out of the hit list: the world's character collides.
fn event_npc(
    ctrl: &mut EntryCtrl,
    cx: &mut entry::Cx,
    seam: &mut dyn entry::Seam,
    e: [i16; 4],
    positions: &[entry::EvPos],
    kite: V4,
    game: &entry::Game,
) -> Option<EventNpc> {
    let mut ep = entry::entry_param_clear();
    ep.ty = 2;
    ep.id = i32::from(e[1]);
    ep.area = game.area;
    ep.area_num = match game.area {
        0 => game.town,
        1 => game.field,
        _ => game.dungeon,
    };
    ep.floor = game.floor;
    ep.block = game.block;
    ep.ent_root = -1;
    if e[0] == 3 {
        ep.param[0] = -1;
    }
    let who = ctrl.entry_object(cx, seam, &mut ep)?;
    let (pos, dirc) = if e[2] < 0 {
        (ee::VF0, ee::VF0)
    } else {
        let p = positions
            .iter()
            .take(16)
            .find(|p| p.num == i32::from(e[2]))
            .or_else(|| positions.get(15))
            .copied()
            .unwrap_or_default();
        let mut pos = p.pos;
        if p.floor >= 9999 && p.block >= 9999 {
            pos = ee::vadd(pos, kite);
        }
        (pos, [0, 0, p.dirc, ONE])
    };
    cx.scene.chars[who].pos = pos;
    if let Some(entry::Obj::Npc(o)) = ctrl.objs.get_mut(who) {
        o.dirc = dirc;
        entry::set_hit_sw(cx.world, who, &mut o.hit, false);
        if e[0] == 3 {
            o.cmnd_flag = true;
        }
    }
    if e[0] == 3 {
        piney_battle::fellow::delete_cmnd(cx.scene, who);
    }
    Some(EventNpc { who, ty: e[0], code: e[1] })
}

/// `+0xe0` bits.
mod bit {
    pub const DISP: u32 = 1 << 1;
}

/// Kite's `bodyHit` height (95), a party member's `bodyHit` kind (6).
const PLAYER_BODY_HEIGHT: F = 0x42be_0000;
const MEMBER_BODY_KIND: u32 = 6;
const WALL_MASK: u32 = 0x4000_0001;

impl Combat {
    /// An area's fights, before the party is built.
    pub fn new(data: Rc<BattleData>, seed: u64) -> Combat {
        Combat {
            data,
            scene: Scene::default(),
            foes: Vec::new(),
            crew: Crew::default(),
            party: Party::default(),
            keep: Keep::default(),
            // `xnote` is a common file (`cmn/XNOTE.CCS`), resident in
            // every area.
            skills: RefCell::new(Skills { xnote: true, ..Skills::default() }),
            ctrl: EntryCtrl::default(),
            reg: Register::default(),
            spc: SpcThread::default(),
            player: kite::Player::default(),
            rand: Rand::new(seed),
            cc: Genrand::default(),
            rnds: 0,
            kite: None,
            members: Vec::new(),
            stored: Vec::new(),
            cast: Cast::default(),
            shows: Vec::new(),
            chat_queue: Vec::new(),
            fx_seen: 0,
            dust: std::collections::HashMap::new(),
            dust_rings: 0,
            force_side_draws: None,
            member_items: Vec::new(),
            battle: crate::talk::InBattle::default(),
            recovery: crate::talk::RecoveryReqs::default(),
            enter: false,
            started: false,
            part_pats: 0,
            ev_hold: None,
            rays: Vec::new(),
            weapons: weapon::Weapons::default(),
            breaths: breath::Breaths::default(),
            trails: Vec::new(),
            cond_fx: std::collections::HashMap::new(),
            cond_fx_shown: true,
            eye_view: false,
            entrance: Entrance::default(),
            boss: None,
            restore: [None; 18],
            npcs: Vec::new(),
            symbol_lights: Default::default(),
            spring_lights: Default::default(),
            rad_lights: Default::default(),
            symbol_fires: Vec::new(),
            ride: ride::Riding::default(),
            kite_menu: Vec::new(),
        }
    }

    /// `ccEvent::MenuBan`'s condition part after its `ccStoreSpcCondition`
    /// (main 0x001b2580): each built character's conditions, buffs and
    /// debuffs cleared (`ccChar::ClearCondition(ccSpcParam *)`).
    /// `ccClearConditionAllEnemy()` (gcmn 0x0042e4f0): `clearConditionEnemy`
    /// on each enemy of the list with `objFlag` set and `freezeFlag` clear,
    /// its conditions cleared and its condition effect ended
    /// (`ClearConditionEffect`), as the Grunty Flute and the fountain do.
    pub fn clear_condition_all_enemy(&mut self) {
        let who: Vec<usize> = self.ctrl.active_enemies(&self.foes).collect();
        for c in who {
            let mut out = Vec::new();
            enemy_ai::clear_condition_enemy(&mut self.scene.chars[c], &mut out);
            enemy_cleared(&mut self.cond_fx, &mut self.shows, c, &out);
        }
    }

    pub fn menu_ban_conditions(&mut self) {
        for &(_, who) in &self.members {
            affect::clear_condition(&mut self.scene.chars[who]);
        }
    }

    /// The party's records as the save has them: the game's ccSpcParam is
    /// the save's own record, so what the menus (Equipment, a book read, an
    /// item given) and the scripts wrote between frames holds. A record the
    /// save kept as the last frame stored it stays as the world left it
    /// since (a boss's ccClearSpcCondition). The frame writes them back at
    /// its end ([`Combat::store_records`]).
    pub fn load_records(&mut self, save: &piney_data::save::SaveData) {
        for &(id, m) in &self.members {
            if let Some(p) = self.scene.chars.get_mut(m).and_then(|c| c.spc_mut()) {
                let saved = SpcParam::from_save(save, id as usize);
                if !self.stored.iter().any(|(i, s)| *i == id && *s == saved) {
                    *p = saved;
                }
            }
        }
    }

    /// The party's records into the save, which is where the game keeps
    /// them (`ccChar` +4 points at `saveData.spcParam[id]`): at each
    /// frame's end, and after anything that changes them between frames
    /// (`ClearCondition(ccSpcParam *)`'s stat changes cleared).
    pub fn store_records(&mut self, save: &mut piney_data::save::SaveData) {
        self.stored.clear();
        for &(id, m) in &self.members {
            if let Some(p) = self.scene.chars[m].spc() {
                p.store(save, id as usize);
                self.stored.push((id, *p));
            }
        }
    }

    /// `ccEvent::MenuClr`'s condition part (main 0x001b265c): each built
    /// character's stored condition back (`ccRestoreSpcCondition`, when
    /// the scene restores: `restore`), then `ccSpcChar::ConditionAdjustment`
    /// (a down character a ghost, a revived one up), its events carried out
    /// as the battle's frame does.
    pub fn menu_clear_conditions(
        &mut self,
        x: &mut Tasks,
        restore: bool,
        store: &[Option<crate::party::StoredCondition>; 18],
    ) {
        let bounds = Self::bounds(x.scene.area, x.hits);
        let mut ev = Vec::new();
        for &(id, who) in &self.members {
            let ch = &mut self.scene.chars[who];
            if restore && let Some(st) = usize::try_from(id).ok().and_then(|i| store.get(i)).copied().flatten() {
                st.restore(ch);
            }
            affect::condition_adjustment(ch, piney_battle::event::Who::Char(who), &mut ev);
        }
        for e in ev {
            self.consequence(e, x.hits, bounds, x.puppet_show);
        }
    }

    /// `ccRestoreSpcCondition(ch)` as `ccPlayer::ccPlayer` (gcmn 0x00597a08)
    /// and `ccFellow::Initialize` (0x0041af2c) call it right after
    /// `SetBaseParam`, for a character in the party (`partyFlag` 1); then
    /// one with no HP stands as a ghost: SP 0, `dead` 4, `ghostFlag`, cloak
    /// 0.5.
    fn restore_condition(&self, id: i32, ch: &mut BChar) {
        if ch.party_flag != 1 {
            return;
        }
        let Some(st) = usize::try_from(id).ok().and_then(|i| self.restore.get(i)).copied().flatten() else { return };
        st.restore(ch);
        if ch.hp == 0 {
            ch.sp = 0;
            ch.cond.v[cond::DEAD] = 4;
            ch.spc_char.flags |= spc_flag::GHOST;
            ch.spc_char.cloak = 0x3f00_0000;
        }
    }

    /// `WORLD_MAN`'s bounds for the area (+0x420): a field's 0..48000, a
    /// dungeon's 0..60000 (`GO(2)`), a town's -24000..24000.
    pub fn bounds(area: i32, hits: &Hits) -> MapBounds {
        match (hits.bounds, area) {
            (Some([a, b, c, d]), _) => MapBounds { min: [a, b], max: [c, d] },
            (None, 2) => MapBounds { min: [0, 0], max: [0x476a_6000, 0x476a_6000] },
            _ => MapBounds { min: [0xc6bb_8000, 0xc6bb_8000], max: [0x46bb_8000, 0x46bb_8000] },
        }
    }

    fn w2p(&self, bounds: &MapBounds, pos: V4) -> V4 {
        let player = self.kite.map_or(ee::VF0, |k| self.scene.chars[k].pos);
        kite::w2p_pos(bounds, player, pos).0
    }

    /// `ccPlayer::ccPlayer(0)` and `ccSPC::Reboot`'s AI for Kite: his
    /// record (`saveData.spcParam[0]`), at `pos` facing `rot`, arriving
    /// (act 13, `transferLag` 1) from a town or standing (act 2, the body
    /// in the collision) in or back from a dungeon; `SetBootStatus(boot)`
    /// before the AI exists, `restraintSW` unless standing, then the AI in
    /// mode 1, manual when `boot` has bit 2. `body` is his model.
    #[allow(clippy::too_many_arguments)]
    pub fn add_kite(
        &mut self,
        save: &SaveData,
        pos: V4,
        rot: F,
        arriving: bool,
        boot: i32,
        party_flag: i8,
        body: Rc<Body>,
        swaps: Vec<(u32, u32)>,
        hits: &mut Hits,
        area: i32,
    ) -> usize {
        let p = SpcParam::from_save(save, 0);
        let (width, height, velocity) = (p.base.width, p.base.height, p.velocity);
        // ccSpcChar::ccSpcChar (gcmn 0x0059d230): cycle = rand() >> 3.
        let cycle = self.rand.rand() >> 3;
        let mut ch = BChar::pc(p);
        ch.affect.func = AffectFunc::Player;
        ch.has_ai = true;
        ch.party_flag = i32::from(party_flag) & 7;
        ch.pos = pos;
        ch.pos_p = [0, 0, pos[2], ONE];
        ch.spc_char.cloak = ONE;
        ch.spc_char.flags = bit::DISP | spc_flag::STOP;
        self.restore_condition(0, &mut ch);
        // A gate-hacked arrival (ccPlayer::ccPlayer with ccCheckGtHack):
        // ghoFlag, act 24, the body on the list at once.
        let hacked = arriving && self.entrance.hacked;
        let act = if hacked {
            kite::act::HACK
        } else if arriving {
            kite::act::ARRIVE
        } else {
            kite::act::IDLE
        };
        ch.spc_char.act_num = act;
        ch.spc_char.act_num_old = if arriving { 0 } else { act };
        let listed = boot & 4 == 0;
        let who = self.scene.add(ch, if listed { 0 } else { 3 });
        let dirc = [0, 0, rot, 0];
        let mut hit =
            CharHit { pos, radius: width, height: PLAYER_BODY_HEIGHT, kind: 7, mask2: WALL_MASK, ..CharHit::default() };
        if !arriving || hacked {
            let mut b = spc::to_body(who, Some(who), &hit, false);
            hits.hit_enable(&mut b);
            hit.sw = true;
        }
        self.cast.gate.state.flag = hacked;
        let s = self.crew.spc.entry(who).or_default();
        s.dirc = dirc;
        s.speed = velocity;
        s.speed_rate = ONE;
        s.transfer_lag = i16::from(arriving && !hacked);
        s.transparency = ONE;
        s.set_transparency = ONE;
        s.body_hit = hit;
        s.act_num = act;
        s.stop_flag = true;
        s.cycle = cycle;
        self.kite = Some(who);
        self.player = kite::Player::default();
        // ccPlayer::ccPlayer (MUT gcmn 0x005c2d8c): on a moving floor
        // (WORLD_MAN::GetTransMode), his place from its centre.
        if let Some(c) = hits.trans {
            let mut d = piney_battle::geom::vsub(pos, c);
            d[3] = ONE;
            self.player.disk_offset = d;
        }
        // SetBootStatus before the AI exists, restraintSW unless standing.
        let mut r = spc::SpcRec::read(&self.scene, &self.crew, who, self.kite, true);
        r.spc_ref().set_boot_status(boot, hits);
        if r.act != 2 {
            r.restraint = true;
        }
        r.write(&mut self.scene, &mut self.crew);
        let chat = if area == 0 { 150 } else { self.entrance.chat };
        let strategy = self.spc.party_strategy as i16;
        let mut ai = BAi::new(who, 0, strategy, chat, &mut self.rand);
        ai.level_old = self.scene.chars[who].level();
        self.crew.ais.insert(who, ai);
        let mut r = spc::SpcRec::read(&self.scene, &self.crew, who, self.kite, true);
        if let Some(a) = r.ai.as_mut() {
            a.change_mode(1);
            if boot & 4 != 0 {
                a.manual_mode(&dirc);
            }
        }
        r.write(&mut self.scene, &mut self.crew);
        kite::to_spc(&self.scene, &mut self.crew, who);
        let clip = self.data.kt.anim(self.scene.chars[who].spc_char.act_num).to_string();
        if let Some(mut a) = Actor::new(body, &clip, Look::Kite, pos, dirc, height, width) {
            a.swaps = swaps;
            self.cast.actors.insert(who, a);
        }
        self.members.retain(|m| m.0 != 0);
        self.members.insert(0, (0, who));
        who
    }

    /// `ccFellow::Initialize` (gcmn 0x0041ae80) and `ccSPC::Reboot`'s AI for
    /// registered character `id` (a `charTbl` row, 1-17) in registry slot
    /// `list_num`: its record, at `pos` facing `rot`, arriving (act 13) after `(rand() % 4) * 5`
    /// frames, on the command list unless `boot` has bit 2, then
    /// `SetBootStatus(boot)`, `restraintSW` unless standing, the AI (mode
    /// 1, manual with bit 2).
    #[allow(clippy::too_many_arguments)]
    pub fn add_member(
        &mut self,
        save: &SaveData,
        id: i32,
        pos: V4,
        rot: F,
        boot: i32,
        party_flag: i8,
        body: Rc<Body>,
        hits: &mut Hits,
        area: i32,
        list_num: i32,
    ) -> usize {
        let p = SpcParam::from_save(save, id as usize);
        let (width, height, velocity) = (p.base.width, p.base.height, p.velocity);
        // ccSpcChar::ccSpcChar (gcmn 0x0059d230): cycle = rand() >> 3; then
        // ccFellow::Initialize (0x0041ae80): atkDellay = (rand() >> 3) & 31
        // before the arrival's transferLag.
        let cycle = self.rand.rand() >> 3;
        let atk_dellay = (self.rand.rand() >> 3) & 31;
        let mut ch = BChar::pc(p);
        ch.affect.func = AffectFunc::Fellow;
        ch.has_ai = true;
        ch.party_flag = i32::from(party_flag) & 7;
        ch.pos = pos;
        ch.pos_p = self.w2p(&Self::bounds(area, hits), pos);
        ch.spc_char.cloak = ONE;
        ch.spc_char.flags = bit::DISP | spc_flag::STOP;
        self.restore_condition(id, &mut ch);
        // A gate-hacked arrival (ccFellow::Initialize with ccCheckGtHack):
        // hidden, dispWait 65, standing, the body on the list; no
        // transferLag draw.
        let hacked = self.entrance.hacked;
        let transfer = !hacked && self.entrance.member_transfer;
        let act = if transfer { fellow::act::TRANSFER_IN } else { fellow::act::EASE };
        if hacked {
            ch.spc_char.flags &= !bit::DISP;
        }
        ch.spc_char.act_num = act;
        ch.spc_char.act_num_old = 0;
        // ccEntryCmnd only when shown (a hacked arrival's waits for its
        // dispWait).
        let listed = boot & 4 == 0 && !hacked;
        let who = self.scene.add(ch, if listed { 0 } else { 3 });
        let dirc = [0, 0, rot, 0];
        // The transfer's lag is drawn in a town and in a field, not for a
        // dungeon's warp.
        let lag = if transfer && area != crate::area::kind::DUNGEON { ((self.rand.rand() % 4) * 5) as i16 } else { 0 };
        let s = self.crew.spc.entry(who).or_default();
        s.dirc = dirc;
        s.speed = velocity;
        s.speed_rate = ONE;
        s.transfer_lag = lag;
        s.cycle = cycle;
        s.atk_dellay = atk_dellay;
        s.transparency = ONE;
        s.set_transparency = ONE;
        s.motion = id as i16;
        // ccFellowNN(n) (gcmn 0x0041ecc0): SpcListNum, its registry slot.
        s.spc_list_num = list_num;
        s.act_num = act;
        s.stop_flag = true;
        s.disp_wait = if hacked { 65 } else { 0 };
        // ccFellow::Initialize (MUT gcmn 0x0042ebdc): on a moving floor,
        // its place from the floor's centre.
        if let Some(c) = hits.trans {
            let mut d = piney_battle::geom::vsub(pos, c);
            d[3] = ONE;
            s.disk_offset = d;
        }
        s.body_hit = CharHit {
            pos,
            radius: width,
            height: ee::div(height, 0x4000_0000),
            kind: MEMBER_BODY_KIND,
            mask2: WALL_MASK,
            ..CharHit::default()
        };
        // Standing (hacked or not): the body on the collision list at once
        // unless dead.
        if !transfer && self.scene.chars[who].cond[cond::DEAD] == 0 {
            let s = self.crew.spc.entry(who).or_default();
            let mut h = s.body_hit;
            let mut b = spc::to_body(who, self.kite, &h, false);
            hits.hit_enable(&mut b);
            h.sw = true;
            self.crew.spc.entry(who).or_default().body_hit = h;
        }
        let mut r = spc::SpcRec::read(&self.scene, &self.crew, who, self.kite, true);
        r.spc_ref().set_boot_status(boot, hits);
        if r.act != 2 {
            r.restraint = true;
        }
        r.write(&mut self.scene, &mut self.crew);
        let chat = if area == 0 { 150 } else { self.entrance.chat };
        let strategy = self.spc.party_strategy as i16;
        let mut ai = BAi::new(who, id as usize, strategy, chat, &mut self.rand);
        ai.level_old = self.scene.chars[who].level();
        self.crew.ais.insert(who, ai);
        let mut r = spc::SpcRec::read(&self.scene, &self.crew, who, self.kite, true);
        if let Some(a) = r.ai.as_mut() {
            a.follow_sw = true;
            a.change_mode(1);
            if boot & 4 != 0 {
                a.manual_mode(&dirc);
            }
        }
        r.write(&mut self.scene, &mut self.crew);
        fellow::sync_out(&mut self.scene, &self.crew, who);
        let clip = self.data.mt.clip(id as i16, self.scene.chars[who].spc_char.act_num).to_string();
        if let Some(a) = Actor::new(body, &clip, Look::Member(id), pos, dirc, height, width) {
            self.cast.actors.insert(who, a);
        }
        self.members.push((id, who));
        who
    }

    /// `ccPartyManager` after `SetParty`: the members by slot.
    pub fn set_party(&mut self, ids: [i32; 3]) {
        let mut p = Party { ids, ..Party::default() };
        for (k, &id) in ids.iter().enumerate() {
            if id >= 0 {
                p.members[k] = self.members.iter().find(|m| m.0 == id).map(|m| m.1);
            }
        }
        p.num = p.members.iter().filter(|m| m.is_some()).count() as i32;
        self.party = p;
    }

    /// The scene index of registered character `id`.
    pub fn who(&self, id: i32) -> Option<usize> {
        self.members.iter().find(|m| m.0 == id).map(|m| m.1)
    }

    /// `ccThEntryCtrl`'s set-up in a field or dungeon: the control for the
    /// place, `ccRegisterDifficultyEnemy`'s rows (main 0x001b7020), then
    /// the event's entries (`ccEntryEventMng`, main 0x001b62e0): each
    /// `entry_mc` a magic portal at its marker, each `entry` of type 5 or 6
    /// its enemies.
    #[allow(clippy::too_many_arguments)]
    pub fn start_entries(
        &mut self,
        x: &mut Tasks,
        rank: i32,
        ty: i32,
        entries_mc: &[[i16; 4]],
        entries: &[[i16; 4]],
        positions: &[entry::EvPos],
        dungeon: Option<DungeonEntries>,
        field: Option<FieldEntries>,
    ) -> Option<piney_data::dungeon::Rng> {
        if self.started {
            return None;
        }
        self.started = true;
        let d = self.data.clone();
        let game = self.entry_game(&x.scene, &x.wm);
        self.ctrl = EntryCtrl::new(&game);
        self.reg.init();
        let server = x.scene.server.clamp(0, 4);
        self.reg.register_list(&d.t, &d.st, server, ty.clamp(0, 6), rank.max(0));
        self.spc.start(x.save, self.battle.in_battle);
        let kite_pos = self.kite.map_or(ee::VF0, |k| self.scene.chars[k].pos);
        let bounds = Self::bounds(x.scene.area, x.hits);
        let mut outs = Vec::new();
        let mut stage = Stage {
            t: &d.t,
            hits: x.hits,
            camera: x.camera,
            pad: x.pad,
            cast: &mut self.cast,
            skills: &self.skills,
            area: x.scene.area,
            bounds,
            player: kite_pos,
            kite: self.kite,
            map2d: Vec::new(),
            map2d_info: [0; 3],
            event_area: x.event_area,
            annihilated: false,
            shows: &mut self.shows,
            enter: false,
            view: kite_pos,
            retarget: Vec::new(),
            kite_act: 0,
            traps: Vec::new(),
            town: None,
            spcs: None,
            tricks: chat::condition_skills(&self.data.t, &self.foes, self.scene.chars.len()),
            chats: Vec::new(),
            manual: self.crew.manual_chars(),
            item_uses: &mut self.member_items,
        };
        let mut cx = entry::Cx {
            t: &d.t,
            st: &d.st,
            scene: &mut self.scene,
            foes: &mut self.foes,
            world: &mut stage,
            cc: &mut self.cc,
            rnds: &mut self.rnds,
            save: x.save,
            game,
            reg: &self.reg,
            part_pats: self.part_pats,
            out: &mut outs,
        };
        let mut seam = Objects;
        let mut restored: Vec<(usize, Option<Actor>)> = Vec::new();
        // restoreEntry, then WORLD_MAN::EntryGimmick in a dungeon: the
        // boxes, portals and idols once per dungeon (entryFlag), the
        // breakables each time (EntryBreakObjectMain is not ported; its
        // guard leaves event 4's dungeon without them).
        if let Some(d) = dungeon {
            if let Some(kept) = d.kept {
                let made = self.ctrl.restore_kept(&mut cx, kept);
                restored = made.into_iter().zip(d.actors).collect();
            }
            let mut rng = d.rng;
            if let Some(mut g) = d.gims {
                piney_battle::gimmick::set_item_box(&mut self.ctrl, &mut cx, &mut seam, &mut g, &mut |n| rng.below(n));
                if entry::dungeon_places_portals(cx.st.volume, game.field) {
                    entry::dungeon_set_magic_circle(&mut self.ctrl, &mut cx, &mut seam, &mut g);
                }
                piney_battle::gimmick::set_idol(&mut self.ctrl, &mut cx, &mut seam, &mut g);
            }
            // DUNGEON::EntryBreakObject, every time: the room's breakables,
            // gimmicks of entRoot 2 (deleted as the room is left).
            if d.dtype < 8 {
                let rows = BREAK_ROWS[usize::from(d.dtype % 4)];
                for (family, pos) in d.breakables {
                    let pick = rows[rng.below(4) as usize];
                    let mut ep = entry::entry_param_clear();
                    ep.pos = pos;
                    ep.ty = 1;
                    ep.id = if family == 2 { pick } else { rows[usize::from(family - 4)] };
                    ep.area = 2;
                    ep.area_num = game.dungeon;
                    ep.floor = game.floor;
                    ep.block = game.block;
                    ep.ent_root = 2;
                    ep.land = 0;
                    self.ctrl.entry_object(&mut cx, &mut seam, &mut ep);
                }
            }
        }
        // WORLD_MAN::EntryGimmick in a field: its foods (and a lake's
        // spring), its magic portals and enemies, the dungeon entrance's
        // swirls and the symbols, every time the field is entered.
        let mut field_rng = None;
        if let Some(mut f) = field {
            entry::world_set_food(&mut self.ctrl, &mut cx, &mut seam, &f.gims, &mut f.rng);
            entry::world_set_magic_circle(
                &mut self.ctrl,
                &mut cx,
                &mut seam,
                &*f.map,
                &mut f.rng,
                f.circle_ofs,
                f.event_area,
                f.start,
            );
            entry::world_set_special_obj(&mut self.ctrl, &mut cx, &mut seam, &f.gims, &mut f.rng);
            field_rng = Some(f.rng);
        }
        for e in entries_mc.iter().filter(|e| e[0] >= 0) {
            if let Some(mut ep) = entry::event_magic_circle(*e, positions, kite_pos, &game) {
                self.ctrl.entry_object(&mut cx, &mut seam, &mut ep);
            }
        }
        for e in entries.iter().filter(|e| matches!(e[0], 5 | 6)) {
            let (mut ep, n) = entry::event_enemy(*e, positions, kite_pos, &game);
            for _ in 0..n {
                self.ctrl.entry_object_n(&mut cx, &mut seam, &mut ep, 1);
            }
        }
        for e in entries.iter().filter(|e| matches!(e[0], 3 | 4)) {
            if let Some(n) = event_npc(&mut self.ctrl, &mut cx, &mut seam, *e, positions, kite_pos, &game) {
                self.npcs.push(n);
            }
        }
        drop(stage);
        // The kept objects' bodies and animation players, as they left.
        for (who, a) in restored {
            if let Some(a) = a {
                self.cast.actors.insert(who, a);
            }
        }
        // The controllers the constructors made, now: the entries' shows
        // can go to presentation before the frame's passes see them.
        for o in &outs {
            match *o {
                entry::Out::Weapon { who, info, n } => self.weapon_made(who, info, n),
                entry::Out::Dust { who, n, .. } => {
                    self.dust.insert(who, vec![0; usize::try_from(n).unwrap_or(0)]);
                }
                entry::Out::Breath { who, slot, info } => self.breath_made(who, slot, info),
                _ => {}
            }
        }
        for o in outs {
            self.shows.push(Show::Entry(o));
        }
        field_rng
    }

    /// `EntryObject(ep)` outside the entry control's frame, the object's
    /// own code the ported classes' ([`Objects`]): a treasure box, an idol
    /// or a portal put where no table put one (tests and tools). Its scene
    /// index.
    pub fn entry_object(&mut self, x: &mut Tasks, ep: &mut EntryParam) -> Option<usize> {
        self.with_entry_cx(x, |ctrl, cx| ctrl.entry_object(cx, &mut Objects, ep))
    }

    /// `entryObject(ep, n)` outside the entry control's frame (an event's
    /// `radiator`): `n` objects of `ep`, the first's scene index.
    pub fn entry_object_n(&mut self, x: &mut Tasks, ep: &mut EntryParam, n: i32) -> Option<usize> {
        self.with_entry_cx(x, |ctrl, cx| ctrl.entry_object_n(cx, &mut Objects, ep, n))
    }

    /// `EntryObject(ep)` of gimmick row `id` at `pos` facing `dirc` in the
    /// area, floor and room the entry control is in (`entRoot` -1), outside
    /// its frame (tests and tools): its scene index.
    pub fn entry_gimmick(&mut self, x: &mut Tasks, id: i32, pos: V4, dirc: V4) -> Option<usize> {
        self.with_entry_cx(x, |ctrl, cx| {
            let g = cx.game;
            let mut ep = entry::entry_param_clear();
            ep.pos = pos;
            ep.dirc = dirc;
            ep.ty = 1;
            ep.id = id;
            ep.area = g.area;
            ep.area_num = match g.area {
                0 => g.town,
                1 => g.field,
                _ => g.dungeon,
            };
            ep.floor = g.floor;
            ep.block = g.block;
            ep.ent_root = -1;
            ctrl.entry_object(cx, &mut Objects, &mut ep)
        })
    }

    /// The entry control with its context over the area, outside its
    /// frame (an event's `remove`): `f` on the control and a [`entry::Cx`]
    /// whose world is the field; its outputs shown.
    pub fn with_entry_cx<R>(&mut self, x: &mut Tasks, f: impl FnOnce(&mut EntryCtrl, &mut entry::Cx) -> R) -> R {
        let d = self.data.clone();
        let game = self.entry_game(&x.scene, &x.wm);
        let kite_pos = self.kite.map_or(ee::VF0, |k| self.scene.chars[k].pos);
        let bounds = Self::bounds(x.scene.area, x.hits);
        let annihilated = self.party.annihilated(&self.scene);
        let mut outs = Vec::new();
        let mut stage = Stage {
            t: &d.t,
            hits: x.hits,
            camera: x.camera,
            pad: x.pad,
            cast: &mut self.cast,
            skills: &self.skills,
            area: x.scene.area,
            bounds,
            player: kite_pos,
            kite: self.kite,
            map2d: Vec::new(),
            map2d_info: [0; 3],
            event_area: x.event_area,
            annihilated,
            shows: &mut self.shows,
            enter: false,
            view: kite_pos,
            retarget: Vec::new(),
            kite_act: 0,
            traps: Vec::new(),
            town: None,
            spcs: None,
            tricks: chat::condition_skills(&self.data.t, &self.foes, self.scene.chars.len()),
            chats: Vec::new(),
            manual: self.crew.manual_chars(),
            item_uses: &mut self.member_items,
        };
        let mut cx = entry::Cx {
            t: &d.t,
            st: &d.st,
            scene: &mut self.scene,
            foes: &mut self.foes,
            world: &mut stage,
            cc: &mut self.cc,
            rnds: &mut self.rnds,
            save: x.save,
            game,
            reg: &self.reg,
            part_pats: self.part_pats,
            out: &mut outs,
        };
        let r = f(&mut self.ctrl, &mut cx);
        drop(stage);
        for o in outs {
            if let entry::Out::Destroyed { who, .. } = o {
                self.cast.actors.remove(&who);
            }
            self.shows.push(Show::Entry(o));
        }
        r
    }

    fn entry_game(&self, s: &crate::area::Scene, wm: &crate::area::WorldMan) -> entry::Game {
        entry::Game {
            area: s.area,
            town: s.town,
            field: s.field,
            dungeon: s.dungeon,
            floor: s.floor,
            block: s.block,
            server: s.server,
            field_type: wm.field_type as i32,
            area_code: wm.words[0] * 1_000_000 + wm.words[1] * 1000 + wm.words[2],
            area_level: wm.area_level,
            player: self.kite,
        }
    }

    /// `ccSkillCheck` of every character, as the affects of a task ask it.
    fn skill_checks(&self) -> Vec<i32> {
        let act = self.kite.map_or(0, |k| self.scene.chars[k].spc_char.act_num);
        let s = self.skills.borrow();
        (0..self.scene.chars.len()).map(|c| s.check(&self.data.t, &self.scene, c, act)).collect()
    }

    /// The queued chat lines ([`Combat::chat_queue`]) run on their members'
    /// AIs, in order. None of them asks the runtime anything (a hit's or
    /// an affect's line only picks, speaks and posts to the bus).
    pub fn drain_chats(
        &mut self,
        t: &piney_battle::tables::Tables,
        save: &mut piney_data::save::SaveData,
        party: &Party,
        ents: &[usize],
    ) {
        if self.chat_queue.is_empty() {
            return;
        }
        let q = std::mem::take(&mut self.chat_queue);
        // The game keeps one record: the members' copies are first taken
        // from the characters, so an affect since their frames (a felling's
        // act 9, a revive's act 2, `targetChar`) is what the lines change
        // and not undone by the copies written back below.
        let kite = self.kite;
        let members: Vec<usize> =
            (self.members.iter().map(|m| m.1)).filter(|&w| Some(w) != kite && self.crew.spc.contains_key(&w)).collect();
        for &w in &members {
            piney_battle::fellow::sync_in(&self.scene, &mut self.crew, w);
        }
        let game = party_ai::Game { in_battle: self.battle.in_battle, ..party_ai::Game::default() };
        let mut rt = chat::Quiet;
        let p = Parts {
            t,
            scene: &mut self.scene,
            party,
            save,
            crew: &mut self.crew,
            game: &game,
            ents,
            rng: &mut self.rand,
        };
        let mut ctx = p.ctx(&mut rt);
        for e in &q {
            chat::run(&mut ctx, e);
        }
        // What a line changed of a member's body (`ccAI::Greeting` stops it:
        // moveFlag and runFlag 0) into the character, or the member's next
        // frame reads the old flags back and it runs on while spoken to.
        for w in members {
            piney_battle::fellow::sync_out(&mut self.scene, &self.crew, w);
        }
    }

    /// `checkPartyAnnihilation()`.
    pub fn annihilated(&self) -> bool {
        self.party.annihilated(&self.scene)
    }

    /// One frame of the battle's tasks (see the module's table), after the
    /// camera's.
    pub fn frame(&mut self, x: &mut Tasks, fx: &mut dyn FxTasks) {
        let Some(kite_i) = self.kite else { return };
        self.cond_fx_shown = x.effect_sw;
        self.eye_view = x.camera.tcam.kind == crate::camera::kind::EYE;
        self.load_records(x.save);
        let d = self.data.clone();
        let t = &d.t;
        self.cast.new_frame();
        self.rays.clear();
        self.trails.clear();
        self.breaths.flames.clear();
        let first_show = self.shows.len();
        let area = x.scene.area;
        let in_battle = self.battle.in_battle;
        let party = self.party;
        // The lines the menus and events raised after last frame's tasks
        // (an item's heal on a member: ChatMessageThanksHeal), before this
        // frame's first draw.
        self.drain_chats(t, x.save, &party, &[]);
        // ccThSpc (48), ccThAISystem (48).
        if let Some(SpcOut::Shout { operation }) = self.spc.frame(t, x.save, &party, area, in_battle, x.puppet_show) {
            self.shows.push(Show::Shout(operation));
        }
        self.crew.tick();
        let env = Env {
            plcol: x.save.u8(piney_data::save::offset::PLCOL),
            menu_forbid: i16::from(x.menu_forbid),
            in_battle,
            count: x.count,
            menu_type: x.menu_type,
            sp_regene_speed: false,
            area,
        };
        let annihilated = self.annihilated();
        let game = party_ai::Game {
            in_battle,
            area,
            field: x.scene.dungeon,
            field_type: x.wm.field_type as i32,
            field_attr: 0,
            menu_type: x.menu_type,
            event_lock: i32::from(x.puppet_show),
            spc_battle_condition: self.spc.battle_condition,
            party_strategy: self.spc.party_strategy,
            player: Some(kite_i),
            field_24: x.scene.field,
            pg_ride_flag: i32::from(self.ride.flag),
            area_prev: x.scene.area_prev,
            server: x.scene.server,
        };
        let mut ents = self.ctrl.list(entry::Kind::Enemy);
        // The boss is on the enemies' command list too.
        ents.extend(self.boss_char());
        let game_e = self.entry_game(&x.scene, &x.wm);
        let bounds = Self::bounds(area, x.hits);
        let registry_num = self.members.len() as i32;
        let kite_pos = self.scene.chars[kite_i].pos;
        let mut stage = Stage {
            t,
            hits: x.hits,
            camera: x.camera,
            pad: x.pad,
            cast: &mut self.cast,
            skills: &self.skills,
            area,
            bounds,
            player: kite_pos,
            kite: self.kite,
            map2d: std::mem::take(&mut x.map2d),
            map2d_info: x.map2d_info,
            event_area: x.event_area,
            annihilated,
            shows: &mut self.shows,
            enter: false,
            view: [kite_pos[0], kite_pos[1], ee::add(kite_pos[2], 0x430c_0000), ONE],
            retarget: Vec::new(),
            kite_act: self.scene.chars[kite_i].spc_char.act_num,
            traps: Vec::new(),
            town: None,
            spcs: x.spcs.as_deref_mut(),
            tricks: chat::condition_skills(&self.data.t, &self.foes, self.scene.chars.len()),
            chats: Vec::new(),
            manual: self.crew.manual_chars(),
            item_uses: &mut self.member_items,
        };
        // SetPathFindingMap: the room's window of the 2D map into buf.
        if x.path_map {
            self.keep.path.set_path_finding_map(area, &mut stage);
        }
        // ccThPlayer (49): ccPlayer::Main, after the menu task's calls on
        // him; asleep while the Grunty carries him.
        let asleep = self.ride.asleep;
        if !asleep {
            let menu_calls = std::mem::take(&mut self.kite_menu);
            let input = kite::Input {
                pad: kite::Pad { pow_l: x.pad.pow_l, dirc_l: x.pad.dirc_l },
                cmnd_target: x.cmnd_target,
                warp: false,
                dne: false,
                bounds,
                menu: true,
            };
            let cell = RefCell::new(&mut stage);
            let mut host = kite::Host {
                t,
                mt: &d.mt,
                party: &party,
                game: &game,
                keep: &mut self.keep,
                spc_registry_num: registry_num,
                world: &cell,
            };
            let mut cx = kite::Cx {
                t,
                kt: &d.kt,
                scene: &mut self.scene,
                party: &party,
                save: x.save,
                crew: &mut self.crew,
                game: &game,
                ents: &ents,
                env: &env,
                input: &input,
                p: &mut self.player,
                rng: &mut self.rand,
                w: &mut host,
            };
            for call in menu_calls {
                match call {
                    KiteMenuCall::Break => kite::break_something(&mut cx, kite_i, None),
                    KiteMenuCall::AttackCancel => kite::attack_cancel(cx.scene, cx.crew, kite_i),
                }
            }
            kite::main(&mut cx, kite_i);
        }
        // ccThPucciguso (49, started after ccThPlayer): the riding Grunty.
        let pad = ride::RidePad {
            pad: kite::Pad { pow_l: x.pad.pow_l, dirc_l: x.pad.dirc_l },
            push: x.pad.push,
            pow_r: x.pad.pow_r,
        };
        let seek = if self.ride.main_on && self.data.volume != piney_data::volume::Volume::Inf {
            ride::SeekView::of(&self.ctrl, &self.scene, x.wm.event, x.dungeon, x.save.name().to_vec())
        } else {
            ride::SeekView::default()
        };
        ride::frame(
            &mut self.ride,
            &mut stage,
            &mut self.scene,
            &mut self.crew,
            kite_i,
            &mut self.rand,
            pad,
            area,
            &seek,
        );
        self.enter |= stage.enter;
        // The lines Kite's frame raised on the party (an affect on a
        // member), and its affects' work on a member's body, as it ends.
        for e in std::mem::take(&mut stage.chats) {
            if RuleParts::takes(&e) {
                let mut r = RuleParts {
                    t,
                    scene: &mut self.scene,
                    crew: &mut self.crew,
                    skills: &self.skills,
                    hits: &mut *stage.hits,
                    kite: self.kite,
                    rand: &mut self.rand,
                };
                r.apply(&e);
                continue;
            }
            let p = Parts {
                t,
                scene: &mut self.scene,
                party: &party,
                save: x.save,
                crew: &mut self.crew,
                game: &game,
                ents: &ents,
                rng: &mut self.rand,
            };
            chat::run(&mut p.ctx(&mut stage), &e);
        }
        // ccThFellowNN (50): each member's ccFellow::Main.
        let members: Vec<usize> = self.members.iter().map(|m| m.1).filter(|&m| m != kite_i && !asleep).collect();
        for m in members {
            stage.player = self.scene.chars[kite_i].pos;
            let checks = {
                let act = self.scene.chars[kite_i].spc_char.act_num;
                let s = self.skills.borrow();
                (0..self.scene.chars.len()).map(|c| s.check(t, &self.scene, c, act)).collect::<Vec<i32>>()
            };
            let chk = |c: usize| checks.get(c).copied().unwrap_or(0);
            let field = fellow::Field { gho_flag: stage.cast.gate.flag(), warp_flag: false };
            let out = {
                let cell = RefCell::new(&mut stage);
                let mut own = Own(&cell);
                let mut rt = Movement {
                    t,
                    mt: &d.mt,
                    party: &party,
                    game: &game,
                    keep: &mut self.keep,
                    spc_registry_num: registry_num,
                    world: &cell,
                    inner: &mut own,
                };
                let mut share = Share::new(&cell);
                let mut fr = fellow::Frame {
                    t,
                    mt: &d.mt,
                    scene: &mut self.scene,
                    party: &party,
                    save: x.save,
                    crew: &mut self.crew,
                    game: &game,
                    field: &field,
                    ents: &ents,
                    env: &env,
                    rng: &mut self.rand,
                    world: &mut share,
                    rt: &mut rt,
                    menu: true,
                    skill_check: &chk,
                    out: Vec::new(),
                };
                fr.main(m);
                fr.out
            };
            for o in out {
                match o {
                    fellow::Out::SetMatrix { pos, dirc } => {
                        if let Some(a) = stage.cast.get_mut(m) {
                            a.ch.pos = pos;
                            a.ch.dirc = dirc;
                        }
                    }
                    fellow::Out::Rule(Event::EnemyRetarget(Who::Char(c))) => stage.retarget.push(c),
                    // A line the member's own hit raised (ChatMessageAttack
                    // from its CheckNote), where its frame ends.
                    fellow::Out::Rule(e) if chat::line_of(&e).is_some() => {
                        let p = Parts {
                            t,
                            scene: &mut self.scene,
                            party: &party,
                            save: x.save,
                            crew: &mut self.crew,
                            game: &game,
                            ents: &ents,
                            rng: &mut self.rand,
                        };
                        chat::run(&mut p.ctx(&mut stage), &e);
                        stage.shows.push(Show::Member(m, fellow::Out::Rule(e)));
                    }
                    fellow::Out::StartArmsEffect(sid) => {
                        let ch = &mut self.scene.chars[m];
                        for [a, b, row] in piney_battle::skill::start_arms_effect(t, ch, i32::from(sid)) {
                            stage.shows.push(Show::ArmsParticles { who: m, a, b, row });
                        }
                    }
                    o => stage.shows.push(Show::Member(m, o)),
                }
            }
        }
        // The fellows' draws: where their frames left them.
        for &(_, m) in self.members.iter().filter(|m| m.1 != kite_i && !asleep) {
            let s = self.crew.spc.get(&m).copied().unwrap_or_default();
            let c = &self.scene.chars[m];
            if let Some(a) = stage.cast.get(m) {
                let (w, h, td) = (a.ch.width, a.ch.height, a.trans_dist);
                let disp = c.spc_char.flags & bit::DISP != 0;
                let (alpha, drawn) = stage.char_draw(c.pos, w, h, s.set_transparency, td);
                if let Some(a) = stage.cast.get_mut(m) {
                    a.alpha = alpha;
                    a.drawn = drawn && disp;
                }
            }
        }
        // An enemy a party member's hit reached picks its target again.
        let retarget = std::mem::take(&mut stage.retarget);
        // ccThEntryCtrl (64).
        stage.player = self.scene.chars[kite_i].pos;
        let checks = {
            let act = self.scene.chars[kite_i].spc_char.act_num;
            let s = self.skills.borrow();
            (0..self.scene.chars.len()).map(|c| s.check(t, &self.scene, c, act)).collect::<Vec<i32>>()
        };
        let chk = |c: usize| checks.get(c).copied().unwrap_or(0);
        let aff = AffectCtx { party: &party, menu: true, skill_check: &chk, boss: None, volume: self.data.volume };
        let pframe = stage::PlayerFrame { bounds, player: self.scene.chars[kite_i].pos };
        let mut cleared = Vec::new();
        {
            let mut ai = enemy_ai::Ai {
                t,
                scene: &mut self.scene,
                foes: &mut self.foes[..],
                world: enemy_ai::World {
                    puppet_show: x.puppet_show,
                    ride: self.ride.flag,
                    active_enemies: 0,
                    player: Some(kite_i),
                    frame: &pframe,
                },
                rand: &mut self.rand,
                cc: &mut self.cc,
                out: Vec::new(),
            };
            for c in retarget {
                if ai.foes.get(c).is_some_and(Option::is_some) {
                    ai.select_target(c);
                }
                cleared.push((c, std::mem::take(&mut ai.out)));
            }
        }
        for (c, out) in cleared {
            enemy_cleared(&mut self.cond_fx, stage.shows, c, &out);
        }
        let mut outs = Vec::new();
        {
            let words = x.wm.words;
            let mut inner = Objects;
            let mut seam = entry::EnemySeam {
                data: &d.md,
                affect: &aff,
                env: &env,
                rand: &mut self.rand,
                frame: &pframe,
                puppet_show: x.puppet_show,
                ride: self.ride.flag,
                player: Some(kite_i),
                book_area: words,
                inner: &mut inner,
            };
            let mut cx = entry::Cx {
                t,
                st: &d.st,
                scene: &mut self.scene,
                foes: &mut self.foes,
                world: &mut stage,
                cc: &mut self.cc,
                rnds: &mut self.rnds,
                save: x.save,
                game: game_e,
                reg: &self.reg,
                part_pats: self.part_pats,
                out: &mut outs,
            };
            self.ctrl.frame(&mut cx, &mut seam);
        }
        // The lines the enemies' hits raised on the party
        // (ChatMessageDamage, ResurrectPlz, AffectMessages) and the rest of
        // their Influence's work, in order (RuleParts).
        for e in std::mem::take(&mut stage.chats) {
            if RuleParts::takes(&e) {
                let mut r = RuleParts {
                    t,
                    scene: &mut self.scene,
                    crew: &mut self.crew,
                    skills: &self.skills,
                    hits: &mut *stage.hits,
                    kite: self.kite,
                    rand: &mut self.rand,
                };
                r.apply(&e);
                continue;
            }
            let p = Parts {
                t,
                scene: &mut self.scene,
                party: &party,
                save: x.save,
                crew: &mut self.crew,
                game: &game,
                ents: &ents,
                rng: &mut self.rand,
            };
            chat::run(&mut p.ctx(&mut stage), &e);
        }
        let traps = std::mem::take(&mut stage.traps);
        self.symbol_fires.clear();
        for o in outs {
            match o {
                entry::Out::SymbolFire { pos, pattern, scale, colour } => {
                    self.symbol_fires.push((pos, pattern, scale, colour));
                }
                // ccGimSymbol's AddGrp / DelGrp of its light.
                entry::Out::SymbolLight { who, pos, intensity } => {
                    match pos {
                        Some(p) => self.symbol_lights.insert(who, (p, intensity)),
                        None => self.symbol_lights.remove(&who),
                    };
                }
                // ccGimEtc's AddGrp / DelGrp of the spring's lights.
                entry::Out::EtcLights { who, lights } => {
                    match lights {
                        Some(l) => self.spring_lights.insert(who, l),
                        None => self.spring_lights.remove(&who),
                    };
                }
                // The spring's ccAnm::Draw at SetMatrix_PosRotZYXScale on
                // refLayer.
                entry::Out::EtcDraw { who, pos, dirc, scale } => {
                    if let Some(a) = stage.cast.get_mut(who) {
                        a.drawn = true;
                        a.alpha = ONE;
                        a.matrix =
                            Some(crate::field_ambient::pos_rot_xyz_scale(pos, dirc, [scale[0], scale[1], scale[2]]));
                    }
                }
                entry::Out::CircleDraw { who, transparency } => {
                    if let Some(a) = stage.cast.get_mut(who) {
                        a.drawn = true;
                        a.alpha = transparency;
                        a.ch.pos = self.scene.chars[who].pos;
                        if let Some(obj) = self.ctrl.entry_obj(who) {
                            a.ch.dirc = obj.dirc;
                        }
                    }
                }
                entry::Out::Destroyed { who, .. } => {
                    stage.cast.actors.remove(&who);
                    self.symbol_lights.remove(&who);
                    self.spring_lights.remove(&who);
                    self.rad_lights.remove(&who);
                    stage.shows.push(Show::Entry(o));
                }
                // ccGimBox::main's ccAnm::Draw, ccGimIdol::main's
                // ccChar::Draw: at the object's place and heading, the box
                // at its transparency, with its row's palette.
                entry::Out::GimDraw { who, dirc, .. } | entry::Out::IdolDraw { who, dirc }
                    if stage.cast.get(who).is_some() =>
                {
                    let alpha = match o {
                        entry::Out::GimDraw { .. } => transparency_of(&o),
                        _ => ONE,
                    };
                    let row = self.ctrl.entry_obj(who).map_or(-1, |e| e.gim_id);
                    let swaps = stage.cast.looks.gimmick(row).map(|g| g.model.clut_swaps()).unwrap_or_default();
                    if let Some(a) = stage.cast.get_mut(who) {
                        a.drawn = true;
                        a.alpha = alpha;
                        a.ch.pos = self.scene.chars[who].pos;
                        a.ch.dirc = dirc;
                        a.swaps = swaps;
                    }
                    stage.shows.push(Show::Entry(o));
                }
                // ccGimFood::main's SetMatrix_PosRotZYXScale and
                // ccChar::Draw: at its place, turn and scale, at the
                // transparency its fade left.
                entry::Out::FoodDraw { who, dirc, scale } if stage.cast.get(who).is_some() => {
                    let alpha = self.ctrl.entry_obj(who).map_or(ONE, |e| e.transparency);
                    let pos = self.scene.chars[who].pos;
                    if let Some(a) = stage.cast.get_mut(who) {
                        a.drawn = true;
                        a.alpha = alpha;
                        a.ch.pos = pos;
                        a.ch.dirc = dirc;
                        a.matrix = Some(food_matrix(pos, dirc, scale));
                    }
                    stage.shows.push(Show::Entry(o));
                }
                entry::Out::Radiate { out: piney_battle::prim::RadOut::Draw(ref rays), .. } => {
                    self.rays.push(rays.clone());
                }
                // ccPrimRadiate::setLight's AddGrp, delLight's DelGrp.
                entry::Out::Radiate { who, out: piney_battle::prim::RadOut::LightOn(l) } => {
                    self.rad_lights.insert(who, l);
                }
                entry::Out::Radiate { who, out: piney_battle::prim::RadOut::LightOff } => {
                    self.rad_lights.remove(&who);
                }
                // ccSpcMessageOpenTrapBox: ccAISysMsgSend(name, -1, Kite's
                // AI, 0xffff, 0, 30).
                entry::Out::SpcMessage(name) => {
                    let id = self.crew.sys_msg_id(kite_i);
                    self.crew.send(name, id as u16, 0xffff, 0, 30, -1, None);
                }
                o => stage.shows.push(Show::Entry(o)),
            }
        }
        // A box's trap on its opener (ccGimBox::invokeTrap, inside the
        // box's frame in the game): trap 0's CalcBattleDamage(opener,
        // skill, 1.0, -1) and the opener's EntryAffect(box, 1, damage);
        // traps 1 and 2 ccItemSkillRequest(box, opener, skill, 0).
        let mut trap_events = Vec::new();
        for (who, target, sid, hit, listed) in traps {
            let Some(tg) = target else { continue };
            if hit {
                let Some(sk) = t.skill(sid).cloned() else { continue };
                let mut ev = Events::new();
                let att = self.scene.chars[who].clone();
                let d = piney_battle::damage::calc_battle_damage(
                    t,
                    &att,
                    &mut self.scene.chars[tg],
                    &sk,
                    ONE,
                    piney_battle::damage::Roll::Draw,
                    listed,
                    &mut self.rand,
                    &env,
                    &mut ev,
                );
                affect::entry_affect(
                    t,
                    &mut self.scene,
                    &aff,
                    tg,
                    Some(who),
                    1,
                    [d.dmg as i16, 0, 0],
                    &mut self.rand,
                    &mut ev,
                );
                trap_events.extend(ev);
            } else {
                let (_, req) =
                    self.skills.borrow_mut().request(t, &mut self.scene, who, Some(tg), sid, 1, &mut self.rand);
                stage::skill_shows(stage.shows, who, Some(tg), sid, req.as_ref());
            }
        }
        // The kills' experience (ccExpDistributor from the enemies'
        // frames).
        let exps: Vec<i16> = stage.shows[first_show..]
            .iter()
            .filter_map(|s| match s {
                Show::Enemy(_, piney_battle::enemy_motion::Call::Rule(enemy_ai::Out::Exp { level })) => Some(*level),
                _ => None,
            })
            .collect();
        // A gold goblin shaking off a hold: game.inBattleDist, which
        // ccThGameCtrl reads on the next frame.
        let dist = stage.shows[first_show..].iter().rev().find_map(|s| match s {
            Show::Enemy(_, piney_battle::enemy_motion::Call::Rule(enemy_ai::Out::InBattleDist(d))) => Some(*d),
            _ => None,
        });
        drop(stage);
        if let Some(d) = dist {
            self.battle.dist = d;
        }
        // consequence shows what it does not consume, as for every other
        // source of events: shown here too, a trap's number or mark came
        // up twice.
        for e in trap_events {
            self.consequence(e, x.hits, bounds, x.puppet_show);
        }
        self.drain_chats(t, x.save, &party, &ents);
        // ccThBossEffect (66): the boss effect manager's pass (on past the
        // boss's exit, as its task is); ccThBoss01 (66): the boss, then the
        // effects its Main made.
        if self.boss.is_some() {
            self.fx_call(x, bounds, |fx, w| fx.boss_effects(w), fx, None);
            let world = enemy_ai::World {
                puppet_show: x.puppet_show,
                ride: self.ride.flag,
                active_enemies: 0,
                player: Some(kite_i),
                frame: &pframe,
            };
            // The events of the boss's hits (ccBossSkillDamage's EntryAffect
            // on a member): consequence shows them, once.
            for e in self.boss_frame(&env, world, &chk, x.hits, x.camera, &x.pad) {
                self.consequence(e, x.hits, bounds, x.puppet_show);
            }
            self.drain_chats(t, x.save, &party, &ents);
            self.fx_call(x, bounds, |fx, w| fx.boss_shows(w), fx, None);
        }
        // ccChar::Draw of each enemy dispEnemy placed: its fog blend and
        // transparency (the affect flash moves on as it draws).
        let fcam = crate::foe::Camera {
            player: self.scene.chars[kite_i].pos,
            cam: x.camera.active().pos,
            deg1: x.camera.active().deg[1],
            eye: x.camera.tcam.kind == crate::camera::kind::EYE,
            bounds: [bounds.min[0], bounds.min[1], bounds.max[0], bounds.max[1]],
            town: false,
            volume: x.camera.volume,
        };
        for (&who, a) in self.cast.actors.iter_mut() {
            a.how = None;
            // ccBoss::Draw (gcmn 0x0045d4b0): ccChar::Draw's blend (a hit's
            // flash, the conditions' colours), but none while the boss is
            // off the command lists (m_bEraseTarget, its `dead` 1).
            if a.look == Look::Boss {
                let ch = &mut self.scene.chars[who];
                let erased = ch.foe_state().and_then(|f| f.boss.as_ref()).is_some_and(|b| b.erase_target != 0);
                let dead = ch.cond[cond::DEAD];
                let blend = if dead == 1 && erased {
                    crate::foe::FogBlend::default()
                } else {
                    crate::foe::char_blend(dead, ch.condition_num, x.effect_sw, &mut ch.affect)
                };
                a.ch.blend = blend.fog();
                continue;
            }
            if !matches!(a.look, Look::Enemy(_)) || a.matrix.is_none() {
                continue;
            }
            let st = self.foes.get(who).and_then(Option::as_ref).map_or(ONE, |f| f.set_transparency);
            a.how = Some(crate::foe::char_draw(&mut self.scene.chars[who], st, true, x.effect_sw, 0, &fcam));
        }
        for level in exps {
            for g in exp::exp_distributor(t, &mut self.scene, &party, x.save, level, x.count) {
                self.shows.push(Show::Exp(g));
            }
        }
        // The party's weapon trails as their frames drew them.
        self.arms_shown(first_show);
        // What the characters' frames asked of their condition effects.
        self.disp_conditions_shown(first_show);
        // ccThEffect (80): a spell's element makes its damage calls here.
        let mut ev = Events::new();
        self.fx_call(x, bounds, |fx, w| fx.effect(w), fx, Some((&env, &mut ev)));
        // ccThSkill (82).
        let player = self.scene.chars[kite_i].pos;
        let w2p = move |p: V4| kite::w2p_pos(&bounds, player, p).0;
        let base = flow::Frame { env: &env, w2p: &w2p, anim_done: false, notes: &[], annihilated };
        // Skills::frame's walk, with an attack spell's element system run
        // where ccSkill::Main runs it (the effects').
        let mut i = 0;
        loop {
            // A run's own animation, stepped where `Main` steps it; with
            // no animation to be had, it ends at once rather than hold the
            // caster (the game's would never end).
            let anim = {
                let sk = self.skills.borrow();
                let Some(run) = sk.runs.get(i) else { break };
                if run.has_anm && run.status == 0 {
                    let file = t.skill(run.id).map(|p| String::from_utf8_lossy(&p.filename).into_owned());
                    let key = run.key;
                    drop(sk);
                    Some(file.and_then(|f| fx.skill_anim(key, &f)).unwrap_or((true, Vec::new())))
                } else {
                    None
                }
            };
            let (key, mut st, out, caster, target_pos, (sid, stype, ty)) = {
                let mut sk = self.skills.borrow_mut();
                let Some(run) = sk.runs.get_mut(i) else { break };
                let mut out = flow::MainOut::default();
                let st = match &anim {
                    Some((done, notes)) => {
                        let f = flow::Frame { anim_done: *done, notes, ..base };
                        run.main(t, &mut self.scene, &f, &mut self.rand, &mut ev, &mut out)
                    }
                    None => run.main(t, &mut self.scene, &base, &mut self.rand, &mut ev, &mut out),
                };
                (run.key, st, out, run.creator, run.target_pos, (run.id, run.stype, run.ty))
            };
            // ccSkillModifyCondition's effSkillStart(ch, sid, stype == 2, 1)
            // on each it took, the "miss" font (ccEntryFlyFontNew(2, -1))
            // over each that resisted; ccSkillCheckNote's shock waves.
            for &c in &out.modify.started {
                let a = i32::from(i32::from(stype) == 2);
                self.shows.push(Show::SkillStart { who: c, sid, a, b: 1 });
            }
            for &c in &out.modify.resisted {
                self.shows.push(Show::Rule(Event::FlyFont { on: Who::Char(c), kind: 2, value: -1 }));
            }
            for _ in 0..out.shock_waves {
                self.shows.push(Show::ShockWave { pos: target_pos, attr: ty & 0xfc });
            }
            if out.heal_sound || !out.sounds.is_empty() {
                self.shows.push(Show::SkillSounds {
                    caster,
                    target_pos,
                    heal: out.heal_sound,
                    notes: out.sounds.clone(),
                });
            }
            if out.spell.is_some() && st == 0 {
                st = self.spell_system(x, bounds, &env, i, &mut ev, fx);
            }
            if st != 0 {
                self.skills.borrow_mut().runs.remove(i);
                fx.spell_remove(key);
            } else {
                i += 1;
            }
            self.shows.push(Show::Skill(flow::Step { key, out, over: st != 0 }));
        }
        let checks = self.skill_checks();
        let chk = |c: usize| checks.get(c).copied().unwrap_or(0);
        let aff = AffectCtx { party: &party, menu: true, skill_check: &chk, boss: None, volume: self.data.volume };
        let ev = affect::apply(t, &mut self.scene, &aff, ev, &mut self.rand);
        for e in ev {
            self.consequence(e, x.hits, bounds, x.puppet_show);
        }
        self.drain_chats(t, x.save, &party, &ents);
        // The party's records back into the save (experience, levels).
        self.store_records(x.save);
        // ccThParticle (98).
        self.fx_call(x, bounds, |fx, w| fx.particle(w), fx, None);
        // ccThFieldDisp (96) draws the party: the hands the next frame's
        // tasks read.
        for &(_, c) in &self.members {
            if let Some(a) = self.cast.get_mut(c) {
                a.keep_hand();
            }
        }
    }

    /// Run `i`'s spell system through the effects, its damage calls made
    /// on the battle as it makes them ([`FxWorld::skill_damage`]); the
    /// run's status after it.
    fn spell_system(
        &mut self,
        x: &mut Tasks,
        bounds: MapBounds,
        env: &Env,
        i: usize,
        ev: &mut Events,
        fx: &mut dyn FxTasks,
    ) -> i8 {
        let run = {
            let sk = self.skills.borrow();
            let r = &sk.runs[i];
            SpellRun {
                key: r.key,
                sid: r.id,
                stype: r.stype,
                count: r.count.wrapping_sub(1),
                hold: r.hold,
                c_pos: r.pos,
                c_dirc: r.dirc,
                t_pos: r.target_pos,
                t_type: r.target_type,
                creator: r.creator,
                target: r.target,
            }
        };
        let mut out = SpellOut::default();
        self.fx_call(x, bounds, |f, w| out = f.spell(w, &run), fx, Some((env, ev)));
        if !out.ran {
            return self.skills.borrow().runs[i].status;
        }
        for c in out.released {
            if let Some(ch) = self.scene.chars.get_mut(c) {
                ch.skill_id = 0;
                ch.skill_status = 0;
            }
        }
        let mut sk = self.skills.borrow_mut();
        let r = &mut sk.runs[i];
        r.status = out.status;
        r.hold = out.hold;
        r.level = out.level;
        r.status
    }

    /// One of the effect tasks over the battle as it stands, with the
    /// shows it has not seen.
    fn fx_call(
        &mut self,
        x: &mut Tasks,
        bounds: MapBounds,
        run: impl FnOnce(&mut dyn FxTasks, &mut FxWorld),
        fx: &mut dyn FxTasks,
        damage: Option<(&Env, &mut Events)>,
    ) {
        let from = self.fx_seen.min(self.shows.len());
        self.dust_pass(from);
        if let Some(k) = self.kite {
            let frame = stage::PlayerFrame { bounds, player: self.scene.chars[k].pos };
            self.weapon_pass(from, x.camera, &frame);
            let frame = stage::PlayerFrame { bounds, player: self.scene.chars[k].pos };
            self.breath_pass(from, x.hits, &frame);
        }
        let d = self.data.clone();
        let mut w = FxWorld {
            t: &d.t,
            scene: &mut self.scene,
            foes: &self.foes,
            ctrl: &self.ctrl,
            cast: &self.cast,
            party: &self.party,
            kite: self.kite,
            members: &self.members,
            rand: &mut self.rand,
            cc: &mut self.cc,
            camera: x.camera,
            hits: x.hits,
            bounds,
            area: (x.scene.area, x.scene.dungeon, x.wm.field_type as i32),
            field: x.scene.field,
            shows: &self.shows[from..],
            damage: damage.map(|(env, ev)| FxDamage { skills: &self.skills, env, ev }),
        };
        run(fx, &mut w);
        self.fx_seen = self.shows.len();
    }

    /// What an event of the rules leads to, outside the affects already
    /// applied: the calls with state made (a normal attack called off, a
    /// body switched, the AI bus's "down" and "up", a talk ended, an enemy
    /// retargeting); the rest shown.
    fn consequence(&mut self, e: Event, hits: &mut Hits, bounds: MapBounds, puppet_show: bool) {
        if chat::line_of(&e).is_some() {
            self.chat_queue.push(e);
            self.shows.push(Show::Rule(e));
            return;
        }
        let d = self.data.clone();
        let mut parts = RuleParts {
            t: &d.t,
            scene: &mut self.scene,
            crew: &mut self.crew,
            skills: &self.skills,
            hits,
            kite: self.kite,
            rand: &mut self.rand,
        };
        if parts.apply(&e) {
            return;
        }
        match e {
            Event::Affect { .. } => {}
            Event::RecoveryReq { on: Who::Char(c), amount } => self.recovery.entry(c, amount),
            Event::EnemyRetarget(Who::Char(c)) => {
                let Some(k) = self.kite else { return };
                let pframe = stage::PlayerFrame { bounds, player: self.scene.chars[k].pos };
                let mut ai = enemy_ai::Ai {
                    t: &self.data.t,
                    scene: &mut self.scene,
                    foes: &mut self.foes[..],
                    world: enemy_ai::World {
                        puppet_show,
                        ride: self.ride.flag,
                        active_enemies: 0,
                        player: Some(k),
                        frame: &pframe,
                    },
                    rand: &mut self.rand,
                    cc: &mut self.cc,
                    out: Vec::new(),
                };
                if ai.foes.get(c).is_some_and(Option::is_some) {
                    ai.select_target(c);
                }
                let out = std::mem::take(&mut ai.out);
                enemy_cleared(&mut self.cond_fx, &mut self.shows, c, &out);
            }
            Event::DispCondition(Who::Char(c)) => self.disp_condition(c),
            // ccChar::ClearConditionEffect: deleteConditionEffect, num -1
            // (the rule set the number already).
            Event::ClearConditionEffect(Who::Char(c)) => {
                if self.cond_fx.remove(&c).is_some() {
                    self.shows.push(Show::ConditionEffect { who: c, act: CondFx::Delete, num: -1 });
                }
            }
            e => self.shows.push(Show::Rule(e)),
        }
    }

    /// `ccChar::DispConditionEffect()` on character `c`: the condition it
    /// shows, and its effect kept, remade or ended
    /// ([`piney_battle::chara::disp_condition_effect`]).
    fn disp_condition(&mut self, c: usize) {
        let Some(ch) = self.scene.chars.get_mut(c) else { return };
        let ep = self.cond_fx.get(&c).copied();
        let act =
            piney_battle::chara::disp_condition_effect(ch, ep, self.cond_fx_shown, self.eye_view, self.data.volume);
        let num = ch.condition_num;
        match act {
            CondFx::Keep => return,
            CondFx::Kill | CondFx::Delete => {
                self.cond_fx.remove(&c);
            }
            CondFx::Set | CondFx::Replace => {
                self.cond_fx.insert(c, num);
            }
        }
        self.shows.push(Show::ConditionEffect { who: c, act, num });
    }

    /// The `DispConditionEffect` calls the characters' own frames made
    /// (Kite's, the members', the enemies' condition counts), and the
    /// `ClearConditionEffect`s of the frames (an enemy's
    /// `clearConditionEnemy` as it dies, a felled party character's
    /// `Influence`), shows from `from` on, in order.
    fn disp_conditions_shown(&mut self, from: usize) {
        use piney_battle::{enemy_ai, enemy_motion, fellow, kite};
        let who = |me: usize, w: Who| match w {
            Who::Me => Some(me),
            Who::Char(c) => Some(c),
            Who::Target | Who::Nobody => None,
        };
        enum Call {
            /// `DispConditionEffect`; `party` for Kite's and a member's,
            /// which their frames make only while they stand.
            Disp {
                party: bool,
            },
            Clear,
        }
        let calls: Vec<(usize, Call)> = self.shows[from.min(self.shows.len())..]
            .iter()
            .filter_map(|s| match s {
                Show::Kite(k, kite::Out::Rule(Event::DispCondition(w)))
                | Show::Member(k, fellow::Out::Rule(Event::DispCondition(w))) => {
                    who(*k, *w).map(|c| (c, Call::Disp { party: true }))
                }
                Show::Enemy(k, enemy_motion::Call::Rule(enemy_ai::Out::Rule(Event::DispCondition(w)))) => {
                    who(*k, *w).map(|c| (c, Call::Disp { party: false }))
                }
                Show::Enemy(k, enemy_motion::Call::Rule(enemy_ai::Out::ClearConditionEffect)) => {
                    Some((*k, Call::Clear))
                }
                // Influence's ClearConditionEffect from an affect in a
                // frame (Kite or a member felled by a blow there, by poison).
                Show::Kite(k, kite::Out::Rule(Event::ClearConditionEffect(w)))
                | Show::Member(k, fellow::Out::Rule(Event::ClearConditionEffect(w)))
                | Show::Enemy(k, enemy_motion::Call::Rule(enemy_ai::Out::Rule(Event::ClearConditionEffect(w)))) => {
                    who(*k, *w).map(|c| (c, Call::Clear))
                }
                _ => None,
            })
            .collect();
        for (c, call) in calls {
            match call {
                // Shown standing, felled since: the fall's clear ends the
                // effect at once, as in the game, not this late look's fade.
                Call::Disp { party: true } if self.scene.chars[c].cond[cond::DEAD] != 0 => {}
                Call::Disp { .. } => self.disp_condition(c),
                Call::Clear => {
                    if self.cond_fx.remove(&c).is_some() {
                        // deleteConditionEffect, conditionNum -1 (set by the rule).
                        self.shows.push(Show::ConditionEffect { who: c, act: CondFx::Delete, num: -1 });
                    }
                }
            }
        }
    }

    /// `ccSpcChar::ArmsEffect`, `ClearArmsEffect` and `SetArmsEffectColor`
    /// as Kite's and the members' frames called them (shows from `from`
    /// on), in order: each character's weapon trails and points
    /// ([`crate::arms::Arms`]) from its hands as its actor stands posed
    /// now. A character none called draws no trail this frame.
    fn arms_shown(&mut self, from: usize) {
        enum Call {
            Effect,
            Clear,
            Colour(i32),
        }
        let t = &self.data.t;
        for a in self.cast.actors.values_mut() {
            if let Some(w) = a.weapon.as_mut() {
                w.strips.clear();
            }
        }
        let calls: Vec<(usize, Call)> = self.shows[from.min(self.shows.len())..]
            .iter()
            .filter_map(|s| match s {
                Show::Kite(k, kite::Out::ArmsEffect) | Show::Member(k, fellow::Out::ArmsEffect) => {
                    Some((*k, Call::Effect))
                }
                Show::Kite(k, kite::Out::ClearArmsEffect) | Show::Member(k, fellow::Out::ClearArmsEffect) => {
                    Some((*k, Call::Clear))
                }
                Show::Kite(k, kite::Out::ArmsEffectColor { sid }) => Some((*k, Call::Colour(*sid))),
                Show::Member(k, fellow::Out::ArmsEffectColor(sid)) => Some((*k, Call::Colour(i32::from(*sid)))),
                _ => None,
            })
            .collect();
        for (who, call) in calls {
            let Some(ch) = self.scene.chars.get(who) else { continue };
            let (job, weapon) = ch.spc().map_or((-1, -1), |p| (p.job, p.equipment[4]));
            let trajectory = ch.spc_char.flags & spc_flag::TRAJECTORY != 0;
            let skill_id = ch.skill_id;
            let Some(a) = self.cast.get_mut(who) else { continue };
            let body = a.ch.body.clone();
            let w = a.weapon.get_or_insert_with(|| {
                let mut w = crate::arms::Arms::default();
                w.equip(&body, job);
                Box::new(w)
            });
            match call {
                Call::Effect => {
                    let root = a.ch.root();
                    let worlds = body.worlds(&a.ch.play, root);
                    let (r, l) = crate::arms::Arms::hands(&body, &worlds, root);
                    let aura = t.equip(i32::from(job), i32::from(weapon)).map_or(-1, |e| e.aura_sw);
                    w.effect(r, l, aura, trajectory, skill_id);
                }
                Call::Clear => w.clear(),
                Call::Colour(sid) => {
                    let stype = t.skill(sid).map_or(0, |s| s.ty);
                    w.set_color(piney_battle::skill::check_type_attribute(stype));
                }
            }
        }
    }

    /// `ccThEvHold` (main 0x001b5040, priority 33), once a frame while an
    /// event's `hold type code` task runs ([`evparty::hold_frame`]: from
    /// the frame after `hold`, the enemies of that type and id held, the
    /// goblins turned to Kite). The event's boss (`bossEntry`, `bossTscb`)
    /// is not kept, so type 7 holds nothing.
    pub fn ev_hold_frame(&mut self, hits: &mut Hits, bounds: MapBounds) {
        if self.ev_hold.is_none() {
            return;
        }
        let d = self.data.clone();
        let party = self.party;
        let checks = self.skill_checks();
        let chk = |c: usize| checks.get(c).copied().unwrap_or(0);
        let aff = AffectCtx { party: &party, menu: true, skill_check: &chk, boss: None, volume: self.data.volume };
        let mut ev = Events::new();
        let task = self.ev_hold.as_mut().expect("checked above");
        evparty::hold_frame(
            task,
            &d.t,
            &mut self.scene,
            &self.ctrl,
            &mut self.foes,
            self.kite,
            evparty::Boss::default(),
            &aff,
            &mut self.rand,
            &mut ev,
        );
        for e in ev {
            self.consequence(e, hits, bounds, false);
        }
    }

    /// The registry as the events' party instructions read it: each slot's
    /// `charTbl` row (`ids`, -1 free) and the scene index of the character
    /// built for it, with the party.
    pub fn roster(&self, ids: [i32; 5]) -> Roster {
        let mut r = Roster { ids, party: self.party, ..Roster::default() };
        for (slot, &id) in ids.iter().enumerate() {
            if id >= 0 {
                r.chars[slot] = self.who(id);
            }
        }
        r
    }

    /// An event's party instruction ([`EvParty`]) over the battle's
    /// characters, the area's collision and the player's frame; with the
    /// characters it took off the command lists (`ccDeleteCmnd`).
    #[allow(clippy::too_many_arguments)]
    pub fn with_ev_party<R>(
        &mut self,
        hits: &mut Hits,
        camera: &mut Camera,
        area: i32,
        roster: &Roster,
        positions: &[entry::EvPos],
        f: impl FnOnce(&mut EvParty) -> R,
    ) -> (R, Vec<entry::Out>) {
        let d = self.data.clone();
        let bounds = Self::bounds(area, hits);
        let player = self.kite.map_or(ee::VF0, |k| self.scene.chars[k].pos);
        let annihilated = self.party.annihilated(&self.scene);
        let mut stage = Stage {
            t: &d.t,
            hits,
            camera,
            pad: CamPad::default(),
            cast: &mut self.cast,
            skills: &self.skills,
            area,
            bounds,
            player,
            kite: self.kite,
            map2d: Vec::new(),
            map2d_info: [0; 3],
            event_area: false,
            annihilated,
            shows: &mut self.shows,
            enter: false,
            view: player,
            retarget: Vec::new(),
            kite_act: 0,
            traps: Vec::new(),
            town: None,
            spcs: None,
            tricks: chat::condition_skills(&self.data.t, &self.foes, self.scene.chars.len()),
            chats: Vec::new(),
            manual: self.crew.manual_chars(),
            item_uses: &mut self.member_items,
        };
        let mut out = Vec::new();
        let no_town = |_: i16| None;
        let mut p = EvParty {
            scene: &mut self.scene,
            crew: &mut self.crew,
            roster,
            ctrl: &self.ctrl,
            foes: &mut self.foes,
            world: &mut stage,
            area,
            positions,
            town_marker: &no_town,
            out: &mut out,
        };
        let r = f(&mut p);
        (r, out)
    }

    /// A decision of the party AI made outside the frames (the CHAT
    /// menu's orders): `f` on a [`party_ai::Ctx`] whose runtime is the
    /// field.
    #[allow(clippy::too_many_arguments)]
    pub fn with_ai<R>(
        &mut self,
        hits: &mut Hits,
        camera: &mut Camera,
        scene: &crate::area::Scene,
        save: &mut SaveData,
        in_battle: i32,
        f: impl FnOnce(&mut party_ai::Ctx) -> R,
    ) -> R {
        let d = self.data.clone();
        let t = &d.t;
        let area = scene.area;
        let bounds = Self::bounds(area, hits);
        let player = self.kite.map_or(ee::VF0, |k| self.scene.chars[k].pos);
        let game = party_ai::Game {
            in_battle,
            area,
            field: scene.dungeon,
            menu_type: -1,
            spc_battle_condition: self.spc.battle_condition,
            party_strategy: self.spc.party_strategy,
            player: self.kite,
            field_24: scene.field,
            ..party_ai::Game::default()
        };
        let ents = self.ctrl.list(entry::Kind::Enemy);
        let annihilated = self.party.annihilated(&self.scene);
        let mut stage = Stage {
            t,
            hits,
            camera,
            pad: CamPad::default(),
            cast: &mut self.cast,
            skills: &self.skills,
            area,
            bounds,
            player,
            kite: self.kite,
            map2d: Vec::new(),
            map2d_info: [0; 3],
            event_area: false,
            annihilated,
            shows: &mut self.shows,
            enter: false,
            view: player,
            retarget: Vec::new(),
            kite_act: 0,
            traps: Vec::new(),
            town: None,
            spcs: None,
            tricks: chat::condition_skills(&self.data.t, &self.foes, self.scene.chars.len()),
            chats: Vec::new(),
            manual: self.crew.manual_chars(),
            item_uses: &mut self.member_items,
        };
        let mut ctx = party_ai::Ctx {
            t,
            scene: &mut self.scene,
            party: &self.party,
            save,
            crew: &mut self.crew,
            game: &game,
            ents: &ents,
            rng: &mut self.rand,
            rt: &mut stage,
        };
        f(&mut ctx)
    }

    /// A party character's `ccSpcChar` members through `f` (piney-world's
    /// [`crate::ai::SpcRef`]: `ManualModeAI`, `SetRemoteCmd` ...), written
    /// back.
    pub fn with_spc<R>(
        &mut self,
        who: usize,
        hits: &mut Hits,
        f: impl FnOnce(&mut crate::ai::SpcRef, &mut Hits) -> R,
    ) -> R {
        let td = self.cast.get(who).is_none_or(|a| a.trans_dist);
        let mut r = spc::SpcRec::read(&self.scene, &self.crew, who, self.kite, td);
        let out = f(&mut r.spc_ref(), hits);
        let t = r.write(&mut self.scene, &mut self.crew);
        if let Some(a) = self.cast.get_mut(who) {
            a.trans_dist = t;
        }
        if who != self.kite.unwrap_or(usize::MAX) {
            fellow::sync_in(&self.scene, &mut self.crew, who);
        }
        out
    }

    /// `ccUseItemRequest(cp, tp, code, 0)`'s rules (`piney_battle::item`):
    /// the steps of the use, the scene already changed by it.
    pub fn use_item(
        &mut self,
        cp: usize,
        tp: usize,
        code: i32,
        parody: bool,
        arg: i32,
    ) -> Vec<piney_battle::item::Step> {
        let t = &self.data.t;
        let mut env = piney_battle::item::ItemEnv {
            player: self.kite,
            party: self.party,
            parody,
            control_mode: i32::from(kite::check_control_mode(&self.crew, cp)),
            sys_msg_id: self.crew.sys_msg_id(cp) as u16,
            running_attack: self.skills.borrow().attack_running(cp),
            ..Default::default()
        };
        piney_battle::item::use_item_request(t, &mut self.scene, &mut env, cp, tp, code, arg, &mut self.rand)
    }

    /// An item's skill (`ccItemSkillRequest` and its variants, as
    /// `ccUseItemRequest` made it) started: the caster's normal attack
    /// ended, the run added with the item's amount, `effSkillStart`.
    pub fn item_skill(&mut self, cp: usize, tp: usize, k: &piney_battle::item::ItemSkill) {
        let t = &self.data.t;
        if self.skills.borrow_mut().add_item(t, &self.scene, cp, tp, k).is_some() {
            stage::skill_shows(&mut self.shows, cp, Some(tp), k.sid, k.request.as_ref());
        }
    }

    /// `ccSkillRequest(plw, target, sid)` from the menus and the action
    /// button (stype 0).
    pub fn skill_request(&mut self, me: usize, target: Option<usize>, sid: i32) {
        let t = &self.data.t;
        let (_, req) = self.skills.borrow_mut().request(t, &mut self.scene, me, target, sid, 0, &mut self.rand);
        stage::skill_shows(&mut self.shows, me, target, sid, req.as_ref());
    }

    /// `EntryAffect(on, by, kind, 0, 0, 0)` as the menus and the events
    /// call it, with what it leads to.
    pub fn entry_affect(&mut self, on: usize, by: Option<usize>, kind: i16, hits: &mut Hits, bounds: MapBounds) {
        self.entry_affect_with(on, by, kind, [0; 3], hits, bounds);
    }

    /// `EntryAffect` from outside the frame (the menus, the scripts), with
    /// the lines it raised run on the members at once: the game's runs
    /// `Influence`, and so `ccAI::Greeting`, inside the call. Left queued
    /// while a menu slept the tasks, a member's greeting (14) landed after
    /// the menu's talk-off (0), and its `talkFlag` stayed set: the member
    /// stood, silent, deaf to orders, for good.
    #[allow(clippy::too_many_arguments)]
    pub fn affect_now(
        &mut self,
        on: usize,
        by: Option<usize>,
        kind: i16,
        p: [i16; 3],
        hits: &mut Hits,
        bounds: MapBounds,
        save: &mut piney_data::save::SaveData,
        ents: &[usize],
    ) {
        self.entry_affect_with(on, by, kind, p, hits, bounds);
        let d = self.data.clone();
        let party = self.party;
        self.drain_chats(&d.t, save, &party, ents);
    }

    /// `EntryAffect(on, by, kind, p0, p1, p2)`, with what it leads to.
    pub fn entry_affect_with(
        &mut self,
        on: usize,
        by: Option<usize>,
        kind: i16,
        p: [i16; 3],
        hits: &mut Hits,
        bounds: MapBounds,
    ) {
        let t = &self.data.t;
        let party = self.party;
        let checks = self.skill_checks();
        let chk = |c: usize| checks.get(c).copied().unwrap_or(0);
        let aff = AffectCtx { party: &party, menu: true, skill_check: &chk, boss: None, volume: self.data.volume };
        let mut ev = Events::new();
        affect::entry_affect(t, &mut self.scene, &aff, on, by, kind, p, &mut self.rand, &mut ev);
        for e in ev {
            self.consequence(e, hits, bounds, false);
        }
    }

    /// `DataDrainMenu`'s rules after its movie (gcmn 0x00533010-0x00533470)
    /// on Kite's target `tp` with skill `sid`: the infection into `save`,
    /// the drops, and `ccDeleteCmnd` on a target that is not a boss
    /// ([`piney_battle::drain::drain`]). A boss's `EntryAffect(21)` is the
    /// menu's own request, before the movie.
    pub fn data_drain(
        &mut self,
        tp: usize,
        sid: i32,
        save: &mut piney_data::save::SaveData,
    ) -> piney_battle::drain::Drain {
        let Some(k) = self.kite else { return piney_battle::drain::Drain::default() };
        let d = piney_battle::drain::drain(&self.data.t, &self.scene, save, k, tp, sid, &mut self.rand);
        if d.remove_target {
            fellow::delete_cmnd(&mut self.scene, tp);
        }
        d
    }

    /// Data Drain's side effect on the party (step 10,
    /// [`piney_battle::drain::side_effect`]), and what it starts
    /// ([`Show::DrainSide`], in the game's order).
    pub fn drain_side_effect(&mut self, save: &mut piney_data::save::SaveData) -> piney_battle::drain::SideEffect {
        let Some(k) = self.kite else { return piney_battle::drain::SideEffect::default() };
        let party = self.party;
        let s = match self.force_side_draws.take() {
            Some(first) => {
                let mut rng = Primed { first: first.into_iter(), rest: &mut self.rand };
                piney_battle::drain::side_effect(&self.data.t, &mut self.scene, &party, k, save, &mut rng)
            }
            None => piney_battle::drain::side_effect(&self.data.t, &mut self.scene, &party, k, save, &mut self.rand),
        };
        self.shows.extend(s.starts.iter().map(|&st| Show::DrainSide(st)));
        s
    }

    /// `ccChar::LevelDown(plw)` (gcmn 0x0056d640): Data Drain's lost level,
    /// then its LEVEL DOWN over Kite ([`Show::DrainLevelDown`]).
    pub fn kite_level_down(&mut self) {
        if let Some(k) = self.kite {
            piney_battle::chara::level_down(&self.data.t, &mut self.scene.chars[k]);
            self.shows.push(Show::DrainLevelDown(k));
        }
    }

    /// Kite's position (`plw +0x40`); the origin before he is built.
    pub fn kite_pos(&self) -> V4 {
        self.kite.and_then(|k| self.scene.chars.get(k)).map_or([0, 0, 0, 0x3f80_0000], |c| c.pos)
    }

    /// Kite's act for the draw, and his heading.
    pub fn kite_dirc(&self) -> V4 {
        self.kite.and_then(|k| self.crew.spc.get(&k)).map_or([0; 4], |s| s.dirc)
    }

    /// The characters on the command lists that are the enemies of the
    /// entry control (for the HUD and the targeting): scene indices.
    pub fn enemies(&self) -> Vec<usize> {
        let mut v = self.ctrl.list(entry::Kind::Enemy);
        v.extend(self.boss_char().filter(|&b| self.scene.listed(b)));
        v
    }

    /// Whether `who` is dead (`condition.dead`).
    pub fn dead(&self, who: usize) -> bool {
        self.scene.chars.get(who).is_some_and(|c| c.cond[cond::DEAD] != 0)
    }

    /// The number of `who`'s live condition effect (`ccChar` +0x2c), if any.
    pub fn condition_effect(&self, who: usize) -> Option<i32> {
        self.cond_fx.get(&who).copied()
    }

    /// The animation slot helper for the draw.
    pub fn slot() -> AnmSlot {
        AnmSlot::Main
    }
}

/// What an affect on Kite or a member does beyond its record, where the
/// game does it inside `Influence` / `ccFellow::Influence` (gcmn 0x0059ac50,
/// 0x0041bdb0): `ccSkillRequest(ch, 0, 0)`, the body in or out of the
/// collision, the bus's "down" (`ccAISysMsgSendP(0x1000c, ...)`) and "up"
/// (`ccAISysMsgDeleteDelay(0x1000d, id, id)`), the talk ended.
struct RuleParts<'a> {
    t: &'a Tables,
    scene: &'a mut Scene,
    crew: &'a mut Crew,
    skills: &'a RefCell<Skills>,
    hits: &'a mut Hits,
    kite: Option<usize>,
    rand: &'a mut Rand,
}

impl RuleParts<'_> {
    /// Whether `e` is one of these ([`RuleParts::apply`]).
    fn takes(e: &Event) -> bool {
        matches!(
            e,
            Event::CancelAttack(Who::Char(_))
                | Event::HitEnable(Who::Char(_))
                | Event::HitDisable(Who::Char(_))
                | Event::SysMsgDown { on: Who::Char(_), .. }
                | Event::SysMsgUp { .. }
                | Event::TalkOff(Who::Char(_))
        )
    }

    /// `e` carried out; false when it is not one of these.
    fn apply(&mut self, e: &Event) -> bool {
        match *e {
            Event::CancelAttack(Who::Char(c)) => {
                self.skills.borrow_mut().request(self.t, self.scene, c, None, 0, 0, self.rand);
            }
            Event::HitEnable(Who::Char(c)) | Event::HitDisable(Who::Char(c)) => {
                let on = matches!(e, Event::HitEnable(_));
                let s = self.crew.spc.entry(c).or_default();
                let mut b = spc::to_body(c, self.kite, &s.body_hit, false);
                self.hits.set_hit_sw(&mut b, on);
                s.body_hit.sw = b.sw;
            }
            Event::SysMsgDown { on: Who::Char(c), id } => {
                self.crew.send(0x1000c, id as u16, 0xffff, 0, 30, -1, Some(c));
            }
            Event::SysMsgUp { id, .. } => {
                self.crew.sys.delete_delay(0x1000d, id as u16, id as u16);
            }
            Event::TalkOff(Who::Char(c)) => {
                if let Some(a) = self.crew.ais.get_mut(&c) {
                    a.talk_flag = false;
                }
            }
            _ => return false,
        }
        true
    }
}

/// Given draws first, then the generator's ([`Combat::force_side_draws`]).
struct Primed<'a> {
    first: std::array::IntoIter<i32, 2>,
    rest: &'a mut Rand,
}

impl Rng for Primed<'_> {
    fn rand(&mut self) -> i32 {
        self.first.next().unwrap_or_else(|| self.rest.rand())
    }
}
