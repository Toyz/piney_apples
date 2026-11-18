---
title: Animation playback
status: partial
volumes: INF, MUT, OUT, QUA
covers: INF SLUS_202.67:0x00150670 ccAnm::SetAnmCtrlWork (its 0x0605 case), 0x00105be0 EigenVector, 0x00105e00 EigenVec2Mat, 0x00105ec0 m2a, 0x00105f80 a2m, 0x00146e30 ccAnmCtrlRot_Init, 0x00122b00 __ieee754_atan2, 0x001269d0 atan, 0x0014e950 ccStream::DecodeF_Obj, 0x00138120 ccCoord::SetMatrix_PosRotZYXScale, 0x00144e90 ccAnmChunk::ConvLCNum2ALCNum, 0x00144d00 ConvALCNum_FsetMatCtrl, 0x00146be0 ccAnmCtrlFVec3_SetCtrl, 0x00146890 ccAnmCtrlFVec3_Set, 0x00146530 ccAnmCtrlFloat_Set, 0x00146660 ccAnmCtrlFloat_SetCtrl, 0x001470c0 ccAnmCtrlRot_SetCtrl, 0x00146f30 ccAnmCtrlRot_Set, 0x00150670 SetAnmCtrlWork, 0x00152270 ccAnm::_AnimateForward, 0x0014e6e0 DecodeF_Morpher, 0x0013af10 ccMorpher::Modify, 0x001109b8 _sceVu0ecossin, 0x0014e1b0 ccStream::DecodeFrameChunk, 0x0014f830 ccStream::DecodeF_Note, 0x00147ff0 ccAnmNote::DelAll, 0x00152210 ccAnm::NoteProcess, 0x00150f50 ccAnm::SetAnm, 0x0014fc80 ccAnm::ccAnm, 0x00152400 ccAnm::_AnimateFrame, 0x00378904 funcDefaultNoteProcess, 0x001494c0 ccStream::PlaySceneMain, 0x001474b0 ccStream::MakeAnimeIndex, 0x0014fe10 ccAnm::DeleteAnmIndex, 0x001524d0 ccAnm::Draw
worklog: 32, 363, 365, 612
---

# Animation playback

How `ccAnm` evaluates an Anime chunk at a given time: object poses, texture
offsets, playback and looping, morph blending, and the notes a step hands
to the character's note function. `tools/anim.py` is the
reference, checked against the game's own functions in `tools/eemu.py` by
`tools/test_anim.py`. The port is `piney_data::anim`, checked against the
reference.

## The chunk

```
Anime (0x0700)   u32 object, u32 frames N, u32 words, then sub-chunks:
  Top 0, the controller records, Top 1 .. Top N-1 each with that frame's
  records, and an end Top: -1 plays once (3,138 of 4,091 in DATA.BIN),
  -2 loops (953)

0x0102 object    u32 obj, u32 flags, then one controller per 3-bit field:
                 bits 0-2 position, 3-5 rotation (degrees), 6-8 scale,
                 9-11 transparency (ccCoord::localtp, +0x88)
0x0202 material  u32 material, u32 flags; bits 0-2 U, 3-5 V (floats)
0x1901 F_Morpher u32 morpher, u16 count, pad, count x (u32 target, f32 weight)
                 applied from its frame on
0x0108 F_Note    u32 object, u32 event, u32 param: see Notes
0x0101 F_Obj     u32 object, u32 (never read), f32 pos[3], f32 rot[3]
                 (degrees), f32 scale[3], f32 transparency, u32 flags: see
                 F_Obj
also             0x0601 F_Ambient, 0x0502 F_Camera, light controllers
                 0x0603 / 0x0605 / 0x0607 / 0x0609: see Cameras and lights
```

Controller kinds are 0 (absent), 1 (one value) or 2 (keyed: `u32 n`, then
`n` × `(u32 frame, value)`), and every keyed controller has at least two
keys. An absent controller means position 0, identity rotation, scale 1,
transparency 1. Every controller record sits under Top 0; `ConvLCNum2ALCNum`
(0x00144e90) compiles them at load.

## Floats and vectors

`ccAnmCtrlFVec3_SetCtrl` / `ccAnmCtrlFloat_SetCtrl` turn the keys into a start
value and (u32 duration, delta) segments, with time in 1/256 frames:

```
if first key frame > 0: a hold segment up to it
per later key:          (256 * (frame - previous frame), key - running sum)
at the end:             a hold of N*256 - last*256
```

