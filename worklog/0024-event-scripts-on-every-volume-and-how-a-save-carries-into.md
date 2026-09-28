---
number: 24
title: Event scripts on every volume, and how a save carries into the next
date: 2026-09-23
area: script, save, volumes, test
files: tools/evscript.py, tools/test_evscript.py, tools/save.py, tools/test_save.py, docs/engine/events.md, docs/formats/save.md
---

# 24. Event scripts on every volume, and how a save carries into the next

[[18]] decoded Infection's event scripts and [[16]] its save. Both leaned on
the symbol table. This entry makes both work from each volume's own code
and follows a save from one volume into the next. The work was done by a
helper agent; I re-ran its checks and read the key sites. The reference is on
[the events page](../docs/engine/events.md#other-volumes) and
[the save page](../docs/formats/save.md#carrying-a-save-forward).

## Event scripts without symbols

`tools/evscript.py` no longer uses Infection's addresses:
- It finds `Execute`'s, `CheckOpen`'s and `SetCurrentOpen`'s switches from
  their code: the `sltiu` bound, the default branch, the jump table and the
  `jr`.
- It measures each case's operand count by running that case in eemu.
- It takes script and message extents from the gaps between arrays. On
  Infection this reproduces every script name and size and all 170 message
  symbols.

`evscript.py check` then walks every script with the volume's own
interpreter: **0 mismatches on all four**. I re-ran it on MUT: 167 Execute
cases, 40 conditions, 12 tags, 196 scripts.

The interpreter grows by one case per volume (166, 167, 167, 168):
- **Opcode 99** gains an operand from MUT on: a noise level.
- **Opcode 168** (MUT on) sends mail 324 once any town has all three Grunty
  types grown up.
- **Opcode 169** has a case only in Quarantine
  (`QUA SLUS_205.64:0x001c52e8`). Outbreak's copy of the ending script
  already contains the opcode, and its interpreter skips it.

Opcode 169 answers [[23]]'s question of what Quarantine loads `KFED.BIN` for:
- It loads `KFED.BIN` and `KFAED.BIN` into two `$gp` globals.
- It adds the `ending.ccs` file list and runs gcmn `0x004f0a30`, a routine
  that starts a thread and waits for it.
- Then it frees both buffers.

I scanned every code word of main and the four overlays for accesses to
those two globals. The only ones are the case's own two stores and the two
loads that pass the buffers to `ccFree`. So the thread does not reach them
through those globals; whether it reaches them another way is not traced.

Other behaviour changes are listed on the events page:
- `fade` resets first;
- `call_lock` clears more bits;
- `staff_roll` grants wallpapers in OUT and QUA;
- character accessors reach the three new party members.

QUA adds seven ML events and shares its other 199 scripts byte for byte with
OUT. Each volume's first story event opens on `event_done 100·(v−1)`.

## The save across volumes

`tools/save.py` runs each volume's own `ccSaveSys` and `ccMcard` code in
eemu, against a memory card kept in Python. So paths, sizes, sums and copies
are whatever the game does.

The main differences from [[16]]:
- **Slot files are `dhdata01`..`12`, not ..13.** `MakeDir` writes twelve,
  and only `mcFname+4` is referenced.
- **From Mutation on, a slot is 0x8d84 bytes.** `ccSaveData` is still
  0x8530 on every volume, with the same member offsets. A 0x854-byte
  extension follows it, holding the lists, parameters and counters of the
  three new party members (Tsukasa, Subaru, Sora, `charTbl` rows 18-20).
  The constructor allocates it (`MUT SLUS_205.62:0x00175650`), and accessors
  route ids 18-20 there.
- Each volume has its own directory (`BASLUS-2056xDOTHACK`) and three icons.
  Mutation's `mcDirName` finally fills in the three later directories that
  Infection had as placeholders.

The carry-over, which `save.py carry --prev` runs end to end:
1. The title screen offers the previous volume's saves when one has
   `clearFlag >= volumeNum - 1`.
2. `LoadInfoPrevReq` reads the previous volume's index. MUT reads
   Infection's, OUT reads MUT's, QUA reads OUT's.
3. `LoadDataPrevReq` reads the slot into `ccSaveData` at the same offsets.
   Mutation leaves its extension zeroed, since Infection has none. OUT and
   QUA copy the previous extension as it is.
4. `ccThDemo` calls `ccStartEventConvert`, which marks event
   `100·(volumeNum−1)` done, then `ConvGame`. `ConvGame` resets every
   character not in the party to its `charTbl` row, reloads names and
   settings, clears the trade lists, and sets `newGameFlag` to 2.

I ran INF→MUT and MUT→OUT. In both, the saved bytes load back identical
apart from padding. For INF→MUT the trace shows the three copy ranges and
the untouched extension.

**Correction to [[16]]:** `ConvGame` is reachable. Infection's `ccThDemo`
calls it at `INF demo.prg:0x00400b2c`, right after `ccStartEventConvert` at
`0x00400b20`, which I read myself. The previous-volume path runs in
Infection too: `volumeNum - 1 = 0` is replaced by 1, so it loads its own
directory and marks event 0 done.

`tools/test_evscript.py` and `tools/test_save.py` cover every extracted
volume and skip the rest.

**Still unknown:** what the Mutation-on blocks at `ccSaveData+0x8432` and
`+0x8462` hold, and the extension's 3 bytes at +0x748 and 256 at +0x754; the
save bit at `+0x5ec8` bit 62 (see [[25]]); what fills OUT's and QUA's BSS
message groups; what OUT's rewritten opcode cases (6, 8, 9, 60, 71, 72, 150)
do; what the ending routine does while the KFED buffers are loaded.
