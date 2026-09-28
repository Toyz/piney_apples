//! The Chaos Gate: `ccChgate` (gcmn chgate.cpp), the gimmick of
//! `gimmickTbl[16]` (`CHGATE.CCS`) every Root Town puts at its `DMY_gate`
//! (`ccSetChaosGate` 0x00458df0), where the game saves and leaves for the
//! fields. Two anms: the gate (`CMP_xmgtwav0`, a 91-frame loop whose omni
//! light lights whoever stands in it) and the magic circle (`CMP_xmgtcir0`),
//! drawn only while the gate menu has it open (`chaosGateInfluence`: command
//! 11 opens, 0 closes). `ccChgate::main` (0x00459280) draws within 4500 of
//! Kite (docs/engine/field-game.md).

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use piney_data::archive::Archive;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::chara::light_matrix;
use crate::draw::{self, CHAR_LAYER, OBJ_LAYER};
use crate::ee::{self, F, ONE, V4};
use crate::pose::Play;
use crate::town::{self, AnimLight, Light, TownLights, anim_draw_fog};

/// `gimmickTbl[16]` (gcmn 0x0061e7f0): its file, and the size
/// `ccGetCameraTransparency` fades it by (+0x18 height, +0x1c width).
pub const FILE: &str = "chgate";
pub const HEIGHT: F = 0x42c8_0000; // 100.0
pub const WIDTH: F = 0x4270_0000; // 60.0
/// Its row of `gimmickTbl` (and so its code).
pub const GIMMICK: i16 = 16;
/// `ccSetChaosGate`'s dummy in the town's file.
pub const DUMMY: &str = "DMY_gate";
/// The gate and its idle loop; the circle and its three animations.
pub const BODY_CLUMP: &str = "CMP_xmgtwav0";
pub const BODY_ANIM: &str = "ANM_xmgtcir1";
pub const RING_CLUMP: &str = "CMP_xmgtcir0";
pub const RING_ANIMS: [&str; 3] = ["ANM_xmgtcir2", "ANM_xmgtcir3", "ANM_xmgtcir4"];
/// `ccChgate::main` draws only within this of the player (4500).
const NEAR: F = 0x458c_a000;
/// `ccChar::Draw` in a town: `ccGetCameraTransparency(pos, width, height,
/// 4000, 400)`, and nothing under 0.05.
const FADE_FAR: F = 0x457a_0000;
const FADE_LEN: F = 0x43c8_0000;
const MIN_DRAWN: F = 0x3d4c_cccd;
/// Sound effects `gateAnm` starts (`ccSeOn3D`).
pub const SE_OPENED: i32 = 71;
pub const SE_CLOSE: i32 = 72;

pub use crate::char::{camera_transparency, w2p};

/// What the gate did in a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GateFrame {
    /// Within 4500 of the player: the gate's anm stepped.
    pub near: bool,
    /// `ccChar::Draw` drew it.
    pub drawn: bool,
    /// The circle stepped and drew.
    pub ring: bool,
    /// `ccSeOn3D`, if a sound started.
    pub sound: Option<i32>,
}

/// One scene file's animation on a clump.
struct Anm {
    play: Play,
    nodes: Vec<u32>,
    /// Its light records, and the light as the last step left it (black,
    /// `ccOmniLight::Init`, before the first).
    lights: Vec<AnimLight>,
    light: Light,
}

impl Anm {
    fn new(file: &SceneFile, clump: &str, anim: &str) -> Result<Anm> {
        let c = file.ccs.find_object(clump).ok_or_else(|| Error::NotFound(clump.into()))?;
        let nodes = file.scene.clumps.iter().find(|(o, _)| *o == c).map(|(_, n)| n.clone()).unwrap_or_default();
        let mut a =
            Anm { play: Play { anim: 0, time: 0, posed: 0, frame_spd: 256 }, nodes, lights: Vec::new(), light: BLACK };
        a.set(file, anim)?;
        Ok(a)
    }

    /// `ccAnm::SetAnm`: frame 0, its light new and black.
    fn set(&mut self, file: &SceneFile, anim: &str) -> Result<()> {
        self.play = Play::new(file, anim).ok_or_else(|| Error::NotFound(anim.into()))?;
        self.lights = town::anim_lights(file, self.play.anim)?.1;
        self.light = BLACK;
        Ok(())
    }

