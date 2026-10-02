---
number: 325
title: "The later volumes' changed event cases: seven compile differently, six change behaviour"
date: 2026-10-01
area: script, volumes, decomp
files: crates/piney-event/src/vm/exec.rs, crates/piney-event/src/state.rs, crates/piney-event/src/host.rs, crates/piney-game/src/field_host.rs, crates/piney-game/src/area_host.rs, crates/piney-event/tests/clean_room.rs, docs/engine/events.md, docs/engine/event-vm.md
---

# 325. The later volumes' changed event cases: seven compile differently, six change behaviour

[[24]] left open what Outbreak's rewritten cases 6, 8, 9, 60, 71, 72 and 150
do. [[40]] noted that the VM runs Infection's rules for every volume,
including the cases [events](../docs/engine/events.md) lists as changed
from Mutation on. Each case was read from `Execute`'s jump table on the
four volumes (INF 0x00355fd0, MUT 0x00370070, OUT 0x00369a20, QUA
0x0025cff0) and compared with INF's.

## Outbreak's seven: the same behaviour

The call lists match INF's except for small functions in main, which do
what INF writes inline:
- OUT `0x001bdd90` writes `ccMenu +0x14`, `+0x16` and `+0xf8 = 1` (150);
- OUT `0x001bddb0` and `0x001bddc0` write `ccAI +0xa2` and `+0xa4`, which
  are INF's `+0x9a` and `+0x9c` moved by 8 (71, 72);
- OUT `0x001bddd0` clears a byte's bit 0 (60: four calls, where INF has
  four inline `and -2`);
- OUT `0x001bddf0` calls `ChangeInfo` and zeroes `ccMessage +0x1e` and
  `+0x34`, as INF does inline after each `ChangeInfo` (9);
- OUT `0x001bde30` is a constructor zeroing `+0x00` (6, 8, 9).

The unnamed gcmn callees match INF's functions by their code (mnemonic
similarity 0.91 to 0.98):

| INF | OUT |
| --- | --- |
| `ManualModeAI` | 0x005c7490 |
| `SetRemoteCmd` | 0x005a6240 |
| `ccEntryCmnd` | 0x005329c0 |
| `SetDircZ` | 0x005a44e0 |
| `CheckMenuType` | 0x0053f540 |

QUA has the same shape: the same call counts, and its setters at
0x001c5410-0x001c5440 write the same offsets. The port's Infection rules
are right for these seven.

## Mutation on: what does change

Checked on MUT, with OUT and QUA the same code:
- **84 `call_lock`** clears characters 0-20 (`slti 21`, MUT 0x001c4990),
  not 0-17 (`slti 18`, INF 0x001af6d8). Bits 18-20 are Tsukasa, Subaru and
  Sora, so on the port they stayed callable.
- **101 `mode`** is always `ChangeRequest(num, 7)` (MUT 0x001c552c). INF's
  `mode 5` also did `ChangeArea(0, lastTown)`.
- **145 `fade`** calls `ccScFade::Init` first (MUT 0x001c6f50). `Init`
  (INF 0x0015fb40) zeroes the four elements' status.
- **146 `fade_more`** continues the stored element only while `fadeNum` is
  0-3. Otherwise it calls `Init` and `EntryFade(count, alpha << 24, 0)`, a
  fade from the alpha to nothing (MUT 0x001c6fc0).
- **167 `desktop_item`** skips a movie (type 2) in Parody Mode, before the
  play test (MUT 0x001c51e4).
- **96, 142, 80, conditions 33 and 34** with characters 18-20: the port's
  accessors bounded the character at 18. The save carries the extension,
  and `piney_data::save::by_id` already mapped 18-20 there.

Uses of characters 18-20 in the scripts:
- INF and MUT: none.
- OUT: 12 `in_party`/`not_in_party`, 6 `call_on_later`, 6 `member_add`.
- QUA: those, plus 30 `friendship` adds and 36 `friendship` conditions.

The port's `friendship` read 0 and its adds did nothing.

The VM now follows the volume (`volume >= 2`) in each of these. `Host::fade`
takes `reset`, and the game's two hosts share `ScFade::event`. The event
accessors (`item`, `skill`, `friendship`, `gold`, `talk_num`) go through
`by_id`, with `CHARACTERS` 21. No Infection script names 18-20, so
Infection's runs do not change.

Two docs claims were wrong:
- **118 `area`:** the change is not the `game+0x14` test, which INF has
  too. Area 126 sets `eventAreaNumber` without quitting `WORLD_MAN` or
  building words (MUT 0x001c5874).
- **8 and 9:** the extra check is a NULL test of `dtMenu` (MUT 0x001be908).
  With no desktop menu the instruction is passed over and `Execute`
  returns 0; INF dereferences the pointer.

The port models neither: it has no state without a desktop menu, and area
126 is used once.

`mode 5` and `area 126` appear in one script, event 314 (block 2: four
streams, `mode 5`, `area 126`, `scene` field 126). Its number is in volume
4's main range, which only Quarantine's pass walks, so it is Quarantine's
ending. Outbreak carries the script and never runs it.

## Checked

- `later_volumes_rules` (piney-event, clean_room) runs one script on
  Infection's rules and on Mutation's, at event 60, a side story every
  volume's pass walks. It checks `call_lock`'s bits, `mode 5`'s requests,
  the Parody Mode movie, and character 19's friendship and talk count in
  the extension. Every Mutation assertion fails on the old code.
- `event_fades_by_volume` (piney-game, field_host) checks `fade` and
  `fade_more` with all four elements busy on both rules.
- The piney-event and piney-game suites pass (203).

**Still unknown:** what `eventAreaNumber` 126 loads for Quarantine's ending
(event 314, `scene` field 126; INF's `eventAreaInfo` has 126 records,
0-125); the port gives area 126 no words and has not played it. What fills
OUT's and QUA's BSS message groups is still open from [[24]].
