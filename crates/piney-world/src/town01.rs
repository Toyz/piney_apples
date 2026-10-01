//! Mac Anu: `ROOTTOWN01` (gcmn town01.cpp, constructor 0x00421470, `Draw`
//! 0x00423b10 through the vtable at 0x00375f90), the first Root Town
//! (`game.town` 0), and what it adds to the [`Base`] every town builds: the
//! town and water files, fog, 32 static models and 11 static objects, the sky,
//! three copies of the water, and the camera-eye clip rules of its `Draw`
//! (docs/engine/field-game.md, "What the town draws"). `tools/test_world_rs.py`
//! runs the game's `Draw` in eemu over eyes all over the town beside
//! [`MacAnu::select`].

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::Mat4;
use piney_data::archive::Archive;
use piney_data::statics::DrawPass;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::draw::OBJ_LAYER;
use crate::ee::{self, F, ONE, V4};
use crate::pose::Play;
use crate::town::{
    Base, FogParams, RootTown, Spec, Town, TownSprite, TownView, WaterZero, anim_draw, clump_models, crisis_rows,
    pos_rot_zyx, water_rows, water0_draw,
};

/// `wateranmname[0]`, `watername[0]`, and the model water 0 draws.
pub const WATER_ANIM: &str = "ANM_sr1wat1_a";
pub const WATER_OBJ: &str = "OBJ_wat00";
pub const WATER_MODEL: &str = "MDL_wat00";
/// `waterUVModifi2`'s reach in Mac Anu: 9000 (0x00423f04).
const WATER_REACH: F = 0x460c_a000;
/// The sky clump, drawn at the origin.
pub const SKY: &str = "CMP_sr1bac1";
/// In crisis (`town01d`) `DrawBG` draws three more clumps over the sky, on
/// `WORLD_MAN`'s layers at +0x49c, +0x4a0 and +0x4a4 (priorities -80, -70,
/// -60); the second scrolls in U, the third in U and V.
pub const CRISIS_SKY: [(&str, i16); 3] = [("CMP_sr1dat1_1", -80), ("CMP_sr1dat1_2", -70), ("CMP_sr1dat1_3", -60)];
pub const CRISIS_MATS: [&str; 2] = ["MAT_sr1dat1_2", "MAT_sr1dat1_3"];
/// The light animation.
pub const LIGHT_ANIM: &str = "ANM_sr1town1a";
/// `town00Light` (gcmn 0x005d4a60): the distant light, then the omnis.
pub const LIGHTS: [(i32, &str); 6] = [
    (0, "LGT_sr1lig1"),
    (1, "LGT_sr1omn01"),
    (1, "LGT_sr1omn02"),
    (1, "LGT_sr1omn03"),
    (1, "LGT_sr1omn04"),
    (1, "LGT_sr1omn05"),
];

/// `cc3d->SetFog(1000, 7500, 0, 85, 0x00144870)` (0x00421630): no fog at
/// 1000, 85 percent at 7500, toward an orange brown. The constructor
/// leaves `ccSys.bgColor` alone.
pub const FOG: FogParams =
    FogParams { near: 1000.0, far: 7500.0, near_rate: 0.0, far_rate: 85.0, colour: [0x70, 0x48, 0x14] };

const SPEC: Spec = Spec {
    no: 0,
    models: "RT_MODELTABLE00",
    objects: Some("RT_OBJTABLE00"),
    light_anim: LIGHT_ANIM,
    lights: &LIGHTS,
    fog: FOG,
    clear: None,
};

/// `ROOTTOWN01::Draw`'s clip rules (0x00423b44-0x00423c94): the camera eye
/// above y 1600 sets `clip[2]`; on the gate plaza `clip[0]`; on the south
/// plaza `clip[1]` (to 2).
const Y_NORTH: F = 0x44c8_0000; // 1600
const GATE_X: F = 0x4416_0000; // 600
const GATE_Y: (F, F) = (0x4599_2000, 0x45d7_a000); // 4900, 6900
const SOUTH_X: F = 0x4496_0000; // 1200
const SOUTH_Y: (F, F) = (0xc5c4_e000, 0xc58f_c000); // -6300, -4600
/// `DrawObj2`: rows 12, 25 and 26 are skipped north of y 2600; rows 21, 25
/// and 28 and the static objects south-west of (-3000, -3200).
const OBJ2_NORTH: F = 0x4522_8000; // 2600
const SW_X: F = 0xc53b_8000; // -3000
const SW_Y: F = 0xc548_0000; // -3200
/// `DrawObj`: row 11 swaps to row 29 and row 8 to row 30 beyond these y
/// (on a plaza, and elsewhere), and row 23 goes north of 1600.
const LOD11_PLAZA: F = 0x44fa_0000; // 2000
const LOD8_PLAZA: F = 0x4509_8000; // 2200
const LOD8: F = 0x4516_0000; // 2400
/// The water's scroll a frame, 0.005, and `vftoi12`'s 4096.
const WATER_STEP: F = 0x3ba3_d70a;
/// `DrawBG`'s two scrolls a frame (`v$1334`, gcmn gp 0x00378168): 0.01
/// (unused) and 0.002, each back to 0 past 1.
const BG_STEP: [F; 2] = [0x3c23_d70a, 0x3b03_126f];
const K4096: F = 0x4580_0000;

