//! A scene: one stream file as `ccStream::InitScene` (0x00148640) builds it and
//! `ccStream::PlaySceneMain` (0x001494c0) plays it through `DecodeFrameSection`
//! (0x0014ded0) and `DecodeFrameChunk` (0x0014e1b0) (`docs/engine/stream.md`,
//! "A scene"). One `ccObj` per node of its Clump chunks, an ExtObj followed to
//! its target across files (`ccGetExternalIndex` 0x00101a50) and a `#` entry
//! resolved to the latest loaded file's; shadow models are taken out. Every
//! node starts at transparency 0 until an `F_Obj` places it. The first pass
//! reads frames 0 and 1; the end Top (-1) ends the scene, -2 also rewinds it.

use std::collections::{HashMap, HashSet};

use glam::Vec3;
use piney_data::anim::{const_radians, ee, rot_bits_of};
use piney_desktop::camera::{Camera, DEFAULT_FOV};
use piney_desktop::view::Frame;

use crate::file::{self, StreamFile};

/// `1.0` as bits.
const ONE: u32 = 0x3f80_0000;
/// The identity, as stored columns of float bits.
pub const IDENTITY: [[u32; 4]; 4] = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];

/// `ccAnmNote.event` values the stream acts on (`ccSetStreamDemoNote`
/// 0x00184340, `ccSndStreamSE` 0x0017caa0).
pub const NOTE_BGM: u32 = 4;
pub const NOTE_DEMO: u32 = 0x8010;
pub const NOTE_MSG: u32 = 0x8001;
pub const NOTE_MSG_OFF: u32 = 0x8011;

/// Every file the stream has loaded so far, in load order (`ccscRoot`, read
/// newest first).
#[derive(Default)]
pub struct Loaded {
    pub files: Vec<StreamFile>,
    /// The PCM track the decoders keep: None on Infection and Mutation
    /// (one track; the voice picks the archive), else the save's voice
    /// byte (Outbreak on: each Pcm chunk and F_Pcm record carries its
    /// language, and `ccPcmSound::Open` is given the voice byte).
    pub pcm_track: Option<u8>,
}

impl Loaded {
    /// Object `obj` of file `f`, or for a `#` entry the same-named object
    /// another loaded file defines, the newest first.
    pub fn resolve(&self, f: usize, obj: u32) -> Option<(usize, u32)> {
        let file = self.files.get(f)?;
        if obj == 0 {
            return None;
        }
        if !file.external(obj) {
            return Some((f, obj));
        }
        let name = file.name(obj)?;
        for (g, other) in self.files.iter().enumerate().rev() {
            if g == f {
                continue;
            }
            // An ExtObj's entry was renamed EXT_ by its decoder: no match.
            if let Some(o) = other.sf.ccs.find_object(name)
                && !other.external(o)
                && other.setup.kind.get(&o).is_some_and(|&k| k != file::EXT_OBJ)
            {
                return Some((g, o));
            }
        }
        None
    }

    /// `ccGetExternalIndex`: the chunk an object finally stands for, ExtObj
    /// targets followed: (file, object, chunk kind).
    pub fn follow(&self, f: usize, obj: u32) -> Option<(usize, u32, u16)> {
        let (mut f, mut o) = self.resolve(f, obj)?;
        for _ in 0..64 {
            let k = *self.files[f].setup.kind.get(&o)?;
            if k != file::EXT_OBJ {
                return Some((f, o, k));
            }
            let t = self.files[f].setup.exts[&o].target;
            (f, o) = self.resolve(f, t)?;
        }
        None
    }

    /// The model an Obj draws, resolved: (file, MDL_ object).
    pub fn obj_model(&self, f: usize, obj: u32) -> Option<(usize, u32)> {
        let def = self.files[f].setup.objs.get(&obj)?;
        self.resolve(f, def.model)
    }

    /// A model's `mtype`.
    pub fn mtype(&self, f: usize, model: u32) -> Option<u16> {
        self.files[f].sf.models.get(&model).map(|m| m.mtype)
    }
}

/// One `ccObj` of the scene: a clump node.
#[derive(Clone, Debug)]
pub struct Node {
    /// The node's object in the scene file: what `F_Obj` records name.
    pub obj: u32,
    pub clump: usize,
    /// The node it hangs from; None: the clump's own coordinate (never
    /// moved, the identity).
    pub parent: Option<usize>,
    /// (file, MDL_) drawn.
    pub model: Option<(usize, u32)>,
    /// ccCoord.flag bit 0: transparency is the parent's times its own.
    pub succession: bool,
    /// ccObj.partFlag: a record at the origin hides it.
    pub part: bool,
    /// The layer it draws on.
    pub layer: i16,
    /// `ccCoord.matrix` as float bits (stored columns).
    pub local: [[u32; 4]; 4],
    pub localtp: f32,
    pub worldtp: f32,
    /// `ccObj.dispSW`: drawn when 3.
    pub disp: u8,
    /// The Obj2 record's modifier (an `MPH_` morpher, whose weights
    /// `F_Morpher` sets); 0: none.
    pub modifier: u32,
    /// Its object's shadow model (`ccObj` +0x9c): (file, MDL_).
    pub shadow_model: Option<(usize, u32)>,
    /// The draw list entry's packet ([`Scene::shadows`]): its Obj2's shadow
    /// layer's, else the default one's.
    pub shadow: Option<usize>,
}

/// A shadow layer's `ccShadowPacket` (0x180 bytes) as `InitScene` makes it
/// (`docs/engine/shadow.md`): mode 1, darkness 0x20, its chunk's buffer;
/// `F_Shadow` sets its light and alpha.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneShadow {
    /// The shadow layer's priority.
    pub layer: i16,
    pub width: u16,
    pub height: u16,
    /// +0x15e, +0x15f: the chunk's.
    pub passes: u8,
    pub spread: u8,
    /// +0x160: the light's direction, (0, 0, 0, 1) until an `F_Shadow`.
    pub light: [f32; 4],
    /// +0x170: the Shadow chunk's length.
    pub length: f32,
    /// +0x174: the alpha as a fraction (0 until an `F_Shadow`): the draw
    /// environment gets `(fptosi(256 a) + 1) >> 1`.
    pub alpha: f32,
}

