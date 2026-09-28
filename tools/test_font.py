#!/usr/bin/env python3
"""tools/font.py against the game's own text code run in tools/eemu.py:
ccGetExtendedCode over every code, ccKanjiStrWidth / Strlen / Strcat /
Separate over a sample of the game's text, ccFont::SetType and
ccSprite::MakePacketStr, and ccKanji::Disp (with Extract) texture byte for
byte and packets word for word. Infection in full; Mutation, Outbreak and
Quarantine, when extracted with a .syms sidecar, each against its own code
on a smaller sample. Skipped when an executable is absent.
`tools/font.py check ELF` runs the same over all of the text."""

import os
import struct
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402
ROOT = os.path.dirname(HERE)
ELF = volume.ELF
DATA = os.path.join(os.path.dirname(ELF), "DATA", "DATA.BIN")
OTHERS = [os.path.join(ROOT, "work", v, "disc", e) for v, e in
          (("mutation", "SLUS_205.62"), ("outbreak", "SLUS_205.63"),
           ("quarantine", "SLUS_205.64"))]

NAMES = [(b"Kite", b"Player"), (b"#R%A\x83\xa2x", b"\x81\x65Q")]
AREA = b"#B\x83\xa2 Boundless Corrupted Fort Walls"

# Per volume, by the executable's own volumeNum: the table strings corpus()
# finds, and the %x pairs ccKanjiStrlen counts (Outbreak's and Quarantine's
# count every pair; Infection's and Mutation's only %% and %#).
CORPUS = {1: 19135, 2: 19687, 3: 19976, 4: 19798}
STRLEN_COUNTED = {1: b"#%", 2: b"#%", 3: bytes(range(256)), 4: bytes(range(256))}

# What font.Game relies on, checked in each executable's code: the functions
# it hooks (or eemu hooks by name) are the ones the text code calls, and the
# globals it points at are the ones the text code reads.
CALLS = [("Extract__7ccKanjiFPc", "SetDataAdrs__9ccBltDataFP1"),
         ("Extract__7ccKanjiFPc", "ccGetExtendedCode__FUs"),
         ("Disp__7ccKanjiFPciff", "Extract__7ccKanjiFPc"),
         ("Disp__7ccKanjiFPciff", "SendPacket__8ccSpriteFv"),
         ("MakePacketStr__8ccSpriteFPci", "ApplyLayerScreenMatrix__6ccViewFPiPf"),
         ("MakePacketStr__8ccSpriteFPci", "sceVu0FTOI0Vector"),
         ("MakePacketStr__8ccSpriteFPci", "__fixunssfdi"),
         ("ccKanjiStrcat__FPcPci", "strcat"),
         ("ccKanjiStrWidth__FPci", "ccKanjiStrWidth__FPci"),
         ("ccKanjiStrlen__FPc", "ccKanjiStrlen__FPc")]
READS = [("ccKanjiStrWidth__FPci", "saveData"), ("Extract__7ccKanjiFPc", "ccSys"),
         ("MakePacketStr__8ccSpriteFPci", "fontTex")]
# The functions font.Game runs; the struct offsets it pokes are Infection's
# (DWARF), so each volume's code must use the same ones.
RUN = ["ccGetExtendedCode__FUs", "ccKanjiStrWidth__FPci", "ccKanjiStrlen__FPc",
       "ccKanjiStrSeparate__FPci", "ccKanjiStrcat__FPcPci", "Extract__7ccKanjiFPc",
       "Disp__7ccKanjiFPciff", "SetType__6ccFontFi", "MakePacketStr__8ccSpriteFPci",
       "MakePacket__8ccSpriteFii", "SendPacket__8ccSpriteFv", "ApplyLayerScreenMatrix__6ccViewFPiPf"]


def _words(prog, name):
    va = prog.symbol_named(name).value
    size = prog.function_at(va).size
    return va, struct.unpack(f"<{size // 4}I", prog.read(va, size))


def calls(prog, name):
    """The jal targets in a function."""
    va, words = _words(prog, name)
    return {((va + 4 * i + 4) & 0xF0000000) | ((w & 0x3FFFFFF) << 2)
            for i, w in enumerate(words) if w >> 26 == 3}


def reads(prog, name):
    """Every absolute or $gp-relative address a function forms."""
    import xfer
    va, words = _words(prog, name)
    return set(xfer.addresses(words, va, prog.gp).values())


