---
number: 477
title: Four old reports from the code: the Status faces, the scroll's sparks, Wonder Battle's chatter run against the game, Elk raised again and again
date: 2026-10-25
area: ui, battle, test
files: tools/test_battle_fellow_rs.py, crates/piney-battle/examples/battle_probe/fellow.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/town_return.rs, crates/piney-game/src/session/tests/revive.rs, docs/engine/battle.md, BUGS.md
started: 2026-10-25T15:05:02-06:00
took: 55m
---

# 477. Four old reports from the code: the Status faces, the scroll's sparks, Wonder Battle's chatter run against the game, Elk raised again and again

GitHub issues 3, 5, 7 and 26 (Infection, all ThreePendant's), each left
as "not reproduced, needs a pad log". This pass went at them from the code:
every writer and ender of the state each report shows, in the game and in
the port. Addresses are INF gcmn unless marked. Shots looked at: #3's
Status page (Mistral, level 1, no face), #5's field after the fight (a
white glow and a white arc where the drained foe stood, "We did it Kite!"),
and #7's video, all 111 frames (ffmpeg): two balloons, the lower changing
every 1 to 3 frames between "Earth Element attacks are effective against
Wood..." and the dash and bull lines, the upper mostly "This one is Wood
Element.", foes Mad Grass.

## #3: the Status page's portrait

The game's writers of `menuFace[slot]` (`ccMenuCtrl` +0xbc, a `ccSprite`
whose `SetTex` takes a texture of `xwin_f00`), from a scan of every
`SetTex` whose sprite comes from +0xbc..+0xc8:
- the constructor, through `ccCheckMenuFaceNameParty` (0x0056a930), each
  scene, from `ccPartyManager.memberID`;
- `PartyInMenu` (0x0053b3b8) and `PartyInMenuT` (0x00567b4c) once
  `ccThPartyAdd`'s thread is done, at `CheckMemberID(face)`'s slot;
- `SetMenuFace` (0x00526940), called only by the event's `party_add`
  (case 77, main 0x001af2b4), and only when `ccEvent` +0xc is 4 or more;
- `PartyInMenuDisp` for slot 3, the Party menu's own picture.

Nothing clears a face: `DelMember` (0x0059cf60), `resignParty`,
`expulsionSpc`, death and the scene changes leave the sprite alone, and
`InitParty` (log in) leaves Kite alone in the party. `memberID` is never
compacted, so a member keeps its slot behind an empty one. `Disp` sends
`menuFace[i]` only for a slot whose `ccPartyManager` pointer is set
(0x00522130), and `StatusMenuDisp` (0x005362d0) counts pages over the same
pointers, so the page and the send agree. The port's `face_tex` is set at
the same points (`menu_task_started`, `set_spcs`, the menu's process 31,
`party_add`) and its file textures are read at start and kept; a stream's
overlay drops and rereads them as it comes and goes, nothing else evicts.

