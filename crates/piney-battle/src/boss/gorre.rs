//! Gorre (`ccBoss05`, boss05.cpp) and its two `ccBoss05Brother`s, Outbreak's
//! second phase boss: `bossTbl` row 4, `bossFunc` code 4, fought in event
//! 218 in field 5. Ported from Outbreak's gcmn.prg (0x00497010-0x0049e590;
//! names and layouts from Infection's DWARF, whose gcmn carries the class
//! unused), with Outbreak's `ccBoss` base under it ([`super`]). Gorre stays
//! out of reach and orders its two brothers, each its own `ccChar` beside
//! it, into the attacks; a brother's own state is a [`Boss`] of its own
//! ([`Part`]), run from Gorre's own frame the way Kyvia runs its core and
//! gomoras (boss-kyvia.md's `Tree`).
//! docs/engine/boss-gorre.md.

pub mod brother;
#[cfg(test)]
mod tests;

use piney_data::field::ee;
use piney_data::volume::Volume;

use self::brother::Brother;
use super::kyvia::Part;
use super::{Boss, Class, Cx, EffKind, Out, Tbl, VF0};
use crate::enemy_ai::get_dirc;
use crate::geom::{self, V4};
use crate::item;

type F = u32;

const ONE: F = 0x3f80_0000;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// `SelectTarget`'s radius here: 999999.
const FAR: F = 0x4974_23f0;

/// `bossFunc`'s code for Gorre, its `bossTbl` row and its file.
pub const CODE: i32 = 4;
pub const ROW: usize = 4;
pub const FILE: &str = "x51";
/// The brothers' own `bossTbl` rows (`ccBoss05Brother::Init`,
/// `ccGetBossParam(id ? 41 : 40)`): distinct from Gorre's own row 4.
pub const BROTHER_ROW: [usize; 2] = [40, 41];
/// The brothers' formation offset off Gorre on x, one each side
/// (`ccBoss05::ccBoss05`, OUT gcmn ctor): 300.
pub const BROTHER_OFFSET: F = 0x4396_0000;
/// `InitBossCamera(200, 1500)`.
pub const CAM_Z: F = 0x4348_0000;
pub const CAM_Y: F = 0x44bb_8000;

/// Gorre's acts (`actNum`, `Think`'s switch at OUT gcmn 0x00498670, INF
/// 0x00498670 table `@1462`, 23 words); acts 9, 10, 17 and 19 are the
/// table's own trap (`sw $zero, 0($zero)`) and never reached from the
/// boss's own tables.
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DMG0: i16 = 1;
    pub const DMG1: i16 = 2;
    pub const KERSE: i16 = 3;
    pub const WAVE: i16 = 4;
    pub const TALK: i16 = 5;
    pub const DATA_DRAIN_ATK: i16 = 6;
    pub const DASH_CENTER: i16 = 7;
    pub const WANDER: i16 = 8;
    pub const DRAIN: i16 = 11;
    pub const EPITAPH: i16 = 12;
    pub const EPITAPH_WAVE: i16 = 13;
    pub const DEAD: i16 = 14;
    pub const NEUTRAL_AGAIN: i16 = 15;
    pub const ESCAPE: i16 = 16;
    pub const CHASE: i16 = 18;
    pub const SKILL: i16 = 20;
    pub const TORNADE: i16 = 21;
    pub const MAGIC: i16 = 22;
}

/// Gorre's tables (`tables::combat`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GorreData {
    /// `boss05NormalActTbl`, `boss05SuperActTbl`, `boss05EpitaphActTbl`,
    /// each to its `-1`.
    pub normal: Vec<i32>,
    pub super_: Vec<i32>,
    pub epitaph: Vec<i32>,
    /// `Boss05AnmTbl`: the act's clip.
    pub anims: Vec<Option<String>>,
    /// `boss05SlaveAnmTbl1`, `boss05SlaveAnmTbl2`: each brother's own clip
    /// by act (`ccBoss05Brother::Init`, by `CheckSlaveID`).
    pub brother_anims: [Vec<Option<String>>; 2],
    /// `@1261`: `OnThinkSkill`'s three spells, one by `ccRand() % 3`; the
    /// same read (and dropped) by `ExecPatternIndex`'s pattern 10.
    pub skills: [i32; 3],
    /// `@1777`: `OnThinkMagic`'s four spells, by `ccRand() % 3` (the
    /// fourth never drawn).
    pub magic_skills: [i32; 4],
}

