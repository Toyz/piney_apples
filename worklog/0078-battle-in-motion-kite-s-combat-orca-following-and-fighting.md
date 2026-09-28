---
number: 78
title: Battle in motion: Kite's combat, Orca following and fighting, path finding, the magic portals and the first field's enemies
date: 2026-09-24
area: battle, test
files: crates/piney-battle/src/world.rs, crates/piney-battle/src/geom.rs, crates/piney-battle/src/frame.rs, crates/piney-battle/src/kite.rs, crates/piney-battle/src/fellow.rs, crates/piney-battle/src/follow.rs, crates/piney-battle/src/navi.rs, crates/piney-battle/src/ai_move.rs, crates/piney-battle/src/party_motion.rs, crates/piney-battle/src/entry.rs, crates/piney-battle/src/races.rs, crates/piney-battle/src/enemy_motion.rs, tools/test_battle_frame_rs.py, tools/test_battle_kite_rs.py, tools/test_battle_fellow_rs.py, tools/test_battle_navi_rs.py, tools/test_battle_spawn_rs.py, tools/test_battle_enemy_motion_rs.py, docs/engine/battle.md
---

# 78. Battle in motion: Kite's combat, Orca following and fighting, path finding, the magic portals and the first field's enemies

[[67]] ported combat's rules and decisions. It said the runtime still had
to move everyone. A lead agent and five helpers have ported that motion
layer into piney-battle, behind a small trait the world implements. It
covers everything field 14 and its dungeon need.

## What moves now

- **Kite.** `ccPlayer::Main`, `ControlMove` and `AnimCtrl` in full: every
  act, the combo, and the notes. Also `Attack`, `AttackCancel`,
  `BreakSomething` and `DamageActuate`. It is a superset of piney-world's
  motion.rs, so the world can call it.
- **A party member.** `ccFellow::Main`, `Move`, `Action` and `CheckNote`,
  `ccSpcChar::HitCheck`, and the ways to follow:
  - `FollowPlayer` and `LeavePlayer`;
  - `FollowTarget` and `FollowTargetDirc`;
  - the obstacle checks.
- **The AI's movement.** The whole dungeon path finding,
  `SetPathFindingMap`, and the town's landmarks. Also `MoveP2P`,
  `FollowBeacon`, `ManualControl` and `ActInTown`, performed for
  [[67]]'s party AI by one runtime (`party_motion::Movement`).
- **The party's thread.** `ccThSpc`: the party's strategy, Kite's shout,
  and `spcBattleCondition`.
- **The entry control.**
  - `ccThEntryCtrl`, `ccEntryObj::routine`, `entryObject`, `entryEnemy`,
    `initObject`;
  - the magic portal (`ccMagicCircle` and its particles);
  - where entries come from: `EntryGimmick`, `SetMagicCircle` for fields
    and dungeons, and the events' `entry_mc`. Event 3 makes four
    portals of goblins around Kite; event 4 makes one on floor 0.
- **The enemies.** `moveEnemy`, `animEnemy`, the note handler, `dispEnemy`'s
  decisions, the six act helpers, and each race's action. The races are
  B, G, K, P, V and 1, which make the field's and dungeon's seven enemies:
  - the field: Goblin 130, Mad Grass 219 and Disco Knife 185;
  - the dungeon: Deadly Moth 268, Magical Goblin 151, Chicken Hand 67 and
    Sword of Chaos 189.

## Bugs found on the way

- **Self-hits.** When a character hits itself, the SP drain now reads HP
  after the HP drain, as the game does.
- **Affect timing.** Each `EntryAffect` is applied when the game makes it.
  The native checks run the real affect functions, and they caught the
  deferred version.
- **`CharHit`'s default.** `mask2` is -1, as the game's constructor sets
  it.

## The name of field 14

The agent found that story area 14 is "Bursting Passed Over Aqua Field";
Hidden Forbidden Holy Ground is 15. The disassembler's comments
(`tools/evscript.py`) had indexed the story-area list by position.
That list is in `eventAreaInfo`'s order, which skips 13, so every name
after that point was off by one. So [[69]] and [[72]] said Hidden
Forbidden Holy Ground. That is fixed, and those entries corrected.

## Checked

- **Against the game.** Six new harnesses:
  - frame (1 check);
  - kite (10);
  - fellow (18);
  - navi (23);
  - spawn (20);
  - enemy motion (11).

  With [[67]]'s six, all 208 checks ran 1,000 cases each with 0
  mismatches. Each compares every field, both generators and the ordered
  call log. The game's own functions run on the Rust eemu.
- **Long runs.** Each new harness also plays runs of 60-300 frames,
  compared after every frame. Composition runs put pieces together on the
  game's side too:
  - the entry control running the real `ccEnemy::main`, with portals,
    chases, attacks, deaths and drains;
  - `ccFellow::Main` with the real path finding;
  - Orca's `ActInTown` in Mac Anu;
  - `ccAI::Brains` in a dungeon room;
  - Kite driven by his AI.
- **Re-run in a clean worktree.** All 13 battle suites pass (the unit
  modes), with the workspace's tests, clippy, fmt and the docs check.

**Still unknown:**
- **Not in the runtime yet.** piney-world's field and dungeon still run
  their own player and stand-in fellows, with no entry control.
  battle.md's "How it plugs into piney-world" lists the wiring: the
  scene, Kite's and the fellows' mains, the entry control's place in the
  frame, the hit queries, and the map's portals.
- **Animation notes.** piney-data's animation reader skips the `F_Note`
  records the world's `anim_notes` must supply.
- **`ccGame.inBattle`.** `ccThGameCtrl`'s `SetInBattle` is not ported.
- **Left to the runtime:**
  - the enemies' skills;
  - the weapon and dust effects;
  - the camera shake;
  - gimmick mains;
  - `CloseDoor`;
  - the Data Drain movie.
- **Game quirks kept:**
  - a dungeon way of 55 cells or more overwrites the game's own beacon
    pointer (the checks stop at 54);
  - destinations the game never sets read stack garbage (the port uses 0).
- **Inferred, not measured.**
  - The third swing is never reached.
  - `ctrlType` 1 and 2 are unused.
  - The equal-priority task order within `ccThSpc`.
