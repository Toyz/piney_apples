//! Collision against a town's hit mesh (`libhit.cpp`, `INF SLUS_202.67:
//! 0x00152ec0`-0x00155c60; gcmn `hit.cpp` and `ccSpcChar::HitCheck`).
//!
//! A town's scene file carries one Hit chunk (0x0b00; Mac Anu's is
//! `HIT_sr1town1hit`, 828 triangles on `MDL_floor_02`), which
//! `ccStream::Decode_Hit` (0x0014ce50) and `ccSetHitData` (0x00149990) turn
//! into polygons with their plane, edges and a class from the normal's
//! elevation: floor (at least about 39 degrees up) or wall. The town
//! registers it once, in world coordinates, and every query runs over it:
//!
//! - [`Hits::land`]: `ccLandHitCheck` (gcmn 0x00571e00), a segment from 105
//!   above to 1000 below a point against the floors: the ground height.
//! - [`Hits::line`]: `_ccHitCheckLM` (0x00153930), a segment against the
//!   polygons crossed front to back: walls between two points, and the
//!   camera's line of sight.
//! - [`Hits::sphere`]: `ccModelHitCheckQ`/`QZ` (0x00155600, 0x00155930), a
//!   sphere against faces, edges and vertices, and the push out of them.
//! - [`Hits::hit_check`]: `ccSpcChar::HitCheck` (gcmn 0x0059ee20), the
//!   player's move slid along the walls and pushed out of the characters.
//! - [`Hits::collision_detection`], `ccCharHit::CollisionDetection`
//!   (0x00153470): a character's body pushed out of every other body in
//!   the `ccCharHit` list whose kind its mask names, then out of the town.
//!
//! The character list (`ccCharHitTop`/`Tail` 0x0037890c/10, filled by
//! `ccCharHit::HitEnable` 0x00153310 and emptied by `HitDisable`
//! 0x00153360) is [`Hits::chars`]: copies of the registered [`Body`]s in
//! list order, each owner's copy refreshed whenever its body goes through a
//! query ([`Hits::collision_detection`]) or [`Hits::sync`]. Kite (kind 7)
//! enters it when his arrival ends; a town's walking PCs (kind 2) when the
//! entry control places them (`ccEntryCtrl::initObject`'s `SetHitSW(1)`),
//! leaving and re-entering (at the tail) as they hide and reappear.
//!
//! All of it in the EE's arithmetic ([`crate::ee`]), as the game does it;
//! `tools/test_world_rs.py` runs the game's own functions beside it.

use std::rc::Rc;

use piney_data::ccs::Ccs;
use piney_data::{Error, Result};

use crate::ee::{
    self, F, ONE, V4, add, atan2f, cross, div, dot, le, lt, mul, normalize, sqrtf, sub, vadd, vscale, vsub,
};

/// The Hit chunk kind.
pub const HIT: u16 = 0x0b00;

/// Polygon classes `ccSetHitData` ORs into the attribute.
pub const FLOOR: u32 = 0x2000_0000;
pub const WALL: u32 = 0x4000_0000;
pub const CEILING: u32 = 0x8000_0000;

/// The masks the player queries with: floors or walls, and bit 0 (every
/// town polygon has it).
pub const LAND_MASK: u32 = 0x2000_0001;
pub const WALL_MASK: u32 = 0x4000_0001;
/// The camera's line-of-sight mask (`cameraPosCalc`).
pub const CAMERA_MASK: u32 = 0x4;

/// `-1.0`, a query's "nothing hit".
const MINUS_ONE: F = 0xbf80_0000;
/// The edge tolerance, -0.001.
const NEG_EPS: F = 0xba83_126f;
/// Results kept: `hitResult[32]`; later ones overwrite the last slot.
pub const RESULTS: usize = 32;

/// A `HIT_POLYGON` (0xa0 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct Polygon {
    pub min: [F; 3],
    /// The group's attribute with the class in bits 29-31.
    pub att: u32,
    pub max: [F; 3],
    /// The plane: `nv . p + d = 0`.
    pub d: F,
    /// The vertices (w 1).
    pub vp: [V4; 3],
    /// The unit face normal (w 1).
    pub nv: V4,
    /// Unit edges vp1->vp2, vp2->vp3, vp3->vp1 (w 0), and their lengths.
    pub dv: [V4; 3],
    pub dvs: [F; 3],
}

