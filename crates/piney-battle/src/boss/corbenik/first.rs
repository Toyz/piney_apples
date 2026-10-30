//! Corbenik's first form (`ccBoss08`, QUA gcmn 0x003acbd0-0x003b00a0):
//! the body (`bossTbl` row 7, `x81`) with eight seeds of its own
//! ([`super::seed`]) it throws at the party, each a boss on its own
//! character counting down to a burst. Its patterns (`boss08ActNormalTbl`)
//! wait, run off, throw seeds, cast and leap; the flash, the pollen and the
//! body press have no pattern of their own (no table holds their words).

use piney_data::field::ee;

use super::eff::{self, Fx};
use super::seed::{self, Seed};
use super::{Gen, Pic, SEEDS, make_fx, manager_pass, word, wrap_hi};
use crate::boss::kyvia::Part;
use crate::boss::{
    BaseActs, Boss, BossBase, BossData, Class, Cx, EffKind, Out, Patterns, Tbl, VF0, base_change_next_pattern,
    base_exec_pattern_index,
};
use crate::enemy_ai::{get_dirc, rand_f2};
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
/// The stage fader's colour (`BeginStageEffect(0x64000000, ...)`).
const DARK: u32 = 0x6400_0000;
/// The leap's turn each jump: 2.0944 (2 pi / 3).
const LEAP_TURN: F = 0x4006_0a92;

/// The first form's acts (`actNum`, `Think` at 0x003ad460; any other is
/// `ChangeAction(0, 1, 4)`).
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const DAMAGE0: i16 = 1;
    pub const DAMAGE1: i16 = 2;
    pub const FLASH: i16 = 3;
    pub const POLLEN: i16 = 4;
    pub const BODY_PRESS: i16 = 5;
    pub const WAVE: i16 = 6;
    pub const DIE: i16 = 14;
    pub const ESCAPE: i16 = 16;
    pub const MAGIC: i16 = 20;
    pub const SEED: i16 = 21;
    /// The base `ExecPatternIndex`'s acts for words no table of
    /// Corbenik's holds (none in its `Think`: the next frame's
    /// `ChangeAction(0, 1, 4)`).
    pub const BASE_DASH_CENTER: i16 = 7;
    pub const BASE_EPITAPH: i16 = 12;
    pub const BASE_RETURN: i16 = 17;
    pub const BASE_CHASE: i16 = 18;
    pub const BASE_WANDER: i16 = 20;
}

/// `ccJumpData`: a leap's start, end and control point, and its time (0
/// to 1, a 25th a frame).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Jump {
    pub sp: V4,
    pub ep: V4,
    pub cp: V4,
    pub time: F,
}

/// `ccBoss08`'s own members (+0x1731d0 on; the seeds at +0x29350 are their
/// own characters, [`First::seed_me`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct First {
    pub seed_me: [usize; SEEDS],
    /// Scratch for the span of [`main`] (or of an `Affect`): the seeds out
    /// of their characters.
    pub seeds: Vec<Part<Seed>>,
    /// `m_bossEff`: the dead effect; `m_stageEff`.
    pub boss_eff: Option<i32>,
    pub stage_eff: i32,
    /// `m_dmgCount`: the blows taken (never read).
    pub dmg_count: i32,
    /// `m_jumpData`: the wave's leap.
    pub jump: Jump,
    pub pat_mode: i32,
    /// `m_deadFlag`: nothing sets it.
    pub dead_flag: bool,
    /// `m_pauseCntDownFlag`: the seeds' countdowns held (the magic's, the
    /// death's).
    pub pause_cnt_down: bool,
    pub entry_seed_num: i32,
    /// `ccBoss::actPos` (+0x1b0): the magic's camera shake.
    pub act_pos: V4,
}

