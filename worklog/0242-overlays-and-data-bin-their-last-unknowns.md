---
number: 242
title: Overlays and DATA.BIN, their last unknowns
date: 2026-09-27
area: format
files: docs/formats/prg.md, docs/formats/data-bin.md
---

# 242. Overlays and DATA.BIN, their last unknowns

Asked for: settle the docs' open questions, starting with the format
pages. `docs/formats/prg.md` and `data-bin.md` are now solid.

## The overlays

- **`overlay_id` at run time.** Nothing reads it. `mwLoadOverlay(path,
  dest)` reads only three header fields at `dest`: `bss_size` (+0x14) and
  the constructor table (+0x18, +0x1c). `dest` comes from
  `overlay_tbl[slot - 1]` in `ccThLoadOverlay`, and every call passes slot
  1 (0x00400800). No instruction in the executable or an overlay loads
  from 0x00400804.
- **Which file.** The event instruction `overlay num` is the only caller of
  `ccLoadOverlay`: 0 `DEMO`, 1 `DESKTOP`, 2 `TOPPAGE`, 3 `GCMN`.
- **Destructors.** The static constructors register destructors on
  `__global_destructor_chain` through `__register_global_object`, and
  nothing else reads the chain. There is no `__destroy_global_chain`, and
  `_exit` goes straight to the kernel. So no destructor ever runs, when an
  overlay is replaced or at exit. The stale records of a replaced overlay
  are never walked.

Mutation, Outbreak and Quarantine load the same way (each through its own
`overlay_tbl`).

## DATA.BIN

- **`usize` 0.** `ccFileListLoad` (0x00164ea8) wires its two threads by
  it:
  - a `usize` other than 0: `FileReadTh` fills the compressed ring and
    `UngzipTh` inflates into the ring `ccStream` reads;
  - 0: `FileReadTh` fills `ccStream`'s ring itself, and `UngzipTh` gets no
    `ccUngzip` and never decodes.

  So 0 means stored without compression.
- **Category 19.** `ccAddFileList` gives a category-19 entry `offset` 0,
  `csize` -1 and `usize` 0, in `directCCSTbl`. `ccFileListLoad` opens
  `cdrom0:\DATA\` + name, sizes it with `sceLseek`, and reads it raw. The
  two unknowns were one: a `usize` of 0 is category 19's.

  No volume adds a category-19 entry:
  - no `{19, name}` pair in any executable or overlay (a scan of every word
    pair);
  - the lists built at run time take constant categories (7, 9, 11, 15 to
    18);
  - every store of the constant 19 was looked at, and none is a file list.

  No disc has a loose `.CCS` in `DATA`.

**Still unknown:** Nothing on these two pages. What category 19 was for
is not in the code; it only shows that the game can load a scene file
from outside the archive.
