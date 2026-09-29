//! `ccBoss05Brother`: one of Gorre's two flanking characters, each its own
//! [`kyvia::Part`] (a full [`Boss`] on its own scene character, run the
//! way Kyvia runs its core and gomoras). Its own act comes from Gorre's
//! `Order` (master to brother); the game's `SendMessage` (brother to
//! master, OUT gcmn 0x0049b680-0x0049c228, a hit or a death reported back)
//! is approximated here by Gorre's own frame polling each brother's HP
//! rather than the full 2984-byte switch.

use piney_data::field::ee;

use super::super::kyvia::{self, Part};
use super::super::{Boss, Cx, Out};
use super::Gorre;
use crate::enemy_ai::get_dirc;
use crate::geom::{self, V4};
use crate::param::cond;

/// `OnThinkNeutral`'s own fixed turn a frame for `m_posB`'s orbit (the
/// only literal float constant it uses): 0x3c8efa35.
const ORBIT_STEP: u32 = 0x3c8e_fa35;

/// Brother's acts (`Think`'s switch, OUT gcmn 0x0049ca90): 4, 12, 13, 14,
/// 21 by name; anything else is `OnThinkNeutral`.
pub mod act {
    pub const NEUTRAL: i16 = 0;
    pub const WAVE: i16 = 4;
    pub const EPITAPH: i16 = 12;
    pub const EPITAPH_WAVE: i16 = 13;
    pub const DEAD: i16 = 14;
    pub const TORNADE: i16 = 21;
}

/// `ccBoss05Brother`'s own members: its formation offset off Gorre
/// (+0x29400, set by `Init`), which of the two it is, and Gorre's own
/// scene index (`m_master`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Brother {
    pub offset: V4,
    pub id: i32,
    pub master_me: usize,
}

/// `ccBoss05Brother::Init` (OUT gcmn 0x0049c410): the master, the offset
/// (kept as `m_posB`, a plain world-space addend: `ccBoss05::TransPosB2W`
/// only adds the master's own place, no heading turn, so the caller's
/// redundant `TransPosB2W` call after `Init` lands on the same place) and
/// this brother's id (0 or 1); its heading starts as Gorre's own.
pub fn new(cx: &mut Cx, me: usize, master_me: usize, offset: V4, dirc: V4, id: i32) -> Part<Brother> {
    let mut b = kyvia::plain_boss();
    b.exit = 0;
    b.draw_sw = 1;
    b.anm_tbl = cx.data.gorre.brother_anims[usize::try_from(id).unwrap_or(0)].clone();
    // `pos_p` is left at its default (0): `Init` never calls
    // `ccTransPosW2P`, only the first `Move` derives it from `pos`.
    cx.scene.chars[me].pos = geom::vadd(cx.scene.chars[master_me].pos, offset);
    b.dirc = dirc;
    let old = cx.me;
    cx.me = me;
    b.change_action(cx, act::NEUTRAL, 0, true);
    cx.me = old;
    Part { b: Box::new(b), x: Box::new(Brother { offset, id, master_me }), me }
}

/// `ccBoss05Brother::Main` (OUT gcmn 0x0049ca20): the base's `Main`
/// (`ccBoss::Main`); Gorre's own frame runs each brother's under it.
pub(super) fn frame(gb: &mut Boss, gx: &mut Gorre, k: usize, cx: &mut Cx) {
    let me = gx.brothers[k].me;
    let old = cx.me;
    cx.me = me;
    let part = &mut gx.brothers[k];
    if part.b.exit == 0 {
        kyvia::head(&mut part.b, cx);
        think(gb, &mut part.b, &mut part.x, cx);
        mov(&mut part.b, &part.x, cx);
        if part.b.draw_sw != 0 {
            part.b.anm_status = i8::from(part.b.anm.forward());
        }
        cx.scene.chars[me].cond[cond::HOLD] = 0;
    }
    cx.me = old;
}

/// `ccBoss05Brother::Move` (OUT gcmn 0x0049c7e0): world place kept at the
/// master's own plus the formation offset (`m_posB`; no heading turn -
/// see [`new`]), the base's own speed layered on top the base's way.
/// `m_posB`'s own drift from `m_moveSpdB`/`m_moveVectorB` (the Wave
/// attack's return and the Tornade spin) is not yet ported: those attacks
/// hold the offset fixed instead.
fn mov(b: &mut Boss, x: &Brother, cx: &mut Cx) {
    let me = cx.me;
    let base = cx.scene.chars[x.master_me].pos;
    let pos = geom::vadd(base, x.offset);
    let mut pos_p = super::super::w2p(cx, pos);
    if !geom::eq(0, b.move_spd) {
        let s = ee::mul(b.move_spd, piney_data::libm::sinf(b.move_dirc));
        let c = ee::mul(b.move_spd, piney_data::libm::cosf(b.move_dirc));
        pos_p[0] = ee::add(pos_p[0], s);
        pos_p[1] = ee::sub(pos_p[1], c);
    }
    pos_p = geom::vadd(pos_p, b.move_vector);
    let pos = super::super::p2w(cx, pos_p);
    let ch = &mut cx.scene.chars[me];
    ch.pos = pos;
    ch.pos_p = pos_p;
}

