//! The upheaval spells (soil, water, wind, dark): `ccSkill::UpheavalSystem`
//! (gcmn 0x005787c0), its pillars (effects 29-41, first switch 0x001c4468)
//! and what they throw when they burst at count 14: `effUpheavalFragment`
//! (42-51) and `effUpheavalFlash` (52-55). Levels 3 and 4 hand the rising to
//! an element manager of the spell's element ([`UpheavalMngr`]) and its
//! pieces ([`UpheavalPart`]). The rules are in docs/engine/effects.md
//! ("UpheavalSystem").

use piney_data::volume::Volume;

use crate::drawelm::{self, AnmObj, DrawElm, PI, TWO_PI};
use crate::ee::{self, F, ONE, V4, VF0};
use crate::effect::{Effect, EffectCtrl, Next, ONE_VECTOR, Obj};
use crate::element::{Animate, Base, Element};
use crate::fall::damage2_of;
use crate::files::ObjRef;
use crate::particle::{FfParam, GenParam, GenRef};
use crate::spell::{attr, camera_shake, check_camera_shake_range, first, noise, strings};
use crate::{Cx, Event, debris, pfx, ring, space, thunder, tornado, vu};

/// The pillars.
pub const PILLAR_FIRST: i16 = 29;
pub const PILLAR_LAST: i16 = 41;
/// `effUpheavalFragment`'s rocks and ice (42-49, which bounce), its leaves
/// and dark (50, 51), `effUpheavalFlash`'s flashes (52-55).
pub const FRAGMENT_FIRST: i16 = 42;
pub const FRAGMENT_BOUNCING: i16 = 49;
pub const FLASH_LAST: i16 = 55;

/// The upheaval's tables.
#[derive(Clone, Debug, PartialEq)]
pub struct UpheavalTables {
    /// `pillarDegTbl` (gcmn 0x00651960): each pillar's bearing.
    pub pillar_deg: [i16; 16],
    /// The level 3-4 managers' bearings: the dark hands' (gcmn 0x005ed580),
    /// the trees' (0x005ed5b0), the rocks' (0x005ed5d0).
    pub hand_deg: [i16; 16],
    pub tree_deg: [i16; 16],
    pub rock_deg: [i16; 16],
    /// `ccIceUpheavalMngrElement::_Level3`'s counts: the ice raised
    /// (0x005ed5a0) and the hits (0x003781f8).
    pub ice_raise: [i32; 4],
    pub ice_hit: [i32; 2],
    /// The rocks' and the trees' clumps (0x005ed6f0, 0x005ed700).
    pub rocks: [String; 4],
    pub trees: [String; 4],
    /// The falling leaves (`effElementGeneratorTbl` + 0x188, 0x005ed0f8,
    /// its force field `effElementFFTbl` + 0x1e0, 0x005ed430) and the ice's
    /// mist (+ 0x230, 0x005ed1a0; + 0x240, 0x005ed490).
    pub leaf_gen: GenParam,
    pub leaf_ff: FfParam,
    pub ice_gen: GenParam,
    pub ice_ff: FfParam,
}

impl UpheavalTables {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> UpheavalTables {
        let t = piney_data::tables::effect::of(volume);
        UpheavalTables {
            pillar_deg: first(t.pillar_deg()),
            hand_deg: first(t.hand_deg()),
            tree_deg: first(t.tree_deg()),
            rock_deg: first(t.rock_deg()),
            ice_raise: first(t.ice_raise()),
            ice_hit: first(t.ice_hit()),
            rocks: strings(t.upheaval_rocks()),
            trees: strings(t.upheaval_trees()),
            leaf_gen: GenParam::other(t.element_generators(), t.element_generators_va(), 7),
            leaf_ff: FfParam::row(t.element_ffs(), t.element_ffs_va(), 15),
            ice_gen: GenParam::other(t.element_generators(), t.element_generators_va(), 10),
            ice_ff: FfParam::row(t.element_ffs(), t.element_ffs_va(), 18),
        }
    }
}

/// `ccSkill::UpheavalSystem` (gcmn 0x005787c0). From Mutation on (MUT gcmn
/// 0x0059e170) it aims at [`Spell::aim`] and lets the caster go at count
/// 93 or at the end, whichever is first, instead of at `TIME1`.
pub fn upheaval_system(ctrl: &mut EffectCtrl, cx: &mut Cx, k: usize) {
    const RELEASE: i16 = 93;
    const START: i16 = 20;
    const TIME1: i16 = START + 15;
    const TIME2: i16 = TIME1 + 20;
    const TIME3: i16 = TIME2 + 20;
    const TIME4: i16 = TIME3 + 20;
    const INTERVAL: i16 = 2;
    const END: i16 = 4 + INTERVAL + 30;
    let attr = cx.spells.runs[k].attr(&cx.spells.data);
    let inf = cx.assets.volume == Volume::Inf;
    let s = cx.spells.runs[k].clone();
    if s.count == 0 {
        match s.target {
            Some(t) if cx.host.check_target(t) => {
                tornado::eff_magic_attack_sign(cx, t, attr);
            }
            _ => {
                cx.spells.runs[k].status = 1;
                s.release(cx);
            }
        }
        let r = &mut cx.spells.runs[k];
        match attr {
            attr::SOIL => r.level = r.id - 200,
            attr::WATER => r.level = r.id - 216,
            attr::WIND => r.level = r.id - 248,
            attr::DARK => r.level = r.id - 280,
            _ => {}
        }
    } else if s.count == START {
        let row = match attr {
            attr::SOIL | attr::WIND => 144,
            attr::WATER => 145,
            attr::DARK => 146,
            // The row is a register the callers never leave unset.
            _ => 144,
        };
        if s.level >= 3 {
            cx.spells.runs[k].elm = upheaval_element_generate(cx, k, s.level);
        }
        let tpos = s.aim(cx.assets.volume);
        pfx::start(cx, row, |g| {
            g.offset = [0, 0, 0x4220_0000, 0];
            g.pos = tpos;
        });
    }
    let s = cx.spells.runs[k].clone();
    let aim = s.aim(cx.assets.volume);
    let c = s.count;
    let pick = |t: i16, first: i32| -> Option<i32> {
        if c == t || c == t + INTERVAL || c == t + 2 * INTERVAL || c == t + 3 * INTERVAL {
            Some((i32::from(c) - i32::from(t)) / 2 + first)
        } else {
            None
        }
    };
    let mut n = 0;
    for (lv, t, first) in [(1, TIME1, 1), (2, TIME2, 5), (3, TIME3, 9), (4, TIME4, 13)] {
        if s.level >= lv
            && let Some(m) = pick(t, first)
        {
            n = m;
        }
    }
    if s.level < 3 && n != 0 {
        pillar(ctrl, cx, aim, attr, n);
    }
    if (s.level < 3 && c == TIME1 + 14) || (s.level == 2 && c == TIME2 + 14) {
        cx.raise(Event::SkillDamage2 {
            spell: s.key,
            attacker: s.creator,
            target: s.target,
            pos: aim,
            ttype: s.t_type,
            sid: s.id,
        });
        cx.raise(Event::Sound3dNote { se: 56, pos: aim, note: 67 });
        if check_camera_shake_range(cx, aim) {
            camera_shake(cx, 0, 2, 10, 0);
        }
    }
    if c == if inf { TIME1 } else { RELEASE } {
        s.release(cx);
    }
    let over = (s.level == 1 && c == TIME1 + END)
        || (s.level == 2 && c == TIME2 + END)
        || (s.level >= 3 && s.elm.is_some_and(|e| cx.spells.elements.get(e).is_some_and(|e| e.deleted())));
    if over {
        let r = &mut cx.spells.runs[k];
        r.hold = false;
        r.status = 1;
        if !inf && c < RELEASE {
            s.release(cx);
        }
    }
}

