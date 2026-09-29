//! `ccBoss05Brother`: one of Gorre's two flanking characters, each its own
//! [`kyvia::Part`] (a full [`Boss`] on its own scene character, run the
//! way Kyvia runs its core and gomoras). Gorre orders it (`Order`); it
//! reports back through `ccBoss05::SendMessage` ([`Msg`]), which is where
//! the three share one HP.
//! Outbreak's gcmn.prg 0x004ae990-0x004b0b50.

use piney_data::field::ee;
use piney_data::libm;

use super::super::kyvia::{self, Part};
use super::super::{ANM_WAVE, Boss, Class, Cx, EffKind, F, ONE, Out, PI, TWO_PI, VF0};
use super::Gorre;
use crate::enemy_ai::get_dirc;
use crate::geom::{self, V4};
use crate::param::cond;

/// `OnThinkNeutral`'s fixed turn a frame of `m_posB` round the master.
const ORBIT_STEP: F = 0x3c8e_fa35;
/// The Wave's own numbers: out at 50 a frame to past 500, round the master
/// 5 degrees a frame (`m_fRad`), home at 50 to within 300.
const WAVE_SPD: F = 0x4248_0000;
const WAVE_OUT: F = 0x43fa_0000;
const WAVE_HOME: F = 0x4396_0000;
const WAVE_STEP: F = 0x3db2_b8c3;
const WAVE_RADIUS: F = 0x41a0_0000;
const NEG_PI: F = 0xc049_0fdb;
/// The Tornade's fade (1/30 a frame) and spin: `m_fRadSpd` from half a
/// degree up by `m_fRadAccel` a frame to at most a quarter turn.
const FADE_STEP: F = 0x3d08_8889;
const TORNADE_SPD: F = 0x3c0e_fa35;
const TORNADE_ACCEL: F = 0x3b64_c389;
const TORNADE_MAX: F = 0x3f49_0fdb;
/// `Init`'s own `m_fRadSpd`, `m_fRadAccel` (the Tornade sets its own).
const INIT_SPD: F = 0x3db2_b8c3;
const INIT_ACCEL: F = 0x3e32_b8c3;
/// The Tornade's spin: the frames its whoosh (169) sounds on.
const TORNADE_SE: [i16; 26] = [
    32, 52, 72, 87, 99, 111, 121, 130, 139, 146, 152, 157, 162, 167, 171, 175, 179, 182, 186, 190, 193, 197, 201, 204,
    207, 211,
];

/// Brother's acts (`Think`'s switch, OUT gcmn 0x004af020): these six have
/// a function; any other act (Gorre's own, ordered along) does nothing.
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const WAVE: i16 = 4;
    pub const EPITAPH: i16 = 12;
    pub const EPITAPH_WAVE: i16 = 13;
    pub const DEAD: i16 = 14;
    /// `Order(3)`/`Order(5)` (Gorre's Kerse and Talk): a watch, no case.
    pub const WATCH: i16 = 15;
    pub const TORNADE: i16 = 21;
}

/// What a brother reports to Gorre (`SendMessage`'s `msg`, also the index
/// of its own `m_sendMsgFlag`). Msg 1 (a push shared through
/// `m_invOffset`) has a case but no sender in Outbreak's code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Msg {
    /// An `EntryAffect` on the brother (its `Affect`).
    Affect = 0,
    /// Back home from the Wave.
    WaveDone = 2,
    /// Stopped or held (`OnThinkNeutral`, `OnThinkEpitaph`).
    Held = 3,
    /// Its dead effect over.
    Dead = 4,
    /// Faded out: Gorre goes to the target.
    TornadeHidden = 5,
    /// The spin over.
    TornadeDone = 6,
    /// 120 frames into the spin: the spell.
    TornadeSpell = 7,
}

/// `ccBoss05Brother`'s own members (INF DWARF, OUT the same offsets).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Brother {
    /// `m_posB` (+0x29400): the place off Gorre's own, world space.
    pub offset: V4,
    /// `m_prevPosB` (+0x29430): `m_posB` as the Wave's spin started.
    pub prev_offset: V4,
    /// `m_moveSpdB`, `m_moveDircB`: `m_posB`'s own step a frame.
    pub spd: F,
    pub dirc: F,
    /// `m_fRad`, `m_fRadSpd`, `m_fRadAccel`, `m_fRadius`.
    pub rad: F,
    pub rad_spd: F,
    pub rad_accel: F,
    pub radius: F,
    /// `m_bTornadeGenFlag`: the spell sent this Tornade.
    pub tornade_spell: bool,
    /// `m_sendMsgFlag`, by [`Msg`]: Gorre acts on some once both are set.
    pub sent: [bool; 8],
    /// `m_bWaveEnd`.
    pub wave_end: bool,
    pub id: i32,
    /// `m_parent`'s scene index.
    pub master_me: usize,
}

