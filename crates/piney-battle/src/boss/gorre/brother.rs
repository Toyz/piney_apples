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
use crate::geom::{self, V4};
use crate::param::cond;

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
/// (`TransPosB2W`'d in the constructor) and this brother's id (0 or 1);
/// its heading starts as Gorre's own.
pub fn new(cx: &mut Cx, me: usize, master_me: usize, offset: V4, dirc: V4, id: i32) -> Part<Brother> {
    let mut b = kyvia::plain_boss();
    b.exit = 0;
    b.draw_sw = 1;
    b.body_hit_sw = 1;
    cx.scene.chars[me].pos = cx.scene.chars[master_me].pos;
    b.dirc = dirc;
    b.act_num = act::NEUTRAL;
    Part { b: Box::new(b), x: Box::new(Brother { offset, id, master_me }), me }
}

/// `ccBoss05Brother::Main` (OUT gcmn 0x0049ca20): the base's `Main`
/// (`ccBoss::Main`); Gorre's own frame runs each brother's under it.
pub(super) fn frame(gb: &mut Boss, gx: &mut Gorre, k: usize, cx: &mut Cx) {
    let me = gx.brothers[k].me;
    let old = cx.me;
    cx.me = me;
    let gorre_dirc = gb.dirc;
    let part = &mut gx.brothers[k];
    if part.b.exit == 0 {
        kyvia::head(&mut part.b, cx);
        think(gb, &mut part.b, &part.x, gorre_dirc, cx);
        mov(&mut part.b, &part.x, gorre_dirc, cx);
        if part.b.draw_sw != 0 {
            part.b.anm_status = i8::from(part.b.anm.forward());
        }
        cx.scene.chars[me].cond[cond::HOLD] = 0;
    }
    cx.me = old;
}

/// `ccBoss05Brother::Move` (OUT gcmn 0x0049c7e0): its place kept at its
/// formation offset off Gorre, turned by Gorre's own heading (its own
/// speed added on top, the base's way).
fn mov(b: &mut Boss, x: &Brother, gorre_dirc: V4, cx: &mut Cx) {
    let me = cx.me;
    let m = geom::rot_matrix_z(&geom::unit_matrix(), gorre_dirc[2]);
    let off = geom::apply_matrix(&m, x.offset);
    let base = cx.scene.chars[x.master_me].pos;
    let mut pos = geom::vadd(base, off);
    if !geom::eq(0, b.move_spd) {
        let s = ee::mul(b.move_spd, piney_data::libm::sinf(b.move_dirc));
        let c = ee::mul(b.move_spd, piney_data::libm::cosf(b.move_dirc));
        pos[0] = ee::add(pos[0], s);
        pos[1] = ee::sub(pos[1], c);
    }
    let pp = super::super::w2p(cx, pos);
    let ch = &mut cx.scene.chars[me];
    ch.pos = pos;
    ch.pos_p = pp;
}

/// `ccBoss05Brother::Think` (OUT gcmn 0x0049ca90).
fn think(gb: &mut Boss, b: &mut Boss, x: &Brother, gorre_dirc: V4, cx: &mut Cx) {
    match b.act_num {
        act::WAVE | act::EPITAPH_WAVE => on_wave(b, gorre_dirc),
        act::EPITAPH => on_neutral(b, gorre_dirc),
        act::DEAD => on_dead(gb, b, x, cx),
        act::TORNADE => on_tornade(gb, b),
        _ => on_neutral(b, gorre_dirc),
    }
}

/// `OnThinkNeutral` / `OnThinkEpitaph` (0x0049cb50, 0x0049cc50): faces
/// Gorre's own heading.
fn on_neutral(b: &mut Boss, gorre_dirc: V4) {
    b.dirc[2] = gorre_dirc[2];
}

/// `OnThinkWave` / `OnThinkEpitaphWave`: holds its place (Gorre's own
/// `on_wave` fires the wave shock at both brothers' places at frame 30).
fn on_wave(b: &mut Boss, gorre_dirc: V4) {
    on_neutral(b, gorre_dirc);
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
pub(super) fn order(b: &mut Boss, cx: &mut Cx, on: i32) {
    let act = i16::try_from(on).unwrap_or(0);
    b.change_action(cx, act, 1, true);
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
