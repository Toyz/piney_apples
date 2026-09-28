---
title: Field generation
status: partial
volumes: all
covers: INF gcmn.prg:0x005a6da0 WORLD::Generate, 0x005a4cd0 WORLD::Init, 0x005adc60 FIELD::MakeField, 0x005a6b00 WORLD::MakeHill, 0x005b4070 FRACTAL2::Generate, 0x005abd20 SetDungeonEnter, 0x005ac7e0 SetLake, 0x005ac120 SetKeyObject, 0x005aba50 SetCover, 0x005a8570 WORLD::DrawMesh, 0x005b0aa0 FIELD_MESH::RelocateMesh, 0x005ae940 FIELD::CalcWorldMeshPosition, 0x005b1890 FCOVER::SetPosition, 0x005b19d0 FCOVER::Draw, 0x005a9070 WORLD::DrawEffect, 0x005a97b0 WORLD::Draw (its weather), 0x005a4840 WORLD::SetRainEffect, 0x005a8860 WORLD::DrawRain, 0x005a8f40 WORLD::DrawSnow, 0x005a6a50 WORLD::SetSmoke2d, 0x005ad600 WORLD::DrawSteam, 0x005aae90 WORLD::SetSpecialObj (the fires), 0x00503650 SNOW::SNOW, 0x00503e80 SNOW::Move, 0x005042f0 SNOW::Draw, 0x005028e0 calcPos, 0x00502ba0 calcPos2, 0x00502330 waterUVModifi, 0x005b1ed0 FIREFLY::FIREFLY, 0x005b2100 FIREFLY::SetBasePosition, 0x005b21a0 FIREFLY::Init, 0x005b2900 FIREFLY::Init (hacked), 0x005b30a0 FIREFLY::Move, 0x005b3a60 FIREFLY::Draw, 0x005b1b60 NSPLINE::PreCalc, 0x005d0560 TOBJ::Init, 0x005d0db0 TOBJ::Move, 0x005d08d0 TOBJ::Draw, 0x005d1440 BIRD::Init, 0x005d1540 BIRD::SetPos, 0x005d1590 BIRD::Move, 0x005d1560 BIRD::Draw, 0x00503360 LENSFLARE::Draw (mode 1); INF SLUS_202.67:0x00106c70 ccBufferSampling::SetShade (the seven-argument form), 0x00106dd0 ccBufferSampling::MakePacket, 0x00108930 ccLayer::Add, 0x0017bf20 tobjSeLoopStart, 0x0017c0d0 tobjSeLoop, 0x0019f190 WORLD_MAN::GetWeather, 0x0019ef50 WORLD_MAN::GetTime, 0x0019f0d0 WORLD_MAN::CheckLensFlare, 0x001da710 ccCheckCameraDeg, 0x001ce300 effSmoke, 0x001412b0 ccShadowModel::Draw, 0x00105820 ccDrawEnv::SetFog, 0x0015c4b0 ccMask::ccMask
worklog: 19, 25, 33, 38, 130
---

# Field generation

A field is built from `fieldSeed`: a noise height map, fractal hills, a
dungeon entrance, maybe a lake, objects, ground cover and a start position.
`tools/field.py gen` reproduces it, checked against the game's own
`WORLD::Generate` run in `tools/eemu.py` (which interprets the EE FPU for
this): 150 fields, 0 mismatches - 105 story areas and 45 random keyword
triples, every field type but 4, 15,171 objects, height maps compared bit for
bit.

The port carries the generator in Rust (`piney_data::field`, worklog 33):
`piney-gen` (`placement::field`) reads the tables out of each volume's
executable into the build (`field::tables_of`), the heights are computed
with the EE's float rules,
and `tools/test_field_rs.py` compares it with `field.py` field by field.

## Order of events

