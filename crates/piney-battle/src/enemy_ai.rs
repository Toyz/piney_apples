//! The enemies' decisions (`enemy.cpp`, gcmn 0x00432840-0x00437bd0): whom a
//! `ccEnemy` fights, which attack it uses, when it chases, backs off, goes
//! home, flinches and dies, and what it is when it spawns or is drained. An
//! enemy is a [`Char`] of the [`Scene`] plus an [`Enemy`] (its `ccEntryObj`
//! and `ccEnemy` members). They draw from `ccRand` ([`Genrand`], main
//! 0x001d9a10), which the races' movement and the field share, so the runtime
//! owns one; newlib's `rand` is a separate argument. The frame, the acts and
//! the RNGs are in docs/engine/battle.md ("Enemy AI", "RNG").

use piney_data::field::ee;
use piney_data::libm;
use piney_data::save::SaveData;
use piney_data::volume::Volume;

use crate::chara::{self, Body, Char, Env};
use crate::event::Event;
use crate::param::{EnemyTable, F_ONE, SkillParam, ThinkParam, cond, ty};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::skill::{self, bits};
use crate::tables::Tables;
use crate::world::CharHit;

const PI: u32 = 0x4049_0fdb;
const NEG_PI: u32 = 0xc049_0fdb;
const TWO_PI: u32 = 0x40c9_0fdb;
const HALF_PI: u32 = 0x3fc9_0fdb;
const F_HALF: u32 = 0x3f00_0000;
const F_TENTH: u32 = 0x3dcc_cccd;
const F_099: u32 = 0x3f7d_70a4;
const F_1_3: u32 = 0x3fa6_6666;
const F_1_5: u32 = 0x3fc0_0000;
const F_MINUS_ONE: u32 = 0xbf80_0000;
const F_2000: u32 = 0x44fa_0000;

/// The acts (`ccEnemy.actNum`), as `defaultThink` and the races'
/// `action()` use them.
pub mod act {
    /// Stand, facing the target if there is one; look for one.
    pub const WAIT: i16 = 0;
    /// Close to the target (within `atkRangeA`): circle it, wait to strike.
    pub const CLOSE: i16 = 1;
    /// Wander with no target.
    pub const WANDER: i16 = 2;
    /// Go for the target.
    pub const CHASE: i16 = 3;
    /// Go back to the spawn point (beyond `area`).
    pub const RETURN: i16 = 4;
    /// Slow down to a stop, then wait.
    pub const STOP: i16 = 5;
    /// An attack, art or spell (`atk_num` is the skill slot).
    pub const ATTACK: i16 = 6;
    /// Flinching from a hit.
    pub const DAMAGE: i16 = 7;
    /// Dying (`condition.dead` 2), 90 frames.
    pub const DYING: i16 = 8;
    /// Dead, fading (`condition.dead` 3 after 30 frames).
    pub const DEAD: i16 = 9;
}

pub use crate::rand::Genrand;

/// `ccRandF(x)` (main 0x001d9a90): `x * r / 2^31` in double precision
/// (`fptodp`, `litodp`, `dpdiv`, `dpmul`, `dptofp`), `r` the next
/// `ccRand()`: between `-x` and `x`. The product of a float and a 31-bit
/// integer is exact in a double, and `dptofp` rounds to nearest with a
/// sticky bit, so IEEE double arithmetic gives the game's bits.
pub fn rand_f(cc: &mut dyn Rng, x: u32) -> u32 {
    let r = f64::from(cc.rand());
    ((f64::from(f32::from_bits(x)) * (r / 2_147_483_648.0)) as f32).to_bits()
}

/// An angle wrapped into -pi..pi as the game does: once each way.
fn wrap_pi(mut v: u32) -> u32 {
    if !ee::le(v, PI) {
        v = ee::sub(v, TWO_PI);
    }
    if ee::lt(v, NEG_PI) {
        v = ee::add(v, TWO_PI);
    }
    v
}

/// `ccRandF(x, y)` (main 0x001d9b20): [`rand_f`] of `x` held to `-y..y`.
pub fn rand_f2(cc: &mut dyn Rng, x: u32, y: u32) -> u32 {
    let v = rand_f(cc, x);
    if ee::lt(v, 0) {
        let ny = y ^ 0x8000_0000;
        if ee::lt(v, ny) { ny } else { v }
    } else if ee::le(v, y) {
        v
    } else {
        y
    }
}

/// `ccSetRadDisperse(&v, x)` (main 0x001da4a0): `v + ccRandF(x)`, wrapped.
pub fn rad_disperse(v: u32, x: u32, cc: &mut dyn Rng) -> u32 {
    wrap_pi(ee::add(v, rand_f(cc, x)))
}

/// `ccGetDirc(a, b)` (main 0x001d9ce0): the heading from `a` to `b` on the
/// ground, `atan2f(dy, dx) + pi/2`, wrapped.
pub fn get_dirc(a: [u32; 4], b: [u32; 4]) -> u32 {
    let dx = ee::sub(b[0], a[0]);
    let dy = ee::sub(b[1], a[1]);
    wrap_pi(ee::add(HALF_PI, libm::atan2f(dy, dx)))
}

/// `ccGetDist(a, b)` (main 0x001d9dd0): the distance on the ground,
/// `sqrtf` of `|b - a|^2` with the third lane zeroed (`sceVu0SubVector`,
/// `sceVu0InnerProduct`), as Infection and Mutation have it.
pub fn get_dist(a: [u32; 4], b: [u32; 4]) -> u32 {
    get_dist_on(Volume::Inf, a, b)
}

/// [`get_dist`] as the volume's main has it: Outbreak's and Quarantine's
/// take the FPU's `sqrt.s` (truncating) inline for newlib's `sqrtf` (OUT
/// main 0x001e6ec0, QUA 0x001ee540).
pub fn get_dist_on(volume: Volume, a: [u32; 4], b: [u32; 4]) -> u32 {
    crate::geom::plane_dist(volume, b, a)
}

/// Where a position is in the player's frame and back: `ccTransPosW2P`
/// (gcmn 0x0059b940, `ccPlayer::W2PPos`) and `ccTransPosP2W` (0x0059b980,
/// `ccPlayer::P2WPos`), the runtime's.
pub trait Frame {
    fn w2p(&self, v: [u32; 4]) -> [u32; 4];
    fn p2w(&self, v: [u32; 4]) -> [u32; 4];
}

/// The frame when the player stands on the ground: both are copies.
#[derive(Clone, Copy, Debug, Default)]
pub struct Identity;

impl Frame for Identity {
    fn w2p(&self, v: [u32; 4]) -> [u32; 4] {
        v
    }
    fn p2w(&self, v: [u32; 4]) -> [u32; 4] {
        v
    }
}

pub static IDENTITY: Identity = Identity;

/// Which `ccSkillParam` an enemy's pointer names: a row of its own table
/// entry (`atc0`, `atc1`, `ski0`, `ski1`, numbered as the skill slots 0, 1,
/// 4, 5) or `skillTbl[sid]` (`ccGetSkillParam`, the spells of slots 2, 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillRef {
    Row(u8),
    Table(i32),
}

impl SkillRef {
    pub fn get(self, t: &Tables, ene_id: i32) -> Option<&SkillParam> {
        match self {
            SkillRef::Table(sid) => t.skill(sid),
            SkillRef::Row(k) => {
                let row = usize::try_from(ene_id).ok().and_then(|i| t.enemies.get(i))?;
                match k {
                    0 => Some(&row.atc[0]),
                    1 => Some(&row.atc[1]),
                    4 => Some(&row.ski[0]),
                    5 => Some(&row.ski[1]),
                    _ => None,
                }
            }
        }
    }
}

/// `ccEnemySkill` (0x10 bytes): one usable attack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemySkill {
    /// Usable now: in range (or a target found) and SP enough.
    pub atk_flag: bool,
    /// Used before anything else (a heal or a revival).
    pub int_flag: bool,
    pub range_flag: bool,
    pub cost_flag: bool,
    /// The slot, 0-5: `atc0`, `atc1`, `mag0`, `mag1`, `ski0`, `ski1`.
    pub atk_id: i8,
    /// The spell's `skillTbl` id, -1 for the table's own attacks.
    pub ski_id: i16,
    /// Its share of the random choice (float bits), from [`set_skill_rate`].
    pub percentage: u32,
    pub ski_param: Option<SkillRef>,
    pub ski_target: Option<usize>,
}

impl Default for EnemySkill {
    /// As `clearSkillList` leaves it.
    fn default() -> Self {
        EnemySkill {
            atk_flag: false,
            int_flag: false,
            range_flag: false,
            cost_flag: false,
            atk_id: -1,
            ski_id: -1,
            percentage: 0,
            ski_param: None,
            ski_target: None,
        }
    }
}

/// `ccEntryParam` (0x60 bytes): where and what to spawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EntryParam {
    pub pos: [u32; 4],
    pub dirc: [u32; 4],
    /// 0 an enemy, 1 a gimmick, 2 an NPC.
    pub ty: i32,
    /// The `enemyTbl` row.
    pub id: i32,
    pub area: i32,
    pub area_num: i32,
    pub floor: i32,
    pub block: i32,
    pub x: i32,
    pub y: i32,
    pub ent_root: i32,
    pub land: i32,
    pub param: [i32; 4],
}

