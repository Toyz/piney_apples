---
number: 238
title: A party member's skill runs its own animation
date: 2026-09-27
area: battle
files: crates/piney-battle/src/flow.rs, crates/piney-world/src/combat/mod.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session.rs
---

# 238. A party member's skill runs its own animation

Reported from Mutation's play: Kite's first combat skill from the Skills
menu played its sound and did nothing else, and the menu did not let go.

## Reproduced

A diagnostic in field 27 walked Kite to the monster portal and, in the
fight, chose PERSONAL, Skills, the first skill (Staccatto, 8), and the
target. The OK took the SP (100 to 85) and set Kite's `skillID` to 8.
Kite then played his art (act 21, `ANM_ctu1ski4`) for about 170 frames.
The Killer Snaker's HP never moved.

The target menu (65) waits in its tail while the world reports a skill
of Kite's running (`ccSkillCheck`). The run stayed there until the fight
ended. It would otherwise have ended at `Main`'s 450-frame limit, for a
skill with no system of its own.

## The game's rule

`_ccSkillRequest` (INF gcmn 0x00572c14; MUT 0x005983d4, the same) gives
the new `ccSkill` its own animation (+0x80) when all of these hold:
- the caster casts the skill itself (`stype` 0);
- the caster is a PC (type bits 0 or 2: `*(char) + 8 & 5`);
- the skill's row names a file (`param + 4`);
- `xnote_ccs` is set.

That animation is `new ccAnm`, `SetAnm(GetChunkAdrsF(GetCCSAdrs(file),
"ANM_" file), 0)`, with `ccSkillCheckNote` as its note function.
`xnote_ccs` is `GetCCSAdrs("xnote")` when `ccThSkill` starts, and
`XNOTE.CCS` is a common (`cmn`) file, so it is set in every area.

`ccSkill::Main` (0x005734d4) then works the animation each frame:
- `_AnimateForward(frameSpd)` while it has data;
- `NoteProcess`, which runs `ccSkillCheckNote`:
  - 0x8005 is `NoteEventAffect`, the hits;
  - 0x8002 is a shock wave;
  - 1 and 2 are the caster's sounds.
- At its end, the caster's skill fields are cleared and the run ends.

The skill files are tiny (`CTU0SKI4.CCS` is 3,604 bytes): timing and
notes only. Nothing draws them.

The port had `SkillRun::has_anm` and `Frame { anim_done, notes }` for
this, but never set the first, and the combat loop always handed `main`
an empty frame.

## The fix

- **The run** (`Skills::add`). A run gets `has_anm` by the rule above,
  with the new `Skills::xnote` in for `xnote_ccs`. The combat sets it on;
  harnesses without the file leave it off, as before.
- **The effects side** (`FxTasks::skill_anim`, in `AreaFx`). On a run's
  first call it reads the skill's file once and makes a `Play` of
  `ANM_<file>`. Each call then steps it (`forward_notes`: its end and its
  notes). The `Play` goes with the run (`spell_remove`).
- **The combat loop.** It steps a run's animation where `Main` does and
  hands the notes and the end to `SkillRun::main`. If the file or the
  animation cannot be had, the run ends at once. The game's would never
  end.

## Checked

The new test `mutation_skill_from_the_menus` plays the reported steps:
- the menu shuts within 400 frames of the OK;
- the monster takes damage.

In the diagnostic run, the monster fell from 1,250 HP in a string of
Staccatto's hits, the skill ended with its animation (about 170 frames),
and the menu shut.

The Rust suites of piney-battle, piney-world and piney-game pass. So do
`test_battle_flow_rs`, `test_battle_kite_rs`, `test_battle_party_ai_rs`
and `test_battle_rs` on Infection and Mutation.

Two other reports looked at here:
- **CHAT after a fight.** Mutation's `ChatMenu` is Infection's
  instruction for instruction. In a field or dungeon it always has the
  Skill Usage, Strategy and Members pages, in battle or not. The one
  extra order, the Sprite Ocarina's, shows only in a dungeon out of
  battle. The battle mode itself ends properly (`in_battle` 1, 2, then
  0).
- **The quit prompt** over another menu drew its text into that menu's
  rows. It kept a held picture without its uploads, while a frame's
  upload ids restart at 0. It now sends the picture's uploads again each
  frame (c3a70e3).

**Still unknown:** Whether any skill's animation draws; these files have
no clump, and nothing in the game draws +0x80. The party AI's skills go
through the same request, but they were not watched in play here.
