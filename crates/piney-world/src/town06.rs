//! Quarantine's sixth Root Town: `ROOTTOWN06` (`game.town` 5), which no
//! symbol names. Its constructor (QUA gcmn 0x004ec350) is the sixth case of
//! `WORLD_MAN::GO(0)`'s town switch; its `Draw` (0x004ecde0) and the rest
//! come through its vtable (0x002785e0). Three static models, five clumps
//! (`CMP_sr6met1`, `_bac1`, `_cro1`-`3`) and one distant light; no minimap
//! (its `DrawMap`, 0x004ecdd0, returns at once). docs/engine/root-towns.md.

use std::collections::HashMap;
use std::sync::Arc;

use glam::Mat4;
use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::statics::DrawPass;
use piney_desktop::layers::Layers;

use crate::draw::OBJ_LAYER;
use crate::ee::{self, F, ONE};
use crate::town::{Base, FogParams, RootTown, Spec, Town, TownSprite, TownView, clump_models};

/// `ChangeScene`'s town number.
pub const NO: i32 = 5;
pub const FILE: &str = "town06";
/// The light animation and its one light (gp 0x00279038: type 0, a
/// distant light `WORLD_MAN::SetLightDirection` takes).
pub const LIGHT_ANIM: &str = "ANM_sr6town1a";
pub const LIGHTS: [(i32, &str); 1] = [(0, "LGT_sr6lig1")];
/// `DrawBG`'s clumps: the first on `bgLayer[0]`, the three crosses on
/// `refLayer`; the last (+0x1ac) on `bgLayer[1]`.
pub const BACK: &str = "CMP_sr6bac1";
pub const CROSSES: [&str; 3] = ["CMP_sr6cro1", "CMP_sr6cro2", "CMP_sr6cro3"];
pub const METEOR: &str = "CMP_sr6met1";

/// `SetFog(1500, 10000, 0, 75, 0x00f0c080)` (0x004ec3cc) and the same
/// colour as `ccSys.bgColor`: Lia Fail's.
pub const FOG: FogParams =
    FogParams { near: 1500.0, far: 10000.0, near_rate: 0.0, far_rate: 75.0, colour: [0x80, 0xc0, 0xf0] };
pub const BG_COLOR: [u8; 3] = [0x80, 0xc0, 0xf0];

const SPEC: Spec = Spec {
    no: NO,
    models: "RT_MODELTABLE06",
    objects: None,
    light_anim: LIGHT_ANIM,
    lights: &LIGHTS,
    fog: FOG,
    clear: Some(BG_COLOR),
};

/// `DrawBG`'s scrolls a frame (QUA gp 0x00279980..0x00279988, read by
/// nothing): 0.01, 0.02 and 0.001, each back to 0 past 1.
const BG_STEP: [F; 3] = [0x3c23_d70a, 0x3ca3_d70a, 0x3a83_126f];

/// `WORLD_MAN`'s layers the town draws on besides `objLayer`.
pub mod layer {
    /// `bgLayer[0]` (+0x494) and `bgLayer[1]` (+0x498).
    pub const BG0: i16 = -100;
    pub const BG1: i16 = -90;
    /// `SetActiveLayer(6)`: refLayer.
    pub const REF: i16 = 30;
}

/// One piece `ROOTTOWN06::Draw` draws, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// `DrawBG`'s clumps at the origin: `CMP_sr6bac1`, then the crosses.
    Back,
    Cross(usize),
    /// `CMP_sr6met1`, last of `DrawBG`.
    Meteor,
    /// A `STATICMODEL` (its `RT_MODELTABLE06` row).
    Model(usize),
}

/// What `ROOTTOWN06` holds besides the [`Base`].
pub struct Town06 {
    back: Vec<u32>,
    crosses: [Vec<u32>; 3],
    meteor: Vec<u32>,
    /// `DrawBG`'s scrolls (read by nothing).
    pub bg_scroll: [F; 3],
}

