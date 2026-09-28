---
number: 186
title: "The loading display between areas: the town's and the area's cards"
date: 2026-09-26
area: ui
files: crates/piney-game/src/loaddisp.rs, crates/piney-game/src/session.rs, docs/engine/field-walk.md, GAPS.md
---

# 186. The loading display between areas: the town's and the area's cards

**How it was found.** After the scene sounds (0184), which neither the
port nor any page knew, an audit listed every Infection function that no
crate names and no page mentions (by name or address), with the named
callers of each:

- 1,716 functions in all.
- Most are the SDK and destructors, or the later volumes' bosses
  (Kyvia, `ccBoss02`-`08`).
- The Ryu Books' viewer (`ccThBook`) needs items no script of any volume
  gives; it went to GAPS' "After Infection".
- One was a gap every player sees: `ccLoadDisp`, the loading display.

**What the game shows.** `ccSetupGameCtrl` loads a scene's files through
`ccLoadResourceFL` and `ccFileListLoad`. The latter calls `ccLoadDispInit`
when the new file list has a file the old lacked (`loadCheck` compares
`sceneFileList` with `dbgList`), and no display is up.

By `game.area` and `areaPrev`:

- **A town:** the town's card, "Aqua Capital" and "Mac Anu".
- **A field from a town:** the area's card, its three keywords
  ("Bursting Passed Over Aqua Field"), unless a stream plays (the gate
  hack's 107) or it is field 8.
- **A type 4 field's first dungeon from a town:** the area's card too.
- **A field from its dungeon, or any dungeon:** the THE WORLD animation
  alone.

Both cards carry `xdl_tit`'s blue banner in two cells, the texts in
kanji type 2, and the server's symbol with "Server" at the top right in
the server's colour.

The task (`ccLoadDispTh`, priority 17) draws until `ccSnd.gameStart`:

- the card, fading in by 8;
- `xdl_load`'s `ANM_xdl_lod1`, the THE WORLD logo at the bottom right,
  under the animation's own camera.

Then, with a card, 30 frames at full and 17 fading out, without the
animation, over the scene's own fade in.

The code read: `ccLoadDispInit`, `setupInit`, `townSetup`, `fieldSetup`,
`serverSetup`, `allDisp`, `townDisp`, `fieldDisp`, `serverDisp`,
`ccLoadDispTh`, `loadCheck` (field-walk.md, "The loading display").

**The port.** `piney-game`'s `loaddisp`:

- `kind` is `ccLoadDispInit`'s choice.
- `LoadDisp::new` holds the set-ups. The texts come from main's
  `townNameTbl`, `serverNameTbl`, "Server" and the separator, and the
  keywords from `AreaTables`.
- `frame` is the task's timeline; `draw` draws over the frame, with the
  upload ids after the frame's.
- The session puts it up in `world_stage`, so every change of scene gets
  it, and so does the log-in from the top page (`gameStart` is watched in
  the events).
- The order matters: `ccKanji::Disp` sends at once and the banner's
  `SendPacketS` after, so the port's prepending layers draw the banner
  under the texts. A first shot had the name hidden under it.

The port's loads take no time, so a card shows a few frames of fading in
over black, the THE WORLD logo, then its 47 frames at the new scene's
start. Shots:

- `--mode field:14`: "Bursting Passed Over Aqua Field", "Δ Server", the
  logo while loading.
- `--mode world`: "Aqua Capital / Mac Anu".

Checks: `which_card` (the init's cases) and `its_frames` (the fade in, 30
held, 17 falling to 0; the plain display ending at the start).

**Still unknown:** How long the game's loads take, so how much of the
fade in and the logo a player sees: not measured. The request's +0x18,
which also stops the display, was not traced to its setter.
