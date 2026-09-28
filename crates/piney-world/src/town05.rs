//! Lia Fail: `ROOTTOWN05` (OUT gcmn town05.cpp, constructor 0x0043c5e0, `Draw`
//! 0x0043e8e0 through the vtable at 0x00384300), the fifth Root Town
//! (`game.town` 4, the server Omega), new in Outbreak (Quarantine's is the
//! same). A sky, 7 static models, and 19 glows (`EFF_se1_1ef1`) at the omni
//! points that wait, then flicker through patterns from `fieldrand`
//! (docs/engine/root-towns.md).

use std::collections::HashMap;
use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::statics::DrawPass;
use piney_data::{Error, Result};
use piney_desktop::layers::Layers;

use crate::cloud;
use crate::draw::{EFF_LAYER, OBJ_LAYER};
use crate::ee::{self, F, ONE, V4};
use crate::town::{Base, FogParams, RootTown, Spec, Town, TownSprite, TownView, clump_models};

/// `ChangeScene`'s town number.
pub const NO: i32 = 4;
pub const FILE: &str = "town05";
/// `DrawBG`'s clump.
pub const SKY: &str = "CMP_sr5bac1";
/// The light animation and `town05Light` (OUT gcmn 0x006013d0).
pub const LIGHT_ANIM: &str = "ANM_sr5bac1a";
pub const LIGHTS: [(i32, &str); 20] = [
    (0, "LGT_fdirect01"),
    (1, "LGT_omni01"),
    (1, "LGT_omni02"),
    (1, "LGT_omni03"),
    (1, "LGT_omni04"),
    (1, "LGT_omni05"),
    (1, "LGT_omni06"),
    (1, "LGT_omni07"),
    (1, "LGT_omni08"),
    (1, "LGT_omni09"),
    (1, "LGT_omni10"),
    (1, "LGT_omni11"),
    (1, "LGT_omni12"),
    (1, "LGT_omni13"),
    (1, "LGT_omni14"),
    (1, "LGT_omni15"),
    (1, "LGT_omni16"),
    (1, "LGT_omni17"),
    (1, "LGT_omni18"),
    (1, "LGT_omni19"),
];
/// The omni points' glow and how many there are (`DMY_omnpoint%.2d`, 1
/// on).
pub const GLOW: &str = "EFF_se1_1ef1";
pub const GLOWS: usize = 19;

/// `SetFog(1500, 10000, 0, 75, 0x00f0c080)` (0x0043c83c) and the same
/// colour as `ccSys.bgColor`: Dun Loireag's pale blue.
pub const FOG: FogParams =
    FogParams { near: 1500.0, far: 10000.0, near_rate: 0.0, far_rate: 75.0, colour: [0x80, 0xc0, 0xf0] };
pub const BG_COLOR: [u8; 3] = [0x80, 0xc0, 0xf0];

const SPEC: Spec = Spec {
    no: NO,
    models: "RT_MODELTABLE05",
    objects: None,
    light_anim: LIGHT_ANIM,
    lights: &LIGHTS,
    fog: FOG,
    clear: Some(BG_COLOR),
};

/// `DrawBG`'s scrolls a frame (OUT gp 0x00386ee0..0x00386ee8): 0.01, 0.02
/// and 0.001.
const BG_STEP: [F; 3] = [0x3c23_d70a, 0x3ca3_d70a, 0x3a83_126f];
/// The area generator's `seed` as the executable holds it.
pub const FIELD_SEED: u32 = 13;

/// `WORLD_MAN`'s layers the town draws on besides `objLayer`.
pub mod layer {
    /// `bgLayer[0]` (+0x494).
    pub const BG0: i16 = -100;
    /// `SetActiveLayer(3)`: effLayer.
    pub const EFF: i16 = crate::draw::EFF_LAYER;
}

/// One omni point's glow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glow {
    /// +0x330: the pattern it draws while it waits.
    pub frame: u16,
    /// +0x37c: the frames it waits before it flickers.
    pub wait: i32,
    /// +0x3c8: the frames it flickers.
    pub flicker: i32,
}

/// One piece `ROOTTOWN05::Draw` draws, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// `DrawBG`: the sky clump at the origin.
    Sky,
    /// A `STATICMODEL` (its `RT_MODELTABLE` row).
    Model(usize),
    /// `DrawMap`: the minimap (the host draws it).
    Map,
    /// Omni point `k`'s glow: its `ccEff` drawn with a pattern.
    Glow(usize, u16),
}

/// What `ROOTTOWN05` holds besides the [`Base`].
pub struct LiaFail {
    sky: Vec<u32>,
    /// `DrawBG`'s scrolls (read by nothing).
    pub bg_scroll: [F; 3],
    /// The omni points (w 1), their glows and the glow's `patNum`.
    pub points: [V4; GLOWS],
    pub glows: [Glow; GLOWS],
    pat_num: u16,
    /// `fieldrand` as the glows draw from it.
    pub rng: Rng,
}

