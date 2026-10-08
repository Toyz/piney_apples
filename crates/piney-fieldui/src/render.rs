//! The frame's draws as GS primitives on the menu layer (242): each sprite
//! queue a TF SPRITE a cell over its texture, the kanji rows over their
//! extracted text, the message window as the desktop draws it.
//!
//! Every send goes to the front of the layer (`ccSprite::SendPacket`
//! prepends), so the last sent is drawn first.

use piney_data::archive::Archive;
use piney_desktop::anm::Ctx as DrawCtx;
use piney_desktop::kanji::{Fonts, Kanji, Kt, Names, expand, extract};
use piney_desktop::message::{MENU_LAYER, WindowTexture, menu_view};
use piney_desktop::sprite::Sprite;
use piney_desktop::view::View;
use piney_draw::{Frame, TexRef};

use crate::ctrl::Draw;
use crate::spr::{Obj, Packet};
use crate::tables::Texts;

/// The textures the menu's sprites cut their cells from.
#[derive(Clone, Debug, Default)]
pub struct Textures {
    /// `menuWin`: `xwindow::TEX_xwindo00` (menuWindow, menuWindowPr,
    /// menuWindowA, eneLife, targetCursol copy it).
    pub window: Option<WindowTexture>,
    /// `menuIcon`: `xwindow::TEX_xicon00` (itemIcon, conIcon).
    pub icon: Option<WindowTexture>,
    /// `menuBg`: `xwin_bg::TEX_xwin_bg0`.
    pub bg: Option<WindowTexture>,
    /// `menuDrain`, `menuProtect`: `xwin_vir::TEX_xwin_vir`.
    pub vir: Option<WindowTexture>,
    /// `fontTex`: `xasc00::TEX_xasc00` (menuFont, flyFont).
    pub font: Option<WindowTexture>,
    /// The faces by `menuFaceCcsList` row.
    pub faces: Vec<Option<WindowTexture>>,
    /// `menuMask` after the Controller page's `SetTex`: `flContMenu`'s
    /// picture (`xcontrol::TEX_xcontrol`), the only thing the menu draws
    /// with menuMask.
    pub control: Option<WindowTexture>,
    /// The gate hack's `hackMask`: `xdhhack::TEX_xdhroma1`.
    pub hack_mask: Option<WindowTexture>,
    /// `ccDfComp`'s: `xwindow::TEX_xwindo01`.
    pub df_comp: Option<WindowTexture>,
    /// A Ryu Book's cover: `str8800e::TEX_x800bac1` (`BOOK::bg`).
    pub book_bg: Option<WindowTexture>,
}

impl Textures {
    pub fn read(archive: &Archive, texts: &Texts) -> Textures {
        let t = |f: &str, n: &str| WindowTexture::read_named(archive, f, n, None);
        Textures {
            window: WindowTexture::read(archive),
            icon: t("xwindow", "TEX_xicon00"),
            bg: t("xwin_bg", "TEX_xwin_bg0"),
            vir: t("xwin_vir", "TEX_xwin_vir"),
            font: t("xasc00", "TEX_xasc00"),
            faces: texts.faces.iter().map(|(f, n)| t(f, n)).collect(),
            control: t(&texts.option.control_file, &texts.option.control_tex),
            hack_mask: t(crate::menus::hack::HACK_FILE, "TEX_xdhroma1"),
            df_comp: t("xwindow", "TEX_xwindo01"),
            book_bg: t("str8800e", "TEX_x800bac1"),
        }
    }
}

fn texture<'a>(t: &'a Textures, obj: Obj, faces: &[i32; 4]) -> Option<&'a WindowTexture> {
    match obj {
        Obj::MenuWindow
        | Obj::MenuWindowPr
        | Obj::MenuWindowA
        | Obj::EneLife
        | Obj::TargetCursol
        | Obj::BookWin
        | Obj::BookButton => t.window.as_ref(),
        Obj::BookBg => t.book_bg.as_ref(),
        Obj::MenuMask => t.control.as_ref(),
        Obj::MenuIcon | Obj::ItemIcon | Obj::ConIcon | Obj::ChatWindow => t.icon.as_ref(),
        Obj::MenuBg => t.bg.as_ref(),
        Obj::MenuDrain | Obj::MenuProtect => t.vir.as_ref(),
        Obj::MenuFont | Obj::FlyFont | Obj::SysFont => t.font.as_ref(),
        Obj::HackMask => t.hack_mask.as_ref(),
        Obj::DfComp => t.df_comp.as_ref(),
        Obj::MenuFace(i) => {
            let f = faces[i as usize];
            if f < 0 { None } else { t.faces.get(f as usize).and_then(|x| x.as_ref()) }
        }
        _ => None,
    }
}