`_Set` (`ccAnmCtrlFVec3_Set` 0x00146890, `ccAnmCtrlFloat_Set` 0x00146530)
walks the segments while a segment's duration is below the remaining
time, adding its delta. Then:

```
value = acc + delta * (u32tof(t) / u32tof(dur))
```

All of this is in EE single precision. So interpolation is linear, holding
the first key before it and the last after it. Durations are u32
differences: a key that goes back in time (a few hundred controllers)
makes a segment of about 2^32 that is never passed, and two keys on one
frame make a segment of 0 that always is. The value depends only on the
target time, not on the steps taken to reach it.

## Rotations

Rotations are not interpolated as Euler angles
(`ccAnmCtrlRot_SetCtrl` 0x001470c0, `_Set` 0x00146f30). All of it runs in
the EE's single precision and VU0's macro mode, except one double `atan2`:
- **Keys.** Each key's degrees are quantised to 1/65536 of a turn,
  `s16(int(deg * 182.044))`, wrapping. Infection and Mutation truncate
  through `fptosi`, Outbreak and Quarantine through `cvt.w.s` (OUT
  0x00145c04). The two agree for every key on the four discs: none is past
  ±1,800 degrees or has exponent 255. The key's matrix is
  `sceVu0RotMatrix`, i.e. Rx·Ry·Rz.
- **Segments.** Each segment stores the step between keys,
  `sceVu0MulMatrix(R_i, R_{i-1}ᵀ)`, as an axis and an angle (`m2a`
  0x00105ec0), in a 32-byte record with its duration.
- **`EigenVector`** (0x00105be0) takes the stored rows of `m − I` (`sub.s`
  on the diagonal). For each row `i` and column `j` it forms the cofactor
  `(i+1, j+1)·(i+2, j+2) − (i+1, j+2)·(i+2, j+1)` (`mula.s`, `msub.s`). Its
  `fabs` runs through `fptodp`, `fabs` and `dptofp`, which only clears the
  sign. The row of the first strictly largest cofactor wins. The axis is
  `sceVu0Normalize(sceVu0OuterProduct(row i+1, row i+2))`, and row `i+1`
  (w 1) goes back too. Below 0x2d2febff (about 1e-11) it answers −1, and
  `m2a` stores zeros: no turn.
- **`EigenVec2Mat`** (0x00105e00) builds the frame `F` whose stored columns
  are the axis, that row and their cross product, each normalised again.
- **The angle** is `atan2(L[1][2], L[1][1])` of `L = F · (m · Fᵀ)` (two
  `sceVu0MulMatrix`). The call is newlib's double `atan2`
  (`__ieee754_atan2` 0x00122b00, `atan` 0x001269d0) between `fptodp` and
  `dptofp`. Its soft-float steps (`dpadd`, `dpsub`, `dpmul`, `dpdiv`) round
  to nearest as IEEE doubles do, so the result is that of one f64 per
  step, rounded to a float. The angle is at most half a turn.
