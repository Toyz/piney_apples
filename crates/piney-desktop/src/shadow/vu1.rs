//! VU1's shadow programs on a draw's [`Setup`] (VU1 0x5d2-0x758), in VU1's
//! arithmetic ([`piney_data::vu1`]): which directions face the light
//! (`mc_DrawShadow1`), the triangles with the unlit ones stretched
//! (`mc_DrawShadow2`), the silhouette's quads (`mc_DrawShadow3`), and the
//! polygons outside the guard band cut at the near plane and to the scissor
//! (`mc_DrawShadowClip`, `mc_ScissorShadowPolyXY`, `mc_ScissorShadowPoly`),
//! as the [`Kick`]s VU1 sends. What one draw leaves in VU1 memory that a
//! later one reads is [`Vu1Carry`].

use piney_data::field::ee;
use piney_data::shadow::{Edge, ShadowMesh, direction};
use piney_data::vu0::{self, F, ONE, V4};
use piney_data::vu1::{cross_xy, ftoi4, itof12, lerp, plus0, project, sign, vsub};

use super::draw::{Blend, Setup};
use super::gif::{FAN_TAG, Kick, Polygon, Xyz2};

/// Triangles and edges a batch carries.
const BATCH: usize = 24;
/// The window of VU1 memory a later draw reads back by address, 0x1ed to
/// 0x211: the clip path's buffers, the fan's tags and registers, and the
/// three flags the direction loop's pipeline stores first.
const BELOW: usize = 0x1ed;
const DIRECTIONS: usize = 0x212;
const FRONT: usize = 0x1ed;
const BEHIND: usize = 0x1f3;
const PING: usize = 0x1f9;
const FAN_TAGS: [usize; 2] = [0x203, 0x204];
const FAN_REGS: [usize; 2] = [0x205, 0x206];
const FAN_VERTEX_TAG: usize = 0x207;
const PONG: usize = 0x208;
/// The w a vertex of a polygon going to the clip path gets on its last
/// vertex (ADC), and one drawn directly.
const ADC: u32 = 0x8000;
const DRAWN: u32 = 0x7f80;

/// What a draw's VU1 work leaves for the next draw's: the colour register
/// the last fan was given (which the next fan's tag kick sends), and the w
/// words of VU1 0x1ed-0x211, which an open edge's second face reads (at
/// `0x211 - face`). VU1's other programs, which run between a frame's
/// shadow passes, are not followed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vu1Carry {
    last_colour: Option<u64>,
    below: [u32; DIRECTIONS - BELOW],
}

impl Default for Vu1Carry {
    fn default() -> Self {
        Vu1Carry { last_colour: None, below: [0; DIRECTIONS - BELOW] }
    }
}

/// `mc_DrawShadow1`'s test: whether `(L.x d.x + L.z d.z) + (0 + L.y d.y)`
/// is negative (the face looks at the light), stored as a w of 0x80 or 0.
fn lit(light: V4, d: [F; 3]) -> u32 {
    let x = ee::add(ee::mul(light[0], d[0]), ee::mul(light[2], d[2]));
    if sign(ee::add(x, ee::add(0, ee::mul(light[1], d[1])))) { 0x80 } else { 0 }
}

/// A vertex as VU1 unpacks it: `ITOF12` of the stored s16s, w 1.
fn vertex(mesh: &ShadowMesh, i: u32) -> V4 {
    let [x, y, z] = mesh.positions.get(i as usize).copied().unwrap_or([0; 3]).map(|c| itof12(c as u32));
    [x, y, z, ONE]
}

/// `ADD.xyz` of the stretch.
fn moved(v: V4, e: V4) -> V4 {
    [ee::add(v[0], e[0]), ee::add(v[1], e[1]), ee::add(v[2], e[2]), v[3]]
}

