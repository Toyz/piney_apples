---
title: Leaving the town - Kite in a field and its dungeon
status: partial
volumes: INF
covers: INF SLUS_202.67:0x00168960 ccSetupGameCtrl (areas 1 and 2), 0x00165530 loadCheck, 0x0019ad70 ccLoadDispInit, 0x0019c290 ccLoadDispTh, 0x0019ba50 ccLoadDisp::allDisp, 0x0019f8e0 WORLD_MAN::GO, 0x001a1190 WORLD_MAN::SetCharPosition, 0x0019eea0 WORLD_MAN::SetGenerateCode, 0x0019dda0 WORLD_MAN::Enter, 0x0019e410 WORLD_MAN::GoField, 0x001a0ef0 WORLD_MAN::SetCenter, 0x001a0fd0 WORLD_MAN::AddCenter, 0x001a20b0 WORLD_MAN::SetActiveLayer, 0x001a4430 ccThFieldDisp, 0x00153760 ccModelHit::HitEnable, 0x001537e0 ccModelHit::HitDisable, 0x0013cf50 ccClump::HitEnable, 0x0013d0b0 ccClump::HitDisable, 0x0013d150 ccClump::SetHitMatrix, 0x00151e60 ccAnm::HitEnable, 0x001520f0 ccAnm::SetHitMatrix, 0x00153930 _ccHitCheckLM, 0x00153a30 prepareHitLine, 0x00153be0 prepareHitSphere, 0x00153e80 checkHitResultAttlibute, 0x00167380 ccGame::ChangeScene, 0x001674a0 ccGame::ChangeArea, 0x001671e0 ccGame::ChangeRequest, 0x0015a200 ccSleepNoSleepThread, 0x0015a010 ccDeleteAllThread, 0x001b5230 ccStartThEvent, 0x001ab9b4 teach_camera1-3 (ccEvent::Execute), 0x001676a0 ccGame::SetInBattle, 0x0019c430 ccLoadDispCheck, 0x001b77c0 ccClearGtHack, 0x001b7840 ccSetGtHack, 0x001b7850 ccCheckGtHack, 0x00378a98 gtHackFlag, 0x00378cc0 ghoFlag, 0x00162870 cameraParamChange; INF gcmn.prg:0x005a97b0 WORLD::Draw, 0x005a8320 WORLD::DrawObject, 0x005a8570 WORLD::DrawMesh, 0x005a7ef0 WORLD::DrawBG, 0x005aa2b0 WORLD::SetCenter, 0x005aa3d0 WORLD::AddCenter, 0x005aa520 WORLD::GetHeight, 0x005b0e90 FOBJECT::Draw, 0x005b13f0 FOBJECT2::Draw, 0x005b1700 FOBJECT2::HitEnable, 0x005b0aa0 FIELD_MESH::RelocateMesh, 0x00571e00 ccLandHitCheck, 0x00597910 ccPlayer::ccPlayer, 0x00598310 ccPlayer::Main, 0x0059b3c0 ccPlayer::MapLoopAdjustPos, 0x0059b470 ccPlayer::W2MPos, 0x0059b5a0 ccPlayer::W2PPos, 0x0059b710 ccPlayer::P2WPos, 0x0059b940 ccTransPosW2P, 0x0059b980 ccTransPosP2W, 0x0059b9c0 ccTransPosFW2LW, 0x0059ff50 ccGetStartPositions, 0x0056b1c0 ccChar::Draw, 0x0053c8e0 ccMenuCtrl::GateoutMenu, 0x0051a000 ccCheckInAreaCmnd, 0x0059cd60 ccCheckGtHackAnm, 0x0051a750 ccInitRecoveryReq, 0x0051a790 ccEntryRecoveryReq, 0x0051a7f0 ccCtrlRecoveryReq, 0x0072eb10 recoveryReq, 0x00572700 ccSkillRequest, 0x00516a80 ccThGameOver, 0x00516a20 ccAddRequestFileListGameOver, 0x00516d30 ccGameOverNoise::Main, 0x00516d90 ccGameOverNoise::NOISE, 0x005170c0 ccGameOverNoise::TV, 0x00517150 ccGameOverNoise::Noise, 0x005171d0 ccGameOverNoise::Noise2, 0x00517240 ccGameOverNoise::Noise3, 0x005172c0 ccGameOverNoise::Noise4, 0x00517330 ccGameOverNoise::StretchTV_Y, 0x005174d0 ccGameOverNoise::StretchTV_X, 0x005175b0 ccGameOverNoise::StretchTV_Init, 0x005175d0 ccGameOverNoise::InitData, 0x0056a7c0 ccOpenGameOverMenu, SLUS_202.67:0x00180580 ccSndGameOver, 0x0059a0c8 ccPlayer::AnimCtrl, 0x0059ce80 ccParty::AddMember, 0x0059cf60 ccParty::DelMember, 0x0059cfe0 ccParty::CheckMemberID, 0x005a08e0 inviteSpc, 0x005a0f50 disbandSpc, 0x005a1070 expulsionSpc, 0x0059bc00 ccPlayer::GateHackingOut, 0x005a10e0 ccAddRequestFileListSpc, 0x005a1490 ccGateHackOutCcsName, 0x006540c0 GateHackOutCcsName, 0x0041ae80 ccFellow::Initialize (the hacked arrival), 0x0057c5f0 ccAI::ccAI (arrivalChatCnt)
---

# Leaving the town - Kite in a field and its dungeon

What happens when Kite leaves Mac Anu for a field and walks it: the change
of scene, `ccSetupGameCtrl`'s set-up for a field (area 1) and a dungeon
(area 2), the field's frame - the player and camera on a ground that wraps,
the objects' collision coming and going with their draw - the dungeon
entrance, and Gate Out back to the town. The town's side of all this, and
the pieces the field shares with it (the camera, `ccPlayer::Main`, the
collision queries), are on [Entering The World](field-game.md); the field's
generation is on [Field generation](field.md) and the dungeon's on
[Dungeon generation](dungeon.md).

`crates/piney-world` (`area`, `field_area`, `field_world`, `dungeon_area`)
and `crates/piney-game` (`area.rs`, the session) are the port;
`tools/test_field_rt.py` and `tools/test_area_rs.py` check it against the
game's own code in `tools/eemu.py` (see [Checks](#checks)).

## The way out

The first story trip is event 2's: its last instructions are `area 14` and
`scene area=1 town=0 field=14`. Instruction 118 (`area n`) makes the story
area's `WORLD_MAN` from its own three words (`SimGenerateCode`, see
[area words](area-words.md)); `scene` is `ccGame::ChangeScene(1, 0, 14, -1,
-1, -1)`, which ends in `ChangeRequest(6, 7)` - `ccSetupGameCtrl` again.
Story area 14 is "Bursting Passed Over Aqua Field" (the script annotations
that name the areas are one row off here): fieldSeed 1420855, field type 10,
weather 0, ground 1, object 1.

The other ways out of a town and back:

```
Chaos Gate warp   WORLD_MAN::SetGenerateCode(a, b, c)          main 0x0019eea0
                    SimGenerateCode(a, b, c)
                    wordparam.fieldType != 4:  ChangeArea(1, eventAreaNumber)
                    else eventAreaNumber 0:    ChangeArea(2, 0)
                    else:                      ChangeScene(2, -2, eventAreaNumber, 0, 0, 0)
Other Servers     ChangeArea(0, town)
the entrance      WORLD_MAN::Enter -> ChangeArea(2, 0)          (the field's frame)
Gate Out          ccMenuCtrl::GateoutMenu, confirmed:           gcmn 0x0053c8e0
                    ChangeArea(0, game.town)
TransFieldMenu    WORLD_MAN::GoField (menu 86)                  main 0x0019e410
                    only in a dungeon (WORLD_MAN.area, +0x08, 2):
                    fieldtype != 4:            ChangeArea(1, eventAreaNumber)
                    fieldtype 4, dungeon 1:    ChangeScene(2, -2, -2, 0, 0, WORLD_MAN+0x100)
```