/// `ROOTTOWN05::ROOTTOWN05`: `town05` (whatever the crisis byte says).
pub fn new(archive: &Arc<Archive>, _crisis: bool) -> Result<Town> {
    let base = Base::read(archive, FILE, &SPEC)?;
    let file = base.file.clone();
    let pat_num = cloud::eff_pat_num(&file.ccs, GLOW).ok_or_else(|| Error::NotFound(format!("{FILE}: {GLOW}")))?.max(1);
    let mut rng = Rng::new(FIELD_SEED);
    let glows = std::array::from_fn(|_| {
        let frame = rng.below(u32::from(pat_num)) as u16;
        let wait = rng.below(100) as i32 + 30;
        let flicker = rng.below(15) as i32 + 1;
        Glow { frame, wait, flicker }
    });
    let mut points = [[0, 0, 0, ONE]; GLOWS];
    for (k, p) in points.iter_mut().enumerate() {
        let name = format!("DMY_omnpoint{:02}", k + 1);
        *p = file
            .ccs
            .find_object(&name)
            .and_then(|o| file.scene.dummies.get(&o))
            .map(|d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE])
            .ok_or(Error::NotFound(name))?;
    }
    let d = LiaFail { sky: clump_models(&file, SKY), bg_scroll: [0; 3], points, glows, pat_num, rng };
    Ok(Town::new(base, d))
}

impl LiaFail {
    /// `ROOTTOWN05::Draw` for this frame: the pieces in its order, with the
    /// scrolls and the glows stepped as it steps them.
    pub fn select(&mut self, base: &mut Base, _v: &TownView) -> Vec<Piece> {
        let mut out = Vec::new();
        // DrawBG (0x0043d200).
        for (x, step) in self.bg_scroll.iter_mut().zip(BG_STEP) {
            *x = ee::add(*x, step);
        }
        for x in &mut self.bg_scroll {
            if !ee::le(*x, ONE) {
                *x = 0;
            }
        }
        out.push(Piece::Sky);
        // DrawObj2, DrawObj, DrawFloor, DrawMap, then the type-0 rows.
        for pass in [DrawPass::Obj2, DrawPass::Obj, DrawPass::Floor] {
            out.extend(base.rows(pass).map(Piece::Model));
        }
        out.push(Piece::Map);
        out.extend(base.rows(DrawPass::Other).map(Piece::Model));
        for (k, g) in self.glows.iter_mut().enumerate() {
            if g.wait == 0 {
                let pattern = self.rng.below(u32::from(self.pat_num)) as u16;
                out.push(Piece::Glow(k, pattern));
                g.flicker -= 1;
                if g.flicker == 0 {
                    g.wait = self.rng.below(100) as i32 + 30;
                    g.flicker = self.rng.below(15) as i32 + 1;
                }
            } else {
                g.wait -= 1;
                out.push(Piece::Glow(k, g.frame));
                g.frame += 1;
                if g.frame == self.pat_num {
                    g.frame = 0;
                }
            }
        }
        out
    }

    /// The pieces into the field's layers; the glows are handed back as
    /// sprites.
    fn draw_pieces(&self, base: &Base, layers: &mut Layers, to_screen: Mat4, pieces: &[Piece]) -> Vec<TownSprite> {
        let none = HashMap::new();
        let mut sprites = Vec::new();
        for &piece in pieces {
            match piece {
                Piece::Sky => base.clump_draw(layers, layer::BG0, to_screen, &self.sky, Mat4::IDENTITY, &none),
                Piece::Model(row) => base.row_draw(layers, OBJ_LAYER, to_screen, row),
                Piece::Map => {}
                Piece::Glow(k, pattern) => sprites.push(TownSprite {
                    file: FILE,
                    eff: GLOW,
                    pos: self.points[k],
                    pattern,
                    layer: EFF_LAYER,
                    scale: [ONE; 2],
                    rotate: 0,
                    transparency: ONE,
                    depth: true,
                }),
            }
        }
        sprites
    }
}

impl RootTown for LiaFail {
    fn draw(&mut self, base: &mut Base, layers: &mut Layers, to_screen: Mat4, v: &TownView) -> Vec<TownSprite> {
        let pieces = self.select(base, v);
        self.draw_pieces(base, layers, to_screen, &pieces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The constructor's pieces: 7 model rows, no static objects, the sky,
    /// the 20 lights, the 19 omni points, and the fog and background
    /// colour.
    #[test]
    fn constructor() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let t = new(&archive, false).unwrap();
        let b = &t.base;
        assert_eq!(b.no, NO);
        assert_eq!((0..7).filter(|&r| b.model(r).is_some()).count(), 7);
        let d = t.class::<LiaFail>().unwrap();
        assert!(!d.sky.is_empty());
        assert_eq!(b.lights.lights.len(), 20);
        assert_eq!(b.clear, Some(BG_COLOR));
        assert!(d.points.iter().all(|p| p[3] == ONE));
        assert!(
            d.glows.iter().all(|g| g.frame < d.pat_num && (30..130).contains(&g.wait) && (1..16).contains(&g.flicker))
        );
    }

    /// Each frame: the sky, the rows by pass, the map, then the 19 glows;
    /// a glow waiting steps its pattern and its wait.
    #[test]
    fn a_frame() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut t = new(&archive, false).unwrap();
        let (base, d) = t.parts_mut::<LiaFail>().unwrap();
        let before = d.glows;
        let p = d.select(base, &TownView::default());
        assert_eq!(p[0], Piece::Sky);
        assert!(p.contains(&Piece::Map));
        assert_eq!(p.iter().filter(|x| matches!(x, Piece::Glow(..))).count(), GLOWS);
        for (b, a) in before.iter().zip(d.glows) {
            assert_eq!(a.wait, b.wait - 1);
        }
    }
}
