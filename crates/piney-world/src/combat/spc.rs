//! The party's characters in the battle's state as the event instructions
//! and the AI's manual paths of [`crate::party`] and [`crate::ai`] see
//! them: a [`SpcRec`] copies what a [`SpcRef`] lends out of the battle's
//! [`Scene`] (`ccChar`, its flag word), the [`Crew`] (the character's `Spc`
//! and `ccAI`) and the cast (`transDist`), and writes it back after.
//!
//! The copies are exact: every field a [`SpcRef`] holds is one the battle
//! keeps (`piney_battle::chara::SpcChar::flags`' bits for `dispSW` ...
//! `ghostFlag` and `recallFlag`; `condition.dead`; `partyFlag`), and the
//! body a copy of the character's `bodyHit` ([`CharHit`]) under the key
//! [`body_id`] gives it in [`Hits::chars`].

use piney_battle::chara::spc_flag;
use piney_battle::party_ai::{Ai as BAi, Crew};
use piney_battle::scene::Scene;
use piney_battle::world::CharHit;

use crate::ai::{Ai, SpcRef};
use crate::ee::{F, V4};
use crate::hit::{self, Hits};
use crate::party::SpcChars;

/// `+0xe0` bits the battle's flag word holds beyond [`spc_flag`].
pub mod flag {
    pub const DISP: u32 = 1 << 1;
    pub const RUN: u32 = 1 << 5;
    pub const RECALL: u32 = 1 << 12;
}

/// The key of a battle character's body in [`Hits::chars`]: Kite's is the
/// player's ([`hit::PLAYER_ID`]), everyone else's its scene index past
/// 0x1000 (clear of the town's keys).
pub fn body_id(who: usize, kite: Option<usize>) -> u32 {
    if Some(who) == kite { hit::PLAYER_ID } else { 0x1000 + who as u32 }
}

/// A `ccCharHit` as the collision list keeps it.
pub fn to_body(who: usize, kite: Option<usize>, h: &CharHit, manual: bool) -> hit::Body {
    hit::Body {
        pos: h.pos,
        radius: h.radius,
        height: h.height,
        mask2: h.mask2,
        offset: h.offset,
        sw: h.sw,
        mask: h.mask,
        kind: h.kind,
        attribute: h.attribute,
        id: body_id(who, kite),
        manual,
    }
}

/// And back.
pub fn from_body(b: &hit::Body) -> CharHit {
    CharHit {
        sw: b.sw,
        mask: b.mask,
        mask2: b.mask2,
        kind: b.kind,
        radius: b.radius,
        height: b.height,
        pos: b.pos,
        offset: b.offset,
        attribute: b.attribute,
    }
}

/// piney-world's `ccAI` (the manual paths' fields) from the battle's.
fn ai_of(a: &BAi) -> Ai {
    Ai {
        manual_sw: a.manual_sw,
        follow_sw: a.follow_sw,
        talk_flag: a.talk_flag,
        run_flag: a.run_flag,
        remote_flag: a.remote_flag,
        go_back_flag: a.go_back_flag,
        invite_flag: a.invite_flag,
        self_flag: a.self_flag,
        battle_flag: a.battle_flag,
        target_flag: a.target_flag,
        chat_cmd_flag: a.chat_cmd_flag,
        first_time: a.first_time,
        chat_request: a.chat_request,
        skill_mask: a.skill_mask,
        strategy_cmd: a.strategy_cmd,
        strategy: a.strategy,
        mode: a.mode,
        mode_old: a.mode_old,
        count: a.count,
        remote_cmd: a.remote_cmd,
        chat_cmd: a.chat_cmd,
        arrival_chat_cnt: a.arrival_chat_cnt,
        g_deg: a.g_deg,
        g_rot_sp: a.g_rot_sp,
        attack_cycle: a.attack_cycle,
        detour_cnt: a.detour_cnt,
        g_point: a.g_point,
        g_pos: a.g_pos,
    }
}

