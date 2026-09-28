//! Magus (`ccBoss03`, boss03.cpp) and its twelve leaves (`ccBoss03Leaf`,
//! [`leaf`]), as Mutation's gcmn.prg has them (0x0049da20-0x004a7974; names
//! and layouts from Infection's DWARF). `ccBossEntryStart(2)` starts
//! `ccThBoss03`, which news the boss ([`new`]) and runs [`main`] each frame;
//! Mutation's event 115 fights it in the arena of field 3. Each leaf is a
//! `ccBoss` of its own character, taken out of it for the body's frame.
//! docs/engine/boss-magus.md.

pub mod leaf;
#[cfg(test)]
mod tests;

use piney_data::field::ee;
use piney_data::libm;

use self::leaf::Leaf;
use super::kyvia::fabs;
use super::{Anm, Boss, Class, Cx, EffKind, Fade, Out, VF0};
use crate::enemy_ai::{get_dirc, get_dist, rand_f};
use crate::geom::{self, V4};
use crate::item;
use crate::param::{SkillParam, cond};

type F = u32;

const ONE: F = 0x3f80_0000;
const PI: F = 0x4049_0fdb;
const NEG_PI: F = 0xc049_0fdb;
const TWO_PI: F = 0x40c9_0fdb;
const HALF_PI: F = 0x3fc9_0fdb;
const NEG_HALF_PI: F = 0xbfc9_0fdb;
/// `SelectTarget`'s radius here: 999999.
const FAR: F = 0x4974_23f0;

/// `bossFunc`'s code for Magus, its `bossTbl` row and the leaves'.
pub const CODE: i32 = 2;
pub const ROW: usize = 2;
pub const LEAF_ROW: usize = 11;
/// `slaveNum[0]`: the leaves.
pub const LEAVES: usize = 12;
/// `bossBlur`'s colours: the constructor's and the rise's.
pub const BLUR: u32 = 0x4880_8080;
pub const BLUR_RISE: u32 = 0x6080_8080;

/// Magus's acts (`actNum`, `Think`'s switch at 0x0049fa50).
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DMG0: i16 = 1;
    pub const DMG1: i16 = 2;
    pub const WAVE: i16 = 4;
    pub const DRAIN_ATK: i16 = 5;
    pub const DASH: i16 = 7;
    /// Mutation's: back toward the centre after a stall (0x004a2ec0).
    pub const TO_CENTRE: i16 = 8;
    /// Mutation's: skill 250 under camera 3 (0x004a1c20).
    pub const CAST: i16 = 9;
    pub const DATA_DRAIN: i16 = 11;
    pub const EPITAPH: i16 = 12;
    pub const EPITAPH_WAVE: i16 = 13;
    pub const DIE: i16 = 14;
    pub const ESCAPE: i16 = 16;
    pub const CHASE: i16 = 18;
    pub const LEAF_DROP: i16 = 20;
    pub const WANDER: i16 = 21;
    pub const STOP: i16 = 22;
    pub const LEAF_COUNT_DOWN: i16 = 23;
    pub const LASER: i16 = 25;
    pub const LEAF_GROW: i16 = 26;
    pub const RISE: i16 = 27;
    pub const FALL: i16 = 28;
    pub const RETURN: i16 = 29;
    pub const DESTROY_LEAF: i16 = 30;
    pub const NEEDLE: i16 = 31;
}

/// Magus's tables (`tables::combat`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MagusData {
    /// `boss03EpitaphActTbl`: the patterns once drained, to its -1.
    pub epitaph: Vec<i32>,
    /// `Boss03AnmTbl`, `Boss03SlaveAnmTbl`: the clip by act.
    pub anims: Vec<Option<String>>,
    pub leaf_anims: Vec<Option<String>>,
    /// `@1538` (pattern 10's), `@2301` (the sixth leaf's).
    pub skills: [i32; 3],
    pub drop_skills: [i32; 3],
}

impl MagusData {
    /// The volume's.
    pub fn of(volume: piney_data::volume::Volume) -> MagusData {
        let t = piney_data::tables::combat::of(volume);
        let s = |v: &[Option<&str>]| v.iter().map(|a| a.map(str::to_string)).collect::<Vec<_>>();
        let three = |v: &[i32]| std::array::from_fn(|k| v.get(k).copied().unwrap_or(0));
        MagusData {
            epitaph: t.magus_epitaph().to_vec(),
            anims: s(t.magus_anims()),
            leaf_anims: s(t.magus_leaf_anims()),
            skills: three(t.magus_skills()),
            drop_skills: three(t.magus_drop_skills()),
        }
    }
}

/// `ccBoss03LaserShock` (0x40 bytes) as the rules keep it: the index
/// `SetupLaserShock` gave it, its fade and its count of frames its beam
/// met the ground.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shock {
    pub index: i32,
    pub transparency: F,
    pub draw: i32,
    pub act_count: i32,
}

/// What Magus shows that the rules do not keep, for the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pic {
    /// `ccBossEffThunderCreate` at laser shock `shock`'s object
    /// (`OBJ_ex31lesNN` of the body, NN five times the shock).
    Thunder { shock: usize },
    /// Shocks 0 to `upto` drawn this frame (`ccBoss03LaserShock::Draw`).
    Shocks { upto: usize },
    /// A shock's burst where its beam met the ground: two
    /// `ccParticleExplode` and `effSmokeRock`.
    ShockBurst { shock: usize },
    /// `EntryAfterImageAnm` of leaf `leaf` (its blink).
    LeafAfterImage { leaf: usize },
    /// `ccEnemyEffDust(pos, 30, 4.0)`: a leaf lands.
    Dust { pos: V4 },
    /// `g_leafDeadGen` at a leaf that dies.
    LeafDead { pos: V4 },
    /// A leaf's `g_energyChargeGenerator` started or killed.
    Charge { leaf: usize, on: bool },
}

/// `ccBoss03`'s own members (+0x29350 on, Mutation's offsets: four more
/// words at +0x52720, the rest from `m_blurRad` on 16 bytes later).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Magus {
    /// `m_anmw`: `xeffect`'s `ANM_xx11wave`.
    pub anm_w: Anm,
    pub eff_dead: Option<i32>,
    pub animate_chase: Fade,
    pub act_cnt: i32,
    pub is_neutral_anm: i32,
    pub atk_wait: i32,
    pub prev_rise: F,
    pub laser_flash: i32,
    pub grow_interval: u32,
    pub epitaph: i32,
    /// `m_camID`: 3 while `CalcCamera` flies camera 3.
    pub cam_id: i32,
    pub shock_idx: i32,
    pub prepare_explode_all: i32,
    pub dropping: i32,
    pub reserve_laser: i32,
    pub explosion: i32,
    pub dmg_count: i32,
    pub high_drive: i32,
    pub reserve_leaf_drop: i32,
    pub explode_leaf_num: i32,
    pub laser_count: i32,
    /// `m_leafGrowSE` is a voice (not -1): the grow's loop plays.
    pub grow_se: bool,
    /// +0x52720: set when Magus is held or strays in the leaf drop; the
    /// leaves' countdowns wait on it and act 8 clears it.
    pub stuck: i32,
    /// +0x52724: the frames in a row `Move` was pushed out of something.
    pub pushed: i32,
    /// +0x52728: act 9's stage effect (-1 none).
    pub stage_id: i32,
    /// +0x5272c: every leaf fell and went; `OnThinkNeutral` regrows them.
    pub regrow: i32,
    pub blur_rad: F,
    pub blur_scale: F,
    pub shocks: [Shock; 12],
    pub no_leaf: i32,
    pub cam_count: i32,
    pub prev_laser: V4,
    pub prev_dirc: V4,
    /// `actPos` (+0x1b0, a `ccBoss` member only Magus uses).
    pub act_pos: V4,
    /// `bossBlur`'s colour (+0x1c).
    pub blur: u32,
    /// `slaveCtrl[0]`: the leaves' characters, by slave.
    pub leaves: Vec<usize>,
    /// Where each leaf hangs on the body (`OBJ_ex31leafNN`'s world place as
    /// last posed), which `Init` reads; the runtime sets it each frame.
    pub leaf_pos: [V4; LEAVES],
    /// Whether each shock's beam meets the ground (`ccHitCheckLM` not -1);
    /// the runtime sets it.
    pub shock_hit: [bool; LEAVES],
}

/// The leaves out of their characters for the body's frame.
#[derive(Debug, Default)]
pub struct Leaves(pub Vec<LeafPart>);

/// A leaf as the body's frame holds it.
#[derive(Debug)]
pub struct LeafPart {
    pub b: Box<Boss>,
    pub x: Box<Leaf>,
    pub me: usize,
}

impl Leaves {
    /// The leaves taken out of their characters.
    pub fn take(cx: &mut Cx, leaves: &[usize]) -> Leaves {
        let mut out = Vec::new();
        for &me in leaves {
            let Some(mut b) = cx.scene.chars.get_mut(me).and_then(|c| c.foe_state_mut()).and_then(|f| f.boss.take())
            else {
                continue;
            };
            match std::mem::replace(&mut b.class, Class::Plain) {
                Class::MagusLeaf(x) => out.push(LeafPart { b, x, me }),
                other => {
                    b.class = other;
                    if let Some(f) = cx.scene.chars[me].foe_state_mut() {
                        f.boss = Some(b);
                    }
                }
            }
        }
        Leaves(out)
    }

