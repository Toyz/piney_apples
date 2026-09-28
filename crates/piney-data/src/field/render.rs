//! How the game draws a field (`docs/engine/field.md`, Drawing), from
//! `WORLD::DrawMesh` (`INF gcmn.prg:0x005a8570`), as `tools/field.py`'s render
//! section models it: the ground tile per chip ([`Field::tile`], z and RGB
//! rewritten by `SetMESH2` from the height cells), ground cover
//! ([`Field::cover_mesh`]), `WORLD::GetHeight` ([`Field::get_height`]) and the
//! background's distant light ([`Light`], [`object_colours`]). The VU0 macro
//! code it uses is modelled with the FPU's rules ([`ee`]), an inference; the
//! rest is the EE's own arithmetic, checked bit for bit by `tools/test_field.py`.

use super::ee::{self, bits};
use super::{CHIPS, Cover, Field, MAP, MeshCell, Tables};
use crate::ccs::{self, Ccs};
use crate::{Bytes, Result, format_err};

type Vec4 = [u32; 4];
type Mat4 = [Vec4; 4];

const ONE: u32 = bits(1.0);
const MINUS_ONE: u32 = bits(-1.0);
const CELL: u32 = bits(600.0);
const CHIP: u32 = bits(1200.0);
/// A model's s16 positions are value * 4096 / vertexScale.
const FIXED_ONE: u32 = bits(4096.0);
/// `WORLD::GetHeight`'s segment runs from z 2,500 down to -1,200; a hidden
/// chip answers -500 and a miss -1.
const RAY_TOP: u32 = bits(2500.0);
const RAY_BOTTOM: u32 = bits(-1200.0);
const HIDDEN_HEIGHT: u32 = bits(-500.0);
/// Its two triangles per cell in cell-local x, y (gcmn 0x00658a30): the
/// corners take the heights of cells (0, 1), (0, 0), (1, 1) and (1, 0),
/// (1, 1), (0, 0) of the cell.
const TRIANGLES: [[(u32, u32, i32, i32); 3]; 2] =
    [[(0, CELL, 0, 1), (0, 0, 0, 0), (CELL, CELL, 1, 1)], [(CELL, 0, 1, 0), (CELL, CELL, 1, 1), (0, 0, 0, 0)]];
/// `collisionLP` lets a point lie this far outside an edge.
const EDGE_SLACK: u32 = 0xba83_126f;
/// `lightVector` (main 0x002fb100).
const LIGHT_VECTOR: Vec4 = [0, 0, MINUS_ONE, 0];
/// `col255to1Vector` (main 0x002f7370): 1/255.
const COL255: u32 = 0x3b80_8081;
/// `FIELD`'s constructor starts every colour at 128.
const BASE_COLOUR: u32 = bits(128.0);
const COLOUR_MAX: u32 = bits(255.0);
/// A model's s8 normals are value * 64.
const NORMAL_UNIT: u32 = bits(64.0);
const SIX: u32 = bits(6.0);
const DEG180: u32 = bits(180.0);
const PI: u32 = 0x4049_0fdb;
const HALF_PI: u32 = 0x3fc9_0fdb;
/// `_sceVu0ecossin`'s polynomial (S5432, main 0x002f7520).
const SIN: [u32; 4] = [0x362e_9c14, 0xb94f_b21f, 0x3c08_873e, 0xbe2a_aaa4];
/// `FCOVER::SetPosition`: a cover's offset by quadrant, and its z.
const COVER_OFFSET: u32 = bits(300.0);
const COVER_Z: u32 = bits(2.5);

/// The distances `DrawMesh` draws by (from the player, in world units): the
/// ground tiles within 8,400, covers within 7,200; both at full alpha to
/// 6,700, then (7,200 - d) / 500.
pub const TILE_RANGE: f32 = 8400.0;
pub const COVER_RANGE: f32 = 7200.0;
pub const FADE_FROM: f32 = 6700.0;
/// The chips `DrawMesh` draws around the player's: -6..5 each way.
pub const DRAWN_CHIPS: i32 = 6;
/// The field's size: a torus of 80 cells.
pub const WORLD_SIZE: f32 = 48_000.0;

// VU0 macro code, modelled with the FPU's rules ------------------------------

fn vu_add(a: Vec4, b: Vec4) -> Vec4 {
    std::array::from_fn(|i| ee::add(a[i], b[i]))
}

fn vu_sub(a: Vec4, b: Vec4) -> Vec4 {
    std::array::from_fn(|i| ee::sub(a[i], b[i]))
}

