---
number: 614
title: The shadow volumes to the word and the pixel
date: 2026-11-18
area: render, test, volumes, tooling, audio
files: crates/piney-desktop/src/shadow/mod.rs, crates/piney-desktop/src/shadow/draw.rs, crates/piney-desktop/src/shadow/vu1.rs, crates/piney-desktop/src/shadow/gif.rs, crates/piney-data/src/vu1.rs, crates/piney-data/src/shadow.rs, crates/piney-data/src/model.rs, crates/piney-draw/src/lib.rs, crates/piney-data/examples/body_probe.rs, tools/test_shadow_rs.py, tools/test_ccs.py, tools/vu.py, tools/test_vu_rt.py, tools/sound.py, tools/test_sound_rt.py, tools/tables.py, tools/test_tables_rt.py, tools/test_mips_rt.py, docs/engine/shadow.md, docs/engine/render.md, docs/formats/ccs-model.md, docs/formats/voice.md, crates/piney-desktop/src/assets.rs, crates/piney-desktop/examples/shadow_volume_probe.rs, crates/piney-world/src/draw.rs, crates/piney-stream/src/draw.rs, crates/piney-eemu/examples/ee_run.rs, tools/volume.py, tools/test_areas.py, tools/test_battle.py, tools/test_dungeon.py, tools/test_evscript.py, tools/test_field.py, tools/test_font.py, tools/test_field_host.py, docs/engine/heap.md
started: 2026-11-18T03:29:27-07:00
took: 22h 33m
---

# 614. The shadow volumes to the word and the pixel

The shadow volumes are now the game's to the word: `piney_desktop::shadow` runs `ccShadowModel::Draw` (INF SLUS_202.67:0x001412b0) in the EE's and VU0's arithmetic and VU1's `mc_DrawShadow1`, `2`, `3`, `mc_DrawShadowClip` and the two scissor programs (VU1 0x5d2-0x758) in VU1's, and gives the GIF packets VU1 kicks. `tools/test_shadow_rs.py` compares every kicked word and every buffer pixel exactly on all four discs, with no allowance left. The bone and skin packets' VIF codes are read and the port's upload matches them word for word; `test_vu` and three more Infection-pinned harnesses follow PINEY_VOLUME. Then, from the frame: the `vf21` a shadow's check reads is what its owner's `ccObj::Draw` left (its rigid model's own check, which the port now runs, or `_SetLWMatrix`'s stack qword), `charTbl`'s 21 rows come from `NewGame`'s loop, the later discs' voice tables end where their code says, and the widened sweep's Infection-pinned harnesses check each disc's own values.

## The shadow path

The EE half, read from 0x001412b0: `R` is `sceVu0InversMatrix` of the world matrix with its translation zeroed; `L_local = R L`; the stretch `(R diag(1/s)) L` times the length, w 0 (mode 3: none); `screen = world_screen * lw`; the box check; `M = B * screen * diag(s)` with `B = to_buffer * to_corner` (`to_corner` the translation `(-c0, -c1)` by `neg.s`, `to_buffer` the scale `w / (c2 - c0)`, `h / (c3 - c1)` and the offset `(4096 - w) / 2`); the near point `(B P) (0, 0, max(near, 8), 1)`; the scissor `(-(0.5 (4096 - w)), ..., 0, -1)` and `(0.5 (4096 + w), ..., 0, 1)`; the registers by mode, swapped when `cross(lw0, lw1) . lw2 < 0`. Every product is `sceVu0MulMatrix`'s or `ApplyMatrix`'s order through `field::ee` (`vu0::mul_matrix`, `vu0::apply`).

