//! The AI's following (`personal.cpp`, gcmn 0x00581500-0x00582c44 and
//! 0x00589e90-0x0058ae4c): how a member the AI drives walks after Kite, steps
//! away from him, closes in on and faces its target, and steers round what
//! stands in its way. [`Follow`] is a [`Runtime`] that performs
//! [`Call::FollowPlayer`], [`Call::LeavePlayer`], [`Call::FollowTarget`] and
//! [`Call::FollowTargetDirc`] and hands every other call to an inner runtime.
//! A follow function only turns the body and sets `moveFlag` and `runFlag`.
//! The rules are in docs/engine/battle.md ("Following").

use piney_data::volume::Volume;

use crate::exp::Party;
use crate::fellow::MotionTables;
use crate::geom::{
    self, F, MINUS_ONE, ONE, V4, add, atan2f, cosf, deg2rad, from_int, le, lt, mul, normalize, rad2deg, sinf, sub,
    vadd, vscale, vsub,
};
use crate::param::AiParam;
use crate::party_ai::{Ai, Call, Crew, Game, Parts, Runtime, Spc, check_action, distance_to_target};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::tables::Tables;
use crate::world::World;

const F_20: F = 0x41a0_0000;
const F_50: F = 0x4248_0000;
const F_70: F = 0x428c_0000;
const F_99: F = 0x42c6_0000;
const F_150: F = 0x4316_0000;
const F_200: F = 0x4348_0000;
const F_250: F = 0x437a_0000;
const F_260: F = 0x4382_0000;
const F_280: F = 0x438c_0000;
const F_350: F = 0x43af_0000;
/// 1.1f.
const F_1_1: F = 0x3f8c_cccd;
const F_1_5: F = 0x3fc0_0000;
const F_2: F = 0x4000_0000;
/// -999.0f: `CheckFrontObstacle`'s "the way the body faces".
pub const FACING: F = 0xc479_c000;

/// The global the following writes: `aiOpenDirc` (main 0x00378ca4, a
/// float), the heading `CheckFrontObstacleF` last found open. 0 in the
/// executable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FollowState {
    pub ai_open_dirc: F,
}

/// `DEG2RAD((short)((short)RAD2DEG(h) + d))`: a heading turned by `d` in
/// 16-bit units, as the code writes it (`RAD2DEG`'s result sign-extended,
/// the sum truncated to a short).
pub fn turn(h: F, d: i32) -> F {
    deg2rad((i32::from(rad2deg(h)) + d) as i16)
}

/// The heading of a ground vector `d`: `atan2f(d.y, d.x)` turned a
/// quarter (`+ 16384`), so that 0 faces -y.
fn heading(d: V4) -> F {
    turn(atan2f(d[1], d[0]), 16384)
}

/// `c.eq.s` against -1.0: nothing in the way.
fn clear(r: F) -> bool {
    geom::eq(MINUS_ONE, r)
}

/// `(float)(int)x` (`fptosi`, then `cvt.s.w`).
fn whole(x: F) -> F {
    from_int(crate::damage::fptosi(x))
}

/// The AI that drives `body` (its key in [`Crew::ais`]; the runtime keys
/// every AI by its body).
fn ai_key(crew: &Crew, body: usize) -> usize {
    crew.ais.iter().find(|(_, a)| a.body == body).map_or(body, |(k, _)| *k)
}

fn param(t: &Tables, a: &Ai) -> AiParam {
    t.ai_params.get(a.param).copied().unwrap_or_default()
}

fn spc(crew: &mut Crew, ch: usize) -> &mut Spc {
    crew.spc.entry(ch).or_default()
}

/// `moveFlag = 0; runFlag = 0`.
fn halt(crew: &mut Crew, body: usize) {
    let s = spc(crew, body);
    s.move_flag = false;
    s.run_flag = false;
}

/// `ch->CheckAction(n)` (`ccSpcChar::CheckAction`, gcmn 0x0059e660, the
/// virtual at +8 of both `ccFellow` and `ccPlayer`).
fn can_act(scene: &Scene, crew: &Crew, ch: usize, n: i32, volume: Volume) -> bool {
    check_action(&scene.chars[ch], crew.spc.get(&ch).map_or(0, |s| s.act_num), n, volume)
}