/// `sceVu0InnerProduct`: vmul.xyz, then x + y, then + z.
fn vu_inner(a: Vec4, b: Vec4) -> u32 {
    ee::add(ee::add(ee::mul(a[0], b[0]), ee::mul(a[1], b[1])), ee::mul(a[2], b[2]))
}

/// `sceVu0Normalize`: Q = sqrt(x*x + y*y + z*z), then 1 / Q, xyz times that;
/// w becomes 0.
fn vu_normalize(v: Vec4) -> Vec4 {
    let q = ee::add(0, ee::sqrt(vu_inner(v, v)));
    let r = ee::div(ONE, q);
    [ee::mul(v[0], r), ee::mul(v[1], r), ee::mul(v[2], r), 0]
}

/// `sceVu0OuterProduct`: opmula then opmsub, each product rounded; w 0.
fn vu_outer(a: Vec4, b: Vec4) -> Vec4 {
    [
        ee::sub(ee::mul(a[1], b[2]), ee::mul(b[1], a[2])),
        ee::sub(ee::mul(a[2], b[0]), ee::mul(b[2], a[0])),
        ee::sub(ee::mul(a[0], b[1]), ee::mul(b[0], a[1])),
        0,
    ]
}

fn vu_scale(v: Vec4, k: u32) -> Vec4 {
    v.map(|c| ee::mul(c, k))
}

/// `sceVu0DivVector`: Q = 1 / k, then every component times Q.
fn vu_div(v: Vec4, k: u32) -> Vec4 {
    let q = ee::div(ONE, k);
    v.map(|c| ee::mul(c, q))
}

/// `sceVu0ApplyMatrix`: columns m, ((c0 x + c1 y) + c2 z) + c3 w.
fn vu_apply(m: &Mat4, v: Vec4) -> Vec4 {
    std::array::from_fn(|k| {
        let acc = ee::mul(m[0][k], v[0]);
        let acc = ee::add(acc, ee::mul(m[1][k], v[1]));
        let acc = ee::add(acc, ee::mul(m[2][k], v[2]));
        ee::add(acc, ee::mul(m[3][k], v[3]))
    })
}

const UNIT: Mat4 = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];

/// `_sceVu0ecossin` (main 0x001109b8) as `sceVu0RotMatrixX/Y/Z` use it: cos
/// an odd polynomial in pi/2 - |angle|, sin +-sqrt(1 - cos^2). (sin, cos).
fn vu_cossin(angle: u32) -> (u32, u32) {
    let neg = ee::lt(angle, 0);
    let x = if neg { ee::add(HALF_PI, angle) } else { ee::sub(HALF_PI, angle) };
    let x2 = ee::mul(x, x);
    let mut p = SIN.map(|k| ee::mul(ee::mul(k, x), x2));
    for v in &mut p[..3] {
        *v = ee::mul(*v, x2);
    }
    let mut r = ee::add(x, p[3]);
    for v in &mut p[..2] {
        *v = ee::mul(*v, x2);
    }
    r = ee::add(r, p[2]);
    p[0] = ee::mul(p[0], x2);
    r = ee::add(ee::add(r, p[1]), p[0]);
    let q = ee::sqrt(ee::sub(ONE, ee::mul(r, r)));
    (if neg { ee::sub(0, q) } else { ee::add(0, q) }, r)
}

/// The rotation's columns times each column of m.
fn vu_mul(cols: &Mat4, m: &Mat4) -> Mat4 {
    m.map(|col| {
        std::array::from_fn(|k| {
            let acc = ee::mul(cols[0][k], col[0]);
            let acc = ee::add(acc, ee::mul(cols[1][k], col[1]));
            let acc = ee::add(acc, ee::mul(cols[2][k], col[2]));
            ee::add(acc, ee::mul(cols[3][k], col[3]))
        })
    })
}

/// `sceVu0RotMatrixX/Y/Z(m, angle)` for axis 0, 1, 2: the axis rotation
/// times m.
fn vu_rot(m: &Mat4, axis: usize, angle: u32) -> Mat4 {
    let (s, c) = vu_cossin(angle);
    let (cs, sn, ns) = (ee::add(0, c), ee::add(0, s), ee::sub(0, s));
    let rot = match axis {
        0 => [[ONE, 0, 0, 0], [0, cs, sn, 0], [0, ns, cs, 0], [0, 0, 0, ONE]],
        1 => [[cs, 0, ns, 0], [0, ONE, 0, 0], [sn, 0, cs, 0], [0, 0, 0, ONE]],
        _ => [[cs, sn, 0, 0], [ns, cs, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]],
    };
    vu_mul(&rot, m)
}

