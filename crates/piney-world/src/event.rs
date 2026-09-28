//! The characters as the event scripts see and drive them: the event
//! interpreter (`piney-event`) asks its host for markers, the command
//! target and the characters, and hands it the character instructions
//! ([`NpcCommand`], [`PcCommand`], `entry`, `remove`); the runtime's host
//! forwards those to these [`World`] methods.
//!
//! A character is named by an event `type` and `code` (`ccEvent::GetSpc`,
//! `GetNpc`, `GetEnemy`; [`crate::event_kind`]):
//!
//! ```text
//! type 0-2   the party: code a charTbl row (0 Kite, 2 Orca), -1/-2 a party
//!            slot, -3 the whole party (ccPartyManager)
//! type 3-4   a town NPC: code its npcTbl row (merchants 0-4, 29 and 158
//!            are merchant-like)
//! type 5-6   an enemy or object entry
//! ```
//!
//! and a place by a marker: in a Root Town `markerEvTbl[marker]`
//! (gcmn 0x00317da0: 0 `DMY_gate`, 1-30 `DMY_marker_ev01`-`30`, 31
//! `DMY_marker71`, 32 `DMY_marker30`), a dummy of the town's file - its
//! position (+0x10) and rotation (+0x20, radians; z the heading).
//!
//! What is done here: markers; putting a character at a point or a marker
//! (`ccEvPcPos` 0x001ae7d8 and its NPC twin: the position, then
//! `ccAI::SetDircZ(RAD2DEG(marker.rot.z))`); turning at once (`chg` 0:
//! `ccAI::SetDircZ` 0x00581500 - the heading `DEG2RAD(dirc)`); facing
//! another character at once; the command target; talk handed to the
//! event. For the party (party.rs, ai.rs): `pc_act`, `pc_mode` and the
//! gradual turns (`chg` != 0: `gRotSp = chg`, 1 meaning 64, and `gDeg` the
//! heading, turned by the AI's remote command 0 each frame). NPCs' walking,
//! acts and gradual turns go to the NPC's own event mode
//! ([`crate::entry::Npc::command`]); the party's walking (`pc_walk_*`,
//! remote commands 1 and 2) and the rest are not ported. These methods
//! return false for what they do not do.

use piney_event::host::{CharRef, Marker, NpcCommand, PcCommand};

use crate::World;
use crate::ee::{self, F, ONE, V4};
use crate::entry::Kind;
use crate::party::{self, FaceTarget, SpcChars};

/// `markerEvTbl` (INF main 0x00317da0, `world::markers`): 33 dummy names.
pub const MARKERS: i16 = 33;

/// `ccAI::SetDircZ(d)` as a heading: `DEG2RAD(d)`.
fn heading(d: i16) -> F {
    ee::deg2rad(d)
}

/// `ccGetDirc(from, to)` (0x001d9ce0): the heading that faces `to` from
/// `from` - `pi/2 + atan2f(dy, dx)` wrapped into (-pi, pi] - as the 16-bit
/// angle `pc_face` hands `ccAI::SetDircZ` (`RAD2DEG`).
pub fn dirc_to(from: V4, to: V4) -> i16 {
    let dx = ee::sub(to[0], from[0]);
    let dy = ee::sub(to[1], from[1]);
    let mut r = ee::add(0x3fc9_0fdb, ee::atan2f(dy, dx));
    if !ee::le(r, 0x4049_0fdb) {
        r = ee::sub(r, 0x40c9_0fdb);
    }
    if ee::lt(r, 0xc049_0fdb) {
        r = ee::add(r, 0x40c9_0fdb);
    }
    ee::rad2deg(r)
}

impl World {
    /// `markerEvTbl[n]` in the town: the dummy's position (w 1) and its
    /// rotation's z, bits.
    pub fn marker_bits(&self, n: i16) -> Option<(V4, F)> {
        if !(0..MARKERS).contains(&n) {
            return None;
        }
        let name = piney_data::tables::world::of(self.volume).markers().get(n as usize)?.to_string();
        let file = &self.town.base.file;
        let obj = file.ccs.find_object(&name)?;
        let d = file.scene.dummies.get(&obj)?;
        let pos = [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE];
        let rot = d.rot.map_or(0, |r| piney_data::anim::const_radians([0, 0, r.z.to_bits()])[2]);
        Some((pos, rot))
    }

    /// [`World::marker_bits`] as the interpreter's `Host::marker`.
    pub fn marker(&self, n: i16) -> Option<Marker> {
        let (p, r) = self.marker_bits(n)?;
        Some(Marker { pos: p.map(f32::from_bits), dirc: f32::from_bits(r) })
    }

