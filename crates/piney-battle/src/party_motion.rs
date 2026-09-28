//! The party's movement assembled: one [`Runtime`] that performs every
//! movement call the party AI makes with the ported code, in place of the
//! runtime's.
//!
//! In the game these are all methods of the same `ccAI` (`personal.cpp`)
//! and the `ccNavi` inside it, calling each other directly. The port has
//! them in three modules, each checked on its own with the others'
//! calls recorded:
//!
//! | call | performed by |
//! | --- | --- |
//! | `FollowPlayer`, `LeavePlayer`, `FollowTarget`, `FollowTargetDirc` | [`crate::follow::Follow`] |
//! | `PathFinding` | [`crate::navi::Navi::path_finding`] on the member's navigation |
//! | `FollowBeacon`, `GoalBeacon`, `ManualControl`, `ActInTown` | [`crate::ai_move::perform`] |
//! | `PlayerAttack` | [`crate::kite::attack`] |
//! | `HitEnable` | [`World::hit_switch`] on the member's `bodyHit` ([`crate::party_ai::Spc::body_hit`]) |
//!
//! Everything else (skill and item requests, the chat lines, transfers,
//! the party changes, line checks the decisions make) goes on to the
//! runtime's own [`Runtime`], `inner`.
//!
//! These functions call the world while the frame that called them also
//! holds it (a member's frame calls the collision, then `ccAI::Brains`
//! asks to follow Kite, which calls the collision again). Both borrow one
//! world through a [`RefCell`], never at the same time: [`Share`] is a
//! handle on it that implements the world's traits by borrowing it for
//! each call.

use std::cell::RefCell;

use crate::exp::Party;
use crate::fellow::{FellowWorld, MotionTables};
use crate::follow::{Follow, FollowState};
use crate::geom::{F, V4};
use crate::navi::{Navi, NaviWorld, PathMap, TownMap};
use crate::party_ai::{Call, Crew, Game, Parts, Runtime};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::tables::Tables;
use crate::world::{AnmSlot, CharHit, Note, World};

/// A handle on a world shared through a [`RefCell`]: each call borrows it
/// for the call alone. [`NaviWorld::map_2d`] hands out a copy of the map,
/// kept in the handle.
pub struct Share<'c, 'w, W: ?Sized> {
    pub cell: &'c RefCell<&'w mut W>,
    map: Vec<u8>,
}

impl<'c, 'w, W: ?Sized> Share<'c, 'w, W> {
    pub fn new(cell: &'c RefCell<&'w mut W>) -> Self {
        Share { cell, map: Vec::new() }
    }
}

impl<W: World + ?Sized> World for Share<'_, '_, W> {
    fn w2p(&mut self, pos: V4) -> V4 {
        self.cell.borrow_mut().w2p(pos)
    }
    fn p2w(&mut self, pos: V4) -> V4 {
        self.cell.borrow_mut().p2w(pos)
    }
    fn land(&mut self, pos: V4, mask: u32) -> F {
        self.cell.borrow_mut().land(pos, mask)
    }
    fn hit_attribute(&mut self) -> u32 {
        self.cell.borrow_mut().hit_attribute()
    }
    fn line(&mut self, from: V4, to: V4, mask: u32, kind: i32) -> F {
        self.cell.borrow_mut().line(from, to, mask, kind)
    }
    fn collide(&mut self, who: usize, hit: &mut CharHit) -> i32 {
        self.cell.borrow_mut().collide(who, hit)
    }
    fn hit_char_type(&mut self) -> u32 {
        self.cell.borrow_mut().hit_char_type()
    }
    fn hit_switch(&mut self, who: usize, hit: &mut CharHit, on: bool) {
        self.cell.borrow_mut().hit_switch(who, hit, on)
    }
    fn camera_deg(&mut self, pos: V4, deg: i16) -> bool {
        self.cell.borrow_mut().camera_deg(pos, deg)
    }
    fn camera_transparency(&mut self, pos: V4, width: F, height: F, far: F, len: F) -> F {
        self.cell.borrow_mut().camera_transparency(pos, width, height, far, len)
    }
    fn anim_set(&mut self, who: usize, slot: AnmSlot, name: &str) {
        self.cell.borrow_mut().anim_set(who, slot, name)
    }
    fn anim_frame(&mut self, who: usize, slot: AnmSlot) -> u16 {
        self.cell.borrow_mut().anim_frame(who, slot)
    }
    fn anim_forward(&mut self, who: usize, slot: AnmSlot, step: u16) -> i16 {
        self.cell.borrow_mut().anim_forward(who, slot, step)
    }
    fn anim_notes(&mut self, who: usize, slot: AnmSlot) -> Vec<Note> {
        self.cell.borrow_mut().anim_notes(who, slot)
    }
}

