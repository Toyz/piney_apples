//! `ccPrimRadiate` (gcmn prim.cpp, 0x00438ac0-0x0043a248): rays of light
//! spread round a point, built each frame and drawn as triangle strips on
//! layer 6, with a `ccOmniLight` in the scene's light group while it shines.
//! A treasure box's and an idol's opening (`ccGimBoxRad`, [`box_ctrl`]) use
//! one, the event's radiator (`ccGimRadiator`, [`radiator_ctrl`]) and the
//! enemies' weapon flashes ([`crate::weapon::rad_ctrl`]) too.
//! Only the plate (type bit 4) is here. The functions are in
//! docs/engine/battle.md ("The weapon trails and flashes"); the world the rays
//! are built and drawn in is a [`RadWorld`].

use crate::entry::Cx;
use crate::geom::{
    self, F, HALF_PI, M4, ONE, PI, TWO_PI, V4, add, apply_matrix, cosf, div, from_int, le, lt, mul, neg, pi_limit,
    rand_f, rot_matrix_x, rot_matrix_y, rot_matrix_z, sinf, sub, trans_matrix, unit_matrix,
};
use crate::rand::Rng;

/// What a radiate asks of the world as it builds and draws its rays.
pub trait RadWorld {
    /// `ccTransPosFW2LW(p, p)` (main 0x0059b9c0): `P2W(W2P(p))`, the point
    /// into Kite's frame.
    fn fw2lw(&mut self, p: V4) -> V4;
    /// `ccRand()`, which `ccRandF` draws a ray's length and bank from.
    fn rand(&mut self) -> i32;
    /// The camera's rotation a plate facing the camera (type bit 1) turns
    /// by: `cameraGetRot2(v, camID)` in the eye view (`checkCameraType()`
    /// 1), else `cameraGetRot(v, camID)`.
    fn camera_rot(&mut self) -> V4;
}

/// The entry control's world: the boxes' and idols' rays, none of which
/// faces the camera.
impl RadWorld for Cx<'_> {
    fn fw2lw(&mut self, p: V4) -> V4 {
        let q = self.world.w2p(p);
        self.world.p2w(q)
    }

    fn rand(&mut self) -> i32 {
        self.cc.rand()
    }

    fn camera_rot(&mut self) -> V4 {
        [0; 4]
    }
}

/// A [`RadWorld`]'s `ccRand()` as an [`Rng`].
struct Draws<'a>(&'a mut dyn RadWorld);

impl Rng for Draws<'_> {
    fn rand(&mut self) -> i32 {
        self.0.rand()
    }
}

/// 1/255.
const K_1_255: F = 0x3b80_8081;
/// 255.0.
const K_255: F = 0x437f_0000;
/// 6.0.
const K_6: F = 0x40c0_0000;
/// 1.4.
const K_1_4: F = 0x3fb3_3333;
const K_2: F = 0x4000_0000;
const K_1000: F = 0x447a_0000;
const K_1E6: F = 0x4974_2400;

/// `ccPrimRadInfo` (0x1c bytes): what a radiate is made from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RadInfo {
    pub ty: i16,
    pub pnum: i16,
    pub center: F,
    pub length: F,
    pub width: F,
    pub fzoom: F,
    pub col0: u32,
    pub col1: u32,
}

/// `boxRadInfo` (gcmn 0x005d5990): a plate of 16 rays, all else 0.
pub const BOX_RAD_INFO: RadInfo =
    RadInfo { ty: 4, pnum: 16, center: 0, length: 0, width: 0, fzoom: 0, col0: 0, col1: 0 };

/// `gimRadInfo` (gcmn 0x005eaf40): the radiator's plate of 32 rays.
pub const GIM_RAD_INFO: RadInfo =
    RadInfo { ty: 4, pnum: 32, center: 0, length: 0, width: 0, fzoom: 0, col0: 0, col1: 0 };

/// One point of a ray (`ccPrimPart` +0x10 + 0x30 i): its colour (+4, RGBA
/// with the alpha in the top byte) and position (+0x10).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrimVert {
    pub color: u32,
    pub pos: V4,
}

