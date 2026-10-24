//! Macha (`ccBoss06`, boss06.cpp), Quarantine's first phase boss: `bossTbl`
//! row 5, `bossFunc` code 5, fought in event 307 (M407) in field 6. Ported
//! from Quarantine's gcmn.prg (0x003a3460-0x003a81e0; names and layouts
//! from Infection's DWARF, whose gcmn carries the class unused), with the
//! `ccBoss` base Outbreak's and Quarantine's share ([`frame`]). Its light
//! ball, charm, eye lasers, skill and magic. docs/engine/boss-macha.md.

pub mod eff;
#[cfg(test)]
mod tests;

use piney_data::field::ee;
use piney_data::libm;
use piney_data::volume::Volume;

use self::eff::Fx;
use super::blur::{BlurFunc, Set};
use super::{AfterImageParams, Anm, Boss, Class, Cx, EffKind, Out, Tbl, VF0};
use crate::enemy_ai::{get_dirc, rand_f};
use crate::geom::{self, V4};
use crate::item;
use crate::param::cond;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const NEG_PI: F = 0xc049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const HALF_PI: F = 0x3fc9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// `SelectTarget`'s radius here: 999999.
const FAR: F = 0x4974_23f0;
/// The stage fader's colour (`BeginStageEffect(0x64000000, ...)`), and the
/// charm's pink.
const DARK: u32 = 0x6400_0000;
const PINK: u32 = 0x64ff_7fff;

/// `bossFunc`'s code for Macha, its `bossTbl` row and its file.
pub const CODE: i32 = 5;
pub const ROW: usize = 5;
pub const FILE: &str = "x61";
/// `InitBossCamera(250, 1000)`.
pub const CAM_Z: F = 0x437a_0000;
pub const CAM_Y: F = 0x447a_0000;
/// The drain's HP (`OnThinkDrain`).
pub const EPITAPH_HP: i16 = 6000;
/// The skill `OnThinkSkill` always casts (`m_skillID` = 161; `@1414`'s
/// three are copied and never read).
pub const SKILL: i32 = 161;
/// The light ball's clips (`anmBallCharge`, `anmBall`), in [`FILE`].
pub const BALL_CHARGE: &str = "ANM_ex61efc0";
pub const BALL: &str = "ANM_ex61efc1";
/// The eyes the lasers leave from (`GetSubstAdrsF`).
pub const EYES: [&str; 2] = ["OBJ_dummy_r_eye", "OBJ_dummy_l_eye"];

/// Macha's acts (`actNum`, `Think`'s switch at 0x003a4310, table `@1261`;
/// any other is `ChangeAction(0, 0, 1)`).
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DAMAGE0: i16 = 1;
    pub const DAMAGE1: i16 = 2;
    pub const LIGHT_BALL: i16 = 3;
    pub const WAVE: i16 = 4;
    pub const CHARM: i16 = 5;
    pub const EYE_LASER: i16 = 6;
    pub const DASH_CENTER: i16 = 7;
    pub const DRAIN: i16 = 11;
    pub const EPITAPH: i16 = 12;
    pub const EPITAPH_WAVE: i16 = 13;
    pub const DEAD: i16 = 14;
    pub const ESCAPE: i16 = 16;
    pub const CHASE: i16 = 18;
    /// `OnThinkBackDash` and `OnThinkTransfer`: in the table, but no
    /// pattern or affect of Macha's changes to either.
    pub const BACK_DASH: i16 = 20;
    pub const TRANSFER: i16 = 21;
    pub const SKILL: i16 = 22;
    pub const DATA_DRAIN_ATK: i16 = 23;
    pub const MAGIC: i16 = 24;
}

/// Macha's tables (`tables::combat`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MachaData {
    /// `boss06NormalActTbl`, `boss06ExpertActTbl`, `boss6EpitaphActTbl`,
    /// each through its closing -1 (`@1143`'s rows 0, 2 and 3; row 1,
    /// `boss06SuperActTbl`, is a lone -1 nothing selects).
    pub normal: Vec<i32>,
    pub expert: Vec<i32>,
    pub epitaph: Vec<i32>,
    /// `BOSS06AnmTbl`: the clip by act.
    pub anims: Vec<Option<String>>,
    /// `@1198`: pattern 10's draw (dropped).
    pub rand_skills: [i32; 3],
    /// `@1913`: `OnThinkMagic`'s spells.
    pub magic_skills: [i32; 4],
}

impl MachaData {
    /// The volume's.
    pub fn of(volume: Volume) -> MachaData {
        let t = piney_data::tables::combat::of(volume);
        let s = |v: &[Option<&str>]| v.iter().map(|a| a.map(str::to_string)).collect::<Vec<_>>();
        let pick = |v: &[i32], k: usize| v.get(k).copied().unwrap_or(0);
        MachaData {
            normal: t.macha_normal().to_vec(),
            expert: t.macha_expert().to_vec(),
            epitaph: t.macha_epitaph().to_vec(),
            anims: s(t.macha_anims()),
            rand_skills: std::array::from_fn(|k| pick(t.macha_rand_skills(), k)),
            magic_skills: std::array::from_fn(|k| pick(t.macha_magic_skills(), k)),
        }
    }

    /// The table by [`Tbl`]: `Super` stands for Macha's expert one.
    fn words(&self, t: Tbl) -> &[i32] {
        match t {
            Tbl::None => &[],
            Tbl::Normal => &self.normal,
            Tbl::Super => &self.expert,
            Tbl::Epitaph => &self.epitaph,
        }
    }
}

/// What Macha shows beside its body, each frame it is drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pic {
    /// `anmBallCharge` (`ANM_ex61efc0`) at the boss 550 up, or `anmBall`
    /// (`ANM_ex61efc1`) at `m_ballPos`, posed at `time`, turned by the
    /// boss's heading (`ccAnm::Draw` on effLayer).
    Ball { charge: bool, time: u32, pos: V4, dirc: V4 },
    /// The light ball's omni light (`ccOmniLight`, colour 0x00ffffff at
    /// 1.0, its range 500 to 250000) at `pos`; `None` once deleted.
    Light(Option<V4>),
    /// `anmw` (`xeffect`'s `ANM_xx11wave`) posed at `time` at `pos`, turned
    /// by `dirc`, at `scale` (2 in the Wave by `DrawParts`, 1 in the
    /// Epitaph's by `ccAnm::Draw`).
    Wave { time: u32, pos: V4, dirc: V4, scale: F },
    /// `effSmoke(pos, vel, 8, 60, 114, 512, 32)`: the back dash's dust.
    Smoke { pos: V4, vel: V4 },
    /// A laser's bolt (`xeffect`'s `CMP_ex31les0`) drawn at a tenth of its
    /// size at `pos`, `Rx(-pi/2) Rx(pitch) Rz(heading)`.
    Laser { pos: V4, pitch: F, heading: F },
    /// A laser on the land at `pos`: `ccParticleExplode(pos, (0, 0, 5), 5,
    /// 1)` and `(0, 0, 2)`, then `effSmokeRock(pos, (-pi/2, 0, turn), 45, 2,
    /// 2, -1)`.
    LaserHit { pos: V4, turn: F },
    /// The light ball's blast (`CMP_ex31exp1`) at `pos`, `scale` each way,
    /// at `transparency`.
    Blast { pos: V4, scale: F, transparency: F },
    /// One of its bursts at `pos`: `ccParticleExplode(pos, 0, size, kind)`,
    /// `effSmoke(pos, 0, size, 30, 31, 512, 32)`, `effSmokeRock(pos, (-pi/2,
    /// 0, 0), rock, 0, 1, -1)`.
    Burst { pos: V4, rock: F, size: F, kind: i32 },
}

