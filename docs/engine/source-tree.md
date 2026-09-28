---
title: The source tree
status: solid
volumes: INF
covers: INF SLUS_202.67 .debug compile units
worklog: 6
---

# The source tree

Every translation unit the DWARF describes, in link order, grouped by where
its code lives. Paths are as the compiler recorded them. A unit built as a
unity build lists the other files whose functions it contains after `+`.
"fns" counts functions with code. Ranges are the lowest and highest address of
the unit's code; a unit with no range has only types and data.

Roots: `D:\usr\RpgUS\prog\source` is the game, `D:\usr\RpgUS\prog\system`
is the engine (`cc` classes), and `C:\CodeWarrior\...` is the compiler's
runtime. 216 translation units, 244 source paths. Header-only files (`.h`)
appear only when they contributed an out-of-line function.

## main (resident)

50 translation units, 1279 functions.

| unit | code | fns | also contains |
| --- | --- | ---: | --- |
| `source\crt0.s` | 0x00100000-0x001000c8 | 0 |  |
| `CW\PS2 Support\gcc_wrapper.c` | 0x001000d0-0x00100174 | 4 |  |
| `source\mwUtils_PS2.c` | 0x00100180-0x00100368 | 3 |  |
| `system\system.cpp` | 0x00100370-0x0010b624 | 143 | sysmem.cpp, sysdebug.cpp, syshelper.cpp, syspad.cpp, sysframe.cpp, sysdl.cpp, sysblt.cpp, sysview.cpp, sysdrawenv.cpp, sysaxis.cpp, syslayer.cpp, syspcm.cpp |
| `CW\2.4_FE_Based_4.2.5\PS2 Support\Msl\MSL_C++\MSL_Common\Src\iostream.cpp` | 0x0010d760-0x0010dc48 | 2 | vector, stdexcept |
| `system\libcc3d.cpp` | 0x00136820-0x00143398 | 147 | shadow.cpp |
| `system\libccs.cpp` | 0x001437e0-0x00149990 | 65 | anmctrl.cpp |
| `system\libccs2.cpp` | 0x00149990-0x00152eb4 | 77 |  |
| `system\libhit.cpp` | 0x00152ec0-0x00155c60 | 27 |  |
| `system\ringbuff.cpp` | 0x00155c60-0x00156ac8 | 26 |  |
| `system\ungzip.cpp` | 0x00156ad0-0x00159620 | 15 |  |
| `system\cdvd.cpp` | 0x00159620-0x00159cb8 | 8 |  |
| `system\thread.cpp` | 0x00159cc0-0x0015a77c | 17 |  |
| `system\main.cpp` | 0x0015a780-0x0015a838 | 1 |  |
| `system\oakdebug.cpp` | 0x0015a840-0x0015a858 | 2 |  |
| `system\sprite.cpp` | 0x0015a860-0x0015c900 | 14 |  |
| `system\font.cpp` | 0x0015c900-0x0015faa0 | 24 |  |
| `system\fade.cpp` | 0x0015faa0-0x0016060c | 12 |  |
| `source\camera.cpp` | 0x00160610-0x00163440 | 29 |  |
| `source\filelib.cpp` | 0x00163440-0x00165630 | 16 |  |
| `source\datatbl.cpp` |  | 0 |  |
| `source\mcard.cpp` | 0x00165630-0x00167178 | 11 |  |
| `source\mother.cpp` | 0x00167180-0x00169af4 | 34 |  |
| `source\sdmng.cpp` | 0x001715e0-0x001794a8 | 78 |  |
| `source\sndlib.cpp` | 0x001794b0-0x00180e34 | 55 |  |
| `source\sndSYS.CPP` | 0x00180e40-0x0018414c | 32 |  |
| `source\snddtbl.cpp` |  | 0 |  |
| `source\sndtbl.cpp` |  | 0 |  |
| `source\strdemo.cpp` | 0x00184150-0x00197d68 | 39 |  |
| `source\stream.cpp` | 0x00197d70-0x0019abf4 | 27 |  |
| `source\strtbl.cpp` |  | 0 |  |
| `source\strtblp.cpp` |  | 0 |  |
| `source\strmsg.cpp` |  | 0 |  |
| `source\loaddisp.cpp` | 0x0019ac00-0x0019c454 | 13 |  |
| `source\world_man.cpp` | 0x0019c460-0x001a4578 | 74 | eventarea.h, roottown.h |
| `source\message.cpp` | 0x001a4580-0x001a6c68 | 25 |  |
| `source\evmng.cpp` | 0x001a6c70-0x001b785c | 51 |  |
| `source\evtbl.cpp` |  | 0 |  |
| `source\evmsg.cpp` |  | 0 |  |
| `source\evWav1.cpp` |  | 0 |  |
| `source\menumsg.cpp` |  | 0 |  |
| `source\menuwin.cpp` | 0x001b7860-0x001bc2f0 | 30 |  |
| `source\particle.cpp` | 0x001bc2f0-0x001c2c2c | 42 |  |
| `source\effect.cpp` | 0x001c2c30-0x001d9618 | 113 |  |
| `source\ptcltbl.cpp` |  | 0 |  |
| `source\evWav2.cpp` |  | 0 |  |
| `source\tradelst.cpp` |  | 0 |  |
| `source\calc.cpp` | 0x001d9620-0x001dabfc | 23 |  |
| `source\evWav3.cpp` |  | 0 |  |
| `source\evWav4.cpp` |  | 0 |  |

