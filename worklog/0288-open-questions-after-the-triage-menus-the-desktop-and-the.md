---
number: 288
title: Open questions after the triage: menus, the desktop and the text
date: 2026-09-28
area: ui
files: UNKNOWNS.md
resolves: 9, 23, 37, 41, 43, 51, 58, 64, 66, 68, 70, 71, 73, 77, 94, 96, 98, 104, 129, 135, 144, 174, 179, 183, 186, 190, 193, 194, 196, 200, 210, 237, 251, 253
---

# 288. Open questions after the triage: menus, the desktop and the text

This entry closes the open questions of 34 entries about the menus and
screens: the desktop and its text, the title, the top page, the field UI
(the HUD, the menus, the minimap), the noise, the staff roll and the
launcher. The triage of 2026-09-28 (UNKNOWNS.md) split every entry's open
paragraph into single questions and checked each one against the later log,
the docs and the code. The answered ones are listed below with their
evidence. What is still open is restated at the end, with the entry that
asked it, the play questions first. Open questions of these entries that
belong to another subsystem are restated in that subsystem's entry: [[283]],
[[284]], [[285]], [[286]], [[289]], [[290]] and [[291]].

## Answered

- [[9]]: how the game decides which mails a volume delivers — answered by
  [[41]] / docs/engine/desktop.md "Mail" (event opcodes 107-109; event 1
  delivers 4, 5 and 320)
- [[9]]: what `ccGetExtendedCode` maps and how the `%` codes are drawn —
  answered by [[23]] / docs/engine/font.md
- [[9]]: the BBS `dateindex` — answered by [[58]] / docs/engine/toppage.md
  (the index of the date cell: 0-5 plain, 0 in parody)
- [[9]]: whether Parody Mode can be switched on in the US Infection —
  answered by [[43]] (`m_ParoFLG` is only cleared; it cannot be chosen)
- [[23]]: what Quarantine loads `KFED` for — answered by [[239]] (read into
  two buffers, never read)
- [[23]]: the line spacing of mail and board bodies — answered by [[37]] /
  docs/engine/desktop.md "Mail" (body lines at 48 + 17 r)
