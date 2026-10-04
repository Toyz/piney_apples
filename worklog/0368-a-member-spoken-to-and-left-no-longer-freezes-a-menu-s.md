---
number: 368
title: A member spoken to and left no longer freezes: a menu's greeting runs inside its affect
date: 2026-10-03
area: battle, ui
files: crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/combat/town.rs, crates/piney-game/src/session/tests/revive.rs, docs/engine/battle.md
---

# 368. A member spoken to and left no longer freezes: a menu's greeting runs inside its affect

Three reports describe one freeze:
- **#36.** BlackRose, then Piros, stopped moving in a field (Cursed
  Despaired Paradise).
- **#36's follow-up.** In another field Gardenia stopped moving after many
  fights. She gave no more chat lines and ignored "First Aid".
- **#26.** Elk, after a revive, ignored every order.

The member keeps its HP, answers the action button, and does nothing
else.

## What does not cause it

The first look at #36 found a member in battle mode that stood off its foe:
Mistral, a caster with spells off, whose `spcAIParam` row keeps 1,800-1,900
back. That is the ported rule, not this freeze.

A watcher run with the whole-story autopilot and god (400,000 frames, every
fight) checked each party member every 30 frames. It flagged any member
still for 1,800 frames while Kite was over 800 away out of battle and
menus, or one with `talkFlag` set for 600 frames without a menu. It found
none. Fights alone do not freeze anyone; the autopilot never talks to a
member.

## Cause

Only one exit in `ccAI::Brains` (gcmn 0x0057ca00) fits all the symptoms.
With `talkFlag` set it turns the member to `gDeg` and returns. The bus
(`ReadSysMsg`), the orders (`ChatCommand`), the act and
`ChatMessageSender` are all skipped: no moves, no lines, no commands taken.

Speaking to a member opens `SpcMenu` (21). It sends `EntryAffect(member,
plw, 14)` (gcmn 0x00541a74); `TalkMenu` sends 15. Both are
`ccAI::Greeting`, which sets `talkFlag`. Cancel (0x00541d78) or a battle
(0x00541c04) sends 0, which clears it. In the game, `EntryAffect`
(0x0056b020) calls the character's influence inside the call (`jalr` at
0x0056b184), so the greeting and the talk-off land in order.

In the port the greeting came out of the rules as a chat-line event
(`chat::line_of`). `Combat::consequence` queued it in `chat_queue`, which
drains in the member's frame. The menu sleeps the tasks, so nothing
drained. The talk-off, a plain consequence (`Event::TalkOff`), applied at
once. When the menu shut and the tasks woke, the queued greeting set
`talkFlag` again, after its own talk-off, and nothing cleared it after
that.

A scripted test reproduced it in story area 14's field with Mia: open her
menu, cancel. Her `talkFlag` read false while the menu was open, and true
once it shut. Gift then cancel did the same.

## Fix

`Combat::affect_now` applies an `EntryAffect` from outside the frame and
then drains the chat queue, as the game runs the influence inside the
call. The field's `FieldWorld::entry_affect` (the menus' and scripts'
affects) and the town's `spc_affect` use it.

## Checked

- **The test.** `a_member_spoken_to_and_left_follows_again` (piney-game)
  opens Mia's menu (`talkFlag` set), cancels (the menu shuts, `talkFlag`
  clear), then walks Kite away for 240 frames: she follows over 300 units.
  With the old path it fails at once: `talkFlag` false while the menu is
  open.
- **Suites.** piney-world (96), piney-game (225), and
  `tools/test_world_rs.py`, `test_battle_chat_rs.py`,
  `test_battle_fellow_rs.py` and `test_battle_party_ai_rs.py` pass.

**Still unknown:**
- Whether #26's Elk froze this way, by being spoken to, is the reporter's
  to confirm. A revive alone did not freeze anyone in the runs above.
