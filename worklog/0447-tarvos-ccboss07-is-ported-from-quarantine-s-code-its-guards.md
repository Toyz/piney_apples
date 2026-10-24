---
number: 447
title: Tarvos (ccBoss07) is ported from Quarantine's code: its guards held off by the kind of blow, its spike rain and phantom drawn as they fly, its blows' notes from the fight's ccRand, and event 310 ends in the arena under god with its music
date: 2026-10-24
area: battle, render, volumes, test
files: crates/piney-battle/src/boss/tarvos.rs, crates/piney-battle/src/boss/tarvos/eff.rs, crates/piney-battle/src/boss/tarvos/tests.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/tarvos.rs, crates/piney-effect/src/tarvos.rs, crates/piney-effect/src/boss.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/combat/mod.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session/tests/arena.rs, crates/piney-game/src/session/tests/tarvos.rs, crates/piney-gen/src/manifest.rs, tools/test_battle_tarvos_rs.py, docs/engine/boss-tarvos.md
started: 2026-10-24T04:18:08-06:00
took: 44m
---

# 447. Tarvos (ccBoss07) is ported from Quarantine's code: its guards held off by the kind of blow, its spike rain and phantom drawn as they fly, its blows' notes from the fight's ccRand, and event 310 ends in the arena under god with its music

Event 310 stopped in field 7's arena. Block 7's `entry 7 6` asked for
a boss the port did not have. Tarvos (`ccBoss07`, boss07.cpp, QUA gcmn
0x003a81e0-0x003ac9f0) is now ported from Quarantine's own code. It uses
the names and layouts of Infection's DWARF; Infection's gcmn carries the
class but never builds it. Its two own effects are ported too: the spike
rain (`ccBossEffSpikeRain`) and the phantom (`ccBossEffPhantom`).
docs/engine/boss-tarvos.md has the whole of it.

Event 308 needed nothing. Under the god survey, Kyvia's third fight on
field 11's disc ends within 200,000 frames of the event's start. The
survey's 40,000 had been too short.

## What it does

Tarvos is `bossTbl` row 6, file `x71`, body `CMP_trall` (drained,
`CMP_trall2` with `ANM_ex7x`). Its acts are:

- a spike rain, a wave and a sacrifice (stream 103 on a member, then 9999
  on them);
- a phantom of fourteen lights;
- a transfer that swaps its guard;
- skills, magic and a Data Drain attack;
- chases and escapes.

It drains to an Epitaph of 7000 HP and dies there.

What its code does that one would not guess:

- **Two guards.** `SetDefMode(0|1)` points `param` at `bossTbl` row 42
  or 43 and copies only that row's stats into the boss. A blow of the
  guarded kind (`ccSkillCheckType` of the blow's skill) shows
  `effResistantShield` at 0.7 its size and 80 up. The drain's
  `SetDefMode(2)` is `SetBaseParam(row 6)` whole.
- **Its Affect draws `ccRand`.** A blow in the neutral sounds 195 at note
  `60 + abs(ccRand() % 5)`. The port's `boss::entry` ran a boss's Affect
  with a throwaway generator, so the note came out wrong (62 against
  63). `BossEnv` now carries the fight's `ccRand` (`cc`). The world hands
  it to the queued affects of the frame, and so does the probe.
- **Its own pattern words read their operands from `patTbl`,** as
  Macha's do. The base's words reach the chase and the escape. No table
  reaches the dash, the wander or the return.
- **The base `IsValidArea` is a box**, the later volumes' code: `|x|`
  and `|y|` each under 2500 past the centre's. `Boss::valid_area` is
  Skeith's circle. The escape went wrong 517 frames in until Tarvos took
  Fidchell's box.
- **The spikes' hits and the phantom's ring happen inside their
  `Draw`,** in the manager's pass. `_SpikeShotCB` (boss skill 24; every
  eighth an eruption, made off) and `ccBossEffAutoSamonRingCreate` used
  to come back as requests after the draw. That put the ring after the
  phantom's SE 35 in the frame's calls. The effects now act on the boss
  as they draw (`Fx::draw(cx, boss)`).
- **The phantom's constructor hands `SetFreeCamPosView` a view it never
  wrote** (sp+144). `operator new`'s frame (`_ccMalloc`, 144 bytes) left
  `$s1` there: OnThinkPhantom's target pointer (a float under 1e-37), its
  sign word, and `$s1`'s upper half. No code of the game writes that
  upper half except by saving and restoring it, so it is what the
  kernel's `StartThread` put in the boss thread's first context
  (`ccTscb::GoThread`, a kernel thread on a `ccMalloc`'d stack). The port
  takes (0, 0, 0). The next manager pass sets the view anew.
- **The transfer's fog** is `PreDrawAnm`'s
  `SetFogBlend(m_blendRate, 0x404080)`. The body keeps it when
  `ccBoss::Draw` sets no blend of its own.

## Checks

`tools/test_battle_tarvos_rs.py` runs the game's ccBoss07 natively
against `battle_probe tarvos`, frame by frame. The spike rain, the
phantom and the eruption also run natively, and their slots are
compared. The cases cover hits of types 1 and 3, blows by real skill
ids, the gauge, the drain, members down and forced acts. 300 random
cases (913,177 frames) agree. They reach acts 0, 3-7, 12-18, 20-23, 25
and 26. Its unittests run the stand-ins' lives.

The harness found three port bugs:
- the note (above);
- the box area (above);
- the ring's order (above).

It also found one harness bug: `ccIBossEff::cmn_bossCam` is set by
`InitBossCamera`, which the harness stubs.

`boss::tarvos::tests` check six things:
- the constructor's guard;
- the shield;
- the super table at half the gauge;
- the drain;
- the rain's four eruptions;
- the phantom's fourteen bursts.

`session::tests::tarvos::tarvos_under_god` (ignored) plays the arena
under god:
- acts 0, 3-6, 12-14, 16, 18, 21-23 and 25 come;
- event 310 ends at frame 43,610;
- the music plays through 43,268 of those frames.

The shots show the body under the blur, the spikes, the phantom's
lights, the transfer and the Epitaph.

`tarvos_fight_to_its_end` (no aid, ignored) has the start's party (70,
70, 75) fall at frame 35,022, before the drain. `event_310_from_its_start`
is kept ignored for the whole runs. The arena set-up, the pilot's play
and the god shots are shared with Macha's tests (`session/tests/arena.rs`).

## Pictures

`piney_effect::tarvos` draws:
- the spikes (`CMP_ex71body2` at a quarter);
- the phantom's sprite (`EFF_x000` on `CLT_x000c7`);
- its lights' `g_phantomGen` generators, which follow the lights through
  anchors, change at the fall and are deleted at each burst;
- the bursts (`effExplode3`, 1 to 4 over 15);
- the radiate;
- the explosion (`explode2`, now shared with Gorre's flash);
- the transfer's and eruption's generators;
- the shield.

piney-gen adds `g_transEffGen`, `g_phantomGen` and their force fields.

**Still unknown:** whether the pilot beats Tarvos with no aid (the
start's party falls at 35,022 frames); what the kernel's `StartThread`
leaves in the upper half of `$s1` for the boss's thread (taken as 0); the spikes', sprite's
and lights' pictures against the game's draw.
