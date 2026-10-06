//! A survey of the story's main events (Infection's 1-31, Mutation's
//! 101-116): each `--mode story:N` start driven by the story autopilot
//! ([`StoryPilot`]) for a while, with what it reached and any panic. A
//! diagnostic: `cargo test --release -p piney-game story_survey -- --ignored
//! --nocapture`. `PINEY_SURVEY_ONLY=N` one start, `PINEY_SURVEY_FRAMES` the
//! frames, `PINEY_SURVEY_CALLS` the last host calls, `PINEY_SURVEY_GOD` the
//! party kept up (and the infection at 0), `PINEY_DEBUG_PILOT` the pilot's trace.

use piney_battle::enemy_ai::act as enemy_act;
use piney_world::area::kind;
use piney_world::field_world::Place;

use super::shrine::Walker;
use super::*;

/// The story autopilot with the walks: [`story_player`] in the towns, on
/// the desktop and the top page, and whenever an event or a menu holds
/// Kite; in a field whose dungeon the story wants ([`story_wants`]), Kite
/// to the entrance's middle and on west down its steps (as
/// `walk_into_dungeon`); in a dungeon, to the room of the event point the
/// story wants ([`Walker`]).
pub(super) struct StoryPilot {
    walker: Walker,
    /// The way to the entrance's door cells, next point first, and where
    /// Kite stood at the last check.
    path: Vec<[f32; 2]>,
    mark: Option<[f32; 2]>,
    /// The same for the walk to a field's Data Bug.
    chase: Vec<[f32; 2]>,
    chase_mark: Option<[f32; 2]>,
    /// The golden goblin run after ([`Self::approach_foe`]), kept to while
    /// it lives: turning to whichever is nearest loses them all. Where and
    /// when the last check of the rush found Kite, and until when he goes
    /// round a wall that stopped it.
    quarry: Option<usize>,
    rush_mark: Option<(u64, [f32; 2])>,
    detour_until: u64,
    /// Until when, and to which side (1 or -1, a right angle off the
    /// way), the walk to a dungeon goes round what stopped it.
    sidestep: (u64, f32),
    /// The fight's action under way, whether its menus have opened, when
    /// it began; when the last one ended; this fight's Skills! (false) or
    /// Magic! (true) order, once out; the target menu's presses toward a
    /// boss's core.
    action: Option<Action>,
    opened: bool,
    began: u64,
    ended: Option<u64>,
    skills_ordered: Option<bool>,
    target_tries: u32,
    /// The frame, and when First Aid! last went out.
    now: u64,
    first_aid_at: Option<u64>,
    /// A foe of this room has held the walk (see [`Walker::held`]): any
    /// foe's broken protect is drained then, not only a boss's.
    held: bool,
    /// In a field: the event NPCs spoken to here (`add_target`s stay on
    /// the list after), the one just spoken to and when, and the place.
    talked: Vec<i32>,
    talking: Option<(i32, u64)>,
    talk_place: Option<(i32, i32)>,
    /// In a town: the same for its event NPCs, and the town.
    town_talked: Vec<(piney_world::entry::Kind, i32)>,
    town_talking: Option<((piney_world::entry::Kind, i32), u64)>,
    talk_town: Option<i32>,
    /// In a field or dungeon the story wants nothing of: since when, and
    /// whether the pilot is on its way out (PERSONAL, Gate Out).
    idle_since: Option<u64>,
    gating: bool,
    /// Rooms whose box or statue for a wanted item was found opened:
    /// field, dungeon, floor, room.
    statues_done: Vec<(i32, i32, i32, i32)>,
    /// Since when the pilot has stood in such a room finding nothing to open.
    nothing_since: Option<((i32, i32, i32, i32), u64)>,
    /// Since when he has stood in a wanted event point's room (see
    /// [`Self::open_point_statue`]).
    point_since: Option<((i32, i32, i32, i32), u64)>,
    /// What Kite carries out of town (bought there when short), the fields
    /// whose foes it was weighed for (once each), and whether he is on his
    /// way out of one to buy it ([`Self::provision`]).
    buys: Vec<Buy>,
    stocked: Vec<i32>,
    leaving: bool,
}

/// What the pilot does in a fight through the menus.
#[derive(Clone, Copy, Debug)]
enum Action {
    /// A CHAT order to both members: the page and row (Skill Usage 0:
    /// Skills! 0, First Aid! 1).
    Chat { page: i16, row: i16 },
    /// CHAT's Members page: member `slot` to use `skill` from a page of
    /// his skills (magic 1, recovery 2) on the target's row (Designate
    /// Skill).
    Designate { slot: usize, page: i16, skill: i16, target: i16 },
    /// Kite's own skill from a page of Skills (attack 0, magic 1,
    /// recovery 2), at the target's row, or the one the menu offers.
    Skill { page: i16, skill: i16, target: Option<i16> },
    /// Kite's item (Items, the first page) on the target's row.
    Item { cat: i8, id: i16, target: i16 },
}

/// The recovery Kite carries or knows: Repth, a Healing Potion,
/// Resurrect, a Mage's Soul.
const REPTH: i16 = 150;
const DATA_DRAIN: i16 = 2;
const RIP_MAEN: i16 = 180;
const POTION: (i8, i16) = (10, 1);
const RESURRECT: (i8, i16) = (10, 5);
const MAGES_SOUL: (i8, i16) = (10, 18);
/// A Speed Charm (`itemTblD` row 56): Ap Do (177), haste for 9000 frames.
const SPEED_CHARM: (i8, i16) = (11, 56);
/// The blows a Speed Charm's haste (9000 frames) lands on golden goblins,
/// about: 27 in 14,000 frames on GOB3-1's, 37 in 18,000 on GOB3-4's.
const BLOWS_A_CHARM: i32 = 15;

impl Default for StoryPilot {
    fn default() -> Self {
        StoryPilot {
            walker: Walker::wary(),
            path: Vec::new(),
            mark: None,
            chase: Vec::new(),
            chase_mark: None,
            quarry: None,
            rush_mark: None,
            detour_until: 0,
            sidestep: (0, -1.0),
            action: None,
            opened: false,
            began: 0,
            ended: None,
            skills_ordered: None,
            target_tries: 0,
            now: 0,
            first_aid_at: None,
            held: false,
            talked: Vec::new(),
            talking: None,
            talk_place: None,
            town_talked: Vec::new(),
            town_talking: None,
            talk_town: None,
            idle_since: None,
            gating: false,
            statues_done: Vec::new(),
            nothing_since: None,
            point_since: None,
            buys: Vec::new(),
            stocked: Vec::new(),
            leaving: false,
        }
    }
}

impl StoryPilot {
    pub(super) fn next(&mut self, s: &Session, f: u64) -> Raw {
        if let Stage::Area(a) = &s.stage
            && !a.streaming()
        {
            self.provision(a);
            if self.leaving
                && let Some(raw) = self.gate_out(a, f).or_else(|| out_of_battle(a))
            {
                return raw;
            }
        }
        if let Stage::World(_) = &s.stage {
            self.leaving = false;
        }
        if let Stage::Area(a) = &s.stage
            && !a.streaming()
            && let Some(raw) = self.fight(a, f)
        {
            return raw;
        }
        if let Stage::Area(a) = &s.stage
            && !a.streaming()
            && a.world().scene().area == kind::FIELD
            && let Some(raw) = self.talk_in_field(a, f)
        {
            return raw;
        }
        if let Stage::Area(a) = &s.stage
            && !a.streaming()
            && let Some(vm) = a.vm()
        {
            let w = a.world();
            let sc = w.scene();
            let wants = story_wants(vm, &w.state().save);
            if sc.area == kind::DUNGEON {
                let to = lake_below(w, &wants)
                    .or_else(|| {
                        wants.iter().find_map(|&x| match x {
                            Want::Point(n) => vm.mng.point(n).filter(|p| p.floor >= 0 && p.block >= 0),
                            _ => None,
                        })
                    })
                    .map(|p| (p.floor as usize, p.block as usize))
                    .or_else(|| self.item_spot(w, &wants).map(|(f, i, _)| (f, i)))
                    .or_else(|| arena_door(w, &wants));
                if std::env::var_os("PINEY_DEBUG_PILOT").is_some() && f.is_multiple_of(500) {
                    eprintln!("GOAL {f} {to:?} at {:?} wants {wants:?}", (sc.floor, sc.block));
                }
                // A marker wanted in this room comes before the walker,
                // who stands still once in its goal room.
                if let Some(raw) = self.walk_to_marker(a, &wants, f) {
                    return raw;
                }
                if let Some(raw) = self.open_item(a, &wants, f) {
                    return raw;
                }
                if let Some(raw) = self.open_point_statue(a, &wants, f) {
                    return raw;
                }
                if let Some(to) = to
                    && let Some(raw) = self.walker.step(s, to, f)
                {
                    return raw;
                }
            }
            if sc.area == kind::FIELD
                && wants.iter().any(|w| matches!(*w, Want::Dungeon(f, _) if f == sc.field as i16))
                && let Some(raw) = self.walk_into_dungeon(a, f)
            {
                return raw;
            }
            if sc.area == kind::FIELD
                && wants
                    .iter()
                    .any(|w| matches!(*w, Want::FieldBlock(f, b) if f == sc.field as i16 && i32::from(b) != sc.block))
                && let Some(raw) = walk_to_door(a)
            {
                return raw;
            }
            if sc.area == kind::FIELD
                && let Some(raw) = self.walk_to_marker(a, &wants, f)
            {
                return raw;
            }
        }
        if let Stage::World(w) = &s.stage {
            self.note_town_talk(w, f);
        }
        if let Stage::World(w) = &s.stage
            && !w.streaming()
            && let Some(raw) = self.walk_in_town(w, f)
        {
            return raw;
        }
        if let Stage::Area(a) = &s.stage
            && !a.streaming()
            && let Some(raw) = self.gate_out(a, f)
        {
            return raw;
        }
        if !matches!(&s.stage, Stage::Area(_)) {
            (self.idle_since, self.gating) = (None, false);
        }
        if !matches!(&s.stage, Stage::Area(_) | Stage::World(_)) {
            self.path.clear();
            self.mark = None;
        }
        story_player_with(s, f, &self.town_talked, &self.buys)
    }

    /// With an important item wanted ([`Want::Item`]): where to take it in
    /// this dungeon, not yet found done. First a box of the story rows
    /// holding it (`GIMMICKDATA` type 0, `flag` its code `0x45_0000 | id`:
    /// area 31's cures, one a floor), at its x, y; else a room whose row
    /// gives its statue an item (`ROOMDATA.itemID`); else the statue room.
    fn item_spot(
        &self,
        w: &piney_world::field_world::FieldWorld,
        wants: &[Want],
    ) -> Option<(usize, usize, Option<[f32; 2]>)> {
        let ids: Vec<i16> = wants
            .iter()
            .filter_map(|x| match *x {
                Want::Item(id) => Some(id),
                _ => None,
            })
            .collect();
        if ids.is_empty() {
            return None;
        }
        let Place::Dungeon(d) = w.place() else { return None };
        let sc = w.scene();
        let fresh = |f: i32, i: i32| !self.statues_done.contains(&(sc.field, sc.dungeon, f, i));
        // On the way down: the nearest floor from this one first (the walker
        // does not climb), then any.
        let boxed = d.edit.and_then(|e| {
            e.gimmicks
                .iter()
                .filter(|g| g.gim_type == 0 && ids.iter().any(|&id| g.flag == 0x45_0000 | i32::from(id)))
                .filter(|g| g.floor >= 0 && g.index >= 0 && fresh(g.floor, g.index))
                .min_by_key(|g| (g.floor < sc.floor, g.floor))
                .map(|g| (g.floor as usize, g.index as usize, Some([g.x as f32, g.y as f32])))
        });
        let row = || {
            d.edit.and_then(|e| {
                e.rooms
                    .iter()
                    .filter(|r| r.item != 0 && r.floor >= 0 && r.index >= 0)
                    .find(|r| fresh(r.floor, r.index))
                    .map(|r| (r.floor as usize, r.index as usize, None))
            })
        };
        let statue = || d.statue_room().filter(|&(f, i)| fresh(f as i32, i as i32)).map(|(f, i)| (f, i, None));
        boxed.or_else(row).or_else(statue)
    }

