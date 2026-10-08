#!/usr/bin/env python3
"""The EE side of the sound system run in tools/eemu.py: what the game's own
sndlib.cpp / sndSYS.CPP functions send to the IOP, for checking
crates/piney-audio's driver against.

    tools/sound_ee.py fixture ELF       the expectations, to stdout
                                        (crates/piney-audio/tests/driver_fixture.txt)

The functions run for real with desktop.prg loaded; what leaves the EE is
caught at the edges and written down instead of sent:

  sceMSIn_PutMsg / PutHsMsg     msg BYTES             MIDI to synthesizer port 0
  ccSndCmd(0xb0 | p, v)         vol P V               sceHSyn_SetVolume
  ccSndCmd3(0x110 | m, 0)       play M                locate 0 + play sequencer m
  ccSndCmd3(0x20 | m, 0)        stop M
  ccSndCmd3(0x200, n)           area N                SNDBASE's area (ccSndSQLoad's context)
  ccSndCmd3(0x300, t)           playtype T            SNDBASE's playtype
  ccSndCmd3(0x130, 0)           change                SNDBASE's bgmChange (the battle music)
  ccSndCmd(0x9310, &dataLoad)   load OFS              a bank from SNDDATA.BIN, then
  ccSndCmd(0x40 | i, addr)        seq I OFFSET        sequence i at bank offset
  sceSdRemote(SetParam 0x0981)  master V              core 1 MVOLL (MVOLR alike)
  sceSdRemote(SetEffectAttr)    reverb CORE MODE      sdCommand 4 (100) and 5 (105)

Other commands (port attributes, bank-to-port, all sound off) are left
out. ccBreathThread, the wait for the other threads, returns at once after
clearing ccSound.soundOffFlag as the sound task would - except in the
jukebox scenarios, where it runs the sound task's frames.

Scenarios, one fixture line each:
  se N                   ccSeOn(N)
  note N K               ccSeOnNote(N, K)
  desktop NO             ccSndChangeData(&Wave[NO], -1), then ccSndBgmCtrl
                         with ccSound.gameStart set (ccSetupDesktop)
  volume NO M S B        after `desktop NO`: ccSetMainVol(M), ccSetSeVol(S),
                         ccSetBgmVol(B), then one frame of ccSndChangeOption
                         and ccSound::sdCommand
  jukebox A B            after `desktop A`: ccSndChangeData(&Wave[B], A), as
                         the jukebox's BgmRead thread calls it, each
                         ccBreathThread(n) running n frames of the sound
                         task (ccFade, ccSndChangeOption, sdCommand)
  seq STEP+STEP...       from a fresh ccSound with the sound task's frames
                         running: `load N` ccSndSQLoad(N) (7 the title, 0
                         the board, 1 the desktop, 2 the town, 3 a field,
                         4 a dungeon, 5 an event bank), `play N` ccSqPlay(N),
                         `stop N` ccSqStop(N), `fade N V T M` ccSqFade,
                         `main V` ccSetMainVol, `start G` ccSound.gameStart,
                         `off` ccAllSoundOff, `frames K` K frames of the task
                         (ccFade with gameStart, ccSceneFade without it),
                         `set NAME V` a field of the world ccSndSQLoad and
                         ccSndBgmCtrl read (see WORLD), `bgm`
                         ccSndBgmCtrl, `hold` ccSndEvRequest(10, 0, 0, 0)
                         (the event instruction `sound 10`), `desktop NO`
                         as the `desktop` scenario (dtBgm NO), `battle B`
                         game.inBattle, `ride R` pgRideFlag, `pginit`
                         ccPgBgmInit, `pgend N` ccPgBgmEnd(N), `breeder X
                         Y` the town file's DMY_merchant6 (what
                         GetChunkAdrsF hands bgmBreed), `kite X Y` Kite's
                         place, `camera X Y` the active camera's, `block B`
                         game.block, `scene` ccSoundMain's scene sound by
                         +0x105: bgmChurch (2) or bgmBreed (3); waterTest
                         (1) is not run

The areas' scenarios (area_scenarios) load the town's (2), a field's (3),
a dungeon's (4) and the event banks (5) for every row the game can pick,
then run ccSndBgmCtrl and, for battle banks, battles starting and ending;
ccSound::bgmChange runs in the sound task's frames as ccSoundRpc runs it,
before ccFade.

    tools/sound_ee.py voice-fixture ELF  the voice requests
                                        (crates/piney-audio/tests/voice_ee_fixture.txt)

The voice scenarios run ccEvVoiceRequest (0x0017e810), ccEvVoiceStop
(0x0017ee40) and the sound task's evVoicePlay (0x0017eca0) with
saveData.voice (+0x842c) and parodyFlag (+0x842b) set, and catch what
reaches sewordCmd (0x00183d00), the RPC to SEWORDS.IRX:

  seword 80e0 OFS SIZE W8 VOL FILE     wordPlay: vBank {ofs, siz, 0, vol, name}
  seword 120                           stop channel 0

A scenario is a list of steps joined by '+', each `voice E M` (a request),
`stop`, `pginit` (ccPgBgmInit), `lang L` (set saveData.voice), `parody P` or `play` (one run of
evVoicePlay, which the sound task makes once a frame); the LANG and PARODY
the scenario starts with come first.

    tools/sound_ee.py hold-fixture ELF...  the race's hold on channel 0
                                        (crates/piney-audio/tests/voice_hold_fixture.txt)

From Mutation on ccEvVoiceStop and ccPgBgmInit's inline stop do nothing
while ccSnd +0x13a (the Flag Race's hold) is set. Steps as above, with
`hold H` (+0x13a), `area A` (game.area, which ccPgBgmInit reads from
Mutation on: 0 the town's path, which sets the hold) and `off`
(ccAllSoundOff); each line starts with the executable's volume.
"""

