//! A map's systems and the one runner that steps and draws them. A def
//! (`crate::town::TownDef`) names its systems ([`SystemDef`], each built
//! from its own data) and its `Draw`'s [`Step`]s in the game's order. A
//! step puts what it shows into the frame ([`Drawn`]); after the frame each
//! system turns its pieces into generic draw [`Request`]s, which the world
//! paints the same way for every map ([`paint`]). Siblings reach each other
//! only by kind ([`Others`]); carried state goes through each system's
//! `take` / `put` of the scroll statics.

pub mod clouds;
pub mod flare;
pub mod scrolls;
pub mod sky;
pub mod statics;
pub mod water;

use std::collections::HashMap;
use std::sync::Arc;

use glam::Mat4;
use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::dungeon::Rng;
use piney_data::statics::DrawPass;
use piney_data::volume::Volume;
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::area::AreaCore;
use crate::chara::pose::Play;
use crate::draw::{self, Draw, Fogging};
use crate::ee::V4;
use crate::town::{TownEvent, TownSprite, TownView, anm_pieces, object_morph};

/// The executable's scroll statics (`crate::town::scroll_statics`).
pub type Statics = [u32; piney_desktop::SCROLL_STATICS];

/// One step of a map's `Draw`, in its def's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// A system's step; a map without that system has nothing to step.
    Run(Kind, Hook),
    /// `DrawMap`: the minimap's place in the order (the host draws it).
    Map,
    /// Steps written once and shared by name ([`PASSES`]).
    Seq(&'static [Step]),
}

/// Which of a system's steps a [`Step::Run`] runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hook {
    /// Its part of `Draw`.
    Draw,
    /// What the first `Draw` makes (the clouds about the player).
    Start,
    /// The camera eye's clip (Mac Anu's `clip[3]`).
    Clip,
    /// A pass's model rows (with the area's fog) or objects: `DrawObj2`,
    /// `DrawObj`, `DrawFloor`, the type-0 rows.
    Pass(DrawPass),
    /// Model rows by number, `from` up to `to`.
    Rows(usize, usize, Fog),
    /// A pass's model rows below a row.
    PassBelow(DrawPass, usize, Fog),
}

/// Whether a model row draws with the area's fog (`STATICMODEL::Draw`) or
/// none (`DrawWithOutFog`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fog {
    Area,
    None,
}

/// `DrawObj2`, `DrawObj` and `DrawFloor` as most towns write them: each
/// pass's rows, the first two then their static objects.
pub const PASSES: Step = Step::Seq(&[
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj2)),
    Step::Run(Kind::Objects, Hook::Pass(DrawPass::Obj2)),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Objects, Hook::Pass(DrawPass::Obj)),
    Step::Run(Kind::Models, Hook::Pass(DrawPass::Floor)),
]);

/// `DrawBG`: its scrolls, then its clumps.
pub const DRAW_BG: Step = Step::Seq(&[Step::Run(Kind::Scrolls, Hook::Draw), Step::Run(Kind::Sky, Hook::Draw)]);

/// What a system put in the frame, for its own draw to read back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// A static object (its table row), stepped.
    Object(usize),
    /// A static model row, with the area's fog or without.
    Model(usize),
    ModelNoFog(usize),
    /// The system's clump `k`.
    Clump(usize),
    /// The system's animation `k` (`waterAnm[k]`, the airship's).
    Anim(usize),
    /// `ccLayer::MakePacketDrawBuffTrans`: the picture so far copied into
    /// the water's texture.
    Copy,
    /// The system's `ccEff` `k` at `pos` with a pattern.
    Sprite {
        k: usize,
        pattern: u16,
        pos: V4,
    },
}

/// One entry of the frame, in the order `Draw` made it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drawn {
    /// A system's piece, by the kind that draws it.
    Piece(Kind, Piece),
    /// `DrawMap`.
    Map,
}