    /// In [`Self::item_spot`]'s room: to the box (or statue) and OK once it
    /// is the command target (the box's and the idol's menus are
    /// [`story_player`]'s). With none left to open, the room is noted done.
    fn open_item(&mut self, a: &crate::area::AreaMode, wants: &[Want], f: u64) -> Option<Raw> {
        let w = a.world();
        let sc = w.scene();
        let (fl, room, at) = self.item_spot(w, wants)?;
        if (fl, room) != (sc.floor as usize, sc.block as usize) || a.ui().menu_type() != -1 {
            return None;
        }
        let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 12);
        if !playing || w.combat().battle.in_battle != 0 {
            return None;
        }
        let target = match at {
            Some(p) => w.unopened_gimmick_near(p),
            None => w.unopened_statue(),
        };
        if std::env::var_os("PINEY_DEBUG_PILOT").is_some() && f.is_multiple_of(500) {
            let c = w.combat();
            let gims: Vec<_> = c
                .ctrl
                .list(piney_battle::entry::Kind::Gimmick)
                .into_iter()
                .map(|i| {
                    let o = c.ctrl.entry_obj(i).map(|o| (o.obj_flag, o.cmnd_flag, o.gim_id));
                    (i, o, c.scene.chars[i].pos.map(|v| f32::from_bits(v) as i32))
                })
                .collect();
            eprintln!(
                "ITEM {f} at {at:?} target {target:?} command {:?} kite {:?} gimmicks {gims:?}",
                w.command_target_code(),
                w.player().body.pos.map(|v| f32::from_bits(v) as i32)
            );
        }
        let key = (sc.field, sc.dungeon, sc.floor, sc.block);
        let Some(i) = target else {
            // Done only after a while: the room's boxes switch on with it.
            match self.nothing_since {
                Some((k, since)) if k == key && f > since + 300 => self.statues_done.push(key),
                Some((k, _)) if k == key => {}
                _ => self.nothing_since = Some((key, f)),
            }
            return None;
        };
        self.nothing_since = None;
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        if w.command_target_code() == Some((piney_world::entry::Kind::Gimmick, i as i32)) {
            return Some(if f.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        }
        let q = w.combat().scene.chars[i].pos.map(f32::from_bits);
        let p = w.player().body.pos.map(f32::from_bits);
        let cam_z = f32::from_bits(w.camera().rot()[2]);
        Some(stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1]))))
    }

    /// In the room of a wanted event point, no foe about, its Gott statue
    /// still shut after 300 frames there: the statue opened, as
    /// [`Self::open_item`] goes. A block that waits on `no_active` waits on
    /// a switched-on statue too (`no_active_object`): MOON-2's (261) point
    /// 2 gives the Moon Knife it then takes.
    fn open_point_statue(&mut self, a: &crate::area::AreaMode, wants: &[Want], f: u64) -> Option<Raw> {
        let w = a.world();
        let sc = w.scene();
        let vm = a.vm()?;
        let at_point = wants.iter().any(|&x| {
            matches!(x, Want::Point(n) if vm.mng.point(n).is_some_and(|p| (i32::from(p.floor), i32::from(p.block)) == (sc.floor, sc.block)))
        });
        let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 12);
        let quiet = playing && a.ui().menu_type() == -1 && w.combat().battle.in_battle == 0;
        let key = (sc.field, sc.dungeon, sc.floor, sc.block);
        if !at_point || !quiet {
            return None;
        }
        let since = match self.point_since {
            Some((k, since)) if k == key => since,
            _ => {
                self.point_since = Some((key, f));
                f
            }
        };
        let i = w.unopened_statue().filter(|_| f >= since + 300)?;
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        if w.command_target_code() == Some((piney_world::entry::Kind::Gimmick, i as i32)) {
            return Some(if f.is_multiple_of(8) { Raw { buttons: Buttons::CROSS, ..still } } else { still });
        }
        let q = w.combat().scene.chars[i].pos.map(f32::from_bits);
        let p = w.player().body.pos.map(f32::from_bits);
        let cam_z = f32::from_bits(w.camera().rot()[2]);
        Some(stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1]))))
    }

    /// Out of a field or dungeon the story wants nothing of (none of its
    /// wants names it; no event NPC waits, no fight, no event playing) for
    /// 300 frames: PERSONAL (menu 1, or 2 in a dungeon), Gate Out (menu
    /// 10), YES, back to town.
    fn gate_out(&mut self, a: &crate::area::AreaMode, f: u64) -> Option<Raw> {
        let w = a.world();
        let sc = w.scene();
        let vm = a.vm()?;
        let wants = story_wants(vm, &w.state().save);
        let here = wants.iter().any(|x| match *x {
            Want::Area(fd) | Want::Dungeon(fd, _) => i32::from(fd) == sc.field,
            Want::Point(_) => sc.area == kind::DUNGEON,
            _ => false,
        });
        let banned = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
            "menu_ban true" => Some(true),
            "menu_ban false" => Some(false),
            _ => None,
        }) == Some(true);
        let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 12);
        let busy = !playing
            || banned
            || w.combat().battle.in_battle != 0
            || !w.event_targets().is_empty()
            || vm.playing().is_some();
        let here = here && !self.leaving;
        if here || (busy && !self.gating) {
            (self.idle_since, self.gating) = (None, false);
            return None;
        }
        let since = *self.idle_since.get_or_insert(f);
        if f < since + 300 && !self.leaving {
            return None;
        }
        self.gating = true;
        let ui = a.ui();
        let m = &ui.ctrl;
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        let press = |b: Buttons| if f.is_multiple_of(8) { Raw { buttons: b, ..still } } else { still };
        let go_to = |row: i16| match m.list().select {
            r if r == row => Buttons::CROSS,
            r if r < row => Buttons::DOWN,
            _ => Buttons::UP,
        };
        Some(match (ui.menu_type(), m.proccess) {
            (-1, _) => press(Buttons::TRIANGLE),
            // A menu changing to the next (`MENU_CHANGING`): wait.
            (88, _) => still,
            // PERSONAL: menu 1 in a field, 2 in a dungeon (0 in a town).
            (1 | 2, 1) => {
                let l = m.list();
                match l.items.iter().take(l.y.max(0) as usize).position(|&it| it == 10) {
                    Some(r) => press(go_to(r as i16)),
                    None => press(Buttons::CIRCLE),
                }
            }
            (10, 1) => press(go_to(0)),
            (10, _) => press(Buttons::CROSS),
            _ => press(Buttons::CIRCLE),
        })
    }

    /// A town's event NPC spoken to: once the talk the pilot started bans
    /// the menus (its event block plays), he is passed by after, as the
    /// `add_target`s of repeatable lines stay on the list (event 108's six).
    fn note_town_talk(&mut self, w: &crate::world::WorldMode, f: u64) {
        let town = w.town_number();
        if self.talk_town != Some(town) {
            (self.town_talked, self.town_talking, self.talk_town) = (Vec::new(), None, Some(town));
        }
        let banned = w.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
            "menu_ban true" => Some(true),
            "menu_ban false" => Some(false),
            _ => None,
        }) == Some(true);
        // The talk's block banned the menus, or the NPC's own menu opened
        // (a shop, a breeder: menus 21-27, 44-46).
        let own_menu = matches!(w.ui().menu_type(), 21..=27 | 44..=46);
        match self.town_talking {
            Some((who, _)) if banned || own_menu => {
                self.town_talked.push(who);
                self.town_talking = None;
            }
            Some((_, at)) if f > at + 120 => self.town_talking = None,
            Some(_) => {}
            None => {
                if let Some(GateGoal::Talk(kind, code)) = gate_goal(w, &self.town_talked, &self.buys)
                    && w.world().command_target() == Some((kind, code))
                {
                    self.town_talking = Some(((kind, code), f));
                }
            }
        }
    }

    /// In a town, the way round the walls to whom [`gate_goal`] walks to
    /// (an event's NPC, else the Chaos Gate), planned again when Kite
    /// stops; within 400 of him, or with a menu or window up, None (the
    /// straight walk and the talk of [`story_player`]).
    fn walk_in_town(&mut self, w: &crate::world::WorldMode, f: u64) -> Option<Raw> {
        use piney_world::entry::Kind;
        let world = w.world();
        let marker = |m: i16| world.marker(m).map(|mk| mk.pos);
        let (target, to) = match gate_goal(w, &self.town_talked, &self.buys)? {
            GateGoal::Talk(kind, code) => ((kind, code), None),
            GateGoal::Shop(code) => ((Kind::Npc, code), None),
            GateGoal::Marker(m) => ((Kind::Gimmick, -1), Some(marker(m)?)),
            GateGoal::Area(_) | GateGoal::Town(_) => ((Kind::Gimmick, 16), None),
            GateGoal::LogOut | GateGoal::Invite(_) | GateGoal::Disband => return None,
        };
        let waiting = w.calls().iter().rev().find_map(|(_, c)| {
            if c.starts_with("message_open") || c.starts_with("announce") || c.starts_with("info_lines") {
                Some(true)
            } else if c.starts_with("message_check") || c.starts_with("message_close") {
                Some(false)
            } else {
                None
            }
        });
        if w.ui().menu_type() != -1 || waiting == Some(true) || world.command_target() == Some(target) {
            self.path.clear();
            return None;
        }
        let to = match to {
            Some(q) => q,
            None => world.char_place(target.0, target.1)?.0.map(f32::from_bits),
        };
        let p = world.player().body.pos.map(f32::from_bits);
        if (to[0] - p[0]).hypot(to[1] - p[1]) < 400.0 {
            self.path.clear();
            return None;
        }
        if f.is_multiple_of(60) {
            let stopped = self.mark.is_some_and(|m| (m[0] - p[0]).hypot(m[1] - p[1]) < 60.0);
            if stopped || self.path.is_empty() {
                let mid = [(p[0] + to[0]) / 2.0, (p[1] + to[1]) / 2.0];
                let span = (to[0] - p[0]).abs().max((to[1] - p[1]).abs());
                let n = ((span + 4000.0) / 150.0) as i32 | 1;
                let near = |q: [f32; 2], _| (q[0] - to[0]).hypot(q[1] - to[1]) < 300.0;
                self.path = path_to(&world.town().base.hits, [p[0], p[1], p[2]], mid, n, 150.0, near);
            }
            self.mark = Some([p[0], p[1]]);
        }
        while self.path.len() > 1 && (self.path[0][0] - p[0]).hypot(self.path[0][1] - p[1]) < 150.0 {
            self.path.remove(0);
        }
        let q = self.path.first().copied()?;
        let cam_z = f32::from_bits(world.camera().rot()[2]);
        Some(stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1]))))
    }

    /// In a field, an event NPC the events wait to be spoken to
    /// (`add_target`) and not yet spoken to here: the way to him round
    /// the walls, and OK once he is the command target. A talk counts once
    /// the event it starts bans the menus; else it is tried again.
    fn talk_in_field(&mut self, a: &crate::area::AreaMode, f: u64) -> Option<Raw> {
        let w = a.world();
        let sc = w.scene();
        if self.talk_place != Some((sc.area, sc.field)) {
            (self.talked, self.talking, self.talk_place) = (Vec::new(), None, Some((sc.area, sc.field)));
        }
        let banned = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
            "menu_ban true" => Some(true),
            "menu_ban false" => Some(false),
            _ => None,
        }) == Some(true);
        if let Some((code, at)) = self.talking {
            if banned {
                self.talked.push(code);
                self.talking = None;
            } else if f > at + 120 {
                self.talking = None;
            }
        }
        let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 12);
        if !playing || banned || a.ui().menu_type() != -1 || self.talking.is_some() {
            return None;
        }
        let code = w
            .event_targets()
            .iter()
            .filter(|&&(t, _)| matches!(t, 3 | 4))
            .map(|&(_, c)| i32::from(c))
            .find(|c| !self.talked.contains(c))?;
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        // An event NPC's stand-in is on the entry control's object list:
        // the command target names it by its scene index.
        let aimed = match w.command_target_code() {
            Some((piney_world::entry::Kind::Npc, c)) => Some(c),
            Some((piney_world::entry::Kind::Gimmick, i)) => {
                w.combat().npcs.iter().find(|n| n.who == i as usize).map(|n| i32::from(n.code))
            }
            _ => None,
        };
        if aimed == Some(code) {
            if f.is_multiple_of(8) {
                self.talking = Some((code, f));
                return Some(Raw { buttons: Buttons::CROSS, ..still });
            }
            return Some(still);
        }
        let to = w.char_pos(3, code as i16)?.map(f32::from_bits);
        let p = w.player().body.pos.map(f32::from_bits);
        let cam_z = f32::from_bits(w.camera().rot()[2]);
        if (to[0] - p[0]).hypot(to[1] - p[1]) < 400.0 {
            self.path.clear();
            return Some(stick_toward(cam_z, (to[0] - p[0]).atan2(-(to[1] - p[1]))));
        }
        if f.is_multiple_of(60) {
            let stopped = self.mark.is_some_and(|m| (m[0] - p[0]).hypot(m[1] - p[1]) < 60.0);
            if stopped || self.path.is_empty() {
                let mid = [(p[0] + to[0]) / 2.0, (p[1] + to[1]) / 2.0];
                let span = (to[0] - p[0]).abs().max((to[1] - p[1]).abs());
                let n = ((span + 4000.0) / 150.0) as i32 | 1;
                let near = |q: [f32; 2], _| (q[0] - to[0]).hypot(q[1] - to[1]) < 300.0;
                self.path = path_to(w.place_hits(), [p[0], p[1], p[2]], mid, n, 150.0, near);
            }
            self.mark = Some([p[0], p[1]]);
        }
        while self.path.len() > 1 && (self.path[0][0] - p[0]).hypot(self.path[0][1] - p[1]) < 150.0 {
            self.path.remove(0);
        }
        let q = self.path.first().copied().unwrap_or([to[0], to[1]]);
        Some(stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1]))))
    }

    /// The fight through the menus, one action at a time and a second
    /// apart: at a fight's start Skills! to both members; a member down,
    /// a member standing with a revive for him (Designate Skill); a member
    /// under 45% of his HP, a member's heal for him, else First Aid!; else
    /// in a fight, Kite's first attack skill he has the SP for.
    fn fight(&mut self, a: &crate::area::AreaMode, f: u64) -> Option<Raw> {
        self.now = f;
        self.held = self.walker.held(a, f);
        let w = a.world();
        let ui = a.ui();
        let m = &ui.ctrl;
        let t = ui.menu_type();
        let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
        let press = |b: Buttons| if f.is_multiple_of(8) { Raw { buttons: b, ..still } } else { still };
        let go_to = |row: i16| match m.list().select {
            s if s == row => Buttons::CROSS,
            s if s < row => Buttons::DOWN,
            _ => Buttons::UP,
        };
        let go_to_item = |item: i16| {
            let l = m.list();
            match l.items.iter().take(l.y.max(0) as usize).position(|&it| it == item) {
                Some(r) => go_to(r as i16),
                None => Buttons::NONE,
            }
        };
        let c = w.combat();
        let fighting = c.battle.in_battle != 0;
        if !fighting {
            self.skills_ordered = None;
        }
        if let Some(act) = self.action {
            self.opened |= t != -1;
            if (self.opened && t == -1) || f > self.began + 600 {
                self.action = None;
                // A skill takes its time to land.
                self.ended = Some(if matches!(act, Action::Chat { .. }) { f } else { f + 180 });
                return None;
            }
            // A skill at a boss's part: the cursor down the candidates
            // until the command target is [`focus`]'s (a few rounds at most).
            let aim = (t == 65 && m.proccess == 1 && matches!(act, Action::Skill { target: None, .. }))
                || (t == 73 && m.proccess != 10 && matches!(act, Action::Designate { page: 1, .. }));
            if aim && let Some(part) = focus(c) {
                let on = w.targeting().target == Some((piney_world::entry::Kind::Enemy, part as i32));
                if !on && self.target_tries < 24 {
                    if f.is_multiple_of(8) {
                        self.target_tries += 1;
                    }
                    return Some(press(Buttons::DOWN));
                }
                if on && t == 73 {
                    return Some(press(Buttons::CROSS));
                }
            }
            // The target menu's "no target" (none in the skill's reach):
            // the action given up; the menus are backed out below.
            if t == 65 && m.proccess == 2 && matches!(act, Action::Skill { .. }) {
                self.action = None;
                self.ended = Some(f);
                return Some(press(Buttons::CROSS));
            }
            let party = w.party();
            return Some(match (t, act) {
                (-1, Action::Skill { .. } | Action::Item { .. }) => press(Buttons::TRIANGLE),
                (-1, _) => press(Buttons::SQUARE),
                (3, Action::Chat { page, .. }) if m.list().page != page => {
                    press(if m.list().page < page { Buttons::RIGHT } else { Buttons::LEFT })
                }
                (3, Action::Chat { row, .. }) => press(go_to(row)),
                (3, Action::Designate { .. }) if m.list().page != 2 => press(Buttons::RIGHT),
                (3, Action::Designate { slot, .. }) => press(go_to(slot as i16 - 1)),
                (71, _) => press(go_to(0)),
                (72, Action::Designate { page, .. }) if m.list().page != page => {
                    press(if m.list().page < page { Buttons::RIGHT } else { Buttons::LEFT })
                }
                (72, Action::Designate { slot, page, skill, .. }) => {
                    let list = piney_fieldui::items::skill_list(
                        &ui.texts().items,
                        w.state(),
                        party[slot] as usize,
                        i32::from(page),
                    );
                    press(go_to(list.iter().position(|&x| x == skill).unwrap_or(0) as i16))
                }
                // The target is not among the menu's (his condition
                // other than normal): back out.
                (73, _) if m.proccess == 10 => press(Buttons::CROSS),
                (73, Action::Designate { target, .. }) if target >= m.list().my => press(Buttons::CIRCLE),
                (73, Action::Designate { target, .. }) => press(go_to(target)),
                // "Cannot be used while dead."
                (0..=2, _) if (2..=3).contains(&m.proccess) => press(Buttons::CROSS),
                // A skill refused (SP short, Data Drain barred): its window.
                (4, _) if (2..=3).contains(&m.proccess) => press(Buttons::CROSS),
                (0..=2, Action::Skill { .. }) => press(go_to_item(4)),
                (0..=2, Action::Item { .. }) => press(go_to_item(5)),
                (4, Action::Skill { page, .. }) if m.list().page != page => {
                    press(if m.list().page < page { Buttons::RIGHT } else { Buttons::LEFT })
                }
                (4, Action::Skill { page, skill, .. }) => {
                    let list = piney_fieldui::items::skill_list(&ui.texts().items, w.state(), 0, i32::from(page));
                    press(go_to(list.iter().position(|&x| x == skill).unwrap_or(0) as i16))
                }
                (5, Action::Item { cat, .. }) if m.list().page != item_page(&ui.texts().items, cat) => {
                    press(if m.list().page < item_page(&ui.texts().items, cat) {
                        Buttons::RIGHT
                    } else {
                        Buttons::LEFT
                    })
                }
                (5, Action::Item { cat, id, .. }) => {
                    let page = i32::from(item_page(&ui.texts().items, cat));
                    let list = piney_fieldui::items::item_list(&ui.texts().items, w.state(), 0, page);
                    let row = list.iter().position(|it| (it.cat, it.id) == (cat, id));
                    press(row.map_or(Buttons::CIRCLE, |r| go_to(r as i16)))
                }
                (65, _) if m.proccess == 2 => press(Buttons::CROSS),
                (65, Action::Skill { target: Some(r), .. } | Action::Item { target: r, .. }) => {
                    press(if r >= m.list().my { Buttons::CIRCLE } else { go_to(r) })
                }
                (65, _) => press(Buttons::CROSS),
                // Data Drain's own menu: its movie, messages and drops take OK.
                (66, _) => press(Buttons::CROSS),
                // Anything else the action opened: back out.
                _ => press(Buttons::CIRCLE),
            });
        }
        // A menu of the last action's still up (its target gone): out.
        // (The target menu's message, no target left, takes OK.)
        // (In a field or dungeon only the pilot opens these.)
        if !self.gating && [0, 1, 2, 3, 4, 5, 65, 66, 71, 72, 73].contains(&t) {
            let message = t == 66
                || (t == 65 && m.proccess == 2)
                || (t == 73 && m.proccess == 10)
                || (t == 71 && (10..=11).contains(&m.proccess))
                || ((0..=2).contains(&t) && (2..=3).contains(&m.proccess))
                || (t == 4 && (2..=3).contains(&m.proccess));
            return Some(press(if message { Buttons::CROSS } else { Buttons::CIRCLE }));
        }
        // A broken protect does not wait for the last skill to land: the
        // break is short (Fidchell's under 300 frames).
        let broken = drain_candidates(c).into_iter().any(|e| drainable(&c.scene.chars[e], false));
        if t != -1 || (!broken && self.ended.is_some_and(|at| f < at + 60)) {
            return None;
        }
        let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 12);
        let banned = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
            "menu_ban true" => Some(true),
            "menu_ban false" => Some(false),
            _ => None,
        });
        if !playing || banned == Some(true) {
            return None;
        }
        let haste = haste_for_gold(a);
        if haste.is_none()
            && let Some(raw) = approach_boss(a)
                .or_else(|| approach_part(a))
                .or_else(|| approach_roamer(a))
                .or_else(|| self.approach_bug(a, f))
                .or_else(|| self.approach_foe(a, f))
        {
            return Some(raw);
        }
        // No skill to pay for (god's SP hid it): the action button's plain
        // blow at the foe the command target names.
        let Some(act) = haste.or_else(|| self.choose(a)) else {
            let on_foe = matches!(w.command_target_code(), Some((piney_world::entry::Kind::Enemy, _)));
            // A golden goblin is in reach for a frame or two: at once.
            let gold = self.quarry.is_some_and(|q| shakes_off_holds(c, q) && c.scene.chars[q].hp > 0);
            let every = if gold { 2 } else { 8 };
            let blow = if f.is_multiple_of(every) { Raw { buttons: Buttons::CROSS, ..still } } else { still };
            return (fighting && on_foe).then_some(blow);
        };
        if std::env::var_os("PINEY_DEBUG_PILOT").is_some() {
            eprintln!("ACT {f} {act:?}");
        }
        match act {
            Action::Chat { page: 0, row: r @ (0 | 6) } => self.skills_ordered = Some(r == 6),
            Action::Chat { page: 0, row: 1 } => self.first_aid_at = Some(f),
            _ => {}
        }
        self.action = Some(act);
        self.opened = false;
        self.began = f;
        self.target_tries = 0;
        Some(still)
    }

    /// The next action: at a fight's start the order to both members
    /// (Magic! against a foe whose Exdefense bars blows or whose physical
    /// defence is the higher, else Skills!); someone down, a member's Rip
    /// Maen, else Kite's Resurrect; someone under 45% of his HP (70% out of
    /// a fight), a member's heal, else Kite's Repth, a Healing Potion, First
    /// Aid!; out of a fight, someone under a third of his SP, a Mage's Soul;
    /// in a fight, Kite's strongest skill in reach of the nearest foe that he
    /// has the SP for and that its Exdefense does not bar.
    fn choose(&self, a: &crate::area::AreaMode) -> Option<Action> {
        let w = a.world();
        let ui = a.ui();
        let c = w.combat();
        let fighting = c.battle.in_battle != 0;
        let party = w.party();
        let chars: Vec<Option<&piney_battle::chara::Char>> =
            party.iter().map(|&id| if id < 0 { None } else { c.who(id).map(|k| &c.scene.chars[k]) }).collect();
        let members = chars.iter().skip(1).filter(|ch| ch.is_some()).count();
        let kite_at = chars.first().copied().flatten().map(|k| k.pos.map(f32::from_bits));
        let foe = focus(c).or_else(|| {
            foes(c).into_iter().filter(|&e| c.scene.chars[e].hp > 0).min_by(|&x, &y| {
                let d = |e: usize| {
                    let q = c.scene.chars[e].pos.map(f32::from_bits);
                    kite_at.map_or(0.0, |p| (q[0] - p[0]).hypot(q[1] - p[1]))
                };
                d(x).total_cmp(&d(y))
            })
        });
        // What a sure hit of `sk` from Kite does to foe `e` (`CalcBattleDamage`
        // at 100, the foe untouched): 0 while its Exdefense holds against the
        // skill (Gaia Turtle's physical, Black Death's magic).
        let t = &c.data.t;
        let sure = |e: usize, sk: &piney_battle::param::SkillParam| {
            let Some(k) = chars.first().copied().flatten() else { return -1 };
            let mut tgt = c.scene.chars[e].clone();
            let (mut none, env) = (|| 0, piney_battle::chara::Env::default());
            let roll = piney_battle::damage::Roll::Quiet(100);
            let one = piney_battle::damage::F_ONE;
            piney_battle::damage::calc_battle_damage(
                t,
                k,
                &mut tgt,
                sk,
                one,
                roll,
                true,
                &mut none,
                &env,
                &mut Vec::new(),
            )
            .dmg
        };
        let hurts = |sid: i32| t.skill(sid).is_none_or(|sk| foe.is_none_or(|e| sure(e, sk) != 0));
        // Foe `e` barred by its Exdefense from blows (kind 1) or spells (2).
        let bars = |e: usize, kind: i32| {
            t.skill(1).is_some_and(|sk| sure(e, &piney_battle::param::SkillParam { ty: kind, ..sk.clone() }) == 0)
        };
        let barred = |kind: i32| foe.is_some_and(|e| bars(e, kind));
        let by_defence = foe.is_some_and(|e| match &c.scene.chars[e].body {
            piney_battle::chara::Body::Foe(fo) => {
                use piney_battle::boss::Class;
                use piney_battle::param::elm;
                // Gorre's brothers share one protect gauge, which each
                // fills by the lower of its two PP defences (9990 against
                // the other kind): the kind that breaks it.
                if matches!(fo.boss.as_ref().map(|b| &b.class), Some(Class::GorreBrother(_))) {
                    fo.row.p_def_pp > fo.row.m_def_pp
                } else {
                    fo.real[elm::P_DEF] > fo.real[elm::M_DEF]
                }
            }
            _ => false,
        });
        let magic = match (barred(1), barred(2)) {
            (true, false) => true,
            (false, true) => false,
            _ => by_defence,
        };
        // Again when the boss part fought needs the other kind.
        let reorder = focus(c).is_some() && self.skills_ordered != Some(magic);
        if fighting && members > 0 && (self.skills_ordered.is_none() || reorder) {
            return Some(Action::Chat { page: 0, row: if magic { 6 } else { 0 } });
        }
        // A boss or a Data Bug with its protect broken (`pp_count` frames
        // left): Kite's Data Drain (Skills, page 5, skill 2) before the
        // break mends.
        let broken = drain_candidates(c).into_iter().any(|e| drainable(&c.scene.chars[e], self.held));
        let drains = piney_fieldui::items::skill_list(&ui.texts().items, w.state(), 0, 5)[0] == DATA_DRAIN;
        let items = &ui.texts().items;
        let state = w.state();
        let carried = |(cat, id): (i8, i16)| {
            (0..piney_fieldui::items::ITEMS)
                .map(|k| piney_fieldui::items::save_item(state, 0, k))
                .any(|it| (it.cat, it.id) == (cat, id) && it.num > 0)
        };
        if fighting
            && broken
            && drains
            && let Some(k) = chars.first().copied().flatten().filter(|k| k.hp > 0)
        {
            // Short of its SP (the menu refuses it): a Mage's Soul first.
            let cost = items.skill(i32::from(DATA_DRAIN)).map_or(0, |p| p.cost);
            if i32::from(k.sp) >= cost {
                return Some(Action::Skill { page: 5, skill: DATA_DRAIN, target: None });
            }
            if carried(MAGES_SOUL) {
                return Some(Action::Item { cat: MAGES_SOUL.0, id: MAGES_SOUL.1, target: 0 });
            }
        }
        let down = |ch: &piney_battle::chara::Char| ch.hp <= 0;
        let limit = if fighting { 45 } else { 70 };
        let low = |ch: &piney_battle::chara::Char| {
            ch.hp > 0 && ch.cond.v[0] == 0 && i32::from(ch.hp) * 100 < i32::from(ch.max_hp) * limit
        };
        // The menus' target rows: the party in order, the fallen for a
        // revive, those in a normal condition else.
        let fallen = |ch: &piney_battle::chara::Char| ch.cond.v[0] != 0 && ch.cond.v[0] != 5;
        let row = |who: usize, revive: bool| {
            chars[..who]
                .iter()
                .filter(|ch| ch.is_some_and(|ch| if revive { fallen(ch) } else { ch.cond.v[0] == 0 }))
                .count() as i16
        };
        let kite = chars.first().copied().flatten().filter(|k| !down(k));
        // ChatMenu1 refuses an order to a member whose `condition[0]` is
        // not 0 (down, or still getting up from a revive: 5).
        let can_order = |ch: &&piney_battle::chara::Char| !down(ch) && ch.cond.v[0] == 0;
        let member_skill = |revive: bool| {
            (1..party.len()).find_map(|slot| {
                let ch = chars[slot].filter(can_order)?;
                let list = piney_fieldui::items::skill_list(items, state, party[slot] as usize, 2);
                let skill = list.iter().copied().filter(|&x| x >= 0).find(|&x| {
                    let cost = items.skill(i32::from(x)).map_or(i32::MAX, |p| p.cost);
                    (x == RIP_MAEN) == revive && !(178..180).contains(&x) && cost <= i32::from(ch.sp)
                })?;
                Some((slot, skill))
            })
        };
        if let Some(who) = chars.iter().position(|ch| ch.is_some_and(down)) {
            let target = row(who, true);
            if let Some((slot, skill)) = member_skill(true) {
                return Some(Action::Designate { slot, page: 2, skill, target });
            }
            if kite.is_some() && carried(RESURRECT) {
                return Some(Action::Item { cat: RESURRECT.0, id: RESURRECT.1, target });
            }
            // A member who knows Rip Maen short of its SP: a Mage's Soul.
            let cost = items.skill(i32::from(RIP_MAEN)).map_or(i32::MAX, |p| p.cost);
            let short = (1..party.len()).find(|&slot| {
                chars[slot].is_some_and(|ch| {
                    !down(ch)
                        && ch.cond.v[0] == 0
                        && i32::from(ch.sp) < cost
                        && piney_fieldui::items::skill_list(items, state, party[slot] as usize, 2).contains(&RIP_MAEN)
                })
            });
            if let Some(slot) = short
                && kite.is_some()
                && carried(MAGES_SOUL)
            {
                return Some(Action::Item { cat: MAGES_SOUL.0, id: MAGES_SOUL.1, target: row(slot, false) });
            }
            if fighting && members > 0 && self.first_aid_at.is_none_or(|at| self.now > at + 600) {
                return Some(Action::Chat { page: 0, row: 1 });
            }
        }
        let lowest = chars
            .iter()
            .enumerate()
            .filter(|(_, ch)| ch.is_some_and(low))
            .min_by_key(|(_, ch)| ch.map_or(i32::MAX, |ch| i32::from(ch.hp) * 100 / i32::from(ch.max_hp).max(1)))
            .map(|(k, _)| k);
        if let Some(who) = lowest {
            let target = row(who, false);
            if let Some((slot, skill)) = member_skill(false) {
                return Some(Action::Designate { slot, page: 2, skill, target });
            }
            if let Some(k) = kite {
                let knows = piney_fieldui::items::skill_list(items, state, 0, 2).contains(&REPTH);
                let cost = items.skill(i32::from(REPTH)).map_or(i32::MAX, |p| p.cost);
                if knows && i32::from(k.sp) >= cost {
                    return Some(Action::Skill { page: 2, skill: REPTH, target: Some(target) });
                }
                if carried(POTION) {
                    return Some(Action::Item { cat: POTION.0, id: POTION.1, target });
                }
            }
            if fighting && members > 0 && self.first_aid_at.is_none_or(|at| self.now > at + 600) {
                return Some(Action::Chat { page: 0, row: 1 });
            }
        }
        if !fighting {
            let dry = chars.iter().position(|ch| {
                ch.is_some_and(|ch| {
                    !down(ch) && ch.cond.v[0] == 0 && ch.max_sp > 0 && i32::from(ch.sp) * 3 < i32::from(ch.max_sp)
                })
            })?;
            return (kite.is_some() && carried(MAGES_SOUL)).then(|| Action::Item {
                cat: MAGES_SOUL.0,
                id: MAGES_SOUL.1,
                target: row(dry, false),
            });
        }
        // A member's spell of the element the foe resists least, if under
        // 100, at the nearest foe.
        let reach = |e: usize| {
            let q = c.scene.chars[e].pos.map(f32::from_bits);
            kite_at.is_some_and(|p| (q[0] - p[0]).hypot(q[1] - p[1]) < 1500.0)
        };
        if fighting
            && let Some(e) = foe.filter(|&e| reach(e))
            && let piney_battle::chara::Body::Foe(fo) = &c.scene.chars[e].body
        {
            let resist = |x: i16| {
                let kind = items.skill(i32::from(x)).map_or(0, |p| p.kind);
                (0..6).filter(|i| kind & (4 << i) != 0).map(|i| fo.real[8 + i]).min().unwrap_or(i16::MAX)
            };
            let best = (1..party.len())
                .filter_map(|slot| {
                    let ch = chars[slot].filter(can_order)?;
                    let list = piney_fieldui::items::skill_list(items, state, party[slot] as usize, 1);
                    let skill = list
                        .iter()
                        .copied()
                        .filter(|&x| x >= 0)
                        .filter(|&x| items.skill(i32::from(x)).is_some_and(|p| p.cost <= i32::from(ch.sp)))
                        .filter(|&x| hurts(i32::from(x)))
                        .min_by_key(|&x| resist(x))?;
                    Some((slot, skill, resist(skill)))
                })
                .min_by_key(|&(_, _, r)| r);
            // At a boss part that magic hurts (Kyvia's core resists no
            // element but guards against one kind), any spell.
            let at_part = magic && focus(c) == Some(e);
            if let Some((slot, skill, r)) = best
                && (r < 100 || at_part)
            {
                return Some(Action::Designate { slot, page: 1, skill, target: 0 });
            }
        }
        let kite = kite?;
        let p = kite.pos.map(f32::from_bits);
        let dist = |e: usize| {
            let q = c.scene.chars[e].pos.map(f32::from_bits);
            (q[0] - p[0]).hypot(q[1] - p[1])
        };
        // Not a foe the walk has given up on, unless it still goes at it
        // (the room's doors shut until it falls): its blows alone may not
        // do (Deadly Present's Exdefense bars them).
        let (near, nearest) = foes(c)
            .into_iter()
            .filter(|&e| (!self.walker.hopeless.contains(&e) || self.walker.foe == Some(e)) && c.scene.chars[e].hp > 0)
            .map(|e| (e, dist(e)))
            .min_by(|x, y| x.1.total_cmp(&y.1))
            .filter(|&(_, d)| d < 600.0)?;
        // A golden goblin that shakes off holds (see [`shakes_off_holds`]):
        // the plain blow.
        if shakes_off_holds(c, near) {
            return None;
        }
        // Of the skills whose reach (`triggerRange`) holds the nearest foe
        // (the target menu offers no one beyond it: Gale of Swords' 350),
        // the strongest (`atk` by `dmgRate`, over the foe's resistance to
        // its element), of the page the foe is weaker to, else of the other
        // (a spell from afar rather than a chase).
        let resist = |sk: &piney_battle::param::SkillParam| {
            let el = foe.and_then(|e| c.scene.chars[e].foe_state()).map(|fo| fo.real);
            (0..6)
                .filter(|&i| sk.attr[i] != 0)
                .map(|i| el.map_or(100, |r| i32::from(r[8 + i])))
                .min()
                .unwrap_or(100)
                .max(1)
        };
        let strength = |x: i16| {
            c.data.t.skill(i32::from(x)).map_or(0, |sk| i32::from(sk.atk) * i32::from(sk.dmg_rate) * 100 / resist(sk))
        };
        let pages = if magic { [1, 0] } else { [0, 1] };
        let (page, skill) = pages.into_iter().find_map(|page| {
            let reach = |x: &i16| {
                items.skill(i32::from(*x)).is_some_and(|sk| sk.cost <= i32::from(kite.sp) && nearest < sk.trigger_range)
            };
            let list = piney_fieldui::items::skill_list(items, state, 0, page);
            let usable = list.iter().copied().filter(|&x| x >= 0).filter(reach).filter(|&x| hurts(i32::from(x)));
            usable.max_by_key(|&x| strength(x)).map(|x| (page, x))
        })?;
        Some(Action::Skill { page: page as i16, skill, target: None })
    }

    /// In a field whose events wait for its foes all down ([`Want::Clear`]),
    /// the first time golden goblins that Kite's blows hurt are seen there:
    /// the Speed Charms (see [`haste_for_gold`]) for the blows their HP
    /// takes, at [`BLOWS_A_CHARM`] a charm, and one more. Short of them, he
    /// goes back to town to buy them (100 GP at any magic shop), as a
    /// player who has met them would; the event's repeatable `entry 5`
    /// block puts them back on his return. GOB3-4's (row 149, pDef 1950:
    /// 70-105 a blow) take some 50 blows: five charms; Kite has two.
    fn provision(&mut self, a: &crate::area::AreaMode) {
        let w = a.world();
        let sc = w.scene();
        if self.leaving || self.action.is_some() || sc.area != kind::FIELD || self.stocked.contains(&sc.field) {
            return;
        }
        let Some(vm) = a.vm() else { return };
        // Once the event's lines are over and Kite is free to act.
        let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 12);
        if !playing
            || vm.playing().is_some()
            || !story_wants(vm, &w.state().save).contains(&Want::Clear(sc.field as i16))
        {
            return;
        }
        let c = w.combat();
        let gold: Vec<usize> = c
            .enemies()
            .into_iter()
            .filter(|&e| c.scene.chars[e].hp > 0 && shakes_off_holds(c, e) && blows_land(c, e))
            .collect();
        if gold.is_empty() {
            return;
        }
        let blows: i32 = gold.iter().map(|&e| i32::from(c.scene.chars[e].hp) / kite_hit(c, e, 1, 100).max(1) + 1).sum();
        let charms = (blows + BLOWS_A_CHARM - 1) / BLOWS_A_CHARM + 1;
        self.stocked.push(sc.field);
        self.buys = vec![Buy { item: SPEED_CHARM, carry: charms }];
        let carried: i32 = (0..piney_fieldui::items::ITEMS)
            .map(|n| piney_fieldui::items::save_item(w.state(), 0, n))
            .filter(|it| (it.cat, it.id) == SPEED_CHARM)
            .map(|it| i32::from(it.num))
            .sum();
        self.leaving = carried < charms;
    }

    /// Kite put where the walk found him stuck.
    pub(super) fn after(&mut self, s: &mut Session) {
        self.walker.put(s);
    }

    /// In a field, a Data Bug on the lists farther than 350 from Kite, who
    /// knows Data Drain: a walk to it, fight or not (its HP stops at half;
    /// only a drain ends it), as [`Self::walk_after`] goes (event 206's, in
    /// field 13's story map, roams far from the party).
    fn approach_bug(&mut self, a: &crate::area::AreaMode, f: u64) -> Option<Raw> {
        let w = a.world();
        let c = w.combat();
        if piney_fieldui::items::skill_list(&a.ui().texts().items, w.state(), 0, 5)[0] != DATA_DRAIN {
            return None;
        }
        let bug = c.enemies().into_iter().find(|&e| c.scene.chars[e].hp > 0 && c.scene.chars[e].ty() & DATA_BUG != 0);
        let q = bug.map(|e| c.scene.chars[e].pos.map(f32::from_bits))?;
        self.walk_after(w, q, 350.0, f, false)
    }

    /// In a field whose events wait for its foes all down ([`Want::Clear`]),
    /// no live Data Bug ([`Self::approach_bug`]'s: 206's field 13) and no
    /// foe within 300: a walk to the live foe with the least HP (the nearest
    /// of those), as [`Self::walk_after`] goes. Side event 251's three
    /// golden goblins run from Kite and heal: one at a time.
    fn approach_foe(&mut self, a: &crate::area::AreaMode, f: u64) -> Option<Raw> {
        let w = a.world();
        let c = w.combat();
        let sc = w.scene();
        let wants = story_wants(a.vm()?, &w.state().save);
        if sc.area != kind::FIELD || !wants.contains(&Want::Clear(sc.field as i16)) {
            return None;
        }
        let p = w.player().body.pos.map(f32::from_bits);
        let at = |e: usize| c.scene.chars[e].pos.map(f32::from_bits);
        let dist = |e: usize| (at(e)[0] - p[0]).hypot(at(e)[1] - p[1]);
        let live: Vec<usize> = c.enemies().into_iter().filter(|&e| c.scene.chars[e].hp > 0).collect();
        // Up to a golden goblin that runs: within the command target's
        // reach (150 and both widths in a fight, `ccCheckTargetRange`).
        let close = |e: usize| if shakes_off_holds(c, e) { 150.0 } else { 300.0 };
        if live.iter().any(|&e| c.scene.chars[e].ty() & DATA_BUG != 0 || dist(e) < close(e)) {
            return None;
        }
        // Golden goblins (see [`shakes_off_holds`]): one casting its heal
        // first (see [`casting_gold`]), else the one run after while it
        // stands, else the nearest that stands (the weakest may stand far
        // off, and flee before Kite gets there). On the run one outpaces
        // Kite (see [`haste_for_gold`]); it stops once he is past
        // `atkRangeB`, and from a stand it gathers speed slowly
        // (`goldAccel` is `crisisRate` squared): rushed from there.
        let act = |e: usize| c.foes.get(e).and_then(Option::as_ref).map_or(-1, |f| f.act_num);
        let standing = |e: &usize| shakes_off_holds(c, *e) && !matches!(act(*e), enemy_act::CLOSE | enemy_act::DAMAGE);
        let weakest =
            |x: &usize, y: &usize| c.scene.chars[*x].hp.cmp(&c.scene.chars[*y].hp).then(dist(*x).total_cmp(&dist(*y)));
        let casting = live.iter().copied().filter(|&e| casting_gold(c, e)).min_by(|&x, &y| dist(x).total_cmp(&dist(y)));
        let foe = casting
            .or_else(|| self.quarry.filter(|q| live.contains(q) && standing(q)))
            .or_else(|| live.iter().copied().filter(standing).min_by(|&x, &y| dist(x).total_cmp(&dist(y))))
            .or_else(|| live.iter().copied().min_by(weakest))?;
        self.quarry = shakes_off_holds(c, foe).then_some(foe);
        if self.quarry.is_some() && !standing(&foe) {
            self.rush_mark = None;
            return Some(Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() });
        }
        if self.quarry.is_some() {
            // Straight at it; a rush that a wall stopped (under 60 in 60
            // frames) goes round it on [`Self::walk_after`]'s way a while.
            let stuck = match self.rush_mark {
                Some((at, m)) if f >= at + 60 => {
                    self.rush_mark = Some((f, [p[0], p[1]]));
                    (m[0] - p[0]).hypot(m[1] - p[1]) < 60.0
                }
                Some(_) => false,
                None => {
                    self.rush_mark = Some((f, [p[0], p[1]]));
                    false
                }
            };
            if stuck {
                self.detour_until = f + 180;
            }
            if f < self.detour_until {
                return self.walk_after(w, at(foe), close(foe), f, true);
            }
            let (q, cam_z) = (at(foe), f32::from_bits(w.camera().rot()[2]));
            return Some(run_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1]))));
        }
        self.walk_after(w, at(foe), close(foe), f, false)
    }

    /// Kite to the event position a wanted block waits for him near
    /// ([`Want::Marker`]), in this field or this room of the dungeon, while
    /// nothing holds him: straight at it across a room, the field's way
    /// round its walls as [`Self::walk_after`]'s.
    fn walk_to_marker(&mut self, a: &crate::area::AreaMode, wants: &[Want], f: u64) -> Option<Raw> {
        let w = a.world();
        let sc = w.scene();
        let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 12);
        let banned = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
            "menu_ban true" => Some(true),
            "menu_ban false" => Some(false),
            _ => None,
        });
        if !playing || banned == Some(true) || a.ui().menu_type() != -1 {
            return None;
        }
        let vm = a.vm()?;
        let here = |p: &piney_event::vm::EvPos| {
            sc.area != kind::DUNGEON || (i32::from(p.floor), i32::from(p.block)) == (sc.floor, sc.block)
        };
        let q = wants
            .iter()
            .find_map(|x| match *x {
                Want::Marker(m) => vm.mng.position(i32::from(m)).filter(here),
                _ => None,
            })?
            .pos;
        if sc.area != kind::DUNGEON {
            return self.walk_after(w, q, 200.0, f, false);
        }
        let p = w.player().body.pos.map(f32::from_bits);
        if (q[0] - p[0]).hypot(q[1] - p[1]) < 200.0 {
            return None;
        }
        let cam_z = f32::from_bits(w.camera().rot()[2]);
        Some(stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1]))))
    }

    /// Kite to `q` while farther than `close`: straight at it until a check
    /// finds him stopped, then [`path_to`] to within 300 of it on a grid
    /// round both; with `rush`, the stick at the rim ([`run_toward`]). None
    /// in a dungeon, or once close.
    fn walk_after(
        &mut self,
        w: &piney_world::field_world::FieldWorld,
        q: [f32; 4],
        close: f32,
        f: u64,
        rush: bool,
    ) -> Option<Raw> {
        let hits = match w.place() {
            Place::Field(fa) => &fa.hits,
            Place::Story(m) => m.hits(),
            Place::Dungeon(_) => return None,
        };
        let p = w.player().body.pos.map(f32::from_bits);
        let far = (q[0] - p[0]).hypot(q[1] - p[1]);
        if far < close {
            self.chase.clear();
            return None;
        }
        if f.is_multiple_of(60) {
            let stopped = self.chase_mark.is_some_and(|m| (m[0] - p[0]).hypot(m[1] - p[1]) < 60.0);
            let lost = self.chase.last().is_some_and(|e| (e[0] - q[0]).hypot(e[1] - q[1]) > 600.0);
            if stopped || lost {
                let mid = [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0];
                let step = (far * 2.2 / 81.0).max(100.0);
                let near = |x: [f32; 2], _| (x[0] - q[0]).hypot(x[1] - q[1]) < 300.0;
                self.chase = path_to(hits, [p[0], p[1], p[2]], mid, 81, step, near);
            }
            self.chase_mark = Some([p[0], p[1]]);
        }
        while self.chase.len() > 1 && (self.chase[0][0] - p[0]).hypot(self.chase[0][1] - p[1]) < 120.0 {
            self.chase.remove(0);
        }
        let to = self.chase.first().copied().unwrap_or([q[0], q[1]]);
        let cam_z = f32::from_bits(w.camera().rot()[2]);
        let h = (to[0] - p[0]).atan2(-(to[1] - p[1]));
        Some(if rush { run_toward(cam_z, h) } else { stick_toward(cam_z, h) })
    }

    /// A frame of the walk into the field's dungeon, while nothing holds
    /// Kite: straight at the entrance's middle while far, then along
    /// [`entrance_path`] (made again whenever a check finds him stopped).
    fn walk_into_dungeon(&mut self, a: &crate::area::AreaMode, f: u64) -> Option<Raw> {
        let w = a.world();
        let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 12);
        let banned = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
            "menu_ban true" => Some(true),
            "menu_ban false" => Some(false),
            _ => None,
        });
        if !playing || banned == Some(true) || a.ui().menu_type() != -1 {
            return None;
        }
        let Place::Field(fa) = w.place() else { return None };
        let mid = fa.dungeon_pos()?.map(f32::from_bits);
        let p = w.player().body.pos.map(f32::from_bits);
        let cam_z = f32::from_bits(w.camera().rot()[2]);
        let toward = |q: [f32; 2]| stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])));
        let far = (mid[0] - p[0]).hypot(mid[1] - p[1]);
        // Pressed into a rock or tree with no way found round it (every
        // field object has its hit: worklog 366): sideways a while, the
        // other way each time.
        if f < self.sidestep.0 {
            let h = (mid[0] - p[0]).atan2(-(mid[1] - p[1])) + self.sidestep.1 * std::f32::consts::FRAC_PI_2;
            return Some(stick_toward(cam_z, h));
        }
        if far > 2500.0 && self.path.is_empty() && !f.is_multiple_of(60) {
            return Some(toward([mid[0], mid[1]]));
        }
        if f.is_multiple_of(60) {
            let stopped = self.mark.is_some_and(|m| (m[0] - p[0]).hypot(m[1] - p[1]) < 60.0);
            if far <= 2500.0 && (stopped || self.path.is_empty()) {
                self.path = entrance_path(&fa.hits, [p[0], p[1], p[2]], [mid[0], mid[1]]);
            } else if far > 2500.0 && stopped {
                // Far and held by a wall: the same search on a grid wide
                // enough to hold him (81 cells, the step the distance asks).
                let step = (far * 2.2 / 81.0).max(100.0);
                self.path = path_to(&fa.hits, [p[0], p[1], p[2]], [mid[0], mid[1]], 81, step, |_, door| door);
                if self.path.is_empty() {
                    let side = if self.sidestep.1 > 0.0 { -1.0 } else { 1.0 };
                    self.sidestep = (f + 90, side);
                }
            }
            self.mark = Some([p[0], p[1]]);
        }
        while self.path.len() > 1 && (self.path[0][0] - p[0]).hypot(self.path[0][1] - p[1]) < 120.0 {
            self.path.remove(0);
        }
        Some(toward(self.path.first().copied().unwrap_or([mid[0], mid[1]])))
    }
}