import argparse
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import eemu
import image  # noqa: E402

# ccVoiceRequest's field voice groups: (group, voice file, table).
FIELD_VOICE = (
    (-2, 0, "chibigusoTbl"), (-3, 0, "gusoponTbl"), (-4, 0, "kizokuTbl"), (-5, 0, "ironTbl"),
    (-6, 0, "poisonTbl"), (-7, 0, "boneTbl"), (-8, 0, "suneTbl"), (-9, 0, "aquaTbl"),
    (-10, 0, "milkTbl"), (-11, 0, "rockerTbl"), (-12, 0, "woodTbl"),
    (-13, 5, "inu0VoiceTbl"), (-14, 5, "inu1VoiceTbl"), (-15, 5, "inu2VoiceTbl"), (-16, 5, "inu3VoiceTbl"),
    (-20, 1, "fountainVoiceTbl"), (-30, 2, "partyInOutVoiceTbl"), (-31, 3, "spcTalkVoiceTbl"),
    (-32, 4, "presentVoiceTbl"), (-40, 19, "fidhellVoiceTbl"),
)

SND = 0x01800000        # a ccSound
SAVE = 0x01810000       # a ccSaveData, 0x8530 bytes
GAME = 0x01820000       # a ccGame
IOP_HEAP = 0x00100000   # what sceSifAllocIopHeap hands out
WM = 0x01830000         # a WORLD_MAN
CHAR, CBASE, EVMNG = 0x01840000, 0x01841000, 0x01842000   # a ccChar, its base, an eventMng
# The scene sounds' world: a town (WORLD_MAN +0x430) whose stream's
# DMY_merchant6 chunk is CHUNK, Kite's ccChar (+0x40 his place), a camera.
TOWN, CHUNK, KITE, CAM = 0x01850000, 0x01851000, 0x01852000, 0x01853000
# checkPartyMenberNum(int) (gcmn.prg; INF 0x0059D100) and pgRideFlag (INF
# 0x00378CDC): each volume's own, by name (Ee.check_party, Ee.pg_ride).
# `set NAME V`: (base, offset, size) of what ccSndSQLoad and ccSndBgmCtrl
# read. piros is checkPartyMenberNum(8)'s answer (0 in the party, -1 not).
WORLD = {
    "area": (GAME, 0x14, 4), "areaPrev": (GAME, 0x18, 4), "town": (GAME, 0x20, 4),
    "field": (GAME, 0x24, 4), "dungeon": (GAME, 0x28, 4), "townPrev": (GAME, 0x38, 4),
    "fieldPrev": (GAME, 0x3C, 4), "dungeonPrev": (GAME, 0x40, 4), "inBattle": (GAME, 0x58, 4),
    "bg": (WM, 0xC, 4), "fieldtype": (WM, 0x10, 4), "dtype0": (WM, 0x34, 4), "dtype1": (WM, 0x38, 4),
    "dtype2": (WM, 0x3C, 4), "crisis": (SAVE, 0x6772, 1), "dtBgm": (SAVE, 0x2237, 1),
}


