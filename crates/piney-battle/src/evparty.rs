//! The event instructions that act on the party and the enemies in a
//! fight's state: the cases of `ccEvent::Execute` (main 0x001a8d20) that
//! walk, place, list and hold characters, the `ccThEvHold` task one of them
//! starts, and `player_skill`'s set-up, wait and step. The interpreter
//! (piney-event) decodes the instructions and calls its host; a host whose
//! characters are this crate's calls these.
//!
//! ```text
//! case  instruction       main        here
//!   13  remove 5/6 code   0x001aa290  get_enemy, remove_enemy (deleteEnemy)
//!   61  pc_walk_pos       0x001ad8ac  EvParty::pc_walk_pos (remote command 1)
//!  164  pc_run_pos        0x001ad9b0  EvParty::pc_walk_pos (remote command 2)
//!   62  pc_walk_dir       0x001adab0  EvParty::pc_walk_dir
//!   63  pc_walk_marker    0x001adbf4  EvParty::pc_walk_marker
//!   64  pc_walk_char      0x001adda4  EvParty::pc_walk_char
//!   66  pc_command        0x001ae164  EvParty::pc_command (ccEntryCmnd / ccDeleteCmnd)
//!   67  pc_put_marker     0x001ae7d8  EvParty::pc_put_marker
//!   68  party_put_marker  0x001ae9e8  EvParty::party_put_marker
//!   69  party_put         0x001aec5c  EvParty::party_put
//!   70  pc_put            0x001aeb84  EvParty::pc_put
//!   71  pc_turn           0x001aed5c  EvParty::pc_turn
//!   72  pc_face           0x001aee30  EvParty::pc_face
//!   74  enemy_put         0x001af028  EvParty::enemy_put
//!  132  hold              0x001b09fc  hold; the task ccThEvHold (0x001b5040): hold_frame
//!  133  hold_end          0x001b0a68  hold_end
//!  153  player_skill      0x001b1f04  player_skill_begin / _busy / _step / _end
//!  163  battle_ready      0x001b23a8  battle_ready
//! ```
//!
//! # Who `pc` names
//!
//! The instructions name party characters three ways (a [`Roster`] holds
//! what they read: `ccSpcManager`'s registry, gcmn 0x00730340, and
//! `ccPartyManager`, 0x00730310):
//!
//! - `GetSpc(code)` (main 0x001b2bb0, [`Roster::get_spc`]): for `code` >= 0
//!   the first registry slot whose id is `code`, its character; for a
//!   negative code the id `memberID[-code]` holds (-3 reads `num` there),
//!   looked up the same way.
//! - [`Roster::named`] (`pc_walk_pos`, `pc_run_pos`, `pc_put`,
//!   `pc_put_marker`, `pc_turn`, `pc_face`): `GetSpc(pc)` for `pc` >= 0,
//!   else `memberChar[-pc]`
//!   (-1, -2 the companions; -3 reads `memberID[0]`, Kite's id 0, as the
//!   pointer, so no one).
//! - [`Roster::registered`] (`pc_walk_dir`, `pc_walk_marker`,
//!   `pc_walk_char`): the first registry slot whose id equals `pc`, sign
//!   extended; a negative `pc` matches only a free slot (id -1).
//! - [`Roster::commanded`] (`pc_command`): the first slot of id `pc`; for
//!   -3 each party slot's member; for -1/-2 that slot's member.
//! - [`Roster::companion`] (`party_put`, `party_put_marker`): the first of
//!   `memberChar[1]`, `memberChar[2]` whose character is not `pc` (by its
//!   base id).
//!
//! A character is a scene index. What these write is the `ccChar` /
//! `ccSpcChar` members the rest of the crate keeps: the position is the
//! [`Char`]'s, the heading and `bodyHit` its [`Spc`]'s, the act, the
//! ghost flag, `cloak` and `dead` the [`Char`]'s (with the [`Spc`] copies of
//! `actNum` and `ghostFlag` kept equal, as between frames), the AI's
//! members the crew's [`Ai`]. An enemy's heading is its
//! [`Enemy::dirc`](crate::enemy_ai::Enemy).
//!
//! # The remote walks
//!
//! Each walk puts the character under manual control
//! (`ccSpcChar::ManualModeAI(1)`, gcmn 0x0059eba0: a dead character is
//! revived, a fallen one stood up, then `ccAI::ManualMode` 0x00583270), sets
//! the remote command (`SetRemoteCmd` 0x005832e0: 1, or 2 for `pc_run_pos`)
//! and the goal (`SetGoalPos(v, 0)` 0x005833a0: `gPoint` -1, `gPos` v); the
//! AI's `ManualControl` then walks there ([`crate::ai_move`]). Remote
//! command 1 moves with `runFlag` set and 2 without it, so `pc_walk_pos` is
//! the running one of the two (the names are the instruction set's). The
//! goals:
//!
//! ```text
//! pc_walk_pos  (10 x, 10 y, 10 z, 1)
//! pc_walk_dir  the character's position + 10 dist (sinf(a), -cosf(a)), a = DEG2RAD(rot); w kept
//! pc_walk_char the target's position + the same offset; w the target's
//! pc_walk_marker  field, dungeon: the first event position with that number;
//!              Root Town: the marker's dummy (markerEvTbl); w 1
//! ```
//!
//! Every multiply and add is the FPU's (`crate::geom`), in the game's
//! order.

