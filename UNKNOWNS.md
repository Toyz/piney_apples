# UNKNOWNS

A ledger of the project's open questions, made on 2026-09-28 (tree at 91e68ba) from
`cairns open` (every worklog entry's **Still unknown:** paragraph) and the
`## Unknown` sections of the reference under docs/. Each paragraph is split
into single questions, and each question is classified against the later
worklog, the docs, BUGS.md, GAPS.md, plans/ and the code. Nothing in the
worklog, the docs or the code was changed for this ledger.

The same day the ledger was folded into the worklog as ten consolidation
entries, one per subsystem: [[282]] the disc, formats, engine core and
tools; [[283]] the draw path; [[284]] the effects and the streams' effect
tasks; [[285]] towns, fields and dungeons; [[286]] battle; [[287]] the event
scripts; [[288]] menus, the desktop and text; [[289]] sound, voices and
movies; [[290]] the save; [[291]] the later volumes. Together they resolve
all 271 entries below, list each answered or dead question with its
evidence, and restate every open one (play first, then volumes, then
research) with the entry that asked it, including the few left by entries
54, 103, 118, 120 and 263. The worklog's open list (`cairns open`) is now
those ten entries. Every cited entry, path and test name was checked to
exist, and about 110 answers were read against their evidence; one was
downgraded to open ([[28]]'s three canal waters, whose cited page
describes Carmina Gade's). The docs' stale and dead `## Unknown` bullets
were removed, and their answers stated in the pages' bodies where the
bodies did not already hold them.

The classes:

- **play**: still open, and it changes what a player of the port sees,
  hears or can do, on any volume.
- **volumes**: open only as a cross-volume check (is it the same on
  Mutation, Outbreak, Quarantine).
- **research**: open, of documentary interest only (hardware rounding,
  console timings, the game's own undefined behaviour, unused data,
  pictures never compared with a console capture, test gaps).
- **answered**: settled by later work; the line cites the entry, page or
  code (and its test) that holds it. Where the citation was not certain,
  the question stayed open.
- **dead**: no longer a question (a process note, a line with no question,
  a port artefact that was replaced).

A doc bullet marked *stale* is answered elsewhere but still listed on its
page; the Docs section says where. Doc bullets that repeat a worklog
question say which.

Outbreak's story run started the same day (plans/outbreak-story.md); the
Outbreak and Quarantine items below predate it.

## Summary

271 entries from `cairns open`, split into 928 questions;
241 bullets from the docs' `## Unknown` sections (45 pages, 10 of
them "None").

| class | worklog questions | doc bullets |
| --- | ---: | ---: |
| play | 51 | 27 |
| volumes | 19 | 0 |
| research | 400 | 179 |
| answered | 455 | 32 |
| dead | 3 | 3 |
| total | 928 | 241 |

Not in `cairns open`, so not counted: entries 54, 103, 118, 120 and 263,
which a later entry's `resolves:` closes (their few remaining points are
under "Closed by resolves" below), and 252 and 261, which say "nothing".

## Play

The actionable queue: still open, and a player of the port would see or
hear the difference. Where several entries (or a doc page) raise the same
gap, they share one line. The Docs section repeats the doc bullets.

### battle

- [[268]] Innis's drains at its breaks gave item drops and no Epitaph in the survey, where `innis_drained_through_the_menus` reaches the Epitaph — Mutation's first boss fight may skip its Epitaph phase; possibly the queued-affect bug [[274]] fixed (a lost 21), not rechecked
- [[272]] Kyvia's gomoras do not push each other; the pad's sway, the arm's picture and the particles are missing (docs/engine/boss-kyvia.md) — Mutation's Kyvia 01 fight, look and movement
- [[274]] Magus's shock beams are taken to meet the ground (no `ccHitCheckLM` on the land); `EntryFlash2`/`3` are one flash; the grow's looping SE plays once (docs/engine/boss-magus.md) — Mutation's Magus fight: hits, flashes, sound

### world


### render

- [[38]] [[39]] [[72]] the field water's run-time textures and scroll, and the clouds' blending, are approximations (docs/engine/field.md, field-walk.md) — every field with water or clouds
- [[97]] water 0's frame-buffer copy is the whole frame, not the game's 128 x 128 point-sampled one (docs/engine/field-game.md, town02.md) — the towns' water
- [[79]] [[231]] `SetFogSw` / `SetShadowSw` on the spell pieces (and on other objects that turn fog off; only the gate was fixed) and the drill's `ccDrawEnv` settings are not modelled (docs/engine/effects.md) — spells and objects fogged or shadowed where the game turns it off
- [[79]] the explosions' palette swaps are kept on the element but not drawn (docs/engine/effects.md) — spell explosions' colours
- docs/engine/effects.md `ccEntryChangeCLUT` swaps every texture on the palette where the game swaps per material — enemies whose palette-swapped texture shares a palette
- [[157]] [[161]] `EntryObject`'s palettes for the lakes' statues and flowers when `GetBG` is not 0 are not ported (docs/engine/dungeon.md) — lake dungeons by evening and night
- [[158]] `DUNGEON.fog` is not zeroed at the game over's TV, and the squeeze moves the whole finished frame, not only what sysLayer's view draws — the game over, in a dungeon most
- [[266]] Innis's pictures are not drawn: missiles, SamonRings, particle generators, blur, shield, mirrors' shards; the MagicSquare for n 1 and 2 (docs/engine/boss-innis.md) — Mutation's first boss
- [[274]] Magus's pictures are named, not drawn: the leaves' markers, charges and bursts, the laser's shocks and thunder, the needles, the smoke; the body's dropped leaves are not hidden (docs/engine/boss-magus.md) — Mutation's Magus
- [[280]] Innis's ice missile and Fidchell's IceBreak are not ported, so nothing throws their rocks — Mutation's and Outbreak's bosses (Fidchell's rules ported in [[294]]; its IceBreak's rocks are still not thrown)

### audio


### ui

- [[317]] [[318]] [[347]] the message window's `mode` differs for a frame just after a book closes (Books IV-VIII's pages: [[347]]) — reading the Ryu Books

### script

- [[87]] the event cameras in event 4's rooms (the user saw the camera in the walls) were not re-shot since each block plays in its own room — event 4's dungeon scenes

### build

- [[259]] a later volume's image played without Infection's disc falls back to `work/`, and `crate::die` ends the process — a player who owns only a later disc

### volumes

Missing pieces that only the later volumes need.