class Ee:
    def __init__(self, elf, overlay="desktop"):
        self.p = image.Program(elf, overlay)
        # VU0 macro mode for the scene sounds' vectors (sceVu0CopyVector,
        # ccGetDist): test_anim's Python machine, eemu.Machine plus VU0.
        from test_anim import _python_machine_class
        self.m = _python_machine_class()(self.p)
        self.log = []
        m = self.m
        self.sym = lambda n: self.p.symbol_named(n).value
        m.store(self.sym("ccSnd"), 4, SND)
        m.store(self.sym("saveData"), 4, SAVE)
        m.store(self.sym("game"), 4, GAME)
        m.store(self.sym("worldman"), 4, WM)
        self.pristine_objects()
        hooks = {
            "ccSndCmd__Fii": self._cmd,
            "ccSndCmd3__Fii": self._cmd3,
            "ccBreathThread__Fi": self._breath,
            "sceSifAllocIopHeap": lambda m, n, *a: IOP_HEAP,
            "sceSifFreeIopHeap": lambda m, *a: 0,
            "sceMSIn_PutMsg": self._put_msg,
            "sceMSIn_PutHsMsg": self._put_hs,
            "sceSdRemote": self._remote,
            "ccEvVoiceStop__Fv": lambda m, *a: 0,
        }
        for name, fn in hooks.items():
            m.hooks[self.sym(name)] = fn
        # checkPartyMenberNum lives in gcmn, which the desktop overlay's
        # image does not map: hooked at the volume's own address.
        self.check_party = image.Program(elf, "gcmn").symbol_named("checkPartyMenberNum__Fi").value
        self.pg_ride = self.sym("pgRideFlag")
        m.hooks[self.check_party] = lambda m, *a: self.party
        m.hooks[self.sym("GetChunkAdrsF__8ccStreamFPCci")] = lambda m, *a: CHUNK
        self.kite_at = self.kite_pointer(image.Program(elf, "gcmn"))
        self.frames = False
        self.snap = bytes(m.mem)

    def pristine_objects(self):
        m = self.m
        self.party = 0xFFFFFFFF
        for base, size in ((SND, 0x150), (SAVE, 0x8530), (GAME, 0x100), (WM, 0x4E0), (TOWN, 0x200),
                           (CHUNK, 0x20), (KITE, 0x50), (CAM, 0x10)):
            m.mem[base:base + size] = bytes(size)
        m.store(WM + 0x430, 4, TOWN)
        for at in (0x1A4, 0x1A8):     # the town's stream: Infection's place, the later volumes'
            m.store(TOWN + at, 4, TOWN + 0x100)
        m.store(self.sym("activeCamPtr"), 4, CAM)
        m.call(self.sym("__ct__7ccSoundFv"), [SND])

    def kite_pointer(self, gcmn):
        """Where bgmBreed reads Kite's ccChar: its first `lui $at` and
        `lw $v0, lo($at)` (plw+0x20, in gcmn.prg)."""
        f = self.p.symbol_named("bgmBreed__Fv")
        hi = None
        for a in range(f.value, f.value + f.size, 4):
            w = self.p.u32(a)
            op, rs, rt, imm = w >> 26, (w >> 21) & 31, (w >> 16) & 31, w & 0xFFFF
            if op == 0x0F and rt == 1:
                hi = imm << 16
            elif op == 0x23 and rs == 1 and rt == 2 and hi is not None:
                return (hi + (imm - 0x10000 if imm & 0x8000 else imm)) & 0xFFFFFFFF
        raise ValueError("bgmBreed reads no Kite")

    def place(self, at, x, y):
        """(x, y, 0, 1) as floats at `at`."""
        for k, v in enumerate((x, y, 0.0, 1.0)):
            self.m.store(at + 4 * k, 4, struct.unpack("<I", struct.pack("<f", v))[0])

    def reset(self):
        self.m.mem[:] = self.snap
        self.log = []
        self.frames = False
        self.party = 0xFFFFFFFF

    # the edges ------------------------------------------------------------------
    def _cmd(self, m, cmd, data, *a):
        if cmd & 0xFFF0 == 0xB0:
            self.log.append(f"vol {cmd & 15} {data}")
        elif cmd == 0x9310:
            dl = self.sym("dataLoad")
            ofs = m.load(dl + 4, 4)
            self.log.append(f"load {ofs}")
            m.store(self.sym("sbuff"), 4, m.load(dl + 12, 4))   # the IOP's reply: bd size
        elif cmd & 0xFFF0 == 0x40:
            self.log.append(f"seq {cmd & 15} {data - IOP_HEAP}")
        return 0

    def _cmd3(self, m, cmd, data, *a):
        if cmd & 0xFFF0 == 0x110:
            self.log.append(f"play {cmd & 15}")
        elif cmd & 0xFFF0 == 0x20:
            self.log.append(f"stop {cmd & 15}")
        elif cmd == 0x200:
            self.log.append(f"area {eemu_s32(data)}")
        elif cmd == 0x300:
            self.log.append(f"playtype {eemu_s32(data)}")
        elif cmd == 0x130:
            self.log.append("change")
        return 0

    def _breath(self, m, n, *a):
        """ccBreathThread(n): n frames of the sound task (ccSoundRpc,
        0x00182dc0) pass - ccFade while the music is not stopped and the
        game has started, ccSndChangeOption, sdCommand, and its
        all-sound-off."""
        if not self.frames:
            m.store(SND + 308, 1, 0)
            return 0
        for _ in range(max(n, 1)):
            self.task()
        return 0

    def task(self):
        """One frame of ccSoundRpc's loop, the parts that reach what is
        logged: ccFade while the music is not held and the game has started,
        ccSceneFade while it has not, ccSndChangeOption, sdCommand, and its
        all-sound-off."""
        m = self.m
        if not m.load(SND + 97, 1) and m.load(SND + 23, 1):
            self.nested("bgmChange__7ccSoundFv", SND)
            self.nested("ccFade__7ccSoundFv", SND)
        if not m.load(SND + 23, 1):
            self.nested("ccSceneFade__7ccSoundFv", SND)
        self.nested("ccSndChangeOption__Fv")
        self.nested("sdCommand__7ccSoundFv", SND)
        m.store(SND + 308, 1, 0)

    def nested(self, name, *args):
        """Call a function from inside a hook, keeping the caller's state."""
        m = self.m
        saved = (list(m.r), list(m.f), m.hi, m.lo, m.hi1, m.lo1, m.fcr31, m.acc, m.steps)
        m.call(self.sym(name), list(args))
        m.r, m.f, m.hi, m.lo, m.hi1, m.lo1, m.fcr31, m.acc, m.steps = saved

    def _put_msg(self, m, ctx, port, msg, *a):
        st = msg & 0xFF
        n = 2 if st & 0xF0 in (0x80, 0xC0, 0xD0) else 3
        self.log.append("msg " + bytes((msg >> (8 * i)) & 0xFF for i in range(n)).hex())
        return 0

    def _put_hs(self, m, ctx, port, ptr, *a):
        n = {0xF9: 5, 0xFD: 7}.get(m.load(ptr, 1), 0)
        self.log.append("msg " + bytes(m.mem[ptr:ptr + n]).hex())
        return 0

    def _remote(self, m, arg, func, entry, value, *a):
        if func == 0x8010 and entry in (0x0981, 0x0A81):
            self.log.append(f"master {entry:x} {value}")
        if func == 0x8130:
            # sdCommand 4 and 5: sceSdSetEffectAttr(core, attr), mode +4.
            self.log.append(f"reverb {entry} {m.load(value + 4, 4):x}")
        return 0

    def call(self, name, *args):
        return self.m.call(self.sym(name), list(args))

    def seq(self, steps):
        for step in steps:
            word, *a = step.split()
            a = [int(x) for x in a] if word != "set" else []
            if word == "load":
                self.call("ccSndSQLoad__Fi", a[0])
            elif word == "play":
                self.call("ccSqPlay__Fi", a[0])
            elif word == "stop":
                self.call("ccSqStop__Fi", a[0])
            elif word == "fade":
                self.call("ccSqFade__FiUsiUc", a[0], a[1], a[2], a[3])
            elif word == "main":
                self.call("ccSetMainVol__Fi", a[0])
            elif word == "start":
                self.m.store(SND + 23, 1, a[0])
            elif word == "off":
                self.call("ccAllSoundOff__Fv")
            elif word == "frames":
                for _ in range(a[0]):
                    self.task()
            elif word == "set":
                name, v = step.split()[1], int(step.split()[2])
                if name == "piros":
                    self.party = 0 if v else 0xFFFFFFFF
                else:
                    base, ofs, size = WORLD[name]
                    self.m.store(base + ofs, size, v & ((1 << (8 * size)) - 1))
            elif word == "bgm":
                self.call("ccSndBgmCtrl__Fv")
            elif word == "hold":
                self.call("ccSndEvRequest__Fiiii", 10, 0, 0, 0)
            elif word == "desktop":
                self.desktop(a[0])
            elif word == "battle":
                self.m.store(GAME + 0x58, 4, a[0])
            elif word == "ride":
                self.m.store(self.pg_ride, 4, a[0])
            elif word == "pginit":
                self.call("ccPgBgmInit__Fv")
            elif word == "pgend":
                self.call("ccPgBgmEnd__Fi", a[0])
            elif word == "breeder":
                self.place(CHUNK + 0x10, a[0], a[1])
            elif word == "kite":
                self.m.store(self.kite_at, 4, KITE)
                self.place(KITE + 0x40, a[0], a[1])
            elif word == "camera":
                self.place(CAM, a[0], a[1])
            elif word == "block":
                self.m.store(GAME + 0x30, 4, a[0])
            elif word == "scene":
                mode = self.m.load(SND + 0x105, 1)
                if self.m.load(SND + 23, 1) and mode in (2, 3):
                    self.call("bgmChurch__Fv" if mode == 2 else "bgmBreed__Fv")
            else:
                raise ValueError(step)

    def wave(self, no):
        return self.sym("Wave") + 24 * no

    def desktop(self, no):
        self.m.store(SAVE + 0x2237, 1, no)
        # Its breaths run the sound task, as the game's thread does.
        frames, self.frames = self.frames, True
        self.call("ccSndChangeData__FP8WaveDatai", self.wave(no), 0xFFFFFFFF)
        self.frames = frames
        self.m.store(SND + 23, 1, 1)      # gameStart
        self.call("ccSndBgmCtrl__Fv")


