---
number: 107
title: piros_colour in the field and dungeon: Piros tinted in the fights' draw
date: 2026-09-25
area: script, render, test
files: crates/piney-game/src/area_host.rs, crates/piney-game/src/area.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/combat/stage.rs, crates/piney-game/src/session/event22.rs, docs/engine/event-vm.md
---

# 107. piros_colour in the field and dungeon: Piros tinted in the fights' draw

Worklog 99 ported `piros_colour` for the town: `crate::piros::Sequence`,
and the town host's flashes and tint. Outside the towns the instruction
still fell to the host default. Event 22 uses it there in two blocks:
- block 16, in area 31's field;
- block 19, after block 18's `in_dungeon` for its dungeon.

Each runs every play pass while Piros is present. They mark him as the
target and hold the tint.

## The port

**The host.** `area_host.rs` now does what the town's host does:
- It builds the `Sequence` from `eventStatus[1]`, the line and whether
  Piros is here, and applies its first actions.
- `busy(PirosColour)` ticks it.
- Its actions go to the sound effects, the field UI's information line,
  the area's `scFadeDef` and Piros's tint. `area.rs` now sends
  `scFadeDef` on the font layer, as the town does.

**The tint.** Piros's tint lives on his actor in the fights.
`FieldWorld::set_affect_colour` finds his character by id and sets the
actor's `AffectColour`.

**The draw.** Like `ccChar::Draw`, the port takes the blend each time the
character is drawn: in the fellow's and Kite's draw hooks in `stage.rs`.
A flash's rate counts down there. `draw_cast` now draws the others through
`Body::draw_char_fog` with that blend.

## Checked

**`event_22_tints_piros_in_area_31`** plays event 22 in both places:
- It starts story:22 directly in area 31's field, and again in its first
  dungeon room.
- Piros is in the party, with `eventStatus[1]` 2: the potion drunk in the
  town.
- Both end with his character tinted fix 1, rate 65, 0x002080ff, and
  `take_unported()` empty.

**The board's blocks.** A first run did nothing:
- Every one of event 22's blocks depends on `eventStatus[0] == 1`, which
  the board's block 1 sets.
- Its block 0, which runs once at the first pass, sets it to 0.
- A start that skips the board has to put the 1 back after block 0 has
  run. The test does that during the set-up (`board_done`).

**The picture.** `event_22_dungeon_shot` turns the camera round with L1
after the tint and shows Piros orange and targeted in the dungeon.

**Still unknown:**
- **The fights' own tints.** The condition colours that `ccChar::Draw`
  applies after an affect's tint are not drawn on the fights' characters.
  Neither are the enemies' own affects: only this blend is taken.
- **Block 18's `in_point 5`.** The test's tint came in the dungeon's first
  room. Whether block 18's `in_point 5` gates block 19 there (event point
  5 being that room), or does not reach it, was not looked into.
