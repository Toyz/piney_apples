---
number: 421
title: Magus's fight shows its pictures: the leaves' markers, charges, bursts, blinks and countdowns, the laser's thunder and shocks where its beams meet the ground, and the dropped leaves hidden on the body
date: 2026-10-21
area: battle, render, test, tooling
files: crates/piney-battle/src/boss/magus.rs, crates/piney-battle/src/boss/magus/leaf.rs, crates/piney-effect/src/magus.rs, crates/piney-effect/src/boss.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/pose.rs, crates/piney-game/src/fx.rs, crates/piney-game/src/area.rs, crates/piney-gen/src/manifest.rs, tools/test_battle_magus_rs.py, tools/test_boss_effect_rs.py, crates/piney-game/src/session/tests/magus_fx.rs, docs/engine/boss-magus.md
started: 2026-10-21T12:17:09-06:00
took: 51m
---

# 421. Magus's fight shows its pictures: the leaves' markers, charges, bursts, blinks and countdowns, the laser's thunder and shocks where its beams meet the ground, and the dropped leaves hidden on the body

Worklog 274 ported Magus's rules and named its pictures (`Out::Magus(Pic)`),
but nothing drew them. A player saw the leaves fall and burst with no
marker smoke, no charge, no burst, no countdown and no blink; the laser
rose with no thunder and no shocks on the ground; the body kept drawing
its dropped leaves on itself, so each dropped leaf showed twice. The
shocks' beams were taken to meet the ground (`shock_hit` true unless a
test said not).

## What the game draws

Mutation's boss03.cpp and bosseff.cpp; every site is in
docs/engine/boss-magus.md, "The pictures":

