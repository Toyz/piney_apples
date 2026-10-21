---
number: 419
title: Kite wears the bracelet's colours in a later volume's new game: the town starts built from a new game's save now run the boot's ccStartEvent before the title's NewGame, as a player's New Game does
date: 2026-10-21
area: world, script, test
files: crates/piney-game/src/world.rs, crates/piney-fieldui/src/newgame.rs, crates/piney-game/src/start.rs, crates/piney-game/src/main.rs, crates/piney-game/src/session.rs, crates/piney-game/src/desktop.rs, crates/piney-game/src/session/tests/kite_colours.rs, crates/piney-game/src/session/tests/event11.rs, docs/engine/field-game.md, docs/engine/event-vm.md
started: 2026-10-21T11:22:21-06:00
took: 15m
---

# 419. Kite wears the bracelet's colours in a later volume's new game: the town starts built from a new game's save now run the boot's ccStartEvent before the title's NewGame, as a player's New Game does

`party_call_shots` (issue #68's test, Mutation) drew Kite grey: white hair
and hat on his model and on his HUD face, BlackRose as she should be. A
player's Mutation build draws him red and teal. The test, and every mode
or test that starts in a town from `world::new_game_state`, built the
save without the boot's `ccStartEvent`.

## What the game does

- Kite's grey is the game's own look before the Data Drain bracelet:
  with `saveData.plcol` (+0x6771) 0, `ccPlayer`'s constructor swaps
  `CLT_ctu1body` for `CLT_ctu1bodyc1`, and `ccCheckMenuFaceNameParty`
  shows face 18. Both are already ported (`Kite::swaps`,
  `FieldUi::set_menu_face`).
- On Mutation the only store to `plcol` besides `Init`'s 0 and the save's
  copies is the `data_drain` op (main 0x001c5bf0). It is scanned over
  main and every overlay with `tools/disasm.py`. Only Infection's event 11
  (`MG0150`, carried on every disc) uses the op.
- `ccThMother` runs `ccSaveData::Init(1)`, then `ccStartEvent(volumeNum,
  0)`. Only after that does the title run `NewGame(1)` and `NewGame(0)`.
  From volume 2 on, `ccStartEvent` brings events 0-49 forward at level 1
  (event 11 included), so a real New Game on Mutation, Outbreak or
  Quarantine has `plcol` 1. From Outbreak it also has `crisis` 1.

## The cause

`world::new_game_state` was `new_game_save`: `Init`, then the title's
`NewGame`, with no boot. On Infection that makes no difference
(`ccStartEvent(1, 0)` only marks event 0 done). On the later volumes it
left `plcol` 0 and the earlier volumes' events unplayed. The users:

- `--mode world`, `field:N` and `dungeon` built from it;
- `--mode world` with scripts booted a second time on top, out of order;
- the story starts (`start::build`, the console's `story N`) booted
  after `NewGame`;
- about forty tests built from it.

A player's path was right: `Session::boot_title` boots and then runs the
title. Two smaller paths were wrong:

- `--no-events` skipped the boot, and with it the boot's writes to the
  save;
- `--mode desktop` without scripts did the same.

Load and CONVERT read their saves whole. A cleared Infection save has
event 11 done, so `plcol` 1.

## Fix

- `world::new_game` builds a player's New Game: `Init`, `desktop::boot`
  (`ccStartEvent`), then `new_game_on` (the title's `NewGame(1)`,
  `NewGame(0)`, Kite's name) and the port's Helba mail. It returns the
  save and the boot's task.
- `new_game_state` is its save. `new_game_events` takes its task through
  the desktop and the board, so nothing boots twice.
- `start::build` and event 11's card test start from `new_game`.
- `new_game_save` (piney-fieldui) keeps `Init` + `NewGame` alone, for
  the eemu probe, and says so.
- `Session::boot_title` and `DesktopMode::new` always run the boot's
  `ccStartEvent` on the save. They keep its task only with scripts on.

## Check

`session::tests::kite_colours`, on all four discs:

- `the_built_new_game_is_the_title_s`: `new_game_state`'s save equals,
  byte for byte, the save a session's power-on, title and NEWGAME hand
  the desktop. The only bytes left out are the play time the desktop
  has begun to count. `plcol` is 1 on Mutation, Outbreak and Quarantine
  and 0 on Infection. Before the fix the first difference was Mutation's
  `plcol`.
- `kite_wears_the_bracelet_s_colours_in_a_later_volume_s_town`: Mac Anu
  on a new game. Kite's body has no palette swap and his face is 0 on the
  later volumes. Infection keeps `CLT_ctu1bodyc1` and face 18.

Also run: piney-game's suite (four threads, 301 passed), piney-fieldui's
tests, clippy, fmt.

Shots: before, /mnt/data/claude/scratch/i68/after/gather-020.png (grey
Kite). After, /mnt/data/claude/scratch/a419/shots/gather-020.png and
gather-060.png (red hat, teal hair, face 0).

**Still unknown:** nothing
