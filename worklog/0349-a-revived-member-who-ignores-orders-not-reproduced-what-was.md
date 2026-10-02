---
number: 349
title: "A revived member who ignores orders: not reproduced; what was ruled out"
date: 2026-10-02
area: battle, test
files: crates/piney-battle/src/party_ai.rs, crates/piney-battle/src/fellow.rs, crates/piney-battle/src/party_chat.rs
---

# 349. A revived member who ignores orders: not reproduced; what was ruled out

Two reports describe one bug. The user's run (INF, a dungeon's B3, Kite and
Elk, a fight with six or more Water Witches, `enemyTbl` row 37) and issue
#26: a member revived after dying several times ignores orders, both
strategies and direct ones. Per #26 it lasts only in the room of the fight.
In the user's video (`work/video_of_bugs/20261002-0701-22.4608026.mp4`) Elk
is out of battle at 47/190 HP with full SP. He stands still, and says
nothing to "Everyone, First Aid!" (16) or to "Elk, use La Repth!" (5 on
Kite). His SP regenerates, so he is alive: `condition_time_count` runs only
for `dead` 0. The run was not reproduced. This entry records what was
checked, so the next look starts past it.

Every way an order is refused makes the member speak. `ChatCommandFulfilCheck`
and the same-order branch of `ChatCommand` each say a line, and the accept
lines for 5 and 16 are templates with no chance roll. A silent member
means one of these:

- The order never reached `ChatCommand`. Brains calls it only for
  `party_flag` 1. `ccFellow::Main` calls Brains only with an AI and without
  `ghoFlag`. A new order waits at `chatCmdFlag` -2 while `goBackFlag` is set
  and `distPL` is over 1750.
- Its lines were dropped. `chat_say` drops them under `manualSW`,
  `chat_gate` needs `party_flag` 1, and `chat_sender` opens nothing while
  the party reads as wiped out.

`manualSW` fits the video best. Brains then runs `ManualControl`: the
member stands still, never acts, and its lines are dropped. Its only
writers are the registry's boot bit 2 at an area's set-up, events'
`ManualModeAI`, and the tutorial. No death or revival path reaches any of
them.

Also ruled out by reading:

- `dead` 5 stuck: the revival count runs in `fade` every frame, and the
  member's SP would not regenerate.
- A skill status or act left set: death and revival both clear
  `skillID`/`skillStatus`.
- `PAUSE`: no AI decision reads it.
- The message bus: entries are withdrawn only for `party_flag` -2.
  Down sends 0x1000c and up deletes 0x1000d. That is the game's own
  pairing, copied as found.
- `ocarinaUseFlag`: set only by order 19 and never cleared in the port,
  which is worth a check of its own (it blocks messages and targets).
- `spcBattleCondition`: 3 lasts one frame.
- The UI's handles map to the right scene index.

Simulated (probes kept out of the tree, the diff in the scratch area),
INF, story 19's field:

- Elk killed and revived nine times, at the down act, lying and as a
  ghost, then ordered: he answered and healed each time.
- Killed mid-cast, then revived and ordered.
- A 40000-frame fight against two Water Witches with 56 deaths. Every
  order was taken. One was not carried out, for want of SP.
- Seven Water Witches with Kite and Elk alone, random revive delays and
  random orders. All taken.
- A monitor over eight seeds of 12000 frames, about 150 deaths, waves of
  3-7 witches, random orders 0-18. It watched, while Elk was alive:
  `manualSW`, `party_flag` other than 1, an order parked at -2 over 120
  frames, hold over 600, `skillStatus` over 900, an act of 7 or more over
  900, no movement over 3000. Nothing fired.

**Still unknown:** the state that silences a revived member in the room of
its fight. A pad log of a run that shows it (the console's `pad_log`
writes the run since power-on) would replay it exactly.