    /// Everything put back.
    pub fn put(self, cx: &mut Cx) {
        for mut p in self.0 {
            p.b.class = Class::MagusLeaf(p.x);
            if let Some(f) = cx.scene.chars[p.me].foe_state_mut() {
                f.boss = Some(p.b);
            }
        }
    }

    /// How many leaves have fallen (`m_bDrop`).
    fn dropped(&self) -> usize {
        self.0.iter().filter(|p| p.x.drop != 0).count()
    }
}

// --- small rules ------------------------------------------------------------------------

/// A table word.
fn word(tbl: &[i32], i: i32) -> i32 {
    usize::try_from(i).ok().and_then(|k| tbl.get(k)).copied().unwrap_or(0)
}

/// `ccRandF()` (main 0x001ef330): `ccRand()` over 2^32.
fn rand_unit(cx: &mut Cx) -> F {
    ee::div(ee::from_int(cx.cc.rand()), 0x4f80_0000)
}

/// An angle into -pi..pi: above pi a turn off, else below -pi a turn on.
fn wrap_hi(v: F) -> F {
    if !ee::le(v, PI) {
        ee::sub(v, TWO_PI)
    } else if ee::lt(v, NEG_PI) {
        ee::add(v, TWO_PI)
    } else {
        v
    }
}

fn magus_of(b: &mut Boss) -> Box<Magus> {
    match std::mem::replace(&mut b.class, Class::Plain) {
        Class::Magus(m) => m,
        other => {
            b.class = other;
            Box::default()
        }
    }
}

/// `CheckEpitaph()` (Mutation's 0x0049e9c0): `m_bEpitaph`.
fn epitaph(x: &Magus) -> bool {
    x.epitaph != 0
}

fn pp_of(cx: &Cx) -> i16 {
    cx.scene.chars[cx.me].foe_state().map_or(0, |f| f.pp)
}

/// `GetMaxPP()` (0x00475540): `bossTbl[base->id].maxPP`.
fn max_pp_of(cx: &Cx) -> i16 {
    let id = usize::try_from(cx.scene.chars[cx.me].id()).unwrap_or(0);
    cx.t.bosses.get(id).map_or(0, |r| r.max_pp)
}

fn held(b: &Boss, cx: &Cx) -> bool {
    b.stop != 0 || cx.scene.chars[cx.me].cond[cond::HOLD] != 0
}

fn cursor(cx: &mut Cx, on: bool) {
    cx.out(Out::CursorOff(on));
}

// --- the constructor ------------------------------------------------------------------------

/// `ccBoss03::ccBoss03` (0x0049db40) with its twelve leaves
/// (`ccBoss03Leaf::ccBoss03Leaf`, `SetMaster(this, k)`), whose characters
/// it adds to the scene. The boss is `scene.chars[cx.me]` (`bossTbl` row
/// 2); it stands 500 behind Kite. `center` is the arena's `DMY_center01`.
pub fn new(cx: &mut Cx, kite_pos: V4, kite_dirc: V4, center: V4) -> Boss {
    let me = cx.me;
    let mut b = super::kyvia::plain_boss();
    b.anm_tbl = cx.data.magus.anims.clone();
    // OffExit, OnDraw, OnBodyHit, OnCheatHP.
    b.exit = 0;
    b.draw_sw = 1;
    b.body_hit_sw = 1;
    b.cheat_hp = 1;
    let mut pos = kite_pos;
    pos[1] = ee::sub(pos[1], 0x43fa_0000);
    cx.scene.chars[me].pos = pos;
    b.dirc = kite_dirc;
    let mut x = Magus {
        anm_w: Anm::new(),
        cam_id: 1,
        laser_flash: -1,
        dmg_count: 100,
        stage_id: -1,
        animate_chase: Fade { transparency: ONE, ..Fade::default() },
        shocks: [Shock { transparency: ONE, ..Shock::default() }; 12],
        shock_hit: [true; LEAVES],
        ..Magus::default()
    };
    x.atk_wait = (cx.cc.rand() & 31).abs();
    for k in 0..LEAVES {
        let lme = leaf::new(cx, k as i32);
        x.leaves.push(lme);
    }
    crate::fellow::entry_cmnd(cx.scene, me);
    change_action(&mut b, &mut x, cx, act::NEUTRAL, 3, true);
    // InitCenterPos; InitStageEffect (the stage fader).
    b.center_pos = [center[0], center[1], center[2], ONE];
    b.center_pos_p = super::w2p(cx, b.center_pos);
    b.use_stage_eff = true;
    // The blur (the manager's +4): on, never ending, 1.01.
    x.blur = BLUR;
    cx.out(Out::Blur { colour: x.blur });
    x.blur_rad = 0;
    x.blur_scale = 0x3f80_a3d7;
    // SetPatternTbl(boss03EpitaphActTbl): its index 0.
    b.pat_index = 0;
    x.reserve_leaf_drop = 1;
    b.class = Class::Magus(Box::new(x));
    b
}

// --- the frame --------------------------------------------------------------------------------

/// `ccThBossEffect`'s pass and `ccBoss03::Main` (0x0049f580).
pub(super) fn main(b: &mut Boss, cx: &mut Cx) {
    let mut x = magus_of(b);
    let mut ls = Leaves::take(cx, &x.leaves);
    manager_pass(b, cx);
    if matches!(b.act_num, act::RETURN | act::WANDER | act::STOP | act::EPITAPH | act::NEUTRAL) {
        x.atk_wait -= 1;
        if x.atk_wait < 0 {
            x.atk_wait = 0;
        }
    }
    if b.exit == 0 {
        frame(b, &mut x, &mut ls, cx);
    }
    calc_camera(b, &mut x, cx);
    let me = cx.me;
    cx.scene.chars[me].cond[cond::HOLD] = 0;
    for p in &ls.0 {
        cx.scene.chars[p.me].cond[cond::HOLD] = 0;
    }
    if b.lock_player != 0
        && let Some(f) = cx.scene.chars[me].foe_state_mut()
        && f.pp_count > 0
    {
        f.pp_count += 1;
    }
    ls.put(cx);
    b.class = Class::Magus(x);
}

/// `ccBoss::Main` (0x00471c10) with Magus's `Think`, `Move` and
/// `PreDrawAnm`, then `Slave()`: each leaf not exited drawn and run.
fn frame(b: &mut Boss, x: &mut Magus, ls: &mut Leaves, cx: &mut Cx) {
    let me = cx.me;
    crate::chara::calc_real(cx.t, &mut cx.scene.chars[me], 0, cx.env, &mut cx.ev);
    b.calc_target_info(cx);
    b.center_pos_p = super::w2p(cx, b.center_pos);
    let pp = cx.scene.chars[me].pos_p;
    b.center_dist = get_dist(pp, b.center_pos_p);
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
    think(b, x, ls, cx);
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
    mov(b, x, cx);
    if b.draw_sw != 0 {
        pre_draw_anm(b, x, cx);
    }
    for k in 0..ls.0.len() {
        if ls.0[k].b.exit == 0 {
            ls.0[k].b.draw_sw = 1;
            leaf::main(ls, k, b, x, cx);
        }
    }
    cx.scene.chars[me].cond[cond::HOLD] = 0;
}

/// Magus's `Move` (Mutation's, 0x0049f400): `ccBoss::Move`, counting the
/// frames in a row the body was pushed out of something.
fn mov(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    let me = cx.me;
    let mut pp = super::w2p(cx, cx.scene.chars[me].pos);
    let spd = b.move_spd;
    if !geom::eq(0, spd) {
        pp[0] = ee::add(pp[0], ee::mul(spd, libm::sinf(b.move_dirc)));
        pp[1] = ee::sub(pp[1], ee::mul(spd, libm::cosf(b.move_dirc)));
    }
    pp = geom::vadd(pp, b.move_vector);
    let mut pos = super::p2w(cx, pp);
    if b.body_hit_sw == 0 {
        x.pushed = 0;
    } else if let Some(mut off) = (cx.collide)(pos) {
        off[2] = 0;
        pos = geom::vadd(pos, off);
        pp = geom::vadd(pp, off);
        x.pushed += 1;
    } else {
        x.pushed = 0;
    }
    let ch = &mut cx.scene.chars[me];
    ch.pos_p = pp;
    ch.pos = pos;
}

/// `ccBoss03::PreDrawAnm` (0x004a08c0): the clip forward (not in the
/// laser's second step), and in its third the shocks' beams by the clip's
/// frame.
fn pre_draw_anm(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    if b.act_num != act::LASER || b.act_proccess != 1 {
        b.anm_status = i8::from(b.anm.forward());
    }
    if b.act_num == act::LASER && b.act_proccess == 2 && b.anm.frame() >= 100 {
        let n = (((b.anm.frame() - 100) as u32 / 15) * 2).min(11) as usize;
        cx.out(Out::Magus(Pic::Shocks { upto: n }));
        for k in 0..=n {
            shock_draw(x, k, cx);
        }
    }
}

