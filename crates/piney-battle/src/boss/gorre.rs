//! Gorre (`ccBoss05`, boss05.cpp) and its two `ccBoss05Brother`s, Outbreak's
//! second phase boss: `bossTbl` row 4, `bossFunc` code 4, fought in event
//! 218 in field 5. Outbreak's gcmn.prg 0x004a9130-0x004b0b50 (names and
//! layouts from Infection's DWARF). Gorre is never listed nor struck; it
//! orders its brothers (each a [`Boss`] on its own character, a [`Part`]
//! for the span of Gorre's frame, as Kyvia's core and gomoras), which
//! report back ([`message`]) and share its one HP. docs/engine/boss-gorre.md.

pub mod brother;
mod message;
#[cfg(test)]
mod tests;

use piney_data::field::ee;
use piney_data::volume::Volume;

use self::brother::Brother;
use super::kyvia::Part;
use super::{Boss, Class, Cx, EffKind, Out, PI, TWO_PI, Tbl, VF0};
use crate::enemy_ai::get_dirc;
use crate::geom::{self, V4};
use crate::item;

type F = u32;

const ONE: F = 0x3f80_0000;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
const NEG_PI: F = 0xc049_0fdb;
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
/// `InitBossCamera(200, 1500)`: the camera's height and distance; 1500 is
/// also the reach of several of Gorre's camera shots.
pub const CAM_Z: F = 0x4348_0000;
pub const CAM_Y: F = 0x44bb_8000;

/// Gorre's acts (`actNum`, `Think`'s switch, OUT gcmn 0x004aa760, table
/// `@1462`); 9, 10, 17 and 19 are the table's own trap, never reached.
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
    /// each through its closing `-1`.
    pub normal: Vec<i32>,
    pub super_: Vec<i32>,
    pub epitaph: Vec<i32>,
    /// `Boss05AnmTbl`: the act's clip.
    pub anims: Vec<Option<String>>,
    /// `boss05SlaveAnmTbl1`, `boss05SlaveAnmTbl2`: each brother's own clip
    /// by act (`ccBoss05Brother::Init`, by `CheckSlaveID`).
    pub brother_anims: [Vec<Option<String>>; 2],
    /// `@1777`: `OnThinkSkill`'s four spells, one by `ccRand() & 3`.
    pub skills: [i32; 4],
    /// `@1672`: `OnThinkMagic`'s four, one by `abs(ccRand()) & 3`.
    pub magic_skills: [i32; 4],
    /// `@2074`: `SendMessage`'s msg 7, the Tornade's spell, by
    /// `ccRand() & 3`.
    pub tornade_skills: [i32; 4],
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
            tornade_skills: std::array::from_fn(|k| pick(t.gorre_tornade_skills(), k)),
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
/// `brothers` is scratch, filled only for the span of [`main`] (or of a
/// brother's `Affect`), the way Kyvia's `Tree` holds its core and gomoras.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gorre {
    pub brother_me: [usize; 2],
    pub brothers: [Part<Brother>; 2],
    /// `useBossCam` (+0x150): the boss camera on (`InitBossCamera`), until
    /// `OffBossCamera` as the three fall.
    pub cam_sw: bool,
    /// `m_transparency`: the Kerse's fade, the brothers' then Gorre's.
    pub fade: F,
    /// `m_talkPos`, `m_talkDirc[2]`: where the Kerse shows Gorre (500 off
    /// the target, facing the camera).
    pub talk_pos: V4,
    pub talk_dirc: F,
    /// `m_effAdrs`: the Kerse's FinalPhotonFlash.
    pub flash: Option<i32>,
    /// `m_skillID`: `OnThinkSkill`'s or `OnThinkMagic`'s spell.
    pub skill_id: i32,
    /// `m_effStart`: `effSkillStart`'s frames before its `endFlag`.
    pub skill_start: Option<i32>,
}

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

/// `this->br0->Order(on)` / `br1->Order(on)`.
fn order_brother(x: &mut Gorre, k: usize, cx: &mut Cx, on: i16) {
    brother::order(&mut x.brothers[k], cx, on);
}

/// Both brothers' `Order(on)`.
fn order_both(x: &mut Gorre, cx: &mut Cx, on: i16) {
    for k in 0..2 {
        order_brother(x, k, cx, on);
    }
}