use crate::affect::{self, AffectCtx};
use crate::chara::spc_flag;
use crate::enemy_ai::Enemy;
use crate::entry::{self, Cx, EntryCtrl, EvPos, Kind, Out};
use crate::event::Events;
use crate::exp::Party;
use crate::geom::{self, F, ONE, V4};
use crate::param::cond;
use crate::party_ai::Crew;
use crate::rand::Rng;
use crate::scene::Scene;
use crate::tables::Tables;
use crate::world::World;

/// 10.0, the instructions' unit.
const K10: F = 0x4120_0000;
/// 200.0 and -200.0 (`pc_put_marker` with no marker).
const K200: F = 0x4348_0000;
const KM200: F = 0xc348_0000;
/// `ccSPC::registry`'s length.
pub const REGISTRY: usize = 5;

/// `ccSpcManager` and `ccPartyManager` as the instructions read them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Roster {
    /// `registry[i].id` (+0x00): the `charTbl` row, -1 free.
    pub ids: [i32; REGISTRY],
    /// `registry[i].charPtr` (+0x1c): the character built for the slot.
    pub chars: [Option<usize>; REGISTRY],
    /// `memberChar`, `memberID`, `num`.
    pub party: Party,
}

impl Default for Roster {
    fn default() -> Self {
        Roster { ids: [-1; REGISTRY], chars: [None; REGISTRY], party: Party::default() }
    }
}

impl Roster {
    /// The first registry slot whose id is `id`, its character (null ends
    /// the search too).
    fn by_id(&self, id: i32) -> Option<usize> {
        let i = self.ids.iter().position(|&r| r == id)?;
        self.chars[i]
    }

    /// `ccEvent::GetSpc(code)` (main 0x001b2bb0): the registered character
    /// `code`; for a negative code the one `memberID[-code]` names (-3 reads
    /// `num`; a negative id names no one). Below -3 the game reads past
    /// `ccParty`; the port names no one.
    pub fn get_spc(&self, code: i32) -> Option<usize> {
        if code >= 0 {
            return self.by_id(code);
        }
        let id = match code {
            -1 => self.party.ids[1],
            -2 => self.party.ids[2],
            -3 => self.party.num,
            _ => return None,
        };
        if id < 0 {
            return None;
        }
        self.by_id(id)
    }

    /// The character `pc_walk_pos`, `pc_run_pos`, `pc_put`,
    /// `pc_put_marker`, `pc_turn` and `pc_face` act on: `GetSpc(pc)` for `pc` >= 0, else
    /// `memberChar[-pc]`. -3 reads `memberID[0]` as the pointer (Kite's id
    /// 0: null), and below that the game reads further ids as pointers; the
    /// port names no one for either.
    pub fn named(&self, pc: i16) -> Option<usize> {
        match pc {
            0.. => self.get_spc(i32::from(pc)),
            -1 => self.party.members[1],
            -2 => self.party.members[2],
            _ => None,
        }
    }

    /// The character `pc_walk_dir`, `pc_walk_marker` and `pc_walk_char` act
    /// on: the first registry slot whose id equals `pc` (sign extended), its
    /// character.
    pub fn registered(&self, pc: i16) -> Option<usize> {
        self.by_id(i32::from(pc))
    }

    /// The characters `pc_command` acts on, in order: for `pc` >= 0 the
    /// first slot of id `pc` (nothing when its character is null); for -3
    /// each party slot's member (`memberID`, then its first registry slot);
    /// for -1/-2 that slot's member. Below -3 the game reads past
    /// `memberID`; the port names no one.
    pub fn commanded(&self, pc: i16) -> Vec<usize> {
        let one = |id: i32| if id >= 0 { self.by_id(id) } else { None };
        match pc {
            0.. => self.by_id(i32::from(pc)).into_iter().collect(),
            -3 => self.party.ids.iter().filter_map(|&id| one(id)).collect(),
            -1 | -2 => one(self.party.ids[(-pc) as usize]).into_iter().collect(),
            _ => Vec::new(),
        }
    }

    /// The companion `party_put` and `party_put_marker` place:
    /// `memberChar[1]`, or `memberChar[2]` when slot 1 is empty or is `pc`
    /// (its base id); none when that one is `pc` too.
    pub fn companion(&self, scene: &Scene, pc: i16) -> Option<usize> {
        let is_pc = |c: usize| scene.chars[c].id() == pc;
        let mut c = self.party.members[1];
        match c {
            Some(m) if is_pc(m) => c = self.party.members[2],
            None => c = self.party.members[2],
            _ => {}
        }
        c.filter(|&m| !is_pc(m))
    }
}

/// `ccEvent::GetEnemy(code)` (main 0x001b2d50): the first object on the
/// entry control's enemy list (walked for its count) whose base type has a
/// bit of 0x60 and whose base id is `code`.
pub fn get_enemy(ctrl: &EntryCtrl, scene: &Scene, code: i32) -> Option<usize> {
    find_on(ctrl, scene, Kind::Enemy, 0x60, code)
}

/// `ccEvent::GetNpc(code)` (main 0x001b2cc0): the same on the NPC list, of
/// base type bit 0x10 for codes 29 and 158 (the merchant-like rows), else
/// 0x08.
pub fn get_npc(ctrl: &EntryCtrl, scene: &Scene, code: i32) -> Option<usize> {
    let mask = if code == 29 || code == 158 { 0x10 } else { 0x08 };
    find_on(ctrl, scene, Kind::Npc, mask, code)
}