/// `sceVu0RotMatrix(unit, rad)`: Rx * Ry * Rz.
fn rot_matrix(rad: [u32; 3]) -> Mat4 {
    let m = vu_rot(&UNIT, 2, rad[2]);
    let m = vu_rot(&m, 1, rad[1]);
    vu_rot(&m, 0, rad[0])
}

// the ground ------------------------------------------------------------------

/// The unnormalised normal `InitQuad` and `GetHeight` compute with the
/// FPU's mula/msub (each product rounded first) for corners a, b, c.
fn triangle_normal(a: [u32; 3], b: [u32; 3], c: [u32; 3]) -> [u32; 3] {
    let (f9, f8) = (ee::sub(b[1], a[1]), ee::sub(c[2], b[2]));
    let (f2, f5) = (ee::sub(b[2], a[2]), ee::sub(c[1], b[1]));
    let nx = ee::sub(ee::mul(f9, f8), ee::mul(f2, f5));
    let (f4, f1) = (ee::sub(c[0], b[0]), ee::sub(b[0], a[0]));
    let ny = ee::sub(ee::mul(f2, f4), ee::mul(f1, f8));
    let nz = ee::sub(ee::mul(f1, f5), ee::mul(f9, f4));
    [nx, ny, nz]
}

/// `collisionLP` (main 0x00152ee0): where segment p0-p1 meets triangle a, b,
/// c with normal n. p0 must be on n's side, p1 not; each edge test lets the
/// point lie 0.001 outside.
fn collision_lp(p0: Vec4, p1: Vec4, a: Vec4, b: Vec4, c: Vec4, n: Vec4) -> Option<Vec4> {
    let d = vu_normalize(vu_sub(p1, p0));
    let k = ee::mul(MINUS_ONE, vu_inner(n, a));
    let s0 = ee::add(k, vu_inner(n, p0));
    let s1 = ee::add(k, vu_inner(n, p1));
    if ee::lt(s0, 0) || !ee::le(s1, 0) {
        return None;
    }
    let nd = vu_inner(n, d);
    if ee::cmp(0, nd).is_eq() {
        return None;
    }
    for (e0, e1) in [(b, a), (c, b), (a, c)] {
        let side = vu_outer(vu_sub(e0, e1), d);
        let to = vu_normalize(vu_sub(p0, e1));
        if ee::lt(vu_inner(side, to), EDGE_SLACK) {
            return None;
        }
    }
    let v = vu_div(vu_scale(d, ee::mul(MINUS_ONE, s0)), nd);
    let hit = vu_add(p0, v);
    Some([hit[0], hit[1], hit[2], ONE])
}

/// `FIELD::GetHeight` (0x005adb80): `map[x][y]`, wrapping 80 down to 0 and -1
/// up to 78 (the game adds 79). Only once: a place further out reads past
/// the map in the game (`FIREFLY2::SetBasePosition2` can ask at one when
/// the camera's turn is far outside -pi..pi); 0 here.
pub(super) fn cell_height(map: &[u32], x: i32, y: i32) -> u32 {
    let w = |v: i32| {
        let v = if v >= MAP as i32 { v - MAP as i32 } else { v };
        if v < 0 { v + MAP as i32 - 1 } else { v }
    };
    let i = i64::from(w(x)) * MAP as i64 + i64::from(w(y));
    usize::try_from(i).ok().and_then(|i| map.get(i)).copied().unwrap_or(0)
}

/// fmod(x, m) as the game gets it, through double and back: exact, and a
/// float again for a float x and m = 600.
fn fmod(x: u32, m: u32) -> u32 {
    let v = f64::from(f32::from_bits(x));
    let r = v % f64::from(f32::from_bits(m));
    let out = (r as f32).to_bits();
    debug_assert_eq!(f64::from(f32::from_bits(out)), r);
    // exponent 0 is zero on the EE
    if out & 0x7f80_0000 == 0 { out & 0x8000_0000 } else { out }
}

