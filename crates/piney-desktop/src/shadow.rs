//! The shadow volumes: `ccShadowModel::Draw` (main 0x001412b0) and the VU1
//! programs it runs (`mc_DrawShadow1`, `2`, `3` and `mc_DrawShadowClip`, VU1
//! 0x5d2-0x759), as `docs/engine/shadow.md` sets them out. A shadow mmat
//! ([`ShadowMesh`]) is stretched away from the light: unlit faces move along
//! it by the shadow's length, and each edge between lit and unlit becomes a
//! quad; faces wound one way count up in the shadow buffer, the others down
//! ([`Poly::front`]). A part behind the near plane is flattened onto it.

use glam::{Mat4, Vec3, Vec4};
use piney_data::shadow::ShadowMesh;
use piney_draw::{Cmd, ShadowGroup, ShadowPass, ShadowPoly};

/// What `ccShadowModel::Draw` takes from the model, the object, its view,
/// the draw environment and the shadow packet.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// The model's vertex scale (`ccModel` +0x0c).
    pub scale: f32,
    /// The object's world matrix (`ccDrawModelParam` +0x90).
    pub local_world: Mat4,
    /// The packet's view: `ccView` +0xd0 (world to GS pixels) and +0x1dc
    /// and +0x1ec (the near and far w).
    pub world_screen: Mat4,
    pub near: f32,
    pub far: f32,
    /// `ccView::GetScreenClip`: the GS pixels the buffer stands for.
    pub clip: [f32; 4],
    /// `ccDrawEnv` +0x90, the light's direction, and +0xa0, the shadow's
    /// length.
    pub light: Vec4,
    pub length: f32,
    /// The packet's +0x15c: 3 draws the model flat (no stretch).
    pub mode: u8,
    /// The buffer, `1 << tw` by `1 << th` pixels.
    pub width: u16,
    pub height: u16,
}

/// One polygon for the buffer: a fan, in buffer pixels, with GS Z.
#[derive(Clone, Debug, PartialEq)]
pub struct Poly {
    pub verts: Vec<[f32; 3]>,
    /// Wound clockwise on screen (y down): the packet's first colour and
    /// blend (`mc_DrawShadow2`'s vf09 and vf11); else the second. A model
    /// mirrored by its matrix swaps them.
    pub front: bool,
}

/// The model's box and the shadow's reach, for `mc0_CheckBoundingBoxShadow`.
fn visible(mesh: &ShadowMesh, scale: f32, to_screen: Mat4, ext: Vec3, clip: [f32; 4]) -> bool {
    let unit = scale / 4096.0;
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in &mesh.positions {
        let v = Vec3::new(f32::from(p[0]), f32::from(p[1]), f32::from(p[2])) * unit;
        (lo, hi) = (lo.min(v), hi.max(v));
    }
    // Every corner of the box and of the box moved along the light; seen
    // when not all are beyond one side of the clip box.
    let mut out = [true; 4];
    for k in 0..16 {
        let c = Vec3::new(
            if k & 1 == 0 { lo.x } else { hi.x },
            if k & 2 == 0 { lo.y } else { hi.y },
            if k & 4 == 0 { lo.z } else { hi.z },
        ) + if k & 8 == 0 { Vec3::ZERO } else { ext };
        let q = to_screen * c.extend(1.0);
        out[0] &= q.x < clip[0] * q.w;
        out[1] &= q.x > clip[2] * q.w;
        out[2] &= q.y < clip[1] * q.w;
        out[3] &= q.y > clip[3] * q.w;
    }
    !out.iter().any(|&o| o)
}

