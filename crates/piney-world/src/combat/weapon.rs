//! The enemies' weapon trails and flashes ([`piney_battle::weapon`]): the
//! controller a race's constructor makes (`entry::Out::Weapon`), run from
//! the race's `exclusive()` ([`enemy_motion::Call::WeaponCtrl`]) and handed
//! its notes ([`enemy_motion::Call::WeaponNote`]), each at what the enemy
//! then was.
//!
//! The controllers draw nothing random, so they run where the effects
//! start the frame's shows (`Combat::fx_call`), in their order, after the
//! entry control's frame: each enemy's nodes are its body posed as this
//! frame drew it (`Call::Draw`'s matrix, the clip where `animEnemy` left
//! it), as `ccAnm::Draw` left the coordinates the game reads. An enemy not
//! drawn this frame keeps its last matrix. The trails go to
//! [`Combat::trails`], the flashes' rays to [`Combat::rays`] (drawn as the
//! boxes'); the flashes' lights, like the boxes', are not put in the
//! scene's light group.

use std::collections::HashMap;

use glam::Mat4;
use piney_battle::blocks::{self, InfoRef};
use piney_battle::enemy_ai::Frame;
use piney_battle::enemy_motion::Call;
use piney_battle::entry;
use piney_battle::geom::M4;
use piney_battle::prim::{RadOut, RadWorld};
use piney_battle::weapon::{Pair, WeaponCtrl, WeaponOut, WeaponWorld};

use super::Combat;
use super::cast::Actor;
use super::stage::{PlayerFrame, Show};
use crate::camera::Camera;
use crate::ee::V4;

/// A weapon's trail this frame (`dispCell`'s packets): the strips and the
/// lines, each as `makePacket` was given them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Trail {
    pub polys: Vec<Pair>,
    pub lines: Vec<Pair>,
}

/// The enemies' weapon controllers by scene index, with the matrix each
/// was last drawn at.
#[derive(Clone, Debug, Default)]
pub struct Weapons {
    pub ctrl: HashMap<usize, WeaponCtrl>,
    pub last: HashMap<usize, M4>,
    /// For tests: the trails' pairs and the flashes' frames drawn so far.
    pub pairs: u64,
    pub flashes: u64,
}

/// What the controller asks of the battle: an enemy's nodes, Kite's frame,
/// the camera.
struct World<'a> {
    actor: Option<&'a Actor>,
    matrix: Option<M4>,
    worlds: Option<HashMap<u32, Mat4>>,
    frame: PlayerFrame,
    camera: &'a Camera,
}

impl RadWorld for World<'_> {
    fn fw2lw(&mut self, p: V4) -> V4 {
        self.frame.p2w(self.frame.w2p(p))
    }

    /// Never asked: the flash's plate has no dpLength or dpBank.
    fn rand(&mut self) -> i32 {
        0
    }

    /// `cameraGetRot2(camID)` in the eye view (`tcam` +0x40), else
    /// `cameraGetRot(camID)`.
    fn camera_rot(&mut self) -> V4 {
        if self.camera.active().kind == crate::camera::kind::EYE { self.camera.tcam.rot3 } else { self.camera.rot() }
    }
}

impl WeaponWorld for World<'_> {
    fn node_matrix(&mut self, name: &str, ext: bool) -> Option<M4> {
        let a = self.actor?;
        let body = &a.ch.body;
        let node = if ext { body.file.ccs.find_object(name) } else { body.node(name) }?;
        if self.worlds.is_none() {
            let root = super::stage::mat4(&self.matrix?);
            self.worlds = Some(body.worlds(&a.ch.play, root));
        }
        let m = self.worlds.as_ref()?.get(&node)?;
        Some(std::array::from_fn(|c| m.col(c).to_array().map(f32::to_bits)))
    }

    /// The enemy's file's dummy `name`: its place (w 1) and turn, the
    /// file's degrees in radians (taken to be what the stream's decoded
    /// chunk holds at +0x10 and +0x20).
    fn chunk(&mut self, name: &str) -> Option<(V4, V4)> {
        let file = &self.actor?.ch.body.file;
        let d = file.scene.dummies.get(&file.ccs.find_object(name)?)?;
        let r = d.rot.unwrap_or_default().to_array().map(f32::to_radians);
        let p = d.pos.to_array();
        Some((
            [p[0].to_bits(), p[1].to_bits(), p[2].to_bits(), 1f32.to_bits()],
            [r[0].to_bits(), r[1].to_bits(), r[2].to_bits(), 0],
        ))
    }
}

impl Combat {
    /// The shows from `from` on: each `ccEnemyWeaponCtrl` constructor
    /// builds its controller from the rows in gcmn, each note and frame
    /// runs it; the trails and rays it draws are kept for the frame.
    pub(super) fn weapon_pass(&mut self, from: usize, camera: &Camera, frame: &PlayerFrame) {
        for i in from..self.shows.len() {
            match self.shows[i] {
                Show::Entry(entry::Out::Weapon { who, info, n }) => self.weapon_made(who, info, n),
                Show::Entry(entry::Out::Destroyed { who, .. }) => {
                    self.weapons.ctrl.remove(&who);
                    self.weapons.last.remove(&who);
                }
                Show::Enemy(who, Call::WeaponNote { note, at }) => {
                    if let Some(c) = self.weapons.ctrl.get_mut(&who) {
                        c.note(&at, note);
                    }
                }
                Show::Enemy(who, Call::WeaponCtrl(at)) => {
                    let Some(c) = self.weapons.ctrl.get_mut(&who) else { continue };
                    let actor = self.cast.get(who);
                    if let Some(m) = actor.and_then(|a| a.matrix) {
                        self.weapons.last.insert(who, m);
                    }
                    let matrix = self.weapons.last.get(&who).copied();
                    let frame = PlayerFrame { bounds: frame.bounds, player: frame.player };
                    let mut w = World { actor, matrix, worlds: None, frame, camera };
                    let mut out = Vec::new();
                    c.ctrl(&at, &mut w, &mut out);
                    for o in out {
                        match o {
                            WeaponOut::Trail { polys, lines, .. } => {
                                let t = Trail { polys: polys.unwrap_or_default(), lines: lines.unwrap_or_default() };
                                self.weapons.pairs += (t.polys.len() + t.lines.len()) as u64;
                                self.trails.push(t);
                            }
                            WeaponOut::Rad(RadOut::Draw(rays)) => {
                                self.weapons.flashes += 1;
                                self.rays.push(rays);
                            }
                            WeaponOut::Rad(_) => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// A race constructor's `ccEnemyWeaponCtrl` (`entry::Out::Weapon`):
    /// the controller over its rows.
    pub(super) fn weapon_made(&mut self, who: usize, info: InfoRef, n: i32) {
        let rows = blocks::weapons(self.data.volume, info, n);
        self.weapons.ctrl.insert(who, WeaponCtrl::new(rows));
    }
}
