---
title: The story maps - EVENTAREA01 (area 13), EVENTAREA02 (area 15), EVENTAREA03 (area 43), EVENTAREA07 (area 16), EVENTAREAB0 (the boss arenas) and EVENTAREAB8 (Kyvia's disc)
status: partial
volumes: INF, MUT
covers: INF gcmn.prg:0x00405a00 EVENTAREA07::EVENTAREA07, 0x00405c80 EVENTAREA07::ChangeBlock, 0x00406b80 EVENTAREA07::Draw, 0x00406730 EVENTAREA07::DrawBG, 0x004068b0 EVENTAREA07::DrawObj, 0x004069a0 EVENTAREA07::DrawObj2, 0x00406a90 EVENTAREA07::DrawFloor, 0x005d2160 EA_MODELTABLE07, 0x005d2200 EA_OBJTABLE07, 0x005d2230 EA_BGNAME07, 0x005d2240 EA_moveTex07, 0x005d2270 EA_MODELTABLE0702, 0x005d22a0 eventarea0702Light, 0x005cff60 STATICOBJECT::SetPos, 0x005d03f0 STATICMODEL::SetPos, 0x004079f0 EVENTAREAB0::EVENTAREAB0, 0x004095e0 EVENTAREAB0::Draw, 0x004091c0 EVENTAREAB0::DrawBG, 0x00409460 EVENTAREAB0::DrawObj, 0x00409510 EVENTAREAB0::DrawFloor, 0x004095b0 EVENTAREAB0::SwitchLayer, 0x004098e0 EVENTAREAB0::NextStage, 0x005d2580 EA_MODELTABLEB0, 0x005d2640 eventareaB0Light, 0x005d26e0 bossFireFlyEff, 0x005d2730 the fireflies' markers (@1153), 0x005b4550 FIREFLY2::FIREFLY2, 0x005b4d30 FIREFLY2::Init, 0x005b49f0 FIREFLY2::SetBasePosition, 0x005b4ba0 FIREFLY2::SetBasePosition2, 0x005b5050 FIREFLY2::Move, 0x005b5eb0 FIREFLY2::Draw, 0x0059b940 ccTransPosW2P, 0x00401e00 EVENTAREA02::EVENTAREA02, 0x00402300 EVENTAREA02::ChangeBlock, 0x004033e0 EVENTAREA02::Draw, 0x00402ff0 DrawBG, 0x00403280 DrawObj2, 0x00403190 DrawObj, 0x00403340 DrawFloor, 0x00402dc0 DrawLensFlare, 0x00400f00 EVENTAREA::EVENTAREA, 0x004010a0 EVENTAREA::SetCenter, 0x004010b0 EVENTAREA::AddCenter, 0x005d1c00 EA_MODELTABLE02, 0x005d1c50 EA_OBJTABLE02, 0x005d1c70 EA_MODELTABLE0202, 0x005d1d00 EA_OBJTABLE0202, 0x005d1d50 EA_moveTex02, 0x005d1d20 eventarea0202Light, 0x005d02d0 STATICMODEL::DrawWithOutFog, 0x005d01c0 STATICMODEL::Draw, 0x00571da0 initHitCheck, 0x00403590 EVENTAREA03::EVENTAREA03, 0x00403bf0 EVENTAREA03::Draw, 0x00403ab0 EVENTAREA03::DrawBG, 0x00403b50 EVENTAREA03::DrawFloor, 0x00639548 kiteSelfTalk; MUT gcmn.prg:0x00416590 EVENTAREA03::EVENTAREA03, 0x00416bf0 EVENTAREA03::Draw, 0x00416ab0 DrawBG, 0x00416b50 DrawFloor, 0x006000f0 EA_MODELTABLE03, 0x00600140 its start, 0x00600170 the fly-over's camera, 0x00669470 kiteSelfTalk; MUT gcmn.prg:0x0041d200 EVENTAREAB8::EVENTAREAB8, 0x0041ed30 EVENTAREAB8::Draw, 0x0041e8c0 its ride, 0x0041e420 DrawBG, 0x0041e5e0 DrawObj, 0x0041e6b0 DrawFloor, 0x0041f130 DrawRock, 0x0041cc80 SetFloatRockParam, 0x0041e750 Move, 0x0041e7d0 IsMove, 0x0041e7e0 HitDisable, 0x0041e850 HitEnable, 0x00600ab0 EA_MODELTABLEB8, 0x00600ad0 EA_OBJTABLEB8, 0x00600af0 rockname, 0x0073eb00 floatRockPos; MUT SLUS_205.62:0x0038afc0 eventareaB8Light, 0x001b8bb0 WORLD_MAN::SetTransMode, 0x001b8bd0 WORLD_MAN::SetTransCenter; MUT gcmn.prg:0x00414110 EVENTAREA01::EVENTAREA01, 0x00414c00 EVENTAREA01::Draw, 0x00414900 DrawBG, 0x00414980 DrawObj, 0x00414a70 DrawObj2, 0x00414b60 DrawFloor, 0x005ffd80 EA_MODELTABLE01, 0x005ffe10 EA_OBJTABLE01, 0x005ffe30 eventarea01Light, 0x005ffeb0 the glows' dummies; MUT SLUS_205.62:0x0038af98 eventarea03Light; INF SLUS_202.67:0x0019f8e0 WORLD_MAN::GO (EVENTAREA branch), 0x0019dda0 WORLD_MAN::Enter (the story maps' doors), 0x001a1ea0 WORLD_MAN::SetStartPos, 0x001a1190 WORLD_MAN::SetCharPosition (event area), 0x0019c740 WORLD_MAN::Quit, 0x0019f2d0 WORLD_MAN::DeleteEvent, 0x001a20b0 WORLD_MAN::SetActiveLayer, 0x001a10c0 WORLD_MAN::GetHeight, 0x0019c4b0 MoveTexture, 0x00377ff0 eventarea02Light, 0x0013bb40 ccEff::SetRenderState, 0x00161590 cameraGetPos, 0x00161700 cameraGetRot2, 0x00138f80 ccLight::~ccLight
worklog: 84, 90, 91, 112, 117, 130, 177, 263, 264, 272, 386
---

# Area 15's story map - EVENTAREA02, Hidden Forbidden Holy Ground

