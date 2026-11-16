---
number: 604
title: The enemy-list overrun: the port registered enemies in a Root Town; ccRegisterEventMng's enemy rows and the lists' memory ported
date: 2026-11-15
area: battle, decomp, volumes, test
files: crates/piney-battle/src/entry.rs, crates/piney-gen/src/manifest.rs, crates/piney-world/src/world.rs, crates/piney-world/src/combat/cast.rs, crates/piney-fieldui/src/words.rs, tools/test_battle_spawn_rs.py
started: 2026-11-15T12:08:01-07:00
took: 7h 17m
---

# 604. The enemy-list overrun: the port registered enemies in a Root Town; ccRegisterEventMng's enemy rows and the lists' memory ported

`infection_whole_story` panicked after event 20 (frame 111,600) at
`list_rows` (crates/piney-battle/src/entry.rs), "len 130, index 144". The
rank did not come from a place the game reaches: a trace of
`Combat::start_entries` showed the call made in a Root Town (`game.area`
0, field -1), type 3, rank 144, with `WORLD_MAN` still holding the last
area's merged words. The game never registers there:
`ccRegisterDifficultyEnemy` returns at once unless `game.area` is 1 or 2
(INF SLUS_202.67:0x001b7038-0x001b7058; MUT 0x001cc75c, OUT 0x001c2e0c,
QUA 0x001ca48c). The port called `register_list` unconditionally, with
its own `rank.max(0)`, `server.clamp(0, 4)` and `ty.clamp(0, 6)`.

## The registration, from the code

`ccRegisterEventMng(int)` (INF main 0x001b6d70, MUT 0x001cc480, OUT
0x001c2b30, QUA 0x001ca1b0) is what registers an area's enemy rows; every
file list calls it (`ccSetFileListTown`, `Field`, `Dungeon` with 0 at INF
main 0x001696d0, 0x001697e0, 0x001698f0; the `Del` variants with 1, which
stop after `ccRegisterRegularGimmick`). In order:

- `ccInitRegisterAll` (INF gcmn 0x0042e720: `ccInitRegisterEnemy`);
- from Mutation on, `game.field == 126` returns here (MUT 0x001cc4a4, OUT
  0x001c2b54, QUA 0x001ca1d4): field 126 is the record with no address
  the later tables add, Quarantine's ending's empty map (event 314,
  docs/engine/evarea.md); Infection has no such test;
- a 14-entry jump table on `game.field` (`sltiu 14`; INF 0x003562c0, MUT
  0x00370370, OUT 0x00369d20, QUA 0x0025d2f0): field 0 and anything
  outside 0-13 (negative included, unsigned) run the next step, fields
  1-13 skip it;
- unless an `entry[k]` (eventMng +0x80, 16 x 8 bytes) has a type with bit
  0x40 (`andi 0x40` on the raw type, -1 skipped),
  `ccRegisterDifficultyEnemy`;
