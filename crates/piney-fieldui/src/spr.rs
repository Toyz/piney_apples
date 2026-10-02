//! The sprites `ccMenuCtrl` owns, as the menu code drives them: a
//! `ccSprite`'s fields set one by one, then `MakePacket` (a cell of the
//! grid) or `MakePacketStr` (a string of cells) queues a packet, and
//! `SendPacket` hands the queue to the layer. [`Spr`] keeps exactly the
//! fields the code writes, and each queued [`Packet`] a copy of them at the
//! call, so the port can be compared call for call with the game and drawn
//! afterwards by [`crate::render`].

use piney_data::tables::kanji::SPRITE_COLOR_TABLE;

/// Which of `ccMenuCtrl`'s sprites (and the globals it draws with).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Obj {
    /// +0x5c `menuWindow`: a `ccMenuWindow` on `menuWin`'s texture
    /// (`xwindow::TEX_xwindo00`), 192 packets.
    MenuWindow,
    /// +0x60 `menuWindowPr`: the party panels' frames, 64 packets.
    MenuWindowPr,
    /// +0x64 `menuWindowA`: additive (alphaBlendType 1), 48 packets.
    MenuWindowA,
    /// +0x68 `eneLife`: enemy life bars and the battle banners, 32 packets.
    EneLife,
    /// +0x6c `targetCursol`: the target cursor, rotated sprites, 32 packets.
    TargetCursol,
    /// +0x70 `menuKanji`: the menu's rows (`ccKanji` Init(3, 24)).
    MenuKanji,
    /// +0x74 `menuKanjiPr` (Init(3, 24)).
    MenuKanjiPr,
    /// +0x78 `nameKanji`: names and the HUD's texts (Init(3, 12)).
    NameKanji,
    /// +0x7c `nameKanjiPr`: the party panels' names (Init(3, 12)).
    NameKanjiPr,
    /// +0x80 `settingKanji[8]` (Init(3, 32)).
    Setting(u8),
    /// +0xb0 `menuFont`: `ccFont` on `fontTex`, 256 packets.
    MenuFont,
    /// +0xb4 `flyFont`: damage numbers and the enemy bars' labels, 160.
    FlyFont,
    /// +0xbc `menuFace[4]`: the party's faces (`xwin_f00::TEX_xwin_fNN`).
    MenuFace(u8),
    /// +0xcc `menuIcon`: `xwindow::TEX_xicon00`.
    MenuIcon,
    /// +0xd0 `itemIcon` (menuIcon's texture), 40 packets.
    ItemIcon,
    /// +0xd4 `conIcon`: condition icons (menuIcon's texture), 84 packets.
    ConIcon,
    /// +0xd8 `menuBg`: the dim behind a menu, `xwin_bg::TEX_xwin_bg0`.
    MenuBg,
    /// +0xdc `menuDrain`: the bracelet's erosion, `xwin_vir`.
    MenuDrain,
    /// +0xe0 `menuProtect`: `xwin_vir` too.
    MenuProtect,
    /// +0xe4 `menuMask`.
    MenuMask,
    /// The gate hack's `hackMask` (`ccHackMenu` +0x90): the slot's core and
    /// count, `xdhhack::TEX_xdhroma1`, on the mask layer (128).
    HackMask,
    /// The global `font` (a `ccFont` on `fontTex`), sent at the flip on the
    /// font layer: the gate hack's core counts.
    SysFont,
    /// `ccChat->window`: the chat balloons' frames, on `menuIcon`'s texture
    /// ([`crate::chat_msg`]).
    ChatWindow,
    /// `ccChat->chat[i]`: a balloon's text.
    ChatKanji(u8),
    /// `ccDfComp`'s sprite (+0x24): the banner of an area whose portals
    /// are all opened, `xwindow::TEX_xwindo01` ([`crate::dfcomp`]).
    DfComp,
    /// A Ryu Book's ([`crate::book`]): `bg`, the cover over the screen
    /// (`str8800e`'s `TEX_x800bac1`); `win`, the page's window; `button`,
    /// on its own layer (243); `title` and `msg[140]`, its kanji rows.
    BookBg,
    BookWin,
    BookButton,
    BookTitle,
    BookMsg(u8),
}