/// `ccBoss06`'s own members (+0x29350 on).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Macha {
    /// `anmw`, `anmBallCharge`, `anmBall`.
    pub anm_w: Anm,
    pub anm_ball_charge: Anm,
    pub anm_ball: Anm,
    /// `m_effLaser`: the first eye's laser, whose `m_bEnabled` the lasers'
    /// act waits on.
    pub eff_laser: Option<i32>,
    pub eff_dead: Option<i32>,
    pub skill_id: i32,
    /// `m_effStart`: `effSkillStart`'s controller, the frames before its
    /// `endFlag` is seen.
    pub eff_start: Option<i32>,
    pub pat_mode: i32,
    /// `m_light` made, and `m_lightRot`.
    pub light: bool,
    pub light_rot: F,
    pub stage_eff: i32,
    pub ball_pos: V4,
    pub transfer_pos: V4,
    pub ball_speed: F,
    /// The blur's callback as Macha last set it, and the one `Transfer`
    /// keeps to give back (`m_orgBlurParamFunc`).
    pub blur_func: Option<BlurFunc>,
    pub org_blur: Option<BlurFunc>,
    /// The eyes' places (`OBJ_dummy_r_eye`, `_l_eye`) as the body was
    /// last posed: the runtime's.
    pub eyes: [V4; 2],
}

fn macha_of(b: &mut Boss) -> Box<Macha> {
    match std::mem::replace(&mut b.class, Class::Plain) {
        Class::Macha(x) => x,
        other => {
            b.class = other;
            Box::default()
        }
    }
}

// --- small rules ------------------------------------------------------------------------

fn word(tbl: &[i32], i: i32) -> i32 {
    usize::try_from(i).ok().and_then(|k| tbl.get(k)).copied().unwrap_or(0)
}

fn target_ok(cx: &Cx, t: Option<usize>) -> bool {
    cx.valid(t) && t.is_some_and(|t| !cx.dead(t))
}

/// `if (ccMenu) ccMenu->cursolOff = on`.
fn cursor(cx: &mut Cx, on: bool) {
    cx.out(Out::CursorOff(on));
}

fn select(b: &mut Boss, cx: &mut Cx, ty: i32) -> Option<usize> {
    let t = b.select_target(cx, ty, FAR);
    cx.scene.chars[cx.me].target_char = t;
    t
}

/// `-pi/2`, then `d`'s x, y and z turns on the unit matrix: where the
/// action cameras put their eye from what they look at.
fn cam_matrix(d: V4) -> geom::M4 {
    let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
    let m = geom::rot_matrix_x(&m, d[0]);
    let m = geom::rot_matrix_y(&m, d[1]);
    geom::rot_matrix_z(&m, d[2])
}

/// The boss camera's free mode (`SetMode(5)`, `SetFreeCamPosView`): the eye
/// `off` from `vp` turned by `d`, looking at `vp`.
fn free_cam(cx: &mut Cx, vp: V4, d: V4, off: V4) {
    let v = geom::apply_matrix(&cam_matrix(d), off);
    let eye = super::p2w(cx, geom::vadd(super::w2p(cx, vp), v));
    cx.out(Out::CamMode { mode: 5 });
    cx.out(Out::FreeCam { pos: eye, view: vp });
}

/// An angle wrapped once: below -pi a turn on, else above pi a turn off.
fn wrap(v: F) -> F {
    if ee::lt(v, NEG_PI) {
        ee::add(v, TWO_PI)
    } else if !ee::le(v, PI) {
        ee::sub(v, TWO_PI)
    } else {
        v
    }
}

/// The white after-image the light ball and the charm leave
/// (`ENTRYAFTERIMAGE_T`): colour 0xffffffff at 0, transparency 1 by -1/30.
pub const WHITE_IMAGE: AfterImageParams = AfterImageParams {
    color: 0xffff_ffff,
    color_alpha: 0,
    color_spd: 0,
    max_frame: 0,
    transparency: ONE,
    trans_spd: 0xbd08_8889,
    ss: VF0,
    es: VF0,
    scale_time: 0,
};

// --- the constructor ------------------------------------------------------------------------

/// `ccBoss06::ccBoss06` (QUA gcmn 0x003a3640): the boss is
/// `scene.chars[cx.me]` (`bossTbl` row 5); it stands 500 behind Kite and 50
/// up. `center` is the arena's `DMY_center01`.
pub fn new(cx: &mut Cx, kite_pos: V4, kite_dirc: V4, center: V4) -> Boss {
    let me = cx.me;
    let mut b = super::kyvia::plain_boss();
    b.anm_tbl = cx.data.macha.anims.clone();
    // OffExit, OnDraw, OnBodyHit, OnCheatHP.
    b.exit = 0;
    b.draw_sw = 1;
    b.body_hit_sw = 1;
    b.cheat_hp = 1;
    let mut pos = kite_pos;
    pos[1] = ee::sub(pos[1], 0x43fa_0000);
    pos[2] = ee::add(pos[2], 0x4248_0000);
    cx.scene.chars[me].pos = pos;
    b.dirc = kite_dirc;
    b.change_action(cx, act::NEUTRAL, 0, true);
    crate::fellow::entry_cmnd(cx.scene, me);
    // InitCenterPos, InitAfterImage(anm, 32), InitBossEffect(32),
    // InitStageEffect, InitBossCamera(250, 1000).
    b.center_pos = [center[0], center[1], center[2], ONE];
    b.center_pos_p = super::w2p(cx, b.center_pos);
    b.use_stage_eff = true;
    let mut x = Box::new(Macha {
        anm_w: Anm::new(),
        anm_ball_charge: Anm::new(),
        anm_ball: Anm::new(),
        stage_eff: -1,
        transfer_pos: b.center_pos,
        blur_func: Some(BlurFunc::Boss03),
        ..Macha::default()
    });
    // The blur on (m_exit 0) under ccBoss03BlurParamCB.
    cx.out(Out::Blur(Set::Enable));
    cx.out(Out::Blur(Set::Func(BlurFunc::Boss03)));
    // SetBlendTypeAfterImage(0): +0x292f8, which nothing reads.
    set_pattern_mode(&mut b, &mut x, cx, 0);
    b.class = Class::Macha(x);
    b
}

