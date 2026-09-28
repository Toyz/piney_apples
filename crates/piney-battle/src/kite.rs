//! Kite himself: the player's side of a fight (`ccPlayer`, gcmn
//! `player.cpp`, 0x005977d0-0x0059ce00), beyond a player walking alone in
//! a town. Where piney-world's `player.rs` and `motion.rs` port the town
//! case of `ccPlayer::Main` (0x00598310), `ControlMove` (0x00598af0) and
//! `AnimCtrl` (0x005993c0), this module ports them whole:
//!
//! - [`anim_ctrl`]: every act (standing, the fidget, walking and running,
//!   the normal attack's acts 15/16 and its combo, the spells and skills
//!   17-21, the "hack" act 24/23, hurt 7/8, down 9/10 and the ghost,
//!   getting up 2, the arrival 13, the gate act 12, breaking a box 25),
//!   the skill state machine (`skillStatus` 1 -> 2), the targets
//!   (`SetTargetDist` 0x0059ab80, `SetTargetDirc` 0x0059aa90), the fade of
//!   a ghost and of a revival, and the animation's notes
//!   (`ccPlayerCheckNote` 0x0059c550 -> [`check_note`] 0x0059c300);
//! - [`main`]: the frame (`ccPlayer::Main`): conditions, the AI when he is
//!   charmed, confused or in manual mode (`ccAI::Brains`), [`control_move`]
//!   otherwise, the collision, the field's map wrap
//!   ([`map_loop_adjust_pos`] 0x0059b3c0), the camera's placement and the
//!   draw;
//! - [`attack`] (`ccPlayer::Attack` 0x0059c580, his AI's attack, which
//!   [`crate::party_ai::Call::PlayerAttack`] asks for), [`attack_cancel`]
//!   (0x0059cad0), [`break_something`] (0x0059cbd0), [`damage_actuate`]
//!   (0x0059ca40), [`result_of_conditions`] (0x0059c270), [`menu_check`]
//!   (`ccPlayerMenuCheck` 0x0059cd70);
//! - the frames a position is given in: the map ([`w2m_pos`] 0x0059b470,
//!   `ccTransPosW2M` 0x0059b900) and the player's ([`w2p_pos`] 0x0059b5a0,
//!   [`p2w_pos`] 0x0059b710, `ccTransPosW2P` 0x0059b940, `ccTransPosP2W`
//!   0x0059b980, [`fw2lw`] 0x0059b9c0), pure functions of the map's
//!   bounds ([`MapBounds`], `WORLD_MAN` +0x420) with which the runtime
//!   implements [`World::w2p`] and [`World::p2w`].
//!
//! # State
//!
//! Kite is a scene character ([`Char`]) whose `ccChar`/`ccSpcChar`
//! members the rules share: `actNum`, `actNumOld`, the flag word at +0xe0
//! (`pauseSW`, `dispSW`, `restraintSW`, `moveFlag`, `stopFlag`, `runFlag`,
//! `ghostFlag`, `weaponChangeSW`, `trajectorySW`, `lostHeadFlag`),
//! `attack`, `cnt`, `cloak`, `anmFlag`, `armsEffectSW`, `targetChar`, the
//! skill, the conditions. His motion members are his [`Spc`] in the
//! [`Crew`] (`dirc`, `speed`, `speedRate`, `nowSpeed`, `movePos`, `cycle`,
//! `stopCnt`, `walkRunCnt`, the act counters `atkAnmCnt`, `actCnt`,
//! `reactCnt`, `transferLag`, `bodyHit`, `hitAttribute`, `transparency`,
//! `setTransparency`: what the party's movement, [`crate::follow`] and
//! [`crate::ai_move`], also reads and writes on him), his `ccAI` is the
//! crew's [`crate::party_ai::Ai`] under his scene index, and the rest of
//! `ccPlayer` is a [`Player`].
//! The party AI reads and writes the copies [`Spc`] keeps of `actNum`,
//! `targetChar`, `moveFlag`, `runFlag`, `ghostFlag` and `stopFlag`; for
//! Kite the [`Char`] holds them, and every function here copies them into
//! his [`Spc`] before it calls into the party AI and when it returns, and
//! back from it after the AI ran.
//!
//! # The runtime
//!
//! Every call the game makes into code that is not rules goes through
//! [`KiteWorld`] (a [`World`] and a [`Runtime`]) at the point and as often
//! as the game makes it: the animation player, the collision, the camera,
//! the draw, `ccSkillRequest` ([`Call::SkillRequest`], which the runtime
//! runs with [`crate::flow::Skills::request`]); what only shows, sounds or
//! is kept for other systems is an [`Out`] handed to [`KiteWorld::out`]
//! in the game's order, the rules' events among them ([`Out::Rule`]). The
//! AI's own decisions (`ccAI::Brains`, `levelCheck`, `SelectAttackSkill`,
//! `UseItem`) run here through [`crate::party_ai::Ctx`].
//!
//! The party AI's movement calls - `FollowTarget`, `FollowTargetDirc`,
//! `FollowPlayer`, `LeavePlayer`, `PathFinding`, `FollowBeacon`,
//! `ManualControl`, `ccPlayer::Attack`, `HitEnable` - are the same game's
//! code as the rest of `ccAI`. A runtime has them performed by the ported
//! code by wrapping its world in a [`Host`]: its [`Runtime`] is
//! [`crate::party_motion::Movement`], which passes what it does not
//! perform on to the world's own. `tools/test_battle_kite_rs.py`'s
//! `main_run_ai` runs [`main`] so, frame by frame against the game with
//! all of those running natively, Kite charmed, confused, a charmed ghost
//! or under an event's remote control in a field.
//!
//! Every `ccChar::EntryAffect` Kite's code makes (his `CalcReal` timers'
//! poison, curse and regeneration in [`main`], his normal attack's or art's
//! hit in [`check_note`]) is applied where the game makes it, with the
//! character's `affectFunc` ([`crate::affect::entry_affect`]: `Influence`
//! for Kite, `ccFellow::Influence`, `ccEnemyInfluence`), so what follows
//! in the same frame sees it: a poison tick that downs him inside
//! `CalcReal` has him falling (act 9) by `AnimCtrl`. What an affect leads
//! to is made there too where the rules keep the state (see
//! [`Out::Rule`]); no [`crate::event::Event::Affect`] is ever handed out,
//! and the runtime applies none of Kite's.
//!
//! Once a frame, on the task `ccThPlayer` (priority 49), the runtime calls
//! [`main`], which calls [`anim_ctrl`] after the camera; when the party
//! AI asks for [`Call::PlayerAttack`] it calls [`attack`]; the item and
//! trap menus call [`break_something`] and [`attack_cancel`]; `Influence`'s
//! [`crate::event::Event::DamageActuate`] from any other caller of
//! `EntryAffect` on Kite is [`damage_actuate`] (on his act from before the
//! affect); `ccThGameCtrl` asks [`menu_check`] and [`check_control_mode`].

use std::cell::RefCell;

use piney_data::save::SaveData;

use crate::chara::{Char, Env, spc_flag};
use crate::damage::fptosi;
use crate::event::{Event, Events, Who};
use crate::exp::Party;
use crate::fellow::MotionTables;
use crate::flow;
use crate::geom::{self, F, ONE, PI, V4, VF0, add, div, from_int, le, lt, mul, neg, sub};
use crate::navi::NaviWorld;
use crate::param::cond;
use crate::party_ai::{self, Call, Crew, Game, Runtime, Spc};
use crate::party_motion::{Keep, Movement};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::skill;
use crate::tables::Tables;
use crate::world::{AnmSlot, CharHit, Note, World};

/// Bits of the flag word at `ccSpcChar` +0xe0 ([`crate::chara::SpcChar::flags`])
/// besides those [`spc_flag`] names.
pub mod flag {
    /// `dispSW`: he is drawn.
    pub const DISP: u32 = 1 << 1;
    /// `runFlag`.
    pub const RUN: u32 = 1 << 5;
    /// `weaponChangeSW` (a signed 2-bit field): 1 equip the new weapon,
    /// -1 (3) delete the old one's file.
    pub const WEAPON_CHANGE: u32 = 3 << 8;
    /// `lostHeadFlag`: the eye view hides him.
    pub const LOST_HEAD: u32 = 1 << 13;
}

const PAUSE: u32 = spc_flag::PAUSE;
const RESTRAINT: u32 = spc_flag::RESTRAINT;
const MOVE: u32 = spc_flag::MOVE;
const STOP: u32 = spc_flag::STOP;
const GHOST: u32 = spc_flag::GHOST;
const TRAJECTORY: u32 = spc_flag::TRAJECTORY;
const RUN: u32 = flag::RUN;
const DISP: u32 = flag::DISP;

/// The acts (`actNum`), each an animation of `playerAnimTbl`.
pub mod act {
    /// Standing in a field or dungeon (`nut0`), its fidget (`nut1`).
    pub const IDLE_FIELD: i16 = 0;
    pub const FIDGET_FIELD: i16 = 1;
    /// Standing in a town (`nut2`), the fidget (`nut3`) and its loop (`nut4`).
    pub const IDLE: i16 = 2;
    pub const FIDGET: i16 = 3;
    pub const FIDGET_LOOP: i16 = 4;
    pub const RUN: i16 = 5;
    pub const WALK: i16 = 6;
    /// Hurt (`dmg0`, `dmg1`), down (`dwn0`) and lying down.
    pub const HURT: i16 = 7;
    pub const HURT2: i16 = 8;
    pub const DOWN: i16 = 9;
    pub const LYING: i16 = 10;
    /// Leaving through a gate (the fade out), then held.
    pub const GATE_OUT: i16 = 12;
    /// Arriving (the fade in).
    pub const ARRIVE: i16 = 13;
    pub const HELD: i16 = 14;
    /// The normal attack's two swings (`atc0`, `atc1`).
    pub const ATTACK1: i16 = 15;
    pub const ATTACK2: i16 = 16;
    /// A spell (`mag0`, `mag1`) and the three arts (`ski2`-`ski4`), by the
    /// skill's type bits 0x100-0x1000.
    pub const MAGIC: i16 = 17;
    pub const ART3: i16 = 21;
    /// A skill of none of those kinds: the "hack" (`hac4b`), then 23.
    pub const HACK_END: i16 = 23;
    pub const HACK: i16 = 24;
    /// Breaking a box or a trap open (`atc0`).
    pub const BREAK: i16 = 25;
}

/// What the functions here read besides [`Tables`] (`tables::combat`):
/// `playerAnimTbl` (INF gcmn 0x006f0560, 26 names of 21 bytes),
/// `DamActuTbl` (INF main 0x003782b8) and `tsp` (INF main 0x003782b0).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KiteTables {
    /// `playerAnimTbl`: each act's clip, `ANM_ctu1nut0` ...
    pub anims: Vec<String>,
    /// `DamActuTbl`.
    pub dam_actu: [i16; 3],
    /// `tsp` (3.1 a frame), float bits.
    pub tsp: F,
}

impl KiteTables {
    /// The volume's.
    pub fn of(volume: piney_data::volume::Volume) -> KiteTables {
        let t = piney_data::tables::combat::of(volume);
        let d = t.dam_actu();
        KiteTables {
            anims: t.player_anims().iter().map(|s| s.to_string()).collect(),
            dam_actu: [d[0], d[1], d[2]],
            tsp: t.tsp().to_bits(),
        }
    }

    /// The clip act `n` plays; empty past the table (the game reads the
    /// bytes after it).
    pub fn anim(&self, n: i16) -> &str {
        usize::try_from(n).ok().and_then(|i| self.anims.get(i)).map_or("", |s| s.as_str())
    }
}

/// The `ccPlayer` members that neither [`Char`] nor [`Spc`] keeps. The
/// `ccChar`/`ccSpcChar` members the party's movement also reads and writes
/// are his [`Spc`]'s: `atkAnmCnt`, `actCnt`, `reactCnt`, `transferLag`,
/// `bodyHit`, `hitAttribute`, `transparency` and `setTransparency`
/// ([`Spc::atk_anm_cnt`] ... [`Spc::set_transparency`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    /// +0x200 bits: 0 `cameraFlag` (the camera is not placed on him), 1
    /// `inactiveSW` (the stick does nothing), 2 `installSW`.
    pub camera_flag: bool,
    pub inactive: bool,
    pub install: bool,
    /// +0x204 `ctrlType`: 0 walking, 1 steering, others backing up.
    pub ctrl_type: i32,
    /// +0x208 `progCtrlFlag`: a cutscene moves him (the gate), not the pad.
    pub prog_ctrl_flag: i32,
    /// +0x238 `distTg`: the ground distance to `targetChar` less its
    /// width, truncated.
    pub dist_tg: i32,
    /// +0x23c `dircTg`: the heading to it.
    pub dirc_tg: F,
    /// +0x240 `posView`, +0x260 `posEye`, +0x270 `angle`: the camera's
    /// target, his eyes, the eye view's angles.
    pub pos_view: V4,
    /// +0x250: the camera's eye as `GateHackingOut` last placed it, which
    /// the field camera takes over when it ends.
    pub pos_cam: V4,
    pub pos_eye: V4,
    pub angle: V4,
    /// +0x2d0 `diskOffset`: where he stands relative to a moving floor's
    /// centre (`WORLD_MAN::GetTransCenter`). `ccPlayer`'s own member, not
    /// `ccFellow`'s ([`Spc::disk_offset`]).
    pub disk_offset: V4,
    /// +0x2e4 `targetCount`: frames of a lean (to 4).
    pub target_count: i16,
    /// +0x2e6 `actOneTwoCnt`: frames left to chain the second swing.
    pub act_one_two_cnt: i16,
    /// +0x2e8 `stressMeter`: pushing against a wall.
    pub stress: i16,
    /// `anm->frameSpd` (+0x9c of the `ccAnm`): the step `AnimCtrl` sets.
    pub frame_spd: u16,
}

