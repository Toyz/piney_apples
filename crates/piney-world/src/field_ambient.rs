//! A field's weather and ambient pictures: what `WORLD::Init` (gcmn
//! 0x005a4cd0) and `WORLD::Generate` (0x005a6da0) make beside the ground and
//! the objects - snow, embers, smoke, steam, heat haze, rain and thunder,
//! fireflies, birds - and what `WORLD::Draw` (0x005a97b0) and
//! `WORLD::DrawEffect` (0x005a9070) draw of it each frame. The pictures come
//! out as [`Op`]s in the game's order; the runtime draws them
//! (docs/engine/field.md, "Weather and ambient pictures").

use piney_data::dungeon::Rng;

use crate::ee::{self, F, ONE, V4};
use crate::field_area::{p2w, w2p};

/// `GetWeather()` of `WORLD_MAN` (main 0x0019f190): the weather of field
/// type `ty` with background row `b`: 0 fine, 1 cloudy, 2 rain, 3 storm, 4
/// light snow, 5 heavy snow.
pub fn get_weather(ty: u32, b: u32) -> u32 {
    match ty {
        0..=3 => u32::from(b == 3),
        4 | 7 | 8 | 9 | 10 => match b {
            3 => 1,
            4 | 5 => 2,
            6 | 7 => 3,
            _ => 0,
        },
        5 | 6 => {
            if b <= 3 {
                4
            } else {
                5
            }
        }
        _ => 0,
    }
}

/// `GetTime()` (main 0x0019ef50): 0 day, 1 evening, 2 night.
pub fn get_time(ty: u32, b: u32) -> u32 {
    match ty {
        4 | 7 | 8 | 9 | 10 => match b {
            1 => 1,
            0 | 3 | 4 | 6 => 0,
            _ => 2,
        },
        5 | 6 => match b {
            1 => 1,
            0 | 3 | 4 => 0,
            _ => 2,
        },
        _ => match b {
            1 => 1,
            0 | 3 => 0,
            _ => 2,
        },
    }
}

/// The field's kind, weather and hack state as the drawing reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Weather {
    /// `WORLD.fieldType` (+0x10) and the background row (+0x14).
    pub field_type: u32,
    pub b: u32,
    /// `isHacked` (+0x2c, `WORLD_MAN` +0xf0): 3 when hacked.
    pub hack: i32,
}

impl Weather {
    pub fn weather(&self) -> u32 {
        get_weather(self.field_type, self.b)
    }

    pub fn time(&self) -> u32 {
        get_time(self.field_type, self.b)
    }

    pub fn hacked(&self) -> bool {
        self.hack == 3
    }

    /// `WORLD_MAN::CheckLensFlare` (main 0x0019f0d0): not hacked, fine,
    /// cloudy or light snow, day or evening.
    pub fn lens_flare(&self) -> bool {
        !self.hacked() && matches!(self.weather(), 0 | 1 | 4) && matches!(self.time(), 0 | 1)
    }
}

/// What the frame's drawing reads of the world around it.
pub struct Env<'a> {
    /// The disc's volume: its code's square root (`sqrt.s` from Outbreak
    /// on, [`ee::sqrtf_on`]).
    pub volume: piney_data::volume::Volume,
    /// The player's place (`plw` +0x40), which `ccTransPosW2P` / `P2W`
    /// wrap about.
    pub player: V4,
    /// `WORLD_MAN::GetCenter` (+0x80).
    pub centre: V4,
    /// `WORLD.ofs_x`, `ofs_y` (+0x6140, +0x6144).
    pub ofs: [F; 2],
    /// `WORLD::GetHeight` (the same as `WORLD_MAN::GetHeight` in a field).
    pub height: &'a dyn Fn(F, F) -> F,
    /// `cameraGetPos(camID)` and `cameraGetPos(1)`.
    pub eye: V4,
    pub eye1: V4,
    /// `cameraGetRot(1)` and `cameraGetRot2(1)`, and `checkCameraType()`
    /// 1 (the eye view), which picks the second.
    pub rot: V4,
    pub rot2: V4,
    pub eye_view: bool,
    /// `ccSys` +0x358 & 1: the frame counter's low bit.
    pub odd: bool,
    /// `WORLD_MAN` +0x420 .. +0x42c: the area's bounds, which the wrap
    /// goes by (a field's 48000, a dungeon's 60000).
    pub bounds: [F; 4],
}

impl Env<'_> {
    pub fn w2p(&self, p: V4) -> V4 {
        w2p(p, self.player, self.bounds)
    }

    pub fn p2w(&self, d: V4) -> V4 {
        p2w(d, self.player, self.bounds)
    }

    pub fn height(&self, x: F, y: F) -> F {
        (self.height)(x, y)
    }

    /// The camera's turn as the eye view picks it.
    pub fn cam_rot(&self) -> V4 {
        if self.eye_view { self.rot2 } else { self.rot }
    }
}

/// The WORLD_MAN layer a picture goes on (`SetActiveLayer(k)`): 3 the
/// effects, 5 TOBJ and BIRD, 6 the heat haze.
pub type Layer = u8;

/// A `ccEff::Draw` as the sprite then stands: its name, place, pattern,
/// scale, turn, transparency and fog bit (`+0x62` bit 5), and whether it
/// is depth-tested (the lens flare's are not: `SetRenderState(
/// CCRS_ZENABLE, 0)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sprite {
    pub layer: Layer,
    /// The CCS file the sprite's chunk is in: `field_eff`, or the field's
    /// own (`fieldccs[type]`) for the snow and embers.
    pub file: &'static str,
    pub name: &'static str,
    pub pos: V4,
    pub pattern: u16,
    pub scale: [F; 2],
    pub rotate: F,
    pub transparency: F,
    pub fog: bool,
    pub ztest: bool,
    /// The `ccEff`'s colour (+0x2c) when its code changes it (a symbol's
    /// fires); None: as `Init` left it.
    pub colour: Option<u32>,
    /// `ccEff::ChangeClut(new, old)` (main 0x0013bb20) as (new, old)
    /// palette names in `file`: the sprite draws with `new` if its palette
    /// is `old` (a dungeon's sparks, `DUNGEON::ChangeClut`); with `old` ""
    /// it draws with `new` whatever (an enemy's breath sets it outright).
    pub clut: Option<(&'static str, &'static str)>,
}

