---
number: 102
title: Data Drain: DataDrainMenu's frames, its rules answered by the field, and the drops handed out
date: 2026-09-25
area: ui, battle, test
files: crates/piney-fieldui/src/menus/drain.rs, crates/piney-fieldui/src/ctrl.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/talk.rs, crates/piney-fieldui/src/tables.rs, crates/piney-fieldui/src/menus/mod.rs, crates/piney-fieldui/src/menus/getitem.rs, crates/piney-fieldui/src/menus/trade.rs, crates/piney-fieldui/src/menus/hack.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, tools/test_fieldui_rs.py, docs/engine/field-ui.md
---

# 102. Data Drain: DataDrainMenu's frames, its rules answered by the field, and the drops handed out

Picking Data Drain in a fight opened menu 66, which the port closed at
once: nothing was drained. That left no virus cores (they are drain
drops), so no gate hack after [[96]]. The rules were already in
`piney_battle::drain`, checked by `tools/test_battle_drain_rs.py`, but
nothing ran them.

## What the game does

`DataDrainMenu` (gcmn 0x00532ae0, 10 KB) runs these steps (the details
are on [the field UI page](../docs/engine/field-ui.md#data-drain-datadrainmenu-menu-66)):
- **0.** Noise and a fade to black, and the movie when `drainDemo` is on.
  Then the rules, which the port answers from the runtime: the boss
  affect, the infection, the drops into `drainItem`, and `ccDeleteCmnd`.
  The targets are cleared, a flash, the tasks woken, three frames.
- **1-3.** The side-effect roll, made twice from step 0. Without a side
  effect: noise, then "Viral infection has spread!".
- **10-12.** With one: the effect (`dataDrainErosionTbl`), noise, a level
  down at frame 30, and its message. Effect 30, SYSTEM ERROR, ends the
  game.
- **20.** `drainCount`, the bracelet's growth with its pages, then menu
  67 with the drops.

`waitCount` is zeroed only at step 0's start, so steps 2 and 11 go on
from the count step 0's fade left.

## The port

- **`crates/piney-fieldui/src/menus/drain.rs`.** Step 0's own frames are
  `Resume` steps (`After::Drain`), as the gate hack's are. Steps 1-20 run
  a frame each.
- **The rules are the runtime's.** `Request::DataDrain` and
  `Request::DrainSideEffect` need no mid-frame stop, unlike an item's use:
  the menu reads the drops only at step 20, and step 0's roll reads the
  save's infection three frames after the rules ran. The runtime answers
  at the frame's end with `FieldUi::drain_drops` and
  `drain_side_effect`.
- **The field world.** `FieldWorld::data_drain`, `drain_side_effect` and
  `kite_level_down` run `piney_battle::drain` on the combat scene.
  `area.rs` handles the requests. `WorldHidden`, which replaces the gate
  hack's `HackScreen` (as `MenuNoise` replaces `HackNoise`), blanks the
  world under the black.
- **`TalkState::drain_item` is now 17 long,** the game's +0x1ac-+0x1f0.
  67 read only 4, and the trade pages use the first 4.
- **The movie is not ported.** The port goes on as with `drainDemo` off.

## Checked

- **`tools/test_fieldui_rs.py`.** It runs the game's `DataDrainMenu`
  natively; 66 is off its unported list.
  - **Hooks.** The effects and fly fonts are hooked, `LevelDown` and the
    game over are logged, and the drain's noise and flashes are logged
    from its own code.
  - **The rules' results.** The infection, drops, side effect and the
    party's HP and SP after it are recorded as the game ran. They are fed
    to the probe in order, as the runtime's answers.
  - **`test_data_drain` (420 and 520 frames).** One run has no side
    effect: the warning, 20, 67 and the item's window. The other has one:
    its noise, its window, and the effect's heal on the party's panels.
  - **Result.** Every frame's state, packets, kanji, message, events and
    watched save bytes match. The ten random battle-menu runs now go
    through Drain too.
- **Along the way.** The side effect's party changes, the target dropped
  by `ccDeleteCmnd` (which keeps the sort chain) and several drains in one
  run were each found by a comparison failing. `drainDemo` 1 was found the
  same way: the game waits on its movie.
- **The other suites.** All six field UI suites (132 tests) pass.
- **`session::tests::data_drain_in_a_fight`.** After event 3, Kite is
  given Data Drain and the bracelet and walks to the east portal. Once
  the goblin's protect is broken, he drains it through PERSONAL, Skills
  and the Data Drain page. The area's rules run (`data_drain 2`), the
  infection moves, and the drop is handed out through 67 and 29.
- **The rest.** The workspace's tests, clippy, fmt and the docs check
  pass.

**Still unknown:**
- **The movie is not ported.** With `drainDemo` on (a new game's
  default), the game plays a stream chosen by the target: 109-111 for an
  enemy by `ccCheckObjectSize`, or one per character for a party member.
  `ccThDrainEnemy` (gcmn 0x00432000) changes the enemy on its frames 30
  and 229. A boss's or a character's movie plays even with `drainDemo`
  off. The port skips all of them.
- **The side effect's visuals are not drawn** except the heals:
  `effSkillStartEffect`, the fly fonts (miss, EXP lost, level down) and
  `effAfterDrain`.
- **The noise** (`MenuNoise`) is still not drawn.
- **The enemy's drained form after affect 13** is the battle's, and was
  not looked at here.