/// The way from `from` to the nearest door floor of the dungeon entrance
/// round `mid` (attribute bit 0x80000, where `ccPlayer::CollisionTest` calls
/// `WORLD_MAN::Enter`): [`path_to`] over a 100-unit grid 3000 either side.
/// Each cell holds every floor a line straight down meets (the entrance's
/// roof, ramp and doorway), or the height map's ground.
fn entrance_path(hits: &piney_world::hit::Hits, from: [f32; 3], mid: [f32; 2]) -> Vec<[f32; 2]> {
    path_to(hits, from, mid, 61, 100.0, |_, door| door)
}

/// A way for Kite at `from` over an `n` by `n` grid of `step` round `mid`
/// to the first floor `goal` takes (its place, whether it is a door):
/// breadth first, a step open where no wall crosses it at knee and waist
/// height his width either side and the floor moves less than 120. The
/// turns only, the end last; empty with no way.
fn path_to(
    hits: &piney_world::hit::Hits,
    from: [f32; 3],
    mid: [f32; 2],
    n: i32,
    step: f32,
    goal: impl Fn([f32; 2], bool) -> bool,
) -> Vec<[f32; 2]> {
    // Half his width, and some.
    const WIDE: f32 = 100.0;
    let mut hits = hits.clone();
    let v = |x: f32, y: f32, z: f32| [x.to_bits(), y.to_bits(), z.to_bits(), 1f32.to_bits()];
    let at = |i: i32, j: i32| [mid[0] + (i - n / 2) as f32 * step, mid[1] + (j - n / 2) as f32 * step];
    let key = |(i, j): (i32, i32)| (j * n + i) as usize;
    let inside = |(i, j): (i32, i32)| (0..n).contains(&i) && (0..n).contains(&j);
    // Each cell's floors: (z, a door).
    let mut floors: Vec<Vec<(f32, bool)>> = vec![Vec::new(); (n * n) as usize];
    for j in 0..n {
        for i in 0..n {
            let [x, y] = at(i, j);
            hits.line(v(x, y, from[2] + 1500.0), v(x, y, from[2] - 1500.0), 0x2000_0001, 1);
            let mut fl: Vec<(f32, bool)> =
                hits.results.iter().map(|r| (f32::from_bits(r.cp[2]), r.att & 0x80000 != 0)).collect();
            if fl.is_empty() {
                fl.push((f32::from_bits(hits.ground_height(x.to_bits(), y.to_bits())), false));
            }
            fl.sort_by(|a, b| b.0.total_cmp(&a.0));
            fl.dedup_by(|a, b| (a.0 - b.0).abs() < 1.0);
            floors[key((i, j))] = fl;
        }
    }
    let mut clear = |a: [f32; 3], b: [f32; 3]| {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let n = dx.hypot(dy).max(1.0);
        let (sx, sy) = (-dy / n * WIDE, dx / n * WIDE);
        (a[2] - b[2]).abs() < 120.0
            && [30.0, 95.0].into_iter().all(|h| {
                [-1.0f32, 0.0, 1.0].into_iter().all(|k| {
                    let (ox, oy) = (sx * k, sy * k);
                    // Both ways: a wall stops a line only from its front.
                    let (p, q) = (v(a[0] + ox, a[1] + oy, a[2] + h), v(b[0] + ox, b[1] + oy, b[2] + h));
                    hits.line(p, q, 0x4000_0001, 1).is_none() && hits.line(q, p, 0x4000_0001, 1).is_none()
                })
            })
    };
    // A node: a cell and one of its floors.
    type Node = ((i32, i32), usize);
    let point = |(c, l): Node| {
        let q = at(c.0, c.1);
        [q[0], q[1], floors[key(c)][l].0]
    };
    // The floor of cell `c` a step from height `z` reaches.
    let level = |c: (i32, i32), z: f32| floors[key(c)].iter().position(|f| (f.0 - z).abs() < 120.0);
    let i0 = ((from[0] - mid[0]) / step).round() as i32 + n / 2;
    let j0 = ((from[1] - mid[1]) / step).round() as i32 + n / 2;
    let mut prev: std::collections::HashMap<Node, Node> = std::collections::HashMap::new();
    let mut queue = std::collections::VecDeque::new();
    // The cells round him he can walk straight to, on his floor.
    for (di, dj) in (-2..=2).flat_map(|i| (-2..=2).map(move |j| (i, j))) {
        let c = (i0 + di, j0 + dj);
        if !inside(c) {
            continue;
        }
        if let Some(l) = level(c, from[2])
            && clear(from, point((c, l)))
        {
            prev.insert((c, l), (c, l));
            queue.push_back((c, l));
        }
    }
    let mut end = None;
    while let Some(n) = queue.pop_front() {
        if goal(at(n.0.0, n.0.1), floors[key(n.0)][n.1].1) {
            end = Some(n);
            break;
        }
        let z = point(n)[2];
        for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let c = (n.0.0 + di, n.0.1 + dj);
            if !inside(c) {
                continue;
            }
            let Some(l) = level(c, z) else { continue };
            if prev.contains_key(&(c, l)) {
                continue;
            }
            // A diagonal only where both sides are open too.
            if di != 0 && dj != 0 {
                let sides = [(n.0.0 + di, n.0.1), (n.0.0, n.0.1 + dj)];
                let open = sides.iter().all(|&s| level(s, z).is_some_and(|ls| clear(point(n), point((s, ls)))));
                if !open {
                    continue;
                }
            }
            if clear(point(n), point((c, l))) {
                prev.insert((c, l), n);
                queue.push_back((c, l));
            }
        }
    }
    let Some(end) = end else { return Vec::new() };
    let (mut n, mut cells) = (end, vec![end.0]);
    while let Some(&b) = prev.get(&n).filter(|&&b| b != n) {
        cells.push(b.0);
        n = b;
    }
    cells.reverse();
    // Only the turns.
    let mut out: Vec<[f32; 2]> = Vec::new();
    for w in cells.windows(3) {
        let (d0, d1) = ((w[1].0 - w[0].0, w[1].1 - w[0].1), (w[2].0 - w[1].0, w[2].1 - w[1].1));
        if d0 != d1 {
            out.push(at(w[1].0, w[1].1));
        }
    }
    out.push(at(end.0.0, end.0.1));
    out
}