## gcmn.prg - the field game

140 translation units, 2731 functions.

| unit | code | fns | also contains |
| --- | --- | ---: | --- |
| `source\LvUpPar.cpp` |  | 0 |  |
| `source\dfcomplete.cpp` | 0x00400880-0x00400ef8 | 6 |  |
| `source\area.cpp` | 0x00400f00-0x00401108 | 8 | eventarea.h |
| `source\area01.cpp` | 0x00401110-0x00401e00 | 7 |  |
| `source\area02.cpp` | 0x00401e00-0x00403584 | 9 |  |
| `source\area03.cpp` | 0x00403590-0x00403f00 | 7 |  |
| `source\area04.cpp` | 0x00403f00-0x00404cc0 | 7 |  |
| `source\area05.cpp` | 0x00404cc0-0x00405a00 | 7 |  |
| `source\area07.cpp` | 0x00405a00-0x00406cdc | 8 |  |
| `source\area06.cpp` | 0x00406ce0-0x004079e8 | 7 |  |
| `source\areab0.cpp` | 0x004079f0-0x00409a0c | 9 |  |
| `source\areabk0.cpp` | 0x00409a10-0x0040bcc8 | 13 |  |
| `source\book.cpp` | 0x0040bcd0-0x0041ad60 | 43 |  |
| `source\fellow.cpp` | 0x0041ad60-0x0041eae8 | 13 | fellow.h |
| `source\fellow01.cpp` | 0x0041eaf0-0x0041ed58 | 4 |  |
| `source\fellow02.cpp` | 0x0041ed60-0x0041efc8 | 4 |  |
| `source\fellow03.cpp` | 0x0041efd0-0x0041f238 | 4 |  |
| `source\fellow04.cpp` | 0x0041f240-0x0041f4a8 | 4 |  |
| `source\fellow05.cpp` | 0x0041f4b0-0x0041f718 | 4 |  |
| `source\fellow06.cpp` | 0x0041f720-0x0041f988 | 4 |  |
| `source\fellow07.cpp` | 0x0041f990-0x0041fbf8 | 4 |  |
| `source\fellow08.cpp` | 0x0041fc00-0x0041fe68 | 4 |  |
| `source\fellow09.cpp` | 0x0041fe70-0x004200d8 | 4 |  |
| `source\fellow10.cpp` | 0x004200e0-0x00420348 | 4 |  |
| `source\fellow11.cpp` | 0x00420350-0x004205b8 | 4 |  |
| `source\fellow12.cpp` | 0x004205c0-0x00420828 | 4 |  |
| `source\fellow13.cpp` | 0x00420830-0x00420a98 | 4 |  |
| `source\fellow14.cpp` | 0x00420aa0-0x00420d08 | 4 |  |
| `source\fellow15.cpp` | 0x00420d10-0x00420f84 | 4 |  |
| `source\fellow16.cpp` | 0x00420f90-0x004211f8 | 4 |  |
| `source\fellow17.cpp` | 0x00421200-0x00421468 | 4 |  |
| `source\town01.cpp` | 0x00421470-0x004240b8 | 16 | roottown.h |
| `source\town02.cpp` | 0x004240c0-0x00426a80 | 8 |  |
| `source\town03.cpp` | 0x00426a80-0x00429bbc | 10 |  |
| `source\town04.cpp` | 0x00429bc0-0x0042c04c | 8 |  |
| `source\town05.cpp` | 0x0042c050-0x0042df0c | 8 |  |
| `source\entctrl.cpp` | 0x0042df10-0x0043283c | 56 | entctrl.h |
| `source\enemy.cpp` | 0x00432840-0x00437bd0 | 43 | enemy.h |
| `source\enesub.cpp` | 0x00437bd0-0x0043ece8 | 63 | enesub.h |
| `source\enelist.cpp` |  | 0 |  |
| `source\enemy1.cpp` | 0x0043ecf0-0x0043f90c | 7 |  |
| `source\enemy2.cpp` | 0x0043f910-0x004402c8 | 8 |  |
| `source\enemy3.cpp` | 0x004402d0-0x00440b58 | 8 |  |
| `source\enemy4.cpp` | 0x00440b60-0x00441548 | 8 |  |
| `source\enemyA.cpp` | 0x00441550-0x004420dc | 7 |  |
| `source\enemyB.cpp` | 0x004420e0-0x00442a7c | 8 |  |
| `source\enemyC.cpp` | 0x00442a80-0x004438d8 | 9 |  |
| `source\enemyD.cpp` | 0x004438e0-0x0044480c | 9 |  |
| `source\enemyE.cpp` | 0x00444810-0x00445494 | 7 |  |
| `source\enemyF.cpp` | 0x004454a0-0x00445fbc | 7 |  |
| `source\enemyG.cpp` | 0x00445fc0-0x004492cc | 15 |  |
| `source\enemyH.cpp` | 0x004492d0-0x00449e58 | 9 |  |
| `source\enemyI.cpp` | 0x00449e60-0x0044a72c | 7 |  |
| `source\enemyK.cpp` | 0x0044a730-0x0044b12c | 7 |  |
| `source\enemyL.cpp` | 0x0044b130-0x0044e49c | 21 |  |
| `source\enemyP.cpp` | 0x0044e4a0-0x0044ee68 | 7 |  |
| `source\enemyS.cpp` | 0x0044ee70-0x0044f938 | 8 |  |
| `source\enemyT.cpp` | 0x0044f940-0x00450328 | 7 |  |
| `source\enemyU.cpp` | 0x00450330-0x004513ac | 8 |  |
| `source\enemyV.cpp` | 0x004513b0-0x00451f88 | 7 |  |
| `source\enemyW.cpp` | 0x00451f90-0x004529f8 | 7 |  |
| `source\enemyz.cpp` | 0x00452a00-0x0045339c | 8 |  |
| `source\gimmick.cpp` | 0x004533a0-0x00453420 | 2 |  |
| `source\gmbox.cpp` | 0x00453420-0x00454838 | 9 |  |
| `source\gmcircle.cpp` | 0x00454840-0x0045621c | 8 |  |
| `source\gmetc.cpp` | 0x00456220-0x0045821c | 12 |  |
| `source\gmfood.cpp` | 0x00458220-0x00458de4 | 5 |  |
| `source\gmgate.cpp` | 0x00458df0-0x00459618 | 8 |  |
| `source\gmidol.cpp` | 0x00459620-0x00459d4c | 4 |  |
| `source\gmsymbol.cpp` | 0x00459d50-0x0045b254 | 14 |  |
| `source\boss.cpp` | 0x0045b260-0x0045fbdc | 78 | boss.h |
| `source\bosscam.cpp` | 0x0045fbe0-0x004610d4 | 12 |  |
| `source\bosseff.cpp` | 0x004610e0-0x0047b2f8 | 205 | bosseff.h |
| `source\boss01.cpp` | 0x0047b300-0x0047ee68 | 26 |  |
| `source\boss02.cpp` | 0x0047ee70-0x0048852c | 54 |  |
| `source\boss03.cpp` | 0x00488530-0x00491aec | 75 |  |
| `source\boss04.cpp` | 0x00491af0-0x00496ee4 | 33 |  |
| `source\boss05.cpp` | 0x00496ef0-0x0049e590 | 50 |  |
| `source\boss06.cpp` | 0x0049e590-0x004a2a94 | 29 |  |
| `source\boss07.cpp` | 0x004a2aa0-0x004a6908 | 32 |  |
| `source\boss08.cpp` | 0x004a6910-0x004ab568 | 41 |  |
| `source\boss08_b.cpp` | 0x004ab570-0x004b1afc | 38 |  |
| `source\boss08_c.cpp` | 0x004b1b00-0x004b8c80 | 58 |  |
| `source\kyvia01.cpp` | 0x004b8c80-0x004bf66c | 20 |  |
| `source\kyvia02.cpp` | 0x004bf670-0x004c7cbc | 23 |  |
| `source\kyvia03.cpp` | 0x004c7cc0-0x004d0c6c | 25 |  |
| `source\kyvia04.cpp` | 0x004d0c70-0x004dd104 | 40 |  |
| `source\kyviaCore.cpp` | 0x004dd110-0x004e3014 | 46 |  |
| `source\kyviaGomora.cpp` | 0x004e3020-0x004e8a18 | 32 |  |
| `source\effect2.cpp` | 0x004e8a20-0x00502328 | 176 | effect2.h |
| `source\water.cpp` | 0x00502330-0x005028dc | 2 |  |
| `source\snow.cpp` | 0x005028e0-0x005053d8 | 22 |  |
| `source\merchan.cpp` | 0x005053e0-0x00506638 | 9 |  |
| `source\rtownnpc.cpp` | 0x00506640-0x0050933c | 15 |  |
| `source\dog.cpp` | 0x00509340-0x0050a454 | 8 |  |
| `source\pgbreed.cpp` | 0x0050a460-0x00510878 | 30 |  |
| `source\pgrider.cpp` | 0x00510880-0x005130ac | 17 |  |
| `source\ccnavi.cpp` | 0x005130b0-0x00515d2c | 34 |  |
| `source\BreakMirror.cpp` | 0x00515d30-0x00516a1c | 8 |  |
| `source\efftblB.cpp` |  | 0 |  |
| `source\enemytbl.cpp` |  | 0 |  |
| `source\bosstbl.cpp` |  | 0 |  |
| `source\navitbl.cpp` |  | 0 |  |
| `source\npctbl.cpp` |  | 0 |  |
| `source\gimtbl.cpp` |  | 0 |  |
| `source\skilltbl.cpp` |  | 0 |  |
| `source\itemtbl.cpp` |  | 0 |  |
| `source\seWavTbl.cpp` |  | 0 |  |
| `source\msgtbl.cpp` |  | 0 |  |
| `source\setbl.cpp` |  | 0 |  |
| `source\equiptbl.cpp` |  | 0 |  |
| `source\charitem.cpp` |  | 0 |  |
| `source\shoplist.cpp` |  | 0 |  |
| `source\itemlist.cpp` |  | 0 |  |
| `source\gameover.cpp` | 0x00516a20-0x005177f4 | 17 |  |
| `source\gamectrl.cpp` | 0x00517800-0x0051c0d4 | 34 | uprollstr.cpp, uprollstr.h |
| `source\menu.cpp` | 0x0051c0e0-0x0056aee8 | 172 |  |
| `source\common.cpp` | 0x0056aef0-0x00571d9c | 59 |  |
| `source\hit.cpp` | 0x00571da0-0x005722e0 | 3 |  |
| `source\skill.cpp` | 0x005722e0-0x00579ac4 | 54 |  |
| `source\lattice.cpp` | 0x00579ad0-0x0057a6c8 | 9 |  |
| `source\useitem.cpp` | 0x0057a6d0-0x0057c5e4 | 4 |  |
| `source\spcchat.cpp` |  | 0 |  |
| `source\personal.cpp` | 0x0057c5f0-0x005977cc | 170 |  |
| `source\player.cpp` | 0x005977d0-0x0059ce00 | 34 |  |
| `source\party.cpp` | 0x0059ce00-0x0059d224 | 10 |  |
| `source\spc.cpp` | 0x0059d230-0x005a1b60 | 70 |  |
| `source\world.cpp` | 0x005a1b60-0x005ad9a0 | 40 |  |
| `source\field.cpp` | 0x005ad9a0-0x005af460 | 19 |  |
| `source\fieldmesh.cpp` | 0x005af460-0x005b0b74 | 7 |  |
| `source\fobject.cpp` | 0x005b0b80-0x005b1b58 | 13 |  |
| `source\fractal.cpp` | 0x005b1b60-0x005b5ff0 | 23 |  |
| `source\roottown.cpp` | 0x005b5ff0-0x005b61dc | 4 |  |
| `source\dungeon.cpp` | 0x005b61e0-0x005cf9ac | 54 |  |
| `source\sobj.cpp` | 0x005cf9b0-0x005d04bc | 11 |  |
| `source\tobj.cpp` | 0x005d04c0-0x005d17c8 | 11 |  |
| `source\BossSkilltbl.cpp` |  | 0 |  |
| `source\voicetbl.cpp` |  | 0 |  |
| `source\fetbl.cpp` |  | 0 |  |
| `source\trapitemlist.cpp` |  | 0 |  |