/// `ccBoss03LaserShock::Draw` (0x004a7630) as far as the rules go: while
/// its beam meets the ground, a fade, and every sixth frame for shocks 0,
/// 3, 6, 9 a burst with one `ccRandF(pi)`.
fn shock_draw(x: &mut Magus, k: usize, cx: &mut Cx) {
    let mut done = 0;
    if x.shock_hit[k] {
        let s = &mut x.shocks[k];
        s.transparency = ee::sub(s.transparency, 0x3c23_d70a);
        if ee::lt(s.transparency, 0) {
            done = 1;
            s.transparency = 0;
        }
        let c = s.act_count;
        s.act_count = c + 1;
        if c == 5 && s.index % 3 == 0 {
            let _ = rand_f(cx.cc, PI);
            cx.out(Out::Magus(Pic::ShockBurst { shock: k }));
            x.shocks[k].act_count = 0;
        }
    }
    x.shocks[k].draw = done;
}

/// `ccBossEffManager::Draw`'s pass over Magus's effects: a needle steps
/// its own life ([`Needle`]), the others theirs.
fn manager_pass(b: &mut Boss, cx: &mut Cx) {
    for k in 0..b.effects.slots.len() {
        let Some(e) = b.effects.slots[k].as_mut() else { continue };
        if !e.enabled {
            b.effects.slots[k] = None;
            continue;
        }
        let EffKind::Needle { .. } = e.kind else {
            e.draw();
            continue;
        };
        let Some(n) = e.needle.as_mut() else { continue };
        if let Some(se) = n.draw() {
            cx.out(Out::Se3d { se, pos: n.pos });
        }
        if n.done {
            e.enabled = false;
        }
    }
}

/// `ccBossEffNeedle` (MUT gcmn 0x00481180, `Draw` 0x004814d0): its
/// needles grow in, hold, and fade; three `ccRandF(pi)` a needle when made.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Needle {
    pub pos: V4,
    pub scale: F,
    pub scale_spd: F,
    pub alpha: F,
    pub alpha_spd: F,
    pub hold: F,
    pub fade_spd: F,
    pub state: i32,
    pub done: bool,
}

impl Needle {
    /// `ccBossEffNeedleCreate(pos, scale, speed, alpha, n, t0, t1, t2)`:
    /// `n` needles turned at random.
    #[allow(clippy::too_many_arguments)]
    fn new(cx: &mut Cx, pos: V4, scale: F, speed: F, alpha: F, n: i32, t0: i32, t1: i32) -> Needle {
        for _ in 0..n {
            for _ in 0..3 {
                let _ = rand_f(cx.cc, PI);
            }
        }
        Needle {
            pos,
            scale,
            scale_spd: speed,
            alpha,
            alpha_spd: ee::div(ee::sub(ONE, alpha), ee::from_int(t0)),
            hold: ee::from_int(t1),
            fade_spd: ee::div(0xbf80_0000, ee::from_int(t1)),
            state: 0,
            done: false,
        }
    }

    /// One `Draw`: the sound it plays, if any.
    fn draw(&mut self) -> Option<i32> {
        let mut se = None;
        let mut hold = ee::to_int(self.hold) as i16;
        let mut a = self.alpha;
        match self.state {
            0 => {
                self.scale = ee::add(self.scale, self.scale_spd);
                a = ee::add(a, self.alpha_spd);
                if !ee::lt(a, ONE) {
                    se = Some(231);
                    a = ONE;
                    self.state += 1;
                }
            }
            1 => {
                hold -= 1;
                if hold <= 0 {
                    self.state += 1;
                }
            }
            2 => {
                self.scale = ee::sub(self.scale, self.scale_spd);
                a = ee::add(a, self.fade_spd);
                if ee::le(a, 0) {
                    self.done = true;
                }
            }
            _ => {}
        }
        self.alpha = a;
        self.hold = ee::from_int(i32::from(hold));
        se
    }
}

// --- ChangeAction and the patterns ----------------------------------------------------------------

/// `ccBoss03::ChangeAction(n, forbid, af)` (0x004a0110): the clip set
/// unless the laser's (it sets its own), and the neutral clip noted.
fn change_action(b: &mut Boss, x: &mut Magus, cx: &mut Cx, n: i16, forbid: i16, af: bool) {
    let ok = match forbid {
        0 => b.act_forbid == 0,
        1 => {
            b.act_forbid = 0;
            true
        }
        2 | 3 => {
            b.act_forbid = forbid as i8;
            true
        }
        _ => false,
    };
    if !ok {
        return;
    }
    b.act_num = n;
    b.act_proccess = 0;
    b.act_count = 0;
    if !af || n == act::LASER {
        return;
    }
    let Some(Some(clip)) = usize::try_from(n).ok().and_then(|k| b.anm_tbl.get(k)).cloned() else { return };
    b.anm.set(&clip, cx.clips);
    // SetupLeafObjPtr unless drained: the body's leaves found anew (every
    // clip of Magus's has them).
    if n == act::NEUTRAL {
        x.is_neutral_anm = 1;
    } else if n != act::EPITAPH {
        x.is_neutral_anm = 0;
    }
}

/// `ccBoss::ChangeNextPattern` (0x00472bb0) as Magus's vtable runs it.
fn change_next_pattern(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    let tbl = cx.data.magus.epitaph.clone();
    if word(&tbl, b.pat_index) == -1 {
        b.pat_index = 0;
    }
    b.move_spd = 0;
    b.move_vector = VF0;
    if epitaph(x) {
        change_action(b, x, cx, act::EPITAPH, 3, true);
    } else {
        change_action(b, x, cx, act::NEUTRAL, 1, true);
    }
    b.pat_index = exec_pattern_index(b, x, cx, &tbl, b.pat_index);
}

/// `ccBoss::ExecPattern(pat, p0, 0, 0)` (0x00472b40).
fn exec_pattern(b: &mut Boss, x: &mut Magus, cx: &mut Cx, pat: i32, p0: i32) {
    let tbl = [pat, p0, 0, 0, -1];
    exec_pattern_index(b, x, cx, &tbl, 0);
}

fn target_ok(cx: &Cx, t: Option<usize>) -> bool {
    cx.valid(t) && t.is_some_and(|t| !cx.dead(t))
}

/// `ccBoss03::ExecPatternIndex(tbl, i)` (0x0049fd30), falling back on
/// `ccBoss::ExecPatternIndex` (0x00472470): the next index. Its words 9
/// and 10 read their target type from `patTbl`, not `tbl`.
fn exec_pattern_index(b: &mut Boss, x: &mut Magus, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
    let mut orig = i;
    let mut s0 = i;
    let mut pat = word(tbl, i);
    s0 += 1;
    if pat == -1 {
        s0 = 1;
        orig = 1;
        pat = word(tbl, 0);
    }
    b.pat_num = pat;
    let me = cx.me;
    match pat {
        14 => change_action(b, x, cx, act::CAST, 3, true),
        2 => {
            b.stop_time = word(tbl, s0);
            s0 += 1;
            b.stop_counter = 0;
            change_action(b, x, cx, act::EPITAPH, 3, true);
        }
        1 => {
            let n = if epitaph(x) { act::EPITAPH_WAVE } else { act::WAVE };
            change_action(b, x, cx, n, 3, true);
        }
        9..=11 => {
            let pat_tbl = cx.data.magus.epitaph.clone();
            if pat != 11 {
                let ty = word(&pat_tbl, s0);
                s0 += 1;
                let t = b.select_target(cx, ty, FAR);
                cx.scene.chars[me].target_char = t;
            }
            if !target_ok(cx, cx.scene.chars[me].target_char) {
                let t = b.select_target(cx, 3, FAR);
                cx.scene.chars[me].target_char = t;
                if !target_ok(cx, t) {
                    exec_pattern(b, x, cx, 2, 30);
                    return orig;
                }
            }
            b.calc_target_info(cx);
            let sid = if pat == 10 {
                let k = (cx.cc.rand() % 3).unsigned_abs() as usize;
                cx.data.magus.skills[k]
            } else {
                let s = word(&pat_tbl, s0);
                s0 += 1;
                s
            };
            if let Some(tp) = cx.scene.chars[me].target_char
                && let Some(s) = item::item_skill_compel(cx.t, cx.scene, me, tp, sid, 1, false, cx.rand)
            {
                cx.out(Out::Skill(tp, s));
            }
            exec_pattern(b, x, cx, 2, -2);
        }
        _ => return base_exec_pattern_index(b, x, cx, tbl, orig),
    }
    s0
}

