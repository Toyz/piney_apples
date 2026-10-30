//! Tarvos (`ccBoss07`, boss07.cpp), Quarantine's second phase boss:
//! `bossTbl` row 6, `bossFunc` code 6, fought in event 310 in field 7.
//! Ported from Quarantine's gcmn.prg (0x003a81e0-0x003ac9e8; names and
//! layouts from Infection's DWARF, whose gcmn carries the class unused),
//! with the `ccBoss` base Outbreak's and Quarantine's share. Its guards
//! (rows 42 and 43, physical and magic), the transfer between them, the
//! spike rain, the phantom, the sacrifice. docs/engine/boss-tarvos.md.

pub mod eff;
#[cfg(test)]
mod tests;

use piney_data::field::ee;
use piney_data::libm;
use piney_data::volume::Volume;

use self::eff::Fx;
use super::{Anm, Boss, Class, Cx, EffKind, Out, Tbl, VF0};
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
/// The stage fader's colour (`BeginStageEffect(0x64000000, ...)`).
const DARK: u32 = 0x6400_0000;

/// `bossFunc`'s code for Tarvos, its `bossTbl` row and its file.
pub const CODE: i32 = 6;
pub const ROW: usize = 6;
pub const FILE: &str = "x71";
/// The guards' rows (`SetDefMode` 0 and 1): physical (exdefense 1), magic
/// (exdefense 2).
pub const GUARD_ROWS: [usize; 2] = [42, 43];
/// `InitBossCamera(200, 1000)`.
pub const CAM_Z: F = 0x4348_0000;
pub const CAM_Y: F = 0x447a_0000;
/// The drain's HP (`OnThinkDrain`).
pub const EPITAPH_HP: i16 = 7000;
/// The transfer's way back in (`ANM_ex71out0`, in [`FILE`]).
pub const OUT_CLIP: &str = "ANM_ex71out0";
/// The fog the transfer blends in (`m_blendColor`).
pub const BLEND_COLOR: u32 = 0x0040_4080;

/// Tarvos's acts (`actNum`, `Think`'s switch at 0x003a9750, table `@1372`;
/// any other is `ChangeAction(0, 0, 4)`).
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DAMAGE0: i16 = 1;
    pub const DAMAGE1: i16 = 2;
    pub const SPIKE_SHOT: i16 = 3;
    pub const WAVE: i16 = 4;
    pub const SACRIFICE: i16 = 5;
    pub const PHANTOM: i16 = 6;
    pub const DASH_CENTER: i16 = 7;
    pub const DRAIN: i16 = 11;
    pub const EPITAPH: i16 = 12;
    pub const EPITAPH_WAVE: i16 = 13;
    pub const DEAD: i16 = 14;
    /// The wander's and return's end: not in `Think`'s table, so the next
    /// frame's `ChangeAction(0, 0, 4)`.
    pub const IDLE: i16 = 15;
    pub const ESCAPE: i16 = 16;
    pub const RETURN: i16 = 17;
    pub const CHASE: i16 = 18;
    pub const WANDER: i16 = 20;
    pub const TRANSFER: i16 = 21;
    pub const SKILL: i16 = 22;
    pub const MAGIC: i16 = 23;
    pub const DATA_DRAIN_ATK: i16 = 25;
    pub const POISON: i16 = 26;
}

/// Tarvos's tables (`tables::combat`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TarvosData {
    /// `boss07NormalActTbl`, `boss07SuperActTbl`, `boss07EpitaphActTbl`
    /// (`@1201`'s three), each through its closing -1.
    pub normal: Vec<i32>,
    pub super_: Vec<i32>,
    pub epitaph: Vec<i32>,
    /// `boss0xAnmTbl`: the clip by act.
    pub anims: Vec<Option<String>>,
    /// `@1178`: pattern 10's draw (dropped); `@1517`: the skill act's.
    pub rand_skills: [i32; 3],
    pub skills: [i32; 3],
    /// `@1568`'s second word: the magic act's spell.
    pub magic_skill: i32,
}

impl TarvosData {
    /// The volume's.
    pub fn of(volume: Volume) -> TarvosData {
        let t = piney_data::tables::combat::of(volume);
        let s = |v: &[Option<&str>]| v.iter().map(|a| a.map(str::to_string)).collect::<Vec<_>>();
        let pick = |v: &[i32], k: usize| v.get(k).copied().unwrap_or(0);
        TarvosData {
            normal: t.tarvos_normal.to_vec(),
            super_: t.tarvos_super.to_vec(),
            epitaph: t.tarvos_epitaph.to_vec(),
            anims: s(t.tarvos_anims),
            rand_skills: std::array::from_fn(|k| pick(t.tarvos_rand_skills, k)),
            skills: std::array::from_fn(|k| pick(t.tarvos_skills, k)),
            magic_skill: pick(t.tarvos_magic_skills, 1),
        }
    }

    /// The table by [`Tbl`] (`@1201`: mode 0 normal, 1 super, 2 epitaph).
    fn words(&self, t: Tbl) -> &[i32] {
        match t {
            Tbl::None => &[],
            Tbl::Normal => &self.normal,
            Tbl::Super => &self.super_,
            Tbl::Epitaph => &self.epitaph,
        }
    }
}

