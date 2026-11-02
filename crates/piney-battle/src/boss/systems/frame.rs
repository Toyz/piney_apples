//! `ccBoss::Main`'s blocks (0x0045c270) on the `ccBoss` members, which a
//! def's [`super::Step`]s run in its class's order, and `ccBoss::Draw`'s
//! body ([`BossBase::body_draw`]).

use piney_data::field::ee;
use piney_data::libm;

use super::super::{BodyDraw, BossBase, Cx, Out, dist, p2w, w2p};
use crate::enemy_ai::get_dirc;
use crate::geom;

impl BossBase {
    /// `ccBoss::Main` (0x0045c270) with the class's `Think` and `Action` in
    /// `class`: the building blocks below in `ccBoss`'s order.
    pub(crate) fn base_main(&mut self, cx: &mut Cx, class: &mut dyn FnMut(&mut BossBase, &mut Cx)) {
        if self.exit != 0 {
            return;
        }
        self.prologue(cx);
        self.tick_stop();
        class(self, cx);
        self.lock_party(cx);
        self.move_(cx);
        self.forward_and_draw(cx);
    }

    /// `CalcReal`, `CalcTargetInfo` and the centre ([`Self::centre`]).
    pub(crate) fn prologue(&mut self, cx: &mut Cx) {
        self.calc_real(cx);
        self.calc_target_info(cx);
        self.centre(cx);
    }

    /// The centre on the plane, its distance and heading from the boss.
    pub(crate) fn centre(&mut self, cx: &mut Cx) {
        self.center_pos_p = w2p(cx, self.center_pos);
        let pp = cx.scene.chars[cx.me].pos_p;
        self.center_dist = dist(cx, pp, self.center_pos_p);
        self.center_dirc = get_dirc(pp, self.center_pos_p);
    }

    /// `stop`'s count: 60 frames still, then let go.
    pub(crate) fn tick_stop(&mut self) {
        if self.stop != 0 {
            self.stop_count += 1;
            if self.stop_count >= 61 {
                self.stop_count = 0;
                self.stop = 0;
            }
            self.move_spd = 0;
        } else {
            self.stop_count = 0;
        }
    }

    /// While `lockPlayer`, the party held (`EntryAffect 5`) and a reserved
    /// menu forbid set; else a reserved release made. True when it made
    /// the release.
    pub(crate) fn lock_party(&mut self, cx: &mut Cx) -> bool {
        let forbid = cx.env.menu_forbid;
        self.lock_party_as(cx, forbid)
    }

    /// [`Self::lock_party`] with the menu's forbid as the class reads it.
    pub(crate) fn lock_party_as(&mut self, cx: &mut Cx, menu_forbid: i16) -> bool {
        if self.lock_player != 0 {
            for m in cx.party.members.into_iter().flatten() {
                if cx.valid(Some(m)) {
                    cx.affect(m, 5, 32767);
                }
            }
            if self.reserve_forbid_menu == 1 && menu_forbid == 0 && cx.env.menu_type == -1 {
                cx.out(Out::MenuForbid { on: true, chat_except: self.reserve_forbid_chat_except != 0 });
                self.reserve_forbid_menu = 0;
                self.reserve_forbid_chat_except = 0;
            }
            false
        } else if self.reserve_forbid_menu == -1 && cx.env.menu_type == -1 {
            cx.out(Out::MenuForbid { on: false, chat_except: false });
            self.reserve_forbid_menu = 0;
            self.reserve_forbid_chat_except = 0;
            true
        } else {
            false
        }
    }

    /// `ccBoss::PreDrawAnm` (the clip forward, `anmStatus` its end) and
    /// `Draw`, when drawn.
    pub(crate) fn forward_and_draw(&mut self, cx: &mut Cx) {
        if self.draw_sw != 0 {
            self.anm_status = i8::from(self.anm.forward());
            self.draw(cx);
        }
    }

    /// `ccBoss::Move` (0x0045c600).
    pub(crate) fn move_(&mut self, cx: &mut Cx) {
        let me = cx.me;
        let mut pp = w2p(cx, cx.scene.chars[me].pos);
        if !geom::eq(0, self.move_spd) {
            let s = ee::mul(self.move_spd, libm::sinf(self.move_dirc));
            pp[0] = ee::add(pp[0], s);
            let c = ee::mul(self.move_spd, libm::cosf(self.move_dirc));
            pp[1] = ee::sub(pp[1], c);
        }
        pp = geom::vadd(pp, self.move_vector);
        let mut pos = p2w(cx, pp);
        if self.body_hit_sw != 0
            && let Some(mut off) = (cx.collide)(pos)
        {
            off[2] = 0;
            pos = geom::vadd(pos, off);
            pp = geom::vadd(pp, off);
        }
        let ch = &mut cx.scene.chars[me];
        ch.pos_p = pp;
        ch.pos = pos;
    }

    /// `ccBoss::Draw`'s body: where its character stands, at its heading,
    /// while `drawSW` holds.
    pub(crate) fn body_draw(&self, cx: &Cx) -> BodyDraw {
        BodyDraw { pos: cx.scene.chars[cx.me].pos, dirc: self.dirc, shown: self.draw_sw != 0, ..BodyDraw::default() }
    }
}
