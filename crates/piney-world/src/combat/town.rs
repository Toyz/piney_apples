//! The party in a Root Town through the battle's machinery: the members are
//! the battle's characters ([`Combat::add_member`], area 0), each run by
//! `ccFellow::Main` on its task (priority 50), walking by `ccAI::ActInTown`
//! or, under an event, `ManualControl`. Kite is the town's own
//! ([`crate::player::Player`]); the battle's scene holds a stand-in for him
//! ([`Combat::add_leader`], kept up by [`Combat::mirror_leader`]) with no body
//! or animation of its own. The town gives the navigation [`TownNav`]: the
//! landmarks ([`crate::navi::NaviMap`]) and `naviPointNameTable`'s dummies.

use std::cell::RefCell;

use piney_battle::chara::{Env, spc_flag};
use piney_battle::event::{Event, Who};
use piney_battle::exp::Party;
use piney_battle::fellow;
use piney_battle::param::SpcParam;
use piney_battle::party_ai::{self, Ai as BAi};
use piney_battle::party_motion::{Movement, Share};
use piney_battle::rand::Rng;
use piney_data::save::SaveData;

use super::stage::{Stage, TownNav};
use super::{Combat, Own, Show, bit};
use crate::camera::{CamPad, Camera};
use crate::ee::{self, ONE, V4};
use crate::hit::Hits;

/// `+0xe0` bit 5 `runFlag`.
const RUN: u32 = 1 << 5;

/// What a town frame of the party reads of the town.
pub struct TownTasks<'a> {
    pub hits: &'a mut Hits,
    pub camera: &'a mut Camera,
    pub save: &'a mut SaveData,
    /// The registry and the party (the members' remote command 5).
    pub spcs: &'a mut crate::party::Spcs,
    pub nav: TownNav<'a>,
    /// `ccMenuCtrl::CheckMenuType()`, `forbid`, `eventMng->puppetShow`.
    pub menu_type: i32,
    pub menu_forbid: bool,
    pub puppet_show: bool,
    /// `ccSys.count`.
    pub count: u32,
    /// `checkPartyAnnihilation()`.
    pub annihilated: bool,
    /// `ccGame.server`: the Root Town's server (Mac Anu 0 Delta, Dun
    /// Loireag 1 Theta), for the arrival's line.
    pub server: i32,
}

impl Combat {
    /// Kite's stand-in for the town's party (see the module doc): his
    /// record, on the command list unless `boot` has bit 2, his AI in mode
    /// 1 (manual with bit 2), `arrivalChatCnt` 150; the constructors'
    /// `rand()` draws (`cycle`, then the AI's `count` and message
    /// counters). Returns his scene index.
    pub fn add_leader(&mut self, save: &SaveData, boot: i32, party_flag: i8) -> usize {
        let p = SpcParam::from_save(save, 0);
        let velocity = p.velocity;
        // ccPlayer::ccPlayer's ccSpcChar::ccSpcChar: cycle = rand() >> 3.
        let cycle = self.rand.rand() >> 3;
        let mut ch = piney_battle::Char::pc(p);
        ch.has_ai = true;
        ch.party_flag = i32::from(party_flag) & 7;
        ch.spc_char.cloak = ONE;
        ch.spc_char.flags = bit::DISP | spc_flag::STOP;
        let listed = boot & 4 == 0;
        let who = self.scene.add(ch, if listed { 0 } else { 3 });
        let s = self.crew.spc.entry(who).or_default();
        s.speed = velocity;
        s.speed_rate = ONE;
        s.transparency = ONE;
        s.set_transparency = ONE;
        s.stop_flag = true;
        s.cycle = cycle;
        let strategy = self.spc.party_strategy as i16;
        let mut ai = BAi::new(who, 0, strategy, 150, &mut self.rand);
        ai.level_old = self.scene.chars[who].level();
        self.crew.ais.insert(who, ai);
        self.kite = Some(who);
        // ChangeMode(1, 1), and ManualMode with bit 2 (through the manual
        // paths' view of the AI).
        let mut r = super::spc::SpcRec::read(&self.scene, &self.crew, who, self.kite, true);
        if let Some(a) = r.ai.as_mut() {
            a.change_mode(1);
            if boot & 4 != 0 {
                a.manual_mode(&[0; 4]);
            }
        }
        r.write(&mut self.scene, &mut self.crew);
        self.members.retain(|m| m.0 != 0);
        self.members.insert(0, (0, who));
        who
    }