/// `ccBoss05Brother::Think` (OUT gcmn 0x0049ca90).
fn think(gb: &mut Boss, b: &mut Boss, x: &mut Brother, cx: &mut Cx) {
    match b.act_num {
        act::WAVE | act::EPITAPH_WAVE => on_wave(b, x, cx),
        act::EPITAPH => on_epitaph(b, x, cx),
        act::DEAD => on_dead(gb, b, x, cx),
        act::TORNADE => on_tornade(gb, b),
        _ => on_neutral(b, x, cx),
    }
}

/// Faces the master (`ccGetDirc(pos_p, master.pos_p)`, `ccSetDirc` mode
/// 256): the turn both `OnThinkNeutral` and `OnThinkEpitaph` open with.
fn face_master(b: &mut Boss, x: &Brother, cx: &Cx) {
    let pp = cx.scene.chars[cx.me].pos_p;
    let master_pp = cx.scene.chars[x.master_me].pos_p;
    b.set_dirc(get_dirc(pp, master_pp));
}

/// `m_posB` turned a fixed step (`ORBIT_STEP`) around Z: `OnThinkNeutral`
/// and `OnThinkEpitaph`'s own slow orbit of the master while not stopped
/// or held.
fn orbit(x: &mut Brother) {
    let m = geom::rot_matrix_z(&geom::unit_matrix(), ORBIT_STEP);
    x.offset = geom::apply_matrix(&m, x.offset);
}

fn stopped_or_held(b: &Boss, cx: &Cx) -> bool {
    b.stop != 0 || cx.scene.chars[cx.me].cond[cond::HOLD] != 0
}

/// `OnThinkNeutral` (0x0049cb50): stopped or held, a hit/pause report
/// (`SendMessage` msg 3, not yet ported) and no turn; else faces the
/// master and orbits it.
fn on_neutral(b: &mut Boss, x: &mut Brother, cx: &mut Cx) {
    b.move_spd = 0;
    if stopped_or_held(b, cx) {
        return;
    }
    face_master(b, x, cx);
    orbit(x);
}

/// `OnThinkEpitaph` (0x0049cc50): faces the master regardless, then (not
/// stopped or held) orbits it the same as [`on_neutral`].
fn on_epitaph(b: &mut Boss, x: &mut Brother, cx: &mut Cx) {
    face_master(b, x, cx);
    if !stopped_or_held(b, cx) {
        orbit(x);
    }
}

/// `OnThinkWave` / `OnThinkEpitaphWave`: holds its place facing the
/// master (Gorre's own `on_wave` fires the wave shock at both brothers'
/// places at frame 30). Not the game's own state machine (the return to
/// formation past frame 15, `m_moveSpdB`'s own easing): docs/engine/boss-gorre.md.
fn on_wave(b: &mut Boss, x: &Brother, cx: &Cx) {
    face_master(b, x, cx);
}

/// `OnThinkTornade` (0x0049d320): faces the target Gorre picked.
fn on_tornade(gb: &mut Boss, b: &mut Boss) {
    b.dirc[2] = gb.target_dirc;
}

/// `OnThinkDead` (0x0049df00): the dead effect once, then off the lists.
fn on_dead(_gb: &mut Boss, b: &mut Boss, _x: &Brother, cx: &mut Cx) {
    if b.eff_dead.is_none() && b.act_proccess == 0 {
        b.eff_dead = Some(b.effect(cx, super::EffKind::Dead));
        b.act_proccess += 1;
    } else if !b.effects.enabled(b.eff_dead) && b.exit == 0 {
        b.exit = 1;
        on_exit(cx);
        cx.out(Out::DeleteCmnd);
    }
}

/// `ccBoss05Brother::Order(on)` (OUT gcmn 0x0049e020): Gorre commands this
/// brother into an act; special `on` values (21: its stats reset to
/// Gorre's own row) beyond a plain `ChangeAction` are approximated here as
/// a plain act change.
/// `ccBoss05Brother::Order(on)` (OUT gcmn 0x0049e020): most `on` values
/// change the act with forbid 3, af true; 15 and 0 (a plain neutral) with
/// forbid 1; 4 (`WAVE`) also resets `m_bWaveEnd` and the anm to
/// "ANM_xx11wave" first (not yet ported, harmless: `OnThinkWave`'s own
/// case 0 sets the same anm). 3 and 5 (Gorre's own Kerse and Talk) become
/// act 15, unnamed in [`act`] (a plain watch: `Think`'s default).
pub(super) fn order(b: &mut Boss, cx: &mut Cx, on: i32) {
    match on {
        15 | 0 => b.change_action(cx, 0, 1, true),
        3 | 5 => b.change_action(cx, 15, 3, true),
        _ => b.change_action(cx, i16::try_from(on).unwrap_or(0), 3, true),
    }
}

/// `ccBoss05Brother::Affect` (OUT gcmn 0x0049e220): the core/gomora kinds
/// ([`kyvia::part_affect`]); Gorre's own frame polls each brother's HP for
/// the death report `SendMessage` would otherwise carry. Its own
/// resistant-shield check on a spell (`ccSkillCheckType`,
/// `effResistantShield`) is not yet ported.
pub(super) fn affect(b: &mut Boss, cx: &mut Cx) {
    kyvia::part_affect(b, cx);
}

/// `ccBoss05Brother::OnExit` (OUT gcmn 0x0049e540): off the lists.
pub(super) fn on_exit(cx: &mut Cx) {
    crate::fellow::delete_cmnd(cx.scene, cx.me);
}
