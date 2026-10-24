---
title: The heap - ccHeap and what it leaves behind
status: solid
volumes: OUT, QUA
covers: OUT SLUS_205.63:0x00100390 ccHeap::InitFirst, 0x00100500 ccHeapFree::SearchFree, 0x001005b0 ccHeapFree::AddTbl, 0x001006e0 _ccMalloc, 0x001009a0 _ccFree, 0x00100b60 ccMemReduce, 0x00100c90 ccMalloc, 0x00100cb0 ccMallocB, 0x00100cd0 ccMemalign, 0x00100ce0 ccMemalignB, 0x00100cf0 ccFree, 0x00386b80 heapTop, 0x00386b84 heapEnd, 0x00386b8c useEnd, 0x00386b90 useEndB, 0x003872d0 freeTbl; OUT gcmn.prg:0x00472e90 ccThBossEffect (its operator new); QUA gcmn.prg:0x003657a0 ccThBossEffect, 0x003aca30 ccThBoss08, 0x003acbd0 ccBoss08::ccBoss08, 0x003b2270 ccBoss08_b::ccBoss08_b's blur write, 0x003b9640 ccBoss08_c::ccBoss08_c's blur write; QUA SLUS_205.64:0x0013ab68 ccClump::Init's new(0xb0), 0x00104200 ccBltData::Init
worklog: 432, 436, 439
---

# The heap - ccHeap and what it leaves behind

Every `new`, `new[]` and `ccMalloc` in the game comes out of one region
that `ccHeap::InitFirst` sets up once at boot. Nothing clears a block
when it is handed out, and nothing resets the region afterwards, so a
field the constructor does not write holds what the bytes' last owner
left there. This page is the allocator, exactly enough to place every
block where the game does, and the one place that matters to the port:
Gorre's blur ([boss.md](boss.md#the-blur)).

`piney_eemu::heap::Heap` is the model. `ee_run` with `EE_HEAP` and
`EE_HEAP_MODEL` runs it beside the game: from power-on through event 218
to Gorre's first frame, all 217,046 handouts of the run land where the
game puts them.

## The region

`InitFirst` (0x00100390) takes `malloc(0x1880870)` from the C library,
rounds it to 16 and writes two 16-byte end blocks: `heapTop` at its start
(0x00773790 on every boot) and `heapEnd` 16 below its end (0x01ff3ff0).
The room between is one hole.

Every block has a 16-byte header: +0 the block below, +4 the block above,
+8 its size. The blocks form one list in address order; a hole is the
room between two of them. A hole of 32 bytes or more is on a free list,
its first 16 bytes its record: +0 / +4 the list's links, +8 the block
below it, +12 its size. Smaller ones are on no list until a neighbour is
freed.

`freeTbl` (0x003872d0) is 256 small lists, one for each size under 4096
in steps of 16, then the large list at +0x800, kept in address order.
`AddTbl` puts a small hole at the front of its list when it lies at or
below the first one, after the last when it lies above that, and at the
front otherwise: it never looks past the two ends.

## Handing out

`_ccMalloc(align, n, _, flags)` (0x001006e0); `ccMalloc` and
`operator new` pass `(0, n, 0, 0)`, `ccMallocB` flags 2, `ccMemalign`
an alignment.

- The size is `(align + 31 + n) & ~15`, the header included.
- `SearchFree` (0x00100500): under 4096, the first hole of the first
  non-empty small list from the size's own upward; otherwise, or when they
  are all empty, the lowest hole on the large list that is big enough.
- The block starts at the hole's start (moved up to the alignment when
  one is asked). The rest of the hole, 32 bytes or more, goes back on a
  list.
- Flags bit 1 (`ccMallocB`): from `heapEnd` down, the first gap between
  blocks that fits; the block ends at the gap's top. `useEndB` is the
  lowest such block, `useEnd` the end of the highest from the bottom; a
  top block may not go below `useEnd`.
- Nothing fits: with flags bit 0 the call returns 0, else it writes to
  address 0.

## Freeing and cutting

`_ccFree` (0x001009a0) takes the block out of the list. Below `useEnd`
the holes on either side come off their lists and the whole room between
the neighbours goes back as one hole. A top block frees its room only
when it is the lowest (`useEndB`), which then becomes the block above.

`ccMemReduce(p, n)` (0x00100b60) cuts a block to `(n + 31) & ~15`; the
cut-off tail joins the hole above it. The stream decoder cuts its
animation buffers this way (`Decode_Anime`).

## Gorre's m_exit

