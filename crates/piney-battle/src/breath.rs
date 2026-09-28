//! The enemies' fire breath (gcmn 0x0043ae90-0x0043c134): `ccEnemyBreath`,
//! its 64 flames (`ccEnemyBrPart`) and the two turns it reads off a matrix and
//! a movement (`ccSetMat2Rot`, `mov2rot`). `ccEnemyH` types 3 and 4 have two
//! breaths, `ccEnemyL` types 2 and 4 one. The race's `exclusive()` runs each
//! breath every frame the enemy is displayed ([`Breath::ctrl`]) and in its
//! breath attacks asks for a flame ([`Breath::set`]). The ground, the walls and
//! the player's frame are [`BreathWorld`]'s; the draws and the smoke are
//! [`BreathOut`]s. The flames' rules are in docs/engine/battle.md.

use piney_data::libm;

use crate::enemy_ai::rand_f;
use crate::geom::*;
use crate::param::le32;
use crate::rand::Rng;

/// A breath's flames (`initBreath` makes 64).
pub const PARTS: usize = 64;

/// 0.05: the least transparency a breath starts a flame at.
const K_0_05: F = 0x3d4c_cccd;
/// 0.36: a flying flame's fade in.
const K_0_36: F = 0x3eb8_51ec;
const K_M0_5: F = 0xbf00_0000;
const K_0_02: F = 0x3ca3_d70a;
const K_0_01: F = 0x3c23_d70a;
const K_15: F = 0x4170_0000;
const K_50: F = 0x4248_0000;
const K_0_5: F = 0x3f00_0000;
/// pi/4 (0x3f490fdb).
const K_PI_4: F = 0x3f49_0fdb;

/// `ccEnemyBrParam` (0x40 bytes): a breath attack's flames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BrParam {
    /// +0 each flame's life in frames; +2 and +4 the attack's frames
    /// (`actCnt`) it breathes in; +6 their mask (a flame when `actCnt &
    /// mask` is 0).
    pub life: u16,
    pub start: u16,
    pub end: u16,
    pub mask: u16,
    /// +0x10 the speed a flame starts at (along the node's turn).
    pub vel: [F; 3],
    /// +0x1c the size it starts at.
    pub scale: F,
    /// +0x20 the speed's growth a frame while flying (times the speed).
    pub accel: [F; 3],
    /// +0x2c the size's growth a frame while flying.
    pub grow: F,
    /// +0x30 the speed's growth a frame once fallen (times the count).
    pub grav: [F; 3],
    /// +0x3c the size's growth a frame once fallen.
    pub grow2: F,
}

impl BrParam {
    /// From its 0x40 bytes.
    pub fn read(b: &[u8]) -> BrParam {
        let w = |o: usize| le32(b, o);
        let h = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        BrParam {
            life: h(0),
            start: h(2),
            end: h(4),
            mask: h(6),
            vel: [w(0x10), w(0x14), w(0x18)],
            scale: w(0x1c),
            accel: [w(0x20), w(0x24), w(0x28)],
            grow: w(0x2c),
            grav: [w(0x30), w(0x34), w(0x38)],
            grow2: w(0x3c),
        }
    }
}

/// `ccEnemyBrInfo` (0x1c bytes): a breath's node and look.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BrInfo {
    /// +0 0: the node's turn is taken turned by (pi/2, pi/2, 0) (the
    /// wyrms' and dragons' heads); 1 as it is (the dogs' necks).
    pub kind: i32,
    /// +4 `effSmoke`'s kind for a burst.
    pub smoke: i32,
    /// +0x10 the clump's node the flames come from, +0x14 the flames'
    /// effect chunk, +0x18 their palette chunk ("" for none).
    pub node: String,
    pub eff: String,
    pub clut: String,
}

