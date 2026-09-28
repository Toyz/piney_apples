---
title: The Quarantine DVD
status: solid
volumes: QUA
covers: QUA SYSTEM.CNF, every file on the disc, QUA DATA/KFED.BIN, QUA DATA/KFAED.BIN, QUA SLUS_205.64:0x001c52e8 event opcode 169, QUA gcmn.prg:0x004f0a30 the staff roll's start, 0x004f0950 its thread
worklog: 239
---

# The Quarantine DVD

What is on the .hack//Quarantine (USA) disc, where, and which page
describes each file. It is measured against [the Outbreak DVD](outbreak.md);
what the four discs share is in [the four discs](volumes.md).

## Volume

ISO 9660 only - no Joliet, no Rock Ridge. 2048-byte sectors, 2,176,912 of
them (4,458,315,776 bytes). System identifier `PLAYSTATION`, volume
identifier empty. The size fits a single-layer DVD-5 (118,192 sectors to
spare); an image does not record the pressed disc's layers.

`SYSTEM.CNF`, 57 bytes:

```
BOOT2 = cdrom0:\SLUS_205.64;1
VER = 1.00
VMODE = NTSC
```

## Files, in disc order

100 files, packed with no gaps from LBA 390 to LBA 2176898. Sizes in bytes.
The last column names the other discs that hold the same bytes (SHA-1,
`tools/iso.py sums`), with the path where it is not the same. The order is
Outbreak's, with the streams from the newest volume back.