/// The seeds taken out of their characters (`Class::CorbenikSeed`),
/// whether each was there.
fn take_seeds(cx: &mut Cx, x: &mut First) -> [bool; SEEDS] {
    x.seeds.clear();
    std::array::from_fn(|k| {
        let (part, found) = seed::take(cx, x.seed_me[k]);
        x.seeds.push(part);
        found
    })
}

fn put_seeds(cx: &mut Cx, x: &mut First, found: [bool; SEEDS]) {
    for (k, part) in std::mem::take(&mut x.seeds).into_iter().enumerate() {
        seed::put(cx, part, found[k]);
    }
}

/// `seed->Method()` with `cx.me` the seed's character.
fn with_seed<R>(x: &mut First, k: usize, cx: &mut Cx, f: impl FnOnce(&mut Boss, &mut Seed, &mut Cx) -> R) -> R {
    let p = &mut x.seeds[k];
    crate::boss::kyvia::as_me(cx, p.me, |cx| f(&mut p.b, &mut p.x, cx))
}

/// Each seed not exited (`CheckExit`) `EraseCmndTarget`'d or
/// `EntryCmndTarget`'d.
fn seeds_targetable(x: &mut First, cx: &mut Cx, on: bool) {
    for k in 0..SEEDS {
        if x.seeds[k].b.base.exit == 0 {
            with_seed(
                x,
                k,
                cx,
                |b, _, cx| if on { b.base.entry_cmnd_target(cx) } else { b.base.erase_cmnd_target(cx) },
            );
        }
    }
}

// --- the constructor ----------------------------------------------------------------------

/// `ccBoss08::ccBoss08` (0x003acbd0): the boss is `scene.chars[cx.me]`
/// (row 7) 500 behind Kite and 10 up, his heading; eight seeds made (row
/// 44, exited, off the lists) on their own characters with it as master;
/// no boss camera and no blur of its own; the normal table's first
/// pattern.
pub fn new(cx: &mut Cx, kite_pos: V4, kite_dirc: V4, center: V4) -> Boss {
    let me = cx.me;
    let mut b = crate::boss::kyvia::plain_boss();
    b.anm_tbl = cx.data.corbenik.anims.clone();
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
    crate::fellow::entry_cmnd(cx.scene, me);
    b.center_pos = [center[0], center[1], center[2], ONE];
    b.center_pos_p = crate::boss::w2p(cx, b.center_pos);
    b.use_stage_eff = true;
    let seed_me = std::array::from_fn(|k| {
        let sm = crate::boss::kyvia::new_char(cx, super::SEED_ROW);
        let part = seed::new(cx, sm, me, k as i32);
        seed::put(cx, part, true);
        sm
    });
    let mut x = Box::new(First {
        seed_me,
        // `m_stageEff` is never set here: the object's own 0 (read only
        // after a `BeginStageEffect` or the death's -1).
        stage_eff: 0,
        entry_seed_num: 1,
        ..First::default()
    });
    let found = take_seeds(cx, &mut x);
    set_pattern_mode(&mut b, &mut x, cx, 0);
    put_seeds(cx, &mut x, found);
    Boss { base: b, class: Class::Corbenik(x) }
}

/// `SetPatternMode(m)` (0x003adc30): mode 0 the normal table (any other a
/// null one), run from its start.
fn set_pattern_mode(b: &mut BossBase, x: &mut First, cx: &mut Cx, m: i32) {
    let t = if m == 0 { Tbl::Normal } else { Tbl::None };
    b.pat_tbl = t;
    b.pat_index = 0;
    x.pat_mode = m;
    let tbl = cx.data.corbenik.words(t);
    b.pat_index = x.exec_pattern_index(b, cx, tbl, 0);
}

// --- the frame -----------------------------------------------------------------------------

impl crate::boss::BossClass for First {
    fn main(&mut self, b: &mut BossBase, cx: &mut Cx) {
        main(b, self, cx);
    }
    fn affect(&mut self, b: &mut BossBase, cx: &mut Cx) {
        affect(b, self, cx);
    }
}