/// An enemy's own state: the `ccEnemy` members (and the `ccEntryObj` and
/// `ccChar` ones the rules use that [`Char`] does not keep). Float members
/// are bits; character pointers are scene indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Enemy {
    /// `ccChar.dirc`.
    pub dirc: [u32; 4],
    /// `ccChar.affectType` and `affectParam[0]`: the last affect taken
    /// (1 damage, 3 poison, 20 revival...), set with `affect_flag` by the
    /// affect handling.
    pub affect_type: i16,
    pub affect_param0: i16,
    // ccEntryObj
    pub obj_flag: bool,
    pub init_flag: bool,
    /// Frozen (far from the player): `begin_frame` only clears it.
    pub freeze_flag: bool,
    /// An affect was taken since the last `interrupt_think`.
    pub affect_flag: bool,
    pub disp_sw: bool,
    pub dest_flag: bool,
    /// Off the command lists (`deleteCmnd` ran).
    pub cmnd_flag: bool,
    /// The middle boss's second model.
    pub ccs2_flag: bool,
    pub fade_flag: i16,
    pub fade_cnt: i16,
    pub ent: EntryParam,
    // ccEnemy
    pub ene_id: i32,
    /// `ccEntryRaceTbl` group and the row's place in it.
    pub race: i32,
    pub race_id: i32,
    pub target_flag: bool,
    /// The attack or flinch animation ended (the race's `action()`).
    pub action_flag: bool,
    /// A Data Drain landed.
    pub drain_flag: bool,
    pub move_flag: u8,
    /// Charmed or confused.
    pub mad_flag: bool,
    /// A middle boss with a protect gauge.
    pub virus_flag: bool,
    pub opt_flag0: bool,
    pub opt_flag1: bool,
    pub ene_type: i16,
    pub ene_rand: i16,
    pub ene_smoke: i16,
    pub ene_part: i16,
    pub act_num: i16,
    pub act_cnt: i16,
    /// The skill slot of the attack running.
    pub atk_num: i16,
    pub atk_cnt: i16,
    /// Frames before the next attack.
    pub atk_dellay: i16,
    /// Frames of the drained form's grace.
    pub drain_cnt: i16,
    /// Frames spent flinching lately.
    pub damage_cnt: i16,
    pub target: Option<usize>,
    pub target_dirc: u32,
    /// Ground distance to the target less both widths.
    pub target_dist: u32,
    pub crisis_rate: u32,
    pub base_dirc: u32,
    /// Ground distance to the spawn point less the width.
    pub base_dist: u32,
    /// The spawn point (world).
    pub bpos: [u32; 4],
    pub mdirc: [u32; 4],
    pub max_spd: u32,
    pub rad_cnt: u32,
    pub hit_cnt: i32,
    pub hit_spd: u32,
    pub speed: u32,
    pub yoffs: u32,
    pub zoffs: u32,
    pub anm_num: i16,
    pub anm_num_old: i16,
    pub frame_num: u16,
    /// The animation ended (`animEnemy`).
    pub anm_flag: i16,
    /// HP / maxHP.
    pub life_rate: u32,
    pub skill_id: i32,
    pub skill_param: Option<SkillRef>,
    pub skill_target: Option<usize>,
    pub skill_num: i32,
    pub skill_list: [EnemySkill; 6],
    // The motion layer's members (spawning, movement, animation).
    /// `ccEntryObj.bodyHit` (+0x160): the body in the collision.
    pub hit: CharHit,
    /// `ccEntryObj.anmTbl` (+0x1b0): whose animation names (`enemyTbl`
    /// row + 1, 0 for none), indexed by `anm_num`.
    pub anm_tbl: u32,
    // spawn: the entry control's members and the race constructors'
    // (entctrl.cpp, the enemy*.cpp constructors; see crate::entry and
    // crate::races).
    /// `ccEntryObj.plDist` (+0xe4): the ground distance to Kite, from
    /// `ccEntryObj::routine`.
    pub pl_dist: u32,
    /// `ccEntryObj.plDirc` (+0xe8): the heading from Kite to the enemy.
    pub pl_dirc: u32,
    /// `ccEntryObj.alpha` (+0xec): 128 from the constructor.
    pub alpha: i32,
    /// `ccEntryObj.grotDeg`, `grotSpd` (+0xf4, +0xf6): an event's turn
    /// toward `grot_deg` (16-bit units) at `grot_spd`.
    pub grot_deg: i16,
    pub grot_spd: i16,
    /// `ccChar.transparency`, `setTransparency` (+0x88, +0x8c): the fade
    /// `routine` runs.
    pub transparency: u32,
    pub set_transparency: u32,
    /// `ccEntryObj.destFunc` (+0x1c8): the gcmn address of the race's
    /// clean-up hook (`ccDestEnemyG` ...), 0 for none.
    pub dest_func: u32,
    /// The race's `wc` (`ccEnemyWeaponCtrl`) and `dc` (`ccEnemyDustCtrl`):
    /// the info block and its count, none without.
    pub weapon: Option<(crate::blocks::InfoRef, i32)>,
    pub dust: Option<(crate::blocks::InfoRef, i32)>,
    /// `ccEnemyG`'s members (+0x340 on), zero for the other races.
    pub gold: crate::races::Gold,
    /// `ccEnemyH` types 3 and 4 (`brt[2]`, +0x348): the two fire breaths
    /// (`ccEnemyBreath`) were made.
    pub breath: bool,
    /// `ccEnemyC`'s `+0x340`: where a type 3 (a scorpion) spins to on a
    /// stop.
    pub spin: u32,
    /// `ccEnemyL`'s `+0x390`: four `ccRandS` its constructor draws (for
    /// the type 3's colours).
    pub l_rand: [i16; 4],
    /// The model's world matrix (`ccChar.anm`'s `ccCoord` +0): what the
    /// last draw of the enemy left (`_SetLWMatrix` from the matrix
    /// `dispEnemy` set), zero before one.
    pub anm_lw: [[u32; 4]; 4],
}

impl Default for Enemy {
    /// As the constructors leave it (`ccChar`, `ccEntryObj`,
    /// `ccEnemy::ccEnemy` 0x00432a90): display on, fading in over 30
    /// frames, full life, an empty skill list.
    fn default() -> Self {
        Enemy {
            dirc: [0; 4],
            affect_type: 0,
            affect_param0: 0,
            obj_flag: false,
            init_flag: false,
            freeze_flag: false,
            affect_flag: false,
            disp_sw: true,
            dest_flag: false,
            cmnd_flag: false,
            ccs2_flag: false,
            fade_flag: 1,
            fade_cnt: 30,
            ent: EntryParam::default(),
            ene_id: 0,
            race: 0,
            race_id: 0,
            target_flag: false,
            action_flag: false,
            drain_flag: false,
            move_flag: 0,
            mad_flag: false,
            virus_flag: false,
            opt_flag0: false,
            opt_flag1: false,
            ene_type: 0,
            ene_rand: 0,
            ene_smoke: 0,
            ene_part: 0,
            act_num: 0,
            act_cnt: 0,
            atk_num: 0,
            atk_cnt: 0,
            atk_dellay: 0,
            drain_cnt: 0,
            damage_cnt: 0,
            target: None,
            target_dirc: 0,
            target_dist: 0,
            crisis_rate: 0,
            base_dirc: 0,
            base_dist: 0,
            bpos: [0; 4],
            mdirc: [0; 4],
            max_spd: 0,
            rad_cnt: 0,
            hit_cnt: 0,
            hit_spd: F_ONE,
            speed: 0,
            yoffs: 0,
            zoffs: 0,
            anm_num: 0,
            anm_num_old: 0,
            frame_num: 0,
            anm_flag: 0,
            life_rate: F_ONE,
            skill_id: 0,
            skill_param: None,
            skill_target: None,
            skill_num: 0,
            skill_list: [EnemySkill::default(); 6],
            hit: CharHit::default(),
            anm_tbl: 0,
            // spawn
            pl_dist: 0,
            pl_dirc: 0,
            alpha: 128,
            grot_deg: 0,
            grot_spd: 0,
            transparency: 0,
            set_transparency: 0,
            dest_func: 0,
            weapon: None,
            dust: None,
            gold: crate::races::Gold::default(),
            breath: false,
            spin: 0,
            l_rand: [0; 4],
            anm_lw: [[0; 4]; 4],
        }
    }
}

impl Enemy {
    /// What `ccEnemyInfluence` (0x00432840) leaves when an affect lands:
    /// the flag `interrupt_think` reads, the kind and first parameter
    /// (`EntryAffect`'s), and the drain flag when `affectEnemy` reported a
    /// Data Drain (affect 13).
    pub fn note_affect(&mut self, kind: i16, p0: i16, drained: bool) {
        self.affect_flag = true;
        self.affect_type = kind;
        self.affect_param0 = p0;
        if drained {
            self.drain_flag = true;
        }
    }

    /// The same for an `EntryAffect(kind, p0)` as it landed
    /// ([`crate::enemy_motion::affect_lands`]): the kind and first
    /// parameter once stored, `affectFlag` (and a Data Drain's
    /// `drainFlag`, affect 13) once the enemy's influence ran.
    pub fn note_landed(&mut self, l: crate::enemy_motion::Landed, kind: i16, p0: i16) {
        if l.stored {
            self.affect_type = kind;
            self.affect_param0 = p0;
        }
        if l.influenced {
            self.affect_flag = true;
            if kind == 13 {
                self.drain_flag = true;
            }
        }
    }
}

/// The globals the enemies' rules read.
#[derive(Clone, Copy)]
pub struct World<'a> {
    /// `eventMng->puppetShow` (`ccEvent` +0x78c): a scripted scene runs.
    pub puppet_show: bool,
    /// `pgRideFlag` (main 0x00378cdc).
    pub ride: bool,
    /// `ccCheckActiveEnemy()` (gcmn 0x0042df10): the enemies on
    /// `g_entCtrl`'s list with `objFlag` set and `freezeFlag` clear.
    pub active_enemies: i32,
    /// `ccPartyManager.memberChar[0]`: the player.
    pub player: Option<usize>,
    pub frame: &'a dyn Frame,
}

impl Default for World<'static> {
    fn default() -> Self {
        World { puppet_show: false, ride: false, active_enemies: 0, player: None, frame: &IDENTITY }
    }
}

/// What the rules ask of the rest of the game, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Out {
    /// An event of the crate's rules (`CalcReal`'s timers ...), `Who::Me`
    /// the enemy.
    Rule(Event),
    /// `targetPtr->EntryAffect(this, 5, 0, 0, 0)`: a special attack holds
    /// its target while it runs in range (the call is made even with no
    /// target, on a null `targetPtr`); from Mutation on `skillTarget`'s.
    Hold { target: Option<usize> },
    /// `ccChar::ClearConditionEffect`: the condition effect is deleted
    /// (`conditionNum` -1 is done).
    ClearConditionEffect,
    /// `ccDeleteCmnd`: off the command lists (done on the scene); the
    /// menu's target moves on if it was this (`ccChangeCmndTarget`).
    DeleteCmnd,
    /// `ccExpDistributor(level)`: the kill's experience
    /// ([`crate::exp::exp_distributor`]).
    Exp { level: i16 },
    /// The enemy book's kill ([`record_kill`]).
    KillRecord { ene_id: i32 },
    /// `effSkillStart(this, skillParam, 0, 0)`.
    SkillStart(Option<SkillRef>),
    /// `g_entCtrl->entryEnemyObject(this)`: the corpse is taken away.
    Remove,
    /// `entryDrainEnemy`: spawn the drained form ([`after_drain_spawn`]).
    DrainSpawn(DrainSpawn),
    /// `game.inBattleDist = d` (`ccGame` +0x60): a gold goblin shaking off
    /// a hold keeps the party out of battle mode (-2) for a frame, then
    /// puts the distance back (2200).
    InBattleDist(u32),
}

/// The drained form `entryDrainEnemy` spawns with
/// `g_entCtrl->entryObject(&ep, 1)`: the entry copied, at the enemy's
/// position and heading, type 0, row [`drain_id`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrainSpawn {
    pub ent: EntryParam,
    /// The new form's `drainCnt`: 60 frames if its row starts a race
    /// group, else 30.
    pub drain_cnt: i16,
}

/// What `startSkill` starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillUse {
    /// A plain attack (slots 0, 1): nothing.
    None,
    /// A spell (slots 2, 3): `ccSkillRequest(this, skillTarget, skillId)`,
    /// that is [`crate::skill::request`] with stype 0.
    Request { target: Option<usize>, sid: i32 },
    /// A special attack (slots 4, 5): its SP paid if there is enough
    /// (`ccSkillCostConsume`, done).
    Cost { paid: bool },
}

/// The enemies' rules over a scene: the tables, the characters, every
/// enemy's state by scene index, the globals, both generators, and the
/// calls the rules make, in order.
pub struct Ai<'a> {
    pub t: &'a Tables,
    pub scene: &'a mut Scene,
    pub foes: &'a mut [Option<Enemy>],
    pub world: World<'a>,
    /// newlib's `rand()`.
    pub rand: &'a mut dyn Rng,
    /// `ccRand()`.
    pub cc: &'a mut dyn Rng,
    pub out: Vec<Out>,
}