/// One picture the drawing asks for, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    Sprite(Sprite),
    /// `effSmoke(pos, v, s, life, t, in, out)` (main 0x001ce300).
    Smoke {
        pos: V4,
        v: V4,
        s: F,
        life: i32,
        t: i32,
        fade_in: u16,
        fade_out: u16,
    },
    /// The 2D smoke's `ccMask::MakePacket(0, 1)`: its transparency and
    /// top-left corner in screen pixels (512 x 512, texture 128 x 128, the
    /// colour's alpha 0x60).
    Mask {
        layer: Layer,
        alpha: F,
        x: F,
        y: F,
    },
    /// `ccClump::Draw` / `ccAnm::Draw` of a model (BIRD, TOBJ, the heat
    /// haze) with its matrix (stored columns) and transparency (the anm's
    /// `localtp`, +0x88; 1 for BIRD's clump). TOBJ casts its shadow
    /// (`ccShadowModel::Draw`, main 0x001412b0) with `ccDrawEnv` +0xa0, the
    /// shadow's length, three times its height, +0xa4, the shadow's
    /// transparency, `(256 alpha + 1) / 2`, and +0x8c `WORLD_MAN` +0x414's
    /// light (+0x410's put back after).
    Model {
        layer: Layer,
        what: Model,
        matrix: [V4; 4],
        alpha: F,
        shadow: Option<(F, u8)>,
    },
    /// `ccSeOn(n)` and `ccSeOn3D(n, pos)`.
    Se(i32),
    Se3d(i32, V4),
    /// `tobjSeLoopStart(pos)`: TOBJ's hum started, silent. Not in a
    /// frame's list: the field runtime sends it once for
    /// [`Ambient::tobj_se_start`].
    SeLoopStart,
    /// `tobjSeLoop(pos, rate)`: TOBJ's hum (`seData[44]`, which
    /// `tobjSeLoopStart` started) at its place, its volume scaled by its
    /// transparency.
    SeLoop(V4, F),
    /// `scFadeDef->EntryFlash(frames, colour, x, y, w, h)`.
    Flash {
        frames: i32,
        colour: u32,
        rect: [F; 4],
    },
}

/// Which model an [`Op::Model`] draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    /// BIRD's clump, `CMP_sfp8how1` of `field_eff`.
    Bird,
    /// TOBJ's animation (`tobjAnm[server]` of `tobjCCS[server]`), and for
    /// server 4 its two runners (`ANM_sft5run0`).
    Tobj,
    TobjRunner(u8),
    /// The heat haze, `ANM_sfzair1a` of `sfzair1`.
    Haze,
}

/// `field_eff`, `WORLD`'s effects file (+0x6154).
pub const EFF_FILE: &str = "field_eff";
/// The heat haze's file and animation (`WORLD::Init`, types 0-3).
pub const HAZE_FILE: &str = "sfzair1";
pub const HAZE_ANIM: &str = "ANM_sfzair1a";
/// The haze's model, whose texture is the picture under it.
pub const HAZE_MODEL: &str = "OBJ_sfzair00";
/// `tobjCCS[server]` and `tobjAnm[server]` (gcmn 0x006576a0, 0x006576c0),
/// and Ω's runners' animation.
pub const TOBJ: [(&str, &str); 5] = [
    ("t1", "ANM_sft1s1a"),
    ("t2", "ANM_sft2bas1a"),
    ("se3_5wh1", "ANM_se3_5wh1a"),
    ("t4", "ANM_sft4s1a"),
    ("t5", "ANM_sft5sle0"),
];
pub const TOBJ_RUNNER: &str = "ANM_sft5run0";
/// BIRD's clump in `field_eff`.
pub const BIRD_CLUMP: &str = "CMP_sfp8how1";

/// The field's own CCS file (`fieldccs[type]`, `WORLD` +0x6148).
pub fn field_file(field_type: u32) -> &'static str {
    piney_data::field::INF.ccs.get(field_type as usize).copied().flatten().unwrap_or(EFF_FILE)
}

/// A float of a `fieldrand` result, which the game converts as unsigned
/// (the results are below 2^31, so as signed).
fn fr(rng: &mut Rng, n: u32) -> F {
    ee::from_int(rng.below(n) as i32)
}

/// `ccEff::Init(chunk, 1)` then `+0x62 &= ~0x20` (the fog bit off), as
/// every ambient sprite is made; scale 1, no turn, transparency 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Eff {
    pub file: &'static str,
    pub name: &'static str,
    pub pos: V4,
    pub scale: [F; 2],
    pub transparency: F,
    pub fog: bool,
}

impl Eff {
    pub fn new(name: &'static str) -> Eff {
        Eff { file: EFF_FILE, name, pos: [0, 0, 0, ONE], scale: [ONE; 2], transparency: ONE, fog: false }
    }

    /// With the fog bit as `Init(chunk, 1)` leaves it.
    pub fn fogged(name: &'static str) -> Eff {
        Eff { fog: true, ..Eff::new(name) }
    }

    pub fn scaled(mut self, s: F) -> Eff {
        self.scale = [s; 2];
        self
    }

    /// `Draw(pattern)` at its own place.
    pub fn draw(&self, layer: Layer, pattern: u16) -> Op {
        self.draw_at(layer, self.pos, pattern)
    }

    /// `Draw(pos, pattern)`.
    pub fn draw_at(&self, layer: Layer, pos: V4, pattern: u16) -> Op {
        Op::Sprite(Sprite {
            layer,
            file: self.file,
            name: self.name,
            pos,
            pattern,
            scale: self.scale,
            rotate: 0,
            transparency: self.transparency,
            fog: self.fog,
            ztest: true,
            colour: None,
            clut: None,
        })
    }
}

const K1300: F = 0x44a2_8000;
const K2600: F = 0x4522_8000;
const K800: F = 0x4448_0000;
const K10: F = 0x4120_0000;
const K100: F = 0x42c8_0000;
const TWO: F = 0x4000_0000;
const MINUS_ONE: F = 0xbf80_0000;
const MINUS_500: F = 0xc3fa_0000;

/// One `SNOW` (gcmn 0x00503650-0x00504380, 0x50 bytes): a snowflake
/// (kinds 0 and 1) or type 0's ember (kind 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snow {
    /// +4: 0 light snow, 1 heavy snow, 2 an ember.
    pub kind: u32,
    /// +0 the ember's life, +8 frames to its next turn.
    pub timer: i32,
    pub turn: i32,
    /// +0xc the sideways spread, +0x10 the fall's scale.
    pub a: F,
    pub b: F,
    pub eff: Eff,
    /// +0x30 place, +0x40 velocity.
    pub pos: V4,
    pub vel: V4,
}

impl Snow {
    /// `calcPos` (0x005028e0): a new flake about the centre, 800 above the
    /// ground (never under 0), drifting and falling.
    fn calc_pos(&mut self, env: &Env, rng: &mut Rng) {
        let x = fr(rng, 2600);
        let y = fr(rng, 2600);
        let c = env.centre;
        self.pos[0] = ee::add(c[0], ee::sub(x, K1300));
        self.pos[1] = ee::add(c[1], ee::sub(y, K1300));
        let mut h = env.height(self.pos[0], self.pos[1]);
        if ee::lt(h, 0) {
            h = 0;
        }
        self.pos[2] = ee::add(K800, h);
        self.pos[3] = ONE;
        self.drift(rng);
        let f = ee::from_int(rng.below(5) as i32 + 1);
        self.vel[2] = ee::mul(ee::mul(MINUS_ONE, f), self.b);
        self.vel[3] = ONE;
    }

