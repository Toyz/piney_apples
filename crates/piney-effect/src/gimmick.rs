//! The dungeon objects' effects (main effect.cpp): a virus core's crystal,
//! the breakables smashed, a trapped box going off and the Statue of God's
//! glow, which `ccGimBox` and `ccGimIdol` (gcmn gmbox.cpp) start.
//!
//! ```text
//! effVirusCrystal(pos)        (main 0x001cf420) the controller -4 at pos,
//!                             life 60; a generator 109 at pos
//! -4's second chain (0x001c96f8): at count 13 and 20 a generator 109 at its
//!   pos
//! effWoodFragment(p, r, v, s, n, x)  (main 0x001cf540; Eggshell 0x001cf8b0,
//!   Pot 0x001cfc20, Bone 0x001cff90 the same with other ids) n pieces:
//!     rn = rand() >> 3
//!     d = Rot(r) Rot(0, (rn << 8) & 0xfc00, rn % 12288 + 4096) (0, -1, 0, 1)
//!     id base + (rn & 15) % 3 (wood 25, eggshell 138, pot 142, bone 146)
//!     speed d v (100 - (rn >> 8) % 50) / 100, velocity v
//!     rotSpeed x (rn >> 4) & 0x3c00, z (rn >> 8) & 0x3c00; rot x and z
//!       on by three times those
//!     pos p + |speed| s (100 - (rn >> 16) % 50) / 100, life 90
//!     x not 0: its clump duplicated (Duplicate(0x3800)) with CLT_x036
//!       swapped for CLT_x036c1 (WoodClt1, WoodClt2)
//!   the last piece
//! effCrushBarrel(pos, x)      (main 0x001d0300; Egg, Pot, Corpse the same
//!   with the other fragments) the fragments (pos, (-pi/2, 0, 0), 18, 50,
//!   15, x); a generator 110 at pos
//! effOpenTrapBox(pos, kind, trap)  (main 0x001d0780) the controller -5 at
//!   pos, life 5, param 1 for trap 0; kind not 0 (a wooden box or barrel):
//!   the wood fragments (pos, (-pi/2, 0, 0), 18, 50, 15, kind); a
//!   generator 110 + trap at pos; ccSeOn3D(39, pos)
//! -5's second chain (0x001c9770), every frame:
//!     rn = rand() >> 3
//!     q = pos + Rot(0, (rn << 8) & 0xfc00, rn % 12288 + 4096)
//!           (0, -75 (100 - (rn >> 16) % 20) / 100, 0, 1), w 1
//!     ccParticleExplode(q, speed, 1.8, param)
//! effStatueOfGod(pos, sw)     (main 0x001d0e60) a generator 125 at pos
//!   running while *sw (the idol's effsw) is not 0
//! ```
//!
//! Each crush also copies `pos` 50 up and never uses it.

use crate::ee::{self, F, ONE, V4};
use crate::effect::EffectCtrl;
use crate::{CharRef, Cx, Event, IntRef, debris, particle, pfx, vu};

/// The controllers.
pub const VIRUS_CRYSTAL: i16 = -4;
pub const OPEN_TRAP_BOX: i16 = -5;

/// `particleGeneratorTbl` rows.
pub const VIRUS_CRYSTAL_ROW: usize = 109;
/// A crush's, and a trap box's 110 + trap (0, 3 or 4).
pub const CRUSH_ROW: usize = 110;
pub const STATUE_OF_GOD_ROW: usize = 125;
/// `effUseSymbol`'s two generators (`particleGeneratorTbl` +0x1ae8 and
/// +0x1b20).
pub const USE_SYMBOL_ROWS: [usize; 2] = [123, 124];

/// `ccSeOn3D(39, pos)`: a trap going off.
pub const SE_OPEN_TRAP_BOX: i32 = 39;

/// The idol's `effsw` (`ccGimIdol` +0x1e0), the glow's switch.
pub const IDOL_EFFSW: u16 = 0x1e0;

/// The breakables' pieces: the first of three effect ids each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fragment {
    /// `effWoodFragment`: a barrel or wooden box (CMP_x036).
    Wood = 25,
    /// `effEggshellFragment`: an egg.
    Eggshell = 138,
    /// `effPotFragment`: a pot.
    Pot = 142,
    /// `effBoneFragment`: a body.
    Bone = 146,
}

impl Fragment {
    /// `ccGimBox::breakObject`'s effect by what broke (0 barrels and
    /// wooden boxes, 1 pots, 2 bodies, 3 eggs).
    pub fn of_crush(what: i32) -> Option<Fragment> {
        match what {
            0 => Some(Fragment::Wood),
            1 => Some(Fragment::Pot),
            2 => Some(Fragment::Bone),
            3 => Some(Fragment::Eggshell),
            _ => None,
        }
    }
}

/// 18.0 and 50.0: a crush's speed and spread.
const CRUSH_V: F = 0x4190_0000;
const CRUSH_S: F = 0x4248_0000;
const CRUSH_N: i32 = 15;

