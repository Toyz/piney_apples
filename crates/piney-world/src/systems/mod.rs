//! A map's systems and the one runner that steps and draws them. A def
//! (`crate::town::TownDef`, `crate::story_map::StoryMapDef`) names its
//! systems ([`SystemDef`], each built from its own data) and its `Draw`'s
//! [`Step`]s in the game's order. A step puts what it shows into the frame
//! ([`Drawn`]); after the frame each system turns its pieces into generic
//! draw [`Request`]s, which the world paints the same way for every map
//! ([`paint`]). Siblings reach each other only by kind ([`Others`]);
//! carried state goes through each system's `take` / `put` ([`Carried`]).

pub mod anim;
pub mod bird;
pub mod bob;
pub mod clouds;
pub mod fireflies;
pub mod fires;
pub mod flare;
pub mod haze;
pub mod lake;
pub mod rain;
pub mod scrolls;
pub mod sky;
pub mod smoke;
pub mod snow;
pub mod statics;
pub mod steam;
pub mod tobj;
pub mod water;

use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use glam::Mat4;
use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::classes::GameClass;
use piney_data::dungeon::Rng;
use piney_data::save::SaveData;
use piney_data::statics::DrawPass;
use piney_data::volume::Volume;
use piney_desktop::assets::SceneFile;
use piney_desktop::layers::Layers;

use crate::area::AreaCore;
use crate::chara::pose::Play;
use crate::draw::{self, Draw, Fogging};
use crate::ee::{F, V4};
use crate::field::ambient::Weather;
use crate::field::area::FieldArea;
use crate::field::def::FieldDef;
use crate::hit::HitModel;
use crate::sky::lensflare::FlareCamera;
use crate::sprite::EffSprite;
use crate::story_map::StoryFrame;
use crate::town::{anm_pieces, object_morph};

/// The executable's scroll statics (`crate::town::scroll_statics`).
pub type Statics = [u32; piney_desktop::SCROLL_STATICS];
/// gcmn's `EA_moveTex` scrolls, (u, v) a row (`SaveState::move_tex`).
pub type MoveTexRows = [[u32; 2]; piney_desktop::MOVE_TEX_ROWS];

/// The field drawing's function statics (`SaveState::field_ambient`):
/// `DrawRain`'s, `DrawSteam`'s and TOBJ's, as words.
pub type FieldStatics = [u32; piney_desktop::FIELD_AMBIENT];

/// What outlives a scene that a system takes as it is made: the statics,
/// gcmn's scrolls for a map made anew (a kept map has its own), and in a
/// field its drawing's statics.
pub struct Carried<'a> {
    pub statics: &'a Statics,
    pub move_tex: Option<&'a MoveTexRows>,
    pub field: Option<&'a FieldStatics>,
}

/// Where a system puts what outlives the scene.
pub struct CarriedMut<'a> {
    pub statics: &'a mut Statics,
    pub move_tex: Option<&'a mut MoveTexRows>,
    pub field: Option<&'a mut FieldStatics>,
}

/// `effSmoke(pos, v, s, life, t, in, out)` (main 0x001ce300): a puff of
/// smoke faded by its distance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Smoke {
    pub pos: V4,
    pub v: V4,
    pub s: F,
    pub life: i32,
    pub t: i32,
    pub fade_in: u16,
    pub fade_out: u16,
}

/// What a map's `Draw` starts besides its drawing, for the host: effects,
/// sounds and the screen's flash, in order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MapEvent {
    /// `effSmokeN(pos, v, scale, life, kind, fade in, fade out)`: a puff of
    /// smoke not faded by distance (the particle's `distSW` cleared).
    SmokeN {
        pos: V4,
        v: V4,
        scale: F,
        life: i32,
        kind: i32,
        fade: (i16, i16),
    },
    /// `effSmoke`.
    Smoke(Smoke),
    /// `ccSeOn(n)` and `ccSeOn3D(n, pos)`.
    Se(i32),
    Se3d {
        n: i32,
        pos: V4,
    },
    /// `tobjSeLoopStart(pos)`: TOBJ's hum started, silent.
    SeLoopStart,
    /// `tobjSeLoop(pos, rate)`: TOBJ's hum (`seData[44]`) at its place, its
    /// volume scaled by its transparency.
    SeLoop(V4, F),
    /// `scFadeDef->EntryFlash(frames, colour, x, y, w, h)`.
    Flash {
        frames: i32,
        colour: u32,
        rect: [F; 4],
    },
}

