//! The magic portal: `ccMagicCircle` (gcmn gmcircle.cpp, 0x39f0 bytes over
//! `ccGimmick`), the gimmick of `gimmickTbl[15]` ("Magic Portal",
//! `XMAGCIR.CCS`) that a field or dungeon puts where its portals stand, and
//! what it draws: the circle (`CMP_xmagcir0` posed by an animation) and up
//! to 128 sparks (`ccMcPart`, each a `ccEff` of `EFF_xmagpat1`).
//!
//! ```text
//! ccEntryCtrl::entryMagicCircle(ep) (gcmn 0x004313f0), from
//!   entryObjectCheck: the portal dropped on the ground (ccLandHitCheck(pos,
//!   0x20000002) + 250 into ep.pos), gimmickTbl[15]'s entry made
//!   (ccEntryGimCircle 0x00455820: new ccMagicCircle), initObject, linked
//!   onto the circle list, g_entCtrl +0x1c (the circles) up by one
//! ccMagicCircle::ccMagicCircle (0x00455900): pos and dirc the entry's,
//!   posP W2P(pos); act 0; CMP_xmagcir0 with the entry's palette CLT_x031c2
//!   on MAT_clut (ccEntryChangeCLUT), ANM_xmagcir1 (the idle loop); no sparks
//!   yet; off the command lists (deleteCmnd(1))
//! ccMagicCircle::main (0x00455b60), each frame after ccEntryObj::routine:
//!   idle and frozen (Kite beyond 10000): the sparks' ccEffs freed; done
//!   the 128 sparks made (ccEff::Init(EFF_xmagpat1, 0)) if they are not
//!   pos through W2P and back (posP kept)
//!   act 0  Kite listed (ccCheckTarget) and within 3000: sounds 216, 217;
//!          act 1
//!   act 1  32 frames; then ANM_xmagcir2 (the opening), act 2, destFlag
//!   act 2  22 frames; then sound 215 and ccEntryCtrl::entryCircleObject
//!          (what the portal holds comes out); act 3
//!   act 3  until the opening's animation ends; act 4
//!   act 4  66 frames; then (entRoot set) the portal counters and, for the
//!          entry control's last circle, ccThDfComp; main returns 1: the
//!          entry control deletes it (ccDestMagicCircle)
//!   createPart: new sparks at the portal by act - 0: 3 of kind 1 a frame;
//!     1: (actCnt / 4 + 1) of kind 2; 2: 8 of kind 4; 3: 2 of kind 3 while
//!     actCnt < 16 (setPart: the first free of 128 from partTop, else the
//!     one at partTop, which moves on)
//!   every live spark's ccMcPart::main
//!   idle beyond 7000 of Kite: nothing drawn; else the animation steps
//!   (anmFlag its end) and, with dispSW (routine: within 7000) and inside
//!   ccCheckCameraDeg(pos, 12288): on the effect layer the circle at
//!   T(pos) Rx Ry Rz at ccGetCameraTransparency(pos, 0, 0, 7000, 600) times
//!   setTransparency, then each live spark at that times the product of its
//!   own transparency and every earlier spark's (the running product,
//!   clamped to 0..1)
//! ccMcPart::main (0x004548e0): the spark's ccEff to ccTransPosFW2LW; at its
//!   first frame the portal's position, a random turn (three ccRand angles)
//!   and a life; then by kind:
//!     1 (idle)    120 frames at scale 5 shrinking to 1: bobs out along its
//!                 turned x by 6 sin(radCnt += 838) and up and down by
//!                 6 sin(1383 cnt)
//!     2 (opening) 10 frames, starting 200 - 6 actCnt + ccRandF(20) out:
//!                 turned a little more each frame (0.072 cos(radCnt += 1638)
//!                 about x, y and z), falling in by 18 sin(radCnt)
//!     3 (spilling) 30 frames: out along x at speed 30 less sin(radCnt +=
//!                 582) each frame, 4 down a frame, scale 5 shrinking to 2
//!     4 (burst)   120 frames: out along x by 8 sin(radCnt += 758),
//!                 ccRandF(1.2) along z
//!   fade: every fourth frame the colour's r, g and b down by one (from
//!   0x80); the pattern steps and wraps; after its life the spark is free
//! ```
//!
//! The portal draws nothing through `ccEffectCtrl`: it is an entry object
//! of its own, run by `ccThEntryCtrl` (priority 64). The runtime makes it
//! with [`MagicCircle::new`], runs `piney_world`'s `EntryObj::routine` on it
//! and then [`MagicCircle::main`] each frame, draws what that returns and
//! acts on its events.