#[test]
#[ignore]
fn story_survey() {
    let only = std::env::var("PINEY_SURVEY_ONLY").ok().and_then(|v| v.parse().ok());
    match only {
        Some(n) => survey("infection", n..=n),
        None => survey("infection", 1..=31),
    }
}

/// Mutation's (M2, 101-116).
#[test]
#[ignore]
fn mutation_story_survey() {
    let only = std::env::var("PINEY_SURVEY_ONLY").ok().and_then(|v| v.parse().ok());
    match only {
        Some(n) => survey("mutation", n..=n),
        None => survey("mutation", 101..=116),
    }
}

/// Outbreak's (M3, 201-219).
#[test]
#[ignore]
fn outbreak_story_survey() {
    let only = std::env::var("PINEY_SURVEY_ONLY").ok().and_then(|v| v.parse().ok());
    match only {
        Some(n) => survey("outbreak", n..=n),
        None => survey("outbreak", 201..=219),
    }
}

/// The party, the members' address and call bits and the story's wants,
/// for `mutation_whole_story`'s trace.
fn whole_state(s: &Session) -> String {
    let (party, vm, save) = match &s.stage {
        Stage::Area(a) => (a.world().party(), a.vm(), &a.world().state().save),
        Stage::World(w) => (w.world().party(), w.vm(), &w.world().state().save),
        _ => return String::new(),
    };
    let wants = vm.map(|vm| story_wants(vm, save));
    let o = piney_data::save::offset::PARTY_MEMBER_FLAG;
    let c = piney_data::save::offset::PARTY_MEMBER_CALL;
    let area = match &s.stage {
        Stage::Area(a) => {
            let w = a.world();
            let c = w.combat();
            let kite = w.player().body.pos.map(f32::from_bits);
            let near = foes(c)
                .iter()
                .map(|&i| {
                    let ch = &c.scene.chars[i];
                    let p = ch.pos.map(f32::from_bits);
                    let o = c.foes.get(i).and_then(|e| e.as_ref()).map(|e| {
                        let bp = e.bpos.map(|v| f32::from_bits(v) as i32);
                        (e.act_num, e.target, e.freeze_flag, e.obj_flag, e.ent.ent_root, e.ent.ty, e.ene_id, bp)
                    });
                    (
                        (p[0] - kite[0]).hypot(p[1] - kite[1]) as i32,
                        ch.hp,
                        ch.id(),
                        p.map(|v| v as i32),
                        ch.spc_char.act_num,
                        ch.target_char,
                        o,
                    )
                })
                .min();
            format!(
                " targets {:?} foes {} nearest {near:?} battle {} menu {} phase {:?} playing {:?} fighters {}",
                w.event_targets(),
                foes(c).len(),
                c.battle.in_battle,
                a.ui().menu_type(),
                w.phase(),
                a.vm().and_then(|v| v.playing()),
                fighters(a)
            )
        }
        _ => String::new(),
    };
    format!("party {party:?} address {:x} call {:x} wants {wants:?}{area}", save.i32(o), save.i32(c))
}

