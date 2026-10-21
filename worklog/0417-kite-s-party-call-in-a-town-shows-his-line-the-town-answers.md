---
number: 417
title: Kite's party call in a town shows his line: the town answers the balloons' places for the party, so the order's own Disp keeps it (#68)
date: 2026-10-21
area: world, script
files: crates/piney-game/src/world.rs, crates/piney-game/src/session/tests/party_call.rs, crates/piney-game/src/session.rs, tools/test_chat_msg_rs.py, docs/engine/field-ui.md
started: 2026-10-21T10:54:17-06:00
took: 13m
---

# 417. Kite's party call in a town shows his line: the town answers the balloons' places for the party, so the order's own Disp keeps it (#68)

Issue #68 (build 3e87cd5, Mutation, "maybe earlier"): in a town, CHAT's
"Everyone gather!" and "Let's scatter!" brought the members' answers
but no balloon over Kite. The clock for this entry also covers the
first look at #70 (the next entry).

## What the game does

`ccMenuCtrl::ChatMenu` (MUT gcmn 0x00547450; INF 0x00528f20), the town
page's OK: `RequestChatCmd(member, townChatAct[row])` to each member
(0x00547a98), then `ccChatMsg::OpenChat(plw, chatMenuStr piece 16 +
row)` (0x00547ad8), then the menu's close: `ccWakeAllThread`,
`ccMenuCtrl::Disp` (0x00547b10), `ccBreathThread`. That `Disp` runs
`ccChatMsg::Disp(still)` in the same pass, so it asks `ccCheckTarget`
and `ccCalcTagPosChar` of Kite at once. No voice is played; the only
sound is the OK's `ccSeOn`. The field's pages do the same with
"Everyone, " and the order (0x00547ff4).

## The cause

The port's `ChatMsg::disp` takes each speaker's state from the runtime's
`World::chat_at`; a speaker missing from it counts as off the lists and
its balloon closes. The town's `menu_task` built that list from the
balloons up before the menu step, so the line the step opened over
Kite had no entry and closed in its own pass. The field's `hud_world`
already added the party's handles; the town's did not. The members'
answers come a frame later from their AI, which is why Rachel's showed.
Every volume: the town runtime is shared, and the reproduction fails on
all four.

## Fix

`WorldMode::menu_task` adds the party's handles to the speakers it
answers for, as the field does. docs/engine/field-ui.md says so and
gives the town lines.

## Check

- `session::tests::party_call::kite_calls_the_party_in_town`: a new game
  on each disc in Mac Anu with BlackRose, CHAT (square), row 0 then row
  1, OK. Kite's balloon reads "Everyone gather!" / "Let's scatter!" and
  is up 120 frames; BlackRose answers. Before the fix: no line (fails on
  Infection's first order).
- `kite_calls_the_party_in_a_field`: event 3's field, Skill Usage's
  first order, "Everyone, Skills!" up 120 frames.
- `test_party_call` in tools/test_chat_msg_rs.py runs the game's
  `ccChatMsg` in eemu: the player's line opened, `Disp(1)` in that pass
  (the menu is still), a member's answer three frames on. The port's
  slots, cells and texts match each frame on all four volumes, and the
  line is up 120 frames, as the session test counts.

Shots (`party_call_shots`, Mutation, 20 frames after the order): before
/mnt/data/claude/scratch/i68/before/gather-020.png (BlackRose's "I got
it Kite!" alone), after /mnt/data/claude/scratch/i68/after/gather-020.png
(Kite's red balloon "Everyone gather!" over him). piney-game's suite
(four threads), the chat harness on four volumes, clippy and fmt pass.

**Still unknown:** nothing
