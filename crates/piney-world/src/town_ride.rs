//! The riding Grunty in a town, from Mutation on: the Flag Race's ride.
//! `ccPuccigusoStart(kind)` (MUT gcmn 0x0052d6f0) in a town changes no
//! music and fades nothing; it sleeps Kite alone (0x005cbb40), his body
//! out of the collision. `ccThPucciguso` (0x0052d540) runs `Main`, paused
//! at first, until the race's end flag (+0xa7); `ccPuccigusoExit`
//! (0x0052d960) puts Kite back and wakes him (act 2). The ride itself is
//! [`piney_battle::ride`] with the race's handling (`Input::race`).

use std::rc::Rc;

use glam::Mat4;
use piney_battle::ride::{self, Globals, Input, Out, RaceRide, Ride, RideAnm, RideTables, RideWorld};
use piney_battle::world::{CharHit, Note};
use piney_desktop::layers::Layers;

use crate::body::{Body, TRALL};
use crate::camera::{CamPad, Camera};
use crate::char::{Char, SHADED};
use crate::combat::ride::BODY_WHO;
use crate::combat::spc;
use crate::ee::{F, ONE, V4};
use crate::hit::Hits;
use crate::race::RaceEvent;
use crate::town::TownLights;

/// `ccThPucciguso`'s first loop: 30 frames of `Main` before the end is
/// looked for, then one more.
const FIRST_MAINS: u32 = 31;

/// The ride's task and object in a town.
pub struct TownRide {
    /// The task's parameter (+0x14).
    pub kind: i32,
    pub tables: Rc<RideTables>,
    /// `puccigusoCharTbl[kind]`, read by `ccLoadFLAddOne`.
    pg_body: Rc<Body>,
    /// `pcgs`, made on the task's first frame.
    pub obj: Option<TownRideObj>,
    /// `pgR`, `pgDIN`.
    pub g: Globals,
    /// `Main`s run so far.
    pub mains: u32,
}

/// `pcgs` and its two clumps.
pub struct TownRideObj {
    pub ride: Ride,
    /// Kite's riding clump (over his own `ctu1body`) and the Grunty's.
    pub kite: Char,
    pub pg: Char,
    notes: Vec<Note>,
}

/// The town as the ride sees it.
struct TownStage<'a> {
    hits: &'a mut Hits,
    camera: &'a mut Camera,
    pad: CamPad,
    volume: piney_data::volume::Volume,
    /// `plw.pos` as the frame has it (Main writes it as it goes).
    player: V4,
    /// `posView` as `cameraSetManual` or `cameraSetEyeLevel` last had it.
    view: V4,
    kite: &'a mut Char,
    pg: &'a mut Char,
    notes: &'a mut Vec<Note>,
    out: Option<(V4, V4, bool)>,
    events: &'a mut Vec<RaceEvent>,
    /// The Grunty's notes' sounds (`ccSeSetParamInu(param, pcgs)`).
    sounds: Vec<(u32, u32)>,
}

impl TownStage<'_> {
    fn ch(&mut self, a: RideAnm) -> &mut Char {
        match a {
            RideAnm::Kite => self.kite,
            RideAnm::Pg => self.pg,
        }
    }

    /// `ccGetCameraTransparency` with the active camera.
    fn fade(&self, pos: V4, width: F, height: F, far: F, len: F, hide: &mut bool) -> F {
        let cam = self.camera.active();
        crate::char::camera_transparency(
            self.volume,
            pos,
            self.player,
            cam.pos,
            cam.deg[1],
            width,
            height,
            far,
            len,
            hide,
        )
    }
}