/// `SetPatternMode(m)` (0x003a3b00): `@1143[m]` set and run from its start.
fn set_pattern_mode(b: &mut Boss, x: &mut Macha, cx: &mut Cx, m: i32) {
    let t = match m {
        0 => Tbl::Normal,
        2 => Tbl::Super,
        3 => Tbl::Epitaph,
        _ => Tbl::None,
    };
    b.pat_tbl = t;
    b.pat_index = 0;
    x.pat_mode = m;
    let tbl = cx.data.macha.words(t).to_vec();
    b.pat_index = exec_pattern_index(b, x, cx, &tbl, 0);
}

// --- the frame --------------------------------------------------------------------------------

/// `ccThBossEffect`'s pass and `ccBoss06::Main` (0x003a4270).
pub(super) fn main(b: &mut Boss, cx: &mut Cx) {
    let mut x = macha_of(b);
    manager_pass(b, cx);
    frame(b, &mut x, cx);
    let me = cx.me;
    cx.scene.chars[me].cond[cond::HOLD] = 0;
    if (b.lock_player != 0 || b.erase_target != 0)
        && b.epitaph == 0
        && let Some(p) = cx.scene.param_mut(me)
        && p.pp_count > 0
    {
        p.pp_count += 1;
    }
    b.class = Class::Macha(x);
}

/// `ccBoss::Main` (QUA gcmn, Outbreak's code) under Macha's `Think`.
fn frame(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    if b.exit != 0 {
        return;
    }
    let me = cx.me;
    b.calc_real(cx);
    b.calc_target_info(cx);
    b.center_pos_p = super::w2p(cx, b.center_pos);
    let pp = cx.scene.chars[me].pos_p;
    b.center_dist = super::dist(cx, pp, b.center_pos_p);
    b.center_dirc = get_dirc(pp, b.center_pos_p);
    if b.stop != 0 {
        b.stop_count += 1;
        if b.stop_count >= 61 {
            b.stop_count = 0;
            b.stop = 0;
        }
        b.move_spd = 0;
    } else {
        b.stop_count = 0;
    }
    think(b, x, cx);
    if b.lock_player != 0 {
        for m in cx.party.members.into_iter().flatten() {
            if cx.valid(Some(m)) {
                cx.affect(m, 5, 32767);
            }
        }
        if b.reserve_forbid_menu == 1 && cx.env.menu_forbid == 0 && cx.env.menu_type == -1 {
            cx.out(Out::MenuForbid { on: true, chat_except: b.reserve_forbid_chat_except != 0 });
            b.reserve_forbid_menu = 0;
            b.reserve_forbid_chat_except = 0;
        }
    } else if b.reserve_forbid_menu == -1 && cx.env.menu_type == -1 {
        cx.out(Out::MenuForbid { on: false, chat_except: false });
        b.reserve_forbid_menu = 0;
        b.reserve_forbid_chat_except = 0;
    }
    b.move_(cx);
    if b.draw_sw != 0 {
        b.anm_status = i8::from(b.anm.forward());
    }
    cx.scene.chars[me].cond[cond::HOLD] = 0;
}

/// `ccBossEffManager::Draw`'s pass: each enabled effect's `Draw` in slot
/// order, a disabled one freed. The lasers fly and the shock fades
/// ([`eff`]); the others are the lives [`super::Eff::draw`] keeps.
fn manager_pass(b: &mut Boss, cx: &mut Cx) {
    for k in 0..b.effects.slots.len() {
        let Some(e) = b.effects.slots[k].as_mut() else { continue };
        if !e.enabled {
            b.effects.slots[k] = None;
            continue;
        }
        let Some(mut fx) = e.macha.take() else {
            e.draw();
            continue;
        };
        let alive = fx.draw(cx);
        if let Some(e) = b.effects.slots[k].as_mut() {
            e.macha = Some(fx);
            e.enabled = alive;
        }
    }
}

/// An effect of Macha's own made in the first free slot.
fn make_fx(b: &mut Boss, cx: &mut Cx, kind: EffKind, fx: Fx, pos: V4) -> i32 {
    let id = b.effects.create(kind);
    if let Some(e) = usize::try_from(id).ok().and_then(|k| b.effects.slots.get_mut(k)).and_then(|s| s.as_mut()) {
        e.macha = Some(Box::new(fx));
    }
    cx.out(Out::Effect { id, kind, pos, dirc: VF0 });
    id
}

// --- the patterns --------------------------------------------------------------------------------

/// `ccBoss::ChangeNextPattern` (QUA gcmn 0x00361e30, Outbreak's).
fn change_next_pattern(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let tbl = cx.data.macha.words(b.pat_tbl).to_vec();
    if word(&tbl, b.pat_index) == -1 {
        b.pat_index = 0;
    }
    b.move_spd = 0;
    b.move_vector = VF0;
    if b.epitaph != 0 {
        b.change_action(cx, act::EPITAPH, 3, true);
    } else {
        b.change_action(cx, act::NEUTRAL, 1, true);
    }
    b.pat_index = exec_pattern_index(b, x, cx, &tbl, b.pat_index);
}

/// `ccBoss::ExecPattern(pat, p0, 0, 0)` (QUA gcmn 0x00360d30).
fn exec_pattern(b: &mut Boss, x: &mut Macha, cx: &mut Cx, pat: i32, p0: i32) {
    let tbl = [pat, p0, 0, 0, -1];
    exec_pattern_index(b, x, cx, &tbl, 0);
}