/// `ccPrimPart` (0xd0 bytes): one ray, inner points 0 and 2, outer 1 and 3.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrimPart {
    /// +0x08 `col0` (the inner points), +0x0c `col1` (the outer).
    pub col0: u32,
    pub col1: u32,
    pub vert: [PrimVert; 4],
}

impl PrimPart {
    /// `ccPrimPart::setAlphaPart(rate)` (gcmn 0x004389c0): each point's
    /// alpha byte its colour's (`col1` for the odd points) times `rate`,
    /// truncated and held to 0-255.
    pub fn set_alpha(&mut self, rate: F) {
        for (i, v) in self.vert.iter_mut().enumerate() {
            let c = if i & 1 != 0 { self.col1 } else { self.col0 };
            let a = crate::damage::fptosi(mul(from_int((c >> 24) as i32), rate)).clamp(0, 255) as u32;
            v.color = (v.color & 0x00ff_ffff) | (a << 24);
        }
    }
}

/// The radiate's `ccOmniLight` as it writes it: +0x40 `matrix` (its
/// position), +0x8d `matCalcSW`, the RGB its +0xb0 `color` is set from
/// (`ccSetColor(color, rgb, 1.0)`, main 0x00138b60), +0xc0 `intensity`,
/// +0xc4 `farDownStart`, +0xc8 `farDownEnd`, +0xcc `farDownEnd2`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OmniLight {
    pub matrix: M4,
    pub mat_calc: bool,
    pub rgb: u32,
    pub intensity: F,
    pub far_start: F,
    pub far_end: F,
    pub far_end2: F,
}

/// `ccPrimRadiate` (0xa0 bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Radiate {
    /// +0x00 bit 0 `radFlag` (shining: [`Radiate::main`] runs), bit 1
    /// `lgtFlag` (the light is in the scene's group).
    pub rad_flag: bool,
    pub lgt_flag: bool,
    /// +0x02 `type`, +0x04 `pnum`.
    pub ty: i16,
    pub pnum: i16,
    /// +0x08 `center` ... +0x30 `dpBank`.
    pub center: F,
    pub scale: F,
    pub lscale: F,
    pub angle: F,
    pub bank: F,
    pub alpha: F,
    pub length: F,
    pub width: F,
    pub fzoom: F,
    pub dp_length: F,
    pub dp_bank: F,
    /// +0x34 `user`: the character the rays belong to.
    pub user: Option<usize>,
    /// +0x38 `info`.
    pub info: RadInfo,
    /// +0x3c `part`.
    pub parts: Vec<PrimPart>,
    /// +0x44 `ccPrimPacket`'s double-buffer index, flipped by each `disp`.
    pub packet: i32,
    /// +0x58 `lgt`.
    pub light: OmniLight,
    /// +0x60 `pos`, +0x70 `rot`.
    pub pos: V4,
    pub rot: V4,
    /// +0x80 `attr`, +0x82 `life`, +0x84 `actNum`, +0x86 `actCnt`, +0x88
    /// `param[4]`.
    pub attr: i16,
    pub life: i16,
    pub act_num: i16,
    pub act_cnt: i16,
    pub param: [F; 4],
}

/// What a frame of a radiate asks of the rest of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RadOut {
    /// `ccLightGrp::AddGrp(lgt)` into `cc3d`'s light group (+0x80), with the
    /// light as it now stands (a light already in the group stays).
    LightOn(OmniLight),
    /// `ccLightGrp::DelGrp(lgt)`.
    LightOff,
    /// `disp`: the rays as triangle strips on layer 6, each point in
    /// Kite's frame with its colour.
    Draw(Vec<[PrimVert; 4]>),
}

