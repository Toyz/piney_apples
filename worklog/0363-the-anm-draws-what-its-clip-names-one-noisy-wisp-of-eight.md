---
number: 363
title: The anm draws what its clip names: one Noisy Wisp of eight, and every ExtObj copy
date: 2026-10-03
area: render, battle
files: crates/piney-data/src/anim.rs, crates/piney-data/src/scene.rs, crates/piney-desktop/src/assets.rs, crates/piney-world/src/pose.rs, crates/piney-world/src/foe.rs, crates/piney-world/src/body.rs, crates/piney-world/src/field_area.rs, crates/piney-world/src/draw.rs, crates/piney-world/examples/foe_probe.rs, tools/test_foe_rs.py, docs/engine/animation.md
---

# 363. The anm draws what its clip names: one Noisy Wisp of eight, and every ExtObj copy

Issue #35 (build 76fc4b6): "Poltergeist / Noisy Wisp type enemies do not
appear to be scaled correctly". In INF, `enemyTbl` rows 280 (Ectoplasm)
and 281 (Noisy Wisp) share the file `eww1`. No row is named Poltergeist.
`foe_probe --shot` drew either as a huge column of spiked red bodies,
stacked several high.

## Cause

`eww1`'s clump `CMP_trall` holds eight bodies, `OBJ_eww1bod01`-`08`, each
with a model. Every clip of the file names one of them. The port drew
every clump node that has a model, so all eight, overlapping, at their
rest places.

The game draws something else. `ccAnm::Draw` (INF SLUS_202.67:0x001524d0)
walks the anm's own entries (count at `*(anm +0x94) +0x20`, 12 bytes each
at anm +0xac). It draws an entry only when bit 4 of its flag byte (+10)
is set: type 0x100 by `ccObj::Draw`, 0xe00 by `ccEffObj::DrawNoAnm`.
`ccAnm::SetAnm` (0x00150f50) builds the entries:
- **One per index entry.** It makes one entry for each entry of the
  chunk's index: `ConvLCNum2ALCNum` (0x00144e90) lists each chunk the
  records name, once, by local chunk number; `MakeAnimeIndex` (0x001474b0)
  keeps the list.
- **Clump nodes.** A clump's Obj node takes the entry that names it
  (flags 6). When several entries name one piece through ExtObj copies,
  the last one takes it.
- **The rest.** Every other Obj entry gets a new `ccObj` (flags 7).
- **Parents.** From 0x00151a24, each entry hangs from its parent slot's
  object, or from the anm itself.

[animation.md](../docs/engine/animation.md#the-anms-objects) has the
whole of it. So a node no record of the clip names is never drawn, and a
piece named by n copies is drawn n times, each posed by its own record
under its own parent. The port had both wrong.

## How much of the data it touches

A scan of every clump and clip pair in INF's DATA.BIN (a throwaway
example, not kept) found 185 pairs that leave a node with a model
unnamed. The enemies among them:
- `eww1` names one body of eight per clip;
- `etn1`, the mimic, waits as a closed chest; its legs, fangs and coins
  are named only by the clips that show them;
- `efm1`, `epx1` and `ekx1` hold spare bodies;
- `elg1` has a needle;
- `eks1`'s `OBJ_trall` carries a model that no clip names. The pose check
  in `tools/test_foe_rs.py` already listed it as never driven.

The other pairs are bosses' files (`x04`, `x31`, `x41`, `x51`, `x61`,
`x71`, `x81`), the title's, `xdttopen0`'s and `drain`'s scenes, and three
room pieces.

Copies drawn once that the game draws several times:
- `ecc1`: legs 1-3 are copies of `OBJ_ecc1leg0`, so four legs from one
  model;
- `ecx1`: six legs and two eyes;
- `evb1`, `evba`, `evbb`: swarms of 4, 8 and 16 bees (body, wings and
  abdomen each, every copy under its own body copy);
- `eus1`: a second sword in the hand while it draws;
- `xgsabod1`, `xgsebod1`: copies of the god statues' meshes;
- `eka1`: two swords named by its clips but in no clump.

The port showed one bee, one leg, no second sword.

## Fix

