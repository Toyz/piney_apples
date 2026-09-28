---
number: 183
title: "The desktop's and title's Controller page: the labelled boxes round the pad"
date: 2026-09-26
area: ui
files: crates/piney-desktop/src/dtmenu.rs, crates/piney-desktop/src/sprite.rs, tools/test_desktop_menu_rs.py, docs/engine/desktop.md, docs/engine/title.md
---

# 183. The desktop's and title's Controller page: the labelled boxes round the pad

`title.md` and `desktop.md` both listed the Controller page's picture boxes
as not ported: under OPTION > Controller on the desktop and on the title
screen, the port drew the scheme list and the pad but none of the labels.
The field's own Controller page had them (worklog 0148's option pages).

**The game's page.** `ccDtMenu::ControllerMenuDisp` (main 0x0016b7f0,
5,624 bytes) was run in eemu through `tools/test_desktop_menu_rs.py`'s
harness and its packets dumped for an open page. They are the field
page's (`ControllerMenuDisp`, gcmn 0x0053d550) cell for cell: the same
positions, cells, colours and `ctrl` bits.

- The buttons' box at (320, 128), 9 x 4, in colour 0.
- START and SELECT at (179, 300), with their two shapes (grid cells at
  (1792, 3776) 39 x 12 and (1024, 3776) 48 x 12).
- The left stick at (39, 300), in colour 0.
- The right stick at (347, 300) and the shoulder buttons at (165, 56),
  both in colour 7, since nothing sets the window's colour back to 0.
- The labels in settingKanji 0-2: `ctrlMenuStrBtn`, `ctrlMenuStrMov`,
  `ctrlMenuStrCam`, the camera's by `camType`.
- The rotation arrows: mirrored with ctrl 0x20 on the left, upside down
  with 0x40 for A-2 and B-2.

**The port.** `DtMenu::draw_controller` now pushes the same cells and
labels into the menu's draw:

- The menu's `Cell` has a `ctrl` for the two flips.
- piney-desktop's `Sprite` gained `flip_v`: the V rows swapped, as
  `MakePacketStr` does for 0x40.
- The grids 3 (24 x 24 at (2048, 1024)), START and SELECT were added.
- The labels' texts are read with the rest of the option texts.
- The page keeps `camType` as it opens it and as its OK sets it.

**Checks.** The harness had skipped the Controller's packets. Now it
compares them, and the setting kanji's packets on every page:

- `test_each_option` and `test_title_each_option` pass, frame by frame,
  over a scheme change;
- a sample frame has 88 window cells and the set0-set2 labels on both
  sides.

A shot of the desktop's page shows the five boxes and their labels round
the pad.

**Still unknown:** Nothing new.
