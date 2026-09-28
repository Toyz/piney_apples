---
number: 265
title: Mutation's dungeons on Mutation's tables, and the pilot's way past side lines
date: 2026-09-28
area: world, test, volumes
files: crates/piney-world/src/dungeon_area.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/shrine.rs
resolves: 263
---

# 265. Mutation's dungeons on Mutation's tables, and the pilot's way past side lines

The dungeons played every volume on Infection's tables.
`DungeonArea::new_banned` took `dungeon::INF` whatever the disc, though
the tables differ by volume. `edit` (the story dungeons' rooms), the floor
counts and `types.save_flag` all change, and so does `volumeNum`, which
`special::exit_field` reads. Area 47's arena door gives field 9 on volume 2
and field 10 otherwise. So event 108 on Mutation left area 47's dungeon
into field 10, and its next block waits in field 9. The dungeon now takes
the field's volume (`dungeon::tables_of`); `DungeonArea::new`, which the
probes and Infection's tests use, stays Infection's. Event 108 then reaches
field 9 and its boss entry (Kyvia, [[264]]).

## Which other tables differ by volume

[[263]] asked whether the towns' Infection tables matter on Mutation.
Comparing the build's placement groups, Infection's against Mutation's:

| group | bytes | differ |
| --- | ---: | --- |
| `field` | 30,759 | 1 byte, the volume |
| `statics` | 9,649 | 81 bytes, each a table's own address |
| `dungeon` | 289,416 / 301,966 | the contents |
| `area` | 32,323 / 32,602 | the contents |
| `sound` | 18,875 / 19,441 | the contents |

The field and statics rows are the same on both volumes, so the towns'
`statics::tables()` and the fields' `field::INF` are right on Mutation.
The area and sound groups are already read by volume, and the dungeon's
now is too. Outbreak's and Quarantine's are still to compare.

## The pilot

- **Side lines in town.** Event 108's six `add_target` NPCs in Carmina
  Gadelica each have a repeatable line, and the targets stay on the list.
  The town pilot went back to the first one forever. It now remembers whom
  it spoke to in the town (once the talk's event bans the menus) and
  passes them by. `gate_goal` and `story_player` take the list
  (`story_player_with`).
- **A foe that loses no HP.** In event 115's field 52 dungeon a level-49
  Squidbod held 2010/2010 against a level-35 party for 20,000 frames. The
  wary walker fought it because it stood within 600, and `choose` attacked
  it. A foe fought for 1,800 frames without losing HP is now walked past
  (`Walker::hopeless`), as a player would.
- **Data Drain's menu** (66) takes OK: its movie, messages and drops wait
  on `check`, and the pilot used to back out with cancel.
- **The story moves to the top page.** After event 107's town scene, its
  next blocks are set on `game_status` 3: Log in and the board are taken
  (operate 6, 7), and Quit (8) ends the event. Such a block now gives
  `Want::Leave`, which logs out when nothing else is wanted. On the top page,
  with Log in and the board both taken, the pilot quits.
- **Out of a field the story is done with.** The first run from a new game
  (`mutation_whole_story`) ended 101-104 in one session, then stood in
  field 58 for 478,000 frames: event 104 ends there, and 105 opens on the
  board. The pilot now goes out (PERSONAL, Gate Out, menu 10, YES) when
  none of the story's wants names the field and nothing is going on for
  300 frames: no event NPC waiting, no fight, no event playing.
- **God gets the fallen up.** The console's `god` kept HP full only for
  the living, and a member who fell stayed down for the rest of the area.
  It now revives him with affect 20, the game's revive (condition 5 and
  its 78 frames getting up), and then his HP. This is the console's, not
  the game's; `heal` still leaves the dead down.

## Comments

piney-game's comment blocks are all 8 lines or fewer now. The story
starts' long form moved to docs/engine/event-vm.md ("Starting later in
the story"), with Mutation's starts and what `start::fights` adds.

**Still unknown:**
- Outbreak's and Quarantine's dungeon, area and sound tables are not yet
  compared with Mutation's.
- Whether the Squidbod could be beaten by a party of the level a player
  brings: the pilot's party is the start's, not a player's.