/// A decoded Hit chunk: `HIT_MODEL` (one `HIT_OBJECT`) and its polygons,
/// and the `ccModelHit` (0xa0 bytes) that puts it in the world.
///
/// ```text
/// ccModelHit +0x00 hitSW  +0x04 next  +0x08 type  +0x0c data (HIT_MODEL)
///            +0x10 rm (model to world)  +0x50 im (its inverse)
/// ```
///
/// A query takes its points into the model's space by subtracting `rm`'s
/// translation, and for `type` other than 0 by `im`'s rotation too
/// (`prepareHitLine` 0x00153a30, `prepareHitSphere` 0x00153be0); results come
/// back through all of `rm`. A town's model sits at the identity; a field's
/// objects are translated (`ccClump::HitEnable` enables with `type` 0).
#[derive(Clone, Debug, PartialEq)]
pub struct HitModel {
    /// The HIT_ object and the model it belongs to.
    pub object: u32,
    pub parent: u32,
    pub min: [F; 3],
    pub max: [F; 3],
    pub polys: Rc<[Polygon]>,
    /// `ccModelHit.rm`, `im` (`ccClump::SetHitMatrix` 0x0013d150: the
    /// coordinate's world matrix and `sceVu0InversMatrix` of it) and
    /// `type` (`ccModelHit::HitEnable(t)`).
    pub rm: [V4; 4],
    pub im: [V4; 4],
    pub kind: i32,
    /// Which owner's `ccModelHit` this is (the port's key: the list holds
    /// each at most once).
    pub id: u32,
}

/// The unit matrix, as stored columns.
pub const UNIT: [V4; 4] = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];

/// `fptoui` (0x00129d08): 0 for a negative or sub-1 value, all ones from
/// 2^32, else truncated.
fn fptoui(v: F) -> u32 {
    if v & 0x8000_0000 != 0 {
        return 0;
    }
    let e = ((v >> 23) & 0xff) as i32 - 127;
    if e < 0 {
        0
    } else if e >= 32 {
        u32::MAX
    } else {
        let m = u64::from(v & 0x7f_ffff | 0x80_0000);
        (if e >= 23 { m << (e - 23) } else { m >> (23 - e) }) as u32
    }
}

/// `ccSetHitData(p, v, 3, attr)` (0x00149990): a triangle's box, normal,
/// class, edges and plane.
pub fn set_hit_data(v: [[F; 3]; 3], attr: u32) -> Polygon {
    let vp: [V4; 3] = v.map(|p| [p[0], p[1], p[2], ONE]);
    let mut min = [0; 3];
    let mut max = [0; 3];
    for k in 0..3 {
        min[k] = vp[0][k];
        for p in &vp[1..] {
            if lt(p[k], min[k]) {
                min[k] = p[k];
            }
        }
    }
    for k in 0..3 {
        max[k] = vp[0][k];
        for p in &vp[1..] {
            if !le(p[k], max[k]) {
                max[k] = p[k];
            }
        }
    }
    // makeNormal (0x00155180).
    let mut nv = normalize(cross(vsub(vp[1], vp[0]), vsub(vp[2], vp[0])));
    nv[3] = ONE;
    let mut t = nv;
    t[2] = 0;
    let h = sqrtf(dot(t, t));
    let ang = atan2f(nv[2], h);
    let val = sub(div(mul(ee::HALF_TURN, add(ee::PI, ang)), ee::PI), ee::HALF_TURN);
    let a = fptoui(val) & 0xffff;
    let class = if (7169..25600).contains(&a) {
        FLOOR
    } else if (0xa001..0xe000).contains(&a) {
        CEILING
    } else {
        WALL
    };
    let mut dv = [[0; 4]; 3];
    let mut dvs = [0; 3];
    for (e, (p, q)) in [(1, 0), (2, 1), (0, 2)].into_iter().enumerate() {
        let mut d = vsub(vp[p], vp[q]);
        d[3] = ONE;
        dvs[e] = sqrtf(dot(d, d));
        dv[e] = normalize(d);
    }
    let d = mul(MINUS_ONE, dot(nv, vp[0]));
    Polygon { min, att: (attr & 0x1fff_ffff) | class, max, d, vp, nv, dv, dvs }
}

impl HitModel {
    /// Every Hit chunk of a scene file (towns have one), decoded as
    /// `Decode_Hit` does: the groups' triangles in order, their stored
    /// vertex normals ignored.
    pub fn read(c: &Ccs) -> Result<Vec<HitModel>> {
        let mut out = Vec::new();
        let d = &c.data;
        let word = |q: usize| -> Result<u32> {
            d.get(q..q + 4)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                .ok_or_else(|| Error::Format(format!("Hit chunk runs past the end at 0x{q:x}")))
        };
        for ch in c.walk().chunks {
            if ch.in_frames || ch.kind != HIT {
                continue;
            }
            let mut q = ch.payload();
            let object = word(q)?;
            let parent = word(q + 4)?;
            let groups = word(q + 8)? & 0xffff;
            q += 16;
            if groups == 0 {
                continue;
            }
            let mut min = [0x4b80_0000; 3];
            let mut max = [0xcb80_0000; 3];
            let mut polys = Vec::new();
            for _ in 0..groups {
                let n = word(q)? / 3;
                let attr = word(q + 4)?;
                q += 8;
                for _ in 0..n {
                    let mut v = [[0; 3]; 3];
                    for (i, p) in v.iter_mut().enumerate() {
                        for (k, x) in p.iter_mut().enumerate() {
                            *x = word(q + 12 * i + 4 * k)?;
                        }
                    }
                    q += 36;
                    let p = set_hit_data(v, attr);
                    for (m, &v) in min.iter_mut().zip(&p.min) {
                        if !le(*m, v) {
                            *m = v;
                        }
                    }
                    for (m, &v) in max.iter_mut().zip(&p.max) {
                        if lt(*m, v) {
                            *m = v;
                        }
                    }
                    polys.push(p);
                }
                q += 36 * n as usize;
            }
            out.push(HitModel {
                object,
                parent,
                min,
                max,
                polys: polys.into(),
                rm: UNIT,
                im: UNIT,
                kind: 0,
                id: object,
            });
        }
        Ok(out)
    }
}

