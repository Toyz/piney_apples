//! A skill's life (`ccSkill`, skill.cpp): the request that starts it, the
//! frames it runs for (`ccThSkill` 0x005722e0 runs each [`SkillRun`]'s
//! `ccSkill::Main` 0x005731d0 in list order) and the moments it takes effect;
//! and the normal attack's hit, which Kite's and a party member's animation
//! notes decide. An attack spell's element systems belong to the effects and
//! are the runtime's ([`SpellSystem`]); the animation is the runtime's too and
//! tells [`SkillRun::main`] whether it finished and which notes it passed. The
//! rules are in docs/engine/battle.md ("A skill's life").

use piney_data::field::ee;
use piney_data::volume::Volume;

use crate::chara::{Char, Env};
use crate::damage::{self, Roll};
use crate::event::{Event, Events, Who};
use crate::geom;
use crate::param::{F_ONE, cond, ty};
use crate::rand::Rng;
use crate::scene::Scene;
use crate::skill::{self, ModifyEvents, bits};
use crate::tables::Tables;

/// How a skill was started: `stype`, the low four bits of `ccSkill`'s flag
/// byte.
pub mod stype {
    /// A character's own skill (`ccSkillRequest`, `ccSkillRequestParam`).
    pub const SKILL: i8 = 0;
    /// An item's effect with no caster action (`ccItemSkillRequest(...,
    /// 0)`).
    pub const ITEM: i8 = 1;
    /// An item used through the caster (`ccItemSkillRequest(..., 1)`).
    pub const ITEM_CAST: i8 = 2;
}

/// A running skill: `ccSkill` (0xb0 bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillRun {
    /// +0x00: the skill id.
    pub id: i32,
    /// +0x0c bits 0-3: see [`stype`].
    pub stype: i8,
    /// +0x0c bits 4-5 (signed): 0 running, 1 ended, -1 cancelled.
    pub status: i8,
    /// +0x0c bit 6: a condition or buff skill, run by
    /// `ConditionModifySystem`.
    pub modify: bool,
    /// +0x0c bit 7: `ccSkillCheck` reports the skill whatever its type.
    pub report: bool,
    /// +0x0d bit 0: the targets are held while it runs (`ccSkillHold`).
    pub hold: bool,
    /// +0x0d bit 1: a condition takes without the resistance roll
    /// (`ccItemSkillCompel`).
    pub force: bool,
    /// +0x10: the spell's level within its element (the systems' own).
    pub level: i32,
    /// +0x14: frames run.
    pub count: i16,
    /// +0x24: the skill's type bits; +0x28 the target's type (for the
    /// area around the target's last position); +0x2c the item's amount.
    pub ty: i32,
    pub target_type: i32,
    pub param: i32,
    /// +0x30, +0x40: the caster's position and direction (float bits),
    /// +0x50 its height, +0x60 the target's position.
    pub pos: [u32; 4],
    pub dirc: [u32; 4],
    pub height: u32,
    pub target_pos: [u32; 4],
    /// +0x54: the attribute-critical flag.
    pub ac_flag: i16,
    /// +0x70, +0x74: caster and target (scene indices).
    pub creator: Option<usize>,
    pub target: Option<usize>,
    /// +0x80: the skill has its own animation (an effect file).
    pub has_anm: bool,
    /// Not the game's: the run's name for the runtime, given by
    /// [`Skills::request`], to tie its animation and effects to it.
    pub key: u32,
}

/// `ccSkill::ccSkill(sid)` (gcmn 0x00572f80): everything clear, height
/// 200.0.
impl SkillRun {
    pub fn new(id: i32) -> SkillRun {
        SkillRun {
            id,
            stype: 0,
            status: 0,
            modify: false,
            report: false,
            hold: false,
            force: false,
            level: 0,
            count: 0,
            ty: 0,
            target_type: 0,
            param: 0,
            pos: [0, 0, 0, F_ONE],
            dirc: [0; 4],
            height: 0x4348_0000,
            target_pos: [0, 0, 0, F_ONE],
            ac_flag: 0,
            creator: None,
            target: None,
            has_anm: false,
            key: 0,
        }
    }
}