/// `ccBoss05Brother::Init` (OUT gcmn 0x004ae990): the master, `bossTbl`
/// row 40 or 41 by `id`, the place (`m_posB` added to the master's), the
/// wave's clip on `anmw`, the neutral.
pub fn new(cx: &mut Cx, me: usize, master_me: usize, offset: V4, dirc: V4, id: i32) -> Part<Brother> {
    let mut b = kyvia::plain_boss();
    b.exit = 0;
    b.draw_sw = 1;
    b.anm_tbl = cx.data.gorre.brother_anims[usize::try_from(id).unwrap_or(0)].clone();
    // `pos_p` is left at 0: only the first `Move` derives it.
    cx.scene.chars[me].pos = geom::apply_matrix(&translate(cx.scene.chars[master_me].pos), offset);
    b.dirc = dirc;
    let old = std::mem::replace(&mut cx.me, me);
    b.change_action(cx, act::NEUTRAL, 0, true);
    cx.me = old;
    b.anm_wave.set(ANM_WAVE, cx.clips);
    let x = Brother { offset, rad_spd: INIT_SPD, rad_accel: INIT_ACCEL, id, master_me, ..Brother::default() };
    Part { b: Box::new(b), x: Box::new(x), me }
}

fn translate(t: V4) -> geom::M4 {
    geom::trans_matrix(&geom::unit_matrix(), t)
}

/// A brother's own call: `cx.me` its character for the span of `f`.
fn as_brother<R>(p: &mut Part<Brother>, cx: &mut Cx, f: impl FnOnce(&mut Boss, &mut Brother, &mut Cx) -> R) -> R {
    let old = std::mem::replace(&mut cx.me, p.me);
    let r = f(&mut p.b, &mut p.x, cx);
    cx.me = old;
    r
}

/// `m_sendMsgFlag[msg] = 1; m_parent->SendMessage(this, msg)`.
fn report(gb: &mut Boss, gx: &mut Gorre, k: usize, msg: Msg, cx: &mut Cx) {
    gx.brothers[k].x.sent[msg as usize] = true;
    let old = std::mem::replace(&mut cx.me, gx.brothers[k].x.master_me);
    super::message::send(gb, gx, k, msg, cx);
    cx.me = old;
}

/// `ccBoss05Brother::Main` (OUT gcmn 0x004aefb0): the base's `Main` with
/// its own `Think` and `Move`, then `hold` cleared and, while Gorre holds
/// the party, a broken protect's count run on.
pub(super) fn frame(gb: &mut Boss, gx: &mut Gorre, k: usize, cx: &mut Cx) {
    let me = gx.brothers[k].me;
    let old = std::mem::replace(&mut cx.me, me);
    if gx.brothers[k].b.exit == 0 {
        kyvia::head(&mut gx.brothers[k].b, cx);
        think(gb, gx, k, cx);
        let p = &mut gx.brothers[k];
        mov(&mut p.b, &mut p.x, cx);
        if p.b.draw_sw != 0 {
            p.b.anm_status = i8::from(p.b.anm.forward());
        }
    }
    cx.scene.chars[me].cond[cond::HOLD] = 0;
    if gb.lock_player != 0
        && let Some(f) = cx.scene.chars[me].foe_state_mut()
        && f.pp_count > 0
    {
        f.pp_count += 1;
    }
    cx.me = old;
}