Most story areas are generated fields ([Field generation](field.md)). A
story area whose `EVENTAREA_INFO.model` is not 0 is a hand-built map
instead, an `EVENTAREA` subclass that `WORLD_MAN::GO(1)` makes in place of
the `WORLD`. Area 15, Hidden Forbidden Holy Ground, where event 11 ends
the opening arc, is `EVENTAREA02`: the scene file `se1_2` in two blocks,
0 the holy ground (the bridge up to the church) and 1 the church's nave.
A door in each block's floor changes block.

The class is built like a town ([Town and event-area assembly](statics.md)):
static models and objects from tables in `gcmn.prg`, background clumps,
the lights of an animation, and the collision carried by one of its
models. The rest of the field's machinery - `ccSetupGameCtrl`, the player,
the camera, the battle's tasks - runs as in a generated field
([Leaving the town](field-walk.md)), with `WORLD_MAN.eventAreaFlag` set.

## GO(1) for a story map

`WORLD_MAN::GO(1)` (main 0x0019f8e0) sets `eventAreaNumber = game.field`,
looks its `EVENTAREA_INFO` up, sets `defSE` by the field type, then:

```text
game.field 0, or info.model 0     the generated field (seed, WORLD, Init,
                                  Generate; eventAreaFlag 0)
else                              eventAreaFlag (+0x124) = 1; position 0;
                                  bounds -48000..48000 both ways; unless
                                  eventmap (+0x444) is set already, by
                                  eventAreaNumber (0x001a0880):
  13 EVENTAREA01    15 EVENTAREA02    16 EVENTAREA07    43 EVENTAREA03
  66 EVENTAREA04    67 EVENTAREA05    91 EVENTAREA06
  9-12 EVENTAREAB8  1-8 EVENTAREAB0   any other: a store to address 0
```

No seed is set and nothing is generated. `ccThFieldDisp` (0x001a4430) then
calls `eventmap->Draw()` each frame instead of `WORLD::Draw`, and no
minimap is drawn. `ccSetupGameCtrl` loads sequence bank 5, the story
area's event bank, instead of 3 when the scene changed
(`GetEventAreaInfo(game.field)->model` 1, 0x00169168).

## The constructor

`EVENTAREA02::EVENTAREA02` (gcmn 0x00401e00), after the base
`EVENTAREA::EVENTAREA` (0x00400f00: the vtable, `bg[0..5]` cleared):

```text
ccs = GetCCSAdrs("se1_2")
cc3d->SetFog(1500, 12000, 0, 70, 0x1e1e1e)     (ChangeBlock sets its own)
ccSys.bgColor (+0x18) = 0x1e1e1e, with the GS values at +0xdd0, +0xce0
modelArray[50], anmArray[25], bg[0..5] cleared; lightNum, lgtAnm, revAnm 0
ChangeBlock(0)
DMY_marker01: chunk +0x1c = chunk +0x28     (the turn about z into w)
WORLD_MAN::SetStartPos(chunk + 0x10)
effccs = GetCCSAdrs("town_z")
LensFlare[0..5] = new ccEff, Init(EFF_sflenz_2 .. _6, _1; fogSw 1)
each: SetRenderState(CCRS_ZENABLE, 0); PRIM (+0x62) bit 0x20 (fog) off
```

`WORLD_MAN::SetStartPos(pos)` (0x001a1ea0) copies `pos` to
`eventStartPos` (+0x70) when `WORLD_MAN.flag` is 1, `game.field` is not 0
and `eventAreaFlag` is 1; otherwise to `fieldStartPos` (+0x450).

## ChangeBlock

`EVENTAREA02::ChangeBlock(block)` (gcmn 0x00402300) sets `blk` (+0x1e0),
deletes the block before's pieces - the 50 `STATICMODEL`s, 25
`STATICOBJECT`s, `bg[0..4]`, `lgtAnm` and `revAnm` - and builds the new
block's. Block 0 is the holy ground; any other block is the church.

| | block 0 | block 1 |
| --- | --- | --- |
| `SetFog` | (250, 7800, 0, 90, 0x323232) | (10000, 15000, 0, 90, 0x323232) |
| `modelNum`, table | 4, `EA_MODELTABLE02` | 7, `EA_MODELTABLE0202` |
| `anmNum`, table | 0, `EA_OBJTABLE02` | 1, `EA_OBJTABLE0202` (`ANM_se1_2la1a`) |
| background | `bg[3]` bg1_1, `bg[2]` wa1_1, `bg[1]` cl1_1, `bg[0]` cl2_1 | `bg[0]` bg1_2 |
| light animation | `ANM_se1_2bac1a` | `ANM_se1_2bac2a` |
| lights (`eventarea02Light`, `eventarea0202Light`) | `LGT_se1_2lig1` (distant) | `LGT_se1_2lig2` (distant), `LGT_omni01`-`03`, `LGT_omni13` |
| `SetStartPos` | `DMY_marker02` (0, 2700, 200), facing 0 | `DMY_marker01_2` (0, 900, 0), facing -pi |
| `revAnm` | none | a new `ccAnm` of `ANM_se1_2la1a` |
| collision | `HIT_se1_2fl1_1hit` on `MDL_se1_2fl1_1` | `HIT_se1_2fl1_2hit` on `MDL_se1_2fl1_2` |

The background clumps each get `SetFogSw(0)`. The light animation is a
`ccAnm` with `SetLightEnv(1)`, `SetAnm`, then one `_AnimateForward`:
`GetAmbient` gives `cc3d`'s ambient, and each light of the table is its
`GetSubstAdrsF` object added to `cc3d`'s light group, a distant one (type
0) also handed to `WORLD_MAN::SetLightDirection`. `BLT_bg`, `BLT_obj`,
`BLT_obj2` and `BLT_floor` become the four texture groups.

`EA_OBJTABLE02` has one row (a clump named `MDL_`), but block 0's
`anmNum` is 0, so it is never built.

The delete loop over the lights runs from `lightNum - 1` down to 1, so it
never deletes `lightList[0]`. It does not matter: deleting `lgtAnm`
(`ccAnm::DeleteAnmIndex` 0x0014fe10) destroys its light objects, and
`ccLight::~ccLight` (0x00138f80) takes each out of its group.

