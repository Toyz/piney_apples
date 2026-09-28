---
number: 11
title: The Chaos Gate keyword system, reproduced and checked against the game code
date: 2026-09-22
area: world, decomp, tooling, test
files: tools/areas.py, tools/test_areas.py, tools/eemu.py, docs/engine/area-words.md
---

# 11. The Chaos Gate keyword system, reproduced and checked against the game code

.hack's field areas are addressed by three keywords typed at a Chaos Gate -
"Bursting Passed Over Aqua Field". The whole generator is
`WORLD_MAN::SimGenerateCode` (`INF SLUS_202.67:0x0019e5c0`, `world_man.cpp`
lines 1464-1581), and `tools/areas.py gen` now reproduces it. The
reproduction was checked by running the game's own function in
`tools/eemu.py` over 412 word triples and comparing every field it writes:
**0 mismatches**. The algorithm is on
[the area-words page](../docs/engine/area-words.md).

## The words

Twelve `WORDPARAM` tables in main, `word_a1` .. `word_c4` (`0x00311790` -
`0x003150c0`): the letter is the word's slot in the address (first, second,
third), the digit a group within the slot. 305 words in all - slot a 100,
slot b 103, slot c 102. `WORDPARAM` (0x30 bytes) is `char *word`, `int ID`,
`int pri`, then nine attributes: `fieldType`, `dungeonSize`, `weather`,
`ground`, `object`, `areaLevel`, `enemyOfs`, `itemOfs`, `circleOfs`. An
attribute of 255 (0 for `dungeonSize`) means the word has no say in it. Base
priority falls table by table - `a1` 900, `b1` 850, `c1` 800, ... `c4` 350 -
with a few words set higher (`Chronicling`, ID 131, is 999).
`WORLD_MAN::GetWordParamPtr(slot, id)` (`0x001a2340`) looks a word up by ID in
the four groups of one slot.

## The generator

1. `seed = A * 1,000,000 + B * 1,000 + C` from the three word IDs. That number
   is also the area's code, stored back into `wordparam.ID`.
2. The RNG is inline: `seed = (seed * 109 + 1021) mod 0xfffffffe`, with a
   32-bit multiply, bumping the global `randcnt` each time. Four draws give
   `fieldSeed` and `dungeonSeed[3]` (`WORLD_MAN+0x44` .. `+0x50`).
3. The three words are insertion-sorted by `pri`, ascending.
4. Nine more draws give random defaults: `fieldType = r % 11`,
   `weather = r % 10` (clamped by field type), `dungeonSize = r % 10 + 1`,
   `ground = r % 3`, `object = r % 3`, `areaLevel = r % 5 + 1`,
   `enemyOfs = r % 20 - 9`, `itemOfs = r % 20 - 9`, `circleOfs = r % 3`.
5. `CopyWordParam` (`0x0019e500`) merges lowest-priority word <- middle <-
   highest, each overwriting only the fields it sets, and the result is merged
   over the random defaults. So every attribute comes from the
   highest-priority word that has an opinion, and the rest from the dice.
6. `ccCheckEventAreaNum` (`0x001a4290`) looks for a fixed story area with
   exactly these three words on the current server (`ccGame.server`,
   `game+0x1c`). If one matches, its record overrides circle, enemy and item
   offsets, field type and weather where it gives them. Story area 71 sets
   enemy and item offsets to 119 when a save flag (`saveData+0x5bc0`, bit 62)
   is set; area 47 sets them to 78 and 122 once `volumeNum >= 3`.
7. Weather is clamped again by field type, `dungeonSize` 11 or more becomes 1,
   and `dungeonData[dungeonSize]` gives the dungeon's level and room counts.
   Word 131 as the first word sets `WORLD_MAN.timeSym`.

The weather clamp is two identical jump tables (`0x003555f0`, `0x003555c0`):
field types 0-4 keep weather 0-3, types 7-10 keep 0-7, types 5-6 map 8 to 4
and 9 to 5 and keep 0-3; anything outside the kept range becomes 0.

## The story areas

`eventAreaInfo` (`0x00315120`) holds 126 `EVENTAREA_INFO` records (0x54
bytes: code, three word strings, server, circle, enemy, item, model, flag,
type, weather, dungeon count, eight protect words). Records 0-12 have no
words. The other 113 are per server; the server numbers were matched against
every server-prefixed address in the English mail and board text:

| server | letter | story areas | addresses in the text that agree |
| ---: | --- | ---: | ---: |
| 0 | Δ Delta | 22 | 15 |
| 1 | Θ Theta | 12 | 10 |
| 2 | Λ Lambda | 28 | 19 |
| 3 | Σ Sigma | 34 | 17 |
| 4 | Ω Omega | 17 | 11 |

Two addresses in the text disagree with the table: a mail sends the player
to `Σ Pulsating Sea of Cloud Whale`, which the table has only on Λ (area 91),
and a board post names `Σ Obedient Someone's Knights`, only on Ω (area 122).
One story area, 94 on Σ (`Howling Hot-blooded 500 Lohan`), uses a first word
that is not in any slot-a table - "Howling" occurs once in the whole
executable, in that record - so it cannot be typed at the gate in this
volume. That the table spans all five servers and the mail mentions all five
is more evidence that Infection carries data for the later volumes.

## Checking it against the game

The check runs the real `SimGenerateCode` in `tools/eemu.py` with a scratch
`WORLD_MAN`, a scratch `ccGame` holding the server, and the globals
`worldman`, `game` and `saveData` pointed at them, then compares every
`WORDPARAM` field, the event number, `fieldSeed`, `levelMax`, `roomMax`,
`timeSym` and the copies into `WORLD_MAN` with `areas.generate()`. 112
reachable story areas plus 300 random triples over servers 0-4: 0 mismatches.
The first attempt reported 412 mismatches of 412, every one a value equal to
the neighbouring field - the harness read the struct 4 bytes early, not a
generator bug. `tools/test_areas.py` holds the check (every story area and 60
random triples, 3.5 s).

Two things had to be added to `eemu` for it: the EE's second multiply
pipeline (`mult1`, `div1`, `mfhi1`...; the compiler interleaves RNG steps
across both pipelines), and Python implementations of `strcmp`, `memcpy`,
`strcpy`, `memset` and the rest, because the game's C library is
hand-vectorised with MMI (`strcmp` uses `pcpyld`) and the interpreter does
not do MMI.

**Still unknown:** what each value of `fieldType`, `weather`, `ground`,
`object` and `circleOfs` looks like in the game; how `enemyOfs`/`itemOfs`
feed the enemy and item tables; what `EVENTAREA_INFO.flag`, `model` and
`protect` do; what `SetDungeonTypeFromField` decides; which story areas
Infection itself can reach; the save flag behind area 71.