def eemu_s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v >> 31 else v


def fixture(elf):
    ee = Ee(elf)
    se = ee.p.symbol_named("seData")
    n_se = se.size // 8
    wave = ee.p.symbol_named("Wave")
    n_wave = wave.size // 24 - 1
    print("# tools/sound_ee.py fixture: the game's EE sound code run in tools/eemu.py; what it sends.")
    print("# SCENARIO ARGS | OUTPUT; OUTPUT; ...")

    def emit(head):
        print(head + " | " + "; ".join(ee.log))

    for n in range(n_se):
        ee.reset()
        ee.call("ccSeOn__Fi", n)
        emit(f"se {n}")
    for n in (0, 4, 21, 42, 100):
        for k in (0, 36, 60, 64, 127, -5):
            ee.reset()
            ee.call("ccSeOnNote__Fic", n, k & 0xFFFFFFFF)
            emit(f"note {n} {k}")
    for no in range(n_wave):
        ee.reset()
        ee.desktop(no)
        emit(f"desktop {no}")
    for no, main, sev, bgm in ((50, 256, 256, 256), (50, 128, 64, 200), (46, 0, 256, 0),
                               (27, 256, 96, 32), (7, 200, 0, 256), (19, 64, 128, 8)):
        ee.reset()
        ee.desktop(no)
        ee.log = []
        ee.call("ccSetMainVol__Fi", main)
        ee.call("ccSetSeVol__Fi", sev)
        ee.call("ccSetBgmVol__Fi", bgm)
        ee.call("ccSndChangeOption__Fv")
        ee.call("sdCommand__7ccSoundFv", SND)
        emit(f"volume {no} {main} {sev} {bgm}")
    for a, b in ((50, 11), (11, 50), (46, 47), (47, 46), (27, 18), (7, 12), (19, 3), (12, 25),
                 (0, 0), (47, 7)):
        ee.reset()
        ee.desktop(a)
        ee.log = []
        ee.frames = True
        ee.call("ccSndChangeData__FP8WaveDatai", ee.wave(b), a)
        emit(f"jukebox {a} {b}")
    for steps in SEQ_SCENARIOS + area_scenarios():
        ee.reset()
        ee.frames = True
        ee.seq(steps)
        emit("seq " + "+".join(steps))
    return 0


