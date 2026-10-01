//! The field and the dungeon as the battle's code sees them: one [`Stage`]
//! borrowed for a frame's tasks implements piney-battle's world traits
//! ([`World`], [`NaviWorld`], [`FellowWorld`], [`MotionWorld`], [`KiteWorld`])
//! and the party AI's [`Runtime`] over the area's collision ([`Hits`]), the
//! camera ([`Camera`]), the actors' animation players ([`Cast`]) and the
//! running skills. Each method is the game function the trait names
//! (`ccTransPosW2P`, `ccLandHitCheck`, `_ccHitCheckLM`, `ccCheckCameraDeg`,
//! ...). What only shows or sounds goes into [`Show`]s, in order.

use std::cell::RefCell;

use piney_battle::entry;
use piney_battle::event::Event;
use piney_battle::fellow::{self, FellowWorld};
use piney_battle::flow::Skills;
use piney_battle::geom::{self, F as BF, M4, MINUS_ONE};
use piney_battle::kite::{self, KiteWorld, MapBounds};
use piney_battle::navi::{Navi, NaviWorld};
use piney_battle::party_ai::{self, Call, Crew, Runtime};
use piney_battle::rand::Rng;
use piney_battle::scene::Scene;
use piney_battle::tables::Tables;
use piney_battle::world::{AnmSlot, CharHit, Note, World};
use piney_battle::{Char as BChar, enemy_motion};

use crate::camera::{CamPad, Camera};
use crate::combat::cast::Cast;
use crate::combat::spc::{self, SpcRec};
use crate::ee::{self, F, ONE, V4};
use crate::hit::{self, Hits};

/// What the battle's code handed to presentation, in order: the effects
/// (numbers, hit marks, trails, dust, the portals' particles) and the
/// sounds read these. piney-effect's hook.
#[derive(Clone, Debug, PartialEq)]
pub enum Show {
    /// Kite's outputs (`kite::Out`), with his scene index.
    Kite(usize, kite::Out),
    /// A party member's (`fellow::Out`).
    Member(usize, fellow::Out),
    /// The entry control's (sounds, the circles' draws, a corpse's box).
    Entry(entry::Out),
    /// An enemy's presentation calls (`enemy_motion::Call`: sounds, the
    /// weapon trail and dust controllers, the camera shake, a gold
    /// goblin's dust).
    Enemy(usize, enemy_motion::Call),
    /// A dust ring an enemy's `ccEnemyDustCtrl` raised this frame
    /// ([`super::dust`]): `ccEnemyEffDustRing(ch, ofs, s, r, n, life, t)`
    /// for the enemy at `pos` facing `dirc`.
    DustRing { pos: V4, dirc: V4, ofs: V4, s: u32, r: u32, n: i32, life: i32, tex: i32 },
    /// A rule's event (numbers `FlyFont`, `HitMark`, particles ...).
    Rule(Event),
    /// A party member's chat balloon: `ccAI::ChatMessageSender`'s
    /// `ccChatMsg::OpenChat(body, text)` over the member (its scene index)
    /// with the text its last line left ([`piney_battle::party_chat`]).
    Chat(usize, Vec<u8>),
    /// `effSkillStart(who, sid, a, b)` as a skill request starts it.
    SkillStart { who: usize, sid: i32, a: i32, b: i32 },
    /// A member's `ccSpcChar::StartArmsEffect`:
    /// `startParticleEffect2(&weaponEffPos[a], &weaponEffPos[b], row,
    /// &armsEffectSW)` (Kite's come as `kite::Out::ArmsParticles`).
    ArmsParticles { who: usize, a: i32, b: i32, row: i32 },
    /// `ccChar::DispConditionEffect`'s or `ClearConditionEffect`'s doing to
    /// the character's condition effect: `setConditionEffect` with `num`
    /// (any old one killed first), `killConditionEffect`,
    /// `deleteConditionEffect`.
    ConditionEffect { who: usize, act: piney_battle::chara::CondFx, num: i32 },
    /// `effPhysicalSkillHitShockWave(pos, attr)`: a physical skill's 0x8002
    /// note, at its target position, `attr` its type & 0xfc.
    ShockWave { pos: V4, attr: i32 },
    /// A kill's experience, each member's share
    /// (`ccEntryFlyFontNewExp(19, exp, pos, ch)` over those in the party).
    Exp(piney_battle::exp::ExpGain),
    /// `ccThSkill`'s step of a running skill (spell systems, the heal
    /// sound, shock waves, footsteps).
    Skill(piney_battle::flow::Step),
    /// The sounds of that step with whom they belong to: the heal sound
    /// (`ccSeOn3D(75, target position)`) and the notes'
    /// `ccSeSetParamSPC(value, caster)`.
    SkillSounds { caster: Option<usize>, target_pos: V4, heal: bool, notes: Vec<u32> },
    /// `ccThSpc`'s shout: Kite names the party's operation.
    Shout(i8),
    /// `effHeal(who, n)` (main 0x001ccd20): an item's recovery on `who`.
    Heal { who: usize, n: i32 },
    /// What Data Drain's side effect starts (`DataDrainMenu` step 10), in
    /// the game's order: the heals, a MISS over whoever resisted,
    /// `effSkillStartEffect`, the exp lost and `effAfterDrain`.
    DrainSide(piney_battle::drain::SideStart),
    /// `ccEntryFlyFontNewLevelDown(23, pos, plw)` over Kite as step 11's
    /// frame 30 takes his level.
    DrainLevelDown(usize),
    /// `ccWordsPlay(sid, who)` (main 0x0017e290), from `_ccSkillRequest`
    /// (gcmn 0x00572b98): a character's own skill or an item used through
    /// it (stype 0 or 2), id 6 or more, names the skill. The sound task
    /// plays it for Kite and the party only (`base->type & 5`) and not
    /// while an event runs (`eventMng` +0x78c).
    Words { who: usize, sid: i32 },
    /// A boss's calls (`piney_battle::boss::Out`: sounds, flashes, its
    /// effects, the menu's locks, the camera, the stage), with its scene
    /// index.
    Boss(usize, piney_battle::boss::Out),
    /// An event NPC's start, with its stand-in's scene index
    /// ([`crate::field_npcs`]): `effTransfer` as a PC or the Administrator
    /// comes or goes, the Administrator's act -5 (its rings and sound).
    Npc(usize, crate::merchant::SysopEvent),
    /// The riding Grunty's sounds and dust (`piney_battle::ride::Out`'s
    /// `Sound` and `Smoke`; [`super::ride`]).
    Ride(piney_battle::ride::Out),
}

