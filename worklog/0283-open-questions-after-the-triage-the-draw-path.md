---
number: 283
title: Open questions after the triage: the draw path
date: 2026-09-28
area: render
files: UNKNOWNS.md
resolves: 17, 26, 29, 32, 34, 35, 36, 38, 39, 47, 48, 55, 61, 84, 88, 107, 112, 127, 130, 137, 138, 139, 145, 146, 147, 151, 152, 157, 158, 160, 161, 165, 231, 238, 244
---

# 283. Open questions after the triage: the draw path

This entry closes the open questions of 35 entries about the draw path: the
VU1 programs and the model packets, animation, fog, lights, shadows, water
and the towns', fields' and dungeons' drawing. The triage of 2026-09-28
(UNKNOWNS.md) split every entry's open paragraph into single questions and
checked each one against the later log, the docs and the code. The answered
ones are listed below with their evidence. What is still open is restated at
the end, with the entry that asked it, the play questions first. Open
questions of these entries that belong to another subsystem are restated in
that subsystem's entry: [[282]], [[284]], [[285]], [[286]], [[287]],
[[288]], [[289]] and [[291]].

## Answered

- [[17]]: the exact z the V2 unpack delivers — answered by [[244]] (PCSX2's
  unpacker copies x, y into z, w; z = S + bits(1/16)); rests on PCSX2's
  source, not a console
- [[17]]: the shadow programs beyond their entry points — answered by
  [[160]] / docs/engine/shadow.md
- [[26]]: the unit of the material crop offsets — answered by [[29]] (a UV
  animation's reference, not an offset)
- [[26]]: the lit models' lighting (the viewer's headlight) — answered by
  [[47]] (VU1's lighting), [[61]] (the town's lights)
- [[26]]: `town01`'s missing water — answered by [[28]] (`wat1`, the canal
  water)
- [[26]]: animation key interpolation — answered by [[32]] (keys, rotations,
  loops and morphs checked)
- [[29]]: the unit of the run-time UV offset, and where
  `ccSetMaterialPacket` makes the ST row — answered by [[137]]
  (`(u >> 4) & 0xff` in 1/256 of the texture, through STROW)
- [[32]]: how the draw turns `ccMaterial.u`/`v` into the ST row — answered
  by [[137]]
- [[32]]: the light, camera, ambient, `F_Obj` and note records — answered by
  [[137]] (read where played; `F_Obj` ported), [[139]], [[278]] (notes)