- `ccRegisterRegularGimmick`, `ccRegisterRegularNpc`;
- each `entry[k]` by `1 << type` (`sllv`, so the type's low five bits):
  `& 7` `ccSPC::EntrySpc`, `& 0x60` `ccRegisterEnemyOne(code)`, `& 0x80`
  `ccSetFileListBoss`, `& 0x18` `ccRegisterNpc`;
- each `entryMc[k]` (+0x100): type 0 `ccRegisterEnemyOne(code)`, type 1
  `ccRegisterGimmick`; then `ccRegisterRandomNpc`.

`ccRegisterDifficultyEnemy` (INF main 0x001b7020, MUT 0x001cc740, OUT
0x001c2df0, QUA 0x001ca470): only in a field or dungeon. A story area
(`game.field` > 0) takes type 6 and `GetEventAreaInfo()->enemy` (+0x18),
255 meaning `25 (areaLevel - 1) + 10 + enemyOfs`, and holds it at 0
(`bgez`); elsewhere the type is `GetFieldAttrb()` and the rank the same
sum, not held. In a dungeon `max(game.floor + 1, 0)` is added; from
Mutation on not when `WORLD_MAN::GetDungeonType()` (`dungeonType[
game.dungeon]`, MUT 0x001b71c0) is 8 or 9. It stores type, server and
rank at `eventMng` +0x790, +0x794, +0x798; nothing reads them (a scan
of INF main, gcmn, desktop, toppage and demo, and of the later volumes'
main and gcmn, finds only these three stores at those offsets each).
The port held the rank at 0 everywhere and added the floor in dungeon
types 8 and 9 too.

## How far a list is read

`ccRegisterEnemyList` (INF gcmn 0x0042ed70) forms `list + (rank << 2)`
and loads with no check; after each row it bumps the pointer and the
rank and steps both back only when the rank equals `num`. A start at
`num` or beyond therefore never steps back. `ccAnalyzeEnemyList` (INF
0x0042f290) is the same loop for three rows. From Mutation on both
(MUT 0x004446f0/0x00444c20, OUT 0x0043f9a0/0x0043fa50, QUA
0x003322b0/0x00332360) first set `rank = num - range` when `num < rank +
range` and clamp nothing after; Outbreak and Quarantine register through
`ccRegisterEnemyOne` and analyze `range` rows into a four-word stack
buffer, comparing three. So a start past the end is an Infection-only
read; a negative rank reads the list before on every volume.

What is there, read from all four images: the 35 lists in
`ccEnemyListInfo` order, each 16-byte aligned. Infection and Mutation:
130 rows each, stride 528, the two words between lists 0. Outbreak and
Quarantine: lists 06 (143 rows, one fill word), 36 (170, two), 46 (138,
two) are longer, fill 0 too. After list 46's fill comes
`ccEnemyListInfo` (INF 0x005da200, MUT 0x006084e0, OUT 0x00605f80, QUA
0x004fd300), then `e1x1AnmTbl`; before list 00, `wpRadInfo`. (The
carried symbol sizes say 520 for every list, which is wrong for the long
ones.) On Infection rank 130 and 131 read 0, row 0 (Razine), and 132 on
the next list's rows.

The port now holds this as the game lays it out: a new derived gen entry
`battle.enemy_list_ends` (crates/piney-gen/src/manifest.rs,
`enemy_list_ends`) reads `ccEnemyListInfo`'s pointers and, for each
list, the words from its end to the next list in memory (the last: to
`ccEnemyListInfo`), typed `EnemyListEnd { va, fill }`.
`piney_battle::entry::EnemyLists::word` resolves `list + 4 * rank`
against those addresses, rows and fills. A read from `ccEnemyListInfo`
on would take a list address for a row, and the game would then write
`exist` far outside `enemyTbl`; no input of the game's gets there (next
section), and the port ends the rows at that point.

## Which inputs reach past an end