fn find_on(ctrl: &EntryCtrl, scene: &Scene, k: Kind, mask: i32, code: i32) -> Option<usize> {
    let l = ctrl.lists[k as usize];
    let mut cur = l.head;
    for _ in 0..l.num {
        let c = cur?;
        let ch = &scene.chars[c];
        if ch.ty() & mask != 0 && i32::from(ch.id()) == code {
            return Some(c);
        }
        cur = ctrl.links.get(c).and_then(|l| l.next);
    }
    None
}

/// `ccCheckTargetTypeId(mask, id)` (gcmn 0x005199e0): the first character
/// on the party's command list (mask bits 0x7), then the foes' (0xe0), then
/// the objects' (the other bits) whose base type has a bit of `mask` and
/// whose base id is `id`.
pub fn check_target_type_id(scene: &Scene, mask: i32, id: i32) -> Option<usize> {
    let lists = [(7, &scene.pc_list), (0xe0, &scene.ene_list), (!0xe7, &scene.obj_list)];
    for (bits, list) in lists {
        if mask & bits == 0 {
            continue;
        }
        if let Some(&c) =
            list.iter().find(|&&c| scene.chars[c].ty() & mask != 0 && i32::from(scene.chars[c].id()) == id)
        {
            return Some(c);
        }
    }
    None
}

/// `ccGetDircPL(pos)` (main 0x001da610): the heading from `pos` toward the
/// player's frame's origin: `pos` into the player's frame
/// (`ccTransPosW2P`), x and y times -1.0, `atan2f(y, x) + pi/2`, wrapped
/// into -pi..pi.
pub fn get_dirc_pl(world: &mut dyn World, pos: V4) -> F {
    let p = world.w2p(pos);
    let x = geom::mul(p[0], geom::MINUS_ONE);
    let y = geom::mul(p[1], geom::MINUS_ONE);
    let mut a = geom::add(geom::HALF_PI, geom::atan2f(y, x));
    if !geom::le(a, geom::PI) {
        a = geom::sub(a, geom::TWO_PI);
    }
    if geom::lt(a, geom::NEG_PI) {
        a = geom::add(a, geom::TWO_PI);
    }
    a
}

/// `(10 x, 10 y, 10 z, 1)`: an instruction's position (`10.0f * (float)v`).
pub fn scaled(x: i16, y: i16, z: i16) -> V4 {
    let s = |v: i16| geom::mul(K10, geom::from_int(i32::from(v)));
    [s(x), s(y), s(z), ONE]
}

/// `v + 10 dist (sinf(a), -cosf(a))` with `a = DEG2RAD(rot)`: the offset of
/// `pc_walk_dir` and `pc_walk_char` (x and y only).
pub fn offset(mut v: V4, rot: i16, dist: i16) -> V4 {
    let a = geom::deg2rad(rot);
    let d = geom::mul(K10, geom::from_int(i32::from(dist)));
    v[0] = geom::add(v[0], geom::mul(d, geom::sinf(a)));
    v[1] = geom::sub(v[1], geom::mul(d, geom::cosf(a)));
    v
}

/// What the party instructions act on.
pub struct EvParty<'a> {
    pub scene: &'a mut Scene,
    pub crew: &'a mut Crew,
    pub roster: &'a Roster,
    /// The entry control, for `GetEnemy` and `GetNpc`.
    pub ctrl: &'a EntryCtrl,
    /// The enemies' own state by scene index (their heading).
    pub foes: &'a mut [Option<Enemy>],
    /// `HitEnable`, and the player's frame (`ccTransPosW2P`/`P2W`).
    pub world: &'a mut dyn World,
    /// `game.area` (+0x14): 0 a Root Town, 1 a field, 2 a dungeon.
    pub area: i32,
    /// `eventMng.evPos[16]` in order (`set_pos`, `marker_pos`).
    pub positions: &'a [EvPos],
    /// A Root Town marker: `markerEvTbl[marker]`'s dummy in the town's file
    /// (`GetChunkAdrsF`): its position (+0x10) and `rot.z` (+0x28). The game
    /// reads through a null chunk when there is none; the port does
    /// nothing then.
    pub town_marker: &'a dyn Fn(i16) -> Option<(V4, F)>,
    /// `ccDeleteCmnd`s of characters that were listed
    /// ([`Out::DeleteCmnd`]: `ccChangeCmndTarget(0)` when it was the
    /// command target).
    pub out: &'a mut Vec<Out>,
}