use piney_desktop::layers::Layers;
use piney_world::draw::{self as wdraw, Draw};

use crate::draw::{Camera, DrawRec};
use crate::ee::{self, F, ONE, V4};
use crate::eff::Eff;
use crate::effect::{EFFECT_LAYER, check_camera_deg};
use crate::files::{Assets, ObjRef};
use crate::{Host, space, vu};
use piney_world::pose::Play;

/// `gimmickTbl[15]`: the file, the circle, its animations, the sparks and
/// the entry's palette.
pub const FILE: &str = "xmagcir";
pub const CLUMP: &str = "CMP_xmagcir0";
pub const IDLE: &str = "ANM_xmagcir1";
pub const OPENING: &str = "ANM_xmagcir2";
pub const SPARK: &str = "EFF_xmagpat1";
pub const CLUT: &str = "CLT_x031c2";
pub const CLUT_MATERIAL: &str = "MAT_clut";
/// `ccMagicCircle::mcpart[128]`.
pub const PARTS: usize = 128;
/// `ccSeOn3D`: 216 and 217 as the portal wakes, 215 as it opens.
pub const SE_WAKE: [i32; 2] = [216, 217];
pub const SE_OPEN: i32 = 215;
/// Kite wakes the portal within 3000 (0x453b8000); it draws within 7000.
const WAKE_DIST: F = 0x453b_8000;
const DRAW_DIST: F = 0x45da_c000;
const FADE_LEN: F = 0x4416_0000;
const VIEW_CONE: i16 = 12288;

/// What the portal asks of the rest of the game.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CircleEvent {
    /// `ccSeOn3D(se, pos)`.
    Sound3d { se: i32, pos: V4 },
    /// `ccEntryCtrl::entryCircleObject(this)` (gcmn 0x00430360): what the
    /// portal holds (foes, an item) comes out.
    EntryObject,
    /// Act 4 over with `entParam.entRoot` set (gcmn 0x00455e94):
    /// `saveData` +0x6864 (portals opened) up by one, at most 10000; and
    /// when the entry control's circle count (`g_entCtrl` +0x1c) is 1,
    /// `ccStartThread(ccThDfComp, 33, 4096)` and, in a field (`game` +0x14
    /// 1), +0x6866 up by one, in a dungeon (2) +0x6866 when
    /// `WORLD_MAN::GetFieldType()` is 4 and `game` +0x28 is 0, else +0x6868.
    Opened,
}

/// What `ccEntryObj::routine` and the world leave for the portal's `main`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CircleInput {
    /// `ccEntryObj` +0xe0 bit 2 `freezeFlag` (Kite beyond 10000) and bit 4
    /// `dispSW` (within 7000), +0xe4 `plDist`.
    pub freeze: bool,
    pub disp_sw: bool,
    pub pl_dist: F,
    /// `ccChar` +0x8c `setTransparency`.
    pub set_transparency: F,
    /// `ccCheckTarget(plw)`: Kite on the command lists.
    pub player_listed: bool,
    /// `entParam.entRoot` (+0x140).
    pub ent_root: i32,
}

/// A `ccMcPart` (0x70 bytes): one spark.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct McPart {
    /// +0x00 bit 0 `anmFlag`: its life is over.
    pub anm_flag: bool,
    /// +0x04 `status`: its kind (1-4), 0 free.
    pub status: i32,
    /// +0x10 `mat`: where it started (the portal's position as the
    /// translation), then its random turn.
    pub mat: vu::M4,
    /// +0x50 `speed`, +0x54 `radCnt`.
    pub speed: F,
    pub rad_cnt: F,
    /// +0x58 `mcActCnt`: the portal's `actCnt` when made.
    pub mc_act_cnt: i16,
    /// +0x5a `effAnmPat`, +0x5c `cnt`, +0x5e `life`.
    pub eff_anm_pat: u16,
    pub cnt: i16,
    pub life: i16,
    /// +0x60 `transparency`, +0x64 `rnd`.
    pub transparency: F,
    pub rnd: F,
    /// +0x68 `eff`: made and freed with the rest.
    pub eff: Option<Box<Eff>>,
}