/// `ccBoss05Brother::Move` (OUT gcmn 0x004aed70): `m_posB` stepped by
/// `m_moveSpdB` and moved by `m_moveVectorB` (never set off (0, 0, 0, 1)),
/// the place Gorre's plus it, then the base's own speed on top. The
/// `m_invOffset` pull after it waits on msg 1, which nothing sends.
fn mov(b: &mut Boss, x: &mut Brother, cx: &mut Cx) {
    if !geom::eq(0, x.spd) {
        x.offset[0] = ee::add(x.offset[0], ee::mul(x.spd, libm::sinf(x.dirc)));
        x.offset[1] = ee::sub(x.offset[1], ee::mul(x.spd, libm::cosf(x.dirc)));
    }
    x.offset = geom::apply_matrix(&translate(VF0), x.offset);
    let pos = geom::apply_matrix(&translate(cx.scene.chars[x.master_me].pos), x.offset);
    let mut pos_p = super::super::w2p(cx, pos);
    if !geom::eq(0, b.move_spd) {
        pos_p[0] = ee::add(pos_p[0], ee::mul(b.move_spd, libm::sinf(b.move_dirc)));
        pos_p[1] = ee::sub(pos_p[1], ee::mul(b.move_spd, libm::cosf(b.move_dirc)));
    }
    pos_p = geom::vadd(pos_p, b.move_vector);
    let pos = super::super::p2w(cx, pos_p);
    let ch = &mut cx.scene.chars[cx.me];
    ch.pos = pos;
    ch.pos_p = pos_p;
}

/// The Wave's two kinds: Gorre's own (act 4) and its Epitaph's (act 13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Wave {
    Plain,
    Epitaph,
}

/// `ccBoss05Brother::Think` (OUT gcmn 0x004af020).
fn think(gb: &mut Boss, gx: &mut Gorre, k: usize, cx: &mut Cx) {
    match gx.brothers[k].b.act_num {
        act::NEUTRAL => on_neutral(gb, gx, k, cx),
        act::EPITAPH => on_epitaph(gb, gx, k, cx),
        act::WAVE => on_wave(gb, gx, k, Wave::Plain, cx),
        act::EPITAPH_WAVE => on_wave(gb, gx, k, Wave::Epitaph, cx),
        act::TORNADE => on_tornade(gb, gx, k, cx),
        act::DEAD => on_dead(gb, gx, k, cx),
        _ => {}
    }
}

/// The smoothed turn (`ccSetDirc` mode 256) toward the master.
fn face_master(b: &mut Boss, x: &Brother, cx: &Cx) {
    let pp = cx.scene.chars[cx.me].pos_p;
    let master_pp = cx.scene.chars[x.master_me].pos_p;
    b.set_dirc(get_dirc(pp, master_pp));
}

/// `m_posB` turned [`ORBIT_STEP`] round Z, its w put back to 1.
fn orbit(x: &mut Brother) {
    let m = geom::rot_matrix_z(&geom::unit_matrix(), ORBIT_STEP);
    x.offset = geom::apply_matrix(&m, x.offset);
    x.offset[3] = ONE;
}

/// `OnThinkNeutral` (OUT gcmn 0x004af0e0): stopped or held, a report
/// ([`Msg::Held`]); else a turn toward the master and a step round it.
fn on_neutral(gb: &mut Boss, gx: &mut Gorre, k: usize, cx: &mut Cx) {
    gx.brothers[k].b.move_spd = 0;
    if super::stopped_or_held(&gx.brothers[k].b, cx) {
        return report(gb, gx, k, Msg::Held, cx);
    }
    let p = &mut gx.brothers[k];
    face_master(&mut p.b, &p.x, cx);
    orbit(&mut p.x);
}

/// `OnThinkEpitaph` (OUT gcmn 0x004af1f0): the turn first, whatever
/// holds it; then as [`on_neutral`].
fn on_epitaph(gb: &mut Boss, gx: &mut Gorre, k: usize, cx: &mut Cx) {
    let p = &mut gx.brothers[k];
    p.b.move_spd = 0;
    face_master(&mut p.b, &p.x, cx);
    if super::stopped_or_held(&gx.brothers[k].b, cx) {
        return report(gb, gx, k, Msg::Held, cx);
    }
    orbit(&mut gx.brothers[k].x);
}