Gorre's fight starts in event 218 (the twins' area, field 5): block 11's
`stream 64` plays first, about 1,700 frames, then the field loads and the
fight starts. `ccBossEntryStart` (gcmn 0x0046c540) starts the effect task,
`ccThBossEffect` (0x00472e90), whose `operator new(0x1038)` is the boss
effect manager: 0x1050 bytes, a large request, so the lowest hole of
0x1050 or more. Its constructor never writes +0x2c, the blur's `m_exit`.

In every run below (more than 80 histories) that hole is the same one:
the 0x33bc0-byte block `Decode_Mmat02` asked for (0x33bb0) while decoding
stream 64, a skinned model's mesh, freed with the stream as the cutscene
ends. Between the free and the manager come about 11,000 handouts and
frees: the stream torn down, the field's files ungzipped and decoded, the
menus. Most go elsewhere. What lands in the cutscene's hole, in order,
for the save loaded after 900 frames at the title (offsets from the
block's start):

| offset | what | from |
| --- | --- | --- |
| 0x00660 | six kanji buffers, 0xc060 | `ccKanji::Init` (the field's menus) |
| 0x0c6c0 | two weapons, a task stack, `ccSPC::Reboot`, a `ccFellow` | a member's entry |
| 0x0eef0 | 40 clumps, objects and `new[]`s, then animation work | the members' models |
| 0x13890 | a weapon, a thread, a 4096 stack, `Reboot` | the last member |
| 0x15740 | the camera's, the AI system's and the entry controller's tasks | the fight's start |
| 0x173e0 | `ccStartThread`'s 4096 stack | `ccBossEntryStart` |
| 0x187f0 | the manager | `ccThBossEffect` |

The manager's +0x2c is the block's +0x1882c, so `m_exit` is the word
`Decode_Mmat02` wrote 0x1882c into its buffer: 0x0100003f.

### It moves with the play

Where the manager lands follows from every handout before it, because
each decides which holes are free when the field loads. The save here is
the port's at event 218 (Kite with Bloody Blades, BlackRose with
Horse-Killer); `ee_run` stands in for the title's Load and the desktop
(`EE_GO`), then invites BlackRose, warps and walks floors 0 to 3 down to
field 5:

| at the title | Kite | BlackRose | offset | `m_exit` | blur |
| --- | --- | --- | --- | --- | --- |
| 900 frames | Bloody Blades | Horse-Killer | 0x1882c | 0x0100003f | no |
| 960 to 1500, each 60 | Bloody Blades | Horse-Killer | 0x183ac to 0x18dcc | ten words, none 0 | no |
| 1440 frames | Bloody Blades | Horse-Killer | 0x185ec | 0xbe800000 | no |
| 1440 frames | San | Life Sword | 0x181ac | 0 | yes |
| 1380 frames | San | Life Sword | 0x18acc | 0x000100c4 | no |
| 1500 frames | San | Life Sword | 0x1838c | 0x01df0036 | no |
| 1200 frames | Steel Blades | Laevateinn | 0x1826c | 0 | yes |
| 240 frames | Bloody Blades | Horse-Killer | 0x0743c | 0x40580000 | no |

San and Life Sword are sold in the towns (`equipShopItemList_05`,
`_03`). At 240 frames the title's set-up task still holds its buffers,
which stay. Thirty-one more runs with other weapons and waits, and about
forty with the save loaded at frame 240, gave other offsets, none 0.

- **The weapons.** Each model and each texture of one moves it; with the
  save loaded at frame 240, Kite's and BlackRose's 31 models spread it
  over 0xfc0.
- **The party.** A third member moves it by 0x6000 or more.
- **Not the way to the field.** Another town first, BlackRose invited
  later, a floor walked twice: the same offset, block for block. Each
  scene change frees what it handed out; the few small blocks town
  leaves (below 0x00a94000) only trade places.

The word under it is stream 64's own data, the same in every run: of the
13,244 words a hole can put there, 465 are 0. So the blur shows for the
histories whose manager lands on one of them, and for no others. Both
come from ordinary play; which one a player gets is set by things as
small as the frame the save was loaded on.

The port has no game heap. It takes the 900-frame history's word,
0x0100003f (`piney_world::combat::boss::HEAP_EXIT`), no blur as for most
histories, and keeps it on `Combat::heap_exit`: a 0 there plays the other
outcome.

## ccBoss08's m_exit

Quarantine's last fight, `ccBoss08` (`bossFunc` code 7), starts in
event 313:

- Block 14 runs in town 4 and ends with `area 8` and `scene 1 4 8`.
- In field 8, block 15's `entry type=7 code=7` starts the fight's tasks,
  and block 16's `stream 105` (1,100 frames) plays before the first form.

`ccThBoss08` (QUA gcmn 0x003aca30) makes the first form, `ccBoss08`. It
runs it to its exit, then `ccThBoss08b_Main`; with `saveData` +0x652f
set it goes straight to `ccThBoss08c_Main`. The effect manager is the
same `operator new(0x1038)` (QUA gcmn 0x003657b4).

A register-tracked scan of every function of the eight classes found
the blur written only by `ccBoss08_b`'s constructor, its
`OnThinkBodyPress` and `OnThinkRootDrain`, and `ccBoss08_c`'s
constructor. The first form's classes (`ccBoss08`, its `ccBoss08Seed`s)
and `ccThBoss08` never touch it. So through the first form `m_exit` is the
word under the manager.

`ee_run` runs Quarantine with the port's save before event 313 (blocks
0-14 played, `eventStatus[0]` 1). The heap model placed every handout
where the game did (56,537 in the first run).