/// `ccBoss06::ExecPatternIndex(tbl, i)` (0x003a3bb0), falling back on
/// `ccBoss::ExecPatternIndex`: the next index. The pattern is `tbl`'s word,
/// but every operand of its own patterns (2, 9, 10, 11) is read from
/// `patTbl` at the running index, whatever `tbl` is: `ExecPattern(2, 30)`
/// waits for `patTbl[1]`.
fn exec_pattern_index(b: &mut Boss, x: &mut Macha, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
    let orig = i;
    let mut s = i;
    let mut pat = word(tbl, s);
    s += 1;
    b.move_spd = 0;
    b.move_vector = VF0;
    if pat == -1 {
        pat = word(tbl, 0);
        s = 1;
    }
    b.pat_num = pat;
    let me = cx.me;
    let own = cx.data.macha.words(b.pat_tbl).to_vec();
    match pat {
        18 => b.change_action(cx, act::MAGIC, 3, true),
        17 => b.change_action(cx, act::DATA_DRAIN_ATK, 3, true),
        16 => b.change_action(cx, act::EYE_LASER, 3, true),
        15 => b.change_action(cx, act::CHARM, 3, true),
        14 => b.change_action(cx, act::LIGHT_BALL, 3, true),
        1 => {
            let n = if b.epitaph != 0 { act::EPITAPH_WAVE } else { act::WAVE };
            b.change_action(cx, n, 3, true);
        }
        2 => {
            let mut t = word(&own, s);
            s += 1;
            if t == -1 && !cx.annihilated() {
                t = 30;
            }
            b.stop_time = t;
            b.stop_counter = 0;
            if b.epitaph != 0 {
                b.change_action(cx, act::EPITAPH, 3, true);
            } else {
                b.change_action(cx, act::NEUTRAL, 1, true);
            }
        }
        9..=11 => {
            if pat != 11 {
                let ty = word(&own, s);
                s += 1;
                if ty < 0 {
                    // A living member's pick: those on the lists and up, one
                    // by ccRand (the first draw dropped).
                    let _ = cx.cc.rand();
                    let up: Vec<usize> =
                        cx.party.members.into_iter().flatten().filter(|&m| cx.valid(Some(m)) && !cx.dead(m)).collect();
                    if up.is_empty() {
                        exec_pattern(b, x, cx, 2, -1);
                        return s;
                    }
                    let r = cx.cc.rand().unsigned_abs() as usize;
                    cx.scene.chars[me].target_char = Some(up[r % up.len()]);
                } else {
                    select(b, cx, ty);
                }
            }
            if !target_ok(cx, cx.scene.chars[me].target_char) {
                let t = select(b, cx, 3);
                if !target_ok(cx, t) {
                    exec_pattern(b, x, cx, 2, 30);
                    return orig;
                }
            }
            b.calc_target_info(cx);
            if pat == 10 {
                // @1198 read, a skill drawn and dropped: OnThinkSkill casts
                // its own.
                let _ = (cx.cc.rand() % 3).abs();
                b.change_action(cx, act::SKILL, 3, true);
                return s;
            }
            let sid = word(&own, s);
            s += 1;
            let on = if pat == 9 { cx.scene.chars[me].target_char } else { Some(me) };
            if let Some(tp) = on
                && let Some(k) = item::item_skill_request(cx.t, cx.scene, me, tp, sid, 1, false, cx.rand)
            {
                cx.out(Out::Skill(tp, k));
            }
            exec_pattern(b, x, cx, 2, -2);
        }
        _ => return base_exec_pattern_index(b, x, cx, tbl, orig),
    }
    s
}

/// `ccBoss::ExecPatternIndex` (QUA gcmn, Outbreak's code).
fn base_exec_pattern_index(b: &mut Boss, x: &mut Macha, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
    let orig = i;
    let mut i = i;
    let mut pat = word(tbl, i);
    i += 1;
    b.move_spd = 0;
    b.move_vector = VF0;
    if pat == -1 {
        pat = word(tbl, 0);
        i = 1;
    }
    b.pat_num = pat;
    let me = cx.me;
    let neutral = |b: &mut Boss, cx: &mut Cx| {
        let n = if b.epitaph != 0 { act::EPITAPH } else { act::NEUTRAL };
        b.change_action(cx, n, 3, true);
    };
    match pat {
        2 => {
            let mut t = word(tbl, i);
            i += 1;
            if t == -1 && !cx.annihilated() {
                t = 30;
            }
            b.stop_time = t;
            b.stop_counter = 0;
            neutral(b, cx);
        }
        3 => b.change_action(cx, super::act::ESCAPE, 3, true),
        4 => b.change_action(cx, super::act::CHASE, 3, true),
        5 => b.change_action(cx, super::act::RETURN, 3, true),
        7 => b.change_action(cx, super::act::WANDER, 3, true),
        6 => {
            let d = ee::from_int(word(tbl, i));
            i += 1;
            let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), b.center_dirc), NEG_HALF_PI);
            let v = geom::apply_matrix(&m, [d, 0, 0, ONE]);
            let p = geom::vadd(v, cx.scene.chars[me].pos_p);
            b.dash_pos = super::p2w(cx, p);
            b.dash_prev_pos = cx.scene.chars[me].pos;
            b.change_action(cx, act::DASH_CENTER, 3, true);
        }
        8 => neutral(b, cx),
        9 | 11 => {
            let ty = word(tbl, i);
            i += 1;
            let sid = word(tbl, i);
            i += 1;
            if pat == 9 {
                let t = select(b, cx, ty);
                if !target_ok(cx, t) {
                    exec_pattern(b, x, cx, 2, 30);
                    return orig;
                }
                b.calc_target_info(cx);
            }
            let on = if pat == 9 { cx.scene.chars[me].target_char } else { Some(me) };
            if let Some(tp) = on
                && let Some(k) = item::item_skill_request(cx.t, cx.scene, me, tp, sid, 0, false, cx.rand)
            {
                cx.out(Out::Skill(tp, k));
            }
            exec_pattern(b, x, cx, 2, 30);
        }
        12 => {
            let sid = word(tbl, i);
            i += 1;
            let members = cx.party.members;
            let mut last = None;
            for m in members.into_iter().flatten() {
                if target_ok(cx, Some(m)) {
                    last = Some(m);
                }
            }
            for m in members.into_iter().flatten() {
                if Some(m) != last
                    && target_ok(cx, Some(m))
                    && let Some(k) = item::item_skill_request(cx.t, cx.scene, me, m, sid, 0, false, cx.rand)
                {
                    cx.out(Out::Skill(m, k));
                }
            }
            if let Some(l) = last
                && target_ok(cx, Some(l))
                && let Some(k) = item::item_skill_request(cx.t, cx.scene, me, l, sid, 1, false, cx.rand)
            {
                cx.out(Out::Skill(l, k));
            }
            exec_pattern(b, x, cx, 2, -2);
        }
        _ => neutral(b, cx),
    }
    i
}

// --- Think ----------------------------------------------------------------------------------