- **`a2m`** (0x00105f80) turns (axis, angle) back into a matrix from
  `sinf` and `cosf` of the angle (newlib's float routines). The cross terms
  are `sceVu0ScaleVector(aa, s)`, negated by `neg.s` on one side of the
  diagonal. The outer rows are `(aa · aa[r]) · (1 − c)`, added last to
  `c` or the cross term. Row 3 and the w lanes stay the unit matrix's.
- **Playback.** `ccAnmCtrlRot_Init` copies the first key's matrix into
  the work. Each call adds its step to the time held in the work. While a
  segment is strictly shorter than that time, `_Set` multiplies its whole
  turn on the left (`sceVu0MulMatrix(a2m(turn), W)`) and takes its
  duration off. Then it returns `a2m(axis, angle · frac) · W`, where
  `frac = u32tof(t) / u32tof(dur)` as for the floats (`div.s`, truncated).
  The running matrix depends only on the segments passed, so many small
  steps give what one large step does.
- **Final hold.** The hold after the last key reuses the last step with the
  wrapped duration `(last − N) · 256`, so the pose creeps by
  `angle · t / 2^32`, too little to see. The port keeps it.
- **Single values.** A single-value rotation is
  `sceVu0RotMatrix(deg · π / 180)`, not quantised.

`sceVu0RotMatrix` takes its cosine from a polynomial (`_sceVu0ecossin`
0x001109b8, coefficients at 0x002f7520) and its sine as `sqrt(1 − cos²)`,
which is off by up to about 5e-4 near 0. That is the game's error, and the
port keeps it.

All of these routines are the same on the four volumes. Mutation's are
Infection's at other addresses. Outbreak's and Quarantine's are recompiled
with the same operations and order (`m2a` OUT 0x001061b0, `a2m` 0x00106270,
`EigenVector` 0x00105ec0, `_Set` 0x00145930). The port is
`piney_data::anim` (`m2a`, `a2m`, `eigen_vector`, `RotKeyed`), with the
double `atan`/`atan2` in `piney_data::libm`. Each segment keeps the running
matrix it starts from, so a pose costs one `a2m` and one product. Every
value matches bit for bit, as far as eemu models the EE and VU0: positions,
scales, transparencies, texture offsets, rotations (single-value and
keyed, all 16 words), the local matrices, and morph weights and blends.

The local matrix is `T(position) · R · S(scale)` (`SetAnmCtrlWork`
0x00150670): `sceVu0MulMatrix(R, S)`, then `sceVu0TransMatrix` adds the
position (`Pose::bits`, which `Pose::matrix` reads). The rotation's w lanes
and fourth row only add zeros there, so its 3x3 decides every word. The
pose goes to the object in `ccAnmIndex.subst`; ExtObj
targets are followed (`ccGetExternalIndex` 0x00101a50), and each record is its
own instance.

A clump's nodes can be ExtObj copies too: `ccStream::GetChunkAdrsF(name)`
(main 0x00144580) walks the stream's index (skipping `#` files) with
`ccMatchIndex` (0x00101ad0), which follows an ExtObj entry to the object
it copies. In the Grunties' files (`cdogbod*`) the first `CMP_trall` is an
animation's (`cdgaevo0.max`), whose 23 nodes are ExtObj copies of the
model file's objects. The port's `Body::of` takes the first object of the
clump's name and follows its nodes' ExtObj copies to their targets, so
such a body poses and draws the model's own objects (worklog 162); a
body whose clump's nodes are its own objects is unchanged.

## Playback

- **Time.** Time is `frameNow · 256 + frameCnt`. `_AnimateForward` (0x00152270)
  adds `ccAnm::frameSpd` (256, one frame per call, by default) and clamps to
  `(N − 1) · 256`. It returns 1 when a play-once animation ends.
- **Looping.** An animation that loops (-2) is posed at `N − 1` when it
  gets there, then reset to 0. So a loop is `N − 1` frames long.
- **Per-frame records.** Morph weights and notes apply only when the whole
  frame changes, never interpolated, and frame 0's records are never
  applied: `ConvLCNum2ALCNum` leaves out Top 0's records and Top 1, so the
  frame data starts with frame 1's records. A step that changes the frame
  calls `DecodeFrameChunk(old, new, 0)` (0x0014e1b0), which reads on from
  where the last read stopped until it reads a Top past `new`: the records
  of frames `old + 1 .. new`, then Top `new + 1` (or the end Top, whose -1
  or -2 `_AnimateForward` acts on). Every Anime chunk in DATA.BIN has a Top
  for every frame (4,091 of 4,091), so this depends only on the two frames.
- **Frame rate.** `ccSystem::frameRate` is 1 at boot, demo, desktop and
  toppage, and 2 after `ccSetupNewGame`. So the field probably runs at 30
  frames a second (inferred).

## Texture offsets

`SetAnmCtrlWork` writes `ccMaterial.u/v = s16(int(x · 4096)) − cropU / cropV`
from a 0x0202 record's U and V (21 files). See the Material chunk's crops on
[the CCSF page](../formats/ccs.md). The draw adds them to each vertex's
texture coordinates through the model packet's STROW, `(u >> 4) & 0xff` in
1/256 of the texture ([rendering](render.md), [the desktop](desktop.md)):
the Chaos Gate, the fields' waters (`field_a`, `field_c`), Aura's body, the
virus cores, the desktop and the title scroll this way.

## F_Obj

An F_Obj record (0x0101, 13 words) gives an object its whole transform
from its frame on. `ccStream::DecodeF_Obj` (main 0x0014e950) reads it when
`DecodeFrameChunk` reaches that frame, like the notes and morph weights, so
frame 0's record is never read. For a plain object it calls
`ccCoord::SetMatrix_PosRotZYXScale` (0x00138120):
`T(pos) · sceVu0RotMatrix(π · deg / 180) · S(scale)`, the composition the
controllers use with the record decoders' radians. It also stores the
transparency, clamped to 0..1, in `localtp` (+0x88). Flag bit 0 goes to
the object's +0xa2, with bit 1 set unless the position is zero and +0xa1
bit 0 is set. For an effect object (0xe00) it restarts the object's
animation.

Only four files have F_Obj records:
- **Three enemies.** `eex1`, `eii1` and `elgx` move their body only this
  way in their damage, down and magic clips, one record a frame, with no
  controller on the body. `eex1`'s `ANM_eex1dmg0` lifts it 3.875 a frame
  and tips it 7.5° about y and −7.5° about z.
- **The desktop.** `xddesk01` has them too.

The port reads them into `Animation::objs`. `obj_poses_at(time)` gives each
driven object its last record read by then, and `locals_at` and
`transforms_at` use them for the objects no controller track drives. The
enemies' transparency (`Foe::node_alphas`) takes theirs. Before this, those
three enemies stood still as they took hits and died.
`piney-data`'s `f_obj_records_pose_the_body` checks `ANM_eex1dmg0`.

## Cameras and lights

The Anime chunks that carry F_Camera (0x0502) and the light records are
played by code that reads those itself, not by `ccAnm`'s pose:
- **Cameras.** The streams' scenes (`piney-stream`'s `scene.rs`) and the
  desktop's and the title's clips (`piney-desktop`'s `anm.rs`, its
  `camera`).