/// What a map's `Draw` reads of the rest of the field.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapView {
    /// `cameraGetPos(camID)`.
    pub eye: V4,
    /// The player's `pos` (`plw`): the clouds gather about it and
    /// `ccCheckCameraDeg` and `W2P` measure from it.
    pub player: V4,
    /// `activeCamPtr`, which `ccCheckCameraDeg` reads.
    pub cam: crate::camera::Cam,
    /// What `LENSFLARE::Draw` reads of the camera.
    pub flare: FlareCamera,
    /// `eventMng.puppetShow` (+0x78c).
    pub puppet_show: bool,
    /// The field view's `world_screen` as the camera left it, which
    /// `waterUVModifi2` projects the water through (sysLayer's view +0xd0).
    pub world_screen: [V4; 4],
    /// `cameraGetPos(1)`, `cameraGetRot(1)` and `cameraGetRot2(1)`, which
    /// a field's rain, thunder and heat haze read.
    pub eye1: V4,
    pub rot: V4,
    pub rot2: V4,
    /// `ccSys` +0x358 & 1: the frame counter's low bit.
    pub odd: bool,
}

impl MapView {
    /// A view with only the camera eye (all Mac Anu's `Draw` reads).
    pub fn at(eye: V4) -> MapView {
        MapView { eye, ..MapView::default() }
    }

    /// `checkCameraType()` 1: the eye view, which turns by
    /// `cameraGetRot2(1)` in place of `cameraGetRot(1)`.
    pub fn eye_view(&self) -> bool {
        self.cam.kind == crate::camera::kind::EYE
    }

    /// The camera's turn as the eye view picks it.
    pub fn cam_rot(&self) -> V4 {
        if self.eye_view() { self.rot2 } else { self.rot }
    }
}

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
    /// A frame the tasks sleep through: what stands, unstepped, on a
    /// layer (`EVENTAREAB8`'s).
    Still(i16),
    /// A frame the tasks sleep through in a field or dungeon: the last
    /// frame's pieces drawn again, what samples the picture under it
    /// brought up to the camera.
    Again,
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
    /// A static model row, with the area's fog or without; on a layer of
    /// the step's choosing.
    Model(usize),
    ModelNoFog(usize),
    ModelOn(usize, i16),
    /// A static object on a layer of the step's choosing, as it stands.
    ObjectOn(usize, i16),
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
    /// The system's `k`-th draw of this frame, as it listed it.
    Item(usize),
    /// A firefly's `ccEff::Draw` as its step made it.
    Fly(crate::sky::firefly::FireflyDraw),
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
    /// with the scrolled materials' offsets `rows`, `alpha` opaque.
    Models {
        file: &'a SceneFile,
        models: Cow<'a, [u32]>,
        world: Mat4,
        layer: i16,
        rows: HashMap<u32, [u8; 2]>,
        fog: Fogging,
        alpha: f32,
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
    Sprite(EffSprite),
    /// `ccAnm::Draw` of a field's or dungeon's animation: each posed
    /// object's drawn model at `alpha` (a bone or skin model on its clump's
    /// nodes as the animation poses them), `rows` over the materials, one
    /// model with its vertex edits; and TOBJ's shadow, `ccShadowModel::Draw`
    /// with `WORLD_MAN` +0x414's packet, its length and transparency.
    Posed {
        file: &'a SceneFile,
        play: &'a Play,
        root: Mat4,
        layer: i16,
        rows: Cow<'a, HashMap<u32, [u8; 2]>>,
        alpha: f32,
        edit: Option<(u32, Arc<piney_draw::VertexEdits>)>,
        shadow: Option<(F, u8)>,
    },
    /// A `ccMask`'s puffs (`MakePacket(0, 1)` each: a 512 x 512 sprite of
    /// the 128 x 128 texture, the colour's alpha 0x60 by the puff's
    /// transparency), then one `SendPacketS`: each (transparency, x, y).
    Masks { layer: i16, tex: &'a piney_desktop::message::WindowTexture, puffs: &'a [[F; 3]] },
}

