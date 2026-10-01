//! The attack spells' element systems (gcmn skill.cpp): what an attack
//! spell's `ccSkill` does each frame once `ccSkill::Main` (gcmn 0x005731d0,
//! `ccThSkill`, priority 82) reaches it through the jump table at 0x006e1210:
//! `FallSystem`, `TornadoSystem`, `ConvergenceSystem`, `UpheavalSystem` and
//! `SummonsSystem`. The fields of `ccSkill` they use are a [`Spell`] owned by
//! [`Effects`]; a runtime calls [`Effects::spell_request`], then each frame
//! [`Spell::sync`] and [`Effects::spell_system`], and [`Effects::spell_remove`].
//! The damage calls are [`Event`]s; see docs/engine/effects.md ("The spells").

use piney_data::volume::Volume;

use crate::ee::{self, F, V4};
use crate::effect::{EffectCtrl, check_camera_deg};
use crate::{
    CharRef, Cx, Effects, Event, Host, convergence, drawelm, element, fall, summoned, summons, thunder, tornado,
    upheaval,
};

/// Element bits of `ccSkillParam.type` (+0x2c), `& 0xfc`.
pub mod attr {
    pub const SOIL: i32 = 0x04;
    pub const WATER: i32 = 0x08;
    pub const FIRE: i32 = 0x10;
    pub const WIND: i32 = 0x20;
    pub const THUNDER: i32 = 0x40;
    pub const DARK: i32 = 0x80;
}

/// Which system `ccSkill::Main` runs for an attack spell (its jump table at
/// gcmn 0x006e1210 on id - 150).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum System {
    Fall,
    Tornado,
    Convergence,
    Upheaval,
    Summons,
}

impl System {
    /// The table's entry for a skill id, None for a spell with no system.
    pub fn of(sid: i32) -> Option<System> {
        use System::*;
        Some(match sid {
            193..=196 | 225..=228 | 257..=260 | 273..=276 => Fall,
            197..=200 | 209..=212 | 229..=232 | 241..=244 | 261..=264 => Tornado,
            213..=216 | 233..=236 | 245..=248 | 265..=268 | 277..=280 => Convergence,
            201..=204 | 217..=220 | 249..=252 | 281..=284 => Upheaval,
            205..=208 | 221..=224 | 237..=240 | 253..=256 | 269..=272 | 285..=294 => Summons,
            _ => return None,
        })
    }
}

/// A spell's `ccSkill` (gcmn skill.cpp, 0xb0 bytes; 0xc0 from Mutation on,
/// [`Spell::t_pos_req`] at +0x70 moving `creator` and all after it 0x10 on)
/// as the systems and the effects read and write it.
#[derive(Clone, Debug, PartialEq)]
pub struct Spell {
    /// Not the game's: the runtime's name for the run (the battle crate's
    /// `SkillRun::key`), which the effects and the damage events carry in
    /// place of the `ccSkill *`.
    pub key: u32,
    /// Not the game's: still on `SkillEntryTop` (`ccSkillEntryCheck`).
    pub live: bool,
    /// +0x00 `ID`.
    pub id: i32,
    /// +0x0c `type` (bits 0-3: 0 the caster's own skill, 1 an item's, 2 an
    /// item through the caster), `endFlag` (bits 4-5: 0 running, 1 over, -1
    /// cancelled), +0x0d `holdFlag` (bit 0).
    pub stype: i8,
    pub status: i8,
    pub hold: bool,
    /// +0x10 `level`, set by the system at count 0 (1-4 within the
    /// element).
    pub level: i32,
    /// +0x14 `count`: frames run, as `ccSkill::Main` passes it to the
    /// system (before its own increment).
    pub count: i16,
    /// +0x16 `trigger`, +0x18 `step`, +0x1a `effNum`, +0x1c `atkCnt`, +0x20
    /// `tempCnt`: the systems' own counters.
    pub trigger: i16,
    pub step: i16,
    pub eff_num: i16,
    pub atk_cnt: i32,
    pub temp_cnt: i32,
    /// +0x24 `skillType` (the row's `type`), +0x28 `tType` (the target's
    /// `type` when it was requested).
    pub skill_type: i32,
    pub t_type: i32,
    /// +0x30 `cPos`, +0x40 `cDirc`: the caster's position and heading;
    /// +0x50 `cHeight`, its height; +0x60 `tPos`, the target's position.
    pub c_pos: V4,
    pub c_dirc: V4,
    pub c_height: F,
    pub t_pos: V4,
    /// From Mutation on, +0x70: the target's position when the spell was
    /// asked for (`_ccSkillRequest`, MUT gcmn 0x005981d4), which
    /// `ccSkill::Main` leaves alone; the systems aim at it where Infection's
    /// aim at `tPos` ([`Spell::aim`]). Unused on Infection.
    pub t_pos_req: V4,
    /// +0x70 `creator`, +0x74 `target`.
    pub creator: Option<CharRef>,
    pub target: Option<CharRef>,
    /// +0x84 `effPtr[8]`: effects the system follows, by slot.
    pub eff_ptr: [Option<usize>; 8],
    /// +0xa4 `m_effElm`: the element the system waits for, by the
    /// manager's slot.
    pub elm: Option<usize>,
}

