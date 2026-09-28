---
title: The Mutation DVD
status: solid
volumes: MUT
covers: MUT SYSTEM.CNF, every file on the disc
worklog: 239
---

# The Mutation DVD

What is on the .hack//Mutation (USA) disc, where, and which page describes
each file. It is measured against [the Infection DVD](layout.md); what the
four discs share and how their code differs is in [the four
discs](volumes.md).

## Volume

ISO 9660 only - no Joliet, no Rock Ridge. 2048-byte sectors, 1,830,448 of
them (3,748,757,504 bytes). System identifier `PLAYSTATION`, volume
identifier empty. The size fits a single-layer DVD-5 (464,656 sectors to
spare); an image does not record the pressed disc's layers.

`SYSTEM.CNF`, 57 bytes:

```
BOOT2 = cdrom0:\SLUS_205.62;1
VER = 1.00
VMODE = NTSC
```

## Files, in disc order

93 files, packed with no gaps from LBA 377 to LBA 1830439. Sizes in bytes.
The last column names the other discs that hold the same bytes (SHA-1,
`tools/iso.py sums`), with the path where it is not the same.

```
     lba  sectors         size  path                          same bytes as
     377        1           57  SYSTEM.CNF                    -
     378     1304      2669392  SLUS_205.62                   -
    1682      124       253201  IOPRP243.IMG                  INF, OUT, QUA
    1806        9        16901  MODULES/CDVDFSV.IRX           INF
    1815       13        26069  MODULES/CDVDMAN.IRX           INF
    1828       11        21101  MODULES/MODMIDI.IRX           INF, OUT, QUA
    1839        4         6977  MODULES/SDRDRV.IRX            INF, OUT, QUA
    1843        4         6553  MODULES/SIO2MAN.IRX           INF, OUT, QUA
    1847       22        44917  MODULES/PADMAN.IRX            INF, OUT, QUA
    1869       30        59509  MODULES/MODHSYN.IRX           INF, OUT, QUA
    1899       13        25397  MODULES/LIBSD.IRX             INF, OUT, QUA
    1912       45        90805  MODULES/MCMAN.IRX             INF, OUT, QUA
    1957        4         7353  MODULES/MCSERV.IRX            INF, OUT, QUA
    1961       15        29744  MODULES/SNDBASE.IRX           -
    1976       21        42352  MODULES/SEWORDS.IRX           -
    1997       31        61952  DATA/DEMO.PRG                 -
    2028      231       471936  DATA/DESKTOP.PRG              -
    2259     1623      3322624  DATA/GCMN.PRG                 -
    3882       91       185216  DATA/TOPPAGE.PRG              -
    3973      504      1032192  DATA/ICON.BIN                 INF, OUT, QUA
    4477    67467    138172416  DATA/DATA.BIN                 -
   71944    20379     41736192  DATA/SNDDATA.BIN              -
   92323     6971     14276608  VOICE_E/SPC00_E.BIN           INF, OUT VOICE2_E/SPC00_E.BIN, QUA VOICE2_E/SPC00_E.BIN
   99294     7555     15472640  VOICE_E/SPC01_E.BIN           INF, OUT VOICE2_E/SPC01_E.BIN, QUA VOICE2_E/SPC01_E.BIN
  106849     8257     16910336  VOICE_E/SPC02_E.BIN           INF, OUT VOICE2_E/SPC02_E.BIN, QUA VOICE2_E/SPC02_E.BIN
  115106     9222     18886656  VOICE_E/SPC03_E.BIN           INF, OUT VOICE2_E/SPC03_E.BIN, QUA VOICE2_E/SPC03_E.BIN
  124328     8576     17563648  VOICE_E/SPC04_E.BIN           INF, OUT VOICE2_E/SPC04_E.BIN, QUA VOICE2_E/SPC04_E.BIN
  132904     8011     16406528  VOICE_E/SPC05_E.BIN           INF, OUT VOICE2_E/SPC05_E.BIN, QUA VOICE2_E/SPC05_E.BIN
  140915     7234     14815232  VOICE_E/SPC06_E.BIN           INF, OUT VOICE2_E/SPC06_E.BIN, QUA VOICE2_E/SPC06_E.BIN
  148149     9244     18931712  VOICE_E/SPC07_E.BIN           INF, OUT VOICE2_E/SPC07_E.BIN, QUA VOICE2_E/SPC07_E.BIN
  157393     7994     16371712  VOICE_E/SPC08_E.BIN           INF, OUT VOICE2_E/SPC08_E.BIN, QUA VOICE2_E/SPC08_E.BIN
  165387     6762     13848576  VOICE_E/SPC09_E.BIN           INF, OUT VOICE2_E/SPC09_E.BIN, QUA VOICE2_E/SPC09_E.BIN
  172149     5612     11493376  VOICE_E/SPC10_E.BIN           INF, OUT VOICE2_E/SPC10_E.BIN, QUA VOICE2_E/SPC10_E.BIN
  177761     7272     14893056  VOICE_E/SPC11_E.BIN           INF, OUT VOICE2_E/SPC11_E.BIN, QUA VOICE2_E/SPC11_E.BIN
  185033     7814     16003072  VOICE_E/SPC12_E.BIN           INF, OUT VOICE2_E/SPC12_E.BIN, QUA VOICE2_E/SPC12_E.BIN
  192847     7456     15269888  VOICE_E/SPC13_E.BIN           INF, OUT VOICE2_E/SPC13_E.BIN, QUA VOICE2_E/SPC13_E.BIN
  200303     6639     13596672  VOICE_E/SPC14_E.BIN           INF, OUT VOICE2_E/SPC14_E.BIN, QUA VOICE2_E/SPC14_E.BIN
  206942     7648     15663104  VOICE_E/SPC15_E.BIN           INF, OUT VOICE2_E/SPC15_E.BIN, QUA VOICE2_E/SPC15_E.BIN
  214590     7221     14788608  VOICE_E/SPC16_E.BIN           INF, OUT VOICE2_E/SPC16_E.BIN, QUA VOICE2_E/SPC16_E.BIN
  221811     6181     12658688  VOICE_E/SPC17_E.BIN           INF, OUT VOICE2_E/SPC17_E.BIN, QUA VOICE2_E/SPC17_E.BIN
  227992    43211     88496128  VOICE_E/PGUSO_E.BIN           INF, OUT, QUA
  271203      745      1525760  VOICE_E/FOOD_E.BIN            INF, OUT, QUA
  271948     4110      8417280  VOICE_E/FOUNT_E.BIN           INF, OUT, QUA
  276058     3580      7331840  VOICE_E/INU_E.BIN             INF, OUT, QUA
  279638    15641     32032768  VOICE_E/PRESENTE.BIN          INF
  295279     7596     15556608  VOICE_E/PARTY_E.BIN           -
  302875     3908      8003584  VOICE_E/SPCTALKE.BIN          -
  306783     2281      4671488  VOICE_E/MIAE.BIN              OUT
  309064    15713     32180224  VOICE_E/EVVOL1SE.BIN          INF, OUT, QUA
  324777    45052     92266496  VOICE_E/EVVOL2_E.BIN          -
  369829    28365     58091520  VOICE_E/EVVOL2SE.BIN          OUT, QUA
  398194    16293     33368064  VOICE/EVVOL1S.BIN             INF, OUT, QUA
  414487    50631    103692288  VOICE/EVVOL2.BIN              -
  465118    30821     63121408  VOICE/EVVOL2S.BIN             OUT, QUA
  495939     4586      9392128  VOICE/FOUNTAIN.BIN            INF, OUT, QUA
  500525     3713      7604224  VOICE/INU.BIN                 INF, OUT, QUA
  504238     6996     14327808  VOICE/SPC00.BIN               INF, OUT VOICE2/SPC00.BIN, QUA VOICE2/SPC00.BIN
  511234     7863     16103424  VOICE/SPC01.BIN               INF, OUT VOICE2/SPC01.BIN, QUA VOICE2/SPC01.BIN
  519097     8207     16807936  VOICE/SPC02.BIN               INF, OUT VOICE2/SPC02.BIN, QUA VOICE2/SPC02.BIN
  527304    10080     20643840  VOICE/SPC03.BIN               INF, OUT VOICE2/SPC03.BIN, QUA VOICE2/SPC03.BIN
  537384     9845     20162560  VOICE/SPC04.BIN               INF, OUT VOICE2/SPC04.BIN, QUA VOICE2/SPC04.BIN
  547229     8254     16904192  VOICE/SPC05.BIN               INF, OUT VOICE2/SPC05.BIN, QUA VOICE2/SPC05.BIN
  555483     7241     14829568  VOICE/SPC06.BIN               INF, OUT VOICE2/SPC06.BIN, QUA VOICE2/SPC06.BIN
  562724     8501     17410048  VOICE/SPC07.BIN               INF, OUT VOICE2/SPC07.BIN, QUA VOICE2/SPC07.BIN
  571225     8867     18159616  VOICE/SPC08.BIN               INF, OUT VOICE2/SPC08.BIN, QUA VOICE2/SPC08.BIN
  580092     7104     14548992  VOICE/SPC09.BIN               INF, OUT VOICE2/SPC09.BIN, QUA VOICE2/SPC09.BIN
  587196     5905     12093440  VOICE/SPC10.BIN               INF, OUT VOICE2/SPC10.BIN, QUA VOICE2/SPC10.BIN
  593101     7137     14616576  VOICE/SPC11.BIN               INF, OUT VOICE2/SPC11.BIN, QUA VOICE2/SPC11.BIN
  600238     8361     17123328  VOICE/SPC12.BIN               INF, OUT VOICE2/SPC12.BIN, QUA VOICE2/SPC12.BIN
  608599     7784     15941632  VOICE/SPC13.BIN               INF, OUT VOICE2/SPC13.BIN, QUA VOICE2/SPC13.BIN
  616383     6901     14133248  VOICE/SPC14.BIN               INF, OUT VOICE2/SPC14.BIN, QUA VOICE2/SPC14.BIN
  623284     7329     15009792  VOICE/SPC15.BIN               INF, OUT VOICE2/SPC15.BIN, QUA VOICE2/SPC15.BIN
  630613     7558     15478784  VOICE/SPC16.BIN               INF, OUT VOICE2/SPC16.BIN, QUA VOICE2/SPC16.BIN
  638171     6279     12859392  VOICE/SPC17.BIN               INF, OUT VOICE2/SPC17.BIN, QUA VOICE2/SPC17.BIN
  644450      648      1327104  VOICE/FOOD.BIN                INF, OUT, QUA
  645098    49624    101629952  VOICE/PGUSO.BIN               INF, OUT, QUA
  694722    18056     36978688  VOICE/PRESENT.BIN             OUT, QUA
  712778     4890     10014720  VOICE/SPCTALK.BIN             OUT, QUA
  717668     8852     18128896  VOICE/PARTY.BIN               OUT, QUA
  726520    63683    130422784  VOICE/BGM.BIN                 -
  790203    42468     86974464  STREAM/STRCMNE.BIN            -
  832671    25955     53155840  STREAM/STRSUBE.BIN            -
  858626   208427    426858496  STREAM/STR2E.BIN              -
 1067053   173175    354662400  STREAM/STR1E.BIN              -
 1240228    42298     86626304  STREAM/STRCMN.BIN             -
 1282526   173237    354789376  STREAM/STR1.BIN               -
 1455763   208360    426721280  STREAM/STR2.BIN               -
 1664123   106457    218023936  STREAM/STRSUB.BIN             -
 1770580     1577      3227652  PSS/LOGO_B.PSS                INF, OUT, QUA
 1772157     1953      3997700  PSS/LOGO_C.PSS                INF, OUT, QUA
 1774110     1585      3244036  PSS/LOGO_H.PSS                INF, OUT, QUA
 1775695    54233    111067140  PSS/OPENING2.PSS              -
 1829928      512      1048576  OUTSIDE.BIN                   INF, OUT, QUA
```

