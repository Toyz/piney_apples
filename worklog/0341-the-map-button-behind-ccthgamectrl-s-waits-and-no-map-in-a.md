---
number: 341
title: The map button behind ccThGameCtrl's waits, and no map in a fight
date: 2026-10-01
area: ui, world
files: crates/piney-world/src/talk.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/map/mod.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, docs/engine/map.md
---

# 341. The map button behind ccThGameCtrl's waits, and no map in a fight

[[77]] left two map gaps open. The map button was taken whatever held
`ccThGameCtrl`'s other buttons. The field's and dungeon's map was drawn
through a battle. Both now follow the game.

## Where the map button's test is

`ccThGameCtrl`'s loop (INF gcmn 0x005178f0) runs, each frame, in this order:

```
Breath(1)
checkPartyAnnihilation() or compulsionGameOver   the game over: the task
                                                 never comes back to its loop
ccCtrlRecoveryReq, ccSortCmnd x3
ccMenu+6 != 66: SetInBattle(...) or inBattle = 0
ccCheckGtHackAnm()   (0x00517d58)               non-zero: back to Breath
menuClrWait > 0      (0x00517d68)               menuClrWait - 1, back to Breath
the map button       (0x00517d84)               assignPADmap and CheckOperate(13, 0)
ccPlayerMenuCheck ... the other buttons
```

The port's `Targeting::frame` (piney-world `talk.rs`) already ran the game
over, `ghoFlag` and `menuClrWait` in this order for the other buttons. The
map button was `piney-game`'s, and it ran before the task with no gate. Now
`Targeting::map_test` is set when a frame gets past `menuClrWait`. Both
worlds clear it at each `step_game_ctrl` and expose it as `map_test()`. The
town (`world.rs`) and the area (`area.rs`) press the map button right after
`step_game_ctrl`, only when it is set. The mode change still lands before
the menu task and the draw, as in the game. Nothing between the old place
and the new reads the mode.

`menu_clear_holds_the_map_button_two_frames` checks the gate in a town's
`Targeting::step`. A `menu_clear` before the second frame holds the test for
two frames (`true, false, false, true, true`). The task's first five frames,
which only sort, do not hold it.

## No map in a fight

`WORLD::Draw` calls `DrawMiniMap` only while `ccGame.inBattle` (+0x58) is 0
(INF gcmn 0x005a9890-0x005a98a4). `DUNGEON::Draw` calls `MakeMiniMap` for
the room under the player every frame, and `DrawMap` only while
`inBattle` is 0 (0x005cef64-0x005cef78). MUT, OUT and QUA test `game`
+0x58 before `DrawMiniMap` the same way, and step `menuClrWait` the same
way in `ccThGameCtrl`.

`map::area_frame` takes `in_battle`. In a field it draws nothing then. In a
dungeon it still enters the room (`MakeMiniMap`) and draws nothing. Its
`last` is emptied, so a sleeping frame after it draws nothing again, as no
packets were made. `the_map_is_away_in_a_fight` walks to event 3's east
portal, fighting its goblins: 237 fight frames, no map in any of them, and
the map on the walk before. With `in_battle` forced false the same run has
the map's packets in all 237. In a dungeon, `the_minimap_stays_across_floors`
(area 23's floors down to the shrine) now leaves the fights out of its
per-room count and checks them apart: 1,424 fight frames, no map in any.
Before, it had counted the fight frames as frames the map must draw in.

**Still unknown:** nothing.
