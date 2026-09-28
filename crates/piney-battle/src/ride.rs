//! The riding Grunty: `ccPucciguso` (gcmn `pgrider.cpp`,
//! 0x00510880-0x005130ac), the adult Grunty the Grunty Flute calls in a
//! field, Kite on its back.
//!
//! - [`adult_check`]: `ccPgAdultCheck(server, slot)` (0x00510650, the end
//!   of `pgbreed.cpp`), which grown Grunty the flute can call from a
//!   server's record.
//! - [`Ride::new`]: the constructor (0x00510f30): Kite's riding clump
//!   (`ctu1body`'s `CMP_trall`) and the Grunty's (`cdogbodN`), both on
//!   act 2, where Kite stands.
//! - [`main`]: `ccPucciguso::Main()` (0x005114e0), a frame of the ride on
//!   its task (`ccThPucciguso`, priority 49): the stick
//!   ([`control_move`] 0x005119c0, [`pad_lever_power`] 0x00511ed0), the
//!   push out of the bodies and the walls, the ground and a dungeon's way
//!   in ([`collision_test`] 0x00512360), the field's wrap
//!   ([`map_loop_adjust_pos`] 0x005123f0), Kite's place (`plw`), the
//!   camera ([`camera_pos_calc`] 0x005124b0, [`camera_pos_set`]
//!   0x00512540), the acts ([`anim_ctrl`] 0x00511f80, the Grunty's notes
//!   [`check_note`] 0x00512fb0), the draws (`ccChar::Draw` for Kite,
//!   [`draw_pg`] 0x00512c80 for the Grunty) and the dust
//!   ([`paw_smoke`] 0x00512670).
//! - [`exit_place`]: where `ccPuccigusoExit` (0x00510bd0) puts each party
//!   member when the ride ends (`puccigusoAngleTbl`).
//!
//! What happens around it - `ccPuccigusoStart` (0x005109c0) on the menu
//! task, `ccThPucciguso` (0x00510880) and `ccPuccigusoExit` with their
//! fades, the party asleep (`ccSpcSleep`, `ccSpcWakeup`), the music
//! (`ccPgBgmInit`, `ccPgBgmEnd`) - is the runtime's (piney-world's
//! `combat::ride`, piney-game's area); `docs/engine/grunty-ride.md` has the
//! whole.
//!
//! # State
//!
//! A [`Ride`] is the object's members (0x1d0 bytes over `ccChar`): its
//! place, heading, flag byte (+0xe0), acts, speeds, the camera's points,
//! the move and its smoothed copy, the body (`ccCharHit` +0x170). Every
//! world call goes through [`RideWorld`] where and as often as the game
//! makes it; what only shows or sounds is an [`Out`]. The globals it reads
//! and writes are [`Globals`] (`pgR`, `pgDIN`) and [`Input`].

use crate::damage::fptosi;
use crate::geom::{self, F, MINUS_ONE, ONE, PI, V4, VF0, add, cosf, div, from_int, le, lt, mul, neg, sinf, sqrtf, sub};
use crate::kite::{self, MapBounds, Pad};
use crate::rand::Rng;
use crate::world::{CharHit, Note};

/// Acts, kinds, registry slots.
pub const ACTS: usize = 7;
pub const KINDS: usize = 9;
pub const SLOTS: usize = 5;

/// The acts (+0xe2), each a clip of both tables.
pub mod act {
    /// Standing (`dgn0` / `nut0`), its fidget (`dgn1` / `nut1`) twice, the
    /// second time into act 4 (standing again).
    pub const IDLE0: i16 = 0;
    pub const FIDGET0: i16 = 1;
    pub const IDLE: i16 = 2;
    pub const FIDGET: i16 = 3;
    pub const IDLE_AFTER: i16 = 4;
    /// Running (`dgr0` / `run0`) and walking (`dgw0` / `wal0`).
    pub const RUN: i16 = 5;
    pub const WALK: i16 = 6;
}

/// Bits of the flag byte (+0xe0).
pub mod flag {
    /// `pauseSW`: `plw`'s, copied in and out each frame.
    pub const PAUSE: u8 = 1 << 0;
    /// The stick moves it.
    pub const MOVE: u8 = 1 << 2;
    /// Standing (set by the constructor).
    pub const STOP: u8 = 1 << 3;
    /// Leaning past 240: running.
    pub const RUN: u8 = 1 << 4;
    /// The eye view from the field camera: nothing drawn of Kite.
    pub const LOST_HEAD: u8 = 1 << 5;
}

/// The Grunty's legs whose places the dust starts from, by the bits of
/// [`paw_smoke`]'s mask, and the puff's life.
pub const LEGS: [(i32, &str, i32); 4] =
    [(1, "OBJ_t0 fl leg2", 35), (2, "OBJ_t0 fr leg2", 35), (4, "OBJ_t0 rl leg2", 37), (8, "OBJ_t0 rr leg2", 37)];

/// `ccLandHitCheck`'s mask, the walls' (`ccHitCheckLM2`, the body's
/// `mask2`) and the body's kind.
pub const LAND: u32 = 0x2000_0001;
pub const WALL: u32 = 0x4000_0001;
pub const BODY_KIND: u32 = 0x0100_0000;