/// `effVirusCrystal(pos)` (main 0x001cf420).
pub fn eff_virus_crystal(ctrl: &mut EffectCtrl, cx: &mut Cx, pos: V4) -> Option<usize> {
    let i = ctrl.new_effect(cx, VIRUS_CRYSTAL)?;
    let e = &mut ctrl.effects[i];
    e.life_time = 60;
    e.pos = pos;
    pfx::start(cx, VIRUS_CRYSTAL_ROW, |g| g.pos = pos);
    Some(i)
}

/// -4's case of the second chain (main 0x001c96f8). The 20 is a register
/// the chain's compares left holding it.
pub fn virus_crystal_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &ctrl.effects[i];
    if e.cnt == 13 || e.cnt == 20 {
        let pos = e.pos;
        pfx::start(cx, VIRUS_CRYSTAL_ROW, |g| g.pos = pos);
    }
}

/// The share `(100 - m) / 100`.
fn share(m: i32) -> F {
    let h = 0x42c8_0000;
    ee::div(ee::sub(h, ee::from_int(m)), h)
}

/// The tilt the fragments and the trap's sparks take: (0, (rn << 8) &
/// 0xfc00, rn % 12288 + 4096) as 16-bit angles.
fn tilt(rn: i32) -> vu::M4 {
    let a = [0, ee::deg2rad(((rn << 8) & 0xfc00) as i16), ee::deg2rad((rn % 12288 + 4096) as i16), 0];
    vu::rot_zyx(&vu::UNIT, a)
}

/// `effWoodFragment(p, r, v, s, n, x)` and the three like it (main
/// 0x001cf540-0x001d02f4): the last piece.
#[allow(clippy::too_many_arguments)]
pub fn eff_fragment(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    kind: Fragment,
    p: V4,
    r: V4,
    v: F,
    s: F,
    n: i32,
    x: i32,
) -> Option<usize> {
    let swap = if x != 0 { wood_clut(cx) } else { None };
    let mut last = None;
    for _ in 0..n {
        let rn = cx.host.rand() >> 3;
        let d = ee::apply(&tilt(rn), [0, 0xbf80_0000, 0, ONE]);
        let d = ee::apply(&vu::rot_zyx(&vu::UNIT, r), d);
        last = ctrl.new_effect(cx, kind as i16 + ((rn & 0xf) % 3) as i16);
        let Some(k) = last else { continue };
        let e = &mut ctrl.effects[k];
        e.speed = ee::vscale(d, ee::mul(v, share((rn >> 8) % 50)));
        e.velocity = v;
        e.rot_speed[0] = ((rn >> 4) & 0x3c00) as u16;
        e.rot_speed[2] = ((rn >> 8) & 0x3c00) as u16;
        e.rot[0] = ee::add(e.rot[0], ee::deg2rad((i32::from(e.rot_speed[0]) * 3) as i16));
        e.rot[2] = ee::add(e.rot[2], ee::deg2rad((i32::from(e.rot_speed[2]) * 3) as i16));
        let out = ee::vscale(ee::normalize(e.speed), ee::mul(s, share((rn >> 16) % 50)));
        e.life_time = 90;
        e.pos = ee::vadd(out, p);
        if x != 0 {
            e.clut_swap = swap;
        }
    }
    last
}

/// `WoodClt1` and `WoodClt2` (0x00378ac0, 0x00378ac4): CLT_x036 and
/// CLT_x036c1 in the file of `effectTbl` row 25, which
/// `ccEffectCtrl::ccEffectCtrl` reads outside the towns; (the one drawn
/// instead of, the one drawn).
fn wood_clut(cx: &Cx) -> Option<(u32, u32)> {
    let o = cx.assets.adrs(Fragment::Wood as i16, false)?;
    let ccs = &cx.assets.files[o.file].ccs;
    Some((ccs.find_object("CLT_x036")?, ccs.find_object("CLT_x036c1")?))
}

/// `effCrushBarrel(pos, x)`, `effCrushEgg`, `effCrushPot`,
/// `effCrushCorpse` (main 0x001d0300-0x001d0770).
pub fn eff_crush(ctrl: &mut EffectCtrl, cx: &mut Cx, kind: Fragment, pos: V4, x: i32) {
    eff_fragment(ctrl, cx, kind, pos, debris::down(), CRUSH_V, CRUSH_S, CRUSH_N, x);
    pfx::start(cx, CRUSH_ROW, |g| g.pos = pos);
}