- **The generators.** Four tables, each with its force fields: the
  leaves' markers, the grow's charge, a dying leaf's smoke and the
  laser's thunder.
  - `ccLeafMark::InitAtOnce` (a leaf's first fall, `Order(24)`) makes two
    markers that follow the marker's place (+0x29420): `g_leafMarkerGen`
    row 0 with ff 0 and 1, and again with ff 1 and 2 and `gSync`, both
    texture 102. It pauses both (+5 bit 0); the landing frees them;
    `PrepareExplode`, `Order(27)` and the death pause them again.
  - `Order(29)` starts the charge at the leaf's place on the body
    (`g_energyChargeGenerator` row 1, ff 0 and 3, distSW off), killing
    the one before; `Order(27)` and `(30)` kill it.
  - `DeadEffect` starts `g_leafDeadGen` (ff 0, texture 108) at the leaf.
  - After `SetupLaserShock` each shock starts `g_thunderGenerator`
    (`ccBossEffThunderCreate`) at its object.
- **A burst** (act 21): `ccBoss03LeafExplode::Draw` (0x004a7100) draws
  xeffect's `EFF_ex31exp2` 100 over the leaf and `OBJ_ex31exp1` scaled by
  its second status, both on `charLayer`.
- **The shocks** (`PreDrawAnm`'s laser step 2, from frame 100). Each
  `Draw` (0x004a7630) runs its beam from the shock's object 2000 down,
  turned by its leaf's local matrix and Magus's heading. `ccHitCheckLM(a,
  end, 0x20000000)` writes the contact into the end. There `EFF_ex31les2`
  is drawn (scale 6, at the fade). Every sixth contact of shocks 0, 3, 6
  and 9 bursts: two `ccParticleExplode` and `effSmokeRock`.
- **The needles** (`ccBossEffNeedle::Draw` 0x004814d0): xeffect's
  `OBJ_ex71body2` per needle on the effect layer.
- **The hiding.** `PreDrawAnm` gives each dropped leaf's `EXT_ex31leaf*`
  object and its `OBJ_ex31ligh*` transparency 0 until the clip's next
  step, unless in the Epitaph or the grow. Act 8 sets the same hiding as
  its after-images' callback (0x0049e7c0).
- **A blink** (`OnThinkCountDown`) enters an after-image of the leaf's anm
  at `frameNow` (`ccBossAfterImageEx::Init` 0x004710b0, `Draw`
  0x00471320, the leaf's `ENTRYAFTERIMAGE_T` at +0x29660): colour
  0x005050ff, transparency 0.5 fading by 1/60, its scale growing from 1 to
  (1, 4, 2) over 30 draws.
- **The countdown**: the seconds left, `MakeSignedNum(2, n)` in the
  global font (type 1, colour 7, shadowed), where `ccCalcTagPos` puts
  the leaf 100 up.

Act 31 (the needles) is never reached in the game. Nothing lowers
`m_dmgCount` (+0x52700): only the constructor (100, 0x0049e388) and the
needle's own branch (0x004a0ef0) write it, and `ccBossEffNeedleCreate` has
no other caller. The port draws them all the same, from the same code.

## In the port

- piney-battle `magus`:
  - `Gen` names each start by its maker; `rows()` gives its table, row,
    force fields, distSW, `gSync` and texture.
  - `Place` (at a point or following a leaf's marker) and `Held` (which
    member keeps it).
  - `Pic::Gen` (kill, pause) where the leaves kill or pause what they
    keep.
  - `Pic::ShockBurst` with the burst's `ccRandF(pi)`, `Pic::Count`,
    `Pic::LeafAfterImage` with its `frameNow`.
  - `Magus::shock_at` (the contact, None for a miss) replaces
    `shock_hit`; `shock_pos` gives the thunder its places; `shock_beam` is
    the beam's end.
  - `GenOp` moved to `boss` for both bosses.
  - A needle's hold compared the count after its decrement; the game
    tests the count before (`hold-- <= 0`), so the needles faded a frame
    early. Found by the effect harness: the rules harness never reaches
    act 31.
- piney-effect `magus`: `start`, `gen_op`, `Needles`, `leaf_explode`,
  `laser_shock`, `shock_burst`; `magus_sync` and `magus_shocks` in
  `Effects`; `nodes::clump` draws a lone OBJ. `boss::Tables` reads the
  four new tables.
- piney-world `combat::boss`:
  - `magus_shocks` finds the shocks before Magus's frame. It poses
    `ANM_ex31atc2` one step on (from frame 0 when another clip plays, as
    `SetupLaserShock` follows its `SetAnm` and step) at the body's last
    place, which is `PreDrawAnm`'s own (`CalcCamera` runs after it).
    Then the line test on the field's hits.
  - The objects are found by the anm entry's name: the shocks' are
    ExtObj copies of `OBJ_ex31les0`-`4`, the leaves' of `OBJ_ex31leaf`.
  - `Play::hidden` (the entries a task hid after the step) and
    `dropped_objects` hide the dropped leaves on the body and, in act 8,
    on its after-images.
  - `AfterImage` carries its fade, blend and growth; `AfterImage::leaf`.
- piney-game: `fx.rs` starts the generators, shocks and bursts from the
  shows, `magus_shows` syncs the needles, bursts and markers, and
  `boss_make` makes the needles. The area's `leaf_counts` sends the
  countdown through `FieldUi::send_font` at the frame's end, after the
  faders, as the global font is sent at the flip.
- piney-gen: the effect group's `boss_thunder(_ff)`,
  `boss_energy_charge(_ff)`, `boss_leaf_marker(_ff)` and
  `boss_leaf_dead(_ff)`. Data version 34.

## Checks

- `tools/test_battle_magus_rs.py` (Mutation) now records each start in
  full, each kill of a kept generator, the markers' pause flags and place,
  the shocks' bursts with their turn, a blink's `frameNow` and the
  countdown's number. 2 tests and bulk 60 from 9000 agree.
- `tools/test_boss_effect_rs.py`'s `test_magus_effects` runs the game's
  `ccBossEffNeedleCreate`, `ccBoss03LeafExplode::Init` and
  `ccBoss03LaserShock::Init` with their `Draw`s natively against
  boss_probe's `needles`, `leafexp` and `lshock`, frame by frame: the
  needles (74 frames), two bursts (124) and shocks 0, 3 and 4 with a
  contact or none (540 each). The draws, the particles and the rocks agree.
  Every volume's boss-effect tests still agree.
- `session::tests::magus_fx::magus_shows_its_leaves_needles_and_laser`
  (event 115's fight, 9000 frames, the laser reserved and the damage
  count run out now and then):
  - markers on the leaves 4,517 frames;
  - dropped leaves hidden 3,465 frames, never shown outside the grow;
  - `OBJ_ex71body2` 108 frames, `OBJ_ex31exp1` 219;
  - shocks on the ground 686 frames, drawn 218, thunder 299.
- piney-battle, piney-effect, piney-world, piney-fieldui tests,
  piney-game's suite (four threads), clippy, fmt, `piney-gen gen
  --check`.

Shots (`magus_shots`, /mnt/data/claude/scratch/a421):

- `magus-01692-act31.png`: the dropped leaves on the ground with their
  markers' smoke, their red blinks and the countdowns.
- `magus-04938-act25.png`: the laser's shocks on the ground about Kite.
- `magus-01206-act27.png`: the leaves charged on the body in the rise.

**Still unknown:** `EntryFlash2` and `EntryFlash3` are still one flash and the grow's loop plays once (worklog 274), the fight's sound and fader, not its pictures. Act 30 and a leaf's 23 are not reached by the harness (worklog 274). The blur (ccBossBlur) is drawn for no boss (worklog 117).