/// `ccBoss::ExecPatternIndex` (0x00472470) under Magus's `ChangeAction`
/// and `CheckEpitaph`.
fn base_exec_pattern_index(b: &mut Boss, x: &mut Magus, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
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
    let neutral = |b: &mut Boss, x: &mut Magus, cx: &mut Cx, forbid: i16| {
        if epitaph(x) {
            change_action(b, x, cx, act::EPITAPH, 3, true);
        } else {
            change_action(b, x, cx, act::NEUTRAL, forbid, true);
        }
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
            neutral(b, x, cx, 3);
        }
        3 => change_action(b, x, cx, super::act::ESCAPE, 3, true),
        4 => change_action(b, x, cx, super::act::CHASE, 3, true),
        5 => change_action(b, x, cx, super::act::RETURN, 3, true),
        7 => change_action(b, x, cx, super::act::WANDER, 3, true),
        6 => {
            let d = ee::from_int(word(tbl, i));
            i += 1;
            let m = geom::rot_matrix_z(&geom::rot_matrix_z(&geom::unit_matrix(), b.center_dirc), NEG_HALF_PI);
            let v = geom::apply_matrix(&m, [d, 0, 0, 0]);
            let p = geom::vadd(v, cx.scene.chars[me].pos_p);
            b.dash_pos = super::p2w(cx, p);
            b.dash_prev_pos = cx.scene.chars[me].pos;
            change_action(b, x, cx, super::act::DASH, 3, true);
        }
        8 => neutral(b, x, cx, 3),
        9 | 11 => {
            let ty = word(tbl, i);
            i += 1;
            let sid = word(tbl, i);
            i += 1;
            if pat == 9 {
                let t = b.select_target(cx, ty, FAR);
                cx.scene.chars[me].target_char = t;
                if !target_ok(cx, t) {
                    exec_pattern(b, x, cx, 2, 30);
                    return orig;
                }
                b.calc_target_info(cx);
            }
            let on = if pat == 9 { cx.scene.chars[me].target_char } else { Some(me) };
            if let Some(tp) = on
                && let Some(s) = item::item_skill_request(cx.t, cx.scene, me, tp, sid, 0, false, cx.rand)
            {
                cx.out(Out::Skill(tp, s));
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
                    && let Some(s) = item::item_skill_request(cx.t, cx.scene, me, m, sid, 0, false, cx.rand)
                {
                    cx.out(Out::Skill(m, s));
                }
            }
            if let Some(l) = last
                && target_ok(cx, Some(l))
                && let Some(s) = item::item_skill_request(cx.t, cx.scene, me, l, sid, 1, false, cx.rand)
            {
                cx.out(Out::Skill(l, s));
            }
            exec_pattern(b, x, cx, 2, -2);
        }
        _ => neutral(b, x, cx, 1),
    }
    i
}

// --- Think ----------------------------------------------------------------------------------

/// `ccBoss03::Think` (0x0049fa50).
fn think(b: &mut Boss, x: &mut Magus, ls: &mut Leaves, cx: &mut Cx) {
    match b.act_num {
        act::NEUTRAL => on_neutral(b, x, ls, cx),
        act::WAVE => on_wave(b, x, cx),
        act::DRAIN_ATK => on_drain_atk(b, x, cx),
        act::DASH => on_dash(b, x, cx),
        act::TO_CENTRE => on_to_centre(b, x, cx),
        act::CAST => on_cast(b, x, cx),
        act::DATA_DRAIN => on_data_drain(b, x, ls, cx),
        act::EPITAPH => on_epitaph(b, x, cx),
        act::EPITAPH_WAVE => on_epitaph_wave(b, x, cx),
        act::DIE => on_die(b, x, cx),
        act::ESCAPE => on_escape(b, x, cx),
        act::CHASE => on_chase(b, x, cx),
        act::LEAF_DROP => on_leaf_drop(b, x, ls, cx),
        act::WANDER => on_wander(b, x, cx),
        act::STOP => on_stop(b, x, cx),
        act::LEAF_COUNT_DOWN => on_leaf_count_down(b, x, ls, cx),
        act::LASER => on_laser(b, x, cx),
        act::LEAF_GROW => on_leaf_grow(b, x, ls, cx),
        act::RISE => on_rise(b, x, ls, cx),
        act::FALL => on_fall(b, x, ls, cx),
        act::RETURN => on_return(b, x, cx),
        act::DESTROY_LEAF => on_destroy_leaf(b, x, ls, cx),
        act::NEEDLE => on_needle(b, x, cx),
        _ => {}
    }
}

/// `OnThinkNeutral` (0x004a0d50): the next attack by the reserves' flags.
fn on_neutral(b: &mut Boss, x: &mut Magus, ls: &mut Leaves, cx: &mut Cx) {
    let me = cx.me;
    b.move_spd = 0;
    b.move_vector = VF0;
    b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 256);
    let c = b.act_count;
    b.act_count += 1;
    if c == 0 {
        x.atk_wait = (cx.cc.rand() & 61).abs();
    }
    b.move_spd = 0;
    let t = b.select_target(cx, 0, FAR);
    cx.scene.chars[me].target_char = t;
    if t.is_none() {
        if x.prepare_explode_all != 0 {
            for k in 0..ls.0.len() {
                leaf::order(ls, k, 27, b, x, cx);
            }
        }
        return;
    }
    b.calc_target_info(cx);
    if x.dmg_count == 0 {
        change_action(b, x, cx, act::NEEDLE, 3, true);
        x.dmg_count = (cx.cc.rand() & 31) + 50;
        return;
    }
    if x.regrow != 0 {
        x.regrow = 0;
        change_action(b, x, cx, act::LEAF_GROW, 3, true);
        return;
    }
    if x.prepare_explode_all != 0 {
        change_action(b, x, cx, act::RISE, 3, true);
        x.prepare_explode_all = 0;
        return;
    }
    if x.dropping != 0 {
        change_action(b, x, cx, act::LEAF_DROP, 3, true);
        return;
    }
    if x.reserve_laser != 0 {
        if ls.dropped() != 0 {
            change_action(b, x, cx, act::DESTROY_LEAF, 3, true);
            return;
        }
        if x.high_drive != 0 && x.laser_count > 0 {
            change_action(b, x, cx, act::DRAIN_ATK, 3, true);
            x.laser_count = 0;
        } else {
            change_action(b, x, cx, act::LASER, 3, true);
            x.laser_count += 1;
        }
        x.reserve_laser = 0;
        x.reserve_leaf_drop = 1;
        return;
    }
    if x.reserve_leaf_drop != 0 {
        if ls.dropped() == LEAVES {
            if max_pp_of(cx) >> 2 < pp_of(cx) {
                x.reserve_laser = 1;
            } else {
                x.reserve_leaf_drop = 1;
            }
            change_action(b, x, cx, act::LEAF_GROW, 3, true);
            return;
        }
        x.reserve_leaf_drop = 0;
        change_action(b, x, cx, act::LEAF_DROP, 3, true);
        return;
    }
    x.reserve_leaf_drop = 1;
    change_action(b, x, cx, act::LEAF_DROP, 3, true);
}

/// `OnThinkEpitaph` (0x004a1310): the drained neutral, its wait and the
/// next pattern.
fn on_epitaph(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    let me = cx.me;
    b.act_count += 1;
    b.move_spd = 0;
    b.move_vector = VF0;
    b.dirc[2] = geom::set_dirc(b.dirc[2], b.target_dirc, 256);
    if !b.wait_over(cx) {
        return;
    }
    let t = b.select_target(cx, 0, FAR);
    cx.scene.chars[me].target_char = t;
    if !cx.valid(t) {
        return;
    }
    b.calc_target_info(cx);
    change_next_pattern(b, x, cx);
}

/// `OnThinkDataDrain` (0x004a1470): drained; the leaves go.
fn on_data_drain(b: &mut Boss, x: &mut Magus, ls: &mut Leaves, cx: &mut Cx) {
    b.cheat_hp = 0;
    x.epitaph = 1;
    change_action(b, x, cx, act::EPITAPH, 3, true);
    b.move_spd = 0;
    for k in 0..ls.0.len() {
        leaf::order(ls, k, 27, b, x, cx);
    }
}

/// `OnThinkChase` (0x004a1530).
fn on_chase(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            b.spd_up(0x4316_0000, 0x4316_0000);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            x.animate_chase.set(b.transparency, 0x3f00_0000, 0x4170_0000);
            cx.se3d(229);
            b.act_proccess += 1;
            return;
        }
        1 => {
            if c & 3 == 3 {
                cx.out(Out::AfterImage);
                if c == 6 {
                    cx.se3d(57);
                }
            }
            let t = cx.scene.chars[me].target_char;
            let mut end = false;
            if cx.valid(t) {
                b.move_dirc = b.target_dirc;
                if ee::lt(b.target_dist, 0x4316_0000) {
                    change_next_pattern(b, x, cx);
                    end = true;
                }
            } else {
                let t = b.select_target(cx, 0, FAR);
                cx.scene.chars[me].target_char = t;
                if !cx.valid(t) {
                    exec_pattern(b, x, cx, 2, -1);
                    end = true;
                } else {
                    b.calc_target_info(cx);
                }
            }
            if !end {
                if c >= 120 {
                    b.act_proccess += 1;
                }
                b.dirc[2] = b.move_dirc;
                x.animate_chase.animate();
                b.set_transparency = x.animate_chase.transparency;
                return;
            }
        }
        2 => {}
        _ => return,
    }
    b.set_transparency = ONE;
    b.entry_cmnd_target(cx);
    cursor(cx, false);
}

