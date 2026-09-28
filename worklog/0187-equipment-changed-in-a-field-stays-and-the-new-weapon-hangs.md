---
number: 187
title: Equipment changed in a field stays, and the new weapon hangs on the model
date: 2026-09-26
area: battle
files: crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-world/src/lib.rs, crates/piney-world/src/town_party.rs, crates/piney-world/src/chara.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, docs/engine/field-ui.md
---

# 187. Equipment changed in a field stays, and the new weapon hangs on the model

**The revert.** In a field, a piece put on through Equipment came off
again at once. The menu writes the save's record (`ChangeEquipment`,
then `CalcReal`), but the combat keeps its own copy of each member's
`SpcParam` and stored it back into the save at the end of every frame,
so the old equipment won.

The game has one record: `ccSpcChar`'s `ccSpcParam` is the save's own
`spcParam[id]`. `Combat::frame` now reads each member's record from the
save at its start, so whatever the menus, a book read or an item given
wrote between frames holds. The frame writes the records back at its end
as before.

`drain_side_effects_show` had set Kite's experience and level on the
combat's copy only. It now writes them to the save as well, where the
game keeps them.

**The model.** Both modes dropped `ChangeEquip`. The town had it on its
list of requests not carried out ("new equipment on its model"); the
field let it fall through. Read:

- `ccSPC::ChangeEquip` (gcmn 0x0059f9e0) switches on the category below
  10. Weapons (0-5) go to `ChangeWeapon`. The armour's `ChangeHelmet`,
  `ChangeArmor`, `ChangeGlove` and `ChangeBoots` change nothing drawn.
- `ChangeWeapon` (0x0059fb50) loads the new weapon's file
  (`ccGetJobWeaponParam(job, item)` +4) through `spcTempFileList`, and
  lets go of a file still coming. With the character built, it puts the
  file at the row's +0x28 and sets `weaponChangeSW` (+0xe1) to 1. Picking
  the worn weapon again undoes a pending change.
- The character's own frame hangs it: at 1, `ccPlayer::Main` and
  `ccFellow::Main` run `EquipWeapon` and leave -1. The frame after,
  `DeleteWeaponCCS` has `ChangeWeaponOldNewCcs` (0x0059fdb0) free the old
  file.

**The port.**

- `FieldWorld::change_equip(id, cat)` hangs the save's weapon
  (`body::weapon_of`) on the combat actor's body. For Kite it also re-arms
  the `Kite` the field builds his actor from.
- `World::change_equip` does the same in the town: `Kite::arm` (split out
  of `read_armed`) re-hangs his blades, and `TownParty::change_weapon`
  re-hangs a member's.
- Both modes route `ChangeEquip` by its handle. The town's `CalcReal`
  stays covered by `calc_real_party`, which runs every frame.

The swap happens as the request is handled, a frame before the game's
character frame would hang it (field-ui.md).

Checks:

- `a_members_new_equipment_stays`: Orca's new body piece stays in the save
  and changes his `tune`. Kite's stays too. Then another twin-blade file
  in his record ends up on his hands after `change_equip(0, 0)`.
- `new_blades_in_his_hands`: the same in Mac Anu, on `Kite::hands`.

**Still unknown:** Nothing new. How many frames the game's weapon load
takes (`ccLoadFLAddOne` before `EquipWeapon` can run) was not measured.