const K2: F = 0x4000_0000;
const K3: F = 0x4040_0000;
const K10: F = 0x4120_0000;
const K100: F = 0x42c8_0000;
const K140: F = 0x430c_0000;
const K150: F = 0x4316_0000;
const K190: F = 0x433e_0000;
const K240: F = 0x4370_0000;
const K250: F = 0x437a_0000;
const K255: F = 0x437f_0000;
const K256: F = 0x4380_0000;
const HALF: F = 0x3f00_0000;
/// 1.3, the walk's most; 6.2 its speed at 1.0.
const WALK_MAX: F = 0x3fa6_6666;
const WALK_SPEED: F = 0x40c6_6666;
/// The move's smoothing: 0.08 of the way a frame while leaning, 0.125
/// while not.
const EASE_ON: F = 0x3da3_d70a;
const EASE_OFF: F = 0x3e00_0000;
/// 0.075 and 0.1: the dust's speed.
const K0075: F = 0x3d99_999a;
const K01: F = 0x3dcc_cccd;
/// 0.05: under it (and not hidden by the camera's nearness) not drawn.
const K005: F = 0x3d4c_cccd;
/// 60.0: the run's speed at full lean (+0xec).
pub const SPEED: F = 0x4270_0000;
/// `pcgsTbl` (gcmn 0x005ee810, the object's `ccCharBaseParam`): +0x18
/// height and +0x1c width, 120.
pub const SIZE: F = 0x42f0_0000;

/// What [`main`] reads of the disc.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RideTables {
    /// `puccigusoAnimTbl` (INF gcmn 0x006aea50): Kite's clip for each act,
    /// `ANM_ctu1dgn0` ...
    pub anims: Vec<String>,
    /// `puccigusoAnimTblPG` (INF gcmn 0x005ee870) as on the disc:
    /// `ANM_cdg0nut0` ...; its eighth character is the kind's digit (the
    /// constructor writes it).
    pub anims_pg: Vec<String>,
    /// `puccigusoCharTbl` (INF gcmn 0x005ee840): each kind's file,
    /// `cdogbod0`, `cdogbod2` ... `cdogbod9`.
    pub files: Vec<String>,
    /// `puccigusoAngleTbl` (INF gcmn 0x005ee908): where each registry
    /// slot's member stands after the ride, in 16-bit angle units off the
    /// heading.
    pub angles: [i16; SLOTS],
}

impl RideTables {
    /// The volume's (`tables::combat`).
    pub fn of(volume: piney_data::volume::Volume) -> RideTables {
        let t = piney_data::tables::combat::of(volume);
        let owned = |v: &[&str]| v.iter().map(|s| s.to_string()).collect();
        let mut angles = [0; SLOTS];
        angles.copy_from_slice(&t.ride_angles()[..SLOTS]);
        RideTables {
            anims: owned(t.ride_anims()),
            anims_pg: owned(t.ride_anims_pg()),
            files: owned(t.ride_files()),
            angles,
        }
    }

    /// Kite's clip for `act`.
    pub fn anim(&self, act: i16) -> &str {
        usize::try_from(act).ok().and_then(|i| self.anims.get(i)).map_or("", |s| s.as_str())
    }

    /// The Grunty's clip for `act` as the constructor leaves the table for
    /// `kind`: the eighth character `'0'` for kind 0, `'0' + kind + 1`
    /// for the others (`cdg2` ... `cdg9`; `cdg1` is the young one's).
    pub fn anim_pg(&self, act: i16, kind: i32) -> String {
        let s = usize::try_from(act).ok().and_then(|i| self.anims_pg.get(i)).cloned().unwrap_or_default();
        let mut b = s.into_bytes();
        if b.len() > 7 {
            let d = if kind > 0 { kind + 1 } else { kind };
            b[7] = (d + 48) as u8;
        }
        String::from_utf8_lossy(&b).into_owned()
    }

    /// The kind's file (`puccigusoCharTbl[kind]`).
    pub fn file(&self, kind: i32) -> &str {
        usize::try_from(kind).ok().and_then(|i| self.files.get(i)).map_or("", |s| s.as_str())
    }
}

/// `saveData.growth[5]` (+0x2194, 0x18 bytes a server): `short type[3]` at
/// +0xe, a grown Grunty in each of the three pens.
pub const SAVE_GROWTH: usize = 0x2194;

/// `ccPgAdultCheck(server, slot)` (gcmn 0x00510650): the kind of the grown
/// Grunty in pen `slot` of `server`'s record (`growth[server].type[slot]`
/// set): 0 in pen 0 whatever the server, else `2 server + slot - 2` (1-8);
/// -1 for an empty pen, server 0 or a slot of 3 and more; -146 for a
/// server outside 1-4, or a negative slot whose word is set (the game's
/// -1 less 145).
pub fn adult_check(save: &piney_data::save::SaveData, server: i32, slot: i32) -> i32 {
    if server == 0 || slot >= 3 {
        return -1;
    }
    if !(1..=4).contains(&server) {
        return -146;
    }
    let at = SAVE_GROWTH as i64 + 24 * i64::from(server) + 0xe + 2 * i64::from(slot);
    if usize::try_from(at).map_or(0, |a| save.i16(a)) == 0 {
        return -1;
    }
    match slot {
        0 => 0,
        1 | 2 => 2 * server + slot - 2,
        _ => -146,
    }
}