/// Pillar `n` (1-16) round `at` (the spell's [`crate::spell::Spell::aim`]).
fn pillar(ctrl: &mut EffectCtrl, cx: &mut Cx, at: V4, attr: i32, n: i32) {
    let id = match attr {
        attr::SOIL => 29 + (n % 4) as i16,
        attr::WATER => 33 + (n % 4) as i16,
        attr::WIND => 37 + (n % 4) as i16,
        attr::DARK => 41,
        // A register the callers never leave unset.
        _ => return,
    };
    let Some(i) = ctrl.new_effect(cx, id) else { return };
    let r = match n {
        ..5 => 0x4316_0000,
        5..9 => 0x4348_0000,
        9..13 => 0x4396_0000,
        _ => 0x43c8_0000,
    };
    let deg = cx.spells.data.upheaval.pillar_deg.get((n - 1) as usize).copied().unwrap_or(0);
    let a = ee::deg2rad(deg);
    let mut p = at;
    p[0] = ee::add(p[0], ee::mul(r, ee::cosf(a)));
    p[1] = ee::add(p[1], ee::mul(r, ee::sinf(a)));
    let turn = (i32::from(ee::rad2deg(a)) + 32767 + 16385) as i16;
    let e = &mut ctrl.effects[i];
    e.life_time = 80;
    e.offset = p;
    e.rot = [0, 0, ee::deg2rad(turn), 0];
    // SetFogSw(clump, 1).
}

/// The pillars' case of the first switch (main 0x001c4468).
pub fn pillar_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    if e.cnt == 0 {
        e.offset[3] = 0xc3fa_0000;
        e.speed[0] = 0x4234_0000;
    } else if e.cnt < 14 {
        e.speed[0] = ee::add(e.speed[0], 0xbf8c_cccd);
        e.offset[3] = ee::add(e.offset[3], e.speed[0]);
    } else if e.cnt < 17 {
        e.speed[0] = ee::mul(0xbf00_0000, e.speed[0]);
        e.offset[3] = ee::add(e.offset[3], e.speed[0]);
    } else if e.cnt < 27 {
        e.speed[0] = 0xc120_0000;
    } else {
        e.offset[3] = ee::add(e.offset[3], e.speed[0]);
    }
    let m = vu::rot_zyx(&vu::UNIT, e.rot);
    let v = ee::apply(&m, [0, 0, e.offset[3], ONE]);
    e.pos = ee::vadd(e.offset, v);
    e.pos[3] = ONE;
    let life = i32::from(e.life_time);
    e.fade_in_out(6, 55, life);
    if e.cnt == 14 {
        let kind = match e.id {
            29..=32 => 0,
            33..=36 => 1,
            37..=40 => 2,
            _ => 3,
        };
        let pos = e.pos;
        let mut r = VF0;
        r[0] = ee::deg2rad(-16384);
        eff_upheaval_fragment(ctrl, cx, pos, r, 0x4220_0000, 6, kind);
        eff_upheaval_flash(ctrl, cx, pos, r, 0x4220_0000, 6, kind);
    }
    Next::Draw
}

/// (0, -1, 0) turned by a random tilt of the draw `rn` (up to 45 degrees
/// off straight up), then by `r`.
fn upward(cx: &mut Cx, r: V4) -> (i32, V4) {
    let rn = cx.host.rand() >> 3;
    let a = [0, ee::deg2rad(((rn << 8) & 0xfc00) as i16), ee::deg2rad((6144 - (rn & 0xfc0)) as i16), 0];
    let d = ee::apply(&vu::rot_zyx(&vu::UNIT, a), [0, 0xbf80_0000, 0, ONE]);
    (rn, ee::apply(&vu::rot_zyx(&vu::UNIT, r), d))
}

/// `(100 - (rn >> 8) % 50) / 100`.
fn share(rn: i32) -> F {
    let h = 0x42c8_0000;
    ee::div(ee::sub(h, ee::from_int((rn >> 8) % 50)), h)
}

/// `ccEff::ChangeClut(ccParticleAdrs(new), ccParticleAdrs(old))` (main
/// 0x0013bb20): the sprite's CLUT becomes `new` only while it is `old`.
pub fn eff_change_clut(cx: &Cx, e: &mut Effect, new: usize, old: usize) {
    let Obj::Eff(f) = &mut e.obj else { return };
    let current = match f.clut {
        Some(c) => Some(c),
        None => cx
            .assets
            .eff_tex
            .get(f.file)
            .and_then(|v| v.get(f.chunk))
            .copied()
            .flatten()
            .map(|t| ObjRef { file: f.file, object: t.clut }),
    };
    let d = &cx.spells.data;
    if let (Some(o), Some(n)) = (d.particle_adrs(cx.assets, old), d.particle_adrs(cx.assets, new))
        && current == Some(o)
    {
        f.clut = Some(n);
    }
}

/// `effUpheavalFragment(p, r, v, n, kind)` (main 0x001d12a0): n pieces
/// thrown up from `p`: rocks 42-45 (soil) or ice 46-49 (water), life 45,
/// three tenths the size; a leaf 50 (wind, life 25, its sprite's pattern
/// count cut to 0-3, turned at random and fogged); dark 51 (life 25,
/// CLUT 150 made 151). The last piece.
#[allow(clippy::too_many_arguments)]
pub fn eff_upheaval_fragment(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    p: V4,
    r: V4,
    v: F,
    n: i32,
    kind: i32,
) -> Option<usize> {
    let mut last = None;
    for _ in 0..n {
        let (rn, d) = upward(cx, r);
        let (id, life, pats) = match kind {
            0 => (42 + ((cx.host.rand() >> 3) % 4) as i16, 45, -1),
            1 => (46 + ((cx.host.rand() >> 3) % 4) as i16, 45, -1),
            2 => (50, 25, (cx.host.rand() >> 3) % 4),
            3 => (51, 25, -1),
            // Registers the callers never leave unset.
            _ => return last,
        };
        last = ctrl.new_effect(cx, id);
        let Some(k) = last else { continue };
        let e = &mut ctrl.effects[k];
        e.life_time = life;
        e.pos = p;
        if kind == 2 {
            let a = ee::deg2rad(((cx.host.rand() >> 3) & 0xf800) as i16);
            if let Obj::Eff(f) = &mut ctrl.effects[k].obj {
                f.pat_num = pats as u16;
                f.rotate = a;
            }
        }
        if kind == 3 {
            eff_change_clut(cx, &mut ctrl.effects[k], 151, 150);
        }
        let e = &mut ctrl.effects[k];
        e.speed = ee::vscale(d, ee::mul(v, share(rn)));
        e.velocity = v;
        e.rot_speed[0] = ((rn >> 4) & 0x3c0) as u16;
        e.rot_speed[2] = ((rn >> 8) & 0x3c0) as u16;
        if kind == 0 || kind == 1 {
            e.scale = ee::vscale(ONE_VECTOR, 0x3e99_999a);
            // SetFogSw(clump, 1).
        }
        if kind == 2
            && let Obj::Eff(f) = &mut e.obj
        {
            f.prim = (f.prim & !0x20) | 0x20;
        }
    }
    last
}