impl Obj {
    /// The name the trace and the harness use.
    pub fn name(self) -> String {
        match self {
            Obj::MenuWindow => "win".into(),
            Obj::MenuWindowPr => "winPr".into(),
            Obj::MenuWindowA => "winA".into(),
            Obj::EneLife => "eneLife".into(),
            Obj::TargetCursol => "target".into(),
            Obj::MenuKanji => "kanji".into(),
            Obj::MenuKanjiPr => "kanjiPr".into(),
            Obj::NameKanji => "name".into(),
            Obj::NameKanjiPr => "namePr".into(),
            Obj::Setting(i) => format!("set{i}"),
            Obj::MenuFont => "font".into(),
            Obj::FlyFont => "fly".into(),
            Obj::MenuFace(i) => format!("face{i}"),
            Obj::MenuIcon => "icon".into(),
            Obj::ItemIcon => "itemIcon".into(),
            Obj::ConIcon => "conIcon".into(),
            Obj::MenuBg => "bg".into(),
            Obj::MenuDrain => "drain".into(),
            Obj::MenuProtect => "protect".into(),
            Obj::MenuMask => "mask".into(),
            Obj::HackMask => "hackmask".into(),
            Obj::SysFont => "sysfont".into(),
            Obj::ChatWindow => "chat".into(),
            Obj::ChatKanji(i) => format!("chat{i}"),
            Obj::DfComp => "dfcomp".into(),
            Obj::BookBg => "bookBg".into(),
            Obj::BookWin => "bookWin".into(),
            Obj::BookButton => "bookButton".into(),
            Obj::BookTitle => "bookTitle".into(),
            Obj::BookMsg(i) => format!("bookMsg{i}"),
        }
    }
}

/// One queued cell or string, with the sprite's fields at the call.
#[derive(Clone, Debug, PartialEq)]
pub struct Packet {
    pub obj: Obj,
    /// The cell (`MakePacket(code, 1)`), or 0 for a string.
    pub code: u8,
    /// `MakePacketStr(str, 0)`: the string, drawn a cell a byte.
    pub text: Option<Vec<u8>>,
    pub dx: f32,
    pub dy: f32,
    pub sx: f32,
    pub sy: f32,
    pub cx: f32,
    pub cy: f32,
    pub rot: f32,
    pub su: i32,
    pub sv: i32,
    pub wu: i32,
    pub wv: i32,
    pub wi: i32,
    /// `color[0]`'s r, g, b and a, the low byte of each word.
    pub rgba: [u8; 4],
    /// ctrl 0x20: mirrored left to right.
    pub flip: bool,
    /// ctrl 0x40: mirrored top to bottom.
    pub flip_v: bool,
    /// A Gouraud sprite's (`SetPrim(m, 2)`) four corners' r, g, b, a
    /// (`color[0..7]`); `None` for the flat ones.
    pub vcol: Option<[[u8; 4]; 4]>,
    /// ctrl 0x10: a dark copy a unit right and down.
    pub shadow: bool,
}

/// A `ccSprite` (or `ccMenuWindow`, `ccKanji`, `ccFont`) as the menu sets
/// it.
#[derive(Clone, Debug)]
pub struct Spr {
    pub obj: Obj,
    /// `ctrl`'s 0x20 bit (FacePanelDisp sets and clears it).
    pub flip: bool,
    /// `ctrl`'s 0x40 bit (the Controller page's arrows).
    pub flip_v: bool,
    pub dx: f32,
    pub dy: f32,
    pub sx: f32,
    pub sy: f32,
    pub cx: f32,
    pub cy: f32,
    pub rot: f32,
    pub su: i32,
    pub sv: i32,
    pub wu: i32,
    pub wv: i32,
    pub wi: i32,
    /// r, g, b as the colour words' low bytes.
    pub rgb: [u8; 3],
    /// The alpha word as stored (`color[0][1] >> 32`).
    pub alpha: i32,
    /// `packetMax`: MakePacket stops queueing past it.
    pub packet_max: usize,
    pub queue: Vec<Packet>,
    /// A Gouraud sprite's corners' r, g, b (the alpha is `alpha`).
    pub vcol: Option<[[u8; 3]; 4]>,
    /// ctrl 0x10 (the gate screens set it on the font and leave it).
    pub shadow: bool,
}