/// Which of the ride's two players: Kite's (+0xd4 over `ctu1body`) or the
/// Grunty's (+0x1c8 over the kind's file).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RideAnm {
    Kite,
    Pg,
}

/// `ccPucciguso`'s members.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ride {
    /// +0x40 `pos`, +0x50 `posP` (only z written: the ground), +0x60 `rot`.
    pub pos: V4,
    pub pos_p: V4,
    pub rot: V4,
    /// +0x80 `hitAttribute`: `checkHitResultAttlibute()` after the ground
    /// test (the footsteps' ground and the shade read it).
    pub hit_attribute: u32,
    /// +0x88 `transparency`, +0x8c `setTransparency` (`ccChar`'s).
    pub transparency: F,
    pub char_set_transparency: F,
    /// +0xe0 ([`flag`]).
    pub flags: u8,
    /// +0xe2 `actNum`, +0xe4 `actNumOld`, +0xe6 the clips' end, +0xe8 the
    /// idle count (440 from the constructor: the first fidget comes 11
    /// frames in).
    pub act: i16,
    pub act_old: i16,
    pub anm_end: i16,
    pub idle: i16,
    /// +0xec the run's speed (60), +0xf0 `speedRate`, +0xf4 `nowSpeed`,
    /// +0xf8 its own `setTransparency` (1.0).
    pub speed: F,
    pub speed_rate: F,
    pub now_speed: F,
    pub set_transparency: F,
    /// +0x100 the kind, +0x104 the frame count (from `rand() >> 3`).
    pub kind: i32,
    pub cycle: i32,
    /// +0x110 `posView` (190 over it), +0x130 `posEye` (250 over it),
    /// +0x140 the eye view's `angle`.
    pub pos_view: V4,
    pub pos_eye: V4,
    pub angle: V4,
    /// +0x150 `movePos`, +0x160 the move eased toward it.
    pub move_pos: V4,
    pub move_ease: V4,
    /// +0x170 `bodyHit`: kind 0x1000000, `mask2` the walls, height 100,
    /// radius 100 plus `nowSpeed`.
    pub hit: CharHit,
    /// Both players' `frameSpd` (+0x9c) as [`anim_ctrl`] last set them.
    pub frame_spd: [u16; 2],
}

/// `pgR` (0x00378c28) and `pgDIN` (0x00378c2c): riding, and a dungeon's
/// way in stepped on while riding (the ride ends with the scene).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Globals {
    pub pg_r: i32,
    pub pg_din: i32,
}

/// What the frame reads of the rest of the game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// `ccSys->pad[0]`'s left stick.
    pub pad: Pad,
    /// `plw.pauseSW` (bit 0 of 0x007302e0).
    pub pause: bool,
    /// `dneFlag` (0x00378cd8).
    pub dne: bool,
    /// `WORLD_MAN` +0x420: the map's bounds.
    pub bounds: MapBounds,
    /// `game.area` (`game` +0x14): the Grunty's fade by the town's
    /// distances (4000 over 400) in area 0, else the field's (7000 over
    /// 600).
    pub area: i32,
}

/// What the ride hands the rest of the game, in the game's order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Out {
    /// `WORLD_MAN::AddCenter(x, y)`: the field's streaming follows it.
    AddCenter { x: F, y: F },
    /// `WORLD_MAN::Enter(pos)`: a way into a dungeon (`pgR` 0, `pgDIN` 1).
    Enter { pos: V4 },
    /// `anm->SetMatrix_PosRotZYX(pos, rot)` on a player.
    Matrix { anm: RideAnm, pos: V4, rot: V4 },
    /// `plw.pos` and `plw.rot` (Kite's own `pos`, `rot`: the ride carries
    /// him) and `plw.pauseSW`.
    Player { pos: V4, rot: V4, pause: bool },
    /// `ccSeSetParamInu(param, this)`: a footstep or a cry (the Grunty's
    /// notes 1 and 2) on the ground `attribute`.
    Sound { param: u32, attribute: u32 },
    /// `effSmoke(pos, v, s, life, t, 512, 32)`: a puff of dust at a leg.
    Smoke { pos: V4, v: V4, s: F, life: i32, t: i32 },
    /// `ccAnm::Draw()` of the Grunty (`DrawPG`) at `transparency`, the
    /// draw environment's alpha byte `alpha` (+0xa4, from `shadow`: the
    /// transparency, or `setTransparency` when the camera's nearness hides
    /// it) and `height` (+0xa0, the shadow's), `shaded` (ground attribute
    /// 0x40000: the ambient halved and the distant light off).
    DrawPg { transparency: F, shadow: F, alpha: u8, height: F, shaded: bool },
}

