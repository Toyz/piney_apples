//! The bosses in the field: what `ccBossEntryStart(code)` (gcmn 0x0045b2a0)
//! starts for an event's `entry 7 code` - `ccThBossEffect` and
//! `bossFunc[code]` at priority 66 - over the battle's scene
//! ([`piney_battle::boss`]). Codes 0 Skeith, 1 Innis, 2 Magus (its leaves
//! characters of their own), 3 Fidchell, 12 Kyvia's first fight (its core
//! and gomoras likewise); the others start nothing. Once `CheckExit()` the
//! task sets its parameter's +0x14, which `absent 7` reads.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use piney_battle::affect::AffectCtx;
use piney_battle::boss::{self, Boss, BossEnv, CamView, Class, Out};
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

/// `bossFunc`'s codes the port has: Skeith (`bossTbl` row 0), Innis
/// (row 1), Magus (row 2), Fidchell (row 3) and Kyvia 01 (row 12).
pub const SKEITH: i32 = 0;
pub const INNIS: i32 = 1;
pub const MAGUS: i32 = 2;
pub const FIDCHELL: i32 = 3;
pub const KYVIA: i32 = 12;

/// Skeith's file and model (`x11`, `CMP_trall`), and the effects' file.
pub const FILE: &str = "x11";
pub const CLUMP: &str = "CMP_trall";
pub const EFF_FILE: &str = "xeffect";
/// Innis's file (`x21`): its body, its three images, its mirrors' shards.
pub const INNIS_FILE: &str = "x21";
/// Innis's images (`Mon1`-`Mon3`' models).
pub const IMAGES: [&str; 3] = ["CMP_ex21mon1", "CMP_ex21mon2", "CMP_ex21mon3"];
/// Kyvia's file (`x01`), its body, and its core's and gomoras' models.
pub const KYVIA_FILE: &str = "x01";
pub const KYVIA_CLUMP: &str = "CMP_trallex01";
pub const KYVIA_PARTS: [&str; 2] = ["CMP_trall2", "CMP_ex01gom2"];
/// Magus's file (`x31`): its body (`CMP_trall`) and its leaves' model.
pub const MAGUS_FILE: &str = "x31";
pub const MAGUS_LEAF: &str = "CMP_ex31leaf";
/// Fidchell's file (`x41`): its body, and the body its Epitaph's clips
/// (`ANM_ex4x*`) move.
pub const FIDCHELL_CLUMP: &str = "CMP_trall1";
pub const FIDCHELL_EPITAPH: &str = "CMP_trallx";
/// The wave's animation in [`EFF_FILE`].
pub const ANM_WAVE: &str = "ANM_xx11wave";

/// `InitBossCamera(z, y)`'s `transfer` by boss: Skeith's (200, 1000), the
/// eye 1000 behind Kite and 200 up at its nearest; Innis's (200, 900).
pub const CAM_TRANSFER: V4 = [0, 0x447a_0000, 0x4348_0000, ONE];
pub const INNIS_CAM_TRANSFER: V4 = [0, 0x4461_0000, 0x4348_0000, ONE];
/// Kyvia's `InitBossCamera(450, 1050)`.
pub const KYVIA_CAM_TRANSFER: V4 = [0, 0x4483_4000, 0x43e1_0000, ONE];
/// Fidchell's `InitBossCamera(250, 1500)`.
pub const FIDCHELL_CAM_TRANSFER: V4 = [0, 0x44bb_8000, 0x437a_0000, ONE];

/// A clip's frames and whether it loops, by name, in the boss's files.
pub type Clips = HashMap<String, (u32, bool)>;

