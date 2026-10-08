//! The riding Grunty: `ccPucciguso` (gcmn `pgrider.cpp`,
//! 0x00510880-0x005130ac), the adult Grunty the Grunty Flute calls in a field,
//! Kite on its back: [`adult_check`] (`ccPgAdultCheck` 0x00510650),
//! [`Ride::new`] (0x00510f30), [`main`] (`ccPucciguso::Main` 0x005114e0, on
//! `ccThPucciguso` at priority 49) and [`exit_place`] (`ccPuccigusoExit`
//! 0x00510bd0). The start, the fades, the party asleep and the music are the
//! runtime's. World calls go through [`RideWorld`]; what shows or sounds is an
//! [`Out`]. docs/engine/grunty-ride.md has the whole.

use piney_data::volume::Volume;

use crate::damage::fptosi;
use crate::geom::{self, F, MINUS_ONE, ONE, PI, V4, VF0, add, cosf, div, from_int, le, lt, mul, neg, sinf, sub};
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
    /// From Mutation on, in a field: the search runs ([`Seek`]).
    pub const SEEK: u8 = 1 << 6;
    /// A line waits in [`Ride::chat`] for its balloon.
    pub const CHAT: u8 = 1 << 7;
}

/// `ccSys->pad[0]`'s push bits the search reads: Triangle calls it (from
/// Outbreak on, starts it), L1 or R1 in the eye view takes over.
pub const TRIANGLE: u32 = 0x10;
pub const SHOULDERS: u32 = 0x0c;
/// The balloon's line (+0xe1 to the acts, 0x51 bytes) and its cut: the
/// copy stops once 79 characters are in.
pub const CHAT_LEN: usize = 0x51;
const CHAT_MAX: usize = 79;
/// `0x0052f9d0`'s event areas with no dungeon (MUT main 0x001b1600; Outbreak's
/// and Quarantine's alike): `WORLD::SetSpecialObj` skips the same.
const NO_DUNGEON: [i32; 20] = [113, 112, 111, 110, 109, 82, 81, 80, 79, 78, 57, 56, 55, 54, 53, 42, 41, 40, 39, 28];
/// The search's types (`ride_seek_types` by kind).
const SEEK_FOOD: i32 = 0;
const SEEK_DUNGEON: i32 = 1;
const SEEK_PORTAL: i32 = 2;
/// A Grunty food's base type flag (`gimmickTbl` rows 22-37).
const FOOD_TYPE: i32 = 0x0080_0000;

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
const K5: F = 0x40a0_0000;
const K100: F = 0x42c8_0000;
const K140: F = 0x430c_0000;
const K125: F = 0x42fa_0000;
const K150: F = 0x4316_0000;
const K190: F = 0x433e_0000;
const K240: F = 0x4370_0000;
const K250: F = 0x437a_0000;
const K255: F = 0x437f_0000;
const K256: F = 0x4380_0000;
const HALF: F = 0x3f00_0000;
/// 0.7 and 19: a town's body for the ride (from Mutation on).
const K07: F = 0x3f33_3333;
const K19: F = 0x4198_0000;
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
    /// The volume: its `sqrtf` ([`geom::sqrt_on`]).
    pub volume: Volume,
    /// From Mutation on, the field search's tables; none on Infection.
    pub seek: Option<SeekTables>,
}

/// The search's tables (`tables::combat`'s `ride_seek_*`, MUT gcmn
/// 0x0061d180, 0x0061d1a8, 0x00684b50-0x00684be0).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SeekTables {
    /// The type by kind: 0 Grunty foods, 1 the dungeon, 2 magic portals.
    pub types: Vec<i32>,
    /// How near counts as there, by type.
    pub ranges: Vec<F>,
    /// The Grunty's lines by kind: found, none, cancelled, near.
    pub lines: [Vec<&'static str>; 4],
}