def offsets(prog, name):
    """{(load or store, width, offset)} off every base but $sp, $gp and $at,
    offset 0 or more: the struct fields a function touches. ($at carries
    lui'd addresses; Infection's Extract writes texture bytes at -1 where
    Outbreak's and Quarantine's subtract 1 first.)"""
    kinds = {0x20: "Lb", 0x24: "Lb", 0x21: "Lh", 0x25: "Lh", 0x23: "Lw", 0x27: "Lw", 0x37: "Ld",
             0x1e: "Lq", 0x31: "Lw", 0x28: "Sb", 0x29: "Sh", 0x2b: "Sw", 0x39: "Sw", 0x3f: "Sd",
             0x1f: "Sq"}
    import xfer
    va, words = _words(prog, name)
    formed = xfer.addresses(words, va, prog.gp)
    out = set()
    for i, w in enumerate(words):
        if w >> 26 in kinds and i not in formed and (w >> 21) & 31 not in (1, 28, 29) \
                and not w & 0x8000:
            out.add((kinds[w >> 26], w & 0xFFFF))
    return out


def check_volume(test, font, fonts, game, texts, disp_texts):
    """The game's own text functions against font.py on one executable:
    every ccGetExtendedCode code, the string functions over `texts` plus the
    synthetic strings (the second name pair over the synthetic ones), SetType,
    MakePacketStr over a few cases, and Disp over `disp_texts` plus the
    synthetic strings that reach the escapes, in every kt."""
    F, f, g = font, fonts, game
    where = f.elf
    for code in range(0x10000):
        test.assertEqual(g.ext(code), F.extended_code(code), (where, hex(code)))
    for ni, (name, real) in enumerate(NAMES):
        g.set_names(name, real)
        for s in (texts + F.synthetic() if ni == 0 else F.synthetic()):
            for kt in range(4):
                test.assertEqual(g.width(s, kt), F.str_width(f, s, kt, name, real), (where, s, kt))
            test.assertEqual(g.strlen(s), F.str_len(s, name, real, f.strlen_counted), (where, s))
            for m in (0, 5, 12, 20):
                test.assertEqual(g.strcat(s, m), F.strcat_field(s, m, name, real), (where, s, m))
    g.set_names(*NAMES[0])
    test.assertEqual(g.strcat(None, 7), F.strcat_field(None, 7))
    blob = b"\0".join(texts[:100]) + b"\0"
    for k in range(0, 100, 9):
        test.assertEqual(g.separate(blob, k), F.str_separate(blob, k), (where, k))
    for t, v in F.FONT_TYPES.items():
        test.assertEqual(g.set_type(t), tuple(float(x) if k < 2 else x for k, x in enumerate(v)),
                         (where, t))
    for t in range(4):
        for ctrl in (1, 0x11, 0x21, 0x41):
            for raw in (False, True):
                # past the right edge of the screen too, where quads are culled
                s = b"HELLO\n1\x7f~" + bytes(range(1, 0x100)) if not raw else \
                    bytes(range(1, 41)) + b"\xfe12\xff"
                gq, gdx = g.font_str(s, t, ctrl, raw, dx=3.0, dy=5.0)
                spr = F.Sprite(ctrl=ctrl)
                spr.set_type(t)
                spr.dx, spr.dy = 3.0, 5.0
                pq = F.make_packet_str(spr, s, raw, F.Screen())
                test.assertEqual(gq, [F.quad_words(q) for q in pq], (where, t, ctrl, raw))
                test.assertEqual(gdx, spr.dx, (where, t, ctrl, raw))
    synth = [s for s in F.synthetic()[:24] if not F.wild(s) and (b"#" in s or b"%" in s or
                                                                 any(c >= 0x80 for c in s))]
    cases = [(s, -1, NAMES[0]) for s in disp_texts] + [(s, c, NAMES[1]) for s in synth for c in (-1, 5)]
    cases.append((AREA, -1, NAMES[0]))
    for i, (s, c, (name, real)) in enumerate(cases):
        g.set_names(name, real)
        for kt in range(4):
            colour = (128, 128, 128, 128) if i % 2 else (100, 90, 80, 64)
            gtex, gclm, gq, gdx = g.disp(s, kt, c, colour=colour, dx=7.0, dy=9.0)
            k = F.Kanji(f, kt, colour=colour)
            k.dx, k.dy = 7.0, 9.0
            pq = k.disp(s, c, name=name, real=real)
            test.assertEqual((gtex, gclm), (bytes(k.tex), k.clm), (where, s, kt, c))
            test.assertEqual(gq, [F.quad_words(q) for q in pq], (where, s, kt, c))
            test.assertEqual(gdx, k.dx, (where, s, kt, c))


