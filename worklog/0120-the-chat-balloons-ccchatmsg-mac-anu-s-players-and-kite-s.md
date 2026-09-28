---
number: 120
title: "The chat balloons: ccChatMsg, Mac Anu's players and Kite's calls"
date: 2026-09-25
area: ui, world, test
files: crates/piney-fieldui/src/chat_msg.rs, crates/piney-fieldui/src/ctrl.rs, crates/piney-fieldui/src/disp.rs, crates/piney-fieldui/src/spr.rs, crates/piney-fieldui/src/render.rs, crates/piney-fieldui/src/world.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/menus/chat.rs, crates/piney-fieldui/src/menus/tutorial.rs, crates/piney-fieldui/examples/chat_probe.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/chat.rs, tools/test_chat_msg_rs.py, docs/engine/field-ui.md
---

# 120. The chat balloons: ccChatMsg, Mac Anu's players and Kite's calls

Nothing drew the chat balloons: not Mac Anu's walking players, not the
CHAT menu's orders, not Kite calling the strategy, not the party's lines
([[110]] left them open). These are `ccChatMsg`, which `ccMenuCtrl` owns.

## ccChatMsg (main 0x001a6160-0x001a6c68)

- **Four balloons.** Each has a text, its width and 120 frames. Opening one
  over a speaker who already has one closes the old one.
- **Disp.** Each balloon counts down (unless the menu holds the world
  still) and closes when its speaker leaves the command lists. It stands
  over the speaker at 0.9 of its height (`ccCalcTagPosChar`, mode 0).
- **Stacking.** `CheckScope` moves a balloon up above the first earlier
  one it overlaps, until it overlaps none.
- **The window.** `DrawWindow` builds it from seven 14 x 16 cells of the
  menu's icon sheet: four corners, two stretched edges, and the tail under
  its first third. The player's is in colour 18, the others' in 1. It
  fades over the last 10 frames, and the text is drawn at once.

`piney_fieldui::chat_msg::ChatMsg` is the port, in `MenuCtrl.chat`, drawn
in `Disp` between the dim and the damage numbers. The runtime answers
where the speakers are each frame (`World::chat_at`): `FieldWorld::chat_point`
in fields and dungeons, the town `World::chat_point` in towns.

## Who speaks now

- **The CHAT menu and the tutorial.** Their `OpenChat` over the player now
  opens the balloon in the same frame (`chat_msg::open_player`). The
  request is still raised.
- **Mac Anu's walking players.** Their `PcEvent::Chat` lines were made and
  never read. The town now takes them (`take_pc_events`) and opens each
  text from GCMN.PRG over its PC.
- **Kite's strategy call in a fight.** `Show::Shout` is now
  `ccSpcShoutOperationName`'s balloon: `chatActionStr` 5, `chatMenuStr`
  `tactics + 1` (as the game reads the save's byte, signed) and
  `chatActionStr` 6.

## Checked

- **`tools/test_chat_msg_rs.py` (new).** It builds the game's `ccChatMsg`
  by its constructor in eemu. Its outside calls are answered: the
  sprites, the kanji, the characters, and `ccCheckTarget` and
  `ccCalcTagPosChar` (GCMN's, hooked by address). `ccKanjiStrWidth` runs
  natively. Twelve runs of 150-400 frames: opens and closes, still
  frames, speakers leaving the lists and the screen, and crowded speakers
  so balloons stack. `chat_probe` runs the port alongside. Every window
  cell (cell, place, size, grid, colour, alpha), every text (place, colour,
  alpha) and every slot's frames, width and place match each frame.
- **`session::tests::chat::mac_anu_players_chat`.** From event 11's start
  in Mac Anu, standing, a walking player speaks at frame 2908 ("Feel a bit
  better!").
- **`mac_anu_chat_shots`** (ignored) shows the blue balloon with its tail
  and the line over the square.
- The workspace's tests, clippy and fmt pass.

**Still unknown:**
- **The party's own lines.** The `ccAI::ChatMessage*` functions (which
  line, drawn from `rand()`, `ChatMessageModify`'s `#` codes) and
  `ChatMessageSender`'s opening are not ported. The rules raise them as
  `Show::Chat` and nothing reads them.
- **A Grunty's line** (`ccPGuso::main`) is not ported.
- **The text's texture.** The port draws the balloon's text through the
  menus' `Draw::Text`, a 128-row texture (`Init(3, 24)`), where the game's
  chat kanji is `Init(2, 16)`. One line of text looks the same.