/// `WORLD::GetHeight` (gcmn 0x005aa520) over heights `map` and hidden chips
/// `check3`, as float bits: a segment from z 2,500 to -1,200 cast at the
/// point's offset in its 600-unit cell against the cell's two triangles;
/// -500 on a hidden chip, -1 if both miss.
pub(super) fn get_height(map: &[u32], check3: &[u8], x: u32, y: u32) -> u32 {
    let cx = ee::to_int(ee::div(x, CELL));
    let cy = ee::to_int(ee::div(y, CELL));
    let chip = (cx / 2) as isize * CHIPS as isize + (cy / 2) as isize;
    if usize::try_from(chip).ok().and_then(|c| check3.get(c)) == Some(&1) {
        return HIDDEN_HEIGHT;
    }
    let (fx, fy) = (fmod(x, CELL), fmod(y, CELL));
    for tri in TRIANGLES {
        let [a, b, c] = tri.map(|(tx, ty, dx, dy)| [tx, ty, cell_height(map, cx + dx, cy + dy), ONE]);
        let [nx, ny, nz] = triangle_normal([a[0], a[1], a[2]], [b[0], b[1], b[2]], [c[0], c[1], c[2]]);
        let n = vu_normalize([nx, ny, nz, 0]);
        let p0 = [fx, fy, RAY_TOP, 0];
        let p1 = [fx, fy, RAY_BOTTOM, 0];
        if let Some(hit) = collision_lp(p0, p1, a, b, c, n) {
            return hit[2];
        }
    }
    MINUS_ONE
}

/// A height as the s16 `SetMESH2` and `SetSmallMESH` store:
/// fptosi(4096 * (h / vertexScale)), low 16 bits.
fn fixed(h: u32, scale: u32) -> i16 {
    ee::to_int(ee::mul(FIXED_ONE, ee::div(h, scale))) as i16
}

/// fptoui into a byte.
fn colour_byte(v: u32) -> u8 {
    ee::to_int(v) as u8
}

/// A field's light: the background row's `LGT_` distant light as the
/// background animation's frame 0 sets it (`F_DistantLight`: rotation in
/// degrees, a packed colour, intensity 1) and its `F_Ambient` colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Light {
    /// `F_Ambient`, packed 0x..BBGGRR.
    pub ambient: u32,
    /// The light's rotation in degrees, as float bits.
    pub rotation: [u32; 3],
    /// Packed 0x00BBGGRR.
    pub colour: u32,
}

/// Anime chunk records.
const TOP: u32 = 0xff01;
const F_AMBIENT: u32 = 0x0601;
const F_DISTANT_LIGHT: u32 = 0x0603;
/// `F_DistantLight` flag for an intensity word.
const INTENSITY: u32 = 0x20;

impl Light {
    /// `ccAnm::GetAmbient` (main 0x001528e0): each byte / 255.
    pub fn ambient_rgb(&self) -> [u32; 3] {
        std::array::from_fn(|i| ee::div(ee::from_int(((self.ambient >> (8 * i)) & 0xff) as i32), COLOUR_MAX))
    }

    /// `ccSetColor` (main 0x00138b60) with intensity 1: each byte times 1/255.
    pub fn colour_rgb(&self) -> [u32; 3] {
        std::array::from_fn(|i| ee::mul(ee::from_int(((self.colour >> (8 * i)) & 0xff) as i32), COL255))
    }

    /// `ccDistantLight::GetDirc`: lightVector through the light's matrix,
    /// `sceVu0RotMatrix` of the rotation in radians (`DecodeF_DistantLight`:
    /// degrees * pi / 180).
    pub fn dirc(&self) -> [u32; 4] {
        let rad = self.rotation.map(|d| ee::div(ee::mul(PI, d), DEG180));
        vu_apply(&rot_matrix(rad), LIGHT_VECTOR)
    }

    /// The direction `Lambert` (gcmn 0x005af2d0) lights along: lightVector
    /// rotated by Rz * Ry * Rx of [`Light::dirc`]'s components taken as
    /// angles - the game's own reading of its direction vector - and
    /// normalised.
    pub fn direction(&self) -> [u32; 4] {
        let dirc = self.dirc();
        let mut m = UNIT;
        for (axis, &angle) in dirc.iter().take(3).enumerate() {
            m = vu_rot(&m, axis, angle);
        }
        vu_normalize(vu_apply(&m, LIGHT_VECTOR))
    }