/// One `HIT_RESULT`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HitResult {
    /// The contact point and the world normal.
    pub cp: V4,
    pub nv: V4,
    pub dist: F,
    /// Sphere hits: 1 face, 2 edge, 3 vertex; which edge or vertex.
    pub step: i32,
    pub ev: i32,
    /// `ccHitResultGrouping`'s group, -1 dropped.
    pub temp: i32,
    /// The polygon, its attribute and its own normal.
    pub poly: usize,
    pub att: u32,
    pub fnv: V4,
}

/// Is the box (`qmin`, `qmax`) clear of (`bmin`, `bmax`)?
fn rejects(qmin: &[F; 3], qmax: &[F; 3], bmin: &[F; 3], bmax: &[F; 3]) -> bool {
    (0..3).any(|k| !le(qmin[k], bmax[k]) || lt(qmax[k], bmin[k]))
}

/// The world normal a result carries: `NORM(APPLY(rm, nv) - rm[3])`, w 1
/// (`sceVu0ApplyMatrix` kept literal even at the unit matrix: it can
/// change the sign of a zero).
fn world_normal(rm: &[V4; 4], nv: V4) -> V4 {
    let mut n = normalize(vsub(ee::apply(rm, nv), rm[3]));
    n[3] = ONE;
    n
}

/// What `WORLD_MAN::GetHeight` (main 0x001a10c0) answers for a field or a
/// dungeon: the ground's height under a map point, which `ccLandHitCheck`
/// falls back on in a field and the camera's `avoidObstacle` asks.
pub trait Heights: std::fmt::Debug {
    fn height(&self, x: F, y: F) -> F;
}

fn is_zero3(v: &V4) -> bool {
    ee::eq(0, v[0]) && ee::eq(0, v[1]) && ee::eq(0, v[2])
}

/// The collision world: the registered hit models (a town has one, at the
/// identity) and libhit's result globals.
#[derive(Clone, Debug, Default)]
pub struct Hits {
    /// The `ccModelHit` list (`ccModelHitTop`..`Tail`), in its order.
    pub models: Vec<HitModel>,
    /// `game.area`: 0 a town, 1 a field, 2 a dungeon.
    pub area: i32,
    /// `WORLD_MAN`'s minx, miny, maxx, maxy (+0x420..+0x42c), when the map
    /// wraps (a field's 0..48000); None in a town, where nothing reaches its
    /// +-24000.
    pub bounds: Option<[F; 4]>,
    /// The field's or dungeon's ground (`WORLD_MAN::GetHeight`).
    pub heights: Option<Rc<dyn Heights>>,
    /// `WORLD_MAN.defSE` (+0x15c): the attribute `ccLandHitCheck` gives the
    /// height map's ground (its footsteps' type bits).
    pub def_se: u32,
    /// `WORLD_MAN::CheckEventArea()`: the field is a story map of its own
    /// (an `EVENTAREA`), with no height map under it.
    pub event_area: bool,
    /// The area's centre as `WORLD_MAN::AddCenter` moves it with the player
    /// each frame: a field's `WORLD.ofs_x`, `ofs_y` (+0x6140), wrapped into
    /// the map, the centre its chips are drawn round.
    pub center: [F; 2],
    /// `hitResult[]` as far as `hitResultNum` fills it, and the count.
    pub results: Vec<HitResult>,
    pub num: u32,
    /// `hitResultNearest`.
    pub nearest: HitResult,
    /// The `ccCharHit` list: the registered characters' bodies in list
    /// order, as their owners last left them.
    pub chars: Vec<Body>,
    /// `hitResultCharType`: the kinds the last `CollisionDetection` touched.
    pub char_type: u32,
}

/// A prepared `HIT_LINE`.
struct Line {
    mask: u32,
    min: [F; 3],
    max: [F; 3],
    sp: V4,
    ep: V4,
    dv: V4,
}

/// A prepared `HIT_SPHERE`.
struct Sphere {
    mask: u32,
    min: [F; 3],
    max: [F; 3],
    r: F,
    r2: F,
    cp: V4,
}

impl Hits {
    pub fn new(models: Vec<HitModel>) -> Self {
        Hits { models, ..Hits::default() }
    }

    fn clear(&mut self) {
        self.results.clear();
        self.num = 0;
    }

    fn record(&mut self, hr: HitResult) {
        if self.results.len() < RESULTS {
            self.results.push(hr);
        } else {
            self.results[RESULTS - 1] = hr;
        }
        self.num += 1;
    }

    fn passes(mask: u32, att: u32, mask_type: i32) -> bool {
        if mask_type == 0 { mask & att != 0 } else { mask & att == mask }
    }

