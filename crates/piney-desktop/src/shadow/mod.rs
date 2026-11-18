//! The shadow volumes (`docs/engine/shadow.md`): a shadow mmat
//! ([`ShadowMesh`]) stretched away from the light into a closed volume whose
//! faces count up and down in a small buffer against the frame's Z. [`draw`]
//! is `ccShadowModel::Draw`'s EE half, [`kicks`] VU1's programs, giving what
//! VU1 sends the GS word for word ([`Kick`]), and [`gs_triangles`] what the
//! GS draws of it; [`ShadowPacket`] and [`Shadows`] the packets a frame
//! collects them in and the passes that count and lay them.

mod draw;
mod gif;
mod vu1;

use std::sync::Arc;

use piney_data::shadow::ShadowMesh;
use piney_data::vu0::{F, M4, V4};
use piney_draw::{Cmd, ShadowGroup, ShadowPass};

pub use draw::{Blend, Mode, Params, Prelude, Setup, ShadowView, Vf21, draw, model_box};
pub use gif::{Kick, Polygon, Qword, Xyz2, gs_triangles};
pub use vu1::{Vu1Carry, kicks};

const MINUS_ONE: F = 0xbf80_0000;

// The packets -----------------------------------------------------------------

/// `ccShadowPacket` `+0x154`: at most 15 alphas a frame; past that a
/// shadow joins the nearest.
const MAX_GROUPS: usize = 15;

/// `zureTbl` (main 0x002fb130): the copies' offsets in sixteenths, one set
/// for each count from 1 to 4.
const ZURE: [[i32; 2]; 10] =
    [[0, 0], [16, 16], [-16, -16], [0, 14], [-14, -8], [14, -8], [16, 16], [-16, 16], [16, -16], [-16, -16]];

/// A volume `AddPacket` took: its mesh and what `Draw` sent VU1.
#[derive(Clone, Debug, PartialEq)]
struct Cast {
    mesh: Arc<ShadowMesh>,
    setup: Setup,
}

/// One alpha's volumes, in the order `AddPacket` chained them.
#[derive(Clone, Debug, PartialEq)]
struct Group {
    alpha: u8,
    casts: Vec<Cast>,
}

/// `ccShadowPacket` (0x160 bytes, `docs/engine/shadow.md`): its buffer,
/// darkness and layer, and this frame's volumes by shadow alpha.
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
    groups: Vec<Group>,
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
    fn add(&mut self, alpha: u8, cast: Cast) {
        if let Some(g) = self.groups.iter_mut().find(|g| g.alpha == alpha) {
            g.casts.push(cast);
            return;
        }
        let at = self.groups.iter().take_while(|g| g.alpha > alpha).count();
        if self.groups.len() < MAX_GROUPS {
            self.groups.insert(at, Group { alpha, casts: vec![cast] });
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
        self.groups[k].casts.push(cast);
    }

    /// `SetShadowPacket` (0x00142b20): this frame's pass over `rect`
    /// (frame-buffer pixels), the groups emptied; None without any. VU1
    /// runs the volumes as the GS gets them: the last group first, each
    /// group's in `AddPacket`'s order.
    pub fn take(&mut self, rect: [f32; 4], carry: &mut Vu1Carry, add: u64) -> Option<ShadowPass> {
        if self.groups.is_empty() {
            return None;
        }
        let mut groups: Vec<ShadowGroup> = std::mem::take(&mut self.groups)
            .into_iter()
            .rev()
            .map(|g| ShadowGroup {
                alpha: g.alpha,
                tris: g
                    .casts
                    .iter()
                    .flat_map(|c| gs_triangles(&kicks(&c.mesh, &c.setup, carry), c.setup.offset, add))
                    .collect(),
            })
            .collect();
        groups.reverse();
        Some(ShadowPass {
            width: self.width,
            height: self.height,
            rect,
            darkness: self.darkness,
            taps: self.taps(),
            groups,
        })
    }
}

/// The draw environment's shadow (`ccDrawEnv` +0x8c to +0xa4), the packets
/// `WORLD_MAN::GO` made (`ccShadowPacket::root`), and the VU state the
/// shadow draws leave for the next.
#[derive(Clone, Debug, PartialEq)]
pub struct Shadows {
    pub packets: Vec<ShadowPacket>,
    /// +0x8c: the packet shadows go to; None casts nothing.
    pub active: Option<usize>,
    /// +0x90: the light's direction (`WORLD_MAN` +0x90, the area's distant
    /// light), as float bits.
    pub light: V4,
    /// +0xa0 (`Reset`: 1000).
    pub length: F,
    /// +0xa4 (`Reset`: 128).
    pub alpha: u8,
    pub view: Option<ShadowView>,
    /// VU0's vf21 as the last object's `ccObj::Draw` left it, which the
    /// shadow's check reads ([`Prelude`]).
    pub vf21: Vf21,
    /// What the last pass left in VU1's memory.
    pub vu1: Vu1Carry,
}

