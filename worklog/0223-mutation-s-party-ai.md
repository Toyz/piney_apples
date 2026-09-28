---
number: 223
title: Mutation's party AI
date: 2026-09-27
area: volumes
files: crates/piney-battle/src/party_ai.rs, crates/piney-battle/src/affect.rs, crates/piney-battle/src/event.rs, crates/piney-battle/src/kite.rs, crates/piney-battle/src/tables.rs, crates/piney-world/src/combat/chat.rs, tools/volume.py, tools/test_battle_party_ai_rs.py, tools/test_battle_rs.py
---

# 223. Mutation's party AI

`tools/test_battle_party_ai_rs.py` passes all 75 of its checks on
Mutation: Infection's 73 and two for functions Mutation adds. Infection
still passes all of its own. The rework of `piney_battle::party_ai` took
five commits. Each Mutation rule sits behind `self.t.volume`, so
Infection's code paths are as they were.

## ccAI's new fields

- **+0x24, the attack command's target** (`Ai::cmd_attack_target`).
  `ChatCommand` copies `targetCCmd` into it when it accepts command 11.
  `ChatCommandExecute` (11) and `Reconnoiter` (strategy 4) follow it
  ([`Ctx::attack_cmd_target`]). `targetCCmd` stays at +0x20 and still
  takes every request's target. Worklog 221 and commit 975c083 put the
  new int at +0x20; `RequestChatCmd`'s store shows it is +0x24.
  `volume.ai_at` now moves Infection's offsets from +0x24 on.
- **+0x9a and +0x9c, the biggest recent hit and the biggest hit**
  (`hit_recent`, `hit_max`). Both `Influence` functions pass each hit to
  an unnamed function (gcmn 0x005c1a60, `Ai::note_hit`). It caps the hit
  at half the member's maxHP and raises both fields. A party member's
  hit is recorded even when it is down; Kite's only when he is up. The
  rule emits `Event::NoteHit`, which the field's chat pass applies.
  `ChatCommand` clears both at a win. Kite's `ReadSysMsg2` clears the
  recent one and `battleFlag`.

On later volumes the harness appends the three fields to `AI_LAYOUT`,
and the probe reads and writes them after the message entry.

## Modes, strategies and swayed members

- **Mode 4 is new: hold.** Infection's modes 4-6 become 5-7 (`mode_of`,
  `Ctx::mode_no`, `Ctx::inf_mode`). A held member acts unless a boss is
  in (`CheckBossEntry(1)`).
- **Strategy 6 is a standing hold.**
  - Most requests turn it back into 3 first.
  - The spells command (14) makes a standing 3 (or 0 or 2 while the
    strategy is 3) into 6.
  - In battle, command 14 with strategy 6 stops the member and holds it.
  - A refused command turns 6 back into 3, with `skillMask` 4 and
    `selfFlag`.
- **Charmed or confused members.**
  - `Reconnoiter` searches for a foe first.
  - `SearchTarget` takes Kite's target only on the same side.
  - `SearchTargetNear` has no range limit for the party.
  - In a field, a member closing in for a commanded skill drops the
    command and fights.
  - A member in battle mode that loses its target stops, in a dungeon
    too.

## Healing and cures

- **The heal line is a rate** (`CheckNeedHealing` and `CheckHealParty`
  take a float, as `fptosi(maxHP * rate / 100)`).
- **Notes are due sooner.** Heal, cure, revive and buff notes are due in
  30 frames on the member's first action, then in 1 (`plan_delay`).
- **Items before skills.**
  - `HealPlzNormalMode` and `HealPlzBattleMode` ask a new function
    (0x005c1ae0, `Ctx::item_first`) whether items come first.
  - It answers -1 when the member cannot act, and 1 when it stands alone
    (0x005c8300, `Ctx::live_members`).
  - It also answers 1 when every other member is unable, or when every
    other member is in danger: HP at most its recent hit, or a tenth of
    its maxHP.
  - `HealSPC`, `ResurrectSPC` and both `CureOnly` functions take the
    answer.
- **Poison and curse.** The normal mode's poison cure is a new function
  (0x005ac400, `Ctx::cure_poison_curse_spc`). It also cures curse
  (skill 162).
- **Battle-mode order.** Paralysis and sleep are cured before the heal.
- **`ChatCommandHealPlz`.**
  - A member that has acted looks every tenth frame.
  - A held member heals too.
  - The member must be free to use a skill or an item.
  - Out of battle the strategy comes back only once no one is below
    full HP.
- **`ReadSysMsg`, kinds 2-0xb** (`Ctx::read_use_later`).
  - A use is skipped unless the patient needs it. A revive needs one
    down (2-4). Any other use needs one alive; a heal also needs one below
    full HP, and a cure needs the condition.
  - Item uses announce nothing here (`UseItem` remarks instead).
  - After a buff or debuff use only a debuff command (17) is done.
  - Kind 0xc and broadcasts 23 and 24 do nothing.

## Buffs and debuffs

- **The planners take a check argument**: 0 plans; 1 reports; 2 reports
  on the SP alone. They also keep a candidate list (global 0x774730).
- **`ChatCommandBuffPlz` and `ChatCommandDeBuffPlz`**
  (`Ctx::chat_command_plz_later`).
  - They look every 35th count, a held member too, and only when it is
    free to use a skill or an item.
  - Out of battle, with nothing planned, the member goes back to its
    strategy only if a check finds no one to help. The checks are
    0x005c2060 and 0x005c1cc0 (`Ctx::plz_check`).
  - For buffs, no buff for the member may be on the bus or delivered in
    the last 37 frames (0x005b42c0).
  - It then says `OnlyBuff` or `OnlyDebuff`.
- **`ChatCommandFulfilCheck`.**
  - The buff and debuff commands are refused with that remark when the
    check finds no one and none is on its way (0x005b4020, 0x005b4170).
  - The heal command is taken in battle even when no one needs a heal.
  - The arts-and-spells command (0) accepts attack-spell items.

## Commands

- **`RequestChatCmd`** writes the target to `targetCCmd` as Infection does.
  The arts commands (0, 13) keep a standing strategy 1 or 4.
- **`ChatCommand`** has no "same command again" path. Every new command
  goes through `ChatCommandFulfilCheck`.
- **`ChatCommandExecute`.**
  - Commands 5 and 99 take `targetCCmd` once and clear it.
  - The Sprite Ocarina's last use says `UseLastItem` instead of
    `UseOcarina`.

## Fixes on the way

- **`piney_battle::tables`.** `Tables` held `debuffTableIndex` and
  `buffTableIndex` as `[i16; 18]`, and `try_into().unwrap_or_default()`
  turned Mutation's 21 rows into zeros. They are slices now. The other
  fixed-size tables `expect` their size, and a test loads all four
  volumes.
- **The harness's bus.** Random messages of kind 6 now carry a skill id,
  and kind 7 an item code. The game read an item code as a skill and ran
  off RAM in `ccSkillCheckType`.
- **The harness's entry field.** It is found by name (`AI_AT["entry"]`),
  no longer as the list's last field.

**Still unknown:** The game's windowed history walk (0x005b42c0) never
moves past a delivered item use whose item casts nothing; the port passes
it over instead of hanging. `Ctx::item_first` reads a member without an
AI as having no recent hit. The game reads through that null pointer.
Who sends message kind 0xc and broadcasts 23 and 24 is unread. The
fellow, kite, event, navi and spawn harnesses have not been rerun on
Mutation since this work.
