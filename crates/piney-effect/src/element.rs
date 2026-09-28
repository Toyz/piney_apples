//! GCMN.PRG's effect elements (effect2.cpp): `ccAnimateObject`, the
//! `ccEffectElement` every spell element derives from, and
//! `ccEffectElementManager`, the 1024 slots `ccThEffect` runs after
//! `ccEffectCtrl::Main`.
//!
//! ```text
//! ccEffectElementManager::m_instance (gcmn, main 0x00378c04): made by
//!   ccThEffect (or the first generator) with every slot empty
//! a generator (ccSkillTornadeElementsGenerate, ccFallElementGenerate ...):
//!   new the element, then the first empty slot; none: the element deleted,
//!   the generator answers 0
//! ccEffectElementManager::Main (gcmn 0x004e8da0), each frame after
//! ccEffectCtrl::Main, on the effect layer (20):
//!   for each of the 1024 slots in order: an element with m_delFlag set is
//!   deleted (its virtual destructor) and the slot emptied; any other runs
//!   its virtual Main
//! ```
//!
//! `ccAnimateObject` (0x130 bytes) carries the fade, the scale animation
//! and the motion towards a point; `ccEffectElement` (0x190) adds the skill
//! it serves, its target, level, flags and counters. Every element's
//! constructor starts with both constructors inlined: the vectors (0, 0, 0,
//! 1), `m_scale` (1, 1, 1, 1), `m_transparency` 1, the flags clear, `m_level`
//! 1, `m_life` -1; `m_fadeSpd` and `m_scaleSpd` are left as the heap had
//! them (zero here).

use crate::ee::{self, F, ONE, V4, VF0};
use crate::effect::EffectCtrl;
use crate::{Cx, convergence, drawelm, fall, ring, shield, space, summoned, tornado, upheaval};

/// `ccEffectElementManager::m_effTbl`.
pub const SLOTS: usize = 1024;

/// `ccAnimateObject` (gcmn effect2.cpp, 0x130 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct Animate {
    /// +0x00 `m_startFade`, +0x04 `m_endFade`, +0x08 `m_fadeSpd`, +0x0c
    /// `m_fadeAccel`, +0x10 `m_fadeFlag`.
    pub start_fade: F,
    pub end_fade: F,
    pub fade_spd: F,
    pub fade_accel: F,
    pub fade_flag: i32,
    /// +0x20 `m_startScale`, +0x30 `m_endScale`, +0x40 `m_scaleSpd`, +0x50
    /// `m_scaleAccel`, +0x60 `m_scaleFlag`.
    pub start_scale: V4,
    pub end_scale: V4,
    pub scale_spd: V4,
    pub scale_accel: V4,
    pub scale_flag: i32,
    /// +0x70 `m_accel`, +0x80 `m_startSpeed`, +0x90 `m_endSpeed`, +0xa0
    /// `m_posFlag`, +0xa4 `m_accelFlag`.
    pub accel: V4,
    pub start_speed: V4,
    pub end_speed: V4,
    pub pos_flag: i32,
    pub accel_flag: i32,
    /// +0xb0 `m_pos`, +0xc0 `m_dirc`, +0xd0 `m_offset`, +0xe0 `m_speed`,
    /// +0xf0 `m_sp`, +0x100 `m_ep`, +0x110 `m_scale`, +0x120
    /// `m_transparency`.
    pub pos: V4,
    pub dirc: V4,
    pub offset: V4,
    pub speed: V4,
    pub sp: V4,
    pub ep: V4,
    pub scale: V4,
    pub transparency: F,
}

impl Default for Animate {
    /// The inlined constructor.
    fn default() -> Self {
        Animate {
            start_fade: 0,
            end_fade: 0,
            fade_spd: 0,
            fade_accel: 0,
            fade_flag: 0,
            start_scale: VF0,
            end_scale: VF0,
            scale_spd: [0; 4],
            scale_accel: VF0,
            scale_flag: 0,
            accel: VF0,
            start_speed: VF0,
            end_speed: VF0,
            pos_flag: 0,
            accel_flag: 0,
            pos: VF0,
            dirc: VF0,
            offset: VF0,
            speed: VF0,
            sp: VF0,
            ep: VF0,
            scale: [ONE; 4],
            transparency: ONE,
        }
    }
}

