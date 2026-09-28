//! Dun Loireag's stray dogs: `ccSetDog` (gcmn 0x00509f30) and `ccDog`
//! (0x00509390, over `ccGimmick`), `npcTbl` rows 141-144 (Johnny, Sal,
//! Lottel, Suzie; base type 0x04000000, `CDOGBOD1.CCS`).
//!
//! ```text
//! ccSetDog         area 0, town 1 only: inuNum 0, then four entries (type
//!                  2, rows 141-144, entRoot -1), each at the first dummy
//!                  of markPosTbl[inuNum] (the constructor counts inuNum up)
//! ccDog::ccDog     the file's CMP_trall, anm inuAnmTbl[0], note callback
//!                  inuCheckNote; bodyHit radius 65, height 120, kind 2,
//!                  mask2 0x40000001, HitEnable; act 3; the route
//!                  markPosTbl[n], the target its second dummy, the heading
//!                  toward it; affectFunc dogAction
//! ccDog::main      posP; by act (+0x204, +0x210 its step):
//!                    3  anm run0 once, then move (runs, 12 a frame)
//!                    4  anm wal0 once, then move (walks, 1.5 a frame)
//!                    1  anm nut1 once, turning to face Kite (ccSetDirc 64)
//!                    2  anm nut0 once, the same
//!                    5  anm nut1, 101 frames, then act +0x20c (3 or 4)
//!                  mask2 0x40000001 nearer Kite than 300, else 1;
//!                  CollisionDetection: pushed out (pos and posP), the
//!                  touch flag and count (+0x218, +0x21a) - else both 0;
//!                  the anm forward, NoteProcess, ccChar::Draw
//! ccDog::move      the turn toward the target by a sixteenth
//!                  (ccGetDircChg 256); within 4096 of it:
//!                    target within 80 or stuck (+0x21c past 120): at the
//!                    target (nearer than 80) rand() & 1 picks walk (4) or
//!                    run (3), through act 5 when it changes, and the next
//!                    dummy (the list wraps); else back to the last one;
//!                    +0x21c 0
//!                    else a step along the heading to the target, turned
//!                    -0.5 in the first 40 frames of a touch and +0.5 in
//!                    60-99 (the count wraps at 120)
//!                  the ground (0x20000002) and its attribute; every 120
//!                  frames, not 50 from where it was 120 frames ago in x, y
//!                  and z: stuck
//! dogAction        affectType 0: act 4; 14: act 1; 15: act 2 (step 0)
//! inuCheckNote     a note 1 or 2 of param p: p != 0 ccSeSetParamInu(p),
//!                  p == 1 also ccDog::effect
//! ccDog::effect    running and more than 0.05 seen: effSmoke(pos, Rz(dirc)
//!                  (0, 0.1 (0.075 (-10) (10 - rand() % 5)), 0, 1), 3.0, 10,
//!                  1, 512, 32)
//! ```
//!
//! The action button opens `NorainuMenu` (44) on a dog, which calls
//! `EntryAffect` 14 (it sits and faces Kite) and 0 when shut (it walks
//! on); Talk's `TalkMenu` calls 15.

use std::rc::Rc;
use std::sync::Arc;

use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::volume::Volume;
use piney_desktop::assets::SceneFile;

use crate::body::{Body, TRALL};
use crate::char::{Char, w2p};
use crate::ee::{self, F, ONE, V4, add, cosf, from_int, lt, mul, sinf, sub, vadd};
use crate::entry::{Npc, NpcCtx};
use crate::hit::{self, WALL_MASK};
use crate::merchant::EntryObj;
use crate::npc::NpcRow;
use crate::rtownpc::{get_dirc, get_dirc_chg, get_dist3d, set_dirc};

/// `npcTbl`'s first dog row; the four are 141-144.
pub const ROW: usize = 141;
pub const COUNT: usize = 4;
/// The town they are in (`game.town` 1, Dun Loireag).
pub const TOWN: i32 = 1;

/// `inuAnmTbl` (gcmn 0x005ee180).
pub const ANM: [&str; 4] = ["ANM_cdg1nut0", "ANM_cdg1nut1", "ANM_cdg1run0", "ANM_cdg1wal0"];