impl SeekTables {
    /// The kind's search type and its range (the constructor's
    /// `ranges[types[kind]]`).
    pub fn kind(&self, kind: i32) -> (i32, F) {
        let ty = usize::try_from(kind).ok().and_then(|k| self.types.get(k)).copied().unwrap_or(0);
        (ty, usize::try_from(ty).ok().and_then(|t| self.ranges.get(t)).copied().unwrap_or(0))
    }

    /// The kind's line.
    pub fn line(&self, say: Say, kind: i32) -> &'static str {
        usize::try_from(kind).ok().and_then(|k| self.lines[say as usize].get(k)).copied().unwrap_or("")
    }
}

/// What the Grunty says of its search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Say {
    Found,
    None,
    Cancel,
    Near,
}

/// From Mutation on, a field ride's search (MUT gcmn +0x158-+0x160,
/// +0x1d0, +0x1e0, +0x24c): the Grunty sniffs out the nearest thing of its
/// kind's type, says so and leads Kite there (docs/engine/grunty-ride.md,
/// "The search").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seek {
    /// +0x158: 0 the stick drives; else the Grunty leads ([`seek_move`]),
    /// 1 the stick may take over, more a count down to 1.
    pub lead: i32,
    /// +0x15c: frames left standing before it sniffs again.
    pub idle: i32,
    /// +0x160: frames it stands before it sets off.
    pub hold: i32,
    /// +0x1d0: where it leads, +0x1e0: how near counts as there.
    pub target: V4,
    pub range: F,
    /// +0x24c: the thing's base name for a line's `#a`; none for the
    /// dungeon (the lines then put Kite's).
    pub name: Option<&'static str>,
}

/// `g_entCtrl`'s list the search walks: gimmicks (+0x28) or magic circles
/// (+0x1c).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeekList {
    Gimmicks,
    Circles,
}

/// What the search reads of an entry: its place, its base's type flags and
/// name, `objFlag` and `destFlag` (+0xe0 bits 0 and 5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeekEntry {
    pub pos: V4,
    pub ty: i32,
    pub name: Option<&'static str>,
    pub active: bool,
    pub going: bool,
}

