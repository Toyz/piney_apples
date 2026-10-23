---
number: 436
title: Gorre's m_exit is stream 64's mesh word where the manager lands in the cutscene's hole: the title wait, the weapons and the party move it, and both outcomes come from ordinary play
date: 2026-10-23
area: engine, tooling, battle, test
files: crates/piney-eemu/src/heap.rs, crates/piney-eemu/src/sys.rs, crates/piney-eemu/src/full.rs, crates/piney-eemu/examples/ee_run.rs, crates/piney-world/src/combat/boss.rs, crates/piney-game/src/session/tests/blur.rs, docs/engine/heap.md, docs/engine/boss.md
resolves: 432
started: 2026-10-22T23:46:11-06:00
took: 3h 28m
---

# 436. Gorre's m_exit is stream 64's mesh word where the manager lands in the cutscene's hole: the title wait, the weapons and the party move it, and both outcomes come from ordinary play

Worklog 432 left Gorre's `m_exit` to "the play since power-on" and took a
word that is not 0. This entry runs that play: the whole game from
`_start`, with every heap handout recorded, up to Gorre's first frame.

## Running the whole program

`piney_eemu::sys` runs an executable under a stand-in EE kernel:
threads with priorities and preemption, semaphores, interrupt and DMA
handlers, alarms, `SetSyscall`, the interrupt-enable bit. `full.rs` adds
the instructions the harnesses never needed (every MMI, the second
pipeline's multiply and divide, the SA register). `examples/ee_run.rs`
boots Outbreak on it and answers the IOP on the game's own library calls:
the disc from the ISO, `sceFs` files, the sound loads, idle pads, a
formatted card. The logo movies end at once and `vcallms`'s bounding-box
test says 1. It runs about 145 million instructions a second; a run from
power-on to the fight is about 9.5 billion, a minute and a half.

`EE_SAVE`, `EE_GO` and `EE_THEN` steer it: the port's event-218 save
written into `saveData` at a chosen frame of the title, the game sent to
town, BlackRose invited through `ccThPartyAdd`, the warp to "Chatting
Snaring Twins", floors 0 to 3, then field 5, where event 218 plays
stream 64 and starts the fight.

## The allocator

`ccHeap` (main 0x00100390-0x00100cf0) is a list of blocks in address
order, each with a 16-byte header, and the holes between them on 256
small lists (one per 16-byte size) and one large list in address order.
`SearchFree` takes the first hole of the smallest non-empty small list,
else the lowest large hole that fits; `AddTbl`'s small insert looks only
at the list's two ends. `ccMallocB` hands out from the top. A free merges
the room between the neighbours; a top block that is not the lowest
leaves its room off the lists. `ccMemReduce` (0x00100b60) cuts a block in
place, which the first replay missed until it was traced: the stream
decoder cuts its animation buffers.

`piney_eemu::heap::Heap` is that allocator. Run beside the game
(`EE_HEAP_MODEL`) it placed all 213,549 handouts of one run and all
217,046 of another where the game did, from the first `new` to the
manager.

## The hole

The manager (`operator new(0x1038)`, 0x1050 bytes) goes to the lowest
large hole that fits. In every history run (more than 80) that is the
0x33bc0-byte block `Decode_Mmat02` took for a skinned mesh of stream 64,
the event's cutscene, freed as it ends. About 11,000 handouts and frees
follow before the manager: the stream's teardown and the field's own
files go elsewhere; six kanji buffers, the members' weapons, models,
task stacks and `ccSPC::Reboot`s, the camera's, AI's and entry
controller's tasks, and `ccBossEntryStart`'s stack fill the hole from its
start. The manager lands where they end, and its +0x2c reads the mesh's
word there.

## Where it lands

Not on a fixed place. With the same save, party and route:

- The frame the save was loaded on at the title moves it (ten waits from
  960 to 1,500 frames: offsets 0x183ac to 0x18dcc).
- The weapons move it (every model and texture; loaded at frame 240,
  Kite's and BlackRose's 31 models spread it over 0xfc0).
- A third member moves it by 0x6000 or more.
- The route does not: another town first, a later invite, a floor walked
  twice give the same offset block for block. Each scene change frees
  all it handed out; two pairs of small blocks town leaves only swap.

Of the mesh's 13,244 words a hole can put there, 465 are 0. Loaded after
1,440 frames with Kite holding San and BlackRose Life Sword (both sold in
town), the manager lands at 0x181ac, on a 0: `m_exit` 0, and the blur
shows through the whole fight. The same weapons at 1,380 or 1,500
frames, or the event's own weapons at 1,440, give words that are not 0.
Steel Blades with Laevateinn at 1,200 frames is another 0. Every other
run (more than 70) gave a word that is not 0.

So the game has both outcomes, chosen by details no player sees. The
port has no game heap to follow. It keeps the word of the 900-frame
history, 0x0100003f (`HEAP_EXIT`), no blur as for most players, now on
`Combat::heap_exit`, where a 0 plays the other outcome as the game
does.

## Checks

- `heap::tests`: first fit, the small lists' order, the top, alignment,
  the cut.
- `session::tests::blur::every_fight_shows_its_blur`: Gorre's fight on
  `HEAP_EXIT` draws no blur.
- `gorre_on_a_zero_word_shows_the_managers_own_blur`: on 0 the reflex is
  there every frame, 0x40808080, turn 0, the scale falling from 1.01 to
  1.0 and held.
- `tools/test_eemu_rs.py`: the harness machines unchanged (the new
  instructions are behind `Features::full`, off for them).
- docs/engine/heap.md has the allocator, the sequence and the histories.

**Still unknown:** nothing