/// One `ccEffObj` of the scene: a clump node drawing an Eff chunk
/// (`ccEffObj::DrawNoAnm`) at its world place, animated by `F_Obj`.
#[derive(Clone, Debug)]
pub struct EffNode {
    /// The node's object in the scene file: what `F_Obj` records name.
    pub obj: u32,
    pub clump: usize,
    /// The model node it hangs from; None: the clump's own coordinate.
    pub parent: Option<usize>,
    /// (file, EFF_ object) it draws.
    pub eff: (usize, u32),
    pub layer: i16,
    /// `ccCoord.matrix` as float bits (stored columns): a translation only.
    pub local: [[u32; 4]; 4],
    /// +0x88 `localtp`.
    pub localtp: f32,
    /// +0x106: the pattern drawn; 0xffff stopped, 0xfffe just started.
    pub pattern: u16,
    /// +0x104 bit 0 (the Obj2 part bit, `Decode_Eff`'s chunk +0x10 bit 1):
    /// a stopped effect restarts where a record moves it off the origin.
    pub part: bool,
    /// The chunk's `patNum`.
    pub pat_num: u16,
    /// The `ccEff` fields `F_Obj` sets: scale x and y, turn (radians),
    /// transparency (f32 bits).
    pub scale: [u32; 2],
    pub rotate: u32,
    pub transparency: u32,
}

/// `ccEffObj::StartAnm` (0x0013c570)'s pattern: started.
pub const PATTERN_START: u16 = 0xfffe;
/// `ccEffObj::EndAnm` (0x0013c580)'s: stopped.
pub const PATTERN_STOP: u16 = 0xffff;

impl EffNode {
    /// `ccEffObj::EndAnm`: `localtp` 0, the pattern stopped.
    fn end_anm(&mut self) {
        self.localtp = 0.0;
        self.transparency = 0;
        self.pattern = PATTERN_STOP;
    }

    /// `ccEffObj::Animate(n)` (0x0013c510): a stopped effect stays so; one
    /// just started takes pattern `n - 1`, else it moves on `n`; past the
    /// last pattern it stops.
    fn animate(&mut self, n: u16) {
        self.pattern = match self.pattern {
            PATTERN_STOP => return,
            PATTERN_START => n.wrapping_sub(1),
            p => p.wrapping_add(n),
        };
        if self.pattern >= self.pat_num {
            self.pattern = PATTERN_STOP;
        }
    }
}

/// A light of the scene's draw environment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub obj: u32,
    pub kind: i16,
    /// Omni: position. Distant: the rotation in radians (bits).
    pub pos: [f32; 3],
    pub rot: [u32; 3],
    /// Colour (0..1), before the intensity.
    pub colour: [f32; 3],
    pub intensity: f32,
    /// Omni fall-off: full to `far_start`, none past `far_end`; 0 is none.
    pub far_start: f32,
    pub far_end: f32,
}

impl Light {
    /// `ccCreateLight` (0x001389b0)'s priority, as a sort key: a distant
    /// light's -1 after every other kind's 0.
    fn priority(&self) -> u8 {
        u8::from(self.kind == file::LIGHT_DISTANT)
    }

    fn new(obj: u32, kind: i16) -> Self {
        // ccOmniLight::Init / ccDistantLight::Init: black, intensity 1.
        Light { obj, kind, pos: [0.0; 3], rot: [0; 3], colour: [0.0; 3], intensity: 1.0, far_start: 0.0, far_end: 0.0 }
    }
}

/// One note a frame raised (`F_Note`, `ccAnmNote`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Note {
    pub frame: u32,
    pub obj: u32,
    pub event: u32,
    pub param: u32,
}

/// `ccStream.state` bits the stream reads.
pub const STATE_ENDED: u16 = 0x10;

