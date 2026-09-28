---
number: 64
title: The field UI: ccThMenu's HUD, event windows and battle menus, frame by frame against the game
date: 2026-09-23
area: ui, test
files: crates/piney-fieldui, crates/piney-desktop/src/message.rs, crates/piney-desktop/src/save.rs, tools/test_fieldui_rs.py, docs/engine/field-ui.md
---

# 64. The field UI: ccThMenu's HUD, event windows and battle menus, frame by frame against the game

To play the opening arc, the field needs its user interface: the party
panels and target window, the event scripts' message windows, and the menus
a battle runs on. All of it is one task in `GCMN.PRG`'s `menu.cpp`,
`ccThMenu` (0x005280d0, priority 34), driving `ccMenuCtrl`. A helper agent
ported it as the new crate `piney-fieldui`; the design is in
`docs/engine/field-ui.md`.

## What it is

- **The task.** `FieldUi::step(pad, world, save, count)` is one pass
  between two `Breath` calls:
  - the menu asked for (`openReqNum`);
  - the change between menus (`menuNext`);
  - the open menu's handler from the jump table at 0x006e0300;
  - `Disp`.
- **The HUD** (`Disp`, 0x0051cfb0):
  - the party panels, with HP and SP bars in `ccGetNowMaxColor`'s colours;
  - the target cursor and window;
  - the enemies' life bars and off-screen arrows;
  - the protect-break marks and the battle banner;
  - the bracelet's gauge, the dim and the minimap's alpha;
  - the new-mail notice.

  Every sprite packet is sent in the game's order.
- **The event windows** (`ccMessage`): speech, information and tutorial
  windows, and `ccEvent::DispInfo`'s announcements. They are reached
  through the seam the event engine's field host will call:
  `message_open`, `message_check`, `message_close`, `announce`, `menu_ban`,
  `open_menu`, `target_forbid`, `map_on` and `noise`.
- **The menus ported:**
  - PERSONAL (0-2) and OPTION (12);
  - CHAT (3);
  - Skills (4), with revival and Data Drain's sub-target;
  - Items (5), including the special items;
  - TARGET (65);
  - the party tutorial's first two (75, 76).

  Any other number closes at once with `Request::Unported(n)`. The table
  of all 89 numbers is in the doc.
- **The world, read-only.** A `World` the runtime fills each frame: the
  game's area and battle state, the three party slots, `cmndTarget` and
  `cmndTargetPrev`, `cmndSortRoot`'s chain with screen points, the three
  `cmndLink` chains, and the player's flags.
- **Requests.** What the menus do to the rest of the game comes back as
  `Request`s, in order:
  - sounds and voices;
  - putting the other tasks to sleep and waking them, with the frozen
    layers;
  - target changes;
  - skill and item uses, and Data Drain's hold and SP;
  - chat orders and the chat line;
  - the minimap's alpha.

## Also changed

- **`piney-desktop`.**
  - `MsgWindow` gained the field's helpers.
  - `SaveState::fresh` now writes all eleven button assignments from
    +0x8404, as `ccSaveData::Init` does: action, personal menu, chat,
    option, map, ok, cancel, and the camera's four. Before, it wrote only
    ok and cancel.
- **`demo_probe.rs`.** Reformatted by `cargo fmt` after [[63]]'s fix.

## Checked

- **Against the game.** `tools/test_fieldui_rs.py` (20 tests) runs the
  game's `ccThMenu` from its start in eemu, with its leaf calls hooked, and
  the same scenario through `fieldui_probe`. Every frame it compares:
  - the menu's and message window's state;
  - every sprite packet;
  - the kanji rows;
  - sounds, voices and target changes;
  - skill and item requests and chat orders;
  - the save's item list.

  The scenarios cover:
  - the panels and the windows;
  - the fallen player;
  - five kinds of target;
  - the enemy bars, the banner, new mail and low HP;
  - every Skills page, revival and Data Drain, and the refusals;
  - Items, including the special items;
  - CHAT in a field, a dungeon, a town and alone;
  - ten random pad runs through the battle menus.

  All pass.
- **Nothing else broke.** In a clean worktree, the desktop's four suites,
  `test_demo_rs` and `test_toppage_rs` pass on top of the change.
- **The workspace.** Its tests pass; clippy, including `piney-fieldui`
  with `trace`, and fmt are clean.

**Still unknown:**
- **Not in the runtime yet.** Nothing calls `FieldUi` yet: Mac Anu shows no
  HUD until the runtime steps it and fills its `World`.
- **Not ported:**
  - the Chaos Gate's menus (28, 57-62, 78, 79), now in progress;
  - the talk, trade and shop menus;
  - the other tutorials;
  - Key Items, Status, Equipment, PARTY, Gate Out and Log Out;
  - the OPTION pages;
  - Data Drain's own menus;
  - the field objects.
- **`ccThGameCtrl` is not ported.** It picks `cmndTarget` and opens menus
  from the buttons.
- **Hooked out of the checks, not ported:**
  - damage numbers;
  - the chat balloon's drawing;
  - `ccNoiz`.
- **Drawing shortcuts.** Rotated sprites are drawn unrotated, and the drain
  gauge's Gouraud cells take their first corner's colour.
