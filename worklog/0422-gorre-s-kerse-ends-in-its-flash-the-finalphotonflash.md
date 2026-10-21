---
number: 422
title: Gorre's Kerse ends in its flash: the FinalPhotonFlash shrinking onto the target, then its burst, ring, smoke and dust, checked against the game's code on every volume
date: 2026-10-21
area: battle, render, test
files: crates/piney-effect/src/gorre.rs, crates/piney-effect/src/boss.rs, crates/piney-game/src/fx.rs, tools/test_boss_effect_rs.py, crates/piney-game/src/session/tests/boss_fx.rs, docs/engine/boss-gorre.md
started: 2026-10-21T13:09:06-06:00
took: 11m
---

# 422. Gorre's Kerse ends in its flash: the FinalPhotonFlash shrinking onto the target, then its burst, ring, smoke and dust, checked against the game's code on every volume

Worklog 303 ported Gorre's Kerse and measured its `FinalPhotonFlash`'s
life (72 `Draw`s) for the rules, but `boss_make` made no picture: the
Kerse's last step ended with nothing at the target, no flash closing on
it, no burst, no ring, no smoke, no sounds.

## What the game draws

Outbreak's bosseff.cpp (docs/engine/boss-gorre.md, "The flash"):

- `ccBossEffFinalPhotonFlashCreate(pos)` (0x0048a6e0): a 0x50-byte effect
  holding particle's `EFF_x000` (`ccEff::Init(chunk, 1)`) with
  `CLT_x000c5` as its palette (`ccEff` +0x3c), the place, scale 1000.
- `Draw` (0x00480b70), shrinking:
  - SE 159 (note 70) the first time;
  - the sprite at both scales, colour 0x80e0ffff, `SetRenderState(0, 0)`
    (no depth test), on the manager's layer;
  - the scale less 100; below 0, `ccBossEffAutoSamonRingCreate(pos,
    (0, 0, 0, 1), (0, 2/3, 10, ?), 176)`, `Explode`, and the burst.
  - The param's fourth word is a stack slot `Draw` never writes. The
    ring's `Create` (0x0048a580) reads only the first three, so it does
    not matter.
- `Explode` (0x00480d00): SEs 35 and 40 (note 55) once; five
  `BurstSmokeGenerator`s at the place (row 2 with ff 2 100 up; rows 2 and
  3 with ff 2 170 up; row 4 with ff 3 and 2 at z 0; row 7 with ff 6, 7
  and 8 at z 90); `ccEnemyEffDust(pos, 10, 5)`.
- After the burst, 60 more `Draw`s and the slot frees: 11 + 61 = 72.

## In the port

- piney-effect `gorre::PhotonFlash` (`new`, `draw`, `explode`), a
  `BossEff::PhotonFlash` made by `Make::FinalPhotonFlash { pos }`. Its
  `Draw` hands back the ring, which the manager makes after it in the
  same pass (as a storm's square and a lightning's wave).
- piney-game's `boss_make` makes it for `EffKind::FinalPhotonFlash`; it
  runs alone, as the dead effect does, alongside the rules' count of its
  life.

## Checks

- `tools/test_boss_effect_rs.py test_gorre_flash` (all four volumes:
  each gcmn has the effect, Gorre fights from Outbreak on): the game's `Create` and the manager's passes natively
  against boss_probe's `photon`, four places, 73 passes each (72 `Draw`s
  and the pass that frees the slot). Compared every pass: the sprite
  (place, scales, colour, palette), the ring's clump, matrix and palette,
  the sounds, the generators (rows, force fields, place), `rand()` and
  `ccRand`. They agree on every volume. Outbreak's other boss-effect tests still
  agree.
- `session::tests::boss_fx::gorres_kerse_shows_its_flash` (event 218's
  arena, 12,000 frames): three Kerses, the flash drawn 33 frames and its
  ring 60.
- piney-effect tests, piney-game's suite (four threads), clippy, fmt.

Shots (`gorre_kerse_shots`, /mnt/data/claude/scratch/a422):

- `gorre-02902.png`: the flash, still near its full scale, over the
  arena.
- `gorre-02916.png`: the burst's smoke and dust on Kite.

**Still unknown:** the brother's shield's shape (worklog 303) is another part of the fight and
stays open. `ccBossBlur` stays undrawn for every boss: its `m_exit` is
never set and lives in memory `_ccMalloc` does not clear, so whether a
fight blurs depends on what the heap held (worklog 117).