    /// `prepareHitLine` (0x00153a30): the segment in the model's space.
    fn prepare_line(sp: V4, ep: V4, mask: u32, model: &HitModel) -> Line {
        let mut s = vsub(sp, model.rm[3]);
        let mut e = vsub(ep, model.rm[3]);
        e[3] = ONE;
        s[3] = ONE;
        if model.kind != 0 {
            let mut im = model.im;
            im[3] = [0, 0, 0, ONE];
            s = ee::apply(&im, s);
            e = ee::apply(&im, e);
            e[3] = ONE;
            s[3] = ONE;
        }
        let (mut min, mut max) = ([0; 3], [0; 3]);
        for k in 0..3 {
            if le(e[k], s[k]) {
                (min[k], max[k]) = (e[k], s[k]);
            } else {
                (min[k], max[k]) = (s[k], e[k]);
            }
        }
        Line { mask, min, max, sp: s, ep: e, dv: normalize(vsub(e, s)) }
    }

    /// `collisionLM` (0x00153f60) over one model.
    fn collision_lm(&mut self, m: usize, l: &Line, mask_type: i32) {
        let model = &self.models[m];
        if rejects(&l.min, &l.max, &model.min, &model.max) {
            return;
        }
        let mut found = Vec::new();
        for (j, p) in model.polys.iter().enumerate() {
            if !Self::passes(l.mask, p.att, mask_type) || rejects(&l.min, &l.max, &p.min, &p.max) {
                continue;
            }
            let ds = add(p.d, dot(p.nv, l.sp));
            let de = add(p.d, dot(p.nv, l.ep));
            if lt(ds, 0) || !le(de, 0) {
                continue;
            }
            let dn = dot(p.nv, l.dv);
            if ee::eq(0, dn) {
                continue;
            }
            if (0..3).any(|e| lt(dot(cross(p.dv[e], l.dv), vsub(l.sp, p.vp[e])), NEG_EPS)) {
                continue;
            }
            // sceVu0DivVector: times the reciprocal.
            let tv = vscale(vscale(l.dv, mul(MINUS_ONE, ds)), div(ONE, dn));
            let dist = sqrtf(dot(tv, tv));
            let mut hp = vadd(l.sp, tv);
            hp[3] = ONE;
            let mut cp = ee::apply(&model.rm, hp);
            cp[3] = ONE;
            found.push(HitResult {
                cp,
                nv: world_normal(&model.rm, p.nv),
                dist,
                poly: j,
                att: p.att,
                fnv: p.nv,
                ..Default::default()
            });
        }
        for hr in found {
            self.record(hr);
        }
    }

    /// `setNearest` (0x00153ce0): the last of the smallest distances.
    fn set_nearest(&mut self) {
        let mut best = self.results[0].dist;
        let mut idx = 0;
        for (i, r) in self.results.iter().enumerate() {
            if le(r.dist, best) {
                best = r.dist;
                idx = i;
            }
        }
        self.nearest = self.results[idx];
    }

    /// `_ccHitCheckLM(sp, ep, mask, maskType)`: the nearest crossing of the
    /// segment, as (contact point, distance), or None.
    pub fn line(&mut self, sp: V4, ep: V4, mask: u32, mask_type: i32) -> Option<(V4, F)> {
        self.clear();
        for m in 0..self.models.len() {
            let l = Self::prepare_line(sp, ep, mask, &self.models[m]);
            self.collision_lm(m, &l, mask_type);
        }
        if self.num == 0 {
            return None;
        }
        self.set_nearest();
        Some((self.nearest.cp, self.nearest.dist))
    }

    /// `ccTransPosW2M` (gcmn 0x0059b900) through `ccPlayer::W2MPos`
    /// (0x0059b470): a point wrapped into the map's bounds (x below minx
    /// gains maxx - minx, above maxx loses it; y likewise), z kept, w 1; and
    /// whether it moved.
    pub fn w2m(&self, pos: V4) -> (V4, bool) {
        let mut p = [pos[0], pos[1], pos[2], ONE];
        let Some([minx, miny, maxx, maxy]) = self.bounds else { return (p, false) };
        let mut moved = false;
        for (v, lo, hi) in [(0, minx, maxx), (1, miny, maxy)] {
            if lt(p[v], lo) {
                p[v] = add(p[v], sub(hi, lo));
                moved = true;
            } else if !le(p[v], hi) {
                p[v] = add(p[v], sub(lo, hi));
                moved = true;
            }
        }
        (p, moved)
    }

    /// `WORLD_MAN::AddCenter(dx, dy)` (main 0x001a0fd0) from
    /// `MapLoopAdjustPos`: in a field (`WORLD::AddCenter` 0x005aa3d0) the
    /// centre wraps into the map's bounds.
    pub fn add_center(&mut self, dx: F, dy: F) {
        for (k, d) in [dx, dy].into_iter().enumerate() {
            let mut v = add(self.center[k], d);
            if let Some(b) = self.bounds {
                let (lo, hi) = (b[k], b[k + 2]);
                if !le(v, hi) {
                    v = add(v, sub(lo, hi));
                } else if lt(v, lo) {
                    v = add(v, sub(hi, lo));
                }
            }
            self.center[k] = v;
        }
    }