/// A [`Runtime`] that performs the following calls ([`Call::FollowPlayer`],
/// [`Call::LeavePlayer`], [`Call::FollowTarget`],
/// [`Call::FollowTargetDirc`]) with this module's functions and passes
/// every other call to `inner`; its [`Runtime::w2p`] is the world's.
pub struct Follow<'a> {
    pub t: &'a Tables,
    pub mt: &'a MotionTables,
    /// `ccPartyManager`: `getPartyMenberChar(0)` is Kite.
    pub party: &'a Party,
    /// `ccGame.area`: 1 a field, 2 a dungeon.
    pub game: &'a Game,
    pub state: &'a mut FollowState,
    pub world: &'a mut dyn World,
    pub inner: &'a mut dyn Runtime,
}

impl Runtime for Follow<'_> {
    fn call(&mut self, call: Call, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) -> i32 {
        match call {
            Call::FollowPlayer { me } => self.follow_player(me, scene, crew, rng),
            Call::LeavePlayer { me } => self.leave_player(me, scene, crew, rng),
            Call::FollowTarget { me, target } => self.follow_target(me, target, scene, crew, rng),
            Call::FollowTargetDirc { me, target } => self.follow_target_dirc(me, target, scene, crew),
            other => return self.inner.call(other, scene, crew, rng),
        }
        0
    }

    /// The following's calls performed here, every other call handed on
    /// with its parts (the movers `FollowBeacon`, `GoalBeacon`,
    /// `ManualControl`, `ActInTown` and `ccPlayer::Attack` need them).
    fn call_ctx(&mut self, call: Call, p: Parts<'_>) -> i32 {
        match call {
            Call::FollowPlayer { .. }
            | Call::LeavePlayer { .. }
            | Call::FollowTarget { .. }
            | Call::FollowTargetDirc { .. } => self.call(call, p.scene, p.crew, p.rng),
            other => self.inner.call_ctx(other, p),
        }
    }

    fn w2p(&mut self, pos: V4) -> V4 {
        self.world.w2p(pos)
    }
}

