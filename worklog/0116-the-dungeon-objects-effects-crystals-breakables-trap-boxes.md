---
number: 116
title: "The dungeon objects' effects: crystals, breakables, trap boxes and the Statue of God's glow"
date: 2026-09-25
area: battle, render, test
files: crates/piney-effect/src/gimmick.rs, crates/piney-effect/src/lib.rs, crates/piney-effect/src/effect.rs, crates/piney-effect/examples/spell_probe.rs, crates/piney-game/src/fx.rs, tools/test_effect_skill_rs.py, tools/test_effect_spell_rs.py, docs/engine/effects.md, docs/engine/battle.md
---

# 116. The dungeon objects' effects: crystals, breakables, trap boxes and the Statue of God's glow

The battle's gimmicks already raise these effects: `ccGimBox`'s break,
virus core and trap, and `ccGimIdol`'s glow (`entry::Out::Crush`,
`VirusCrystal`, `OpenTrapBox`, `StatueOfGod`). Nothing drew them, so a
smashed barrel just vanished and a Statue of God had no glow.

## Ported (`piney_effect::gimmick`)

- **`effVirusCrystal`.** Its controller -4 and generator 109. The
  controller's case in `ccEffect::Main` restarts the generator at count 13
  and 20. The 20 is not written in the case: it is the register the
  chain's compares left holding it.
- **The fragments.** `effWoodFragment`, `effEggshellFragment`,
  `effPotFragment` and `effBoneFragment` are one function with different
  ids (25, 138, 142, 146, plus 0-2 at random). Each piece tumbles and
  bounces as the ported debris do. With x not 0 its clump is duplicated
  and CLT_x036 swapped for CLT_x036c1 (`WoodClt1` and `WoodClt2`, read from
  effect row 25's file).
- **The crushes.** `effCrushBarrel`, `effCrushEgg`, `effCrushPot` and
  `effCrushCorpse` throw 15 pieces and start generator 110.
- **`effOpenTrapBox`.** Its controller -5 throws a `ccParticleExplode`
  every frame of its five. A wooden box or barrel also splinters with the
  swapped palette. It plays generator 110 plus the trap, and sound 39: the
  second 39, after `invokeTrap`'s own.
- **`effStatueOfGod`.** Generator 125 with its `syncSW` on the idol's
  `effsw` (+0x1e0). The port reads that through `Host::char_int`, which
  the field's host answers from the idol's entry object.

`fx.rs` starts each from the entry control's shows.

## Checked

- **`tools/test_effect_skill_rs.py`'s new `gimmick` group.** It runs the
  game's starters in eemu on the spell harness's machine against
  `spell_probe`:
  - its sweep: the crystal, every crush with x 0 and 1, the trap box for
    both kinds and traps 0, 3 and 4, and the statue;
  - 30 random cases.

  The statue's switch is set on at the start and off some frames later.
  33 cases, 3476 frames, 17541 draws and every fragment id match bit for
  bit: slots, draws, generators, events, both random generators. The
  recorder now names a pointer into a character (`["at", cid, off]`),
  which the statue's `syncSW` needed. The other five groups still pass.
- **`story_4_shots`.** It shows the statue room with the glow's sparks in
  front of the Gott statue.
- The workspace's tests, clippy and fmt pass.

**Still unknown:**
- **`effVirusCrystal` from `ccRtownPC::eventMode`**, a Root Town event's,
  is not wired: the town's PCs do not run that mode.
- **The idol's dust ring** (`ccGimIdol::main` from frame 100) is not
  drawn.
- **No shot of a barrel smashed or a trap going off** yet. The
  harness covers the effects, but no session test breaks a barrel.