/// One enemy's rules at work: `Ai` with the enemy's own state taken out.
struct Cx<'b, 'a> {
    t: &'a Tables,
    scene: &'b mut Scene,
    foes: &'b [Option<Enemy>],
    world: World<'a>,
    rand: &'b mut dyn Rng,
    cc: &'b mut dyn Rng,
    out: &'b mut Vec<Out>,
    me: usize,
}

impl<'a> Ai<'a> {
    fn with<R>(&mut self, me: usize, f: impl FnOnce(&mut Cx<'_, 'a>, &mut Enemy) -> R) -> R {
        let mut e = self.foes[me].take().expect("an enemy's state");
        let mut cx = Cx {
            t: self.t,
            scene: &mut *self.scene,
            foes: &*self.foes,
            world: self.world,
            rand: &mut *self.rand,
            cc: &mut *self.cc,
            out: &mut self.out,
            me,
        };
        let r = f(&mut cx, &mut e);
        self.foes[me] = Some(e);
        r
    }

    /// The start of `ccEnemy::main` (0x00432cd0), up to `think()`.
    pub fn begin_frame(&mut self, me: usize, env: &Env) -> Begin {
        self.with(me, |cx, e| begin_frame(cx, e, env))
    }

    /// `ccEnemy::defaultThink` (0x00434ac0).
    pub fn default_think(&mut self, me: usize) {
        self.with(me, default_think)
    }

    /// `ccEnemyG::thinkGold` (gcmn 0x00447b20): a gold goblin's think.
    pub fn think_gold(&mut self, me: usize) {
        self.with(me, think_gold)
    }

    /// `ccEnemy::interruptThink` (0x004353f0).
    pub fn interrupt_think(&mut self, me: usize) {
        self.with(me, interrupt_think)
    }

    /// `ccEnemy::main` after `action()`: `condition.hold = 0`.
    pub fn end_action(&mut self, me: usize) {
        self.scene.chars[me].cond[cond::HOLD] = 0;
    }

    /// `ccEnemy::routineEnemy` (0x00433990).
    pub fn routine_enemy(&mut self, me: usize, env: &Env) {
        self.with(me, |cx, e| routine_enemy(cx, e, env))
    }

    /// `ccEnemy::checkEnemy` (0x00433710).
    pub fn check_enemy(&mut self, me: usize) {
        self.with(me, check_enemy)
    }

    /// `ccEnemy::checkCrisisRate` (0x00433610).
    pub fn check_crisis_rate(&mut self, me: usize) {
        self.with(me, |cx, e| check_crisis_rate(cx, e))
    }

    /// `ccEnemy::setAct(act)` (0x00433470).
    pub fn set_act(&mut self, me: usize, a: i16) {
        self.with(me, |cx, e| set_act(cx, e, a))
    }

    /// `ccEnemy::selectAttack` (0x00436940).
    pub fn select_attack(&mut self, me: usize) -> bool {
        self.with(me, select_attack)
    }

    /// `ccEnemy::checkSkillList` (0x004360d0).
    pub fn check_skill_list(&mut self, me: usize) -> i32 {
        self.with(me, check_skill_list)
    }

    /// `ccEnemy::selectSkillTarget(sid, spp)` (0x004366a0).
    pub fn select_skill_target(&mut self, me: usize, sid: i32, sk: &SkillParam) -> Option<usize> {
        self.with(me, |cx, e| select_skill_target(cx, e, sid, sk))
    }

    /// `ccEnemy::selectTarget(lttype)` (0x00436cb0).
    pub fn select_target_by(&mut self, me: usize, lttype: i32) -> bool {
        self.with(me, |cx, e| select_target_by(cx, e, lttype))
    }

    /// `ccEnemy::selectTarget()` (0x004371b0).
    pub fn select_target(&mut self, me: usize) -> bool {
        self.with(me, select_target)
    }

    /// `ccEnemy::setInterval` (0x00436c40).
    pub fn set_interval(&mut self, me: usize) {
        self.with(me, |cx, e| set_interval(cx, e))
    }

    /// `ccEnemy::startSkill` (0x00435ac0), from the animation's note 0x8003.
    pub fn start_skill(&mut self, me: usize) -> SkillUse {
        self.with(me, start_skill)
    }

    /// `ccEnemy::affectSkill` (0x00435b80), from the animation's note 0x8005.
    pub fn affect_skill(&mut self, me: usize) -> Option<(usize, SkillRef)> {
        self.with(me, |cx, e| affect_skill(cx, e))
    }

    /// `ccEnemy::clearConditionEnemy` (0x004335d0).
    pub fn clear_condition_enemy(&mut self, me: usize) {
        clear_condition_enemy(&mut self.scene.chars[me], &mut self.out);
    }

    /// The decisions of `animEnemy` (0x00434560) when `moveFlag` is 2:
    /// `actNum` set to 0 for `selectTarget(0)`, the target's distance read
    /// for the animation's speed, `actNum` put back, `selectTarget()`.
    /// Returns the distance the animation uses.
    pub fn anim_retarget(&mut self, me: usize) -> u32 {
        self.with(me, |cx, e| {
            let a = e.act_num;
            e.act_num = 0;
            select_target_by(cx, e, 0);
            let d = e.target_dist;
            e.act_num = a;
            select_target(cx, e);
            d
        })
    }
}

/// What the start of `ccEnemy::main` decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Begin {
    /// Go on: think, interrupt, act, move.
    Continue,
    /// `condition.dead` was 3: [`Out::Remove`]; main returns 1.
    Removed,
    /// Frozen: conditions cleared; main returns `ret` (1 if off the lists
    /// or dying, else 0 after resetting to wait).
    Frozen { ret: u8 },
    /// Drained this frame: [`Out::DrainSpawn`]; main returns 1.
    Drained,
}

fn row<'t>(t: &'t Tables, e: &Enemy) -> &'t EnemyTable {
    &t.enemies[e.ene_id as usize]
}

/// The row's `ccEnemyThinkParam` (`tParam`) as the game reads it: a gold
/// goblin of volume 3 or 4 had `checkGold` write 50000 into its row's
/// `area`, `territory` and `viewRange` when it was made (and every goblin
/// of a row has that row's volume).
fn think(t: &Tables, e: &Enemy) -> ThinkParam {
    let mut p = row(t, e).think;
    if e.gold.volume == 3 || e.gold.volume == 4 {
        p.area = GOLD_RANGE;
        p.territory = GOLD_RANGE;
        p.view_range = GOLD_RANGE;
    }
    p
}

/// 50000, `checkGold`'s ranges.
const GOLD_RANGE: u32 = 0x4743_5000;

fn is_zero(v: u32) -> bool {
    ee::cmp(v, 0) == std::cmp::Ordering::Equal
}

fn width(ch: &Char) -> u32 {
    ch.base().width
}

/// `ccCheckObjectSize(ch)` (gcmn 0x0042e120) for an enemy: its row's
/// `ccEntry.esize` (1, 3 or 4). Entries of type 1 and 2 read `gimmickTbl`
/// and `npcTbl` instead; an enemy's `entParam.type` is always 0 (the spawn
/// lists and `entryDrainEnemy` make only type 0), so those give 0 here.
pub fn object_size(t: &Tables, ent: &EntryParam) -> i32 {
    if ent.ty != 0 {
        return 0;
    }
    usize::try_from(ent.id).ok().and_then(|i| t.enemies.get(i)).map_or(0, |r| r.esize)
}

/// `ccGetEnemyRace(id, &race, &raceId)` (gcmn 0x0042e2b0): the race group
/// of `ccEntryRaceTbl` an `enemyTbl` row is in and its place there; None
/// past the last group (the game leaves both unset).
pub fn enemy_race(t: &Tables, id: i32) -> Option<(i32, i32)> {
    let mut total = 0;
    for (race, &n) in t.race_num.iter().enumerate() {
        let first = total;
        total += n;
        if id < total {
            return Some((race as i32, id - first));
        }
    }
    None
}

/// `ccCheckMiddleBoss(id)` (gcmn 0x0042e270): a row of type 0x40 keeps
/// its base form's row in `base.gold`; -1 for the rest.
pub fn middle_boss(t: &Tables, id: i32) -> i32 {
    let r = &t.enemies[id as usize].param.base;
    if r.ty == crate::param::ty::MIDDLE_BOSS { r.gold } else { -1 }
}

/// `ccGetDrainId(id)` (gcmn 0x0042e3c0): the row a drained enemy becomes:
/// a middle boss's base form, else the first row of its race group, and
/// rows 203-206 become 203.
pub fn drain_id(t: &Tables, id: i32) -> i32 {
    let m = middle_boss(t, id);
    if m >= 0 {
        return m;
    }
    let mut total = 0;
    let mut race = t.race_num.len();
    for (r, &n) in t.race_num.iter().enumerate() {
        total += n;
        if id < total {
            race = r;
            break;
        }
    }
    let first: i32 = t.race_num[..race].iter().sum();
    if (203..=206).contains(&id) { 203 } else { first }
}

/// Which of the six skill slots have an animation: `ccEntry.anm` of the
/// row (a `char[][30]` table), slot `i` usable when its name is not empty.
/// None for a row without a table (a middle boss's has its base form's,
/// given as the area loads: [`crate::tables::Tables`]).
pub fn anim_slots(row: &EnemyTable) -> Option<[bool; 6]> {
    let names = row.anm?;
    Some(std::array::from_fn(|i| names.get(i).is_some_and(|s| !s.is_empty())))
}

/// `ccEnemy::clearSkillList` (0x00435c40).
pub fn clear_skill_list(e: &mut Enemy) {
    e.skill_list = [EnemySkill::default(); 6];
    e.skill_num = 0;
}

/// `ccEnemy::initSkillList` (0x00435cf0): the usable slots, in slot order:
/// `atc0`, `atc1`, `ski0`, `ski1` when their row's type is not -1, the
/// spells `mag0`, `mag1` when their id is not 0; each only if the
/// animation table has that slot (`anm`). Then [`set_skill_rate`].
pub fn init_skill_list(t: &Tables, e: &mut Enemy, anm: [bool; 6]) {
    e.skill_num = 0;
    let r = row(t, e);
    for (i, &has_anm) in anm.iter().enumerate() {
        let (sid, sp, ok) = match i {
            0 | 1 => (-1, SkillRef::Row(i as u8), r.atc[i].ty != -1),
            4 | 5 => (-1, SkillRef::Row(i as u8), r.ski[i - 4].ty != -1),
            _ => {
                let m = r.mag[i - 2];
                (m, SkillRef::Table(m), m != 0)
            }
        };
        if has_anm && ok {
            let s = &mut e.skill_list[e.skill_num as usize];
            s.atk_id = i as i8;
            s.ski_id = sid as i16;
            s.ski_param = Some(sp);
            e.skill_num += 1;
        }
    }
    set_skill_rate(e);
}