impl Spell {
    /// `new ccSkill(sid)` (gcmn 0x00572f80): everything clear, `cHeight`
    /// 200, `cPos` and `tPos` (0, 0, 0, 1).
    pub fn new(key: u32, id: i32) -> Spell {
        Spell {
            key,
            live: true,
            id,
            stype: 0,
            status: 0,
            hold: false,
            level: 0,
            count: 0,
            trigger: 0,
            step: 0,
            eff_num: 0,
            atk_cnt: 0,
            temp_cnt: 0,
            skill_type: 0,
            t_type: 0,
            c_pos: ee::VF0,
            c_dirc: [0; 4],
            c_height: 0x4348_0000,
            t_pos: ee::VF0,
            t_pos_req: [0; 4],
            creator: None,
            target: None,
            eff_ptr: [None; 8],
            elm: None,
        }
    }

    /// The fields `ccSkill::Main` keeps up to date before the system runs
    /// (from the battle crate's `SkillRun`): the count before its
    /// increment, the caster's position, heading and height, the target's
    /// position and type, both characters (None once dropped).
    #[allow(clippy::too_many_arguments)]
    pub fn sync(
        &mut self,
        count: i16,
        c_pos: V4,
        c_dirc: V4,
        t_pos: V4,
        t_type: i32,
        creator: Option<CharRef>,
        target: Option<CharRef>,
    ) {
        self.count = count;
        self.c_pos = c_pos;
        self.c_dirc = c_dirc;
        self.t_pos = t_pos;
        self.t_type = t_type;
        self.creator = creator;
        self.target = target;
    }

    /// `this->param->type & 0xfc`: the spell's element bit.
    pub fn attr(&self, data: &SpellData) -> i32 {
        data.skill_type(self.id) & 0xfc
    }

    /// Where the systems' sounds, damage and effects go: `tPos` (the
    /// target's position this frame) on Infection, from Mutation on the
    /// target's position at the request (+0x70).
    pub fn aim(&self, volume: Volume) -> V4 {
        if volume == Volume::Inf { self.t_pos } else { self.t_pos_req }
    }

    /// `endFlag = 1`.
    fn end(&mut self) {
        self.status = 1;
    }

    /// The caster of its own skill (type 0 or 2) may act again: its
    /// `skillID` and `skillStatus` cleared.
    pub(crate) fn release(&self, cx: &mut Cx) {
        if let Some(c) = self.creator
            && (self.stype == 0 || self.stype == 2)
        {
            cx.raise(Event::SkillRelease { ch: c });
        }
    }
}

/// The spells' tables, read from the executable (GCMN.PRG loaded over it).
#[derive(Clone, Debug)]
pub struct SpellData {
    /// `skillTbl[id].type` (+0x2c).
    pub skill_types: Vec<i32>,
    /// `TornadoSystem`'s `sndcode` (gcmn 0x00651920): the rings' sound by
    /// level.
    pub tornado_se: [i32; 4],
    /// The tornado rings' tables (effects 89-91).
    pub rings: tornado::RingTables,
    /// The thunder's (effect -17, the bolts).
    pub thunder: thunder::ThunderTables,
    /// `ccTornadeElement`'s timing.
    pub tornade: tornado::TornadeTables,
    /// The fall's.
    pub fall: fall::FallTables,
    /// The convergence's.
    pub convergence: convergence::ConvergenceTables,
    /// The upheaval's.
    pub upheaval: upheaval::UpheavalTables,
    /// The summons'.
    pub summons: summons::SummonsTables,
    /// The summons' level 3-4 elements'.
    pub summoned: summoned::SummonedTables,
    /// The drawn elements' names and kinds.
    pub draw: drawelm::DrawTables,
    /// `particleCcsAnmTbl` (main 0x003739e0): (file, object) by index, what
    /// `ccParticleAdrs(n)` answers.
    pub particle_ccs_anm: Vec<(String, String)>,
    /// The skill starts' rings ([`crate::skillstart`]), the shock wave's
    /// ([`crate::shockwave`]), the cures' generators ([`crate::heal`]) and
    /// the stat changes' textures ([`crate::ability`]).
    pub skill_start: crate::skillstart::Tables,
    pub shock: crate::shockwave::Tables,
    pub heal: crate::heal::Tables,
    pub ability: crate::ability::Tables,
}

