//! What a hit shows over the character it lands on (main effect.cpp and
//! particle.cpp, gcmn battle.cpp): `ccHitMarkDisp` (gcmn 0x005714e0) with the
//! spark, ring (131) and photons (-23) it starts on the struck one's skin
//! facing the attacker, its side flipping each hit (`hitFlip` +0x84); the
//! protect gauge's break and return (`effProtect` main 0x001cf250, effects
//! 19-24); and the attribute guard (`ccParticleAttributeGuard` 0x001bcc40,
//! effect 170, on `effAttributeGuardEntry[20]` until it ends). The rules are
//! in docs/engine/effects.md ("Hits and the numbers over characters").

use std::collections::HashMap;

use piney_data::volume::Volume;

use crate::ee::{self, F, ONE, V4};
use crate::effect::{EffectCtrl, Next, ONE_VECTOR, Obj, check_camera_deg};
use crate::files::ObjRef;
use crate::particle;
use crate::{CharRef, Cx, Event, VecRef, vu};

/// Effect ids.
pub const HIT_RING: i16 = 131;
pub const HIT_PHOTON: i16 = -23;
/// effProtect's: 19-21 broken (ANM_xdhpros0 / m0 / l0), 22-24 the other
/// (ANM_xdhpros1 / m1 / l1).
pub const PROTECT_BROKEN: i16 = 19;
pub const PROTECT_OTHER: i16 = 22;
pub const ATTRIBUTE_GUARD: i16 = 170;
/// `effAttributeCritical`'s: EFF_x061, the word over a weak spot.
pub const ATTRIBUTE_CRITICAL: i16 = 169;
/// `particleGeneratorTbl` row of the attribute critical's sparks (and
/// the critical's).
pub const CRITICAL_GENERATOR: usize = 59;
/// The words over a character (EFF_x055-x059; ids 150-154 share one
/// case): `ccParticleDying`'s, `ccParticleCritical`'s,
/// `ccParticleNoDamage`'s.
pub const WORD_DYING: i16 = 150;
pub const WORD_CRITICAL: i16 = 151;
pub const WORD_NO_DAMAGE: i16 = 152;
/// Their generators: (row, lift over the feet: half the height, or 15 over
/// the head).
const DYING_GENERATORS: [(usize, Lift); 2] = [(60, Lift::OverHead), (206, Lift::Half)];
const CRITICAL_GENERATORS: [(usize, Lift); 1] = [(CRITICAL_GENERATOR, Lift::Half)];
const NO_DAMAGE_GENERATORS: [(usize, Lift); 2] = [(207, Lift::OverHead), (208, Lift::OverHead)];
/// `effSBL` (main .sbss 0x00378ad8): the layer `ccEffectCtrl` makes at
/// priority 60 with sysLayer's view.
pub const EFF_SBL_LAYER: i16 = 60;
/// By `CheckCharAttribute(ch, 1)` 0-7 (anything else as 0):
/// `effAttributeCritical`'s CLUT row (jump table 0x00375000: CLT_x061,
/// CLT_x061c1-c5) and `ccParticleAttributeCritical`'s generator texture row
/// (0x003741b0: CLT_x039c5, c1, EFF_x039, c3, c2, c4).
const CRITICAL_CLUT: [usize; 8] = [237, 237, 237, 238, 239, 240, 241, 242];
const CRITICAL_TEX: [i16; 8] = [209, 209, 209, 205, 44, 207, 206, 208];
/// Sounds.
pub const SE_PROTECT_BROKEN: i32 = 79;
pub const SE_PROTECT_OTHER: i32 = 80;
pub const SE_ATTRIBUTE_GUARD: i32 = 99;
/// `particleGeneratorTbl` row of the hit spark (`ccParticleHitMark`).
pub const HIT_MARK_GENERATOR: usize = 1;
/// `particleCcsAnmTbl` rows: the photon's texture, the ring's CLUTs
/// (CLT_x037, CLT_x037c1).
pub const PHOTON_TEX: i32 = 102;
pub const RING_CLUT: usize = 190;
pub const RING_CLUT_HIT: usize = 191;
/// `effAttributeGuardEntry` (main .bss 0x003feeb0).
pub const GUARD_ENTRIES: usize = 20;

const HALF: F = 0x3f00_0000;