| history | the manager | under its +0x2c | `m_exit` | blur |
| --- | --- | --- | --- | --- |
| town 4, BlackRose and member 2, save at 900 frames | 0x01465c30 | a town-4 clump object's +0x2c (`ccClump::Init`'s `new(0xb0)`) | 0 | yes |
| town 4, BlackRose alone, 900 frames | 0x013a7890 | a 0x10000 `ccBltData` buffer of town 4 | 0xffea7682 | no |
| town 4, both members, 1,440 frames | 0x014663a0 | the same buffer | 0xbebeb6d4 | no |
| town 0, both members, 900 frames | 0x014667c0 | a model's DMA colour buffer (`Decode_Mmat01`) | 0x80657077 | no |
| field 8 at once, Kite alone, 900, 240 or 1,440 frames | 0x012c7480, 0x013c2e90, 0x012c7350 | nothing: past every block handed out | the boot's | - |

- **On the story's way.** The manager lands in the room town 4's scene
  freed at the change, and reads what one of its owners wrote there.
  As for Gorre, the party and the frame the save was loaded on decide
  which. Both outcomes come from ordinary play.
- **Straight to the field.** Nothing is ever handed out at or above the
  manager, so its word is RAM no code of the game writes.
  `EE_FILL=a5a5a5a5` fills the heap's one hole at the first `_ccMalloc`,
  and all three runs read 0xa5a5a5a5 back.

  The game never reloads itself (no call of `LoadExecPS2` or
  `ExecPS2`). So that RAM is what the console's boot (the BIOS's loader)
  left. That code is not on the disc, and nothing here can say what it
  holds.

  The story never takes this way: block 14 always runs in town 4
  first. Only a console start at field 8 would.

The port has no `ccBoss08` yet. It keeps the first history's word,
`piney_world::combat::boss::BOSS08_HEAP_EXIT` = 0: the manager's own blur
through the first form, from the fight's first frame.

## Checks

- `ee_run` runs any volume. It finds `_start`'s `$gp`,
  `_ccMalloc`'s return and `ccThBossEffect`'s `new` by symbol.
  `EE_AREA=n` or `EE_THEN`'s `118,n` is the event instruction `area n`
  (`WORLD_MAN.eventAreaNumber`). `EE_FILL=word` fills the heap's one
  hole at the first handout.
- `EE_SAVE` writes the save over `saveData`, then calls
  `ccSaveData::LoadGame()`, as `ccThDemo` does after a load (worklog 466).
  `LoadGame` rebuilds the names' pointers and allocates nothing.
  - Before that call, character 0's name pointer stayed the port's
    stand-in `SAVE_VA` (0x01810000), which lies inside the heap.
  - Unfilled, it read as an empty name. Under `EE_FILL` the menu's
    `FacePanelDisp` walked `ccKanjiStrlen` over 25 MB of 0xa5. That was
    the town-4 "stall", the harness's own.
- `ee_run` with `EE_HEAP` prints the manager's word, the blocks under it
  and what came into the hole (`MANAGER at ...`); `EE_TRACE` writes every
  handout, free and cut; `EE_DUMP_FREED=size:path` the bytes of a freed
  block of that size.
- `EE_HEAP_MODEL` checks `piney_eemu::heap::Heap` against every handout.
- `heap::tests`: the first fit, the small lists' order, the top, the
  alignment and the cut, each on a fresh region.
- `session::tests::blur`: Gorre's fight on `HEAP_EXIT` draws no blur;
  on 0 it draws the manager's own every frame (scale 1.01 easing to 1.0,
  turn 0, 0x40808080).