/// `ccBoss06::Think` (0x003a4310).
fn think(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    match b.act_num {
        act::NEUTRAL => on_neutral(b, x, cx, false),
        act::DAMAGE0 | act::DAMAGE1 => on_damage(b, cx),
        act::LIGHT_BALL => on_light_ball(b, x, cx),
        act::WAVE => on_wave(b, x, cx),
        act::CHARM => on_charm(b, x, cx),
        act::EYE_LASER => on_eye_laser(b, x, cx),
        act::DASH_CENTER => on_dash_center(b, x, cx),
        act::DRAIN => on_drain(b, x, cx),
        act::EPITAPH => on_neutral(b, x, cx, true),
        act::EPITAPH_WAVE => on_epitaph_wave(b, x, cx),
        act::DEAD => on_dead(b, x, cx),
        act::ESCAPE => on_escape(b, x, cx),
        act::CHASE => on_chase(b, x, cx),
        act::BACK_DASH => on_back_dash(b, cx),
        act::TRANSFER => on_transfer(b, x, cx),
        act::SKILL => on_skill(b, x, cx),
        act::DATA_DRAIN_ATK => on_data_drain_atk(b, x, cx),
        act::MAGIC => on_magic(b, x, cx),
        _ => b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// `OnThinkNeutral` (0x003a4980) and `OnThinkEpitaph` (0x003a76b0, which
/// also counts): turned to the target, the wait, then the next pattern at
/// the nearest.
fn on_neutral(b: &mut Boss, x: &mut Macha, cx: &mut Cx, epitaph: bool) {
    if epitaph {
        b.act_count += 1;
        b.move_spd = 0;
        b.move_vector = VF0;
        b.set_dirc(b.target_dirc);
    } else {
        b.set_dirc(b.target_dirc);
        b.move_spd = 0;
        b.move_vector = VF0;
    }
    if !b.wait_over(cx) {
        return;
    }
    let t = select(b, cx, 0);
    if !cx.valid(t) {
        return;
    }
    b.calc_target_info(cx);
    change_next_pattern(b, x, cx);
}

/// `OnThinkDamage` (0x003a4ae0): SE 243 once; back to the neutral as the
/// clip ends, the wait counted on through it.
fn on_damage(b: &mut Boss, cx: &mut Cx) {
    if b.act_proccess == 0 {
        b.act_proccess += 1;
        cx.se3d(243);
    }
    let (time, mut counter) = (b.stop_time, b.stop_counter);
    if time > 0 && time != counter {
        counter += 1;
    }
    if b.anm_status != 0 {
        b.change_action(cx, act::NEUTRAL, 0, true);
    }
    b.stop_time = time;
    b.stop_counter = counter;
}

/// `OnThinkChase` (0x003a4be0): after the target at 100 until within 300
/// or 301 frames, an after-image every fourth; out of the arena a dash of
/// 500.
fn on_chase(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    if c & 3 == 0 {
        // SetBlendTypeAfterImage(1), EntryAfterImage(0), (0); its SE 224 at
        // c == 1 cannot come.
        cx.out(Out::AfterImage);
    }
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 4);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.calc_target_info(cx);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.move_dirc = b.target_dirc;
            b.move_spd = 0x42c8_0000;
            b.act_proccess += 1;
        }
        1 => {
            if ee::lt(b.target_dist, 0x4396_0000) || c >= 301 {
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                change_next_pattern(b, x, cx);
                return;
            }
            let pp = cx.scene.chars[me].pos_p;
            if !super::fidchell::valid_area(b, cx, pp) {
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                exec_pattern(b, x, cx, 6, 500);
                return;
            }
        }
        _ => {}
    }
    b.move_dirc = b.target_dirc;
    b.dirc[2] = b.target_dirc;
}

/// `OnThinkDashCenter` (0x003a4e90): the base's dash (act 7) at 50 to
/// `dashPos`, 121 frames at most, off the targets.
fn on_dash_center(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    b.spd_up(0x4248_0000, 0x4248_0000);
    match b.act_proccess {
        0 => {
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.act_proccess += 1;
            cx.se3d(229);
        }
        1 => {
            if b.act_count & 3 == 3 {
                cx.out(Out::AfterImage);
                if b.act_count == 3 {
                    let pos = cx.pos();
                    cx.out(Out::Se3dNote { se: 57, pos, note: 55 });
                    cx.se3d(57);
                }
            }
            if b.act_count >= 121 {
                b.act_proccess += 1;
                return;
            }
            let p = super::w2p(cx, b.dash_pos);
            let pp = cx.scene.chars[me].pos_p;
            b.move_dirc = get_dirc(pp, p);
            if ee::lt(super::dist(cx, pp, p), 0x4348_0000) {
                b.act_proccess += 1;
            }
            b.dirc[2] = b.move_dirc;
            b.act_count += 1;
        }
        2 => {
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkSkill` (0x003a50a0): skill 161 on the target under the boss
/// camera's free eye, the stage darkened; its start at 45, the skill once
/// the start ends (15 frames at least), the stage back 50 frames on.
fn on_skill(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    b.set_dirc(b.target_dirc);
    match b.act_proccess {
        0 => {
            // SelectTarget's type is a1 as the switch left it: 4, the most
            // physical attack.
            let t = select(b, cx, 4);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.calc_target_info(cx);
            cx.out(Out::SwitchLayer);
            x.stage_eff = b.begin_stage_effect(cx, DARK, [15, 15, 30000]);
            let mut vp = b.target_pos;
            vp[2] = 0x4348_0000;
            free_cam(cx, vp, [0, 0, b.target_dirc, 0], [0x447a_0000, 0, 0, ONE]);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.lock_player(cx, true);
            x.skill_id = SKILL;
            cx.out(Out::CinemaSkill(x.skill_id));
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 => {
            if c < 45 {
                return;
            }
            cx.out(Out::SkillStart { who: me, sid: x.skill_id });
            x.eff_start = Some(super::fidchell::eff::SKILL_START_LIFE);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2 => {
            // The start's controller: its endFlag once its life is out.
            if let Some(n) = x.eff_start {
                x.eff_start = (n > 1).then_some(n - 1);
            }
            if c < 15 || x.eff_start.is_some() {
                return;
            }
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            if let Some(tp) = cx.scene.chars[me].target_char
                && let Some(k) = item::item_skill_compel(cx.t, cx.scene, me, tp, x.skill_id, 0, false, cx.rand)
            {
                cx.out(Out::Skill(tp, k));
            }
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        3 if c == 50 => {
            let id = std::mem::replace(&mut x.stage_eff, -1);
            b.end_stage_effect(cx, id, DARK, 15);
            b.act_proccess += 1;
            b.act_count = 0;
        }
        4 if c == 15 => {
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.out(Out::CamMode { mode: 6 });
            cx.out(Out::SwitchLayer);
            cx.out(Out::Cinema(None));
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkEscape` (0x003a55e0): away from the target at 30, turned a
/// quarter off it every 32 frames (toward the centre out of the arena),
/// for 300 frames.
fn on_escape(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    b.move_spd = 0x41f0_0000;
    if c & 31 == 0 {
        let off = libm::fabsf(ee::sub(b.target_dirc, b.center_dirc));
        let mut r =
            if ee::lt(off, HALF_PI) { ee::sub(b.target_dirc, HALF_PI) } else { ee::add(HALF_PI, b.target_dirc) };
        if ee::le(r, PI) && ee::lt(r, NEG_PI) {
            r = ee::add(r, TWO_PI);
        }
        b.m_act_dirc[2] = r;
    }
    b.move_dirc = geom::set_dirc(b.move_dirc, b.m_act_dirc[2], 256);
    let pp = cx.scene.chars[me].pos_p;
    if !super::fidchell::valid_area(b, cx, pp) {
        b.m_act_dirc[2] = b.center_dirc;
    }
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 0);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.calc_target_info(cx);
            let pp = cx.scene.chars[me].pos_p;
            if !super::fidchell::valid_area(b, cx, pp) {
                b.move_dirc = b.center_dirc;
            }
            b.act_proccess += 1;
        }
        1 => {
            if c == 300 {
                change_next_pattern(b, x, cx);
                return;
            }
            if b.stop != 0 || cx.scene.chars[me].cond[cond::HOLD] != 0 {
                b.move_spd = 0;
            }
        }
        _ => {}
    }
    b.dirc[2] = b.move_dirc;
}

