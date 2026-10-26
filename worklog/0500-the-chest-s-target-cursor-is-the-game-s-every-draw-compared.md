---
number: 500
title: The chest's target cursor is the game's: every draw compared for a chest target, and the ghost square is the diamond cell's own inner square
date: 2026-10-25
area: ui
files: tools/test_fieldui_rs.py, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-game/src/session/tests/boxes.rs, BUGS.md
started: 2026-10-25T23:38:12-06:00
took: 13m
---

# 500. The chest's target cursor is the game's: every draw compared for a chest target, and the ghost square is the diamond cell's own inner square

BUGS.md: the chest's target cursor shows a ghost square. The earlier
finding was that the four corner arrows and the diamond are the game's;
which square was the ghost had not been pinned down.

**Every path the cursor takes for a chest** (`Disp`'s target cursor, INF
gcmn 0x0051ecbc - 0x0051f610; the port's `disp::target_cursor`):
- **the four corner arrows:** `targetCursol` cells 0-3 at (1584, 2048),
  14 x 16, pulsing out from `cursolAlpha`;
- **the diamond:** the 24 x 24 cell at (2464, 2048), drawn 32 x 32,
  scaled by the pulse and turned pi/4 (gcmn 0x0051f36c);
- **no attack cursor:** it is drawn only with `plAttack` on an enemy;
- **no subTarget squares:** these are filled only by the target and chat
  menus for an area skill's other targets;
- **the target window:** the name ("Treasure"), the action
  ("Investigate").
`tools/test_fieldui_rs.py` `test_target` now has a Treasure case (type
0x100000). It compares every draw of the game's `Disp` with the port's
over 70 frames from the cursor's first pulse, and none differs.
Running the harness first needed the probe's `Request` match to take the
new `FieldSeed` (it would not build). The whole harness then passed (74
tests).

**The squares are in the art.** `xwindow::TEX_xwindo00`, which
`targetCursol` draws from:
- The diamond's cell at (154, 128) is a dark square outline, a white
  band and a second, inner dark square. Turned pi/4 it shows two
  concentric squares, the inner one thin and dark.
- Each corner arrow's 14-wide cell carries a dark edge along its whole
  width, past the triangle.
The new ignored session test `chest_cursor_shots` walks Kite to area
18's B2 story box until it is his command target. Its shots show exactly
these pieces and nothing else. The strips follow the GS's rules (worklog
499).

So nothing in the port draws a square the game does not. The ghost is
the cell's inner square.

**Checked.** test_fieldui_rs.py (74 tests), the session shots, and
piney-game's suite (336).

Docs: BUGS.md.

**Still unknown:** nothing