/// The first `N` floats of a generated table, as bits (0 past it).
pub(crate) fn bits<const N: usize>(v: &[f32]) -> [F; N] {
    std::array::from_fn(|i| v.get(i).map_or(0, |x| x.to_bits()))
}

/// The first `N` values of a generated table (0 past it).
pub(crate) fn first<const N: usize, T: Copy + Default>(v: &[T]) -> [T; N] {
    std::array::from_fn(|i| v.get(i).copied().unwrap_or_default())
}

/// The strings of a generated table.
pub(crate) fn strings<const N: usize>(v: &[&str]) -> [String; N] {
    std::array::from_fn(|i| v.get(i).map_or_else(String::new, |s| s.to_string()))
}

impl SpellData {
    /// The volume's (`tables::effect`, the skills' types from `battle`).
    pub fn read(volume: Volume) -> SpellData {
        let t = piney_data::tables::effect::of(volume);
        SpellData {
            skill_types: piney_data::tables::battle::of(volume).skills().iter().map(|s| s.kind).collect(),
            tornado_se: first(t.tornado_se()),
            rings: tornado::RingTables::read(volume),
            thunder: thunder::ThunderTables::read(volume),
            tornade: tornado::TornadeTables::read(volume),
            fall: fall::FallTables::read(volume),
            convergence: convergence::ConvergenceTables::read(volume),
            upheaval: upheaval::UpheavalTables::read(volume),
            summons: summons::SummonsTables::read(volume),
            summoned: summoned::SummonedTables::read(volume),
            draw: drawelm::DrawTables::read(volume),
            particle_ccs_anm: t
                .ccs_anm()
                .iter()
                .map(|r| (r.ccs.unwrap_or("").to_string(), r.anm.unwrap_or("").to_string()))
                .collect(),
            skill_start: crate::skillstart::Tables::read(volume),
            shock: crate::shockwave::Tables::read(volume),
            heal: crate::heal::Tables::read(volume),
            ability: crate::ability::Tables::read(volume),
        }
    }

    /// `ccParticleAdrs(n)` (main 0x001c2a00): `particleCcsAdrs[n]`, the
    /// object `particleCcsAnmTbl[n]` names.
    pub fn particle_adrs(&self, assets: &crate::files::Assets, n: usize) -> Option<crate::files::ObjRef> {
        let (ccs, name) = self.particle_ccs_anm.get(n)?;
        assets.find(ccs, name)
    }

    /// `ccGetSkillParam(sid)->type`.
    pub fn skill_type(&self, sid: i32) -> i32 {
        usize::try_from(sid).ok().and_then(|i| self.skill_types.get(i)).copied().unwrap_or(0)
    }
}

/// The spells' state in [`Effects`]: their `ccSkill`s, the tables.
#[derive(Clone, Debug)]
pub struct Spells {
    /// `SkillEntryTop`'s attack spells, in order (runs deleted by ccThSkill
    /// stay, `live` false, for the effects that still point at them).
    pub runs: Vec<Spell>,
    /// `ccEffectElementManager`.
    pub elements: element::Manager,
    /// `effWork2`: the 100 `ccEffect2`s.
    pub effect2: Vec<thunder::Effect2>,
    /// The drill missiles' (effect 168's) two `ccAnm`s, by effect slot
    /// ([`summons::drill_pre`]).
    pub drills: std::collections::HashMap<usize, summons::Drill>,
    pub data: SpellData,
}

impl Spells {
    pub fn read(volume: Volume) -> Spells {
        Spells {
            runs: Vec::new(),
            elements: element::Manager::default(),
            effect2: vec![thunder::Effect2::default(); thunder::SLOTS2],
            drills: std::collections::HashMap::new(),
            data: SpellData::read(volume),
        }
    }

