//! What the motion layer asks of the world: the queries and calls the
//! game's movement, spawning and animation code makes into code that is not
//! rules - collision, the camera, the player's frame, the animation
//! players. The field runtime implements [`World`] over its own systems
//! (piney-world's `hit.rs`, `camera.rs`, the `ccAnm`s its `chara.rs` and
//! `body.rs` play); the checks implement it with scripts that answer what
//! the harness answers the game's own code with.
//!
//! Every method stands for one game function and is called exactly where,
//! and as often as, the game calls that function, so a world with state
//! (a hit list, an animation's clock) sees the same sequence the game's
//! does. Characters are scene indices ([`crate::scene::Scene`]); floats are
//! bit patterns ([`crate::geom`]).

use crate::geom::{F, V4};

/// `ccCharHit` (0x50 bytes, main `libhit.cpp`): a character's body in the
/// collision. The motion code owns it (an enemy's at `ccEntryObj` +0x160,
/// a party member's `bodyHit` at +0x1a0) and fills `pos`, `radius` and
/// `height` before [`World::collide`]; the world keeps the list of bodies
/// (`ccCharHitTop`/`Tail`, main 0x0037890c/10) and answers `offset`.
///
/// ```text
/// +0x00 hitSW  +0x04 mask  +0x08 mask2  +0x0c type  +0x10 next
/// +0x14 radius  +0x18 height  +0x20 pos  +0x30 offset  +0x40 attribute
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharHit {
    /// `hitSW`: in the world's list of bodies.
    pub sw: bool,
    /// The kinds of body it is pushed out of (-1 from the constructor).
    pub mask: u32,
    /// The polygons of the ground it is pushed out of (`mask2`).
    pub mask2: u32,
    /// Its own kind (`type`: 7 the player, 2 a party member, ...).
    pub kind: u32,
    pub radius: F,
    pub height: F,
    pub pos: V4,
    /// The push out of the others and the ground, [`World::collide`]'s.
    pub offset: V4,
    /// The ground attribute of its last contact.
    pub attribute: u32,
}

impl Default for CharHit {
    /// `ccCharHit::ccCharHit` (main 0x00153220): out of the list, every
    /// mask, radius 1.
    fn default() -> Self {
        CharHit {
            sw: false,
            mask: u32::MAX,
            mask2: u32::MAX,
            kind: 0,
            radius: crate::geom::ONE,
            height: 0,
            pos: crate::geom::VF0,
            offset: [0; 4],
            attribute: 0,
        }
    }
}

/// A note an animation passed (`ccAnmNote`: +0x4 `event`, +0x8 `param`,
/// from the clip's `F_Note` records), as `ccAnm::NoteProcess` hands them
/// to the character's note function in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Note {
    /// 0x8005 a normal attack's hit, 0x8003 an enemy's skill start, 0x8002
    /// a shock wave, 1 and 2 sounds ...
    pub event: u32,
    pub param: u32,
}

/// Which of a character's animation players: `ccChar.anm` (+0xd4), or
/// the second one a middle boss's second model plays (`ccEnemy` +0x1bc).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnmSlot {
    Main,
    Second,
}

/// What the motion layer asks of the world.
pub trait World {
    /// `ccTransPosW2P(out, pos)` (gcmn 0x0059b940, `ccPlayer::W2PPos`
    /// 0x0059b5a0): a world position in the player's frame (a field's map
    /// wraps around the player; in a town and a dungeon it is a copy).
    fn w2p(&mut self, pos: V4) -> V4;
    /// `ccTransPosP2W(out, pos)` (gcmn 0x0059b980, `ccPlayer::P2WPos`
    /// 0x0059b710): back from the player's frame.
    fn p2w(&mut self, pos: V4) -> V4;
    /// `ccLandHitCheck(pos, mask)` (gcmn 0x00571e00): the height of the
    /// ground under `pos` among the polygons `mask` names (a segment from
    /// 105 above to 1000 below).
    fn land(&mut self, pos: V4, mask: u32) -> F;
    /// `checkHitResultAttlibute()` (main 0x00153e80): the attribute of the
    /// nearest polygon the last query hit.
    fn hit_attribute(&mut self) -> u32;
    /// `_ccHitCheckLM(from, to, mask, kind)` (main 0x00153930): the segment
    /// against the polygons `mask` names; -1.0 ([`crate::geom::MINUS_ONE`])
    /// when nothing is in the way. `kind` 0 is `ccHitCheckLM` (main
    /// 0x001538d0), 1 `ccHitCheckLM2` (0x00153900).
    fn line(&mut self, from: V4, to: V4, mask: u32, kind: i32) -> F;
    /// `ccCharHit::CollisionDetection()` (main 0x00153470) on `who`'s
    /// body: `hit.offset` set to the push out of every other body in the
    /// list whose kind `hit.mask` names (while `hit.sw`), then out of the
    /// ground polygons `hit.mask2` names around `pos + height`, and
    /// `hit.attribute`. Returns 1 when a body was touched, plus 2 when the
    /// ground was. The world refreshes its list's copy of the body first.
    fn collide(&mut self, who: usize, hit: &mut CharHit) -> i32;
    /// `hitResultCharType` (main 0x0037892c): the kinds of the bodies the
    /// last [`World::collide`] touched.
    fn hit_char_type(&mut self) -> u32;
    /// `ccCharHit::HitEnable()` (main 0x00153310, `on`) and `HitDisable()`
    /// (0x00153360): `who`'s body into the list at its tail, or out of it
    /// (`hit.sw` follows).
    fn hit_switch(&mut self, who: usize, hit: &mut CharHit, on: bool);
    /// `ccCheckCameraDeg(pos, deg)` (main 0x001da710): whether `pos` lies
    /// within `deg` (16-bit units) either side of the active camera's line
    /// of sight, on the ground.
    fn camera_deg(&mut self, pos: V4, deg: i16) -> bool;
    /// `ccGetCameraTransparency(pos, width, height, far, len)` (main
    /// 0x001da880): 1.0 in the open, toward 0 as the camera closes in on
    /// the character and beyond `far` over `len`.
    fn camera_transparency(&mut self, pos: V4, width: F, height: F, far: F, len: F) -> F;
    /// `ccStream::GetChunkAdrsF(name, 0)` then `ccAnm::SetAnm(chunk, 0)`
    /// (main 0x00150f50) on `who`'s player `slot`: play the clip `name`
    /// (an `ANM_` object of the character's scene file) from frame 0.
    fn anim_set(&mut self, who: usize, slot: AnmSlot, name: &str);
    /// `ccAnm.frameNow` (+0x98): the frame the player is at.
    fn anim_frame(&mut self, who: usize, slot: AnmSlot) -> u16;
    /// `ccAnm::_AnimateForward(step)` (main 0x00152270), `step` in 1/256
    /// frames (`ccAnm.frameSpd`, +0x9c): 1 when a play-once clip reached
    /// its end, else 0; 0 without a clip (the callers test `ccAnm` +0xac
    /// first).
    fn anim_forward(&mut self, who: usize, slot: AnmSlot, step: u16) -> i16;
    /// `ccAnm::NoteProcess()` (main 0x00152210): the notes the last
    /// [`World::anim_forward`] passed, in order, for the character's note
    /// function to act on.
    fn anim_notes(&mut self, who: usize, slot: AnmSlot) -> Vec<Note>;
    /// `patNum` (`ccEff` +0x30) of the Eff chunk `name` in character
    /// `who`'s own file (a symbol's fires); None when unknown.
    fn eff_pat_num(&mut self, _who: usize, _name: &str) -> Option<u16> {
        None
    }
}