impl EvParty<'_> {
    fn dirc(&self, ch: usize) -> V4 {
        self.crew.spc.get(&ch).map_or([0; 4], |s| s.dirc)
    }

    /// `ccSpcChar::ManualModeAI(ev)` (gcmn 0x0059eba0): with `ev` 1 and the
    /// party not wiped out (`checkPartyAnnihilation`), a dead character
    /// (`dead` not 0) is brought back - `dead` 0, `cloak` 1.0, `ghostFlag`
    /// off, `bodyHit` into the world (`HitEnable`) - and one in acts 9-11
    /// stands (act 2, old act -1); then `ccAI::ManualMode` when it has an
    /// AI: `manualSW`, `gDeg` its heading (`RAD2DEG`), `gRotSp` 64,
    /// `remoteCmd` 0.
    pub fn manual_mode_ai(&mut self, ch: usize, ev: i32) {
        if ev == 1 && !self.roster.party.annihilated(self.scene) {
            if self.scene.chars[ch].cond[cond::DEAD] != 0 {
                let c = &mut self.scene.chars[ch];
                c.cond[cond::DEAD] = 0;
                c.spc_char.cloak = ONE;
                c.spc_char.flags &= !spc_flag::GHOST;
                let s = self.crew.spc.entry(ch).or_default();
                s.ghost = false;
                let mut hit = s.body_hit;
                self.world.hit_switch(ch, &mut hit, true);
                self.crew.spc.entry(ch).or_default().body_hit = hit;
                self.scene.chars[ch].spc_char.hit_enabled = hit.sw;
            }
            let act = self.scene.chars[ch].spc_char.act_num;
            if matches!(act, 9..=11) {
                self.scene.chars[ch].spc_char.act_num = 2;
                self.scene.chars[ch].spc_char.act_num_old = -1;
                self.crew.spc.entry(ch).or_default().act_num = 2;
            }
        }
        self.manual_mode(ch);
    }

    /// `ccAI::ManualMode()` (gcmn 0x00583270) of the character's AI, if it
    /// has one.
    pub fn manual_mode(&mut self, ch: usize) {
        let Some(body) = self.crew.ais.get(&ch).map(|a| a.body) else { return };
        let deg = geom::rad2deg(self.dirc(body)[2]) as u16;
        let a = self.crew.ais.get_mut(&ch).expect("checked above");
        a.manual_sw = true;
        a.g_deg = deg;
        a.g_rot_sp = 64;
        a.remote_cmd = 0;
    }

    /// `ccAI::SetRemoteCmd(cmd)` (gcmn 0x005832e0) on the character's AI:
    /// `remoteCmd`, `remoteFlag`; 0 takes `gDeg` from the heading, 5 marks a
    /// body of no party (`partyFlag` 0) as leaving (-2). The game has no
    /// case without an AI (it writes through null); the port does nothing.
    pub fn set_remote_cmd(&mut self, ch: usize, cmd: i32) {
        let Some(body) = self.crew.ais.get(&ch).map(|a| a.body) else { return };
        let deg = geom::rad2deg(self.dirc(body)[2]) as u16;
        let a = self.crew.ais.get_mut(&ch).expect("checked above");
        a.remote_cmd = cmd as i16;
        a.remote_flag = true;
        if cmd == 0 {
            a.g_deg = deg;
        } else if cmd == 5 {
            let c = &mut self.scene.chars[body];
            if ((c.party_flag & 7) << 29) >> 29 == 0 {
                c.party_flag = 6;
            }
        }
    }

    /// `ccAI::SetGoalPos(v, 0)` (gcmn 0x005833a0): `gPoint` -1, `gPos` v.
    pub fn set_goal_pos(&mut self, ch: usize, v: V4) {
        if let Some(a) = self.crew.ais.get_mut(&ch) {
            a.g_point = -1;
            a.g_pos = v;
        }
    }

    /// `ccAI::SetDircZ(dd)` (gcmn 0x00581500) of the character's AI: `gDeg`
    /// dd, its body's heading `DEG2RAD(dd)` at once. A character without an
    /// AI is not turned (the game writes through the null AI).
    pub fn set_dirc_z(&mut self, ch: usize, dd: u16) {
        let Some(a) = self.crew.ais.get_mut(&ch) else { return };
        a.g_deg = dd;
        let body = a.body;
        self.crew.spc.entry(body).or_default().dirc[2] = geom::deg2rad(dd as i16);
    }

    /// A walk's common start: manual control, the remote command.
    fn remote(&mut self, ch: usize, cmd: i32) {
        self.manual_mode_ai(ch, 1);
        self.set_remote_cmd(ch, cmd);
    }

    /// `pc_command pc on` (case 66, main 0x001ae164): each character
    /// [`Roster::commanded`] names onto its command list (`ccEntryCmnd`,
    /// gcmn 0x00519630) or, with `on` 0, off it (`ccDeleteCmnd`,
    /// 0x00519700). True when someone was named.
    pub fn pc_command(&mut self, pc: i16, on: i16) -> bool {
        let who = self.roster.commanded(pc);
        for &c in &who {
            if on != 0 {
                entry::entry_cmnd(self.scene, c);
            } else {
                entry::delete_cmnd_char(self.scene, self.out, c);
            }
        }
        !who.is_empty()
    }

    /// `pc_walk_pos pc x y z` (case 61, main 0x001ad8ac; `run` false,
    /// remote command 1) and `pc_run_pos` (case 164, 0x001ad9b0; `run`
    /// true, remote command 2): [`Roster::named`]'s character walks to
    /// `(10 x, 10 y, 10 z, 1)`.
    pub fn pc_walk_pos(&mut self, pc: i16, x: i16, y: i16, z: i16, run: bool) -> bool {
        let Some(c) = self.roster.named(pc) else { return false };
        self.remote(c, if run { 2 } else { 1 });
        self.set_goal_pos(c, scaled(x, y, z));
        true
    }

    /// `pc_walk_dir pc rot dist` (case 62, main 0x001adab0):
    /// [`Roster::registered`]'s character walks `10 dist` from where it
    /// stands toward `rot` (the game's 16-bit angle).
    pub fn pc_walk_dir(&mut self, pc: i16, rot: i16, dist: i16) -> bool {
        let Some(c) = self.roster.registered(pc) else { return false };
        self.remote(c, 1);
        let v = offset(self.scene.chars[c].pos, rot, dist);
        self.set_goal_pos(c, v);
        true
    }

    /// `pc_walk_marker pc marker` (case 63, main 0x001adbf4):
    /// [`Roster::registered`]'s character walks to the marker: in a field
    /// or a dungeon the first event position numbered `marker`, in a Root
    /// Town the marker's dummy, w 1.0. With no such position (or in another
    /// area) the game walks to what its stack held; the port to the origin.
    pub fn pc_walk_marker(&mut self, pc: i16, marker: i16) -> bool {
        let Some(c) = self.roster.registered(pc) else { return false };
        self.remote(c, 1);
        let mut v = match self.area {
            1 | 2 => self.positions.iter().take(16).find(|p| p.num == i32::from(marker)).map(|p| p.pos),
            0 => (self.town_marker)(marker).map(|(p, _)| p),
            _ => None,
        }
        .unwrap_or([0; 4]);
        v[3] = ONE;
        self.set_goal_pos(c, v);
        true
    }

    /// `pc_walk_char pc type code rot dist` (case 64, main 0x001adda4):
    /// [`Roster::registered`]'s character walks to a point `10 dist` toward
    /// `rot` from a character: by `1 << type` (the shift's low five bits),
    /// 4 `GetSpc(code)`, 0x18 `GetNpc(code)`, 0x60 `GetEnemy(code)`. With no
    /// such character the walk is set up (manual control, remote command 1)
    /// but the goal is left as it was. True when the goal was set.
    pub fn pc_walk_char(&mut self, pc: i16, ty: i16, code: i16, rot: i16, dist: i16) -> bool {
        let Some(c) = self.roster.registered(pc) else { return false };
        self.remote(c, 1);
        let Some(t) = self.find_char(ty, code) else { return false };
        let v = offset(self.scene.chars[t].pos, rot, dist);
        self.set_goal_pos(c, v);
        true
    }

    /// The character an instruction's `type code` pair names by `1 << type`
    /// (`sllv`: the low five bits): 4 `GetSpc`, 0x18 `GetNpc`, 0x60
    /// `GetEnemy`.
    pub fn find_char(&self, ty: i16, code: i16) -> Option<usize> {
        let bit = 1u32 << (ty as u32 & 31);
        let code = i32::from(code);
        if bit & 4 != 0 {
            self.roster.get_spc(code)
        } else if bit & 0x18 != 0 {
            get_npc(self.ctrl, self.scene, code)
        } else if bit & 0x60 != 0 {
            get_enemy(self.ctrl, self.scene, code)
        } else {
            None
        }
    }

    /// `pc_put pc x y z` (case 70, main 0x001aeb84): [`Roster::named`]'s
    /// character's position becomes `(10 x, 10 y, 10 z, 1)`; nothing else
    /// (not its heading, not the ground).
    pub fn pc_put(&mut self, pc: i16, x: i16, y: i16, z: i16) -> bool {
        let Some(c) = self.roster.named(pc) else { return false };
        self.scene.chars[c].pos = scaled(x, y, z);
        true
    }

    /// `party_put pc x y z` (case 69, main 0x001aec5c): the same for
    /// [`Roster::companion`].
    pub fn party_put(&mut self, pc: i16, x: i16, y: i16, z: i16) -> bool {
        let Some(c) = self.roster.companion(self.scene, pc) else { return false };
        self.scene.chars[c].pos = scaled(x, y, z);
        true
    }

    /// Placed at a marker: the position, then `SetDircZ(RAD2DEG(rot))`.
    fn place(&mut self, c: usize, pos: V4, rot: F) {
        self.scene.chars[c].pos = pos;
        self.set_dirc_z(c, geom::rad2deg(rot) as u16);
    }

    /// `ccEntryEventMng` (main 0x001b62e0) for an event's `entry TYPE CODE
    /// MARKER PARAM` of a party character (types 0-2) in a field or a
    /// dungeon: the character of the registry slot whose id is `code` put at
    /// the event position numbered `marker` (Kite's position `kite` added
    /// when that position's floor and block are 9999 or more; a negative
    /// marker, or none of that number, the origin), facing its `dirc`
    /// (`SetDircZ(RAD2DEG(dirc))`); with `param` 5 onto its command list
    /// (`ccEntryCmnd`). False when `code` has no character.
    pub fn place_entry(&mut self, code: i16, marker: i16, param: i16, kite: V4) -> bool {
        let Some(c) = self.roster.registered(code) else { return false };
        let p = if marker >= 0 { entry::event_pos(self.positions, marker, kite) } else { None };
        let (pos, dirc) = p.map_or(([0, 0, 0, ONE], 0), |p| (p.pos, p.dirc));
        self.place(c, pos, dirc);
        if param == 5 {
            entry::entry_cmnd(self.scene, c);
        }
        true
    }

    /// `pc_put_marker pc marker` (case 67, main 0x001ae7d8):
    /// [`Roster::named`]'s character put at a marker, facing its way. In a
    /// field or a dungeon: the first event position numbered `marker` (its
    /// position and `dirc`; none, nothing); a negative `marker` instead puts
    /// the character at `(-200 sinf(a), 200 cosf(a))` in the player's frame
    /// (its z and w kept), `a` = [`get_dirc_pl`] of where it stands, back in
    /// the world (`ccTransPosP2W`), its heading unchanged. In a Root Town:
    /// the marker's dummy. Elsewhere nothing.
    pub fn pc_put_marker(&mut self, pc: i16, marker: i16) -> bool {
        let Some(c) = self.roster.named(pc) else { return false };
        match self.area {
            1 | 2 if marker >= 0 => self.put_at_position(c, marker),
            1 | 2 => {
                let mut v = self.scene.chars[c].pos;
                let a = get_dirc_pl(self.world, v);
                v[0] = geom::mul(KM200, geom::sinf(a));
                v[1] = geom::mul(K200, geom::cosf(a));
                self.scene.chars[c].pos = self.world.p2w(v);
                true
            }
            0 => self.put_at_town_marker(c, marker),
            _ => false,
        }
    }

    /// `party_put_marker pc marker` (case 68, main 0x001ae9e8):
    /// [`Roster::companion`] put at the first event position numbered
    /// `marker` (any number, negative ones included) in a field or a
    /// dungeon, or at the marker's dummy in a Root Town.
    pub fn party_put_marker(&mut self, pc: i16, marker: i16) -> bool {
        let Some(c) = self.roster.companion(self.scene, pc) else { return false };
        match self.area {
            1 | 2 => self.put_at_position(c, marker),
            0 => self.put_at_town_marker(c, marker),
            _ => false,
        }
    }

    fn put_at_position(&mut self, c: usize, num: i16) -> bool {
        let Some(p) = self.positions.iter().take(16).find(|p| p.num == i32::from(num)).copied() else {
            return false;
        };
        self.place(c, p.pos, p.dirc);
        true
    }

    fn put_at_town_marker(&mut self, c: usize, marker: i16) -> bool {
        let Some((pos, rot)) = (self.town_marker)(marker) else { return false };
        self.place(c, pos, rot);
        true
    }

    /// The turn of `pc_turn` and `pc_face` on the character's AI: `chg` 0
    /// turns at once (`SetDircZ`); otherwise `gRotSp` = `chg` (1 meaning
    /// 64) and `gDeg` = `dd`, which `ManualControl`'s remote command 0 turns
    /// toward. Nothing without an AI (the game writes through null).
    fn turn(&mut self, c: usize, dd: u16, chg: i16) {
        if chg == 0 {
            self.set_dirc_z(c, dd);
        } else if let Some(a) = self.crew.ais.get_mut(&c) {
            a.g_rot_sp = if chg == 1 { 64 } else { chg };
            a.g_deg = dd;
        }
    }

    /// `pc_turn pc dirc chg` (case 71, main 0x001aed5c): [`Roster::named`]'s
    /// character turns to `dirc` (the game's 16-bit angle), at once for
    /// `chg` 0, else at `chg` a frame.
    pub fn pc_turn(&mut self, pc: i16, dirc: i16, chg: i16) -> bool {
        let Some(c) = self.roster.named(pc) else { return false };
        self.turn(c, dirc as u16, chg);
        true
    }

    /// `pc_face pc type code chg` (case 72, main 0x001aee30):
    /// [`Roster::named`]'s character turns toward the character `type code`
    /// names ([`EvParty::find_char`]): the heading `RAD2DEG(ccGetDirc(its
    /// position, the other's))`, taken as `pc_turn` takes it. Nothing when
    /// either is missing.
    pub fn pc_face(&mut self, pc: i16, ty: i16, code: i16, chg: i16) -> bool {
        let Some(c) = self.roster.named(pc) else { return false };
        let Some(t) = self.find_char(ty, code) else { return false };
        let d = geom::get_dirc(self.scene.chars[c].pos, self.scene.chars[t].pos);
        self.turn(c, geom::rad2deg(d) as u16, chg);
        true
    }

    /// `enemy_put enemy posnum` (case 74, main 0x001af028): `GetEnemy(enemy)`
    /// to the first event position numbered `posnum`: its position, and the
    /// heading `(0, 0, dirc, 1)`.
    pub fn enemy_put(&mut self, enemy: i16, posnum: i16) -> bool {
        let Some(e) = get_enemy(self.ctrl, self.scene, i32::from(enemy)) else { return false };
        let Some(p) = self.positions.iter().take(16).find(|p| p.num == i32::from(posnum)).copied() else {
            return false;
        };
        self.scene.chars[e].pos = p.pos;
        if let Some(Some(f)) = self.foes.get_mut(e) {
            f.dirc = [0, 0, p.dirc, ONE];
        }
        true
    }
}

