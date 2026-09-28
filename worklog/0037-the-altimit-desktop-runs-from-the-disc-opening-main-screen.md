---
number: 37
title: The ALTIMIT desktop runs from the disc: opening, main screen and mail
date: 2026-09-23
area: ui, render, decomp, test
files: crates/piney-desktop, crates/piney-draw, crates/piney-gs, crates/piney-game, crates/piney-input, tools/desktop_tables.py, tools/test_desktop_rs.py, docs/engine/desktop.md
---

# 37. The ALTIMIT desktop runs from the disc: opening, main screen and mail

The user asked for more than a model viewer: a game viewer that boots into
the ALTIMIT desktop and can be navigated. This entry is the first half of
that:
- a runtime that runs a mode on the game's own clock and draws what the game
  would send the GS;
- the desktop itself, ported from `DESKTOP.PRG`.

A helper agent reverse engineered and ported the desktop. I wrote the
runtime, the pad reading and the GS back end, and integrated the two.

## Shape

The desktop is one of `ccThMother`'s modes (request 3, `DESKTOP.PRG`, see
[the overview](../docs/engine/overview.md)). The port keeps that seam: each
part of the game is a mode that takes the pad and returns one frame's draw
list.

- **`piney-input`** is `ccPad::Read` (0x00102d40) and `SetAnalogStick`:
  - pressed, released and repeat bits (a held combination repeats after 15
    frames);
  - the left stick turned into D-pad bits by eighths.

  The game's quirks are kept: a stick with its raw y exactly 128 reads as
  down, and the up-right eighth presses right alone.
- **`piney-draw`** is what the EE sends the GS, as data:
  - primitives in frame-buffer pixels, with the ALPHA register, alpha test,
    Z test and SCISSOR;
  - model draws through VU1, per mmat, with the transparency, STROW, wrap and
    alpha reference the material packet carries;
  - run-time textures (the text `ccKanji` rasterises).
- **`piney-gs`** draws that on wgpu, in the game's 512 x 448 frame buffer:
  - it stays in the art's gamma, where the GS blends, and is shown at 4:3;
  - the ALPHA register's `((A - B) * C >> 7) + D` becomes blend factors, with
    FIX as the blend constant;
  - TFX and TCC are in the shader, 0x80 = 1.0;
  - GS Z is larger-nearer;
  - a failing FB_ONLY alpha test gates only the Z write, done as two passes;
  - the GS's pixel-corner rule becomes a half-pixel shift.

  GPU tests check exact pixel coverage, MIX and ADD blending, the alpha test,
  scissor, MODULATE at 0x80 against the stored texture, and the presenter's
  letterbox and sRGB round trip.
- **`piney-game`** runs a mode on NTSC vertical blanks (59.94 a second),
  stepping it every `frameRate` of them. `--shot` plays scripted presses with
  no window.
- **`piney-desktop`** is `Desktop_control` and `MailList_control`, with the
  main executable's 2D and 3D draw paths they use (`ccSprite`, `ccLayer`,
  `ccKanji`, `ccAnm::Draw`, `ccView`).
- **Content** is read at run time from `DESKTOP.PRG` and `SLUS_202.67` on
  the image: mail and reply text, wallpaper and music tables, the kanji
  fonts. The engine's own tables (glyph trims, escape colours, the blend
  table, mail reply links) are generated Rust from `tools/desktop_tables.py`.

## What the desktop is

Most of it is 3D: `xddesk01`'s models animated by `ccAnm`, seen through a
camera (`ANM_xddcamer`). On top of that:
- 2D sprites from `TEX_xddcurs1` for cursors, icons and window tiles;
- text as run-time PSMT4 textures.

2D is drawn through the default layer, which maps logical coordinates to GS
ones with a 7/6 vertical stretch (XYOFFSET 0x7000, 0x7200).

**Draw order.** Layers go in ascending priority: 126 the 3D back layer, 127
kanji, 128 and 129 the mail windows, 240 font and fade. Within a layer,
opaque models and sprites go in reverse call order, because both are
prepended to the layer's list. Translucent mmats come after them, back to
front by screen Z.

The saveData the desktop reads is the shared `piney_data::save::SaveData`:
- `mailList` / `mailOrderList`, through `NewMail`, `ReadNewMail` and
  `CheckMail`;
- the wallpaper and music choice;
- the assigned confirm and cancel buttons.

It also reads two `ccEvent` members, the operation lock `operate` and
`operateSet`. The event engine writes them.

## Checked

`tools/test_desktop_rs.py` runs the game's own functions in eemu against the
port:
- `SelectMode`;
- `DrawLogo`, over 1,024 cases;
- `NewIconDraw`, over 700 frames, bit-exact;
- the mail list's `MoveLine`, `SetLength` and `TimeAlphaCurDraw`;
- `ccKanji`'s layout against `tools/font.py` over the mail tables.

All pass. The camera was checked on eemu points: (0,0,0) lands at pixel
(256, 224) and (100,0,0) at (257, 224).

Through `piney-game` and `piney-gs`:
- **Opening.** The icons fly in, with sound effects 0 and 1 and music 50
  requested.
- **Main screen.** THE WORLD, MAILER, NEWS, ACCESSORY, AUDIO, DATA, each
  mode's panel, the ALTIMIT logo, wallpaper 49.
- **Navigation.** Down twice selects ACCESSORY with the cursor sound.
- **Mail.** With mails 4, 5 and 320 delivered (`--mail`; event 1 gives a new
  game these), the Mailer shows "You have 3 new mails.", the inbox lists CC
  Corporation, CC Corporation and Yasuhiko, and the first opens with the CC
  logo and its text.

**Still unknown:**
- **The other modes.** News, Accessory (wallpaper), Audio and Data, which the
  agent is porting next.
- **The rest of the game around it.**
  - The event scripts that deliver mail and lock icons: the event engine is
    being built, and until then mail comes from `--mail`.
  - Sound: requests are printed, not played.
  - What comes before and after the desktop: `DEMO.PRG`'s title and opening,
    name entry, and The World's `TOPPAGE.PRG`.
- **Two renderer approximations.** The 2-pass FB_ONLY differs from the GS
  where one draw overlaps itself, and the GS's 8-bit rounding inside the
  pixel pipeline is not modelled.