/// A generic draw: what the world paints for any map.
pub enum Request<'a> {
    /// Models of `file` at `world` (`ccClump::Draw`, `STATICMODEL::Draw`)
    /// with the scrolled materials' offsets `rows`.
    Models {
        file: &'a SceneFile,
        models: &'a [u32],
        world: Mat4,
        layer: i16,
        rows: HashMap<u32, [u8; 2]>,
        fog: Fogging,
    },
    /// `ccAnm::Draw` of `play` under `root`: `rows` over the animation's
    /// own offsets, and one model drawn with its vertex edits (the water's).
    Anim {
        file: &'a SceneFile,
        play: &'a Play,
        root: Mat4,
        layer: i16,
        rows: Option<HashMap<u32, [u8; 2]>>,
        fog: Fogging,
        edit: Option<(u32, Arc<piney_draw::VertexEdits>)>,
    },
    /// A `ccEff` for the host's effects.
    Sprite(TownSprite),
}

/// `r` into the layers; a sprite goes to `sprites` for the host.
pub fn paint(layers: &mut Layers, to_screen: Mat4, r: &Request, sprites: &mut Vec<TownSprite>) {
    match r {
        Request::Models { file, models, world, layer, rows, fog } => {
            for &model in *models {
                let d = Draw {
                    file,
                    model,
                    world: *world,
                    alpha: 1.0,
                    rows,
                    lights: None,
                    nodes: &[],
                    morph: Vec::new(),
                    clut_swaps: Vec::new(),
                };
                draw::model_edited(layers, *layer, to_screen, &d, None, *fog);
            }
        }
        Request::Anim { file, play, root, layer, rows, fog, edit } => {
            let own = play.uv_rows(file);
            let rows = rows.as_ref().unwrap_or(&own);
            let morph = play.morph(file);
            for (entry, obj, world) in anm_pieces(file, play, *root, &[]) {
                let target = file.scene.ext.get(&obj).copied().unwrap_or(obj);
                let Some(&model) = file.obj_model.get(&target) else { continue };
                let m = object_morph(file, &morph, entry);
                let e = edit.as_ref().filter(|(w, _)| *w == model).map(|(_, e)| e);
                let d = Draw {
                    file,
                    model,
                    world,
                    alpha: 1.0,
                    rows,
                    lights: None,
                    nodes: &[],
                    morph: m,
                    clut_swaps: Vec::new(),
                };
                draw::model_edited(layers, *layer, to_screen, &d, e, *fog);
            }
        }
        Request::Sprite(s) => sprites.push(*s),
    }
}

/// What a system's constructor reads: the disc, the area's file, the
/// save's crisis byte and a copy of `fieldrand` (what a constructor draws
/// from it is not kept: `GO(0)` hands the town the stream it was given).
pub struct BuildCx<'a> {
    pub archive: &'a Arc<Archive>,
    /// Whose tables the statics read.
    pub tables: Volume,
    pub file: &'a std::rc::Rc<SceneFile>,
    pub crisis: bool,
    pub rng: Rng,
}

/// What a step reads and writes: the area's core (its file, `fieldrand`,
/// the volume its collision takes), the view, the frame so far and the
/// effects and sounds started.
pub struct StepCx<'a> {
    pub core: &'a mut AreaCore,
    pub view: &'a TownView,
    pub frame: &'a mut Vec<Drawn>,
    pub events: &'a mut Vec<TownEvent>,
}

impl StepCx<'_> {
    /// `piece` in the frame, drawn by `kind`.
    pub fn put(&mut self, kind: Kind, piece: Piece) {
        self.frame.push(Drawn::Piece(kind, piece));
    }
}

/// What a draw reads: the area's core and its fog as VU1 puts it on the
/// buildings (`SetFog`).
pub struct DrawCx<'a> {
    pub core: &'a AreaCore,
    pub fog: Fogging,
}