/// What the ride asks of the world. Each method is the game function it
/// names, called where and as often as the game calls it.
pub trait RideWorld {
    /// `ccLandHitCheck(pos, mask)` (gcmn 0x00571e00).
    fn land(&mut self, pos: V4, mask: u32) -> F;
    /// `checkHitResultAttlibute()` (main 0x00153e80).
    fn hit_attribute(&mut self) -> u32;
    /// `hitResultNum` and `hitResultNearest` +0x48: the attribute of the
    /// nearest polygon the last ground query hit, none when it hit nothing.
    fn hit_result(&mut self) -> Option<u32>;
    /// `ccHitCheckLM2(from, to, mask)` (main 0x00153900): -1.0 when
    /// nothing is in the way.
    fn line(&mut self, from: V4, to: V4, mask: u32) -> F;
    /// `ccCharHit::CollisionDetection()` (main 0x00153470) on the ride's
    /// body: `hit.offset` the push out of the others and the walls.
    fn collide(&mut self, hit: &mut CharHit) -> i32;
    /// `ccCharHit::SetHitSW(1)` / `HitDisable()`: the body into the list or
    /// out.
    fn hit_switch(&mut self, hit: &mut CharHit, on: bool);
    /// `checkCameraType()` / `activeCamPtr->type`: 1 the eye view.
    fn camera_type(&mut self) -> i32;
    /// `checkCameraID()`: 1 the field's camera.
    fn camera_id(&mut self) -> i32;
    /// `cameraGetRot(&v, camID)`.
    fn camera_rot(&mut self) -> V4;
    /// `activeCamPtr->resetFlag` and `resetDirc`.
    fn camera_reset(&mut self) -> Option<F>;
    /// `activeCamPtr->resetFlag = 0`.
    fn clear_camera_reset(&mut self);
    /// `cameraSetEyeLevel(posEye, angle)`: turns `angle`.
    fn camera_set_eye_level(&mut self, pos_eye: V4, angle: &mut V4);
    /// `cameraSetManual(posView)`.
    fn camera_set_manual(&mut self, pos_view: V4);
    /// `cameraSet()`.
    fn camera_set(&mut self);
    /// `ccGetCameraTransparency(pos, width, height, far, len, &hide)`
    /// (main 0x001da880): the fade and whether the camera's nearness hid
    /// it.
    fn camera_transparency(&mut self, pos: V4, width: F, height: F, far: F, len: F) -> (F, bool);
    /// `ccStream::GetChunkAdrsF(name, 0)` then `ccAnm::SetAnm(chunk, 0)`.
    fn anim_set(&mut self, anm: RideAnm, name: &str);
    /// `ccAnm::_AnimateForward(step)`: 1 when a play-once clip ended; 0
    /// without a clip.
    fn anim_forward(&mut self, anm: RideAnm, step: u16) -> i16;
    /// `ccAnm::NoteProcess()`: the notes the last step passed, for
    /// [`check_note`].
    fn anim_notes(&mut self, anm: RideAnm) -> Vec<Note>;
    /// `ccClump::GetObjAdrsF(name)` on the Grunty's clump, its
    /// `_SetLWMatrix()` (or the local matrix of a root), and the matrix's
    /// translation: where the leg is.
    fn leg(&mut self, name: &str) -> V4;
    /// `ccChar::Draw()` (gcmn 0x0056b1c0) on Kite's riding clump at `pos`
    /// with `setTransparency` `set_transparency`: the `transparency` it
    /// leaves.
    fn draw(&mut self, pos: V4, set_transparency: F) -> F;
    /// Every [`Out`], in order.
    fn out(&mut self, o: Out);
}

impl Ride {
    /// `ccPucciguso::ccPucciguso(kind)` (gcmn 0x00510f30) with `plw`'s
    /// place and heading: the flags (only `STOP`), act 2 (-1 before), the
    /// idle count at 440, speed 60, `cycle` from `rand() >> 3`, the body
    /// in the list (`SetHitSW(1)`), both players on act 2's clips.
    pub fn new(kind: i32, plw_pos: V4, plw_rot: V4, t: &RideTables, w: &mut dyn RideWorld, rng: &mut dyn Rng) -> Ride {
        let cycle = rng.rand() >> 3;
        let mut r = Ride {
            pos: plw_pos,
            pos_p: VF0,
            rot: plw_rot,
            hit_attribute: 0,
            transparency: ONE,
            char_set_transparency: ONE,
            flags: flag::STOP,
            act: act::IDLE,
            act_old: -1,
            anm_end: 0,
            idle: 440,
            speed: SPEED,
            speed_rate: 0,
            now_speed: 0,
            set_transparency: ONE,
            kind,
            cycle,
            pos_view: [0; 4],
            pos_eye: [0; 4],
            angle: VF0,
            move_pos: VF0,
            move_ease: VF0,
            hit: CharHit {
                mask2: WALL,
                kind: BODY_KIND,
                radius: K100,
                height: K100,
                pos: plw_pos,
                ..CharHit::default()
            },
            frame_spd: [256, 256],
        };
        w.hit_switch(&mut r.hit, true);
        w.anim_set(RideAnm::Kite, t.anim(r.act));
        w.anim_set(RideAnm::Pg, &t.anim_pg(r.act, kind));
        r
    }
}

