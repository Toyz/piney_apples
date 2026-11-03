//! The fireflies, two classes of the game's: `FIREFLY2`
//! ([`crate::sky::firefly`]), the boss arenas' made over the file's markers
//! and field type 9's roaming about the camera ([`Fireflies2`]); and
//! `FIREFLY` ([`crate::field::firefly`]), a field's or a lake room's by
//! night or hacked ([`Fireflies`]). Each moves and draws from `fieldrand`
//! every frame.

use piney_data::statics::Position;
use piney_data::{Error, Result};

use super::{BuildCx, DrawCx, Hook, Kind, MapSystem, Others, Piece, Request, RoomCx, RoomPart, StepCx};
use crate::draw::EFF_LAYER;
use crate::ee;
use crate::field::ambient::{EFF_FILE, Env, Ground, uint_float};
use crate::field::firefly::FieldFirefly;
use crate::sky::firefly::{Firefly, FireflyDraw};
use crate::sprite::{EffSprite, EffState};

/// Who makes the `FIREFLY2`s, and so how they fly.
#[derive(Clone, Copy, Debug)]
pub enum Firefly2Made {
    /// The arena's constructor, the k-th over marker `k % markers`; each
    /// frame drawn, then moved.
    Markers(&'static [&'static str]),
    /// Field type 9's `Generate` about the start, roaming about the camera;
    /// each frame moved, then drawn, not in the rain.
    Roam,
}

/// The sprite the `FIREFLY2`s draw.
#[derive(Clone, Copy, Debug)]
pub enum Firefly2Eff {
    /// The area's own file's `bossFireFlyEff`, by `game.field`.
    ByField(&'static [&'static str]),
    /// `field_eff`'s.
    Field(&'static str),
}

/// How many are made, by whom, and their sprite.
#[derive(Clone, Copy, Debug)]
pub struct Firefly2Def {
    pub count: usize,
    pub made: Firefly2Made,
    pub eff: Firefly2Eff,
}

/// The `FIREFLY2`s (+0x1c4), their sprite, and for roamers whether it
/// rains.
#[derive(Clone, Debug)]
pub struct Fireflies2 {
    def: &'static Firefly2Def,
    pub fireflies: Vec<Firefly>,
    file: &'static str,
    eff: &'static str,
    rains: bool,
}

impl Fireflies2 {
    pub fn new(def: &'static Firefly2Def, cx: &mut BuildCx) -> Result<Fireflies2> {
        let volume = cx.volume;
        let mut fireflies = Vec::with_capacity(def.count);
        let mut rains = false;
        match def.made {
            Firefly2Made::Markers(markers) => {
                let file = cx.file;
                let (c, sc) = (&file.ccs, &file.scene);
                for k in 0..def.count {
                    let marker = markers[k % markers.len()];
                    if c.find_object(marker).is_none() {
                        return Err(Error::NotFound(format!("{}: {marker}", file.stem)));
                    }
                    let (at, _) = crate::town::dummy_bits(&Position::Dummy(marker), c, sc);
                    fireflies.push(Firefly::arena(volume, &mut cx.rng, at));
                }
            }
            // WORLD::Generate's: SetDirc of three (fieldrand(2000) - 1000)
            // / 1000, SetBasePosition at the start raised by fieldrand(300).
            Firefly2Made::Roam => {
                let f = cx.field.expect("a field's fireflies");
                let area = f.field.expect("a field");
                rains = f.weather.rains();
                let env = Env::at(volume, f.centre, Ground::Field(&area.field));
                let fly = env.fly();
                let start = area.start_pos();
                let rng = &mut cx.rng;
                let d = |rng: &mut piney_data::dungeon::Rng| {
                    ee::div(ee::from_int(rng.below(2000) as i32 - 1000), 0x447a_0000)
                };
                for _ in 0..def.count {
                    let dirc = [d(rng), d(rng), d(rng), 0];
                    let mut base = start;
                    base[2] = ee::add(base[2], uint_float(rng.below(300)));
                    fireflies.push(Firefly::roamer(rng, dirc, base, &fly));
                }
            }
        }
        let (file, eff) = match def.eff {
            Firefly2Eff::ByField(effs) => {
                (cx.stem, usize::try_from(cx.field()).ok().and_then(|f| effs.get(f)).copied().unwrap_or(""))
            }
            Firefly2Eff::Field(eff) => (EFF_FILE, eff),
        };
        Ok(Fireflies2 { def, fireflies, file, eff, rains })
    }
}