/// The hits' own state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hits {
    /// `ccChar` +0x84 `hitFlip` by character: nothing but `ccHitMarkDisp`
    /// reads or writes it (a character not seen yet has it clear).
    pub flip: HashMap<CharRef, bool>,
    /// `effAttributeGuardEntry[20]`: the characters an attribute guard is
    /// showing on.
    pub guard: [Option<CharRef>; GUARD_ENTRIES],
    /// `particleCcsAnmTbl`.
    pub particle_ccs: Vec<(String, String)>,
    /// `panelCamDist`.
    pub panel_cam_dist: F,
}

impl Hits {
    /// The volume's (`tables::effect`).
    pub fn read(volume: Volume) -> Hits {
        let t = piney_data::tables::effect::of(volume);
        Hits {
            flip: HashMap::new(),
            guard: [None; GUARD_ENTRIES],
            particle_ccs: t
                .ccs_anm()
                .iter()
                .map(|r| (r.ccs.unwrap_or("").to_string(), r.anm.unwrap_or("").to_string()))
                .collect(),
            panel_cam_dist: t.panel_cam_dist().to_bits(),
        }
    }

    /// `effCheckAttributeGuardEntry(ch)` (main 0x001d9510): its index, or
    /// None (-1).
    pub fn check_guard(&self, ch: CharRef) -> Option<usize> {
        self.guard.iter().position(|g| *g == Some(ch))
    }

    /// `effAddAttributeGuardEntry(ch)` (0x001d9560): the first empty entry
    /// (none: nothing).
    pub fn add_guard(&mut self, ch: CharRef) -> Option<usize> {
        let i = self.guard.iter().position(|g| g.is_none())?;
        self.guard[i] = Some(ch);
        Some(i)
    }

    /// `effDelAttributeGuardEntry(ch)` (0x001d95c0): the first entry of ch
    /// cleared.
    pub fn del_guard(&mut self, ch: CharRef) -> Option<usize> {
        let i = self.check_guard(ch)?;
        self.guard[i] = None;
        Some(i)
    }
}

/// `ccParticleAdrs(i)` (main 0x001c2a00): `particleCcsAdrs[i]`, the chunk
/// `particleCcsAnmTbl[i]` names.
pub fn particle_adrs(cx: &Cx, i: usize) -> Option<ObjRef> {
    let (file, chunk) = cx.hits.particle_ccs.get(i)?;
    cx.assets.find(file, chunk)
}

/// The ground angle from `from` to `to` (z ignored) and a quarter turn:
/// `DEG2RAD(RAD2DEG(atan2f(dy, dx)) + 16384)`, and the raw angle.
fn facing(from: V4, to: V4) -> (F, F) {
    let d = ee::vsub([to[0], to[1], 0, to[3]], [from[0], from[1], 0, from[3]]);
    let a = ee::atan2f(d[1], d[0]);
    (ee::deg2rad(ee::rad2deg(a).wrapping_add(16384)), a)
}

/// `ccHitMarkDisp(ch, attacker)` (gcmn 0x005714e0).
pub fn hit_mark_disp(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, attacker: CharRef) {
    if !cx.host.check_target(ch) || !cx.host.check_target(attacker) {
        return;
    }
    let mut p = cx.host.char_pos(ch);
    p[2] = ee::add(p[2], ee::mul(HALF, cx.host.char_height(ch)));
    let mut q = cx.host.char_pos(attacker);
    q[2] = ee::add(q[2], ee::mul(HALF, cx.host.char_height(attacker)));
    let d = ee::normalize(ee::vsub(q, p));
    let at = ee::vadd(ee::vscale(d, cx.host.char_width(ch)), p);
    eff_hit_mark(cx, at);
    let flip = cx.hits.flip.entry(ch).or_insert(false);
    *flip = !*flip;
    let flip = *flip;
    let side = ee::deg2rad(if flip { 4096 } else { -4096 });
    let same = ch == attacker;
    let (quarter, raw) = if same { (0, 0) } else { facing(cx.host.char_pos(ch), cx.host.char_pos(attacker)) };
    let heading = cx.host.char_dirc(ch)[2];
    let turn = if same { heading } else { quarter };
    eff_hit_ring(ctrl, cx, at, [0, side, turn, 0]);
    let turn = if same {
        ee::deg2rad(ee::rad2deg(heading).wrapping_add(if flip { -16384 } else { 16384 }))
    } else if flip {
        raw
    } else {
        ee::deg2rad(ee::rad2deg(raw).wrapping_add(i16::MIN))
    };
    eff_hit_photon(ctrl, cx, at, [ee::deg2rad(-4096), side, turn, 0]);
}