/// `r` into the layers; a sprite goes to `sprites` for the host.
pub fn paint(layers: &mut Layers, to_screen: Mat4, r: &Request, sprites: &mut Vec<EffSprite>) {
    match r {
        Request::Models { file, models, world, layer, rows, fog, alpha } => {
            for &model in models.iter() {
                let d = Draw {
                    file,
                    model,
                    world: *world,
                    alpha: *alpha,
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
        Request::Posed { file, play, root, layer, rows, alpha, edit, shadow } => {
            for inst in play.instances(file, *root) {
                let Some((model, _)) = file.drawn_model(inst.object) else { continue };
                let e = edit.as_ref().filter(|(m, _)| *m == model).map(|(_, e)| e);
                let nodes = skin_nodes(file, play, *root, inst.object, model);
                let d = Draw {
                    file,
                    model,
                    world: inst.world,
                    alpha: *alpha,
                    rows,
                    lights: None,
                    nodes: &nodes,
                    morph: Vec::new(),
                    clut_swaps: Vec::new(),
                };
                draw::model_edited(layers, *layer, to_screen, &d, e, Fogging::None);
            }
            if let Some((length, shadow_alpha)) = *shadow {
                let back = layers.shadows.active;
                layers.shadows.active = back.map(|_| piney_desktop::shadow::TOBJ_PACKET);
                layers.shadows.length = crate::ee::f(length);
                layers.shadows.alpha = shadow_alpha;
                let objects = play.instances(file, *root).into_iter().map(|i| (i.object, i.world));
                draw::cast_shadows(layers, file, objects);
                layers.shadows.active = back;
            }
        }
        Request::Masks { layer, tex, puffs } if !puffs.is_empty() => {
            let mut sp = piney_desktop::sprite::Sprite::mask(*layer, tex.tex.clone(), tex.tex_h, 64);
            sp.colour = [0x80, 0x80, 0x80, 0x60];
            for &[alpha, x, y] in puffs.iter() {
                sp.transp = crate::ee::f(alpha);
                sp.cx = 0.0;
                sp.cy = 0.0;
                sp.dx = crate::ee::f(x);
                sp.dy = crate::ee::f(y);
                sp.sx = 512.0;
                sp.sy = 512.0;
                sp.su = 128;
                sp.sv = 128;
                sp.wu = 0;
                sp.wv = 0;
                sp.wi = 1;
                sp.make_packet(0, &piney_desktop::view::LayerView::default_layer());
            }
            sp.send(layers);
        }
        Request::Masks { .. } => {}
    }
}

/// A bone or skin model's node matrices: the nodes of its object's clump
/// as `play` poses them under `root`, for `SetAnm` builds an anm-made
/// model's node table from the anm's own objects (main 0x00151950); none
/// for any other model.
pub(crate) fn skin_nodes(file: &SceneFile, play: &Play, root: Mat4, obj: u32, model: u32) -> Vec<Mat4> {
    let skinned = file.models.get(&model).is_some_and(|i| i.mtype & 6 != 0);
    let Some(nodes) = file.scene.clump_of(obj).filter(|_| skinned) else { return Vec::new() };
    let nodes: Vec<u32> = nodes.iter().map(|n| file.scene.ext.get(n).copied().unwrap_or(*n)).collect();
    let worlds = play.worlds(file, root, &nodes);
    nodes.iter().map(|n| worlds.get(n).copied().unwrap_or(root)).collect()
}

/// `ccRand` for a shorter while (a trait object's lifetime does not
/// shorten inside an `Option` by itself).
pub fn shorten<'s>(
    cc: Option<&'s mut (dyn piney_battle::rand::Rng + '_)>,
) -> Option<&'s mut dyn piney_battle::rand::Rng> {
    match cc {
        Some(cc) => Some(cc),
        None => None,
    }
}

