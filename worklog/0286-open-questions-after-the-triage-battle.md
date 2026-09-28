---
number: 286
title: Open questions after the triage: battle
date: 2026-09-28
area: battle
files: UNKNOWNS.md
resolves: 12, 13, 22, 67, 78, 109, 110, 123, 149, 155, 167, 168, 169, 171, 187, 192, 221, 222, 223, 229, 230, 268, 272, 273, 274
---

# 286. Open questions after the triage: battle

This entry closes the open questions of 25 entries about battle: the tables,
the rules, the enemies' and the party's decisions, items, Data Drain and the
bosses' rules. The triage of 2026-09-28 (UNKNOWNS.md) split every entry's
open paragraph into single questions and checked each one against the later
log, the docs and the code. The answered ones are listed below with their
evidence. What is still open is restated at the end, with the entry that
asked it, the play questions first. Open questions of these entries that
belong to another subsystem are restated in that subsystem's entry: [[282]],
[[284]], [[285]], [[287]], [[288]] and [[291]].

## Answered

- [[12]]: what the enemy table's `type`, `Exdefense`, `entry.*` and AI
  fields drive — answered by [[22]], [[67]], [[78]] / docs/engine/battle.md
  (the type bits, `Exdefense` at `+0x64`, `entryEnemy`); only type bits
  0x100 and 0x200 remain, below
- [[12]]: how `enemyList00`..`15` and `enemyOfs` pick rows — answered by
  [[13]], [[275]] / `crates/piney-battle/src/entry.rs` `register_list`
  (`tools/test_battle_spawn_rs.py`)
- [[12]]: the skill and item effect fields — answered by [[22]], [[67]] /
  docs/engine/battle.md, `crates/piney-battle/src/skill.rs`, `item.rs`
- [[12]]: the `msg` pointer in the character and boss rows — answered by
  [[208]] (an address of its message table)
- [[13]]: the race and drain expansion for type-64 entries — answered by
  [[78]]: 0x40 rows are middle bosses, their base and drained forms
  registered (`crates/piney-battle/src/entry.rs` `Register::register`;
  `tools/test_battle_spawn_rs.py`)
- [[13]]: what `ccGame.field` holds — answered by [[84]] /
  docs/engine/evarea.md (the story area number, `eventAreaNumber`; 0 for a
  generated area)
- [[13]]: how the three registered enemies become encounters (magic portals,
  `circleOfs`) — answered by [[78]], [[164]] / docs/engine/battle.md "Where
  the entries come from"
- [[13]]: how `itemOfs` picks treasure — answered by [[68]] /
  `crates/piney-fieldui/src/menus/getitem.rs` `area_item`
  (`tools/test_battle_flow_rs.py`, [[67]])
- [[22]]: the Data Drain drop roll and side-effect table, read not run —
  answered by [[102]] (`tools/test_battle_drain_rs.py`;
  `tools/test_fieldui_rs.py` `test_data_drain`), [[141]]
- [[22]]: `_ccSkillRequest`'s attribute-critical roll and area splash damage
  — answered by [[67]] (`tools/test_battle_rs.py`: `ccSkillDamage` single,
  area and point; `_ccSkillRequest`)
- [[22]]: the effects of sleep, paralysis, charm and confusion on behaviour
  — answered by [[67]], [[78]] / docs/engine/battle.md "Enemy AI", "Status
  effects"
- [[22]]: boss skills (`_ccBossSkillDamage`) — answered by [[109]] /
  docs/engine/boss.md
- [[22]]: item use — answered by [[67]], [[230]] /
  `crates/piney-battle/src/item.rs` (`tools/test_battle_items_rs.py`)
- [[22]]: enemy AI beyond its structure — answered by [[67]], [[78]]
  (`tools/test_battle_enemy_ai_rs.py`)
- [[22]]: what `noDeathFlag` means — answered by docs/engine/battle.md (a
  member with it: HP and SP only rise), [[192]]
- [[22]]: whether `plcol` is the bracelet — answered by
  docs/engine/battle.md (`data_drain` sets it with Kite's skill 2)
- [[22]]: whether volumes 2-4 changed the battle rules — answered by [[25]]
  (`Exdefense`, the area skill), [[221]]
- [[67]]: the run-time's animation and notes — answered by [[78]], [[139]],
  [[278]]
- [[67]]: enemy spawning and portals (`ccThEntryCtrl`, `entryEnemy`,
  `entryMagicCircle`) — answered by [[78]], [[80]], [[164]]
- [[67]]: enemy movement and each race's `action()` — answered by [[78]],
  [[168]], [[169]], [[171]]
- [[67]]: party movement (`FollowPlayer` to `ActInTown`) and
  `ccFellow::Action`/`Main` — answered by [[78]], [[110]]
- [[67]]: the spells' element systems — answered by [[79]], [[81]] (the five
  spell systems)
- [[67]]: the Data Drain movie and transformation — answered by [[105]],
  [[229]]