- **piney-data.** `Animation::objects` is the object half of the index:
  each object named by an object, F_Obj or F_Note record, once, in the
  order first named, with the piece it drives.
  `Animation::entry_poses_at` gives each entry its own pose (its track,
  else its last F_Obj record reached). `Scene::model_of` is each Obj
  chunk's model, as `ccObj` holds it; `model_owner` keeps only a model's
  first owner. `SceneFile::drawn_model` is the model `ccObj::Draw` draws.
- **piney-world.** `Play::instances` builds `SetAnm`'s objects in index
  order. Each has its world matrix, under its parent entry or the root,
  and its own transparency. `foe::Model::draw_with`, `Body::draw_parts`
  and the field's TOBJ (`anim_edited`) draw those. Their shadows follow,
  through `draw::cast_shadows`, which now takes (object, world) pairs. A
  model hung on a node (`Body::attach`, the weapons) is drawn on the
  node's own object, the last entry naming it.
- **The node's own pose.** A clump node named by several entries is the
  last one's object. The port had taken the first track's pose for the
  node (`Animation::locals_at`), which feeds skinning and every reader of
  a node. `Play::worlds` and `foe::Model::worlds` now start from the
  instances, a later entry overwriting an earlier, and `Body::node_alphas`
  does the same. The other nodes keep their rest pose under their
  parents.

The other `ccAnm::Draw`s in the port already drew by entry: the
desktop's (`piney_desktop::anm`) and the effects'
(`piney_effect::nodes::anm`). The streams' scenes are `ccStream`'s, with
their own rule (`piney_stream::Scene::visible`).

## Checked

- **Pictures.** `foe_probe --shot`, before and after:
  - Noisy Wisp: one small wisp, not the column;
  - the mimic: a plain closed chest while it waits;
  - `evb1`: four bees;
  - `ecc1`: its four legs;
  - `eus1`: both swords.
- **The game's index.** `tools/test_foe_rs.py` gets `index`. The game's
  `ConvLCNum2ALCNum` runs over every animation of 13 files: the above,
  plus `eks1` and `ebl1`. Its Obj and ExtObj entries, in order, match
  `Animation::objects` for all 81 animations. The answer is a new
  `foe_probe` request, `index STEM`.
- **The node's pose.** `tools/test_foe_rs.py`'s `pose` now also covers
  rows 79, 80, 246 and 271 (`ecx1`, `ecc1`, `eus1`, `evb1`). Its harness
  gives each clump node the coord of the last entry driving it, as
  `SetAnm` does; it used to take the first. Result: 132 frames, 2,220
  nodes, 0 mismatches. With the harness's old first-entry rule, the same
  run shows 84 mismatches.
- **Rust test.** `the_anm_draws_what_its_animation_names`
  (piney-world):
  - the Noisy Wisp draws `OBJ_eww1bod01` alone;
  - the waiting mimic draws its two chest halves;
  - `evb1` draws 16 objects, four of them bodies at four places;
  - `eus1`'s ski0 draws its sword twice.
- **Suites.** These pass:
  - the piney-data, piney-desktop, piney-world (92 + 12 + 3) and
    piney-game (222) suites;
  - `tools/test_foe_rs.py`, `test_world_rs.py`, `test_field_ambient_rs.py`,
    `test_grunty_rs.py`, `test_evchar_rs.py` and `test_battle_boss_rs.py`;
  - the boss harnesses on their own volumes: Magus, Innis and Kyvia on
    MUT; Fidchell and Gorre on OUT.

## Volumes

The scan of the files it touches covered only INF's DATA.BIN. The code
was compared on all four volumes:
- **MUT.** `SetAnm`, `Draw`, `MakeAnimeIndex` and `ConvLCNum2ALCNum` are
  INF's, but for data addresses.
- **OUT, QUA.** All four are recompiled, with other registers and stack
  slots. The flag logic is the same: `Draw` tests bit 4 and draws types
  0x100 and 0xe00; `SetAnm` stores 1, 6, 0 and `| 6`, tests bits 1 and 2
  for the made objects and the parents, stores the parent at +0x80, and
  clears +0x30.
- **The index check.** `index`, with the game's own `ConvLCNum2ALCNum`,
  passes on every volume (`PINEY_VOLUME=mutation`, `outbreak`,
  `quarantine`): 81 animations each, 0 mismatches.

**Still unknown:** nothing.
