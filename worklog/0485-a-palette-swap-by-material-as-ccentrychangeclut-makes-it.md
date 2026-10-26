---
number: 485
title: A palette swap by material, as ccEntryChangeCLUT makes it: other materials on the same palette keep their colours
date: 2026-10-25
area: render
files: crates/piney-draw/src/lib.rs, crates/piney-gs/src/lib.rs, crates/piney-desktop/src/soft.rs, crates/piney-world/src/foe.rs, crates/piney-effect/src/portal.rs, crates/piney-stream/src/file.rs
resolves: 284.3
started: 2026-10-25T18:13:47-06:00
took: 13m
---

# 485. A palette swap by material, as ccEntryChangeCLUT makes it: other materials on the same palette keep their colours

Play question 284.3: the port did `ccEntryChangeCLUT` as a swap of the
palette that `MAT_clut`'s texture uses, so every other texture on that
palette took the new colours too. The game swaps per material.

**The code.**
- `ccEntryChangeCLUT` (gcmn 0x0042e670) runs for an enemy
  (`initEnemyCCS`), a gimmick box (`ccGimBox`) and the portal
  (`ccMagicCircle`). It calls `ccClump::Duplicate(0x2000)`, then
  `ccClump::ChangeClut(entry.clut, MAT_clut)`.
- That `ChangeClut` (main 0x0013d610) walks each Obj node's model's used
  materials (`SetUseMaterialTbl`'s list at +0x38). Where the
  `ccMaterial`'s chunk (+0xc) is `MAT_clut`, it sets the material's
  palette (+4).
- The palette form, `ccClump::ChangeClut(clut, old)` and
  `ccModel::ChangeClut` (0x0013d530, 0x0013aad0), sets +4 on every
  material whose palette is now `old` (all of them with `old` 0).
- So both change a `ccMaterial`'s palette in turn: one by material, one
  by the palette as earlier changes left it.

**The port.**
- `ModelDraw::clut_swaps` now takes both kinds: (CLT_, CLT_) by palette
  and (MAT_, CLT_) by material. Object numbers never collide within a
  file.
- `piney_draw::clut_of` folds them in order for each mmat, as the game
  changes the material. piney-gs and the soft renderer both use it; the
  soft renderer had ignored the swaps until now.
- `foe::Model::change_clut` (enemies, boxes, the portal's look, the flag
  race's flags), the effects' portal and the book covers'
  `StreamFile::change_clut` (`mat->tex.clutChunk`, also per material) now
  give (MAT_, CLT_).

**What it changes.** Over every `enemyTbl` row, 15 rows have another
material on `MAT_clut`'s palette, so in the port they took the swap too:
- ecs1 (87); egk1 (145-150); elg1 (205), with five other materials;
- esf1 (231, 232); eus1 (247, 248); ezb1 (293, 294, 296).
Those materials now keep their own colours.

The book covers' test (`a_ryu_books_cover_takes_the_books_palette`) now
expects Book III's swap as (`MAT_clut`, `CLT_x800bac1c2`).

Checks:
- `palettes_change_by_material_and_by_palette` (piney-draw).
- `a_palette_swap_is_the_materials_own` (piney-world). It replaces the
  test that only checked that no other material shared `MAT_clut`'s
  texture. It now covers rows 87, 145 and 205 too, and checks every
  material of each file against the swaps. Three swaps share their
  palette.

**Still unknown:** nothing