/// A brother's own call (`EraseCmndTarget`, `EntryCmndTarget`): `cx.me`
/// its character for the span of `f`.
fn on_brother(x: &mut Gorre, k: usize, cx: &mut Cx, f: impl FnOnce(&mut Boss, &mut Cx)) {
    let old = std::mem::replace(&mut cx.me, x.brothers[k].me);
    f(&mut x.brothers[k].b, cx);
    cx.me = old;
}

/// Both brothers untargetable (`EraseCmndTarget`), the cursor off.
fn erase_brothers(x: &mut Gorre, cx: &mut Cx) {
    for k in 0..2 {
        on_brother(x, k, cx, |bb, cx| bb.erase_cmnd_target(cx));
    }
    cursor(cx, true);
}

/// Both brothers targetable again (`EntryCmndTarget`), the cursor back.
fn entry_brothers(x: &mut Gorre, cx: &mut Cx) {
    for k in 0..2 {
        on_brother(x, k, cx, |bb, cx| bb.entry_cmnd_target(cx));
    }
    cursor(cx, false);
}

/// `ccBoss05::IsValidArea(posP)` (OUT gcmn 0x004aa100): the world place
/// inside 2500 plus the centre's own x (y) of 0 either way.
fn valid_area(b: &Boss, cx: &Cx, pp: V4) -> bool {
    let w = super::p2w(cx, pp);
    let lim = |c: F| ee::sub(ee::add(0x453b_8000, c), 0x43fa_0000);
    ee::lt(w[0] & 0x7fff_ffff, lim(b.center_pos[0])) && ee::lt(w[1] & 0x7fff_ffff, lim(b.center_pos[1]))
}

fn stopped_or_held(b: &Boss, cx: &Cx) -> bool {
    b.stop != 0 || cx.scene.chars[cx.me].cond[crate::param::cond::HOLD] != 0
}

/// `ccBoss05::ChangeAction` (OUT gcmn 0x004aa6b0): the base's own, then
/// both brothers `Order`'d into the same act, taken or not (not from
/// [`new`]: its first `ChangeAction` comes before the brothers).
fn change_action(b: &mut Boss, x: &mut Gorre, cx: &mut Cx, act: i16, forbid: i16, af: bool) {
    b.change_action(cx, act, forbid, af);
    order_both(x, cx, act);
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

/// The camera shot several acts share (the Tornade, the Skill, the Magic,
/// the Dead): `off` turned by `d` (Rz(-pi/2), Rx, Ry, Rz) from `vp`, and
/// camera 3 there looking at `vp`.
fn focus_camera(cx: &mut Cx, vp: V4, d: V4, off: V4) {
    let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
    let m = geom::rot_matrix_x(&m, d[0]);
    let m = geom::rot_matrix_y(&m, d[1]);
    let m = geom::rot_matrix_z(&m, d[2]);
    let v = geom::apply_matrix(&m, off);
    let eye = super::p2w(cx, geom::vadd(super::w2p(cx, vp), v));
    cx.out(Out::CameraChange(3));
    cx.out(Out::CameraPos { cam: 3, pos: eye });
    cx.out(Out::CameraView { cam: 3, view: vp });
}

// --- the constructor ------------------------------------------------------------------------

/// `ccBoss05::ccBoss05` (OUT gcmn 0x004a9130): the boss is
/// `scene.chars[cx.me]` (`bossTbl` row 4), 500 behind Kite; two
/// `ccBoss05Brother`s made (rows 40 and 41, `BROTHER_OFFSET` either side,
/// Gorre's heading) and entered (`EntrySlave`); the Normal table's first
/// pattern.
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
    // No `ccEntryCmnd`: Gorre is never on the command lists (only its
    // brothers are), so nothing targets or affects it.
    b.change_action(cx, act::NEUTRAL, 0, true);
    b.center_pos = [center[0], center[1], center[2], ONE];
    b.center_pos_p = super::w2p(cx, b.center_pos);
    b.use_stage_eff = true;
    // `InitBossCamera(200, 1500)`; `MaxRenge` left at the camera's own
    // 1000 (Fidchell's constructor alone sets its 600 after).

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

    let mut x = Gorre { brother_me, cam_sw: true, ..Gorre::default() };
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
    let found: [bool; 2] = std::array::from_fn(|k| {
        let (part, found) = take_brother(cx, x.brother_me[k]);
        x.brothers[k] = part;
        found
    });
    // The effect manager's pass: one array for the three.
    b.effects.tick();
    for p in &mut x.brothers {
        p.b.effects.tick();
    }
    if b.exit == 0 {
        frame(b, &mut x, cx);
        // `Slave` (OUT gcmn 0x004ad980): each brother's own `Main`.
        for k in 0..2 {
            brother::frame(b, &mut x, k, cx);
        }
        cx.scene.chars[cx.me].cond[crate::param::cond::HOLD] = 0;
    }
    for (k, ok) in found.into_iter().enumerate() {
        put_brother(cx, std::mem::take(&mut x.brothers[k]), ok);
    }
    b.class = Class::Gorre(x);
}

