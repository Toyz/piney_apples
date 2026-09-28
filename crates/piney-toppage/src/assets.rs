//! What the top page reads: its scene `xdttopen0` out of `DATA.BIN`, and
//! the volume's generated tables (`piney_data::tables::toppage`: the
//! board's threads and words, the idle animation; `tables::game`: the
//! server of each town the hand-off uses; the bitmap fonts).

use std::rc::Rc;
use std::sync::Arc;

use piney_data::Result;
use piney_data::archive::Archive;
use piney_data::iso::Iso;
use piney_data::tables::sjis::encode;
use piney_data::tables::{game, toppage};
use piney_desktop::assets::{SceneFile, read_fonts};
use piney_desktop::kanji::Fonts;
use piney_desktop::message::WindowTexture;

use crate::bbs::{BbsAssets, Labels, MASK_TEX};
use crate::content;

/// `toppageFileList` (toppage.prg 0x00404980): `XDTTOPEN0.CCS`, category
/// 4, the one file the top page adds to the common lists.
pub const TOP_FILE: &str = "xdttopen0";

/// Everything the top page reads from the disc.
pub struct Assets {
    /// The disc's volume, whose tables the top page reads.
    pub disc: piney_data::volume::Volume,
    pub archive: Arc<Archive>,
    pub fonts: Fonts,
    /// `volumeNum`.
    pub volume: u32,
    /// The idle animation, `@1088[volumeNum - 1]`.
    pub neutral: String,
    /// The board's scene, tables and strings; both tables are read, the
    /// save's `parodyFlag` picks one.
    pub bbs: BbsAssets,
    /// `ccGame::ChangeScene`'s server of each town.
    pub servers: [i32; 8],
}

impl Assets {
    pub fn read(iso: &mut Iso, archive: Arc<Archive>) -> Result<Self> {
        let file = Rc::new(SceneFile::read(&archive, TOP_FILE)?);
        let fonts = read_fonts(iso.volume()?, &archive)?;
        // `volumeNum` is the disc's volume's number.
        let disc = iso.volume()?;
        let volume = disc.number() as u32;
        // `_ChangeMode` copies the table to the stack and reads it at
        // `sp + 44 + 4 volumeNum`, 4 bytes before the copy: entry
        // `volumeNum - 1`.
        let entry = volume.clamp(1, 4) - 1;
        let neutral = toppage::NEUTRAL[entry as usize].to_string();
        let mask_tex = WindowTexture::read_named(&archive, TOP_FILE, MASK_TEX, None).map(|w| (w.tex, w.tex_h));
        let labels =
            Labels { author: encode(*toppage::AUTHOR), time: encode(*toppage::TIME), player: encode(*toppage::PLAYER) };
        let bbs = BbsAssets {
            file,
            tables: [content::threads(disc, false), content::threads(disc, true)],
            mask_tex,
            new_msg: encode(*toppage::NEW_MSG),
            no_msg: encode(*toppage::NO_MSG),
            labels,
        };
        let mut servers = [0; 8];
        servers.copy_from_slice(game::of(disc).town_server);
        Ok(Assets { disc, archive, fonts, volume, neutral, bbs, servers })
    }
}
