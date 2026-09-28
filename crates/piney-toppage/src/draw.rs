//! The draw calls the top page makes, each recorded for `trace` as the
//! game's side of the check sees it: a labelled `ccAnm`, `ccKanji::Disp`
//! and the board's mask (`ccSprite::MakePacket` / `SendPacket`).

use std::rc::Rc;

use piney_desktop::anm::{Anm, Ctx};
use piney_desktop::assets::SceneFile;
use piney_desktop::kanji::{Fonts, Kanji, Names};
use piney_desktop::layers::BACK_LAYER;
use piney_desktop::sprite::Sprite;
use piney_desktop::view::LayerView;

/// `ccLayer::active` while the top page runs: `sysLayer` (0x00379b10,
/// priority 0), which `ccThToppage` makes active. Every `ccAnm` draws
/// there, and so does a `ccKanji` with no layer of its own. The port's
/// animations draw into [`BACK_LAYER`]; it stands for `sysLayer` here,
/// below everything else the top page draws, as priority 0 is.
pub const ACTIVE_LAYER: i16 = BACK_LAYER;

/// `ccThBBSCtrl::m_lpLayer`: `ccLayer::Init(127, NULL)`.
pub const BBS_LAYER: i16 = 127;

/// `m_lpLayer`'s view: `SetFrame(0, 0, 512, 448, 256, 224, 1, 6/7)`
/// (`ccThBBSCtrl::Init` 0x00401c3c), one unit a frame-buffer pixel.
pub fn bbs_view() -> LayerView {
    LayerView::frame(0.0, 0.0, 512.0, 448.0, 1.0, 6.0 / 7.0)
}

/// `sysLayer`'s view: `InitCCSys`'s `SetFrame(0, 0, 512, 384, 256, 192,
/// 1, 1)`.
pub fn active_view() -> LayerView {
    LayerView::default_layer()
}

/// A `ccAnm` with the member name the trace records it by.
#[derive(Clone)]
pub struct Anim {
    pub anm: Anm,
    pub label: &'static str,
}

impl Anim {
    pub fn new(label: &'static str) -> Self {
        Anim { anm: Anm::new(), label }
    }

    /// `ccAnm::SetAnm(ccsc, name, 0)`.
    pub fn set(&mut self, file: &Rc<SceneFile>, name: &str) {
        #[cfg(feature = "trace")]
        crate::trace::record("set", self.label, || name.to_string());
        self.anm.set(file, name);
    }

    /// `if (anmIndex) r = _AnimateForward(frameSpd)`: one step; true when a
    /// play-once animation has ended.
    pub fn step(&mut self) -> bool {
        if !self.anm.is_set() {
            return false;
        }
        let r = self.anm.forward();
        #[cfg(feature = "trace")]
        crate::trace::record("fwd", self.label, || {
            format!("{} {} {}", self.anm.frame_spd, i32::from(r), self.anm.frame_now())
        });
        r
    }

    /// `ccAnm::Draw()`.
    pub fn draw(&self, ctx: &mut Ctx) {
        #[cfg(feature = "trace")]
        crate::trace::record("draw", self.label, String::new);
        self.anm.draw(ctx);
    }

    /// `ccAnm.frameNow` (+0x98).
    pub fn frame_now(&self) -> u32 {
        self.anm.frame_now()
    }
}

/// A `ccKanji` with its label.
#[derive(Clone, Debug)]
pub struct Text {
    pub k: Kanji,
    pub label: &'static str,
}

impl Text {
    /// `new ccKanji; Init(3, 16)`: a 128-row texture, 16 packets, the
    /// drop shadow on.
    pub fn new(label: &'static str) -> Self {
        Text { k: Kanji::init(3, 16), label }
    }

    /// `ccKanji::Disp(s, -1)` on `layer` through `view`: rasterised and
    /// sent at once.
    pub fn disp(&mut self, ctx: &mut Ctx, fonts: &Fonts, names: &Names, layer: i16, view: &LayerView, s: &[u8]) {
        #[cfg(feature = "trace")]
        crate::trace::record("disp", self.label, || {
            let [r, g, b, a] = self.k.colour;
            let ctrl = 1 | if self.k.shadow { 0x10 } else { 0 };
            let hex: String = s.iter().map(|c| format!("{c:02x}")).collect();
            format!(
                "{:08x} {:08x} {r} {g} {b} {a} {ctrl} {} {hex}",
                self.k.dx.to_bits(),
                self.k.dy.to_bits(),
                self.k.packet_max
            )
        });
        ctx.disp(fonts, &mut self.k, layer, view, s, names);
    }
}

/// One `MakePacket(0, 1)` of the board's mask: the cell (`wu`, `wv` in
/// 1/16 texels, `su` x `sv` texels, one a row) drawn `sx` x `sy` at
/// (`dx`, `dy`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub wu: i32,
    pub wv: i32,
    pub su: i32,
    pub sv: i32,
    pub sx: f32,
    pub sy: f32,
    pub dx: f32,
    pub dy: f32,
}

/// Sets the mask's cell and queues it (`ccSprite::MakePacket(0, 1)`).
pub fn packet(mask: &mut Sprite, view: &LayerView, c: Cell) {
    (mask.wu, mask.wv, mask.wi, mask.su, mask.sv) = (c.wu, c.wv, 1, c.su, c.sv);
    (mask.sx, mask.sy, mask.dx, mask.dy) = (c.sx, c.sy, c.dx, c.dy);
    #[cfg(feature = "trace")]
    crate::trace::record("pkt", "mask", || {
        format!(
            "{} {} {} {} {} {:08x} {:08x} {:08x} {:08x}",
            c.wu,
            c.wv,
            1,
            c.su,
            c.sv,
            c.sx.to_bits(),
            c.sy.to_bits(),
            c.dx.to_bits(),
            c.dy.to_bits()
        )
    });
    mask.make_packet(0, view);
}

/// `ccSprite::SendPacket()` of the mask.
pub fn send(mask: &mut Sprite, ctx: &mut Ctx) {
    #[cfg(feature = "trace")]
    crate::trace::record("send", "mask", String::new);
    mask.send(&mut ctx.layers);
}
