//! One CCSF file of a stream after `ccStream::DecodeSetup`: the setup chunks
//! the scene is built from (Obj, ExtObj, Obj2, Clump, Camera, Light, Layer,
//! Shadow, Pcm, then Frame), and the frame section as a list of records. The
//! layouts are in `docs/formats/ccs.md`, the Shadow chunk in
//! `docs/engine/shadow.md` and the Pcm chunk (one per voice language from
//! Outbreak on) in `docs/formats/voice.md`.

use std::collections::HashMap;
use std::rc::Rc;

use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::ccs;
use piney_desktop::assets::SceneFile;

pub const OBJ: u16 = 0x0100;
pub const MATERIAL: u16 = 0x0200;
pub const TEXTURE: u16 = 0x0300;
pub const CLUT: u16 = 0x0400;
pub const CAMERA: u16 = 0x0500;
pub const LIGHT: u16 = 0x0600;
pub const MODEL: u16 = 0x0800;
pub const CLUMP: u16 = 0x0900;
pub const EXT_OBJ: u16 = 0x0a00;
pub const PARTICLE: u16 = 0x0d00;
pub const EFF: u16 = 0x0e00;
pub const DUMMY_POS: u16 = 0x1300;
pub const DUMMY_POS_ROT: u16 = 0x1400;
pub const LAYER: u16 = 0x1700;
pub const SHADOW: u16 = 0x1800;
pub const MORPHER: u16 = 0x1900;
pub const OBJ2: u16 = 0x2000;
pub const PCM: u16 = 0x2200;

/// Frame-section kinds (`ccStream::DecodeFrameChunk` 0x0014e1b0).
pub const TOP: u16 = 0xff01;
pub const F_OBJ: u16 = 0x0101;
pub const F_NOTE: u16 = 0x0108;
pub const F_MATERIAL: u16 = 0x0201;
pub const F_CAMERA: u16 = 0x0502;
pub const F_AMBIENT: u16 = 0x0601;
pub const F_DISTANT_LIGHT: u16 = 0x0602;
pub const F_DIRECT_LIGHT: u16 = 0x0604;
pub const F_SPOT_LIGHT: u16 = 0x0606;
pub const F_OMNI_LIGHT: u16 = 0x0608;
pub const F_SHADOW: u16 = 0x1801;
pub const F_MORPHER: u16 = 0x1901;
pub const F_PCM: u16 = 0x2201;

/// The Top frame number that ends the section; -2 also resets the scene.
pub const END: u32 = 0xffff_ffff;
pub const END_RESET: u32 = 0xffff_fffe;

/// `ccLight.type` from the Light chunk (`ccCreateLight` 0x00138960).
pub const LIGHT_DISTANT: i16 = 1;
pub const LIGHT_DIRECT: i16 = 2;
pub const LIGHT_SPOT: i16 = 3;
pub const LIGHT_OMNI: i16 = 4;

/// Obj2 flag bits.
pub const OBJ2_SUCCESSION: u32 = 1;
pub const OBJ2_PART: u32 = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObjDef {
    pub parent: u32,
    pub model: u32,
    pub shadow: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExtDef {
    pub parent: u32,
    pub target: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Obj2 {
    pub flags: u32,
    pub modifier: u32,
    pub layer: u32,
    pub slayer: u32,
}

/// A frame-section chunk: its kind and where its payload is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    pub kind: u16,
    pub at: usize,
    pub words: usize,
}

/// `Decode_Layer` (0x0014c5d0): each draw layer's priority, in the order
/// the chunk lists them; a null entry is the scene's default layer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayerTable {
    /// LYR_ object -> priority.
    pub layers: HashMap<u32, i16>,
    /// The default layer's priority, when the chunk has one (else the scene
    /// draws on the layer it was given).
    pub default: Option<i16>,
    /// Shadow layers (kind 1): LYR_ object -> priority, 0 the default one.
    pub shadows: HashMap<u32, i16>,
}

/// `Decode_Shadow` (0x0014c3e0): a shadow layer's `ccShadowChunk`, which
/// `InitScene` makes a `ccShadowPacket` of.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShadowChunk {
    /// `SetBuffer(x, y, w, h)`: the buffer, `w` x `h`.
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    /// The packet's +0x15e (the copies laid) and +0x15f (how far apart).
    pub passes: u8,
    pub spread: u8,
    /// The shadow's length (the packet's +0x170).
    pub length: f32,
}

