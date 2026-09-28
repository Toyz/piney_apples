---
number: 264
title: Area 13's map, the pilot's talks in fields, and Mutation's bosses
date: 2026-09-28
area: world, test, battle, volumes
files: crates/piney-world/src/evarea01.rs, crates/piney-world/src/evarea03.rs, crates/piney-world/src/town.rs, crates/piney-world/src/evarea.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs
---

# 264. Area 13's map, the pilot's talks in fields, and Mutation's bosses

This follows [[263]]. Event 115 goes through field 13, and event 107's arena
fight needed a look. Both lead to the same finding: Mutation's story
fights three bosses the port has not got.

## Area 13: `EVENTAREA01`

Field 13 is `EVENTAREA01` (MUT gcmn 0x00414110 constructor, 0x00414c00
`Draw`). Its parts:

- **The scene.** `se1_1`: `EA_MODELTABLE01`'s seven rows (a floor,
  five type-2 pieces, one type-3), `EA_OBJTABLE01`'s one animated object,
  and `bg[0]` `CMP_se1_1bg1` with fog off.
- **The lights.** The light animation `ANM_se1_1bac1a` and
  `eventarea01Light`'s eleven rows: one distant light, then ten omni.
- **The rest.** Four BLT groups. Fog (150, 4000, 0, 80, 0x1e1e1e). The
  start is `DMY_marker_ev01`, its w the dummy's turn about z.
- **Seven glows.** `EFF_se1_1ef1`, `Init` with fog, then PRIM's fog
  bit cleared, one at each of `DMY_effpoint01`-`07`. For each, the
  constructor draws a frame, a wait (30-129) and a flicker count (1-15)
  from `fieldrand`.

`Draw` is:
1. `DrawBG`.
2. On `objLayer`: `DrawObj2` (type 3), `DrawObj` (type 2), `DrawFloor`
   (type 1) and the type-0 rows.
3. On `effLayer`: each glow.

A waiting glow counts down and draws its animation's next frame. When its
wait runs out it flickers: a random frame for `count` frames, then a new
wait and count.

The class is built like a town, so the port builds it on `town::Base`. A
new `Base::read_volume` reads the volume's own tables; `Base::read` is
Infection's as before. `StorySprite` gains `fog`, so these glows draw
without fog but keep the depth test (`flare` drops both).
`evarea01::tests::area_13` checks the pieces, the lights, the collision,
the glows' first frames and the draw.

## A bug in area 43's map

`Area43`'s draw filtered `DrawPass::Obj` for its "pass-0" models. Pass 0
is `DrawPass::Other`; `Obj` is type 2. The map drew nothing. `area_43`
now also checks that the draw puts something out.

## The pilot

- **Talks in a field.** Event 115's field 13 waits for six `talked_to`s.
  Each is a repeatable block that sets a status, and block 21 wants all six.
  The `add_target`s stay on the list after a talk. So the pilot walks
  (`path_to` over the place's collision) to each event NPC not yet spoken
  to here, presses OK once he is the command target, and counts the talk
  once the event's `menu_ban` shows. `FieldWorld` gains
  `event_targets`, `command_target_code` and `place_hits`.
- **Board posts rank below the story's wants.** In event 108 the pilot
  logged in to Dun Loireag, logged out for an unread post, found the
  board held on the top page (operate 7), and logged in again.
  `ccStartThEvent` resets the event manager in town, so the hold does not
  show there. Mail still comes first (events wait on `mail_got`); posts
  now come after the wants.
- **Data Drain.** A boss whose protect is broken (`pp_count` frames
  left) is drained: Skills, page 5, skill 2.
- **`mutation_whole_story`** runs a new game to event 116's end and
  prints each story event's end as it comes.

## The bosses

In event 107's arena the foe list stayed empty: the port starts only
Skeith. `ccBossEntryStart(code)` (MUT gcmn 0x00470c40) starts
`ccThBossEffect` and `bossFunc[code]` (MUT 0x006192c0, 16 rows):

```text
0 Boss01 (Skeith)   1 Boss02 (Innis)   2 Boss03 (Magus)   3 Boss04
4 Boss05            5 Boss06           6 Boss07           7 Boss08
8-11 none           12-15 Kyvia01-04
```

Mutation's story enters three of them with `entry` type 7:

| event | code | boss | where |
| --- | --- | --- | --- |
| 107 | 1 | `ccBoss02` Innis, with `ccBoss02Slave` | field 2, `EVENTAREAB0` |
| 108 | 12 | `ccThKyvia01` and the `kyvia*` classes | field 9, `EVENTAREAB8` |
| 115 | 2 | `ccBoss03` Magus, with `ccBoss03Leaf` | field 3, `EVENTAREAB0` |

Their size on Mutation:
- `ccBoss02`: 54 functions, 38,616 bytes.
- `ccBoss03`: 70 functions, 38,248 bytes.
- The `kyvia` classes: 98 functions, 74,448 bytes, shared by the four
  Kyvia fights.

Skeith's `ccBoss01` is 26 functions and 15,064 bytes. `EVENTAREAB8`
(fields 9-12: 40 `FLOATROCK`s, 25 clouds, a disc that carries the party
under `WORLD_MAN::SetTransMode`) is Kyvia's stage and goes with that port.

**Still unknown:**
- Innis, Kyvia 01 with `EVENTAREAB8`, and Magus are to port. Until then
  events 107, 108 and 115 stop at their bosses.
- Whether `EVENTAREA01`'s `DrawObj` and `DrawObj2` layer differs from
  `objLayer`: they draw inside their BLT groups with the active layer
  that `Draw` set (1).
