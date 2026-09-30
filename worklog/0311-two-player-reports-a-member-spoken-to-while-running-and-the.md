---
number: 311
title: Two player reports: a member spoken to while running, and the party's battle chatter
date: 2026-09-30
area: battle, world
files: crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/town.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/party.rs, crates/piney-world/src/town_party.rs, crates/piney-world/examples/evchar_probe.rs, crates/piney-battle/src/party_ai.rs, crates/piney-game/src/session/tests/town_return.rs, crates/piney-game/src/session/tests/chat.rs, BUGS.md
---

# 311. Two player reports: a member spoken to while running, and the party's battle chatter

GitHub issues 2 and 7, both from Infection. Addresses are INF gcmn unless
marked otherwise.

## Issue 2: a member spoken to while running

The screenshot shows Mistral's menu (`SpcMenu`, 21) open in Mac Anu with
her greeting in the message window, and Mistral nowhere in view. That is
the bug worklog 277 fixed (6edf655). `ccAI::Greeting` (0x00583190) clears
`moveFlag` and `runFlag` in the members' records. Before 6edf655 nothing
wrote them to her character before her next frame read it back, so she
ran on.

Worklog 277's test sent `EntryAffect(14)` to Piros directly. The new test
goes through the menu, as the report shows it:
- a new game in Mac Anu, with BlackRose and Mistral called by the Party
  menu;
- Mistral running to a shop (`actType` 8);
- Kite walking up to her and pressing the action button.

`SpcMenu` sends `EntryAffect` 14 on its first pass (0x00541a74), where the
port sends it too. Mistral stops where the menu found her. Within 20
frames `Brains`' talkFlag turn (`ccSetDirc(dirc, DEG2RAD(gDeg), 64)`,
0x0057cad4) faces her to Kite, and she stays stopped through Talk (47,
`EntryAffect` 15). With 6edf655's write taken out, she runs 23 units on
the first frame, as in the screenshot. So the report's build predates
6edf655, and nothing else was found wrong
(`mistral_spoken_to_while_running_stands_facing_kite`). The same path
runs in all volumes.

## Issue 7: the party's battle chatter

The video (3.7 s at 30 fps) shows BlackRose's and Mistral's balloons
changing every frame or two. The lines are `ChatMessageAttackTarget`'s
(0x0058d7b0): the element line (`AttributeFollow`) and the dash, bull and
timid lines.

**The game's pacing.** There is no chat timer:
- a line goes into the AI and opens at the member's next
  `ChatMessageSender` (0x00586420, the end of `Brains`);
- `ccChatMsg::OpenChat` (main 0x001a67c0) replaces the speaker's balloon;
- `ChatMessage` (0x00586130) checks only manual control.

The pacing lies with the callers. `AttackTarget` is said each time
`Reconnoiter` (0x00592aa0) takes a member into fighting (mode 3).
`Attack` and `Damage` speak every third hit (`atkMsgCnt`, `dmgMsgCnt`).
`WalkingTalk` and `Neutral` come on the bus's 0x10014 and 0x10015, sent
again in 300 or 500 frames plus 11 x `(rand() >> 3) % 3`. The bus ticks
once a frame (`ccThAISystem`, 0x0058c850). The chat harness passes, and so
do the party AI and fellow harnesses: `Brains`, `ActInField`,
`ActInTown`, `Reconnoiter`, `FollowTarget`, and `ccFellow::Main` over
60-300 frames. The town's, the field's idle and the battle's lines all
come from harnessed code; only the runtime's wiring is outside it.

**The reproduction.** A random Δ field (`random_areas(0, ..)`'s second)
with BlackRose and Mistral, Kite walking to the portals and attacking.
Mistral says bursts of `AttackTarget` lines 3-4 frames apart. She is a
Wavemaster: spcAIParam rows 9, 10, 16 and 17 have territory 1800 and
stopRange 1800. The loop:
1. Fighting, `FollowTarget` (0x00582090) halts her at 1800 from her foe.
2. The foe drifts away; `Reconnoiter` finds it beyond her territory and
   makes her wary (2). `FollowTarget` then runs her a step in.
3. The next `Reconnoiter` makes her fight again, and she says a line.

That loop is the game's code, and nothing in it differs.

**What did differ: the party's strategy.**
- `SpcListNum` (ccSpcChar +0xe8) is the member's registry slot.
  `ccSPC::Reboot` (0x005a00d0) makes each member with it through
  `ccSpcStart[id]` (`ccFellowNN(n)`, 0x0041ecc0). `ChatCommand`
  (0x00583d60) puts a character of slot 0 (Kite) back to strategy 0. The
  port never set the slot, so every member went back to strategy 0 each
  frame: the Operation and the CHAT menu's strategy orders held for one
  frame at most. `Combat::add_member` and `TownParty::build` now take the
  slot.
- `partyStrategy` (main 0x00378ce0) is a global. `ccSpcSetOperation`
  (0x005a1930) sets it each frame of `ccThSpc`, and `ccAI::ccAI`
  (0x0057c5f0) starts an AI with it. The port kept it per scene, from 0,
  so a new area's members fought as Wonder Battle until a fight ended
  (`ChatCommand`'s `ChangeStrategyCMD(partyStrategy)`). `Spcs` now carries
  it from scene to scene: a scene's members get the one before's, and 0
  after power-on.

In 20000 frames of the same field, Union Battle (operation 8) now gives 6
battle lines in 3 fights, against 71 in 10 under Wonder Battle. The
members go for Kite's target instead of the nearest foes. Mutation and
Outbreak have the same check, constructor and global (MUT `ChatCommand`
0x005aa334, OUT 0x005a6f84; `partyStrategy` MUT 0x0038bc68, OUT
0x00387020), so there is no gate. `a_member_keeps_the_strategy_it_is_told`
fails without the slot; `the_operation_holds_from_a_field_s_start` fails
without the carry. `evchar_probe` passes the slot too, and
`test_evchar_rs.py` still passes.

**Still unknown:** whether the reporter's party was under Wonder Battle. If
it was, the video's chatter may be the game's own loop at a Wavemaster's
1800 edge, unless the game's foes drift less than the port's. No capture
of the game was at hand to compare. Why BlackRose (a Heavy Blade,
territory 1500, stopRange 100) chattered too was not reproduced. In one
run a walking PC (body id 268, kind 2) pushed Mistral about 50 units during
her talk, after which she no longer faced Kite; that was not looked into.
