---
number: 285
title: Open questions after the triage: towns, fields and dungeons
date: 2026-09-28
area: world
files: UNKNOWNS.md
resolves: 11, 19, 21, 25, 27, 28, 30, 31, 33, 59, 65, 72, 80, 81, 90, 93, 97, 106, 108, 111, 132, 140, 142, 148, 154, 162, 163, 164, 166, 177, 191, 206, 277
---

# 285. Open questions after the triage: towns, fields and dungeons

This entry closes the open questions of 33 entries about the world: the area
words and the generators, the Root Towns, the fields and the dungeons, their
objects, gimmicks and people. The triage of 2026-09-28 (UNKNOWNS.md) split
every entry's open paragraph into single questions and checked each one
against the later log, the docs and the code. The answered ones are listed
below with their evidence. What is still open is restated at the end, with
the entry that asked it, the play questions first. Open questions of these
entries that belong to another subsystem are restated in that subsystem's
entry: [[282]], [[283]], [[284]], [[286]], [[287]], [[288]] and [[291]].

## Answered

- [[11]]: how `enemyOfs` and `itemOfs` feed the enemy and item tables —
  answered by [[13]] (enemy lists), [[275]] (story areas use column 6),
  [[68]] (`AreaItem`; `crates/piney-fieldui/src/menus/getitem.rs`
  `area_item`)
- [[11]]: what `EVENTAREA_INFO.flag` does — answered by [[179]]
  (`WORLD_MAN::GO` copies it to `hackFlag`; 3 is hacked and turns on
  `interNoiz`)
- [[11]]: what `EVENTAREA_INFO.model` does, in part — answered by [[112]]
  (sound bank 5 by model) / docs/engine/evarea.md (model 1 is a story map
  file); the rest is below
- [[11]]: what `SetDungeonTypeFromField` decides — answered by [[19]] (the
  dungeon type; 12,288 inputs checked) / docs/engine/dungeon.md
- [[11]], [[21]]: the save flag behind area 71, `saveData+0x5bc0` bit 62 —
  answered by [[220]] (`eventFlag[217]` bit 62: event 217 done)
- [[19]], [[30]]: the treasure, circles and idols `EntryGimmick` places
  later — answered by [[98]] (dungeons), [[164]] (fields): placed at run
  time by the ported setters, not derived from the seed
- [[19]], [[30]]: the story dungeons' room models — answered by [[114]] /
  `crates/piney-data/src/dungeon/special.rs` `event_room`
- [[19]]: object heights — answered by [[38]]
- [[21]]: whether `DUNGEON::Generate` and `WORLD::Generate` match on the
  other volumes — answered by [[25]] (0 mismatches on MUT, OUT, QUA)
- [[25]]: what `saveData+0x5ec8` bit 62 means — answered by [[220]]
  (`eventFlag[314]`: event 314 done)
