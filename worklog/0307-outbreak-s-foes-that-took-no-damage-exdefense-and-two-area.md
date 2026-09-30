---
number: 307
title: Outbreak's foes that took no damage: Exdefense, and two area rules
date: 2026-09-30
area: battle, volumes
files: crates/piney-battle/src/damage.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/side_events.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-game/src/session/tests/data_bug.rs, crates/piney-game/src/session.rs, docs/engine/battle.md, plans/outbreak-side-events.md, BUGS.md
---

# 307. Outbreak's foes that took no damage: Exdefense, and two area rules

Worklog 306 left three side events of Outbreak open, each on a foe that
took no damage: MARLOWE-2 (256) in room 0-2 of field 84's dungeon,
TERASHIMA-1 (262) in room 2-6 of field 90's, and SERVER-3 (263) at Black
Death. None of them was a fault in the port. The game bars these foes from
one kind of hit, and the autopilot kept using that kind.

## Where the damage stopped

The survey got a log of each foe's HP and last affect (kind, damage,
skill, attacker) as they change (`PINEY_SURVEY_HITS`). Every hit landed
and every affect reached the foe. `CalcBattleDamage` returned 0:

- **256.** Gaia Turtle (OUT row 114): pDef 700, mDef 1,220, Exdefense
  0x1. Every ATTACK sent 0; Juk Kruz (skill 245) took 194 now and then.
- **262.** Deadly Present (row 243): pDef 1,920, Exdefense 0x1. Juk Kruz
  took 383 a cast until the walk gave the last two up. With the doors
  shut, Kite then went at them with ATTACK for 0.
- **263.** Black Death (row 176): pDef 4,500, mDef 2,400, Exdefense 0x2.
  Kite cast Juk Kruz for 0 on every try. Its 9,990 is the physical PP
  defence, the protect gauge's, not pDef.

## The game's rule

`ccChar::CalcBattleDamage` (INF gcmn 0x0056d910) reads the row's
`exdefense` (+0x64). Bit 0x1 bars a physical skill while the target's
`real` pDef is not below the row's. Bit 0x2 does the same for magic and
mDef, and bits 0x4-0x80 for a skill of that element. A barred hit does 0
and fills no protect gauge. Infection keeps the whole value while any
matching defence holds (0x0056dc64). From Mutation on, only the bits
still held count (MUT 0x005933f8, OUT 0x0058f5c0).

`tools/battle.py damage` runs Outbreak's own code. Kite at 75 does 0 to
the Gaia Turtle with ATTACK, and 31-35 a blow to Black Death. So Black
Death is beaten with blows (about 300 of them) or with spells once a
debuff has lowered its mDef.

`test_battle_rs.py` agreed: damage, exdefense and affect gave 0
mismatches in 300-500 cases on INF, MUT, OUT and QUA, and protect on INF
and OUT. A new test pins the
rule on Outbreak's rows: `exdefense_bars_a_kind_while_its_defence_holds`.

## The autopilot

`StoryPilot::choose` chose the kind by the higher defence alone. That
meant spells at Black Death, and blows (with Skills! to the members) at
the Gaia Turtle. Now a sure hit (`CalcBattleDamage` at 100 on a copy of
the foe) tells it whether a kind is barred:

- A foe barred from one kind gets the other.
- Skills that would do 0 are left out, both Kite's and the members'.
- `choose` also skipped a foe the walk had given up (`hopeless`), even
  while the walk still went at it behind shut doors. The walk now records
  its foe (`Walker::foe`), and that foe is fought with skills.

SIGN-7 (265) then met its Data Bug (row 261, level 68) at another moment.
The bug felled a Kite of 75 (1,395 HP) with two blows of 703 in one frame,
at 47,391. Before this change, the run had finished at 131,678 by luck.
The side events now raise a lone Kite to 90 (a harness aid, as god is).
The story keeps its 75.

Now 25 of the 26 side events finish; only GOB3-5 (254) is open. The new
times are 256 at 66,308, 262 at 41,348, 263 at 20,134 and 265 at 62,918.
`server_3_black_death_falls_to_blows` fails with the old choice (open
after 28,000 frames) and passes with the new one.

## Two area rules the harness found

`test_battle_rs.py` area_damage showed two differences on the later
discs. Both are ported:

- **From Mutation on.** `ccSkillDamage` gives up on a downed target only
  in its single-target branch (MUT 0x00599988). An area skill aimed at a
  downed foe still hits the foes around it. Infection returns at once.
  The port had Infection's rule everywhere. Test:
  `an_area_skill_at_a_downed_foe_hits_those_around_it_from_mutation_on`.
- **From Outbreak on.** All three area rules set the types to 0xe0 once
  they walk the foes' list: `ccSkillDamage` OUT 0x00595dac, its point
  variant 0x005962f8, `ccSkillDamage2` 0x0059664c, and QUA 0x004886ec,
  0x00488c38, 0x00488f8c. An area skill aimed at an enemy then also hits a
  Data Bug or a boss in reach. Test:
  `an_area_skill_takes_in_every_foe_type_from_outbreak_on`.

Both tests fail without their fix. With both fixes, area_damage and
skill_damage show 0 mismatches in 1,000 cases on INF, MUT and OUT. Before,
the misses were 26 of 300 and 5 of 500.

## Infection's drained forms

Reported as "too tanky" (BUGS.md). Two paths were tested:

- A drained Gremlin (row 129, 50 HP) is a target after its 60-frame grace
  and falls to the attack button: `a_drained_form_falls_to_kites_blows`.
- A drained Data Bug's base form (row 113) has no `virusFlag`, and a hit
  of 100 takes 100: `a_drained_data_bug_s_base_form_takes_whole_hits`.
  `affectEnemy` reads the flag from the enemy itself (INF 0x00433c0c:
  byte 0x250, bit 6).

Every drained form on Infection is a level-0, 50-HP row with no
Exdefense, so the report stays open. 186 of Infection's 303 rows do carry
an Exdefense bit, which leaves a barred kind of hit at 0. That may be what
the report saw.

## Checks

- Suites: piney-battle 117 + 5, piney-world 88 + 3 + 12, piney-game 171.
- Clippy (the three crates, all targets) and fmt are clean.
- Enemy AI (15 checks) and enemy motion (11) match on OUT at 200 cases.
- `test_battle_kite_rs.py` on OUT: `main_run` differs in 10 of 200 cases
  and `main_run_ai` in 31, each in one float of an AI record. Infection
  and Mutation match. This was there before these changes.
- `outbreak_whole_story` ends at 408,000, from 463,200 (203 at 63,300,
  from 77,700). `mutation_whole_story` ends at 506,100, from 613,800.

**Still unknown:** why Infection's drained forms feel too tanky (a pad log
of the fight is needed); the one-ULP float in Outbreak's Kite `Main` AI
record (word 34 of 71); which debuff lowers Black Death's mDef or the Gaia
Turtle's pDef below the table in play; whether a Data Bug's two blows in
one frame (SIGN-7) happen on the game as they do in the port.
