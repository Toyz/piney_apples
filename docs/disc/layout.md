---
title: The Infection DVD
status: solid
volumes: INF
covers: INF SYSTEM.CNF, every file on the disc, IOPRP243.IMG, INF SLUS_202.67:0x00159620 ccCdInit, 0x00180e40 ccLoad_SndModules, 0x00159980 ccLoadModule, 0x001659b0 ccMcard::MakeDir; INF demo.prg:0x0040b560 strFileOpen
worklog: 1, 14, 239
---

# The Infection DVD

What is on the .hack//Infection (USA) disc, where, and which page describes each
file.

## Volume

ISO 9660 only - no Joliet, no Rock Ridge. 2048-byte sectors, 1,268,832 of them
(2,598,567,936 bytes). System identifier `PLAYSTATION`, volume identifier
empty. The size fits a single-layer DVD-5; the layer structure itself was not checked.

`SYSTEM.CNF`, 57 bytes:

```
BOOT2 = cdrom0:\SLUS_202.67;1
VER = 1.00
VMODE = NTSC
```

## Files, in disc order

86 files, packed with no gaps from LBA 370 to LBA 1268821. Sizes in bytes.

```
     lba  sectors         size  path
     370        1           57  SYSTEM.CNF
     371     9387     19223388  SLUS_202.67
    9758      124       253201  IOPRP243.IMG
    9882        9        16901  MODULES/CDVDFSV.IRX
    9891       13        26069  MODULES/CDVDMAN.IRX
    9904       11        21101  MODULES/MODMIDI.IRX
    9915        4         6977  MODULES/SDRDRV.IRX
    9919        4         6553  MODULES/SIO2MAN.IRX
    9923       22        44917  MODULES/PADMAN.IRX
    9945       30        59509  MODULES/MODHSYN.IRX
    9975       13        25397  MODULES/LIBSD.IRX
    9988       45        90805  MODULES/MCMAN.IRX
   10033        4         7353  MODULES/MCSERV.IRX
   10037       33        67536  MODULES/SNDBASE.IRX
   10070       37        74512  MODULES/SEWORDS.IRX
   10107       31        61568  DATA/DEMO.PRG
   10138      211       432128  DATA/DESKTOP.PRG
   10349     1525      3121280  DATA/GCMN.PRG
   11874       85       172800  DATA/TOPPAGE.PRG
   11959      504      1032192  DATA/ICON.BIN
   12463    66731    136665088  DATA/DATA.BIN
   79194    20369     41715712  DATA/SNDDATA.BIN
   99563     6971     14276608  VOICE_E/SPC00_E.BIN
  106534     7555     15472640  VOICE_E/SPC01_E.BIN
  114089     8257     16910336  VOICE_E/SPC02_E.BIN
  122346     9222     18886656  VOICE_E/SPC03_E.BIN
  131568     8576     17563648  VOICE_E/SPC04_E.BIN
  140144     8011     16406528  VOICE_E/SPC05_E.BIN
  148155     7234     14815232  VOICE_E/SPC06_E.BIN
  155389     9244     18931712  VOICE_E/SPC07_E.BIN
  164633     7994     16371712  VOICE_E/SPC08_E.BIN
  172627     6762     13848576  VOICE_E/SPC09_E.BIN
  179389     5612     11493376  VOICE_E/SPC10_E.BIN
  185001     7272     14893056  VOICE_E/SPC11_E.BIN
  192273     7814     16003072  VOICE_E/SPC12_E.BIN
  200087     7456     15269888  VOICE_E/SPC13_E.BIN
  207543     6639     13596672  VOICE_E/SPC14_E.BIN
  214182     7648     15663104  VOICE_E/SPC15_E.BIN
  221830     7221     14788608  VOICE_E/SPC16_E.BIN
  229051     6181     12658688  VOICE_E/SPC17_E.BIN
  235232    43211     88496128  VOICE_E/PGUSO_E.BIN
  278443      745      1525760  VOICE_E/FOOD_E.BIN
  279188     4110      8417280  VOICE_E/FOUNT_E.BIN
  283298     3580      7331840  VOICE_E/INU_E.BIN
  286878     7887     16152576  VOICE_E/PARTY_E.BIN
  294765    15641     32032768  VOICE_E/PRESENTE.BIN
  310406     3871      7927808  VOICE_E/SPCTALKE.BIN
  314277    15713     32180224  VOICE_E/EVVOL1SE.BIN
  329990    92165    188753920  VOICE_E/EVVOL1_E.BIN
  422155    98552    201834496  VOICE/EVVOL1.BIN
  520707    16293     33368064  VOICE/EVVOL1S.BIN
  537000     4586      9392128  VOICE/FOUNTAIN.BIN
  541586     3713      7604224  VOICE/INU.BIN
  545299    16683     34166784  VOICE/PRESENT.BIN
  561982     6996     14327808  VOICE/SPC00.BIN
  568978     7863     16103424  VOICE/SPC01.BIN
  576841     8207     16807936  VOICE/SPC02.BIN
  585048    10080     20643840  VOICE/SPC03.BIN
  595128     9845     20162560  VOICE/SPC04.BIN
  604973     8254     16904192  VOICE/SPC05.BIN
  613227     7241     14829568  VOICE/SPC06.BIN
  620468     8501     17410048  VOICE/SPC07.BIN
  628969     8867     18159616  VOICE/SPC08.BIN
  637836     7104     14548992  VOICE/SPC09.BIN
  644940     5905     12093440  VOICE/SPC10.BIN
  650845     7137     14616576  VOICE/SPC11.BIN
  657982     8361     17123328  VOICE/SPC12.BIN
  666343     7784     15941632  VOICE/SPC13.BIN
  674127     6901     14133248  VOICE/SPC14.BIN
  681028     7329     15009792  VOICE/SPC15.BIN
  688357     7558     15478784  VOICE/SPC16.BIN
  695915     6279     12859392  VOICE/SPC17.BIN
  702194      648      1327104  VOICE/FOOD.BIN
  702842     7873     16123904  VOICE/PARTY.BIN
  710715    49624    101629952  VOICE/PGUSO.BIN
  760339     4250      8704000  VOICE/SPCTALK.BIN
  764589    30769     63014912  VOICE/BGM.BIN
  795358    41937     85886976  STREAM/STRCMNE.BIN
  837295   173175    354662400  STREAM/STR1E.BIN
 1010470    41742     85487616  STREAM/STRCMN.BIN
 1052212   173237    354789376  STREAM/STR1.BIN
 1225449     1577      3227652  PSS/LOGO_B.PSS
 1227026     1953      3997700  PSS/LOGO_C.PSS
 1228979     1585      3244036  PSS/LOGO_H.PSS
 1230564    37745     77299716  PSS/OPENING.PSS
 1268309      512      1048576  OUTSIDE.BIN
```

