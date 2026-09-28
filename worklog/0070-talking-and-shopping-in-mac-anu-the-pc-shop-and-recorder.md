---
number: 70
title: Talking and shopping in Mac Anu: the PC, shop and Recorder menus, wired into the runtime
date: 2026-09-23
area: ui, world, test
files: crates/piney-fieldui/src/talk.rs, crates/piney-fieldui/src/menus/talk.rs, crates/piney-fieldui/src/menus/merchant.rs, crates/piney-fieldui/src/menus/shop.rs, crates/piney-fieldui/src/menus/record.rs, crates/piney-fieldui/src/menus/trade.rs, crates/piney-fieldui/src/menus/breeder.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/talk.rs, tools/test_fieldui_shop_rs.py, docs/engine/field-game.md
---

# 70. Talking and shopping in Mac Anu: the PC, shop and Recorder menus, wired into the runtime

The user could not talk to NPCs or use the shops. [[66]] made X on a
character ask for its menu, but the menus closed at once, unported. An
agent has now ported them.

## The menus

Each is checked frame by frame against the game's own `ccThMenu` in eemu:
- **Walking PCs.** 22 PcMenu and 47 TalkMenu: the PCs' lines, the trading
  PCs' offers, the merchants' chained lines and their voices.
- **The shops.** The lists of 24, 25 and 26 (Vender, Recorder,
  Fairyshop), with:
  - 52 Buy and 51 Sell, each with its Disp;
  - Elf's Haven's 54 Store and 55 Withdraw;
  - the Recorder's 53 Record, which saves through `piney_desktop`'s
    `ccSaveSys` and card code. The card files it writes are compared too.

Lines, stock, prices and texts are all read from the executable.

**Still to come from the agent:**
- Trade (48/49);
- 21 (a party member);
- 23 (an administrator);
- 27 and 56 (the Grunty breeder);
- 50 (a gift).

Until then, each closes at once.

## How the runtime drives them

- **Who is spoken to.** When `ccThGameCtrl` asks for a talk or shop menu,
  the runtime first tells the field UI who was spoken to
  (`FieldUi::talk_to`, the NPC's handle and `npcTbl` row). The pages read
  the character's base parameters from there, as the game's read
  `cmndTarget`.
- **`EntryAffect` from the menus.** 14 as a list opens, 15 as TalkMenu
  speaks, 0 as it closes. These arrive as `Request::Affect` and go to
  the NPC's `influence`. [[65]] had the world send 14 and 0 itself, for
  lack of menus; that stand-in is removed, so the menus are the only
  source, as in the game.
- **The target.** The shop pages drop `cmndTarget` and give it back
  (`ccChangeCmndTarget`), and set `cmndTargetFix`. These go to the new
  `World::change_command_target` and `World::set_target_fix`.
- **The Recorder's card.** The field UI saves to the session's card
  directory, starting at the (port, file) where the title's Load left
  `ccSaveSys`. `--mode world` uses the default card.

## Checked

- **Against the game.** `tools/test_fieldui_shop_rs.py`, 31 tests
  including random pad runs through the pages, and
  `tools/test_fieldui_rs.py`'s 33 still pass.
- **In the runtime.** In `--mode world --no-events`, walking up the bridge
  to Bell and pressing X opens her list (Talk, Trade) with her greeting,
  with the target frame on her.
- **In a clean worktree.** The workspace's tests, clippy (including
  piney-fieldui with `trace`), fmt and the docs check pass. So do the
  shop, field UI, world, characters' commands and data screen suites.

**Still unknown:**
- **Not ported.** The pages listed above.
- **The camera.** `SetMerchantCamera` (gcmn 0x005269d0) turns the camera
  on the merchant (`changeCamera(3)`), and `changeCamera(1)` turns it
  back. Both arrive as `Request::Talk` and are not carried out, so the
  camera stays where it was.
- **The generator.** PcMenu draws from its own copy of `rand()`, not the
  world's shared generator.
- **The Recorder's card position.** It follows the title's Load. A save
  made on the desktop's Data screen after that is not followed.
- **Not shot.** No runtime shot of a shop: I could not steer Kite to a
  booth by stick presses alone. The shop pages are covered by the checks
  above, and the Recorder's request by the world's disc test.