/// `effHitMark(pos)` (main 0x001cc3e0): `ccParticleHitMark(pos)`.
pub fn eff_hit_mark(cx: &mut Cx, pos: V4) {
    particle::cc_particle_hit_mark(cx, pos);
}

/// `effHitRing(pos, rot)` (main 0x001cc410): effect 131 at `pos` turned
/// by `rot` (XYZ order), 15 frames; its clump duplicated
/// (`Duplicate(0x3800)`) with CLT_x037 swapped for CLT_x037c1
/// (`ChangeClut(particleCcsAdrs[191], particleCcsAdrs[190])`).
pub fn eff_hit_ring(ctrl: &mut EffectCtrl, cx: &mut Cx, pos: V4, rot: V4) -> Option<usize> {
    let i = ctrl.new_effect(cx, HIT_RING)?;
    let swap = particle_adrs(cx, RING_CLUT).zip(particle_adrs(cx, RING_CLUT_HIT)).map(|(a, b)| (a.object, b.object));
    let e = &mut ctrl.effects[i];
    e.life_time = 15;
    e.pos = pos;
    e.rot = rot;
    e.zyx_flag = false;
    e.clut_swap = swap;
    Some(i)
}

/// `effHitPhoton(pos, rot)` (main 0x001cc550): controller -23.
pub fn eff_hit_photon(ctrl: &mut EffectCtrl, cx: &mut Cx, pos: V4, rot: V4) -> Option<usize> {
    let i = ctrl.new_effect(cx, HIT_PHOTON)?;
    let e = &mut ctrl.effects[i];
    e.pos = pos;
    e.rot = rot;
    Some(i)
}

/// Effect 131's case of `Main`'s first switch (main 0x001c5ce4).
pub fn ring_pre(ctrl: &mut EffectCtrl, _cx: &mut Cx, i: usize) -> Next {
    let e = &mut ctrl.effects[i];
    let life = ee::from_int(i32::from(e.life_time));
    let cnt = ee::from_int(i32::from(e.cnt));
    let xz = ee::add(HALF, ee::mul(cnt, ee::div(ee::sub(0x4020_0000, HALF), life)));
    e.scale[0] = xz;
    e.scale[2] = xz;
    e.scale[1] = ee::add(0x3ecc_cccd, ee::mul(cnt, ee::div(ee::sub(0x4000_0000, 0x3ecc_cccd), life)));
    let life = i32::from(e.life_time);
    e.fade_out(8, life);
    Next::Draw
}

/// Effect -23's case of the second chain (main 0x001cba88): five photons
/// on counts 0 and 1, then the end.
pub fn photon_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let (cnt, pos, rot) = {
        let e = &ctrl.effects[i];
        (e.cnt, e.pos, e.rot)
    };
    if cnt >= 2 {
        ctrl.effects[i].end_flag = true;
        return;
    }
    for _ in 0..5 {
        let Some(k) = particle::cc_particle_setup(cx, PHOTON_TEX, pos, 12, 0, None) else { continue };
        let r = cx.host.rand() >> 3;
        let tilt = ee::deg2rad((r & 0x7c0) as i16);
        let v = [ee::sinf(tilt), ee::mul(0xbf80_0000, ee::cosf(tilt)), 0, 0];
        let spin = ee::deg2rad(((r >> 3) & 0xff00) as u16 as i16);
        let v = ee::apply(&vu::rot_y(&vu::UNIT, spin), v);
        let mut dirc = ee::normalize(v);
        dirc[3] = ONE;
        let m = vu::rot_z(&vu::rot_x(&vu::UNIT, rot[0]), rot[2]);
        dirc = ee::apply(&m, dirc);
        dirc[3] = ONE;
        let r = (cx.host.rand() >> 3) % 101;
        let speed = ee::sub(0x41f0_0000, ee::div(ee::mul(0x41a8_0000, ee::from_int(r)), 0x42c8_0000));
        // Its generator: effect.cpp's static hitPhotonDummyG (0x003ff1f0).
        let gene = cx.particles.statics[2];
        let p = &mut cx.particles.slots[k];
        p.dirc = dirc;
        p.speed = speed;
        p.velocity = ee::vscale(dirc, speed);
        p.fade_in_d = 1024;
        p.fade_out_d = 113;
        p.gene = gene;
    }
}