impl RideWorld for TownStage<'_> {
    fn land(&mut self, pos: V4, mask: u32) -> F {
        self.hits.land(pos, mask)
    }
    fn hit_attribute(&mut self) -> u32 {
        self.hits.attribute()
    }
    fn hit_result(&mut self) -> Option<u32> {
        (self.hits.num != 0).then_some(self.hits.nearest.att)
    }
    fn line(&mut self, from: V4, to: V4, mask: u32) -> F {
        match self.hits.line(from, to, mask, 1) {
            Some((_, d)) => d,
            None => piney_battle::geom::MINUS_ONE,
        }
    }
    fn collide(&mut self, hit: &mut CharHit) -> i32 {
        let mut b = spc::to_body(BODY_WHO, None, hit, false);
        let r = self.hits.collision_detection(&mut b);
        hit.offset = b.offset;
        hit.attribute = b.attribute;
        r
    }
    fn hit_switch(&mut self, hit: &mut CharHit, on: bool) {
        let mut b = spc::to_body(BODY_WHO, None, hit, false);
        self.hits.set_hit_sw(&mut b, on);
        hit.sw = b.sw;
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
    fn camera_reset(&mut self) -> Option<F> {
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
        self.camera.set_manual(pos_view, &pad, 0, self.hits);
    }
    fn camera_set(&mut self) {
        let p = [self.view[0], self.view[1], 0, ONE];
        match self.hits.bounds {
            Some(b) => self.camera.set_wrapped(p, b),
            None => self.camera.set(p),
        }
    }
    fn camera_transparency(&mut self, pos: V4, width: F, height: F, far: F, len: F) -> (F, bool) {
        let mut hide = false;
        let f = self.fade(pos, width, height, far, len, &mut hide);
        (f, hide)
    }
    fn anim_set(&mut self, a: RideAnm, name: &str) {
        self.ch(a).set_anim(name);
    }
    fn anim_forward(&mut self, a: RideAnm, step: u16) -> i16 {
        let ch = self.ch(a);
        ch.play.frame_spd = u32::from(step);
        let file = ch.body.file.clone();
        let (ended, notes) = ch.play.forward_notes(&file);
        if a == RideAnm::Pg {
            *self.notes = notes.into_iter().map(|(event, param)| Note { event, param }).collect();
        }
        i16::from(ended)
    }
    fn anim_notes(&mut self, _a: RideAnm) -> Vec<Note> {
        self.notes.clone()
    }
    fn leg(&mut self, name: &str) -> V4 {
        let pg = &*self.pg;
        let Some(n) = pg.body.node(name) else { return [0, 0, 0, ONE] };
        let worlds = pg.body.worlds(&pg.play, pg.root());
        worlds.get(&n).map_or([0, 0, 0, ONE], |m| {
            let t = m.w_axis;
            [t.x.to_bits(), t.y.to_bits(), t.z.to_bits(), ONE]
        })
    }
    fn draw(&mut self, pos: V4, set_transparency: F) -> F {
        // ccChar::Draw in a town: 4000 over 400, the camera's nearness
        // unless it is the eye view.
        let mut hide = self.camera.active().kind == crate::camera::kind::EYE;
        let fade = self.fade(pos, ride::SIZE, ride::SIZE, crate::char::FADE_FAR, crate::char::FADE_LEN, &mut hide);
        let t = crate::ee::mul(set_transparency, fade);
        let k = &mut *self.kite;
        k.pos = pos;
        k.transparency = t;
        k.shadow_t = t;
        k.drawn = !(crate::ee::lt(t, crate::char::MIN_DRAWN) && !hide);
        t
    }
    fn out(&mut self, o: Out) {
        match o {
            Out::Matrix { anm, pos, rot } => {
                let ch = self.ch(anm);
                ch.pos = pos;
                ch.dirc = rot;
            }
            Out::Player { pos, rot, pause } => {
                self.player = pos;
                self.out = Some((pos, rot, pause));
            }
            Out::DrawPg { transparency, shadow, shaded, .. } => {
                let pg = &mut *self.pg;
                pg.transparency = transparency;
                pg.shadow_t = shadow;
                pg.hit_attribute = if shaded { SHADED } else { 0 };
                pg.drawn = true;
            }
            Out::Sound { param, attribute } => self.sounds.push((param, attribute)),
            Out::Smoke { pos, v, s, life, t } => self.events.push(RaceEvent::RideSmoke { pos, v, s, life, t }),
            // No streaming nor dungeon in a town.
            Out::AddCenter { .. } | Out::Enter { .. } => {}
        }
    }
}

impl crate::World {
    /// `ccPuccigusoStart(kind)` in a town (MUT gcmn 0x0052d6f0): refused
    /// while riding; else the menu's ban and panels, `pgRideFlag` and
    /// `pgR`, Kite asleep, standing and off the command lists with his body
    /// out of the collision, the kind's file read and the task started.
    pub(crate) fn ride_start_town(&mut self, kind: i32) -> bool {
        if self.ride.as_ref().is_some_and(|r| r.g.pg_r != 0) {
            return false;
        }
        self.race_events.push(RaceEvent::Forbid(1));
        self.race_events.push(RaceEvent::Panel(3));
        let tables = Rc::new(RideTables::of(self.volume));
        let file = tables.file(kind).to_string();
        let body = match Body::read(&self.archive, &file, TRALL) {
            Ok(b) => Rc::new(b),
            Err(e) => {
                tracing::warn!("the Grunty {file}: {e}");
                return false;
            }
        };
        // ccSPC::Sleep(1): Kite alone, standing, off the lists, asleep;
        // in the party, his body out.
        let p = &mut self.player;
        p.acts.stop_flag = true;
        p.listed = false;
        self.town.base.hits.hit_disable(&mut p.hit_body);
        self.kite_asleep = true;
        self.camera.type_lock = false;
        self.ride =
            Some(TownRide { kind, tables, pg_body: body, obj: None, g: Globals { pg_r: 1, pg_din: 0 }, mains: 0 });
        true
    }

    /// `pgRideFlag`: the town's ride is on.
    pub fn town_riding(&self) -> bool {
        self.ride.is_some()
    }

