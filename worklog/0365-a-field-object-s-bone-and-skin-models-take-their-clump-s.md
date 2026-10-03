---
number: 365
title: A field object's bone and skin models take their clump's nodes: the dungeon mouth's hands, not a pink block
date: 2026-10-03
area: render, world
files: crates/piney-world/src/field_area.rs, docs/engine/field.md, docs/engine/animation.md
---

# 365. A field object's bone and skin models take their clump's nodes: the dungeon mouth's hands, not a pink block

Issue #37 (build 76fc4b6): "Strange graphic in mouth of dungeon". In
Cursed Despaired Paradise (INF story area 23, Theta, field type 8), a
pink rectangular block stood in the dungeon entrance's open mouth.

## Cause

The entrance is `field_k`'s `CMP_sfk1sto1`, played by `ANM_sfk1sto1a`.
`MDL_sfk1sto1` (rigid, 8 mmats) is the head. `MDL_sfk1sto1_a`-`h` are
eight bone and skin models (mtype 5) over the clump's 49 nodes: two
hands, each finger a chain of `OBJ_l/r ... finger` nodes. A bone or skin
vertex sits at its node's world matrix times its own position
([ccs-model.md](../docs/formats/ccs-model.md)).

An animated field object drew each posed object's model with no node
matrices (`nodes: &[]`). With none, the hands' vertices stayed near the
object's origin, the middle of the mouth, as the pink block. The game
gives an anm-made bone or skin model a node table: `ccAnm::SetAnm`
(INF SLUS_202.67:0x00151950-0x00151a04) fills it from the model chunk's
node list (+0x44, count +0x48), each node the object of its own entry
([animation.md](../docs/engine/animation.md#the-anms-objects)).

## Fix

`field_area::skin_nodes`: a bone or skin model's matrices are the nodes
of its object's clump (`Scene::clump_of`, ExtObj copies followed), as the
anm poses them (`Play::worlds`). Two draws use it:
- the field's animated objects (entrances, keys, lakes);
- `anim_edited` (TOBJ and the haze).

Both now draw `Play::instances` as [[363]] made the clump draws do. The
field object draw takes each Obj's own model (`SceneFile::drawn_model`)
in place of its lowest-numbered one.

Scanned over INF's DATA.BIN, the only scene files outside the characters
with bone or skin models are `field_k` and `field_k1` (this entrance,
eight each) and `t5` (Ω's TOBJ, one). The other draws that pass no node
matrices (towns, dungeons, the event areas) draw files without any.

## Checked

- **Pictures.** A diagnostic walked Kite to 2,600 south of the entrance,
  facing it. Without the fix: the pink block in the mouth, as the report
  shows. With it: the head, flanked by two pink clawed hands on the
  ground at its sides. (Facing the camera took a turn with R1/L1: the
  field camera's `rot()[2]` is the heading it looks along.)
- **The test.** `the_mouths_hands_take_their_bones` (piney-world) poses
  `ANM_sfk1sto1a`. `MDL_sfk1sto1_a` gets 49 node matrices, its left index
  finger's world more than 500 out along x; the rigid head gets none.
- **Suites.** piney-world (94) and piney-game (223) pass, as does
  `tools/test_field_ambient_rs.py` (the TOBJ path).

**Still unknown:**
- What the hands look like in the game is not checked against a capture.
  The port now places them over their nodes as the anm poses them; a
  screenshot of the entrance from the game would confirm it.