/// `OnThinkTransfer` (0x003a58a0), which nothing reaches: off the targets,
/// the blur's callback swapped for `ccBoss06BlurParamFuncCB`, a fade out
/// to `m_transferPos`, a fade in, the blur eased home, then the neutral.
fn on_transfer(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            x.org_blur = x.blur_func;
            x.blur_func = Some(BlurFunc::Boss06);
            cx.out(Out::Blur(Set::Func(BlurFunc::Boss06)));
            cx.out(Out::BlurSinRot(NEG_HALF_PI));
            b.act_proccess += 1;
        }
        1 => {
            let mut t = ee::sub(b.set_transparency, 0x3c36_0b61);
            if ee::lt(t, 0) {
                t = 0;
                cx.scene.chars[me].pos = x.transfer_pos;
                b.act_proccess += 1;
            }
            b.transparency = t;
            b.set_transparency = t;
        }
        2 => {
            let mut t = ee::add(b.set_transparency, 0x3d88_8889);
            if !ee::le(t, ONE) {
                t = ONE;
                cx.out(Out::Blur(Set::Ease));
                b.act_proccess += 1;
            }
            b.transparency = t;
            b.set_transparency = t;
        }
        3 if cx.disc.blur_exit != 0 => {
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            b.change_action(cx, act::NEUTRAL, 1, false);
            if let Some(f) = x.org_blur {
                cx.out(Out::Blur(Set::Func(f)));
            }
            x.blur_func = x.org_blur;
            cx.out(Out::Blur(Set::Enable));
            x.transfer_pos = b.center_pos;
        }
        _ => {}
    }
    b.set_dirc(b.target_dirc);
}

/// `OnThinkBackDash` (0x003a5b60), which nothing reaches: back from the
/// camera at 150, zigzagging a third of a turn every 10 frames with dust,
/// for 31 frames; out of the arena it stops at once.
fn on_back_dash(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    if c == 0 {
        b.erase_cmnd_target(cx);
        cursor(cx, true);
        b.spd_up(0x4316_0000, 0x4316_0000);
        let pp = cx.scene.chars[me].pos_p;
        if !super::fidchell::valid_area(b, cx, pp) {
            b.move_dirc = wrap(ee::add(PI, cx.cam.rot[2]));
        }
        b.dirc[2] = b.move_dirc;
        return;
    }
    let pp = cx.scene.chars[me].pos_p;
    if !super::fidchell::valid_area(b, cx, pp) {
        b.entry_cmnd_target(cx);
        cursor(cx, false);
        b.move_spd = 0;
        b.move_vector = VF0;
        b.change_action(cx, act::NEUTRAL, 1, false);
        return;
    }
    if b.act_count % 10 == 0 {
        let r = cx.cam.rot[2];
        b.move_dirc = if b.act_proccess != 0 { ee::add(0x3f86_0a92, r) } else { ee::sub(r, 0x3f86_0a92) };
        b.act_proccess ^= 1;
        b.move_dirc = wrap(b.move_dirc);
        let vel =
            [ee::mul(0x4040_0000, libm::cosf(b.move_dirc)), ee::mul(0x4040_0000, libm::sinf(b.move_dirc)), 0, ONE];
        let pos = cx.pos();
        cx.out(Out::Macha(Pic::Smoke { pos, vel }));
    }
    if b.act_count >= 31 {
        b.entry_cmnd_target(cx);
        cursor(cx, false);
        b.move_spd = 0;
        b.move_vector = VF0;
        b.change_action(cx, act::NEUTRAL, 3, false);
    }
    b.dirc[2] = b.move_dirc;
}

/// `OnThinkLightBall` (0x003a5f10), on effLayer: the ball charged over the
/// head with a light circling it, then thrown at the target, quickening by
/// 2.5 a frame to 500; boss skill 21 where it lands, and its shock.
fn on_light_ball(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    let f = b.anm.frame();
    if f < 41 && f & 3 == 0 {
        cx.out(Out::AfterImageWith(WHITE_IMAGE));
    }
    match b.act_proccess {
        0 => {
            x.ball_speed = 0x42c8_0000;
            x.anm_ball_charge.set(BALL_CHARGE, cx.clips);
            x.anm_ball.set(BALL, cx.clips);
            x.ball_pos = cx.pos();
            x.ball_pos[2] = 0x4409_8000;
            // The light made at the origin (ccLight's unit matrix); it
            // follows the ball from frame 20.
            x.light = true;
            cx.out(Out::Macha(Pic::Light(Some([0, 0, 0, ONE]))));
            x.light_rot = rand_f(cx.cc, PI);
            b.lock_player(cx, true);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            cx.out(Out::Cinema(Some(21)));
            cx.out(Out::SwitchLayer);
            x.stage_eff = b.begin_stage_effect(cx, DARK, [15, 15, 30000]);
            let mut vp = cx.pos();
            vp[2] = ee::add(vp[2], 0x4396_0000);
            let d = [b.dirc[0], b.dirc[1], b.target_dirc, b.dirc[3]];
            free_cam(cx, vp, d, [0x44fa_0000, 0, 0, ONE]);
            b.dirc[2] = b.target_dirc;
            cx.out(Out::Se { se: 242 });
            b.act_proccess += 1;
        }
        1 => {
            if f < 20 {
                return;
            }
            let done = x.anm_ball_charge.forward();
            let mut p = cx.pos();
            p[2] = 0x4409_8000;
            cx.out(Out::Macha(Pic::Ball { charge: true, time: x.anm_ball_charge.posed, pos: p, dirc: b.dirc }));
            let mut p = cx.pos();
            p[2] = 0x4396_0000;
            x.light_rot = ee::add(x.light_rot, 0x3e56_7750);
            if !ee::le(x.light_rot, PI) {
                x.light_rot = ee::sub(x.light_rot, TWO_PI);
            }
            let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI), x.light_rot);
            let v = geom::apply_matrix(&m, [0x4348_0000, 0, 0, ONE]);
            let p = super::p2w(cx, geom::vadd(super::w2p(cx, p), v));
            cx.out(Out::Macha(Pic::Light(Some(p))));
            if f == 20 {
                cx.out(Out::SeNote { se: 246, note: 55 });
            }
            if done {
                cx.out(Out::SeNote { se: 57, note: 60 });
                cx.out(Out::Macha(Pic::Light(None)));
                x.light = false;
                b.act_proccess += 1;
            }
        }
        2 => {
            let a = wrap(libm::atan2f(0x4409_8000, b.target_dist));
            let m = geom::rot_matrix_y(&geom::unit_matrix(), a);
            let m = geom::rot_matrix_z(&m, b.target_dirc);
            let m = geom::rot_matrix_z(&m, NEG_HALF_PI);
            let v = geom::apply_matrix(&m, [x.ball_speed, 0, 0, ONE]);
            x.ball_pos = super::p2w(cx, geom::vadd(super::w2p(cx, x.ball_pos), v));
            x.anm_ball.forward();
            cx.out(Out::Macha(Pic::Ball { charge: false, time: x.anm_ball.posed, pos: x.ball_pos, dirc: b.dirc }));
            if f == 140 {
                let vp = b.target_pos;
                free_cam(cx, vp, b.dirc, [0x44bb_8000, 0, 0x42c8_0000, ONE]);
            }
            x.ball_speed = ee::add(x.ball_speed, 0x4020_0000);
            if !ee::le(x.ball_speed, 0x43fa_0000) {
                x.ball_speed = 0x43fa_0000;
            }
            if ee::le(x.ball_pos[2], 0) {
                b.act_proccess += 1;
                let tp = b.target_pos;
                b.skill_damage(cx, None, Some(tp), 21);
                x.ball_pos[2] = 0;
                let pos = x.ball_pos;
                // ccBossEffLightBallShock's constructor makes its ring
                // first: (0, 2/3, 10), model 180.
                let param = [0, 0x3f2a_aaab, 0x4120_0000, 0];
                b.effect_at(cx, EffKind::AutoSamonRingOf { n: 180, param }, pos, [0, 0, 0, ONE]);
                make_fx(b, cx, EffKind::LightBallShock, Fx::Shock(eff::Shock::new(pos)), pos);
                cx.out(Out::SeNote { se: 35, note: 60 });
                b.act_count = 0;
            }
        }
        3 => {
            let c = b.act_count;
            b.act_count += 1;
            if c < 60 || b.anm_status == 0 {
                return;
            }
            cx.out(Out::CamMode { mode: 6 });
            let id = std::mem::replace(&mut x.stage_eff, -1);
            b.end_stage_effect(cx, id, DARK, 15);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        4 => {
            let c = b.act_count;
            b.act_count += 1;
            if c != 15 {
                return;
            }
            cx.out(Out::Cinema(None));
            cx.out(Out::SwitchLayer);
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            b.unlock_player();
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
    let _ = me;
}

/// `OnThinkWave` (0x003a6ad0): the wave twice its size at 40 frames of the
/// clip, a clear flash, boss skill 22 15 frames on with Macha's SP drain
/// at 100.
fn on_wave(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            x.anm_w.set(super::ANM_WAVE, cx.clips);
            x.anm_w.speed = 512;
            b.act_proccess += 1;
            let pos = cx.pos();
            cx.out(Out::Se3dNote { se: 242, pos, note: 62 });
        }
        1 => {
            let f = b.anm.frame();
            if (f as u32) < 40 {
                return;
            }
            if f == 40 {
                let (pos, dirc) = (cx.pos(), b.dirc);
                b.effect_at(cx, EffKind::WaveShock, pos, dirc);
                cx.out(Out::Flash { t: 15, colour: 0x00e0_ffff });
            } else {
                let c = b.act_count;
                b.act_count += 1;
                if c == 15 {
                    wave_damage(b, cx);
                }
            }
            let done = x.anm_w.forward();
            let mut pos = cx.pos();
            pos[2] = ee::add(pos[2], 0x4248_0000);
            cx.out(Out::Macha(Pic::Wave { time: x.anm_w.posed, pos, dirc: b.dirc, scale: 0x4000_0000 }));
            if done && b.anm_status != 0 {
                change_next_pattern(b, x, cx);
            }
            if cx.cam.shake {
                cx.quake_cam(0x420c_0000);
            }
        }
        _ => {}
    }
    let _ = me;
}