/// `markPosTbl` (gcmn 0x005ee220): each dog's route, `markerTbl1`-`4`,
/// the town's dummies walked in turn (back to the first after the last).
pub const ROUTES: [&[&str]; COUNT] = [
    &[
        "DMY_marker46",
        "DMY_marker45",
        "DMY_marker44",
        "DMY_marker09",
        "DMY_marker46",
        "DMY_marker67",
        "DMY_marker10",
        "DMY_marker49",
        "DMY_marker67",
    ],
    &[
        "DMY_marker03",
        "DMY_marker06",
        "DMY_marker48",
        "DMY_marker47",
        "DMY_marker08",
        "DMY_marker07",
        "DMY_marker05",
        "DMY_marker04",
    ],
    &["DMY_marker18", "DMY_marker19", "DMY_marker20"],
    &["DMY_marker26", "DMY_marker27", "DMY_marker28", "DMY_marker29", "DMY_marker28", "DMY_marker27"],
];

/// The dogs' bodies in the character list: [`BODY_ID`] plus the dog's
/// number (the port's keys).
pub const BODY_ID: u32 = 0x200;
/// `bodyHit`: radius 65, height 120, kind 2.
const RADIUS: F = 0x4282_0000;
const HEIGHT: F = 0x42f0_0000;
const KIND: u32 = 2;
/// Walls only nearer Kite than 300.
const NEAR_KITE: F = 0x4396_0000;
/// At the target within 80.
const ARRIVE: F = 0x42a0_0000;
/// A walk's and a run's step.
const WALK: F = 0x3fc0_0000;
const RUN: F = 0x4140_0000;
/// The turn away from a touch.
const HALF: F = 0x3f00_0000;
/// The stuck check's reach.
const MOVED: F = 0x4248_0000;
/// The ground `move` stands the dog on.
const LAND_MASK: u32 = 0x2000_0002;

/// What a dog's frame asks of the rest of the game.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DogEvent {
    /// `ccSeSetParamInu(param, dog)`: at `pos`, on ground `attribute`
    /// (params 0 and 1 are steps).
    Note { param: u32, pos: V4, attribute: u32 },
    /// `effSmoke(pos, v, 3.0, 10, 1, 512, 32)`: a running step's dust.
    Smoke { pos: V4, v: V4 },
}

/// One dog.
#[derive(Clone)]
pub struct Dog {
    pub row: NpcRow,
    pub ch: Char,
    pub entry: EntryObj,
    /// `inuNum` when it was made: its route.
    pub num: usize,
    /// The route's dummies (+0x228 the one walked to, +0x22c the first,
    /// +0x230 the last one reached), as indices into `route`.
    pub route: Vec<V4>,
    pub cur: usize,
    pub prev: usize,
    /// +0x1e0 the target, +0x1f0 where it stood at the last stuck check.
    pub target: V4,
    pub saved: V4,
    /// +0x204 `act`, +0x208 last frame's, +0x20c what act 5 goes on to,
    /// +0x210 the act's step, +0x214 act 5's count.
    pub act: i32,
    pub act_old: i32,
    pub act_next: i32,
    pub step: i32,
    pub cnt: i32,
    /// +0x218 touching, +0x21a for how long, +0x21c the stuck count.
    pub hit_flag: i16,
    pub hit_cnt: i16,
    pub stuck: i16,
    /// +0x9c `affectType`: the command `EntryAffect` last passed.
    pub affect_type: i16,
    /// What this frame asked for.
    pub events: Vec<DogEvent>,
}

/// The position of dummy `name` in `file`.
fn dummy(file: &SceneFile, name: &str) -> Option<V4> {
    let d = file.ccs.find_object(name).and_then(|o| file.scene.dummies.get(&o))?;
    Some([d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE])
}

impl Dog {
    /// `ccEntryCtrl::entryObject` and `ccDog::ccDog` for dog `num` (row
    /// 141 + `num`) with `body`, on its route through `town_file`'s
    /// dummies; its body onto the character list of `hits`.
    pub fn new(row: &NpcRow, body: Rc<Body>, num: usize, town_file: &SceneFile, hits: &mut hit::Hits) -> Option<Dog> {
        let route: Vec<V4> = ROUTES.get(num)?.iter().map(|n| dummy(town_file, n).unwrap_or(ee::VF0)).collect();
        let pos = *route.first()?;
        let target = *route.get(1)?;
        let dirc = [0, 0, get_dirc(pos, target), ONE];
        let mut ch = Char::new(body, ANM[0], pos, dirc, row.height, row.width)?;
        ch.hit = hit::Body {
            pos,
            radius: RADIUS,
            height: HEIGHT,
            mask2: WALL_MASK,
            kind: KIND,
            id: BODY_ID + num as u32,
            ..hit::Body::default()
        };
        hits.hit_enable(&mut ch.hit);
        Some(Dog {
            row: row.clone(),
            ch,
            entry: EntryObj::default(),
            num,
            route,
            cur: 1,
            prev: 0,
            target,
            saved: pos,
            act: 3,
            act_old: 3,
            act_next: -1,
            step: 0,
            cnt: 0,
            hit_flag: 0,
            hit_cnt: 0,
            stuck: 0,
            affect_type: 0,
            events: Vec::new(),
        })
    }