/// One piece `ROOTTOWN01::Draw` draws, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// A `STATICOBJECT` (its `RT_OBJTABLE` row) within its clip: its
    /// animation stepped, then drawn.
    Object(usize),
    /// `DrawBG`: the sky clump at the origin.
    Sky,
    /// `DrawBG` in crisis: the crisis clump `k` at the origin on its own
    /// layer.
    CrisisSky(usize),
    /// A `STATICMODEL` (its `RT_MODELTABLE` row).
    Model(usize),
    /// `waterAnm[k]`, stepped and drawn; 1 and 2 with the offset `ccAnm::
    /// SetUV` gave them this frame (1 in U, 2 in V).
    Water(usize),
    /// `DrawMap`: the minimap (the host draws it).
    Map,
}

/// What `ROOTTOWN01` holds besides the [`Base`].
pub struct MacAnu {
    pub water_file: Rc<SceneFile>,
    water_morphers: HashMap<u32, u32>,
    /// The sky clump's models (drawn at the origin, unanimated).
    sky: Vec<u32>,
    /// In crisis, `CRISIS_SKY`'s models; `DrawBG`'s scrolls and what it
    /// wrote into the materials' offsets (`vftoi12` of the second).
    crisis_sky: Option<[Vec<u32>; 3]>,
    bg_scroll: [F; 2],
    pub bg_uv: u16,
    /// `waterAnm[0..2]`.
    water: [Play; 3],
    /// The water's scroll, `u$1841` (gcmn gp 0x00378b54): +0.005 a frame.
    water_u: F,
    /// What the last draw passed `ccAnm::SetUV` (`vftoi12` of the scroll).
    pub water_uv: u16,
    /// Water 0's texture coordinates and texture.
    pub water0: WaterZero,
    /// `clip[3]` (+0x1b0..+0x1b8) as the last draw set them.
    pub clip: [i32; 3],
}

/// `ROOTTOWN01::ROOTTOWN01`: `town01d` when the save's crisis byte is set,
/// else `town01`, and `wat1`.
pub fn new(archive: &Arc<Archive>, crisis: bool) -> Result<Town> {
    let stem = if crisis { "town01d" } else { "town01" };
    let base = Base::read(archive, stem, &SPEC)?;
    let water_file = Rc::new(SceneFile::read(archive, "wat1")?);
    let sky = clump_models(&base.file, SKY);
    let crisis_sky = crisis.then(|| CRISIS_SKY.map(|(n, _)| clump_models(&base.file, n)));
    let wplay = Play::new(&water_file, WATER_ANIM).ok_or(Error::NotFound(WATER_ANIM.into()))?;
    let mut water = [wplay.clone(), wplay.clone(), wplay];
    for w in &mut water {
        // One _AnimateForward after SetAnm.
        w.forward(&water_file);
    }
    let water_morphers = piney_data::anim::morphers(&water_file.ccs).unwrap_or_default();
    let water0 = WaterZero::new(&water_file, WATER_MODEL, WATER_REACH)?;
    let m = MacAnu {
        water_file,
        water_morphers,
        sky,
        crisis_sky,
        bg_scroll: [0; 2],
        bg_uv: 0,
        water,
        water_u: 0,
        water_uv: 0,
        water0,
        clip: [0; 3],
    };
    Ok(Town::new(base, m))
}

/// `ROOTTOWN01::Draw`'s `clip[3]` for a camera eye.
pub fn clip_of(eye: V4) -> [i32; 3] {
    let (x, y) = (eye[0], eye[1]);
    let within = |v: F, lo: F, hi: F| ee::lt(lo, v) && ee::lt(v, hi);
    let mut clip = [0; 3];
    if !ee::le(y, Y_NORTH) {
        clip[2] = 1;
    }
    if within(x, ee::neg(GATE_X), GATE_X) && within(y, GATE_Y.0, GATE_Y.1) {
        clip[0] = 1;
    }
    if within(x, ee::neg(SOUTH_X), SOUTH_X) && within(y, SOUTH_Y.0, SOUTH_Y.1) {
        clip[1] = 2;
    }
    clip
}