A `STATICMODEL` (0x005cffd0) enables its model's `ccModelHit`
(`HitEnable(0)`, at the identity) as it is built, and the model's
destructor destroys it (`ccModelHit::~ccModelHit` 0x001536f0 takes it off
the list): each block's collision is its floor model's one Hit chunk.
`se1_2` also holds `MDL_se1_2do1_1` and `_2`, which no table names.

## Draw

`EVENTAREA02::Draw` (gcmn 0x004033e0), from `ccThFieldDisp`:

```text
DrawBG                         block 0: MoveTexture(EA_moveTex02[0], [1]);
                               bg[3] on bgLayer[3] (-70), bg[2] on bgLayer[2]
                               (-80), SetActiveLayer(3): bg[1], bg[0] on
                               effLayer (20); block 1: bg[0] on bgLayer[0]
                               (-100); each SetMatrix_PosRotZYX(0, 0), Draw
SetActiveLayer(2)  obj2Layer (-10)
  DrawObj2                     the type-3 rows: Draw, but modelArray[2]
                               DrawWithOutFog
  DrawObj                      the type-2 rows, then the type-2 static objects
  revAnm                       _AnimateForward(frameSpd);
                               SetMatrix_PosRotZYX((0, 5400, 0, 0),
                               (0, 3.14, -3.14)); Draw
SetActiveLayer(1)  objLayer (0)
  DrawFloor                    the type-1 rows
  the type-0 rows (none in either table)
SetActiveLayer(3)  effLayer (20)
  blk 1 and not eventMng.puppetShow (+0x78c): DrawLensFlare
```

`WORLD_MAN::SetActiveLayer(n)` (main 0x001a20b0) goes through a jump table
(0x00355790): 0 `sysLayer`, 1 `objLayer`, 2 `obj2Layer`, 3 `effLayer`
(+0x4c8), 4 `floorLayer` (+0x4c4), 5 `charLayer`, 6 `refLayer`. `GO`
gives the layers these priorities:

```text
bgLayer[0..9]  -100 -90 -80 -70 -60 -50 -45 -40 -35 -30
objLayer 0   obj2Layer -10   floorLayer -20   charLayer 10   effLayer 20
refLayer 30
```

`STATICMODEL::Draw` (0x005d01c0) draws its model at `sceVu0TransMatrix` of
the unit matrix by its position, with fog (draw flags 0x2c).
`DrawWithOutFog` (0x005d02d0) takes the position relative to the player
(`ccTransPosW2P`), and draws only while `sqrt(x x + y y)` (in double) is
under `clip` or `clip` is 0, without fog (flags 0x0c). Block 0's
`modelArray[2]` is the mountains, `MDL_se1_2mou1`; every clip is 100,000.
`STATICOBJECT::Draw` steps and draws its animation only within its clip of
the camera eye ([statics](statics.md)).

`MoveTexture(t, ccs)` (main 0x0019c4b0) adds `u_step` and `v_step` to `u`
and `v`, each back to 0 once past 1, and writes `vftoi12(u)` and
`vftoi12(v)` less the material's crop into the material `matname`
(`GetSubstAdrsF`) at +0x14 and +0x16. `EA_moveTex02` scrolls
`MAT_se1_2cl1` (the clouds) and `MAT_se1_2wa1` (the water) in V by 0.01 a
frame. The table is `gcmn.prg`'s own data, so the scroll lives as long as
the overlay does, across maps.

`revAnm` is the church's lanterns once more, turned over under its floor.

### The lens flare

`DrawLensFlare` (gcmn 0x00402dc0) draws the six flares toward
`DMY_se1_2lig1point` (6000, 23600, 4700), block 0's sun. For flare `i`,
with `s[] = (0.2, 0.3, 0.5, 0.55, 0.75, 0.8)` (`@1260`):

```text
sun   = the dummy's position (w 1)
eye   = cameraGetPos(camID)
rot   = cameraGetRot2(1) if checkCameraType() is 1 (the eye view), else
        cameraGetRot(1): tcam +0x40 or +0x20, all four lanes (the
        argument is left at 1 by the test, whatever camID is)
a     = cameraGetView(camID) with x 0, y -2500, z 0 (w kept)
a     = sceVu0ApplyMatrix(sceVu0RotMatrix(unit, rot), a) + eye   (4 lanes)
a.z  += 500
v     = sun - a; v.x *= -0.2; v.y *= -0.2; v.z = 0; v += a; v.z = -200
pos   = (v - sun) * s[i] + sun
LensFlare[i]->Draw(pos, 0)
```

The flares have no depth test and no fog, and `effLayer` comes after the
map's other layers: they are drawn over the church.

## The ground

`ccLandHitCheck` (gcmn 0x00571e00) in a field falls back on
`WORLD_MAN::GetHeight` only when `CheckEventArea()` (`eventAreaFlag`) is 0;
on a story map a miss answers the point's own z and leaves no result.
`WORLD_MAN::GetHeight` (main 0x001a10c0) answers 0 there, which is what
the camera's `avoidObstacle` sees. `WORLD_MAN::SetCenter` and `AddCenter`
call `EVENTAREA::SetCenter` and `AddCenter` (`cx`, `cy` at +0x08; no wrap).
The -48000..48000 bounds are never reached, so `ccTransPosW2M` never
moves a point and `ccTransPosFW2LW` changes a point only by the rounding
of `(p - player) + player`.

## Where the party stands

`WORLD_MAN::SetCharPosition` (main 0x001a1190) in a field with
`game.field` set and `eventAreaFlag` 1 stands the leader at
`eventStartPos`, facing its w. The others, facing as he does:

```text
fields 16, 66      (+200, -150)   (-200, -150)   (0, +300)
field 12           the leader at (82, -11122, 910) facing pi whatever
                   eventStartPos says; the others at (-54, -11525, 910),
                   (256, -11519, 910) and (0, +300) from him
any other (15)     (+300, +150)   (-300, +150)   (0, +300)
```

Kite arrives (act 13) from a town. `ccPlayer::ccPlayer` (gcmn 0x00597910)
in a field takes act 13 only when `game.areaPrev` is 0 (or act 24 for a
gate hacking); coming from another field - the church's door - or from a
dungeon he stands (act 2) with his body on the collision list at once.

## The doors

`WORLD_MAN::Enter` (main 0x0019dda0), which `ccPlayer::Main` calls on
ground with attribute bit 0x80000, in a field with `game.field` set and
`info.model` 1:

