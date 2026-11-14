---
number: 596
title: ccThEntryCtrl on Outbreak and Quarantine: thread ends as function starts in the symbol carry, deleteAllObject by its callees, the later passes' calls, and the battle spawn harness on all four volumes
date: 2026-11-14
area: tooling, volumes, battle, test
files: crates/piney-gen/src/xfer.rs, crates/piney-gen/src/syms.rs, crates/piney-gen/src/carry.rs, crates/piney-battle/src/entry.rs, tools/volume.py, tools/test_battle_spawn_rs.py, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/side_events.rs, docs/disc/volumes.md, docs/engine/battle.md
resolves: 595
started: 2026-11-14T12:00:32-07:00
took: 3h 3m
---

# 596. ccThEntryCtrl on Outbreak and Quarantine: thread ends as function starts in the symbol carry, deleteAllObject by its callees, the later passes' calls, and the battle spawn harness on all four volumes

Outbreak's and Quarantine's `ccThEntryCtrl` is GCMN.PRG:0x00442580 (OUT)
and 0x00334e90 (QUA), the task `ccSetupGameCtrl` starts at priority 64
(OUT main 0x00168900, QUA 0x00168790: `lui/addiu $a0`, `li $a1, 64`,
`jal ccStartThread`, as INF main 0x001693f0). The symbol carry missed it
because of how it finds function starts, not because the function moved.

## Why the carry missed it

