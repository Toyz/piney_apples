//! The chat balloons over the characters: `ccChatMsg` (main 0x001a6160-
//! 0x001a6c68, `ccChat`), which `ccMenuCtrl`'s constructor makes on the
//! menu's layer (its window on `menuIcon`'s texture) and its `Disp` draws
//! between the dim and the damage numbers. Four slots of up to 70 bytes of
//! text, each shown 120 frames and faded over the last 10; `CheckScope`
//! moves an overlapping balloon up until it meets none. The steps are in
//! docs/engine/field-ui.md (the chat balloons).

use piney_desktop::kanji::{Fonts, Kt, Names, str_width};

use crate::Request;
use crate::ctrl::{Ctx, Draw, MenuCtrl};
use crate::spr::{Obj, Spr};

/// `ccChatMsg::OpenChat(plw, text)` from the menus: the player's line,
/// also handed on as [`Request::OpenChat`].
pub fn open_player(m: &mut MenuCtrl, x: &mut Ctx, text: Vec<u8>) {
    if let Some(p) = &x.world.party[0] {
        m.chat.open(p.handle, &text, x.fonts, &x.save.names());
    }
    x.req.push(Request::OpenChat(text));
}

/// `cf` at an open: two seconds.
pub const CHAT_FRAMES: i16 = 120;
/// `scope[i][3]`: a balloon's height for `CheckScope`.
const HEIGHT: i32 = 30;
/// `ccSpriteColorTable` rows: the player's balloon, the others', the text.
const COL_PLAYER: usize = 18;
const COL_OTHER: usize = 1;
const COL_TEXT: usize = 7;
/// `data[i]` is 71 bytes.
const TEXT_MAX: usize = 70;

/// One of `ccChatMsg`'s four balloons.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChatSlot {
    /// `data[i]`, `cf[i]`, `cn[i]`.
    pub text: Vec<u8>,
    pub cf: i16,
    pub cn: i16,
    /// `chatChar[i]`: the speaker's handle ([`crate::CharInfo::handle`]).
    pub who: u32,
    /// `scope[i]`: x, y, width, height.
    pub scope: [i32; 4],
}

/// Where a speaker is this frame, as the runtime answers for `Disp`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChatAt {
    pub who: u32,
    /// `ccCheckTarget(ch)`: on a command list.
    pub listed: bool,
    /// `ccCalcTagPosChar(ch, p, (0, 0, 0.9 height), 0)` when it passes.
    pub at: Option<(i32, i32)>,
}

/// `ccChatMsg`.
#[derive(Clone, Debug)]
pub struct ChatMsg {
    pub slots: [ChatSlot; 4],
    /// +0x00 `window`: `SetPrim(48, 0)`, cells 14 x 16 from (0, 336) of
    /// `menuIcon`'s texture, 4 a row, alpha 96.
    pub window: Spr,
}

impl Default for ChatMsg {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatMsg {
    /// `ccChatMsg::ccChatMsg` (0x001a6160).
    pub fn new() -> Self {
        let mut window = Spr::new(Obj::ChatWindow, 48);
        window.su = 14;
        window.sv = 16;
        window.wu = 0;
        window.wv = 5376;
        window.wi = 4;
        window.alpha = 96;
        let mut slots: [ChatSlot; 4] = Default::default();
        for s in &mut slots {
            s.scope[3] = HEIGHT;
        }
        ChatMsg { slots, window }
    }

    /// `ccChatMsg::OpenChat(ch, text)` (0x001a67c0).
    pub fn open(&mut self, who: u32, text: &[u8], fonts: &Fonts, names: &Names) {
        if let Some(s) = self.slots.iter_mut().find(|s| s.who == who) {
            s.cf = 0;
        }
        if let Some(s) = self.slots.iter_mut().find(|s| s.cf == 0) {
            let end = text.iter().position(|&c| c == 0).unwrap_or(text.len());
            s.text = text[..end.min(TEXT_MAX)].to_vec();
            s.cf = CHAT_FRAMES;
            s.cn = str_width(fonts, &s.text, Kt::SmallProportional, names) as i16;
            s.who = who;
        }
    }

    /// `ccChatMsg::CloseChat()` (0x001a6900).
    pub fn close(&mut self) {
        for s in &mut self.slots {
            s.cf = 0;
        }
    }

    /// The speakers of the balloons still up, for the runtime's
    /// [`ChatAt`]s.
    pub fn speakers(&self) -> Vec<u32> {
        self.slots.iter().filter(|s| s.cf > 0).map(|s| s.who).collect()
    }

