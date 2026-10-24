---
number: 456
title: Kyvia's third and fourth bodies never fade: their constructors clear transDist, found by cameras forced into the fade band
date: 2026-10-24
area: battle
files: crates/piney-battle/src/boss.rs crates/piney-battle/src/boss/kyvia.rs tools/test_battle_boss_rs.py tools/test_battle_kyvia_rs.py tools/test_battle_gorre_rs.py docs/engine/boss.md
---

# 456. Kyvia's third and fourth bodies never fade: their constructors clear transDist, found by cameras forced into the fade band

## Kyvia's third and fourth bodies and the camera fade

Worklog 455 left Kyvia's fourth body unchecked: its transparency stayed 1
in every random case. The cameras are now forced into and around the
fade band against it.

**What the forced cameras showed.** `bb.FORCED_DRAW_CAMS` puts the
active camera 7360 out (half through the fade), 7680 and 2048 aside at a
pitch, 512 off (the near fade), 9216 out (gone), and the eye view at
9216. Against the fourth body, the game kept 1 with `drawSW` set in
every frame, and `ccBossKyvia04::Main` calls `ccBoss::Draw` (QUA gcmn
0x003dbc7c). The port faded it, and the first frame failed.

**Why.** A scan for word writes to +0x90 in every volume's boss code
finds only `ccBoss`'s constructor setting it and two clearing it:
`ccBossKyvia03`'s and `ccBossKyvia04`'s constructors (OUT gcmn
0x004de640, 0x004e7708; QUA 0x003d0f50, 0x003da018; INF and MUT have the
same pair). Without `transDist`, `Draw` leaves
`transparency = setTransparency`. The third and fourth bodies never
fade; their core and gomoras do. The other +0x90 writes are on the stack
or on other objects (after-images, the camera, missiles).

**The port.** `Boss::trans_dist_off`, set by `kyvia::new` for the third
and fourth fights; `Boss::draw` skips the fade with it.

**Harness cases.** `test_forced_cameras` in the Kyvia harness (each
volume's fights) and in Gorre's runs each forced camera
(`bb.forced_draw_cases`). They pass on Quarantine's third and fourth
fights, Outbreak's second and Mutation's first. Gorre's brothers fade
in them. The Kyvia bulk runs pass again on Quarantine's third and fourth
fights.

**Still unknown:**
- Nothing left on the fade; the draw's other inputs (the condition tint,
  the hit flash) are drawing only and not compared.