/// What the runtime supplies a frame of [`SkillRun::main`].
pub struct Frame<'a> {
    pub env: &'a Env,
    /// Converts a world position to the player's frame (`ccTransPosW2P`,
    /// `ccPlayer::W2PPos`, gcmn 0x0059b940), for the area around the
    /// target's last position.
    pub w2p: &'a dyn Fn([u32; 4]) -> [u32; 4],
    /// The skill's animation this frame, when it has one: whether it
    /// finished (`ccAnm::_AnimateForward`), and the notes it passed
    /// (`ccAnm::NoteProcess`, type and value), in order.
    pub anim_done: bool,
    pub notes: &'a [(u32, u32)],
    /// `checkPartyAnnihilation()`: Resurrect does nothing then.
    pub annihilated: bool,
}

/// An attack spell's element system, which the runtime runs with the
/// spell's effects: which one, for which skill of the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellSystem {
    /// `ccSkill::FallSystem` (gcmn 0x00577a30): meteors
    /// (`effMeteoFireBall2`, `ccFallElementGenerate` from level 3); each
    /// lands with `ccSkillDamage2` around the target's position.
    Fall,
    /// `ccSkill::TornadoSystem` (0x00577540): levels 1-2 hit nine times,
    /// every 5 frames from frame 35 (`ccSkillDamage`); from level 3
    /// `ccSkillTornadeElementsGenerate`'s elements hit.
    Tornado,
    /// `ccSkill::ConvergenceSystem` (0x00578060).
    Convergence,
    /// `ccSkill::UpheavalSystem` (0x005787c0).
    Upheaval,
    /// `ccSkill::SummonsSystem` (0x00579000).
    Summons,
}

/// The system `ccSkill::Main` runs for a spell (its jump table at gcmn
/// 0x006e1210, by skill id).
pub fn spell_system(sid: i32) -> Option<SpellSystem> {
    use SpellSystem::*;
    Some(match sid {
        193..=196 | 225..=228 | 257..=260 | 273..=276 => Fall,
        197..=200 | 209..=212 | 229..=232 | 241..=244 | 261..=264 => Tornado,
        213..=216 | 233..=236 | 245..=248 | 265..=268 | 277..=280 => Convergence,
        201..=204 | 217..=220 | 249..=252 | 281..=284 => Upheaval,
        205..=208 | 221..=224 | 237..=240 | 253..=256 | 269..=272 | 285..=294 => Summons,
        _ => return None,
    })
}

/// What a frame of `ccSkill::Main` asked of the runtime besides the
/// events.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MainOut {
    /// `ccSkillModifyCondition`'s starts and resists.
    pub modify: ModifyEvents,
    /// A spell's element system to run this frame.
    pub spell: Option<SpellSystem>,
    /// `ccSeOn3D(75, target position)`: the healing sound.
    pub heal_sound: bool,
    /// `effPhysicalSkillHitShockWave(target position)` notes (0x8002).
    pub shock_waves: u32,
    /// `ccSeSetParamSPC(value, caster)` notes (types 1 and 2) of a
    /// party caster.
    pub sounds: Vec<u32>,
}

fn clear_skill(ch: &mut Char) {
    ch.skill_id = 0;
    ch.skill_status = 0;
}

impl SkillRun {
    fn casting(&self) -> bool {
        self.stype == stype::SKILL || self.stype == stype::ITEM_CAST
    }

    fn end(&mut self) {
        self.status = 1;
    }

    /// Clears the caster's skill fields when it cast the skill itself.
    fn release(&self, scene: &mut Scene) {
        if let Some(c) = self.creator
            && self.casting()
        {
            clear_skill(&mut scene.chars[c]);
        }
    }

