---
number: 300
title: Party members' skill words: not reproduced, the port's path is the game's
date: 2026-09-28
area: audio, battle
files: crates/piney-audio/src/driver.rs, crates/piney-game/src/area.rs, crates/piney-world/src/combat/stage.rs
---

# 300. Party members' skill words: not reproduced, the port's path is the game's

Reported in play (INF, through the launcher): Kite names his skills, but
party members (BlackRose, Orca, Mistral) using skills say nothing. No code
changed: every path tried speaks, and each step matches the game.

## The game's rule

- **One caller.** `ccWordsPlay(sid, ch)` (INF SLUS_202.67:0x0017e290) is
  called only by `_ccSkillRequest` (INF GCMN.PRG:0x00572b98), for stype 0
  or 2 and ids 6 and up. `ccSkillRequest` (0x00572700) and the item
  wrappers (0x00572754, 0x005727b4, 0x005727f8, 0x00572834) all reach it,
  so the menus, the party AI (`ccFellow::UseSkill` 0x0041e470,
  `ccFellow::Attack` 0x0041e590) and the enemies name skills alike.
- **The queue.** It is refused while an event runs (`eventMng` +0x78c),
  for `base->type & 5` clear, or for ids 304 and up; else (`base` +0x0c,
  sid) goes into `ccSnd` +0xd0.
- **The line.** `skillVoicePlay` (0x0017e350) takes the first entry:
  - the file is `spcVoiceData[c]` (0x00307230; `spcVoiceDataE`
    0x003072d0);
  - the rows are `voiceData[c]` (0x003071e0, `voiceKiteTbl` ..
    `voiceHerubaTbl`; `voiceDataE` 0x00307280);
  - the row is the sid less a base, by the jump table at 0x0034d830 and
    bit 0 of `ccGetSkillParam(sid)+0x2c`;
  - `sewordCmd(0x80e0)` sends it, and the queue empties.

## What was checked

- **The data.** `charTbl` gives type 0x6 to every member (Kite 0x7), so
  `& 5` passes. The port's row rule is the jump table case by case. Every
  row of the Kite, Orca, BlackRose and Mistral tables has a line in both
  languages.
- **The session.** A member's cast reaches `Event::SkillWords` with its
  `charTbl` row. It was tried three ways:
  - CHAT's Designate Skill (Orca, Repth);
  - the AI's own arts after `RequestChatCmd` 13 (Orca, ids 33 and 34);
  - its spells after 14 (Mistral, id 209).
- **The sound.** Fed to a headless `Audio` through `main`'s `handle` with
  the real event history, both Kite's Repth and then Orca's were heard
  (rms up to 7716). All 18 characters' lines play through the engine.
- **During an event.** Casts during event 3's lesson are silent for Kite
  and Orca alike (`puppetShow`), as the game's +0x78c check says.

The probes are kept at /mnt/data/claude/scratch/voice (`probes.diff`,
`scratch_words.rs`), not in the tree.

## A difference that does not explain it

A row the rule puts outside a character's table is handled differently:
- the game reads the memory beside the table, sends that, and empties the
  queue;
- the port returns and keeps the word, so it is tried every frame and
  every later word is lost.

The ids that land there are other classes' arts (6-59 for BlackRose, 6-32
for Orca) and 296-303 (297-303 for Kite). No INF item uses one: the
highest item skill is Recovery Drink's 295. That silence would also take
Kite's words.

**Still unknown:** the play that silenced the members (area, party,
strategy, save), needed to reproduce it; and what the game sends for an
outside row, which would need `voiceData`'s neighbours as memory.
