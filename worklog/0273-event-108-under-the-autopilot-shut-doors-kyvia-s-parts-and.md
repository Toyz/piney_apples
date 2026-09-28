---
number: 273
title: Event 108 under the autopilot: shut doors, Kyvia's parts and its healing gomora
date: 2026-09-28
area: test, battle
files: crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/shrine.rs, crates/piney-game/src/session.rs, crates/piney-game/src/area.rs
---

# 273. Event 108 under the autopilot: shut doors, Kyvia's parts and its healing gomora

With Kyvia ported ([[272]]), `PINEY_SURVEY_ONLY=108 PINEY_SURVEY_GOD=1`
now ends `story 108: done` in 420,000 frames. It reaches field 9 at about
frame 188,000 and beats Kyvia by about 255,000. Four things were in the
way, all in the pilot. The fourth needed a harness aid.

## The shut doors rule was wrong

[[270]] had the dungeon walker fight every foe in a room while
`DungeonArea.door.door_anm` was false. That flag is `MoveDoor`'s
`clear_all`: no active object anywhere, not "this room's doors are shut".
In area 47's dungeon it held the walker on a Hackberry King whose HP comes
back (536, then 718 of 1410), and 108 never reached field 9.

Doors made shut by `SetDoor` stay shut until `doorFlag` turns true. The
rule now reads `!door.door_flag` (with doors in the room), and only while
the walk's goal is in another room.

## Kyvia's core and gomoras are foes

`Combat::enemies()` holds the field's foes and the boss body, but not a
boss's parts. Kyvia's core and its five gomoras are characters of their
own on the enemy list. So the pilot saw no foe on the disc, and Kite stood
still.

The pilot's `foes` now adds a boss's listed parts. `approach_part` walks
Kite to within 350 of one when no field foe is near, since the core rides
its spline round the disc.

## Which part, and which kind

The core at attribute 0 (row 32) has P_DEF 9990, P_EVA 9990 and M_DEF 2000,
at level 99. Row 33 is the reverse. A gomora of attribute 0 (a heal, every
270 frames) casts skill 295 on the core. Only damage of `DmgWait` (800)
within one window sticks. When the window ends, `ChangeAttribute` sets the
HP back to `DmgCount`, which is the window's starting HP, raised by any
heal (`CheckCountAtkTime`).

The pilot's `focus` is therefore the living attribute-0 gomora, then the
core. Kite's skill and the members' spells go to it through the target
menus (65 for Kite, 73 for a member). The pilot presses DOWN until the
world's `cmndTarget` is the focus; menu 73 moves `cmndTarget` as 65 does.
Magic or physical follows the focus's P_DEF against M_DEF, and the
members' Skills! or Magic! order is given again when that flips. A
member's spell at a core that magic hurts goes out even though the core
resists no element (every resist is 100).

## The party was too weak: a harness aid

At the start's levels (Kite 30, the members 26, starting gear) the party
did 10 to 70 a hit on the core. The healer undid it faster than the party
could deal it: the best window reached 839 once, and the core was back at
5000 within 3000 frames. The damage is the game's `CalcBattleDamage`, so
this is not a port error. A player would come with more levels and better
gear, and the pilot neither grinds nor shops.

- The console has a new `exp N`, which gives N experience to each member
  in the field. `CheckLevelUp` (`chara::check_level_up`, run from Kite's
  and the fellows' frames) turns every 1000 into a level, as a fight's
  experience does.
- Under god mode, `levels_for_boss` holds the party at 60 or above while a
  boss's parts are up (`BOSS_PARTS_LEVEL`).

At 60 the core falls in about 60,000 frames of fighting: 2429, 1594, 806,
then 434 over successive windows. Kyvia falls, the body exits, and block
23 runs.

**Still unknown:**
- The lowest level at which this pilot can beat Kyvia 01; 60 was the first
  tried. What a player's level and gear at this point are was not measured.
- The Hackberry King in area 47's dungeon takes the pilot about 170,000
  frames; whether its HP should come back as fast as it does was not
  checked against the game.