impl MapSystem for Fireflies2 {
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        if hook != Hook::Draw || self.rains {
            return;
        }
        let Some(mut rng) = cx.core.field_rand else { return };
        let mut drawn: Vec<FireflyDraw> = Vec::new();
        {
            let env = Env::of(cx);
            let fly = env.fly();
            for f in &mut self.fireflies {
                match self.def.made {
                    Firefly2Made::Markers(_) => {
                        f.draw_in(&fly, &mut drawn);
                        f.step_in(&fly, &mut rng);
                    }
                    Firefly2Made::Roam => {
                        f.step_in(&fly, &mut rng);
                        f.draw_in(&fly, &mut drawn);
                    }
                }
            }
        }
        cx.core.field_rand = Some(rng);
        drawn.into_iter().for_each(|d| cx.put(Kind::Firefly2, Piece::Fly(d)));
    }

    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Fly(d) = piece else { return None };
        Some(Request::Sprite(EffSprite {
            scale: [d.scale; 2],
            transparency: d.transparency,
            ..EffSprite::new(self.file, self.eff, d.pos, d.pattern, EFF_LAYER, EffState::INIT)
        }))
    }
}

/// Who makes the `FIREFLY`s: a field's `Generate` (each at a random place
/// of the field), or a lake's `SetRoom` (0x005c38c0, by the room's centre,
/// anew for each room).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Made {
    Generate,
    Room,
}

/// How many `FIREFLY`s are made by night or hacked, and by whom.
#[derive(Clone, Copy, Debug)]
pub struct FirefliesDef {
    pub count: usize,
    pub made: Made,
}

/// The `FIREFLY`s, whether they are made (by night or hacked) and hacked,
/// whether the area has a field (their new bases stand on it), their
/// sparks' sprite, and this frame's sprites.
#[derive(Clone, Debug)]
pub struct Fireflies {
    def: &'static FirefliesDef,
    pub fireflies: Vec<FieldFirefly>,
    night: bool,
    hacked: bool,
    field: bool,
    spark: &'static str,
    drawn: Vec<EffSprite>,
}

impl Fireflies {
    /// `WORLD::Generate`'s: by night or hacked, each at a random place of
    /// the field. A lake's come with each room.
    pub fn new(def: &'static FirefliesDef, cx: &mut BuildCx) -> Result<Fireflies> {
        let f = cx.field.expect("an area's fireflies");
        let w = f.weather;
        let night = w.time() == 2 || w.hacked();
        let mut fireflies = Vec::new();
        if let (Made::Generate, Some(area), true) = (def.made, f.field, night) {
            let bounds = [0x473b_8000, 0x473b_8000];
            for _ in 0..def.count {
                fireflies.push(FieldFirefly::new(w.hacked(), bounds, Ground::Field(&area.field), &mut cx.rng));
            }
        }
        let (field, spark) = (f.def.field, f.def.spark);
        Ok(Fireflies { def, fireflies, night, hacked: w.hacked(), field, spark, drawn: Vec::new() })
    }
}

impl MapSystem for Fireflies {
    /// `Draw`: each one's `Move`, then its `Draw`. `Again`: its `Draw`
    /// alone (a lake's, which draws while the tasks sleep).
    fn step(&mut self, hook: Hook, cx: &mut StepCx, _: &mut Others) {
        let step = match hook {
            Hook::Draw => true,
            Hook::Again => false,
            _ => return,
        };
        if self.fireflies.is_empty() {
            return;
        }
        let Some(mut rng) = cx.core.field_rand else { return };
        self.drawn.clear();
        {
            let env = Env::of(cx);
            for f in &mut self.fireflies {
                if step {
                    f.step(&env, self.field, &mut rng);
                }
                f.draw(&env, self.spark, &mut rng, &mut self.drawn);
            }
        }
        cx.core.field_rand = Some(rng);
        (0..self.drawn.len()).for_each(|k| cx.put(Kind::Fireflies, Piece::Item(k)));
    }

    /// A lake's `SetRoom`: by night or hacked, five new ones by the room's
    /// centre.
    fn room(&mut self, part: RoomPart, cx: &mut RoomCx, _: &mut Others) {
        let (RoomPart::Fireflies, Made::Room, true, Some(slot)) = (part, self.def.made, self.night, cx.slot) else {
            return;
        };
        if cx.room.is_none() {
            return;
        }
        let hacked = self.hacked;
        self.fireflies = (0..self.def.count).map(|_| FieldFirefly::lake(hacked, slot.pos, cx.rng)).collect();
    }

    fn draw<'a>(&'a self, piece: Piece, _: &DrawCx<'a>) -> Option<Request<'a>> {
        let Piece::Item(k) = piece else { return None };
        self.drawn.get(k).map(|s| Request::Sprite(*s))
    }
}