impl Animate {
    /// `SetFade(st, et, time)` (gcmn 0x004e8f70): at once to `et` for a time
    /// not above 0, else from `st` by `(et - st) / time` a frame.
    pub fn set_fade(&mut self, st: F, et: F, time: F) {
        if ee::le(time, 0) {
            self.transparency = et;
        } else {
            self.start_fade = st;
            self.end_fade = et;
            self.fade_spd = ee::div(ee::sub(et, st), time);
            self.transparency = st;
            self.fade_flag = 1;
        }
    }

    /// `AnimateFade()` (0x004e8fc0): a step of the fade; 1 once it is over
    /// (or not running), the speed growing by `m_fadeAccel` until then.
    pub fn animate_fade(&mut self) -> bool {
        if self.fade_flag == 0 {
            return true;
        }
        self.transparency = ee::add(self.transparency, self.fade_spd);
        let e = self.end_fade;
        let done =
            if ee::lt(e, self.start_fade) { ee::lt(self.transparency, e) } else { !ee::le(self.transparency, e) };
        if done {
            self.transparency = e;
            self.fade_flag = 0;
            return true;
        }
        self.fade_spd = ee::add(self.fade_spd, self.fade_accel);
        false
    }

    /// `SetScaleAnm(ss, es, time)` (0x004e90c0): at once to `es` for a time
    /// not above 0, else from `ss` by `(es - ss) / time` a frame in x, y and
    /// z, with no acceleration.
    pub fn set_scale_anm(&mut self, ss: V4, es: V4, time: F) {
        if ee::le(time, 0) {
            self.scale = es;
            return;
        }
        self.start_scale = ss;
        self.end_scale = es;
        for i in 0..3 {
            self.scale_spd[i] = ee::div(ee::sub(es[i], ss[i]), time);
        }
        self.scale = ss;
        self.scale_flag = 1;
        self.scale_accel = VF0;
    }

    /// `SetScaleAnm(ss, es, time)` (0x004e9070) with (s, s, s, 1).
    pub fn set_scale_anm_f(&mut self, ss: F, es: F, time: F) {
        self.set_scale_anm([ss, ss, ss, ONE], [es, es, es, ONE], time);
    }

    /// `AnimateScale()` (0x004e91c0): a step of the scale, each lane held at
    /// its end; true only when it was not running.
    pub fn animate_scale(&mut self) -> bool {
        if self.scale_flag == 0 {
            return true;
        }
        let mut t = ee::vadd(self.scale, self.scale_spd);
        if clamp_towards(&mut t, self.start_scale, self.end_scale) {
            self.scale_flag = 0;
        }
        self.scale = t;
        self.scale_spd = ee::vadd(self.scale_spd, self.scale_accel);
        false
    }

    /// `AnimateSpeed()` (0x004e9340): the speed on by `m_accel`, each lane
    /// held at `m_endSpeed`; true when every lane got there.
    pub fn animate_speed(&mut self) -> bool {
        if self.accel_flag == 0 {
            return false;
        }
        let mut v = ee::vadd(self.speed, self.accel);
        let done = clamp_towards(&mut v, self.start_speed, self.end_speed);
        self.speed = v;
        if done {
            self.accel_flag = 0;
        }
        done
    }

    /// `AnimatePos()` (0x004e94a0): the position on by the speed towards
    /// `m_ep` (in the player's frame), each lane held there; true when it
    /// arrived (the speed then left as it was), else the speed's step.
    pub fn animate_pos(&mut self, player: V4, bounds: [F; 4]) -> bool {
        if self.pos_flag != 0 {
            let sp = space::w2p(self.sp, player, bounds);
            let ep = space::w2p(self.ep, player, bounds);
            let mut p = ee::vadd(space::w2p(self.pos, player, bounds), self.speed);
            let done = clamp_towards(&mut p, sp, ep);
            self.pos = space::p2w(p, player, bounds);
            if done {
                self.pos_flag = 0;
                return true;
            }
        }
        self.animate_speed();
        false
    }
}