Outbreak moved `ccThEntryCtrl`'s frame walk out into a function of its own
(OUT gcmn 0x004416d0, QUA 0x00333fe0), which calls one function per list
(OUT 0x00441c30, 0x00441f20, 0x004421b0, 0x00442440; QUA 0x00334540,
0x00334830, 0x00334ac0, 0x00334d50), each the same walk as Infection's
inline loop (head from +0x14/+0x20/+0x2c/+0x38, `objFlag` bit 0,
`ccEntryObj::routine`, vtable +8, the list's delete). What is left of
`ccThEntryCtrl` is the set-up and a loop of `Breath(1)` and that call, so
it is 464 bytes, not 916, and its loop head is OUT 0x00442728, QUA
0x00335038 (INF 0x00431b08, MUT 0x00447498).

`piney-gen syms` pairs functions at "likely starts": `jal` targets, the
word after a `jr $ra`, after a padded tail `j`. A thread ends in an
endless loop with no `jr $ra`, and the function after it here,
`ccThEntryCtrlDelete` (OUT 0x00442750), is only ever stored as an address
(the task's delete hook at +0x40). So the two read as one 1212-byte
function, and the order pass put `ccThEntryCtrlDelete` on its start,
leaving `ccThEntryCtrl` unnamed. Mutation never hit it: its bodies are
Infection's, so `exact` paired `ccThEntryCtrl` and the address it stores
named the delete hook.

The fix adds thread ends to the likely starts (`Side::thread_ends`):
after a `b` back to a loop's head and its delay slot, the next word, when
it opens a stack frame (`addiu $sp, $sp, -n`), no branch reaches it and its
code up to the next start does not branch back above it. A first try
without the frame test, or requiring padding after the `b`, found dozens
of false starts on Infection (switch cases after a `continue`, the fellows'
threads); with it, run on Infection's own code, the rule finds 22 starts
the other rules miss, and every one is a named function's start.

## What else the same cause hid

On both OUT and QUA: `ccThEntryCtrl`; `ccThPlayer` (ccPlayerStart had its
place); `FileReadTh` and `UngzipTh` split from `ccFileListLoad`;
`Main__12Data_Control` split from `ccThDemo`; `ccGzipThread`;
`Func_str8000` and `Func_str8800` (OUT main 0x001891f0, 0x001897a0: the
same calls as Infection's, their set-up and tear-down moved into helpers);
`Func_str0001` moved from a merged span to its own start (OUT 0x0018b350,
QUA 0x0018b400, opening with `OBJ_se1_6flo1` as Infection's); and the
sizes of `ccThDrainEnemy`, `ccThSkill`, `ccThAISystem`, `ccThControl`,
`ccThCamera`, `ccThParticle`, `ccThEffect`, `ccThEffectStr`, `ReadThread`
and `ccLoadStreamOnMemReadTh`, each of which had run over its delete hook.
On Mutation the starts turned fourteen `prefix` and `call` names into `exact` ones
and added `Func_str8800`, `Func_str7100` and `Func_str0300`, with their
literal tables.

## Two more misnamings on the way

`deleteAllObject` sat on the new walk function (OUT 0x004416d0), which
has its shape: four calls. `ccEvent::Execute` calls the real one (OUT
0x00441720, QUA 0x00334030; the `jal` at OUT main 0x001b5920). A new
`callees` pass moves an `order` name to the one unnamed start between its
named neighbours whose calls name Infection's callees in order, where its
own place's do not: that one function on each, nothing on Mutation. The
walk had been hooked as `deleteAllObject` by any harness naming it.

The spawn harness then needed `ccDestEnemyV`, which no pass named on OUT
or QUA: `ccEnemyV::ccEnemyV` is a `ref` name, and the transfer's rounds
propagate only from what they paired. A `late_calls` pass votes calls and
taken addresses from every named body after the later passes, and pairs
in order the code addresses a body builds where both build as many (OUT
builds `ccDestEnemyV`, 0x004625e0, with other registers at another
place). It names 17 on each of OUT and QUA, all checked against the
store at +0x1c8 in each race's constructor or the call: the destructors of
races H, I, K, L, P, S, T, U, V, W, Z, `delLight__8ccEnemyL`,
`ccThFellow10Delete`, `ccThKyvia03Delete`, `ccBossKyvia04::SwitchGateOFF`
and `EntrySlave`, `EVENTAREAB8::HitEnable`. Before it `volume.va` had
found ccDestEnemyK and the others through `carried`'s any-section search,
at addresses inside unrelated rows (OUT 0x00460f10 for K).

`piney-gen carry` fell back to main's name for an overlay global the
volume moved to main. For a literal (`@4704`, numbered per file) that put
48, 45 and 45 overlay strings of MUT, OUT and QUA on main's literals of
the same number (OUT's staff-roll names on `ccUngzip`'s data); literals
no longer take the fallback. Exposed when Mutation's `Func_str0300`
became exact and named main's `@4704`. Most are carried to their own
overlay now. Of the 5, 10 and 31 left uncarried, 1, 2 and 23 are texts
the volume does not hold (Quarantine's staff roll is another), 3, 6 and 6
texts it holds more than once, and the rest two: toppage's `Symbol` (7
bytes, under the content pass's 8) and, on OUT and QUA, demo's
`chb1body`, which those volumes keep in main (OUT 0x003832a0).

## The harness on Outbreak and Quarantine

`volume.loop_head(name)` reads the head of a thread's endless loop from
its last `b` back, and the trampoline jumps there; Infection's offset into
the function would land OUT before `ccEntryEventMng`. Two game
differences then showed, both read from the code:

- `DUNGEON::SetMagicCircle`'s story circle takes its position's `w` from a
  stack word the call never writes. INF/MUT build the vector at sp+160 of
  a 176-byte frame (4 below the caller's sp); OUT/QUA at sp+64 (100
  below; OUT gcmn 0x005e9288). The harness found it by offset 4; it now
  reads the slot from the function (`story_circle_w`). The port takes 1.0
  on every volume as before.
- `WORLD::SetFood` picks a food node by `ccRand() % n` signed on INF/MUT
  (`div`, INF gcmn 0x005aaafc), unsigned on OUT/QUA (`divu`, OUT
  0x005d397c, QUA 0x004c63dc), so the later two never redraw on a negative
  remainder. `entry::world_set_food` now does the same by volume; with the
  signed draw the foods swapped places and every `ccRand` after them moved
  on (the symbols' values in the same case).

The food draw moves the play bot's GOB3-1 run on Outbreak: two charms
ran out at frame 18,000 with one goblin at 72 HP, and the pilot went back
for the same two every time. `StoryPilot::provision` now carries one more
charm for each run on a field that fell short, and the test's budget is
48,000 frames (the event ends at 40,527).

## Checks

- test_battle_spawn_rs, all six checks: INF, MUT, OUT, QUA OK (OUT and
  QUA failed to start on main).
- With the old sidecars and carries put back against the new ones, on MUT,
  OUT and QUA: test_battle_event_rs 7, test_battle_kite_rs 5,
  test_bosscam_rs 3, test_field_rt 5, test_dungeon_rt 12, test_demo_rs 19
  tests, all OK both ways; the same on INF after. `test_stream_rs tables`
  writes the MUT, OUT and QUA stream table fixtures unchanged.
- `piney-gen syms --check` and `carry --check` current for all three;
  `gen --check` current (29 files, the tables' bytes included).
- The battle pins: 290 calls, 0 differ. Shots with the food change against
  the same tree without it: field_type_shots 168 and field_gims_shots 28
  the same; field_shots 2320, the 1160 of INF and MUT the same, 654 of OUT
  and QUA differ (327 each: the foods, and every `ccRand` after them).
- piney-game suite (uncapped): 348 passed, 0 failed, 152 ignored;
  piney-gen and piney-battle 166 passed; clippy and fmt clean.

The sidecars and carries are data under `work/` (main's copies are
untouched; this worktree's `work/` holds its own): main's need
`piney-gen syms` and `carry` for MUT, OUT and QUA after the merge, and
the OUT/QUA field goldens their new shots.

**Still unknown:** nothing new; the words a story circle's `w` holds in play stay as docs/engine/battle.md's Unknown has them (the slot is now located on every volume: 4 below the caller's sp on INF/MUT, 100 on OUT/QUA).