/// `remove 5|6 code` (case 13, main 0x001aa290, types 5 and 6):
/// `GetEnemy(code)` deleted from the entry control (`deleteEnemy`, gcmn
/// 0x004312d0: [`EntryCtrl::delete_enemy`]). Returns whom. (`remove -1`
/// is `deleteAllObject`, gcmn 0x00430e70; type 2 takes a party member out,
/// which is the registry's.)
pub fn remove_enemy(ctrl: &mut EntryCtrl, cx: &mut Cx, code: i16) -> Option<usize> {
    let e = get_enemy(ctrl, cx.scene, i32::from(code))?;
    ctrl.delete_enemy(cx, e);
    Some(e)
}

/// The `ccThEvHold` task (main 0x001b5040, "EVENT HOLD"), which `hold type
/// code` starts at priority 33 (`ccStartThread(ccThEvHold, 33, 2048)`,
/// kept in `eventMng.holdTscb` +0x7c0 with `type` and `code` in the tscb's
/// +0x14 and +0x18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hold {
    pub ty: i32,
    pub code: i32,
    /// The task has made its first `Breath`: from the next frame on it
    /// acts. False in the frame `hold` ran.
    pub armed: bool,
}

/// `hold type code` (case 132, main 0x001b09fc): starts the task unless one
/// runs already (then nothing, the new type and code ignored).
pub fn hold(task: &mut Option<Hold>, ty: i16, code: i16) {
    if task.is_none() {
        *task = Some(Hold { ty: i32::from(ty), code: i32::from(code), armed: false });
    }
}

