//! `ccThMother`'s pause while no controller is in port 1 (main, INF
//! 0x00167d28-0x00168018; MUT 0x00167ea8, OUT 0x00167130, QUA 0x00166fc0,
//! the same code on all four). The mother task runs first in a frame
//! (priority 2). In a playing mode (`eventMng.phase` at least 4), with
//! `game.pauseFlag` clear and `ccSys.pad[0].status` 0, it holds the screen,
//! puts the tasks to sleep, fades `ccGame.fade` and draws "P A U S E"; with
//! a controller back, the START line, and START ends it. See
//! docs/engine/overview.md ("The pause").

use piney_desktop::anm::Ctx;
use piney_desktop::sprite::Sprite;
use piney_desktop::view::LayerView;

use crate::fade::ScFade;

/// `EntryFade(1, 0, 0x58000000, 0, 0, 512, 384)` on `ccGame.fade`.
pub const FADE_TIME: i16 = 1;
pub const FADE_COLOUR: u32 = 0x5800_0000;
/// The font's colour: `ccSpriteColorTable[20]`, alpha 128.
pub const COLOUR: usize = 20;
/// The pad's START bit.
pub const START: u32 = 0x800;

/// What the mother does besides drawing, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    /// `fontOffFlip`, `game.layer->OffFlipExcept`: the screen holds.
    FlipOff,
    /// `ccSys.stillLayer.flipFlag` one down, not below 0.
    StillDown,
    /// `ccSleepNoSleepThread(1, 1)`: every task but the no-sleep ones.
    SleepAll,
    /// The event task's no-sleep bit (2) cleared, then `ccSleepThread`.
    SleepEvent,
    /// `DeleteFade(n)`, `ccWakeAllThread`.
    WakeAll,
    /// `fontOnFlip`, `game.layer->OnFlipExcept`.
    FlipOn,
    /// `ccWakeThread(event task)`, its bit 2 set again.
    WakeEvent,
}

/// A `MakePacketStr(text, 0)` on `fontSys` after `ccFont::SetType(kind)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    pub text: Vec<u8>,
    /// 1 (12 x 16 cells) for "P A U S E", 0 (8 x 12) for the START line.
    pub kind: i32,
    /// Where it starts, `n` cells of its type across and down.
    pub dx: f32,
    pub dy: f32,
}

/// One frame of the mother's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub calls: Vec<Call>,
    pub texts: Vec<Text>,
    /// The fade's elements drawn (`SendPacket`): col0, col1, cnt, tcnt.
    pub fades: Vec<(u32, u32, i16, i16)>,
    /// The other tasks, and the event task, run this frame.
    pub tasks: bool,
    pub event: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// The loop: the fade sent and the texts drawn each frame.
    Held,
    /// START was pushed and the tasks woken (`ccWakeAllThread` takes the
    /// event task's one sleep too): next frame the layers flip again and
    /// every task runs.
    Woken,
    Over,
}

/// The pause once begun.
#[derive(Clone, Debug, PartialEq)]
pub struct Pause {
    state: State,
    /// `ccGame.fade` (the pause's element and any the scene left) and the
    /// element `EntryFade` took.
    pub fade: ScFade,
    element: i32,
    /// "P A U S E" and the START line (`pause_texts`).
    texts: [Vec<u8>; 2],
}

/// The mother's test (INF 0x00167d28): a playing mode, `pauseFlag` clear,
/// no controller.
pub fn starts(playing: bool, pause_flag: bool, plugged: bool) -> bool {
    playing && !pause_flag && !plugged
}

/// `ccFont::SetType(t)` for the pause's two types: the cell's grid (`wu`,
/// `wv` in 1/16 texels, cells a row), its texels and its size.
pub fn font_type(s: &mut Sprite, t: i32) {
    (s.wu, s.wv, s.su, s.sv, s.wi, s.sx, s.sy) = font_cells(t);
}

/// `SetType(t)`'s fields: `wu`, `wv`, `su`, `sv`, cells a row, `sx`, `sy`.
pub fn font_cells(t: i32) -> (i32, i32, i32, i32, i32, f32, f32) {
    match t {
        1 => (0, 0, 12, 16, 16, 12.0, 16.0),
        _ => (2048, 2688, 8, 12, 16, 8.0, 12.0),
    }
}

impl Pause {
    /// The frame it begins: the screen held, the tasks asleep, the fade
    /// entered on `fade`, then the loop's first frame.
    pub fn begin(texts: [Vec<u8>; 2], fade: ScFade, plugged: bool, push: u32) -> (Pause, Frame) {
        let mut p = Pause { state: State::Held, fade, element: -1, texts };
        p.element = p.fade.entry_fade(FADE_TIME, 0, FADE_COLOUR);
        let mut f = p.frame(plugged, push);
        f.calls.splice(0..0, [Call::FlipOff, Call::StillDown, Call::SleepAll, Call::SleepEvent]);
        (p, f)
    }

    /// The event task runs again: the pause is gone.
    pub fn over(&self) -> bool {
        self.state == State::Over
    }

