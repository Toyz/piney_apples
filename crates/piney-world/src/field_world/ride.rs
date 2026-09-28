//! The riding Grunty's calls on the field (`pgrider.cpp`), for the
//! sequence piney-game runs (`ride.rs`: `ccPuccigusoStart` on the menu
//! task, `ccThPucciguso`, `ccPuccigusoExit` with their fades, the menu and
//! the music): the flags, the party put to sleep and woken, the object
//! made and deleted. The ride's frame itself is the battle's
//! ([`crate::combat::ride`]).
//!
//! ```text
//! ride_start        ccPuccigusoStart's first slice: only in a field and
//!                   not riding (pgR); pgRideFlag, pgR 1, pgDIN 0,
//!                   camTypeLock
//! ride_sleep_party  its fade out done: ccSpcSleep (every registered
//!                   character standing, off the command lists, asleep),
//!                   then each in the party's body out of the collision
//!                   and its condition effect cleared
//! ride_create       ccLoadFLAddOne of the kind's file and ccThPucciguso:
//!                   the object made on the task's first frame, Main from
//!                   then on
//! ride_start_done   its fade in done: camTypeLock 0
//! ride_exit_begin   ccPuccigusoExit in a field: camTypeLock 1, the ride's
//!                   body out (ccDeleteCmnd finds it on no list)
//! ride_exit_wake    its fade out done: each party member not down back in
//!                   the collision, the members 150 behind Kite
//!                   (puccigusoAngleTbl), ccSpcWakeup
//! ride_end          its fade in done (or a dungeon entered): the object
//!                   gone, pgRideFlag and pgR 0, camTypeLock 0
//! ```

use std::rc::Rc;

use piney_battle::chara::spc_flag;
use piney_battle::event::{Event, Who};
use piney_battle::ride::{self, RideTables};

use super::{FieldWorld, Place, tasks};
use crate::body::{Body, TRALL};
use crate::camera::CamPad;
use crate::combat::Show;
use crate::combat::ride::{BODY_WHO, Pending};
use crate::combat::spc;
use crate::party::SpcChars;

impl FieldWorld {
    /// `pgRideFlag`: riding (from the flute's call to the end of the
    /// dismount).
    pub fn pg_ride(&self) -> bool {
        self.combat.ride.flag
    }

    /// From Mutation on, `ccSnd` +0x139: `ccWordsPlay` stays silent.
    pub fn voices_off(&self) -> bool {
        self.voices_off
    }

    /// `pgR`, `pgDIN`.
    pub fn pg_globals(&self) -> ride::Globals {
        self.combat.ride.g
    }

    /// `ccPgAdultCheck(game.server, k)` for the three pens: the kind of the
    /// grown Grunty the flute can call from each, None for none.
    pub fn pg_adult(&self) -> [Option<i32>; 3] {
        let server = self.scene.server;
        std::array::from_fn(|k| Some(ride::adult_check(&self.save.save, server, k as i32)).filter(|&n| n >= 0))
    }

    /// `ccPuccigusoStart(kind)`'s first slice (gcmn 0x005109c0): nothing
    /// outside a field (`game.area` 1) or while riding (`pgR`); else
    /// `pgRideFlag` 1, `pgR` 1, `pgDIN` 0 and the camera's type locked. The
    /// menu's ban and panels, `ccPgBgmInit` and the fade are the caller's.
    pub fn ride_start(&mut self) -> bool {
        let r = &mut self.combat.ride;
        if self.scene.area != 1 || r.g.pg_r != 0 {
            return false;
        }
        r.flag = true;
        r.g.pg_r = 1;
        r.g.pg_din = 0;
        self.camera.type_lock = true;
        true
    }

    /// `ccSpcSleep()` (gcmn 0x005a0760), then `ccPuccigusoStart`'s loop
    /// over the registry's first five: each registered character standing
    /// (`moveFlag` and `runFlag` 0, `stopFlag` 1), off its command list and
    /// asleep; each in the party (`partyFlag` 1) with its body out of the
    /// collision (`HitDisable`) and its condition effect cleared
    /// (`ClearConditionEffect`).
    pub fn ride_sleep_party(&mut self) {
        let party: Vec<i32> = self.with_party(|spcs, chars, hits| {
            let mut party = Vec::new();
            for r in spcs.registry.iter().take(ride::SLOTS) {
                if r.id == -1 {
                    continue;
                }
                let Some(c) = chars.spc(r.id) else { continue };
                *c.move_flag = false;
                *c.stop_flag = true;
                *c.run_flag = false;
                *c.listed = false;
            }
            for r in spcs.registry.iter().take(ride::SLOTS) {
                if r.id == -1 {
                    continue;
                }
                let Some(c) = chars.spc(r.id) else { continue };
                if *c.party_flag == 1 {
                    hits.hit_disable(c.hit);
                    party.push(r.id);
                }
            }
            party
        });
        for id in party {
            if let Some(w) = self.combat.who(id) {
                self.combat.shows.push(Show::Rule(Event::ClearConditionEffect(Who::Char(w))));
            }
        }
        self.combat.ride.asleep = true;
    }

    /// `ccLoadFLAddOne({11, puccigusoCharTbl[kind]})` and
    /// `ccStartThread(ccThPucciguso, 49, 0x800)`: the kind's file read, the
    /// object made on the task's first frame (over Kite's own `ctu1body`),
    /// `Main` each frame from then on.
    pub fn ride_create(&mut self, kind: i32) -> Result<(), String> {
        let tables = Rc::new(RideTables::of(self.combat.data.volume));
        let file = tables.file(kind).to_string();
        let body = Body::read(&self.archive, &file, TRALL).map_err(|e| format!("{file}: {e}"))?;
        self.combat.ride.pending = Some(Pending { kind, tables, body: Rc::new(body) });
        self.combat.ride.main_on = true;
        Ok(())
    }

