---
number: 294
title: Fidchell, Outbreak's first phase boss, and its four spells
date: 2026-09-28
area: battle, render, volumes, test
files: crates/piney-battle/src/boss/fidchell.rs, crates/piney-battle/src/boss/fidchell/eff.rs, crates/piney-battle/src/boss/fidchell/tests.rs, crates/piney-battle/src/boss.rs, crates/piney-battle/examples/battle_probe/fidchell.rs, crates/piney-world/src/combat/boss.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-gen/src/manifest.rs, tools/test_battle_fidchell_rs.py, docs/engine/boss-fidchell.md
---

# 294. Fidchell, Outbreak's first phase boss, and its four spells

Outbreak's event 207 now fights Fidchell in field 4 and runs to its end.
Block 9's `entry type=7 code=3` makes `ccBoss04` (`bossTbl` row 3, file
x41) and block 11's `absent type=7 code=3` follows once it dies. The
reference is [docs/engine/boss-fidchell.md](../docs/engine/boss-fidchell.md).

## How it was taken apart

The 31 functions of boss04.cpp are Infection's (INF gcmn
0x00491af0-0x00496ee4, with DWARF: `PRED_TEXT`, `PREDICTION_T` and the
class's 0x230 bytes of own members), but the fight is Outbreak's, so the
port is Outbreak's code (OUT gcmn 0x004a3470-0x004a900c). Outbreak's
compile is still close to Infection's unoptimised output, so Magus's
lifter (straight-line pseudo-C over the disassembly, [[274]]) read it as
well, and a normalised diff of the two lifts gave the changes. The class
runs on the base `ccBoss` (`Main`, `Move`, `ChangeAction`,
`ChangeNextPattern`, `IsValidArea`): where Fidchell uses it, Outbreak's
base differs from Mutation's in stack slots and the display size read
from globals (its `SelectTarget` is recompiled past a text diff; the
harness agrees with the port's).

Fidchell's own acts: the prediction (act 3) picks one of four spells by
`ccSys.count & 3`, shows its text (x41's `ANM_ex41txt1-4`) and asks a
voice, then lays `@1441`'s skill (171, 170, 173, 169) on each member;
pattern 15 casts it (act 5): the meteors (`ccBossEffMeteoSworm`, 50
meteors), Skeith's ice (force generators, ring, ice at each member, the
screen reversed), the thunders (`ccBossEffThunderStorm`, 16 bolts) or the
quake (`ccBossEffRockTower`, 128 towers). Beside them the wave, a member
drained through menu 74 (stream 68), chases, escapes, back dashes, dashes,
a skill of `@2048` and in the Epitaph a spell of `@2081` and a self-heal
(skill 151). Drained, it has 5000 HP.

The game's slips are kept. `OnThinkPrediction`'s last step counts its
frame twice. `OnThinkEscape` never sets a heading. `OnThinkBackDash` turns
at its first frame only when already out of the arena. Pattern 9 with a
negative type picks among the members whose `dead` is set. Pattern 10
draws a skill it drops (`OnThinkSkill` draws again). `OnThinkMagic`'s
request flag is `a3` as the frame left it (the harness measures 1). The
ice deletes its `ccBufferReverce` and layer and keeps the pointers.
`OnThinkTransfer` (act 21, never reached) tests its turn against pi twice.

What Outbreak changed from Infection: the boss camera (250 and a
`MaxRenge` of 600 against 200); shorter Normal and Super tables; cinemas
on the prediction (64-67), the spell (14-17), the skill and the spell
(`OnCinemaMode` with a skill id, OUT 0x004733b0) and the member drain; the
menu cursor turned off around every move off the targets; the spell's
camera (1800 off the centre, then behind the target) and Fidchell's turn
to the centre; the ice's end (no second `ChangeNextPattern`, its SEs at
the target); the thunders' camera call dropped (its vectors still worked
out).

## The square root