    /// `calcPos2` (0x00502ba0): an ember on the ground about the centre,
    /// rising.
    fn calc_pos2(&mut self, env: &Env, rng: &mut Rng) {
        let x = fr(rng, 2600);
        let y = fr(rng, 2600);
        let c = env.centre;
        self.pos[0] = ee::add(c[0], ee::sub(x, K1300));
        self.pos[1] = ee::add(c[1], ee::sub(y, K1300));
        self.pos[2] = env.height(self.pos[0], self.pos[1]);
        self.pos[3] = ONE;
        self.drift(rng);
        let f = ee::from_int(rng.below(3) as i32 + 2);
        self.vel[2] = ee::mul(f, self.b);
        self.vel[3] = ONE;
    }

    /// The sideways drift: `(fieldrand(a) - a / 2) / 10` for x and y.
    fn drift(&mut self, rng: &mut Rng) {
        let n = ee::to_int(self.a) as u32;
        let x = fr(rng, n);
        let half = ee::div(self.a, TWO);
        self.vel[0] = ee::div(ee::sub(x, half), K10);
        let y = fr(rng, n);
        self.vel[1] = ee::div(ee::sub(y, half), K10);
    }

    /// `SNOW(stream, kind)` (0x00503650) for kinds 0-2.
    pub fn new(kind: u32, field_type: u32, env: &Env, rng: &mut Rng) -> Snow {
        let (a, b) = match kind {
            0 => (0x4248_0000, TWO),
            1 => (0x4348_0000, 0x4080_0000),
            _ => (0x4170_0000, ONE),
        };
        let mut s = Snow { kind, timer: 0, turn: 0, a, b, eff: Eff::new(""), pos: [0; 4], vel: [0; 4] };
        if kind >= 2 {
            s.turn = rng.below(10) as i32 + 3;
            s.timer = rng.below(50) as i32 + 10;
        }
        if kind <= 1 {
            let r = fr(rng, 100);
            let k = if ee::lt(r, 0x41c8_0000) {
                0
            } else if ee::lt(r, 0x4248_0000) {
                1
            } else if ee::lt(r, 0x4296_0000) {
                2
            } else {
                3
            };
            const G: [&str; 4] = ["EFF_sfg8sno1", "EFF_sfg8sno2", "EFF_sfg8sno3", "EFF_sfg8sno4"];
            const H: [&str; 4] = ["EFF_sfh8sno1", "EFF_sfh8sno2", "EFF_sfh8sno3", "EFF_sfh8sno4"];
            s.eff = Eff::new(if field_type == 5 { G[k] } else { H[k] });
            s.eff.file = field_file(field_type);
            s.calc_pos(env, rng);
            let z0 = rng.below(800) as i32;
            // GetHeight(x, y != -500): the comparison went into the
            // argument, so this reads the height at y 0 or 1.
            let probe = if ee::eq(s.pos[1], MINUS_500) { 0 } else { ONE };
            s.pos[2] = if ee::eq(env.height(s.pos[0], probe), 0) {
                ee::from_int(z0)
            } else {
                ee::add(ee::from_int(z0), env.height(s.pos[0], s.pos[1]))
            };
            if kind == 1 {
                s.vel[2] = ee::from_int(-(rng.below(40) as i32 + 10));
            }
        } else {
            s.eff = Eff::new("EFF_sfa8fir1").scaled(0x3f00_0000);
            s.eff.file = field_file(field_type);
            s.calc_pos2(env, rng);
        }
        s
    }

    /// `SNOW::Move` (0x00503e80).
    pub fn step(&mut self, env: &Env, rng: &mut Rng) {
        self.pos = ee::vadd(self.pos, self.vel);
        self.pos[3] = ONE;
        let mut d = env.w2p(self.pos);
        for k in 0..2 {
            if !ee::le(d[k], K1300) {
                d[k] = ee::sub(d[k], K2600);
                self.pos = env.p2w(d);
            }
            if ee::lt(d[k], ee::neg(K1300)) {
                d[k] = ee::add(d[k], K2600);
                self.pos = env.p2w(d);
            }
        }
        self.eff.pos = env.p2w(d);
        if self.kind <= 1 {
            if ee::lt(self.pos[2], 0) {
                self.calc_pos(env, rng);
                if self.kind == 1 {
                    self.vel[2] = ee::from_int(-(rng.below(40) as i32 + 10));
                }
            }
            return;
        }
        self.timer -= 1;
        self.turn -= 1;
        if self.turn == 0 {
            self.turn = rng.below(10) as i32 + 3;
            self.drift(rng);
        }
        if self.timer == 0 {
            self.calc_pos2(env, rng);
            let d = env.w2p(self.pos);
            self.eff.pos = env.p2w(d);
            self.timer = rng.below(60) as i32 + 20;
        }
    }

    /// `SNOW::Draw` (0x005042f0) on `layer`: an ember fades out over its
    /// last 32 frames.
    pub fn draw(&mut self, layer: Layer, out: &mut Vec<Op>) {
        if self.kind >= 2 {
            self.eff.transparency = if self.timer < 32 { ee::div(ee::from_int(self.timer), K100) } else { ONE };
        }
        out.push(self.eff.draw(layer, 0));
    }
}

/// The 2D smoke (types 2, 3, 5, 6): `WORLD`'s `ccMask` on `TEX_sfsmo1` of
/// `field_eff`, puffs drifting across the screen from the left
/// (`SetSmoke2d`, 0x005a6a50; DrawEffect 0x005a93ac).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Smoke2d {
    /// +0x6178: x, y, dx, dy in screen pixels.
    pub puffs: [[i32; 4]; 5],
    /// +0x61c8 puffs shown, +0x61cc frames shown, +0x61d0 frames asleep,
    /// +0x61d4 the transparency (an int: 1 shown, 0 gone).
    pub count: usize,
    pub active: i32,
    pub sleep: i32,
    pub alpha: i32,
}

impl Smoke2d {
    fn puff(rng: &mut Rng) -> [i32; 4] {
        let x = -(rng.below(512) as i32);
        let y = rng.below(384) as i32;
        let dx = rng.below(20) as i32 + 20;
        let dy = rng.below(20) as i32 - 10;
        [x, y, dx, dy]
    }

    /// `WORLD::Init`'s: the sleep, then `SetSmoke2d(0..4)`.
    pub fn new(rng: &mut Rng) -> Smoke2d {
        let sleep = rng.below(300) as i32 + 150;
        let puffs = std::array::from_fn(|_| Smoke2d::puff(rng));
        Smoke2d { puffs, count: 0, active: 0, sleep, alpha: 0 }
    }