impl<W: NaviWorld + ?Sized> NaviWorld for Share<'_, '_, W> {
    fn map_2d_info(&mut self) -> [i32; 3] {
        self.cell.borrow_mut().map_2d_info()
    }
    fn map_2d(&mut self) -> &[u8] {
        self.map = self.cell.borrow_mut().map_2d().to_vec();
        &self.map
    }
    fn route_search_by_map(&mut self, navi: &mut Navi, finish: &mut i16, start: V4, goal: V4) -> i32 {
        self.cell.borrow_mut().route_search_by_map(navi, finish, start, goal)
    }
    fn navi_point(&mut self, name: i16) -> V4 {
        self.cell.borrow_mut().navi_point(name)
    }
    fn marker(&mut self, k: i32) -> V4 {
        self.cell.borrow_mut().marker(k)
    }
}

impl<W: FellowWorld + ?Sized> FellowWorld for Share<'_, '_, W> {
    fn trans_mode(&mut self) -> bool {
        self.cell.borrow_mut().trans_mode()
    }
    fn trans_center(&mut self) -> V4 {
        self.cell.borrow_mut().trans_center()
    }
    fn event_area(&mut self) -> bool {
        self.cell.borrow_mut().event_area()
    }
    fn draw(&mut self, who: usize) -> bool {
        self.cell.borrow_mut().draw(who)
    }
}

/// What the party's movement keeps between frames: the following's
/// `aiOpenDirc`, the dungeon's path-finding map (`buf`, `buf2`,
/// `bufFlag`), the Root Town's landmarks.
#[derive(Clone, Debug, Default)]
pub struct Keep {
    pub follow: FollowState,
    pub path: PathMap,
    pub town: TownMap,
}

/// The party's movement as a [`Runtime`], borrowed for a frame.
pub struct Movement<'a, 'c, 'w, W: NaviWorld + ?Sized> {
    pub t: &'a Tables,
    pub mt: &'a MotionTables,
    /// `ccPartyManager`: Kite is member 0.
    pub party: &'a Party,
    /// `ccGame.area`: 0 the Root Town, 1 a field, 2 a dungeon.
    pub game: &'a Game,
    pub keep: &'a mut Keep,
    /// `ccSpcRegistryNum()` (gcmn 0x005a1620): the members registered.
    pub spc_registry_num: i32,
    pub world: &'c RefCell<&'w mut W>,
    /// The runtime's own calls.
    pub inner: &'a mut dyn Runtime,
}

/// The follow calls' [`Follow::inner`]: `PathFinding` performed on the
/// member's navigation, everything else passed on.
struct PathRuntime<'a, 'c, 'w, W: NaviWorld + ?Sized> {
    area: i32,
    path: &'a mut PathMap,
    world: Share<'c, 'w, W>,
    inner: &'a mut dyn Runtime,
}

/// `ccNavi::PathFindingInDungeon(from, to)` (gcmn 0x005148e0) of `me`'s
/// AI, as [`crate::ai_move`]'s `Ctx::path_finding` does it.
fn path_finding(
    crew: &mut Crew,
    me: usize,
    area: i32,
    path: &mut PathMap,
    w: &mut dyn NaviWorld,
    from: V4,
    to: V4,
) -> i32 {
    let Some(a) = crew.ais.get_mut(&me) else { return 0 };
    let mut finish = a.navi_finish;
    let r = a.navi.path_finding(&mut finish, area, path, w, from, to);
    a.navi_finish = finish;
    r
}

impl<W: NaviWorld + ?Sized> Runtime for PathRuntime<'_, '_, '_, W> {
    fn call(&mut self, call: Call, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) -> i32 {
        match call {
            Call::PathFinding { me, from, to } => {
                path_finding(crew, me, self.area, self.path, &mut self.world, from, to)
            }
            other => self.inner.call(other, scene, crew, rng),
        }
    }
    fn call_ctx(&mut self, call: Call, p: Parts<'_>) -> i32 {
        match call {
            Call::PathFinding { .. } => self.call(call, p.scene, p.crew, p.rng),
            other => self.inner.call_ctx(other, p),
        }
    }
    fn w2p(&mut self, pos: V4) -> V4 {
        self.world.w2p(pos)
    }
}

/// The runtime of the calls [`crate::ai_move`]'s movers make while they
/// hold the navigation map: the following (`ActInTown`'s `FollowPlayer`
/// and `LeavePlayer`), `HitEnable`, and the rest passed on. A path search
/// asked from here (a follow in a dungeon, which no mover makes) has no
/// map to search and is passed on as well.
struct Nested<'a, 'c, 'w, W: NaviWorld + ?Sized> {
    t: &'a Tables,
    mt: &'a MotionTables,
    party: &'a Party,
    game: &'a Game,
    follow: &'a mut FollowState,
    world: &'c RefCell<&'w mut W>,
    inner: &'a mut dyn Runtime,
}