/// Boss skill 22 at the boss with its `DRAIN_SP` at 100 (`condition[3]`),
/// both drains cleared after.
fn wave_damage(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    cx.scene.chars[me].cond[cond::DRAIN_SP] = 100;
    let pos = cx.pos();
    b.skill_damage(cx, None, Some(pos), 22);
    cx.scene.chars[me].cond[cond::DRAIN_HP] = 0;
    cx.scene.chars[me].cond[cond::DRAIN_SP] = 0;
}

/// `OnThinkEpitaphWave` (0x003a7810): the Epitaph's wave, a flash at 35
/// frames of the clip, boss skill 22 15 frames on.
fn on_epitaph_wave(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            x.anm_w.set(super::ANM_WAVE, cx.clips);
            x.anm_w.speed = 512;
            b.act_proccess += 1;
        }
        1 => {
            let f = b.anm.frame();
            if f == 16 {
                cx.se3d(223);
                return;
            }
            if f == 30 {
                cx.se3d(224);
                return;
            }
            if (f as u32) < 35 {
                return;
            }
            let c = b.act_count;
            b.act_count += 1;
            // The count read again after the step: the blow at its 15th.
            if c == 0 {
                let (pos, dirc) = (cx.pos(), b.dirc);
                b.effect_at(cx, EffKind::WaveShockScaled { scale: 0x4000_0000 }, pos, dirc);
                cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
            } else if b.act_count == 15 {
                wave_damage(b, cx);
            }
            let done = x.anm_w.forward();
            let pos = cx.pos();
            cx.out(Out::Macha(Pic::Wave { time: x.anm_w.posed, pos, dirc: b.dirc, scale: ONE }));
            if b.anm_status != 0 && done {
                b.change_action(cx, act::EPITAPH, 3, true);
            }
            if cx.cam.shake {
                cx.quake_cam(0x420c_0000);
            }
        }
        _ => {}
    }
}

