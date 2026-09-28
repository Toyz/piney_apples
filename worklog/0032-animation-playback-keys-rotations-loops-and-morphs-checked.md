---
number: 32
title: Animation playback: keys, rotations, loops and morphs, checked against the game
date: 2026-09-23
area: render, format, test
files: tools/anim.py, tools/test_anim.py, crates/piney-data/src/anim.rs, crates/piney-data/tests/anim.rs, crates/piney-data/tests/anim_fixture.txt, docs/engine/animation.md
---

# 32. Animation playback: keys, rotations, loops and morphs, checked against the game

Until now the viewer could only show an Anime chunk's first frame
(`Scene::anime_frame0`, [[26]]). The user asked why `--anime` did nothing to
the flags and ships. Playing an animation needs the rule the game uses to get
from one key to the next. A helper agent worked it out from `ccAnm` and
checked it by running the game's own functions. I re-ran its checks on
integration. What is true now is on [the animation page](../docs/engine/animation.md).

## How it was checked

`tools/test_anim.py` runs the game's animation code in eemu. It subclasses
eemu to add what that code reaches and eemu lacked:
- the VU0 macro instructions;
- `lqc2`/`sqc2` and `ldl`/`ldr`;
- about a dozen MMI instructions (`pext*`, `pmaddh`, `paddsh`, `ppach` and
  the like).

VU0 arithmetic uses eemu's FPU model: every result is truncated, and a
multiply-add rounds its product first. Only the allocators,
`ccRingBufferTh::OpenData` and `ccDrawPacketCtrl::GetWork` are stubbed.

The harness then does three things:
- **Controllers.** It compiles a chunk with `ConvLCNum2ALCNum` and evaluates
  it with `SetAnmCtrlWork`, reading the position, matrices and float that
  function leaves in its scratch memory.
- **Playback.** It steps `_AnimateForward` at speeds 256, 100, 384 and 700,
  comparing the frame counter, the return value, every object's matrix and
  every morpher's state after each step.
- **Blending.** It runs `ccMorpher::Modify` on real models (xp_flag, ekx1)
  and on synthetic edge cases.

The unit tests pick animations for their quirks:
- ordinary loops: town01's flags and ship;
- morphing: xp_flag;
- keys sharing a frame: chgate;
- keys going back in time: eldl, cbu3body;
- keys past the last frame: xdhhack;
- texture offsets: xdl_load;
- a play-once animation with notes: ekx1.

A bulk run over 150 random animations (`bulk 150 4`) passed:

| Check | Cases | Result |
| --- | ---: | --- |
| position, scale, transparency | 11,175 | bit for bit |
| single-value rotations | 3,946 | bit for bit |
| keyed rotations | 3,582 | worst 1.53e-5 |
| several steps equal one | 1,405 | identical |
| playbacks | 43 (6,061 steps, 233 ends, 99 loop resets) | all match |
| object matrices during playback | 107,263 | all match |
| morph states | 576 | all match |

Keyed rotations are the one inexact part. The reference does the axis-angle
work in double; the game uses float `sinf`/`cosf`, a double `atan2` and
float matrix products. Its running matrix drifts a little at every key it
multiplies in: 3e-6 at the first key of `cha2body ANM_cha2mag1`, 1.3e-5
after about 70. So the tolerance is 1e-5 plus 1e-7 per key passed.

## The port

`piney_data::anim` holds the port, with the EE's float arithmetic in a
private module. The agent checked that arithmetic once against eemu on
200,000 random operand pairs, with no differences.

`tests/anim.rs` checks the port two ways:
- **Against the reference.** It reads `anim_fixture.txt` (392 lines, numbers
  only, from `test_anim.py fixture`). Everything matches exactly except
  rotations, which are within 3e-8: the reference's doubles rounded to f32.
- **Over the whole disc.** It reads every Anime chunk in DATA.BIN: 4,091, of
  which 953 loop, with 229,107 object records and 11,086 F_Morpher records.
  At frame 0 it agrees with `Scene::anime_frame0` on position and scale bit
  for bit (the EE reads denormal keys as zero). Rotation agrees within
  8.3e-4, the key quantisation plus the VU sine.

On integration: `test_anim.py` 4 tests OK, `cargo test -p piney-data --test
anim` 2 tests OK.

## What changes for the viewer

- **Rotations turn about one axis.** The game turns at constant speed about a
  single axis between key orientations, the short way round. A viewer that
  interpolated Euler angles would swing flags and doors through the wrong
  path.
- **A loop is N − 1 frames.** A looping animation shows frame N − 1, then
  resets to 0 without showing it. Frame 0 is not seen again after the first
  pass.
- **Morph weights step.** F_Morpher weights change only on whole frames, from
  frame 1's records on, and are never interpolated. This is what moves the
  flags' cloth ([[26]] skipped the morph targets as geometry).
- **Instances.** `controllers_at` keeps one pose per record, so ExtObj copies
  stay separate instances, as [[31]] found for the dungeon rooms.
- **A fourth channel.** Object records carry transparency in flag bits 9-11
  (`ccCoord::localtp`), which the old readers skipped.

**Still unknown:** how the draw turns `ccMaterial.u/v` into the ST row (so
the unit of an animated texture offset on screen); the light, camera,
ambient, F_Obj and note records, which are only counted; whether the field
changes `ccAnm::frameSpd`; that 30 frames a second in the field is right
(`frameRate` 2 after `ccSetupNewGame` is read as vblanks per frame).