/// `ccEnemy::setSkillRate` (0x00435ef0): each slot's share, `1/n` weighted
/// by its spell: healing (150-155) and revival (180) 0, regeneration and
/// haste (175-177) x1.5, the conditions (156-162) x1.3, normalised.
pub fn set_skill_rate(e: &mut Enemy) {
    let n = e.skill_num.clamp(0, 6) as usize;
    let mut r = [0u32; 6];
    let mut total = 0;
    for (s, ri) in e.skill_list[..n].iter().zip(r.iter_mut()) {
        let mut v = ee::div(F_ONE, ee::from_int(e.skill_num));
        match s.ski_id {
            150..=155 | 180 => v = ee::mul(v, 0),
            175..=177 => v = ee::mul(v, F_1_5),
            156..=162 => v = ee::mul(v, F_1_3),
            _ => {}
        }
        *ri = v;
        total = ee::add(total, v);
    }
    for (s, ri) in e.skill_list[..n].iter_mut().zip(r) {
        s.percentage = ee::div(ri, total);
    }
}

/// `ccEnemy::clearTarget` (0x00436c80): no target; face on.
pub fn clear_target(e: &mut Enemy) {
    e.target_flag = false;
    e.mdirc[2] = e.dirc[2];
    e.target_dirc = e.dirc[2];
}

/// `ccChar::ClearCondition(ccEnemyParam *)` (gcmn 0x0056cbf0) and
/// `ClearConditionEffect` (0x00570180), the pair `clearConditionEnemy`
/// calls: every condition but `dead` cleared, speed back to 1, buffs and
/// debuffs gone, the protect gauge emptied, `conditionNum` -1.
pub fn clear_condition_enemy(ch: &mut Char, out: &mut Vec<Out>) {
    clear_condition(ch);
    if let Body::Foe(f) = &mut ch.body {
        f.temp = [0; 16];
        f.time = [0; 16];
        f.pp = 0;
        f.pp_count = 0;
    }
    ch.condition_num = -1;
    out.push(Out::ClearConditionEffect);
}

/// `ccChar::ClearCondition()` (gcmn 0x0056cb50): all but `dead`.
fn clear_condition(ch: &mut Char) {
    for i in 1..16 {
        ch.cond[i] = 0;
    }
    ch.cond.speed_value = F_ONE;
}

/// `ccCheckTargetTypeId(type, id)` (gcmn 0x005199e0): some character on
/// the lists of the type's sides with a type bit in common and that id.
pub fn check_target_type_id(scene: &Scene, tyb: i32, id: i32) -> bool {
    let has = |list: &[usize]| {
        list.iter().any(|&c| {
            let ch = &scene.chars[c];
            tyb & ch.ty() != 0 && i32::from(ch.id()) == id
        })
    };
    (tyb & 7 != 0 && has(&scene.pc_list))
        || (tyb & ty::FOE != 0 && has(&scene.ene_list))
        || (tyb & !0xe7 != 0 && has(&scene.obj_list))
}

/// `ccSearchNearPerson(me, type, flag, range)` (gcmn 0x00519af0): the
/// nearest character on the ground (from `posP`) closer than `range`: the
/// player (type bit 1, from `ccPartyManager`), then the party's list
/// (bits 0x6), the foes' (0xe0), the objects' (the rest), those sharing a
/// type bit and not `me`; `flag` 0 the living, 2 the dying (`dead` 2), 6
/// anyone. Later ones win ties. With nobody found, a charmed or confused
/// `me` finds itself.
pub fn search_near_person(
    scene: &Scene,
    world: &World,
    volume: Volume,
    me: usize,
    tyb: i32,
    flag: i32,
    range: u32,
) -> Option<usize> {
    let mp = scene.chars[me].pos_p;
    let mut best = None;
    let mut bd = F_MINUS_ONE;
    let flag_ok = |c: usize| {
        let d = scene.chars[c].cond[cond::DEAD];
        flag == 6 || (flag == 0 && d == 0) || (flag == 2 && d == 2)
    };
    let consider = |c: usize, best: &mut Option<usize>, bd: &mut u32| {
        if !flag_ok(c) {
            return;
        }
        let d = get_dist_on(volume, mp, scene.chars[c].pos_p);
        if ee::lt(d, range) && (!ee::lt(*bd, d) || ee::lt(*bd, 0)) {
            *best = Some(c);
            *bd = d;
        }
    };
    if tyb & 1 != 0
        && let Some(p) = world.player
    {
        let pc = &scene.chars[p];
        if check_target_type_id(scene, pc.ty(), i32::from(pc.id())) {
            consider(p, &mut best, &mut bd);
        }
    }
    let lists: [(bool, &Vec<usize>); 3] =
        [(tyb & 6 != 0, &scene.pc_list), (tyb & ty::FOE != 0, &scene.ene_list), (tyb & !0xe7 != 0, &scene.obj_list)];
    for (on, list) in lists {
        if !on {
            continue;
        }
        for &c in list.iter() {
            if tyb & scene.chars[c].ty() != 0 && c != me {
                consider(c, &mut best, &mut bd);
            }
        }
    }
    if best.is_none() {
        let c = &scene.chars[me].cond;
        if c[cond::CHARM] != 0 || c[cond::CONFUSION] != 0 {
            best = Some(me);
        }
    }
    best
}

/// `ccEnemy::checkSkillRange(ch, range)` (0x004365a0): `ch` is alive and
/// is `me`, or stands closer than `range` on the ground.
pub fn check_skill_range(scene: &Scene, volume: Volume, me: usize, c: usize, range: u32) -> bool {
    let ch = &scene.chars[c];
    if ch.cond[cond::DEAD] != 0 {
        return false;
    }
    if c == me {
        return true;
    }
    ee::lt(get_dist_on(volume, scene.chars[me].pos_p, ch.pos_p), range)
}

impl Cx<'_, '_> {
    fn ch(&self) -> &Char {
        &self.scene.chars[self.me]
    }

    /// `ccEnemy.lifeRate` of a character on the foes' list (a boss has
    /// padding at that offset: 0 here).
    fn life_rate(&self, e: &Enemy, c: usize) -> u32 {
        if c == self.me {
            return e.life_rate;
        }
        self.foes.get(c).and_then(|f| f.as_ref()).map_or(0, |f| f.life_rate)
    }

    fn search(&self, tyb: i32, flag: i32, range: u32) -> Option<usize> {
        search_near_person(self.scene, &self.world, self.t.volume, self.me, tyb, flag, range)
    }
}

/// `ccEnemy::checkCrisisRate` (0x00433610): how pressed the enemy is,
/// `1 - targetDist / atkRangeA` held to 0.1..1 (0 in a puppet show); not
/// changed when `atkRangeA` is 0.
fn check_crisis_rate(cx: &Cx, e: &mut Enemy) {
    let a = think(cx.t, e).atk_range_a;
    if is_zero(a) {
        return;
    }
    if cx.world.puppet_show {
        e.crisis_rate = 0;
        return;
    }
    let mut v = ee::div(e.target_dist, a);
    if !ee::le(v, F_ONE) {
        v = F_ONE;
    }
    if ee::lt(v, 0) {
        v = 0;
    }
    v = ee::sub(F_ONE, v);
    if ee::lt(v, F_TENTH) {
        v = F_TENTH;
    }
    if !ee::le(v, F_ONE) {
        v = F_099;
    }
    e.crisis_rate = v;
}

/// `ccEnemy::checkEnemy` (0x00433710): the position through the player's
/// frame, the heading and distance home, and the target's: kept while it
/// is on the lists and alive (a revival spell's target may be dying);
/// itself when charmed or confused, at distance 0 with its heading
/// scattered every few frames by its size.
fn check_enemy(cx: &mut Cx, e: &mut Enemy) {
    let me = cx.me;
    let f = cx.world.frame;
    let ch = &mut cx.scene.chars[me];
    ch.pos_p = f.w2p(ch.pos);
    ch.pos = f.p2w(ch.pos_p);
    let pp = ch.pos_p;
    let bp = f.w2p(e.bpos);
    e.base_dirc = get_dirc(pp, bp);
    e.base_dist = ee::sub(get_dist_on(cx.t.volume, pp, bp), width(ch));
    if !e.target_flag {
        return;
    }
    let t = match e.skill_target {
        Some(s) if e.act_num == act::ATTACK && s != me => Some(s),
        _ => e.target,
    };
    let ok = t.is_some_and(|t| cx.scene.listed(t) && (e.skill_id == 180 || cx.scene.chars[t].cond[cond::DEAD] == 0));
    let Some(t) = t.filter(|_| ok) else {
        clear_target(e);
        return;
    };
    if t == me {
        if e.act_cnt & 4 == 0 {
            let spread = match object_size(cx.t, &e.ent) {
                1 => Some(0x3ea0_d97c),
                3 => Some(0x3f49_0fdb),
                4 => Some(HALF_PI),
                _ => None,
            };
            if let Some(x) = spread {
                e.target_dirc = rad_disperse(e.target_dirc, x, cx.cc);
            }
        }
        e.target_dist = 0;
        e.crisis_rate = F_HALF;
        return;
    }
    let tc = &cx.scene.chars[t];
    e.target_dirc = get_dirc(pp, tc.pos_p);
    e.target_dist = get_dist_on(cx.t.volume, pp, tc.pos_p);
    e.target_dist = ee::sub(e.target_dist, ee::add(width(&cx.scene.chars[me]), width(tc)));
    check_crisis_rate(cx, e);
}

/// `ccEnemy::routineEnemy` (0x00433990): `CalcReal(0)` (the frame's
/// condition timers), the life rate, the top speed (`maxSpd * hitSpd *
/// speedValue`), the attack delay counting down (twice as fast in a
/// puppet show) and the flinch count (not while flinching).
fn routine_enemy(cx: &mut Cx, e: &mut Enemy, env: &Env) {
    let mut ev = Vec::new();
    chara::calc_real(cx.t, &mut cx.scene.chars[cx.me], 0, env, &mut ev);
    cx.out.extend(ev.into_iter().map(Out::Rule));
    let ch = cx.ch();
    e.life_rate = ee::div(ee::from_int(i32::from(ch.hp)), ee::from_int(i32::from(ch.max_hp)));
    e.max_spd = ee::mul(ch.cond.speed_value, ee::mul(think(cx.t, e).max_spd, e.hit_spd));
    if e.atk_dellay > 0 {
        e.atk_dellay -= 1;
    }
    if cx.world.puppet_show && e.atk_dellay > 0 {
        e.atk_dellay -= 1;
    }
    if e.act_num != act::DAMAGE && e.damage_cnt > 0 {
        e.damage_cnt -= 1;
    }
}

/// `ccEnemy::setInterval` (0x00436c40): the delay before the next attack,
/// the row's `attackDelay` plus 8 frames for every active enemy.
fn set_interval(cx: &Cx, e: &mut Enemy) {
    let d = think(cx.t, e).attack_delay.wrapping_add(cx.world.active_enemies.wrapping_mul(8));
    e.atk_dellay = d as i16;
}

