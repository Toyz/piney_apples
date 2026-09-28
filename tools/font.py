#!/usr/bin/env python3
"""How the game turns a string into glyph quads - see docs/engine/font.md.

    tools/font.py glyphs   ELF [--font F]             the glyph table of one font, or all
    tools/font.py extended ELF                        ccGetExtendedCode, and what every %x draws
    tools/font.py measure  ELF TEXT [--name S] [--real-name S]
                                                      ccKanjiStrWidth, ccKanjiStrlen, and what
                                                      ccKanji::Disp actually draws
    tools/font.py render   ELF TEXT OUT.png [--font F] [--width PX] [--center] [--scale N]
                                                      draw it as the game would
    tools/font.py sheet    ELF OUTDIR                 the two bitmap fonts as PNG
    tools/font.py check    ELF [--sample N] [--disp N]
                                                      the game's own text functions, run in
                                                      tools/eemu.py, against these

TEXT is Unicode turned into Shift-JIS (Δ is 83 a2), with \\n between lines and
\\xNN for a raw byte; or @mail:N, @mail:N:title, @remail:N, @bbs:T:P,
@news:N, @event:E:M to take it from the game's tables (--parody for the twin).
DATA.BIN, for the palette and TEX_xasc00, is looked for next to ELF (--data).

F names a font: aN is ccFont type N, kN is a ccKanji with kt N (N alone is kN).

ccFont (fontSys, fontDef, font; drawn by ccSprite::MakePacketStr 0x0015aed0)
is fixed-cell ASCII out of TEX_xasc00 (xasc00.cmp, 256x256 4-bit).
ccFont::SetType (0x0015cc20) picks the grid:

    type  cell   grid origin  cells a row   holds
    0     8x12   (128, 168)   16            ASCII 0x20-0x7f, small
    1     12x16  (0, 0)       16            ASCII 0x20-0x7f, 0x7f is a Δ
    2     10x12  (128, 96)    12            ASCII 0x20-0x60, capitals only
    3     14x16  (192, 0)     4             big digits: ' ', '!'-'*' = 0-9, then
                                            + , - . / 0 1 2 3 4 5 6 7 = M I S E X P L V D O W N -

A byte c in 0x21-0x7f draws cell c - 0x20; every other byte, 0x0a included,
only advances. The advance is the cell width: no kerning, no escapes. (The
function sets a newline flag for 0x0a and then skips it as a control
character; only the raw-code mode MakePacket uses, where 0xff ends and 0xfe
starts a line, can break a line.) Cells are addressed from the top of the
upright image; the stored rows are bottom-up, and the V coordinates run
downwards from (256 - v) * 16 - 1, which puts them back the right way up.

ccKanji (font.cpp) is the proportional text of mail, the board, dialogue and
menus. kt is 0 after ccKanji::Init; the call sites store 2 in ccMessage (dialogue),
ccLoadDisp (loading screens) and BOOK, 3 in name entry and the staff roll, 1 in
BOOK's pages, and leave 0 for chat, WORLD, DUNGEON, the mailer, the board and
most menus. Disp (0x0015e380) expands #0/#1 and maps Shift-JIS into a work buffer,
then Extract (0x0015d490) rasterises it into the sprite's own 128-texel-wide
4-bit texture from bitmap fonts compiled into the executable - ef8x16
(0x002f5600, kt 0/1, 8x16 cells) or ef12x20 (0x002f2180, kt 2/3, 12x20) - 16
or 10 glyphs to a texture row, 112 glyphs each; Disp then draws it as one
SPRITE per run of glyphs that share a colour and a texture row, with a drop
shadow one pixel right and down in (16,16,16). kt 0 and 2 are proportional:
glyph g advances cell - englishFontOfsS[g] (0x002fb5f0) or cell -
englishFontOfsL[g] (0x002fb4f0) and is copied that far left; kt 1 and 3 are
fixed, with the glyph moved right by 2 * (ofs / 4) texels. Glyph numbers:

    ASCII c               c - 0x20
    %0-%9                 95-104   Δ Λ Σ Ω Θ ○ △ □ ×, and %9 %A make a ★
    %A-%Z                 105-130  %B %C make a ⊘, %D ‘, %E ®, %F %G blank;
                                   %H on lies past the 112 drawn glyphs
    %%  %#                '%'  '#'
    other %x              nothing
    any other byte        63, '_'

Both paths use CLT_xasc00: 0 clear, 1-3 white, 4-15 a black-to-white ramp,
A = 0x80. TEX0 has TFX MODULATE and TCC on, the blend is (Cs - Cd) * As + Cd,
and the colour escapes of ccKanji (#R #G #B #Y, #W back) set the vertex
colour from ccSpriteColorTable (0x002fb430), keeping the alpha.

The game does no wrapping: a mail body, a board post and a message are NUL-
separated lines, and ccKanjiStrSeparate(m, n) (0x0015f860) only finds line
n. ccKanjiStrWidth (0x0015f2a0) is used to centre (ccMessage: 256 - w / 2).

The addresses above are Infection's. ELF may be any of the four volumes:
where the fonts, the trim tables, the escape colours and the text tables are
(and how many mails and replies there are) is read from that executable's
own code, so the stripped volumes need no names for them. The fonts, trims
and colours are the same on all four; Outbreak's and Quarantine's
ccKanjiStrlen counts every %x pair, not only %% and %# (strlen_counted).
"""

import argparse
import bisect
import collections
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from image import Program  # noqa: E402

# ccFont::SetType (0x0015cc20): sx, sy (draw size, float), su, sv (texels per
# cell), wu, wv (grid origin in texels; the game keeps them x16), wi (cells a row)
FONT_TYPES = {
    0: (8, 12, 8, 12, 128, 168, 16),
    1: (12, 16, 12, 16, 0, 0, 16),
    2: (10, 12, 10, 12, 128, 96, 12),
    3: (14, 16, 14, 16, 192, 0, 4),
}
BIG_DIGITS = " 0123456789MISEXPLVDOWN-"     # type 3, cell n <- byte 0x20 + n

# ccGetExtendedCode (0x0015d390): the whole switch. Anything else comes back
# unchanged, which every caller treats as "no glyph".
EXTENDED = {
    0x83a2: 0x2530,     # Δ -> %0
    0x83a9: 0x2531,     # Λ -> %1
    0x83b0: 0x2532,     # Σ -> %2
    0x83b6: 0x2533,     # Ω -> %3
    0x83a6: 0x2534,     # Θ -> %4
    0x819b: 0x2535,     # ○ -> %5
    0x81a2: 0x2536,     # △ -> %6
    0x81a0: 0x2537,     # □ -> %7
    0x817e: 0x2538,     # × -> %8
    0x8165: 0x2544,     # ‘ -> %D
}

# What glyphs 95-111 show in both bitmap fonts (read off ef8x16 and ef12x20).
# The star and the no-entry sign are two glyphs wide.
EXTRA_NAMES = ["Δ", "Λ", "Σ", "Ω", "Θ", "○", "△", "□", "×", "★ left", "★ right", "⊘ left",
               "⊘ right", "‘", "®", "", ""]

KFont = collections.namedtuple("KFont", "cw ch per_row bitmap stride table proportional")
# ccKanji.kt -> font. Disp and Extract test kt == 0 / 1 for the small font and
# kt == 0 / 2 for the proportional copy; everything else is large and fixed.
KANJI = {
    0: KFont(8, 16, 16, "ef8x16", 64, "englishFontOfsS", True),
    1: KFont(8, 16, 16, "ef8x16", 64, "englishFontOfsS", False),
    2: KFont(12, 20, 10, "ef12x20", 96, "englishFontOfsL", True),
    3: KFont(12, 20, 10, "ef12x20", 96, "englishFontOfsL", False),
}
GLYPHS = 112            # drawn glyphs in each bitmap font
TEX_W = 64              # bytes a row of a ccKanji texture: 128 texels at 4 bits

# ccSpriteColorTable entries the escapes use, by table offset / 8, as
# Infection's Disp has them (all four volumes agree). Fonts reads each escape's
# colour from where the executable's own Disp loads it (escape_addresses).
ESCAPE_COLOURS = {ord("R"): 18, ord("G"): 20, ord("B"): 17, ord("Y"): 6}
SHADOW = (16, 16, 16)   # MakePacketStr's shadow colour (ctrl & 0x10)

# GS screen: 512x448 around (2048, 2048). MakePacketStr culls a quad that
# lies wholly outside it.
SCREEN_X, SCREEN_Y = 0x7000, 0x7200
CULL_X1, CULL_Y1 = 0x9000, 0x8e00


def kfont(kt):
    if kt in KANJI:
        return KANJI[kt]
    return KANJI[3]     # the code's default branches: large, fixed


def c_div(a, b):
    """C integer division, truncating towards zero."""
    q = abs(a) // abs(b)
    return q if (a < 0) == (b < 0) else -q


def extended_code(code):
    return EXTENDED.get(code & 0xffff, code & 0xffff)


def percent_glyph(d):
    """The glyph a %x pair draws, or None when it draws nothing."""
    if 0x30 <= d <= 0x39:
        return d + 47
    if 0x41 <= d <= 0x5a:
        return d + 40
    if d in (0x25, 0x23):
        return d - 0x20
    return None


def glyph_name(g):
    if 0 <= g < 95:
        return chr(0x20 + g)
    if 95 <= g < GLYPHS:
        return EXTRA_NAMES[g - 95]
    return "?"


# ---------------------------------------------------------------------------
# Where the fonts and tables are, read from each executable's own code. The
# later volumes are stripped and carry Infection's names only where
# piney-gen syms matched them (the bitmap fonts, for one, are named nowhere),
# so the code that uses a thing says where it is.