/// The polygons a shadow mmat sends this frame; empty when the box and
/// its shadow are off screen.
pub fn volume(mesh: &ShadowMesh, p: &Params) -> Vec<Poly> {
    let lw = p.local_world;
    // sceVu0InversMatrix (the transpose, as for a rotation) without its
    // translation: the light in the model's axes.
    let rt = Mat4::from_cols(
        Vec4::new(lw.x_axis.x, lw.y_axis.x, lw.z_axis.x, 0.0),
        Vec4::new(lw.x_axis.y, lw.y_axis.y, lw.z_axis.y, 0.0),
        Vec4::new(lw.x_axis.z, lw.y_axis.z, lw.z_axis.z, 0.0),
        Vec4::W,
    );
    let light = p.light.truncate();
    let local = rt.transform_vector3(light);
    // In stored units (the positions' / 4096) times the length; nothing for
    // mode 3.
    let ext = if p.mode == 3 { Vec3::ZERO } else { rt.transform_vector3(light / p.scale) * p.length };
    let view_world = p.world_screen * lw;
    if !visible(mesh, p.scale, view_world, local * p.length, p.clip) {
        return Vec::new();
    }
    // The GS pixels to the buffer's: its (4096 - w) / 2 offset, from which
    // the buffer's own pixels are counted.
    let (w, h) = (f32::from(p.width), f32::from(p.height));
    let (sx, sy) = (w / (p.clip[2] - p.clip[0]), h / (p.clip[3] - p.clip[1]));
    let (ox, oy) = ((4096.0 - w) / 2.0, (4096.0 - h) / 2.0);
    let buffer = Mat4::from_cols(
        Vec4::new(sx, 0.0, 0.0, 0.0),
        Vec4::new(0.0, sy, 0.0, 0.0),
        Vec4::Z,
        Vec4::new(ox - p.clip[0] * sx, oy - p.clip[1] * sy, 0.0, 1.0),
    );
    let m = buffer * view_world * Mat4::from_scale(Vec3::splat(p.scale));
    // The near plane's point (`view_screen` of (0, 0, max(near, 8), 1)):
    // its w is the distance, and its Z always lands past what VU1's ftoi4
    // holds (0x7fffffff), where the part of a polygon behind the plane is
    // put.
    let near_w = p.near.max(8.0);
    let near = Vec4::new(0.0, 0.0, 268_435_456.0 * near_w, near_w);
    // mc_DrawShadow1: each direction lit (the light against it) or not.
    let lit: Vec<bool> = mesh
        .normals
        .iter()
        .map(|n| {
            let n = Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32) / 16_777_216.0;
            (local.x * n.x + local.z * n.z) + local.y * n.y < 0.0
        })
        .collect();
    let lit_of = |f: i32| usize::try_from(f).ok().and_then(|f| lit.get(f).copied());
    let det = lw.x_axis.truncate().cross(lw.y_axis.truncate()).dot(lw.z_axis.truncate());
    let at = |i: u32, moved: bool| {
        let s = mesh.positions.get(i as usize).copied().unwrap_or([0; 3]);
        let v = Vec3::new(f32::from(s[0]), f32::from(s[1]), f32::from(s[2])) / 4096.0;
        m * (if moved { v + ext } else { v }).extend(1.0)
    };
    let mut out = Vec::new();
    // mc_DrawShadow2: the faces, the unlit ones moved.
    for &(t, face) in &mesh.tris {
        let moved = !lit.get(usize::from(face)).copied().unwrap_or(false);
        clip(&[at(t[0], moved), at(t[1], moved), at(t[2], moved)], near, det < 0.0, (ox, oy), &mut out);
    }
    // mc_DrawShadow3: an edge between a lit face and an unlit one. An open
    // edge's far side reads VU memory below the directions; taken as unlit.
    for e in &mesh.edges {
        let (a, b) = (lit_of(e.face_a).unwrap_or(false), lit_of(e.face_b).unwrap_or(false));
        if a == b {
            continue;
        }
        let (v0, v1) = if a { (e.v0, e.v1) } else { (e.v1, e.v0) };
        clip(&[at(v0, false), at(v1, false), at(v1, true), at(v0, true)], near, det < 0.0, (ox, oy), &mut out);
    }
    out
}

/// `mc_DrawShadowClip` and `mc_ScissorShadowPolyXY` on one polygon: its
/// part in front of the near plane, and its part behind it with each vertex
/// there put on the plane (its x and y kept, the plane's z and w). The
/// edges of the frame are left to the scissor.
fn clip(poly: &[Vec4], near: Vec4, mirrored: bool, offset: (f32, f32), out: &mut Vec<Poly>) {
    let (mut front, mut back) = (Vec::new(), Vec::new());
    let behind = |v: Vec4| v.w - near.w < 0.0;
    for (i, &a) in poly.iter().enumerate() {
        let b = poly[(i + 1) % poly.len()];
        if behind(a) {
            back.push(Vec4::new(a.x, a.y, near.z, near.w));
        } else {
            front.push(a);
        }
        if behind(a) != behind(b) {
            let (s, t) = if behind(a) { (b, a) } else { (a, b) };
            let d = t - s;
            let x = s + d * ((near.w - s.w) / d.w);
            front.push(x);
            back.push(x);
        }
    }
    for part in [front, back] {
        if part.len() < 3 {
            continue;
        }
        let verts: Vec<[f32; 3]> = part
            .iter()
            .map(|v| {
                let q = 1.0 / v.w;
                [v.x * q - offset.0, v.y * q - offset.1, v.z * q * piney_draw::MODEL_Z_SCALE]
            })
            .collect();
        let (e1, e2) = (
            [verts[1][0] - verts[0][0], verts[1][1] - verts[0][1]],
            [verts[2][0] - verts[0][0], verts[2][1] - verts[0][1]],
        );
        let cross = e1[0] * e2[1] - e1[1] * e2[0];
        out.push(Poly { verts, front: (cross < 0.0) != mirrored });
    }
}