impl Radiate {
    /// `new ccPrimRadiate(inf, chr)` (gcmn 0x00438ac0).
    pub fn new(info: RadInfo, user: Option<usize>) -> Radiate {
        let mut r = Radiate {
            rad_flag: false,
            lgt_flag: false,
            ty: 0,
            pnum: 0,
            center: 0,
            scale: 0,
            lscale: 0,
            angle: 0,
            bank: 0,
            alpha: 0,
            length: 0,
            width: 0,
            fzoom: 0,
            dp_length: 0,
            dp_bank: 0,
            user,
            info,
            parts: Vec::new(),
            packet: 0,
            light: OmniLight::default(),
            pos: [0; 4],
            rot: [0; 4],
            attr: 0,
            life: 0,
            act_num: 0,
            act_cnt: 0,
            param: [0; 4],
        };
        r.parts = vec![PrimPart::default(); usize::try_from(info.pnum).unwrap_or(0)];
        r.init();
        r
    }

    /// `ccPrimRadiate::init()` (gcmn 0x00438d10).
    pub fn init(&mut self) {
        self.pos = geom::VF0;
        self.rot = [0; 4];
        self.ty = self.info.ty;
        self.pnum = self.info.pnum;
        self.center = self.info.center;
        self.angle = div(TWO_PI, from_int(i32::from(self.pnum)));
        self.bank = 0;
        self.scale = ONE;
        self.lscale = ONE;
        self.alpha = ONE;
        self.width = self.info.width;
        self.length = self.info.length;
        self.fzoom = self.info.fzoom;
        self.dp_length = 0;
        self.dp_bank = 0;
        self.set_color(self.info.col0, self.info.col1);
        self.rad_flag = false;
        self.lgt_flag = false;
        self.act_cnt = 0;
        self.act_num = 0;
        self.life = 0;
        self.param = [0; 4];
    }

    /// `ccPrimRadiate::setColorRadiate(col0, col1)` (gcmn 0x00438e30) for a
    /// radiate without a centre part: each ray's inner points `col0`,
    /// outer `col1`; the light's reach and colour (`ccSetColor(col0 &
    /// 0xffffff, 1.0)`).
    pub fn set_color(&mut self, col0: u32, col1: u32) {
        for p in &mut self.parts {
            p.col0 = col0;
            p.col1 = col1;
            for (i, v) in p.vert.iter_mut().enumerate() {
                v.color = if i & 1 != 0 { col1 } else { col0 };
            }
        }
        let l = &mut self.light;
        l.far_start = 0;
        l.far_end = K_1000;
        l.far_end2 = K_1E6;
        l.rgb = col0 & 0x00ff_ffff;
    }

    /// `ccPrimRadiate::setLight(pos)` (gcmn 0x00439290): `pos` into Kite's
    /// frame in place, the light there (`TransMatrix` of the unit,
    /// `matCalcSW`) and into `cc3d`'s group, `lgtFlag` up.
    pub fn set_light(&mut self, w: &mut dyn RadWorld, out: &mut Vec<RadOut>) {
        self.pos = w.fw2lw(self.pos);
        self.light.matrix = trans_matrix(&unit_matrix(), self.pos);
        self.light.mat_calc = true;
        out.push(RadOut::LightOn(self.light));
        self.lgt_flag = true;
    }

    /// `ccPrimRadiate::delLight()` (gcmn 0x00439330): with `lgtFlag`, the
    /// light out of the group and the flag down.
    pub fn del_light(&mut self, out: &mut Vec<RadOut>) {
        if self.lgt_flag {
            out.push(RadOut::LightOff);
            self.lgt_flag = false;
        }
    }

    /// `ccPrimRadiate::create()` (gcmn 0x004393a0) of a plate: `rot`, for
    /// one facing the camera (type bit 1) with y `piLimit(-cam.x)`, z
    /// `piLimit(pi/2 + cam.z)` and w 0 (`rot` itself kept); `m = RotZ(z)
    /// RotY(y) RotX(x)` of the unit matrix, then `createPlate(m)`.
    pub fn create(&mut self, w: &mut dyn RadWorld) {
        let mut rot = self.rot;
        if self.ty & 1 != 0 {
            let cam = w.camera_rot();
            rot[1] = pi_limit(neg(cam[0]));
            rot[2] = pi_limit(add(HALF_PI, cam[2]));
            rot[3] = 0;
        }
        let m = rot_matrix_x(&unit_matrix(), rot[0]);
        let m = rot_matrix_y(&m, rot[1]);
        let m = rot_matrix_z(&m, rot[2]);
        if self.parts.is_empty() {
            return;
        }
        if self.ty & 4 != 0 {
            self.create_plate(w, &m);
        }
    }