/// `ccPucciguso::Main()` (gcmn 0x005114e0): one frame of the ride.
///
/// `pauseSW` from `plw`; the move from the stick ([`control_move`]); where
/// it would land (the ground under it), pushed out of the bodies and the
/// walls as `ccSpcChar::HitCheck` does for a walker (twice, the second
/// halfway), then along the walls: blocked, the move shrinks to half the
/// way past the body's radius (toward the push); unpushed, a wall in the
/// way stops it; the place on by the move; the ground and the way in
/// ([`collision_test`]); `hitAttribute`; the wrap ([`map_loop_adjust_pos`]);
/// both players' matrices, `plw`'s place, heading and pause; the camera;
/// the acts; nothing of Kite drawn in the eye view (`LOST_HEAD`), else at
/// its own transparency; Kite's clump (`ccChar::Draw`), the Grunty
/// ([`draw_pg`]); the count on.
pub fn main(r: &mut Ride, w: &mut dyn RideWorld, input: &Input, g: &mut Globals, t: &RideTables, rng: &mut dyn Rng) {
    r.flags = (r.flags & !flag::PAUSE) | u8::from(input.pause);
    r.move_pos = VF0;
    control_move(r, w, input, rng);
    let mut hp = moved(r.move_pos, r.pos);
    hp[2] = w.land(hp, LAND);
    r.hit.pos = hp;
    r.hit.radius = add(K100, r.now_speed);
    let raised = |mut v: V4, h: F| {
        v[2] = add(v[2], h);
        v
    };
    if w.collide(&mut r.hit) != 0 {
        hp = moved(geom::vadd(r.move_pos, r.hit.offset), r.pos);
        hp[2] = w.land(hp, LAND);
        r.hit.pos = hp;
        if w.collide(&mut r.hit) != 0 {
            let mut h2 = geom::vadd(hp, r.hit.offset);
            h2[2] = w.land(h2, LAND);
            h2[3] = ONE;
            hp = geom::vscale(geom::vadd(hp, h2), HALF);
            hp[3] = ONE;
            hp[2] = w.land(hp, LAND);
        }
        let h = r.hit.height;
        let mut d = w.line(raised(r.pos, h), raised(hp, h), WALL);
        if !geom::eq(MINUS_ONE, d) {
            d = sub(d, r.hit.radius);
            if !le(d, 0) {
                d = mul(d, HALF);
            }
            let mut v = geom::vsub(hp, r.pos);
            v[2] = 0;
            r.move_pos = geom::vscale(geom::normalize(v), d);
        } else {
            r.move_pos = geom::vsub(hp, r.pos);
        }
    } else {
        let h = r.hit.height;
        let d = w.line(raised(r.pos, h), raised(hp, h), WALL);
        if !geom::eq(MINUS_ONE, d) {
            r.move_pos = VF0;
        }
    }
    r.pos[0] = add(r.pos[0], r.move_pos[0]);
    r.pos[1] = add(r.pos[1], r.move_pos[1]);
    collision_test(r, w, input, g);
    r.hit_attribute = w.hit_attribute();
    map_loop_adjust_pos(r, w, input, g);
    w.out(Out::Matrix { anm: RideAnm::Kite, pos: r.pos, rot: r.rot });
    w.out(Out::Matrix { anm: RideAnm::Pg, pos: r.pos, rot: r.rot });
    w.out(Out::Player { pos: r.pos, rot: r.rot, pause: r.flags & flag::PAUSE != 0 });
    camera_pos_calc(r);
    camera_pos_set(r, w);
    anim_ctrl(r, w, t, rng);
    let tr = if r.flags & flag::LOST_HEAD != 0 { 0 } else { r.set_transparency };
    r.transparency = tr;
    r.char_set_transparency = tr;
    r.transparency = w.draw(r.pos, r.char_set_transparency);
    draw_pg(r, w, input);
    r.cycle = r.cycle.wrapping_add(1);
}

/// A move from `pos`: x, y and z added, w 1.
fn moved(mv: V4, pos: V4) -> V4 {
    [add(mv[0], pos[0]), add(mv[1], pos[1]), add(mv[2], pos[2]), ONE]
}

/// `ccPucciguso::PadLeverPower(p)` (gcmn 0x00511ed0): below 64 nothing,
/// 64-127 eased to 32-126, from 128 as it is (`ccPlayer`'s without the
/// target's first frames).
pub fn pad_lever_power(power: F) -> F {
    let v = fptosi(power) as i16;
    let r: i32 = if v < 64 {
        0
    } else if v < 128 {
        (i32::from(v) - 64) * 3 / 2 + 32
    } else {
        i32::from(v)
    };
    from_int(i32::from(r as i16))
}

