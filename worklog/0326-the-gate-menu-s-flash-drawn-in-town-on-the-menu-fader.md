---
number: 326
title: The gate menu's flash drawn in town on the menu fader
date: 2026-10-01
area: ui, world
files: crates/piney-fieldui/src/lib.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/tests/dun_loireag.rs
---

# 326. The gate menu's flash drawn in town on the menu fader

[[68]] left the Chaos Gate menu's white flash out. `piney-fieldui`'s gate
menu asks for it as `Request::Flash` when the menu opens, when it is
cancelled and on the second page. The town listed the request among those
it drops, and the field had no arm for it.

`GateMenu` (INF gcmn 0x0055cbe0) calls `EntryFlash(this->+0xb8, 8,
0x3040c0c0, 0, 0, 512, 448)` at 0x0055cca8 and 0x0055cfb0. That is
`ccMenu`'s fader, which `piney-fieldui` keeps as `MenuCtrl::menu_fade` and
draws in `disp`. `tools/test_fieldui_rs.py` treats this flash as the
runtime's: it logs `flash` and leaves `menuFade` alone, so the request
stays. The town and the field now carry it out with `FieldUi::gate_flash`,
which is `menu_fade.entry_flash(8, 0x3040c0c0)`.

The test is `the_gate_menu_flashes` (piney-game, dun_loireag). A new game
logs in to Mac Anu at the gate and talks to it. When the gate menu is up,
`menu_fade` must hold an element of colour `0x3040c0c0` over 8 frames. It
fails with the request dropped (all four elements empty) and passes with
it.

The game has one fader at `ccMenu +0xb8`. The event VM's `fade` and
`fade_more` drive it too ([events](../docs/engine/event-vm.md)). The port
has two copies: `MenuCtrl::menu_fade` for the menus, and `ScFade` in the
field and town hosts for the scripts. Each has four elements of its own.
In the game, a script's fade and a menu's flash share those four, and
Mutation's `fade` (`ccScFade::Init`, [[325]]) also clears a menu's flash.

**Still unknown:** whether a script's fade and a menu flash ever overlap
in play, which is the only case the two copies differ in. Not surveyed.
The port keeps them apart.
