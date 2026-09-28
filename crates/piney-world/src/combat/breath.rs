//! The enemies' fire breath ([`piney_battle::breath`]): the breaths a race's
//! constructor makes, asked for a flame ([`Call::BreathSet`]) and run
//! ([`Call::BreathCtrl`]) from the race's `exclusive()`. Like the weapon
//! trails they run in `Combat::fx_call`, so the flames' bursts draw
//! `ccRand()` there, not inside the enemy's frame: the enemies after a
//! bursting breath see `ccRand` a few draws on. The flames draw as ambient
//! sprites ([`Combat::breath_ops`]), the bursts as `effSmoke` puffs.

use std::collections::HashMap;

use piney_battle::blocks::{self, InfoRef};
use piney_battle::breath::{Breath, BreathOut, BreathWorld};
use piney_battle::enemy_ai::Frame;
use piney_battle::enemy_motion::Call;
use piney_battle::entry;
use piney_battle::geom::{F, M4};

use super::Combat;
use super::stage::{PlayerFrame, Show};
use crate::ee::V4;
use crate::field_ambient::{Op, Sprite};
use crate::hit::Hits;

/// A flame as the frame drew it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flame {
    pub file: &'static str,
    pub eff: &'static str,
    pub clut: Option<&'static str>,
    pub pos: V4,
    pub pattern: u16,
    pub scale: [F; 2],
    pub rotate: F,
    pub alpha: F,
}

/// The enemies' breaths by (scene index, slot), and the frame's flames
/// and bursts.
#[derive(Clone, Debug, Default)]
pub struct Breaths {
    pub map: HashMap<(usize, usize), Breath>,
    pub flames: Vec<Flame>,
    pub smoke: Vec<Op>,
    /// For tests: the flames drawn and the bursts so far.
    pub drawn: u64,
    pub bursts: u64,
}

/// A name kept for the frames to come (the ambient sprites name their
/// file and chunk by `&'static str`).
fn keep(s: &str) -> &'static str {
    thread_local! {
        static KEPT: std::cell::RefCell<HashMap<String, &'static str>> = Default::default();
    }
    KEPT.with(|k| *k.borrow_mut().entry(s.to_string()).or_insert_with(|| Box::leak(s.to_string().into_boxed_str())))
}

struct World<'a> {
    hits: &'a mut Hits,
    frame: PlayerFrame,
}

impl BreathWorld for World<'_> {
    fn line(&mut self, a: V4, b: V4, mask: u32) -> F {
        match self.hits.line(a, b, mask, 1) {
            Some((_, d)) => d,
            None => piney_battle::geom::MINUS_ONE,
        }
    }

    fn land(&mut self, p: V4, mask: u32) -> F {
        self.hits.land(p, mask)
    }

    fn w2p(&mut self, p: V4) -> V4 {
        self.frame.w2p(p)
    }

    fn p2w(&mut self, p: V4) -> V4 {
        self.frame.p2w(p)
    }
}

impl Combat {
    /// The shows from `from` on: each breath made, asked for a flame and
    /// run as its enemy's `exclusive()` did.
    pub(super) fn breath_pass(&mut self, from: usize, hits: &mut Hits, frame: &PlayerFrame) {
        for i in from..self.shows.len() {
            match self.shows[i] {
                Show::Entry(entry::Out::Breath { who, slot, info }) => self.breath_made(who, slot, info),
                Show::Entry(entry::Out::Destroyed { who, .. }) => {
                    self.breaths.map.retain(|&(w, _), _| w != who);
                }
                Show::Enemy(who, Call::BreathSet { slot, param, disp, transparency, act_cnt }) => {
                    let param = blocks::br_param(self.data.volume, param);
                    let Some(node) = self.breaths.map.get(&(who, slot)).map(|b| b.info.node.clone()) else {
                        continue;
                    };
                    let mut m = self.node_world(who, &node).unwrap_or_else(piney_battle::geom::unit_matrix);
                    if let Some(b) = self.breaths.map.get_mut(&(who, slot)) {
                        b.set(&param, disp, transparency, act_cnt, &mut m);
                    }
                }
                Show::Enemy(who, Call::BreathCtrl { slot }) => {
                    let file = self.cast.get(who).map(|a| keep(&a.ch.body.file.stem));
                    let Some(b) = self.breaths.map.get_mut(&(who, slot)) else { continue };
                    let mut w = World { hits, frame: PlayerFrame { bounds: frame.bounds, player: frame.player } };
                    let mut out = Vec::new();
                    b.ctrl(&mut w, &mut self.cc, &mut out);
                    let eff = keep(&b.info.eff);
                    let clut = (!b.info.clut.is_empty()).then(|| keep(&b.info.clut));
                    for o in out {
                        match o {
                            BreathOut::Draw { pos, scale, rotate, alpha, pattern, .. } => {
                                self.breaths.drawn += 1;
                                if let Some(file) = file {
                                    self.breaths.flames.push(Flame {
                                        file,
                                        eff,
                                        clut,
                                        pos,
                                        pattern,
                                        scale,
                                        rotate,
                                        alpha,
                                    });
                                }
                            }
                            BreathOut::Smoke { pos, vel, size, n, kind, fade_in, fade_out } => {
                                self.breaths.bursts += 1;
                                let (fade_in, fade_out) = (fade_in as u16, fade_out as u16);
                                self.breaths.smoke.push(Op::Smoke {
                                    pos,
                                    v: vel,
                                    s: size,
                                    life: n,
                                    t: kind,
                                    fade_in,
                                    fade_out,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// A race constructor's breath (`entry::Out::Breath`).
    pub(super) fn breath_made(&mut self, who: usize, slot: usize, info: InfoRef) {
        let info = blocks::br_info(self.data.volume, info);
        self.breaths.map.insert((who, slot), Breath::new(info));
    }

    /// The frame's flames as ambient sprites (layer 3, `ccEff::Init(chunk,
    /// 0)`: no fog; the palette chunk set outright), then the bursts'
    /// puffs, taken.
    pub fn breath_ops(&mut self) -> Vec<Op> {
        let mut ops: Vec<Op> = self
            .breaths
            .flames
            .drain(..)
            .map(|f| {
                Op::Sprite(Sprite {
                    layer: 3,
                    file: f.file,
                    name: f.eff,
                    pos: f.pos,
                    pattern: f.pattern,
                    scale: f.scale,
                    rotate: f.rotate,
                    transparency: f.alpha,
                    fog: false,
                    ztest: true,
                    colour: None,
                    clut: f.clut.map(|c| (c, "")),
                })
            })
            .collect();
        ops.append(&mut self.breaths.smoke);
        ops
    }

    /// The world matrix of enemy `who`'s node `name` as it was last drawn.
    pub(super) fn node_world(&self, who: usize, name: &str) -> Option<M4> {
        let a = self.cast.get(who)?;
        let body = &a.ch.body;
        let node = body.node(name)?;
        let m = a.matrix.or_else(|| self.weapons.last.get(&who).copied())?;
        let worlds = body.worlds(&a.ch.play, super::stage::mat4(&m));
        let w = worlds.get(&node)?;
        Some(std::array::from_fn(|c| w.col(c).to_array().map(f32::to_bits)))
    }
}