/// What Tarvos shows beside its body, and what its effects draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pic {
    /// `anmw` (`ANM_xx11wave`) posed at `time` at `pos` turned by `dirc`:
    /// the wave's by `ccAnm::Draw` on layer 3, the Epitaph's by
    /// `DrawParts(anmw, 1)`.
    Wave { time: u32, pos: V4, dirc: V4, parts: bool },
    /// `ccDrawEnv::SetFogBlend(rate, colour)` before the body's draw: the
    /// transfer's fog.
    Fog { rate: F, colour: u32 },
    /// `ccBossEffTransEffectCreate(pos)` (0x0037db00): three particle
    /// generators (`g_transEffGen`, two of `MagicSquareGenerator`'s), no
    /// slot.
    TransEffect { pos: V4 },
    /// `ccBossEffEruptionCreate(pos, 102, 100, 10)` (0x0037b7b0): its
    /// particles (`g_eruptionGenerator`) at `pos`.
    Eruption { pos: V4 },
    /// A spike of the rain (`CMP_ex71body2` at a quarter, `Rx(-pi/2)
    /// Rx(pitch) Rz(heading)`, at `pos`, at its transparency).
    Spike { pos: V4, pitch: F, heading: F, transparency: F },
    /// The phantom's sprite (`particle`'s `EFF_x000`, `CLT_x000c7`,
    /// 0x80e0ffff) at `pos`, `size` each way.
    PhantomSprite { pos: V4, size: F },
    /// The phantom made: its fourteen lights' generators (`g_phantomGen`
    /// row 0, the first eight going), each following its light.
    PhantomStart,
    /// The phantom's fourteen lights: where each is and its state (0
    /// circling, 3 waiting to fall, 2 falling, 5 burst). All at 3 at once:
    /// `ChangePtclGenerator` (row 1 on the first seven); at 5 a light's
    /// generator is deleted.
    PhantomLights(Box<[(V4, i32); 14]>),
    /// A light's burst into the ground: `effExplode3(pos, 0, 1, kind)`,
    /// its scale animated (1, 4, 15).
    Burst { pos: V4, kind: i32 },
    /// `effRadiateSomething2(pos, (0, -pi/2, 0), 45, 4, 16)`.
    Radiate { pos: V4 },
    /// `ccBossEffExplode2Create(pos)` (0x0037d760): four particle
    /// generators and `ccEnemyEffDust(pos, 10, 5)`, no slot.
    Explode { pos: V4 },
    /// `effResistantShield(this, magic, -1)` on a blow the guard holds off,
    /// its scale 0.7 and its offset 80 up.
    Shield { magic: i32 },
}

/// `ccBoss07`'s own members (+0x29350 on).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tarvos {
    /// `anmw`.
    pub anm_w: Anm,
    /// `m_effAura`, `m_effSpike`, `m_effPhantom`, `m_effDead`.
    pub eff_aura: Option<i32>,
    pub eff_spike: Option<i32>,
    pub eff_phantom: Option<i32>,
    pub eff_dead: Option<i32>,
    /// `m_effStart`: `effSkillStart`'s controller, the frames before its
    /// `endFlag` is seen.
    pub eff_start: Option<i32>,
    pub skill_id: i32,
    pub stage_eff: i32,
    /// `m_patternMode`, `m_defMode` (0 physical guard, 1 magic guard, 2
    /// drained).
    pub pattern_mode: i32,
    pub def_mode: i32,
    /// `m_blendRate`: the transfer's fog.
    pub blend_rate: F,
}