- [[32]]: whether the field changes `ccAnm::frameSpd` — answered by [[137]]
  (yes: Kite's `AnimCtrl`, motions, act -5)
- [[32]]: that the field runs at 30 frames a second — answered by [[59]]
  (frame rate 2)
- [[34]]: `SetLight`'s omni lights and glow effects — answered by [[157]]
  (`SetLight` ported)
- [[34]]: the `clutType` 3 and 4 palette swaps — answered by [[147]]
  (`SetClutList`, `ChangeClut`)
- [[34]]: `GetBG` for the lake dungeons, and the white plane in lake
  doorways — answered by [[161]] (`GetBG` picks the sky domes; the sky shows
  through the doorways)
- [[34]]: whether the GS fog uses view depth as the viewer assumes —
  answered by [[145]] (VU1's per-vertex `fogB + fogA * w`;
  `piney_draw::DepthFog`)
- [[35]]: the morph blends (`F_Morpher` cloth) — answered by [[36]]
- [[35]], [[36]]: texture-offset animation — answered by [[137]]
- [[35]]: the animation's transparency channel, read but not drawn —
  answered by [[100]] (objects drawn at their own transparency), [[137]]
  (`F_Obj`'s `localtp`)
- [[38]]: how the fog table's near, far and percentage map to the GS fog —
  answered by [[145]] (`DepthFog::set_fog` is `SetFog`'s arithmetic)
- [[39]]: the viewer's missing draw window and fade, sky scroll and
  player-centred background — answered by [[72]] /
  `crates/piney-world/src/field_area.rs` (`DrawBG` centred on the player,
  the sky's V scroll), `crates/piney-data/src/field/render.rs` (`FADE_FROM`
  6700, `COVER_RANGE` 7200)
- [[39]]: the viewer's fog depth mapping — answered by [[145]]
- [[47]]: the desktop's and the field's lit models against VU1 — answered by
  [[47]]'s shared fix, [[61]] (the town's lights), [[141]]
  (`tools/test_lights_rs.py`: the light group against
  `ccDrawEnv::SetLightMatrix`)
- [[47]]: the "transparency" report — answered by [[48]] (`FB_ONLY` drew
  colour before Z)
- [[61]]: the merchants, the Recorder, the walking PCs and Orca — answered
  by [[65]], [[69]], [[70]]
- [[61]]: the gate's menu — answered by [[68]]
- [[61]]: water, fog, shadows and the arrival's effect in town — answered by
  [[97]], [[145]], [[160]], [[129]]
- [[61]]: the user's movement stalls — answered by [[62]] (a round DualSense
  stick against a DualShock 2's threshold)
- [[84]]: the story map's models drawn without `SetFog`'s fog — answered by
  [[145]]
- [[84]]: event 11's instructions in the map — answered by [[90]], [[91]]
- [[107]]: the condition colours `ccChar::Draw` applies in the fights —
  answered by [[195]] / `crates/piney-world/src/foe.rs` `char_blend` (used
  by the combat draw; `foe.rs` unit tests)
- [[112]]: `FIREFLY2`, the arena's fireflies — answered by [[117]] /
  docs/engine/evarea.md (checked against the game's,
  `tools/test_evarea_rs.py`)
- [[127]]: the port's fog per model, not per vertex — answered by [[145]]
- [[130]]: `EntryGimmick` in a field — answered by [[164]]
- [[130]]: `WORLD_MAN`'s two `ccBufferSampling` depth shades — answered by
  [[152]]
- [[130]]: `TOBJ`'s shadow and sounds — answered by [[160]] (shadow),
  [[150]] (the hum)
- [[137]]: the direct light controller (0x0605) — answered by [[151]]
- [[138]]: `ChangeClut`, the door palette swap for `clutType` 3 and 4 —
  answered by [[147]]
- [[157]]: the lakes' fireflies — answered by [[161]]

**Still unknown:**
- (play) [[38]], [[39]], [[72]]: the field water's run-time textures and
  scroll, and the clouds' blending, are approximations
  (docs/engine/field.md, field-walk.md) — every field with water or clouds.
- (play) [[97]]: water 0's frame-buffer copy is the whole frame, not the
  game's 128 x 128 point-sampled one (docs/engine/field-game.md, town02.md)
  — the towns' water.
- (play) [[157]], [[161]]: `EntryObject`'s palettes for the lakes' statues
  and flowers when `GetBG` is not 0 are not ported (docs/engine/dungeon.md)
  — lake dungeons by evening and night.
- (play) [[158]]: `DUNGEON.fog` is not zeroed at the game over's television,
  and the squeeze moves the whole finished frame, not only what `sysLayer`'s
  view draws — a game over, in a dungeon most.
- (play) [[68]]: the white flash of the gate menu in town is dropped
  (`R::Flash` in `crates/piney-game/src/world.rs`) — leaving a town through
  the gate.
- [[17]]: the DMA order of the matrix, material and model packets
  (docs/engine/render.md Unknown).
- [[17]]: the full bone and skin packet layout, and which of the `b`/`m`
  program families is skin (docs/engine/render.md Unknown).
- [[17]]: what `view + 0x110` holds, and the `mc04b`/`mc04m` variants
  (docs/engine/render.md Unknown).
- [[244]]: the port's ST ignores the `1 + S * 2^-23` factor, 3e-5 of a
  repeat, below a texel.
- [[26]]: textures defined in other files (`#` entries) show white in the
  viewer; how `#` entries resolve is not written down.
- [[27]]: the 15 morph-target-shaped models that no `F_Morpher` in their own
  file names.
- [[36]]: which of the 11,086 `F_Morpher` records are faces and which cloth;
  not needed to draw them.
- [[55]]: `ccMorpher`'s `GetWork` first-call path has never run (the same
  arithmetic).
- [[137]]: the spot light controller (0x0607); no Infection room uses one
  (docs/engine/animation.md Unknown).
- [[137]]: `F_Obj`'s flag byte (`+0xa2`) is not modelled, except its
  effect-object branch for the streams ([[156]]).
- [[137]]: whether an `F_Obj` pose outlives its clip; `F_Obj` is not checked
  in eemu as a whole, only its parts.
- [[151]]: whether anything gives the direct light a parent coordinate, and
  whether the room anm's loop re-applies its frame-0 place.
- [[141]]: the areas' light positions and colours at frame 1 are not checked
  (docs/engine/field-game.md Unknown).
- [[141]]: `ccDrawEnv +0x88`, a second light group `SetLightMatrix` walks
  first, is not traced (docs/engine/field-game.md Unknown).
- [[61]]: the order in which the gate's light joins the town's; [[141]]
  checks only the group logic.
- [[114]]: how `ccLight` combines an event room's `LGT_` record with
  `SetRoom`'s place; the block's clump nodes drawn at its matrix
  (docs/engine/dungeon.md Unknown).
- [[160]]: the shadow bounding box check reads `vf21`, never set; an open
  edge's second face reads VU1 memory below the directions (game quirks).
- [[160]]: shadow modes 2 and 3, never made in Infection, are not ported;
  the later volumes are not surveyed for them.
- [[160]]: `EVENTAREA02` and `07` set the light again after `GO`; noted, not
  followed.
- [[38]]: the field objects' own draw code, and the type-6 `FOBJECT2`s
  `DrawMesh` draws (docs/engine/field.md Unknown).
- [[38]]: whether `ExtObj`-shared object models are lit once or per use.
- [[38]]: whether a tile's alpha is clamped past 7,200 (docs/engine/field.md
  Unknown).