/// Each of x, y and z of `v` held at `end` once past it (going up from
/// `start` or down): true when all three are there.
fn clamp_towards(v: &mut V4, start: V4, end: V4) -> bool {
    let mut done = true;
    for i in 0..3 {
        if ee::lt(start[i], end[i]) {
            if ee::lt(v[i], end[i]) {
                done = false;
            } else {
                v[i] = end[i];
            }
        } else if ee::le(v[i], end[i]) {
            v[i] = end[i];
        } else {
            done = false;
        }
    }
    done
}

/// `ccEffectElement` (gcmn effect2.cpp, 0x190 bytes): `ccAnimateObject`
/// and what every element shares.
#[derive(Clone, Debug, PartialEq)]
pub struct Base {
    pub anim: Animate,
    /// +0x130 `m_skillPtr`: the spell it serves (its key).
    pub skill: Option<u32>,
    /// +0x134 `m_target`.
    pub target: Option<crate::CharRef>,
    /// +0x138 `m_level` (1 from the constructor), +0x13c `m_endFlag`,
    /// +0x140 `m_delFlag` (the manager deletes it), +0x144 `m_life` (-1),
    /// +0x148 `m_attr`, +0x14c `m_count`, +0x150 `m_proccess`, +0x154
    /// `m_status[4]`, +0x164 `m_interval`.
    pub level: i32,
    pub end_flag: i32,
    pub del_flag: i32,
    pub life: i32,
    pub attr: i32,
    pub count: i32,
    pub proccess: i32,
    pub status: [i32; 4],
    pub interval: i32,
    /// +0x170 `m_accel`.
    pub accel: V4,
    /// +0x180 `m_syncPos`, +0x184 `m_syncDirc`: vectors it follows.
    pub sync_pos: Option<crate::VecRef>,
    pub sync_dirc: Option<crate::VecRef>,
    /// Not the game's: the order the manager took it in (0 for one never
    /// added), which tells a pointer's element from a later one in its
    /// slot.
    pub serial: u32,
}

impl Default for Base {
    fn default() -> Self {
        Base {
            anim: Animate::default(),
            skill: None,
            target: None,
            level: 1,
            end_flag: 0,
            del_flag: 0,
            life: -1,
            attr: 0,
            count: 0,
            proccess: 0,
            status: [0; 4],
            interval: 0,
            accel: VF0,
            sync_pos: None,
            sync_dirc: None,
            serial: 0,
        }
    }
}

/// An element in the manager, by class.
#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    /// The slot's element is running (taken out while its `Main` runs).
    Busy,
    Ring(Box<ring::RingElement>),
    Tornade(Box<tornado::TornadeElement>),
    Fall(Box<fall::FallElement>),
    Convergence(Box<convergence::ConvergenceElement>),
    ThunderFall(Box<fall::ThunderFallElement>),
    /// A drawn element in the manager (`effExplode3`'s explosion).
    Draw(Box<drawelm::DrawElm>),
    /// The upheaval's level 3-4 managers and their pieces.
    Upheaval(Box<upheaval::UpheavalMngr>),
    UpheavalPart(Box<upheaval::UpheavalPart>),
    /// The summons' level 3-4 system element and `effEnergyGrow`'s sparks.
    Summons(Box<summoned::SummonsSystem>),
    EnergyGrow(Box<summoned::EnergyGrow>),
    /// `ccResistantShieldElement` ([`shield`]).
    Shield(Box<shield::ResistantShield>),
}

impl Element {
    /// The shared part.
    pub fn base(&self) -> Option<&Base> {
        Some(match self {
            Element::Busy => return None,
            Element::Ring(e) => &e.base,
            Element::Tornade(e) => &e.base,
            Element::Fall(e) => &e.base,
            Element::Convergence(e) => &e.base,
            Element::ThunderFall(e) => &e.base,
            Element::Draw(e) => &e.base,
            Element::Upheaval(e) => &e.base,
            Element::UpheavalPart(e) => &e.base,
            Element::Summons(e) => &e.base,
            Element::EnergyGrow(e) => &e.base,
            Element::Shield(e) => &e.base,
        })
    }