/// What `_ccSkillRequest` (gcmn 0x00572860) shows of a request: its
/// `effSkillStart` calls (on the target, or the caster for an item's own
/// effect), and the caster naming it ([`Show::Words`]) when the request
/// calls `ccWordsPlay`.
pub fn skill_shows(
    shows: &mut Vec<Show>,
    who: usize,
    target: Option<usize>,
    sid: i32,
    req: Option<&piney_battle::skill::Request>,
) {
    use piney_battle::event::Who;
    let Some(r) = req else { return };
    for &(on, a, b) in &r.skill_start {
        let on = match on {
            Who::Me => Some(who),
            Who::Target => target,
            Who::Char(c) => Some(c),
            Who::Nobody => None,
        };
        if let Some(on) = on {
            shows.push(Show::SkillStart { who: on, sid, a, b });
        }
    }
    if r.words {
        shows.push(Show::Words { who, sid });
    }
}

/// The field or dungeon for one frame of the battle's tasks.
pub struct Stage<'a> {
    pub t: &'a Tables,
    pub hits: &'a mut Hits,
    pub camera: &'a mut Camera,
    pub pad: CamPad,
    pub cast: &'a mut Cast,
    pub skills: &'a RefCell<Skills>,
    /// `game.area`.
    pub area: i32,
    /// `WORLD_MAN` +0x420: the map's bounds.
    pub bounds: MapBounds,
    /// Kite's position (`plw->pos`) as the frames that read it see it.
    pub player: V4,
    pub kite: Option<usize>,
    /// The dungeon's 2D map of the floor (`WORLD_MAN::Get2DMapPtr`) and the
    /// room's window (`Get2DMapInfo`).
    pub map2d: Vec<u8>,
    pub map2d_info: [i32; 3],
    /// `WORLD_MAN::CheckEventArea()`.
    pub event_area: bool,
    /// `checkPartyAnnihilation()`, as `ManualModeAI` reads it.
    pub annihilated: bool,
    pub shows: &'a mut Vec<Show>,
    /// `WORLD_MAN::Enter(pos)` was asked (a dungeon's entrance, its doors).
    pub enter: bool,
    /// `posView` as Kite's `CameraPosSet` last gave it: `cameraSet`'s
    /// player.
    pub view: V4,
    /// Enemies a hit asked to pick their target again
    /// (`ccEnemy::selectTarget`), in order.
    pub retarget: Vec<usize>,
    /// Kite's `actNum` for `ccSkillCheck`.
    pub kite_act: i16,
    /// The boxes' traps asked this frame (`ccGimBox::invokeTrap`): (box,
    /// opener, skill, true for trap 0's hit / false for a trap's skill,
    /// both on the command lists as the call was made), carried out by the
    /// entry control's frame when its objects are done. The box leaves its
    /// list (`deleteCmnd`) right after the call in the same frame, so
    /// `CalcBattleDamage`'s `ccCheckTarget` is the call's.
    pub traps: Vec<(usize, Option<usize>, i32, bool, bool)>,
    /// A Root Town's navigation, for `ActInTown`'s walks
    /// (`ccNavi::RouteSearchByMap` and the town's dummies); None in a
    /// field.
    pub town: Option<TownNav<'a>>,
    /// The registry and the party, for the party changes the members' AI
    /// asks (`resignParty`, `disbandSpc`: remote command 5); None where
    /// they are not carried out.
    pub spcs: Option<&'a mut crate::party::Spcs>,
    /// Each character's enemy skills that change conditions
    /// ([`super::chat::condition_skills`]), as `ChatMessageAttackTarget`
    /// counts them.
    pub tricks: Vec<i32>,
    /// The chat lines the enemies' hits raised on the party
    /// ([`super::chat::line_of`]), run by the frame when the entry
    /// control's pass is done.
    pub chats: Vec<Event>,
    /// The members' item uses (`ccUseItemRequest` from their AI), carried
    /// out by the runtime after the frame ([`super::MemberItem`]).
    pub item_uses: &'a mut Vec<super::MemberItem>,
}