    /// `WORLD_MAN::GetHeight(x, y)` (main 0x001a10c0) as
    /// `ccSetGroundHeight` asks it: a town's `ROOTTOWN::GetHeight` is 0, as
    /// is a story field's own map.
    pub fn ground_height(&self, x: F, y: F) -> F {
        match (&self.heights, self.event_area) {
            (Some(h), false) => h.height(x, y),
            _ => 0,
        }
    }

    /// `ccLandHitCheck(pos, mask)` (gcmn 0x00571e00): the highest floor
    /// polygon from 105 above the point (taken into the map) to 1000 below,
    /// or its z when there is none. In a field (`game.area` 1, not a story
    /// map) a miss answers the height map instead (`ccSetGroundHeight`; the
    /// point's own z when that is below 0) and leaves one result behind:
    /// that point, normal (0, 0, 1), distance 0, attribute
    /// `defSE | 0x20000f0f`.
    pub fn land(&mut self, pos: V4, mask: u32) -> F {
        let (mp, _) = self.w2m(pos);
        let (mut sp, mut ep) = (mp, mp);
        sp[2] = add(sp[2], 0x42d2_0000);
        ep[2] = sub(ep[2], 0x447a_0000);
        let hit = self.line(sp, ep, mask, 1);
        let z = match hit {
            Some((cp, _)) => cp[2],
            None => mp[2],
        };
        if self.area != 1 || self.event_area || hit.is_some() {
            return z;
        }
        let mut h = self.ground_height(mp[0], mp[1]);
        if lt(h, 0) {
            h = mp[2];
        }
        let mut nearest = self.nearest;
        nearest.cp = [mp[0], mp[1], h, ONE];
        nearest.nv = [0, 0, ONE, ONE];
        nearest.dist = 0;
        nearest.poly = usize::MAX;
        nearest.att = self.def_se | 0x2000_0f0f;
        self.nearest = nearest;
        self.results.clear();
        self.results.push(nearest);
        self.num = 1;
        h
    }

    /// `prepareHitSphere` (0x00153be0): the centre in the model's space,
    /// and the box round it there.
    fn prepare_sphere(cp: V4, r: F, mask: u32, model: &HitModel) -> Sphere {
        let mut c = vsub(cp, model.rm[3]);
        c[3] = ONE;
        if model.kind != 0 {
            c = vsub(ee::apply(&model.im, c), model.im[3]);
            c[3] = ONE;
        }
        let min = [sub(c[0], r), sub(c[1], r), sub(c[2], r)];
        let max = [add(c[0], r), add(c[1], r), add(c[2], r)];
        Sphere { mask, min, max, r, r2: mul(r, r), cp: c }
    }

    /// `collisionQM` (0x001545e0) over one model.
    fn collision_qm(&mut self, m: usize, q: &Sphere, mask_type: i32) {
        let model = &self.models[m];
        if rejects(&q.min, &q.max, &model.min, &model.max) {
            return;
        }
        let mut found = Vec::new();
        for (j, p) in model.polys.iter().enumerate() {
            if !Self::passes(q.mask, p.att, mask_type) || rejects(&q.min, &q.max, &p.min, &p.max) {
                continue;
            }
            let mut ev = 0;
            let ds = add(p.d, dot(p.nv, q.cp));
            if lt(ds, 0) {
                continue;
            }
            let mut hp: Option<V4> = None;
            let mut dist = 0;
            let mut step = 0;
            if !ee::eq(0, ds) {
                if !le(ds, q.r) {
                    continue;
                }
                step = 1;
                dist = ds;
                let qv = vscale(p.nv, mul(MINUS_ONE, ds));
                let inside = (0..3).all(|e| !lt(dot(cross(p.dv[e], qv), normalize(vsub(q.cp, p.vp[e]))), NEG_EPS));
                if inside {
                    hp = Some(vadd(q.cp, qv));
                }
            }
            if hp.is_none() {
                step = 2;
                for e in 0..3 {
                    let (dv, vp) = (p.dv[e], p.vp[e]);
                    if is_zero3(&dv) {
                        continue;
                    }
                    let t = sub(dot(dv, q.cp), dot(vp, dv));
                    if lt(t, 0) || !le(t, p.dvs[e]) {
                        continue;
                    }
                    let mut pt = vadd(vscale(dv, t), vp);
                    pt[3] = ONE;
                    let tv = vsub(pt, q.cp);
                    let d2 = dot(tv, tv);
                    if !le(d2, q.r2) {
                        continue;
                    }
                    let dd = sqrtf(d2);
                    if ev != 0 && !le(dd, dist) {
                        continue;
                    }
                    hp = Some(pt);
                    dist = dd;
                    ev = e as i32 + 1;
                }
                if ev == 0 {
                    step = 3;
                    let mut stop = false;
                    for e in 0..3 {
                        let vp = p.vp[e];
                        let tv = vsub(vp, q.cp);
                        let d2 = dot(tv, tv);
                        if e < 2 {
                            if !le(d2, q.r2) {
                                continue;
                            }
                        } else if !le(d2, q.r2) && ev == 0 {
                            stop = true;
                            break;
                        }
                        let dd = sqrtf(d2);
                        if ev != 0 && !le(dd, dist) {
                            continue;
                        }
                        hp = Some(vp);
                        dist = dd;
                        ev = e as i32 + 1;
                    }
                    if stop || ev == 0 {
                        continue;
                    }
                }
            }
            let Some(mut hp) = hp else { continue };
            hp[3] = ONE;
            let mut cp = ee::apply(&model.rm, hp);
            cp[3] = ONE;
            found.push(HitResult {
                cp,
                nv: world_normal(&model.rm, p.nv),
                dist,
                step,
                ev,
                temp: 0,
                poly: j,
                att: p.att,
                fnv: p.nv,
            });
        }
        for hr in found {
            self.record(hr);
        }
    }