    /// Frame 0 of Anime `anime` in c: its `F_Ambient` and the
    /// `F_DistantLight` of object `light`.
    pub fn from_anime(c: &Ccs, anime: &str, light: &str) -> Result<Light> {
        let d = &c.data;
        let name = |i: u32| c.objects.get(i as usize).map(|o| o.name.as_str());
        for ch in c.walk().chunks {
            if ch.kind == ccs::FRAME {
                break;
            }
            if ch.kind != ccs::ANIME || name(d.u32_at(ch.payload())?) != Some(anime) {
                continue;
            }
            let words = d.u32_at(ch.payload() + 8)? as usize;
            let (mut q, end) = (ch.payload() + 12, ch.payload() + 12 + 4 * words);
            let (mut ambient, mut found) = (None, None);
            while q < end {
                let kind = d.u32_at(q)? & 0xffff;
                let n = d.u32_at(q + 4)? as usize;
                let w = |i: usize| d.u32_at(q + 8 + 4 * i);
                match kind {
                    TOP if w(0)? != 0 => break,
                    F_AMBIENT => ambient = Some(w(0)?),
                    F_DISTANT_LIGHT if name(w(0)?) == Some(light) => {
                        if w(1)? & INTENSITY != 0 {
                            return format_err(format!("{light}: an intensity other than 1 is not modelled"));
                        }
                        found = Some(([w(2)?, w(3)?, w(4)?], w(5)?));
                    }
                    _ => {}
                }
                q += 8 + 4 * n;
            }
            let (Some(ambient), Some((rotation, colour))) = (ambient, found) else {
                return format_err(format!("{anime}: no frame-0 ambient or light {light}"));
            };
            if colour >> 24 != 0 {
                return format_err(format!("{light}: an HSV colour is not modelled"));
            }
            return Ok(Light { ambient, rotation, colour });
        }
        format_err(format!("no Anime chunk {anime}"))
    }
}

/// `Lambert`'s colour: each channel colour * (ambient + max(0, -n.L) *
/// light colour), clamped to 0-255.
fn lambert(direction: Vec4, n: Vec4, colour: [u32; 3], ambient: [u32; 3], light: [u32; 3]) -> [u32; 3] {
    let dot = ee::mul(vu_inner(n, direction), MINUS_ONE);
    let dot = if ee::lt(dot, 0) { 0 } else { dot };
    std::array::from_fn(|i| {
        let v = ee::mul(colour[i], ee::add(ambient[i], ee::mul(dot, light[i])));
        if ee::lt(v, 0) {
            0
        } else if !ee::le(v, COLOUR_MAX) {
            COLOUR_MAX
        } else {
            v
        }
    })
}

/// What `CalcObjectVertexColor` (gcmn 0x005a4560) does to each vertex of a
/// rigid model of the field's object clumps in `WORLD::Init`: its RGB lit
/// like the ground, from its own bytes and its s8 normal / 64, back to bytes
/// by fptosi; alpha kept. A clump listed twice in the tables is lit twice.
pub fn object_colours(normals: &[[u8; 4]], colours: &[[u8; 4]], light: &Light) -> Vec<[u8; 4]> {
    let direction = light.direction();
    let (amb, lc) = (light.ambient_rgb(), light.colour_rgb());
    normals
        .iter()
        .zip(colours)
        .map(|(n, c)| {
            let nf: Vec4 =
                std::array::from_fn(
                    |i| {
                        if i < 3 { ee::div(ee::from_int(i32::from(n[i] as i8)), NORMAL_UNIT) } else { 0 }
                    },
                );
            let lit = lambert(direction, nf, std::array::from_fn(|i| ee::from_int(i32::from(c[i]))), amb, lc);
            [ee::to_int(lit[0]) as u8, ee::to_int(lit[1]) as u8, ee::to_int(lit[2]) as u8, c[3]]
        })
        .collect()
}

/// A ground tile as `FIELD_MESH::RelocateMesh` (0x005b0aa0) and `SetMESH2`
/// leave it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tile {
    pub chip: [u32; 2],
    /// Not on a hidden chip (`FIELD_MESH.dispSW`).
    pub visible: bool,
    /// The translation it is drawn at: the chip's centre (float bits; z 0).
    pub pos: [u32; 3],
    /// (vertex, s16 z) for each vertex `SetMESH2` rewrites.
    pub z: Vec<(u8, i16)>,
    /// (vertex, RGB) likewise, when colours were given.
    pub colours: Vec<(u8, [u8; 3])>,
}

/// A cover tile as `WORLD::SetCover` sets it up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverMesh {
    /// A `SmallMeshName` model.
    pub mesh: &'static str,
    /// Its translation: the chip's centre plus the quadrant's offset
    /// (float bits), as `DrawMesh` adds them.
    pub pos: [u32; 3],
    pub z: Vec<(u8, i16)>,
    pub colours: Vec<(u8, [u8; 3])>,
}

fn cell_colour(colours: &[[u32; 3]], x: i32, y: i32) -> [u32; 3] {
    colours[x.rem_euclid(MAP as i32) as usize * MAP + y.rem_euclid(MAP as i32) as usize]
}