/// `ccEnemyBrPart` (0x48 bytes) with its `ccEff`'s fields that move.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BrPart {
    /// +0 bit 0 on the ground, bit 1 against a wall.
    pub landed: bool,
    pub wall: bool,
    /// +2 frames left, +6 the state (1 flying, 2 fallen), +8 frames in
    /// the state, +0xa the effect's pattern.
    pub life: u16,
    pub state: u16,
    pub count: u16,
    pub pattern: u16,
    /// +0xc the alpha it fades by.
    pub alpha: F,
    /// +0x10 its turn, +0x20 its speed (along the turn).
    pub rot: V4,
    pub vel: V4,
    /// +0x30 in use.
    pub active: bool,
    /// +0x34 its share of the attack (0 the first flame).
    pub phase: F,
    /// +0x40.
    pub param: BrParam,
    /// The `ccEff` (+0x38): +0x10 place, +0x20 and +0x24 size, +0x28 turn,
    /// +0x34 alpha.
    pub pos: V4,
    pub scale: [F; 2],
    pub rotate: F,
    pub eff_alpha: F,
}

/// `ccEnemyBreath` (0x10 bytes): +0 the flames alive, +8 the info, +0xc
/// the flames (a list in the order `initBreath` made them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Breath {
    pub count: i32,
    pub info: BrInfo,
    pub parts: Vec<BrPart>,
}

/// What the flames ask of the world.
pub trait BreathWorld {
    /// `ccHitCheckLM2(a, b, mask)`: where along a to b a wall is, -1.0 for
    /// none.
    fn line(&mut self, a: V4, b: V4, mask: u32) -> F;
    /// `ccLandHitCheck(p, mask)`: the ground's height under `p`.
    fn land(&mut self, p: V4, mask: u32) -> F;
    /// `ccTransPosW2P`, `ccTransPosP2W`: into the player's frame and back.
    fn w2p(&mut self, p: V4) -> V4;
    fn p2w(&mut self, p: V4) -> V4;
}

/// What a breath's frame puts out, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreathOut {
    /// `SetActiveLayer(3)`, `ccEff::Draw(pattern)`, `SetActiveLayer(0)`
    /// for flame `part`.
    Draw { part: usize, pos: V4, scale: [F; 2], rotate: F, alpha: F, pattern: u16 },
    /// `effSmoke(pos, vel, size, n, kind, fade_in, fade_out)`: a burst.
    Smoke { pos: V4, vel: V4, size: F, n: i32, kind: i32, fade_in: i32, fade_out: i32 },
}

/// An angle wrapped into -pi..pi once each way.
fn wrap(mut v: F) -> F {
    if !le(v, PI) {
        v = sub(v, TWO_PI);
    }
    if lt(v, NEG_PI) {
        v = add(v, TWO_PI);
    }
    v
}

/// `cos(pi/2 phase - pi/4)`: a flame's share of the spread.
fn spread(phase: F) -> F {
    libm::cosf(wrap(sub(mul(HALF_PI, phase), K_PI_4)))
}

/// `ccSetMat2Rot(m, out)` (gcmn 0x0043ae90): the turn (x, y, z; w 0) that
/// `sceVu0RotMatrix` would build `m`'s rotation from, read off by undoing
/// it about x, then y, then z.
pub fn mat2rot(m: &M4) -> V4 {
    let v = apply_matrix(m, [0, ONE, ONE, 0]);
    let a = libm::atan2f(v[2], v[1]);
    let m = rot_matrix_x(m, neg(a));
    let v = apply_matrix(&m, [ONE, 0, 0, 0]);
    let t = libm::atan2f(v[2], v[0]);
    let b = neg(t);
    let m = rot_matrix_y(&m, t);
    let v = apply_matrix(&m, [ONE, 0, 0, 0]);
    let c = libm::atan2f(v[1], v[0]);
    [a, b, c, 0]
}