/// `ccBoss05::Main` (OUT gcmn 0x004aa210) up to `Slave`: `CalcRealEx(0)`,
/// the centre's distance and heading, the stop's count, `hold` while a
/// brother's is set, `Think`, the party held while `lockPlayer` (a broken
/// protect's count on), `Move`, the count handed to both brothers.
fn frame(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
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
    // Held while either brother is.
    let held = x.brothers.iter().any(|p| cx.scene.chars[p.me].cond[crate::param::cond::HOLD] != 0);
    cx.scene.chars[me].cond[crate::param::cond::HOLD] = i16::from(held);
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
        if let Some(f) = cx.scene.chars[me].foe_state_mut()
            && f.pp_count > 0
        {
            f.pp_count += 1;
        }
    } else if b.reserve_forbid_menu == -1 && cx.env.menu_type == -1 {
        cx.out(Out::MenuForbid { on: false, chat_except: false });
        b.reserve_forbid_menu = 0;
        b.reserve_forbid_chat_except = 0;
    }
    b.move_(cx);
    // No `PreDrawAnm`: Gorre's own clip stands still but in the Kerse.
    // The broken protect's count is the three's.
    let count = cx.scene.chars[me].foe_state().map_or(0, |f| f.pp_count);
    for p in &x.brothers {
        if let Some(f) = cx.scene.chars[p.me].foe_state_mut() {
            f.pp_count = count;
        }
    }
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

