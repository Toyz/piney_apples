//! Skeith in the field: what `ccBossEntryStart(0)` (gcmn 0x0045b2a0)
//! starts for an event's `entry 7 0` - `ccThBossEffect` and `ccThBoss01`
//! at priority 66, after the entry control (64) and before the effects
//! (80) - over the battle's scene ([`piney_battle::boss`]).
//!
//! `ccThBoss01` (0x0047b340) news the `ccBoss01` and runs `Main` each
//! frame; once `CheckExit()` it sets its task parameter's +0x14, which an
//! event's `absent 7` reads, and breathes on. The effect manager's pass is
//! the first thing [`piney_battle::boss::Boss::main`] does.
//!
//! An affect on the boss from Kite's or a member's frame (their contexts
//! carry no [`BossEnv`]) waits on the boss ([`boss::apply_queued`]) and is
//! applied first in its task, the same frame.
//!
//! The boss's camera ([`crate::bosscam`]) is made with it and runs after
//! its `Move`; the dead effect hands the view to camera 3.

use std::collections::HashMap;
use std::rc::Rc;

use piney_battle::affect::AffectCtx;
use piney_battle::boss::{self, Boss, BossEnv, Out};
use piney_battle::chara::{AffectFunc, Env};
use piney_battle::enemy_ai::World;
use piney_battle::event::{Event, Events, Who};
use piney_battle::param::cond;
use piney_battle::world::CharHit;

use super::Combat;
use super::cast::{Actor, Look};
use super::stage::Show;
use crate::body::Body;
use crate::bosscam::BossCam;
use crate::camera::{CamPad, Camera, id};
use crate::cinema::{Cinema, Name as CinemaName, Sent as CinemaSent};
use crate::ee::{self, ONE, V4};
use crate::lattice::{Lattice, StripVertex};

/// `bossTbl`'s row for Skeith (`ccGetBossParam(0)`).
pub const SKEITH: usize = 0;

/// The boss's file and model (`x11`, `CMP_trall`), and the effects' file.
pub const FILE: &str = "x11";
pub const CLUMP: &str = "CMP_trall";
pub const EFF_FILE: &str = "xeffect";
/// The skill names' texture in [`FILE`] and the cinema number Skeith's
/// magic asks for it with (`_g_cinemaSkillName[2]`).
pub const SKILL_NAME_TEX: &str = "TEX_ske_skl";
pub const SKEITH_CINEMA_NAME: i32 = 2;
/// The wave's animation in it.
pub const ANM_WAVE: &str = "ANM_xx11wave";

/// `InitBossCamera(200, 1000)`'s `transfer`: the eye 1000 behind Kite and
/// 200 up at its nearest.
pub const CAM_TRANSFER: V4 = [0, 0x447a_0000, 0x4348_0000, ONE];

/// A clip's frames and whether it loops, by name, in the boss's files.
pub type Clips = HashMap<String, (u32, bool)>;

/// What the field keeps of the boss's tasks.
pub struct BossRun {
    /// The boss's scene index.
    pub me: usize,
    /// `ccThBoss01`'s parameter +0x14: the boss has exited.
    pub exit: bool,
    pub clips: Rc<Clips>,
    /// `bodyHit` (`ccCharHit`): radius `base->width`, height
    /// `base->height`, its kind `base->type`.
    pub hit: CharHit,
    pub look: Rc<BossLook>,
    /// `m_aiMngr`'s after-images being drawn.
    pub afterimages: Vec<AfterImage>,
    /// The wave's `DrawParts` this frame: its animation's pose time, at the
    /// boss's place and heading.
    pub wave: Option<(u32, V4, V4)>,
    /// `bossCam` and `bossCamSW` (+0x14c, +0x150): the camera, and whether
    /// `Main` still runs it.
    pub cam: Option<BossCam>,
    pub cam_sw: bool,
    /// `ccBufferReverce::MakePacket` this frame (the magic's last part):
    /// the frame inverted on the boss's own layer.
    pub reverse: bool,
    /// The cross's trail (`ccLattice(2, 7)`, +0x2935c) and what its `Disp`
    /// sent this frame, with its sort key.
    pub lattice: Lattice,
    pub trail: (Vec<StripVertex>, u32),
    /// `ccBossEffManager`'s cinema (+0x00) and what its `Draw` sent this
    /// frame.
    pub cinema: Cinema,
    pub cinema_sent: CinemaSent,
}

