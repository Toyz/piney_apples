---
number: 80
title: The first fights: event 3's goblins, the portals and free play in field 14, Kite and Orca against the game's enemies
date: 2026-09-24
area: battle, world, script, test
files: crates/piney-world/src/combat, crates/piney-world/src/foe.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/talk.rs, crates/piney-world/src/map/mod.rs, crates/piney-battle/src/evparty.rs, crates/piney-battle/src/flow.rs, crates/piney-data/src/anim.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/area.rs, crates/piney-game/src/area_host.rs, crates/piney-game/src/session.rs, tools/test_battle_event_rs.py, tools/test_gamectrl_rs.py, tools/test_foe_rs.py, tools/anim.py, docs/engine/battle.md, docs/engine/field-walk.md
---

# 80. The first fights: event 3's goblins, the portals and free play in field 14, Kite and Orca against the game's enemies

The parts were ported and checked separately:
- the rules and decisions ([[67]]);
- the motion ([[78]]);
- the effects ([[79]]);
- the battle menus ([[64]]).

A combat agent and its helpers have now put them together in the field.
A new game plays events 2 and 3, with event 3's fight, and then free
play.

## What plays

- **Event 3's tutorial.** The portal opens on a goblin held by `hold`, and
  `player_skill`'s X hits it until it is down. The skill, target and chat
  tutorials (menus 80-83) complete, and the event ends with `menu_clear`.
- **Free play.**
  - Walk to a portal and a goblin comes out.
  - `inBattle` turns on and the battle music plays.
  - Orca fights.
  - X on the targeted enemy is Kite's normal attack.
  - The kill gives experience.
- **What is drawn.** The enemies with their looks, fog blend and hit
  flash (foe.rs). Through piney-effect:
  - the portals, sparks and hit marks;
  - the rising damage numbers and the ATTACK and CRITICAL words;
  - the deaths and the arrival.

  The HUD's target window shows the goblin's name and HP. The minimap
  shows the portals.

## How it is built

- **The frame.** piney-world's new combat module keeps piney-battle's
  scene in the field and dungeon worlds. It runs the game's frame order:
  1. ccThSpc;
  2. the AI bus;
  3. `kite::main`;
  4. the fellows;
  5. the entry control;
  6. `ccThEffect`;
  7. `ccThSkill`;
  8. `ccThParticle`.
- **The portal.** piney-battle's entry control is the portal's logic. It
  spawns the goblins, and piney-effect's `MagicCircle::render` draws them.
  So the portal, ported twice, now has one owner for each part.
- **The battle half of `ccThGameCtrl`** is `talk::Targeting::frame`:
  `SetInBattle` and the attack button.
- **The event instructions** the fights use are in piney-battle's
  evparty.rs:
  - `pc_command`, the walks, the puts, and turns and faces;
  - `hold` / `ccThEvHold`, `player_skill` and `battle_ready`;
  - `remove` of types 5 and 6, and `enemy_put`.
- **Animation notes.** piney-data's animation reader now reads the
  `F_Note` records that attacks land on, checked against the game.

## Checked

- **New harnesses against the game.**
  - `test_battle_event_rs`: the event instructions.
  - `test_gamectrl_rs`: the battle half of `ccThGameCtrl`.
  - `test_foe_rs`: the enemies' drawing decisions.
- **Earlier suites.** The animation notes (`test_anim`) and the suites
  these files touch pass (the earlier battle suites at 1,000 cases each):
  - the battle suites;
  - world, field, dungeon and map;
  - the field UI;
  - the event characters;
  - effects.
- **Runtime test.** `event_3_plays_its_fights` plays a new game through
  events 2 and 3's fights in about 4 seconds. This checks the port's own
  behaviour, not against the game.
- **Re-run in a clean worktree.** All of the above pass, with the
  workspace's tests, clippy, fmt and the docs check.
- **Shots.**
  - A portal opening.
  - A targeted goblin at 50/50.
  - Kite's blow: 12 damage, the ATTACK mark, the goblin flashing red at
    38/50.
  - Orca fighting beside him.
  - The goblin down.

**Still unknown:**
- **Still being ported:**
  - the dungeon side: entering field 14's dungeon and event 4;
  - the fight's sounds (`ccSeOn3D` and the enemy and party sound
    parameters).
- **Not done:**
  - the AI's chat lines;
  - spells through piney-effect;
  - Data Drain's presentation;
  - game over past its signal;
  - `boss()` for the events.