```
     lba  sectors         size  path                          same bytes as
     390        1           57  SYSTEM.CNF                    -
     391      755      1546192  SLUS_205.64                   -
    1146      124       253201  IOPRP243.IMG                  INF, MUT, OUT
    1270       13        25397  MODULES/LIBSD.IRX             INF, MUT, OUT
    1283       45        90805  MODULES/MCMAN.IRX             INF, MUT, OUT
    1328        4         7353  MODULES/MCSERV.IRX            INF, MUT, OUT
    1332       30        59509  MODULES/MODHSYN.IRX           INF, MUT, OUT
    1362       11        21101  MODULES/MODMIDI.IRX           INF, MUT, OUT
    1373       22        44917  MODULES/PADMAN.IRX            INF, MUT, OUT
    1395        4         6977  MODULES/SDRDRV.IRX            INF, MUT, OUT
    1399        4         6553  MODULES/SIO2MAN.IRX           INF, MUT, OUT
    1403       21        42192  MODULES/SEWORDS.IRX           OUT
    1424       15        29616  MODULES/SNDBASE.IRX           OUT
    1439    62463    127924224  VOICE/BGM.BIN                 OUT
   63902      648      1327104  VOICE/FOOD.BIN                INF, MUT, OUT
   64550     4586      9392128  VOICE/FOUNTAIN.BIN            INF, MUT, OUT
   69136     3713      7604224  VOICE/INU.BIN                 INF, MUT, OUT
   72849     8852     18128896  VOICE/PARTY.BIN               MUT, OUT
   81701    49624    101629952  VOICE/PGUSO.BIN               INF, MUT, OUT
  131325    18056     36978688  VOICE/PRESENT.BIN             MUT, OUT
  149381     4890     10014720  VOICE/SPCTALK.BIN             MUT, OUT
  154271    16293     33368064  VOICE/EVVOL1S.BIN             INF, MUT, OUT
  170564    30821     63121408  VOICE/EVVOL2S.BIN             MUT, OUT
  201385    54061    110716928  VOICE/EVVOL3S.BIN             OUT
  255446    24145     49448960  VOICE/EVVOL4S.BIN             -
  279591    44033     90179584  VOICE/EVVOL4.BIN              -
  323624    15713     32180224  VOICE_E/EVVOL1SE.BIN          INF, MUT, OUT
  339337    28365     58091520  VOICE_E/EVVOL2SE.BIN          MUT, OUT
  367702    47809     97912832  VOICE_E/EVVOL3SE.BIN          OUT
  415511    23517     48162816  VOICE_E/EVVOL4SE.BIN          -
  439028    43329     88737792  VOICE_E/EVVOL4_E.BIN          -
  482357      745      1525760  VOICE_E/FOOD_E.BIN            INF, MUT, OUT
  483102     4110      8417280  VOICE_E/FOUNT_E.BIN           INF, MUT, OUT
  487212     3580      7331840  VOICE_E/INU_E.BIN             INF, MUT, OUT
  490792     8869     18163712  VOICE_E/PARTY_E.BIN           OUT
  499661    43211     88496128  VOICE_E/PGUSO_E.BIN           INF, MUT, OUT
  542872    17357     35547136  VOICE_E/PRESENTE.BIN          OUT
  560229     4533      9283584  VOICE_E/SPCTALKE.BIN          OUT
  564762     6971     14276608  VOICE2_E/SPC00_E.BIN          INF VOICE_E/SPC00_E.BIN, MUT VOICE_E/SPC00_E.BIN, OUT
  571733     7555     15472640  VOICE2_E/SPC01_E.BIN          INF VOICE_E/SPC01_E.BIN, MUT VOICE_E/SPC01_E.BIN, OUT
  579288     8257     16910336  VOICE2_E/SPC02_E.BIN          INF VOICE_E/SPC02_E.BIN, MUT VOICE_E/SPC02_E.BIN, OUT
  587545     9222     18886656  VOICE2_E/SPC03_E.BIN          INF VOICE_E/SPC03_E.BIN, MUT VOICE_E/SPC03_E.BIN, OUT
  596767     8576     17563648  VOICE2_E/SPC04_E.BIN          INF VOICE_E/SPC04_E.BIN, MUT VOICE_E/SPC04_E.BIN, OUT
  605343     8011     16406528  VOICE2_E/SPC05_E.BIN          INF VOICE_E/SPC05_E.BIN, MUT VOICE_E/SPC05_E.BIN, OUT
  613354     7234     14815232  VOICE2_E/SPC06_E.BIN          INF VOICE_E/SPC06_E.BIN, MUT VOICE_E/SPC06_E.BIN, OUT
  620588     9244     18931712  VOICE2_E/SPC07_E.BIN          INF VOICE_E/SPC07_E.BIN, MUT VOICE_E/SPC07_E.BIN, OUT
  629832     7994     16371712  VOICE2_E/SPC08_E.BIN          INF VOICE_E/SPC08_E.BIN, MUT VOICE_E/SPC08_E.BIN, OUT
  637826     6762     13848576  VOICE2_E/SPC09_E.BIN          INF VOICE_E/SPC09_E.BIN, MUT VOICE_E/SPC09_E.BIN, OUT
  644588     5612     11493376  VOICE2_E/SPC10_E.BIN          INF VOICE_E/SPC10_E.BIN, MUT VOICE_E/SPC10_E.BIN, OUT
  650200     7272     14893056  VOICE2_E/SPC11_E.BIN          INF VOICE_E/SPC11_E.BIN, MUT VOICE_E/SPC11_E.BIN, OUT
  657472     7814     16003072  VOICE2_E/SPC12_E.BIN          INF VOICE_E/SPC12_E.BIN, MUT VOICE_E/SPC12_E.BIN, OUT
  665286     7456     15269888  VOICE2_E/SPC13_E.BIN          INF VOICE_E/SPC13_E.BIN, MUT VOICE_E/SPC13_E.BIN, OUT
  672742     6639     13596672  VOICE2_E/SPC14_E.BIN          INF VOICE_E/SPC14_E.BIN, MUT VOICE_E/SPC14_E.BIN, OUT
  679381     7648     15663104  VOICE2_E/SPC15_E.BIN          INF VOICE_E/SPC15_E.BIN, MUT VOICE_E/SPC15_E.BIN, OUT
  687029     7221     14788608  VOICE2_E/SPC16_E.BIN          INF VOICE_E/SPC16_E.BIN, MUT VOICE_E/SPC16_E.BIN, OUT
  694250     6181     12658688  VOICE2_E/SPC17_E.BIN          INF VOICE_E/SPC17_E.BIN, MUT VOICE_E/SPC17_E.BIN, OUT
  700431     6396     13099008  VOICE2_E/SPC18_E.BIN          -
  706827     7283     14915584  VOICE2_E/SPC19_E.BIN          -
  714110     8434     17272832  VOICE2_E/SPC20_E.BIN          -
  722544     6996     14327808  VOICE2/SPC00.BIN              INF VOICE/SPC00.BIN, MUT VOICE/SPC00.BIN, OUT
  729540     7863     16103424  VOICE2/SPC01.BIN              INF VOICE/SPC01.BIN, MUT VOICE/SPC01.BIN, OUT
  737403     8207     16807936  VOICE2/SPC02.BIN              INF VOICE/SPC02.BIN, MUT VOICE/SPC02.BIN, OUT
  745610    10080     20643840  VOICE2/SPC03.BIN              INF VOICE/SPC03.BIN, MUT VOICE/SPC03.BIN, OUT
  755690     9845     20162560  VOICE2/SPC04.BIN              INF VOICE/SPC04.BIN, MUT VOICE/SPC04.BIN, OUT
  765535     8254     16904192  VOICE2/SPC05.BIN              INF VOICE/SPC05.BIN, MUT VOICE/SPC05.BIN, OUT
  773789     7241     14829568  VOICE2/SPC06.BIN              INF VOICE/SPC06.BIN, MUT VOICE/SPC06.BIN, OUT
  781030     8501     17410048  VOICE2/SPC07.BIN              INF VOICE/SPC07.BIN, MUT VOICE/SPC07.BIN, OUT
  789531     8867     18159616  VOICE2/SPC08.BIN              INF VOICE/SPC08.BIN, MUT VOICE/SPC08.BIN, OUT
  798398     7104     14548992  VOICE2/SPC09.BIN              INF VOICE/SPC09.BIN, MUT VOICE/SPC09.BIN, OUT
  805502     5905     12093440  VOICE2/SPC10.BIN              INF VOICE/SPC10.BIN, MUT VOICE/SPC10.BIN, OUT
  811407     7137     14616576  VOICE2/SPC11.BIN              INF VOICE/SPC11.BIN, MUT VOICE/SPC11.BIN, OUT
  818544     8361     17123328  VOICE2/SPC12.BIN              INF VOICE/SPC12.BIN, MUT VOICE/SPC12.BIN, OUT
  826905     7784     15941632  VOICE2/SPC13.BIN              INF VOICE/SPC13.BIN, MUT VOICE/SPC13.BIN, OUT
  834689     6901     14133248  VOICE2/SPC14.BIN              INF VOICE/SPC14.BIN, MUT VOICE/SPC14.BIN, OUT
  841590     7329     15009792  VOICE2/SPC15.BIN              INF VOICE/SPC15.BIN, MUT VOICE/SPC15.BIN, OUT
  848919     7558     15478784  VOICE2/SPC16.BIN              INF VOICE/SPC16.BIN, MUT VOICE/SPC16.BIN, OUT
  856477     6279     12859392  VOICE2/SPC17.BIN              INF VOICE/SPC17.BIN, MUT VOICE/SPC17.BIN, OUT
  862756      102       208896  VOICE2/SPC18.BIN              OUT
  862858      105       215040  VOICE2/SPC19.BIN              OUT
  862963       89       182272  VOICE2/SPC20.BIN              OUT
  863052       30        60544  DATA/DEMO.PRG                 -
  863082      226       460928  DATA/DESKTOP.PRG              -
  863308     1650      3377280  DATA/GCMN.PRG                 -
  864958       91       185600  DATA/TOPPAGE.PRG              -
  865049    20386     41750528  DATA/SNDDATA.BIN              -
  885435    69518    142372864  DATA/DATA.BIN                 -
  954953       18        35200  DATA/KFAED.BIN                -
  954971      336       688000  DATA/KFED.BIN                 -
  955307      504      1032192  DATA/ICON.BIN                 INF, MUT, OUT
  955811     1577      3227652  PSS/LOGO_B.PSS                INF, MUT, OUT
  957388     1953      3997700  PSS/LOGO_C.PSS                INF, MUT, OUT
  959341     1585      3244036  PSS/LOGO_H.PSS                INF, MUT, OUT
  960926    36985     75743236  PSS/OPENING4.PSS              -
  997911   379185    776570880  STREAM/STR4E.BIN              -
 1377096    55959    114604032  STREAM/STRCMNE.BIN            OUT
 1433055   157819    323213312  STREAM/STRSUBE.BIN            -
 1590874   203872    417529856  STREAM/STR3E.BIN              -
 1794746   208450    426905600  STREAM/STR2E.BIN              OUT
 2003196   173191    354695168  STREAM/STR1E.BIN              OUT
 2176387      512      1048576  OUTSIDE.BIN                   INF, MUT, OUT
```