impl Follow<'_> {
    /// `ccNavi::PathFindingInDungeon(from, to)` (gcmn 0x005148e0) of the
    /// member's navigation, through the inner runtime.
    fn path(&mut self, me: usize, from: V4, to: V4, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) -> i32 {
        self.inner.call(Call::PathFinding { me, from, to }, scene, crew, rng)
    }

    /// Kite gone (`getPartyMenberChar(0)` off the lists): stop, and forget
    /// following and going back.
    fn lost_player(&mut self, body: usize, crew: &mut Crew) {
        halt(crew, body);
        let k = ai_key(crew, body);
        if let Some(a) = crew.ais.get_mut(&k) {
            a.follow_sw = false;
            a.go_back_flag = false;
        }
    }

    /// `ccAI::FollowPlayer()` (gcmn 0x00581580): walk after Kite, toward a point
    /// 200 in front of him turned by `fpAngleOffset[slot]` (main 0x003782a0).
    /// In a dungeon a wall in the way asks the navigation for a route; in a
    /// field `CheckFrontObstacleF` starts a detour. It arrives within 70 of the
    /// point or `fpOkRange` (120) of Kite and sets off beyond 280, running
    /// beyond 350. The rules are in docs/engine/battle.md ("Following").
    pub fn follow_player(&mut self, body: usize, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) {
        let pl = self.party.members[0];
        let Some(pl) = pl.filter(|&p| scene.listed(p)) else {
            self.lost_player(body, crew);
            return;
        };
        let key = ai_key(crew, body);
        let p = scene.chars[body].pos_p;
        let mut dir = spc(crew, body).dirc;
        let dist_pl = crew.ais[&key].dist_pl;
        let dist = if spc(crew, body).move_flag {
            let pd = spc(crew, pl).dirc;
            let s0 = rad2deg(pd[2]);
            let k = self.party.slot_of(i32::from(scene.chars[body].id()));
            let off = self.mt.angle_offset(k);
            let a = deg2rad((i32::from(s0) + i32::from(off)) as i16);
            let mut g = geom::VF0;
            g[0] = add(g[0], mul(F_200, sinf(a)));
            g[1] = sub(g[1], mul(F_200, cosf(a)));
            let h = atan2f(sub(g[1], p[1]), sub(g[0], p[0]));
            let h = turn(h, 16384);
            let mut d = vsub(g, p);
            d[2] = 0;
            let f = geom::length_on(self.t.volume, d);
            if !le(dist_pl, F_200) {
                dir[2] = h;
            }
            f
        } else {
            dist_pl
        };
        let area = self.game.area;
        match area {
            1 => {
                let goal = scene.chars[pl].pos;
                self.detour(body, key, goal, &mut dir, scene, crew);
            }
            2 if self.check_front_obstacle(body, dir[2], scene, crew) => {
                let (a, b) = (scene.chars[body].pos, scene.chars[pl].pos);
                if self.path(body, a, b, scene, crew, rng) > 0 {
                    return;
                }
                if spc(crew, body).move_flag {
                    halt(crew, body);
                }
                return;
            }
            _ => {}
        }
        let (pl_move, pl_run, pl_dirc) = {
            let s = spc(crew, pl);
            (s.move_flag, s.run_flag, s.dirc)
        };
        if spc(crew, body).move_flag {
            if lt(dist, F_70) || lt(crew.ais[&key].dist_pl, self.mt.ok_range) {
                crew.ais.get_mut(&key).expect("the member's AI").go_back_flag = false;
                if pl_move && !pl_run && !le(dist, F_20) {
                    spc(crew, body).run_flag = false;
                } else {
                    dir[2] = pl_dirc[2];
                    halt(crew, body);
                }
            } else {
                let s = spc(crew, body);
                s.move_flag = true;
                if lt(dist, F_50) && s.run_flag {
                    s.run_flag = false;
                } else if !le(dist, F_250) && !s.run_flag && (!pl_move || pl_run) {
                    s.run_flag = true;
                }
            }
        } else if !le(dist, F_280) {
            if can_act(scene, crew, body, 1, self.t.volume) {
                let s = spc(crew, body);
                s.move_flag = true;
                if !pl_move || pl_run {
                    s.run_flag = true;
                }
                if !le(dist, F_350) {
                    s.run_flag = true;
                }
            }
        } else {
            crew.ais.get_mut(&key).expect("the member's AI").go_back_flag = false;
            spc(crew, body).move_flag = false;
        }
        if crew.ais[&key].detour_cnt <= 0 {
            spc(crew, body).dirc = dir;
        }
    }

    /// The field's detour, `FollowPlayer`'s and `FollowTarget`'s:
    /// `CheckFrontObstacleF(goal)`; blocked with no detour running, take
    /// `aiOpenDirc` at once (the body's heading is set here) and start a
    /// detour of 30 frames, 60 when only the widest ways are open (2); a
    /// detour running is renewed to 60 on a 2. A clear way (0) cuts a
    /// detour of 6 or more to 5.
    fn detour(&mut self, body: usize, key: usize, goal: V4, dir: &mut V4, scene: &mut Scene, crew: &mut Crew) {
        let r = self.check_front_obstacle_f(body, goal, scene, crew);
        let a = crew.ais.get_mut(&key).expect("the member's AI");
        if r > 0 {
            if a.detour_cnt <= 0 {
                dir[2] = self.state.ai_open_dirc;
                if r == 2 {
                    a.detour_cnt = 60;
                } else if a.detour_cnt <= 0 {
                    a.detour_cnt = 30;
                }
                spc(crew, body).dirc = *dir;
            } else if r == 2 {
                a.detour_cnt = 60;
            }
        } else if r == 0 && a.detour_cnt >= 6 {
            a.detour_cnt = 5;
        }
    }

    /// `ccAI::LeavePlayer()` (gcmn 0x00581cb0): step away from Kite (a member
    /// called over, mode 5), heading away from his `posP`. In a dungeon a wall
    /// that way asks for a route (the game passes the direction, scaled, as the
    /// goal); none: stop. Walking beyond 260 it arrives (`goBackFlag` 0, it
    /// turns about and stops); standing within 150 it starts walking if able.
    pub fn leave_player(&mut self, body: usize, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) {
        let pl = self.party.members[0];
        let Some(pl) = pl.filter(|&p| scene.listed(p)) else {
            self.lost_player(body, crew);
            return;
        };
        let key = ai_key(crew, body);
        let mut dir = spc(crew, body).dirc;
        let p = scene.chars[body].pos_p;
        let q = scene.chars[pl].pos_p;
        let h = heading(vsub(p, q));
        let dist = crew.ais[&key].dist_pl;
        if !spc(crew, body).move_flag || lt(dist, F_200) {
            dir[2] = h;
        }
        if self.game.area == 2 && self.check_front_obstacle(body, dir[2], scene, crew) {
            let a = scene.chars[body].pos;
            let b = scene.chars[pl].pos;
            let d = vscale(normalize(vsub(a, b)), F_280);
            if self.path(body, a, d, scene, crew, rng) != 0 {
                return;
            }
            if spc(crew, body).move_flag {
                halt(crew, body);
            }
            return;
        }
        if spc(crew, body).move_flag {
            if !le(dist, F_260) {
                crew.ais.get_mut(&key).expect("the member's AI").go_back_flag = false;
                dir[2] = turn(dir[2], 32768);
                spc(crew, body).dirc = dir;
                halt(crew, body);
            }
        } else if lt(dist, F_150) && can_act(scene, crew, body, 1, self.t.volume) {
            spc(crew, body).move_flag = true;
        }
        if spc(crew, body).move_flag {
            spc(crew, body).dirc = dir;
        }
    }

    /// `ccAI::FollowTarget(target)` (gcmn 0x00582090): close in on a target on
    /// the lists. `dircTg` is the heading to it and `distTg` the ground distance
    /// less both widths, truncated; the member turns to it beyond `noTurnRange`
    /// (from Mutation on at any distance). A dungeon's wall asks for a route, a
    /// field's obstacle a detour; then it runs or stops by `stopRange`,
    /// `attackRange` and, wary (mode 2), Kite's `territory`.
    pub fn follow_target(
        &mut self,
        body: usize,
        target: Option<usize>,
        scene: &mut Scene,
        crew: &mut Crew,
        rng: &mut dyn Rng,
    ) {
        let Some(tg) = target.filter(|&t| scene.listed(t)) else { return };
        let key = ai_key(crew, body);
        let p = scene.chars[body].pos_p;
        let mut dir = spc(crew, body).dirc;
        let t = self.world.w2p(scene.chars[tg].pos);
        let mut d = vsub(t, p);
        let h = heading(d);
        d[2] = 0;
        let dist =
            sub(sub(geom::length_on(self.t.volume, d), scene.chars[body].base().width), scene.chars[tg].base().width);
        let dist = whole(dist);
        let pr = param(self.t, &crew.ais[&key]);
        {
            let a = crew.ais.get_mut(&key).expect("the member's AI");
            a.dirc_tg = h;
            a.dist_tg = dist;
        }
        // From Mutation on it turns whatever the distance.
        let later = self.t.volume != Volume::Inf;
        if can_act(scene, crew, body, 2, self.t.volume) && (later || !le(dist, pr.no_turn_range)) {
            dir[2] = h;
        }
        let area = self.game.area;
        match area {
            2 if self.check_front_obstacle(body, dir[2], scene, crew) => {
                let (a, b) = (scene.chars[body].pos, scene.chars[tg].pos);
                if self.path(body, a, b, scene, crew, rng) != 0 {
                    return;
                }
                if spc(crew, body).move_flag {
                    halt(crew, body);
                }
                return;
            }
            1 => {
                let goal = scene.chars[tg].pos;
                self.detour(body, key, goal, &mut dir, scene, crew);
            }
            _ => {}
        }
        if scene.chars[body].skill_id < 2 && crew.ais[&key].detour_cnt <= 0 {
            spc(crew, body).dirc = dir;
        }
        let kite = self.party.members[0];
        if crew.ais[&key].mode == 2 {
            if spc(crew, body).move_flag {
                if lt(dist, pr.stop_range) {
                    halt(crew, body);
                    return;
                }
                if lt(distance_to_target(self.t.volume, scene, body, kite), pr.territory) {
                    spc(crew, body).run_flag = true;
                } else {
                    halt(crew, body);
                }
                return;
            }
            if !can_act(scene, crew, body, 1, self.t.volume) {
                return;
            }
            if !le(dist, pr.stop_range) || lt(distance_to_target(self.t.volume, scene, body, kite), pr.territory) {
                let s = spc(crew, body);
                s.move_flag = true;
                s.run_flag = true;
            }
            return;
        }
        if spc(crew, body).move_flag {
            if lt(dist, pr.stop_range) {
                halt(crew, body);
            } else {
                spc(crew, body).run_flag = true;
            }
        } else if can_act(scene, crew, body, 1, self.t.volume) && !le(dist, pr.attack_range) {
            let s = spc(crew, body);
            s.move_flag = true;
            s.run_flag = true;
        }
    }

    /// `ccAI::FollowTargetDirc(target)` (gcmn 0x00582ad0): face a target on
    /// the lists and not dying (`dead` 1). `dircTg` is the heading to its
    /// `posP`, `distTg` the ground distance less the target's width,
    /// truncated; the member turns to it when able (`CheckAction(2)`)
    /// beyond `noTurnRange`.
    pub fn follow_target_dirc(&mut self, body: usize, target: Option<usize>, scene: &mut Scene, crew: &mut Crew) {
        let Some(tg) = target.filter(|&t| scene.listed(t)) else { return };
        if scene.chars[tg].cond[crate::param::cond::DEAD] == 1 {
            return;
        }
        let key = ai_key(crew, body);
        let p = scene.chars[body].pos_p;
        let mut dir = spc(crew, body).dirc;
        let t = scene.chars[tg].pos_p;
        let mut d = vsub(t, p);
        let h = heading(d);
        d[2] = 0;
        let dist = whole(sub(geom::length_on(self.t.volume, d), scene.chars[tg].base().width));
        let pr = param(self.t, &crew.ais[&key]);
        {
            let a = crew.ais.get_mut(&key).expect("the member's AI");
            a.dirc_tg = h;
            a.dist_tg = dist;
        }
        if can_act(scene, crew, body, 2, self.t.volume) && !le(dist, pr.no_turn_range) {
            dir[2] = h;
        }
        spc(crew, body).dirc = dir;
    }

    /// `ccAI::CheckFrontObstacle(d)` (gcmn 0x00589e90): whether a wall
    /// stands within the body's reach along heading `d` (-999.0,
    /// [`FACING`]: the way the body faces): the line from its feet raised
    /// 99 to `bodyHit.radius + 1.1 speed` along the heading
    /// (`ccHitCheckLM`, mask 1). From Mutation on (gcmn 0x005b15d0) the
    /// reach is `1.5 bodyHit.radius`, whatever the speed.
    pub fn check_front_obstacle(&mut self, body: usize, d: F, scene: &Scene, crew: &mut Crew) -> bool {
        let mut a = scene.chars[body].pos;
        let s = spc(crew, body);
        let h = if geom::eq(FACING, d) { s.dirc[2] } else { d };
        a[2] = add(a[2], F_99);
        let r = if self.t.volume == Volume::Inf {
            add(s.body_hit.radius, mul(F_1_1, s.speed))
        } else {
            mul(F_1_5, s.body_hit.radius)
        };
        let mut v = [mul(r, sinf(h)), mul(mul(MINUS_ONE, r), cosf(h)), 0, 0];
        v = vadd(v, a);
        v[3] = ONE;
        !clear(self.world.line(a, v, 1, 0))
    }

    /// `ccAI::CheckFrontObstacleF(goal)` (gcmn 0x00589fe0): the field's look
    /// ahead toward `goal`: a line of 1.5 `r` (`r = CFOFP x bodyHit.radius`,
    /// CFOFP 2.5), clear: 0; else six probes in pairs at 45, 63 and 101 degrees,
    /// the first pair with an open one setting `aiOpenDirc` (returns 1, 2 for
    /// the last pair, -1 all blocked). The double arithmetic (`sqrt`, `atan2`)
    /// rounds as IEEE doubles do. The probes are in docs/engine/battle.md.
    pub fn check_front_obstacle_f(&mut self, body: usize, goal: V4, scene: &Scene, crew: &mut Crew) -> i32 {
        let s = spc(crew, body);
        let height = s.body_hit.height;
        let r = mul(self.mt.cfofp, s.body_hit.radius);
        let mut a = scene.chars[body].pos;
        let h = heading(vsub(goal, a));
        a[2] = add(a[2], height);
        let len = mul(F_1_5, r);
        let v = step(a, len, h);
        if clear(self.world.line(a, v, 1, 0)) {
            return 0;
        }
        let root2 = scaled(r, 2.0);
        let root5 = scaled(r, 5.0);
        let deg = i32::from(atan2_deg());
        let mut f = [0u8; 6];
        let probes = [
            (root2, turn(h, -8192)),
            (root2, turn(h, 8192)),
            (root5, turn(h, -deg)),
            (root5, turn(h, deg)),
            (mul(F_2, r), turn(h, -18432)),
            (mul(F_2, r), turn(h, 18432)),
        ];
        for (k, &(len, ang)) in probes.iter().enumerate() {
            let v = step(a, len, ang);
            f[k] = if clear(self.world.line(a, v, 1, 0)) {
                if clear(self.world.line(v, goal, 1, 0)) { 0 } else { 1 }
            } else {
                2
            };
        }
        // The first probe not blocked picks the way; when the next probe
        // is fully open and this one only half, the next one's way instead.
        let next_open = |k: usize| f[k + 1] == 0 && f[k] != 0;
        let (r, way) = if f[0] != 2 {
            (1, if next_open(0) { 8192 } else { -8192 })
        } else if f[1] != 2 {
            (1, if next_open(1) { -deg } else { 8192 })
        } else if f[2] != 2 {
            (1, if next_open(2) { deg } else { -deg })
        } else if f[3] != 2 {
            (1, if next_open(3) { -18432 } else { deg })
        } else if f[4] != 2 {
            (2, if next_open(4) { 18432 } else { -18432 })
        } else if f[5] != 2 {
            (2, 18432)
        } else {
            return -1;
        };
        self.state.ai_open_dirc = turn(h, way);
        r
    }
}

