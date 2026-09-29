//! Fidchell (`ccBoss04`, boss04.cpp), Outbreak's first phase boss: `bossTbl`
//! row 3, `bossFunc` code 3, fought in event 207 (M307) in field 4. Ported
//! from Outbreak's gcmn.prg (0x004a3470-0x004a900c; names and layouts from
//! Infection's DWARF, whose gcmn carries the class unused), with Outbreak's
//! `ccBoss` base under it ([`frame`]). It predicts one of four spells, then
//! casts it; the spells are effects of their own ([`eff`]).
//! docs/engine/boss-fidchell.md.

pub mod eff;
#[cfg(test)]
mod tests;

use piney_data::field::ee;
use piney_data::libm;
use piney_data::volume::Volume;

use self::eff::{Fx, MeteoSworm, RockTower, ThunderStorm};
use super::{Anm, Boss, Class, Cx, EffKind, Out, Tbl, VF0};
use crate::enemy_ai::get_dirc;
use crate::geom::{self, V4};
use crate::item;
use crate::param::cond;

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const NEG_PI: F = 0xc049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// `SelectTarget`'s radius here: 999999.
const FAR: F = 0x4974_23f0;

/// `bossFunc`'s code for Fidchell, its `bossTbl` row and its file.
pub const CODE: i32 = 3;
pub const ROW: usize = 3;
pub const FILE: &str = "x41";
/// The drain's HP (`Affect`'s 21).
pub const EPITAPH_HP: i16 = 5000;
/// `bossBlur`'s colour under `ccBoss03BlurParamCB` (OUT gcmn 0x00472b90).
pub const BLUR: u32 = 0x4880_8080;
/// `InitBossCamera(250, 1500)`'s `MaxRenge`, set after it (Outbreak's 600).
pub const CAM_MAX_RANGE: F = 0x4416_0000;

/// Fidchell's acts (`actNum`, `Think`'s switch at 0x004a3f00; any other is
/// `ChangeAction(0, 0, 4)`).
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DMG0: i16 = 1;
    pub const DMG1: i16 = 2;
    pub const PREDICTION: i16 = 3;
    pub const WAVE: i16 = 4;
    pub const EXEC_PREDICTION: i16 = 5;
    pub const DRAIN_ATK: i16 = 6;
    pub const DASH_CENTER: i16 = 7;
    pub const DATA_DRAIN: i16 = 11;
    pub const EPITAPH: i16 = 12;
    pub const EPITAPH_WAVE: i16 = 13;
    pub const DEAD: i16 = 14;
    pub const ESCAPE: i16 = 16;
    pub const CHASE: i16 = 18;
    pub const WANDER: i16 = 20;
    pub const TRANSFER: i16 = 21;
    pub const BACK_DASH: i16 = 22;
    pub const JUMP: i16 = 23;
    pub const SKILL: i16 = 24;
    pub const MAGIC: i16 = 25;
}

/// The event number the prediction's voice is asked with
/// (`ccEvVoiceRequest(-40, n)`, OUT gcmn 0x004a51c0): a field voice group.
pub const PRED_VOICE_EVENT: i32 = -40;

/// Fidchell's tables (`tables::combat`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FidchellData {
    /// `boss04NormalActTbl`, `boss04SuperActTbl`, `boss04EpitaphActTbl`, each
    /// to its -1.
    pub normal: Vec<i32>,
    pub super_: Vec<i32>,
    pub epitaph: Vec<i32>,
    /// `boss0xAnmTbl`: the clip by act.
    pub anims: Vec<Option<String>>,
    /// `@1038`, `@1039`: the prediction's text clip and voice by `m_predId`.
    pub pred_texts: Vec<Option<String>>,
    pub pred_voices: [i32; 4],
    /// `@1441`: the skill the prediction lays on each member.
    pub pred_skills: [i32; 4],
    /// `@2048`: `OnThinkSkill`'s; `@2081`: `OnThinkMagic`'s.
    pub skills: [i32; 3],
    pub magic_skills: [i32; 4],
}

impl FidchellData {
    /// The volume's.
    pub fn of(volume: Volume) -> FidchellData {
        let t = piney_data::tables::combat::of(volume);
        let s = |v: &[Option<&str>]| v.iter().map(|a| a.map(str::to_string)).collect::<Vec<_>>();
        let pick = |v: &[i32], k: usize| v.get(k).copied().unwrap_or(0);
        FidchellData {
            normal: t.fidchell_normal().to_vec(),
            super_: t.fidchell_super().to_vec(),
            epitaph: t.fidchell_epitaph().to_vec(),
            anims: s(t.fidchell_anims()),
            pred_texts: s(t.fidchell_pred_texts()),
            pred_voices: std::array::from_fn(|k| pick(t.fidchell_pred_voices(), k)),
            pred_skills: std::array::from_fn(|k| pick(t.fidchell_pred_skills(), k)),
            skills: std::array::from_fn(|k| pick(t.fidchell_skills(), k)),
            magic_skills: std::array::from_fn(|k| pick(t.fidchell_magic_skills(), k)),
        }
    }

    fn words(&self, t: Tbl) -> &[i32] {
        match t {
            Tbl::None => &[],
            Tbl::Normal => &self.normal,
            Tbl::Super => &self.super_,
            Tbl::Epitaph => &self.epitaph,
        }
    }
}

/// `PREDICTION_T` (`m_predInfo`, +0x29530): the spell's camera and fade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PredInfo {
    pub cam_pos: V4,
    pub cam_view: V4,
    pub quake_sw: i8,
    pub quake_offset: F,
    pub transparency: F,
    pub act_count: i32,
    /// `revLayer` made (the ice's reversed layer).
    pub rev_layer: bool,
}

impl Default for PredInfo {
    fn default() -> PredInfo {
        PredInfo {
            cam_pos: VF0,
            cam_view: VF0,
            quake_sw: 0,
            quake_offset: 0,
            transparency: ONE,
            act_count: 0,
            rev_layer: false,
        }
    }
}

/// What Fidchell shows or asks beside the other bosses' calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pic {
    /// The prediction's text (`m_predTxt.anmText` of x41) drawn this frame
    /// on its own layer (254) at `pos`, turned by `dirc`.
    Text {
        clip: String,
        time: u32,
        pos: V4,
        dirc: V4,
    },
    /// `ccEvVoiceRequest(PRED_VOICE_EVENT, n)`, `ccEvVoiceStop()`.
    Voice(i32),
    VoiceStop,
    /// `effSmoke(pos, vel, size, 60, 114, 512, 32)`: the back dash's dust
    /// (size 8), a rock tower breaking out (10).
    Smoke {
        pos: V4,
        vel: V4,
        size: F,
    },
    /// A meteor's landing (`effSmokeRock`, two `ccParticleExplode`).
    MeteorLand {
        pos: V4,
    },
    /// `effRadiateSomething2(pos, rot, 4, 4, 45)`: a tower's rocks.
    Radiate {
        pos: V4,
        rot: V4,
    },
    /// A thunder's strike (`ccBossEffThunder2::Shock`): `effFlareRing`,
    /// four `effRadiateSomething2` turned by `turn`, five
    /// `ccParticleExplode` at `vels`.
    Shock {
        pos: V4,
        turn: F,
        vels: [V4; 5],
    },
}