/// `OnThinkCharm` (0x003a6d70): with any member up, the stage turned pink
/// under the boss camera's free eye; at 95 frames of the clip skill 159
/// compelled on each member up; back as the clip ends.
fn on_charm(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    let f = b.anm.frame();
    if f & 7 == 0 {
        cx.out(Out::AfterImageWith(WHITE_IMAGE));
    }
    match b.act_proccess {
        0 => {
            let up = cx.party.members.into_iter().flatten().any(|m| cx.valid(Some(m)) && !cx.dead(m));
            if !up {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.lock_player(cx, true);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            cx.out(Out::Cinema(Some(69)));
            cx.out(Out::SwitchLayer);
            x.stage_eff = b.begin_stage_effect(cx, PINK, [15, 15, 30000]);
            b.dirc[2] = b.target_dirc;
            let mut vp = cx.pos();
            vp[2] = ee::add(vp[2], 0x4348_0000);
            free_cam(cx, vp, [0, 0, b.target_dirc, 0], [0x447a_0000, 0, 0x4316_0000, ONE]);
            b.act_proccess += 1;
        }
        1 => {
            match f {
                2 => cx.out(Out::SeNote { se: 242, note: 59 }),
                40 => cx.out(Out::SeNote { se: 242, note: 62 }),
                70 => cx.out(Out::SeNote { se: 242, note: 60 }),
                _ => {}
            }
            if f == 95 {
                for m in cx.party.members.into_iter().flatten() {
                    if target_ok(cx, Some(m))
                        && let Some(k) = item::item_skill_compel(cx.t, cx.scene, me, m, 159, 0, false, cx.rand)
                    {
                        cx.out(Out::Skill(m, k));
                    }
                }
                return;
            }
            if b.anm_status != 0 {
                let id = std::mem::replace(&mut x.stage_eff, -1);
                b.end_stage_effect(cx, id, PINK, 15);
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        2 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 15 {
                cx.out(Out::Cinema(None));
                cx.out(Out::SwitchLayer);
                cx.out(Out::CamMode { mode: 6 });
                b.unlock_player();
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkEyeLaser` (0x003a72f0): a laser from each eye at where the
/// target stands, at 100; once the first has struck the land, a blow of
/// nothing on the target and the neutral.
fn on_eye_laser(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            let tp = b.target_pos;
            let [r, l] = x.eyes;
            let first = make_fx(b, cx, EffKind::LaserShot, Fx::Laser(eff::Laser::new(cx, r, tp, 0x42c8_0000)), r);
            make_fx(b, cx, EffKind::LaserShot, Fx::Laser(eff::Laser::new(cx, l, tp, 0x42c8_0000)), l);
            x.eff_laser = Some(first);
            b.act_proccess += 1;
        }
        1 => {
            if b.effects.enabled(x.eff_laser) {
                return;
            }
            if let Some(t) = cx.scene.chars[me].target_char.filter(|&t| cx.valid(Some(t))) {
                cx.affect(t, 1, 0);
            }
            b.change_action(cx, act::NEUTRAL, 1, true);
        }
        _ => {}
    }
}

/// `OnThinkDead` (0x003a7500): off the lists, the dead effect; exit once
/// it ends.
fn on_dead(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            crate::fellow::delete_cmnd(cx.scene, cx.me);
            cx.out(Out::DeleteCmnd);
            x.eff_dead = Some(b.begin_dead_effect(cx, 0x44fa_0000, 0x44fa_0000));
            b.act_proccess += 1;
        }
        1 if !b.effects.enabled(x.eff_dead) => b.exit = 1,
        _ => {}
    }
}

/// `OnThinkDrain` (0x003a75d0): drained; 6000 HP, the Epitaph's table,
/// act 12.
fn on_drain(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    b.cheat_hp = 0;
    cx.scene.chars[me].hp = EPITAPH_HP;
    cx.scene.chars[me].max_hp = EPITAPH_HP;
    set_pattern_mode(b, x, cx, 3);
    b.change_action(cx, act::EPITAPH, 3, true);
    b.move_spd = 0;
}

/// `OnThinkDataDrainAtk` (0x003a7aa0): a member drained through menu 74
/// (stream 98).
fn on_data_drain_atk(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 0);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.calc_target_info(cx);
            b.lock_player(cx, false);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            cx.out(Out::Cinema(Some(-1)));
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 if c == 45 => {
            let id = cx.scene.chars[me].target_char.map_or(-1, |t| i32::from(cx.scene.chars[t].id()));
            let slot = cx.party.slot_of(id);
            cx.out(Out::StreamMenu { stream: 98, mask: 1 << slot });
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2 if cx.env.menu_type == -1 => {
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            let t = cx.scene.chars[me].target_char;
            if let Some(tp) = t
                && target_ok(cx, t)
            {
                cx.affect(tp, 13, 0);
            }
            cx.out(Out::Cinema(None));
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkMagic` (0x003a7d30): one of `@1913` by the frame count on the
/// target under the boss camera's free eye, the stage darkened; the spell
/// at 45, the stage back when it is over (90-150 frames by spell).
fn on_magic(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 0);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.calc_target_info(cx);
            cx.out(Out::SwitchLayer);
            x.stage_eff = b.begin_stage_effect(cx, DARK, [15, 15, 3000]);
            b.lock_player(cx, true);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            x.skill_id = cx.data.macha.magic_skills[(cx.env.count & 3) as usize];
            cx.out(Out::CinemaSkill(x.skill_id));
            let mut vp = b.target_pos;
            vp[2] = 0x4348_0000;
            free_cam(cx, vp, [0, 0, b.target_dirc, 0], [0x44fa_0000, 0, 0, ONE]);
            b.act_proccess += 1;
        }
        1 if c == 45 => {
            if let Some(tp) = cx.scene.chars[me].target_char
                && let Some(k) = item::item_skill_request(cx.t, cx.scene, me, tp, x.skill_id, 1, false, cx.rand)
            {
                cx.out(Out::Skill(tp, k));
            }
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2 => {
            let wait = match x.skill_id {
                260 | 220 => 120,
                204 => 150,
                228 => 90,
                _ => -1,
            };
            if c == wait {
                let id = std::mem::replace(&mut x.stage_eff, -1);
                b.end_stage_effect(cx, id, DARK, 15);
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        3 if c == 15 => {
            cx.out(Out::Cinema(None));
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.out(Out::SwitchLayer);
            b.unlock_player();
            cx.out(Out::CamMode { mode: 6 });
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

// --- Affect ---------------------------------------------------------------------------------

/// `ccBoss06::Affect` (0x003a44a0): its own hits (1 and 3), the base's for
/// the rest. A hit lands whatever `actForbid` or the player's lock; under
/// the cheat it takes a tenth, never below half. At half its gauge in the
/// normal table, the expert table.
pub(super) fn affect(b: &mut Boss, cx: &mut Cx) {
    let mut x = macha_of(b);
    affect_of(b, &mut x, cx);
    b.class = Class::Macha(x);
}

fn affect_of(b: &mut Boss, x: &mut Macha, cx: &mut Cx) {
    let me = cx.me;
    let (t, p0, person) = {
        let a = &cx.scene.chars[me].affect;
        (a.ty, a.param[0], a.person)
    };
    if t != 1 && t != 3 {
        b.base_affect(cx);
        return;
    }
    if cx.annihilated() || cx.game_over {
        return;
    }
    cx.out(Out::FlyFont { kind: 2, n: i32::from(p0) });
    if p0 < 0 {
        return;
    }
    cx.out(Out::HitMark { by: person });
    let (hp, mhp) = (cx.scene.chars[me].hp, cx.scene.chars[me].max_hp);
    let mut h = if b.cheat_hp != 0 { (hp - p0 / 10).max(mhp / 2) } else { hp - p0 };
    if h <= 0 {
        h = 0;
        cx.out(Out::ClearSpcCondition);
        crate::affect::clear_conditions(&mut cx.scene.chars[me]);
        b.move_spd = 0;
        b.move_vector = VF0;
        b.change_action(cx, act::DEAD, 2, true);
    } else if t == 1 && b.act_num == act::NEUTRAL {
        let (time, counter) = (b.stop_time, b.stop_counter);
        let n = if cx.env.count & 1 != 0 { act::DAMAGE0 } else { act::DAMAGE1 };
        b.change_action(cx, n, 0, true);
        b.stop_time = time;
        b.stop_counter = counter;
    }
    cx.scene.chars[me].hp = h;
    let pp = cx.scene.param(me).map_or(0, |p| p.pp);
    let id = usize::try_from(cx.scene.chars[me].id()).unwrap_or(0);
    let max = cx.t.bosses.get(id).map_or(0, |r| r.max_pp);
    if pp < max >> 1 || x.pat_mode != 0 {
        return;
    }
    set_pattern_mode(b, x, cx, 2);
}