    /// `ccChatMsg::Disp(still)` (0x001a6320): the draws, in send order.
    /// `player` is the player's handle (`ccPartyManager.memberChar[0]`).
    pub fn disp(&mut self, still: bool, at: &[ChatAt], player: Option<u32>, draws: &mut Vec<Draw>) {
        let mut shown = 0u8;
        for (i, s) in self.slots.iter_mut().enumerate() {
            if s.cf > 0 && !still {
                s.cf -= 1;
            }
            if s.cf <= 0 {
                continue;
            }
            let Some(a) = at.iter().find(|a| a.who == s.who).filter(|a| a.listed) else {
                s.cf = 0;
                continue;
            };
            let Some((x, y)) = a.at else { continue };
            s.scope[0] = x - i32::from(s.cn) / 3;
            s.scope[1] = y - 16;
            s.scope[2] = i32::from(s.cn);
            shown |= 1 << i;
        }
        self.check_scope();
        for i in 0..4 {
            if shown & (1 << i) == 0 {
                continue;
            }
            let s = &self.slots[i];
            let col = if player == Some(s.who) { COL_PLAYER } else { COL_OTHER };
            let a = if s.cf < 10 { i32::from(s.cf) * 72 / 10 } else { 72 };
            let (x, y, n) = (s.scope[0], s.scope[1], i32::from(s.cn));
            let text = s.text.clone();
            self.draw_window(x, y, n, col, a);
            let c = piney_data::tables::kanji::SPRITE_COLOR_TABLE[COL_TEXT];
            let rgba = [c[0], c[1], c[2], (a * 128 / 72) as u8];
            draws.push(Draw::Text {
                obj: Obj::ChatKanji(i as u8),
                text,
                dx: x as f32,
                dy: y as f32,
                rgba,
                count: -1,
                kt: 0,
            });
        }
        let packets = self.window.take();
        draws.push(Draw::Send(packets));
    }

    /// `ccChatMsg::CheckScope()` (0x001a6690).
    fn check_scope(&mut self) {
        let (mut top, mut bottom) = (9999, -9999);
        for i in 0..4 {
            if self.slots[i].cf <= 0 {
                continue;
            }
            let h = self.slots[i].scope[3];
            let mut y = self.slots[i].scope[1];
            let mut bot = y + h;
            if y < bottom && top < bot {
                loop {
                    let hit = (0..i).find(|&j| {
                        let o = &self.slots[j];
                        o.cf > 0 && o.scope[1] < bot && y < o.scope[1] + o.scope[3]
                    });
                    let Some(j) = hit else { break };
                    bot = self.slots[j].scope[1];
                    y = bot - h;
                }
                self.slots[i].scope[1] = y;
            }
            top = top.min(y);
            bottom = bottom.max(bot);
        }
    }

    /// `ccChatMsg::DrawWindow(x, y, n, col, a)` (0x001a6940).
    fn draw_window(&mut self, x: i32, y: i32, n: i32, col: usize, a: i32) {
        let w = &mut self.window;
        w.set_colour(col);
        w.set_alpha(a);
        let cells = [
            (x - 14, y - 8, 14, 0),
            (x, y - 8, n, 1),
            (x + n, y - 8, 14, 2),
            (x - 14, y + 8, 14, 8),
            (x, y + 8, n, 9),
            (x + n, y + 8, 14, 10),
            (x + n / 3, y + 24, 14, 7),
        ];
        for (cx, cy, sx, code) in cells {
            w.sx = sx as f32;
            w.sy = 16.0;
            w.dx = cx as f32;
            w.dy = cy as f32;
            w.make_packet(code);
        }
        w.sx = 14.0;
        w.sy = 16.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fonts() -> Fonts {
        Fonts::of(piney_data::volume::Volume::Inf, Vec::new())
    }

    #[test]
    fn a_balloon_lasts_two_seconds_and_fades() {
        let mut c = ChatMsg::new();
        c.open(5, b"Hi!", &fonts(), &Names::default());
        assert_eq!(c.slots[0].cf, CHAT_FRAMES);
        let at = [ChatAt { who: 5, listed: true, at: Some((200, 100)) }];
        let mut draws = Vec::new();
        for _ in 0..CHAT_FRAMES - 1 {
            c.disp(false, &at, Some(0), &mut draws);
        }
        assert_eq!(c.slots[0].cf, 1);
        c.disp(false, &at, Some(0), &mut draws);
        assert_eq!(c.speakers(), Vec::<u32>::new());
    }

    #[test]
    fn overlapping_balloons_stack() {
        let mut c = ChatMsg::new();
        c.open(1, b"one", &fonts(), &Names::default());
        c.open(2, b"two", &fonts(), &Names::default());
        let at = [
            ChatAt { who: 1, listed: true, at: Some((200, 100)) },
            ChatAt { who: 2, listed: true, at: Some((210, 110)) },
        ];
        let mut draws = Vec::new();
        c.disp(false, &at, None, &mut draws);
        // The second moved up over the first.
        assert_eq!(c.slots[1].scope[1], c.slots[0].scope[1] - HEIGHT);
    }
}
