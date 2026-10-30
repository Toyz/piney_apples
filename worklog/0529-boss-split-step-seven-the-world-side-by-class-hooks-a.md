---
number: 529
title: Boss split, step seven: the world side by class hooks, a BossDef row a boss, one Picture list, the shared wave and after-image outs
date: 2026-10-30
area: battle
files: crates/piney-battle/src/boss.rs, crates/piney-battle/src/boss/, crates/piney-battle/examples/battle_probe/, crates/piney-effect/src/macha.rs, crates/piney-effect/src/tarvos.rs, crates/piney-world/src/combat/boss.rs, crates/piney-world/src/combat/corbenik.rs, crates/piney-world/src/field/world.rs, crates/piney-game/src/session/tests/boss_fx.rs, docs/engine/boss.md, docs/engine/boss-gorre.md, docs/engine/boss-innis.md, plans/rust-cleanup.md
started: 2026-10-30T12:20:24-06:00
took: 48m
---

# 529. Boss split, step seven: the world side by class hooks, a BossDef row a boss, one Picture list, the shared wave and after-image outs

The seventh step of section 2's boss split (`plans/rust-cleanup.md`, the
world-side item; 523-528 before it). The field matched on the boss's code
or class in four files: the look's files and images, the constructor, the
camera, the hit's radius, which body a clip poses, the parts, the wave's
time, Kyvia's arm, Innis's images and shards, Gorre's body, Magus's hidden
leaves, Corbenik's part matrices and barrier clip. Now a boss is one row
of `BOSSES` and one `Shape` impl. The 288 pinned probe answers are byte
for byte the same, and so are twelve boss shot runs.

## The battle side: `Shape`

`piney_battle::boss::Shape` is what the field reads of a class after its
frame, each with a default:

- `wave`: `m_anmw` as `DrawWave` poses it (Skeith, Innis, Magus,
  Fidchell, Macha, Tarvos);
- `parts`: Kyvia's core then gomoras, Magus's leaves, Gorre's brothers,
  each Corbenik form's seeds, roots, or eyes and killer eyes;
- `images`: Innis's slaves as `Figure`s (character, `ccBoss`, image);
- `shards`: Innis's mirrors' shards drawn this frame;
- `hidden`: Magus's dropped leaves outside the Epitaph;
- `drawn_at`: where the body is drawn; Gorre's only in the Kerse's step
  2 (`gorre::body` is gone);
- `matrix`, `draws_again`: a Corbenik seed's, root's and eye's
  `PreDrawAnm`, moved out of the field.

`Class::shape` and `Class::with_rules` share one arm list (`each_class!`).

## The field: `BossDef` and `Picture`

- **`BossDef`** (`combat/boss.rs`): code, file, body clump, the image
  clumps, Innis's shards, `BodyPick` (an Epitaph's clips, or by clip for
  Corbenik), `PartBody` (in order, first, by clip), `CamDef`, the hit's
  radius, the constructor. `BossLook::load` and `start_boss` read the
  row; a code the port lacks still starts nothing.
  - The images are read from the body's own file. Every boss's image
    file was its body's file, so each read it twice before.
  - `Fight::file` is a `const fn` for the table.
- **`Picture`**: Corbenik's pictures (`Shown` before), Kyvia's arm,
  Macha's light ball and Innis's shards, in one list `draw_boss` draws in
  one loop. `BossRun::{ball, arm, shards, corbenik}` are gone.
  - Only one boss is up at a time and the four draw on their own layers
    or with nothing else of their boss, so the draw order is the same.

## The outs folded

- **`Out::Wave { who, time, pos, dirc, scale }`** for Macha's, Tarvos's,
  a Gorre brother's and Corbenik's waves (four `Pic::Wave`s). The base's
  `DrawWave` stays: its time is read after the frame.
- **`Out::AfterImageAnm { who, clip, frame, pos, dirc, params }`** for
  Kyvia's arm, a Gorre brother's trail and a Corbenik seed's. The clip
  and the params come with the out, so the field's three copies are one.
- **`Out::DrawAnm { clip, time, pos, dirc, layer }`** for Kyvia's arm
  (`DrawArm`) and Macha's ball (`Pic::Ball`).
- `corbenik::Pic::BarrierHit` carries its clip, so the field no longer
  reads the second form's members.
- `gorre::Pic` is gone. The probe prints the shared outs as it printed
  the old ones.
- The effects' own `Pic`s (smoke, radiate, bursts, shields) are left for
  the particle `GenSpec` item.

## Also

- Macha's `free_cam` is `Cx::free_cam_from`.
- Corbenik's second and third forms' identical `out_from` is one.
- `start_boss`'s stale `too_many_arguments` allow is gone.

## Found, not changed

Kyvia's core emits `DrawWave` from inside Kyvia's frame. The field draws
it at Kyvia's place at the class's wave time, which Kyvia's class does
not have (time 0), not at the core with its own `anmw`. Behaviour is kept
here. The fix is an `Out::Wave` with the core as `who`, checked against
the game's draw.

The diff is about 870 lines added and 700 removed.

## Checks

- The pin: 288 probe calls replayed, every answer byte for byte the same.
- Shots before and after, every PNG byte for byte the same: `innis_shots`,
  `gorre_brothers_shots`, `gorre_kerse_shots`,
  `gorre_brothers_model_shots`, `kyvia_attack_shots`, `magus_shots`,
  `fidchell_shots`, `kyvia_third_shots`, `macha_shots`,
  `tarvos_under_god`, `corbenik_under_god`.
- The tests of piney-battle, piney-world, piney-effect and piney-event,
  and piney-game's full suite, release.
- The boss harnesses against the game on their volumes (the eleven
  `test_battle_*_rs`), `test_boss_effect_rs` on all four volumes and
  `test_bosscam_rs` on Infection, Mutation and Quarantine.
- clippy with all features, fmt, `cairns check`, `tools/docs.py check`.
- `too_many_arguments`: 152 before, 151 after.

**Still unknown:** whether the game draws Kyvia's core's wave at the core
with its own `anmw` (the code says so; no harness or shot covers it).