- [[24]] [[325]] what fills Outbreak's and Quarantine's BSS message groups; what `eventAreaNumber` 126 loads (Quarantine's ending, event 314; the port gives it no words) (docs/engine/events.md) — Quarantine's ending
- [[65]] [[116]] `npc_act -3` (row 139's drain, with `effVirusCrystal`) and `-5` are not ported; only Mutation's event 114 uses them (docs/engine/events.md, effects.md, field-game.md) — Mutation's event 114
- [[275]] `marker_pos` on a plain field (`WORLD_MAN` +0x438's stream) is not ported, and no event map but `EVENTAREA01` answers `StoryMap::file` — Mutation's field events
- [[395]] from Mutation on a field ride's charge (MUT gcmn 0x0052f790, the knockback 0x0052f250, `+0x158`..`+0x160`), its Grunty chat (0x00530e50, 0x00530dd0) and the `+0x1e0` range are not ported (docs/engine/grunty-ride.md) — a later volume's field ride with the flute
- [[394]] [[397]] menus 90 and 91 (MUT gcmn 0x0058d470, 0x0058d9f0 with its page 0x0058edc0 and helpers 0x0058f5d0-0x0058f930, MUT main 0x0017b0b0, 0x0017b140: list 90 is Talk and Item List, 91 the item registry that gives desktop items, "Items were registered to the ...") are not ported. Who opens them is traced by [[397]]: the Event NPC (flags 0x10000000) opens 90 (MUT gcmn 0x00536aa0); the port shuts it at once — the later volumes' Event NPC
- [[272]] Kyvia's later fights (levels 2-5, the EX mode); fields 10-12 not played through (docs/engine/evarea.md, boss-kyvia.md, GAPS.md) — Mutation onward
- docs/engine/battle.md `ccBoss04`'s Affect and AI are not ported — a later volume's boss (ported in [[294]])
- [[72]] [[84]] the other `EVENTAREA` classes (04-06; areas 66, 67, 91) get generated fields (docs/engine/evarea.md, field-walk.md) — the later volumes' story maps; no Infection script reaches them
- [[112]] `EVENTAREAB0`'s `NextStage` (field 8, `se4_8`) is not ported (docs/engine/evarea.md) — a later volume's last arena
- [[114]] [[157]] type 32's tree and leaves (`ANM_se4_2lea`, `LEAF`) are not ported (docs/engine/dungeon.md) — a later volume's event room
- [[220]] on Outbreak and Quarantine `GetEventAreaInfo(0)` returns NULL where the port finds row 0 — Outbreak's and Quarantine's areas
- [[77]] [[260]] `DrawMap` of towns 2-4 draws four sprites in the later volumes; [[260]] and docs/engine/map.md say the port draws three, but crates/piney-world/src/map/town.rs now carries a fourth (MARKS) sprite on the towns' own layers: to confirm — the later towns' maps
- [[260]] which events place characters in Fort Ouph and Lia Fail — Outbreak's and Quarantine's stories
- [[393]] Mutation's `ccRtownPC` (MUT gcmn 0x00522040): body kind 8, 50 `rtpcCcsName` names, the later rows' weapon tables, the SEARCH PCs' set-up (0x00521e50) — answered by [[397]] (rtownpc.rs `Pool`, `Search`, `search_mode`; tools/test_world_rs.py `TownPcsAgainstGame` on all four volumes)
- [[393]] `ccSetMerchant(0)` from Mutation on also makes row 175 + town ("Event NPC") when `saveData+0x652c` is set, its act (0x00521a00) and affect (0x00521990), and what sets the byte — answered by [[397]]: `eventStatus[52]`, which event 317 (ITEM COMPLETE) sets when mail 328 is read (merchant.rs `EVENT_NPC`, `event_npc_act`; `MerchantsAgainstGame.test_event_npc`)

## Volumes

Cross-volume checks: the same code or data on the later discs, not yet
compared. (GAPS.md "After Infection" tracks the phases.)

### battle

- [[21]] whether ccRegisterDifficultyEnemy matches on the other volumes — read on all four ([[275]]), never run in eemu there
- [[67]] piney-battle checked on Infection only — MUT's harnesses run ([[219]]-[[224]]); OUT and QUA not
- [[278]] how the enemy tables changed row by row on OUT — only offsets compared

### world
- [[397]] tools/test_world_rs.py on Mutation: `PropsAgainstGame.test_town_draw` stops on a syscall in Mutation's `ROOTTOWN01::Draw`, and `WorldAgainstGame.test_frames_random` differs in Kite's camera at frame 88 of "town01 random 0" — from before [[397]], not read
- [[397]] in Carmina Gadelica merchants 15 and 16 land higher in the port than in the merchant harness (z 2.0 and 1.4e-4 against 0); whether ROOTTOWN03 registers its Hit chunk otherwise than ROOTTOWN01 is not read

- [[206]] where Mutation's and Outbreak's idolItemList44 is — neither candidate reads like Infection's
- [[260]] Fort Ouph and Lia Fail not compared with the game in eemu — not compared
- [[275]] Outbreak's and Quarantine's fellow deletes not compared — no name carried

### render

- [[280]] OUT's and QUA's sqrt.s in the distance fade — see [[281]]
- [[281]] OUT's and QUA's one-ulp float differences; whether the per-volume hooks and the 42-spike wave make upheaval agree there — not rerun

### audio

- [[203]] Outbreak's and Quarantine's ccVoiceRequest cases — [[227]] leaves their spcVoiceData to be checked
- [[278]] who ids 18-20 are on OUT and QUA (ccCharBaseParam.id rows) and their tables' sounds — [[239]] names charTbl rows 18-20 (Tsukasa, Subaru, Sora); the sound tables not followed

### video

- [[267]] Outbreak's and Quarantine's versions of the stream effect tasks Mutation changed (Func_str1070, str7100, str8800, str0300) — not compared
- [[269]] Outbreak's and Quarantine's Func_str1070 — not compared
- [[271]] Outbreak's and Quarantine's str7100, str8800 and str0300 — not compared

### ui

- [[94]] the later volumes' staff rolls (stfroll2-4) not tried — GAPS.md lists Quarantine's staff roll as its own
- [[276]] Mutation's staff roll and what the desktop does after it (the save for Outbreak's carry-over) — not followed

- [[390]] tools/test_fieldui_rs.py on Outbreak and Quarantine: every Ryu Book case matches, but 7 (OUT) and 8 (QUA) of the other menus' cases differ: three of noise, whose Disp block the harness does not find there (`DISP_NOIZ`, `GT_HACK` left empty: test_inter_noiz, test_a_new_scenes_menu, test_gate_hack), and test_gate_refusal, test_chat_member_refusals, test_low_hp (conIcon), test_spring_throw, QUA's test_skill_refusals — not read
- [[392]] tools/test_fieldui_talk_rs.py on Outbreak and Quarantine: test_admin and test_breeder_about differed in the message window's mode (1 against 4) on a chained record (`Check(1)`) — answered by [[396]]: the later volumes' talk records were cut to Infection's lengths, so the chained record was `errorData`; the whole harness now matches on all four volumes
- [[392]] tools/test_fieldui_shop_rs.py's Recorder pages on Mutation (test_new_directory, test_save_new, test_save_overwrite): from Mutation on `RecordMenu` drops the 10-frame hold after a save's result (MUT gcmn 0x00577a8c, 0x00577af0, 0x00577b54) unless `game+0x7c` or `ccSys+0x26c` is set; the port keeps Infection's hold, and what `ccSys+0x26c` is is not known. Quarantine's Buy pages and merchant lists differ (test_buy, test_buy_cancels, test_random_buy at frame 40; test_merchant_pages, test_random_lists) — not read. Outbreak's and Quarantine's test_other_talk were the cut talk records too, answered by [[396]]

### volumes

- [[198]] the 296, 561 and 565 functions voldiff marks changed on MUT, OUT, QUA — Mutation's read as met ([[219]]-[[228]]); OUT's and QUA's not (GAPS.md Phase 3)
- [[218]] voldiff counts 296 changed and 11 missing functions among those the port covers on MUT — see [[198]]
- [[220]] most of the other changed functions still to be read one by one — see [[198]]
- [[265]] Outbreak's and Quarantine's dungeon, area and sound tables not compared with Mutation's — not compared

## Research

Open, but only of documentary interest; the port does not change for a
player until one of these turns out wrong.

### battle

- [[12]] whether the placeholder enemies can appear — not revisited
- [[22]] normal (non-drain) drops — nothing found in battle.md; ccExpDistributor gives experience only; not surveyed
- [[22]] what type bits 0x100/0x200 mean — docs/engine/battle.md Unknown (set on spells, read by no rule)
- [[22]] that the protect break is the Data Drain window — still inferred (docs/engine/battle.md Unknown)
- [[67]] party_ai::Ai::new has no check — test gap
- [[73]] CalcReal skips a worn piece of -1 where the game reads the row before — no new-game member wears -1
- [[75]] idle animations under an event's manual control not compared frame by frame — the game's rule (ccFellow::Action)
- [[78]] a dungeon way of 55 cells or more overwrites the beacon pointer; unset destinations read stack garbage — game quirks (docs/engine/battle.md Unknown)
- [[78]] the third swing never reached; ctrlType 1 and 2 unused — inferred (docs/engine/battle.md Unknown)
- [[78]] the equal-priority task order within ccThSpc — inferred
- [[79]] the magic portal ported twice (piney-battle entry.rs and piney-effect) — code duplication, both checked
- [[95]] what bit 0 of ccGetSkillParam(sid)+0x2c means — [[101]] asks again; not named
- [[98]] whether ccThEntryCtrl's first slice runs in the frame of ccEnableThEvent(4) or the next — inferred from task order
- [[101]] bit 0 of a skill row's +0x2c — not named
- [[105]] the race's base form ccThDrainEnemy builds for a state 2 no caller sets — unused
- [[109]] the effects' particles draw from the game's generators; no test runs rules and effects together — docs/engine/boss.md "Not described yet"
- [[110]] ccThSpc and ccThAISystem run before a leaver's task ends (the game's ends at slot 50, after them) — docs/engine/field-game.md Unknown
- [[117]] rules and effects not checked together against the game; IceBreak's Create order and ccRand — docs/engine/boss.md "Not described yet"
- [[123]] levelOld in the game's heap when a member is built in town — docs/engine/battle.md Unknown
- [[123]] whether the stray # lines freeze a real console — docs/engine/battle.md Unknown
- [[123]] the raised lines are said after the task, not inside the hit, so their rand() draws come later — ordering
- [[123]] what sets saveData +0x220d (id 1's garbled lines) — docs/engine/battle.md Unknown
- [[155]] ccGimSymbol::objMain not run in eemu against the port — read from the code
- [[158]] the party under manual control and ccClearConditionAllEnemy at the game over are not run — nothing after the freeze reads them
- [[162]] ccGimSymbol's main not run in eemu — see [[155]]
- [[167]] goldVolume outside 1-4 leaves wear to a register — checkGold never makes one
- [[167]] a gold goblin's acts 2 (wander) and 4 (home) did not come up in the sample — sample gap
- [[167]] a shaken-off hold not played through — test gap
- [[168]] how often setBreath and the scorpions' spin come up — counted roughly
- [[171]] ccEnemyL's type 3 (rows 203-206): what they are and whether any event places them — not ported; no Delta list registers them (GAPS.md)
- [[171]] the wyrms' and dragons' setBreath did not come up in a 300-case count — read, not measured
- [[187]] how many frames the game's weapon load takes before EquipWeapon — not measured
- [[192]] whether a menu or script writes a party record in the same frame gap as the world's clears — not surveyed
- [[223]] the game's windowed history walk never moves past a delivered item use that casts nothing; the port passes it over instead of hanging — game bug avoided
- [[223]] Ctx::item_first reads a member without an AI as having no hit; the game reads through a null pointer — game UB
- [[223]] who sends message kind 0xc and broadcasts 23 and 24 — unread
- [[229]] no test drains an area skill's sub-targets (Drain Arc, Drain Heart) — test gap
- [[230]] a member's item use is carried out after the AI's frame — one frame late for a same-frame reader
- [[230]] no test drives a member's own item decision end to end — test gap
- [[230]] CHAT's heal order out of battle plans once and drops it — as the code has it
- [[238]] the party AI's skills through the same request not watched in play — not watched
- [[265]] whether the Squidbod can be beaten at the level a player brings — the pilot's party is the start's
- [[266]] act 7 (the shake) on Innis itself never reached — not checked
- [[268]] whether a party at a player's level breaks Innis's protect sooner — pilot's party is the start's
- [[272]] why the Hackberry King in area 47's dungeon recovers under the pilot's fight, and whether the game's would — not checked
- [[273]] the lowest level at which the pilot beats Kyvia 01; a player's level and gear there — not measured
- [[273]] whether the Hackberry King's HP should come back as fast as it does — not checked against the game
- [[274]] Magus's acts 30 and 31 and a leaf's 23 never reached — whether any path reaches them
- [[274]] whether Skeith's and Innis's drains through the menus lost their 21 before [[274]]'s fix — not checked
- [[275]] what the community note's enemyList00 refers to — the code gives story areas column 6
- [[276]] whether a run without the level aid beats Kyvia 01 — not tried
- [[279]] which enemy and area the report's staff came from, and whether it was drained — report detail

### world

- [[11]] what each value of fieldType, weather, ground, object, circleOfs looks like in the game — the port runs the ported generator/draw for every value ([[33]], [[38]], [[130]]); no per-value catalogue; still in docs/engine/area-words.md Unknown
- [[11]] EVENTAREA_INFO.protect beyond protect[1] and the item pairs, and .dungeonNum — docs/engine/area-words.md Unknown
- [[19]] why dungeon types 8 and 9 pick a down-stairs room on their single floor — docs/engine/dungeon.md Unknown
- [[19]] EVENTAREA_INFO.protect[] beyond index 1 — docs/engine/area-words.md Unknown
- [[19]] sd4a.cmp, named by DungeonName2 but not in DATA.BIN — not revisited
- [[21]] why area 47 gets a different item offset from volume 3 — design intent; the mechanism is in [[21]]
- [[21]] what area 126 is — docs/engine/area-words.md: a record with no address added from MUT ([[220]]); purpose unknown
- [[25]] what DUNGEON.ishack means — still in docs/engine/dungeon.md Unknown; [[161]] reads 3 as hacked (the lakes' fireflies)
- [[33]] where the game would loop forever, the port gives up with NoSite; no case reached — not revisited
- [[59]] rand not seeded from the game's shared state, so idle fidgets differ in time — inherent to a run's history
- [[65]] whether a party member or the gate has a body on the character list — not revisited
- [[65]] ccSetRtownPC's second argument on the event path — answered by [[393]]: -1, the loop's `li $a1, -1` (INF main 0x001b6318, MUT 0x001cb9dc)
- [[65]] ccSys +0x358, the frame count that picks the PCs — an input depending on run time
- [[65]] an event NPC placed after set-up goes to the list's end — order not checked
- [[72]] SimGenerateCode and GO move the global generator; the port leaves those draws aside — docs/engine/field-walk.md Unknown
- [[97]] fieldrand's state on entering a town (the port starts at 13) and the scrolls' statics starting at 0 per town — phase only
- [[106]] how long the game holds the Chaos Gate loop (ccLoadResourceFL's time) — not measured (docs/engine/field-walk.md Unknown)
- [[106]] ghoCam's markers from the desktop's anm player not compared — docs/engine/field-walk.md Unknown
- [[108]] the town branch of SetCharPosition read, not run — offsets match the field's checked ones
- [[110]] the town frame gets menu_type -1 or 0, not which menu — not revisited
- [[110]] a second rebootSpcManager in one town visit makes no new AI for Kite's stand-in — not revisited
- [[111]] RoomSelect's bgColor 0 and GS words not modelled — the change's fade covers them
- [[111]] SimGenerateCode's moves of seed and randcnt left aside — docs/engine/field-walk.md Unknown
- [[112]] fieldrand's seed on arrival not carried across scenes (the arena's firefly bobbing) — phase only
- [[114]] area 67's type-34 row behind a door with no branch would crash the game; whether the story reaches it — not known
- [[115]] fieldrand's seed on arrival not carried from the dungeon — same as [[112]]
- [[132]] ccEntryEventMng's second argument to ccSetRtownPC (the port uses -1) — answered by [[393]]: -1
- [[132]] the event NPC classes step outside the entry control's NPC turn, so their effects start a frame late — docs/engine/field-game.md Unknown
- [[134]] whether Kite should walk around the fenced statue in a statue room's open doorway (area 18's dungeon) — not compared
- [[140]] DungeonEntries.rng is a copy, so the dungeon's fieldrand does not advance with the breakables' draws — later draws can differ from the game's
- [[142]] the dogs' walks not compared frame by frame — not compared
- [[142]] ccDog's constructor leaves the stuck count (+0x21c) unset — the port starts at 0
- [[154]] the Zeit statue not reached and opened in play — last floor of its dungeon
- [[154]] the fourth clock (+0x70) has no reader found — not found
- [[157]] a story dungeon's fieldrand after Generate — docs/engine/dungeon.md Unknown
- [[162]] the Grunties' camera writes land a frame's NPCs later than the game's — ordering
- [[162]] runAway's one frame of checkHitResultAttlibute not compared — harness lacks the collision's last result
- [[163]] what row 19 with param[2] 0 was for — no setter leaves it 0
- [[163]] GetNearDoorPosition with every mark beyond 12,000 returns stack words — the port keeps the row's place
- [[164]] the node places in the field's objects use the dungeon posing, checked on rooms only — not checked on field objects
- [[164]] where the foods stand in the game not looked at in a picture — sums put them 250-370 from centre
- [[164]] the game's result for a field with no dungeon entrance (in-points 0) — unknown
- [[166]] the ride's task and the first cancel check run a frame later than the game's — ordering
- [[166]] the port sleeps and wakes the whole party where ccSPC::Wakeup wakes only those in it — the same in Infection's fields
- [[166]] plw +0x10 and ccDamUprStr::CtrlAll in the ride menu's waits not kept — not modelled
- [[166]] the Grunty's legs' places from the port's last pose — not the game's last draw
- [[166]] riding into a dungeon's way in (pgDIN) not played through — test gap
- [[177]] area 16's EVENTAREA07 not compared with a game picture — not compared
- [[179]] whether any Infection field is hacked outside the crisis (EVENTAREA_INFO.flag 3) — not surveyed
- [[250]] whether inviteSpc in a field or dungeon is ever reached — docs/engine/field-game.md Unknown
- [[272]] EVENTAREAB8 held by unit tests, not compared with the game's Draw — docs/engine/evarea.md Unknown
- [[277]] whether a walking PC can run off in a case the tests do not cover — only a member's run caught
- [[395]] the Flag Race's task runs its first breath in the frame the menu asks; the game's priority (63) against the other tasks is not modelled (docs/engine/flag-race.md Unknown)
- [[395]] the race's restart flag `+0xa6` (MUT gcmn 0x005ff520 reads it) — no code found sets it (docs/engine/flag-race.md Unknown)

### render

- [[17]] the DMA order of the matrix, material and model packets — docs/engine/render.md Unknown
- [[17]] the full bone and skin packet layout, and which of the b/m program families is skin — docs/engine/render.md Unknown
- [[17]] what view + 0x110 holds — docs/engine/render.md Unknown
- [[17]] the mc04b/mc04m variants — docs/engine/render.md Unknown
- [[17]] the effect program beyond its entry point — docs/engine/render.md Unknown
- [[26]] whether the game really shows town01's floors that dark — never compared with a console picture ([[234]] says the same of a field)
- [[26]] textures defined in other files (# entries) show white in the viewer — no later entry says how '#' entries resolve; not seen reported in the runtime
- [[27]] the 15 morph-target-shaped models no F_Morpher names in their own file — not revisited
- [[36]] which of the 11,086 F_Morpher records are faces and which cloth — not needed to draw them
- [[37]] the 2-pass FB_ONLY where one draw overlaps itself — [[48]] fixed the translucent layers; its own open point (failing and passing pixels of one draw overlapping) has no known case
- [[37]] the GS's 8-bit rounding inside the pixel pipeline is not modelled — [[175]] snapped vertex alpha only
- [[38]] the field objects' own draw code and the type-6 FOBJECT2s DrawMesh draws — docs/engine/field.md Unknown
- [[38]] whether ExtObj-shared object models are lit once or per use — not revisited
- [[38]] whether tile alpha is clamped past 7,200 — docs/engine/field.md Unknown
- [[38]] whether VU0 hardware rounds as the modelled truncation — needs a console
- [[48]] edge rasterisation (GS fill rules against the GPU's) and the selection-bar row — not explained
- [[48]] failing-pixel order where failing and passing pixels of one draw overlap — no test, no known case
- [[48]] both renderers are ours; neither compared with a real GS frame capture — needs a capture
- [[53]] the glows and stream 2's figure checked against the code, not a console frame — needs a capture
- [[53]] which group (opaque or sorted) a mmat lands in is not compared with the game — from the disassembly
- [[53]] blend types 2 and 3: the streams not surveyed for them — not revisited
- [[55]] ccMorpher's GetWork first-call path never ran — same arithmetic
- [[55]] whether any stream uses a morph weight that is negative or 8 or more — not surveyed
- [[57]] stream 2's effect task pixels and timing — packets checked, not pixels
- [[57]] the rand state when stream 2 starts — matches only from a known state (docs/engine/stream.md Unknown)
- [[57]] VRAM past the draw buffer reads 0 in the port — docs/engine/stream.md Unknown
- [[59]] the crisis town's drawing not checked by eye — not compared with the game
- [[61]] the near-plane rule read from the VU1 microcode, not run — not run
- [[61]] the order in which the gate's light joins the town's — [[141]] checks only the group logic
- [[61]] the CPU renderer drops triangles behind the camera instead of cutting them — the software model only; not revisited
- [[69]] the fader drawn with the desktop's corner constants in the field — not checked
- [[79]] undefined behaviour taken one way (odd sizes or drain types, stack leftovers as 0, the goblin summons' read) — docs/engine/effects.md Unknown
- [[79]] a sprite's fog — docs/engine/effects.md: Prim has no fog, the effects' own sprites never fog
- [[83]] effResistantShield with no affectPerson reads through a null pointer — game UB (docs/engine/effects.md Unknown)
- [[83]] effHeal's return: the game returns 0, the port its slot — not compared, no caller uses it
- [[83]] the new particle generator rows seen only in shots — docs/engine/effects.md Unknown
- [[84]] EA_moveTex02's scroll restarts at 0 with a new map after the town, where the game's carries on — docs/engine/evarea.md Unknown
- [[88]] the sharp-bilinear scaler not seen at the user's window size — checked by test only
- [[88]] no setting for a plain or television-like filter — a port option idea, not the game's
- [[96]] the gate hack's 3D not compared with the game's pixels — calls compared
- [[97]] water 0's place for waterUVModifi2's reach taken as its root — the object's coordinate not checked in a run
- [[97]] what the clouds look like on the console — no capture
- [[99]] the tint's pixels not compared — port's fog blend
- [[99]] where scFadeDef draws (its layer word +0x94) — taken as the font layer
- [[100]] whether ccAnm::Draw of a fog-off clump (the portal) takes the field's lights — drawn unlit
- [[104]] a noise band's copy past the draw buffer reads 0 — as in the stream
- [[105]] the drained enemy's look in the movie not compared — lights worked out at its matrix
- [[114]] how ccLight combines an event room's LGT_ record with SetRoom's place; the block's clump nodes drawn at its matrix — docs/engine/dungeon.md Unknown
- [[117]] ccBossBlur — not ported: whether Skeith's fight blurs depends on the heap (docs/engine/boss.md "Not described yet")
- [[117]] the trail's projection, the reversed sprite and the light on the characters' shading checked by eye only — no capture
- [[119]] the feet's pose for the dust: port's stance at the start against the game's last draw — not measured
- [[121]] the port's draw clips by the scissor, not the letterbox's clip box (bboxClipMin/Max) — not revisited
- [[127]] what SetFog(0, 0, ...)'s divisions by zero leave in fogA/fogB (stream 4) — taken as no fog; [[267]] meets the same in Func_str0932
- [[128]] hold 7 and ccBossBlur — see [[117]]
- [[130]] the field's smoke puffs start a frame late — docs/engine/field.md Unknown
- [[130]] the balloon's cloth scroll and Ω's runners not seen in a shot — story server is Δ
- [[130]] the lens flare left above the picture by the follow camera; the eye view fails its camera test — as the game's code; no shot
- [[131]] strEffectStopFlag is never set by anything the port runs — its setter in the game not named
- [[133]] stream 15's parts' pixels not checked — draws checked
- [[136]] stream 15's opening against a game frame — draw state only
- [[136]] the other streams' light pictures (3, 5, 7, 8, 10, 11, 16) not compared — docs/engine/stream.md Unknown
- [[137]] the spot light controller (0x0607) — no Infection room uses one (docs/engine/animation.md Unknown)
- [[137]] F_Obj's flag byte (+0xa2) — not modelled (docs/engine/animation.md Unknown); its effect-object branch is, for the streams ([[156]])
- [[137]] whether an F_Obj pose outlives its clip — not checked
- [[137]] F_Obj not checked in eemu — composed from checked parts
- [[139]] the walking PCs' variants and steps not compared with the game's pictures and sounds — docs/engine/field-game.md Unknown
- [[141]] Data Drain step 10's effects start at the next effect task, their rand() draws a frame late — ordering (docs/engine/field-ui.md "Not yet known")
- [[141]] the side effects' pixels not compared — no capture
- [[141]] the areas' light positions and colours at frame 1 not checked — docs/engine/field-game.md Unknown
- [[141]] ccDrawEnv +0x88, a second light group SetLightMatrix walks first — not traced (docs/engine/field-game.md Unknown)
- [[145]] the per-vertex fog not compared with a game picture; F values not run against VU1; PRIM.FGE taken from SetFogSw — docs/engine/evarea.md Unknown
- [[146]] the REGION_REPEAT shades not compared with the game's pictures — texel choice checked
- [[147]] no picture of a swapped dungeon compared; a twin the file lacks gets a null palette in the game — the port leaves it unswapped
- [[149]] the symbol's fires and light not compared frame by frame — not compared
- [[151]] whether anything gives the direct light a parent coordinate, and whether the room anm's loop re-applies its frame-0 place — not run in the game
- [[152]] the field's depth shades against a game picture; the GS's bilinear weights and reads past the frame; model Z clamped at 2^32 where the VU saturates — not modelled
- [[153]] no close-up of streams 5 or 15 pictured against the game; str0581's divZ 3000 and stream 5's 500 not looked at frame by frame — docs/engine/stream.md Unknown
- [[155]] the cast on a lake symbol and its puffs not looked at on screen — not looked at
- [[156]] str0001's clouds not compared with the game's picture — no capture
- [[156]] ccAnm clumps' effect nodes in other files — none found to have one (docs/engine/effects.md Unknown)
- [[157]] what ccObj::Duplicate copies of the water's object — assumed (docs/engine/dungeon.md Unknown)
- [[160]] the console's choice between the direct and clipped shadow path (pipelined sticky sign) — differs only past Z 0x7fffffff (docs/engine/shadow.md)
- [[160]] the shadow bounding box check reads vf21, never set; an open edge's second face reads VU1 memory below the directions — game quirks
- [[160]] shadow modes 2 and 3, never made in Infection — not ported; later volumes not surveyed
- [[160]] EVENTAREA02 and 07 reset the light after GO — noted, not followed
- [[161]] the lake sky's scroll starts at 0 in each dungeon, where the game's runs on — phase only
- [[162]] the growing-up particles and the foods' look not compared with the console — no capture
- [[165]] the double atan2 is Rust's libm, not newlib's — agreed in every case
- [[165]] the effects' shakes applied after the frame, not inside the effect task — only rand() order within the frame
- [[170]] whether ccChar::Draw leaves node coordinates stale on a culled frame — docs/engine/battle.md Unknown
- [[170]] what a stream's decoded dummy chunk holds at +0x10 and +0x20 — the port takes the file's place, w 1, degrees as radians
- [[172]] the flames not compared with a game picture; the sprite's turn (+0x28) stays 0 — not compared
- [[172]] the runtime's bursts draw ccRand after the entry control's frame — ordering
- [[172]] the first enemies' weapon trails and dust not looked at after the fix — not looked at
- [[175]] whether the window showed Skeith's lag worse than the headless run — not measured in the window
- [[175]] the drain's erasing of the figure inferred from the rules, not compared — not compared
- [[177]] whether the game's GetChunkAdrsF falls back to other loaded files for block 1 — not read
- [[195]] the skill starts and shock waves not compared with the game's pictures in play — no capture
- [[197]] the weapon trails and particles not compared with the game's pictures — no capture
- [[197]] orange specks far off in the Flame Dance shots come from none of the weapon generators; which effect draws them is unknown — unexplained
- [[197]] a member's StartArmsEffect not seen in play; the hands' matrices are the port's pose — not compared
- [[234]] no picture of the real game to compare the symbol room's darkness — light values harness-checked
- [[238]] whether any skill's animation draws (no clump; nothing draws +0x80) — not found
- [[244]] the port's ST ignores the 1 + S * 2^-23 factor — 3e-5 of a repeat, below a texel
- [[264]] whether EVENTAREA01's DrawObj/DrawObj2 layer differs from objLayer — the active layer Draw set
- [[266]] which clump the game draws the Epitaph's ANM_ex2x* clips on — not checked
- [[272]] which of CMP_ex01gom2-4 each gomora draws — docs/engine/boss-kyvia.md Unknown
- [[279]] whether effAbilityDown's icon rows include a staff — rows not rendered one by one
- [[279]] whether a killed foe's condition effect fades or ends at once (DispConditionEffect against the game's delete order) — not traced
- [[280]] what ccEffect::Main ids 0, 113 and -11..-8 were for — nothing makes them
- [[280]] what a real PS2 keeps at address 0 (the -11..-8 case would overwrite it) — console
- [[281]] why Mutation moved the spells' releases (93, 120, 60, 100) — design
- [[281]] AreaFx::spell requests a spell at its first system call, so +0x70 is the target's place at count 0 — a runtime asking earlier must pass it

### audio

- [[327]] whether the sound thread runs `sdCommand` during the jukebox's CD load (reverb off for the load's length) — the harness loads at once
- [[14]] the 48 kHz PCM rate rests on the SPU2's fixed rate and measurements, not on code — docs/formats/voice.md Unknown
- [[14]] the SPU2's exact ADPCM rounding — still open in [[243]] (port rounds as PCSX2); needs a console
- [[42]] the SPU2 itself (envelope timing, ENVX/ENDX, interpolation, reverb) from documentation, not hardware — docs/engine/sound.md Unknown
- [[50]] the load-confirmation jingle's m_tempPN never initialised — game UB; port starts at 0
- [[60]] no check compares the music with the game's playing — only the tables and call order
- [[89]] --voice not in any playthrough test — test gap
- [[150]] the travellers' hum not listened to against the game — follows the code
- [[184]] the canals' and the church's volumes not compared with the game playing — docs/engine/sound.md Unknown
- [[185]] Dun Loireag's sequence 2 not listened to — follows the code
- [[243]] the SPU2's ADPCM rounding — needs a console (docs/formats/snddata.md Unknown)
- [[245]] whether a Recovery Drink used by Kite on Quarantine stops voice channel 0 on a console — follows the code (docs/formats/voice.md Unknown)
- [[245]] whether other characters' skill-word bases reach their tables' last rows — only Kite's traced

### video

- [[49]] the IPU's IDCT exact arithmetic — needs a console
- [[49]] what the SPU2 plays after the last audio block until audioDecReset — not checked
- [[49]] how many frames the console takes before the first picture — not measured
- [[52]] the frames the disc read and decoding take — not modelled ([[175]] reads ahead on a thread)
- [[52]] the intro's end flash drawn after stream 0, not over it — docs/engine/title.md Unknown (placed 20 frames in by inference)
- [[85]] the exact frame ccEventStream's loop sees the stream end — docs/engine/stream.md Unknown
- [[105]] streams 37, 44, 67, 69 and 100 (a character's drain by id) not played — docs/engine/field-ui.md "Not yet known or not ported"
- [[133]] how many frames the game loads between str0580 and str0581 — not known (docs/engine/stream.md Unknown)
- [[133]] str0581's frame-0 note (cue 900) given to the task's second pass — not checked (docs/engine/stream.md Unknown)
- [[175]] the streams' own load left as it is — the game streams from disc as it plays
- [[240]] which demo disc each STRT.BIN scene belonged to; how its menus' selection worked — not in the files
- [[249]] whether the four discs' LOGO_B.PSS and LOGO_C.PSS are the same file — the build's dedup would say
- [[267]] no harness compares these tasks with the game's code frame by frame — test gap
- [[269]] whether OBJ_xpart00-05's models carry a Bbox (the game would drop parts the port keeps) — not checked
- [[388]] the console's own Movie 13 and Movie 16 played from the desktop's Audio screen not seen — needs a PS2 or PCSX2 capture

### ui

- [[326]] whether a script's fade and a menu flash ever overlap in play (the game shares `ccMenu +0xb8`; the port keeps two faders) — not surveyed
- [[9]] the meaning of mailFlg and reFlg — their writes are ported (docs/engine/desktop.md "Mail"); readers unnamed, still in docs/engine/text.md Unknown
- [[23]] how the GS samples the kt 0 and kt 2 sprites whose sx and su differ by a texel — docs/engine/font.md Unknown (renderer assumes top-left sampling)
- [[23]] which view and scale the font layer uses — docs/engine/font.md Unknown
- [[23]] where the ccKanji texture sits in VRAM — docs/engine/font.md Unknown (desktop.md: one scratch page re-uploaded per kanji)
- [[43]] the boot check's timing and the stream's end-flash alignment are inferred — not measured
- [[45]] the kana, symbol and kanji grids of name entry, not reached in the US build — unreachable
- [[51]] LOAD_FRAMES: the real time between ChangeRequest and ccSetupDesktop's ccAllSoundOff — docs/engine/title.md Unknown
- [[58]] whether the top page task's first Main runs in the frame it starts — docs/engine/toppage.md Unknown
- [[58]] how the fader behaves while the system menu freezes the layers — docs/engine/toppage.md Unknown
- [[58]] the setup passes run at once and draw nothing — timing of set-up passes not modelled frame by frame
- [[66]] a menu a button opens stops the camera and player one frame later than the game (world tasks before the menu task) — task order; same kind as [[76]], [[101]]
- [[68]] the gate-target fix has no eemu check of its own — test gap
- [[69]] menu 79 bumps its wait twice a frame, so Kite is mid warp-out at the change — as the game (checked)
- [[70]] no runtime shot of a shop — covered by checks
- [[71]] the equipment, party-add and Controller tasks finish one frame later than the shortest the game can take — disc timing
- [[71]] past the equipment tables the port reads zeros where the game reads what follows — game UB
- [[71]] the OPTION helper's CD load real frame count — not measured
- [[73]] PresentMenu's states 20-22 and BreedingMenu's 23 with index 0 never set; a trader of neither kind reads a stale register — as the game
- [[75]] no shot shows the world moving under a tutorial menu — follows the code
- [[76]] camera 3 appears one frame later than the game (menus run after the world's tasks) — task order
- [[77]] what sets WORLD_MAN.specialRoom (+0x160) beyond the known rooms — docs/engine/map.md Unknown
- [[77]] RT01ICONPOS and the pulse angles are the game's globals; the port starts a new area's pulse at 0 — docs/engine/map.md Unknown
- [[77]] the map for special dungeon types 8 and 9 and story rooms of type 15 and up not checked; labels compared as strings only — test gap
- [[94]] the staff roll's pixels and the ending song's start and stop not compared — not compared
- [[94]] ccBufferSampling (+0x138c) and the second mask (+0x1358) created and never used — as the game
- [[96]] after success proccess stays 2 until the area changes; how many frames the game shows — not measured
- [[100]] the minimap's markers not checked frame by frame — tools/test_map_rs.py covers the rule
- [[104]] the field noise's pixels not compared — no capture
- [[144]] the menu's turn uses f32 sine and cosine, not the VU's cossin — a sixteenth of a pixel at most
- [[144]] a rotated cell's off-screen test reads a leftover width and height — the port takes 0 (docs/engine/map.md Unknown)
- [[148]] the object menus not compared with the game's pictures — checks are frame by frame in eemu
- [[158]] the game-over noise's draws not checked frame by frame against ccGameOverNoise — counts tested
- [[159]] ccMessage's rate-2 drawing (its button's pulse) not compared — harness hooks it
- [[174]] whether the game's info_now on the set-up screen places its lines where SetupScreen does — not compared
- [[174]] the tail of the last window before stream 10 not timed — not timed
- [[179]] the crisis town's noise not seen in a picture — follows the code
- [[186]] how long the game's loads take (how much of the loading display's fade and logo a player sees) — not measured
- [[186]] the load request's +0x18, which also stops the display — setter not traced
- [[190]] the spells' screen noise not compared with the game's picture — no capture
- [[193]] the ALL PORTALS OPEN banner not compared with the game's pictures — no capture
- [[194]] the panels' shake and the protect marks not compared with the game's pictures — no capture
- [[196]] the present's remark not run in a town with three members — test gap
- [[226]] whether the PS2 shows the recall's fades black too — every checked step matches the game
- [[237]] the port's quit prompt uses the desktop's window where the field's own confirmations use the field menu's — a port window, looks the same
- [[247]] nothing tells the player that a finished part's save loads in the next; New Game from a previous volume's save not checked from a build — port launcher; the import itself answered by [[387]]
- [[251]] whether the unused selector had music of its own — no code survives
- [[253]] whether the unused selector drew a copyright of its own — no code survives

### script

- [[328]] near_marker while Kite rides a carrier (`GetTransMode`): which areas do it — not surveyed; no test
- [[11]] which story areas Infection itself can reach — derivable from the scripts' gate_add (events.md op 89/90); no list made
- [[18]] how far the parody script diverges beyond the lines compared — only sampled
- [[69]] the event camera's unknowns — listed on docs/engine/field-game.md (see Docs)
- [[91]] player_distance in town not run whole against the game — reuses checked pieces
- [[91]] the church's messages, cameras and timing only seen to run — not compared
- [[99]] piros_colour status 9 with an operand past 5 — no script uses it
- [[107]] whether block 18's in_point 5 gates block 19 in the dungeon's first room — not looked into
- [[143]] event 27's talk (walking PC row 85) not played through by a test — test gap
- [[177]] event 62 not played through end to end (mail, words, portal, scene back) — test gap
- [[178]] walks along the town navigator's route (gPoint not -1) not ported for Kite — no event sets one
- [[178]] event 30's walks in Dun Loireag not run in a test — same code as Mac Anu's
- [[178]] the side events not played to their ends — need a player
- [[180]] event 61 driven only to its opening lines; events 58 and 60 without tests — test gap
- [[189]] whether a player of Infection alone reaches friendship 50 with Black Rose by event 14 — no play measured
- [[228]] what sets and clears eventStatus[40] in Mutation's story — not followed
- [[270]] whether the game delivers mail 88 in the same desktop session as the port — not checked against the executable
- [[277]] BlackRose's own lines at the holy ground once her talk target was right — not checked

### save

- [[45]] save_va: the real heap address NewGame stores as character 0's name pointer — not measured
- [[45]] real disc and card load times — the port loads at once
- [[50]] whether ccSaveSys keeps its card position through a reset — not checked
- [[70]] the Recorder's card position follows the title's Load, not a later desktop Data save — cursor start only
- [[73]] the card calls finish at once, so the Recorder's card-busy frames do not show — card timing
- [[159]] the card-busy messages never on screen; a console card's time — docs/engine/desktop.md Unknown
- [[193]] what reads the save's portal counts (+0x6864, +0x6866, +0x6868) — docs/engine/effects.md Unknown
- [[246]] what each eventStatus index counts — belongs to the scripts that use it
- [[254]] whether a player wants a save's own options to win when loading a card made elsewhere — port design choice

### engine

- [[15]] ccSystem::Ctrl's other branches (screen-mode changes, flags at +0xbd4/+0xbd5) — only the vibration countdown was read ([[232]])
- [[69]] the load before F0 (ccLoadResourceFL) takes no frames — docs/engine/event-vm.md Unknown
- [[82]] the load time between the mode change and the desktop's set-up — docs/engine/event-vm.md Unknown

### input

- [[62]] the DualShock 2's real values at full diagonals — recalled, not measured
- [[62]] half-way diagonals read slightly stronger; the analogue walk may feel different — not measured
- [[232]] the rumble not felt on a real pad (gilrs needs the pad's force-feedback node) — environment
- [[232]] the pad's connection states (ccPad::Ctrl's other states, scePadInfoAct) not modelled — a connected pad is a ready DualShock

### volumes

- [[20]] which compiler options changed between MUT and OUT — not revisited
- [[25]] whether hardware sqrt.s rounds as eemu models it — needs a console
- [[173]] only unnamed stretches of 8 KB or more looked at; how much of OUT's and QUA's unnamed code is Infection's recompiled — would need shape matching
- [[205]] Mutation's remaining carry misses (@1489 in ChangeScene, @5293-@5295 in Execute, InitSpcParam's default item lists) — not revisited by name
- [[205]] Outbreak's and Quarantine's 29 and 28 carry misses — [[206]]: 23 and 28
- [[221]] the code that reads the new ccSkill vector and ccAI fields — [[223]] found +0x24; [[224]]: the hit fields beyond item_first unread
- [[222]] Mutation's new ccAI fields (+0x24, +0xa0) and the ccSkill vector — +0x24 read in [[223]]; the rest unread ([[224]])
- [[224]] the port keeps 750 gimPos slots; Mutation searches only the first 75 per floor with fewer than ten floors — rare case
- [[224]] what Mutation's new ccAI hit fields drive beyond item_first — unread
- [[227]] what talkNum[1] stands for on Mutation and what sets it — the port only reads it
- [[227]] where the English party tables are short, whether the game plays the next table's rows — not known
- [[235]] whether a Mutation save can ever make events 2-4 run — their conditions need events 1 and 2 done
- [[239]] the names of the functions in the unnamed stretches — survive nowhere
- [[248]] the level a player carrying Infection's clear data brings to event 104 — not measured
- [[250]] the level and equipment a player brings to Mutation's dungeons — not measured

### format

- [[324]] whether a name built at run time (sprintf names, names read from enemyTbl/gimmickTbl/npcTbl) can miss its category table — every static file list on all four volumes was checked by [[324]] and hits; the built names were not
- [[242]] what category 19 (a loose scene file outside the archive) was for — not in the code

### decomp

- [[4]] EE encodings the game never uses (debug/perf moves, rsqrt.s, max.s/min.s, vrnext/vrget, pmfhl variants) are unconfirmed by any binary — decoded from manuals only
- [[6]] the four-byte member missing from ccChunkIndex and 18 other classes' unexplained gaps — not revisited

### build

- [[259]] whether root() (the repository path, compiled in) is reached in a build run on another machine — untested

### tooling

- [[63]] piney-eemu: subclass overrides of load/store/set do not reach the interpreter; exec overrides run slowly — tool limitation
- [[63]] piney-eemu RAM cannot grow past 32 MB; registers are live views — tool limitation
- [[63]] iopemu's machine stays on eemu.py — tool limitation
- [[82]] a replayed story start lacks earlier experience and items; mails and posts as ccEventFlagSet leaves them — story starts are a test tool
- [[82]] story:12 starts on the desktop where the game's first block would run in the top page's set-up — story starts are a test tool
- [[86]] the unported-default counts say which ran, not how often — tool limitation
- [[125]] stream_shot has no party members (the field's StreamMenu draws them) — a shot tool limit
- [[148]] Kite starts event 29's dungeon session as a ghost (dead 4) — may be the story start's doing
- [[149]] a mid-dungeon story start leaves the room undrawn in its picture — story-start tool
- [[161]] decoding sd9 a second time after sd4 makes the interpreter's DecodeSetup read a bad pointer — test ordering (docs/engine/dungeon.md Unknown)
- [[176]] the host-call check plays blocks in isolation; methods reached only through earlier state may not show — method limit
- [[181]] whether xfer's ref and content passes misname anything beyond the tables they were built for — statics equal row for row
- [[182]] a reader that swallows its error (.ok()?) would not show in smoke runs — method limit
- [[198]] the voldiff "recompiled" rule is a heuristic (a changed field or swapped comparison passes) — method limit
- [[198]] some OUT/QUA "changed" menus call nothing Infection's do, pointing at misplaced carried names — to check in xfer.py
- [[202]] finders for Mutation's 8 carry misses; OUT's and QUA's 44 — [[205]], [[206]] leave 5, 23 and 28
- [[204]] the remaining carry misses — [[206]]: MUT 5, OUT 23, QUA 28
- [[204]] readers that take an Infection constant through a shape the scan does not see — not surveyed
- [[206]] the remaining carry misses — tooling
- [[219]] the other harnesses still name some Infection addresses of their own — moved to va() as each runs on a later volume
- [[233]] the game's log lines (stderr) do not reach the in-game console; no horizontal scroll — port console
- [[241]] nothing records from the window; a key to record live play — port tool idea
- [[247]] the window's switch from the launcher to a part checked headless only — not seen in a window
- [[255]] how long an image's first start takes on a slow drive — not measured
- [[258]] the default build at ~/.local/share/piney/game at data version 3 has to be remade — operational
- [[262]] the dialogue scan misses lines under four words — tool limit
- [[262]] text in the games' textures is not text to the scan — tool limit
- [[262]] the history's blobs not scanned; the history will not be published as it is — plans/release.md
- [[271]] why piney-gen syms did not carry Func_str7100, Func_str8800, Func_str0300 to Mutation — a guess: jump-table relocations
- [[276]] how the autopilot run compares with a player's play in time — fights are long
- [[395]] tools/test_grunty_rs.py fails on Outbreak and Quarantine before a Grunty is made (`__vt__10ROOTTOWN01` not named there) — answered by [[397]]: the harnesses find ROOTTOWN01's vtable through its constructor (+0x1b0 there), leave the unnamed `AwakeDistantLight` to run, and find the Grunty's `dogAction` as the function before `dogAction2`; the grunty harness matches on all four volumes

## Answered

- [[1]] SNDDATA.BIN's format — answered by [[14]] / docs/formats/snddata.md
- [[1]] ICON.BIN's format — answered by [[16]] (iconBinTbl, twelve icons) / docs/formats/save.md; noted in [[239]]
- [[1]] the voice banks' format — answered by [[14]] / docs/formats/voice.md
- [[1]] BGM.BIN's format — answered by [[14]] / docs/formats/voice.md "Streamed music"
- [[1]] the STREAM/ files' format — answered by [[14]] (Pcm chunks), [[52]] / docs/formats/ccs.md, docs/formats/voice.md
- [[1]] whether OUTSIDE.BIN is read by anything — answered by [[239]] (no string OUTSIDE in any volume's code)
- [[1]] what IOPRP243.IMG replaces in the IOP's ROM modules — answered by [[239]] / docs/disc/layout.md "The IOP image" (15 ROMDIR modules)
- [[2]] relocation type 123 — answered by [[243]] (micro address >> 3 in the 15-bit immediate) / docs/engine/executable.md
- [[2]] what the mc_* symbols resolve to — answered by [[243]] (VU entry points, 21 referenced)
- [[2]] how much of the DWARF survives — answered by [[6]] (survives complete; tools/dwarf1.py)
- [[2]] what the MWo3 header's overlay id is used for — answered by [[242]] (nothing reads it) / docs/formats/prg.md
- [[4]] the VU1 microcode is not disassembled — answered by [[17]] (tools/vu.py, tools/test_vu.py)
- [[5]] what fills directCCSTbl and which files use category 19 — answered by [[242]] (ccAddFileList; no volume adds one) / docs/formats/data-bin.md "Category 19"
- [[5]] the two s16 fields (+0x24, +0x26) of a sceneFileList entry — answered by [[324]] (deleteCnt, a reference count; addFlag, always -1) / docs/formats/data-bin.md "File-list entries"
- [[5]] whether any requested name can miss its category table — static lists answered by [[324]] (all 679 rows hit on the four volumes); run-time names carried to [[324]]
- [[6]] the contents of the .mwcats sections — answered by [[243]] (per-function size records) / docs/engine/executable.md
- [[9]] how the game decides which mails a volume delivers — answered by [[41]] / docs/engine/desktop.md "Mail" (event opcodes 107-109; event 1 delivers 4, 5, 320)
- [[9]] what ccGetExtendedCode maps and how % codes are drawn — answered by [[23]] / docs/engine/font.md
- [[9]] the BBS dateindex — answered by [[58]] / docs/engine/toppage.md (index of the date cell, 0-5 plain, 0 parody)
- [[9]] whether Parody Mode can be switched on in the US Infection — answered by [[43]] (m_ParoFLG only cleared; cannot be chosen)
- [[10]] rigid ITOF12, ST scale and culling inferred from the EE side — answered by [[17]] (the microcode settles all four) and [[244]] (ST scale)
- [[10]] the 15 extra words per mmat in the size field of mtype & 4 models — answered by [[244]] (overstated size, nothing reads it) / docs/formats/ccs-model.md
- [[10]] the model flag bits above blend type and what zoffs does — answered by [[244]] (bit 3 dropped; zoffs never read)
- [[10]] Anime sub-kinds other than 0x0102 — answered by [[32]] / [[244]] (all eleven kinds, docs/formats/ccs.md "Anime sub-kinds")
- [[10]] texture and CLUT flag bits and the CLUT's unknown_1 — answered by [[244]] (unknown_1 is cpsm) / docs/formats/ccs.md "Texture and CLUT flags"
- [[11]] how enemyOfs/itemOfs feed the enemy and item tables — answered by [[13]] (enemy lists), [[275]] (story areas use column 6), [[68]] (AreaItem, crates/piney-fieldui/src/menus/getitem.rs area_item)
- [[11]] what EVENTAREA_INFO.flag does — answered by [[179]] (WORLD_MAN::GO copies it to hackFlag; 3 = hacked, turns on interNoiz)
- [[11]] what EVENTAREA_INFO.model does (in part) — [[112]] (sound bank 5 by model), docs/engine/evarea.md (model 1 = a story map file); model beyond these still in docs/engine/area-words.md Unknown
- [[11]] what SetDungeonTypeFromField decides — answered by [[19]] (the dungeon type; 12,288 inputs checked) / docs/engine/dungeon.md
- [[11]] the save flag behind area 71 — answered by [[220]] (eventFlag[217] bit 62, event 217 done; +0x5bc0)
- [[12]] what the enemy table's type, Exdefense, entry.* and AI fields drive — answered by [[22]], [[67]], [[78]] / docs/engine/battle.md (type bits, Exdefense +0x64, entryEnemy); only type bits 0x100/0x200 remain (battle.md Unknown)
- [[12]] how enemyList00..15 and enemyOfs pick rows — answered by [[13]], [[275]] / crates/piney-battle/src/entry.rs register_list (tools/test_battle_spawn_rs.py)
- [[12]] skill and item effect fields — answered by [[22]], [[67]] / docs/engine/battle.md, crates/piney-battle/src/skill.rs, item.rs
- [[12]] the msg pointer in the character and boss rows — answered by [[208]] (an address of its message table)
- [[13]] the race/drain expansion for type-64 entries — answered by [[78]]: 0x40 rows are middle bosses, their base and drained forms registered (crates/piney-battle/src/entry.rs Register::register; tools/test_battle_spawn_rs.py); docs/engine/area-words.md still says "not modelled" (stale)
- [[13]] what ccGame.field holds — answered by [[84]] / docs/engine/evarea.md (the story area number, eventAreaNumber; 0 for a generated area)
- [[13]] how the three registered enemies become encounters (magic portals, circleOfs) — answered by [[78]], [[164]] / docs/engine/battle.md "Where the entries come from"
- [[13]] how itemOfs picks treasure — answered by [[68]] / crates/piney-fieldui/src/menus/getitem.rs area_item (tools/test_battle_flow_rs.py per [[67]])
- [[14]] the .sq MIDI content and which sequence of a bank the jukebox plays — answered by [[42]] (sequencer) / crates/piney-data/src/sound/mod.rs Tables::wave_sequence (1 for rows 47, 27, 7)
- [[14]] the sound-effect maps (seData, SE tables, strSndTbl) — answered by [[42]], [[81]] / docs/engine/sound.md; [[243]] notes the question was stale
- [[14]] what parodyFlag changes in ccEvVoiceRequest — answered by [[95]] / docs/formats/voice.md (events 0-49 have no voice in Parody Mode)
- [[14]] the Pcm chunk's type, bitNum, trackType — answered by [[245]] (Decode_Pcm uses only dataNum and dataSize)
- [[15]] ccEvent::Execute, the event script interpreter — answered by [[18]], [[40]] / docs/engine/event-vm.md (overview.md's bullet is stale)
- [[15]] what modes 1 and 0x1000 do — answered by [[331]] (1: the soft reset, from both Reset menus and the game over; 0x1000: every task deleted, asked for by nothing on INF) / docs/engine/overview.md
- [[15]] the full list of tasks each mode starts and their priorities — answered by [[334]] / docs/engine/overview.md "Tasks"
- [[16]] what mcFname[0] (BISLPS-00000HUCKER) is for — answered by docs/formats/save.md (nothing uses it; only mcFname+4 is referenced)
- [[16]] the icon.sys fields the game fills in — answered by docs/formats/save.md (title from mcTitleName, break at byte 10)
- [[16]] which of the twelve icons each volume uses — answered by docs/formats/save.md (volume v uses iconBinTbl 3(v-1) to 3(v-1)+2)
- [[16]] the meaning of most event-flag bits — answered by [[40]] / docs/engine/events.md (bit 62 done, 63 closed); [[246]]
- [[16]] whether ConvGame is reachable — answered by [[24]] / docs/formats/save.md (ccThDemo calls it when the later titles' opening returns 4)
- [[16]] how the later volumes read Infection's saves — answered by [[24]], [[200]] / docs/formats/save.md (LoadInfoPrevReq, LoadDataPrevReq)
- [[17]] the exact z the V2 unpack delivers — answered by [[244]] (PCSX2's unpacker copies x,y into z,w; z = S + bits(1/16)); rests on PCSX2's source, not a console
- [[17]] the shadow programs beyond their entry points — answered by [[160]] / docs/engine/shadow.md
- [[18]] the numbering of streams, markers, menus, sound commands, player operations, NPC/PC action codes — answered by [[176]], [[178]], [[179]] / docs/engine/events.md Unknown ("nothing about the numbering is unknown any more"); npc_act -3/-5 unported (MUT event 114 only)
- [[18]] the mail and BBS state values — answered by docs/engine/events.md (bbsList 0/1/3/7; mail 4-6), [[58]]
- [[18]] the camera angle units — answered by [[179]] / docs/engine/events.md (DEG2RAD shorts, 65536 a turn; distances in tenths)
- [[18]] whether the ML events (400-455) can open in Infection — answered by [[189]] (twelve ML events open in Infection)
- [[19]] the treasure, circles and idols EntryGimmick places later — answered by [[98]] (dungeons), [[164]] (fields): placed at run time by the ported setters, not derived from the seed
- [[19]] story-dungeon room models — answered by [[114]] / crates/piney-data/src/dungeon/special.rs event_room (docs/engine/dungeon.md Unknown bullet is stale)
- [[19]] object heights — answered by [[38]] (how the game draws a field: object heights)
- [[20]] what QUA uses KFED.BIN and KFAED.BIN for — answered by [[239]] (loaded around the staff roll, never read)
- [[20]] what OUT's STREAM/STRT.BIN and the VOICE2 banks hold — answered by [[239]], [[240]] (demo-disc scenes; SPC18-20 skill words)
- [[20]] whether OUT/QUA's missing CDVD modules moved into IOPRP243.IMG — answered by [[239]] (no; every disc uses the image's own)
- [[20]] what the functions new in volumes 2-4 are — listed by [[173]], [[239]] / docs/disc/unnamed-code.md (423 stretches); their names survive nowhere (research)
- [[20]] whether anything but the index changed in the later volumes — answered by [[21]], [[24]], [[25]], [[198]] (area generator, scripts, save, generators, battle, text; voldiff audit)
- [[21]] whether DUNGEON::Generate and WORLD::Generate match on the other volumes — answered by [[25]] (0 mismatches on MUT, OUT, QUA)
- [[21]] what the save flag at saveData+0x5bc0 bit 62 records — answered by [[220]] (eventFlag[217] bit 62, event 217 done)
- [[22]] the Data Drain drop roll and side-effect table, read not run — answered by [[102]] (tools/test_battle_drain_rs.py, tools/test_fieldui_rs.py test_data_drain), [[141]]
- [[22]] _ccSkillRequest's attribute-critical roll and area splash damage — answered by [[67]] (tools/test_battle_rs.py: ccSkillDamage single/area/point, _ccSkillRequest)
- [[22]] the effects of sleep, paralysis, charm and confusion on behaviour — answered by [[67]], [[78]] / docs/engine/battle.md "Enemy AI", "Status effects"
- [[22]] boss skills (_ccBossSkillDamage) — answered by [[109]] / docs/engine/boss.md
- [[22]] item use — answered by [[67]], [[230]] / crates/piney-battle/src/item.rs (tools/test_battle_items_rs.py)
- [[22]] enemy AI beyond its structure — answered by [[67]], [[78]] (tools/test_battle_enemy_ai_rs.py)
- [[22]] what noDeathFlag means — answered by docs/engine/battle.md (a member with it: HP and SP only rise), [[192]]
- [[22]] whether plcol is the bracelet — answered by docs/engine/battle.md (data_drain sets it with Kite's skill 2)
- [[22]] whether volumes 2-4 changed the battle rules — answered by [[25]] (Exdefense, area skill), [[221]]
- [[23]] what Quarantine loads KFED for — answered by [[239]] (read into two buffers, never read)
- [[23]] the line spacing of mail and board bodies — answered by [[37]] / docs/engine/desktop.md "Mail" (body lines at 48 + 17 r); docs/engine/font.md Unknown bullet is stale
- [[23]] whether the text code changed in volumes 2-4 — answered by [[25]] (OUT's ccKanjiStrlen counts every %x pair)
- [[24]] what the Mutation-on blocks at +0x8432 and +0x8462 hold — answered by [[246]] (zeroed by Init, copied, never read)
- [[24]] the extension's 3 bytes at +0x748 — answered by [[200]] / docs/formats/save.md (pcTradeCount of characters 18-20)
- [[24]] the extension's 256 bytes at +0x754 — answered by [[246]] (cleared and copied, nothing else)
- [[24]] the save bit at +0x5ec8 bit 62 — answered by [[220]] (eventFlag[314] bit 62, event 314 done; the random-path dungeon flag)
- [[24]] what the ending routine does while the KFED buffers are loaded — answered by [[239]] (gcmn 0x004f0a30 starts STFROLL_VOL4 and waits)
- [[24]] what OUT's rewritten cases 6, 8, 9, 60, 71, 72 and 150 do — answered by [[325]] (INF's behaviour; inline writes moved into small functions) / docs/engine/events.md
- [[25]] what saveData+0x5ec8 bit 62 means — answered by [[220]] (eventFlag[314] done)
- [[25]] MakeFloor's story path with more than 10 floors — answered by [[181]] (area 125's 15 floors carried) / crates/piney-data/src/dungeon/mod.rs Tables::edit_floor_count
- [[25]] ccSkillDamage's area path — answered by [[67]] (tools/test_battle_rs.py, ccSkillDamage area)
- [[25]] the Data Drain drop roll — answered by [[102]] (tools/test_battle_drain_rs.py)
- [[26]] the unit of the material crop offsets — answered by [[29]] (a UV animation's reference, not an offset)
- [[26]] the lit models' lighting (viewer headlight) — answered by [[47]] (VU1's lighting), [[61]] (the town's lights)
- [[26]] town01's missing water — answered by [[28]] (wat1, the canal water)
- [[26]] animation key interpolation — answered by [[32]] (keys, rotations, loops, morphs checked)
- [[27]] the canal water's placement (wat1, three duplicates) — answered by [[28]] / docs/engine/statics.md, docs/engine/root-towns.md
- [[27]] the sky and CMP_sr1dat1_* clumps the constructor builds — answered by crates/piney-world/src/town01.rs (sky, CRISIS_SKY) (tools/test_world_rs.py)
- [[27]] which LOD DrawObj picks and the flags at +0x1b0/+0x1b4 — answered by crates/piney-world/src/town01.rs (rows 11/8 swap to 29/30 by y; clip[3] at +0x1b0..+0x1b8)
- [[27]] animation beyond frame 0 — answered by [[32]], [[35]]
- [[28]] why the water is three copies, their timing and blend — answered by docs/engine/root-towns.md (waters 1 and 2 are made and stepped once, never drawn; water 0 drawn), [[97]]
- [[28]] the waves (morph playback) — answered by [[36]] (morph targets blend)
- [[28]] what town_z holds — answered by docs/engine/statics.md (no models: the lens flare's and clouds' seven Eff chunks), [[97]]
- [[28]] how EVENTAREAB0 picks its area — answered by [[112]] (fields 1-8; area 27's last door)
- [[28]] what draws type 0 rows — answered by docs/engine/statics.md (ROOTTOWN01::Draw draws them itself; other classes' not traced)
- [[29]] the unit of the runtime UV offset and where ccSetMaterialPacket makes the ST row — answered by [[137]] ((u >> 4) & 0xff in 1/256 of the texture, through STROW)
- [[30]] the story dungeons' room models — answered by [[114]] / crates/piney-data/src/dungeon/special.rs
- [[30]] the item boxes, circles and idols EntryGimmick places — answered by [[98]], [[164]]
- [[30]] the start positions of rooms without a lake entry (OBJ_0ppp) — answered by docs/engine/dungeon.md "The start" (crates/piney-world/src/dungeon_area.rs, tools/test_dungeon_rt.py)
- [[30]] the other volumes' dungeon tables — answered by [[181]], [[265]] (generated per volume)
- [[31]] the rotation and placement rule that makes rooms join — answered by [[34]] (the room matrix was right)
- [[31]] the door models and where they go — answered by [[34]], [[87]]
- [[31]] per-type fog and ambient, and room lights from LGT_ objects — answered by [[34]] (fog), [[157]] (SetLight), [[151]] (direct lights)
- [[31]] whether rooms off the player's neighbourhood are drawn together — answered by docs/engine/dungeon.md "Assembly" (DUNGEON::Draw draws one room at a time)
- [[32]] how the draw turns ccMaterial.u/v into the ST row — answered by [[137]]
- [[32]] the light, camera, ambient, F_Obj and note records — answered by [[137]] (read where played; F_Obj ported), [[139]], [[278]] (notes)
- [[32]] whether the field changes ccAnm::frameSpd — answered by [[137]] (yes: Kite's AnimCtrl, motions, act -5)
- [[32]] that the field runs at 30 frames a second — answered by [[59]] (frame rate 2)
- [[33]] object heights from WORLD::GetHeight — answered by [[38]] / crates/piney-data/src/field/render.rs Field::get_height (178 points checked)
- [[33]] how the ground mesh is built and where the cover is drawn — answered by [[38]]
- [[33]] what is rolled after the start position: weather and EntryGimmick — answered by [[130]] (weather), [[164]] (EntryGimmick)
- [[33]] the other volumes' field tables — answered by [[188]]
- [[33]] the keyword-to-seed step — answered by crates/piney-data/src/area/mod.rs sim_generate_code (tools/test_area_rs.py, [[220]])
- [[34]] SetLight's omni lights and glow effects — answered by [[157]] (SetLight ported; GAPS.md "Dungeon rooms' dressing")
- [[34]] the clutType 3/4 palette swaps — answered by [[147]] (SetClutList, ChangeClut)
- [[34]] GetBG for the lake dungeons and the white plane in lake doorways — answered by [[161]] (GetBG picks the sky domes; the sky shows through the doorways)
- [[34]] whether the GS fog uses view depth as the viewer assumes — answered by [[145]] (VU1's per-vertex fogB + fogA * w; piney_draw::DepthFog)
- [[35]] the morph blends (F_Morpher cloth) — answered by [[36]]
- [[35]] texture-offset animation — answered by [[137]]
- [[35]] the animation's transparency channel, read but not drawn — answered by [[100]] (objects drawn at their own transparency), [[137]] (F_Obj localtp)
- [[36]] texture-offset animation — answered by [[137]]
- [[37]] the desktop's other modes: News, Accessory, Audio, Data — answered by docs/engine/desktop.md "News", "Accessory", "Audio", "Data" (crates/piney-desktop)
- [[37]] the event scripts that deliver mail and lock icons — answered by [[40]], [[41]]
- [[37]] the desktop's sound printed, not played — answered by [[42]], [[46]]
- [[37]] DEMO.PRG's title and opening, name entry, TOPPAGE.PRG — answered by [[43]], [[45]], [[58]]
- [[38]] how the fog table's near, far and percentage map to the GS fog — answered by [[145]] (DepthFog::set_fog is SetFog's arithmetic); docs/engine/field.md Unknown bullet is stale
- [[39]] the viewer's missing draw window and fade, sky scroll and player-centred background — answered by [[72]] / crates/piney-world/src/field_area.rs (DrawBG centred on the player, the sky's V scroll), crates/piney-data/src/field/render.rs (FADE_FROM 6700, COVER_RANGE 7200)
- [[39]] the viewer's fog depth mapping — answered by [[145]]
- [[40]] the story areas' gate words and servers not loaded from the executable — answered by [[181]], [[217]] (generated per volume) / crates/piney-data/src/area/mod.rs AreaTables
- [[40]] piros_colour's frame shape — answered by [[99]], [[107]] (ported, tools/test_piros_rs.py)
- [[40]] the field side of the Host (markers, distances, entries) — answered by [[74]], [[91]], [[134]], [[180]]
- [[40]] the later volumes' own semantics in the VM — answered by [[325]] (mode, fade, fade_more, desktop_item, call_lock, characters 18-20 by volume; `later_volumes_rules`)
- [[41]] the scripts' windows on the desktop — answered by [[45]] (the setup stage draws the event's windows)
- [[41]] name entry (NameEntry_Control) — answered by [[45]]
- [[41]] the script's movie streams and overlay loads skipped — answered by [[49]], [[52]] (streams), [[91]] (the desktop's overlay and pass)
- [[41]] what the setup screen shows during event 1's 59 frames; DEMO.PRG's title and opening — answered by [[45]], [[43]]
- [[42]] the field, town, dungeon and event music contexts and battle-music switching — answered by [[60]], [[185]] (towns), [[80]] (battle music, docs/engine/sound.md "battle music bgmChange")
- [[42]] voice lines, cutscene PCM, the 3D sound effect API — answered by [[46]], [[95]] (voices), [[52]] (stream PCM), [[81]] (ccSeOn3D)
- [[42]] sdCommand's reverb switches (cases 4 and 5): who asks for them — answered by [[327]] (ccSndChangeData around its load; the port's `Command::Reverb`)
- [[43]] the title's load screen and Option menu stand-ins — answered by [[50]], [[51]]
- [[43]] [[201]] [[226]] CONVERT, a previous volume's save into a new part — answered by [[387]] (issue #55: `LoadInfoPrevReq`, `LoadDataPrevReq`, proccess 4/7/9, the `m_PrevFlg` branches, `ccStartEventConvert` and `ConvGame` ported; test_data_load_prev on the three later discs, ConvGameLaterVolumes, the_ending_s_save_converts_into_mutation) / docs/engine/title.md "CONVERT"
- [[43]] PSS movie playback and the stream — answered by [[49]], [[52]]
- [[43]] the title music context — answered by [[46]], [[51]]
- [[43]] SetNextData / PlayNextData — answered by [[201]] / crates/piney-demo/src/opening.rs set_next_data
- [[44]] NewGame(0)'s copy of charTbl into spcParam — answered by [[76]] (tools/test_save_init_rs.py)
- [[44]] the setup screen and name entry during event 1 are black — answered by [[45]]
- [[44]] the title's music, the PSS movies, the intro stream, The World — answered by [[46]], [[49]], [[52]], [[58]], [[59]]
- [[46]] the logos, OPENING.PSS and the intro stream counted as played; their audio — answered by [[49]], [[52]]
- [[46]] the field's voices (ccVoiceRequest negative ids, per-character voices) — answered by [[95]], [[101]]
- [[46]] the title's lit icons differing from the CPU GS model — answered by [[47]] (the directional term was doubled)
- [[47]] the desktop's and the field's lit models against VU1 — answered by [[47]]'s shared fix, [[61]] (the town's lights), [[141]] (tools/test_lights_rs.py: the light group against ccDrawEnv::SetLightMatrix)
- [[47]] the "transparency" report — answered by [[48]] (FB_ONLY drew colour before Z)
- [[49]] the other volumes' movies — answered by [[245]] (later_openings_match_ffmpeg_exactly)
- [[50]] LoadGame's vibration — answered by [[232]] (piney_input::actuator)
- [[50]] [[51]] [[196]] Adjust Screen's display offset (`Request::DisplayOffset`) was dropped — answered by [[339]] (`Event::DisplayOffset`, the presenter moves the picture; present_moves_the_picture_by_the_display_offset) / docs/engine/overview.md "The screen"
- [[50]] LoadGame's camera scheme — answered by crates/piney-world/src/camera.rs (camera_mode from the save), CameraType requests applied (crates/piney-game/src/world.rs, area.rs); [[254]]
- [[51]] the port's all-sound-off cutting only the voices — answered by BUGS.md (the dungeon music under Helba's theme; all_sound_off_stops_the_music)
- [[52]] the streams' effect objects and effect tasks (StreamDemoFuncTbl) — answered by [[54]], [[57]], [[121]], [[122]], [[124]], [[125]], [[126]], [[133]], [[156]], [[160]]
- [[52]] the subtitles under a stream — answered by [[85]]
- [[52]] the sequenced music around a stream (ccSndStreamCtrl) — answered by [[85]]; per volume in BUGS.md (stream_ctrl takes the volume)
- [[52]] the Audio screen's movies have no test of their own — answered by [[388]] (issues #53, #54: audio_movie_draws_the_gate_in, audio_movie_draws_the_drain_s_resident_models, every_hash_object_resolves_where_its_stream_plays; stream 18 over the desktop's files in tools/test_stream_rs.py fixture) / docs/engine/desktop.md "Audio"
- [[56]] what ccSnd +98 does — answered by [[327]] (+0x62: the Audio screen's movie holds the stream sound calls) / docs/engine/sound.md
- [[56]] the port-0 branch of the volume — answered by [[327]] (no SQTBL plays on port 0; `port_vol_set` gives port 0 `se_vol`)
- [[57]] stream 2's fog — answered by [[127]], [[145]]
- [[57]] the table's 21 other effect tasks — answered by [[85]], [[121]], [[122]], [[124]], [[125]], [[126]], [[133]] (all of Infection's)
- [[58]] the field game (mode 5/6) — answered by [[59]]
- [[58]] `SoundFadeOut` stopped every sequence at once; the game's `ccSoundFadeOut` fades over 8 frames — answered by [[321]] (`Driver::sound_fade_out`, `ccSqFade(n, 0, 8, 3)` per sequence)
- [[59]] the event task in town (event 2's arrival with Orca) — answered by [[69]]
- [[59]] NPCs and the Chaos Gate — answered by [[61]], [[65]]
- [[59]] the HUD, map and field menus — answered by [[66]], [[68]], [[71]], [[77]]
- [[59]] leaving the town — answered by [[72]]
- [[59]] the town's bank and ccSndBgmCtrl's town case — answered by [[60]], [[185]]
- [[59]] footsteps and ambient loops — answered by [[138]] (footsteps), [[184]] (the canals)
- [[59]] the town's fog — answered by [[145]]
- [[59]] the water textured from the frame buffer — answered by [[97]] (water 0 drawn in both towns)
- [[59]] Kite's shadow — answered by [[160]]
- [[59]] the arrival's effect — answered by [[79]], [[129]] (towns run effTransfer)
- [[59]] other towns — answered by [[97]] (Dun Loireag), [[218]], [[260]]
- [[60]] ccSndBgmCtrl's branches for town types 1-4 — answered by [[185]] (types 1 and 3 start sequence 2 with three sequences; every town sequence 0)
- [[60]] the crisis flag ccSnd +0x105 and the three-sequence condition — answered by [[185]]
- [[61]] the merchants, the Recorder, the walking PCs and Orca — answered by [[65]], [[69]], [[70]]
- [[61]] the gate's menu — answered by [[68]]
- [[61]] water, fog, shadows and the arrival's effect in town — answered by [[97]], [[145]], [[160]], [[129]]
- [[61]] the user's movement stalls — answered by [[62]] (a round DualSense stick against a DualShock 2's threshold)
- [[64]] nothing calls FieldUi yet — answered by [[66]]
- [[64]] the Chaos Gate's menus (28, 57-62, 78, 79) — answered by [[68]], [[96]] (62, the gate hack)
- [[64]] the talk, trade and shop menus — answered by [[70]], [[73]]
- [[64]] the other tutorials — answered by [[68]], [[75]]
- [[64]] Key Items, Status, Equipment, PARTY, Gate Out, Log Out, OPTION pages — answered by [[71]]
- [[64]] Data Drain's own menus — answered by [[102]]
- [[64]] the field objects' menus — answered by [[98]], [[148]]
- [[64]] ccThGameCtrl (cmndTarget, menus from buttons) — answered by [[66]], [[77]] (the map button)
- [[64]] damage numbers — answered by [[79]]
- [[64]] the chat balloon's drawing — answered by [[120]]
- [[64]] ccNoiz — answered by [[104]]
- [[64]] rotated sprites drawn unrotated; the drain gauge's Gouraud cells — answered by [[144]]
- [[65]] the party's own AI (ccThFellow, ccAI): walking, turns, pc_act — answered by [[67]], [[78]], [[110]]
- [[65]] the administrators — answered by [[73]]
- [[65]] the other ccThGameCtrl buttons and the talk requests' menus — answered by [[66]], [[70]], [[77]], [[135]] (menu ban)
- [[65]] the walking PCs' texture variants, footsteps and dust — answered by [[139]]
- [[66]] the menu buttons not run in eemu — answered by [[80]] (tools/test_gamectrl_rs.py "frames": ccThGameCtrl with its buttons)
- [[66]] the menus' target requests (TargetFix, Target, TargetClear) dropped — answered by [[70]] / crates/piney-game/src/world.rs, area.rs apply them
- [[66]] the talk, shop and gate menus and the map button — answered by [[68]], [[70]], [[77]]
- [[66]] PERSONAL's last row drew " og Out" — answered by [[68]] ("Log Out lost its L")
- [[67]] the runtime's animation and notes — answered by [[78]], [[139]], [[278]]
- [[67]] enemy spawning and portals (ccThEntryCtrl, entryEnemy, entryMagicCircle) — answered by [[78]], [[80]], [[164]]
- [[67]] enemy movement and each race's action() — answered by [[78]], [[168]], [[169]], [[171]]
- [[67]] party movement (FollowPlayer ... ActInTown) and ccFellow::Action/Main — answered by [[78]], [[110]]
- [[67]] the spells' element systems — answered by [[79]], [[81]] (the five spell systems)
- [[67]] the Data Drain movie and transformation — answered by [[105]], [[229]]
- [[67]] damage numbers, hit marks and sounds — answered by [[79]], [[81]]
- [[67]] bosses: _ccBossSkillDamage, their affects and AI — answered by [[109]] (Skeith); [[266]], [[272]], [[274]] (Mutation's)
- [[67]] ccEnemyG::thinkGold — answered by [[167]]
- [[67]] the chat text — answered by [[123]]
- [[67]] ChangeEquipReport in a field or dungeon — answered by [[123]] (FieldWorld::equip_report)
- [[67]] a member's equipment remark in town (`ChangeEquipReport`) was dropped — answered by [[337]] (`World::equip_report`; a_member_reports_his_equipment_in_town)
- [[68]] the gate hack (62) — answered by [[96]]
- [[68]] GoToArea and ChangeArea go nowhere — answered by [[72]]
- [[68]] TransferOut dropped — answered by [[129]] (towns run effTransfer)
- [[68]] the AI orders dropped — answered by [[148]] (ChatMenu1-3)
- [[68]] Party Add made no character — answered by [[74]], [[90]], [[108]]
- [[68]] the white flash of the gate menu in town was dropped — answered by [[326]] (`FieldUi::gate_flash` on `menu_fade`; `the_gate_menu_flashes`)
- [[69]] end_event not reached after the scene — answered by [[72]], [[74]]
- [[69]] WORLD_MAN::SetEventData — answered by [[87]]
- [[69]] teach_input (event 3's camera lesson) — answered by [[75]]
- [[69]] the party's AI, pc_walk remote commands 1 and 2, the message bus — answered by [[78]], [[178]] (the towns' remote walks), [[223]]
- [[69]] when the field set-up enables the event task — answered by [[329]] (ccEnableThEvent 0, 2, 4 at 0x00168c98, 0x00168f94, 0x001694d4; the game+4 exit now followed) / docs/engine/event-vm.md
- [[70]] the pages still to come: Trade (48/49), 21, 23, 27 and 56, 50 — answered by [[73]]
- [[70]] SetMerchantCamera's changeCamera(3)/(1) not carried out — answered by [[76]] (the merchant's camera; tools/test_merchcam_rs.py)
- [[70]] PcMenu draws from its own copy of rand(), not the world's shared generator — answered by [[383]] (the menus' MenuRand is SaveState::rand, the area's or town's live generator; issue #52)
- [[71]] the runtime's new-game save (skill list zeros, stats 0) — answered by [[76]] (ccSaveData::Init, NewGame, InitSpcParam; tools/test_save_init_rs.py)
- [[71]] what bootParam means beyond Add's greeting — answered by docs/engine/battle.md "The party's bootParam from area to area" ([[74]], [[80]])
- [[71]] what SetActuater(1, 160, 200) means — answered by [[232]] (small motor, power 160, 200 ms, the queue)
- [[71]] where drainDemo, voice and strWinMode are read — answered by [[105]] (drainDemo), [[89]] (voice), [[85]] (strWinMode, the subtitles)
- [[71]] still unported 62, 66/67, 71-73 — answered by [[96]], [[102]], [[148]]
- [[71]] two copies of rand(): the menus keep their own; FieldUi::set_rand is not wired — answered by [[383]] (one generator; a_breakables_drop_draws_on_the_areas_rand)
- [[72]] the party left behind in the field — answered by [[74]]
- [[72]] event 3 does not start (VM asleep in scene) — answered by [[74]]
- [[72]] the field's entry control is empty — answered by [[78]], [[80]], [[164]]
- [[72]] fog taken per model — answered by [[145]]
- [[72]] story maps of their own (EVENTAREA files) — answered for Infection's by [[84]], [[112]], [[177]]
- [[72]] only Mac Anu, Other Servers refused — answered by [[97]], [[218]], [[260]]
- [[73]] a member's book use (SpcUseItem) and PresentOther dropped — answered by [[196]]
- [[73]] feeding a Grunty (Feed, GruntyGrowth) and World::grunty — answered by [[162]]
- [[73]] CalcReal and ChangeEquip dropped — answered by [[76]] (per-frame CalcReal), [[187]] (equipment changed in a field stays)
- [[73]] set_spc_base_msg not called — answered by crates/piney-fieldui/src/newgame.rs (called by InitSpcParam's port)
- [[73]] nothing gets Kite to menus 21, 23, 27, 50, 56 in Mac Anu — answered by [[108]], [[110]] (members in town), [[90]]
- [[73]] set_rand is not wired — answered by [[383]] (FieldUi draws from the save's rand; the hosts lend it with_live_state)
- [[74]] menus 80 and 83, player_skill, hold, the magic portals (entry_mc), members' field AI — answered by [[78]], [[80]] / docs/engine/battle.md "player_skill", "hold"
- [[74]] pc_command, the walks and the puts — answered by [[178]] (remote walks), [[134]]
- [[74]] the event positions in a field or dungeon — answered by [[87]], [[134]] (evPos outside the towns); plain-field marker_pos is [[275]]'s
- [[74]] Kite stays held after event 3 (bootParam 4) — answered by docs/engine/battle.md "The party's bootParam from area to area"
- [[74]] the party not carried back into Mac Anu after Gate Out — answered by [[108]]
- [[74]] registered characters outside the party are not built — answered by [[180]]
- [[74]] when the event task wakes after `scene` — answered by [[330]] (it never sleeps: flags |= 3; the pass runs on in the frame, phase -1)
- [[75]] the town's event host gap for teach_camera3 — answered by crates/piney-world/src/lib.rs World::teach_camera (the town's)
- [[76]] per-frame CalcReal only in Mac Anu — answered by BUGS.md (kites_buff_times_out_in_town; the field's frame ran it already), crates/piney-game/src/world.rs calc_real_party
- [[76]] what partyMemberCall 0x3fffe, tactics 7, drainDemo and growth are for — answered by [[246]] / docs/formats/save.md
- [[77]] portals, gimmicks and fountains not fed into the map; Fairy's Orb's yellow areas — answered by [[164]], BUGS.md (a_fairys_orb_shows_the_portals)
- [[77]] what sets mapHideFlag — answered by docs/engine/map.md (constructor, RoomSelect, the stairs in GotoNextRoom; BUGS.md minimap fix)
- [[77]] Dun Loireag's DrawMap — answered by [[97]]
- [[77]] the map button held by `menuClrWait`, `ccCheckGtHackAnm` and the game over, and the field's and dungeon's map away while `ccGame.inBattle` — answered by [[341]] (`Targeting::map_test`, `area_frame`'s `in_battle`; menu_clear_holds_the_map_button_two_frames, the_map_is_away_in_a_fight)
- [[78]] piney-world's field and dungeon ran stand-ins with no entry control — answered by [[80]]
- [[78]] F_Note records skipped by the animation reader — answered by [[138]], [[139]], [[278]]
- [[78]] ccThGameCtrl's SetInBattle — answered by [[80]] (tools/test_gamectrl_rs.py inbattle)
- [[78]] the enemies' skills — answered by [[81]], [[83]]
- [[78]] the weapon and dust effects — answered by [[118]], [[119]], [[170]], [[197]]
- [[78]] the camera shake — answered by [[165]]
- [[78]] gimmick mains and CloseDoor — answered by [[87]], [[98]], [[116]], [[140]]
- [[78]] the Data Drain movie — answered by [[105]]
- [[79]] piney-effect not called by the runtime — answered by [[80]], [[81]]
- [[79]] effSkillStart and effSkillStartEffect — answered by [[83]], [[195]]
- [[79]] the exec rings and the physical skills' shock wave — answered by [[83]], [[195]]
- [[79]] heals, cures, Sanity, Resurrect, stat changes, the resistant shield — answered by [[83]], [[194]]
- [[79]] opening boxes, removing traps, the breakables' fragments — answered by [[98]], [[116]], [[140]]
- [[79]] the in-engine streams' effects — answered by [[131]]
- [[80]] the dungeon side: field 14's dungeon and event 4 — answered by [[87]], [[98]]
- [[80]] the fight's sounds (ccSeOn3D, enemy and party sound parameters) — answered by [[81]]
- [[80]] the AI's chat lines — answered by [[120]], [[123]]
- [[80]] spells through piney-effect — answered by [[81]]
- [[80]] Data Drain's presentation — answered by [[102]], [[105]]
- [[80]] game over past its signal — answered by [[158]]
- [[80]] boss() for the events — answered by [[113]]
- [[81]] the dungeon entry and event 4 — answered by [[87]]
- [[81]] the party AI's chat lines, Data Drain's presentation, game over, boss() — answered by [[123]], [[102]], [[105]], [[158]], [[113]]
- [[82]] WORLD_MAN::SetEventData — answered by [[87]]
- [[82]] Mac Anu's gate-address window empty — answered by [[90]] (FieldUi::announce_lines)
- [[82]] streams 4-6's subtitles — answered by [[85]]
- [[82]] event 10's sound 8 — answered by [[176]] (the sound instruction's eleven cases)
- [[83]] the new starters not wired into piney-game's fx.rs — answered by [[194]], [[195]]
- [[83]] the breakables' crush and fragment effects, the bosses' own deaths — answered by [[116]], [[140]]; Skeith's death effects (GAPS.md "Stale lines")
- [[84]] the story map's models drawn without SetFog's fog — answered by [[145]]
- [[84]] event 11's instructions in the map — answered by [[90]], [[91]]
- [[85]] streams in The World (town and field hosts) — answered by [[90]], [[91]]
- [[85]] stream 5's eight hit marks (effHitMarkStr) — answered by [[122]], [[131]]
- [[85]] the view's clip at divZ — answered by [[153]]
- [[85]] the arc's other nine effect tasks — answered by [[121]], [[122]], [[124]], [[126]]
- [[86]] the desktop's play_pass_done and load_overlay — answered by [[91]]
- [[86]] branches the talk-through did not take may call other defaults — answered by [[176]] (every host call the scripts reach)
- [[87]] event 4 host methods add_spc_item, item_get_menu(_end), remove_trap_done, room, gimmick, boss — answered by [[98]], [[111]], [[113]] / crates/piney-game/src/area_host.rs
- [[87]] the dungeon's own objects (boxes, Fortune Wire, idols) — answered by [[98]]
- [[87]] SetDoor's ban room — answered by [[114]] (the ban block)
- [[87]] the door palettes for clutType 3 and 4 — answered by [[147]]
- [[89]] changing Voice in The World's Options: whether the driver follows at once — answered by [[322]] (skill words now preceded by the save's voice options)
- [[90]] party members stand still in town — answered by [[110]] (the members under ActInTown in town)
- [[90]] event 11's block 17 at the gate and the rest of event 11 in area 15 — answered by [[91]], [[129]]
- [[90]] inviteSpc's warp-in of an unregistered member at the Chaos Gate — answered for the towns by docs/engine/field-walk.md (party_add); the table-equipment copy is not ported ([[180]]: no script makes one)
- [[90]] the gate-address window in town — answered by [[90]] itself (FieldUi::announce_lines)
- [[90]] ccEvent::CheckOperate(9)'s whole talk rule not run in eemu — answered by [[333]] (600 random cases of the game's code; check_operate_matches_the_game)
- [[91]] the church's streams lack subtitles and music — answered by crates/piney-game/src/area_host.rs, field_host.rs (StreamPlayer::event)
- [[91]] party_remove, game_over and clear_gate_hack still defaults in the fields — answered by crates/piney-game/src/area_host.rs (all three implemented)
- [[92]] piros_colour — answered by [[99]], [[107]]
- [[92]] trans not run in a playthrough (events 14, 27, 29) — answered by crates/piney-game/src/area_host.rs, field_host.rs trans; [[93]], [[143]]
- [[92]] no story starts past event 14 — answered by [[93]]
- [[93]] Dun Loireag not ported — answered by [[97]]
- [[93]] piros_colour and the town's characters' affectColor — answered by [[99]]
- [[93]] the later events' fields and dungeons, fights and bosses not played — answered by [[109]]-[[117]], [[129]], [[159]] (the ending played to a saved card)
- [[94]] the generators' state at the ending (random characters) — answered by [[386]] (the roll's `srand` takes the session's frames since power-on, its `ccRand` the last scene's through SaveState::cc; the_staff_roll_draws_from_the_frames_and_the_last_scenes_ccrand)
- [[95]] the fights' calls (Event::SkillWords) not wired — answered by [[101]]
- [[95]] Kite calling the party's strategy (Show::Shout) — answered by [[120]]
- [[95]] vBank+0x08 left as evVoicePlay last wrote it — answered by [[335]] (SEWORDS' wordPlay never reads it) / docs/formats/voice.md
- [[96]] the hacked arrival (GateHackingOut) — answered by [[106]]
- [[96]] the gate hack's noise (ccNoiz) — answered by [[104]]
- [[96]] gate_hack_anim waits on ghoFlag — answered by [[106]] (ghoFlag set by the constructor, cleared by GateHackingOut)
- [[96]] the gate hack's cancel: `ccSndGateHackCtrl` restarts sequences 0 and 2 at volume 0 and fades them in — answered by [[316]] (`Driver::gate_hack_ctrl`; `the_gate_hack_silences_the_music_and_a_cancel_brings_it_back`)
- [[97]] ccDog and the chibi Grunties — answered by [[142]], [[162]]
- [[97]] ROOTTOWN03-05 — answered by [[218]], [[260]]; their four-sprite DrawMap is [[77]]'s
- [[98]] the doors' palettes for clutType 3 and 4 and their sound ids — answered by [[147]], [[138]]
- [[98]] effOpenTrapBox, effStatueOfGod, effVirusCrystal, effCrush*, the idol's dust ring — answered by [[116]], [[118]]; effVirusCrystal from ccRtownPC::eventMode remains ([[116]])
- [[98]] EntryBreakObject — answered by [[140]]
- [[98]] the objects' menus 34-37 and 39, DataDrainMenu (66) — answered by [[148]], [[102]]
- [[98]] the map item step (WaitMap) — answered by BUGS.md (a_fairys_orb_shows_the_portals)
- [[98]] the Grunty ride item step — answered by [[166]]
- [[98]] DisableThEvent — answered by crates/piney-game/src/world.rs, area.rs (ccDisableThEvent)
- [[98]] the party AI's UseItemRequest does nothing in the field — answered by [[230]]
- [[98]] the epitaphs (`ccEpitaphMsg`) and a Ryu Book's use (`ccThBook`) as item steps — answered by [[292]] (the Key Items menu's use, the epitaph's pages in `menus::useitem`) and [[317]], [[318]] (books 1-3)
- [[98]] [[317]] Ryu Books 4-8's pages and rewards — answered by [[347]] (`Disp04`-`08`, `PadControl04`, `05`, `08`, `CheckItemGet04`-`08`; test_book_4 to test_book_8 against the game)
- [[99]] piros_colour in the field and dungeon — answered by [[107]]
- [[101]] Kite's strategy balloon and the party's chat lines — answered by [[120]], [[123]]
- [[101]] a request the menus make reaches the sound task a frame after the game's — answered by [[336]] (the menu task now runs after ccThGameCtrl and before the world's other tasks; a_skill_in_a_fight_names_itself)
- [[102]] the Data Drain movie — answered by [[105]]
- [[102]] the side effect's visuals (effSkillStartEffect, fly fonts, effAfterDrain) — answered by [[141]]
- [[102]] the noise (MenuNoise) — answered by [[104]]
- [[102]] the enemy's drained form after affect 13 — answered by [[229]]
- [[104]] ccGameOverNoise — answered by [[158]]
- [[104]] which town is game.town 4 — answered by BUGS.md (console town 4 is Lia Fail), [[260]]
- [[105]] Func_str8000, the drain streams' effect task — answered by [[125]]
- [[106]] the party's arrival chat lines (arrivalChatCnt) — answered by [[123]] (ChatMessageEnteredTown / EnteredField)
- [[107]] the condition colours ccChar::Draw applies in the fights — answered by [[195]] / crates/piney-world/src/foe.rs char_blend (used in combat/mod.rs; foe.rs unit tests)
- [[108]] no following in town — answered by [[110]]
- [[109]] which EVENTAREAB0 area event 30 loads; SwitchLayer and DMY_center01 — answered by [[112]], [[113]]
- [[109]] the boss entry (type 7) and present/absent 7 in the field's runtime — answered by [[113]] / docs/engine/boss.md
- [[109]] CMP_trall, the afterimages, the cross's trail, ccBossCam, the stage fader, the reversed layer, the effects' pictures — answered by [[113]], [[115]], [[117]] / docs/engine/boss.md (tools/test_bosscam_rs.py)
- [[109]] menu 74 (StreamMenu) — answered by [[113]]
- [[110]] the members' chat lines and window; ChatMessageEnteredTown and EnteredField — answered by [[123]]
- [[111]] RoomSelect's 71 and 77 branches (OBJ_user_point) — answered by crates/piney-world/src/dungeon_area.rs room_select (user_point)
- [[112]] FIREFLY2 (the arena's fireflies) — answered by [[117]] / docs/engine/evarea.md (checked against the game's FIREFLY2s, tools/test_evarea_rs.py)
- [[112]] sound bank 5 read from the session's WORLD_MAN, which for the arena is area 27's — answered by [[370]] (`GetEventAreaInfo(game.field)`'s model, main 0x00169160; skeith_plays_its_own_music)
- [[113]] the cross's trail, the six effects, the reversed layer and the quake — answered by [[115]], [[117]]
- [[113]] ccBossCam and the dead camera — answered by [[115]] (tools/test_bosscam_rs.py)
- [[115]] DrawCross's trail, the six effects and the reversed layer — answered by [[117]]
- [[116]] the idol's dust ring — answered by [[118]] (an idol opening)
- [[116]] no shot of a barrel smashed or a trap going off — answered by [[148]] (a breakable broken in play)
- [[117]] Skeith's cinema — answered by [[128]] (tools/test_cinema_rs.py)
- [[117]] the fields' type-1 fireflies (SetBasePosition2) — answered by [[130]] / crates/piney-world/src/firefly.rs (tools/test_field_ambient_rs.py); docs/engine/evarea.md bullet is stale
- [[119]] effSmoke's other callers — answered by [[190]]
- [[121]] Func_str0090's and Func_str0110's fog — answered by [[127]]
- [[121]] the effect tasks of streams 7-9, 11-16 and the str7xxx-str9xxx scenes — answered by [[122]], [[124]], [[125]], [[126]], [[133]]
- [[122]] what the marks and the transfer look like in a stream (ccThEffectStr) — answered by [[131]] (tools/test_effect_str_rs.py)
- [[122]] stream 7's fog — answered by [[127]]
- [[122]] streams 11-16's and the str7xxx-str9xxx scenes' tasks — answered by [[124]], [[125]], [[126]], [[133]]
- [[124]] stream 15's Func_str0580 and Func_str0581 — answered by [[133]]
- [[124]] the str7100, str8000, str8800 and str9xxx tasks — answered by [[125]], [[126]]
- [[124]] the marks and transfers (ccThEffectStr) — answered by [[131]]
- [[124]] the fog of streams 13 and 16 — answered by [[127]]
- [[125]] stream 20's fog — answered by [[127]]
- [[125]] Func_str7100, Func_str8800, Func_str0580/0581 — answered by [[126]], [[133]]
- [[126]] stream 15's Func_str0580 and Func_str0581 — answered by [[133]]
- [[127]] the port's fog per model, not per vertex — answered by [[145]]
- [[128]] the cinema's other names (other bosses' rows, x01-x04) — answered by crates/piney-world/src/cinema.rs (all 72 rows of _g_cinemaSkillName; tools/test_cinema_rs.py)
- [[129]] the black screen in the church — answered by [[174]], BUGS.md (story 11's set-up windows)
- [[129]] a merchant's sysopeAct transfer — answered by crates/piney-world/src/merchant.rs sysope_act, [[132]]
- [[129]] the stream effects not checked (tools/test_effect_str_rs.py to be written) — answered by [[131]]
- [[130]] EntryGimmick in a field — answered by [[164]]
- [[130]] WORLD_MAN's two ccBufferSampling depth shades — answered by [[152]]
- [[130]] TOBJ's shadow and sounds — answered by [[160]] (shadow), [[150]] (the hum)
- [[131]] stream 15's Func_str0580 and Func_str0581 — answered by [[133]]
- [[133]] the shades on the GPU (REGION_REPEAT as a plain repeat) — answered by [[146]]
- [[133]] VU1's clip at divZ for stream 15 — answered by [[153]]
- [[134]] whether ccGetDist in CheckOpen takes the wrapped frame for both points — answered by [[328]] (the player's posP is FZeroPosition with his ground height; a carrier's move while riding)
- [[135]] the fountain's FountainMenu3 pair — answered by [[163]]
- [[135]] ccClearSpcCondition, which the bosses' deaths call — answered by [[192]]
- [[136]] piney-gs clipping at a fixed 1000, not divZ — answered by [[153]]
- [[137]] the direct light controller (0x0605) — answered by [[151]]
- [[138]] ChangeClut, the door palette swap for clutType 3 and 4 — answered by [[147]]
- [[142]] the dogs' menu draws talkNum from rand(), not the town's ccRand — answered by [[386]] (the town lends its ccRand as SaveState::cc; a_dogs_talk_line_draws_the_towns_ccrand)
- [[148]] whether "Chronicling" can be had in Infection; timeSym and gameCnt — answered by [[154]]
- [[148]] ccGimSymbol not ported (symbols as stand-ins) — answered by [[149]]
- [[149]] row 18's objMain (the lakes' symbol) — answered by [[155]]
- [[150]] TOBJ's shadow (ccShadowModel) — answered by [[160]]
- [[157]] the lakes' fireflies — answered by [[161]]
- [[162]] the ride (ccPgAdultCheck, ccPuccigusoStart, ccPucciguso, pgRideFlag) — answered by [[166]]
- [[162]] a field's EntryGimmick (SetFood, portals, special objects) — answered by [[164]]
- [[163]] row 21 (ENTRANCE, effDungeonEntrance) made only by the harness — answered by [[164]] (SetSpecialObj makes the entrance swirls)
- [[168]] the fire breath (ccEnemyBreath) not drawn — answered by [[172]]
- [[168]] the motion of L, W, D, E, T, A, 4 and S — answered by [[169]], [[171]]
- [[169]] ccEnemyL (dragons and snakoids) stands still — answered by [[171]]
- [[169]] the turtles' foot dust and the Cerberus's breath not drawn — answered by [[172]] (GAPS.md: drawn)
- [[170]] the enemies' weapon flashes (and the boxes') put a light in the scene's light group in the game; the port does not — answered by [[377]] (`Combat::rad_lights`, `Weapons::lights` into `cast_lights`; docs/engine/battle.md "The weapon trails and flashes")
- [[171]] the breaths not drawn — answered by [[172]]
- [[176]] whether the SE port's volume is ever below full when sound 8 0 256 runs — answered by [[344]] (port 0 takes seVol whatever p1 says; six uses across the volumes, all after streams but M110's)
- [[180]] inviteSpc's build at the Chaos Gate — answered for the towns by docs/engine/field-walk.md (party_add), [[250]]
- [[181]] how the later volumes pick their event voice tables — answered by [[226]] (each disc's own ccEvVoiceRequest decoded by groups)
- [[182]] the readers still use Infection's addresses into generated bytes — answered by [[208]]-[[217]] (typed per-volume tables; the World off the executable image)
- [[184]] what ccSnd +0x110 is — answered by [[327]] (`free[2]` of `int free[4]`, only ever cleared)
- [[188]] the sound's voices across volumes (EvVoice groups per volume) — answered by [[226]], [[227]]
- [[191]] `come_back` builds the lake room's doors open where the game asks `ccCheckActiveObject` — answered by [[343]] (the game's lists are empty then: a new scene deletes every entry, so `SetDoor` builds them open too, and `MoveDoor` shuts them for what the set-up makes)
- [[191]] what reads WORLD_MAN+0x58[n] — answered by [[343]] (`WORLD_MAN::EntryGimmick`'s `entryFlag[1 + n]`; the port cleared it nowhere and kept a lake's entries across its dungeons; a_lake_keeps_both_its_dungeons)
- [[191]] the walk between a lake's two dungeons not compared with the game's run — answered by [[379]] (tools/test_dungeon_rt.py's test_lake_stairs runs the game's `Enter` on both stairs, `GoField` in both dungeons and `GO(2)`'s way back up against the port; it found the lake's `startpos[1]` left at 0, issue #51)
- [[195]] the party's weapon trails (ccSpcChar::ArmsEffect) and StartArmsEffect's particles — answered by [[197]]
- [[196]] the pad's vibration in town — answered by [[232]]
- [[199]] Mutation's save (extension, accessors, Init, NewGame, InitSpcParam) — answered by [[200]], [[226]] (tools/test_save_init_rs.py on MUT)
- [[199]] saveSysMsg and MAIL_REPLIES carried entry by entry, not re-derived per volume — answered by crates/piney-gen/src/sinit.rs (each volume's static initialisers run in piney-eemu)
- [[199]] a table a later volume grew read with Infection's length — answered by [[203]], [[221]] (grown tables)
- [[200]] the later titles' menus (five items) — answered by [[201]]
- [[200]] the desktop's tables read with Infection's row counts — answered by [[209]] (typed per-volume tables)
- [[200]] what reads the blocks at +0x8432 and +0x8462 and the extension's last 0x100 bytes — answered by [[246]] (nothing)
- [[201]] Outbreak's and Quarantine's titles not run — answered by [[204]] (all three later discs reach their desktops)
- [[202]] why Mutation's desktop setup stalls — answered by [[204]] (the carry's pointers: name entry's strings, the mail)
- [[203]] each later volume's ccEvVoiceRequest groups — answered by [[226]] (each disc's own decoded)
- [[203]] Mutation's ccVoiceRequest cases — answered by [[227]] (Mutation's field voices, food and skill words)
- [[203]] OUT's and QUA's setbl rows and their ccSeSetParam* — answered by [[278]] (notes_are_the_volumes_own)
- [[203]] whether ccSndStreamCtrl's stream numbers changed — answered by BUGS.md (stream_ctrl takes the volume; the_later_volumes_music_around_the_streams)
- [[204]] OUT's and QUA's ccParticle::Setup switch — answered by [[205]] (decoded by register)
- [[207]] the rebuild restoring the systems one at a time — answered by [[216]] (Infection whole again on generated tables)
- [[208]] the rest of the rebuild (desktop tables, top page, field UI, world, battle, effects, streams) — answered by [[209]], [[210]], [[215]], [[216]], [[217]]
- [[208]] msg in a character's row kept as an address until its reader is rebuilt — answered by [[214]] (the talk pages on generated talk records)
- [[209]] Mutation's ccThEvent second flag at main 0x0038bd44 — answered by [[225]], [[395]] (the flag race; the event task waits while it runs in the town: piney-game world.rs, `World::racing`)
- [[225]] [[394]] Mutation's Flag Race (menu 88, its page, the race's task and object, its flags, timer and result) — answered by [[395]] (docs/engine/flag-race.md; tools/test_race_rs.py, test_fieldui_talk_rs.py, test_grunty_rs.py; `mutations_flag_race_won`)
- [[209]] the card uses Infection's directory and slot size on every disc — answered by [[226]] (directory and size from the executable)
- [[209]] streams fail on the later discs; Mutation's opening movies skipped — answered by [[215]], BUGS.md (later_volumes_play_their_voice_track)
- [[209]] the record menu reads saveSysMsg through piney_data::sinit; announcements read getItemMenuStr from the image — answered by [[210]], [[216]]
- [[210]] six trade and shop harness cases fail (errorData) — answered by [[214]]
- [[210]] the world, areas, streams and announcements still read the executable — answered by [[215]], [[217]]
- [[211]] whether the carry's Rust port gives the same cache byte for byte (difflib) — answered by [[212]] (piney-gen carry --check)
- [[211]] the six trade and shop failures — answered by [[214]]
- [[212]] whether the symbol transfer (xfer.py transfer, .syms) ports exactly — answered by [[213]]
- [[213]] the five older Python generators remain — answered by [[256]]-[[258]] (every table group in the build; the Python generators are gone from tools/)
- [[214]] session tests stop at the world's and areas' loads — answered by [[215]]-[[217]]
- [[215]] the town and field readers read GCMN.PRG with Infection's addresses; mapmsg fails; announcements empty — answered by [[217]] (the World off the executable image)
- [[216]] Mutation's boot past the desktop — answered by [[217]]-[[228]], [[276]]
- [[216]] the five older Python generators — answered by [[256]]-[[258]]
- [[217]] Carmina Gade (town 2) — answered by [[218]]
- [[217]] M201's fields and dungeons, Mutation's gate hacks and Data Bugs at a start — answered by [[248]], [[250]], [[263]]-[[276]]
- [[218]] Carmina Gade's map and Mutation's changed Mac Anu and Dun Loireag maps — answered by [[225]] (own layers, the fourth mask; crates/piney-world/src/map/town.rs)
- [[218]] Carmina Gade not compared with the game in eemu — answered by [[219]] (tools/test_town03_rs.py)
- [[218]] the desktop and board have no autopilot — answered by [[225]] (the survey's player drives them)
- [[219]] Carmina Gade's DrawMap and Mutation's changed town maps — answered by [[225]]
- [[221]] Mutation's ccAI::UseItem changes (CheckAction(7), held members, remarks) — answered by [[222]]
- [[221]] Mutation's four new chat tables (tolerances, only one buff or debuff) — answered by [[222]], [[223]]
- [[221]] the items, fellow, kite, party AI, enemy AI, spawn, event, chat and drain harnesses fail on Mutation — answered by [[222]]-[[224]]
- [[222]] the callers of the other remarks (ChatCommandExecute, BuffPlz, DeBuffPlz, FulfilCheck) — answered by [[223]]
- [[222]] the party AI harness's five failures on Mutation — answered by [[223]]
- [[223]] the fellow, kite and spawn harnesses not rerun on Mutation — answered by [[224]]; event and navi not named there
- [[225]] the survey's player goes no further than the town — answered by [[236]], [[248]]
- [[225]] the Data screen's and save menus' harnesses fail on Mutation — answered by [[226]]
- [[226]] Mutation's field, skill and food voices — answered by [[227]]
- [[226]] Mutation's BOSSTALK.BIN — answered by [[227]], [[239]] (only Outbreak's disc carries it)
- [[227]] Outbreak's and Quarantine's spcVoiceData 21 files — answered by [[239]] (SPC18-20: Tsukasa, Subaru, Sora), [[278]]
- [[228]] the eight tutorial and item-window failures on Mutation — answered by [[235]] (Mutation has no lesson messages; a harness artifact)
- [[228]] interNoiz on Mutation's scenario — answered by [[235]]
- [[229]] no test drains an enemy on Mutation's disc — answered by [[266]], [[268]] (Innis drained)
- [[236]] why the warps of 107-115 come back to menu 88; the waiting starts' add_target NPCs — answered by [[248]], [[263]], [[264]] (the autopilot's talks), [[276]]
- [[239]] what STRT.BIN's scenes look like — answered by [[240]] (STRT.BIN played: the demo discs' screens)
- [[248]] why the party calls of 105, 109, 112, 113 loop at menu 68 — answered by [[250]] (members called by address)
- [[248]] why 107's and 111's gate hacks do not pass — answered by [[250]], [[270]] (the gate hack's cores)
- [[248]] what holds 103 in its town — answered by [[263]] (event 103's add_target)
- [[250]] why CHAT's member menu (71) loops in 107 and 112 — answered by [[263]] (menu 71's refusal)
- [[256]] how many of the 337 shared statics' users need changing when their groups move — answered by [[257]] (every table group moved into the build)
- [[257]] the generator still reads its inputs from work/ — answered by [[259]] (the generator reads the discs)
- [[258]] the generator reads the executables, overlays, symbols and DWARF from work/ — answered by [[259]]
- [[264]] Innis, Kyvia 01 with EVENTAREAB8, and Magus — answered by [[266]], [[272]], [[274]]
- [[266]] whether the autopilot finishes event 107 once it reaches Innis — answered by [[268]]
- [[267]] Func_str1070 — answered by [[269]]
- [[267]] str7100, str8800 and str0300 changed on Mutation — answered by [[271]] (they are Infection's code)
- [[270]] the whole run after 108 not seen — answered by [[276]] (101 to 116 under the autopilot)
- [[280]] Mutation's convergence aim, effSkillChargeObj's new argument, the drill's endFlag — answered by [[281]] (tools/test_effect_spell_rs.py on MUT, 0 mismatches)
- [[317]] the Ryu Book cover's palette from the second book on — answered by [[390]] (`ccThBook` puts `stream_cluts[n]` on `str8800e`'s `MAT_clut`, kept while the town's files are; `a_ryu_books_cover_takes_the_books_palette`) / docs/engine/stream.md "The streams' common files"
- [[347]] the Ryu Books' pages on the later volumes — answered by [[390]] (Mutation's book.cpp ported, Outbreak's and Quarantine's changes; every test_book_* on all four volumes) / docs/engine/field-ui.md "The later volumes' books"
- [[388]] the draw turns the stream camera with glam's sine, not libvu0's — answered by [[389]] (`piney_desktop::camera`: SetMatrix_PosRotXYZ(Debug), SetView and ccSetViewScreenClipMatrix in VU0's and the EE's arithmetic; world_screen bit for bit on all 1837 fixture frames and on 301 cameras of tools/test_stream_rs.py cameras) / docs/engine/desktop.md "The 3D draw"

## Dead

- [[76]] Init's fields not repaired in old saves (townMoveFlag, partyMemberCall, tactics, cameraMode, drainDemo, idol ranks) — concerns the port's own saves made before [[76]]'s fix, not the game
- [[81]] other agents working in the same piney-audio files — a process note, not a question
- [[183]] "Nothing new." — no question stated
- [[87]], [[111]] prev_room, [[109]], [[117]] hold 7 — no script of any of the four volumes uses either ([[323]])

## Closed by resolves

Entries `cairns open` leaves out because a later entry names them in
`resolves:`. Their questions, for completeness:

- [[54]] (closed by [[55]]) answered: ccMorpher::Modify against a run of the game — answered by [[55]]
- [[54]] (closed by [[55]]) answered: Func_str0001's effects (noise, feedback, reversed buffer, fades, fog) — answered by [[57]], [[127]], [[156]]
- [[54]] (closed by [[55]]) answered: town or field models with morphers — answered by crates/piney-world/src/pose.rs, town01.rs (the canal waves)
- [[103]] (closed by [[108]]) answered: inviteSpc building an unregistered member at the Chaos Gate — answered for the towns by docs/engine/field-walk.md (party_add); [[250]]
- [[103]] (closed by [[108]]) answered: the town does not take the party back from the field — answered by [[108]]
- [[103]] (closed by [[108]]) research: the fellow task's own frames (a dismissed leaver's exit and walk away) not run against the game in a field — rules compared only
- [[118]] (closed by [[119]]) answered: effSmoke's other callers (Grunties, DrawSteam, DrawEffect, ccGimSymbol, explode, ccDog, bosses) — answered by [[190]] / docs/engine/effects.md (every other Infection caller ported); the later volumes' bosses' remain
- [[118]] (closed by [[119]]) answered: the party's running dust (ccEffPawSmoke) — answered by [[119]]
- [[118]] (closed by [[119]]) research: no shot of the dust — the harness covers it
- [[120]] (closed by [[123]]) answered: the party's own lines (ccAI::ChatMessage*, ChatMessageSender) — answered by [[123]]
- [[120]] (closed by [[123]]) answered: a Grunty's line (ccPGuso::main) — answered by [[162]]
- [[120]] (closed by [[123]]) research: the balloon's text drawn through a 128-row Init(3, 24) texture where the game's chat kanji is Init(2, 16) — one line looks the same
- [[263]] (closed by [[265]]) research: why event 108 logs in to Dun Loireag — not revisited
- [[263]] (closed by [[265]]) research: how the pilot should find a Data Bug's protect break for Data Drain — autopilot
- [[263]] (closed by [[265]]) answered: EVENTAREAB8 (field 9, event 108) and EVENTAREA01 (field 13, event 115) — answered by [[272]], [[264]]
- [[263]] (closed by [[265]]) answered: the towns read Infection's statics::tables() on every volume — answered by [[181]] (the statics of all three later volumes equal Infection's row for row)