    /// `ccHitResultGrouping` (0x001552e0): results with the same face
    /// normal form a group, of which only the nearest is kept (`temp`).
    fn grouping(&mut self) {
        let n = self.num as usize;
        let r = &mut self.results;
        if n == 0 {
            return;
        }
        if n == 1 {
            r[0].temp = 0;
            return;
        }
        let n = n.min(RESULTS);
        for x in r.iter_mut().take(n) {
            x.temp = -1;
        }
        for i in 0..n - 1 {
            if r[i].temp != -1 {
                continue;
            }
            r[i].temp = i as i32;
            for j in i + 1..n {
                if r[j].temp == -1 {
                    let d = vsub(r[i].fnv, r[j].fnv);
                    if lt(dot(d, d), 0x3586_37bd) {
                        r[j].temp = i as i32;
                    }
                }
            }
        }
        if r[n - 1].temp == -1 {
            r[n - 1].temp = n as i32 - 1;
        }
        for i in 0..n - 1 {
            if r[i].temp != i as i32 {
                continue;
            }
            let mut k = i;
            for j in i + 1..n {
                if r[k].temp == r[j].temp {
                    if !le(r[k].dist, r[j].dist) {
                        r[k].temp = -1;
                        k = j;
                    } else {
                        r[j].temp = -1;
                    }
                }
            }
        }
    }

    /// `ccModelHitCheckQ(offset, pos, r, mask, maskType)` (`keep_z` false)
    /// or `QZ` (true): a sphere of radius `r` at `pos`; the push out of
    /// every face group it touches added to `offset` (horizontally only for
    /// Q), w 1; the nearest distance or None.
    pub fn sphere(&mut self, offset: &mut V4, pos: V4, r: F, mask: u32, mask_type: i32, keep_z: bool) -> Option<F> {
        let mut cp = pos;
        cp[3] = ONE;
        self.clear();
        for m in 0..self.models.len() {
            let q = Self::prepare_sphere(cp, r, mask, &self.models[m]);
            self.collision_qm(m, &q, mask_type);
        }
        let d = if self.num > 0 {
            self.set_nearest();
            Some(self.nearest.dist)
        } else {
            None
        };
        if d.is_some() {
            self.grouping();
            for i in 0..(self.num as usize).min(RESULTS) {
                let h = self.results[i];
                if h.temp == -1 {
                    continue;
                }
                let mut s = vscale(h.nv, sub(r, h.dist));
                if !keep_z {
                    s[2] = 0;
                }
                *offset = vadd(*offset, s);
            }
        }
        offset[3] = ONE;
        d
    }

    /// `checkHitResultAttlibute` (0x00153e80) after a query: every result's
    /// attribute ORed, with the ground-type nibbles of the nearest result
    /// that has them. With none, the nibbles come from whatever `$a2` held,
    /// which after `CollisionTest` is the nearest contact's x.
    pub fn attribute(&self) -> u32 {
        let mut v = 0;
        let mut best = 0x461c_4000; // 10000.0
        let mut col = self.nearest.cp[0];
        for h in self.results.iter().take((self.num as usize).min(RESULTS)) {
            let a = h.att;
            if a & 0x4_0000 != 0x4_0000 && a & 0x00f0_f0f0 != 0x00e0_00e0 && lt(h.dist, best) {
                col = a;
                best = h.dist;
            }
            v |= a;
        }
        (v & 0xff0f_0f0f) | (col & 0x00f0_f0f0)
    }

    /// Whether [`Hits::attribute`]'s ground-type nibbles come from a result
    /// (else from the stale register, which in a field - many models on the
    /// list - holds an address inside the hit data: the game's own layout,
    /// not reproduced).
    pub fn attribute_qualified(&self) -> bool {
        self.results
            .iter()
            .take((self.num as usize).min(RESULTS))
            .any(|h| h.att & 0x4_0000 != 0x4_0000 && h.att & 0x00f0_f0f0 != 0x00e0_00e0 && lt(h.dist, 0x461c_4000))
    }
}

/// The id of the player's body in [`Hits::chars`]: [`Body::default`]'s.
pub const PLAYER_ID: u32 = 0;

