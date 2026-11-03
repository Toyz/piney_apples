//! Area 13's story map: `EVENTAREA01` (gcmn area01.cpp; MUT 0x00414110 the
//! constructor, 0x00414c00 `Draw`), which `GO(1)` makes for field 13
//! (Mutation's event 115). The scene `se1_1`: seven static models, one
//! object, a background, eleven lights, and seven `EFF_se1_1ef1` glows that
//! play their animation and now and then flicker (docs/engine/evarea.md).

use piney_data::classes::StoryMapGameClass;
use piney_data::statics::{DrawPass, Position};
use piney_data::{Error, Result};

use crate::ee::{ONE, V4};
use crate::sprite::{EffSprite, EffState};
use crate::story_map::{BASE_BLOCK, BASE_STORY_MAP, BlockDef, LightsDef, Start, StoryMapDef, layer};
use crate::systems::sky::{SkyClump, SkyDef};
use crate::systems::statics::{ModelsDef, ObjectsDef};
use crate::systems::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, PASSES, Piece, Request, Step, StepCx, SystemDef};
use crate::town::FogParams;

/// `game.field` of this map.
pub const AREA: i32 = 13;
const FILE: &str = "se1_1";
const EFF: &str = "EFF_se1_1ef1";
/// Where the glows stand (MUT 0x005ffeb0).
const GLOWS: [&str; 7] = [
    "DMY_effpoint01",
    "DMY_effpoint02",
    "DMY_effpoint03",
    "DMY_effpoint04",
    "DMY_effpoint05",
    "DMY_effpoint06",
    "DMY_effpoint07",
];

/// `eventarea01Light`: the distant light, then ten omni lights.
const LIGHTS: LightsDef = LightsDef {
    anim: "ANM_se1_1bac1a",
    table: &[
        "LGT_se1_1lig1",
        "LGT_se1_1omn01",
        "LGT_se1_1omn02",
        "LGT_se1_1omn03",
        "LGT_se1_1omn04",
        "LGT_se1_1omn05",
        "LGT_se1_1omn06",
        "LGT_se1_1omn07",
        "LGT_se1_1omn08",
        "LGT_se1_1omn09",
        "LGT_se1_1omn10",
    ],
    skip_missing: false,
};

/// `SetFog(150, 4000, 0, 80, 0x1e1e1e)`, and `ccSys.bgColor` 0x1e1e1e.
const FOG: FogParams = FogParams { near: 150.0, far: 4000.0, near_rate: 0.0, far_rate: 80.0, colour: [0x1e; 3] };
pub const BG_COLOR: [u8; 3] = [0x1e; 3];

static MODELS: ModelsDef = ModelsDef::on("EA_MODELTABLE01", layer::OBJ);
static OBJECTS: ObjectsDef = ObjectsDef::on("EA_OBJTABLE01", layer::OBJ);
/// `DrawBG`: the background at the identity, fog off.
static SKY: SkyDef = SkyDef { clumps: &[SkyClump::on("CMP_se1_1bg1", layer::BG0)] };

/// One glow (`eff[k]` +0x1c4, `pos` +0x1e0, `frame` +0x250, `wait`
/// +0x26c, `count` +0x288).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glow {
    pub pos: V4,
    pub frame: u32,
    pub wait: u32,
    pub count: u32,
}

/// `EVENTAREA01`'s own: the glows (`eff[k]` +0x1c4) and the pattern
/// count of their `ccEff` (+0x30), and the frames the last `Draw` drew.
#[derive(Clone, Debug)]
pub struct Flicker {
    pub glows: [Glow; 7],
    pub pat_num: u16,
    frames: [u32; 7],
}

impl Flicker {
    /// The constructor's: each glow's frame, wait and flicker drawn from
    /// `fieldrand` in that order; then their places.
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<Flicker> {
        let file = cx.file;
        let (c, sc) = (&file.ccs, &file.scene);
        let pat_num =
            crate::sky::cloud::eff_pat_num(c, EFF).ok_or_else(|| Error::NotFound(format!("{FILE}: {EFF}")))?;
        let mut glows = [Glow { pos: [0, 0, 0, ONE], frame: 0, wait: 0, count: 0 }; 7];
        for g in &mut glows {
            g.frame = cx.rng.below(u32::from(pat_num));
            g.wait = cx.rng.below(100) + 30;
            g.count = cx.rng.below(15) + 1;
        }
        for (g, name) in glows.iter_mut().zip(GLOWS) {
            if c.find_object(name).is_none() {
                return Err(Error::NotFound(format!("{FILE}: {name}")));
            }
            g.pos = crate::town::dummy_bits(&Position::Dummy(name), c, sc).0;
        }
        Ok(Flicker { glows, pat_num, frames: [0; 7] })
    }
}