/// What a Root Town gives the party's navigation: its landmarks with their
/// links and main lines (`ccSetNaviMap`), and the points
/// `naviPointNameTable` names.
#[derive(Clone, Copy)]
pub struct TownNav<'a> {
    pub map: &'a crate::navi::NaviMap,
    /// `naviPointNameTable` (gcmn 0x00653c80) by name: 1 `DMY_gate`, 2-7
    /// `DMY_merchant1`-`6`, the positions of the town file's dummies.
    pub points: &'a [V4],
}

impl Stage<'_> {
    fn w2p_of(&self, pos: V4) -> V4 {
        kite::w2p_pos(&self.bounds, self.player, pos).0
    }

    fn p2w_of(&self, pos: V4) -> V4 {
        kite::p2w_pos(&self.bounds, self.player, pos).0
    }

    /// `ccCheckCameraDeg(pos, deg)` (main 0x001da710): `pos` within `deg`
    /// either side of the line from the active camera's eye to its target,
    /// on the ground, all three through `W2PPos`.
    pub fn check_camera_deg(&self, pos: V4, deg: i16) -> bool {
        let cam = self.camera.active();
        let p = self.w2p_of(pos);
        let c = self.w2p_of(cam.pos);
        let v = self.w2p_of(cam.view);
        let a = ee::rad2deg(ee::atan2f(ee::sub(p[1], c[1]), ee::sub(p[0], c[0])));
        let b = ee::rad2deg(ee::atan2f(ee::sub(v[1], c[1]), ee::sub(v[0], c[0])));
        let d = (i32::from(deg) + (i32::from(a) - i32::from(b))) as i16;
        d > 0 && i32::from(d) < 2 * i32::from(deg)
    }

    /// `ccGetCameraTransparency(pos, width, height, far, len, &hide)`
    /// (main 0x001da8b0) with the active camera.
    pub fn transparency(&self, pos: V4, width: F, height: F, far: F, len: F, hide: &mut bool) -> F {
        let cam = self.camera.active();
        let p = self.w2p_of(pos);
        let c = self.w2p_of(cam.pos);
        let d = ee::vsub(c, p);
        let dist = ee::sqrtf_on(self.camera.volume, ee::dot(d, d));
        let t = ee::div(ee::from_int(8192 - i32::from(cam.deg[1])), 0x4600_0000);
        let u = ee::sub(ONE, t);
        let mut r = ee::add(ee::mul(width, t), ee::mul(height, u));
        if ee::lt(r, 0x4334_0000) {
            r = 0x4334_0000;
        }
        let r2 = ee::mul(0x3fa0_0000, r);
        if !*hide && ee::lt(dist, r2) {
            *hide = true;
            let near = ee::mul(0x3f4c_cccd, r);
            let f = ee::sub(dist, near);
            return if ee::le(f, 0) { 0 } else { ee::div(f, ee::sub(r2, near)) };
        }
        *hide = false;
        if ee::le(far, 0) || ee::le(dist, far) {
            return ONE;
        }
        let over = ee::sub(dist, far);
        if ee::le(over, len) { ee::div(ee::sub(len, over), len) } else { 0 }
    }

    /// `ccChar::Draw()`'s test for a party character at `pos` (the area's
    /// fade: 7000 over 600 outside a town): its transparency and whether it
    /// is drawn (under 0.05 not, unless the camera's nearness faded it).
    pub fn char_draw(&self, pos: V4, width: F, height: F, set_transparency: F, trans_dist: bool) -> (F, bool) {
        let eye = self.camera.active().kind == crate::camera::kind::EYE;
        let mut hide = !trans_dist || eye;
        let (far, len) = if self.area == 0 { (0x457a_0000, 0x43c8_0000) } else { (0x45da_c000, 0x4416_0000) };
        let fade = self.transparency(pos, width, height, far, len, &mut hide);
        let t = ee::mul(set_transparency, fade);
        (t, !(ee::lt(t, 0x3d4c_cccd) && !hide))
    }

    fn body(&self, who: usize, h: &CharHit, manual: bool) -> hit::Body {
        spc::to_body(who, self.kite, h, manual)
    }

    /// `ccSpcChar::HitCheck(movePos)` (gcmn 0x0059ee20) on Kite, as
    /// `fellow::Frame::hit_check` does it for a member.
    #[allow(clippy::too_many_arguments)]
    fn kite_hit_check(
        &mut self,
        me: usize,
        ch: &BChar,
        hit: &mut CharHit,
        now_speed: BF,
        manual: bool,
        move_pos: &mut V4,
    ) -> i32 {
        const LAND: u32 = 0x2000_0001;
        const WALL: u32 = 0x4000_0001;
        let mut res = 0;
        let mv = *move_pos;
        let pos = ch.pos;
        let land = |s: &mut Self, mut v: V4| {
            v[2] = s.hits.land(v, LAND);
            v
        };
        let mut hp1 = mv;
        for k in 0..3 {
            hp1[k] = ee::add(hp1[k], pos[k]);
        }
        hp1[3] = ONE;
        hp1 = land(self, hp1);
        hit.pos = hp1;
        hit.radius = ee::add(ch.base().width, now_speed);
        if ch.spc_char.act_num == 14 || ch.cond[piney_battle::param::cond::DEAD] == 4 {
            self.hit_switch(me, hit, false);
        }
        let saved = hit.mask;
        if manual {
            hit.mask = 0xffff_fff8;
        }
        let height = hit.height;
        let raised = |mut v: V4| {
            v[2] = ee::add(v[2], height);
            v
        };
        let clear = |d: BF| d == MINUS_ONE;
        if self.collide(me, hit) != 0 {
            if self.hits.char_type & 2 != 0 {
                res |= 1;
            }
            hp1 = ee::vadd(mv, hit.offset);
            for k in 0..3 {
                hp1[k] = ee::add(hp1[k], pos[k]);
            }
            hp1[3] = ONE;
            hp1 = land(self, hp1);
            hit.pos = hp1;
            if self.collide(me, hit) != 0 {
                if self.hits.char_type & 2 != 0 {
                    res |= 1;
                }
                let mut hp2 = ee::vadd(hp1, hit.offset);
                hp2 = land(self, hp2);
                hp2[3] = ONE;
                hp1 = ee::vscale(ee::vadd(hp1, hp2), 0x3f00_0000);
                hp1[3] = ONE;
                hp1 = land(self, hp1);
            }
            let mut m = if !clear(self.line(raised(pos), raised(hp1), WALL, 1)) { ee::VF0 } else { ee::vsub(hp1, pos) };
            let mut b = raised(ee::vadd(pos, m));
            b[3] = ONE;
            if !clear(self.line(raised(pos), b, WALL, 1)) {
                m = ee::VF0;
            }
            *move_pos = m;
            let run = ch.spc_char.flags & spc::flag::RUN != 0;
            let spd = if run { 0x4100_0000 } else { ONE };
            let n = ee::neg(spd);
            if ee::lt(m[0], spd) && !ee::le(m[0], n) && ee::lt(m[1], spd) && !ee::le(m[1], n) {
                res |= 2;
            }
        } else if !clear(self.line(raised(pos), raised(hp1), WALL, 1)) {
            *move_pos = ee::VF0;
        }
        if manual {
            hit.mask = saved;
        }
        res
    }

    /// A party character's record through `f`, written back.
    fn with_rec(&mut self, scene: &mut Scene, crew: &mut Crew, who: usize, f: impl FnOnce(&mut SpcRec, &mut Hits)) {
        let td = self.cast.get(who).is_none_or(|a| a.trans_dist);
        let mut r = SpcRec::read(scene, crew, who, self.kite, td);
        f(&mut r, self.hits);
        let t = r.write(scene, crew);
        if let Some(a) = self.cast.get_mut(who) {
            a.trans_dist = t;
        }
    }
}