@unittest.skipUnless(os.path.exists(ELF), "game executable not present")
class TestFont(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import font
        cls.F = font
        cls.fonts = font.Fonts(ELF)
        cls.game = font.Game(cls.fonts.p)
        cls.corpus = [s for _, s in font.corpus(ELF)]

    def setUp(self):
        self.game.set_names(*NAMES[0])

    # static facts -----------------------------------------------------------
    def test_tables(self):
        F, f = self.F, self.fonts
        self.assertEqual(len(self.corpus), 19135)
        self.assertEqual([f.trim(0, g) for g in (0, 1, 73, 76)], [1, 3, 5, 4])       # ' ' ! i l
        self.assertEqual([f.trim(2, g) for g in (0, 1, 73, 76)], [3, 6, 7, 6])
        self.assertEqual(f.colours[18][:3], (128, 56, 56))                           # #R
        self.assertEqual(f.colours[17][:3], (48, 80, 128))                           # #B
        for kt in (0, 2):
            k = F.KANJI[kt]
            blank = [g for g in range(F.GLYPHS)
                     if not any(any(f.row(f.glyph_src(kt, g) + r * k.stride, k.cw // 2))
                                for r in range(k.ch))]
            self.assertEqual(blank, [0, 64, 110, 111])
        self.assertEqual(F.percent_glyph(ord("0")), 95)
        self.assertEqual(F.expand(AREA), b"#B%0 Boundless Corrupted Fort Walls")

    def test_widths(self):
        F, f = self.F, self.fonts
        self.assertEqual([F.str_width(f, b"Power of the Bracelet", kt) for kt in range(4)],
                         [158, 168, 224, 252])
        self.assertEqual([F.str_width(f, AREA, kt) for kt in range(4)], [237, 256, 334, 384])
        self.assertEqual(F.str_len(AREA), 32)
        self.assertEqual(F.str_len(b"%0%A%%"), 1)           # only %% and %# count
        self.assertEqual(F.str_width(f, b"\x81\x65", 2), 0)  # the StrWidth quirk
        k = F.Kanji(f, 2)
        k.disp(b"\x81\x65")
        self.assertEqual(k.dx, 8.0)                          # while Disp draws it 8 wide

    # against the game ---------------------------------------------------------
    def test_extended_code(self):
        for code in range(0x10000):
            self.assertEqual(self.game.ext(code), self.F.extended_code(code), hex(code))

    def test_string_functions(self):
        F, f, g = self.F, self.fonts, self.game
        texts = self.corpus[::40] + F.synthetic()
        for ni, (name, real) in enumerate(NAMES):
            g.set_names(name, real)
            for s in (texts if ni == 0 else F.synthetic()):
                for kt in range(4):
                    self.assertEqual(g.width(s, kt), F.str_width(f, s, kt, name, real), (s, kt))
                self.assertEqual(g.strlen(s), F.str_len(s, name, real), s)
                for m in (0, 5, 12, 20):
                    self.assertEqual(g.strcat(s, m), F.strcat_field(s, m, name, real), (s, m))
        self.assertEqual(g.strcat(None, 7), F.strcat_field(None, 7))

    def test_separate(self):
        blob = b"\0".join(self.corpus[:300]) + b"\0"
        for k in range(0, 300, 7):
            self.assertEqual(self.game.separate(blob, k), self.F.str_separate(blob, k), k)

    def test_set_type(self):
        for t, v in self.F.FONT_TYPES.items():
            want = tuple(float(x) if k < 2 else x for k, x in enumerate(v))
            self.assertEqual(self.game.set_type(t), want)

    def test_make_packet_str(self):
        F = self.F
        for t in range(4):
            for ctrl in (1, 0x11, 0x21, 0x41):
                for raw in (False, True):
                    for s in (b"HELLO world 123", b"a\nb\x7f~", bytes(range(1, 0xff))):
                        data = s[:40] + b"\xff" if raw else s
                        gq, gdx = self.game.font_str(data, t, ctrl, raw, dx=3.0, dy=5.0)
                        spr = F.Sprite(ctrl=ctrl)
                        spr.set_type(t)
                        spr.dx, spr.dy = 3.0, 5.0
                        pq = F.make_packet_str(spr, data, raw, F.Screen())
                        self.assertEqual(gq, [F.quad_words(q) for q in pq], (t, ctrl, raw, s))
                        self.assertEqual(gdx, spr.dx)

    def test_newline_only_advances(self):
        gq, gdx = self.game.font_str(b"a\nb", 1)
        self.assertEqual(len(gq), 2)
        self.assertEqual(gdx, 36.0)
        self.assertEqual(gq[1][2][0] - gq[0][2][0], 2 * 12 * 16)

    def test_disp(self):
        F, f, g = self.F, self.fonts, self.game
        texts = [(s, -1, NAMES[0]) for s in self.corpus[::800]]
        texts += [(s, c, nm) for s in F.synthetic()[:24] if not F.wild(s)
                  for c in (-1, 5) for nm in NAMES]
        texts.append((AREA, -1, NAMES[0]))
        for i, (s, c, (name, real)) in enumerate(texts):
            g.set_names(name, real)
            for kt in range(4):
                colour = (128, 128, 128, 128) if i % 2 else (100, 90, 80, 64)
                gtex, gclm, gq, gdx = g.disp(s, kt, c, colour=colour, dx=7.0, dy=9.0)
                k = F.Kanji(f, kt, colour=colour)
                k.dx, k.dy = 7.0, 9.0
                pq = k.disp(s, c, name=name, real=real)
                self.assertEqual((gtex, gclm), (bytes(k.tex), k.clm), (s, kt, c))
                self.assertEqual(gq, [F.quad_words(q) for q in pq], (s, kt, c))
                self.assertEqual(gdx, k.dx, (s, kt, c))

    def test_area_quads(self):
        """The address line of mail 0 in the small font: two runs, a shadow
        under each, the second texture row starting at glyph 16."""
        F = self.F
        q = F.Kanji(self.fonts, 0).disp(AREA)
        self.assertEqual([F.quad_words(x) for x in q], [
            ((16, 16, 16, 128), (0, 2047), (28688, 29200), (1920, 1792), (30592, 29456)),
            ((48, 80, 128, 128), (0, 2047), (28672, 29184), (1920, 1792), (30576, 29440)),
            ((16, 16, 16, 128), (0, 1791), (30608, 29200), (1872, 1536), (32464, 29456)),
            ((48, 80, 128, 128), (0, 1791), (30592, 29184), (1872, 1536), (32448, 29440))])

    @unittest.skipUnless(os.path.exists(DATA), "DATA.BIN not present")
    def test_render(self):
        import tempfile
        with tempfile.TemporaryDirectory() as d:
            out = os.path.join(d, "t.png")
            w, h = self.F.render(self.fonts, [AREA], "k0", out)
            self.assertEqual((w, h), (245, 25))
            self.assertTrue(os.path.getsize(out) > 100)

    def test_addresses_from_code(self):
        """What font.py reads out of the code is Infection's named objects."""
        F, f = self.F, self.fonts
        sym = lambda p, n: p.symbol_named(n).value      # noqa: E731
        for n in ("ef8x16", "ef12x20", "englishFontOfsS", "englishFontOfsL", "ccSpriteColorTable"):
            self.assertEqual(f.va[n], sym(f.p, n), n)
        self.assertEqual(f.strlen_counted, STRLEN_COUNTED[1])
        d = F.Program(ELF, "desktop")
        self.assertEqual(F.mail_tables(d), {n: (sym(d, n), d.symbol_named(n).size // size)
                                            for n, size in (("MailTbl", 0x48), ("MailTblp", 0x48),
                                                            ("ReMail", 0x14), ("ReMailp", 0x14))})
        self.assertEqual(F.news_count(d), d.symbol_named("HtmlTbl").size // 0x1c)


class TestOtherVolumes(unittest.TestCase):
    """Mutation, Outbreak and Quarantine, each against its own text code
    (through the names piney-gen syms carries, and the addresses font.py
    reads out of the code where a name is missing or wrong). The fonts, trim tables and
    colours are Infection's; Outbreak's and Quarantine's ccKanjiStrlen counts
    every %x pair."""

    def test_volumes(self):
        import font
        from image import Program
        present = [p for p in OTHERS if os.path.exists(p) and os.path.exists(p + ".syms")]
        if not present:
            self.skipTest("no other volume extracted with a .syms sidecar")
        inf = Program(ELF) if os.path.exists(ELF) else None
        inf_fonts = font.Fonts(ELF) if inf else None
        for path in present:
            with self.subTest(path=path):
                fonts = font.Fonts(path)
                prog = fonts.p
                vol = prog.u32(prog.symbol_named("volumeNum").value)
                self.assertEqual(vol, OTHERS.index(path) + 2)
                for caller, callee in CALLS:
                    self.assertIn(prog.symbol_named(callee).value, calls(prog, caller), (caller, callee))
                for fn, glob in READS:
                    self.assertIn(prog.symbol_named(glob).value, reads(prog, fn), (fn, glob))
                if inf:
                    for fn in RUN:
                        self.assertEqual(offsets(prog, fn), offsets(inf, fn), fn)
                    # the fonts, trims and colours are Infection's; what lies
                    # past them (glyphs 112-130, trims 128-130) is not
                    for n, size in (("ef12x20", 13440), ("ef8x16", 7168), ("englishFontOfsS", 256),
                                    ("englishFontOfsL", 256)):
                        self.assertEqual(fonts.row(fonts.va[n], size),
                                         inf_fonts.row(inf_fonts.va[n], size), n)
                    self.assertEqual(fonts.colours, inf_fonts.colours)
                    self.assertEqual(fonts.escape, inf_fonts.escape)
                self.assertEqual(fonts.strlen_counted, STRLEN_COUNTED[vol])
                texts = [s for _, s in font.corpus(path)]
                self.assertEqual(len(texts), CORPUS[vol])
                check_volume(self, font, fonts, font.Game(prog), texts[::150], texts[::2500])


if __name__ == "__main__":
    unittest.main()