Field type 4 has no field: its words take the party straight into the
first dungeon, a story one keeping `game.field` as its area number.

## ccSetupGameCtrl for a field

`ccSetupGameCtrl` (main 0x00168960) runs as for the town up to the area
switch (the fade out of 10 frames over the old scene, its tasks still
running under it; sounds off; the tasks deleted; two frames held black),
then for area 1:

```
SetTradeItemTown           when the scene changed and it came from a town
fieldSel; ccSetFileListField
ccSndSQLoad(3)             or 5 for a story map of its own (EVENTAREA_INFO.model 1);
                           4 in a dungeon; nothing unless the scene changed
the tasks                  ccThGameCtrl 33, ccThMenu 34, ccThEntryCtrl 64,
                           ccThFieldDisp 96, ccThCamera 40, ccThSpc 48,
                           ccThSkill 82, ccThEffect 80, ccThParticle 98
WORLD_MAN::GO(1)           the field made (below)
ccGetStartPositions        WORLD_MAN::SetCharPosition for the leader
rebootSpcManager           ccPlayer::ccPlayer; WORLD_MAN::SetCenter at his feet
ccSnd.gameStart = 1
the fade in (10 frames), then ccSndBgmCtrl
```

The tasks are started in the slice that also runs `GO`, the start
positions, `rebootSpcManager` and `ccEnableThEvent(4)`; each runs its first
slice from the next walk of the task list. `ccThEntryCtrl`'s first slice is
its set-up (`restoreEntry` or `initEntryCCS`, `WORLD_MAN::EntryGimmick`,
`ccEntryEventMng`), then its first `Breath`, so the entries are made (and a
dungeon room's doors shut for them) before the event's first pass that
plays; its loop runs from the frame after. The port runs the set-up on the
frame of `Phase::Play(0)` and the tasks' frames from `Play(1)`
([dungeon](dungeon.md#the-doors-and-the-events)).

`WORLD_MAN::GO(1)` (main 0x0019f8e0): `eventAreaNumber = game.field`;
`defSE` (+0x15c), the ground attribute the height map answers with, from a
table by field type (0x00355760):

```
type   0       1       2           3           4       5           6           7           8       9       10
defSE  0x20f0  0x20f0  0x0090b0c0  0x0090b0c0  0xc000  0x00e0e0e0  0x00d0d0d0  0x00405060  0x6040  0x6040  0xc000
```

the bounds 0..48000 both ways (+0x420..+0x42c); the RNG seeded with
`fieldSeed`; a new `WORLD`, `SetHackFlag`, `WORLD::Init` and
`WORLD::Generate` ([Field generation](field.md)): the ground, the dungeon
entrance, key, sub and lake objects as `FOBJECT2`s (`fobj2[]`, the
entrance's hit enabled at once by `FOBJECT2::HitEnable`), the base and tree
objects as `FOBJECT`s at their chips (`fobj[40][40]`).

`WORLD_MAN::SetCharPosition` (main 0x001a1190) in a field: coming from a
town (`areaPrev` not 2) the leader stands at `fieldStartPos` facing 0;
back from the field's dungeon he stands beside the entrance
(`dungeonPos[0]`), 500 up, by field type:

```
type 10   (+900, 0)    facing 1.5675
type 6    (0, +900)    facing pi
else      (0, -900)    facing 0
```

The other three places (`StartPos[1..3]`; `ccGetStartPositions(1, 2, 3)`
asks for all of them) are round the leader, facing as he does:

```
field     (+200, +100)   (-200, +100)   (0, +300)
dungeon   (300, -150)    (-300, -150)   (0, -300)   turned by his facing
                                                    (sceVu0RotMatrix (0, 0, dirc))
```

`ccGetStartPositions` then gives each registered character the place of
its party slot (`ccParty::CheckMemberID`: slot 1 the first, slot 2 the
second), the origin to one outside the party.

`rebootSpcManager` (`ccSPC::Reboot`) builds everyone from the registry
`ccSpcManager` carried over, with the `bootParam`s the event passes left:
Kite (`ccPlayer::ccPlayer`, `SetBootStatus`) from a town at act 13 (the
arrival fade), back from the dungeon or inside one standing (act 2) with
his body on the collision list at once; each member (`ccFellow::Initialize`)
arriving after `(rand() % 4) * 5` frames, its AI manual when its
`bootParam` has bit 2. The party, the HUD's panels and faces with it,
comes along from area to area.

A character the event registers outside the party (`entry 0-2 code`,
`ccRegisterEventMng`: `EntrySpc` and its `bootParam`) is built by
`Reboot` too, at the origin, not in the HUD. `ccThEntryCtrl`'s set-up
(`ccEntryEventMng`) then places every such entry at its event position
(the marker's, with Kite's position added when the floor or block is
9999 or more) facing its `dirc`, and a `param` of 5 puts it on
`ccEntryCmnd`, the event's command list. Side event 61 builds Natsume
this way in her dungeon room; the port's `FieldWorld::set_spc_entries`
takes the registrations and `EvParty::place_entry` the placing.

## The loading display

`ccSetupGameCtrl` loads the scene's files with `ccLoadResourceFL`, whose
`ccFileListLoad` puts up `ccLoadDisp` (main 0x0019ad70-0x0019c430). It
does so when the new scene list has a file the old one lacked
(`loadCheck`, 0x00165530), no display is up (`ccLoadDispCheck`) and the
request's +0x18 is clear. `ccLoadDispInit` picks by `game.area` and
`areaPrev`:

| entering | shown |
| --- | --- |
| a town | the card: `townNameTbl`'s two lines ("Aqua Capital", "Mac Anu") centred at y 168 and 196 |
| a field from a town, no stream playing (not field 8) | the card: the three keywords joined by a space, centred at y 182 |
| a type 4 field's dungeon from a town, no stream playing | the same card |
| a field from its dungeon, a dungeon otherwise | the animation only |

The gate hack's arrival has stream 107 playing (`strPtr`), so it gets no
display.

The card (`allDisp`):

- `xdl_tit`'s banner in two cells: 256 x 128 at (56, 128), then 144 x 128
  from V 128 at (312, 128).
- The texts in `ccKanji` type 2, white.
- The server's symbol (`serverNameTbl`) at (364, 40) and "Server" at
  (388, 40), coloured by server: Δ 2, Θ 1, Λ 4, Σ 6, Ω 5.
- All at the display's alpha. The texts are sent first, so the banner
  lies under them.

Its task, `ccLoadDispTh` (0x0019c290, priority 17):

1. Until `ccSnd.gameStart`, each frame: the card with its alpha rising by
   8 to 128, and `xdl_load`'s `ANM_xdl_lod1` forward and drawn on the
   world's layer under its own camera (the THE WORLD logo at the bottom
   right).
2. With a card: 30 frames at 128, then 17 falling by 8, without the
   animation, over the scene's own fade in.
3. It is deleted.

The port's loads take no time, so only a few frames of step 1 show and
the card mostly shows for step 2's 47 frames, as the game's does once
its load is done. piney-game's `loaddisp` builds the display in the
session's `world_stage` (the town from the top page, every change of
scene), and ends step 1 on the scene's `GameStart`.

## The event task across the change

The event task that asked for the change is asleep inside `scene`:
`ChangeRequest(6, 7)` calls `ccSleepNoSleepThread(1, 1)` (0x0015a200),
which raises the sleep count of every thread without the no-sleep bit
(bit 0 of +0x10) and then, unless the caller has bit 1, puts the calling
thread to sleep (`SleepThread`, flag 0x20). The event thread keeps its
no-sleep bit - `ccDeleteAllThread` (0x0015a010) passes it over too - so
its count stays 0 and it is woken at the next frame's schedule: the
instruction returns and the block goes on. Event 2's next instruction is
`end_event`, which closes it (bit 63); the set-up's `ccStartThEvent`
(0x001b5230) turns every closed event done (bit 62), so event 3's
`event_done 2` holds at the pass at phase 0. The port resumes the pass in
the area's first frame (the game in the first frame of the fade out; the
save's flags are all that moves between) and then runs the set-up's
passes as the town does: 0, 2 (event 3 sets the party's `pc_mode` 4 and
registers its magic portals), `rebootSpcManager`, 4 at F0, and one play
pass a frame before the tasks.

Event 3 (TEACH-F) then plays: Orca's camera lesson (messages 2-6, the
prompts `teach_camera1..3` counting the pad as `ccEvent::Execute`
0x001ab9b4 does), the field's explanation, the map and the Fairy's Orb,
up to the fight, which runs over the battle's characters
([battle](battle.md#how-it-plugs-into-piney-world)): `pc_command` lists
Kite, `pc_walk_dir` walks him to the portal south of the start, which opens
on a goblin that `hold 5 130` keeps facing him; `player_skill` has the
attack button hit it until it is down; then the skill lesson (menus 80-82:
PERSONAL, Skills, Repth on Orca) and the chat lesson (83), and the event
ends with `hold_end`, the camera given back and `menu_clear`, which frees
both from manual control.

The lesson's prompts give the event camera to the pad, so what the
player tries moves the view while the lesson counts it:

```
teach_camera1   prompt open: cpCtrl 5 (turn: L1/R1 held with camCtrlType 1,
                the right stick with 0), the camera task started
teach_camera2   prompt open: cpCtrl 6 (zoom: the right stick with 1, R1/R2
                held with 0; and turn)
teach_camera3   the reset button taken (R2 with 1, L1 with 0; 0x001abfb0):
                cpCtrl 7 and the task started if it is not - the camera
                turns back behind Kite a quarter of the way a frame; then
                45 frames and the prompt closes
```

## The field wraps

A field is a torus 48,000 units a side. Nothing is moved when the player
crosses an edge; each frame the positions are taken relative to the player
and wrapped into -24,000..24,000:

```
ccPlayer::W2PPos (ccTransPosW2P)  pos - player, wrapped by the bounds' span; z kept
ccPlayer::P2WPos (ccTransPosP2W)  back, onto the player's side of the map
ccPlayer::W2MPos                  a world point into 0..48000 (the height map's)
ccTransPosFW2LW                   W2P then P2W: the copy nearest the player
MapLoopAdjustPos                  after the move: the player wrapped into the
                                  bounds and WORLD_MAN::AddCenter(move)
WORLD::AddCenter                  ofs += move, wrapped into 0..48000
```

`cameraSet` passes the eye and the target through `ccTransPosFW2LW`, so the
camera never sees the seam; the objects are drawn at their copy nearest the
player (`FOBJECT::Draw`, `FOBJECT2::Draw`: `ccTransPosP2W(ccTransPosW2P(wp))`).

## Collision

The field's collision is two layers. `ccLandHitCheck` (gcmn 0x00571e00)
first asks the `ccModelHit` list, as in a town; in a field (`game.area` 1)
when nothing there is found below, it takes `WORLD_MAN::GetHeight`
(`WORLD::GetHeight`, the height map interpolated over its chip) and answers
with `defSE` as the ground attribute. The wall and camera queries see only
the list.

The list holds the objects near the player. A `ccModelHit` belongs to a
model (`ccModel.hit`, +0x3c) - every object of that model shares one, and
its matrix is where the last of them was drawn. The draw keeps the list:

```
WORLD::Draw          first call: ten DrawSnow (weather 4, 5); the depth
                     shades (sysLayer); layer 6 (refLayer): the heat haze
                     (types 0-3); the lake's water
                     layer 1 (objLayer): DrawObject
  DrawObject         the FOBJECT2s but the entrance (0), key (1) and type 6;
                     then the FOBJECTs of the 12 x 12 chips round the centre
                     (-6..5 each way), x outer, each wrapped once: every one
                     first HitDisable'd, then drawn
  FOBJECT::Draw      its copy nearest the player; the fade: full to 6,700
                     from the player, gone at 7,200 (over 500); SetHitMatrix
                     at its place, HitEnable (onto the list's tail), the
                     anm stepped
                     layer 2 (obj2Layer): the entrance and key objects
  FOBJECT2::Draw     beyond 9,600: HitDisable, not drawn; within: placed,
                     full to 8,600, faded over 1,000, SetHitMatrix, HitEnable
                     layer 4 (floorLayer): DrawMesh - the ground tiles and
                     cover of the same chips, and the type-6 objects
                     layer 3 (effLayer): DrawEffect - the rain and the
                     storm, type 7's fires' smoke, the snow and type 0's
                     embers, the 2D smoke, the fireflies
                     layer 5 (charLayer): TOBJ, BIRD
                     layer 3: the lens flare; type 0: DrawSteam(1)
                     DrawBG on the background layers
```

The weather and the ambient pictures are [field.md](field.md)'s
("Weather and ambient pictures").

So the list the player walks in a frame is the one the last frame's draw
left, in its order: the entrance first (enabled by `GO`), then objects in
the draw's order. `ccModelHit::HitEnable` (main 0x00153760) appends a hit
not already on the list; `HitDisable` (0x001537e0) unlinks it. The hit's
matrix is type 0 (translation only): `SetMatrix_PosRotZYX(pos, 0)`, the
inverse not read.

Since the objects of one model share its hit, only the last of them drawn
in a frame stands in the way: Kite walks through the others of that model,
and the camera's `avoidObstacle` does not keep out of them either.

`checkHitResultAttlibute` takes the ground-type nibbles of the nearest
result unless it is shaded (0x40000) or of type 0xe000e0; when none
qualifies it takes them from `$a2`, which in a field holds an address inside
the hit data - where the game's heap put it. The port marks such a frame
(`Player::attribute_qualified`) and the checks compare the other bits.

The dungeon entrance's own hit carries attribute 0x80000 on the floor of
its doorway: `ccPlayer::Main` standing on it calls `WORLD_MAN::Enter`,
which asks `ChangeArea(2, 0)` - the first dungeon, floor 0, room 0.

In story area 14 the entrance stands at (17400, 24000, 0), its hit
unturned (`SetMatrix_PosRotZYX(wp, 0)`): 208 polygons, most of them walls
(0x40e00fef and others), the steps down into the pit 0x20606f6f, and the
doorway's floor two triangles of 0x20686f6f at z -465.5 spanning x
16218-16748 and y 23698-24290 - 652 to 1182 units west of the middle,
across its axis. Kite reaches it walking west down the steps from the
middle; the ground between the steps and the doorway (x down to about
17075, z -246 to -441) carries no 0x80000.

## A change of scene and the event task

`ccGame::ChangeRequest` (main 0x001671e0) calls `ccDisableThEvent`
whatever asked for the change: the event instruction `scene`, and the
world's own - `WORLD_MAN::Enter` at the entrance, a dungeon door or
stairs, Gate Out, `GoField`. The event task makes no pass from then
(phase -1) until the next set-up's `ccEnableThEvent(0)`, so a block of the
area being left cannot start during its fade out.

## Drawing him

`ccChar::Draw` (gcmn 0x0056b1c0) fades a character by the camera's distance
with `ccGetCameraTransparency(pos, width, height, far, len)`: in a town
(`game.area` 0) far 4,000 over 400; in a field or dungeon far 7,000 over
600. It draws (`ccAnm::Draw`) unless the transparency is under 0.05 and the
camera is not hiding him.

## Into the dungeon and back

`WORLD_MAN::Enter`'s `ChangeArea(2, 0)` runs `ccSetupGameCtrl` again for
area 2: `GO(2)` builds the field's first dungeon (story area 14's is the
hand-made `D0001`), Kite stands at `WORLD_MAN.position` in room 0, and each
door or stair he walks into is another `Enter`: a door `GotoNextRoom`s and
changes the scene to the next room, the up stairs of floor 0 go back to
the field with `ChangeArea(1, eventAreaNumber)`, where `SetCharPosition`
puts him beside the entrance. The dungeon side - its rooms, hits, doors and
the walking - is on [Dungeon generation](dungeon.md#entering-and-walking-a-dungeon).

The following camera in a dungeon (`cameraPosCalc`, `game.area` 2) is kept
in front of the walls by the line from his head alone (`ccHitCheckLM`,
mask 4: pulled to 10 short of the hit), with neither the field's ground
check nor the town's push out of the walls. Walked into and along the four
walls of `D0001`'s first room (`story_4_wall_shots`, 600 frames), no frame
has a wall between his head and the camera; where the camera closes in on
him he fades (`ccGetCameraTransparency`).

## The party's comings and goings

Membership changes outside the towns go through the same functions as in
Mac Anu ([field game](field-game.md)), over the field's characters: the
registry and `ccParty` are the area's `Spcs` (party.rs, carried between
scenes by the session), the characters are the fights' (`BattleChars`),
and after each change the combat's `ccPartyManager` (`Combat::set_party`)
is taken again from `memberID`, so a member who left is no longer the
party's in the fights, the panels or the AI, and one who joined is.

- `party_add pc` (case 77) and the menu's `ccThPartyAdd`
  (`Request::AddMember`): `ccParty::AddMember` (gcmn 0x0059ce80), the
  first slot whose `memberID` is -1 takes `inviteSpc(pc)` (0x005a08e0).
  For a registered id: the registry's `partyFlag` 1, then the character's
  own (+0xe0 bits 14-16) when it is built: -2 (leaving) turns `recallFlag`
  (+0xe1 bit 4) on, -1 (left) turns it off and comes back as 1, 0 becomes
  1. `memberChar` is the character pointer; when it is null the slot keeps
  `memberID` -1 and AddMember returns -1. An unregistered id is registered
  and built at the Chaos Gate when `registryNum` is under 5 (not ported),
  else null. The host sets the new slot's menu face (`SetMenuFace`).
- `party_remove pc` (case 78) and PARTY's Remove and Disband
  (`Request::DelMember(slot)`): `ccParty::DelMember` (0x0059cf60), slots
  above 0 whose `memberID` is not -1: `memberChar` 0, `disbandSpc`
  (0x005a0f50: the registry's `partyFlag` 0; the character -2 with recall
  off if it had been recalled, else -1), `memberID` -1, `num` - 1. The
  character stays where it stands; with `partyFlag` -1 its AI halts.
- The gate's leave (`Request::TransferOut(member)`, 30 k + 5 frames per
  member): `ccSpcChar::TransferOut` on the member's character.
- A leaver the fellow task lets go (`partyFlag` -2 and act 14 set
  `exitFlag`): the next frame `ccThFellow02` wakes to it instead of running
  Main, calls `expulsionSpc(listNum)` (0x005a1070,
  `DelMember(CheckMemberID(id))`) and deletes the character, whose delete
  frees the registry slot unless it is still a member. The port does both
  the frame after, before the combat frame (`FieldWorld::fellows_gone`; the
  town's `TownParty::frame` the same), and drops the character from the
  combat's members and cast.

Event 11 (M111, the church) is the first to use them: its tail (block 25)
fades out and `party_remove 15` takes BlackRose out in the field before
`scene` goes back to Mac Anu. No event script uses `party_add`; M121,
M122, M124, M125, S409 and S410 also `party_remove` members.

## The hacked arrival

After the gate hack's OK (menu 62, [field UI](field-ui.md): `ccGame.setupMode
= 1`, `ccSetGtHack()` sets `gtHackFlag` (main 0x00378a98), then the
warp), the field's set-up runs differently:

```text
ccStartThEvent        ccClearGtHack (0x001b77c0): the flag stays only in a
                      field (or a dungeon of type 8 or 9) entered from a town
ccSndSQLoad           as always
setupMode             stream 107 through ccRequestLoadStreamGateHack(107,
                      game.town, game.field): the Chaos Gate's movie while
                      the files load (stream.md, "The gate hack's movie");
                      the set-up waits for it
ccAddRequestFileListSpc  gateHackingOutID by game.field (19 -> 0, 22 -> 1,
                      ... 91 -> 18, else -1) and x<name>.ccs on the list
                      (GateHackOutFileList; name from GateHackOutCcsName)
rebootSpcManager      ccPlayer::ccPlayer: ghoFlag 1, act 24 (ANM_ctu1hac4b),
                      his body on the collision list, no transfer in;
                      ccFellow::Initialize: hidden (dispSW off, not on the
                      command list), dispWait 65, act 2, body on the list,
                      no transferLag draw (atkDellay's still drawn);
                      ccAI::ccAI: arrivalChatCnt -1 (150 without the hack)
setupMode = 0         at the end
```

Each needs `game.area` 1 (or `WORLD_MAN`'s field type 4 with `game.dungeon`
0) and `areaPrev` 0; Kite's and the fellows' also `gateHackingOutID` not -1
and `volumeNum` below 4 (this disc is 1).

`ccPlayer::GateHackingOut` (gcmn 0x0059bc00) runs from `ccPlayer::Main` in
acts 23 and 24 on `progCtrlFlag` (+0x208):

```text
0  ghoCam = new ccAnm on x<name>'s ANM_str<name>0; its markers
   OBJ_marker_cam1, _target0, _man0 (GetSubstAdrsF); changeCamera(3); one
   _AnimateForward; place; ghoCamArmsT 0; speedRate 1; cameraFlag on;
   stopFlag on, moveFlag off; the blades' dispSW 0
1  _AnimateForward; ended: step 7, the blades' dispSW 3; place
7  act 23 and ghoCamArmsT >= 1: step 8; else ghoCamArmsT + 0.1 (to 1)
8  changeCamera(1); tcam's eye and target the last placed (+0x250, posView),
   cameraParamChange; act 2; restraint off; cameraFlag off; step 0;
   ghoFlag 0; ghoCam deleted
place: ecam's eye at OBJ_marker_cam1 (kept at +0x250), its target at
   OBJ_marker_target0 (posView), Kite at OBJ_marker_man0; then
   cameraParamChange (main 0x00162870: dist, deg and rot from the two)
```

Steps 2-6 (+0x114 to 41 and 111, `speedRate` and +0x224 easing) are not
reached from 1. Act 24's clip ends into act 23 (`AnimCtrl`), which loops.
In acts 23 and 24 `ccPlayer::Main` gives the blades `ghoCamArmsT` as their
transparency. While `ghoFlag` is set `ccThGameCtrl` does nothing past its
check, the fellows' AI does not run, and the event instruction
`gate_hack_anim` (case 157) waits on `ccCheckGtHackAnm`, with `forbid` and
`targetForbid` 1 and the panels and the map closing, all given back after.
Events 18, 25 and 30 wait on it in the hacked area (fields 19, 23 and 27,
from Mac Anu or Dun Loireag).

In the port the session carries `gtHackFlag` and `setupMode` from the
town's menu (`Event::GateHacked`) into the area, whose host clears the
flag by `ccClearGtHack`'s rule; the set-up holds on its last black frame
while `AreaMode` plays the movie (`StreamPlayer::gate_hack`), with
`ResetPause` once the loop has played through (nothing loads meanwhile);
`FieldWorld::entrance` gives the party's constructors the arrival
(`Combat::entrance`) and loads the camera file; `combat::gate_out` is
`GateHackingOut` on the desktop's `ccAnm` player, its markers the
animation's world positions.

## In battle: ccThGameCtrl

`ccThGameCtrl` (gcmn 0x00517800, priority 33) is the task
[Entering The World](field-game.md#talking) describes for the town. In a
field or a dungeon the same loop also keeps `ccGame`'s battle state, chooses
among enemies, and turns the action button on an enemy into the normal
attack. `$s4` is `ccPartyManager`'s first character, read once when the
task starts (it breathes until there is one); `plw` (0x00730300) is the
player. Both are Kite. The loop body (0x005178f0-0x00518a98), each frame
after its `Breath`:

```text
checkPartyAnnihilation (0x0059d080)  as many of ccPartyManager's three down (+0x08
  or compulsionGameOver (0x00378c74)   dead neither 0 nor 5) as its +0x18 count: the game over
ccCtrlRecoveryReq (0x0051a7f0)       the delayed recoveries (below)
cmndSortRoot = 0; ccSortCmnd x 3     the party's, the enemies', the others' lists
the battle state                     unless ccMenu +0x06 menu is 66 (Data Drain)
ccCheckGtHackAnm (0x0059cd60)        ghoFlag set: next frame
menuClrWait (0x00378c78) > 0         one less: next frame
the map button                       (map.md)
ccPlayerMenuCheck (0x0059cd70)       0: next frame
ccLoadDispCheck (main 0x0019c430)    ld +0x19c nonzero: next frame
the task's first 5 frames            next frame
state 0                              the target, then the buttons
states 1-4                           become 5 at once
state 5                              CheckMenuType() == -1: state 0, $s4 +0xe0 bit 0
                                     (pauseSW) cleared
```

`ghoFlag` (0x00378cc0) is set by `ccPlayer`'s constructor when he arrives
by gate hacking and cleared by `GateHackingOut`. `menuClrWait` is 2 after
the events' `menu_clear` (`ccEvent::MenuClr`, main 0x001b25c0).
`compulsionGameOver` is set by `DataDrainMenu`.

### The battle state

`ccGame` +0x58 `inBattle`, +0x5c `inBattleCnt`, +0x60 `inBattleDist`.
`ChangeScene` sets 0, 0 and 2200; the event instruction `battle_ready`
sets `inBattleDist` to -1.0. Nothing else in the executable calls
`SetInBattle`.

```text
if ccCheckTargetTypeId($s4->base->type, $s4->base->id):
    v = 1  if inBattleDist < 0 and game.field (+0x24) is 1-12
        1  if inBattleDist == -2.0
        1  if ccCheckInAreaCmnd(m, 0xe0, 6, inBattleDist) for any m of ccPartyManager's three
        0  otherwise
    ccGame::SetInBattle(v)
else:
    inBattle = inBattleCnt = 0

SetInBattle(v) (main 0x001676a0):
    if inBattleCnt > 0:  inBattleCnt -= 1
    elif inBattle != v:  inBattleCnt = 60
                         inBattle = 2 if inBattle == 1 and v == 0 else v
```

So a fight begins the first frame an enemy on the lists is within
`inBattleDist` of a party member, unless the count from the last change is
still running. After a change, `inBattle` holds for at least 60 frames.
When no enemy is in reach any more, 1 becomes 2 (the fight's end), and 60
frames later 2 becomes 0. With a negative `inBattleDist`, fields 1-12 are
a fight whatever the enemies, and elsewhere any listed enemy counts,
whatever its distance. With -2.0 it is always a fight. `ccThSpc` turns
these values into the party AI's battle condition
([battle rules](battle.md#the-partys-bookkeeping)).

`ccCheckTargetTypeId(type, id)` (gcmn 0x005199e0) is the first listed
character whose base type shares a bit with `type` and whose base id
(+0x0c) is `id`. It walks the party's list for `type & 7`, the enemies'
for `& 0xe0`, and the others' for `& ~0xe7`. Asked for Kite (type 7, id
0, from `charTbl` row 0), it answers whether he is on the lists, unless
another of the party shares his id. `menu_ban` (`ccEvent::MenuBan`,
main 0x001b2460) takes the whole `ccSpcManager` registry off the lists and
puts the party's AI under manual control, so `inBattle` is 0 through an
event's cutscene. `menu_clear` enters them again in registry order, Kite
first (all but a character at act 14), and sets `menuClrWait` to 2.

`ccCheckInAreaCmnd(ch, type, mode, dist)` (gcmn 0x0051a000) answers
whether someone of `type` is within `dist` of `ch` on the ground. For each
other character it takes `d = other.posP - ch.posP` with z 0 and w 1; the
other counts when `dist < 0` or `sqrtf(sceVu0InnerProduct(d, d)) < dist`.
The `mode` decides who is counted: 0 the standing (`dead` 0), 2 the falling
(`dead` 2), 6 anyone, and any other mode no one.

```text
type & 1      the leader (ccPartyManager[0]), when ccCheckTargetTypeId finds his
              type and id; ch itself is not excluded here
type & 6      the party's list from its second character (its head skipped)
type & 0xe0   the enemies' list
type & ~0xe7  the others' list
              each: base.type & type, not ch, counted by mode
```

It borrows 48 bytes of the `ccSys` +0x25c scratch. The party's head is
usually Kite: `ccEntryCmnd` appends, and he is entered before the members
when the party is built, by `ccSPC::Wakeup` and by `MenuClr`. When
`ccPlayer::AnimCtrl` (0x0059a0c8) enters him at the end of an arrival, the
members may already be on the list. Then the head skipped is a member, and
Kite is walked like the rest.

### The target in a fight

`ccSelectTarget(mode)` and `ccCheckTargetRange` are the town's
([Talking](field-game.md#talking)). The fight adds the following:

- **When it runs.** The selection runs only with `cmndTargetFix` 0, the
  player listed (`ccCheckTarget(plw)`) and `ccSkillCheck($s4) < 2`. A
  fixed target is only dropped when it has left the lists. While the
  player is off the lists, nothing re-checks `cmndTarget`: it can point at
  a character that has left them, and the action button still reads that
  character's base type.
- **The player.** `ccSelectTarget` answers none while `plw` is down
  (+0x08), asleep (+0x1a), confused (+0x1c), charmed (+0x1e) or paralysed
  (+0x24).
- **The candidates.** Anyone down (+0x08 nonzero) is passed over. The
  party and the walking PCs (type & 0x0700000f) are passed over when:
  - `inBattle` is nonzero;
  - `game.field` is 1-12;
  - it is asleep, confused, charmed or paralysed;
  - it is a party member (type bit 4) whose act (+0xee) is 12-14, at a
    gate.

  Enemies and everyone else are always kept.
- **The reach.** With `inBattle` nonzero, `ccCheckTargetRange` answers 0
  for type 0x8000. The reach is 150 for type & 0x078400ef (400 in the eye
  view) and 60 otherwise. Outside the eye view, a character beyond 60 plus
  both widths but within the reach plus both widths is range 1 inside the
  1.2 rad cone and range 2 inside 2.0 rad: the wider ring, which only modes
  1 and 2 go through. The eye view has no range 2.
- **The stack arrays.** The candidates in range 1 go into `near[8]` and
  those in range 2 into `wide[8]`: two arrays on the stack (sp+0x50,
  sp+0x70), one after the other, with no bound. A ninth candidate in range
  1 is written over `wide[0]` and read back from there. A ninth in range 2
  is written past the function's frame.

### The buttons in a fight

The order of the checks is the town's
([the menu buttons](field-game.md#the-menu-buttons)). In a fight:

- `ccPlayerMenuCheck` needs the party standing, `ccSkillCheck(plw) < 2`
  and `plw`'s act neither 12 nor 13. `ccSkillCheck` (gcmn 0x005723e0)
  answers the id of the player's first running skill that a hit
  interrupts: 1 is the normal attack, 0 none.
- `ccSpcChar::CheckControlMode(plw)` (0x0059f550) reads the player's AI
  (+0x128) byte 0 bit 0, the events' manual control. When it is set,
  nothing more happens that frame.
- The held and down checks read `$s4`'s condition.

The action button (operation 9):

```text
no cmndTarget                 pauseSW cleared; next frame (the tail below is skipped)
target type in the menu table the menu, pauseSW set, state 5
target type & 0xe0            if ccSkillCheck($s4) == 0:
                                  ccSkillRequest($s4, cmndTarget, 1)    the normal attack
                                  ccMenu +0xf4 plAttack = 1; cmndTargetFix = 1
the tail (and without the     if plAttack and ccSkillCheck($s4) != 1:
action button)                    plAttack = 0; cmndTargetFix = 0
```

So X on an enemy asks for the normal attack and fixes the target, and on
the next frames the stick does not change it. The first frame that
reaches the tail with `ccSkillCheck` no longer answering 1 clears
`plAttack` and the fix: the swing is over, or the request did not go
through (the tail runs in the same frame as the request). While a skill
answers, a press asks for nothing.

### The game over

When the party is wiped out, or `compulsionGameOver` is set, the task
leaves its loop (0x0051791c):

1. The same frame: `compulsionGameOver` = 1, `game.pauseFlag` (+0x7c) =
   1, `ccMenu.forbid` = 1 and +0x0c, +0x10 = 3; `CloseMenu` and
   `ccMessage::Close`. For each of `ccSpcManager`'s five registry
   characters, one standing or reviving (`dead` 0 or 5) is put under
   manual control (`ManualModeAI(1)`, `SetRemoteCmd(ai, 0)`), and every one
   is taken off the lists (`ccDeleteCmnd`). Then
   `ccClearConditionAllEnemy`.
2. The next frame: `CloseChat`, `ccChangeCmndTarget(0)`, `ccSndGameOver`;
   `ccThGameOver` (0x00516a80) is started at priority 33 and `game.status`
   becomes 6.
3. Each frame after that, until `ccThGameOver`'s +0x14 turns positive:
   the registry characters still listed are dealt with as in step 1, and
   the menu's fields are set again.
4. Then `ccGame::InitScene` and `ccDeleteAllThread` (the task keeps itself
   by setting its +0x10 bit 0 around the call). `ccSys` +0x18 = 0, and the
   64-bit words at +0xce0 and +0xdd0 become 0x3f800000_80000000.
   `Breath(2)`, `ccAddRequestFileListGameOver`, `ccThGameOver`'s +0x14 =
   2, and the task deletes itself.

`ccSndGameOver` (SLUS 0x00180580) is the sound's `gameInterrupt` and
`sq_fade(i, 0, 20, 3)` for every sequence slot below `sq_num`: the music
fades out over 20 steps.

### `ccThGameOver` and its noise

`ccThGameOver` (0x00516a80) makes a `ccGameOverNoise` (0xa8 bytes; its
`ccNoiz` draws its bands from newlib's `rand`), then:

1. `Breath` 180 times, then once more before the noise's first frame.
2. `ccGameOverNoise::Main` once a frame until it answers 0, then +0x14 =
   1. `ccThGameCtrl` tears the field down the next frame (step 4 above).
3. It waits for +0x14 = 2, the file list's files (`GameOverFileList`:
   "over"), and 40 more `Breath`s.
4. `ccLayer::Init(129, 0)`, the "over" file's `ANM_xdggame0` (GAME OVER,
   red on black) set on it, the fog off, and the clip run to its end.
5. `ccOpenGameOverMenu` (0x0056a7c0): `ccGame::ChangeRequest(1, 7)`, the
   mother task's soft reset. The game goes back to the title.

`Main` (0x00516d30) runs `NOISE` and, while it is still active, `TV`.
`NOISE` (0x00516d90) steps its phase (+0x10) and frame count (+0x14):

| Phase | Frames | Each frame |
| --- | --- | --- |
| 1 | 51 | with no burst running, `ccRand() % 8 < 4` starts `Noise2`: 0-9 frames of the sampling |
| 2 | 41 | the same odds start `Noise3`: 0-9 frames of the sampling and the inversion |
| 3 | 1 | `ccSys.bgColor` 0 (and in a dungeon `DUNGEON.fog` 0), `Noise`: 30-149 frames of both, and the TV (+0x0c) to 1; the phase to 0 |
| 0 | until the TV is done | the running burst only |
| 4 | after | with no burst, `ccRand() % 8 < 2` starts `Noise4`: 20-29 frames of the sampling |

While a burst runs (+0x04 frames left): `ccSeOn(94)` on every tenth frame
before the TV starts; the two counters at +0x18 and +0x1c are picked
again from `ccRand() % 50` when they run out; `ccNoiz::SetRN(1)` and
`ccNoiz::Draw` (its own layer; [the noise](field-ui.md#the-noise-ccnoiz-0xe8)).

`TV` (0x005170c0) is the television switching off:

- 1: `StretchTV_Y` (0x00517330): the picture's height (+0x88, from 384)
  loses half of itself (at least 1) about the middle. At 1: `ccSeOn(90)`,
  the flash armed, the TV to 2. The flash (`EntryFlash(6, 0x80ffffff, 0,
  0, 512, 384)` on the noise's own `ccScFade` on sysLayer) fires once the
  squeeze (+0x9c, h / 384) is under a half. `InitData` leaves it armed,
  so it fires twice: at the second squeeze and at the line. Only
  `StretchTV_Y` sends the fader's packet, so a flash shows only while the
  picture is squeezing. sysLayer's view is `SetFrame(0, y, 512, h, 256,
  h / 2, 1, h / 384)`.
- 2 to 4: a frame each.
- 5: `StretchTV_X` (0x005174d0): the width (+0x84, from 512) loses a
  quarter (at least 1) about the middle; at 1, the TV to 6. The view is
  `SetFrame(x, y, w, h, w / 2, h / 2, 1, w / 512)`.
- 6: `StretchTV_Init` (0x005175b0) and done: `Main` answers 0 from here,
  and `NOISE` goes on at phase 4.

With the noise's odds, the TV starts 93 frames into the noise, the
picture is a line 9 frames later and a dot 22 frames after that: about
5 seconds from the signal to the black screen, and 3 more to the title.

### The delayed recoveries

`recoveryReq` (gcmn 0x0072eb10) is 16 slots of 12 bytes:

```text
+0x00  ccChar *ch   null when free
+0x04  int amount   written as a word, read back as a short
+0x08  int count
```

- `ccInitRecoveryReq` (0x0051a750) frees every slot when the task starts.
- `ccEntryRecoveryReq(ch, amount)` (0x0051a790) is called by
  `CalcBattleDamage` when a hit's element heals its target. It takes the
  first free slot with count 10, and drops the request when none is free.
- `ccCtrlRecoveryReq` runs once a frame. A slot whose character is off the
  lists or down is freed. Any other slot counts down, and at 0 calls
  `EntryAffect(ch, ch, 7, (short) amount, 0, 0)` and is freed.

### The battle half in the port

`piney-world`'s `talk`:

```text
Targeting::frame      the loop body: leader and candidates, talk::Input, talk::Battle
                      (the leader's list entry, id, condition, act and manual control,
                      ccPartyManager, ccMenu.menu, CheckMenuType, ld, ghoFlag,
                      compulsionGameOver), &mut InBattle, &mut dyn talk::Host
                      -> Out { step: Ctrl, pl_attack, pause }
Ctrl                  Action (a menu on the target), Open (a button's menu),
                      Attack (the normal attack was requested), GameOver
Host                  ccEvent::CheckOperate, ccSkillCheck(plw), ccSkillRequest(plw, t, 1)
                      (called where the game calls it, so the tail sees the request),
                      ccCtrlRecoveryReq
Targeting::step       the town's call of frame (Kite alone; a menu's wait ended by close_menu)
select_target, Scope  ccSelectTarget with the player's state, inBattle and the field
in_area, type_id_listed, battle_now, annihilated, InBattle (set, battle_ready)
RecoveryReqs<T>       recoveryReq: entry, ctrl
menu_clear            menu_clear's menuClrWait = 2
```

The candidates are the three lists in order without the leader.
`LeaderState::at` is his place on the party's list (0 the head), which
only `in_area` reads. A target that has left the lists answers the action
button with the base flags it had when it was chosen.

### Checks for the battle half

`tools/test_gamectrl_rs.py` runs the `gamectrl_probe` example beside the
game's own code in the interpreter, over command lists laid out as
`ccEntryCmnd` links them. No case or frame differs:

| check | what | cases |
| --- | --- | ---: |
| `SetInBattle` | random `inBattle`, `inBattleCnt` and values | 2,000 |
| `ccCheckInAreaCmnd` and `ccCheckTargetTypeId` | random crowds (party, enemies, objects, some down or falling), with Kite listed or not, at the head of the party's list or further down; the centre the leader, a listed character or an unlisted point; every type bit; modes 0, 1, 2, 3, 6; negative, -2, 0 and ordinary distances. 516 inside, 789 found | 3,000 |
| `ccSortCmnd`, `ccCheckTargetRange`, `ccSelectTarget` | in and out of a fight; fields 1-12 and others; conditions on candidates and the player; members at a gate; crowds of 10-19 (the arrays' overlap). 941 with a target | 3,000 |
| `ccThGameCtrl`, frame by frame | 500 runs of a fight: enemies closing in, falling and leaving the lists; the party following; conditions coming and going; the buttons; menus opening and closing (a small model of `ccThMenu`); the skill check before and after the request; `menu_clear`, loading, gate hacking, `inBattleDist` changes; Kite leaving the lists and coming back first or last; delayed recoveries | 104,132 frames |
| the same | 600 runs with Kite's conditions and `plAttack` changing often | 102,897 frames |
| the same | 1,500 short runs where the party falls: the game over's frame and the next | 61,523 frames |

Across the three `ccThGameCtrl` runs, the paths taken:

- 3,788 normal attacks asked for (757 of them cleared in the same frame,
  the request not taken);
- 4,397 attack flags cleared by the tail, 6,647 while held, 1,577 while
  down;
- 1,575 game overs (991 of the party, 584 forced);
- 274 menus on a target and 6,525 from the other buttons;
- 11,756 delayed recoveries landing;
- frames waiting on `menu_clear` 4,061, `ghoFlag` 5,325, loading 5,342, a
  skill 31,140, Kite at a gate 12,409; under manual control 8,112; with
  Kite off the lists 9,565.

Each frame compares `cmndTarget`, `cmndTargetPrev`, `cmndTargetPriNum`,
`cmndTargetFix`, `plAttack`, pauseSW, `inBattle`, `inBattleCnt`, the menu
asked for (`openReqNum` and `mode`, by `firstTime`), each `ccSkillRequest`,
each recovery's `EntryAffect`, the `cmndSortRoot` chain and the game over.
`ccEvent::CheckOperate`, `ccSkillCheck` and `ccSkillRequest` answer from
the harness's plan, so the checks cover the task's control flow around
them, not piney-battle's skill list.

Unknown or left out:

- The map button (select, operation 13) is [the map's](map.md); `frame`
  leaves it out.
- The game over: the port does not put the party under manual control
  (`ManualModeAI`, `SetRemoteCmd`) or run `ccClearConditionAllEnemy`, and
  leaves `DUNGEON.fog` as it is at the TV (the frame is cleared black and
  the field is gone within 40 frames). The view's squeeze moves the whole
  finished frame (every layer, not only those drawn through sysLayer's
  view) into the band, in frame-buffer pixels.
- A stale target is read through the flags it had when chosen. The game
  reads the character's memory as it is then: the same while the character
  exists, unknown once it has been freed.
- `wide[]` past its eighth entry writes into the caller's frame, which
  `ccThGameCtrl` never reads back. The port models only what
  `ccSelectTarget` reads back of its own writes.

## The port

```
piney_data::area       Go (SetGenerateCode's call), flag71
piney_world::area      Scene (ccGame's scene, ChangeScene, ChangeArea, go),
                       WorldMan (SimGenerateCode's fields; set_generate_code)
field_area             FieldArea: GO(1)'s field, its objects, the hit list,
                       the object passes, WORLD::Draw into the layers,
                       SetCenter / AddCenter, the wrap; the weather's frame
                       and models (field_ambient, field_firefly)
field_world            FieldWorld: ccSetupGameCtrl for areas 1 and 2, the
                       frame, Enter, Gate Out, the dungeon's doors; the
                       battle's tasks in their order (combat), the
                       effects' hook (FieldFx)
combat                 Combat: the battle over the area (Stage, the world
                       traits; Cast, the animation players; spc, the
                       SpcRef adapter), battle.md's "How it plugs into
                       piney-world"
foe                    the enemies' and the portal's looks and ccChar::Draw
dungeon_area           DungeonArea: GO(2)'s dungeon, its rooms and doors
evarea                 EventArea: area 15's story map, EVENTAREA02 (its
                       own page), and Kept (what WORLD_MAN keeps between
                       scenes)
hit                    the list with rm/im per hit, ccLandHitCheck's field
                       path, the height map's attribute
player                 MapLoopAdjustPos, W2MPos, the entrance bit
gameover               GameOverNoise: ccGameOverNoise's Main, NOISE, TV
                       and the view frame it leaves
piney-game area.rs     AreaMode: the field's mode 6 with the field's menus,
                       the party's panels, the event task and its set-up
piney-game area_host.rs  the event scripts' host outside the towns
piney-game fx.rs       AreaFx: piney-effect over the battle (the starters,
                       ccThEffect, ccThParticle, the portals' draw, the
                       numbers); fx/ambient.rs the weather's sprites and
                       smoke
piney-game gameover.rs  GameOverTask: ccThGameCtrl's second step and
                       ccThGameOver (the waits, the noise, the tear-down,
                       GAME OVER, the reset); squeeze, the view frame
                       applied to the field's frame
piney-game session.rs  the change of scene: the fade out over the old mode,
                       then the next; SetGenerateCode from the gate's words
```

`piney-game --mode world` plays event 2 in Mac Anu (the field host,
[event VM](event-vm.md)); its `area 14` and `scene` reach the session,
which fades Mac Anu out, makes story area 14's `WORLD_MAN` (instruction
118 on the town's server) and sets the field up. The Chaos Gate's warp
(`Request::GoToArea`, `SetGenerateCode`) and Gate Out go the same way.
`--mode field:14` starts on a new game's save in story area 14's field (as
event 2 leaves it), `--mode field:14/TYPE,ROW[,HACK[,SEED]]` with another
field type, background row, hack flag and seed in its `WORLD_MAN`,
`--mode dungeon:14` in its dungeon's first room;
`--press F:gateout` confirms Gate Out on frame F, `F:gofield` takes
TransFieldMenu's way out of a dungeon. Log Out (`ChangeRequest(4, 7)`)
leaves a field or dungeon for the top page as it does the town.

## Checks

`tools/test_field_rt.py` lays the field the probe makes (story area 14's)
out in the interpreter's memory the way `WORLD::Generate` leaves it - the
height map, the chips, the `FOBJECT`s and `FOBJECT2`s - with one
`ccModelHit` per Hit chunk decoded by the game's `Decode_Hit`, and runs:

- `WORLD_MAN::SetCharPosition` for every field type from a town and from
  the dungeon, with the three members' places, and in a dungeon the
  members' places round 40 random positions and facings;
- `ccLandHitCheck` with `game.area` 1 at 1,500 points on, beside and away
  from the objects, every object's hits placed (each model's hit so at the
  last object of it), and on the bare height map: z, the result count, the
  nearest result's attribute and point, and `checkHitResultAttlibute`. The
  hit meshes are mostly walls: 36 of the points land on one (the
  entrance's floor, the tops of plinths and rocks), the rest on the height
  map;
- `WORLD::DrawObject` and the entrance and key pass for 300 player places
  across the field and its edges, one after another: each object's place,
  fade, draw, anm steps, and the hit list left behind, order and matrices;
- frame by frame, `cameraMain`, `ccPlayer::Main` and the object passes from
  Kite's arrival at the start: walking and running (320 frames), to the
  dungeon entrance, `WORLD_MAN::Enter` from frame 370 and on down its
  steps (420), three runs of random pads from random places (360 each),
  and running off each of the four edges (200 each): everything the town's
  check compares, plus Enter, the centre and the hit list - 2,620 frames,
  none differing.

`piney-game`'s `event_3_opens_in_the_field` plays a new game's Log in
through event 2 (the pad as the town's test scripts it) into the field:
the party [0, 2, -1] with Orca at Kite's start + (200, 100), event 2
done, and Orca's lines 2-6 of event 3 opened; `event_3_log` prints every
call event 3 makes; `the_camera_lesson_moves_the_camera` holds L1, the
right stick and R2 through the three prompts and asserts the view turns,
zooms and turns back. These are the runtime's checks, not the game's.

`tools/test_area_rs.py` holds `WORLD_MAN::SetGenerateCode` against the
game for every story area's words and 300 random triples: the call it
makes (`ChangeArea(1, n)`, `ChangeArea(2, 0)` or `ChangeScene(2, -2, n, 0,
0, 0)`).

`tools/test_party_rs.py` runs `ccParty::AddMember`, `DelMember` and
`expulsionSpc` (with `inviteSpc`, `disbandSpc` and `CheckMemberID`)
natively over 600 random registries, parties and characters' +0xe0 words,
one to five calls each (adds of registered ids, and of unregistered ids
while the registry is full; slots -1 to 2; every registered slot's
expulsion), against the world_probe's `party` request on party.rs's
`Spcs`: every call's return, the registry's ids and `partyFlag`s,
`memberChar`, `memberID`, `num` and each character's `partyFlag` and
`recallFlag`, none differing.

`tools/test_gate_out_rs.py` runs the game's `ccClearGtHack` (240 cases:
area, `areaPrev`, `game.dungeon`, its dungeon type), `ccAddRequestFileListSpc`
(an empty registry; 2,376 cases of area, `areaPrev`, field type, dungeon,
field and `gtHackFlag`: the ID, whether it was looked at, the file listed)
and `ccPlayer::GateHackingOut` frame by frame (400 runs of up to 60 frames
from step 0 and from later steps, all nine reached, with the camera
functions and `cameraParamChange` the game's and `ghoCam` scripted:
whether it is set, when it ends, the markers each frame) against the
world_probe's `gtclear`, `ghoid` and `gho` requests: none differ.

`piney-game`'s `gate_hack_arrives_hacking_out` hacks Mac Anu's gate to
area 19 (story:19) with Mia and Elk put in the party as the area starts:
the movie plays `str7100`, `str7200`, `str7300` and `str7404`;
`clear_gate_hack` keeps the flag; Kite goes through acts 24 and 23 under
`ecam` with his blades hidden, then fading in, then stands (act 2) with
`tcam` back and `ghoFlag` clear; the members start hidden, off the command
list and standing, and appear on their 65th frame. `gate_hack_arrival_shots`
(ignored) takes pictures of it.

`piney-game`'s `event_11_church_ends_with_blackrose_out` plays event 11
from the join through the holy ground and the church: BlackRose is in the
party in the field, `party_remove 15` leaves her registry `partyFlag` 0
and her character's -1 with the party without her, the party the field
hands back to the session is without her, and no instruction fell to a
host default (`take_unported` empty).

## Unknown

- The entry control in a field holds `EntryGimmick`'s portals, enemies,
  foods, a lake's spring, the entrance's swirls and the symbols, and the
  event's entries ([battle.md](battle.md#where-the-entries-come-from)).
- The water's run-time textures and the sky's clouds' blending are
  approximations. (The fog is VU1's by each vertex's depth, as the game's:
  `piney_draw::DepthFog`.)
- Of the story maps of their own (`EVENTAREA_INFO.model` not 0, an
  `EVENTAREA` class) areas 15's and 16's and the boss arenas are built
  ([their page](evarea.md)); the others, which no Infection script
  reaches, get a generated field.
- A `HIT_` node's local matrix is taken as the identity inside its model.
- Where no result qualifies, the ground attribute's type nibbles come from
  a stale register holding a heap address; they cannot be matched.
- `SimGenerateCode` also moves the global RNG (`seed`, `randcnt`), which
  the port's area words leave aside; so does `GO` (`fieldrand`).
- Of the Root Towns, Mac Anu (Δ) and Dun Loireag (Θ, [town02.md](town02.md))
  are built; `ROOTTOWN03`-`05` belong to the later volumes (GAPS.md, "After
  Infection").
- `ccPlayer::CollisionTest` guards `WORLD_MAN::Enter` only with `dneFlag`,
  so the game may call it again on the frames of the fade out while Kite
  still stands on the entrance or a door; the port takes the first change
  of scene and lets the dungeon's `GotoNextRoom` run again as the game
  would. Whether the set-up stops the old tasks sooner is not traced.
- `WORLD_MAN+0x58[n]`, which `Enter` and `GoField` clear for the dungeon
  being left (`game.dungeon`) on a lake's way between its dungeons: what
  reads it is not traced. (`GoField` for field type 4, the second
  dungeon back to the first at `lastRoom` +0x100, is ported; `Enter`
  sets `lastRoom` from `game.block` on the lake's stairs down: dungeon.md.)
- `inviteSpc`'s build of an unregistered character at the Chaos Gate is
  ported for the towns ([field game](field-game.md), `party_add`); its
  copy of the character table's equipment into the registry (+0x10 -
  +0x16 from `ccGetCharParam` +0xc8 - +0xce, then `ChangeWeapon` with
  +0xd0) is not: the port builds the character from the save. (The
  registered characters outside the party are built at the origin and
  placed by `ccEntryEventMng`, below; the party's own following and
  fighting in the fields is the battle's `ccAI`, [battle](battle.md).)
- The hacked arrival's load time: the game holds the Chaos Gate's loop
  (`str7300`) for as long as `ccLoadResourceFL` reads the field's files; the
  port loads nothing then and ends the loop after one pass. How long the
  game's load takes is not measured.
- `ghoCam`'s markers are read off the desktop's `ccAnm` player (glam's
  matrices, not VU0's): `GateHackingOut`'s own logic is checked against the
  game with scripted markers, the camera file's animation is not.
- The event thread's wake is read from the thread library (the no-sleep
  bit); the frame it wakes on is not run against the game.