/// `effUpheavalFlash(p, r, v, n, kind)` (main 0x001d1740): n flashes
/// thrown up from `p`, life 25, each sprite's pattern count cut to 0-3:
/// 52 (soil, CLUT 118 made 121), 53 (water), 54 (wind, 118 made 119), 55
/// (dark, 118 made 120). The last.
#[allow(clippy::too_many_arguments)]
pub fn eff_upheaval_flash(ctrl: &mut EffectCtrl, cx: &mut Cx, p: V4, r: V4, v: F, n: i32, kind: i32) -> Option<usize> {
    let (id, clut) = match kind {
        0 => (52, Some(121)),
        1 => (53, None),
        2 => (54, Some(119)),
        3 => (55, Some(120)),
        // A register the callers never leave unset.
        _ => return None,
    };
    let mut last = None;
    for _ in 0..n {
        let (rn, d) = upward(cx, r);
        last = ctrl.new_effect(cx, id);
        let Some(k) = last else { continue };
        let e = &mut ctrl.effects[k];
        e.pos = p;
        e.life_time = 25;
        let pats = (cx.host.rand() >> 3) % 4;
        if let Obj::Eff(f) = &mut ctrl.effects[k].obj {
            f.pat_num = pats as u16;
        }
        if let Some(c) = clut {
            eff_change_clut(cx, &mut ctrl.effects[k], c, 118);
        }
        let e = &mut ctrl.effects[k];
        e.speed = ee::vscale(d, ee::mul(v, share(rn)));
        e.velocity = v;
        e.rot_speed[0] = ((rn >> 4) & 0x3c0) as u16;
        e.rot_speed[2] = ((rn >> 8) & 0x3c0) as u16;
    }
    last
}

/// The fragments 42-49's cases of the second chain (main 0x001c9b44 and
/// 0x001ca0a8): the rocks' landing and bounce (`debris`), 0.75 a bounce.
pub fn fragment_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    debris::bounce_post(ctrl, cx, i, 0x3f40_0000, 0xbf40_0000);
}

/// 50-55's case of the second chain (main 0x001ca60c): FadeOut(5,
/// lifeTime).
pub fn flash_post(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) {
    let e = &mut ctrl.effects[i];
    let life = i32::from(e.life_time);
    e.fade_out(5, life);
}

/// `ccUpheavalElementGenerate(skill, level)` (gcmn 0x005007e0): the level
/// 3-4 manager of the spell's element (soil `ccSoilUpheavalMngrElement`,
/// water `ccIceUpheavalMngrElement`, wind `ccTreeUpheavalMngrElement`, dark
/// `ccDarkUpheavalMngrElement`) in the first empty slot of the element
/// manager (none, or any other element: None).
pub fn upheaval_element_generate(cx: &mut Cx, k: usize, level: i32) -> Option<usize> {
    let s = cx.spells.runs[k].clone();
    let m = match s.attr(&cx.spells.data) {
        attr::WATER => UpheavalMngr::ice(cx, &s, level),
        attr::WIND => UpheavalMngr::tree(cx, &s, level),
        attr::SOIL => UpheavalMngr::soil(&s, level),
        attr::DARK => UpheavalMngr::dark(cx, &s, level),
        _ => return None,
    };
    cx.spells.elements.add(Element::Upheaval(Box::new(m)))
}

/// One of `ccIceUpheavalMngrElement`'s `ELEMENT_T`s (8 bytes): an ice
/// spike (not in the element manager; the manager runs it) and whether it
/// is up.
#[derive(Clone, Debug, PartialEq)]
pub struct IceSlot {
    pub elm: Option<DrawElm>,
    pub status: i32,
}

/// `ccIceUpheavalMngrElement`'s own (0xaf0 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct IceParts {
    /// +0x5a4 `m_elements[128]`, +0x9a4 `m_bigIce` (level 4's), +0x9b0
    /// `m_animateBigIce` (set up by the constructor, never used).
    pub elements: Vec<IceSlot>,
    pub big_ice: Option<DrawElm>,
    pub animate_big_ice: Animate,
}

/// `ccTreeUpheavalMngrElement`'s own (0x720 bytes), level 4's great tree.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeParts {
    /// +0x5a4 `m_bigTree` (CMP_x407_1), +0x5b0 `m_treeOffset` (its rise;
    /// level 3 leaves it as the heap had it: zero here), +0x5c0
    /// `m_anmRoots` (ANM_x403), +0x5d0 `m_treeAnimate`, +0x700 `m_gpLeaf`
    /// (the falling leaves' generator, paused until the tree is up).
    pub big_tree: Option<ObjRef>,
    pub tree_offset: V4,
    pub anm_roots: Option<AnmObj>,
    pub tree_animate: Animate,
    pub gp_leaf: Option<GenRef>,
}

/// Which manager.
#[derive(Clone, Debug, PartialEq)]
pub enum MngrKind {
    Soil,
    Ice(Box<IceParts>),
    Tree(Box<TreeParts>),
    Dark,
}

/// The upheaval's level 3-4 managers (`ccMoveElement`s, 0x5c0 bytes and their
/// class's own): each raises its element's pieces round the target in waves
/// and hits at set counts, taking `m_tPos` from the spell while it is on
/// `SkillEntryTop`. Soil 0x004f0d30 (levels 3 / 4 0x004f0db0 / 0x004f1000),
/// water 0x004ee950, wind 0x004efd60, dark 0x004edce0; their waves are in
/// docs/engine/effects.md ("UpheavalSystem").
#[derive(Clone, Debug, PartialEq)]
pub struct UpheavalMngr {
    pub base: Base,
    /// +0x5a0 `m_skillPtr`: the spell's key.
    pub skill: u32,
    /// `m_tPos` (+0x5b0; the ice's +0xae0, the tree's +0x710).
    pub t_pos: V4,
    pub kind: MngrKind,
}

/// `(c - t) / 2 + first` when c is t, t + 2, t + 4 or t + 6 of one of the
/// waves.
fn wave(c: i32, waves: &[(i32, i32)]) -> Option<i32> {
    waves.iter().find_map(|&(t, first)| [t, t + 2, t + 4, t + 6].contains(&c).then_some((c - t) / 2 + first))
}

/// A bearing's heading: a quarter turn back from it.
fn heading(a: F) -> V4 {
    let turn = (i32::from(ee::rad2deg(a)) + 32767 + 16385) as i16;
    [0, 0, ee::deg2rad(turn), 0]
}

/// The spell's hit: `ccSkillDamage2` while it is on `SkillEntryTop`, the
/// sound and, in the camera's range, `cameraShake(0, 2, 10, 0)`.
fn wave_hit(cx: &mut Cx, key: u32, t_pos: V4) {
    damage2_of(cx, key);
    cx.raise(Event::Sound3dNote { se: 56, pos: t_pos, note: 67 });
    if check_camera_shake_range(cx, t_pos) {
        camera_shake(cx, 0, 2, 10, 0);
    }
}

