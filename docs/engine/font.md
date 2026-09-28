---
title: Text rendering
status: partial
volumes: all
covers: INF SLUS_202.67:0x0015e380 ccKanji::Disp, 0x0015d490 ccKanji::Extract, 0x0015d390 ccGetExtendedCode, 0x0015f2a0 ccKanjiStrWidth, 0x0015f120 ccKanjiStrlen, 0x0015eed0 ccKanjiStrcat, 0x0015f860 ccKanjiStrSeparate, 0x0015aed0 ccSprite::MakePacketStr, 0x0015cc20 ccFont::SetType, 0x0015ca70 fontSetup, 0x002f5600 ef8x16, 0x002f2180 ef12x20, 0x002fb5f0 englishFontOfsS, 0x002fb4f0 englishFontOfsL, 0x002fb430 ccSpriteColorTable; QUA DATA/KFED.BIN, DATA/KFAED.BIN
worklog: 23, 25
---

# Text rendering

How a string becomes glyph quads. The strings themselves, and their `#` and
`%` escapes, are on [the desktop text page](text.md). `tools/font.py`
reproduces both drawing paths, and `font.py check` runs the game's own text
functions in `tools/eemu.py` over all 19,135 table strings plus 125 synthetic
ones: 0 mismatches.

There are two paths:
- `ccFont` draws fixed-cell ASCII from a texture.
- `ccKanji` rasterises proportional text from bitmap fonts compiled into the
  executable.

Both use the palette `CLT_xasc00`: index 0 is clear, 1-3 white, 4-15 a
black-to-white ramp, alpha 0x80. TEX0 has TCC on and TFX MODULATE
(`ccTexChunk::SetBuffAdrs`, 0x00137090); TEX1 sets only MXL, so sampling is
nearest. The blend is `alphaBlendTbl[0]` (0x00348580),
`(Cs - Cd) * As + Cd`.

## ccFont: fixed cells

`fontSetup` (0x0015ca70) loads `xasc00::TEX_xasc00`, and
`ccSprite::MakePacketStr` (0x0015aed0) draws from it. `ccFont::SetType`
(0x0015cc20) picks the grid; u and v are in the upright texture:

| type | cell | origin | cells a row | holds |
| ---: | --- | --- | ---: | --- |
| 0 | 8×12 | (128, 168) | 16 | ASCII 0x20-0x7f |
| 1 | 12×16 | (0, 0) | 16 | ASCII 0x20-0x7f; 0x7f draws Δ |
| 2 | 10×12 | (128, 96) | 12 | 0x20-0x60, capitals only |
| 3 | 14×16 | (192, 0) | 4 | big digits: byte 0x20+n is cell n |

- Type 3's cells read ` 0123456789MISEXPLVDOWN-`. The game really stores
  its strings that way: `gcmn` holds `+,--` for MISS and `./0` for EXP.
- A byte in 0x21-0x7f draws cell `c - 0x20`; any other byte only advances.
  The advance is the cell width, with no kerning and no escapes.
- The texture rows are stored bottom-up, and V runs from `(256 - v)·16 - 1`
  down, which draws the glyph upright.
- In string mode, 0x0a sets a newline flag and is then skipped as a control
  byte, so `\n` only advances one cell. Only raw mode (`MakePacket`: 0xff
  ends, 0xfe starts a line) breaks lines.

## ccKanji: proportional text

Mail, the board, dialogue and menus. `ccKanji::Disp` (0x0015e380) runs in two
passes.

**Pass 1: expansion** into an 82-byte work buffer, which has no bounds check
(no table string expands past 81 bytes):
- `#0` becomes `plName` and `#1` becomes `plRealName`.
- Other `#x` and `%x` pairs are kept.
- A Shift-JIS pair goes through `ccGetExtendedCode`. If it maps, it becomes
  that `%x`. If not, it becomes `_` and the scan advances **one** byte, so
  the trail byte is read again as a character. The Japanese parody text
  therefore renders as `_r______` and the like.

**Extract** (0x0015d490) rasterises the buffer into the sprite's own 4-bit
texture, 128 texels wide, from one of two bitmap fonts:

| font | VA | sheet | row bytes | used by |
| --- | --- | --- | ---: | --- |
| `ef8x16` | 0x002f5600 | 128×112 | 64 | kt 0, 1 |
| `ef12x20` | 0x002f2180 | 192×140 | 96 | kt 2, 3 |