/// And back into the battle's.
fn ai_into(w: &Ai, a: &mut BAi) {
    a.manual_sw = w.manual_sw;
    a.follow_sw = w.follow_sw;
    a.talk_flag = w.talk_flag;
    a.run_flag = w.run_flag;
    a.remote_flag = w.remote_flag;
    a.go_back_flag = w.go_back_flag;
    a.invite_flag = w.invite_flag;
    a.self_flag = w.self_flag;
    a.battle_flag = w.battle_flag;
    a.target_flag = w.target_flag;
    a.chat_cmd_flag = w.chat_cmd_flag;
    a.first_time = w.first_time;
    a.chat_request = w.chat_request;
    a.skill_mask = w.skill_mask;
    a.strategy_cmd = w.strategy_cmd;
    a.strategy = w.strategy;
    a.mode = w.mode;
    a.mode_old = w.mode_old;
    a.count = w.count;
    a.remote_cmd = w.remote_cmd;
    a.chat_cmd = w.chat_cmd;
    a.arrival_chat_cnt = w.arrival_chat_cnt;
    a.g_deg = w.g_deg;
    a.g_rot_sp = w.g_rot_sp;
    a.attack_cycle = w.attack_cycle;
    a.detour_cnt = w.detour_cnt;
    a.g_point = w.g_point;
    a.g_pos = w.g_pos;
}

/// One character's `ccSpcChar` members, copied out of the battle.
#[derive(Clone, Debug)]
pub struct SpcRec {
    pub who: usize,
    pub ai: Option<Ai>,
    pub pos: V4,
    pub dirc: V4,
    pub dead: i16,
    pub skill_id: i16,
    pub skill_status: i16,
    pub transparency: F,
    pub set_transparency: F,
    pub trans_dist: bool,
    pub disp: bool,
    pub restraint: bool,
    pub move_flag: bool,
    pub stop_flag: bool,
    pub run_flag: bool,
    pub ghost: bool,
    pub no_death: bool,
    pub recall: bool,
    pub party_flag: i8,
    pub act: i16,
    pub act_old: i16,
    pub anm_flag: i16,
    pub act_cnt: i16,
    pub transfer_lag: i16,
    pub cloak: F,
    pub hit: hit::Body,
    pub listed: bool,
}

impl SpcRec {
    /// `who`'s members as the battle holds them; `trans_dist` is the
    /// cast's (`ccChar` +0x90).
    pub fn read(scene: &Scene, crew: &Crew, who: usize, kite: Option<usize>, trans_dist: bool) -> SpcRec {
        let c = &scene.chars[who];
        let s = crew.spc.get(&who).copied().unwrap_or_default();
        let ai = crew.ais.get(&who).map(ai_of);
        let f = c.spc_char.flags;
        let manual = ai.as_ref().is_some_and(|a| a.manual_sw);
        SpcRec {
            who,
            pos: c.pos,
            dirc: s.dirc,
            dead: c.cond[piney_battle::param::cond::DEAD],
            skill_id: c.skill_id,
            skill_status: c.skill_status,
            transparency: s.transparency,
            set_transparency: s.set_transparency,
            trans_dist,
            disp: f & flag::DISP != 0,
            restraint: f & spc_flag::RESTRAINT != 0,
            move_flag: f & spc_flag::MOVE != 0,
            stop_flag: f & spc_flag::STOP != 0,
            run_flag: f & flag::RUN != 0,
            ghost: f & spc_flag::GHOST != 0,
            no_death: c.no_death,
            recall: f & flag::RECALL != 0,
            party_flag: (((c.party_flag & 7) << 29) >> 29) as i8,
            act: c.spc_char.act_num,
            act_old: c.spc_char.act_num_old,
            anm_flag: c.anm_flag,
            act_cnt: s.act_cnt,
            transfer_lag: s.transfer_lag,
            cloak: c.spc_char.cloak,
            hit: to_body(who, kite, &s.body_hit, manual),
            listed: scene.listed(who),
            ai,
        }
    }