VU1, as the microcode runs: vertices `ITOF12` with w 1, moved by the stretch when their face's flag is 0, `mulax/madday/maddaz/maddw` through `M`, `DIV Q` and `MULQ.xyz`, `FTOI4`; the winding `(v1 - v0).x (v2 - v0).y - (v2 - v0).x (v1 - v0).y` of the divided points, its sign bit picking the first or second colour and ALPHA; the direct kick holding every polygon, ADC (0x8000) on the last vertex of one that goes to the clip path, 0x7f80 otherwise, a quad's first two vertices always 0x8000. The clip path cuts at the near point's w (a crossing always from the vertex in front, `front + d (near.w - front.w) / d.w`, the vertices behind put on the plane), and each part goes through `mc_ScissorShadowPolyXY`: a kick of 0x203 (the fan tag, 0x204 and 0x205 as the last polygon left it) before the colour and ALPHA are stored, four cuts (x low, x high, y low, y high) in homogeneous space with the buffers alternating at 0x208 and 0x1f9 (`ADDx` copies flushing -0 as the code does), then the TRIFAN from 0x204 with each vertex's w counting down from 0x8001. `piney_data::vu1` holds the VU1 operations (`itof12`, `ftoi4`, `project`, `cross_xy`, `lerp`, `plus0`, `sign`); everything else is `field::ee` and `vu0`.

**The path question is settled from the code.** Every flag test in the shadow, model and box programs stands exactly four pairs after the instruction it tests: `FMAND` at 0x630 after the cross product at 0x62c, 0x691 after 0x68d, 0x6bd/0x6be after 0x6b9/0x6ba, 0x73e/0x740 after 0x73a/0x73c, 0x6f3 after 0x6ef; `FSAND 0x002` at 0x0e1 and 0x128 after the `SUBx.w` at 0x0dd and 0x124; `FSAND 0x080` at 0x62e and 0x69a after the last guard-band `SUBA` at 0x62a and 0x696. So the status flag, like the MAC flag, lands at the end of the FMAC's four stages, and `FSSET 0` (0x624, 0x68e), in the pair before the first subtraction in both programs, isolates them only if its write lands among the FMAC writes in issue order. The polygon takes the clip path exactly when a vertex is outside the guard band. The old harness had the status flag set at once, which let the next vertex's transform, the screen edges and the cross product into the test; the port had taken the clip path for everything. Both now take the pipelined model (`flagged` in the harness; `Vu1::outside` in the port).

