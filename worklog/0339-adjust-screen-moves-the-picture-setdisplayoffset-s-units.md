---
number: 339
title: Adjust Screen moves the picture: SetDisplayOffset's units, and the presenter's shift
date: 2026-10-01
area: render, ui, engine
files: crates/piney-gs/src/lib.rs, crates/piney-game/src/main.rs, crates/piney-game/src/mode.rs, crates/piney-game/src/session.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, docs/engine/overview.md
---

# 339. Adjust Screen moves the picture: SetDisplayOffset's units, and the presenter's shift

Adjust Screen (the desktop's `ScreenMenu`, INF SLUS_202.67:0x0016d4d0, and
the field Option's, INF GCMN.PRG:0x0053f060) calls
`ccSystem::SetDisplayOffset(x, y)` every frame it is open. The port's menus
already asked for it (`Request::DisplayOffset`), but the game dropped the
request, so the option did nothing ([[50]], [[51]], [[196]]). The picture now
moves.

## What the offset is in

`SetDisplayOffset` (INF SLUS_202.67:0x0010ab30) keeps (x, y) at +0x20 /
+0x22 and writes base + x (clamped 0..4095) and base + y (0..2046) into the
DX and DY of both display buffers' DISPLAY register (+0xbf8, +0xc20). With
the field flag (+0xbd6) 0 it also writes them, y + 1, into read circuit 1's
pair at +0xe10 / +0xe20, the game's deflicker ([[340]]). The base (+0x1c, +0x1e) is DISPLAY's DX and DY
as `SetScreenModeMain` (0x0010ad40) leaves them after `sceGsSetDefDBuff`
(0x0010b57c-0x0010b594).

`ccSystem::Init` (0x0010a900) asks `SetScreenMode(512, 448, 0)`, which
`SetScreenModeMain` turns into `sceGsResetGraph(0, 1, 2, 0)`: NTSC,
interlaced, frame mode, one 448-line buffer. `sceGsSetDefDispEnv`
(0x0010de50) sets, for NTSC interlaced:

```
MAGH = (2559 + w) / w - 1      4 at 512 wide (5 clocks a pixel), 3 at 640
DX   = 636 + dx * (MAGH + 1)
DY   = 50 + dy
DH   = 2 h - 1 (field mode), h - 1 (frame mode)
```

So DX counts video clocks (2,560 across the picture at either width), and DY
counts lines of the interlaced frame: one row of the 448. Adjust Screen
steps x by 3 (-48..48) and y by 1 (-16..16), up to 9.6 pixels across and 16
rows down. An earlier draft of this change took y as half-lines and halved
it; DH = h - 1 for the 448-line frame mode shows a DY unit is a whole row.

MUT, OUT and QUA have the same `SetDisplayOffset` (MUT 0x0010abd0, OUT
0x0010b1a0, QUA 0x0010af70; OUT's and QUA's differ from INF's only in
instruction scheduling) and the same constants in `sceGsSetDefDispEnv`
(636, 50, 72, 656).

## The port

`Event::DisplayOffset { x, y }` carries the request out of the desktop,
the title and top page (through `desktop::event`), the town (`world.rs`)
and the fields (`area.rs`). Two places set it from the save, as the game
does through its own `SetDisplayOffset(screenX, screenY)`
([title](../docs/engine/title.md)): the title's boot, where the kept
settings (`crates/piney-game/src/settings.rs`, `screen_x` / `screen_y`) are already in the
save, and `LoadGame`.

The app keeps the last offset and hands `Presenter::present` a shift of
(x / 5, y) in pixels of the 512 x 448 picture. The presenter moves the
picture in its shader (`picture()`: the sample point less the shift, black
outside the picture), inside the same 4:3 box. A viewport moved by the shift
was tried first, but wgpu rejects a viewport that leaves the attachment,
which a window with no bars would do at any offset. The console overlay does
not move.

`present_moves_the_picture_by_the_display_offset` presents a cleared frame
moved an eighth across and down on a 600 x 450 surface: black left of x 75
and above y 56, the colour from there. `options_kept_across_the_parts`
writes `screen_x = -12`, `screen_y = 7` into the settings file and sees
`DisplayOffset { x: -12, y: 7 }` at power-on.

**Still unknown:** nothing about the offset. A television's overscan, which
hid some of the edge the offset brings in, is not modelled: the port shows
the whole frame and the black where it moved, by choice.