A check run on every frame of the whole piney-game suite (332 tests) for
"an occupied member slot whose face is not its member's" never fired. It
did fire for slot 0 in event 3's field: the bracelet's colours set
mid-scene leave Kite's row-18 face until the next scene, as the game's
constructor-only write does. `the_status_portraits_follow_the_slots` adds
the routes the old tests lacked: BlackRose felled in the field, Gate Out,
then BlackRose taken out (slot 1 empty, Mistral's page 1 on slot 2), a
field and back. Each page draws its member's face. No cause found.

## #5: the scroll's sparks after a drain

The Hanged Man (skill 157) through `ConditionModifySystem` (0x00576c50) and
`ccSkillModifyCondition` (0x005756f0) starts `effSkillStart(foe, 157, a,
1)`: a generator of row 178 plus the element (gLife 10), the exec ring 82,
the controller -13 and its circle 85, all ended by their own lives. The
lasting ones are the condition's: `_ccSkillModifyCondition` writes
`conditionNum` 1 itself, and `DispConditionEffect` (0x0056f950) makes
`ccConditionEffect` 1, two generators of row 198 with gLife -1 and
`effAbilityDown`'s controller -22. Only `killConditionEffect`/
`deleteConditionEffect` end the generators: from `DispConditionEffect` (a
fall, a changed condition, the effects switched off) and
`ClearConditionEffect` (0x00570180), whose callers are the drain's
`clearConditionEnemy` (0x004335d0), `interruptThink`, `selectTarget`,
`ccPuccigusoStart`, the two `Influence`s and `ConditionAdjustment`. The
game's own leak is a `setConditionEffect` over a pointer while
`conditionNum` is -1; only the constructor, `DispConditionEffect` and
`ClearConditionEffect` write -1, each with the pointer, so it does not
arise.

The port keeps two records, the battle's `cond_fx` and the effects'
`conditions`. A check that they are equal after every frame of the whole
suite never fired. The scroll on event 3's goblin, the drain, the drained
form felled, 3000 frames on: no generator on any character, no effect with
a target, nothing held; the same with the drain's movie, and with the
reporter-era lookup in the town's `effectTbl2` (before 3ce4532), where 12
and 22 never end but have no object and start no particles
(`a_scroll_paralysed_foe_drained_leaves_nothing_after_the_fight`). The
spell's drained foe never has the condition set again: the drain takes it
off the lists in the enemy's frame, before `ccThSkill`, and
`ConditionModifySystem` asks `ccCheckTarget` first. No cause found.

## #7: Wonder Battle against the game

Wonder Battle is strategy 0 (`ccSpcSetOperation` 0x005a1930 maps only
operations 8-10), so the `SpcListNum` fault that b120f67 fixed, which put
a member back to strategy 0 each frame (`ChatCommand` 0x00583d60), changed
nothing for this player. The display has no pacing either:
`ccChatMsg::OpenChat` (main 0x001a67c0) frees the speaker's balloon and
takes the first free one every call, which is why the lines jump between
the two balloons in the video.

`tools/test_battle_fellow_rs.py` gained `RunFieldEdge`: 300-600 frames of
`ccThAISystem` and `ccFellow::Main` natively, Orca, BlackRose or Mistral
(`spcAIParam` rows 2, 15, 16) under strategy 0 in battle, one to three foes
within 250 of the member's territory (800, 1500, 1800), each drifting at 0
to 9 a frame and placed each frame in both (the probe's `Run` reads their
places after Kite's). 200 cases, 88,800 frames: every frame equal, the AI's
chat calls included. The game's code says `AttackTarget` 132 times in
them, mostly 0 or 1 a fight, with bursts at a Wavemaster's edge: frames 0,
61, 65, 68, 72, 78 in one, 18 lines in one 509-frame fight of an earlier
40. The shortest gap is 3 frames. So the chatter is the game's loop, given
foes that move about the edge; whether the video's foe motion was the
game's is the enemy side's, harnessed on its own.

## #26: Elk after being raised

Closed already: the reporter confirmed it was #36's freeze (worklog 368).
What the AI keeps through a fall and a revival was gone over again:
`manualSW` (written only by the registry's boot bit, events and the
tutorial), `talkFlag`, the strategy and `strategyCMD`, `SpcListNum`,
`partyFlag`, `ocarinaUseFlag` (cleared only by `ccAISystem`'s constructor,
0x0058bef0, a new one each scene in both), the down and up messages
(0x1000c sent, 0x1000d withdrawn). `elk_raised_again_and_again_takes_orders`
fells Elk three times in one fight and raises him with the Resurrect while
down, lying and as a ghost: his AI's fields are as before each time, and
an ordered Repth on Kite (`RequestChatCmd(5)`) is cast.

The other steps of the clock: piney-game suite 334 passed; piney-battle's
tests, the fellow harness, clippy and fmt clean.

**Still unknown:**
- #3: what left the reporter's Mistral page without its face; no path in the game's code or the port's leaves an occupied member slot faceless
- #5: what left the reporter's sparks; no ender of the scroll's or the condition's effects is missing in the port
- #7: whether the foes of the reporter's fight moved about the members' territory as the port's did