impl Default for Player {
    fn default() -> Self {
        Player {
            camera_flag: false,
            inactive: false,
            install: false,
            ctrl_type: 0,
            prog_ctrl_flag: 0,
            dist_tg: 0,
            dirc_tg: 0,
            pos_view: VF0,
            pos_cam: VF0,
            pos_eye: VF0,
            angle: VF0,
            disk_offset: VF0,
            target_count: 0,
            act_one_two_cnt: 0,
            stress: 0,
            frame_spd: 256,
        }
    }
}

/// `WORLD_MAN` +0x420-+0x42c: the map's bounds, `minX minY maxX maxY`
/// (float bits). A field's map wraps around them; in a town they are
/// +-24000 and nothing reaches them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapBounds {
    pub min: [F; 2],
    pub max: [F; 2],
}

/// The pad as `ControlMove` reads it (`ccSys->pad[0]`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pad {
    /// `powL` (+0x2b0): the left stick's lean, 0-255.
    pub pow_l: u8,
    /// `dircL` (+0x2b4): its direction, radians.
    pub dirc_l: F,
}

/// The globals the functions here read that are no character's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub pad: Pad,
    /// `cmndTarget != 0` (0x00378c64): something is targeted.
    pub cmnd_target: bool,
    /// `worldman->warpFlag` (+0x168): arriving by warp.
    pub warp: bool,
    /// `dneFlag` (0x00378cd8): leaving a dungeon (no area change on the
    /// ground's attribute).
    pub dne: bool,
    /// `WORLD_MAN` +0x420: the map's bounds.
    pub bounds: MapBounds,
    /// `ccMenu` exists (it does in the field): a hit shakes the party
    /// panel ([`crate::affect::AffectCtx::menu`]).
    pub menu: bool,
}

/// What Kite's code hands the rest of the game besides the calls that
/// answer: presentation, and the rules' events, in the game's order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Out {
    /// An event of the crate's rules (`CalcReal`, `CalcBattleDamage`, the
    /// `affectFunc` of a character an affect reached) for the runtime to
    /// show or run. Never an [`Event::Affect`]: every `EntryAffect` of
    /// Kite's code is applied where the game calls it (see [`Cx`]'s
    /// `entry_affect`), and of what it leads to, the calls with state the
    /// rules keep are made there too: `ccSkillRequest(ch, 0, 0)`
    /// ([`Event::CancelAttack`], through [`Runtime::call`]),
    /// `ccCharHit::HitEnable`/`HitDisable` on Kite's body, his
    /// `DamageActuate` ([`Out::Actuate`]), the "down" and "up" messages on
    /// the AI bus ([`Event::SysMsgDown`], [`Event::SysMsgUp`]) and a
    /// member's `talkFlag` ([`Event::TalkOff`]). Left to the runtime, and
    /// so run after the call: an enemy's `selectTarget`
    /// ([`Event::EnemyRetarget`]), the AI's lines
    /// ([`Event::ChatDamage`], [`Event::ChatResurrectPlz`],
    /// [`Event::AffectMessages`], [`Event::Greeting`]) and another
    /// character's body switch.
    Rule(Event),
    /// `ccSpcChar::SetArmsEffectColor(sid)` (gcmn 0x0059e460): the weapon
    /// trails take the skill's element's colour.
    ArmsEffectColor {
        sid: i32,
    },
    /// `startParticleEffect2(&weaponEffPos[a], &weaponEffPos[b], attr,
    /// &armsEffectSW)` from `ccSpcChar::StartArmsEffect` (0x0059e530): an
    /// art's element particles along a blade (`weaponEffPos`, +0x160).
    ArmsParticles {
        a: i32,
        b: i32,
        attr: i32,
    },
    /// `ccSpcChar::ArmsEffect()` (0x0059ddd0): the weapon trails this frame.
    ArmsEffect,
    /// `ccSpcChar::ClearArmsEffect()` (0x0059e3b0): the trails end.
    ClearArmsEffect,
    /// `effSkillStart(this, sid, item, 0)`: a skill's opening effect.
    SkillStart {
        sid: i32,
        item: i32,
    },
    /// `effTransfer(this)`, `effWarpTransfer(this)`: the arrival's effect.
    Transfer,
    WarpTransfer,
    /// `ccSeSetParamSPC(param, this)`: a footstep (notes 1 and 2).
    Sound {
        param: u32,
    },
    /// `ccEffPawSmoke(this, speed)`: dust while running.
    PawSmoke {
        speed: F,
    },
    /// `effLevelUp(this)`.
    LevelUp,
    /// `effOpenBox(pos)`: the down act ends (twice).
    OpenBox {
        pos: V4,
    },
    /// `ccSpcChar::EquipWeapon()` (0x0059d650), `DeleteWeaponCCS()`
    /// (0x0059dda0): the weapon changed.
    EquipWeapon,
    DeleteWeaponCcs,
    /// `WORLD_MAN::AddCenter(x, y)`: the field's streaming follows him.
    AddCenter {
        x: F,
        y: F,
    },
    /// `WORLD_MAN::Enter(pos)`: he stepped onto ground with attribute bit
    /// 0x80000 (a way into a dungeon or out of the area).
    Enter {
        pos: V4,
    },
    /// `anm->SetMatrix_PosRotZYX(pos, dirc)`: his model's root.
    Matrix {
        pos: V4,
        dirc: V4,
    },
    /// `plw` +0x8 and bit 0 of +0x0: `attack` and `pauseSW` for the menus.
    PlayerWork {
        attack: i32,
        pause: bool,
    },
    /// Acts 23 and 24: the blades' models take `ghoCamArmsT`.
    GateArms,
    /// `ccPad::SetActuater(pad, 1, power, 100)`: the pad rumbles.
    Actuate {
        power: i16,
    },
}

/// What Kite's code asks of the world beyond [`World`]; the party AI's
/// calls are [`Runtime::call`]'s. Each method is the game function it
/// names, called where and as often as the game calls it.
pub trait KiteWorld: World + Runtime {
    /// Every [`Out`], in order.
    fn out(&mut self, o: Out);
    /// `checkCameraType()` (main 0x00160cb0), `activeCamPtr->type`: 1 the
    /// eye view, 3 following him.
    fn camera_type(&mut self) -> i32;
    /// `checkCameraID()` (main 0x00160ca0): the active camera, 1 the field's.
    fn camera_id(&mut self) -> i32;
    /// `cameraGetRot(&v, camID)` (main 0x00161610): the camera's rotation
    /// (z its heading).
    fn camera_rot(&mut self) -> V4;
    /// `activeCamPtr->resetFlag` (+0x60) and `resetDirc` (+0x64): the
    /// heading of a camera reset while one runs.
    fn camera_reset(&mut self) -> Option<F>;
    /// `activeCamPtr->resetFlag = 0`: leaning away ends the reset.
    fn clear_camera_reset(&mut self);
    /// `cameraSetEyeLevel(posEye, angle)` (main 0x00162020): the eye view
    /// from his eyes; turns `angle`.
    fn camera_set_eye_level(&mut self, pos_eye: V4, angle: &mut V4);
    /// `cameraSetManual(posView)` (main 0x00161820): the following camera
    /// on his head.
    fn camera_set_manual(&mut self, pos_view: V4);
    /// `cameraSet()` (main 0x00161260): the view matrix.
    fn camera_set(&mut self);
    /// `ccSpcChar::HitCheck(movePos)` (gcmn 0x0059ee20) on Kite (`ch`, his
    /// body `hit`, `nowSpeed`, his AI's `manualSW`): the move pushed out of
    /// the other bodies and slid along the walls. Returns the touch bits
    /// (1 a body, 2 the ground).
    fn hit_check(
        &mut self,
        me: usize,
        ch: &Char,
        hit: &mut CharHit,
        now_speed: F,
        manual: bool,
        move_pos: &mut V4,
    ) -> i32;
    /// `hitResultNum` (main 0x00378924) and `hitResultNearest` +0x48
    /// (0x00383ba8): the attribute of the nearest polygon the last ground
    /// query hit, none when it hit nothing.
    fn hit_result(&mut self) -> Option<u32>;
    /// `WORLD_MAN::GetTransMode()` (main 0x001a3ad0): he stands on a moving
    /// floor.
    fn trans_mode(&mut self) -> bool;
    /// `WORLD_MAN::GetTransCenter(v)` (main 0x001a3b10): its centre.
    fn trans_center(&mut self) -> V4;
    /// `ccChar::Draw()` (gcmn 0x0056b1c0) on Kite at `pos`: sets his
    /// `transparency` (`set_transparency` times the camera's fade) and
    /// draws him; whether he was drawn.
    fn draw(&mut self, me: usize, pos: V4, set_transparency: F, transparency: &mut F) -> bool;
    /// `ccSkillCheck(ch)` (gcmn 0x005723e0): the id of `ch`'s running skill
    /// a hit interrupts, 0 for none ([`crate::flow::Skills::check`] with
    /// the player's `actNum`); `EntryAffect`'s `affectFunc` asks it. A pure
    /// query: [`crate::affect`] asks it for every affect on a party member,
    /// the game only on the paths that use it.
    fn skill_check(&mut self, scene: &Scene, ch: usize) -> i32;
    /// `ccPlayer::GateHackingOut()` (gcmn 0x0059bc00): a gate-hacked
    /// arrival's cutscene in acts 23 and 24 (its camera animation,
    /// `progCtrlFlag`'s steps), which the runtime runs on Kite's state
    /// (`piney_world::combat::gate_out`).
    fn gate_hacking_out(&mut self, me: usize, ch: &mut Char, p: &mut Player, spc: &mut Spc);
}

/// Everything Kite's frame reads and writes, borrowed for the call.
pub struct Cx<'a> {
    pub t: &'a Tables,
    pub kt: &'a KiteTables,
    pub scene: &'a mut Scene,
    pub party: &'a Party,
    pub save: &'a mut SaveData,
    pub crew: &'a mut Crew,
    pub game: &'a Game,
    /// `g_entCtrl`'s enemies, for the party AI.
    pub ents: &'a [usize],
    /// The globals `CalcReal` and `CalcBattleDamage` read.
    pub env: &'a Env,
    pub input: &'a Input,
    pub p: &'a mut Player,
    pub rng: &'a mut dyn Rng,
    pub w: &'a mut dyn KiteWorld,
}

// ---------------------------------------------------------------------------
// The copies the party AI keeps

/// Kite's `actNum`, `targetChar` and flags into his [`Spc`], where the
/// party AI reads them.
pub fn to_spc(scene: &Scene, crew: &mut Crew, me: usize) {
    let c = &scene.chars[me];
    let f = c.spc_char.flags;
    let s = crew.spc.entry(me).or_default();
    s.act_num = c.spc_char.act_num;
    s.target_char = c.target_char;
    s.move_flag = f & MOVE != 0;
    s.run_flag = f & RUN != 0;
    s.ghost = f & GHOST != 0;
    s.stop_flag = f & STOP != 0;
}

/// And back, after the party AI ran on him (with `bodyHit.hitSW`, which
/// a mover may have switched on, into [`crate::chara::SpcChar::hit_enabled`]).
pub fn from_spc(scene: &mut Scene, crew: &Crew, me: usize) {
    let Some(s) = crew.spc.get(&me) else { return };
    let c = &mut scene.chars[me];
    c.spc_char.act_num = s.act_num;
    c.target_char = s.target_char;
    c.spc_char.hit_enabled = s.body_hit.sw;
    let f = &mut c.spc_char.flags;
    for (bit, on) in [(MOVE, s.move_flag), (RUN, s.run_flag), (GHOST, s.ghost), (STOP, s.stop_flag)] {
        if on {
            *f |= bit;
        } else {
            *f &= !bit;
        }
    }
}

