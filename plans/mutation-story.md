# Mutation's story end to end

Goal: a new game of Mutation plays to its ending, every story event (M201-M216,
101-116) finishing under the story autopilot, with the story maps it passes
through ported as the game builds them. Started 2026-09-28.

## How it is checked

- `mutation_story_survey` (piney-game, `--ignored --nocapture`): each event's
  start (`--mode story:N`, `crate::start`) driven by `StoryPilot` with
  `PINEY_SURVEY_GOD`; "done" once the event's flag is closed or done. 101 is
  the new game's desktop.
- `mutation_whole_story`: one run from the new game to 116's end. It
  ends every event, 101 to 116, and reaches the staff roll at frame
  703,500 (worklog 276); again at 751,800 after Outbreak's pilot changes
  and Fidchell (worklogs 293, 294), and at 613,800 after 206's (305).
  Event 115 from its own start stops in field 52's dungeon (a level-34
  party against a level-49 Squidbod); the whole run passes it.

## Where it stands (2026-09-28)

| event | state |
| --- | --- |
| 101 | done (new game: Mac Anu, field 27's dungeon, on into 104's field) |
| 102-106, 109-114, 116 | done |
| 107 | done (worklog 268): Innis drained and beaten, the town's breeder, the top page's Quit |
| 108 | done (worklogs 272, 273): Kyvia's disc, the core and gomoras fought through the menus, the party held at level 60 for it |
| 115 | done (worklogs 274, 275): field 13's six talks, Magus drained and beaten |

## The pilot's fixes so far

- The survey reads the event flag on every stage, and a closed event
  (`end_event`, bit 63) counts as its end.
- A dungeon's arena door (`GotoNextRoom` -255, `special::exit_field`) and a
  lake's dungeon 1 (`Enter` -1 on field type 4) are goals.
- Members still getting up from a revive (`condition[0]` 5) are not given
  orders: `ChatMenu1` refuses them, and the open menu pauses the fight, so
  the revive never ended.
- The top page skips a command the events take over (`add_operate`,
  command + 6), and writes a post waiting to be written (state 7).
- A repeatable block's event point is not a goal (a side line).
- Towns: a breadth-first way round the walls (`path_to`).
- A boss with its protect broken out of Data Drain's reach: walk to it.
- `PINEY_SURVEY_GOD` also holds Kite's infection at 0, so a drain cannot
  roll its game over (side effect 30).
- Gate Out from a field or dungeon: the PERSONAL menu is 1 there, 2 in a
  dungeon (0 only in town).
- A dungeon room whose doors are shut (`door_anm` false) is fought out,
  foes the pilot calls hopeless or not.
- The gate hack (62): god mode gives the Virus Cores its area asks for;
  the pilot drains no common foes to find them.
- The desktop: Mail only for the listed inbox (mail sent while the
  desktop is up is listed at its next opening).
- A boss's parts (Kyvia's core and gomoras) are foes; the healing gomora
  first, then the core, through the target menus; god mode holds the
  party at level 60 while they are up (the console's `exp`).

## Story maps Mutation needs

`WORLD_MAN::GO(1)` builds a story map by field (docs/engine/evarea.md):

| field | class | event | ported |
| --- | --- | --- | --- |
| 2, 3 | `EVENTAREAB0` (arenas) | 107, 115 | yes |
| 43 | `EVENTAREA03` | 102 | yes (worklog 263) |
| 9 | `EVENTAREAB8` | 108 | yes (worklog 272) |
| 13 | `EVENTAREA01` | 115 | yes (worklog 264) |

One `StoryMap` trait (`piney_world::story_map`), held as
`Place::Story(Box<dyn StoryMap>)`; every ported class is one.

`EVENTAREA03` (MUT gcmn 0x00416590 constructor, 0x00416bf0 `Draw`): scene
`se2_1`; `EA_MODELTABLE03` (two pass-0 models); `eventarea03Light` one
distant light; fog (150, 4000, 0, 80, 0x1e1e1e); start (0, 0, 0) w 1.0.
`Draw` flies the camera (camera 3) from view (1322, -1128, 1014) and pos
(1678, -1433, 1188) to (-210, 167, 36) and (162, -133, 178) over 120 of 140
frames, opens `kiteSelfTalk` at 140, and on its close `MenuClr`,
`changeCamera(1)` and `ChangeArea(0, 1)`; unless area 43 is marked on the
server (`gateListMark[server][1] & 0x800`, event 102's mark), when it only
draws. docs/engine/evarea.md has it all.

## Bosses Mutation needs

`ccBossEntryStart(code)` starts `bossFunc[code]` (worklog 264):

| event | code | boss | size (MUT) | ported |
| --- | --- | --- | --- | --- |
| 107 | 1 | `ccBoss02` Innis (+ `ccBoss02Slave`) | 54 functions, 38.6 KB | yes (worklog 266) |
| 108 | 12 | `ccThKyvia01` (+ the `kyvia*` classes, `EVENTAREAB8`) | 98 functions, 74.4 KB shared | yes (worklog 272) |
| 115 | 2 | `ccBoss03` Magus (+ `ccBoss03Leaf`) | 70 functions, 38.2 KB | yes (worklog 274) |

One agent at a time ports them, in story order, after the Skeith port
(docs/engine/boss.md, piney-battle's `boss.rs`, the harness
`tools/test_battle_boss_rs.py`).

## Stream effects Mutation adds or changes

`StreamDemoFuncTbl` (MUT main 0x00367210, 40 rows) runs a stream's effect
task by the scene's name. Infection's 22 rows come first; Mutation adds 18
more, 13 functions (Outbreak has the same names):

| scene | MUT main | state |
| --- | --- | --- |
| `str0710` | 0x0019c140 | stream 24, event 101's opening: ported (`opening.rs`) |
| `str0770` | 0x0019df80 | stream 25: ported (`mutation.rs`) |
| `str0780` | 0x0019e9e0 | stream 26: ported |
| `str0820`, `str0821` | 0x0019f5d0, 0x001a03a0 | stream 27: ported |
| `str0880`, `str0885` | 0x001a1050 | ported |
| `str0932` | 0x001a1cb0 | ported |
| `str1040`, `str1041` | 0x001a2880, 0x001a4930 | ported |
| `str1050` | 0x001a5640 | ported |
| `str1070` | 0x001a6af0 | ported, with its part system |
| `str1090` | 0x001a8570 | ported |
| `str9204`-`str9206` | 0x001a92a0 | ported |
| `str9201`, `str9301` | 0x00189ed0 | `Func_str9101`'s, mapped to it |

Without a task a stream plays bare. `str7100`, `str8800` and `str0300`
carry no Infection name, but their code is Infection's (worklog 271): the
port's tasks are Mutation's too.