```
WORLD_MAN::GO: seed = fieldSeed; randcnt = 0
WORLD::Init    consumes a fixed number of draws (snow, smoke):
               field types 5, 6: 700, or 1600 when weather >= 4;
               type 0: 112; types 2, 3, 5, 6: another 21
WORLD::Generate:
  1  heights[80][80] = fieldrand(64) each                 FIELD::MakeField
  2  story area (ccGame.field != 0): mark chips 19-21 x 19-21, flatten
     around (20, 20), start there
  3  hills: one large (fieldrand(10) + 20 cells) and 3 / 5 / 8 small
     (fieldrand(10) + 10) by `ground`; each a FRACTAL2 of a 5x5 key grid
     (fieldrand(128) on the rim, fieldrand(768) inside) interpolated with
     natural splines, clamped at 0, placed at fieldrand(80 - size) x2,
     retried up to 100 times while it overlaps another hill
  4  dungeon entrance (SetDungeonEnter); a lake for field types 2, 3, 8, 9, 10
  5  InitQuad
  6  objects: key (20), sub (30), base (35), tree (35; SetTreeObject_B for
     type 1); type 7 halves key, thirds base, halves tree; type 1 divides
     trees by 6; all counts x 0.8 / 0.9 / 1.0 by `object`
  7  InitQuad; ground cover (2 draws per free chip); start position
```

Grids: `FIELD.map`, 80 x 80 floats at `FIELD+0xaf02c`, 600 units per cell;
four 40 x 40 byte chip grids - `check` (+0xce430), `check2` (+0xcea70),
`check3` (+0xcf0b0, hidden) and `mnt` (+0xcf6f0, hill).

Each object: `idx = fieldrand(count)` in its table, then position
`fieldrand(40 - w)`, `fieldrand(40 - h)` until the site passes that object's
tests (free chips, no hill, height span within 64); 300 failures in one call
end it. Objects with the flatten flag level their 3w x 3h cells. The tables
are `FOBJECT_INFO_TABLE` rows (0x1c bytes: anm, clump, type, w, h, rotflag,
flat) through `FOBJECT_TABLE[11]`.

Of those fields, only the names, the size and `flat` are read.
- **The tables are read in two places.** The rows are reached only through
  the six `FOBJECT_TABLE`s. `WORLD::Init` reads them for the models' vertex
  colours, and `Generate`'s setters read them to place objects.
- **`type` (0-3) is never read.**
  - 0: key rows.
  - 1: sub rows, the lake fields' own lake rows, and three of
    `BaseObjTBL_I`'s six (`CMP_sfi3tom1`-`3`).
  - 2: the other base rows and all tree rows.
  - 3: entrance rows.
- **`rotflag` (0, 1 or 2 in 71 / 12 / 85 rows) is never read.** `FOBJECT` and
  `FOBJECT2` keep no pointer to their row, and their constructors set the
  rotation to zero. So no field object is rotated.
- **`flat` is always a single bit.** 0x2 levels the site and hides it from
  the cover (118 rows), 0x4 marks the lake pieces (5), and 0x8 asks for the
  height test (45). The code tests them as bits.

The dungeon entrance in a story area must be at least 8,000 units from the
start - within 8,000 for event 14. The start in a random field is the first
free 2 x 2 chip with `fieldrand(100) >= 96`; protected story areas also need
it more than 7,000 from the entrance. Field type 4 has no field: its gate
leads straight into the dungeon.

## Drawing

`tools/field.py` and `piney_data::field` (`Field::scene`) reproduce what
the game draws. The parts marked *read* were read from the code; the rest was
checked by running the game's functions in eemu (`tools/test_field.py`).

**Ground.**
- **Tiles.** Every chip is one copy of the type's `BaseMeshName` model: 24
  vertices spanning ±600, vertexScale 600. `RelocateMesh` puts it at
  (600 + 1200·mx, 600 + 1200·my, 0), unrotated.
- **Heights and colours.** `SetMESH2` rewrites only z and colour. Vertex k
  gets the height of cell (2mx + dx, 2my + dy), stored as s16
  `fptosi(4096·h / scale)`, and that cell's lit RGB, keeping its alpha. x, y
  and UVs stay the template's.
- **Hidden chips.** A chip under a levelled object or the lake draws no tile.

**Lighting.** The ground's vertex colours are the map's own:
- `InitQuad` makes two face normals per cell, from opposite diagonals.
- `CalcQuadVertexNormal` averages six faces per vertex. One of them is
  `quad[x+1][y-1]` where the neighbour `quad[x][y-1]` was meant, a game bug
  kept in the port.
- `CalcVertexColor` gives `128·(ambient + max(0, −n·L)·colour)`, clamped to
  0-255.
- **The light.** It comes from `BGTBL[type][weather]`: a light and an
  animation in `bg_{x}{weather+1}`, whose frame 0 carries `F_Ambient` and
  `F_DistantLight`. `Lambert` passes the light's direction vector to
  `RotMatrixX/Y/Z` as if it were angles; kept as the game does it.