/// The effect manager's pass, then `ccBoss08::Main` (0x003ad000):
/// `ccBoss::Main` (its `Slave` each seed's own `Main`), `hold` cleared.
pub(crate) fn main(b: &mut BossBase, x: &mut First, cx: &mut Cx) {
    let found = take_seeds(cx, x);
    manager_pass(b, cx);
    frame(b, x, cx);
    put_seeds(cx, x, found);
    cx.scene.chars[cx.me].cond[cond::HOLD] = 0;
}

/// `ccBoss::Main` (QUA gcmn 0x0035fda0) on the first form: `PreDrawAnm`
/// its own (the blink, the vibe's scale), `Slave` the seeds'.
fn frame(b: &mut BossBase, x: &mut First, cx: &mut Cx) {
    if b.exit != 0 {
        return;
    }
    b.prologue(cx);
    b.tick_stop();
    think(b, x, cx);
    b.lock_party(cx);
    b.move_(cx);
    if b.draw_sw != 0 {
        pre_draw_anm(b);
        b.draw(cx);
    }
    // `Slave` (0x003ad030): each seed not exited, its own `Main`.
    for k in 0..SEEDS {
        if x.seeds[k].b.base.exit == 0 {
            seed::main(b, x, k, cx);
        }
    }
}

/// `ccBoss08::PreDrawAnm` (0x003ad5d0): the clip forward. Its blink
/// and its vibe's scale are the flash's alone (`_ChangeVibeType(4)`,
/// which nothing calls): the scale stays 1 and no fog is set.
fn pre_draw_anm(b: &mut BossBase) {
    b.anm_status = i8::from(b.anm.forward());
}

// --- the patterns --------------------------------------------------------------------------

impl Patterns for First {
    const ACTS: BaseActs = super::BASE_ACTS;

    fn words<'d>(&self, b: &BossBase, data: &'d BossData) -> &'d [i32] {
        data.corbenik.words(b.pat_tbl)
    }

    fn change_next_pattern(&mut self, b: &mut BossBase, cx: &mut Cx) {
        // `ccBoss08::ChangeNextPattern` (0x003ade20): the countdowns go on.
        self.pause_cnt_down = false;
        base_change_next_pattern(self, b, cx);
    }

    /// `ccBoss08::ExecPatternIndex(tbl, i)` (0x003adae0): 1 the wave (6), 14
    /// the magic (20) with the countdowns held, 15 n the seeds (21, `n` kept);
    /// the rest the base's.
    fn exec_pattern_index(&mut self, b: &mut BossBase, cx: &mut Cx, tbl: &[i32], i: i32) -> i32 {
        let x = self;
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
        match pat {
            15 => {
                x.entry_seed_num = word(tbl, s);
                s += 1;
                b.change_action(cx, act::SEED, 3, true);
            }
            14 => {
                x.pause_cnt_down = true;
                b.change_action(cx, act::MAGIC, 3, true);
            }
            1 => b.change_action(cx, act::WAVE, 3, true),
            _ => return base_exec_pattern_index(x, b, cx, tbl, i),
        }
        s
    }
}

/// `ccBoss08::Think` (0x003ad460).
fn think(b: &mut BossBase, x: &mut First, cx: &mut Cx) {
    match b.act_num {
        act::NEUTRAL => on_neutral(b, x, cx),
        act::DAMAGE0 | act::DAMAGE1 => b.change_action(cx, act::NEUTRAL, 0, false),
        // The flash, the pollen and the body press: no pattern word and
        // no call reaches them (the flash's one way in is
        // `_ChangeVibeType(4)`, which nothing calls).
        act::FLASH | act::POLLEN | act::BODY_PRESS => {}
        act::WAVE => on_wave(b, x, cx),
        act::DIE => on_die(b, x, cx),
        act::MAGIC => on_magic(b, x, cx),
        act::ESCAPE => on_escape(b, x, cx),
        act::SEED => on_seed(b, x, cx),
        _ => b.change_action(cx, act::NEUTRAL, 1, true),
    }
}