## What each is

| file | page |
| --- | --- |
| `SLUS_202.67` | [the EE executable](../engine/executable.md) |
| `DATA/*.PRG` | [overlays](../formats/prg.md) |
| `DATA/DATA.BIN` | [the DATA.BIN archive](../formats/data-bin.md) |
| `DATA/SNDDATA.BIN` | [sound banks](../formats/snddata.md) |
| `DATA/ICON.BIN` | the memory card icons of all four volumes, three each, which `ccMcard::MakeDir` copies into a new save's directory ([saves](../formats/save.md#files)) |
| `MODULES/*.IRX`, `IOPRP243.IMG` | IOP modules ([the IOP image](#the-iop-image)); `SNDBASE.IRX` and `SEWORDS.IRX` carry symbols and DWARF (worklog 14), the rest are stripped |
| `PSS/*.PSS` | MPEG-2 program streams; `OPENING.PSS` audio is 48 kHz stereo in an `SShd` private stream |
| `STREAM/*.BIN` | gzip archives of [CCSF](../formats/ccs.md) cutscenes, audio in [Pcm chunks](../formats/voice.md) |
| `VOICE/*.BIN`, `VOICE_E/*.BIN` | [voice lines and streamed music](../formats/voice.md) |
| `OUTSIDE.BIN` | 1,048,576 zero bytes |

## The IOP image

`IOPRP243.IMG` is the same 253,201 bytes on all four discs: a ROMDIR image
(`RESET`, `ROMDIR`, `EXTINFO`, then the modules), built
`20011205-113144` (its `ROMDIR` comment: `conffile,ioprp243.img`). Its
modules, with their `EXTINFO` versions, all `PsII... 2430`:

| module | version | comment | bytes |
| --- | --- | --- | ---: |
| `LOADCORE` | 0x0203 | `Module_Manager` | 9,717 |
| `SIFCMD` | 0x0207 | `IOP_SIF_rpc_interface` | 10,105 |
| `SIFMAN` | 0x0201 | `IOP_SIF_manager` | 6,009 |
| `THREADMAN` | 0x0202 | `Multi_Thread_Manager` | 36,945 |
| `IOMAN` | 0x0203 | `IO/File_Manager` | 12,545 |
| `MODLOAD` | 0x0205 | `Moldule_File_loader` | 17,589 |
| `FILEIO` | 0x020d | `FILEIO_service` | 19,573 |
| `CDVDMAN` | 0x021a | `cdvd_driver` | 64,437 |
| `CDVDFSV` | 0x021a | `cdvd_ee_driver` | 38,837 |
| `LOADFILE` | 0x0201 | `LoadModuleByEE` | 11,617 |
| `TIMEMANI` | 0x0201 | `Timer_Manager` | 5,813 |
| `ROMDRV` | 0x0201 | `ROM_file_driver` | 3,881 |
| `EESYNC` | 0x0201 | `SyncEE` | 1,545 |
| `SYSCLIB` | 0x0202 | `System_C_lib` | 10,205 |
| `STDIO` | 0x0203 | `Stdio` | 3,377 |

`ccCdInit` (0x00159620) starts the disc:
1. `sceSifInitRpc`, `sceCdInit`, `sceCdMmode(2)`.
2. `sceSifRebootIop("cdrom0:\IOPRP243.IMG;1")` until it succeeds, then
   `sceSifSyncIop` until done.
3. The RPC and CD again, `sceFsReset`.
4. The six files it reads by sector, looked up by name
   (`sceCdSearchFile`, each until found).
5. Six modules from `cdrom0:\MODULES\` (`ccLoadModule`): `sio2man`,
   `padman`, `mcman`, `mcserv`, `libsd`, `sdrdrv`.

`ccLoad_SndModules` loads the other four: `modhsyn`, `modmidi`, `sewords`,
`sndbase`. These ten are every module any volume names; the executable's
`libcdvd` is `PsIIlibcdvd 2420`.

`MODULES/CDVDFSV.IRX` (16,901 bytes) and `CDVDMAN.IRX` (26,069) are version
1.04 (`PsIIcdvdman 134`, `PsIIcdvdfsv 134`). No executable names them, so
the CD drivers the game runs are always the image's (2.26). Mutation
carries the same two files; Outbreak and Quarantine leave them off.

## How the game reads the disc

Every read starts from a name:
- **By sector.** Only files looked up with `sceCdSearchFile`, then read
  with `ccCdvd::Read2` / `ReadSt` at the LBA it returns plus an offset
  inside the file:
  - `ccCdInit`'s six: `\DATA\SNDDATA.BIN`, `\DATA\DATA.BIN`, and
    `\STREAM\STRCMN`, `STRCMNE`, `STR1`, `STR1E`. The archives' own
    tables give the offsets ([the DATA.BIN archive](../formats/data-bin.md),
    [streams](../engine/stream.md#the-tables)).
  - `ccMcard::MakeDir`: `\DATA\ICON.BIN`.
  - demo.prg's `strFileOpen`: `\PSS\` and the movie's name, streamed with
    `sceCdStStart`.
- **By path.** The overlays (`cdrom0:\DATA\GCMN.PRG` ...), the modules,
  and the voice files, which the IOP's sound modules open themselves
  (`cdrom0:\VOICE\...`).

No code holds a sector number of its own.

## Notes

`VOICE/` holds the Japanese voice banks and `VOICE_E/` the English ones, file
for file, except that `VOICE/BGM.BIN` has no English twin. `STREAM/` pairs
the same way: `STR1.BIN` / `STR1E.BIN`, `STRCMN.BIN` / `STRCMNE.BIN`.

`OUTSIDE.BIN` is the last file and entirely zero, 1,048,576 bytes, the same
on all four discs. No executable or overlay of any volume contains the
string `OUTSIDE`, and nothing reads the disc by sector without a name, so
nothing reads it. It pads the end of the disc (inference).

## Unknown

None: every file is accounted for above. (An image does not record the
pressed disc's layers; the size fits a single-layer DVD-5.)