/// A character's `ccCharHit` (0x50 bytes; the player's `bodyHit` at
/// +0x1a0, an entry object's at +0x160).
///
/// ```text
/// +0x00 hitSW  +0x04 mask (kinds it is pushed out of)  +0x08 mask2 (the
/// town's polygons)  +0x0c type (its own kind)  +0x10 next  +0x14 radius
/// +0x18 height  +0x20 pos  +0x30 offset  +0x40 attribute
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub pos: V4,
    pub radius: F,
    /// 95.0 for the player: the sphere's centre above the feet, and the
    /// height of the wall segments.
    pub height: F,
    /// `mask2`: 0x40000001, the walls.
    pub mask2: u32,
    pub offset: V4,
    /// `hitSW`: in the character list.
    pub sw: bool,
    /// +0x04, -1 from `ccCharHit::ccCharHit`: the kinds of body it is
    /// pushed out of.
    pub mask: u32,
    /// +0x0c: its kind (7 the player, 2 a walking PC).
    pub kind: u32,
    /// +0x40: the ground attribute of its last contact with the town.
    pub attribute: u32,
    /// Which character it is: the key of its copy in [`Hits::chars`]
    /// (the port's; [`PLAYER_ID`] for Kite).
    pub id: u32,
    /// `ccSpcChar::CheckControlMode()` of its owner (the AI's `manualSW`):
    /// `HitCheck` then masks it with -8, out of no character of kinds 1, 2
    /// or 4. A new game's Kite has it clear (`ccSPC::Initialise` leaves his
    /// registry's boot status 0, and only a boot status with bit 4 calls
    /// `ccAI::ManualMode`).
    pub manual: bool,
}

impl Default for Body {
    /// The player's, as `ccPlayer::ccPlayer` leaves it: kind 7, every mask.
    fn default() -> Self {
        Body {
            pos: [0, 0, 0, ONE],
            radius: ONE,
            height: 0x42be_0000,
            mask2: WALL_MASK,
            offset: [0; 4],
            sw: false,
            mask: u32::MAX,
            kind: 7,
            attribute: 0,
            id: PLAYER_ID,
            manual: false,
        }
    }
}

impl Hits {
    /// `ccCharHit::HitEnable`: into the character list at its tail, unless
    /// already in it.
    pub fn hit_enable(&mut self, body: &mut Body) {
        if !body.sw {
            body.sw = true;
            self.chars.push(*body);
        }
    }

    /// `ccCharHit::HitDisable`: out of the list.
    pub fn hit_disable(&mut self, body: &mut Body) {
        if body.sw {
            body.sw = false;
            if let Some(k) = self.chars.iter().position(|c| c.id == body.id) {
                self.chars.remove(k);
            }
        }
    }

    /// `ccCharHit::SetHitSW(on)`.
    pub fn set_hit_sw(&mut self, body: &mut Body, on: bool) {
        if on { self.hit_enable(body) } else { self.hit_disable(body) }
    }

    /// The list's copy of `body` made current (its owner moved it).
    pub fn sync(&mut self, body: &Body) {
        if let Some(c) = self.chars.iter_mut().find(|c| c.id == body.id) {
            *c = *body;
        }
    }
}

impl Hits {
    /// `ccCharHit::CollisionDetection` (0x00153470): `offset` the push out
    /// of every other body in the list whose kind `mask` names (while
    /// `body` is in the list itself), `(r1 + r2 - d)` along the line
    /// between the centres, `hitResultCharType` ([`Hits::char_type`]) their
    /// kinds; then, with `mask2`, the push out of the town's polygons
    /// around the sphere at `pos + height`, and `attribute` its ground.
    /// 1 when a character was touched, plus 2 when the town was.
    pub fn collision_detection(&mut self, body: &mut Body) -> i32 {
        body.offset = [0; 4];
        self.char_type = 0;
        self.sync(body);
        let mut r = 0;
        if body.sw {
            let mut offset = body.offset;
            for other in &self.chars {
                if other.id == body.id || body.mask & other.kind == 0 {
                    continue;
                }
                let d = vsub(other.pos, body.pos);
                let d = sqrtf(dot(d, d));
                let over = sub(add(other.radius, body.radius), d);
                if lt(over, 0) {
                    continue;
                }
                let mut v = vscale(normalize(vsub(body.pos, other.pos)), over);
                v[3] = 0;
                offset = vadd(offset, v);
                r = 1;
                self.char_type |= other.kind;
            }
            body.offset = offset;
        }
        if body.mask2 != 0 {
            let mut tp = body.pos;
            tp[2] = add(tp[2], body.height);
            tp[3] = ONE;
            body.attribute = 0;
            let mut off = body.offset;
            let d = self.sphere(&mut off, tp, body.radius, body.mask2, 1, false);
            body.offset = off;
            if d.is_some() {
                r += 2;
                body.attribute = self.attribute();
            }
        }
        r
    }