// The packets -----------------------------------------------------------------

/// `ccShadowPacket` `+0x154`: at most 15 alphas a frame; past that a
/// shadow joins the nearest.
const MAX_GROUPS: usize = 15;

/// `zureTbl` (main 0x002fb130): the copies' offsets in sixteenths, one set
/// for each count from 1 to 4.
const ZURE: [[i32; 2]; 10] =
    [[0, 0], [16, 16], [-16, -16], [0, 14], [-14, -8], [14, -8], [16, 16], [-16, 16], [16, -16], [-16, -16]];

/// `ccShadowPacket` (0x160 bytes, `docs/engine/shadow.md`): its buffer,
/// darkness and layer, and this frame's polygons by shadow alpha.
#[derive(Clone, Debug, PartialEq)]
pub struct ShadowPacket {
    /// Its layer's priority.
    pub layer: i16,
    /// +0x15c: 1 counts front faces up and back faces down.
    pub mode: u8,
    /// `1 << tw` by `1 << th` (`SetBuffer`).
    pub width: u16,
    pub height: u16,
    /// +0x15d.
    pub darkness: u8,
    /// +0x15e: the times `TransShadowTex` lays the buffer (mode 1), and
    /// +0x15f: how far apart (`zureTbl`'s sixteenths times it, over 16).
    pub passes: u8,
    pub spread: u8,
    /// `GetPacketList`'s list: descending alpha.
    groups: Vec<ShadowGroup>,
}

impl ShadowPacket {
    pub fn new(layer: i16, width: u16, height: u16, darkness: u8) -> ShadowPacket {
        ShadowPacket { layer, mode: 1, width, height, darkness, passes: 1, spread: 0, groups: Vec::new() }
    }

    /// The same with +0x15e and +0x15f set (a stream's chunk's).
    pub fn with_copies(mut self, passes: u8, spread: u8) -> ShadowPacket {
        (self.passes, self.spread) = (passes, spread);
        self
    }

    /// `TransShadowTex`'s copies: `passes` of them (one past mode 1), each
    /// at `zureTbl[passes (passes - 1) / 2 + i] * spread >> 4` sixteenths
    /// of a pixel.
    pub fn taps(&self) -> Vec<[f32; 2]> {
        let n = if self.mode < 2 { usize::from(self.passes) } else { 1 };
        (0..n)
            .map(|i| {
                let [x, y] = ZURE.get(n * (n - 1) / 2 + i).copied().unwrap_or([0, 0]);
                let s = i32::from(self.spread);
                [((s * x) >> 4) as f32 / 16.0, ((s * y) >> 4) as f32 / 16.0]
            })
            .collect()
    }

    /// `AddPacket(alpha, ...)` through `GetPacketList` (main 0x00142ec0):
    /// the group of that alpha, else a new one after the last of higher
    /// alpha, else (15 already) the nearer of its neighbours.
    pub fn add(&mut self, alpha: u8, polys: Vec<Poly>) {
        let polys = polys.into_iter().map(|p| ShadowPoly { verts: p.verts, add: p.front });
        if let Some(g) = self.groups.iter_mut().find(|g| g.alpha == alpha) {
            g.polys.extend(polys);
            return;
        }
        let at = self.groups.iter().take_while(|g| g.alpha > alpha).count();
        if self.groups.len() < MAX_GROUPS {
            self.groups.insert(at, ShadowGroup { alpha, polys: polys.collect() });
            return;
        }
        // Full: the last of higher alpha, or the one after it when that is
        // nearer (the list's head when none is higher).
        let k = match at {
            0 => 0,
            _ if at < self.groups.len()
                && i32::from(alpha) - i32::from(self.groups[at].alpha)
                    < i32::from(self.groups[at - 1].alpha) - i32::from(alpha) =>
            {
                at
            }
            _ => at - 1,
        };
        self.groups[k].polys.extend(polys);
    }