/// `ccBoss04`'s own members (+0x29350 on).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fidchell {
    /// `anmw`: `xeffect`'s `ANM_xx11wave`.
    pub anm_w: Anm,
    /// `m_predTxt`: the text's `ccAnm`, its place (+0x29480) and turn.
    pub pred_txt: Anm,
    pub pred_pos: V4,
    pub pred_dirc: V4,
    /// `m_effMagic`, `m_effDead`: the effects waited on, by slot.
    pub eff_magic: Option<i32>,
    pub eff_dead: Option<i32>,
    /// `m_br`: the ice's `ccBufferReverce` made.
    pub br: bool,
    /// `m_effStart`: `effSkillStart`'s controller (-12, life 7), the frames
    /// before its `endFlag` is seen.
    pub eff_start: Option<i32>,
    pub act_sub_proccess: i32,
    pub act_sub_count: i32,
    pub stage_eff_id: i32,
    pub pred_id: i32,
    pub reserve_pred: i32,
    pub rot_z: F,
    pub transparency: F,
    pub skill_id: i32,
    pub pred: PredInfo,
    pub pat_mode: i32,
    /// Camera 3's eye and view as Fidchell last set them, which its own
    /// `cameraGetPos(3)` and `cameraGetView(3)` read back.
    pub cam3: (V4, V4),
}

