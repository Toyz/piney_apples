---
number: 215
title: Kite's, the streams' and the effects' tables generated
date: 2026-09-27
area: volumes
files: crates/piney-gen/src/manifest.rs, crates/piney-gen/src/layout.rs, crates/piney-gen/src/dtype.rs, crates/piney-data/src/tables, crates/piney-battle/src, crates/piney-stream/src, crates/piney-effect/src, crates/piney-game/src
---

# 215. Kite's, the streams' and the effects' tables generated

The Infection boot got through the desktop but stopped entering The World.
Every run-time image is GCMN.PRG over an empty executable, so every read
of main's data failed:

- `DamActuTbl`, in Kite's tables;
- `streamTblE`, in every stream;
- `effectCCSTbl`, in the field's effects.

Three systems now read generated tables instead, all four discs'. An
inventory of every run-time image read left in the crates (world towns,
fields, dungeons, streams, effects, battle, the story announcements) set
the order: what the boot reaches first.

## The groups

| group | what |
| --- | --- |
| `combat` | Kite's clips (`playerAnimTbl`), `DamActuTbl`, `tsp`; the members' clips (`fellowAnimTbl`) and the following's constants; the ride's clips, files and angles; Skeith's act tables, clips and random skills |
| `stream` | each stream's files (`streamTbl`, `streamTblE`), its BGM changes (`strSndTbl`), the gate hack's files, `str7000Out`, the subtitles, the stream effects' rotations, the ending's cue run (`eventObjTbl_0580`/`0581`) and rock scales |
| `effect` | the effect files and rows, the particle system's tables, `ccParticle::Setup`'s texture switch, the spells' tables, the hits' and the fly font's, the element and boss generator rows |

The readers take `of(volume)`: `KiteTables`, `MotionTables`,
`RideTables`, `SkeithData`, `table::Def`, `Subtitles`, the stream
effects' and ending's tables, `Effects::new` and `StreamEffects::new`
with every spell's tables. The port no longer builds an image for any of
them.

## What the later discs showed

- **Outbreak and Quarantine have no Japanese-voice streams.** Their
  `ccStreamInit` builds `streamTblE` alone, and their gate hack has only
  the `E` tables. The Japanese-voice entries are absent there, and the
  readers take the `E` ones as the game does.
- **Reading past a table's end.** `hitRot0120a`, `hitRot0240`,
  `particleTbl` and the thunder's time lists are read on past their ends,
  as the game does. On a later disc the bytes there are that disc's own,
  so the vote by likeness to Infection chose wrong places.
  `hit_rot_0120a` landed a row off on Outbreak and Quarantine. Such an
  entry now takes the carry's place alone (`.carried()`).
- Only the thunder's time lists (their tail) differ between the discs.
  Every other effect table is identical on all four.

## The generator

- **Bitfields.** `ccParticleGeneratorParam` packs `gType`, `rType` and
  `dType` into one halfword. A member with `AT_bit_size` is a `Bits`
  layout, read from its storage unit. Its `write` sets only those bits.
  DWARF 1 says `AT_bit_offset` counts from the most significant bit, but
  here it counts from the least.
- **An end row kept.** `array_through` keeps the row `until` finds:
  `strSndTbl`'s BGM tables end on a record the cursor rests on.
- **All-capital names.** A DWARF name in capitals becomes CamelCase
  (`FOOD_PARAM`, `STREAMDATA`, `CCSND_STR_BGM`).
- **Decoded code.** `ccParticle::Setup`'s switch is decoded from each
  volume's code in the generator, no longer at run time.
- **`piney-gen global NAME`** prints a global's DWARF type and its
  struct's members. Every table here was laid out with it.

Particle rows keep their identity as the volume's own address of the row
(`*_va` entries), which the harnesses compare. The boss effects' rows
stay keyed by Infection's addresses as their names.

## Checks

- Kite's, the members', the ride's, Skeith's and the navigation's battle
  harnesses pass.
- The stream crate's tests pass: the stream fixture, the subtitles, the
  ending and the effects. So does the audio crate's stream test.
- piney-effect's tests pass, and `effects_on_every_disc` builds the
  field's and the stream's effects on all four discs.
- The effect harnesses all pass against the game run in eemu:

  | harness | cases |
  | --- | --- |
  | effect | 6 |
  | particle | 8 |
  | hit | 3 |
  | misc | 9 |
  | draw | 2 |
  | skill | 6 |
  | spell | 5 |
  | stream | 1 |
  | boss | 4 |
- `the_world_top_page_and_into_mac_anu` passes. The World enters Mac Anu
  again.

**Still unknown:** The town and field readers in piney-world still read
GCMN.PRG with Infection's addresses (`npcTbl`, the navigation tables,
`rtpc*`, the field map's icons), and `mapmsg` and `markerEvTbl` are
main's. Infection's towns work on them, but no later disc does, and the
field map's `mapmsg` fails on every disc. The story announcements' main
strings read empty. These are next, with the older generators behind
their tables.