impl World for Stage<'_> {
    fn w2p(&mut self, pos: V4) -> V4 {
        self.w2p_of(pos)
    }
    fn p2w(&mut self, pos: V4) -> V4 {
        self.p2w_of(pos)
    }
    fn land(&mut self, pos: V4, mask: u32) -> BF {
        self.hits.land(pos, mask)
    }
    fn hit_attribute(&mut self) -> u32 {
        self.hits.attribute()
    }
    fn line(&mut self, from: V4, to: V4, mask: u32, kind: i32) -> BF {
        match self.hits.line(from, to, mask, kind) {
            Some((_, d)) => d,
            None => MINUS_ONE,
        }
    }
    fn collide(&mut self, who: usize, hit: &mut CharHit) -> i32 {
        let mut b = self.body(who, hit, false);
        let r = self.hits.collision_detection(&mut b);
        hit.offset = b.offset;
        hit.attribute = b.attribute;
        r
    }
    fn hit_char_type(&mut self) -> u32 {
        self.hits.char_type
    }
    fn hit_switch(&mut self, who: usize, hit: &mut CharHit, on: bool) {
        let mut b = self.body(who, hit, false);
        self.hits.set_hit_sw(&mut b, on);
        hit.sw = b.sw;
    }
    fn hit_sync(&mut self, who: usize, hit: &CharHit) {
        let b = self.body(who, hit, false);
        self.hits.sync(&b);
    }
    fn camera_deg(&mut self, pos: V4, deg: i16) -> bool {
        self.check_camera_deg(pos, deg)
    }
    fn camera_transparency(&mut self, pos: V4, width: BF, height: BF, far: BF, len: BF) -> BF {
        let mut hide = false;
        self.transparency(pos, width, height, far, len, &mut hide)
    }
    fn enemy_ccs(&mut self, who: usize, row: i32) {
        self.cast.enemy_ccs(who, row);
    }
    fn anim_set(&mut self, who: usize, slot: AnmSlot, name: &str) {
        self.cast.set(who, slot, name);
    }
    fn anim_frame(&mut self, who: usize, slot: AnmSlot) -> u16 {
        self.cast.get(who).map_or(0, |a| a.frame(slot))
    }
    fn anim_forward(&mut self, who: usize, slot: AnmSlot, step: u16) -> i16 {
        self.cast.get_mut(who).map_or(0, |a| a.forward(slot, step))
    }
    fn anim_notes(&mut self, who: usize, slot: AnmSlot) -> Vec<Note> {
        self.cast.get(who).map_or_else(Vec::new, |a| a.notes(slot))
    }
    fn eff_pat_num(&mut self, who: usize, name: &str) -> Option<u16> {
        let crate::combat::cast::Look::Gimmick(row) = self.cast.get(who)?.look else { return None };
        let g = self.cast.looks.gimmick(row)?;
        crate::foe::eff_pat_num(&g.model.body.file, name).ok()
    }
}