impl Default for Shadows {
    fn default() -> Shadows {
        Shadows {
            packets: Vec::new(),
            active: None,
            light: [0, 0, MINUS_ONE, 0],
            length: 0x447a_0000,
            alpha: 128,
            view: None,
            vf21: Vf21::Known([0; 4]),
            vu1: Vu1Carry::default(),
        }
    }
}

/// The two packets' layer: `WORLD_MAN` +0x4bc's (objLayer, 0) plus 2.
pub const SHADOW_LAYER: i16 = 2;
/// [`Shadows::field`]'s packets: the characters' and TOBJ's.
pub const CHAR_PACKET: usize = 0;
pub const TOBJ_PACKET: usize = 1;

/// `alphaBlendTbl` (alike on every disc).
fn alpha_blend() -> &'static [u64] {
    piney_data::tables::kanji::shared().alpha_blend_tbl
}

impl Shadows {
    /// `WORLD_MAN::GO`'s packets: 256 x 256 at darkness 0x30 laid once
    /// (the characters'), 128 x 128 at 0x20 laid twice 14 sixteenths apart
    /// (TOBJ's), the first active, the light from the area.
    pub fn field(light: V4, view: ShadowView) -> Shadows {
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

    /// The owner's `ccObj::Draw` up to its shadow ([`Prelude`]), then
    /// `ccShadowModel::Draw` of a shadow mesh at `local_world`, with the
    /// environment's alpha, length and light, into the active packet.
    pub fn cast(&mut self, mesh: &Arc<ShadowMesh>, scale: F, local_world: M4, prelude: Prelude) {
        let Some(v) = self.view else { return };
        self.vf21 = prelude.vf21(self.vf21, &v, &local_world);
        let Some(k) = self.active else { return };
        if self.alpha == 0 || mesh.normals.is_empty() {
            return;
        }
        let Some(pk) = self.packets.get_mut(k) else { return };
        let Some(mode) = Mode::of(pk.mode) else { return };
        let p = Params {
            scale,
            local_world,
            light: self.light,
            length: self.length,
            mode,
            tw: pk.width.trailing_zeros() as u8,
            th: pk.height.trailing_zeros() as u8,
        };
        if let Some(setup) = draw(mesh, &v, &p, alpha_blend(), &mut self.vf21) {
            pk.add(self.alpha.min(128), Cast { mesh: Arc::clone(mesh), setup });
        }
    }

    /// `SetShadowPacketAll` (0x00142e70): each packet's pass, by layer, in
    /// the order the GS gets them (of one priority the newer first).
    pub fn flush(&mut self) -> Vec<(i16, Cmd)> {
        let Some(v) = self.view else { return Vec::new() };
        let c = v.clip.map(f32::from_bits);
        let rect = [c[0] - v.offset[0], c[1] - v.offset[1], c[2] - v.offset[0], c[3] - v.offset[1]];
        let add = alpha_blend().get(1).copied().unwrap_or(0);
        let carry = &mut self.vu1;
        let mut out: Vec<(i16, Cmd)> = self
            .packets
            .iter_mut()
            .filter_map(|p| Some((p.layer, Cmd::Shadow(Box::new(p.take(rect, carry, add)?)))))
            .collect();
        out.reverse();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piney_data::vu0;

    fn cast() -> Cast {
        let blend = Blend { colour: 0, alpha: 0 };
        let setup = Setup {
            scissor: [[0; 4]; 2],
            near: [0; 4],
            guard: [[0; 4]; 2],
            m: vu0::UNIT,
            blends: [blend; 2],
            light: [0; 4],
            stretch: [0; 4],
            offset: [0; 2],
        };
        Cast { mesh: Arc::new(ShadowMesh::default()), setup }
    }

    /// `GetPacketList`: one group per alpha, kept in descending alpha; the
    /// sixteenth alpha joins its nearer neighbour.
    #[test]
    fn groups_by_alpha_descending() {
        let mut p = ShadowPacket::new(2, 256, 256, 0x30);
        for a in [64, 128, 64, 100, 1] {
            p.add(a, cast());
        }
        let alphas: Vec<u8> = p.groups.iter().map(|g| g.alpha).collect();
        assert_eq!(alphas, [128, 100, 64, 1]);
        assert_eq!(p.groups[2].casts.len(), 2);
        let mut full = ShadowPacket::new(2, 256, 256, 0x30);
        for a in (0..MAX_GROUPS as u8).map(|k| 120 - 8 * k) {
            full.add(a, cast());
        }
        // 111: between 112 (nearer) and 104.
        full.add(111, cast());
        assert_eq!(full.groups.len(), MAX_GROUPS);
        assert_eq!(full.groups.iter().find(|g| g.alpha == 112).unwrap().casts.len(), 2);
        let pass = full.take([0.0, 0.0, 512.0, 448.0], &mut Vu1Carry::default(), 0x48).unwrap();
        assert_eq!(pass.groups.len(), MAX_GROUPS);
        assert!(full.take([0.0; 4], &mut Vu1Carry::default(), 0x48).is_none());
    }

    /// A box under a light from above casts a closed volume: its faces and
    /// the silhouette's quads, front and back; with no Z to stop them, every
    /// buffer pixel counts as many up as down.
    #[test]
    fn a_box_casts_a_closed_volume() {
        use piney_data::model::{Kind, Mmat};
        let k = f32::to_bits;
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
        // Looking down -z from 500 up: GS pixels about (2048, 2048), w the
        // depth, Z falling with it.
        let screen: M4 = [
            [k(400.0), 0, 0, 0],
            [0, k(-400.0), 0, 0],
            [k(-2048.0), k(-2048.0), 0, k(-1.0)],
            [k(1_024_000.0), k(1_024_000.0), k(1.0e8), k(500.0)],
        ];
        let view = ShadowView {
            world_screen: screen,
            view_screen: vu0::UNIT,
            near: k(8.0),
            far: k(1_048_576.0),
            clip: [k(1792.0), k(1824.0), k(2304.0), k(2272.0)],
            offset: [1792.0, 1824.0],
        };
        let p = Params {
            scale: k(64.0),
            local_world: vu0::UNIT,
            light: [k(0.3), k(0.2), k(-0.93), 0],
            length: k(100.0),
            mode: Mode::Count,
            tw: 8,
            th: 8,
        };
        let mut vf21 = Vf21::Known([0; 4]);
        let setup = draw(&mesh, &view, &p, &[0x44, 0x48, 0x42], &mut vf21).expect("in view");
        // The check leaves its last corner: the reach's x, y, z from vf21's.
        assert!(matches!(vf21, Vf21::Known(v) if v[3] != 0));
        let tris = gs_triangles(&kicks(&mesh, &setup, &mut Vu1Carry::default()), setup.offset, 0x48);
        // 12 faces and at least four silhouette quads of two triangles.
        assert!(tris.len() >= 12 + 8, "{}", tris.len());
        assert!(tris.iter().any(|t| t.add) && tris.iter().any(|t| !t.add));
        let mut count = vec![0i32; 256 * 256];
        let mut touched = 0;
        for t in &tris {
            t.pixels(256, 256, |x, y, _| {
                count[y * 256 + x] += if t.add { 1 } else { -1 };
                touched += 1;
            });
        }
        assert!(touched > 0);
        assert!(count.iter().all(|&n| n == 0));
    }

    /// `ccObj::Draw` before the shadow: `ccModel::Draw`'s check leaves the
    /// matrix times the box's greatest corner whatever `_SetLWMatrix` left;
    /// without it a parent leaves the chain's qword, and nothing else
    /// touches vf21.
    #[test]
    fn the_owners_draw_sets_vf21() {
        let k = f32::to_bits;
        let view = ShadowView {
            world_screen: vu0::UNIT,
            view_screen: vu0::UNIT,
            near: k(8.0),
            far: k(1_048_576.0),
            clip: [0; 4],
            offset: [0.0; 2],
        };
        let mut lw = vu0::UNIT;
        lw[3] = [k(10.0), k(20.0), k(30.0), k(1.0)];
        let before = Vf21::Known([k(5.0); 4]);
        let max = [k(1.0), k(2.0), k(3.0), k(1.0)];
        let checked = Prelude { parent: true, checked: Some(max) };
        assert_eq!(checked.vf21(before, &view, &lw), Vf21::Known([k(11.0), k(22.0), k(33.0), k(1.0)]));
        assert_eq!(Prelude { parent: true, checked: None }.vf21(before, &view, &lw), Vf21::Chain);
        assert_eq!(Prelude::default().vf21(before, &view, &lw), before);
    }
}