```text
game.field 0, or info.model not 1    position cleared, ChangeArea(2, 0) (a
                                     generated field's dungeon entrance)
eventAreaNumber 15    game.block 0 or -1: ChangeBlock(1),
                        ChangeScene(-2, -2, -2, -2, -2, 1)
                      else: ChangeBlock(0), ChangeScene(-2, .., -2, 0)
16, 91                game.block 0 or -1: ChangeArea(2, 0)
                      else: EVENTAREA07::ChangeBlock(0), ChangeScene(.., 0)
66                    ChangeArea(2, 0)
67, any other         nothing
```

The doors are in the floors' Hit chunks, attribute 0x20686f6f: block 0's
at x 0..190 from y 3575 in the church's doorway (the floor narrows to it
from the bridge), block 1's from y 176 down to -600 at the nave's foot.
Kite changes block where he stands, and the fade out of the change runs
over the new block's geometry: he lands on the church's floor far from
its door, so `Enter` is not called again.

The set-up that follows keeps the map. `ccDeleteAllThread` deletes
`ccThFieldDisp`, whose delete calls `WORLD_MAN::Quit` (0x0019c740). With
`game.area` already the new area's, `Quit` deletes the layers and:

```text
new area 0 (town)      ROOTTOWN, WORLD, eventmap (DeleteEvent), dungeons
new area 1 (field)     ROOTTOWN; eventmap unless eventAreaNumber is 15;
                       the dungeons
new area 2 (dungeon)   WORLD, eventmap, ROOTTOWN; the dungeon unless
                       CompDungeon
```

`initHitCheck` (gcmn 0x00571da0) empties the `ccModelHit` list only for a
first set-up, a town, or a field entered from a town, so the new block's
floor stays on the list. `GO(1)` finds `eventmap` set and keeps it, and
`SetCharPosition` stands the party where `ChangeBlock` left
`eventStartPos`. The scene did not change (`area`, `town`, `field`,
`dungeon`), so no sequence bank is loaded.

## The port

`crates/piney-world/src/evarea.rs`:

```text
EventArea::new            the constructor (the flares' ccEffs are the
                          effects')
EventArea::change_block   ChangeBlock: the pieces, the collision, the
                          lights, SetFog's values, eventStartPos, revAnm
EventArea::select         Draw's pieces in order with their layers, the
                          scrolls and animations stepped
EventArea::lens_flare     DrawLensFlare's positions
EventArea::draw           the pieces into the layers; the flares handed back
                          as StorySprites
EventArea::enter          Enter on area 15's door
EventArea::start_positions  SetCharPosition in the map
Kept                      what WORLD_MAN keeps between scenes: the dungeon,
                          or area 15's map
```

`field_world::FieldWorld` holds it as `Place::Event`: `enter` builds it
for area 15 (or takes the kept one after a door), stands the party with
`start_positions`, runs the same frame as a field (the battle's tasks
over the map's collision, `Hits::event_area`), calls `EventArea::enter`
from `WORLD_MAN::Enter`, draws it in `ccThFieldDisp`'s place, and hands
the flares to `FieldFx::story_sprites`, which `piney-game`'s `AreaFx`
draws with piney-effect's `ccEff::Draw`. The session keeps the map
across the church's set-up in the slot that keeps a dungeon.
`piney-game --mode field:15` starts on the holy ground; the Chaos Gate's
words for area 15 lead there as well (from `--mode story:11`, once
BlackRose has given them). Event 11's instructions in the map (the
arrival's camera, BlackRose, the church's streams) are the event host's:
`crates/piney-game/src/area_host.rs` and `field_host.rs`.

## Checks

`tools/test_evarea_rs.py` builds `EVENTAREA02` with its own constructor in
the interpreter, over `se1_2`'s objects as `tools/test_world_rs.py`'s
`Pieces` serves them, and compares with the `evarea_probe` example:

- the set-up: the constructor, then `ChangeBlock(1)`, `(0)`, `(1)`: the
  constructor's fog, files and flares (`Init`'s chunks and fog switch,
  `SetRenderState`, the PRIM bit), `ccSys.bgColor`; after each, every
  `STATICMODEL`'s type, model and position, the `STATICOBJECT`'s
  animation, root, position, clip and time, the background clumps, the
  lights added to the group, the hits enabled, `SetFog`, `eventStartPos`
  and `revAnm`;
- the draw: `EVENTAREA02::Draw` over 662 frames in both blocks and back,
  the eye and the player all over them, an event scene now and then:
  every piece in order (1,712 clumps, 3,584 models of which 350 without
  fog, 312 static objects, 312 `revAnm`s, 1,566 flares), each with its
  layer; the clumps' and objects' matrices, the models' translations,
  `revAnm`'s matrix, the animations' times, the flares' positions, and
  the offsets `MoveTexture` wrote;
- the lens flare alone for 400 random camera states (eye, view, both
  rotations, the eye view or not): six positions each;
- the ground: `ccLandHitCheck` at 1,500 points of each block (357 and
  581 on the floor, 3 and 9 on a door): z, the result count, the nearest
  result's attribute and point, `checkHitResultAttlibute`;
- the doors: `WORLD_MAN::Enter` from `game.block` -1, 1, 0, 1, 0 over the
  map (the `ChangeScene` asked, the state after), then
  `SetCharPosition` for the leader and the three others;
- frame by frame, `cameraMain` and `ccPlayer::Main` over each block's
  collision: Kite from `DMY_marker01` up the bridge into the church's
  door (`Enter` from frame 343), three runs of random pads on the holy
  ground, the church from `DMY_marker01_2` up the nave and out of its door
  (`Enter` from frame 279), two runs of random pads in the church:
  everything the town's check compares, plus `Enter` and the centre -
  3,000 frames, none differing.

`piney-game`'s `the_church_door_in_and_out` runs `--mode field:15`'s
session through both doors (the event bank loaded on arrival and not at
the doors, Kite standing at `DMY_marker01_2` and `DMY_marker02`);
`the_gate_takes_its_words_to_the_holy_ground` warps from Mac Anu with
area 15's words; `story_11_warps_to_the_holy_ground` does the same from
`--mode story:11` with the event task, whose set-up passes there end and
play goes on. These are the runtime's checks.

## Area 13: EVENTAREA01

Mutation's event 115 goes to field 13 (MUT gcmn 0x00414110 constructor,
0x00414c00 `Draw`).

