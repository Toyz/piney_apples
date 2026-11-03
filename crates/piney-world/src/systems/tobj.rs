//! `TOBJ` (gcmn 0x005d04c0-0x005d13c0): the server's traveller that now and
//! then crosses a field (`fieldrand(100) >= 86` in `Generate`): Δ's airship
//! (`ANM_sft1s1a` of `t1`), Θ's balloon (`sft2bas1a`, its cloth
//! scrolling), Λ's whale (`se3_5wh1a`), Σ's (`sft4s1a`, standing), Ω's
//! sleigh (`sft5sle0` and two runners, `ANM_sft5run0`). Its cloth scrolls
//! are `Move`'s statics, which outlive every field.

use std::borrow::Cow;
use std::rc::Rc;

use piney_data::Result;
use piney_desktop::assets::SceneFile;

use super::{BuildCx, Carried, CarriedMut, DrawCx, Hook, Kind, MapEvent, MapSystem, Others, Piece, Request, StepCx};
use crate::chara::pose::Play;
use crate::draw;
use crate::ee::{self, F, ONE, V4};
use crate::field::ambient::{Env, statics, uint_float};

/// `tobjCCS[server]` and `tobjAnm[server]` (gcmn 0x006576a0, 0x006576c0),
/// and Ω's runners' animation and dummies.
const TOBJ: [(&str, &str); 5] = [
    ("t1", "ANM_sft1s1a"),
    ("t2", "ANM_sft2bas1a"),
    ("se3_5wh1", "ANM_se3_5wh1a"),
    ("t4", "ANM_sft4s1a"),
    ("t5", "ANM_sft5sle0"),
];
const RUNNER: &str = "ANM_sft5run0";
const RUNNER_DUMMIES: [&str; 2] = ["OBJ_dummy_rei0", "OBJ_dummy_rei1"];
/// The balloon's cloth materials, whose V the scrolls set.
const CLOTHS: [&str; 2] = ["MAT_sft2clo1", "MAT_sft2clo2"];

/// 57.29578: radians from degrees.
const DEG: F = 0x4265_2ee0;
const K10: F = 0x4120_0000;
const K2400: F = 0x4516_0000;

/// One `TOBJ`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Traveller {
    /// +0: the server, 0-4.
    pub server: u32,
    /// +4 frames of this crossing, +8 beyond 19200 of the player (not
    /// drawn), +0xc frames asleep between crossings.
    pub timer: i32,
    pub far: bool,
    pub sleep: i32,
    /// +0x10 place, +0x20 turn, +0x30 heading, +0x34 speed (10), +0x38
    /// transparency.
    pub pos: V4,
    pub rot: V4,
    pub angle: F,
    pub speed: F,
    pub alpha: F,
}

/// TOBJ's function statics: `Move`'s cloth scrolls `u`, `v` (gcmn
/// 0x00378da0, 0x00378da8), and what the last `Move` wrote into the
/// balloon's cloths before stepping them: `MAT_sft2clo1`'s V offset
/// (+0x16) `ftoi12(u)`, `MAT_sft2clo2`'s `ftoi12(v)`, each less the
/// material's own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cloth {
    pub u: F,
    pub v: F,
    pub shown: [i32; 2],
}

impl Traveller {
    /// `TOBJ::Init(stream, tobjAnm[server], server)` (0x005d0560).
    pub fn new(server: u32, rng: &mut piney_data::dungeon::Rng) -> Traveller {
        let angle = ee::div(ee::from_int(rng.below(360) as i32 - 180), DEG);
        let timer = rng.below(600) as i32 + 630;
        let mut pos = [0, 0, 0, 0];
        if server != 3 {
            pos[0] = uint_float(rng.below(12000));
            pos[1] = uint_float(rng.below(12000));
        } else {
            pos[0] = ee::sub(uint_float(rng.below(5000)), 0x451c_4000);
            pos[1] = ee::sub(uint_float(rng.below(5000)), 0x451c_4000);
        }
        let mut t = Traveller {
            server,
            timer,
            far: false,
            sleep: 0,
            pos,
            rot: [0, 0, angle, ONE],
            angle,
            speed: K10,
            alpha: 0,
        };
        if server == 3 {
            t.rot = [0x3f86_0a92, 0, 0, ONE];
            t.timer = 1;
        }
        t
    }