/// A scene playing.
pub struct Scene {
    /// Index of the scene's file in [`Loaded::files`].
    pub file: usize,
    pub nodes: Vec<Node>,
    /// Node object -> node.
    pub by_obj: HashMap<u32, usize>,
    /// A node's object name -> node (`ccStream::GetSubstAdrsF` on the
    /// scene's own objects).
    pub by_name: HashMap<String, usize>,
    /// Each node's object name.
    names: Vec<Option<String>>,
    /// Each clump's nodes, in slot order (None: a node drawn some other
    /// way, an effect): bone and skin mmats' matrices.
    pub clumps: Vec<Vec<Option<usize>>>,
    /// The effect nodes, in clump order ([`EffNode`]).
    pub effs: Vec<EffNode>,
    /// `ccStreamDrawLayerList`: (layer, nodes) in the order `Draw` walks.
    pub draw_list: Vec<(i16, Vec<usize>)>,
    /// The layer the scene draws on by default.
    pub default_layer: i16,
    /// `ccStream.cptr`'s object: the first Camera chunk.
    pub camera_obj: Option<u32>,
    pub camera: Camera,
    /// The camera's position and rotation (radians) as float bits, as
    /// `DecodeF_Camera` left them: [`Scene::world_view_bits`] is built from
    /// them.
    pub camera_bits: ([u32; 3], [u32; 3]),
    /// `ccStream.ambient` (0..1).
    pub ambient: [f32; 3],
    /// The draw environment's light group, in its order.
    pub lights: Vec<Light>,
    /// Material name -> ccMaterial u, v (1/4096) set by `F_Material`: the
    /// scene's Material chunks reach the models' materials of the same name
    /// (InitScene's `modifyMat` links, 0x00148b74).
    pub uv: HashMap<String, [i16; 2]>,
    /// Morpher -> (target model, weight), from `F_Morpher` (not drawn).
    pub morph: HashMap<u32, Vec<(u32, f32)>>,
    /// Not the game's: node -> transparency set from outside
    /// ([`Scene::set_alpha`]), over the scene's own and its `dispSW`.
    pub alpha: HashMap<usize, f32>,
    /// Not the game's: node -> world matrix set from outside
    /// ([`Scene::place`]), over its own and its parents' (its children
    /// follow it).
    pub placed: HashMap<usize, [[u32; 4]; 4]>,
    /// The next record `DecodeFrameChunk` reads.
    cursor: usize,
    /// `frameOffsetTbl`: record index of each frame's Top, once read. Only
    /// a file loaded whole has one (InitScene allocates it when the reader's
    /// ring buffer is in memory mode, 0x00149340); a scene read through the
    /// stream's ring buffer has none, so it neither seeks nor rewinds.
    offsets: Option<Vec<Option<usize>>>,
    pub frame_now: u32,
    /// `frameEnd`: the Frame chunk's count less one.
    pub frame_end: u32,
    pub frame_spd: i16,
    frame_cnt: i16,
    /// `ccStream.ctrl` bit 0: held at the last frame; bit 5: rewind.
    ctrl: u16,
    pub state: u16,
    /// Notes raised since the last take.
    pub notes: Vec<Note>,
    /// PCM blocks (1024 bytes each) read since the last take.
    pub pcm: Vec<Vec<u8>>,
    /// Models (file, MDL_) whose TEST the effect task set to NEVER /
    /// FB_ONLY (`ccObj::SetRenderState(CCRS_ZWRITEENABLE, 0)`): colour
    /// always, no Z.
    pub z_write_off: HashSet<(usize, u32)>,
    /// The frame of the view the scene's layers share, which an effect
    /// task may change (`ccView::SetFrame`; stream 6's letterbox).
    pub frame: Frame,
    /// The draw environment's fog an effect task set (`ccDrawEnv::SetFog`),
    /// on every object but [`Scene::fog_off`]'s.
    pub fog: Option<SceneFog>,
    /// Nodes whose `fogSW` bit 3 the task cleared: never fogged.
    pub fog_off: HashSet<usize>,
    /// The scene view's `divZ` (+0x25c), where VU1 clips: 1000 unless a
    /// task set another (`Stream::div_z`).
    pub div_z: f32,
    /// The shadow packets, and shadow layer object (0 the default) -> packet.
    pub shadows: Vec<SceneShadow>,
    pub shadow_by_obj: HashMap<u32, usize>,
}

/// `ccDrawEnv`'s fog as `SetFog` leaves it: per vertex, VU1's
/// `F = clamp(fogB + fogA w, fMin, fMax)` at view depth `w` (255 clear),
/// towards `FOGCOL`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneFog {
    pub a: f32,
    pub b: f32,
    pub min: f32,
    pub max: f32,
    /// R, G, B (`FOGCOL`: the low three bytes of the colour word, R low).
    pub colour: [u8; 3],
}

impl SceneFog {
    /// `SetFog(near, far, colour)` (0x00105890): clear to `near`, all the
    /// fog colour from `far`.
    pub fn new(near: f32, far: f32, colour: u32) -> SceneFog {
        SceneFog::with_rates(near, far, 0.0, 100.0, colour)
    }

    /// `SetFog(near, far, nearRate, farRate, colour)` (0x00105820): `F`
    /// from `2.55 (100 - nearRate)` at `near` to `2.55 (100 - farRate)` at
    /// `far`, held beyond.
    pub fn with_rates(near: f32, far: f32, near_rate: f32, far_rate: f32, colour: u32) -> SceneFog {
        let min = 2.55 * (100.0 - far_rate);
        let max = 2.55 * (100.0 - near_rate);
        let a = (min - max) / (far - near);
        SceneFog { a, b: max - near * a, min, max, colour: [colour as u8, (colour >> 8) as u8, (colour >> 16) as u8] }
    }

    /// `F` at view depth `w`.
    pub fn value(&self, w: f32) -> u8 {
        (self.b + self.a * w).clamp(self.min.min(self.max), self.max.max(self.min)).clamp(0.0, 255.0) as u8
    }
}

