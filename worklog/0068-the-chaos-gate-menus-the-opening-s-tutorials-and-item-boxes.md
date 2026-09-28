---
number: 68
title: The Chaos Gate menus, the opening's tutorials and item boxes, and the gate a target again
date: 2026-09-23
area: ui, world, test
files: crates/piney-fieldui/src/menus/gate.rs, crates/piney-fieldui/src/menus/getitem.rs, crates/piney-fieldui/src/menus/party.rs, crates/piney-fieldui/src/words.rs, crates/piney-fieldui/src/menus/tutorial.rs, crates/piney-desktop/src/kanji.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/world.rs, tools/test_fieldui_rs.py, tools/test_desktop_rs.py, tools/test_desktop_menu_rs.py, tools/test_demo_rs.py, tools/test_toppage_rs.py, tools/test_morph_rs.py, tools/test_field_rs.py, tools/test_dungeon_rs.py, tools/test_desktop_data_rs.py, crates/piney-demo/examples/demo_probe.rs, docs/engine/field-ui.md
---

# 68. The Chaos Gate menus, the opening's tutorials and item boxes, and the gate a target again

The user could not use the Chaos Gate. Two things stood in the way:
- the gate's menus were not ported;
- the gate never became the command target, so the action button had
  nothing to open.

The field UI agent ported the menus. The target was a one-line bug in
[[65]]'s port.

## The menus

- **The Chaos Gate.**
  - Its pages:
    - 28 GateMenu (Random, New Keyword, Word List, Warp History, Other
      Servers);
    - 57 GtRandomMenu;
    - 58 GtNewMenu, with the keyword entry;
    - 59 GtListMenu;
    - 60 GtRecordMenu;
    - 61 GtTownMenu.
  - The area words are in words.rs.
  - **The warp.** A warp enters the words (`areaCodeSet`) and updates
    `gateRecord` and `areaCount`. It then takes the party out
    (`TransferOut` at frames 5, 35 and 65), and from frame 211 asks every
    frame for the area the words make (`SetGenerateCode`).
  - **Not ported.** 62, the gate hack, is a cinematic with its own stream
    and minigame.
- **The opening's tutorials.**
  - 77 PartyInMenuT (Party > Add, walked through);
  - 78/79 (the gate and New Keyword);
  - 80-83 (the skills, target and chat lessons);
  - 84/85 ItemBoxMenuT.
  - **A game bug, kept.** The game's jump table case for 84 has no break,
    so `ItemBoxMenuT` runs twice a frame under 84. The port does the same.
- **Items got.** 29 GetItemMenu and 30 ReplaceItemMenu cover the item got,
  99 already held, a full bag, exchange and give up. `AreaItem` draws a
  box's item from the area's lists.
- **Shared code.** menus/party.rs has `PartyInMenuDisp` and `LevelDisp`,
  for Party > Add (68) next.
- **New requests** (docs/engine/field-ui.md has the table):
  - `Affect` (`EntryAffect`: 11 opens the gate's circle, 0 closes it);
  - `Flash`;
  - `KeepLayers`;
  - `AreaCodeSet`;
  - `TransferOut`;
  - `DeleteNoPartyMember`;
  - `GoToArea`;
  - `ChangeArea`;
  - `AreaLevel`;
  - `AddMember`;
  - four orders to a member's AI (`ChatOrder`, `ManualOff`,
    `ManualModeAi`, `RemoteCmd`).

## "Log Out" lost its L

- **The bug.** `kanji::extract` dropped any glyph whose start came out
  below byte 1 of the texture. The first glyph of the texture's last row
  starts at byte 0, so PERSONAL's eighth row drew " og Out".
- **The fix.** The guard is now below 0. Only the nibble write at byte -1,
  the one that lands outside the texture, is skipped. `tools/font.py`
  has the same change.

## The gate as a target

- **The bug.** `World::game_ctrl` put the gate on the command list at its
  world position. Everyone else there is at `posP`, relative to Kite. The
  gate therefore sat about 6,300 units away and was never in range.
