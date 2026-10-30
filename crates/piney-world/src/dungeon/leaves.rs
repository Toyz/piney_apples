//! Type 32's tree and falling leaves (`se4_2`'s event room, area 101):
//! `DUNGEON::SetRoom` makes `ANM_se4_2lea` at the room's centre (+0xd38bc)
//! and twenty `LEAF`s (+0xd38c0, INF gcmn 0x00505050); `DrawEff` steps and
//! draws them after the fireflies (0x005ce108); `DeleteRoom` frees them.

use glam::Mat4;
use piney_battle::rand::Genrand;
use piney_data::anim::{rot_x_bits_of, rot_z_bits_of};
use piney_desktop::layers::Layers;
use piney_draw::DepthFog;

use super::{Anm, DungeonArea};
use crate::draw::{self, Draw, EFF_LAYER};
use crate::ee::{self, F, ONE, V4};

/// The tree's animation and the leaves' model, in the room's file (`spccs`).
pub const TREE_ANM: &str = "ANM_se4_2lea";
pub const LEAF_MODEL: &str = "MDL_se4_2lea";
pub const LEAVES: usize = 20;
/// The `EditDungeon` row type that has them.
pub const LEAF_ROOM: i32 = 32;
/// `LEAF` +0x08 and +0x0c: what `calcLeafPos` scatters by (15.0, 0.6).
const SPREAD: F = 0x4170_0000;
const FALL: F = 0x3f19_999a;
/// The spin's step either way (0.05).
const SPIN: F = 0x3d4c_cccd;
/// A leaf below this height starts again from the top (300.0).
const FLOOR: F = 0x4396_0000;

/// One `LEAF` (0x50 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct Leaf {
    /// +0x04: frames until the spin's direction is drawn again.
    pub timer: i32,
    /// +0x10 the spin's step, +0x14 the turn about z, +0x18 about x
    /// (`ccRandF() * -2`, owed to the caller until it draws it).
    pub spin: F,
    pub rot_z: F,
    pub rot_x: Option<F>,
    /// +0x20 the room's centre, +0x30 the place about it, +0x40 the step.
    pub centre: V4,
    pub off: V4,
    pub vel: V4,
}

/// The tree and its leaves.
#[derive(Clone, Debug)]
pub struct Leaves {
    pub tree: Anm,
    pub leaves: Vec<Leaf>,
}

/// `fieldrand(n)` as a float (`cvt.s.w`; never past 2^31 here).
fn rand_f(rng: &mut piney_data::dungeon::Rng, n: u32) -> F {
    ee::from_int(rng.below(n) as i32)
}

/// `calcLeafPos(off, vel, 15.0, 0.6)` (INF gcmn 0x00504da0): a place 50
/// either way and 400-999 up, falling `fieldrand(5) + 1` times 0.6 a
/// frame and drifting `(fieldrand(15) - 7.5) / 10` each way.
fn calc_leaf_pos(rng: &mut piney_data::dungeon::Rng) -> (V4, V4) {
    const FIFTY: F = 0x4248_0000;
    const TEN: F = 0x4120_0000;
    let x = ee::sub(rand_f(rng, 100), FIFTY);
    let y = ee::sub(rand_f(rng, 100), FIFTY);
    let z = ee::from_int(rng.below(600) as i32 + 400);
    let n = ee::to_int(SPREAD) as u32;
    let half = ee::div(SPREAD, 0x4000_0000);
    let vx = ee::div(ee::sub(rand_f(rng, n), half), TEN);
    let vy = ee::div(ee::sub(rand_f(rng, n), half), TEN);
    let fall = ee::from_int(rng.below(5) as i32 + 1);
    let vz = ee::mul(ee::mul(0xbf80_0000, fall), FALL);
    ([x, y, z, ONE], [vx, vy, vz, ONE])
}

/// The spin's step: -0.05 when the draw is at most 50, else 0.05.
fn spin_of(r: u32) -> F {
    if r < 51 { SPIN | 0x8000_0000 } else { SPIN }
}

impl Leaf {
    /// `LEAF::LEAF(spccs, centre)`: its timer, spin and place
    /// (`fieldrand`), its tilt owed (`ccRandF`).
    fn new(rng: &mut piney_data::dungeon::Rng, centre: V4) -> Leaf {
        let timer = rng.below(20) as i32 + 10;
        let spin = spin_of(rng.below(100));
        let (off, vel) = calc_leaf_pos(rng);
        Leaf { timer, spin, rot_z: 0, rot_x: None, centre, off, vel }
    }

    /// `LEAF::Move` (0x00505360): the step added (w back to 1); under 300
    /// up it starts again.
    fn step(&mut self, rng: &mut piney_data::dungeon::Rng) {
        self.off = std::array::from_fn(|k| ee::add(self.off[k], self.vel[k]));
        self.off[3] = ONE;
        if ee::lt(self.off[2], FLOOR) {
            (self.off, self.vel) = calc_leaf_pos(rng);
        }
    }

    /// `LEAF::Draw`'s matrix (0x005051f0): turned about z, then x, at the
    /// centre plus its place.
    fn matrix(&self) -> [V4; 4] {
        let unit = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];
        let mut m = rot_z_bits_of(unit, self.rot_z);
        m = rot_x_bits_of(m, self.rot_x.unwrap_or(0));
        for ((t, c), o) in m[3].iter_mut().zip(self.centre).zip(self.off).take(3) {
            *t = ee::add(*t, ee::add(c, o));
        }
        m
    }

    /// `LEAF::Draw`'s turn after the matrix: z on by the spin; at the
    /// timer's end a new timer and the spin back toward 0.
    fn turn(&mut self, rng: &mut piney_data::dungeon::Rng) {
        self.rot_z = ee::add(self.rot_z, self.spin);
        self.timer -= 1;
        if self.timer == 0 {
            self.timer = rng.below(20) as i32 + 10;
            self.spin = if ee::lt(self.rot_z, 0) { SPIN } else { SPIN | 0x8000_0000 };
        }
    }
}

