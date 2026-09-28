---
number: 225
title: Mutation's town maps, its desktop, and the survey past the desktop
date: 2026-09-27
area: volumes
files: crates/piney-world/src/map/town.rs, crates/piney-world/examples/map_probe.rs, tools/test_map_rs.py, crates/piney-gen/src/syms.rs, crates/piney-gen/src/manifest.rs, crates/piney-desktop/src/acces.rs, crates/piney-desktop/src/audio.rs, crates/piney-desktop/src/dtmenu.rs, crates/piney-desktop/src/mail.rs, crates/piney-game/src/session.rs, tools/test_desktop_rs.py, tools/test_save_init_rs.py, tools/volume.py
---

# 225. Mutation's town maps, its desktop, and the survey past the desktop

The Mutation story survey stopped with "the town's map: not found: town
2's map" in Carmina Gade, and most of its starts sat idle at the
desktop. This entry ports Mutation's town maps and fixes the desktop's
differences that the harnesses showed. The survey's player now drives
the desktop and the top page, and every Mutation start reaches The World.

## The town maps

Mutation changed `ROOTTOWN::DrawMap` in three ways:

- **The class layout.** The map part of the class sits 4 bytes further
  on, from mapw at +0x80 to steph at +0x8c.
- **Mac Anu.** Its map is Infection's.
- **Dun Loireag and Carmina Gade.**
  - Each constructor makes two layers of its own. The map goes on a layer
    of priority 40 with mapLayer's frame. A fourth mask goes on a layer of
    priority 45 over the whole screen.
  - The arrow moves to mapLayer.
  - Carmina Gade's map (`town03(d)::TEX_sr3map1`, `RT03ICONPOS`, steps
    1.02 and 1.013) is offset by (16, 46) before the scroll.
  - Its sign 3 draws its balloon mirrored and flipped (ctrl 0x60). Left of
    the middle, that sign's label is drawn 6 lower.
  - While the flag race runs in the town, the fourth mask shows the three
    racers. The race's check is 0x005ff790: the area is 0 and the race's
    state at 0x0038bd44 is set. The racers are at 0x00774a90, each with
    its fade at +0x1ec. No sign draws while the race runs, but each sign
    still fades in or out.

The port takes the race from its input. The world passes none, because
the flag race itself is not ported. The map harness gives racers of its
own, and it runs Mac Anu, Dun Loireag, Carmina Gade and the race against
the game. It finds the race's globals from the code (the first unnamed
callee of Dun Loireag's `DrawMap`, the state it loads, and the table
built after the call).

The field map's harness ran `WORLD::Generate`'s painting loop at
Infection's addresses, carried by their offset in the function. Mutation's
`Generate` has 0x28 more bytes before the loop. The harness now finds the
loop by its code.

## Two sidecar passes (piney-gen syms)

- **code.** The data pass had put Mutation's `MailTbl` 194 rows below the
  table the mailer reads, inside code. The new pass pairs the address
  builds of paired functions whose bodies build as many addresses. A
  global the data pass placed moves when every such vote agrees and there
  are at least two votes. It moves one global on Mutation (`MailTbl`) and
  one on Quarantine. The port's mail table was already right, because
  the generator finds it by the code.
- **ctor.** The overlays' static initialisers are now named from their
  constructor lists, which are paired by position with Infection's. No
  other pass named them, because their bodies are runs of copies. This
  names 21 on Mutation and 18 on each of the other two, among them
  `__sinit_NameEntry.cpp`. The name entry harness runs that initialiser.

## The desktop

- **Wallpapers.** `AddWallList`'s unlock bits count only the rows that are
  not originals. Mutation has rows after the originals (49-51), and its
  rows 52 on take bits 49 on. The port had used the row as the bit.
- **Music.** From Mutation on, `AddWaveList` walks `Wave` to its "NULL"
  row and skips the original (50). It lists each row by its bit, below a
  global that equals the table's rows (52 on each later disc). Mutation's
  row 51, "BGM 51", has category -1, so the generator now ends `Wave` at
  its "NULL" row rather than at category -1.
- **The Controller page.** Mutation moved one camera label from x 369 to
  375. The matched functions hid this, because the exact pass masks
  immediates. The field's own `ControllerMenuDisp` keeps 369.
- **The harnesses.**
  - `MailList_control` holds 30 photos on Mutation (27 on Infection), so
    the fields from `Tempstream` (+0x7c) on are 12 bytes further on
    (`volume.mail_at`).
  - The save-init harness allocated at its record's own address. Mutation's
    `ccSaveData` constructor allocates the extension there, which wrote
    over the boot save that every desktop, board and field UI harness
    starts from.
  - The boot test compares the extension too.
  - Infection's `Init` cases are left to `InitLaterVolumes` on the later
    discs.

On Mutation the desktop, menu, name and save-init harnesses now pass, and
they still pass on Infection.

## The survey

The story player now drives the desktop and the top page:

- **Mail.** It reads each unread mail, opened from the list and then
  closed. On a mail with replies it takes the first reply and moves up to
  YES to send it.
- **News.** It reads each unread headline.
- **Log in.** It then goes to The World and logs in.
- **Event blocks.** While an event block plays, it presses OK for the
  block's windows.

Event 102's block 3 had been waiting on its window, because the player
was pushing at the locked ring. Every start from 102 to 114 now reaches
Dun Loireag. 115 opens in Carmina Gade with its map, and 116 plays the
ending into the staff roll. `PINEY_SURVEY_ONLY=N` runs one start, and
`PINEY_SURVEY_TRACE` prints the desktop's state.

**Still unknown:** The flag race (its menus 88-91, the PG_FLAG gimmick
and the racers) is not ported. The survey's player goes no further than
the town, because it does not yet take the gate to a story area. The
Data screen's and save menus' harnesses still fail on Mutation: its
`ccSaveSys::MainProccess` grew from 9068 to 13164 bytes, and the port's
card knows only Infection's directory and slot size.