    /// `ccSkill::Main` (gcmn 0x005731d0), one frame. Returns the status:
    /// non-zero means the skill is over and is deleted.
    #[allow(clippy::too_many_arguments)]
    pub fn main(
        &mut self,
        t: &Tables,
        scene: &mut Scene,
        f: &Frame,
        rng: &mut dyn Rng,
        ev: &mut Events,
        out: &mut MainOut,
    ) -> i8 {
        if self.status != 0 {
            return self.status;
        }
        if let Some(c) = self.creator {
            let d = scene.chars[c].cond[cond::DEAD];
            if !scene.listed(c) || !(d == 0 || d == 1) {
                if self.id == 1 {
                    clear_skill(&mut scene.chars[c]);
                }
                self.creator = None;
            }
        }
        if let Some(c) = self.creator
            && scene.listed(c)
        {
            self.pos = scene.chars[c].pos;
        }
        if self.casting() {
            let gone = match self.creator {
                Some(c) => !scene.listed(c) || scene.chars[c].dead(),
                None => true,
            };
            if gone && t.skill(self.id).is_some_and(|s| s.ty & bits::PHYSICAL != 0) {
                self.status = -1;
                return self.status;
            }
        }
        if let Some(tg) = self.target {
            let goes_on = t.skill(self.id).is_some_and(|s| s.ty & bits::UNTARGETED != 0);
            if scene.listed(tg) {
                if !scene.chars[tg].dead() || goes_on || self.id == skill::RESURRECT || self.has_anm || self.id == 1 {
                    self.target_pos = scene.chars[tg].pos;
                } else {
                    self.end();
                    if self.creator.is_some() {
                        self.release(scene);
                    }
                    return self.status;
                }
            } else if goes_on || self.has_anm || self.id == 1 {
                self.target = None;
            } else {
                self.end();
                if self.creator.is_some() {
                    self.release(scene);
                }
                return self.status;
            }
        }
        if self.has_anm {
            for &(kind, value) in f.notes {
                self.check_note(t, scene, f, kind, value, rng, ev, out);
            }
            if f.anim_done {
                if self.creator.is_some() {
                    self.release(scene);
                }
                self.end();
                self.hold = false;
            }
        }
        if self.modify {
            self.condition_modify_system(t, scene, rng, ev, out);
        } else if skill::check_type_of(t, self.id) == 0 {
            if self.id == 1 {
                if let Some(c) = self.creator
                    && scene.listed(c)
                    && self.status == 1
                {
                    clear_skill(&mut scene.chars[c]);
                }
            } else {
                if let Some(c) = self.creator
                    && scene.listed(c)
                    && scene.chars[c].ty() & 0x20 != 0
                {
                    self.end();
                }
                if self.count >= 450 {
                    if let Some(c) = self.creator
                        && scene.listed(c)
                    {
                        clear_skill(&mut scene.chars[c]);
                    }
                    self.end();
                    self.hold = false;
                }
            }
        } else if self.ty & bits::MAGIC != 0 {
            match self.id {
                150..=155 | 295 => self.healing_system(t, scene, ev, out),
                178..=180 => self.recovery_system(t, scene, f, ev),
                id => match spell_system(id) {
                    Some(s) => out.spell = Some(s),
                    None => {
                        if let Some(c) = self.creator
                            && scene.listed(c)
                            && self.casting()
                            && scene.chars[c].ty() & 5 != 0
                        {
                            clear_skill(&mut scene.chars[c]);
                        }
                        self.end();
                    }
                },
            }
        } else if self.id != 0 {
            self.end();
        }
        if self.hold {
            self.hold_targets(t, scene, f, ev);
        }
        self.count = self.count.wrapping_add(1);
        self.status
    }

    fn hold_targets(&self, t: &Tables, scene: &Scene, f: &Frame, ev: &mut Events) {
        let Some(sk) = t.skill(self.id) else { return };
        let creator = self.creator.unwrap_or(NOBODY);
        let mut e = Events::new();
        match self.target {
            Some(tg) if scene.listed(tg) => {
                skill::hold(t.volume, scene, creator, tg, sk, &mut e);
            }
            _ => {
                skill::hold_at(t.volume, scene, creator, (f.w2p)(self.target_pos), self.target_type, sk, &mut e);
            }
        }
        nobody(&mut e);
        ev.extend(e);
    }