/// What the field keeps of the boss's tasks.
pub struct BossRun {
    /// `bossFunc`'s code (`eventMng.bossEntry`).
    pub code: i32,
    /// The boss's scene index.
    pub me: usize,
    /// The task's parameter +0x14: the boss has exited.
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
    /// `ccBufferReverce::MakePacket` this frame (Skeith's magic's last
    /// part): the frame inverted on the boss's own layer.
    pub reverse: bool,
    /// Skeith's cross's trail (`ccLattice(2, 7)`, +0x2935c) and what its
    /// `Disp` sent this frame, with its sort key.
    pub lattice: Lattice,
    pub trail: (Vec<StripVertex>, u32),
    /// `ccBossEffManager`'s cinema (+0x00) and what its `Draw` sent this
    /// frame.
    pub cinema: Cinema,
    pub cinema_sent: CinemaSent,
    /// Innis's images' scene indices, drawn while theirs `drawSW` holds.
    pub images: Vec<usize>,
    /// Kyvia's core and gomoras (their characters), drawn while theirs
    /// `drawSW` holds.
    pub parts: Vec<usize>,
    /// `WORLD_MAN.eventmap` as an `EVENTAREAB8` this frame (Kyvia's disc),
    /// and whether the boss asked it on (`EVENTAREAB8::Move`).
    pub disc: boss::DiscView,
    pub disc_next: bool,
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
/// its files, the effects' file (the wave), the cinema's skill names and
/// Innis's images.
pub struct BossLook {
    pub code: i32,
    pub body: Rc<Body>,
    pub clips: Rc<Clips>,
    pub eff: Rc<piney_desktop::assets::SceneFile>,
    pub eff_morphers: HashMap<u32, u32>,
    /// `_g_cinemaSkillName[n]` of this boss's file (`OnCinemaMode(n)`'s
    /// name), by `n`: on Infection Skeith's 2 (x11's `TEX_ske_skl`, row
    /// 0); on Mutation Innis's 5-8 and 70 (`TEX_ini_skl`, rows 0-4).
    pub names: HashMap<i32, CinemaName>,
    /// Innis's images (`CMP_ex21mon1`-`3` of x21), by `MonsterID`;
    /// Kyvia's core and gomora ([`KYVIA_PARTS`] of x01).
    pub images: Vec<Rc<Body>>,
}

impl BossLook {
    /// Boss `code`'s look (Skeith's for any code the port lacks).
    pub fn load(
        archive: &std::sync::Arc<piney_data::archive::Archive>,
        code: i32,
        volume: piney_data::volume::Volume,
    ) -> piney_data::Result<BossLook> {
        let (file, clump) = match code {
            INNIS => (INNIS_FILE, CLUMP),
            MAGUS => (MAGUS_FILE, CLUMP),
            FIDCHELL => (boss::fidchell::FILE, FIDCHELL_CLUMP),
            KYVIA => (KYVIA_FILE, KYVIA_CLUMP),
            _ => (FILE, CLUMP),
        };
        let body = Rc::new(Body::read(archive, file, clump)?);
        let mut clips = Clips::new();
        for stem in [file, EFF_FILE] {
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
        let mut names = HashMap::new();
        let mut textures: HashMap<&str, Option<piney_desktop::message::WindowTexture>> = HashMap::new();
        for (n, row) in piney_data::tables::combat::of(volume).cinema_skill_names().iter().enumerate() {
            let (Some(f), Some(tex)) = (row.file, row.tex) else { continue };
            if f != file {
                continue;
            }
            let w = textures
                .entry(tex)
                .or_insert_with(|| piney_desktop::message::WindowTexture::read_named(archive, file, tex, None));
            if let Some(w) = w {
                names.insert(n as i32, CinemaName { tex: w.tex.clone(), tex_h: w.tex_h, row: row.row });
            }
        }
        let images = match code {
            INNIS => {
                let file = Rc::new(piney_desktop::assets::SceneFile::read(archive, INNIS_FILE)?);
                IMAGES.iter().filter_map(|c| Body::of(file.clone(), c).ok().map(Rc::new)).collect()
            }
            KYVIA => {
                let file = Rc::new(piney_desktop::assets::SceneFile::read(archive, KYVIA_FILE)?);
                KYVIA_PARTS.iter().filter_map(|c| Body::of(file.clone(), c).ok().map(Rc::new)).collect()
            }
            MAGUS => {
                let file = Rc::new(piney_desktop::assets::SceneFile::read(archive, MAGUS_FILE)?);
                Body::of(file, MAGUS_LEAF).ok().map(Rc::new).into_iter().collect()
            }
            FIDCHELL => {
                let file = Rc::new(piney_desktop::assets::SceneFile::read(archive, boss::fidchell::FILE)?);
                Body::of(file, FIDCHELL_EPITAPH).ok().map(Rc::new).into_iter().collect()
            }
            _ => Vec::new(),
        };
        Ok(BossLook { code, body, clips: Rc::new(clips), eff, eff_morphers, names, images })
    }
}

impl Combat {
    /// `ccBossEntryStart(code)`: the boss made (`ccBoss01::ccBoss01`,
    /// `ccBoss02::ccBoss02`, `ccBossKyvia01::ccBossKyvia01(1)`) at the
    /// arena's centre `center` (`DMY_center01`) or Kyvia's disc (`disc`)
    /// and its actor. Nothing for a code the port lacks, without the
    /// tables or without Kite.
    pub fn start_boss(
        &mut self,
        look: &Rc<BossLook>,
        center: V4,
        disc: boss::DiscView,
        env: &Env,
        camera: &mut Camera,
    ) {
        let code = look.code;
        if self.boss.is_some() || !matches!(code, SKEITH | INNIS | MAGUS | FIDCHELL | KYVIA) {
            return;
        }
        let d = self.data.clone();
        let (Some(data), Some(kite)) = (d.bosses.as_ref(), self.kite) else { return };
        let Some(row) = d.t.bosses.get(code as usize).cloned() else { return };
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
        let mut land = |p: V4| p[2];
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
            cam: cam_view(camera, None, false),
            land: &mut land,
            disc,
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        let b = match code {
            INNIS => boss::innis::new(&mut cx, kite_pos, kite_dirc, center),
            MAGUS => boss::magus::new(&mut cx, kite_pos, kite_dirc, center),
            FIDCHELL => boss::fidchell::new(&mut cx, kite_pos, kite_dirc, center),
            KYVIA => boss::kyvia::new(&mut cx),
            _ => Boss::new(&mut cx, kite_pos, kite_dirc, center),
        };
        let out = std::mem::take(&mut cx.out);
        drop(cx);
        let ctor_out = out.clone();
        for o in out {
            self.shows.push(Show::Boss(me, o));
        }
        let pos = self.scene.chars[me].pos;
        let (h, w) = (self.scene.chars[me].base().height, self.scene.chars[me].base().width);
        let clip0 = b.anm.clip.clone().unwrap_or_default();
        if let Some(a) = Actor::new(look.body.clone(), &clip0, Look::Boss, pos, b.dirc, h, w) {
            self.cast.actors.insert(me, a);
        }
        let images = match &b.class {
            Class::Innis(x) => x.slaves.iter().map(|s| s.me).collect(),
            _ => Vec::new(),
        };
        let parts = match &b.class {
            Class::Kyvia(x) => {
                let mut v = vec![x.core];
                if let Some(Class::KyviaCore(c)) =
                    self.scene.chars[x.core].foe_state().and_then(|f| f.boss.as_ref()).map(|b| &b.class)
                {
                    v.extend(c.gomoras.iter().copied());
                }
                v
            }
            Class::Magus(x) => x.leaves.clone(),
            _ => Vec::new(),
        };
        if let Some(f) = self.scene.chars[me].foe_state_mut() {
            f.boss = Some(Box::new(b));
        }
        let ty = self.scene.chars[me].ty() as u32;
        let hit = CharHit { pos, radius: w, height: h, kind: ty, ..CharHit::default() };
        let transfer = match code {
            INNIS => INNIS_CAM_TRANSFER,
            KYVIA => KYVIA_CAM_TRANSFER,
            FIDCHELL => FIDCHELL_CAM_TRANSFER,
            _ => CAM_TRANSFER,
        };
        // InitBossCamera(z, y), in the constructor, and what the
        // constructor set on it then. Magus makes none: it turns camera 3
        // itself (`CalcCamera`).
        let cam = (code != MAGUS).then(|| {
            let mut cam = BossCam::new(camera, transfer, kite_pos);
            for o in &ctor_out {
                cam_out(&mut cam, o, camera);
            }
            cam
        });
        self.boss = Some(BossRun {
            code,
            me,
            exit: false,
            clips: look.clips.clone(),
            hit,
            look: look.clone(),
            afterimages: Vec::new(),
            wave: None,
            cam_sw: cam.is_some(),
            cam,
            reverse: false,
            lattice: Lattice::new(2, 7),
            trail: (Vec::new(), 0),
            cinema: Cinema::new(),
            cinema_sent: CinemaSent::default(),
            images,
            parts,
            disc,
            disc_next: false,
        });
    }