/// `OnThinkWave` (OUT gcmn 0x004afe10) and `OnThinkEpitaphWave`
/// (0x004af300): out past 500, 30 frames still, half a turn round the
/// master, the wave's strike (skill 18 at frame 55 of the clip), then home
/// to within 300 and [`Msg::WaveDone`]. The Epitaph's has no sound on the
/// way out, a slower trail, its own clips and no turn or body hit after.
fn on_wave(gb: &mut Boss, gx: &mut Gorre, k: usize, kind: Wave, cx: &mut Cx) {
    let (me, master) = (gx.brothers[k].me, gx.brothers[k].x.master_me);
    let (pp, mpp) = (cx.scene.chars[me].pos_p, cx.scene.chars[master].pos_p);
    let to_master = get_dirc(pp, mpp);
    let from_master = get_dirc(mpp, pp);
    let dist = super::dist(cx, pp, mpp);
    let p = &mut gx.brothers[k];
    let (b, x) = (&mut p.b, &mut p.x);
    match b.act_proccess {
        0 => {
            x.radius = WAVE_RADIUS;
            x.rad = 0;
            x.prev_offset = x.offset;
            b.anm_wave.set(ANM_WAVE, cx.clips);
            b.erase_cmnd_target(cx);
            b.body_hit_sw = 0;
            b.act_proccess += 1;
        }
        1 if ee::le(dist, WAVE_OUT) => {
            x.dirc = from_master;
            x.spd = WAVE_SPD;
        }
        1 => {
            x.spd = 0;
            let c = b.act_count;
            b.act_count += 1;
            if c == 30 {
                b.act_proccess += 1;
                b.act_count = 0;
                x.prev_offset = x.offset;
                if kind == Wave::Plain {
                    cx.out(Out::Se3dNote { se: 57, pos: cx.pos(), note: 55 });
                }
            }
        }
        2 => spin(b, x, kind, cx),
        3 => strike(b, kind, cx),
        4 => {
            let c = b.act_count;
            b.act_count += 1;
            if c >= 15 {
                x.spd = WAVE_SPD;
                x.dirc = to_master;
                if ee::le(dist, WAVE_HOME) {
                    x.wave_end = true;
                    report(gb, gx, k, Msg::WaveDone, cx);
                    let p = &mut gx.brothers[k];
                    p.b.act_proccess += 1;
                    p.b.act_count = 0;
                    p.x.spd = 0;
                    p.b.entry_cmnd_target(cx);
                    if kind == Wave::Plain {
                        p.b.body_hit_sw = 1;
                    }
                }
            }
        }
        _ => {}
    }
    if kind == Wave::Plain {
        let p = &mut gx.brothers[k];
        p.b.set_dirc(p.x.dirc);
    }
}

/// The Wave's spin: `m_fRad` on 5 degrees, `m_posB` its start turned by
/// it (wrapped into -pi..pi); past pi, the strike's clip.
fn spin(b: &mut Boss, x: &mut Brother, kind: Wave, cx: &mut Cx) {
    let c = b.act_count;
    b.act_count += 1;
    match kind {
        Wave::Plain => {
            if c & 3 == 0 {
                cx.out(Out::AfterImage);
                if b.act_count == 1 {
                    cx.se3d(229);
                }
            }
        }
        Wave::Epitaph => {
            if c == 5 {
                b.act_count = 0;
                cx.out(Out::AfterImage);
            }
        }
    }
    b.move_spd = 0;
    x.rad = ee::add(x.rad, WAVE_STEP);
    let mut r = x.rad;
    if ee::lt(r, NEG_PI) {
        r = ee::add(r, TWO_PI);
    }
    if !ee::le(r, PI) {
        r = ee::sub(r, TWO_PI);
    }
    if !ee::le(x.rad, PI) {
        b.act_proccess += 1;
        b.act_count = 0;
        let clip = match (kind, x.id) {
            (Wave::Epitaph, _) => "ANM_ex5xact",
            (Wave::Plain, 0) => "ANM_ex51act2",
            (Wave::Plain, _) => "ANM_ex52act2",
        };
        b.anm.set(clip, cx.clips);
    }
    let m = geom::rot_matrix_z(&geom::unit_matrix(), r);
    x.offset = geom::apply_matrix(&m, x.prev_offset);
}

/// The Wave's strike on the clip's frames: sounds at 15 and 35; from 40
/// the wave drawn (`anmw` forward, then at double speed) and skill 18 at
/// 55; once the clip and the wave are both over, the act's own clip. Below
/// 40 the game tests a register it never set there (`this`, nonzero), so
/// the clip's end alone ends it.
fn strike(b: &mut Boss, kind: Wave, cx: &mut Cx) {
    let f = b.anm.frame();
    let over = match f {
        15 => {
            cx.se3d(223);
            true
        }
        35 => {
            cx.se3d(224);
            true
        }
        f if (f as u32) < 40 => true,
        _ => {
            let ended = b.anm_wave.forward();
            b.anm_wave.speed = 512;
            cx.out(Out::DrawWave);
            if b.anm.frame() == 55 {
                let pos = cx.pos();
                b.skill_damage(cx, None, Some(pos), 18);
            }
            ended
        }
    };
    if b.anm_status != 0 && over {
        let slot = if kind == Wave::Plain { 0 } else { 12 };
        if let Some(Some(clip)) = b.anm_tbl.get(slot).cloned() {
            b.anm.set(&clip, cx.clips);
        }
        b.act_proccess += 1;
        b.act_count = 0;
    }
}