impl Spr {
    /// `new ccSprite` + `SetPrim(m, t)`: colour 128 grey, alpha 128,
    /// nothing queued.
    pub fn new(obj: Obj, packet_max: usize) -> Self {
        Spr {
            obj,
            flip: false,
            flip_v: false,
            dx: 0.0,
            dy: 0.0,
            sx: 0.0,
            sy: 0.0,
            cx: 0.0,
            cy: 0.0,
            rot: 0.0,
            su: 0,
            sv: 0,
            wu: 0,
            wv: 0,
            wi: 1,
            rgb: [128, 128, 128],
            alpha: 128,
            packet_max,
            queue: Vec::new(),
            vcol: None,
            shadow: false,
        }
    }

    /// `color = ccSpriteColorTable[i]` (r, g, b; the alpha kept).
    pub fn set_colour(&mut self, i: usize) {
        let c = SPRITE_COLOR_TABLE[i];
        self.rgb = [c[0], c[1], c[2]];
    }

    /// `color` from a packed 0xAABBGGRR word (r, g, b; the alpha kept).
    pub fn set_rgb_u32(&mut self, c: u32) {
        self.rgb = [c as u8, (c >> 8) as u8, (c >> 16) as u8];
    }

    /// `color[0][1] = b | a << 32`.
    pub fn set_alpha(&mut self, a: i32) {
        self.alpha = a;
    }

    /// `ccMenuWindow::SetType`-style grid: cells `su` x `sv` texels drawn
    /// `sx` x `sy`, from (wu, wv) in 1/16 texels, `wi` a row.
    #[allow(clippy::too_many_arguments)]
    pub fn set_grid(&mut self, su: i32, sv: i32, sx: f32, sy: f32, wu: i32, wv: i32, wi: i32) {
        self.su = su;
        self.sv = sv;
        self.sx = sx;
        self.sy = sy;
        self.wu = wu;
        self.wv = wv;
        self.wi = wi;
    }

    fn packet(&self, code: u8, text: Option<Vec<u8>>) -> Packet {
        Packet {
            obj: self.obj,
            code,
            text,
            dx: self.dx,
            dy: self.dy,
            sx: self.sx,
            sy: self.sy,
            cx: self.cx,
            cy: self.cy,
            rot: self.rot,
            su: self.su,
            sv: self.sv,
            wu: self.wu,
            wv: self.wv,
            wi: self.wi,
            rgba: [self.rgb[0], self.rgb[1], self.rgb[2], self.alpha as u8],
            flip: self.flip,
            flip_v: self.flip_v,
            vcol: self.vcol.map(|v| v.map(|c| [c[0], c[1], c[2], self.alpha as u8])),
            shadow: self.shadow,
        }
    }

    /// `MakePacket(code, 1)`: the cell at (dx, dy); dx moves on by sx.
    /// Code 0xff is the raw string's end: nothing is drawn.
    pub fn make_packet(&mut self, code: i32) {
        let code = code as u8;
        if code == 0xff {
            return;
        }
        // Packets past `packetMax` are recorded; the renderer drops them.
        let p = self.packet(code, None);
        self.queue.push(p);
        self.dx = piney_desktop::eef::add(self.dx, self.sx);
    }

    /// `MakePacketStr(str, 0)`: a cell a byte from (dx, dy); dx moves on by
    /// sx a byte.
    pub fn make_str(&mut self, s: &[u8]) {
        let p = self.packet(0, Some(s.to_vec()));
        self.queue.push(p);
        for _ in 0..s.len() {
            self.dx = piney_desktop::eef::add(self.dx, self.sx);
        }
    }

    /// `SendPacket`: the queue, emptied.
    pub fn take(&mut self) -> Vec<Packet> {
        std::mem::take(&mut self.queue)
    }
}

/// `ccFont::SetType(t)` (0x0015cc20): the cell grid of `fontTex`.
pub fn font_type(s: &mut Spr, t: i32) {
    match t {
        0 => s.set_grid(8, 12, 8.0, 12.0, 2048, 2688, 16),
        1 => s.set_grid(12, 16, 12.0, 16.0, 0, 0, 16),
        2 => s.set_grid(10, 12, 10.0, 12.0, 2048, 1536, 12),
        3 => s.set_grid(14, 16, 14.0, 16.0, 3072, 0, 4),
        _ => {}
    }
}