impl Cx<'_> {
    fn ch(&mut self, me: usize) -> &mut Char {
        &mut self.scene.chars[me]
    }

    fn spc(&mut self, me: usize) -> &mut Spc {
        self.crew.spc.entry(me).or_default()
    }

    fn flags(&self, me: usize) -> u32 {
        self.scene.chars[me].spc_char.flags
    }

    fn set(&mut self, me: usize, bits: u32) {
        self.scene.chars[me].spc_char.flags |= bits;
    }

    fn clear(&mut self, me: usize, bits: u32) {
        self.scene.chars[me].spc_char.flags &= !bits;
    }

    fn act(&self, me: usize) -> i16 {
        self.scene.chars[me].spc_char.act_num
    }

    /// `ccSpcChar::SetActNum(n)` (gcmn 0x0059d630).
    fn set_act(&mut self, me: usize, n: i16) {
        self.scene.chars[me].spc_char.act_num = n;
    }

    /// `ccSpcChar::SetActNumOld(n)` (gcmn 0x0059d640).
    fn set_act_old(&mut self, me: usize, n: i16) {
        self.scene.chars[me].spc_char.act_num_old = n;
    }

    fn cond(&self, me: usize, i: usize) -> i16 {
        self.scene.chars[me].cond[i]
    }

    fn manual(&self, me: usize) -> bool {
        self.crew.ais.get(&me).is_some_and(|a| a.manual_sw)
    }

    /// `ccSkillRequest(this, tp, sid)` (gcmn 0x00572700): the runtime's
    /// ([`crate::flow::Skills::request`]), which may end his running normal
    /// attack (`skillStatus` 0) before the code reads it again.
    fn skill_request(&mut self, me: usize, target: Option<usize>, sid: i32) {
        self.w.call(Call::SkillRequest { me, target, sid }, self.scene, self.crew, self.rng);
    }

    /// `ccSpcChar::CheckSysMsgID()` (gcmn 0x0059f530) as the unsigned
    /// short the senders pass.
    fn msg_id(&self, me: usize) -> u16 {
        self.crew.sys_msg_id(me) as u16
    }

    /// `ccSkillDamageValue(this, tp, sid)` (gcmn 0x00594e90); 0 for no
    /// target (it is not on the lists).
    fn value(&self, me: usize, tp: Option<usize>, sid: i32) -> i32 {
        let Some(tp) = tp else { return 0 };
        let (a, b) = (self.scene.listed(me), self.scene.listed(tp));
        crate::damage::skill_damage_value(
            self.t,
            &self.scene.chars[me],
            &self.scene.chars[tp],
            sid,
            a,
            b,
            &Env::default(),
        )
        .dmg
    }

    /// The damage a started skill will do, announced to the party:
    /// `ccAISysMsgSendP(0x10008, -1, id, id, 0, 0, value, target)`.
    fn announce(&mut self, me: usize, sid: i32) {
        let tp = self.scene.chars[me].target_char;
        let a = self.msg_id(me);
        let b = self.msg_id(me);
        let v = self.value(me, tp, sid);
        self.crew.send(0x10008, a, b, 0, 0, v, tp);
    }

    /// The party AI on Kite, with this call's borrows.
    fn ai(&mut self) -> party_ai::Ctx<'_> {
        party_ai::Ctx {
            t: self.t,
            scene: &mut *self.scene,
            party: self.party,
            save: &mut *self.save,
            crew: &mut *self.crew,
            game: self.game,
            ents: self.ents,
            rng: &mut *self.rng,
            rt: &mut *self.w,
        }
    }

    fn out(&mut self, o: Out) {
        self.w.out(o);
    }

    /// The events of a rule Kite's code ran, in order: an `EntryAffect`
    /// is applied at once ([`Cx::entry_affect`]), the rest handed out.
    fn rules(&mut self, me: usize, ev: Events) {
        for mut e in ev {
            e.map_who(|w| if w == Who::Me { Who::Char(me) } else { w });
            match e {
                Event::Affect { on: Who::Char(on), by, kind, p } => {
                    let by = if let Who::Char(i) = by { Some(i) } else { None };
                    self.entry_affect(me, on, by, kind, p);
                }
                e => self.consequence(me, e),
            }
        }
    }

    /// `ccChar::EntryAffect(by, kind, p0, p1, p2)` (gcmn 0x0056b020) on
    /// `on`, where the game calls it: [`crate::affect::entry_affect`] with
    /// the character's `affectFunc`, then what it leads to in order
    /// ([`Cx::consequence`]); a party member's copies in its [`Spc`] follow.
    ///
    /// `Influence` calls `DamageActuate` before it sets the hurt or down
    /// act, so the rumble is decided on the act the character had when
    /// the affect came (the event carries it: held, act 14, feels nothing
    /// even when the hit downs him).
    fn entry_affect(&mut self, me: usize, on: usize, by: Option<usize>, kind: i16, p: [i16; 3]) {
        let running = self.w.skill_check(self.scene, on);
        let check = move |_: usize| running;
        let ctx = crate::affect::AffectCtx {
            party: self.party,
            menu: self.input.menu,
            skill_check: &check,
            boss: None,
            volume: self.t.volume,
        };
        let mut ev = Events::new();
        crate::affect::entry_affect(self.t, self.scene, &ctx, on, by, kind, p, self.rng, &mut ev);
        if self.crew.spc.contains_key(&on) {
            to_spc(self.scene, self.crew, on);
        }
        for e in ev {
            self.consequence(me, e);
        }
    }

    /// What an affect (or a rule) leads to: the calls with state made here
    /// as the game makes them, the rest handed out ([`Out::Rule`]).
    fn consequence(&mut self, me: usize, e: Event) {
        match e {
            Event::CancelAttack(Who::Char(c)) => self.skill_request(c, None, 0),
            Event::HitEnable(Who::Char(c)) | Event::HitDisable(Who::Char(c)) if c == me => {
                let on = matches!(e, Event::HitEnable(_));
                hit_switch(self, me, on);
            }
            Event::DamageActuate { value, act } => {
                if let Some(o) = actuate(self.kt, act, value) {
                    self.w.out(o);
                }
            }
            Event::SysMsgDown { on: Who::Char(c), .. } => {
                // ccAISysMsgSendP(0x1000c, -1, id, 0xffff, 0, 30, -1, ch)
                let id = self.crew.sys_msg_id(c) as u16;
                self.crew.send(0x1000c, id, 0xffff, 0, 30, -1, Some(c));
            }
            Event::SysMsgUp { on: Who::Char(c), .. } => {
                // ccAISysMsgDeleteDelay(0x1000d, id, id)
                let id = self.crew.sys_msg_id(c) as u16;
                self.crew.sys.delete_delay(0x1000d, id, id);
            }
            Event::TalkOff(Who::Char(c)) => {
                if let Some(a) = self.crew.ais.get_mut(&c) {
                    a.talk_flag = false;
                }
            }
            e => self.w.out(Out::Rule(e)),
        }
    }
}

// ---------------------------------------------------------------------------
// The host: the party's movement performed by the ported code

/// A [`KiteWorld`] over a world `W` whose party-AI calls are performed by
/// the ported code: its [`Runtime`] is [`crate::party_motion::Movement`] (the
/// following, the dungeon's path finding, the AI's movers, his own
/// [`attack`], `HitEnable`), and what that does not perform goes on to
/// `W`'s own [`Runtime`] (the skill and item requests, the chat lines,
/// the transfers, the lines of sight the decisions ask). Every [`World`]
/// and [`KiteWorld`] call is `W`'s.
///
/// `W` is shared through a [`RefCell`] with the movement, which calls the
/// world while Kite's frame holds it (his `Main` asks `ccAI::Brains`,
/// which asks `FollowTarget`, which asks the collision): each call
/// borrows it for the call alone. A host keeps [`crate::party_motion::Keep`]
/// across frames and makes one of these for each call into this module:
///
/// ```text
/// let cell = RefCell::new(&mut my_world);     // W: KiteWorld + NaviWorld
/// let mut host = kite::Host { t, mt, party, game, keep: &mut keep, spc_registry_num, world: &cell };
/// kite::main(&mut Cx { t, kt, scene, party, save, crew, game, ents, env, input, p, rng, w: &mut host }, me);
/// ```
pub struct Host<'a, 'c, 'w, W: ?Sized> {
    pub t: &'a Tables,
    pub mt: &'a MotionTables,
    /// `ccPartyManager`: Kite is member 0.
    pub party: &'a Party,
    /// `ccGame.area` and the rest the movement reads.
    pub game: &'a Game,
    /// What the movement keeps between frames.
    pub keep: &'a mut Keep,
    /// `ccSpcRegistryNum()` (gcmn 0x005a1620).
    pub spc_registry_num: i32,
    pub world: &'c RefCell<&'w mut W>,
}

/// `W`'s own [`Runtime`], borrowed from the cell for each call: the
/// movement's `inner`.
struct Own<'c, 'w, W: ?Sized>(&'c RefCell<&'w mut W>);

impl<W: Runtime + ?Sized> Runtime for Own<'_, '_, W> {
    fn call(&mut self, call: Call, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) -> i32 {
        self.0.borrow_mut().call(call, scene, crew, rng)
    }
    fn call_ctx(&mut self, call: Call, p: party_ai::Parts<'_>) -> i32 {
        self.0.borrow_mut().call_ctx(call, p)
    }
    fn w2p(&mut self, pos: V4) -> V4 {
        Runtime::w2p(&mut **self.0.borrow_mut(), pos)
    }
}

impl<W: KiteWorld + NaviWorld + ?Sized> Host<'_, '_, '_, W> {
    /// `f` on the movement of this call.
    fn movement<R>(&mut self, f: impl FnOnce(&mut dyn Runtime) -> R) -> R {
        let mut own = Own(self.world);
        let mut m = Movement {
            t: self.t,
            mt: self.mt,
            party: self.party,
            game: self.game,
            keep: &mut *self.keep,
            spc_registry_num: self.spc_registry_num,
            world: self.world,
            inner: &mut own,
        };
        f(&mut m)
    }
}

impl<W: KiteWorld + NaviWorld + ?Sized> Runtime for Host<'_, '_, '_, W> {
    fn call(&mut self, call: Call, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) -> i32 {
        self.movement(|m| m.call(call, scene, crew, rng))
    }
    fn call_ctx(&mut self, call: Call, p: party_ai::Parts<'_>) -> i32 {
        self.movement(|m| m.call_ctx(call, p))
    }
    fn w2p(&mut self, pos: V4) -> V4 {
        World::w2p(&mut **self.world.borrow_mut(), pos)
    }
}

impl<W: World + ?Sized> World for Host<'_, '_, '_, W> {
    fn w2p(&mut self, pos: V4) -> V4 {
        World::w2p(&mut **self.world.borrow_mut(), pos)
    }
    fn p2w(&mut self, pos: V4) -> V4 {
        self.world.borrow_mut().p2w(pos)
    }
    fn land(&mut self, pos: V4, mask: u32) -> F {
        self.world.borrow_mut().land(pos, mask)
    }
    fn hit_attribute(&mut self) -> u32 {
        self.world.borrow_mut().hit_attribute()
    }
    fn line(&mut self, from: V4, to: V4, mask: u32, kind: i32) -> F {
        self.world.borrow_mut().line(from, to, mask, kind)
    }
    fn collide(&mut self, who: usize, hit: &mut CharHit) -> i32 {
        self.world.borrow_mut().collide(who, hit)
    }
    fn hit_char_type(&mut self) -> u32 {
        self.world.borrow_mut().hit_char_type()
    }
    fn hit_switch(&mut self, who: usize, hit: &mut CharHit, on: bool) {
        self.world.borrow_mut().hit_switch(who, hit, on)
    }
    fn camera_deg(&mut self, pos: V4, deg: i16) -> bool {
        self.world.borrow_mut().camera_deg(pos, deg)
    }
    fn camera_transparency(&mut self, pos: V4, width: F, height: F, far: F, len: F) -> F {
        self.world.borrow_mut().camera_transparency(pos, width, height, far, len)
    }
    fn anim_set(&mut self, who: usize, slot: AnmSlot, name: &str) {
        self.world.borrow_mut().anim_set(who, slot, name)
    }
    fn anim_frame(&mut self, who: usize, slot: AnmSlot) -> u16 {
        self.world.borrow_mut().anim_frame(who, slot)
    }
    fn anim_forward(&mut self, who: usize, slot: AnmSlot, step: u16) -> i16 {
        self.world.borrow_mut().anim_forward(who, slot, step)
    }
    fn anim_notes(&mut self, who: usize, slot: AnmSlot) -> Vec<Note> {
        self.world.borrow_mut().anim_notes(who, slot)
    }
}

impl<W: KiteWorld + NaviWorld + ?Sized> KiteWorld for Host<'_, '_, '_, W> {
    fn out(&mut self, o: Out) {
        self.world.borrow_mut().out(o)
    }
    fn camera_type(&mut self) -> i32 {
        self.world.borrow_mut().camera_type()
    }
    fn camera_id(&mut self) -> i32 {
        self.world.borrow_mut().camera_id()
    }
    fn camera_rot(&mut self) -> V4 {
        self.world.borrow_mut().camera_rot()
    }
    fn camera_reset(&mut self) -> Option<F> {
        self.world.borrow_mut().camera_reset()
    }
    fn clear_camera_reset(&mut self) {
        self.world.borrow_mut().clear_camera_reset()
    }
    fn camera_set_eye_level(&mut self, pos_eye: V4, angle: &mut V4) {
        self.world.borrow_mut().camera_set_eye_level(pos_eye, angle)
    }
    fn camera_set_manual(&mut self, pos_view: V4) {
        self.world.borrow_mut().camera_set_manual(pos_view)
    }
    fn camera_set(&mut self) {
        self.world.borrow_mut().camera_set()
    }
    fn hit_check(
        &mut self,
        me: usize,
        ch: &Char,
        hit: &mut CharHit,
        now_speed: F,
        manual: bool,
        move_pos: &mut V4,
    ) -> i32 {
        self.world.borrow_mut().hit_check(me, ch, hit, now_speed, manual, move_pos)
    }
    fn hit_result(&mut self) -> Option<u32> {
        self.world.borrow_mut().hit_result()
    }
    fn trans_mode(&mut self) -> bool {
        self.world.borrow_mut().trans_mode()
    }
    fn trans_center(&mut self) -> V4 {
        self.world.borrow_mut().trans_center()
    }
    fn draw(&mut self, me: usize, pos: V4, set_transparency: F, transparency: &mut F) -> bool {
        self.world.borrow_mut().draw(me, pos, set_transparency, transparency)
    }
    fn skill_check(&mut self, scene: &Scene, ch: usize) -> i32 {
        self.world.borrow_mut().skill_check(scene, ch)
    }
    fn gate_hacking_out(&mut self, me: usize, ch: &mut Char, p: &mut Player, spc: &mut Spc) {
        self.world.borrow_mut().gate_hacking_out(me, ch, p, spc)
    }
}

