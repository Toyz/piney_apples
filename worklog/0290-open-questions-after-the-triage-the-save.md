---
number: 290
title: Open questions after the triage: the save
date: 2026-09-28
area: save
files: UNKNOWNS.md
resolves: 16, 24, 44, 45, 50, 76, 159, 201, 246, 254
---

# 290. Open questions after the triage: the save

This entry closes the open questions of 10 entries about the save: the
memory card format, the new game, the title's load and what carries between
volumes. The triage of 2026-09-28 (UNKNOWNS.md) split every entry's open
paragraph into single questions and checked each one against the later log,
the docs and the code. The answered ones are listed below with their
evidence. What is still open is restated at the end, with the entry that
asked it, the play questions first. Open questions of these entries that
belong to another subsystem are restated in that subsystem's entry: [[288]],
[[289]] and [[291]].

## Answered

- [[16]]: what `mcFname[0]` (`BISLPS-00000HUCKER`) is for — answered by
  docs/formats/save.md (nothing uses it; only `mcFname+4` is referenced)
- [[16]]: the `icon.sys` fields the game fills in — answered by
  docs/formats/save.md (the title from `mcTitleName`, the break at byte 10)
- [[16]]: which of the twelve icons each volume uses — answered by
  docs/formats/save.md (volume v uses `iconBinTbl` 3(v-1) to 3(v-1)+2)
- [[16]]: the meaning of most event-flag bits — answered by [[40]] /
  docs/engine/events.md (bit 62 done, 63 closed); [[246]]
- [[16]]: whether `ConvGame` is reachable — answered by [[24]] /
  docs/formats/save.md (`ccThDemo` calls it when the later titles' opening
  returns 4)
- [[16]]: how the later volumes read Infection's saves — answered by [[24]],
  [[200]] / docs/formats/save.md (`LoadInfoPrevReq`, `LoadDataPrevReq`)
- [[24]]: what the Mutation-on blocks at `+0x8432` and `+0x8462` hold —
  answered by [[246]] (zeroed by `Init`, copied, never read)
- [[24]]: the extension's 3 bytes at `+0x748` — answered by [[200]] /
  docs/formats/save.md (`pcTradeCount` of characters 18-20)
- [[24]]: the extension's 256 bytes at `+0x754` — answered by [[246]]
  (cleared and copied, nothing else)
- [[24]]: the save bit at `+0x5ec8` bit 62 — answered by [[220]]
  (`eventFlag[314]` bit 62: event 314 done, the random-path dungeon flag)
- [[24]]: what the ending routine does while the `KFED` buffers are loaded —
  answered by [[239]] (gcmn 0x004f0a30 starts `STFROLL_VOL4` and waits)
- [[44]]: `NewGame(0)`'s copy of `charTbl` into `spcParam` — answered by
  [[76]] (`tools/test_save_init_rs.py`)
- [[44]]: the setup screen and name entry during event 1 are black —
  answered by [[45]]
- [[44]]: the title's music, the PSS movies, the intro stream, The World —
  answered by [[46]], [[49]], [[52]], [[58]], [[59]]
- [[50]]: `LoadGame`'s vibration — answered by [[232]]
  (`piney_input::actuator`)
- [[50]]: `LoadGame`'s camera scheme — answered by
  `crates/piney-world/src/camera.rs` (`camera_mode` from the save) and the
  `CameraType` requests applied (`crates/piney-game/src/world.rs`,
  `area.rs`); [[254]]
- [[76]]: per-frame `CalcReal` only in Mac Anu — answered by BUGS.md
  (`kites_buff_times_out_in_town`; the field's frame ran it already),
  `crates/piney-game/src/world.rs` `calc_real_party`
- [[76]]: `Init`'s fields not repaired in old saves (`townMoveFlag`,
  `partyMemberCall`, tactics, `cameraMode`, `drainDemo`, idol ranks) — no
  longer applies: it concerned the port's own saves made before [[76]]'s
  fix, not the game
- [[76]]: what `partyMemberCall` 0x3fffe, tactics 7, `drainDemo` and growth
  are for — answered by [[246]] / docs/formats/save.md
- [[201]]: Outbreak's and Quarantine's titles not run — answered by [[204]]
  (all three later discs reach their desktops)

**Still unknown:**
- (play) [[43]], [[201]], [[226]]: CONVERT, a previous volume's save into a
  new part: `Data_Control`'s `m_PrevFlg` load screen reads the card, but
  `Request::ConvGame` only reaches "title asks" in
  `crates/piney-game/src/session.rs`, so nothing is imported
  (docs/engine/title.md) — starting Mutation, Outbreak or Quarantine from
  the previous part's clear data.
- [[246]]: what each `eventStatus` index counts; it belongs to the scripts
  that use it.
- [[193]]: what reads the save's portal counts (`+0x6864`, `+0x6866`,
  `+0x6868`) (docs/engine/effects.md Unknown).
- [[45]]: `save_va`: the real heap address `NewGame` stores as character 0's
  name pointer.
- [[50]]: whether `ccSaveSys` keeps its card position through a reset.
- [[70]]: the Recorder's card position follows the title's Load, not a later
  desktop Data save (the cursor's start only).
- [[45]], [[73]], [[159]]: real disc and card times: the port's card calls
  finish at once, so the card-busy frames and messages never show
  (docs/engine/desktop.md Unknown).
- [[254]]: whether a player wants a save's own options to win when loading a
  card made elsewhere (a port design choice).
