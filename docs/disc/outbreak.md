---
title: The Outbreak DVD
status: solid
volumes: OUT
covers: OUT SYSTEM.CNF, every file on the disc, OUT STREAM/STRT.BIN (its six scenes, played), OUT VOICE2
worklog: 239, 240, 247
---

# The Outbreak DVD

What is on the .hack//Outbreak (USA) disc, where, and which page describes
each file. It is measured against [the Infection DVD](layout.md) and [the
Mutation DVD](mutation.md); what the four discs share is in [the four
discs](volumes.md).

## Volume

ISO 9660 only - no Joliet, no Rock Ridge. 2048-byte sectors, 1,859,600 of
them (3,808,460,800 bytes). System identifier `PLAYSTATION`, volume
identifier empty. The size fits a single-layer DVD-5 (435,504 sectors to
spare); an image does not record the pressed disc's layers.

`SYSTEM.CNF`, 57 bytes:

```
BOOT2 = cdrom0:\SLUS_205.63;1
VER = 1.00
VMODE = NTSC
```

## Files, in disc order

96 files, packed with no gaps from LBA 386 to LBA 1859588. Sizes in bytes.
The last column names the other discs that hold the same bytes (SHA-1,
`tools/iso.py sums`), with the path where it is not the same. The order is
not Infection's: the modules, then the voices, then `DATA`, the movies and
the streams.

