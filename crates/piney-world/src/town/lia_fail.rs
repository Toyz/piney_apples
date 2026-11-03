//! Lia Fail: `ROOTTOWN05` (OUT gcmn town05.cpp, constructor 0x0043c5e0, `Draw`
//! 0x0043e8e0 through the vtable at 0x00384300), the fifth Root Town
//! (`game.town` 4, the server Omega), new in Outbreak (Quarantine's is the
//! same). A sky, 7 static models, and its own 19 glows (`EFF_se1_1ef1`) at
//! the omni points that wait, then flicker through patterns from `fieldrand`
//! (docs/engine/root-towns.md).

use piney_data::classes::TownGameClass;
use piney_data::statics::DrawPass;
use piney_data::{Error, Result};

use crate::draw::{EFF_LAYER, OBJ_LAYER};
use crate::ee::{F, ONE, V4};
use crate::map::town::{ArrowPlace, Balloons, MarkY, MinimapDef};
use crate::sky::cloud;
use crate::sprite::{EffSprite, EffState};
use crate::systems::scrolls::ScrollsDef;
use crate::systems::sky::{SkyClump, SkyDef};
use crate::systems::statics::ModelsDef;
use crate::systems::{
    BuildCx, DRAW_BG, DrawCx, Hook, Kind, MapSystem, Others, PASSES, Piece, Request, Step, StepCx, SystemDef,
};
use crate::town::scroll_statics as statics;
use crate::town::{BASE_TOWN, FogParams, TownDef};

pub const FILE: &str = "town05";
/// `DrawBG`'s clump, on `bgLayer[0]` (+0x494).
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

/// `DrawBG`'s scrolls a frame (OUT gp 0x00386ee0..0x00386ee8, read by
/// nothing): 0.01, 0.02 and 0.001.
pub const BG_STEP: [F; 3] = [0x3c23_d70a, 0x3ca3_d70a, 0x3a83_126f];

static MODELS: ModelsDef = ModelsDef::on("RT_MODELTABLE05", OBJ_LAYER);
static SCROLLS: ScrollsDef = ScrollsDef::statics(&BG_STEP, statics::TOWN05_BG, &[]);
static SKY_CLUMPS: SkyDef = SkyDef { clumps: &[SkyClump::on(SKY, -100)] };

/// `ROOTTOWN05::DrawMap`: its own layers (2.0 a step, OUT gcmn 0x0043c7ec),
/// racers' marks `ccMask(6, 6)` (0x0043caec) 24 across and level, the
/// arrow at (87, 92), a sign only within the map's 160 rows and dimmed
/// within (8, 46) and (10, 45).
static MINIMAP: MinimapDef = MinimapDef {
    iconpos: "RT05ICONPOS",
    tex: "TEX_sr5map1",
    step: (0x4000_0000, 0x4000_0000),
    race: true,
    later_only: true,
    offset: (0, 0),
    racer_offset: (0, 0),
    mark_mask: (6, 6),
    mark_x: 0x41c0_0000,
    mark_y: MarkY::Level,
    arrow_base: (0x42ae_0000, 0x42b8_0000),
    arrow: ArrowPlace::Base,
    odd_sign: None,
    sign_window: true,
    balloons: Balloons::OutsideRace,
    dim: Some([8, 46, 10, 45]),
    hold_fade: false,
};

/// `ROOTTOWN05` (`town05`, whatever the crisis byte says): its `Draw` is
/// `DrawBG`, `DrawObj2`, `DrawObj`, `DrawFloor`, `DrawMap`, the type-0
/// rows, then the glows.
pub static LIA_FAIL: TownDef = TownDef {
    no: 4,
    class: TownGameClass::Roottown05,
    name: "Lia Fail",
    file: FILE,
    light_anim: LIGHT_ANIM,
    lights: &LIGHTS,
    fog: FOG,
    clear: Some(BG_COLOR),
    start: Some(([0x4416_0000, 0xc573_c000, 0, ONE], [0, 0, crate::ee::PI, 0])),
    minimap: Some(&MINIMAP),
    systems: &[
        SystemDef::Models(&MODELS),
        SystemDef::Scrolls(&SCROLLS),
        SystemDef::Sky(&SKY_CLUMPS),
        SystemDef::Glows(&()),
    ],
    order: &[
        DRAW_BG,
        PASSES,
        Step::Map,
        Step::Run(Kind::Models, Hook::Pass(DrawPass::Other)),
        Step::Run(Kind::Glows, Hook::Draw),
    ],
    ..BASE_TOWN
};

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

