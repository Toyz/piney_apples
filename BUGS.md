# BUGS

- [x] during cut scene with kite and black rose after ocra opening and about to go to hidden forbidden holy ground blackrose weapon is missing (party members now get ccSpcChar::EquipWeapon; fixed)

- [x] gating between town -> field seems to be missing the animation (the town now runs effTransfer for Kite, members and walking PCs; fixed)

- [x] hidden forbidden holy ground never actually loads just black screens (the town left its event task running through the gate; fixed)

- [x] notice you're aren't exporting tbl data typically like we did with scene tbl and such (tools/tables.py dumpall: every DWARF struct array, 586 of them, in work/infection/tables/<main|gcmn|desktop|toppage|demo>/)

- [x] minimap clobbers when you change floors instead of being relative to that floor (the stairs now set mapHideFlag and the map task sleeps in the change frame; fixed)

- [x] first few scenes of story:31 the camera clips the ground culling it out (not a clip: the game draws it the same. str0580's own tracks fade the ground and towers out at frames 61-70 and never bring them back; from 121 the camera looks down past the horizon onto the sky dome's dark bottom. The port's scene now matches the game's frame by frame through frame 420; that check also found the stream lights in the wrong order (distant lights go last), fixed)

- [x] gate out/in effects still missing within cut scenes (streams now run their effTransferStr/effHitMarkStr; members transfer in fields and towns; fixed)

