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
    /// The camera as `cameraSet` left it: the eye of `cameraGetPos(camID)`
    /// and `activeCamPtr`'s eye and view point (`ccCheckCameraDeg`), both
    /// `cameraList[camID]`: `tcam` in play, `ecam` while an event's camera
    /// holds the view (main 0x00161590, 0x001da750).
    pub fn from_field(c: &FieldCamera) -> Camera {
        let active = c.active();
        Camera {
            eye: active.pos,
            cam_pos: active.pos,
            cam_view: active.view,
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

#[cfg(test)]
mod tests {
    use piney_world::camera::{Camera as FieldCamera, Scheme, id};

    use super::*;
    use crate::effect::check_camera_deg;

    /// An event's camera (`changeCamera(3)`, `ecam`) facing the other way
    /// from the field's: the eye and the cone follow `ecam`, so a point
    /// before the field camera and behind the event's is not shown.
    #[test]
    fn the_cone_is_the_active_cameras() {
        let f = |v: f32| v.to_bits();
        let mut c = FieldCamera::new([0, 0, 0, ONE], [0; 4], 0, Scheme::new(1));
        c.tcam.pos = [f(0.0), f(-500.0), f(200.0), ONE];
        c.tcam.view = [0, 0, f(100.0), ONE];
        c.ecam.pos = [f(0.0), f(500.0), f(200.0), ONE];
        c.ecam.view = [f(0.0), f(1000.0), f(100.0), ONE];
        let at = [f(0.0), f(300.0), 0, ONE];
        let shown = |c: &FieldCamera| {
            let cam = Camera::from_field(c);
            check_camera_deg(at, [0, 0, 0, ONE], space::TOWN_BOUNDS, cam.cam_pos, cam.cam_view, 12288)
        };
        assert!(shown(&c));
        c.change_camera(id::EVENT);
        assert_eq!(Camera::from_field(&c).eye, c.ecam.pos);
        assert!(!shown(&c));
    }
}
