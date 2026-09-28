//! The subtitles under an event's stream: `ccEventStream(num, 1)` (0x001b5670)
//! shows the lines of `evStrMsgTbl[num]` (0x003112f0; `evStrMsgTblp` 0x00311510
//! in Parody Mode), 12-byte `EVSTRMSG` records indexed by the scene's notes, in
//! a speech window on a layer of its own (242), each frame after the scene's
//! task and before the stream's own (`docs/engine/stream.md`, "Subtitles"). A
//! record shows with Movie Text on or its flag 0x800. When the stream returns
//! the window vanishes without its fade; it is the desktop's speech window
//! ([`piney_desktop::message`]), and no page cursor ever shows.

use piney_data::archive::Archive;
use piney_data::save::{SaveData, offset};
use piney_data::tables::{sjis, stream};
use piney_data::volume::Volume;
use piney_data::{Error, Result};
use piney_desktop::anm::Ctx;
use piney_desktop::assets::read_fonts;
use piney_desktop::kanji::{Fonts, Names};
use piney_desktop::message::{MENU_LAYER, MsgDraw, MsgWindow, WindowTexture, render};
use piney_desktop::view::View;
use piney_draw::Frame;

/// `ccMsgData.emode` the call passes: not 0x100 (typed from nothing).
pub const EMODE: i32 = 0x200;
/// A record's flag that shows it with Movie Text off.
pub const FLAG_ALWAYS: u32 = 0x800;
/// The call's layer: `ccLayer::Init(242, 0)`, the menu layer's priority.
pub const LAYER: i16 = MENU_LAYER;
/// `ccGetStreamDemoMsg`'s answers besides a line: -1 close, -2 nothing new.
pub const MSG_CLOSE: i32 = -1;
pub const MSG_NONE: i32 = -2;

/// One `evStrMsgTbl` record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrMsg {
    pub flag: u32,
    /// None for a NULL name.
    pub name: Option<Vec<u8>>,
    /// `ccKanjiStrSeparate(text, 0..2)`: the three lines (an empty one is
    /// still a line, a pointer to an empty string).
    pub lines: [Vec<u8>; 3],
}

/// The records of `evStrMsgTbl[num]` (`evStrMsgTblp` with `parody`; INF
/// main 0x003112f0, 0x00311510, 134 pointers each), None when the stream
/// has no table. A note past its table reads the records after it (the
/// next stream's, as the game's pointer walks; the highest line any
/// Infection note asks for is 27): the generated table holds them on, as
/// far as they read as records with text.
pub fn read_table(volume: Volume, num: usize, parody: bool) -> Result<Option<Vec<StrMsg>>> {
    if num >= crate::table::COUNT {
        return Err(Error::NotFound(format!("stream {num}")));
    }
    let t = stream::of(volume);
    let tables = if parody { t.subtitles_parody() } else { t.subtitles() };
    let Some(rows) = tables.get(num).copied().flatten() else { return Ok(None) };
    let line = |l: &[&str], k: usize| l.get(k).map_or_else(Vec::new, |s| sjis::encode(s));
    Ok(Some(
        rows.iter()
            .map(|r| {
                let l = r.str.unwrap_or_default();
                StrMsg {
                    flag: r.emode as u32,
                    name: r.name.map(sjis::encode),
                    lines: [line(l, 0), line(l, 1), line(l, 2)],
                }
            })
            .collect(),
    ))
}

/// What the window is drawn with: the volume's fonts and `DATA.BIN`'s
/// `xwindow` texture (both resident while a stream plays).
pub struct Look {
    fonts: Fonts,
    texture: Option<WindowTexture>,
}

impl Look {
    pub fn read(volume: piney_data::volume::Volume, data: &Archive) -> Result<Look> {
        Ok(Look { fonts: read_fonts(volume, data)?, texture: WindowTexture::read(data) })
    }
}

/// `ccEventStream`'s window over one stream.
pub struct Subtitles {
    records: Vec<StrMsg>,
    /// `saveData.strWinMode` (+0x8430), the Movie Text option.
    movie_text: bool,
    names: Names,
    window: MsgWindow,
    look: Option<Look>,
    /// The call has returned: the window is closed and no longer drawn.
    closed: bool,
}

impl Subtitles {
    /// The window for stream `num` as the save sets it up (Movie Text,
    /// Parody Mode, the names `#0` and `#1` stand for); None when the
    /// stream has no table (the call then makes no window). Without a
    /// [`Look`] the passes still run but [`Subtitles::frame`] draws nothing.
    pub fn read(volume: Volume, num: usize, save: &SaveData) -> Result<Option<Subtitles>> {
        let Some(records) = read_table(volume, num, save.parody())? else { return Ok(None) };
        Ok(Some(Subtitles::new(
            records,
            save.u8(offset::STR_WIN_MODE) != 0,
            Names { name: save.name().to_vec(), real: save.real_name().to_vec() },
        )))
    }

    pub fn new(records: Vec<StrMsg>, movie_text: bool, names: Names) -> Subtitles {
        Subtitles { records, movie_text, names, window: MsgWindow::default(), look: None, closed: false }
    }

    /// Draw with `look`.
    pub fn with_look(mut self, look: Look) -> Subtitles {
        self.look = Some(look);
        self
    }

    pub fn records(&self) -> &[StrMsg] {
        &self.records
    }

    /// The `ccMessage`, for inspection.
    pub fn window(&self) -> &MsgWindow {
        &self.window
    }

    /// One pass of the call's loop while the stream plays: `msg` is what
    /// `ccGetStreamDemoMsg` answers this frame ([`crate::Stream::demo_msg`]);
    /// returns `Disp`'s calls.
    pub fn pass(&mut self, msg: i32) -> Vec<MsgDraw> {
        if self.closed {
            return Vec::new();
        }
        match msg {
            MSG_CLOSE => self.window.close(),
            MSG_NONE => {}
            m => {
                if let Some(r) = usize::try_from(m).ok().and_then(|m| self.records.get(m))
                    && (self.movie_text || r.flag & FLAG_ALWAYS != 0)
                {
                    let [a, b, c] = &r.lines;
                    self.window.change(EMODE, r.name.as_deref(), [Some(a), Some(b), Some(c)], &self.names);
                }
            }
        }
        // The page cursor (the only use of the frame rate) never shows.
        self.window.disp(1)
    }

    /// The stream has returned: `Close`, and nothing drawn again.
    pub fn close(&mut self) {
        if !self.closed {
            self.window.close();
            self.closed = true;
        }
    }

    pub fn closed(&self) -> bool {
        self.closed
    }

    /// A pass's draws as a frame of their own: layer 242's commands and the
    /// text's texture uploads. Layer 242 is above every layer a stream
    /// draws on, so they go after the stream's own.
    pub fn frame(&self, draws: &[MsgDraw]) -> Frame {
        let mut ctx = Ctx::new(View::default());
        if let Some(look) = &self.look {
            render(draws, &mut ctx, &look.fonts, &self.names, look.texture.as_ref());
        }
        ctx.finish()
    }

    /// [`Subtitles::frame`] added to the stream's frame `f`.
    pub fn draw_into(&self, draws: &[MsgDraw], f: &mut Frame) {
        let sub = self.frame(draws);
        f.uploads.extend(sub.uploads);
        f.cmds.extend(sub.cmds);
    }
}
