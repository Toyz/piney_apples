---
number: 352
title: "Mutation's recall fades are smooth: its boards are rigid and key on their boxes"
date: 2026-10-02
area: render, video, volumes
files: crates/piney-stream/src/draw.rs
supersedes: 226
resolves: 226
---

# 352. Mutation's recall fades are smooth: its boards are rigid and key on their boxes

[[226]] described the recall in MUT stream 23 (`str0695e`). Its pages
cross-fade through a black board (`MDL_blackbord_a01`) one unit in front
of them, or a white one for flashes. 226 took both as translucent models
without a box and keyed them with the w-row formula. That key put the
board first; the page behind it then failed the depth test, and every
fade came out as a cut to black. 226 left open whether the PS2 shows them
that way, since the page layout suggests smooth fades.

[[351]] found the game's rule. Every model without `mtype & 6` keys the
sorted group on its own vertex box (`ccModel::Init` 0x0013a550,
`ccBbox_SetBox`). Read from `STREAM/STR2E.BIN::str0695e.tmp`, every board
is rigid: `MDL_blackbord_a01`, `MDL_whitebord_a01` and all the
`MDL_bord_*` are mtype 0, flag 0. So in the game the page, which is
farther, is drawn first and the board over it.

The port now keys them that way (the change of 351). Stream 23 was shot
again (English, frames 1000-1144, every 8): the WORLD BOARD page fades
smoothly to black over about 32 frames (1096-1128). The fades 226 saw as
cuts are smooth, as the layout meant. `exact_key` stays, now only for bone
and skin models, and its comment no longer cites the recall.

**Still unknown:** nothing.