    /// `SetShadowPacket` (0x00142b20): this frame's pass over `rect`
    /// (frame-buffer pixels), the groups emptied; None without any.
    pub fn take(&mut self, rect: [f32; 4]) -> Option<ShadowPass> {
        if self.groups.is_empty() {
            return None;
        }
        Some(ShadowPass {
            width: self.width,
            height: self.height,
            rect,
            darkness: self.darkness,
            taps: self.taps(),
            groups: std::mem::take(&mut self.groups),
        })
    }
}

/// What the shadow's packet reads of its view (sysLayer's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowView {
    /// `ccView` +0xd0: world to GS pixels.
    pub world_screen: Mat4,
    /// +0x1dc, +0x1ec.
    pub near: f32,
    pub far: f32,
    /// `GetScreenClip`, GS pixels.
    pub clip: [f32; 4],
    /// XYOFFSET in pixels: the clip less it is the frame's rectangle.
    pub offset: [f32; 2],
}

/// The draw environment's shadow (`ccDrawEnv` +0x8c to +0xa4) and the
/// packets `WORLD_MAN::GO` made (`ccShadowPacket::root`).
#[derive(Clone, Debug, PartialEq)]
pub struct Shadows {
    pub packets: Vec<ShadowPacket>,
    /// +0x8c: the packet shadows go to; None casts nothing.
    pub active: Option<usize>,
    /// +0x90: the light's direction (`WORLD_MAN` +0x90, the area's distant
    /// light).
    pub light: Vec4,
    /// +0xa0 (`Reset`: 1000).
    pub length: f32,
    /// +0xa4 (`Reset`: 128).
    pub alpha: u8,
    pub view: Option<ShadowView>,
}

impl Default for Shadows {
    fn default() -> Shadows {
        Shadows {
            packets: Vec::new(),
            active: None,
            light: Vec4::new(0.0, 0.0, -1.0, 0.0),
            length: 1000.0,
            alpha: 128,
            view: None,
        }
    }
}

/// The two packets' layer: `WORLD_MAN` +0x4bc's (objLayer, 0) plus 2.
pub const SHADOW_LAYER: i16 = 2;
/// [`Shadows::field`]'s packets: the characters' and TOBJ's.
pub const CHAR_PACKET: usize = 0;
pub const TOBJ_PACKET: usize = 1;

impl Shadows {
    /// `WORLD_MAN::GO`'s packets: 256 x 256 at darkness 0x30 laid once
    /// (the characters'), 128 x 128 at 0x20 laid twice 14 sixteenths apart
    /// (TOBJ's), the first active, the light from the area.
    pub fn field(light: Vec4, view: ShadowView) -> Shadows {
        Shadows {
            packets: vec![
                ShadowPacket { spread: 14, ..ShadowPacket::new(SHADOW_LAYER, 256, 256, 0x30) },
                ShadowPacket { passes: 2, spread: 14, ..ShadowPacket::new(SHADOW_LAYER, 128, 128, 0x20) },
            ],
            active: Some(CHAR_PACKET),
            light,
            view: Some(view),
            ..Shadows::default()
        }
    }

    /// `ccShadowModel::Draw` of a shadow mesh at `local_world`, with the
    /// environment's alpha, length and light, into the active packet.
    pub fn cast(&mut self, mesh: &ShadowMesh, scale: f32, local_world: Mat4) {
        let (Some(k), Some(v)) = (self.active, self.view) else { return };
        if self.alpha == 0 {
            return;
        }
        let Some(pk) = self.packets.get_mut(k) else { return };
        if pk.mode == 0 {
            return;
        }
        let p = Params {
            scale,
            local_world,
            world_screen: v.world_screen,
            near: v.near,
            far: v.far,
            clip: v.clip,
            light: self.light,
            length: self.length,
            mode: pk.mode,
            width: pk.width,
            height: pk.height,
        };
        let polys = volume(mesh, &p);
        if !polys.is_empty() {
            pk.add(self.alpha.min(128), polys);
        }
    }