/// The layer a sprite sends on: the menu's, but the gate hack's two.
fn layer(obj: Obj) -> i16 {
    match obj {
        Obj::HackMask => crate::menus::hack::MASK_LAYER,
        Obj::BookButton => crate::book::BUTTON_LAYER,
        Obj::SysFont => piney_desktop::layers::FONT_LAYER,
        _ => MENU_LAYER,
    }
}

/// The view a sprite's layer has: the menu's (`SetFrame(0, 0, 512, 384,
/// 256, 192, 1, 6/7)`), but for the layers made with no view of their own
/// (`ccLayer::Init(n, NULL)`: `SetFrame(0, 0, 512, 384, 256, 192, 1, 1)`),
/// `fontInit`'s `fontLayer` (the global `font`) and the gate hack's mask
/// layer 128.
fn view_of(obj: Obj, menu: &piney_desktop::view::LayerView) -> piney_desktop::view::LayerView {
    match obj {
        Obj::SysFont | Obj::HackMask => piney_desktop::view::LayerView::frame(0.0, 0.0, 512.0, 384.0, 1.0, 1.0),
        _ => *menu,
    }
}

fn blend(obj: Obj) -> usize {
    match obj {
        Obj::MenuWindowA => 1,
        _ => 0,
    }
}

/// Not the game's: one axis of a packet under the HUD scale `k` about its
/// anchor `a`: (position, size, offset), the offset folded into the
/// position. At `k` 1, or with no anchor, the packet's own.
fn shrink(a: Option<f32>, k: f32, d: f32, size: f32, c: f32) -> (f32, f32, f32) {
    match a {
        Some(a) if k != 1.0 => (a + k * (d + c - a), size * k, 0.0),
        _ => (d, size, c),
    }
}

/// One packet through a sprite: a cell, or a string of cells; `k` the HUD
/// scale ([`crate::spr::Anchor`]).
fn packet(s: &mut Sprite, p: &Packet, view: &piney_desktop::view::LayerView, k: f32) {
    let (dx, sx, cx) = shrink(p.anchor.x, k, p.dx, p.sx, p.cx);
    let (dy, sy, cy) = shrink(p.anchor.y, k, p.dy, p.sy, p.cy);
    let pulled = |a: Option<f32>| if a.is_some() && k != 1.0 { 0.5 } else { 0.0 };
    s.uv_inset = [pulled(p.anchor.x), pulled(p.anchor.y)];
    s.wu = p.wu;
    s.wv = p.wv;
    s.wi = p.wi;
    s.su = p.su;
    s.sv = p.sv;
    s.sx = sx;
    s.sy = sy;
    s.cx = cx;
    s.cy = cy;
    s.dx = dx;
    s.dy = dy;
    s.flip_u = p.flip;
    s.shadow = p.shadow;
    s.colour = p.rgba;
    // targetCursol is the one rotated sprite (SetPrim type TRF); the
    // Gouraud ones (the drain gauge's) carry their corners' colours.
    s.rot = (p.obj == Obj::TargetCursol).then_some(p.rot);
    s.vcol = p.vcol;
    match &p.text {
        None => s.make_packet(p.code, view),
        Some(t) => {
            for &c in t {
                if (0x21..0x80).contains(&c) {
                    s.make_packet(c - 0x20, view);
                } else {
                    s.dx = piney_desktop::eef::add(s.dx, s.sx);
                }
            }
        }
    }
}