impl Scene {
    /// `InitScene(layer, 0)` on file `f` of `loaded`, drawing by default on
    /// layer `layer`.
    pub fn new(loaded: &Loaded, f: usize, layer: i16) -> Scene {
        let file = &loaded.files[f];
        let s = &file.setup;
        let default_layer = s.layers.as_ref().and_then(|t| t.default).unwrap_or(layer);
        let layer_of =
            |obj: u32| -> i16 { s.layers.as_ref().and_then(|t| t.layers.get(&obj).copied()).unwrap_or(default_layer) };
        // InitScene's packets: the default shadow layer's and each shadow
        // layer's with a Shadow chunk (only with a Layer chunk).
        let mut shadows = Vec::new();
        let mut shadow_by_obj = HashMap::new();
        if let Some(t) = &s.layers {
            let mut objs: Vec<(&u32, &file::ShadowChunk)> = s.shadows.iter().collect();
            objs.sort_by_key(|(o, _)| **o);
            for (&obj, c) in objs {
                let Some(&layer) = t.shadows.get(&obj) else { continue };
                shadow_by_obj.insert(obj, shadows.len());
                shadows.push(SceneShadow {
                    layer,
                    width: c.w,
                    height: c.h,
                    passes: c.passes,
                    spread: c.spread,
                    light: [0.0, 0.0, 0.0, 1.0],
                    length: c.length,
                    alpha: 0.0,
                });
            }
        }
        let mut nodes: Vec<Node> = Vec::new();
        let mut clumps = Vec::new();
        // Per clump: (slot, layer) of its effect nodes.
        let mut effect_layers: Vec<Vec<(usize, i16)>> = Vec::new();
        let mut effs: Vec<EffNode> = Vec::new();
        let mut eff_chunks: HashMap<usize, Vec<piney_effect::eff::EffChunk>> = HashMap::new();
        // InitScene walks the chunk index in object order; the clumps are
        // the scene file's own.
        let mut clump_defs: Vec<&(u32, Vec<u32>)> = s.clumps.iter().collect();
        clump_defs.sort_by_key(|(o, _)| *o);
        for (ci, (_, members)) in clump_defs.iter().enumerate() {
            // CompleteChunk takes nodes whose model is a shadow model out.
            let kept: Vec<u32> = members
                .iter()
                .copied()
                .filter(|&n| {
                    let shadow = loaded
                        .follow(f, n)
                        .filter(|&(_, _, k)| k == file::OBJ)
                        .and_then(|(g, o, _)| loaded.obj_model(g, o))
                        .and_then(|(g, m)| loaded.mtype(g, m))
                        .is_some_and(|t| t & 8 != 0);
                    !shadow
                })
                .collect();
            let first = nodes.len();
            let mut slots = Vec::new();
            let mut effects = Vec::new();
            for &n in &kept {
                let Some((g, o, _)) = loaded.follow(f, n).filter(|&(_, _, k)| k == file::OBJ) else {
                    // Effects (drawn by ccEffObj::DrawNoAnm, not here) still
                    // take their place in the draw list's layer order;
                    // particles do not.
                    if let Some((g, o, _)) = loaded.follow(f, n).filter(|&(_, _, k)| k == file::EFF) {
                        let o2 = s.obj2.get(&n).copied().unwrap_or_default();
                        let layer = if o2.layer != 0 { layer_of(o2.layer) } else { default_layer };
                        effects.push((slots.len(), layer));
                        let chunks =
                            eff_chunks.entry(g).or_insert_with(|| piney_effect::eff::decode(&loaded.files[g].sf.ccs));
                        let pat_num = chunks.iter().find(|c| c.object == o).map_or(0, |c| c.pat_num());
                        // InitScene ends every effect's animation (EndAnm).
                        effs.push(EffNode {
                            obj: n,
                            clump: ci,
                            parent: None,
                            eff: (g, o),
                            layer,
                            local: IDENTITY,
                            localtp: 0.0,
                            pattern: PATTERN_STOP,
                            part: o2.flags & file::OBJ2_PART != 0,
                            pat_num,
                            scale: [0x3f80_0000; 2],
                            rotate: 0,
                            transparency: 0,
                        });
                    }
                    slots.push(None);
                    continue;
                };
                let o2 = s.obj2.get(&n).copied().unwrap_or_default();
                let model = loaded.obj_model(g, o);
                slots.push(Some(nodes.len()));
                nodes.push(Node {
                    obj: n,
                    clump: ci,
                    parent: None,
                    model,
                    succession: o2.flags & file::OBJ2_SUCCESSION != 0,
                    part: o2.flags & file::OBJ2_PART != 0,
                    layer: if o2.layer != 0 { layer_of(o2.layer) } else { default_layer },
                    local: IDENTITY,
                    localtp: 0.0,
                    worldtp: 0.0,
                    disp: if model.is_some() { 3 } else { 0 },
                    modifier: o2.modifier,
                    shadow_model: loaded.files[g].setup.objs.get(&o).filter(|d| d.shadow != 0).map(|d| (g, d.shadow)),
                    shadow: s
                        .obj2
                        .get(&n)
                        .map_or(Some(0), |o2| Some(o2.slayer))
                        .and_then(|l| shadow_by_obj.get(&l).or_else(|| shadow_by_obj.get(&0)).copied()),
                });
            }
            // parentTbl: the node the node's own parent field names, else
            // the clump.
            for i in first..nodes.len() {
                let obj = nodes[i].obj;
                let parent = s.exts.get(&obj).map(|e| e.parent).or_else(|| s.objs.get(&obj).map(|d| d.parent));
                nodes[i].parent = parent.and_then(|p| (first..nodes.len()).find(|&j| j != i && nodes[j].obj == p));
            }
            for e in effs.iter_mut().filter(|e| e.clump == ci) {
                let parent = s.exts.get(&e.obj).map(|x| x.parent).or_else(|| s.objs.get(&e.obj).map(|d| d.parent));
                e.parent = parent.and_then(|p| (first..nodes.len()).find(|&j| nodes[j].obj == p));
            }
            clumps.push(slots);
            effect_layers.push(effects);
        }
        let by_obj = nodes.iter().enumerate().map(|(i, n)| (n.obj, i)).collect();
        // The draw list: per clump (object order), per node, prepended to
        // its layer's entry; layer entries prepended as they appear.
        let mut draw_list: Vec<(i16, Vec<usize>)> = Vec::new();
        for (slots, effects) in clumps.iter().zip(&effect_layers) {
            for (k, slot) in slots.iter().enumerate() {
                let (l, node) = match slot {
                    Some(i) => (nodes[*i].layer, Some(*i)),
                    None => match effects.iter().find(|(e, _)| *e == k) {
                        Some(&(_, l)) => (l, None),
                        None => continue,
                    },
                };
                match draw_list.iter_mut().find(|(p, _)| *p == l) {
                    Some((_, v)) => {
                        if let Some(i) = node {
                            v.insert(0, i);
                        }
                    }
                    None => draw_list.insert(0, (l, node.into_iter().collect())),
                }
            }
        }
        let mut cams: Vec<u32> = s.cameras.clone();
        cams.sort();
        // InitScene adds the lights in lightChunkRoot order, the file's
        // reversed. AddGrp (0x00139060) keeps the group in descending
        // priority, equal priorities in the order added, and ccCreateLight
        // gives a distant light priority -1 and the others 0: the omni
        // lights come before the distant ones (str0580's LGT_se1_5omn*
        // before LGT_se1_5lig1).
        let mut lights: Vec<Light> = s.lights.iter().rev().map(|&(o, k)| Light::new(o, k)).collect();
        lights.sort_by_key(|l| l.priority());
        let names: Vec<Option<String>> = nodes.iter().map(|n| file.name(n.obj).map(str::to_string)).collect();
        let by_name = names.iter().enumerate().filter_map(|(i, n)| n.clone().map(|n| (n, i))).collect();
        Scene {
            file: f,
            names,
            nodes,
            by_obj,
            by_name,
            clumps,
            effs,
            draw_list,
            default_layer,
            camera_obj: cams.first().copied(),
            camera: Camera { pos: Vec3::ZERO, rot: Vec3::ZERO, fov: DEFAULT_FOV },
            camera_bits: ([0; 3], [0; 3]),
            ambient: [0.0; 3],
            lights,
            uv: HashMap::new(),
            morph: HashMap::new(),
            alpha: HashMap::new(),
            placed: HashMap::new(),
            cursor: 0,
            offsets: None,
            frame_now: 0,
            frame_end: s.frames.saturating_sub(1),
            frame_spd: 256,
            frame_cnt: 0,
            ctrl: 0,
            state: 0,
            notes: Vec::new(),
            pcm: Vec::new(),
            z_write_off: HashSet::new(),
            frame: Frame::DEFAULT,
            fog: None,
            div_z: piney_draw::DIV_Z,
            fog_off: HashSet::new(),
            shadows,
            shadow_by_obj,
        }
    }