impl UpheavalMngr {
    fn base_of(s: &crate::spell::Spell, level: i32) -> Base {
        Base { level, target: s.target, ..Base::default() }
    }

    /// `new ccSoilUpheavalMngrElement(skill, level)` (inlined in the
    /// generator).
    fn soil(s: &crate::spell::Spell, level: i32) -> UpheavalMngr {
        let mut base = Self::base_of(s, level);
        base.anim.pos = s.t_pos;
        base.anim.sp = s.t_pos;
        UpheavalMngr { base, skill: s.key, t_pos: s.t_pos, kind: MngrKind::Soil }
    }

    /// `new ccIceUpheavalMngrElement(skill, level)` (gcmn 0x004ee2a0).
    fn ice(cx: &mut Cx, s: &crate::spell::Spell, level: i32) -> UpheavalMngr {
        let mut base = Self::base_of(s, level);
        base.attr = s.attr(&cx.spells.data);
        base.anim.pos = s.t_pos;
        let parts = IceParts {
            elements: vec![IceSlot { elm: None, status: 0 }; 128],
            big_ice: None,
            animate_big_ice: Animate::default(),
        };
        let mut m = UpheavalMngr { base, skill: s.key, t_pos: s.t_pos, kind: MngrKind::Ice(Box::new(parts)) };
        match level {
            3 => m.ice_setup3(cx),
            4 => m.ice_setup4(cx),
            _ => {}
        }
        m
    }

    /// `new ccTreeUpheavalMngrElement(skill, level)` (gcmn 0x004ef920).
    fn tree(cx: &mut Cx, s: &crate::spell::Spell, level: i32) -> UpheavalMngr {
        let mut base = Self::base_of(s, level);
        base.anim.pos = s.t_pos;
        base.anim.sp = s.t_pos;
        let parts = TreeParts {
            big_tree: None,
            tree_offset: [0; 4],
            anm_roots: None,
            tree_animate: Animate::default(),
            gp_leaf: None,
        };
        let mut m = UpheavalMngr { base, skill: s.key, t_pos: s.t_pos, kind: MngrKind::Tree(Box::new(parts)) };
        if level == 4 {
            m.tree_setup4(cx);
        }
        m
    }

    /// `new ccDarkUpheavalMngrElement(skill, level)` (gcmn 0x004ed980): at
    /// the model 500 below the target (ccHitCheckLM2), else at the target.
    fn dark(cx: &mut Cx, s: &crate::spell::Spell, level: i32) -> UpheavalMngr {
        let mut base = Self::base_of(s, level);
        base.attr = s.attr(&cx.spells.data);
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let tp = s.t_pos;
        let mut q = space::w2p(tp, player, bounds);
        q[2] = ee::sub(q[2], 0x43fa_0000);
        let mut to = space::p2w(q, player, bounds);
        let p = if ee::eq(0xbf80_0000, cx.host.hit_check_lm2(tp, &mut to, 0x2000_0000)) { tp } else { to };
        base.anim.pos = p;
        base.anim.sp = p;
        base.anim.ep = p;
        UpheavalMngr { base, skill: s.key, t_pos: s.t_pos, kind: MngrKind::Dark }
    }