    /// DrawEffect's frame of it.
    pub fn frame(&mut self, rng: &mut Rng, out: &mut Vec<Op>) {
        if self.sleep != 0 {
            self.sleep -= 1;
            if self.sleep == 0 {
                self.active = rng.below(300) as i32 + 150;
                self.alpha = 1;
                self.count = rng.below(4) as usize + 1;
            }
            return;
        }
        if self.active != 0 {
            self.active -= 1;
        } else {
            if self.alpha != 0 {
                // fptosi(alpha - 0.005): the int goes straight to 0.
                self.alpha = ee::to_int(ee::sub(ee::from_int(self.alpha), 0x3ba3_d70a));
            }
            if ee::le(ee::from_int(self.alpha), 0) {
                self.sleep = rng.below(300) as i32 + 150;
            }
        }
        let alpha = ee::from_int(self.alpha);
        for i in 0..self.count.min(5) {
            let [x, y, dx, dy] = self.puffs[i];
            out.push(Op::Mask { layer: 3, alpha, x: ee::from_int(x), y: ee::from_int(y) });
            let (x, y) = (x + dx, y + dy);
            self.puffs[i][0] = x;
            self.puffs[i][1] = y;
            if !ee::le(ee::from_int(x), 0x4400_0000) || !ee::le(ee::from_int(y), 0x43c0_0000) || y < -512 {
                self.puffs[i] = Smoke2d::puff(rng);
            }
        }
    }
}

/// The drawing's function statics, which live as long as the game does
/// (the first `DrawRain` of the whole game seeds its thunder's count):
/// `DrawRain`'s `tcnt`, `tcnt2`, `secnt`, `seflag` (gcmn 0x00378d24-40),
/// `DrawSteam`'s `life`, `sleep`, `step`, `num` and `pos`
/// (0x00378d5c-74, 0x00730580).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Statics {
    /// None until the first `DrawRain` draws it.
    pub tcnt: Option<i32>,
    pub tcnt2: i32,
    pub secnt: i32,
    pub seflag: bool,
    pub steam_life: i32,
    pub steam_sleep: i32,
    pub steam_step: F,
    pub steam_num: i32,
    pub steam_pos: V4,
}

impl Statics {
    /// As the executable starts them.
    pub fn new() -> Statics {
        Statics { tcnt2: 750, steam_step: 0x3f00_0000, ..Statics::default() }
    }
}

/// The statics as the last field left them (the game's live as long as
/// it runs).
static KEPT: std::sync::Mutex<Option<(Statics, Cloth)>> = std::sync::Mutex::new(None);

/// The drawing's statics for a new field: the last field's, or the
/// executable's own the first time.
pub fn take_statics() -> (Statics, Cloth) {
    KEPT.lock().ok().and_then(|mut k| k.take()).unwrap_or((Statics::new(), Cloth::default()))
}

/// A field's statics kept for the next.
pub fn keep_statics(s: Statics, c: Cloth) {
    if let Ok(mut k) = KEPT.lock() {
        *k = Some((s, c));
    }
}

/// One of the fifty rain drops (`WORLD` +0x5400, 64 bytes each).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drop {
    /// +0 the drop, +4 its splash (`EFF_sfp8rai6`).
    pub drop: Eff,
    pub splash: Eff,
    /// +0x10 where it falls (x, y about the eye), +0x20 the splash's
    /// (kept, never drawn: the splash is drawn at the drop's place).
    pub pos: V4,
    pub splash_pos: V4,
    /// +0x30 the drop's pattern (0-19), +0x34 the splash's (0-14).
    pub pattern: i32,
    pub splash_pattern: i32,
    /// +0x38 the splash shows.
    pub splash_on: bool,
}

/// The storm's lightning: `EFF_sfp8thu1-4` (+0x6080..), the one struck
/// (+0x6094) and the frames it shows (+0x6090).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Thunder {
    pub effs: [Eff; 4],
    pub index: usize,
    pub frames: i32,
}

impl Thunder {
    pub fn new() -> Thunder {
        let e = ["EFF_sfp8thu1", "EFF_sfp8thu2", "EFF_sfp8thu3", "EFF_sfp8thu4"].map(Eff::new);
        Thunder { effs: e, index: 0, frames: 0 }
    }
}

impl Default for Thunder {
    fn default() -> Self {
        Thunder::new()
    }
}

/// `SetRainEffect` (0x005a4840): the drops for rain (weather 2:
/// `EFF_sfp8rai1-4`) or storm (3: `rai7-10`), each picked by
/// `fieldrand(4000) / 1000`, with its pattern from `fieldrand(20)`.
pub fn set_rain(weather: u32, rng: &mut Rng) -> Vec<Drop> {
    const RAIN: [&str; 4] = ["EFF_sfp8rai1", "EFF_sfp8rai2", "EFF_sfp8rai3", "EFF_sfp8rai4"];
    const STORM: [&str; 4] = ["EFF_sfp8rai7", "EFF_sfp8rai8", "EFF_sfp8rai9", "EFF_sfp8rai10"];
    (0..50)
        .map(|_| {
            let k = (rng.below(4000) / 1000) as usize;
            let drop = Eff::fogged(if weather == 2 { RAIN[k] } else { STORM[k] });
            let pattern = rng.below(20) as i32;
            Drop {
                drop,
                splash: Eff::fogged("EFF_sfp8rai6"),
                pos: [0; 4],
                splash_pos: [0; 4],
                pattern,
                splash_pattern: 0,
                splash_on: false,
            }
        })
        .collect()
}

/// `DrawRain` (0x005a8860) for weather 2 and 3, on layer 3. Each drop is
/// placed again every frame: the flag the code tests (+0x3c) is only ever
/// cleared, and the one it sets is `WORLD` +0x1c.
pub fn draw_rain(
    drops: &mut [Drop],
    thunder: Option<&mut Thunder>,
    weather: u32,
    st: &mut Statics,
    env: &Env,
    rng: &mut Rng,
    out: &mut Vec<Op>,
) {
    if st.tcnt.is_none() {
        st.tcnt = Some(rng.below(60) as i32 + 60);
    }
    let eye = env.eye1;
    for d in drops.iter_mut() {
        let x = rng.below(3000) as i32 - 1500;
        let y = rng.below(3000) as i32 - 1500;
        d.pos[0] = ee::add(ee::from_int(x), eye[0]);
        d.pos[1] = ee::add(ee::from_int(y), eye[1]);
        let mut p = env.p2w(env.w2p(d.pos));
        p[2] = env.height(p[0], p[1]);
        d.drop.pos = p;
        out.push(d.drop.draw(3, d.pattern as u16));
        d.pattern += 1;
        if d.pattern == 20 {
            d.pattern = 0;
            d.splash_pattern = 0;
            d.splash_on = true;
            d.splash_pos = d.pos;
            d.splash_pos[2] = ee::add(d.splash_pos[2], K10);
        }
        if d.splash_on {
            p[2] = ee::add(K10, env.height(p[0], p[1]));
            d.splash.pos = p;
            out.push(d.splash.draw(3, d.splash_pattern as u16));
            d.splash_pattern += 1;
            if d.splash_pattern == 15 {
                d.splash_on = false;
            }
        }
    }
    if weather == 3
        && let Some(th) = thunder
    {
        storm(th, st, env, rng, out);
    }
}