    /// `ccPrimRadiate::createPlate(mat0)` (gcmn 0x00439520).
    fn create_plate(&mut self, w: &mut dyn RadWorld, mat0: &M4) {
        let hw = mul(div(self.width, K_2), self.scale);
        let mut len = mul(self.scale, mul(self.length, self.lscale));
        let c = mul(self.center, self.scale);
        let z = self.fzoom;
        let bank = rot_matrix_y(&unit_matrix(), self.bank);
        let facing = self.ty & 1 != 0;
        for i in 0..usize::try_from(self.pnum).unwrap_or(0) {
            if !geom::eq(self.dp_length, 0) {
                let r = rand_f(&mut Draws(&mut *w), mul(self.length, self.dp_length));
                len = mul(self.scale, mul(self.lscale, add(self.length, r)));
            }
            let mut turn = bank;
            if !geom::eq(self.dp_bank, 0) {
                let r = rand_f(&mut Draws(&mut *w), mul(self.bank, self.dp_bank));
                turn = rot_matrix_y(&turn, r);
            }
            let a = pi_limit(mul(self.angle, from_int(i as i32)));
            let rx = rot_matrix_x(&unit_matrix(), a);
            let mut m: M4 = std::array::from_fn(|r| apply_matrix(mat0, rx[r]));
            m[3] = self.pos;
            let outer = |y: F| {
                let v = [0, y, add(c, len), ONE];
                let v = if facing { v } else { apply_matrix(&turn, v) };
                apply_matrix(&m, v)
            };
            let neg_hw = geom::neg(hw);
            let p = &mut self.parts[i];
            p.vert[0].pos = apply_matrix(&m, [0, neg_hw, c, ONE]);
            p.vert[1].pos = outer(mul(neg_hw, z));
            p.vert[2].pos = apply_matrix(&m, [0, hw, c, ONE]);
            p.vert[3].pos = outer(mul(hw, z));
            p.set_alpha(self.alpha);
        }
    }

    /// `ccPrimRadiate::disp(part)` (gcmn 0x00439df0): the packet's buffer
    /// flipped, each point into Kite's frame in place (`ccTransPosFW2LW`),
    /// the rays out to be drawn on layer 6.
    pub fn disp(&mut self, w: &mut dyn RadWorld, out: &mut Vec<RadOut>) {
        self.packet = (self.packet + 1) & 1;
        let mut rays = Vec::with_capacity(self.parts.len());
        for p in &mut self.parts {
            for v in &mut p.vert {
                v.pos = w.fw2lw(v.pos);
            }
            rays.push(p.vert);
        }
        out.push(RadOut::Draw(rays));
    }
}