/// `WORLD::GetHeight` at world (x, y) before `Generate` has made the
/// heights: `FIELD`'s constructor leaves the height map and the hidden
/// chips as the heap had them, taken here as a fresh heap's zeros.
pub fn blank_height(x: u32, y: u32) -> u32 {
    static MAP_ZERO: [u32; 6400] = [0; 6400];
    static HIDDEN_ZERO: [u8; 1600] = [0; 1600];
    get_height(&MAP_ZERO, &HIDDEN_ZERO, x, y)
}

impl Field {
    /// `WORLD::GetHeight` at world (x, y) (float bits) over the final
    /// heights. The objects' z is the same query made as each was placed.
    pub fn get_height(&self, x: u32, y: u32) -> u32 {
        get_height(&self.map, &self.check3, x, y)
    }

    /// `FIELD::InitQuad`'s two normalised face normals per quad `[x][y]`:
    /// face 0 of corners (x, y+1), (x, y), (x+1, y+1), face 1 of (x+1, y),
    /// (x+1, y+1), (x, y+1).
    fn quad_faces(&self) -> Vec<[Vec4; 2]> {
        let h = |x: i32, y: i32| cell_height(&self.map, x, y);
        let mut out = Vec::with_capacity(MAP * MAP);
        for x in 0..MAP as i32 {
            for y in 0..MAP as i32 {
                let (px, px1) = (ee::mul(CELL, ee::from_int(x)), ee::mul(CELL, ee::from_int(x + 1)));
                let (py, py1) = (ee::mul(CELL, ee::from_int(y)), ee::mul(CELL, ee::from_int(y + 1)));
                let (p0, p1) = ([px, py, h(x, y)], [px1, py, h(x + 1, y)]);
                let (p2, p3) = ([px, py1, h(x, y + 1)], [px1, py1, h(x + 1, y + 1)]);
                let n0 = triangle_normal(p2, p0, p3);
                let n1 = triangle_normal(p1, p3, p2);
                out.push([vu_normalize([n0[0], n0[1], n0[2], ONE]), vu_normalize([n1[0], n1[1], n1[2], ONE])]);
            }
        }
        out
    }

    /// `FIELD::CalcQuadVertexNormal` (0x005aea20): per vertex `[x][y]` (x-major)
    /// the normalised mean of six faces - both of quads (x, y) and (x-1, y-1),
    /// face 1 of (x-1, y) and face 0 of (x+1, y-1), the game's choice where
    /// (x, y-1) would be the neighbour. Float bits.
    pub fn vertex_normals(&self) -> Vec<[u32; 4]> {
        let faces = self.quad_faces();
        let q = |x: i32, y: i32| faces[x.rem_euclid(MAP as i32) as usize * MAP + y.rem_euclid(MAP as i32) as usize];
        let mut out = Vec::with_capacity(MAP * MAP);
        for x in 0..MAP as i32 {
            for y in 0..MAP as i32 {
                let mut s = vu_add(q(x, y)[0], q(x, y)[1]);
                s = vu_add(s, q(x - 1, y - 1)[0]);
                s = vu_add(s, q(x - 1, y - 1)[1]);
                s = vu_add(s, q(x - 1, y)[1]);
                s = vu_add(s, q(x + 1, y - 1)[0]);
                out.push(vu_normalize(vu_div(s, SIX)));
            }
        }
        out
    }

    /// `FIELD::CalcVertexColor` (0x005aed50) as `WORLD::Generate` runs it
    /// after the objects: `FIELD.color[x][y]` (x-major, RGB float bits, from
    /// 128) lit by the light at each vertex normal.
    pub fn vertex_colours(&self, light: &Light) -> Vec<[u32; 3]> {
        let direction = light.direction();
        let (amb, lc) = (light.ambient_rgb(), light.colour_rgb());
        self.vertex_normals().into_iter().map(|n| lambert(direction, n, [BASE_COLOUR; 3], amb, lc)).collect()
    }

    fn rewrite(&self, cells: &[MeshCell], x: i32, y: i32, scale: u32) -> Vec<(u8, i16)> {
        cells
            .iter()
            .map(|&(k, dx, dy)| (k, fixed(cell_height(&self.map, x + i32::from(dx), y + i32::from(dy)), scale)))
            .collect()
    }