    fn height_at(env: &Env, pos: V4, above: F) -> F {
        let h = env.height(pos[0], pos[1]);
        if ee::le(h, 0) { above } else { ee::add(above, h) }
    }

    /// `TOBJ::Move` (0x005d0db0). `cloth` holds the balloon's scrolls.
    pub fn step(&mut self, env: &Env, cloth: &mut Cloth, rng: &mut piney_data::dungeon::Rng) {
        if self.sleep != 0 {
            self.sleep -= 1;
            return;
        }
        let d;
        match self.server {
            0 | 2 | 4 => {
                let sx = ee::mul(self.speed, ee::sinf(self.angle));
                let sy = ee::mul(ee::neg(self.speed), ee::cosf(self.angle));
                self.pos = ee::vadd(self.pos, [sx, sy, 0, ONE]);
                d = env.w2p(self.pos);
                self.pos = env.p2w(d);
                self.far = false;
                let s = ee::add(ee::mul(d[0], d[0]), ee::mul(d[1], d[1]));
                if f64::from(ee::f(s)).sqrt() > 19200.0 {
                    self.far = true;
                }
                self.pos[2] = Traveller::height_at(env, self.pos, K2400);
            }
            1 => {
                self.far = false;
                let q = |x: F| ee::to_int(ee::mul(x, 0x4580_0000));
                cloth.shown = [q(cloth.u), q(cloth.v)];
                cloth.u = ee::add(cloth.u, 0x3b44_9ba6);
                if !ee::le(cloth.u, ONE) {
                    cloth.u = ee::sub(cloth.u, ONE);
                }
                cloth.v = ee::add(cloth.v, 0x3ac4_9ba6);
                if !ee::le(cloth.v, ONE) {
                    cloth.v = ee::sub(cloth.v, ONE);
                }
                d = env.w2p(self.pos);
                self.pos = env.p2w(d);
                self.pos[2] = Traveller::height_at(env, self.pos, 0x457a_0000);
            }
            3 => {
                self.far = false;
                d = env.w2p(self.pos);
                self.pos = env.p2w(d);
                self.pos[2] = Traveller::height_at(env, self.pos, 0x45bb_8000);
            }
            _ => d = [0; 4],
        }
        self.timer -= 1;
        if self.timer != 0 {
            return;
        }
        self.angle = ee::div(ee::from_int(rng.below(360) as i32 - 180), DEG);
        self.rot[2] = self.angle;
        self.timer = rng.below(500) as i32 + 230;
        self.alpha = 0;
        self.pos = env.player;
        self.pos[0] = ee::add(self.pos[0], ee::from_int(rng.below(15000) as i32 - 7500));
        self.pos[1] = ee::add(self.pos[1], ee::from_int(rng.below(15000) as i32 - 7500));
        self.pos[2] = Traveller::height_at(env, self.pos, K2400);
        if self.server == 1 {
            self.pos[2] = ee::add(self.pos[2], 0x44c8_0000);
            self.rot[2] = 0;
            let r = crate::town::pc::get_dirc([self.pos[0], self.pos[1], 0, 0], d);
            self.rot[0] = ee::div(r, 0x4080_0000);
        }
        if self.server == 3 {
            self.pos = env.player;
            self.pos[0] = ee::add(self.pos[0], ee::sub(uint_float(rng.below(4000)), 0x44fa_0000));
            self.pos[1] = ee::add(self.pos[1], ee::sub(uint_float(rng.below(4000)), 0x44fa_0000));
        }
        self.sleep = rng.below(100) as i32 + 30;
    }
}

