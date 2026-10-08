---
number: 400
title: A new save's Rankings shows the town's racers: the port's empty ranks already take race_ranks in turn, as the game's page and ranking do; the old shot showed the test's own seeded records
date: 2026-10-08
area: ui, test
files: crates/piney-game/src/session/tests/flag_race.rs, docs/engine/field-ui.md
---

# 400. A new save's Rankings shows the town's racers: the port's empty ranks already take race_ranks in turn, as the game's page and ranking do; the old shot showed the test's own seeded records

[[395]]'s Rankings shot (`/mnt/data/claude/scratch/i56/mut-race-rankings.png`)
showed 2nd and 3rd place as "Kite / Iron Grunty 05:00:00". Was the
port's fallback to the town's racers wrong?

## What the game does

- **The racers.** `race_ranks` (MUT gcmn 0x006d8610) holds three a town,
  each a name, a time in frames (30 a second) and a Grunty's `npcTbl` row.
  Dun Loireag has Balmung 600 (0:20), Gardenia 626 and Cima 646.
- **A new save.** `Init` (MUT 0x00175740) zeroes the records at +0x8432.
  The only other code that touches them is `MainProccess`'s copies to and
  from the card (MUT 0x001732d4, 0x0017411c, 0x00174dc8). No `NewGame` or
  conversion writes them.
- **The Rankings page** (0x0058ca90). A rank whose time is 0 shows the
  town's next racer in turn.
- **The ranking** (main 0x0017a860) does the same:
  - it compares the time with each rank, taking an empty rank's racer in
    turn;
  - on a new rank it moves the lower records down, empty ones included.

  So after a first win from a new save, ranks 2 and 3 are empty and show
  Balmung and Gardenia.

## Cause

The port does the same: `rankings_menu_disp` takes `race_ranks` in turn,
and `test_rankings` in tools/test_fieldui_talk_rs.py matches the game.
That includes a player's rank at 1 with 2 and 3 empty, the case after a
first win.

The shot's 2nd and 3rd were the test's own records. `flag_race_shots` and
`mutations_flag_race_won` set all three of Dun Loireag's ranks to 9000
frames (5:00) with Grunty 146 (Iron Grunty), so the race would win.
Records hold no name, so the page names them the player's: "Kite".

## Fix

Nothing in the port.
- `flag_race_shots` now starts from a new save's records.
- `mutations_rankings_show_the_towns_racers` checks the page's three
  names. A new save shows Balmung, Gardenia and Cima. With the player's
  time first, it shows Kite, Balmung and Gardenia.
- `mutations_flag_race_won` still sets slow records; its ride's driver
  is the next entry's.

## Checked

- `mutations_rankings_show_the_towns_racers` and
  `mutations_rankings_open_and_close` pass.
- The new shot, `/mnt/data/claude/scratch/i56/c/mut-race-rankings.png`:
  Kite 00:16:50 first, then Balmung / Iron Grunty 00:20:00 and Gardenia /
  Iron Grunty 00:20:86.
- piney-game's suite (four threads), clippy, fmt, `cairns check` and
  `tools/docs.py check` pass.

**Still unknown:** nothing.