/// `OnThinkEpitaphWave` (0x004a17d0): the drained wave, row 12 at 60.
fn on_epitaph_wave(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            x.anm_w.set(super::ANM_WAVE, cx.clips);
            b.act_proccess += 1;
            cx.se3d(223);
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 60 {
                let pos = cx.pos();
                b.skill_damage(cx, None, Some(pos), 12);
            }
            if b.anm.frame() == 30 {
                cx.se3d(224);
            }
            if b.act_count >= 35 {
                if b.act_count == 36 {
                    b.effect(cx, EffKind::WaveShock);
                    cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
                }
                let pos = cx.pos();
                cx.out(Out::CameraShake { pos, s: [2, 2, 3, 2] });
                let done = x.anm_w.forward();
                x.anm_w.speed = 512;
                cx.out(Out::DrawWave);
                if done && b.anm_status != 0 {
                    change_action(b, x, cx, act::EPITAPH, 3, true);
                }
            }
        }
        _ => {}
    }
    b.spd_down(0, 0x40a0_0000);
}

/// `OnThinkDataDrainAtk` (0x004a1a50): a member drained through menu 74
/// (stream 46).
fn on_drain_atk(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            let t = b.select_target(cx, 0, FAR);
            cx.scene.chars[me].target_char = t;
            if !target_ok(cx, t) {
                change_action(b, x, cx, act::NEUTRAL, 3, true);
                return;
            }
            b.calc_target_info(cx);
            b.lock_player(cx, false);
            let id = t.map_or(-1, |t| i32::from(cx.scene.chars[t].id()));
            let slot = cx.party.slot_of(id);
            cx.out(Out::StreamMenu { stream: 46, mask: 1 << slot });
            b.act_proccess += 1;
        }
        1 if cx.env.menu_type == -1 => {
            let t = cx.scene.chars[me].target_char;
            if let Some(tp) = t
                && cx.valid(t)
            {
                cx.affect(tp, 13, 0);
            }
            b.unlock_player();
            change_action(b, x, cx, act::NEUTRAL, 3, true);
        }
        _ => {}
    }
}

/// Mutation's act 9 (0x004a1c20): the stage darkened, camera 3 on the
/// target, skill 250 at frame 15, the stage back at 90, camera 1 at 15.
fn on_cast(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            let t = b.select_target(cx, 0, FAR);
            cx.scene.chars[me].target_char = t;
            if !target_ok(cx, t) {
                exec_pattern(b, x, cx, 2, -1);
                return;
            }
            b.calc_target_info(cx);
            x.stage_id = b.begin_stage_effect(cx, 0x6400_0000, [15, 15, 3000]);
            b.lock_player(cx, true);
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            let view = [b.target_pos[0], b.target_pos[1], 0x4348_0000, b.target_pos[3]];
            let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
            let m = geom::rot_matrix_x(&m, 0);
            let m = geom::rot_matrix_y(&m, 0);
            let m = geom::rot_matrix_z(&m, b.target_dirc);
            let v = geom::apply_matrix(&m, [0x447a_0000, 0, 0, ONE]);
            let p = geom::vadd(v, super::w2p(cx, view));
            let eye = super::p2w(cx, p);
            cx.out(Out::CameraChange(3));
            cx.out(Out::CameraPos { cam: 3, pos: eye });
            cx.out(Out::CameraView { cam: 3, view });
            b.act_proccess += 1;
        }
        1 if c == 15 => {
            // The flag is a3 as the frame left it (not set here): 1, cast
            // through Magus (the harness measures it).
            if let Some(tp) = cx.scene.chars[me].target_char
                && let Some(s) = item::item_skill_request(cx.t, cx.scene, me, tp, 250, 1, false, cx.rand)
            {
                cx.out(Out::Skill(tp, s));
            }
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2 if c == 90 => {
            let id = std::mem::replace(&mut x.stage_id, -1);
            b.end_stage_effect(cx, id, 0x6400_0000, 15);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        3 if c == 15 => {
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            b.unlock_player();
            cx.out(Out::CameraChange(1));
            change_next_pattern(b, x, cx);
        }
        _ => {}
    }
}

/// `OnThinkNeedle` (0x004a1f80): needles out of the ground 465 ahead,
/// boss skill 11 there two frames on.
fn on_needle(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    let me = cx.me;
    let m = geom::trans_matrix(&geom::unit_matrix(), [0, 0xc3e8_8000, 0x4348_0000, ONE]);
    let m = geom::rot_matrix_z(&m, b.dirc[2]);
    let m = geom::trans_matrix(&m, cx.scene.chars[me].pos_p);
    let p = geom::apply_matrix(&m, [0, 0, 0, ONE]);
    let at = super::p2w(cx, p);
    match b.act_proccess {
        0 => {
            if !cx.valid(cx.scene.chars[me].target_char) {
                let t = b.select_target(cx, 0, FAR);
                cx.scene.chars[me].target_char = t;
                if t.is_none() {
                    change_action(b, x, cx, act::NEUTRAL, 3, false);
                    return;
                }
            }
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c >= 21 {
                let n = Needle::new(cx, at, 0, 0x4248_0000, 0x3e99_999a, 128, 3, 15);
                let id = b.effects.create(EffKind::Needle { n: 128 });
                if let Ok(k) = usize::try_from(id)
                    && let Some(Some(e)) = b.effects.slots.get_mut(k)
                {
                    e.needle = Some(Box::new(n));
                }
                cx.out(Out::Effect { id, kind: EffKind::Needle { n: 128 }, pos: at, dirc: b.dirc });
                b.act_proccess += 1;
                b.act_count = 0;
            }
        }
        2 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 2 {
                b.skill_damage(cx, None, Some(at), 11);
            }
            if b.act_count >= 35 {
                change_action(b, x, cx, act::NEUTRAL, 3, true);
            }
        }
        _ => {}
    }
}

/// `OnThinkDie` (0x004a2200): the dead effect, then exit once it ends.
fn on_die(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            crate::fellow::delete_cmnd(cx.scene, cx.me);
            cx.out(Out::DeleteCmnd);
            b.move_spd = 0;
            b.move_vector = VF0;
            x.eff_dead = Some(b.begin_dead_effect(cx, 0x451c_4000, 0x451c_4000));
            b.act_proccess += 1;
        }
        1 if x.eff_dead.is_some() && !b.effects.enabled(x.eff_dead) => {
            x.eff_dead = None;
            b.act_proccess += 1;
            b.exit = 1;
        }
        _ => {}
    }
}

/// `ccBoss::IsValidArea(p)` (0x00473800): the world place's x and y each
/// under the centre's plus 2500 (a sign lost to `fabs`).
fn valid_area(b: &Boss, cx: &Cx, p: V4) -> bool {
    let w = super::p2w(cx, p);
    let lx = ee::sub(ee::add(0x453b_8000, b.center_pos[0]), 0x43fa_0000);
    let ly = ee::sub(ee::add(0x453b_8000, b.center_pos[1]), 0x43fa_0000);
    ee::lt(fabs(w[0]), lx) && ee::lt(fabs(w[1]), ly)
}

/// `OnThinkReturn` (0x004a2300): back to within 100 of the centre, then
/// stop.
fn on_return(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    b.spd_up(0x4120_0000, 0x4120_0000);
    if held(b, cx) {
        b.move_spd = 0;
        return;
    }
    let pp = cx.scene.chars[cx.me].pos_p;
    if !valid_area(b, cx, pp) {
        b.move_dirc = b.center_dirc;
    } else if ee::le(b.center_dist, 0x42c8_0000) {
        b.stop_time = 0;
        b.stop_counter = 0;
        change_action(b, x, cx, act::STOP, 3, true);
    }
    b.dirc[2] = geom::set_dirc(b.dirc[2], b.move_dirc, 256);
}

/// `OnThinkStop` (0x004a2420): the stop time down, then back to neutral at
/// the clip's start.
fn on_stop(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    b.move_spd = 0;
    b.move_vector = VF0;
    if b.stop_time != 0 {
        b.stop_time -= 1;
    }
    if b.stop_time != 0 || b.anm.frame() != 0 {
        return;
    }
    if epitaph(x) {
        change_action(b, x, cx, act::EPITAPH, 3, true);
    } else {
        change_action(b, x, cx, act::NEUTRAL, 3, true);
    }
    b.stop_time = 0;
    b.stop_counter = 0;
}

