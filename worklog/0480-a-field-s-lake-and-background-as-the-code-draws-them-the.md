---
number: 480
title: A field's lake and background as the code draws them: the lake's three animations and mirror, the hacked fields' backgrounds, DrawBG's turns and aurora
date: 2026-10-25
area: render, world, test
files: crates/piney-world/src/field_ambient.rs, crates/piney-world/src/field_area.rs, crates/piney-data/src/field/render.rs, crates/piney-data/src/field/mod.rs, tools/test_field_ambient_rs.py, docs/engine/field.md
resolves: 283.1, 284.2, 284.4
started: 2026-10-25T16:10:03-06:00
took: 56m
---

# 480. A field's lake and background as the code draws them: the lake's three animations and mirror, the hacked fields' backgrounds, DrawBG's turns and aurora

[[283]] carried, from [[38]], [[39]] and [[72]], that a field's water and
its clouds were approximations. Read against INF gcmn's `WORLD::Draw`
(0x005a97b0), `SetLake` (0x005ac7e0), `WORLD::Init` (0x005a4cd0) and
`WORLD::DrawBG` (0x005a7ef0), the port was further off than that, and on
every field.

## The lake

`SetLake` makes three `ccAnm`s of `field_eff`'s `ANM_sfwat1_1a` (`WORLD`
+0x53b0, +0x53b4, +0x53b8), each `SetFogSw(0)`, translated to the lake
`FOBJECT2`'s place at height 0. Each frame `WORLD::Draw`:
- writes `vftoi12(u$4930)` less the chunk's own into `MAT_sfzwat1_1`'s U
  (0x005a9ac8), and `ccModel::SetUV(0, vftoi12(v$4933), 0, 1)` into the
  materials of the third animation's `OBJ_water00` (0x005a9b78);
- moves all three to the copy of the place nearest the player
  (`ccTransPosW2P`, `P2W` of the second's own translation);
- draws the first and third on objLayer, the second on refLayer with
  `waterUVModifi(OBJ_water00)` (0x005a9d14), the picture under it;
- moves both scrolls on by 0.005, both back to 0 once `v` is past 1.

`ccModel`'s material list holds the stream's own `ccMaterial`s
(`ccMmat::Init` takes the chunk's object), so every animation draws with
both offsets. The port drew one animation on objLayer with a U scroll on
every material of `field_eff`, wrapping by 1; no mirror, no V, and the
scroll began at 0 in each field. `u$4930` and `v$4933` are statics of the
executable (gp 0x00378d4c, 0x00378d54); they now carry from field to field
in `SaveState::scroll_statics` (`WORLD_LAKE`), as [[474]]'s do. The
picture is the haze's machinery (`WaterZero`, the frame copy), so
[[97]]'s 128 x 128 copy still applies to it.

## The background

`WORLD::Init` picks the background by `isHacked` (+0x2c): `WORLD::WORLD`
makes it 2 and `GO` sets it from `WORLD_MAN` +0xf0, which is 2, or 3 in the
crisis and in the story areas whose `EVENTAREA_INFO.flag` is 3 (17 to 20,
22, 23, 25, 27, 44 to 48, 50 and more on INF). Not 2, it reads `backccs2`
(`bg_<x>1_{weather+1}`), `BGTBL2` (the sky is the row's `p`, then three
data layers and the mountains) and `BgMatName2`, and takes its light and
fog from `BGTBL2` (0x005a5a14, 0x005a5bb0). The port read `backccs`,
`BGTBL` and `BgMatName` always: every hacked field showed a plain sky,
light and fog.

`DrawBG` was ported as a V scroll on the sky about the player. The code:
- plain: `BgMatName`'s V from `v$4519`, which then goes on by 0.003, back
  by 1 past 1 (the port set 0);
- hacked: `rot2$4526.z` on and `rot3$4527.z` back by 0.001, each wrapped
  at 3.14; `BgMatName2`'s V written; the mountains (+0x60c0) drawn
  unturned on -60; `v$4519` back by 0.003, on by 1 below 0;
- both: the sky (-100) and first layer (-90) unturned, the second (-80) by
  `rot2`, field type 6's aurora (-80) by `rot3` with
  `SetUV(vftoi12(v$4519), 0, 0, 2)` of its first model (its U, after the
  scroll moved), the last layer (-70) by `rot3`;
- all about `WORLD`'s centre (+0x6140, `SetCenter`/`AddCenter`), not the
  player.
The layers are `WORLD_MAN` +0x494 to +0x4a4, priorities -100 to -60
(`GO`). The port drew the aurora on -60, over the mountains. The turns and
the scroll are statics too (`WORLD_BG`, `WORLD_BG_TURN`), so a plain field
after a hacked one draws its second cloud layer and mountains turned as
the hacked one left them, as the game does.

## The port

- piney-data: `Backgrounds::file_name`, `row_of`, `has_hacked`;
  `Field::scene_of(.., hacked)` with `Background { clumps, aurora,
  sky_material, hacked }` (`scene` is the plain one).
- piney-world: `field_ambient::lake` and `DrawBg` give the lake's and the
  background's draws as `Op`s, in `WORLD::Draw`'s order (the lake after
  the haze, the background last); `FieldArea` draws them with the
  materials' rows. `FieldArea::with_world` reads the hacked background
  when `hack` is not 2 (type 4 has none). `ee::vftoi12` replaces four
  copies in the towns.
- `tools/test_field_ambient_rs.py` no longer stubs `DrawBG`, records the
  lake's animations and `SetUV`, the background's clumps with their
  layers and matrices and both material writes, and sweeps isHacked 2 and
  3 (the rows past `BGTBL`'s, which `Init` would read from what follows,
  are left out). 120 cases of 1,000 frames and type 6's 12 of 3,400 (past
  the turns' wrap at 3,140): 0 mismatches.
- Tests: `area14_has_its_lake`; `a_field_scene_from_the_disc` now takes
  the same field hacked (`bg_p1_1`, `CMP_sfp9bac1` to `CMP_sfp9mou1`,
  `MAT_sfp9dat1_3`, `BGTBL2`'s light). Area 14's lake and area 18's
  hacked sky were rendered headless and looked at.

Two more of [[284]]'s list were already answered and are closed here:
284.2, the explosions' palette swaps, by [[411]] (`AnmObj::change_clut`),
and 284.4, the weapon flashes' light, by [[377]] (`Combat::rad_lights`).

The other steps of the clock: piney-game suite 334 passed; piney-world,
piney-data and piney-desktop tests, clippy and fmt clean.

**Still unknown:** how the lake's mirror and the hacked skies look on a PS2 beside the port's (the frame copy's size is [[97]]'s)