fn tarvos_of(b: &mut Boss) -> Box<Tarvos> {
    match std::mem::replace(&mut b.class, Class::Plain) {
        Class::Tarvos(x) => x,
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

/// `OnMagicCamera(vp, d, offset, 2)` (0x003a8eb0): with a boss camera,
/// its free eye `offset` from `vp` turned by `Rz(-pi/2) Rx Ry Rz` of `d`,
/// looking at `vp`.
fn magic_camera(cx: &mut Cx, vp: V4, d: V4, off: V4) {
    let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
    let m = geom::rot_matrix_x(&m, d[0]);
    let m = geom::rot_matrix_y(&m, d[1]);
    let m = geom::rot_matrix_z(&m, d[2]);
    let v = geom::apply_matrix(&m, off);
    let eye = super::p2w(cx, geom::vadd(super::w2p(cx, vp), v));
    if cx.boss_cam {
        cx.out(Out::CamMode { mode: 5 });
        cx.out(Out::FreeCam { pos: eye, view: vp });
    }
}

/// The skill's and the magic's camera: the eye 1500 out and 70 up from
/// the target along `targetDirc`, looking at the target 200 up.
fn spell_camera(b: &Boss, cx: &mut Cx) {
    let mut vp = b.target_pos;
    vp[2] = 0x4348_0000;
    let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
    let m = geom::rot_matrix_z(&m, b.target_dirc);
    let v = geom::apply_matrix(&m, [0x44bb_8000, 0, 0x428c_0000, ONE]);
    let eye = super::p2w(cx, geom::vadd(super::w2p(cx, b.target_pos), v));
    cx.out(Out::CamMode { mode: 5 });
    cx.out(Out::FreeCam { pos: eye, view: vp });
}

/// `bossCam->QuakeCam((35, 15, 35))`, the wave's and transfer's.
fn quake(cx: &mut Cx) {
    if cx.boss_cam {
        cx.quake_vec([0x420c_0000, 0x4170_0000, 0x420c_0000]);
    }
}

// --- the constructor ------------------------------------------------------------------------

/// `ccBoss07::ccBoss07` (QUA gcmn 0x003a8300): the boss is
/// `scene.chars[cx.me]` (`bossTbl` row 6); it stands 500 behind Kite.
/// `center` is the arena's `DMY_center01`.
pub fn new(cx: &mut Cx, kite_pos: V4, kite_dirc: V4, center: V4) -> Boss {
    let me = cx.me;
    let mut b = super::kyvia::plain_boss();
    b.anm_tbl = cx.data.tarvos.anims.clone();
    // OffExit, OnDraw, OnBodyHit, OnCheatHP.
    b.exit = 0;
    b.draw_sw = 1;
    b.body_hit_sw = 1;
    b.cheat_hp = 1;
    let mut pos = kite_pos;
    pos[1] = ee::sub(pos[1], 0x43fa_0000);
    cx.scene.chars[me].pos = pos;
    b.dirc = kite_dirc;
    b.change_action(cx, act::NEUTRAL, 0, true);
    crate::fellow::entry_cmnd(cx.scene, me);
    // InitAfterImage(anm, 30), InitStageEffect, InitCenterPos,
    // InitBossEffect(32), InitBossCamera(200, 1000).
    b.use_stage_eff = true;
    b.center_pos = [center[0], center[1], center[2], ONE];
    b.center_pos_p = super::w2p(cx, b.center_pos);
    let mut x = Box::new(Tarvos { anm_w: Anm::new(), stage_eff: -1, ..Tarvos::default() });
    // The blur on (m_exit 0) under ccBoss03BlurParamCB.
    cx.out(Out::Blur(super::blur::Set::Enable));
    cx.out(Out::Blur(super::blur::Set::Func(super::blur::BlurFunc::Boss03)));
    set_pattern_mode(&mut b, &mut x, cx, 0);
    set_def_mode(&mut x, cx, 0);
    b.class = Class::Tarvos(x);
    b
}

/// `SetPatternMode(m)` (0x003a8e00): `@1201[m]` set and run from its start.
fn set_pattern_mode(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx, m: i32) {
    let t = match m {
        0 => Tbl::Normal,
        1 => Tbl::Super,
        _ => Tbl::Epitaph,
    };
    x.pattern_mode = m;
    b.pat_tbl = t;
    b.pat_index = 0;
    let tbl = cx.data.tarvos.words(t).to_vec();
    b.pat_index = exec_pattern_index(b, x, cx, &tbl, 0);
}

/// `SetDefMode(d)` (0x003a9040): 0 and 1 point `param` at a guard's row
/// and copy its stats into the boss's (`real`), nothing else; 2 is
/// `SetBaseParam(row 6)` whole (HP and SP full, the gauge and conditions
/// cleared).
fn set_def_mode(x: &mut Tarvos, cx: &mut Cx, d: i32) {
    let me = cx.me;
    x.def_mode = d;
    match d {
        0 | 1 => {
            let Some(r) = cx.t.bosses.get(GUARD_ROWS[d as usize]).cloned() else { return };
            let elm = r.elm;
            if let Some(f) = cx.scene.chars[me].foe_state_mut() {
                f.row = r;
            }
            if let Some(p) = cx.scene.param_mut(me) {
                p.real = elm;
            }
        }
        2 => super::kyvia::set_base_param(cx, me, ROW),
        _ => {}
    }
}

// --- the frame --------------------------------------------------------------------------------

/// `ccThBossEffect`'s pass and `ccBoss07::Main` (0x003a91e0).
pub(super) fn main(b: &mut Boss, cx: &mut Cx) {
    let mut x = tarvos_of(b);
    manager_pass(b, cx);
    frame(b, &mut x, cx);
    let me = cx.me;
    cx.scene.chars[me].cond[cond::HOLD] = 0;
    if b.lock_player != 0
        && b.epitaph == 0
        && let Some(p) = cx.scene.param_mut(me)
        && p.pp_count > 0
    {
        p.pp_count += 1;
    }
    b.class = Class::Tarvos(x);
}

/// `ccBoss::Main` (QUA gcmn, Outbreak's code) under Tarvos's `Think` and
/// `PreDrawAnm` (0x003a96b0: the body not posed while the transfer's
/// stream plays; the transfer's fog).
fn frame(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
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
        let transfer = b.act_num == act::TRANSFER;
        if !(transfer && b.act_proccess == 2) {
            b.anm_status = i8::from(b.anm.forward());
        }
        if transfer && !ee::le(x.blend_rate, 0) {
            cx.out(Out::Tarvos(Pic::Fog { rate: x.blend_rate, colour: BLEND_COLOR }));
        }
        b.draw(cx);
    }
    cx.scene.chars[me].cond[cond::HOLD] = 0;
}

/// `ccBossEffManager::Draw`'s pass: each enabled effect's `Draw` in slot
/// order, a disabled one freed. The spikes' hits call `_SpikeShotCB`
/// (0x003ab560) as they land; the phantom makes its ring as it draws.
fn manager_pass(b: &mut Boss, cx: &mut Cx) {
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
        let Some(mut fx) = e.tarvos.take() else {
            e.draw();
            k += 1;
            continue;
        };
        let alive = fx.draw(cx, b);
        if let Some(e) = b.effects.slots[k].as_mut() {
            e.tarvos = Some(fx);
            e.enabled = alive;
        }
        k += 1;
    }
}