    fn set_anm(&mut self, k: usize) {
        self.ch.set_anim(ANM[k]);
    }

    /// `ccDog::main` (0x00509680) after `routine`.
    pub fn main(&mut self, ctx: &mut NpcCtx) {
        self.ch.pos_p = w2p(self.ch.pos, ctx.player);
        let face = |d: &mut Dog| {
            let mut z = d.ch.dirc[2];
            set_dirc(&mut z, d.entry.pl_dirc, 64);
            d.ch.dirc[2] = z;
        };
        match self.act {
            3 | 4 => {
                if self.step == 0 {
                    self.set_anm(if self.act == 3 { 2 } else { 3 });
                    self.step += 1;
                }
                self.walk(ctx);
            }
            1 | 2 => {
                if self.step == 0 {
                    self.set_anm(if self.act == 1 { 1 } else { 0 });
                    self.step += 1;
                }
                face(self);
            }
            5 => match self.step {
                0 => {
                    self.set_anm(1);
                    self.step += 1;
                    self.cnt = 0;
                }
                1 => {
                    if self.cnt >= 101 {
                        self.step = 2;
                    }
                    self.cnt += 1;
                }
                2 if matches!(self.act_next, 3 | 4) => {
                    self.act = self.act_next;
                    self.step = 0;
                }
                _ => {}
            },
            _ => {}
        }
        self.ch.hit.mask2 = if lt(self.entry.pl_dist, NEAR_KITE) { WALL_MASK } else { 1 };
        self.ch.hit.pos = self.ch.pos;
        if ctx.hits.collision_detection(&mut self.ch.hit) != 0 {
            let off = self.ch.hit.offset;
            self.ch.pos = vadd(self.ch.pos, off);
            self.ch.pos_p = vadd(self.ch.pos_p, off);
            self.ch.hit.pos = self.ch.pos;
            ctx.hits.sync(&self.ch.hit);
            self.hit_flag = 1;
            self.hit_cnt = self.hit_cnt.wrapping_add(1);
        } else {
            self.hit_flag = 0;
            self.hit_cnt = 0;
        }
        // _AnimateForward, SetMatrix_PosRotZYX, NoteProcess (inuCheckNote).
        let (_, notes) = self.ch.play.forward_notes(&self.ch.body.file);
        for (event, param) in notes {
            if !matches!(event, 1 | 2) {
                continue;
            }
            if param != 0 {
                self.events.push(DogEvent::Note { param, pos: self.ch.pos, attribute: self.ch.hit_attribute });
            }
            if param == 1 {
                self.effect(ctx.rand);
            }
        }
        // ccChar::Draw.
        self.ch.fade(&ctx.view);
        self.act_old = self.act;
    }