/// One of the frame's draws: TOBJ's animation (0) or a runner's (1, 2), its
/// matrix, transparency and shadow (`ccShadowModel::Draw`'s length, three
/// times its height, and transparency, `(256 alpha + 1) / 2`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TobjDraw {
    pub play: usize,
    pub matrix: [V4; 4],
    pub alpha: F,
    pub shadow: (F, u8),
}

/// The traveller (none unless `Generate`'s roll made one), its file and
/// animations (none without the file), the cloth scrolls, and this
/// frame's draws.
#[derive(Clone)]
pub struct Tobj {
    pub tobj: Option<Traveller>,
    made: Option<(Rc<SceneFile>, Vec<Play>)>,
    pub cloth: Cloth,
    pub drawn: Vec<TobjDraw>,
}

impl Tobj {
    /// `Generate`'s roll on servers 0-4, `TOBJ::Init` (Δ's airship and
    /// Λ's whale start their hum, `tobjSeLoopStart`), and its file.
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<Tobj> {
        let server = cx.field.expect("a field's TOBJ").server;
        let mut tobj = None;
        if (0..=4).contains(&server) && cx.rng.below(100) >= 86 {
            tobj = Some(Traveller::new(server as u32, &mut cx.rng));
            if matches!(server, 0 | 2) {
                cx.made.push(MapEvent::SeLoopStart);
            }
        }
        let made = tobj.as_ref().and_then(|t| {
            let (stem, anim) = TOBJ[t.server as usize];
            let f = Rc::new(SceneFile::read(cx.archive, stem).ok()?);
            let mut plays: Vec<Play> = Play::new(&f, anim).into_iter().collect();
            if t.server == 4 {
                for _ in 0..2 {
                    plays.extend(Play::new(&f, RUNNER));
                }
            }
            Some((f, plays))
        });
        Ok(Tobj { tobj, made, cloth: Cloth::default(), drawn: Vec::new() })
    }

    /// Where its animation puts Ω's runners under the matrix `m`.
    pub fn runner_places(&self, m: &[V4; 4]) -> [V4; 2] {
        let mut out = [[0, 0, 0, ONE]; 2];
        let Some((file, plays)) = &self.made else { return out };
        let Some(play) = plays.first() else { return out };
        let worlds = play.worlds(file, draw::mat(m), &[]);
        for (k, name) in RUNNER_DUMMIES.into_iter().enumerate() {
            let Some(o) = file.ccs.find_object(name) else { continue };
            if let Some(w) = worlds.get(&o) {
                let t = w.w_axis;
                out[k] = [t.x.to_bits(), t.y.to_bits(), t.z.to_bits(), ONE];
            }
        }
        out
    }

