---
number: 501
title: The launcher's icons turn as the title's: the whole icon by AllAnimate's m_Rot_ICO, not the letter alone
date: 2026-10-26
area: ui
files: crates/piney-game/src/launcher.rs, crates/piney-demo/src/opening.rs, BUGS.md
started: 2026-10-25T23:51:28-06:00
took: 10m
---

# 501. The launcher's icons turn as the title's: the whole icon by AllAnimate's m_Rot_ICO, not the letter alone

BUGS.md: the launcher's turning icons are close to the title menu's but
not the same. Worklog 253 found two differences. The title's turning
part is the crystal's shell about a still letter, while the selector
turned its letter, which read mirrored for half the turn. And the
selector steps at 30 a second.

**Which code decides.** The launcher plays the selector Outbreak's disc
left unused (`trial_v2st`). Its own code does not survive, so the
title's turning icons are the model.
- The title's `AllAnimate` (INF demo.prg 0x004047e0) takes `m_Rot_ICO.y`
  0.03 (0x3cf5c28f) lower each vblank, wrapped by 2 pi above pi and below
  -pi.
- It places each icon's whole anm with `SetMatrix_PosRotXYZ(dummy,
  m_Rot_ICO)`. So shell and letter turn together about y at the icon's
  dummy (docs/engine/title.md, "Every frame").
- The port's title does the same, checked bit-exact against the game.

**The fix.**
- The selector's icon root `OBJ_xdt_ico_X0_` is now turned by the
  title's `m_Rot_ICO`; its child `X1`, the letter, turns with it.
- The angle advances two vblanks per launcher frame, through
  `piney_demo::opening::spin_icons`. That is `AllAnimate`'s step,
  factored out, and the title's opening now calls it too.
- The launcher's own turn of `X1` alone (`ICON_TURN`) is gone.
- The selector stays at its stream's 30 a second, as every stream scene
  runs. The icons now turn at the title's speed.

**Checked.** `selector_icon_shots`: twelve shots across the turn show
each whole icon turning about its place, all four rows alike. The
launcher's tests, piney-demo's and piney-game's suite (336) pass.

Docs: BUGS.md.

**Still unknown:** nothing