/// `OnThinkTornade` (OUT gcmn 0x004af8d0): facing the master, fade out
/// ([`Msg::TornadeHidden`]: Gorre at the target), fade back, then 211
/// frames round the master at a quickening turn ([`Msg::TornadeSpell`] at
/// 120, [`Msg::TornadeDone`] after).
fn on_tornade(gb: &mut Boss, gx: &mut Gorre, k: usize, cx: &mut Cx) {
    let (me, master) = (gx.brothers[k].me, gx.brothers[k].x.master_me);
    let (pp, mpp) = (cx.scene.chars[me].pos_p, cx.scene.chars[master].pos_p);
    let p = &mut gx.brothers[k];
    let (b, x) = (&mut p.b, &mut p.x);
    b.dirc[2] = get_dirc(pp, mpp);
    let c = b.act_count;
    b.act_count += 1;
    match b.act_proccess {
        0 | 1 if c < 15 => {}
        0 => {
            if b.act_count == 16 {
                cx.out(Out::SeNote { se: 57, note: 55 });
            }
            let mut t = ee::sub(b.set_transparency, FADE_STEP);
            if ee::le(t, 0) {
                report(gb, gx, k, Msg::TornadeHidden, cx);
                t = 0;
                let p = &mut gx.brothers[k];
                p.b.act_count = 0;
                p.b.act_proccess += 1;
            }
            let p = &mut gx.brothers[k];
            p.b.transparency = t;
            p.b.set_transparency = t;
            p.x.rad = 0;
            p.x.rad_spd = TORNADE_SPD;
            p.x.rad_accel = TORNADE_ACCEL;
            p.x.tornade_spell = false;
        }
        1 => {
            if b.act_count == 16 {
                cx.out(Out::SeNote { se: 57, note: 55 });
            }
            let mut t = ee::add(b.set_transparency, FADE_STEP);
            if !ee::le(t, ONE) {
                t = ONE;
                b.act_count = 0;
                b.act_proccess += 1;
            }
            b.transparency = t;
            b.set_transparency = t;
        }
        2 if c >= 211 => {
            b.act_proccess += 1;
            report(gb, gx, k, Msg::TornadeDone, cx);
        }
        2 => {
            let m = geom::rot_matrix_z(&geom::unit_matrix(), x.rad);
            x.offset = geom::apply_matrix(&m, x.offset);
            x.rad = wrap_pi(x.rad_spd);
            let mut spd = ee::add(x.rad_spd, x.rad_accel);
            if !ee::lt(spd, TORNADE_MAX) {
                spd = TORNADE_MAX;
            }
            let n = b.act_count;
            if n == 120 && !x.tornade_spell {
                x.tornade_spell = true;
                report(gb, gx, k, Msg::TornadeSpell, cx);
            }
            gx.brothers[k].x.rad_spd = spd;
            if TORNADE_SE.contains(&n) {
                cx.out(Out::SeNote { se: 169, note: 55 });
            }
        }
        _ => {}
    }
}

/// Into -pi..pi by one turn either way.
pub(super) fn wrap_pi(v: F) -> F {
    if ee::lt(v, NEG_PI) {
        ee::add(v, TWO_PI)
    } else if !ee::le(v, PI) {
        ee::sub(v, TWO_PI)
    } else {
        v
    }
}

/// `OnThinkDead` (OUT gcmn 0x004b0490): the dead effect at its place and
/// off the lists; once the effect is gone, [`Msg::Dead`] and `OnExit`.
fn on_dead(gb: &mut Boss, gx: &mut Gorre, k: usize, cx: &mut Cx) {
    let p = &mut gx.brothers[k];
    match p.b.act_proccess {
        0 => {
            p.b.eff_dead = Some(p.b.effect(cx, EffKind::Dead));
            delete_cmnd(cx);
            p.b.act_proccess += 1;
        }
        1 if !p.b.effects.valid(p.b.eff_dead) => {
            report(gb, gx, k, Msg::Dead, cx);
            let p = &mut gx.brothers[k];
            on_exit(&mut p.b, cx);
            p.b.eff_dead = None;
            p.b.act_proccess += 1;
        }
        _ => {}
    }
}