    /// One frame (INF 0x00167e08): the fade sent; "P A U S E" at 16 and 10
    /// cells of type 1; with a controller, the START line at 18 and 16
    /// cells of type 0, and START pushed ends it (`DeleteFade`, the tasks
    /// woken, one more frame breathed before the layers flip).
    pub fn frame(&mut self, plugged: bool, push: u32) -> Frame {
        match self.state {
            State::Woken => {
                self.state = State::Over;
                return Frame {
                    calls: vec![Call::FlipOn, Call::WakeEvent],
                    tasks: true,
                    event: true,
                    ..Frame::default()
                };
            }
            State::Over => return Frame { tasks: true, event: true, ..Frame::default() },
            State::Held => {}
        }
        let mut f = Frame { fades: self.fade.advance(), ..Frame::default() };
        let at = |kind: i32, x: f32, y: f32| {
            let (.., sx, sy) = font_cells(kind);
            (x * sx, y * sy)
        };
        let (dx, dy) = at(1, 16.0, 10.0);
        f.texts.push(Text { text: self.texts[0].clone(), kind: 1, dx, dy });
        if plugged {
            let (dx, dy) = at(0, 18.0, 16.0);
            f.texts.push(Text { text: self.texts[1].clone(), kind: 0, dx, dy });
            if push & START != 0 {
                if let Some(e) = usize::try_from(self.element).ok().and_then(|i| self.fade.elm.get_mut(i)) {
                    e.status = 0;
                }
                f.calls.push(Call::WakeAll);
                self.state = State::Woken;
            }
        }
        f
    }
}

/// `fontSys` as `fontInit` and `fontSetup` make it (main 0x0015c900,
/// 0x0015ca70): a 512-packet TF sprite on `xasc00`'s `TEX_xasc00`, its own
/// palette. It sends to `ccSys.stillLayer`, drawn here over the rest.
pub fn font(archive: &piney_data::archive::Archive) -> piney_data::Result<Sprite> {
    use piney_data::Error;
    let ccs = piney_data::ccs::Ccs::parse(archive.inflate_named("xasc00")?)?;
    let object = ccs.find_object("TEX_xasc00").ok_or_else(|| Error::NotFound("xasc00::TEX_xasc00".into()))?;
    let (textures, _) = piney_data::texture::read(&ccs)?;
    let t = textures.iter().find(|t| t.object == object).ok_or_else(|| Error::NotFound("TEX_xasc00".into()))?;
    let tex = piney_draw::TexRef::Ccs { file: "xasc00".into(), texture: object, clut: t.clut };
    Ok(Sprite::mask(piney_desktop::layers::FONT_LAYER, tex, 1 << t.th, 512))
}

/// The texts on `fontSys` (`ccSpriteColorTable[20]`, alpha 128), each a
/// `MakePacketStr(text, 0)`: a cell for each byte in 0x21-0x7f (code
/// `c - 0x20`), every byte a cell's width on.
pub fn draw_texts(ctx: &mut Ctx, font: &mut Sprite, texts: &[Text]) {
    let view = LayerView::default_layer();
    let t = piney_data::tables::kanji::shared().sprite_color_table[COLOUR];
    font.colour = [t[0], t[1], t[2], 0x80];
    for x in texts {
        font_type(font, x.kind);
        font.dx = x.dx;
        font.dy = x.dy;
        for &c in x.text.iter().take_while(|&&c| c != 0) {
            if (0x21..0x80).contains(&c) {
                font.make_packet(c - 0x20, &view);
            } else {
                font.dx = piney_desktop::eef::add(font.dx, font.sx);
            }
        }
    }
    font.send(&mut ctx.layers);
}

#[cfg(test)]
mod tests {
    use super::*;
    use piney_data::volume::Volume;

    /// The texts as each volume's `ccThMother` passes them to
    /// `MakePacketStr`: Infection's START line misspelt, fixed from Mutation.
    #[test]
    fn the_texts_are_each_volume_s() {
        for v in [Volume::Inf, Volume::Mut, Volume::Out, Volume::Qua] {
            let t = piney_data::tables::game::of(v).pause_texts;
            let want = if v == Volume::Inf { "START button: Retun to game" } else { "START button: Return to game" };
            assert_eq!(t, ["P A U S E", want], "{v:?}");
        }
    }

    /// Unplugged: the begin's calls and "P A U S E" alone; plugged, the
    /// START line; START: `WakeAll`, then a frame that flips back and runs
    /// every task, and the fade's element deleted.
    #[test]
    fn the_pause_runs_to_start() {
        let texts = [b"P A U S E".to_vec(), b"START".to_vec()];
        let (mut p, f) = Pause::begin(texts, ScFade::default(), false, 0);
        assert_eq!(f.calls, [Call::FlipOff, Call::StillDown, Call::SleepAll, Call::SleepEvent]);
        assert_eq!((f.texts.len(), f.tasks, f.event, f.fades.len()), (1, false, false, 1));
        assert_eq!((f.texts[0].dx, f.texts[0].dy), (192.0, 160.0));
        let f = p.frame(true, 0);
        assert_eq!((f.texts.len(), f.texts[1].dx, f.texts[1].dy), (2, 144.0, 192.0));
        assert!(f.calls.is_empty());
        let f = p.frame(true, START);
        assert_eq!((f.calls.as_slice(), f.tasks), ([Call::WakeAll].as_slice(), false));
        let f = p.frame(true, 0);
        assert_eq!((f.calls.as_slice(), f.tasks, f.event), ([Call::FlipOn, Call::WakeEvent].as_slice(), true, true));
        assert!(p.over() && p.fade.elm.iter().all(|e| e.status == 0));
    }
}