    /// `ccSkillCheckNote(note)` (gcmn 0x00573a10): a note of the skill's
    /// animation. 0x8005 is [`SkillRun::note_event_affect`] with the
    /// note's value; 0x8002 a shock wave at the target; 1 and 2 a sound of
    /// a party caster.
    #[allow(clippy::too_many_arguments)]
    fn check_note(
        &mut self,
        t: &Tables,
        scene: &mut Scene,
        f: &Frame,
        kind: u32,
        value: u32,
        rng: &mut dyn Rng,
        ev: &mut Events,
        out: &mut MainOut,
    ) {
        match kind {
            1 | 2 => {
                if let Some(c) = self.creator
                    && scene.chars[c].ty() & ty::PC != 0
                {
                    out.sounds.push(value);
                }
            }
            0x8005 => self.note_event_affect(t, scene, f, value, rng, ev),
            0x8002 => out.shock_waves += 1,
            _ => {}
        }
    }

    /// `ccSkill::NoteEventAffect(kind)` (gcmn 0x005738e0): an animation
    /// note of the skill takes effect. 1 and 3: the skill's damage on its
    /// target ([`damage::skill_damage`]), or around where the target was
    /// if it is gone or down; 2 and 4: `EntryAffect(kind, 1)` on the
    /// target; 5 and 6 start and stop holding.
    pub fn note_event_affect(
        &mut self,
        t: &Tables,
        scene: &mut Scene,
        f: &Frame,
        kind: u32,
        rng: &mut dyn Rng,
        ev: &mut Events,
    ) {
        let Some(sk) = t.skill(self.id).cloned() else { return };
        match kind {
            1 | 3 => {
                let Some(creator) = self.creator else { return };
                match self.target {
                    Some(tg) if scene.listed(tg) && !scene.chars[tg].dead() => {
                        damage::skill_damage(t, scene, creator, tg, &sk, &mut self.ac_flag, self.id, rng, f.env, ev);
                    }
                    _ => {
                        let pos = (f.w2p)(self.target_pos);
                        damage::skill_damage_at(t, scene, creator, pos, self.target_type, &sk, self.id, rng, f.env, ev);
                    }
                }
            }
            2 | 4 => {
                if let Some(tg) = self.target
                    && scene.listed(tg)
                {
                    let by = self.creator.map_or(Who::Nobody, Who::Char);
                    ev.push(Event::affect(Who::Char(tg), by, kind as i16, 1, 0, 0));
                }
            }
            5 => self.hold = true,
            6 => self.hold = false,
            _ => {}
        }
    }

    /// `ccSkill::ConditionModifySystem` (gcmn 0x00576c50): on the first
    /// frame, the condition or buff on the target (or around where it
    /// was), a party caster of a buff or heal paying its SP; then the
    /// skill ends, unless a party caster's animation still runs.
    fn condition_modify_system(
        &mut self,
        t: &Tables,
        scene: &mut Scene,
        rng: &mut dyn Rng,
        ev: &mut Events,
        out: &mut MainOut,
    ) {
        let target_ok =
            self.target.is_some_and(|tg| scene.listed(tg) && (!scene.chars[tg].dead() || self.id == skill::RESURRECT));
        let creator_ok =
            self.stype == stype::ITEM || self.creator.is_some_and(|c| scene.listed(c) && !scene.chars[c].dead());
        if target_ok && self.count == 0 && creator_ok {
            let creator = self.creator.unwrap_or(NOBODY);
            let tg = self.target.unwrap_or(NOBODY);
            if scene.listed(tg) {
                skill::skill_modify_condition(
                    t,
                    scene,
                    creator,
                    tg,
                    self.id,
                    i32::from(self.stype),
                    self.force,
                    rng,
                    &mut out.modify,
                );
                nobody(&mut out.modify.events);
                ev.append(&mut out.modify.events);
            }
            if let Some(c) = self.creator
                && scene.listed(c)
                && self.stype == stype::SKILL
                && scene.chars[c].ty() & 5 != 0
                && self.ty & (bits::BUFF | bits::HEAL) != 0
            {
                let cost = t.skill(self.id).map_or(0, |s| s.cost);
                let v = i32::from(scene.chars[c].sp) - cost;
                scene.chars[c].sp = if v < 0 { 0 } else { v as i16 };
            }
        }
        let _ = ev;
        match self.creator {
            Some(c) if self.casting() && scene.chars[c].ty() & 0x85 != 0 => {
                let ch = &mut scene.chars[c];
                // A party caster whose animation still runs (anmFlag 0)
                // keeps the skill; a boss caster ends it at once.
                if ch.anm_flag != 0 || ch.ty() & ty::BOSS != 0 {
                    clear_skill(ch);
                    self.end();
                }
            }
            _ => self.end(),
        }
    }