- [[23]]: whether the text code changed in volumes 2-4 — answered by [[25]]
  (Outbreak's `ccKanjiStrlen` counts every `%x` pair)
- [[37]]: the desktop's other modes: News, Accessory, Audio, Data — answered
  by docs/engine/desktop.md "News", "Accessory", "Audio", "Data"
  (`crates/piney-desktop`)
- [[37]]: the event scripts that deliver mail and lock icons — answered by
  [[40]], [[41]]
- [[37]]: the desktop's sound printed, not played — answered by [[42]],
  [[46]]
- [[37]], [[41]]: `DEMO.PRG`'s title and opening, name entry, `TOPPAGE.PRG`
  — answered by [[43]], [[45]], [[58]]
- [[41]]: the scripts' windows on the desktop — answered by [[45]] (the
  setup stage draws the event's windows)
- [[41]]: name entry (`NameEntry_Control`) — answered by [[45]]
- [[41]]: the script's movie streams and overlay loads skipped — answered by
  [[49]], [[52]] (streams), [[91]] (the desktop's overlay and pass)
- [[41]]: what the setup screen shows during event 1's 59 frames — answered
  by [[45]]
- [[43]]: the title's load screen and Option menu stand-ins — answered by
  [[50]], [[51]]
- [[43]]: PSS movie playback and the stream — answered by [[49]], [[52]]
- [[43]]: the title music context — answered by [[46]], [[51]]
- [[43]]: `SetNextData` and `PlayNextData` — answered by [[201]] /
  `crates/piney-demo/src/opening.rs` `set_next_data`
- [[51]]: the port's all-sound-off cutting only the voices — answered by
  BUGS.md (the dungeon music under Helba's theme;
  `all_sound_off_stops_the_music`)
- [[58]]: the field game (modes 5 and 6) — answered by [[59]]
- [[64]]: nothing calls `FieldUi` yet — answered by [[66]]
- [[64]]: the Chaos Gate's menus (28, 57-62, 78, 79) — answered by [[68]],
  [[96]] (62, the gate hack)
- [[64]]: the talk, trade and shop menus — answered by [[70]], [[73]]
- [[64]]: the other tutorials — answered by [[68]], [[75]]
- [[64]]: the Key Items, Status, Equipment, PARTY, Gate Out, Log Out and
  OPTION pages — answered by [[71]]
- [[64]]: Data Drain's own menus — answered by [[102]]
- [[64]]: the field objects' menus — answered by [[98]], [[148]]
- [[64]]: `ccThGameCtrl` (`cmndTarget`, menus from buttons) — answered by
  [[66]], [[77]] (the map button)
- [[64]]: damage numbers — answered by [[79]]
- [[64]]: the chat balloon's drawing — answered by [[120]]
- [[64]]: `ccNoiz` — answered by [[104]]
- [[64]]: rotated sprites drawn unrotated; the drain gauge's Gouraud cells —
  answered by [[144]]
- [[66]]: the menu buttons not run in eemu — answered by [[80]]
  (`tools/test_gamectrl_rs.py` `frames`: `ccThGameCtrl` with its buttons)
- [[66]]: the menus' target requests (`TargetFix`, `Target`, `TargetClear`)
  dropped — answered by [[70]] / `crates/piney-game/src/world.rs`, `area.rs`
  apply them
- [[66]]: the talk, shop and gate menus and the map button — answered by
  [[68]], [[70]], [[77]]
- [[66]]: PERSONAL's last row lost the L of Log Out — answered by [[68]]
- [[68]]: the gate hack (62) — answered by [[96]]
- [[68]]: `GoToArea` and `ChangeArea` go nowhere — answered by [[72]]
- [[68]]: `TransferOut` dropped — answered by [[129]] (towns run
  `effTransfer`)
- [[68]]: the AI orders dropped — answered by [[148]] (`ChatMenu1`-`3`)
- [[68]]: Party Add made no character — answered by [[74]], [[90]], [[108]]
- [[70]]: the pages still to come: Trade (48, 49), 21, 23, 27, 56 and 50 —
  answered by [[73]]
- [[70]]: `SetMerchantCamera`'s `changeCamera(3)` and `(1)` not carried out
  — answered by [[76]] (the merchant's camera; `tools/test_merchcam_rs.py`)
- [[71]]: the run-time's new-game save (skill list zeros, stats 0) —
  answered by [[76]] (`ccSaveData::Init`, `NewGame`, `InitSpcParam`;
  `tools/test_save_init_rs.py`)
- [[71]]: what `bootParam` means beyond Add's greeting — answered by
  docs/engine/battle.md "The party's `bootParam` from area to area" ([[74]],
  [[80]])
- [[71]]: what `SetActuater(1, 160, 200)` means — answered by [[232]] (the
  small motor, power 160, 200 ms, the queue)
- [[71]]: where `drainDemo`, `voice` and `strWinMode` are read — answered by
  [[105]] (`drainDemo`), [[89]] (voice), [[85]] (`strWinMode`, the
  subtitles)
- [[71]]: still unported: 62, 66 and 67, 71-73 — answered by [[96]],
  [[102]], [[148]]
- [[73]]: a member's book use (`SpcUseItem`) and `PresentOther` dropped —
  answered by [[196]]
- [[73]]: feeding a Grunty (`Feed`, `GruntyGrowth`) and `World::grunty` —
  answered by [[162]]
- [[73]]: `CalcReal` and `ChangeEquip` dropped — answered by [[76]]
  (per-frame `CalcReal`), [[187]] (equipment changed in a field stays)
- [[73]]: `set_spc_base_msg` not called — answered by
  `crates/piney-fieldui/src/newgame.rs` (called by `InitSpcParam`'s port)
- [[73]]: nothing gets Kite to menus 21, 23, 27, 50 and 56 in Mac Anu —
  answered by [[108]], [[110]] (members in town), [[90]]
- [[77]]: portals, gimmicks and fountains not fed into the map; the Fairy's
  Orb's yellow areas — answered by [[164]], BUGS.md
  (`a_fairys_orb_shows_the_portals`)
- [[77]]: what sets `mapHideFlag` — answered by docs/engine/map.md (the
  constructor, `RoomSelect`, the stairs in `GotoNextRoom`)
- [[77]]: Dun Loireag's `DrawMap` — answered by [[97]]
- [[96]]: the hacked arrival (`GateHackingOut`) — answered by [[106]]
- [[96]]: the gate hack's noise (`ccNoiz`) — answered by [[104]]
- [[96]]: `gate_hack_anim` waits on `ghoFlag` — answered by [[106]]
  (`ghoFlag` set by the constructor, cleared by `GateHackingOut`)
- [[98]]: the doors' palettes for `clutType` 3 and 4, and their sound ids —
  answered by [[147]], [[138]]
- [[98]]: `effOpenTrapBox`, `effStatueOfGod`, `effVirusCrystal`,
  `effCrush*`, the idol's dust ring — answered by [[116]], [[118]];
  `effVirusCrystal` from `ccRtownPC::eventMode` remains, in the volumes'
  entry
- [[98]]: `EntryBreakObject` — answered by [[140]]
- [[98]]: the objects' menus 34-37 and 39, `DataDrainMenu` (66) — answered
  by [[148]], [[102]]
- [[98]]: the map item step (`WaitMap`) — answered by BUGS.md
  (`a_fairys_orb_shows_the_portals`)
- [[98]]: the Grunty ride item step — answered by [[166]]
- [[98]]: `DisableThEvent` — answered by `crates/piney-game/src/world.rs`,
  `area.rs` (`ccDisableThEvent`)
- [[98]]: the party AI's `UseItemRequest` does nothing in the field —
  answered by [[230]]
- [[104]]: `ccGameOverNoise` — answered by [[158]]
- [[104]]: which town is `game.town` 4 — answered by BUGS.md (the console's
  town 4 is Lia Fail), [[260]]
- [[129]]: the black screen in the church — answered by [[174]], BUGS.md
  (story 11's set-up windows)
- [[129]]: a merchant's `sysopeAct` transfer — answered by
  `crates/piney-world/src/merchant.rs` `sysope_act`, [[132]]
- [[129]]: the stream effects not checked (`tools/test_effect_str_rs.py` to
  be written) — answered by [[131]]
- [[135]]: the fountain's `FountainMenu3` pair — answered by [[163]]
- [[135]]: `ccClearSpcCondition`, which the bosses' deaths call — answered
  by [[192]]
- [[183]]: "Nothing new." — no longer applies: the entry states no question
- [[196]]: the pad's vibration in town — answered by [[232]]
- [[200]]: the later titles' menus (five items) — answered by [[201]]
- [[200]]: the desktop's tables read with Infection's row counts — answered
  by [[209]] (typed per-volume tables)
- [[200]]: what reads the blocks at `+0x8432` and `+0x8462` and the
  extension's last 0x100 bytes — answered by [[246]] (nothing)
- [[210]]: six trade and shop harness cases fail (`errorData`) — answered by
  [[214]]
- [[210]]: the world, areas, streams and announcements still read the
  executable — answered by [[215]], [[217]]

**Still unknown:**
- (play) [[98]]: the epitaphs (`ccEpitaphMsg`, `Step::Epitaph`) and the book
  viewer (`ccThBook`, `Step::BookStart`) are not carried out:
  `crates/piney-world/src/field_world.rs` `item_step` returns false and
  `crates/piney-game/src/area.rs` logs "item step not carried out" — using
  those items shows nothing.
- (play) [[77]]: the map button is not held by `menuClrWait`,
  `ccCheckGtHackAnm` or a wiped-out party, and the field and dungeon map is
  not hidden while `ccGame.inBattle` (docs/engine/map.md; `area_frame` in
  `crates/piney-world/src/map` has no battle test; check whether
  `ccThMenu`'s map alpha already hides it) — the minimap in fights.
- (play) [[50]], [[51]], [[196]]: Adjust Screen's display offset
  (`Request::DisplayOffset`) is dropped (`crates/piney-game/src/desktop.rs`,
  `world.rs`) — the Screen option does nothing.
- (play) [[67]]: a member's equipment remark in town (`ChangeEquipReport`)
  is dropped (`crates/piney-game/src/world.rs`) — equipping a member in
  town.
- (play) docs/engine/title.md: the opening's cancel flash is not modelled —
  skipping the opening stream.
- (volumes) [[94]]: the later volumes' staff rolls (`stfroll2`-`4`) are not
  tried; GAPS.md lists Quarantine's staff roll as its own.
- (volumes) [[276]]: Mutation's staff roll, and what the desktop does after
  it (the save for Outbreak's carry-over).
- [[9]]: the meaning of `mailFlg` and `reFlg`: their writes are ported
  (docs/engine/desktop.md "Mail"), their readers are unnamed
  (docs/engine/text.md Unknown).
- [[23]]: how the GS samples the kt 0 and kt 2 sprites, whose `sx` and `su`
  differ by a texel; the renderer assumes top-left sampling
  (docs/engine/font.md Unknown).
- [[23]]: which view and scale the font layer uses (docs/engine/font.md
  Unknown).
- [[23]]: where the `ccKanji` texture sits in VRAM; the port re-uploads one
  scratch page per kanji (docs/engine/font.md Unknown).
- [[120]]: the balloon's text is drawn through a 128-row `Init(3, 24)`
  texture, where the game's chat kanji is `Init(2, 16)`; one line looks the
  same.
- [[45]]: the kana, symbol and kanji grids of name entry, not reachable in
  the US build.
- [[251]], [[253]]: whether the unused selector had music, or drew a
  copyright, of its own; no code survives.
- [[77]]: what sets `WORLD_MAN.specialRoom` (`+0x160`) beyond the known
  rooms (docs/engine/map.md Unknown).
- [[77]]: `RT01ICONPOS` and the pulse angles are the game's globals; the
  port starts a new area's pulse at 0 (docs/engine/map.md Unknown).
- [[144]]: a rotated cell's off-screen test reads a leftover width and
  height; the port takes 0 (docs/engine/map.md Unknown).
- [[144]]: the menu's turn uses `f32` sine and cosine, not the VU's; a
  sixteenth of a pixel at most.
- [[73]]: `PresentMenu`'s states 20-22 and `BreedingMenu`'s 23 with index 0
  never set; a trader of neither kind reads a stale register, as in the
  game.
- [[71]]: past the equipment tables the port reads zeros where the game
  reads what follows (game UB).
- [[94]]: `ccBufferSampling` (`+0x138c`) and the second mask (`+0x1358`) are
  created and never used, as in the game.
- [[70]], [[71]], [[73]]: the menus keep their own copy of `rand()`
  (`PcMenu` among them), not the world's shared generator;
  `FieldUi::set_rand` is not wired.
- [[142]]: the dogs' menu draws `talkNum` from `rand()`, not from the town's
  `ccRand`.
- [[94]]: the generators' state at the ending (the random characters)
  differs from a console's run.
- [[66]], [[76]]: a menu a button opens stops the camera and player a frame
  later than the game, and camera 3 appears a frame later (the menus run
  after the world's tasks).
- [[71]]: the equipment, party-add and Controller tasks finish one frame
  later than the shortest the game can take (disc timing).
- [[69]]: menu 79 bumps its wait twice a frame, so Kite is mid warp-out at
  the change, as in the game.
- [[96]]: after a success, `proccess` stays 2 until the area changes; how
  many frames the game shows it.
- [[58]]: whether the top page task's first `Main` runs in the frame it
  starts (docs/engine/toppage.md Unknown).
- [[58]]: how the fader behaves while the system menu freezes the layers
  (docs/engine/toppage.md Unknown).
- [[58]]: the set-up passes run at once and draw nothing; their timing is
  not modelled frame by frame.
- [[174]]: whether the game's `info_now` on the set-up screen places its
  lines where `SetupScreen` does.
- [[174]]: the tail of the last window before stream 10 is not timed.
- [[43]]: the boot check's timing and the stream's end-flash alignment are
  inferred.
- [[51]]: `LOAD_FRAMES`: the real time between `ChangeRequest` and
  `ccSetupDesktop`'s `ccAllSoundOff` (docs/engine/title.md Unknown).
- [[71]]: the OPTION helper's CD load, in real frames.
- [[186]]: how long the game's loads take, and so how much of the loading
  display's fade and logo a player sees.
- [[186]]: the load request's `+0x18`, which also stops the display; its
  setter is not traced.
- [[226]]: whether the PS2 shows the recall's fades black too; every checked
  step matches the game.
- [[237]]: the port's quit prompt uses the desktop's window, where the
  field's own confirmations use the field menu's.
- [[247]]: nothing tells the player that a finished part's save loads in the
  next; New Game from a previous volume's save is not checked from a build
  (the import itself is the save's entry).
- [[68]]: the gate-target fix has no eemu check of its own.
- [[196]]: the present's remark is not run in a town with three members.
- [[100]]: the minimap's markers are not checked frame by frame;
  `tools/test_map_rs.py` covers the rule.
- [[77]]: the map for the special dungeon types 8 and 9 and for story rooms
  of type 15 and up is not checked; labels are compared as strings only.
- [[158]]: the game-over noise's draws are not checked frame by frame
  against `ccGameOverNoise`; the counts are tested.
- [[159]]: `ccMessage`'s rate-2 drawing (its button's pulse); the harness
  hooks it.
- [[70]]: no run-time shot of a shop; covered by the checks.
- [[75]]: no shot shows the world moving under a tutorial menu.
- [[148]]: the object menus against the game's pictures; the checks are
  frame by frame in eemu.
- [[94]]: the staff roll's pixels and the ending song's start and stop, not
  compared.
- [[104]], [[179]], [[190]]: the field noise's, the crisis town's and the
  spells' screen noise's pixels; no capture.
- [[193]]: the ALL PORTALS OPEN banner against the game's pictures.
- [[194]]: the panels' shake and the protect marks against the game's
  pictures.
