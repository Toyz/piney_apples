---
number: 101
title: The fights' voices: a skill request names its skill, and what else speaks in a fight
date: 2026-09-25
area: audio, battle, test
files: crates/piney-world/src/combat/stage.rs, crates/piney-world/src/combat/mod.rs, crates/piney-game/src/area.rs, crates/piney-game/src/mode.rs, crates/piney-game/src/session.rs, crates/piney-battle/examples/battle_probe/main.rs, tools/test_battle_rs.py, docs/engine/sound.md, docs/engine/battle.md
---

# 101. The fights' voices: a skill request names its skill, and what else speaks in a fight

Worklog 95 ported the voice side in piney-audio:
- the skill-words queue (`ccWordsPlay`);
- its player (`skillVoicePlay`);
- the field's voice groups (`ccVoiceRequest`).

What was left was to raise the calls from the fights.

## The skill words

**One caller.** `ccWordsPlay` is called from `_ccSkillRequest` only (gcmn
0x00572b98). The call comes after the caster's `skillID` and
`skillStatus` are set, for stype 0 or 2 and ids 6 and up:
- stype 0 is a character's own skill: the menus, the party AI and the
  enemies;
- stype 2 is an item used through its user;
- stype 1, an item's effect on the target alone, names nothing.

The enemies' calls reach the queue and are dropped there by its own check
(`base->type & 5`).

**Already in piney-battle.** The rule is `skill::Request::words`, and
`test_battle_rs`'s `request` check already compared whether
`ccWordsPlay` is called. That check now compares its arguments too: the
skill, and the caster (`a0` the sid, `a1` the caster). 2000 cases, 0
mismatches; in the test's own seed, 833 of 2000 call it.

**The wiring.**
- **piney-world.** `stage::skill_shows` shows a request's start and, when
  the request calls `ccWordsPlay`, a new `Show::Words { who, sid }`. It
  replaces the five places that pushed `Show::SkillStart` by hand:
  - the menus and the action button (`Combat::skill_request`);
  - the party AI's `Call::SkillRequest`;
  - the enemies' `Call::SkillRequest`;
  - the items' skills (`item_skill`);
  - the traps' skills.
- **piney-game.** The area's `battle_sounds` turns it into
  `Event::SkillWords` with:
  - the caster's `base->type`;
  - its `charTbl` row (`base` +0x0c);
  - bit 0 of the skill row's +0x2c;
  - `puppetShow`, the `eventMng` +0x78c the queue checks.

  `main.rs` already routed that to `Audio::words_play`. The
  `#[allow(dead_code)]` on the variant is gone.

**The session test.** `a_skill_in_a_fight_names_itself`:
- after event 3, at the east portal with the goblin out and the battle
  mode on, Kite opens PERSONAL, then Skills, and uses the third page's
  first skill (the lesson's Repth) on a party member through TARGET;
- the session hears `SkillWords` with Kite's type (5) and row (0), a
  skill id of 6 or more, the skill's type bit, and no event running;
- a piney-audio `Driver` fed it sends Kite's line for that skill
  (`skillVoicePlay`: a `Command::Voice`).

## What else speaks in a fight

**The voice groups.** No code of the ordinary fights calls
`ccEvVoiceRequest` or `ccVoiceRequest`. Their callers are:
- the message window (the events' lines);
- the gate tutorial (`GtNewMenuT`);
- the Grunties' growth (`ccPGuso::evoAct*`);
- one boss: Skeith's prediction (`ccBoss04::OnThinkPrediction`: group
  -40, Fidchell).

The bosses are not ported. The Kyvia bosses' `DeadKyviaVoice` and
`kyviaCore::CoreVoice` are sound effects (`ccSeOn3DNote`).

**Kite's strategy call** (`Show::Shout`, `ccSpcShoutOperationName`) speaks
no line either. It opens a chat balloon over Kite: `ccChatMsg::OpenChat`
with `chatActionStr` piece 5, the strategy's `chatMenuStr` piece, then
piece 6. The chat balloon (`ccChatMsg`) is not ported anywhere, so
`Show::Shout` still draws nothing. It belongs with the party's chat lines
(the queued "chat lines" item).

## Checked

- `tools/test_battle_rs.py` passes, and so does its `request` check in
  bulk: 2000 cases, 0 mismatches.
- The other suites: `test_battle_items_rs` and `test_battle_frame_rs`.
- The workspace's tests, the new session test, clippy, fmt and the docs
  check.

**Still unknown:**
- **A frame late.** A request the menus make reaches the sound task a
  frame after the game's would. The world's shows are taken before the
  menu task runs, so the word is queued the next frame.
- **The balloon.** Kite's strategy balloon and the party's chat lines
  (`ccChatMsg`) are not ported.
- **The type bit.** What bit 0 of a skill row's +0x2c means is not named
  (worklog 95).