impl NaviWorld for Stage<'_> {
    fn map_2d_info(&mut self) -> [i32; 3] {
        self.map2d_info
    }
    fn map_2d(&mut self) -> &[u8] {
        &self.map2d
    }
    /// `ccNavi::RouteSearchByMap` in a Root Town (piney-world's
    /// [`crate::navi::Navi::route_search`]); nothing outside one.
    fn route_search_by_map(&mut self, navi: &mut Navi, finish: &mut i16, start: V4, goal: V4) -> i32 {
        let Some(t) = self.town else { return 0 };
        let mut n = crate::navi::Navi::default();
        let r = n.route_search(t.map, start, goal, self.hits);
        navi.goal_pos = n.goal_pos;
        navi.name = n.name;
        navi.step = n.step;
        navi.landmark = n.landmark;
        navi.dist = n.dist;
        navi.dirc = n.dirc;
        navi.route = n.route;
        *finish = n.finish;
        i32::from(r)
    }
    fn navi_point(&mut self, name: i16) -> V4 {
        let t = self.town.and_then(|t| usize::try_from(name).ok().and_then(|i| t.points.get(i)).copied());
        t.unwrap_or(ee::VF0)
    }
    fn marker(&mut self, k: i32) -> V4 {
        self.town.map_or(ee::VF0, |t| t.map.mark(k).pos)
    }
}

