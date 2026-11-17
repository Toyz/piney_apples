---
number: 611
title: The light group to the bit on every disc
date: 2026-11-17
area: render, world, video, volumes, test
files: crates/piney-data/src/light.rs, crates/piney-data/src/vu0.rs, crates/piney-data/src/field/ee.rs, crates/piney-data/src/anim.rs, crates/piney-world/src/chara/kite.rs, crates/piney-world/src/town/mod.rs, crates/piney-world/src/dungeon/mod.rs, crates/piney-stream/src/scene.rs, crates/piney-stream/src/draw.rs, crates/piney-demo/src/draw.rs, tools/test_lights_rs.py, tools/test_stream_rs.py, tools/test_demo_rs.py, docs/engine/field-game.md, docs/engine/stream.md, docs/engine/animation.md, docs/engine/dungeon.md, crates/piney-world/src/systems/statics.rs, crates/piney-world/src/story_map/disc.rs, docs/engine/evarea.md, tools/envscan.py, tools/test_dungeon_rt.py
started: 2026-11-17T03:15:55-07:00
took: 10h 3m
---

# 611. The light group to the bit on every disc

`tools/test_lights_rs.py` compared the port's light group with the game's
within 1e-4 for the directions and 1e-5 for the colours; compared exactly,
309 of 400 random groups differed in the last bits. The port did the light
maths in glam f32 where the game does it on the EE's FPU and in VU0 macro
mode, then on VU1. It now does every op the game does, in the game's order,
and the harness compares every word.

## The game's ops

Each kind's `CheckRange` (INF main addresses; the later discs' are the same
code but for one op):

- **Omni** (0x00139830): nothing at intensity 1/128 or below. `d = at - pos`
  (`sceVu0SubVector`, all four lanes), `sceVu0MulVector(d, d)`, the sum
  `z + (x + y)` by `add.s`. With a fall-off end (+0xc8) not 0: nothing past
  `d2 <= end2` (+0xcc, the square `SetAnmCtrlWork` stored by `mul.s`), the
  distance by `sqrtf`, full to the start, else `(i (end - dist)) / (end -
  start)` (`sub.s`, `mul.s`, `div.s`) and nothing at 1/128 or below. The
  direction `sceVu0Normalize(d)`, the colour `sceVu0ScaleVector(colour, f)`.
- **Direct** (0x00139330): `sceVu0InversMatrix` of its lwMatrix, the point
  through it (`ApplyMatrix`), `z = neg.s` of its z; nothing behind
  (`c.lt.s z, 0`) or past the end; `x x + y y` by `mula.s`/`madd.s`; nothing
  past `r1 r1` (+0x154); the fade along the beam, then `sqrtf` of the
  radius and the fade from r0 to r1; nothing at 1/128 or below. The
  direction `lightVector` (0, 0, -1, 0) through the lwMatrix.
- **Spot** (0x001395b0): the same point, `sqrtf(x x + y y)`, nothing past
  `tan1 z` (+0x158), the fade along the beam, then between `tan0 z` and
  `tan1 z`.
- **Distant** (0x00139170): `lightVector` through its lwMatrix, kept at
  +0xd0, the colour times the intensity.

`SetLightMatrix` (0x00105900) keeps three (priority, row) records: a light
above the lowest one's priority that answers takes its place and the row
of the record it pushes out; its direction goes in that row with w 1, its
colour with w 0, the ambient in row 3. `sceVu0NormalLightMatrix` scales
each row by -1, `Normalize`s it and transposes. `ccSetMatrixPacket`
multiplies that by the model's lwMatrix (`sceVu0MulMatrix`, VU memory
16-18). VU1's `mc_SetMatrix` sums each light's lane squared (`mula`,
`madda`, `madd`), multiplies by `ERSQRT` of it, and the colours by `loi
2.0`.

By disc: `SetLightMatrix`, `sceVu0NormalLightMatrix`, the libvu0 routines,
`ccSetMatrixPacket`'s light part and `mc_SetMatrix` are the same code on all
four (Outbreak and Quarantine recompiled with other registers; the FPU op
sequences compared one by one). The one difference is the square root in
the three ranged `CheckRange`s: newlib's `sqrtf` (`__ieee754_sqrtf`) on
Infection and Mutation, `sqrt.s` inline from Outbreak on (OUT 0x00137d74
omni, 0x00137900 direct, 0x00137b28 spot; QUA alike): `libm::sqrtf_on`.
`lightVector` is (0, 0, -1, 0) on every disc.

