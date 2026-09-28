---
number: 224
title: Mutation's movers, town AI and spawns, and two sidecar passes
date: 2026-09-27
area: volumes
files: crates/piney-gen/src/syms.rs, crates/piney-gen/src/xfer.rs, crates/piney-battle/src/follow.rs, crates/piney-battle/src/kite.rs, crates/piney-battle/src/ai_move.rs, crates/piney-battle/src/entry.rs, crates/piney-battle/src/event.rs, crates/piney-battle/examples/battle_probe, tools/test_battle_fellow_rs.py, tools/test_battle_kite_rs.py, tools/test_battle_event_rs.py, tools/test_battle_navi_rs.py, tools/test_battle_spawn_rs.py
---

# 224. Mutation's movers, town AI and spawns, and two sidecar passes

After the party AI (worklog 223), five more battle harnesses failed on
Mutation: fellow, kite, event, navi and spawn. With them, the chat and
enemy motion harnesses, whose random streams moved, showed two more
gaps. All fifteen battle harnesses now pass on Mutation and still pass
on Infection.

## The probe's shared readers

The fellow, kite and navi requests each kept a copy of the party AI's
`read_ai`, `ai_json` and message helpers, without Mutation's new ccAI
fields. The probe ran out of tokens on every such request. They now use
`party_ai`'s. Navi adds its navigation block after the shared fields
(`ai_fields`). About 450 duplicated lines are gone.

## Rules Mutation changed

- **`ccAI::FollowTarget`.** The member turns to its target at any
  distance (Infection only beyond `noTurnRange`).
- **`ccAI::CheckFrontObstacle`.** The probe line reaches
  `1.5 bodyHit.radius`. Infection's was `bodyHit.radius + 1.1 speed`.
- **Kite's arrival by warp (act 13).** The fade runs to frame 50 and the
  act ends after 60. Infection's ran to 40 and ended there.
- **`ccAI::ActInTown`.**
  - A party member stands back (`actType` 100, mode 1, stopped) while it
    or Kite goes through a gate (act 12).
  - When Kite is lost (off the lists, or the member out of the party),
    acts 3, 94 and 97 also stop the member. Act 3 also drops the command
    when Kite is gone.
  - Act 96 neither greets Kite nor goes to him while he rides the Grunty
    (`pgRideFlag`).
- **`ccRegisterEnemyList` and `ccAnalyzeEnemyList`.**
  - A start too near the list's end moves back, so that all
    `ccRegisterEnemyRange` rows fit. Infection stayed on the last row
    instead.
  - The analysis reads the range's rows and compares the first three.
- **The hit record** (`Event::NoteHit`, worklog 223) carries whether the
  character has an AI. The game writes through a null pointer
  otherwise, and the harnesses record the call with 0.
- **`ccTransPosW2P`, `P2W` and `FW2LW`.** Without a player (`plw.pw`
  null) they leave the position as it is. The port follows this for the
  magic circle's particles (`McPart::main`). The other `FW2LW` users
  always have a player in play.

## The harnesses

- **Fellow, kite, enemy motion and spawn** hook the hit record, as
  `test_battle_rs` does.
  Fellow points the player global at the character whose affect function
  is Kite's `Influence`. That function reads Kite's AI through the
  global.
- **Event** reads the ccAI fields back through `volume.ai_at`. It had read
  Infection's offsets, which on Mutation land on the new hit fields.
- **Chat** lets Mutation's remarks run natively, as it does every other
  line, and compares their text. It had inherited the party AI
  harness's remark hooks.
- **Navi** compares `ActInTown`'s lines by their bytes. The game passes a
  string's address. The same string sits in two tables, so a table-and-row
  lookup was ambiguous. The probe gives the volume's text
  (`ChatTexts`).
- **Spawn.**
  - The port names an object's destructor (`ccEntryObj` +0x1c8) by
    Infection's address. The harness sends those names to the port and
    writes the volume's addresses into the game. It maps every `ccDest*`
    function Infection names, both ways.
  - Mutation's `DUNGEON` allocates its floors. Their number is at +0x430,
    and the gimPos slots run to 75 per floor (Infection's fixed 750).
    `rotate` sits behind a pointer at +0x7cc, and `edit` is at +0x934.
    The harness gives ten floors (750 slots, as the port keeps).

## Two sidecar passes (piney-gen syms)

Mutation's `ccThEvHold` was placed inside `ccEvent::Execute`. The ref
pass had taken a "likely start" after a `jr $ra` in the middle of
Execute's switch. The real function starts after a thread's endless
loop, which `entries()` does not list.

- **The ref pass** now skips a start whose code branches below it. No
  function does that. This also drops the `__sinit_system.cpp` rows on
  all three later volumes, which were a string table.
- **A new prefix pass.** An Infection function not named yet is placed
  when its first 24 masked words are shared by no other Infection
  function and open exactly one unnamed start in the volume. The starts
  include those after an endless `b` loop. It names six functions on
  Mutation (`ccThEvHold`, `ccPlayerStart` and others) and five on each of
  Outbreak and Quarantine.
- Adding the loop starts to `entries()` itself cost the order pass about
  330 names, so they stay local to the prefix pass.

**Still unknown:** The port keeps 750 gimPos slots. With fewer
than ten floors, Mutation searches only the first 75 per floor for a free
one. What the new ccAI hit fields drive beyond `item_first` is unread.