/// `ccEnemy::setAct(act)` (0x00433470): the next act, `act_cnt` restarted
/// when it changes. Closing in (1) becomes an attack when the race's row
/// is past its first (`raceId` above 0), no delay remains and an attack is
/// ready; in a puppet show it becomes an attack or a wait. An attack (6)
/// needs one ready, else it waits within `atkRangeB` and chases beyond.
fn set_act(cx: &mut Cx, e: &mut Enemy, a: i16) {
    let old = e.act_num;
    e.act_num = match a {
        act::CLOSE => {
            if cx.world.puppet_show {
                if e.atk_dellay == 0 && select_attack(cx, e) { act::ATTACK } else { act::WAIT }
            } else if e.race_id > 0 && e.atk_dellay == 0 && select_attack(cx, e) {
                act::ATTACK
            } else {
                act::CLOSE
            }
        }
        act::ATTACK => {
            if select_attack(cx, e) {
                act::ATTACK
            } else if ee::le(e.target_dist, think(cx.t, e).atk_range_b) {
                act::WAIT
            } else {
                act::CHASE
            }
        }
        a => a,
    };
    if e.act_num != old {
        e.act_cnt = 0;
    }
}

/// `ccEnemy::selectSkillTarget(sid, spp)` (0x004366a0): whom a spell
/// slot would be cast on, within the spell's `triggerRange`: a heal, the
/// foe (itself included) with the lowest life rate under half; a buff,
/// the first foe without it; a debuff, the party member without it whom
/// it would take on (`ccCheckConditionSkillSuccess`, which draws `rand()`)
/// with the lowest tolerance (spirit + body, 2000 for one who cannot die).
/// None for the table's own attacks (sid -1).
fn select_skill_target(cx: &mut Cx, e: &Enemy, sid: i32, sk: &SkillParam) -> Option<usize> {
    if sid == -1 {
        return None;
    }
    let range = sk.trigger_range;
    let me = cx.me;
    let mut best = None;
    let mut rate = F_MINUS_ONE;
    let first = |rate: u32| ee::cmp(F_MINUS_ONE, rate) == std::cmp::Ordering::Equal;
    if sk.ty & bits::HEAL != 0 {
        for c in cx.scene.ene_list.clone() {
            if check_skill_range(cx.scene, cx.t.volume, me, c, range) {
                let lr = cx.life_rate(e, c);
                if ee::lt(lr, F_HALF) && (!ee::le(rate, lr) || first(rate)) {
                    best = Some(c);
                    rate = lr;
                }
            }
        }
        return best;
    }
    if sk.ty & bits::BUFF != 0 {
        return cx.scene.ene_list.clone().into_iter().find(|&c| {
            check_skill_range(cx.scene, cx.t.volume, me, c, range)
                && skill::target_condition_by_skill(&cx.scene.chars[c], sid) == 0
        });
    }
    if sk.ty & bits::DEBUFF != 0 {
        for c in cx.scene.pc_list.clone() {
            if !check_skill_range(cx.scene, cx.t.volume, me, c, range)
                || skill::target_condition_by_skill(&cx.scene.chars[c], sid) != 0
                || !skill::condition_success(&cx.scene.chars[c], sid, cx.scene.listed(c), cx.rand)
            {
                continue;
            }
            let ch = &cx.scene.chars[c];
            let v = if ch.no_death {
                F_2000
            } else {
                let re = ch.real();
                ee::from_int(i32::from(re[15]) + i32::from(re[14]))
            };
            if !ee::le(rate, v) || first(rate) {
                best = Some(c);
                rate = v;
            }
        }
        return best;
    }
    None
}

/// `ccEnemy::checkSkillList` (0x004360d0): which slots can be used now.
/// Revival (180) needs a dying foe in range (`ccSearchNearPerson(me, 0x60,
/// 2, triggerRange)`) and, like a heal, jumps the queue; heals, buffs and
/// debuffs need a target ([`Ai::select_skill_target`]); anything else
/// needs the target within `triggerRange`. All need the SP. Returns how
/// many are usable.
fn check_skill_list(cx: &mut Cx, e: &mut Enemy) -> i32 {
    let mut n = 0;
    let (mut sid, mut sp): (i32, Option<SkillRef>) = (-1, None);
    for i in 0..e.skill_num.clamp(0, 6) as usize {
        let s = e.skill_list[i];
        match s.atk_id {
            0 | 1 | 4 | 5 => {
                sid = -1;
                sp = s.ski_param;
            }
            2 | 3 => {
                sid = i32::from(s.ski_id);
                sp = Some(SkillRef::Table(sid));
            }
            _ => {}
        }
        let Some(sk) = sp.and_then(|r| r.get(cx.t, e.ene_id)).cloned() else {
            e.skill_list[i].atk_flag = false;
            continue;
        };
        let (int_flag, tgt, in_range) = if sid == 180 {
            let t = cx.search(ty::ENEMY, 2, sk.trigger_range);
            (true, t, t.is_some())
        } else if sk.ty & bits::HEAL != 0 {
            let t = select_skill_target(cx, e, sid, &sk);
            (true, t, t.is_some())
        } else if sk.ty & (bits::DEBUFF | bits::BUFF) != 0 {
            let t = select_skill_target(cx, e, sid, &sk);
            (false, t, t.is_some())
        } else {
            (false, e.target, skill::range_check(&sk, e.target_dist))
        };
        let cost = skill::cost_check(cx.ch(), &sk);
        let s = &mut e.skill_list[i];
        s.int_flag = int_flag;
        s.range_flag = in_range;
        s.cost_flag = cost;
        if in_range && cost {
            s.atk_flag = true;
            s.ski_id = sid as i16;
            s.ski_param = sp;
            s.ski_target = tgt;
            n += 1;
        } else {
            s.atk_flag = false;
        }
    }
    n
}

fn take_skill(e: &mut Enemy, i: usize) {
    let s = e.skill_list[i];
    e.skill_id = i32::from(s.ski_id);
    e.skill_param = s.ski_param;
    e.skill_target = s.ski_target;
    e.anm_num = i16::from(s.atk_id);
    e.atk_num = i16::from(s.atk_id);
}

/// `ccEnemy::selectAttack` (0x00436940): the attack to use, into
/// `skill_id`, `skill_param`, `skill_target` and `atk_num` (= `anm_num`).
/// None while paralysed or asleep. Charmed or confused, the first slot if
/// it is usable, on the current target. Else a usable heal or revival
/// first, else one of the usable slots at random by their shares
/// (`0.5 + ccRandF(0.5)` against the running sum).
fn select_attack(cx: &mut Cx, e: &mut Enemy) -> bool {
    let c = &cx.ch().cond;
    if c[cond::PARALYSIS] != 0 || c[cond::SLEEP] != 0 {
        return false;
    }
    let n = check_skill_list(cx, e);
    let c = &cx.ch().cond;
    if c[cond::CHARM] != 0 || c[cond::CONFUSION] != 0 {
        let s = e.skill_list[0];
        if !s.atk_flag {
            return false;
        }
        e.skill_id = -1;
        e.skill_param = s.ski_param;
        e.skill_target = e.target;
        e.anm_num = i16::from(s.atk_id);
        e.atk_num = i16::from(s.atk_id);
        return true;
    }
    if n == 0 {
        return false;
    }
    let num = e.skill_num.clamp(0, 6) as usize;
    if let Some(i) = (0..num).find(|&i| e.skill_list[i].int_flag && e.skill_list[i].atk_flag) {
        take_skill(e, i);
        return true;
    }
    let mut total = 0;
    for s in &e.skill_list[..num] {
        if s.atk_flag {
            total = ee::add(total, s.percentage);
        }
    }
    let w: Vec<u32> =
        e.skill_list[..num].iter().map(|s| if s.atk_flag { ee::div(s.percentage, total) } else { 0 }).collect();
    let r = ee::add(F_HALF, rand_f(cx.cc, F_HALF));
    let mut sum = 0;
    for (i, &wi) in w.iter().enumerate() {
        if is_zero(wi) {
            continue;
        }
        sum = ee::add(sum, wi);
        if ee::lt(r, sum) {
            if !e.skill_list[i].atk_flag {
                return false;
            }
            take_skill(e, i);
            return true;
        }
    }
    false
}

/// `ccEnemy::selectTarget(lttype)` (0x00436cb0): whom to fight. Charmed the
/// nearest other foe, confused the nearest of either side; else among the
/// first six of the party in `viewRange`, type 0 the nearest, 1 the lowest
/// HP, 2 the lowest level, failing that the nearest player. Other types
/// compare an uninitialised register, taken as 0 here (the value in every
/// call `main` makes). The rules are in docs/engine/battle.md ("Enemy AI").
fn select_target_by(cx: &mut Cx, e: &mut Enemy, lttype: i32) -> bool {
    let me = cx.me;
    let pcs = cx.scene.pc_list.clone();
    let alive = pcs.iter().filter(|&&c| cx.scene.chars[c].cond[cond::DEAD] == 0).count();
    let n = pcs.len().min(6);
    if (alive == 0 || n == 0) && e.act_num == act::WAIT {
        e.target_flag = false;
        e.mdirc[2] = e.dirc[2];
        e.target_dirc = e.dirc[2];
        clear_condition_enemy(&mut cx.scene.chars[me], cx.out);
        if !cx.world.ride {
            e.act_cnt = 1;
        }
        return false;
    }
    let view = think(cx.t, e).view_range;
    let c = cx.ch().cond;
    if c[cond::CHARM] != 0
        && let Some(p) = cx.search(ty::ENEMY, 0, view)
    {
        e.target_flag = true;
        e.target = Some(p);
        return true;
    }
    if c[cond::CONFUSION] != 0
        && let Some(p) = cx.search(0x63, 0, view)
    {
        e.target_flag = true;
        e.target = Some(p);
        return true;
    }
    let mp = cx.ch().pos;
    // (distance, 0 when not a candidate; HP; level)
    let rows: Vec<(u32, i16, i16)> = pcs[..n]
        .iter()
        .map(|&p| {
            let pc = &cx.scene.chars[p];
            let d = get_dist_on(cx.t.volume, mp, pc.pos);
            if pc.cond[cond::DEAD] == 0 && ee::lt(d, view) {
                let d = if pc.no_death && n != 1 { 0 } else { d };
                (d, pc.hp, pc.level())
            } else {
                (0, pc.hp, 0)
            }
        })
        .collect();
    let mut idx = None;
    if lttype == 0 {
        let mut best = view;
        for (i, &(d, _, _)) in rows.iter().enumerate() {
            if !is_zero(d) && ee::lt(d, best) {
                best = d;
                idx = Some(i);
            }
        }
    } else {
        let mut bv: i16 = 32767;
        let mut v: i16 = 0;
        for (i, &(d, hp, lv)) in rows.iter().enumerate() {
            if is_zero(d) {
                continue;
            }
            match lttype {
                1 => v = hp,
                2 => v = lv,
                _ => {}
            }
            if v < bv {
                bv = v;
                idx = Some(i);
            }
        }
    }
    match idx {
        None => {
            if let Some(p) = cx.search(3, 0, view) {
                e.target_flag = true;
                e.target = Some(p);
            }
        }
        Some(i) => {
            e.target_flag = true;
            e.target = Some(pcs[i]);
        }
    }
    if e.target_flag {
        check_enemy(cx, e);
        true
    } else {
        false
    }
}

