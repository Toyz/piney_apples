//! The port's quit prompt (not the game's): Escape opens the desktop menu's
//! own confirmation window (`ccDtMenu`'s Reset, menu 6, with the port's
//! lines, [`piney_desktop::dtmenu::QUIT_INFO`]) over whatever the game
//! shows. The game stands still under it: its last finished picture, read
//! back from the GS once, is shown under the window each frame, and nothing
//! else runs. (Drawing the game's frame again would not do: the desktop's
//! frames sample the previous picture, `TexRef::PreviousFrame`, which then
//! holds the window, and each frame fed it back into the next.) OK closes
//! the port; Cancel, or Escape again, goes back to the game.
//!
//! The window reads the pad as the game's menus do: the save's OK and
//! Cancel buttons, the directions to move. Enter (START on the keyboard)
//! counts as OK here.

use std::sync::Arc;

use piney_data::archive::Archive;
use piney_data::save::{SaveData, offset};
use piney_data::volume::Volume;
use piney_desktop::SaveState;
use piney_desktop::anm::Ctx;
use piney_desktop::dtmenu::{DtMenu, MenuCtx};
use piney_desktop::kanji::{Fonts, Names};
use piney_desktop::message::WindowTexture;
use piney_desktop::view::View;
use piney_draw::{
    AlphaTest, Cmd, Depth, DrawState, Filter, Frame, Prim, PrimKind, Rgba, Scissor, TexFunc, TexRef, TexState, Upload,
    UploadFormat, Vertex, Wrap,
};
use piney_input::{Buttons, Pad, Raw};

pub struct QuitPrompt {
    menu: DtMenu,
    save: SaveState,
    fonts: Fonts,
    pad: Pad,
    /// The game's picture as the prompt found it, as a frame that draws it
    /// under the window.
    held: Option<Frame>,
}

/// What a frame of the prompt came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Open,
    Quit,
    Back,
}

impl QuitPrompt {
    /// The window opening over the game. `save`: the game's, for its OK and
    /// Cancel buttons (a new game's when there is none yet).
    pub fn open(volume: Volume, archive: &Arc<Archive>, fonts: Fonts, save: Option<SaveData>) -> Self {
        let save = save.unwrap_or_else(default_save);
        let mut menu = DtMenu::new(volume, WindowTexture::read(archive), None);
        menu.open_quit();
        QuitPrompt { menu, save: SaveState::new(save), fonts, pad: Pad::default(), held: None }
    }

    /// Whether the game's picture has been kept yet.
    pub fn holding(&self) -> bool {
        self.held.is_some()
    }

    /// Keeps the game's last finished picture (`width` x `height` RGBA, as
    /// the GS reads it back; None: black) to draw under the window: one
    /// upload, 0, and one sprite over the screen, as the movie player
    /// draws a picture. The window's uploads are numbered after it.
    pub fn hold(&mut self, picture: Option<(u16, u16, Vec<u8>)>) {
        let mut f = Frame::new();
        f.clear = Rgba::BLACK;
        if let Some((w, h, mut rgba)) = picture {
            // PSMCT32's alpha is in GS units; DECAL without TCC ignores it.
            for px in rgba.as_chunks_mut::<4>().0 {
                px[3] = 0x80;
            }
            f.uploads.push(Upload {
                id: 0,
                width: w,
                height: h,
                format: UploadFormat::Psmct32,
                pixels: rgba,
                clut: Vec::new(),
            });
            let state = DrawState {
                blend: None,
                alpha_test: AlphaTest::Off,
                depth: Depth::NONE,
                texture: Some(TexState {
                    tex: TexRef::Upload(0),
                    func: TexFunc::Decal,
                    use_alpha: false,
                    filter: Filter::Linear,
                    wrap: Wrap::Clamp,
                }),
                scissor: Scissor::FULL,
            };
            let v = |x: f32, y: f32, u: f32, v: f32| Vertex { x, y, z: 0, u, v, rgba: Rgba::NEUTRAL };
            f.cmds.push(Cmd::Prim(Prim {
                kind: PrimKind::Sprite,
                gouraud: false,
                state,
                verts: vec![
                    v(0.0, 0.0, 0.5, 0.5),
                    v(f32::from(f.width), f32::from(f.height), f32::from(w) + 0.5, f32::from(h) + 0.5),
                ],
            }));
        }
        self.held = Some(f);
    }

    /// The held picture alone, drawn once as the prompt closes: the GS's
    /// previous picture is then the game's again, not the window's, for
    /// the game's first frame back to sample.
    pub fn backdrop(&self) -> Frame {
        self.held.clone().unwrap_or_default()
    }

    /// A frame of `ccThDtMenu` on `raw` (Enter as OK), with the sounds the
    /// window asks for.
    pub fn frame(&mut self, raw: &Raw) -> (Answer, Vec<i32>) {
        let mut raw = *raw;
        if raw.buttons.contains(Buttons::START) {
            raw.buttons = (raw.buttons & !Buttons::START) | Buttons(self.save.ok());
        }
        self.pad.read(&raw);
        let mut req = Vec::new();
        let mut x = MenuCtx {
            save: &mut self.save,
            req: &mut req,
            pad: &self.pad,
            enable_reset: false,
            names: Names::default(),
            save_sys: None,
        };
        self.menu.frame(&mut x);
        let se = req
            .into_iter()
            .filter_map(|r| match r {
                piney_desktop::Request::Se(piney_desktop::Se(n)) => Some(n),
                _ => None,
            })
            .collect();
        let answer = if self.menu.quit_ok {
            Answer::Quit
        } else if self.menu.closed() {
            Answer::Back
        } else {
            Answer::Open
        };
        (answer, se)
    }

    /// The picture: the game's kept one with the window drawn over it (its
    /// texts' uploads after the picture's, `Ctx::disp` numbering them by
    /// count).
    pub fn picture(&mut self, frame_rate: u32) -> Frame {
        let mut frame = self.held.clone().unwrap_or_default();
        let mut ctx = Ctx::new(View::default());
        ctx.uploads = std::mem::take(&mut frame.uploads);
        self.menu.disp(&mut ctx, &self.fonts, &Names::default(), frame_rate);
        let m = ctx.finish();
        frame.uploads = m.uploads;
        frame.cmds.extend(m.cmds);
        frame
    }
}

/// A save with `ccSaveData::Init`'s buttons (OK Cross, Cancel Circle), for
/// a prompt before any game.
fn default_save() -> SaveData {
    let mut s = SaveData::new();
    s.set_i16(offset::ASSIGN_PAD_OK, Buttons::CROSS.bits() as i16);
    s.set_i16(offset::ASSIGN_PAD_CANCEL, Buttons::CIRCLE.bits() as i16);
    s
}