impl<W: NaviWorld + ?Sized> Runtime for Nested<'_, '_, '_, W> {
    fn call(&mut self, call: Call, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) -> i32 {
        match call {
            Call::FollowPlayer { .. }
            | Call::LeavePlayer { .. }
            | Call::FollowTarget { .. }
            | Call::FollowTargetDirc { .. } => {
                let mut w = Share::new(self.world);
                let mut f = Follow {
                    t: self.t,
                    mt: self.mt,
                    party: self.party,
                    game: self.game,
                    state: &mut *self.follow,
                    world: &mut w,
                    inner: &mut *self.inner,
                };
                f.call(call, scene, crew, rng)
            }
            Call::HitEnable { ch } => {
                hit_enable(self.world, crew, ch);
                0
            }
            other => self.inner.call(other, scene, crew, rng),
        }
    }
    fn call_ctx(&mut self, call: Call, p: Parts<'_>) -> i32 {
        match call {
            Call::FollowPlayer { .. }
            | Call::LeavePlayer { .. }
            | Call::FollowTarget { .. }
            | Call::FollowTargetDirc { .. }
            | Call::HitEnable { .. } => self.call(call, p.scene, p.crew, p.rng),
            other => self.inner.call_ctx(other, p),
        }
    }
    fn w2p(&mut self, pos: V4) -> V4 {
        self.world.borrow_mut().w2p(pos)
    }
}

/// `ccCharHit::HitEnable()` (main 0x00153310) on `ch`'s `bodyHit`.
fn hit_enable<W: World + ?Sized>(world: &RefCell<&mut W>, crew: &mut Crew, ch: usize) {
    let s = crew.spc.entry(ch).or_default();
    world.borrow_mut().hit_switch(ch, &mut s.body_hit, true);
}

impl<W: NaviWorld + ?Sized> Movement<'_, '_, '_, W> {
    fn follow_call(&mut self, call: Call, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) -> i32 {
        let mut pr = PathRuntime {
            area: self.game.area,
            path: &mut self.keep.path,
            world: Share::new(self.world),
            inner: &mut *self.inner,
        };
        let mut w = Share::new(self.world);
        let mut f = Follow {
            t: self.t,
            mt: self.mt,
            party: self.party,
            game: self.game,
            state: &mut self.keep.follow,
            world: &mut w,
            inner: &mut pr,
        };
        f.call(call, scene, crew, rng)
    }
}

impl<W: NaviWorld + ?Sized> Runtime for Movement<'_, '_, '_, W> {
    fn call(&mut self, call: Call, scene: &mut Scene, crew: &mut Crew, rng: &mut dyn Rng) -> i32 {
        match call {
            Call::FollowPlayer { .. }
            | Call::LeavePlayer { .. }
            | Call::FollowTarget { .. }
            | Call::FollowTargetDirc { .. } => self.follow_call(call, scene, crew, rng),
            Call::PathFinding { me, from, to } => {
                let mut w = Share::new(self.world);
                path_finding(crew, me, self.game.area, &mut self.keep.path, &mut w, from, to)
            }
            Call::HitEnable { ch } => {
                hit_enable(self.world, crew, ch);
                0
            }
            other => self.inner.call(other, scene, crew, rng),
        }
    }

    fn call_ctx(&mut self, call: Call, p: Parts<'_>) -> i32 {
        match call {
            Call::FollowBeacon { .. }
            | Call::GoalBeacon { .. }
            | Call::ManualControl { .. }
            | Call::ActInTown { .. } => {
                let mut nested = Nested {
                    t: self.t,
                    mt: self.mt,
                    party: self.party,
                    game: self.game,
                    follow: &mut self.keep.follow,
                    world: self.world,
                    inner: &mut *self.inner,
                };
                let mut w = Share::new(self.world);
                let mut nav = crate::ai_move::Nav {
                    world: &mut w,
                    path: &mut self.keep.path,
                    town: &self.keep.town,
                    spc_registry_num: self.spc_registry_num,
                };
                let mut ctx = p.ctx(&mut nested);
                crate::ai_move::perform(call, &mut ctx, &mut nav).unwrap_or(0)
            }
            Call::PlayerAttack { me, target, n } => {
                let mut ctx = p.ctx(self);
                crate::kite::attack(&mut ctx, me, target, n)
            }
            Call::FollowPlayer { .. }
            | Call::LeavePlayer { .. }
            | Call::FollowTarget { .. }
            | Call::FollowTargetDirc { .. }
            | Call::PathFinding { .. }
            | Call::HitEnable { .. } => self.call(call, p.scene, p.crew, p.rng),
            other => self.inner.call_ctx(other, p),
        }
    }

    fn w2p(&mut self, pos: V4) -> V4 {
        self.world.borrow_mut().w2p(pos)
    }
}