    /// `ccPuccigusoStart`'s end: the camera's type free again.
    pub fn ride_start_done(&mut self) {
        self.camera.type_lock = false;
    }

    /// `ccPuccigusoExit`'s first slice in a field (gcmn 0x00510c18): the
    /// camera's type locked, `ccDeleteCmnd(pcgs)` (on no list) and the
    /// ride's body out of the collision. `Main` goes on through the fade.
    pub fn ride_exit_begin(&mut self) {
        // From Mutation on ccThPucciguso lets the words be heard again.
        self.voices_off = false;
        self.camera.type_lock = true;
        let hits = match &mut self.place {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Event(e) => &mut e.hits,
            Place::Arena(a) => &mut a.hits,
            Place::Giant(g) => &mut g.hits,
        };
        if let Some(o) = self.combat.ride.obj.as_deref_mut() {
            let mut b = spc::to_body(BODY_WHO, None, &o.ride.hit, false);
            hits.hit_disable(&mut b);
            o.ride.hit.sw = b.sw;
        }
    }

    /// `ccPuccigusoExit` after its fade out (gcmn 0x00510ca8): over the
    /// registry's first five, each in the party not down back in the
    /// collision (`HitEnable`), each member but Kite turned to Kite's
    /// heading and put 150 out from him at `puccigusoAngleTbl`'s angle
    /// ([`ride::exit_place`]); then `ccSpcWakeup` (`ccSPC::Wakeup`, gcmn
    /// 0x005a02e0): each in the party with no skill (`skillID`,
    /// `skillStatus` 0), act 0 from none (`actNumOld` -1), `armsEffectSW`
    /// 0, standing and free (`restraintSW`, `trajectorySW`, `pauseSW`,
    /// `moveFlag`, `runFlag` 0, `stopFlag` 1), its condition adjusted
    /// (`ConditionAdjustment`), on its command list and awake; and the
    /// party's AI told (`ccAISysMsgSend(0x1000e, -1, Kite's id, 0xffff, 0,
    /// 15)`: `ChatMessageQuitPucciguso`). `Main` stops.
    pub fn ride_exit_wake(&mut self) {
        let Some(k) = self.combat.kite else { return };
        let Some(tables) = self.combat.ride.obj.as_ref().map(|o| o.tables.clone()) else { return };
        let pos = self.combat.scene.chars[k].pos;
        let rot = self.combat.crew.spc.get(&k).map_or([0; 4], |s| s.dirc);
        let woken: Vec<i32> = self.with_party(|spcs, chars, hits| {
            for (i, r) in spcs.registry.iter().enumerate().take(ride::SLOTS) {
                if r.id == -1 {
                    continue;
                }
                let Some(c) = chars.spc(r.id) else { continue };
                if *c.party_flag != 1 {
                    continue;
                }
                if *c.dead == 0 {
                    hits.hit_enable(c.hit);
                }
                if r.id != 0 {
                    *c.dirc = rot;
                    *c.pos = ride::exit_place(&tables, pos, rot, i);
                }
            }
            let mut woken = Vec::new();
            for r in spcs.registry.iter().take(ride::SLOTS) {
                if r.id == -1 {
                    continue;
                }
                let Some(c) = chars.spc(r.id) else { continue };
                if *c.party_flag != 1 {
                    continue;
                }
                *c.skill_id = 0;
                *c.skill_status = 0;
                *c.act = 0;
                *c.act_old = -1;
                *c.restraint = false;
                *c.move_flag = false;
                *c.stop_flag = true;
                *c.run_flag = false;
                *c.listed = true;
                woken.push(r.id);
            }
            woken
        });
        for id in woken {
            let Some(w) = self.combat.who(id) else { continue };
            let ch = &mut self.combat.scene.chars[w];
            ch.spc_char.arms_effect_sw = 0;
            ch.spc_char.flags &= !(spc_flag::TRAJECTORY | spc_flag::PAUSE);
        }
        // ConditionAdjustment on each, its events carried out.
        let store = self.spcs.store;
        let info = self.task_info();
        let mut x = tasks(&mut self.place, &mut self.camera, &mut self.save.save, CamPad::default(), &info);
        self.combat.menu_clear_conditions(&mut x, false, &store);
        let id = self.combat.crew.sys_msg_id(k);
        self.combat.crew.send(0x1000e, id as u16, 0xffff, 0, 15, -1, None);
        let r = &mut self.combat.ride;
        r.asleep = false;
        r.main_on = false;
    }

    /// `ccPuccigusoExit`'s end: the object deleted (`~ccPucciguso`,
    /// `ccFileListDeleteOne`), `pgRideFlag` and `pgR` 0 and the camera's
    /// type free. Its body leaves the collision if it is still in it (a
    /// dungeon entered: the scene goes with it).
    pub fn ride_end(&mut self) {
        let hits = match &mut self.place {
            Place::Field(f) => &mut f.hits,
            Place::Dungeon(d) => &mut d.hits,
            Place::Event(e) => &mut e.hits,
            Place::Arena(a) => &mut a.hits,
            Place::Giant(g) => &mut g.hits,
        };
        let r = &mut self.combat.ride;
        if let Some(o) = r.obj.take() {
            let mut b = spc::to_body(BODY_WHO, None, &o.ride.hit, false);
            hits.hit_disable(&mut b);
        }
        r.pending = None;
        r.flag = false;
        r.g.pg_r = 0;
        r.main_on = false;
        self.camera.type_lock = false;
    }
}