// ---------------------------------------------------------------------------
// Targets

/// `ccPlayer::SetTargetDist()` (gcmn 0x0059ab80): the ground distance from
/// Kite to `targetChar` less its width, truncated (`distTg`); a target
/// down is dropped. Nothing for a target off the lists.
pub fn set_target_dist(scene: &mut Scene, me: usize, p: &mut Player) {
    let Some(tg) = scene.chars[me].target_char else { return };
    if !scene.listed(tg) {
        return;
    }
    if scene.chars[tg].dead() {
        scene.chars[me].target_char = None;
        return;
    }
    let a = scene.chars[me].pos;
    let b = &scene.chars[tg];
    let d = geom::vsub(a, b.pos);
    let v = [d[0], d[1], 0, ONE];
    p.dist_tg = fptosi(sub(geom::sqrtf(geom::dot(v, v)), b.base().width));
}

/// `ccPlayer::SetTargetDirc()` (gcmn 0x0059aa90): the heading to
/// `targetChar` (`dircTg`, `atan2f(dx, -dy)`), which he turns to unless
/// the camera is the eye view; his AI in manual mode takes it too
/// (`ccAI::SetDircZ` 0x00581500). A target down is dropped; none, or
/// himself, changes nothing.
pub fn set_target_dirc(cx: &mut Cx, me: usize) {
    let Some(tg) = cx.scene.chars[me].target_char else { return };
    if !cx.scene.listed(tg) || tg == me {
        return;
    }
    if cx.scene.chars[tg].dead() {
        cx.scene.chars[me].target_char = None;
        return;
    }
    let t = cx.scene.chars[tg].pos;
    let m = cx.scene.chars[me].pos;
    let dy = sub(t[1], m[1]);
    cx.p.dirc_tg = geom::atan2f(sub(t[0], m[0]), mul(geom::MINUS_ONE, dy));
    if cx.w.camera_type() != 1 {
        let d = cx.p.dirc_tg;
        cx.spc(me).dirc[2] = d;
    }
    if cx.manual(me) {
        let v = geom::rad2deg(cx.spc(me).dirc[2]) as u16;
        set_dirc_z(cx, me, v);
    }
}

/// `ccAI::SetDircZ(v)` (gcmn 0x00581500) of Kite's AI: its `gDeg` and his
/// heading.
fn set_dirc_z(cx: &mut Cx, me: usize, v: u16) {
    if let Some(a) = cx.crew.ais.get_mut(&me) {
        a.g_deg = v;
    }
    cx.spc(me).dirc[2] = geom::deg2rad(v as i16);
}

// ---------------------------------------------------------------------------
// The weapon's effects

/// `ccSpcChar::StartArmsEffect(sid)` (gcmn 0x0059e530): an art or the
/// normal attack with an element lights the weapon (`armsEffectSW` 1 and
/// the particles on the job's hands); one without ends it
/// ([`skill::start_arms_effect`]).
fn start_arms_effect(cx: &mut Cx, me: usize, sid: i32) {
    let t = cx.t;
    for [a, b, attr] in skill::start_arms_effect(t, cx.ch(me), sid) {
        cx.out(Out::ArmsParticles { a, b, attr });
    }
}

// ---------------------------------------------------------------------------
// AnimCtrl

/// `ccPlayer::AnimCtrl()` (gcmn 0x005993c0), once a frame from [`main`]
/// after the camera: the act from the skill, the stick and the
/// conditions, and the act's animation.
///
/// ```text
/// SetTargetDist; a normal attack aimed at the wrong side ends
/// paused while moving: standing (2 in a town, else 0)
/// skillStatus bit 0 (a skill starts): the normal attack swings (15, or 16
///     after a first), a spell or art plays 17-21 by its type bits, any
///     other skill 24; the target turned to, the expected damage announced
/// bit 1 (it runs): the combo (attack 1-4: 30 frames to chain, a third
///     swing when the target is within armsRange)
/// the fidget: 451 frames standing (rand() % 60 again)
/// the animation ended: the act that follows (the jump table 0x006f07e0)
/// down: the fade to a ghost (dead 3 -> 4), a revival's fade in (dead 5)
/// starting and stopping, walking and running (6, 5)
/// SetAnm on a new act; frameSpd; _AnimateForward; the arrival (13) and
/// the gate (12) fades; the notes (CheckNote)
/// ```
pub fn anim_ctrl(cx: &mut Cx, me: usize) {
    set_target_dist(cx.scene, me, cx.p);
    let ch = &cx.scene.chars[me];
    if ch.skill_id == 1
        && let Some(tg) = ch.target_char
        && cx.scene.listed(tg)
    {
        let ty = |cx: &Cx| cx.scene.chars[tg].ty() as u32;
        if cx.cond(me, cond::CONFUSION) == 0 && cx.cond(me, cond::CHARM) == 0 && ty(cx) & 0xe0 == 0 {
            cx.skill_request(me, None, 0);
        }
        if cx.cond(me, cond::CHARM) != 0 && ty(cx) & 0x0700_000f == 0 {
            cx.skill_request(me, None, 0);
        }
        if cx.cond(me, cond::CONFUSION) != 0 && ty(cx) & 0x0700_00ef == 0 {
            cx.skill_request(me, None, 0);
        }
    }
    if cx.flags(me) & PAUSE != 0 && matches!(cx.act(me), act::WALK | act::RUN) {
        let a = if cx.game.area == 0 { act::IDLE } else { act::IDLE_FIELD };
        cx.set_act(me, a);
        cx.clear(me, RESTRAINT);
        cx.ch(me).spc_char.attack = 0;
        cx.set(me, STOP);
        cx.clear(me, MOVE | RUN | TRAJECTORY);
    }
    skill_state(cx, me);
    let c = &cx.scene.chars[me];
    if c.skill_status & 2 != 0 && c.skill_id == 1 {
        let s = cx.spc(me);
        s.atk_anm_cnt = s.atk_anm_cnt.wrapping_add(1);
    }
    // The fidget.
    let calm = cx.w.camera_type() != 1
        && !cx.manual(me)
        && cx.cond(me, cond::HOLD) == 0
        && cx.cond(me, cond::SLEEP) == 0
        && cx.cond(me, cond::PARALYSIS) == 0
        && cx.cond(me, cond::DEAD) == 0
        && cx.act(me) < 4
        && cx.game.in_battle != 1;
    if calm {
        let s = cx.spc(me);
        s.react_cnt = s.react_cnt.wrapping_add(1);
        if s.react_cnt >= 451 {
            let r = (cx.rng.rand() % 60) as i16;
            cx.spc(me).react_cnt = r;
            match cx.act(me) {
                act::IDLE => cx.set_act(me, act::FIDGET),
                act::IDLE_FIELD => cx.set_act(me, act::FIDGET_FIELD),
                _ => {}
            }
        }
    } else {
        cx.spc(me).react_cnt = 0;
    }
    if cx.scene.chars[me].anm_flag != 0 {
        anim_ended(cx, me);
    }
    down_and_up(cx, me);
    start_stop(cx, me);
    let f = cx.flags(me);
    if f & STOP == 0 && f & MOVE != 0 {
        if f & RUN != 0 {
            if cx.act(me) == act::WALK {
                cx.set_act(me, act::RUN);
            }
        } else if cx.act(me) == act::RUN {
            cx.set_act(me, act::WALK);
        }
    }
    let moving = cx.flags(me) & MOVE != 0;
    let s = cx.spc(me);
    s.walk_run_cnt = if moving { s.walk_run_cnt.wrapping_add(1) } else { 0 };
    if cx.manual(me) && cx.act(me) == act::IDLE_FIELD {
        cx.set_act(me, act::FIDGET_FIELD);
    }
    let a = cx.act(me);
    if cx.scene.chars[me].spc_char.act_num_old != a {
        let name = cx.kt.anim(a).to_string();
        cx.w.anim_set(me, AnmSlot::Main, &name);
    }
    // The walk and run clips play faster with the lean.
    let speed_rate = cx.spc(me).speed_rate;
    let speed_value = cx.scene.chars[me].cond.speed_value;
    let spd = |k: F| geom::fptoui(mul(0x4380_0000, mul(mul(k, speed_rate), speed_value))) as u16;
    cx.p.frame_spd = match a {
        act::RUN => spd(0x3fb0_0000),
        act::WALK => spd(0x3f8c_cccd),
        _ => 256,
    };
    let step = cx.p.frame_spd;
    let ended = cx.w.anim_forward(me, AnmSlot::Main, step);
    cx.ch(me).anm_flag = ended;
    match cx.act(me) {
        act::ARRIVE => arrive(cx, me),
        act::GATE_OUT => gate_out(cx, me),
        _ => {}
    }
    let a = cx.act(me);
    cx.set_act_old(me, a);
    for n in cx.w.anim_notes(me, AnmSlot::Main) {
        check_note(cx, me, n);
    }
    let s = cx.crew.spc.entry(me).or_default();
    if s.transfer_lag > 0 {
        s.transfer_lag -= 1;
    } else {
        s.act_cnt = s.act_cnt.wrapping_add(1);
    }
    let p = &mut *cx.p;
    if p.act_one_two_cnt > 0 {
        p.act_one_two_cnt -= 1;
        if p.act_one_two_cnt <= 0 {
            p.act_one_two_cnt = 0;
            cx.ch(me).spc_char.attack = 0;
        }
    }
    to_spc(cx.scene, cx.crew, me);
}

/// The skill's state (`skillStatus`): bit 0 a skill was requested and
/// starts now, bit 1 it runs.
fn skill_state(cx: &mut Cx, me: usize) {
    let ss = cx.scene.chars[me].skill_status;
    if ss & 1 != 0 {
        let sid = cx.scene.chars[me].skill_id;
        let aimed = sid == 180 || cx.scene.chars[me].target_char.is_some_and(|t| cx.scene.listed(t));
        if !aimed {
            cx.skill_request(me, None, 0);
            cx.ch(me).spc_char.attack = 0;
            return;
        }
        if sid == 1 {
            if cx.act(me) >= 7 {
                return;
            }
            let c = cx.ch(me);
            c.skill_status = (c.skill_status ^ 1) | 2;
            if c.spc_char.attack == 1 {
                c.spc_char.act_num = act::ATTACK2;
                c.spc_char.attack = 2;
            } else {
                c.spc_char.act_num = act::ATTACK1;
                c.spc_char.attack = 1;
            }
            c.anm_flag = 0;
            cx.spc(me).atk_anm_cnt = 0;
            swing_flags(cx, me);
            set_target_dirc(cx, me);
            let sid = i32::from(cx.scene.chars[me].skill_id);
            cx.out(Out::ArmsEffectColor { sid });
            cx.announce(me, 1);
            return;
        }
        let ty = cx.t.skill(i32::from(sid)).map_or(0, |s| s.ty);
        let a = [(0x100, 17), (0x200, 18), (0x400, 19), (0x800, 20), (0x1000, 21)]
            .iter()
            .find(|&&(bit, _)| ty & bit != 0)
            .map(|&(_, a)| a);
        let Some(a) = a else {
            if cx.scene.chars[me].spc_char.act_num_old == act::HACK {
                return;
            }
            cx.set_act(me, act::HACK);
            cx.ch(me).anm_flag = 0;
            cx.set(me, RESTRAINT);
            cx.clear(me, TRAJECTORY);
            return;
        };
        cx.set_act(me, a);
        let c = cx.ch(me);
        c.skill_status = (c.skill_status ^ 1) | 2;
        cx.set_act_old(me, -1);
        cx.spc(me).atk_anm_cnt = 0;
        cx.ch(me).anm_flag = 0;
        swing_flags(cx, me);
        set_target_dirc(cx, me);
        let c = &cx.scene.chars[me];
        let (sid, item) = (i32::from(c.skill_id), i32::from(c.skill_status & 8 != 0));
        cx.out(Out::SkillStart { sid, item });
        cx.out(Out::ArmsEffectColor { sid });
        start_arms_effect(cx, me, sid);
        if cx.manual(me) || cx.cond(me, cond::CHARM) != 0 || cx.cond(me, cond::CONFUSION) != 0 {
            return;
        }
        let ct = skill::check_type_of(cx.t, i32::from(cx.scene.chars[me].skill_id));
        if ct != 0 && ct != 1 {
            return;
        }
        let sid = i32::from(cx.scene.chars[me].skill_id);
        cx.announce(me, sid);
    } else if ss & 2 != 0 {
        if cx.scene.chars[me].skill_id != 1 || cx.cond(me, cond::SLEEP) != 0 || cx.cond(me, cond::PARALYSIS) != 0 {
            return;
        }
        let attack = cx.scene.chars[me].spc_char.attack;
        if attack < 3 {
            // The swing ended with no second one asked for: the attack is
            // over, and a new one can chain for 30 frames.
            let over = match cx.act(me) {
                act::IDLE_FIELD => true,
                act::ATTACK1 | act::ATTACK2 => cx.scene.chars[me].anm_flag != 0,
                _ => false,
            };
            if over {
                cx.skill_request(me, None, 0);
                cx.p.act_one_two_cnt = 30;
            }
        } else if attack == 3 {
            if cx.cond(me, cond::DEAD) != 0 {
                cx.skill_request(me, None, 0);
                cx.ch(me).spc_char.attack = 0;
            } else if cx.act(me) < 5 && cx.spc(me).atk_anm_cnt >= 50 {
                cx.ch(me).spc_char.attack = 4;
            }
        } else if attack == 4 && cx.act(me) < 7 {
            let tc = cx.scene.chars[me].target_char;
            let d = party_ai::distance_to_target(cx.scene, me, tc);
            let a = &cx.crew.ais[&me];
            let arms = cx.t.ai_params.get(a.param).map_or(0, |r| r.arms_range);
            let near = if geom::eq(party_ai::F_MINUS_ONE, d) { le(a.dist_tg, arms) } else { le(d, arms) };
            if !near {
                cx.skill_request(me, None, 0);
                cx.ch(me).spc_char.attack = 0;
                return;
            }
            cx.set_act(me, act::ATTACK1);
            cx.spc(me).atk_anm_cnt = 0;
            cx.ch(me).anm_flag = 0;
            cx.set(me, RESTRAINT | TRAJECTORY);
            cx.ch(me).spc_char.attack = 2;
            set_target_dirc(cx, me);
            cx.announce(me, 1);
        }
    } else if ss == 0 && cx.scene.chars[me].skill_id == 1 {
        cx.ch(me).skill_id = 0;
        cx.clear(me, RESTRAINT | TRAJECTORY);
    }
}