/// `ccPucciguso::ControlMove()` (gcmn 0x005119c0): the move from the left
/// stick, eased.
///
/// Not leaning (or paused): `MOVE` off, the eased move 1/8 of the way
/// back to nothing a frame, and every fourth frame with it still over 10,
/// dust from all four legs thrown back along it. Leaning: `speedRate` the
/// lean over 255, `MOVE` on, `RUN` past 240; the heading against the
/// camera's (straight ahead while a camera reset runs and the stick stays
/// within 2047 of it, else the reset ends); walking at `6.2 * lean / 140`
/// (at most 1.3 of it), running at `60 * lean / 255`; the move 0.08 of the
/// way toward that a frame. It turns to the heading unless in the eye
/// view.
pub fn control_move(r: &mut Ride, w: &mut dyn RideWorld, input: &Input, rng: &mut dyn Rng) -> i32 {
    let mut s0 = (i32::from(geom::rad2deg(add(PI, r.rot[2]))) - 32768) as i16;
    let mut power = pad_lever_power(from_int(i32::from(input.pad.pow_l)));
    if r.flags & flag::PAUSE != 0 {
        power = 0;
    }
    r.now_speed = 0;
    if geom::eq(power, 0) {
        r.flags &= !flag::MOVE;
        r.move_pos[0] = 0;
        r.move_pos[1] = 0;
        r.move_ease[0] = add(r.move_ease[0], mul(EASE_OFF, sub(r.move_pos[0], r.move_ease[0])));
        r.move_ease[1] = add(r.move_ease[1], mul(EASE_OFF, sub(r.move_pos[1], r.move_ease[1])));
        r.move_pos[0] = r.move_ease[0];
        r.move_pos[1] = r.move_ease[1];
        r.move_ease[2] = 0;
        if r.cycle & 3 == 0 {
            let len = sqrtf(geom::dot(r.move_ease, r.move_ease));
            if !le(len, K10) {
                // (float)atan2((double)y, (double)x), a quarter turn on.
                let y = f64::from(f32::from_bits(r.move_ease[1]));
                let x = f64::from(f32::from_bits(r.move_ease[0]));
                let a = (y.atan2(x) as f32).to_bits();
                let d = (i32::from(geom::rad2deg(a)) + 16384) as i16;
                paw_smoke(r, w, geom::deg2rad(d), 15, rng);
            }
        }
    } else {
        r.speed_rate = div(power, K255);
        r.flags |= flag::MOVE;
        if !le(power, K240) {
            r.flags |= flag::RUN;
        } else {
            r.flags &= !flag::RUN;
        }
        let mut dirc = input.pad.dirc_l;
        if let Some(reset) = w.camera_reset() {
            let a = i32::from(geom::rad2deg(reset));
            let b = i32::from(geom::rad2deg(dirc));
            if ((a - b).abs() as i16) < 2048 {
                dirc = PI;
            } else {
                w.clear_camera_reset();
            }
        }
        let s = geom::rad2deg(add(PI, dirc));
        let rot = w.camera_rot();
        let c = i32::from(geom::rad2deg(add(PI, rot[2]))) - 32768;
        let c = fptosi(from_int(c)) as i16;
        s0 = (i32::from(c) - i32::from(s)) as i16;
        let heading = geom::deg2rad(s0);
        let v = if r.flags & flag::RUN == 0 {
            r.speed_rate = div(power, K140);
            if !le(r.speed_rate, WALK_MAX) {
                r.speed_rate = WALK_MAX;
            }
            mul(WALK_SPEED, r.speed_rate)
        } else {
            mul(r.speed, r.speed_rate)
        };
        r.now_speed = v;
        r.move_pos[0] = mul(v, sinf(heading));
        r.move_pos[1] = mul(neg(v), cosf(heading));
        r.move_ease[0] = add(r.move_ease[0], mul(EASE_ON, sub(r.move_pos[0], r.move_ease[0])));
        r.move_ease[1] = add(r.move_ease[1], mul(EASE_ON, sub(r.move_pos[1], r.move_ease[1])));
        r.move_pos[0] = r.move_ease[0];
        r.move_pos[1] = r.move_ease[1];
    }
    if w.camera_type() != 1 && !geom::eq(power, 0) {
        r.rot[2] = geom::deg2rad(s0);
    }
    1
}

/// `ccPucciguso::AnimCtrl()` (gcmn 0x00511f80): the acts and both
/// players.
///
/// Standing (acts 0-3) the idle count runs to 451, then 2 fidgets (3), 0
/// fidgets (1); a fidget's clip ended, 3 goes on to 4 and 1 back to 2.
/// Starting to move: `STOP` off, walking (6); stopping: `STOP` on, a walk
/// or run back to 2; moving, `RUN` switches 6 and 5. A new act sets both
/// clips; the frame speed is `2 * 256 * speedRate` walking, `256 *
/// speedRate` running, else 256; both step, the second's end kept, and
/// the Grunty's notes go to [`check_note`].
pub fn anim_ctrl(r: &mut Ride, w: &mut dyn RideWorld, t: &RideTables, rng: &mut dyn Rng) {
    if r.act < 4 {
        r.idle = r.idle.wrapping_add(1);
        if r.idle >= 451 {
            r.idle = 0;
            match r.act {
                act::IDLE => r.act = act::FIDGET,
                act::IDLE0 => r.act = act::FIDGET0,
                _ => {}
            }
        }
    } else {
        r.idle = 0;
    }
    if r.anm_end != 0 {
        match r.act {
            act::FIDGET => r.act = act::IDLE_AFTER,
            act::FIDGET0 => r.act = act::IDLE,
            _ => {}
        }
    }
    let stop = r.flags & flag::STOP != 0;
    let moving = r.flags & flag::MOVE != 0;
    if stop && moving {
        r.flags &= !flag::STOP;
        r.act = act::WALK;
    } else if !stop && !moving {
        r.flags |= flag::STOP;
        if r.act == act::WALK || r.act == act::RUN {
            r.act = act::IDLE;
        }
    }
    if r.flags & flag::STOP == 0 && r.flags & flag::MOVE != 0 {
        let run = r.flags & flag::RUN != 0;
        if run && r.act == act::WALK {
            r.act = act::RUN;
        } else if !run && r.act == act::RUN {
            r.act = act::WALK;
        }
    }
    if r.act_old != r.act {
        w.anim_set(RideAnm::Kite, t.anim(r.act));
        w.anim_set(RideAnm::Pg, &t.anim_pg(r.act, r.kind));
    }
    let spd = match r.act {
        act::WALK => geom::fptoui(mul(K2, mul(K256, r.speed_rate))) as u16,
        act::RUN => geom::fptoui(mul(K256, r.speed_rate)) as u16,
        _ => 256,
    };
    r.frame_spd = [spd, spd];
    r.anm_end = w.anim_forward(RideAnm::Kite, spd);
    r.anm_end = w.anim_forward(RideAnm::Pg, spd);
    for n in w.anim_notes(RideAnm::Pg) {
        check_note(r, w, n, rng);
    }
    r.act_old = r.act;
}