    fn recolour(cells: &[MeshCell], x: i32, y: i32, colours: Option<&[[u32; 3]]>) -> Vec<(u8, [u8; 3])> {
        let Some(colours) = colours else { return Vec::new() };
        cells
            .iter()
            .map(|&(k, dx, dy)| (k, cell_colour(colours, x + i32::from(dx), y + i32::from(dy)).map(colour_byte)))
            .collect()
    }

    /// The ground tile of chip (mx, my), for a template of vertexScale
    /// `scale` (float bits; 600 for every field type's), with the colours of
    /// [`Field::vertex_colours`] when given.
    pub fn tile(&self, t: &Tables, mx: u32, my: u32, scale: u32, colours: Option<&[[u32; 3]]>) -> Tile {
        let centre = |v: u32| ee::add(CELL, ee::mul(CHIP, ee::from_int(v as i32)));
        let (x, y) = (2 * mx as i32, 2 * my as i32);
        Tile {
            chip: [mx, my],
            visible: self.check3[mx as usize * CHIPS + my as usize] != 1,
            pos: [centre(mx), centre(my), 0],
            z: self.rewrite(t.mesh.tile, x, y, scale),
            colours: Self::recolour(t.mesh.tile_colour, x, y, colours),
        }
    }

    /// A cover tile, for a template of vertexScale `scale` (300 for every
    /// field type's): `FCOVER::SetPosition`'s offset and `SetSmallMESH` for
    /// cell (2i + 1 + (t & 1), 2j + 1 + (t >> 1)).
    pub fn cover_mesh(&self, t: &Tables, cover: &Cover, scale: u32, colours: Option<&[[u32; 3]]>) -> CoverMesh {
        let [ox, oy, oz] = cover_offset(cover.quadrant);
        let centre = |v: u32| ee::add(CELL, ee::mul(CHIP, ee::from_int(v as i32)));
        let (cx, cy) =
            ((2 * cover.x + 1 + (cover.quadrant & 1)) as i32, (2 * cover.y + 1 + (cover.quadrant >> 1)) as i32);
        CoverMesh {
            mesh: cover.mesh,
            pos: [ee::add(ox, centre(cover.x)), ee::add(oy, centre(cover.y)), oz],
            z: self.rewrite(t.mesh.cover, cx, cy, scale),
            colours: Self::recolour(t.mesh.cover_colour, cx, cy, colours),
        }
    }
}

/// `FCOVER::SetPosition` (0x005b1890): a cover's offset from its chip's
/// centre by quadrant (x -300 for quadrants 0 and 2, y -300 for 0 and 1),
/// and its z, as float bits.
pub fn cover_offset(quadrant: u32) -> [u32; 3] {
    let neg = ee::sub(0, COVER_OFFSET);
    [if quadrant & 1 == 0 { neg } else { COVER_OFFSET }, if quadrant & 2 == 0 { neg } else { COVER_OFFSET }, COVER_Z]
}

/// The lake's water surface: `SetLake` plays the animation three times at
/// the lake object's centre (z 0, no rotation or scale), each drawn in its
/// own layer - one with its UVs scrolled each frame (`waterUVModifi`), two
/// with textures `Generate` makes at run time (not modelled).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Water {
    /// The CCS file (`WORLD.effccs`, `field_eff`) ...
    pub ccs: &'static str,
    /// ... and the Anime chunk in it.
    pub anime: &'static str,
    pub pos: [f32; 3],
}

/// What `WORLD::DrawBG` (gcmn 0x005a7ef0) draws: the clumps centred on the
/// player's position at z 0 (`WORLD.BGPos`), unrotated, each in its own
/// background layer; the sky material's V scrolls by 0.003 a frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Background {
    /// The background CCS file (without `.cmp`).
    pub ccs: String,
    /// The sky, two cloud layers and the mountains, and for field type 6
    /// the aurora: Clump chunks in [`Background::ccs`], in drawing order
    /// (the aurora before the mountains).
    pub clumps: Vec<&'static str>,
    pub sky_material: Option<&'static str>,
    /// `ccDrawEnv::SetFog`'s arguments.
    pub fog: super::Fog,
}

/// Everything the game draws of a field that `WORLD::Generate` decides,
/// ready for a renderer. Every transform is a translation: nothing is
/// rotated or scaled.
#[derive(Clone, Debug)]
pub struct Scene {
    /// The field's CCS file, which holds the ground, cover and object models.
    pub ccs: &'static str,
    /// The ground tile model (a `BaseMeshName`), its vertexScale, and one
    /// [`Tile`] per chip: that model at the tile's translation with the
    /// listed vertices' z (s16, `z * scale / 4096` in world units) and RGB
    /// replaced; x, y and UVs are the model's.
    pub ground_model: &'static str,
    pub ground_scale: f32,
    pub tiles: Vec<Tile>,
    /// The cover tiles, each a `SmallMeshName` model likewise rewritten; all
    /// of a field's share one vertexScale.
    pub cover_scale: f32,
    pub covers: Vec<CoverMesh>,
    pub objects: Vec<super::Placement>,
    pub water: Option<Water>,
    pub background: Background,
    /// The light the ground, the covers and (through [`object_colours`],
    /// [`Tables::lit_clumps`] times per clump) the object models are lit by.
    pub light: Light,
}