/// With the gate hack (62) open, the Virus Cores its area asks for, given
/// through the console: the pilot drains no common foes to find them. A
/// harness aid, as god is.
pub(super) fn cores_for_hack(s: &mut Session) {
    let Stage::World(w) = &s.stage else { return };
    let Some(h) = w.ui().ctrl.hack.as_deref() else { return };
    let save = &w.world().state().save;
    let short: Vec<(i32, i32)> = (0..4)
        .map(|k| (h.protect[2 * k], h.protect[2 * k + 1]))
        .filter(|&(_, need)| need > 0)
        .map(|(id, need)| (id, need - i32::from(save.u8(piney_data::save::offset::IMP_ITEM_LIST + id as usize) as i8)))
        .filter(|&(_, n)| n > 0)
        .collect();
    for (id, n) in short {
        s.console(&format!("core {} {n}", (b'A' + id as u8) as char));
    }
}

/// The level the party is held at while a boss's parts are up (Kyvia's).
const BOSS_PARTS_LEVEL: i16 = 60;
/// The level against Fidchell, whose gauge (a physical defence of 2000)
/// and self-healing Epitaph outlast the story's levels (50 or so).
const FIDCHELL_LEVEL: i16 = 75;
/// The level against Gorre: its two brothers share its protect gauge, so
/// neither goes down quickly at the story's levels either.
const GORRE_LEVEL: i16 = 75;
/// The level of a Kite the story sends into a dungeon alone (event 206):
/// at the new game's 50, its locked rooms of Napylons (healers) hold him
/// some 25,000 frames each, and in the goal room the Data Bug's drained
/// form (Comad Goo) fells his 963 HP in one blow, past god's heal.
pub(super) const LONE_LEVEL: i16 = 75;

/// With a boss's parts up, the party raised to [`BOSS_PARTS_LEVEL`]
/// (with Fidchell or Gorre up, [`FIDCHELL_LEVEL`]/[`GORRE_LEVEL`]; Kite
/// alone in a dungeon where the story wants him alone, [`LONE_LEVEL`])
/// through the console's `exp` (the game's own level-ups): the pilot does
/// not grind, and at the story's start levels (30 or so) the party cannot
/// outpace Kyvia's healing gomora. A harness aid, as god is.
pub(super) fn levels_for_boss(s: &mut Session) {
    let Stage::Area(a) = &s.stage else { return };
    let w = a.world();
    let c = w.combat();
    let boss = c.boss.as_ref().filter(|r| !r.exit).map(|r| r.code);
    let alone = || {
        w.scene().area == kind::DUNGEON
            && c.members.len() == 1
            && a.vm().is_some_and(|vm| story_wants(vm, &w.state().save).contains(&Want::Alone))
    };
    let level = match boss {
        Some(piney_world::combat::boss::FIDCHELL) => FIDCHELL_LEVEL,
        Some(piney_world::combat::boss::GORRE) => GORRE_LEVEL,
        _ if focus(c).is_some() => BOSS_PARTS_LEVEL,
        _ if alone() => LONE_LEVEL,
        _ => return,
    };
    let low = c.members.iter().filter_map(|&(_, k)| c.scene.chars[k].spc()).map(|p| p.base.level).min();
    if let Some(lv) = low.filter(|&lv| lv < level) {
        let n = (i32::from(level - lv) * 1000).min(30000);
        s.console(&format!("exp {n}"));
    }
}

/// Event `n`'s flag, from the save of whatever stage the session is on.
pub(super) fn event_flag(s: &mut Session, n: i32) -> Option<u64> {
    let save = match &mut s.stage {
        Stage::World(w) => Some(&w.world().state().save),
        Stage::Area(a) => Some(&*a.save_mut()),
        Stage::Desktop(d) => d.save_mut().map(|s| &*s),
        Stage::TopPage(t) => t.save_mut().map(|s| &*s),
        _ => None,
    };
    save.map(|s| s.event_flag(n as usize))
}

/// Infection from its first story start (event 3) to its last event's end
/// under the autopilot, as [`mutation_whole_story`].
#[test]
#[ignore]
fn infection_whole_story() {
    whole_story("infection");
}

/// Mutation from a new game to its ending under the autopilot, the party
/// kept up: each story event's end as it comes, until event 116's. A
/// diagnostic (`--ignored --nocapture`; `PINEY_SURVEY_FRAMES`, 2,000,000).
#[test]
#[ignore]
fn mutation_whole_story() {
    whole_story("mutation");
}

/// Outbreak's, as [`mutation_whole_story`]: 201 to 219.
#[test]
#[ignore]
fn outbreak_whole_story() {
    whole_story("outbreak");
}

/// The survey's aids in a whole story (`PINEY_SURVEY_AIDS`). A run without
/// god stops at the first game over, which takes the game to the title.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Aids {
    /// God, the infection held at 0, the hacks' cores, the levels for the
    /// bosses (unset).
    All,
    /// No god: the party levelled to the foes it meets and Kite's potions
    /// and souls topped up in town instead, as a player grinds and shops
    /// (`levels`).
    Levels,
    /// None (`0`).
    None,
}

impl Aids {
    fn from_env() -> Aids {
        match std::env::var("PINEY_SURVEY_AIDS").as_deref() {
            Ok("0") => Aids::None,
            Ok("levels") => Aids::Levels,
            _ => Aids::All,
        }
    }
}

/// The levels a player keeps over the foes he meets ([`levels_for_foes`]).
const OVER_FOES: i16 = 3;

/// Without god: each member below the strongest foe (not a boss) standing
/// here and [`OVER_FOES`] gets the experience to that level (the game's own
/// level-ups on his next frame, as the console's `exp` gives them), as a
/// player who fights his way up would have. A harness aid.
fn levels_for_foes(s: &mut Session) {
    use piney_battle::chara::Body;
    let Stage::Area(a) = &mut s.stage else { return };
    let c = a.world_mut().combat_mut();
    // A boss's row level (Skeith's 99) is nominal: [`levels_for_boss`]'s.
    let top = foes(c)
        .into_iter()
        .filter_map(|k| match &c.scene.chars[k].body {
            Body::Foe(f) if f.boss.is_none() => Some(f.row.base.level),
            _ => None,
        })
        .max();
    let Some(top) = top.map(|lv| lv + OVER_FOES) else { return };
    let members: Vec<usize> = c.members.iter().map(|&(_, k)| k).collect();
    for k in members {
        if let Some(p) = c.scene.chars[k].spc_mut()
            && p.base.level < top
        {
            let n = (i32::from(top - p.base.level) * 1000).min(30000) as i16;
            p.base.exp = p.base.exp.saturating_add(n);
        }
    }
}

/// In a town, Kite's Healing Potions and Mage's Souls topped up to five
/// each, as a player buys them (the pilot does not shop). A harness aid.
fn supplies(s: &mut Session) {
    let Stage::World(w) = &s.stage else { return };
    let state = w.world().state();
    let count = |(cat, id): (i8, i16)| -> i32 {
        (0..piney_fieldui::items::ITEMS)
            .map(|k| piney_fieldui::items::save_item(state, 0, k))
            .filter(|it| (it.cat, it.id) == (cat, id))
            .map(|it| i32::from(it.num))
            .sum()
    };
    let short: Vec<((i8, i16), i32)> =
        [POTION, MAGES_SOUL].into_iter().map(|it| (it, 5 - count(it))).filter(|&(_, n)| n > 0).collect();
    for ((cat, id), n) in short {
        s.console(&format!("item {cat} {id} {n}"));
    }
}

