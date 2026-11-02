---
number: 549
title: Cubia's white flashes were its gomoras' after-images drawn as the whole body; Tartarga posed through his skin's own clump
date: 2026-11-01
area: battle, render, world
files: crates/piney-battle/src/boss/kyvia/gomora.rs, crates/piney-battle/src/boss/kyvia/core.rs, crates/piney-world/src/chara/body.rs, crates/piney-battle/examples/battle_probe/kyvia.rs, tools/test_battle_kyvia_rs.py, crates/piney-game/src/session/tests/tarvos.rs, crates/piney-game/src/session/tests/player_logs.rs
started: 2026-11-01T22:06:23-07:00
took: 1h
---

# 549. Cubia's white flashes were its gomoras' after-images drawn as the whole body; Tartarga posed through his skin's own clump

Issue #80 (QUA, build e70f6a8): heavy lag whenever Cubia took a big hit and
flashed white, in the fight before Tarvos (field 11, `ccBossKyvia03`); and
Tartarga invisible in Lia Fail as the NPCs leave.

## The lag and the white flash

The player's log replayed through the launcher with every frame of the fight
drawn headless (`player_log_replay`, `PINEY_RENDER_MS=12`): 2,510 of ~19,900
frames took over 12 ms to draw and read back, the worst 50 ms. Frame 15510
(`PINEY_BENCH`) drew 1,196 models, 2,507,631 vertices: every object of `x03`
38 times, 36 of them white (fog F 0, colour 0xffffff). The 38 were 19 live
after-images of the body (`AfterImage::boss`, fading 0.05 a draw) times the
two copies of each object. The simulation itself had no slow frame.

Their source: the core's `ThinkESCAPE` and a dashing gomora's `Think` call
`EntryAfterImage(&m_eai)` (QUA gcmn 0x003ea428, 0x003f0700) with `this` the
core or the gomora, so each makes an after-image of itself: its own clip at
its frame and place, in its `m_eai` (gomora: colour 0x007b7b31 at 30,
`maxFrame` 80, transparency 0.5 by -0.02, `InitData` MUT 0x004fe170, QUA
0x003ed59c; core: colour 0x00187500, the rest the same, its transparency set
to its alpha before each, QUA 0x003ea414). The port runs the parts inside
Kyvia's frame on one `Cx`, so their `Out::AfterImage` landed in the body's
outputs, and the field drew each as the whole Cubia in white. Five gomoras
dashing kept 19 alive.

They now go out as `Out::AfterImagePart` with the part, its frame and its
`m_eai`; the field already draws those with the part's image. Frame 15510
draws 161 models in 6.2 ms (was 50 ms), and the fight has no slow frame left
but its first. `a_dashing_gomora_leaves_its_own_after_images` (piney-battle)
holds it; `tools/test_battle_kyvia_rs.py` now records each `EntryAfterImage`'s
frame and words, and agrees on Mutation's first fight, Outbreak's second and
Quarantine's third and fourth (its `test_core_wave` fails on the last three
before and after: the forced case never waves). The battle pins agree but for
those words.

## Tartarga

Event 310 block 14 in Lia Fail (`lia_fail_after_tarvos`): after `trans_off 3
102` Tartarga (row 102, file `cts1`) was drawn, transparency 1, yet nothing
showed where the cameras frame him. His file has two clumps: `CMP_trall`
(`OBJ_trall` alone) and `CMP_t0` (49 nodes); his skin `MDL_cts1body` hangs
on `OBJ_cts1body`, the 49th node of `CMP_t0`. `ccRtownPC::ccRtownPC` (QUA gcmn
0x0040f95c) makes the PC from `CMP_trall`; the port posed every bone and skin
model through that clump's one node, so the whole body fell to his feet,
below the shot. `Body` now keeps, for a model on an object outside its clump,
the node table of the clump that lists that object (`model_clumps`); `cts2`
and `cts4` are the only other town files of that shape. Posed so, the body
stands where `camz_set 40 -466 3 77 -433 9` and the move after it frame him
as he speaks. `tartarga_is_drawn_whole_in_the_goodbyes` checks his skin gets
the 49 matrices and stays in the shot; it failed with one.

**Still unknown:** the game's code path that gives a model outside the made clump its own clump's node table: ccAnm::SetAnm (QUA main 0x001502bc) inits such an object with no coords, and ccModel::DrawBoneType reads the model's own table (+0x10, +0x14); the rule is inferred from event 310's cameras framing Tartarga whole