/// `effOpenTrapBox(pos, kind, trap)` (main 0x001d0780): kind 0 a treasure
/// box, 1 a wooden box or barrel; trap 0 (the hit), 3 or 4 (the skills).
pub fn eff_open_trap_box(ctrl: &mut EffectCtrl, cx: &mut Cx, pos: V4, kind: i32, trap: i32) -> Option<usize> {
    let i = ctrl.new_effect(cx, OPEN_TRAP_BOX)?;
    let e = &mut ctrl.effects[i];
    e.life_time = 5;
    e.pos = pos;
    e.param = i32::from(trap == 0);
    if kind != 0 {
        eff_fragment(ctrl, cx, Fragment::Wood, pos, debris::down(), CRUSH_V, CRUSH_S, CRUSH_N, kind);
    }
    if let Ok(row) = usize::try_from(CRUSH_ROW as i32 + trap) {
        pfx::start(cx, row, |g| g.pos = pos);
    }
    cx.raise(Event::Sound3d { se: SE_OPEN_TRAP_BOX, pos });
    Some(i)
}

/// -5's case of the second chain (main 0x001c9770): a spark thrown up to
/// 75 out from its pos, every frame of its five.
pub fn open_trap_box_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    const REACH: F = 0x4296_0000;
    const SCALE: F = 0x3fe6_6666;
    let rn = cx.host.rand() >> 3;
    let m = tilt(rn);
    let y = ee::mul(ee::mul(0xbf80_0000, share((rn >> 16) % 20)), REACH);
    let e = &ctrl.effects[i];
    let mut q = ee::vadd(ee::apply(&m, [0, y, 0, ONE]), e.pos);
    q[3] = ONE;
    let (speed, param) = (e.speed, e.param);
    particle::cc_particle_explode(cx, q, speed, SCALE, param);
}

/// `effUseSymbol(pos)` (main 0x001d0d70): a symbol cast, generators 123
/// and 124 at `pos`.
pub fn eff_use_symbol(cx: &mut Cx, pos: V4) {
    for row in USE_SYMBOL_ROWS {
        pfx::start(cx, row, |g| g.pos = pos);
    }
}

/// `effFountain(pos, &effsw)` (main 0x001d0ae0)'s generators
/// (`particleGeneratorTbl` +0x1a40, +0x1a78), and
/// `effBossRoomEntrance(pos, &effsw)`'s (0x001d0be0, +0x1ab0).
pub const FOUNTAIN_ROWS: [usize; 2] = [120, 121];
pub const BOSS_ROOM_ENTRANCE_ROW: usize = 122;
/// `effDungeonEntrance`'s generator (`particleGeneratorTbl` +0x3410) and
/// `effDungeonEntranceColor[4]` (main 0x003401f0): its `pTexMod` by kind
/// (-1: the row's own).
pub const DUNGEON_ENTRANCE_ROW: usize = 238;
pub const DUNGEON_ENTRANCE_COLOR: [i16; 4] = [-1, 0x66, 0x67, 0x68];
/// `ccGimEtc`'s `effsw` (+0x1e0), the same word as the idol's.
pub const ETC_EFFSW: u16 = 0x1e0;

/// `effFountain(pos, &effsw)` (main 0x001d0ae0): the Spring of Myst's
/// mist, generators 120 and 121 at `pos` while the spring's `effsw`
/// holds.
pub fn eff_fountain(cx: &mut Cx, pos: V4, spring: CharRef) {
    for row in FOUNTAIN_ROWS {
        pfx::start(cx, row, |g| {
            g.pos = pos;
            g.sync_sw = Some(IntRef::CharAt(spring, ETC_EFFSW));
        });
    }
}

/// `effBossRoomEntrance(pos, &effsw)` (main 0x001d0be0): the warning at a
/// boss room's door, generator 122 while its `effsw` holds.
pub fn eff_boss_room_entrance(cx: &mut Cx, pos: V4, warning: CharRef) {
    pfx::start(cx, BOSS_ROOM_ENTRANCE_ROW, |g| {
        g.pos = pos;
        g.sync_sw = Some(IntRef::CharAt(warning, ETC_EFFSW));
    });
}

/// `effDungeonEntrance(pos, &effsw, kind)` (main 0x001d0c90): the swirl at
/// a field's dungeon entrance (`ccGimEtc` row 21, kind 2), generator 238
/// at `pos` with `pTexMod` `effDungeonEntranceColor[kind]` (stored only
/// when not -1, which the generator starts with anyway), while the
/// entrance's `effsw` holds.
pub fn eff_dungeon_entrance(cx: &mut Cx, pos: V4, entrance: CharRef, kind: usize) {
    pfx::start(cx, DUNGEON_ENTRANCE_ROW, |g| {
        g.pos = pos;
        g.sync_sw = Some(IntRef::CharAt(entrance, ETC_EFFSW));
        g.p_tex_mod = DUNGEON_ENTRANCE_COLOR.get(kind).copied().unwrap_or(-1);
    });
}

/// `effStatueOfGod(pos, &effsw)` (main 0x001d0e60): the glow over a Statue
/// of God, `idol`'s `effsw` its switch.
pub fn eff_statue_of_god(cx: &mut Cx, pos: V4, idol: CharRef) {
    pfx::start(cx, STATUE_OF_GOD_ROW, |g| {
        g.pos = pos;
        g.sync_sw = Some(IntRef::CharAt(idol, IDOL_EFFSW));
    });
}
