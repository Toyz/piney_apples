//! What a boss shows each frame, as the field and the effects read it, the
//! same for every boss: its body's draw, its parts and the images it keeps,
//! the shards it drew, the objects its draw hides, its sword trail, its
//! wave's pose and the places its effects follow ([`Output`], written at the
//! end of its frame); and the pictures it draws beside the bodies
//! ([`Picture`], asked in order as [`super::Out::Picture`]).

use super::{Anm, F, ONE, V4};
use crate::geom::M4;

/// The frame's draw requests of one boss (a part keeps its own).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    pub body: BodyDraw,
    /// The characters of its parts, drawn and hit with it, in order.
    pub parts: Vec<usize>,
    /// The images it keeps inside its own members (Innis's slaves).
    pub figures: Vec<Figure>,
    /// The look's shard clumps drawn (`DrawPartsClump`): which, where, the
    /// turn.
    pub clumps: Vec<(usize, V4, V4)>,
    /// Objects of the body's clip its draw hides, and whether its
    /// after-images hide them too.
    pub hidden: Vec<&'static str>,
    pub hidden_in_images: bool,
    /// The wave's pose (`m_anmw`), which [`super::Out::DrawWave`] draws.
    pub wave: Option<u32>,
    pub trail: Trail,
    /// The places its effects' generators follow, and the bursts drawn.
    pub follow: Vec<(Follow, V4)>,
    pub explodes: Vec<super::magus::leaf::Explode>,
}

/// Where `ccBoss::Draw` puts the body and whether it draws it; what its
/// `PreDrawAnm` changes: a matrix of its own, the clip drawn once more.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BodyDraw {
    pub pos: V4,
    pub dirc: V4,
    pub shown: bool,
    pub matrix: Option<M4>,
    pub again: bool,
}

/// An image a boss keeps inside its own members: its character, which of
/// the look's images draws it, and its `ccBoss`'s pose and draw.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Figure {
    pub me: usize,
    pub image: usize,
    pub anm: Anm,
    pub dirc: V4,
    pub alpha: F,
    pub shown: bool,
}

/// A sword trail the field keeps for the boss (`ccLattice`): none, kept
/// (its `Disp` alone), or a row added at two dummies of the body as last
/// drawn, under a node's world matrix.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Trail {
    #[default]
    None,
    Keep,
    Row {
        node: &'static str,
        points: [&'static str; 2],
    },
}

/// A place an effect's generator follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Follow {
    /// The character's `pos` (Kyvia's core's and its auras').
    Char(usize),
    /// Kyvia's `DedDmgVec` (+0x29540), the death's smoke.
    DeadSmoke,
    /// `LBpos[k]` (+0x294a0), `LBposSub[k]` (+0x294d0): a beam's ends.
    BeamHead(u8),
    BeamTail(u8),
    /// A gomora's `AuraPos` (+0x293f0), by its character.
    Aura(usize),
    /// The magical ball's `PlaPos[k]` and `PlaPos2[k]` (the boss's one
    /// ball).
    Plasma(u8),
    Plasma2(u8),
    /// Hell's express's `Pos[k]` (a soul's place; the boss's one).
    Soul(u8),
    /// A leaf's marker (`ccBoss03Leaf` +0x29420), by its character.
    Marker(usize),
}

/// Which file a picture's object or clip is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PicFile {
    Boss,
    /// `xeffect`.
    Eff,
}

/// The layer a picture's clip goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnmLayer {
    Obj,
    Eff,
    Char,
}

/// Where a picture is put: `SetMatrix_PosRotZYX(pos, dirc)`, a matrix, or
/// a node of the body as it was last drawn (`GetSubstAdrsF`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Root {
    At { pos: V4, dirc: V4 },
    Matrix(M4),
    Node(&'static str),
}

/// `SetFogBlend(rate, colour)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blend {
    pub rate: F,
    pub colour: u32,
}

/// A picture a boss's code or effects draw beside the bodies: an object of
/// a file (lit, on charLayer) or a clip posed at `time`; `alpha` None is
/// the body's `setTransparency` as its frame left it, `clip` None the
/// body's clip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Picture {
    Obj {
        file: PicFile,
        name: &'static str,
        at: Root,
        alpha: Option<F>,
        fog: Option<Blend>,
        blend: Option<u8>,
    },
    Anm {
        file: PicFile,
        layer: AnmLayer,
        clip: Option<String>,
        time: u32,
        at: Root,
        alpha: Option<F>,
        blend: Option<u8>,
    },
}

/// Where a count is shown: over a character, or at a place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    Char(usize),
    At(V4),
}

impl Picture {
    /// A clip of the boss's file at `pos` turned by `dirc`, opaque (Kyvia's
    /// arm, `DrawParts(anmw, 0)`; Macha's light ball, `ccAnm::Draw`).
    pub fn anm(clip: String, time: u32, pos: V4, dirc: V4, layer: AnmLayer) -> Picture {
        let at = Root::At { pos, dirc };
        Picture::Anm { file: PicFile::Boss, layer, clip: Some(clip), time, at, alpha: Some(ONE), blend: None }
    }
}

impl Output {
    /// Ready for the frame's requests: every list emptied, its room kept.
    pub fn clear(&mut self) {
        self.body = BodyDraw::default();
        self.parts.clear();
        self.figures.clear();
        self.clumps.clear();
        self.hidden.clear();
        self.hidden_in_images = false;
        self.wave = None;
        self.trail = Trail::None;
        self.follow.clear();
        self.explodes.clear();
    }
}