fn model_scale(c: &Ccs, name: &str) -> Result<f32> {
    let Some(obj) = c.find_object(name) else {
        return format_err(format!("{name} not in {}", c.name));
    };
    match crate::model::models(c)?.into_iter().find(|m| m.object == obj) {
        Some(m) => Ok(m.scale),
        None => format_err(format!("no Model chunk for {name}")),
    }
}

impl Field {
    /// The scene of this field: `field_ccs` is its CCS file
    /// ([`Tables::ccs`]), `background` the background file of its field type
    /// and weather ([`Backgrounds::ccs_name`](super::Backgrounds::ccs_name)).
    pub fn scene(&self, t: &Tables, field_ccs: &Ccs, background: &Ccs) -> Result<Scene> {
        let (ft, weather) = (self.params.field_type, self.params.weather);
        let Some(ccs) = t.ccs[ft as usize] else {
            return format_err(format!("field type {ft} has no field"));
        };
        let Some(row) = t.backgrounds.row(ft, weather) else {
            return format_err(format!("no background for field type {ft}, weather {weather}"));
        };
        let light = Light::from_anime(background, row.anime, row.light)?;
        let colours = self.vertex_colours(&light);
        let ground_model = t.base_mesh[ft as usize][0];
        let ground_scale = model_scale(field_ccs, ground_model)?;
        let tiles = (0..CHIPS as u32)
            .flat_map(|x| (0..CHIPS as u32).map(move |y| (x, y)))
            .map(|(x, y)| self.tile(t, x, y, ground_scale.to_bits(), Some(&colours)))
            .collect();
        let cover_scale = match self.covers.first() {
            Some(c) => model_scale(field_ccs, c.mesh)?,
            None => 0.0,
        };
        let covers = self.covers.iter().map(|c| self.cover_mesh(t, c, cover_scale.to_bits(), Some(&colours))).collect();
        let water = self.objects.iter().find(|o| o.kind == super::Kind::Lake).map(|o| Water {
            ccs: t.effect_ccs,
            anime: t.water,
            pos: [f32::from_bits(o.pos[0]), f32::from_bits(o.pos[1]), 0.0],
        });
        let mut clumps = row.clumps[..3].to_vec();
        if ft == AURORA_FIELD {
            clumps.extend(t.backgrounds.aurora.get(weather as usize));
        }
        clumps.push(row.clumps[3]);
        Ok(Scene {
            ccs,
            ground_model,
            ground_scale,
            tiles,
            cover_scale,
            covers,
            objects: self.placements(t),
            water,
            background: Background {
                ccs: t.backgrounds.ccs_name(ft, weather),
                clumps,
                sky_material: t.backgrounds.material[ft as usize].get(weather as usize).copied(),
                fog: row.fog,
            },
            light,
        })
    }
}

/// The field type whose background adds an aurora (`WORLD::Init`,
/// `WORLD::DrawBG`: `fieldType == 6`).
const AURORA_FIELD: u32 = 6;

impl Tables {
    /// The clumps `WORLD::Init` runs `CalcObjectVertexColor` over for a field
    /// type, and how many times: every row of the entrance, base, sub, key and
    /// tree tables, and of the lake table on the lake fields. Lighting changes
    /// the loaded models in place, so a clump listed twice is lit twice.
    pub fn lit_clumps(&self, field_type: u32) -> Vec<(&'static str, u32)> {
        let ft = field_type as usize;
        let mut tables = vec![&self.enter, &self.base, &self.sub, &self.key, &self.tree];
        if self.lake_fields.contains(&field_type) {
            tables.push(&self.lake);
        }
        let mut out: Vec<(&'static str, u32)> = Vec::new();
        for row in tables.into_iter().flat_map(|t| t.by_type[ft].iter()) {
            match out.iter_mut().find(|(c, _)| *c == row.clump) {
                Some((_, n)) => *n += 1,
                None => out.push((row.clump, 1)),
            }
        }
        out
    }
}