- **Objects.** `WORLD::Init`'s `CalcObjectVertexColor` lights the type's table
  clumps the same way, in the loaded model data. A clump listed twice is lit
  twice: `SubObjTBL_K` has two.

**Cover.** A cover is the 6-vertex ±300 template (vertexScale 300) at
(600 + 1200·i ± 300, 600 + 1200·j ± 300, 2.5). Quadrants 0 and 2 have x
negative, and 0 and 1 have y negative. `SetSmallMESH` gives it the
heights and colours of cell (2i + qx, 2j + qy), so it sits 2.5 above the
ground.

**Object heights.** `WORLD::GetHeight` casts a segment from z 2500 down to
−1200 into the two triangles of the point's 600-unit cell, using
`collisionLP` with 0.001 edge slack. It returns −500 on a hidden chip and −1
on a miss. An object's z is that query at the moment it is placed; the
entrance, the lake, type 1's tree rows and levelled objects get 0 instead.

**Around the player** (*read*). The field is a torus 48,000 units (80 cells)
across. `DrawMesh` draws the 12 × 12 chips around the player, −6 to +5 each
way, each at the copy of its position nearest the player. Tiles beyond
8,400 are culled, and tiles fade between 6,700 and 7,200 with alpha
(7200 − d) / 500. Cover draws within 7,200, fading from 6,700.

**Lake** (*read*). The three copies of `field_eff`'s `ANM_sfwat1_1a` sit at
the lake object's centre, (x, y, 0). They differ in render layer: one
scrolls its UVs every frame, and two use a texture `Generate` builds at run
time.

**Background** (*read*).
- `DrawBG` draws the sky, two cloud layers and the mountains, centred on the
  player, each in its own background layer. Field type 6 adds an aurora
  (`CMP_sfh7aur{weather+1}`). Only the sky material's V scrolls, by 0.003 a
  frame.
- Fog comes from the same per-type, per-weather table: colour, near, far and
  a percentage.

## Weather and ambient pictures

Beside the ground and the objects, `WORLD` makes and draws a field's
weather and its moving scenery. `WORLD_MAN::GetWeather` and `GetTime` read
the field type and the background row `b` (`WORLD_MAN` +0x0c):

```
GetWeather   0 fine, 1 cloudy, 2 rain, 3 storm, 4 light snow, 5 heavy snow
  types 0-3        b 3: 1, else 0
  types 4, 7-10    b 3: 1; b 4, 5: 2; b 6, 7: 3; else 0
  types 5, 6       b 0-3: 4, else 5
GetTime      0 day, 1 evening, 2 night
  types 4, 7-10    b 1: 1; b 0, 3, 4, 6: 0; else 2
  types 5, 6       b 1: 1; b 0, 3, 4: 0; else 2
  types 0-3        b 1: 1; b 0, 3: 0; else 2
CheckLensFlare   not hacked (WORLD_MAN +0xf0 != 3), weather 0, 1 or 4,
                 time 0 or 1
```

**What `Init` and `Generate` make** (every draw `fieldrand`):

```
WORLD::Init       weather 5: 200 SNOW(1) (8 draws each), weather 4: 100
                  SNOW(0) (7 each); type 0: 16 embers, SNOW(2) (7 each);
                  types 2, 3, 5, 6: the 2D smoke asleep fieldrand(300) +
                  150, its five puffs (SetSmoke2d: x -fieldrand(512), y
                  fieldrand(384), dx fieldrand(20) + 20, dy fieldrand(20)
                  - 10); type 0: DrawSteam(0); types 0-3: the heat haze
                  (sfzair1's ANM_sfzair1a, OBJ_sfzair00 duplicated)
WORLD::Generate   after SetStartPos: server 0-4 (ccGame +0x1c, always):
(its tail)        TOBJ when fieldrand(100) >= 86; weather 2, 3:
                  SetRainEffect (50 drops); weather 3: the thunder's four
                  sprites; night or hacked, not type 9: 5 FIREFLY at
                  (fieldrand(48000), fieldrand(48000)) on the ground; type
                  9: 15 FIREFLY2 about the start; type 10, not night, no
                  rain: BIRD over the dungeon entrance
SetSpecialObj     (EntryGimmick) type 7's fires: each chip's FOBJECT
                  (fobj[x][y], y outer) with an animation, its node
                  OBJ_roc1fire then OBJ_roc3fire, at the node's place in
                  the animation plus the object's wp (+0x30), w 1
```

