//! The one sprite request: a `ccEff` drawn with `ccEff::Draw(pos, pattern)`
//! ([`EffSprite`]), whatever asks for it - a town's or a story map's lens
//! flare, clouds and glows, a field's weather and fireflies, a dungeon's
//! sparks, a symbol's fire, an enemy's breath. What its owner did to the
//! `ccEff` after `Init(chunk, 1)` is the request's [`EffState`], one of the
//! game's own code paths; the effect hosts keep one `ccEff` per file, name
//! and state (piney-game's `fx::sprites`).

use crate::ee::{F, ONE, V4};

/// `eff` of scene file `file` at `pos` with `pattern`, on `layer`, at its
/// scale, turn and transparency, its colour (+0x2c) where the code sets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffSprite {
    pub file: &'static str,
    pub eff: &'static str,
    pub pos: V4,
    pub pattern: u16,
    pub layer: i16,
    pub scale: [F; 2],
    pub rotate: F,
    pub transparency: F,
    /// None: the colour as `Init` left it (all but a symbol's fires).
    pub colour: Option<u32>,
    pub state: EffState,
}

impl EffSprite {
    /// `eff` of `file` at `pos`, `pattern`, on `layer` in `state`: scale
    /// 1, no turn, opaque, its own colour.
    pub fn new(file: &'static str, eff: &'static str, pos: V4, pattern: u16, layer: i16, state: EffState) -> EffSprite {
        EffSprite { file, eff, pos, pattern, layer, scale: [ONE; 2], rotate: 0, transparency: ONE, colour: None, state }
    }
}

/// What a `ccEff`'s owner did to it after `Init(chunk, 1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffState {
    /// PRIM's fog bit (+0x62 bit 5) kept.
    pub fog: bool,
    /// The depth test kept (`SetRenderState(CCRS_ZENABLE, 0)` turns it off).
    pub ztest: bool,
    pub clut: Option<Clut>,
}

impl EffState {
    /// As `Init(chunk, 1)` leaves it: fogged, depth-tested (Lia Fail's
    /// glows, which the buildings hide; an arena's fireflies).
    pub const INIT: EffState = EffState { fog: true, ztest: true, clut: None };
    /// `LENSFLARE`'s and `CLOUD`'s constructors: `SetRenderState(
    /// CCRS_ZENABLE, 0)` and PRIM's fog bit cleared.
    pub const FLARE: EffState = EffState { fog: false, ztest: false, clut: None };
    /// PRIM's fog bit cleared, the depth test kept (`EVENTAREA01`'s glows,
    /// the weather's sprites that clear it, a breath's flames).
    pub const NO_FOG: EffState = EffState { fog: false, ztest: true, clut: None };

    /// Depth-tested, with the fog bit as the drawing left it.
    pub const fn fogged(fog: bool) -> EffState {
        EffState { fog, ztest: true, clut: None }
    }
}

/// A palette the code gives the `ccEff`, by name in its file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Clut {
    /// `eff->tex.clutChunk = new` outright (an enemy's breath).
    Set(&'static str),
    /// `ccEff::ChangeClut(new, old)` (main 0x0013bb20): `new` while the
    /// palette is `old` (a dungeon's sparks, `DUNGEON::ChangeClut`).
    Change { new: &'static str, old: &'static str },
}