# The title (ccSetupDemo's ccSndSQLoad(7); ccThDemo's ccSetMainVol,
# ccSqPlay(0), ccSqStop(0) and, leaving for the desktop, ccSqFade(0, 0, 8,
# 3)), the board's and the desktop's loads, and the fades both ways.
SEQ_SCENARIOS = [
    ["load 7"],
    ["load 0"],
    ["load 1"],
    ["load 7", "stop 0", "main 256", "play 0", "frames 2"],
    ["load 7", "stop 0", "main 128", "play 0", "frames 2", "play 0"],
    ["load 7", "play 0", "fade 0 0 8 3", "frames 12"],
    ["load 7", "play 0", "fade 0 128 4 1", "frames 8", "stop 0", "stop 0"],
    ["load 7", "play 0", "start 1", "fade 0 0 8 3", "frames 12"],
    ["load 7", "play 0", "fade 0 0 8 3", "frames 3", "off", "frames 2"],
    ["load 7", "play 0", "frames 1", "load 7", "play 0"],
    ["load 1", "play 0", "play 1", "fade 1 0 6 3", "frames 8"],
    ["main 64", "load 7", "play 0", "frames 1"],
]


# Weathers SimGenerateCode leaves with each field type (areas.clamp_weather).
FIELD_WEATHERS = [4, 4, 4, 4, 4, 6, 6, 8, 8, 8, 8]


def area_scenarios():
    """ccSetupGameCtrl's loads and ccSndBgmCtrl for every row the areas can
    pick, and the battle music switching both ways."""
    out = []
    start = ["start 1", "bgm", "frames 1"]
    for ft in range(11):
        for bg in range(FIELD_WEATHERS[ft]):
            out.append(["set area 1", "set areaPrev 0", f"set fieldtype {ft}", f"set bg {bg}", "load 3"] + start)
    for ft in (0, 5, 10):
        out.append(["set area 1", "set areaPrev 0", f"set fieldtype {ft}", "set bg 1", "set piros 1", "load 3"]
                   + start)
    for dt in range(11):
        out.append(["set area 2", "set areaPrev 1", "set dungeon 1", f"set dtype1 {dt}", "load 4"] + start)
    # The same dungeon again: CheckSceneReplace is false, nothing starts.
    out.append(["set area 2", "set areaPrev 2", "set dungeon 1", "set dungeonPrev 1", "set dtype1 3", "load 4"]
               + start)
    for field in range(123):
        for prev in (1, 2):
            out.append([f"set area {prev}", f"set areaPrev {prev}", f"set field {field}", "load 5"] + start)
    for town in range(5):
        for crisis in (0, 1):
            out.append(["set area 0", "set areaPrev 1", f"set town {town}", f"set crisis {crisis}", "load 2"]
                       + start)
    # The event instruction `sound 10` holds the next ccSndBgmCtrl only.
    out.append(["set area 1", "set fieldtype 10", "set bg 0", "hold", "load 3", "start 1", "bgm", "frames 1",
                "bgm", "frames 1"])
    out.append(["hold", "set area 2", "set areaPrev 2", "load 4", "start 1", "bgm", "bgm", "frames 1"])
    # The desktop's ccSndChangeData sets +0xf0 to 1 whatever loaded last:
    # its ccSndBgmCtrl takes the hold (the endings' `sound 10`), and the
    # staff roll's ccSndBgmCtrl afterwards starts the theme.
    for no in (50, 27):
        out.append(["set area 0", "load 2", "start 1", "bgm", "frames 1", "hold", f"desktop {no}", "frames 1",
                    "bgm", "frames 1"])
    out.append(["set area 1", "set fieldtype 2", "set bg 2", "load 3", "start 1", "bgm", "frames 1",
                "desktop 12", "frames 1", "bgm", "frames 1"])
    # Battles: a field and a dungeon of each play type, and on a Grunty.
    fight = ["frames 2", "battle 1", "frames 34", "battle 0", "frames 34"]
    for ft, bg in ((0, 0), (2, 2), (4, 2), (10, 5)):
        out.append(["set area 1", f"set fieldtype {ft}", f"set bg {bg}", "load 3"] + start + fight)
    for dt in (0, 3, 4):
        out.append(["set area 2", "set areaPrev 1", f"set dtype0 {dt}", "load 4"] + start + fight)
    out.append(["set area 1", "set fieldtype 2", "set bg 2", "load 3"] + start + ["ride 1"] + fight)
    # The Grunty Flute's ride (ccPgBgmInit as it starts, ccPgBgmEnd(0) back
    # to the field's music or (1) into a dungeon) over a field of each play
    # type and a battle bank's switched music.
    for ft, bg in ((0, 0), (2, 2), (4, 2), (10, 5)):
        for n in (0, 1):
            out.append(["set area 1", f"set fieldtype {ft}", f"set bg {bg}", "load 3"] + start
                       + ["frames 3", "ride 1", "pginit", "frames 12", f"pgend {n}", "ride 0", "frames 14"])
    out.append(["set area 1", "set fieldtype 2", "set bg 2", "load 3"] + start
               + ["battle 1", "frames 34", "battle 0", "ride 1", "pginit", "frames 5", "pgend 0", "ride 0",
                  "frames 34"])
    out.append(["set area 1", "set field 66", "set areaPrev 1", "load 5"] + start + fight)
    out.extend(scene_scenarios())
    return out


