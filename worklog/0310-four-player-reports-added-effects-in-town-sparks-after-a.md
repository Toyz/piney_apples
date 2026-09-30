---
number: 310
title: Four player reports: added effects in town, sparks after a drain, area skills' marks, a charm cast
date: 2026-09-30
area: battle, ui, render
files: crates/piney-game/src/world.rs, crates/piney-world/src/town_party.rs, crates/piney-fieldui/src/disp.rs, crates/piney-fieldui/tests/fieldui.rs, crates/piney-battle/src/item.rs, crates/piney-battle/examples/battle_probe/items.rs, tools/test_battle_items_rs.py, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/fairy_orb.rs, crates/piney-game/src/fx.rs, crates/piney-world/src/field_world.rs, BUGS.md
---

# 310. Four player reports: added effects in town, sparks after a drain, area skills' marks, a charm cast

GitHub issues 1, 5, 6 and 8, all from Infection. Each one got a test that
fails without its fix. Addresses are INF gcmn unless marked otherwise.

## Issue 1: the Status menu showed no added effects in a town

The Status and Equipment menus read the added effects (critical,
drainHP, drainSP, dying, invincible) from the character's conditions 2-6.
`ccChar::CalcReal` fills them through `ConditionBattleEffect`
(0x0056c940): the largest value of each among the equipped pieces.
`ccPlayer::Main` (0x005983c8) and `ccFellow::Main` run it every frame, in
towns as well.

The port's town characters did run it: `Combat::town_frame` calls
`calc_real` on Kite's stand-in and on each member. But the menus' view of
the party (`ui_world`, piney-game world.rs) was built from the save
records, with the conditions left at zero. A save loaded into a town
therefore showed "Added Effects:" empty. The equipment menu's comparison
then showed every effect as new.

The fields were not affected: the HUD reads `ch.cond` from the fight's
characters, and `battle_effects` works from the same conditions. So the
report's "may not work at all" is only the town's display.

Fix: `TownParty::char(id)`, and `ui_world` copies that character's
conditions and speed. Test: `a_loaded_town_shows_the_equipment_s_added_effects`
gives Kite a blade with critical 2 and enters Mac Anu. Without the fix
the Status menu reads 0.

## Issue 5: a paralysis spell's sparks outlived a Data Drain

The sparks are the foe's own `ccConditionEffect` number 1:
- `setConditionEffect` main 0x001c24a0, constructor case 1 at 0x001c174c;
- two generators of `particleGeneratorTbl[198]` whose life is -1, so they
  never end by themselves.

Skill 157's `_ccSkillModifyCondition` (0x00575cf0) only sets paralysis
900 and `conditionNum` 1. Its `effSkillStart` pieces are short-lived.
A killed foe keeps running frames, and `dead` 2 makes its
`DispConditionEffect` kill the effect. A drained foe runs no more frames:
only `clearConditionEnemy` (0x004335d0, called at 0x00432e9c just before
`deleteCmnd`) can end it, through `ClearConditionEffect` (0x00570180).
That is worklog 279's fix (d894a29, 2026-09-28).

On the current tree the case is fixed. The new test
`a_drained_paralysed_foe_keeps_no_sparks` paralyses the goblin of
`drain_in_a_fight` as skill 157 would and keeps its HP up (the members
killed it before the drain). The test adds `FxCensus::char_generators`
(each live, unkilled generator's character) to check the effect side
too. Once the drop is handed out, no generator follows the drained foe.
With 279's world part turned off, the test fails: the effect is still
live on foe 52. Right after the drain, one short generator follows the
foe for four frames and ends on its own. The reporter's build probably
predates d894a29.

## Issue 6: an area skill's other targets were not marked

`TargetMenu` (0x00531af0; 0x00532178-0x00532348) and `ChatMenu3`
(0x0052c0b0, a member's skill) fill `subTarget[16]` (+0x1f4) every frame.
Which targets:
- skills whose `type` (+0x2c) has 0x6000, and Data Drain 3 and 5;
- in the target's chain;
- not dead;
- within `targetRange` (+0x20) plus their own `base.width` of the centre,
  measured flat.

The centre is Kite's posP for `type & 0x2000` (the arts, TargetMenu only)
and the target's otherwise. Items that cast a skill go through the same
rule with the item's skill.

The port already filled `subTarget` but never drew it or cleared it.
`Disp` (0x0051f3d8-0x0051f588) draws each entry inside the target
cursor's block:
- the diamond's cell (2464, 2048), 24x24 texels, drawn 20x20;
- centred (-10, -10), rotation 0, the cursor's pulse `f20`, colour and
  alpha;
- at `ccCalcTagPosChar(c, 0.45 * height)`, x held to 0..512, y to
  16..448.

Every path then zeroes the array (0x0051f5ec-0x0051f610). Without that
clear, an entry that left reach kept its mark, and it was also sent to
`DrainAffect`.

Mutation (0x0053d58c, `subTarget` +0x1f8) and Outbreak (0x00538620) draw
it with the same numbers. Test: `an_area_skill_marks_its_other_targets`
uses the Tiger Claws art (skill 7, type 0x2801, around Kite) and an attack
spell (193, type 0xc106, around the target). Each marks only the goblin
beside its own centre, and the mark goes once the goblin is out of reach.

## Issue 8: a Speed Charm held Kite still

The Speed Charm (11/56) runs skill 177 through Kite: `ccUseItemRequest`
(0x0057aa80) → `ccItemSkillRequest(pw, tp, 177, 1)`, stype 2.
`_ccSkillRequest` stores `skillID`, `skillStatus` 9 and `targetChar = tp`
(0x00572b78-0x00572b80). The port's `item_skill` left `target_char` as
it was. Kite's `AnimCtrl` (0x005995c0) found nothing to cast at, so act
18 never started. `ConditionModifySystem` then waited forever for an
animation end, and `control_move` would not walk him while skill 177 was
on.

Fix: an item cast through the user (stype 2) now aims at its target,
members' items too. `tools/test_battle_items_rs.py` now compares
`targetChar`. With the fix `targetChar` agrees in all three volumes
(Infection 300 cases a check, Mutation and Outbreak 100). Without it, 49
of 100 Infection `item_skill` cases differ. `_ccSkillRequest` stores it at MUT 0x00598330 and OUT
0x005948a0.

Tests:
- `an_item_cast_through_the_user_aims_it` (item.rs);
- `a_speed_charm_on_kite_ends`: PERSONAL, the charms' page, Kite. He
  plays act 18 and skill 177 is gone 300 frames on. Without the fix he
  never casts.

Outbreak's harness has a separate difference in category 13 (the map
item's calls, `showMapInfo`): 11 of 100 `use_item` and 7 of 100
`ai_use_item` cases. It is not this change.

**Still unknown:**
- Which build the issue 5 reporter ran. Also unchecked: three spots that
  could leave an orphaned condition effect:
  - `selectTarget`'s clear, dropped when the party is wiped
    (combat/mod.rs ~2155, ~1595);
  - `EnemyRetarget` flushing another enemy's output under the current one
    (enemy_motion.rs ~589);
  - `clear_condition_all_enemy`, which leaves `conditionNum` as it is.
- The attack cursor's scale branch (0x0051f21c) may read the previous
  `f20` where the port uses `ca`. It is untested.
