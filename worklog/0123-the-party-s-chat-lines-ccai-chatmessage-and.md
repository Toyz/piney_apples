---
number: 123
title: "The party's chat lines: ccAI::ChatMessage* and ChatMessageSender"
date: 2026-09-25
area: battle, ui, world, test
files: crates/piney-battle/src/party_chat.rs, crates/piney-world/examples/evchar_probe.rs, crates/piney-battle/src/party_ai.rs, crates/piney-battle/src/ai_move.rs, crates/piney-battle/src/event.rs, crates/piney-battle/src/affect.rs, crates/piney-battle/src/tables.rs, crates/piney-battle/src/lib.rs, crates/piney-battle/examples/battle_probe/party_ai.rs, crates/piney-world/src/combat/chat.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/stage.rs, crates/piney-world/src/combat/town.rs, crates/piney-world/src/town_party.rs, crates/piney-world/src/party.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/ai.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/tests/chat.rs, crates/piney-game/src/session/event22.rs, tools/test_battle_chat_rs.py, docs/engine/battle.md, docs/engine/field-ui.md, docs/engine/field-game.md
resolves: 120
---

# 123. The party's chat lines: ccAI::ChatMessage* and ChatMessageSender

[[120]] drew the balloons, but the party still said nothing. The decisions
called `Call::Chat`, the runtime answered 0, and `ChatMessageSender` only
ran the arrival count down. These are `ccAI::ChatMessage*` (`personal.cpp`,
gcmn 0x00586130-0x00592a98): 57 functions over 77 tables of lines in
GCMN.PRG.

## How a line is said

- **Picked, not shown.** A function picks its line and writes it into the
  AI (+0x24, 80 bytes) through `ChatMessageModify`, then sets
  `chatRequest`. The member's next `ChatMessageSender` (the end of
  `Brains`) opens it with `ccChatMsg::OpenChat` and clears the request.
- **The template.** Most functions check `partyFlag` 1 (no draw
  otherwise) and read the row `MessageIndex()` (the character id; 18 for id
  1 while `saveData` +0x220d is set, id 1's lines garbled). The tables hold
  one line per character, or three, chosen by `(rand() >> 3) % 100` (below
  60, 90) or `% 3`. Nothing is said under manual control, but the draws are
  made first.
- **`ChatMessageModify`.** `#0` is the leader's name, `#1` a1 or the
  leader's, `#a` and `#b` a2 and a3 or " ". It stops at 79 bytes. Any
  other `#` code never advances, so the loop spins forever. Two lines have
  one: Orca's third `fatalDamagedMessages` ("Oh #@&%!") and Rachel's
  `charmConfusionMessages` ("Oh %&*#,"). The port stops the copy there.
- **The specific ones.** `AttackTarget` returns the kind it said (element
  4, crowd 5, condition skills 6, level 1-3). `Attack` and `Damage` keep
  their every-third counters, and `AttributeGuard` sets -3. `Damage`,
  `Neutral` and `ThanksHeal` file or withdraw the "heal me" request
  (0x10016, in `11 ((rand() >> 3) & 3)` frames). `ConditionModify` sends
  0x10012 again in 150. Rachel (id 12) compares weapon prices in
  `Neutral`, and Natsume (id 11) notes a member two levels up. Some lines
  turn the member (`atan2f(d.x, -d.y)` when `CheckAction(1)` passes).
  `Greeting` only points the AI.
- **`ChatMessageSender`.** Nothing once the party is wiped out. When
  `arrivalChatCnt` reaches 0 out of a fight, the arrival's line is said:
  `EnteredTown` (the server's `arrive*Messages`), or `EnteredField` from a
  town, or in a type-4 dungeon.

## In the port

- **`piney_battle::party_chat`.** It holds `ChatTexts` (the tables and
  `getAttributeStr`'s), `modify`, `Ctx::chat_line` for every function,
  `Ctx::chat_sender` and `Ctx::greeting`.
- **The stage's runtime.** It answers `Call::Chat` and
  `Call::ChatMessageSender` with them. The text goes out as
  `Show::Chat(body, text)`. The area opens it over the member (handle `1
  << 24 | id`), and the town does too (`take_party_chats`,
  `handle(Kind::Spc, id)`).
- **Lines the rules raise.** These are `ChatAttack`,
  `ChatAttributeCritical`, `ChatDamage`, `ChatResurrectPlz`,
  `AffectMessages` and `Greeting`, and they were events nobody read.
  `combat::chat` now runs them on the member's AI after the task that
  raised them: a member's frame, Kite's, the entry control's pass, the
  skills' pass, a trap, the boss. Those from menus and events run at the
  next frame's start. `ChatDamage` and `AffectMessages` now carry the HP
  before the hit, which is what `ccFellow::Influence` holds when it calls
  them.
- **`ChangeEquipReport`.** In a field or dungeon the equipment order says
  `EquipOK` or `EquipNOT` (`FieldWorld::equip_report`). The town's is not
  wired.
- **`levelOld`.** The AI constructor never sets it, and `_ccMalloc` does
  not clear. The port had it at 0, so Piros said "Level... UP!" 30 frames
  into Mac Anu. It now starts at the character's level.

## Checked

- **`tools/test_battle_chat_rs.py` (new).** It runs the 45 entry points in
  eemu against `battle_probe chat`, 120 random worlds each (500 each with
  `bulk 500`). The worlds vary names, enemy skill lists, +0x220d,
  `areaPrev`, the server, the kept text and manual control. The kept
  text, every AI's fields, the bus, the bodies' flags and headings, the
  balloon opened and the `rand()` count all match. Cases that pick a stray-`#` line hang the game's
  `ChatMessageModify` and are drawn again. One case caught a name taken
  from no character (`GratsLevelUp(NULL)`). The game reads it through
  address 0, which is zero in eemu, so `#1` gives the leader's name. The
  port now passes no name there.
- **`party_chat::tests::modify_fills_the_codes`.** The codes, the 79-byte
  stop, and the stop at a stray code.
- **`session::tests::chat::a_member_speaks_in_a_fight`.** In event 3's
  east portal fight, Orca says two of his fight lines over his head while
  `inBattle` is on. `member_chat_shot` (ignored) shows
  his balloon.
- **`session::event22::piros_says_his_arrival_line_in_mac_anu`.** Back in
  Mac Anu, Piros says his arrival line 151 frames in.
- The workspace's tests, clippy and fmt pass. So do the party AI, fellow,
  Kite, navi, battle, enemy motion, party and event-character suites.

A Grunty's line (`ccPGuso::main`) is not this machinery. It calls
`OpenChat` itself with its own texts, so it is left.

**Still unknown:**
- **`levelOld` in the game.** What the heap holds there when a member is
  built in a town. The game may announce a level change on arrival.
- **The stray `#` lines.** Whether they freeze a real console as they
  freeze eemu's run.
- **The order of raised lines.** The game says them inside the hit, and
  the port says them after the task. Their `rand()` draws fall later in
  the frame than the game's.
- **`saveData` +0x220d.** What sets it (id 1's garbled lines).