A SNOW places itself (`calcPos`) within 1,300 of `WORLD_MAN`'s centre: at
800 above the ground, falling `(fieldrand(5) + 1) b` a frame and drifting
`(fieldrand(a) - a/2) / 10` (light: a 50, b 2; heavy: a 200, b 4, and a fall
of `fieldrand(40) + 10`). Type 5's flakes are `EFF_sfg8sno1`-`4`, type 6's
`EFF_sfh8sno1`-`4` (a quarter each by `fieldrand(100)`); the embers
(`calcPos2`) `EFF_sfa8fir1` at half scale, rising from the ground and
fading over their last 32 frames. `Move` keeps each within 1,300 of the
player each way, and a flake below 0 is placed again.

**What `Draw` draws**, after the objects and the ground, on the layers
`WORLD_MAN::SetActiveLayer` names (0 sysLayer, 1 objLayer, 2 obj2Layer, 3
effLayer, 4 floorLayer, 5 charLayer, 6 refLayer):

```
first call     weather 4, 5: ten DrawSnow (the flakes settle)
layer 6        the heat haze (types 0-3): the anm 3,000 ahead of the
               player along the camera's heading (6,000 for types 2, 3),
               scaled 1.3 across, stepped, transparency 0.7; its model's
               texture is the picture drawn so far (waterUVModifi: each
               vertex samples the screen where it stands)
layer 3        DrawEffect:
  DrawRain       weather 2, 3: every drop placed afresh each frame within
                 1,500 of camera 1's eye, on the ground; its pattern steps
                 0-19; at 20 its splash (EFF_sfp8rai6) plays 15 patterns
                 there, 10 above the ground
  the storm      weather 3: a bolt (EFF_sfp8thu1-4) 12,000 ahead of the
                 camera, up to 45 degrees either side, 900 up, every 30-59
                 frames (the first after 60-119), shown four frames at
                 WORLD's centre offset; a
                 screen flash (EntryFlash(10, 0x80ffffff, 0, 0, 512, 384))
                 every 900-1,499 frames (the first after 750), sound 236
                 30-49 frames later
  the fires      type 7, even frames: each fire within 6,500 of the eye
                 puffs effSmoke(fire + m (10, 0, 0), m (1, 0, 0) + (0, 0,
                 2.5), 1, 8, 208, 512, 32), m three ccRand turns
  DrawSnow       the flakes moved, then drawn; type 0: the embers
  the 2D smoke   types 2, 3, 5, 6: asleep 150-449 frames, then shown
                 150-449 frames: fieldrand(4) + 1 puffs of TEX_sfsmo1 (a
                 512 x 512 ccMask of the 128 x 128 texture, its colour's
                 alpha 0x60) drifting right across the screen
  the fireflies  night, hacked, or type 9: type 9's FIREFLY2s (none in
                 rain), else the 5 FIREFLYs
layer 5        TOBJ Move, Draw; BIRD Move, Draw
layer 3        the lens flare (CheckLensFlare): LENSFLARE::Draw(bg,
               DMY_<x>lig_<b+1>point, 1)
               type 0: DrawSteam(1) - every 120-270 frames eight bursts,
               3-6 frames apart, of 2-6 effSmoke puffs (t 109) from a
               vent within 1,000 of the player, its sound 43 (ccSeOn3D)
               at the first
```

`FIREFLY` flies a loop of three closed natural splines (`NSPLINE`, the
`FRAC2` spline) about its base with a trail of sixteen sprites and throws
off `FIREFLY2` sparks; hacked it is a data bug, `EFF_sfzdigi0` with a
trail of `EFF_sfzdigi1` glyphs (a new trail sprite every sixth frame, each
pattern `fieldrand(15)`). Type 9's `FIREFLY2`s ([evarea.md](evarea.md)'s
class, type 1) wander within 2,000 of the camera and are put back about it
past that. `TOBJ` is the server's traveller (named here from its files):
Δ's airship (`t1`), Θ's balloon (`t2`, its two cloths' V scrolled 0.003
and 0.0015 a frame), Λ's whale (`se3_5wh1`), Σ's (`t4`) and Ω's sleigh
(`t5`, with two runners at its `OBJ_dummy_rei0`/`1`). It shows for 630-1,229
frames, fading in and out - Δ's, Λ's and Ω's flying across at 10 a frame,
the others standing - then sleeps. It casts its shadow (`ccDrawEnv` +0xa0
three times its height, +0xa4 its transparency) into `WORLD_MAN`'s second
packet, 128 x 128 laid twice ([the shadow volumes](shadow.md)). The airship and the whale
hum: `Init` starts `seData[44]` looping silent (`tobjSeLoopStart`), and
each `Draw` while its transparency is not 0 sets the loop's pan and volume
for its place, the volume scaled by the transparency (`tobjSeLoop`;
sound.md). `BIRD` (`CMP_sfp8how1`)
flies out from the dungeon entrance at 20 a frame, turning 0.0314 a frame;
after half a turn it is back at the entrance and turns the other way.