/// A swing or cast holds him where he stands: `restraintSW` and
/// `trajectorySW` set, `moveFlag` and `runFlag` clear, `stopFlag` set.
fn swing_flags(cx: &mut Cx, me: usize) {
    cx.set(me, RESTRAINT | TRAJECTORY | STOP);
    cx.clear(me, MOVE | RUN);
}

/// The act that follows one whose animation ended (the jump table at gcmn
/// 0x006f07e0).
fn anim_ended(cx: &mut Cx, me: usize) {
    match cx.act(me) {
        0 | 2 | 5 | 9..=11 => {}
        1 => {
            cx.set_act(me, act::IDLE);
            cx.clear(me, RESTRAINT);
        }
        3 => {
            cx.set_act(me, act::FIDGET_LOOP);
            cx.clear(me, RESTRAINT);
        }
        12 => {
            cx.set_act(me, act::HELD);
            cx.set(me, RESTRAINT);
            cx.clear(me, MOVE | TRAJECTORY);
        }
        13 => {
            cx.set_act(me, act::IDLE);
            cx.set_act_old(me, act::IDLE);
            cx.clear(me, RESTRAINT);
            if !cx.manual(me) {
                entry_cmnd(cx.scene, me);
            }
            if cx.cond(me, cond::DEAD) == 0 {
                hit_switch(cx, me, true);
            }
        }
        a @ 15..=21 => {
            if cx.flags(me) & MOVE != 0 {
                cx.set_act(me, act::RUN);
            }
            if cx.flags(me) & STOP != 0 {
                cx.set_act(me, act::IDLE_FIELD);
            }
            if a >= 19 {
                cx.ch(me).spc_char.arms_effect_sw = 0;
            }
            cx.clear(me, RESTRAINT | TRAJECTORY);
        }
        24 => cx.set_act(me, act::HACK_END),
        _ => {
            cx.set_act(me, act::IDLE_FIELD);
            cx.clear(me, RESTRAINT);
        }
    }
}

/// `ccCharHit::HitEnable()` / `HitDisable()` on his body.
fn hit_switch(cx: &mut Cx, me: usize, on: bool) {
    let s = cx.crew.spc.entry(me).or_default();
    cx.w.hit_switch(me, &mut s.body_hit, on);
    cx.scene.chars[me].spc_char.hit_enabled = on;
}

/// `ccEntryCmnd(ch)` (gcmn 0x00519630): onto the end of the command list
/// its type names (the party's for 0x7, the foes' for 0xe0, else the
/// objects'), unless already on one.
pub fn entry_cmnd(scene: &mut Scene, ch: usize) {
    if scene.listed(ch) {
        return;
    }
    let ty = scene.chars[ch].ty();
    if ty & 7 != 0 {
        scene.pc_list.push(ch);
    } else if ty & 0xe0 != 0 {
        scene.ene_list.push(ch);
    } else {
        scene.obj_list.push(ch);
    }
}

/// Down and up: lying down (`dead` 3, act 10) fades out over 50 frames
/// into a ghost (`dead` 4, act 2), which fades to half over 30 frames; a
/// revival (`dead` 5) fades back in from frame 18 to 78.
fn down_and_up(cx: &mut Cx, me: usize) {
    let dead = cx.cond(me, cond::DEAD);
    if dead == 3 || dead == 4 {
        match cx.act(me) {
            6 | 5 | 2 | 0 => {
                let cloak = if dead == 4 && cx.flags(me) & GHOST == 0 {
                    let cnt = cx.scene.chars[me].spc_char.cnt;
                    if cnt < 30 {
                        cx.ch(me).spc_char.cnt = cnt.wrapping_add(1);
                        mul(0x3f00_0000, div(from_int(cnt), 0x41f0_0000))
                    } else {
                        cx.clear(me, RESTRAINT);
                        cx.ch(me).spc_char.cnt = 0;
                        cx.set(me, GHOST);
                        cx.out(Out::ClearArmsEffect);
                        0x3f00_0000
                    }
                } else {
                    0x3f00_0000
                };
                cx.ch(me).spc_char.cloak = cloak;
            }
            10 => {
                let cnt = cx.scene.chars[me].spc_char.cnt;
                let cloak = if cnt < 50 {
                    cx.ch(me).spc_char.cnt = cnt.wrapping_add(1);
                    div(from_int(50 - cnt), 0x4248_0000)
                } else {
                    cx.ch(me).anm_flag = 0;
                    cx.set_act(me, act::IDLE);
                    cx.set_act_old(me, -1);
                    cx.set(me, RESTRAINT);
                    cx.clear(me, MOVE | TRAJECTORY);
                    cx.ch(me).cond.v[cond::DEAD] = 4;
                    cx.ch(me).spc_char.cnt = 0;
                    if cx.party.annihilated(cx.scene) {
                        cx.clear(me, DISP);
                    }
                    0
                };
                cx.ch(me).spc_char.cloak = cloak;
            }
            _ => {}
        }
    }
    if cx.cond(me, cond::DEAD) == 5 {
        let cnt = cx.scene.chars[me].spc_char.cnt;
        let cloak = if cnt < 18 {
            cx.ch(me).spc_char.cnt = cnt + 1;
            0
        } else if cnt < 78 {
            cx.ch(me).spc_char.cnt = cnt + 1;
            div(from_int(cnt), 0x429c_0000)
        } else {
            cx.clear(me, RESTRAINT);
            cx.ch(me).spc_char.cnt = 0;
            cx.clear(me, GHOST);
            cx.ch(me).cond.v[cond::DEAD] = 0;
            cx.out(Out::ClearArmsEffect);
            ONE
        };
        cx.ch(me).spc_char.cloak = cloak;
    }
}

/// Starting to walk (act 6; `stopFlag` clears) and stopping (standing: 2
/// in a town, a ghost or under manual control, else 0).
fn start_stop(cx: &mut Cx, me: usize) {
    let f = cx.flags(me);
    let (stop, moving) = (f & STOP != 0, f & MOVE != 0);
    if stop && moving {
        cx.clear(me, STOP);
        if cx.flags(me) & RESTRAINT != 0 {
            cx.set_act(me, act::RUN);
        }
        cx.set_act(me, act::WALK);
    } else if !stop && !moving {
        cx.set(me, STOP);
        cx.clear(me, RUN);
        if matches!(cx.act(me), act::WALK | act::RUN) {
            let town = cx.game.area == 0;
            let a = if town || cx.manual(me) || cx.cond(me, cond::DEAD) == 4 { act::IDLE } else { act::IDLE_FIELD };
            cx.set_act(me, a);
        }
    }
}

/// Act 13, arriving: nothing drawn until frame 50 (20 by warp), the
/// arrival's effect at 30 (0), a fade in to 70 (40), then the act ends;
/// a ghost at half. From Mutation on a warp's fade runs to 50 and the act
/// ends after 60.
fn arrive(cx: &mut Cx, me: usize) {
    let warp = cx.input.warp;
    let later = cx.t.volume != piney_data::volume::Volume::Inf;
    let (eff, s, full, e) = match (warp, later) {
        (true, false) => (0, 20, 40, 40),
        (true, true) => (0, 20, 50, 60),
        _ => (30, 50, 70, 70),
    };
    let n = cx.spc(me).act_cnt;
    let ghost = cx.flags(me) & GHOST != 0;
    let mut cloak = 0;
    if n == eff {
        cx.out(if warp { Out::WarpTransfer } else { Out::Transfer });
    } else if n > e {
        cx.ch(me).anm_flag = 1;
        cloak = if ghost { 0x3f00_0000 } else { ONE };
    } else if n > s {
        cloak = div(from_int(i32::from(n - s)), from_int(i32::from(full - s)));
        if !le(cloak, ONE) {
            cloak = ONE;
        }
        if ghost {
            cloak = mul(cloak, 0x3f00_0000);
        }
    }
    cx.ch(me).spc_char.cloak = if n < s { 0 } else { cloak };
}

/// Act 12, leaving through a gate: the effect and his body off the
/// collision at frame 1, a fade out from 22 to 41, the act ends at 141.
fn gate_out(cx: &mut Cx, me: usize) {
    let n = cx.spc(me).act_cnt;
    let mut cloak = 0;
    if n == 1 {
        cx.out(Out::Transfer);
        hit_switch(cx, me, false);
    } else if n >= 141 {
        cx.ch(me).anm_flag = 1;
    } else if n >= 22 {
        cloak = div(from_int(41 - i32::from(n)), 0x41a0_0000);
        if lt(cloak, 0) {
            cloak = 0;
        }
    }
    cx.ch(me).spc_char.cloak = if n < 22 { ONE } else { cloak };
}