## Docs

Every bullet of the `## Unknown` sections, page by page. Pages whose
section reads "None": docs/disc/layout.md, mutation.md, outbreak.md,
quarantine.md; docs/engine/executable.md; docs/formats/ccs.md,
ccs-model.md, data-bin.md, prg.md, save.md. (docs/engine/field-ui.md has
no `## Unknown`; its "Not yet known or not ported" list is cited above.)

### docs/content/game-data.md

- docs/content/game-data.md: the meaning of the enemy type, Exdefense, AI and entry fields — answered (stale; docs/engine/battle.md ([[22]], [[67]], [[78]]); duplicates [[12]])
- docs/content/game-data.md: skill and item effect encodings — answered (stale; docs/engine/battle.md ([[22]], [[67]]))
- docs/content/game-data.md: how areas choose enemies (enemyList00..15, enemyOfs) — answered (stale; [[13]], [[275]], crates/piney-battle/src/entry.rs register_list)
- docs/content/game-data.md: the msg field's target — answered (stale; [[208]], [[214]] (its message table's address))

### docs/disc/volumes.md

- docs/disc/volumes.md: the names of the functions in the unnamed stretches — research (survive nowhere; duplicates [[239]])

### docs/engine/animation.md

- docs/engine/animation.md: the spot light controller (0x0607) not evaluated — research (no Infection room uses one; duplicates [[137]])
- docs/engine/animation.md: F_Obj's flag byte (+0xa2); whether an F_Obj pose survives into the next clip — research (duplicates [[137]])