**The box check**, read too: `mc0_CheckBoundingBoxShadow`'s first two `add` set only w (`add.w vf21, vf19, vf20`), so its last eight corners are the reach `L_local len` itself and `vf21`'s x, y, z as the object's `ccObj::Draw` left it (below); a side of the clip box (+0x1d0/+0x1e0) with no corner within culls. The box is `DecodeShadowModel`'s extremes through `ccBbox_SetBox`, and the extremes are an else-if (0x00140ab4): a value below the least so far is not tried as the greatest, so the first vertex sets only minima (the harness found this on its second draw: a y maximum of -5.875 below the box's own positive vertices). The initial values are `Decode_Model`'s 0x10000 and -0x10000 (0x0014bddc), which the harness's decoder now uses too.

**VU1's memory between draws.** An open edge's second face is read at `0x211 - face`: `e4w1`'s four open edges (the only ones in DATA.BIN on all four discs) are on faces 8 and 10, so 0x209 and 0x207. 0x207 is the fan's second tag, copied by `mc_DrawShadow1` every draw (w 0: unlit); 0x209 is what the last clipped polygon left (a fan vertex, w 0x8000, or a scissor pass's float). 0x211 is computed in the pipeline's prologue from the fan tag taken as floats. The port keeps what VU1's memory carries from draw to draw (`Vu1Carry`, below), and runs the VU1 programs at the pass, in the GS's order (the last alpha group first, `AddPacket`'s order within one), keeping each cast's setup and an `Arc` of its mesh (`SceneFile::shadows` now holds `Arc<ShadowMesh>`).

**The GS.** `gs_triangles` reads the kicks as PATH1 PACKED (PRE sets PRIM and empties the queue; A+D to ALPHA_2; XYZ2 without ADC draws a TRIANGLE's third, a TRISTRIP's last three, a TRIFAN's first and last two). `ShadowTri` (piney-draw) carries the vertices in sixteenths of the buffer and the 32-bit Z; `ShadowTri::pixels` is the repo's GS rule made exact: a pixel whose top-left corner lies inside, or on a top or left edge, Z the plane's truncated (integer edge functions, i128). The CPU GS counts with it, the GPU pass draws the same triangles.

## The port's shape

`piney_desktop::shadow` is now three modules over typed values, the old
glam `volume` (host floats, its own polygon clipper) gone: `draw.rs`
(`ccShadowModel::Draw` to a `Setup`: `Mode` an enum of the three the code
knows, the two `Blend`s by winding, the box check's result an enum
`Seen`), `vu1.rs` (the programs as functions of what they compute: the
directions' flags, `triangles`, `edges`, `clip`, `scissor`, `cut`, over
`V4` points and the shared `field::ee`/`vu0`/`vu1` ops), and `gif.rs`
(`Kick`: `Triangles`, `Quads`, `FanTag`, `Fan` of `Xyz2`s, with
`Kick::words` the GIF packet for the harness and `gs_triangles` the GS's
reading). What VU1 memory carries between draws is `Vu1Carry`: the last
fan's colour and the w words of 0x1ed-0x211 that an open edge reads. An
open edge's second face is `Option<u16>` in `piney_data::shadow::Edge`
(was `-1 - face`).

## The harness

`VolumeAgainstGame` keeps one VU0 and one VU1 for all draws, runs the box check in `tools/vu.py`'s VU0 (no more hook returning 1), fills the box with the game's `ccBbox_SetBox`, reads each kick when kicked, and compares with `shadow_volume_probe` (which carries `vf21` and VU1's memory the same way): the box words, whether the draw goes on and `vf21` after the check, every kicked word, then every pixel the two GS models count at depth 0 and at the draw's median Z. 300 random draws (one in eight now looking away from the model) and eight draws of `e4w1`: on Infection 308 draws, 10 culled, 444 clipped polygons, 246,116 words and 29,753,344 pixels the same; on each later disc 8 culled, 429 clipped, 29,851,648 pixels. The first run of the exact port matched every word on the first 20 draws once the box's else-if was in.

## The bone and skin packets

Run in eemu over every bone and skin mmat (`tools/test_ccs.py` `BodyPacketTest`, the `Mmat02` class), `Decode_Mmat02`'s packets are: bone, STCYCL 3,1 once, then per batch of up to 82 vertices BASE (0x118, 0x210, 0x308 in turn), OFFSET 0, the GIF tag (V4-32), STMASK 0x40404040 and the positions as V3-32 floats, the normals with the strip flag as V4-8, STMOD offset, STMASK 0x50505050 and the ST as V2-16, FLUSH, MSCAL `mc03m_DrawModel0` then `1`; skin, per batch of whole vertices, the entry words as V3-16 (weight * 16, the matrix address `20 + 4 slot`, the entries left of the vertex - a countdown, not just a count on the first), the positions V3-32, the normals V4-8 (each entry the vertex's flag; no vertex mixes them), then STCYCL 4,4 and every vertex's ST, the tag, MSCAL `mc03b`. The "halfwords" are the entry words; the positions are floats the EE made, `sceVu0ITOF4Vector` times `vertexScale / 256`. The ST goes as `(S & 0xfff) << 4` (0x0014af08), which the port did not have. `Model::body_words` and `model::body_st` are now the port's upload; `body_probe` writes them and the test compares every unpacked word: 2,994 bone and 185 skin mmats on Infection, 3,070 and 189 on each later disc, all equal. The old `s16 * scale / 4096` equalled `body_words` for all 821,259 bone lanes and no stored ST reaches 0x1000, so the rendered values do not change; piney-gs, the CPU GS and `scene::place` take the bone and skin words now.

## The later discs' harnesses

`test_vu` became `test_vu_rt`: a stripped executable's uploads are where its own code sends them (`ccSystem::RefrashDMA`'s REF tag, INF 0x0010aa00; `InitCCSys`'s one `sceDmaSend`), its entries what its lui/addiu pairs build (`Microcode.code_refs`, through `mips.track_lui`; on Infection equal to the relocations, all 22). The same 21 programs on all four.

Infection-pinned and outside the sweep were `test_sound`, `test_tables`, `test_mips`, `test_dwarf1` and `test_demangle`. The first three now follow PINEY_VOLUME as `test_sound_rt`, `test_tables_rt` and `test_mips_rt`:

- `test_sound_rt`: the banks, header tables and VAG loop flags per disc (Mutation and Outbreak 1,018 VAGs, Quarantine 1,011), the BGM (eight tracks from Mutation on), the voice files the code names, and str7100's PCM. That last one found the tool reading Outbreak's and Quarantine's streams wrongly: from Outbreak a Pcm chunk and an F_Pcm record carry a voice language (the port already had it, OUT 0x0014cab0, 0x0014d240); `sound.stream_pcm(data, lang)` now reads them. `sound.load_voices` names the files from each volume's own code (the Spc tables, `ccVoiceRequest`'s and `ccVoicePgFood`'s cases by the `voiceFile` each stores, `event_tables`) instead of Infection's table names; on Infection it gives the same 54 files and 8,208 lines as before.
- `test_tables_rt`: `tables.py` on a stripped executable takes Infection's DWARF types and the volume's placement (`volume.carried_rows`, moved out of `sound.Program.rows`).
- `test_mips_rt`: the overlay mapping at each volume's own base (Mutation's gcmn is not at 0x00400800), and the later executables carry no relocations.

`test_dwarf1` and `test_demangle` stay as they are: they test DWARF and C++ symbol names, which only Infection's executable has. The harnesses that already follow PINEY_VOLUME but whose names keep them out of the sweep (`test_anim`, `test_areas`, `test_battle`, `test_ccs`, `test_dungeon`, `test_event_vm`, `test_evscript`, `test_field`, `test_field_host`, `test_font`; `test_save` runs every volume itself) were left as they were then; the widened sweep (below) takes them in.

Found on the way: the Mutation-on `.syms` sidecars name the tables `ccVoiceRequest`'s BOSSTALK case loads `inu0VoiceTbl` and `inu0VoiceTblE` (Infection's `fidhellVoiceTbl`'s slot), while its INU case loads the real `inu0` tables elsewhere (MUT 0x006ce080, 0x006cf200). piney-gen reads those tables from the code, not by name, so the port is not affected.

## vf21 before the check, from the frame

The only EE code that writes `vf21` is `ccCoord::_SetLWMatrix` (0x00138380, a scan of every function); the only VU0 programs are the two box checks. So what a shadow's check reads is what its own object's `ccObj::Draw` (0x0013f220) did just before:

- `_SetLWMatrix`, when the object has a parent (+0x80). It pushes the chain on VU0's stack with `qmtc2 $a0` (pointer, its sign, `$a0`'s upper 64 bits) and leaves `vf21` the qword above the topmost changed coord (+0x8d): vf0 at the root, else that coord's parent. `ccAnm::SetAnm` hangs every entry from its parent slot's object or the anm, and every stream node hangs from a node or its clump's coordinate, so every object the port casts from has a parent.
- `ccModel::Draw` (0x0013eab0), when the model is drawn (above 1/128): a rigid model with mmats is checked against its box with `mc0_CheckBoundingBox`, which leaves `vf21` `world_screen * world` times the box's greatest corner whether it culls or not. Bone and skin models have no box.

Of the 3,191 shadow owners in `DATA.BIN`, 2,582 have an empty placeholder model and 609 a rigid one. The port had carried the last shadow check's `vf21`, wrong for both.

**What the console has there.** `ee_run` now leaves `vf21` as each box check does (the matrix times its last corner; it still says 1) and logs every shadow check (`EE_VF21`). Outbreak, the event-218 save, 7,200 frames, 68,933 shadow checks in three drawing threads: every stack qword read x a `ccCoord`'s address (0x00a9xxxx to 0x0159xxxx), y 0, and z, w `$a0`'s upper half. Per thread (each has its own registers), watching every instruction in the four images that writes `$a0`'s upper half: in the first (frames 1,035-4,395, the event's streams) none ran but `_SetLWMatrix`'s own `qmfc2 $a0, $vf20`, which hands it back unchanged, so it is the thread's first, 0; in the second (4,491-5,339) the last was `setNearest`'s `lq $a0` (OUT main 0x00152dec), the nearest hit's ground attribute at hitResult +0x48 (0x20004f8f, 0); the third drew only rigid models' owners. Run through `mc0_CheckBoundingBoxShadow` in `tools/vu.py`, all 65,566 stack-qword checks give the same result with x and z 0; of the 3,367 checks after a rigid model's own, 3,243 give another with `vf21` 0. By the EE adder (exact, then truncated) a tiny x or z moves a corner's lane by at most one step of its last bit, so the result can differ only on a corner within one step of a bound.

**The port.** `Vf21` is typed: `Known(V4)` (a check's last corner, or VU0's reset zero) or `Chain` (the stack's qword, whose heap and register bits the port reads as 0). `Prelude { parent, checked }` is the owner's `ccObj::Draw` up to its shadow; `Shadows::cast` takes it and applies it before the check, `ModelInfo::checked_max(transparency)` giving the rigid model's greatest corner when `ccModel::Draw` checks it. `cast_shadows` (piney-world's bodies, foes, TOBJ) and the stream's `cast_shadow` pass the object's transparency (the anm's times the node's).

**The box.** `Decode_Mmat01` keeps a rigid model's extremes with the same else-if as `DecodeShadowModel` (0x0014badc): a value below the least so far is not tried as the greatest. `vertex_box_centre` had taken plain min and max in host f32; 68 of Infection's 7,691 rigid models with mmats get another greatest corner (`title1`-`3`'s among them), so their sort centre was off too. `piney_data::model::VertexBox` is now the one box (`ccBbox_SetBox` over the decoders' extremes, `cvt.s.w` times `scale / 4096` in the EE's arithmetic): the shadow's `model_box`, the rigid model's checked corner and `ModelInfo::centre` all take it.

**The harness.** `OwnerAgainstGame` runs `ccObj::Draw` itself in eemu, on `eemu_rs.Vu0Machine` (VU0's integer registers and stack, which `_SetLWMatrix` needs): an object under a parent, its `_SetLWMatrix`, its model's `ccModel::Draw` up to the check (`ccSetMatrixPacket` refused, so nothing else is drawn), then the shadow, with `vf21` one register between the EE's macro code and the VU0 programs. Ninety draws a disc, a third each with the model seen, empty and faded: seen, `vf21` after the check the same word for word; otherwise the stack's qword (the parent's address, 0) read in; on all four discs every box, check and kicked word the same as the port's.

## VU1 0x209 before a pass

The shadow pass's GS steps are DIRECTHL only; no other VU1 program runs inside a pass. So 0x209 (and 0x20f, 0x210) holds what the frame's earlier VU1 work left. From the packets and the microcode: a rigid batch in buffer A (TOPS 0xec, OFFSET 0x18a) writes its output from TOPS + 0xc9, four qwords a vertex (`mc_DrawTri*`: the fog and ST, RGBAQ, XYZ2 with `isw.w` of the ADC word), so with 21 vertices or more vertex 20's XYZ2 lands at 0x209, w its ADC word; a bone batch at BASE 0x118 with 81 vertices or more puts vertex 80's position there (STCYCL 3,1, w the row's 1.0); a skin batch's data stops short of it; the scissor and effect programs have buffers of their own; a pass's own clip path writes it at 0x208's second qword. The port draws models on the GPU from its own geometry, not through VU1's memory, so it cannot know which batch ran last or how its triangles fell; it carries what the last shadow pass left. Only `e4w1`'s open edge on face 8 reads 0x209.

## charTbl and the voice tables

`ccSaveData::NewGame` (demo) walks `charTbl` with `slti 18` on Infection and `slti 21` from Mutation on (MUT 0x00176578, OUT 0x00175d20, QUA 0x00175bb0): piney-gen's 21 is right, `volume.carried_rows` (22 on Mutation and Outbreak, from the gap to the next sidecar symbol, which holds unnamed data) was wrong. It now takes the count of a table the code walks from that loop (`volume.WALKERS`, `loop_rows`: the `slti` before the backward branch whose loop body builds the table's address); `test_tables_rt` reads `charTbl` whole again on every disc. piney-gen and its output are unchanged.

From Mutation on `ccVoiceRequest`'s party, talk and present cases read Mia's English lines from part of a shared table: the row is the message plus an `addiu` after the `talkNum` getter, for the messages two `slti` bound (piney-gen's `voice_case` already reads them): messages 3-5 less 3, 2-3 plus 1 and 5-9 as they are, so rows 0-9. `sound._voice_stores` reads the same bounds. An event's rows are the ones its messages ask for (`evMsgTbl`, Parody Mode's `evMsgTblp`), which cuts Outbreak's English side event 266 and Quarantine's 361, one-row arrays in `.sdata` whose following words are other data. With that, `test_sound_rt` tiles every disc's voice files: 8,208 lines on Infection, 8,212 on Mutation, 8,726 on Outbreak, 9,421 on Quarantine, and names the stretches no reachable row reads: Mutation's event 101 message 2 and Outbreak's event 204 message 2 (their rows -1, the lines still on the disc, both languages), a line between Outbreak's and Quarantine's side events 262 and 263 in `EVVOL3S`, Quarantine's side event 355 row 25 past its 25 messages, and Quarantine's event 315 row 27, which plays event 314's line instead of the one at its place.

## The widened sweep's new harnesses

The sweep now runs every `tools/test_*.py` that names PINEY_VOLUME: 103 harnesses, 412 runs. The ten it newly takes in pass on Infection; on the later discs 23 runs failed, the same on a clean worktree of main 3238cb5, so none is this entry's. Most pinned Infection's content inside the class PINEY_VOLUME points at, while a `TestOtherVolumes` class beside it already runs each later disc's code. Each pin is now the disc's own, from the tool whose comparison with that disc's game code passes:

- `test_areas`: 127 events from Mutation on, the first area's enemies in each disc's own list order (Outbreak and Quarantine: Mad Grass, Disco Knife, Goblin), `volumeNum` and the substitute records for events 47 and 71.
- `test_battle`: Kite as `charTbl` starts him (level 1, 30, 50, 70: his stats, the rolls that hit a Goblin, the least and most damage); from Mutation on the Exdefense rule's 256 and 21 characters (the DWARF sizes are Infection's alone).
- `test_dungeon`: from Mutation on 90 `EditDungeon` entries, area 125's 15 floors, the flag at +0x5ec8.
- `test_evscript`: each disc's script count, switch tables and loop head, survey, first main event and its line 1 in both languages, 21 names (NewGame's count); a later sidecar names Infection's scripts with Infection's sizes and its new ones (19, 40, 56) not at all; the message arrays' symbols are Infection's alone (that one test is skipped elsewhere).
- `test_field`: `ccGetDist` calls newlib's `sqrtf` on Infection and Mutation and has `sqrt.s` inline from Outbreak on.
- `test_font`: each disc's corpus count and `ccKanjiStrlen`'s counted set, which `test_string_functions` had not passed to `str_len` (Outbreak's and Quarantine's count every `%x` pair); the named objects are Infection's alone (that one test is skipped elsewhere).

- `test_field_host`: its fixture is Infection's `ccGame::ChangeScene` and fades, which the port's `field_host` replays. On a later disc 55 scene lines differ, all in the server: from Mutation on `ChangeScene` stores the town as the server (MUT 0x0016756c, OUT 0x001667bc, QUA 0x0016664c) where Infection reads its table (0x00306dc0) for a town of 0 or more. The two agree on every town play gives (0-4: docs/engine/toppage.md), and the fixture's random towns run from -3 to 11; on a later disc the harness now checks that every other word is the fixture's and that a differing line's server is its town, outside 0-4.

One is not fixed here: `test_event_vm`'s `test_new_game_desktop` starts Infection's opening (`ccStartEvent(1, 0)`) and expects its mails 4, 5 and 320; a later disc's new game goes through the previous volume's save (`ccStartEventConvert`, `ConvGame`), which the harness does not model. It fails on main too.

## Checked

On the tip (f9cd2a0 for the sweep, which differs from the commit only in this entry's took and this list; d4ab42c, the same code before the harness pins, for the rest):

- `test_shadow_rs` OK on all four volumes: the decoder (1,957 distinct
  mmats on Infection, 1,964 / 1,965 / 1,965 later), `VolumeAgainstGame`
  (Infection 308 draws, 10 culled, 444 clipped, 246,116 words and
  29,753,344 pixels the same; later discs 8 culled, 429 clipped, 238,560 /
  238,776 / 236,888 words, 29,851,648 pixels) and `OwnerAgainstGame` (90
  draws each, 30 seen, 30 empty, 30 faded, the same). `test_sound_rt` and
  `test_tables_rt` OK on all four with no skip; `test_save` OK (9 tests,
  every volume).
- The sweep (103 harnesses, 412 runs) on f9cd2a0: all OK but
  `test_event_vm` on the three later discs (one test, below; the same on
  main) and `test_piros_rs` and `test_stream_rs`, which print no unittest
  line, as on main; the 356 runs main 3238cb5 had give its results, and
  before the pins 23 later-disc runs of the ten new harnesses failed on
  the tip and on a clean main worktree alike.
- Battle pins: 291 calls, 0 differ (probe wt-shadow at d4ab42c).
- Golden shots, the tip against the same-day base from main 3238cb5
  (`shadow-base`, equal to `world2/shots/base`): 3,493 of 4,550 differ, the
  same files as before this round. Against this entry's first tip
  (`shadow-tip2`): 30 differ, all `.frames` digests (44 frames' pictures,
  no event and no PNG): field shots 26 logs, dungeon shots 4. With the
  owner's Prelude left out (a patch, not committed) 23 logs (33 frames)
  come back to `shadow-tip2`'s; with the rigid box's old centre too
  (`vertex_box_centre`'s plain min and max in host f32), the other 7 (11
  frames: field type 9 block 2 and `d3` on the later discs) do, all 2,732
  field and dungeon files equal. So 33 frames are shadows taking the clip
  path or not by the owner's own check, 11 the sort centre of a rigid
  model.
- Crate tests 785 passed, 0 failed; the piney-game suite 349 / 0 / 153;
  clippy on all targets and features clean; fmt; `piney-gen gen`, `syms`,
  `carry --check` current; `cairns check`; `tools/docs.py check`.

**Still unknown:**
- The bits of `_SetLWMatrix`'s stack qword (a `ccCoord`'s heap address, the drawing thread's `$a0` upper half), which the port reads as 0: they can change a shadow's check only on a corner within one step of a bound, which none of 65,566 real checks in `ee_run` does, but the port has no heap addresses or EE registers to be sure of every frame.
- VU1 0x209 before a shadow pass, which `e4w1`'s open edge on face 8 reads: the last model batch to write it (a rigid batch in buffer A with 21 vertices or more: vertex 20's XYZ2 w, its ADC word; a bone batch at 0x118 with 81 or more: the row's 1.0; a scissor or effect buffer), which needs VU1's memory under every model batch of the frame; the port draws models from its own geometry and carries the last shadow pass's word.
- `test_event_vm`'s new game on the desktop on a later disc, through the previous volume's converted save (`ccStartEventConvert`, `ConvGame`), which the harness does not model.
- The Mutation-on `.syms` sidecars still name `fidhellVoiceTbl`'s tables `inu0VoiceTbl` and `inu0VoiceTblE` (piney-gen's carry); nothing reads them by those names.
