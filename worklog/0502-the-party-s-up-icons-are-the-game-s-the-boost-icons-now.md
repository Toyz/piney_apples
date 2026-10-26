---
number: 502
title: The party's UP icons are the game's: the boost icons now compared with ConditionIconDisp, and the members' buff casting already the game's
date: 2026-10-26
area: ui
files: tools/test_fieldui_rs.py, crates/piney-fieldui/examples/fieldui_probe.rs, BUGS.md
started: 2026-10-26T00:01:38-06:00
took: 10m
---

# 502. The party's UP icons are the game's: the boost icons now compared with ConditionIconDisp, and the members' buff casting already the game's

BUGS.md: many stat UP icons on the field (the party's boosts). One cause
was found earlier and fixed: buffs froze through town visits. Left
unmeasured was whether the members cast more of them than the game does,
and whether the icons are drawn as the game draws them.

**The icons.** `ConditionIconDisp` (INF gcmn 0x00522910):
- It draws up to 21 icons, five a row upward. Twelve of them are the
  boosts: the abilities read from personality +0x88 (temp[0, 1, 2, 4, 5,
  6]) and the six elements from +0x98. Each shows DOWN below 0 and UP
  above (cells 7-18 and 23-34).
- The rest are the conditions and speed (0x00522a80 - 0x00522dc0).
- tools/test_fieldui_rs.py could not set a member's temp, so this half
  had never been compared. A `boost` line now carries the fourteen values
  to the probe; the harness writes them to the personality.
- `test_boosts` compares every draw:
  - Kite with every ability and element up (fourteen icons on one
    panel);
  - Orca with a mix of ups and downs;
  - Mistral with conditions besides.
  No draw differs.

**The casting.** The members' buffs come from:
- `ccAI::BuffForUnusedFellow` (0x00593ce0);
- `SearchBuffUnusedFellow`;
- `CheckBuffSkillListNum`;
- `ChatCommandBuffPlz` (0x00585c70), reached through
  `ActInField`/`ActInDungeon` and `Reconnoiter`.
Each is compared with the game on random worlds by
tools/test_battle_party_ai_rs.py: return, every ccAI field, the message
bus, the skill requests made in order. The fights are compared frame by
frame by tools/test_battle_fellow_rs.py (worklog 477). So the members
boost exactly as often as the game's code has them.

**The time.** The boosts time out everywhere, towns included (the fix
above, `CalcReal` in town).

So a party that boosts often shows a panel full of UP icons in the game
too.

**Checked.** The whole field UI harness (75 tests) and piney-game's suite
(336).

Docs: BUGS.md.

**Still unknown:** nothing
