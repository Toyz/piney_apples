//! `BIRD` (gcmn 0x005d1440-0x005d17c8): field type 10's hawk by day out
//! of the rain, `CMP_sfp8how1` of `field_eff`, circling out and back from
//! its home over the dungeon's entrance, on charLayer.

use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;

use piney_data::Result;
use piney_desktop::assets::SceneFile;

use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, StepCx};
use crate::draw::{self, Fogging};
use crate::ee::{self, F, ONE, V4};
use crate::field::ambient::Env;

/// BIRD's clump in `field_eff`.
const CLUMP: &str = "CMP_sfp8how1";
/// The turn a frame (0.0314).
const TURN: F = 0x3d00_adfd;

/// One `BIRD`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hawk {
    /// +0x10 the turn a frame, +0x40 the heading.
    pub turn: F,
    pub angle: F,
    /// +0x20 home, +0x30 place.
    pub home: V4,
    pub pos: V4,
    /// +4: farther than 9600 by its own reckoning (never read).
    pub far: bool,
}

impl Hawk {
    /// `Init(stream, "CMP_sfp8how1", 0)` then `SetPos(x, y)`.
    pub fn new(x: F, y: F) -> Hawk {
        let home = [x, y, 0x4448_0000, 0];
        Hawk { turn: TURN, angle: ee::PI, home, pos: home, far: false }
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
                self.turn = TURN | 0x8000_0000;
                self.pos[..3].copy_from_slice(&self.home[..3]);
            }
        } else if ee::le(self.angle, ee::neg(ee::PI)) {
            self.angle = ee::sub(self.angle, self.turn);
            self.turn = TURN;
            self.pos[..3].copy_from_slice(&self.home[..3]);
        }
        m
    }
}

/// The hawk (none by night or in the rain), its clump's models, and where
/// the last `Move` put it.
#[derive(Clone)]
pub struct Bird {
    pub hawk: Option<Hawk>,
    eff: Option<Rc<SceneFile>>,
    models: Vec<u32>,
    pub matrix: [V4; 4],
}

impl Bird {
    /// `Generate`'s, by day out of the rain, home at the dungeon's
    /// entrance (`WORLD_MAN` +0x460).
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<Bird> {
        let f = cx.field.expect("a field's BIRD");
        let w = f.weather;
        let area = f.field;
        let hawk = (w.time() != 2 && !w.rains())
            .then(|| area.and_then(|a| a.dungeon_pos()).unwrap_or([0; 4]))
            .map(|e| Hawk::new(e[0], e[1]));
        let eff = area.and_then(|a| a.eff.clone()).filter(|_| hawk.is_some());
        let models = eff.as_ref().map_or_else(Vec::new, |e| {
            let nodes: Vec<u32> = e
                .ccs
                .find_object(CLUMP)
                .and_then(|o| e.scene.clumps.iter().find(|(c, _)| *c == o))
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
            let mut m: Vec<u32> = nodes.iter().filter_map(|n| e.obj_model.get(n).copied()).collect();
            m.sort_unstable();
            m.dedup();
            m
        });
        Ok(Bird { hawk, eff, models, matrix: [[0; 4]; 4] })
    }
}

impl MapSystem for Bird {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let Some(h) = &mut self.hawk else { return };
        self.matrix = h.step(&Env::of(cx));
        cx.put(Kind::Bird, Piece::Clump(0));
    }

    /// `ccClump::Draw` on charLayer.
    fn draw<'a>(&'a self, _: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        Some(Request::Models {
            file: self.eff.as_ref()?,
            models: Cow::Borrowed(&self.models),
            world: draw::mat(&self.matrix),
            layer: crate::field::area::ambient_layer(5),
            rows: HashMap::new(),
            fog: Fogging::None,
            alpha: 1.0,
        })
    }
}