    /// The class `ccEffectElementManager` knows it by.
    pub fn class(&self) -> &'static str {
        match self.kind {
            MngrKind::Soil => "ccSoilUpheavalMngrElement",
            MngrKind::Ice(_) => "ccIceUpheavalMngrElement",
            MngrKind::Tree(_) => "ccTreeUpheavalMngrElement",
            MngrKind::Dark => "ccDarkUpheavalMngrElement",
        }
    }

    /// The manager's `Main`.
    pub fn main(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        if cx.spells.entry_check(self.skill)
            && let Some(s) = cx.spells.get(self.skill)
        {
            self.t_pos = s.t_pos;
        }
        match self.kind {
            MngrKind::Soil => self.soil_level(ctrl, cx),
            MngrKind::Ice(_) => match self.base.level {
                3 => self.ice_level3(ctrl, cx),
                4 => self.ice_level4(ctrl, cx),
                _ => {}
            },
            MngrKind::Tree(_) => {
                if self.base.level == 3 {
                    self.soil_level(ctrl, cx);
                } else {
                    self.tree_level4(cx);
                }
            }
            MngrKind::Dark => self.dark_main(cx),
        }
    }

    /// The soil's `_Level3` / `_Level4` (gcmn 0x004f0db0, 0x004f1000) and
    /// the tree's `_Level3` (0x004efde0): the rocks (trees) in waves.
    fn soil_level(&mut self, _ctrl: &mut EffectCtrl, cx: &mut Cx) {
        let c = self.base.count;
        self.base.count = c + 1;
        let tree = matches!(self.kind, MngrKind::Tree(_));
        let four = self.base.level != 3 && !tree;
        let waves: &[(i32, i32)] =
            if four { &[(35, 1), (55, 5), (75, 9), (95, 13)] } else { &[(35, 1), (55, 5), (75, 9)] };
        if let Some(n) = wave(c, waves) {
            let e = if tree { self.entry_tree(cx, n) } else { self.entry_rock(cx, n) };
            if !four && let Some(b) = e.and_then(|i| cx.spells.elements.base_mut(i)) {
                let s = if tree { 0x4040_0000 } else { 0x4000_0000 };
                b.anim.scale[0] = s;
                b.anim.scale[1] = s;
                b.anim.scale[2] = s;
            }
        }
        if c == 49 || c == 69 || c == 89 || (four && c == 109) {
            wave_hit(cx, self.skill, self.t_pos);
        } else if c == if four { 131 } else { 111 } {
            self.base.del_flag = 1;
        }
    }

    /// `ccSoilUpheavalMngrElement::_Entry(n)` (gcmn 0x004f1280): rock n,
    /// life 80.
    fn entry_rock(&mut self, cx: &mut Cx, n: i32) -> Option<usize> {
        let mut e = UpheavalPart::rock(cx);
        e.base.life = 80;
        let (r, s) = match n {
            ..5 => (0x437a_0000, ONE),
            5..9 => (0x4396_0000, 0x4000_0000),
            9..13 => (0x43c8_0000, 0x4040_0000),
            _ => (0x43fa_0000, 0x4060_0000),
        };
        let deg = cx.spells.data.upheaval.rock_deg.get((n - 1) as usize).copied().unwrap_or(0);
        let a = ee::deg2rad(deg);
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let mut p = space::w2p(self.base.anim.sp, player, bounds);
        p[0] = ee::add(p[0], ee::mul(r, ee::cosf(a)));
        p[1] = ee::add(p[1], ee::mul(r, ee::sinf(a)));
        let p = space::p2w(p, player, bounds);
        let an = &mut e.base.anim;
        an.pos = p;
        an.sp = p;
        an.scale[0] = s;
        an.scale[1] = s;
        an.scale[2] = s;
        an.dirc = heading(a);
        cx.spells.elements.add(Element::UpheavalPart(Box::new(e)))
    }

    /// `ccTreeUpheavalMngrElement::_EntryLevel4(n)` (gcmn 0x004f0740):
    /// tree n, life 80.
    fn entry_tree(&mut self, cx: &mut Cx, n: i32) -> Option<usize> {
        let mut e = UpheavalPart::tree(cx);
        e.base.life = 80;
        let r = match n {
            ..5 => 0x43e1_0000,
            5..9 => 0x43fa_0000,
            9..13 => 0x4416_0000,
            _ => 0x442f_0000,
        };
        let deg = cx.spells.data.upheaval.tree_deg.get((n - 1) as usize).copied().unwrap_or(0);
        let a = ee::deg2rad(deg);
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let mut p = space::w2p(self.base.anim.sp, player, bounds);
        p[0] = ee::add(p[0], ee::mul(r, ee::cosf(a)));
        p[1] = ee::add(p[1], ee::mul(r, ee::sinf(a)));
        let p = space::p2w(p, player, bounds);
        let an = &mut e.base.anim;
        an.pos = p;
        an.sp = p;
        an.dirc = heading(a);
        cx.spells.elements.add(Element::UpheavalPart(Box::new(e)))
    }

    /// `ccDarkUpheavalMngrElement::Main` (gcmn 0x004edce0).
    fn dark_main(&mut self, cx: &mut Cx) {
        let c = self.base.count;
        self.base.count = c + 1;
        let level = self.base.level;
        if c == 0 {
            cx.raise(Event::Sound3dNote { se: 191, pos: self.base.anim.pos, note: 72 });
        }
        if let Some(n) = wave(c, &[(20, 1), (40, 5), (60, 9)]) {
            self.entry_hand(cx, n);
        }
        if level == 4
            && let Some(n) = wave(c, &[(80, 13)])
        {
            self.entry_hand(cx, n);
        }
        if c == 80 || c == 100 || c == 120 || (level == 4 && c == 140) {
            damage2_of(cx, self.skill);
            cx.raise(Event::Sound3dNote { se: 171, pos: self.t_pos, note: 56 });
        }
        if (level == 3 && c == 126) || (level == 4 && c == 146) {
            self.base.del_flag = 1;
        }
    }

    /// `ccDarkUpheavalMngrElement::_Entry(n)` (gcmn 0x004edfa0): hand n,
    /// life 80, round the manager's place (not through the player's
    /// frame), 1 to 2.5 its size.
    fn entry_hand(&mut self, cx: &mut Cx, n: i32) -> Option<usize> {
        let mut e = UpheavalPart::hand(cx);
        e.base.life = 80;
        let r = match n {
            ..5 => 0x4316_0000,
            5..9 => 0x4348_0000,
            9..13 => 0x4396_0000,
            _ => 0x43c8_0000,
        };
        let deg = cx.spells.data.upheaval.hand_deg.get((n - 1) as usize).copied().unwrap_or(0);
        let a = ee::deg2rad(deg);
        let mut p = self.base.anim.pos;
        p[0] = ee::add(p[0], ee::mul(r, ee::cosf(a)));
        p[1] = ee::add(p[1], ee::mul(r, ee::sinf(a)));
        e.base.anim.pos = p;
        e.base.anim.sp = p;
        e.base.anim.dirc = heading(a);
        let s = thunder::add_abs(1.0, thunder::rand_f(cx, 0x3fc0_0000));
        e.base.anim.scale[0] = s;
        e.base.anim.scale[1] = s;
        e.base.anim.scale[2] = s;
        cx.spells.elements.add(Element::UpheavalPart(Box::new(e)))
    }

    fn ice_parts(&mut self) -> &mut IceParts {
        match &mut self.kind {
            MngrKind::Ice(p) => p,
            _ => unreachable!("an ice manager's part"),
        }
    }

    /// `_SetupLevel3` (gcmn 0x004ef360): 32 spikes (ice 6) at the target,
    /// leaning at random, to grow (1, 1, 5) over 8 frames and fade in over
    /// 4.
    fn ice_setup3(&mut self, cx: &mut Cx) {
        let pos = self.base.anim.pos;
        let (ss, es) = (VF0, [ONE, ONE, 0x40a0_0000, ONE]);
        for i in 0..32 {
            let mut e = DrawElm::ice(cx, 6);
            let y = ee::fabsf(thunder::rand_f(cx, PI));
            let z = thunder::rand_f(cx, PI);
            let a = &mut e.base.anim;
            a.pos = pos;
            a.dirc[1] = y;
            a.dirc[2] = z;
            grow(a, ss, es, 0x4100_0000);
            self.ice_parts().elements[i] = IceSlot { elm: Some(e), status: 0 };
        }
    }

    /// `_SetupLevel4` (gcmn 0x004ef530): the great spike (ice 8, (3, 3,
    /// 0.01) to (3, 3, 10) over 15 frames speeding up by 0.05) and 128
    /// spikes 300 out in the player's frame, leaning out, to grow (1, 1, 2).
    fn ice_setup4(&mut self, cx: &mut Cx) {
        let pos = self.base.anim.pos;
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let center = space::w2p(pos, player, bounds);
        let mut big = DrawElm::ice(cx, 8);
        {
            let a = &mut big.base.anim;
            a.pos = pos;
            let ss = [0x4040_0000, 0x4040_0000, 0x3c23_d70a, ONE];
            let es = [0x4040_0000, 0x4040_0000, 0x4120_0000, ONE];
            a.start_scale = ss;
            a.end_scale = es;
            for k in 0..3 {
                a.scale_spd[k] = ee::div(ee::sub(es[k], ss[k]), 0x4170_0000);
            }
            a.scale = ss;
            a.scale_flag = 1;
            a.scale_accel = [0, 0, 0x3d4c_cccd, ONE];
            a.start_fade = 0;
            a.end_fade = ONE;
            a.fade_spd = 0x3d88_8889;
            a.transparency = 0;
            a.fade_flag = 1;
        }
        self.ice_parts().big_ice = Some(big);
        // (float)(pi / 2) as a double.
        let half_pi = f64::from_bits(0x3ff9_21fb_6000_0000);
        for i in 0..128 {
            let mut e = DrawElm::ice(cx, 8);
            let r = thunder::rand_f(cx, 0x3f71_463b);
            let y = ((f64::from(f32::from_bits(r))).abs() - half_pi) as f32;
            let z = thunder::rand_f(cx, PI);
            let mut ang = ee::add(z, 0x3fc9_0fdb);
            if !ee::le(ang, PI) {
                ang = ee::sub(ang, TWO_PI);
            }
            let mut p = center;
            p[0] = ee::add(p[0], ee::mul(0x4396_0000, ee::cosf(ang)));
            p[1] = ee::add(p[1], ee::mul(0x4396_0000, ee::sinf(ang)));
            let a = &mut e.base.anim;
            a.dirc[1] = y.to_bits();
            a.dirc[2] = z;
            a.pos = space::p2w(p, player, bounds);
            grow(a, VF0, [ONE, ONE, 0x4000_0000, ONE], 0x4100_0000);
            self.ice_parts().elements[i] = IceSlot { elm: Some(e), status: 0 };
        }
    }

    /// `_Entry(n, count)` (gcmn 0x004ef2c0): `count` more of the first n
    /// spikes up; the last one's index, or -1 once none is left.
    fn ice_entry(&mut self, n: usize, count: i32) -> i32 {
        let els = &mut self.ice_parts().elements;
        let mut v = -1;
        for _ in 0..count {
            match els[..n].iter().position(|e| e.status == 0) {
                Some(i) => {
                    els[i].status = 1;
                    v = i as i32;
                }
                None => return -1,
            }
        }
        v
    }

    /// The spikes that are up: each one's `Main`, its fade and its growth;
    /// true when all of them are done.
    fn ice_run(&mut self, cx: &mut Cx) -> bool {
        let mut all = true;
        for slot in self.ice_parts().elements.iter_mut() {
            if slot.status == 0 {
                continue;
            }
            let Some(e) = slot.elm.as_mut() else { continue };
            e.main(cx);
            let faded = e.base.anim.animate_fade();
            let scaled = e.base.anim.animate_scale();
            if !(faded && scaled) {
                all = false;
            }
        }
        all
    }

    /// `ccIceUpheavalMngrElement::_Level3` (gcmn 0x004ee9e0).
    fn ice_level3(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        let all = self.ice_run(cx);
        let tp = self.t_pos;
        match self.base.proccess {
            0 => {
                let c = self.base.count;
                self.base.count = c + 1;
                if c == 15 {
                    self.base.count = 0;
                    self.base.proccess += 1;
                }
            }
            1 => {
                let c = self.base.count;
                self.base.count = c + 1;
                let (raise, hit) = (cx.spells.data.upheaval.ice_raise, cx.spells.data.upheaval.ice_hit);
                if raise.contains(&c) {
                    if self.ice_entry(32, 11) == -1 {
                        self.base.proccess += 1;
                        self.base.count = 0;
                    }
                    cx.raise(Event::Sound3dNote { se: 56, pos: tp, note: 67 });
                    if check_camera_shake_range(cx, tp) {
                        camera_shake(cx, 2, 2, 10, 2);
                    }
                    noise(cx, 20);
                }
                if hit.contains(&c) {
                    damage2_of(cx, self.skill);
                }
            }
            2 if all => {
                let c = self.base.count;
                self.base.count = c + 1;
                if c == 30 {
                    cx.raise(Event::Sound3dNote { se: 66, pos: tp, note: 48 });
                    cx.raise(Event::Sound3d { se: 35, pos: tp });
                    if check_camera_shake_range(cx, tp) {
                        camera_shake(cx, 2, 2, 30, 0);
                    }
                    damage2_of(cx, self.skill);
                    self.ice_delete(ctrl, cx);
                }
            }
            _ => {}
        }
    }

    /// `ccIceUpheavalMngrElement::_Level4` (gcmn 0x004eedb0).
    fn ice_level4(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        let c = self.base.count;
        self.base.count = c + 1;
        if let Some(b) = self.ice_parts().big_ice.as_mut() {
            b.main(cx);
        }
        let all = self.ice_run(cx);
        let tp = self.t_pos;
        match self.base.proccess {
            0 => {
                if c == 0 {
                    cx.raise(Event::Sound3d { se: 227, pos: tp });
                } else if c == 15 {
                    let (param, ff) = (cx.spells.data.upheaval.ice_gen, cx.spells.data.upheaval.ice_ff);
                    let pos = self.base.anim.pos;
                    pfx::start_param(cx, param, [Some(ff), None, None, None], |g| {
                        g.pos = pos;
                        g.p_tex_mod = 115;
                    });
                    cx.raise(Event::Flash { time: 15, color: 0x80e0_ffff, rect: [0, 0, 0x4400_0000, 0x43c0_0000] });
                    self.base.proccess += 1;
                    self.base.count = 0;
                }
                if let Some(b) = self.ice_parts().big_ice.as_mut() {
                    b.base.anim.animate_fade();
                }
            }
            1 => {
                if self.ice_parts().big_ice.as_mut().is_none_or(|b| b.base.anim.animate_scale()) {
                    self.base.count = 0;
                    self.base.proccess += 1;
                }
            }
            2 if c & 7 == 7 => {
                // 43 spikes a wave; 42 from Outbreak on (OUT gcmn 0x005067e0).
                let wave = if matches!(cx.assets.volume, Volume::Inf | Volume::Mut) { 43 } else { 42 };
                if self.ice_entry(128, wave) == -1 {
                    self.base.count = 0;
                    self.base.proccess += 1;
                }
                if c == 15 || c == 23 || c == 31 {
                    wave_hit_note(cx, self.skill, tp);
                }
            }
            3 if all && c == 15 => {
                cx.raise(Event::Sound3d { se: 35, pos: tp });
                cx.raise(Event::Sound3dNote { se: 66, pos: tp, note: 48 });
                if check_camera_shake_range(cx, tp) {
                    camera_shake(cx, 2, 2, 10, 0);
                }
                damage2_of(cx, self.skill);
                self.ice_delete(ctrl, cx);
            }
            _ => {}
        }
    }

    /// `ccIceUpheavalMngrElement::Delete()` (gcmn 0x004ee710): 8 pieces of
    /// ice thrown (effRadiateSomething) and 24 more (effRadiateSomething2),
    /// each 1 to 3 its size, 5 rings (161), and m_delFlag. The throw's x
    /// and w are the stack's leftovers in the game; 0 here.
    fn ice_delete(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        let pos = self.base.anim.pos;
        let v = 0x4234_0000;
        let mut r: V4 = [0, 0, 0, 0];
        for second in [false, true] {
            for _ in 0..if second { 24 } else { 8 } {
                r[1] = if second { ee::deg2rad(-16384) } else { 0x3f49_0fdb };
                r[2] = thunder::rand_f(cx, PI);
                let s = thunder::add_abs(1.0, thunder::rand_f(cx, 0x4000_0000));
                let e = if second {
                    debris::eff_radiate_something2(ctrl, cx, pos, r, v, attr::WATER, 4)
                } else {
                    debris::eff_radiate_something(ctrl, cx, pos, r, v, attr::WATER, 4)
                };
                if let Some(k) = e {
                    ctrl.effects[k].scale = [s, s, s, ONE];
                }
            }
        }
        for _ in 0..5 {
            r[1] = 0x3f49_0fdb;
            r[2] = thunder::rand_f(cx, PI);
            ring::eff_summon_ring_element(cx, pos, r, 161);
        }
        self.base.del_flag = 1;
    }

    fn tree_parts(&mut self) -> &mut TreeParts {
        match &mut self.kind {
            MngrKind::Tree(p) => p,
            _ => unreachable!("a tree manager's part"),
        }
    }

    /// `ccTreeUpheavalMngrElement::_SetupLevel4` (gcmn 0x004f0a20): the
    /// great tree (fading in over 15 frames, growing to 5), its roots, and
    /// the leaves' generator 400 above, paused.
    fn tree_setup4(&mut self, cx: &mut Cx) {
        let pos = self.base.anim.pos;
        let big_tree = drawelm::particle_clump(cx, "CMP_x407_1");
        let anm_roots = AnmObj::new(cx, "particle", "ANM_x403");
        let (param, ff) = (cx.spells.data.upheaval.leaf_gen, cx.spells.data.upheaval.leaf_ff);
        let mut at = pos;
        at[2] = ee::add(at[2], 0x43c8_0000);
        let leaf = pfx::start_param(cx, param, [Some(ff), None, None, None], |g| g.pos = at);
        if let Some(g) = cx.particles.get_mut(leaf) {
            g.pause = true;
        }
        let t = self.tree_parts();
        t.big_tree = big_tree;
        let a = &mut t.tree_animate;
        a.start_fade = 0;
        a.end_fade = ONE;
        a.fade_spd = 0x3d88_8889;
        a.transparency = 0;
        a.fade_flag = 1;
        let es = [0x40a0_0000, 0x40a0_0000, 0x40a0_0000, ONE];
        a.start_scale = VF0;
        a.end_scale = es;
        for k in 0..3 {
            a.scale_spd[k] = ee::div(ee::sub(es[k], VF0[k]), 0x4170_0000);
        }
        a.scale = VF0;
        a.scale_flag = 1;
        a.scale_accel = VF0;
        a.pos = pos;
        t.tree_offset = VF0;
        t.anm_roots = anm_roots;
        t.gp_leaf = Some(leaf);
    }

    /// `ccTreeUpheavalMngrElement::_Level4` (gcmn 0x004f0030): the great
    /// tree and its roots drawn; it rises from 500 down at 25 a frame, then
    /// the leaves fall, the trees come up in four waves, and at 131 it fades
    /// and shrinks away over 10 frames.
    fn tree_level4(&mut self, cx: &mut Cx) {
        let sp = self.base.anim.sp;
        let t = self.tree_parts();
        let a = t.tree_animate.clone();
        drawelm::draw_clump(cx, t.big_tree, vu::pos_rot_zyx_scale(a.pos, a.dirc, a.scale), a.transparency);
        if let Some(anm) = t.anm_roots.as_mut() {
            anm.forward(cx);
            anm.draw(cx, vu::pos_rot_zyx_scale(a.pos, a.dirc, ee::vscale(a.scale, 0x3f00_0000)), a.transparency);
        }
        match self.base.proccess {
            0 => {
                let c = self.base.count;
                self.base.count = c + 1;
                let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
                let base = space::w2p(sp, player, bounds);
                let tp = self.t_pos;
                let t = self.tree_parts();
                let mut off = t.tree_offset;
                let mut up = false;
                if c == 0 {
                    off[2] = 0xc3fa_0000;
                    for k in 0..3 {
                        t.tree_animate.speed[k] = 0x4234_0000;
                    }
                    cx.raise(Event::Sound3d { se: 56, pos: tp });
                    cx.raise(Event::Sound3d { se: 67, pos: tp });
                } else {
                    off[2] = ee::add(off[2], 0x41c8_0000);
                    if !ee::lt(off[2], 0) {
                        off[2] = 0;
                        up = true;
                    }
                }
                let t = self.tree_parts();
                t.tree_offset = off;
                t.tree_animate.pos = space::p2w(ee::vadd(base, off), player, bounds);
                let faded = t.tree_animate.animate_fade();
                let scaled = t.tree_animate.animate_scale();
                if faded && scaled && up {
                    if let Some(g) = t.gp_leaf.and_then(|g| cx.particles.get_mut(g)) {
                        g.pause = false;
                    }
                    self.base.count = 0;
                    self.base.proccess += 1;
                }
            }
            1 => {
                let c = self.base.count;
                self.base.count = c + 1;
                if let Some(n) = wave(c, &[(35, 1), (55, 5), (75, 9), (95, 13)]) {
                    self.entry_tree(cx, n);
                }
                if [49, 69, 89, 109].contains(&c) {
                    wave_hit(cx, self.skill, self.t_pos);
                }
                if c == 131 {
                    let a = &mut self.tree_parts().tree_animate;
                    a.start_fade = ONE;
                    a.end_fade = 0;
                    a.fade_spd = 0xbdcc_cccd;
                    a.transparency = ONE;
                    a.fade_flag = 1;
                    let ss = [0x40a0_0000, 0x40a0_0000, 0x40a0_0000, ONE];
                    let es = VF0;
                    a.start_scale = ss;
                    a.end_scale = es;
                    for k in 0..3 {
                        a.scale_spd[k] = ee::div(ee::sub(es[k], ss[k]), 0x4120_0000);
                    }
                    a.scale = ss;
                    a.scale_flag = 1;
                    a.scale_accel = VF0;
                    self.base.proccess += 1;
                }
            }
            2 => {
                let a = &mut self.tree_parts().tree_animate;
                let faded = a.animate_fade();
                let scaled = a.animate_scale();
                if faded && scaled {
                    self.base.del_flag = 1;
                }
            }
            _ => {}
        }
    }
}