## What each is

| file | what | page |
| --- | --- | --- |
| `SLUS_205.62` | the EE executable, stripped; Infection's names carried onto it (`SLUS_205.62.syms`) | [the four discs](volumes.md#names-for-the-stripped-executables) |
| `IOPRP243.IMG` | the IOP replacement image the game reboots the IOP with, the same on every disc | [the Infection DVD](layout.md#the-iop-image) |
| `MODULES/*.IRX` | IOP modules: the ten `ccCdInit` and `ccLoad_SndModules` load, and the unused `CDVDFSV` and `CDVDMAN` | [the Infection DVD](layout.md#the-iop-image) |
| `DATA/*.PRG` | the four overlays | [overlays](../formats/prg.md) |
| `DATA/DATA.BIN` | the archive, 1,031 records | [the DATA.BIN archive](../formats/data-bin.md), [the four discs](volumes.md#databin-across-volumes) |
| `DATA/SNDDATA.BIN` | sound banks | [sound banks](../formats/snddata.md) |
| `DATA/ICON.BIN` | the save icons of all four volumes | [saves](../formats/save.md#files) |
| `VOICE/SPC00`-`SPC17`, `VOICE_E/SPC00_E`-`SPC17_E` | the party's skill words (`spcVoiceData`), Infection's files | [voice](../formats/voice.md), [sound](../engine/sound.md) |
| `VOICE/EVVOL1S`, `VOICE_E/EVVOL1SE` | volume 1's side events' lines, Infection's files | [voice](../formats/voice.md) |
| `VOICE/EVVOL2`, `VOICE_E/EVVOL2_E` | volume 2's main events' lines | [voice](../formats/voice.md) |
| `VOICE/EVVOL2S`, `VOICE_E/EVVOL2SE` | volume 2's side events' lines | [voice](../formats/voice.md) |
| `VOICE_E/MIAE.BIN` | Mia's English field lines after she has been talked to (`talkNum`) | [sound](../engine/sound.md) |
| `VOICE/PARTY`, `PRESENT`, `SPCTALK`, their `_E` twins | the party's field lines, presents and talk | [sound](../engine/sound.md) |
| `VOICE/PGUSO`, `FOOD`, `FOUNTAIN`, `INU` and twins | Grunties, Grunty food, the fountain, the dog | [sound](../engine/sound.md) |
| `VOICE/BGM.BIN` | streamed music | [voice](../formats/voice.md#streamed-music-voicebgmbin) |
| `STREAM/*.BIN` | the in-engine streams: `STRCMN`, `STR1`, `STR2`, `STRSUB`, each Japanese and English (`E`) | [streams](../engine/stream.md) |
| `PSS/LOGO_*.PSS` | the three logo movies, Infection's | [PSS](../formats/pss.md) |
| `PSS/OPENING2.PSS` | the opening movie | [PSS](../formats/pss.md) |
| `OUTSIDE.BIN` | 1,048,576 zero bytes; nothing names it | [the Infection DVD](layout.md#notes) |

## Notes

**Against Infection's disc.**
- New: `VOICE/EVVOL2.BIN`, `EVVOL2S.BIN`, `VOICE_E/EVVOL2_E.BIN`,
  `EVVOL2SE.BIN` and `MIAE.BIN`; `STREAM/STR2*.BIN` and `STRSUB*.BIN`;
  `PSS/OPENING2.PSS`.
- Gone: `VOICE/EVVOL1.BIN` and `VOICE_E/EVVOL1_E.BIN`, volume 1's main
  events' lines (its side events' stay), and `PSS/OPENING.PSS`.
- The same bytes as Infection's: `IOPRP243.IMG`; every module but
  `SNDBASE.IRX` and `SEWORDS.IRX`; `ICON.BIN`; the logos; `OUTSIDE.BIN`; and
  the voice files `SPC*`, `PGUSO*`, `FOOD*`, `FOUNT*`/`FOUNTAIN`, `INU*`,
  `EVVOL1S*` and `PRESENTE`.
- `SNDBASE.IRX` (29,744 bytes) and `SEWORDS.IRX` (42,352) are rebuilt and
  smaller: Infection's carry their symbols and DWARF, these do not.
- `STREAM/STR1.BIN` and `STR1E.BIN` keep Infection's sizes. 19 of their gzip
  members differ only in the header's time stamp (packed again on
  2002-10-23, the same inflated bytes); one scene changed, `str9102`
  (7,156,433 bytes inflated in Infection, 7,173,272 here).

**The CD modules.** `MODULES/CDVDFSV.IRX` and `CDVDMAN.IRX` are
Infection's files: version 1.04 (`PsIIcdvdman 134`). No executable names
them. The game's CD drivers are `IOPRP243.IMG`'s, version 2.26.
Outbreak and Quarantine leave the two files off.

## Unknown

None: every file is accounted for above.