/// `ccGimBoxRad::ctrl()` (gcmn 0x0043c1b0), a box's or an idol's opening
/// light: act 0 sets the colours, sizes and height it shines from, act 1 for
/// 52 frames turns it about x, pulses it with `param[0]` and sways it with
/// `param[1]`, its light in the scene's group; act 2 puts it out. The
/// constants are in docs/engine/battle.md ("The rays").
pub fn box_ctrl(r: &mut Radiate, cx: &mut Cx, out: &mut Vec<RadOut>) {
    const ROT_STEP: F = 0x3e38_51ec;
    const K_0_4: F = 0x3ecc_cccd;
    const K_0_1: F = 0x3dcc_cccd;
    const K_0_2: F = 0x3e4c_cccd;
    const K_0_5: F = 0x3f00_0000;
    const K_3: F = 0x4040_0000;
    const K_10: F = 0x4120_0000;
    const K_30: F = 0x41f0_0000;
    const K_60: F = 0x4270_0000;
    const K_80: F = 0x42a0_0000;
    const K_4: F = 0x4080_0000;
    const STEP0: F = 0x3d80_adfd;
    const STEP1: F = 0x3d00_adfd;
    match r.act_num {
        0 => {
            if let Some(u) = r.user {
                r.pos = cx.scene.chars[u].pos;
            }
            r.rot[1] = 0xbfc9_0fdb;
            let c0 = fractional_hsv(0x802a_ffff, ONE, K_0_4);
            let c1 = fractional_hsv(0x802a_ffff, 0, 0);
            r.set_color(c0, c1);
            r.center = 0;
            r.bank = 0;
            r.length = K_80;
            r.width = K_60;
            r.fzoom = K_4;
            r.dp_length = K_0_1;
            r.dp_bank = K_0_1;
            r.act_num += 1;
            r.act_cnt = 0;
            r.param[0] = 0;
            r.param[1] = 0;
            r.param[2] = r.pos[2];
            shine(r, cx, out);
        }
        1 => shine(r, cx, out),
        2 => {
            r.rad_flag = false;
            r.del_light(out);
        }
        _ => {}
    }

    fn shine(r: &mut Radiate, cx: &mut Cx, out: &mut Vec<RadOut>) {
        let mut x = add(r.rot[0], ROT_STEP);
        if !le(x, PI) {
            x = sub(x, TWO_PI);
        }
        r.rot[0] = x;
        if lt(r.rot[0], geom::NEG_PI) {
            r.rot[0] = add(r.rot[0], TWO_PI);
        }
        r.alpha = mul(K_0_5, sinf(r.param[0]));
        r.light.intensity = mul(K_3, sinf(r.param[0]));
        r.set_light(cx, out);
        r.bank = mul(geom::HALF_PI, sinf(r.param[1]));
        r.pos[2] = add(sub(r.param[2], K_10), mul(K_60, cosf(r.param[1])));
        r.length = add(K_30, mul(K_60, sinf(r.param[0])));
        r.width = add(K_30, mul(K_30, sinf(r.param[0])));
        r.fzoom = add(ONE, mul(K_2, sinf(r.param[0])));
        r.dp_length = mul(K_0_2, sinf(r.param[0]));
        r.dp_bank = mul(K_0_1, sinf(r.param[0]));
        r.param[0] = add(r.param[0], STEP0);
        r.param[1] = add(r.param[1], STEP1);
        let c = r.act_cnt;
        r.act_cnt = c.wrapping_add(1);
        if c >= 51 {
            r.act_num += 1;
            r.act_cnt = 0;
        }
    }
}

/// `ccGimRadiator::ctrl()` (gcmn 0x00456220), the event's radiator (a
/// bracelet's shine): act 0 starts its two phases and keeps its height;
/// act 1 each frame turns it 0.18 about x, pulses its alpha and light with
/// `sin param[0]`, lights its place, draws its rays 5 + 10 sin long from
/// 4 out, 5 wide, banked 0.5, cyan fading outward; act 2 puts it out.
pub fn radiator_ctrl(r: &mut Radiate, cx: &mut Cx, out: &mut Vec<RadOut>) {
    const ROT_STEP: F = 0x3e38_51ec;
    const K_0_4: F = 0x3ecc_cccd;
    const K_0_5: F = 0x3f00_0000;
    const K_4: F = 0x4080_0000;
    const K_5: F = 0x40a0_0000;
    const K_10: F = 0x4120_0000;
    const STEP0: F = 0x3d80_adfd;
    const STEP1: F = 0x3d00_adfd;
    const HSV: u32 = 0x4080_ffff;
    match r.act_num {
        0 | 1 => {
            if r.act_num == 0 {
                r.act_num = 1;
                r.act_cnt = 0;
                r.param[0] = 0;
                r.param[1] = 0;
                r.param[2] = r.pos[2];
            }
            r.rot[0] = pi_limit(add(r.rot[0], ROT_STEP));
            r.alpha = sinf(r.param[0]);
            r.light.intensity = sinf(r.param[0]);
            r.set_light(cx, out);
            r.bank = K_0_5;
            r.length = add(K_5, mul(K_10, sinf(r.param[0])));
            r.width = K_5;
            r.center = K_4;
            r.fzoom = 0;
            r.dp_length = mul(K_0_4, sinf(r.param[0]));
            r.dp_bank = mul(K_0_4, sinf(r.param[0]));
            r.param[0] = pi_limit(add(r.param[0], STEP0));
            r.param[1] = pi_limit(add(r.param[1], STEP1));
            let c0 = fractional_hsv(HSV, K_0_5, ONE);
            let c1 = fractional_hsv(HSV, 0, 0);
            r.set_color(c0, c1);
        }
        2 => {
            r.rad_flag = false;
            r.del_light(out);
        }
        _ => {}
    }
}