/// `DrawRain`'s storm: a bolt somewhere ahead every 30-59 frames (the
/// first after 60-119), shown four frames at the centre's offset; a flash
/// of the screen every 900-1499 frames (the first after 750) and its
/// thunder (sound 236) 30-49 frames later.
fn storm(th: &mut Thunder, st: &mut Statics, env: &Env, rng: &mut Rng, out: &mut Vec<Op>) {
    let tcnt = st.tcnt.get_or_insert(0);
    *tcnt -= 1;
    st.tcnt2 -= 1;
    if *tcnt == 0 {
        th.index = (rng.below(4000) / 1000) as usize;
        let e = &mut th.effs[th.index];
        e.pos[0] = 0;
        e.pos[1] = 0xc63b_8000;
        let dy = rng.below(6000) as i32 - 3000;
        // The difference is converted as unsigned: a negative one comes
        // out near 2^32.
        e.pos[1] = ee::add(e.pos[1], uint_float(dy as u32));
        e.pos[2] = 0x4461_0000;
        let mut rot = env.cam_rot();
        rot[0] = 0;
        rot[1] = 0;
        let r = ee::sub(ee::div(uint_float(rng.below(157)), K100), 0x3f48_f5c3);
        let z = ee::add(rot[2], r);
        if ee::lt(z, 0xc048_f5c3) || !ee::le(z, 0x4048_f5c3) {
            rot[2] = ee::mul(MINUS_ONE, rot[2]);
        }
        rot[2] = ee::add(rot[2], r);
        let m = piney_data::anim::rot_bits([rot[0], rot[1], rot[2]]);
        e.pos = ee::apply(&m, e.pos);
        th.frames = 4;
        *tcnt = rng.below(30) as i32 + 30;
    }
    if st.tcnt2 == 0 {
        out.push(Op::Flash { frames: 10, colour: 0x80ff_ffff, rect: [0, 0, 0x4400_0000, 0x43c0_0000] });
        st.tcnt2 = rng.below(600) as i32 + 900;
        st.secnt = rng.below(20) as i32 + 30;
        st.seflag = true;
    }
    if st.seflag {
        st.secnt -= 1;
    }
    if st.secnt == 0 && st.seflag {
        out.push(Op::Se(236));
        st.seflag = false;
    }
    if th.frames != 0 {
        let e = &mut th.effs[th.index];
        // Added for the draw and taken off again, rounding each way.
        e.pos[0] = ee::add(e.pos[0], env.ofs[0]);
        e.pos[1] = ee::add(e.pos[1], env.ofs[1]);
        out.push(e.draw(3, 0));
        e.pos[0] = ee::sub(e.pos[0], env.ofs[0]);
        e.pos[1] = ee::sub(e.pos[1], env.ofs[1]);
        th.frames -= 1;
    }
}

/// An unsigned int as a float, as the EE converts one (`cvt.s.w` of the
/// value halved, rounding bit kept, doubled when the top bit is set).
fn uint_float(u: u32) -> F {
    if (u as i32) >= 0 {
        ee::from_int(u as i32)
    } else {
        let h = (u >> 1) | (u & 1);
        let f = ee::from_int(h as i32);
        ee::add(f, f)
    }
}

/// DrawEffect's smoke over one of type 7's fires (`WORLD` +0x360): the
/// fire's place brought to the player's side (and kept so); within 6500 of
/// the eye on even frames, one `effSmoke` thrown 10 out along a direction
/// turned by three `ccRand()` angles about x, then y, then z.
pub fn fire_smoke(fire: &mut V4, env: &Env, cc: &mut dyn FnMut() -> u32, out: &mut Vec<Op>) {
    *fire = env.p2w(env.w2p(*fire));
    let d = piney_battle::enemy_ai::get_dist_on(env.volume, env.eye, *fire);
    if !ee::lt(d, 0x45cb_2000) || env.odd {
        return;
    }
    let unit = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];
    let a = ee::deg2rad(cc() as u16 as i16);
    let mut m = piney_data::anim::rot_x_bits_of(unit, a);
    let b = ee::deg2rad(cc() as u16 as i16);
    m = piney_data::anim::rot_y_bits_of(m, b);
    let c = ee::deg2rad(cc() as u16 as i16);
    m = piney_data::anim::rot_z_bits_of(m, c);
    let off = ee::apply(&m, [K10, 0, 0, ONE]);
    let mut pos = *fire;
    for k in 0..3 {
        pos[k] = ee::add(pos[k], off[k]);
    }
    let mut v = ee::apply(&m, [ONE, 0, 0, ONE]);
    v[2] = ee::add(v[2], 0x4020_0000);
    out.push(Op::Smoke { pos, v, s: ONE, life: 8, t: 208, fade_in: 512, fade_out: 32 });
}

/// `WORLD::DrawSteam(1)` (0x005ad600), type 0's steam vents: asleep, or a
/// burst of 2-6 `effSmoke` puffs at a place about the player (chosen, with
/// its hiss, `ccSeOn3D(43)`, at the first of eight bursts 3-6 frames
/// apart), then asleep for 120-270 frames.
pub fn draw_steam(st: &mut Statics, env: &Env, rng: &mut Rng, out: &mut Vec<Op>) {
    if st.steam_sleep != 0 {
        st.steam_sleep -= 1;
        return;
    }
    if st.steam_num == 0 {
        let x = rng.below(2000) as i32 - 1000;
        let y = rng.below(2000) as i32 - 1000;
        st.steam_pos[0] = ee::from_int(x);
        st.steam_pos[1] = ee::from_int(y);
        st.steam_pos = env.p2w(st.steam_pos);
        st.steam_pos[2] = ee::add(0x41a0_0000, env.height(st.steam_pos[0], st.steam_pos[1]));
        st.steam_pos[3] = ONE;
        out.push(Op::Se3d(43, st.steam_pos));
    }
    st.steam_sleep = rng.below(4) as i32 + 3;
    let n = rng.below(5) as i32 + 2;
    for _ in 0..n {
        let vx = ee::div(uint_float(rng.below(50)), K100);
        let vy = ee::div(uint_float(rng.below(50)), K100);
        let vz = uint_float(rng.below(10) + 10);
        let s = rng.below(5) as i32 + 3;
        st.steam_life = (rng.below(3) as i32 + 1) * 10;
        out.push(Op::Smoke {
            pos: st.steam_pos,
            v: [vx, vy, vz, ONE],
            s: ee::from_int(s),
            life: st.steam_life,
            t: 109,
            fade_in: 512,
            fade_out: 32,
        });
    }
    st.steam_num += 1;
    if st.steam_num == 8 {
        st.steam_sleep = (rng.below(6) as i32 + 4) * 30;
        st.steam_num = 0;
    }
    st.steam_sleep -= 1;
}

/// `WORLD::DrawSteam(0)` from `Init`: the statics started again.
pub fn reset_steam(st: &mut Statics) {
    st.steam_sleep = 0;
    st.steam_step = 0x3f00_0000;
    st.steam_num = 0;
}