/// `ccPlayer::CheckNote(note)` (gcmn 0x0059c300), through
/// `ccPlayerCheckNote` (0x0059c550), for each note his animation passed:
/// 1 and 2 are footsteps (with dust while running), 0x8005 the normal
/// attack's or art's hit ([`flow::player_attack_note`]), whose
/// `EntryAffect` on the target is applied at once; the rest nothing.
pub fn check_note(cx: &mut Cx, me: usize, note: Note) {
    match note.event {
        1 | 2 => {
            if cx.flags(me) & DISP != 0 {
                cx.out(Out::Sound { param: note.param });
            }
            if cx.act(me) == act::RUN {
                let speed = cx.spc(me).speed;
                cx.out(Out::PawSmoke { speed });
            }
        }
        0x8005 => {
            let mut ev = Events::new();
            flow::player_attack_note(cx.t, cx.scene, me, &mut cx.p.dist_tg, cx.rng, cx.env, &mut ev);
            cx.rules(me, ev);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Main

/// `ccPlayer::ResultOfConditions()` (gcmn 0x0059c270): asleep, paralysed,
/// held or down (`dead` 2, 3), the stick does nothing (`inactiveSW`).
pub fn result_of_conditions(ch: &Char, p: &mut Player) {
    let c = &ch.cond;
    let d = c[cond::DEAD];
    p.inactive = c[cond::SLEEP] != 0 || c[cond::PARALYSIS] != 0 || c[cond::HOLD] != 0 || d == 2 || d == 3;
}

/// `ccPlayer::PadLeverPower(p)` (gcmn 0x005992b0): the stick's strength as
/// he moves by it. Below 64 nothing; 64-127 eased to 32-126; from 128 as
/// it is. The first four frames of a lean give nothing while a target is
/// up (`cmndTarget`).
pub fn pad_lever_power(p: &mut Player, power: F, cmnd_target: bool) -> F {
    let v = fptosi(power) as i16;
    let r: i32 = if v < 64 {
        p.target_count = 0;
        0
    } else {
        let mut r = if v < 128 { (i32::from(v) - 64) * 3 / 2 + 32 } else { i32::from(v) };
        if p.target_count < 4 {
            p.target_count += 1;
            if cmnd_target {
                r = 0;
            }
        }
        r
    };
    from_int(r)
}

/// 255.0, 230.0, 140.0, 1.3, 0.4, 5.0.
const K255: F = 0x437f_0000;
const RUN_POWER: F = 0x4366_0000;
const K140: F = 0x430c_0000;
const WALK_MAX: F = 0x3fa6_6666;
const K04: F = 0x3ecc_cccd;
const K5: F = 0x40a0_0000;

/// `ccPlayer::ControlMove()` (gcmn 0x00598af0): the move from the left
/// stick. Held, asleep or paralysed he does not move (returns 0). By
/// `ctrlType`:
///
/// - 0: nothing while a skill (or the first swing) or `pauseSW` /
///   `restraintSW` holds him; leaning ends a running normal attack, walks
///   (`power / 140`, at most 1.3, times `speedValue * tsp`) or runs (past
///   230, `power / 255 * speedValue * speed`) toward the stick against the
///   camera's heading (straight ahead while a camera reset runs and the
///   stick stays within 2047 of it);
/// - 1: steering: the stick's x turns him (`5 (|sin| - 0.4) power` a frame
///   past 0.4), its y drives him forward or back at `|power cos| / 255`,
///   `pos` moved at once as well as `movePos`;
/// - other: backwards along the stick.
///
/// He faces the move's heading unless the camera is the eye view.
pub fn control_move(cx: &mut Cx, me: usize) -> i32 {
    let r = control_move_body(cx, me);
    to_spc(cx.scene, cx.crew, me);
    r
}

fn control_move_body(cx: &mut Cx, me: usize) -> i32 {
    if cx.cond(me, cond::HOLD) != 0 || cx.cond(me, cond::SLEEP) != 0 || cx.cond(me, cond::PARALYSIS) != 0 {
        cx.clear(me, MOVE);
        return 0;
    }
    let mut s0 = (i32::from(geom::rad2deg(add(PI, cx.spc(me).dirc[2]))) - 32768) as i16;
    let pow_l = from_int(i32::from(cx.input.pad.pow_l));
    let mut power = pad_lever_power(cx.p, pow_l, cx.input.cmnd_target);
    if cx.p.inactive {
        power = 0;
    }
    cx.spc(me).now_speed = 0;
    let dirc_l = cx.input.pad.dirc_l;
    let speed_value = cx.scene.chars[me].cond.speed_value;
    match cx.p.ctrl_type {
        0 => 'walk: {
            cx.spc(me).speed_rate = div(power, K255);
            let c = &cx.scene.chars[me];
            let (sid, f) = (c.skill_id, c.spc_char.flags);
            if sid >= 2 || (sid == 1 && c.spc_char.attack == 1) || (sid == 0 && f & (PAUSE | RESTRAINT) != 0) {
                break 'walk;
            }
            if geom::eq(power, 0) {
                cx.clear(me, MOVE);
                break 'walk;
            }
            if cx.scene.chars[me].skill_status & 2 != 0 {
                cx.skill_request(me, None, 0);
            }
            cx.set(me, MOVE);
            let run = !le(power, RUN_POWER);
            if run {
                cx.set(me, RUN);
            } else {
                cx.clear(me, RUN);
            }
            let mut dirc = dirc_l;
            if let Some(reset) = cx.w.camera_reset() {
                let d = geom::rad2deg(reset).wrapping_sub(geom::rad2deg(dirc));
                if (-2047..2048).contains(&d) {
                    dirc = PI;
                } else {
                    cx.w.clear_camera_reset();
                }
            }
            let s = geom::rad2deg(add(PI, dirc));
            let rot = cx.w.camera_rot();
            s0 = geom::rad2deg(rot[2]).wrapping_sub(s);
            let heading = geom::deg2rad(s0);
            let tsp = cx.kt.tsp;
            let s = cx.spc(me);
            let v = if !run {
                let mut rate = div(power, K140);
                if !le(rate, WALK_MAX) {
                    rate = WALK_MAX;
                }
                s.speed_rate = rate;
                mul(s.speed_rate, mul(speed_value, tsp))
            } else {
                mul(s.speed_rate, mul(speed_value, s.speed))
            };
            s.now_speed = v;
            s.move_pos[0] = mul(v, geom::sinf(heading));
            s.move_pos[1] = mul(neg(v), geom::cosf(heading));
        }
        1 => 'steer: {
            let c = &cx.scene.chars[me];
            let (sid, f) = (c.skill_id, c.spc_char.flags);
            if sid >= 2 || (sid == 0 && f & (PAUSE | RESTRAINT) != 0) {
                break 'steer;
            }
            if geom::eq(power, 0) {
                cx.clear(me, MOVE);
                break 'steer;
            }
            if cx.scene.chars[me].skill_status != 0 {
                cx.ch(me).skill_status = 0;
                cx.skill_request(me, None, 0);
            }
            cx.set(me, MOVE);
            let sin = geom::sinf(dirc_l);
            // fptodp, fabs, _dpflt against 0.4f as a double.
            let turn = if f64::from(f32::from_bits(sin)).abs() < f64::from(f32::from_bits(K04)) {
                0
            } else if lt(sin, 0) {
                mul(mul(K5, add(K04, sin)), power)
            } else {
                mul(mul(K5, sub(sin, K04)), power)
            };
            let drive = mul(power, neg(geom::cosf(dirc_l)));
            // fptodp, fabs, dpdiv by 255.0, dptofp.
            let rate = (f64::from(f32::from_bits(drive)).abs() / 255.0) as f32;
            cx.spc(me).speed_rate = rate.to_bits();
            s0 = (i32::from(s0) + fptosi(turn)) as i16;
            let heading = geom::deg2rad(s0);
            let drive = div(drive, K255);
            let s = cx.spc(me);
            let speed = s.speed;
            s.move_pos[0] = mul(mul(speed_value, speed), mul(drive, geom::sinf(heading)));
            s.move_pos[1] = mul(mul(speed_value, neg(speed)), mul(drive, geom::cosf(heading)));
            let dx = mul(mul(speed_value, speed), mul(drive, geom::sinf(heading)));
            let pos = &mut cx.scene.chars[me].pos;
            pos[0] = add(pos[0], dx);
            let dy = mul(mul(speed_value, speed), mul(drive, geom::cosf(heading)));
            pos[1] = sub(pos[1], dy);
        }
        _ => {
            let s = cx.spc(me);
            let speed = s.speed;
            s.move_pos[0] = mul(mul(speed_value, neg(speed)), mul(power, geom::sinf(dirc_l)));
            s.move_pos[1] = mul(mul(speed_value, neg(speed)), mul(power, geom::cosf(dirc_l)));
            let dx = mul(mul(speed_value, speed), mul(power, geom::sinf(dirc_l)));
            let pos = &mut cx.scene.chars[me].pos;
            pos[0] = sub(pos[0], dx);
            let dy = mul(mul(speed_value, speed), mul(power, geom::cosf(dirc_l)));
            pos[1] = sub(pos[1], dy);
        }
    }
    if cx.w.camera_type() != 1 && !geom::eq(power, 0) {
        cx.spc(me).dirc[2] = geom::deg2rad(s0);
    }
    1
}

/// `ccPlayer::CollisionTest()` (gcmn 0x0059b340): stood on the floor
/// under him (`ccLandHitCheck(pos, 0x20000001)`, `pos.z` and `posP.z`);
/// ground with attribute bit 0x80000 is a way into another area
/// (`WORLD_MAN::Enter`) unless he is leaving a dungeon.
fn collision_test(cx: &mut Cx, me: usize) {
    let pos = cx.scene.chars[me].pos;
    let z = cx.w.land(pos, 0x2000_0001);
    let c = cx.ch(me);
    c.pos[2] = z;
    c.pos_p[2] = z;
    if !cx.input.dne
        && let Some(attr) = cx.w.hit_result()
        && attr & 0x80000 != 0
    {
        let pos = cx.scene.chars[me].pos;
        cx.out(Out::Enter { pos });
    }
}

/// `ccPlayer::MapLoopAdjustPos()` (gcmn 0x0059b3c0): the field's streaming
/// follows the move (`WORLD_MAN::AddCenter`), and a position past the
/// map's edge wraps round ([`w2m_pos`]) and is stood on the ground again.
pub fn map_loop_adjust_pos(cx: &mut Cx, me: usize) {
    let mp = cx.spc(me).move_pos;
    cx.out(Out::AddCenter { x: mp[0], y: mp[1] });
    let (pos, wrapped) = w2m_pos(&cx.input.bounds, cx.scene.chars[me].pos);
    cx.ch(me).pos = pos;
    if wrapped {
        collision_test(cx, me);
    }
}

/// `ccPlayer::CameraPosCalc()` (gcmn 0x0059ba20): the camera's target and
/// his eyes 140 above his feet, the eye view's heading his. Nothing while
/// `cameraFlag` keeps the camera off him.
fn camera_pos_calc(cx: &mut Cx, me: usize) {
    if cx.p.camera_flag {
        return;
    }
    let pos = cx.scene.chars[me].pos;
    let head = [pos[0], pos[1], add(pos[2], K140), ONE];
    cx.p.pos_view = head;
    cx.p.pos_eye = head;
    cx.p.angle[2] = cx.spc(me).dirc[2];
}

/// `ccPlayer::CameraPosSet()` (gcmn 0x0059bac0): the eye view from his
/// eyes (his heading follows its turn and `lostHeadFlag` hides him while
/// the field camera is active), else the camera on his head; then the
/// view (`cameraSet`).
fn camera_pos_set(cx: &mut Cx, me: usize) {
    if !cx.p.camera_flag {
        if cx.w.camera_type() == 1 {
            let eye = cx.p.pos_eye;
            cx.w.camera_set_eye_level(eye, &mut cx.p.angle);
            if cx.w.camera_id() == 1 {
                cx.set(me, flag::LOST_HEAD);
                let z = cx.p.angle[2];
                cx.spc(me).dirc[2] = z;
            } else {
                cx.clear(me, flag::LOST_HEAD);
            }
        } else {
            let v = cx.p.pos_view;
            cx.w.camera_set_manual(v);
            cx.clear(me, flag::LOST_HEAD);
        }
    }
    cx.w.camera_set();
}

/// The stats a buff or debuff times out on (`ConditionTimeCount`): pEva,
/// mEva and the tolerances are never timed.
const TIMED: [usize; 12] = [0, 1, 2, 4, 5, 6, 8, 9, 10, 11, 12, 13];

/// `ccChar::CalcReal(flag)` (gcmn 0x0056ba30) on Kite, with the timers'
/// affects applied where the game applies them: for a party member with
/// `flag` 0, `maxHP`/`maxSP`, then [`condition_time_count`], then the rest
/// of [`crate::chara::calc_real`] (the stats do not enter the timers or
/// what the affects do), and `DispConditionEffect` outside the Root Town.
pub fn calc_real(cx: &mut Cx, me: usize, flag: i32) {
    let mut ev = Events::new();
    let pc = cx.scene.chars[me].is_pc();
    if pc && flag == 0 {
        let c = &mut cx.scene.chars[me];
        if let Some((hp, sp)) = c.spc().map(|p| (p.max_hp, p.max_sp)) {
            c.max_hp = hp;
            c.max_sp = sp;
        }
        condition_time_count(cx, me);
        crate::chara::calc_real(cx.t, &mut cx.scene.chars[me], 1, cx.env, &mut ev);
        if cx.env.area != 0 {
            ev.push(Event::DispCondition(Who::Me));
        }
    } else {
        crate::chara::calc_real(cx.t, &mut cx.scene.chars[me], flag, cx.env, &mut ev);
    }
    cx.rules(me, ev);
}

/// `ccChar::ConditionTimeCount()` (gcmn 0x0056bfd0) on Kite
/// ([`crate::chara::condition_time_count`]'s rules) with each
/// `EntryAffect` applied where the game makes it, so that what follows
/// reads what it left (a poison tick that downs him clears his conditions
/// before the curse and SP are looked at) and his
/// `CheckSpRegeneSpeed()` (0x0059f380) is asked at the natural SP.
pub fn condition_time_count(cx: &mut Cx, me: usize) {
    use crate::chara::cmod;
    let env = *cx.env;
    let ch = &mut cx.scene.chars[me];
    if let Some((temp, time)) = ch.temp_time_mut() {
        for i in TIMED {
            let t = time[i];
            if t > 0 {
                time[i] = t - 1;
                if t - 1 == 0 {
                    temp[i] = 0;
                }
            } else if t < 0 {
                time[i] = 0;
                temp[i] = 0;
            }
        }
    }
    let is_pc = ch.is_pc();
    let c = &mut ch.cond;
    for i in [cond::SLEEP, cond::CONFUSION, cond::CHARM, cond::PARALYSIS] {
        if c[i] > 0 {
            c[i] -= 1;
        } else if c[i] < 0 {
            c[i] = 0;
        }
    }
    if c[cond::SPEED] > 0 {
        c[cond::SPEED] -= 1;
        if c[cond::SPEED] == 0 {
            c.speed_value = ONE;
        }
    } else if c[cond::SPEED] < 0 {
        c[cond::SPEED] = 0;
        c.speed_value = ONE;
    }
    let at_least_1 = |v: i32| if v == 0 { 1 } else { v } as i16;
    let c = &mut cx.scene.chars[me];
    if c.cond[cond::REGENE_HP] != 0 {
        c.cond.v[cond::REGENE_HP] = c.cond[cond::REGENE_HP].wrapping_sub(1);
        if cmod(i32::from(c.cond[cond::REGENE_HP]), 60) == 0 {
            let v = at_least_1(i32::from(c.max_hp) / 50);
            cx.entry_affect(me, me, Some(me), 9, [v, 0, 0]);
        }
    }
    let c = &mut cx.scene.chars[me];
    if c.cond[cond::POISON] != 0 && env.menu_type == -1 {
        c.cond.v[cond::POISON] = c.cond[cond::POISON].wrapping_sub(1);
        if cmod(i32::from(c.cond[cond::POISON]), 90) == 0 && !(is_pc && c.no_death) {
            let v = at_least_1(i32::from(c.max_hp) / 100);
            cx.entry_affect(me, me, Some(me), 3, [v, 0, 0]);
        }
    }
    if cx.scene.chars[me].cond[cond::CURSE] == 0 {
        let tick = env.count.is_multiple_of(60);
        let regene = crate::affect::sp_regene_speed(&cx.scene.chars[me]);
        let c = &mut cx.scene.chars[me];
        let max_sp = i32::from(c.max_sp);
        if is_pc {
            let div = match (regene, env.in_battle != 0) {
                (true, true) => 50,
                (true, false) => 20,
                (false, true) => 100,
                (false, false) => 33,
            };
            if tick {
                c.sp = c.sp.wrapping_add(at_least_1(max_sp / div));
            }
            if c.sp > c.max_sp {
                c.sp = c.max_sp;
            }
        } else if tick {
            c.sp = c.sp.wrapping_add(at_least_1(max_sp / 50));
            if c.sp > c.max_sp {
                c.sp = c.max_sp;
            }
        }
    }
    let c = &mut cx.scene.chars[me];
    if c.cond[cond::REGENE_SP] != 0 {
        c.cond.v[cond::REGENE_SP] = c.cond[cond::REGENE_SP].wrapping_sub(1);
        if cmod(i32::from(c.cond[cond::REGENE_SP]), 60) == 0 {
            let v = at_least_1(i32::from(c.max_sp) / 50);
            cx.entry_affect(me, me, Some(me), 10, [v, 0, 0]);
        }
    }
    let c = &mut cx.scene.chars[me];
    if c.cond[cond::CURSE] != 0 {
        c.cond.v[cond::CURSE] = c.cond[cond::CURSE].wrapping_sub(1);
        if cmod(i32::from(c.cond[cond::CURSE]), 90) == 0 {
            let v = at_least_1(i32::from(c.max_sp) / 100);
            cx.entry_affect(me, me, Some(me), 4, [v, 0, 0]);
        }
    }
}

/// `ccAI::ReadSysMsg2()` (gcmn 0x0058be10): Kite's AI, not deciding, drops
/// the messages delivered to it. From Mutation on, when the battle is won
/// (`spcBattleCondition` 5, `battle_won`) it also clears `battleFlag` and
/// forgets the biggest recent hit.
pub fn read_sys_msg2(crew: &mut Crew, me: usize, battle_won: bool) {
    let Some(a) = crew.ais.get_mut(&me) else { return };
    if battle_won {
        a.battle_flag = 0;
        a.hit_recent = 0;
    }
    let e = &mut a.sys_msg;
    while e.msg_num != 0 {
        e.msg_num -= 1;
        if e.msg_top == e.msg_tail {
            e.msg_tail += 1;
            if e.msg_tail >= e.msg_max {
                e.msg_tail = 0;
            }
        }
        e.msg_top += 1;
        if e.msg_top >= e.msg_max {
            e.msg_top = 0;
        }
    }
}

/// `ccPlayer::Main()` (gcmn 0x00598310), Kite's frame on `ccThPlayer`:
///
/// ```text
/// his AI onto the message bus; a weapon change
/// CalcReal(dead); ResultOfConditions; a level up (effLevelUp); ccAI::levelCheck
/// movePos = (0, 0, 0, 1)
/// acts 23, 24: GateHackingOut, then the camera
/// on a moving floor: carried with it (diskOffset)
/// down (dead 2): 90 frames, then lying (act 10, dead 3), effOpenBox twice
/// the party wiped out: still
/// in manual mode, charmed or confused: ccAI::Brains drives him (held,
///     asleep or paralysed he does not move), movePos from his heading
/// else ControlMove (not in a cutscene, alive, a ghost or reviving), his
///     AI's targetFlag cleared and its messages dropped
/// HitCheck (stress against walls: 0x10004 to the party at 101); pos += movePos
/// CollisionTest; hitAttribute; MapLoopAdjustPos
/// CameraPosCalc, CameraPosSet; AnimCtrl
/// the matrix; plw; transparency (0 when the eye view hides him)
/// the draw and the weapon's trails; diskOffset; hold cleared; stopCnt; cycle
/// ```
pub fn main(cx: &mut Cx, me: usize) {
    if cx.crew.ais.get(&me).is_some_and(|a| a.sys_msg.id == -1) {
        cx.crew.add_entry(me);
        let id = cx.crew.sys_msg_id(me);
        cx.ch(me).spc_char.sys_msg_id = id;
    }
    let wc = ((cx.flags(me) >> 8) & 3) as i32;
    let wc = (wc << 30) >> 30;
    if wc == -1 {
        cx.out(Out::DeleteWeaponCcs);
        cx.clear(me, flag::WEAPON_CHANGE);
    } else if wc == 1 {
        cx.out(Out::EquipWeapon);
        cx.set(me, flag::WEAPON_CHANGE);
    }
    let dead = i32::from(cx.cond(me, cond::DEAD));
    calc_real(cx, me, dead);
    result_of_conditions(&cx.scene.chars[me], cx.p);
    if crate::chara::check_level_up(cx.t, &mut cx.scene.chars[me]) != 0 && cx.act(me) != act::HELD {
        cx.out(Out::LevelUp);
    }
    if cx.crew.ais.contains_key(&me) {
        to_spc(cx.scene, cx.crew, me);
        cx.ai().level_check(me);
        from_spc(cx.scene, cx.crew, me);
    }
    cx.spc(me).move_pos = VF0;
    if !matches!(cx.act(me), act::HACK | act::HACK_END) {
        motion(cx, me);
    } else {
        let s = cx.crew.spc.entry(me).or_default();
        cx.w.gate_hacking_out(me, &mut cx.scene.chars[me], cx.p, s);
    }
    camera_pos_calc(cx, me);
    camera_pos_set(cx, me);
    anim_ctrl(cx, me);
    let c = &cx.scene.chars[me];
    let (pos, dirc) = (c.pos, cx.crew.spc.get(&me).map_or(VF0, |s| s.dirc));
    cx.out(Out::Matrix { pos, dirc });
    let c = &cx.scene.chars[me];
    let (attack, pause) = (i32::from(c.spc_char.attack), c.spc_char.flags & PAUSE != 0);
    cx.out(Out::PlayerWork { attack, pause });
    let t = if cx.flags(me) & flag::LOST_HEAD != 0 { 0 } else { cx.scene.chars[me].spc_char.cloak };
    let s = cx.spc(me);
    s.transparency = t;
    s.set_transparency = t;
    if matches!(cx.act(me), act::HACK | act::HACK_END) {
        cx.out(Out::GateArms);
    }
    if cx.flags(me) & DISP != 0 {
        let pos = cx.scene.chars[me].pos;
        let s = cx.crew.spc.entry(me).or_default();
        let st = s.set_transparency;
        if cx.w.draw(me, pos, st, &mut s.transparency) {
            let d = cx.cond(me, cond::DEAD);
            let alive = d == 0 || (cx.flags(me) & GHOST != 0 && d == 4);
            if alive && !matches!(cx.act(me), 12 | 13 | 14 | 24) {
                cx.out(Out::ArmsEffect);
            } else {
                cx.out(Out::ClearArmsEffect);
            }
        } else {
            cx.out(Out::ClearArmsEffect);
        }
    }
    if !cx.manual(me) && cx.w.trans_mode() {
        let c = cx.w.trans_center();
        let pos = cx.scene.chars[me].pos;
        let mut d = geom::vsub(pos, c);
        d[3] = ONE;
        cx.p.disk_offset = d;
    }
    cx.ch(me).cond.v[cond::HOLD] = 0;
    let moving = cx.flags(me) & MOVE != 0;
    let s = cx.spc(me);
    s.stop_cnt = if moving { 0 } else { s.stop_cnt.saturating_add(1) };
    s.cycle = s.cycle.wrapping_add(1);
    to_spc(cx.scene, cx.crew, me);
}

/// `Main` from the moving floor to `MapLoopAdjustPos`: what moves him.
fn motion(cx: &mut Cx, me: usize) {
    if !cx.manual(me) && cx.w.trans_mode() {
        let c = cx.w.trans_center();
        let pos = cx.scene.chars[me].pos;
        let (c, _) = w2p_pos(&cx.input.bounds, pos, c);
        let d = cx.p.disk_offset;
        let mut pp = geom::vadd(c, d);
        pp[3] = ONE;
        let (w, _) = p2w_pos(&cx.input.bounds, pos, pp);
        let ch = cx.ch(me);
        ch.pos_p = pp;
        ch.pos = w;
    }
    if cx.cond(me, cond::DEAD) == 2 {
        let c = cx.ch(me);
        let old = c.spc_char.cnt;
        c.spc_char.cnt = old.wrapping_sub(1);
        if old < 0 {
            c.spc_char.cnt = 0;
            c.anm_flag = 1;
            c.spc_char.act_num = act::LYING;
            c.spc_char.act_num_old = act::LYING;
            c.cond.v[cond::DEAD] = 3;
            let pos = c.pos;
            cx.out(Out::OpenBox { pos });
            cx.out(Out::OpenBox { pos });
        }
    }
    if cx.party.annihilated(cx.scene) {
        cx.clear(me, MOVE | RUN);
    } else if cx.manual(me) || cx.cond(me, cond::CHARM) != 0 || cx.cond(me, cond::CONFUSION) != 0 {
        to_spc(cx.scene, cx.crew, me);
        cx.ai().brains(me);
        from_spc(cx.scene, cx.crew, me);
        if cx.cond(me, cond::HOLD) != 0 || cx.cond(me, cond::SLEEP) != 0 || cx.cond(me, cond::PARALYSIS) != 0 {
            cx.clear(me, MOVE);
        }
        let f = cx.flags(me);
        let lean = if f & MOVE == 0 {
            0
        } else if f & RUN != 0 {
            0x42c8_0000
        } else {
            K140
        };
        let tsp = cx.kt.tsp;
        let speed_value = cx.scene.chars[me].cond.speed_value;
        let s = cx.spc(me);
        let heading = s.dirc[2];
        let v = if f & RUN == 0 {
            s.speed_rate = div(lean, K140);
            mul(s.speed_rate, mul(speed_value, tsp))
        } else {
            s.speed_rate = div(lean, 0x42c8_0000);
            mul(s.speed_rate, mul(speed_value, s.speed))
        };
        s.now_speed = v;
        s.move_pos[0] = mul(v, geom::sinf(heading));
        s.move_pos[1] = mul(neg(v), geom::cosf(heading));
    } else {
        let d = cx.cond(me, cond::DEAD);
        if cx.p.prog_ctrl_flag == 0 && matches!(d, 0 | 4 | 5) {
            control_move(cx, me);
        }
        if let Some(a) = cx.crew.ais.get_mut(&me) {
            a.target_flag = 0;
        }
        let won = cx.t.volume != piney_data::volume::Volume::Inf && cx.game.spc_battle_condition == 5;
        read_sys_msg2(cx.crew, me, won);
    }
    if cx.p.prog_ctrl_flag == 0 {
        let manual = cx.manual(me);
        let ns = cx.spc(me).now_speed;
        let mut mp = cx.spc(me).move_pos;
        let s = cx.crew.spc.entry(me).or_default();
        let r = cx.w.hit_check(me, &cx.scene.chars[me], &mut s.body_hit, ns, manual, &mut mp);
        cx.spc(me).move_pos = mp;
        if r & 3 == 3 {
            cx.p.stress = cx.p.stress.wrapping_add(2);
            if cx.p.stress >= 101 {
                cx.p.stress = 50;
                let id = cx.msg_id(me);
                cx.crew.send(0x10004, id, 0xffff, 0, 0, 0, None);
            }
        } else {
            cx.p.stress = cx.p.stress.wrapping_sub(8);
            if cx.p.stress < 0 {
                cx.p.stress = 0;
            }
        }
    }
    let mp = cx.spc(me).move_pos;
    let pos = &mut cx.scene.chars[me].pos;
    pos[0] = add(pos[0], mp[0]);
    pos[1] = add(pos[1], mp[1]);
    collision_test(cx, me);
    let a = cx.w.hit_attribute();
    cx.spc(me).hit_attribute = a;
    map_loop_adjust_pos(cx, me);
}

// ---------------------------------------------------------------------------
// The AI's attack, the menus' calls

/// `ccPlayer::Attack(tp, n)` (gcmn 0x0059c580): Kite's attack on `tp`
/// when his AI drives him ([`Call::PlayerAttack`]). While no skill runs:
/// every 180th frame of his task (`cycle`), neither confused nor charmed,
/// able to act and allowed arts or spells, he chooses
/// (`ccAI::SelectAttackSkill`; none is the normal attack). `tp` becomes
/// his target; then the normal attack starts within `armsRange` (the AI's
/// `distTg` for a distance of exactly -1, or within 30 of his stride while
/// moving) and a running one ends on a target at `dead` 1; a skill starts
/// within its `triggerRange`, an item is used (`ccAI::UseItem`). Every
/// attempt counts in the AI's `atkTargetCnt`, and a skill or item
/// announces its damage to the party (0x10008). Returns 1 done, -1 out of
/// range, 0 unable.
pub fn attack(ctx: &mut party_ai::Ctx, me: usize, tp: usize, _n: i32) -> i32 {
    from_spc(ctx.scene, ctx.crew, me);
    let r = attack_body(ctx, me, tp);
    to_spc(ctx.scene, ctx.crew, me);
    r
}

fn attack_body(ctx: &mut party_ai::Ctx, me: usize, tp: usize) -> i32 {
    if ctx.scene.chars[me].skill_id >= 2 {
        return 1;
    }
    let can = |ctx: &party_ai::Ctx, n: i32| {
        let c = &ctx.scene.chars[me];
        party_ai::check_action(c, c.spc_char.act_num, n, ctx.t.volume)
    };
    let mut s0 = 1;
    let mut item = false;
    let cycle = ctx.crew.spc.get(&me).map_or(0, |s| s.cycle);
    let c = &ctx.scene.chars[me].cond;
    if crate::chara::cmod(cycle, 180) == 0
        && c[cond::CONFUSION] == 0
        && c[cond::CHARM] == 0
        && (can(ctx, 6) || can(ctx, 7))
        && ctx.check_skill_mask(me) & 3 != 0
    {
        let r = ctx.select_attack_skill(me, tp);
        if r < 0 {
            s0 = 1;
        } else {
            item = r == 2;
            s0 = ctx.crew.select_attack_skill_result;
        }
    }
    ctx.scene.chars[me].target_char = Some(tp);
    let dist = party_ai::distance_to_target(ctx.scene, me, Some(tp));
    let dead1 = ctx.scene.chars[tp].cond[cond::DEAD] == 1;
    let range = |t: &Tables, sid: i32| t.skill(sid).is_some_and(|k| skill::range_check(k, dist));
    if s0 == 1 {
        if !can(ctx, 3) {
            return 0;
        }
        if ctx.scene.chars[me].skill_id == 1 {
            if dead1 {
                let target = ctx.scene.chars[me].target_char;
                request(ctx, me, target, 0);
            }
            return 1;
        }
        if dead1 {
            return 0;
        }
        let near = if ctx.scene.chars[me].spc_char.flags & MOVE != 0 {
            let ns = ctx.crew.spc.get(&me).map_or(0, |s| s.now_speed);
            lt(sub(dist, ns), 0x41f0_0000)
        } else {
            let a = &ctx.crew.ais[&me];
            let arms = ctx.t.ai_params.get(a.param).map_or(0, |r| r.arms_range);
            if geom::eq(party_ai::F_MINUS_ONE, dist) { le(a.dist_tg, arms) } else { le(dist, arms) }
        };
        if !near {
            return -1;
        }
        let target = ctx.scene.chars[me].target_char;
        request(ctx, me, target, s0);
    } else if item {
        if !can(ctx, 7) {
            return 0;
        }
        let sk = ctx.t.item(s0).map_or(0, |p| p.skill_id);
        if !range(ctx.t, sk) {
            return -1;
        }
        if dead1 {
            return 0;
        }
        to_spc(ctx.scene, ctx.crew, me);
        ctx.use_item(me, s0, Some(tp));
        from_spc(ctx.scene, ctx.crew, me);
    } else {
        if !can(ctx, 6) {
            return 0;
        }
        if !range(ctx.t, s0) {
            return -1;
        }
        if dead1 {
            return 0;
        }
        let target = ctx.scene.chars[me].target_char;
        request(ctx, me, target, s0);
    }
    if let Some(a) = ctx.crew.ais.get_mut(&me) {
        a.atk_target_cnt = a.atk_target_cnt.wrapping_add(1);
    }
    if item || s0 != 1 {
        let target = ctx.scene.chars[me].target_char;
        let id = ctx.crew.sys_msg_id(me) as u16;
        let sid = if item { ctx.t.item(s0).map_or(0, |p| p.skill_id) } else { s0 };
        let v = target.map_or(0, |t| {
            let (a, b) = (ctx.scene.listed(me), ctx.scene.listed(t));
            let (cp, tc) = (&ctx.scene.chars[me], &ctx.scene.chars[t]);
            crate::damage::skill_damage_value(ctx.t, cp, tc, sid, a, b, &Env::default()).dmg
        });
        ctx.crew.send(0x10008, id, id, 0, 0, v, target);
    }
    1
}

/// `ccSkillRequest(me, target, sid)` (gcmn 0x00572700) from inside the
/// party AI's call: the runtime's.
fn request(ctx: &mut party_ai::Ctx, me: usize, target: Option<usize>, sid: i32) {
    ctx.rt.call(Call::SkillRequest { me, target, sid }, ctx.scene, ctx.crew, ctx.rng);
}

/// `ccPlayer::AttackCancel()` (gcmn 0x0059cad0), when a box or trap menu
/// opens: a swing (15, 16) or a box being broken (25) stops; he stands
/// (act 0), free unless it was the box.
pub fn attack_cancel(scene: &mut Scene, crew: &mut Crew, me: usize) {
    let ch = &mut scene.chars[me];
    let a = ch.spc_char.act_num;
    if !matches!(a, 15 | 16 | 25) {
        return;
    }
    let f = &mut ch.spc_char.flags;
    if a == 25 {
        *f &= !RESTRAINT;
    } else {
        *f |= RESTRAINT;
    }
    ch.spc_char.act_num = act::IDLE_FIELD;
    ch.spc_char.act_num_old = -1;
    ch.spc_char.arms_effect_sw = 0;
    ch.spc_char.flags &= !(TRAJECTORY | MOVE | RUN);
    to_spc(scene, crew, me);
}

/// `ccPlayer::BreakSomething(tp)` (gcmn 0x0059cbd0), when he opens a box
/// or a trap: act 25, held, facing it; the trails take the normal
/// attack's colour.
pub fn break_something(cx: &mut Cx, me: usize, tp: Option<usize>) {
    cx.set_act(me, act::BREAK);
    cx.set_act_old(me, -1);
    cx.ch(me).spc_char.arms_effect_sw = 0;
    cx.set(me, RESTRAINT | TRAJECTORY);
    cx.clear(me, MOVE | RUN);
    cx.ch(me).target_char = tp;
    set_target_dirc(cx, me);
    cx.out(Out::ArmsEffectColor { sid: 1 });
    to_spc(cx.scene, cx.crew, me);
}

/// `ccPlayer::DamageActuate(dmg)` (gcmn 0x0059ca40), from `Influence`'s
/// hit: the pad rumbles `DamActuTbl[0]` for under 10, `[1]` under 100,
/// else `[2]` (nothing in act 14 or for no damage). `Influence` calls it
/// before it sets the hurt or down act: `act` is Kite's act as the affect
/// found it, which [`crate::event::Event::DamageActuate`] carries.
pub fn damage_actuate(kt: &KiteTables, act: i16, dmg: i32) -> Option<Out> {
    actuate(kt, act, dmg)
}

/// `DamageActuate` on a character in act `a`.
fn actuate(kt: &KiteTables, a: i16, dmg: i32) -> Option<Out> {
    if a == act::HELD || dmg <= 0 {
        return None;
    }
    let i = if dmg < 10 {
        0
    } else if dmg < 100 {
        1
    } else {
        2
    };
    Some(Out::Actuate { power: kt.dam_actu[i] })
}

/// `ccPlayerMenuCheck()` (gcmn 0x0059cd70), before `ccThGameCtrl` opens a
/// menu: the party not wiped out, Kite's running skill interruptible or
/// none (`ccSkillCheck(plw) < 2`: `skill_check`, [`flow::Skills::check`]),
/// and neither leaving nor arriving (acts 12, 13).
pub fn menu_check(party: &Party, scene: &Scene, me: usize, skill_check: i32) -> bool {
    if party.annihilated(scene) || skill_check >= 2 {
        return false;
    }
    !matches!(scene.chars[me].spc_char.act_num, 12 | 13)
}

/// `ccSpcChar::CheckControlMode()` (gcmn 0x0059f550): his AI is in manual
/// mode (`manualSW`).
pub fn check_control_mode(crew: &Crew, me: usize) -> bool {
    crew.ais.get(&me).is_some_and(|a| a.manual_sw)
}

// ---------------------------------------------------------------------------
// The frames

/// 2.0.
const K2: F = 0x4000_0000;

/// `ccPlayer::W2MPos(mp, wp)` (gcmn 0x0059b470), `ccTransPosW2M`
/// (0x0059b900): a world position in the map: past an edge it comes in
/// from the other side (x, then y); `w` 1. Returns whether it wrapped.
pub fn w2m_pos(b: &MapBounds, wp: V4) -> (V4, bool) {
    let mut t = wp;
    let mut r = false;
    for (i, v) in t.iter_mut().enumerate().take(2) {
        let (lo, hi) = (b.min[i], b.max[i]);
        if lt(*v, lo) {
            *v = add(*v, sub(hi, lo));
            r = true;
        } else if !le(*v, hi) {
            *v = add(*v, sub(lo, hi));
            r = true;
        }
    }
    t[2] = wp[2];
    t[3] = ONE;
    (t, r)
}

/// `ccPlayer::W2PPos(pp, wp)` (gcmn 0x0059b5a0), `ccTransPosW2P`
/// (0x0059b940) with Kite at `player`: a world position relative to him on
/// the ground, taken the short way round the map (within half its size);
/// z the world's, `w` 1. Returns whether it wrapped.
pub fn w2p_pos(b: &MapBounds, player: V4, wp: V4) -> (V4, bool) {
    let mut t = geom::vsub(wp, player);
    t[3] = ONE;
    let mut r = false;
    for (i, v) in t.iter_mut().enumerate().take(2) {
        let (lo, hi) = (b.min[i], b.max[i]);
        let h = div(add(lo, hi), K2);
        if lt(*v, sub(lo, h)) {
            *v = add(*v, sub(hi, lo));
            r = true;
        } else if !le(*v, sub(hi, h)) {
            *v = add(*v, sub(lo, hi));
            r = true;
        }
    }
    t[2] = wp[2];
    (t, r)
}

/// `ccPlayer::P2WPos(wp, pp)` (gcmn 0x0059b710), `ccTransPosP2W`
/// (0x0059b980) with Kite at `player`: back from his frame to the world's,
/// wrapped toward the map's middle from the half he stands in; z the
/// frame's, `w` 1. Returns whether it wrapped.
pub fn p2w_pos(b: &MapBounds, player: V4, pp: V4) -> (V4, bool) {
    let mut t = geom::vadd(pp, player);
    t[3] = ONE;
    let mut r = false;
    for i in 0..2 {
        let (lo, hi) = (b.min[i], b.max[i]);
        let h = div(add(lo, hi), K2);
        if lt(player[i], h) {
            if !le(pp[i], sub(hi, h)) {
                t[i] = add(t[i], sub(lo, hi));
                r = true;
            }
        } else if lt(pp[i], sub(lo, h)) {
            t[i] = add(t[i], sub(hi, lo));
            r = true;
        }
    }
    t[2] = pp[2];
    (t, r)
}

/// `ccTransPosFW2LW(out, in)` (gcmn 0x0059b9c0): a world position taken
/// into Kite's frame and back ([`w2p_pos`], [`p2w_pos`]): the copy of it
/// nearest him.
pub fn fw2lw(b: &MapBounds, player: V4, wp: V4) -> V4 {
    let (pp, _) = w2p_pos(b, player, wp);
    p2w_pos(b, player, pp).0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(x: f32) -> F {
        x.to_bits()
    }

    fn bounds(lo: f32, hi: f32) -> MapBounds {
        MapBounds { min: [f(lo), f(lo)], max: [f(hi), f(hi)] }
    }

    #[test]
    fn the_map_wraps_at_its_edges() {
        let b = bounds(-1000.0, 1000.0);
        let (p, r) = w2m_pos(&b, [f(1200.0), f(-50.0), f(7.0), 0]);
        assert!(r);
        assert_eq!(p, [f(-800.0), f(-50.0), f(7.0), ONE]);
        let (p, r) = w2m_pos(&b, [f(1000.0), f(-1000.0), 0, 0]);
        assert!(!r);
        assert_eq!(p[..2], [f(1000.0), f(-1000.0)]);
    }

    #[test]
    fn the_player_frame_takes_the_short_way() {
        let b = bounds(-1000.0, 1000.0);
        let me = [f(900.0), 0, f(10.0), ONE];
        // A point just past the far edge is 200 ahead, not 1800 behind.
        let (pp, r) = w2p_pos(&b, me, [f(-900.0), f(100.0), f(5.0), ONE]);
        assert!(r);
        assert_eq!(pp, [f(200.0), f(100.0), f(5.0), ONE]);
        // Back to the world it lands past the edge he is near: the copy
        // of the point nearest him, which W2MPos wraps in again.
        let (wp, r) = p2w_pos(&b, me, pp);
        assert!(!r);
        assert_eq!(wp, [f(1100.0), f(100.0), f(5.0), ONE]);
        assert_eq!(fw2lw(&b, me, [f(-900.0), f(100.0), f(5.0), ONE]), wp);
        // From the other half a point far behind comes round.
        let me = [f(-900.0), 0, 0, ONE];
        let (wp, r) = p2w_pos(&b, me, [f(1200.0), 0, 0, ONE]);
        assert!(r);
        assert_eq!(wp[0], f(-1700.0));
    }

    #[test]
    fn the_stick_eases_in() {
        let mut p = Player::default();
        assert_eq!(pad_lever_power(&mut p, f(63.0), false), 0);
        assert_eq!(pad_lever_power(&mut p, f(100.0), false), f(86.0));
        assert_eq!(pad_lever_power(&mut p, f(200.0), true), 0);
        p.target_count = 4;
        assert_eq!(pad_lever_power(&mut p, f(200.0), true), f(200.0));
    }

    #[test]
    fn the_rumble_grows_with_the_hit() {
        let kt = KiteTables { dam_actu: [64, 128, 255], ..KiteTables::default() };
        assert_eq!(damage_actuate(&kt, 0, 0), None);
        assert_eq!(damage_actuate(&kt, 0, 9), Some(Out::Actuate { power: 64 }));
        assert_eq!(damage_actuate(&kt, 0, 99), Some(Out::Actuate { power: 128 }));
        assert_eq!(damage_actuate(&kt, 0, 100), Some(Out::Actuate { power: 255 }));
        assert_eq!(damage_actuate(&kt, act::HELD, 500), None);
    }
}