impl RideTables {
    /// The volume's (`tables::combat`).
    pub fn of(volume: Volume) -> RideTables {
        let t = piney_data::tables::combat::of(volume);
        let owned = |v: &[&str]| v.iter().map(|s| s.to_string()).collect();
        let mut angles = [0; SLOTS];
        angles.copy_from_slice(&t.ride_angles()[..SLOTS]);
        let seek = (volume != Volume::Inf).then(|| SeekTables {
            types: t.ride_seek_types().to_vec(),
            ranges: t.ride_seek_ranges().iter().map(|v| v.to_bits()).collect(),
            lines: [t.ride_seek_found(), t.ride_seek_none(), t.ride_seek_cancel(), t.ride_seek_near()]
                .map(<[_]>::to_vec),
        });
        RideTables {
            anims: owned(t.ride_anims()),
            anims_pg: owned(t.ride_anims_pg()),
            files: owned(t.ride_files()),
            angles,
            volume,
            seek,
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
    /// The volume's code is Mutation's or later (a town's rules in area 0).
    pub later: bool,
    /// From Mutation on, the field search, and +0xe1 the balloon's line
    /// ([`flag::CHAT`] while it waits).
    pub seek: Seek,
    pub chat: [u8; CHAT_LEN],
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
    /// From Mutation on, in a town: the Flag Race's handling of the
    /// Grunty ridden (the race's +0x64-+0x70).
    pub race: Option<RaceRide>,
    /// `ccSys->pad[0]`'s push (+0x2d0) and right stick's lean (`powR`,
    /// +0x2b1): the search's buttons.
    pub push: u32,
    pub pow_r: u8,
}

/// The Flag Race's handling of the Grunty ridden (MUT gcmn race object
/// +0x64-+0x70, from the race's tables by the Grunty's kind): the top
/// speed, the acceleration, and the move's easing leaning and not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RaceRide {
    pub max: F,
    pub accel: F,
    pub ease_on: F,
    pub ease_off: F,
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
    /// From Mutation on, `ccEntryCmnd(this)`: the ride on the command
    /// list, which a balloon over it needs (`ccCheckTarget`).
    EntryCmnd,
    /// `ccChatMsg::OpenChat(ccChat, this, +0xe1)`: the Grunty's line over
    /// it, to its NUL (MUT gcmn 0x00530dd0).
    Chat([u8; CHAT_LEN]),
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
    /// From Mutation on, what the field search reads (a town never
    /// searches): a list's entries from its head (`next` +0x1c4),
    /// `WORLD_MAN.eventAreaNumber` (+0x120) and `dungeonPos[0]` (+0x460),
    /// `ccTransPosW2P(pos)`, and Kite's name (`getPartyMenberChar(0)`'s).
    fn seek_list(&mut self, _list: SeekList) -> Vec<SeekEntry> {
        Vec::new()
    }
    fn dungeon(&mut self) -> (i32, V4) {
        (0, VF0)
    }
    fn w2p(&mut self, pos: V4) -> V4 {
        pos
    }
    fn kite_name(&mut self) -> Vec<u8> {
        Vec::new()
    }
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
                // From Mutation on 25 over the radius (MUT gcmn 0x0052e030).
                height: if t.volume == Volume::Inf { K100 } else { K125 },
                pos: plw_pos,
                ..CharHit::default()
            },
            frame_spd: [256, 256],
            later: t.volume != Volume::Inf,
            // From Mutation on (MUT gcmn 0x0052de50): nothing found, 10
            // frames' hold, the kind's range.
            seek: Seek {
                hold: 10,
                target: VF0,
                range: t.seek.as_ref().map_or(0, |s| s.kind(kind).1),
                ..Seek::default()
            },
            chat: [0; CHAT_LEN],
        };
        w.hit_switch(&mut r.hit, true);
        w.anim_set(RideAnm::Kite, t.anim(r.act));
        w.anim_set(RideAnm::Pg, &t.anim_pg(r.act, kind));
        if r.later {
            w.out(Out::EntryCmnd);
        }
        r
    }
}

/// `ccPucciguso::Main()` (gcmn 0x005114e0): one frame of the ride: the move
/// from the stick ([`control_move`]), pushed out of the bodies and the walls
/// as `ccSpcChar::HitCheck` does for a walker, the ground and the way in
/// ([`collision_test`]), the wrap, `plw`'s place, the camera, the acts and the
/// draws (Kite's clump, then the Grunty's, [`draw_pg`]). The order is in
/// docs/engine/grunty-ride.md ("A frame: Main").
pub fn main(r: &mut Ride, w: &mut dyn RideWorld, input: &Input, g: &mut Globals, t: &RideTables, rng: &mut dyn Rng) {
    if t.volume != Volume::Inf {
        return main_later(r, w, input, g, t, rng);
    }
    r.flags = (r.flags & !flag::PAUSE) | u8::from(input.pause);
    r.move_pos = VF0;
    control_move(t, r, w, input, rng);
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
    anim_ctrl(r, w, t, input, rng);
    let tr = if r.flags & flag::LOST_HEAD != 0 { 0 } else { r.set_transparency };
    r.transparency = tr;
    r.char_set_transparency = tr;
    r.transparency = w.draw(r.pos, r.char_set_transparency);
    draw_pg(r, w, input);
    r.cycle = r.cycle.wrapping_add(1);
}