**The constructor.**

- The scene is `se1_1`: `EA_MODELTABLE01`'s seven rows and
  `EA_OBJTABLE01`'s one, placed at their dummies.
  - Row 0 (`MDL_se1_1fl1`) is type 1.
  - Rows 1-5 (`MDL_se1_1o_1`-`o_5`) are type 2.
  - Row 6 (`MDL_se11obj2`, clip 5500) is type 3.
  - The object, `ANM_se1_1obj1a`, is type 2.
- `bg[0]` is `CMP_se1_1bg1`, with `SetFogSw(0)`.
- `SetFog(150, 4000, 0, 80, 0x1e1e1e)`, and `ccSys.bgColor` 0x1e1e1e.
- `lgtAnm` is `ANM_se1_1bac1a`, stepped once. `eventarea01Light` holds 11
  rows: `LGT_se1_1lig1` distant, then `LGT_se1_1omn01`-`10` omni.
- `BLT_bg`, `BLT_obj`, `BLT_obj2` and `BLT_floor`.
- The start is `DMY_marker_ev01`'s position, its w the dummy's rotation z.
- Seven glows, each a `ccEff` of `EFF_se1_1ef1`:
  - `Init(chunk, 1)`, then PRIM's fog bit (0x20 of +0x62) is cleared.
  - Each is placed at `DMY_effpoint01`-`07` (the table at 0x005ffeb0).
  - Three draws from `fieldrand` each, in this order:
    - `frame` (+0x250) = `fieldrand(patNum)`;
    - `wait` (+0x26c) = `fieldrand(100) + 30`;
    - `count` (+0x288) = `fieldrand(15) + 1`.

**Draw.**

1. `DrawBG`: the clump at the identity on `bgLayer[0]`.
2. `SetActiveLayer(1)`, then `DrawObj2` (type 3, in `BLT_obj2`), `DrawObj`
   (type 2, in `BLT_obj`), `DrawFloor` (type 1, in `BLT_floor`) and the
   type-0 rows.
3. `SetActiveLayer(3)`, then each glow:

| `wait` | the glow |
| --- | --- |
| not 0 | `wait` - 1; `ccEff::Draw(pos, frame)`; `frame` + 1, back to 0 at `patNum` |
| 0 | `ccEff::Draw(pos, fieldrand(patNum))`; `count` - 1; at 0 a new `wait` and `count` as above |

**The port.** `piney_world::evarea01::Area13`, a `StoryMap` on
`town::Base` (`Base::read_volume`, the volume's tables). The glows are
`StorySprite`s with `fog` off.

## Area 43: EVENTAREA03

Mutation's event 102 goes to field 43 and leaves at once. Out of the story,
the map shows a scene of its own and sends the party back. The class is the
same on both volumes but for 4 bytes a function (MUT gcmn 0x00416590
constructor, 0x00416bf0 `Draw`; INF 0x00403590, 0x00403bf0).

```text
EVENTAREA03 (0x1cc bytes): EVENTAREA, then
+0x1c4  int cnt     the fly-over's frame
+0x1c8  int flag    gateListMark[game.server][1] & 0x800: area 43 marked
```

**The constructor.**

- The scene is `se2_1`, with `modelArray`, `anmArray` and `bg` cleared.
- `SetFog(150, 4000, 0, 80, 0x1e1e1e)`, and `ccSys.bgColor` 0x1e1e1e.
- `EA_MODELTABLE03`: two models, both pass 0 (`MDL_se2_1fl1`,
  `MDL_se2_1ba1`, clip 99999.9).
- `lgtAnm` is `ANM_se2_1_1a`, stepped once, with its ambient.
- `eventarea03Light` has one row: `LGT_se2_1lig1`, a distant light (type
  0: `AddGrp` and `WORLD_MAN::SetLightDirection`; type 1 would be an omni
  light, `AddGrp` only).
- `BLT_bg` and `BLT_floor`; `SetStartPos((0, 0, 0), w 1.0)`.

**Draw.** While `flag` is 0:

| `cnt` | what happens |
| --- | --- |
| 0 | `changeCamera(3)`, the view at (1322, -1128, 1014) and the position at (1678, -1433, 1188), `cameraGetRot` into `cameraSetRot`, `ccEvent::MenuBan` |
| 0-139 | t = min(cnt / 120, 1); the view and position move linearly to (-210, 167, 36) and (162, -133, 178); `cnt` + 1 |
| 140 | `ccMsg->Open(&kiteSelfTalk, kiteSelfTalk.name, -1, -1)` (emode 0x100, one line, no voice); `cnt` + 1 |
| 141 | once `ccMsg->Check(0)` answers: `ccEvent::MenuClr`, `changeCamera(1)`, `ChangeArea(0, 1)` |

The moves write x, y and z; w, and the scratch `cameraGetRot` reads,
are unset stack. Every frame then does `SetActiveLayer(1)`, `DrawFloor`
through the vtable (the pass-1 models inside `BLT_floor`: none here) and
the pass-0 models. `DrawBG` would draw `bg[0]`, which is never set, and
`Draw` does not call it.

During event 102 the area is marked, so the event's block 4 fades out and
changes scene while the map only draws.

**The port.** `piney_world::evarea03::Area43`, a `StoryMap`. Its `frame`
runs the scene: it moves camera 3, and sends `MenuBan` and the message to
the game as `Request::Story`. `AreaMode` answers `Check(0)` with the
frame's pad before the field steps. `kiteSelfTalk` is the fieldui
group's `kite_self_talk` (and its address), a root of the talk records.

## Area 16: EVENTAREA07, Hideous Someone's Giant

Infection reaches area 16 after the ending: side event 62, "SERVER1",
gives its words with a mail and waits in its dungeon. The map is a
giant's arm laid out as a bridge up to a doorway, under a pale sky.

`EVENTAREA07::EVENTAREA07` (gcmn 0x00405a00) reads `se1_7_1` (+0x1a4) and
`town_z` (+0x250, a `LENSFLARE` of it); clears the model, object,
background and cloud arrays; zeroes the bob (+0x1d0..+0x1d8) and its
direction (+0x1c4); and calls `ChangeBlock(0)`.

`ChangeBlock(block)` (0x00405c80) sets `blk` (+0x1e4), deletes the block
before's models, objects, background clumps, lights, light animation and
clouds, and builds:

| | block 0 | block 1 |
| --- | --- | --- |
| `SetFog` | (15000, 35000, 0, 80, 0xdcfae6) | (500, 5000, 0, 80, 0) |
| `ccSys.bgColor` | 0xdcfae6 | kept |
| models, objects | `EA_MODELTABLE07` (8: `MDL_se1_7ob1_1`-`4` type 2, `ob2_1`-`4` type 3), `EA_OBJTABLE07` (2: `ANM_se1_7fl1a` type 1, `wi1a` type 2) | `EA_MODELTABLE0702` (2), no objects |
| background | `EA_BGNAME07`: `bg[0..3]` = `CMP_se1_7bg1`, `cl3`, `cl1`, `cl2`, fog off | none |
| light animation, lights | `ANM_se1_7bg1a`: `LGT_se1_7lig1` (distant) | `ANM_se1_7bg2a`: `LGT_se1_7lig2`, `LGT_omn01`, `LGT_omn02` |
| `SetStartPos` | `DMY_marker02` when `game.areaPrev` (+0x18) is 2 (back from the dungeon), else `DMY_marker01` | `DMY_marker03` |
| then | 25 `CLOUD`s of type 1 in `town_z` ([clouds](town02.md)); `anmArray[1]`, `modelArray[0]` and `[4]` `SetPos` at the bob | |

`STATICMODEL::SetPos` (0x005d03f0) and `STATICOBJECT::SetPos` (0x005cff60)
copy the position: the model's `ccModelHit` takes the translation and its
inverse, the object's coordinate `SetMatrix_PosRotZYX`. The collision is
`HIT_se1_7ob1hit` on `MDL_se1_7ob1_1` in block 0, `HIT_se1_7ob1_5hit` in
block 1.

Block 1's names are `se1_7_2`'s, but `ChangeBlock` looks them up through
`se1_7_1`'s handle, and `DMY_marker03` is in neither file. No Infection
script brings the map to block 1.

`Draw` (0x00406b80), through the vtable (0x00375bb0: +0x0c `DrawBG`,
+0x10 `DrawObj`, +0x14 `DrawObj2`, +0x18 `DrawFloor`):

```text
block 0   DrawBG
          objLayer: DrawObj2, DrawObj, DrawFloor
          effLayer: each CLOUD's Move, then its Draw
          LENSFLARE::Draw(se1_7_1, "DMY_se1_7lig1point", 0)
block 1   objLayer: DrawObj;  floorLayer: DrawFloor
```

`DrawBG` (0x00406730) steps the bob first. It draws `fieldrand(25)`: while
rising (+0x1c4 set) the bob's z (+0x1d8) goes up by it, and past 350
stops at 350 and turns; while sinking it goes down, and below 0 stops at 0
and turns. Then `MoveTexture` of `EA_moveTex07`'s rows (`MAT_se1_7clo1`
and `MAT_se1_7cl3`, V by 0.1 a frame). Then `bg[k]` on `bgLayer[k]` at
`SetMatrix_PosRotZYX((0, 0, bob), 0)`. So the sky and its clouds rise and
fall under the bridge, which stays where it was set. `DrawObj`,
`DrawObj2` and `DrawFloor` draw the type-2, -3 and -1 models, then
objects, each inside its texture group's lock.