## The lights' values

What the group holds comes from more code, and all of it is ported the same
way:

- **`SetAnmCtrlWork`** (0x00150670): a distant light's local matrix is its
  rotation (`ccAnmCtrlRot_Set`), a direct or spot light's `TransMatrix` of
  rotation and place, an omni light's the unit matrix moved to its place;
  the colour `ccSetColor(c, 1)` or between keys `ccSetBlendColor`
  (`ccAnmCtrlColor_Set` 0x001461e0, its segments as
  `ccAnmCtrlColor_SetCtrl` 0x00146310 builds them, the share by `cvt.s.w`
  and `div.s`); the floats by `ccAnmCtrlFloat_Set`; each square by `mul.s`;
  a spot light's half angles `(pi deg / 180) / 2` kept as their `tanf`.
- **The parent.** `SetAnm` (0x001518a0) makes the anm the light's parent,
  so `_SetLWMatrix` takes the light through the anm's matrix (the gate's
  root; the unit matrix for the towns' and story maps' light anms, which
  their constructors never set: `ROOTTOWN01`, `EVENTAREA01`,
  `EVENTAREAB0`, `EVENTAREA02::ChangeBlock` checked).
- **`GetAmbient`** (0x001528e0): each byte `cvt.s.w`, `div.s` by 255.
- **The dungeon's distant light** is `ccLight(1, 0)` (gcmn 0x005b8c78):
  priority 0, not the -1 `ccCreateLight` gives. The port had -1. Its grey
  0.7 is copied in, intensity 1 (`Init`), `SetMatrix_RotZYX`.
- **The room lights** (`SetLight`, `ccLight(4, 1)`): `ccSetColor`, the
  fall-off 100 to 250 with its square 62500 stored as a constant, the unit
  matrix moved to the dummy's place. The symbol's light stores 1000 and
  1000000; the meteor swarm's 500 and 250000.
- **The streams**: `DecodeF_OmniLight` and `DecodeF_DistantLight` as
  above, and `DecodeF_DirectLight` (0x0014f130) and `DecodeF_SpotLight`
  (0x0014f3b0), which the port had skipped as "no stream on this disc
  records them". They do: Infection's and Mutation's str0090 and str0110,
  Outbreak's str1070 and others carry direct lights, Quarantine's str2062
  the four discs' only spot lights (1282 records). A direct light's
  `Init` (0x001392d0) leaves radii 1000 and 2000 and the square unwritten.

## The port

- `piney_data::light`: `Light` (`Kind::Distant { dir }`, `Direct { matrix,
  far, radius }`, `Spot { matrix, far, cone }`, `Omni { pos, far }`, the
  colour, intensity and priority as bits; `Falloff` with its stored
  square), `check_range`, `set_light_matrix`, `normal_light_matrix`,
  `light_regs` (to VU1's registers), `set_color` (moved from piney-effect,
  which had the only full port), `blend_color`, `ambient`, and
  `AnimLight`/`anim_lights` over anim.rs's controllers.
- `piney_data::vu0`: the libvu0 routines piney-world's `ee` had, plus
  `vmul`, `transpose`, `trans_matrix`, and `invers` (from the dungeon);
  piney-world's `ee` and `dungeon` re-export them. `rsqrt` (the EE's
  `rsqrt.s`, VU1's `ERSQRT` as tools/vu.py runs it) moves from piney-eemu
  to `field::ee`; piney-eemu re-exports it.
- `town::Light` is `piney_data::light::Light`; `TownLights` carries the
  ambient as bits and the disc (`volume`), which picks the square root.
  `chara::kite::light_matrix` hands the group to `light_regs` and turns the
  registers into `piney_draw::Lights`. The empty slots now carry VU1's
  directions too (their colour is 0).
- The producers: the towns, story maps, the gate (its root), the field
  (its light under the unit anm), the dungeon (priority 0) and its event
  room, the room lights, the battle's omni lights and the boss effects'.
- piney-stream: `scene::Light` holds a `piney_data::light::Light` the
  records set, the direct and spot records are decoded, and `Loaded` and
  `Scene` carry the disc. The title's light (piney-demo) goes through
  `light_regs` too.

## Checks