## What each is

| file | what | page |
| --- | --- | --- |
| `SLUS_205.64` | the EE executable, stripped and recompiled; Infection's names carried onto it (`SLUS_205.64.syms`) | [the four discs](volumes.md#names-for-the-stripped-executables) |
| `IOPRP243.IMG` | the IOP replacement image, the same on every disc | [the Infection DVD](layout.md#the-iop-image) |
| `MODULES/*.IRX` | the ten IOP modules the game loads, Outbreak's | [the Infection DVD](layout.md#the-iop-image) |
| `DATA/*.PRG` | the four overlays | [overlays](../formats/prg.md) |
| `DATA/DATA.BIN` | the archive, 1,072 records (Outbreak's list; five members differ) | [the four discs](volumes.md#databin-across-volumes) |
| `DATA/SNDDATA.BIN` | sound banks | [sound banks](../formats/snddata.md) |
| `DATA/KFED.BIN`, `KFAED.BIN` | kanji font sheets, 20x20: loaded for the staff roll and never read | [KFED.BIN and KFAED.BIN](#kfedbin-and-kfaedbin) |
| `DATA/ICON.BIN` | the save icons of all four volumes | [saves](../formats/save.md#files) |
| `VOICE2/SPC00`-`SPC20`, `VOICE2_E/SPC00_E`-`SPC20_E` | the characters' skill words, Tsukasa's, Subaru's and Sora's English ones (18-20) new here | [the Outbreak DVD](outbreak.md#voice2) |
| `VOICE/EVVOL1S`-`EVVOL3S`, `VOICE_E/EVVOL1SE`-`EVVOL3SE` | volumes 1 to 3's side events' lines | [voice](../formats/voice.md) |
| `VOICE/EVVOL4`, `EVVOL4S`, `VOICE_E/EVVOL4_E`, `EVVOL4SE` | volume 4's main and side events' lines | [voice](../formats/voice.md) |
| `VOICE/PARTY`, `PRESENT`, `SPCTALK`, `PGUSO`, `FOOD`, `FOUNTAIN`, `INU` and twins | the party's field lines, presents, talk; Grunties, food, the fountain, the dog | [sound](../engine/sound.md) |
| `VOICE/BGM.BIN` | streamed music, Outbreak's | [voice](../formats/voice.md#streamed-music-voicebgmbin) |
| `STREAM/STR1E`-`STR4E`, `STRCMNE`, `STRSUBE` | the in-engine streams, English only | [streams](../engine/stream.md) |
| `PSS/LOGO_*.PSS` | the three logo movies, Infection's | [PSS](../formats/pss.md) |
| `PSS/OPENING4.PSS` | the opening movie | [PSS](../formats/pss.md) |
| `OUTSIDE.BIN` | 1,048,576 zero bytes; nothing names it | [the Infection DVD](layout.md#notes) |

## KFED.BIN and KFAED.BIN

688,000 and 35,200 bytes: kanji sheets in `kf20x20`'s and `kfa20x20`'s
layout, which Infection to Outbreak carry in the executable unreferenced
([text rendering](../engine/font.md#the-kanji-fonts)).

They are read once, by event opcode 169 (`0x001c52e8`), which only
Quarantine's executable has, in event 314 "Ending", block 3
(`ending_kanji`):
1. `game+0x7c = 1`.
2. `\DATA\KFED.BIN` and `\DATA\KFAED.BIN` are read whole into two `$gp`
   globals (`-29188` and `-29184`, `0x0027956c` and `0x00279570`).
3. `ccLoadFLAdd` loads the file list at `0x00219340`: `ending.ccs`.
4. gcmn `0x004f0a30` starts the thread at `0x004f0950` (priority 66, stack
   0x1000) and breathes until its tscb's +0x14 is set. The thread is
   `STFROLL_VOL4`, Quarantine's staff roll. It draws with `ccKanji`
   (`Init`), sprites of `TEX_hackrogo` and `TEX_hackrogo_2`, particle
   generators, and the `ending` animations.
5. Both buffers are freed.

Nothing reads the two globals. Every instruction in the executable and the
four overlays that addresses `$gp` -29188 or -29184 is one of the handler's
two stores and two loads for `ccFree`. No instruction builds either
address, and no word holds it. The staff roll's text goes through
`ccKanji`, whose fonts are the executable's `ef12x20` and `ef8x16`. So the
US build loads 723,200 bytes of kanji and frees them unread, presumably
the Japanese build's staff roll's font (inference).

## Notes

**Against Outbreak's disc.**
- New: `VOICE/EVVOL4*`, `VOICE_E/EVVOL4*`; `VOICE2_E/SPC18_E.BIN` to
  `SPC20_E.BIN`; `DATA/KFED.BIN`, `KFAED.BIN`; `STREAM/STR4E.BIN`;
  `PSS/OPENING4.PSS`.
- Gone: `VOICE/EVVOL3.BIN` and `VOICE_E/EVVOL3_E.BIN`, volume 3's main
  events' lines; `VOICE/BOSSTALK.BIN` and `VOICE_E/BSTALK_E.BIN`;
  `VOICE_E/MIAE.BIN`; `STREAM/STRT.BIN`.
- The voice tables still name the gone files: the file list (`FILES_E` in
  `piney_data::tables::voice`, shared with Outbreak) has `BSTALK_E.BIN` and
  `MIAE.BIN`, and Fidchell's group (19) reads `BOSSTALK.BIN`. On this disc
  those lines would not open.
- `STR3E.BIN` is 417,529,856 bytes here and 647,034,880 on Outbreak. Both
  hold the same 48 scenes (47 names); 24 are the same bytes. Each of the
  other 23 (`str1195` to `str1620`, `str9402` to `str9505`) has the same
  frames, but a single [Pcm track](../formats/voice.md#cutscene-audio-ccsf-pcm-chunks)
  where Outbreak's has two. Outbreak's two setup chunks (`0x2200`) are
  alike (`trackType` 1) but for the byte at +0x07, 1 in the first and 0 in
  the second. That byte is padding in `CCSTRM_PCM` (DWARF, `libccs2.cpp`),
  which nothing reads. Each frame carries an `F_Pcm` for each track;
  Quarantine keeps the first. `str1500`, for one, inflates to
  133,999,132 bytes on Outbreak and 99,946,148 here, with 5,302 frames in
  both.
- `STRSUBE.BIN` differs from Outbreak's by 4,096 bytes. `STR1E`, `STR2E`
  and `STRCMNE` are Outbreak's bytes.
- The executable is 1,546,192 bytes, 1,103,616 less than Outbreak's.
  Outbreak's carries all six bitmap fonts (1,148,800 bytes); this one keeps
  `ef12x20` and `ef8x16` (20,608), drops `kf14x16` and `kfa14x16`, and ships
  `kf20x20` and `kfa20x20`'s sheets as `KFED.BIN` and `KFAED.BIN` ([the four
  discs](volumes.md#the-font-tables-in-quarantine)).

## Unknown

None: every file is accounted for above.