/// `_SpikeShotCB(ch, pos, index)` (0x003ab560): boss skill 24 on a living
/// target; every eighth spike sounds 171 and erupts (`ccBossEffEruption`,
/// made off: its slot goes at the next pass).
fn spike_hit(b: &mut Boss, cx: &mut Cx, t: usize, pos: V4, index: i32) {
    if !target_ok(cx, Some(t)) {
        return;
    }
    b.skill_damage(cx, Some(t), None, 24);
    if index & 7 == 0 {
        cx.out(Out::Se3d { se: 171, pos });
        let id = b.effects.create(EffKind::Eruption);
        if let Some(e) = usize::try_from(id).ok().and_then(|k| b.effects.slots.get_mut(k)).and_then(|s| s.as_mut()) {
            e.enabled = false;
        }
        cx.out(Out::Effect { id, kind: EffKind::Eruption, pos, dirc: VF0 });
        cx.out(Out::Tarvos(Pic::Eruption { pos }));
    }
}

/// An effect of Tarvos's own made in the first free slot.
fn make_fx(b: &mut Boss, cx: &mut Cx, kind: EffKind, fx: Fx, pos: V4) -> i32 {
    let id = b.effects.create(kind);
    if let Some(e) = usize::try_from(id).ok().and_then(|k| b.effects.slots.get_mut(k)).and_then(|s| s.as_mut()) {
        e.tarvos = Some(Box::new(fx));
    }
    cx.out(Out::Effect { id, kind, pos, dirc: VF0 });
    id
}

// --- the patterns --------------------------------------------------------------------------------

