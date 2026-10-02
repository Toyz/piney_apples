---
number: 345
title: "Three player reports: a node keeps its own transparency, the book's cover every frame, one area a gate trip"
date: 2026-10-01
area: render, ui, save
files: crates/piney-world/src/body.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/tests/fairy_orb.rs, crates/piney-game/src/session/tests/dun_loireag.rs
supersedes: 319
---

# 345. Three player reports: a node keeps its own transparency, the book's cover every frame, one area a gate trip

Three issues from the same player, all INF.

## #20: the Gott statue vanished once its box was looted

The screenshot shows a dungeon's B3 with the statue gone after "You now have
Rainbow Card!". [[319]] made `Body::draw` multiply each node's animated
transparency by its parents', as it read `ccCoord::_GetTransparency` (main
0x00138490). That reading was wrong.

`_GetTransparency` walks to a node's parent (`+0x80`) only while the node's
flag bit 0 (`+0x8c`) is set. `SetAnmCtrlWork` (0x001507c8) writes the
animated value to the node's local (`+0x88`). With bit 0 set it marks the
node for that walk (bit 2); without it, it writes the value straight to the
world transparency (`+0x84`). Bit 0 comes from the object's Obj2 chunk
(`flags`, "transparency inherits the parent's"): `Decode_Obj` (0x0014cb30)
copies it to `ccObjChunk+0xc`, and `ccClump::Init` (0x0013c718) tests it. A
node with no Obj2 chunk has flags 0. Every Obj2 chunk in the four volumes'
`DATA.BIN` was read: about 219,000 to 221,000 a volume, and none has bit 0.
So a node's transparency is its own animated value and nothing else.
`ccClump::Init` does put the root under the clump itself (0x0013c944), but
with bit 0 clear that link is never walked for transparency.

The seven statues (`xgs?bod1`) show what the product broke. In the d, f, l
and w statues, the fall `ANM_xgs?dwn0` and `ANM_xgs?nut1` key the root
`OBJ_trall` to 0 from frame 0. Multiplied down, that hid the whole statue
the moment it began to fall. In a, e and t the root stays at 1, which is why
[[319]]'s tutorial statue (a) looked right. `Body::node_alphas` now returns
each node's own value. `a_fallen_gott_statue_stays_drawn` poses all seven at
the fall's last frame and at `nut1`: `OBJ_o_god_m0_` at 1, the ring at 0.
The enemies' models share the function.

## #21: the Ryu Book's cover strobed

The player's video (`work/video_of_bugs`, not in the repository) shows the
cover stream every other frame, with black between. Worklog 336 moved the
town's menu task after the event task. The menu task steps the book's
stream (`ccThBook`'s `ccThExecuteStream`) and leaves its frame in
`st.stream.frame`. But the step reads that slot only before the menu task
runs, so the frame showed one game frame late. On that later frame the step
returned at once, skipped the menu task, and so stepped no stream; the next
frame showed the town's own picture, black in the video. The stream thus advanced every
other frame. Half the cancel presses landed on frames that never stepped
it, which is the "sometimes the button doesn't register" of the report.
The town now shows the book's frame as soon as the menu task has made it.
`read_a_ryu_book` (behind `a_ryu_book_opens_in_town`) checks that the
cover's frame moves on every frame it plays. Without the fix, 360 frames
stood still.

## #22: "Number of areas visited" 141

The player's save (`work/savecards/repro/dhdata12`) holds 141 at +0x6862,
the counter Book I shows (`BOOK::Disp01`, gcmn 0x00410640). A scan of the
code for stores to +0x6862 finds five pairs in GCMN.PRG (a store and its
clamp to 10000, at file offsets 0x15d5ec, 0x15e9ec, 0x162708, 0x163770 and
0x16644c; the first is `GtRandomMenu`'s, gcmn 0x0055dde4). `MainProccess` copies it on load and
save, and `ccSaveData::Init` zeroes it. The port has the same five.

But a gate's leave state (Random's 5, New Keyword's, the lists' 7) stays put
once `leave` returns true. Every frame the menu runs after that, it counts
again and asks `GoToArea` again. In the game the menu task sleeps in the
`ChangeRequest` its `SetGenerateCode` makes. The port's session runs the
town for the 10 frames of the leave's fade, and the town ran its menu task
through them. `a_random_warp_counts_one_area` takes a new game's Random warp
from Mac Anu: 10 counted without the fix, 1 with it. The town now skips its
menu task once it has asked to leave (`leaving`). The 141 is about 14 trips.
A save already counted stays as it is.

**Still unknown:** whether any of the streams' files (`STREAM/*.BIN`) carry
an Obj2 chunk with bit 0 (only `DATA.BIN` was read); the stream player
draws through its own code.