/// Level 4 ice's hit: the sound, `cameraShake(0, 2, 10, 0)` in range, then
/// `ccSkillDamage2`.
fn wave_hit_note(cx: &mut Cx, key: u32, t_pos: V4) {
    cx.raise(Event::Sound3dNote { se: 56, pos: t_pos, note: 67 });
    if check_camera_shake_range(cx, t_pos) {
        camera_shake(cx, 0, 2, 10, 0);
    }
    damage2_of(cx, key);
}

/// `SetScaleAnm(ss, es, time)` and `SetFade(0, 1, 4)` as the ice's set-ups
/// write them out: no acceleration, the fade 0.25 a frame.
fn grow(a: &mut Animate, ss: V4, es: V4, time: F) {
    a.start_scale = ss;
    a.end_scale = es;
    for k in 0..3 {
        a.scale_spd[k] = ee::div(ee::sub(es[k], ss[k]), time);
    }
    a.scale = ss;
    a.scale_flag = 1;
    a.scale_accel = VF0;
    a.start_fade = 0;
    a.end_fade = ONE;
    a.fade_spd = 0x3e80_0000;
    a.transparency = 0;
    a.fade_flag = 1;
}

/// What a level 3-4 upheaval piece draws.
#[derive(Clone, Debug, PartialEq)]
pub enum PartKind {
    /// `ccSoilUpheavalElement`: +0x190 `m_rock` (its own clump, one of
    /// four).
    Rock { rock: Option<ObjRef> },
    /// `ccTreeUpheavalElement`: +0x190 `m_tree`.
    Tree { tree: Option<ObjRef> },
    /// `ccDarkHandElement`: +0x190 `m_hand` (ANM_x605, once), +0x1a0
    /// `m_offset`.
    Hand { hand: Option<AnmObj>, offset: V4 },
}