/// `ROOTTOWN06::ROOTTOWN06`: `town06` (no crisis variant).
pub fn new(archive: &Arc<Archive>, _crisis: bool) -> Result<Town> {
    let base = Base::read_volume(archive, piney_data::volume::Volume::Qua, FILE, &SPEC)?;
    let file = base.file.clone();
    let d = Town06 {
        back: clump_models(&file, BACK),
        crosses: CROSSES.map(|n| clump_models(&file, n)),
        meteor: clump_models(&file, METEOR),
        bg_scroll: [0; 3],
    };
    Ok(Town::new(base, d))
}

impl Town06 {
    /// `ROOTTOWN06::Draw` for this frame: the pieces in its order, with the
    /// scrolls stepped as `DrawBG` steps them.
    pub fn select(&mut self, base: &Base) -> Vec<Piece> {
        // DrawBG (0x004eca00).
        for (x, step) in self.bg_scroll.iter_mut().zip(BG_STEP) {
            *x = ee::add(*x, step);
        }
        for x in &mut self.bg_scroll {
            if !ee::le(*x, ONE) {
                *x = 0;
            }
        }
        let mut out = vec![Piece::Back];
        out.extend((0..3).map(Piece::Cross));
        out.push(Piece::Meteor);
        // SetActiveLayer(1); DrawObj2 (type 3), DrawObj (type 2), DrawFloor
        // (type 1), DrawMap (nothing), then the type-0 rows.
        for pass in [DrawPass::Obj2, DrawPass::Obj, DrawPass::Floor, DrawPass::Other] {
            out.extend(base.rows(pass).map(Piece::Model));
        }
        out
    }

    fn draw_pieces(&self, base: &Base, layers: &mut Layers, to_screen: Mat4, pieces: &[Piece]) {
        let none = HashMap::new();
        for &piece in pieces {
            match piece {
                Piece::Back => base.clump_draw(layers, layer::BG0, to_screen, &self.back, Mat4::IDENTITY, &none),
                Piece::Cross(k) => {
                    base.clump_draw(layers, layer::REF, to_screen, &self.crosses[k], Mat4::IDENTITY, &none)
                }
                Piece::Meteor => base.clump_draw(layers, layer::BG1, to_screen, &self.meteor, Mat4::IDENTITY, &none),
                Piece::Model(row) => base.row_draw(layers, OBJ_LAYER, to_screen, row),
            }
        }
    }
}

impl RootTown for Town06 {
    fn draw(&mut self, base: &mut Base, layers: &mut Layers, to_screen: Mat4, _v: &TownView) -> Vec<TownSprite> {
        let pieces = self.select(base);
        self.draw_pieces(base, layers, to_screen, &pieces);
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive() -> Option<Arc<Archive>> {
        crate::town::tests::archive_of("quarantine")
    }

    /// The constructor's pieces: 3 model rows (floor, obj, obj2), the five
    /// clumps, the one light, and the fog and background colour.
    #[test]
    fn constructor() {
        let Some(archive) = archive() else { return };
        let t = new(&archive, false).unwrap();
        let b = &t.base;
        assert_eq!(b.no, NO);
        assert_eq!((0..3).filter(|&r| b.model(r).is_some()).count(), 3);
        let d = t.class::<Town06>().unwrap();
        assert!(!d.back.is_empty() && !d.meteor.is_empty());
        assert!(d.crosses.iter().all(|c| !c.is_empty()));
        assert_eq!(b.lights.lights.len(), 1);
        assert_eq!(b.clear, Some(BG_COLOR));
    }

    /// Each frame: the back, the three crosses and the meteor, then the
    /// rows by pass (type 3, 2, 1; no type 0).
    #[test]
    fn a_frame() {
        let Some(archive) = archive() else { return };
        let mut t = new(&archive, false).unwrap();
        let (base, d) = t.parts_mut::<Town06>().unwrap();
        let p = d.select(base);
        assert_eq!(
            p,
            [
                Piece::Back,
                Piece::Cross(0),
                Piece::Cross(1),
                Piece::Cross(2),
                Piece::Meteor,
                Piece::Model(2),
                Piece::Model(1),
                Piece::Model(0)
            ]
        );
        assert_eq!(d.bg_scroll, BG_STEP);
    }
}