/// `ccEnemy::selectTarget()` (0x004371b0): by the row's `targetType`.
fn select_target(cx: &mut Cx, e: &mut Enemy) -> bool {
    let tt = think(cx.t, e).target_type;
    select_target_by(cx, e, tt)
}

/// `ccEnemy::startSkill` (0x00435ac0): the attack in slot `atk_num`
/// starts: nothing for a plain attack; else `effSkillStart`, then a spell
/// is requested on `skill_target` or a special attack's SP paid.
fn start_skill(cx: &mut Cx, e: &mut Enemy) -> SkillUse {
    if e.atk_num == 0 || e.atk_num == 1 {
        return SkillUse::None;
    }
    cx.out.push(Out::SkillStart(e.skill_param));
    match e.atk_num {
        2 | 3 => SkillUse::Request { target: e.skill_target, sid: e.skill_id },
        4 | 5 => {
            let sk = e.skill_param.and_then(|r| r.get(cx.t, e.ene_id)).cloned().unwrap_or_default();
            SkillUse::Cost { paid: skill::cost_consume(&mut cx.scene.chars[cx.me], &sk) }
        }
        _ => SkillUse::None,
    }
}

/// `ccEnemy::affectSkill` (0x00435b80): the attack lands if its target is
/// still on the lists and alive (a revival's may be dying) and the enemy's
/// target is within the skill's `triggerRange`:
/// `ccSkillDamage(this, skillTarget, skillParam, 0)` (gcmn 0x00573dc0),
/// that is [`crate::damage::skill_damage`] with sid 0 and no attribute
/// critical. Returns whom and with what.
fn affect_skill(cx: &Cx, e: &Enemy) -> Option<(usize, SkillRef)> {
    let t = e.skill_target?;
    if !cx.scene.listed(t) || !(e.skill_id == 180 || cx.scene.chars[t].cond[cond::DEAD] == 0) {
        return None;
    }
    let r = e.skill_param?;
    let sk = r.get(cx.t, e.ene_id)?;
    skill::range_check(sk, e.target_dist).then_some((t, r))
}

/// `ccEntryObj::deleteCmnd(flg)` (gcmn 0x0042fe40): once, off the command
/// list of its type (`ccDeleteCmnd`, gcmn 0x00519700).
fn delete_cmnd(cx: &mut Cx, e: &mut Enemy, flg: bool) {
    if e.cmnd_flag {
        return;
    }
    delete_from_lists(cx.scene, cx.me);
    cx.out.push(Out::DeleteCmnd);
    e.cmnd_flag = flg;
}

/// `ccDeleteCmnd(ch)` (gcmn 0x00519700) on the scene: if listed, removed
/// from the list its type names (party 0x7, foes 0xe0, objects).
pub fn delete_from_lists(scene: &mut Scene, c: usize) {
    if !scene.listed(c) {
        return;
    }
    let tyb = scene.chars[c].ty();
    let list = if tyb & 7 != 0 {
        &mut scene.pc_list
    } else if tyb & ty::FOE != 0 {
        &mut scene.ene_list
    } else {
        &mut scene.obj_list
    };
    if let Some(k) = list.iter().position(|&x| x == c) {
        list.remove(k);
    }
}

/// `saveData.enemyKillCount` (+0x68b7, `char[313]`) and `enemyKillArea`
/// (+0x69f0, `short[313][4]`).
pub const SAVE_ENEMY_KILL_COUNT: usize = 0x68b7;
pub const SAVE_ENEMY_KILL_AREA: usize = 0x69f0;

/// The enemy book's record of a kill or drain (in `ccEnemy::main` and
/// `interruptThink`): the row's count up by one (a signed byte, held at
/// 99), and where: `game->server` and `worldman`'s area words A, B, C.
pub fn record_kill(save: &mut SaveData, ene_id: i32, server: i32, area: [i32; 3]) {
    let at = SAVE_ENEMY_KILL_COUNT + ene_id as usize;
    let n = (save.u8(at) as i8).wrapping_add(1);
    save.set_u8(at, if n >= 100 { 99 } else { n as u8 });
    let a = SAVE_ENEMY_KILL_AREA + 8 * ene_id as usize;
    save.set_i16(a, server as i16);
    for (k, v) in area.iter().enumerate() {
        save.set_i16(a + 2 + 2 * k, *v as i16);
    }
}

/// `ccEnemy::entryDrainEnemy` (0x004359e0): the enemy fades out at once
/// and its drained form (row [`drain_id`]) is spawned where it stands.
fn entry_drain_enemy(cx: &Cx, e: &mut Enemy) -> DrainSpawn {
    let did = drain_id(cx.t, e.ene_id);
    let mut ent = e.ent;
    ent.pos = cx.ch().pos;
    ent.dirc = e.dirc;
    ent.ty = 0;
    ent.id = did;
    e.fade_flag = 0;
    DrainSpawn { ent, drain_cnt: if cx.t.check_drain_enemy(did) { 60 } else { 30 } }
}

/// What `entryDrainEnemy` does to the new form once `entryObject` has
/// made it: fully opaque (the runtime's), `condition.dead` 1 and the grace
/// frames, during which `begin_frame` holds it and then revives it
/// (`effAfterDrain(obj, -1)` is the runtime's).
pub fn after_drain_spawn(e: &mut Enemy, ch: &mut Char, spawn: &DrainSpawn) {
    ch.cond[cond::DEAD] = 1;
    e.drain_cnt = spawn.drain_cnt;
}

/// The start of `ccEnemy::main` (0x00432cd0): a drained form's grace
/// counts down, holding it, and revives it (`dead` 0) at the end; a corpse
/// whose `dead` is 3 is taken away; a frozen enemy has its conditions
/// cleared and, unless off the lists or dying, drops its target and waits,
/// stopped; else `routineEnemy`, `checkEnemy`, and a Data Drain taken last
/// frame ends it: the book's record, conditions cleared, off the lists, the
/// drained form spawned (no experience).
fn begin_frame(cx: &mut Cx, e: &mut Enemy, env: &Env) -> Begin {
    let me = cx.me;
    if e.drain_cnt != 0 {
        e.drain_cnt = e.drain_cnt.wrapping_sub(1);
        if e.drain_cnt != 0 {
            cx.scene.chars[me].cond[cond::HOLD] = 1;
        } else {
            cx.scene.chars[me].cond[cond::DEAD] = 0;
        }
    }
    if cx.ch().cond[cond::DEAD] == 3 {
        cx.out.push(Out::Remove);
        return Begin::Removed;
    }
    if e.freeze_flag {
        clear_condition_enemy(&mut cx.scene.chars[me], cx.out);
        if e.cmnd_flag {
            return Begin::Frozen { ret: 1 };
        }
        let d = cx.ch().cond[cond::DEAD];
        if d == 2 || d == 3 {
            return Begin::Frozen { ret: 1 };
        }
        clear_target(e);
        set_act(cx, e, act::WAIT);
        e.speed = 0;
        e.anm_num = 6;
        e.anm_num_old = 0;
        return Begin::Frozen { ret: 0 };
    }
    routine_enemy(cx, e, env);
    check_enemy(cx, e);
    if e.drain_flag {
        cx.out.push(Out::KillRecord { ene_id: e.ene_id });
        clear_condition_enemy(&mut cx.scene.chars[me], cx.out);
        delete_cmnd(cx, e, true);
        let s = entry_drain_enemy(cx, e);
        cx.out.push(Out::DrainSpawn(s));
        return Begin::Drained;
    }
    Begin::Continue
}

/// `ccEnemyG::setActGold(act)` (gcmn 0x00447a60): act 1 (flee) is 0
/// in a puppet show; act 6 (attack) only when `selectAttack` finds one,
/// else 0; `actCnt` 0 on a change.
fn set_act_gold(cx: &mut Cx, e: &mut Enemy, a: i16) {
    let old = e.act_num;
    e.act_num = match a {
        1 => {
            if cx.world.puppet_show {
                0
            } else {
                1
            }
        }
        act::ATTACK => {
            if select_attack(cx, e) {
                act::ATTACK
            } else {
                0
            }
        }
        _ => a,
    };
    if e.act_num != old {
        e.act_cnt = 0;
    }
}

/// `ccEnemyG::thinkGold` (gcmn 0x00447b20), a gold goblin's think in place of
/// `defaultThink`: conditions wear off faster by `goldVolume`, holds are
/// shaken off (`goldDisHold`), and the acts flee rather than close in. The
/// table is in docs/engine/battle.md ("The gold goblins").
fn think_gold(cx: &mut Cx, e: &mut Enemy) {
    let me = cx.me;
    let tp = think(cx.t, e);
    select_target_by(cx, e, 0);
    if !ee::le(e.target_dist, tp.atk_range_a) {
        select_target(cx, e);
    }
    const WEAR: [usize; 4] = [cond::SLEEP, cond::CONFUSION, cond::CHARM, cond::PARALYSIS];
    if WEAR.iter().any(|&k| cx.ch().cond[k] != 0) {
        // goldVolume is 1-4 (checkGold); the game leaves any other with
        // a register's value.
        let d: i16 = match e.gold.volume {
            4 => 4,
            3 => 3,
            2 => 2,
            _ => 0,
        };
        for k in WEAR {
            let v = &mut cx.scene.chars[me].cond[k];
            if *v > 0 {
                *v = v.wrapping_sub(d);
                if *v < 0 {
                    *v = 0;
                }
            }
        }
    }
    match e.gold.dis_hold {
        1 => {
            cx.out.push(Out::InBattleDist(0xc000_0000));
            delete_cmnd(cx, e, false);
            e.gold.dis_hold = 2;
        }
        2 => {
            cx.out.push(Out::InBattleDist(0x4509_8000));
            if !e.cmnd_flag {
                crate::enemy_motion::entry_cmnd(cx.scene, me);
            }
            e.gold.dis_hold = 3;
        }
        3 => e.gold.dis_hold = 0,
        _ => {}
    }
    let hold = cx.ch().cond[cond::HOLD];
    if e.gold.volume >= 2 {
        let c = cx.ch().cond;
        if e.affect_flag && hold != 0 && c[cond::PARALYSIS] == 0 && c[cond::SLEEP] == 0 && !cx.world.puppet_show {
            if e.affect_type == 3 || e.affect_type == 1 {
                e.gold.dis_hold = 1;
            }
        } else if e.gold.volume >= 3 {
            if hold != 0 {
                if !e.gold.pre_hold {
                    if e.gold.esc_cnt == 0 {
                        e.gold.esc_cnt = 1;
                    }
                    e.gold.esc_pos = cx.ch().pos;
                } else {
                    if e.gold.esc_cnt != 0 {
                        e.gold.esc_cnt += 1;
                    }
                    e.gold.esc_pos = cx.world.frame.p2w(cx.world.frame.w2p(e.gold.esc_pos));
                    let d = get_dist_on(cx.t.volume, cx.ch().pos, e.gold.esc_pos);
                    if !ee::le(d, 0x43fa_0000) && e.gold.dis_hold == 0 {
                        e.gold.dis_hold = 1;
                    }
                }
            } else if e.gold.esc_cnt != 0 {
                e.gold.esc_cnt -= 1;
            }
        } else if hold == 0 && e.gold.pre_hold && e.gold.dis_hold == 0 {
            e.gold.dis_hold = 1;
        }
    }
    e.gold.pre_hold = hold & 1 != 0;
    let puppet = cx.world.puppet_show;
    match e.act_num {
        0 => {
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if cx.ch().cond[cond::HOLD] != 0 {
                if !puppet {
                    set_act_gold(cx, e, 1);
                }
                return;
            }
            if e.target_flag {
                if !ee::le(e.target_dist, tp.atk_range_c) {
                    set_act_gold(cx, e, act::CHASE);
                } else if ee::lt(e.target_dist, tp.atk_range_a) || e.atk_dellay != 0 {
                    set_act_gold(cx, e, 1);
                } else {
                    set_act_gold(cx, e, act::ATTACK);
                }
                return;
            }
            if select_target(cx, e) {
                set_act_gold(cx, e, act::CHASE);
                return;
            }
            if e.act_cnt & 0x3f != 0 {
                return;
            }
            if cx.cc.rand() & 1 != 0 {
                set_act_gold(cx, e, act::WANDER);
            }
        }
        1 => {
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if e.target_flag && (ee::le(e.target_dist, tp.atk_range_b) || e.act_cnt < 17) {
                return;
            }
            set_act_gold(cx, e, act::STOP);
        }
        2 => {
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if e.act_cnt & 0xff == 0 && cx.cc.rand() & 3 != 0 {
                set_act_gold(cx, e, act::STOP);
            }
            if select_target(cx, e) {
                set_act_gold(cx, e, act::CHASE);
            }
        }
        3 => {
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if e.target_flag {
                if ee::lt(e.target_dist, tp.atk_range_b) {
                    set_act_gold(cx, e, act::WAIT);
                }
            } else {
                set_act_gold(cx, e, act::STOP);
            }
        }
        4 => {
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if ee::lt(e.base_dist, tp.territory) {
                set_act_gold(cx, e, act::STOP);
            }
        }
        5 => {
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if ee::lt(e.speed, F_ONE) {
                e.speed = 0;
                set_act_gold(cx, e, act::WAIT);
            }
            if e.act_cnt < 33 || cx.cc.rand() & 3 != 0 {
                return;
            }
            match cx.cc.rand() & 3 {
                0 => set_act_gold(cx, e, act::CHASE),
                1 => set_act_gold(cx, e, 1),
                _ => {
                    e.speed = 0;
                    set_act_gold(cx, e, act::WAIT);
                }
            }
        }
        6 => {
            if e.act_cnt == 0 {
                e.action_flag = false;
            }
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if e.action_flag || e.act_cnt >= 301 {
                set_act_gold(cx, e, act::WAIT);
                e.action_flag = false;
                set_interval(cx, e);
            }
            if !e.target_flag {
                set_act_gold(cx, e, act::WAIT);
            }
        }
        7 => {
            if e.act_cnt == 0 {
                e.action_flag = false;
            }
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if e.action_flag {
                set_act_gold(cx, e, act::WAIT);
                e.action_flag = false;
            }
        }
        8 | 9 => e.act_cnt = e.act_cnt.wrapping_add(1),
        _ => {}
    }
}