/// `ccRandF(x)` (main 0x001d9a90): `x * r / 2^31` in double precision
/// (`fptodp`, `litodp`, `dpdiv`, `dpmul`, `dptofp`), `r` the next
/// `ccRand()` (`genrand` as a signed int).
pub fn rand_f(host: &mut dyn Host, x: F) -> F {
    let r = f64::from(host.genrand() as i32);
    ((f64::from(f32::from_bits(x)) * (r / 2_147_483_648.0)) as f32).to_bits()
}

/// `DEG2RAD((short)ccRand())`.
fn rand_angle(host: &mut dyn Host) -> F {
    ee::deg2rad(host.genrand() as i16)
}

/// `ccGetCameraTransparency(pos, 0, 0, far, fade)` (main 0x001da880) of a
/// thing with no size: 0 at 0.8 of 180 from the active camera's eye
/// rising to 1 at 1.25 of it, 1 out to `far`, then down to 0 over `fade`
/// (distances through `W2PPos`).
pub fn camera_transparency(pos: V4, eye: V4, player: V4, bounds: [F; 4], far: F, fade: F) -> F {
    let p = space::w2p(pos, player, bounds);
    let c = space::w2p(eye, player, bounds);
    let mut d = ee::vsub(c, p);
    d[3] = ONE;
    let dist = ee::sqrtf(ee::dot(d, d));
    // r = width t + height (1 - t) is 0 here, so 180 (and t, from the
    // camera's pitch, drops out).
    let r: F = 0x4334_0000;
    let r2 = ee::mul(0x3fa0_0000, r);
    if ee::lt(dist, r2) {
        let near = ee::mul(0x3f4c_cccd, r);
        let f = ee::sub(dist, near);
        return if ee::le(f, 0) { 0 } else { ee::div(f, ee::sub(r2, near)) };
    }
    if ee::le(far, 0) || ee::le(dist, far) {
        return ONE;
    }
    let over = ee::sub(dist, far);
    if ee::le(over, fade) { ee::div(ee::sub(fade, over), fade) } else { 0 }
}

/// The portal.
#[derive(Clone, Debug)]
pub struct MagicCircle {
    /// Its file in [`Assets`] ([`Assets::add_file`]), the circle and its
    /// animations, the spark's Eff chunk.
    file: usize,
    clump: ObjRef,
    spark: (usize, ObjRef),
    /// The animation the ccAnm plays (`SetAnm`'s chunk).
    anim: ObjRef,
    /// `ccEntryChangeCLUT`'s swap on the clump: (the palette `MAT_clut`'s
    /// texture draws with, `CLT_x031c2`).
    swaps: Vec<(u32, u32)>,
    /// +0x40 `pos`, +0x50 `posP`, +0x60 `dirc`.
    pub pos: V4,
    pub pos_p: V4,
    pub dirc: V4,
    /// +0x1d4 `actNum`, +0x1d8 `actCnt`, +0x1dc `anmFlag` (the animation's
    /// last step ended it).
    pub act: i32,
    pub act_cnt: i32,
    pub anm_flag: bool,
    /// +0xe0 bit 5 `destFlag` (set as it opens), bit 4 `dispSW` as `main`
    /// left it.
    pub dest_flag: bool,
    pub disp_sw: bool,
    /// +0x1e0 `partFlag` (the sparks' ccEffs exist), +0x1e4 `partTop`.
    pub part_flag: bool,
    pub part_top: i32,
    pub parts: Vec<McPart>,
    /// +0xd4 the ccAnm: its playback, the matrix `SetMatrix_PosRotZYX` last
    /// gave it, its transparency (+0x88).
    pub play: Play,
    pub matrix: vu::M4,
    pub transparency: F,
}

/// What a frame of the portal did.
#[derive(Clone, Debug, Default)]
pub struct CircleFrame {
    /// `main`'s result: the entry control deletes the portal.
    pub delete: bool,
    pub events: Vec<CircleEvent>,
    /// The circle (`DrawRec::Anm`) and the sparks, in the order sent.
    pub draws: Vec<DrawRec>,
}