    /// The members lent as a [`SpcRef`].
    pub fn spc_ref(&mut self) -> SpcRef<'_> {
        SpcRef {
            ai: self.ai.as_mut(),
            pos: &mut self.pos,
            dirc: &mut self.dirc,
            dead: &mut self.dead,
            skill_id: &mut self.skill_id,
            skill_status: &mut self.skill_status,
            transparency: &mut self.transparency,
            set_transparency: &mut self.set_transparency,
            trans_dist: &mut self.trans_dist,
            disp: &mut self.disp,
            restraint: &mut self.restraint,
            move_flag: &mut self.move_flag,
            stop_flag: &mut self.stop_flag,
            run_flag: &mut self.run_flag,
            ghost: &mut self.ghost,
            no_death: &mut self.no_death,
            recall: &mut self.recall,
            party_flag: &mut self.party_flag,
            act: &mut self.act,
            act_old: &mut self.act_old,
            anm_flag: &mut self.anm_flag,
            act_cnt: &mut self.act_cnt,
            transfer_lag: &mut self.transfer_lag,
            cloak: &mut self.cloak,
            hit: &mut self.hit,
            listed: &mut self.listed,
            velocity: 0,
        }
    }

    /// The members back into the battle: the flag word's bits, the
    /// character, its `Spc` (and the `actNum` and flag copies the party AI
    /// reads there), its `ccAI`, the command list (`ccEntryCmnd` /
    /// `ccDeleteCmnd`). Returns `transDist`.
    pub fn write(&self, scene: &mut Scene, crew: &mut Crew) -> bool {
        let who = self.who;
        let c = &mut scene.chars[who];
        c.pos = self.pos;
        c.cond[piney_battle::param::cond::DEAD] = self.dead;
        c.skill_id = self.skill_id;
        c.skill_status = self.skill_status;
        c.no_death = self.no_death;
        c.party_flag = i32::from(self.party_flag) & 7;
        c.spc_char.act_num = self.act;
        c.spc_char.act_num_old = self.act_old;
        c.anm_flag = self.anm_flag;
        c.spc_char.cloak = self.cloak;
        c.spc_char.hit_enabled = self.hit.sw;
        let bits = [
            (flag::DISP, self.disp),
            (spc_flag::RESTRAINT, self.restraint),
            (spc_flag::MOVE, self.move_flag),
            (spc_flag::STOP, self.stop_flag),
            (flag::RUN, self.run_flag),
            (spc_flag::GHOST, self.ghost),
            (flag::RECALL, self.recall),
        ];
        for (b, on) in bits {
            if on {
                c.spc_char.flags |= b;
            } else {
                c.spc_char.flags &= !b;
            }
        }
        let s = crew.spc.entry(who).or_default();
        s.dirc = self.dirc;
        s.transparency = self.transparency;
        s.set_transparency = self.set_transparency;
        s.act_cnt = self.act_cnt;
        s.transfer_lag = self.transfer_lag;
        s.body_hit = from_body(&self.hit);
        s.act_num = self.act;
        s.move_flag = self.move_flag;
        s.stop_flag = self.stop_flag;
        s.run_flag = self.run_flag;
        s.ghost = self.ghost;
        if let (Some(w), Some(a)) = (&self.ai, crew.ais.get_mut(&who)) {
            ai_into(w, a);
        }
        match (self.listed, scene.listed(who)) {
            (true, false) => piney_battle::fellow::entry_cmnd(scene, who),
            (false, true) => {
                piney_battle::fellow::delete_cmnd(scene, who);
            }
            _ => {}
        }
        self.trans_dist
    }
}

/// The party's characters in the battle as [`SpcChars`] (registry id to
/// scene index by `ids`): every one copied out when made and written back
/// by [`BattleChars::finish`].
pub struct BattleChars {
    pub recs: Vec<(i32, SpcRec)>,
}

impl BattleChars {
    /// The characters `ids` names ((registry id, scene index)).
    pub fn read(
        scene: &Scene,
        crew: &Crew,
        ids: &[(i32, usize)],
        kite: Option<usize>,
        trans_dist: &dyn Fn(usize) -> bool,
    ) -> BattleChars {
        let recs = ids.iter().map(|&(id, who)| (id, SpcRec::read(scene, crew, who, kite, trans_dist(who)))).collect();
        BattleChars { recs }
    }

    /// Every record written back; `trans_dist` gets each one's `transDist`.
    pub fn finish(self, scene: &mut Scene, crew: &mut Crew, trans_dist: &mut dyn FnMut(usize, bool)) {
        for (_, r) in self.recs {
            let t = r.write(scene, crew);
            trans_dist(r.who, t);
        }
    }
}

impl SpcChars for BattleChars {
    fn spc(&mut self, id: i32) -> Option<SpcRef<'_>> {
        self.recs.iter_mut().find(|(i, _)| *i == id).map(|(_, r)| r.spc_ref())
    }

    fn pos(&self, id: i32) -> Option<V4> {
        self.recs.iter().find(|(i, _)| *i == id).map(|(_, r)| r.pos)
    }
}

/// `hits`' copy of a body made current after a record changed it.
pub fn sync_body(hits: &mut Hits, r: &SpcRec) {
    hits.sync(&r.hit);
}
