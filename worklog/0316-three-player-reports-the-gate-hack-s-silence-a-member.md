---
number: 316
title: Three player reports: the gate hack's silence, a member revived while lying, the markers of the fallen
date: 2026-10-01
area: audio, battle, ui
files: crates/piney-audio/src/driver.rs, crates/piney-audio/src/lib.rs, crates/piney-game/src/world.rs, crates/piney-game/src/mode.rs, crates/piney-game/src/main.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/combat/stage.rs, crates/piney-game/src/session/tests/gate_hack.rs, crates/piney-game/src/session/tests/revive.rs, crates/piney-game/src/session.rs, BUGS.md
---

# 316. Three player reports: the gate hack's silence, a member revived while lying, the markers of the fallen

GitHub issues 13, 14 and 15 (Infection). Each was played in the session
before any change, and each new test fails without its fix. Addresses are
INF; gcmn where marked.

## Issue 13: the town's music under the gate hack

`GtHackMenu` (gcmn 0x005661e0) calls `ccSndGateHack` (main 0x00180780) at
+0x78 (0, as the menu opens), +0x754 (1, a cancel), +0x894 and +0xa58 (2).
`ccSndGateHack(0)` sets `ccSnd.gateHack` (+0x63) to 1 and writes the fades
of sequence 0 and, with `sqNum` 3, sequence 2 inline: switch 3, 20 frames,
from the port's volume to 0. Switch bit 1 stops the sequence at the end.
`ccSoundRpc` calls `ccSndGateHackCtrl` (0x00180910) each frame after
`ccFade`: states 1 and 3 run `ccSceneFade` too, so fade 0 steps twice a
frame (stopped at frame 11) and fade 2 once (frame 21). State 2 (from
`ccSndGateHack(1)`) is `ccSqPlayVol(0, 0)`, a switch-1 fade up to the table
volume, the same for sequence 2, then state 3. `ccSndGateHack(2)` sets 0.

The port turned the request into `ccSqFade` with switch 0, which the fades
skip, so nothing faded. Now `Driver::gate_hack` and `gate_hack_ctrl` carry
the state, and the menu's request goes to the audio as
`Event::GateHackSound`. Mutation, Outbreak and Quarantine run the same two
functions and call them from the same four places (OUT's `GtNewMenu`).

`gate_hack_silences_the_town_s_music` plays story 19's Mac Anu to the
slots and cancels. The session's events drive a headless engine. Before:
rms 4663 at the slots, sequence 0 playing. After: 0 at the slots, back
to 5332 after the cancel (511 on the way, the tune's quiet part). The
driver test checks the frames 11 and 21.

## Issue 14: a member revived while lying stayed down

Affect 20 in `ccFellow::Influence` (gcmn 0x0041c318) revives a member
with `dead` 2-4: act 9 or 10 becomes 2 (`SetActNum`), skill 0, `dead` 5,
`HitEnable`. The port's rule is right. The world keeps a second copy of
the member's act and flags for the AI. `drain_chats` runs after each
frame's lines (a fall's `ResurrectPlz`, a revive's thanks) and wrote that
copy back over the character. The copy still held the act from the
member's last frame. So a felled member never played act 9, and one
revived while lying (`dead` 3, act 10, 50 frames) got act 10 back. After
`dead` 5's 78 frames she was up with full HP, lying for good: the report's
picture. Neither confusion nor the killer matters. A revive as a ghost
(`dead` 4, act 2) put back act 2, which is why Elk got up. The copy is
now taken from the characters before the lines run.

The same reading found the world dropping the rest of `Influence`'s work
when an affect lands in another character's frame (an enemy's blow from
its note 0x8005, or Kite's on a member): `ccSkillRequest(ch, 0, 0)`,
`HitDisable` / `HitEnable`, the bus's "down" (0x1000c) and "up", the talk
ended. `ccFellow`'s own frame already did these. They now go, in order with
the lines, through `RuleParts` after the enemies' and Kite's passes. The
collision list loses the body at once, as `HitDisable` is called. The same
calls are in all four volumes' `Influence` and `ccFellow::Influence`.

`a_member_felled_and_revived_while_lying_gets_up`: Mia, confused, beside
a Mimic. The Mimic's spells confuse Kite, and his blow fells her. Then the
Resurrect while she lies, and again felled by Elk's blow. Before: act 10
with `dead` 0 after the revive (a fall by the world's affect also kept
act 2 instead of 9, seen in a probe). After: 9, 10, then up.

## Issue 15: the markers of the fallen

`Influence` (gcmn 0x0059afb8) and `ccFellow::Influence` call
`ClearConditionEffect` as one falls (`deleteConditionEffect`). One down
never shows its conditions again: `CalcReal(dead)` skips
`DispConditionEffect`. The world carried out the call only from its own
pass, so a fall in another character's frame kept the marker. Mia felled
by Kite's blow (as above) kept the "?" forever. For Kite felled by a
Mimic's blow, his own earlier `DispConditionEffect` was evaluated after the
frame and only faded the marker over about 40 frames. The game deletes it
at once, the frame he falls (the picture: Kite still standing at 0 HP).
Now the calls from all three kinds of frame are carried out in order. A
queued look at a party character felled since is left to the fall's
clear.

`the_fallen_keep_no_condition_marker`: Kite and Mia felled by poison,
Mia by Kite's blow, Kite by a Mimic's. Before: Mia's marker outlived the
fall.

**Still unknown:** whether the report's picture is the frame Kite fell
(the fade above) or another way to his marker; that needs the reporter's
`--pad-log` of that fight.