### docs/engine/area-words.md

- docs/engine/area-words.md: what the values of fieldType, weather, ground, object, circleOfs produce — research (duplicates [[11]])
- docs/engine/area-words.md: how itemOfs selects items, and the type-64 enemy expansion — answered (stale; crates/piney-fieldui/src/menus/getitem.rs area_item ([[68]]), crates/piney-battle/src/entry.rs Register::register ([[78]], tools/test_battle_spawn_rs.py))
- docs/engine/area-words.md: EVENTAREA_INFO.model beyond the music bank, .protect beyond protect[1] and the item pairs, .dungeonNum — research (duplicates [[11]], [[19]])
- docs/engine/area-words.md: the port reads Infection's addresses only; the other volumes' tables by tools/areas.py alone — answered (stale; per-volume generated area tables ([[181]], [[220]]; tools/test_area_rs.py on PINEY_VOLUME))

### docs/engine/battle.md

- docs/engine/battle.md: the labels of type bits 0x100/0x200 and the art bits' names — research (duplicates [[22]])
- docs/engine/battle.md: that the protect break is the Data Drain window (inferred); plcol as the bracelet is settled — research (duplicates [[22]])
- docs/engine/battle.md: the spell element systems' own timing (Fall, Convergence, Upheaval, Summons, Tornado from level 3) — answered (stale for this page; [[79]], [[281]] (tools/test_effect_spell_rs.py: every system's frames and releases, 0 mismatches))
- docs/engine/battle.md: the other bosses' Affect and AI (ccBoss02-04) — answered (stale in part: ccBoss02 Innis [[266]], ccBoss03 Magus [[274]]; ccBoss04 not ported (play, volumes)) (ccBoss04 ported in [[294]])
- docs/engine/battle.md: ccBoss04's Affect and AI — play (a later volume's boss, not ported) (ported in [[294]])
- docs/engine/battle.md: Kite's third swing; ControlMove's ctrlType 1 and 2 — research (inferred unreachable; duplicates [[78]])
- docs/engine/battle.md: what the party AI does with message 0x10004 (Kite pressed against a wall) — research (not traced)
- docs/engine/battle.md: WORLD_MAN's trans mode as a carrier; weaponChangeSW -1; fpAngleOffset[-1] — research (inferred or measured only)
- docs/engine/battle.md: a dungeon way of 55+ cells, destinations never set, gRotSp 0 — research (game quirks; duplicates [[78]])
- docs/engine/battle.md: ccHitCheckLM in MakeMiniMap; saveData +0x220d; ground attributes 0x20000 and 0x80000 — research (inferred; duplicates [[123]])
- docs/engine/battle.md: a story dungeon's circles' w from the caller's stack; a middle boss's anmTbl at 0xb4; a circle's random row with nothing registered — research (game UB)
- docs/engine/battle.md: ccEnemy1::action with eneType outside 0-5 — research (no constructor makes one)
- docs/engine/battle.md: a new ccAI's levelOld from the heap — research (duplicates [[123]])
- docs/engine/battle.md: whether the two stray-# lines freeze a real console — research (duplicates [[123]])
- docs/engine/battle.md: whether ccChar::Draw leaves node coordinates stale on a culled frame — research (duplicates [[170]])
- docs/engine/battle.md: (events section) pc_walk_marker with no such event position — research (the checks do not make it)
- docs/engine/battle.md: (events section) what eventMng.bossTscb +0x14 counts — research (hold 7 unused in Infection)
- docs/engine/battle.md: (events section) remove -1 (deleteAllObject) not ported — research (crates/piney-world/src/field_world.rs remove_entry; which scripts use it outside the towns not surveyed)
- docs/engine/battle.md: (events section) pc_act and pc_mode on this crate's state — answered (piney-world ports them (Combat::menu_ban_conditions, menu_clear_conditions))