impl MapSystem for Flicker {
    /// Each glow's `ccEff::Draw` this frame: waiting, its animation's next
    /// frame (wrapping at the last); its wait out, a random frame, `count`
    /// times, then a new wait (30-129) and flicker (1-15).
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        let n = u32::from(self.pat_num).max(1);
        self.frames = [0; 7];
        if let Some(rng) = &mut cx.core.field_rand {
            for (k, g) in self.glows.iter_mut().enumerate() {
                if g.wait == 0 {
                    self.frames[k] = rng.below(n) & 0xffff;
                    g.count = g.count.wrapping_sub(1);
                    if g.count == 0 {
                        g.wait = rng.below(100) + 30;
                        g.count = rng.below(15) + 1;
                    }
                } else {
                    g.wait -= 1;
                    self.frames[k] = g.frame;
                    g.frame += 1;
                    if g.frame == u32::from(self.pat_num) {
                        g.frame = 0;
                    }
                }
            }
        }
        (0..self.glows.len()).for_each(|k| cx.put(Kind::Flicker, Piece::Item(k)));
    }

    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Item(k) = piece else { return None };
        let (g, pat) = (self.glows.get(k)?, self.frames[k] as u16);
        Some(Request::Sprite(EffSprite::new(FILE, EFF, g.pos, pat, layer::EFF, EffState::NO_FOG)))
    }
}

/// `EVENTAREA01`, field 13: the disc's tables, `DMY_marker_ev01`'s start,
/// `fieldrand` (the glows) the game's one stream. `Draw`: `DrawBG`, on
/// objLayer `DrawObj2`, `DrawObj`, `DrawFloor` and the type-0 rows, then
/// the glows on effLayer. Through a frame the tasks sleep the objects are
/// neither stepped nor drawn.
pub static AREA_13: StoryMapDef = StoryMapDef {
    class: StoryMapGameClass::Eventarea01,
    fields: &[AREA],
    file: FILE,
    clear: BG_COLOR,
    blocks: &[BlockDef {
        lights: Some(LIGHTS),
        fog: Some(FOG),
        start: Start::marker("DMY_marker_ev01"),
        systems: &[
            SystemDef::Models(&MODELS),
            SystemDef::Objects(&OBJECTS),
            SystemDef::Sky(&SKY),
            SystemDef::Flicker(&()),
        ],
        order: &[
            Step::Run(Kind::Sky, Hook::Draw),
            PASSES,
            Step::Run(Kind::Models, Hook::Pass(DrawPass::Other)),
            Step::Run(Kind::Flicker, Hook::Draw),
        ],
        asleep: Some(&[
            Step::Run(Kind::Sky, Hook::Draw),
            Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj2)),
            Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj)),
            Step::Run(Kind::Models, Hook::Pass(DrawPass::Floor)),
            Step::Run(Kind::Models, Hook::Pass(DrawPass::Other)),
            Step::Run(Kind::Flicker, Hook::Draw),
        ]),
        ..BASE_BLOCK
    }],
    ..BASE_STORY_MAP
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story_map::tests::made;
    use crate::systems::statics::{StaticModels, StaticObjects};
    use piney_data::volume::Volume;

    /// Mutation's area 13: the seven models, the object, the eleven
    /// lights, the collision, and the glows' first frames from fieldrand.
    #[test]
    fn area_13() {
        let Some(archive) = crate::town::tests::archive_of("mutation") else { return };
        let mut a = made(&archive, Volume::Mut, &AREA_13, 1, 0);
        assert_eq!(a.systems().get::<StaticModels>().unwrap().len(), 7);
        assert!(a.systems().get::<StaticObjects>().unwrap().object(0).is_some());
        assert_eq!(a.core.lights.lights.len(), 11);
        assert!(!a.core.hits.models.is_empty());
        let f = a.systems().get::<Flicker>().unwrap();
        assert!(f.pat_num > 0);
        assert!(f.glows.iter().all(|g| g.frame < u32::from(f.pat_num) && (30..130).contains(&g.wait)));
        let mut layers = piney_desktop::layers::Layers::default();
        let mut cc = piney_battle::rand::Genrand::default();
        let v = crate::systems::MapView::default();
        let sprites = a.draw(&mut layers, glam::Mat4::IDENTITY, &v, true, &mut cc);
        assert_eq!(sprites.len(), 7);
        assert!(sprites.iter().all(|s| s.state == EffState::NO_FOG && s.layer == layer::EFF));
        assert!(!layers.flatten().is_empty());
    }
}