    /// `ccSkill::HealingSystem` (gcmn 0x00576ea0): at frame 75 (at once for
    /// an item's), the heal on the target, or around where it was; a
    /// caster of its own skill pays the SP, and the skill ends.
    fn healing_system(&mut self, t: &Tables, scene: &mut Scene, ev: &mut Events, out: &mut MainOut) {
        if self.stype != stype::ITEM && self.count != 75 {
            return;
        }
        if let Some(tg) = self.target
            && scene.listed(tg)
        {
            out.heal_sound = true;
            let param = if self.id == 295 { self.param } else { 0 };
            let creator = self.creator.unwrap_or(usize::MAX);
            if creator != usize::MAX {
                skill::recovery(t, scene, creator, tg, self.id, param, ev);
            }
        }
        if let Some(c) = self.creator
            && self.casting()
        {
            if self.stype == stype::SKILL {
                let cost = t.skill(self.id).map_or(0, |s| s.cost);
                let v = i32::from(scene.chars[c].sp) - cost;
                scene.chars[c].sp = if v < 0 { 0 } else { v as i16 };
            }
            clear_skill(&mut scene.chars[c]);
        }
        self.end();
    }

    /// `ccSkill::RecoverySystem` (gcmn 0x00577070): the cures, see
    /// [`skill::recovery_system`].
    fn recovery_system(&mut self, t: &Tables, scene: &mut Scene, f: &Frame, ev: &mut Events) {
        let Some(tg) = self.target else { return };
        let listed = scene.listed(tg);
        let mut e = Events::new();
        let r = match self.creator {
            Some(c) if c != tg => {
                let (a, b) = pair_mut(&mut scene.chars, c, tg);
                skill::recovery_system(
                    t,
                    Some(a),
                    b,
                    self.id,
                    self.count,
                    i32::from(self.stype),
                    f.annihilated,
                    listed,
                    &mut e,
                )
            }
            // A cure on the caster itself: the target's part, then the
            // caster's on the same character.
            Some(c) => {
                let ch = &mut scene.chars[c];
                match skill::cure_target(ch, self.id, self.count, f.annihilated, listed, &mut e) {
                    None => skill::CureResult { end: false },
                    Some(pay) => skill::cure_creator(t, Some(ch), self.id, i32::from(self.stype), pay),
                }
            }
            None => skill::recovery_system(
                t,
                None,
                &mut scene.chars[tg],
                self.id,
                self.count,
                i32::from(self.stype),
                f.annihilated,
                listed,
                &mut e,
            ),
        };
        damage::place(&mut e, self.creator.unwrap_or(NOBODY), tg);
        nobody(&mut e);
        ev.extend(e);
        if r.end {
            self.end();
        }
    }
}

/// The index standing for a null character pointer in a call: the
/// events name it [`Who::Nobody`].
const NOBODY: usize = usize::MAX;

fn nobody(ev: &mut [Event]) {
    for e in ev {
        e.map_who(|w| if w == Who::Char(NOBODY) { Who::Nobody } else { w });
    }
}

/// Two distinct elements of a slice, both mutable.
fn pair_mut<T>(v: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    assert_ne!(a, b);
    if a < b {
        let (x, y) = v.split_at_mut(b);
        (&mut x[a], &mut y[0])
    } else {
        let (x, y) = v.split_at_mut(a);
        (&mut y[0], &mut x[b])
    }
}

/// The skills running, in `SkillEntryTop`'s order.
#[derive(Clone, Debug, Default)]
pub struct Skills {
    pub runs: Vec<SkillRun>,
    /// The next run's [`SkillRun::key`].
    pub next_key: u32,
    /// `xnote_ccs` (`GetCCSAdrs("xnote")` when `ccThSkill` starts): the
    /// common note file is resident, so a skill a party member casts gets
    /// its own animation ([`SkillRun::has_anm`]). The runtime sets it; a
    /// harness without the file leaves it off.
    pub xnote: bool,
}