/// `ccEffect::MainStr`'s effect -1 (main 0x001d8fe4): the stream's photons,
/// as [`photon_post`] with the stream's static generator
/// (`hitPhotonDummyStrG` 0x003ff300) and the effect's layer.
pub fn photon_post_str(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let (cnt, pos, rot, layer) = {
        let e = &ctrl.effects[i];
        (e.cnt, e.pos, e.rot, e.layer)
    };
    if cnt >= 2 {
        ctrl.effects[i].end_flag = true;
        return;
    }
    for _ in 0..5 {
        let Some(k) = particle::cc_particle_setup(cx, PHOTON_TEX, pos, 12, 0, None) else { continue };
        let r = cx.host.rand() >> 3;
        let tilt = ee::deg2rad((r & 0x7c0) as i16);
        let v = [ee::sinf(tilt), ee::mul(0xbf80_0000, ee::cosf(tilt)), 0, 0];
        let spin = ee::deg2rad(((r >> 3) & 0xff00) as u16 as i16);
        let v = ee::apply(&vu::rot_y(&vu::UNIT, spin), v);
        let mut dirc = ee::normalize(v);
        dirc[3] = ONE;
        let m = vu::rot_z(&vu::rot_x(&vu::UNIT, rot[0]), rot[2]);
        dirc = ee::apply(&m, dirc);
        dirc[3] = ONE;
        let r = (cx.host.rand() >> 3) % 101;
        let speed = ee::sub(0x41f0_0000, ee::div(ee::mul(0x41a8_0000, ee::from_int(r)), 0x42c8_0000));
        let gene = cx.particles.statics[3];
        let p = &mut cx.particles.slots[k];
        p.dirc = dirc;
        p.speed = speed;
        p.velocity = ee::vscale(dirc, speed);
        p.fade_in_d = 1024;
        p.fade_out_d = 113;
        p.gene = gene;
        p.layer = layer;
        // +0x02's low nibble (strFlag) 1, before the rand()s in the game.
        p.str_flag = 1;
    }
}

/// `effProtect(ch, broken, kind)` (main 0x001cf250). A `kind` -1 whose
/// size gives none leaves the game's id uninitialised (whatever `s0`
/// held): the port starts nothing.
pub fn eff_protect(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, broken: i32, kind: i32) -> Option<usize> {
    let kind = if kind == -1 {
        match cx.host.object_size(ch) {
            1 => 2,
            3 => 1,
            4 => 0,
            _ => -1,
        }
    } else {
        kind
    };
    if !(0..=2).contains(&kind) {
        return None;
    }
    let id = if broken != 0 { PROTECT_OTHER } else { PROTECT_BROKEN } + kind as i16;
    let i = ctrl.new_effect(cx, id)?;
    let pos = cx.host.char_pos(ch);
    let e = &mut ctrl.effects[i];
    e.pos = pos;
    e.target = Some(ch);
    e.pos_t = pos;
    e.pos_ptr = Some(VecRef::CharPos(ch));
    if broken != 0 {
        cx.events.push(Event::Sound3d { se: SE_PROTECT_OTHER, pos });
    }
    Some(i)
}

/// Effects 19-21's case of the second chain (main 0x001ca628): sound 79
/// at count 30.
pub fn protect_post(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) {
    let e = &ctrl.effects[i];
    if e.cnt == 30 {
        cx.events.push(Event::Sound3d { se: SE_PROTECT_BROKEN, pos: e.pos });
    }
}

/// `effAttributeGuard(ch, attacker)` (main 0x001cc890). A size other than
/// 1, 3 or 4 scales by whatever the caller left in `f20`; the port uses 1.
pub fn eff_attribute_guard(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, attacker: CharRef) -> Option<usize> {
    if !cx.host.check_target(ch) || !cx.host.check_target(attacker) {
        return None;
    }
    let i = ctrl.new_effect(cx, ATTRIBUTE_GUARD)?;
    let pos = cx.host.char_pos(ch);
    let mut p = pos;
    p[2] = ee::add(p[2], ee::mul(HALF, cx.host.char_height(ch)));
    let turn = if ch == attacker { cx.host.char_dirc(ch)[2] } else { facing(pos, cx.host.char_pos(attacker)).0 };
    let s = match cx.host.object_size(ch) {
        1 => 0x4060_0000,
        3 => 0x4000_0000,
        _ => ONE,
    };
    let e = &mut ctrl.effects[i];
    e.target = Some(ch);
    e.pos_t = pos;
    e.pos = p;
    e.rot = [0, 0, turn, 0];
    e.scale = ee::vscale(ONE_VECTOR, s);
    cx.events.push(Event::Sound3d { se: SE_ATTRIBUTE_GUARD, pos });
    Some(i)
}