def _words(p, va):
    """The function at va: its symbol's extent, else up to the first
    jr $ra and its delay slot."""
    f = p.function_at(va)
    if f is not None and f.value == va:
        n = f.size
    else:
        n = 0
        while p.u32(va + n) != 0x03e00008:
            n += 4
        n += 8
    return struct.unpack(f"<{n // 4}I", p.read(va, n))


def _formed(p, va):
    """(words, [(instruction index, opcode, address)]) for every absolute
    address the function at va forms with lui and a low half, in code order."""
    import xfer
    words = _words(p, va)
    a = xfer.addresses(words, va, None)
    return words, [(i, words[i] >> 26, a[i]) for i in sorted(a)]


def _fn(p, name):
    s = p.symbol_named(name)
    if s is None:
        raise KeyError(f"{p.path}: no {name}")
    return s.value


def font_addresses(p):
    """{name: VA} of the two bitmap fonts and the two trim tables. ccKanji::
    Extract forms them with lui + addiu: ef8x16 and englishFontOfsS for kt 0
    and 1, then ef12x20 and englishFontOfsL for the rest."""
    _, formed = _formed(p, _fn(p, "Extract__7ccKanjiFPc"))
    got = []
    for _, op, a in formed:
        if op == 0x09 and a not in got:
            got.append(a)
    if len(got) != 4:
        raise ValueError(f"{p.path}: ccKanji::Extract forms {len(got)} addresses, not 4")
    return dict(zip(("ef8x16", "englishFontOfsS", "ef12x20", "englishFontOfsL"), got))


def escape_addresses(p):
    """{escape byte: VA of the u64 colour it loads}, from ccKanji::Disp's test
    of the byte after '#': li reg, 'X' then beq on reg, and the case loads its
    colour with lui + ld. #W, which loads nothing, restores the starting
    colour; any other byte keeps the colour."""
    words, formed = _formed(p, _fn(p, "Disp__7ccKanjiFPciff"))
    loads = {i: a for i, op, a in formed if op == 0x37}
    out = {}
    for i, w in enumerate(words):
        imm, rt = w & 0xffff, (w >> 16) & 31
        if w >> 26 != 0x09 or (w >> 21) & 31 or not 0x41 <= imm <= 0x5a:
            continue
        for j in range(i + 1, min(i + 3, len(words))):
            b = words[j]
            if b >> 26 == 0x04 and rt in ((b >> 21) & 31, (b >> 16) & 31):
                off = b & 0xffff
                t = j + 1 + (off - 0x10000 if off & 0x8000 else off)
                hit = [loads[k] for k in range(t, t + 4) if k in loads]
                if hit:
                    out[imm] = hit[0]
                break
    return out


def _step(regs, w):
    """One instruction of a path walk over known register values (regs):
    constants, the two string bytes, a little arithmetic; anything else
    written becomes unknown. -> 1 for addiu r, r, 1 (a count), else 0."""
    op, rs, rt, imm = w >> 26, (w >> 21) & 31, (w >> 16) & 31, w & 0xffff
    simm = imm - 0x10000 if imm & 0x8000 else imm
    if op in (0x02, 0x03) or (op == 0 and (w & 63) in (0x08, 0x09)):
        raise ValueError("the walk left the function")
    if op in (0x01, 0x06, 0x07) or 0x14 <= op <= 0x17:
        raise ValueError("a branch the walk does not follow")
    if op == 0:
        rd, fn = (w >> 11) & 31, w & 63
        if w and rd:
            regs[rd] = regs.get(rs) if fn in (0x21, 0x2d, 0x25) and rt == 0 else None
    elif op == 0x09:
        regs[rt] = None if regs.get(rs) is None else regs[rs] + simm
        return int(rs == rt != 0 and simm == 1)
    elif op == 0x0c:
        regs[rt] = None if regs.get(rs) is None else regs[rs] & imm
    elif op == 0x0f:
        regs[rt] = imm << 16
    elif op in (0x20, 0x24):
        regs[rt] = regs["str"][simm] if rs != 28 and simm in (0, 1) else None
        if op == 0x20 and regs[rt] is not None and regs[rt] >= 0x80:
            regs[rt] -= 0x100
    elif 0x08 <= op <= 0x0f or 0x20 <= op <= 0x27 or op == 0x37:
        regs[rt] = None
    regs[0] = 0
    return 0


def strlen_counted(p):
    """The bytes x for which ccKanjiStrlen counts a %x pair as a glyph. Each
    x is read by walking the executable's own code for the string "%x" from
    the loop head (the first lb r, 0(s)) back to it, counting the
    addiu r, r, 1 on the way. Infection's and Mutation's test for '%' and '#';
    Outbreak's and Quarantine's count every pair."""
    words = _words(p, _fn(p, "ccKanjiStrlen__FPc"))
    head = next(i for i, w in enumerate(words) if w >> 26 in (0x20, 0x24) and not w & 0xffff)
    out = bytearray()
    for x in range(256):
        regs = {0: 0, "str": (0x25, x)}
        pc = head
        n = 0
        for _ in range(200):
            w = words[pc]
            op, rs, rt, imm = w >> 26, (w >> 21) & 31, (w >> 16) & 31, w & 0xffff
            if op in (0x04, 0x05):
                a, b = regs.get(rs), regs.get(rt)
                if a is None or b is None:
                    raise ValueError(f"{p.path}: ccKanjiStrlen branches on an unknown value")
                n += _step(regs, words[pc + 1])         # the delay slot
                pc = pc + 1 + (imm - 0x10000 if imm & 0x8000 else imm) if (a == b) == (op == 0x04) \
                    else pc + 2
                if pc <= head:
                    break
                continue
            n += _step(regs, w)
            pc += 1
        else:
            raise ValueError(f"{p.path}: ccKanjiStrlen's %x path does not loop")
        if n:
            out.append(x)
    return bytes(out)


