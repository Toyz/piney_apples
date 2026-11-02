//! Carmina Gade: `ROOTTOWN03` (MUT gcmn town03.cpp, constructor 0x0043b980,
//! `Draw` 0x0043e400 through the vtable at 0x00388f60), the third Root Town
//! (`game.town` 2), new in Mutation. Infection's executable carries an earlier
//! `ROOTTOWN03` no disc reaches; this is Mutation's. Four sky clumps, 34
//! static models, one water drawn through a frame-buffer copy (two more made
//! but never drawn), and its own `AIRSHIP` flying six legs between
//! `DMY_marker69`-`72`, puffing smoke from `fieldrand`
//! (docs/engine/root-towns.md).

use piney_data::classes::TownGameClass;
use piney_data::dungeon::Rng;
use piney_data::statics::DrawPass;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;

use crate::chara::pose::Play;
use crate::draw::{OBJ_LAYER, OBJ2_LAYER};
use crate::ee::{self, F, ONE, V4};
use crate::systems::scrolls::{ScrollsDef, Wrap};
use crate::systems::sky::{Axes, MatScroll, SkyClump, SkyDef};
use crate::systems::statics::ModelsDef;
use crate::systems::water::{WaterDef, WaterFile, WaterSpec};
use crate::systems::{
    BuildCx, DRAW_BG, DrawCx, Fog, Hook, Kind, MapSystem, Others, Piece, Request, Step, StepCx, SystemDef,
};
use crate::town::scroll_statics as statics;
use crate::town::{BASE_TOWN, FogParams, TownDef, TownEvent};

/// `DrawBG`'s clumps and their layers: the sky's two and the mountains'
/// two, on `bgLayer[0]`-`[3]`.
pub const BACK: [(&str, i16); 4] =
    [("CMP_sr3bac1", -100), ("CMP_sr3bac2", -90), ("CMP_sr3mou1", -80), ("CMP_sr3mou2", -70)];
/// The light animation and `town03Light` (MUT gcmn 0x00603500).
pub const LIGHT_ANIM: &str = "ANM_sr3bac1a";
pub const LIGHTS: [(i32, &str); 12] = [
    (0, "LGT_sr3lig1"),
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
];
/// The water, rooted at the origin (`@1139`, `@1140`).
pub const WATER_ANIM: &str = "ANM_sr3wat1a";
pub const WATER_MODEL: &str = "MDL_sr3wat00";
/// `waterUVModifi2`'s reach: 32000.
const WATER_REACH: F = 0x46fa_0000;

/// `SetFog(3000, 15000, 0, 85, 0x0014140c)` and the same colour as
/// `ccSys.bgColor`: a dark teal.
pub const FOG: FogParams =
    FogParams { near: 3000.0, far: 15000.0, near_rate: 0.0, far_rate: 85.0, colour: [0x0c, 0x14, 0x14] };
pub const BG_COLOR: [u8; 3] = [0x0c, 0x14, 0x14];

/// `DrawBG`'s scrolls a frame, `v$1474` (MUT gcmn gp 0x0038b118): 0.001 and
/// 0.002.
pub const BG_STEP: [F; 2] = [0x3a83_126f, 0x3b03_126f];
/// The water's scroll a frame, `u$1875`: 0.005, back to 0 at 1.
pub const WATER: WaterSpec = WaterSpec { step: 0x3ba3_d70a, wrap: Wrap::AtOrPast };
/// The rows `Draw` draws without fog first (29-33), below which it draws
/// the type-0 rows after them, and `DrawFloor`'s rows (0-7).
const NO_FOG_ROWS: (usize, usize) = (29, 34);
const FLOOR_ROWS: (usize, usize) = (0, 8);

/// In crisis (`town03d`) the clumps over them, on `bgLayer[4]`-`[6]`:
/// `MAT_sr3dat1_2` by the second scroll in U, `_3` in U and V.
const CRISIS_MATS: [MatScroll; 2] =
    [MatScroll::new("MAT_sr3dat1_2", 1, Axes::U), MatScroll::new("MAT_sr3dat1_3", 1, Axes::UV)];

