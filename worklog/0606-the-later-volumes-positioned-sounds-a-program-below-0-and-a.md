---
number: 606
title: The later volumes' positioned sounds: a program below 0 and a step on a ground with no row are silent from Mutation on, and test_sound3d's probe pinned to Infection's tables
date: 2026-11-16
area: audio, volumes, test
files: crates/piney-audio/src/se3d.rs, crates/piney-audio/src/driver.rs, crates/piney-audio/examples/se3d_probe.rs, crates/piney-game/src/area.rs, crates/piney-fieldui/examples/chat_probe.rs, crates/piney-fieldui/examples/fieldui_probe.rs, tools/test_sound3d_rs.py, tools/test_chat_msg_rs.py, docs/engine/sound.md, BUGS.md
resolves: 605
started: 2026-11-16T00:54:56-07:00
took: 55m
---

# 606. The later volumes' positioned sounds: a program below 0 and a step on a ground with no row are silent from Mutation on, and test_sound3d's probe pinned to Infection's tables

`tools/test_sound3d_rs.py` failed on Outbreak (7 failures, 2 errors of
10 tests), and on Mutation and Quarantine as well; Infection passed. Two
causes in the harness hid two real port bugs, both from Mutation on.

## The harness: Infection's tables on every disc

The harness runs the disc `PINEY_VOLUME` names, but:

- `se3d_probe` read `sound::tables_of(Volume::Inf)` and played everything
  through a `Driver::new()`, whose volume is Infection. So the port side
  used Infection's `seData` (237 rows; 259 from Mutation on), Infection's
  setbl.cpp and Infection's code paths. The probe now takes the disc's
  `volumeNum` as its argument, as `gamectrl_probe` does since 605, and
  passes the volume to `spc_note`, `pc_note`, `enemy_note` and
  `inu_note`.
- The harness's own case tables were Infection's: `N_SE = 237`,
  `portdata.setbl("INF")` for the params, 19 character ids and 19
  categories. It now takes the row count and the tables from the disc's
  build data and tries every id and category of the disc's tables,
  padding included (21 + 3 ids on Outbreak and Quarantine).
- `GROUNDS`, the hit attributes `seHitAttr` tests, had eight of its values
  wrapped in `inf_va()`, as if they were addresses (0x00304050 and the
  like are ground codes compared with `hitAttribute & 0x00f0f0f0`). On
  Infection `inf_va` is the identity, so it did no harm there; on the
  later discs those eight grounds became other numbers. They are plain
  constants again. `seHitAttr` is the same function on all four discs (the
  same compares; Outbreak's and Quarantine's add nops after branches).
- The two errors (`read ... outside RAM` in `ccSeSetParamSPC` and
  `ccSeSetParamEnemy`, on Outbreak and Quarantine only) came from params
  picked in Infection's tables, which Mutation shares and the other two
  do not; with the disc's tables they are gone.

Not a regression: the harness has followed `PINEY_VOLUME` since the first
commit (3fa1e9e), with the probe pinned from the start. This answers 605's
open question: both, the probe's tables and the harness's own.

## The port: what Mutation's code adds

With the probe on the right disc, three checks still differed on
Mutation, Outbreak and Quarantine (on, note, loop) and three more (spc,
pc, inu). Both are tests Infection's code lacks.

**A program below 0 is no sound.** From Mutation on `ccSeOn3D`,
`ccSeOn3DNote`, `ccSeOn3DLoop` and `ccSeOffLoop` each load the row's
`progNo` first and return when it is negative, before `calcVel`:

| | ccSeOn3D | ccSeOn3DNote | ccSeOn3DLoop | ccSeOffLoop |
| --- | --- | --- | --- | --- |
| MUT main | 0x0017c4cc | 0x0017c6a4 | 0x0017c890 | 0x0017cd64 |
| OUT main | 0x0017c15c | 0x0017c344 | 0x0017c540 | 0x0017ca34 |
| QUA main | 0x0017c0bc | 0x0017c2a4 | 0x0017c4a0 | 0x0017c994 |