impl FellowWorld for Stage<'_> {
    fn trans_mode(&mut self) -> bool {
        self.hits.trans.is_some()
    }
    fn trans_center(&mut self) -> V4 {
        self.hits.trans.unwrap_or(ee::VF0)
    }
    fn event_area(&mut self) -> bool {
        self.event_area
    }
    /// `ccChar::Draw()` on the member: whether it is drawn where its last
    /// frame left it (the arms effect runs only then; the draw itself
    /// happens with the field's).
    fn draw(&mut self, who: usize) -> bool {
        let Some(a) = self.cast.get(who) else { return false };
        let (_, drawn) = self.char_draw(a.ch.pos, a.ch.width, a.ch.height, ONE, a.trans_dist);
        // ccChar::Draw's blend: the affect's tint (a flash counts down).
        if let Some(a) = self.cast.get_mut(who) {
            a.ch.blend = a.ch.affect.blend();
        }
        drawn
    }
}

impl enemy_motion::MotionWorld for Stage<'_> {
    /// `checkCameraShakeRange(pos)` (main 0x00162f10): `pos` in the
    /// active camera's view (`ccCheckCameraDeg(pos, 12288)`) and nearer
    /// its eye than 2000.
    fn shake_range(&mut self, pos: V4) -> bool {
        if !self.check_camera_deg(pos, 12288) {
            return false;
        }
        let d = ee::vsub(pos, self.camera.active().pos);
        ee::lt(ee::sqrtf_on(self.camera.volume, ee::dot(d, d)), 0x44fa_0000)
    }

    fn call(&mut self, who: usize, c: enemy_motion::Call, at: &mut enemy_motion::At) {
        use enemy_motion::Call as C;
        match c {
            C::SkillRequest { target, sid } => {
                let (_, req) = self.skills.borrow_mut().request(at.t, at.scene, who, target, sid, 0, at.rand);
                skill_shows(self.shows, who, target, sid, req.as_ref());
            }
            C::Draw { matrix } => {
                if let Some(a) = self.cast.get_mut(who) {
                    a.matrix = Some(matrix);
                    a.drawn = true;
                    let pos = at.scene.chars[who].pos;
                    a.ch.pos = pos;
                }
            }
            // dispEnemy's second model (a middle boss), drawn before the
            // body (ccChar::Draw).
            C::DrawSecond { matrix, alpha } => {
                if let Some(a) = self.cast.get_mut(who) {
                    a.second_draw = Some((matrix, alpha));
                    a.drawn = true;
                }
            }
            // ccEnemyCheckNote's cameraShake(kind, 2, 20, 2): a turning
            // shake, its turn from rand().
            C::CameraShake { kind } => self.camera.shake.shake(kind, 2, 20, 2, &mut || at.rand.rand()),
            C::TrapDamage { target, sid } => {
                let listed = at.scene.listed(who) && target.is_some_and(|t| at.scene.listed(t));
                self.traps.push((who, target, sid, true, listed));
            }
            C::TrapSkill { target, sid } => self.traps.push((who, target, sid, false, false)),
            C::Rule(piney_battle::enemy_ai::Out::Rule(e)) if super::chat::line_of(&e).is_some() => {
                self.chats.push(e);
                self.shows.push(Show::Enemy(who, C::Rule(piney_battle::enemy_ai::Out::Rule(e))));
            }
            c => self.shows.push(Show::Enemy(who, c)),
        }
    }
}