/// One `ccBossAfterImageEx`: the boss's pose when it was entered, fading
/// (Skeith's `m_eai`: colour -1 at `colorAlpha` 100, `transparency` 1.0 by
/// `transSpd` -0.05 a draw, `maxFrame` 20), drawn on effLayer.
#[derive(Clone, Debug, PartialEq)]
pub struct AfterImage {
    pub clip: String,
    pub time: u32,
    pub pos: V4,
    pub dirc: V4,
    pub transparency: f32,
}

/// `m_eai.transSpd`.
pub const AFTERIMAGE_FADE: f32 = 0.05;

/// What the boss's look needs from the disc: the body, every clip of
/// its files, and the effects' file (the wave).
pub struct BossLook {
    pub body: Rc<Body>,
    pub clips: Rc<Clips>,
    pub eff: Rc<piney_desktop::assets::SceneFile>,
    pub eff_morphers: HashMap<u32, u32>,
    /// The cinema's skill name (`_g_cinemaSkillName[2]`: x11's
    /// `TEX_ske_skl`, row 0), the only one Skeith asks for.
    pub skill_name: Option<CinemaName>,
}

impl BossLook {
    pub fn load(archive: &std::sync::Arc<piney_data::archive::Archive>) -> piney_data::Result<BossLook> {
        let body = Rc::new(Body::read(archive, FILE, CLUMP)?);
        let mut clips = Clips::new();
        for stem in [FILE, EFF_FILE] {
            let ccs = piney_data::ccs::Ccs::parse(archive.inflate_named(stem)?)?;
            let scene = piney_data::scene::Scene::read(&ccs)?;
            for a in piney_data::anim::Animation::all(&ccs, &scene)? {
                if let Some(name) = ccs.object_name(a.object) {
                    clips.insert(name.to_string(), (a.frames, a.looping));
                }
            }
        }
        let eff = Rc::new(piney_desktop::assets::SceneFile::read(archive, EFF_FILE)?);
        let eff_morphers = piney_data::anim::morphers(&eff.ccs).unwrap_or_default();
        let skill_name = piney_desktop::message::WindowTexture::read_named(archive, FILE, SKILL_NAME_TEX, None)
            .map(|w| CinemaName { tex: w.tex, tex_h: w.tex_h, row: 0 });
        Ok(BossLook { body, clips: Rc::new(clips), eff, eff_morphers, skill_name })
    }
}

