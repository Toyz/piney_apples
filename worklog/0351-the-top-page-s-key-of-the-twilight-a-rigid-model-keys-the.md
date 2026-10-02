---
number: 351
title: "The top page's Key of the Twilight: a rigid model keys the sorted group on its own vertex box, not a Bbox chunk"
date: 2026-10-02
area: render, ui
files: crates/piney-desktop/src/assets.rs, crates/piney-toppage/tests/toppage.rs, docs/engine/desktop.md, docs/formats/ccs-model.md
---

# 351. The top page's Key of the Twilight: a rigid model keys the sorted group on its own vertex box, not a Bbox chunk

Issue #25 and the user's reference: on the top page (INF `xdttopen0`) the
sword carries "Key of the Twilight" in yellow down its blade, lit by an
orange glow. The port showed a bare blade.

## What the port drew

The text (`MDL_xdtworld1`, alpha 0.6-0.9) and its glow (`MDL_xdthotw`,
alpha 0.2-0.4, scale 1.0-1.2) are children of the sword (`ExtObj` parent
`OBJ_xdtsword`). The text sits 2000 units in front of the blade, the glow
3000. Both are translucent (texture flag 0x08) and so go to the layer's
sorted group. Both alpha-test GEQUAL `trunc(aref * t)` with AFAIL
FB_ONLY, so each writes Z where its alpha passes. The port keyed both
with the w-row formula, `M[2][3] / M[3][3]`, because no Bbox chunk names
them. That key is the model's z scale over its depth: the glow (scale
over 1) came first, wrote Z over its bright core, and the text behind it
failed the depth test. In the opening the text showed at frame 110 and
went when the glow came in. Drawn alone, each landed in the right place.

## The game's rule

Every rule on the way was checked and agreed:
- the w-row formula itself (`ccModel::Draw` 0x0013ebbc reads offsets 12,
  28, 44 over 60);
- `ccDLSort::Add` and the in-order walk (`StoreDLNode` 0x00108100,
  ascending);
- AREF (`ccSetMaterialPacket` 0x0013e770, `trunc(aref * t)`);
- ZBUF never masked (set once in `SetScreenModeMain`);
- the blend type (`flag & 3`; bit 3 dropped);
- vertex alpha 0x80.

So the game itself was run. `tools/test_effect_draw_rs.py`'s
`SceneMachine` decodes `xdttopen0` with the game's own `DecodeSetup` and
builds `ANM_xdttop1a` with the game's `ccAnm`. Its matrices match the
port's at every step tried (1, 30, 60, 120). Letting `ccModel::Draw` run
natively under the camera's view then stopped in
`_ccCheckBoundingBoxEx`. The game had taken the bounding-box branch.

`ccModel::Init` (0x0013a550) sets `ccModel` +4, the box pointer, to the
model chunk's own box (`ccModelChunk` +0x10) for every model without
`mtype & 6`, and to null for a bone or skin model. `Decode_Model` fills
that box for every model through `ccBbox_SetBox` (0x001388c0). Per axis
it takes the integer min and max over all vertices, starting at 0x10000
and -0x10000, doubled about the centre only for `mtype & 6`. The centre
at +0x30 is `((min + max) >> 1) * vertexScale / 4096`. `Decode_Bbox`
fills a list of its own and never reaches it.

So a rigid model always keys on the screen Z of its vertex box's centre.
The w-row key is only for bone and skin models. The desktop page and the
port had it the other way round: a key from a Bbox chunk, and the w row
for everything without one. Read from the chunks the game decoded, the
centre matches the port's new `vertex_box_centre` on all 41 models of
`xdttopen0` and all 138 of `xddesk01`.

With the real key the text (screen Z 5342) is drawn before the nearer
glow (5368). The glow lays its orange over the lettering, as in the
reference.

## Tests

- `the_key_of_the_twilight_is_drawn_before_its_glow` (piney-toppage) at
  frames 150, 300 and 421. It fails with the old keying ("the glow drawn
  before the text").
- `model_box_centres_are_the_games`: the sword, text and glow centres as
  the game's decoder gives them.

The desktop, demo, stream, world and game suites pass unchanged.

**Still unknown:**
- [[226]] took Mutation's recall boards (`MDL_blackbord_a01`) as models
  without a box, and left open why their cross-fades come out black. If
  those boards are rigid they now key on their box centres, which may be
  the smooth fade 226 suspected. Not yet looked at.
- The port does not cull by the box (`_ccCheckBoundingBoxEx`, VU0
  `mc0_CheckBoundingBox`). That cull only drops models wholly outside the
  view.