impl Runtime for Stage<'_> {
    fn call(&mut self, call: Call, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) -> i32 {
        match call {
            Call::SkillRequest { me, target, sid } => {
                let (_, req) = self.skills.borrow_mut().request(self.t, scene, me, target, sid, 0, rng);
                skill_shows(self.shows, me, target, sid, req.as_ref());
                0
            }
            Call::ManualModeAi { ch, ev } => {
                let annihilated = self.annihilated;
                self.with_rec(scene, crew, ch, |r, hits| r.spc_ref().manual_mode_ai(ev, annihilated, hits));
                0
            }
            Call::FaceTalk { me, g_deg } => {
                let s = crew.spc.entry(me).or_default();
                s.dirc[2] = geom::set_dirc(s.dirc[2], geom::deg2rad(g_deg as i16), 64);
                0
            }
            Call::TransferOut { ch } => {
                self.with_rec(scene, crew, ch, |r, hits| r.spc_ref().transfer_out(hits));
                0
            }
            Call::TransferIn { ch } => {
                self.with_rec(scene, crew, ch, |r, _| r.spc_ref().transfer_in());
                0
            }
            Call::HitCheckLm { from, to, mask } => self.line(from, to, mask, 0) as i32,
            Call::HitEnable { ch } => {
                let s = crew.spc.entry(ch).or_default();
                let mut h = s.body_hit;
                self.hit_switch(ch, &mut h, true);
                crew.spc.entry(ch).or_default().body_hit = h;
                0
            }
            // Remote command 5 leaving the party: resignParty (a member)
            // or disbandSpc, where the registry is at hand (a town).
            Call::ResignParty { id } | Call::DisbandSpc { id } if self.spcs.is_some() => {
                let how = if matches!(call, Call::ResignParty { .. }) {
                    crate::ai::PartyLeave::Resign
                } else {
                    crate::ai::PartyLeave::Disband
                };
                let ids: Vec<(i32, usize)> = (0..scene.chars.len())
                    .filter(|&w| scene.chars[w].is_pc() && Some(w) != self.kite)
                    .map(|w| (i32::from(scene.chars[w].id()), w))
                    .collect();
                let cast = &*self.cast;
                let td = |w: usize| cast.get(w).is_none_or(|a| a.trans_dist);
                let mut chars = spc::BattleChars::read(scene, crew, &ids, self.kite, &td);
                if let Some(spcs) = self.spcs.as_deref_mut() {
                    spcs.leave(id, how, &mut chars);
                }
                let cast = &mut *self.cast;
                chars.finish(scene, crew, &mut |w, t| {
                    if let Some(a) = cast.get_mut(w) {
                        a.trans_dist = t;
                    }
                });
                0
            }
            // A member's item: the runtime carries the use out after the
            // frame (the rules, then its steps).
            Call::UseItemRequest { me, target, item, flag } => {
                self.item_uses.push(super::MemberItem { user: me, target, code: item, arg: flag });
                0
            }
            // The chat window's queue and the party's changes are the
            // field's, not reached in the opening's fights.
            _ => 0,
        }
    }

    /// A chat line (`ccAI::ChatMessage*`) and the chat's sender
    /// (`ccAI::ChatMessageSender`: the balloon it opens goes to the shows)
    /// run with the decision's whole context; the rest as [`Stage::call`].
    fn call_ctx(&mut self, call: Call, p: party_ai::Parts<'_>) -> i32 {
        match call {
            Call::Chat { me, msg } => p.ctx(self).chat_line(me, msg),
            Call::ChatMessageSender { me } => {
                let body = p.crew.ais.get(&me).map_or(me, |a| a.body);
                let opened = p.ctx(self).chat_sender(me);
                if let Some(text) = opened {
                    self.shows.push(Show::Chat(body, text));
                }
                0
            }
            _ => self.call(call, p.scene, p.crew, p.rng),
        }
    }

    fn foe_condition_skills(&mut self, ch: usize) -> i32 {
        self.tricks.get(ch).copied().unwrap_or(0)
    }

    fn w2p(&mut self, pos: V4) -> V4 {
        self.w2p_of(pos)
    }
}