- Both are 4-bit, 16 glyphs a row with rows top-down, 112 glyphs each.
- Glyphs are written into the texture bottom-up, in VRAM order.
- kt 0 and 1 put 16 glyphs in a texture row. kt 2 and 3 put 10 a row, in 6
  rows starting 8 texel rows up.
- **kt 0 and 2 are proportional.** Each glyph is copied
  `(cell + 1 - trim) / 2` bytes wide, shifted left by the trim so far (an odd
  shift is done with a nibble shift). The rest of the row is filled with the
  first byte of glyph 0's rows.
- **kt 1 and 3 are fixed.** Each glyph moves right by `2·(trim / 4)` texels.
- The trims come from `englishFontOfsS` (0x002fb5f0) and `englishFontOfsL`
  (0x002fb4f0), both s16[128]. The advance is `8 - S[g]` for kt 0 and
  `12 - L[g]` for kt 2: a space is 7 / 9, an `i` 3 / 5.

**Pass 2: runs.** One SPRITE is drawn per run of glyphs sharing a colour and
a texture row, within `packetMax` packets (16 for mail and dialogue).
- Every run is preceded by a shadow quad (ctrl 0x10, colour (16,16,16,A),
  offset one view unit right and down), so 16 packets hold 8 runs.
- Sprite widths differ: kt 2 has `sx = w`, `su = w - 1`; kt 0 has
  `sx = w - 1`, `su = w`, then `dx += 1`; kt 1 and 3 have `sx = su = w`.

The colour escapes index `ccSpriteColorTable` (0x002fb430, u64 RGBA[24]) and
keep the current alpha. An unknown `#x` still ends the run.

| escape | entry | RGB |
| --- | ---: | --- |
| `#R` | 18 | 128, 56, 56 |
| `#G` | 20 | 72, 128, 72 |
| `#B` | 17 | 48, 80, 128 |
| `#Y` | 6 | 128, 128, 0 |
| `#W` | - | the starting colour |

**kt by caller.** kt is 0 after `Init`:
- kt 2: dialogue (`ccMessage`), loading screens (`ccLoadDisp`), BOOK;
- kt 3: name entry, the staff roll;
- kt 1: BOOK's pages;
- kt 0: chat, WORLD, DUNGEON, the mailer, the board, most menus.

## Glyph numbers

| input | glyph |
| --- | --- |
| ASCII `c` | `c - 0x20` |
| any other byte | 63 (`_`) |
| `%0`-`%9` | 95-104: Δ Λ Σ Ω Θ ○ △ □ ×, and `%9%A` together one ★ (two wide) |
| `%A`-`%Z` | 105-130: `%B%C` one ⊘ (two wide), `%D` ‘, `%E` ®, `%F` `%G` blank |
| `%%`, `%#` | `%`, `#` |
| any other `%x` | nothing |

`%H` onward reads past the 112 glyphs. `%X`-`%Z` also take their trim from
past `englishFontOfsS` (from `cameraList`, e.g. −15576), which sends
Extract's writes outside the texture. The game does not guard against this;
`font.py` does. The last text row sits at the texture's bottom texel row,
so its first glyph starts at byte 0: a full texture (the PERSONAL list's
eighth row, "Log Out") is written there like any other, and only a nibble
shift at byte 0 would write the byte before the texture. An earlier guard
in the port dropped that first glyph whole (" og Out");
`tools/test_font.py` and `tools/test_desktop_rs.py` now draw a string that
fills every row.

`ccGetExtendedCode` (0x0015d390) maps ten Shift-JIS codes and returns
everything else unchanged:

| Shift-JIS | 83a2 | 83a9 | 83b0 | 83b6 | 83a6 | 819b | 81a2 | 81a0 | 817e | 8165 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| char | Δ | Λ | Σ | Ω | Θ | ○ | △ | □ | × | ‘ |
| becomes | `%0` | `%1` | `%2` | `%3` | `%4` | `%5` | `%6` | `%7` | `%8` | `%D` |

## Measuring

The game never wraps by width: bodies are NUL-separated lines.

- **`ccKanjiStrSeparate(m, n)`** (0x0015f860) skips n lines. A byte outside
  0x20-0x7f takes the next byte with it.