### docs/engine/boss-kyvia.md

- docs/engine/boss-kyvia.md: the gomoras' pushes, the pad's sway, the arm's picture; the particles named, not drawn — play (duplicates [[272]])
- docs/engine/boss-kyvia.md: the EX mode and levels 2-5 (Kyvia's later fights) — play (duplicates [[272]])
- docs/engine/boss-kyvia.md: which of x01's gomora clumps each attribute draws — research (duplicates [[272]])

### docs/engine/boss-magus.md

- docs/engine/boss-magus.md: the leaves', laser's, needles' and smoke's pictures named, not drawn; dropped leaves not hidden — play (duplicates [[274]])
- docs/engine/boss-magus.md: the shocks' beams taken to meet the ground (ccHitCheckLM not ported) — play (duplicates [[274]])
- docs/engine/boss-magus.md: EntryFlash2 and EntryFlash3 drawn as one flash; the grow's looping SE plays once — play (duplicates [[274]])

### docs/engine/desktop.md

- docs/engine/desktop.md: the save menus' card-busy operations never shown (the port's card is instant) — research (duplicates [[159]])
- docs/engine/desktop.md: ccScFade's ContinueFade and CheckFade to the exact frame — research (not measured)
- docs/engine/desktop.md: how long card calls take on a console; the busy indicator; ccThSaveSys's order — research (console timing)
- docs/engine/desktop.md: what ccScFade status 9 (EntryFlash3) draws; how ccSndChangeData treats the old piece — research (the new piece's sequence for 47, 27, 7 is ported (crates/piney-data/src/sound/mod.rs wave_sequence))
- docs/engine/desktop.md: how long a wallpaper takes to load (ccThWallLoad) — research (disc timing)
- docs/engine/desktop.md: how long ccLoadFLAddOne takes to load a news page — research (disc timing)
- docs/engine/desktop.md: DispInfo's branch before play not run against the game — research (no Infection script announces before play)
- docs/engine/desktop.md: whether xwindow and xwin_f00 are resident during name entry; layer 131 in the two frames after Main returns 1 — research (not traced)
- docs/engine/desktop.md: the cursor blink's phase from ccSys.count — answered by [[386]] (the session hands the desktop its frames since power-on, `Desktop::set_count`)

### docs/engine/dungeon.md

- docs/engine/dungeon.md: the palette swaps over the whole file; the door functions' own ChangeClut; no picture compared — research (duplicates [[147]])
- docs/engine/dungeon.md: type 32's leaves (ANM_se4_2lea, LEAF), a later volume's room — play (duplicates [[114]])
- docs/engine/dungeon.md: how ccLight combines an event room's LGT_ record with SetRoom's place; the ban block's nodes — research (duplicates [[114]])
- docs/engine/dungeon.md: EntryObject's palettes for the lakes' statues and flowers with GetBG not 0 — play (duplicates [[157]], [[161]])
- docs/engine/dungeon.md: a story dungeon's fieldrand after Generate — research (duplicates [[157]])
- docs/engine/dungeon.md: what ccObj::Duplicate(0x2008)/(0x2000) copy of the water's object — research (duplicates [[157]])
- docs/engine/dungeon.md: whether a room's dummies can make more than 32 room lights — research (none checked comes near)
- docs/engine/dungeon.md: the warps (warpPoint) — research; a lake's way between its dungeons not run against the game — answered by [[379]] (test_lake_stairs)
- docs/engine/dungeon.md: a keyed rotation's last bits (double precision interpolation) — research (rounding)
- docs/engine/dungeon.md: why DecodeSetup reads a bad pointer decoding sd9 after sd4 — research (duplicates [[161]])
- docs/engine/dungeon.md: what the save bit at +0x5ec8 bit 62 means — answered (stale; [[220]] (eventFlag[314] done); ishack stays research ([[25]]))
- docs/engine/dungeon.md: story-dungeon room models (MakeRoom(ROOMDATA *)) — answered (stale; [[114]], crates/piney-data/src/dungeon/special.rs)
- docs/engine/dungeon.md: why types 8/9 pick a down-stairs room on their one floor — research (duplicates [[19]])
- docs/engine/dungeon.md: "SetDungeonTypeFromField on 12,288 inputs" — dead (a check's line that landed under Unknown, not a question)

### docs/engine/effects.md

- docs/engine/effects.md: unknown_0c of the Eff chunk (112 in every chunk) — research (read past)
- docs/engine/effects.md: what ccEffect::level means to the ids that use it — research (beyond each case's code)
- docs/engine/effects.md: ccEffObj nodes in a clump (DrawNoAnm) — research (none found; not ported)
- docs/engine/effects.md: ccBltData flag bit 1; a sprite's FOG dropped (the effects' sprites never fog) — research (not needed)
- docs/engine/effects.md: Draw's unsorted path, a TEST reference over 255, the effect objects' shadow switch — research (never reached)
- docs/engine/effects.md: effVirusCrystal from ccRtownPC::eventMode (Mutation's event 114) and the later volumes' bosses' effSmoke — play (not ported)
- docs/engine/effects.md: effResistantShield's null read; effAbilityUp/Down with a failed new — research (game UB)
- docs/engine/effects.md: what the effects' generator rows look like on screen beyond the shots — research (no capture)
- docs/engine/effects.md: CheckCharAttribute, ccCheckObjectSize, ccConditionIconNum, condition.dead, the command lists as host inputs — dead (a crate-boundary note; the runtime supplies them)
- docs/engine/effects.md: ccCheckObjectSize sizes other than 1, 3, 4 and drain types other than 0 or 1 — research (game UB)
- docs/engine/effects.md: hitFlip's first value (ccChar's constructor leaves it) — research (port starts clear)
- docs/engine/effects.md: whether any menu clears flyFont's ctrl bit 0x10 — research (not found)
- docs/engine/effects.md: what each particle generator row looks like on screen — research (no capture)
- docs/engine/effects.md: what reads the save's portal counts; entryCircleObject, what comes out of a portal — research (duplicates [[193]])
- docs/engine/effects.md: ccEntryChangeCLUT swaps every texture on the palette, where the game swaps per material — play (known divergence; not checked on screen)
- docs/engine/effects.md: SetFogSw and SetShadowSw on the spell pieces, the drill's ccDrawEnv settings not modelled — play (duplicates [[79]], [[231]])
- docs/engine/effects.md: the explosions' palette swaps not in the draw record — play (duplicates [[79]])
- docs/engine/effects.md: ccEnergyGrowElement's m_genRnd — research (0 at its one caller)
- docs/engine/effects.md: ccThunderBoltElement's MissileSmoke/BurstSmoke paths — research (no data reaches them)
- docs/engine/effects.md: the wind charge pieces' rand() written into lwMatrix and dropped — research (no effect on the picture)

### docs/engine/evarea.md

- docs/engine/evarea.md: the fog by depth not compared with the game's pictures; PRIM.FGE from SetFogSw — research (duplicates [[145]])
- docs/engine/evarea.md: EA_moveTex02's scroll starts at 0 on a new map after the town — research (phase only; duplicates [[84]])
- docs/engine/evarea.md: EVENTAREAB8 not compared with the game's Draw frame by frame — research (duplicates [[272]])
- docs/engine/evarea.md: fields 10-12 not played through their events (Kyvia's later fights) — play (volumes)
- docs/engine/evarea.md: the other EVENTAREA classes (04-06; areas 66, 67, 91) and EVENTAREAB0's NextStage — play (duplicates [[72]], [[112]])
- docs/engine/evarea.md: the fields' type-1 fireflies — answered (stale; [[130]], crates/piney-world/src/firefly.rs (tools/test_field_ambient_rs.py))
- docs/engine/evarea.md: event 11's instructions in the map — answered ([[90]], [[91]])

### docs/engine/events.md

- docs/engine/events.md: what fills OUT's and QUA's BSS message groups; what OUT's rewritten cases do — play (duplicates [[24]])
- docs/engine/events.md: what gcmn 0x004f0a30's thread does with the KFED buffers — answered ([[239]] (the staff roll; the buffers are never read))
- docs/engine/events.md: npc_act -3 and -5 (row 139's drain, the Administrator's -5) not ported; only Mutation's event 114 uses them — play (duplicates [[65]])
- docs/engine/events.md: the numbering of streams, markers, menus, operations, sound commands, act codes — answered (the bullet itself closes it ([[176]], [[178]], [[179]]))
- docs/engine/events.md: the boards, the windows, the camera units — answered (the bullet itself closes it ([[179]]))

### docs/engine/event-vm.md

- docs/engine/event-vm.md: the load between passes 2 and 4 — research (duplicates [[69]])
- docs/engine/event-vm.md: DispInfo's setup-screen branch not run against the game — research (no Infection script announces before play)
- docs/engine/event-vm.md: a desktop-item announcement's book* strings read past DESKTOP.PRG's end — research (Infection's desktop scripts never reach desktop_item)
- docs/engine/event-vm.md: how long ccThMother takes between ChangeRequest(3, 7) and the desktop's setup — research (duplicates [[82]])
- docs/engine/event-vm.md: piros_colour 9 with an operand past 5 — research (no script does it)
- docs/engine/event-vm.md: volume 2-4 story events have no message arrays on Infection's disc — research (not played there)

### docs/engine/field-game.md

- docs/engine/field-game.md: water 0's frame-buffer copy is the whole frame, not the game's 128 x 128 — play (duplicates [[97]])
- docs/engine/field-game.md: the town's per-vertex fog not compared with the game's pictures — research (duplicates [[145]])
- docs/engine/field-game.md: the areas' light values at frame 1 not checked — research (duplicates [[141]])
- docs/engine/field-game.md: SetLightMatrix's second group at ccDrawEnv +0x88 — research (duplicates [[141]])
- docs/engine/field-game.md: the walking PCs' presentation not compared; ccSys+0x358 an input; rtownnpc.cpp's globals start at zero per visit; a walker's first nextPos — research (duplicates [[65]], [[139]])
- docs/engine/field-game.md: event mode -3 (effVirusCrystal) not ported — play (Mutation's event 114; duplicates [[65]], [[116]])
- docs/engine/field-game.md: whether the Chaos Gate has a body on the character list — research (duplicates [[65]])
- docs/engine/field-game.md: the event NPCs outside the towns: their steps and effects a frame late — research (duplicates [[132]]; the second argument answered by [[393]])
- docs/engine/field-game.md: inviteSpc of an unregistered character in a field or dungeon; StartPos for members already in the party at an area's start — research (duplicates [[250]])
- docs/engine/field-game.md: ccThSpc and ccThAISystem run before a leaver's task ends — research (duplicates [[110]])
- docs/engine/field-game.md: the event camera's work-area reads, stack garbage, currentOpen, base-id lookups, FIFO order within a priority — research (game UB and inference)
- docs/engine/field-game.md: the CPU renderer drops triangles behind the camera — research (duplicates [[61]])
- docs/engine/field-game.md: rand()'s sequence is shared, so the fidget's start cannot match — research (duplicates [[59]])
- docs/engine/field-game.md: the ground attribute's type bits from a stale register — research (the port takes the nearest contact's)
- docs/engine/field-game.md: ccSetChaosGate leaves its entry's +0x44 unset — research (same ground either way)
- docs/engine/field-game.md: the anm lights' colour and float controllers evaluated between keys in f32; the game's steps through segments — research (not run against it)

### docs/engine/field.md

- docs/engine/field.md: how the fog table's near, far and percentage map to the GS fog — answered (stale; [[145]] (DepthFog::set_fog))
- docs/engine/field.md: the water's run-time texture and UV scroll — play (approximated; duplicates [[38]], [[72]])
- docs/engine/field.md: the objects' own draw code and the type-6 FOBJECT2s DrawMesh draws — research (duplicates [[38]])
- docs/engine/field.md: whether a tile's alpha is clamped beyond 7,200 — research (duplicates [[38]])
- docs/engine/field.md: EntryGimmick's fieldrand draws on the entry task's first slice; the port makes them on Play(0)'s frame — research (ordering)
- docs/engine/field.md: object heights and everything after the start position — answered (stale; [[38]], [[130]], [[164]])
- docs/engine/field.md: the port's effSmoke puffs start a frame after the game's — research (duplicates [[130]])
- docs/engine/field.md: BIRD's clump drawn as its model at the bird's matrix; the weather's sprites unfogged — research (as the effects')
- docs/engine/field.md: the weather shots are the port's; no game frame compared — research (no capture)

### docs/engine/field-walk.md

- docs/engine/field-walk.md: what the entry control in a field holds — answered (the bullet states it: [[164]])
- docs/engine/field-walk.md: the water's run-time textures and the clouds' blending are approximations — play (duplicates [[72]])
- docs/engine/field-walk.md: story maps of their own other than 15's, 16's and the arenas get generated fields — play (duplicates [[72]]; no Infection script reaches them)
- docs/engine/field-walk.md: a HIT_ node's local matrix taken as the identity — research (not checked)
- docs/engine/field-walk.md: the ground attribute's type nibbles from a stale register — research (cannot be matched)
- docs/engine/field-walk.md: SimGenerateCode and GO move the global RNG; the port leaves those draws aside — research (duplicates [[72]], [[111]])
- docs/engine/field-walk.md: ROOTTOWN03-05 belong to the later volumes — answered (stale; [[218]], [[260]] built them)
- docs/engine/field-walk.md: CollisionTest guards Enter only with dneFlag; whether the set-up stops the old tasks sooner — research (not traced)
- docs/engine/field-walk.md: what reads WORLD_MAN+0x58[n] — answered by [[343]] (`WORLD_MAN::EntryGimmick`, entryFlag)
- docs/engine/field-walk.md: inviteSpc's copy of the table's equipment into the registry — research (no script makes one)
- docs/engine/field-walk.md: the hacked arrival's load time — research (duplicates [[106]])
- docs/engine/field-walk.md: ghoCam's markers from the desktop's anm player — research (duplicates [[106]])
- docs/engine/field-walk.md: the event thread's wake frame — research (duplicates [[74]])

### docs/engine/font.md

- docs/engine/font.md: how the GS turns the packets into pixels (top-left sampling for kt 0 and 2) — research (duplicates [[23]])
- docs/engine/font.md: the display scale of the font layer — research (duplicates [[23]])
- docs/engine/font.md: the line spacing of mail and board bodies — answered (stale; docs/engine/desktop.md "Mail" (48 + 17 r), [[37]])
- docs/engine/font.md: where ccKanji's texture sits in VRAM next to xasc00 — research (duplicates [[23]])
- docs/engine/font.md: what QUA's ending routine does while the KFED buffers are loaded — answered (stale; [[239]])

### docs/engine/grunty-ride.md

- docs/engine/grunty-ride.md: the ride's first Main and the cancel check a frame late — research (duplicates [[166]])
- docs/engine/grunty-ride.md: the port sleeps and wakes the whole party — research (duplicates [[166]])
- docs/engine/grunty-ride.md: plw +0x10 not kept — research (nothing the port runs reads it)
- docs/engine/grunty-ride.md: ccDamUprStr::CtrlAll not run in the menu's waits — research (no number flies then)
- docs/engine/grunty-ride.md: the Grunty's leg nodes from the port's last pose — research (duplicates [[166]])
- docs/engine/grunty-ride.md: pgDIN and the flute outside a field not played through — research (test gap)

### docs/engine/map.md

- docs/engine/map.md: which gimmicks an entry control holds beyond the portals — research (depends on objects not yet constructed)
- docs/engine/map.md: menuClrWait, ccCheckGtHackAnm and the party's annihilation do not hold the map button; ccGame.inBattle does not hide the map — play (duplicates [[77]])
- docs/engine/map.md: what sets WORLD_MAN.specialRoom beyond the known rooms; ccMenu.mapStatus 3 — research (duplicates [[77]])
- docs/engine/map.md: the rotated cull reads a leftover width and height — research (duplicates [[144]])
- docs/engine/map.md: RT01ICONPOS and the pulse angles restart per area — research (phase only; duplicates [[77]])
- docs/engine/map.md: ROOTTOWN03-05's DrawMap — play (duplicates [[260]]; crates/piney-world/src/map/town.rs now has a fourth sprite, to confirm)
- docs/engine/map.md: ShowMap reads EVENTAREA_INFO by eventAreaNumber; the host takes the model its WORLD_MAN was made with — research (the same area)

### docs/engine/overview.md

- docs/engine/overview.md: ccEvent::Execute, the event script interpreter — answered (stale; [[40]], docs/engine/event-vm.md)

### docs/engine/particles.md

- docs/engine/particles.md: ccParticle::MainStr (streams' particles); the port runs Main for them — research (not compared apart)
- docs/engine/particles.md: forceType 10 with a force.w not 0 — research (no row has one)
- docs/engine/particles.md: a deleted generator's particle reads freed memory — research (game UB)
- docs/engine/particles.md: ccParticleExplode past type 7, texture id 237, Setup's styles past 2 — research (read beyond their tables)
- docs/engine/particles.md: what each generator row is for beyond the named callers — research (not surveyed)

### docs/engine/render.md

- docs/engine/render.md: the DMA order of the three packets — research (duplicates [[17]])
- docs/engine/render.md: the bone and skin packet layout; which of mc03b/mc03m is skin — research (duplicates [[17]])
- docs/engine/render.md: view + 0x110; the mc04b/mc04m variants; the effect program — research (duplicates [[17]])

### docs/engine/root-towns.md

- docs/engine/root-towns.md: the BLT chunks' GS uploads not described — research (the renderer does not need them)

### docs/engine/shadow.md

- docs/engine/shadow.md: which path (direct or clipped) a volume's polygon takes on the console — research (duplicates [[160]])
- docs/engine/shadow.md: mc0_CheckBoundingBoxShadow's moved corners read an unset register — research (duplicates [[160]])
- docs/engine/shadow.md: an open edge's second face read from VU1 memory below the directions — research (duplicates [[160]])
- docs/engine/shadow.md: modes 2 and 3, never made in Infection — research (duplicates [[160]])
- docs/engine/shadow.md: packet +0x15e's values; whether a stream's copies of 0 occur — research (not surveyed)
- docs/engine/shadow.md: EVENTAREA02 and 07 set the light again in ChangeBlock; the port takes the area's distant light each frame — research (duplicates [[160]])
- docs/engine/shadow.md: a stream node naming shadow layer 0 — research (the port uses the default packet)

### docs/engine/source-tree.md

- docs/engine/source-tree.md: "nothing about the list itself" — dead (no question stated)

### docs/engine/sound.md

- docs/engine/sound.md: which params the animations' notes 1 and 2 carry; reads past setbl's table — research (not surveyed)
- docs/engine/sound.md: ccSndSQLoad(6) (sqDataStream) not ported — research (nothing in Infection asks for it)
- docs/engine/sound.md: the canals' and the church's volumes not compared with the game playing — research (duplicates [[184]])
- docs/engine/sound.md: event bank rows past 122 read past sqDataEvent — research (no scenario)
- docs/engine/sound.md: the SPU2's own behaviour from documentation — research (duplicates [[42]])
- docs/engine/sound.md: the synthesizer paths no data takes — research (none reached)

### docs/engine/statics.md

- docs/engine/statics.md: why the water is three copies — answered (docs/engine/root-towns.md (waters 1 and 2 made and stepped once, never drawn); [[97]])
- docs/engine/statics.md: how EVENTAREAB0 chooses its area — answered (stale; [[112]], docs/engine/evarea.md)
- docs/engine/statics.md: the sky CMP_sr1bac1 and town01d's CMP_sr1dat1_1-3 built by the constructor — answered (described in the bullet; crates/piney-world/src/town01.rs (sky, CRISIS_SKY))
- docs/engine/statics.md: ROOTTOWN03-05's Draw — answered (stale; [[218]], [[260]]; their DrawMap is the map.md line)

### docs/engine/stream.md

- docs/engine/stream.md: the hit marks and transfers of streams 5, 7-9, 11, 12, 14-16 — answered ([[131]] (drawn by the stream's own effects))
- docs/engine/stream.md: VU1's divZ clip modelled for the unlit rigid program; no close-up compared — research (duplicates [[153]])
- docs/engine/stream.md: the lights' priority order checked in streams 15 and 6 only — research (duplicates [[136]])
- docs/engine/stream.md: stream 15's opening against a game picture — research (duplicates [[136]])
- docs/engine/stream.md: stream 15's parts checked as draws, not packets — research (duplicates [[133]])
- docs/engine/stream.md: stream 15's switch and str0581's frame-0 note — research (duplicates [[133]])
- docs/engine/stream.md: str0580's first 240 frames show only sky, clouds and figures — answered (the scene's own (BUGS.md story:31; [[136]]))
- docs/engine/stream.md: the fog's pixels; the feedback's bottom edge; a band's copy past the draw buffer — research (duplicates [[57]])
- docs/engine/stream.md: the rand state when stream 2 starts on the PS2 — research (duplicates [[57]])
- docs/engine/stream.md: the task's timing from priorities; an old task's last frames under flag 4 — research (duplicates [[57]])
- docs/engine/stream.md: frames the reading and decoding take; where ccGetStreamFrame falls in a frame — research (duplicates [[52]])
- docs/engine/stream.md: the pause (flag 1) released after the Chaos Gate loop plays through, not when ccLoadResourceFL returns — research (duplicates [[106]])
- docs/engine/stream.md: strse of strSndTbl — research (no stream has one)
- docs/engine/stream.md: when ccEventStream's loop sees the end — research (duplicates [[85]])
- docs/engine/stream.md: direct and spot light records — research (no Infection stream has them)

### docs/engine/text.md

- docs/engine/text.md: reFlg and mailFlg — research (duplicates [[9]])
- docs/engine/text.md: fromNO's table — answered (docs/engine/desktop.md "Mail" (the sender's photo CMP_xddphot{fromNO + 1}))
- docs/engine/text.md: dateindex — answered (docs/engine/toppage.md (the date cell, 0-5))
- docs/engine/text.md: HtmlData.flg — research (not traced)
- docs/engine/text.md: what ccGetExtendedCode maps and how % codes are drawn — answered (the bullet points to docs/engine/font.md ([[23]]))
- docs/engine/text.md: which mails and posts the game actually delivers in Infection — research (the scripts deliver them (op 107-109, [[41]]); no list made)

### docs/engine/title.md

- docs/engine/title.md: how many frames ccThSaveSys takes to answer the boot check — research (the port says 2)
- docs/engine/title.md: how the opening stream's frames line up with the task's (the end flash) — research (duplicates [[52]])
- docs/engine/title.md: the opening's cancel flash is not modelled — answered by [[342]] (`Demo::stream_tick`; the_opening_stream_flashes)
- docs/engine/title.md: m_tempPN's first value — research (duplicates [[50]])
- docs/engine/title.md: how many frames DataRead and the index read take on a real card — research (card timing)
- docs/engine/title.md: volumes 2-4: Data_Control's m_PrevFlg branches and ccSaveData::ConvGame — answered by [[387]] (docs/engine/title.md "CONVERT (volumes 2-4)")
- docs/engine/title.md: what sf 7 of ChangeRequest means — research (not traced)
- docs/engine/title.md: what the logo PSS files show and how long each is — research (not written up; [[245]]: the same bytes on all four discs)
- docs/engine/title.md: how many frames ccSetupDesktop takes to reach ccAllSoundOff — research (duplicates [[51]])
- docs/engine/title.md: how long the IOP takes over the 0x9310 load — research (console timing)
- docs/engine/title.md: where the sound task runs in a frame relative to ccThDemo — research (task order)

### docs/engine/toppage.md

- docs/engine/toppage.md: whether the task's first Main runs in its start frame — research (duplicates [[58]])
- docs/engine/toppage.md: whether kNewMsg's label is ever visible past the board's scene — research (not traced)
- docs/engine/toppage.md: how ccScFade draws while the system menu holds the layers' flip — research (duplicates [[58]])
- docs/engine/toppage.md: when ~ccThToppageCtrl runs after a hand-off (the posts read written back) — research (the port writes them at the hand-off)
- docs/engine/toppage.md: the EE's div by zero result from PCSX2 — research (not measured)
- docs/engine/toppage.md: which events fill timeIdolRankStr — research (not traced)

### docs/engine/town02.md

- docs/engine/town02.md: the Grunties' camera writes land a frame's NPCs late — research (duplicates [[162]])
- docs/engine/town02.md: effEvolvePG's and effGrowPG's particles not compared — research (duplicates [[162]])
- docs/engine/town02.md: the dogs' walks; the stuck count +0x21c — research (duplicates [[142]])
- docs/engine/town02.md: EFF_srzsmo1's alpha test (clouds by colour) on the console — research (duplicates [[97]])
- docs/engine/town02.md: the frame-buffer copy's size and point sampling — play (duplicates [[97]])
- docs/engine/town02.md: whether water 0's coordinate has a local translation — research (duplicates [[97]])
- docs/engine/town02.md: fieldrand on entering the town; the scrolls across visits — research (duplicates [[97]])

### docs/formats/pss.md

- docs/formats/pss.md: the IPU's IDCT arithmetic — research (duplicates [[49]])
- docs/formats/pss.md: what the SPU2 plays after the last audio set — research (duplicates [[49]])
- docs/formats/pss.md: frames between ccDecodeMpeg's call and the first picture — research (duplicates [[49]])

### docs/formats/snddata.md

- docs/formats/snddata.md: the SPU2's own ADPCM rounding — research (duplicates [[14]], [[243]])

### docs/formats/voice.md

- docs/formats/voice.md: the 48 kHz rate from the SPU2's fixed rate and measurements — research (duplicates [[14]])
- docs/formats/voice.md: the transfer interrupt's timing and the cut tail — research (libsd's documented behaviour)
- docs/formats/voice.md: how long the disc takes before a line's first sample — research (disc timing)
- docs/formats/voice.md: whether a Recovery Drink used by Kite on Quarantine stops channel 0 — research (duplicates [[245]])
