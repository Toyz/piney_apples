---
number: 494
title: The later towns' map: the fourth sprite is the game's, and Fort Ouph's and Lia Fail's DrawMap ported and checked against the game
date: 2026-10-25
area: ui
files: crates/piney-world/src/map/town.rs, crates/piney-world/examples/map_probe.rs, tools/test_map_rs.py, docs/engine/map.md
resolves: 291.12
started: 2026-10-25T20:20:36-06:00
took: 20m
---

# 494. The later towns' map: the fourth sprite is the game's, and Fort Ouph's and Lia Fail's DrawMap ported and checked against the game

Play question [[77]]/[[260]] (291.12): the later volumes' town maps draw
four sprites, but docs/engine/map.md said the port draws three, and only
Dun Loireag's and Carmina Gade's `DrawMap` had been compared. Fort
Ouph's and Lia Fail's had not.

**The fourth sprite is the game's.** Every later town's constructor
makes map[3] on a layer of priority 45 over the screen, with the town's
own texture (OUT gcmn `ROOTTOWN04` 0x00439c50, `ROOTTOWN05` 0x0043c5e0).
`DrawMap` draws the flag race's racers with it, and its send order is
map[2], map[3], map[0], map[1], as the port's is. Lia Fail's mask is
`ccMask(6, 6)` (0x0043caec), not 128 x 32. Its two layers sit at
+0x414 and +0x418.

**What Fort Ouph's and Lia Fail's `DrawMap` change** (OUT 0x0043b0d0,
0x0043d550; the same on MUT and QUA). The port drew both towns through
Carmina Gade's path less its offsets. tools/test_map_rs.py found them
off from the first frame. Read from the code:
- Fort Ouph moves the player's y by 16 (0x0043b254); Lia Fail does not
  move it.
- The arrow sits at (70 + x, (80 + y) - 14) in Fort Ouph, and at
  (87 + x, (92 + y) - 14) in Lia Fail. Its point in screen pixels is
  that base + (340 + x) across and ((base + 36) + y) - 14 down.
- Both towns dim a sign whose balloon (bx, by) has the point inside
  bx + 8..bx + 46 and by + 10..by + 45. Dun Loireag's box is
  bx..bx + 54 and by + 10..by + 49.
- The racers: Fort Ouph's are drawn 12 below where Dun Loireag's are
  (y + 12, not y - 12). Lia Fail's are 12 further right and at y.
  Neither town moves its racers by the map's offset.
- While the race runs, Fort Ouph's signs keep their fade (the increment
  is under the race check, 0x0043be90). Lia Fail's still fade in.
- A Lia Fail sign shows only while its row is inside the map's 160 rows
  (`top < y < top + 160`, 0x0043dc3c); otherwise it fades out.
- In both towns the balloon's cell and map[0]'s centre (-27, -29) are
  set only when no race is running.

**Checked.** tools/test_map_rs.py `test_fort_ouph`, `test_lia_fail`, and
the flag race on all four later towns. Each is 95 frames of the real
`DrawMap` over masks the real `ccMask` builds, compared packet by packet,
together with the `ICONPOS` table and the vertices:
- Fort Ouph: 1,356 packets (38 dimmed), and 836 with 128 racers;
- Lia Fail: 1,358 (34 dimmed), and 789 with 65 racers;
- no packet differs, on Mutation, Outbreak and Quarantine.
Infection's whole map harness still passes. The field and dungeon map
tests error on Mutation and Quarantine as they did before; they are
Infection harnesses.

Docs: map.md (the later towns' section, the checks, the stale
"not ported" line removed), UNKNOWNS.md.

**Still unknown:** nothing