/// `hold_end` (case 133, main 0x001b0a68): `ccDeleteThread` of the task and
/// `holdTscb` 0. The task is at its `Breath` when this runs (the event task
/// is priority 32), so it does not act in this frame.
pub fn hold_end(task: &mut Option<Hold>) {
    *task = None;
}

/// What `ccThEvHold`'s type 7 reads of the event's boss:
/// `eventMng.bossEntry` (+0x08) and the param of `eventMng.bossTscb`
/// (+0x7bc, its +0x14; None without the task).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Boss {
    pub entry: i32,
    pub task_param: Option<i32>,
}

/// One frame of `ccThEvHold` (priority 33: after `ccThEvent` (32),
/// `ccThGameCtrl` and any event camera task started before it (both 33),
/// before `ccThMenu` (34), the camera (40), the party (48-50) and the entry
/// control (64)). In the frame the task started it only sets itself up.
/// Then each frame, by type:
///
/// - 5, 6: each object of the entry control's enemy list (walked for the
///   count it has at the start) whose base type has bit `1 << type` and
///   whose base id is `code` takes `EntryAffect(0, 5, 0, 0, 0)` (hold:
///   `cond.hold` 1, [`affect::entry_affect`], nothing for one off the
///   command lists or drained), and for `code` 130 (the tutorial's
///   goblins) turns at once to Kite: its heading `(0, 0,
///   ccGetDirc(its position, Kite's), 1)`;
/// - 7: when `bossEntry` is `code` and the boss task's param is not 0, the
///   boss `ccCheckTargetTypeId(0x80, code)` finds takes the same hold;
/// - anything else: nothing.
///
/// `kite` is `plw.pw` (the game reads through it without a check).
#[allow(clippy::too_many_arguments)]
pub fn hold_frame(
    task: &mut Hold,
    t: &Tables,
    scene: &mut Scene,
    ctrl: &EntryCtrl,
    foes: &mut [Option<Enemy>],
    kite: Option<usize>,
    boss: Boss,
    ctx: &AffectCtx,
    rng: &mut dyn Rng,
    ev: &mut Events,
) {
    if !task.armed {
        task.armed = true;
        return;
    }
    match task.ty {
        5 | 6 => {
            let bit = 1i32 << (task.ty & 31);
            let l = ctrl.lists[Kind::Enemy as usize];
            let mut cur = l.head;
            for _ in 0..l.num {
                let Some(c) = cur else { break };
                if scene.chars[c].ty() & bit != 0 && i32::from(scene.chars[c].id()) == task.code {
                    affect::entry_affect(t, scene, ctx, c, None, 5, [0; 3], rng, ev);
                    if task.code == 130
                        && let Some(k) = kite
                    {
                        let d = geom::get_dirc(scene.chars[c].pos, scene.chars[k].pos);
                        if let Some(Some(f)) = foes.get_mut(c) {
                            f.dirc = [0, 0, d, ONE];
                        }
                    }
                }
                cur = ctrl.links.get(c).and_then(|l| l.next);
            }
        }
        7 if boss.entry == task.code && boss.task_param.is_some_and(|p| p != 0) => {
            if let Some(b) = check_target_type_id(scene, 1 << 7, task.code) {
                affect::entry_affect(t, scene, ctx, b, None, 5, [0; 3], rng, ev);
            }
        }
        _ => {}
    }
}