/// The disc's story from its new game to its last event's end under the
/// autopilot, the survey's aids on.
fn whole_story(disc: &str) {
    let frames: u64 = std::env::var("PINEY_SURVEY_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(2_000_000);
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
    let Some(volume) = Iso::open(&iso).ok().and_then(|mut d| d.volume().ok()) else { return };
    let story = crate::start::story(volume);
    // The first event a story start can be made at (Infection's 1 and 2 are
    // the desktop's).
    let first = story.iter().copied().find(|n| crate::start::POINTS.contains(n));
    let (Some(first), Some(&last)) = (first, story.last()) else { return };
    let Some(mut s) = story_session_on(disc, first, |_| {}) else { return };
    let aids = Aids::from_env();
    if aids == Aids::All {
        s.console("god");
    }
    let mut pad = Pad::default();
    let mut pilot = StoryPilot::default();
    let mut ended: Vec<i32> = Vec::new();
    // The last place in The World and its state, for a game over's report.
    let mut last_place = String::new();
    // The fight's last 20 seconds (a line each half second), for the same.
    let mut fight: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    // `PINEY_SURVEY_STALL`: frames with no event ended before the run is
    // given up (200,000).
    let stall: u64 = std::env::var("PINEY_SURVEY_STALL").ok().and_then(|v| v.parse().ok()).unwrap_or(200_000);
    let mut last_end = 0u64;
    for f in 0..frames {
        // As the survey's god: the infection held at 0, the hack's cores.
        if aids != Aids::None && f.is_multiple_of(30) {
            s.console("infection 0");
            cores_for_hack(&mut s);
            levels_for_boss(&mut s);
            if aids == Aids::Levels {
                levels_for_foes(&mut s);
                supplies(&mut s);
            }
        }
        let raw = pilot.next(&s, f);
        pilot.after(&mut s);
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if matches!(s.stage, Stage::Area(_) | Stage::World(_)) && f.is_multiple_of(30) {
            last_place = format!("{} {}", Mode::title(&s), whole_state(&s));
        }
        // `PINEY_TRACE_BOSS`: a boss fight a line a second: the party, the
        // boss's act, HP and protect.
        if std::env::var_os("PINEY_TRACE_BOSS").is_some()
            && f.is_multiple_of(60)
            && let Stage::Area(a) = &s.stage
            && let Some(me) = a.world().combat().boss_char()
        {
            let c = a.world().combat();
            let ch = &c.scene.chars[me];
            let b = ch.foe_state().and_then(|fo| fo.boss.as_ref().map(|b| (b.act_num, fo.pp)));
            println!("BOSS {f} {b:?} hp {}/{} {}", ch.hp, ch.max_hp, fighters(a));
        }
        if f.is_multiple_of(30)
            && let Stage::Area(a) = &s.stage
            && a.world().combat().battle.in_battle != 0
        {
            fight.push_back(format!("{f} {}", fighters(a)));
            if fight.len() > 40 {
                fight.pop_front();
            }
        }
        if matches!(s.stage, Stage::Title(_)) && !last_place.is_empty() {
            for l in &fight {
                println!("FIGHT {l}");
            }
            panic!("a game over after {ended:?} at frame {f}; last in The World: {last_place}");
        }
        if !f.is_multiple_of(300) {
            continue;
        }
        if std::env::var_os("PINEY_DEBUG_PILOT").is_some() && f.is_multiple_of(3000) {
            eprintln!("WHOLE {f} {} {}", Mode::title(&s), whole_state(&s));
        }
        for &n in story {
            if !ended.contains(&n) && event_flag(&mut s, n).is_some_and(|x| x & 3 << 62 != 0) {
                ended.push(n);
                last_end = f;
                println!("{f}: event {n} ended - {}", Mode::title(&s));
            }
        }
        if ended.contains(&last) {
            return;
        }
        if f - last_end > stall {
            let menu = match &s.stage {
                Stage::Area(a) => {
                    let m = &a.ui().ctrl;
                    format!(
                        "menu {} proccess {} status {} wait {} list {:?}",
                        a.ui().menu_type(),
                        m.proccess,
                        m.menu_status,
                        m.wait_count,
                        m.list()
                    )
                }
                _ => String::new(),
            };
            panic!(
                "the story stalled after {ended:?} at frame {f}: {} {} | {menu} | pilot {:?} opened {} began {}",
                Mode::title(&s),
                whole_state(&s),
                pilot.action,
                pilot.opened,
                pilot.began
            );
        }
    }
    panic!("the story stopped after {ended:?} at {frames} frames: {}", Mode::title(&s));
}

/// Each start among `events` on the disc `disc`, driven for
/// `PINEY_SURVEY_FRAMES` frames (4000).
fn survey(disc: &str, events: std::ops::RangeInclusive<i32>) {
    let frames: u64 = std::env::var("PINEY_SURVEY_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(4000);
    for n in events {
        let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../work/{disc}/{disc}.iso"));
        if !iso.exists() {
            return;
        }
        if crate::start::build(&iso, n).is_err() {
            println!("story {n:2}: no start");
            continue;
        }
        piney_event::host::take_unported();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut s = story_session_on(disc, n, |_| {})?;
            // `PINEY_SURVEY_GOD`: the console's god (the party at full HP
            // and SP in the fields and dungeons), to follow the story past
            // fights the pilot would lose; Kite's infection held at 0, so a
            // boss's Data Drain cannot roll its game over (effect 30); the
            // gate hack's Virus Cores given; the party's levels for Kyvia.
            let god = std::env::var_os("PINEY_SURVEY_GOD").is_some();
            if god {
                s.console("god");
            }
            let mut pad = Pad::default();
            let mut titles: Vec<String> = Vec::new();
            let mut flag = 0u64;
            let mut pilot = StoryPilot::default();
            for f in 0..frames {
                if god && f.is_multiple_of(30) {
                    s.console("infection 0");
                    cores_for_hack(&mut s);
                    levels_for_boss(&mut s);
                }
                let raw = pilot.next(&s, f);
                // `PINEY_DEBUG_PILOT`: the pilot's actions and goals, and
                // every 50 frames in a field or dungeon the place, the
                // pad, the party and the foes.
                if std::env::var_os("PINEY_DEBUG_PILOT").is_some()
                    && f.is_multiple_of(50)
                    && let Stage::Area(a) = &s.stage
                {
                    let m = &a.ui().ctrl;
                    eprintln!(
                        "PILOT {f} {} {:?} proccess {} status {} {}",
                        Mode::title(&s),
                        raw.buttons,
                        m.proccess,
                        m.menu_status,
                        fighters(a)
                    );
                }
                if std::env::var_os("PINEY_DEBUG_PILOT").is_some()
                    && f.is_multiple_of(50)
                    && let Stage::World(w) = &s.stage
                {
                    let world = w.world();
                    let at = |p: [u32; 4]| p.map(|v| f32::from_bits(v) as i32);
                    let targets: Vec<_> = world
                        .event_targets()
                        .iter()
                        .map(|&(t, c)| {
                            (t, c, world.char_place(piney_world::entry::Kind::Npc, i32::from(c)).map(|p| at(p.0)))
                        })
                        .collect();
                    eprintln!(
                        "TOWN {f} {} {:?} kite {:?} targets {targets:?} command {:?} {}",
                        Mode::title(&s),
                        raw.buttons,
                        at(world.player().body.pos),
                        world.command_target(),
                        whole_state(&s)
                    );
                }
                pilot.after(&mut s);
                pad.read(&raw);
                s.step(&pad);
                s.take_events();
                let t = Mode::title(&s);
                let key: String = t
                    .split(" - ")
                    .filter(|p| !p.starts_with("frame") && !p.starts_with("play") && !p.starts_with("Kite at"))
                    .collect::<Vec<_>>()
                    .join(" - ");
                if titles.last() != Some(&key) {
                    titles.push(key);
                }
                if let Some(x) = event_flag(&mut s, n) {
                    flag = x;
                }
            }
            if std::env::var("PINEY_SURVEY_CALLS").is_ok() {
                let calls = match &s.stage {
                    Stage::World(w) => w.calls().iter().rev().take(40).cloned().collect::<Vec<_>>(),
                    Stage::Area(a) => a.calls().iter().rev().take(40).cloned().collect::<Vec<_>>(),
                    _ => Vec::new(),
                };
                for c in calls.iter().rev() {
                    eprintln!("CALL {c:?}");
                }
            }
            Some((titles, flag))
        }));
        let unported = piney_event::host::take_unported();
        match r {
            Ok(Some((t, flag))) => {
                // Closed (bit 63, its `end_event` played) turns done (62) at
                // the next mode's `ccStartThEvent`: both are the end.
                let state = if flag & 3 << 62 != 0 { "done".to_string() } else { format!("blocks {:#x}", flag) };
                println!("story {n:2}: {state}; {} places; last: {}", t.len(), t.last().cloned().unwrap_or_default());
                if t.len() > 1 {
                    // `PINEY_SURVEY_PATH` prints every place, one a line.
                    if std::env::var("PINEY_SURVEY_PATH").is_ok() {
                        for p in &t {
                            println!("          {p}");
                        }
                    } else {
                        println!(
                            "          path: {}",
                            t.iter().rev().take(6).rev().cloned().collect::<Vec<_>>().join(" | ")
                        );
                    }
                }
            }
            Ok(None) => println!("story {n:2}: no session"),
            Err(e) => {
                let msg =
                    e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()));
                println!("story {n:2}: PANIC {}", msg.unwrap_or_default());
            }
        }
        if !unported.is_empty() {
            println!("          unported: {unported:?}");
        }
    }
}

/// A lake's dungeon 1 wanted from its dungeon 0: a goal one floor past the
/// lake's last, which takes that floor's stairs down (`Enter`'s -1 on field
/// type 4: `ChangeScene(2, -2, -2, 1, 0, 0)`).
fn lake_below(w: &piney_world::field_world::FieldWorld, wants: &[Want]) -> Option<piney_event::vm::EvPoint> {
    let sc = w.scene();
    let Place::Dungeon(d) = w.place() else { return None };
    let below =
        wants.iter().any(|x| matches!(*x, Want::Dungeon(f, n) if f == sc.field as i16 && i32::from(n) > sc.dungeon));
    below.then_some(piney_event::vm::EvPoint { floor: d.floors.len() as i16, block: 0, num: -1 })
}

/// A Data Bug: `type` 0x40 (the common foes are 0x20), a foe of some
/// 20,000 HP that the story drains (Infection's rows 115, 201, 224, 235;
/// Outbreak's event 203 row 164).
const DATA_BUG: i32 = 0x40;

/// A boss or a Data Bug alive with its protect broken (`pp_count` frames
/// left), or with `any` any foe (a fight that holds the walk: event 206's
/// Napylons, who heal each other faster than a lone Kite hurts them):
/// Data Drain's to take.
fn drainable(ch: &piney_battle::chara::Char, any: bool) -> bool {
    ch.hp > 0
        && matches!(&ch.body, piney_battle::chara::Body::Foe(f)
            if (any || f.boss.is_some() || ch.ty() & DATA_BUG != 0) && f.pp_count > 0)
}

/// Data Drain's reach (2000 plus the boss's width).
const DRAIN_NEAR: f32 = 1200.0;

/// In a fight, a boss with its protect broken farther than [`DRAIN_NEAR`]
/// from Kite, who knows Data Drain: the stick toward it (the target menu
/// finds no one out of reach, and Innis roams where Skeith came to him).
fn approach_boss(a: &crate::area::AreaMode) -> Option<Raw> {
    let w = a.world();
    let c = w.combat();
    if c.battle.in_battle == 0
        || piney_fieldui::items::skill_list(&a.ui().texts().items, w.state(), 0, 5)[0] != DATA_DRAIN
    {
        return None;
    }
    let boss = drain_candidates(c).into_iter().find(|&e| drainable(&c.scene.chars[e], false))?;
    let p = w.player().body.pos.map(f32::from_bits);
    let q = c.scene.chars[boss].pos.map(f32::from_bits);
    if (q[0] - p[0]).hypot(q[1] - p[1]) < DRAIN_NEAR {
        return None;
    }
    let cam_z = f32::from_bits(w.camera().rot()[2]);
    Some(stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1]))))
}

/// What Data Drain may take: the foes on the lists, and Gorre's brothers
/// (Gorre itself is never listed; its brothers carry the protect gauge).
fn drain_candidates(c: &piney_world::combat::Combat) -> Vec<usize> {
    let mut v = c.enemies();
    if let Some(r) = c.boss.as_ref().filter(|r| !r.exit && r.code == piney_world::combat::boss::GORRE) {
        v.extend(r.parts.iter().copied().filter(|&p| c.scene.listed(p)));
    }
    v
}

/// [`stick_toward`] with the stick pushed to the square's rim: a lean
/// that the per-axis dead zone leaves under 231 walks (`ControlMove`'s
/// 230), and Kite walking cannot close on a goblin.
fn run_toward(cam_z: f32, h: f32) -> Raw {
    use std::f32::consts::PI;
    let want = (cam_z - PI - h).rem_euclid(2.0 * PI);
    (0..128)
        .map(|k| {
            let a = k as f32 * PI / 64.0;
            let r = 127.0 / a.cos().abs().max(a.sin().abs());
            let (lx, ly) =
                ((128.0 + r * a.cos()).clamp(0.0, 255.0) as u8, (128.0 + r * a.sin()).clamp(0.0, 255.0) as u8);
            Raw { analog: true, lx, ly, rx: 128, ry: 128, ..Raw::default() }
        })
        .filter_map(|raw| {
            let mut p = Pad::default();
            p.read(&raw);
            let d = (p.dirc_l.rem_euclid(2.0 * PI) - want).abs();
            (p.pow_l > 230).then_some((d.min(2.0 * PI - d), raw))
        })
        .min_by(|x, y| x.0.total_cmp(&y.0))
        .map_or_else(|| stick_toward(cam_z, h), |(_, raw)| raw)
}

/// A Speed Charm on Kite while a golden goblin that runs (see
/// [`shakes_off_holds`]) is on the field and he is not hasted. One runs at
/// `maxSpd` (22) times 1 + 1.6 `crisisRate` squared (`moveGold` act 1,
/// `crisisRate` 1 - distance / 1300): 40 within 400. Kite runs about 26 a
/// frame, 46 hasted (x1.75).
fn haste_for_gold(a: &crate::area::AreaMode) -> Option<Action> {
    let w = a.world();
    let c = w.combat();
    let &(_, k) = c.members.first()?;
    let state = w.state();
    let carried = (0..piney_fieldui::items::ITEMS)
        .map(|n| piney_fieldui::items::save_item(state, 0, n))
        .any(|it| (it.cat, it.id) == SPEED_CHARM && it.num > 0);
    let gold = c.enemies().into_iter().any(|e| c.scene.chars[e].hp > 0 && shakes_off_holds(c, e) && blows_land(c, e));
    (gold && carried && c.scene.chars[k].hp > 0 && c.scene.chars[k].cond[piney_battle::param::cond::SPEED] == 0)
        .then_some(Action::Item { cat: SPEED_CHARM.0, id: SPEED_CHARM.1, target: 0 })
}

/// The Items menu's page that lists category `cat` (`SetItemList`'s
/// filters: -1 recovery and tools, -2 the equipment, else one category).
fn item_page(t: &piney_fieldui::items::ItemTables, cat: i8) -> i16 {
    let c = i32::from(cat);
    let lists = |f: i32| match f {
        -1 => c == 10 || c == 13,
        -2 => (0..10).contains(&c),
        f => c == f,
    };
    t.pages.iter().position(|&f| lists(f)).unwrap_or(0) as i16
}

