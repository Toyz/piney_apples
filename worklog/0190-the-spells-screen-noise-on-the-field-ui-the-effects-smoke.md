---
number: 190
title: "The spells' screen noise on the field UI; the effects' smoke callers all accounted for"
date: 2026-09-26
area: battle
files: crates/piney-game/src/fx.rs, crates/piney-game/src/area.rs, crates/piney-world/src/field_world.rs, crates/piney-fieldui/src/lib.rs, crates/piney-game/src/session.rs, docs/engine/battle.md, docs/engine/effects.md
---

# 190. The spells' screen noise on the field UI; the effects' smoke callers all accounted for

A sweep of the docs for "not ported" found a line in `battle.md`'s "What
the runtime still lacks": "The effects' camera shakes, noise and flashes
are not carried out yet." The shakes and flashes have been carried out
for some time: `area.rs` takes them from the effects each frame. The
noise was not. `fx.rs` dropped piney-effect's `Event::Noise` with a
comment saying so.

**The noise.** A blast in the camera's reach (`checkCameraShakeRange`:
within 67.5 degrees of the view and 2000 of the eye) calls
`ccMenu->noiz->SetNoizBs(20)` as well as `cameraShake`. The callers:

- the tornado spells at count 40;
- the convergence, upheaval and fall systems;
- the summons.

On screen the last frame is laid over this one 2.8% larger at alpha
0x50, fading (field-ui.md, the noise's +4). The field UI already had
`ccNoiz` for `interNoiz` and the event instructions.

Now:

- `FieldFx::take_noises` hands the frame's requests over.
- `GameFx` keeps `Noise` events for it, as it keeps the shakes and flashes.
- `AreaMode` gives each to the new `FieldUi::noise_bs`, which is
  `SetNoizBs`.

`a_spell_lands_through_the_effects` used to cast Tornado the moment the
goblin came out, some 2,500 away, where the blast is out of the camera's
reach. It now walks Kite to within 1,200 of the goblin first. The noise's
count then reads 19: the 20 set, less the one frame drawn before the read.
The tornado's level and blows are checked as before.

**The smoke.** `effects.md` listed `WORLD::DrawSteam` and `ccGimSymbol`
as callers of `effSmoke` that were not ported. Both are ported, in
`field_ambient` and `gimmick.rs`'s `obj_main`. Every caller in Infection
was listed with the disassembler's cross references. Those not ported
are the later volumes' bosses:

- `ccBoss04` and `ccBoss06`'s `OnThinkBackDash`;
- `ccBoss08_b`;
- `ccBossEffLightBallShock` and `ccBossEffRockTower`.

`effVirusCrystal` is ported for the virus boxes (`ccGimBox::virusMain`).
Only `ccRtownPC::eventMode`'s call is left, in the mode that volume 2's
event 114 gives.

**Still unknown:** Nothing new. The noise's look was not compared with
the game's picture.