None in ordinary play, on any volume. Outside a story area the rank is
at most `25 * 4 + 10 + 10` (the words' highest `enemyOfs` is 10, the
generator's `rand() % 20 - 9` also 10) plus a dungeon's `floor + 1` of
at most 5 (`dungeonData`'s highest `levelMax`), 125, and the lowest is
`10 - 10` = 0. Infection's story areas' `enemy` values reach 125 (area
42, four floors by its words: 129 at most) and 123 (area 25, five
floors: 128); every story area with `enemy` over 110 has words, and no
event instruction puts the party on a dungeon floor (`scene` targets on
Infection are towns and fields only). From 128 the rank holds on row
129. From Mutation on, story `enemy` values up to 162 are moved back by
the clamp. The 144 was the port's town call alone.

## What changed

`Register::register_event_mng` is `ccRegisterEventMng(0)`'s enemy part
(init, field 126, the field table, the 0x40 bar, the difficulty rows,
the event's 5/6 entries and type-0 portals, in the game's order) and
`entry::difficulty` is `ccRegisterDifficultyEnemy` over a
`DifficultyIn` (`WorldMan::difficulty_in` fills it). The world records
the event's `entry[]`/`entryMc[]` at the set-up's first pass, where the
port already runs `ccRegisterEventMng(0)`'s NPC count, and
`Combat::start_entries` registers from them; the looks loaded for an
area are now the rows that registration marks existing (`load_looks`
takes them) instead of three list rows plus the event's. So a Root Town
registers nothing, story fields 1-13 register only the event's enemies
(their portals draw from those), and from Mutation on field 126 nothing.
`list_rows` and `analyze_levels` read through `EnemyLists`; piney-fieldui's
`Words::analyze_enemies` (the gate's area level) uses them instead of
its own copy, which read level 0 past an end.

## Tests

tools/test_battle_spawn_rs.py: `register` now draws ranks past each
list's end (n to n + 134; not list 46, whose end borders
`ccEnemyListInfo`) and before its start (-130 to -1; not list 00), and
the new `event_register` runs `ccRegisterEventMng(0)` natively on a
random place (area 0-3, fields -1, 0, 1-13, 14-160, 126, floors -2 to 6,
`dungeonType[]`, level 1-5, offset -10 to 40, story `enemy` 0-170 or
255, entries of every type with 0x40 and +32 variants, portals of types
0-2), stubbing its non-enemy callees, against the probe's new `sreg` op
4. 300 cases of each: 0 mismatches on all four volumes. On the old code
(main's crates and data) `register` fails at its first case with the
probe's panic at entry.rs:226, "len 130 but the index is 244".

`cargo test -p piney-game --release -- --ignored infection_whole_story`
(uncapped) now passes: 1 passed in 353.61 s, past event 20 where it
panicked at frame 111,600 on main.

## When `ccRegisterEventMng` reads the entries (follow-up)

Settled from the code on all four volumes. At an area's set-up
`ccSetupGameCtrl` runs `ccInitRand`, `ccStartThEvent`,
`WORLD_MAN::SetEventData`, `ccEnableThEvent(0)`, then the area's
`ccSetFileList*` (town, field or dungeon) and `ccFileExistCheck`, then
`ccEnableThEvent(2)` (INF main 0x00168c98-0x00168f94, MUT
0x00168e18-0x00169114, OUT 0x001680f4-0x00168448, QUA
0x00167f84-0x001682d8). `ccEnableThEvent(0)` breathes until `eventMng`
+0xc is 1 or negative (INF main 0x001b52c0), that is until pass 0 has
settled, and nothing between it and the file list breathes
(`SetSpcItemTown`, `SetTradeItemTown`, `fieldSel`, `dungeonSel`). So
the set-up's call reads `entry[]`/`entryMc[]` exactly as pass 0 leaves
them, which is where the port already took them; the open question of
the first part is closed.

There is a second reader the first part missed. The `stream` instruction
(case 10), in play only (`lv` 2, the `slti s5, 2` at INF main
0x001aa0bc), with `game.status` 5 and `area == areaPrev`, calls the
area's `ccSetFileList*Del` (`ccRegisterEventMng(1)`: `ccInitRegisterAll`
and the regular gimmicks) before `ccEventStream` and the area's
`ccSetFileList*` (`ccRegisterEventMng(0)`) after it (INF main 0x001aa0c8
and 0x001aa170, MUT 0x001bf310 and 0x001bf3b8, OUT 0x001b5600 and
0x001b56a8, QUA 0x001bce68 and 0x001bcf10). The reload reads the entries
as they are after the stream: the enemy rows are registered anew (and
their files loaded), each party entry goes through `ccSPC::EntrySpc`
again (a registered member keeps its slot, INF gcmn 0x0059f740) with its
boot param, `registNpcNum` (+0x1c) grows again by the town NPC entries
and by `ccSpcManager.registryNum` (INF main 0x001b6fd4), and in a Root
Town `ccRegisterRandomNpc` (0x001b7660, towns only) draws from `ccRand`
again. None of its other callees draws (`ccRegisterInu`,
`ccRegisterMerchant`, `ccRegisterNpc`, `ccRegisterRegularGimmick`,
`ccRegisterGimmick`, `ccSetFileListBoss`, `EntrySpc`,
`ccInitRegisterAll`, `ccRegisterEnemyOne`, the file lists,
`ccFileExistCheck`). The port did none of this, and drew the town's
walking PCs at their placement rather than in the set-up's call. Now the
set-up's call (`World::set_registry_entries`, after pass 0) also does
the party slots, adds `registryNum` to `registNpcNum` and in a Root Town
draws the walking PCs then, keeping the rows for `ccEntryEventMng`'s
placement (`place_entries`, which draws only if no pass drew). piney-event's
`stream` calls the host's `reload_file_list` after the stream under the
game's conditions; the area host grows `registNpcNum` by the town NPC
entries and `World::reload_file_list` registers the enemy rows anew
(looks loaded), the party slots, `registryNum` again and the town's draw,
whose rows replace the kept ones. Since `ccEntryEventMng` places when the
tasks start, after pass 2, a stream played before then places the second
draw: the golden shots' Mac Anu chat (2 frames) and event 11 warp (8
frames) now show another walking PC in the background, drawn for
`registNpcNum` 2 instead of 1 (the reload ran there with area and
`areaPrev` 0 and one registered member); the other 4,540 shots are
main's. The VM's own reload step changes nothing in the event manager:
the event harness's fixture stubs the file lists, so `registNpcNum`'s
growth is the host's (tests/vm.rs `every_block_played_matches_the_game`
caught a first try that grew it in the VM). Held by
`a_stream_in_play_reloads_the_area_s_file_list` (piney-event): the
reload under status 5 with the area unchanged in a town, field and
dungeon, none with the area changed, outside 0-2 or on the desktop.