/// `OnThinkNeutral` (0x003adce0): still; the wait (a count up to
/// `stopTime`; -1 for ever; -2 until its skill is over), then the next
/// pattern at the target.
fn on_neutral(b: &mut BossBase, x: &mut First, cx: &mut Cx) {
    b.move_spd = 0;
    b.move_vector = VF0;
    let me = cx.me;
    if b.stop_time > 0 {
        let c = b.stop_counter;
        b.stop_counter += 1;
        if b.stop_time >= c {
            return;
        }
        b.stop_time = 0;
    } else if b.stop_time == -1 {
        return;
    } else if b.stop_time == -2 {
        if cx.scene.chars[me].skill_status != 0 {
            return;
        }
        b.stop_time = 0;
    }
    let t = b.select(cx, 0);
    if !cx.valid(t) {
        return;
    }
    b.calc_target_info(cx);
    x.change_next_pattern(b, cx);
}

/// The target picked for an act (`SelectTarget(0)`), or the 2-1 wait when
/// it is gone or down.
fn pick(b: &mut BossBase, x: &mut First, cx: &mut Cx) -> bool {
    let t = b.select(cx, 0);
    if !cx.target_ok(t) {
        x.exec_pattern(b, cx, 2, -1);
        return false;
    }
    b.calc_target_info(cx);
    true
}

/// A leap from where it stands at `dirc`: 1000 out, the control point 500
/// out and 1000 up (`m_jumpData`), its time 0.
fn leap(b: &mut BossBase, x: &mut First, cx: &mut Cx, dirc: F) {
    let me = cx.me;
    let pos = cx.scene.chars[me].pos;
    x.jump.sp = pos;
    let c = piney_data::libm::cosf(dirc);
    let s = piney_data::libm::sinf(dirc);
    let mut p = crate::boss::w2p(cx, pos);
    p[0] = ee::add(p[0], ee::mul(0x447a_0000, c));
    p[1] = ee::add(p[1], ee::mul(0x447a_0000, s));
    x.jump.ep = crate::boss::p2w(cx, p);
    let mut p = crate::boss::w2p(cx, pos);
    p[0] = ee::add(p[0], ee::mul(0x43fa_0000, c));
    p[1] = ee::add(p[1], ee::mul(0x43fa_0000, s));
    x.jump.cp = crate::boss::p2w(cx, p);
    x.jump.cp[2] = ee::add(x.jump.cp[2], 0x447a_0000);
    x.jump.time = 0;
    b.move_dirc = dirc;
}

/// `_calcJumpPos(pos, jump)` (0x003ade90): the cubic of `sp`, `cp`, `cp`,
/// `ep` (Bernstein, through `ApplyMatrix`) in the player's frame, w 1.
fn calc_jump_pos(cx: &Cx, j: &Jump) -> V4 {
    let t = j.time;
    let u = ee::sub(ONE, t);
    let u2 = ee::mul(u, u);
    let t2 = ee::mul(t, t);
    let k =
        [ee::mul(u2, u), ee::mul(ee::mul(0x4040_0000, u2), t), ee::mul(ee::mul(0x4040_0000, u), t2), ee::mul(t, t2)];
    let m = [
        crate::boss::w2p(cx, j.sp),
        crate::boss::w2p(cx, j.cp),
        crate::boss::w2p(cx, j.cp),
        crate::boss::w2p(cx, j.ep),
    ];
    let mut p = crate::boss::p2w(cx, geom::apply_matrix(&m, k));
    p[3] = ONE;
    p
}

