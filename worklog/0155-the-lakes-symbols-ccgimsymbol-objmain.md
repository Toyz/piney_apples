---
number: 155
title: "The lakes' symbols: ccGimSymbol::objMain"
date: 2026-09-26
area: battle
files: crates/piney-battle/src/gimmick.rs, crates/piney-battle/src/entry.rs, crates/piney-battle/examples/battle_probe/spawn.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/shrine.rs, docs/engine/field-ui.md
---

# 155. The lakes' symbols: ccGimSymbol::objMain

Worklog 0149 ported the story dungeons' symbols (gimmick row 17,
`symMain`) and left row 18, the lakes' `SYMBOL_OBJ`, with its constructor
only. In a lake dungeon (types 8 and 9) `DUNGEON::SetMagicCircle` puts one
at every rolled `OBJ_0ps*` slot of the rooms. They stood there dark and did
nothing, and SymbolMenu (36) on one never cast.

## objMain (gcmn 0x0045aed0)

`ccGimSymbol::main` (0x0045aa40) returns `objMain`'s result for row 18.
Unlike row 17 there is no body, no fires and no `initFlag` handling.

- **Every frame.** The light goes to 1 + `ccRandF(0.2)` at its place, 60
  above the symbol, taken to the player's frame (`ccTransPosFW2LW`), and
  into the group.
- **act 0.** The count goes up. Affect 11 (SymbolMenu's close) starts
  act 1.
- **act 1, frame 0.** Off the command lists (`deleteCmnd(1)`), sound 35 at
  the symbol, `effUseSymbol` at the light, and `ccItemSkillRequest(this,
  affectPerson, trapNum, 0)`. Unlike `symMain`, `param[2]` is left alone.
- **act 1, frame 30.** Act 2 with the light out.
- **act 2.** It returns 1, so the entry control deletes it and
  `ccDestGimSymbol` takes the light away.
- **Smoke.** On an even count, one puff: `effSmoke(pos, v, 1.0, 8, 204,
  512, 32)`. It starts 10 out from the light along a heading of three
  `ccRand()` turns (X, Y, Z) and flies out along it, 2.5 up. That is
  `invokeEff`'s heading with 2.5 in place of 2.0.

## The port

- `gimmick::obj_main` holds the above.
- `invoke_eff` and it share `scatter` for the heading.
- New `Out::SymbolSmoke { pos, v }`, which piney-game's fx sends to
  `effSmoke` with those arguments.

## Checks

`a_lake_symbol_glows_and_puffs` (session, in shrine.rs, whose walker it
uses) warps from Mac Anu with words 1, 136 (Solitary: field type 4, no
field, straight into a lake) and 150. A scan of the words found lakes
with and without symbols; this one rolls a symbol in room 1. Kite walks
there, and the symbol runs: act 0, its count past 20, its light in the
group. The session tests' `disc`, `start`, `hold`, `wait` and `playing`
became `pub(super)` for it.

**Still unknown:** `objMain` was not run in eemu against the port. The
cast on a lake symbol and the puffs on screen were not looked at.