/// `dec2str(n, v, buf, 0)` (0x0015cec0): `n` digits of `v`, the leading
/// zeros but the last as spaces.
pub fn dec2str(n: i32, v: u64, zero: bool) -> Vec<u8> {
    let n = n.max(0) as usize;
    let mut out = vec![0u8; n];
    let mut v = v;
    for i in (0..n).rev() {
        out[i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    if v == 0 && !zero {
        for b in out.iter_mut().take(n.saturating_sub(1)) {
            if *b != b'0' {
                break;
            }
            *b = b' ';
        }
    }
    out
}

/// `ccFont::MakeNum(0, n, v)` (0x0015cd40): `v` in `n` digits (at most
/// 127), leading zeros blank, as a string of cells.
pub fn make_num(s: &mut Spr, n: i32, v: i32) {
    let n = n.min(127);
    // The value goes to dec2str as an unsigned 64-bit long (sign-extended).
    let text = dec2str(n, v as i64 as u64, false);
    s.make_str(&text);
}

/// `sdec2str(n, v, buf, 0)` (main 0x0015cfc0): a sign cell then `n`
/// digits of `v`; leading zeros but the last blank, the sign (a space, or
/// `-`) just before the first digit; a value too wide keeps its sign in
/// front.
pub fn sdec2str(n: i32, v: i64) -> Vec<u8> {
    let n = n.max(0) as usize;
    let sign = if v < 0 { b'-' } else { b' ' };
    let mut v = v.wrapping_abs();
    let mut out = vec![b'0'; n + 1];
    for i in (1..=n).rev() {
        out[i] = b'0'.wrapping_add((v % 10) as u8);
        v /= 10;
    }
    if v != 0 {
        out[0] = sign;
        return out;
    }
    let mut s = 0;
    while s < n && out[s] == b'0' {
        out[s] = b' ';
        s += 1;
    }
    // The first cell is always a '0' blanked above, so s >= 1.
    out[s.max(1) - 1] = sign;
    out
}

/// `ccFont::MakeSignedNum(n, v)` (main 0x0015cde0): `v` with its sign in
/// `n` digits (at most 126), as a string of cells.
pub fn make_signed_num(s: &mut Spr, n: i32, v: i64) {
    let n = if n >= 127 { 126 } else { n };
    let text = sdec2str(n, v);
    s.make_str(&text);
}

/// `ccKanji::SetClm(c, u, v, wi)` (0x0015edb0) for a kt 0 / 1 kanji
/// (8 x 16 glyph cells): a row of `c` glyphs.
pub fn set_clm(s: &mut Spr, c: i32, u: i32, v: i32, wi: i32) {
    let (w, h) = (8, 16);
    s.sx = (c * w) as f32;
    s.su = c * w;
    s.sy = h as f32;
    s.sv = h;
    s.wu = (u * w) << 4;
    s.wv = (v * h) << 4;
    s.wi = wi;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dec2str_blanks_leading_zeros() {
        assert_eq!(dec2str(3, 7, false), b"  7");
        assert_eq!(dec2str(3, 0, false), b"  0");
        assert_eq!(dec2str(4, 1234, false), b"1234");
        assert_eq!(dec2str(2, 105, false), b"05");
    }

    #[test]
    fn sdec2str_puts_the_sign_before_the_digits() {
        assert_eq!(sdec2str(3, 5), b"   5");
        assert_eq!(sdec2str(3, -5), b"  -5");
        assert_eq!(sdec2str(3, 0), b"   0");
        assert_eq!(sdec2str(3, 123), b" 123");
        assert_eq!(sdec2str(3, -120), b"-120");
        assert_eq!(sdec2str(2, -999), b"-99");
    }

    #[test]
    fn packets_advance() {
        let mut s = Spr::new(Obj::MenuWindow, 4);
        s.sx = 14.0;
        s.make_packet(3);
        s.make_packet(4);
        assert_eq!(s.dx, 28.0);
        assert_eq!(s.queue.len(), 2);
        assert_eq!(s.queue[1].dx, 14.0);
    }
}