`WORLD_MAN::Enter` for area 16: from block 0 (or -1) `ChangeArea(2, 0)`,
its dungeon; else `ChangeBlock(0)` and `ChangeScene(.., 0)`. The dungeon
is generated from the area's words (dungeon type 7, two floors of 11 and 7
rooms, 7 and 2 portals). Its portals give Mystery Rocks, Mu Guardians and
Tetra Armors, one to three in any mix, drawn at each opening
([battle.md](battle.md#spawning)).
`WORLD_MAN::Quit` deletes the map whenever the area is left. Coming back
from the dungeon, `GO(1)` makes a new one, which stands the party at
`DMY_marker02`. `SetCharPosition` for fields 16 and 66 puts the others at
(+200, -150), (-200, -150) and (0, +300).

The port is `crates/piney-world/src/evarea07.rs`, held as
`field_world::Place::Giant`. The clouds and flares are story sprites
(`StorySprite` carries a cloud's turn). `the_giants_blocks`,
`the_background_bobs` and the session's
`the_giants_door_to_its_dungeon_and_back` check it.

## The boss arenas: EVENTAREAB0

`GO(1)` makes an `EVENTAREAB0` for fields 1-8. Every one of their
`EVENTAREA_INFO` rows has model 1, flag 3 and type 10, and none has words:
the party arrives from a dungeon's door (`WORLD_MAN::Enter` -255,
[Dungeon generation](dungeon.md)), with `WORLD_MAN` still the area it came
from. Each field is a stage with its own scene file. The constructor's jump
table (0x006a3d60) picks it by `game.field`:

```text
field  1      2      3      4      5      6      7      8
stage  se1_5  se2_3  se2_4  se3_2  se3_4  se4_3  se4_5  se4_7 (+ se4_8)
```

Infection uses field 1: Skeith's arena, through area 27's last door
([Bosses](boss.md)).

**The constructor** (gcmn 0x004079f0):

1. `sw` = (0, 1, 0), `layerSw` 0.
2. The field's two digits go over characters 6 and 8 of
   `EA_MODELTABLEB0`'s and `eventareaB0Light`'s names (they read `se1_5` in
   the executable).
3. `SetFog(1500, 12000, 0, 70, 0x1e1e1e)`, and `ccSys.bgColor` 0x1e1e1e.
4. Nine `STATICMODEL`s:
   - `fl0`-`fl4` of type 1 (floor);
   - `ob0` of type 0;
   - `ob1`-`ob3` of type 2.
5. `lgtAnm` is the stage's `ANM_*bac1a` (`SetLightEnv(1)`). `bg[0..2]` are
   `CMP_*bac1`, `clo1` and `clo2`, with fog off.
6. The 20 lights at the animation's frame 1: `LGT_*lig1` distant (also
   `SetLightDirection`), then `omn01`-`omn19`. The ambient comes from the
   animation.
7. `BLT_bg`, `BLT_obj`, `BLT_floor`.
8. `SetStartPos(DMY_center01)`.
9. 54 `FIREFLY2`s, the k-th over `DMY_marker(k % 18 + 1)` (the table at
   0x005d2730), with the field's `bossFireFlyEff` (`EFF_se1_5fir1`). Each
   one's constructor, `Init` and `SetBasePosition` draw from `fieldrand`
   (six draws a firefly), so the constructor's last act moves it 324
   times.

The collision is `fl0`'s Hit chunk, `HIT_se1_5fl0hit`.

**Draw** (0x004095e0), unless `stageSW`:

1. `DrawBG`:
   - `MAT_*clo1`'s v is set from a static that grows 0.0015 a frame,
     wrapping past 1. The material takes the value before the step.
   - `bg[k]` is drawn on `bgLayer[k]` (-100, -90, -80) at the identity.
2. objLayer: `DrawObj`, the type-2 rows.
3. floorLayer: `DrawFloor`, the type-1 rows.
4. `modelArray[5]` (`ob0`) on refLayer (30), or on objLayer while `layerSw`
   is set. `SwitchLayer` (0x004095b0) toggles `layerSw`: Skeith's magic
   swaps it twice.
5. effLayer: for each firefly, its `Draw`, then its `Move` (below).
6. `ob1`-`ob3` bob. Each one's z rises by `fieldrand(5)` until past 50, then
   sinks by it until under -50 (`sw[k]`). `GO(1)` sets no seed for a story
   map, so they draw on from wherever `fieldrand` stood.

### The fireflies: `FIREFLY2`

A `FIREFLY2` (0x120 bytes, gcmn firefly.cpp) is a `ccEff` sprite that
wanders about a base point and throws off sparks, which are `FIREFLY2`s
too (+0x9c set). The arena's are type 0 (+0x00). Type 1, which
`WORLD::Generate` and `FIREFLY::Move` make for the fields, follows the
player (`SetBasePosition2`, 0x005b4ba0: a point up to 1500 each way from
the base, turned by the camera's heading, at the ground's height); the
fields' fireflies are on the [field](field.md) page, and both types are
ported in `crates/piney-world/src/firefly.rs`.

```text
FIREFLY2()            0x005b4550  timer (+0xa0) fieldrand(20) + 10, pattern
                                  (+0x90) fieldrand(100); velocity (+0x70)
                                  (0, 0, 1.0), speed (+0x84) 1.0, spread
                                  (+0x88) 15.0, transparency (+0x80) 0, no
                                  sparks
Init(stream, name)    0x005b4d30  life (+0xa4) fieldrand(200) + 220; new
                                  ccEff, Init(chunk, 1) (fog on, the depth
                                  test), scale 1.5; the name kept
SetBasePosition(p)    0x005b49f0  base (+0x100) = p; pos (+0xb0) x, y =
                                  fieldrand(500) - 250, fieldrand(500) - 250;
                                  pos += base (all four lanes); z =
                                  fieldrand(100) (not the base's); w 1
```

`Draw` (0x005b5eb0):

1. A firefly (not a spark) under life 100 sets its transparency to 0.1
   times its life (up to 9.9; the sprite's alpha saturates).
2. The `ccEff` takes the transparency.
3. `rel = ccTransPosW2P(pos)`: the position less the player's.
4. Only while `sqrt(x x + y y)` of `rel` (single `mula`/`madd`, then
   `fptodp`, `sqrt`, `dptofp`) is 1000 or less: `ccEff::Draw(P2W(rel),
   pattern)`, then each spark's `Draw`.

`Move` (0x005b5050), type 0:

1. `cameraGetPos` and `ccGetDist` are measured (type 1 uses them).
2. If `rel` is more than 1000 across, nothing more: the firefly stands
   still, neither fading, ageing nor making sparks.
3. The fade. A firefly over life 100, or a spark, under transparency 1
   adds 0.1 (clamped to 1); a firefly under life 100 takes `life / 100`.
4. Life down by 1.
5. `pos += velocity * speed` in x, y, z. The z velocity stays 1.0, so a
   firefly rises a unit a frame until it starts again. +0x110 adds the
   same.
6. The pattern steps, 0 again at 100.
7. A firefly with under 3 sparks makes one when `fieldrand(100)` is 61 or
   more. `new FIREFLY2` (its two draws), in the first free slot, then:
   - `fieldrand(100)` 61 or more, a drifting spark: velocity `-v +
     (fieldrand(100) / 100 - 0.5)` in x, y, z in that order, life
     `fieldrand(30) + 60`, scale 0.75;
   - else a quick spark: velocity `fieldrand(200) / 100 - 1` in x, y, z,
     life `fieldrand(5) + 5`, speed 2, scale 1.2.
   Either starts at the firefly's position with transparency 0.2.
8. Each spark's `Move`. A spark at life 0 is deleted and the count drops.
9. A spark stops here. A firefly's timer counts down; at 0 it is
   `fieldrand(20) + 10` again, and the x and y velocity become
   `(fieldrand(15) - 7.5) / 10` each (15 from the spread).
10. At life 0: `SetBasePosition(rel)`, then life `fieldrand(200) + 220`.

The last step bases the firefly on its position *relative to the
player*: a firefly that lives out its life starts again near the
world's origin, as far from it as it stood from the player. The arena's
centre is the origin, so they gather about it.

Standing at `DMY_center01` (0, 0), Kite has few or none in reach. The
markers are 1200 or more across; only a firefly of `DMY_marker04` or
`09` (x +-1200) that its +-250 put 200 or more toward him comes within
1000. The fireflies show and move once he walks out toward them.

**NextStage** (0x004098e0), field 8 only:
- deletes the models;
- drops the lights;
- sets `stageSW`;
- `Draw` then draws only the last stage's anm, `se4_8`.

**The port.** `piney_world::evarea_b0::Arena`. `FieldWorld` holds it as
`Place::Arena` for fields 1-8: standing the party at `DMY_center01`, the
battle's tasks over its collision, drawing it in `ccThFieldDisp`'s place.
The arena keeps its own `fieldrand`; the fireflies
(`piney_world::firefly`, type 0) are made in the constructor from it and
drawn and moved in `Draw` with the player's position. Their sprites go to
the effects' `story_sprites` (`EFF_se1_5fir1` of `se1_5`, `Init(chunk,
1)`'s render state, the scale, transparency and pattern) on effLayer.
`piney-game --mode field:1` starts there, with area 27's `WORLD_MAN` on town
1.

**Checked.** `test_evarea_rs.py`'s `ArenaAgainstGame` builds the game's
`EVENTAREAB0` for field 1 over `se1_5`, its fireflies the game's own
`FIREFLY2`s, and compares it with `evarea_probe`'s `arena`. It checks:

- the models' rows, names and positions;
- the background clumps, the lights in their order, the distant light;
- the hit models, `SetFog`, `eventStartPos`, `bgColor`.

Then 900 `Draw`s with `SwitchLayer` now and then and the player walking
between the markers and the origin: every piece in order with its layer
(each firefly's and spark's `ccEff::Draw` with its position, pattern,
transparency and scale), the bobbing positions, the scroll's v and
`fieldrand`'s seed. They match.

## Kyvia's disc: EVENTAREAB8

`GO(1)` makes an `EVENTAREAB8` (areabk0.cpp) for fields 9-12, Kyvia's
four stages ([Kyvia's first fight](boss-kyvia.md)). Fields 9-11 are
`se1_6`: a sky, 40 rising rocks and a disc that carries the party along
a path with `WORLD_MAN::SetTransMode` on. Field 12 adds `se4_9`'s three
pieces and no disc.

**The constructor** (MUT gcmn 0x0041d200):

1. The models of `EA_MODELTABLEB8` (one, `MDL_se1_6flo1`, the floor) and
   the objects of `EA_OBJTABLEB8` (one, the disc `ANM_se1_6ob1a`).
2. `bg[0..3]`: `CMP_se1_6bac1`, `clo1_1`, `clo1_2`, `moo1` on `bgLayer`
   0-3, fog off. `SetFog(6000, 20000, 0, 90, 0)`, `bgColor` 0. One light,
   `LGT_se1_6lig1` of `eventareaB8Light`, from `ANM_se1_6bg1a`.
3. By field, the disc's animation (+0xcb0), the camera and dummy of its
   path (+0x1cc) and `discCnt` (+0x1c4):

```text
field  animation       camera          dummy          discCnt
9      ANM_se2_4_1_c   CAM_camera2_4   OBJ_dummy2_4   2
10     ANM_se3_3_1_c   CAM_camera3_3   OBJ_dummy3_3   0
11     ANM_se4_4_1_c   CAM_camera4_4   OBJ_dummy4_4   0
```

4. Back in the field it came from (`fieldPrev` == field), field 9's disc
   starts at frame 419 (0x1a300) and field 11's at 2339 (0x92300).
5. `discPos` (+0xc90) is `DMY_marker01`. The start is the path's dummy
   (its world translation) plus the marker, facing 2.0; field 12's is its
   own, (82, -11122, 910) facing pi, with `CMP_hit01`-`03` at their places
   under `ANM_se4_7_1a`.
6. 40 `FLOATROCK`s (`SetFloatRockParam`, 0x0041cc80): a life of
   `fieldrand(150) + 90`, a rise of `fieldrand(20) + 5`, the place's own
   draws (then overwritten), a free cell of the 10 x 10 `floatRockPos`
   by two `ccRand() % 10` (the centre, 6 x 6 in fields 10-12, 4 x 4 in 9,
   taken first), 800 a cell (1500 in field 10) from the grid's middle, a
   turn of `fieldrand(15) / 1000` either way; the rock's clump
   (`rockname`) by `fieldrand(6)`.
7. 25 `CLOUD`s of type 1, made and never moved or drawn.
8. Fields 9-11: `SetTransCenter(start)`, `SetTransDiff(0)`, and the
   disc's collision (`HitEnable`, `SetHitMatrix` at its root).

**Draw** (0x0041ed30):

1. `DrawBG` (fields 9-11): `MAT_se1_6clo1`'s v from a static that falls
   0.003 a frame from 1.0, back to 1.0 below 0; the four clumps.
2. effLayer: `DrawObj`, the disc.
3. Fields 9 and 10: each rock's `DrawRock` (0x0041f130): fading in over
   40 frames and out over its last 40, turning and rising, drawn when
   `ccCheckCameraDeg(pos, 12288)`; at the end of its life its cell is
   freed and `SetFloatRockParam` again.
4. objLayer: `DrawFloor`.
5. The disc steps: field 9 to its animation's end; field 10 to frame
   420, 800, then the end; field 11 to 400, 1270, 2340, then the end, by
   `discCnt`. `move` (+0x1c8) clears where it stops; `IsMove`
   (0x0041e7d0) reads it and `Move` (0x0041e750, from Kyvia) raises
   `discCnt` (held at 2 in field 10 and 3 in field 11).
6. The ride (0x0041e8c0), fields 9-11: the disc steps again, is put where
   its path was the frame before (`STATICOBJECT::SetPos`, its hits with
   it), that place becomes the party's centre (`SetTransCenter`) and the
   path's point now is kept (`discPrevPos`, +0xca0), which Kyvia reads.

While the trans mode is on, `ccPlayer::ccPlayer` (MUT gcmn 0x005c2d8c) and
`ccFellow::Initialize` (0x0042ebdc) set `diskOffset` to the character's
place less the centre, and the party rides with the disc.

**The port.** `piney_world::evarea_b8::DiscArea`, a `StoryMap`; its tests
(`the_disc_of_field_9`, `the_disc_rides_to_its_end`,
`the_later_discs_stop_at_their_stages`) hold the pieces, the light, the
hits, the rocks' cells and the ride. `FieldWorld` hands Kyvia the disc
(`IsMove`, `discPrevPos`, `DMY_marker01`) each frame and carries out
`Move`.

## Unknown

- The fog by depth (`piney_draw::DepthFog`, worklog 0145) is not
  compared with the game's pictures: which models the game draws with
  PRIM.FGE is taken from `SetFogSw` (off for the background, on for the
  rest).
- `EA_moveTex02` lives in `gcmn.prg`'s data: the port keeps the scroll
  with the map through its doors, but a new map (after the town) starts
  it at 0, where the game carries it on.
- `EVENTAREAB8` is not compared with the game's `Draw` frame by frame:
  its pieces and the ride are held by unit tests only, and fields 10-12
  have not been played through their events.
- The other `EVENTAREA` classes (04-06) are not ported (areas 66, 67,
  91), nor `EVENTAREAB0`'s `NextStage`: their story areas still get a
  generated field.
