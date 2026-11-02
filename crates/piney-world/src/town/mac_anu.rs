//! Mac Anu: `ROOTTOWN01` (gcmn town01.cpp, constructor 0x00421470, `Draw`
//! 0x00423b10 through the vtable at 0x00375f90), the first Root Town
//! (`game.town` 0): the town and water files, fog, 32 static models and 11
//! static objects (the ships), the sky, three copies of the water, and its
//! own the camera-eye clip rules of its `Draw` ([`Clip`];
//! docs/engine/field-game.md, "What the town draws"). `tools/test_world_rs.py`
//! runs the game's `Draw` in eemu over eyes all over the town beside
//! [`MAC_ANU`]'s steps.

use piney_data::Result;
use piney_data::classes::TownGameClass;
use piney_data::statics::DrawPass;

use crate::draw::OBJ_LAYER;
use crate::ee::{self, F, ONE, V4};
use crate::systems::scrolls::{ScrollsDef, Wrap};
use crate::systems::sky::{Axes, MatScroll, SkyClump, SkyDef};
use crate::systems::statics::{ModelsDef, ObjectsDef, StaticModels, StaticObjects};
use crate::systems::water::{WaterDef, WaterFile, WaterSpec};
use crate::systems::{BuildCx, DRAW_BG, Hook, Kind, MapSystem, Others, Piece, Step, StepCx, SystemDef};
use crate::town::scroll_statics as statics;
use crate::town::{BASE_TOWN, FogParams, TownDef};

/// `wateranmname[0]`, `watername[0]`, and the model water 0 draws.
pub const WATER_ANIM: &str = "ANM_sr1wat1_a";
pub const WATER_MODEL: &str = "MDL_wat00";
/// `waterUVModifi2`'s reach in Mac Anu: 9000 (0x00423f04).
const WATER_REACH: F = 0x460c_a000;
/// The sky clump, drawn at the origin.
pub const SKY: &str = "CMP_sr1bac1";
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
/// The water's scroll a frame, 0.005, less 1 past 1.
const WATER: WaterSpec = WaterSpec { step: 0x3ba3_d70a, wrap: Wrap::Less };
/// `DrawBG`'s two scrolls a frame (`v$1334`, gcmn gp 0x00378168): 0.01
/// (unused) and 0.002, each back to 0 past 1.
const BG_STEP: [F; 2] = [0x3c23_d70a, 0x3b03_126f];

/// In crisis (`town01d`) `DrawBG` draws three more clumps over the sky, on
/// `WORLD_MAN`'s layers at +0x49c, +0x4a0 and +0x4a4 (priorities -80, -70,
/// -60): `MAT_sr1dat1_2` scrolls by the second scroll in U, `_3` in U and V.
const CRISIS_MATS: [MatScroll; 2] =
    [MatScroll::new("MAT_sr1dat1_2", 1, Axes::U), MatScroll::new("MAT_sr1dat1_3", 1, Axes::UV)];

static MODELS: ModelsDef = ModelsDef { table: "RT_MODELTABLE00", layer: OBJ_LAYER };
static OBJECTS: ObjectsDef = ObjectsDef { table: "RT_OBJTABLE00" };
static SCROLLS: ScrollsDef = ScrollsDef { steps: &BG_STEP, slot: statics::TOWN01_BG };
static SKY_CLUMPS: SkyDef = SkyDef {
    clumps: &[
        SkyClump::on(SKY, OBJ_LAYER),
        SkyClump::crisis("CMP_sr1dat1_1", -80, &CRISIS_MATS),
        SkyClump::crisis("CMP_sr1dat1_2", -70, &CRISIS_MATS),
        SkyClump::crisis("CMP_sr1dat1_3", -60, &CRISIS_MATS),
    ],
};
/// `wat1`'s water at the origin: 2, 1 and 0 drawn on objLayer, no copy.
static WATER_DEF: WaterDef = WaterDef {
    file: WaterFile::Own("wat1"),
    anim: WATER_ANIM,
    model: WATER_MODEL,
    reach: WATER_REACH,
    root: [0, 0, 0, ONE],
    spec: WATER,
    slot: statics::TOWN01_WATER,
    drawn: &[2, 1, 0],
    layer: OBJ_LAYER,
    copy: false,
};

/// `ROOTTOWN01` (`town01`, `town01d` in crisis): `ROOTTOWN01::Draw` is the
/// clip, the ships, `DrawBG`, `DrawObj2` and `DrawObj` by the clip, the
/// water (`ccAnm::SetUV`, `waterUVModifi2`, 2, 1 and 0), `DrawFloor` by
/// the clip, `DrawMap`, then the tower (the type-0 rows).
pub static MAC_ANU: TownDef = TownDef {
    no: 0,
    class: TownGameClass::Roottown01,
    name: "Mac Anu",
    file: "town01",
    crisis_file: Some("town01d"),
    light_anim: LIGHT_ANIM,
    lights: &LIGHTS,
    fog: FOG,
    start: Some((crate::START_POS, crate::START_DIRC)),
    systems: &[
        SystemDef::Models(&MODELS),
        SystemDef::Objects(&OBJECTS),
        SystemDef::Scrolls(&SCROLLS),
        SystemDef::Sky(&SKY_CLUMPS),
        SystemDef::Water(&WATER_DEF),
        SystemDef::MacAnu(&()),
    ],
    order: &[
        Step::Run(Kind::MacAnu, Hook::Clip),
        Step::Run(Kind::Objects, Hook::Pass(DrawPass::Other)),
        DRAW_BG,
        Step::Run(Kind::MacAnu, Hook::Pass(DrawPass::Obj2)),
        Step::Run(Kind::MacAnu, Hook::Pass(DrawPass::Obj)),
        Step::Run(Kind::Water, Hook::Draw),
        Step::Run(Kind::MacAnu, Hook::Pass(DrawPass::Floor)),
        Step::Map,
        Step::Run(Kind::Models, Hook::Pass(DrawPass::Other)),
    ],
    ..BASE_TOWN
};