- [[67]]: damage numbers, hit marks and sounds — answered by [[79]], [[81]]
- [[67]]: the bosses: `_ccBossSkillDamage`, their affects and AI — answered
  by [[109]] (Skeith); [[266]], [[272]], [[274]] (Mutation's)
- [[67]]: `ccEnemyG::thinkGold` — answered by [[167]]
- [[67]]: the chat text — answered by [[123]]
- [[67]]: `ChangeEquipReport` in a field or dungeon — answered by [[123]]
  (`FieldWorld::equip_report`); the town's is in the menus' entry
- [[78]]: `piney-world`'s field and dungeon ran stand-ins with no entry
  control — answered by [[80]]
- [[78]]: `F_Note` records skipped by the animation reader — answered by
  [[138]], [[139]], [[278]]
- [[78]]: `ccThGameCtrl`'s `SetInBattle` — answered by [[80]]
  (`tools/test_gamectrl_rs.py`, in battle)
- [[78]]: the enemies' skills — answered by [[81]], [[83]]
- [[78]]: the weapon and dust effects — answered by [[118]], [[119]],
  [[170]], [[197]]
- [[78]]: the camera shake — answered by [[165]]
- [[78]]: the gimmicks' mains and `CloseDoor` — answered by [[87]], [[98]],
  [[116]], [[140]]
- [[78]]: the Data Drain movie — answered by [[105]]
- [[109]]: which `EVENTAREAB0` area event 30 loads; `SwitchLayer` and
  `DMY_center01` — answered by [[112]], [[113]]
- [[109]]: the boss entry (type 7) and present/absent 7 in the field's
  run-time — answered by [[113]] / docs/engine/boss.md
- [[109]]: `CMP_trall`, the afterimages, the cross's trail, `ccBossCam`, the
  stage fader, the reversed layer, the effects' pictures — answered by
  [[113]], [[115]], [[117]] / docs/engine/boss.md
  (`tools/test_bosscam_rs.py`)
- [[109]]: menu 74 (`StreamMenu`) — answered by [[113]]
- [[110]]: the members' chat lines and window; `ChatMessageEnteredTown` and
  `EnteredField` — answered by [[123]]
- [[149]]: row 18's `objMain` (the lakes' symbol) — answered by [[155]]
- [[168]], [[171]]: the fire breath (`ccEnemyBreath`) not drawn — answered
  by [[172]]
- [[168]]: the motion of L, W, D, E, T, A, 4 and S — answered by [[169]],
  [[171]]
- [[169]]: `ccEnemyL` (dragons and snakoids) stands still — answered by
  [[171]]
- [[169]]: the turtles' foot dust and the Cerberus's breath not drawn —
  answered by [[172]] (GAPS.md: drawn)
- [[221]]: Mutation's `ccAI::UseItem` changes (`CheckAction(7)`, held
  members, remarks) — answered by [[222]]
- [[221]]: Mutation's four new chat tables (tolerances, only one buff or
  debuff) — answered by [[222]], [[223]]
- [[221]]: the items, fellow, kite, party AI, enemy AI, spawn, event, chat
  and drain harnesses fail on Mutation — answered by [[222]], [[223]],
  [[224]]
- [[222]]: the callers of the other remarks (`ChatCommandExecute`,
  `BuffPlz`, `DeBuffPlz`, `FulfilCheck`) — answered by [[223]]
- [[222]]: the party AI harness's five failures on Mutation — answered by
  [[223]]
- [[223]]: the fellow, kite, event, navi and spawn harnesses not rerun on
  Mutation — answered by [[224]] (all five rerun there)
- [[229]]: no test drains an enemy on Mutation's disc — answered by [[266]],
  [[268]] (Innis drained)

**Still unknown:**
- (play) [[268]]: Innis's drains at its breaks gave item drops and no
  Epitaph in the survey, where `innis_drained_through_the_menus` reaches the
  Epitaph; possibly the queued-affect bug [[274]] fixed (a lost 21), not
  rechecked — Mutation's first boss fight may skip its Epitaph phase.
- (play) [[272]]: Kyvia's gomoras do not push each other; the pad's sway,
  the arm's picture and the particles are missing
  (docs/engine/boss-kyvia.md) — Mutation's Kyvia 01 fight.
- (play) [[274]]: Magus's shock beams are taken to meet the ground (no
  `ccHitCheckLM` on the land); `EntryFlash2` and `3` are one flash; the
  grow's looping SE plays once (docs/engine/boss-magus.md) — Mutation's
  Magus fight.
- (volumes) [[21]]: whether `ccRegisterDifficultyEnemy` matches on the other
  volumes; read on all four ([[275]]), never run in eemu there.
- (volumes) [[67]]: `piney-battle` is checked against Infection and Mutation
  ([[219]] to [[224]]); not against Outbreak and Quarantine.
- (volumes) [[278]]: how Outbreak's enemy tables changed row by row; only
  the offsets are compared.
- [[12]], [[22]]: what type bits 0x100 and 0x200 mean; set on spells, read
  by no rule (docs/engine/battle.md Unknown).
- [[22]]: that the protect break is the Data Drain window is still inferred
  (docs/engine/battle.md Unknown).