    /// The town's Kite into his stand-in, before the members' frames: his
    /// place (and `posP`, his own frame's), heading, act, the step's flags,
    /// `dead`, `partyFlag`, display, whether he is on the command list, and
    /// his AI's manual switch.
    pub fn mirror_leader(&mut self, p: &crate::player::Player) {
        let Some(k) = self.kite else { return };
        let listed = p.listed;
        let c = &mut self.scene.chars[k];
        c.pos = p.body.pos;
        c.pos_p = [0, 0, p.body.pos[2], ONE];
        c.spc_char.act_num = p.acts.act;
        c.cond.v[piney_battle::param::cond::DEAD] = p.dead;
        c.party_flag = i32::from(p.party_flag) & 7;
        let mut f = c.spc_char.flags & !(spc_flag::MOVE | spc_flag::STOP | RUN | bit::DISP);
        for (on, b) in [(p.body.move_flag, spc_flag::MOVE), (p.acts.stop_flag, spc_flag::STOP), (p.body.run_flag, RUN)]
        {
            if on {
                f |= b;
            }
        }
        if p.disp {
            f |= bit::DISP;
        }
        c.spc_char.flags = f;
        let s = self.crew.spc.entry(k).or_default();
        s.dirc = p.body.dirc;
        s.move_flag = p.body.move_flag;
        s.run_flag = p.body.run_flag;
        s.stop_flag = p.acts.stop_flag;
        s.act_num = p.acts.act;
        s.now_speed = p.body.now_speed;
        s.speed_rate = p.body.speed_rate;
        s.transparency = p.transparency;
        s.set_transparency = p.set_transparency;
        let on = self.scene.pc_list.contains(&k);
        if listed && !on {
            self.scene.pc_list.insert(0, k);
        } else if !listed && on {
            self.scene.pc_list.retain(|&w| w != k);
        }
        if let (Some(ai), Some(ta)) = (self.crew.ais.get_mut(&k), p.ai.as_ref()) {
            ai.manual_sw = ta.manual_sw;
        }
    }