    /// `_AnimateForward(frameSpd)`, the light following (under `root`).
    fn forward(&mut self, file: &SceneFile, root: Mat4) -> bool {
        let ended = self.play.forward(file);
        if let Some(l) = self.lights.first() {
            let mut light = l.at(self.play.posed);
            light.pos = root.transform_point3(light.pos);
            self.light = light;
        }
        ended
    }
}

/// `ccOmniLight::Init` (0x001397e0): black, intensity 1, no fall-off.
const BLACK: Light = Light {
    kind: 4,
    pos: Vec3::ZERO,
    dir: Vec3::ZERO,
    colour: Vec3::ZERO,
    intensity: 1.0,
    far_start: 0.0,
    far_end: 0.0,
    radius: [0.0; 2],
    priority: 0,
};

/// A Root Town's Chaos Gate.
pub struct Gate {
    pub file: Rc<SceneFile>,
    morphers: HashMap<u32, u32>,
    /// +0x40 `pos` (DMY_gate's position) and +0x60 `dirc` (0, 0, -pi).
    pub pos: V4,
    pub dirc: V4,
    /// `SetMatrix_PosRotZYX(pos, dirc)` as stored.
    pub root: [V4; 4],
    /// +0xd4: the gate; +0x1e4: the magic circle.
    body: Anm,
    ring: Anm,
    /// +0x1ec `gateAnm`'s state, +0x1f0 its frame count, +0x1e8 the
    /// circle's animation ended.
    pub state: u32,
    pub count: u32,
    pub ended: bool,
    /// ccChar +0x8c (1 from `ccChar::ccChar`; `routine`'s fades never run
    /// for the gate) and +0x88, what it is drawn at.
    pub cloak: F,
    pub transparency: F,
    /// The last frame's.
    pub frame: GateFrame,
}

impl Gate {
    /// `ccSetChaosGate` and `ccChgate::ccChgate`: `town` is the town's file,
    /// which holds `DMY_gate`.
    pub fn new(archive: &Arc<Archive>, town: &SceneFile) -> Result<Gate> {
        let file = Rc::new(SceneFile::read(archive, FILE)?);
        let dummy = town
            .ccs
            .find_object(DUMMY)
            .and_then(|d| town.scene.dummies.get(&d))
            .ok_or_else(|| Error::NotFound(DUMMY.into()))?;
        let pos = [dummy.pos.x.to_bits(), dummy.pos.y.to_bits(), dummy.pos.z.to_bits(), ONE];
        // (0, 0, DEG2RAD(-32768)); w is whatever the stack held.
        let dirc = [0, 0, ee::deg2rad(-32768), 0];
        let root = town::pos_rot_zyx(pos, [dirc[0], dirc[1], dirc[2]]);
        let body = Anm::new(&file, BODY_CLUMP, BODY_ANIM)?;
        let ring = Anm::new(&file, RING_CLUMP, RING_ANIMS[0])?;
        if let Some(n) = RING_ANIMS.iter().find(|n| file.anim(n).is_none()) {
            return Err(Error::NotFound((*n).into()));
        }
        let morphers = piney_data::anim::morphers(&file.ccs).unwrap_or_default();
        Ok(Gate {
            file,
            morphers,
            pos,
            dirc,
            root,
            body,
            ring,
            state: 0,
            count: 0,
            ended: false,
            cloak: ONE,
            transparency: ONE,
            frame: GateFrame::default(),
        })
    }

    /// `chaosGateInfluence` (0x00459570): what the gate menu's command does
    /// to it; 11 opens the circle (or, open, keeps it looping), 0 closes it.
    pub fn influence(&mut self, cmnd: i16) {
        match cmnd {
            0 => self.state = 5,
            11 => self.state = if self.state == 0 { 1 } else { 3 },
            _ => {}
        }
    }

    /// `ccChgate::gateAnm` (0x004593e0): the circle's animations; a sound
    /// when one starts.
    fn gate_anm(&mut self) -> Result<Option<i32>> {
        let file = self.file.clone();
        let mut sound = None;
        match self.state {
            1 => {
                self.ring.set(&file, RING_ANIMS[0])?;
                self.state = 2;
                self.count = 0;
            }
            2 => {
                if self.ended {
                    self.ended = false;
                    self.state = 3;
                }
                self.count += 1;
                if self.count - 1 == 30 {
                    sound = Some(SE_OPENED);
                }
            }
            3 => {
                self.ring.set(&file, RING_ANIMS[1])?;
                self.state = 4;
            }
            5 => {
                self.ring.set(&file, RING_ANIMS[2])?;
                self.state = 6;
                sound = Some(SE_CLOSE);
            }
            6 if self.ended => {
                self.ended = false;
                self.state = 0;
            }
            _ => {}
        }
        Ok(sound)
    }