/// `OnThinkWander` (0x004a2540).
fn on_wander(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    b.spd_up(0x4120_0000, 0x4120_0000);
    if held(b, cx) {
        b.move_spd = 0;
        return;
    }
    let pp = cx.scene.chars[cx.me].pos_p;
    if !valid_area(b, cx, pp) {
        if epitaph(x) {
            change_next_pattern(b, x, cx);
        } else {
            change_action(b, x, cx, act::RETURN, 3, true);
        }
        return;
    }
    let c = b.act_count;
    b.act_count += 1;
    if c == 0 {
        let r = rand_unit(cx);
        let d = ee::add(b.center_dirc, r);
        b.move_dirc = d;
        if ee::lt(d, NEG_PI) {
            b.move_dirc = ee::add(d, TWO_PI);
        }
        if !ee::le(b.move_dirc, PI) {
            b.move_dirc = ee::sub(b.move_dirc, TWO_PI);
        }
        b.act_proccess = ((cx.cc.rand() & 31) + 90) as i16;
    }
    if b.act_count < b.act_proccess {
        b.dirc[2] = geom::set_dirc(b.dirc[2], b.move_dirc, 256);
        return;
    }
    b.move_spd = 0;
    if epitaph(x) {
        change_next_pattern(b, x, cx);
        return;
    }
    b.stop_time = 0;
    b.stop_counter = 0;
    change_action(b, x, cx, act::STOP, 3, true);
}

/// `OnThinkDash` (0x004a27a0): to `dashPos` at 50, off the targets.
fn on_dash(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    let me = cx.me;
    b.spd_up(0x4248_0000, 0x4248_0000);
    match b.act_proccess {
        0 | 1 => {
            if b.act_proccess == 0 {
                b.erase_cmnd_target(cx);
                cursor(cx, true);
                b.act_proccess += 1;
                cx.se3d(229);
            }
            if b.act_count & 3 == 3 {
                cx.out(Out::AfterImage);
                if b.act_count == 6 {
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
            if ee::lt(get_dist(pp, p), 0x42c8_0000) {
                b.act_proccess += 1;
            }
            b.dirc[2] = b.move_dirc;
            b.act_count += 1;
        }
        2 => {
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            change_next_pattern(b, x, cx);
            b.set_transparency = ONE;
        }
        _ => {}
    }
}

/// `OnThinkEscape` (0x004a2990): 120 frames off at 30, its heading
/// swinging every 32; it turns `moveDirc` toward `dirc.x` (0) each frame.
fn on_escape(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    let c = b.act_count;
    b.act_count += 1;
    if c >= 121 {
        b.entry_cmnd_target(cx);
        cursor(cx, false);
        change_next_pattern(b, x, cx);
        return;
    }
    b.move_spd = 0x41f0_0000;
    if held(b, cx) {
        b.move_spd = 0;
        return;
    }
    if c == 0 {
        let r = rand_f(cx.cc, PI);
        let d = ee::add(b.center_dirc, r);
        b.move_dirc = d;
        b.m_act_dirc[0] = d;
        b.m_act_dirc[2] = d;
        b.move_dirc = wrap_hi(d);
        x.act_pos = cx.pos();
    }
    if c & 31 == 31 {
        b.act_proccess ^= 1;
        let f = if b.act_proccess != 0 {
            ee::add(0x3f06_0a92, b.m_act_dirc[2])
        } else {
            ee::sub(b.m_act_dirc[2], 0x3f06_0a92)
        };
        b.m_act_dirc[0] = wrap_hi(f);
    }
    b.move_dirc = geom::set_dirc(b.move_dirc, b.dirc[0], 512);
    let pp = cx.scene.chars[cx.me].pos_p;
    if !valid_area(b, cx, pp) {
        change_next_pattern(b, x, cx);
        return;
    }
    b.dirc[2] = b.move_dirc;
}

/// `OnThinkWave` (0x004a2c40): SEs at 0 and 30, the wave at 35, boss
/// skill 12 25 frames on.
fn on_wave(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    match b.act_proccess {
        0 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 0 {
                cx.se3d(223);
            } else if b.act_count == 30 {
                cx.se3d(224);
            }
            if b.act_count == 35 {
                x.anm_w.set(super::ANM_WAVE, cx.clips);
                b.act_count = 0;
                b.act_proccess += 1;
                cx.out(Out::Flash { t: 15, colour: 0x80e0_ffff });
                b.effect(cx, EffKind::WaveShock);
            }
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 25 {
                let pos = cx.pos();
                b.skill_damage(cx, None, Some(pos), 12);
            }
            let pos = cx.pos();
            cx.out(Out::CameraShake { pos, s: [2, 2, 3, 2] });
            x.anm_w.speed = 512;
            let done = x.anm_w.forward();
            cx.out(Out::DrawWave);
            if b.anm_status != 0 && done {
                change_action(b, x, cx, act::NEUTRAL, 3, true);
            }
        }
        _ => {}
    }
    b.spd_down(0, 0x40a0_0000);
}

/// Mutation's act 8 (0x004a2ec0): off the targets and body hit, 100 a
/// frame toward the centre until 1500 from where it began.
fn on_to_centre(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    let me = cx.me;
    b.dirc[2] = b.move_dirc;
    match b.act_proccess {
        0 => {
            b.erase_cmnd_target(cx);
            cursor(cx, true);
            b.body_hit_sw = 0;
            b.move_dirc = b.center_dirc;
            b.move_spd = 0x42c8_0000;
            x.act_pos = cx.pos();
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c & 3 == 0 {
                cx.out(Out::AfterImage);
            }
            let from = super::w2p(cx, x.act_pos);
            if !ee::le(get_dist(from, cx.scene.chars[me].pos_p), 0x44bb_8000) {
                b.act_proccess += 1;
            }
        }
        2 => {
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            b.body_hit_sw = 1;
            x.stuck = 0;
            if epitaph(x) {
                change_action(b, x, cx, act::EPITAPH, 3, true);
            } else {
                change_action(b, x, cx, act::NEUTRAL, 3, true);
            }
        }
        _ => {}
    }
}

/// `OnThinkLeafDrop` (0x004a3100): Magus circles the centre, a leaf off
/// every 60 frames; the first and seventh bring the wave, the sixth a
/// skill; held too long or astray, act 8.
fn on_leaf_drop(b: &mut Boss, x: &mut Magus, ls: &mut Leaves, cx: &mut Cx) {
    let me = cx.me;
    b.spd_up(0x4120_0000, 0x4120_0000);
    x.dropping = 1;
    b.dirc[2] = geom::set_dirc(b.dirc[2], b.move_dirc, 256);
    if x.pushed >= 90 {
        to_centre(b, x, cx);
        return;
    }
    if held(b, cx) {
        b.move_spd = 0;
        b.move_vector = VF0;
        let c = b.act_proccess;
        b.act_proccess += 1;
        if c >= 61 {
            change_action(b, x, cx, act::WAVE, 3, true);
            return;
        }
    }
    let d = b.center_dist;
    if !ee::le(d, 0x44bb_8000) {
        b.move_dirc = ee::sub(ee::add(HALF_PI, b.center_dirc), 0x3e32_b8c3);
        b.dirc[2] = geom::set_dirc(b.dirc[2], b.move_dirc, 256);
        let pp = cx.scene.chars[me].pos_p;
        if !valid_area(b, cx, pp) {
            to_centre(b, x, cx);
        }
        return;
    }
    if ee::lt(d, 0x447a_0000) {
        let f = ee::sub(ONE, ee::div(d, 0x44bb_8000));
        b.move_dirc = ee::add(ee::add(HALF_PI, b.center_dirc), ee::mul(HALF_PI, f));
    } else {
        b.move_dirc = ee::add(HALF_PI, b.center_dirc);
    }
    let c = x.act_cnt;
    x.act_cnt += 1;
    if c < 60 {
        return;
    }
    x.act_cnt = 0;
    let mut id = -1;
    for k in 0..ls.0.len() {
        if ls.0[k].x.drop == 0 {
            ls.0[k].x.act_cnt = 0;
            ls.0[k].b.exit = 0;
            leaf::order(ls, k, 24, b, x, cx);
            id = ls.0[k].x.slave_id;
            break;
        }
    }
    if id == -1 {
        x.act_cnt = 0;
        x.no_leaf = 1;
        if max_pp_of(cx) >> 2 < pp_of(cx) {
            x.reserve_laser = 1;
        } else {
            x.reserve_leaf_drop = 1;
        }
        change_action(b, x, cx, act::NEUTRAL, 3, true);
        return;
    }
    if (id % 6).abs() == 0 {
        change_action(b, x, cx, act::WAVE, 3, true);
        return;
    }
    if id != 5 {
        return;
    }
    let t = b.select_target(cx, 3, FAR);
    cx.scene.chars[me].target_char = t;
    if target_ok(cx, t) {
        let k = (cx.cc.rand() % 3).unsigned_abs() as usize;
        let sid = cx.data.magus.drop_skills[k];
        if let Some(tp) = t
            && let Some(s) = item::item_skill_compel(cx.t, cx.scene, me, tp, sid, 0, false, cx.rand)
        {
            cx.out(Out::Skill(tp, s));
        }
    }
    change_action(b, x, cx, act::STOP, 3, true);
}

/// The leaf drop's way out when stuck or astray: `stuck`, still, act 8.
fn to_centre(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    x.stuck = 1;
    b.move_spd = 0;
    b.move_vector = VF0;
    change_action(b, x, cx, act::TO_CENTRE, 3, true);
}

/// `OnThinkLeafCountDown` (0x004a36b0): each fallen leaf's countdown
/// started again (order 25).
fn on_leaf_count_down(b: &mut Boss, x: &mut Magus, ls: &mut Leaves, cx: &mut Cx) {
    let c = x.act_cnt;
    x.act_cnt += 1;
    if c < 0 {
        return;
    }
    for k in 0..ls.0.len() {
        if ls.0[k].x.drop != 0 && ls.0[k].b.exit == 0 {
            leaf::order(ls, k, 25, b, x, cx);
        }
    }
    change_action(b, x, cx, act::NEUTRAL, 3, true);
    x.act_cnt = 0;
}

/// `OnThinkLeafGrow` (0x004a37c0): the grow clip at a fifth of its speed,
/// the leaves' charges on at frame 1, and all back on the body at its end.
fn on_leaf_grow(b: &mut Boss, x: &mut Magus, ls: &mut Leaves, cx: &mut Cx) {
    b.move_spd = 0;
    b.move_vector = VF0;
    if b.act_count == 1 {
        for k in 0..ls.0.len() {
            leaf::order(ls, k, 29, b, x, cx);
        }
        x.grow_se = true;
        let pos = cx.pos();
        cx.out(Out::SeLoop { se: 230, pos: Some(pos) });
    }
    let c = b.act_count;
    b.act_count += 1;
    if c == 0 {
        b.anm.speed = 51;
        x.grow_interval = 0;
    }
    if b.anm_status == 0 {
        return;
    }
    for k in 0..ls.0.len() {
        leaf::order(ls, k, 30, b, x, cx);
        ls.0[k].x.drop = 0;
    }
    x.dropping = 0;
    if x.grow_se {
        cx.out(Out::SeLoop { se: 230, pos: None });
    }
    x.grow_se = false;
    b.anm.speed = 256;
    change_action(b, x, cx, act::NEUTRAL, 3, true);
}

/// `OnThinkDestroyLeaf` (0x004a39a0): at the clip's first frame, each
/// leaf still on the body sent off to fade (order 28), one a pass.
fn on_destroy_leaf(b: &mut Boss, _x: &mut Magus, ls: &mut Leaves, cx: &mut Cx) {
    b.move_spd = 0;
    b.move_vector = VF0;
    if b.anm.frame() != 0 {
        return;
    }
    let c = b.act_count;
    b.act_count += 1;
    if c != 0 {
        return;
    }
    for _ in 0..LEAVES {
        for k in 0..ls.0.len() {
            if ls.0[k].x.drop == 0 {
                ls.0[k].x.act_cnt = 0;
                ls.0[k].b.exit = 0;
                leaf::order(ls, k, 28, b, _x, cx);
                break;
            }
        }
    }
}

/// `OnThinkLaser` (0x004a3ae0): locked, cinema 9, camera 3; up to 1500,
/// the laser clip and its shocks, down with boss skill 9 at the target.
fn on_laser(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 0 {
                // LockPlayer's argument is left in a1 by `Think`: its jump
                // table's address, so the chat stays open.
                b.lock_player(cx, true);
                b.erase_cmnd_target(cx);
                cursor(cx, true);
                cx.out(Out::Cinema(Some(9)));
                cx.out(Out::CameraChange(3));
                x.cam_id = 3;
                b.move_vector = VF0;
                b.move_spd = 0;
                x.prev_laser = cx.pos();
                x.prev_dirc = b.dirc;
            }
            if b.act_count == 5 {
                cx.out(Out::Se { se: 223 });
            }
            if ee::lt(cx.scene.chars[me].pos[2], 0x44bb_8000) {
                b.move_vector[2] = 0x4248_0000;
            } else {
                if b.anm.frame() == 0 {
                    b.act_proccess += 1;
                    b.act_count = 0;
                }
                b.move_vector[2] = 0;
            }
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 0 {
                if let Some(Some(clip)) = b.anm_tbl.get(act::LASER as usize).cloned() {
                    b.anm.set(&clip, cx.clips);
                }
                if b.anm.clip.is_some() {
                    b.anm.forward();
                }
                setup_laser_shock(x);
                for k in 0..LEAVES {
                    cx.out(Out::Magus(Pic::Thunder { shock: k }));
                }
                cx.out(Out::Se { se: 68 });
            } else if b.act_count == 40 {
                b.act_proccess += 1;
                b.act_count = 0;
            }
        }
        2 => {
            if b.anm_status != 0 {
                cx.scene.chars[me].pos[2] = 0;
                b.act_proccess += 1;
                b.act_count = 0;
                cx.scene.chars[me].pos = x.prev_laser;
                b.dirc = x.prev_dirc;
                cx.scene.chars[me].pos[2] = 0;
                b.move_vector[2] = 0;
            }
        }
        3 if ee::le(cx.scene.chars[me].pos[2], 0) => {
            let tp = b.target_pos;
            b.skill_damage(cx, None, Some(tp), 9);
            b.move_vector[2] = 0;
            cx.scene.chars[me].pos[2] = 0;
            x.cam_id = 1;
            cx.out(Out::CameraChange(1));
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            change_action(b, x, cx, act::NEUTRAL, 3, true);
            cx.out(Out::Cinema(None));
            b.act_proccess += 1;
        }
        _ => {}
    }
    if b.act_proccess == 0 {
        return;
    }
    match b.anm.frame() {
        222 => cx.out(Out::Se { se: 40 }),
        202 | 187 | 172 | 157 | 142 | 127 => cx.out(Out::SeNote { se: 56, note: 70 }),
        190 | 175 | 165 | 150 | 135 | 120 => cx.out(Out::SeNote { se: 61, note: 72 }),
        _ => {}
    }
}