/// A divided vertex as VU1 stores it, with the w an `ISW` gives it.
fn xyz2(p: V4, w: u32) -> Xyz2 {
    Xyz2 { xyz: [ftoi4(p[0]), ftoi4(p[1]), ftoi4(p[2])], w }
}

/// A draw's VU1 run: the setup, the flags the directions hold, the carried
/// memory, the kicks so far.
struct Run<'a> {
    s: &'a Setup,
    flags: Vec<u32>,
    carry: &'a mut Vu1Carry,
    fan_sent: bool,
    kicks: Vec<Kick>,
}

impl Run<'_> {
    /// The w word at a VU1 address of the window or of the directions; the
    /// memory below the window (an open edge on a face past 36, which no
    /// model has) is not followed and reads 0.
    fn w(&self, at: usize) -> u32 {
        match at.checked_sub(BELOW) {
            Some(k) if k < self.carry.below.len() => self.carry.below[k],
            Some(_) => self.flags.get(at - DIRECTIONS).copied().unwrap_or(0),
            None => 0,
        }
    }

    /// The w words a store of `vs` at `at` leaves.
    fn store(&mut self, at: usize, ws: impl IntoIterator<Item = u32>) {
        for (k, w) in ws.into_iter().enumerate() {
            match (at + k).checked_sub(BELOW) {
                Some(i) if i < self.carry.below.len() => self.carry.below[i] = w,
                Some(_) => {
                    if let Some(f) = self.flags.get_mut(at + k - DIRECTIONS) {
                        *f = w;
                    }
                }
                None => {}
            }
        }
    }

    /// A face's flag as `ILW.w` reads it: the low 16 bits of the w word.
    fn flag(&self, at: usize) -> u32 {
        self.w(at) & 0xffff
    }

    /// Divided vertices out of the guard band: the sticky sign of the
    /// `SUBA.xyw`s between `FSSET` and `FSAND` (the status flag, like the
    /// MAC flag, lands four pairs late, so only they reach the test).
    fn outside(&self, projected: &[V4]) -> bool {
        let [lo, hi] = self.s.guard;
        projected
            .iter()
            .any(|v| [0, 1, 3].into_iter().any(|k| sign(ee::sub(v[k], lo[k])) || sign(ee::sub(hi[k], v[k]))))
    }

    /// A polygon through `M`, divided: its blend by its winding, and
    /// whether it goes to the clip path.
    fn transform<const N: usize>(&self, verts: [V4; N]) -> ([V4; N], [V4; N], Blend, bool) {
        let clip = verts.map(|v| vu0::apply(&self.s.m, v));
        let p = clip.map(project);
        let blend = self.s.blend(cross_xy(vsub(p[1], p[0]), vsub(p[2], p[0])));
        (clip, p, blend, self.outside(&p))
    }

    /// `mc_DrawShadow2` (0x5ef) on a batch of triangles: each with its
    /// vertices moved when its face is unlit.
    fn triangles(&mut self, mesh: &ShadowMesh, batch: &[([u32; 3], u16)]) {
        let mut queue = Vec::new();
        let polys = batch
            .iter()
            .map(|&(t, face)| {
                let unlit = self.flag(DIRECTIONS + usize::from(face)) == 0;
                let (clip, p, blend, out) = self.transform(t.map(|i| {
                    let v = vertex(mesh, i);
                    if unlit { moved(v, self.s.stretch) } else { v }
                }));
                if out {
                    queue.push(clip.to_vec());
                }
                let last = if out { ADC } else { DRAWN };
                Polygon { blend, verts: vec![xyz2(p[0], 0), xyz2(p[1], 0), xyz2(p[2], last)] }
            })
            .collect();
        self.kicks.push(Kick::Triangles(polys));
        self.clip(&queue);
    }

    /// `mc_DrawShadow3` (0x641) on a batch of edges: each whose two faces'
    /// flags differ, the quad `(a, b, a + e, b + e)`, a and b in the order
    /// its first face runs them when that face is lit. An open edge's second
    /// flag is read below the directions, at `0x211 - face`.
    fn edges(&mut self, mesh: &ShadowMesh, batch: &[Edge]) {
        let mut queue = Vec::new();
        let mut polys = Vec::new();
        for edge in batch {
            let first = self.flag(DIRECTIONS + usize::from(edge.face_a));
            let second = match edge.face_b {
                Some(f) => DIRECTIONS + usize::from(f),
                None => (DIRECTIONS - 1).wrapping_sub(usize::from(edge.face_a)),
            };
            if first == self.flag(second) {
                continue;
            }
            let (a, b) = match first {
                0 => (vertex(mesh, edge.v1), vertex(mesh, edge.v0)),
                _ => (vertex(mesh, edge.v0), vertex(mesh, edge.v1)),
            };
            let e = self.s.stretch;
            let (clip, p, blend, out) = self.transform([a, b, moved(a, e), moved(b, e)]);
            if out {
                // Queued as a polygon: a, b, b + e, a + e.
                queue.push(vec![clip[0], clip[1], clip[3], clip[2]]);
            }
            let last = if out { ADC } else { DRAWN };
            let verts = vec![xyz2(p[0], ADC), xyz2(p[1], ADC), xyz2(p[2], last), xyz2(p[3], last)];
            polys.push(Polygon { blend, verts });
        }
        self.kicks.push(Kick::Quads(polys));
        self.clip(&queue);
    }

    /// `mc_DrawShadowClip` (0x6a7): each queued polygon cut at the near
    /// plane's w into the part in front and the part behind (its vertices
    /// there put on the plane: x and y kept, the near point's z and w),
    /// each through [`Self::scissor`]. A crossing is made from the vertex
    /// in front toward the one behind.
    fn clip(&mut self, queue: &[Vec<V4>]) {
        let near = self.s.near;
        let behind = |v: V4| sign(ee::sub(v[3], near[3]));
        for poly in queue {
            let n = poly.len();
            // Copied to 0x1f9, closed by its first vertex.
            self.store(PING, poly.iter().chain(&poly[..1]).map(|v| v[3]));
            let (mut front, mut back) = (Vec::new(), Vec::new());
            for (k, &a) in poly.iter().enumerate() {
                let b = poly[(k + 1) % n];
                let (from, to) = if behind(a) {
                    back.push([a[0], a[1], near[2], near[3]]);
                    (plus0(b), plus0(a))
                } else {
                    front.push(a);
                    (plus0(a), plus0(b))
                };
                if behind(a) != behind(b) {
                    let d = vsub(to, from);
                    let x = lerp(from, d, ee::div(ee::sub(near[3], from[3]), d[3]));
                    front.push(x);
                    back.push(x);
                }
            }
            self.store(FRONT, front.iter().map(|v| v[3]));
            self.store(BEHIND, back.iter().map(|v| v[3]));
            for (part, at) in [(front, FRONT), (back, BEHIND)] {
                if !part.is_empty() {
                    self.scissor(part, at);
                }
            }
        }
    }

    /// `mc_ScissorShadowPolyXY` (0x6e2): the winding from the first three
    /// vertices divided; the fan's tag kicked before its registers are
    /// written; the polygon cut to the scissor (x low, x high, y low, y
    /// high, in homogeneous space, the buffers taking turns at 0x208 and
    /// 0x1f9); then, if anything is left, sent as a TRIFAN.
    fn scissor(&mut self, poly: Vec<V4>, at: usize) {
        let p: Vec<V4> = poly.iter().take(3).map(|&v| project(v)).collect();
        let winding = match p[..] {
            [a, b, c] => cross_xy(vsub(b, a), vsub(c, a)),
            _ => 0,
        };
        self.kicks.push(Kick::FanTag { fan_sent: self.fan_sent, colour: self.carry.last_colour });
        let blend = self.s.blend(winding);
        self.carry.last_colour = Some(blend.colour);
        self.store(FAN_REGS[0], [0, 0]);
        let [lo, hi] = self.s.scissor.map(plus0);
        let passes = [(lo, 0, at, PONG), (hi, 0, PONG, PING), (lo, 1, PING, PONG), (hi, 1, PONG, PING)];
        let mut poly = poly;
        for (bound, lane, from, to) in passes {
            poly = self.cut(&poly, from, to, bound, lane);
            if poly.is_empty() {
                return;
            }
        }
        let verts: Vec<Xyz2> = poly
            .iter()
            .enumerate()
            .map(|(k, &v)| xyz2(project(v), 0x8001u32.wrapping_sub(k as u32) & 0xffff))
            .collect();
        self.store(PONG, verts.iter().map(|v| v.w));
        self.fan_sent = true;
        self.kicks.push(Kick::Fan { blend, verts });
    }

    /// `mc_ScissorShadowPoly` (0x733): the polygon at `from` (closed by a
    /// copy of its first vertex) cut to one side of `bound` in `lane`,
    /// written at `to`. A vertex is out when `bound.lane v.w - v.lane
    /// bound.w` is negative; a crossing runs from the vertex in toward the
    /// one out, `d.lane / (bound.w dv.lane - dv.w bound.lane)` of the way.
    fn cut(&mut self, poly: &[V4], from: usize, to: usize, bound: V4, lane: usize) -> Vec<V4> {
        self.store(from + poly.len(), [poly[0][3]]);
        let gap = |v: V4| plus0([0, 1, 2, 3].map(|k| ee::sub(ee::mul(bound[k], v[3]), ee::mul(v[k], bound[3]))));
        let mut out = Vec::new();
        // The first vertex as loaded, the rest as `ADDx` copies.
        let mut a = poly[0];
        for k in 0..poly.len() {
            let b = poly.get(k + 1).copied().unwrap_or(poly[0]);
            let (da, db) = (gap(a), gap(b));
            let (a_out, b_out) = (sign(da[lane]), sign(db[lane]));
            if !a_out {
                out.push(a);
            }
            if a_out != b_out {
                let (inner, d, step) = if a_out { (plus0(b), vsub(a, b), db) } else { (plus0(a), vsub(b, a), da) };
                let w = ee::sub(ee::mul(bound[3], d[lane]), ee::mul(d[3], bound[lane]));
                out.push(lerp(inner, d, ee::div(step[lane], w)));
            }
            a = plus0(b);
        }
        self.store(to, out.iter().map(|v| v[3]));
        out
    }
}