def scene_scenarios():
    """The breeder's tune (bgmBreed) and the church's music (bgmChurch):
    near and far, and across the loads that clear their `free[4]` (a town
    other than Mac Anu, area 15's event bank) and those that do not."""
    town = ["set area 0", "set areaPrev 1", "load 2", "start 1", "bgm", "frames 1"]
    near, far = ["kite -2200 -5600", "scene", "frames 32"], ["kite 3000 3000", "scene", "frames 32"]
    loireag, carmina = ["breeder -2200 -5800"], ["breeder -3830 -3450"]
    out = [
        # Near the breeder and away, twice, in Dun Loireag.
        ["set town 1"] + town + loireag + near + far + near + ["kite -2200 -6900", "scene", "frames 2"],
        # Dun Loireag's tune, then Carmina Gadelica's breeder read afresh.
        ["set town 1"] + town + loireag + near + ["set town 2"] + town + carmina
        + ["kite -3830 -3250", "scene", "frames 32"],
        # Mac Anu between them clears nothing, and runs no bgmBreed.
        ["set town 1"] + town + loireag + near + ["set town 0"] + town + ["scene", "frames 2", "set town 3"] + town
        + ["breeder 0 0", "kite 0 500", "scene", "frames 32"],
        # A field between them does not clear either; the next town does.
        ["set town 1"] + town + loireag + near + ["set area 1", "set fieldtype 2", "set bg 2", "load 3", "start 1",
                                                    "bgm", "scene", "frames 2", "set town 2"] + town + carmina
        + ["kite -3830 -3250", "scene", "frames 32"],
    ]
    church = ["set area 1", "set areaPrev 1", "set field 15", "load 5", "start 1", "bgm", "frames 1"]
    walk = ["camera 0 0", "scene", "frames 2", "camera 0 3000", "scene", "frames 2", "block 1", "scene", "frames 32",
            "block 0", "scene", "frames 32"]
    out.append(church + walk)
    # The ranch's tune on, then area 15: its load clears it, so the church
    # starts its music.
    out.append(["set town 1"] + town + loireag + near + church + walk)
    return out


VOICE_FLAG = 0x842C     # ccSaveData.voice: 0 Japanese, otherwise English
PARODY_FLAG = 0x842B    # ccSaveData.parodyFlag