/// `ccPuccigusoCheckNote(note)` (gcmn 0x00512fb0), the Grunty's note
/// function: notes 1 and 2 are `ccSeSetParamInu(param, this)` (the dogs'
/// table: footsteps on the ground, cries), running with dust from all four
/// legs, a fidget's param-0 step with dust from the front two.
pub fn check_note(r: &mut Ride, w: &mut dyn RideWorld, note: Note, rng: &mut dyn Rng) {
    if !matches!(note.event, 1 | 2) {
        return;
    }
    w.out(Out::Sound { param: note.param, attribute: r.hit_attribute });
    if r.act == act::RUN {
        paw_smoke(r, w, r.rot[2], 15, rng);
    }
    if r.act == act::FIDGET && note.param == 0 {
        paw_smoke(r, w, r.rot[2], 3, rng);
    }
}

/// `ccPucciguso::PawSmoke(dirc, legs)` (gcmn 0x00512670): a puff of dust
/// at a leg. Each leg in `legs` ([`LEGS`]) takes its place, a speed of
/// `0.1 * 0.075 * -speed * (10 - rand() % 5)` back along `dirc` and its
/// life, the last one taken winning (one `rand()` each); none on ground
/// whose attribute (`& 0xf0f0f0`) is one of the five soft kinds, type 4 on
/// the eleven dark ones, else 133.
pub fn paw_smoke(r: &Ride, w: &mut dyn RideWorld, dirc: F, legs: i32, rng: &mut dyn Rng) {
    let m = geom::rot_matrix_z(&geom::unit_matrix(), dirc);
    let mut pos = [0; 4];
    let mut v = VF0;
    let (mut s, mut life) = (0, 0);
    for &(bit, name, l) in &LEGS {
        if legs & bit == 0 {
            continue;
        }
        let p = w.leg(name);
        pos = [p[0], p[1], p[2], ONE];
        let k = from_int(10 - rng.rand() % 5);
        v = VF0;
        v[1] = mul(K01, mul(mul(K0075, mul(MINUS_ONE, r.speed)), k));
        v = geom::apply_matrix(&m, v);
        s = K3;
        life = l;
    }
    w.land(pos, 0x2000_0000);
    let a = w.hit_attribute() & 0x00f0_f0f0;
    const NONE: [u32; 5] = [0x00b0_c000, 0x00c0_d000, 0x0060_b0d0, 0x0070_c0e0, 0x0080_80f0];
    const DARK: [u32; 11] = [
        0x0030_4050,
        0x0040_5060,
        0x0050_7080,
        0x0070_90a0,
        0x0090_b0c0,
        0x00c0_c0c0,
        0x00d0_d0d0,
        0x00e0_e0e0,
        0x0040_4040,
        0x0050_5050,
        0x0060_6060,
    ];
    if NONE.contains(&a) {
        return;
    }
    let t = if DARK.contains(&a) { 4 } else { 133 };
    w.out(Out::Smoke { pos, v, s, life, t });
}

/// `ccPucciguso::CollisionTest()` (gcmn 0x00512360): stood on the ground
/// (`pos.z`, `posP.z`); ground with attribute bit 0x80000 is a dungeon's
/// way in (`pgR` 0, `pgDIN` 1, `WORLD_MAN::Enter`), unless leaving one.
pub fn collision_test(r: &mut Ride, w: &mut dyn RideWorld, input: &Input, g: &mut Globals) {
    let z = w.land(r.pos, LAND);
    r.pos[2] = z;
    r.pos_p[2] = z;
    if !input.dne
        && let Some(attr) = w.hit_result()
        && attr & 0x80000 != 0
    {
        g.pg_r = 0;
        g.pg_din = 1;
        w.out(Out::Enter { pos: r.pos });
    }
}

/// `ccPucciguso::MapLoopAdjustPos()` (gcmn 0x005123f0): the streaming
/// follows the move (`WORLD_MAN::AddCenter`); past the map's edge the
/// place wraps round ([`kite::w2m_pos`]) and is stood on the ground again.
pub fn map_loop_adjust_pos(r: &mut Ride, w: &mut dyn RideWorld, input: &Input, g: &mut Globals) {
    w.out(Out::AddCenter { x: r.move_pos[0], y: r.move_pos[1] });
    let (pos, wrapped) = kite::w2m_pos(&input.bounds, r.pos);
    r.pos = pos;
    if wrapped {
        collision_test(r, w, input, g);
    }
}

