//! What VU1 kicks to the GS for a shadow draw ([`Kick`]): the batches'
//! polygons, the clipped polygons' fans, and the tag kick before each fan.
//! [`Kick::words`] is the GIF packet as the GS takes it (PATH1, PACKED),
//! for comparing with the game's word for word; [`gs_triangles`] the
//! triangles the GS draws of it.

use piney_draw::ShadowTri;

use super::draw::Blend;

/// A VU1 quadword.
pub type Qword = [u32; 4];

/// GS registers by A+D address.
const RGBAQ: u32 = 0x01;
const ALPHA_2: u32 = 0x43;
const EOP: u32 = 0x8000;
/// `DecodeShadowModel`'s batch tags past NLOOP and EOP (TRIANGLE and
/// TRISTRIP, ABE, context 2; A+D, A+D, then three or four XYZ2).
const TRIANGLE_TAG: [u32; 3] = [0x5121_c000, 0x0005_55ee, 0];
const STRIP_TAG: [u32; 3] = [0x6122_4000, 0x0055_55ee, 0];
/// The packet's 0x18d, a TRIFAN tag (two A+D, EOP, PRE), which
/// `mc_DrawShadow1` copies to VU1 0x203 and 0x204.
pub const FAN_TAG: Qword = [0x0000_8002, 0x1122_c000, 0xeeee_eeee, 0xeeee_eeee];
/// The fan's vertex tag (0x18e, copied to 0x207): XYZ2, its NLOOP set per
/// fan.
const FAN_VERTEX_TAG: [u32; 3] = [0x1000_0000, 5, 0];

/// An XYZ2 as VU1 stores it: `FTOI4` of the divided x, y and z (the GS
/// takes x's and y's low 16 bits), and the w word an `ISW` wrote, whose bit
/// 15 is the ADC (the vertex draws no triangle).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Xyz2 {
    pub xyz: [u32; 3],
    pub w: u32,
}

impl Xyz2 {
    fn adc(&self) -> bool {
        self.w & 0x8000 != 0
    }

    fn word(&self) -> Qword {
        [self.xyz[0], self.xyz[1], self.xyz[2], self.w]
    }
}

/// A triangle or a quad of a batch, with its blend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Polygon {
    pub blend: Blend,
    pub verts: Vec<Xyz2>,
}

/// One `XGKICK` of a shadow draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kick {
    /// `mc_DrawShadow2`'s batch, a TRIANGLE list: every triangle, the
    /// clipped ones with their third vertex's ADC set.
    Triangles(Vec<Polygon>),
    /// `mc_DrawShadow3`'s batch, a TRISTRIP: each silhouette quad `(a, b,
    /// a + e, b + e)`, its first two vertices' ADC set.
    Quads(Vec<Polygon>),
    /// `mc_ScissorShadowPolyXY`'s first kick, of VU1 0x203: the fan's tag
    /// with 0x204 (the tag again, its NLOOP 2 once a fan was sent this draw)
    /// and 0x205 (the colour register the last fan was given, zero before
    /// any) as its two A+D: it sets PRIM and nothing that draws.
    FanTag { fan_sent: bool, colour: Option<u64> },
    /// A clipped polygon, from 0x204: its blend, then its vertices as a
    /// TRIFAN, the w words counting down from 0x8001.
    Fan { blend: Blend, verts: Vec<Xyz2> },
}

/// An A+D quadword.
fn a_d(value: u64, reg: u32) -> Qword {
    [value as u32, (value >> 32) as u32, reg, 0]
}

impl Blend {
    fn words(&self) -> [Qword; 2] {
        [a_d(self.colour, RGBAQ), a_d(self.alpha, ALPHA_2)]
    }
}

impl Kick {
    /// The GIF packet, as VU1 memory holds it when kicked.
    pub fn words(&self) -> Vec<Qword> {
        let batch = |tag: [u32; 3], loops: usize, polys: &[Polygon]| -> Vec<Qword> {
            let head = [EOP | loops as u32, tag[0], tag[1], tag[2]];
            std::iter::once(head)
                .chain(polys.iter().flat_map(|p| p.blend.words().into_iter().chain(p.verts.iter().map(Xyz2::word))))
                .collect()
        };
        match self {
            Kick::Triangles(polys) => batch(TRIANGLE_TAG, polys.len(), polys),
            Kick::Quads(polys) => batch(STRIP_TAG, polys.len(), polys),
            Kick::FanTag { fan_sent, colour } => {
                let mut again = FAN_TAG;
                if *fan_sent {
                    again[0] = 2;
                }
                vec![FAN_TAG, again, colour.map_or([0; 4], |c| a_d(c, RGBAQ))]
            }
            Kick::Fan { blend, verts } => {
                let mut head = FAN_TAG;
                head[0] = 2;
                let vertex_tag = [(verts.len() as u32 + EOP) & 0xffff, FAN_VERTEX_TAG[0], FAN_VERTEX_TAG[1], 0];
                [head]
                    .into_iter()
                    .chain(blend.words())
                    .chain([vertex_tag])
                    .chain(verts.iter().map(Xyz2::word))
                    .collect()
            }
        }
    }
}

/// The triangles the GS draws of the kicks: a TRIANGLE list's third vertex
/// without ADC, a TRISTRIP's (each quad's first two vertices have it, so a
/// strip never joins two quads), a TRIFAN's from the third. Of a fan tag's
/// kick, none. A triangle counts up when its ALPHA is `add`
/// (`alphaBlendTbl[1]`); `offset` is the buffer's XYOFFSET in sixteenths.
pub fn gs_triangles(kicks: &[Kick], offset: [i32; 2], add: u64) -> Vec<ShadowTri> {
    let tri = |blend: &Blend, [a, b, c]: [&Xyz2; 3]| ShadowTri {
        xy: [a, b, c].map(|v| [(v.xyz[0] & 0xffff) as i32 - offset[0], (v.xyz[1] & 0xffff) as i32 - offset[1]]),
        z: [a, b, c].map(|v| v.xyz[2]),
        add: blend.alpha == add,
    };
    let mut out = Vec::new();
    for kick in kicks {
        match kick {
            Kick::Triangles(polys) => out.extend(polys.iter().filter_map(|p| match &p.verts[..] {
                [a, b, c] if !c.adc() => Some(tri(&p.blend, [a, b, c])),
                _ => None,
            })),
            Kick::Quads(polys) => {
                for p in polys {
                    for w in p.verts.windows(3) {
                        if !w[2].adc() {
                            out.push(tri(&p.blend, [&w[0], &w[1], &w[2]]));
                        }
                    }
                }
            }
            Kick::FanTag { .. } => {}
            Kick::Fan { blend, verts } => {
                for k in 2..verts.len() {
                    if !verts[k].adc() {
                        out.push(tri(blend, [&verts[0], &verts[k - 1], &verts[k]]));
                    }
                }
            }
        }
    }
    out
}