static MODELS: ModelsDef = ModelsDef { table: "RT_MODELTABLE03", layer: OBJ2_LAYER };
static SCROLLS: ScrollsDef = ScrollsDef { steps: &BG_STEP, slot: statics::TOWN03_BG };
static SKY_CLUMPS: SkyDef = SkyDef {
    clumps: &[
        SkyClump::on(BACK[0].0, BACK[0].1),
        SkyClump::on(BACK[1].0, BACK[1].1),
        SkyClump::on(BACK[2].0, BACK[2].1),
        SkyClump::on(BACK[3].0, BACK[3].1),
        SkyClump::crisis("CMP_sr3dat1_1", -60, &CRISIS_MATS),
        SkyClump::crisis("CMP_sr3dat1_2", -50, &CRISIS_MATS),
        SkyClump::crisis("CMP_sr3dat1_3", -45, &CRISIS_MATS),
    ],
};
/// The town's own water at the origin: only 0 drawn (+0x1bc..+0x1c4 made),
/// the copy after it.
static WATER_DEF: WaterDef = WaterDef {
    file: WaterFile::Area,
    anim: WATER_ANIM,
    model: WATER_MODEL,
    reach: WATER_REACH,
    root: [0, 0, 0, ONE],
    spec: WATER,
    slot: statics::TOWN03_WATER,
    drawn: &[0],
    layer: OBJ_LAYER,
    copy: true,
};

/// `ROOTTOWN03` (`town03`, `town03d` in crisis): its `Draw` is water 0 and
/// the copy, the airship's `Move` and draw, `DrawBG`, `DrawObj2` and
/// `DrawObj` (the rows of type 3, 2), `DrawFloor` (rows 0-7), rows 29-33
/// without fog and the type-0 rows below them, `DrawMap`; the water's
/// scroll steps last.
pub static CARMINA_GADE: TownDef = TownDef {
    no: 2,
    class: TownGameClass::Roottown03,
    name: "Carmina Gadelica",
    file: "town03",
    crisis_file: Some("town03d"),
    light_anim: LIGHT_ANIM,
    lights: &LIGHTS,
    fog: FOG,
    clear: Some(BG_COLOR),
    start: Some(([0, 0x4416_0000, 0, ONE], crate::START_DIRC)),
    systems: &[
        SystemDef::Models(&MODELS),
        SystemDef::Scrolls(&SCROLLS),
        SystemDef::Sky(&SKY_CLUMPS),
        SystemDef::Water(&WATER_DEF),
        SystemDef::Airship(&()),
    ],
    order: &[
        Step::Run(Kind::Water, Hook::Draw),
        Step::Run(Kind::Airship, Hook::Draw),
        DRAW_BG,
        Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj2)),
        Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj)),
        Step::Run(Kind::Models, Hook::Rows(FLOOR_ROWS.0, FLOOR_ROWS.1, Fog::Area)),
        Step::Run(Kind::Models, Hook::Rows(NO_FOG_ROWS.0, NO_FOG_ROWS.1, Fog::None)),
        Step::Run(Kind::Models, Hook::PassBelow(DrawPass::Other, NO_FOG_ROWS.0, Fog::None)),
        Step::Map,
    ],
    ..BASE_TOWN
};

/// The airship's dummies, its clip and its numbers.
pub mod ship {
    use crate::ee::F;