/// What a system's constructor reads: the disc, the file its names are
/// in (the area's, or a block's own), the save's crisis byte and
/// `fieldrand` (a town's constructor draws on a copy: `GO(0)` hands the
/// town the stream it was given; a story map's draws are kept); for a
/// story map, what `GO(1)` made it for and the game's `ccRand`.
pub struct BuildCx<'a> {
    pub archive: &'a Arc<Archive>,
    /// Whose tables the statics read, and the disc's own.
    pub tables: Volume,
    pub volume: Volume,
    pub file: &'a Rc<SceneFile>,
    /// The def's file by name, for a sprite of it.
    pub stem: &'static str,
    pub crisis: bool,
    pub rng: Rng,
    pub map: Option<MapCx<'a>>,
    pub cc: Option<&'a mut dyn piney_battle::rand::Rng>,
    /// In a field or dungeon, what `WORLD_MAN` holds of the area.
    pub field: Option<FieldCx<'a>>,
    /// What the constructors started besides their drawing (TOBJ::Init's
    /// `tobjSeLoopStart`), for the host once the area is up.
    pub made: Vec<MapEvent>,
}

/// What a field's or dungeon's systems are made from beyond the map's: the
/// field type's def, the area's weather row and hack flag, `ccGame.server`,
/// and in a field the field itself as `Init` and `Generate` left it, with
/// `WORLD_MAN`'s centre as `Init` finds it.
#[derive(Clone, Copy)]
pub struct FieldCx<'a> {
    pub def: &'static FieldDef,
    pub weather: Weather,
    pub server: i32,
    pub field: Option<&'a FieldArea>,
    pub centre: V4,
}

/// What a story map is made for: `game.field`, `game.fieldPrev`,
/// `game.server`, the save, and the stage its class's names are written
/// for (`EVENTAREAB0`'s `se1_5`, read as the file's).
#[derive(Clone, Copy)]
pub struct MapCx<'a> {
    pub field: i32,
    pub field_prev: i32,
    pub server: i32,
    /// The save, as the constructor reads it (a later `ChangeBlock` has none).
    pub save: Option<&'a SaveData>,
    pub stage: Option<&'static str>,
}

impl BuildCx<'_> {
    /// `name` as the file has it: the class's stage written over with the
    /// file's own.
    pub fn name<'n>(&self, name: &'n str) -> Cow<'n, str> {
        match self.map.and_then(|m| m.stage) {
            Some(from) if name.contains(from) => Cow::Owned(name.replace(from, &self.file.stem)),
            _ => Cow::Borrowed(name),
        }
    }

    /// `game.field` (-1 in a town).
    pub fn field(&self) -> i32 {
        self.map.map_or(-1, |m| m.field)
    }
}

/// What a block's systems do once they are all made (`ChangeBlock`'s
/// tail): the area's core (its collision list, start, centre), the
/// block's file's Hit chunks, `fieldrand`.
pub struct PlaceCx<'a> {
    pub core: &'a mut AreaCore,
    pub hit_models: &'a [HitModel],
    pub rng: &'a mut Rng,
}

/// What the battle asks of `WORLD_MAN.eventmap`, for whichever of its
/// systems answers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
    /// `SwitchLayer`: the reflection onto objLayer and back.
    SwitchLayer,
    /// The last stage's clump on or off (QUA gcmn 0x0030aa40).
    StageClump(bool),
    /// `EVENTAREAB8::Move`: the disc on to its next stop.
    DiscMove,
    /// `body[n]`'s `HitEnable` / `HitDisable`.
    BodyHit(usize, bool),
}

/// What a map's systems tell the battle each frame.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// `EVENTAREAB8`'s disc as Kyvia reads it.
    pub disc: Option<piney_battle::boss::DiscView>,
}