fn fidchell_of(b: &mut Boss) -> Box<Fidchell> {
    match std::mem::replace(&mut b.class, Class::Plain) {
        Class::Fidchell(x) => x,
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

/// `ccGetDist(a, b)` as the volume's main has it ([`super::sqrt_of`]).
fn dist(cx: &Cx, a: V4, b: V4) -> F {
    let x = ee::sub(b[0], a[0]);
    let y = ee::sub(b[1], a[1]);
    super::sqrt_of(cx, ee::add(ee::add(ee::mul(x, x), ee::mul(y, y)), ee::mul(0, 0)))
}

fn target_ok(cx: &Cx, t: Option<usize>) -> bool {
    cx.valid(t) && t.is_some_and(|t| !cx.dead(t))
}

fn held(b: &Boss, cx: &Cx) -> bool {
    b.stop != 0 || cx.scene.chars[cx.me].cond[cond::HOLD] != 0
}

/// `if (ccMenu) ccMenu->cursolOff = on`.
fn cursor(cx: &mut Cx, on: bool) {
    cx.out(Out::CursorOff(on));
}

fn cinema_off(cx: &mut Cx) {
    cx.out(Out::Cinema(None));
}

/// `ccTransPosP2W(W2P(from) + m v)`: a place `v` off `from`, turned by `m`.
fn off_from(cx: &Cx, from: V4, m: &geom::M4, v: V4) -> V4 {
    let v = geom::apply_matrix(m, v);
    super::p2w(cx, geom::vadd(super::w2p(cx, from), v))
}

/// `Rz(-pi/2)` then `Rz(r)` on the unit matrix.
fn behind(r: F) -> geom::M4 {
    let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
    geom::rot_matrix_z(&m, r)
}

fn set_cam3(x: &mut Fidchell, cx: &mut Cx, pos: V4, view: V4) {
    cx.out(Out::CameraPos { cam: 3, pos });
    cx.out(Out::CameraView { cam: 3, view });
    x.cam3 = (pos, view);
}

/// `ccBoss::CalcTargetInfo` (OUT gcmn 0x0046ff20) with the volume's
/// `ccGetDist`.
fn calc_target_info(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    let Some(t) = cx.scene.chars[me].target_char.filter(|&t| cx.scene.listed(t)) else { return };
    b.target_pos_p = cx.scene.chars[t].pos_p;
    b.target_pos = cx.scene.chars[t].pos;
    let pp = cx.scene.chars[me].pos_p;
    b.target_dirc = get_dirc(pp, b.target_pos_p);
    b.target_dist = dist(cx, pp, b.target_pos_p);
}

fn select(b: &mut Boss, cx: &mut Cx, ty: i32) -> Option<usize> {
    let t = b.select_target(cx, ty, FAR);
    cx.scene.chars[cx.me].target_char = t;
    t
}

/// `ccBoss::IsValidArea(p)` (OUT gcmn 0x0046f100): the world place's x and
/// y each under the centre's plus 2500, their signs lost to `fabs`.
fn valid_area(b: &Boss, cx: &Cx, p: V4) -> bool {
    let w = super::p2w(cx, p);
    let lx = ee::sub(ee::add(0x453b_8000, b.center_pos[0]), 0x43fa_0000);
    let ly = ee::sub(ee::add(0x453b_8000, b.center_pos[1]), 0x43fa_0000);
    ee::lt(super::kyvia::fabs(w[0]), lx) && ee::lt(super::kyvia::fabs(w[1]), ly)
}

/// An angle into -pi..pi: below -pi a turn on, else above pi a turn off.
fn wrap_lo(v: F) -> F {
    if ee::lt(v, NEG_PI) {
        ee::add(v, TWO_PI)
    } else if !ee::le(v, PI) {
        ee::sub(v, TWO_PI)
    } else {
        v
    }
}

// --- the constructor ------------------------------------------------------------------------

/// `ccBoss04::ccBoss04` (OUT gcmn 0x004a3590): the boss is
/// `scene.chars[cx.me]` (`bossTbl` row 3); it stands 500 behind Kite and
/// 10 up. `center` is the arena's `DMY_center01`.
pub fn new(cx: &mut Cx, kite_pos: V4, kite_dirc: V4, center: V4) -> Boss {
    let me = cx.me;
    let mut b = super::kyvia::plain_boss();
    b.anm_tbl = cx.data.fidchell.anims.clone();
    // OffExit, OnDraw, OnBodyHit, OnCheatHP.
    b.exit = 0;
    b.draw_sw = 1;
    b.body_hit_sw = 1;
    b.cheat_hp = 1;
    let mut pos = kite_pos;
    pos[1] = ee::sub(pos[1], 0x43fa_0000);
    pos[2] = ee::add(pos[2], 0x4120_0000);
    cx.scene.chars[me].pos = pos;
    b.dirc = kite_dirc;
    b.change_action(cx, act::NEUTRAL, 0, true);
    let x = Fidchell {
        anm_w: Anm::new(),
        pred_txt: Anm::new(),
        pred_pos: VF0,
        pred_dirc: VF0,
        stage_eff_id: -1,
        pred: PredInfo::default(),
        cam3: (VF0, VF0),
        ..Fidchell::default()
    };
    crate::fellow::entry_cmnd(cx.scene, me);
    // InitCenterPos; InitStageEffect; InitBossCamera(250, 1500), its
    // MaxRenge 600; the blur on under ccBoss03BlurParamCB.
    b.center_pos = [center[0], center[1], center[2], ONE];
    b.center_pos_p = super::w2p(cx, b.center_pos);
    b.use_stage_eff = true;
    cx.out(Out::CamMaxRange(CAM_MAX_RANGE));
    cx.out(Out::Blur { colour: BLUR });
    let mut x = Box::new(x);
    // SetPatternTbl(boss04NormalActTbl), m_patMode 0, the first pattern.
    b.pat_tbl = Tbl::Normal;
    b.pat_index = 0;
    x.pat_mode = 0;
    start_table(&mut b, &mut x, cx, Tbl::Normal);
    b.class = Class::Fidchell(x);
    b
}

/// A table's start as the constructor, `Affect` and `OnThinkDrain` make it:
/// a prediction reserved (and not drained) is cast at once (pattern 15);
/// else the reserve goes and the table runs from its start.
fn start_table(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx, t: Tbl) {
    if b.epitaph == 0 && x.reserve_pred != 0 {
        exec_pattern(b, x, cx, 15, 0);
        return;
    }
    x.reserve_pred = 0;
    let tbl = cx.data.fidchell.words(t).to_vec();
    b.pat_index = exec_pattern_index(b, x, cx, &tbl, 0);
}

// --- the frame --------------------------------------------------------------------------------

/// `ccThBossEffect`'s pass and `ccBoss::Main` (OUT gcmn 0x0046d490) under
/// Fidchell's `Think`.
pub(super) fn main(b: &mut Boss, cx: &mut Cx) {
    let mut x = fidchell_of(b);
    manager_pass(b, &mut x, cx);
    frame(b, &mut x, cx);
    b.class = Class::Fidchell(x);
}

/// `ccBoss::Main` (OUT gcmn 0x0046d490): Outbreak's, `ccGetDist` its
/// `sqrt.s`; `Action` and `Slave` are the base's (nothing).
fn frame(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    if b.exit != 0 {
        return;
    }
    let me = cx.me;
    crate::chara::calc_real(cx.t, &mut cx.scene.chars[me], 0, cx.env, &mut cx.ev);
    calc_target_info(b, cx);
    b.center_pos_p = super::w2p(cx, b.center_pos);
    let pp = cx.scene.chars[me].pos_p;
    b.center_dist = dist(cx, pp, b.center_pos_p);
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
        // ccBoss::PreDrawAnm (0x0046e6c0).
        b.anm_status = i8::from(b.anm.forward());
    }
    cx.scene.chars[me].cond[cond::HOLD] = 0;
}

/// `ccBossEffManager::Draw`'s pass: each enabled effect's `Draw` in slot
/// order, a disabled one freed. A thunder storm's `Draw` makes a magic
/// square, which the same pass then reaches if its slot comes later.
fn manager_pass(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    let mut k = 0;
    while k < b.effects.slots.len() {
        let Some(e) = b.effects.slots[k].as_mut() else {
            k += 1;
            continue;
        };
        if !e.enabled {
            b.effects.slots[k] = None;
            k += 1;
            continue;
        }
        let Some(mut fx) = e.fidchell.take() else {
            e.draw();
            k += 1;
            continue;
        };
        let alive = match fx.as_mut() {
            // m_syncPos: the boss's m_predInfo.camView.
            Fx::Meteo(m) => {
                let (alive, sync) = m.draw(cx);
                if let Some(v) = sync {
                    x.pred.cam_view = v;
                }
                alive
            }
            Fx::Storm(s) => {
                let (alive, square) = s.draw(cx);
                if let Some(pos) = square {
                    b.effect_at(cx, EffKind::MagicSquare { n: 1 }, pos, VF0);
                }
                alive
            }
            Fx::Tower(t) => t.draw(cx),
        };
        if let Some(e) = b.effects.slots[k].as_mut() {
            e.fidchell = Some(fx);
            if !alive {
                e.enabled = false;
            }
        }
        k += 1;
    }
}

/// An effect of Fidchell's own made in the first free slot.
fn make_fx(b: &mut Boss, cx: &mut Cx, kind: EffKind, fx: Fx, pos: V4) -> i32 {
    let id = b.effects.create(kind);
    if let Some(e) = usize::try_from(id).ok().and_then(|k| b.effects.slots.get_mut(k)).and_then(|s| s.as_mut()) {
        e.fidchell = Some(Box::new(fx));
    }
    cx.out(Out::Effect { id, kind, pos, dirc: VF0 });
    id
}

// --- the patterns --------------------------------------------------------------------------------

/// `ccBoss::ChangeNextPattern` (OUT gcmn 0x0046e490).
fn change_next_pattern(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    let tbl = cx.data.fidchell.words(b.pat_tbl).to_vec();
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

/// `ccBoss::ExecPattern(pat, p0, 0, 0)` (OUT gcmn 0x0046e420).
fn exec_pattern(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx, pat: i32, p0: i32) {
    let tbl = [pat, p0, 0, 0, -1];
    exec_pattern_index(b, x, cx, &tbl, 0);
}

/// `ccBoss04::ExecPatternIndex(tbl, i)` (OUT gcmn 0x004a40b0), falling back
/// on `ccBoss::ExecPatternIndex` (0x0046dd20): the next index.
fn exec_pattern_index(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
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
    match pat {
        18 => b.change_action(cx, act::MAGIC, 3, true),
        17 => b.change_action(cx, act::BACK_DASH, 3, true),
        16 => b.change_action(cx, act::DRAIN_ATK, 3, true),
        1 => {
            let n = if b.epitaph != 0 { act::EPITAPH_WAVE } else { act::WAVE };
            b.change_action(cx, n, 3, true);
        }
        9..=11 => {
            if pat != 11 {
                let ty = word(tbl, s);
                s += 1;
                if ty < 0 {
                    // A living member's pick: those on the lists and down
                    // (`dead` set), one by ccRand (the first draw dropped).
                    let _ = cx.cc.rand();
                    let mut down = Vec::new();
                    for m in cx.party.members.into_iter().flatten() {
                        if cx.valid(Some(m)) && cx.dead(m) {
                            down.push(m);
                        }
                    }
                    if down.is_empty() {
                        exec_pattern(b, x, cx, 2, -1);
                        return s;
                    }
                    let r = cx.cc.rand().unsigned_abs() as usize;
                    cx.scene.chars[me].target_char = Some(down[r % down.len()]);
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
            calc_target_info(b, cx);
            if pat == 10 {
                // @1272 read, a skill drawn and dropped: OnThinkSkill draws
                // its own.
                let _ = (cx.cc.rand() % 3).abs();
                b.change_action(cx, act::SKILL, 3, true);
                return s;
            }
            let sid = word(tbl, s);
            s += 1;
            let on = if pat == 9 { cx.scene.chars[me].target_char } else { Some(me) };
            if let Some(tp) = on
                && let Some(k) = item::item_skill_request(cx.t, cx.scene, me, tp, sid, 1, false, cx.rand)
            {
                cx.out(Out::Skill(tp, k));
            }
            exec_pattern(b, x, cx, 2, -2);
        }
        15 => {
            b.change_action(cx, act::EXEC_PREDICTION, 3, true);
            x.reserve_pred = 0;
        }
        14 => {
            x.reserve_pred = 1;
            b.change_action(cx, act::PREDICTION, 3, true);
        }
        _ => return base_exec_pattern_index(b, x, cx, tbl, orig),
    }
    s
}

/// `ccBoss::ExecPatternIndex` (OUT gcmn 0x0046dd20).
fn base_exec_pattern_index(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
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
                calc_target_info(b, cx);
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

/// `ccBoss04::Think` (OUT gcmn 0x004a3f00).
fn think(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    match b.act_num {
        act::NEUTRAL => on_neutral(b, x, cx),
        act::DMG0 | act::DMG1 => on_dmg(b, cx),
        act::PREDICTION => on_prediction(b, x, cx),
        act::WAVE => on_wave(b, x, cx, false),
        act::EXEC_PREDICTION => on_exec_prediction(b, x, cx),
        act::DRAIN_ATK => on_drain_atk(b, x, cx),
        act::DASH_CENTER => on_dash_center(b, x, cx),
        act::DATA_DRAIN => on_drain(b, x, cx),
        act::EPITAPH => on_epitaph(b, x, cx),
        act::EPITAPH_WAVE => on_wave(b, x, cx, true),
        act::DEAD => on_dead(b, x, cx),
        act::ESCAPE => on_escape(b, x, cx),
        act::CHASE => on_chase(b, x, cx),
        act::WANDER => change_next_pattern(b, x, cx),
        act::TRANSFER => on_transfer(b, x, cx),
        act::BACK_DASH => on_back_dash(b, x, cx),
        act::JUMP => {}
        act::SKILL => on_skill(b, x, cx),
        act::MAGIC => on_magic(b, x, cx),
        _ => b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// `OnThinkNeutral` (0x004a46b0) and `OnThinkEpitaph` (0x004a8140, which
/// also counts): the wait, then the next pattern at the nearest.
fn on_neutral(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    b.move_spd = 0;
    b.move_vector = VF0;
    b.set_dirc(b.target_dirc);
    if !b.wait_over(cx) {
        return;
    }
    let t = select(b, cx, 0);
    if !cx.valid(t) {
        return;
    }
    calc_target_info(b, cx);
    change_next_pattern(b, x, cx);
}

fn on_epitaph(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    b.act_count += 1;
    on_neutral(b, x, cx);
}

/// `OnThinkDmg` (0x004a4810): back to the neutral at once.
fn on_dmg(b: &mut Boss, cx: &mut Cx) {
    let n = if b.epitaph != 0 { act::EPITAPH } else { act::NEUTRAL };
    b.change_action(cx, n, 1, true);
    b.spd_down(0, 0x40a0_0000);
}

/// `OnThinkWave` (0x004a48e0) and `OnThinkEpitaphWave` (0x004a82a0, its
/// frames earlier): the wave, boss skill 13 by the clip's frame.
fn on_wave(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx, drained: bool) {
    match b.act_proccess {
        0 => {
            x.anm_w.set(super::ANM_WAVE, cx.clips);
            x.anm_w.speed = 512;
            b.act_proccess += 1;
        }
        1 => {
            let (se2, from, skill) = if drained { (35, 40, 55) } else { (42, 44, 59) };
            let shock = if drained { 40 } else { 45 };
            let f = b.anm.frame();
            if f == 25 {
                cx.se3d(223);
            } else if f == se2 {
                cx.se3d(224);
            }
            if (f as u32) < from as u32 {
                return;
            }
            if f == shock {
                b.effect(cx, EffKind::WaveShock);
                cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
            } else if f == skill {
                let pos = cx.pos();
                b.skill_damage(cx, None, Some(pos), 13);
            }
            if cx.cam.shake && cx.boss_cam {
                cx.quake_vec([0x4170_0000; 3]);
            }
            let done = x.anm_w.forward();
            cx.out(Out::DrawWave);
            if b.anm_status != 0 && done {
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkDrain` (0x004a4b70): drained; the Epitaph's table, act 12.
fn on_drain(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    b.cheat_hp = 0;
    b.pat_tbl = Tbl::Epitaph;
    b.pat_index = 0;
    x.pat_mode = 2;
    start_table(b, x, cx, Tbl::Epitaph);
    b.change_action(cx, act::EPITAPH, 3, true);
    b.move_spd = 0;
}

/// The prediction's cinema by `m_predId`, and the spell's.
const PRED_CINEMA: [i32; 4] = [64, 65, 67, 66];
const SPELL_CINEMA: [i32; 4] = [14, 15, 17, 16];

/// `OnThinkPrediction` (0x004a4cb0): camera 3 before Fidchell, one of four
/// spells picked by the frame count (`m_predId`), its text shown and its
/// voice heard, then its skill laid on each member; the spell comes with
/// pattern 15.
fn on_prediction(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 if c == 0 => {
            let t = select(b, cx, 0);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            calc_target_info(b, cx);
            b.lock_player(cx, false);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            let pos = cx.pos();
            let view = [pos[0], pos[1], 0x4396_0000, pos[3]];
            let eye = off_from(cx, pos, &behind(b.target_dirc), [0x44bb_8000, 0, 0x4248_0000, ONE]);
            cx.out(Out::CameraChange(3));
            set_cam3(x, cx, eye, view);
            x.pred_id = (cx.env.count & 3) as i32;
            cx.out(Out::Cinema(Some(PRED_CINEMA[x.pred_id as usize])));
        }
        0 if c >= 45 => {
            cx.out(Out::SwitchLayer);
            x.stage_eff_id = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 30000]);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 if c >= 15 => {
            cx.out(Out::Fidchell(Pic::VoiceStop));
            b.act_proccess += 1;
        }
        2 => {
            let k = x.pred_id as usize;
            if let Some(Some(clip)) = cx.data.fidchell.pred_texts.get(k).cloned() {
                x.pred_txt.set(&clip, cx.clips);
            }
            // cameraGetPos(camID), cameraGetRot(camID), each w 1, through
            // W2P; the text 1000 ahead of the eye toward the turn.
            let mut p = x.cam3.0;
            p[3] = ONE;
            let mut r = cx.cam.rot;
            r[3] = ONE;
            let (p, r) = (super::w2p(cx, p), super::w2p(cx, r));
            let d = get_dirc(p, r);
            let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), d), NEG_HALF_PI);
            let v = geom::apply_matrix(&m, [0x447a_0000, 0, 0, ONE]);
            x.pred_pos = super::p2w(cx, geom::vadd(p, v));
            let voice = cx.data.fidchell.pred_voices[k];
            cx.out(Out::Fidchell(Pic::Voice(voice)));
            b.act_proccess += 1;
        }
        3 => {
            x.pred_dirc = cx.cam.rot;
            let done = x.pred_txt.forward();
            let clip = x.pred_txt.clip.clone().unwrap_or_default();
            cx.out(Out::Fidchell(Pic::Text { clip, time: x.pred_txt.posed, pos: x.pred_pos, dirc: x.pred_dirc }));
            if !done {
                return;
            }
            let t = b.select_target(cx, 0, FAR);
            if target_ok(cx, t) {
                cx.scene.chars[me].target_char = t;
                calc_target_info(b, cx);
                let mut vp = cx.scene.chars[t.unwrap()].pos;
                vp[2] = ee::add(vp[2], 0x4348_0000);
                let d = [0, 0, b.target_dirc, ONE];
                let off = [0x44bb_8000, 0, 0x42c8_0000, ONE];
                on_magic_camera_at(b, x, cx, vp, d, off, 3);
            }
            b.act_count = 0;
            b.act_proccess += 1;
        }
        4 if c == 0 => {
            let sid = cx.data.fidchell.pred_skills[x.pred_id as usize & 3];
            for m in cx.party.members.into_iter().flatten() {
                if target_ok(cx, Some(m))
                    && let Some(k) = item::item_skill_compel(cx.t, cx.scene, me, m, sid, 0, false, cx.rand)
                {
                    cx.out(Out::Skill(m, k));
                }
            }
        }
        4 if c == 60 => {
            b.act_count = 0;
            b.act_proccess += 1;
        }
        5 => {
            let id = std::mem::replace(&mut x.stage_eff_id, -1);
            b.end_stage_effect(cx, id, 0x6400_0000, 15);
            b.act_proccess += 1;
            b.act_count = 0;
        }
        6 => {
            // The count again: its value after the first step is checked.
            let c2 = b.act_count;
            b.act_count += 1;
            if c2 == 15 {
                cx.out(Out::SwitchLayer);
                b.unlock_player();
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                cx.out(Out::CameraChange(2));
                off_magic_camera(b, cx, 0);
                cinema_off(cx);
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkDrainAtk` (0x004a5550): a member drained through menu 74
/// (stream 68).
fn on_drain_atk(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
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
            calc_target_info(b, cx);
            b.lock_player(cx, false);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            cx.out(Out::Cinema(Some(-1)));
            b.act_proccess += 1;
            b.act_count = 0;
        }
        1 if c == 45 => {
            let id = cx.scene.chars[me].target_char.map_or(-1, |t| i32::from(cx.scene.chars[t].id()));
            let slot = cx.party.slot_of(id);
            cx.out(Out::StreamMenu { stream: 68, mask: 1 << slot });
            b.act_proccess += 1;
        }
        2 if cx.env.menu_type == -1 => {
            let t = cx.scene.chars[me].target_char;
            if let Some(tp) = t
                && target_ok(cx, t)
            {
                cx.affect(tp, 13, 0);
            }
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cinema_off(cx);
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkExecPrediction` (0x004a57e0): the predicted spell. The camera
/// 1800 off the centre, `ANM_ex41act1` at 30, then 2000 behind the target
/// (the boss camera's free mode for the quake), the spell until it ends,
/// and a 15-frame fade back in.
fn on_exec_prediction(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 if c == 0 => {
            let t = select(b, cx, 0);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            calc_target_info(b, cx);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.lock_player(cx, true);
            if let Some(&n) = SPELL_CINEMA.get(x.pred_id as usize) {
                cx.out(Out::Cinema(Some(n)));
            }
            let pos = cx.pos();
            let mut view = pos;
            view[2] = ee::add(view[2], 0x4396_0000);
            let eye = off_from(cx, pos, &behind(b.center_dirc), [0x44e1_0000, 0, 0x4248_0000, ONE]);
            spell_camera(x, cx, eye, view);
            b.dirc[2] = b.center_dirc;
        }
        0 if c == 30 => {
            b.anm.set("ANM_ex41act1", cx.clips);
            x.act_sub_count = 0;
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 if b.anm_status != 0 => {
            b.dirc[2] = b.target_dirc;
            b.act_proccess += 1;
            if x.pred_id != 1 {
                cx.out(Out::SwitchLayer);
                x.stage_eff_id = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 0x7fff]);
            }
            x.act_sub_proccess = 0;
            x.act_sub_count = 0;
            x.pred = PredInfo::default();
            let t = cx.scene.chars[me].target_char.unwrap_or(me);
            let mut view = cx.scene.chars[t].pos;
            view[2] = ee::add(view[2], 0x4348_0000);
            let eye = off_from(cx, b.target_pos, &behind(b.target_dirc), [0x44fa_0000, 0, 0x4248_0000, ONE]);
            spell_camera(x, cx, eye, view);
        }
        1 => {
            let s = x.act_sub_count;
            x.act_sub_count += 1;
            match s {
                0 => cx.out(Out::Se { se: 233 }),
                45 => cx.out(Out::Se { se: 196 }),
                70 => cx.out(Out::SeNote { se: 196, note: 65 }),
                _ => {}
            }
        }
        2 => {
            let done = match x.pred_id {
                0 => on_meteo_sworm(b, x, cx),
                1 => on_ice_break(b, x, cx),
                2 => on_thunder_storm(b, x, cx),
                3 => on_ground_quake(b, x, cx),
                _ => false,
            };
            if done {
                b.act_proccess += 1;
                if x.pred_id != 1 && x.stage_eff_id >= 0 {
                    b.end_stage_effect(cx, x.stage_eff_id, 0x6400_0000, 15);
                }
                x.stage_eff_id = -1;
            }
        }
        3 => {
            let mut f = ee::add(x.pred.transparency, 0x3d88_8889);
            if !ee::lt(f, ONE) {
                f = ONE;
                if x.pred_id == 3 {
                    cx.out(Out::CamMode { mode: 6 });
                } else {
                    cx.out(Out::CameraChange(2));
                }
                if x.pred_id != 1 {
                    cx.out(Out::SwitchLayer);
                }
                b.unlock_player();
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                cinema_off(cx);
                change_next_pattern(b, x, cx);
            }
            b.transparency = f;
            b.set_transparency = f;
            x.pred.transparency = f;
        }
        _ => {}
    }
}

/// The spell's camera: camera 3 at `eye` looking at `view`, or for the
/// quake (`m_predId` 3) the boss camera's free mode there.
fn spell_camera(x: &mut Fidchell, cx: &mut Cx, eye: V4, view: V4) {
    if x.pred_id != 3 {
        cx.out(Out::CameraChange(3));
        set_cam3(x, cx, eye, view);
    } else {
        cx.out(Out::CamMode { mode: 5 });
        cx.out(Out::FreeCam { pos: eye, view });
    }
}

/// `OnMagicCamera(vp, d, offset, camID)` (0x004a7160): the eye `offset`
/// from `vp` turned by `d`; camera `camID` there, or for camera 2 the boss
/// camera's free mode.
#[allow(clippy::too_many_arguments)]
fn on_magic_camera_at(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx, vp: V4, d: V4, off: V4, cam: i32) {
    let _ = b;
    let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
    let m = geom::rot_matrix_x(&m, d[0]);
    let m = geom::rot_matrix_y(&m, d[1]);
    let m = geom::rot_matrix_z(&m, d[2]);
    let eye = off_from(cx, vp, &m, off);
    if cam == 2 {
        if cx.boss_cam {
            cx.out(Out::CamMode { mode: 5 });
            cx.out(Out::FreeCam { pos: eye, view: vp });
            cx.out(Out::CameraPos { cam: 3, pos: eye });
            x.cam3.0 = eye;
        }
    } else {
        cx.out(Out::CameraChange(cam));
        cx.out(Out::CameraPos { cam, pos: eye });
        cx.out(Out::CameraView { cam, view: vp });
        if cam == 3 {
            x.cam3 = (eye, vp);
        }
    }
}

/// `OnMagicCamera(tp)` (0x004a6fc0) for the target: camera 3 1500 off
/// the target toward the centre (`CalcEffectCameraPos`), looking 200 over
/// it; `m_predInfo` anew with that eye and the target's place.
fn on_magic_camera(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    cx.out(Out::CameraChange(3));
    let view = b.target_pos;
    let mut v2 = view;
    v2[2] = ee::add(v2[2], 0x4348_0000);
    let eye = b.calc_effect_camera_pos(cx, view, 0x44bb_8000);
    set_cam3(x, cx, eye, v2);
    x.pred = PredInfo { cam_pos: eye, cam_view: view, ..PredInfo::default() };
}

/// `OffMagicCamera(mode)` (0x004a7300): the boss camera's mode 6, or
/// camera 2 again.
fn off_magic_camera(b: &mut Boss, cx: &mut Cx, mode: i32) {
    let _ = b;
    if mode != 0 {
        if cx.boss_cam {
            cx.out(Out::CamMode { mode: 6 });
        }
    } else {
        cx.out(Out::CameraChange(2));
    }
}

/// `OnMeteoSworm` (0x004a60b0): a magic square, Fidchell hidden, then 50
/// meteors from 1000 behind the target and 3000 up (`ccBossEffMeteoSworm`),
/// the camera's view following the last; boss skill 14 once they land.
fn on_meteo_sworm(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) -> bool {
    if x.act_sub_proccess != 0 {
        let v = x.pred.cam_view;
        cx.out(Out::CameraView { cam: 3, view: v });
        x.cam3.1 = v;
    }
    match x.act_sub_proccess {
        0 => {
            let s = x.act_sub_count;
            x.act_sub_count += 1;
            if s == 0 {
                b.effect(cx, EffKind::MagicSquare { n: 2 });
                b.draw_sw = 0;
            }
            if x.act_sub_count < 30 {
                return false;
            }
            let mut start = b.calc_effect_camera_pos(cx, b.target_pos, 0xc47a_0000);
            let end = b.target_pos;
            start[2] = 0x453b_8000;
            on_magic_camera(b, x, cx);
            let (mut p, mut v) = x.cam3;
            p[2] = ee::add(p[2], 0x42c8_0000);
            v[2] = ee::add(v[2], 0x4348_0000);
            set_cam3(x, cx, p, v);
            let m = MeteoSworm::new(cx, start, end, 50, 0x43fa_0000, 0x4248_0000);
            x.eff_magic = Some(make_fx(b, cx, EffKind::MeteoSworm { n: 50 }, Fx::Meteo(m), start));
            x.act_sub_proccess += 1;
            false
        }
        1 => {
            if b.effects.enabled(x.eff_magic) {
                return false;
            }
            let pos = cx.pos();
            b.skill_damage(cx, None, Some(pos), 14);
            off_magic_camera(b, cx, 0);
            b.draw_sw = 1;
            true
        }
        _ => false,
    }
}

/// `OnIceBreak` (0x004a6330): Skeith's magic: the force generators over
/// each member, the ring, the ice at each member (the screen reversed while
/// it lasts), then boss skill 2 at the target.
fn on_ice_break(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) -> bool {
    let me = cx.me;
    match x.act_sub_proccess {
        0 => {
            if !target_ok(cx, cx.scene.chars[me].target_char) {
                let t = select(b, cx, 0);
                if !cx.valid(t) {
                    change_next_pattern(b, x, cx);
                    return true;
                }
                calc_target_info(b, cx);
            }
            cx.out(Out::SwitchLayer);
            x.stage_eff_id = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 0x7fff]);
            b.effect(cx, EffKind::MagicSquare { n: 0 });
            cx.se3d(225);
            x.act_sub_proccess += 1;
            false
        }
        1 => {
            let s = x.act_sub_count;
            x.act_sub_count += 1;
            if s != 30 {
                return false;
            }
            let rot = [0x3fc9_0fdb, 0, 0, ONE];
            let k = EffKind::ForceGenerator {
                num: 8,
                life: 100,
                speed: 0x4120_0000,
                r0: 0x4348_0000,
                r1: 0x4348_0000,
                clt: 8,
            };
            for m in cx.party.members.into_iter().flatten() {
                if target_ok(cx, Some(m)) {
                    let mut p = cx.scene.chars[m].pos;
                    p[2] = 0x447a_0000;
                    x.eff_magic = Some(b.effect_at(cx, k, p, rot));
                    cx.se3d(226);
                }
            }
            x.act_sub_proccess += 1;
            false
        }
        2 => {
            let s = x.act_sub_count;
            x.act_sub_count += 1;
            if s == 130 {
                cx.se3d(227);
            }
            if b.effects.enabled(x.eff_magic) {
                return false;
            }
            let rot = [0, 0, b.target_dirc, ONE];
            x.eff_magic = Some(b.effect_at(cx, EffKind::AutoSamonRing { n: 35 }, b.target_pos, rot));
            x.act_sub_count = 0;
            x.act_sub_proccess += 1;
            false
        }
        3 => {
            if b.effects.enabled(x.eff_magic) {
                return false;
            }
            let mut none = true;
            for m in cx.party.members.into_iter().flatten() {
                if cx.valid(Some(m)) {
                    let p = cx.scene.chars[m].pos;
                    x.eff_magic = Some(b.effect_at(cx, EffKind::IceBreak, p, VF0));
                    none = false;
                }
            }
            if none {
                let p = b.target_pos;
                x.eff_magic = Some(b.effect_at(cx, EffKind::IceBreak, p, VF0));
            }
            // The reversed layer (priority 1) and its ccBufferReverce.
            x.pred.rev_layer = true;
            x.br = true;
            b.draw_sw = 0;
            cx.out(Out::SwitchLayer);
            let id = std::mem::replace(&mut x.stage_eff_id, -1);
            b.end_stage_effect(cx, id, 0x6400_0000, 15);
            cx.out(Out::Se { se: 228 });
            x.act_sub_proccess += 1;
            false
        }
        4 => {
            x.act_sub_count += 1;
            if b.effects.enabled(x.eff_magic) {
                cx.out(Out::Reverse);
                return false;
            }
            b.draw_sw = 1;
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            b.unlock_player();
            x.eff_magic = None;
            let tp = b.target_pos;
            b.skill_damage(cx, None, Some(tp), super::SKILL_MAGIC);
            // m_br and revLayer deleted, their pointers kept.
            cx.out(Out::Se3dNote { se: 66, pos: tp, note: 52 });
            cx.out(Out::Se3d { se: 65, pos: tp });
            cx.out(Out::Se3d { se: 56, pos: tp });
            true
        }
        _ => false,
    }
}

/// `OnThunderStorm` (0x004a6ae0): a magic square, then at 30 sixteen
/// thunders 200 round the target (`ccBossEffThunderStorm`) with a flash,
/// Fidchell hidden; boss skill 17 once they end.
fn on_thunder_storm(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) -> bool {
    match x.act_sub_proccess {
        0 => {
            let s = x.act_sub_count;
            x.act_sub_count += 1;
            if s == 0 {
                b.effect(cx, EffKind::MagicSquare { n: 1 });
                return false;
            }
            if x.act_sub_count != 30 {
                return false;
            }
            let at = b.target_pos;
            let st = ThunderStorm::new(cx, at, 0x4348_0000, 16);
            x.eff_magic = Some(make_fx(b, cx, EffKind::ThunderStorm { n: 16 }, Fx::Storm(st), at));
            x.act_sub_proccess += 1;
            x.act_sub_count = 0;
            b.draw_sw = 0;
            cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
            false
        }
        1 => {
            if b.effects.enabled(x.eff_magic) {
                return false;
            }
            let pos = cx.pos();
            b.skill_damage(cx, None, Some(pos), 17);
            b.draw_sw = 1;
            off_magic_camera(b, cx, 0);
            true
        }
        _ => false,
    }
}

/// `OnGroundQuake` (0x004a6d20): 128 rock towers round the target
/// (`ccBossEffRockTower`), Fidchell hidden, the free camera 1000 off the
/// centre; a quake each frame, and boss skill 16 once they fall.
fn on_ground_quake(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) -> bool {
    match x.act_sub_proccess {
        0 => {
            let at = b.target_pos;
            let t = RockTower::new(cx, at, 128);
            x.eff_magic = Some(make_fx(b, cx, EffKind::RockTower { n: 128 }, Fx::Tower(t), at));
            b.draw_sw = 0;
            let mut view = b.target_pos;
            view[2] = 0;
            let eye = off_from(cx, b.target_pos, &behind(b.center_dirc), [0x447a_0000, 0, 0x43fa_0000, ONE]);
            cx.out(Out::FreeCam { pos: eye, view });
            x.act_sub_proccess += 1;
            x.act_sub_count = 0;
            false
        }
        1 => {
            cx.quake_vec([0x420c_0000, 0x420c_0000, 0]);
            if b.effects.enabled(x.eff_magic) {
                return false;
            }
            b.draw_sw = 1;
            let pos = cx.pos();
            b.skill_damage(cx, None, Some(pos), 16);
            true
        }
        _ => false,
    }
}

/// `OnThinkBackDash` (0x004a7360): back off the camera's way at 150 for 31
/// frames, turning 60 degrees either side every tenth, off the targets.
fn on_back_dash(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    if c == 0 {
        b.erase_cmnd_target(cx);
        cursor(cx, true);
        b.spd_up(0x4316_0000, 0x4316_0000);
        let pp = cx.scene.chars[me].pos_p;
        if valid_area(b, cx, pp) {
            return;
        }
        b.move_dirc = wrap_lo(ee::add(PI, cx.cam.rot[2]));
        return;
    }
    let pp = cx.scene.chars[me].pos_p;
    if !valid_area(b, cx, pp) {
        b.move_dirc = b.center_dirc;
    }
    if b.act_count % 10 == 0 {
        let r = cx.cam.rot[2];
        b.move_dirc = if b.act_proccess != 0 { ee::add(0x3f86_0a92, r) } else { ee::sub(r, 0x3f86_0a92) };
        b.act_proccess ^= 1;
        b.move_dirc = wrap_lo(b.move_dirc);
        let vel =
            [ee::mul(0x4040_0000, libm::cosf(b.move_dirc)), ee::mul(0x4040_0000, libm::sinf(b.move_dirc)), 0, ONE];
        let pos = cx.pos();
        cx.out(Out::Fidchell(Pic::Smoke { pos, vel, size: 0x4100_0000 }));
    }
    if b.act_count >= 31 {
        b.entry_cmnd_target(cx);
        cursor(cx, false);
        b.move_spd = 0;
        b.move_vector = VF0;
        change_next_pattern(b, x, cx);
    }
}

/// `OnThinkChase` (0x004a76f0): at 100 toward where the target stood,
/// after-images, until within 200 or 151 frames; out of the arena, a dash
/// of 500.
fn on_chase(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            if !target_ok(cx, cx.scene.chars[me].target_char) {
                let t = select(b, cx, 0);
                if !target_ok(cx, t) {
                    change_next_pattern(b, x, cx);
                    return;
                }
            }
            calc_target_info(b, cx);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            cx.out(Out::SeNote { se: 57, note: 55 });
            b.move_dirc = b.target_dirc;
            b.move_spd = 0x42c8_0000;
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if ee::lt(b.target_dist, 0x4348_0000) || c >= 151 {
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                change_next_pattern(b, x, cx);
                return;
            }
            let pp = cx.scene.chars[me].pos_p;
            if !valid_area(b, cx, pp) {
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                exec_pattern(b, x, cx, 6, 500);
                return;
            }
            if c & 3 == 0 {
                cx.out(Out::AfterImage);
                if c == 0 {
                    cx.out(Out::Se { se: 229 });
                }
            }
        }
        _ => {}
    }
    b.dirc[2] = b.move_dirc;
}

/// `OnThinkEscape` (0x004a79a0): away at 100 (50 from frame 61) for 150
/// frames while the target is within 1500; out of the arena after 30, a
/// dash of 1500. Its heading is whatever `moveDirc` was.
fn on_escape(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            b.move_spd = 0x42c8_0000;
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c >= 150 || !ee::le(b.target_dist, 0x44bb_8000) {
                change_next_pattern(b, x, cx);
                return;
            }
            if b.act_count >= 61 {
                b.move_spd = 0x4248_0000;
            }
            if b.act_count >= 30 {
                let pp = cx.scene.chars[me].pos_p;
                if !valid_area(b, cx, pp) {
                    exec_pattern(b, x, cx, 6, 1500);
                    return;
                }
            }
            if held(b, cx) {
                b.move_spd = 0;
            }
        }
        _ => {}
    }
    b.set_dirc(b.move_dirc);
}

/// `OnThinkDashCenter` (0x004a8530): the base's dash (act 7) at 50 to
/// `dashPos`, 121 frames at most, off the targets.
fn on_dash_center(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
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
                    cx.se3d(229);
                }
            }
            if b.act_count >= 121 {
                b.act_proccess += 1;
                return;
            }
            let p = super::w2p(cx, b.dash_pos);
            let pp = cx.scene.chars[me].pos_p;
            b.move_dirc = get_dirc(pp, p);
            if ee::lt(dist(cx, pp, p), 0x4348_0000) {
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

/// `OnThinkDead` (0x004a8750): off the lists, the dead effect; exit once
/// it and the clip end.
fn on_dead(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            b.move_spd = 0;
            b.move_vector = VF0;
            crate::fellow::delete_cmnd(cx.scene, cx.me);
            cx.out(Out::DeleteCmnd);
            x.eff_dead = Some(b.begin_dead_effect(cx, 0x44fa_0000, 0x44fa_0000));
            b.act_proccess += 1;
        }
        1 => {
            if x.eff_dead.is_some() && !b.effects.enabled(x.eff_dead) {
                x.eff_dead = None;
            }
            if b.anm_status != 0 && x.eff_dead.is_none() {
                b.exit = 1;
            }
        }
        _ => {}
    }
}

/// `OnThinkSkill` (0x004a8860): one of `@2048` on the target under camera
/// 3, its start effect and cinema; the skill 30 frames on, then 65 more.
fn on_skill(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    let me = cx.me;
    b.set_dirc(b.target_dirc);
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 0);
            if !cx.valid(t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            calc_target_info(b, cx);
            let mut vp = b.target_pos;
            vp[2] = 0x4348_0000;
            let d = [0, 0, b.target_dirc, ONE];
            let off = [0x447a_0000, 0, 0, ONE];
            on_magic_camera_at(b, x, cx, vp, d, off, 3);
            let k = (cx.cc.rand() % 3).unsigned_abs() as usize;
            x.skill_id = cx.data.fidchell.skills[k];
            cx.out(Out::SkillStart { who: me, sid: x.skill_id });
            x.eff_start = Some(eff::SKILL_START_LIFE);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.lock_player(cx, true);
            cx.out(Out::CinemaSkill(x.skill_id));
            b.act_proccess += 1;
        }
        1 => {
            // The start's controller: its endFlag once its life is out.
            if let Some(n) = x.eff_start {
                x.eff_start = (n > 1).then_some(n - 1);
            }
            let c = b.act_count;
            b.act_count += 1;
            if c < 30 || x.eff_start.is_some() {
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
        2 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 50 {
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        3 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 15 {
                b.unlock_player();
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                cx.out(Out::CameraChange(2));
                cinema_off(cx);
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkMagic` (0x004a8c40): one of `@2081` by the frame count on the
/// target under camera 3, the stage darkened; the spell at 60, the stage
/// back when it is over (60, 150 or 90 frames by spell).
fn on_magic(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
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
            calc_target_info(b, cx);
            cx.out(Out::SwitchLayer);
            x.stage_eff_id = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 3000]);
            b.lock_player(cx, true);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            let mut vp = b.target_pos;
            vp[2] = 0x4348_0000;
            let d = [0, 0, b.target_dirc, ONE];
            let off = [0x447a_0000, 0, 0, ONE];
            on_magic_camera_at(b, x, cx, vp, d, off, 3);
            x.skill_id = cx.data.fidchell.magic_skills[(cx.env.count & 3) as usize];
            cx.out(Out::CinemaSkill(x.skill_id));
            b.act_proccess += 1;
        }
        1 if c == 60 => {
            // The flag is a3 as the frame left it (the harness measures it).
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
                259 => 90,
                203 | 219 => 150,
                227 => 60,
                _ => -1,
            };
            if c == wait {
                let id = std::mem::replace(&mut x.stage_eff_id, -1);
                b.end_stage_effect(cx, id, 0x6400_0000, 15);
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        3 if c == 15 => {
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            b.unlock_player();
            cx.out(Out::CameraChange(2));
            cx.out(Out::SwitchLayer);
            cinema_off(cx);
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkTransfer` (0x004a7b40), which no pattern of Fidchell's reaches:
/// fade out spinning, to the centre 1000 up, a force generator, down
/// spinning as it fades in, then the neutral. The after-image it fills
/// every tenth frame is never entered.
fn on_transfer(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    const SIXTIETH: F = 0x3c88_8889;
    const STEP: F = 0x3f20_d97c;
    let me = cx.me;
    match b.act_proccess {
        0 => {
            b.lock_player(cx, false);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            x.transparency = ONE;
            x.rot_z = 0;
            b.act_proccess += 1;
        }
        1 => {
            let mut f = ee::sub(x.transparency, SIXTIETH);
            if ee::lt(f, 0) {
                f = 0;
                b.act_proccess += 1;
            }
            x.transparency = f;
            let mut r = ee::add(x.rot_z, STEP);
            if ee::lt(r, NEG_PI) {
                r = ee::add(r, TWO_PI);
            }
            if !ee::le(r, PI) {
                r = ee::sub(r, TWO_PI);
            }
            x.rot_z = r;
            b.move_vector[1] = 0x4185_5555;
            let c = b.act_count;
            b.act_count += 1;
            if c == 10 {
                b.act_count = 0;
            }
        }
        2 => {
            let mut pp = geom::vadd(b.center_pos_p, VF0);
            pp[2] = 0x447a_0000;
            cx.scene.chars[me].pos_p = pp;
            cx.scene.chars[me].pos = super::p2w(cx, pp);
            b.act_proccess += 1;
        }
        3 => {
            let rot = [0x3fc9_0fdb, 0, 0, 0xbf80_0000];
            let k = EffKind::ForceGenerator {
                num: 16,
                life: 120,
                speed: 0x4185_5555,
                r0: 0x447a_0000,
                r1: 0x447a_0000,
                clt: 5,
            };
            let pos = cx.pos();
            b.effect_at(cx, k, pos, rot);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        4 => {
            let mut f = ee::add(x.transparency, SIXTIETH);
            if !ee::le(f, ONE) {
                f = ONE;
            }
            x.transparency = f;
            let mut r = ee::add(x.rot_z, STEP);
            if !ee::le(r, PI) {
                r = ee::sub(r, TWO_PI);
            }
            // The second test is against pi, not -pi: nearly every turn
            // gains a whole turn.
            if ee::lt(r, PI) {
                r = ee::add(r, TWO_PI);
            }
            x.rot_z = r;
            b.move_vector[2] = 0xc185_5555;
            let next = geom::vadd(cx.scene.chars[me].pos_p, b.move_vector);
            if ee::lt(next[2], 0) {
                b.move_vector = VF0;
                cx.scene.chars[me].pos_p[2] = 0;
                b.act_proccess += 1;
            }
            let c = b.act_count;
            b.act_count += 1;
            if c == 10 {
                b.act_count = 0;
            }
        }
        5 => {
            let pp = cx.scene.chars[me].pos_p;
            cx.scene.chars[me].pos = super::p2w(cx, pp);
            x.transparency = ONE;
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            b.change_action(cx, act::NEUTRAL, 3, true);
        }
        _ => {}
    }
    b.transparency = x.transparency;
    b.set_transparency = x.transparency;
    b.dirc[2] = x.rot_z;
}

// --- Affect ----------------------------------------------------------------------------------

/// `ccBoss04::Affect` (OUT gcmn 0x004a3d00): `ccBoss::Affect`, then the
/// drain's 5000 HP, and the Super table once a hit leaves the gauge half
/// full.
pub(super) fn affect(b: &mut Boss, cx: &mut Cx) {
    let mut x = fidchell_of(b);
    affect_of(b, &mut x, cx);
    b.class = Class::Fidchell(x);
}

fn affect_of(b: &mut Boss, x: &mut Fidchell, cx: &mut Cx) {
    let me = cx.me;
    let t = cx.scene.chars[me].affect.ty;
    if t != 21 && (b.act_forbid == 2 || b.lock_player != 0) {
        return;
    }
    b.base_affect(cx);
    let t = cx.scene.chars[me].affect.ty;
    if t == 21 {
        let ch = &mut cx.scene.chars[me];
        ch.hp = EPITAPH_HP;
        ch.max_hp = EPITAPH_HP;
    } else if (t == 1 || t == 3) && x.pat_mode == 0 {
        let pp = cx.scene.chars[me].foe_state().map_or(0, |f| f.pp);
        let id = usize::try_from(cx.scene.chars[me].id()).unwrap_or(0);
        let max = cx.t.bosses.get(id).map_or(0, |r| r.max_pp);
        if pp >= max >> 1 {
            b.pat_tbl = Tbl::Super;
            b.pat_index = 0;
            x.pat_mode = 1;
            start_table(b, x, cx, Tbl::Super);
        }
    }
}
