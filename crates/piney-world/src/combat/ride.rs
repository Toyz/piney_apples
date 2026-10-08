//! The riding Grunty on the field's tasks: `ccPucciguso`
//! ([`piney_battle::ride`]) and the state around it the battle's tasks read
//! ([`Riding`]). `ccThPucciguso` (gcmn 0x00510880, priority 49) runs where
//! `kite::main` would while Kite's and the party's tasks sleep; its world is
//! the frame's [`Stage`] ([`RideStage`]), with the ride's two clumps as its
//! animation players. Sounds and dust go out as [`Show::Ride`]; `pgRideFlag`
//! ([`Riding::flag`]) turns the enemies home and the party's AI off. The
//! sequence around it is piney-game's (docs/engine/grunty-ride.md).

use std::rc::Rc;

use glam::Mat4;
use piney_battle::chara::spc_flag;
use piney_battle::kite::{KiteWorld, Pad};
use piney_battle::party_ai::Crew;
use piney_battle::rand::Rand;
use piney_battle::ride::{self, Globals, Input, Out, Ride, RideAnm, RideTables, RideWorld};
use piney_battle::scene::Scene;
use piney_battle::world::{CharHit, Note, World};
use piney_desktop::layers::Layers;

use super::spc;
use super::stage::{Show, Stage};
use crate::char::{Char, SHADED};
use crate::ee::{F, ONE, V4};
use crate::town::TownLights;

/// The ride's body's key in the collision's list (`ccCharHit` +0x170 of
/// the object; no scene character has it).
pub const BODY_WHO: usize = 0xefff;

/// `pgRideFlag`, `pgR`, `pgDIN`, the party's sleep and the object.
#[derive(Default)]
pub struct Riding {
    /// `pgRideFlag` (0x00378cdc): from `ccPuccigusoStart` to the end of
    /// `ccPuccigusoExit`.
    pub flag: bool,
    /// `pgR`, `pgDIN`.
    pub g: Globals,
    /// `ccSpcSleep` to `ccSpcWakeup`: Kite's and the members' tasks asleep.
    pub asleep: bool,
    /// `ccThPucciguso` calls `Main` this frame (its first 30 frames and on,
    /// and through `ccPuccigusoExit`'s fade out).
    pub main_on: bool,
    /// `pcgs` (0x00378c24).
    pub obj: Option<Box<RideObj>>,
    /// The task started and not yet run: the object is made on its first
    /// frame.
    pub pending: Option<Pending>,
}

/// `ccThPucciguso`'s parameter (tcb +0x14, the kind) and what the object
/// is made of: the tables and the kind's body.
pub struct Pending {
    pub kind: i32,
    pub tables: Rc<RideTables>,
    pub body: Rc<crate::body::Body>,
}

/// The object and its two clumps.
pub struct RideObj {
    pub ride: Ride,
    pub tables: Rc<RideTables>,
    /// Kite's riding clump (+0xd0/+0xd4 over `ctu1body`) and the Grunty's
    /// (+0x1c4/+0x1c8 over the kind's file), with what this frame drew.
    pub kite: Char,
    pub pg: Char,
    /// The notes the Grunty's last step passed.
    pub notes: Vec<Note>,
}

impl RideObj {
    /// `ccPucciguso::ccPucciguso(kind)` at `plw`'s place and heading: Kite's
    /// clump over `kite_body` (his own file, his clut swaps) and the
    /// Grunty's over `pg_body`, both on act 2.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: i32,
        plw_pos: V4,
        plw_rot: V4,
        tables: Rc<RideTables>,
        kite_body: Rc<crate::body::Body>,
        swaps: Vec<(u32, u32)>,
        pg_body: Rc<crate::body::Body>,
        stage: &mut Stage,
        rand: &mut Rand,
    ) -> Option<RideObj> {
        let mut kite = Char::new(kite_body, tables.anim(ride::act::IDLE), plw_pos, plw_rot, ride::SIZE, ride::SIZE)?;
        kite.clut_swaps = swaps;
        let mut pg =
            Char::new(pg_body, &tables.anim_pg(ride::act::IDLE, kind), plw_pos, plw_rot, ride::SIZE, ride::SIZE)?;
        let mut notes = Vec::new();
        let r = {
            let mut w = RideStage { stage, kite: &mut kite, pg: &mut pg, notes: &mut notes, player: None };
            Ride::new(kind, plw_pos, plw_rot, &tables, &mut w, rand)
        };
        Some(RideObj { ride: r, tables, kite, pg, notes })
    }
}

impl Riding {
    /// The two clumps as this frame's `Main` left them: Kite's riding clump
    /// (`ccChar::Draw`) and the Grunty (`DrawPG`), each with its shadow.
    pub fn draw(&self, layers: &mut Layers, to_screen: Mat4, lights: &TownLights) {
        if let Some(o) = &self.obj {
            o.kite.draw(layers, to_screen, lights);
            o.pg.draw(layers, to_screen, lights);
        }
    }
}

/// The frame's world as the ride sees it.
pub struct RideStage<'s, 'a, 'b> {
    pub stage: &'s mut Stage<'a>,
    pub kite: &'b mut Char,
    pub pg: &'b mut Char,
    pub notes: &'b mut Vec<Note>,
    /// `plw`'s place, heading and pause as `Main` wrote them.
    pub player: Option<(V4, V4, bool)>,
}

impl RideStage<'_, '_, '_> {
    fn ch(&mut self, a: RideAnm) -> &mut Char {
        match a {
            RideAnm::Kite => self.kite,
            RideAnm::Pg => self.pg,
        }
    }
}