    /// `ccThBossEffect` and the boss task's frame: the queued affects,
    /// the class's `Main`, the boss's actor set from it; what it asks goes
    /// to the shows ([`Show::Boss`]). The events its affects cause come
    /// back.
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
        // checkCameraShakeRange of the boss's place, as its Think asks it.
        let shake = shake_range(camera, &world, self.scene.chars[me].pos);
        let cv = cam_view(camera, run.cam.as_ref(), shake);
        let d = self.data.clone();
        let Some(data) = d.bosses.as_ref() else { return ev };
        let clip = |name: &str| clips.get(name).copied();
        let party = self.party;
        let benv = BossEnv { t: &d.t, data, clips: &clip, env, game_over: false };
        let actx =
            AffectCtx { party: &party, menu: true, skill_check: check, boss: Some(&benv), volume: self.data.volume };
        boss::apply_queued(&mut self.scene, &actx, me, &mut self.rand, &mut ev);
        // Kyvia's core and gomoras: their affects of the frame so far, and
        // what they asked (their hits' marks and numbers).
        let parts = run.parts.clone();
        let disc = run.disc;
        let mut part_out = Vec::new();
        for &p in &parts {
            boss::apply_queued(&mut self.scene, &actx, p, &mut self.rand, &mut ev);
            if let Some(pb) = self.scene.chars[p].foe_state_mut().and_then(|f| f.boss.as_mut()) {
                part_out.extend(pb.take_pending().into_iter().map(|o| (p, o)));
            }
        }
        let Some(mut b) = self.scene.chars[me].foe_state_mut().and_then(|f| f.boss.take()) else { return ev };
        let mut out = b.take_pending();
        // Magus's leaves hang where its body was last posed: `Init` reads
        // `OBJ_ex31leafNN`'s world matrix.
        if let (Class::Magus(x), Some(a)) = (&mut b.class, self.cast.actors.get(&me)) {
            x.leaf_pos = leaf_points(&a.ch.body, &a.ch.play, a.ch.pos, a.ch.dirc, x.leaf_pos);
        }
        // ccBoss::Move: SetHitSW(bodyHitSW), then the body at the new place
        // pushed out of the others (CollisionDetection).
        let kite = self.kite;
        let mut body = super::spc::to_body(me, kite, &hit, false);
        hits.set_hit_sw(&mut body, b.body_hit_sw != 0);
        let hits = RefCell::new(hits);
        let mut collide = |pos: V4| {
            let mut hits = hits.borrow_mut();
            let mut body = super::spc::to_body(me, kite, &CharHit { pos, sw: true, ..hit }, false);
            hits.sync(&body);
            (hits.collision_detection(&mut body) != 0).then_some(body.offset)
        };
        // BreakMirror's ccLandHitCheck(pos, 0x20000000).
        let mut land = |pos: V4| hits.borrow_mut().land(pos, 0x2000_0000);
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
            cam: cv,
            land: &mut land,
            disc,
            me,
            out: Vec::new(),
            ev: Events::new(),
        };
        b.main(&mut cx);
        out.extend(std::mem::take(&mut cx.out));
        ev.extend(std::mem::take(&mut cx.ev));
        drop(cx);
        let hits = hits.into_inner();
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
        let skeith = b.class == Class::Skeith;
        let cross = (skeith && b.act_num == piney_battle::boss::act::CROSS && !exited)
            .then(|| self.cast.actors.get(&me).and_then(|a| sword_points(&a.ch.body, &a.ch.play, a.ch.pos, a.ch.dirc)))
            .flatten();
        if let Some(r) = self.boss.as_mut()
            && skeith
        {
            if let Some([p1, p2]) = cross {
                r.lattice.next_vertex();
                r.lattice.set_pos(p1, 0);
                r.lattice.set_pos(p2, 1);
            }
            r.trail = r.lattice.disp();
        }
        let dirc = b.dirc;
        let wave_time = match &b.class {
            Class::Innis(x) => x.anm_w.posed,
            Class::Magus(x) => x.anm_w.posed,
            Class::Fidchell(x) => x.anm_w.posed,
            _ => b.anm_wave.posed,
        };
        let clip = b.anm.clip.clone().unwrap_or_default();
        let time = b.anm.posed;
        let kite_pos = kite.map(|k| self.scene.chars[k].pos);
        if let Some(r) = self.boss.as_mut() {
            // Think and Action's QuakeCam, SetMode, SetFreeCamPosView and
            // BeginDeadEffect, then after Move the camera (bossCam &&
            // bossCamSW: CamMain).
            for o in &out {
                match o {
                    // OnCinemaMode's other checks (the party wiped out, a
                    // forced game over) end the fight first here.
                    Out::Cinema(Some(n)) => {
                        let name = r.look.names.get(n).cloned();
                        r.cinema.on(name);
                    }
                    Out::Cinema(None) => r.cinema.off(),
                    // A skill's name (OUT gcmn 0x00472550's rows) is not
                    // read: the bars alone.
                    Out::CinemaSkill(_) => r.cinema.on(None),
                    Out::Quake(v) => {
                        if let Some(c) = r.cam.as_mut() {
                            c.quake(*v);
                        }
                    }
                    Out::CamMode { mode } => {
                        if let Some(c) = r.cam.as_mut() {
                            c.set_mode(*mode, camera);
                        }
                    }
                    Out::FreeCam { pos, view } => {
                        if let Some(c) = r.cam.as_mut() {
                            c.free_cam_pos_view(*pos, *view);
                        }
                    }
                    // Mutation's +0xe0, the pitch CamMain turns the view by.
                    Out::CamPitch { add, v } => {
                        if let Some(c) = r.cam.as_mut() {
                            c.xrot = if *add { ee::add(c.xrot, *v) } else { *v };
                        }
                    }
                    Out::CamModeRange { .. } | Out::CamInitLock | Out::CamRotXLimit(_) | Out::CamMaxRange(_) => {
                        if let Some(c) = r.cam.as_mut() {
                            cam_out(c, o, camera);
                        }
                    }
                    Out::DiscNextStage => r.disc_next = true,
                    // Magus's cameras: changeCamera, cameraSetPos/View.
                    Out::CameraChange(n) => camera.change_camera(*n as i16),
                    Out::CameraPos { cam, pos } => camera.cam_mut(*cam as i16).pos = *pos,
                    Out::CameraView { cam, view } => camera.cam_mut(*cam as i16).view = *view,
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
        // Fidchell's Epitaph clips move its other body.
        let pos = self.scene.chars[me].pos;
        self.fidchell_body(&b, me);
        if let Some(a) = self.cast.actors.get_mut(&me) {
            pose(a, &b.anm, pos, b.dirc);
            a.drawn = b.draw_sw != 0 && !exited;
            a.alpha = b.set_transparency;
            a.trans_dist = false;
        }
        self.image_actors(&b, exited);
        if let Some(f) = self.scene.chars[me].foe_state_mut() {
            f.boss = Some(b);
        }
        self.part_actors(&parts, exited);
        for (p, o) in part_out {
            match &o {
                Out::HitMark { by } => {
                    let by = by.map_or(Who::Nobody, Who::Char);
                    self.shows.push(Show::Rule(Event::HitMark { on: Who::Char(p), by }));
                }
                Out::FlyFont { kind, n } => {
                    self.shows.push(Show::Rule(Event::FlyFont { on: Who::Char(p), kind: *kind, value: *n }));
                }
                _ => {}
            }
            self.shows.push(Show::Boss(p, o));
        }
        for o in out {
            match &o {
                Out::DeleteCmnd => self.scene.ene_list.retain(|&c| c != me),
                // checkCameraShakeRange(pos), then cameraShake: its `rand()`.
                Out::CameraShake { pos, s } => {
                    if shake_range(camera, &world, *pos) {
                        let rand = &mut self.rand;
                        camera.shake.shake(s[0], s[1], s[2], s[3], &mut || piney_battle::Rng::rand(rand));
                    }
                }
                // ccItemSkillRequest: the skill run from the boss on its
                // target, as an item's through a character.
                Out::Skill(tp, k) => self.item_skill(me, *tp, k),
                // A gomora's skill (ccItemSkillRequest, ccSkillRequestParam).
                Out::SkillFrom { user, target, skill } => self.item_skill(*user, *target, skill),
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
            for k in std::mem::take(&mut r.images).into_iter().chain(std::mem::take(&mut r.parts)) {
                self.cast.actors.remove(&k);
            }
        }
        ev
    }

    /// Fidchell's body for its clip: `CMP_trallx` under the Epitaph's
    /// `ANM_ex4x*`, `CMP_trall1` under the rest.
    fn fidchell_body(&mut self, b: &Boss, me: usize) {
        let Some(look) = self.boss.as_ref().map(|r| r.look.clone()).filter(|l| l.code == FIDCHELL) else { return };
        let clip = b.anm.clip.clone().unwrap_or_default();
        let body = match look.images.first() {
            Some(x) if clip.starts_with("ANM_ex4x") => x.clone(),
            _ => look.body.clone(),
        };
        let pos = self.scene.chars[me].pos;
        let (h, w) = (self.scene.chars[me].base().height, self.scene.chars[me].base().width);
        if let Some(a) = self.cast.actors.get_mut(&me)
            && !Rc::ptr_eq(&a.ch.body, &body)
            && let Some(n) = Actor::new(body, &clip, Look::Boss, pos, b.dirc, h, w)
        {
            *a = n;
        }
    }

    /// Innis's images (`ccBoss02Slave::Draw` through `ccBoss::Draw`): each
    /// a `CMP_ex21monN` of x21 at the image's clip, place and turn while
    /// its `drawSW` holds.
    fn image_actors(&mut self, b: &Boss, exited: bool) {
        let Class::Innis(x) = &b.class else { return };
        let Some(look) = self.boss.as_ref().map(|r| r.look.clone()) else { return };
        for s in &x.slaves {
            let shown = s.b.draw_sw != 0 && s.b.exit == 0 && !exited;
            let Some(body) = look.images.get(s.samon_id.clamp(0, 2) as usize).cloned() else { continue };
            let pos = self.scene.chars[s.me].pos;
            let entry = self.cast.actors.entry(s.me);
            let a = match entry {
                std::collections::btree_map::Entry::Occupied(o) => o.into_mut(),
                std::collections::btree_map::Entry::Vacant(v) => {
                    if !shown {
                        continue;
                    }
                    let clip = s.b.anm.clip.clone().unwrap_or_default();
                    let Some(a) = Actor::new(body.clone(), &clip, Look::Boss, pos, s.b.dirc, 0, 0) else { continue };
                    v.insert(a)
                }
            };
            if !Rc::ptr_eq(&a.ch.body, &body) {
                let clip = s.b.anm.clip.clone().unwrap_or_default();
                if let Some(n) = Actor::new(body.clone(), &clip, Look::Boss, pos, s.b.dirc, 0, 0) {
                    *a = n;
                }
            }
            pose(a, &s.b.anm, pos, s.b.dirc);
            a.drawn = shown;
            a.alpha = s.transparency;
            a.trans_dist = false;
        }
    }

    /// Kyvia's core and gomoras (`ccBoss::Draw` of each): the core's
    /// model and the gomoras' at their clips, places and turns, at their
    /// `Alpha`, while their `drawSW` holds.
    fn part_actors(&mut self, parts: &[usize], exited: bool) {
        let Some(look) = self.boss.as_ref().map(|r| r.look.clone()) else { return };
        for (k, &p) in parts.iter().enumerate() {
            let Some(pb) = self.scene.chars[p].foe_state().and_then(|f| f.boss.as_ref()) else { continue };
            let shown = pb.draw_sw != 0 && pb.exit == 0 && !exited;
            // Kyvia's core then its gomoras; Magus's leaves, one model.
            let pick = usize::from(k > 0 && look.code != MAGUS);
            let Some(body) = look.images.get(pick).cloned() else { continue };
            let (anm, dirc, alpha) = (pb.anm.clone(), pb.dirc, pb.set_transparency);
            let pos = self.scene.chars[p].pos;
            let a = match self.cast.actors.entry(p) {
                std::collections::btree_map::Entry::Occupied(o) => o.into_mut(),
                std::collections::btree_map::Entry::Vacant(v) => {
                    if !shown {
                        continue;
                    }
                    let clip = anm.clip.clone().unwrap_or_default();
                    let Some(a) = Actor::new(body, &clip, Look::Boss, pos, dirc, 0, 0) else { continue };
                    v.insert(a)
                }
            };
            pose(a, &anm, pos, dirc);
            a.drawn = shown;
            a.alpha = alpha;
            a.trans_dist = false;
        }
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

/// An actor posed as a `ccAnm` left it: its clip (set anew when it
/// changed), the time and the pose `PreDrawAnm`'s step made, the place.
fn pose(a: &mut Actor, anm: &piney_battle::boss::Anm, pos: V4, dirc: V4) {
    if let Some(c) = &anm.clip
        && a.ch.play.anim != a.ch.body.anim(c).unwrap_or(usize::MAX)
    {
        a.set(super::AnmSlot::Main, c);
    }
    a.ch.play.time = anm.time;
    a.ch.play.posed = anm.posed;
    a.ch.pos = pos;
    a.ch.dirc = dirc;
}

/// What a boss's rules read of the cameras this frame: the active
/// camera's turn (`cameraGetRot(camID)`), camera 2's eye and view, and
/// the boss camera's `CheckMoveCamera`.
fn cam_view(camera: &Camera, bcam: Option<&BossCam>, shake: bool) -> CamView {
    let c = camera.cam(id::BATTLE);
    CamView {
        rot: camera.rot(),
        pos: c.pos,
        view: c.view,
        moving: bcam.is_some_and(BossCam::check_move_camera),
        reset: bcam.map_or(0, |b| b.reset),
        shake,
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

/// `checkCameraShakeRange(pos)` (main 0x00162f10): `pos` in the active
/// camera's view (`ccCheckCameraDeg(pos, 12288)`) and nearer its eye than
/// 2000, all through `W2PPos`.
fn shake_range(camera: &Camera, world: &World, pos: V4) -> bool {
    let cam = camera.active();
    let (p, c, v) = (world.frame.w2p(pos), world.frame.w2p(cam.pos), world.frame.w2p(cam.view));
    let a = ee::rad2deg(ee::atan2f(ee::sub(p[1], c[1]), ee::sub(p[0], c[0])));
    let b = ee::rad2deg(ee::atan2f(ee::sub(v[1], c[1]), ee::sub(v[0], c[0])));
    let d = (12288 + (i32::from(a) - i32::from(b))) as i16;
    if !(d > 0 && i32::from(d) < 2 * 12288) {
        return false;
    }
    let dv = ee::vsub(pos, cam.pos);
    ee::lt(ee::sqrtf(ee::dot(dv, dv)), 0x44fa_0000)
}

/// Where each of Magus's twelve leaves hangs (`OBJ_ex31leaf`,
/// `OBJ_ex31leaf01`-`11` of x31) on the body posed by `play` at `pos`,
/// `dirc`; a leaf the model lacks keeps `last`'s.
fn leaf_points(body: &crate::body::Body, play: &crate::pose::Play, pos: V4, dirc: V4, last: [V4; 12]) -> [V4; 12] {
    let worlds = body.worlds(play, crate::body::root(pos, dirc));
    std::array::from_fn(|k| {
        let name = if k == 0 { "OBJ_ex31leaf".to_string() } else { format!("OBJ_ex31leaf{k:02}") };
        let Some(m) = body.node(&name).and_then(|n| worlds.get(&n)) else { return last[k] };
        let p = *m * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
        [p.x.to_bits(), p.y.to_bits(), p.z.to_bits(), p.w.to_bits()]
    })
}

/// What a boss asks of its camera beside the frame's moves: a mode with a
/// range (Kyvia's 3 and 4), `InitLock`, `SetRotXLimit` and the pitch.
fn cam_out(c: &mut BossCam, o: &Out, camera: &mut Camera) {
    match o {
        Out::CamModeRange { mode, range } => c.set_mode_range(*mode, *range, camera),
        Out::CamInitLock => c.init_lock = true,
        Out::CamRotXLimit(v) => c.set_rot_x_limit(*v),
        Out::CamPitch { add, v } => c.xrot = if *add { ee::add(c.xrot, *v) } else { *v },
        Out::CamMaxRange(v) => c.max_range = *v,
        _ => {}
    }
}

/// The boss's position with its z raised by `dz`, for a sound or a look.
pub fn above(pos: V4, dz: f32) -> V4 {
    [pos[0], pos[1], ee::add(pos[2], ee::k(dz)), ONE]
}