    /// One frame of `ccThEntryCtrl` for the gate: `ccEntryObj::routine`
    /// (nothing that shows) and `ccChgate::main` up to the draw, for Kite at
    /// `player` and the camera at `cam` (pitch `deg1`, the eye view when
    /// `eye`).
    pub fn step(&mut self, player: V4, cam: V4, deg1: i16, eye: bool) -> GateFrame {
        let sound = self.gate_anm().unwrap_or(None);
        let mut f = GateFrame { sound, ..GateFrame::default() };
        let rel = w2p(self.pos, player);
        let d2 = ee::add(ee::add(ee::mul(rel[0], rel[0]), ee::mul(rel[1], rel[1])), ee::mul(rel[2], rel[2]));
        // fptodp, sqrt, dptofp: a rounded square root.
        if ee::lt(ee::sqrtf(d2), NEAR) {
            f.near = true;
            let (file, root) = (self.file.clone(), draw::mat(&self.root));
            self.body.forward(&file, root);
            // ccChar::Draw: +0x90 is 1, so `hide` starts set only in the
            // eye view (which skips the near-camera fade).
            let mut hide = eye;
            let fade = camera_transparency(self.pos, player, cam, deg1, WIDTH, HEIGHT, FADE_FAR, FADE_LEN, &mut hide);
            self.transparency = ee::mul(self.cloak, fade);
            f.drawn = !(ee::lt(self.transparency, MIN_DRAWN) && !hide);
            if self.state != 0 {
                self.ended = self.ring.forward(&file, root);
                f.ring = true;
            }
        }
        self.frame = f;
        f
    }

    /// The gate's animation times (the gate's, the circle's).
    pub fn times(&self) -> (u32, u32) {
        (self.body.play.time, self.ring.play.time)
    }

    /// The draw environment's lights with the gate's two added after the
    /// town's (`ccLightGrp::AddGrp` keeps equal priorities in the order
    /// they came).
    pub fn lights(&self, town: &TownLights) -> TownLights {
        let mut out = town.clone();
        out.lights.push(self.body.light);
        out.lights.push(self.ring.light);
        out
    }

