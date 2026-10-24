---
number: 437
title: Gorre's brothers are drawn: each brother's ccBoss::Main draws the clump its clip poses, its trail and its own wave, none of which the port had drawn since Gorre was ported
date: 2026-10-23
area: battle, render, test, tooling
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/gorre.rs, crates/piney-battle/src/boss/gorre/brother.rs, crates/piney-battle/examples/battle_probe/gorre.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/launcher.rs, crates/piney-game/src/mode.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/boss_fx.rs, crates/piney-game/src/session/tests/outbreak_end.rs, tools/test_battle_gorre_rs.py, docs/engine/boss-gorre.md, GAPS.md
started: 2026-10-23T22:22:06-06:00
took: 1h 8m
---

# 437. Gorre's brothers are drawn: each brother's ccBoss::Main draws the clump its clip poses, its trail and its own wave, none of which the port had drawn since Gorre was ported

Issue #73 (shugo21-bit, build a2c6920): in Outbreak's last fight, at
"Chatting Snaring Twins", Gorre could not be seen except in the attack
cutscenes. The screenshots show the HP panel, the brothers' two HP bars
and the target cursor in the air, and no bodies. The cutscene shot that
shows both brothers is a stream's.

## Not a regression

The brief suspected this week's Gorre commits. None of them touched it:

- The brothers have never been drawn in the field, from the port of
  Gorre on (d8894df, worklog 303).
- `Combat::part_actors` makes a part's actor from `BossLook::images`.
  For Kyvia that holds the core and the gomora, for Magus the leaf, and
  for Gorre nothing. `part_actors` has not changed since 7ede483, and
  no commit ever gave Gorre an `images` arm (`git log -G`).
- A probe at HEAD (1547fb5) of event 218's arena found no actor for
  either brother on any frame, while their `drawSW` was 1.

So the player saw what every build since September has drawn.

## What the game draws

`ccBoss05::Main` never calls `ccBoss::Main`, so Gorre's own body is drawn
only in the Kerse. Each brother's `Main` is `ccBoss::Main` (OUT gcmn
0x0046d490). While `drawSW` (+0x1d5) holds, it ends with `PreDrawAnm`,
`ccBoss::Draw` and `DrawAfterImages`:

- `ccBoss::Draw` (0x0046e730) draws the anm at +0xd4 with
  `ccAnm::Draw`, which draws the clip's own objects. The anm is set from
  `boss05SlaveAnmTbl1`/`2` by act:
  - the first brother's `ANM_ex51*` pose `CMP_trall`'s objects;
  - the second's `ANM_ex52*` pose `CMP_trall2`'s;
  - the Epitaph's `ANM_ex5x*` (both brothers) pose `CMP_trallx`'s.
- `EntryAfterImage` (0x0046ef20) passes the brother's anm, place (+0x40)
  and turn (+0x60) to `EntryAfterImageAnm`. Every call in boss05.cpp
  passes a null param, so the manager uses its own (0x0046c7b8): colour
  0x00505000 at 100, transparency 1 by -1/80, `maxFrame` 40, `ss` and
  `es` (0, 0, 0, 1), no scale. `ccBossAfterImageEx::Init` (0x0046c9d0)
  takes the anm at `frameNow`, or steps it once from frame 0. The calls:
  - the Wave's spin, every fourth frame;
  - the Epitaph Wave's, every sixth;
  - Gorre's dash: `m_brotherM`'s, then `m_brotherP`'s (0x004ac414,
    0x004ac42c).
- The strike's wave is the brother's own `anmw`.
  `SetMatrix_PosRotZYX(+0x40, +0x60)` puts it at the brother, then
  `DrawParts(anmw, 0)` (0x004b02d0, 0x004b02fc).

## What the port did

- No brother actor ever existed.
- The brothers' `Out::AfterImage` and `Out::DrawWave` went into Gorre's
  out list. The field drew them as Gorre's own:
  - the trail with Gorre's clip at Gorre's place, under the ground
    500 behind Kite;
  - the wave with Gorre's `anm_wave`, never stepped, and only one of
    the two.

## The fix

- piney-battle:
  - `gorre::Pic` has `Image` and `Wave`. Each carries the brother's
    scene character, its clip and `frameNow` or its wave's pose time,
    and its place and turn as the game's calls read them.
  - The spin, the Epitaph's spin and the dash send `Image`; the strike
    sends `Wave`.
  - `AfterImageParams::ENTRY` holds the manager's own params, and
    `Anm::new` is public.
- piney-world:
  - `BossLook::images` holds Gorre's three clumps (`GORRE_BROTHERS`).
  - `part_actors` takes the clump the brother's clip poses
    (`clip_body`). That is the one whose nodes hold the clip's first
    track's object. It makes the actor again when the body changes,
    for the Epitaph.
  - `BossRun::waves` is a list, so both waves are drawn.
  - A brother's `Image` becomes an `AfterImage::anm` of `x51` in
    `ENTRY`'s params.

The field now shows both brothers through the fight. In the shots they
are tall masks with scaly backs, and their trails are teal (0x505000).

## Checks

- **Against the game's code.** `tools/test_battle_gorre_rs.py` now
  records each `EntryAfterImage` and `DrawParts`: the brother, the anm's
  `frameNow`, the place and the turn. `battle_probe` gives the same from
  `gorre::Pic`. Results, every frame agreeing:
  - `bulk 100` from 9900: 303,376 frames;
  - from 20000: 331,118 frames;
  - from 40000: 316,682 frames;
  - in all three, 7,085 trails and 29,780 waves a brother;
  - 30 cases on Quarantine.
- **`gorres_brothers_are_drawn_through_the_fight`** (`boss_fx`) plays
  6,000 frames of event 218's arena:
  - from Gorre's first `Main`, each brother's actor is drawn exactly on
    the frames its `drawSW` holds (5,635 frames);
  - every field frame on the boss camera, with the brothers above
    `ccObj::Draw`'s 1/128, draws `x51` models of both (each its own
    `ex51` or `ex52`, or two `ex5x`);
  - both waves are drawn apart (176 frames, never one alone), and the
    trails are seen (580 frames).

  At 1547fb5 it fails on the first frame.
- **Shots** (`gorre_brothers_shots`): before in
  /mnt/data/claude/scratch/a437/before, after in
  /mnt/data/claude/scratch/a437/after.

## The player's pad log

`launcher::replay_pad_log` is the old `replay_through_the_launcher` loop
as a helper. It changes three things:

- It starts the launcher on the row the log's own titles show. The
  player's launcher remembered Outbreak; the replay had started on
  Infection.
- It runs the log's console lines before the frames they came before.
  The old loop dropped them. This log has `console god` at line 3096,
  Without it the run parted at line 5985. There, in area 2 field 71, the
  player's Kite (in god mode) stopped in act 7, and the replay's (not in
  god mode) ran on.
- It hands each frame's mode, events and engine to the caller.

`session::tests::outbreak_end::issue_73_replay` uses it. A log line's
title shows the state two steps before its pad, as `main` writes it, and
the replay agrees with every title up to the fight.

The replay reaches Gorre's arena and plays the player's fight:

- 28,592 fight frames, both brothers drawn in 27,863. The rest are the
  Kerse's and Tornade's frames, where `drawSW` is 0 or the fade is 0.
- Then the ending, where the desktop's sequence starts under the staff
  roll. That is the issue's second report, entry 438.

**Still unknown:** nothing.