/// `ccEnemy::defaultThink` (0x00434ac0): every race's `think()`. Every fourth
/// frame of an act (not an attack) an enemy with a close range retargets;
/// then the act moves on by the row's think parameters (`atkRangeA`-`C`, the
/// attack delay, the 240-frame limits). The acts are in
/// docs/engine/battle.md ("Enemy AI").
fn default_think(cx: &mut Cx, e: &mut Enemy) {
    let me = cx.me;
    let tp = think(cx.t, e);
    if !is_zero(tp.atk_range_a) && e.act_cnt & 3 == 1 && e.act_num != act::ATTACK {
        select_target_by(cx, e, 0);
        if !ee::le(e.target_dist, tp.atk_range_a) {
            select_target(cx, e);
        }
    }
    let puppet = cx.world.puppet_show;
    match e.act_num {
        act::WAIT => {
            e.act_cnt = e.act_cnt.wrapping_add(1);
            let c = cx.ch().cond;
            if c[cond::PARALYSIS] != 0 || c[cond::SLEEP] != 0 {
                return;
            }
            let hold = c[cond::HOLD] != 0;
            if hold && puppet {
                return;
            }
            if hold {
                if e.damage_cnt == 0
                    || e.atk_dellay != 0
                    || !e.target_flag
                    || !ee::lt(e.target_dist, tp.atk_range_c)
                    || e.race_id == 0
                {
                    return;
                }
                set_act(cx, e, act::ATTACK);
                return;
            }
            if e.target_flag {
                if !ee::le(e.target_dist, tp.atk_range_c) {
                    if e.act_cnt >= 31 {
                        set_act(cx, e, act::CHASE);
                    }
                    return;
                }
                if !is_zero(tp.atk_range_a) && ee::lt(e.target_dist, tp.atk_range_a) {
                    set_act(cx, e, act::CLOSE);
                    return;
                }
                if e.atk_dellay != 0 {
                    return;
                }
                set_act(cx, e, act::ATTACK);
                return;
            }
            if select_target(cx, e) {
                set_act(cx, e, act::CHASE);
                return;
            }
            if e.act_cnt & 0x3f != 0x3f {
                return;
            }
            if cx.cc.rand() & 1 != 0 {
                set_act(cx, e, act::WANDER);
            }
        }
        act::CLOSE => {
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if !e.target_flag {
                set_act(cx, e, act::STOP);
                return;
            }
            if !ee::le(e.target_dist, tp.atk_range_b) && e.act_cnt >= 17 {
                if e.race_id == 0 || e.atk_dellay == 0 || ee::lt(e.life_rate, F_HALF) || cx.cc.rand() & 1 != 0 {
                    set_act(cx, e, act::ATTACK);
                } else {
                    e.atk_dellay >>= 1;
                    set_act(cx, e, act::STOP);
                }
            }
            if e.act_cnt >= 241 || (e.act_cnt >= 91 && e.hit_cnt >= 91) {
                set_act(cx, e, act::ATTACK);
                if e.act_num == act::ATTACK {
                    e.hit_cnt = 0;
                }
            }
            if e.atk_dellay != 0 || e.race_id == 0 || e.act_cnt < 17 {
                return;
            }
            if cx.ch().cond[cond::HOLD] == 0 {
                if select_attack(cx, e) {
                    set_act(cx, e, act::ATTACK);
                }
                return;
            }
            if e.damage_cnt == 0 {
                return;
            }
            set_act(cx, e, act::ATTACK);
        }
        act::WANDER => {
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if e.act_cnt & 0xff == 0 && cx.cc.rand() & 3 != 0 {
                set_act(cx, e, act::STOP);
            }
            if select_target(cx, e) {
                set_act(cx, e, act::CHASE);
            }
        }
        act::CHASE => {
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if !e.target_flag || ee::lt(e.target_dist, tp.atk_range_b) {
                set_act(cx, e, act::STOP);
            }
            if e.act_cnt >= 241 && cx.cc.rand() & 7 == 0 {
                set_act(cx, e, act::STOP);
            }
        }
        act::RETURN => {
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if ee::le(e.base_dist, tp.territory) {
                set_act(cx, e, act::STOP);
                return;
            }
            if e.act_cnt < 241 || e.act_cnt & 0x1f != 0 {
                return;
            }
            if cx.cc.rand() & 1 != 0 {
                set_act(cx, e, act::STOP);
            }
        }
        act::STOP => {
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if ee::lt(e.speed, F_ONE) {
                e.speed = 0;
                set_act(cx, e, act::WAIT);
            }
        }
        act::ATTACK => {
            if e.act_cnt == 0 {
                e.action_flag = false;
            }
            e.act_cnt = e.act_cnt.wrapping_add(1);
            if (e.atk_num == 4 || e.atk_num == 5)
                && let Some(sk) = e.skill_param.and_then(|r| r.get(cx.t, e.ene_id))
                && skill::range_check(sk, e.target_dist)
            {
                // From Mutation on the skill's own target is held.
                let target = if cx.t.volume == piney_data::volume::Volume::Inf { e.target } else { e.skill_target };
                cx.out.push(Out::Hold { target });
            }
            if e.action_flag || e.act_cnt >= 241 {
                set_act(cx, e, act::WAIT);
                e.action_flag = false;
                set_interval(cx, e);
            }
        }
        act::DAMAGE => {
            if e.act_cnt == 0 {
                e.action_flag = false;
            }
            e.act_cnt = e.act_cnt.wrapping_add(1);
            e.damage_cnt = e.damage_cnt.wrapping_add(1);
            if !e.action_flag {
                if e.act_cnt >= 241 {
                    set_act(cx, e, act::WAIT);
                }
                return;
            }
            let c = cx.scene.chars[me].cond;
            if c[cond::PARALYSIS] != 0 || c[cond::SLEEP] != 0 || (c[cond::HOLD] != 0 && puppet) {
                set_act(cx, e, act::WAIT);
            } else if !is_zero(tp.atk_range_a) {
                if e.race_id == 0 || ee::lt(e.life_rate, F_HALF) || cx.cc.rand() & 3 == 0 {
                    let k = cx.cc.rand() % 3;
                    select_target_by(cx, e, k);
                    set_act(cx, e, act::CLOSE);
                } else {
                    set_act(cx, e, act::WAIT);
                }
            } else if e.atk_dellay == 0 && select_target(cx, e) {
                set_act(cx, e, act::ATTACK);
            } else {
                set_act(cx, e, act::WAIT);
            }
            e.action_flag = false;
        }
        act::DYING | act::DEAD => e.act_cnt = e.act_cnt.wrapping_add(1),
        _ => {}
    }
}