Outbreak's main has the FPU's `sqrt.s` inline in `ccGetDist` (OUT
0x001e6ec0) and in `_ccBossSkillDamage`, where Infection and Mutation call
newlib's `sqrtf`. `sqrt.s` truncates, so the two part in the last bit
wherever newlib's rounds up. With newlib's the harness's case 8100 parts
at frame 61 (`targetDist` one bit high). The port's boss rules now take `sqrt_of(cx, v)` by the
disc's volume (the field's `Sqrt` choice, [[280]]), in Fidchell's frame
and target and in the skill damage's range test; on Infection and
Mutation nothing changes. Other callers of the battle's `get_dist` on
Outbreak still take newlib's.

## The spells

The spells' effects are what the boss waits on, and three of them draw
`ccRand` and play sounds in their `Draw`s, and the meteors write the
boss's camera view each frame, so their rules are ported
(`fidchell::eff`): the meteors' flights (`ccBossEffMeteo2::Init` and
`Move`: heading, pitch by `atan2f`, a pull in doubles), the bolts' growth,
strike and fade (16 `ccRandF` a `Draw` plus one a segment, 16 at the
strike), the storm's waits and rumbles (and the magic square the first
bolt's end makes inside the manager's pass), the towers' waits, rise and
fade. Their pictures are named (`Out::Fidchell`), not drawn.

## Checks

`tools/test_battle_fidchell_rs.py` (`PINEY_VOLUME=outbreak`) builds the
game's `ccBoss04` and runs the base's `Main` natively (with the manager's
pass, the three spells native) against battle_probe's `fidchell`, frame by
frame. A case has one to three members, 800 to 7000 frames, hits and the
gauge, the drain's 13 and 21 with hits after, its own skills ending, a
member down for a while, a push, the menu's type, the camera turning and
`checkCameraShakeRange` answering no on every nth frame. Compared each
frame: the acts, movement, place, target, HP, gauge, flags, clips, every
own member, the boss camera's `MaxRenge`, the effects' slots, the party,
the calls, the menu, `rand()` and `ccRand`'s index. `bulk 40` (seed
9700): 40 cases, 125,426 frames agree; acts 0, 3-7, 12-14, 16, 18, 22, 24
and 25 and all four spells are reached. `test_effect_lives` runs the nine
stand-ins' `Create`s and `Draw`s natively: the WaveShock 45, the magic
squares 91, 71, 91, the force generators 133 and 193, the ring 20, the ice
123, the dead effect 120, the port's lives. Skeith's, Innis's, Magus's and
Kyvia's harnesses still pass. `boss::fidchell::tests` hold five cases.

## Under the autopilot

`PINEY_SURVEY_ONLY=207 PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=120000` ends
`story 207: done`. The fight begins at frame 43,650. Two aids were
needed. Kite stood where the pilot left him while Fidchell roamed its
arena (its skills need a foe within 400): the pilot now walks him to
within 350 of Fidchell when no other foe is near (`approach_roamer`). And
at the story's levels (about 53) the gauge (a physical defence of 2000,
`maxPP` 16000) took 50,000 frames and the Epitaph's self-heal outpaced
the party: the party is raised to 75 through the console's `exp` while
Fidchell is up (`levels_for_boss`). A broken protect no longer waits out
the last skill's 240 frames before Kite drains (the break lasts under 300).
Even so the first break (77,350) is missed, Kite far and stuck; the second
(96,950) is drained, and the body dies at 112,100. With these aids
Mutation's 107 and 108 (150,000 frames) and Outbreak's 203 (60,000) still
end done; Mutation's 115 (200,000) stops in field 52, before Magus's
field, with or without them (4880 places either way).

**Still unknown:**
- The pictures: the prediction's text, the meteors and their light, the
  bolts, the towers and the dust are named, not drawn; the voices are not
  played; a skill's cinema shows its bars without the name (OUT
  0x00472550's rows are not read).
- `effSkillStart`'s controller is taken to end 8 frames on; its real end
  under the effect task is not run with the boss (any end under 30 frames
  gives the same fight).
- `OnThinkTransfer` is ported but no pattern reaches it, so it is
  unchecked; acts 20 and 23 likewise.
- Why the pilot's first drain fails: Kite is held against something about
  1200 off Fidchell when the gauge breaks.
