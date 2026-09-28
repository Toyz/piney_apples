//! A [`Host`] from plain values, for a runtime that gathers them each frame
//! from the field (`piney_world::World`: the player, the camera, the
//! characters an effect may name) and lends its `rand()`.

use piney_world::camera::Camera as FieldCamera;
use piney_world::player::Player;

use crate::draw::Camera;
use crate::ee::{F, ONE, V4};
use crate::{CharRef, Host, space};

/// A character as the effects read it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharState {
    pub id: CharRef,
    /// `ccChar` +0x40 `pos`, +0x60 `dirc`.
    pub pos: V4,
    pub dirc: V4,
    /// The parameter row's +0x18 `height`, +0x1c `width`.
    pub height: F,
    pub width: F,
}

impl CharState {
    /// Kite as the field's player: `plw`.
    pub fn player(id: CharRef, p: &Player) -> CharState {
        CharState { id, pos: p.body.pos, dirc: p.body.dirc, height: p.height, width: p.width }
    }
}

impl Camera {
    /// The field camera as `cameraSet` left it: `tcam` is both
    /// `cameraList[camID]` and `activeCamPtr`.
    pub fn from_field(c: &FieldCamera) -> Camera {
        Camera {
            eye: c.tcam.pos,
            cam_pos: c.tcam.pos,
            cam_view: c.tcam.view,
            world_view: c.world_view,
            world_screen: c.world_screen,
            env: Default::default(),
        }
    }
}

/// The world's inputs for one frame.
pub struct Simple<'a> {
    /// The game's one `rand()`.
    pub rand: &'a mut dyn FnMut() -> i32,
    /// `genrand()`, the Mersenne Twister behind `ccRand` (the portal's
    /// sparks); 0 without one.
    pub genrand: Option<&'a mut dyn FnMut() -> u32>,
    pub player: V4,
    pub bounds: [F; 4],
    pub camera: Camera,
    pub chars: Vec<CharState>,
}

impl<'a> Simple<'a> {
    /// A town's bounds, no characters.
    pub fn new(rand: &'a mut dyn FnMut() -> i32, player: V4, camera: Camera) -> Simple<'a> {
        Simple { rand, genrand: None, player, bounds: space::TOWN_BOUNDS, camera, chars: Vec::new() }
    }

    fn get(&self, c: CharRef) -> Option<&CharState> {
        self.chars.iter().find(|s| s.id == c)
    }
}

impl Host for Simple<'_> {
    fn rand(&mut self) -> i32 {
        (self.rand)()
    }
    fn genrand(&mut self) -> u32 {
        self.genrand.as_mut().map_or(0, |g| g())
    }
    fn player_pos(&self) -> V4 {
        self.player
    }
    fn bounds(&self) -> [F; 4] {
        self.bounds
    }
    fn camera(&self) -> Camera {
        self.camera
    }
    fn char_pos(&self, c: CharRef) -> V4 {
        self.get(c).map_or([0, 0, 0, ONE], |s| s.pos)
    }
    fn char_dirc(&self, c: CharRef) -> V4 {
        self.get(c).map_or([0; 4], |s| s.dirc)
    }
    fn char_height(&self, c: CharRef) -> F {
        self.get(c).map_or(0, |s| s.height)
    }
    fn char_width(&self, c: CharRef) -> F {
        self.get(c).map_or(0, |s| s.width)
    }
}