impl MagicCircle {
    /// `ccMagicCircle::ccMagicCircle` (gcmn 0x00455900) with the entry's
    /// `pos` (already on the ground, 250 up) and `dirc`; `player` and
    /// `bounds` for `posP`. None when `XMAGCIR.CCS` is not in `assets`.
    pub fn new(assets: &Assets, pos: V4, dirc: V4, player: V4, bounds: [F; 4]) -> Option<MagicCircle> {
        let file = assets.file_index(FILE)?;
        let f = &assets.files[file];
        let clump = ObjRef { file, object: f.ccs.find_object(CLUMP)? };
        let spark_obj = ObjRef { file, object: f.ccs.find_object(SPARK)? };
        let (j, _) = assets.eff_chunk(spark_obj)?;
        let play = Play::new(f, IDLE)?;
        let anim = ObjRef { file, object: f.ccs.find_object(IDLE)? };
        let swaps = clut_swaps(f).into_iter().collect();
        Some(MagicCircle {
            file,
            clump,
            spark: (j, spark_obj),
            anim,
            swaps,
            pos,
            pos_p: space::w2p(pos, player, bounds),
            dirc,
            act: 0,
            act_cnt: 0,
            anm_flag: false,
            dest_flag: false,
            disp_sw: true,
            part_flag: false,
            part_top: 0,
            parts: vec![McPart::default(); PARTS],
            play,
            matrix: [[0; 4]; 4],
            transparency: ONE,
        })
    }

    /// The circle's clump (for the draw), and the palette swap
    /// `ccEntryChangeCLUT` makes on it: `MAT_clut`'s texture's palette for
    /// `CLT_x031c2`.
    pub fn clump(&self) -> ObjRef {
        self.clump
    }

    /// `ccMagicCircle::setPart(ptype, lmat, num)` (gcmn 0x00455510).
    fn set_part(&mut self, ptype: i32, lmat: &vu::M4, num: i32) {
        for _ in 0..num {
            let found = (0..PARTS as i32)
                .map(|v| (self.part_top + v) % PARTS as i32)
                .find(|&k| self.parts[k as usize].status == 0);
            let k = match found {
                Some(k) => k as usize,
                None => {
                    let k = self.part_top as usize;
                    self.part_top += 1;
                    if self.part_top >= PARTS as i32 {
                        self.part_top -= PARTS as i32;
                    }
                    k
                }
            };
            let act_cnt = self.act_cnt;
            let p = &mut self.parts[k];
            p.status = ptype;
            p.anm_flag = false;
            p.rad_cnt = 0;
            p.cnt = 0;
            p.eff_anm_pat = 0;
            p.transparency = ONE;
            p.mat = *lmat;
            p.mc_act_cnt = act_cnt as i16;
            if let Some(eff) = &mut p.eff {
                eff.pos = [0, 0, 0, ONE];
                eff.rotate = 0;
                eff.color = 0x0080_8080;
            }
        }
    }

    /// `ccMagicCircle::createPart()` (gcmn 0x00455710).
    fn create_part(&mut self) {
        let mut m = vu::UNIT;
        m[3] = self.pos;
        match self.act {
            0 => self.set_part(1, &m, 3),
            1 => self.set_part(2, &m, (self.act_cnt >> 2) + 1),
            2 => self.set_part(4, &m, 8),
            3 if self.act_cnt < 16 => self.set_part(3, &m, 2),
            _ => {}
        }
    }

