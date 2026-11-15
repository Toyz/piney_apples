---
number: 603
title: An enemy's area attack aimed at a downed target hits those around it from Mutation on
date: 2026-11-15
area: battle, volumes, test
files: crates/piney-battle/src/damage.rs, crates/piney-battle/src/enemy_motion.rs, docs/engine/battle.md
started: 2026-11-15T08:22:54-07:00
took: 1h 21m
---

# 603. An enemy's area attack aimed at a downed target hits those around it from Mutation on

The port was wrong, not the harness. `test_battle_enemy_motion_rs.py`
failed on Mutation in `mainedge` (found by [[601]], the same on main
b1a9d90 and edb7810): at frame 2, call 74, enemy c5 (row 74) passed its
attack's note 0x8005 straight to the weapon trail, where the game first
made `ccEntryFlyFontNew(2, 0, c5)` and `ccHitMarkDisp(c5, c5)` and set
the enemy's affect flag (eflags 49 against 57). c5's skill slot held the
revival (skill 180) aimed at character 1, a foe on the enemies' list with
`dead` 2.

`ccEnemy::affectSkill` lets skill 180 aim at a downed target and calls
`ccSkillDamage(this, skillTarget, skillParam, 0)`; it is the same on all
four (INF gcmn 0x00435b80, MUT 0x0044b520, OUT 0x004466f0, QUA
0x00339000, the last two only with nops added). The difference is in the
five-argument `ccSkillDamage`:

```
INF 0x00573ef8  lh $v0, 8($s3); beqz -> go on      a downed target: return 0, before anything
MUT 0x005996a0  bnez after ccCheckTarget(target): no dead test here
MUT 0x00599988  lh $v0, 8($s3); bnez -> return     only the single-target branch (targetRange <= 0)
OUT 0x00595fe8, QUA 0x00488928                     the same as Mutation
```

So from Mutation on an area attack aimed at a downed target still hits
the living around it. On Mutation the area takes the target's type bits,
so c5, a foe of the same type standing in its own reach, hit itself
(`CalcBattleDamage` gave 0 here) and showed the "0" and the hit mark.
`damage::skill_damage` (the spells' and members' path) already had the
rule from the area_damage check; `Motion::skill_damage`, the enemies'
copy that applies each affect after its roll, still refused a downed
target on every volume. The entry test is now written once,
`damage::skill_damage_starts` (listed attacker with `dead` 0 or 1, listed
target, and on Infection a living one), and both callers use it, with
the single-target branch giving up on a downed target. The other variants
were checked and need nothing: `ccSkillDamage2` is an exact match on
Mutation (0x00599e80) and the point variants are unchanged.

Outbreak and Quarantine passed before only because their random cases
never reached a revival on a downed foe with others in reach; the rule
applies to them too.

Checks, 2026-11-15: `test_battle_enemy_motion_rs` 4 of 4 (Mutation's
`mainedge` now passes), with `test_battle_rs`, `test_battle_flow_rs`,
`test_battle_event_rs`, `test_battle_kite_rs`, `test_battle_spawn_rs`,
`test_foe_rs`, `test_field_rt` and `test_dungeon_rt` on all four, 36 of
36, every line the same as [[602]]'s leg-5 run apart from the fix; battle
pins 291 calls, 0 differ; golden shots 4550, 0 differ against
world2/shots/base; crate tests 42 ok; piney-game suite 348 passed, 0
failed, 153 ignored; gen --check, clippy, fmt clean. The docs are
docs/engine/battle.md ("From Mutation on").

**Still unknown:** nothing.