/// A level 3-4 upheaval piece (a `ccDrawElement` in the element manager): the
/// rock and tree (Draw gcmn 0x004fc7c0 / 0x004fcee0) rise from -1170, the hand
/// (0x004f8de0) from -500, slow and bounce, burst or sound at 14 to 17, and
/// sink and fade (docs/engine/effects.md, "UpheavalSystem").
#[derive(Clone, Debug, PartialEq)]
pub struct UpheavalPart {
    pub base: Base,
    pub kind: PartKind,
}

impl UpheavalPart {
    fn fading_in(spd: F) -> Base {
        let mut base = Base::default();
        base.anim.start_fade = 0;
        base.anim.end_fade = ONE;
        base.anim.fade_spd = spd;
        base.anim.transparency = 0;
        base.anim.fade_flag = 1;
        base
    }

    /// `new ccSoilUpheavalElement(-1)` (gcmn 0x004fc5a0): a random rock of
    /// four (abs(ccRand() & 3)).
    fn rock(cx: &mut Cx) -> UpheavalPart {
        let i = (cx.host.genrand() as i32 & 3).wrapping_abs();
        let name = cx.spells.data.upheaval.rocks[i as usize].clone();
        let mut base = Self::fading_in(0x3e00_0000);
        base.attr = attr::SOIL;
        UpheavalPart { base, kind: PartKind::Rock { rock: drawelm::particle_clump(cx, &name) } }
    }