    /// `Host::command_target`: the character the action button would act
    /// on, its base flags and code.
    pub fn command_target_ref(&self) -> Option<CharRef> {
        let (kind, code) = self.targeting.target?;
        let types = match kind {
            Kind::Npc => self.npc(code).map_or(0, |n| n.flags()),
            Kind::Spc => self.party.flags(code),
            Kind::Gimmick => crate::talk::flags::CHAOS_GATE,
            Kind::Enemy => 0,
        };
        Some(CharRef { handle: code as u32, types, code: code as i16 })
    }

    /// The `near_marker` condition's distance in a town:
    /// `ccGetDist(ccTransPosW2P(pos), plw->posP)` (`ccEvent::CheckOpen`,
    /// main 0x001a7d9c), on the ground; the player's own `posP` is his
    /// position in his own frame.
    pub fn player_distance(&self, pos: V4) -> F {
        let p = self.player.body.pos;
        piney_battle::enemy_ai::get_dist(crate::char::w2p(pos, p), crate::char::w2p(p, p))
    }

    /// `trans_off` / `trans_on` (`ccEvent::Execute` cases 161 and 162, main
    /// 0x001b2248, 0x001b22f4): the character's `transDist` (+0x90),
    /// `ccChar::Draw`'s near fade. The type's bit picks the list: 2 a party
    /// member (`GetSpc`), 3 or 4 an NPC (`GetNpc`), 5 or 6 an enemy
    /// (`GetEnemy`: none in a town). False when there is no such character.
    pub fn set_trans(&mut self, ty: i16, code: i16, on: bool) -> bool {
        let bit = 1u32.checked_shl(ty as u32).unwrap_or(0);
        let code = i32::from(code);
        if bit & 0x4 != 0 {
            if code == 0 {
                self.player.trans_dist = on;
                return true;
            }
            return match self.party.actor_mut(code) {
                Some(a) => {
                    a.trans_dist = on;
                    true
                }
                None => false,
            };
        }
        if bit & 0x18 != 0 {
            return match self.npc_mut(code) {
                Some(n) => {
                    n.char_mut().trans_dist = on;
                    true
                }
                None => false,
            };
        }
        false
    }

    /// `ccCheckTarget(t)`: the character is still on the entry control's
    /// lists. The town's references carry the code alone
    /// ([`World::command_target_ref`]), so the kind is the command
    /// target's when it has that code, else any placed character's.
    pub fn target_alive(&self, t: &CharRef) -> bool {
        let code = i32::from(t.code);
        match self.targeting.target {
            Some((kind, c)) if c == code => self.char_place(kind, code).is_some(),
            _ => [Kind::Spc, Kind::Npc, Kind::Gimmick].into_iter().any(|k| self.char_place(k, code).is_some()),
        }
    }

    /// A character's position and heading by event type and code: Kite
    /// (party code 0), an NPC, or the Chaos Gate.
    pub fn char_pos(&self, ty: i16, code: i16) -> Option<(V4, V4)> {
        match crate::event_kind(ty)? {
            Kind::Spc if code == 0 || code == -3 => Some((self.player.body.pos, self.player.body.dirc)),
            Kind::Spc => self.party.place(i32::from(code)),
            Kind::Npc => self.npc(i32::from(code)).map(|n| (n.char().pos, n.char().dirc)),
            _ => None,
        }
    }

    /// Kite's heading set at once (`ccAI::SetDircZ`).
    fn kite_turn(&mut self, d: i16) {
        self.player.spc().set_dirc_z(d as u16);
    }