/// Whether foe `e` is a golden goblin that a blow frees from a hold
/// (`goldFlag`, `goldVolume` 2 up: `thinkGold`'s `goldDisHold`): an art
/// lands one hit of its combo, the normal attack (which holds nothing)
/// one blow before the goblin flinches and runs (`moveGold` act 7).
fn shakes_off_holds(c: &piney_world::combat::Combat, e: usize) -> bool {
    c.foes.get(e).and_then(Option::as_ref).is_some_and(|f| f.gold.flag && f.gold.volume >= 2)
}

/// Whether foe `e` is a golden goblin that shakes off holds in its attack
/// act: GOB3-1's cast La Repth (`mag0` 153) there, backing off at no more
/// than `maxSpd`. `interruptThink` makes no foe in its attack act flinch,
/// so both swings of the normal attack land.
fn casting_gold(c: &piney_world::combat::Combat, e: usize) -> bool {
    shakes_off_holds(c, e) && c.foes.get(e).and_then(Option::as_ref).is_some_and(|f| f.act_num == enemy_act::ATTACK)
}

/// What Kite's skill `sid` does to foe `e` at the roll `roll`
/// (`CalcBattleDamage` on a copy of the foe, no side effects): -1 a miss,
/// 0 a hit its Exdefense bars.
fn kite_hit(c: &piney_world::combat::Combat, e: usize, sid: i32, roll: i32) -> i32 {
    let (Some(&(_, k)), Some(sk)) = (c.members.first(), c.data.t.skill(sid)) else { return -1 };
    let mut tgt = c.scene.chars[e].clone();
    let (mut none, env) = (|| 0, piney_battle::chara::Env::default());
    let roll = piney_battle::damage::Roll::Quiet(roll);
    let (t, one) = (&c.data.t, piney_battle::damage::F_ONE);
    piney_battle::damage::calc_battle_damage(
        t,
        &c.scene.chars[k],
        &mut tgt,
        sk,
        one,
        roll,
        true,
        &mut none,
        &env,
        &mut Vec::new(),
    )
    .dmg
}

/// Whether Kite's plain blow can hurt foe `e`: at the best roll short of
/// a sure hit (94) it lands and does damage. GOB3-5's golden goblins
/// (row 156, pEva 9990) evade all but the sure hits, at pDef 9990.
fn blows_land(c: &piney_world::combat::Combat, e: usize) -> bool {
    kite_hit(c, e, 1, 94) > 0
}

/// In a fight with nothing holding Kite: away from the nearest foe until
/// the fight is over (Gate Out refuses in one: `GateoutMenu`).
fn out_of_battle(a: &crate::area::AreaMode) -> Option<Raw> {
    let w = a.world();
    let c = w.combat();
    let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 12);
    if c.battle.in_battle == 0 || !playing || a.ui().menu_type() != -1 {
        return None;
    }
    let p = w.player().body.pos.map(f32::from_bits);
    let d = |e: usize| {
        let q = c.scene.chars[e].pos.map(f32::from_bits);
        (q[0] - p[0]).hypot(q[1] - p[1])
    };
    let e = c.enemies().into_iter().filter(|&e| c.scene.chars[e].hp > 0).min_by(|&x, &y| d(x).total_cmp(&d(y)))?;
    let q = c.scene.chars[e].pos.map(f32::from_bits);
    let cam_z = f32::from_bits(w.camera().rot()[2]);
    Some(run_toward(cam_z, (p[0] - q[0]).atan2(-(p[1] - q[1]))))
}

/// The foes the pilot fights: the field's, and a boss's parts while its
/// lists hold them (Kyvia's core and gomoras; the body is not a target).
fn foes(c: &piney_world::combat::Combat) -> Vec<usize> {
    let mut v = c.enemies();
    if let Some(r) = c.boss.as_ref().filter(|r| !r.exit) {
        v.extend(r.parts.iter().copied().filter(|&p| c.scene.listed(p) && c.scene.chars[p].hp > 0));
    }
    v
}

/// The boss part to fight first while the lists hold it: a gomora of
/// attribute 0 (its attack heals Kyvia's core 200), Gorre's brother of the
/// lower physical PP defence, else the core (the first part).
fn focus(c: &piney_world::combat::Combat) -> Option<usize> {
    use piney_battle::boss::Class;
    let r = c.boss.as_ref().filter(|r| !r.exit)?;
    let up = |p: usize| c.scene.listed(p) && c.scene.chars[p].hp > 0;
    let healer = r.parts.iter().skip(1).copied().filter(|&p| up(p)).find(|&p| {
        let b = c.scene.chars[p].foe_state().and_then(|f| f.boss.as_ref());
        matches!(b.map(|b| &b.class), Some(Class::Gomora(g)) if g.my_attribute == 0)
    });
    // Gorre's brothers share one protect gauge: the one a physical hit
    // fills (row 41's physical PP defence is 2000, row 40's 9990).
    let brother = r
        .parts
        .iter()
        .copied()
        .filter(|&p| up(p))
        .filter_map(|p| {
            let f = c.scene.chars[p].foe_state()?;
            matches!(f.boss.as_ref().map(|b| &b.class), Some(Class::GorreBrother(_))).then_some((p, f.row.p_def_pp))
        })
        .min_by_key(|&(_, d)| d)
        .map(|(p, _)| p);
    healer.or(brother).or_else(|| r.parts.first().copied().filter(|&p| up(p)))
}

/// In a fight with no field foe near, a walk to within 350 of [`focus`]'s
/// part, else of the boss's nearest listed part.
fn approach_part(a: &crate::area::AreaMode) -> Option<Raw> {
    let w = a.world();
    let c = w.combat();
    if c.battle.in_battle == 0 {
        return None;
    }
    let r = c.boss.as_ref().filter(|r| !r.exit)?;
    let p = w.player().body.pos.map(f32::from_bits);
    let dist = |e: usize| {
        let q = c.scene.chars[e].pos.map(f32::from_bits);
        (q[0] - p[0]).hypot(q[1] - p[1])
    };
    let part = focus(c).or_else(|| {
        r.parts
            .iter()
            .copied()
            .filter(|&e| c.scene.listed(e) && c.scene.chars[e].hp > 0)
            .min_by(|&x, &y| dist(x).total_cmp(&dist(y)))
    })?;
    if dist(part) < 350.0 || c.enemies().into_iter().any(|e| c.scene.chars[e].hp > 0 && dist(e) < 400.0) {
        return None;
    }
    let q = c.scene.chars[part].pos.map(f32::from_bits);
    let cam_z = f32::from_bits(w.camera().rot()[2]);
    Some(stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1]))))
}

/// In a fight with no field foe near, a walk to within 350 of a boss
/// that roams its arena and is a target (Fidchell: its chases, escapes
/// and dashes; Gorre: its own chase and dash to the centre, leaving Kite
/// standing out of his skills' reach).
fn approach_roamer(a: &crate::area::AreaMode) -> Option<Raw> {
    let w = a.world();
    let c = w.combat();
    let r = c.boss.as_ref().filter(|r| {
        !r.exit && matches!(r.code, piney_world::combat::boss::FIDCHELL | piney_world::combat::boss::GORRE)
    })?;
    if c.battle.in_battle == 0 {
        return None;
    }
    let me = &c.scene.chars[r.me];
    if !c.scene.listed(r.me) || me.hp <= 0 || me.cond[piney_battle::param::cond::DEAD] != 0 {
        return None;
    }
    let p = w.player().body.pos.map(f32::from_bits);
    let dist = |e: usize| {
        let q = c.scene.chars[e].pos.map(f32::from_bits);
        (q[0] - p[0]).hypot(q[1] - p[1])
    };
    if dist(r.me) < 350.0 || c.enemies().into_iter().any(|e| e != r.me && c.scene.chars[e].hp > 0 && dist(e) < 400.0) {
        return None;
    }
    let q = me.pos.map(f32::from_bits);
    let cam_z = f32::from_bits(w.camera().rot()[2]);
    Some(stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1]))))
}

/// Kite straight to the story map's door ([`piney_world::field_world::FieldWorld::door`]),
/// the story wanting the block beyond it: while the field plays, no menu
/// is up and the events do not bar it.
fn walk_to_door(a: &crate::area::AreaMode) -> Option<Raw> {
    let w = a.world();
    let playing = matches!(w.phase(), piney_world::Phase::Play(n) if n > 12);
    let banned = a.calls().iter().rev().find_map(|(_, c)| match c.as_str() {
        "menu_ban true" => Some(true),
        "menu_ban false" => Some(false),
        _ => None,
    });
    if !playing || banned == Some(true) || a.ui().menu_type() != -1 {
        return None;
    }
    let door = w.door()?.map(f32::from_bits);
    let p = w.player().body.pos.map(f32::from_bits);
    let cam_z = f32::from_bits(w.camera().rot()[2]);
    Some(stick_toward(cam_z, (door[0] - p[0]).atan2(-(door[1] - p[1]))))
}

/// The room of this dungeon whose door leads out to a boss arena the story
/// wants: `GotoNextRoom`'s -255 into `special::exit_field`'s field, through
/// the dungeon's room of type 16 or more (area 46's to field 2).
fn arena_door(w: &piney_world::field_world::FieldWorld, wants: &[Want]) -> Option<(usize, usize)> {
    let Place::Dungeon(d) = w.place() else { return None };
    let field = piney_data::dungeon::special::exit_field(d.event_area, w.volume().number() as u32)?;
    wants.contains(&Want::Area(field as i16)).then_some(())?;
    let r = d.edit?.rooms.iter().find(|r| r.room_type >= 16)?;
    Some((r.floor as usize, r.index as usize))
}

/// The party's and the foes' HP and levels (the foes by name), for
/// `PINEY_DEBUG_PILOT`.
fn fighters(a: &crate::area::AreaMode) -> String {
    use piney_battle::chara::Body;
    let c = a.world().combat();
    let one = |k: usize| {
        let ch = &c.scene.chars[k];
        match &ch.body {
            Body::Spc(p) => format!(
                "{}/{} sp {}/{} lv {} cond {:?} act {} cnt {} flags {:#x}",
                ch.hp,
                ch.max_hp,
                ch.sp,
                ch.max_sp,
                p.base.level,
                &ch.cond.v[..2],
                ch.spc_char.act_num,
                ch.spc_char.cnt,
                ch.spc_char.flags
            ),
            Body::Foe(f) => {
                format!(
                    "{} {}/{} lv {} pp {} break {}{}",
                    String::from_utf8_lossy(&f.row.name),
                    ch.hp,
                    ch.max_hp,
                    f.row.base.level,
                    f.pp,
                    f.pp_count,
                    if f.boss.is_some() { " boss" } else { "" }
                )
            }
            Body::Other { .. } => String::new(),
        }
    };
    let party: Vec<_> = c.members.iter().map(|&(_, k)| one(k)).collect();
    let foes: Vec<_> = foes(c).into_iter().map(one).collect();
    format!("party {party:?} foes {foes:?}")
}

/// Outbreak's story start 211 in area 47's field: the straight walk to
/// the dungeon's mouth runs into a rock (every field object has its hit,
/// worklog 366), and no way round is found from where he stands
/// pressed to it. The pilot steps aside and goes on: Kite is in the
/// dungeon within 10,000 frames.
#[test]
fn story_211_walks_round_the_rock_into_area_47_s_dungeon() {
    let Some(mut s) = story_session_on("outbreak", 211, |_| {}) else { return };
    s.console("god");
    let (mut pad, mut pilot) = (Pad::default(), StoryPilot::default());
    for f in 0..10_000u64 {
        let raw = pilot.next(&s, f);
        pilot.after(&mut s);
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if let Stage::Area(a) = &s.stage
            && a.world().scene().area == kind::DUNGEON
            && a.world().scene().field == 47
        {
            return;
        }
    }
    panic!("not in area 47's dungeon: {}", Mode::title(&s));
}

/// Event 18 to its end in area 19's dungeon (the autopilot, the party
/// kept up), then block 20's `scene area=0 town=0` back in Mac Anu: how
/// long the town's loading card stays up and what each frame costs there.
/// A diagnostic: `cargo test --release -p piney-game event_18_back_in_town
/// -- --ignored --nocapture`.
#[test]
#[ignore]
fn event_18_back_in_town() {
    let Some(mut s) = story_session_on("infection", 18, |_| {}) else { return };
    s.console("god");
    let mut pad = Pad::default();
    let mut pilot = StoryPilot::default();
    let mut was_dungeon = false;
    let mut back = None;
    for f in 0..400_000u64 {
        if f.is_multiple_of(30) {
            cores_for_hack(&mut s);
        }
        let raw = pilot.next(&s, f);
        pilot.after(&mut s);
        pad.read(&raw);
        let t0 = std::time::Instant::now();
        s.step(&pad);
        let took = t0.elapsed();
        s.take_events();
        match &s.stage {
            Stage::Area(a) if a.world().scene().area == 2 => was_dungeon = true,
            Stage::World(_) if was_dungeon && back.is_none() => {
                back = Some(f);
                eprintln!("{f}: back in town: {}", Mode::title(&s));
            }
            _ => {}
        }
        if let Some(b) = back {
            let k = f - b;
            if k < 20 || k % 60 == 0 || took.as_millis() > 30 {
                eprintln!(
                    "+{k}: {:.1} ms, card {}, started {}, {}",
                    took.as_secs_f64() * 1000.0,
                    s.load_disp.is_some(),
                    s.load_started,
                    Mode::title(&s)
                );
            }
            if k > 900 {
                break;
            }
        }
    }
    assert!(back.is_some(), "never back in town: {}", Mode::title(&s));
}

/// `--mode story:18`: Log in to Mac Anu, where block 2 (Mia and Elk)
/// opens at once: the town's loading card, `gameStart` and each frame's
/// cost, printed. A diagnostic (`--ignored --nocapture`).
#[test]
#[ignore]
fn event_18_town_card() {
    let Some(mut s) = story_session_on("infection", 18, |_| {}) else { return };
    let mut pad = Pad::default();
    let mut pilot = StoryPilot::default();
    for f in 0..1500u64 {
        let raw = pilot.next(&s, f);
        pilot.after(&mut s);
        pad.read(&raw);
        let t0 = std::time::Instant::now();
        s.step(&pad);
        let took = t0.elapsed();
        s.take_events();
        if f < 10 || f % 50 == 0 || took.as_millis() > 30 {
            eprintln!(
                "{f}: {:.1} ms, card {}, started {}, {}",
                took.as_secs_f64() * 1000.0,
                s.load_disp.is_some(),
                s.load_started,
                Mode::title(&s)
            );
        }
    }
}