fn delete_cmnd(cx: &mut Cx) {
    crate::fellow::delete_cmnd(cx.scene, cx.me);
    cx.out(Out::DeleteCmnd);
}

/// `ccBoss05Brother::OnExit` (OUT gcmn 0x004b0b00): out, off the lists,
/// no body hit.
fn on_exit(b: &mut Boss, cx: &mut Cx) {
    b.exit = 1;
    delete_cmnd(cx);
    b.body_hit_sw = 0;
}

/// `ccBoss05Brother::Order(on)` (OUT gcmn 0x004b05b0): 0 and 15 a plain
/// neutral (forbid 1); 3 and 5 the watch; 4 `m_bWaveEnd` cleared and the
/// wave's clip set on `anmw` first; anything else that act, forbid 3.
pub(super) fn order(p: &mut Part<Brother>, cx: &mut Cx, on: i16) {
    as_brother(p, cx, |b, x, cx| match on {
        0 | 15 => b.change_action(cx, act::NEUTRAL, 1, true),
        3 | 5 => b.change_action(cx, act::WATCH, 3, true),
        act::WAVE => {
            x.wave_end = false;
            b.anm_wave.set(ANM_WAVE, cx.clips);
            b.change_action(cx, act::WAVE, 3, true);
        }
        _ => b.change_action(cx, on, 3, true),
    });
}

/// `ccBoss05Brother::Affect` (OUT gcmn 0x004b07c0), from an `EntryAffect`
/// on the brother's own character (never inside Gorre's frame): Gorre and
/// the other brother taken out of their characters for its span, so the
/// report ([`Msg::Affect`]) reaches them.
pub(super) fn affect_alone(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    let Class::GorreBrother(x) = std::mem::replace(&mut b.class, Class::Plain) else { return };
    let master = x.master_me;
    let taken = cx.scene.chars.get_mut(master).and_then(|c| c.foe_state_mut()).and_then(|f| f.boss.take());
    let Some(mut gb) = taken else {
        b.class = Class::GorreBrother(x);
        return;
    };
    let mut gx = super::gorre_of(&mut gb);
    let k = usize::from(gx.brother_me[1] == me);
    let (other, found) = super::take_brother(cx, gx.brother_me[1 - k]);
    gx.brothers[1 - k] = other;
    gx.brothers[k] = Part { b: Box::new(std::mem::take(b)), x, me };
    // The report first (a drain only before the brother's Epitaph,
    // `CheckEpitaph`); what Gorre makes of it is Gorre's to show.
    let start = cx.out.len();
    if cx.scene.chars[me].affect.ty != 13 || gx.brothers[k].b.epitaph == 0 {
        report(&mut gb, &mut gx, k, Msg::Affect, cx);
    }
    gb.pending.extend(cx.out.split_off(start));
    local_affect(&mut gx.brothers[k].b, cx);
    let part = std::mem::take(&mut gx.brothers[k]);
    *b = *part.b;
    b.class = Class::GorreBrother(part.x);
    super::put_brother(cx, std::mem::take(&mut gx.brothers[1 - k]), found);
    gb.class = Class::Gorre(gx);
    if let Some(f) = cx.scene.chars[master].foe_state_mut() {
        f.boss = Some(gb);
    }
}

/// `Affect`'s own part after the report: `stop` by 5 and 6; a hit's
/// shield (row 40 against a physical skill, 41 against a spell), number
/// and mark.
fn local_affect(b: &mut Boss, cx: &mut Cx) {
    let me = cx.me;
    let a = &cx.scene.chars[me].affect;
    let (ty, p0, p1, by) = (a.ty, a.param[0], a.param[1], a.person);
    match ty {
        6 => b.stop = 0,
        5 => b.stop = 1,
        1 | 3 => {
            let shield = match cx.scene.chars[me].id() {
                40 => crate::skill::check_type_of(cx.t, i32::from(p1)) == 0,
                41 => crate::skill::check_type_of(cx.t, i32::from(p1)) == 1,
                _ => false,
            };
            if shield {
                cx.out(Out::Shield);
            }
            if !cx.annihilated() && !cx.game_over {
                cx.out(Out::FlyFont { kind: 2, n: i32::from(p0) });
                cx.out(Out::HitMark { by });
            }
        }
        _ => {}
    }
}