```
     lba  sectors         size  path                          same bytes as
     386        1           57  SYSTEM.CNF                    -
     387     1294      2649808  SLUS_205.63                   -
    1681      124       253201  IOPRP243.IMG                  INF, MUT, QUA
    1805       13        25397  MODULES/LIBSD.IRX             INF, MUT, QUA
    1818       45        90805  MODULES/MCMAN.IRX             INF, MUT, QUA
    1863        4         7353  MODULES/MCSERV.IRX            INF, MUT, QUA
    1867       30        59509  MODULES/MODHSYN.IRX           INF, MUT, QUA
    1897       11        21101  MODULES/MODMIDI.IRX           INF, MUT, QUA
    1908       22        44917  MODULES/PADMAN.IRX            INF, MUT, QUA
    1930        4         6977  MODULES/SDRDRV.IRX            INF, MUT, QUA
    1934        4         6553  MODULES/SIO2MAN.IRX           INF, MUT, QUA
    1938       21        42192  MODULES/SEWORDS.IRX           QUA
    1959       15        29616  MODULES/SNDBASE.IRX           QUA
    1974    62463    127924224  VOICE/BGM.BIN                 QUA
   64437     1429      2926592  VOICE/BOSSTALK.BIN            -
   65866      648      1327104  VOICE/FOOD.BIN                INF, MUT, QUA
   66514     4586      9392128  VOICE/FOUNTAIN.BIN            INF, MUT, QUA
   71100     3713      7604224  VOICE/INU.BIN                 INF, MUT, QUA
   74813     8852     18128896  VOICE/PARTY.BIN               MUT, QUA
   83665    49624    101629952  VOICE/PGUSO.BIN               INF, MUT, QUA
  133289    18056     36978688  VOICE/PRESENT.BIN             MUT, QUA
  151345     4890     10014720  VOICE/SPCTALK.BIN             MUT, QUA
  156235     1331      2725888  VOICE_E/BSTALK_E.BIN          -
  157566    16293     33368064  VOICE/EVVOL1S.BIN             INF, MUT, QUA
  173859    30821     63121408  VOICE/EVVOL2S.BIN             MUT, QUA
  204680    54061    110716928  VOICE/EVVOL3S.BIN             QUA
  258741    53227    109008896  VOICE/EVVOL3.BIN              -
  311968    48041     98387968  VOICE_E/EVVOL3_E.BIN          -
  360009    47809     97912832  VOICE_E/EVVOL3SE.BIN          QUA
  407818    15713     32180224  VOICE_E/EVVOL1SE.BIN          INF, MUT, QUA
  423531    28365     58091520  VOICE_E/EVVOL2SE.BIN          MUT, QUA
  451896      745      1525760  VOICE_E/FOOD_E.BIN            INF, MUT, QUA
  452641     4110      8417280  VOICE_E/FOUNT_E.BIN           INF, MUT, QUA
  456751     3580      7331840  VOICE_E/INU_E.BIN             INF, MUT, QUA
  460331     2281      4671488  VOICE_E/MIAE.BIN              MUT
  462612     8869     18163712  VOICE_E/PARTY_E.BIN           QUA
  471481    43211     88496128  VOICE_E/PGUSO_E.BIN           INF, MUT, QUA
  514692    17357     35547136  VOICE_E/PRESENTE.BIN          QUA
  532049     4533      9283584  VOICE_E/SPCTALKE.BIN          QUA
  536582     6971     14276608  VOICE2_E/SPC00_E.BIN          INF VOICE_E/SPC00_E.BIN, MUT VOICE_E/SPC00_E.BIN, QUA
  543553     7555     15472640  VOICE2_E/SPC01_E.BIN          INF VOICE_E/SPC01_E.BIN, MUT VOICE_E/SPC01_E.BIN, QUA
  551108     8257     16910336  VOICE2_E/SPC02_E.BIN          INF VOICE_E/SPC02_E.BIN, MUT VOICE_E/SPC02_E.BIN, QUA
  559365     9222     18886656  VOICE2_E/SPC03_E.BIN          INF VOICE_E/SPC03_E.BIN, MUT VOICE_E/SPC03_E.BIN, QUA
  568587     8576     17563648  VOICE2_E/SPC04_E.BIN          INF VOICE_E/SPC04_E.BIN, MUT VOICE_E/SPC04_E.BIN, QUA
  577163     8011     16406528  VOICE2_E/SPC05_E.BIN          INF VOICE_E/SPC05_E.BIN, MUT VOICE_E/SPC05_E.BIN, QUA
  585174     7234     14815232  VOICE2_E/SPC06_E.BIN          INF VOICE_E/SPC06_E.BIN, MUT VOICE_E/SPC06_E.BIN, QUA
  592408     9244     18931712  VOICE2_E/SPC07_E.BIN          INF VOICE_E/SPC07_E.BIN, MUT VOICE_E/SPC07_E.BIN, QUA
  601652     7994     16371712  VOICE2_E/SPC08_E.BIN          INF VOICE_E/SPC08_E.BIN, MUT VOICE_E/SPC08_E.BIN, QUA
  609646     6762     13848576  VOICE2_E/SPC09_E.BIN          INF VOICE_E/SPC09_E.BIN, MUT VOICE_E/SPC09_E.BIN, QUA
  616408     5612     11493376  VOICE2_E/SPC10_E.BIN          INF VOICE_E/SPC10_E.BIN, MUT VOICE_E/SPC10_E.BIN, QUA
  622020     7272     14893056  VOICE2_E/SPC11_E.BIN          INF VOICE_E/SPC11_E.BIN, MUT VOICE_E/SPC11_E.BIN, QUA
  629292     7814     16003072  VOICE2_E/SPC12_E.BIN          INF VOICE_E/SPC12_E.BIN, MUT VOICE_E/SPC12_E.BIN, QUA
  637106     7456     15269888  VOICE2_E/SPC13_E.BIN          INF VOICE_E/SPC13_E.BIN, MUT VOICE_E/SPC13_E.BIN, QUA
  644562     6639     13596672  VOICE2_E/SPC14_E.BIN          INF VOICE_E/SPC14_E.BIN, MUT VOICE_E/SPC14_E.BIN, QUA
  651201     7648     15663104  VOICE2_E/SPC15_E.BIN          INF VOICE_E/SPC15_E.BIN, MUT VOICE_E/SPC15_E.BIN, QUA
  658849     7221     14788608  VOICE2_E/SPC16_E.BIN          INF VOICE_E/SPC16_E.BIN, MUT VOICE_E/SPC16_E.BIN, QUA
  666070     6181     12658688  VOICE2_E/SPC17_E.BIN          INF VOICE_E/SPC17_E.BIN, MUT VOICE_E/SPC17_E.BIN, QUA
  672251     6996     14327808  VOICE2/SPC00.BIN              INF VOICE/SPC00.BIN, MUT VOICE/SPC00.BIN, QUA
  679247     7863     16103424  VOICE2/SPC01.BIN              INF VOICE/SPC01.BIN, MUT VOICE/SPC01.BIN, QUA
  687110     8207     16807936  VOICE2/SPC02.BIN              INF VOICE/SPC02.BIN, MUT VOICE/SPC02.BIN, QUA
  695317    10080     20643840  VOICE2/SPC03.BIN              INF VOICE/SPC03.BIN, MUT VOICE/SPC03.BIN, QUA
  705397     9845     20162560  VOICE2/SPC04.BIN              INF VOICE/SPC04.BIN, MUT VOICE/SPC04.BIN, QUA
  715242     8254     16904192  VOICE2/SPC05.BIN              INF VOICE/SPC05.BIN, MUT VOICE/SPC05.BIN, QUA
  723496     7241     14829568  VOICE2/SPC06.BIN              INF VOICE/SPC06.BIN, MUT VOICE/SPC06.BIN, QUA
  730737     8501     17410048  VOICE2/SPC07.BIN              INF VOICE/SPC07.BIN, MUT VOICE/SPC07.BIN, QUA
  739238     8867     18159616  VOICE2/SPC08.BIN              INF VOICE/SPC08.BIN, MUT VOICE/SPC08.BIN, QUA
  748105     7104     14548992  VOICE2/SPC09.BIN              INF VOICE/SPC09.BIN, MUT VOICE/SPC09.BIN, QUA
  755209     5905     12093440  VOICE2/SPC10.BIN              INF VOICE/SPC10.BIN, MUT VOICE/SPC10.BIN, QUA
  761114     7137     14616576  VOICE2/SPC11.BIN              INF VOICE/SPC11.BIN, MUT VOICE/SPC11.BIN, QUA
  768251     8361     17123328  VOICE2/SPC12.BIN              INF VOICE/SPC12.BIN, MUT VOICE/SPC12.BIN, QUA
  776612     7784     15941632  VOICE2/SPC13.BIN              INF VOICE/SPC13.BIN, MUT VOICE/SPC13.BIN, QUA
  784396     6901     14133248  VOICE2/SPC14.BIN              INF VOICE/SPC14.BIN, MUT VOICE/SPC14.BIN, QUA
  791297     7329     15009792  VOICE2/SPC15.BIN              INF VOICE/SPC15.BIN, MUT VOICE/SPC15.BIN, QUA
  798626     7558     15478784  VOICE2/SPC16.BIN              INF VOICE/SPC16.BIN, MUT VOICE/SPC16.BIN, QUA
  806184     6279     12859392  VOICE2/SPC17.BIN              INF VOICE/SPC17.BIN, MUT VOICE/SPC17.BIN, QUA
  812463      102       208896  VOICE2/SPC18.BIN              QUA
  812565      105       215040  VOICE2/SPC19.BIN              QUA
  812670       89       182272  VOICE2/SPC20.BIN              QUA
  812759       30        60544  DATA/DEMO.PRG                 -
  812789      235       480256  DATA/DESKTOP.PRG              -
  813024     1629      3335680  DATA/GCMN.PRG                 -
  814653       91       185472  DATA/TOPPAGE.PRG              -
  814744    69518    142372864  DATA/DATA.BIN                 -
  884262    20379     41736192  DATA/SNDDATA.BIN              -
  904641      504      1032192  DATA/ICON.BIN                 INF, MUT, QUA
  905145     1577      3227652  PSS/LOGO_B.PSS                INF, MUT, QUA
  906722     1953      3997700  PSS/LOGO_C.PSS                INF, MUT, QUA
  908675     1585      3244036  PSS/LOGO_H.PSS                INF, MUT, QUA
  910260    35857     73433092  PSS/OPENING3.PSS              -
  946117   173191    354695168  STREAM/STR1E.BIN              QUA
 1119308   208450    426905600  STREAM/STR2E.BIN              QUA
 1327758   315935    647034880  STREAM/STR3E.BIN              -
 1643693    55959    114604032  STREAM/STRCMNE.BIN            QUA
 1699652   157817    323209216  STREAM/STRSUBE.BIN            -
 1857469     1608      3293184  STREAM/STRT.BIN               -
 1859077      512      1048576  OUTSIDE.BIN                   INF, MUT, QUA
```