So nothing is sent, `testVelocity` (+0x148) is not written,
`ccSeOn3DLoop` returns -1, and `ccSeOffLoop` sends no note off and leaves
`loopID[id]` as it was. Rows 233 and 234 are the ones with a negative
program there. Infection has no test and sends them as program
`progNo & 0x7f` (0x77). The port had this for `ccSeOn`/`ccSeOnNote`
(MUT 0x0017c310, 0x0017c3f4) but not for the 3D forms. It is now one
function, `se3d::sounds(volume, se)`, which `Driver::se_note` and the four
3D senders ask.

**A step on a ground with no row is silent.** `seHitAttr` gives row 202
for a ground it does not know. Infection's `ccSeOnPCStep`,
`ccSeSetParamPC` and `ccSeSetParamInu` then play row 202 at note 67. From
Mutation on each one returns there (`li v1, 202; beq a0, v1, <return>`):
`ccSeOnPCStep` MUT 0x0017ce14, OUT 0x0017caf4, QUA 0x0017ca54;
`ccSeSetParamPC` 0x0017d234, 0x0017cf84, 0x0017cee4; `ccSeSetParamInu`
0x0017d3c8, 0x0017d138, 0x0017d098. The notes by who steps are the same
as Infection's. `se3d::step` now takes the volume and answers `None`
there, and `pc_note` takes the volume as the others do (`area.rs` passes
the world's).

`tobjSeLoopStart` and `tobjSeLoop` play the fixed row 44 (program 31), so
there is nothing to test there; the tobj check passes on all four.

## Results

`test_sound3d_rs`: 0 mismatches on all four discs, 1,500 cases a check
and in bulk 50,000 cases of each of the ten checks (500,000 a disc). The
new unit tests `later_volumes_skip_programs_below_0` and
`later_volumes_step_silently_on_a_ground_with_no_row` pin both.

The golden shots: 4,550 files, 3 differ, all `.frames` digests of
`dungeon_shots` area 101 on Mutation, Outbreak and Quarantine. The
pictures are the same (every picture digest equal); on 17 frames of
each the events digest changes and the count goes from 3 to 2: a party
member's footstep (`ccSeSetParamSPC`, param 0) on a ground with no row,
the only event this change can drop in a dungeon, which those discs do
not play. Infection's run is unchanged.

A Quarantine player's pad log (bug 78's, 110,325 frames from power-on)
replayed through the launcher (`replay_through_the_launcher`, every
frame's events handled) prints the same 11,320 lines on main and here.
The battle pins: 291 calls, 0 differ. The piney-game suite 348 passed, 0
failed, 153 ignored, as on main; the crates' tests, `gen --check`,
clippy and fmt clean.

## Other probes pinned to Infection

Searched `crates/*/examples` for `Volume::Inf`, for the defaults that are
Infection's (`Driver::new()`, `Config::default()`, `record::State`), and
for each `PINEY_VOLUME` harness for how it starts its probe (the disc's
ISO, its `volumeNum`, or nothing):

- `chat_probe` (`test_chat_msg_rs`): `Fonts::of(Volume::Inf)`; now the
  disc's, given its `volumeNum`. The two tests passed before and after
  on all four discs.
- `fieldui_probe` (`run` request, the centred lines' widths):
  `Fonts::of(Volume::Inf)` while the rest of the probe used the disc's
  volume; now `volume`. The six fieldui harnesses give the same results
  before and after on all four discs, log for log.
- The other uses are checks of the volume, not pins (`ride_probe`,
  `party_ai`, `kite`, `spell_probe`, `world_probe`), `dump_setbl`'s
  default with no argument, and `demo_probe`'s `Config::default()`, which
  `Demo::new` overwrites with the disc's volume. The probes started with
  no argument (`dust_probe`, `breath_probe`, `weapon_probe`,
  `morph_probe`, `shadow_probe`, `shadow_volume_probe`) name no volume.

The fieldui harnesses fail on the later discs on main and here alike
(`test_fieldui_rs` 8 on Outbreak and 9 on Quarantine, `shop` 1 and 6,
`personal` 4 on each of Mutation, Outbreak and Quarantine, `option` 2 on
each): a status value one off ("  -3" for "  -2"), a condition icon's y,
the message cells of the option pages. None is a pinned volume; they are
the field UI's own differences on those discs and are not taken up here.

**Still unknown:** nothing.