/// `mov2rot(mov, out, _)` (gcmn 0x0043b000): the turn of a flame moving
/// along `mov`: turned a quarter about z, its heading about y from x and z,
/// then about z from x and y (x the first, less pi/2; y -0; z the second,
/// less pi/2; each wrapped once).
pub fn mov2rot(mov: V4) -> V4 {
    let m = rot_matrix(&unit_matrix(), [0, 0, HALF_PI, ONE]);
    let d = apply_matrix(&m, [mov[0], mov[1], mov[2], 0]);
    let a = wrap(sub(libm::atan2f(d[0], d[2]), HALF_PI));
    let m = rot_matrix(&unit_matrix(), [0, neg(a), 0, 0]);
    let d = apply_matrix(&m, [d[0], d[1], d[2], 0]);
    let b = libm::atan2f(d[1], d[0]);
    [a, neg(0), wrap(sub(b, HALF_PI)), 0]
}

fn neg(v: F) -> F {
    v ^ 0x8000_0000
}

impl Breath {
    /// `ccEnemyBreath::initBreath(info)` (gcmn 0x0043bdb0): 64 flames, none
    /// alive (their effects `ccEff::Init(chunk, 0)` with the palette).
    pub fn new(info: BrInfo) -> Breath {
        Breath { count: 0, info, parts: vec![BrPart::default(); PARTS] }
    }

    /// `ccEnemyBreath::setBreath(param, obj)` (gcmn 0x0043bfe0) for an
    /// enemy displayed (`disp`) at `transparency` in frame `act_cnt` of its
    /// attack, the breath's node at `node` (the node's world matrix, which
    /// a kind 0 breath changes).
    pub fn set(&mut self, param: &BrParam, disp: bool, transparency: F, act_cnt: i16, node: &mut M4) {
        if !disp || lt(transparency, K_0_05) {
            return;
        }
        let a = i32::from(act_cnt);
        if a < i32::from(param.start) || i32::from(param.end) < a || a & i32::from(param.mask) != 0 {
            return;
        }
        let Some(k) = self.parts.iter().position(|p| !p.active) else { return };
        self.init(k, node, param);
        let lnum = (i32::from(param.end) - i32::from(param.start)) / (i32::from(param.mask) + 1);
        let lrate = div(from_int(self.count), from_int(lnum));
        self.parts[k].phase = lrate;
        self.count += 1;
    }

    /// `ccEnemyBrPart::init(m, param)` (gcmn 0x0043b2e0).
    fn init(&mut self, k: usize, node: &mut M4, param: &BrParam) {
        let kind = self.info.kind;
        let p = &mut self.parts[k];
        p.active = true;
        p.state = 1;
        p.count = 0;
        p.landed = false;
        p.wall = false;
        p.pattern = 0;
        p.param = *param;
        p.life = param.life;
        p.vel = [param.vel[0], param.vel[1], param.vel[2], 0];
        p.scale = [param.scale, param.scale];
        p.alpha = 0;
        p.eff_alpha = 0;
        p.rotate = 0;
        p.pos = node[3];
        if kind == 0 {
            let r = rot_matrix(&unit_matrix(), [HALF_PI, HALF_PI, 0, ONE]);
            *node = mul_matrix(node, &r);
        }
        node[3][0] = 0;
        node[3][1] = 0;
        node[3][2] = 0;
        p.rot = mat2rot(node);
    }

    /// `ccEnemyBreath::ctrlBreath()` (gcmn 0x0043bee0): each live flame
    /// moves, changes and is drawn; a flame out of life is freed.
    pub fn ctrl(&mut self, w: &mut dyn BreathWorld, cc: &mut dyn Rng, out: &mut Vec<BreathOut>) {
        for k in 0..PARTS {
            if !self.parts[k].active {
                continue;
            }
            self.move_part(k, w, cc, out);
            self.anim(k);
            let p = &mut self.parts[k];
            out.push(BreathOut::Draw {
                part: k,
                pos: p.pos,
                scale: p.scale,
                rotate: p.rotate,
                alpha: p.eff_alpha,
                pattern: p.pattern,
            });
            p.life = p.life.wrapping_sub(1);
            if p.life == 0 {
                p.active = false;
                self.count -= 1;
            }
        }
    }

