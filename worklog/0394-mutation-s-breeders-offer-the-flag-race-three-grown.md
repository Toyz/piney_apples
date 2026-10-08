---
number: 394
title: Mutation's breeders offer the Flag Race: three grown Grunties in the town's pens and the race's mail read, then Talk, Flag Race and Rankings
date: 2026-10-08
area: ui, volumes, save, test
files: crates/piney-fieldui/src/menus/breeder.rs, crates/piney-fieldui/src/ctrl.rs, crates/piney-fieldui/src/menus/mod.rs, crates/piney-fieldui/src/disp.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-game/src/world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/tests/flag_race.rs, crates/piney-gen/src/manifest.rs, crates/piney-data/src/tables/fieldui.rs, crates/piney-data/src/pack.rs, tools/test_fieldui_rs.py, tools/test_fieldui_talk_rs.py, docs/engine/field-ui.md, docs/formats/save.md, UNKNOWNS.md
---

# 394. Mutation's breeders offer the Flag Race: three grown Grunties in the town's pens and the race's mail read, then Talk, Flag Race and Rankings

Issue #56 (Mutation, build 4cce396): Dun Loireag's Grunty breeder lists
Talk, About Grunties, About Food and About Breeding although the three
Grunty kinds have been raised. On the PS2 the same breeder lists Talk,
Flag Race and Rankings.

## What the game does

Mutation's `BreederMenu` (MUT gcmn 0x005624c0) is Infection's with one
test added, at the list's build and again at OK:

- `ccMenuCtrl +0x12c` must be 3 or more. Mutation's constructor (MUT gcmn
  0x0053a394) clears it, and in a town (`game.area` 0) counts the pens
  `ccPgAdultCheck(game.server, k)` finds a grown Grunty in. Mutation put
  this short at +0x12c, so `protect[12]` and the rest move on by 2.
- `mailList[324]` (`saveData +0x23a8`, a signed byte) must be 3 or more:
  the mail that tells of the race, read. `grunty_mail` delivers it once a
  town holds a grown Grunty in each pen.

When both hold the rows are Talk and `breederMenuStr` pieces 3 and 4
(Flag Race, Rankings) and the list has 3 rows. Otherwise they are the
three "About" pages, 4 rows. OK on Flag Race goes to menu 88, on
Rankings to 89, with the list's `prev` 27, the window closed, `firstTime`
0 and `ccMsg->Close`.

From Mutation on there are 93 menu lists, not 89 (`InitMenuList`'s bound,
MUT gcmn 0x0053b0e0). `CheckMenuType` answers 92 while a menu changes,
where Infection answers 88. Menus 88 and 89 are the race's (`ccThMenu`'s
table at 0x00722210; their pages are `ExceptionDisp`'s cases 88 and 89).

**Rankings** (89, MUT gcmn 0x0058c930) drops the target and draws its
page until OK or cancel, then returns to the breeder. The page
(0x0058ca90) is a window of 24 by 12 with four columns of header. Each
rank shows its racer and Grunty, and its time as minutes, seconds and
hundredths (frames / 1800, % 1800 / 30, % 30 x 100 / 30). The time is in
the menu's `font`, which `Disp` makes the global `font` while it runs.
The ranks come from the save:

- `saveData +0x8432`: three ranks a town (12 bytes a server), each the
  player's (time, Grunty's `npcTbl` row).
- A rank with time 0 is held by the town's own racer, the next of a gcmn
  table (0x006d8610 by `server - 1`: Balmung, Gardenia, Cima in Dun
  Loireag).
- MUT main 0x0017a860 enters a race's time: at the first rank whose time
  it beats, it moves the lower ones down and returns that rank. Else it
  returns 4 within 30 frames of the third, or 0.
- `saveData +0x8462`, which the docs had as written but never read, holds
  the race's prize counts: a byte a rank, 3 a town (`+0x8462 + 3 (server -
  1) + rank - 1`; Flag Race reads it at MUT gcmn 0x0058b964).

## Cause

The port ran Infection's `BreederMenu` on every volume: no count, no mail
test, the "About" rows always. Menus 88 and 89 did not exist, and 88 was
the port's "changing" number on every volume.

## Fix

- **The count.** `MenuCtrl::pg_adult_num` (Mutation's +0x12c), counted by
  `count_pg_adults` when the menu task is made. The town passes its
  server to `FieldUi::menu_task_started`, fields and dungeons none.
- **The list.** `breeder_menu` builds the race's rows when `race_offered`
  holds and sets the list's rows from Mutation on. Its OK goes to 88 or 89.
- **Rankings.** `rankings_menu` and `rankings_menu_disp`.
- **The lists.** `MenuCtrl::changing()` is the volume's last list (88 or
  92), and `list_at` keeps a list index within the volume's lists. Every
  `clamp(0, 88)` became `list_at`.
- **The tables** (piney-gen, data version 26):
  - `elements` is 93 rows from Mutation on, and `breeder_str` 5 pieces.
  - `race_str` holds the race's texts, which follow `helpStr` in main.
  - `race_ranks` and `race_grunties` hold the towns' racers and the three
    Grunties offered with their stars. They are found by their two getters'
    code (`addiu $v1, $a0, -1`, `sll`, `addu`, `sll 3`, then the
    `lui/addiu`), one place on each later volume.

## Checked

- **Against the game's own code** (tools/test_fieldui_talk_rs.py, eemu,
  Mutation):
  - `test_breeder_race`: the list with three pens and mail 4 or 6, and
    without (two pens, mail 2 or 0).
  - `test_rankings`: Rankings in four towns, with none to three of the
    player's ranks, closed by OK or cancel.
  - The harness now reads the protect marks through `volume.menu_at` and
    compares the count at +0x12c too.
  - The whole talk harness (38) matches on Mutation and Infection. So does
    tools/test_fieldui_rs.py (74 cases).
- **Reproduced and fixed** (piney-game `flag_race`):
  - `mutations_breeder_offers_the_flag_race`: a new Mutation game in Dun
    Loireag with its pens and mail 324 set. Kite is set down before the
    breeder and speaks to him. The list must read "Talk Flag Race
    Rankings" with three pens and the mail read, else the "About" rows.
    Before the fix it failed on the first case.
  - `mutations_rankings_open_and_close`: Rankings opens with
    `exceptionDisp` 1 and closes back to the breeder.
- **Shots** (`breeder_race_shot`, ignored):
  - `/mnt/data/claude/scratch/i56/before/mut-breeder-race.png`: the
    "About" rows, the report's picture.
  - `/mnt/data/claude/scratch/i56/mut-breeder-race.png`: Talk, Flag Race,
    Rankings, as on the PS2.
  - `/mnt/data/claude/scratch/i56/mut-rankings.png`: Dun Loireag's three
    racers and their times.
- piney-game's suite (four threads), the tests of piney-fieldui,
  piney-gen and piney-data, `piney-gen gen --check`, clippy on the
  touched crates, fmt, `cairns check` and `tools/docs.py check` pass.

**Still unknown:** the race itself is not ported yet: the next entry's
work, all in the repo's reach.
- Flag Race (88, MUT gcmn 0x0058a4f0, page 0x0058c310) is still closed at
  once.
- The race's task and object (0x005ff680, 0x005fd120), its flags
  (gimmick 45, made by `ccSetChibiGuso` when the pens are full) and HUD,
  and the riding Grunty's Mutation changes in a town are not ported.
- Menus 90 and 91 (MUT gcmn 0x0058d470, 0x0058d9f0) are not ported, and
  who opens them is not traced: list 90 is Talk and Item List.
