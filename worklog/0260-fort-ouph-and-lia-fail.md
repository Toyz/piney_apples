---
number: 260
title: Fort Ouph and Lia Fail
date: 2026-09-27
area: volumes, world
files: crates/piney-world/src/town04.rs, crates/piney-world/src/town05.rs, crates/piney-world/src/town.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/map/town.rs, crates/piney-game/src/session.rs, crates/piney-game/src/world.rs, crates/piney-audio/tests/render.rs
---

# 260. Fort Ouph and Lia Fail

All five Root Towns are now ported. The two added are Fort Ouph (`game.town`
3, server Sigma, `town04.rs`) and Lia Fail (4, Omega, `town05.rs`). The
owner wanted every town playable, so they came in ahead of the Outbreak and
Quarantine stories. The console's `town N` reaches all five from any disc.

Both classes are Outbreak's: `ROOTTOWN04` and `ROOTTOWN05` in OUT gcmn, the
same size as Quarantine's. Infection's executable has earlier builds of
both that no disc reaches, as it does for `ROOTTOWN03` ([[218]]). Outbreak's
carried names cover only the constructors, `DrawBG` and `DrawMap`. The
other methods were found through the vtables:

| class | vtable | `Draw` | `DrawBG` | `DrawObj` | `DrawObj2` | `DrawFloor` |
| --- | --- | --- | --- | --- | --- | --- |
| `ROOTTOWN04` | 0x003842d0 | 0x0043c430 | 0x0043aa40 | 0x0043ae50 | 0x0043af40 | 0x0043b030 |
| `ROOTTOWN05` | 0x00384300 | 0x0043e8e0 | 0x0043d200 | 0x0043d370 | 0x0043d410 | 0x0043d4b0 |

## Fort Ouph

Fort Ouph is Dun Loireag's pattern without the water.

- **The constructor** (0x00439c50):
  - `town04`, or `town04d` in crisis.
  - Fog `SetFog(1500, 10000, 0, 30, 0x00c8f0fb)`, and the background the
    same pale warm colour.
  - 16 `STATICMODEL`s and one `STATICOBJECT`.
  - The sky `CMP_sr4bac1`, the sun `CMP_sr4sun1`, and three cloud clumps.
  - In crisis, `CMP_sr4dat1_1`-`3`.
  - Six lights (`LGT_sr4lig1`, `LGT_sr4omn1`-`5`).
  - `town_z`'s 25 `CLOUD`s and `LENSFLARE`.
- **`DrawBG`.** Two scrolls step each frame, by 0.01 and 0.002.
  - The cloud materials take the first in U. Dun Loireag's take V.
  - The sky goes on `bgLayer[0]`, the sun at `DMY_sr4lig1point` on `[1]`,
    and the crisis clumps on `[2]`-`[4]`.
  - The cloud clumps go on `bgLayer[5]` (-50). Dun Loireag puts them on
    effLayer. The `WORLD_MAN` offsets +0x494 .. +0x4a8 are `bgLayer[0]` ..
    `[5]`, confirmed against Infection's `ROOTTOWN02::DrawBG`.
- **`Draw`.**
  - `DrawBG`, then `DrawObj2`, `DrawObj`, `DrawFloor` and `DrawMap`.
  - The lens flare, then the clouds.
  - No type-0 rows.

## Lia Fail

- **The constructor** (0x0043c5e0):
  - `town05` only; there is no crisis file.
  - Fog `(1500, 10000, 0, 75, 0x00f0c080)`, Dun Loireag's colour.
  - 7 `STATICMODEL`s and no objects.
  - The sky `CMP_sr5bac1`.
  - 20 lights: `LGT_fdirect01` and `LGT_omni01`-`19`.
  - Six `ccEff`s of the lens flare, which nothing draws.
  - 19 `ccEff`s of `EFF_se1_1ef1` at `DMY_omnpoint01`-`19`, each with a
    pattern, a wait and a flicker count from `fieldrand`.
- **`DrawBG`** steps three scrolls that nothing reads, and draws the sky on
  `bgLayer[0]`.
- **`Draw`.**
  - `DrawBG`, the three passes, `DrawMap`, then the type-0 rows.
  - The 19 glows on effLayer.
  - A waiting glow draws its pattern and steps it, back to 0 at `patNum`.
  - When its wait runs out, it flickers: a random pattern each frame for
    its count, then new wait and flicker counts.

## What else they needed

- **Minimaps.** `TownMap` takes `RT04ICONPOS`/`RT05ICONPOS` and each
  constructor's `stepw`/`steph`: 0.8 for Fort Ouph, 2.0 for Lia Fail. They
  draw on the later volumes' own layers. Carmina Gade's map now opens on
  Infection's disc too.
- **Music.** Every town's bank loads and sounds on Infection's disc
  (`every_town_has_music`).
- **Already per town.** The start places (`SetCharPosition`), the static
  tables and the NPCs needed nothing. Lia Fail's walking PCs (the Cossack
  Leader by the gate) come up from the tables as they are.

## Checks

The two towns' tests pass:
- `town04::tests::constructor` and `first_draw`: 16 rows, the object, six
  lights, the crisis clumps, and the `DrawBG` order;
- `town05::tests::constructor` and `a_frame`: 7 rows, 20 lights, 19
  points, and the glows' first step.

Headless shots through `town 3` and `town 4` show each town, its Chaos Gate
and its minimap with signs. The whole workspace's tests pass (705).

**Still unknown:**
- `DrawMap` for towns 2-4 draws four sprites in the later volumes; the
  port draws Infection's three, as for Carmina Gade.
- Neither town has been compared with the game in eemu.
- Which events place characters in these towns (Outbreak's and
  Quarantine's stories) is for those volumes' e2e.