/// The setup section, as the scene needs it.
#[derive(Clone, Debug, Default)]
pub struct Setup {
    /// Object -> the kind of chunk that defines it in this file.
    pub kind: HashMap<u32, u16>,
    pub objs: HashMap<u32, ObjDef>,
    pub exts: HashMap<u32, ExtDef>,
    pub obj2: HashMap<u32, Obj2>,
    /// (clump, nodes), in file order.
    pub clumps: Vec<(u32, Vec<u32>)>,
    /// Camera objects, in file order.
    pub cameras: Vec<u32>,
    /// (light, type), in file order.
    pub lights: Vec<(u32, i16)>,
    pub layers: Option<LayerTable>,
    /// Shadow layer object (0 the default) -> its Shadow chunk.
    pub shadows: HashMap<u32, ShadowChunk>,
    /// The Pcm chunks, in file order: Infection's and Mutation's streams
    /// have one, Outbreak's and Quarantine's one a voice language.
    pub pcms: Vec<PcmChunk>,
    /// The Frame chunk's count: frames 0 .. count - 1.
    pub frames: u32,
}

/// A Pcm chunk: its language (+7), the payload offset of its first block,
/// its blocks and the words in a block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PcmChunk {
    pub lang: u8,
    pub at: usize,
    pub blocks: usize,
    pub words: usize,
}

impl Setup {
    /// The Pcm chunk `Decode_Pcm` plays: with `track` None (Infection's and
    /// Mutation's decoder, which takes every one) the first; with a track
    /// (Outbreak's, OUT 0x0014cab0, which reads past a chunk whose language
    /// is not `ccPcmSound +0x1c`, the voice byte `Open` was given) the one
    /// of that language.
    pub fn pcm(&self, track: Option<u8>) -> Option<(usize, usize, usize)> {
        let c = match track {
            None => self.pcms.first(),
            Some(t) => self.pcms.iter().find(|c| c.lang == t),
        }?;
        Some((c.at, c.blocks, c.words))
    }
}

/// A stream file.
pub struct StreamFile {
    /// The archive member's stem: what `ModelDraw::file` names.
    pub stem: String,
    pub sf: Rc<SceneFile>,
    pub setup: Setup,
    /// The frame section after the Frame chunk, Tops included.
    pub records: Vec<Record>,
}

impl StreamFile {
    /// Read member `stem` of `archive` (inflated once, by
    /// `piney_desktop::assets::SceneFile`).
    pub fn read(archive: &Archive, stem: &str) -> Result<Self> {
        let sf = Rc::new(SceneFile::read(archive, stem)?);
        let (setup, records) = parse(&sf.ccs)?;
        Ok(StreamFile { stem: stem.to_string(), sf, setup, records })
    }

    pub fn data(&self) -> &[u8] {
        &self.sf.ccs.data
    }

    pub fn u32_at(&self, at: usize) -> u32 {
        self.data().get(at..at + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
    }

    pub fn f32_at(&self, at: usize) -> f32 {
        f32::from_bits(self.u32_at(at))
    }

    /// Whether object `obj` is defined in another file (`#` in the index).
    pub fn external(&self, obj: u32) -> bool {
        let c = &self.sf.ccs;
        c.objects.get(obj as usize).and_then(|o| c.files.get(o.file as usize)).is_some_and(|f| f.starts_with('#'))
    }

    pub fn name(&self, obj: u32) -> Option<&str> {
        self.sf.ccs.object_name(obj)
    }

    /// The name the game's chunk index holds after the setup:
    /// `Decode_ExtObj` (0x0014cb80) writes "EXT" over the first three
    /// letters of an ExtObj's (`OBJ_x` becomes `EXT_x`).
    pub fn index_name(&self, obj: u32) -> Option<String> {
        let n = self.name(obj)?;
        if self.setup.kind.get(&obj) == Some(&EXT_OBJ) && n.len() >= 3 {
            Some(format!("EXT{}", &n[3..]))
        } else {
            Some(n.to_string())
        }
    }

    /// The frame numbers of the Tops, in order (the end Top included).
    pub fn tops(&self) -> impl Iterator<Item = u32> + '_ {
        self.records.iter().filter(|r| r.kind == TOP).map(|r| self.u32_at(r.at))
    }
}