impl KiteWorld for Stage<'_> {
    fn out(&mut self, o: kite::Out) {
        let me = self.kite.unwrap_or(0);
        match o {
            kite::Out::AddCenter { x, y } => self.hits.add_center(x, y),
            kite::Out::Enter { .. } => self.enter = true,
            kite::Out::Matrix { pos, dirc } => {
                if let Some(a) = self.cast.get_mut(me) {
                    a.ch.pos = pos;
                    a.ch.dirc = dirc;
                }
            }
            kite::Out::Rule(Event::EnemyRetarget(piney_battle::Who::Char(c))) => self.retarget.push(c),
            kite::Out::Rule(e) if super::chat::line_of(&e).is_some() => {
                self.chats.push(e);
                self.shows.push(Show::Kite(me, o));
            }
            // Acts 23 and 24: the blades take ghoCamArmsT (hidden while
            // GateHackingOut has their dispSW 0).
            kite::Out::GateArms => {
                let t = self.cast.gate.arms_alpha();
                if let Some(a) = self.cast.get_mut(me) {
                    a.arms = Some(t);
                }
            }
            o => self.shows.push(Show::Kite(me, o)),
        }
    }
    fn camera_type(&mut self) -> i32 {
        self.camera.active().kind
    }
    fn camera_id(&mut self) -> i32 {
        i32::from(self.camera.cam_id)
    }
    fn camera_rot(&mut self) -> V4 {
        self.camera.rot()
    }
    fn camera_reset(&mut self) -> Option<BF> {
        let a = self.camera.active();
        a.reset_flag.then_some(a.reset_dirc)
    }
    fn clear_camera_reset(&mut self) {
        self.camera.active_mut().reset_flag = false;
    }
    fn camera_set_eye_level(&mut self, pos_eye: V4, angle: &mut V4) {
        self.view = pos_eye;
        let pad = self.pad;
        self.camera.eye_level(pos_eye, angle, &pad);
    }
    fn camera_set_manual(&mut self, pos_view: V4) {
        self.view = pos_view;
        let pad = self.pad;
        let area = self.area;
        self.camera.set_manual(pos_view, &pad, area, self.hits);
    }
    fn camera_set(&mut self) {
        let p = [self.view[0], self.view[1], 0, ONE];
        match self.hits.bounds {
            Some(b) => self.camera.set_wrapped(p, b),
            None => self.camera.set(p),
        }
    }
    fn hit_check(
        &mut self,
        me: usize,
        ch: &BChar,
        hit: &mut CharHit,
        now_speed: BF,
        manual: bool,
        move_pos: &mut V4,
    ) -> i32 {
        self.kite_hit_check(me, ch, hit, now_speed, manual, move_pos)
    }
    fn hit_result(&mut self) -> Option<u32> {
        (self.hits.num != 0).then_some(self.hits.nearest.att)
    }
    fn trans_mode(&mut self) -> bool {
        self.hits.trans.is_some()
    }
    fn trans_center(&mut self) -> V4 {
        self.hits.trans.unwrap_or(ee::VF0)
    }
    fn draw(&mut self, me: usize, pos: V4, set_transparency: BF, transparency: &mut BF) -> bool {
        let Some(a) = self.cast.get(me) else { return false };
        let (w, h, td) = (a.ch.width, a.ch.height, a.trans_dist);
        let (t, drawn) = self.char_draw(pos, w, h, set_transparency, td);
        *transparency = t;
        if let Some(a) = self.cast.get_mut(me) {
            a.ch.pos = pos;
            a.alpha = t;
            a.drawn = drawn;
            a.ch.blend = a.ch.affect.blend();
        }
        drawn
    }
    fn skill_check(&mut self, scene: &Scene, ch: usize) -> i32 {
        self.skills.borrow().check(self.t, scene, ch, self.kite_act)
    }
    fn gate_hacking_out(
        &mut self,
        _me: usize,
        ch: &mut BChar,
        p: &mut kite::Player,
        spc: &mut piney_battle::party_ai::Spc,
    ) {
        let k = super::gate_out::Kite {
            pos: &mut ch.pos,
            flags: &mut ch.spc_char.flags,
            act: &mut ch.spc_char.act_num,
            speed_rate: &mut spc.speed_rate,
            prog: &mut p.prog_ctrl_flag,
            camera_flag: &mut p.camera_flag,
            pos_cam: &mut p.pos_cam,
            pos_view: &mut p.pos_view,
        };
        self.cast.gate.run(self.camera, k);
    }
}

/// `ccTransPosW2P` / `P2W` for the enemies' rules (`enemy_ai::Frame`):
/// Kite's position and the bounds, fixed for the entry control's task.
pub struct PlayerFrame {
    pub bounds: MapBounds,
    pub player: V4,
}

impl piney_battle::enemy_ai::Frame for PlayerFrame {
    fn w2p(&self, v: V4) -> V4 {
        kite::w2p_pos(&self.bounds, self.player, v).0
    }
    fn p2w(&self, v: V4) -> V4 {
        kite::p2w_pos(&self.bounds, self.player, v).0
    }
}

/// The matrix `dispEnemy` built as glam's.
pub fn mat4(m: &M4) -> glam::Mat4 {
    glam::Mat4::from_cols_array(&std::array::from_fn(|i| f32::from_bits(m[i / 4][i % 4])))
}