/// Lia Fail's own: the omni points (w 1), their glows and the glow's
/// `patNum`.
#[derive(Clone, Debug)]
pub struct Glows {
    pub points: [V4; GLOWS],
    pub glows: [Glow; GLOWS],
    pat_num: u16,
}

impl Glows {
    /// The constructor's: each glow's pattern, wait and flicker from
    /// `fieldrand`, each point at `DMY_omnpointNN`.
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<Glows> {
        let file = &**cx.file;
        let pat_num = cloud::eff_pat_num(&file.ccs, GLOW).ok_or_else(|| Error::NotFound(format!("{FILE}: {GLOW}")))?;
        let pat_num = pat_num.max(1);
        let rng = &mut cx.rng;
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
        Ok(Glows { points, glows, pat_num })
    }
}

impl MapSystem for Glows {
    /// Each glow: waiting, its pattern steps; flickering, a pattern from
    /// `fieldrand`, and when the flicker ends a new wait and flicker.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let Some(rng) = &mut cx.core.field_rand else { return };
        let mut shown = Vec::with_capacity(GLOWS);
        for (k, g) in self.glows.iter_mut().enumerate() {
            if g.wait == 0 {
                shown.push((k, rng.below(u32::from(self.pat_num)) as u16));
                g.flicker -= 1;
                if g.flicker == 0 {
                    g.wait = rng.below(100) as i32 + 30;
                    g.flicker = rng.below(15) as i32 + 1;
                }
            } else {
                g.wait -= 1;
                shown.push((k, g.frame));
                g.frame += 1;
                if g.frame == self.pat_num {
                    g.frame = 0;
                }
            }
        }
        for (k, pattern) in shown {
            cx.put(Kind::Glows, Piece::Sprite { k, pattern, pos: self.points[k] });
        }
    }

    /// The glow's `ccEff`, kept in the town's depth test and fog, so the
    /// buildings hide it.
    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Sprite { pattern, pos, .. } = piece else { return None };
        Some(Request::Sprite(EffSprite::new(FILE, GLOW, pos, pattern, EFF_LAYER, EffState::INIT)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::Drawn;
    use crate::systems::MapView;
    use crate::systems::statics::StaticModels;
    use crate::town::Town;
    use piney_data::volume::Volume;

    fn town() -> Option<Town> {
        let archive = crate::town::tests::archive()?;
        Some(Town::open(&archive, Volume::Out, 4, false).unwrap())
    }

    /// The constructor's pieces: 7 model rows, no static objects, the sky,
    /// the 20 lights, the 19 omni points, and the fog and background
    /// colour.
    #[test]
    fn constructor() {
        let Some(t) = town() else { return };
        assert_eq!(t.def.no, 4);
        let models = t.systems().get::<StaticModels>().unwrap();
        assert_eq!((0..7).filter(|&r| models.model(r).is_some()).count(), 7);
        assert_eq!(t.core.lights.lights.len(), 20);
        assert_eq!(t.def.clear, Some(BG_COLOR));
        let g = t.systems().get::<Glows>().unwrap();
        assert!(g.points.iter().all(|p| p[3] == ONE));
        assert!(
            g.glows.iter().all(|x| x.frame < g.pat_num && (30..130).contains(&x.wait) && (1..16).contains(&x.flicker))
        );
    }

    /// Each frame: the sky, the rows by pass, the map, then the 19 glows;
    /// a glow waiting steps its pattern and its wait.
    #[test]
    fn a_frame() {
        let Some(mut t) = town() else { return };
        let before = t.systems().get::<Glows>().unwrap().glows;
        t.step(&MapView::default());
        assert_eq!(t.frame()[0], Drawn::Piece(Kind::Sky, Piece::Clump(0)));
        assert!(t.frame().contains(&Drawn::Map));
        let glows = t.frame().iter().filter(|d| matches!(d, Drawn::Piece(Kind::Glows, _))).count();
        assert_eq!(glows, GLOWS);
        for (b, a) in before.iter().zip(t.systems().get::<Glows>().unwrap().glows) {
            assert_eq!(a.wait, b.wait - 1);
        }
    }
}