    /// `Host::pc`: the party character instructions, for Kite (code 0; the
    /// puts also -3) and the party members events placed: `pc_act`,
    /// `pc_mode`, `pc_turn`, `pc_face`, the remote walks and `pc_command`
    /// (party.rs), put and put at a marker. True when done (`pc_face` of types 0 and 1, which name no one
    /// in the game, counts as done).
    pub fn pc_command(&mut self, c: PcCommand) -> bool {
        let kite = |pc: i16| pc == 0 || pc == -3;
        match c {
            PcCommand::Act { pc, act } => {
                return self
                    .with_party(|spcs, chars, hits| party::pc_act(spcs, chars, hits, i32::from(pc), i32::from(act)));
            }
            PcCommand::Mode { pc, param } => {
                self.set_boot_param(pc, param);
                return true;
            }
            PcCommand::Turn { pc, dirc, chg } => {
                return self.with_party(|spcs, chars, _| party::pc_turn(spcs, chars, i32::from(pc), dirc, chg));
            }
            PcCommand::WalkPos { pc, x, y, z, run } => {
                let s = |v: i16| ee::mul(ee::from_int(i32::from(v)), 0x4120_0000);
                let goal = party::WalkGoal::Pos([s(x), s(y), s(z), ONE]);
                return self.with_party(|spcs, chars, hits| {
                    let Some(id) = spcs.turned(i32::from(pc)) else { return false };
                    let annihilated = spcs.annihilated(chars);
                    party::pc_walk(chars, hits, id, if run { 2 } else { 1 }, goal, annihilated)
                });
            }
            PcCommand::WalkDir { pc, rot, dist } => {
                return self.with_party(|spcs, chars, hits| {
                    let Some(id) = spcs.find(i32::from(pc)).map(|_| i32::from(pc)) else { return false };
                    let annihilated = spcs.annihilated(chars);
                    party::pc_walk(chars, hits, id, 1, party::WalkGoal::Dir { rot, dist }, annihilated)
                });
            }
            PcCommand::WalkMarker { pc, marker } => {
                // A Root Town's marker: its dummy, w 1 (none: the origin).
                let mut v = self.marker_bits(marker).map_or([0; 4], |(p, _)| p);
                v[3] = ONE;
                return self.with_party(|spcs, chars, hits| {
                    let Some(id) = spcs.find(i32::from(pc)).map(|_| i32::from(pc)) else { return false };
                    let annihilated = spcs.annihilated(chars);
                    party::pc_walk(chars, hits, id, 1, party::WalkGoal::At(v), annihilated)
                });
            }
            PcCommand::WalkChar { pc, ty, code, rot, dist } => {
                // The target by 1 << type: 4 GetSpc(code), 0x18 GetNpc(code);
                // a town has no enemies.
                let bit = 1u32.wrapping_shl((ty & 31) as u32);
                let target = if bit & 4 != 0 {
                    self.with_party(|spcs, chars, _| spcs.get_spc(i32::from(code)).and_then(|id| chars.pos(id)))
                } else if bit & 0x18 != 0 {
                    self.npc(i32::from(code)).map(|n| n.char().pos)
                } else {
                    None
                };
                return self.with_party(|spcs, chars, hits| {
                    let Some(id) = spcs.find(i32::from(pc)).map(|_| i32::from(pc)) else { return false };
                    let annihilated = spcs.annihilated(chars);
                    let Some(t) = target else {
                        // The walk is set up; the goal stays as it was.
                        if let Some(mut c) = chars.spc(id) {
                            c.manual_mode_ai(1, annihilated, hits);
                            c.set_remote_cmd(1);
                        }
                        return false;
                    };
                    let goal = party::WalkGoal::At(party::walk_offset(t, rot, dist));
                    party::pc_walk(chars, hits, id, 1, goal, annihilated)
                });
            }
            PcCommand::Command { pc, on } => {
                return self.with_party(|spcs, chars, _| party::pc_command(spcs, chars, i32::from(pc), on != 0));
            }
            PcCommand::Face { pc, ty, code, chg } => {
                let to = match FaceTarget::of(ty, code) {
                    FaceTarget::Spc(code) => {
                        self.with_party(|spcs, chars, _| spcs.get_spc(code).and_then(|id| chars.pos(id)))
                    }
                    FaceTarget::Npc(code) => self.npc(code).map(|n| n.char().pos),
                    FaceTarget::Enemy(_) => return false,
                    FaceTarget::None => return true,
                };
                let Some(to) = to else { return false };
                return self.with_party(|spcs, chars, _| party::pc_face(spcs, chars, i32::from(pc), to, chg));
            }
            _ => {}
        }
        match c {
            PcCommand::Put { pc, x, y, z } if kite(pc) => {
                let s = |v: i16| ee::mul(ee::from_int(i32::from(v)), 0x4120_0000);
                self.player.body.pos = [s(x), s(y), s(z), ONE];
                true
            }
            PcCommand::PutMarker { pc, marker } if kite(pc) => {
                let Some((pos, rot)) = self.marker_bits(marker) else { return false };
                self.player.body.pos = pos;
                self.kite_turn(ee::rad2deg(rot));
                true
            }
            PcCommand::Put { pc, x, y, z } | PcCommand::PartyPut { pc, x, y, z } => {
                let s = |v: i16| ee::mul(ee::from_int(i32::from(v)), 0x4120_0000);
                let pos = [s(x), s(y), s(z), ONE];
                self.with_party(|_, chars, _| chars.members.spc(i32::from(pc)).map(|c| *c.pos = pos).is_some())
            }
            PcCommand::PutMarker { pc, marker } | PcCommand::PartyPutMarker { pc, marker } => {
                let Some((pos, rot)) = self.marker_bits(marker) else { return false };
                self.with_party(|_, chars, _| {
                    let Some(mut c) = chars.members.spc(i32::from(pc)) else { return false };
                    *c.pos = pos;
                    c.set_dirc_z(ee::rad2deg(rot) as u16);
                    true
                })
            }
            _ => false,
        }
    }