class VoiceEe(Ee):
    """Ee with the real ccEvVoiceStop and sewordCmd caught."""

    def __init__(self, elf, overlay="desktop"):
        super().__init__(elf, overlay)
        m = self.m
        del m.hooks[self.sym("ccEvVoiceStop__Fv")]
        m.hooks[self.sym("sewordCmd__Fii")] = self._seword
        self.snap = bytes(m.mem)

    def _seword(self, m, cmd, data, *a):
        if cmd == 0x80E0:
            ofs, size, w8, vol = (m.load(data + 4 * i, 4, i < 3) for i in range(4))
            name = eemu._cstr(m, data + 16).decode("latin-1")
            self.log.append(f"seword 80e0 {ofs} {size} {w8} {vol:x} {name}")
        else:
            self.log.append(f"seword {cmd:x}")
        return 0

    def run(self, lang, parody, steps):
        self.reset()
        self.m.store(SAVE + VOICE_FLAG, 1, lang)
        self.m.store(SAVE + PARODY_FLAG, 1, parody)
        for step in steps:
            word, *args = step.split()
            if word == "voice":
                e, k = (int(x) for x in args)
                self.call("ccEvVoiceRequest__Fii", e & 0xFFFFFFFF, k & 0xFFFFFFFF)
            elif word == "stop":
                self.call("ccEvVoiceStop__Fv")
            elif word == "pginit":
                self.call("ccPgBgmInit__Fv")
            elif word == "play":
                self.call("evVoicePlay__Fv")
            elif word == "lang":
                self.m.store(SAVE + VOICE_FLAG, 1, int(args[0]))
            elif word == "parody":
                self.m.store(SAVE + PARODY_FLAG, 1, int(args[0]))
            elif word == "words":
                # ccWordsPlay(sid, ch): a ccChar whose base has the type
                # (+0x08) and the charTbl row (+0x0c), eventMng +0x78c the
                # event running. (The last argument, the skill's type bit,
                # is the port's; the game reads it itself.)
                c, ty, sid, ev = (int(x) for x in args[:4])
                m = self.m
                m.mem[CHAR:CHAR + 0x10] = bytes(0x10)
                m.mem[CBASE:CBASE + 0x40] = bytes(0x40)
                m.mem[EVMNG:EVMNG + 0x800] = bytes(0x800)
                m.store(CHAR, 4, CBASE)
                m.store(CBASE + 0x08, 4, ty)
                m.store(CBASE + 0x0C, 2, c & 0xFFFF)
                m.store(self.sym("eventMng"), 4, EVMNG)
                m.store(EVMNG + 0x78C, 4, ev)
                self.call("ccWordsPlay__FiP6ccChar", sid & 0xFFFFFFFF, CHAR)
            elif word == "skill":
                self.call("skillVoicePlay__Fv")
            elif word == "hold":
                self.m.store(SND + 0x13A, 1, int(args[0]))
            elif word == "area":
                self.m.store(GAME + 0x14, 4, int(args[0]))
            elif word == "off":
                self.call("ccAllSoundOff__Fv")
            else:
                raise ValueError(step)

    def skill_bit(self, sid):
        """ccGetSkillParam(sid)+0x2c & 1, as skillVoicePlay reads it."""
        self.reset()
        self.m.call(self.sym("ccGetSkillParam__Fi"), [sid & 0xFFFFFFFF])
        p = self.m.r[2] & 0xFFFFFFFF
        return self.m.load(p + 0x2C, 4) & 1


def voice_rows(p):
    """{(event, lang): rows} for the events whose volume 1 table entry is
    not NULL."""
    out = {}
    for table, base, lang in (("evVoiceDataVol1M", 0, 0), ("evVoiceDataVol1S", 50, 0),
                              ("evVoiceDataVol1ME", 0, 1), ("evVoiceDataVol1SE", 50, 1)):
        s = p.symbol_named(table)
        for i in range(50):
            x = p.u32(s.value + 4 * i)
            if x:
                out[(base + i, lang)] = p.symbol_named(p.name_at(x)).size // 8
    return out