impl DungeonArea {
    /// The `EditDungeon` row type of an event room at (f, i).
    pub(super) fn event_room_type(&self, f: usize, i: usize) -> Option<i32> {
        let e = self.edit?;
        e.rooms.iter().find(|r| r.room_type >= 16 && r.floor == f as i32 && r.index == i as i32).map(|r| r.room_type)
    }

    /// `SetRoom`'s type 32 (INF gcmn 0x005c2fe8): the tree at the room's
    /// centre (`ccAnm::SetAnm` of `spccs`, one step on), then the twenty
    /// leaves; each owes a `ccRandF` ([`DungeonArea::settle_leaves`]).
    pub(super) fn set_leaves(&mut self, centre: [F; 2]) {
        // The room's centre; its w is whatever SetRoom's stack held, which
        // nothing reads (0 here).
        let centre = [centre[0], centre[1], 0, 0];
        let mut root = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];
        for (t, c) in root[3].iter_mut().zip(centre).take(3) {
            *t = ee::add(*t, c);
        }
        let Some(tree) = self.make_anm_in(true, TREE_ANM, &root, 1) else { return };
        let leaves = (0..LEAVES).map(|_| Leaf::new(&mut self.rng, centre)).collect();
        self.dress.leaves = Some(Leaves { tree, leaves });
        self.leaf_rand_owed += LEAVES as u32;
    }

    /// The `ccRandF()`s (`genrand` as a signed int over 2^32, times -2)
    /// the leaves made since the last call owe, drawn from `cc` in their
    /// order: the leaves standing take the last of them.
    pub fn settle_leaves(&mut self, cc: &mut Genrand) {
        let owed = std::mem::take(&mut self.leaf_rand_owed) as usize;
        let draws: Vec<u32> = (0..owed).map(|_| cc.next_u32()).collect();
        let Some(l) = self.dress.leaves.as_mut() else { return };
        let fresh: Vec<&mut Leaf> = l.leaves.iter_mut().filter(|x| x.rot_x.is_none()).collect();
        let tail = draws.len().saturating_sub(fresh.len());
        for (leaf, &g) in fresh.into_iter().zip(&draws[tail..]) {
            let f = ee::div(ee::from_int(g as i32), 0x4f80_0000);
            leaf.rot_x = Some(ee::mul(0xc000_0000, f));
        }
    }

    /// `DrawEff`'s last part's steps (0x005ce108): the tree one frame on,
    /// then each leaf's `Move` and its `Draw`'s matrix and turn; the
    /// leaves' matrices. Without `step`, only the matrices.
    pub fn step_leaves(&mut self, step: bool) -> Vec<[V4; 4]> {
        let Some(mut l) = self.dress.leaves.take() else { return Vec::new() };
        if step {
            let file = self.src(true).0.clone();
            l.tree.play.forward(&file);
            l.tree.objects = self.pose_with(true, &l.tree.play, &l.tree.root, None);
        }
        let rng = &mut self.rng;
        let out = l
            .leaves
            .iter_mut()
            .map(|leaf| {
                if !step {
                    return leaf.matrix();
                }
                leaf.step(rng);
                let m = leaf.matrix();
                leaf.turn(rng);
                m
            })
            .collect();
        self.dress.leaves = Some(l);
        out
    }

    /// `DrawEff`'s last part drawn: the tree, then each leaf's model
    /// unfogged at its matrix (PRIM 0xc).
    pub(super) fn draw_leaves(&mut self, layers: &mut Layers, to_screen: Mat4, fog: DepthFog, step: bool) {
        let matrices = self.step_leaves(step);
        let Some(l) = self.dress.leaves.as_ref() else { return };
        self.draw_special_anm(layers, to_screen, &l.tree, fog);
        let file = self.src(true).0;
        let Some(model) = file.ccs.find_object(LEAF_MODEL) else { return };
        let rows = std::collections::HashMap::new();
        for m in &matrices {
            let d = Draw {
                file,
                model,
                world: draw::mat(m),
                alpha: 1.0,
                rows: &rows,
                lights: None,
                nodes: &[],
                morph: Vec::new(),
                clut_swaps: Vec::new(),
            };
            draw::model_edited(layers, EFF_LAYER, to_screen, &d, None, draw::Fogging::None);
        }
    }

    /// The tree's objects on the effects' layer, fogged by depth.
    fn draw_special_anm(&self, layers: &mut Layers, to_screen: Mat4, anm: &Anm, fog: DepthFog) {
        let (file, obj_models, _) = self.src(true);
        let rows = anm.play.uv_rows(file);
        for o in &anm.objects {
            let Some(models) = obj_models.get(&o.target) else { continue };
            for &model in models {
                let d = Draw {
                    file,
                    model,
                    world: draw::mat(&o.world),
                    alpha: 1.0,
                    rows: &rows,
                    lights: None,
                    nodes: &[],
                    morph: Vec::new(),
                    clut_swaps: Vec::new(),
                };
                draw::model_edited(layers, EFF_LAYER, to_screen, &d, None, draw::Fogging::Depth(fog));
            }
        }
    }
}