    /// `ccMagicCircle::main()` (gcmn 0x00455b60).
    pub fn main(&mut self, assets: &Assets, host: &mut dyn Host, input: &CircleInput) -> CircleFrame {
        let mut out = CircleFrame::default();
        self.disp_sw = input.disp_sw;
        if self.act == 0 && input.freeze {
            if self.part_flag {
                for p in &mut self.parts {
                    p.eff = None;
                }
                self.part_flag = false;
            }
            return out;
        }
        if !self.part_flag {
            let (j, obj) = self.spark;
            let c = &assets.effs[obj.file][j];
            for p in &mut self.parts {
                p.status = 0;
                p.eff = Some(Box::new(Eff::init(obj.file, j, c, false, &assets.alpha_blend)));
            }
            self.part_flag = true;
            self.part_top = 0;
        }
        let (player, bounds) = (host.player_pos(), host.bounds());
        self.pos_p = space::w2p(self.pos, player, bounds);
        self.pos = space::p2w(self.pos_p, player, bounds);
        let file = &assets.files[self.file];
        match self.act {
            0 => {
                if input.player_listed && ee::le(input.pl_dist, WAKE_DIST) {
                    for se in SE_WAKE {
                        out.events.push(CircleEvent::Sound3d { se, pos: self.pos });
                    }
                    self.act = 1;
                    self.act_cnt = 0;
                }
            }
            1 => {
                let old = self.act_cnt;
                self.act_cnt += 1;
                if old >= 31 {
                    if let (Some(p), Some(o)) = (Play::new(file, OPENING), file.ccs.find_object(OPENING)) {
                        self.play = p;
                        self.anim = ObjRef { file: self.file, object: o };
                    }
                    self.act = 2;
                    self.act_cnt = 0;
                    self.dest_flag = true;
                }
            }
            2 => {
                let old = self.act_cnt;
                self.act_cnt += 1;
                if old >= 21 {
                    out.events.push(CircleEvent::Sound3d { se: SE_OPEN, pos: self.pos });
                    out.events.push(CircleEvent::EntryObject);
                    self.act = 3;
                    self.act_cnt = 0;
                }
            }
            3 => {
                self.act_cnt += 1;
                if self.anm_flag {
                    self.act = 4;
                    self.act_cnt = 0;
                }
            }
            4 => {
                let old = self.act_cnt;
                self.act_cnt += 1;
                if old >= 65 {
                    if input.ent_root != 0 {
                        out.events.push(CircleEvent::Opened);
                    }
                    out.delete = true;
                    return out;
                }
            }
            _ => {}
        }
        self.create_part();
        for p in &mut self.parts {
            if p.status != 0 {
                part_main(p, host, player, bounds);
            }
        }
        if self.act == 0 && !ee::lt(input.pl_dist, DRAW_DIST) {
            return out;
        }
        self.anm_flag = self.play.forward(file);
        if !self.disp_sw {
            return out;
        }
        let cam = host.camera();
        self.disp_sw = check_camera_deg(self.pos, player, bounds, cam.cam_pos, cam.cam_view, VIEW_CONE);
        if !self.disp_sw {
            return out;
        }
        let mut t = ee::mul(
            camera_transparency(self.pos, cam.cam_pos, player, bounds, DRAW_DIST, FADE_LEN),
            input.set_transparency,
        );
        self.transparency = t;
        self.matrix = vu::trans(&vu::rot_zyx(&vu::UNIT, self.dirc), self.pos);
        out.draws.push(DrawRec::Anm {
            obj: self.anim,
            play: self.play.clone(),
            matrix: self.matrix,
            alpha: t,
            layer: EFFECT_LAYER,
        });
        for p in &mut self.parts {
            if p.status == 0 {
                continue;
            }
            t = ee::mul(t, p.transparency);
            if !ee::le(t, ONE) {
                t = ONE;
            }
            if ee::lt(t, 0) {
                t = 0;
            }
            if let Some(eff) = &mut p.eff {
                eff.transparency = t;
                out.draws.push(DrawRec::Eff { eff: eff.clone(), pat: p.eff_anm_pat, layer: EFFECT_LAYER });
            }
        }
        out
    }
}

/// `ccEntryChangeCLUT(ent, clump)` (gcmn 0x0042e670) on the circle:
/// `ccClump::ChangeClut(CLT_x031c2, MAT_clut)`, as a swap of the palette
/// `MAT_clut`'s texture uses (every texture on that palette takes it).
fn clut_swaps(f: &piney_desktop::assets::SceneFile) -> Option<(u32, u32)> {
    let mat = f.ccs.find_object(CLUT_MATERIAL)?;
    let tex = f.scene.materials.get(&mat)?.texture;
    let (textures, _) = piney_data::texture::read(&f.ccs).ok()?;
    let from = textures.iter().find(|t| t.object == tex)?.clut;
    Some((from, f.ccs.find_object(CLUT)?))
}