/// What a step reads and writes: the area's core (its file, `fieldrand`,
/// the volume its collision takes), the view, the frame so far, the
/// effects and sounds started, and the game's `ccRand` where a map's
/// `Draw` draws from it (`EVENTAREAB8`'s rocks).
pub struct StepCx<'a> {
    pub core: &'a mut AreaCore,
    pub view: &'a MapView,
    pub frame: &'a mut Vec<Drawn>,
    pub events: &'a mut Vec<MapEvent>,
    pub cc: Option<&'a mut dyn piney_battle::rand::Rng>,
}

impl StepCx<'_> {
    /// `piece` in the frame, drawn by `kind`.
    pub fn put(&mut self, kind: Kind, piece: Piece) {
        self.frame.push(Drawn::Piece(kind, piece));
    }
}

/// What a draw reads: the area's core, its fog as VU1 puts it on the
/// buildings (`SetFog`), and the offsets the scrolls last wrote into the
/// file's materials, which every model of the file is drawn with.
pub struct DrawCx<'a> {
    pub core: &'a AreaCore,
    pub fog: Fogging,
    pub rows: &'a HashMap<u32, [u8; 2]>,
}

/// What a system does: its steps, its pieces as requests, what of it
/// carries across visits, and in a story map what it does once its block
/// is made, the scene it runs, the battle's calls it answers and what it
/// tells the battle.
pub(crate) trait MapSystem {
    fn step(&mut self, _hook: Hook, _cx: &mut StepCx, _others: &mut Others) {}

    fn draw<'a>(&'a self, _piece: Piece, _cx: &DrawCx<'a>) -> Option<Request<'a>> {
        None
    }

    /// What it holds of what outlives the scene, as the game holds it.
    fn take(&mut self, _c: &Carried) {}
    fn put(&self, _c: &mut CarriedMut) {}

    fn place(&mut self, _cx: &mut PlaceCx, _others: &mut Others) {}

    fn scene(&mut self, _x: &mut StoryFrame) {}

    fn signal(&mut self, _s: Signal, _cx: &mut PlaceCx) {}

    fn report(&self, _r: &mut Report) {}

    /// The materials' offsets it writes (`MoveTexture`, `DrawBG`).
    fn rows(&self, _file: &SceneFile, _out: &mut HashMap<u32, [u8; 2]>) {}
}

/// A system's own type, found in a list by its kind.
pub trait SystemKind: Sized {
    const KIND: Kind;
    /// Whether its draws take from `fieldrand` (the area then holds it).
    const FIELD_RAND: bool;
    /// The game's class its objects are, if it has one (a system that is
    /// part of a class's `Draw` has none).
    const CLASS: Option<GameClass>;
    fn of(s: &System) -> Option<&Self>;
    fn of_mut(s: &mut System) -> Option<&mut Self>;
}

