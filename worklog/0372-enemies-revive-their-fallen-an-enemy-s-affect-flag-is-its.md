---
number: 372
title: Enemies revive their fallen: an enemy's affect flag is its character's
date: 2026-10-04
area: battle
files: crates/piney-battle/src/chara.rs, crates/piney-battle/src/enemy_ai.rs, crates/piney-battle/src/enemy_motion.rs, crates/piney-world/src/combat/mod.rs, crates/piney-battle/examples/battle_probe/enemy_ai.rs, crates/piney-battle/examples/battle_probe/enemy_motion.rs, crates/piney-battle/examples/battle_probe/spawn.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/enemy_revive.rs, crates/piney-game/src/session/tests/side_events.rs, docs/engine/battle.md
---

# 372. Enemies revive their fallen: an enemy's affect flag is its character's

A player report (#41, build f6e01cc): enemies never revived each other.

## Who revives, and how

Rip Maen (skill 180: `triggerRange` 4000, 40 SP) sits in a spell slot
of eight Infection rows: the Menhirs (179 Menhir, 180 Goil Menhir, 181
Statue Menhir, 182 Sphinx Menhir, 183 Fiend Menhir, `mag0`) and the Arks
(97 Ark Prince, 98 Skate Rat Ark, 99 Alucard, `mag1`).

- `checkSkillList` marks the slot usable while a foe is dying (`dead` 2)
  within 4000 (`ccSearchNearPerson(me, 0x60, 2, range)`) and the caster
  has the SP. `selectAttack` takes such a slot first (`intFlag`);
  `setSkillRate` gives it no share of the random draw.
- The run's `ccSkill::RecoverySystem` gives the target full HP and no
  SP (not while the player's party is all down) and `EntryAffect(20)`.
- `affectEnemy` restores the HP of a `dead` 2 enemy. The next
  `interruptThink` sees the affect and wakes it: `dead` 0, wait. It stays
  on the lists; no experience was given (that comes at act 8's frame 91).

## The cause

The port had all of that. A probe with two groups of Menhirs (`put_enemy`
makes three) beside Kite in story area 14's field felled one with
`EntryAffect(1, 9999)`. Menhir 49 cast Rip Maen on it 21 frames later.
It got its 730 HP back and stayed `dead` 2 in act 8, then left the lists
at frame 91 and died.

`ccEnemyInfluence` (0x00432840) sets `affectFlag` (`ccEntryObj` +0xe0
bit 3) before `affectEnemy`, and `drainFlag` (+0x250 bit 2) after a Data
Drain. It does this whatever made the affect. The port kept a second copy
of both, and of `affectType` and `affectParam[0]`, on `enemy_ai::Enemy`.
Only an enemy's own frame (`enemy_motion`'s `entry_affect`) and the menus
(`Combat::entry_affect_with`, worklog 0229) wrote it. A skill's run
(`ccThSkill`, where Rip Maen lands), Kite's blows and the members' blows
all went to the character alone. So `interruptThink` never saw them:

- a revival never woke its target;
- no blow from Kite, a member or any skill made an enemy flinch (act 7).
  Before the fix, Mia's 10-damage blow on a Menhir left it closing in;
  now it flinches that frame.

## The fix

The character is the one copy, as the game's one object is:

- `Char::enemy_affected` is `spc_flag::AFFECT`, which `enemy_influence`
  already set.
- `Char::take_enemy_affect` is `interruptThink`'s read: it clears the
  flag and returns `affect.ty` and `affect.param[0]`.
- `Char::enemy_drained` is `enemy_flag::DRAIN`.

`Enemy` lost `affect_flag`, `affect_type`, `affect_param0` and
`drain_flag`. `note_affect`, `note_landed`, `affect_lands` and `Landed`
are gone. The gold goblins' break from a hold (`thinkGold`, from Mutation
on) reads the same flag. `battle_probe` reads and writes these words
through the character (`OnChar`).

## Checks

- `enemy_revive::a_menhir_revives_its_fallen`: a Menhir cast Rip Maen on
  the fallen one; it is up with full HP and no SP, and still listed past
  the 90 frames of dying.
- `enemy_revive::a_members_blow_flinches_a_foe`: Kite opens the fight
  with Convergence; Mia's first blow on a Menhir that is not attacking
  makes it flinch.
- Both fail on 38f58bf and pass now.
- The eemu harnesses pass: `test_battle_enemy_ai_rs.py` (unit, and bulk
  100 of every check, 0 mismatches), `test_battle_enemy_motion_rs.py`,
  `test_battle_spawn_rs.py`.
- piney-battle's, piney-world's and piney-game's tests pass (233 passed,
  82 ignored, four threads).

`gob3_1_golden_goblin_is_run_down` (Outbreak) had only passed because of
the bug. Kite's arts and blows hold their target, and a held golden goblin
that a blow reaches breaks free, so each blow ends the skill or combo.
The trio's Repths (150) go on without end: their SP stayed at 405
through every heal. Under the pilot they held at 400-870 of 1,170 HP
for 40,000 frames. The
survey's `god` aids now include `gold_within_a_blow`, which keeps a golden
goblin's HP at 250 so one blow fells it. Then the event's flow (block 5's
goblins, block 8's `no_active`) passes again.

The ignored `infection_whole_story` stalls in Mac Anu after events 1-4
and 10, at frame 215,400. It does so on 38f58bf as well, at the same
frame, so the stall predates this change.

**Still unknown:** whether a lone Kite can run down GOB3-1's golden
goblins in the real game, or how players do it. That needs a capture of
the event on a PS2 or a player's pad log. The `infection_whole_story`
stall in Mac Anu, already there on 38f58bf, is not looked into here.