- **What the game does.** `ccChgate::main` (0x00459280) sets `posP`
  (+0x50) to `ccTransPosW2P` of `pos` (+0x40) every frame.
- **The fix.** The port now does the same.
- **What the user sees.** The gate becomes the target within a few steps.
  X opens menu 28, and the circle lights on the ground.

## In the runtime

piney-game's world mode takes the new requests:
- **`Affect`** goes to the gate or an NPC through the new
  `World::affect`.
- **`AddMember`** fills the next party slot. That member's panel then
  shows from its `spcParam` record and `charTbl` name.
- **`AreaLevel` and `AreaCodeSet`** are kept.
- **`GoToArea` and `ChangeArea`** are shown in the window title until the
  fields exist.
- **The rest are no-ops for now:** the flash, the party's warps, and the
  orders to its AI.

The UI's `World` now carries:
- the server;
- the area level;
- free story-area slots, (-1, -1). Zero would have read as area 0 barred.

## Seven harnesses ran old builds

- **What I saw.** The desktop suite failed the new every-row string in the
  clean worktree, but passed in the main tree with the same kanji.rs.
- **The cause.** `test_desktop_rs.py` builds its probe with cargo, which
  follows `CARGO_TARGET_DIR`. It then runs the binary from
  `ROOT/target/release/examples`, which does not. In a worktree whose
  build goes elsewhere, the harness ran whatever binary was left in
  `target/`: here, a build from that morning.
- **Where else.** The desktop menu, demo, top page, morph, field and
  dungeon harnesses had the same path.
- **The fix.** All seven now take the probe from `CARGO_TARGET_DIR` when
  it is set, as the world, field UI and battle harnesses already did.
- **What it means.** Earlier clean-worktree runs of those suites, such as
  the desktop and top-page runs behind [[64]], may have tested older code.
  They were re-run below with fresh builds.

Fresh builds turned up two failures that the old binaries had hidden:
- **The data screen's harness.** Its Python copy of `SaveState::fresh`
  still wrote only ok and cancel. Since [[64]], `fresh` writes all eleven
  button assignments. The game's `ccSaveData::Init` (0x001743d0) writes
  them too: 0x40, 0x10, 0x80, 0x800, 0x100, 0x40, 0x20, 8, 2, 4, 1 from
  +0x8404. The copy now writes them as well, so the saved cards match
  again.
- **The demo probe's lighting command.** It lit models through
  `convert::mmat` with an identity transform and read one vertex per
  triangle corner. Since the near-plane cut ([[61]]), a vertex at w 1 is
  cut away, so the probe got nothing and panicked. Lit models run VU1's
  lit program (whole triangles, no cut). The probe now asks for that, with
  positions shrunk to fit the GS space. The colours depend only on the
  normals and the lights.

## Checked

- **Against the game.** `tools/test_fieldui_rs.py` now has 33 tests, all
  matching the game's `ccThMenu` frame by frame. They cover every gate
  page, the warp's requests, each tutorial and the item menus.
- **The "L" fix.** `test_desktop_rs.py` and `test_font.py` include strings
  that fill every row.
- **In a clean worktree.** The workspace's tests, clippy (including
  piney-fieldui with `trace`), fmt and the docs check pass. With fresh
  builds, these suites pass against the game:
  - the field UI;
  - the desktop's four (desktop, menu, data, name);
  - demo;
  - top page;
  - morph;
  - field;
  - dungeon;
  - world;
  - font.
- **Shots from the runtime.** "Log Out" is drawn whole. The gate targeted,
  then the Chaos Gate menu over the lit circle.

**Still unknown:**
- **Not ported.** The gate hack (62).
- **Not in the runtime yet.**
  - `GoToArea` and `ChangeArea` go nowhere: no field runtime yet.
  - The flash, `TransferOut`, and the AI orders are dropped.
- **Party Add is only half real.** It shows a member's panel, but makes
  no character: that needs the party's AI and movement.
- **Not checked.** The gate-target fix has no eemu check of its own.
  `TalkAgainstGame` checks the targeting functions with posP given, not
  where each character's posP comes from.