- [[25]]: `MakeFloor`'s story path with more than 10 floors — answered by
  [[181]] (area 125's 15 floors carried) /
  `crates/piney-data/src/dungeon/mod.rs` `Tables::edit_floor_count`
- [[25]]: `ccSkillDamage`'s area path — answered by [[67]]
  (`tools/test_battle_rs.py`, `ccSkillDamage` area)
- [[25]]: the Data Drain drop roll — answered by [[102]]
  (`tools/test_battle_drain_rs.py`)
- [[27]]: the canal water's placement (`wat1`, three duplicates) — answered
  by [[28]] / docs/engine/statics.md (all three rooted at the identity)
- [[27]]: the sky and `CMP_sr1dat1_*` clumps the constructor builds —
  answered by `crates/piney-world/src/town01.rs` (the sky, `CRISIS_SKY`;
  `tools/test_world_rs.py`)
- [[27]]: which LOD `DrawObj` picks, and the flags at `+0x1b0`/`+0x1b4` —
  answered by `crates/piney-world/src/town01.rs` (rows 11 and 8 swap to 29
  and 30 by y; `clip[3]` at `+0x1b0..+0x1b8`)
- [[27]]: animation beyond frame 0 — answered by [[32]], [[35]]
- [[28]]: the waves (morph playback) — answered by [[36]] (morph targets
  blend)
- [[28]]: what `town_z` holds — answered by docs/engine/statics.md (no
  models: the lens flare's and the clouds' seven Eff chunks), [[97]]
- [[28]]: how `EVENTAREAB0` picks its area — answered by [[112]] (fields
  1-8; area 27's last door)
- [[28]]: what draws type 0 rows — answered for Mac Anu by
  docs/engine/statics.md (`ROOTTOWN01::Draw` draws them itself); the other
  classes are below
- [[30]]: the start positions of rooms without a lake entry (`OBJ_0ppp`) —
  answered by docs/engine/dungeon.md "The start"
  (`crates/piney-world/src/dungeon_area.rs`, `tools/test_dungeon_rt.py`)
- [[30]]: the other volumes' dungeon tables — answered by [[181]], [[265]]
  (generated per volume)
- [[31]]: the rotation and placement rule that makes rooms join — answered
  by [[34]] (the room matrix was right)
- [[31]]: the door models and where they go — answered by [[34]], [[87]]
- [[31]]: per-type fog and ambient, and room lights from `LGT_` objects —
  answered by [[34]] (fog), [[157]] (`SetLight`), [[151]] (direct lights)
- [[31]]: whether rooms away from the player are drawn together — answered
  by docs/engine/dungeon.md "Assembly" (`DUNGEON::Draw` draws one room at a
  time)
- [[33]]: object heights from `WORLD::GetHeight` — answered by [[38]] /
  `crates/piney-data/src/field/render.rs` `Field::get_height` (178 points
  checked)
- [[33]]: how the ground mesh is built and where the cover is drawn —
  answered by [[38]]
- [[33]]: what is rolled after the start position: the weather and
  `EntryGimmick` — answered by [[130]] (weather), [[164]] (`EntryGimmick`)
- [[33]]: the other volumes' field tables — answered by [[188]]
- [[33]]: the keyword-to-seed step — answered by
  `crates/piney-data/src/area/mod.rs` `sim_generate_code`
  (`tools/test_area_rs.py`, [[220]])
- [[59]]: the event task in town (event 2's arrival with Orca) — answered by
  [[69]]
- [[59]]: NPCs and the Chaos Gate — answered by [[61]], [[65]]
- [[59]]: the HUD, map and field menus — answered by [[66]], [[68]], [[71]],
  [[77]]
- [[59]]: leaving the town — answered by [[72]]
- [[59]]: the town's bank and `ccSndBgmCtrl`'s town case — answered by
  [[60]], [[185]]
- [[59]]: footsteps and ambient loops — answered by [[138]] (footsteps),
  [[184]] (the canals)
- [[59]]: the town's fog — answered by [[145]]
- [[59]]: the water textured from the frame buffer — answered by [[97]]
  (water 0 drawn in both towns)
- [[59]]: Kite's shadow — answered by [[160]]
- [[59]]: the arrival's effect — answered by [[79]], [[129]] (towns run
  `effTransfer`)
- [[59]]: the other towns — answered by [[97]] (Dun Loireag), [[218]],
  [[260]]
- [[65]]: the party's own AI (`ccThFellow`, `ccAI`): walking, turns,
  `pc_act` — answered by [[67]], [[78]], [[110]]
- [[65]]: the administrators — answered by [[73]]
- [[65]]: the other `ccThGameCtrl` buttons and the talk requests' menus —
  answered by [[66]], [[70]], [[77]], [[135]] (the menu ban)
- [[65]]: the walking PCs' texture variants, footsteps and dust — answered
  by [[139]]
- [[72]]: the party left behind in the field — answered by [[74]]
- [[72]]: event 3 does not start (the VM asleep in the scene) — answered by
  [[74]]
- [[72]]: the field's entry control is empty — answered by [[78]], [[80]],
  [[164]]
- [[72]]: fog taken per model — answered by [[145]]
- [[72]]: story maps of their own (`EVENTAREA` files) — answered for
  Infection's by [[84]], [[112]], [[177]]; the later volumes' classes are in
  the volumes' entry
- [[72]]: only Mac Anu; the other servers refused — answered by [[97]],
  [[218]], [[260]]
- [[80]], [[81]]: the dungeon side: field 14's dungeon and event 4 —
  answered by [[87]], [[98]]
- [[80]]: the fight's sounds (`ccSeOn3D`, the enemy and party sound
  parameters) — answered by [[81]]
- [[80]], [[81]]: the AI's chat lines — answered by [[120]], [[123]]
- [[80]]: spells through `piney-effect` — answered by [[81]]
- [[80]], [[81]]: Data Drain's presentation — answered by [[102]], [[105]]
- [[80]], [[81]]: the game over past its signal — answered by [[158]]
- [[80]], [[81]]: `boss()` for the events — answered by [[113]]
- [[81]]: other agents working in the same `piney-audio` files — no longer
  applies: a process note, not a question
- [[90]], [[108]]: party members stand still in town; no following —
  answered by [[110]] (the members under `ActInTown` in town)
- [[90]]: event 11's block 17 at the gate and the rest of event 11 in area
  15 — answered by [[91]], [[129]]
- [[90]]: `inviteSpc`'s warp-in of an unregistered member at the Chaos Gate
  — answered for the towns by docs/engine/field-walk.md (`party_add`); the
  fields and dungeons are below
- [[90]]: the gate-address window in town — answered by [[90]] itself
  (`FieldUi::announce_lines`)
- [[93]]: Dun Loireag not ported — answered by [[97]]
- [[93]]: `piros_colour` and the town's characters' `affectColor` — answered
  by [[99]]
- [[93]]: the later events' fields and dungeons, fights and bosses not
  played — answered by [[109]] to [[117]], [[129]], [[159]] (the ending
  played to a saved card)
- [[97]]: `ccDog` and the chibi Grunties — answered by [[142]], [[162]]
- [[97]]: `ROOTTOWN03`-`05` — answered by [[218]], [[260]]; their
  four-sprite `DrawMap` is in the volumes' entry
- [[106]]: the party's arrival chat lines (`arrivalChatCnt`) — answered by
  [[123]] (`ChatMessageEnteredTown`, `EnteredField`)
- [[111]]: `RoomSelect`'s 71 and 77 branches (`OBJ_user_point`) — answered
  by `crates/piney-world/src/dungeon_area.rs` `room_select`
- [[148]]: whether "Chronicling" can be had in Infection; `timeSym` and
  `gameCnt` — answered by [[154]]
- [[148]]: `ccGimSymbol` not ported (symbols as stand-ins) — answered by
  [[149]]
- [[162]]: the ride (`ccPgAdultCheck`, `ccPuccigusoStart`, `ccPucciguso`,
  `pgRideFlag`) — answered by [[166]]
- [[162]]: a field's `EntryGimmick` (`SetFood`, portals, special objects) —
  answered by [[164]]
- [[163]]: row 21 (`ENTRANCE`, `effDungeonEntrance`) made only by the
  harness — answered by [[164]] (`SetSpecialObj` makes the entrance swirls)

**Still unknown:**
- (play) [[191]]: `come_back` builds the lake room's doors open, where the
  game asks `ccCheckActiveObject` — climbing back into a lake dungeon.
- (volumes) [[206]]: where Mutation's and Outbreak's `idolItemList44` is;
  neither candidate reads like Infection's.
- (volumes) [[260]]: Fort Ouph and Lia Fail are not compared with the game
  in eemu.
- (volumes) [[275]]: Outbreak's and Quarantine's fellow deletes are not
  compared; no name was carried.
- [[11]]: what each value of `fieldType`, `weather`, `ground`, `object` and
  `circleOfs` looks like in the game; the port runs the ported generator and
  draw for every value ([[33]], [[38]], [[130]]), but there is no catalogue
  (docs/engine/area-words.md Unknown).
- [[11]], [[19]]: `EVENTAREA_INFO.model` beyond the sound bank and the story
  map file; `.protect` beyond `protect[1]` and the item pairs; `.dungeonNum`
  (docs/engine/area-words.md Unknown).
- [[21]]: what area 126 is: a record with no address, added from Mutation
  ([[220]]).
- [[21]]: why area 47 gets a different item offset from volume 3 (design;
  the mechanism is in [[21]]).
- [[179]]: whether any Infection field is hacked outside the crisis
  (`EVENTAREA_INFO.flag` 3); not surveyed.
- [[19]]: why dungeon types 8 and 9 pick a down-stairs room on their single
  floor (docs/engine/dungeon.md Unknown).
- [[19]]: `sd4a.cmp`, named by `DungeonName2` but not in `DATA.BIN`.
- [[25]]: what `DUNGEON.ishack` means; [[161]] reads 3 as hacked (the lakes'
  fireflies) (docs/engine/dungeon.md Unknown).
- [[114]]: area 67's type-34 row behind a door with no branch would crash
  the game; whether the story reaches it.
- [[33]]: where the game would loop forever, the port gives up with
  `NoSite`; no case has reached it.
- [[28]]: why the canal water is three copies of one animation. Water 0
  draws the refraction and waters 1 and 2 scroll in U and V ([[97]],
  docs/engine/town02.md), but what the first copy's other `Duplicate` flag
  (8200 against 8192) changes is not known. (The triage cited
  docs/engine/root-towns.md here, whose undrawn waters are Carmina Gade's.)
- [[28]]: what draws type 0 rows in the town classes other than
  `ROOTTOWN01`; not traced.
- [[163]]: what row 19 with `param[2]` 0 was for; no setter leaves it 0.
- [[163]]: `GetNearDoorPosition` with every mark beyond 12,000 returns stack
  words; the port keeps the row's place.
- [[164]]: the game's result for a field with no dungeon entrance (in-points
  0).
- [[164]]: the node places of the field's objects use the dungeon posing,
  checked on rooms only.
- [[164]]: where the foods stand in the game; sums put them 250-370 from the
  centre, not looked at in a picture.
- [[154]]: the fourth clock (`+0x70`) has no reader found.
- [[154]]: the Zeit statue, on the last floor of its dungeon, is not reached
  and opened in play.
- [[191]]: what reads `WORLD_MAN+0x58[n]` (docs/engine/field-walk.md
  Unknown).
- [[191]]: the walk between a lake's two dungeons against the game's run
  (docs/engine/dungeon.md Unknown).
- [[111]]: `RoomSelect`'s `bgColor` 0 and GS words are not modelled; the
  change's fade covers them.
- [[134]]: whether Kite should walk around the fenced statue in a statue
  room's open doorway (area 18's dungeon).
