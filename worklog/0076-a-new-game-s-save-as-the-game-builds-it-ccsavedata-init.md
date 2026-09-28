---
number: 76
title: A new game's save as the game builds it: ccSaveData::Init, NewGame and InitSpcParam, and the merchant's camera
date: 2026-09-23
area: save, world, test
files: crates/piney-data/src/save/init.rs, crates/piney-data/src/save.rs, crates/piney-desktop/src/save.rs, crates/piney-desktop/src/savesys.rs, crates/piney-fieldui/src/newgame.rs, crates/piney-world/src/camera.rs, crates/piney-world/src/evcam.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, tools/test_save_init_rs.py, tools/test_merchcam_rs.py, docs/formats/save.md, docs/engine/field-game.md
---

# 76. A new game's save as the game builds it: ccSaveData::Init, NewGame and InitSpcParam, and the merchant's camera

The user found the new game wrong:
- Kite had no skills;
- his stats read 0;
- the bag held 40 zero entries, so Buy refused ("you will exceed the item
  type limit of 40").

The talk agent traced the bag to `ccSaveData::Init`. `SaveState::fresh`
had claimed to be Init "as far as it goes", but it went little further
than the name, the buttons and the options. An agent has now ported the
boot's save completely.

## What the game does, now ported

- **The boot.** The constructor zeroes the record and runs `Init(0)`, then
  `ccThMother` runs `Init(1)` (main 0x001743d0). Init fills the lists with
  their empties:
  - item slots id -1, category -1;
  - skills -1;
  - the mail order, gate order and gate record lists, fountains and area
    bans.

  It also sets the buttons, the options, `ccSound`'s levels (256 and
  output 1), and the time-idol ranks' text, which is read from the disc.
  One loop is kept as the game has it: the `protectArea[5]` clear runs 32
  times, so it also zeroes `fountainRecord[0..27]`.
- **A new game.** `NewGame(1)` then `NewGame(0)`. The trade lists come
  from `NewGame`'s `InitTradeItem`, not Init.
- **Log in.** `ccSetupNewGame` runs `InitSpcParam` (main 0x00175d30): the
  starting equipment, items and skills, through the ported
  `ccChangeEquipment`, `AddItem` and `ccSetLevelParam`. Then it sets
  `newGameFlag` to 1 and runs `SetSpcBaseMsg`. Both Log in paths (the top
  page, and `--mode world` / `field:N` / `dungeon:N`) now do all three.
- **Kite's stats.** They were 0 because the port never ran `ccChar::
  CalcReal`, which the game runs every frame in `ccPlayer::Main` and
  `ccFellow::Main`. The town now runs it on the party each frame.

A new game now starts as the game's does:
- skills 6, 150 and 233 (Saber Dance, Repth, Vak Kruz);
- Amateur Blades and four pieces of armour;
- eight stacks of items;
- 63 HP and 13 SP, with his stats.

## Older saves

A slot the port wrote before this has zero lists and no menu buttons. The
title's Load now repairs it (`SaveData::repair_port_save`, in
`SaveSys::read_data`):
- **Buttons.** Each zero assignment gets Init's, when every assignment
  but ok and cancel is zero.
- **Lists.** A list Init fills gets Init's fill when it is entirely zero
  bytes. The game adds each entry once, so it never leaves one of those
  all zero.
- **Left alone.** Lists where zero is a real state are not repaired.

[[74]]'s `repair_buttons` on entering The World does the same for the
buttons and stays as a second guard.

## The merchant's camera

- **What the game does.** When a shop opens, `SetMerchantCamera` (gcmn
  0x005269d0) makes camera 3 (`ecam`) the one drawn. Its view is the
  merchant's position plus 100 up, and its eye is 500 along the
  merchant's facing, turned 7.5 degrees down. The breeders use fixed
  views by server. `changeCamera(1)` goes back to `tcam`, which kept
  following Kite meanwhile.
- **The port.** `Camera::set_merchant` and `World::set_merchant_camera` /
  `change_camera` implement it, and the runtime handles the shop's two
  camera requests.
- **Mac Anu's merchants.** All five take the general branch: the breeders
  are other towns' `npcTbl` rows 10, 16, 22 and 28.

## Harnesses that started from the wrong save

Many harnesses built their game-side starting save by hand: zeroed
memory with ok and cancel set, or a partial copy. So each compared the
port against a save the game never has:
- desktop, desktop menu, desktop data, desktop name;
- top page, demo;
- field UI and its five siblings;
- world and event VM.

They all now start from the save the game's own boot builds in eemu
(`boot_save()` / `fresh_save()` in `test_save_init_rs.py`). The event VM's
fixture was regenerated: its boot had run with `ccSnd` null, so the
volumes were 0.

## Checked

- **Init and the boot.** `tools/test_save_init_rs.py`, against the game in
  eemu:
  - `Init(0)` and `Init(1)` on zeroed, 0xff-filled and random records;
  - the boot sequence;
  - the repair: game saves and random saves are untouched, and an old port
    save comes back equal to the boot's;
  - end to end: the boot, `NewGame(1)`, `NewGame(0)`, `InitSpcParam`,
    `newGameFlag` and `SetSpcBaseMsg`, against `--mode world`'s save. All
    0x8530 bytes match, with and without Parody Mode.
- **The merchant's camera.** `tools/test_merchcam_rs.py`: Mac Anu's five
  merchants, the four breeder servers and 24 random characters, over 487
  frames, bit-exact. It compares the cameras, `world_view` and
  `world_screen`, `cameraGetRot`, and the switch back.
- **The checks catch breakage.** Each port was broken on purpose once, and
  each check caught it.
- **Re-run in a clean worktree** with fresh builds: those two, and every
  harness listed above, plus the workspace's tests, clippy, fmt and the
  docs check.
- **Shots.**
  - A new game's Items (8/40), Status (three skills, five pieces, 63/13,
    stats), Equipment and Skills.
  - The Recorder's and the Magic Shop's cameras.
  - Buy asking how many, then 1 of 40 bought for 900 GP.

**Still unknown:**
- **Not in fields and dungeons yet.** The per-frame `CalcReal` runs only in
  Mac Anu.
- **Not repaired in old saves:** Init's `townMoveFlag`,
  `partyMemberCall`, `tactics`, `cameraMode`, `drainDemo` and the idol
  ranks.
- **One frame late.** Camera 3 appears one frame later than in the game,
  because the menus run after the world's tasks.
- **Unknown uses.** What `partyMemberCall` 0x3fffe, `tactics` 7,
  `drainDemo` and `growth` are for in Infection.