/// `BIRD` (gcmn 0x005d1440-0x005d17c8): type 10's hawk, `CMP_sfp8how1`
/// of `field_eff`, circling out and back from its home.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bird {
    /// +0x10 the turn a frame (0.0314), +0x40 the heading.
    pub turn: F,
    pub angle: F,
    /// +0x20 home, +0x30 place.
    pub home: V4,
    pub pos: V4,
    /// +4: farther than 9600 by its own reckoning (never read).
    pub far: bool,
}

const BIRD_TURN: F = 0x3d00_adfd;

impl Bird {
    /// `Init(stream, "CMP_sfp8how1", 0)` then `SetPos(x, y)`.
    pub fn new(x: F, y: F) -> Bird {
        let home = [x, y, 0x4448_0000, 0];
        Bird { turn: BIRD_TURN, angle: ee::PI, home, pos: home, far: false }
    }

    /// `BIRD::Move` (0x005d1590): a step of 20 along the heading, the
    /// matrix at its place turned by the heading about z; past half a turn
    /// either way it turns back from home.
    pub fn step(&mut self, env: &Env) -> [V4; 4] {
        let sx = ee::from_int(ee::to_int(ee::mul(0x41a0_0000, ee::sinf(self.angle))));
        let sy = ee::from_int(ee::to_int(ee::mul(0xc1a0_0000, ee::cosf(self.angle))));
        let d = env.w2p(self.pos);
        self.pos = env.p2w(d);
        self.pos = ee::vadd(self.pos, [sx, sy, 0, ONE]);
        // sqrt(d.y + d.y + d.x d.x): d.y where d.y d.y was meant.
        let s = ee::add(d[1], ee::add(d[1], ee::mul(d[0], d[0])));
        self.far = f64::from(ee::f(s)).sqrt() > 9600.0;
        let m = crate::town::pos_rot_zyx(self.pos, [0, 0, self.angle]);
        self.angle = ee::add(self.angle, self.turn);
        if !ee::le(self.turn, 0) {
            if !ee::lt(self.angle, ee::PI) {
                self.angle = ee::sub(self.angle, self.turn);
                self.turn = BIRD_TURN | 0x8000_0000;
                self.pos[..3].copy_from_slice(&self.home[..3]);
            }
        } else if ee::le(self.angle, ee::neg(ee::PI)) {
            self.angle = ee::sub(self.angle, self.turn);
            self.turn = BIRD_TURN;
            self.pos[..3].copy_from_slice(&self.home[..3]);
        }
        m
    }
}

/// `TOBJ` (gcmn 0x005d04c0-0x005d13c0): the server's traveller that now
/// and then crosses a field (`fieldrand(100) >= 86` in `Generate`): Δ's
/// airship (`ANM_sft1s1a` of `t1`), Θ's balloon (`sft2bas1a`, its cloth
/// scrolling), Λ's whale (`se3_5wh1a`), Σ's (`sft4s1a`, standing), Ω's
/// sleigh (`sft5sle0` and two runners, `ANM_sft5run0`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tobj {
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

/// 57.29578: radians from degrees.
const DEG: F = 0x4265_2ee0;
const K2400: F = 0x4516_0000;

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

/// The balloon's cloth materials, whose V the scrolls set.
pub const CLOTHS: [&str; 2] = ["MAT_sft2clo1", "MAT_sft2clo2"];