fn parse(c: &ccs::Ccs) -> Result<(Setup, Vec<Record>)> {
    let d = &c.data;
    let u32_at = |at: usize| d.get(at..at + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()));
    let u16_at = |at: usize| d.get(at..at + 2).map_or(0, |b| u16::from_le_bytes(b.try_into().unwrap()));
    let mut s = Setup::default();
    let mut records = Vec::new();
    for ch in c.walk().chunks {
        let q = ch.payload();
        if ch.in_frames {
            records.push(Record { kind: ch.kind, at: q, words: ch.size_words as usize });
            continue;
        }
        let first = u32_at(q);
        match ch.kind {
            OBJ => {
                let shadow = if c.version >= 0x96 { u32_at(q + 12) } else { 0 };
                s.objs.insert(first, ObjDef { parent: u32_at(q + 4), model: u32_at(q + 8), shadow });
                s.kind.insert(first, OBJ);
            }
            EXT_OBJ => {
                s.exts.insert(first, ExtDef { parent: u32_at(q + 4), target: u32_at(q + 8) });
                s.kind.insert(first, EXT_OBJ);
            }
            SHADOW => {
                let f = d.get(q + 20..q + 24).map_or(0.0, |b| f32::from_le_bytes(b.try_into().unwrap()));
                s.shadows.insert(
                    first,
                    ShadowChunk {
                        x: u16_at(q + 4),
                        y: u16_at(q + 6),
                        w: u16_at(q + 8),
                        h: u16_at(q + 10),
                        passes: d.get(q + 16).copied().unwrap_or(0),
                        spread: d.get(q + 17).copied().unwrap_or(0),
                        length: f,
                    },
                );
            }
            OBJ2 => {
                s.obj2.insert(
                    first,
                    Obj2 {
                        flags: u32_at(q + 4),
                        modifier: u32_at(q + 8),
                        layer: u32_at(q + 12),
                        slayer: u32_at(q + 16),
                    },
                );
            }
            CLUMP => {
                let n = u16_at(q + 4) as usize;
                s.clumps.push((first, (0..n).map(|k| u32_at(q + 8 + 4 * k)).collect()));
                s.kind.insert(first, CLUMP);
            }
            CAMERA => {
                s.cameras.push(first);
                s.kind.insert(first, CAMERA);
            }
            LIGHT => {
                s.lights.push((first, u16_at(q + 4) as i16));
                s.kind.insert(first, LIGHT);
            }
            LAYER => {
                let n = u16_at(q) as usize;
                let mut t = LayerTable::default();
                let entries: Vec<(u8, u32)> =
                    (0..n).map(|k| (d.get(q + 4 + 8 * k).copied().unwrap_or(0), u32_at(q + 8 + 8 * k))).collect();
                // Decode_Layer: without a null draw-layer entry the default
                // layer comes first; without a null shadow-layer (kind 1)
                // entry the default shadow layer next; then each entry in
                // order takes the next number.
                let mut pri: i16 = 0;
                if !entries.iter().any(|&(k, o)| k == 0 && o == 0) {
                    t.default = Some(pri);
                    pri += 1;
                }
                if !entries.iter().any(|&(k, o)| k == 1 && o == 0) {
                    t.shadows.insert(0, pri);
                    pri += 1;
                }
                for (kind, obj) in entries {
                    if kind == 0 {
                        if obj == 0 {
                            t.default = Some(pri);
                        } else {
                            t.layers.insert(obj, pri);
                            s.kind.insert(obj, LAYER);
                        }
                    } else if kind == 1 {
                        t.shadows.insert(obj, pri);
                    }
                    pri += 1;
                }
                s.layers = Some(t);
            }
            PCM => {
                s.pcms.push(PcmChunk {
                    lang: d.get(q + 7).copied().unwrap_or(0),
                    at: q + 16,
                    blocks: u32_at(q + 8) as usize,
                    words: u32_at(q + 12) as usize,
                });
            }
            ccs::FRAME => s.frames = first,
            MODEL | MATERIAL | TEXTURE | CLUT | EFF | PARTICLE | MORPHER | DUMMY_POS | DUMMY_POS_ROT => {
                s.kind.entry(first).or_insert(ch.kind);
            }
            _ => {}
        }
    }
    Ok((s, records))
}