    pub const MARKERS: [&str; 4] = ["DMY_marker69", "DMY_marker70", "DMY_marker71", "DMY_marker72"];
    pub const ANIM: &str = "ANM_sr3shi01_a";
    /// Each leg's (from, to) markers, by `+0x04`.
    pub const LEGS: [(usize, usize); 6] = [(0, 1), (1, 2), (2, 3), (3, 2), (2, 1), (1, 0)];
    /// The model's height below the ship's place: z -500 while it is set.
    pub const DRAW_Z: F = 0xc3fa_0000;
    /// A leg's `t` a frame: 0.0005.
    pub const T_STEP: F = 0x3a03_126f;
    /// The turn's and the wait's frames.
    pub const TURN: i32 = 300;
    pub const WAIT: i32 = 300;
    /// By the leg just begun: the turn a frame (`dirc.z` less it for
    /// legs 0, 4 and 5, plus it for the others) and the heading the turn
    /// ends on. pi/300 (0x3c2b92a6) and -pi/600 (0xbbab92a6).
    pub const TURNS: [(F, bool, F); 6] = [
        (0x3c2b_92a6, false, 0),
        (0xbbab_92a6, true, 0xbfc9_0fdb),
        (0xbbab_92a6, true, 0x4049_0fdb),
        (0xbc2b_92a6, true, 0),
        (0xbbab_92a6, false, 0x3fc9_0fdb),
        (0xbbab_92a6, false, 0x4049_0fdb),
    ];
    /// The chimneys (in the ship's frame, -180 and 180 across, 500 on, 900
    /// up), the puffs' velocity while it flies and while it waits (x -2 and
    /// 2), and their texture and fades.
    pub const CHIMNEYS: [F; 2] = [0xc334_0000, 0x4334_0000];
    pub const CHIMNEY_Y: F = 0x43fa_0000;
    pub const CHIMNEY_Z: F = 0x4461_0000;
    pub const PUFF_X: [F; 2] = [0xc000_0000, 0x4000_0000];
    pub const FLY_V: [F; 2] = [0x4040_0000, 0x40a0_0000];
    pub const WAIT_V: [F; 2] = [0x3f80_0000, 0xc100_0000];
    pub const PUFF_TEX: i32 = 109;
    pub const PUFF_FADE: (i16, i16) = (512, 32);
    /// `ccSeOn3D(257, pos)` with each pair of puffs while it waits.
    pub const SE: i32 = 257;
}

/// The `AIRSHIP` (0xd0 bytes).
#[derive(Clone, Debug)]
pub struct Airship {
    /// +0x04 the leg (0-5), +0x0c the turn's frames left, +0x10 the wait's,
    /// +0xb0 the leg's `t`, +0xb8 the frames to the next puffs.
    pub leg: usize,
    pub turn: i32,
    pub wait: i32,
    pub t: F,
    pub puff: i32,
    /// +0x20 its place (z 0 once it has moved), +0x30 its heading (only z
    /// turns), +0x40 the leg's way (to less the place it began from).
    pub pos: V4,
    pub dirc: V4,
    pub way: V4,
    /// +0x50..+0x80 the four markers, +0x90 the leg's start, +0xa0 its end.
    pub markers: [V4; 4],
    pub from: V4,
    pub to: V4,
    /// +0xbc its ccAnm, and the root `Move` set it at before it moved.
    pub play: Play,
    pub root: [V4; 4],
}

impl Airship {
    /// `town03Ship` as `ROOTTOWN03`'s constructor makes it.
    pub fn new(_: &'static (), cx: &mut BuildCx) -> Result<Airship> {
        Airship::read(cx.file)
    }

    /// `AIRSHIP::AIRSHIP(stream)`: at marker 69 bound for 70, the first
    /// puffs next frame.
    pub fn read(file: &SceneFile) -> Result<Airship> {
        let dummy = |name: &str| {
            file.ccs
                .find_object(name)
                .and_then(|o| file.scene.dummies.get(&o))
                .map(|d| [d.pos.x.to_bits(), d.pos.y.to_bits(), d.pos.z.to_bits(), ONE])
                .ok_or_else(|| Error::NotFound(name.into()))
        };
        let markers =
            [dummy(ship::MARKERS[0])?, dummy(ship::MARKERS[1])?, dummy(ship::MARKERS[2])?, dummy(ship::MARKERS[3])?];
        let play = Play::new(file, ship::ANIM).ok_or_else(|| Error::NotFound(ship::ANIM.into()))?;
        let (pos, to) = (markers[0], markers[1]);
        Ok(Airship {
            leg: 0,
            turn: 0,
            wait: 0,
            t: 0,
            puff: 1,
            pos,
            dirc: [0; 4],
            way: sub(to, pos),
            markers,
            from: pos,
            to,
            play,
            root: unit(),
        })
    }