/// `ccEnemy::interruptThink` (0x004353f0), after `think()` every frame: a
/// target out of `viewRange` or an enemy out of its `area` is let go, charm
/// and confusion (`mad_flag`) and holds, the affect just taken (a revival,
/// dying, a flinch), and dying itself: `dead` 2, after 90 frames dead (9) with
/// the experience ([`Out::Exp`]) and the book's record, `dead` 3 30 frames
/// later. The rules are in docs/engine/battle.md ("Enemy AI").
fn interrupt_think(cx: &mut Cx, e: &mut Enemy) {
    let me = cx.me;
    let tp = think(cx.t, e);
    if e.target_flag && e.act_num != act::ATTACK && !ee::le(e.target_dist, tp.view_range) {
        clear_target(e);
        e.speed = 0;
        set_act(cx, e, act::WAIT);
    }
    if !ee::le(e.base_dist, tp.area) && (0..=3).contains(&e.act_num) {
        clear_target(e);
        set_act(cx, e, act::RETURN);
    }
    let c = cx.ch().cond;
    if !e.mad_flag {
        if c[cond::CHARM] != 0 || c[cond::CONFUSION] != 0 {
            e.mad_flag = true;
        }
    } else if c[cond::CHARM] == 0 && c[cond::CONFUSION] == 0 {
        e.mad_flag = false;
        select_target(cx, e);
        // From Mutation on it also stops what it was doing.
        if cx.t.volume != piney_data::volume::Volume::Inf {
            set_act(cx, e, act::WAIT);
        }
    }
    if e.mad_flag {
        select_target(cx, e);
        if e.target == Some(me) {
            if e.atk_dellay > 0 {
                e.atk_dellay -= 1;
            }
            if (e.act_num == act::CLOSE || e.act_num == act::CHASE) && e.atk_dellay == 0 {
                set_act(cx, e, act::ATTACK);
            }
        }
    }
    let c = cx.ch().cond;
    if c[cond::HOLD] != 0 || c[cond::PARALYSIS] != 0 || c[cond::SLEEP] != 0 {
        e.speed = 0;
    }
    let c = cx.ch().cond;
    if (c[cond::PARALYSIS] != 0 || c[cond::SLEEP] != 0 || (c[cond::HOLD] != 0 && cx.world.puppet_show))
        && !(act::ATTACK..=act::DEAD).contains(&e.act_num)
    {
        clear_target(e);
        set_act(cx, e, act::WAIT);
    }
    if e.affect_flag {
        let hp = cx.ch().hp;
        match e.affect_type {
            20 => {
                if cx.ch().cond[cond::DEAD] == 2 {
                    cx.scene.chars[me].cond[cond::DEAD] = 0;
                    set_act(cx, e, act::WAIT);
                }
            }
            3 => {
                if hp <= 0 {
                    set_act(cx, e, act::DYING);
                }
            }
            1 => {
                let busy = e.act_num == act::ATTACK || e.act_num == act::DAMAGE;
                if hp <= 0 {
                    set_act(cx, e, act::DYING);
                } else if cx.ch().level() >= 31 {
                    if e.damage_cnt == 0 && e.affect_param0 > 0 && !busy {
                        set_act(cx, e, act::DAMAGE);
                    }
                } else if e.affect_param0 > 0 && !busy {
                    set_act(cx, e, act::DAMAGE);
                }
            }
            _ => {}
        }
        e.affect_flag = false;
    }
    if cx.ch().hp <= 0 && e.act_num < act::DYING {
        set_act(cx, e, act::DYING);
    }
    if e.act_num == act::DEAD {
        if e.act_cnt >= 31 {
            cx.scene.chars[me].cond[cond::DEAD] = 3;
        }
    } else if e.act_num == act::DYING {
        if e.act_cnt == 0 {
            cx.scene.chars[me].cond[cond::DEAD] = 2;
            clear_condition_enemy(&mut cx.scene.chars[me], cx.out);
        }
        if e.act_cnt >= 91 {
            set_act(cx, e, act::DEAD);
            clear_condition_enemy(&mut cx.scene.chars[me], cx.out);
            delete_cmnd(cx, e, true);
            cx.out.push(Out::Exp { level: cx.ch().level() });
            e.fade_flag = 2;
            e.fade_cnt = 30;
            cx.out.push(Out::KillRecord { ene_id: e.ene_id });
        }
    }
}

/// `ccEnemy::initEnemy(ent)` (0x00433260), the spawn's rules after the
/// constructors: the entry, the row and its race, home at the spawn point,
/// the row's stats (`SetBaseParam`), a first attack delay of 32-95 frames, a
/// middle boss's gauge (`virusFlag`, on the enemy and on the character's
/// `ccEnemy` flag word), the skill list ([`init_skill_list`]),
/// the dust colour (given) and `ene_rand` (`ccRand`). The models, animations
/// and body hit are the runtime's.
pub fn init_enemy(
    t: &Tables,
    ent: &EntryParam,
    anm: [bool; 6],
    dust: i16,
    frame: &dyn Frame,
    cc: &mut dyn Rng,
) -> (Char, Enemy) {
    let mut e = Enemy { ent: *ent, ene_id: ent.id, ..Enemy::default() };
    if let Some((race, race_id)) = enemy_race(t, ent.id) {
        e.race = race;
        e.race_id = race_id;
    }
    e.ene_part = ent.param[3] as i16;
    let row = &t.enemies[ent.id as usize];
    let mut ch = Char::foe(row.param.clone());
    ch.condition_num = -1;
    ch.skill_status = -1;
    ch.ent_root = ent.ent_root as u32;
    ch.pos = ent.pos;
    ch.pos_p = frame.w2p(ent.pos);
    e.bpos = ent.pos;
    e.dirc = ent.dirc;
    e.mdirc = ent.dirc;
    clear_target(&mut e);
    e.atk_dellay = ((cc.rand() & 0x3f) + 32) as i16;
    if middle_boss(t, ent.id) >= 0 {
        e.ccs2_flag = true;
        e.fade_flag = 0;
        // initEnemyCCS (0x004331f8): virusFlag, the bit affectEnemy reads
        // off the character (a tenth of each hit, never below half).
        if row.param.max_pp != -1 {
            e.virus_flag = true;
            ch.spc_char.enemy_flags |= crate::chara::enemy_flag::VIRUS;
        }
    }
    init_skill_list(t, &mut e, anm);
    e.ene_smoke = dust;
    e.ene_rand = cc.rand() as i16;
    (ch, e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::param::{Base, FoeRow};

    /// Hits of 9999 on a spawned row, each through `affectEnemy`: the HP
    /// after each and whether the whole 9999 was shown each time.
    fn hits_on(t: &Tables, id: i32, n: usize) -> (Char, Vec<i16>, bool) {
        let ent = EntryParam { id, ..crate::entry::entry_param_clear() };
        let mut r = || 0;
        let (ch, _) = init_enemy(t, &ent, [true; 6], 0, &Identity, &mut r);
        let mut scene = Scene { chars: vec![ch.clone()], ene_list: vec![0], ..Scene::default() };
        let mut shown = true;
        let hp = (0..n)
            .map(|_| {
                scene.chars[0].affect.ty = 1;
                scene.chars[0].affect.param = [9999, 0, 0];
                let mut ev = crate::event::Events::new();
                crate::affect::affect_enemy(t, &mut scene, 0, &mut ev);
                shown &= ev.iter().any(|e| matches!(e, Event::FlyFont { kind: 2, value: 9999, .. }));
                scene.chars[0].hp
            })
            .collect();
        (ch, hp, shown)
    }

    /// A Data Bug (Infection's row 115: type 0x40, a protect gauge) as
    /// `initEnemy` makes it is virus-flagged on its character, so
    /// `affectEnemy` (gcmn 0x00433c0c) takes a tenth of each hit and stops
    /// it at half its maxHP, the whole hit shown. Its base form (113) falls
    /// to 0.
    #[test]
    fn a_data_bug_is_not_beaten_by_damage() {
        let t = Tables::of(piney_data::volume::Volume::Inf);
        let (bug, hp, shown) = hits_on(&t, 115, 40);
        assert_ne!(bug.spc_char.enemy_flags & chara::enemy_flag::VIRUS, 0);
        assert!(shown);
        assert_eq!(bug.max_hp, 20169);
        assert_eq!(hp[0], 20169 - 999);
        assert_eq!(hp[9], 20169 - 9990);
        assert_eq!(hp[10], 10084);
        assert!(hp.iter().all(|&h| h >= 10084));
        let (base, hp, _) = hits_on(&t, 113, 2);
        assert_eq!(base.spc_char.enemy_flags & chara::enemy_flag::VIRUS, 0);
        assert_eq!(hp[0], 0);
    }

    #[test]
    fn rand_f_scales_by_two_to_the_31() {
        let mut r = || -0x4000_0000;
        assert_eq!(rand_f(&mut r, F_HALF), 0xbe80_0000);
        let mut r = || 0x4000_0000;
        assert_eq!(rand_f(&mut r, F_ONE), F_HALF);
    }

    #[test]
    fn directions() {
        let o = [0, 0, 0, F_ONE];
        let north = [0, F_ONE, 0, F_ONE];
        // atan2(1, 0) + pi/2 = pi.
        assert_eq!(get_dirc(o, north), 0x4049_0fdb);
        assert_eq!(get_dist(o, [0x4040_0000, 0x4080_0000, 0x4120_0000, F_ONE]), 0x40a0_0000);
    }

    #[test]
    fn outbreak_distance_truncates() {
        // |(1, 2)| = sqrt(5): newlib rounds to 0x400f1bbd, sqrt.s cuts to ...bc.
        let (o, p) = ([0, 0, 0, F_ONE], [F_ONE, 0x4000_0000, 0, F_ONE]);
        assert_eq!(get_dist_on(Volume::Mut, o, p), 0x400f_1bbd);
        assert_eq!(get_dist_on(Volume::Out, o, p), 0x400f_1bbc);
        assert_eq!(get_dist_on(Volume::Qua, o, p), 0x400f_1bbc);
    }

    fn scene_with(chars: Vec<(i32, [u32; 4], i16)>) -> Scene {
        let mut s = Scene::default();
        for (t, p, dead) in chars {
            let mut row = FoeRow { base: Base { ty: t, ..Base::default() }, ..FoeRow::default() };
            row.max_hp = 10;
            let mut c = Char::foe(row);
            c.pos_p = p;
            c.cond[cond::DEAD] = dead;
            s.add(c, if t & 7 != 0 { 0 } else { 1 });
        }
        s
    }

    #[test]
    fn near_person_takes_later_ties_and_skips_the_dead() {
        let f = |x: f32| x.to_bits();
        let s = scene_with(vec![
            (0x20, [0, 0, 0, F_ONE], 0),
            (0x20, [f(3.0), 0, 0, F_ONE], 0),
            (0x20, [0, f(3.0), 0, F_ONE], 0),
            (0x20, [f(1.0), 0, 0, F_ONE], 2),
        ]);
        let w = World::default();
        assert_eq!(search_near_person(&s, &w, Volume::Inf, 0, 0x60, 0, f(10.0)), Some(2));
        assert_eq!(search_near_person(&s, &w, Volume::Inf, 0, 0x60, 2, f(10.0)), Some(3));
        assert_eq!(search_near_person(&s, &w, Volume::Inf, 0, 0x60, 0, f(2.0)), None);
    }

    #[test]
    fn skill_rates_weight_by_spell() {
        let mut e = Enemy { skill_num: 3, ..Enemy::default() };
        e.skill_list[0].ski_id = -1;
        e.skill_list[1].ski_id = 150;
        e.skill_list[2].ski_id = 156;
        set_skill_rate(&mut e);
        assert_eq!(e.skill_list[1].percentage, 0);
        let (a, c) = (f32::from_bits(e.skill_list[0].percentage), f32::from_bits(e.skill_list[2].percentage));
        assert!((a + c - 1.0).abs() < 1e-6 && c > a);
    }

    #[test]
    fn kill_record_holds_at_99() {
        let mut save = SaveData::new();
        save.set_u8(SAVE_ENEMY_KILL_COUNT + 5, 99);
        record_kill(&mut save, 5, 1, [2, 3, 4]);
        assert_eq!(save.u8(SAVE_ENEMY_KILL_COUNT + 5), 99);
        assert_eq!(save.i16(SAVE_ENEMY_KILL_AREA + 40 + 6), 4);
        save.set_u8(SAVE_ENEMY_KILL_COUNT + 6, 127);
        record_kill(&mut save, 6, 0, [0; 3]);
        assert_eq!(save.u8(SAVE_ENEMY_KILL_COUNT + 6), 0x80);
    }
}