**The depth shades.** `WORLD_MAN::WORLD_MAN` makes two `ccBufferSampling`s
(+0x418, +0x41c) with `SetShade(0, 896, 0, 8, 7, 3000, 0x48808080)` and
`SetShade(0, 896, 0, 7, 6, 6000, 0x48808080)` (main 0x00106c70): CLAMP
both ways, scale 1, and a texture of their own (`texuse` 1) at x 896 of
buffer 0, 256 x 128 and 128 x 64. Every `WORLD::Draw` sends them first on
sysLayer (+0x41c, then +0x418; each prepends, so the 3,000 one draws
first). `MakePacket` (main 0x00106dd0) sends two strips for each:
- a context-2 strip that copies the whole 512 x 448 picture being drawn
  into the texture (bilinear, no tests, no blend, Z masked);
- that texture drawn back over sysLayer's view box (UVs its whole size,
  bilinear), blended `(Cs - Cd) As + Cd` at 0x48, alpha NEVER with
  FB_ONLY, Z GEQUAL at the depth's Z with no Z write.

So each pixel farther than 3,000 takes 56% of a quarter-size copy, and
beyond 6,000 again of an eighth-size one: the distance goes soft. sysLayer,
made at start-up, draws after `GO`'s objLayer of the same priority 0
(piney-desktop's `Layers::older`), so the sky, the ground and the objects
soften while the characters (charLayer) and the effects stay sharp. The
port draws them as `piney_desktop::noiz::Sampling::shade_own`'s strips on
`TexRef::ScaledFrame`, a texture the renderer reads through the smaller
copy; the strips match what the game's `MakePacket` sends in eemu (places,
Z, UVs, colour).

The lens flare in a field is `LENSFLARE::Draw` in mode 1:
`ccCheckCameraDeg` is asked about the camera's own place, so it tests only
that the camera looks within 67.5 degrees of +x; the sun is the background
dummy taken to the player's side of the map, each flare wrapped likewise.

**The game's slips, kept.**
- `DrawRain` sets `WORLD` +0x1c where it means the drop's "placed" flag,
  so every drop is placed again every frame (50 x 2 draws).
- A SNOW's height asks `WORLD_MAN::GetHeight(x, y != -500)`: the ground at
  y 0 or 1.
- `WORLD::Init`'s SNOWs ask `GetHeight` before `Generate` has made the map:
  heap garbage in the game (zeros in eemu, and in the port). They ask about
  `WORLD_MAN`'s centre as the last area left it (`GO` does not set it); the
  port takes the start position.
- BIRD's far test is `sqrt(d.y + d.y + d.x^2) > 9,600`.
- The 2D smoke's transparency is an int: it goes from 1 to 0 in a frame.
- The bolt's sideways offset `fieldrand(6000) - 3000` is converted as
  unsigned: a negative one lands near 2^32.
- `FIREFLY2::SetBasePosition2` asks the height at the offset, not at the
  place.