    /// The last step's draws: the gate on layer 5 (`CHAR_LAYER`) at its
    /// transparency, lit by `lights` (the town's with the gate's); the
    /// circle, while used, on sysLayer (priority 0). Both anms are
    /// `SetFogSw(0)`: no fog, whatever the town's (the portal's additive
    /// plane would take the fog colour on its black border).
    pub fn draw(&self, layers: &mut Layers, to_screen: Mat4, lights: &TownLights) {
        let root = draw::mat(&self.root);
        let lit = |world: Mat4| light_matrix(lights, world, false);
        let fm = (&*self.file, &self.morphers);
        let fog = draw::Fogging::None;
        if self.frame.drawn {
            let alpha = ee::f(self.transparency);
            let b = &self.body;
            anim_draw_fog(layers, CHAR_LAYER, to_screen, fm, &b.play, root, &b.nodes, alpha, None, Some(&lit), fog);
        }
        if self.frame.ring {
            let r = &self.ring;
            anim_draw_fog(layers, OBJ_LAYER, to_screen, fm, &r.play, root, &r.nodes, 1.0, None, Some(&lit), fog);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn w2p_keeps_z_and_wraps() {
        let p = w2p([0x4000_0000, 0, 0x4416_0000, 0], [0x3f80_0000, 0, 0, ONE]);
        assert_eq!(p, [ONE, 0, 0x4416_0000, ONE]);
        let far = w2p([0x46c3_5000, 0, 0, ONE], [0xc6c3_5000, 0, 0, ONE]); // 25000 - -25000
        assert_eq!(ee::f(far[0]), 2000.0);
    }

    #[test]
    fn transparency_fades_near_the_camera() {
        let gate = [0, 0x45c4_e000, 0x4416_0000, ONE]; // (0, 6300, 600)
        let player = [0, 0x45af_0000, 0x4416_0000, ONE];
        let mut hide = false;
        let far = [0, 0x45a0_0000, 0x4416_0000, ONE]; // 1000 away
        assert_eq!(camera_transparency(gate, player, far, 1512, WIDTH, HEIGHT, FADE_FAR, FADE_LEN, &mut hide), ONE);
        assert!(!hide);
        let close = [0, 0x45c4_e000, 0x4416_0000, ONE];
        assert_eq!(camera_transparency(gate, player, close, 1512, WIDTH, HEIGHT, FADE_FAR, FADE_LEN, &mut hide), 0);
        assert!(hide);
    }

    fn gate() -> Option<Gate> {
        let archive = crate::town::tests::archive()?;
        let town = SceneFile::read(&archive, "town01").unwrap();
        Some(Gate::new(&archive, &town).unwrap())
    }

    /// At `DMY_gate` (0, 6300, 600) turned by -pi about z, its idle loop
    /// stepping each frame within 4500 of Kite, its light 166.8 above it.
    #[test]
    fn stands_at_the_gate_plaza() {
        let Some(mut g) = gate() else { return };
        assert_eq!(g.pos.map(ee::f), [0.0, 6300.0, 600.0, 1.0]);
        assert_eq!(g.root[0], [0xbf80_0000, 0, 0, 0]);
        assert_eq!(g.root[1], [0, 0xbf80_0000, 0, 0]);
        assert_eq!(g.root[3].map(ee::f), [0.0, 6300.0, 600.0, 1.0]);
        let kite = [0, 0x45af_0000, 0x4416_0000, ONE];
        let cam = [0, 0x45cd_0000, 0x445c_0000, ONE];
        let f = g.step(kite, cam, 1512, false);
        assert_eq!((f.near, f.drawn, f.ring, f.sound), (true, true, false, None));
        assert_eq!((g.transparency, g.times()), (ONE, (256, 0)));
        let l = g.lights(&TownLights { ambient: Vec3::ZERO, lights: Vec::new(), fog: None });
        let light = l.lights[0];
        assert_eq!((light.kind, light.far_start, light.far_end, light.intensity), (4, 400.0, 500.0, 1.0));
        assert!((light.pos - Vec3::new(0.0, 6300.0, 766.819)).length() < 0.01, "{:?}", light.pos);
        assert!((light.colour - Vec3::new(110.0, 137.0, 214.0) / 255.0).length() < 0.001);
        // The circle's light: black until the circle steps.
        assert_eq!(l.lights[1].colour, Vec3::ZERO);
        // Far from Kite it neither steps nor draws.
        let far = [0, 0x4480_0000, 0, ONE];
        assert!(!g.step(far, cam, 1512, false).near);
        assert_eq!(g.times().0, 256);
    }

    /// The gate menu's commands: 11 opens the circle (ANM_xmgtcir2, sound
    /// 71 once its count passes 30, then the ANM_xmgtcir3 loop), 0 closes
    /// it.
    #[test]
    fn the_circle_opens_and_closes() {
        let Some(mut g) = gate() else { return };
        let kite = [0, 0x45af_0000, 0x4416_0000, ONE];
        let cam = [0, 0x45cd_0000, 0x445c_0000, ONE];
        g.influence(11);
        let mut sounds = Vec::new();
        for i in 0..120 {
            let f = g.step(kite, cam, 1512, false);
            assert!(f.ring);
            if let Some(s) = f.sound {
                sounds.push((i, s));
            }
        }
        assert_eq!(sounds, [(31, SE_OPENED)]);
        assert_eq!(g.state, 4);
        g.influence(0);
        let f = g.step(kite, cam, 1512, false);
        assert_eq!((f.sound, g.state), (Some(SE_CLOSE), 6));
        for _ in 0..100 {
            g.step(kite, cam, 1512, false);
        }
        assert_eq!(g.state, 0);
        assert!(!g.step(kite, cam, 1512, false).ring);
    }

    /// A new game's arrival: the gate is drawn from the first frames, on the
    /// character layer.
    #[test]
    fn drawn_on_arrival() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut iso = piney_data::iso::Iso::open("../../work/infection/infection.iso").unwrap();
        let tables = piney_demo::newgame::NewGameTables::of(iso.volume().unwrap());
        let mut save = crate::SaveState::fresh();
        piney_demo::newgame::new_game(&mut save.save, 0, &tables, piney_demo::newgame::SAVE_VA);
        let mut world = crate::World::enter(&mut iso, archive, save).unwrap();
        let mut pad = piney_input::Pad::default();
        let mut drawn = 0;
        for _ in 0..40 {
            pad.read(&piney_input::Raw::default());
            let frame = world.step(&pad);
            let gate_models =
                frame.cmds.iter().filter(|c| matches!(c, piney_draw::Cmd::Model(m) if m.file == FILE)).count();
            if world.gate().frame.drawn {
                assert!(gate_models > 0);
                drawn += 1;
            }
        }
        assert!(drawn > 20, "{drawn}");
    }
}