/// `effAttributeCritical(ch)` (main 0x001ccb40): effect 169 (EFF_x061)
/// over `ch` (offset half its height, following `posT`), 50 frames,
/// bouncing at 10; its `ccEff` drawn with the element's CLUT
/// (`particleCcsAdrs[237 + ...]` into `ccTex` +0x3c), depth test off
/// (`SetRenderState(0, 0)`), on `effSBL` (priority 60).
pub fn eff_attribute_critical(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef) -> Option<usize> {
    let i = ctrl.new_effect(cx, ATTRIBUTE_CRITICAL)?;
    let pos = cx.host.char_pos(ch);
    let h = ee::mul(HALF, cx.host.char_height(ch));
    let a = cx.host.char_attribute(ch, 1);
    let row = usize::try_from(a).ok().and_then(|a| CRITICAL_CLUT.get(a)).copied().unwrap_or(237);
    let clut = particle_adrs(cx, row);
    let e = &mut ctrl.effects[i];
    e.target = Some(ch);
    e.pos_t = pos;
    e.offset = [0, 0, h, 0];
    e.life_time = 50;
    e.velocity = 0x4120_0000;
    e.temp[0] = 0;
    if let Obj::Eff(eff) = &mut e.obj {
        // The element's palette into its ccEff's ccTex.clutChunk (+0x3c).
        eff.clut = clut;
        // ccEff::SetRenderState(0, 0) (main 0x0013bb40): TEST_1's ZTE.
        eff.test &= !(1 << 16);
    }
    e.layer = Some(EFF_SBL_LAYER);
    Some(i)
}

/// `ccParticleAttributeCritical(ch)` (main 0x001bca90): the word, a
/// generator of `particleGeneratorTbl[59]` following its `posT` (offset half
/// the height, the element's texture row at +0x2c), and `cameraShake(0, 2,
/// 2, 0)` when `ch` is within 2000 of the eye and in the camera's cone
/// (`checkCameraShakeRange`, main 0x00162f10).
pub fn particle_attribute_critical(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef) -> Option<usize> {
    let a = cx.host.char_attribute(ch, 1);
    let tex = usize::try_from(a).ok().and_then(|a| CRITICAL_TEX.get(a)).copied().unwrap_or(209);
    let i = eff_attribute_critical(ctrl, cx, ch)?;
    let h = ee::mul(HALF, cx.host.char_height(ch));
    let mut g = cx.particles.generator(cx.assets, CRITICAL_GENERATOR);
    g.sync_pos_type = false;
    g.sync_pos = Some(VecRef::EffectPosT(i));
    g.offset = [0, 0, h, 0];
    g.p_tex_mod = tex;
    cx.particles.start(g);
    let pos = cx.host.char_pos(ch);
    if camera_shake_range(cx, pos) {
        cx.events.push(Event::CameraShake([0, 2, 2, 0]));
    }
    Some(i)
}

#[derive(Clone, Copy)]
pub(crate) enum Lift {
    /// z + height / 2.
    Half,
    /// z + 15 + height.
    OverHead,
}

/// `ccParticleCritical(ch)` (main 0x001bc540): sparks
/// (`particleGeneratorTbl[59]` on ch's pos, half its height up), the word
/// (effect 151), and `cameraShake(0, 2, 2, 0)` as the attribute critical
/// has it.
pub fn particle_critical(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef) -> Option<usize> {
    let i = particle_word(ctrl, cx, ch, &CRITICAL_GENERATORS, WORD_CRITICAL);
    let pos = cx.host.char_pos(ch);
    if camera_shake_range(cx, pos) {
        cx.events.push(Event::CameraShake([0, 2, 2, 0]));
    }
    i
}

/// `ccParticleDying(ch)` (main 0x001bc6d0): generators 60 (15 over the
/// head) and 206 (half the height), the word (effect 150).
pub fn particle_dying(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef) -> Option<usize> {
    particle_word(ctrl, cx, ch, &DYING_GENERATORS, WORD_DYING)
}

