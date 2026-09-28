---
number: 148
title: "A member's own orders and the field objects' menus: ChatMenu1-3, ItemObjMenu to TimeIdolMenu"
date: 2026-09-26
area: ui, test
files: crates/piney-fieldui/src/menus/chat_member.rs, crates/piney-fieldui/src/menus/objects.rs, crates/piney-fieldui/src/menus/mod.rs, crates/piney-fieldui/src/menus/chat.rs, crates/piney-fieldui/src/menus/getitem.rs, crates/piney-fieldui/src/ctrl.rs, crates/piney-fieldui/src/lib.rs, crates/piney-fieldui/src/tables.rs, crates/piney-fieldui/src/world.rs, crates/piney-fieldui/examples/fieldui_probe.rs, crates/piney-world/src/combat/mod.rs, crates/piney-world/src/field_world.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session/tests/chat.rs, crates/piney-game/src/session/tests/shrine.rs, tools/test_fieldui_rs.py, docs/engine/field-ui.md, GAPS.md
---

# 148. A member's own orders and the field objects' menus: ChatMenu1-3, ItemObjMenu to TimeIdolMenu

A GAPS item. The port closed CHAT's member menus (71-73) and the field
objects' menus 34-37 and 39 at once, so the player could not tell a
member to heal and could not break a breakable.

## CHAT's member menus

- **What they are.** `ChatMenu1` (gcmn 0x0052a950) lists the member's own
  orders by place: Change Equipment in town; Designate Skill, Change
  Equipment, First Aid!, Designate Target, Assemble and Standby in a
  battle; the first three elsewhere. `ChatMenu2` (0x0052ba80) is the
  member's skills on `SkillMenuDisp`'s pages. `ChatMenu3` (0x0052c0b0) is
  the target: the party for a skill aimed at allies, else the enemies
  within 2200 in front of the camera.
- **What they ask of the battle.** `RequestChatCmd(member, cmd, target,
  skill)`: 16, 3 or 4 from 71, 5 with the skill or 11 from 73. The player's
  line goes into the balloon ("Orca, use Repth!").
- **Refusals.** A fallen member's rows say "Cannot be used while dead.".
  Change Equipment asks `ccSpcChar::CheckChangeEquip` (0x0059f3c0): the
  conditions, a skill under way (`skillID` 2 or more), acts 12-14. Its
  result is also CHAT's cancel mark. A member with no skill, too little
  SP or no target gets its own window.
- **Found on the way.** 71's `Select` passes `af` 1: `$a3` still holds the
  1 its dispatch compared. 73's text for a skill on the member ends
  "yourself!!", from the strings themselves.
- **The port.** `menus/chat_member.rs`. The orders go out as
  `Request::ChatOrder`, which the runtime already passes to the member's
  AI. `CharInfo::act` carries `actNum`. The help after the shut runs on
  the menu still open.
- **A runtime fix.** The runtime answered balloon positions only for the
  speakers of the previous frame, so the line a menu opened was dropped in
  the same frame (CHAT's own orders too). `hud_world` now also answers for
  the party.

## The field objects

- **What opens each.** `ccThGameCtrl`'s table maps a base type to
  `openReqNum` 0x1020 + k.
  - 34 (0x10000): the breakables, rows 2, 4 and 7-14. `EntryBreakObject`
    puts them in every room; story rows of type 0 put rows 2 and 4.
  - 35 (0x20000): rows 3 and 5. Nothing places them.
  - 36 (0x40000): the symbols, rows 17 and 18. Story rows of type 3 kind 4
    put them in event areas 17, 18, 20 and 24; the lake dungeons get 18.
  - 37 (0x80000): the virus crystal, row 6. Only later volumes' story
    rows use it.
  - 39 (0x200000): the Zeit statue, row 44. `SetIDOL` places it only under
    `timeSym`, set when the first word is 131, "Chronicling".
- **34 and 35.** Kite breaks the object (`BreakSomething`, act 25), then
  `AttackCancel`, `breakCount` +1. On 34, `rand() % 3` decides between an
  item from `areaItemList` and shutting. On 35, the trap's window closes
  on the OK button itself. With no trap or an unknown skill, 35 goes back
  to `proccess` 4 every frame and never ends.
- **36 and 37.** 36 shows "Received effects of <skill>!" and counts
  `symbolCount`. 37 gives the crystal's item, or Virus Core A.
- **39.** Its time is `gameCnt[2]`: `ccAddPlayTime` counts it and
  `ChangeRequest` zeroes it when a town is left.
  `CheckTimeIdolRankIn` and `SetTimeIdolRank` (main 0x001789a0,
  0x00178870) keep five ranks at +0x6775. The places gained give
  `timeIdolItem`s through 67. The message's second line is the rank's
  title: the code copies " #Wrecord." and then overwrites it.
- **The port.** `menus/objects.rs`, with `Request::BreakSomething` and
  `AttackCancel`. The runtime plays them before Kite's next frame
  (`Combat::kite_menu`, with piney-battle's `kite::break_something` and
  `attack_cancel`). `Game::game_cnt` carries `gameCnt`.

## Checks

- **The harness.** `tools/test_fieldui_rs.py` no longer stubs 71-73. New
  scenarios cover every order, the refusals, backing out, town and field,
  eight random runs, and each object menu's paths (the save's ranks
  watched). The probe gained `busy` (`skillID`, `actNum`), `mode` and
  `gamecnt`. All 50 tests, and the other field-ui suites, match every
  frame.
- **Session tests.** In `chat_a_member_to_heal_in_a_fight`, at event 3's
  goblins, CHAT > Orca > Designate Skill > Repth > Kite: Orca casts
  Repth and names it. In `a_breakable_opens_item_obj_menu`, in area 26's
  room (0, 6), Kite breaks a skeleton, `breakCount` rises and the item
  comes. `the_object_menus_open_by_type` opens 35, 36, 37 and 39 on a
  breakable given each type.
- **Shots.** `chat_member_shots` saves to scratch/chat_member and
  `object_menu_shots` to scratch/obj_menus.

**Still unknown:** Whether "Chronicling" can be had in Infection. The
runtime neither hands `timeSym` to `SetIDOL` nor counts `gameCnt`, so the
Zeit statue never appears and would read 00:00:00. `ccGimSymbol` is not
ported: Infection's symbols are stand-ins, so 36 names skill 0. Kite
starts event 29's dungeon session as a ghost (dead 4, no HP), so the
breakable test stands him up. That may be the start's doing or a real
state. The menus are not compared with the game's pictures.