    /// The run named `key`.
    pub fn get(&self, key: u32) -> Option<&Spell> {
        self.runs.iter().rev().find(|s| s.key == key)
    }

    pub fn get_mut(&mut self, key: u32) -> Option<&mut Spell> {
        self.runs.iter_mut().rev().find(|s| s.key == key)
    }

    fn index(&self, key: u32) -> Option<usize> {
        self.runs.iter().rposition(|s| s.key == key)
    }

    /// `ccSkillEntryCheck(sp)` (gcmn 0x00572f40): the skill is still on
    /// `SkillEntryTop`.
    pub fn entry_check(&self, key: u32) -> bool {
        self.get(key).is_some_and(|s| s.live)
    }
}

impl Effects {
    /// `_ccSkillRequest`'s `ccSkill` for an attack spell (gcmn 0x00572860,
    /// the constructor at 0x00572f80): the caster's position, heading and
    /// height, the target's position (from Mutation on twice, +0x70 too) and
    /// `type` when it is on the lists, the row's `type`. The request's own
    /// rules (SP, the attribute critical, holding) are the battle crate's.
    pub fn spell_request(
        &mut self,
        host: &dyn Host,
        key: u32,
        sid: i32,
        stype: i8,
        creator: Option<CharRef>,
        target: Option<CharRef>,
    ) {
        let mut s = Spell::new(key, sid);
        s.stype = stype;
        s.creator = creator;
        s.target = target;
        if let Some(c) = creator {
            s.c_pos = host.char_pos(c);
            s.c_dirc = host.char_dirc(c);
            s.c_height = host.char_height(c);
        }
        if let Some(t) = target
            && host.check_target(t)
        {
            s.t_pos = host.char_pos(t);
            if self.assets.volume != Volume::Inf {
                s.t_pos_req = s.t_pos;
            }
            s.t_type = host.char_type(t);
        }
        s.skill_type = self.spells.data.skill_type(sid);
        if let Some(i) = self.spells.index(key) {
            self.spells.runs.remove(i);
        }
        self.spells.runs.push(s);
    }

    /// ~ccSkill (gcmn 0x005730f0): off `SkillEntryTop`; `ccSkillEntryCheck`
    /// fails from now on.
    pub fn spell_remove(&mut self, key: u32) {
        if let Some(s) = self.spells.get_mut(key) {
            s.live = false;
        }
        // Keep only the runs something may still ask about.
        let n = self.spells.runs.len();
        if n > 64 {
            self.spells.runs.retain(|s| s.live);
        }
    }

    /// `ccSkill::Main`'s call of the spell's element system, this frame:
    /// false when `key` is not an attack spell with a system.
    pub fn spell_system(&mut self, host: &mut dyn Host, key: u32) -> bool {
        let Some(k) = self.spells.index(key) else { return false };
        let Some(sys) = System::of(self.spells.runs[k].id) else { return false };
        let (ctrl, mut cx) = self.split(host);
        match sys {
            System::Tornado => tornado_system(ctrl, &mut cx, k),
            System::Fall => fall::fall_system(ctrl, &mut cx, k),
            System::Convergence => convergence::convergence_system(ctrl, &mut cx, k),
            System::Upheaval => upheaval::upheaval_system(ctrl, &mut cx, k),
            System::Summons => summons::summons_system(ctrl, &mut cx, k),
        }
        true
    }
}

/// What `ccThEffect` runs after the 500 `ccEffect`s: the 100 `ccEffect2`s
/// (the rest of `ccEffectCtrl::Main`) and `ccEffectElementManager::Main`
/// (gcmn 0x004e8da0).
pub fn after_effects(ctrl: &mut EffectCtrl, cx: &mut Cx) {
    thunder::effect2_main(cx);
    element::manager_main(ctrl, cx);
}

/// `m_effElm->m_delFlag`.
fn element_deleted(cx: &Cx, slot: usize) -> bool {
    cx.spells.elements.get(slot).is_some_and(|e| e.deleted())
}

/// `checkCameraShakeRange(pos)` (main 0x00162f10): `pos` within 67.5
/// degrees of the active camera's view (`ccCheckCameraDeg`) and nearer
/// than 2000 to its eye.
pub fn check_camera_shake_range(cx: &Cx, pos: V4) -> bool {
    let cam = cx.host.camera();
    if !check_camera_deg(pos, cx.host.player_pos(), cx.host.bounds(), cam.cam_pos, cam.cam_view, 12288) {
        return false;
    }
    let d = ee::vsub(pos, cam.cam_pos);
    ee::lt(ee::sqrtf_on(cx.assets.volume, ee::dot(d, d)), 0x44fa_0000)
}