/// `ccParticleNoDamage(ch)` (main 0x001bc8c0): generators 207 and 208 (both
/// 15 over the head), the word (effect 152).
pub fn particle_no_damage(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef) -> Option<usize> {
    particle_word(ctrl, cx, ch, &NO_DAMAGE_GENERATORS, WORD_NO_DAMAGE)
}

/// The three's common body: each generator on ch's pos (syncPosType 0,
/// `ccParticleCtrlAddGenerator`), then `ccNewEffect(id)`: offset ch's pos
/// with z + height / 2 (a point, which the case draws from), 50 frames,
/// bouncing at 10, depth test off, on `effSBL`.
pub(crate) fn particle_word(
    ctrl: &mut EffectCtrl,
    cx: &mut Cx,
    ch: CharRef,
    gens: &[(usize, Lift)],
    id: i16,
) -> Option<usize> {
    let h = cx.host.char_height(ch);
    for &(row, lift) in gens {
        let z = match lift {
            Lift::Half => ee::mul(HALF, h),
            Lift::OverHead => ee::add(0x4170_0000, h),
        };
        let mut g = cx.particles.generator(cx.assets, row);
        g.sync_pos_type = false;
        g.sync_pos = Some(VecRef::CharPos(ch));
        g.offset = [0, 0, z, 0];
        cx.particles.start(g);
    }
    let i = ctrl.new_effect(cx, id)?;
    let mut p = cx.host.char_pos(ch);
    p[2] = ee::add(p[2], ee::mul(HALF, h));
    let e = &mut ctrl.effects[i];
    e.offset = p;
    e.life_time = 50;
    e.velocity = 0x4120_0000;
    e.temp[0] = 0;
    if let Obj::Eff(eff) = &mut e.obj {
        eff.test &= !(1 << 16);
    }
    e.layer = Some(EFF_SBL_LAYER);
    Some(i)
}

/// Effects 150-154's case of `Main`'s first switch (main 0x001c5d90):
/// 169's, from the offset (a point) itself.
pub fn word_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let cam = cx.host.camera().cam_pos;
    let dist = cx.hits.panel_cam_dist;
    let e = &mut ctrl.effects[i];
    if appear(e) {
        let base = e.offset;
        bounce(e, base, cam, dist);
    }
    Next::Draw
}

/// The words' first frames: hidden (the sprite's scale 0) until count 3,
/// then 2.5, fading in over 3 frames and out over the last 5. False while
/// hidden.
fn appear(e: &mut crate::effect::Effect) -> bool {
    appear_after(e, 3, 0x4020_0000)
}

/// [`appear`] with the level up's numbers too (0x001c60cc: hidden until
/// count 50, then 3): hidden (scale 0) until count `delay`, then `scale`,
/// fading in over 3 frames and out over the last 5.
pub(crate) fn appear_after(e: &mut crate::effect::Effect, delay: i16, scale: F) -> bool {
    let shown = e.cnt >= delay;
    if let Obj::Eff(eff) = &mut e.obj {
        let s = if shown { scale } else { 0 };
        eff.scale_y = s;
        eff.scale_x = s;
    }
    if !shown {
        return false;
    }
    let s5 = i32::from(e.cnt) - i32::from(delay);
    if s5 <= 3 {
        e.fade_in(3, s5);
    } else {
        let life = i32::from(e.life_time);
        e.fade_out(5, life);
    }
    true
}

/// The words' place and bounce: `base` pulled to `dist` from the eye, z
/// less `velocity` sin(temp), temp on by 72 degrees a frame, the bounce 0.7
/// as high each time the angle's sign turns, still under 0.5.
pub(crate) fn bounce(e: &mut crate::effect::Effect, base: V4, cam: V4, dist: F) {
    e.pos = trans_pos_w2czw(base, cam, dist);
    let old = e.temp[0] as u16 as i16;
    e.temp[0] = e.temp[0].wrapping_add(0x10000 / 5);
    let new = e.temp[0] as u16 as i16;
    e.pos[2] = ee::sub(e.pos[2], ee::mul(e.velocity, ee::sinf(ee::deg2rad(new))));
    if (old >= 0) != (new >= 0) {
        e.velocity = ee::mul(e.velocity, 0x3f33_3333);
    }
    if ee::lt(e.velocity, HALF) {
        e.velocity = 0;
    }
}

