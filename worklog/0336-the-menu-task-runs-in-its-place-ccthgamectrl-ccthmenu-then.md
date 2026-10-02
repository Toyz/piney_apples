---
number: 336
title: "The menu task runs in its place: ccThGameCtrl, ccThMenu, then the world"
date: 2026-10-01
area: engine, ui, audio
files: crates/piney-world/src/field_world.rs, crates/piney-world/src/lib.rs, crates/piney-game/src/area.rs, crates/piney-game/src/world.rs, crates/piney-game/src/session.rs, crates/piney-game/src/session/tests/chat.rs, docs/engine/field-ui.md, GAPS.md
---

# 336. The menu task runs in its place: ccThGameCtrl, ccThMenu, then the world

[[334]] found the cause of [[101]]'s late words. The game runs, in this
order:
1. `ccThGameCtrl` (33);
2. `ccThMenu` (34);
3. the camera (40), the party (48), the entries (64), the effects and
   the skills.

The town's and the field's modes ran the menu task after every world task.
Every request the menus make (a skill, an item, an order) therefore reached
the characters, and the sound it leads to the driver, a frame late.

## The change

- **`FieldWorld::step_game_ctrl` and `World::step_game_ctrl`** (the town)
  run `ccThGameCtrl` on its own, under the same condition as before (play
  from F1, awake). Their `frame` skips it once it has run (`ctrl_done`).
  Anything that only calls `step_into` behaves as before.
- **`AreaMode::step`** now runs:
  1. the event task, the map button and the ride's slot;
  2. `step_game_ctrl`, then what it raised: the game over's first step,
     the talks, the attack flag;
  3. the new `menu_task`: Data Drain's movie, the ride's and the Fairy's
     Orb's slots, `ui.step_into` and its requests, which carry out item
     uses and steps at once;
  4. `world.step_into`, the camera on;
  5. the shows, the sounds, the menu's rules, the chats and the map.

  The menu runs from F1, which is the frame it ran in before (the old
  test of `f >= 2` came after the world had counted the frame). Data
  Drain's black now keeps the world out of the frame by stepping it into
  a frame of its own, not by clearing what was drawn, which would have
  wiped the menus.
- **`WorldMode::step`** (the towns) does the same. The book's stream and
  the gate hack's screen go the same way. The walking PCs' and the
  members' chat lines come from later tasks, so the menus show them the
  next frame, as in the game.

## The tests that moved

Five tests assumed the old order:
- **`a_skill_in_a_fight_names_itself`** now also requires Kite's
  `SkillWords` in the frame the target is confirmed (menu 65's CROSS).
  The old order fails it (words at 249, confirm at 248). The new one
  passes.
- **`a_spell_lands_through_the_effects`**: the blast's noise reads 20 at
  the frame's end, not 19. `SetNoizBs(20)` comes from the effects (80),
  after the menu task's `Disp` (34) has drawn that frame's noise.
- **`the_last_portal_puts_up_its_banner`**: 91 frames on, not 90. The
  entry control (64) starts `ccThDfComp` after the menu task, so its first
  `Main` is the next frame, and it then runs its 90 `Main`s. Before, the
  port started and ran it in one frame.
- **`a_scroll_on_a_foe_is_cast_by_kite`** watched for `skill_id` 193 at a
  frame's end. The request sets it, and the party's and the skills' tasks
  clear it in the same frame, as the cast (act 17) begins. Only the old
  order left it visible, so the test now takes the cast's act as the use.
- **`chat_a_member_to_heal_in_a_fight`** recorded the order after scanning
  the frame's events. The member now names Repth in the order's own
  frame.

The piney-game (204), piney-world and piney-fieldui suites pass. So do
Mutation's and Outbreak's whole stories.

**Still unknown:** the remaining frame-level order inside the world's own
step (the camera, `ccThSpc`, `ccThEntryCtrl`, the effects, `ccThSkill`)
was left as it was. The rest of [[101]]'s list (the chat balloon, the
skill row's type bit) is unchanged.