## The ignored playthroughs

Every ignored piney-game test (153), uncapped, one at a time, on the
branch with this follow-up (the final code; the last 44 under a 20 GB
memory-capped scope each, after two editor crashes stopped the run):
145 pass, 8 fail, every result the same as an earlier full run of the
follow-up before the walking PCs' draw moved. The branch's worktree got
a `work/` of its own (symlinks to main's but for `data`), since the
first part's `gen` had rewritten the shared `work/data/*/TABLES/battle.bin`
in main's `work/`; main's were restored from main's own `gen` in a
scratch worktree at 65cec7c, which ran the eight failures:

| test | branch | main 65cec7c |
| --- | --- | --- |
| corbenik_fight_to_its_end | party fell at 6,006 | the same |
| kyvia_fourth_under_the_pilot | 312 not past its fight at 399,937 | the same |
| event_307_from_its_start | party fell at 31,687 | the same |
| machas_fight_to_its_end | party fell at 1,796 | the same |
| event_310_from_its_start | party fell at 6,323 | the same |
| tarvos_fight_to_its_end | party fell at 37,103 | the same |
| quarantine_whole_story | stalled after 311 at 656,400 (field 12, play 128,488) | stalled after 311 at 700,200 (field 12, play 145,318) |
| rumble_on_the_pad | no pad with force feedback | the same |

The seven are worklog [[597]]'s known failures, six of them to the frame;
Quarantine's whole story crosses fields, so the moved draws move its
frames, and it stops where main's does, at event 312's fourth Kyvia
fight. `rumble_on_the_pad` wants a force-feedback pad plugged in. No test
fails here that passes on main, so no bot or port change followed.

## The reload checked against the game's code

The ten changed shots rested on the code reading above, so the reload's
NPC part was then run as the game's code. tools/test_world_rs.py
`test_stream_reload` sets the town machine (`TownPcsRun`) to a Root
Town's state at the stream (`game.status` 5, `area` and `areaPrev` 0,
`ccSpcManager`'s five slots, boot params and `registryNum`, `entry[]`,
`registNpcNum`, `game.server` and the done events behind the pool,
`ccRand` from `ccInitRand` at a frame count), runs the game's own
`ccSetFileListTownDel` and then `ccSetFileListTown` (their file requests
and the registrations outside the party, the NPC files and the walking
PCs answer nothing; `ccRegisterEventMng`, `EntrySpc` and
`ccRegisterRandomNpc` run), and compares `registNpcNum` after, the
registry, the sixteen rows drawn and the next `ccRand` with world_probe's
new `pcreload`, which runs the port's reload: `town_npc_entries` and
`register_event_npcs`, the two pieces `World::reload_file_list` now calls.
The first five cases are the shots' state (Kite alone, no entries,
`registNpcNum` 1 as pass 0 leaves it, five frame counts), then 80 random
ones (one to three registered members, entries of every type and type
35, `registNpcNum` 0-3, 16, 17, random counts, servers and done events
from Mutation on). 85 cases agree on all four volumes. The shots' case
comes out `registNpcNum` 2 and fourteen walking PCs, as the port has it.
With the probe made to skip the reload (the old port), the check fails at
its first case (`registNpcNum` 1 against the game's 2). No port change
followed, so nothing was shot again.

The time is three clocks: the first part's 31m (to the entry), and the
follow-up's from 13:57 to 20:13 less about 15 minutes in which an editor
crash at 18:30 had stopped the session; the first part's checks between
12:41 and 13:57 ran unclocked. The check against the game's code added a
third clock, 20:12 to 20:57 (45m).

**Still unknown:** nothing.