/// `cameraShake(power, cycle, time, dirc)`.
pub fn camera_shake(cx: &mut Cx, power: i32, cycle: i32, time: i32, dirc: i32) {
    cx.raise(Event::CameraShake([power, cycle, time, dirc]));
}

/// `ccMenu->noiz->SetNoizBs(bs)`.
pub fn noise(cx: &mut Cx, bs: i32) {
    cx.raise(Event::Noise { bs });
}

/// `ccSkillDamage(creator, target, ccGetSkillParam(ID), &acFlag, ID)`.
pub(crate) fn damage(cx: &mut Cx, s: &Spell) {
    cx.raise(Event::SkillDamage { spell: s.key, attacker: s.creator, target: s.target, sid: s.id });
}

/// `ccSkillDamage(creator, pos, tType, ccGetSkillParam(ID), ID)`.
fn damage_at(cx: &mut Cx, s: &Spell, pos: V4) {
    cx.raise(Event::SkillDamageAt { attacker: s.creator, pos, ttype: s.t_type, sid: s.id });
}

/// `ccSkill::TornadoSystem` (gcmn 0x00577540): level = id - (196 soil, 208
/// water, 228 fire, 240 wind, 260 thunder); smoke at 20, the rings and sound
/// at 30 (the caster released), `ccSkillDamage` every 5 frames from 35 to 75,
/// the shake at 40 (`BLUR`), the end after 75. From level 3 the tornado
/// element runs it, and the skill ends once the element is deleted. From
/// Mutation on (MUT gcmn 0x0059ccf0) it aims at [`Spell::aim`] and lets the
/// caster go at the end instead of at 30.
fn tornado_system(ctrl: &mut EffectCtrl, cx: &mut Cx, k: usize) {
    const BLUR: i16 = 40;
    const LAST: i16 = 75;
    let attr = cx.spells.runs[k].attr(&cx.spells.data);
    if cx.spells.runs[k].count == 0 {
        let s = &mut cx.spells.runs[k];
        match attr {
            attr::SOIL => s.level = s.id - 196,
            attr::WATER => s.level = s.id - 208,
            attr::FIRE => s.level = s.id - 228,
            attr::WIND => s.level = s.id - 240,
            attr::THUNDER => s.level = s.id - 260,
            _ => {}
        }
        let target = s.target;
        if target.is_some_and(|t| cx.host.check_target(t)) {
            tornado::eff_magic_attack_sign(cx, target.unwrap_or(0), attr);
            let level = cx.spells.runs[k].level;
            if level >= 3 {
                cx.spells.runs[k].elm = tornado::tornade_elements_generate(cx, k, level);
            }
        } else {
            let s = cx.spells.runs[k].clone();
            cx.spells.runs[k].end();
            if s.creator.is_some() {
                s.release(cx);
            }
        }
    }
    let s = cx.spells.runs[k].clone();
    let inf = cx.assets.volume == Volume::Inf;
    let aim = s.aim(cx.assets.volume);
    if let Some(e) = s.elm
        && s.level >= 3
    {
        if element_deleted(cx, e) {
            let s = &mut cx.spells.runs[k];
            s.elm = None;
            s.hold = false;
            s.end();
            let s = s.clone();
            if s.creator.is_some() {
                s.release(cx);
            }
        }
        return;
    }
    match s.count {
        20 => {
            tornado::eff_skill_tornade_smoke(cx, aim, ee::VF0, attr, s.level);
        }
        30 => {
            let se = cx.spells.data.tornado_se[(s.level - 1).clamp(0, 3) as usize];
            cx.raise(Event::Sound3d { se, pos: aim });
            tornado::eff_skill_tornade_rings_pos(ctrl, cx, aim, attr, s.level);
            if inf && s.creator.is_some() {
                s.release(cx);
            }
        }
        35 | 40 | 45 | 50 | 55 | 60 | 65 | 70 | 75 => match s.target {
            Some(t) if cx.host.check_target(t) && cx.host.char_dead(t) == 0 => damage(cx, &s),
            _ => damage_at(cx, &s, aim),
        },
        _ => {}
    }
    if s.count == BLUR && check_camera_shake_range(cx, aim) {
        noise(cx, 20);
        camera_shake(cx, 0, 2, 20, 2);
    }
    if s.count > LAST {
        let r = &mut cx.spells.runs[k];
        r.hold = false;
        r.end();
        if !inf {
            s.release(cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::Camera;
    use crate::ee::ONE;
    use crate::host::{CharState, Simple};

    #[test]
    fn every_attack_spell_has_its_system() {
        let count = |s: System| (150..304).filter(|&id| System::of(id) == Some(s)).count();
        assert_eq!(
            [System::Fall, System::Tornado, System::Convergence, System::Upheaval, System::Summons].map(count),
            [16, 20, 20, 16, 30]
        );
        assert_eq!(System::of(192), None);
        assert_eq!(System::of(295), None);
    }

    /// Casts `sid` from character 1 on character 2 and runs it as
    /// `ccSkill::Main` would (the effects, then the system with the count
    /// before its increment) until it ends: the frames and the events.
    fn cast(sid: i32) -> Option<(i16, Vec<Event>)> {
        let mut fx = crate::testing::effects()?;
        fx.ctrl.town = false;
        let mut rng = piney_world::Rand(11);
        let mut next = || rng.rand();
        let at = [0, 0, 0, ONE];
        let cam = [0, 0xc448_0000, 0x43fa_0000, ONE];
        let camera = Camera { eye: cam, cam_pos: cam, cam_view: at, ..Camera::default() };
        let mut host = Simple::new(&mut next, at, camera);
        let c = |id, x: f32, y: f32| CharState {
            id,
            pos: [ee::k(x), ee::k(y), 0, ONE],
            dirc: [0; 4],
            height: 0x4320_0000,
            width: 0x4220_0000,
        };
        host.chars.push(c(1, 0.0, -300.0));
        host.chars.push(c(2, 0.0, 300.0));
        fx.spell_request(&host, 7, sid, 0, Some(1), Some(2));
        let mut events = Vec::new();
        for count in 0..400 {
            fx.step(&mut host);
            fx.spells.get_mut(7)?.count = count;
            assert!(fx.spell_system(&mut host, 7));
            events.extend(fx.take_events());
            if fx.spells.get(7)?.status != 0 {
                return Some((count, events));
            }
        }
        None
    }

    fn damages(events: &[Event]) -> usize {
        events
            .iter()
            .filter(|e| {
                matches!(e, Event::SkillDamage { .. } | Event::SkillDamageAt { .. } | Event::SkillDamage2 { .. })
            })
            .count()
    }

    #[test]
    fn a_level_one_tornado_hits_nine_times() {
        let Some((end, events)) = cast(197) else { return };
        assert_eq!(end, 76);
        assert_eq!(damages(&events), 9);
        assert_eq!(events.iter().filter(|e| matches!(e, Event::SkillRelease { ch: 1 })).count(), 1);
    }

    #[test]
    fn the_level_one_systems_end_when_the_game_ends_them() {
        // Upheaval: the damage at 49, the end at 71; summons: 52 and 62.
        let Some((end, events)) = cast(201) else { return };
        assert_eq!((end, damages(&events)), (71, 1));
        let Some((end, events)) = cast(205) else { return };
        assert_eq!((end, damages(&events)), (62, 1));
        // Convergence: one damage once the pieces are in.
        let Some((_, events)) = cast(213) else { return };
        assert_eq!(damages(&events), 1);
        // Fall: a meteor, its damage as it lands.
        let Some((_, events)) = cast(193) else { return };
        assert_eq!(damages(&events), 1);
    }

    #[test]
    fn levels_three_and_four_end_with_their_elements() {
        // (spell, the count it ends at, its hits): the fall's meteors and
        // the upheavals' waves hit 3 times at level 3 and 4 at level 4 (the
        // ice's last wave twice), the convergence and the summons once;
        // the drill (289) hits at 150 and ends at 190.
        let want = [
            (195, 56, 3),
            (196, 76, 4),
            (203, 132, 3),
            (204, 152, 4),
            (215, 61, 1),
            (216, 117, 1),
            (219, 100, 3),
            (220, 88, 3),
            (239, 136, 1),
            (240, 158, 1),
            (251, 132, 3),
            (252, 184, 4),
            (283, 147, 3),
            (284, 167, 4),
            (287, 143, 1),
            (288, 251, 1),
            (289, 190, 1),
            (293, 136, 1),
        ];
        for (sid, end, hits) in want {
            let Some((e, events)) = cast(sid) else { return };
            assert_eq!((sid, e, damages(&events)), (sid, end, hits));
        }
    }
}