def voice_fixture(elf):
    ee = VoiceEe(elf)
    rows = voice_rows(ee.p)
    print("# tools/sound_ee.py voice-fixture: ccEvVoiceRequest, ccEvVoiceStop and evVoicePlay "
          "run in tools/eemu.py; what reaches sewordCmd.")
    print("# LANG PARODY STEP+STEP... | OUTPUT; OUTPUT; ...")
    scenarios = []
    # Every row of every volume 1 table, in both languages, with and without
    # Parody Mode; and one past each table's end is left out (the game reads
    # the next table's rows there, and no script asks for one).
    for (e, lang), n in sorted(rows.items(), key=lambda kv: (kv[0][1], kv[0][0])):
        for k in range(n):
            for parody in (0, 1):
                scenarios.append((lang, parody, [f"voice {e} {k}", "play"]))
    # Events with no table, and the other volumes' events (their files are
    # not on this disc).
    for e in (0, 5, 9, 19, 32, 49, 55, 57, 64, 99, 100, 101, 116, 149, 150, 151, 168, 199,
              200, 201, 249, 250, 300, 349, 350, 400, 449, 499, -1):
        for lang in (0, 1):
            scenarios.append((lang, 0, [f"voice {e} 0", "play"]))
    # The slots and the shared request. (A request for an event with no
    # table sets the shared request to NULL, so one made earlier in the
    # frame would send whatever is at address 0; left out.)
    scenarios += [
        (1, 0, ["play"]),
        (1, 0, ["stop", "play"]),
        (1, 0, ["stop", "play", "play"]),
        (1, 0, ["voice 1 1", "play", "play"]),
        (1, 0, ["voice 1 1", "stop", "play"]),
        (1, 0, ["stop", "voice 1 1", "play"]),
        (1, 0, ["voice 1 0", "voice 1 2", "play"]),
        (0, 0, ["voice 2 3", "voice 59 7", "voice 1 1", "play"]),
        (1, 0, ["voice 1 1", "stop", "stop", "play"]),
        (1, 0, ["voice 1 1", "voice 2 0", "play"]),
        (1, 0, ["voice 1 1", "voice 3 5", "play"]),
        (0, 0, ["voice 1 1", "lang 1", "play"]),
        (1, 0, ["voice 1 1", "lang 0", "play"]),
        (1, 1, ["voice 1 1", "play"]),
        (1, 0, ["voice 1 1", "parody 1", "voice 2 0", "play"]),
        (1, 0, ["voice 1 1", "play", "voice 1 2", "play", "stop", "play"]),
        # ccPgBgmInit (the Grunty Flute's call): a stop in the first free
        # slot, as ccEvVoiceStop.
        (1, 0, ["pginit", "play"]),
        (1, 0, ["voice 1 1", "pginit", "play"]),
        (0, 0, ["voice 2 3", "voice 1 1", "pginit", "play"]),
    ]
    for lang, parody, steps in scenarios:
        ee.run(lang, parody, steps)
        print(f"{lang} {parody} {'+'.join(steps)} | " + "; ".join(ee.log))
    # ccVoiceRequest's groups (events below -1), whose tables are in
    # gcmn.prg: every row of each, in both languages, with and without
    # Parody Mode; groups with no case; and a group's request between event
    # voices in one frame.
    fe = VoiceEe(elf, "gcmn")
    field = []
    for g, _, name in FIELD_VOICE:
        n = fe.p.symbol_named(name).size // 8
        for lang in (0, 1):
            for k in range(n):
                for parody in (0, 1):
                    field.append((lang, parody, [f"voice {g} {k}", "play"]))
    for g in (-17, -21, -29, -33, -39, -41, -100):
        field.append((1, 0, [f"voice {g} 0", "play"]))
    field += [
        (1, 0, ["voice -30 1", "voice 1 1", "play"]),
        (1, 0, ["voice 1 1", "voice -30 1", "play"]),
        (0, 0, ["voice -2 0", "stop", "play"]),
    ]
    for lang, parody, steps in field:
        fe.run(lang, parody, steps)
        print(f"{lang} {parody} {'+'.join(steps)} | " + "; ".join(fe.log))
    # ccWordsPlay and skillVoicePlay: every skill of every character in
    # English, every seventh in Japanese, then the gates (an event running,
    # a caster of neither type, an id past 303), rows past the files'
    # characters, the queue full, and a word with no line left queued.
    bits = {sid: fe.skill_bit(sid) for sid in range(304)}
    w = lambda c, sid, ty=1, ev=0: f"words {c} {ty} {sid} {ev} {bits.get(sid, 0)}"
    words = []
    for c in range(18):
        for sid in range(304):
            words.append((1, 0, [w(c, sid), "skill"]))
        for sid in range(0, 304, 7):
            words.append((0, 0, [w(c, sid), "skill"]))
    words += [
        (1, 0, [w(0, 10, ev=1), "skill"]),
        (1, 0, [w(0, 10, ty=2), "skill"]),
        (1, 0, [w(0, 10, ty=4), "skill"]),
        (1, 0, [w(0, 304), "skill"]),
        (1, 0, [w(18, 10), "skill"]),
        (1, 0, [w(19, 10), "skill"]),
        (1, 0, ["skill"]),
        (1, 0, [w(0, 10), w(2, 40), w(15, 70), w(15, 71), w(8, 100), "skill"]),
        (1, 0, [w(2, 40), "skill", "skill"]),
        (1, 0, [w(0, 0), "skill", w(0, 10), "skill"]),
    ]
    for lang, parody, steps in words:
        fe.run(lang, parody, steps)
        print(f"{lang} {parody} {'+'.join(steps)} | " + "; ".join(fe.log))
    return 0


HOLD_SCENARIOS = (
    "stop+play", "hold 1+stop+play", "stop+hold 1+play", "hold 1+stop+hold 0+play",
    "hold 1+stop+hold 0+stop+play", "hold 1+stop+stop+play", "hold 1+off+play", "off+play",
    "area 1+pginit+play", "area 1+hold 1+pginit+play",
)
# ccPgBgmInit's town path (the race), which only the later volumes have.
HOLD_TOWN = ("area 0+pginit+stop+play", "area 0+pginit+hold 0+stop+play", "area 0+pginit+off+play")


def hold_fixture(elfs):
    print("# tools/sound_ee.py hold-fixture: ccEvVoiceStop, ccPgBgmInit and ccAllSoundOff with "
          "ccSnd +0x13a, then evVoicePlay, in tools/eemu.py; what reaches sewordCmd.")
    print("# VOLUME STEP+STEP... | OUTPUT; ...")
    for elf in elfs:
        name = {"SLUS_202.67": "inf", "SLUS_205.62": "mut", "SLUS_205.63": "out",
                "SLUS_205.64": "qua"}[os.path.basename(elf)]
        ee = VoiceEe(elf)
        for steps in HOLD_SCENARIOS + (HOLD_TOWN if name != "inf" else ()):
            ee.run(1, 0, steps.split("+"))
            print(f"{name} {steps} | " + "; ".join(ee.log))
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("fixture")
    p.add_argument("elf")
    p = sub.add_parser("voice-fixture")
    p.add_argument("elf")
    p = sub.add_parser("hold-fixture")
    p.add_argument("elf", nargs="+")
    args = parser.parse_args()
    if args.cmd == "hold-fixture":
        return hold_fixture(args.elf)
    if args.cmd == "voice-fixture":
        return voice_fixture(args.elf)
    return fixture(args.elf)


if __name__ == "__main__":
    sys.exit(main())