    /// `ccSpcChar::HitCheck(movePos)` in a town: the move from `pos`
    /// pushed out of the walls and the characters around where it lands,
    /// or stopped dead when a wall lies between; returns the result bits (1:
    /// a walking PC, kind 2, was touched; 2: barely moving after a push)
    /// and the move. (Its `HitDisable` for act 14 or a dead character is
    /// not reached in a town.)
    pub fn hit_check(
        &mut self,
        body: &mut Body,
        pos: V4,
        move_pos: V4,
        width: F,
        now_speed: F,
        run: bool,
    ) -> (i32, V4) {
        let mut result = 0;
        let saved_mask = body.mask;
        if body.manual {
            body.mask = 0xffff_fff8;
        }
        let mut hp1 = move_pos;
        for k in 0..3 {
            hp1[k] = add(hp1[k], pos[k]);
        }
        hp1[3] = ONE;
        hp1[2] = self.land(hp1, LAND_MASK);
        body.pos = hp1;
        body.radius = add(width, now_speed);
        let h = body.height;
        let mut mv = move_pos;
        let raised = |mut p: V4| {
            p[2] = add(p[2], h);
            p
        };
        if self.collision_detection(body) != 0 {
            if self.char_type & 2 != 0 {
                result |= 1;
            }
            hp1 = vadd(move_pos, body.offset);
            for k in 0..3 {
                hp1[k] = add(hp1[k], pos[k]);
            }
            hp1[3] = ONE;
            hp1[2] = self.land(hp1, LAND_MASK);
            body.pos = hp1;
            if self.collision_detection(body) != 0 {
                if self.char_type & 2 != 0 {
                    result |= 1;
                }
                let mut hp2 = vadd(hp1, body.offset);
                hp2[2] = self.land(hp2, LAND_MASK);
                hp2[3] = ONE;
                hp1 = vscale(vadd(hp1, hp2), 0x3f00_0000);
                hp1[3] = ONE;
                hp1[2] = self.land(hp1, LAND_MASK);
            }
            mv = if self.line(raised(pos), raised(hp1), WALL_MASK, 1).is_some() { ee::VF0 } else { vsub(hp1, pos) };
            let mut bp = raised(vadd(pos, mv));
            bp[3] = ONE;
            if self.line(raised(pos), bp, WALL_MASK, 1).is_some() {
                mv = ee::VF0;
            }
            let spd = if run { 0x4100_0000 } else { ONE };
            let nspd = ee::neg(spd);
            if lt(mv[0], spd) && !le(mv[0], nspd) && lt(mv[1], spd) && !le(mv[1], nspd) {
                result |= 2;
            }
        } else if self.line(raised(pos), raised(hp1), WALL_MASK, 1).is_some() {
            mv = ee::VF0;
        }
        body.mask = saved_mask;
        self.sync(body);
        (result, mv)
    }
}

impl crate::camera::CameraHits for Hits {
    fn line(&mut self, sp: V4, ep: V4) -> Option<(V4, F)> {
        Hits::line(self, sp, ep, CAMERA_MASK, 0)
    }

    fn sphere(&mut self, pos: V4) -> Option<V4> {
        let mut off = [0; 4];
        Hits::sphere(self, &mut off, pos, 0x41c8_0000, 1, 0, true).map(|_| off)
    }

    /// `ccTransPosW2M` then `ccSetGroundHeight`.
    fn ground(&mut self, pos: V4) -> F {
        let (p, _) = self.w2m(pos);
        self.ground_height(p[0], p[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bodies_push_each_other_apart() {
        let mut hits = Hits::default();
        let mut pc = Body { pos: [ee::k(50.0), 0, 0, ONE], radius: ee::k(35.0), kind: 2, id: 0x100, ..Body::default() };
        let mut kite = Body { radius: ee::k(45.0), mask2: 0, ..Body::default() };
        hits.hit_enable(&mut pc);
        // Out of the list, nothing touches him.
        assert_eq!(hits.collision_detection(&mut kite), 0);
        hits.hit_enable(&mut kite);
        assert_eq!(hits.chars.iter().map(|b| b.id).collect::<Vec<_>>(), [0x100, PLAYER_ID]);
        assert_eq!(hits.collision_detection(&mut kite), 1);
        // 30 along the line, through sceVu0Normalize's reciprocal: -29.999998.
        let away = |d: f32| {
            let mut v = vscale(normalize([ee::k(d), 0, 0, ONE]), ee::k(30.0));
            v[3] = 0;
            v
        };
        assert_eq!(kite.offset, away(-50.0));
        assert!((ee::f(kite.offset[0]) + 30.0).abs() < 1e-5);
        assert_eq!(hits.char_type, 2);
        // A PC is pushed out of him (and his kind 7 is in its mask).
        pc.mask2 = 0;
        assert_eq!(hits.collision_detection(&mut pc), 1);
        assert_eq!((pc.offset, hits.char_type), (away(50.0), 7));
        // Masked with -8 (his AI in manual mode) he passes through.
        kite.mask = 0xffff_fff8;
        assert_eq!(hits.collision_detection(&mut kite), 0);
        hits.hit_disable(&mut pc);
        assert_eq!(hits.chars.len(), 1);
    }
}