/// What VU1 kicks for a draw ([`super::draw`]'s setup), in order: each
/// batch of triangles, then of edges, each followed by its clipped
/// polygons. `carry` is what the last draw left.
pub fn kicks(mesh: &ShadowMesh, s: &Setup, carry: &mut Vu1Carry) -> Vec<Kick> {
    // mc_DrawShadow1: the fan's tags kept, then each direction's flag. The
    // loop's software pipeline stores three flags below the directions
    // first: 0x211's is the test on the fan's tag taken as floats; 0x20f's
    // and 0x210's, the MAC flag and vf18 the last program left, are not
    // followed (no model's open edge reads them).
    let flags = mesh.normals.iter().map(|&n| lit(s.light, direction(n))).collect();
    let mut run = Run { s, flags, carry, fan_sent: false, kicks: Vec::new() };
    run.store(FAN_TAGS[0], [FAN_TAG[3], FAN_TAG[3]]);
    run.store(FAN_VERTEX_TAG, [0]);
    run.store(DIRECTIONS - 1, [lit(s.light, [FAN_TAG[0], FAN_TAG[1], FAN_TAG[2]])]);
    for batch in mesh.tris.chunks(BATCH) {
        run.triangles(mesh, batch);
    }
    for batch in mesh.edges.chunks(BATCH) {
        run.edges(mesh, batch);
    }
    run.kicks
}
