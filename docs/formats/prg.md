---
title: The .PRG overlay
status: solid
volumes: all
covers: INF DATA/GCMN.PRG, INF DATA/DEMO.PRG, INF DATA/DESKTOP.PRG, INF DATA/TOPPAGE.PRG, INF SLUS_202.67:0x001002c0 mwLoadOverlay, 0x00168030 ccLoadOverlay, 0x001680e0 ccThLoadOverlay, 0x00377b70 overlay_tbl, 0x0010bd20 __register_global_object, 0x00379bc0 __global_destructor_chain, 0x001b0330 ccEvent::Execute's overlay
worklog: 2, 4, 242
---

# The .PRG overlay

A Metrowerks CodeWarrior overlay: code and data that load into the shared
window at 0x00400800, one overlay at a time, replacing the last. Loaded by
`mwLoadOverlay` (`INF SLUS_202.67:0x001002c0`).

## Layout

All little-endian. The header is loaded along with the rest, so a byte at file
offset `o` lands at `load_va + o`.

```
header      0x40 bytes, at load_va
  char[4]   magic            "MWo3"
  u32       overlay_id       1 gcmn, 2 demo, 3 desktop, 4 toppage
  u32       load_va          0x00400800 in all four
  u32       text_size
  u32       data_size
  u32       bss_size
  u32       ctor_start       VA of the static-constructor table
  u32       ctor_end         VA one past its last entry
  char[32]  name             "gcmn.prg", NUL-padded

text        at +0x40, text_size bytes
data        at +0x40 + text_size, data_size bytes
bss         at +0x40 + text_size + data_size, bss_size bytes, not stored
```

File size is always `0x40 + text_size + data_size`.

## The four overlays

| file | id | text | data | bss | ctors | footprint end |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `GCMN.PRG` | 1 | 0x1d0fc0 | 0x129080 | 0x35d80 | 0x006fa850-0x006fa85c (3) | 0x00730600 |
| `DEMO.PRG` | 2 | 0xd3c0 | 0x1c80 | 0xb80 | 0x0040f810-0x0040f81c (3) | 0x00410400 |
| `DESKTOP.PRG` | 3 | 0x1adc0 | 0x4ea00 | 0x380 | 0x00469fc0-0x00469ffc (15) | 0x0046a380 |
| `TOPPAGE.PRG` | 4 | 0x4140 | 0x26180 | 0 | 0x0042ab00-0x0042ab00 (0) | 0x0042ab00 |

"Footprint end" is `load_va + 0x40 + text + data + bss`, and equals the end of
the matching ELF program header (`vaddr + memsz`) in all four.

## Fields

`text_size`, `data_size`, `bss_size` - checked against the ELF symbol table:
the lowest data object of each overlay sits exactly at
`load_va + 0x40 + text_size`, and every function lies below it except the
static initialisers.

`ctor_start`, `ctor_end` - a table of function pointers. Every entry in the
three non-empty tables resolves to a `__sinit_<source>.cpp` symbol, the
compiler-generated static initialiser for one source file.

## Loading

`mwLoadOverlay(path, dest)` (`INF SLUS_202.67:0x001002c0`):
1. `mwBload` reads the whole file to `dest`.
2. `FlushCache(2)`.
3. `bss_size` bytes after what it read are zeroed.
4. `__initialize_cpp_rts(ctor_start, ctor_end)` runs the static
   constructors.

It returns 1 on success and 0 if the read returned nothing. Of the header
it reads only `bss_size` (+0x14) and the constructor table (+0x18, +0x1c),
at `dest`.

`dest` comes from the caller, not the header. The one caller is
`ccThLoadOverlay` (0x001680e0), the thread `ccLoadOverlay(slot, path)`
(0x00168030, priority 17, stack 0xc00) starts and waits for. It reads
`overlay_tbl[slot - 1]` (0x00377b70), retrying until the load succeeds.
`__sinit_mother.cpp` fills the table at boot with two addresses:
- `overlay_loadaddr_1`, 0x00400800;
- `overlay_loadaddr_2`, from `__data_end`.

Every call passes slot 1.

The only caller of `ccLoadOverlay` is the event instruction `overlay num`
(`ccEvent::Execute`, 0x001b0330). Its operand picks the file:

| `num` | file |
| ---: | --- |
| 0 | `cdrom0:\DATA\DEMO.PRG` |
| 1 | `cdrom0:\DATA\DESKTOP.PRG` |
| 2 | `cdrom0:\DATA\TOPPAGE.PRG` |
| 3 | `cdrom0:\DATA\GCMN.PRG` |

So the header's `overlay_id` and `load_va` agree with how the game loads
each overlay, but nothing reads them: no instruction in the executable or
any overlay loads from 0x00400804.

## Destructors

The static constructors register their objects' destructors with
`__register_global_object` (0x0010bd20). It links a record into
`__global_destructor_chain` (0x00379bc0):
- main's `__sinit_system.cpp`, `__sinit_libcc3d.cpp`, `__sinit_effect.cpp`;
- `gcmn.prg`'s `__sinit_gamectrl.cpp`;
- and the others'.

Nothing reads the chain but that function, as it links the next record.
The executable has no `__destroy_global_chain`, and `_exit` jumps
straight to the kernel's `Exit`. So no destructor runs, whether an
overlay is replaced or the game ends. An overlay's records stay linked
after the overlay is replaced, pointing into memory that the next overlay
overwrites, which does no harm because nothing walks them.

## Notes

The static initialisers are not in `.text`: CodeWarrior places them at the tail
of `.data`, immediately before the constructor table. A disassembler that only
treats `.text` as code will miss them.

The overlay sections in the ELF (`gcmn.prg` and so on) have size 0, but their
symbols (section index 6, 8, 10, 12) and relocations (`.relgcmn.prg` ...) are
present - see [the executable](../engine/executable.md). Since all four share
0x00400800, an address in the window means nothing without saying which
overlay is loaded.

## Other volumes

Mutation's, Outbreak's and Quarantine's `mwLoadOverlay` read the same
three header fields (+0x14, +0x18, +0x1c) at the load address.
`ccThLoadOverlay` indexes the volume's own `overlay_tbl` ($gp -32640 on
Mutation; -28952 on Outbreak, -29072 on Quarantine). Outbreak's and
Quarantine's `ccLoadOverlay` is called from four places in
`ccEvent::Execute`, as Infection's. None of the four has a
`__destroy_global_chain`. The overlays' header values per volume are in
[the four discs](../disc/volumes.md).

## Unknown

None: the fields, the loading and the constructors are accounted for
above.