/// `ccPrimRadiate::main()` (gcmn 0x0043a1c0): with `radFlag`, the class's
/// `ctrl` (vtable +8), `create`, `disp`.
pub fn main(r: &mut Radiate, cx: &mut Cx, out: &mut Vec<RadOut>, ctrl: fn(&mut Radiate, &mut Cx, &mut Vec<RadOut>)) {
    if !r.rad_flag {
        return;
    }
    ctrl(r, cx, out);
    r.create(cx);
    if !r.parts.is_empty() {
        r.disp(cx, out);
    }
}

/// [`main`] of a box's or idol's rays ([`box_ctrl`]).
pub fn box_main(r: &mut Radiate, cx: &mut Cx, out: &mut Vec<RadOut>) {
    main(r, cx, out, box_ctrl);
}

/// libm `modff(x)` on bits: (fraction, integral part), both with `x`'s
/// sign.
pub fn modff(x: F) -> (F, F) {
    let j0 = ((x >> 23) & 0xff) as i32 - 127;
    let sign = x & 0x8000_0000;
    if j0 < 23 {
        if j0 < 0 {
            return (x, sign);
        }
        let i = 0x007f_ffffu32 >> j0;
        if x & i == 0 {
            return (sign, x);
        }
        let ip = x & !i;
        (sub(x, ip), ip)
    } else {
        (sign, x)
    }
}

/// `hsv2rgb(col)` (gcmn 0x00437bd0): the low three bytes (v, s, h, each
/// /255) to R, G, B by the hue's sector (`h 6` split by `modff`); outside
/// sectors 0-5 the three lanes back as they were; times 255, truncated.
pub fn hsv2rgb(col: u32) -> u32 {
    let lane = |b: u32| mul(from_int(b as i32), K_1_255);
    let (v, s, h) = (lane(col & 0xff), lane((col >> 8) & 0xff), lane((col >> 16) & 0xff));
    let (f, ip) = modff(mul(h, K_6));
    let p = mul(v, sub(ONE, s));
    let q = mul(v, sub(ONE, mul(s, f)));
    let t = mul(v, sub(ONE, mul(s, sub(ONE, f))));
    let rgb = match crate::damage::fptosi(ip) as u32 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        5 => [v, p, q],
        _ => [v, s, h],
    };
    let o = rgb.map(|x| geom::to_int(mul(x, K_255)) as u32 & 0xff);
    (o[2] << 16) | (o[1] << 8) | o[0]
}

/// `ccFractionalHsv(c, per0, per1)` (gcmn 0x004380c0): the saturation
/// byte times `1.4 - per0` and the alpha (top byte) times `per1`, each
/// truncated and held to 0-255; the colour through [`hsv2rgb`].
pub fn fractional_hsv(c: u32, per0: F, per1: F) -> u32 {
    let (v, s, h, a) = (c & 0xff, (c >> 8) & 0xff, (c >> 16) & 0xff, c >> 24);
    let s2 = crate::damage::fptosi(mul(from_int(s as i32), sub(K_1_4, per0))).clamp(0, 255) as u32;
    let a2 = crate::damage::fptosi(mul(from_int(a as i32), per1)).clamp(0, 255) as u32;
    (a2 << 24) | hsv2rgb(v | (s2 << 8) | (h << 16))
}
