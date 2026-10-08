---
number: 399
title: The Event NPC's Item List: menus 90 and 91 register what Kite holds, wears and keeps at Elf's Haven, and once all is in give the desktop items and send the NPC away
date: 2026-10-08
area: ui, volumes, build, test
files: crates/piney-fieldui/src/menus/registry.rs, crates/piney-fieldui/src/menus/mod.rs, crates/piney-fieldui/src/menus/talk.rs, crates/piney-fieldui/src/menus/flag_race.rs, crates/piney-fieldui/src/talk.rs, crates/piney-fieldui/src/disp.rs, crates/piney-data/src/save.rs, crates/piney-data/src/save/init.rs, crates/piney-data/src/pack.rs, crates/piney-data/src/tables/fieldui.rs, crates/piney-data/src/tables/race.rs, crates/piney-data/src/tables/registry.rs, crates/piney-data/src/tables/mod.rs, crates/piney-gen/src/registry.rs, crates/piney-gen/src/race.rs, crates/piney-gen/src/manifest.rs, crates/piney-gen/src/talk.rs, crates/piney-gen/src/lib.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/item_list.rs, crates/piney-game/src/session/tests/town_walk.rs, crates/piney-game/src/session/tests/ranch_music.rs, tools/test_fieldui_talk_rs.py, tools/test_fieldui_shop_rs.py, tools/test_fieldui_rs.py, tools/volume.py, docs/engine/field-ui.md, docs/engine/flag-race.md, docs/formats/save.md, UNKNOWNS.md
resolves: 394, 397
---

# 399. The Event NPC's Item List: menus 90 and 91 register what Kite holds, wears and keeps at Elf's Haven, and once all is in give the desktop items and send the NPC away

[[397]] made the Event NPC (`npcTbl` 175 + town, flags 0x10000000) stand
in the town while ITEM COMPLETE's status is set. Its action button opens
menu 90, which the port shut at once.

## What the game does

The steps are in [field-ui.md](../docs/engine/field-ui.md#the-event-npc-menu-90-and-item-list-91).

- **Menu 90** (MUT gcmn 0x0058d470) is `SpcMenu`'s shape. It shows the
  list (Talk 47, Item List 91) and, the first time, the line by server.
- **Item List** (91, 0x0058d9f0) works in three parts.
  - First it registers. It takes each category's items in the bag and at
    Elf's Haven (0x0058f930), then Kite's weapon and armour. Each listed
    item that is not yet registered goes into the save's registry.
  - It says how many came in from each place.
  - Once every listed item is in, the NPC speaks. Then come wallpaper 56,
    BGM 51 and movies 90-96 (sound 74 each). `eventStatus[52]` is cleared
    and the menu shuts, so the NPC goes.
- **The groups and pages.** Otherwise Item List shows the four groups
  (Weapons, Armors, Items, Key Items) with the count. Each group has a
  page per category. A page names every listed id: lit where registered,
  grey where not.
- **The registry** is the save extension's +0x754. [[246]] had found it
  written and never read. It holds 16 rows of 128 bits by category (MUT
  main 0x0017af60, 0x0017b0b0, 0x0017b140).
- **The categories.** Their sizes, listed counts and left-out ids are
  code. They are the same on Mutation, Outbreak and Quarantine.
- **Outbreak on** builds its messages with the number first ("05 items
  were registered to the"). It gives "There are no items" two lines;
  Mutation gives it one.

## Found on the way

- **Disp's kinds.** From Mutation on `Disp` draws the target cursor for
  kinds 0x1fbfdfff and names "Talk" for 0x1f001f1f (MUT gcmn 0x0053ce74,
  0x0053c92c). The port had Infection's masks, which lack 0x10000000, so
  it drew no cursor on the Event NPC.
- **TalkMenu's kinds.**
  - From Mutation on its by-server table covers 0x18001f00, which
    includes the Event NPC.
  - A breeder whose menu counted three grown Grunties, with mail 324 at 3
    or more, says the race's line (MUT gcmn 0x0056d2a8, record 0x00660b40).
  - The port had used Infection's 0x08001f00 for both cases.
- **The talk and base harnesses** read `menuList` clamped to 88. From
  Mutation on the last row is 92 (`volume.last_menu_list`).

## Fix

- **piney-data.**
  - `registry_row`, `SaveData::registered` and `register`.
  - `ext::ITEM_REGISTRY` (was `TAIL`).
- **piney-gen.**
  - New `registry` group, found through menu 91's code: the NPC's record
    (`done_va`), each group's page count and categories.
  - `fieldui`'s `item_list_str`, `item_list_tags` and `item_list_rows`,
    after the Flag Race's texts.
  - `race.talk_va`, found after TalkMenu's read of mail 324.
  - Both records are talk roots. `DATA_VERSION` is 30.
- **piney-fieldui.**
  - `menus::registry`: `event_npc_menu`, `item_list_menu` and
    `item_list_menu_disp`, the categories as a `Shelf` table, and the
    count.
  - `Disp`'s and `TalkMenu`'s kinds by volume.

## Checked

- **Against the game's code.** tools/test_fieldui_talk_rs.py has a new
  class, `EventNpcPages`:
  - the list by town, and Talk;
  - Item List registering from the bag, Elf's Haven, the worn pieces and
    key items, then its groups and pages scrolled;
  - nothing new;
  - the completion (the word, the three desktop items and seven movies,
    the status cleared);
  - complete already;
  - six random runs.

  It passes on Mutation, Outbreak and Quarantine. `test_breeder_race_talk`
  covers TalkMenu's race line (before the fix the message text differed at
  frame 35). The whole talk harness passes on all four volumes (48 tests).
  The shop and base harnesses fail as before only where UNKNOWNS.md lists
  them.
- **Played** (piney-game, Mutation, Dun Loireag), in `item_list`:
  - `the_event_npc_registers_the_bag`: Kite walks to the Event NPC
    (`town_walk::TownWalker`, also now the ranch test's), opens its list
    and Item List. The bag's items are registered, the weapon past the
    listed ids is not, and the count is out of 758. Cancel returns to 90.
  - `item_complete_gives_the_desktop_items_and_the_npc_goes`: with the
    registry full but one item in the bag, the desktop gets wallpaper 56,
    BGM 51 and movies 90-96, the status is cleared, and the NPC leaves the
    town.
- piney-game's suite (four threads), piney-fieldui's and piney-data's
  tests, `piney-gen gen --check`, clippy, fmt, `cairns check` and
  `tools/docs.py check` pass.

**Still unknown:** nothing.