/// `ccBoss05::ExecPatternIndex` (OUT gcmn 0x004a99a0), falling back on
/// `ccBoss::ExecPatternIndex` ([`Boss::base_exec_pattern_index`], whose act
/// numbers for patterns 2-8 and 12 already match Gorre's own `act`
/// module). Pattern 10 skips its word (the spell is `OnThinkSkill`'s own).
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
        10 => {
            s += 1;
            change_action(b, x, cx, act::SKILL, 3, true);
        }
        1 => {
            let n = if b.epitaph != 0 { act::EPITAPH_WAVE } else { act::WAVE };
            change_action(b, x, cx, n, 3, true);
        }
        9 | 11 => {
            if pat == 9 {
                let ty = word(tbl, s);
                s += 1;
                if ty < 0 {
                    // A member standing, by `abs(ccRand()) % n` (a first
                    // draw dropped).
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
            calc_target_info(b, cx);
            let sid = word(tbl, s);
            s += 1;
            // The second brother casts: on the target (9), or on the first
            // and on itself (11).
            let (br0, br1) = (x.brothers[0].me, x.brothers[1].me);
            let on = if pat == 11 { vec![br0, br1] } else { cx.scene.chars[me].target_char.into_iter().collect() };
            for t in on {
                if let Some(k) = item::item_skill_request(cx.t, cx.scene, br1, t, sid, 0, false, cx.rand) {
                    cx.out(Out::SkillFrom { user: br1, target: t, skill: k });
                }
            }
            exec_pattern(b, x, cx, 2, 60);
        }
        _ => {
            // The base's `ChangeAction` is Gorre's own through the vtable,
            // which orders both brothers too. Every pattern of the base
            // Gorre reaches changes the act (forbid 1 or 3, never
            // refused), leaving `actProccess` 0: -1 before tells.
            let proccess = std::mem::replace(&mut b.act_proccess, -1);
            let r = b.base_exec_pattern_index(cx, tbl, orig);
            if b.act_proccess == -1 {
                b.act_proccess = proccess;
            } else {
                order_both(x, cx, b.act_num);
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
        act::WANDER => on_wander(b, x, cx),
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

/// `OnThinkDrain` (OUT gcmn 0x004aaa40): drained, the Epitaph (act 12,
/// which marks `epitaph`) and its table, `m_patMode` 2.
fn on_drain(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    change_action(b, x, cx, act::EPITAPH, 3, true);
    b.set_pattern_tbl(Tbl::Epitaph);
    b.pat_mode = 2;
    let tbl = cx.data.gorre.words(Tbl::Epitaph).to_vec();
    b.pat_index = exec_pattern_index(b, x, cx, &tbl, 0);
}

/// `OnThinkWave` (OUT gcmn 0x004ad0c0) and `OnThinkEpitaphWave`
/// (0x004ad740): both brothers untargetable and without a body, then from
/// frame 115 the camera's quake and at 115 the flash and a wave shock at
/// each brother; the brothers' own [`Msg::WaveDone`] ends it. The plain
/// Wave runs a broken protect's count on.
fn on_wave(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let c = b.act_count;
    b.act_count += 1;
    if b.act_num == act::WAVE
        && let Some(f) = cx.scene.chars[cx.me].foe_state_mut()
        && f.pp_count > 0
    {
        f.pp_count += 1;
    }
    match b.act_proccess {
        0 => {
            for k in 0..2 {
                on_brother(x, k, cx, |bb, cx| bb.erase_cmnd_target(cx));
            }
            cursor(cx, true);
            for p in &mut x.brothers {
                p.b.body_hit_sw = 0;
            }
            b.act_proccess += 1;
        }
        1 if c >= 115 => {
            if cx.cam.shake {
                cx.quake_cam(0x4170_0000);
            }
            if c == 115 {
                cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
                let d = b.dirc;
                for k in 0..2 {
                    let bp = cx.scene.chars[x.brothers[k].me].pos;
                    b.effect_at(cx, EffKind::WaveShock, bp, d);
                }
            }
        }
        _ => {}
    }
}
/// `OnThinkTalk` (OUT gcmn 0x004acdf0): a member (type 2) held under the
/// cinema; at 45 its stream (71); once the menu is shut the second
/// brother's skill 20 on it and a flash.
fn on_talk(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    stream_attack(b, x, cx, 2, 71);
}

/// `OnThinkDataDrainAtk` (OUT gcmn 0x004ac060): as [`on_talk`] with a
/// target of type 0 and stream 72, then the drain (`EntryAffect` 13).
fn on_data_drain_atk(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    stream_attack(b, x, cx, 0, 72);
}

/// The Talk and the Data Drain: their shared steps, by their target type
/// and stream.
fn stream_attack(b: &mut Boss, x: &mut Gorre, cx: &mut Cx, ty: i32, stream: i32) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            let t = select(b, cx, ty);
            if !target_ok(cx, t) {
                return exec_pattern(b, x, cx, 2, -1);
            }
            calc_target_info(b, cx);
            b.lock_player(cx, false);
            erase_brothers(x, cx);
            cx.out(Out::Cinema(Some(-1)));
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 if c == 45 => {
            let id = cx.scene.chars[me].target_char.map_or(-1, |t| i32::from(cx.scene.chars[t].id()));
            let slot = cx.party.slot_of(id);
            cx.out(Out::StreamMenu { stream, mask: 1 << slot });
            b.act_proccess += 1;
            if stream == 71 {
                b.act_count = 0;
            }
        }
        2 if cx.env.menu_type == -1 => {
            let t = cx.scene.chars[me].target_char;
            if stream == 71 {
                on_brother(x, 1, cx, |bb, cx| {
                    bb.skill_damage(cx, t, None, 20);
                });
                cinema_off(cx);
                cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
            } else if let Some(t) = t.filter(|&t| cx.valid(Some(t))) {
                cx.affect(t, 13, 0);
            }
            b.unlock_player();
            entry_brothers(x, cx);
            if stream == 72 {
                cinema_off(cx);
            }
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkDashCenter` (OUT gcmn 0x004ac320): the base's dash at 50, the
/// brothers untargetable and held, a trail of both; at the dash's place
/// (or 121 frames) the next pattern.
fn on_dash_center(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let me = cx.me;
    b.spd_up(0x4248_0000, 0x4248_0000);
    match b.act_proccess {
        0 => {
            erase_brothers(x, cx);
            b.act_proccess += 1;
            cx.se3d(229);
        }
        1 => {
            if b.act_count & 3 == 3 {
                cx.out(Out::AfterImage);
                cx.out(Out::AfterImage);
                if b.act_count == 3 {
                    cx.se3d(57);
                }
            }
            if b.act_count >= 121 {
                b.act_proccess += 1;
                return;
            }
            for k in 0..2 {
                cx.affect(x.brothers[k].me, 5, 0);
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
            entry_brothers(x, cx);
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkChase` (OUT gcmn 0x004ad320): at a target (type 7) at 100, the
/// brothers turned with Gorre, untargetable; close (300) or 60 frames on,
/// the next pattern; out of the arena, a dash of 500 back.
fn on_chase(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 7);
            if !target_ok(cx, t) {
                return exec_pattern(b, x, cx, 2, -1);
            }
            calc_target_info(b, cx);
            b.move_spd = 0x42c8_0000;
            erase_brothers(x, cx);
            b.act_proccess += 1;
        }
        1 => {
            b.move_dirc = b.target_dirc;
            b.dirc[2] = b.target_dirc;
            for p in &mut x.brothers {
                p.b.dirc = b.dirc;
            }
            let c = b.act_count;
            b.act_count += 1;
            if c == 60 || ee::lt(b.target_dist, 0x4396_0000) {
                entry_brothers(x, cx);
                change_next_pattern(b, x, cx);
            } else if !valid_area(b, cx, cx.scene.chars[me].pos_p) {
                entry_brothers(x, cx);
                exec_pattern(b, x, cx, 6, 500);
            }
        }
        _ => {}
    }
}

/// `OnThinkEscape` (OUT gcmn 0x004aaf40): every 32 frames a way a quarter
/// turn off the target's, across the centre's (the centre's own out of the
/// arena); at 100 on a target, 30 after 60 frames, the next pattern at 300.
fn on_escape(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let me = cx.me;
    let quarter = 0x3fc9_0fdb;
    let c = b.act_count;
    b.act_count += 1;
    if c & 31 == 0 {
        let off = ee::sub(b.target_dirc, b.center_dirc) & 0x7fff_ffff;
        let r = if ee::lt(off, quarter) { ee::sub(b.target_dirc, quarter) } else { ee::add(quarter, b.target_dirc) };
        b.m_act_dirc[2] = if ee::le(r, PI) && ee::lt(r, NEG_PI) { ee::add(r, TWO_PI) } else { r };
    }
    if !valid_area(b, cx, cx.scene.chars[me].pos_p) {
        b.m_act_dirc[2] = b.center_dirc;
    }
    b.move_dirc = geom::set_dirc(b.move_dirc, b.m_act_dirc[2], 256);
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 0);
            if !target_ok(cx, t) {
                return exec_pattern(b, x, cx, 2, -1);
            }
            calc_target_info(b, cx);
            if !valid_area(b, cx, cx.scene.chars[me].pos_p) {
                b.move_dirc = b.center_dirc;
            }
            b.move_spd = 0x42c8_0000;
            b.act_proccess += 1;
        }
        1 => {
            if c >= 60 {
                b.move_spd = 0x41f0_0000;
            }
            if c == 300 {
                change_next_pattern(b, x, cx);
            } else if stopped_or_held(b, cx) {
                b.move_spd = 0;
            }
        }
        _ => {}
    }
}

