//! Quarantine's sixth Root Town: `ROOTTOWN06` (`game.town` 5), which no
//! symbol names. Its constructor (QUA gcmn 0x004ec350) is the sixth case of
//! `WORLD_MAN::GO(0)`'s town switch; its `Draw` (0x004ecde0) and the rest
//! come through its vtable (0x002785e0). Three static models, five clumps
//! (`CMP_sr6met1`, `_bac1`, `_cro1`-`3`) and one distant light; no minimap
//! (its `DrawMap`, 0x004ecdd0, returns at once). docs/engine/root-towns.md.
//! The module is named for its loading card, `townNameTbl`'s sixth row:
//! Lia Fail's words again, "Relic City".

use piney_data::classes::TownGameClass;
use piney_data::statics::DrawPass;
use piney_data::volume::Volume;

use crate::draw::OBJ_LAYER;
use crate::ee::F;
use crate::systems::scrolls::ScrollsDef;
use crate::systems::sky::{SkyClump, SkyDef};
use crate::systems::statics::ModelsDef;
use crate::systems::{DRAW_BG, Hook, Kind, PASSES, Step, SystemDef};
use crate::town::scroll_statics as statics;
use crate::town::{BASE_TOWN, FogParams, TownDef};

/// The light animation and its one light (gp 0x00279038: type 0, a
/// distant light `WORLD_MAN::SetLightDirection` takes).
pub const LIGHT_ANIM: &str = "ANM_sr6town1a";
pub const LIGHTS: [(i32, &str); 1] = [(0, "LGT_sr6lig1")];
/// `DrawBG`'s clumps: the first on `bgLayer[0]` (-100), the three crosses
/// on `refLayer` (`SetActiveLayer(6)`, 30); the last (+0x1ac) on
/// `bgLayer[1]` (-90).
pub const BACK: &str = "CMP_sr6bac1";
pub const CROSSES: [&str; 3] = ["CMP_sr6cro1", "CMP_sr6cro2", "CMP_sr6cro3"];
pub const METEOR: &str = "CMP_sr6met1";
const REF_LAYER: i16 = 30;

/// `SetFog(1500, 10000, 0, 75, 0x00f0c080)` (0x004ec3cc) and the same
/// colour as `ccSys.bgColor`: Lia Fail's.
pub const FOG: FogParams =
    FogParams { near: 1500.0, far: 10000.0, near_rate: 0.0, far_rate: 75.0, colour: [0x80, 0xc0, 0xf0] };
pub const BG_COLOR: [u8; 3] = [0x80, 0xc0, 0xf0];

/// `DrawBG`'s scrolls a frame (QUA gp 0x00279980..0x00279988, read by
/// nothing): 0.01, 0.02 and 0.001, each back to 0 past 1.
pub const BG_STEP: [F; 3] = [0x3c23_d70a, 0x3ca3_d70a, 0x3a83_126f];

static MODELS: ModelsDef = ModelsDef { table: "RT_MODELTABLE06", layer: OBJ_LAYER };
static SCROLLS: ScrollsDef = ScrollsDef { steps: &BG_STEP, slot: statics::TOWN06_BG };
static SKY_CLUMPS: SkyDef = SkyDef {
    clumps: &[
        SkyClump::on(BACK, -100),
        SkyClump::on(CROSSES[0], REF_LAYER),
        SkyClump::on(CROSSES[1], REF_LAYER),
        SkyClump::on(CROSSES[2], REF_LAYER),
        SkyClump::on(METEOR, -90),
    ],
};

/// `ROOTTOWN06` (`town06`, no crisis variant, Quarantine's tables): its
/// `Draw` is `DrawBG`, then `SetActiveLayer(1)`, `DrawObj2` (type 3),
/// `DrawObj` (type 2), `DrawFloor` (type 1), `DrawMap` (nothing) and the
/// type-0 rows. `SetCharPosition` has no case for it.
pub static RELIC_CITY: TownDef = TownDef {
    no: 5,
    class: TownGameClass::Roottown06,
    name: "town06",
    volumes: &[Volume::Qua],
    file: "town06",
    tables: Volume::Qua,
    light_anim: LIGHT_ANIM,
    lights: &LIGHTS,
    fog: FOG,
    clear: Some(BG_COLOR),
    systems: &[SystemDef::Models(&MODELS), SystemDef::Scrolls(&SCROLLS), SystemDef::Sky(&SKY_CLUMPS)],
    order: &[DRAW_BG, PASSES, Step::Run(Kind::Models, Hook::Pass(DrawPass::Other))],
    ..BASE_TOWN
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::scrolls::Scrolls;
    use crate::systems::sky::Sky;
    use crate::systems::statics::StaticModels;
    use crate::systems::{Drawn, Piece};
    use crate::town::{Town, TownView};

    fn town() -> Option<Town> {
        let archive = crate::town::tests::archive_of("quarantine")?;
        Some(Town::open(&archive, Volume::Qua, 5, false).unwrap())
    }

    /// The constructor's pieces: 3 model rows (floor, obj, obj2), the five
    /// clumps, the one light, and the fog and background colour.
    #[test]
    fn constructor() {
        let Some(t) = town() else { return };
        assert_eq!(t.def.no, 5);
        let models = t.systems().get::<StaticModels>().unwrap();
        assert_eq!((0..3).filter(|&r| models.model(r).is_some()).count(), 3);
        assert!((0..5).all(|k| t.systems().get::<Sky>().unwrap().made(k)));
        assert_eq!(t.core.lights.lights.len(), 1);
        assert_eq!(t.def.clear, Some(BG_COLOR));
    }

    /// Each frame: the back, the three crosses and the meteor, then the
    /// rows by pass (type 3, 2, 1; no type 0).
    #[test]
    fn a_frame() {
        let Some(mut t) = town() else { return };
        t.step(&TownView::default());
        let clump = |k: usize| Drawn::Piece(Kind::Sky, Piece::Clump(k));
        let model = |row: usize| Drawn::Piece(Kind::Models, Piece::Model(row));
        assert_eq!(t.frame(), [clump(0), clump(1), clump(2), clump(3), clump(4), model(2), model(1), model(0)]);
        assert_eq!(t.systems().get::<Scrolls>().unwrap().v, BG_STEP);
    }
}