/// The `ccMenuCtrl` members and globals `player_skill` sets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SkillMenu {
    /// `panelStatus` (+0x0c) and `mapStatus` (+0x10): 0 shut, 1 fading
    /// in, 2 open, 3 fading out.
    pub panel_status: i16,
    pub map_status: i16,
    /// `plAttack` (+0xf4): the attack button is down this frame.
    pub pl_attack: i16,
    /// `targetForbid` (+0x102).
    pub target_forbid: i16,
    /// `cmndTargetFix` (gcmn 0x00378c6c): the command target held.
    pub cmnd_target_fix: i32,
}

/// `player_skill`'s set-up (case 153, main 0x001b1f04): the party panels
/// fade in (`panelStatus` 1), targeting allowed (`targetForbid` 0), the
/// command target held (`cmndTargetFix` 1), and the target's
/// `entParam.param[2]` (`ccEntryObj` +0x150) cleared: an enemy with it 0
/// leaves no treasure box when it is taken away
/// ([`EntryCtrl::entry_enemy_object`]). The game has no case without a
/// target (it writes through null).
pub fn player_skill_begin(
    menu: &mut SkillMenu,
    cmnd_target: Option<usize>,
    ctrl: &mut EntryCtrl,
    foes: &mut [Option<Enemy>],
) {
    menu.panel_status = 1;
    menu.target_forbid = 0;
    menu.cmnd_target_fix = 1;
    let Some(c) = cmnd_target else { return };
    if let Some(Some(f)) = foes.get_mut(c) {
        f.ent.param[2] = 0;
    } else if let Some(o) = ctrl.entry_obj_mut(c) {
        o.ent.param[2] = 0;
    }
}