    /// `ccEnemyBrPart::move()` (gcmn 0x0043b620).
    fn move_part(&mut self, k: usize, w: &mut dyn BreathWorld, cc: &mut dyn Rng, out: &mut Vec<BreathOut>) {
        let p = &mut self.parts[k];
        match p.state {
            1 => {
                if p.count == 0 {
                    p.vel[0] = mul(p.vel[0], spread(p.phase));
                }
                for i in 0..3 {
                    p.vel[i] = add(p.vel[i], mul(p.vel[i], p.param.accel[i]));
                }
            }
            2 => {
                let c = from_int(i32::from(p.count));
                for i in 0..3 {
                    p.vel[i] = add(p.vel[i], mul(p.param.grav[i], c));
                }
            }
            _ => {}
        }
        if lt(p.vel[0], 0) {
            p.vel[0] = 0;
        }
        let m = rot_matrix(&unit_matrix(), p.rot);
        let d = apply_matrix(&m, p.vel);
        let mut next = vadd(p.pos, d);
        p.count = p.count.wrapping_add(1);
        if !p.wall {
            let r = w.line(p.pos, next, 0x4000_0000);
            if eq(r, MINUS_ONE) {
                p.wall = false;
            } else {
                p.wall = true;
                if p.state == 1 {
                    p.state = 2;
                    p.count = 0;
                }
                p.vel[0] = 0;
            }
        }
        let mut z = w.land(next, 0x2000_0002);
        let nz = next[2];
        if !eq(z, nz) {
            z = add(z, mul(K_15, p.scale[1]));
        }
        if lt(nz, z) {
            p.landed = true;
            if p.state == 1 {
                p.state = 2;
                p.count = 0;
            }
        }
        if p.landed {
            next[2] = z;
        }
        if (p.landed || p.wall) && p.count < 7 && p.count & 1 != 0 {
            self.explode(k, cc, out);
        }
        let p = &mut self.parts[k];
        p.rot = mov2rot(vsub(next, p.pos));
        p.pos = next;
        let q = w.w2p(p.pos);
        p.pos = w.p2w(q);
    }

    /// `ccEnemyBrPart::anim()` (gcmn 0x0043b480).
    fn anim(&mut self, k: usize) {
        let p = &mut self.parts[k];
        match p.state {
            1 => {
                p.pattern = p.pattern.wrapping_add(4);
                if p.pattern >= 16 {
                    p.pattern = 0;
                }
                p.scale[0] = add(p.scale[0], p.param.grow);
                p.scale[1] = add(p.scale[1], p.param.grow);
                set_dist(&mut p.alpha, ONE, K_0_36);
                p.eff_alpha = p.alpha;
            }
            2 => {
                p.pattern = p.pattern.wrapping_add(1);
                if p.pattern >= 32 {
                    p.pattern = 31;
                }
                p.scale[0] = add(p.scale[0], p.param.grow2);
                p.scale[1] = add(p.scale[1], p.param.grow2);
                set_dist(&mut p.alpha, K_M0_5, K_0_02);
                if lt(p.alpha, K_0_01) {
                    p.alpha = 0;
                }
                p.eff_alpha = p.alpha;
            }
            _ => {}
        }
    }

    /// `ccEnemyBrPart::explode()` (gcmn 0x0043bb00): a burst of smoke.
    fn explode(&mut self, k: usize, cc: &mut dyn Rng, out: &mut Vec<BreathOut>) {
        let smoke = self.info.smoke;
        let p = &self.parts[k];
        let mut pos = p.pos;
        for c in pos.iter_mut().take(3) {
            *c = add(*c, rand_f(cc, K_50));
        }
        let m = rot_matrix(&unit_matrix(), p.rot);
        let mut v = apply_matrix(&m, p.vel);
        for c in v.iter_mut().take(3) {
            *c = mul(*c, K_0_5);
        }
        let size = mul(p.scale[0], spread(p.phase));
        out.push(BreathOut::Smoke { pos, vel: v, size, n: 8, kind: smoke, fade_in: 512, fade_out: 32 });
    }
}