    /// `ccEntryEventMng`'s step for one of `eventMng.entry[]` (the event's
    /// `entry type code marker param`): a party member (types 0-2) at the
    /// marker, facing its heading, on the command list when `param` is 5 -
    /// Kite himself for code 0; a town PC (type 3, `npcTbl` row `code`)
    /// through `ccSetRtownPC` and set at the marker, with the entry
    /// control's set-up ([`World::place_entries`]) or at once after it.
    /// True when placed or queued; administrators (type 4) and objects and
    /// enemies (5-7) are not ported.
    pub fn entry(&mut self, ty: i16, code: i16, marker: i16, param: i16) -> bool {
        match crate::event_kind(ty) {
            Some(Kind::Spc) => {}
            Some(Kind::Npc) => return self.entry_npc(ty, code, marker),
            _ => return false,
        }
        // Registered for the town's set-up, which builds a member and puts
        // it at its marker (rebootSpcManager, ccEntryEventMng), or at once
        // after it; the port puts Kite himself at his marker at once.
        if !self.entry_spc(code, marker, param) {
            return false;
        }
        if code != 0 {
            return true;
        }
        let (pos, rot) = if marker >= 0 { self.marker_bits(marker).unwrap_or(([0; 4], 0)) } else { ([0; 4], 0) };
        self.player.body.pos = pos;
        self.player.spc().set_dirc_z(ee::rad2deg(rot) as u16);
        true
    }

    /// `remove type code` (`ccEvent::Execute` 0x001aa290): type 2 takes a
    /// party member out of the party (`ccParty::DelMember` of its slot) and
    /// the registry (`ccSPC::DelSpc` of each slot with its id), and the port
    /// takes it off the field; types 5/6 delete an enemy or object
    /// (`ccEntryCtrl::deleteEnemy`) and -1 every object
    /// (`deleteAllObject`), neither of which the town has yet; other types
    /// do nothing.
    pub fn remove(&mut self, ty: i16, code: i16) {
        if ty == 2 {
            let code = i32::from(code);
            self.with_party(|spcs, chars, _| {
                if let Some(s) = spcs.member_id.iter().position(|&m| m == code) {
                    spcs.del_member(s as i32, chars);
                }
            });
            for i in 0..party::REGISTRY {
                if self.spcs.registry[i].id == code {
                    self.spcs.del_spc(i);
                }
            }
            self.party.drop_member(code, &mut self.town.base.hits);
            self.party.combat.set_party(self.spcs.party());
        }
    }

    /// `Host::npc`: put, put at a marker, turn and face at once for the NPC
    /// whose code is `npc`; anything else goes to its own event mode. True
    /// when done.
    pub fn npc_command(&mut self, c: NpcCommand) -> bool {
        let code = |c: &NpcCommand| match *c {
            NpcCommand::Act { npc, .. }
            | NpcCommand::WalkPos { npc, .. }
            | NpcCommand::WalkDir { npc, .. }
            | NpcCommand::WalkMarker { npc, .. }
            | NpcCommand::WalkChar { npc, .. }
            | NpcCommand::PutMarker { npc, .. }
            | NpcCommand::Put { npc, .. }
            | NpcCommand::Turn { npc, .. }
            | NpcCommand::Face { npc, .. } => i32::from(npc),
        };
        let target = match c {
            NpcCommand::Face { ty, code, .. } => self.char_pos(ty, code).map(|(p, _)| p),
            _ => None,
        };
        let marker = match c {
            NpcCommand::PutMarker { marker, .. } | NpcCommand::WalkMarker { marker, .. } => self.marker_bits(marker),
            _ => None,
        };
        let Some(n) = self.npc_mut(code(&c)) else { return false };
        if n.command(&c, marker, target) {
            return true;
        }
        let ch = n.char_mut();
        match c {
            NpcCommand::Put { x, y, z, .. } => {
                let s = |v: i16| ee::mul(ee::from_int(i32::from(v)), 0x4120_0000);
                ch.pos = [s(x), s(y), s(z), ONE];
                true
            }
            NpcCommand::PutMarker { .. } => {
                let Some((pos, rot)) = marker else { return false };
                ch.pos = pos;
                ch.dirc[2] = heading(ee::rad2deg(rot));
                true
            }
            NpcCommand::Turn { dirc, chg: 0, .. } => {
                ch.dirc[2] = heading(dirc);
                true
            }
            NpcCommand::Face { chg: 0, .. } => {
                let Some(to) = target else { return false };
                ch.dirc[2] = heading(dirc_to(ch.pos, to));
                true
            }
            _ => false,
        }
    }
}