    /// The party's tasks in a town: `ccThSpc` and `ccThAISystem` (48), then
    /// each member's `ccFellow::Main` (50) over the town's collision, then
    /// where each is drawn (`ccChar::Draw`'s test). Kite's stand-in is the
    /// caller's to keep current ([`Combat::mirror_leader`]).
    pub fn town_frame(&mut self, x: &mut TownTasks) {
        let d = self.data.clone();
        let t = &d.t;
        let Some(kite_i) = self.kite else { return };
        let area = 0;
        let party: Party = self.party;
        self.load_records(x.save);
        // The lines the menus raised after last frame's tasks (a member
        // spoken to: ccAI::Greeting, then its talkFlag cleared), before
        // this frame's.
        self.drain_chats(t, x.save, &party, &[]);
        if let Some(piney_battle::frame::SpcOut::Shout { operation }) =
            self.spc.frame(t, x.save, &party, area, 0, x.puppet_show)
        {
            self.shows.push(Show::Shout(operation));
        }
        self.crew.tick();
        let env = Env {
            plcol: x.save.u8(piney_data::save::offset::PLCOL),
            menu_forbid: i16::from(x.menu_forbid),
            in_battle: 0,
            count: x.count,
            menu_type: x.menu_type,
            sp_regene_speed: false,
            area,
        };
        let game = party_ai::Game {
            in_battle: 0,
            area,
            field: -1,
            field_type: 0,
            field_attr: 0,
            menu_type: x.menu_type,
            event_lock: i32::from(x.puppet_show),
            spc_battle_condition: self.spc.battle_condition,
            party_strategy: self.spc.party_strategy,
            player: Some(kite_i),
            field_24: -1,
            pg_ride_flag: 0,
            area_prev: 0,
            server: x.server,
        };
        let ents: Vec<usize> = Vec::new();
        let bounds = Self::bounds(area, x.hits);
        let registry_num = x.spcs.registry_num;
        let kite_pos = self.scene.chars[kite_i].pos;
        let kite_act = self.scene.chars[kite_i].spc_char.act_num;
        let mut stage = Stage {
            t,
            hits: &mut *x.hits,
            camera: &mut *x.camera,
            pad: CamPad::default(),
            cast: &mut self.cast,
            skills: &self.skills,
            area,
            bounds,
            player: kite_pos,
            kite: self.kite,
            map2d: Vec::new(),
            map2d_info: [0; 3],
            event_area: false,
            annihilated: x.annihilated,
            shows: &mut self.shows,
            enter: false,
            view: [kite_pos[0], kite_pos[1], ee::add(kite_pos[2], 0x430c_0000), ONE],
            retarget: Vec::new(),
            kite_act,
            traps: Vec::new(),
            town: Some(x.nav),
            spcs: Some(&mut *x.spcs),
            tricks: Vec::new(),
            chats: Vec::new(),
            item_uses: &mut self.member_items,
        };
        // ccThPlayer (49) before the members: ccPlayer::Main's
        // CalcReal(dead) on Kite (gcmn 0x005983c8), his buffs timing out
        // and his SP coming back in a town as in a field; then ccThFellow
        // (50), each member's Main.
        let members: Vec<usize> = self.members.iter().map(|m| m.1).filter(|&m| m != kite_i).collect();
        for m in std::iter::once(kite_i).chain(members) {
            stage.player = self.scene.chars[kite_i].pos;
            let checks = {
                let act = self.scene.chars[kite_i].spc_char.act_num;
                let s = self.skills.borrow();
                (0..self.scene.chars.len()).map(|c| s.check(t, &self.scene, c, act)).collect::<Vec<i32>>()
            };
            let chk = |c: usize| checks.get(c).copied().unwrap_or(0);
            let field = fellow::Field { gho_flag: false, warp_flag: false };
            let out = {
                let cell = RefCell::new(&mut stage);
                let mut own = Own(&cell);
                let mut rt = Movement {
                    t,
                    mt: &d.mt,
                    party: &party,
                    game: &game,
                    keep: &mut self.keep,
                    spc_registry_num: registry_num,
                    world: &cell,
                    inner: &mut own,
                };
                let mut share = Share::new(&cell);
                let mut fr = fellow::Frame {
                    t,
                    mt: &d.mt,
                    scene: &mut self.scene,
                    party: &party,
                    save: x.save,
                    crew: &mut self.crew,
                    game: &game,
                    field: &field,
                    ents: &ents,
                    env: &env,
                    rng: &mut self.rand,
                    world: &mut share,
                    rt: &mut rt,
                    menu: true,
                    skill_check: &chk,
                    out: Vec::new(),
                };
                if m == kite_i {
                    let dead = i32::from(fr.scene.chars[m].cond[piney_battle::param::cond::DEAD]);
                    fr.calc_real(m, dead, &env);
                } else {
                    fr.main(m);
                }
                fr.out
            };
            for o in out {
                match o {
                    fellow::Out::SetMatrix { pos, dirc } => {
                        if let Some(a) = stage.cast.get_mut(m) {
                            a.ch.pos = pos;
                            a.ch.dirc = dirc;
                        }
                    }
                    fellow::Out::Rule(Event::EnemyRetarget(Who::Char(_))) => {}
                    fellow::Out::Rule(e) if super::chat::line_of(&e).is_some() => {
                        let p = party_ai::Parts {
                            t,
                            scene: &mut self.scene,
                            party: &party,
                            save: x.save,
                            crew: &mut self.crew,
                            game: &game,
                            ents: &ents,
                            rng: &mut self.rand,
                        };
                        super::chat::run(&mut p.ctx(&mut stage), &e);
                    }
                    o => stage.shows.push(Show::Member(m, o)),
                }
            }
        }
        for &(_, m) in self.members.iter().filter(|m| m.1 != kite_i) {
            let s = self.crew.spc.get(&m).copied().unwrap_or_default();
            let c = &self.scene.chars[m];
            if let Some(a) = stage.cast.get(m) {
                let (w, h, td) = (a.ch.width, a.ch.height, a.trans_dist);
                let disp = c.spc_char.flags & bit::DISP != 0;
                let (alpha, drawn) = stage.char_draw(c.pos, w, h, s.set_transparency, td);
                if let Some(a) = stage.cast.get_mut(m) {
                    a.alpha = alpha;
                    a.drawn = drawn && disp;
                }
            }
        }
        // The party's records back into the save (the buffs' timers).
        self.store_records(x.save);
    }

    /// A member's scene index by `charTbl` row (Kite's stand-in for 0).
    pub fn member(&self, id: i32) -> Option<usize> {
        self.members.iter().find(|m| m.0 == id).map(|m| m.1)
    }

    /// Member `w` taken off the town (`remove 2 code`, or its task's
    /// delete): off the command lists and the collision, not drawn.
    pub fn drop_member(&mut self, w: usize, hits: &mut Hits) {
        piney_battle::fellow::delete_cmnd(&mut self.scene, w);
        if let Some(s) = self.crew.spc.get_mut(&w)
            && s.body_hit.sw
        {
            let mut b = super::spc::to_body(w, self.kite, &s.body_hit, false);
            hits.hit_disable(&mut b);
            s.body_hit.sw = false;
        }
        self.scene.chars[w].spc_char.flags &= !bit::DISP;
        self.members.retain(|m| m.1 != w);
        self.cast.actors.remove(&w);
    }

    /// The members' drawn stand-ins: each actor's place (its last
    /// `SetMatrix`), for the town's lists.
    pub fn member_pos(&self, id: i32) -> Option<V4> {
        self.member(id).map(|w| self.scene.chars[w].pos)
    }
}