/// Mac Anu's own: `clip[3]` (+0x1b0..+0x1b8) as the last `Draw` set them,
/// and the rows and objects `DrawObj2`, `DrawObj` and `DrawFloor` draw by
/// them.
#[derive(Clone, Debug, Default)]
pub struct Clip {
    pub clip: [i32; 3],
}

impl Clip {
    pub fn new(_: &'static (), _: &mut BuildCx) -> Result<Clip> {
        Ok(Clip::default())
    }
}

impl MapSystem for Clip {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, others: &mut Others) {
        let eye = cx.view.eye;
        let (x, y) = (eye[0], eye[1]);
        let clip = self.clip;
        let Hook::Pass(pass) = hook else {
            if hook == Hook::Clip {
                self.clip = clip_of(eye);
            }
            return;
        };
        let Some(rows) = others.get_mut::<StaticModels>().map(|m| m.rows(pass)) else { return };
        let south_west = ee::lt(x, SW_X) && ee::lt(y, SW_Y);
        let mut model = |row: usize| cx.put(Kind::Models, Piece::Model(row));
        match pass {
            // DrawObj2 (0x00422900): the rows of type 3, then the static
            // objects of type 3 (none south-west).
            DrawPass::Obj2 => {
                for row in rows {
                    let skip = if clip[0] != 0 {
                        [18, 20, 21, 23, 25, 26].contains(&row)
                    } else if clip[1] != 0 {
                        [18, 20, 21, 27, 28].contains(&row)
                    } else {
                        (!ee::le(y, OBJ2_NORTH) && [12, 25, 26].contains(&row))
                            || (south_west && [21, 25, 28].contains(&row))
                    };
                    if !skip {
                        model(row);
                    }
                }
                if south_west {
                    return;
                }
                let volume = cx.core.hits.volume;
                let Some(objects) = others.get_mut::<StaticObjects>() else { return };
                for row in objects.step_pass(DrawPass::Obj2, eye, volume, &cx.core.file) {
                    cx.put(Kind::Objects, Piece::Object(row));
                }
            }
            // DrawObj (0x004224c0): the rows of type 2; 11 and 8 swap to
            // their far versions, 29 and 30, which are skipped in their own
            // turn.
            DrawPass::Obj => {
                let plaza = clip[0] != 0 || clip[1] != 0;
                let (lod11, lod8) = if plaza { (LOD11_PLAZA, LOD8_PLAZA) } else { (Y_NORTH, LOD8) };
                for row in rows {
                    let skip = if clip[0] != 0 {
                        [7, 9, 10, 12, 14, 15].contains(&row)
                    } else if clip[1] != 0 {
                        // clip[2] (north of 1600) cannot be set with clip[1]
                        // (the south plaza): the game's test of it here
                        // never passes.
                        [7, 9, 10, 16, 17].contains(&row) || (clip[2] != 0 && row == 12)
                    } else {
                        !ee::le(y, Y_NORTH) && row == 23
                    };
                    if skip {
                        continue;
                    }
                    match row {
                        11 if !ee::le(y, lod11) => model(29),
                        8 if !ee::le(y, lod8) => model(30),
                        29 | 30 => {}
                        r => model(r),
                    }
                }
            }
            // DrawFloor (0x00422ba0): the rows of type 1.
            DrawPass::Floor => {
                for row in rows {
                    let skip = if clip[0] != 0 {
                        [0, 2, 3, 5].contains(&row)
                    } else if clip[1] != 0 {
                        [0, 2, 3].contains(&row)
                    } else {
                        !ee::le(y, Y_NORTH) && row == 5
                    };
                    if !skip {
                        model(row);
                    }
                }
            }
            DrawPass::Other => {}
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::Drawn;
    use crate::town::{Town, TownView};

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
        let mut t = Town::open(&archive, piney_data::volume::Volume::Inf, 0, false).unwrap();
        let eye = [0, 5600f32.to_bits(), 880f32.to_bits(), ONE];
        t.step(&TownView::at(eye));
        let (m, o, w) = (Kind::Models, Kind::Objects, Kind::Water);
        let p = |k: Kind, piece: Piece| Drawn::Piece(k, piece);
        use Piece::{Anim, Clump, Model, Object};
        let want = [
            p(o, Object(9)),
            p(o, Object(10)),
            p(Kind::Sky, Clump(0)),
            p(m, Model(19)),
            p(m, Model(22)),
            p(m, Model(24)),
            p(m, Model(27)),
            p(m, Model(28)),
            p(o, Object(3)),
            p(o, Object(4)),
            p(m, Model(30)),
            p(m, Model(29)),
            p(m, Model(13)),
            p(m, Model(16)),
            p(m, Model(17)),
            p(w, Anim(2)),
            p(w, Anim(1)),
            p(w, Anim(0)),
            p(m, Model(1)),
            p(m, Model(4)),
            p(m, Model(6)),
            Drawn::Map,
            p(m, Model(31)),
        ];
        assert_eq!(t.frame(), want);
        assert_eq!(t.systems().get::<Clip>().unwrap().clip, [1, 0, 1]);
        let objects = t.systems().get::<StaticObjects>().unwrap();
        assert_eq!(objects.object(3).map(|o| o.0), Some(256));
    }
}