/// The end of a probe: `p` plus `len` along heading `a` (`len sinf(a)`,
/// `-len cosf(a)`, 0), w 1.0.
fn step(p: V4, len: F, a: F) -> V4 {
    let v = [mul(len, sinf(a)), mul(mul(MINUS_ONE, len), cosf(a)), 0, 0];
    let mut v = vadd(v, p);
    v[3] = ONE;
    v
}

/// `dptofp(dpmul(fptodp(r), sqrt(k)))`: `r` times a double square root,
/// rounded back to a float.
fn scaled(r: F, k: f64) -> F {
    ((f64::from(f32::from_bits(r)) * k.sqrt()) as f32).to_bits()
}

/// `(short)RAD2DEG((float)atan2(2.0, 1.0))`: the probes' second angle
/// (atan 2, 63.4 degrees) in 16-bit units.
fn atan2_deg() -> i16 {
    rad2deg((2.0f64.atan2(1.0) as f32).to_bits())
}

/// `ccAI::SetDircZ(dd)` (gcmn 0x00581500): the body faces `dd` (16-bit
/// units): `gDeg = dd`, `dirc.z = DEG2RAD((short)dd)`.
pub fn set_dirc_z(crew: &mut Crew, body: usize, dd: u16) {
    let key = ai_key(crew, body);
    if let Some(a) = crew.ais.get_mut(&key) {
        a.g_deg = dd;
    }
    spc(crew, body).dirc[2] = deg2rad(dd as i16);
}