/// `ccPucciguso::CameraPosCalc()` (gcmn 0x005124b0): the camera's target
/// 190 above, the eyes 250 above, the eye view's heading the ride's.
pub fn camera_pos_calc(r: &mut Ride) {
    r.pos_view = [r.pos[0], r.pos[1], add(r.pos[2], K190), ONE];
    r.pos_eye = [r.pos[0], r.pos[1], add(r.pos[2], K250), ONE];
    r.angle[2] = r.rot[2];
}

/// `ccPucciguso::CameraPosSet()` (gcmn 0x00512540): by the camera's type,
/// the eye view from the eyes (with the field's camera the heading
/// follows its turn and Kite is not drawn: `LOST_HEAD`), else the camera
/// on the target; then `cameraSet`.
pub fn camera_pos_set(r: &mut Ride, w: &mut dyn RideWorld) {
    if w.camera_type() == 1 {
        let eye = r.pos_eye;
        w.camera_set_eye_level(eye, &mut r.angle);
        if w.camera_id() == 1 {
            r.flags |= flag::LOST_HEAD;
            r.rot[2] = r.angle[2];
        } else {
            r.flags &= !flag::LOST_HEAD;
        }
    } else {
        let v = r.pos_view;
        w.camera_set_manual(v);
        r.flags &= !flag::LOST_HEAD;
    }
    w.camera_set();
}

/// `ccPucciguso::DrawPG()` (gcmn 0x00512c80), the Grunty's `ccChar::Draw`:
/// on the character layer, in the eye view at full, else faded by the
/// camera (`ccGetCameraTransparency` at the town's 4000 over 400 or the
/// field's 7000 over 600) times `setTransparency`; drawn unless under
/// 0.05 with the camera's nearness not the cause; on shaded ground
/// (attribute bit 0x40000) with the ambient halved and the distant light
/// off. (Its fog blend, `ccChar` +0xa8-+0xac, is never set on the ride.)
pub fn draw_pg(r: &mut Ride, w: &mut dyn RideWorld, input: &Input) {
    let eye = w.camera_type() == 1;
    if eye {
        r.char_set_transparency = ONE;
        r.transparency = ONE;
    }
    let (far, len) = if input.area == 0 { (0x457a_0000, 0x43c8_0000) } else { (0x45da_c000, 0x4416_0000) };
    let (mut f, hide) = w.camera_transparency(r.pos, SIZE, SIZE, far, len);
    if eye {
        f = ONE;
    }
    r.transparency = mul(r.char_set_transparency, f);
    let by = if hide { r.char_set_transparency } else { r.transparency };
    let alpha = ((fptosi(mul(K256, by)) + 1) >> 1) as u8;
    if lt(r.transparency, K005) && !hide {
        return;
    }
    w.out(Out::DrawPg {
        transparency: r.transparency,
        shadow: by,
        alpha,
        height: add(SIZE, mul(K2, SIZE)),
        shaded: r.hit_attribute & 0x40000 != 0,
    });
}

/// `ccPuccigusoExit`'s place for the member in registry slot `slot` (not
/// Kite: he stays where the ride left him): 150 out from `plw.pos` at
/// `puccigusoAngleTbl[slot]` off `plw`'s heading (135 degrees either side
/// behind for slots 1 and 2); the member takes `plw`'s heading too.
pub fn exit_place(t: &RideTables, plw_pos: V4, plw_rot: V4, slot: usize) -> V4 {
    let a = geom::rad2deg(plw_rot[2]);
    let off = t.angles.get(slot).copied().unwrap_or(0) as u16;
    let r = geom::deg2rad((i32::from(off) + i32::from(a)) as i16);
    let mut p = [0; 4];
    p[0] = add(p[0], mul(K150, sinf(r)));
    p[1] = sub(p[1], mul(K150, cosf(r)));
    geom::vadd(p, plw_pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stick_eases_in() {
        let p = |v: f32| f32::from_bits(pad_lever_power(v.to_bits()));
        assert_eq!(p(63.0), 0.0);
        assert_eq!(p(64.0), 32.0);
        assert_eq!(p(100.0), 86.0);
        assert_eq!(p(127.0), 126.0);
        assert_eq!(p(200.0), 200.0);
    }

    #[test]
    fn the_grunty_clip_names_its_kind() {
        let t = RideTables { anims_pg: vec!["ANM_cdg0nut0".into()], ..RideTables::default() };
        assert_eq!(t.anim_pg(0, 0), "ANM_cdg0nut0");
        assert_eq!(t.anim_pg(0, 1), "ANM_cdg2nut0");
        assert_eq!(t.anim_pg(0, 8), "ANM_cdg9nut0");
    }

    #[test]
    fn the_adult_check_reads_the_pens() {
        let mut s = piney_data::save::SaveData::new();
        assert_eq!(adult_check(&s, 2, 1), -1);
        s.set_i16(SAVE_GROWTH + 24 * 2 + 0xe + 2, 1);
        assert_eq!(adult_check(&s, 2, 1), 3);
        s.set_i16(SAVE_GROWTH + 24 * 4 + 0xe + 4, 1);
        assert_eq!(adult_check(&s, 4, 2), 8);
        s.set_i16(SAVE_GROWTH + 24 + 0xe, 1);
        assert_eq!(adult_check(&s, 1, 0), 0);
        assert_eq!(adult_check(&s, 0, 0), -1);
        assert_eq!(adult_check(&s, 1, 3), -1);
    }
}
