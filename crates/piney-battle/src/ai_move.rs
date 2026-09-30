//! The party AI's own movers (`personal.cpp`, gcmn 0x0057f660-0x005833a0 and
//! 0x005975b0): `FollowBeacon`, `MoveP2P`, `SetTargetPosDirc`,
//! `CheckGoalBeaconPos`, the events' remote control (`ManualControl` and its
//! set-up) and a member's life in the Root Town (`ActInTown`,
//! `FollowTargetTown`, `TownNavigator*`). [`perform`] runs the calls they
//! answer; a mover is a method of [`Ctx`] with a [`Nav`]. Where the game reads
//! a destination it never set, the port starts from (0, 0, 0, 0). The rules
//! are in docs/engine/battle.md ("Party navigation").

use piney_data::volume::Volume;

use crate::chara::Body;
use crate::damage::fptosi;
use crate::geom::{self, F, V4};
use crate::navi::{self, NaviWorld, PathMap, TownMap};
use crate::party_ai::{Call, Chat, Ctx, check_action, distance_to_target};

/// 50.0, 150.0, 170.0, 180.0, 200.0, 260.0, 280.0, 500.0.
const F_50: F = 0x4248_0000;
const F_150: F = 0x4316_0000;
const F_170: F = 0x432a_0000;
const F_180: F = 0x4334_0000;
const F_200: F = 0x4348_0000;
const F_260: F = 0x4382_0000;
const F_280: F = 0x438c_0000;
const F_500: F = 0x43fa_0000;
const F_TWO: F = 0x4000_0000;
/// `recallFlag` (`ccSpcChar` +0xe1 bit 4) and `dispSW` (+0xe0 bit 1) in
/// [`crate::chara::SpcChar::flags`].
pub const RECALL: u32 = 1 << 12;
pub const DISP: u32 = 1 << 1;

/// The world and the maps the movers use.
pub struct Nav<'a> {
    pub world: &'a mut dyn NaviWorld,
    /// The dungeon's path finding globals.
    pub path: &'a mut PathMap,
    /// The Root Town's landmarks.
    pub town: &'a TownMap,
    /// `ccSpcRegistryNum()` (gcmn 0x005a1620): `ccSpcManager` +0xdc, the
    /// members registered.
    pub spc_registry_num: i32,
}

/// The tables of lines `ActInTown` says (`char *[19]` each, by
/// `MessageIndex()`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TownLine {
    /// `byebyeMessages` (gcmn 0x006531b0).
    Byebye,
    /// `gotoWeaponShopMessages` (0x00653020), `gotoMagicShopMessages`
    /// (0x00653110), `gotoRecordShopMessages` (0x00653390),
    /// `gotoGoodsShopMessages` (0x00653070), `gotoFairyShopMessages`
    /// (0x006530c0), `gotoEtcMessages` (0x00653160).
    GotoWeaponShop,
    GotoMagicShop,
    GotoRecordShop,
    GotoGoodsShop,
    GotoFairyShop,
    GotoEtc,
}

impl TownLine {
    /// The table's address in GCMN.PRG.
    pub fn table(self) -> u32 {
        match self {
            TownLine::Byebye => 0x0065_31b0,
            TownLine::GotoWeaponShop => 0x0065_3020,
            TownLine::GotoMagicShop => 0x0065_3110,
            TownLine::GotoRecordShop => 0x0065_3390,
            TownLine::GotoGoodsShop => 0x0065_3070,
            TownLine::GotoFairyShop => 0x0065_30c0,
            TownLine::GotoEtc => 0x0065_3160,
        }
    }
}

/// The EE's remainder: the dividend itself for a division by zero.
fn ee_rem(a: i32, b: i32) -> i32 {
    if b == 0 { a } else { a.wrapping_rem(b) }
}

/// `((rand() >> 3) % n) * k + base`, as the town's timers are drawn.
fn draw(ctx: &mut Ctx, n: i32, k: i32, base: i32) -> i16 {
    let r = ctx.rng.rand() >> 3;
    (ee_rem(r, n) * k + base) as i16
}

/// Runs the calls this module performs: [`Call::PathFinding`],
/// [`Call::FollowBeacon`], [`Call::GoalBeacon`], [`Call::ManualControl`],
/// [`Call::ActInTown`], returning what the game's function returns; None
/// for any other call.
pub fn perform(call: Call, ctx: &mut Ctx, nav: &mut Nav) -> Option<i32> {
    Some(match call {
        Call::PathFinding { me, from, to } => ctx.path_finding(nav, me, from, to),
        Call::FollowBeacon { me } => {
            ctx.follow_beacon(nav, me);
            0
        }
        Call::GoalBeacon { me, pos } => ctx.check_goal_beacon_pos(me, pos),
        Call::ManualControl { me } => ctx.manual_control(nav, me),
        Call::ActInTown { me } => ctx.act_in_town(nav, me),
        _ => return None,
    })
}