class Fonts:
    """The font data out of the executable, and TEX_xasc00 / CLT_xasc00 out of
    DATA.BIN when it is there."""

    def __init__(self, elf, data=None):
        self.elf = elf
        self.p = Program(elf)
        self.va = font_addresses(self.p)
        # The escape colours, at the addresses Disp loads them from; and the
        # table they sit in, where the escapes agree with ESCAPE_COLOURS.
        self.escape_va = escape_addresses(self.p)
        self.strlen_counted = strlen_counted(self.p)
        self.escape = {c: tuple(self.p.read(a, 4)) for c, a in self.escape_va.items()}
        bases = {a - 8 * ESCAPE_COLOURS[c] for c, a in self.escape_va.items() if c in ESCAPE_COLOURS}
        ct = self.va["ccSpriteColorTable"] = bases.pop() if len(bases) == 1 else None
        self.colours = [tuple(self.p.read(ct + 8 * i, 4)) for i in range(24)] if ct else None
        # Glyph numbers run to 130 (%Z). Past the 112 glyphs and the 128-entry
        # tables the game reads whatever follows in memory; so does this, from
        # the image, a page at a time, wherever the volume put things.
        self._pages = {}
        self.data = data or os.path.join(os.path.dirname(os.path.abspath(elf)), "DATA", "DATA.BIN")
        self._xasc = None

    def _page(self, n):
        pg = self._pages.get(n)
        if pg is None:
            pg = self._pages[n] = self.p.read(n << 12, 0x1000)
        return pg

    def s16(self, va):
        return struct.unpack("<h", self.row(va, 2))[0]

    def trim(self, kt, g):
        return self.s16(self.va[kfont(kt).table] + 2 * g)

    def glyph_src(self, kt, g):
        """VA of glyph g's top row in its bitmap."""
        f = kfont(kt)
        base = self.va[f.bitmap]
        return base + (g >> 4) * f.stride * f.ch + (g & 15) * (f.cw // 2)

    def byte(self, va):
        return self._page(va >> 12)[va & 0xfff]

    def row(self, va, n):
        o = va & 0xfff
        if o + n <= 0x1000:
            return self._page(va >> 12)[o:o + n]
        out = bytearray()
        while n > 0:
            k = min(n, 0x1000 - (va & 0xfff))
            out += self._page(va >> 12)[va & 0xfff:(va & 0xfff) + k]
            va += k
            n -= k
        return bytes(out)

    def xasc(self):
        """(texture, palette): TEX_xasc00 as 256 rows of 256 palette indices in
        stored (bottom-up) order, which is VRAM order; CLT_xasc00 as RGBA."""
        if self._xasc is None:
            import ccs
            import ccstex
            c = ccs.Ccs(ccs.load(self.data + "::xasc00.cmp"))
            textures, cluts = ccstex.read(c)
            tx = textures[0]
            w, h = tx.size
            pixels = tx.levels[0][2]
            idx = bytearray(w * h)
            for i in range(w * h):
                b = pixels[i >> 1]
                idx[i] = (b >> 4) if i & 1 else b & 15
            pal = [tuple(col) for col in cluts[tx.clut].colours]
            self._xasc = (Texture(w, h, idx), pal)
        return self._xasc


class Texture:
    """w x h palette indices, row 0 first as in VRAM."""

    def __init__(self, w, h, idx):
        self.w, self.h, self.idx = w, h, idx

    @classmethod
    def from4(cls, data, w, h):
        idx = bytearray(w * h)
        for i in range(w * h):
            b = data[i >> 1]
            idx[i] = (b >> 4) if i & 1 else b & 15
        return cls(w, h, idx)

    def texel(self, u, v):
        return self.idx[(v % self.h) * self.w + (u % self.w)]


# ---------------------------------------------------------------------------
# The string functions, each as the game has it (font.cpp).

def _pad(raw):
    # The game reads a byte or two past a trailing '#', '%' or lead byte; zeros
    # there are what a string followed by more zeros gives.
    return bytes(raw) + b"\0\0\0"


def expand(raw, name=b"Kite", real=b"Player"):
    """ccKanji::Disp's first pass (0x0015e380): #0 and #1 become the names,
    #x and %x are kept, a mapped Shift-JIS pair becomes its %x, and any other
    byte outside 0x20-0x7f becomes '_' - one byte only, so the trail byte of an
    unmapped pair is read again. The game's buffer is char[82] with no check."""
    s = _pad(raw)
    out = bytearray()
    i = 0
    while s[i]:
        c = s[i]
        if c == 0x23 and s[i + 1] == 0x30:
            out += name
            i += 2
        elif c == 0x23 and s[i + 1] == 0x31:
            out += real
            i += 2
        elif c in (0x23, 0x25):
            out += s[i:i + 2]
            i += 2
        elif 0x20 <= c < 0x80:
            out.append(c)
            i += 1
        else:
            e = extended_code(c << 8 | s[i + 1])
            if e >> 8 == 0x25:
                out += bytes((0x25, e & 0xff))
                i += 2
            else:
                out.append(0x5f)
                i += 1
    return bytes(out)


def str_width(fonts, raw, kt, name=b"Kite", real=b"Player"):
    """ccKanjiStrWidth(m, kt) (0x0015f2a0), in pixels. kt 1 counts 8 and kt 3
    counts 12 a glyph; kt 0 uses 8 - englishFontOfsS, anything else 12 -
    englishFontOfsL. Quirk: for a Shift-JIS pair that maps to %A-%Z it tests
    the pair's own trail byte for A-Z, so ‘ (81 65 -> %D) counts 0."""
    def adv(g):
        if kt == 1:
            return 8
        if kt == 0:
            return 8 - fonts.trim(0, g)
        if kt == 3:
            return 12
        return 12 - fonts.trim(2, g)

    s = _pad(raw)
    w = 0
    i = 0
    while s[i]:
        c = s[i]
        if c == 0x23:
            if s[i + 1] == 0x30:
                w += str_width(fonts, name, kt, name, real)
            elif s[i + 1] == 0x31:
                w += str_width(fonts, real, kt, name, real)
            i += 2
        elif c == 0x25:
            if kt == 1:
                w += 8
            elif kt == 3:
                w += 12
            else:
                g = percent_glyph(s[i + 1])
                if g is not None:
                    w += adv(g)
            i += 2
        elif 0x20 <= c < 0x80:
            w += adv(c - 0x20)
            i += 1
        else:
            e = extended_code(c << 8 | s[i + 1])
            if e >> 8 == 0x25:
                lo = e & 0xff
                if kt == 1:
                    w += 8
                elif kt == 3:
                    w += 12
                elif 0x30 <= lo <= 0x39:
                    w += adv(lo + 47)
                elif 0x41 <= s[i + 1] <= 0x5a:
                    w += adv(lo + 40)
                i += 2
            else:
                w += adv(63)
                i += 1
    return w


def str_len(raw, name=b"Kite", real=b"Player", counted=b"%#"):
    """ccKanjiStrlen (0x0015f120): glyphs. Quirk: a %x pair counts 1 only
    when x is in `counted`, the executable's own set (Fonts.strlen_counted):
    Infection's and Mutation's count only %% and %#, so %0-%9 and %A-%Z count
    nothing, though a Shift-JIS pair that maps to one counts 1; Outbreak's
    and Quarantine's count every pair."""
    s = _pad(raw)
    n = 0
    i = 0
    while s[i]:
        c = s[i]
        if c == 0x23:
            if s[i + 1] == 0x30:
                n += str_len(name, name, real, counted)
            elif s[i + 1] == 0x31:
                n += str_len(real, name, real, counted)
            i += 2
        elif c == 0x25:
            if s[i + 1] in counted:
                n += 1
            i += 2
        elif 0x20 <= c < 0x80:
            n += 1
            i += 1
        else:
            n += 1
            i += 2 if extended_code(c << 8 | s[i + 1]) >> 8 == 0x25 else 1
    return n


def str_separate(buf, n, start=0):
    """ccKanjiStrSeparate(m, n) (0x0015f860): the offset of line n in NUL-
    separated lines. A byte outside 0x20-0x7f takes the next byte with it."""
    i = start
    for _ in range(n):
        while True:
            c = buf[i]
            i += 1
            if c == 0:
                break
            if not 0x20 <= c < 0x80:
                i += 1
    return i


def strcat_field(raw, m, name=b"Kite", real=b"Player"):
    """What ccKanjiStrcat(s0, s1, m) (0x0015eed0) appends to s0: s1 cut or
    padded with spaces to m glyphs, names expanded, Shift-JIS mapped (or '_'),
    #x kept and not counted. raw None is a null s1: m spaces."""
    out = bytearray()
    t = 0
    s = None if raw is None else _pad(raw)
    i = 0
    while t < m:
        if s is None or s[i] == 0:
            out.append(0x20)
            t += 1
            continue
        c = s[i]
        if c == 0x23 and s[i + 1] in (0x30, 0x31):
            src = name if s[i + 1] == 0x30 else real
            for b in src:
                if t >= m or b == 0:
                    break
                out.append(b)
                t += 1
            i += 2
        elif c == 0x23:
            out += s[i:i + 2]
            i += 2
        elif c == 0x25:
            out += s[i:i + 2]
            i += 2
            t += 1
        elif 0x20 <= c < 0x80:
            out.append(c)
            i += 1
            t += 1
        else:
            e = extended_code(c << 8 | s[i + 1])
            if e >> 8 == 0x25:
                out += bytes((0x25, e & 0xff))
                i += 2
            else:
                out.append(0x5f)
                i += 1
            t += 1
    # a trailing '#' or '%' copies the NUL after it, which ends the C string
    return bytes(out).split(b"\0")[0]


def extract(fonts, strn, kt, th=128, tex=None):
    """ccKanji::Extract (0x0015d490) over an expanded string: rasterise it into
    a 128 x th 4-bit texture (bytes in VRAM order, rows bottom-up as the game
    lays them). -> (texture bytes, glyphs extracted, which is ccKanji.clm)."""
    f = kfont(kt)
    small = kt in (0, 1)
    rows = 16 if small else 20
    lines = th // rows
    cwb = f.cw // 2                         # bytes a cell
    per_row = 128 // f.cw                   # 16 or 10
    base = 0 if small else 8 * TEX_W        # 6 x 20 rows of 128: the top 120
    if tex is None:
        tex = bytearray(TEX_W * th)
    s = _pad(strn)
    i = c = l = aflp = clm = 0
    last_font = None

    def row0():
        """Byte offset of the current text line's top pixel row."""
        return base + (lines - l) * rows * TEX_W - TEX_W

    def tail():
        # the rest of the texture row, filled with byte 0 of each row of glyph 0
        src = fonts.va[last_font.bitmap]
        dst = row0() + c * cwb - c_div(aflp, 2)
        n = c_div(aflp, 2) + 64 - c * cwb
        if dst - (rows - 1) * TEX_W < 0 or dst + n > len(tex):
            return      # after a wild %X-%Z trim, as above
        for r in range(rows):
            fill = fonts.byte(src + r * last_font.stride)
            d = dst - r * TEX_W
            for j in range(n):
                tex[d + j] = fill

    while s[i]:
        code = extended_code(s[i] << 8 | s[i + 1])
        hi, lo = code >> 8, code & 0xff
        if hi == 0x23:
            i += 2
            continue
        if hi == 0x25:
            i += 2
            g = percent_glyph(lo)
            if g is None:
                continue
        elif 0x20 <= hi < 0x80:
            g = hi - 0x20
            i += 1
        else:
            g = 63
            i += 1
        last_font = f
        afi = fonts.trim(kt, g)
        src = fonts.glyph_src(kt, g)
        if kt in (0, 2):
            dst = row0() + c * cwb - c_div(aflp, 2)
            afo = c_div(f.cw + 1 - afi, 2)
            # only %X-%Z, whose trims come from past englishFontOfsS, can put
            # dst outside the texture; the game writes there, this does not.
            # The first glyph of the last line starts at byte 0.
            wild = dst - (rows - 1) * TEX_W < 0 or dst + afo > len(tex)
            for r in range(0 if wild else rows):
                sr = fonts.row(src + r * f.stride, max(afo, 0))
                d = dst - r * TEX_W
                if aflp & 1:
                    # half a byte to the left: low nibble is the left texel
                    # (at byte 0 the game writes the byte before the texture)
                    for j in range(afo):
                        if d + j >= 1:
                            tex[d + j - 1] = (tex[d + j - 1] & 0x0f) | ((sr[j] << 4) & 0xf0)
                        tex[d + j] = sr[j] >> 4
                else:
                    tex[d:d + afo] = sr[:afo] if afo > 0 else b""
            aflp += afi
        else:
            dst = row0() + c * cwb
            afo = c_div(afi, 4)
            for r in range(rows):
                sr = src + r * f.stride
                d = dst - r * TEX_W
                for j in range(max(afo, 0)):
                    if d + j < len(tex):
                        tex[d + j] = 0
                for j in range(max(afo, 0), cwb):
                    tex[d + j] = fonts.byte(sr + j - afo)
        clm += 1
        c += 1
        if c >= per_row:
            if kt in (0, 2):
                tail()
            aflp = c = 0
            l += 1
            if l >= lines:
                return tex, clm
    if c and kt in (0, 2):
        tail()
    return tex, clm


# ---------------------------------------------------------------------------
# Quads: ccSprite::MakePacketStr for an unrotated textured sprite.

Quad = collections.namedtuple("Quad", "rgba uv0 xy0 uv1 xy1 shadow")


class Screen:
    """ccView.layer_screen reduced to what MakePacketStr uses: logical x, y to
    GS 12.4 as trunc(x * sx + ox). The game's is screenW * 16 * ax / 512 by
    screenH * 16 * ay / 384 (ccView::SetLayerCenter)."""

    def __init__(self, sx=16.0, sy=16.0, ox=SCREEN_X, oy=SCREEN_Y):
        self.sx, self.sy, self.ox, self.oy = sx, sy, ox, oy

    def apply(self, x, y):
        return int(x * self.sx + self.ox), int(y * self.sy + self.oy)


class Sprite:
    """The ccSprite fields MakePacketStr reads. su, sv are texels; wu, wv are
    kept x16 as the game keeps them; th is the texture height in texels."""

    def __init__(self, ctrl=1, th=256, colour=(128, 128, 128, 128), packets=512):
        self.ctrl = ctrl
        self.th = th
        self.colour = list(colour)
        self.transp = 1.0
        self.dx = self.dy = self.cx = self.cy = 0.0
        self.sx = self.sy = 0.0
        self.su = self.sv = self.wu = self.wv = 0
        self.wi = 1
        self.packet_num = 0
        self.packet_max = packets

    def set_type(self, t):
        sx, sy, su, sv, wu, wv, wi = FONT_TYPES[t]
        self.sx, self.sy, self.su, self.sv = float(sx), float(sy), su, sv
        self.wu, self.wv, self.wi = wu * 16, wv * 16, wi


def make_packet_str(spr, data, raw_mode, screen):
    """ccSprite::MakePacketStr(str, t) (0x0015aed0) for ctrl bits 1 (textured),
    0x10 (shadow), 0x20 / 0x40 (flip U / V); no rotation (2) or per-vertex
    colour (4, 8). -> [Quad]; moves spr.dx / dy and packet_num like the game."""
    out = []
    x, y = screen.apply(spr.dx + spr.cx, spr.dy + spr.cy)
    w, h = int(spr.sx * screen.sx), int(spr.sy * screen.sy)
    x_start, dx_start = x, spr.dx
    r, g, b, a = spr.colour
    a = int(a * spr.transp)
    main = (r, g, b, a)
    shade = SHADOW + (a,)
    su16, sv16, th16 = spr.su * 16, spr.sv * 16, spr.th * 16
    for ch in bytes(data) + b"\0":
        if not raw_mode:
            if ch == 0:
                break
            if ch < 33 or ch >= 128:
                x += w
                spr.dx += spr.sx
                continue
            cell = ch - 32
        else:
            if ch == 0xff:
                break
            if ch == 0xfe:
                x = x_start
                y += h
                spr.dx = dx_start
                spr.dy += spr.sy
                continue
            cell = ch
        for pas in (0, 1):
            if spr.packet_num >= spr.packet_max:
                return out
            if pas == 0:
                if not spr.ctrl & 0x10:
                    continue
                x, y = int(float(x) + screen.sx), int(float(y) + screen.sy)
                col = shade
            else:
                if spr.ctrl & 0x10:
                    x, y = int(float(x) - screen.sx), int(float(y) - screen.sy)
                col = main
            if x > CULL_X1 or x + w < SCREEN_X or y > CULL_Y1 or y + h < SCREEN_Y:
                continue
            u0 = (cell % spr.wi) * su16 + spr.wu
            v0 = th16 - spr.wv - (cell // spr.wi) * sv16 - 1
            u1, v1 = u0 + su16, v0 - (sv16 - 1)
            if spr.ctrl & 0x20:
                u0, u1 = u1 - 1, u0 - 1
            if spr.ctrl & 0x40:
                v0, v1 = v1, v0
            out.append(Quad(col, (u0, v0), (x, y), (u1, v1), (x + w, y + h), pas == 0))
            spr.packet_num += 1
        x += w
        spr.dx += spr.sx
    return out


class Kanji(Sprite):
    """A ccKanji: its own 128 x th texture and the font kt. ccKanji::Init
    (0x0015d0f0) gives ctrl 0x11 (SetPrim type 0 plus the shadow bit)."""

    def __init__(self, fonts, kt=0, th=128, colour=(128, 128, 128, 128), packets=16):
        super().__init__(ctrl=0x11, th=th, colour=colour, packets=packets)
        self.fonts = fonts
        self.kt = kt
        self.tex = bytearray(TEX_W * th)
        self.clm = 0

    def disp(self, raw, c=-1, screen=None, name=b"Kite", real=b"Player"):
        """ccKanji::Disp(str, c) (0x0015e380): draw at most c glyphs (c < 0:
        all). -> [Quad]."""
        if c == 0:
            return []
        screen = screen or Screen()
        strn = expand(raw, name, real)
        self.tex, self.clm = extract(self.fonts, strn, self.kt, self.th, self.tex)
        if c < 0 or c > self.clm:
            c = self.clm
        kt = self.kt
        small = kt in (0, 1)
        cw, ch, per_row = (8, 16, 16) if small else (12, 20, 10)

        def adv(g):
            if kt == 2:
                return cw - self.fonts.trim(2, g)
            if kt == 0:
                return cw - self.fonts.trim(0, g)
            return cw

        start = list(self.colour)
        quads = []
        s = _pad(strn)
        i = glyphs = run = row_used = run_u = run_w = 0
        pending = 0
        while True:
            flush = end = False
            ch_ = s[i]
            if ch_ == 0 or glyphs >= c:
                end = True
                flush = run != 0
            elif ch_ == 0x23:
                pending = s[i + 1]
                flush = run != 0
                i += 2
            elif ch_ == 0x25:
                g = percent_glyph(s[i + 1])
                i += 2
                if g is None:
                    continue
                run_w += adv(g)
                glyphs += 1
                run += 1
                flush = row_used + run >= per_row
            else:
                glyphs += 1
                run += 1
                flush = row_used + run >= per_row
                run_w += adv(ch_ - 0x20 if 0x20 <= ch_ < 0x80 else 63)
                i += 1
            if flush:
                if kt == 2:
                    self.sx, self.su = float(run_w), run_w - 1
                elif kt == 0:
                    self.sx, self.su = float(run_w - 1), run_w
                else:
                    self.sx, self.su = float(run_w), run_w
                self.sy, self.sv = float(ch), ch
                self.wu = run_u * 16
                self.wv = c_div(glyphs - 1, per_row) * ch * 16
                self.wi = 1
                run_u += run_w
                run_w = 0
                quads += make_packet_str(self, b"\0\xff", True, screen)
                if kt == 0:
                    self.dx += 1.0
                row_used += run
                if row_used >= per_row:
                    row_used = run_u = 0
                run = 0
            if end:
                break
            if pending:
                alpha = self.colour[3]
                if pending == ord("W"):
                    self.colour = list(start)
                elif pending in self.fonts.escape:
                    self.colour = list(self.fonts.escape[pending][:3]) + [0]
                self.colour[3] = alpha
                pending = 0
        self.colour = start
        self.packet_num = 0         # SendPacket
        return quads


# ---------------------------------------------------------------------------
# Drawing the quads the way the GS would.

class Canvas:
    def __init__(self, w, h, bg=(0x20, 0x20, 0x28)):
        self.w, self.h = w, h
        self.bg = bg
        self.px = [list(bg) + [255] if bg else [0, 0, 0, 0] for _ in range(w * h)]

    def blend(self, x, y, rgba):
        if not (0 <= x < self.w and 0 <= y < self.h):
            return
        d = self.px[y * self.w + x]
        r, g, b, a = rgba
        if self.bg:
            # ALPHA (Cs - Cd) * As >> 7 + Cd, clamped
            for k, cs in enumerate((r, g, b)):
                d[k] = max(0, min(255, ((cs - d[k]) * a >> 7) + d[k]))
        else:
            fa = min(a, 128) / 128
            da = d[3] / 255
            oa = fa + da * (1 - fa)
            if oa:
                for k, cs in enumerate((r, g, b)):
                    d[k] = round((cs * fa + d[k] * da * (1 - fa)) / oa)
            d[3] = round(oa * 255)

    def draw(self, quad, tex, pal, screen):
        """A textured SPRITE: pixel (px, py) is drawn when x0 <= 16 px < x1, and
        samples U, V interpolated at its top-left corner, point-sampled (the
        GS's rule as understood here, not checked on hardware). MODULATE: C =
        Ct * Cv >> 7, A = At * Av >> 7."""
        (u0, v0), (x0, y0), (u1, v1), (x1, y1) = quad.uv0, quad.xy0, quad.uv1, quad.xy1
        cr, cg, cb, ca = quad.rgba
        for py in range(-(-y0 // 16), -(-y1 // 16)):
            v = v0 + (v1 - v0) * (py * 16 - y0) / (y1 - y0)
            ty = int(v // 16)
            for px in range(-(-x0 // 16), -(-x1 // 16)):
                u = u0 + (u1 - u0) * (px * 16 - x0) / (x1 - x0)
                tr, tg, tb, ta = pal[tex.texel(int(u // 16), ty)]
                if not ta:
                    continue
                rgba = (min(255, tr * cr >> 7), min(255, tg * cg >> 7),
                        min(255, tb * cb >> 7), min(255, ta * ca >> 7))
                self.blend(px - screen.ox // 16, py - screen.oy // 16, rgba)

    def rgba(self, scale=1):
        out = bytearray()
        for y in range(self.h):
            row = bytearray()
            for x in range(self.w):
                row += bytes(self.px[y * self.w + x]) * scale
            out += bytes(row) * scale
        return bytes(out)


def kanji_texture(k):
    return Texture.from4(k.tex, 128, k.th)


# ---------------------------------------------------------------------------
# Text in, from the command line or from the game's tables.

def parse_text(s):
    """Unicode with \\n and \\xNN -> [bytes] lines, Shift-JIS encoded."""
    lines = [bytearray()]
    i = 0
    while i < len(s):
        ch = s[i]
        if ch == "\\" and i + 1 < len(s):
            nx = s[i + 1]
            if nx == "n":
                lines.append(bytearray())
                i += 2
                continue
            if nx == "x" and i + 3 < len(s) + 1:
                lines[-1].append(int(s[i + 2:i + 4], 16))
                i += 4
                continue
            if nx == "\\":
                lines[-1].append(0x5c)
                i += 2
                continue
        if ch == "\n":
            lines.append(bytearray())
        else:
            lines[-1] += ch.encode("cp932")
        i += 1
    return [bytes(x) for x in lines]


def _cstr(p, va, n=512):
    return p.cstr(va, n) if va else b""


def _lines(p, va, count):
    out = []
    buf = p.read(va, 4096) if va else b""
    for k in range(count):
        o = str_separate(buf, k)
        out.append(buf[o:buf.index(0, o)])
    return out


def mail_tables(d):
    """{name: (VA, records)} for MailTbl, MailTblp, ReMail and ReMailp, from
    the desktop overlay's code (d = Program(elf, "desktop")). Mutation's
    carried MailTbl name is wrong and Outbreak and Quarantine have none, and
    the mail count grew (326 in Infection, 375 after), so:
    - MailList_control::AddMailList walks the mails up to its slti bound,
      and forms MailTblp and then MailTbl (the parody test comes first), each
      with +4 and +8 beside it;
    - a static constructor of each table copies ReMail[k] into a mail's reply
      (__sinit_mailtbl.cpp: lui/addiu ReMail + 20k, then MailTbl + 0x48m +
      0x20), so ReMail is the other addresses that constructor forms."""
    words, formed = _formed(d, _fn(d, "AddMailList__16MailList_controlFv"))
    bound = [w & 0xffff for w in words if w >> 26 == 0x0a]
    if len(bound) != 1:
        raise ValueError(f"{d.path}: AddMailList has {len(bound)} slti, not 1")
    seen = {a for _, op, a in formed if op == 0x09}
    bases = []
    for _, op, a in formed:
        if op == 0x09 and a + 4 in seen and a + 8 in seen and a not in bases:
            bases.append(a)
    if len(bases) != 2:
        raise ValueError(f"{d.path}: AddMailList forms {len(bases)} mail tables, not 2")
    n = bound[0]
    out = {"MailTblp": (bases[0], n), "MailTbl": (bases[1], n)}
    ov = d.overlay
    for name, (tbl, _) in (("ReMail", out["MailTbl"]), ("ReMailp", out["MailTblp"])):
        for cva in range(ov.ctor_start, ov.ctor_end, 4):
            _, formed = _formed(d, d.u32(cva))
            formed = [a for _, op, a in formed if op == 0x09]      # lui + addiu
            into = [a for a in formed if tbl <= a < tbl + 0x48 * n]
            src = sorted({a for a in formed if not tbl <= a < tbl + 0x48 * n})
            if into and src:
                if src[-1] - src[0] != 0x14 * (len(src) - 1):
                    raise ValueError(f"{d.path}: {name} is not one run of records")
                out[name] = (src[0], len(src))
                break
        else:
            raise ValueError(f"{d.path}: no constructor fills {name}")
    return out


def news_count(d):
    """HtmlTbl's records: AddHtmlList walks them until a title "NULL", which
    is counted here too (Infection's HtmlTbl object ends with it)."""
    tbl = _fn(d, "HtmlTbl")
    k = 0
    while True:
        title = d.u32(tbl + 0x1c * k + 8)
        k += 1
        if _cstr(d, title) == b"NULL":
            return k


def event_messages(p):
    """{table name: [(event, VA of its message array, records)]} for evMsgTbl
    and evMsgTblp. ccEvent::Execute reads table[ev / 50][ev % 50][msg] with
    no bound. The two tables are parallel, so a table has as many groups as
    fit before evMsgTblp; a message array (12-byte records) runs up to the
    next array or group table, whichever starts first - Infection's objects
    end there, give or take zero padding, and the others' are not named."""
    tabs = {n: _fn(p, n) for n in ("evMsgTbl", "evMsgTblp")}
    ngroups = (tabs["evMsgTblp"] - tabs["evMsgTbl"]) // 4
    found = {}
    starts = set(tabs.values())
    for name, t in tabs.items():
        found[name] = []
        for gi in range(ngroups):
            grp = p.u32(t + 4 * gi)
            if not grp:
                continue
            starts.add(grp)
            for ei in range(50):
                base = p.u32(grp + 4 * ei)
                if base:
                    starts.add(base)
                    found[name].append((50 * gi + ei, base))
    starts = sorted(starts)
    out = {}
    for name, arrays in found.items():
        out[name] = []
        for ev, base in arrays:
            i = bisect.bisect_right(starts, base)
            if i < len(starts):
                n = (starts[i] - base) // 12
            else:
                n = 0
                while any(p.read(base + 12 * n, 12)):
                    n += 1
            out[name].append((ev, base, n))
    return out


def table_text(elf, ref, parody=False):
    """@mail:N[:title] @remail:N @bbs:T:P @news:N @event:E:M -> (lines, kt the
    game draws it with)."""
    kind, _, rest = ref.lstrip("@").partition(":")
    args = rest.split(":") if rest else []
    if kind in ("mail", "remail"):
        d = Program(elf, "desktop")
        tables = mail_tables(d)
        if kind == "mail":
            tbl = tables["MailTblp" if parody else "MailTbl"][0]
            rec = d.read(tbl + 0x48 * int(args[0]), 0x20)
            _, _, _, title, frm, _, line, body = struct.unpack("<iiiIIiiI", rec)
        else:
            tbl = tables["ReMailp" if parody else "ReMail"][0]
            _, title, frm, line, body = struct.unpack("<iIIiI", d.read(tbl + 0x14 * int(args[0]), 0x14))
        if len(args) > 1 and args[1] == "title":
            return [_cstr(d, title)], 0
        return [_cstr(d, title), _cstr(d, frm)] + _lines(d, body, line), 0
    if kind == "bbs":
        t = Program(elf, "toppage")
        tbl = _fn(t, "bbsThreadTblP" if parody else "bbsThreadTbl")
        ttitle, msgs, num = struct.unpack("<IIi", t.read(tbl + 12 * int(args[0]), 12))
        _, title, who, lines, body = struct.unpack("<iIIiI", t.read(msgs + 0x14 * int(args[1]), 0x14))
        return [_cstr(t, title), _cstr(t, who)] + _lines(t, body, lines), 0
    if kind == "news":
        d = Program(elf, "desktop")
        tbl = _fn(d, "HtmlTbl")
        for k in range(news_count(d)):
            no, _, title = struct.unpack("<iiI", d.read(tbl + 0x1c * k, 12))
            if no == int(args[0]):
                return [_cstr(d, title)], 0
        raise SystemExit(f"no news {args[0]}")
    if kind == "event":
        p = Program(elf)
        ev, msg = int(args[0]), int(args[1])
        tables = _fn(p, "evMsgTblp" if parody else "evMsgTbl")
        grp = p.u32(tables + 4 * (ev // 50))
        base = p.u32(grp + 4 * (ev % 50)) if grp else 0
        if not base:
            raise SystemExit(f"event {ev} has no messages")
        _, who, text = struct.unpack("<iII", p.read(base + 12 * msg, 12))
        lines = _lines(p, text, 3)
        while lines and not lines[-1]:
            lines.pop()
        return ([_cstr(p, who)] if who else []) + lines, 2
    raise SystemExit(f"unknown source {ref}")


def corpus(elf):
    """[(where, raw)] every string the tables hold that ccKanji draws: event
    messages and speakers, mail and replies, board threads and posts, news
    headlines; English and parody. Where each table is and how long comes
    from the executable's code (event_messages, mail_tables, news_count);
    evMsgTbl, HtmlTbl and bbsThreadTbl carry the right names on every volume
    (each is the address its reader forms)."""
    out = []
    p = Program(elf)
    for tname, arrays in event_messages(p).items():
        for ev, base, n in arrays:
            for k in range(n):
                _, who, text = struct.unpack("<iII", p.read(base + 12 * k, 12))
                where = f"{tname}[{ev}][{k}]"
                if who:
                    out.append((where + ".name", _cstr(p, who)))
                if text:
                    for n_, ln in enumerate(_lines(p, text, 3)):
                        out.append((f"{where}.{n_}", ln))
    d = Program(elf, "desktop")
    mail = mail_tables(d)
    for tname in ("MailTbl", "MailTblp"):
        va, n = mail[tname]
        for k in range(n):
            rec = d.read(va + 0x48 * k, 0x20)
            _, _, _, title, frm, _, line, body = struct.unpack("<iiiIIiiI", rec)
            out += [(f"{tname}[{k}].title", _cstr(d, title)), (f"{tname}[{k}].from", _cstr(d, frm))]
            out += [(f"{tname}[{k}].{n_}", ln) for n_, ln in enumerate(_lines(d, body, line))]
    for tname in ("ReMail", "ReMailp"):
        va, n = mail[tname]
        for k in range(n):
            _, title, frm, line, body = struct.unpack("<iIIiI", d.read(va + 0x14 * k, 0x14))
            out += [(f"{tname}[{k}].title", _cstr(d, title)), (f"{tname}[{k}].from", _cstr(d, frm))]
            out += [(f"{tname}[{k}].{n_}", ln) for n_, ln in enumerate(_lines(d, body, line))]
    va = _fn(d, "HtmlTbl")
    for k in range(news_count(d)):
        title = struct.unpack("<I", d.read(va + 0x1c * k + 8, 4))[0]
        out.append((f"HtmlTbl[{k}]", _cstr(d, title)))
    t = Program(elf, "toppage")
    for tname in ("bbsThreadTbl", "bbsThreadTblP"):
        va = _fn(t, tname)
        th = 0
        while True:
            title, msgs, num = struct.unpack("<IIi", t.read(va + 12 * th, 12))
            if not title or not 0 < num < 1000:
                break
            out.append((f"{tname}[{th}]", _cstr(t, title)))
            for k in range(num):
                _, ptitle, who, lines, body = struct.unpack("<iIIiI", t.read(msgs + 0x14 * k, 0x14))
                w = f"{tname}[{th}][{k}]"
                out += [(w + ".title", _cstr(t, ptitle)), (w + ".name", _cstr(t, who))]
                out += [(f"{w}.{n}", ln) for n, ln in enumerate(_lines(t, body, lines))]
            th += 1
    return [(w, s) for w, s in out if s]


# ---------------------------------------------------------------------------
# The game's own code in tools/eemu.py.

class Game:
    """The text functions run in tools/eemu.py on scratch objects, for checking
    everything above: ccGetExtendedCode, ccKanjiStrWidth / Strlen / Separate /
    Strcat, ccKanji::Extract and Disp (packets captured at SendPacket),
    ccFont::SetType and ccSprite::MakePacketStr. The VU0 macro-mode helpers
    (ApplyLayerScreenMatrix, sceVu0FTOI0Vector) run in Python; the view is
    diag(16, 16) plus the screen offset, so their arithmetic is exact."""

    SAVE, SYS, SCRATCH = 0x01800000, 0x01801000, 0x01802000
    OBJ, LAYER, VIEW, BIT = 0x01810000, 0x01810400, 0x01810800, 0x01810c00
    TEXA, TEXB = 0x01811000, 0x01814000
    STR, OUT, PKT = 0x01820000, 0x01828000, 0x01830000

    def __init__(self, program):
        import eemu
        self.eemu = eemu
        self.p = program
        m = self.m = eemu.Machine(program)
        sym = lambda n: program.symbol_named(n).value      # noqa: E731
        self.fn = {k: sym(v) for k, v in dict(
            ext="ccGetExtendedCode__FUs", width="ccKanjiStrWidth__FPci", strlen="ccKanjiStrlen__FPc",
            separate="ccKanjiStrSeparate__FPci", strcat="ccKanjiStrcat__FPcPci",
            extract="Extract__7ccKanjiFPc", disp="Disp__7ccKanjiFPciff", settype="SetType__6ccFontFi",
            packetstr="MakePacketStr__8ccSpriteFPci").items()}
        m.store(sym("saveData"), 4, self.SAVE)
        m.store(sym("ccSys"), 4, self.SYS)
        m.store(self.SYS + 604, 4, self.SCRATCH)
        m.store(sym("fontTex"), 4, self.BIT)        # only tested for non-zero
        m.hooks[sym("SetDataAdrs__9ccBltDataFP1")] = lambda *a: 0
        m.hooks[sym("ApplyLayerScreenMatrix__6ccViewFPiPf")] = self._apply
        m.hooks[sym("sceVu0FTOI0Vector")] = self._ftoi0
        m.hooks[sym("SendPacket__8ccSpriteFv")] = self._send
        # float -> unsigned long long; its dptoul uses dsrlv, which eemu lacks
        m.hooks[sym("__fixunssfdi")] = self._fixunssfdi
        mat = [16.0, 0, 0, 0, 0, 16.0, 0, 0, 0, 0, 1.0, 0, float(SCREEN_X), float(SCREEN_Y), 0, 1.0]
        for k, v in enumerate(mat):
            m.store(self.VIEW + 0x190 + 4 * k, 4, eemu.f_from_py(v))
        m.store(self.LAYER + 44, 4, self.VIEW)
        self.sent = []
        self.set_names(b"Kite", b"Player")

    # hooks --------------------------------------------------------------
    def _f(self, va):
        return self.m.load(va, 4)

    def _apply(self, m, view, ivec, fvec, _):
        e = self.eemu
        v = [self._f(fvec + 4 * k) for k in range(4)]
        rows = [[self._f(view + 0x190 + 16 * r + 4 * k) for k in range(4)] for r in range(4)]
        for k in range(4):
            acc = e.f_mul(rows[0][k], v[0])
            acc = e.f_add(acc, e.f_mul(rows[1][k], v[1]))
            acc = e.f_add(acc, e.f_mul(rows[2][k], v[2]))
            acc = e.f_add(acc, e.f_mul(rows[3][k], v[3]))
            m.store(ivec + 4 * k, 4, e.f_to_int(acc))
        return 0

    def _ftoi0(self, m, dst, src, *_):
        for k in range(4):
            m.store(dst + 4 * k, 4, self.eemu.f_to_int(self._f(src + 4 * k)))
        return 0

    def _fixunssfdi(self, m, *_):
        n, e = self.eemu.f_value(m.f[12])
        if n <= 0:
            return 0
        return n << e if e >= 0 else n >> -e

    def _send(self, m, obj, *_):
        n = m.load(obj + 32, 4)
        pkt = m.load(obj + 44, 4)
        self.sent.append(self.read_quads(pkt, n))
        m.store(obj + 32, 4, 0)
        return 0

    # helpers ------------------------------------------------------------
    def put(self, va, raw, pad=16):
        self.m.mem[va:va + len(raw) + pad] = bytes(raw) + b"\0" * pad

    def set_names(self, name, real):
        self.name, self.real = name, real
        self.put(self.SAVE, name[:23])
        self.put(self.SAVE + 0x18, real[:23])

    def call(self, fn, *args):
        return self.m.call(self.fn[fn], args)

    def read_quads(self, pkt, n):
        """n six-qword SPRITE packets from +176: TEX0 RGBAQ UV XYZ2 UV XYZ2."""
        out = []
        for k in range(n):
            q = pkt + 176 + 96 * k
            w = [self.m.load(q + 16 * j + 4 * i, 4) for j in range(6) for i in range(4)]
            out.append((tuple(w[4:8]), (w[8], w[9]), (w[12], w[13]), (w[16], w[17]), (w[20], w[21])))
        return out

    # the string functions -----------------------------------------------
    def ext(self, code):
        return self.call("ext", code)

    def width(self, raw, kt):
        self.put(self.STR, raw)
        return self.eemu.sx(self.call("width", self.STR, kt), 32)

    def strlen(self, raw):
        self.put(self.STR, raw)
        return self.eemu.sx(self.call("strlen", self.STR), 32)

    def separate(self, buf, n):
        self.put(self.STR, buf)
        return self.call("separate", self.STR, n) - self.STR

    def strcat(self, raw, m):
        self.put(self.OUT, b"", 256)
        if raw is None:
            self.call("strcat", self.OUT, 0, m)
        else:
            self.put(self.STR, raw)
            self.call("strcat", self.OUT, self.STR, m)
        mem = self.m.mem
        return bytes(mem[self.OUT:mem.index(0, self.OUT)])

    # ccKanji ------------------------------------------------------------
    def _kanji(self, kt, th, colour, packets, dx=0.0, dy=0.0):
        m, e = self.m, self.eemu
        m.mem[self.OBJ:self.OBJ + 0xe8] = bytes(0xe8)
        m.store(self.OBJ + 4, 4, 0x11)
        m.store(self.OBJ + 12, 4, 6)
        m.store(self.OBJ + 36, 4, packets)
        m.store(self.OBJ + 44, 4, self.PKT)
        m.store(self.OBJ + 48, 4, e.f_from_py(1.0))
        m.store(self.OBJ + 64, 4, e.f_from_py(dx))
        m.store(self.OBJ + 68, 4, e.f_from_py(dy))
        r, g, b, a = colour
        m.store(self.OBJ + 104, 8, r | g << 32)
        m.store(self.OBJ + 112, 8, b | a << 32)
        m.store(self.OBJ + 168, 4, self.LAYER)
        m.store(self.OBJ + 176, 8, 7 << 26 | (th.bit_length() - 1) << 30)
        m.store(self.OBJ + 208, 4, self.BIT)
        m.store(self.OBJ + 212, 2, kt)
        m.store(self.OBJ + 216, 4, 1)           # Extract flips it to 0: TEXA
        m.store(self.OBJ + 220, 4, self.TEXA)
        m.store(self.OBJ + 224, 4, self.TEXB)
        m.mem[self.TEXA:self.TEXA + TEX_W * th] = bytes(TEX_W * th)

    def extract(self, strn, kt, th=128):
        self._kanji(kt, th, (128, 128, 128, 128), 16)
        self.put(self.STR, strn)
        self.m.call(self.fn["extract"], (self.OBJ, self.STR))
        return bytes(self.m.mem[self.TEXA:self.TEXA + TEX_W * th]), self.m.load(self.OBJ + 214, 2)

    def disp(self, raw, kt, c=-1, th=128, colour=(128, 128, 128, 128), packets=16, dx=0.0, dy=0.0):
        """-> (texture, clm, quads, dx after)."""
        self._kanji(kt, th, colour, packets, dx, dy)
        self.put(self.STR, raw)
        self.sent = []
        self.m.call(self.fn["disp"], (self.OBJ, self.STR, c))
        quads = self.sent[0] if self.sent else []
        tex = bytes(self.m.mem[self.TEXA:self.TEXA + TEX_W * th])
        dx_after = self.eemu.f_to_py(self.m.load(self.OBJ + 64, 4))
        return tex, self.m.load(self.OBJ + 214, 2), quads, dx_after

    # ccFont -------------------------------------------------------------
    def set_type(self, t):
        self.m.mem[self.OBJ:self.OBJ + 0xd0] = bytes(0xd0)
        self.call("settype", self.OBJ, t)
        f = [self.eemu.f_to_py(self.m.load(self.OBJ + o, 4)) for o in (72, 76)]
        i = [self.eemu.sx(self.m.load(self.OBJ + o, 4), 32) for o in (80, 84, 88, 92, 96)]
        return (f[0], f[1], i[0], i[1], i[2] / 16, i[3] / 16, i[4])

    def font_str(self, raw, t, ctrl=1, raw_mode=False, colour=(128, 128, 128, 128), packets=512,
                 dx=0.0, dy=0.0):
        """ccFont::SetType(t) then MakePacketStr(str, raw_mode) -> (quads, dx after)."""
        m, e = self.m, self.eemu
        self.set_type(t)
        m.store(self.OBJ + 4, 4, ctrl)
        m.store(self.OBJ + 12, 4, 6)
        m.store(self.OBJ + 36, 4, packets)
        m.store(self.OBJ + 44, 4, self.PKT)
        m.store(self.OBJ + 48, 4, e.f_from_py(1.0))
        m.store(self.OBJ + 64, 4, e.f_from_py(dx))
        m.store(self.OBJ + 68, 4, e.f_from_py(dy))
        r, g, b, a = colour
        m.store(self.OBJ + 104, 8, r | g << 32)
        m.store(self.OBJ + 112, 8, b | a << 32)
        m.store(self.OBJ + 168, 4, self.LAYER)
        m.store(self.OBJ + 176, 8, 8 << 26 | 8 << 30)
        self.put(self.STR, raw)
        m.call(self.fn["packetstr"], (self.OBJ, self.STR, int(raw_mode)))
        quads = self.read_quads(self.PKT, m.load(self.OBJ + 32, 4))
        return quads, e.f_to_py(m.load(self.OBJ + 64, 4))


def quad_words(q):
    """A Quad as the words the game writes, for comparing with Game's."""
    m = 0xffffffff
    return (tuple(q.rgba), (q.uv0[0] & m, q.uv0[1] & m), (q.xy0[0] & m, q.xy0[1] & m),
            (q.uv1[0] & m, q.uv1[1] & m), (q.xy1[0] & m, q.xy1[1] & m))


# ---------------------------------------------------------------------------
# check

def synthetic():
    """Strings that reach the corners: every escape and %x, every mapped pair,
    unmapped pairs, lone lead bytes, control bytes, long lines."""
    out = [b"", b" ", b"#0", b"#1", b"#0 and #1", b"#R#G#B#Y#W#Q#", b"%", b"#",
           b"%%%#%0%9%A%Z%a%!%\x80", b"ABC\x83\xa2DEF\x81\x65\x81\x7e",
           b"\x98\x72\x97\xcd", b"\x83\x83\xa2", b"\x0a\x09x\x7f", b"\xff\xfe",
           bytes(range(0x20, 0x80)), b"W" * 40, b"i" * 70, b"#Ymm#Wmm#Bmm#Rmm#Gmm#Wmm",
           b"%A%B%C%D%E%F%G%H%I", b"-" * 112 + b"Log Out", b"-" * 50 + b"Log Out"]
    for k in range(0x20, 0x80):
        out.append(b"%" + bytes([k]) + b"x")
    for code in EXTENDED:
        out.append(b"a" + bytes([code >> 8, code & 0xff]) + b"b")
    return out


def wild(s):
    """True when s holds %X-%Z. Their trims come from past englishFontOfsS and
    send Extract's writes outside the texture, so Disp checks leave them out."""
    return any(s[k] == 0x25 and s[k + 1] in b"XYZ" for k in range(len(s) - 1))


def run_check(elf, sample=1, disp=300, out=sys.stdout, progress=True):
    """The game's functions against these, over the text tables and synthetic
    strings. -> number of mismatches."""
    fonts = Fonts(elf)
    game = Game(fonts.p)
    bad = 0

    def report(what, key, got, want):
        nonlocal bad
        bad += 1
        if bad <= 20:
            print(f"MISMATCH {what} {key!r}: game {got!r}, python {want!r}", file=out)

    # ccGetExtendedCode, every code
    n = 0
    for code in range(0x10000):
        g, py = game.ext(code), extended_code(code)
        if g != py:
            report("ccGetExtendedCode", hex(code), hex(g), hex(py))
        n += 1
    print(f"ccGetExtendedCode: {n} codes", file=out)

    texts = [s for _, s in corpus(elf)]
    texts = texts[::sample] + synthetic()
    names = [(b"Kite", b"Player"), (b"#R%A\x83\xa2x", b"\x81\x65Q")]
    counts = collections.Counter()
    for ni, (name, real) in enumerate(names):
        game.set_names(name, real)
        pool = texts if ni == 0 else synthetic()
        for j, s in enumerate(pool):
            if progress and j % 2000 == 0 and j:
                print(f"  ... strings {j}/{len(pool)}", file=sys.stderr)
            for kt in (0, 1, 2, 3):
                g, py = game.width(s, kt), str_width(fonts, s, kt, name, real)
                if g != py:
                    report(f"ccKanjiStrWidth kt {kt}", s, g, py)
                counts["width"] += 1
            g, py = game.strlen(s), str_len(s, name, real, fonts.strlen_counted)
            if g != py:
                report("ccKanjiStrlen", s, g, py)
            counts["strlen"] += 1
            for m in (0, 5, 12, 20):
                g, py = game.strcat(s, m), strcat_field(s, m, name, real)
                if g != py:
                    report(f"ccKanjiStrcat m {m}", s, g, py)
                counts["strcat"] += 1
    game.set_names(b"Kite", b"Player")
    for m in (0, 7):
        if game.strcat(None, m) != strcat_field(None, m):
            report("ccKanjiStrcat null", m, game.strcat(None, m), strcat_field(None, m))
        counts["strcat"] += 1
    # ccKanjiStrSeparate over the multi-line texts as the tables hold them
    blob = b"\0".join(texts[:400]) + b"\0"
    for k in range(0, 60):
        g, py = game.separate(blob, k), str_separate(blob, k)
        if g != py:
            report("ccKanjiStrSeparate", k, g, py)
        counts["separate"] += 1
    print(f"ccKanjiStrWidth: {counts['width']} (string, kt); ccKanjiStrlen: {counts['strlen']}; "
          f"ccKanjiStrcat: {counts['strcat']}; ccKanjiStrSeparate: {counts['separate']}", file=out)

    # ccFont::SetType
    for t in range(4):
        g = game.set_type(t)
        want = tuple(float(v) if k < 2 else v for k, v in enumerate(FONT_TYPES[t]))
        if g != want:
            report("ccFont::SetType", t, g, want)
    # ccSprite::MakePacketStr, string and raw mode, with and without shadow
    fstrings = [b"HELLO world 123", bytes(range(0x01, 0x100)), b"a\nb\x7f~", b"MISS +,--", b"\x00"]
    nq = 0
    for t in range(4):
        for ctrl in (1, 0x11, 0x21, 0x41):
            for raw in (False, True):
                for s in fstrings:
                    data = s if not raw else bytes(b for b in s if b != 0xff)[:40] + b"\xff"
                    gq, gdx = game.font_str(data, t, ctrl, raw, dx=3.0, dy=5.0)
                    spr = Sprite(ctrl=ctrl, th=256)
                    spr.set_type(t)
                    spr.dx, spr.dy = 3.0, 5.0
                    pq = make_packet_str(spr, data if not raw else data, raw, Screen())
                    if [tuple(x) for x in gq] != [quad_words(q) for q in pq] or gdx != spr.dx:
                        report(f"MakePacketStr type {t} ctrl {ctrl:#x} raw {raw}", s, (len(gq), gdx),
                               (len(pq), spr.dx))
                    nq += len(gq)
    print(f"ccFont::SetType: 4 types; MakePacketStr: {4 * 4 * 2 * len(fstrings)} calls, "
          f"{nq} quads", file=out)

    # ccKanji::Disp, which runs Extract: texture byte for byte, packets word
    # for word.
    table = [s for _, s in corpus(elf)]
    table = table[::max(1, len(table) // disp)] if disp else []
    cases = [(s, -1, names[0]) for s in table]
    cases += [(s, c, nm) for s in synthetic() if not wild(s) for c in (-1, 5) for nm in names]
    nd = nquads = 0
    for i, (s, c, (name, real)) in enumerate(cases):
        game.set_names(name, real)
        for kt in (0, 1, 2, 3):
            colour = (128, 128, 128, 128) if i % 2 else (100, 90, 80, 64)
            gtex, gclm, gq, gdx = game.disp(s, kt, c, colour=colour, dx=7.0, dy=9.0)
            k = Kanji(fonts, kt, colour=colour)
            k.dx, k.dy = 7.0, 9.0
            pq = k.disp(s, c, name=name, real=real)
            if gtex != bytes(k.tex) or gclm != k.clm:
                diff = sum(1 for a, b in zip(gtex, k.tex) if a != b)
                report(f"ccKanji::Extract kt {kt}", s, (gclm, diff), k.clm)
            if gq != [quad_words(q) for q in pq] or gdx != k.dx:
                report(f"ccKanji::Disp kt {kt} c {c}", s, (len(gq), gdx), (len(pq), k.dx))
            nd += 1
            nquads += len(gq)
        if progress and i % 50 == 0:
            print(f"  ... Disp {i}/{len(cases)}", file=sys.stderr)
    print(f"ccKanji::Disp and Extract: {nd} calls ({len(table)} table strings, the rest "
          f"synthetic), {nquads} quads", file=out)
    print(f"{len(texts) - len(synthetic())} table strings, {len(synthetic())} synthetic; "
          f"{bad} mismatches", file=out)
    return bad


# ---------------------------------------------------------------------------
# CLI

def font_arg(s):
    s = s.lower()
    if s.isdigit():
        s = "k" + s
    if len(s) != 2 or s[0] not in "ak" or s[1] not in "0123":
        raise argparse.ArgumentTypeError("font is a0-a3 or k0-k3")
    return s


def print_glyphs(fonts, which):
    for f in which:
        n = int(f[1])
        if f[0] == "a":
            sx, sy, su, sv, wu, wv, wi = FONT_TYPES[n]
            cells = 24 if n == 3 else (65 if n == 2 else 96)
            print(f"# a{n}: ccFont type {n}, TEX_xasc00 (xasc00.cmp), {su}x{sv} cells from ({wu}, {wv}), "
                  f"{wi} a row; advance {sx}, line {sy}. u, v are texels in the upright image;")
            print("#     the game's UV are (u * 16, (256 - v) * 16 - 1) to "
                  "((u + w) * 16, (256 - v - h) * 16)")
            print("byte  shows  cell    u    v   w   h  advance")
            for cell in range(cells):
                u = wu + (cell % wi) * su
                v = wv + (cell // wi) * sv
                shows = BIG_DIGITS[cell] if n == 3 else chr(0x20 + cell)
                shows = {" ": "' '", "\x7f": "Δ"}.get(shows, shows)
                print(f"0x{0x20 + cell:02x}  {shows:5}  {cell:4}  {u:3}  {v:3}  {su:2}  {sv:2}  {sx:7}")
        else:
            k = KANJI[n]
            fixed = "" if k.proportional else ", fixed pitch"
            print(f"# k{n}: ccKanji kt {n}, {k.bitmap} (0x{fonts.va[k.bitmap]:08x}), {k.cw}x{k.ch} cells, "
                  f"16 a row, {k.stride} bytes a pixel row; trim from {k.table} "
                  f"(0x{fonts.va[k.table]:08x}){fixed}")
            if k.proportional:
                print("#     advance = cell - trim; Extract copies (cell + 1 - trim) / 2 bytes a row")
            else:
                print("#     advance = cell; the glyph is moved right 2 * (trim / 4) texels")
            print("glyph  shows  source VA    x    y  trim  advance  copied")
            for g in range(131):
                src = fonts.glyph_src(n, g)
                t = fonts.trim(n, g)
                if k.proportional:
                    adv = k.cw - t
                    copied = 2 * max(0, c_div(k.cw + 1 - t, 2))
                else:
                    adv, copied = k.cw, k.cw - 2 * c_div(t, 4)
                shows = glyph_name(g) if g < GLYPHS else "(past the bitmap)"
                shows = "' '" if shows == " " else (shows or "(blank)")
                print(f"{g:5}  {shows:5}  0x{src:08x}  {(g & 15) * k.cw:3}  {(g >> 4) * k.ch:3}  "
                      f"{t:4}  {adv:7}  {copied:6}")
        print()


def print_extended(fonts):
    print("# ccGetExtendedCode (0x0015d390): the Shift-JIS pairs that have a glyph")
    print("sjis  char  becomes  glyph")
    for code, e in EXTENDED.items():
        ch = bytes((code >> 8, code & 0xff)).decode("cp932")
        g = percent_glyph(e & 0xff)
        print(f"{code:04x}  {ch}     %{chr(e & 0xff)}       {g} {glyph_name(g)}")
    print("# any other pair comes back unchanged: Disp, Strcat and StrWidth write or count '_'")
    print("# and step one byte, so the trail byte is read again as a character")
    print()
    print("# %x in a string, as Disp / Extract draw it and StrWidth counts it (kt 0 / kt 2 widths)")
    print("x   glyph  shows       S  L")
    for d in list(range(0x30, 0x3a)) + list(range(0x41, 0x5b)) + [0x25, 0x23]:
        g = percent_glyph(d)
        shows = glyph_name(g) if g < GLYPHS else "(past the bitmap)"
        shows = shows or "(blank)"
        print(f"%{chr(d)}  {g:5}  {shows:10}  {8 - fonts.trim(0, g):2} {12 - fonts.trim(2, g):2}")
    print("# every other %x: nothing drawn, width 0 (kt 1 / 3 still count 8 / 12 in StrWidth)")


def measure(fonts, raw, name, real):
    strn = expand(raw, name, real)
    print(f"text        {raw!r}")
    print(f"expanded    {strn!r}")
    print(f"ccKanjiStrlen          {str_len(raw, name, real, fonts.strlen_counted)}")
    for kt in range(4):
        k = Kanji(fonts, kt)
        quads = k.disp(raw, -1, name=name, real=real)
        mains = [q for q in quads if not q.shadow]
        print(f"kt {kt}: ccKanjiStrWidth {str_width(fonts, raw, kt, name, real):4}   Disp draws "
              f"{k.clm} glyphs in {len(mains)} runs, advancing {k.dx:g} px")


def render(fonts, lines, font, out, width=None, center=False, scale=1, bg=(0x20, 0x20, 0x28),
           colour=(128, 128, 128, 128), shadow=False, line_height=None, packets=None, name=b"Kite",
           real=b"Player"):
    screen = Screen()
    margin = 4
    quads = []
    if font[0] == "k":
        kt = int(font[1])
        pitch = line_height or kfont(kt).ch
        widths = [str_width(fonts, ln, kt, name, real) for ln in lines]
        w = width or max(widths + [1]) + 2 * margin
        _, pal = fonts.xasc()
        for n, ln in enumerate(lines):
            k = Kanji(fonts, kt, colour=colour, packets=packets or 16)
            k.dx = float(margin + (c_div(w - 2 * margin, 2) - c_div(widths[n], 2) if center else 0))
            k.dy = float(margin + n * pitch)
            quads.append((k.disp(ln, -1, screen, name, real), kanji_texture(k)))
        h = margin * 2 + len(lines) * pitch
    else:
        t = int(font[1])
        sx, sy = FONT_TYPES[t][:2]
        pitch = line_height or sy
        w = width or max(len(ln) for ln in lines) * sx + 2 * margin
        tex, pal = fonts.xasc()
        for n, ln in enumerate(lines):
            spr = Sprite(ctrl=0x11 if shadow else 1, colour=colour, packets=packets or 512)
            spr.set_type(t)
            spr.dx = float(margin + ((w - 2 * margin - len(ln) * sx) // 2 if center else 0))
            spr.dy = float(margin + n * pitch)
            quads.append((make_packet_str(spr, ln, False, screen), tex))
        h = margin * 2 + len(lines) * pitch
    canvas = Canvas(w, h + 1, bg)
    for qs, tex in quads:
        for q in qs:
            canvas.draw(q, tex, pal, screen)
    import png
    png.write_rgba(out, canvas.w * scale, canvas.h * scale, canvas.rgba(scale))
    return canvas.w, canvas.h


def sheet(fonts, outdir):
    import png
    os.makedirs(outdir, exist_ok=True)
    try:
        _, pal = fonts.xasc()
    except (OSError, KeyError):
        pal = [(0, 0, 0, 0)] + [(255, 255, 255, 128)] * 3 + [(v, v, v, 128) for v in range(0, 256, 22)]
    for kt, name in ((0, "ef8x16"), (2, "ef12x20")):
        f = KANJI[kt]
        w, h = 16 * f.cw, 7 * f.ch
        data = fonts.row(fonts.va[f.bitmap], f.stride * h)
        tex = Texture.from4(data, w, h)
        canvas = Canvas(w, h)
        for y in range(h):
            for x in range(w):
                r, g, b, a = pal[tex.texel(x, y)]
                if a:
                    canvas.blend(x, y, (r, g, b, a))
        png.write_rgba(os.path.join(outdir, name + ".png"), w * 3, h * 3, canvas.rgba(3))
        print(f"{name}.png  {w}x{h}, 112 glyphs")


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("glyphs")
    p.add_argument("elf")
    p.add_argument("--font", type=font_arg)
    sub.add_parser("extended").add_argument("elf")
    for name in ("measure", "render"):
        p = sub.add_parser(name)
        p.add_argument("elf")
        p.add_argument("text")
        if name == "render":
            p.add_argument("out")
            p.add_argument("--font", type=font_arg, help="a0-a3 or k0-k3 (default k0, k2 for @event)")
            p.add_argument("--width", type=int, help="image width in pixels (the game never wraps)")
            p.add_argument("--center", action="store_true", help="centre each line on its StrWidth")
            p.add_argument("--scale", type=int, default=1)
            p.add_argument("--bg", default="32,32,40", help="R,G,B or none for transparent")
            p.add_argument("--colour", default="128,128,128,128", help="vertex R,G,B,A, 128 = 1.0")
            p.add_argument("--shadow", action="store_true",
                           help="shadow bit for a0-a3 (ccKanji always has it)")
            p.add_argument("--line-height", type=int)
            p.add_argument("--packets", type=int,
                           help="packetMax: 16 for mail and dialogue, 512 for ccFont")
            p.add_argument("--data", help="DATA.BIN (default: next to ELF)")
        p.add_argument("--name", default="Kite", help="#0, ccSaveData.plName")
        p.add_argument("--real-name", default="Player", help="#1, ccSaveData.plRealName")
        p.add_argument("--parody", action="store_true")
    p = sub.add_parser("sheet")
    p.add_argument("elf")
    p.add_argument("out")
    p = sub.add_parser("check")
    p.add_argument("elf")
    p.add_argument("--sample", type=int, default=1, help="every Nth table string (default all)")
    p.add_argument("--disp", type=int, default=300, help="table strings through Disp (default 300)")
    args = parser.parse_args()

    if args.cmd == "check":
        return 1 if run_check(args.elf, args.sample, args.disp) else 0
    fonts = Fonts(args.elf, getattr(args, "data", None))
    if args.cmd == "glyphs":
        every = ["a0", "a1", "a2", "a3", "k0", "k1", "k2", "k3"]
        print_glyphs(fonts, [args.font] if args.font else every)
    elif args.cmd == "extended":
        print_extended(fonts)
    elif args.cmd == "sheet":
        sheet(fonts, args.out)
    else:
        default_kt = 0
        if args.text.startswith("@"):
            lines, default_kt = table_text(args.elf, args.text, args.parody)
        else:
            lines = parse_text(args.text)
        name, real = args.name.encode("cp932"), args.real_name.encode("cp932")
        if args.cmd == "measure":
            for ln in lines:
                measure(fonts, ln, name, real)
        else:
            font = args.font or f"k{default_kt}"
            bg = None if args.bg == "none" else tuple(int(v) for v in args.bg.split(","))
            colour = tuple(int(v) for v in args.colour.split(","))
            w, h = render(fonts, lines, font, args.out, args.width, args.center, args.scale, bg, colour,
                          args.shadow, args.line_height, args.packets, name, real)
            print(f"{args.out}: {w}x{h} ({len(lines)} lines, font {font})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