impl MagicCircle {
    /// A frame's draws into `layers` through `camera`'s view: the circle
    /// with its palette swap (`ccAnm::Draw`: each animated object's model
    /// at its matrix and at its own transparency, the pose's times the
    /// portal's, none at or below 1/128: [`crate::nodes::anm`]), the sparks
    /// as sprites. The opening (`ANM_xmagcir2`) fades every object out by
    /// its last frame, so the portal is gone from sight while act 4 still
    /// counts.
    pub fn render(&self, frame: &CircleFrame, assets: &Assets, layers: &mut Layers, camera: &Camera) {
        let to_screen = wdraw::screen(&camera.world_screen);
        for d in &frame.draws {
            let DrawRec::Anm { obj, play, matrix, alpha, layer } = d else {
                d.render(assets, layers, camera);
                continue;
            };
            let file = &assets.files[obj.file];
            let morphers = &assets.morphers[obj.file];
            let rows = play.uv_rows(file);
            let morph = play.morph(file);
            for o in crate::nodes::anm(assets, *obj, play, matrix, *alpha) {
                let model = o.model;
                let morph: Vec<(u32, f32)> = morph
                    .iter()
                    .filter(|(mph, _)| morphers.get(mph) == Some(&model))
                    .flat_map(|(_, t)| t.iter().copied())
                    .collect();
                let draw = Draw {
                    file,
                    model,
                    world: wdraw::mat(&o.world),
                    alpha: ee::f(o.alpha),
                    rows: &rows,
                    lights: None,
                    nodes: &[],
                    morph,
                    clut_swaps: self.swaps.clone(),
                };
                wdraw::model(layers, *layer, to_screen, draw);
            }
        }
    }
}

/// The spark's translation: `mat`'s last column with w 1.
fn origin(m: &vu::M4) -> V4 {
    [m[3][0], m[3][1], m[3][2], ONE]
}

/// Three `ccRand` turns about x, y and z of the unit matrix.
fn random_turn(host: &mut dyn Host) -> vu::M4 {
    let m = vu::rot_x(&vu::UNIT, rand_angle(host));
    let m = vu::rot_y(&m, rand_angle(host));
    vu::rot_z(&m, rand_angle(host))
}

/// `radCnt` on by `step` in the game's 16-bit angles.
fn turn(rad: F, step: i16) -> F {
    ee::deg2rad(ee::rad2deg(rad).wrapping_add(step))
}

/// `scale -= by` while above `floor`, x then y.
fn shrink(eff: &mut Eff, floor: F, by: F) {
    if !ee::le(eff.scale_x, floor) {
        eff.scale_x = ee::sub(eff.scale_x, by);
    }
    if !ee::le(eff.scale_y, floor) {
        eff.scale_y = ee::sub(eff.scale_y, by);
    }
}