impl Ctx<'_> {
    fn ai_ref(&self, me: usize) -> &crate::party_ai::Ai {
        &self.crew.ais[&me]
    }

    fn ai_at(&mut self, me: usize) -> &mut crate::party_ai::Ai {
        self.crew.ais.get_mut(&me).expect("an AI for the member")
    }

    fn body_of(&self, me: usize) -> usize {
        self.ai_ref(me).body
    }

    fn spc_at(&mut self, ch: usize) -> &mut crate::party_ai::Spc {
        self.crew.spc.entry(ch).or_default()
    }

    fn spc_ref(&self, ch: usize) -> crate::party_ai::Spc {
        self.crew.spc.get(&ch).copied().unwrap_or_default()
    }

    /// `ch->CheckAction(n)`.
    fn able(&self, ch: usize, n: i32) -> bool {
        check_action(&self.scene.chars[ch], self.spc_ref(ch).act_num, n, self.t.volume)
    }

    /// `personality->velocity` (`ccSpcParam` +0xd4).
    fn velocity(&self, ch: usize) -> F {
        match &self.scene.chars[ch].body {
            Body::Spc(p) => p.velocity,
            _ => 0,
        }
    }

    /// `moveFlag = 0; runFlag = 0`.
    fn stand(&mut self, ch: usize) {
        let s = self.spc_at(ch);
        s.move_flag = false;
        s.run_flag = false;
    }

    fn rt_call(&mut self, c: Call) -> i32 {
        let p = crate::party_ai::Parts {
            t: self.t,
            scene: &mut *self.scene,
            party: self.party,
            save: &mut *self.save,
            crew: &mut *self.crew,
            game: self.game,
            ents: self.ents,
            rng: &mut *self.rng,
        };
        self.rt.call_ctx(c, p)
    }

    /// `actTypeOld = actType; actType = n; actCnt = 0`.
    fn set_act(&mut self, me: usize, n: i16) {
        let a = self.ai_at(me);
        a.act_type_old = a.act_type;
        a.act_type = n;
        a.act_cnt = 0;
    }

    /// `ccAI::MoveP2P(p1, p2, runFlag)` (gcmn 0x00582d50): a step from
    /// `p1` toward `p2`. The distance on the ground less the body's radius
    /// (`bodyHit.radius`), truncated to a whole number, is returned. The
    /// body turns to face `p2` (the heading through `RAD2DEG`/`DEG2RAD`)
    /// when it may act (`CheckAction(2)`) and is not within its stride
    /// (`velocity`); a body standing starts walking (`moveFlag`, and
    /// `runFlag` as asked) when it may (`CheckAction(1)`) and is beyond
    /// its stride; a body walking starts running when asked.
    pub fn move_p2p(&mut self, me: usize, p1: V4, p2: V4, run: i32) -> F {
        let body = self.body_of(me);
        let mut tmp = self.spc_ref(body).dirc;
        let mut d = geom::vsub(p2, p1);
        let ang = geom::deg2rad(geom::rad2deg(geom::atan2f(d[0], geom::neg(d[1]))));
        d[2] = 0;
        let dist = geom::length_on(self.t.volume, d);
        let r = geom::from_int(fptosi(geom::sub(dist, self.spc_ref(body).body_hit.radius)));
        let vel = self.velocity(body);
        if self.able(body, 2) && !geom::lt(r, vel) {
            tmp[2] = ang;
        }
        if self.spc_ref(body).move_flag {
            if run != 0 {
                self.spc_at(body).run_flag = true;
            }
        } else if self.able(body, 1) && !geom::le(r, vel) {
            let s = self.spc_at(body);
            s.move_flag = true;
            s.run_flag = run != 0;
        }
        self.spc_at(body).dirc = tmp;
        r
    }

    /// `ccAI::SetTargetPosDirc(tp)` (gcmn 0x005830f0): the body turns to
    /// face `tp` at once: `dircTg` (and the heading) `atan2f(d.x, -d.y)`
    /// of `d` = `tp` in the player's frame less the body's `posP`.
    pub fn set_target_pos_dirc(&mut self, me: usize, tp: V4) {
        let body = self.body_of(me);
        let w = self.rt.w2p(tp);
        let d = geom::vsub(w, self.scene.chars[body].pos_p);
        let h = geom::atan2f(d[0], geom::neg(d[1]));
        self.ai_at(me).dirc_tg = h;
        self.spc_at(body).dirc[2] = h;
    }

    /// `ccNavi::PathFindingInDungeon(from, to)` (gcmn 0x005148e0) of the
    /// member's navigation ([`navi::Navi::path_finding`]).
    pub fn path_finding(&mut self, nav: &mut Nav, me: usize, from: V4, to: V4) -> i32 {
        let area = self.game.area;
        let a = self.crew.ais.get_mut(&me).expect("an AI for the member");
        let mut finish = a.navi_finish;
        let r = a.navi.path_finding(&mut finish, area, nav.path, nav.world, from, to);
        a.navi_finish = finish;
        r
    }

    /// `ccNavi::GetDestination(p)` (gcmn 0x00514770) of the member's
    /// navigation; `p` as it was in a field.
    fn destination(&self, nav: &Nav, me: usize, p: &mut V4) -> i16 {
        self.ai_ref(me).navi.get_destination(self.game.area, nav.town, p)
    }

    /// `ccAI::FollowBeacon()` (gcmn 0x00582930): a step along the route
    /// (running). Within 150 of the destination the next landmark is
    /// taken (unless the navigation has finished); past the last: finished
    /// (`finishFlag` 1, `lastMarker` the goal's name), facing the goal
    /// landmark's point when it has a name ([`NaviWorld::navi_point`]),
    /// standing, and in the Root Town resting (`actType` 11) for
    /// `((rand() >> 3) % 15) * 30 + 100` frames.
    pub fn follow_beacon(&mut self, nav: &mut Nav, me: usize) {
        let body = self.body_of(me);
        let mp = self.scene.chars[body].pos;
        let mut tp = [0; 4];
        self.destination(nav, me, &mut tp);
        let r = self.move_p2p(me, mp, tp, 1);
        if !geom::lt(r, F_150) {
            return;
        }
        if self.ai_ref(me).navi_finish == 0 {
            {
                let n = &mut self.ai_at(me).navi;
                n.landmark = n.landmark.wrapping_add(1);
            }
        }
        let n = &self.ai_ref(me).navi;
        if n.landmark < n.step {
            return;
        }
        let a = self.ai_at(me);
        a.navi_finish = 1;
        a.last_marker = a.navi.name;
        let name = a.navi.name;
        if name != 0 {
            let p = nav.world.navi_point(name);
            self.set_target_pos_dirc(me, p);
        }
        self.stand(body);
        if self.game.area == 0 {
            let t = draw(self, 15, 30, 100);
            self.ai_at(me).act_time = t;
            self.set_act(me, 11);
        }
    }

    /// `ccAI::CheckGoalBeaconPos(p)` (gcmn 0x005975b0): 1 when `p`'s map
    /// cell (`Get2DPos`, `fptosi`, as shorts) is not the last beacon's
    /// (`route[beacon.num - 1]`), else 0.
    pub fn check_goal_beacon_pos(&self, me: usize, p: V4) -> i32 {
        let n = &self.ai_ref(me).navi;
        let c = navi::pos_2d(p);
        let (px, py) = (fptosi(c[0]) as i16, fptosi(c[1]) as i16);
        let b = n.check_beacon_pos_2d(n.beacon_num.wrapping_sub(1));
        let (bx, by) = (fptosi(b[0]) as i16, fptosi(b[1]) as i16);
        i32::from(px != bx || py != by)
    }

    /// `ccAI::ManualMode()` (gcmn 0x00583270): the member under remote
    /// control (`manualSW`), `gDeg` its heading, `gRotSp` 64, `remoteCmd`
    /// 0.
    pub fn manual_mode(&mut self, me: usize) {
        let body = self.body_of(me);
        let deg = geom::rad2deg(self.spc_ref(body).dirc[2]);
        let a = self.ai_at(me);
        a.manual_sw = true;
        a.g_deg = deg as u16;
        a.g_rot_sp = 64;
        a.remote_cmd = 0;
    }

    /// `ccAI::SetRemoteCmd(cmd)` (gcmn 0x005832e0): an event's command
    /// (`remoteCmd`, `remoteFlag` 1); 0 keeps the present heading
    /// (`gDeg`), 5 takes a member of no party out of it (`partyFlag` -2).
    pub fn set_remote_cmd(&mut self, me: usize, cmd: i32) {
        let body = self.body_of(me);
        let a = self.ai_at(me);
        a.remote_cmd = cmd as i16;
        a.remote_flag = true;
        if cmd == 0 {
            let deg = geom::rad2deg(self.spc_ref(body).dirc[2]);
            self.ai_at(me).g_deg = deg as u16;
        } else if cmd == 5 {
            let ch = &mut self.scene.chars[body];
            if party_flag(ch.party_flag) == 0 {
                ch.party_flag = 6;
            }
        }
    }

    /// `ccAI::SetGoalPos(v, t)` (gcmn 0x005833a0): where the remote control
    /// walks to (`gPos`): straight (`t` 0, `gPoint` -1) or along the town's
    /// route from the body (`t` 1, `RouteSearchByMap`, `gPoint` -2).
    pub fn set_goal_pos(&mut self, nav: &mut Nav, me: usize, v: V4, t: i32) {
        match t {
            0 => self.ai_at(me).g_point = -1,
            1 => {
                let body = self.body_of(me);
                let start = self.scene.chars[body].pos;
                self.town_navigator_pos(nav, me, start, v);
                self.ai_at(me).g_point = -2;
            }
            _ => {}
        }
        self.ai_at(me).g_pos = v;
    }

    /// `ccAI::ManualControl()` (gcmn 0x00580ef0): the member under an event's
    /// remote control, by `remoteCmd`: 0 stand and turn toward `gDeg`, 1 walk
    /// (2 run) to `gPos` straight or along the town route, 3 and 4 transfer in
    /// and out, 5 leave the party, 6 hide, 7 show. "Done" clears `remoteCmd`
    /// and `remoteFlag`; the others leave `remoteFlag` 1. Returns 0. The table
    /// is in docs/engine/battle.md ("Remote control").
    pub fn manual_control(&mut self, nav: &mut Nav, me: usize) -> i32 {
        let body = self.body_of(me);
        let dirc = self.spc_ref(body).dirc;
        let pos = self.scene.chars[body].pos;
        let cmd = self.ai_ref(me).remote_cmd;
        let act = self.spc_ref(body).act_num;
        match cmd {
            0 => {
                self.stand(body);
                let a = self.ai_ref(me);
                let (g_deg, sp) = (a.g_deg, a.g_rot_sp);
                let mut d = dirc;
                d[2] = geom::set_dirc(d[2], geom::deg2rad(g_deg as i16), i32::from(sp));
                let facing = u32::from(g_deg) == geom::rad2deg(d[2]) as i32 as u32;
                let stop = self.spc_ref(body).stop_flag;
                self.ai_at(me).remote_flag = !(stop && facing);
                self.spc_at(body).dirc = d;
            }
            1 | 2 => {
                let run = i32::from(cmd != 2);
                let g_point = self.ai_ref(me).g_point;
                if g_point == -1 {
                    let g_pos = self.ai_ref(me).g_pos;
                    let r = self.move_p2p(me, pos, g_pos, run);
                    if geom::lt(r, F_50) {
                        self.arrive(me, body, dirc);
                    }
                } else {
                    if g_point != -2 {
                        let mut g_pos = self.ai_ref(me).g_pos;
                        self.town_navigator_point(nav, me, &mut g_pos, pos, i32::from(g_point));
                        let a = self.ai_at(me);
                        a.g_pos = g_pos;
                        a.g_point = -2;
                    }
                    let mut tp = [0; 4];
                    self.destination(nav, me, &mut tp);
                    let r = self.move_p2p(me, pos, tp, run);
                    if geom::lt(r, F_50) {
                        if self.ai_ref(me).navi_finish == 0 {
                            {
                                let n = &mut self.ai_at(me).navi;
                                n.landmark = n.landmark.wrapping_add(1);
                            }
                        }
                        let n = &self.ai_ref(me).navi;
                        if n.landmark >= n.step {
                            let a = self.ai_at(me);
                            a.navi_finish = 1;
                            a.last_marker = a.navi.name;
                            self.arrive(me, body, dirc);
                        }
                    }
                }
            }
            3 => {
                if act == 2 {
                    self.done(me);
                } else {
                    if act == 14 {
                        self.rt_call(Call::TransferIn { ch: body });
                    }
                    self.ai_at(me).remote_flag = true;
                }
            }
            4 => {
                if act == 14 {
                    self.done(me);
                } else {
                    if act != 12 {
                        self.rt_call(Call::TransferOut { ch: body });
                    }
                    self.ai_at(me).remote_flag = true;
                }
            }
            5 => {
                if act != 14 {
                    if act != 12 {
                        self.rt_call(Call::TransferOut { ch: body });
                        let id = i32::from(self.scene.chars[body].base().id);
                        if party_flag(self.scene.chars[body].party_flag) == 1 {
                            self.rt_call(Call::ResignParty { id });
                        } else {
                            self.rt_call(Call::DisbandSpc { id });
                        }
                        self.scene.chars[body].party_flag = 6;
                    }
                    self.ai_at(me).remote_flag = true;
                }
            }
            6 => {
                if act != 14 {
                    self.spc_at(body).act_num = 14;
                    self.scene.chars[body].spc_char.act_num_old = -1;
                }
                let s = self.spc_at(body);
                s.transparency = 0;
                s.set_transparency = 0;
                self.scene.chars[body].spc_char.cloak = 0;
                self.done(me);
            }
            7 => {
                if act == 14 {
                    self.spc_at(body).act_num = 2;
                    self.scene.chars[body].spc_char.act_num_old = -1;
                    let dead = self.scene.chars[body].cond[crate::param::cond::DEAD];
                    if dead == 0 || dead == 5 {
                        self.rt_call(Call::HitEnable { ch: body });
                    }
                }
                let s = self.spc_at(body);
                s.transparency = geom::ONE;
                s.set_transparency = geom::ONE;
                self.scene.chars[body].spc_char.cloak = geom::ONE;
                self.scene.chars[body].spc_char.flags |= DISP;
                self.done(me);
            }
            _ => {}
        }
        0
    }

    /// `remoteCmd = 0; remoteFlag = 0`.
    fn done(&mut self, me: usize) {
        let a = self.ai_at(me);
        a.remote_cmd = 0;
        a.remote_flag = false;
    }

    /// The end of a remote walk: standing, the command done with
    /// `remoteFlag` 1, `gDeg` the heading the body had at the frame's
    /// start.
    fn arrive(&mut self, me: usize, body: usize, dirc: V4) {
        self.stand(body);
        let a = self.ai_at(me);
        a.remote_cmd = 0;
        a.remote_flag = true;
        a.g_deg = geom::rad2deg(dirc[2]) as u16;
    }

    /// `ccAI::TownNavigator(goal, start, dest)` (gcmn 0x00582c50): the
    /// route from `start` to the landmark named `dest`
    /// (`ccNaviSearchLandmark`); `goal` becomes its position when there is
    /// one. Returns what `RouteSearchByMap` returns.
    pub fn town_navigator(&mut self, nav: &mut Nav, me: usize, goal: &mut V4, start: V4, dest: i32) -> i32 {
        let n = nav.town.search_landmark(dest);
        self.town_navigator_point(nav, me, goal, start, n)
    }

    /// `ccAI::TownNavigatorPoint(goal, start, n)` (gcmn 0x00582cc0): the
    /// same to landmark number `n`.
    pub fn town_navigator_point(&mut self, nav: &mut Nav, me: usize, goal: &mut V4, start: V4, n: i32) -> i32 {
        if let Some(p) = nav.town.landmark_pos(n) {
            *goal = p;
        }
        self.town_navigator_pos(nav, me, start, *goal)
    }

    /// `ccAI::TownNavigatorPos(start, goal)` (gcmn 0x00582d20): the
    /// member's `RouteSearchByMap(start, goal)`.
    pub fn town_navigator_pos(&mut self, nav: &mut Nav, me: usize, start: V4, goal: V4) -> i32 {
        let a = self.crew.ais.get_mut(&me).expect("an AI for the member");
        let mut finish = a.navi_finish;
        let r = nav.world.route_search_by_map(&mut a.navi, &mut finish, start, goal);
        a.navi_finish = finish;
        r
    }

    /// `ccAI::MessageIndex()` (gcmn 0x0058dc40): the body's character id,
    /// 18 for id 1 while `saveData` +0x220d is set.
    pub fn message_index(&self, me: usize) -> i32 {
        let id = i32::from(self.scene.chars[self.body_of(me)].base().id);
        if id == 1 && self.save.u8(0x220d) != 0 { 18 } else { id }
    }

    fn line(&mut self, me: usize, table: TownLine) {
        let body = self.body_of(me);
        let i = self.message_index(me);
        self.rt_call(Call::Chat { me: body, msg: Chat::Line(table, i) });
    }

    /// `ccAI::FollowTargetTown(target)` (gcmn 0x005826b0): a step after a
    /// listed `target` in the town: `dircTg` the heading to it
    /// (`atan2f(d.y, d.x)` turned a quarter, through the 16-bit units),
    /// `distTg` the ground distance less both widths, truncated; the body
    /// turns when it may act and the target is beyond `noTurnRange`, stops
    /// within 50, starts running beyond 180.
    pub fn follow_target_town(&mut self, me: usize, target: Option<usize>) {
        let Some(tg) = target.filter(|&t| self.scene.listed(t)) else { return };
        let body = self.body_of(me);
        let posp = self.scene.chars[body].pos_p;
        let mut tmp = self.spc_ref(body).dirc;
        let tp = self.rt.w2p(self.scene.chars[tg].pos);
        let mut d = geom::vsub(tp, posp);
        let deg = geom::rad2deg(geom::atan2f(d[1], d[0]));
        let dirc_tg = geom::deg2rad((i32::from(deg) + 0x4000) as i16);
        self.ai_at(me).dirc_tg = dirc_tg;
        d[2] = 0;
        let bw = self.scene.chars[body].base().width;
        let tw = self.scene.chars[tg].base().width;
        let dist = geom::sub(geom::sub(geom::length_on(self.t.volume, d), bw), tw);
        let dist_tg = geom::from_int(fptosi(dist));
        self.ai_at(me).dist_tg = dist_tg;
        let no_turn = self.t.ai_params.get(self.ai_ref(me).param).map_or(0, |p| p.no_turn_range);
        if self.able(body, 2) && !geom::le(dist_tg, no_turn) {
            tmp[2] = dirc_tg;
        }
        self.spc_at(body).dirc = tmp;
        if self.spc_ref(body).move_flag {
            if geom::lt(dist_tg, F_50) {
                self.stand(body);
            }
        } else if self.able(body, 1) && !geom::le(dist_tg, F_180) {
            let s = self.spc_at(body);
            s.move_flag = true;
            s.run_flag = true;
        }
    }

    /// Kite's reach: `ccCheckTarget(leader)` and the member itself in the
    /// party (`partyFlag` 1); otherwise the town walk starts over
    /// (`finishFlag` 1, `actType` 0). Returns whether to go on.
    ///
    /// From Mutation on the member also stops, and going to Kite
    /// (`drop_cmd`, `actType` 3) it drops the command when he is gone.
    fn leader_near(&mut self, me: usize, leader: Option<usize>, drop_cmd: bool) -> bool {
        let body = self.body_of(me);
        let listed = leader.is_some_and(|l| self.scene.listed(l));
        if !listed || party_flag(self.scene.chars[body].party_flag) != 1 {
            let a = self.ai_at(me);
            a.navi_finish = 1;
            a.act_type = 0;
            if self.t.volume != Volume::Inf {
                self.stand(body);
                if drop_cmd && !listed {
                    let a = self.ai_at(me);
                    a.chat_cmd = -1;
                    a.chat_cmd_flag = 0;
                }
            }
            return false;
        }
        true
    }

    /// Meeting Kite again: `actType` 94 (following), `followSW`, and the
    /// greeting once (`inviteFlag`).
    fn rejoin(&mut self, me: usize) {
        self.ai_at(me).act_time = 0;
        self.set_act(me, 94);
        self.ai_at(me).follow_sw = true;
        if self.ai_ref(me).invite_flag {
            let body = self.body_of(me);
            self.rt_call(Call::Chat { me: body, msg: Chat::Reencounter });
            self.ai_at(me).invite_flag = false;
        }
    }

    /// `ccAI::ActInTown()` (gcmn 0x0057f660): a member in the Root Town, once a
    /// frame: the party's orders (a recall, a member leaving, the chat commands
    /// 12 and 9), then mode 5 keeps its distance from Kite and mode 1 walks the
    /// town by `actType` (rest, the shops, a landmark at random, following
    /// Kite). A walk ends within twice the stride of its landmark; stuck for
    /// 200 frames (`noMoveCnt`) it looks for the nearest landmark. Returns 0.
    /// The `actType` table is in docs/engine/battle.md ("The town walk").
    pub fn act_in_town(&mut self, nav: &mut Nav, me: usize) -> i32 {
        let body = self.body_of(me);
        let mp = self.scene.chars[body].pos;
        let mut tp: V4 = [0; 4];
        let leader = self.party.members[0];
        let act = self.spc_ref(body).act_num;
        let moving_act = matches!(act, 12..=14);
        // A recalled member comes back to the party.
        let flags = self.scene.chars[body].spc_char.flags;
        if flags & RECALL != 0 && party_flag(self.scene.chars[body].party_flag) < 0 && !moving_act {
            self.scene.chars[body].spc_char.flags &= !RECALL;
            self.scene.chars[body].party_flag = 1;
            self.set_act(me, 3);
            self.ai_at(me).navi_finish = 1;
        }
        // A member leaving the party.
        if party_flag(self.scene.chars[body].party_flag) == -1 && !moving_act {
            self.scene.chars[body].party_flag = 6;
            self.ai_at(me).act_time = 0;
            self.set_act(me, 2);
            let a = self.ai_at(me);
            a.navi_finish = 1;
            a.chat_cmd = -1;
            a.chat_cmd_flag = 0;
            if nav.spc_registry_num == 5 {
                self.stand(body);
                self.set_act(me, 1);
                let t = draw(self, 10, 4, 30);
                self.ai_at(me).act_time = t;
                self.line(me, TownLine::Byebye);
            }
        }
        // The player's commands.
        if self.ai_ref(me).chat_cmd_flag > 0 {
            match self.ai_ref(me).chat_cmd {
                12 => {
                    if self.ai_ref(me).act_type != 0 {
                        let a = self.ai_at(me);
                        a.navi_finish = 1;
                        a.act_type = 0;
                        self.change_mode(me, 1, 0);
                    }
                    let a = self.ai_at(me);
                    a.chat_cmd = -1;
                    a.chat_cmd_flag = 0;
                }
                9 if !matches!(self.ai_ref(me).act_type, 3 | 94 | 97) => {
                    let a = self.ai_at(me);
                    a.navi_finish = 1;
                    a.act_type = 3;
                    self.change_mode(me, 1, 0);
                }
                _ => {}
            }
        }
        // From Mutation on a party member stands back (actType 100) while it
        // or Kite goes through a gate (act 12).
        if self.t.volume != Volume::Inf
            && party_flag(self.scene.chars[body].party_flag) == 1
            && self.ai_ref(me).act_type != 100
        {
            let kite_out = self.game.player.is_some_and(|p| self.spc_ref(p).act_num == 12);
            if kite_out || self.spc_ref(body).act_num == 12 {
                self.change_mode(me, 1, 0);
                self.stand(body);
                let a = self.ai_at(me);
                a.act_time = 100;
                a.act_type_old = a.act_type;
                a.act_type = 100;
                a.act_cnt = 0;
            }
        }
        match self.inf_mode(self.ai_ref(me).mode) {
            5 => {
                self.keep_away(me, body, leader);
                return 0;
            }
            1 => {}
            _ => return 0,
        }
        let act_type = self.ai_ref(me).act_type;
        match act_type {
            0 => {
                self.stand(body);
                let t = draw(self, 10, 30, 30);
                self.ai_at(me).act_time = t;
                self.set_act(me, 99);
            }
            1 => {
                if self.count_down(me) {
                    self.rt_call(Call::TransferOut { ch: body });
                    self.set_act(me, 98);
                }
            }
            2 => {
                if self.ai_ref(me).navi_finish != 0 {
                    self.town_navigator(nav, me, &mut tp, mp, 1);
                }
                let n = &self.ai_ref(me).navi;
                let lm = nav.town.mark(i32::from(n.route_byte(i32::from(n.landmark)))).pos;
                let r = self.move_p2p(me, mp, lm, 1);
                if geom::lt(r, F_50) {
                    {
                        let n = &mut self.ai_at(me).navi;
                        n.landmark = n.landmark.wrapping_add(1);
                    }
                    let n = &self.ai_ref(me).navi;
                    if n.landmark >= n.step {
                        self.ai_at(me).navi_finish = 1;
                        self.stand(body);
                        self.set_act(me, 1);
                        let t = draw(self, 10, 4, 30);
                        self.ai_at(me).act_time = t;
                        self.line(me, TownLine::Byebye);
                    }
                }
                if self.ai_ref(me).no_move_cnt >= 201 {
                    let a = self.ai_at(me);
                    a.no_move_cnt = 0;
                    a.navi_finish = 1;
                }
            }
            3 => self.go_to_leader(nav, me, body, leader, mp, &mut tp),
            4 | 12 => {
                let n = i32::from(self.ai_ref(me).act_dummy);
                if let Some(p) = nav.town.landmark_pos(n) {
                    tp = p;
                }
                let r = self.move_p2p(me, mp, tp, 1);
                if geom::lt(r, F_50) {
                    self.stand(body);
                    let t = draw(self, 10, 30, 30);
                    self.ai_at(me).act_time = t;
                    self.set_act(me, 99);
                }
                if self.ai_ref(me).no_move_cnt >= 201 {
                    self.ai_at(me).no_move_cnt = 0;
                    let k = nav.town.search_near_landmark_n(self.t.volume, mp, 3);
                    self.ai_at(me).act_dummy = k as i16;
                }
            }
            k @ 5..=9 => {
                if self.ai_ref(me).navi_finish != 0 {
                    let (dest, line) = match k {
                        5 => (2, TownLine::GotoWeaponShop),
                        6 => (3, TownLine::GotoMagicShop),
                        7 => (6, TownLine::GotoRecordShop),
                        8 => (4, TownLine::GotoGoodsShop),
                        _ => (5, TownLine::GotoFairyShop),
                    };
                    self.ai_at(me).no_move_cnt = 0;
                    self.town_navigator(nav, me, &mut tp, mp, dest);
                    if i32::from(self.ai_ref(me).last_marker) != dest {
                        self.line(me, line);
                    }
                }
                self.walk(nav, me, body, mp, &mut tp);
            }
            10 => {
                if self.ai_ref(me).navi_finish != 0 {
                    let r = self.rng.rand() >> 3;
                    let k = ee_rem(r, nav.town.num.wrapping_sub(1)) + 1;
                    if let Some(p) = nav.town.landmark_pos(k) {
                        tp = p;
                    }
                    self.town_navigator_pos(nav, me, mp, tp);
                    self.line(me, TownLine::GotoEtc);
                    self.ai_at(me).no_move_cnt = 0;
                }
                self.walk(nav, me, body, mp, &mut tp);
            }
            11 => {
                if self.count_down(me) {
                    self.ai_at(me).act_time = 10;
                    self.set_act(me, 99);
                }
            }
            20 => self.walk(nav, me, body, mp, &mut tp),
            94 => {
                if self.leader_near(me, leader, false) {
                    let l = leader.expect("a listed leader");
                    self.follow_target_town(me, leader);
                    if geom::lt(distance_to_target(self.t.volume, self.scene, body, leader), F_150) {
                        if self.spc_ref(l).stop_flag {
                            self.stand(body);
                        }
                        let a = self.ai_at(me);
                        a.follow_sw = false;
                        a.navi_finish = 1;
                        a.no_move_cnt = 0;
                        a.act_time = 0;
                        self.set_act(me, 97);
                    }
                    if self.ai_ref(me).no_move_cnt >= 201
                        && !geom::le(distance_to_target(self.t.volume, self.scene, body, leader), F_500)
                    {
                        self.lost_leader(me);
                    }
                }
            }
            95 => {
                self.stand(body);
                self.ai_at(me).act_time = 100;
                self.set_act(me, 96);
            }
            96 => self.wait_for_leader(me, body),
            97 => {
                if self.leader_near(me, leader, false) {
                    self.rt_call(Call::FollowPlayer { me: body });
                    let a = self.ai_ref(me);
                    if a.no_move_cnt >= 201 && !geom::le(a.dist_pl, F_500) {
                        self.lost_leader(me);
                    }
                }
            }
            99 => {
                if self.count_down(me) {
                    let k = (ee_rem(self.rng.rand() >> 3, 6) + 5) as i16;
                    if (self.scene.chars[body].base().id == 1 && k == 7) || self.ai_ref(me).act_type_old == k {
                        self.ai_at(me).act_time = 1;
                    } else {
                        self.ai_at(me).act_time = 0;
                        self.set_act(me, k);
                    }
                }
            }
            100 => {
                if self.spc_ref(body).act_num == 2 {
                    self.ai_at(me).act_time = 0;
                    self.set_act(me, 0);
                }
            }
            // After 97 at once, else once the timer has run out.
            101 if self.ai_ref(me).act_type_old == 97 || self.count_down(me) => {
                let a = self.ai_at(me);
                a.act_time = 0;
                a.act_type = a.act_type_old;
                a.act_cnt = 0;
            }
            _ => {}
        }
        // Every frame of mode 1.
        let a = self.ai_at(me);
        a.act_cnt = a.act_cnt.wrapping_add(1);
        let now = self.scene.chars[body].pos;
        if self.spc_ref(body).move_flag {
            let old = self.ai_ref(me).pos_old;
            if navi::cell_delta(now[0], old[0]) == 0
                && navi::cell_delta(now[1], old[1]) == 0
                && self.ai_ref(me).no_move_cnt < 32767
            {
                self.ai_at(me).no_move_cnt += 1;
            }
        }
        self.ai_at(me).pos_old = now;
        0
    }

    /// `--actTime > 0` is false: the timer has run out.
    fn count_down(&mut self, me: usize) -> bool {
        let a = self.ai_at(me);
        a.act_time = a.act_time.wrapping_sub(1);
        a.act_time <= 0
    }

    /// Kite lost from a follow: `followSW` 0, the route over, `actType` 3.
    fn lost_leader(&mut self, me: usize) {
        let a = self.ai_at(me);
        a.follow_sw = false;
        a.navi_finish = 1;
        a.no_move_cnt = 0;
        self.set_act(me, 3);
    }

    /// `actType` 3: to Kite by the town's route.
    fn go_to_leader(&mut self, nav: &mut Nav, me: usize, body: usize, leader: Option<usize>, mp: V4, tp: &mut V4) {
        if !self.leader_near(me, leader, true) {
            return;
        }
        let l = leader.expect("a listed leader");
        if geom::le(distance_to_target(self.t.volume, self.scene, body, leader), F_200) {
            let a = self.ai_at(me);
            a.navi_finish = 1;
            a.last_marker = a.navi.name;
            *tp = self.scene.chars[l].pos;
            self.set_target_pos_dirc(me, *tp);
            if self.spc_ref(l).stop_flag {
                self.stand(body);
            }
            self.rejoin(me);
            return;
        }
        if self.ai_ref(me).navi_finish != 0 {
            *tp = self.scene.chars[l].pos;
            self.town_navigator_pos(nav, me, mp, *tp);
        } else {
            self.destination(nav, me, tp);
            let r = self.move_p2p(me, mp, *tp, 1);
            if geom::lt(r, F_150) {
                if self.ai_ref(me).navi_finish == 0 {
                    {
                        let n = &mut self.ai_at(me).navi;
                        n.landmark = n.landmark.wrapping_add(1);
                    }
                }
                let n = &self.ai_ref(me).navi;
                if n.landmark >= n.step {
                    let a = self.ai_at(me);
                    a.navi_finish = 1;
                    a.last_marker = a.navi.name;
                    *tp = self.scene.chars[l].pos;
                    let n = &self.ai_ref(me).navi;
                    let last = i32::from(n.route_byte(i32::from(n.step) - 1));
                    if last == nav.town.search_near_landmark(self.t.volume, *tp) {
                        if !geom::le(distance_to_target(self.t.volume, self.scene, body, leader), F_200) {
                            self.rejoin(me);
                        }
                    } else {
                        *tp = self.scene.chars[l].pos;
                        self.town_navigator_pos(nav, me, mp, *tp);
                    }
                }
            }
        }
        if self.ai_ref(me).no_move_cnt >= 201 {
            let a = self.ai_at(me);
            a.navi_finish = 1;
            a.no_move_cnt = 0;
        }
    }

    /// The walk along the town's route (`actType` 5-10 and 20).
    fn walk(&mut self, nav: &mut Nav, me: usize, body: usize, mp: V4, tp: &mut V4) {
        self.destination(nav, me, tp);
        let r = self.move_p2p(me, mp, *tp, 1);
        if geom::lt(r, geom::mul(F_TWO, self.velocity(body))) {
            if self.ai_ref(me).navi_finish == 0 {
                {
                    let n = &mut self.ai_at(me).navi;
                    n.landmark = n.landmark.wrapping_add(1);
                }
            }
            let n = &self.ai_ref(me).navi;
            if n.landmark >= n.step {
                let a = self.ai_at(me);
                a.navi_finish = 1;
                a.last_marker = a.navi.name;
                let name = a.navi.name;
                if name != 0 {
                    let p = nav.world.navi_point(name);
                    self.set_target_pos_dirc(me, p);
                }
                self.stand(body);
                let t = draw(self, 15, 30, 100);
                self.ai_at(me).act_time = t;
                self.set_act(me, 11);
            }
        }
        if self.ai_ref(me).no_move_cnt >= 201 {
            let a = self.ai_at(me);
            a.navi_finish = 1;
            a.no_move_cnt = 0;
            self.set_act(me, 12);
            let k = nav.town.search_near_landmark(self.t.volume, mp);
            self.ai_at(me).act_dummy = k as i16;
        }
    }

    /// From Mutation on, Kite rides the Grunty (`pgRideFlag`): the member
    /// neither greets him nor goes to him.
    fn riding(&self) -> bool {
        self.t.volume != Volume::Inf && self.game.pg_ride_flag != 0
    }

    /// `actType` 96: waiting for Kite (while transferring in, `actNum` 13,
    /// for `actTime` frames first).
    fn wait_for_leader(&mut self, me: usize, body: usize) {
        if self.spc_ref(body).act_num == 13 && !self.count_down(me) {
            return;
        }
        if geom::lt(self.ai_ref(me).dist_pl, F_280) {
            let a = self.ai_at(me);
            a.act_type_old = a.act_type;
            a.act_cnt = 0;
            if self.ai_ref(me).invite_flag && self.able(body, 1) && !self.riding() {
                self.rt_call(Call::Chat { me: body, msg: Chat::Reencounter });
                let a = self.ai_at(me);
                a.invite_flag = false;
                a.act_time = 0;
                a.act_type = 94;
                a.follow_sw = true;
            } else {
                let a = self.ai_at(me);
                a.act_time = 100;
                a.act_type = 99;
                a.follow_sw = false;
            }
        } else if self.ai_ref(me).invite_flag && !self.riding() {
            self.ai_at(me).act_time = 0;
            self.set_act(me, 3);
        } else {
            let a = self.ai_at(me);
            a.act_time = 10;
            a.act_type_old = a.act_type;
            a.act_type = 99;
            a.follow_sw = false;
            a.act_cnt = 0;
        }
    }

    /// Mode 5: out of Kite's way.
    fn keep_away(&mut self, me: usize, body: usize, leader: Option<usize>) {
        let dt = distance_to_target(self.t.volume, self.scene, body, leader);
        if !self.ai_ref(me).follow_sw && geom::lt(dt, F_150) {
            self.ai_at(me).follow_sw = true;
        }
        if self.ai_ref(me).follow_sw {
            self.rt_call(Call::LeavePlayer { me: body });
            if geom::lt(dt, F_170) {
                return;
            }
            if self.able(body, 2) {
                let mut d = self.spc_ref(body).dirc;
                let deg = geom::rad2deg(d[2]);
                d[2] = geom::deg2rad((i32::from(deg) + 0x8000) as i16);
                self.spc_at(body).dirc = d;
                self.stand(body);
                self.ai_at(me).follow_sw = false;
            }
        } else {
            if geom::le(dt, F_260) {
                return;
            }
            if self.able(body, 0) {
                self.spc_at(body).target_char = None;
            }
            let a = self.ai_at(me);
            a.target = None;
            a.target_flag = 0;
            self.change_mode(me, 1, 0);
        }
    }
}

/// `ccSpcChar.partyFlag`, the signed 3-bit field.
pub(crate) fn party_flag(v: i32) -> i32 {
    ((v & 7) << 29) >> 29
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn town_timers_draw_as_the_game() {
        let mut r = crate::rand::Rand(12345);
        let v = crate::rand::Rng::rand(&mut r.clone()) >> 3;
        let mut c = crate::rand::Rand(12345);
        let got = {
            let x = crate::rand::Rng::rand(&mut c) >> 3;
            (x % 15) * 30 + 100
        };
        assert_eq!(got, (v % 15) * 30 + 100);
        assert_eq!(ee_rem(7, 0), 7);
        assert_eq!(ee_rem(-7, 3), -1);
        r = crate::rand::Rand(1);
        assert!(crate::rand::Rng::rand(&mut r) >= 0);
    }

    #[test]
    fn party_flag_is_signed() {
        assert_eq!(party_flag(6), -2);
        assert_eq!(party_flag(7), -1);
        assert_eq!(party_flag(1), 1);
    }
}