/// `player_skill`'s wait: it goes on while the command target (read again
/// each frame) is alive (`condition.dead` 0). Tested at once after the
/// set-up, then after each frame's step; the first step is a frame after
/// the set-up (`ccBreathThread(1)` first).
pub fn player_skill_busy(scene: &Scene, cmnd_target: Option<usize>) -> bool {
    cmnd_target.is_some_and(|c| scene.chars[c].cond[cond::DEAD] == 0)
}

/// One frame of `player_skill`, decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillStep {
    /// Kite's skill runs (`ccSkillCheck(plw)` not 0): nothing.
    Busy,
    /// The confirm button was pushed: `ccSkillRequest(plw, cmndTarget, 1)`
    /// (a normal attack on the command target) and `plAttack` 1.
    Attack,
    /// `plAttack` 0.
    Idle,
}

/// `player_skill`'s step: `skill_check` is `ccSkillCheck(plw)` (gcmn
/// 0x005723e0, [`crate::flow::Skills::check`]), `push` the pad's pushed
/// buttons (`ccSys.pad[0].push`, +0x2d0) and `assign_ok` the save's
/// confirm button (`saveData.assignPADok`, +0x840e, sign extended into the
/// mask).
pub fn player_skill_step(skill_check: i32, push: u32, assign_ok: i16) -> SkillStep {
    if skill_check != 0 {
        SkillStep::Busy
    } else if push & (i32::from(assign_ok) as u32) != 0 {
        SkillStep::Attack
    } else {
        SkillStep::Idle
    }
}

/// The step's effect on the menu: `plAttack`. True when the runtime is to
/// make the skill request.
pub fn player_skill_apply(menu: &mut SkillMenu, step: SkillStep) -> bool {
    match step {
        SkillStep::Busy => false,
        SkillStep::Attack => {
            menu.pl_attack = 1;
            true
        }
        SkillStep::Idle => {
            menu.pl_attack = 0;
            false
        }
    }
}

/// `player_skill`'s end: the target let go (`cmndTargetFix` 0),
/// `plAttack` 0, the panels and the minimap fading out (`panelStatus`,
/// `mapStatus` 3).
pub fn player_skill_end(menu: &mut SkillMenu) {
    menu.cmnd_target_fix = 0;
    menu.pl_attack = 0;
    menu.panel_status = 3;
    menu.map_status = 3;
}

/// `battle_ready` (case 163, main 0x001b23a8): `game.inBattleDist` (+0x60)
/// = -1.0 (and `AddOperate(14, 0, 0)`, the interpreter's). `ccThGameCtrl`
/// (gcmn 0x00517c54) then sets `inBattle` whenever an enemy is listed and
/// `game.field` (+0x24) is 1-12; on other fields a negative distance
/// finds no enemy near (-2.0 would force the fight anywhere).
pub fn battle_ready(in_battle_dist: &mut F) {
    *in_battle_dist = geom::MINUS_ONE;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roster() -> Roster {
        let mut r = Roster::default();
        r.ids[0] = 0;
        r.chars[0] = Some(0);
        r.ids[1] = 2;
        r.chars[1] = Some(1);
        r.party = Party { members: [Some(0), Some(1), None], ids: [0, 2, -1], num: 2 };
        r
    }

    #[test]
    fn who_pc_names() {
        let r = roster();
        assert_eq!(r.get_spc(2), Some(1));
        assert_eq!(r.get_spc(-1), Some(1));
        // -3 reads num (2): Orca's id.
        assert_eq!(r.get_spc(-3), Some(1));
        assert_eq!(r.named(-1), Some(1));
        assert_eq!(r.named(-3), None);
        assert_eq!(r.registered(2), Some(1));
        assert_eq!(r.registered(-1), None);
        assert_eq!(r.commanded(-3), vec![0, 1]);
        assert_eq!(r.commanded(-2), Vec::<usize>::new());
    }

    #[test]
    fn player_skill_steps() {
        assert_eq!(player_skill_step(3, 0x40, 0x40), SkillStep::Busy);
        assert_eq!(player_skill_step(0, 0x40, 0x40), SkillStep::Attack);
        assert_eq!(player_skill_step(0, 0x20, 0x40), SkillStep::Idle);
        // A negative button word sign-extends into the high bits.
        assert_eq!(player_skill_step(0, 0x8000_0000, -1), SkillStep::Attack);
        let mut m = SkillMenu::default();
        assert!(player_skill_apply(&mut m, SkillStep::Attack));
        assert_eq!(m.pl_attack, 1);
        player_skill_end(&mut m);
        assert_eq!((m.pl_attack, m.panel_status, m.map_status, m.cmnd_target_fix), (0, 3, 3, 0));
    }

    #[test]
    fn a_hold_ignores_a_second_one() {
        let mut h = None;
        hold(&mut h, 5, 130);
        hold(&mut h, 6, 7);
        assert_eq!(h, Some(Hold { ty: 5, code: 130, armed: false }));
        hold_end(&mut h);
        assert_eq!(h, None);
    }

    #[test]
    fn scaled_and_offset() {
        assert_eq!(scaled(1, -2, 0), [0x4120_0000, 0xc1a0_0000, 0, ONE]);
        // rot 0 faces -y: dist 14 moves y by -140.
        let v = offset([0, 0, 0, ONE], 0, 14);
        assert_eq!(v[1], geom::k(-140.0));
    }
}
