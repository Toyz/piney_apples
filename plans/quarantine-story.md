# Quarantine's story end to end

Goal: Quarantine plays from power-on, and from a carried-over Outbreak save
(`ConvGame`), through every story event (M401-M414, 301-314) to its ending
and staff roll under the story autopilot, as Infection, Mutation and Outbreak
do (plans/mutation-story.md, plans/outbreak-story.md). Started 2026-10-24.

## The story

`tools/evscript.py list work/quarantine/disc/SLUS_205.64`: M401-M414 are
events 301-314 (314 "Ending"), each opening on the one before. M415 (315)
is the after-game (Kite and Orca on their journey again), M416 and M417 the
member and item extras; they are not the story.

| event | start | first located block |
| --- | --- | --- |
| 301 | the new game's desktop | the desktop, then Fort Ouph (town 3), town 5, Lia Fail (town 4) |
| 302, 303, 305-307, 311, 314 | the desktop | a dungeon (302: field 100; 307: 107), the desktop |
| 304 | Fort Ouph (`in_town 3`) | |
| 308-310, 313 | Lia Fail (`in_town 4`) | 313's first blocks in towns 0-3 only mark its gate |
| 312 | Mac Anu (`in_town 0`) | |

The fights: 307 `entry 7 5` (`ccBoss06`, field 6's arena), 308 `entry 7 14`
(Kyvia's third, field 11's disc), 310 `entry 7 6` (`ccBoss07`, field 7's
arena), 312 Kyvia's fourth (field 12, 11), 313 `entry 7 7` (`ccBoss08`,
field 8's arena). 314 is field 126 (`area 126`, `scene 1 0 126`), then
`ending_kanji` (op 169: `STFROLL_VOL4`, the hacking logos) and the
desktop's `staff_roll`.

## How it is checked

- `quarantine_story_starts_open_their_event`: each start opens its event.
- `quarantine_story_survey` (`--ignored --nocapture`, `PINEY_SURVEY_GOD`):
  each start driven by `StoryPilot`.
- `quarantine_whole_story`: one run from the new game to 314's end; and one
  from a carried Outbreak save (to come).

## Where it stands

Survey of 2026-10-24 (`PINEY_SURVEY_GOD=1 PINEY_SURVEY_FRAMES=40000`, each
event from its own start, 447 s):

| event | state | last place, and why |
| --- | --- | --- |
| 301 | blocks 0x7b | Fort Ouph. Block 7 wants town 5 (`town06`, `ROOTTOWN06`), which the port cannot build; the gate's warp to town 4 lands in town 5 while `eventStatus[51]` is set (QUA gcmn 0x0047306c) |
| 302-306, 309 | done | |
| 307 | blocks 0xf | field 6's arena: `start_boss` has no code 5 (`ccBoss06`) |
| 308 | blocks 0xf7 | field 11: Kyvia's third fight still going at 40,000 frames |
| 310 | blocks 0x3d1 | field 7's arena: no code 6 (`ccBoss07`) |
| 311 | blocks 0x6ff | field 103's dungeon: the first of four goal rooms done at 40,000 frames |
| 312 | blocks 0x209 | field 108's dungeon at 40,000 frames |
| 313 | blocks 0x1 | Lia Fail: BlackRose's "Are you ready?" not answered (the party she asks for) |
| 314 | blocks 0x7 | field 122 through the gate: `area 126` (no words) and its `scene` unported |

## The gaps, in story order

1. **301**: `town06` (`ROOTTOWN06`, QUA gcmn 0x004ec350; its table found by
   piney-gen), the gate's town-5 warp. Then blocks 7-10.
2. **307**: `ccBoss06` (code 5), with its effects (`ccBossEffLaserShot`,
   `ccBossEffLightBallShock`, ...).
3. **308**: the pilot through Kyvia's third fight.
4. **310**: `ccBoss07` (code 6).
5. **311, 312**: longer runs (four goal rooms; Kyvia's fourth).
6. **313**: the pilot's answer to BlackRose; `ccBoss08` (code 7) in all its
   forms.
7. **314**: `area` / `scene` to field 126, `ending_kanji`, `STFROLL_VOL4`,
   the hacking logos.
8. The whole runs: from the new game, and from an Outbreak save
   (`ConvGame`).

Side finding to settle (worklog 439): town 4 stalls at about 1,440
vblanks with the heap pre-filled: some town-4 code reads heap memory it
never wrote and expects 0.