/// A queue with cells mirrored top to bottom (ctrl 0x40, which
/// `MakePacketStr` does by swapping the quad's two V rows): each packet
/// made alone, its SPRITE's V swapped when flipped, all sent as the one
/// group `SendPacket` makes.
fn send_flipped(s: &Sprite, packets: &[Packet], view: &piney_desktop::view::LayerView, ctx: &mut DrawCtx, k: f32) {
    let mut group = Vec::new();
    for p in packets {
        let mut one = s.clone();
        packet(&mut one, p, view, k);
        let mut layers = piney_desktop::layers::Layers::default();
        one.send(&mut layers);
        for mut c in layers.flatten() {
            if p.flip_v
                && let piney_draw::Cmd::Prim(prim) = &mut c
                && prim.verts.len() == 2
            {
                let v0 = prim.verts[0].v;
                prim.verts[0].v = prim.verts[1].v;
                prim.verts[1].v = v0;
            }
            group.push(c);
        }
    }
    ctx.layers.prepend(MENU_LAYER, group);
}

/// The frame of the menu layer.
pub fn frame(draws: &[Draw], textures: &Textures, fonts: &Fonts, names: &Names, faces: &[i32; 4], k: f32) -> Frame {
    let mut ctx = DrawCtx::new(View::default());
    draw(draws, textures, fonts, names, faces, &mut ctx, k);
    ctx.finish()
}

/// The menu layer's packets sent into a frame the rest of the game is
/// building (its uploads numbered after the ones already there); `k` the
/// HUD scale ([`crate::FieldUi::hud_scale`]).
pub fn draw(
    draws: &[Draw],
    textures: &Textures,
    fonts: &Fonts,
    names: &Names,
    faces: &[i32; 4],
    ctx: &mut DrawCtx,
    hud: f32,
) {
    let view = menu_view();
    for d in draws {
        match d {
            Draw::Msg(m) => piney_desktop::message::render(m, ctx, fonts, names, textures.window.as_ref()),
            Draw::Send(packets) => {
                let Some(first) = packets.first() else { continue };
                let Some(tex) = texture(textures, first.obj, faces) else { continue };
                let mut s = Sprite::mask(layer(first.obj), tex.tex.clone(), tex.tex_h, 4096);
                s.alpha_blend = blend(first.obj);
                let view = view_of(first.obj, &view);
                if packets.iter().any(|p| p.flip_v) {
                    send_flipped(&s, packets, &view, ctx, hud);
                    continue;
                }
                for p in packets {
                    packet(&mut s, p, &view, hud);
                }
                s.send(&mut ctx.layers);
            }
            Draw::Hack(h) => {
                // ccLayer::active = the screen's layer, its view set to the
                // camera animation's camera; each animation drawn there.
                let mut v = View::default();
                if let Some(cam) = h.cam.camera {
                    v.set_camera(&cam);
                }
                let l = crate::menus::hack::HACK_LAYER;
                h.back.draw_on(ctx, l, &v);
                for a in [&h.crystals, &h.frame, &h.comp].into_iter().flatten() {
                    a.draw_on(ctx, l, &v);
                }
            }
            Draw::Noiz(cmds) => ctx.layers.prepend(piney_desktop::noiz::NOIZ_LAYER, cmds.clone()),
            Draw::Fade { c0, c1, cnt, tcnt } => {
                piney_desktop::fade::draw_on(ctx, MENU_LAYER, *c0, *c1, (*cnt).max(0) as u32, (*tcnt).max(0) as u32);
            }
            Draw::Text { text, dx, dy, rgba, count, kt, .. } => {
                let mut k = Kanji::init(3, 24);
                match *kt {
                    1 => k.kt = Kt::SmallFixed,
                    2 => k.kt = Kt::LargeProportional,
                    _ => {}
                }
                k.colour = *rgba;
                k.dx = *dx;
                k.dy = *dy;
                ctx.disp_count(fonts, &mut k, MENU_LAYER, &view, text, names, *count);
            }
            Draw::Kanji { text, packets, kt, .. } => {
                if packets.is_empty() {
                    continue;
                }
                let mut k = Kanji::init(3, 24);
                let strn = expand(text, names);
                let kt = if *kt == 3 { Kt::LargeFixed } else { Kt::SmallProportional };
                k.clm = extract(fonts, &strn, kt, k.th, &mut k.tex);
                let id = ctx.uploads.len() as u32;
                ctx.uploads.push(k.upload(id, fonts));
                let mut s = Sprite::mask(MENU_LAYER, TexRef::Upload(id), k.th as i32, 4096);
                for p in packets {
                    packet(&mut s, p, &view, hud);
                }
                s.send(&mut ctx.layers);
            }
        }
    }
}