    /// The root `Move` sets the ccAnm at: `SetMatrix_PosRotXYZ` of the place
    /// 500 lower and the heading.
    fn set_root(&self) -> [V4; 4] {
        let mut p = self.pos;
        p[2] = ship::DRAW_Z;
        let r = self.dirc;
        let m = piney_data::anim::rot_z_bits_of(
            piney_data::anim::rot_y_bits_of(piney_data::anim::rot_x_bits_of(unit(), r[0]), r[1]),
            r[2],
        );
        let mut m = m;
        for (k, &v) in p.iter().take(3).enumerate() {
            m[3][k] = ee::add(m[3][k], v);
        }
        m
    }

    /// `AIRSHIP::Move`: the ccAnm set and stepped, then the turn, the wait
    /// or the leg; the puffs and sounds it makes go to `events`.
    pub fn move_(&mut self, file: &SceneFile, rng: &mut Rng, events: &mut Vec<TownEvent>) {
        self.root = self.set_root();
        self.play.forward(file);
        self.pos[2] = 0;
        if self.turn != 0 {
            self.turn -= 1;
            if let Some(&(d, add, end)) = ship::TURNS.get(self.leg) {
                self.dirc[2] = if add { ee::add(self.dirc[2], d) } else { ee::sub(self.dirc[2], d) };
                if self.turn == 0 {
                    self.dirc[2] = end;
                }
            }
            if self.turn == 0 {
                self.wait = ship::WAIT;
            }
            return;
        }
        if self.wait != 0 {
            self.wait -= 1;
            if self.puff == 0 {
                self.puffs(ship::WAIT_V, rng, events);
                events.push(TownEvent::Se3d { n: ship::SE, pos: self.pos });
                self.puff = rng.below(8) as i32 + 6;
            } else {
                self.puff -= 1;
            }
            return;
        }
        self.t = ee::add(ship::T_STEP, self.t);
        if !ee::lt(self.t, ONE) {
            self.t = 0;
            self.pos = self.to;
            self.turn = ship::TURN;
            self.leg = (self.leg + 1) % 6;
            let (a, b) = ship::LEGS[self.leg];
            self.from = self.markers[a];
            self.to = self.markers[b];
            self.way = sub(self.to, self.pos);
            return;
        }
        let s = scale(self.way, self.t);
        self.pos = add(self.from, s);
        if self.puff == 0 {
            self.puffs(ship::FLY_V, rng, events);
            self.puff = rng.below(8) as i32 + 6;
        } else {
            self.puff -= 1;
        }
    }

    /// The two chimneys' puffs: each place and velocity turned by the
    /// heading (`sceVu0RotMatrix`), the place from the ship's; the size 3
    /// to 5 and the life 40 to 79 from `fieldrand`.
    fn puffs(&self, v: [F; 2], rng: &mut Rng, events: &mut Vec<TownEvent>) {
        let m = piney_data::anim::rot_bits([self.dirc[0], self.dirc[1], self.dirc[2]]);
        for k in 0..2 {
            let at = apply(&m, [ship::CHIMNEYS[k], ship::CHIMNEY_Y, ship::CHIMNEY_Z, ONE]);
            let vel = apply(&m, [ship::PUFF_X[k], v[0], v[1], ONE]);
            let pos = add(at, self.pos);
            let scale = ee::from_int(rng.below(3) as i32 + 3);
            let life = rng.below(40) as i32 + 40;
            events.push(TownEvent::SmokeN { pos, v: vel, scale, life, kind: ship::PUFF_TEX, fade: ship::PUFF_FADE });
        }
    }
}

fn unit() -> [V4; 4] {
    [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]]
}

/// `sceVu0AddVector` (all four).
fn add(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| ee::add(a[i], b[i]))
}

/// `sceVu0SubVector` (all four).
fn sub(a: V4, b: V4) -> V4 {
    std::array::from_fn(|i| ee::sub(a[i], b[i]))
}

/// `sceVu0ScaleVector` (all four).
fn scale(a: V4, t: F) -> V4 {
    a.map(|x| ee::mul(x, t))
}