impl Tobj {
    /// `TOBJ::Init(stream, tobjAnm[server], server)` (0x005d0560).
    pub fn new(server: u32, rng: &mut Rng) -> Tobj {
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
        let mut t =
            Tobj { server, timer, far: false, sleep: 0, pos, rot: [0, 0, angle, ONE], angle, speed: K10, alpha: 0 };
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
    pub fn step(&mut self, env: &Env, cloth: &mut Cloth, rng: &mut Rng) {
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
                self.pos[2] = Tobj::height_at(env, self.pos, K2400);
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
                self.pos[2] = Tobj::height_at(env, self.pos, 0x457a_0000);
            }
            3 => {
                self.far = false;
                d = env.w2p(self.pos);
                self.pos = env.p2w(d);
                self.pos[2] = Tobj::height_at(env, self.pos, 0x45bb_8000);
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
        self.pos[2] = Tobj::height_at(env, self.pos, K2400);
        if self.server == 1 {
            self.pos[2] = ee::add(self.pos[2], 0x44c8_0000);
            self.rot[2] = 0;
            let r = crate::rtownpc::get_dirc([self.pos[0], self.pos[1], 0, 0], d);
            self.rot[0] = ee::div(r, 0x4080_0000);
        }
        if self.server == 3 {
            self.pos = env.player;
            self.pos[0] = ee::add(self.pos[0], ee::sub(uint_float(rng.below(4000)), 0x44fa_0000));
            self.pos[1] = ee::add(self.pos[1], ee::sub(uint_float(rng.below(4000)), 0x44fa_0000));
        }
        self.sleep = rng.below(100) as i32 + 30;
    }

    /// `TOBJ::Draw` (0x005d08d0) on layer 5: nothing asleep; the fade in
    /// (to 0.8 for Δ and Λ, else 1) and out over its last 100 frames; the
    /// matrix at its place; `tobjSeLoop` while its transparency is not 0;
    /// Ω's runners where `runners` puts them for the
    /// matrix (the places of `OBJ_dummy_rei0`, `rei1` in its animation).
    pub fn draw(&mut self, runners: &dyn Fn(&[V4; 4]) -> [V4; 2], out: &mut Vec<Op>) {
        if self.sleep != 0 {
            return;
        }
        let cap = if matches!(self.server, 0 | 2) { 0x3f4c_cccd } else { ONE };
        let mut shown = if self.server == 0 { 0x3f4c_cccd } else { ONE };
        if self.timer > 100 && ee::lt(self.alpha, ONE) {
            self.alpha = ee::add(self.alpha, 0x3c23_d70a);
            if !ee::le(self.alpha, cap) {
                self.alpha = cap;
            }
            shown = self.alpha;
        }
        if self.timer < 100 {
            self.alpha = ee::sub(self.alpha, 0x3c23_d70a);
            if ee::lt(self.alpha, 0) {
                self.alpha = 0;
            }
            shown = self.alpha;
        }
        if self.server == 3 {
            self.rot = [0x3f86_0a92, 0, 0, self.rot[3]];
        }
        let m = crate::town::pos_rot_zyx(self.pos, [self.rot[0], self.rot[1], self.rot[2]]);
        // The hum follows it while it shows, far or not; Ω's sleigh calls
        // too, but nothing started a hum for it.
        if matches!(self.server, 0 | 2 | 4) && self.alpha & 0x7fff_ffff != 0 {
            out.push(Op::SeLoop(self.pos, self.alpha));
        }
        if self.far {
            return;
        }
        let shadow =
            Some((ee::mul(0x4040_0000, self.pos[2]), ((ee::to_int(ee::mul(0x4380_0000, self.alpha)) + 1) >> 1) as u8));
        out.push(Op::Model { layer: 5, what: Model::Tobj, matrix: m, alpha: shown, shadow });
        if self.server == 4 {
            for (k, &p) in runners(&m).iter().enumerate() {
                let m = crate::town::pos_rot_zyx(p, [self.rot[0], 0, self.rot[2]]);
                out.push(Op::Model { layer: 5, what: Model::TobjRunner(k as u8), matrix: m, alpha: shown, shadow });
            }
        }
    }
}

/// `SetMatrix_PosRotXYZScale(pos, rot, scale)` (main 0x00138050): the
/// scale, turned about x, then y, then z, then moved to `pos`.
pub fn pos_rot_xyz_scale(pos: V4, rot: V4, scale: [F; 3]) -> [V4; 4] {
    let mut m = [[scale[0], 0, 0, 0], [0, scale[1], 0, 0], [0, 0, scale[2], 0], [0, 0, 0, ONE]];
    m = piney_data::anim::rot_x_bits_of(m, rot[0]);
    m = piney_data::anim::rot_y_bits_of(m, rot[1]);
    m = piney_data::anim::rot_z_bits_of(m, rot[2]);
    for k in 0..3 {
        m[3][k] = ee::add(m[3][k], pos[k]);
    }
    m
}

/// The heat haze of field types 0-3 (`ANM_sfzair1a` of `sfzair1`, its
/// texture the frame buffer): 6000 ahead of the player along the
/// camera's heading (3000 for types 0 and 1), scaled 1.3 wide, at
/// transparency 0.7, on layer 6.
pub fn haze(field_type: u32, env: &Env) -> Op {
    let mut pos = env.player;
    pos[2] = 0;
    let ahead = if field_type <= 1 { 0xc53b_8000 } else { 0xc5bb_8000 };
    let mut rot = env.cam_rot();
    rot[0] = 0;
    rot[1] = 0;
    let m = piney_data::anim::rot_bits([rot[0], rot[1], rot[2]]);
    let mut off = ee::apply(&m, [0, ahead, 0, ONE]);
    off = ee::vadd(off, pos);
    off[3] = ONE;
    let off = env.p2w(env.w2p(off));
    let matrix = pos_rot_xyz_scale(off, rot, [0x3fa6_6666, ONE, ONE]);
    Op::Model { layer: 6, what: Model::Haze, matrix, alpha: 0x3f33_3333, shadow: None }
}

/// The lens flare's sun for field type `ty` and background row `b`: the
/// dummy `DMY_<x>lig_<b + 1>point` of the background's file (the table at
/// gcmn 0x006588f0 by type; none for type 4).
pub fn flare_dummy(ty: u32, b: u32) -> Option<String> {
    let x = match ty {
        0 => "sfalig",
        1 => "sfblig",
        2 => "sfclig",
        3 => "sfdlig",
        5 => "sfglig",
        6 => "sfhlig",
        7 => "sfilig",
        8 => "sfklig",
        9 => "sfmlig",
        10 => "sfp7lig",
        _ => return None,
    };
    let rows = if ty <= 3 { 4 } else { 7 };
    (b < rows).then(|| format!("DMY_{x}_{}point", b + 1))
}

/// `LENSFLARE::Draw(bgstream, name, 1)` (0x00503360) as a field draws it,
/// on layer 3: nothing unless `ccCheckCameraDeg(cameraGetPos(camID),
/// 12288)` - which, asked of the camera's own place, tests only that the
/// camera looks within 67.5 degrees of +x; the sun the dummy's place taken
/// from the player's (`ccTransPosP2W`); each of the six flares brought to
/// the player's side of the map.
pub fn field_flare(dummy: V4, cam: &crate::evarea::FlareCamera, env: &Env, out: &mut Vec<Op>) {
    if !check_camera_deg(cam.eye, cam.eye, cam.view, crate::lensflare::VIEW_DEG, env) {
        return;
    }
    let sun = env.p2w(dummy);
    for (k, p) in crate::lensflare::points(sun, cam).into_iter().enumerate() {
        let p = env.p2w(env.w2p(p));
        out.push(Op::Sprite(Sprite {
            layer: 3,
            file: EFF_FILE,
            name: crate::lensflare::FLARES[k],
            pos: p,
            pattern: 0,
            scale: [ONE; 2],
            rotate: 0,
            transparency: ONE,
            fog: false,
            ztest: false,
            colour: None,
            clut: None,
        }));
    }
}

/// `ccCheckCameraDeg(pos, deg)` (main 0x001da710) over the field's frame:
/// `pos` within `deg` of where the camera (`eye` looking at `view`) looks,
/// on the ground, all taken relative to the player.
fn check_camera_deg(pos: V4, eye: V4, view: V4, deg: i16, env: &Env) -> bool {
    let c = env.w2p(eye);
    let v = env.w2p(view);
    let p = env.w2p(pos);
    let a = ee::rad2deg(ee::atan2f(ee::sub(p[1], c[1]), ee::sub(p[0], c[0])));
    let b = ee::rad2deg(ee::atan2f(ee::sub(v[1], c[1]), ee::sub(v[0], c[0])));
    let x = (i32::from(deg) + (i32::from(a) - i32::from(b))) as i16;
    x > 0 && i32::from(x) < 2 * i32::from(deg)
}

/// A field's weather and ambient pictures, as `WORLD` holds them.
#[derive(Clone, Debug, PartialEq)]
pub struct Ambient {
    pub w: Weather,
    /// `WORLD` +0x38: the snowflakes, or type 0's sixteen embers.
    pub snow: Vec<Snow>,
    pub smoke: Option<Smoke2d>,
    pub drops: Vec<Drop>,
    pub thunder: Option<Thunder>,
    /// Type 7's fires (+0x358, +0x360), which `SetSpecialObj` places.
    pub fires: Vec<V4>,
    pub fireflies: Vec<crate::field_firefly::FieldFirefly>,
    pub roamers: Vec<crate::firefly::Firefly>,
    pub tobj: Option<Tobj>,
    /// `TOBJ::Init` called `tobjSeLoopStart` (Δ's airship, Λ's whale) and
    /// the runtime has not yet started the hum.
    pub tobj_se_start: bool,
    pub bird: Option<Bird>,
    /// Types 0-3: the heat haze.
    pub haze: bool,
    /// +0x34: `Draw` has run once.
    pub drawn: bool,
}

/// What a frame's [`Ambient::draw`] reads beyond [`Env`].
pub struct Frame<'a> {
    pub env: Env<'a>,
    /// The lens flare's camera, and its sun (the background's dummy).
    pub flare_cam: crate::evarea::FlareCamera,
    pub sun: Option<V4>,
    /// The places of Ω's runners' dummies in TOBJ's animation under its
    /// matrix.
    pub runners: &'a dyn Fn(&[V4; 4]) -> [V4; 2],
}