/// `SetupLaserShock` (0x0049f080): each of the twelve shocks (the body's
/// `OBJ_ex31les00`-`55`) set on its leaf; `m_shockIdx` 0.
fn setup_laser_shock(x: &mut Magus) {
    for (k, s) in x.shocks.iter_mut().enumerate() {
        *s = Shock { index: k as i32, transparency: ONE, draw: 0, act_count: 0 };
    }
    x.shock_idx = 0;
}

/// `OnThinkRise` (0x004a3fd0): the leaves about to burst lift Magus to
/// 5000; the flash, boss skill 10 at 300/7 a leaf, one leaf burst every 5
/// frames.
fn on_rise(b: &mut Boss, x: &mut Magus, ls: &mut Leaves, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            let c = b.act_count;
            b.act_count += 1;
            if c == 0 {
                x.explode_leaf_num = 0;
                x.blur = BLUR_RISE;
                cx.out(Out::Blur { colour: x.blur });
                cx.out(Out::CameraChange(3));
                x.cam_id = 3;
                b.move_vector = VF0;
                b.move_spd = 0;
                b.lock_player(cx, true);
                b.erase_cmnd_target(cx);
                cursor(cx, true);
                cx.out(Out::Cinema(Some(10)));
                cx.out(Out::SeNote { se: 61, note: 36 });
            }
            b.move_vector[2] = 0x437a_0000;
            let z = cx.scene.chars[me].pos[2];
            if !ee::lt(z, 0x459c_4000) {
                b.move_vector[2] = 0;
                b.act_proccess += 1;
                b.act_count = 0;
            } else if !ee::lt(z, 0x44fa_0000) {
                b.move_vector[2] = ee::add(0x4120_0000, ee::div(ee::sub(0x459c_4000, z), 0x40c0_0000));
            }
        }
        1 => {
            b.move_vector[2] = 0;
            let c = b.act_count;
            b.act_count += 1;
            if c >= 15 {
                b.act_count = 0;
                b.act_proccess += 1;
            }
        }
        _ => {
            b.act_count += 1;
            if b.act_proccess == 2 && b.act_count >= 5 {
                cx.out(Out::FlashFade { t: [60, 60, 0], colour: 0x80e0_ffff });
                // The count's loop never steps its leaf: leaf 0, twelve
                // times over.
                if let Some(p) = ls.0.first() {
                    for _ in 0..LEAVES {
                        if p.b.exit == 0 && p.x.drop != 0 && p.x.explode == 0 {
                            x.explode_leaf_num += 1;
                        }
                    }
                }
                let mut sk: SkillParam = Boss::skill(cx, 10);
                sk.atk = (300 * x.explode_leaf_num / 7) as i16;
                let tp = b.target_pos;
                b.skill_damage_with(cx, tp, &sk);
                b.act_proccess += 1;
            }
            if b.act_count == 5 {
                b.act_count = 0;
                for k in 0..ls.0.len() {
                    let p = &ls.0[k];
                    if p.b.exit == 0 && p.x.drop != 0 && p.x.explode == 0 {
                        leaf::order(ls, k, 26, b, x, cx);
                        break;
                    }
                }
            }
        }
    }
}