## What each is

| file | what | page |
| --- | --- | --- |
| `SLUS_205.63` | the EE executable, stripped and recompiled; Infection's names carried onto it (`SLUS_205.63.syms`) | [the four discs](volumes.md#names-for-the-stripped-executables) |
| `IOPRP243.IMG` | the IOP replacement image, the same on every disc | [the Infection DVD](layout.md#the-iop-image) |
| `MODULES/*.IRX` | the ten IOP modules the game loads | [the Infection DVD](layout.md#the-iop-image) |
| `DATA/*.PRG` | the four overlays | [overlays](../formats/prg.md) |
| `DATA/DATA.BIN` | the archive, 1,072 records | [the DATA.BIN archive](../formats/data-bin.md), [the four discs](volumes.md#databin-across-volumes) |
| `DATA/SNDDATA.BIN` | sound banks | [sound banks](../formats/snddata.md) |
| `DATA/ICON.BIN` | the save icons of all four volumes | [saves](../formats/save.md#files) |
| `VOICE2/SPC00`-`SPC17`, `VOICE2_E/SPC00_E`-`SPC17_E` | the party's skill words (`spcVoiceData`): Infection's `VOICE/SPC*` files, moved | [VOICE2](#voice2) |
| `VOICE2/SPC18`-`SPC20` | Tsukasa's, Subaru's and Sora's skill words in Japanese: one or two clips each | [VOICE2](#voice2) |
| `VOICE/EVVOL1S`, `EVVOL2S`, `VOICE_E/EVVOL1SE`, `EVVOL2SE` | volumes 1 and 2's side events' lines | [voice](../formats/voice.md) |
| `VOICE/EVVOL3`, `EVVOL3S`, `VOICE_E/EVVOL3_E`, `EVVOL3SE` | volume 3's main and side events' lines | [voice](../formats/voice.md) |
| `VOICE/BOSSTALK`, `VOICE_E/BSTALK_E` | the boss talk group (19): Fidchell's lines | [sound](../engine/sound.md) |
| `VOICE_E/MIAE.BIN` | Mia's English field lines after she has been talked to, Mutation's file | [sound](../engine/sound.md) |
| `VOICE/PARTY`, `PRESENT`, `SPCTALK`, `PGUSO`, `FOOD`, `FOUNTAIN`, `INU` and twins | the party's field lines, presents, talk; Grunties, food, the fountain, the dog | [sound](../engine/sound.md) |
| `VOICE/BGM.BIN` | streamed music | [voice](../formats/voice.md#streamed-music-voicebgmbin) |
| `STREAM/STR1E`-`STR3E`, `STRCMNE`, `STRSUBE` | the in-engine streams, English only | [streams](../engine/stream.md) |
| `STREAM/STRT.BIN` | six scenes no code opens: demo discs' titles, a teaser, a four-part selector | [STRT.BIN](#strtbin) |
| `PSS/LOGO_*.PSS` | the three logo movies, Infection's | [PSS](../formats/pss.md) |
| `PSS/OPENING3.PSS` | the opening movie | [PSS](../formats/pss.md) |
| `OUTSIDE.BIN` | 1,048,576 zero bytes; nothing names it | [the Infection DVD](layout.md#notes) |

## VOICE2

The characters' skill words (`spcVoiceData`, what `skillVoicePlay` plays
when a character uses a skill) moved from `VOICE` to `VOICE2`.
- `SPC00`-`SPC17` and `SPC00_E`-`SPC17_E` are Infection's and Mutation's
  `VOICE/SPC*` files byte for byte.
- `SPC18`-`SPC20` are the three characters added in the character table
  (`charTbl` rows 18-20): Tsukasa, Subaru and Sora. Each Japanese file
  holds two clips of real audio: `SPC18`'s two are the same, and `SPC19`'s
  and `SPC20`'s differ.
- The English files for 18-20 are not on this disc. The skill voice table
  (`SKILL_21` in `piney_data::tables::voice`, the same on Quarantine) still
  names `VOICE2_E/SPC18_E.BIN` to `SPC20_E.BIN`, so those three are silent
  in English here. Quarantine carries them.

## STRT.BIN

3,293,184 bytes: six gzip members, each named `tmp.ccs` in its gzip header,
each a CCSF file:

| offset | CCSF name | gzip time (UTC) | inflated | objects |
| --- | --- | --- | ---: | --- |
| 0x0 | `title1_st1t` | 2002-10-01 06:28 | 1,011,896 | the title's desktop models (`xdt_bac_`, `xdt_tit_`, `xdt_ico_` ...), `camera10`, `omni00` |
| 0x17000 | `title1_st1t2` | 2002-10-11 00:54 | 1,011,896 | the same set |
| 0x2e000 | `title2_st1` | 2003-03-04 13:57 | 1,102,432 | the same set, `omni01` |
| 0x46000 | `str9999e` | 2003-03-13 11:02 | 2,382,212 | a setup file: `back01`, `ring01`, `plane01`, `hack`, `coming` and their walls |
| 0x94000 | `str9999` | 2003-03-13 12:55 | 7,703,772 | its scene: `camera01`, `plane01`-`06`, `box01`, `box02` |
| 0x2ff800 | `trial_v2st` | 2003-03-13 12:44 | 579,360 | `trilog00`-`10`, the title's `xdt_` models |

No code opens the file:
- `ccCdInit` looks up `STRCMNE`, `STR1E`, `STR2E`, `STRSUBE` and `STR3E`
  only;
- no string in the executable or any overlay contains `STRT`;
- no stream table names `title1_st1t`, `str9999` or `trial_v2st`.

`title2_st1` is a title stream name the game does use, but that one is in
`STRCMNE.BIN` at another size. The dates fall within the game's making
(members sampled from `STRCMNE.BIN` and `STR3E.BIN` date from 2002-08 to
2003-02).

Played (`piney-game --mode loose:STREAM/STRT.BIN:N`, below), they are the
demo discs' own screens:

| stream | scenes | frames, with the restart | what it shows |
| ---: | --- | ---: | --- |
| 0 | `title1_st1t` | 413 | the `.hack//INFECTION DEMO` title (感染拡大) on the orange hexagon backdrop, no menu |
| 1 | `title1_st1t2` | 413 | the same title with its menu: NEWGAME, DATALOAD, OPTION |
| 2 | `title2_st1` | 453 | a `.hack//MUTATION DEMO` title (悪性変異) on a blue backdrop, its menu NEWGAME, DATALOAD, OPTION, CONVERT |
| 3 | `str9999e`, `str9999` | 553 | a teaser: rings, the `.hack//INFECTION` logo, "February 2003", `www.dothack.com` and the Bandai logo over Aura |
| 4 | `trial_v2st` | 453 | a selector over the party's art: `.hack//INFECTION Part 1`, `MUTATION Part 2`, `OUTBREAK Part 3`, `QUARANTINE Part 4` |

The menus are the scenes' own models: they fade in as the retail title's
do, and nothing answers them. "February 2003" is Infection's American
release; the Mutation title and the four-part selector point at a demo
disc for the series. They are not the retail volumes' title streams,
which live in `STRCMNE.BIN` (inference from their names and what they
show).

**The selector's objects.** `trial_v2st` holds two copies of most of its
objects: one under `d\trial\anm\tristr00.max` that the frames pose (the
four rows `OBJ_xdt_men_X1_` at x -7500, y 9000, 4500, 0 and -4500, their
icons `ico_X0`/`X1`, the shadows `sha`, the moving hexagons `dot`, the
backgrounds `bac_00`/`05`), and a still set from `trilogo0.max` at the
origin that no frame record touches and that stays at transparency 0.
The still set has what a menu over the rows would need:
- per row X, a highlight `OBJ_xdt_men_X0_`: the row's text in a model
  starting at its origin (x -4795.3 to 19800, where the posed row's is
  centred, -12300 to 12300), with two glows as children, `lig_X0` (a
  round one, `trilog05`) and `lig_X1` (a bar, `trilog06`);
- the labels `mes_X0` (and `mes_21`, `mes_31`) on `trilog09`: PLAYABLE
  DEMO, TRAILER, COMING SOON;
- a third icon for rows 2 and 3 (`ico_22`, `ico_32`), and a second
  background (`bac_01`).

The code that placed them is on no disc. The port's launcher
(`crates/piney-game/src/launcher.rs`) puts row X's highlight on the posed
row, 7,504.7 units left of its origin so the two texts line up.

**The viewer.** Not the game's: `--mode loose:PATH[:N]` reads an archive
no stream table lists (`piney_stream::load::scan`: a gzip member on each
sector boundary, named by its CCSF header) and makes its streams
(`loose_streams`). A `strNNNNe` setup file (`type` 0, `flag` 16) goes with
the scene named by its stem; every other member is a scene alone. It plays
stream N with `Stream::loose` at 30 frames a second, again when it ends;
START (Enter) goes on to the next. With `--shot`, `--webp OUT.webp`
writes the frames as an animated WebP (one loop of stream 2 is
`--frames 453`, about 8 MB at the default quality).

## Notes

**Against Mutation's disc.**
- New: `VOICE2`, `VOICE2_E`; `VOICE/EVVOL3*`, `VOICE_E/EVVOL3*`,
  `VOICE/BOSSTALK.BIN`, `VOICE_E/BSTALK_E.BIN`; `STREAM/STR3E.BIN`,
  `STRT.BIN`; `PSS/OPENING3.PSS`.
- Gone: the Japanese stream archives (`STR1`, `STR2`, `STRCMN`, `STRSUB`);
  `VOICE/EVVOL2.BIN` and `VOICE_E/EVVOL2_E.BIN`, volume 2's main events'
  lines; the unused `MODULES/CDVDFSV.IRX` and `CDVDMAN.IRX`; `VOICE/SPC*`
  (now `VOICE2`).
- `SNDBASE.IRX` (29,616 bytes) and `SEWORDS.IRX` (42,192) are rebuilt again,
  the same as Quarantine's.
- `STR1E.BIN`, `STR2E.BIN` and `STRCMNE.BIN` are Quarantine's bytes;
  `STR3E.BIN` (647,034,880) and `STRSUBE.BIN` are not. 23 of `STR3E`'s
  scenes carry two Pcm tracks here and one on Quarantine ([the Quarantine
  DVD](quarantine.md#notes)).

## Unknown

None: every file is accounted for above.