/// `OnThinkWander` (OUT gcmn 0x004aba30): a way drawn (`ccRandF(pi)`),
/// then 120 frames at 30 turning to it (still while stopped or held); out
/// of the arena, a dash of 500 back.
fn on_wander(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            b.m_act_dirc[2] = crate::enemy_ai::rand_f(cx.cc, PI);
            b.act_proccess += 1;
        }
        1 => {
            b.move_spd = 0x41f0_0000;
            if !valid_area(b, cx, cx.scene.chars[me].pos_p) {
                return exec_pattern(b, x, cx, 6, 500);
            }
            if stopped_or_held(b, cx) {
                b.move_spd = 0;
            } else {
                b.move_dirc = geom::set_dirc(b.move_dirc, b.m_act_dirc[2], 256);
                b.dirc[2] = b.move_dirc;
            }
            let c = b.act_count;
            b.act_count += 1;
            if c == 120 {
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// The Kerse's fade a frame: a fifteenth (the brothers), a thirtieth
/// (Gorre).
const FADE_FAST: F = 0x3d88_8889;
const FADE_SLOW: F = 0x3d08_8889;

/// `OnThinkKerse` (OUT gcmn 0x004ab2e0), seven steps (`@1644`): a target
/// (type 11) under the cinema (19) and camera 3; the brothers fade out;
/// Gorre's own clip (the only time it runs) until it fades at frame 80 into
/// the FinalPhotonFlash; the second brother's skill 19 at the target; once
/// the flash is gone the brothers back, fading in; the stage fader out;
/// Gorre undrawn and neutral (forbid 3).
fn on_kerse(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let c = b.act_count;
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 11);
            if !target_ok(cx, t) {
                return exec_pattern(b, x, cx, 2, -1);
            }
            calc_target_info(b, cx);
            b.lock_player(cx, true);
            cx.out(Out::SwitchLayer);
            b.stage_eff_id = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 30000]);
            x.fade = ONE;
            b.transparency = ONE;
            b.set_transparency = ONE;
            erase_brothers(x, cx);
            cx.out(Out::Cinema(Some(19)));
            let mut eye = b.calc_effect_camera_pos(cx, b.target_pos, CAM_Y);
            eye[2] = 0x4248_0000;
            let mut vp = b.target_pos;
            vp[2] = 0x4348_0000;
            cx.out(Out::CameraChange(3));
            cx.out(Out::CameraPos { cam: 3, pos: eye });
            cx.out(Out::CameraView { cam: 3, view: vp });
            x.talk_pos = b.calc_effect_camera_pos(cx, b.target_pos, 0xc3fa_0000);
            x.talk_dirc = get_dirc(super::w2p(cx, x.talk_pos), super::w2p(cx, eye));
            b.act_proccess += 1;
        }
        1 => {
            let t = x.fade;
            for p in &mut x.brothers {
                p.b.transparency = t;
                p.b.set_transparency = t;
            }
            let t = ee::sub(t, FADE_FAST);
            if ee::lt(t, 0) {
                for p in &mut x.brothers {
                    p.b.draw_sw = 0;
                }
                x.fade = ONE;
                b.act_proccess += 1;
            } else {
                x.fade = t;
            }
        }
        2 => {
            match b.anm.frame() {
                75 => cx.out(Out::Se { se: 234 }),
                0 => {
                    cx.out(Out::SeNote { se: 191, note: 50 });
                    cx.out(Out::Se { se: 191 });
                }
                f if (f as u32) >= 80 => {
                    let mut t = ee::sub(x.fade, FADE_SLOW);
                    if ee::lt(t, 0) {
                        t = 0;
                        let tp = b.target_pos;
                        x.flash = Some(b.effect_at(cx, EffKind::FinalPhotonFlash, tp, VF0));
                        b.act_count = 0;
                        b.act_proccess += 1;
                    }
                    b.transparency = t;
                    b.set_transparency = t;
                    x.fade = t;
                }
                _ => {}
            }
            // Gorre's own clip runs here only (`PreDrawAnm` is never called
            // on Outbreak's Gorre), posed at `m_talkPos`.
            b.anm_status = i8::from(b.anm.forward());
        }
        3 => {
            b.act_count += 1;
            if c == 15 {
                let tp = b.target_pos;
                on_brother(x, 1, cx, |bb, cx| {
                    bb.skill_damage(cx, None, Some(tp), 19);
                });
            }
            if !b.effects.enabled(x.flash) {
                for p in &mut x.brothers {
                    p.b.draw_sw = 1;
                }
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        4 => {
            let mut t = ee::add(x.fade, FADE_FAST);
            if !ee::le(t, ONE) {
                t = ONE;
                b.act_proccess += 1;
            }
            x.fade = t;
            for p in &mut x.brothers {
                p.b.transparency = t;
                p.b.set_transparency = t;
            }
        }
        5 => {
            b.act_count += 1;
            if c == 15 {
                let id = std::mem::replace(&mut b.stage_eff_id, -1);
                b.end_stage_effect(cx, id, 0x8000_0000, 15);
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        6 => {
            b.act_count += 1;
            if c == 15 {
                entry_brothers(x, cx);
                cx.out(Out::SwitchLayer);
                cx.out(Out::CameraChange(2));
                b.unlock_player();
                b.draw_sw = 0;
                cinema_off(cx);
                change_action(b, x, cx, act::NEUTRAL, 3, true);
            }
        }
        _ => {}
    }
}

/// `OnThinkTornade` (OUT gcmn 0x004ac5d0): a target (type 3), the party
/// held, the brothers untargetable, the cinema (68) and camera 3 on the
/// target; the brothers' own Tornade does the rest ([`Msg::TornadeHidden`],
/// [`Msg::TornadeSpell`], [`Msg::TornadeDone`]).
fn on_tornade(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    if b.act_proccess != 0 {
        return;
    }
    let t = select(b, cx, 3);
    if !target_ok(cx, t) {
        return exec_pattern(b, x, cx, 2, -1);
    }
    calc_target_info(b, cx);
    b.lock_player(cx, true);
    for k in 0..2 {
        on_brother(x, k, cx, |bb, cx| bb.erase_cmnd_target(cx));
    }
    cursor(cx, true);
    cx.out(Out::Cinema(Some(68)));
    let mut vp = b.target_pos;
    vp[2] = ee::add(vp[2], 0x4348_0000);
    focus_camera(cx, vp, [0, 0, b.target_dirc, ONE], [CAM_Y, 0, 0, ONE]);
    b.act_count = 0;
    b.act_proccess += 1;
}

/// `OnThinkSkill` (OUT gcmn 0x004ac850): one of `@1777` (157 and 158 the
/// first brother's, 159 and 160 the second's) started under the stage
/// fader, camera 3 and the spell's cinema; once `effSkillStart` ends the
/// caster's `ccItemSkillCompel` on the target; 50 frames on the fader out;
/// 15 more, the next pattern.
fn on_skill(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let me = cx.me;
    b.set_dirc(b.target_dirc);
    let caster = |sid: i32| usize::from(!matches!(sid, 157 | 158));
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 0);
            if !target_ok(cx, t) {
                return exec_pattern(b, x, cx, 2, -1);
            }
            calc_target_info(b, cx);
            cx.out(Out::SwitchLayer);
            b.stage_eff_id = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 30000]);
            let k = (cx.cc.rand() & 3).unsigned_abs() as usize;
            x.skill_id = cx.data.gorre.skills[k];
            let who = x.brothers[usize::from(k >= 2)].me;
            cx.out(Out::SkillStart { who, sid: x.skill_id });
            x.skill_start = Some(super::fidchell::eff::SKILL_START_LIFE);
            let mut vp = b.target_pos;
            vp[2] = 0x4348_0000;
            focus_camera(cx, vp, [0, 0, b.target_dirc, ONE], [0x447a_0000, 0, 0, ONE]);
            erase_brothers(x, cx);
            b.lock_player(cx, true);
            cx.out(Out::CinemaSkill(x.skill_id));
            b.act_proccess += 1;
        }
        1 => {
            // The start's controller: its endFlag once its life is out.
            if let Some(n) = x.skill_start {
                x.skill_start = (n > 1).then_some(n - 1);
            }
            if x.skill_start.is_some() {
                return;
            }
            let sid = x.skill_id;
            let k = caster(sid);
            let target = cx.scene.chars[me].target_char;
            on_brother(x, k, cx, |bb, cx| {
                bb.entry_cmnd_target(cx);
                let user = cx.me;
                if let Some(t) = target
                    && let Some(s) = item::item_skill_compel(cx.t, cx.scene, user, t, sid, 0, false, cx.rand)
                {
                    cx.out(Out::SkillFrom { user, target: t, skill: s });
                }
                bb.erase_cmnd_target(cx);
            });
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 50 {
                let id = std::mem::replace(&mut b.stage_eff_id, -1);
                b.end_stage_effect(cx, id, 0x6400_0000, 15);
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        3 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 15 {
                cinema_off(cx);
                b.unlock_player();
                entry_brothers(x, cx);
                cx.out(Out::SwitchLayer);
                cx.out(Out::CameraChange(2));
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkMagic` (OUT gcmn 0x004abb90): one of `@1672` (by
/// `abs(ccRand()) & 3`) named in the cinema under the stage fader and
/// camera 3; at 15 the second brother casts it on the target; 60 on the
/// brothers back; 15 more, the fader out and the next pattern.
fn on_magic(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    match b.act_proccess {
        0 => {
            let t = select(b, cx, 0);
            if !target_ok(cx, t) {
                return exec_pattern(b, x, cx, 2, -1);
            }
            calc_target_info(b, cx);
            cx.out(Out::SwitchLayer);
            b.stage_eff_id = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 30000]);
            b.lock_player(cx, true);
            erase_brothers(x, cx);
            b.act_proccess += 1;
            let mut vp = b.target_pos;
            vp[2] = ee::add(vp[2], 0x4348_0000);
            focus_camera(cx, vp, [0, 0, b.target_dirc, ONE], [CAM_Y, 0, 0x42c8_0000, ONE]);
            let k = (cx.cc.rand().unsigned_abs() & 3) as usize;
            x.skill_id = cx.data.gorre.magic_skills[k];
            cx.out(Out::CinemaSkill(x.skill_id));
        }
        1 => {
            b.act_count += 1;
            if c == 15 {
                let br1 = x.brothers[1].me;
                if let Some(t) = cx.scene.chars[me].target_char
                    && let Some(s) = item::item_skill_request(cx.t, cx.scene, br1, t, x.skill_id, 0, false, cx.rand)
                {
                    cx.out(Out::SkillFrom { user: br1, target: t, skill: s });
                }
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        2 => {
            b.act_count += 1;
            if c == 60 {
                entry_brothers(x, cx);
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        3 => {
            b.act_count += 1;
            if c == 15 {
                cx.out(Out::SwitchLayer);
                cinema_off(cx);
                cx.out(Out::CameraChange(2));
                let id = std::mem::replace(&mut b.stage_eff_id, -1);
                b.end_stage_effect(cx, id, 0x6400_0000, 15);
                b.unlock_player();
                entry_brothers(x, cx);
                change_next_pattern(b, x, cx);
            }
        }
        _ => {}
    }
}

/// `OnThinkDead` (OUT gcmn 0x004aaaf0): no body, both brothers ordered
/// dead and set 300 either side of Gorre across the way to the centre,
/// camera 3 3000 off Gorre looking at it; their own [`Msg::Dead`] ends
/// the fight.
fn on_dead(b: &mut Boss, x: &mut Gorre, cx: &mut Cx) {
    if b.act_proccess != 0 {
        return;
    }
    b.body_hit_sw = 0;
    order_both(x, cx, brother::act::DEAD);
    let quarter = 0x3fc9_0fdb;
    let sides = [ee::add(quarter, b.center_dirc), ee::sub(b.center_dirc, quarter)];
    for (k, d) in sides.into_iter().map(brother::wrap_pi).enumerate() {
        let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), quarter), d);
        let off = geom::apply_matrix(&m, [BROTHER_OFFSET, 0, 0, ONE]);
        let p = &mut x.brothers[k];
        let master = cx.scene.chars[p.x.master_me].pos;
        cx.scene.chars[p.me].pos = geom::apply_matrix(&geom::trans_matrix(&geom::unit_matrix(), master), off);
        p.x.offset = off;
    }
    let mut vp = cx.pos();
    vp[2] = ee::add(vp[2], 0x43fa_0000);
    focus_camera(cx, vp, [0, 0, b.center_dirc, ONE], [0x453b_8000, 0, 0x42c8_0000, ONE]);
    b.move_spd = 0;
    b.move_vector = VF0;
    b.act_proccess += 1;
}

/// Where Gorre's own body is drawn, if anywhere: only in the Kerse's step
/// 2 (`SetMatrix_PosRotZYX(m_talkPos, m_talkDirc)`, `ccBoss::Draw`); its
/// `Main` draws nothing else.
pub fn body(b: &Boss) -> Option<(V4, V4)> {
    match &b.class {
        Class::Gorre(x) if b.act_num == act::KERSE && b.act_proccess == 2 => {
            Some((x.talk_pos, [0, 0, x.talk_dirc, ONE]))
        }
        _ => None,
    }
}

// --- Affect ----------------------------------------------------------------------------------

/// `ccBoss05::Affect` (OUT gcmn 0x004aa200): empty (and Gorre is never
/// listed, so nothing reaches it); the fight is won through its brothers.
pub(super) fn affect(_b: &mut Boss, _cx: &mut Cx) {}

/// `ccBoss05Brother::Affect`, for a `Class::GorreBrother` character.
pub(super) fn brother_affect(b: &mut Boss, cx: &mut Cx) {
    brother::affect_alone(b, cx);
}