    /// The scene's file was loaded whole before the stream (an on-memory
    /// record, type -2): give it the `frameOffsetTbl`, so its frames seek
    /// and a -2 end Top rewinds it (`ResetScene`).
    pub fn on_memory(&mut self, loaded: &Loaded) {
        let mut t = vec![None; loaded.files[self.file].setup.frames.max(1) as usize];
        t[0] = Some(0);
        self.offsets = Some(t);
    }

    pub fn ended(&self) -> bool {
        self.state & STATE_ENDED != 0
    }

    /// `ctrl` bit 0: the scene is held (at its last frame); the effect task
    /// stops its counters.
    pub fn paused(&self) -> bool {
        self.ctrl & 1 != 0
    }

    /// The Pcm chunk's blocks: what `Decode_Pcm` hands the PCM player
    /// before the scene starts.
    pub fn preroll(loaded: &Loaded, f: usize) -> Vec<Vec<u8>> {
        let file = &loaded.files[f];
        let Some((at, blocks, words)) = file.setup.pcm(loaded.pcm_track) else { return Vec::new() };
        (0..blocks)
            .filter_map(|b| file.data().get(at + 4 * words * b..at + 4 * words * (b + 1)).map(<[u8]>::to_vec))
            .collect()
    }

    /// `DecodeFrameSection` (0x0014ded0): one step of `frameSpd`.
    pub fn decode(&mut self, loaded: &Loaded) {
        if self.ctrl & 0x20 != 0 {
            self.ctrl &= !0x20;
            self.cursor = self.offsets.as_ref().and_then(|t| t[0]).unwrap_or(0);
            self.frame_now = 1;
            self.decode_chunk(loaded, 0, 1);
            return;
        }
        let s0 = self.frame_now;
        let mut step: i32 = 0;
        if self.ctrl & 1 == 0 {
            self.frame_cnt = self.frame_cnt.wrapping_add(self.frame_spd);
            step = i32::from(self.frame_cnt >> 8);
            self.frame_cnt = i16::from(self.frame_cnt as i8);
        }
        if step < 0 {
            // Reverse play (a debug speed): not used by the game's streams.
            return;
        }
        let mut s1 = s0 + step as u32;
        if s1 > self.frame_end {
            self.ctrl |= 1;
            s1 = self.frame_end;
        }
        if s1 == s0 {
            return;
        }
        // A frame already read once is sought through frameOffsetTbl;
        // without one the records go on from where the last step stopped,
        // after the Top that ended it, still counted as frame s0.
        if let Some(t) = &self.offsets
            && let Some(at) = (s0 + 1..=s1).rev().find_map(|k| t.get(k as usize).copied().flatten())
        {
            self.cursor = at;
        }
        self.frame_now = s1;
        match self.decode_chunk(loaded, s0, s1) {
            file::END => self.state |= 4 | STATE_ENDED,
            file::END_RESET => {
                self.state |= 4 | STATE_ENDED;
                self.reset();
            }
            _ => {}
        }
    }

    /// `ResetScene` (0x00149790) as the -2 end Top calls it: without a
    /// `frameOffsetTbl` it does nothing, and the scene ends as at -1.
    fn reset(&mut self) {
        if self.offsets.is_none() {
            return;
        }
        self.frame_cnt = 0;
        self.frame_now = 0;
        self.ctrl = 0x20;
        self.state &= !(4 | STATE_ENDED | 0x20);
        self.effs.iter_mut().for_each(EffNode::end_anm);
    }

