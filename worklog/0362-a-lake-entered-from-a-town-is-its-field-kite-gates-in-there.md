---
number: 362
title: A lake entered from a town is its field: Kite gates in there and PERSONAL has Gate Out
date: 2026-10-03
area: world, ui
files: crates/piney-world/src/field_world.rs, crates/piney-game/src/session.rs, docs/engine/dungeon.md
---

# 362. A lake entered from a town is its field: Kite gates in there and PERSONAL has Gate Out

Issue #34: in the "jungle" areas there was no way to leave (no Gate Out
on PERSONAL), and Kite appeared with no gate-in swirl, though his
members had theirs. Those areas are the lakes. A lake is field type 4: it
has no field, and the area opens on its first dungeon, the lake itself
([dungeon.md](../docs/engine/dungeon.md)). The port treated that dungeon
as any other dungeon, which was wrong in two places.

## Kite's arrival

`ccPlayer::ccPlayer` (INF gcmn.prg:0x00597d84) uses the field rule when
`GetFieldType()` is 4 and `game.dungeon` is 0: act 13 (the gate-in) when
`areaPrev` is 0, the town. `FieldWorld::reboot` had only the plain rule,
in which a dungeon never arrives. It now takes the field rule for a
lake's dungeon 0 entered from a town. Coming up from the lake's second
dungeon (`areaPrev` 2), Kite still stands, as before.

## PERSONAL

`PERSONAL` (INF gcmn.prg:0x005181d0) opens menu 1 (the field's, rows 4,
5, 6, 7, 8, 63, 87, 10, Gate Out last) in a dungeon of type 8 or 9, and
menu 2 (no Gate Out) in any other. [field-ui.md](../docs/engine/field-ui.md)
already said so. The port's menu input held `dungeon_type: 0`, so every
dungeon got menu 2. It now passes `WORLD_MAN.dungeon_type[game.dungeon]`;
a lake's are 8 and 9.

## Test

`a_lake_entered_from_town_is_its_field` (piney-game) enters story area 33
(a lake, field type 4) from the town with `areaPrev` 0. Kite's act reaches
13, and the triangle opens menu 1, whose last row is 10 (Gate Out). Each
half of the fix, undone alone, fails it.

**Still unknown:** nothing.