    /// `SetShadowPacketAll` (0x00142e70): each packet's pass, by layer, in
    /// the order the GS gets them (of one priority the newer first).
    pub fn flush(&mut self) -> Vec<(i16, Cmd)> {
        let Some(v) = self.view else { return Vec::new() };
        let rect = [v.clip[0] - v.offset[0], v.clip[1] - v.offset[1], v.clip[2] - v.offset[0], v.clip[3] - v.offset[1]];
        let mut out: Vec<(i16, Cmd)> =
            self.packets.iter_mut().filter_map(|p| Some((p.layer, Cmd::Shadow(Box::new(p.take(rect)?))))).collect();
        out.reverse();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poly(front: bool) -> Poly {
        Poly { verts: vec![[0.0, 0.0, 1.0], [4.0, 0.0, 1.0], [0.0, 4.0, 1.0]], front }
    }

    /// `GetPacketList`: one group per alpha, kept in descending alpha; the
    /// sixteenth alpha joins its nearer neighbour.
    #[test]
    fn groups_by_alpha_descending() {
        let mut p = ShadowPacket::new(2, 256, 256, 0x30);
        for a in [64, 128, 64, 100, 1] {
            p.add(a, vec![poly(true)]);
        }
        let alphas: Vec<u8> = p.groups.iter().map(|g| g.alpha).collect();
        assert_eq!(alphas, [128, 100, 64, 1]);
        assert_eq!(p.groups[2].polys.len(), 2);
        let mut full = ShadowPacket::new(2, 256, 256, 0x30);
        for a in (0..MAX_GROUPS as u8).map(|k| 120 - 8 * k) {
            full.add(a, vec![poly(false)]);
        }
        // 111: between 112 (nearer) and 104.
        full.add(111, vec![poly(true)]);
        assert_eq!(full.groups.len(), MAX_GROUPS);
        assert_eq!(full.groups.iter().find(|g| g.alpha == 112).unwrap().polys.len(), 2);
        let pass = full.take([0.0, 0.0, 512.0, 448.0]).unwrap();
        assert_eq!(pass.groups.len(), MAX_GROUPS);
        assert!(full.take([0.0; 4]).is_none());
    }

    /// A box standing on the ground under a light from above casts: its
    /// top faces lit and kept, the bottom moved down by the length, the
    /// sides' edges become quads; front and back faces both present.
    #[test]
    fn a_box_casts_a_closed_volume() {
        use piney_data::model::{Kind, Mmat};
        let s = 1024i16;
        let positions: Vec<[i16; 3]> =
            (0..8).map(|k| [[0, s][k & 1], [0, s][k >> 1 & 1], [0, s][k >> 2 & 1]]).collect();
        // Outward-facing triangles of the unit cube.
        let quads = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
        let triangles = quads.iter().flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]]).collect();
        let mm = Mmat {
            kind: Kind::Shadow,
            name: None,
            material: None,
            slot: None,
            positions,
            normals: Vec::new(),
            flags: Vec::new(),
            colours: Vec::new(),
            uvs: Vec::new(),
            skin: Vec::new(),
            triangles,
        };
        let mesh = ShadowMesh::build(&mm).expect("a closed box");
        assert_eq!(mesh.normals.len(), 6);
        assert_eq!(mesh.edges.len(), 12);
        // A camera 500 back and above, looking at the box.
        // Camera space: x right, y down, z (and w) the depth ahead.
        let (f, u, r) = (Vec3::new(0.0, 500.0, -300.0).normalize(), Vec3::Z, Vec3::X);
        let up = r.cross(f).normalize() * u.dot(r.cross(f)).signum();
        let eye = Vec3::new(0.0, -500.0, 300.0);
        let view = Mat4::from_cols(
            Vec4::new(r.x, -up.x, f.x, 0.0),
            Vec4::new(r.y, -up.y, f.y, 0.0),
            Vec4::new(r.z, -up.z, f.z, 0.0),
            Vec4::new(-r.dot(eye), up.dot(eye), -f.dot(eye), 1.0),
        );
        // GS pixels about (2048, 2048) and a Z falling with the depth.
        let proj = Mat4::from_cols(
            Vec4::new(400.0, 0.0, 0.0, 0.0),
            Vec4::new(0.0, 400.0, 0.0, 0.0),
            Vec4::new(2048.0, 2048.0, 0.0, 1.0),
            Vec4::new(0.0, 0.0, 1.0e8, 0.0),
        );
        let to_gs = Mat4::IDENTITY;
        let p = Params {
            scale: 64.0,
            local_world: Mat4::IDENTITY,
            world_screen: to_gs * proj * view,
            near: 8.0,
            far: 1_048_576.0,
            clip: [1792.0, 1824.0, 2304.0, 2272.0],
            light: Vec4::new(0.3, 0.2, -1.0, 0.0).normalize(),
            length: 100.0,
            mode: 1,
            width: 256,
            height: 256,
        };
        let polys = volume(&mesh, &p);
        // 12 faces and the silhouette's quads (4 of the box's 12 edges at
        // the least).
        assert!(polys.len() >= 16, "{}", polys.len());
        assert!(polys.iter().any(|q| q.front) && polys.iter().any(|q| !q.front));
        assert_eq!(polys.iter().filter(|q| q.verts.len() == 4).count() + 12, polys.len());
    }
}