- **Ambient and lights.** F_Ambient (`ccAnm::GetAmbient`: each byte
  over 255) and the distant (0x0603), direct (0x0605), spot (0x0607) and
  omni (0x0609) light records come through `piney_data::light::
  anim_lights` (`town::anim_lights` in piney-world). By flag: bits 0-2
  position, 3-5 rotation, 6-8 colour, 9-11 intensity, 12-14 and 15-17 the
  fall-off's or the beam's start and end, 18-20 and 21-23 a direct
  light's two radii or a spot light's two half angles in degrees, as
  `SetAnmCtrlWork` (main 0x00150670) reads them; the floats and the place
  as `ccAnmCtrlFloat_Set` and `FVec3_Set` (above), a colour by
  `ccAnmCtrlColor_Set` (0x001461e0: `ccSetColor` of a constant, or
  between keys `ccSetBlendColor`, each colour by its share). They serve
  the towns' light tables, the Chaos Gate, the story maps (`evarea`,
  `evarea_b0`) and the dungeons' special rooms; every light animation of
  the four discs' `DATA.BIN` is checked word for word against the game's
  `ccAnm` (`tools/test_lights_rs.py anims`, [the light
  group](field-game.md)). Lia Fail's `LGT_fspot01` is the only spot light
  record, and no light table names it. The fields take frame 0's ambient
  and distant light (`piney-data`'s `field/render.rs`).

## Frame speed

The field changes `frameSpd` (`ccAnm` +0x9c), which is 256 by default:
- **Kite.** `ccPlayer::AnimCtrl` sets it by act and speed (`piney-battle`'s
  `kite.rs`).
- **The town player.** The same (`piney-world`'s `motion.rs`).
- **Enemies and members.** Their motion code sets it
  (`combat/cast.rs`).
- **The Administrator.** His act −5 sets 384 (`merchant.rs`).

## Morphing

`DecodeF_Morpher` (0x0014e6e0) sets a morpher's targets and weights.
`ccMorpher::Modify` (0x0013af10) blends in integers:

```
target positions at another vertexScale:  int(t * (tscale / bscale))
weights:                                  s16(int(w * 4096))
each component:  base + ((Σ s16(t − base) * w) >> 12)   sum wraps at 32 bits,
                                                         the add saturates
```

Normals are not morphed.

## Notes

```
0x0108 F_Note   u32 object, u32 event, u32 param          after its frame's Top

ccAnmNote (24 bytes, operator new)   made by ccStream::DecodeF_Note (0x0014f830)
  +0x00 next      the list's link
  +0x04 event     the record's
  +0x08 param     the record's
  +0x0c time      the frame being read: the step's old frame for frame
                  old + 1's notes, the note's own frame after that
  +0x10 coord     ccAnmIndex[object].subst: the object's instance
  +0x14 ccstag    ccAnmIndex[object].ccstag
```

DATA.BIN holds 2,697 F_Note records, all of three words and all naming
`OBJ_xnote`, in 944 of the 4,091 Anime chunks (171 of them loops). Events:
2 (1,976), 0x8005 (487), 0x8003 (175), 0x8002 (57), 3 (2). 466 frames hold
more than one note, 16 notes sit at the last frame and 3 at frame 0 (`ecm1`
`ANM_ecm1mag0`, `eug1` `ANM_eug1dwn0`, `eugz` `ANM_eugzdwn0`), which are
never handed on.

- **The list.** `DecodeF_Note` pushes each note on the head of
  `ccAnm.noteRoot` (+0xa0), or of the stream's own `noteRoot` (+0xb4) when
  no anm is reading.
- **Collected.** `_AnimateForward` first frees the list
  (`ccAnmNote::DelAll` 0x00147ff0) and sets `noteRoot` to 0. Only a step
  that changes `frameNow` reads, so the list then holds the notes of frames
  `old + 1 .. new`, `new` clamped to `N − 1`: the last frame's notes come
  with the step that reaches it. A zero step, a step inside a frame, and a
  step held at a play-once animation's last frame leave it empty. A loop's
  reset rewinds the reader, so each pass hands on frames `1 .. N − 1`
  again.
- **Delivered.** `ccAnm::NoteProcess` (0x00152210) walks the list from the
  head and calls `funcNoteProcess` (+0xa4) with each note, or does nothing
  when that is 0. The list is last in, first out, so the notes go out in
  reverse file order: the newest frame's first, and a frame's last record
  first. Kite's `ANM_ctu1atc0` (`ctu1body`) has (2, 2) then (0x8005, 1) at
  frame 5, so NoteProcess hands on the hit (0x8005) before the sound (2).
  `ccStream::PlaySceneMain` (0x001494c0) walks a stream's list (+0xb4)
  from the head the same way, through the stream's function (+0xa8).
- **Kept.** Neither `NoteProcess` nor `ccAnm::SetAnm` (0x00150f50) touches
  `noteRoot`. The notes stay until the next `_AnimateForward` (or
  `~ccAnm`): a second `NoteProcess` hands them on again, and one after a
  `SetAnm` with no step between hands on the old clip's. `ccAnm::ccAnm`
  (0x0014fc80) sets `funcNoteProcess` from `funcDefaultNoteProcess`
  (0x00378904, 0 in the executable). `_AnimateFrame` (0x00152400) frees
  the list when the frame asked for is the current one, and otherwise
  clears `funcNoteProcess` around its own `_AnimateForward` (which does
  not call it).
- **Note functions.** The characters call `SetAnm`, `_AnimateForward`, then
  `NoteProcess` (for example `ccPlayer::AnimCtrl`, gcmn 0x005993c0).
  `ccEnemyCheckNote` (gcmn 0x004328b0), `ccPlayer::CheckNote` (0x0059c300),
  `ccFellow::CheckNote` (0x0041e1e0) and `ccSkillCheckNote` (0x00573a10)
  switch on `event` and read `param`. A scan of every function that takes a
  `ccAnmNote *` finds only `ccSetStreamDemoNote` (main 0x00184340) reading
  `time`, `coord` or `ccstag`.

`tools/anim.py` (`Animation.passed_notes`, `forward_notes`) and the port
(`Animation::passed_notes`, `forward_notes`, `piney_world::chara::pose::Play::forward_notes`)
give the notes in delivery order. `tools/test_anim.py notes` runs each
animation with notes through the game's `_AnimateForward` and
`NoteProcess`, with a hooked `funcNoteProcess` recording each note:
- **Steps.** Random steps: 0, one frame, odd values within a frame, several
  frames at once, and past the end.
- **Restarts.** A play-once animation that has ended is restarted half the
  time, the way `_AnimateForward` restarts a loop.
- **Compared.** Each note's event, param, time and object, in order, the
  frame counter and the return value.
- **Result.** All 944 animations: 131,577 steps (13,288 zero, 4,019 loop
  resets, 15,529 restarts), 60,759 notes (11,812 steps with several), 0
  mismatches. A second `NoteProcess` gave the same notes all 26,586 times
  it was tried.

## The anm's objects

`ccAnm::SetAnm` (0x00150f50) makes the anm's objects from the chunk's
index, and `ccAnm::Draw` (0x001524d0) draws those and nothing else.

- **The index.** `ConvLCNum2ALCNum` (0x00144e90) lists each chunk the
  records name once, in the order first named, by local chunk number: an
  ExtObj copy is its own entry. Objects are named by the object records
  (0x0102), F_Obj (0x0101) and F_Note (0x0108). `MakeAnimeIndex`
  (0x001474b0) keeps the list at chunk +0x1c (count +0x20). At +0x28 it
  keeps each Obj, ExtObj, 0xd00 and 0xe00 entry's parent slot: the entry
  whose chunk is its chunk's parent, else -1.
- **SetAnm.** It makes one 12-byte entry per index entry at anm +0xac:
  +4 the object, +8 the type of the chunk it names (an ExtObj followed by
  `ccGetExternalIndex`), +10 flags (1). Each external index's +0x30 points
  at its entry, so of several entries naming one piece the last wins.
  Each Obj node of the anm's clumps (+0xa8) with an entry takes it (+4 the
  node, flags 6). Every entry still flagged 1 gets a new `ccObj` from its
  chunk (`ccObj::Init`, flags 7; `DeleteAnmIndex`, 0x0014fe10, frees
  those). Each made object whose model is bone or skin (model +0x20 & 6)
  then gets the model's node table (0x00151950-0x00151a04): the model
  chunk's node list (+0x44, count +0x48), each node the object of its
  entry, and model +0x24 |= 0x40. From 0x00151a24, each entry with bit 2
  hangs from its parent slot's object, or from the anm itself for -1. The
  +0x30 pointers are cleared at the end.
- **Draw.** Each entry with bit 4 is drawn: type 0x100 by
  `ccObj::Draw(anm +0x88)`, 0xe00 by `ccEffObj::DrawNoAnm`.

So a clump node that no record of the clip names is not drawn. The
Noisy Wisp's file (`eww1`) holds eight bodies and every clip names one;
the mimic (`etn1`) waits as a plain chest, its legs and fangs named only
by the clips that show them. A piece named by several ExtObj copies is
drawn once per copy, each posed by its own record under its own parent:
- `ecc1`'s four legs come from one model;
- `evb1` is a swarm of four bees;
- `eus1` holds a second sword in its hand while it draws.

In Infection's DATA.BIN, 185 clump and clip pairs leave a node with a
model unnamed: the enemies `efm1`, `eks1`, `ekx1`, `elg1`, `epx1`,
`etn1` and `eww1`; the bosses' `x04`, `x31`, `x41`, `x51`, `x61`, `x71`
and `x81`; the title's, `xdttopen0`'s and `drain`'s scenes; and three
room pieces (`field_b`, `field_b1`, `se2_2`).

A clump node several entries name is the last entry's object, so its
matrix and transparency, which skinning and the code that reads a node
use, are that entry's.

The port is `Animation::objects` and `Play::instances`; `Play::worlds`,
`foe::Model::worlds` and `Body::node_alphas` give a named node its entry's
pose. Each world matrix is `ccCoord::_SetLWMatrix`'s product (main
0x00138380, `sceVu0MulMatrix`) of the parent's and the local one, in VU0's
arithmetic from the anm's own matrix down (`body::root` is
`SetMatrix_PosRotZYX`, 0x001382f0: `sceVu0RotMatrix`, then
`sceVu0TransMatrix`), and a node's transparency the anm's times its own by
`mul.s`. `tools/test_foe_rs.py` and `tools/test_world_rs.py` compare every
word of the nodes' and the hands' matrices. The desktop's anm (`piney_desktop::anm`) and the effects'
(`piney_effect::nodes::anm`) already drew by entry. `tools/test_foe_rs.py`
checks the index against the game's `ConvLCNum2ALCNum` (`index`), and
nodes named through several copies (`ecx1`, `ecc1`, `eus1`, `evb1`)
against the game's poses (`pose`).

MUT's `SetAnm`, `Draw`, `MakeAnimeIndex` and `ConvLCNum2ALCNum` match
INF's except for data addresses. OUT's and QUA's are recompiled but keep
the same flag logic. The `index` check passes on all four volumes.

## Unknown

- F_Obj's flag byte (+0xa2) is not modelled. Its effect-object branch
  (0xe00) is, for the streams ([stream.md](stream.md#effect-nodes)). Neither is whether a pose F_Obj left survives into the next
  clip when that clip does not drive the object (the port starts every
  clip from the rest pose).