/// A long autopilot run (`PINEY_SURVEY_ONLY`, default event 17, the party
/// kept up): each 1000 frames' mean and worst step time and the effects'
/// census, to see whether a frame's cost climbs as the fights go on.
/// A diagnostic (`--ignored --nocapture`; `PINEY_SURVEY_FRAMES`).
#[test]
#[ignore]
fn frame_cost_over_a_long_run() {
    let n = std::env::var("PINEY_SURVEY_ONLY").ok().and_then(|v| v.parse().ok()).unwrap_or(17);
    let frames: u64 = std::env::var("PINEY_SURVEY_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(20000);
    let Some(mut s) = story_session_on("infection", n, |_| {}) else { return };
    s.console("god");
    // `PINEY_SURVEY_GS`: each frame drawn by a headless GS too, its texture
    // and pipeline counts and the draw's time printed.
    let mut gs = std::env::var_os("PINEY_SURVEY_GS")
        .map(|_| piney_gs::Gs::headless(piney_gs::Assets::new(s.archive.clone())).unwrap());
    let mut draw_ms = 0f64;
    let mut pad = Pad::default();
    let mut pilot = StoryPilot::default();
    let (mut sum, mut worst) = (0f64, 0f64);
    for f in 1..=frames {
        let raw = pilot.next(&s, f);
        pilot.after(&mut s);
        pad.read(&raw);
        let t0 = std::time::Instant::now();
        let frame = s.step(&pad);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        s.take_events();
        if let Some(gs) = &mut gs {
            let t1 = std::time::Instant::now();
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            draw_ms += t1.elapsed().as_secs_f64() * 1000.0;
        }
        sum += ms;
        worst = worst.max(ms);
        if f % 1000 == 0 {
            eprint!("{} draw commands; ", frame.cmds.len());
            if let Some(gs) = &gs {
                let (t, p) = gs.counts();
                eprint!("gs textures {t} pipelines {p}, draw mean {:.2} ms; ", draw_ms / 1000.0);
                draw_ms = 0.0;
            }
            let census = match &s.stage {
                Stage::Area(a) => {
                    let c = a.world().fx().census();
                    format!("effects {} particles {} gens {}", c.effects.len(), c.particles, c.weapon_generators)
                }
                _ => String::new(),
            };
            eprintln!("{f}: mean {:.2} ms, worst {worst:.1} ms, {census} - {}", sum / 1000.0, Mode::title(&s));
            (sum, worst) = (0.0, 0.0);
        }
    }
}

/// Event 3's goblins fought to the end (god mode, the action button on
/// the nearest foe) and the field shot headless as BATTLE MODE OFF comes
/// and after: what the fallen foes leave drawn. `PINEY_SHOTS=DIR cargo
/// test --release -p piney-game after_a_fight_shots -- --ignored
/// --nocapture` (default `/mnt/data/claude/scratch/afterfight`).
#[test]
#[ignore]
fn after_a_fight_shots() {
    let Some((mut s, mut f)) = story_to_field(0) else { return };
    let dir = std::env::var("PINEY_SHOTS").unwrap_or_else(|_| "/mnt/data/claude/scratch/afterfight".into());
    std::fs::create_dir_all(&dir).unwrap();
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/infection/infection.iso");
    let archive = Arc::new(Archive::new(Iso::open(&iso).unwrap().read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let mut gs = piney_gs::Gs::headless(piney_gs::Assets::new(archive)).unwrap();
    let mut log = Vec::new();
    story_until(&mut s, &mut f, "menu_ban false", 60, &mut log);
    s.console("god");
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let mut pad = Pad::default();
    let mut fought = false;
    let mut ended = None;
    let mut shots = 0;
    for n in 0..6000u32 {
        let (raw, in_battle) = {
            let Stage::Area(a) = &s.stage else { panic!("left the area") };
            let w = a.world();
            let c = w.combat();
            let k = c.kite.unwrap();
            let p = c.scene.chars[k].pos.map(f32::from_bits);
            let alive: Vec<usize> = c.enemies().into_iter().filter(|&e| !c.dead(e)).collect();
            let in_battle = c.battle.in_battle != 0;
            let raw = if let Some(&e) = alive.iter().min_by_key(|&&e| {
                let q = c.scene.chars[e].pos.map(f32::from_bits);
                ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2)) as i64
            }) {
                let q = c.scene.chars[e].pos.map(f32::from_bits);
                let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
                if dx * dx + dy * dy < 250.0 * 250.0 || in_battle && n.is_multiple_of(2) {
                    Raw { buttons: if n % 6 == 0 { Buttons::CROSS } else { Buttons::NONE }, ..still }
                } else {
                    stick_toward(f32::from_bits(w.camera().rot()[2]), dx.atan2(-dy))
                }
            } else {
                let (dx, dy) = (28600.0 - p[0], 24600.0 - p[1]);
                if dx * dx + dy * dy < 1500.0 * 1500.0 || fought {
                    still
                } else {
                    stick_toward(f32::from_bits(w.camera().rot()[2]), dx.atan2(-dy))
                }
            };
            (raw, in_battle)
        };
        fought |= in_battle;
        if fought && !in_battle && ended.is_none() {
            ended = Some(n);
            println!("the fight ended at {n}");
        }
        pad.read(&raw);
        let frame = s.step(&pad);
        s.take_events();
        if let Some(e) = ended
            && [0u32, 40, 120, 300].contains(&(n - e))
        {
            gs.set_overlay(Mode::archive(&s));
            gs.render(&frame);
            let (w, h) = gs.target_size();
            let path = format!("{dir}/after-{}.png", n - e);
            std::fs::write(&path, piney_gs::png::encode(w, h, &gs.read_back())).unwrap();
            println!("{path}: {}", Mode::title(&s));
            shots += 1;
            if shots == 4 {
                break;
            }
        }
    }
    assert!(ended.is_some(), "no fight ended");
}

/// Event 115 in field 13 with blocks 0-12 played: block 11's `area 13`
/// gives `WORLD_MAN` area 13's row, so the field is `EVENTAREA01`; block
/// 14's `marker_pos` reads its `DMY_marker_evNN` dummies and puts the six
/// NPCs there; the pilot talks to each (block 15-20's statuses), and block
/// 21 goes on to the arena, field 3.
#[test]
fn event_115_field_13_talks_to_six() {
    let iso = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../work/mutation/mutation.iso");
    if !iso.exists() {
        return;
    }
    let mut d = Iso::open(&iso).unwrap();
    let archive = std::sync::Arc::new(Archive::new(d.read_path("DATA/DATA.BIN").unwrap()).unwrap());
    let start = crate::start::build(&iso, 115).unwrap();
    let mut state = start.state;
    let flag = state.save.event_flag(115);
    state.save.set_event_flag(115, flag | ((1 << 13) - 1));
    state.save.set_u8(piney_data::save::offset::EVENT_STATUS, 1);
    let mut scene = piney_world::area::Scene::log_in(&mut state.save);
    let wm = crate::area::story_world_man(&mut d, 52, false).unwrap();
    let wm = crate::area::ev_area_number(&iso, 13, &wm, &state.save).unwrap();
    scene.change_scene(1, 2, 13, -1, -1, -1, &mut state.save);
    let mut s = Session::in_world(iso, archive, None, state, Some(start.vm), scene, Some(wm)).unwrap();
    let mut pad = Pad::default();
    let mut pilot = StoryPilot::default();
    let (mut story_map, mut placed, mut arena) = (false, false, false);
    for f in 0..8000u64 {
        let raw = pilot.next(&s, f);
        pilot.after(&mut s);
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if let Stage::Area(a) = &s.stage {
            let w = a.world();
            if w.scene().field == 13 {
                story_map |= matches!(w.place(), Place::Story(_));
                placed |= w.event_targets().len() == 6
                    && w.event_targets().iter().all(|&(t, c)| w.char_pos(t, c).is_some_and(|p| p[0] != 0 || p[1] != 0));
            }
            if w.scene().field == 3 {
                arena = true;
                break;
            }
        }
    }
    let status: Vec<u8> = match &s.stage {
        Stage::Area(a) => {
            (10..16).map(|k| a.world().state().save.u8(piney_data::save::offset::EVENT_STATUS + k)).collect()
        }
        _ => Vec::new(),
    };
    assert!(story_map, "field 13 was not EVENTAREA01");
    assert!(placed, "the six NPCs were not at their markers");
    assert_eq!(status, vec![1; 6], "not every talk ran");
    assert!(arena, "block 21 did not go to field 3: {}", Mode::title(&s));
}

/// Event 13 (MG0340) in Mac Anu: Mia and Elk are entered for the talk
/// (`entry 2 1`, `entry 2 10`); block 2, near marker 31, runs `remove -1
/// -1` and `scene -2` to reload the town. The old scene's fellow tasks
/// drop them from the registry, so the town comes back without them.
/// (Kite is put at the marker; the pilot does not walk to markers.)
#[test]
fn event_13_leaves_mia_and_elk_out() {
    let Some(mut s) = story_session_on("infection", 13, |_| {}) else { return };
    let mut pad = Pad::default();
    let mut pilot = StoryPilot::default();
    let registered = |s: &Session, id: i32| match &s.stage {
        Stage::World(w) => w.world().spcs().registry.iter().any(|r| r.id == id),
        _ => false,
    };
    let (mut met, mut done) = (false, None);
    for f in 0..30000u64 {
        let raw = pilot.next(&s, f);
        pilot.after(&mut s);
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        met |= registered(&s, 1) && registered(&s, 10);
        if f.is_multiple_of(60)
            && event_flag(&mut s, 13).is_some_and(|x| x & 2 != 0)
            && let Stage::World(w) = &mut s.stage
            && !w.streaming()
        {
            w.world_mut().pc_command(piney_event::host::PcCommand::PutMarker { pc: 0, marker: 31 });
        }
        if done.is_none() && event_flag(&mut s, 13).is_some_and(|x| x & 3 << 62 != 0) {
            done = Some(f);
        }
        if let Some(d) = done
            && f > d + 600
            && matches!(s.stage, Stage::World(_))
        {
            break;
        }
    }
    assert!(met, "Mia and Elk were never entered: {}", Mode::title(&s));
    assert!(done.is_some(), "event 13 did not end: {}", Mode::title(&s));
    assert!(!registered(&s, 1) && !registered(&s, 10), "still in the town: {}", Mode::title(&s));
}

/// Event 13's block 2: Kite put at marker 31, the fade and `scene -2` set
/// Mac Anu up again with nothing new to read, so `loadCheck` puts up no
/// card (the town's card showed over the scene), and block 3's stream 13
/// plays.
#[test]
fn event_13_s_town_again_has_no_card() {
    let Some(mut s) = story_session_on("infection", 13, |_| {}) else { return };
    let mut pad = Pad::default();
    let mut pilot = StoryPilot::default();
    let mut put = None;
    for f in 0..30000u64 {
        let raw = pilot.next(&s, f);
        pilot.after(&mut s);
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if let Some(p) = put {
            assert!(s.load_disp.is_none(), "a card at {}: {}", f - p, Mode::title(&s));
            if matches!(&s.stage, Stage::World(w) if w.streaming()) {
                return;
            }
            assert!(f < p + 600, "no stream 13: {}", Mode::title(&s));
        } else if event_flag(&mut s, 13).is_some_and(|x| x & 2 != 0)
            && let Stage::World(w) = &mut s.stage
            && !w.streaming()
        {
            w.world_mut().pc_command(piney_event::host::PcCommand::PutMarker { pc: 0, marker: 31 });
            put = Some(f);
        }
    }
    panic!("block 1 never ended: {}", Mode::title(&s));
}

/// Event 14 at the Expansive Haunted Sea of Sand's dungeon: the pilot to
/// the room where block 5 puts the Administrator (type 4, code 29), then
/// Kite talks to him (block 7) and to BlackRose (15), and what the screen
/// does after is printed. A diagnostic (`--ignored --nocapture`).
#[test]
#[ignore]
fn event_14_blackrose_after_the_administrator() {
    let Some(mut s) = story_session_on("infection", 14, |_| {}) else { return };
    s.console("god");
    let mut pad = Pad::default();
    let mut pilot = StoryPilot::default();
    let still = Raw { analog: true, lx: 128, ly: 128, rx: 128, ry: 128, ..Raw::default() };
    let press = |b: Buttons| Raw { buttons: b, ..still };
    // 0 the pilot, 1 to the Administrator, 2 his talk, 3 to BlackRose, 4 after.
    let (mut step, mut at) = (0, 0u64);
    for f in 0..80000u64 {
        let raw = match &s.stage {
            Stage::Area(a) if step > 0 => {
                let w = a.world();
                let c = w.combat();
                let playing = a.vm().is_some_and(|v| v.playing().is_some()) || a.streaming();
                let aimed = match w.command_target_code() {
                    Some((piney_world::entry::Kind::Npc, c)) => Some((4, c)),
                    Some((piney_world::entry::Kind::Gimmick, i)) => {
                        c.npcs.iter().find(|n| n.who == i as usize).map(|n| (4, i32::from(n.code)))
                    }
                    Some((piney_world::entry::Kind::Spc, id)) => Some((2, id)),
                    _ => None,
                };
                let p = w.player().body.pos.map(f32::from_bits);
                let cam_z = f32::from_bits(w.camera().rot()[2]);
                let toward = |q: [f32; 4]| stick_toward(cam_z, (q[0] - p[0]).atan2(-(q[1] - p[1])));
                match step {
                    1 | 3 if playing || a.ui().menu_type() != -1 => {
                        if f.is_multiple_of(24) {
                            press(Buttons::CROSS)
                        } else {
                            still
                        }
                    }
                    1 if aimed == Some((4, 29)) => {
                        step = 2;
                        at = f;
                        press(Buttons::CROSS)
                    }
                    1 => w.char_pos(4, 29).map(|q| toward(q.map(f32::from_bits))).unwrap_or(still),
                    2 if playing || a.ui().menu_type() != -1 => {
                        if f.is_multiple_of(24) {
                            press(Buttons::CROSS)
                        } else {
                            still
                        }
                    }
                    2 if f > at + 120 => {
                        step = 3;
                        still
                    }
                    3 if aimed == Some((2, 15)) => {
                        step = 4;
                        at = f;
                        println!("{f}: to BlackRose - {}", Mode::title(&s));
                        press(Buttons::CROSS)
                    }
                    3 => c.who(15).map(|k| toward(c.scene.chars[k].pos.map(f32::from_bits))).unwrap_or(still),
                    _ if playing => {
                        if f.is_multiple_of(24) {
                            press(Buttons::CROSS)
                        } else {
                            still
                        }
                    }
                    _ if a.ui().menu_type() != -1 && f > at + 300 => {
                        if f.is_multiple_of(40) {
                            press(Buttons::CIRCLE)
                        } else {
                            still
                        }
                    }
                    _ => still,
                }
            }
            _ => pilot.next(&s, f),
        };
        if step == 0 {
            pilot.after(&mut s);
        }
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        if step == 0
            && let Stage::Area(a) = &s.stage
            && a.world().event_targets().contains(&(4, 29))
            && a.world().char_pos(4, 29).is_some()
            && matches!(a.world().phase(), piney_world::Phase::Play(n) if n > 12)
        {
            step = 1;
            println!("{f}: the Administrator - {}", Mode::title(&s));
        }
        if step >= 1 && f.is_multiple_of(60) {
            let (menu, cam) = match &s.stage {
                Stage::Area(a) => {
                    let c = a.world().camera();
                    let cam = format!(
                        "cam {} dist {:.0} kind {} puppet {} lock {}",
                        c.cam_id,
                        f32::from_bits(c.tcam.dist),
                        c.tcam.kind,
                        c.puppet_show,
                        c.type_lock
                    );
                    (a.ui().menu_type(), cam)
                }
                _ => (-9, String::new()),
            };
            println!("{f} step {step} menu {menu} {cam} - {}", Mode::title(&s));
        }
        if step == 4 && f > at + 1500 {
            break;
        }
    }
    println!("end: step {step} - {}", Mode::title(&s));
}

/// Issue #39: Bob gives story area 24's keywords in a field (event 26,
/// `gate_add_msg 24`), and `DispInfo`'s window came up empty: the field's
/// host composed only a member's address. Its two lines are the
/// desktop's: the gate address, and `getItemMenuStr[7]`.
#[test]
fn bob_s_keywords_show_in_the_field() {
    let Some(mut s) = story_session_on("infection", 26, |_| {}) else { return };
    s.console("god");
    let mut pad = Pad::default();
    let mut pilot = StoryPilot::default();
    for f in 0..200_000u64 {
        let raw = pilot.next(&s, f);
        pilot.after(&mut s);
        pad.read(&raw);
        s.step(&pad);
        s.take_events();
        let Stage::Area(a) = &s.stage else { continue };
        if a.calls().iter().any(|(_, c)| c == "announce GateAddress { area: 24 }") {
            let sf = a.ui().ctrl.msg.sf;
            assert!(sf[0] > 0 && sf[1] > 0, "the window's lines: {sf:?}");
            return;
        }
    }
    panic!("Bob never gave the keywords: {}", Mode::title(&s));
}