- `tools/test_lights_rs.py`: random groups of distant (priority -1 and 0),
  omni (0 and 1), direct and spot lights under random lwMatrices (turned,
  tilted, scaled), run through the game's `ccCreateLight`, `AddGrp`,
  `SetLightMatrix`, `ccSetMatrixPacket` and VU1's `mc_SetMatrix`; every
  register word of every slot against the port. Unit tests (400 groups)
  and bulk 2000 (3089 lit slots) and 3000: 0 mismatches on all four
  discs. The game run on Outbreak against the port told Infection's code:
  66 of 400 groups differ, so the square root is seen.
- `tools/test_lights_rs.py anims`: every light animation of each disc's
  DATA.BIN (158 on Infection, 159 on the others; 2490 and 2496 light
  states) played by the game's `ccAnm` (`SetAnm`, `_AnimateForward`
  steps through a keyed loop, `GetDirc`, `_SetLWMatrix`, `GetAmbient`)
  against `AnimLight::at`: 0 mismatches. The unit test runs the towns',
  the gate's keyed lights, the shrine's direct light, Lia Fail's spot
  light and se1_6's keyed colour.
- `crates/piney-stream/tests/stream.rs`: the stream fixture's
  `SetLightMatrix` lines are now compared word for word (1440 lit frames);
  `stream_lights_{inf,mut,out,qua}.txt` (`test_stream_rs.py lights`) hash
  the light matrix at every object drawn per frame of str0090, str1070
  and str2062: all equal, with 1200, 1200, 14223 and 22043 objects
  reached by a beam or a cone.
- `tools/test_demo_rs.py`'s icon lighting: VU1's registers word for word,
  1964 vertices (2628 on Quarantine), 0 off by one.
- The sweep on d52b23d: 356 runs, every one as on main
  (`summary-main.txt`), 0 regressions; test_lights_rs, test_world_rs,
  test_field_rt, test_dungeon_rt, test_demo_rs and the battle harnesses
  that draw characters among them, on all four discs.
- Battle pins (`pins.sh`, probe built at d52b23d in wt-lights): 291
  calls, 0 differ.
- Golden shots: a base from a clean worktree of main 124d93d equals
  `shots/base` (4550 files, 0 differ). The tip differs in 650 files: 235
  PNGs, every one by at most 1 in any channel and in few pixels (the most,
  238 pixels over 127 of event 11's church shots), and 415 `.frames`
  digests of the dungeon and field runs, in 1990 frames of which only the
  picture digest changed, never the events' digest or count. The draws'
  only changed input is `ModelDraw::lights`: the lit characters' colours
  move by the light group's last bits.
- The piney-game suite 349 passed, 0 failed, 153 ignored; the crate tests
  780 passed; clippy (all targets and features) clean; fmt; `piney-gen gen
  syms carry --check`; `cairns check`; `docs.py check`.

## The event room's light, and SetRoom's sp+192

The first pass left the event room's light at `750 (x + 4, y + 4, 0)`
with the rotation `SetMatrix_PosRotZYX` reads from SetRoom's sp+192, a
word no instruction of `SetRoom` writes before the call. Running `SetRoom`,
`RoomSelect` and `GotoNextRoom` in tools/test_dungeon_rt.py's machine,
the stack under the call filled with a sentinel first, and then
`_SetLWMatrix` of the light (DUNGEON+0x370), settles it:

- **The parent.** `SetAnm` (main 0x001518a0) sets every light's +0x80 to
  the anm, so the light stands under the room's anm, which the tail's
  `SetMatrix((pos, 0), (0, 0, rotate))` (0x005c37a8) places at the room.
  The port had no parent.
- **The place.** The tail's `_AnimateForward` (0x005c3734) runs
  `SetAnmCtrlWork` again when its step sets a pose, and that puts the
  record's place (0, 0, 2500) back: the beam stands 2500 over the room's
  centre, where `GotoNextRoom` stands the party (`OBJ_user_point`). Every
  event room's anm does so but `se3_3`'s (type 31, area 77): its two
  frames are already shown, so its light keeps (12000, 6750, 0) from the
  centre, 5500 outside the 12000-square room past its radius of 500.
- **The rotation.** x is a return address an earlier callee of `SetRoom`
  saved there (0x005c3a94 when `SetRoom` is called directly, 0x0014fdd8
  through the door), y is 0, both written inside `SetRoom`'s own calls;
  an address below 0x00800000 has exponent 0 and VU0 takes it as 0. z is
  0 through `GotoNextRoom` (its own callees write it), and through
  `RoomSelect`, `GO`, `Generate`, `SetItemBox` and `ShowMap` it is the
  caller's older word (the sentinel survived). It only matters in
  `se3_3`'s room, where any z turns the beam about its own vertical axis
  at a place no character can reach: no draw's registers depend on it.
- **Every row.** The four discs' EditDungeon rows of types 25 to 35 are
  the same ten; those with a light are 25 (areas 23 and 25), 29, 30, 31
  and 32.

The port puts the record's light at the anm's time under the room's root,
or, when the tail's step sets no pose (`Animation::forward`'s `pose_at`),
the row's place under it. tools/test_dungeon_rt.py now compares the light
the port adds (lwMatrix, colour, intensity, beam, radii) with the game's
after every event room's `SetRoom` and every door into one: 8 and 16
lights, word for word, on Infection and Quarantine (and the sweep's runs
on all four).

