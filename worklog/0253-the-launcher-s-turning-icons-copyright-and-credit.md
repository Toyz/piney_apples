---
number: 253
title: The launcher's turning icons, copyright and credit
date: 2026-09-27
area: ui
files: crates/piney-game/src/launcher.rs, crates/piney-game/src/main.rs, crates/piney-stream/src/scene.rs
---

# 253. The launcher's turning icons, copyright and credit

Asked for from play: the launcher's icons should turn as the title's
menus do, the title's copyright line should be under the rows, and a
"ported by" credit should go on the right.

## The icons

The title's menu icons are `ANM_xdt_ic00`-`ic03` in its `title1`-`4`
file, each a 121-frame loop of two objects. The outer `OBJ_xdt_ico_X0_`
stands still. The inner `OBJ_xdt_ico_X1_` makes one turn about its y axis
every 120 frames at 60 a second: the pose at frame 30 is a quarter turn,
and frame 120 is back at the start.

The selector's rows use the same object names (`OBJ_xdt_ico_00_` to
`_32_`; rows 2 and 3 also have an `X2`). Its scene holds its last frame,
so they stood still. Once the rows answer, the launcher turns each row's
inner icon (`X1`) one turn every 60 of its frames, which is two seconds at
its two vblanks a frame, as the title does. `Scene::turn_y(node, ry)`
places a node as a child of itself with `sceVu0RotMatrixY(ry)` would
stand (`ee_mul(world, RotY)`), and its children turn with it.

## The copyright

The title draws its copyright as part of `m_back[2]`, `ANM_xdt_ne09`:
"the digits and copyright behind". The line itself is the object
`OBJ_xdt_cop_00_`, at (-19700, -17000, 1700), scaled 0.85. The selector's
scene has no such object.

Main now hands the launcher its disc's `DATA.BIN` and volume. The
launcher reads that volume's title file (Outbreak's `title3` from a full
build, whose line reads 2001-2003) and sets up `ANM_xdt_ne09` with every
object but the copyright switched off (`set_disp`). It draws that with the
demo's `draw_anm` through the title's camera, `ANM_xdtcam00`. The GS finds
`title3`'s textures in the disc's `DATA.BIN`, behind the selector's
`STRT.BIN`.

## The credit

`PORTED_BY` ("Ported by helba") is drawn in the game's font (`Kanji` type
2), white, flush right 24 in from the screen's edge, on the copyright's
second line. Its uploads are numbered after the frame's own, as the quit
prompt's are.

Headless at frame 1420 of a full build, the rows are at rest with the
chosen row lit and the icons mid-turn, the copyright's two lines are
bottom left and the credit bottom right.

**Still unknown:** Whether the unused selector drew a copyright of its
own on the demo discs. No code for it survives.