/// What a system does: its steps, its pieces as requests, and what of it
/// carries across visits.
pub(crate) trait MapSystem {
    fn step(&mut self, _hook: Hook, _cx: &mut StepCx, _others: &mut Others) {}

    fn draw<'a>(&'a self, _piece: Piece, _cx: &DrawCx<'a>) -> Option<Request<'a>> {
        None
    }

    /// The scroll statics it holds, as the game holds them.
    fn take(&mut self, _s: &Statics) {}
    fn put(&self, _s: &mut Statics) {}
}

/// A system's own type, found in a list by its kind.
pub trait SystemKind: Sized {
    const KIND: Kind;
    /// Whether its draws take from `fieldrand` (the area then holds it).
    const FIELD_RAND: bool;
    fn of(s: &System) -> Option<&Self>;
    fn of_mut(s: &mut System) -> Option<&mut Self>;
}

/// The systems: their objects, their kinds, what a def builds each from,
/// and the one `match` that steps and draws each.
macro_rules! systems {
    ($($(#[$m:meta])* $v:ident($t:ty, $d:ty, $rand:expr)),* $(,)?) => {
        /// One of a map's systems.
        pub enum System {
            $($(#[$m])* $v(Box<$t>),)*
        }

        /// A system's kind.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Kind {
            $($v,)*
        }

        /// A system as a def names it: its kind and what it is built from.
        #[derive(Clone, Copy, Debug)]
        pub enum SystemDef {
            $($v(&'static $d),)*
        }

        impl SystemDef {
            pub fn kind(&self) -> Kind {
                match self {
                    $(SystemDef::$v(_) => Kind::$v,)*
                }
            }

            /// Whether the system's draws take from `fieldrand`.
            pub fn field_rand(&self) -> bool {
                match self {
                    $(SystemDef::$v(_) => <$t as SystemKind>::FIELD_RAND,)*
                }
            }

            /// The system as its class's constructor makes it.
            pub fn build(&self, cx: &mut BuildCx) -> Result<System> {
                Ok(match self {
                    $(SystemDef::$v(d) => System::$v(Box::new(<$t>::new(d, cx)?)),)*
                })
            }
        }

        impl System {
            pub fn kind(&self) -> Kind {
                match self {
                    $(System::$v(_) => Kind::$v,)*
                }
            }

            fn step(&mut self, hook: Hook, cx: &mut StepCx, others: &mut Others) {
                match self {
                    $(System::$v(s) => MapSystem::step(&mut **s, hook, cx, others),)*
                }
            }

            fn draw<'a>(&'a self, piece: Piece, cx: &DrawCx<'a>) -> Option<Request<'a>> {
                match self {
                    $(System::$v(s) => MapSystem::draw(&**s, piece, cx),)*
                }
            }

            fn take(&mut self, s: &Statics) {
                match self {
                    $(System::$v(x) => MapSystem::take(&mut **x, s),)*
                }
            }

            fn put(&self, s: &mut Statics) {
                match self {
                    $(System::$v(x) => MapSystem::put(&**x, s),)*
                }
            }
        }

        $(impl SystemKind for $t {
            const KIND: Kind = Kind::$v;
            const FIELD_RAND: bool = $rand;

            fn of(s: &System) -> Option<&Self> {
                match s {
                    System::$v(x) => Some(x),
                    #[allow(unreachable_patterns)]
                    _ => None,
                }
            }

            fn of_mut(s: &mut System) -> Option<&mut Self> {
                match s {
                    System::$v(x) => Some(x),
                    #[allow(unreachable_patterns)]
                    _ => None,
                }
            }
        })*
    };
}

systems! {
    /// `STATICMODEL`s: a model table's placed rows.
    Models(statics::StaticModels, statics::ModelsDef, false),
    /// `STATICOBJECT`s: an object table's animated objects.
    Objects(statics::StaticObjects, statics::ObjectsDef, false),
    /// `DrawBG`'s scrolls, carried in the executable's statics.
    Scrolls(scrolls::Scrolls, scrolls::ScrollsDef, false),
    /// `DrawBG`'s clumps, the crisis sky's among them.
    Sky(sky::Sky, sky::SkyDef, false),
    /// A town's water: `waterAnm[0..2]`, water 0 through the picture.
    Water(water::Water, water::WaterDef, false),
    /// `CLOUD`s about the player.
    Clouds(clouds::Clouds, clouds::CloudsDef, true),
    /// `LENSFLARE`.
    LensFlare(flare::LensFlare, flare::FlareDef, false),
    /// Mac Anu's own: its camera-eye clip of the rows and objects.
    MacAnu(crate::town::mac_anu::Clip, (), false),
    /// Carmina Gade's own: its `AIRSHIP`.
    Airship(crate::town::carmina_gadelica::Airship, (), true),
    /// Lia Fail's own: its omni points' glows.
    Glows(crate::town::lia_fail::Glows, (), true),
}

/// A map's systems, in its def's order.
#[derive(Default)]
pub struct Systems(Vec<System>);

impl Systems {
    /// Each of `defs` as its constructor makes it.
    pub fn build(defs: &[SystemDef], cx: &mut BuildCx) -> Result<Systems> {
        Ok(Systems(defs.iter().map(|d| d.build(cx)).collect::<Result<_>>()?))
    }

    /// The system of `T`'s kind.
    pub fn get<T: SystemKind>(&self) -> Option<&T> {
        self.0.iter().find_map(T::of)
    }

    pub fn get_mut<T: SystemKind>(&mut self) -> Option<&mut T> {
        self.0.iter_mut().find_map(T::of_mut)
    }

    fn of_kind(&self, kind: Kind) -> Option<&System> {
        self.0.iter().find(|s| s.kind() == kind)
    }

    /// The system of `kind` and the others beside it.
    fn split(&mut self, kind: Kind) -> Option<(&mut System, Others<'_>)> {
        let k = self.0.iter().position(|s| s.kind() == kind)?;
        let mut all: Vec<&mut System> = self.0.iter_mut().collect();
        let me = all.remove(k);
        Some((me, Others(all)))
    }

    /// `steps` in order, into `frame` and `events`.
    pub fn run(
        &mut self,
        steps: &'static [Step],
        core: &mut AreaCore,
        view: &TownView,
        frame: &mut Vec<Drawn>,
        events: &mut Vec<TownEvent>,
    ) {
        for &s in steps {
            match s {
                Step::Map => frame.push(Drawn::Map),
                Step::Seq(inner) => self.run(inner, core, view, frame, events),
                Step::Run(kind, hook) => {
                    let Some((me, mut others)) = self.split(kind) else { continue };
                    let mut cx = StepCx { core, view, frame, events };
                    me.step(hook, &mut cx, &mut others);
                }
            }
        }
    }

    /// The frame's pieces as requests, in its order.
    pub fn requests<'a>(&'a self, frame: &[Drawn], cx: &DrawCx<'a>) -> Vec<Request<'a>> {
        frame
            .iter()
            .filter_map(|d| match *d {
                Drawn::Piece(kind, piece) => self.of_kind(kind)?.draw(piece, cx),
                Drawn::Map => None,
            })
            .collect()
    }

    /// The statics each system holds, as the game holds them.
    pub fn take(&mut self, s: &Statics) {
        self.0.iter_mut().for_each(|x| x.take(s));
    }

    pub fn put(&self, s: &mut Statics) {
        self.0.iter().for_each(|x| x.put(s));
    }
}

/// The systems beside the one stepping.
pub struct Others<'a>(Vec<&'a mut System>);

impl Others<'_> {
    /// The sibling of `T`'s kind.
    pub fn get_mut<T: SystemKind>(&mut self) -> Option<&mut T> {
        self.0.iter_mut().find_map(|s| T::of_mut(s))
    }
}