- [[38]]: whether VU0 rounds as the modelled truncation; needs a console.
- [[61]]: the near-plane rule is read from the VU1 microcode, not run.
- [[61]]: the CPU renderer drops triangles behind the camera instead of
  cutting them (the software model only).
- [[48]]: edge rasterisation (the GS's fill rules against the GPU's) and the
  selection bar's row are not explained.
- [[37]], [[48]]: the order of failing and passing pixels where one draw
  overlaps itself (`FB_ONLY`'s two passes); no test and no known case.
- [[37]]: the GS's 8-bit rounding inside the pixel pipeline is not modelled;
  [[175]] snapped vertex alpha only.
- [[48]]: both renderers are ours; neither is compared with a real GS frame
  capture.
- [[53]]: which group, opaque or sorted, a mmat lands in is taken from the
  disassembly, not compared with the game.
- [[57]]: VRAM past the draw buffer reads 0 in the port
  (docs/engine/stream.md Unknown).
- [[121]]: the port's draw clips by the scissor, not by the letterbox's clip
  box (`bboxClipMin`/`Max`).
- [[152]]: the GS's bilinear weights and reads past the frame for the depth
  shades; model Z clamped at 2^32 where the VU saturates.
- [[145]]: the per-vertex fog: F values not run against VU1; `PRIM.FGE`
  taken from `SetFogSw` (docs/engine/evarea.md Unknown).
- [[157]]: what `ccObj::Duplicate` copies of the water's object
  (docs/engine/dungeon.md Unknown).
- [[97]]: water 0's place for `waterUVModifi2`'s reach is taken as its root;
  its coordinate is not checked in a run.
- [[84]]: `EA_moveTex02`'s scroll restarts at 0 with a new map after the
  town, where the game's carries on (docs/engine/evarea.md Unknown).
- [[161]]: the lake sky's scroll starts at 0 in each dungeon, where the
  game's runs on (phase only).
- [[99]]: where `scFadeDef` draws (its layer word `+0x94`); taken as the
  font layer.
- [[69]]: the fader drawn in the field with the desktop's corner constants;
  not checked.
- [[130]]: the lens flare is left above the picture by the follow camera,
  and the eye view fails its camera test, as in the game's code; no shot.
- [[165]]: the double `atan2` is Rust's libm, not newlib's; they agree in
  every case tried.
- [[170]]: whether `ccChar::Draw` leaves node coordinates stale on a culled
  frame (docs/engine/battle.md Unknown).
- [[177]]: whether the game's `GetChunkAdrsF` falls back to other loaded
  files for block 1.
- [[264]]: whether `EVENTAREA01`'s `DrawObj`/`DrawObj2` layer differs from
  `objLayer`; the port takes the active layer Draw set.
- [[26]], [[59]]: whether the game shows `town01`'s floors that dark, and
  the crisis town's drawing; never compared with a console picture.
- [[97]]: what the clouds look like on the console; no capture.
- [[99]]: the piros tint's pixels, drawn by the port's fog blend; not
  compared.
- [[96]]: the gate hack's 3D against the game's pixels; the calls are
  compared.
- [[130]]: the balloon's cloth scroll and Ω's runners, not seen in a shot
  (the story server is Δ).
- [[139]]: the walking PCs' variants and steps against the game's pictures
  and sounds (docs/engine/field-game.md Unknown).
- [[145]]: the per-vertex fog against a game picture.
- [[146]]: the `REGION_REPEAT` shades against the game's pictures; the texel
  choice is checked.
- [[147]]: a swapped dungeon against a picture; a twin the file lacks gets a
  null palette in the game, where the port leaves it unswapped.
- [[152]]: the field's depth shades against a game picture.
- [[88]]: the sharp-bilinear scaler is not seen at the user's window size,
  only checked by test; there is no setting for a plain or television-like
  filter (a port option).
