---
number: 193
title: "ALL FIELD PORTALS OPEN: the banner of an area's last portal (ccThDfComp); the entries' placeless sounds"
date: 2026-09-26
area: ui
files: crates/piney-fieldui/src/dfcomp.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/spr.rs, crates/piney-fieldui/src/render.rs, crates/piney-fieldui/src/disp.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session.rs, docs/engine/field-ui.md, docs/engine/effects.md, docs/engine/battle.md
---

# 193. ALL FIELD PORTALS OPEN: the banner of an area's last portal (ccThDfComp); the entries' placeless sounds

After the boss's `ClearSpcCondition` (0192), the same check was run on
the entry control's outputs. Seven variants of `piney_battle::entry::Out`
are matched nowhere in piney-world or piney-game; the combat passes them
on as `Show::Entry`, and nothing reads them. Five have their effect made
elsewhere:

- `GoldRanges`: the AI's `think` reads the widened ranges.
- `Clut`: the looks' palette swap for rows 154-157.
- `CircleParts`, `PartDraw` and `Layer`: the effects draw the portals.

Two were real gaps.

**`Se2d`, `ccSeOn(se)`.** The spring's sounds (215, 104), the boxes' (104)
and the virus crystal's (75) were never played. `area.rs` now sends each
as `Event::Se`, as it does for the boss's `ccSeOn`.

**`AreaCleared`, `ccStartThread(ccThDfComp, 33, 4096)`.** The circle
raises it at act 4 when it is the area's last. `effects.md` listed
`ccThDfComp` as unknown. It is the banner the game shows then. `Init`
puts a `ccSprite` on `xwindow::TEX_xwindo01` on the menu's layer. That
texture holds "ALL DUNGEON / PORTALS OPEN" at rows 0-31 and "ALL FIELD /
PORTALS OPEN" at rows 33-63.

`Main` draws the two lines for 85 frames, then 5 more with nothing:

- They come in from the sides: the slide is a sum growing by 1/2.75 each
  step, one step fewer each frame.
- They fade in over 15 frames to 96, and hold.
- They fade out over the last 10 while the scale grows by 0.06 a frame
  and the lines part.

A field, and a lake's first dungeon, take the field banner. Any other
dungeon takes the dungeon one, its top line 5 to the right. The full
timeline is in field-ui.md, "All portals open".

`piney_fieldui::dfcomp::DfComp` is that task. It runs after the menu's
task, using a new `Obj::DfComp` and its texture. The area starts it with
the scene's kind, the field type and the dungeon.

Checks:

- `its_frames`: the alpha 0, 6, 12 ... 96 ... 9, the rows, the slide
  finished by frame 15 with the halves at 252 and 260, the scale 256 wide
  then growing, and the dungeon's offset.
- `the_last_portal_puts_up_its_banner`: an `AreaCleared` in event 3's
  field keeps the banner up for 90 frames.
- `last_portal_banner_shots` (ignored; `PINEY_SHOTS`) puts "ALL FIELD /
  PORTALS OPEN" in yellow over the field at frame 40.

**Still unknown:** The banner was not compared with the game's pictures.
What reads the save's portal counts (+0x6864, +0x6866, +0x6868) is still
unknown.
