---
number: 218
title: Carmina Gade and its airship
date: 2026-09-27
area: volumes
files: crates/piney-world/src/town03.rs, crates/piney-world/src/town.rs, crates/piney-effect/src/dust.rs, crates/piney-game/src/world.rs, crates/piney-game/src/start.rs, crates/piney-game/src/session/tests/survey.rs
---

# 218. Carmina Gade and its airship

Mutation's M215 opens in Carmina Gade, the third Root Town (`game.town`
2), and players go there from M203 on. The port had only Mac Anu and Dun
Loireag. Carmina Gade is now `town03.rs`: `ROOTTOWN03` as Mutation's
executable has it.

Infection's executable also has a `ROOTTOWN03`, but it is an earlier build
that no disc reaches. Mutation's is larger in every function: the
constructor grows from 3660 to 4184 bytes, `Draw` shrinks from 796 to 592,
and `DrawMap` grows from 4016 to 5184. The port follows Mutation's.

## The town

- **The constructor.**
  - The file is `town03`, or `town03d` in crisis.
  - It makes 34 `STATICMODEL`s from `RT_MODELTABLE03` and no static
    objects, so a class's object table is now optional.
  - The lights come from `ANM_sr3bac1a` by `town03Light`: `LGT_sr3lig1` and
    11 omnis.
  - The fog is `SetFog(3000, 15000, 0, 85)`, and the fog and the
    background share the colour 0x0014140c.
  - Four background clumps: two skies and two mountain ranges.
  - Three waters, of which only the first is ever drawn.
- **`Draw`.**
  - Water 0 and its copy, on objLayer.
  - The airship.
  - `DrawBG`: the four clumps on `bgLayer[0]`-`[3]`, and in crisis three
    more on `bgLayer[4]`-`[6]`, with the scrolling materials.
  - On obj2Layer (not objLayer, as in Dun Loireag): the rows by pass, then
    rows 29-33 and the type-0 rows below 29, both with
    `STATICMODEL::DrawWithOutFog`.
- **The layers.** `WORLD_MAN::GO`'s `bgLayer` priorities, read from
  Mutation's code: -100, -90, -80, -70, -60, -50, -45, -40, -35 and -30.
- **Left out.**
  - The BLT chunks (`ccBltGrpChunk`) only load textures into the GS.
  - `u$1875` is stepped but nothing reads it.

## The airship

`AIRSHIP` (0x0043af10; `Move` 0x0043b0c0) flies between `DMY_marker69` and
`72` and back, in six legs:

- A leg takes 2000 frames: its `t` grows by 0.0005 in single floats, which
  stays just below 1 at the 2000th step.
- The ship then turns for 300 frames to the next heading, by pi/300 or
  pi/600 a frame, and waits 300 frames.
- Its model is set 500 below its place (`SetMatrix_PosRotXYZ`), before it
  moves that frame.
- It puffs from two chimneys every 6 to 13 frames (`fieldrand`) while it
  flies and while it waits.
- While it waits, each pair of puffs sounds SE 257 (`ccSeOn3D`).

The puffs are `effSmokeN`, which is `effSmoke` with the particle's `distSW`
cleared (`dust::eff_smoke_n`). A town's draws now queue their effects and
sounds (`TownEvent`), and the session drains them after the draw
(`World::take_town_events`).

## Checks

- `town03`'s tests:
  - the constructor's pieces;
  - the draw order;
  - the first leg ending between frames 2000 and 2010 at marker 70, with
    the puffs made.
- `mutation_story_starts_open_their_event` passes: M215 opens in Carmina
  Gade with the party empty.
- piney-game: 126 passed, 0 failed, 60 ignored. The town02 and particle
  harnesses pass against the game.
- `mutation_story_survey` (4000 frames each):
  - no panic, and no host call left at its default;
  - M215 plays its opening in Carmina Gade;
  - M216 plays the ending stream (35);
  - the desktop and board starts idle, as the autopilot drives only The
    World. M208 waits on its `info_now` windows.

**Still unknown:** Carmina Gade's map. `ROOTTOWN03::DrawMap` draws four
sprites on layers of its own, and Mutation changed Mac Anu's and Dun
Loireag's maps in the same way. The port's town map is still Infection's
three sprites. The town has not yet been compared with the game in eemu.
The desktop and board have no autopilot, so the survey does not play
Mutation's desktop-led events through. `voldiff ported --volume MUT`
counts 296 changed and 11 missing functions among those the port covers.
