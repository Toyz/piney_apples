---
number: 455
title: Every boss fades by the camera's distance as ccBoss::Draw does, checked against the game in each boss's harness
date: 2026-10-24
area: battle
files: crates/piney-battle/src/boss.rs crates/piney-battle/src/boss crates/piney-battle/examples/battle_probe crates/piney-world/src/combat/boss.rs tools/test_battle_boss_rs.py docs/engine/boss.md
---

# 455. Every boss fades by the camera's distance as ccBoss::Draw does, checked against the game in each boss's harness

## Every boss's camera fade

Worklog 453 ported `ccBoss::Draw`'s transparency for Corbenik's third
form only. The question left: which other bosses draw through it, and
what their rules read back.

**What the code says.**
- `ccBoss::Draw` is the same in all four volumes (INF gcmn 0x0045d4b0,
  MUT 0x00472e50, OUT 0x0046e730, QUA 0x00361040): with `transDist` (+0x90,
  1 from `ccChar`'s constructor) and `checkCameraType() != 1`,
  `transparency = setTransparency * ccGetCameraTransparency(pos, base
  width, base height, far, fade, &hide)` (7000 and 600 in a field, 4000
  and 400 in a town), else `transparency = setTransparency`. It draws the
  clip at 0.05 or more, or when `hide`.
- Raw `jal` scans list the same callers in every volume: `ccBoss::Main`
  (Skeith, Magus, Fidchell, Macha, Tarvos, Corbenik's three forms, its
  eyes, Gorre's brothers, Innis's images) and the classes with their own
  `Main`: Innis, Magus's leaves, Gorre's Kerse only, the seeds, the second
  form, Kyvia's four bodies (unnamed functions on OUT and QUA), its core
  and gomoras. Innis, the core, the gomoras and the Kerse set both words
  to their alpha first, so the alpha is faded.
- The rest of `Draw` (the hit flash and condition tint by `SetFogBlend`,
  the draw environment's alpha, the clip) is drawing.

**The port.** `Boss::draw` (was `draw_fade`) at each of those places;
`Boss::drawn` keeps whether the clip was drawn. The world's actors take
`transparency` and `drawn` for the boss and its parts (they took
`setTransparency`). Innis's images took their own +0x29360, which their
code only ever sets to 1; nothing of theirs writes +0x88 or +0x8c, so
they draw at 1 faded like the rest.

**The harnesses.** `tools/test_battle_boss_rs.py` gains `draw_case`,
`install_draw`, `put_draw_camera` and `own_fog`: every boss harness runs
`Draw` natively against a stand-in `activeCamPtr` (in the arena, about
it, in the 7000-7800 shell, or past it; a random `deg[1]`; one case in
ten the eye view) and compares `transparency` for the boss and its parts
each frame (Skeith's third word; Innis's images and Kyvia's body, core
and gomoras added). The probes read the camera (`read_fade`); the boss's
base takes its row's height and width (`row_size`), which the request did
not carry. A first run found that gap: the probe's Skeith had height 0
and missed the near fade. Tarvos's and Corbenik's `SetFogBlend` recorders
now skip `Draw`'s own calls. The first form's probe follows the camera
type its rules write.

**Checked.** Bulk runs pass: Skeith 8 cases, Innis 8, Kyvia's fights
(MUT 6, OUT 4 and its first 3, QUA third 4 and fourth 4), Magus 6,
Fidchell 6, Gorre 6, Macha 6, Tarvos 6, Corbenik's forms 6, 6 and 3. The
fade changed transparency in 4,593 of 7,364 Skeith frames, 1,571 of
12,084 Innis, 9,957 of 17,955 Magus, 3,072 of 14,893 Fidchell, 1,647 of
14,893 Macha, 5,893 of 22,483 Tarvos, 1,069 of 21,632 and 4,732 of 20,889
for Corbenik's first two forms, and 22,325 of 127,386 Kyvia part frames.
Gorre's random cases never faded; two forced cameras (7360 out, and 512
near) pass. Unit test `the_camera_fades_the_body_as_ccboss_draw_does`.

**Still unknown:**
- Kyvia's fourth fight's body kept 1 in every case (its parts faded), so
  the body's own fade is checked through the first three fights only.
- The world's boss pictures with the fade are not compared with game
  frames (no shot).