- [[72]], [[111]]: `SimGenerateCode` and `GO` move the global generator's
  seed and `randcnt`; the port leaves those draws aside
  (docs/engine/field-walk.md Unknown).
- [[97]]: `fieldrand`'s state on entering a town (the port starts at 13),
  and the scrolls' statics starting at 0 in each town (phase only).
- [[112]], [[115]]: `fieldrand`'s seed on arrival is not carried across
  scenes, from the dungeon or into the arena (the fireflies' bobbing; phase
  only).
- [[157]]: a story dungeon's `fieldrand` after `Generate`
  (docs/engine/dungeon.md Unknown).
- [[140]]: `DungeonEntries.rng` is a copy, so the dungeon's `fieldrand` does
  not advance with the breakables' draws; later draws can differ from the
  game's.
- [[59]]: `rand` is not seeded from the game's shared state, so idle fidgets
  differ in time (a run's history).
- [[65]]: whether a party member or the gate has a body on the character
  list.
- [[65]], [[132]]: the second argument to `ccSetRtownPC` on the event path
  (`ccEntryEventMng`); the port uses -1 (docs/engine/field-game.md Unknown).
- [[65]]: `ccSys +0x358`, the frame count that picks the PCs, is an input
  that depends on run time.
- [[65]]: an event NPC placed after the set-up goes to the list's end; the
  order is not checked.
- [[132]]: the event NPC classes step outside the entry control's NPC turn,
  so their effects start a frame late (docs/engine/field-game.md Unknown).
- [[277]]: whether a walking PC can run off in a case the tests do not
  cover; only a member's run was caught.
- [[108]]: the town branch of `SetCharPosition` is read, not run; its
  offsets match the field's checked ones.
- [[110]]: the town frame gets `menu_type` -1 or 0, not which menu.
- [[110]]: a second `rebootSpcManager` in one town visit makes no new AI for
  Kite's stand-in.
- [[90]], [[103]], [[180]], [[250]]: `inviteSpc` in a field or dungeon:
  whether it is ever reached, and its copy of the table's equipment into the
  registry, not ported because no script makes one
  (docs/engine/field-game.md, field-walk.md Unknown).
- [[106]]: how long the game holds the Chaos Gate loop (`ccLoadResourceFL`'s
  time) (docs/engine/field-walk.md Unknown).
- [[106]]: `ghoCam`'s markers from the desktop's anm player, not compared
  (docs/engine/field-walk.md Unknown).
- [[142]]: `ccDog`'s constructor leaves the stuck count (`+0x21c`) unset;
  the port starts at 0.
- [[142]]: the dogs' walks, not compared frame by frame.
- [[162]]: the Grunties' camera writes land a frame's NPCs later than the
  game's (ordering).
- [[162]]: `runAway`'s one frame of `checkHitResultAttlibute`; the harness
  lacks the collision's last result.
- [[166]]: the ride's task and its first cancel check run a frame later than
  the game's (ordering).
- [[166]]: the port sleeps and wakes the whole party, where `ccSPC::Wakeup`
  wakes only those in it (the same in Infection's fields).
- [[166]]: `plw +0x10` and `ccDamUprStr::CtrlAll` in the ride menu's waits
  are not kept.
- [[166]]: the Grunty's legs' places come from the port's last pose, not the
  game's last draw.
- [[166]]: riding into a dungeon's way in (`pgDIN`) is not played through.
- [[177]]: area 16's `EVENTAREA07` against a game picture.
- [[272]]: `EVENTAREAB8` is held by unit tests, not compared with the game's
  `Draw` (docs/engine/evarea.md Unknown).