/// A run's animation in a frame: whether it finished, and the notes it
/// passed (type, value), as [`Frame`] takes them.
pub type AnimFrame = (bool, Vec<(u32, u32)>);

/// What one running skill did in a frame of [`Skills::frame`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// The run's [`SkillRun::key`].
    pub key: u32,
    /// What it asked of the runtime besides the events.
    pub out: MainOut,
    /// The run is over and was deleted.
    pub over: bool,
}

impl Skills {
    /// `ccSkillCheck(ch)` (gcmn 0x005723e0): the id of the first running
    /// skill `ch` cast whose status is 0 that a hit interrupts: one flagged
    /// so, a physical skill, or a spell while Kite (`player_act`) is in a
    /// casting act (17, 18); 0 for none. Only for characters of type
    /// 0x0700000f; Mutation's rule (0x00597bb0) is in docs/engine/battle.md.
    pub fn check(&self, t: &Tables, scene: &Scene, ch: usize, player_act: i16) -> i32 {
        if t.volume != piney_data::volume::Volume::Inf {
            let c = &scene.chars[ch];
            return self
                .runs
                .iter()
                .find(|r| {
                    r.creator == Some(ch)
                        && r.id != 0
                        && r.status == 0
                        && (c.ty() & 0x0700_000f == 0 || r.id == i32::from(c.skill_id))
                })
                .map_or(0, |r| r.id);
        }
        for r in &self.runs {
            if r.creator != Some(ch) || r.id == 0 || r.status != 0 || scene.chars[ch].ty() & 0x0700_000f == 0 {
                continue;
            }
            if r.report {
                return r.id;
            }
            let Some(sk) = t.skill(r.id) else { continue };
            if sk.ty & bits::PHYSICAL != 0 || (sk.ty & bits::MAGIC != 0 && (player_act == 17 || player_act == 18)) {
                return r.id;
            }
        }
        0
    }

    /// `_ccSkillRequest(cp, tp, sid, stype)` (gcmn 0x00572860) with the
    /// `ccSkill` it creates: see [`skill::request`] for the rules. Returns
    /// the new run's index, and the request's other effects.
    #[allow(clippy::too_many_arguments)]
    pub fn request(
        &mut self,
        t: &Tables,
        scene: &mut Scene,
        cp: usize,
        tp: Option<usize>,
        sid: i32,
        st: i8,
        rng: &mut dyn Rng,
    ) -> (Option<usize>, Option<skill::Request>) {
        let running = self.attack_running(cp);
        self.end_attack(cp, st);
        let tpi = tp.unwrap_or(cp);
        let Some(req) = skill::request(t, scene, cp, tpi, sid, i32::from(st), running, rng) else {
            return (None, None);
        };
        // With skillID and skillStatus, `_ccSkillRequest` sets the caster's
        // `targetChar` (+0x78) to `tp` (gcmn 0x00572b80), null included.
        if st != stype::ITEM {
            scene.chars[cp].target_char = tp;
        }
        let i = self.add(t, scene, cp, tp, sid, st, &req);
        (Some(i), Some(req))
    }

    /// Whether `cp`'s normal attack (skill 1) is running: what
    /// `_ccSkillRequest` and the item rules take as `running_attack`.
    pub fn attack_running(&self, cp: usize) -> bool {
        self.runs.iter().any(|r| r.creator == Some(cp) && r.id == 1)
    }

    /// A character's own skill or an item cast through it (stype 0, 2)
    /// ends its running normal attack.
    fn end_attack(&mut self, cp: usize, st: i8) {
        if st == stype::SKILL || st == stype::ITEM_CAST {
            for r in &mut self.runs {
                if r.creator == Some(cp) && r.id == 1 {
                    r.status = 1;
                }
            }
        }
    }