/// The systems: their objects, their kinds, what a def builds each from,
/// and the one `match` that steps and draws each.
macro_rules! systems {
    ($($(#[$m:meta])* $v:ident($t:ty, $d:ty, $rand:expr, $class:expr)),* $(,)?) => {
        /// One of a map's systems.
        #[derive(Clone)]
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

        impl Kind {
            /// The game's class its system's objects are.
            pub fn class(self) -> Option<GameClass> {
                match self {
                    $(Kind::$v => <$t as SystemKind>::CLASS,)*
                }
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

            fn take(&mut self, c: &Carried) {
                match self {
                    $(System::$v(x) => MapSystem::take(&mut **x, c),)*
                }
            }

            fn put(&self, c: &mut CarriedMut) {
                match self {
                    $(System::$v(x) => MapSystem::put(&**x, c),)*
                }
            }

            fn place(&mut self, cx: &mut PlaceCx, others: &mut Others) {
                match self {
                    $(System::$v(x) => MapSystem::place(&mut **x, cx, others),)*
                }
            }

            fn scene(&mut self, sx: &mut StoryFrame) {
                match self {
                    $(System::$v(x) => MapSystem::scene(&mut **x, sx),)*
                }
            }

            fn signal(&mut self, s: Signal, cx: &mut PlaceCx) {
                match self {
                    $(System::$v(x) => MapSystem::signal(&mut **x, s, cx),)*
                }
            }

            fn report(&self, r: &mut Report) {
                match self {
                    $(System::$v(x) => MapSystem::report(&**x, r),)*
                }
            }

            fn rows(&self, file: &SceneFile, out: &mut HashMap<u32, [u8; 2]>) {
                match self {
                    $(System::$v(x) => MapSystem::rows(&**x, file, out),)*
                }
            }
        }

        $(impl SystemKind for $t {
            const KIND: Kind = Kind::$v;
            const FIELD_RAND: bool = $rand;
            const CLASS: Option<GameClass> = $class;

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
    Models(statics::StaticModels, statics::ModelsDef, false, Some(GameClass::Staticmodel)),
    /// `STATICOBJECT`s: an object table's animated objects.
    Objects(statics::StaticObjects, statics::ObjectsDef, false, Some(GameClass::Staticobject)),
    /// `DrawBG`'s scrolls, carried in the executable's statics.
    Scrolls(scrolls::Scrolls, scrolls::ScrollsDef, false, None),
    /// `DrawBG`'s clumps, the crisis sky's among them.
    Sky(sky::Sky, sky::SkyDef, false, None),
    /// A town's water: `waterAnm[0..2]`, water 0 through the picture.
    Water(water::Water, water::WaterDef, false, None),
    /// `CLOUD`s.
    Clouds(clouds::Clouds, clouds::CloudsDef, true, Some(GameClass::Cloud)),
    /// `LENSFLARE`, or a class's own copy of its drawing.
    LensFlare(flare::LensFlare, flare::FlareDef, false, Some(GameClass::Lensflare)),
    /// `DrawBG`'s bob (`EVENTAREA07`'s sky).
    Bob(bob::Bob, bob::BobDef, true, None),
    /// A `ccAnm` the class steps and draws at a root of its own.
    Anim(anim::Anim, anim::AnimDef, false, Some(GameClass::CcAnm)),
    /// `FIREFLY2`s about the file's markers, or roaming about the camera.
    Firefly2(fireflies::Fireflies2, fireflies::Firefly2Def, true, Some(GameClass::Firefly2)),
    /// `FIREFLY`s: a field's by night or hacked, a lake room's.
    Fireflies(fireflies::Fireflies, fireflies::FirefliesDef, true, Some(GameClass::Firefly)),
    /// `SNOW`: a field's snowflakes, or field type 0's embers.
    Snow(snow::Snow, snow::SnowDef, true, Some(GameClass::Snow)),
    /// A field's 2D smoke (`WORLD`'s `ccMask`).
    Smoke(smoke::Smoke2d, (), true, None),
    /// `WORLD::DrawSteam`: field type 0's steam vents.
    Steam(steam::Steam, (), true, None),
    /// `WORLD::Draw`'s heat haze.
    Haze(haze::Haze, haze::HazeDef, false, None),
    /// `WORLD::SetLake`'s water.
    Lake(lake::Lake, (), false, None),
    /// `DrawRain`'s drops.
    Rain(rain::Rain, (), true, None),
    /// `DrawRain`'s storm: the lightning, the flash, the thunder.
    Thunder(rain::Thunder, (), true, None),
    /// `DrawEffect`'s smoke over field type 7's fires.
    Fires(fires::Fires, (), false, None),
    /// `TOBJ`: the server's traveller.
    Tobj(tobj::Tobj, (), true, Some(GameClass::Tobj)),
    /// `BIRD`: field type 10's hawk.
    Bird(bird::Bird, (), false, Some(GameClass::Bird)),
    /// Mac Anu's own: its camera-eye clip of the rows and objects.
    MacAnu(crate::town::mac_anu::Clip, (), false, None),
    /// Carmina Gade's own: its `AIRSHIP`.
    Airship(crate::town::carmina_gadelica::Airship, (), true, Some(GameClass::Airship)),
    /// Lia Fail's own: its omni points' glows.
    Glows(crate::town::lia_fail::Glows, (), true, None),
    /// `EVENTAREA01`'s own: its flickering glows.
    Flicker(crate::story_map::truths_core::Flicker, (), true, None),
    /// `EVENTAREA03`'s own: the fly-over while area 43 is unmarked.
    FlyOver(crate::story_map::abyss::FlyOver, (), false, None),
    /// The ending's own: the party's panels out.
    Panels(crate::story_map::ending::Panels, (), false, None),
    /// `EVENTAREAB0`'s own: the reflection `SwitchLayer` moves.
    Reflection(crate::story_map::boss_arena::Reflection, (), false, None),
    /// `EVENTAREAB0`'s own: the three models that bob.
    Bobbing(crate::story_map::boss_arena::Bobbing, (), true, None),
    /// `EVENTAREAB0`'s own: field 8's last stage.
    LastStage(crate::story_map::boss_arena::LastStage, (), false, None),
    /// `EVENTAREAB8`'s own: the disc and the path it carries the party on.
    Disc(crate::story_map::disc::Disc, crate::story_map::disc::DiscDef, false, None),
    /// `EVENTAREAB8`'s own: its `FLOATROCK`s.
    Rocks(crate::story_map::disc::Rocks, (), true, None),
    /// `EVENTAREAB8`'s own: field 12's three pieces and their animation.
    Bodies(crate::story_map::disc::Bodies, (), false, None),
}

/// A map's systems: a block's (`ChangeBlock` makes them anew) before the
/// map's own, each list in its def's order.
#[derive(Clone, Default)]
pub struct Systems {
    all: Vec<System>,
    block: usize,
}

impl Systems {
    /// Each of `defs` as its constructor makes it.
    pub fn build(defs: &[SystemDef], cx: &mut BuildCx) -> Result<Systems> {
        Ok(Systems { all: defs.iter().map(|d| d.build(cx)).collect::<Result<_>>()?, block: 0 })
    }

    /// Each of `defs` made after those already made.
    pub fn extend(&mut self, defs: &[SystemDef], cx: &mut BuildCx) -> Result<()> {
        for d in defs {
            self.all.push(d.build(cx)?);
        }
        Ok(())
    }

    /// The block's systems made anew from `defs`, before the map's.
    pub fn build_block(&mut self, defs: &[SystemDef], cx: &mut BuildCx) -> Result<()> {
        let made = defs.iter().map(|d| d.build(cx)).collect::<Result<Vec<_>>>()?;
        self.all.splice(..self.block, made);
        self.block = defs.len();
        Ok(())
    }

    /// Each system placed once its block is made, in order ([`PlaceCx`]).
    pub fn place(&mut self, cx: &mut PlaceCx) {
        for k in 0..self.all.len() {
            let (me, mut others) = self.split_at(k);
            me.place(cx, &mut others);
        }
    }

    /// What the scene runs (`StoryMap`'s own scene), each system in turn.
    pub fn scene(&mut self, x: &mut StoryFrame) {
        self.all.iter_mut().for_each(|s| s.scene(x));
    }

    /// The battle's call, to each system.
    pub fn signal(&mut self, s: Signal, cx: &mut PlaceCx) {
        self.all.iter_mut().for_each(|x| x.signal(s, cx));
    }

    /// What the systems tell the battle.
    pub fn report(&self) -> Report {
        let mut r = Report::default();
        self.all.iter().for_each(|s| s.report(&mut r));
        r
    }

    /// The offsets the scrolls last wrote into `file`'s materials.
    pub fn rows(&self, file: &SceneFile) -> HashMap<u32, [u8; 2]> {
        let mut out = HashMap::new();
        self.all.iter().for_each(|s| s.rows(file, &mut out));
        out
    }

    /// The system of `T`'s kind.
    pub fn get<T: SystemKind>(&self) -> Option<&T> {
        self.all.iter().find_map(T::of)
    }

    pub fn get_mut<T: SystemKind>(&mut self) -> Option<&mut T> {
        self.all.iter_mut().find_map(T::of_mut)
    }

    fn of_kind(&self, kind: Kind) -> Option<&System> {
        self.all.iter().find(|s| s.kind() == kind)
    }

    /// The system of `kind` and the others beside it.
    fn split(&mut self, kind: Kind) -> Option<(&mut System, Others<'_>)> {
        let k = self.all.iter().position(|s| s.kind() == kind)?;
        Some(self.split_at(k))
    }

    fn split_at(&mut self, k: usize) -> (&mut System, Others<'_>) {
        let mut all: Vec<&mut System> = self.all.iter_mut().collect();
        let me = all.remove(k);
        (me, Others(all))
    }

    /// `steps` in order, into `frame` and `events`.
    pub fn run(
        &mut self,
        steps: &'static [Step],
        core: &mut AreaCore,
        view: &MapView,
        frame: &mut Vec<Drawn>,
        events: &mut Vec<MapEvent>,
    ) {
        self.run_rand(steps, core, view, frame, events, None);
    }

    /// [`Systems::run`] with the game's `ccRand` for a `Draw` that draws
    /// from it.
    pub fn run_rand(
        &mut self,
        steps: &'static [Step],
        core: &mut AreaCore,
        view: &MapView,
        frame: &mut Vec<Drawn>,
        events: &mut Vec<MapEvent>,
        mut cc: Option<&mut (dyn piney_battle::rand::Rng + '_)>,
    ) {
        for &s in steps {
            match s {
                Step::Map => frame.push(Drawn::Map),
                Step::Seq(inner) => self.run_rand(inner, core, view, frame, events, cc.as_deref_mut()),
                Step::Run(kind, hook) => {
                    let Some((me, mut others)) = self.split(kind) else { continue };
                    let mut cx = StepCx { core, view, frame, events, cc: shorten(cc.as_deref_mut()) };
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

    /// What outlives the scene, into each system as the game holds it.
    pub fn take(&mut self, c: &Carried) {
        self.all.iter_mut().for_each(|x| x.take(c));
    }

    pub fn put(&self, c: &mut CarriedMut) {
        self.all.iter().for_each(|x| x.put(c));
    }
}

/// The systems beside the one stepping.
pub struct Others<'a>(Vec<&'a mut System>);

impl Others<'_> {
    /// The sibling of `T`'s kind.
    pub fn get_mut<T: SystemKind>(&mut self) -> Option<&mut T> {
        self.0.iter_mut().find_map(|s| T::of_mut(s))
    }

    pub fn get<T: SystemKind>(&self) -> Option<&T> {
        self.0.iter().find_map(|s| T::of(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Issue #37: Cursed Despaired Paradise's dungeon mouth (field type 8,
    /// `ANM_sfk1sto1a`) has two hands, bone and skin models over the
    /// clump's finger nodes. Drawn with no node matrices they fell to the
    /// mouth's origin as a pink block; they take their clump's nodes as
    /// the anm poses them, out at the mouth's sides.
    #[test]
    fn the_mouths_hands_take_their_bones() {
        let Some(archive) = crate::town::tests::archive() else { return };
        let file = SceneFile::read(&archive, crate::field::ambient::field_file(Volume::Inf, 8)).unwrap();
        let mut play = Play::new(&file, "ANM_sfk1sto1a").unwrap();
        play.forward(&file);
        let obj = |n: &str| file.ccs.find_object(n).unwrap();
        // The objects by their models (an animation's ExtObj copies share
        // the names).
        let owner = |m: &str| (file.scene.model_owner[&obj(m)], obj(m));
        let (hand, model) = owner("MDL_sfk1sto1_a");
        let bones = skin_nodes(&file, &play, Mat4::IDENTITY, hand, model);
        assert_eq!(bones.len(), 49, "CMP_sfk1sto1's nodes");
        let nodes = file.scene.clump_of(hand).unwrap();
        let finger = nodes.iter().position(|&n| file.ccs.object_name(n) == Some("OBJ_l index finger 01")).unwrap();
        assert!(bones[finger].w_axis.x > 500.0, "the left hand out at the side: {:?}", bones[finger].w_axis);
        let (head, model) = owner("MDL_sfk1sto1");
        assert!(skin_nodes(&file, &play, Mat4::IDENTITY, head, model).is_empty(), "rigid");
    }
}