impl RideWorld for RideStage<'_, '_, '_> {
    fn land(&mut self, pos: V4, mask: u32) -> F {
        World::land(self.stage, pos, mask)
    }
    fn hit_attribute(&mut self) -> u32 {
        World::hit_attribute(self.stage)
    }
    fn hit_result(&mut self) -> Option<u32> {
        KiteWorld::hit_result(self.stage)
    }
    fn line(&mut self, from: V4, to: V4, mask: u32) -> F {
        World::line(self.stage, from, to, mask, 1)
    }
    fn collide(&mut self, hit: &mut CharHit) -> i32 {
        let mut b = spc::to_body(BODY_WHO, None, hit, false);
        let r = self.stage.hits.collision_detection(&mut b);
        hit.offset = b.offset;
        hit.attribute = b.attribute;
        r
    }
    fn hit_switch(&mut self, hit: &mut CharHit, on: bool) {
        let mut b = spc::to_body(BODY_WHO, None, hit, false);
        self.stage.hits.set_hit_sw(&mut b, on);
        hit.sw = b.sw;
    }
    fn camera_type(&mut self) -> i32 {
        KiteWorld::camera_type(self.stage)
    }
    fn camera_id(&mut self) -> i32 {
        KiteWorld::camera_id(self.stage)
    }
    fn camera_rot(&mut self) -> V4 {
        KiteWorld::camera_rot(self.stage)
    }
    fn camera_reset(&mut self) -> Option<F> {
        KiteWorld::camera_reset(self.stage)
    }
    fn clear_camera_reset(&mut self) {
        KiteWorld::clear_camera_reset(self.stage)
    }
    fn camera_set_eye_level(&mut self, pos_eye: V4, angle: &mut V4) {
        KiteWorld::camera_set_eye_level(self.stage, pos_eye, angle)
    }
    fn camera_set_manual(&mut self, pos_view: V4) {
        KiteWorld::camera_set_manual(self.stage, pos_view)
    }
    fn camera_set(&mut self) {
        KiteWorld::camera_set(self.stage)
    }
    fn camera_transparency(&mut self, pos: V4, width: F, height: F, far: F, len: F) -> (F, bool) {
        let mut hide = false;
        let f = self.stage.transparency(pos, width, height, far, len, &mut hide);
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
        let (t, drawn) = self.stage.char_draw(pos, ride::SIZE, ride::SIZE, set_transparency, true);
        let k = &mut *self.kite;
        k.pos = pos;
        k.transparency = t;
        k.shadow_t = t;
        k.drawn = drawn;
        t
    }
    fn out(&mut self, o: Out) {
        match o {
            Out::AddCenter { x, y } => self.stage.hits.add_center(x, y),
            Out::Enter { .. } => self.stage.enter = true,
            Out::Matrix { anm, pos, rot } => {
                let ch = self.ch(anm);
                ch.pos = pos;
                ch.dirc = rot;
            }
            Out::Player { pos, rot, pause } => self.player = Some((pos, rot, pause)),
            Out::DrawPg { transparency, shadow, shaded, .. } => {
                let pg = &mut *self.pg;
                pg.transparency = transparency;
                pg.shadow_t = shadow;
                pg.hit_attribute = if shaded { SHADED } else { 0 };
                pg.drawn = true;
            }
            o => self.stage.shows.push(Show::Ride(o)),
        }
    }
}

/// `ccThPucciguso`'s slot (after `ccThPlayer`): `ccPucciguso::Main` while
/// [`Riding::main_on`], with Kite's pause in and his place, heading and
/// pause out (`plw`). Nothing is drawn of the ride on a frame it does not
/// run.
#[allow(clippy::too_many_arguments)]
pub fn frame(
    r: &mut Riding,
    stage: &mut Stage,
    scene: &mut Scene,
    crew: &mut Crew,
    kite: usize,
    rand: &mut Rand,
    pad: Pad,
    area: i32,
) {
    let Riding { obj, g, main_on, pending, .. } = r;
    // ccThPucciguso's first frame: new ccPucciguso(kind) where plw stands,
    // Kite's riding clump over his own file.
    if let Some(p) = pending.take() {
        let pos = scene.chars[kite].pos;
        let rot = crew.spc.get(&kite).map_or([0; 4], |s| s.dirc);
        let own = stage.cast.get(kite).map(|a| (a.ch.body.clone(), a.swaps.clone()));
        if let Some((kb, swaps)) = own {
            *obj = RideObj::new(p.kind, pos, rot, p.tables, kb, swaps, p.body, stage, rand).map(Box::new);
        }
    }
    let Some(o) = obj.as_deref_mut() else { return };
    o.kite.drawn = false;
    o.pg.drawn = false;
    if !*main_on {
        return;
    }
    let input = Input {
        pad,
        pause: scene.chars[kite].spc_char.flags & spc_flag::PAUSE != 0,
        dne: false,
        bounds: stage.bounds,
        area,
        // The Flag Race rides in towns only.
        race: None,
    };
    let RideObj { ride: rd, tables, kite: kc, pg, notes } = o;
    let mut w = RideStage { stage, kite: kc, pg, notes, player: None };
    ride::main(rd, &mut w, &input, g, tables, rand);
    if let Some((pos, rot, pause)) = w.player {
        let ch = &mut scene.chars[kite];
        ch.pos = pos;
        if pause {
            ch.spc_char.flags |= spc_flag::PAUSE;
        } else {
            ch.spc_char.flags &= !spc_flag::PAUSE;
        }
        crew.spc.entry(kite).or_default().dirc = rot;
    }
}