/// `OnThinkWave` (0x003adff0): its heading the leap's; at the target (the
/// wait else) the two squares' generators (SE 228), off the targets; 60
/// frames on, three leaps turning 2 pi / 3 each (SE 223 at the first, 224
/// at their tops), each landing in a WaveShock and a wave (`WaveFuncCB`);
/// then back on the targets and the next pattern.
fn on_wave(b: &mut BossBase, x: &mut First, cx: &mut Cx) {
    let me = cx.me;
    b.dirc[2] = b.move_dirc;
    match b.act_proccess {
        0 => {
            if !pick(b, x, cx) {
                return;
            }
            let pos = cx.pos();
            cx.out(Out::Se3d { se: 228, pos });
            cx.out(Out::Corbenik(Pic::Particles { which: Gen::WaveRing, pos }));
            cx.out(Out::Corbenik(Pic::Particles { which: Gen::WaveSquare, pos }));
            b.erase_cmnd_target(cx);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c < 60 {
                return;
            }
            if !pick(b, x, cx) {
                return;
            }
            b.erase_cmnd_target(cx);
            let d = wrap_hi(ee::add(LEAP_TURN, b.center_dirc));
            leap(b, x, cx, d);
            cx.se3d(223);
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2..=4 => {
            let t = x.jump.time;
            if !ee::lt(t, 0x3ef5_c28f) && ee::le(t, 0x3f05_1eb8) {
                cx.se3d(224);
            }
            let t = ee::add(t, 0x3d23_d70a);
            x.jump.time = t;
            if !ee::lt(t, ONE) {
                let d = wrap_hi(ee::add(LEAP_TURN, b.move_dirc));
                leap(b, x, cx, d);
                let (pos, dirc) = (cx.pos(), b.dirc);
                b.effect_at(cx, EffKind::WaveShock, pos, dirc);
                let w = eff::Wave::new(cx, pos);
                make_fx(b, cx, EffKind::CorbenikWave, Fx::Wave(Box::new(w)), pos);
                b.act_proccess += 1;
            }
            let p = calc_jump_pos(cx, &x.jump);
            cx.scene.chars[me].pos = p;
        }
        5 => {
            b.entry_cmnd_target(cx);
            x.change_next_pattern(b, cx);
        }
        _ => {}
    }
}

/// `OnThinkEscape` (0x003ae780): away from the target at 20, turned a
/// quarter off it every 32 frames (toward the centre out of the arena),
/// for 90 frames.
fn on_escape(b: &mut BossBase, x: &mut First, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    b.move_spd = 0x41a0_0000;
    if c & 31 == 0 {
        let off = (f64::from(f32::from_bits(ee::sub(b.target_dirc, b.center_dirc))).abs() as f32).to_bits();
        let mut r =
            if ee::lt(off, HALF_PI) { ee::sub(b.target_dirc, HALF_PI) } else { ee::add(HALF_PI, b.target_dirc) };
        if ee::le(r, PI) && ee::lt(r, NEG_PI) {
            r = ee::add(r, TWO_PI);
        }
        b.m_act_dirc[2] = r;
    }
    b.move_dirc = geom::set_dirc(b.move_dirc, b.m_act_dirc[2], 256);
    let pp = cx.scene.chars[me].pos_p;
    if !crate::boss::fidchell::valid_area(b, cx, pp) {
        b.m_act_dirc[2] = b.center_dirc;
    }
    match b.act_proccess {
        0 => {
            let t = b.select(cx, 0);
            if !cx.target_ok(t) {
                x.exec_pattern(b, cx, 2, -1);
                return;
            }
            b.calc_target_info(cx);
            let pp = cx.scene.chars[me].pos_p;
            if !crate::boss::fidchell::valid_area(b, cx, pp) {
                b.move_dirc = b.center_dirc;
            }
            b.act_proccess += 1;
        }
        1 => {
            if c == 90 {
                x.change_next_pattern(b, cx);
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

/// `OnThinkSeed` (0x003aea40): SE 51 (note 55) at the clip's frame 25; at
/// 50 SE 61, 35 (note 65) and 256; from 50, every other frame an exited
/// seed thrown (`Order(3)`; the first time the throw's smoke); the next
/// pattern once three are out and the clip is over.
fn on_seed(b: &mut BossBase, x: &mut First, cx: &mut Cx) {
    let pos = cx.pos();
    let f = b.anm.frame();
    if f == 25 {
        cx.out(Out::Se3dNote { se: 51, pos, note: 55 });
    } else if f == 50 {
        cx.out(Out::Se3d { se: 61, pos });
        cx.out(Out::Se3dNote { se: 35, pos, note: 65 });
        cx.out(Out::Se3d { se: 256, pos });
    }
    if (b.anm.frame() as u32) < 50 {
        return;
    }
    let c = b.act_count;
    b.act_count += 1;
    if c & 1 != 0 {
        if let Some(k) = (0..SEEDS).find(|&k| x.seeds[k].b.base.exit != 0) {
            let master = cx.scene.chars[cx.me].pos;
            with_seed(x, k, cx, |sb, sx, cx| seed::order(&mut sb.base, sx, cx, 3, master));
        }
        if b.act_proccess == 0 {
            let mut p = pos;
            p[2] = ee::add(p[2], 0x43fa_0000);
            cx.out(Out::Corbenik(Pic::Particles { which: Gen::SeedShot, pos: p }));
        }
        b.act_proccess += 1;
    }
    if b.act_proccess >= 3 && b.anm_status != 0 {
        x.change_next_pattern(b, cx);
    }
}

/// `OnThinkDie` (0x003af4b0): off the lists with its seeds, the party held,
/// the dead effect; camera 2 2000 out and 50 up from its place at height
/// 200, looking there, the countdowns held; after 60 frames the eye moving
/// 10 a frame; at 150 a white flash and its command gone; at 250 menu 74's
/// stream 106, the body hidden; once the menu closes, out (camera 1).
fn on_die(b: &mut BossBase, x: &mut First, cx: &mut Cx) {
    let me = cx.me;
    match b.act_proccess {
        0 => {
            b.erase_cmnd_target(cx);
            seeds_targetable(x, cx, false);
            cx.out(Out::CursorOff(true));
            b.lock_player(cx, false);
            let pos = cx.pos();
            x.boss_eff = Some(b.effect_at(cx, EffKind::Dead, pos, VF0));
            b.dirc[2] = b.center_dirc;
            let d = b.dirc;
            let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
            let m = geom::rot_matrix_x(&m, d[0]);
            let m = geom::rot_matrix_y(&m, d[1]);
            let m = geom::rot_matrix_z(&m, d[2]);
            let mut view = pos;
            view[2] = 0x4348_0000;
            let v = geom::apply_matrix(&m, [0x44fa_0000, 0, 0x4248_0000, ONE]);
            let eye = crate::boss::p2w(cx, geom::vadd(v, crate::boss::w2p(cx, view)));
            cx.out(Out::CameraChange(2));
            cx.out(Out::CameraPos { cam: 2, pos: eye });
            cx.out(Out::CameraView { cam: 2, view });
            cx.out(Out::CameraType(3));
            x.pause_cnt_down = true;
            x.stage_eff = -1;
            b.act_proccess += 1;
        }
        1 => {
            let c = b.act_count;
            b.act_count += 1;
            if c >= 60 {
                // `cameraGetPos(camID)` moved 10 across the centre's way.
                let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
                let m = geom::rot_matrix_z(&m, b.center_dirc);
                let v = geom::apply_matrix(&m, [0x4120_0000, 0, 0, ONE]);
                let eye = crate::boss::w2p(cx, cx.cam.active_pos);
                let pos = crate::boss::p2w(cx, geom::vadd(eye, v));
                cx.out(Out::CameraPos { cam: cx.cam.active, pos });
            }
            if c == 150 {
                // The flash's element lands in `m_stageEff`.
                x.stage_eff = 0;
                cx.out(Out::FlashFade { t_in: 100, t_out: 20, hold: Some(1), colour: 0x80ff_ffff });
                cx.out(Out::PanelStatus(3));
                crate::fellow::delete_cmnd(cx.scene, me);
                cx.out(Out::DeleteCmnd);
            }
            if x.boss_eff.is_some() && !b.effects.enabled(x.boss_eff) {
                x.boss_eff = None;
            }
            if c >= 250 {
                cx.out(Out::StreamMenu { stream: 106, mask: 512 });
                b.draw_sw = 0;
                b.act_proccess += 1;
            }
        }
        2 => {
            if cx.env.menu_type != -1 {
                return;
            }
            b.exit = 1;
            crate::fellow::delete_cmnd(cx.scene, me);
            cx.out(Out::DeleteCmnd);
            cx.out(Out::CameraChange(1));
        }
        _ => {}
    }
}

/// `OnThinkMagic` (0x003af990): at the target (the wait else), off the
/// lists with its seeds, the party held, the cinema of skill 252, the dark
/// stage, camera 2 2500 out and 500 up from the target toward the centre;
/// 60 frames on skill 252 at it; then the stage back 160 frames on and
/// the view shaken (a `ccRandF` kick at 95, 115, 135 and 155, damped by
/// -0.9); 15 frames after, all back and the next pattern.
fn on_magic(b: &mut BossBase, x: &mut First, cx: &mut Cx) {
    let me = cx.me;
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 => {
            if !pick(b, x, cx) {
                return;
            }
            b.erase_cmnd_target(cx);
            seeds_targetable(x, cx, false);
            cx.out(Out::CursorOff(true));
            b.lock_player(cx, true);
            cx.out(Out::CinemaSkill(252));
            cx.out(Out::SwitchLayer);
            x.stage_eff = b.begin_stage_effect(cx, DARK, [15, 15, 3000]);
            let tp = b.target_pos;
            let mut view = tp;
            view[2] = ee::add(view[2], 0x42c8_0000);
            let d = get_dirc(crate::boss::w2p(cx, tp), b.center_pos_p);
            let m = geom::rot_matrix_z(&geom::unit_matrix(), NEG_HALF_PI);
            let m = geom::rot_matrix_z(&m, d);
            let v = geom::apply_matrix(&m, [0x451c_4000, 0, 0x43fa_0000, ONE]);
            let eye = crate::boss::p2w(cx, geom::vadd(crate::boss::w2p(cx, tp), v));
            cx.out(Out::CameraChange(2));
            cx.out(Out::CameraPos { cam: 2, pos: eye });
            cx.out(Out::CameraView { cam: 2, view });
            b.act_count = 0;
            b.act_proccess += 1;
        }
        1 => {
            if c != 60 {
                return;
            }
            if let Some(tp) = cx.scene.chars[me].target_char
                && let Some(k) = item::item_skill_request(cx.t, cx.scene, me, tp, 252, 0, false, cx.rand)
            {
                cx.out(Out::Skill(tp, k));
            }
            b.act_count = 0;
            b.act_proccess += 1;
        }
        2 => {
            if c == 160 {
                b.end_stage_effect(cx, x.stage_eff, DARK, 15);
                x.stage_eff = -1;
                b.act_count = 0;
                b.act_proccess += 1;
            }
            if c < 95 {
                return;
            }
            if matches!(c, 95 | 115 | 135 | 155) {
                x.act_pos = VF0;
                x.act_pos[2] = rand_f2(cx.cc, 0x42c8_0000, 0x4248_0000);
            }
            // `cameraGetView` and `cameraSetView` on `camID`, the active
            // camera (QUA gcmn 0x003afd98), not camera 2.
            let view = geom::vadd(cx.cam.active_view, x.act_pos);
            cx.out(Out::CameraView { cam: cx.cam.active, view });
            let v = ee::mul(x.act_pos[2], 0xbf66_6666);
            x.act_pos[2] = v;
            if f64::from(f32::from_bits(v)).abs() < 10.0 {
                x.act_pos[2] = 0;
            }
        }
        3 => {
            if c != 15 {
                return;
            }
            b.unlock_player();
            b.entry_cmnd_target(cx);
            seeds_targetable(x, cx, true);
            cx.out(Out::CursorOff(false));
            cx.out(Out::SwitchLayer);
            cx.out(Out::Cinema(None));
            cx.out(Out::CameraChange(1));
            x.change_next_pattern(b, cx);
            b.act_proccess += 1;
        }
        _ => {}
    }
}

// --- Affect --------------------------------------------------------------------------------

/// `ccBoss08::Affect` (0x003ad0d0): nothing while held off (`actForbid`
/// 2) or the player is locked; 6 and 5 the stop; 9 and 7 a heal (number
/// 20); a blow (1, 3), unless the party is out: its number, SE 195, the
/// mark and the HP down; at 0 the death (each seed's too); else a type-1
/// blow the damage act. No cheat, no drain.
pub(crate) fn affect(b: &mut BossBase, x: &mut First, cx: &mut Cx) {
    let found = take_seeds(cx, x);
    affect_of(b, x, cx);
    put_seeds(cx, x, found);
}

fn affect_of(b: &mut BossBase, x: &mut First, cx: &mut Cx) {
    let me = cx.me;
    if b.act_forbid == 2 || b.lock_player != 0 {
        return;
    }
    let (t, p0, person) = {
        let a = &cx.scene.chars[me].affect;
        (a.ty, a.param[0], a.person)
    };
    let hp = cx.scene.chars[me].hp;
    let mhp = cx.scene.chars[me].max_hp;
    match t {
        6 => b.stop = 0,
        5 => b.stop = 1,
        7 | 9 => {
            cx.out(Out::FlyFont { kind: 20, n: i32::from(p0) });
            cx.scene.chars[me].hp = crate::boss::heal_hp(hp, p0, mhp);
        }
        1 | 3 => {
            if cx.annihilated() || cx.game_over {
                return;
            }
            cx.out(Out::FlyFont { kind: 2, n: i32::from(p0) });
            if p0 < 0 {
                return;
            }
            let pos = cx.pos();
            cx.out(Out::Se3d { se: 195, pos });
            cx.out(Out::HitMark { by: person });
            let mut h = crate::boss::hit_hp(hp, p0, None, mhp);
            if h <= 0 {
                h = 0;
                // `m_pauseCntDownFlag`: the seeds' counts held (not
                // `m_deadFlag`, which nothing sets).
                x.pause_cnt_down = true;
                for k in 0..SEEDS {
                    if x.seeds[k].b.base.exit == 0 {
                        with_seed(x, k, cx, |sb, _, cx| sb.base.change_action(cx, seed::act::DEAD, 2, true));
                    }
                }
                b.move_spd = 0;
                b.move_vector = VF0;
                b.change_action(cx, act::DIE, 2, true);
            } else {
                // `m_preFlash` is the flash's alone: never set.
                x.dmg_count += 1;
                if t == 1 {
                    let n = if cx.env.count & 1 != 0 { act::DAMAGE0 } else { act::DAMAGE1 };
                    b.change_action(cx, n, 0, true);
                }
            }
            cx.scene.chars[me].hp = h;
        }
        _ => {}
    }
}

/// `ccBoss08::SendMessage(seed, 0)` (0x003ad9d0): each other seed within
/// boss skill 31's area (`targetRange`) of `seed` not yet bursting told to burst
/// (`Order(5)`).
pub(crate) fn send_message(x: &mut First, from: usize, at: V4, cx: &mut Cx) {
    let reach = cx.t.boss_skills.get(31).map_or(0, |s| s.target_range);
    for k in 0..SEEDS {
        if k == from {
            continue;
        }
        let q = cx.scene.chars[x.seeds[k].me].pos_p;
        if ee::le(crate::boss::dist(cx, at, q), reach) && !x.seeds[k].x.explode {
            with_seed(x, k, cx, |sb, sx, cx| seed::order(&mut sb.base, sx, cx, 5, VF0));
        }
    }
}