/// `OnThinkFall` (0x004a4460): down at 50 to the height it rose from, 30
/// frames, then camera 1 and the leaves back if all fell.
fn on_fall(b: &mut Boss, x: &mut Magus, ls: &mut Leaves, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            b.move_vector[2] = 0xc248_0000;
            if ee::le(cx.scene.chars[me].pos[2], 0) {
                b.move_vector[2] = 0;
                b.act_proccess += 1;
                cx.scene.chars[me].pos[2] = x.prev_rise;
            }
        }
        1 => {
            b.move_vector[2] = 0;
            let c = b.act_count;
            b.act_count += 1;
            if c >= 30 {
                b.act_proccess += 1;
                b.act_count = 0;
            }
        }
        2 => {
            cx.out(Out::CameraChange(1));
            x.cam_id = 1;
            b.unlock_player();
            b.entry_cmnd_target(cx);
            cursor(cx, false);
            cx.scene.chars[me].pos[2] = 0;
            cx.out(Out::Cinema(None));
            if ls.dropped() == LEAVES {
                for p in &mut ls.0 {
                    p.x.drop = 0;
                }
                x.regrow = 1;
            }
            change_action(b, x, cx, act::NEUTRAL, 3, true);
        }
        _ => {}
    }
}

/// `ccBoss03::OnExitLeaf` (0x004a0c20): once every leaf fell and went
/// (not while rising or falling), the regrow asked and the destroy ended.
pub(crate) fn on_exit_leaf(b: &mut Boss, x: &mut Magus, ls: &Leaves, cx: &mut Cx) {
    if b.act_num == act::RISE || b.act_num == act::FALL {
        return;
    }
    if ls.0.iter().any(|p| p.x.drop == 0 || p.b.exit == 0) {
        return;
    }
    x.regrow = 1;
    if b.act_num == act::DESTROY_LEAF {
        change_action(b, x, cx, act::NEUTRAL, 3, true);
    }
    x.no_leaf = 0;
}

/// The rise's leaves' `ChangeAction(27, 3, 4)` on the body from a leaf's
/// countdown.
pub(crate) fn begin_rise(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    change_action(b, x, cx, act::RISE, 3, true);
}

/// The last burst's `ChangeAction(28, 3, 4)` on the body.
pub(crate) fn begin_fall(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    change_action(b, x, cx, act::FALL, 3, true);
}

// --- CalcCamera -------------------------------------------------------------------------------

/// `ccBoss03::CalcCamera` (0x0049e9e0): camera 3 while `m_camID` is 3 -
/// round the rise, behind the fall, and in the laser over Kite, where
/// Magus stands above him from the clip's frame 135.
fn calc_camera(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    if x.cam_id != 3 {
        return;
    }
    let me = cx.me;
    let pos = cx.scene.chars[me].pos;
    match b.act_num {
        act::RISE => {
            let f = wrap_hi(ee::add(0x40b8_4e89, b.dirc[2]));
            let neg = [pos[0] ^ 0x8000_0000, pos[1] ^ 0x8000_0000, pos[2] ^ 0x8000_0000, ONE];
            let m = geom::trans_matrix(&geom::unit_matrix(), neg);
            let m = geom::rot_matrix_y(&m, 0x3e86_0a92);
            let m = geom::rot_matrix_z(&m, f);
            let m = geom::trans_matrix(&m, pos);
            let e = [ee::add(pos[0], ONE), ee::add(pos[1], ONE), ee::add(pos[2], 0x44bb_8000), pos[3]];
            let eye = geom::apply_matrix(&m, e);
            cx.out(Out::CameraPos { cam: 3, pos: eye });
            cx.out(Out::CameraView { cam: 3, view: pos });
        }
        act::FALL => {
            let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
            let m = geom::rot_matrix_z(&m, b.target_dirc);
            let v = geom::apply_matrix(&m, [0x44fa_0000, 0, 0, ONE]);
            let mut eye = geom::vadd(pos, v);
            eye[2] = 0x42c8_0000;
            cx.out(Out::CameraPos { cam: 3, pos: eye });
            cx.out(Out::CameraView { cam: 3, view: pos });
        }
        act::LASER if b.act_proccess == 0 => {
            b.dirc[2] = b.center_dirc;
            laser_eye(b, cx, pos);
            cx.out(Out::CameraView { cam: 3, view: pos });
        }
        act::LASER if b.act_proccess == 2 => {
            let frame = b.anm.frame() as u32;
            let mut at_135 = frame == 135;
            if frame < 135 {
                let c = b.act_count;
                b.act_count += 1;
                if c == 15 {
                    b.act_count = 0;
                    at_135 = false;
                }
            }
            if at_135 {
                x.prev_laser = pos;
                let kite = cx.scene.pc_list.first().copied();
                if let Some(k) = kite {
                    let kp = cx.scene.chars[k].pos;
                    cx.scene.chars[me].pos[0] = kp[0];
                    cx.scene.chars[me].pos[1] = kp[1];
                }
                b.dirc[2] = geom::set_dirc(b.dirc[2], b.center_dirc, 256);
                x.cam_id = 3;
                let pos = cx.scene.chars[me].pos;
                laser_eye(b, cx, pos);
                let view = [pos[0], pos[1], 0x4120_0000, pos[3]];
                cx.out(Out::CameraView { cam: 3, view });
                x.cam_count = 0;
                cx.out(Out::FlashFade { t: [80, 80, 20], colour: 0x80e0_ffff });
                x.laser_flash = 0;
            }
            let _ = cx.cc.rand();
            if frame >= 150 {
                x.cam_count += 1;
                let mut f = rand_unit(cx);
                if ee::lt(f, NEG_PI) {
                    f = ee::add(f, TWO_PI);
                }
                if !ee::le(f, PI) {
                    f = ee::sub(f, TWO_PI);
                }
                let pos = cx.scene.chars[me].pos;
                let z = ee::add(0x4120_0000, ee::mul(0x42c8_0000, libm::sinf(f)));
                cx.out(Out::CameraView { cam: 3, view: [pos[0], pos[1], z, pos[3]] });
            }
        }
        _ => {}
    }
}

/// The laser's eye: 1000 behind Magus's heading, a tenth aside, 50 up.
fn laser_eye(b: &Boss, cx: &mut Cx, pos: V4) {
    let m = geom::rot_matrix_z(&geom::unit_matrix(), b.dirc[2]);
    let e = [ee::add(pos[0], 0x3dcc_cccd), ee::sub(pos[1], 0x447a_0000), 0x4248_0000, pos[3]];
    let v = geom::apply_matrix(&m, geom::vsub(e, pos));
    let eye = geom::vadd(v, pos);
    cx.out(Out::CameraPos { cam: 3, pos: eye });
}

// --- Affect -----------------------------------------------------------------------------------

/// `ccBoss03::Affect` (0x0049f6a0): the drain (13) to act 11 and the grow's
/// sound off; 21's 4500 HP; a hit as the base's, then the laser reserved
/// once its protect gauge is past half; the rest the base's.
pub(super) fn affect(b: &mut Boss, cx: &mut Cx) {
    let mut x = magus_of(b);
    affect_of(b, &mut x, cx);
    b.class = Class::Magus(x);
}

fn affect_of(b: &mut Boss, x: &mut Magus, cx: &mut Cx) {
    let me = cx.me;
    let (t, p0, person) = {
        let a = &cx.scene.chars[me].affect;
        (a.ty, a.param[0], a.person)
    };
    if t != 21 && (b.lock_player != 0 || b.act_forbid == 2) {
        return;
    }
    let hp = cx.scene.chars[me].hp;
    let mhp = cx.scene.chars[me].max_hp;
    match t {
        13 => {
            change_action(b, x, cx, act::DATA_DRAIN, 2, true);
            cx.scene.chars[me].affect.ty = 0;
            if x.grow_se {
                cx.out(Out::SeLoop { se: 230, pos: None });
            }
        }
        21 => {
            let ch = &mut cx.scene.chars[me];
            ch.hp = 4500;
            ch.max_hp = 4500;
            if let Some(f) = ch.foe_state_mut() {
                f.pp = -1;
                f.pp_count = 0;
            }
            b.anm.speed = 256;
        }
        1 | 3 => {
            if cx.annihilated() || cx.game_over {
                return;
            }
            cx.out(Out::FlyFont { kind: 2, n: i32::from(p0) });
            if p0 >= 0 {
                cx.out(Out::HitMark { by: person });
                let mut h = if b.cheat_hp != 0 { hp.wrapping_sub(p0 / 10).max(mhp / 2) } else { hp.wrapping_sub(p0) };
                if h <= 0 {
                    h = 0;
                    cx.out(Out::ClearSpcCondition);
                    crate::affect::clear_conditions(&mut cx.scene.chars[me]);
                    change_action(b, x, cx, act::DIE, 2, true);
                } else if t == 1 {
                    let n = if cx.env.count & 1 != 0 { act::DMG0 } else { act::DMG1 };
                    change_action(b, x, cx, n, 0, true);
                }
                cx.scene.chars[me].hp = h;
            }
            if epitaph(x) || x.high_drive != 0 {
                return;
            }
            if pp_of(cx) >= max_pp_of(cx) >> 1 {
                x.reserve_laser = 1;
                x.high_drive = 1;
            }
        }
        _ => b.base_affect(cx),
    }
}