    pub fn base_mut(&mut self) -> Option<&mut Base> {
        Some(match self {
            Element::Busy => return None,
            Element::Ring(e) => &mut e.base,
            Element::Tornade(e) => &mut e.base,
            Element::Fall(e) => &mut e.base,
            Element::Convergence(e) => &mut e.base,
            Element::ThunderFall(e) => &mut e.base,
            Element::Draw(e) => &mut e.base,
            Element::Upheaval(e) => &mut e.base,
            Element::UpheavalPart(e) => &mut e.base,
            Element::Summons(e) => &mut e.base,
            Element::EnergyGrow(e) => &mut e.base,
            Element::Shield(e) => &mut e.base,
        })
    }

    /// `m_delFlag`.
    pub fn deleted(&self) -> bool {
        self.base().is_some_and(|b| b.del_flag != 0)
    }
}

/// `ccEffectElementManager`: the slots.
#[derive(Clone, Debug)]
pub struct Manager {
    pub slots: Vec<Option<Element>>,
    /// The last [`Base::serial`] given.
    pub serial: u32,
}

impl Default for Manager {
    fn default() -> Self {
        Manager { slots: vec![None; SLOTS], serial: 0 }
    }
}

/// A pointer to an element in the manager: its slot and serial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElmPtr {
    pub slot: usize,
    pub serial: u32,
}

impl Manager {
    /// A generator's registration: the first empty slot, or None (the
    /// element is deleted).
    pub fn add(&mut self, mut e: Element) -> Option<usize> {
        let i = self.slots.iter().position(|s| s.is_none())?;
        self.serial += 1;
        if let Some(b) = e.base_mut() {
            b.serial = self.serial;
        }
        self.slots[i] = Some(e);
        Some(i)
    }

    /// [`Manager::add`], answering a pointer to it.
    pub fn add_ptr(&mut self, e: Element) -> Option<ElmPtr> {
        let slot = self.add(e)?;
        Some(ElmPtr { slot, serial: self.serial })
    }

    /// The element `p` points at, while it is still there (running
    /// counts: it is taken out of its slot while its `Main` runs).
    pub fn get_ptr(&self, p: ElmPtr) -> Option<&Element> {
        self.get(p.slot).filter(|e| e.base().is_none_or(|b| b.serial == p.serial))
    }

    pub fn get_ptr_mut(&mut self, p: ElmPtr) -> Option<&mut Element> {
        self.slots.get_mut(p.slot).and_then(|s| s.as_mut()).filter(|e| e.base().is_none_or(|b| b.serial == p.serial))
    }

    pub fn get(&self, i: usize) -> Option<&Element> {
        self.slots.get(i).and_then(|s| s.as_ref())
    }

    /// The shared part of the element in slot `i`.
    pub fn base_mut(&mut self, i: usize) -> Option<&mut Base> {
        self.slots.get_mut(i).and_then(|s| s.as_mut()).and_then(|e| e.base_mut())
    }
}

/// `ccEffectElementManager::Main` (gcmn 0x004e8da0).
pub fn manager_main(ctrl: &mut EffectCtrl, cx: &mut Cx) {
    for i in 0..SLOTS {
        let Some(e) = cx.spells.elements.slots[i].take() else { continue };
        if e.deleted() {
            continue;
        }
        cx.spells.elements.slots[i] = Some(Element::Busy);
        let e = run(e, ctrl, cx);
        cx.spells.elements.slots[i] = Some(e);
    }
}

/// An element's virtual `Main`.
fn run(mut e: Element, ctrl: &mut EffectCtrl, cx: &mut Cx) -> Element {
    match &mut e {
        Element::Busy => {}
        Element::Ring(r) => r.main(cx),
        Element::Tornade(t) => t.main(ctrl, cx),
        Element::Fall(f) => f.main(ctrl, cx),
        Element::Convergence(c) => c.main(ctrl, cx),
        Element::ThunderFall(t) => t.main(cx),
        Element::Draw(d) => d.main(cx),
        Element::Upheaval(u) => u.main(ctrl, cx),
        Element::UpheavalPart(u) => u.main(ctrl, cx),
        Element::Summons(u) => u.main(ctrl, cx),
        Element::EnergyGrow(u) => u.main(cx),
        Element::Shield(s) => s.main(cx),
    }
    e
}
