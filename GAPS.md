# GAPS

What a player of Infection would still meet, from a read of every Unknown
list in docs/engine (258 items, 2026-09-26). Most of those items are not
gaps. They are fields whose meaning is unknown although the port matches
the game, the game's own undefined behaviour, timings of the console's
card and disc, or later volumes. The ones below are the real ones, most
visible first. Mark them [x] with a note when done.

## Missing in play

- [x] The save after the staff roll: desktop menus 8 and 9 (`SaveSelMenu`, `SaveMenu`) save the clear data to a card slot (desktop.md "The save menus after the staff roll"; checked against the game's ccThDtMenu and ccSaveSys, and the ending played to a saved card: worklog 0159)
- [x] Dungeon rooms' dressing: `SetObject`'s statues, flowers and walls (with hits), `SetAnmObject`'s animated objects (with hits), `SetWater`'s water and sparks, and `SetLight`'s omni lights and glows (dungeon.md "The dressing"; checked against the game's SetRoom and DrawEff: worklog 0157; the lakes' sky and fireflies: worklog 0161)
- [x] Data Drain's side-effect visuals: the fly fonts, `effSkillStartEffect`, `effAfterDrain` (field-ui.md; each start of step 10 in the game's order and step 11's LEVEL DOWN now shown, checked against the game and on a drain of Skeith: worklog 141)
- [x] CHAT's per-member menus (71-73) and the field objects' menus 34-37 and 39 (field-ui.md; checked against the game, a member ordered to heal in a fight and a breakable broken in play: worklog 0148)
- [x] The Zeit statue (39): `timeSym` handed to `SetIDOL` and `gameCnt` counted; "Chronicling" is a BBS word of Infection's (field-ui.md, worklog 0154)
- [x] The lakes' fountains: `ccGimEtc` row 20 (the spring, `initFountain`, `ctrlFountain`), menus 40-42 (`FountainMenu`, `FountainMenu2`, `FountainMenu3`: the item thrown in and Monsieur's golden axe game), `SetFountainCamera`, `effFountain`, the voices and `SetFountain`; and row 19, the boss rooms' warnings (`effBossRoomEntrance`, `GetNearDoorPosition`, `CheckBossEffect`) (field-ui.md "The spring (40 - 42)", battle.md; checked against the game's ccGimEtc::main, the menus and GetNearDoorPosition, and an item thrown in on a night lake in play: worklog 0163)
- [x] Dun Loireag's chibi Grunties (`ccSetChibiGuso`, `ccPGuso`, menus 45 and 46) and the Grunty foods (`ccGimFood`, menu 43) (town02.md, field-ui.md; checked against the game's ccPGuso, ccGimFood, the menus and effEvolvePG/effGrowPG, a Grunty fed and grown and a food picked up in play: worklog 0162)
- [x] The riding Grunty: the Grunty Flute's call (`ccPgAdultCheck`, `ccPuccigusoStart`), `ccPucciguso` and `pgRideFlag` (grunty-ride.md; the object's functions, the start's and dismount's order of calls and ccPgBgmInit/End checked against the game in eemu, a ride and its dismount in a field played through the session: worklog 0166)
- [x] A field's `WORLD_MAN::EntryGimmick`: `WORLD::SetFood` (the foods and a lake's spring), `SetMagicCircle` (the portals and enemies), `SetSpecialObj` (the entrance swirls, `effDungeonEntrance`, and the symbols) (battle.md "Where the entries come from"; checked against the game's setters: worklog 0164)
- [x] The game over: `ccThGameOver`'s noise, the TV switching off, GAME OVER and the reset to the title (field-walk.md; worklog 0158)
- [x] The shadow volumes: every character's, the weapons', TOBJ's and the streams' (`ccShadowModel`, `ccShadowPacket`, the VU1 programs; shadow.md, checked against the game's DecodeShadowModel and ccShadowModel::Draw with VU1: worklog 0160)
- [x] The shrine's direct light (0x0605 controller) (animation.md, dungeon.md; checked against the game's SetLightMatrix: worklog 0151)
- [x] The enemy races: every race's constructor and motion (battle.md "Enemy movement and animation"; checked against the game: worklogs 0168, 0169, 0171), with the fire breath and the turtles' foot dust drawn (worklog 0172); a stress run fights through 35 random fields of Δ (`many_fields_fight`). Not ported: `ccEnemyL`'s type 3 (rows 203-206, `moveELG`/`dispELG`), which no list of the Delta server registers
- [x] Drawing: the near clip at the streams' `divZ` (stream.md; worklog 0153; the rotated and Gouraud sprites are done, worklog 0144; the fog by depth, worklog 0145; REGION_REPEAT in piney-gs, worklog 0146)

- [x] The loading display between areas (`ccLoadDisp`, main 0x0019ad70-0x0019c430, 6.4 KB: the `xdl_load` scene, the server, field and town cards, a 47-frame outro once the area's set-up is done); `ccFileListLoad` puts it up whenever the new scene list has a file the old one lacked (`loadCheck`). Found by an audit of the functions no crate or page names; ported (field-walk.md "The loading display", worklog 186)

## Done in the sweep

- [x] Kite's footsteps and running dust in town, the Chaos Gate's circle sounds, the dungeon doors' sounds (worklog 0138)
- [x] The walking PCs' texture variants (`changeTEX`, worklog 0139), their footsteps and dust
- [x] The dungeons' breakables (`EntryBreakObject`, worklog 0140)
- [x] Dun Loireag's dogs (`ccSetDog`, `ccDog`, `NorainuMenu` 44; worklog 0142)
- [x] The story dungeons' symbols (`ccGimSymbol` row 17: its skill, cast, light and fires; worklog 0149; the lakes' row 18, `objMain`, too)
- [x] The dungeons' palette swaps for clutType 3 and 4 and the lakes (`SetClutList`, `ChangeClut`; worklog 0147)
- [x] Talking to an event's NPC in a field or dungeon (event 17's Administrator, `talked_to 4 29`; worklog 0143)
- [x] The party's HP and conditions through the doors, and around the event scenes (`storeCondition`, `menu_ban` / `menu_clear`; worklog 0135)
- [x] F_Obj: three enemies' damage, down and magic clips (worklog 0137)
- [x] Stale lines: the party's field AI, the members' town lines, Skeith's death effects (ported: `BeginDeadEffect`, `ccBossEffDead`)

