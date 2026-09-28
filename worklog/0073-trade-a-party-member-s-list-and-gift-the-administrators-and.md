---
number: 73
title: Trade, a party member's list and Gift, the administrators and the Grunty breeders
date: 2026-09-23
area: ui, test
files: crates/piney-fieldui/src/menus/trade.rs, crates/piney-fieldui/src/menus/talk.rs, crates/piney-fieldui/src/menus/breeder.rs, crates/piney-fieldui/src/talk.rs, crates/piney-fieldui/src/world.rs, crates/piney-game/src/world.rs, tools/test_fieldui_trade_rs.py, tools/test_fieldui_talk_rs.py, tools/test_fieldui_shop_rs.py, docs/engine/field-ui.md
---

# 73. Trade, a party member's list and Gift, the administrators and the Grunty breeders

[[70]] left part of the talk menus to come. The talk agent's two helpers
have now finished them. All 17 menus that the action button opens on a
character are now ported, each checked frame by frame against the game's
`ccThMenu`.

## The menus

- **Trade.** 48 TradeMenu and 49 TradeSubMenu: a PC's offers, the gauge,
  Approve, and the piece given in exchange.
- **A party member.** 21 SpcMenu and 50 PresentMenu: Gift, with the
  member's thanks and `AddSpcItem`.
- **The administrators.** 23 NpcMenu.
- **The Grunty breeders.** 27 BreederMenu and 56 BreedingMenu: Give Food,
  with the Grunty's growth.

## Merging

The shop agent merged the two helpers' work with each other and with
[[71]]'s:
- **One copy of each shared piece.**
  - `AddSpcItem`, which Trade and Gift share;
  - `ChangeEquipment` and the skill bookkeeping, from Equipment's
    equip.rs;
  - the menu fade, from PERSONAL's.
- **Members' stats written as Equipment writes them.** A member's trade or
  gift writes its `real` and `tune` to the save through `equip::calc_real`.
  It also raises the existing `CalcReal` and `ChangeEquip` requests.

## In the runtime

The action button's menus on a party member (21), an administrator (23)
and a breeder (27) now tell the field UI who was spoken to first, as the
PC and shop menus already did. Without that, the pages found no
character.

## Checked

- **Against the game.**
  - `tools/test_fieldui_trade_rs.py`: 10 tests.
  - `tools/test_fieldui_talk_rs.py`: 25 tests.
  - The shop, field UI, PERSONAL and OPTION suites still pass.
  - Members in the trade and gift tests now have levels and worn pieces,
    so their `real` and `tune` are compared too.
- **Re-run in a clean worktree.** All of these pass, with the workspace's
  tests, clippy (including piney-fieldui with `trace`), fmt and the docs
  check.
- **Shots from the runtime** (the talk agent's): Crest's list, and Trade
  with its offers, the gauge and Approve, then the give page.

**Still unknown:**
- **Not wired into the runtime.** These requests are dropped:
  - a member's book use (`SpcUseItem`);
  - `PresentOther`;
  - feeding a Grunty (`Feed`, `GruntyGrowth`);
  - `CalcReal` and `ChangeEquip`.

  The runtime also does not fill `World::grunty` or call
  `set_spc_base_msg`.
- **Not reached in Mac Anu yet.** Nothing gets Kite to 21, 23, 27, 50 or
  56 there: no member walks the town before the events. They are
  checked only against the game.
- **Only as the game does.** PresentMenu's states 20-22 and BreedingMenu's
  23 with index 0 are never set by any code found. A trader of neither
  kind reads a stale register in the game; the port offers nothing.
- **The empty worn piece.** piney-battle's `CalcReal` skips a worn piece
  of -1, where the game reads the row before its table. No new-game
  member wears -1.
- **Not modelled.**
  - The card calls finish at once, so the Recorder's "Saving" frames do
    not show.
  - `set_rand` is not wired.