- [[22]]: normal (non-drain) drops: nothing found; `ccExpDistributor` gives
  experience only; not surveyed.
- [[12]]: whether the placeholder enemies can appear.
- [[95]], [[101]]: what bit 0 of a skill row's `+0x2c`
  (`ccGetSkillParam(sid) + 0x2c`) means.
- [[171]]: `ccEnemyL`'s type 3 (rows 203-206): what they are and whether any
  event places them; not ported, and no Δ list registers them (GAPS.md).
- [[105]]: the race's base form `ccThDrainEnemy` builds for a state 2 that
  no caller sets.
- [[275]]: what the community note's `enemyList00` refers to; the code gives
  story areas column 6.
- [[78]]: a dungeon way of 55 cells or more overwrites the beacon pointer,
  and unset destinations read stack garbage (game quirks;
  docs/engine/battle.md Unknown).
- [[78]]: the third swing is never reached, and `ctrlType` 1 and 2 are
  unused (inferred; docs/engine/battle.md Unknown).
- [[78]]: the equal-priority task order within `ccThSpc` (inferred).
- [[98]]: whether `ccThEntryCtrl`'s first slice runs in the frame of
  `ccEnableThEvent(4)` or the next (inferred from the task order).
- [[110]]: `ccThSpc` and `ccThAISystem` run before a leaver's task ends,
  where the game's ends at slot 50, after them (docs/engine/field-game.md
  Unknown).
- [[73]]: `CalcReal` skips a worn piece of -1 where the game reads the row
  before; no new-game member wears -1.
- [[123]]: `levelOld` from the game's heap when a member is built in town
  (docs/engine/battle.md Unknown).
- [[123]]: whether the stray `#` lines freeze a real console
  (docs/engine/battle.md Unknown).
- [[123]]: the raised lines are said after the task, not inside the hit, so
  their `rand()` draws come later.
- [[123]]: what sets `saveData +0x220d` (id 1's garbled lines)
  (docs/engine/battle.md Unknown).
- [[223]]: the game's windowed history walk never moves past a delivered
  item use that casts nothing; the port passes it over instead of hanging.
- [[223]]: `Ctx::item_first` reads a member without an AI as having no hit,
  where the game reads through a null pointer.
- [[223]]: who sends message kind 0xc and broadcasts 23 and 24.
- [[230]]: a member's item use is carried out after the AI's frame, one
  frame late for a reader in the same frame.
- [[230]]: CHAT's heal order out of battle plans once and drops it, as the
  code has it.
- [[192]]: whether a menu or script writes a party record in the same frame
  gap as the world's clears.
- [[158]]: the party under manual control and `ccClearConditionAllEnemy` at
  the game over are not run; nothing after the freeze reads them.
- [[167]]: `goldVolume` outside 1-4 leaves wear to a register; `checkGold`
  never makes one.
- [[167]]: a gold goblin's acts 2 (wander) and 4 (home) did not come up in
  the sample.
- [[168]]: how often `setBreath` and the scorpions' spin come up; counted
  roughly.
- [[171]]: the wyrms' and dragons' `setBreath` did not come up in a 300-case
  count.
- [[187]]: how many frames the game's weapon load takes before
  `EquipWeapon`.
- [[155]], [[162]]: `ccGimSymbol::objMain` is read from the code, not run in
  eemu against the port.
- [[79]]: the magic portal is ported twice (`piney-battle`'s `entry.rs` and
  `piney-effect`); both are checked.
- [[266]]: act 7 (the shake) on Innis itself is never reached.
- [[274]]: Magus's acts 30 and 31 and a leaf's 23 are never reached; whether
  any path reaches them.
- [[274]]: whether Skeith's and Innis's drains through the menus lost their
  21 before [[274]]'s fix.
- [[109]], [[117]]: the bosses' rules and effects are not checked together
  against the game; the effects' particles draw from the game's generators;
  `IceBreak`'s `Create` order and `ccRand` (docs/engine/boss.md "Not
  described yet").
- [[103]]: the fellow task's own frames (a dismissed leaver's exit and walk
  away) are not run against the game in a field; the rules are compared.
- [[75]]: idle animations under an event's manual control are not compared
  frame by frame (the game's rule, `ccFellow::Action`).
- [[238]]: the party AI's skills through the same request are not watched in
  play.
- [[67]]: `party_ai::Ai::new` has no check.
- [[167]]: a shaken-off hold is not played through.
- [[229]]: no test drains an area skill's sub-targets (Drain Arc, Drain
  Heart).
- [[230]]: no test drives a member's own item decision end to end.
- [[265]], [[268]]: whether a party at a player's level beats the Squidbod
  and breaks Innis's protect sooner; the pilot's party is the start's.
- [[273]], [[276]]: the lowest level at which the pilot beats Kyvia 01,
  whether a run without the level aid does, and a player's level and gear
  there.
- [[272]], [[273]]: why the Hackberry King in area 47's dungeon recovers
  under the pilot's fight, and whether the game's would as fast.
- [[279]]: which enemy and area the report's staff came from, and whether it
  was drained.