impl Ambient {
    /// `WORLD::Init`'s part (after the models), drawing from `rng` (which
    /// `GO` seeded; `Init`'s draws come before `Generate`'s) with the
    /// heights as `Init` sees them (the height map not yet made).
    pub fn init(w: Weather, env: &Env, st: &mut Statics, rng: &mut Rng) -> Ambient {
        let ty = w.field_type;
        let weather = w.weather();
        let mut snow = Vec::new();
        match weather {
            5 => snow.extend((0..200).map(|_| Snow::new(1, ty, env, rng))),
            4 => snow.extend((0..100).map(|_| Snow::new(0, ty, env, rng))),
            _ => {}
        }
        if ty == 0 {
            snow = (0..16).map(|_| Snow::new(2, ty, env, rng)).collect();
        }
        let smoke = matches!(ty, 2 | 3 | 5 | 6).then(|| Smoke2d::new(rng));
        if ty == 0 {
            reset_steam(st);
        }
        Ambient {
            w,
            snow,
            smoke,
            drops: Vec::new(),
            thunder: None,
            fires: Vec::new(),
            fireflies: Vec::new(),
            roamers: Vec::new(),
            tobj: None,
            tobj_se_start: false,
            bird: None,
            haze: ty <= 3,
            drawn: false,
        }
    }

    /// `WORLD::Generate`'s part after `SetStartPos`: `rng` as the layout
    /// left it; `server` is `ccGame.server`, `start` the start position,
    /// `entrance` the dungeon entrance's (`WORLD_MAN` +0x460).
    pub fn generate(&mut self, server: i32, start: V4, entrance: V4, env: &Env, rng: &mut Rng) {
        let w = self.w;
        let ty = w.field_type;
        if (0..=4).contains(&server) && rng.below(100) >= 86 {
            self.tobj = Some(Tobj::new(server as u32, rng));
            self.tobj_se_start = matches!(server, 0 | 2);
        }
        let weather = w.weather();
        if matches!(weather, 2 | 3) {
            self.drops = set_rain(weather, rng);
        }
        if weather == 3 {
            self.thunder = Some(Thunder::new());
        }
        if (w.time() == 2 || w.hacked()) && ty != 9 {
            let bounds = [0x473b_8000, 0x473b_8000];
            for _ in 0..5 {
                let f = crate::field_firefly::FieldFirefly::new(w.hacked(), bounds, env.height, rng);
                self.fireflies.push(f);
            }
        }
        if ty == 9 {
            let fly = crate::firefly::Fly {
                volume: env.volume,
                player: env.player,
                bounds: env.bounds,
                eye: env.eye,
                rot: env.cam_rot(),
                height: env.height,
            };
            for _ in 0..15 {
                let d = |rng: &mut Rng| ee::div(ee::from_int(rng.below(2000) as i32 - 1000), 0x447a_0000);
                let dirc = [d(rng), d(rng), d(rng), 0];
                let mut base = start;
                base[2] = ee::add(base[2], uint_float(rng.below(300)));
                self.roamers.push(crate::firefly::Firefly::roamer(rng, dirc, base, &fly));
            }
        }
        if ty == 10 && w.time() != 2 && !matches!(weather, 2 | 3) {
            self.bird = Some(Bird::new(entrance[0], entrance[1]));
        }
    }

    /// `DrawSnow` (0x005a8f40): every flake moved, then every one drawn.
    fn draw_snow(&mut self, layer: Layer, env: &Env, rng: &mut Rng, out: &mut Vec<Op>) {
        if !matches!(self.w.weather(), 4 | 5) {
            return;
        }
        for s in &mut self.snow {
            s.step(env, rng);
        }
        for s in &mut self.snow {
            s.draw(layer, out);
        }
    }

    /// `WORLD::Draw`'s and `DrawEffect`'s pictures for a frame, in their
    /// order. `cc` is `ccRand`; `cloth` TOBJ's scrolls.
    pub fn draw(
        &mut self,
        f: &Frame,
        st: &mut Statics,
        cloth: &mut Cloth,
        rng: &mut Rng,
        cc: &mut dyn FnMut() -> u32,
    ) -> Vec<Op> {
        let env = &f.env;
        let mut out = Vec::new();
        let ty = self.w.field_type;
        let weather = self.w.weather();
        if !self.drawn {
            self.drawn = true;
            if matches!(weather, 4 | 5) {
                for _ in 0..10 {
                    self.draw_snow(0, env, rng, &mut out);
                }
            }
        }
        if self.haze {
            out.push(haze(ty, env));
        }
        // DrawEffect, layer 3.
        if matches!(weather, 2 | 3) {
            draw_rain(&mut self.drops, self.thunder.as_mut(), weather, st, env, rng, &mut out);
        }
        if ty == 7 {
            for fire in &mut self.fires {
                fire_smoke(fire, env, cc, &mut out);
            }
        }
        self.draw_snow(3, env, rng, &mut out);
        if ty == 0 {
            for s in &mut self.snow {
                s.step(env, rng);
            }
            for s in &mut self.snow {
                s.draw(3, &mut out);
            }
        }
        if let Some(s) = &mut self.smoke {
            s.frame(rng, &mut out);
        }
        if self.w.time() == 2 || self.w.hacked() || ty == 9 {
            if ty == 9 {
                if !matches!(weather, 2 | 3) {
                    let fly = crate::firefly::Fly {
                        volume: env.volume,
                        player: env.player,
                        bounds: env.bounds,
                        eye: env.eye,
                        rot: env.cam_rot(),
                        height: env.height,
                    };
                    for r in &mut self.roamers {
                        r.step_in(&fly, rng);
                        let mut draws = Vec::new();
                        r.draw_in(&fly, &mut draws);
                        out.extend(draws.into_iter().map(|d| {
                            Op::Sprite(Sprite {
                                layer: 3,
                                file: EFF_FILE,
                                name: "EFF_sfzfir1",
                                pos: d.pos,
                                pattern: d.pattern,
                                scale: [d.scale; 2],
                                rotate: 0,
                                transparency: d.transparency,
                                fog: true,
                                ztest: true,
                                colour: None,
                                clut: None,
                            })
                        }));
                    }
                }
            } else {
                for fl in &mut self.fireflies {
                    fl.step(env, ty, rng);
                    fl.draw(env, "EFF_sfpfir_1", rng, &mut out);
                }
            }
        }
        // Layer 5: TOBJ, then BIRD.
        if let Some(t) = &mut self.tobj {
            t.step(env, cloth, rng);
            t.draw(f.runners, &mut out);
        }
        if let Some(b) = &mut self.bird {
            let m = b.step(env);
            out.push(Op::Model { layer: 5, what: Model::Bird, matrix: m, alpha: ONE, shadow: None });
        }
        // Layer 3: the lens flare.
        if self.w.lens_flare()
            && let Some(sun) = f.sun
        {
            field_flare(sun, &f.flare_cam, env, &mut out);
        }
        if ty == 0 {
            draw_steam(st, env, rng, &mut out);
        }
        out
    }
}