- **`ccKanjiStrWidth`** (0x0015f2a0) counts 8 per glyph for kt 1 and 12 for
  kt 3. It uses S for kt 0 and L for every other kt, proportionally, which
  is not what Disp draws. For a Shift-JIS pair that maps to `%A`-`%Z` it
  tests the pair's own trail byte, so ‘ measures 0 though Disp draws it 8 or
  2 wide. Callers centre with it, e.g. `ccMessage` uses `256 - w/2`.
- **`ccKanjiStrlen`** (0x0015f120) counts `%%` and `%#` as glyphs, but
  `%0`-`%9` and `%A`-`%Z` count 0.
- **`ccKanjiStrcat(s0, s1, m)`** (0x0015eed0) pads or cuts `s1` to m glyphs
  with spaces, copying `#x` pairs without counting them.

## The kanji fonts

Infection also carries four 4-bit kanji sheets that nothing references (no
relocation in main or any overlay, and no unrelocated reference either):

| blob | VA | bytes |
| --- | --- | ---: |
| `kf14x16` | 0x001dea80 | 385,280 |
| `kf20x20` | 0x0023cb80 | 688,000 |
| `kfa20x20` | 0x002e4b00 | 35,200 |
| `kfa14x16` | 0x002ed480 | 19,712 |

- `kf20x20` is a sheet 320 texels wide: 160-byte rows, 16 glyphs of 20×20 a
  row, 215 rows, so 3,440 glyphs. Rows run top-down with the low nibble on
  the left, in Shift-JIS order from 0x8140.
- `kfa20x20` holds Δ Λ Σ Ω Θ, then kaomoji and extra kanji. So the parody
  text's line-final `%B`, `%D`, `%E`, `%G`, `%I`, `%J`, `%L`, `%M` are
  probably the Japanese build's face marks (inference). The US build draws
  them as ⊘ halves, ‘, ®, blanks, or garbage past the bitmap.
- Mutation and Outbreak carry all six font blobs byte-identical.
- Quarantine keeps only `ef12x20` and `ef8x16` in the executable. It names
  `\DATA\KFED.BIN` and `\DATA\KFAED.BIN`, the same sizes and sheet layout as
  `kf20x20` and `kfa20x20`, with most of the ink in the same places (523k of
  about 630k-660k inked nibbles overlap).
- KFED's bytes differ from `kf20x20`'s because the index use changed:
  Infection's background is index 1 and Quarantine's is 0, and the shading
  values are not a permutation of each other. So the glyphs were
  re-rasterised or re-shaded (inference).

## Other volumes

`font.py check` on each volume's own code:
- MUT: 19,687 table strings, 3,148 `Disp` calls;
- OUT: 19,976 table strings, 3,148 calls;
- QUA: 19,798 table strings, 3,156 calls.

All 65,536 extended codes were checked on each. 0 mismatches. `font.py`
reads the fonts, trims and colour table through `Extract`'s and `Disp`'s own
address loads. They are identical on all four volumes, but at different
addresses: e.g. MUT's `ef12x20` is at `0x00307a00` and its
`ccSpriteColorTable` at `0x00310cb0`.

The one change: **`ccKanjiStrlen` in OUT and QUA** (OUT `0x0015e1b0`) counts
every `%x` pair as a glyph. INF and MUT count only `%%` and `%#`. `font.py`
reads which by walking the executable's own function.

Text counts from the code: 375 mails (INF 326), 364 replies (INF 294) and 355
board posts (INF 341) from MUT on.

Quarantine reads `KFED.BIN` and `KFAED.BIN` only in event opcode 169, during
the ending ([events](events.md#other-volumes)); the text code never touches
them.

## Unknown

- How the GS turns the packets into pixels. The renderer assumes top-left
  sampling, which matters only for the one-texel `sx`/`su` mismatch of kt 0
  and 2; the packets themselves are verified.
- The display scale of the font layer. Logical space is 512×384 and
  `ccView::SetLayerCenter` scales to 512×448, but the view the font layer uses
  was not traced; renders are 1:1 in texels.
- Line spacing of mail and board bodies (caller code, not traced; renders
  use the cell height).
- Where `ccKanji`'s texture sits in VRAM next to `xasc00`.
- What the ending routine (QUA gcmn `0x004f0a30`) does while the KFED
  buffers are loaded.