impl GorreData {
    /// The volume's.
    pub fn of(volume: Volume) -> GorreData {
        let t = piney_data::tables::combat::of(volume);
        let s = |v: &[Option<&str>]| v.iter().map(|a| a.map(str::to_string)).collect::<Vec<_>>();
        let pick = |v: &[i32], k: usize| v.get(k).copied().unwrap_or(0);
        GorreData {
            normal: t.gorre_normal().to_vec(),
            super_: t.gorre_super().to_vec(),
            epitaph: t.gorre_epitaph().to_vec(),
            anims: s(t.gorre_anims()),
            brother_anims: [s(t.gorre_brother_anims()), s(t.gorre_brother2_anims())],
            skills: std::array::from_fn(|k| pick(t.gorre_skills(), k)),
            magic_skills: std::array::from_fn(|k| pick(t.gorre_magic_skills(), k)),
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

/// `ccBoss05`'s own members: the two brothers (+0x29350, +0x29354), each
/// its own [`Boss`] on its own scene character (`brother_me`, persistent);
/// `brothers` is scratch, filled only for the span of [`main`], the way
/// Kyvia's `Tree` holds its core and gomoras.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gorre {
    pub brother_me: [usize; 2],
    pub brothers: [Part<Brother>; 2],
    /// The dead effect once both brothers are down and the Epitaph's
    /// table runs its course (not measured against the game: `SendMessage`
    /// msg 21, which touches `boss05SuperActTbl` and a party-over check,
    /// is not yet decoded past a stat reset).
    pub eff_dead: Option<i32>,
    /// Frames since both brothers went down: a stand-in for whatever the
    /// Epitaph's own table (or `SendMessage` msg 21) actually does once
    /// it runs its course; past [`EPITAPH_GRACE`] Gorre is taken as
    /// finished too, so the fight can end.
    pub epitaph_frames: i32,
}

/// See [`Gorre::epitaph_frames`].
const EPITAPH_GRACE: i32 = 300;

fn gorre_of(b: &mut Boss) -> Box<Gorre> {
    match std::mem::replace(&mut b.class, Class::Plain) {
        Class::Gorre(x) => x,
        other => {
            b.class = other;
            Box::default()
        }
    }
}

/// A brother taken out of its own character (`Class::GorreBrother`,
/// `super::kyvia::Tree::take`'s way): the part, and whether one was there
/// to take (a wrongly-typed or missing slot is left alone and this is
/// `false`, so [`put_brother`] knows not to write a placeholder over it).
fn take_brother(cx: &mut Cx, me: usize) -> (Part<Brother>, bool) {
    let taken = cx.scene.chars.get_mut(me).and_then(|c| c.foe_state_mut()).and_then(|f| f.boss.take());
    let Some(mut gb) = taken else { return (Part { me, ..Part::default() }, false) };
    match std::mem::replace(&mut gb.class, Class::Plain) {
        Class::GorreBrother(x) => (Part { b: gb, x, me }, true),
        other => {
            gb.class = other;
            if let Some(f) = cx.scene.chars[me].foe_state_mut() {
                f.boss = Some(gb);
            }
            (Part { me, ..Part::default() }, false)
        }
    }
}

/// A brother put back into its own character, unless [`take_brother`]
/// found nothing there to begin with.
fn put_brother(cx: &mut Cx, mut part: Part<Brother>, found: bool) {
    if !found {
        return;
    }
    part.b.class = Class::GorreBrother(part.x);
    if let Some(f) = cx.scene.chars[part.me].foe_state_mut() {
        f.boss = Some(part.b);
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
    super::sqrt_of(cx, ee::add(ee::mul(x, x), ee::mul(y, y)))
}

fn target_ok(cx: &Cx, t: Option<usize>) -> bool {
    cx.valid(t) && t.is_some_and(|t| !cx.dead(t))
}

fn select(b: &mut Boss, cx: &mut Cx, ty: i32) -> Option<usize> {
    let t = b.select_target(cx, ty, FAR);
    cx.scene.chars[cx.me].target_char = t;
    t
}

fn cursor(cx: &mut Cx, on: bool) {
    cx.out(Out::CursorOff(on));
}

fn cinema_off(cx: &mut Cx) {
    cx.out(Out::Cinema(None));
}

/// `this->br0->Order(on)` / `br1->Order(on)`: `cx.me` swapped to the
/// brother's own scene character for the call.
fn order_brother(x: &mut Gorre, k: usize, cx: &mut Cx, on: i32) {
    let me = x.brothers[k].me;
    let old = cx.me;
    cx.me = me;
    brother::order(&mut x.brothers[k].b, cx, on);
    cx.me = old;
}

/// `ccBoss05::ChangeAction` (OUT gcmn 0x004985c0): the base's own
/// `ChangeAction`, then both brothers `Order`'d into the same act (not
/// called from [`new`]: the brothers do not exist yet at its own first
/// `ChangeAction`).
fn change_action(b: &mut Boss, x: &mut Gorre, cx: &mut Cx, act: i16, forbid: i16, af: bool) {
    b.change_action(cx, act, forbid, af);
    for k in 0..2 {
        order_brother(x, k, cx, act as i32);
    }
}

/// `CalcTargetInfo`'s target part, since Gorre's own `Main` only inlines
/// the centre's (`ccBoss::CalcTargetInfo`, OUT gcmn 0x0046ff20).
fn calc_target_info(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    let Some(t) = cx.scene.chars[me].target_char.filter(|&t| cx.scene.listed(t)) else { return };
    b.target_pos_p = cx.scene.chars[t].pos_p;
    b.target_pos = cx.scene.chars[t].pos;
    let pp = cx.scene.chars[me].pos_p;
    b.target_dirc = get_dirc(pp, b.target_pos_p);
    b.target_dist = dist(cx, pp, b.target_pos_p);
}

/// `ccBoss::CalcEffectCameraPos`-shaped focus used by several attacks
/// (`OnThinkKerse`, `OnThinkTornade`, `OnThinkSkill`, `OnThinkMagic`): a
/// place `off` from `vp`, turned by `d` (Rz, Rx, Ry, Rz), then camera 3
/// there looking at `vp`.
fn focus_camera(cx: &mut Cx, vp: V4, d: V4, off: V4) -> (V4, V4) {
    let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
    let m = geom::rot_matrix_x(&m, d[0]);
    let m = geom::rot_matrix_y(&m, d[1]);
    let m = geom::rot_matrix_z(&m, d[2]);
    let v = geom::apply_matrix(&m, off);
    let eye = super::p2w(cx, geom::vadd(super::w2p(cx, vp), v));
    cx.out(Out::CameraChange(3));
    cx.out(Out::CameraPos { cam: 3, pos: eye });
    cx.out(Out::CameraView { cam: 3, view: vp });
    (eye, vp)
}

// --- the constructor ------------------------------------------------------------------------

/// `ccBoss05::ccBoss05` (OUT gcmn ctor): the boss is `scene.chars[cx.me]`
/// (`bossTbl` row 4); it stands 500 behind Kite. Two `ccBoss05Brother`s are
/// made, each its own character of row 4 too (`Init`, `TransPosB2W` off
/// `BROTHER_OFFSET` either way of it, their heading copied off Gorre), and
/// entered (`EntrySlave`).
pub fn new(cx: &mut Cx, kite_pos: V4, kite_dirc: V4, center: V4) -> Boss {
    let me = cx.me;
    let mut b = super::kyvia::plain_boss();
    b.anm_tbl = cx.data.gorre.anims.clone();
    b.exit = 0;
    b.draw_sw = 1;
    // `OffBodyHit()` (vtable +0x60), not On: Gorre itself takes no direct
    // hit (`Affect` is empty; the brothers carry the body hit).
    b.body_hit_sw = 0;
    b.cheat_hp = 1;
    let mut pos = kite_pos;
    pos[1] = ee::sub(pos[1], 0x43fa_0000);
    cx.scene.chars[me].pos = pos;
    b.dirc = kite_dirc;
    b.change_action(cx, act::NEUTRAL, 0, true);
    crate::fellow::entry_cmnd(cx.scene, me);
    b.center_pos = [center[0], center[1], center[2], ONE];
    b.center_pos_p = super::w2p(cx, b.center_pos);
    b.use_stage_eff = true;
    cx.out(Out::CamMaxRange(CAM_Y));

    let mut brother_me = [0usize; 2];
    for (k, slot) in brother_me.iter_mut().enumerate() {
        let row = BROTHER_ROW[k];
        let bm = super::kyvia::new_char(cx, row);
        super::kyvia::set_base_param(cx, bm, row);
        crate::fellow::entry_cmnd(cx.scene, bm);
        let sign = if k == 0 { BROTHER_OFFSET } else { BROTHER_OFFSET ^ 0x8000_0000 };
        let part = brother::new(cx, bm, me, [sign, 0, 0, ONE], kite_dirc, k as i32);
        put_brother(cx, part, true);
        *slot = bm;
    }

    let mut x = Gorre { brother_me, ..Gorre::default() };
    b.pat_tbl = Tbl::Normal;
    b.pat_index = 0;
    b.pat_mode = 0;
    let found: [bool; 2] = std::array::from_fn(|k| {
        let (part, found) = take_brother(cx, brother_me[k]);
        x.brothers[k] = part;
        found
    });
    let tbl = cx.data.gorre.words(Tbl::Normal).to_vec();
    b.pat_index = exec_pattern_index(&mut b, &mut x, cx, &tbl, 0);
    for (k, ok) in found.into_iter().enumerate() {
        put_brother(cx, std::mem::take(&mut x.brothers[k]), ok);
    }
    b.class = Class::Gorre(Box::new(x));
    b
}

// --- the frame --------------------------------------------------------------------------------

pub(super) fn main(b: &mut Boss, cx: &mut Cx) {
    let mut x = gorre_of(b);
    b.effects.tick();
    let found: [bool; 2] = std::array::from_fn(|k| {
        let (part, found) = take_brother(cx, x.brother_me[k]);
        x.brothers[k] = part;
        found
    });
    frame(b, &mut x, cx);
    for k in 0..2 {
        brother::frame(b, &mut x, k, cx);
    }
    for (k, ok) in found.into_iter().enumerate() {
        put_brother(cx, std::mem::take(&mut x.brothers[k]), ok);
    }
    b.class = Class::Gorre(x);
}

/// `ccBoss05::Main` (OUT gcmn 0x00498110): `CalcRealEx(0)`, the centre's
/// distance and heading (`ccGetDist`, its `sqrt.s`), the stop's count,
/// `Think`, the party held while `lockPlayer`, `Move`, the boss camera's
/// `CamMain`, the stage fader, `PreDrawAnm`, the draws, `hold` cleared.
fn frame(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
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
    // Both brothers down (their own `SendMessage(this, 4)`, approximated
    // as a poll here): drained, to the Epitaph's table; past its grace
    // Gorre itself is taken as finished (see `epitaph_frames`).
    if b.epitaph == 0 && cx.scene.chars[x.brothers[0].me].hp <= 0 && cx.scene.chars[x.brothers[1].me].hp <= 0 {
        b.epitaph = 1;
        on_drain(b, x, cx);
    } else if b.epitaph != 0 && b.act_num != act::DEAD {
        x.epitaph_frames += 1;
        if x.epitaph_frames >= EPITAPH_GRACE {
            change_action(b, x, cx, act::DEAD, 2, true);
        }
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
    cx.scene.chars[me].cond[crate::param::cond::HOLD] = 0;
}

// --- the patterns --------------------------------------------------------------------------------

fn change_next_pattern(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let tbl = cx.data.gorre.words(b.pat_tbl).to_vec();
    if word(&tbl, b.pat_index) == -1 {
        b.pat_index = 0;
    }
    b.move_spd = 0;
    b.move_vector = VF0;
    if b.epitaph != 0 {
        change_action(b, x, cx, act::EPITAPH, 3, true);
    } else {
        change_action(b, x, cx, act::NEUTRAL, 1, true);
    }
    b.pat_index = exec_pattern_index(b, x, cx, &tbl, b.pat_index);
}

fn exec_pattern(b: &mut Boss, x: &mut Gorre, cx: &mut Cx, pat: i32, p0: i32) {
    let tbl = [pat, p0, 0, 0, -1];
    exec_pattern_index(b, x, cx, &tbl, 0);
}

/// `ccBoss05::ExecPatternIndex` (OUT gcmn 0x004978d0), falling back on
/// `ccBoss::ExecPatternIndex` ([`Boss::base_exec_pattern_index`], whose act
/// numbers for patterns 2-8 and 12 already match Gorre's own `act`
/// module).
fn exec_pattern_index(b: &mut Boss, x: &mut Gorre, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
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
        19 => change_action(b, x, cx, act::MAGIC, 3, true),
        18 => change_action(b, x, cx, act::DATA_DRAIN_ATK, 3, true),
        17 => change_action(b, x, cx, act::TORNADE, 3, true),
        16 => change_action(b, x, cx, act::TALK, 3, true),
        15 => change_action(b, x, cx, act::KERSE, 3, true),
        1 => {
            let n = if b.epitaph != 0 { act::EPITAPH_WAVE } else { act::WAVE };
            change_action(b, x, cx, n, 3, true);
        }
        9..=11 => {
            if pat != 11 {
                let ty = word(tbl, s);
                s += 1;
                if ty < 0 {
                    // A living member's pick: those on the lists and down
                    // (dead), one by ccRand (the first draw dropped).
                    let _ = cx.cc.rand();
                    let mut down = Vec::new();
                    for m in cx.party.members.into_iter().flatten() {
                        if cx.valid(Some(m)) && cx.dead(m) {
                            down.push(m);
                        }
                    }
                    if down.is_empty() {
                        exec_pattern(b, x, cx, 2, -1);
                        return orig;
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
            if pat == 10 {
                // @1261 read (rand()%3 abs) and dropped: OnThinkSkill draws
                // its own.
                let _ = (cx.cc.rand() % 3).abs();
                change_action(b, x, cx, act::SKILL, 3, true);
                return s;
            }
            let sid = word(tbl, s);
            s += 1;
            let (br0, br1) = (x.brothers[0].me, x.brothers[1].me);
            if pat == 11 {
                if let Some(k) = item::item_skill_request(cx.t, cx.scene, br1, br0, sid, 0, false, cx.rand) {
                    cx.out(Out::SkillFrom { user: br1, target: br0, skill: k });
                }
                if let Some(k) = item::item_skill_request(cx.t, cx.scene, br1, br1, sid, 0, false, cx.rand) {
                    cx.out(Out::SkillFrom { user: br1, target: br1, skill: k });
                }
            } else if let Some(t) = cx.scene.chars[me].target_char
                && let Some(k) = item::item_skill_request(cx.t, cx.scene, br1, t, sid, 0, false, cx.rand)
            {
                cx.out(Out::SkillFrom { user: br1, target: t, skill: k });
            }
            exec_pattern(b, x, cx, 2, 60);
        }
        _ => {
            // The base's own act numbers for patterns 2-8 and 12 already
            // match Gorre's own `act` module; `ccBoss05::ChangeAction`'s
            // own brother `Order` (wrapped here as [`change_action`]) is
            // not itself reached through the base, so it is repeated here
            // whenever the base's own call actually changed the act.
            let before = b.act_num;
            let r = b.base_exec_pattern_index(cx, tbl, orig);
            if b.act_num != before {
                for k in 0..2 {
                    order_brother(x, k, cx, b.act_num as i32);
                }
            }
            return r;
        }
    }
    s
}

// --- Think ----------------------------------------------------------------------------------

/// `ccBoss05::Think` (OUT gcmn 0x00498670).
fn think(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    match b.act_num {
        act::NEUTRAL => on_neutral(b, x, cx),
        act::DMG0 | act::DMG1 => on_dmg(b, x, cx),
        act::KERSE => on_kerse(b, x, cx),
        act::WAVE => on_wave(b, x, cx),
        act::TALK => on_talk(b, x, cx),
        act::DATA_DRAIN_ATK => on_data_drain_atk(b, x, cx),
        act::DASH_CENTER => on_dash_center(b, x, cx),
        act::WANDER => change_next_pattern(b, x, cx),
        act::DRAIN => on_drain(b, x, cx),
        act::EPITAPH => on_epitaph(b, x, cx),
        act::EPITAPH_WAVE => on_wave(b, x, cx),
        act::DEAD => on_dead(b, x, cx),
        act::NEUTRAL_AGAIN => on_neutral(b, x, cx),
        act::ESCAPE => on_escape(b, x, cx),
        act::CHASE => on_chase(b, x, cx),
        act::SKILL => on_skill(b, x, cx),
        act::TORNADE => on_tornade(b, x, cx),
        act::MAGIC => on_magic(b, x, cx),
        // The table's own trap (acts 9, 10, 17, 19): unreached from the
        // boss's own tables.
        _ => {}
    }
}

/// `OnThinkNeutral` (0x004987f0) and its `act` 15 twin, and `OnThinkEpitaph`
/// (0x0049b0d0, which also counts): the wait (`Boss::wait_over`), then the
/// next pattern at the nearest.
fn on_neutral(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
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

fn on_epitaph(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    b.act_count += 1;
    on_neutral(b, x, cx);
}

/// `OnThinkDmg` (0x004990e0): back to the neutral at once.
fn on_dmg(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let n = if b.epitaph != 0 { act::EPITAPH } else { act::NEUTRAL };
    change_action(b, x, cx, n, 1, true);
    b.spd_down(0, 0x40a0_0000);
}

/// `OnThinkDrain` (0x00498940): drained; the Epitaph's table.
fn on_drain(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    change_action(b, x, cx, act::EPITAPH, 3, true);
    b.pat_tbl = Tbl::Epitaph;
    let tbl = cx.data.gorre.words(Tbl::Epitaph).to_vec();
    b.pat_index = exec_pattern_index(b, x, cx, &tbl, 0);
}

/// `OnThinkWave` (0x0049ac00) and `OnThinkEpitaphWave`: both brothers
/// ordered into their own wave pose, a wave shock at each's place, a
/// quake and a flash checked at the boss's own place.
fn on_wave(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            let n = if b.epitaph != 0 { brother::act::EPITAPH_WAVE } else { brother::act::WAVE };
            for k in 0..2 {
                order_brother(x, k, cx, n as i32);
            }
            b.act_proccess += 1;
            b.act_count = 0;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 30 {
                let pos = cx.pos();
                cx.out(Out::CameraShake { pos, s: [115, 115, 115, 115] });
                cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
                for k in 0..2 {
                    let bp = cx.scene.chars[x.brothers[k].me].pos;
                    let d = x.brothers[k].b.dirc;
                    x.brothers[k].b.effect_at(cx, EffKind::WaveShock, bp, d);
                }
            }
            if c >= 90 {
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                let n = if b.epitaph != 0 { act::EPITAPH } else { act::NEUTRAL };
                for k in 0..2 {
                    order_brother(x, k, cx, if n == act::EPITAPH { 12 } else { 0 });
                }
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkTalk` (0x0049a9d0): a member locked into a stream menu (a
/// dialogue), then boss skill 20 on it.
fn on_talk(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 2);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            calc_target_info(b, cx);
            b.lock_player(cx, false);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.act_proccess += 1;
            b.act_count = 0;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c < 71 {
                return;
            }
            if let Some(t) = cx.scene.chars[me].target_char {
                let pos = cx.scene.chars[t].pos;
                b.skill_damage(cx, Some(t), Some(pos), 20);
            }
            cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkDataDrainAtk` (0x00499e10): a member drained (`EntryAffect`
/// type 13), as Fidchell's `OnThinkDrainAtk`.
fn on_data_drain_atk(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
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
            b.act_proccess += 1;
            b.act_count = 0;
        }
        1 if c == 45 => {
            let id = cx.scene.chars[me].target_char.map_or(-1, |t| i32::from(cx.scene.chars[t].id()));
            let slot = cx.party.slot_of(id);
            cx.out(Out::StreamMenu { stream: 72, mask: 1 << slot });
            b.act_proccess += 1;
        }
        2 if cx.env.menu_type == -1 => {
            if let Some(tp) = cx.scene.chars[me].target_char
                && target_ok(cx, Some(tp))
            {
                cx.affect(tp, 13, 0);
            }
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkDashCenter` (0x0049a020): the base's dash (act 7) at 50, both
/// brothers held (`EntryAffect` type 5) while it runs.
fn on_dash_center(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let me = cx.me;
    b.spd_up(0x4248_0000, 0x4248_0000);
    match b.act_proccess {
        0 => {
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            cx.se3d(229);
            b.act_proccess += 1;
            b.act_count = 0;
        }
        1 => {
            if b.act_count & 3 == 3 {
                cx.out(Out::AfterImage);
                if b.act_count == 3 {
                    cx.out(Out::Se3dNote { se: 57, pos: cx.pos(), note: 55 });
                    cx.se3d(229);
                }
            }
            for k in 0..2 {
                if cx.valid(Some(x.brothers[k].me)) {
                    cx.affect(x.brothers[k].me, 5, 32767);
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

/// `OnThinkChase` (0x0049ae50): at the target (type 7) until close or out
/// of time; a dash of 500 out of the arena.
fn on_chase(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            if !target_ok(cx, cx.scene.chars[me].target_char) {
                let t = select(b, cx, 7);
                if !target_ok(cx, t) {
                    change_next_pattern(b, x, cx);
                    return;
                }
            }
            calc_target_info(b, cx);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.move_dirc = b.target_dirc;
            b.move_spd = 0x42c8_0000;
            b.act_proccess += 1;
            b.act_count = 0;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if ee::lt(b.target_dist, 0x4348_0000) || c >= 60 {
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                change_next_pattern(b, x, cx);
                return;
            }
            let pp = cx.scene.chars[me].pos_p;
            if !b.valid_area(pp) {
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                exec_pattern(b, x, cx, 6, 500);
            }
        }
        _ => {}
    }
    b.dirc[2] = b.move_dirc;
}

/// `OnThinkEscape` (part of the base's fallback for other patterns; Gorre
/// only reaches it from `ChangeAction`'s own direct act, never a
/// pattern). Away from the target at a fixed speed.
fn on_escape(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            b.move_spd = 0x42c8_0000;
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c >= 150 || !ee::le(b.target_dist, CAM_Y) {
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
    b.set_dirc(b.move_dirc);
}

/// `OnThinkKerse` (0x004991b0): a camera focus on the target, a stage
/// dark, a finishing flash effect, then boss skill 19 on it.
fn on_kerse(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
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
            b.lock_player(cx, true);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            cx.out(Out::SwitchLayer);
            b.stage_eff_id = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 30000]);
            let mut vp = b.target_pos;
            vp[2] = ee::add(vp[2], 0x4348_0000);
            let d = [0, 0, b.target_dirc, ONE];
            let off = [0x44bb_8000, 0, 0x4248_0000, ONE];
            focus_camera(cx, vp, d, off);
            cx.se3d(234);
        }
        0 if c >= 75 => {
            let tp = b.target_pos;
            b.effect_at(cx, EffKind::FinalPhotonFlash, tp, VF0);
            cx.out(Out::SeNote { se: 191, note: 50 });
            cx.se3d(191);
            b.act_proccess += 1;
            b.act_count = 0;
        }
        1 if c >= 15 => {
            let pos = cx.pos();
            b.skill_damage(cx, cx.scene.chars[me].target_char, Some(pos), 19);
            let id = std::mem::replace(&mut b.stage_eff_id, -1);
            b.end_stage_effect(cx, id, 0x6400_0000, 15);
            b.act_proccess += 1;
            b.act_count = 0;
        }
        2 if c >= 15 => {
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.out(Out::SwitchLayer);
            b.unlock_player();
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkTornade` (0x0049a290): a camera focus off the centre, then a
/// whirlwind (its picture stands in as a `WaveShock` at the target for
/// now).
fn on_tornade(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 if c == 0 => {
            let t = select(b, cx, 3);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            calc_target_info(b, cx);
            b.lock_player(cx, true);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            let mut vp = cx.pos();
            vp[2] = ee::add(vp[2], 0x4396_0000);
            let d = [0, 0, b.center_dirc, ONE];
            let off = [0x44bb_8000, 0, 0x4248_0000, ONE];
            focus_camera(cx, vp, d, off);
        }
        0 if c >= 60 => {
            b.act_proccess += 1;
            b.act_count = 0;
        }
        1 => {
            if c == 0 {
                let tp = b.target_pos;
                b.effect_at(cx, EffKind::WaveShock, tp, VF0);
                for k in 0..2 {
                    order_brother(x, k, cx, brother::act::TORNADE as i32);
                }
            }
            if c >= 90 {
                let pos = b.target_pos;
                b.skill_damage(cx, cx.scene.chars[me].target_char, Some(pos), 21);
                for k in 0..2 {
                    order_brother(x, k, cx, brother::act::NEUTRAL as i32);
                }
                b.act_proccess += 1;
                b.act_count = 0;
            }
        }
        2 => {
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.out(Out::CameraChange(2));
            b.unlock_player();
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkSkill` (0x0049a4e0): one of `@1261` on the target, under
/// `effSkillStart`'s own controller, as Fidchell's `OnThinkSkill`.
fn on_skill(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let me = cx.me;
    b.set_dirc(b.target_dirc);
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 0);
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            calc_target_info(b, cx);
            cx.out(Out::SwitchLayer);
            b.stage_eff_id = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 30000]);
            let mut vp = b.target_pos;
            vp[2] = ee::add(vp[2], 0x4348_0000);
            let d = [0, 0, b.target_dirc, ONE];
            let off = [0x447a_0000, 0, 0, ONE];
            focus_camera(cx, vp, d, off);
            let k = (cx.cc.rand() % 3).unsigned_abs() as usize;
            let sid = cx.data.gorre.skills[k];
            cx.out(Out::SkillStart { who: me, sid });
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.lock_player(cx, true);
            b.act_proccess += 1;
            b.act_count = 0;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c < 50 {
                return;
            }
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            if let Some(tp) = cx.scene.chars[me].target_char
                && let Some(k) =
                    item::item_skill_compel(cx.t, cx.scene, me, tp, cx.data.gorre.skills[0], 0, false, cx.rand)
            {
                cx.out(Out::Skill(tp, k));
            }
            let id = std::mem::replace(&mut b.stage_eff_id, -1);
            b.end_stage_effect(cx, id, 0x6400_0000, 15);
            b.act_proccess += 1;
            b.act_count = 0;
        }
        2 => {
            let c = b.act_count;
            b.act_count += 1;
            if c >= 15 {
                b.entry_cmnd_target(cx);
                cursor(cx, false);
                cx.out(Out::SwitchLayer);
                b.unlock_player();
                cx.out(Out::CameraChange(2));
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkMagic` (0x004999e0): one of `@1777`'s spells cast on the
/// target under camera 3, the stage darkened.
fn on_magic(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
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
            cx.out(Out::SwitchLayer);
            b.stage_eff_id = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 30000]);
            b.lock_player(cx, true);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            let mut vp = b.target_pos;
            vp[2] = ee::add(vp[2], 0x4348_0000);
            let d = [0, 0, b.target_dirc, ONE];
            let off = [0x44bb_8000, 0, 0x4248_0000, ONE];
            focus_camera(cx, vp, d, off);
        }
        0 if c >= 60 => {
            let k = (cx.cc.rand() % 3).unsigned_abs() as usize;
            let sid = cx.data.gorre.magic_skills[k];
            if let Some(t) = cx.scene.chars[me].target_char
                && let Some(sk) = item::item_skill_request(cx.t, cx.scene, me, t, sid, 0, false, cx.rand)
            {
                cx.out(Out::Skill(t, sk));
            }
            let id = std::mem::replace(&mut b.stage_eff_id, -1);
            b.end_stage_effect(cx, id, 0x6400_0000, 15);
            b.act_proccess += 1;
            b.act_count = 0;
        }
        1 if c >= 15 => {
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.out(Out::SwitchLayer);
            b.unlock_player();
            cx.out(Out::CameraChange(2));
            cinema_off(cx);
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkDead` (0x004989f0): a camera behind Gorre toward the brothers'
/// place, then the dead effect (`begin_dead_effect`, as the base's own
/// bosses use) until it ends, then `exit`. Not measured against the
/// game's own two-phase camera math (docs/engine/boss-gorre.md).
fn on_dead(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            b.move_spd = 0;
            b.move_vector = VF0;
            let br0 = cx.scene.chars[x.brothers[0].me].pos;
            let br1 = cx.scene.chars[x.brothers[1].me].pos;
            let mid = geom::vscale(geom::vadd(br0, br1), 0x3f00_0000);
            let view = cx.pos();
            let d = get_dirc(view, mid);
            let m = geom::rot_matrix_z(&geom::unit_matrix(), d);
            let off = geom::apply_matrix(&m, [0x453b_8000, 0, 0x42c8_0000, ONE]);
            let eye = super::p2w(cx, geom::vadd(super::w2p(cx, view), off));
            cx.out(Out::CameraChange(3));
            cx.out(Out::CameraPos { cam: 3, pos: eye });
            cx.out(Out::CameraView { cam: 3, view });
            for k in 0..2 {
                order_brother(x, k, cx, brother::act::DEAD as i32);
            }
            crate::fellow::delete_cmnd(cx.scene, cx.me);
            cx.out(Out::DeleteCmnd);
            x.eff_dead = Some(b.effect(cx, EffKind::Dead));
            b.act_proccess += 1;
        }
        1 if x.eff_dead.is_some() && !b.effects.enabled(x.eff_dead) => {
            x.eff_dead = None;
            b.exit = 1;
        }
        _ => {}
    }
}

// --- Affect ----------------------------------------------------------------------------------

/// `ccBoss05::Affect` (OUT gcmn 0x00498100): empty. Gorre itself takes no
/// direct hit; the fight is won through its brothers.
pub(super) fn affect(_b: &mut Boss, _cx: &mut Cx) {}

/// `ccBoss05Brother::Affect`, for a `Class::GorreBrother` character.
pub(super) fn brother_affect(b: &mut Boss, cx: &mut Cx) {
    brother::affect(b, cx);
}