    /// The `ccSkill` a request that went through creates, appended to the
    /// list; returns its index.
    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        t: &Tables,
        scene: &Scene,
        cp: usize,
        tp: Option<usize>,
        sid: i32,
        st: i8,
        req: &skill::Request,
    ) -> usize {
        let mut run = SkillRun::new(sid);
        run.stype = st;
        run.creator = Some(cp);
        run.target = tp;
        run.pos = scene.chars[cp].pos;
        run.height = scene.chars[cp].base().height;
        if let Some(tg) = tp
            && scene.listed(tg)
        {
            run.target_pos = scene.chars[tg].pos;
            run.target_type = scene.chars[tg].ty();
        }
        run.ty = t.skill(sid).map_or(0, |s| s.ty);
        run.modify = req.modify;
        run.hold = req.hold;
        run.ac_flag = req.ac_flag;
        // `_ccSkillRequest` (gcmn 0x00572c14; MUT 0x005983d4): a skill a
        // PC (type bits 0 or 2) casts itself, whose row names a file, with
        // `xnote` resident, gets `new ccAnm` of `ANM_<file>` from that file
        // (+0x80), its notes to `ccSkillCheckNote`.
        run.has_anm = self.xnote
            && st == stype::SKILL
            && scene.chars[cp].ty() & 5 != 0
            && t.skill(sid).is_some_and(|s| !s.filename.is_empty());
        run.key = self.next_key;
        self.next_key = self.next_key.wrapping_add(1);
        self.runs.push(run);
        self.runs.len() - 1
    }

    /// The `ccSkill` an item started ([`crate::item::ItemSkill`], from
    /// [`crate::item::use_item_request`]'s steps): the rules already ran
    /// (with [`Skills::attack_running`] as `running_attack`); this ends the
    /// caster's normal attack as `_ccSkillRequest` does and adds the run,
    /// with the item's amount and compel bit. None when no skill was made.
    pub fn add_item(
        &mut self,
        t: &Tables,
        scene: &Scene,
        cp: usize,
        tp: usize,
        item: &crate::item::ItemSkill,
    ) -> Option<usize> {
        let req = item.request.as_ref()?;
        let st = item.stype as i8;
        self.end_attack(cp, st);
        let i = self.add(t, scene, cp, Some(tp), item.sid, st, req);
        if let Some(p) = item.param {
            self.runs[i].param = p;
        }
        self.runs[i].force = item.compel;
        Some(i)
    }

    /// `ccThSkill`'s frame (gcmn 0x005722e0): each skill's Main in list
    /// order, the ones that are over deleted. `anim` gives a run's
    /// animation this frame ([`Frame::anim_done`], [`Frame::notes`]); the
    /// rest of `base` is shared. Returns a [`Step`] per run.
    pub fn frame(
        &mut self,
        t: &Tables,
        scene: &mut Scene,
        anim: &dyn Fn(&SkillRun) -> AnimFrame,
        base: &Frame,
        rng: &mut dyn Rng,
        ev: &mut Events,
    ) -> Vec<Step> {
        let mut steps = Vec::new();
        let mut i = 0;
        while i < self.runs.len() {
            let (anim_done, notes) = anim(&self.runs[i]);
            let fr = Frame { env: base.env, w2p: base.w2p, anim_done, notes: &notes, annihilated: base.annihilated };
            let mut out = MainOut::default();
            let st = self.runs[i].main(t, scene, &fr, rng, ev, &mut out);
            let key = self.runs[i].key;
            if st != 0 {
                self.runs.remove(i);
            } else {
                i += 1;
            }
            steps.push(Step { key, out, over: st != 0 });
        }
        steps
    }
}

/// `ccSpcChar::DistanceToTarget(tt)` (gcmn 0x0059e860): the ground distance
/// between the two (the volume's `sqrtf`), less both widths; 100000.0 for a
/// target off the lists.
pub fn spc_distance_to_target(volume: Volume, scene: &Scene, me: usize, tt: usize) -> u32 {
    if !scene.listed(tt) {
        return 0x47c3_5000;
    }
    let a = &scene.chars[me];
    let b = &scene.chars[tt];
    let d = geom::plane_dist(volume, b.pos_p, a.pos_p);
    ee::sub(d, ee::add(b.base().width, a.base().width))
}