impl MotionTables {
    /// `fpAngleOffset[k]` (main 0x003782a0) for a party slot `k` from
    /// `checkPartyMenberNum`, -1 (not in the party) reading the halfword
    /// before the array as the game does.
    pub fn angle_offset(&self, k: i32) -> u16 {
        self.fp_angle_offset[(k + 1).clamp(0, 4) as usize]
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;

    #[test]
    fn turning_wraps_in_sixteen_bits() {
        // A quarter turn from 0 is pi/2, which RAD2DEG truncates to 16383.
        assert_eq!(turn(0, 16384), deg2rad(16384));
        assert_eq!(rad2deg(turn(0, 16384)), 16383);
        // Half a turn from -pi wraps through the short to 0.
        assert_eq!(turn(geom::neg(geom::PI), 32768), 0);
        // atan 2 is 63.43 degrees: 11547 sixteen-bit units after the truncation.
        assert_eq!(atan2_deg(), 11547);
    }

    #[test]
    fn probes_scale_through_doubles() {
        // sqrt 2 rounded to a float; 10 sqrt 5 = 22.36068.
        assert_eq!(scaled(ONE, 2.0), 0x3fb5_04f3);
        assert_eq!(scaled(geom::k(10.0), 5.0), 0x41b2_e2ac);
        assert_eq!(geom::cmp(scaled(0, 2.0), 0), Ordering::Equal);
    }
}