**The port.** `piney_world::field_ambient` makes and steps all of it:
`Ambient::init` and `generate` at `FieldArea::with_world`, `Ambient::draw`
a frame's pictures as a list of `Op`s in the game's order (sprites,
`effSmoke` puffs, the 2D smoke's masks, models, sounds, flashes);
`field_firefly` is `FIREFLY`, `firefly` `FIREFLY2`. `FieldArea` keeps
`fieldrand` where `Generate` left it and the drawing's function statics
(the steam's, the thunder's timers, the balloon's scrolls) across fields
as the game does; `ambient_frame` runs a frame (the last one's list again
while the tasks sleep) and `ambient_models` draws the haze (the frame
buffer as its texture), TOBJ and BIRD and the 2D smoke. The sprites go
through piney-effect (`piney-game fx/ambient.rs`: each `ccEff` made once as
`Init` left it, fog and depth test as the game set them), the puffs through
`effSmoke`, the sounds and the flash through the area's events.

`tools/test_field_ambient_rs.py` runs the game's `WORLD::Init`,
`Generate` and `Draw` (with `DrawEffect`, `DrawRain`, `DrawSnow`,
`DrawSteam`, SNOW, FIREFLY, FIREFLY2, TOBJ, BIRD and the lens flare as they
are) in eemu and the `ambient_probe` example side by side: every field
type and background row, hacked and not, a random server and seed, and a
walk of the player and the camera; compared are `fieldrand`'s count after
`Init` and after `Generate`, what they made, and each frame's pictures in
order (names, places, patterns, scales, transparencies, fog bits, layers,
matrices, the masks, the puffs, the sounds with TOBJ's hum, the flash)
with the counts of `fieldrand` and `ccRand`. 124 cases of 1,000 frames: 0
mismatches.
`piney-game --mode field:14/TYPE,ROW[,HACK[,SEED]]` shows another field's
weather over story area 14.

## EE floating point

The hills are floating-point, so matching them needs the EE FPU's rules, as
`tools/eemu.py` now implements: no denormals (exponent 0 is zero), no
infinities or NaNs (exponent 255 is an ordinary number), overflow saturates
to the largest float, division by zero gives it signed, results truncate
toward zero, and `madd`/`msub` round the product first. Real hardware
sometimes differs from exact truncation in the last bit; that is not
modelled and has not shown up in any case checked.

## Other volumes

Each volume's own `WORLD::Init` and `WORLD::Generate` in eemu against
`tools/field.py`, per volume MUT, OUT, QUA:
- all 110 type and weather pairs;
- 33 random fields (every type three times);
- all 112 typeable story areas.

0 mismatches. `FIELD`, `WORLD` and the grid offsets are the same on all
four, as are every constant, the six object tables and `SmallMeshName`.
Differences:
- `ccGetDist` in OUT and QUA (OUT `0x001e6e70`) uses the FPU's `sqrt.s`
  instead of newlib's `sqrtf`. `field.py` picks by scanning the function.
  No case has shown a difference (grid distances never land within an ulp
  of 8000 or 7000).
- `IsProtectArea` returns the area-71/47 substitutes and searches 127
  records. From MUT on, area 100's `protect[1]` is 0, so 27 areas are
  protected instead of 28.
- From MUT on, `saveData+0x6772 == 1` or `WORLD+0x2c == 3` selects other
  BG tables in `Init` and `Generate`. This has no effect on the RNG or
  the layout.

The generated tables of the four volumes are equal but for the square
root (`volume.rs`'s `each_volume_has_its_tables`). `placement::field` reads
`WORLD::Generate`'s constants from the code, and Outbreak's and
Quarantine's compiler moved them: `this` in `$s2` (Infection's `$s0`,
found from the prologue's `move`), the objects' percentage in `$f1`
(Infection's `$f20`), a nop after the `SetDungeonEnter` call's delay slot,
and the hill loops' counters in `$s3`/`$s4` the other way round (taken
from each loop's decrement).

## Unknown

- How the fog table's near, far and percentage map to the GS fog.
- The water's run-time texture and UV scroll, the objects' own draw code,
  and the type-6 `FOBJECT2`s `DrawMesh` draws.
- Whether a tile's alpha is clamped beyond 7,200.

- `WORLD_MAN::EntryGimmick`'s `SetFood`, `SetMagicCircle` and
  `SetSpecialObj` ([battle.md](battle.md#where-the-entries-come-from))
  draw `fieldrand` on the entry task's first slice. That task (priority 64)
  runs before `ccThFieldDisp` (96) in the first walk, so its draws follow
  `Generate`'s and come before the weather's. The port makes them on the
  frame of `Play(0)`, before the field's first draw. Type 7's fires are
  still found as the field is made.
- Object heights and everything after the start position.
- The port's `effSmoke` puffs start at the next frame's effect task, a
  frame after the game's (its `WORLD::Draw` runs before `ccThParticle`).
- BIRD's clump is drawn as its model at the bird's matrix, and the
  weather's sprites are drawn unfogged (as all the effects' are).
- The shots (rain, snow, night with fireflies, the bird, type 7's fires,
  type 0's steam, hacked) are the port's; no frame of the game's own
  picture was compared.
