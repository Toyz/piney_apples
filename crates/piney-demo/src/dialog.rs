//! What `Data_Control` (DEMO.PRG, DataControl.cpp) draws with: the texts
//! and the cursor texture, read once from the disc. The boot's memory-card
//! question (`BootMem_Control`, [`crate::card`]) and the load screen
//! (`DataLoad_Control`, [`crate::dataload`]) are both `Data_Control`s and
//! draw through the same `InfoMessage`, `YesNoDialogue` and
//! `TimeAlphaCurDraw`.
//!
//! Every text is a `ccKanji` built with `Init(3, 16)` (the demo's own
//! `ccKanji::ccKanji` 0x00403600) in `ccSpriteColorTable[7]` (0x80, 0x80,
//! 0x80, 0x80) unless `SetLoadPar` recolours it. The messages are
//! `saveSysMsg` (the strings `__sinit_sdmng.cpp` puts there, in the
//! executable), split by `ccKanjiStrSeparate`; the other strings are the
//! overlay's, through the `STR_` pointers in the executable's `.sdata`.

use piney_data::tables::sjis::encode;
use piney_data::tables::title;
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::assets::SceneFile;
use piney_desktop::kanji::Fonts;
use piney_draw::TexRef;

/// `InfoMessage(pn, FLG)`: at most five lines with FLG, four without.
pub const LINES: usize = 5;
/// `ccKanji::Init(3, 16)`.
pub const KANJI_L: u32 = 3;
pub const KANJI_PACKETS: usize = 16;
/// Both screens' layers are framed `SetFrame(0, 0, 512, 384, 256, 192, 1,
/// ay)`, `ay` 6/7 (0x3f5b6db7).
pub const LAYER_AY: u32 = 0x3f5b_6db7;

/// The fonts, texts and cursor texture the screens draw with.
pub struct DialogAssets {
    pub fonts: Fonts,
    pub yes: Vec<u8>,
    pub no: Vec<u8>,
    pub highlight: Vec<u8>,
    pub lv: Vec<u8>,
    pub alltime: Vec<u8>,
    pub nodeta: Vec<u8>,
    pub memorycard: Vec<u8>,
    pub slot1: Vec<u8>,
    pub slot2: Vec<u8>,
    pub data: Vec<u8>,
    pub clear: Vec<u8>,
    pub parody: Vec<u8>,
    pub vol: Vec<u8>,
    /// By message number: `saveSysMsg[n]`'s first lines, up to the first
    /// empty one (at most [`LINES`]); `None` where the entry is null.
    pub messages: Vec<Option<Vec<Vec<u8>>>>,
    /// `TEX_xgtcur00` in `title1` (the cursors and the button), and its
    /// height.
    pub cursor: TexRef,
    pub cursor_h: i32,
}

impl DialogAssets {
    /// The volume's texts (`piney_data::tables::title`: the `STR_`
    /// pointers in `.sdata` at gp 0x003783ec on, demo.prg's `@1114` "#G"
    /// and `@1612` "Vol.", and `saveSysMsg`).
    pub fn read(volume: Volume, fonts: Fonts, title: &SceneFile) -> Result<Self> {
        // Each message's lines up to the first empty one (InfoMessage stops
        // there), at most LINES.
        let messages = title::of(volume)
            .save_sys_msg
            .iter()
            .map(|m| m.map(|lines| lines.iter().take(LINES).take_while(|l| !l.is_empty()).map(|l| encode(l)).collect()))
            .collect();
        let find = |name: &str| title.ccs.find_object(name).ok_or_else(|| Error::NotFound(name.to_string()));
        let texture = find(crate::names::CURSOR_TEXTURE)?;
        let clut = find("CLT_xgtcur00")?;
        let (textures, _) = piney_data::texture::read(&title.ccs)?;
        let cursor_h = textures.iter().find(|t| t.object == texture).map_or(0, |t| t.height(0) as i32);
        Ok(DialogAssets {
            fonts,
            yes: encode(*title::YES),
            no: encode(*title::NO),
            highlight: encode(*title::HIGHLIGHT),
            lv: encode(*title::LV),
            alltime: encode(*title::ALLTIME),
            nodeta: encode(*title::NODETA),
            memorycard: encode(*title::MEMORYCARD),
            slot1: encode(*title::SLOT1),
            slot2: encode(*title::SLOT2),
            data: encode(*title::DATA),
            clear: encode(*title::CLEAR),
            parody: encode(*title::PARODY),
            vol: encode(*title::VOL),
            messages,
            cursor: TexRef::Ccs { file: title.stem.clone(), texture, clut },
            cursor_h,
        })
    }

    /// `saveSysMsg[pn & 0xfff]`'s lines (`GetMessage` 0x00174300).
    pub fn message(&self, pn: u32) -> Option<&[Vec<u8>]> {
        self.messages.get((pn & 0xfff) as usize)?.as_deref()
    }
}