- [x] credits overlapped the desktop which seems like a bug also might be state transition mis (staff_roll's ccSleepAllThread: the desktop now sleeps, undrawn, from the roll to its ccWakeAllThread; fixed)

- [x] gate hacking test placement is shifted or incorrect (looks like it's placed too high up) (the core table and the slot label drew through the menu view; their layers have their own, aspect 1; fixed)

- [x] also looks like im missing virus core M(S) from invetory for story 18, my guess is we need to have been given both or gotten a second some on are travels unclean (right guess: event 11 gives one, event 17's Data Bug drain the other; story starts now carry the drained cores and earlier hacks; fixed)

- [x] game (play) time is not tracked (ccAddPlayTime every main-loop frame; fixed)

- [x] (found while fixing) story NPCs in fields and dungeons (Meg, the Administrator, events 14, 17, 26, 27, 29) are not built or drawn yet (the entry control now makes them and the world runs the towns' classes for them: put, turn, face, act, trans, their transfers; fixed)

- [x] the gate-in sound and effect played every time a party member loaded in, in a dungeon (members always started in act 13; ccFellow::Initialize has them come in through the gate only in a town or a field come to from one, and in a dungeon only on a warp: they now stand; fixed)

- [x] story 11: after Balmung and BlackRose's streams in the church the screen stayed black ("loading 1", Kite in act 13) before Data Drain (block 24's "The book. / Open the book." windows open in the set-up pass at phase 2, before the field's menu exists; they went to the field UI, which does not draw during the load, so nothing showed and the pass waited on a button. They now open on the set-up's own screen, black with the event's window, as the desktop's set-up does; fixed)

- [x] during Skeith's spawn the game lags badly at the start of his animation (stream 5 held one frame 135 ms when the book's bubble first drew from `str0120`, which the renderer read then: it now reads a stream's files on a thread as the stream starts. What remains is each stream's own load at its first frame, 75-200 ms, at the cut between streams; fixed)

- [x] Data Drain z-fights its textures when Skeith drains Orca (a gold ring fading in at alpha 2 writes Z to erase the figure; the GPU's float alpha fell under the test's 2 at some pixels, so the erasure was dithered: the shader snaps the vertex alpha to the GS's integer; fixed)

--- feature ---

Maybe an ingame quake console so we can debug and test various game features more efficiently, such as give me items, states, different party members, and other game mechanics.

  - [x] started: F1 (or `) opens a console; `help` lists: item, core (virus cores), gold, heal, address (a member's address, then PERSONAL > Party > Add), flag, time, where, story N (restart at a story start). `--console "cmd;cmd"` runs them headless with --shot.

- [ ] the launcher's turning icons glitch: close to the title menu's turning icons but not the same (worklog 253 turns OBJ_xdt_ico_X1_ about its y by Scene::turn_y; compare against the title's ANM_xdt_ic00 pose by pose)

- [x] the dungeon's music kept playing from the Gott statue room through Skeith's scene and on under the Helba theme (the desktop set-up's `ccAllSoundOff` sends SNDBASE's 0x140, whose `allSoundOff` stops every sequencer; the port's only silenced the voices sounding, so the dungeon's sequencer played on under streams 4-6; `all_sound_off_stops_the_music`; fixed)

- [ ] the STATUS title bar has 1-pixel gaps (found: the title's blank cell, `xwindow` cell 38, is 8 texels of bar and a clear ninth column in all four discs' texture, and `DispLine(0)` draws it 9 wide with nearest sampling, so the game's own code and data give the same gaps; waiting on a PS2 capture of a titled window to compare)

- [ ] the chest's target cursor shows a ghost square (the four corner arrows and the diamond are the game's: `targetCursol`'s square outline turned a fixed pi/4 at gcmn 0x0051f36c; which square is the ghost is still to be pinned down)

- [ ] after event 18's Mia and Elk scene in area 19's dungeon, back in Mac Anu (block 20's `scene area=0 town=0`), the town's loading card stays up over a stray camera and the frame rate collapses

- [x] a Fairy's Orb does not reveal the whole map (the portals and the rest) (its `ccUseItemRequest` step `WaitMap` reached the world as a step the port did not carry out: `WORLD_MAN::ShowMap` was never called. The field host now calls it on the step and once a frame before the menu task until it is done (a field's `WORLD::ShowMap` at once, a dungeon's floor one room a frame), and the menu breathes until then and 20 frames in all, as gcmn 0x0057b95c-0x0057b9d0 does; `a_fairys_orb_shows_the_portals`; fixed. The field map always shows the dungeon entrance; the orb adds the magic portals)

- [x] the board's keywords "Expansive Haunted Sea of Sand": the field has no dungeon (it has one: story area 17's field (seed 219639073, type 3) puts its entrance, `CMP_sfd1sto1`, at chip (1, 28), the map's far west edge, as the game's `WORLD::Generate` does (tools/test_field_rs.py); the start is chip (20, 20). The map not showing it is the Fairy's Orb item below; not a bug)

- [ ] many stat UP icons on the field (the party's boosts) - to be pinned down (one cause found and fixed: buffs froze through town visits, above, so they piled up across fields; whether the members also cast more of them than the game does is not yet measured)

- [ ] enemies' effects stay after they die (a floating staff and red marks after BATTLE MODE OFF) (not reproduced with event 3's goblins: `after_a_fight_shots` fights them to the end and shoots the field 0, 40, 120 and 300 frames after the fight; nothing of theirs is left drawn. Which enemy carried the staff, and in which area, is still to be told)

- [x] Kite's idle animations (the jump and the other fidgets) never play when he stands still (they do: after 451 frames standing (7.5 s) he jumps (`nut3`), then loops `nut4`, in towns and fields alike, as `AnimCtrl` counts `reactCnt`; the count only runs with no enemy near (`inBattle` not 1), outside the eye view, and not while the menu or an event holds him; checked headless in field 14: the jump at frames 540-580; not a bug)

- [x] NPCs (such as Helba) keep moving while Kite talks to them (Helba's menu is `SpcMenu`, a registered character's: its `EntryAffect(14)` / `(15)` / `(0)` reached nothing in a town, where `World::affect` passed on only the gate's, the Grunties' and the walking PCs'. It now goes to the character's `ccFellow::Influence` through the battle's rules, whose `ccAI::Greeting` sets talkFlag and gDeg toward Kite so that `Brains` stands it facing him until 0 clears the flag; the town's party frame runs the queued lines first, as the field's does; `a_member_spoken_to_stands_facing_kite`; fixed. The walking PCs' `rTownNPCInfluence` already stood them)

- [x] a registered character's menu (`SpcMenu`: Talk / Trade / Gift) draws over the target window's HP and SP, with the target cursor still up (not a bug: the owner's PS2 capture of a walking PC's Talk / Trade (Panta) shows the target cursor staying on the PC, the name frame at y 96 and the list under it from y 114, as the port draws it (`pc_menu_shot`). A party character's list is the same code's other case: `ccMenuCtrl::Disp` (gcmn 0x0051d390-0x0051d41c) puts it at (39, 129), a row taller, its rows from 169, behind the target window's HP and SP (`piros_menu_shot`))

- [x] the party's field status (the UP boosts) carries over from one field to the next (the game carries them too: the timed buffs live in `ccSpcParam` (`temp`, `time`), the save's own record, and `ccSetupGameCtrl` stores every member's conditions for the next area (`ccStoreSpcCondition`, gcmn 0x0056cc40). But they time out everywhere: `ccPlayer::Main` runs `CalcReal(dead)` every frame in a town as in a field (gcmn 0x005983c8). The port's town never ran Kite's, and its party frame never read the members' records from the save or wrote them back, so a town visit froze them; the town frame now does both (`kites_buff_times_out_in_town`); fixed)

- [x] the protect break's animation is missing (the console's `protect` only set the foes' `ppCount`; it now raises the break as a hit does: `effProtect` (ANM_xdhpros*, sound 79), `SetProtect`'s marks; `the_console_cheats` checks the marks. A break by hits runs `CalcBattleDamage`'s own events, which reach the effect; the effect draws (effect_shot --effect protect). If it was a hit's break that showed nothing, still to be found)

- [x] a console command to go to a town (`town N`: 0 Mac Anu, 1 Dun Loireag, 2 Carmina Gadelica, 3 Fort Ouph, 4 Lia Fail; Net Slum is not one of Infection's towns)

- [ ] the music breaks between towns after the console's `town` (the session's own events through a headless engine give each town its music on field -> Dun Loireag -> Mac Anu -> Dun Loireag, `town_jumps_keep_the_music`; what was heard and where the jump started from are still to be told)

- [x] Lia Fail's lamp glows (blooms) drew in front of the buildings (`ROOTTOWN05`'s 19 `EFF_se1_1ef1` glows keep `ccEff::Init`'s depth test and fog: its constructor and `Draw` (OUT gcmn 0x0043e8e0) never call `SetRenderState`; the port drew every town sprite with the depth test off, as only `LENSFLARE` and `CLOUD` set it; `TownSprite::depth`, checked with `town_shots`; fixed)

- [x] Outbreak's and Quarantine's opening cut scenes broken audio-wise (their streams carry a PCM track a voice language: each Pcm chunk (its byte +7) and each F_Pcm record (`u32 id, u8 lang, pad, u32 blocks, u32 words`, one more word than Infection's) says its language, and `DecodeF_Pcm` (OUT 0x0014d240) reads past a record whose language is not `ccPcmSound +0x1c`, the save's voice byte `ccRequestLoadStream` gives `Open`. The port read Infection's layout, took the language for the block count and the block count for the words, and played 28-byte scraps: 0.8 s of noise over stream 52's 113 s. It now keeps the voice's track (`Loaded::pcm_track`); and a disc with only `streamTblE` reads the `E` archives whatever the voice, where a Japanese voice found no stream at all; `later_volumes_play_their_voice_track`, `volume_sound_survey`; fixed)

- [x] the later volumes' music around streams: `ccSndStreamCtrl` was Infection's switch for every volume (fixed: each volume's own, Quarantine's being Outbreak's: `stream_ctrl` takes the volume; piney-stream no longer filters by Infection's stream lists, the switch decides as the game's does; the Chaos Gate stream is 113 from Mutation on, not 107, `table::gate_stream`; each volume's code run in eemu, `tools/test_stream_rs.py music` with `PINEY_VOLUME`, 1399 scenarios each: `the_later_volumes_music_around_the_streams`)

- [ ] the later volumes' animation-note sounds: `setbl` (`spc0SeData` .. `inuSeData`) is Infection's for every volume

- [ ] Infection, towns: an NPC being talked to can run off mid-talk
- [x] Infection, Mac Anu: after the cut scene with Mia and Elk (event 13, MG0340: `entry 2 1`, `entry 2 10`, then `remove -1 -1` and `scene -2`) both stay in the town (the game drops them at the change of scene: each fellow task's delete, `ccThFellow01Delete` .. `17Delete` (gcmn 0x0041ec20 ..), frees the registry slot of a character whose own partyFlag is not 1; the port kept them registered and built them again. Fixed: `Spcs::delete_fellows` at every change of scene; `event_13_leaves_mia_and_elk_out`)
- [x] a gamepad's D-pad did nothing while the sticks worked (a pad with no SDL mapping, DirectInput among them, reports its D-pad as a hat, gilrs's `DPadX` / `DPadY` axes, not as buttons; those axes now press the directions too, and the pad log shows them)
- [ ] Infection: some particle effects are missing (which ones not yet known)
- [ ] Infection, the Haunted Expansive Sea of Sand's dungeon entrance: talking to BlackRose after the Administrator leaves the screen stuck as the church once did, and it persists
- [ ] the members' chat: Balmung's line that names two elements (Thunder against Darkness) overflows the game's chat buffer and crashes the game (a community find; the same overflow explains a Rachel party oddity in Infection); check the port neither crashes nor differs
- [ ] the community's word on story areas' enemies: the first enemy list of each server (`enemyList00` for Delta's event fields); the executable (`ccRegisterDifficultyEnemy`, all four volumes) passes type 6, `ccEnemyListInfo[server][6]`: compare