impl MacAnu {
    /// `ROOTTOWN01::Draw` for a camera at `v.eye` (`cameraGetPos`): the
    /// pieces it draws in order, the objects' animations and the water
    /// stepped as it steps them.
    pub fn select(&mut self, base: &mut Base, v: &TownView) -> Vec<Piece> {
        let eye = v.eye;
        let (x, y) = (eye[0], eye[1]);
        let clip = clip_of(eye);
        let rows = base.table.rows;
        let mut out = Vec::new();
        // The static objects of type 0: the ships.
        out.extend(base.step_objects(DrawPass::Other, eye).into_iter().map(Piece::Object));
        self.clip = clip;
        // DrawBG (0x00422250): the scrolls step; the sky at the origin, and
        // in crisis the three clumps over it.
        for (v, step) in self.bg_scroll.iter_mut().zip(BG_STEP) {
            *v = ee::add(*v, step);
            if !ee::le(*v, ONE) {
                *v = 0;
            }
        }
        out.push(Piece::Sky);
        if self.crisis_sky.is_some() {
            self.bg_uv = (ee::to_int(ee::mul(self.bg_scroll[1], K4096)) & 0xffff) as u16;
            out.extend((0..3).map(Piece::CrisisSky));
        }
        // DrawObj2 (0x00422900): the rows of type 3, then the static
        // objects of type 3 (none south-west).
        let south_west = ee::lt(x, SW_X) && ee::lt(y, SW_Y);
        for row in base.rows(DrawPass::Obj2) {
            let skip = if clip[0] != 0 {
                [18, 20, 21, 23, 25, 26].contains(&row)
            } else if clip[1] != 0 {
                [18, 20, 21, 27, 28].contains(&row)
            } else {
                (!ee::le(y, OBJ2_NORTH) && [12, 25, 26].contains(&row)) || (south_west && [21, 25, 28].contains(&row))
            };
            if !skip {
                out.push(Piece::Model(row));
            }
        }
        if !south_west {
            out.extend(base.step_objects(DrawPass::Obj2, eye).into_iter().map(Piece::Object));
        }
        // DrawObj (0x004224c0): the rows of type 2; 11 and 8 swap to their
        // far versions, 29 and 30, which are skipped in their own turn.
        let plaza = clip[0] != 0 || clip[1] != 0;
        let (lod11, lod8) = if plaza { (LOD11_PLAZA, LOD8_PLAZA) } else { (Y_NORTH, LOD8) };
        for (row, r) in rows.iter().enumerate() {
            if r.pass != DrawPass::Obj {
                continue;
            }
            let skip = if clip[0] != 0 {
                [7, 9, 10, 12, 14, 15].contains(&row)
            } else if clip[1] != 0 {
                // clip[2] (north of 1600) cannot be set with clip[1] (the
                // south plaza): the game's test of it here never passes.
                [7, 9, 10, 16, 17].contains(&row) || (clip[2] != 0 && row == 12)
            } else {
                !ee::le(y, Y_NORTH) && row == 23
            };
            if skip {
                continue;
            }
            let draw = match row {
                11 if !ee::le(y, lod11) => 29,
                8 if !ee::le(y, lod8) => 30,
                29 | 30 => continue,
                r => r,
            };
            out.push(Piece::Model(draw));
        }
        // The water: ccAnm::SetUV of vftoi12(u) into 1's U and 2's V,
        // waterUVModifi2 of water 0 (rooted at the identity, unposed), the
        // scroll steps, then 2, 1 and 0 step and draw.
        self.water0.modify(
            base.hits.volume,
            &pos_rot_zyx([0, 0, 0, ONE], [0; 3]),
            [0, 0, 0, ONE],
            eye,
            &v.world_screen,
        );
        self.water_uv = (ee::to_int(ee::mul(self.water_u, K4096)) & 0xffff) as u16;
        self.water_u = ee::add(self.water_u, WATER_STEP);
        if !ee::le(self.water_u, ONE) {
            self.water_u = ee::sub(self.water_u, ONE);
        }
        for k in [2usize, 1, 0] {
            self.water[k].forward(&self.water_file);
            out.push(Piece::Water(k));
        }
        // DrawFloor (0x00422ba0): the rows of type 1.
        for row in base.rows(DrawPass::Floor) {
            let skip = if clip[0] != 0 {
                [0, 2, 3, 5].contains(&row)
            } else if clip[1] != 0 {
                [0, 2, 3].contains(&row)
            } else {
                !ee::le(y, Y_NORTH) && row == 5
            };
            if !skip {
                out.push(Piece::Model(row));
            }
        }
        out.push(Piece::Map);
        // The rows of type 0: the tower.
        out.extend(base.rows(DrawPass::Other).map(Piece::Model));
        out
    }