/// `sceVu0ApplyMatrix`.
fn apply(m: &[V4; 4], v: V4) -> V4 {
    let mut out = [0; 4];
    for (i, o) in out.iter_mut().enumerate() {
        let mut s = ee::mul(m[0][i], v[0]);
        for c in 1..4 {
            s = ee::add(s, ee::mul(m[c][i], v[c]));
        }
        *o = s;
    }
    out
}

impl MapSystem for Airship {
    /// `AIRSHIP::Move` from `fieldrand`, then its ccAnm drawn where it set it.
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw {
            return;
        }
        if let Some(rng) = &mut cx.core.field_rand {
            self.move_(&cx.core.file, rng, cx.events);
        }
        cx.put(Kind::Airship, Piece::Anim(0));
    }

    fn draw<'a>(&'a self, piece: Piece, cx: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Anim(_) = piece else { return None };
        let root = crate::draw::mat(&self.root);
        let (file, play, fog) = (&*cx.core.file, &self.play, cx.fog);
        Some(Request::Anim { file, play, root, layer: OBJ_LAYER, rows: None, fog, edit: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::Drawn;
    use crate::systems::sky::Sky;
    use crate::systems::statics::StaticModels;
    use crate::town::{Town, TownView};
    use piney_data::volume::Volume;

    fn town() -> Option<Town> {
        let archive = crate::town::tests::archive_of("mutation")?;
        Some(Town::open(&archive, Volume::Mut, 2, false).unwrap())
    }

    /// The constructor's pieces: 34 model rows, no static objects, the four
    /// background clumps, the lights, the fog and background colour, and
    /// the airship at marker 69 bound for 70.
    #[test]
    fn constructor() {
        let Some(t) = town() else { return };
        let s = t.systems();
        assert_eq!(t.def.no, 2);
        let models = s.get::<StaticModels>().unwrap();
        assert_eq!((0..34).filter(|&r| models.model(r).is_some()).count(), 34);
        let sky = s.get::<Sky>().unwrap();
        assert!((0..4).all(|k| sky.made(k)) && (4..7).all(|k| !sky.made(k)));
        assert_eq!(t.core.lights.lights.len(), 12);
        assert_eq!(t.def.clear, Some(BG_COLOR));
        let ship = s.get::<Airship>().unwrap();
        assert_eq!((ship.pos, ship.to), (ship.markers[0], ship.markers[1]));
        assert!(!t.core.hits.models.is_empty());
    }

    /// Every frame's pieces come in `Draw`'s order, and the airship flies
    /// its first leg: 2000 frames, then the turn.
    #[test]
    fn the_airship_flies_its_first_leg() {
        let Some(mut t) = town() else { return };
        let v = TownView::default();
        t.step(&v);
        let p = |k: Kind, piece: Piece| Drawn::Piece(k, piece);
        assert_eq!(
            &t.frame()[..7],
            &[
                p(Kind::Water, Piece::Anim(0)),
                p(Kind::Water, Piece::Copy),
                p(Kind::Airship, Piece::Anim(0)),
                p(Kind::Sky, Piece::Clump(0)),
                p(Kind::Sky, Piece::Clump(1)),
                p(Kind::Sky, Piece::Clump(2)),
                p(Kind::Sky, Piece::Clump(3)),
            ]
        );
        assert_eq!(t.frame().last(), Some(&Drawn::Map));
        let mut frames = 1;
        while t.systems().get::<Airship>().unwrap().turn == 0 {
            t.step(&v);
            frames += 1;
            assert!(frames < 2100, "the first leg did not end");
        }
        // t += 0.0005 in single floats: just short of 1 after 2000 steps.
        assert!((2000..2010).contains(&frames), "{frames}");
        let ship = t.systems().get::<Airship>().unwrap();
        assert_eq!((ship.leg, ship.turn, ship.pos), (1, ship::TURN, ship.markers[1]));
        assert!(t.take_events().iter().any(|e| matches!(e, TownEvent::SmokeN { .. })));
    }
}