/// `ccBoss::ChangeNextPattern` (QUA gcmn 0x00361e30, Outbreak's).
fn change_next_pattern(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    let tbl = cx.data.tarvos.words(b.pat_tbl).to_vec();
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
fn exec_pattern(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx, pat: i32, p0: i32) {
    let tbl = [pat, p0, 0, 0, -1];
    exec_pattern_index(b, x, cx, &tbl, 0);
}

/// `ccBoss07::ExecPatternIndex(tbl, i)` (0x003a8700), falling back on
/// `ccBoss::ExecPatternIndex`: the next index. As Macha's, every operand
/// of its own patterns (2, 9, 10, 11) is read from `patTbl` at the running
/// index, whatever `tbl` is.
fn exec_pattern_index(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
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
    let own = cx.data.tarvos.words(b.pat_tbl).to_vec();
    match pat {
        19 => b.change_action(cx, act::MAGIC, 3, true),
        21 => b.change_action(cx, act::PHANTOM, 3, true),
        17 => b.change_action(cx, act::TRANSFER, 3, true),
        20 => b.change_action(cx, act::DATA_DRAIN_ATK, 3, true),
        14 => b.change_action(cx, act::SPIKE_SHOT, 3, true),
        15 => b.change_action(cx, act::SACRIFICE, 3, true),
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
                // @1178 read, a skill drawn and dropped: OnThinkSkill casts
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
fn base_exec_pattern_index(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
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

/// `ccBoss07::Think` (0x003a9750).
fn think(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    match b.act_num {
        act::NEUTRAL | act::EPITAPH => on_neutral(b, x, cx),
        act::DAMAGE0 | act::DAMAGE1 => b.change_action(cx, act::NEUTRAL, 1, false),
        act::SPIKE_SHOT => on_spike_shot(b, x, cx),
        act::WAVE => on_wave(b, x, cx),
        act::SACRIFICE => on_sacrifice(b, x, cx),
        act::PHANTOM => on_phantom(b, x, cx),
        act::DASH_CENTER => on_dash_center(b, x, cx),
        act::DRAIN => on_drain(b, x, cx),
        act::EPITAPH_WAVE => on_epitaph_wave(b, x, cx),
        act::DEAD => on_dead(b, x, cx),
        act::ESCAPE => on_escape(b, x, cx),
        act::RETURN => on_return(b, cx),
        act::CHASE => on_chase(b, x, cx),
        act::WANDER => on_wander(b, cx),
        act::TRANSFER => on_transfer(b, x, cx),
        act::SKILL => on_skill(b, x, cx),
        act::MAGIC => on_magic(b, x, cx),
        act::DATA_DRAIN_ATK => on_data_drain_atk(b, x, cx),
        // OnThinkPoison (0x003ac9e0) returns at once.
        act::POISON => {}
        _ => b.change_action(cx, act::NEUTRAL, 0, true),
    }
}

/// `OnThinkNeutral` (0x003a9920) and `OnThinkEpitaph` (0x003ac5f0), the
/// same code: turned to the target, the wait, then the next pattern at
/// the nearest.
fn on_neutral(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    b.set_dirc(b.target_dirc);
    b.move_spd = 0;
    b.move_vector = VF0;
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

/// `OnThinkDrain` (0x003a9a80): drained. The Epitaph (act 12), the guard
/// dropped (`SetDefMode(2)`: row 6 whole), 7000 HP, the Epitaph's table,
/// the aura off.
fn on_drain(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    let me = cx.me;
    b.cheat_hp = 0;
    b.change_action(cx, act::EPITAPH, 3, true);
    set_def_mode(x, cx, 2);
    cx.scene.chars[me].hp = EPITAPH_HP;
    cx.scene.chars[me].max_hp = EPITAPH_HP;
    set_pattern_mode(b, x, cx, 2);
    b.move_spd = 0;
    if let Some(id) = x.eff_aura.take()
        && let Some(e) = usize::try_from(id).ok().and_then(|k| b.effects.slots.get_mut(k)).and_then(|s| s.as_mut())
    {
        e.enabled = false;
    }
}

/// `OnThinkDead` (0x003a9bc0): off the lists, the dead effect; exit once
/// it ends.
fn on_dead(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    b.move_spd = 0;
    b.move_vector = VF0;
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

/// `OnThinkSacrifice` (0x003a9cb0): a member (`SelectTarget(2)`, a leftover
/// register's) under the magic camera and a dark stage; at 30 frames
/// stream 103 on it; once the menu closes the stage back and 9999 on it
/// (`EntryAffect(this, 1, 9999)`); 60 frames on, the neutral.
fn on_sacrifice(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 2);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.calc_target_info(cx);
            b.lock_player(cx, true);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            cx.out(Out::Cinema(Some(-1)));
            let mut vp = t.map_or(VF0, |t| cx.scene.chars[t].pos);
            vp[2] = 0x4248_0000;
            b.dirc[2] = b.target_dirc;
            magic_camera(cx, vp, b.dirc, [0x447a_0000, 0, 0x4348_0000, ONE]);
            cx.out(Out::SwitchLayer);
            x.stage_eff = b.begin_stage_effect(cx, DARK, [30, 15, 30000]);
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 30 {
                let id = cx.scene.chars[me].target_char.map_or(-1, |t| i32::from(cx.scene.chars[t].id()));
                let slot = cx.party.slot_of(id);
                cx.out(Out::StreamMenu { stream: 103, mask: 1 << slot });
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        2 if cx.env.menu_type == -1 => {
            let id = std::mem::replace(&mut x.stage_eff, -1);
            b.end_stage_effect(cx, id, DARK, 30);
            let t = cx.scene.chars[me].target_char;
            if let Some(tp) = t
                && target_ok(cx, t)
            {
                cx.affect(tp, 1, 9999);
            }
            b.act_proccess += 1;
        }
        3 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 60 {
                cx.out(Out::SwitchLayer);
                if cx.boss_cam {
                    cx.out(Out::CamMode { mode: 6 });
                }
                cx.out(Out::Cinema(None));
                b.unlock_player();
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkWave` (0x003aa060): `anmw` on `ANM_xx11wave` at 512; SE 223 and
/// 224 at the body's frames 6 and 35; at 40 a WaveShock; boss skill 23
/// fifteen frames on; the wave drawn and the camera shaken every frame
/// from 40, the next pattern as the body's clip ends.
fn on_wave(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            x.anm_w.set(super::ANM_WAVE, cx.clips);
            x.anm_w.speed = 512;
            b.act_proccess += 1;
        }
        1 => {
            let f = b.anm.frame();
            if f == 6 {
                cx.se3d(223);
            } else if f == 35 {
                cx.se3d(224);
            }
            if (f as u32) < 40 {
                return;
            }
            if f == 40 {
                let (pos, dirc) = (cx.pos(), b.dirc);
                b.effect_at(cx, EffKind::WaveShock, pos, dirc);
            }
            let c = b.act_count;
            b.act_count += 1;
            if c == 15 {
                let pos = cx.pos();
                b.skill_damage(cx, None, Some(pos), 23);
            }
            quake(cx);
            x.anm_w.forward();
            let pos = cx.pos();
            cx.out(Out::Tarvos(Pic::Wave { time: x.anm_w.posed, pos, dirc: b.dirc, parts: false }));
            if b.anm_status != 0 {
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkSkill` (0x003aa2a0): one of `@1517` (by `ccRand`) on the target
/// under the free eye, the stage dark; SE 245 at 30, the skill's start at
/// 60, the skill once the start ends; the stage back 50 frames on, the
/// neutral 15 after.
fn on_skill(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    b.set_dirc(b.target_dirc);
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 0);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.calc_target_info(cx);
            cx.out(Out::SwitchLayer);
            x.stage_eff = b.begin_stage_effect(cx, DARK, [15, 15, 30000]);
            spell_camera(b, cx);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.lock_player(cx, true);
            x.skill_id = cx.data.tarvos.skills[(cx.cc.rand() % 3).unsigned_abs() as usize];
            cx.out(Out::CinemaSkill(x.skill_id));
            x.eff_start = None;
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 => {
            if c == 30 {
                cx.out(Out::Se { se: 245 });
            } else if c == 60 {
                // Its endFlag clear the frame it is made.
                cx.out(Out::SkillStart { who: me, sid: x.skill_id });
                x.eff_start = Some(super::fidchell::eff::SKILL_START_LIFE);
                return;
            }
            // The start's controller: its endFlag once its life is out.
            let Some(n) = x.eff_start else { return };
            if n > 1 {
                x.eff_start = Some(n - 1);
                return;
            }
            x.eff_start = None;
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
        2 if c == 50 => {
            let id = std::mem::replace(&mut x.stage_eff, -1);
            b.end_stage_effect(cx, id, DARK, 15);
            b.act_proccess += 1;
            b.act_count = 0;
        }
        3 if c == 15 => {
            cx.out(Out::CamMode { mode: 6 });
            cx.out(Out::Cinema(None));
            cx.out(Out::SwitchLayer);
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.out(Out::CameraChange(2));
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkDashCenter` (0x003aa7b0): the base's dash at 50 to `dashPos`,
/// 121 frames at most, off the targets.
fn on_dash_center(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
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

/// `OnThinkMagic` (0x003aa9b0): `@1568`'s second spell on the target under
/// the free eye, the stage dark; SE 245 at 30, the spell at 75; the camera
/// shaken from 100 to 160 frames on (the screen breaking up every 20), the
/// stage back at 240 (120 for spells 268 and 276), the neutral 15 after.
fn on_magic(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
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
            x.skill_id = cx.data.tarvos.magic_skill;
            cx.out(Out::CinemaSkill(x.skill_id));
            spell_camera(b, cx);
            b.act_proccess += 1;
        }
        1 => {
            if c == 30 {
                cx.out(Out::Se { se: 245 });
            }
            if c == 75 {
                if let Some(tp) = cx.scene.chars[me].target_char
                    && let Some(k) = item::item_skill_request(cx.t, cx.scene, me, tp, x.skill_id, 1, false, cx.rand)
                {
                    cx.out(Out::Skill(tp, k));
                }
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        2 => {
            let end = if matches!(x.skill_id, 268 | 276) { 120 } else { 240 };
            if cx.boss_cam && (100..161).contains(&c) {
                cx.quake_vec([0x420c_0000, 0x420c_0000, 0x420c_0000]);
                if (c - 100) % 20 == 0 {
                    cx.out(Out::Noise(20));
                }
            }
            if c == end {
                let id = std::mem::replace(&mut x.stage_eff, -1);
                b.end_stage_effect(cx, id, DARK, 15);
                b.act_proccess += 1;
                b.act_count = 0;
            }
        }
        3 if c == 15 => {
            cx.out(Out::SwitchLayer);
            cx.out(Out::Cinema(None));
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            b.unlock_player();
            if cx.boss_cam {
                cx.out(Out::CamMode { mode: 6 });
            }
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkPhantom` (0x003aaea0): at Kite (else the nearest), the phantom
/// (`ccBossEffPhantomCreate(pos, centerDirc)`); once it is off, boss skill
/// 25 from where Tarvos stands.
fn on_phantom(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            let kite = cx.world.player;
            let t = if target_ok(cx, kite) {
                kite
            } else {
                let t = b.select_target(cx, 0, FAR);
                if !target_ok(cx, t) {
                    exec_pattern(b, x, cx, 2, -1);
                    return;
                }
                t
            };
            let Some(t) = t else { return };
            let pos = cx.scene.chars[t].pos;
            b.lock_player(cx, true);
            cx.out(Out::Cinema(Some(25)));
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            let fx = Fx::Phantom(Box::new(eff::Phantom::new(cx, pos, b.center_dirc)));
            x.eff_phantom = Some(make_fx(b, cx, EffKind::Phantom, fx, pos));
            b.act_proccess += 1;
        }
        1 => {
            // ccGetBossEffAdrs's object: its flag read whether freed or not.
            if b.effects.enabled(x.eff_phantom) || x.eff_phantom.is_none_or(|id| id < 0) {
                return;
            }
            let pos = cx.pos();
            b.skill_damage(cx, None, Some(pos), 25);
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.out(Out::Cinema(None));
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkDataDrainAtk` (0x003ab0d0): a member drained through stream 104.
fn on_data_drain_atk(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 3);
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
            cx.out(Out::StreamMenu { stream: 104, mask: 1 << slot });
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

/// `OnThinkSpikeShot` (0x003ab360): SE 245 (note 57) at the target; at
/// 30 frames the spike rain; the target held (`EntryAffect(this, 5)`)
/// while it falls, the next pattern once it is off.
fn on_spike_shot(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            if c == 0 {
                let t = select(b, cx, 0);
                if !target_ok(cx, t) {
                    exec_pattern(b, x, cx, 2, -1);
                    return;
                }
                b.calc_target_info(cx);
                cx.out(Out::SeNote { se: 245, note: 57 });
            } else if c == 30 {
                let Some(t) = cx.scene.chars[me].target_char else { return };
                let pos = cx.scene.chars[t].pos;
                let fx = Fx::Spikes(Box::new(eff::SpikeRain::new(cx, t)));
                x.eff_spike = Some(make_fx(b, cx, EffKind::SpikeRain, fx, pos));
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        1 => {
            let t = cx.scene.chars[me].target_char;
            if let Some(tp) = t
                && target_ok(cx, t)
            {
                cx.affect(tp, 5, 0);
            }
            if !b.effects.enabled(x.eff_spike) {
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkWander` (0x003ab620): at 5 a random way for 60-89 frames, then
/// act 15 (the neutral the next frame); out of the arena, a wait of 30.
fn on_wander(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    b.move_spd = 0x40a0_0000;
    if b.stop != 0 || cx.scene.chars[me].cond[cond::HOLD] != 0 {
        b.move_spd = 0;
        return;
    }
    let pp = cx.scene.chars[me].pos_p;
    if !super::fidchell::valid_area(b, cx, pp) {
        b.stop_time = 30;
        b.stop_counter = 0;
        b.change_action(cx, act::IDLE, 3, true);
    }
    match b.act_proccess {
        0 => {
            b.move_dirc = rand_f(cx.cc, PI);
            b.act_count = (cx.cc.rand() % 30 + 60) as i16;
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count -= 1;
            if c == 0 {
                b.change_action(cx, act::IDLE, 3, true);
            }
        }
        _ => {}
    }
    b.dirc[2] = geom::set_dirc(b.dirc[2], b.move_dirc, 256);
}

/// `OnThinkTransfer` (0x003ab7b0): the guard changed. Off the targets
/// (nothing with the party down or a forced game over); SE 244 at the
/// clip's 80, 70, 65 and 55; past 90 stream 102; once the menu closes,
/// the trans effect; the fog thickening to 70 as the camera shakes; at
/// the clip's end a random place within 2000 of the centre, `ANM_ex71out0`
/// and the trans effect again; the fog thinning from the clip's 80 to 100;
/// at its end the other guard (`SetDefMode`) and the next pattern.
fn on_transfer(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    let me = cx.me;
    let f = b.anm.frame();
    match b.act_proccess {
        0 => {
            if cx.annihilated() || cx.game_over {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.body_hit_sw = 1;
            x.blend_rate = 0;
            let pos = cx.pos();
            cx.out(Out::Se3dNote { se: 245, pos, note: 48 });
            b.act_proccess += 1;
        }
        1 => {
            if matches!(f, 80 | 70 | 65 | 55) {
                cx.out(Out::Se { se: 244 });
            }
            if (f as u32) < 90 {
                return;
            }
            cx.out(Out::StreamMenu { stream: 102, mask: 0 });
            b.act_proccess += 1;
        }
        2 => {
            if cx.env.menu_type != -1 {
                return;
            }
            b.unlock_player();
            let pos = cx.pos();
            cx.out(Out::Tarvos(Pic::TransEffect { pos }));
            b.act_proccess += 1;
        }
        3 => {
            if matches!(f, 125 | 110 | 101) {
                cx.out(Out::SeNote { se: 244, note: 55 });
            }
            if matches!(f, 151 | 130 | 108 | 100) {
                let pos = cx.pos();
                cx.out(Out::Se3dNote { se: 56, pos, note: 60 });
            }
            quake(cx);
            let mut r = ee::add(x.blend_rate, ONE);
            if !ee::le(r, 0x428c_0000) {
                r = 0x428c_0000;
            }
            x.blend_rate = r;
            if b.anm_status == 0 {
                return;
            }
            let mut v = [rand_f(cx.cc, 0x44fa_0000), 0, 0, ONE];
            let m = geom::rot_matrix_z(&geom::unit_matrix(), rand_f(cx.cc, PI));
            v = geom::apply_matrix(&m, v);
            let p = geom::vadd(b.center_pos_p, v);
            cx.scene.chars[me].pos_p = p;
            cx.scene.chars[me].pos = super::p2w(cx, p);
            b.anm.set(OUT_CLIP, cx.clips);
            let pos = cx.pos();
            cx.out(Out::Tarvos(Pic::TransEffect { pos }));
            b.act_count = 0;
            b.act_proccess += 1;
        }
        4 => {
            if matches!(f, 65 | 55 | 45) {
                cx.out(Out::SeNote { se: 244, note: 55 });
            }
            let c = b.act_count;
            b.act_count += 1;
            if c == 45 {
                let pos = cx.pos();
                cx.out(Out::Tarvos(Pic::TransEffect { pos }));
            }
            if matches!(f, 89 | 74 | 60 | 48 | 21) {
                let pos = cx.pos();
                cx.out(Out::Se3dNote { se: 56, pos, note: 60 });
            }
            if f & 7 == 7 {
                quake(cx);
            }
            if (80..101).contains(&(f as u32)) {
                x.blend_rate = ee::sub(x.blend_rate, 0x4060_0000);
            }
            if ee::lt(x.blend_rate, 0) {
                x.blend_rate = 0;
            }
            if b.anm_status == 0 {
                return;
            }
            let d = if x.def_mode == 0 { 1 } else { 0 };
            set_def_mode(x, cx, d);
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkChase` (0x003abf20): after the target (`SelectTarget(3)`) at
/// 100, an after-image every fourth frame (SE 57 with the first); ended
/// within 150, after 150 frames, out of the arena or with the target
/// down.
fn on_chase(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 3);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.calc_target_info(cx);
            b.move_dirc = b.target_dirc;
            b.move_spd = 0x42c8_0000;
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c & 3 == 0 {
                cx.out(Out::AfterImage);
                if c == 0 {
                    let pos = cx.pos();
                    cx.out(Out::Se3dNote { se: 57, pos, note: 55 });
                }
            }
            let down = !target_ok(cx, cx.scene.chars[me].target_char);
            let pp = cx.scene.chars[me].pos_p;
            if ee::le(b.target_dist, 0x4316_0000) || c >= 150 || !super::fidchell::valid_area(b, cx, pp) || down {
                b.act_proccess += 1;
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                change_next_pattern(b, x, cx);
                return;
            }
            b.move_dirc = b.target_dirc;
            b.dirc[2] = b.target_dirc;
        }
        _ => {}
    }
}

/// `OnThinkEscape` (0x003ac210): away from the target at 30, turned a
/// quarter off it every 32 frames (toward the centre out of the arena),
/// for 300 frames. Macha's code.
fn on_escape(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
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

/// `OnThinkReturn` (0x003ac4d0): toward the centre at 5 for 90-119
/// frames, then act 15 with a wait of 30.
fn on_return(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    b.move_spd = 0x40a0_0000;
    b.move_dirc = b.center_dirc;
    b.dirc[2] = b.center_dirc;
    if b.stop != 0 || cx.scene.chars[me].cond[cond::HOLD] != 0 {
        b.move_spd = 0;
        return;
    }
    match b.act_proccess {
        0 => {
            b.act_count = (cx.cc.rand() % 30 + 90) as i16;
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count -= 1;
            if c == 0 {
                b.stop_time = 30;
                b.stop_counter = 0;
                b.change_action(cx, act::IDLE, 3, true);
            }
        }
        _ => {}
    }
}

/// `OnThinkEpitaphWave` (0x003ac750): `anmw` on `ANM_xx11wave` at 512; SE
/// 223 and 224 at the body's frames 11 and 25; from the count's 30 a
/// WaveShock and a flash, boss skill 23 at 45; the wave drawn through
/// `DrawParts` and the camera shaken every frame from 29, the next pattern
/// once both clips end.
fn on_epitaph_wave(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            x.anm_w.set(super::ANM_WAVE, cx.clips);
            x.anm_w.speed = 512;
            b.act_proccess += 1;
        }
        1 => {
            let f = b.anm.frame();
            if f == 11 {
                cx.se3d(223);
            } else if f == 25 {
                cx.se3d(224);
            }
            let c = b.act_count;
            b.act_count += 1;
            if c < 29 {
                return;
            }
            if b.act_count == 30 {
                let (pos, dirc) = (cx.pos(), b.dirc);
                b.effect_at(cx, EffKind::WaveShock, pos, dirc);
                cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
            } else if b.act_count == 45 {
                let pos = cx.pos();
                b.skill_damage(cx, None, Some(pos), 23);
            }
            quake(cx);
            let done = x.anm_w.forward();
            let pos = cx.pos();
            cx.out(Out::Tarvos(Pic::Wave { time: x.anm_w.posed, pos, dirc: b.dirc, parts: true }));
            if b.anm_status != 0 && done {
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

// --- Affect ---------------------------------------------------------------------------------

/// `ccBoss07::Affect` (0x003a9260): nothing while the player is locked,
/// nor (but for a drain, 21) while `actForbid` is 2. A blow (1, 3) with
/// damage in the neutral sounds 195 at a random note (60-64); 21 drains
/// (`OnThinkDrain`) and leaves the gauge at -1; 13 (a Data Drain on the
/// guarded boss) is dropped; the rest are the base's. Then, not drained,
/// on a blow: half the gauge brings the super table, and a blow of the
/// kind the guard holds off shows the shield (0.7 its size, 80 up).
pub(super) fn affect(b: &mut Boss, cx: &mut Cx) {
    let mut x = tarvos_of(b);
    affect_of(b, &mut x, cx);
    b.class = Class::Tarvos(x);
}

fn affect_of(b: &mut Boss, x: &mut Tarvos, cx: &mut Cx) {
    let me = cx.me;
    if b.lock_player != 0 {
        return;
    }
    let t0 = cx.scene.chars[me].affect.ty;
    if t0 != 21 && b.act_forbid == 2 {
        return;
    }
    let p0 = cx.scene.chars[me].affect.param[0];
    match t0 {
        1 | 3 => {
            if p0 > 0 && b.act_num == act::NEUTRAL {
                let pos = cx.pos();
                let note = (cx.cc.rand() % 5).unsigned_abs() as u8 + 60;
                cx.out(Out::Se3dNote { se: 195, pos, note });
            }
            b.base_affect(cx);
        }
        21 => {
            on_drain(b, x, cx);
            if let Some(p) = cx.scene.param_mut(me) {
                p.pp = -1;
                p.pp_count = 0;
            }
        }
        13 => cx.scene.chars[me].affect.ty = 0,
        _ => b.base_affect(cx),
    }
    let t = cx.scene.chars[me].affect.ty;
    if b.epitaph != 0 || !matches!(t, 1 | 3) {
        return;
    }
    if x.pattern_mode == 0 {
        let pp = cx.scene.param(me).map_or(0, |p| p.pp);
        let max = cx.scene.chars[me].foe_state().map_or(0, |f| f.row.max_pp);
        if pp >= max >> 1 {
            set_pattern_mode(b, x, cx, 1);
        }
    }
    if b.epitaph != 0 {
        return;
    }
    let sid = i32::from(cx.scene.chars[me].affect.param[1]);
    let kind = crate::skill::check_type_of(cx.t, sid);
    let shield = match x.def_mode {
        0 => kind == 0,
        1 => kind == 1,
        _ => false,
    };
    if shield {
        cx.out(Out::Tarvos(Pic::Shield { magic: x.def_mode }));
    }
}