    /// `ccThPucciguso`'s slot in a town (after the player's): the object
    /// made on the first frame (paused), `Main` each frame; from the 32nd
    /// on the race's end ends the ride first (`ccPuccigusoExit`).
    pub(crate) fn ride_slot(&mut self, pad: &CamPad, race: Option<RaceRide>, end: bool) {
        let Some(mut r) = self.ride.take() else { return };
        if r.mains >= FIRST_MAINS && end {
            self.ride_exit_town(r);
            return;
        }
        if r.obj.is_none() {
            let (pos, rot) = (self.player.body.pos, self.player.body.dirc);
            let mut kite = Char::new(
                Rc::new(self.kite.body.clone()),
                r.tables.anim(ride::act::IDLE),
                pos,
                rot,
                ride::SIZE,
                ride::SIZE,
            );
            if let Some(k) = kite.as_mut() {
                k.clut_swaps = self.kite.swaps(self.save.save.u8(piney_data::save::offset::PLCOL) != 0);
            }
            let pg = Char::new(
                r.pg_body.clone(),
                &r.tables.anim_pg(ride::act::IDLE, r.kind),
                pos,
                rot,
                ride::SIZE,
                ride::SIZE,
            );
            let (Some(mut kite), Some(mut pg)) = (kite, pg) else {
                tracing::warn!("the Grunty's clumps");
                return;
            };
            let mut notes = Vec::new();
            let mut events = Vec::new();
            let mut next = || self.rand.rand();
            let mut w = TownStage {
                hits: &mut self.town.base.hits,
                camera: &mut self.camera,
                pad: *pad,
                volume: self.volume,
                player: pos,
                view: self.player.pos_view,
                kite: &mut kite,
                pg: &mut pg,
                notes: &mut notes,
                out: None,
                events: &mut events,
                sounds: Vec::new(),
            };
            let mut ride = Ride::new(r.kind, pos, rot, &r.tables, &mut w, &mut next);
            ride.flags |= ride::flag::PAUSE;
            r.obj = Some(TownRideObj { ride, kite, pg, notes });
        }
        let Some(o) = r.obj.as_mut() else { return };
        o.kite.drawn = false;
        o.pg.drawn = false;
        let input = Input {
            pad: piney_battle::kite::Pad { pow_l: pad.pow_l, dirc_l: pad.dirc_l },
            pause: self.player.acts.pause,
            dne: false,
            bounds: crate::combat::Combat::bounds(0, &self.town.base.hits),
            area: 0,
            race,
        };
        let mut events = Vec::new();
        let TownRideObj { ride: rd, kite, pg, notes } = o;
        let mut next = || self.rand.rand();
        let mut w = TownStage {
            hits: &mut self.town.base.hits,
            camera: &mut self.camera,
            pad: *pad,
            volume: self.volume,
            player: self.player.body.pos,
            view: self.player.pos_view,
            kite,
            pg,
            notes,
            out: None,
            events: &mut events,
            sounds: Vec::new(),
        };
        ride::main(rd, &mut w, &input, &mut r.g, &r.tables, &mut next);
        let (out, view, sounds) = (w.out, w.view, std::mem::take(&mut w.sounds));
        if let Some((pos, rot, pause)) = out {
            self.player.body.pos = pos;
            self.player.body.dirc = rot;
            self.player.acts.pause = pause;
        }
        self.player.pos_view = view;
        let at = rd.pos;
        self.race_events.extend(events);
        self.race_events.extend(sounds.into_iter().map(|(param, attribute)| RaceEvent::RideSound {
            param,
            attribute,
            pos: at,
        }));
        r.mains += 1;
        self.ride = Some(r);
    }

    /// `ccPuccigusoExit` in a town (MUT gcmn 0x0052d960): the ride's body
    /// out, Kite's back unless he is down, `ccSPC::Wakeup(1)` (act 2,
    /// standing, free, on the lists), the object and the flags gone.
    fn ride_exit_town(&mut self, r: TownRide) {
        self.camera.type_lock = true;
        if let Some(o) = r.obj {
            let mut b = spc::to_body(BODY_WHO, None, &o.ride.hit, false);
            self.town.base.hits.hit_disable(&mut b);
        }
        let p = &mut self.player;
        if p.dead == 0 {
            self.town.base.hits.hit_enable(&mut p.hit_body);
        }
        p.skill_status = 0;
        p.acts.act = crate::motion::act::IDLE;
        p.acts.act_old = -1;
        p.acts.restraint = false;
        p.acts.pause = false;
        p.acts.stop_flag = true;
        p.listed = true;
        self.kite_asleep = false;
        self.camera.type_lock = false;
    }

    /// The ride's clumps as its last `Main` left them.
    pub(crate) fn draw_ride(&self, layers: &mut Layers, to_screen: Mat4, lights: &TownLights) {
        if let Some(o) = self.ride.as_ref().and_then(|r| r.obj.as_ref()) {
            o.kite.draw(layers, to_screen, lights);
            o.pg.draw(layers, to_screen, lights);
        }
    }

    /// 0x005310b0(pos, rot): the ride set down there, when there is one.
    pub(crate) fn ride_place(&mut self, pos: V4, rot: V4) {
        if let Some(o) = self.ride.as_mut().and_then(|r| r.obj.as_mut()) {
            o.ride.pos = pos;
            o.ride.rot = rot;
        }
    }
}