## After Infection

One build boots all four discs, with no read of the boot executable at run
time: plans/volumes.md, plans/build-data.md. Status on 2026-09-27.

- [x] Phase 0: `Volume` (`piney_data::volume`, detected from `GCMN.PRG`);
  the generators run for all four volumes
- [x] The executable off the port: every table generated per volume from the
  discs by `piney-gen` into the build (`PINEY/TABLES`), shared where the
  volumes agree (plans/build-data.md steps 1-3; worklogs 257-259)
- [x] Phase 2: Mutation's, Outbreak's and Quarantine's data generated
- [ ] Phase 3: the ported functions audited across volumes (`tools/voldiff.py
  ported`, worklog 198: 296 of 2,138 changed in MUT, 561 in OUT, 565 in QUA;
  each to be read and handled). Found so far and handled: the streams' PCM
  track a voice language (Outbreak on); `ccSndStreamCtrl` (the music
  around streams); `setbl` (the animation notes' sounds)
- [ ] Phase 4: each volume's story end to end. Infection: done. Mutation: in
  progress, 13 of 16 events finish under the autopilot
  (plans/mutation-story.md). Outbreak and Quarantine: power-on to the
  desktop with the opening streams' sound; the rest after Mutation

What the later volumes' code adds, by `tools/voldiff.py` (docs/disc/volumes.md
"What the later volumes' code adds"):

- [ ] The new streams' effect functions: all thirteen of Mutation's ported
  (worklog 267, 269); Mutation's `str7100` / `str8800` / `str0300` are
  Infection's code (worklog 271); Outbreak's and Quarantine's own
  (`str1430` ...) not yet
- [ ] The event engine's new instructions: decoded by volume (99 `noise`,
  168 `grunty_mail`, 169 `ending_kanji`); `grunty_mail` is ported (the
  desktop's), Quarantine's `ending_kanji` not
- [ ] The Grunty race (`PG_RACE`, `town06`) and Quarantine's staff roll and
  hacking logos
- [ ] A new menu after `AreaInfoMenu` (messages, number input)
- [ ] Kyvia's fights: [x] the first (`ccBossKyvia01`, `kyviaCore`,
  `kyviaGomora`, worklog 272), [ ] the later ones (levels 2-5, the EX
  mode, the `ex2`-`ex4` models)
- [x] The Root Towns 02-05 (`ROOTTOWN03`-`05`, their `DrawMap`; worklogs
  218, 260) - the party of 21 (`SPC_01`-`SPC_20`) still to check
- [ ] The other `EVENTAREA` classes (04-06; areas 66, 67, 91).
  Area 16's `EVENTAREA07` (worklog 177), area 43's `EVENTAREA03` (worklog
  263), area 13's `EVENTAREA01` (worklog 264) and Kyvia's disc
  `EVENTAREAB8` (fields 9-12, worklog 272) are ported, all behind
  `StoryMap`
- [ ] Mutation's bosses: [x] Innis (`ccBoss02`, worklog 266; its pictures
  and event 107 under the autopilot still open), [x] Kyvia 01 (`ccThKyvia01`,
  the `kyvia*` classes, worklog 272; event 108 under the autopilot still
  open), [x] Magus (`ccBoss03`, `ccBoss03Leaf`, worklog 274; its pictures
  and event 115 under the autopilot still open)

## Asked for, not the game's

- [x] Console `invite_party ID`: a character into the party whatever the
  story's state, for testing (a town: at once, at the gate; a field: with
  the next area; `the_console_invites`)
- [ ] Console toggles for the developers' leftovers: the bosses' `OnCheatHP` /
  `OffCheatHP`, `ccPGuso::testDisp`, `ccStream::PrtDebugStatus`, `waterTest`
  (main 0x0017bac0). No free camera: `ccCam::SetMatrix_PosRotXYZDebug` is the
  streams' camera (`PlaySceneMain` calls it every frame)
- [ ] A player-facing README for the release (plans/release.md)
