---
number: 192
title: "Skeith's death clears the party's conditions (ccClearSpcCondition); the records' reload keeps what the world changed between frames"
date: 2026-09-26
area: battle
files: crates/piney-world/src/field_world.rs, crates/piney-world/src/combat/mod.rs, crates/piney-game/src/area.rs, crates/piney-game/src/session/tests/skeith.rs, docs/engine/battle.md, docs/engine/field-ui.md, docs/engine/sound.md, docs/engine/field-game.md
---

# 192. Skeith's death clears the party's conditions (ccClearSpcCondition); the records' reload keeps what the world changed between frames

The "not ported" sweep went on through `battle.md`, `sound.md`,
`field-game.md` and `field-ui.md`.

## ccClearSpcCondition

`battle.md` said `ccClearSpcCondition` (gcmn 0x0056d300) was not ported.
piney-battle's boss did raise it, as `Out::ClearSpcCondition`, when a
hit takes Skeith's HP to 0. But `area.rs`'s `boss_calls` let it fall
through to `_ => {}`. So after the fight the party kept its poison, its
stat changes, and anyone under a condition.

The function (read):

- `ccSpcConditionEffectOFF`.
- For each registry id 0-17 with a character built (`CheckSpc`):
  - `condition.dead` is saved;
  - `ManualModeAI(1)`, which revives the fallen, and
    `ai->SetRemoteCmd(0)`;
  - `noDeathFlag` (+0xe0 bit 7) set;
  - `ccDeleteCmnd`;
  - `ClearCondition()`, then `ccClearParamElement` (0x005703f0, sixteen
    shorts zeroed) on the record's +0x88 and +0xa8;
  - `dead` put back as it was.

So the revival is taken back, but its `cloak`, `ghost` and body-on-list
stay.

`FieldWorld::clear_spc_condition` does the same over the members built,
and `boss_calls` calls it. `event_30_ends_with_skeith` poisons Kite and
raises his physical attack in his record (with its timer) before the
end. At the frame his HP reaches 0 both are gone and `noDeathFlag` is
set.

## The records between frames

The test first found the raised stat still in the save a frame after the
death. Worklog 187 made each frame start by reading every member's record
back from the save, so that the menus' writes hold. That was also what
undid, a frame later, any change the world made to the battle's copy
between frames:

- this clear;
- `MenuBan`'s `ClearCondition(ccSpcParam *)`;
- the fountain's hold.

In the game there is one record. `ccChar` +4 points at
`saveData.spcParam[id]`.

Now `Combat::store_records` stores the copies and remembers what it
stored:

- each frame's end calls it, and so do the three changes above;
- a frame starts by reading back only the records the save no longer
  holds as stored, which are the ones a menu or script wrote.

A record changed from both sides between two frames takes the save's.
None is known.

## The rest of the sweep

- `sound.md`: `ccSndSQLoad(6)` has no caller in Infection. The callers
  are `ccSetupDemo` (7), `ccSetupToppage` (0) and `ccSetupGameCtrl` (2 to
  5), in main and every overlay.
- `field-game.md`: `ManualControl`'s walks by the town navigator are
  ported (`ai_move::manual_control`, `TownNavigatorPoint`); the line said
  not. Event mode -3's virus crystal is volume 2's event 114 alone.
- `field-ui.md`: the Grunty's own balloons (`pgChatTbl`) are ported.
- GAPS' "Type-32 rooms' leaves, the dungeons' warps" stays under "After
  Infection". Only story areas 101 (type 32) and 52 and 88 (warp rows)
  have them. Infection's own events never open them: 114, 260 and 303
  do, and those are later volumes'.

**Still unknown:** Whether any menu or script writes a record in the
same frame gap as one of the world's clears (then the save's wins).