/// `ccPlayer::CheckNote` (gcmn 0x0059c300), note 0x8005 of Kite's own
/// attack animation: the hit of his normal attack or art on `targetChar`
/// (listed, alive, HP left). The distance on the ground between their
/// `pos`, less the target's width, truncated, is kept in `ccPlayer` +0x238
/// (`last_dist`); within 450 it is `CalcBattleDamage(target, skillID, 1.0,
/// -1)` and `EntryAffect(1, dmg, skillID)`, farther a miss (-1).
#[allow(clippy::too_many_arguments)]
pub fn player_attack_note(
    t: &Tables,
    scene: &mut Scene,
    me: usize,
    last_dist: &mut i32,
    rng: &mut dyn Rng,
    env: &Env,
    ev: &mut Events,
) {
    let Some(tg) = scene.chars[me].target_char else { return };
    if !scene.listed(tg) || scene.chars[tg].dead() || scene.chars[tg].hp == 0 {
        return;
    }
    let a = scene.chars[me].pos;
    let b = &scene.chars[tg];
    let d = geom::plane_dist(t.volume, a, b.pos);
    *last_dist = damage::fptosi(ee::sub(d, b.base().width));
    let sid = scene.chars[me].skill_id;
    if ee::le(ee::from_int(*last_dist), 0x43e1_0000) {
        let dmg = hit_with(t, scene, me, tg, i32::from(sid), rng, env, ev);
        ev.push(Event::affect(Who::Char(tg), Who::Char(me), 1, dmg as i16, sid, 0));
    } else {
        ev.push(Event::affect(Who::Char(tg), Who::Char(me), 1, -1, sid, 0));
    }
}

/// `ccChar::CalcBattleDamage(t, sid, mag, h)` (gcmn 0x0056d8a0): the
/// skill row's damage at 1.0 with a drawn roll.
#[allow(clippy::too_many_arguments)]
fn hit_with(
    t: &Tables,
    scene: &mut Scene,
    me: usize,
    tg: usize,
    sid: i32,
    rng: &mut dyn Rng,
    env: &Env,
    ev: &mut Events,
) -> i32 {
    let sk = t.skill(sid).cloned().unwrap_or_default();
    damage::calc_damage_in(t, scene, me, tg, &sk, F_ONE, Roll::Draw, rng, env, ev).dmg
}

/// `ccFellow::CheckNote` (gcmn 0x0041e1e0), note 0x8005 of a party
/// member's attack animation: as Kite's, but in range when
/// [`spc_distance_to_target`] is within the AI's `armsRange` (the AI's
/// `distTg` when the distance is exactly -1.0); a hit on a foe is
/// reported to its AI (`ChatMessageAttack`).
#[allow(clippy::too_many_arguments)]
pub fn fellow_attack_note(
    t: &Tables,
    scene: &mut Scene,
    me: usize,
    arms_range: u32,
    dist_tg: u32,
    rng: &mut dyn Rng,
    env: &Env,
    ev: &mut Events,
) {
    let Some(tg) = scene.chars[me].target_char else { return };
    if !scene.listed(tg) || scene.chars[tg].dead() || scene.chars[tg].hp == 0 {
        return;
    }
    let d = spc_distance_to_target(t.volume, scene, me, tg);
    let d = if ee::cmp(d, 0xbf80_0000) == std::cmp::Ordering::Equal { dist_tg } else { d };
    let sid = scene.chars[me].skill_id;
    if ee::le(d, arms_range) {
        let dmg = hit_with(t, scene, me, tg, i32::from(sid), rng, env, ev);
        ev.push(Event::affect(Who::Char(tg), Who::Char(me), 1, dmg as i16, sid, 0));
        if scene.chars[me].has_ai && scene.chars[tg].ty() & ty::FOE != 0 {
            ev.push(Event::ChatAttack { ai: Who::Char(me), on: Who::Char(tg), dmg, sid: i32::from(sid) });
        }
    } else {
        ev.push(Event::affect(Who::Char(tg), Who::Char(me), 1, -1, sid, 0));
    }
}
