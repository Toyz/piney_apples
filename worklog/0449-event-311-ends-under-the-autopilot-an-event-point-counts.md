---
number: 449
title: Event 311 ends under the autopilot: an event point counts only in its own dungeon, the members' Sprite Ocarina leaves a dungeon the story is done with, and a lake by its Gate Out; area 108's way out is field 12 from Mutation on, so 312 reaches Kyvia's fourth fight
date: 2026-10-24
area: world, test, volumes
files: crates/piney-data/src/dungeon/special.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/survey.rs, crates/piney-game/src/session/tests/arena.rs, crates/piney-game/src/session/tests/kyvia_fourth.rs, crates/piney-game/src/session/tests/side_events.rs, docs/engine/dungeon.md, plans/quarantine-story.md
started: 2026-10-24T05:33:59-06:00
took: 1h 53m
---

# 449. Event 311 ends under the autopilot: an event point counts only in its own dungeon, the members' Sprite Ocarina leaves a dungeon the story is done with, and a lake by its Gate Out; area 108's way out is field 12 from Mutation on, so 312 reaches Kyvia's fourth fight

Quarantine's events 311 and 312 had stopped in their dungeons in the god
survey (`PINEY_SURVEY_GOD`). Event 311 stood in field 103's dungeon with
the first of four goal rooms done. Event 312 stood in field 108's dungeon.

## 311: four dungeons, one point number

Event 311 sends Kite to four dungeons, 103 to 106. Each has a boss smoke
at its point 1 (blocks 8-19).

The story's wants listed `Point(1)` with no dungeon. In dungeon 103,
after its core monster fell, the pilot took dungeon 104's `Point(1)` to
be this dungeon's point 1. It walked back there and stood still.

A point want now carries the dungeon its block is set in:
`Want::Point(num, Some((field, dungeon)))`, from the block's
`in_dungeon` or `scene`. A point counts only in that dungeon, for the
walk, the statue and staying.

Leaving was the next gap. A dungeon has no Gate Out. A player leaves one
by CHAT's eighth order, which the menu offers only in a dungeon whose
floors are not a lake's: a member plays his Sprite Ocarina. Members keep
it in stock (`SetSpcItemTown` never sells it). `gate_out` now gives that
order in a dungeon the story wants nothing of, and waits through the
ocarina's menu 86 (`GoField`).

Dungeon 103 is below a lake (field type 4, dungeon 1), so the ocarina
goes back up to the lake. The lake has the field's PERSONAL menu, with
Gate Out (docs/engine/dungeon.md), so the pilot gates out there as from
a field.

A dungeon whose way out leads to a wanted field (an arena) counts as
wanted. The ocarina never takes the pilot away from an arena door.

With these, 311 ends under god within the survey's 300,000 frames.

## 312: area 108's way out

Event 312's block 10 wants field 12, Kyvia's fourth disc. The way there
is area 108's dungeon: its door into the room of type 16 or more,
`GotoNextRoom`'s -255, `WORLD_MAN::Enter`'s `ChangeArea(1, n)`.

`special::exit_field` had area 108 go to field 9, Infection's
destination. The `Enter` of each volume, read for its `li $v1, 108`
branch, gives:
- Infection 9;
- Mutation, Outbreak and Quarantine 12 (QUA main 0x001b07d4).

The pilot's `arena_door` looked for field 9, which no want named, so it
never walked there. The dungeon's own exit would have taken a player to
field 9 too. It now gives 12 from Mutation on, with a unit test.

312 now reaches the disc and the fight there under the pilot:
- blocks 0xe09;
- Kite walks to each place the body waits at (`kyvia_place`:
  `CoreAppearPos[CoreLvCount]` while `StFlg` is 1, as a player goes on
  past the opened gate);
- the healing gomoras are fought first.

It does not end. The cores wear down some 25,000 frames each at the
party's 78-81. At the fourth place the gomoras' heals (skill 295, 200 a
cast) outpace the party's blows: the core went from 7000 down to 4960
and back over 100,000 frames. `kyvia_fourth_under_the_pilot` (ignored)
plays this from the disc. The aided `kyvia_s_fourth_fight_to_its_end`
still plays the fight to its end.

Tarvos (code 6) joins the roaming bosses the pilot walks to.
`arena::EVENT_312` puts a party on the disc for tests.

**Still unknown:** how the pilot wears the fourth place's core faster
than its gomoras heal it (a player's party, skills or order), for the
no-aid runs.