impl Combat {
    /// `ccBossEntryStart(0)`: Skeith made (`ccBoss01::ccBoss01`) at the
    /// arena's centre `center` (`InitCenterPos`, `DMY_center01`), 500
    /// below Kite, and its actor. Nothing without the tables or a Kite.
    pub fn start_boss(&mut self, look: &Rc<BossLook>, center: V4, env: &Env, camera: &mut Camera) {
        if self.boss.is_some() {
            return;
        }
        let d = self.data.clone();
        let (Some(data), Some(kite)) = (d.skeith.as_ref(), self.kite) else { return };
        let Some(row) = d.t.bosses.get(SKEITH).cloned() else { return };
        let mut ch = piney_battle::Char::foe(row);
        // ccChar::ccChar: no condition (conditionNum -1).
        ch.condition_num = -1;
        ch.affect.func = AffectFunc::Boss;
        let me = self.scene.add(ch, 3);
        let kite_pos = self.scene.chars[kite].pos;
        let kite_dirc = self.kite_dirc();
        let clips = look.clips.clone();
        let clip = |name: &str| clips.get(name).copied();
        let party = self.party;
        let check = |_: usize| 0;
        let benv = BossEnv { t: &d.t, data, clips: &clip, env, game_over: false };
        let actx =
            AffectCtx { party: &party, menu: true, skill_check: &check, boss: Some(&benv), volume: self.data.volume };
        let mut none = |_| None;
        let mut cx = boss::Cx {
            t: &d.t,
            data,
            scene: &mut self.scene,
            party: &party,
            world: World { player: Some(kite), ..World::default() },
            env,
            actx: &actx,
            rand: &mut self.rand,
            cc: &mut self.cc,
            clips: &clip,
            collide: &mut none,
            game_over: false,
            boss_cam: true,
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        let b = Boss::new(&mut cx, kite_pos, kite_dirc, center);
        let out = std::mem::take(&mut cx.out);
        drop(cx);
        for o in out {
            self.shows.push(Show::Boss(me, o));
        }
        let pos = self.scene.chars[me].pos;
        let (h, w) = (self.scene.chars[me].base().height, self.scene.chars[me].base().width);
        let clip0 = b.anm.clip.clone().unwrap_or_default();
        if let Some(a) = Actor::new(look.body.clone(), &clip0, Look::Boss, pos, b.dirc, h, w) {
            self.cast.actors.insert(me, a);
        }
        if let Some(f) = self.scene.chars[me].foe_state_mut() {
            f.boss = Some(Box::new(b));
        }
        let ty = self.scene.chars[me].ty() as u32;
        let hit = CharHit { pos, radius: w, height: h, kind: ty, ..CharHit::default() };
        self.boss = Some(BossRun {
            me,
            exit: false,
            clips: look.clips.clone(),
            hit,
            look: look.clone(),
            afterimages: Vec::new(),
            wave: None,
            // InitBossCamera(200, 1000), the constructor's last steps but two.
            cam: Some(BossCam::new(camera, CAM_TRANSFER, kite_pos)),
            cam_sw: true,
            reverse: false,
            lattice: Lattice::new(2, 7),
            trail: (Vec::new(), 0),
            cinema: Cinema::new(),
            cinema_sent: CinemaSent::default(),
        });
    }

    /// `ccThBossEffect` and `ccThBoss01`'s frame: the queued affects,
    /// `ccBoss01::Main`, the boss's actor set from it; what it asks goes to
    /// the shows ([`Show::Boss`]). The events its affects cause come back.
    pub(super) fn boss_frame(
        &mut self,
        env: &Env,
        world: World,
        check: &dyn Fn(usize) -> i32,
        hits: &mut crate::hit::Hits,
        camera: &mut Camera,
        pad: &CamPad,
    ) -> Events {
        let mut ev = Events::new();
        let Some(run) = self.boss.as_mut() else { return ev };
        if run.exit {
            return ev;
        }
        // ccThBossEffect's pass, before the boss's task: the cinema's Draw.
        run.cinema_sent = run.cinema.step();
        let me = run.me;
        let hit = run.hit;
        let clips = run.clips.clone();
        let d = self.data.clone();
        let Some(data) = d.skeith.as_ref() else { return ev };
        let clip = |name: &str| clips.get(name).copied();
        let party = self.party;
        let benv = BossEnv { t: &d.t, data, clips: &clip, env, game_over: false };
        let actx =
            AffectCtx { party: &party, menu: true, skill_check: check, boss: Some(&benv), volume: self.data.volume };
        boss::apply_queued(&mut self.scene, &actx, me, &mut self.rand, &mut ev);
        let Some(mut b) = self.scene.chars[me].foe_state_mut().and_then(|f| f.boss.take()) else { return ev };
        let mut out = b.take_pending();
        // ccBoss::Move: SetHitSW(bodyHitSW), then the body at the new place
        // pushed out of the others (CollisionDetection).
        let kite = self.kite;
        let mut body = super::spc::to_body(me, kite, &hit, false);
        hits.set_hit_sw(&mut body, b.body_hit_sw != 0);
        let mut collide = |pos: V4| {
            let mut body = super::spc::to_body(me, kite, &CharHit { pos, sw: true, ..hit }, false);
            hits.sync(&body);
            (hits.collision_detection(&mut body) != 0).then_some(body.offset)
        };
        let mut cx = boss::Cx {
            t: &d.t,
            data,
            scene: &mut self.scene,
            party: &party,
            world,
            env,
            actx: &actx,
            rand: &mut self.rand,
            cc: &mut self.cc,
            clips: &clip,
            collide: &mut collide,
            game_over: false,
            boss_cam: true,
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        b.main(&mut cx);
        out.extend(std::mem::take(&mut cx.out));
        ev.extend(std::mem::take(&mut cx.ev));
        drop(cx);
        let exited = b.exit != 0;
        // The body's copy in the list where the boss stands now.
        let at = CharHit { pos: self.scene.chars[me].pos, sw: b.body_hit_sw != 0 && !exited, ..hit };
        let mut body = super::spc::to_body(me, kite, &at, false);
        hits.set_hit_sw(&mut body, at.sw);
        hits.sync(&body);
        // ccBoss01::DrawCross (0x0047c390), after ccBoss::Main: in the
        // cross, the sword's two dummies (DMY_xdummy_w01, _w02 of x11) under
        // OBJ_ex11swd as it was last drawn, a new row of the trail; then
        // the trail's Disp.
        let cross = (b.act_num == piney_battle::boss::act::CROSS && !exited)
            .then(|| self.cast.actors.get(&me).and_then(|a| sword_points(&a.ch.body, &a.ch.play, a.ch.pos, a.ch.dirc)))
            .flatten();
        if let Some(r) = self.boss.as_mut() {
            if let Some([p1, p2]) = cross {
                r.lattice.next_vertex();
                r.lattice.set_pos(p1, 0);
                r.lattice.set_pos(p2, 1);
            }
            r.trail = r.lattice.disp();
        }
        let dirc = b.dirc;
        let wave_time = b.anm_wave.posed;
        let clip = b.anm.clip.clone().unwrap_or_default();
        let time = b.anm.posed;
        let kite_pos = kite.map(|k| self.scene.chars[k].pos);
        if let Some(r) = self.boss.as_mut() {
            // Think and Action's QuakeCam and BeginDeadEffect, then after
            // Move the camera (bossCam && bossCamSW: CamMain).
            for o in &out {
                match o {
                    // OnCinemaMode's other checks (the party wiped out, a
                    // forced game over) end the fight first here.
                    Out::Cinema(Some(n)) => {
                        let name = (*n == SKEITH_CINEMA_NAME).then(|| r.look.skill_name.clone()).flatten();
                        r.cinema.on(name);
                    }
                    Out::Cinema(None) => r.cinema.off(),
                    Out::Quake(v) => {
                        if let Some(c) = r.cam.as_mut() {
                            c.quake(*v);
                        }
                    }
                    Out::DeadCamera { eye, view, .. } => {
                        if r.cam_sw {
                            if r.cam.is_some() {
                                camera.boss_cam_off();
                            } else {
                                camera.change_camera(id::FIELD);
                            }
                        }
                        r.cam_sw = false;
                        camera.change_camera(id::EVENT);
                        let e = camera.cam_mut(id::EVENT);
                        e.pos = *eye;
                        e.view = *view;
                    }
                    _ => {}
                }
            }
            if r.cam_sw
                && let (Some(c), Some(k)) = (r.cam.as_mut(), kite_pos)
            {
                c.cam_main(camera, pad, k, at.pos);
            }
            r.hit = at;
            // EntryAfterImage in the frame's Think, then DrawAfterImages:
            // each image steps its fade as it draws, the new one too.
            r.wave = None;
            r.reverse = out.contains(&Out::Reverse);
            for o in &out {
                match o {
                    Out::AfterImage => r.afterimages.push(AfterImage {
                        clip: clip.clone(),
                        time,
                        pos: at.pos,
                        dirc,
                        transparency: 1.0,
                    }),
                    Out::DrawWave => r.wave = Some((wave_time, at.pos, dirc)),
                    _ => {}
                }
            }
            for a in &mut r.afterimages {
                a.transparency -= AFTERIMAGE_FADE;
            }
            r.afterimages.retain(|a| a.transparency > 0.0);
        }
        // The actor: ccAnm::Draw of the boss's clip at its frame, placed by
        // SetMatrix_PosRotZYX(pos, dirc), at setTransparency, while drawn.
        let pos = self.scene.chars[me].pos;
        if let Some(a) = self.cast.actors.get_mut(&me) {
            if let Some(c) = &b.anm.clip
                && a.ch.play.anim != a.ch.body.anim(c).unwrap_or(usize::MAX)
            {
                a.set(super::AnmSlot::Main, c);
            }
            a.ch.play.time = b.anm.time;
            a.ch.play.posed = b.anm.posed;
            a.ch.pos = pos;
            a.ch.dirc = b.dirc;
            a.drawn = b.draw_sw != 0 && !exited;
            a.alpha = b.set_transparency;
            a.trans_dist = false;
        }
        if let Some(f) = self.scene.chars[me].foe_state_mut() {
            f.boss = Some(b);
        }
        for o in out {
            match &o {
                Out::DeleteCmnd => self.scene.ene_list.retain(|&c| c != me),
                // ccItemSkillRequest: the skill run from the boss on its
                // target, as an item's through a character.
                Out::Skill(tp, k) => self.item_skill(me, *tp, k),
                // ccHitMarkDisp and ccEntryFlyFontNew over the boss: the
                // effects' as for any character's.
                Out::HitMark { by } => {
                    let by = by.map_or(Who::Nobody, Who::Char);
                    self.shows.push(Show::Rule(Event::HitMark { on: Who::Char(me), by }));
                }
                Out::FlyFont { kind, n } => {
                    self.shows.push(Show::Rule(Event::FlyFont { on: Who::Char(me), kind: *kind, value: *n }));
                }
                _ => {}
            }
            self.shows.push(Show::Boss(me, o));
        }
        if exited && let Some(r) = self.boss.as_mut() {
            r.exit = true;
            self.cast.actors.remove(&me);
        }
        ev
    }

    /// The boss's scene index while it is in the field.
    pub fn boss_char(&self) -> Option<usize> {
        self.boss.as_ref().filter(|r| !r.exit).map(|r| r.me)
    }

    /// An event's `present 7` / `absent 7`: the boss task exists and has
    /// not exited.
    pub fn boss_present(&self) -> bool {
        self.boss.as_ref().is_some_and(|r| !r.exit)
    }

    /// The boss's HP for the HUD: (HP, max HP), None without a boss.
    pub fn boss_hp(&self) -> Option<(i16, i16)> {
        let me = self.boss_char()?;
        let c = &self.scene.chars[me];
        (c.cond[cond::DEAD] == 0).then_some((c.hp, c.max_hp))
    }
}

/// The sword's two trail points (`DMY_xdummy_w01`, `_w02` of the boss's
/// file) under `OBJ_ex11swd`'s world matrix (`GetObjAdrsF` of the clump),
/// the body posed by `play` at `pos`, `dirc` (`sceVu0ApplyMatrix`).
fn sword_points(body: &crate::body::Body, play: &crate::pose::Play, pos: V4, dirc: V4) -> Option<[V4; 2]> {
    let sword = body.node("OBJ_ex11swd")?;
    let m = *body.worlds(play, crate::body::root(pos, dirc)).get(&sword)?;
    let point = |name: &str| -> Option<V4> {
        let d = body.file.scene.dummies.get(&body.file.ccs.find_object(name)?)?;
        let p = m * glam::Vec4::new(d.pos.x, d.pos.y, d.pos.z, 1.0);
        Some([p.x.to_bits(), p.y.to_bits(), p.z.to_bits(), p.w.to_bits()])
    };
    Some([point("DMY_xdummy_w01")?, point("DMY_xdummy_w02")?])
}

/// The boss's position with its z raised by `dz`, for a sound or a look.
pub fn above(pos: V4, dz: f32) -> V4 {
    [pos[0], pos[1], ee::add(pos[2], ee::k(dz)), ONE]
}