    /// `DecodeFrameChunk(nowf, decf, 0)`: records until a Top past `decf`;
    /// returns the last Top's frame number.
    fn decode_chunk(&mut self, loaded: &Loaded, nowf: u32, decf: u32) -> u32 {
        let f = &loaded.files[self.file];
        let mut nowf = nowf;
        while decf >= nowf {
            let Some(&r) = f.records.get(self.cursor) else { return file::END };
            let here = self.cursor;
            self.cursor += 1;
            match r.kind {
                file::TOP => {
                    nowf = f.u32_at(r.at);
                    if let Some(slot) = self.offsets.as_mut().and_then(|t| t.get_mut(nowf as usize)) {
                        *slot = Some(here);
                    }
                }
                file::F_OBJ => self.f_obj(f, r.at),
                file::F_CAMERA => self.f_camera(f, r.at),
                file::F_AMBIENT => {
                    let c = f.u32_at(r.at);
                    // DecodeF_Ambient: sceVu0ITOF0Vector then times 1/255.
                    self.ambient = [0, 8, 16].map(|s| ee_f(ee::mul(ee::from_int(((c >> s) & 0xff) as i32), COL255)));
                }
                file::F_OMNI_LIGHT => self.f_omni(f, r.at),
                file::F_DISTANT_LIGHT => self.f_distant(f, r.at),
                file::F_MATERIAL => self.f_material(f, r.at),
                file::F_SHADOW => self.f_shadow(f, r.at),
                file::F_MORPHER => {
                    let m = f.u32_at(r.at);
                    let n = f.u32_at(r.at + 4) & 0xffff;
                    let w =
                        (0..n as usize).map(|k| (f.u32_at(r.at + 8 + 8 * k), f.f32_at(r.at + 12 + 8 * k))).collect();
                    self.morph.insert(m, w);
                }
                file::F_NOTE => self.notes.push(Note {
                    frame: nowf,
                    obj: f.u32_at(r.at),
                    event: f.u32_at(r.at + 4),
                    param: f.u32_at(r.at + 8),
                }),
                // DecodeF_Pcm: Infection's (0x0014e4a0) u32 id, u16 blocks,
                // u16, u32 words; Outbreak's (OUT 0x0014d240) u32 id, u8
                // lang, pad, u32 blocks, u32 words, a record of another
                // language read past.
                file::F_PCM => {
                    let (blocks, words, first) = match loaded.pcm_track {
                        None => ((f.u32_at(r.at + 4) & 0xffff) as usize, f.u32_at(r.at + 8) as usize, r.at + 12),
                        Some(t) if f.data().get(r.at + 4) == Some(&t) => {
                            (f.u32_at(r.at + 8) as usize, f.u32_at(r.at + 12) as usize, r.at + 16)
                        }
                        Some(_) => (0, 0, 0),
                    };
                    for b in 0..blocks {
                        let at = first + 4 * words * b;
                        if let Some(bytes) = f.data().get(at..at + 4 * words) {
                            self.pcm.push(bytes.to_vec());
                        }
                    }
                }
                // Frame, F_Shadow, the direct and spot lights, vertex and
                // normal records: read and not used here.
                _ => {}
            }
        }
        nowf
    }

    /// `DecodeF_Obj` (0x0014e950).
    fn f_obj(&mut self, f: &StreamFile, at: usize) {
        let obj = f.u32_at(at);
        if let Some(e) = self.effs.iter_mut().find(|e| e.obj == obj) {
            f_obj_eff(e, f, at);
            return;
        }
        let Some(&i) = self.by_obj.get(&obj) else { return };
        let w = |k: usize| f.u32_at(at + 4 * k);
        let pos = [w(2), w(3), w(4)];
        let rot = const_radians([w(5), w(6), w(7)]);
        let scale = [w(8), w(9), w(10)];
        let tp = f32::from_bits(w(11)).clamp(0.0, 1.0);
        let disp = w(12) & 1;
        let n = &mut self.nodes[i];
        let at_origin = pos.iter().all(|&p| ee::to_f64(p) == 0.0);
        n.disp = if n.part && at_origin { disp as u8 } else { (disp | 2) as u8 };
        n.local = pos_rot_scale(pos, rot, scale);
        n.localtp = tp;
        if !n.succession {
            n.worldtp = tp;
        }
    }

    /// `DecodeF_Camera` (0x0014ece0): the first camera only has a `ccCam`.
    fn f_camera(&mut self, f: &StreamFile, at: usize) {
        let obj = f.u32_at(at);
        if Some(obj) != self.camera_obj {
            return;
        }
        let flag = f.u32_at(at + 4);
        if flag & 1 != 0 {
            return;
        }
        let mut v = [0u32; 8];
        let mut q = at + 8;
        for (bit, slot) in v.iter_mut().enumerate() {
            if flag & (2 << bit) == 0 {
                *slot = f.u32_at(q);
                q += 4;
            }
        }
        let rot = const_radians([v[3], v[4], v[5]]);
        self.camera_bits = ([v[0], v[1], v[2]], rot);
        let c = &mut self.camera;
        c.pos = Vec3::new(ee_f(v[0]), ee_f(v[1]), ee_f(v[2]));
        c.rot = Vec3::new(ee_f(rot[0]), ee_f(rot[1]), ee_f(rot[2]));
        if flag & 0x100 == 0 {
            c.fov = ee_f(v[7]);
        }
    }

    /// `ccCam::SetMatrix_PosRotXYZDebug` (0x001385a0) as `PlaySceneMain`
    /// calls it, in VU0's arithmetic: the unit matrix turned a half turn
    /// about x, times the stream's matrix (+0xc0, which `Init` leaves the
    /// unit matrix), turned about x, y, then z, moved to the position, then
    /// `sceVu0InversMatrix`. World to view, as the game has it.
    pub fn world_view_bits(&self) -> [[u32; 4]; 4] {
        use piney_data::anim::{rot_x_bits_of, rot_y_bits_of, rot_z_bits_of, vu_mul};
        const ONE: u32 = 0x3f80_0000;
        const PI: u32 = 0x4049_0fdb;
        let unit = [[ONE, 0, 0, 0], [0, ONE, 0, 0], [0, 0, ONE, 0], [0, 0, 0, ONE]];
        let (pos, rot) = self.camera_bits;
        let m = vu_mul(&rot_x_bits_of(unit, PI), &unit);
        let m = rot_z_bits_of(rot_y_bits_of(rot_x_bits_of(m, rot[0]), rot[1]), rot[2]);
        // sceVu0TransMatrix: the position added to the last row.
        let r = m[3];
        let m = [m[0], m[1], m[2], [ee::add(r[0], pos[0]), ee::add(r[1], pos[1]), ee::add(r[2], pos[2]), r[3]]];
        // sceVu0InversMatrix (0x001107b0): the transposed rotation and the
        // position taken back through it.
        let t = m[3];
        let rows: [[u32; 4]; 3] = std::array::from_fn(|i| [m[0][i], m[1][i], m[2][i], 0]);
        let mut out = [rows[0], rows[1], rows[2], [0, 0, 0, t[3]]];
        for k in 0..3 {
            let acc = ee::mul(rows[0][k], t[0]);
            let acc = ee::add(acc, ee::mul(rows[1][k], t[1]));
            out[3][k] = ee::sub(0, ee::add(acc, ee::mul(rows[2][k], t[2])));
        }
        out
    }