## The second group at ccDrawEnv +0x88

`tools/envscan.py` lists every store that can reach a `ccDrawEnv`'s +0x88
on a disc: from every way a `ccDrawEnv` pointer is built (the globals
`cc3d` and `active__9ccDrawEnv`, a `ccStream`'s environment at +0x158,
`this` in `ccDrawEnv`'s methods, `InitScene`'s argument, the object
`__ct__9ccDrawEnv` builds), followed through moves until overwritten,
over main, the four overlays and their static initialisers. Three stores
on each disc: `Reset` (INF 0x00105574) clears it, `ccAnm::Draw`
(0x001527bc) points it at the drawn anm's own light group (+0xf0) and
clears it after (0x001528ac); OUT 0x0015171c and 0x0015181c, MUT and QUA
alike.

An anm's own group holds the lights `SetAnm` made without
`SetLightEnv(1)` first. Every animation of the four discs' DATA.BIN with
light records was followed to the code that plays it (its name's
relocations): the towns', fields', event areas', event rooms' and the
Chaos Gate's set `SetLightEnv(1)` first, as do `STATICOBJECT`s; the only
one without is the desktop's `ANM_xddesk01`, whose 148 (151 from
Mutation) models are all unlit. The three files with both light records
and lit models (chgate, se1_6, se2_6) go through `SetLightEnv(1)`. So the
second group never lights a model on any disc, and the port needs none.

## The disc's light

Following that turned up a light the port lacked: a `STATICOBJECT`'s
`SetLightEnv(1)` before `SetAnm` (gcmn 0x005cfc54) puts its anm's light
records in the draw environment's group, the anm their parent, before
the area's own lights, and the object's `_AnimateForward` moves them. On
the four discs that is `EVENTAREAB8`'s disc (`ANM_se1_6ob1a`'s
`LGT_omn01`): an omni 100 below the disc, full to 600 and gone at 1200,
its colour keyed (0.3, 1, 1 at frame 1). `StaticObjects` now carries its
objects' light records and moves them with each step; the towns and
story maps put them in `TownLights::objects` before their step (the
characters draw before the area steps its objects), and `light_matrix`
walks them before the area's lights. The light records themselves are the
`anims` check's (se1_6's included).

## Checks of the second pass

- The sweep on 502aae4: 356 runs, every one as on main, 0 regressions;
  test_dungeon_rt's event room lights word for word on all four discs.
- Battle pins on 502aae4: 291 calls, 0 differ.
- test_lights_rs bulk 2000 and `anims` again: 0 mismatches on all four
  discs. The suite 349 / 0 / 153, the crate tests 780 passed (the disc
  test now checks the disc's light), clippy clean, fmt, piney-gen's
  checks, docs.
- Golden shots of 502aae4 against the same day's base: 708 files differ.
  Against the first pass's shots 59 do, every one from the two fixes: the
  disc fields (`story_map_shots` 09-11 on all four discs, 48 PNGs, and
  `kyvia_stage_shots`' 5), where Kite on the disc now takes its cyan
  light from below (the changed pixels are his alone, up to 232 in a
  channel); and the event rooms (`shrine_map_shots` f2-b2's 4 and
  `dungeon_shots` area 101's PNG and digest), whose lit models now stand
  in the beam over the room's centre. The rest is the first pass's: at
  most 1 in a channel.

**Still unknown:** nothing of this work. The two left open by its first
pass are settled above: the event room's rotation never reaches a draw,
and the second group never lights a model.
