---
number: 479
title: Twenty-two play questions were already answered: the triage's list against the work since
date: 2026-10-25
area: test
resolves: 283.5, 285.1, 286.2, 286.3, 284.5, 284.6, 284.7, 288.1, 288.2, 288.3, 288.4, 288.5, 289.1, 289.2, 289.3, 290.1, 291.4, 291.5, 291.6, 291.7, 291.8, 291.9
started: 2026-10-25T16:06:15-06:00
took: 3m
---

# 479. Twenty-two play questions were already answered: the triage's list against the work since

`cairns open` lists 40 questions marked (play), all carried by the
triage entries [[282]] to [[291]] of 2026-09-28. Twenty-two of them were
answered by work done since. Those entries never named the question they
closed, so it stayed on the list. Each one below was checked against that
entry and against the code as it stands:

| question | answered by | where it holds |
| --- | --- | --- |
| 283.5 the gate menu's white flash in town | [[326]] | the flash drawn on the menu's fader |
| 285.1 `come_back` builds a lake room's doors open | [[343]], [[379]] | `ccCheckActiveObject(0, lastRoom)` (gcmn 0x0042e010) answers 1 there, so the game builds them open too |
| 286.2 Kyvia's gomoras not pushing, the pad's sway, the arm, the particles | [[427]], [[426]], [[420]] | `kyviaGomora::Move`'s `CollisionDetection`, `Pad_Control`'s `ZrotControlFlg`, `HandAtk`'s `DrawParts`, the five generator tables |
| 286.3 Magus's beams to the ground, `EntryFlash2`/`3`, the grow's loop | [[421]], [[428]] | `ccHitCheckLM` on the land, both flashes on the fader, `ccSeOn3DLoop` kept for `ccSeOffLoop` |
| 284.5 Innis's pictures | [[411]], [[414]] | missiles, rings, shards, particles, the shield; the square for each n |
| 284.6 Magus's pictures | [[421]] | markers, charges, bursts, needles, smoke, thunder |
| 284.7 Innis's ice missile, Fidchell's `IceBreak` | [[411]], [[414]], [[294]] | Absolute Zero's ice; `OnIceBreak` (OUT gcmn 0x004a6330) makes `EffKind::IceBreak`, the ice rocks |
| 288.1 the epitaphs and the book viewer | [[292]], [[390]] | `useitem.rs` carries `Step::Epitaph` and `Step::BookStart` |
| 288.2 the map button's waits, no map in a fight | [[341]] | `Targeting::map_test` after `menuClrWait`; `DrawMiniMap` only while `inBattle` is 0 |
| 288.3 Adjust Screen | [[339]] | `SetDisplayOffset` moves the picture |
| 288.4 a member's equipment remark in town | [[337]] | `ChangeEquipReport` in town |
| 288.5 the opening's cancel flash | [[342]] | the cancel's and the end's flashes over the stream |
| 289.1 `SoundFadeOut` cut the music | [[321]] | `ccSoundFadeOut`'s 8 frames |
| 289.2 the gate hack's cancel | [[316]] | `ccSndGateHackCtrl`'s state 2: `ccSqPlayVol(0, 0)` and the fade up |
| 289.3 the Voice option in The World | [[322]] | the game reads the save at each line; the port sends the options before each |
| 290.1 CONVERT | [[387]] | `ConvGame` makes the previous part's saves the new one's |
| 291.4 Mutation's Flag Race | [[394]] to [[401]] | the race played as a player rides it |
| 291.5 Kyvia's levels 2-5 and the EX mode | [[304]], [[433]], [[434]] | docs/engine/boss-kyvia.md: nothing open |
| 291.6 `ccBoss04`'s Affect and AI | [[294]] | Fidchell, checked against Outbreak's code |
| 291.7 the later bosses' `effSmoke` | [[295]], [[420]], [[421]], [[443]], [[457]] | Fidchell's, Kyvia's, Magus's, Macha's and Corbenik's back dashes and drains |
| 291.8 `EVENTAREA04`-`06` | [[472]] | field 66 its own map |
| 291.9 `EVENTAREAB0`'s `NextStage` | [[454]] | field 8's last stage |

What the answering entries still leave open stays on their own lists. The
boss blur (`ccBossBlur`, [[117]], in [[411]]'s and [[421]]'s) and the
radiator light's look on a PS2 ([[377]]) are among those. The rest of the
40 are worked one by one from here.

**Still unknown:** nothing