    fn light(&mut self, obj: u32) -> Option<&mut Light> {
        self.lights.iter_mut().find(|l| l.obj == obj)
    }

    /// `DecodeF_OmniLight` (0x0014f6a0): position, colour, intensity, the
    /// fall-off distances.
    fn f_omni(&mut self, f: &StreamFile, at: usize) {
        let obj = f.u32_at(at);
        let Some(l) = self.light(obj) else { return };
        l.pos = [f.f32_at(at + 8), f.f32_at(at + 12), f.f32_at(at + 16)];
        l.colour = colour(f.u32_at(at + 20));
        l.intensity = f.f32_at(at + 24);
        l.far_start = f.f32_at(at + 28);
        l.far_end = f.f32_at(at + 32);
    }

    /// `DecodeF_Shadow` (0x0014e830): the shadow layer's packet (its
    /// object's, else the default one's) gets the light `sceVu0RotMatrix`
    /// of the record's degrees turns (0, 0, -1) to, and its alpha.
    fn f_shadow(&mut self, f: &StreamFile, at: usize) {
        let obj = f.u32_at(at);
        let Some(&k) = self.shadow_by_obj.get(&obj).or_else(|| self.shadow_by_obj.get(&0)) else { return };
        let r = const_radians([f.u32_at(at + 4), f.u32_at(at + 8), f.u32_at(at + 12)]);
        let m = piney_effect::vu::rot_zyx(&piney_effect::vu::UNIT, [r[0], r[1], r[2], 0]);
        // sceVu0ApplyMatrix of (0, 0, -1, 0): the third column negated.
        let sh = &mut self.shadows[k];
        sh.light = [0, 1, 2, 3].map(|i| if i == 3 { 0.0 } else { -f32::from_bits(m[2][i]) });
        sh.alpha = f.f32_at(at + 16);
    }

    /// `DecodeF_DistantLight` (0x0014ef80): rotation, colour, and with flag
    /// 0x20 the intensity.
    fn f_distant(&mut self, f: &StreamFile, at: usize) {
        let obj = f.u32_at(at);
        let flag = f.u32_at(at + 4);
        let Some(l) = self.light(obj) else { return };
        l.rot = const_radians([f.u32_at(at + 8), f.u32_at(at + 12), f.u32_at(at + 16)]);
        l.colour = colour(f.u32_at(at + 20));
        if flag & 0x20 != 0 {
            l.intensity = f.f32_at(at + 24);
        }
    }

    /// `DecodeF_Material` (0x0014e5b0): the material's texture offset, u
    /// and v as `s16(int(4096 x))`; flag 2 records carry none.
    fn f_material(&mut self, f: &StreamFile, at: usize) {
        let obj = f.u32_at(at);
        if f.u32_at(at + 4) & 2 != 0 {
            return;
        }
        let q = |x: u32| ee::to_int(ee::mul(0x4580_0000, x)) as i16;
        let uv = [q(f.u32_at(at + 8)), q(f.u32_at(at + 12))];
        if let Some(name) = f.name(obj) {
            self.uv.insert(name.to_string(), uv);
        }
    }

    /// `lwMatrix` of node `i` (`ccCoord::_SetLWMatrix` 0x00138380): its
    /// parents' matrices times its own, VU0's multiply-adds.
    pub fn world(&self, i: usize) -> [[u32; 4]; 4] {
        if let Some(m) = self.placed.get(&i) {
            return *m;
        }
        let n = &self.nodes[i];
        // A top node's parent is the clump's own coordinate, whose lwMatrix
        // is its (identity) matrix: the product still rounds (-0 + 0).
        let parent = match n.parent {
            Some(p) if p != i => self.world(p),
            _ => IDENTITY,
        };
        ee_mul(&parent, &n.local)
    }

    /// Effect node `k`'s world matrix (`DrawNoAnm`'s `lwMatrix`): its
    /// parent's times its own.
    pub fn eff_world(&self, k: usize) -> [[u32; 4]; 4] {
        let e = &self.effs[k];
        let parent = e.parent.map_or(IDENTITY, |p| self.world(p));
        ee_mul(&parent, &e.local)
    }

    /// The world matrix of the node named `name` (`GetSubstAdrsF(name)`'s
    /// `lwMatrix`), None when the scene has no such object.
    pub fn world_named(&self, name: &str) -> Option<[[u32; 4]; 4]> {
        self.by_name.get(name).map(|&i| self.world(i))
    }

    /// `ccObj::Draw(1.0)`'s test: the transparency when node `i` draws its
    /// model (a model, `dispSW` 3, above 1/128), else None.
    pub fn visible(&self, i: usize) -> Option<f32> {
        let n = &self.nodes[i];
        if let (Some(_), Some(&tp)) = (n.model, self.alpha.get(&i)) {
            return (tp > 1.0 / 128.0).then_some(tp);
        }
        if n.model.is_none() || n.disp != 3 {
            return None;
        }
        let tp = self.transparency(i);
        (tp > 1.0 / 128.0).then_some(tp)
    }

    /// Every node of an object named `name`, by object: a scene's files can
    /// each have one ([`Scene::by_name`] keeps the last).
    pub fn nodes_named(&self, name: &str) -> Vec<usize> {
        let mut out: Vec<(u32, usize)> = self
            .by_obj
            .iter()
            .filter(|&(_, &i)| self.names.get(i).is_some_and(|n| n.as_deref() == Some(name)))
            .map(|(&o, &i)| (o, i))
            .collect();
        out.sort();
        out.into_iter().map(|(_, i)| i).collect()
    }

    /// Not the game's: node `i` drawn at transparency `alpha` whatever the
    /// scene's records say, or as they say again (None).
    pub fn set_alpha(&mut self, i: usize, alpha: Option<f32>) {
        match alpha {
            Some(a) => self.alpha.insert(i, a),
            None => self.alpha.remove(&i),
        };
    }

    /// Not the game's: node `i` (and its children with it) put at the
    /// world matrix `m`, or where the scene has it again (None).
    pub fn place(&mut self, i: usize, m: Option<[[u32; 4]; 4]>) {
        match m {
            Some(m) => self.placed.insert(i, m),
            None => self.placed.remove(&i),
        };
    }