/// Mutation's `ccPucciguso::Main()` (MUT gcmn 0x0052e3a0), also
/// Outbreak's and Quarantine's. Infection's but for the body and the
/// walls:
/// - in a town the body is 0.7 of the Grunty's width (`pcgsTbl` +0x1c)
///   round and 19 more high, set before each push; in a field its
///   radius is 100 plus `nowSpeed` as before;
/// - a wall between the ride and where the pushes left it stops the
///   move, and so does one between it and where the move takes it;
/// - while the Grunty leads ([`Seek::lead`]) [`seek_move`] moves it in
///   place of the stick, and a line waiting opens its balloon at the end.
fn main_later(r: &mut Ride, w: &mut dyn RideWorld, input: &Input, g: &mut Globals, t: &RideTables, rng: &mut dyn Rng) {
    let town = input.area == 0;
    r.flags = (r.flags & !flag::PAUSE) | u8::from(input.pause);
    r.move_pos = VF0;
    if r.seek.lead != 0 {
        seek_move(t, r, w, input, rng);
    } else {
        control_move(t, r, w, input, rng);
    }
    let body = |r: &mut Ride| {
        if town {
            r.hit.radius = mul(K07, SIZE);
            r.hit.height = add(K19, r.hit.radius);
        } else {
            r.hit.radius = add(K100, r.now_speed);
        }
    };
    let raised = |mut v: V4, h: F| {
        v[2] = add(v[2], h);
        v
    };
    let mut hp = moved(r.move_pos, r.pos);
    hp[2] = w.land(hp, LAND);
    r.hit.pos = hp;
    body(r);
    if w.collide(&mut r.hit) != 0 {
        hp = moved(geom::vadd(r.move_pos, r.hit.offset), r.pos);
        hp[2] = w.land(hp, LAND);
        r.hit.pos = hp;
        if town {
            body(r);
        }
        if w.collide(&mut r.hit) != 0 {
            let mut h2 = geom::vadd(hp, r.hit.offset);
            h2[2] = w.land(h2, LAND);
            h2[3] = ONE;
            hp = geom::vscale(geom::vadd(hp, h2), HALF);
            hp[3] = ONE;
            hp[2] = w.land(hp, LAND);
        }
        let h = r.hit.height;
        r.move_pos = if geom::eq(MINUS_ONE, w.line(raised(r.pos, h), raised(hp, h), WALL)) {
            geom::vsub(hp, r.pos)
        } else {
            VF0
        };
        let mut to = geom::vadd(r.pos, r.move_pos);
        to[3] = ONE;
        if !geom::eq(MINUS_ONE, w.line(raised(r.pos, h), raised(to, h), WALL)) {
            r.move_pos = VF0;
        }
    } else {
        let h = r.hit.height;
        if !geom::eq(MINUS_ONE, w.line(raised(r.pos, h), raised(hp, h), WALL)) {
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
    anim_ctrl(r, w, t, input, rng);
    let tr = if r.flags & flag::LOST_HEAD != 0 { 0 } else { r.set_transparency };
    r.transparency = tr;
    r.char_set_transparency = tr;
    r.transparency = w.draw(r.pos, r.char_set_transparency);
    draw_pg(r, w, input);
    open_chat(r, w, input);
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
/// stick, eased (0.08 of the way a frame leaning, 1/8 back to nothing not
/// leaning, with dust every fourth frame). It walks at `6.2 * lean / 140` (at
/// most 1.3 of it) and runs past 240 at `60 * lean / 255`, and turns to the
/// heading unless in the eye view (docs/engine/grunty-ride.md, "The stick").
pub fn control_move(t: &RideTables, r: &mut Ride, w: &mut dyn RideWorld, input: &Input, rng: &mut dyn Rng) -> i32 {
    let volume = t.volume;
    if volume != Volume::Inf {
        return control_move_later(t, r, w, input, rng);
    }
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
            let len = geom::length_on(volume, r.move_ease);
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

/// Mutation's `ccPucciguso::ControlMove()` (MUT gcmn 0x0052e970), the
/// speed kept frame to frame (`nowSpeed`). Leaning in a town, the race's
/// acceleration times the walk's or run's speed is added up to its top,
/// the move easing by `ease_on`; in a field the speed is Infection's. Not
/// leaning, the move eases back by `ease_off` (a field's 1/8). The heading
/// follows the stick but in the eye view. In a field the search
/// ([`flag::SEEK`]) sniffs again after 40 frames standing, and Triangle
/// calls it off; from Outbreak on Triangle also starts it.
fn control_move_later(t: &RideTables, r: &mut Ride, w: &mut dyn RideWorld, input: &Input, rng: &mut dyn Rng) -> i32 {
    let volume = t.volume;
    let field = input.area == 1;
    let triangle = input.push & TRIANGLE != 0;
    if matches!(volume, Volume::Out | Volume::Qua) && field && r.flags & flag::SEEK == 0 && triangle {
        // OUT gcmn 0x00529834: the start (Mutation has none).
        if seek(t, r, w, input) != 0 {
            r.flags |= flag::SEEK;
            r.seek = Seek { lead: 40, idle: 0, hold: 20, ..r.seek };
            say(t, r, w, Say::Found, r.seek.name);
            return 0;
        }
        say(t, r, w, Say::None, None);
    }
    let town = input.area == 0;
    let race = input.race.unwrap_or_default();
    let mut s0 = (i32::from(geom::rad2deg(add(PI, r.rot[2]))) - 32768) as i16;
    let mut power = pad_lever_power(from_int(i32::from(input.pad.pow_l)));
    if r.flags & flag::PAUSE != 0 {
        power = 0;
    }
    if geom::eq(power, 0) {
        if r.flags & flag::SEEK != 0 {
            r.seek.idle -= 1;
        }
        r.flags &= !flag::MOVE;
        let ease = if town { race.ease_off } else { EASE_OFF };
        r.move_pos[0] = 0;
        r.move_pos[1] = 0;
        r.move_ease[0] = add(r.move_ease[0], mul(ease, sub(r.move_pos[0], r.move_ease[0])));
        r.move_ease[1] = add(r.move_ease[1], mul(ease, sub(r.move_pos[1], r.move_ease[1])));
        r.move_pos[0] = r.move_ease[0];
        r.move_pos[1] = r.move_ease[1];
        // The speed the eased move still makes along the heading.
        let along = |c: F, m: F| if geom::eq(0, c) { 0 } else { div(m, c) & 0x7fff_ffff };
        let a = along(sinf(r.rot[2]), r.move_ease[0]);
        let b = along(cosf(r.rot[2]), mul(MINUS_ONE, r.move_ease[1]));
        let v = mul(HALF, add(a, b));
        if lt(v, r.now_speed) {
            r.now_speed = v;
        }
        r.move_ease[2] = 0;
        if r.cycle & 3 == 0 && !le(geom::length_on(volume, r.move_ease), K10) {
            let y = f64::from(f32::from_bits(r.move_ease[1]));
            let x = f64::from(f32::from_bits(r.move_ease[0]));
            let a = (y.atan2(x) as f32).to_bits();
            let d = (i32::from(geom::rad2deg(a)) + 16384) as i16;
            paw_smoke(r, w, geom::deg2rad(d), 15, rng);
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
        let run = r.flags & flag::RUN != 0;
        if !run {
            r.speed_rate = div(power, K140);
            if !le(r.speed_rate, WALK_MAX) {
                r.speed_rate = WALK_MAX;
            }
        }
        let v = match (town, run) {
            (true, false) => add(r.now_speed, mul(mul(WALK_SPEED, r.speed_rate), race.accel)),
            (true, true) => add(r.now_speed, mul(race.accel, mul(race.max, r.speed_rate))),
            (false, false) => mul(WALK_SPEED, r.speed_rate),
            (false, true) => mul(r.speed, r.speed_rate),
        };
        let v = if town && !le(v, race.max) { race.max } else { v };
        r.now_speed = v;
        let ease = if town { race.ease_on } else { EASE_ON };
        r.move_pos[0] = mul(v, sinf(heading));
        r.move_pos[1] = mul(neg(v), cosf(heading));
        r.move_ease[0] = add(r.move_ease[0], mul(ease, sub(r.move_pos[0], r.move_ease[0])));
        r.move_ease[1] = add(r.move_ease[1], mul(ease, sub(r.move_pos[1], r.move_ease[1])));
        r.move_pos[0] = r.move_ease[0];
        r.move_pos[1] = r.move_ease[1];
        if r.flags & flag::SEEK != 0 {
            r.seek.idle = 40;
        }
    }
    if w.camera_type() != 1 {
        r.rot[2] = geom::deg2rad(s0);
    }
    // MUT gcmn 0x0052f0a0: standing long enough, it sniffs again; Triangle
    // calls the search off.
    if field && r.flags & flag::SEEK != 0 && r.seek.idle <= 0 {
        if seek(t, r, w, input) != 0 {
            r.seek = Seek { lead: 1, idle: 0, hold: 20, ..r.seek };
            say(t, r, w, Say::Found, r.seek.name);
            return 0;
        }
        r.flags &= !flag::SEEK;
        say(t, r, w, Say::None, None);
    }
    if field && r.flags & flag::SEEK != 0 && triangle {
        r.flags &= !flag::SEEK;
        r.seek.lead = 0;
        r.seek.idle = 0;
        say(t, r, w, Say::Cancel, None);
        return 0;
    }
    1
}

/// MUT gcmn 0x0052f250: the Grunty leads (`Main` calls it for
/// `ControlMove` while [`Seek::lead`] is set). At 1 Triangle calls it off
/// and the stick (in the eye view also L1, R1 or the right stick) takes
/// over for 60 frames' wait; more counts down to 1. It stands
/// [`Seek::hold`] frames, then runs at 1.3 of its speed toward
/// [`Seek::target`], and once within [`Seek::range`] says it is there.
pub fn seek_move(t: &RideTables, r: &mut Ride, w: &mut dyn RideWorld, input: &Input, rng: &mut dyn Rng) -> i32 {
    if r.seek.lead == 1 {
        if input.push & TRIANGLE != 0 {
            r.flags &= !flag::SEEK;
            r.seek = Seek { lead: 0, idle: 0, hold: 20, ..r.seek };
            say(t, r, w, Say::Cancel, None);
            return 0;
        }
        let mut power = pad_lever_power(from_int(i32::from(input.pad.pow_l)));
        if w.camera_type() == 1 && (input.push & SHOULDERS != 0 || input.pow_r >= 64) {
            power = add(power, ONE);
        }
        if !geom::eq(power, 0) {
            r.seek.idle = 60;
            r.seek.lead = 0;
            return 0;
        }
    } else {
        r.seek.lead -= 1;
        if r.seek.lead <= 0 {
            r.seek.lead = 1;
        }
    }
    if r.seek.hold > 0 {
        r.seek.hold = (r.seek.hold - 1).max(0);
        r.flags &= !(flag::MOVE | flag::RUN);
        r.speed_rate = ONE;
        r.move_pos[0] = 0;
        r.move_pos[1] = 0;
        r.move_ease[0] = add(r.move_ease[0], mul(EASE_OFF, sub(r.move_pos[0], r.move_ease[0])));
        r.move_ease[1] = add(r.move_ease[1], mul(EASE_OFF, sub(r.move_pos[1], r.move_ease[1])));
        r.move_pos[0] = r.move_ease[0];
        r.move_pos[1] = r.move_ease[1];
        r.move_ease[2] = 0;
        if r.cycle & 3 == 0 && !le(geom::length_on(t.volume, r.move_ease), K10) {
            let y = f64::from(f32::from_bits(r.move_ease[1]));
            let x = f64::from(f32::from_bits(r.move_ease[0]));
            let a = (y.atan2(x) as f32).to_bits();
            let d = (i32::from(geom::rad2deg(a)) + 16384) as i16;
            paw_smoke(r, w, geom::deg2rad(d), 15, rng);
        }
        return 1;
    }
    r.flags |= flag::MOVE | flag::RUN;
    r.speed_rate = WALK_MAX;
    if lt(r.now_speed, K5) {
        r.now_speed = mul(r.speed, r.speed_rate);
    }
    let v = mul(r.speed, r.speed_rate);
    let mut p = geom::vsub(w.w2p(r.seek.target), r.pos_p);
    let a = piney_data::libm::atan2f(p[1], p[0]);
    let heading = geom::deg2rad((geom::rad2deg(a) as u16 as i32 + 16384) as i16);
    r.rot[2] = heading;
    p[2] = 0;
    if le(geom::length_on(t.volume, p), r.seek.range) {
        r.flags &= !flag::SEEK;
        r.seek.lead = 0;
        say(t, r, w, Say::Near, r.seek.name);
        return 0;
    }
    r.move_pos[0] = mul(v, sinf(heading));
    r.move_pos[1] = mul(neg(v), cosf(heading));
    r.move_ease[0] = add(r.move_ease[0], mul(EASE_ON, sub(r.move_pos[0], r.move_ease[0])));
    r.move_ease[1] = add(r.move_ease[1], mul(EASE_ON, sub(r.move_pos[1], r.move_ease[1])));
    r.move_pos[0] = r.move_ease[0];
    r.move_pos[1] = r.move_ease[1];
    1
}

/// MUT gcmn 0x0052f790: the search by the kind's type (0x0052f800 foods,
/// 0x0052f9d0 the dungeon, 0x0052faa0 portals). It sets the target and the
/// name and answers -1 within 100 of the range, 1 farther, 0 for none.
pub fn seek(t: &RideTables, r: &mut Ride, w: &mut dyn RideWorld, input: &Input) -> i32 {
    let Some(st) = t.seek.as_ref() else { return 0 };
    let near = add(K100, r.seek.range);
    match st.kind(r.kind).0 {
        SEEK_DUNGEON => {
            let (area, pos) = w.dungeon();
            if NO_DUNGEON.contains(&area) {
                return 0;
            }
            r.seek.target = pos;
            r.seek.name = None;
            let d = geom::length_on(t.volume, w.w2p(pos));
            if lt(d, near) { -1 } else { 1 }
        }
        ty @ (SEEK_FOOD | SEEK_PORTAL) => {
            let list = if ty == SEEK_FOOD { SeekList::Gimmicks } else { SeekList::Circles };
            let mut best: Option<(SeekEntry, F)> = None;
            for e in w.seek_list(list) {
                let food = ty != SEEK_FOOD || e.ty & FOOD_TYPE != 0;
                if !food || (input.area == 2 && !e.active) || e.going {
                    continue;
                }
                let mut p = w.w2p(e.pos);
                p[2] = 0;
                p[3] = ONE;
                let d = geom::length_on(t.volume, p);
                // The first, or one no farther (scratch +4 starts at -1).
                if best.is_none_or(|(_, b)| !lt(b, d) || lt(b, 0)) {
                    best = Some((e, d));
                }
            }
            let Some((e, d)) = best else { return 0 };
            r.seek.target = e.pos;
            r.seek.name = e.name;
            if lt(d, near) { -1 } else { 1 }
        }
        _ => 0,
    }
}

/// MUT gcmn 0x00530e50: the kind's line into the balloon's buffer, `#a`
/// the name (Kite's for none), cut once 79 characters are in, and the
/// line set waiting ([`flag::CHAT`]). No table holds a `#` before anything
/// but `a`, on which the game's copy would not move on.
fn say(t: &RideTables, r: &mut Ride, w: &mut dyn RideWorld, what: Say, name: Option<&'static str>) {
    let line = t.seek.as_ref().map_or("", |s| s.line(what, r.kind)).as_bytes();
    let mut out = [0u8; CHAT_LEN];
    let (mut n, mut i) = (0, 0);
    let mut put = |b: u8, n: &mut usize| {
        if let Some(c) = out.get_mut(*n) {
            *c = b;
        }
        *n += 1;
    };
    while let Some(&c) = line.get(i) {
        if c == b'#' && line.get(i + 1) == Some(&b'a') {
            let who = name.map_or_else(|| w.kite_name(), |s| s.as_bytes().to_vec());
            who.iter().take_while(|&&b| b != 0).for_each(|&b| put(b, &mut n));
            i += 2;
        } else {
            put(c, &mut n);
            i += 1;
        }
        if n >= CHAT_MAX {
            break;
        }
    }
    put(0, &mut n);
    r.chat = out;
    r.flags |= flag::CHAT;
}

/// MUT gcmn 0x00530dd0, at the end of `Main`: outside a town a waiting line
/// opens its balloon over the ride.
fn open_chat(r: &mut Ride, w: &mut dyn RideWorld, input: &Input) {
    if input.area != 0 && r.flags & flag::CHAT != 0 {
        w.out(Out::Chat(r.chat));
        r.flags &= !flag::CHAT;
    }
}

/// `ccPucciguso::AnimCtrl()` (gcmn 0x00511f80): the acts and both players'
/// clips: the idle fidgets after 451 frames standing, walking (6) and running
/// (5) by `RUN`, back to 2 on stopping; the frame speed `2 * 256 * speedRate`
/// walking, `256 * speedRate` running, else 256; the Grunty's notes go to
/// [`check_note`] (docs/engine/grunty-ride.md, "The acts").
pub fn anim_ctrl(r: &mut Ride, w: &mut dyn RideWorld, t: &RideTables, input: &Input, rng: &mut dyn Rng) {
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
    // From Mutation on the ride runs from the start in a town, and keeps
    // running there (MUT gcmn 0x0052fe24, 0x0052ff18).
    let town = r.later && input.area == 0;
    let stop = r.flags & flag::STOP != 0;
    let moving = r.flags & flag::MOVE != 0;
    if stop && moving {
        r.flags &= !flag::STOP;
        r.act = if town { act::RUN } else { act::WALK };
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
        } else if !run && r.act == act::RUN && !town {
            r.act = act::WALK;
        }
    }
    if r.act_old != r.act {
        w.anim_set(RideAnm::Kite, t.anim(r.act));
        w.anim_set(RideAnm::Pg, &t.anim_pg(r.act, r.kind));
    }
    let spd = match r.act {
        act::WALK => geom::fptoui(mul(K2, mul(K256, r.speed_rate))) as u16,
        // A town's run plays twice as fast from Mutation on.
        act::RUN if town => geom::fptoui(mul(mul(K256, r.speed_rate), K2)) as u16,
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
    // From Mutation on not in a town (MUT gcmn 0x00530190).
    if !(r.later && input.area == 0)
        && !input.dne
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

    /// Each later volume's search lines: a `#` only ever starts `#a` (the
    /// game's copy would not move past another), and a line with the
    /// longest gimmick name fits the balloon's 79-character cut.
    #[test]
    fn the_search_lines_put_only_names() {
        for v in [Volume::Mut, Volume::Out, Volume::Qua] {
            let t = RideTables::of(v);
            let s = t.seek.expect("the search's tables");
            assert_eq!((s.types.len(), s.ranges.len()), (KINDS, 3));
            let longest = piney_data::tables::battle::of(v)
                .gimmicks()
                .iter()
                .filter_map(|g| g.param.base.name)
                .map(str::len)
                .max()
                .unwrap_or(0);
            for line in s.lines.iter().flatten() {
                let b = line.as_bytes();
                for (i, _) in b.iter().enumerate().filter(|&(_, &c)| c == b'#') {
                    assert_eq!(b.get(i + 1), Some(&b'a'), "{v:?}: {line}");
                }
                assert!(b.len() + longest < CHAT_MAX, "{v:?}: {line}");
            }
        }
        assert!(RideTables::of(Volume::Inf).seek.is_none());
    }
}