/// `ccMcPart::main()` (gcmn 0x004548e0) of a live spark.
fn part_main(p: &mut McPart, host: &mut dyn Host, player: V4, bounds: [F; 4]) {
    const FIVE: F = 0x40a0_0000;
    let Some(eff) = p.eff.as_deref_mut() else { return };
    eff.pos = space::fw2lw(eff.pos, player, bounds);
    let first = p.cnt == 0;
    match p.status {
        1 => {
            if first {
                p.life = 120;
                eff.scale_y = FIVE;
                eff.scale_x = FIVE;
                eff.pos = origin(&p.mat);
                p.mat = random_turn(host);
                p.rnd = rand_f(host, 0x3e4c_cccd);
                if ee::lt(p.rnd, 0) {
                    p.rnd = ee::mul(p.rnd, 0xbf80_0000);
                }
            } else {
                p.rad_cnt = turn(p.rad_cnt, 838);
                p.speed = ee::mul(0x40c0_0000, ee::sinf(p.rad_cnt));
                let v = ee::apply(&p.mat, [p.speed, 0, 0, ONE]);
                eff.pos = ee::vadd(eff.pos, v);
                let b = ee::deg2rad((i32::from(p.cnt) * 1383) as i16);
                let v = [0, 0, ee::mul(0x40c0_0000, ee::sinf(b)), 0];
                eff.pos = ee::vadd(eff.pos, v);
                shrink(eff, ONE, 0x3d23_d70a);
            }
        }
        2 => {
            if first {
                p.life = 10;
                eff.scale_y = FIVE;
                eff.scale_x = FIVE;
                let o = origin(&p.mat);
                let m = random_turn(host);
                let r = rand_f(host, 0x41a0_0000);
                let x = ee::add(ee::sub(0x43c8_0000, ee::mul(0x40c0_0000, ee::from_int(i32::from(p.mc_act_cnt)))), r);
                let v = ee::apply(&m, [x, 0, 0, ONE]);
                eff.pos = ee::vadd(o, v);
                p.mat = m;
            } else {
                p.rad_cnt = turn(p.rad_cnt, 1638);
                p.speed = ee::mul(0x41a0_0000, ee::mul(0x3f66_6666, ee::sinf(p.rad_cnt)));
                let v = [ee::neg(p.speed), 0, 0, ONE];
                p.speed = ee::mul(0x3d93_74bc, ee::cosf(p.rad_cnt));
                if !ee::le(p.speed, 0x4049_0fdb) {
                    p.speed = ee::sub(p.speed, 0x40c9_0fdb);
                }
                if ee::lt(p.speed, 0xc049_0fdb) {
                    p.speed = ee::add(p.speed, 0x40c9_0fdb);
                }
                let m = vu::rot_x(&p.mat, p.speed);
                let m = vu::rot_y(&m, p.speed);
                let m = vu::rot_z(&m, p.speed);
                p.mat = m;
                let v = ee::apply(&m, v);
                eff.pos = ee::vadd(eff.pos, v);
                shrink(eff, ONE, 0x3e8f_5c29);
            }
        }
        3 => {
            if first {
                p.life = 30;
                eff.scale_y = FIVE;
                eff.scale_x = FIVE;
                p.speed = 0x41f0_0000;
                eff.pos = origin(&p.mat);
                p.mat = random_turn(host);
            } else {
                p.rad_cnt = turn(p.rad_cnt, 582);
                p.speed = ee::sub(p.speed, ee::sinf(p.rad_cnt));
                let v = ee::apply(&p.mat, [p.speed, 0, 0, 0]);
                eff.pos = ee::vadd(eff.pos, v);
                eff.pos = ee::vadd(eff.pos, [0, 0, 0xc080_0000, 0]);
                shrink(eff, 0x4000_0000, 0x3d75_c28f);
            }
        }
        4 => {
            if first {
                p.life = 120;
                eff.scale_x = FIVE;
                eff.scale_y = FIVE;
                eff.pos = origin(&p.mat);
                p.mat = random_turn(host);
            } else {
                p.rad_cnt = turn(p.rad_cnt, 758);
                p.speed = ee::mul(0x4100_0000, ee::sinf(p.rad_cnt));
                let z = rand_f(host, 0x3f99_999a);
                let v = ee::apply(&p.mat, [p.speed, 0, z, ONE]);
                eff.pos = ee::vadd(eff.pos, v);
                shrink(eff, ONE, 0x3cf5_c28f);
            }
        }
        _ => {}
    }
    fade(p);
    let old = p.cnt;
    p.cnt = p.cnt.wrapping_add(1);
    if p.life < old {
        p.anm_flag = true;
    }
    let pat_num = p.eff.as_ref().map_or(0, |e| e.pat_num);
    p.eff_anm_pat = p.eff_anm_pat.wrapping_add(1);
    if i32::from(p.eff_anm_pat) >= i32::from(pat_num) {
        p.eff_anm_pat = 0;
    }
    if p.anm_flag {
        p.status = 0;
    }
}