    /// The pieces into the field's layers, all on objLayer.
    fn draw_pieces(&self, base: &Base, layers: &mut Layers, to_screen: Mat4, pieces: &[Piece]) {
        let wf = (&*self.water_file, &self.water_morphers);
        for &piece in pieces {
            match piece {
                Piece::Object(row) => base.object_draw(layers, OBJ_LAYER, to_screen, row),
                Piece::Sky => base.clump_draw(layers, OBJ_LAYER, to_screen, &self.sky, Mat4::IDENTITY, &HashMap::new()),
                Piece::CrisisSky(k) => {
                    let Some(models) = &self.crisis_sky else { continue };
                    // The materials' offsets: the scroll less each one's
                    // crop, MAT_sr1dat1_2 in U, MAT_sr1dat1_3 in U and V.
                    let rows = crisis_rows(&base.file, &CRISIS_MATS, self.bg_uv);
                    base.clump_draw(layers, CRISIS_SKY[k].1, to_screen, &models[k], Mat4::IDENTITY, &rows);
                }
                Piece::Model(row) => base.row_draw(layers, OBJ_LAYER, to_screen, row),
                // Water 0: the picture so far as its texture, at the UVs
                // waterUVModifi2 wrote.
                Piece::Water(0) => {
                    water0_draw(layers, OBJ_LAYER, to_screen, wf, &self.water[0], Mat4::IDENTITY, &self.water0);
                }
                Piece::Water(k) => {
                    let rows = water_rows(&self.water_file, k, self.water_uv);
                    anim_draw(layers, to_screen, wf, &self.water[k], Mat4::IDENTITY, &[], 1.0, Some(&rows));
                }
                Piece::Map => {}
            }
        }
    }
}

impl RootTown for MacAnu {
    fn draw(&mut self, base: &mut Base, layers: &mut Layers, to_screen: Mat4, v: &TownView) -> Vec<TownSprite> {
        let pieces = self.select(base, v);
        self.draw_pieces(base, layers, to_screen, &pieces);
        Vec::new()
    }

    fn water_time(&self, k: usize) -> u32 {
        self.water[k].time
    }

    fn water0(&self) -> Option<&WaterZero> {
        Some(&self.water0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_rules() {
        let eye = |x: f32, y: f32| [x.to_bits(), y.to_bits(), 800f32.to_bits(), ONE];
        assert_eq!(clip_of(eye(0.0, 5600.0)), [1, 0, 1]);
        assert_eq!(clip_of(eye(0.0, -5000.0)), [0, 2, 0]);
        assert_eq!(clip_of(eye(0.0, 1600.0)), [0, 0, 0]);
        assert_eq!(clip_of(eye(600.0, 5600.0)), [0, 0, 1]);
        assert_eq!(clip_of(eye(-1199.0, -6299.0)), [0, 2, 0]);
    }

    /// What the game's `ROOTTOWN01::Draw` drew for the arrival's camera eye
    /// (0, 5600, 880), run in eemu (`tools/test_world_rs.py`).
    #[test]
    fn gate_plaza_pieces() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let mut t = new(&archive, false).unwrap();
        let eye = [0, 5600f32.to_bits(), 880f32.to_bits(), ONE];
        use Piece::*;
        let want = [
            Object(9),
            Object(10),
            Sky,
            Model(19),
            Model(22),
            Model(24),
            Model(27),
            Model(28),
            Object(3),
            Object(4),
            Model(30),
            Model(29),
            Model(13),
            Model(16),
            Model(17),
            Water(2),
            Water(1),
            Water(0),
            Model(1),
            Model(4),
            Model(6),
            Map,
            Model(31),
        ];
        let (base, m) = t.parts_mut::<MacAnu>().unwrap();
        assert_eq!(m.select(base, &TownView::at(eye)), want);
        assert_eq!(m.clip, [1, 0, 1]);
        assert_eq!(t.base.object(3).map(|o| o.0), Some(256));
    }
}