    /// `ccDog::move` (0x00509a80).
    fn walk(&mut self, ctx: &mut NpcCtx) {
        let pos = self.ch.pos;
        let dist = get_dist3d(pos, self.target);
        let mut dir = get_dirc(pos, self.target);
        let s2 = ee::rad2deg(dir);
        let s1 = ee::rad2deg(self.ch.dirc[2]);
        let s1 = (i32::from(s1) + get_dirc_chg(s1, s2, 256)) as i16;
        self.ch.dirc[2] = ee::deg2rad(s1);
        if ((i32::from(s2 as u16) - i32::from(s1 as u16) + 4096) & 0xffff) < 8192 {
            if !ee::le(dist, ARRIVE) && self.stuck < 121 {
                if self.hit_flag != 0 {
                    let c = self.hit_cnt;
                    if c < 40 {
                        dir = sub(dir, HALF);
                    } else if (60..100).contains(&c) {
                        dir = add(dir, HALF);
                    } else if c >= 120 {
                        self.hit_cnt = 0;
                    }
                }
                let v = if self.act == 4 { WALK } else { RUN };
                self.ch.pos[0] = add(self.ch.pos[0], mul(v, sinf(dir)));
                self.ch.pos[1] = sub(self.ch.pos[1], mul(v, cosf(dir)));
            } else {
                if lt(dist, ARRIVE) {
                    let next = if ctx.rand.rand() & 1 != 0 { 4 } else { 3 };
                    if self.act != next {
                        self.act = 5;
                        self.act_next = next;
                        self.step = 0;
                    }
                    self.prev = self.cur;
                    self.cur += 1;
                    if self.cur >= self.route.len() {
                        self.cur = 0;
                    }
                } else {
                    std::mem::swap(&mut self.cur, &mut self.prev);
                }
                self.target = self.route[self.cur];
                self.stuck = 0;
            }
        }
        // ccTransPosW2M (no wrap in a town) and the ground.
        let mut p = self.ch.pos;
        p[3] = ONE;
        self.ch.pos[2] = ctx.hits.land(p, LAND_MASK);
        self.ch.hit_attribute = ctx.hits.attribute();
        if self.stuck == 120 {
            let far = |a: F, b: F| {
                let d = sub(a, b);
                !ee::le(d, MOVED) || lt(d, ee::neg(MOVED))
            };
            if (0..3).any(|k| far(self.ch.pos[k], self.saved[k])) {
                self.stuck = 0;
            }
        }
        if self.stuck == 0 {
            self.saved = self.ch.pos;
        }
        self.stuck += 1;
    }

    /// `ccDog::effect` (0x0050a2e0): a running step's dust.
    fn effect(&mut self, rand: &mut crate::Rand) {
        if ee::le(self.ch.transparency, 0x3d4c_cccd) || self.act != 3 {
            return;
        }
        let m = crate::evcam::rot_z(&[[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]], self.ch.dirc[2]);
        let p = [self.ch.pos[0], self.ch.pos[1], self.ch.pos[2], ONE];
        let r = rand.rand() % 5;
        let s = mul(0xbf80_0000, 0x4120_0000);
        let y = mul(0x3dcc_cccd, mul(mul(0x3d99_999a, s), from_int(10 - r)));
        let v = ee::apply(&m, [0, y, 0, ONE]);
        self.events.push(DogEvent::Smoke { pos: p, v });
    }

    /// `dogAction` (0x0050a1f0), the dog's `affectFunc`.
    pub fn action(&mut self, cmd: i16) {
        self.affect_type = cmd;
        let act = match cmd {
            0 => 4,
            14 => 1,
            15 => 2,
            _ => return,
        };
        self.act = act;
        self.step = 0;
    }
}

impl Npc for Dog {
    fn code(&self) -> i32 {
        i32::from(self.row.id)
    }

    fn flags(&self) -> u32 {
        self.row.flags
    }

    fn char(&self) -> &Char {
        &self.ch
    }

    fn char_mut(&mut self) -> &mut Char {
        &mut self.ch
    }

    /// `ccEntryObj::routine`, then `ccDog::main`.
    fn step(&mut self, ctx: &mut NpcCtx) {
        self.events.clear();
        self.entry.routine(&mut self.ch, ctx.player);
        self.main(ctx);
    }

    fn listed(&self) -> bool {
        self.entry.listed
    }

    fn influence(&mut self, cmd: i16) {
        self.action(cmd);
    }
}

/// `ccSetDog` (gcmn 0x00509f30) in town `town` (area 0): the four dogs,
/// in Dun Loireag only; their file `CDOGBOD1.CCS`.
pub fn set_dogs(
    archive: &Arc<Archive>,
    volume: Volume,
    town_file: &SceneFile,
    hits: &mut hit::Hits,
    town: i32,
) -> Result<Vec<Dog>> {
    if town != TOWN {
        return Ok(Vec::new());
    }
    let mut body: Option<Rc<Body>> = None;
    let mut out = Vec::new();
    for num in 0..COUNT {
        let row = NpcRow::of(volume, ROW + num)?;
        let b = match &body {
            Some(b) => b.clone(),
            None => {
                let b = Rc::new(Body::read(archive, &row.stem(), TRALL)?);
                body = Some(b.clone());
                b
            }
        };
        if let Some(d) = Dog::new(&row, b, num, town_file, hits) {
            out.push(d);
        }
    }
    Ok(out)
}