    /// Node `i` turned `ry` (radians, float bits) about its own y axis from
    /// where the scene puts it, as a child of it with that rotation
    /// (`sceVu0RotMatrixY`) would stand: the title's turning icons. Its
    /// children turn with it.
    pub fn turn_y(&mut self, i: usize, ry: u32) {
        self.placed.remove(&i);
        let rest = self.world(i);
        let r = piney_data::anim::rot_y_bits_of(IDENTITY, ry);
        self.placed.insert(i, ee_mul(&rest, &r));
    }

    /// Every node in `ccStreamDrawLayerList::Draw`'s order, with its layer.
    pub fn walk(&self) -> impl Iterator<Item = (i16, usize)> + '_ {
        self.draw_list.iter().flat_map(|(l, v)| v.iter().map(move |&i| (*l, i)))
    }

    /// The transparency `ccObj::Draw` multiplies in: `worldtp`, or with
    /// succession its own times its parent's (`_GetTransparency` 0x00138490).
    pub fn transparency(&self, i: usize) -> f32 {
        let n = &self.nodes[i];
        match (n.succession, n.parent) {
            (true, Some(p)) if p != i => ee_f(ee::mul(n.localtp.to_bits(), self.transparency(p).to_bits())),
            (true, None) => n.localtp,
            _ => n.worldtp,
        }
    }
}

/// 1/255 (`col255to1Vector` 0x002f7370).
const COL255: u32 = 0x3b80_8081;

fn ee_f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `ccSetColor(out, rgba, 1.0)` for an alpha of 0: each channel / 255.
fn colour(c: u32) -> [f32; 3] {
    [0, 8, 16].map(|s| ee_f(ee::mul(ee::from_int(((c >> s) & 0xff) as i32), COL255)))
}

/// `sceVu0MulMatrix(a, b)`: a times b, as stored columns.
pub fn ee_mul(a: &[[u32; 4]; 4], b: &[[u32; 4]; 4]) -> [[u32; 4]; 4] {
    piney_data::anim::vu_mul(a, b)
}

/// `DecodeF_Obj`'s effect branch (0x0014eb80): a stopped effect with the
/// part bit (re)starts where the record's place is off the origin; any
/// other starts as the record's transparency comes on from a `localtp` of
/// 0. Then `Animate(1)`, the matrix the place alone (`sceVu0UnitMatrix`,
/// `TransMatrix`), `localtp` and the Eff's transparency the record's
/// (clamped to 0..1), its scale the record's x and y and its turn the
/// record's z (radians).
fn f_obj_eff(e: &mut EffNode, f: &StreamFile, at: usize) {
    let w = |k: usize| f.u32_at(at + 4 * k);
    let pos = [w(2), w(3), w(4)];
    let rot = const_radians([w(5), w(6), w(7)]);
    let tp = f32::from_bits(w(11)).clamp(0.0, 1.0);
    let off_origin = pos.iter().any(|&p| ee::to_f64(p) != 0.0);
    if e.part && e.pattern == PATTERN_STOP {
        if off_origin {
            e.pattern = PATTERN_START;
        }
    } else if e.localtp == 0.0 && tp != 0.0 {
        e.pattern = PATTERN_START;
    }
    e.animate(1);
    let mut m = IDENTITY;
    m[3] = [pos[0], pos[1], pos[2], 0x3f80_0000];
    e.local = m;
    e.localtp = tp;
    e.transparency = tp.to_bits();
    e.scale = [w(8), w(9)];
    e.rotate = rot[2];
}

/// `ccCoord::SetMatrix_PosRotZYXScale(pos, rot, scale)` (0x00138120): a
/// scale matrix, `sceVu0RotMatrix` by `rot` (Rx Ry Rz on the left), then
/// the translation stored in the fourth column.
pub fn pos_rot_scale(pos: [u32; 3], rot: [u32; 3], scale: [u32; 3]) -> [[u32; 4]; 4] {
    let s = [[scale[0], 0, 0, 0], [0, scale[1], 0, 0], [0, 0, scale[2], 0], [0, 0, 0, ONE]];
    let mut m = rot_bits_of(s, rot);
    // sceVu0TransMatrix: the fourth column's xyz plus pos.
    for k in 0..3 {
        m[3][k] = ee::add(m[3][k], pos[k]);
    }
    m
}

/// Stored columns of bits to a glam matrix.
pub fn to_mat4(m: &[[u32; 4]; 4]) -> glam::Mat4 {
    glam::Mat4::from_cols_array_2d(&m.map(|c| c.map(f32::from_bits)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pos_rot_scale_is_t_r_s() {
        let one = 1.0f32.to_bits();
        let m = pos_rot_scale([10.0f32.to_bits(), 0, 0], [0, 0, 0], [2.0f32.to_bits(), one, one]);
        let g = to_mat4(&m);
        let p = g.transform_point3(Vec3::new(1.0, 1.0, 0.0));
        assert!((p - Vec3::new(12.0, 1.0, 0.0)).length() < 1e-6, "{p:?}");
    }

    /// `SetFog(200, 7000, colour)` clears to 200 and fogs fully from 7,000;
    /// `SetFog(1000, 7500, 0, 85, colour)` holds at 2.55 x 15 past 7,500.
    #[test]
    fn fog_lines() {
        let f = SceneFog::new(200.0, 7000.0, 0x0014_1e00);
        assert_eq!(
            (f.value(0.0), f.value(200.0), f.value(3600.0), f.value(7000.0), f.value(9000.0)),
            (255, 255, 127, 0, 0)
        );
        assert_eq!(f.colour, [0x00, 0x1e, 0x14]);
        let g = SceneFog::with_rates(1000.0, 7500.0, 0.0, 85.0, 0x0014_4870);
        assert_eq!((g.value(500.0), g.value(7500.0), g.value(20000.0)), (255, 38, 38));
    }
}