    /// `TOBJ::Draw` (0x005d08d0) on layer 5: nothing asleep; the fade in
    /// (to 0.8 for Δ and Λ, else 1) and out over its last 100 frames; the
    /// matrix at its place; `tobjSeLoop` while its transparency is not 0
    /// (Ω's sleigh calls too, but nothing started a hum for it); Ω's
    /// runners where its animation puts their dummies.
    fn draw_tobj(&mut self, out: &mut Vec<MapEvent>) {
        self.drawn.clear();
        let Some(t) = &mut self.tobj else { return };
        if t.sleep != 0 {
            return;
        }
        let cap = if matches!(t.server, 0 | 2) { 0x3f4c_cccd } else { ONE };
        let mut shown = if t.server == 0 { 0x3f4c_cccd } else { ONE };
        if t.timer > 100 && ee::lt(t.alpha, ONE) {
            t.alpha = ee::add(t.alpha, 0x3c23_d70a);
            if !ee::le(t.alpha, cap) {
                t.alpha = cap;
            }
            shown = t.alpha;
        }
        if t.timer < 100 {
            t.alpha = ee::sub(t.alpha, 0x3c23_d70a);
            if ee::lt(t.alpha, 0) {
                t.alpha = 0;
            }
            shown = t.alpha;
        }
        if t.server == 3 {
            t.rot = [0x3f86_0a92, 0, 0, t.rot[3]];
        }
        let m = crate::town::pos_rot_zyx(t.pos, [t.rot[0], t.rot[1], t.rot[2]]);
        if matches!(t.server, 0 | 2 | 4) && t.alpha & 0x7fff_ffff != 0 {
            out.push(MapEvent::SeLoop(t.pos, t.alpha));
        }
        if t.far {
            return;
        }
        let shadow = (ee::mul(0x4040_0000, t.pos[2]), ((ee::to_int(ee::mul(0x4380_0000, t.alpha)) + 1) >> 1) as u8);
        let (rot, server) = (t.rot, t.server);
        self.drawn.push(TobjDraw { play: 0, matrix: m, alpha: shown, shadow });
        if server == 4 {
            for (k, p) in self.runner_places(&m).into_iter().enumerate() {
                let m = crate::town::pos_rot_zyx(p, [rot[0], 0, rot[2]]);
                self.drawn.push(TobjDraw { play: 1 + k, matrix: m, alpha: shown, shadow });
            }
        }
    }
}

impl MapSystem for Tobj {
    /// `Move`, then `Draw`; its animations on while it is awake.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw || self.tobj.is_none() {
            return;
        }
        let Some(mut rng) = cx.core.field_rand else { return };
        if let Some(t) = &mut self.tobj {
            t.step(&Env::of(cx), &mut self.cloth, &mut rng);
        }
        cx.core.field_rand = Some(rng);
        let mut out = Vec::new();
        self.draw_tobj(&mut out);
        cx.events.extend(out);
        let awake = self.tobj.as_ref().is_some_and(|t| t.sleep == 0);
        if let Some((file, plays)) = &mut self.made
            && awake
        {
            plays.iter_mut().for_each(|p| {
                p.forward(file);
            });
        }
        (0..self.drawn.len()).for_each(|k| cx.put(Kind::Tobj, Piece::Anim(k)));
    }

    /// On charLayer with its shadow; the balloon's cloths' V as `Move`
    /// left them.
    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Anim(k) = piece else { return None };
        let d = self.drawn.get(k)?;
        let (file, plays) = self.made.as_ref()?;
        let play = plays.get(d.play)?;
        let mut rows = play.uv_rows(file);
        if self.tobj.as_ref().is_some_and(|t| t.server == 1) {
            for (name, set) in CLOTHS.into_iter().zip(self.cloth.shown) {
                let Some(mat) = file.ccs.find_object(name) else { continue };
                let Some(m) = file.scene.materials.get(&mat) else { continue };
                let v = ((set as u16).wrapping_sub(m.crop_v) >> 4) as u8;
                rows.entry(mat).or_insert([0, 0])[1] = v;
            }
        }
        Some(Request::Posed {
            file,
            play,
            root: draw::mat(&d.matrix),
            layer: crate::field::area::ambient_layer(5),
            rows: Cow::Owned(rows),
            alpha: ee::f(d.alpha),
            edit: None,
            shadow: Some(d.shadow),
        })
    }

    fn take(&mut self, c: &Carried) {
        let Some(w) = c.field else { return };
        let w = &w[statics::CLOTH..];
        self.cloth = Cloth { u: w[0], v: w[1], shown: [w[2] as i32, w[3] as i32] };
    }

    fn put(&self, c: &mut CarriedMut) {
        let Some(w) = c.field.as_deref_mut() else { return };
        let c = &self.cloth;
        w[statics::CLOTH..statics::CLOTH + 4].copy_from_slice(&[c.u, c.v, c.shown[0] as u32, c.shown[1] as u32]);
    }
}