    /// `new ccTreeUpheavalElement(-1)` (gcmn 0x004fcc00): a random tree of
    /// four, twice the size.
    fn tree(cx: &mut Cx) -> UpheavalPart {
        let i = (cx.host.genrand() as i32 & 3).wrapping_abs();
        let name = cx.spells.data.upheaval.trees[i as usize].clone();
        let mut base = Self::fading_in(0x3e00_0000);
        base.attr = attr::WIND;
        for k in 0..3 {
            base.anim.scale[k] = 0x4000_0000;
        }
        UpheavalPart { base, kind: PartKind::Tree { tree: drawelm::particle_clump(cx, &name) } }
    }

    /// `new ccDarkHandElement` (gcmn 0x004f8b60).
    fn hand(cx: &Cx) -> UpheavalPart {
        let base = Self::fading_in(0x3e2a_aaab);
        UpheavalPart { base, kind: PartKind::Hand { hand: AnmObj::new(cx, "particle", "ANM_x605"), offset: VF0 } }
    }

    /// The class `ccEffectElementManager` knows it by.
    pub fn class(&self) -> &'static str {
        match self.kind {
            PartKind::Rock { .. } => "ccSoilUpheavalElement",
            PartKind::Tree { .. } => "ccTreeUpheavalElement",
            PartKind::Hand { .. } => "ccDarkHandElement",
        }
    }

    /// `ccDrawElement::Main` (gcmn 0x004ea670), then the class's `Draw`.
    pub fn main(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        let life = self.base.life;
        if life >= 0 {
            self.base.life = life - 1;
            if life == 0 {
                self.base.del_flag = 1;
            }
        }
        if matches!(self.kind, PartKind::Hand { .. }) { self.hand_draw(cx) } else { self.rise(ctrl, cx) }
    }

    /// The rock's and the tree's `Draw`.
    fn rise(&mut self, ctrl: &mut EffectCtrl, cx: &mut Cx) {
        let c = self.base.count;
        self.base.count = c + 1;
        let a = self.base.anim.clone();
        let (obj, tree) = match self.kind {
            PartKind::Rock { rock } => (rock, false),
            PartKind::Tree { tree } => (tree, true),
            PartKind::Hand { .. } => return,
        };
        drawelm::draw_clump(cx, obj, vu::pos_rot_zyx_scale(a.pos, a.dirc, a.scale), a.transparency);
        let (mut off, mut spd) = (a.offset, a.speed);
        if c == 0 {
            off[2] = 0xc492_4000;
            spd[0] = 0x42c8_0000;
        } else if c < 14 {
            spd[0] = ee::add(spd[0], 0xbf8c_cccd);
            off[2] = ee::add(off[2], spd[0]);
        } else if c < 17 {
            spd[0] = ee::mul(0xbf00_0000, spd[0]);
            off[2] = ee::add(off[2], spd[0]);
        } else if c < 27 {
            spd[0] = 0xc120_0000;
        } else {
            off[2] = ee::add(off[2], spd[0]);
        }
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let an = &mut self.base.anim;
        an.offset = off;
        an.speed = spd;
        an.pos = space::p2w(ee::vadd(space::w2p(an.sp, player, bounds), off), player, bounds);
        match self.base.proccess {
            0 if c < 14 => {
                self.base.anim.animate_fade();
            }
            0 => {
                let pos = self.base.anim.pos;
                let kind = if tree { 2 } else { 0 };
                // The tree throws along the stack's leftovers in the game; 0
                // here.
                let mut r: V4 = [0; 4];
                if !tree {
                    r = VF0;
                    r[0] = ee::deg2rad(-16384);
                    r[2] = thunder::rand_f(cx, PI);
                }
                eff_upheaval_fragment(ctrl, cx, pos, r, 0x4220_0000, 2, kind);
                debris::eff_radiate_something2(ctrl, cx, pos, r, 0x4234_0000, self.base.attr, 4);
                let mut r = VF0;
                r[0] = ee::deg2rad(-16384);
                eff_upheaval_flash(ctrl, cx, pos, r, 0x4220_0000, 6, kind);
                if !tree {
                    ring::eff_summon_ring_element(cx, self.base.anim.sp, self.base.anim.dirc, 202);
                }
                let an = &mut self.base.anim;
                an.start_fade = ONE;
                an.end_fade = 0;
                an.fade_spd = 0xbdcc_cccd;
                an.transparency = ONE;
                an.fade_flag = 1;
                self.base.proccess += 1;
            }
            1 if c > 27 && self.base.anim.animate_fade() => self.base.del_flag = 1,
            _ => {}
        }
    }

    /// `ccDarkHandElement::Draw` (gcmn 0x004f8de0).
    fn hand_draw(&mut self, cx: &mut Cx) {
        let c = self.base.count;
        self.base.count = c + 1;
        let (player, bounds) = (cx.host.player_pos(), cx.host.bounds());
        let base = space::w2p(self.base.anim.sp, player, bounds);
        let PartKind::Hand { hand, offset } = &mut self.kind else { return };
        let mut off = *offset;
        let a = &mut self.base.anim;
        if c == 0 {
            off[2] = 0xc3fa_0000;
            for k in 0..3 {
                a.speed[k] = 0x4234_0000;
            }
        } else if c < 14 {
            let s = ee::sub(a.speed[0], 0x3f8c_cccd);
            for k in 0..3 {
                a.speed[k] = s;
            }
            off[2] = ee::add(off[2], s);
        } else if c < 17 {
            let s = ee::mul(0xbf00_0000, a.speed[0]);
            for k in 0..3 {
                a.speed[k] = s;
            }
            off[2] = ee::add(off[2], s);
        } else if c < 60 {
            if c == 17 {
                cx.raise(Event::Sound3d { se: 56, pos: a.pos });
            }
            for k in 0..3 {
                a.speed[k] = 0xc120_0000;
            }
        } else {
            off[2] = ee::add(off[2], a.speed[0]);
        }
        let a = self.base.anim.clone();
        if let Some(h) = hand.as_mut() {
            h.forward(cx);
            h.draw(cx, vu::pos_rot_zyx_scale(a.pos, a.dirc, a.scale), a.transparency);
        }
        *offset = off;
        self.base.anim.pos = space::p2w(ee::vadd(base, off), player, bounds);
        match self.base.proccess {
            0 if c < 14 => {
                self.base.anim.animate_fade();
            }
            0 => {
                let an = &mut self.base.anim;
                an.start_fade = ONE;
                an.end_fade = 0;
                an.fade_spd = 0xbdcc_cccd;
                an.transparency = ONE;
                an.fade_flag = 1;
                self.base.proccess += 1;
            }
            1 if c >= 61 && self.base.anim.animate_fade() => self.base.del_flag = 1,
            _ => {}
        }
    }
}