/// `ccMcPart::fade()` (gcmn 0x00454840): every fourth frame the colour's
/// r, g and b down by one (not below 0); its alpha byte up by one every
/// frame, wrapping.
fn fade(p: &mut McPart) {
    let cnt = p.cnt;
    let Some(eff) = p.eff.as_deref_mut() else { return };
    let c = eff.color;
    let (mut r, mut g, mut b, a) = (c & 0xff, (c >> 8) & 0xff, (c >> 16) & 0xff, c >> 24);
    if cnt & 3 == 0 {
        r = r.saturating_sub(1);
        g = g.saturating_sub(1);
        b = b.saturating_sub(1);
    }
    let a = (a + 1).wrapping_shl(24);
    eff.color = a | (b << 16) | (g << 8) | r;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ee::k;
    use crate::host::Simple;

    #[test]
    fn the_portal_wakes_opens_and_goes() {
        const ISO: &str = "../../work/infection/infection.iso";
        if !std::path::Path::new(ISO).exists() {
            return;
        }
        let mut iso = piney_data::iso::Iso::open(ISO).unwrap();
        let archive = piney_data::archive::Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap();
        let mut fx = crate::Effects::new(&archive, iso.volume().unwrap()).unwrap();
        fx.assets.add_file(&archive, FILE).unwrap();
        let mut r = || 0;
        let eye = [0, k(-1200.0), k(500.0), ONE];
        let cam = Camera { eye, cam_pos: eye, cam_view: [0, 0, k(250.0), ONE], ..Camera::default() };
        let mut host = Simple::new(&mut r, [0, 0, 0, ONE], cam);
        let mut c =
            MagicCircle::new(&fx.assets, [0, k(1000.0), k(250.0), ONE], [0; 4], [0, 0, 0, ONE], space::TOWN_BOUNDS)
                .unwrap();
        assert!(!c.swaps.is_empty());
        let input = CircleInput {
            freeze: false,
            disp_sw: true,
            pl_dist: k(1000.0),
            set_transparency: ONE,
            player_listed: true,
            ent_root: 1,
        };
        let (mut events, mut frames, mut sparks) = (Vec::new(), 0, 0);
        loop {
            let out = c.main(&fx.assets, &mut host, &input);
            events.extend(out.events);
            sparks = sparks.max(out.draws.iter().filter(|d| matches!(d, DrawRec::Eff { .. })).count());
            frames += 1;
            if out.delete || frames > 1000 {
                break;
            }
        }
        let pos = c.pos;
        assert_eq!(
            events,
            [
                CircleEvent::Sound3d { se: 216, pos },
                CircleEvent::Sound3d { se: 217, pos },
                CircleEvent::Sound3d { se: SE_OPEN, pos },
                CircleEvent::EntryObject,
                CircleEvent::Opened
            ]
        );
        assert!(frames < 1000 && sparks > 50, "{frames} frames, {sparks} sparks");
    }

    /// `ccAnm::Draw` of the opening (`ANM_xmagcir2`, stepped 256 a frame
    /// as `ccMagicCircle::main` steps it): each object at its own
    /// transparency, so the ball flares and goes, the rings grow and
    /// shrink away and the sphere fades; from the opening's last frame on
    /// (act 3's end, all of act 4) nothing of the portal is drawn.
    #[test]
    fn the_opening_fades_the_portal_out() {
        const ISO: &str = "../../work/infection/infection.iso";
        if !std::path::Path::new(ISO).exists() {
            return;
        }
        let mut iso = piney_data::iso::Iso::open(ISO).unwrap();
        let archive = piney_data::archive::Archive::new(iso.read_path("DATA/DATA.BIN").unwrap()).unwrap();
        let mut fx = crate::Effects::new(&archive, iso.volume().unwrap()).unwrap();
        let k = fx.assets.add_file(&archive, FILE).unwrap();
        let file = &fx.assets.files[k];
        let obj = ObjRef { file: k, object: file.ccs.find_object(CLUMP).unwrap() };
        let mut play = Play::new(file, OPENING).unwrap();
        let name = |o: u32| file.ccs.object_name(o).unwrap_or("").to_string();
        let mut drawn = Vec::new();
        for f in 0..180 {
            play.forward(file);
            let d = crate::nodes::anm(&fx.assets, obj, &play, &vu::UNIT, ONE);
            drawn.push((f, d.iter().map(|d| name(d.obj)).collect::<Vec<_>>()));
        }
        assert!(drawn[29].1.iter().any(|n| n == "OBJ_xmagball"), "the ball at its height: {:?}", drawn[29]);
        assert!(!drawn[70].1.iter().any(|n| n == "OBJ_xmagball"), "the ball gone: {:?}", drawn[70]);
        let last = drawn.iter().filter(|(_, d)| !d.is_empty()).map(|(f, _)| *f).max().unwrap();
        assert!((100..111).contains(&last), "the last frame drawn: {last}, {:?}", &drawn[95..115]);
    }
}
