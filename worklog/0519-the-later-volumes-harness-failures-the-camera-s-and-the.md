---
number: 519
title: The later volumes' harness failures: the camera's and the rebuilt player's square root, evchar's fellow and ccAI, demo's probe as its disc
date: 2026-10-30
area: test, world
files: crates/piney-world/src/camera.rs, crates/piney-world/src/player.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/field_world.rs, crates/piney-demo/src/lib.rs, crates/piney-demo/examples/demo_probe.rs, tools/test_demo_rs.py, tools/test_evchar_rs.py, tools/vu.py
started: 2026-10-30T08:29:34-06:00
took: 28m
---

# 519. The later volumes' harness failures: the camera's and the rebuilt player's square root, evchar's fellow and ccAI, demo's probe as its disc

The three harnesses that failed on Outbreak before the Table lift (worklog
518's "Still unknown"), fixed and run on every volume. The work before an
API cut-off was not timed; the clock here covers the rest (checks,
evchar, the commits).

## evcam: the camera's square root, and the player's

`test_evcam_rs` failed 5 tests on Outbreak, each by one or two ulps in
`ecam.rot.x`. That is the pitch, `atan2f(pos.z - view.z, h)`, where `h`
comes from `sqrtf_on(volume, ...)`. Outbreak and Quarantine take `sqrt.s`
(worklog 508), but `Camera::new` set `volume: Volume::Inf`, and "its
owner sets it after" was left to each caller. The world and the field
did set it. The probes did not, and neither did any other caller.

`Player::new` had the same trap. Worse, `Player::build`, which
`party.rs`'s `build_spc` uses to rebuild Kite in a town on the
registry's reboot, never set it. So on Outbreak and Quarantine that Kite
took Infection's square root for his camera transparency and distances.
That was a port bug in play, not only in the probes.

`Camera::new` and `Player::new` now take the volume as their first
argument, and the field went: no caller can leave it at Infection.
`Player::build` takes `hits.volume` (the disc its collision was read
from). The world's and field world's later `x.volume = volume` lines are
gone. The tests use `Volume::Inf`.

## evchar: the later volumes' fellow constructor and ccAI

`test_evchar_rs` errored in its own set-up on Outbreak. The
`__ct__10ccFellow02Fi` it calls has no name from Mutation on: none of the
17 `ccFellowNN` classes are carried. The harness now finds it the
later-volume way:

- `ccSpcStart[2]` is a carried global (INF 0x00654078), whose pointer is
  the volume's `ccFellow02Start`;
- the first call there to an unnamed function, after its operator new,
  is the constructor.

The harness also read and wrote `ccAI` at Infection's offsets. From
Mutation on, that struct grows (`volume.ai_at`), so `remote_cmd`,
`g_deg`, `g_rot_sp`, `mode`, `arrival`, `detour`, the reboot's `+0xB6`
and the talk op's `+0x9A` now go through it. With both fixed, the port
matches on every volume: 12,330 frames, 311 instructions and 39
scenarios each.

## demo: the logo movie and the rest

`test_demo_rs` had 3 failures and 2 errors on Outbreak. None was in the
port's game path. Each was the probe or the harness running as
Infection on another disc:

- **LogoMain** (`OPENING.PSS` where Outbreak's code plays
  `OPENING3.PSS`). The port picks the movie by `Control::volume()`
  (`m_NowVol`), and `Opening` returns the disc's, so the game itself
  plays the right movie. But the trait had a default of 1, which the
  probe's `Script` took. The default is gone, so every control must say
  its volume, and `Script` takes the probe's `vol N`. The probe's `vol`
  starts at the disc's, and so does the harness's `VOL` (`DISC_VOL`,
  `volume.number()`); `@volumes` still runs all four.
- **The thread** ran Outbreak's ccThDemo into `PollSema`. From Mutation
  on, ccThDemo looks for a previous volume's save on both cards (OUT
  0x001713c0) after the boot check, and then sets `m_Max_CurNO` to 4
  whatever it found (OUT 0x004152e0, whose argument it overwrites). The
  port's `max_cur` is 4 on the later volumes already. The harness now
  answers `CheckPort` with 0 (nothing to read). It also needed
  `m_NowVol` set, which the hooked constructor never set: the opening
  stream (50 on Outbreak) is picked by it.
- **The question texts** compared Infection's `saveSysMsg` with the
  disc's. The probe now reads `title::of(disc)` (Outbreak's "Memory card
  ... 682KB", where Infection's has "memory card ... 685KB").
- **LoadGame's names** read 18 `ccsNameList` rows. From Mutation on
  there are 21: `spcNameList` follows the list, and the carried
  symbol's size is Infection's. The harness now counts
  `(spcNameList - ccsNameList) / 32`.
- **The icon lighting** read `title1` and VU1's labels. The later
  executables are stripped. The probe and harness now read the disc's
  `title<N>`. `tools/vu.py`'s `Microcode` finds each VU's upload in a
  stripped executable by Infection's bytes (the same, byte for byte, on
  all three later discs) and takes Infection's labels, which are micro
  addresses.

## Checks

- On all four volumes: `test_demo_rs` (19 tests), `test_evcam_rs` and
  `test_evchar_rs` pass.
- On Infection and Outbreak, for the probes whose constructors changed:
  `test_merchcam_rs`, `test_bosscam_rs`, `test_party_rs` and
  `test_world_rs` pass.
- The tests of piney-world, piney-effect and piney-demo, and
  piney-game's suite, release.
- clippy with all features, fmt.

**Still unknown:** nothing