/// `checkCameraShakeRange(pos)` (main 0x00162f10): in the camera's cone
/// (`ccCheckCameraDeg(pos, 12288)`) and nearer the active camera's eye than
/// 2000.
pub fn camera_shake_range(cx: &Cx, pos: V4) -> bool {
    let cam = cx.host.camera();
    if !check_camera_deg(pos, cx.host.player_pos(), cx.host.bounds(), cam.cam_pos, cam.cam_view, 12288) {
        return false;
    }
    let d = ee::vsub(pos, cam.cam_pos);
    ee::lt(ee::sqrtf_on(cx.assets.volume, ee::dot(d, d)), 0x44fa_0000)
}

/// `ccTransPosW2CZW(out, pos, dist)` (main 0x001633c0): the point `dist`
/// from the active camera's eye towards `pos`.
pub fn trans_pos_w2czw(pos: V4, eye: V4, dist: F) -> V4 {
    let v = ee::vscale(ee::normalize(ee::vsub(pos, eye)), dist);
    let mut out = ee::vadd(v, eye);
    out[3] = ONE;
    out
}

/// Effect 169's case of `Main`'s first switch (main 0x001c5f0c): hidden
/// (scale 0) for 3 frames, then 2.5, in over 3 frames and out over the last
/// 5; at `posT` + offset pulled to 1000 from the eye (drawn over the
/// scene), bouncing: z less `velocity` sin(temp), temp on by 72 degrees a
/// frame, the bounce 0.7 as high each time the angle's sign turns, still
/// under 0.5.
pub fn critical_pre(ctrl: &mut EffectCtrl, cx: &mut Cx, i: usize) -> Next {
    let cam = cx.host.camera().cam_pos;
    let dist = cx.hits.panel_cam_dist;
    let target = ctrl.effects[i].target.filter(|&t| cx.host.check_target(t));
    let tpos = target.map(|t| cx.host.char_pos(t));
    let e = &mut ctrl.effects[i];
    if appear(e) {
        if let Some(p) = tpos {
            e.pos_t = p;
        }
        let mut base = ee::vadd(e.pos_t, e.offset);
        base[3] = ONE;
        bounce(e, base, cam, dist);
    }
    Next::Draw
}

/// `ccParticleAttributeGuard(ch, attacker)` (main 0x001bcc40).
pub fn particle_attribute_guard(ctrl: &mut EffectCtrl, cx: &mut Cx, ch: CharRef, attacker: CharRef) -> Option<usize> {
    if !cx.host.check_target(ch) || !cx.host.check_target(attacker) || cx.hits.check_guard(ch).is_some() {
        return None;
    }
    let i = eff_attribute_guard(ctrl, cx, ch, attacker)?;
    cx.hits.add_guard(ch);
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{Camera, DrawRec};
    use crate::host::{CharState, Simple};

    #[test]
    fn a_hit_shows_a_ring_and_photons() {
        let Some(mut fx) = crate::testing::effects() else { return };
        fx.ctrl.town = false;
        let mut rng = piney_world::Rand(7);
        let mut draws = 0;
        let mut next = || {
            draws += 1;
            rng.rand()
        };
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
        host.chars.push(c(1, 0.0, 0.0));
        host.chars.push(c(2, 300.0, 0.0));
        fx.hit_mark(&mut host, 1, 2);
        // The spark 40 towards the attacker at half height (one ulp short:
        // the EE's division truncates).
        let marks: Vec<V4> = fx.particles.started.iter().map(|g| g.pos).collect();
        assert_eq!(marks, [[0x421f_ffff, 0, ee::k(80.0), ONE]]);
        let ids: Vec<i16> = fx.ctrl.effects.iter().filter(|e| e.status != 0).map(|e| e.id).collect();
        assert_eq!(ids, [HIT_RING, HIT_PHOTON]);
        assert!(fx.ctrl.effects[0].clut_swap.is_some());
        let mut rings = 0;
        let mut last = 0;
        for f in 0..30 {
            fx.step(&mut host);
            rings += fx.draws().iter().filter(|d| matches!(d, DrawRec::Clump { clut: Some(_), .. })).count();
            if fx.ctrl.effects.iter().any(|e| e.status != 0) {
                last = f;
            }
        }
        let dummy = fx.particles.statics[2];
        let photons = fx.particles.live().filter(|(_, p)| p.gene == dummy && p.life_time == 12).count();
        assert_eq!(photons, 10);
        assert_eq!(rings, 17);
        assert_eq!(last, 16);
        drop(host);
        assert_eq!(draws, 20);
    }
}