## demo.prg - opening and movies

5 translation units, 127 functions.

| unit | code | fns | also contains |
| --- | --- | ---: | --- |
| `source\demo.cpp` | 0x00400880-0x00400bbc | 3 |  |
| `source\chartbl.cpp` |  | 0 |  |
| `source\DataControl.cpp` | 0x00400bc0-0x004043a8 | 26 |  |
| `source\opening.cpp` | 0x004043b0-0x00409de8 | 31 |  |
| `source\mpeg.cpp` | 0x00409df0-0x0040db80 | 67 |  |

## desktop.prg - the in-game desktop

17 translation units, 210 functions.

| unit | code | fns | also contains |
| --- | --- | ---: | --- |
| `source\MediaTbl.cpp` |  | 0 |  |
| `source\desktop.cpp` | 0x00400880-0x004009f8 | 3 |  |
| `source\desktopMode.cpp` | 0x00400a00-0x00403c38 | 16 |  |
| `source\acces.cpp` | 0x00403c40-0x00405700 | 19 | vector |
| `source\audio.cpp` | 0x00405700-0x00408350 | 22 |  |
| `source\mailer.cpp` | 0x00408350-0x0040d1c4 | 35 |  |
| `source\webnews.cpp` | 0x0040d1d0-0x0040edd4 | 22 |  |
| `source\savedata.cpp` | 0x0040ede0-0x0041240c | 20 |  |
| `source\htmltbl.cpp` |  | 0 |  |
| `source\mailtbl.cpp` |  | 0 |  |
| `source\mailtblp.cpp` |  | 0 |  |
| `source\walltbl.cpp` |  | 0 |  |
| `source\wavetbl.cpp` |  | 0 |  |
| `source\stfroll.cpp` | 0x00412410-0x0041430c | 24 |  |
| `source\srdata.cpp` |  | 0 |  |
| `source\NameEntry.cpp` | 0x00414310-0x0041b5b0 | 49 |  |
| `source\NameCodeTbl.cpp` |  | 0 |  |

## toppage.prg - the bulletin board

4 translation units, 49 functions.

| unit | code | fns | also contains |
| --- | --- | ---: | --- |
| `source\toppage.cpp` | 0x00400880-0x00401728 | 14 |  |
| `source\bbs.cpp` | 0x00401730-0x0040495c | 35 |  |
| `source\bbsmsg.cpp` |  | 0 |  |
| `source\bbsmsgP.cpp` |  | 0 |  |

## Unknown

Nothing.
