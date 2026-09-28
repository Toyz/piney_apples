#!/usr/bin/env python3
"""crates/piney-world's dungeon runtime (dungeon_area.rs) against the game's
own DUNGEON code run in tools/eemu.py.

The dungeon's scene file (sd1..sda, the "a" set for texType 1) is decoded
in the interpreter by the game's own ccStream::DecodeSetup, so every ccAnm,
ccObj, ccModel and ccModelHit below is the game's. A scratch DUNGEON,
WORLD_MAN and ccGame are filled as DUNGEON::DUNGEON and WORLD_MAN::GO(2)
leave them (type, isEventArea, the EditDungeon rows, the fog table the
constructor picks, gimPos, UpRoom/DownRoom), and the real code runs:

  - DUNGEON::Generate (gcmn 0x005c12a0): random dungeons and story areas'
    EditDungeon tables, down to MakeRoom's room models, rotations, centres
    and startpos (the OBJ_0ppp dummies, through the real SetAnm,
    _AnimateForward, SetMatrix_PosRotZYX and _SetLWMatrix), UpRoom and
    DownRoom, and the SetRoom(0, 0) it ends with; GetStartPosition;
  - DUNGEON::SetRoom (0x005c1ca0) for every room built: the fog and
    ambient it hands ccDrawEnv, and the ccModelHit list it leaves - each
    hit's owner, type, rm and im to the bit - with SetDoor's doors; its
    dressing (SetWater's water and sparks, SetLight's room lights and omni
    lights, SetObject's clumps and walls, SetAnmObject's animated objects)
    and fieldrand after it, also in the lake dungeons (test_lake_dressing);
    DUNGEON::DrawEff (0x005cdee0) frame by frame (test_draw_eff);
  - ccLandHitCheck (0x00571e00) with game.area 2 at random points of each
    room: the height, the result count, the nearest result's contact
    point, distance and attribute, and checkHitResultAttlibute;
  - DUNGEON::GetHeight (0x005cf030) at random points;
  - DUNGEON::GotoNextRoom (0x005c9e10) from every door cell and the stairs
    rooms: the answer, WORLD_MAN.position and the room it builds (its hit
    list);
  - DUNGEON::RoomSelect (0x005c95c0) after ClearRoom and SetRoom, as the
    events' `room` has WORLD_MAN::RoomSelect run it: the position, level,
    roomEnterFlag, mapHideFlag and ccMenu's map status, and the room built;
  - the story dungeons' event rooms (test_special_rooms): the event room's
    own scene file (spccs) decoded beside the dungeon's, MakeRoom's rows
    of type 16 on, the event rooms' SetRoom (chunk, hits, SetFog,
    SetAmbient), GotoNextRoom's area branches with the room free and
    banned (ccSaveData::SetAreaBan, and SetDoor's ban block,
    CMP_o_block_m0_, in the room left), WORLD_MAN::Enter's first
    ChangeArea / ChangeScene (recorded), and RoomSelect into them;
  - DUNGEON::Draw (0x005ce930) before and after each door's GotoNextRoom,
    at the spot he left, the spot he arrived at and random points: the
    room of the player's cell, and whether the room and its doors are
    drawn (ccAnm::Draw recorded; the minimap, map, layers and door sounds
    stubbed);
  - Kite in a room (KiteInDungeon): cameraMain (main 0x00160cc0) then
    ccPlayer::Main (gcmn 0x00598310) frame by frame over the room the real
    SetRoom registered, with game.area 2, WORLD_MAN's bounds 0..60000 and
    the DUNGEON behind WORLD_MAN::AddCenter / GetHeight, from the start
    position walking into the walls and through the doors, as
    tools/test_world_rs.py's frames do in the town (its Field set-up,
    Kite's anm on tools/anim.py), and each frame whether he stepped on an
    entrance (WORLD_MAN::Enter).

beside crates/piney-world/examples/dungeon_probe.rs doing the same.

Stubbed in the interpreter: operator new (a bump allocator), the
semaphores, ccDrawEnv::SetFog / SetAmbient (recorded), ccLightGrp::DelGrp
and AddGrp (recorded). ccCheckActiveObject sees an empty
entry list, as the port assumes (no enemies): doors stand open. Pieces
posed by a keyed rotation (the doors that swing) are compared to 1e-5 /
0.05 and counted apart: piney_data::anim interpolates those in double
precision.

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import os
import random
import shutil
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
ISO = volume.ISO
DATA = volume.DATA
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "dungeon_probe")

ONE = 0x3F800000
M32 = 0xFFFFFFFF
# Scratch memory past the program's end (0x00730600).
CCSDATA, SYS, SCRATCH, GAME, SAVE, WM = 0x00800000, 0x00D00000, 0x00D10000, 0x00D20000, 0x00D30000, 0x00D40000
MISC, DRAWENV, MENU, ENT, PLAYER, ARGS = 0x00D41000, 0x00D41100, 0x00D41400, 0x00D41600, 0x00D42000, 0x00D43000
# The heap starts past test_world_rs's fixed areas (0x01000000-0x010fffff),
# which the frames below share the machine with.
DG, GIMPOS, HEAP0, HEAP_END = 0x00E00000, 0x00EE0000, 0x01100000, 0x01F00000
DG_SIZE = 0xD3910
PLW = inf_va(0x00730300)

# The story areas whose dungeons GotoNextRoom has branches of its own for.
SPECIAL_EVENTS = {16, 23, 25, 27, 46, 47, 48, 66, 71, 73, 77, 91, 101, 108}
# Their dungeons with rows of type 16 or more (story area, dungeon index),
# and the areas whose bans CheckAreaBan reads where it is not their own.
SPECIAL_DUNGEONS = ((23, 0), (25, 0), (16, 0), (27, 0), (91, 0), (46, 0), (47, 0), (48, 1), (66, 0), (67, 0),
                    (71, 0), (73, 0), (77, 0), (101, 0), (108, 0))
# GotoNextRoom's areas whose branch takes any row of type 16 or more.
LEAVE_AREAS = (108, 73, 47, 66, 46, 27)
SPECIAL_BANS = {16: 108, 91: 108}


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def bf(b):
    return struct.unpack("<f", struct.pack("<I", b & M32))[0]


def ordered(b):
    """A float's bits as an integer ordered like the floats."""
    b &= M32
    return -(b & 0x7FFFFFFF) if b & 0x80000000 else b


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                    "dungeon_probe"], cwd=ROOT, check=True)


class Probe:
    """dungeon_probe, one request at a time."""

    def __init__(self):
        self.p = subprocess.Popen([EXAMPLE, ISO], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
                                  cwd=ROOT)

    def ask(self, line):
        self.p.stdin.write(line + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        self.p.wait()
        self.p.stdout.close()


class Game:
    """The game's dungeon code in the interpreter over one decoded file."""

    def __init__(self):
        from image import Program
        from test_stream_rs import machine_class
        self.prog = Program(ELF, "gcmn")
        self.m = machine_class()(self.prog)
        self.sym = lambda n: self.prog.symbol_named(n).value      # noqa: E731
        m = self.m
        self.heap = HEAP0
        for name in ("ccMalloc__FUi", "__nw__FUi", "ccMemalign__FUiUi", "__nwa__FUi"):
            m.hooks[self.sym(name)] = self.malloc
        m.hooks[self.sym("ccMemalignB__FUiUi")] = lambda mm, al, n, *a: self.malloc(mm, n)
        m.hooks[self.sym("_ccMalloc__FUiUiUii")] = lambda mm, al, n, *a: self.malloc(mm, n)
        for name in ("ccFree__FPv", "__dl__FPv", "__dla__FPv", "_ccFree__FPv"):
            m.hooks[self.sym(name)] = lambda mm, *a: 0
        m.hooks[self.sym("OpenData__14ccRingBufferThFPv")] = self.open_data
        for name in ("CreateSema", "DeleteSema", "SignalSema", "WaitSema", "iSignalSema", "DIntr", "EIntr"):
            m.hooks[self.sym(name)] = lambda mm, *a: 1
        self.fogs = []
        m.hooks[self.sym("SetFog__9ccDrawEnvFffffUi")] = self.set_fog
        m.hooks[self.sym("SetAmbient__9ccDrawEnvFPf")] = self.set_ambient
        m.hooks[self.sym("DelGrp__10ccLightGrpFP7ccLight")] = lambda mm, *a: 0
        m.store(self.sym("ccSys"), 4, SYS)
        m.store(SYS + 604, 4, SCRATCH)
        m.store(self.sym("game"), 4, GAME)
        m.store(self.sym("saveData"), 4, SAVE)
        m.store(self.sym("worldman"), 4, WM)
        m.store(self.sym("cc3d"), 4, DRAWENV)
        m.store(self.sym("ccMenu"), 4, MENU)
        m.store(self.sym("g_entCtrl"), 4, ENT)
        m.store(PLW, 4, PLAYER)
        m.store(self.sym("volumeNum"), 4, 1)
        self.stem = None
        self.base = HEAP0
        self.names = {}
        self.data = None
        # The event room's scene file (spccs): its stream, its name, and its
        # Anime chunks by address.
        self.spccs, self.spccs_name, self.anm_chunks = 0, "", {}
        m.hooks[self.sym("GetCCSAdrs__8ccStreamFPCc")] = (
            lambda mm, name, *a: self.spccs if self.spccs and self.cstr(name) == self.spccs_name else 0)
        self.lights = []
        m.hooks[self.sym("AddGrp__10ccLightGrpFP7ccLight")] = lambda mm, grp, light, *a: self.lights.append(light) or 0
        self.changes = []
        m.hooks[self.sym("ChangeArea__6ccGameFii")] = (
            lambda mm, *a: self.changes.append(("area", mm.r[5] & M32, mm.r[6] & M32)) or 0)
        s32 = lambda v: v - (1 << 32) if v & 0x80000000 else v    # noqa: E731
        # this, a, t, fd in a0-a3; d, f, b in t0-t2 (registers 8-10).
        m.hooks[self.sym("ChangeScene__6ccGameFiiiiii")] = (
            lambda mm, *a: self.changes.append(("scene",) + tuple(s32(mm.r[k] & M32) for k in (5, 6, 7, 8, 9, 10)))
            or 0)

    def malloc(self, m, n, *_):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        if self.heap > HEAP_END:
            raise RuntimeError("heap exhausted")
        m.mem[a:a + n] = bytes(n)
        return a

    def open_data(self, m, reader, adrs, *_):
        m.store(reader + 36, 4, adrs)
        m.store(reader + 40, 4, 0x01FF0000)
        return 0

    def set_fog(self, m, env, colour, *_):
        f = m.f
        self.fogs.append((f[12] & M32, f[13] & M32, f[14] & M32, f[15] & M32, colour & M32))
        return 0

    def set_ambient(self, m, env, v, *_):
        self.fogs.append(tuple(m.load(v + 4 * k, 4) for k in range(3)))
        return 0

    def call(self, name, *args, fargs=()):
        for i, v in enumerate(fargs):
            self.m.f[12 + i] = v
        return self.m.call(self.sym(name) if isinstance(name, str) else name, list(args), limit=2_000_000_000)

    def cstr(self, a):
        out = bytearray()
        while self.m.mem[a]:
            out.append(self.m.mem[a])
            a += 1
        return out.decode("latin-1")

    def rvec(self, a, n=4):
        return [self.m.load(a + 4 * i, 4) for i in range(n)]

    # --- the scene file -----------------------------------------------------
    def load(self, stem, spccs=""):
        """ccStream::DecodeSetup over the file, and over the event room's
        (`spccs`, which GetCCSAdrs then answers); the heap after them is kept
        for each dungeon made from them."""
        if self.stem == (stem, spccs):
            return
        import ccs
        import gzarc
        if self.data is None:
            self.data = gzarc.open_bytes(DATA)
            self.members = {mm.name.lower(): mm for mm in gzarc.members(self.data)}
        data = gzarc.inflate(self.data, self.members[stem + ".cmp"])
        m = self.m
        m.mem[CCSDATA:CCSDATA + len(data)] = data
        # One stream at a time: the old one leaves ccStream's list with the
        # heap it lived on.
        m.store(self.sym("ccscRoot"), 4, 0)
        self.heap = HEAP0
        self.ccs = self.stream(CCSDATA)
        self.spccs, self.spccs_name, self.anm_chunks = 0, spccs, {}
        if spccs:
            data2 = gzarc.inflate(self.data, self.members[spccs + ".cmp"])
            # On the heap: past the first file's data lie the game's globals.
            at = self.malloc(m, len(data2) + 16)
            m.mem[at:at + len(data2)] = data2
            self.spccs = self.stream(at)
            for name, _ in ccs.Ccs(data2).objects:
                if name.startswith("ANM_"):
                    m.mem[ARGS:ARGS + len(name) + 1] = name.encode() + b"\0"
                    self.anm_chunks[self.call("GetChunkAdrsF__8ccStreamFPCci", self.spccs, ARGS, 0)] = name
        # field_eff, which the lakes' constructor keeps at +0x34c for their
        # fireflies.
        data3 = gzarc.inflate(self.data, self.members["field_eff.cmp"])
        at = self.malloc(m, len(data3) + 16)
        m.mem[at:at + len(data3)] = data3
        self.eff = self.stream(at)
        self.base = self.heap
        self.stem = (stem, spccs)

    def stream(self, data):
        s = self.malloc(self.m, 0x190)
        self.call("__ct__8ccStreamFP14ccStreamReader", s, 0)
        self.m.store(s + 0x160, 4, data)
        self.call("DecodeSetup__8ccStreamFv", s)
        return s

    # --- DUNGEON as its constructor and GO(2) leave it -----------------------
    def make(self, info, seeds, field_type, weather, level_max, room_max, event, code, server, hack=0):
        m = self.m
        self.load(info["file"], info.get("spccs", ""))
        self.heap = self.base
        self.call("initHitCheck1__Fv")
        m.mem[DG:DG + DG_SIZE] = bytes(DG_SIZE)
        m.mem[GIMPOS:GIMPOS + 750 * 0x30] = bytes(750 * 0x30)
        for i in range(750):
            m.store(GIMPOS + 0x30 * i + 0x22, 1, 0xFF)
        m.store(DG + 0x0, 4, code)
        m.store(DG + 0x10, 4, info["dtype"])
        m.store(DG + 0x14, 4, info["event"])
        m.store(DG + 0x4C, 4, 1)
        m.store(DG + 0x50, 4, info["fog"])
        m.store(DG + 0x144, 4, info["fog_va"])
        m.store(DG + 0x148, 4, info["fog_index"])
        m.store(DG + 0x424, 4, GIMPOS)
        m.store(DG + 0xD3884, 4, self.ccs)
        # The constructor's ishack (WORLD_MAN.hackFlag, +0xf0) and, in the
        # lakes, field_eff.
        m.store(DG + 0x40, 4, hack)
        if info["dtype"] in (8, 9):
            m.store(DG + 0x34C, 4, self.eff)
        for i in range(10):
            m.store(DG + 0xD3538 + 4 * i, 4, 15)
            m.store(DG + 0xD3560 + 4 * i, 4, 15)
        for i in range(150):
            m.store(DG + 0x30894 + 0x4C * i, 4, 10)
        # The constructor's realmap: every cell {d 0, next 15, here 15}.
        m.mem[DG + 0x430:DG + 0x430 + 10 * 0x4B00] = bytes((0, 15, 15)) * (10 * 6400)
        ed = 0
        if info["event"]:
            base = self.sym("EditDungeon")
            for k in range(88):
                if m.load(base + 24 * k, 4) == info["event"] and m.load(base + 24 * k + 4, 4) == code:
                    ed = base + 24 * k
            rows, n = m.load(ed + 8, 4), m.load(ed + 12, 4)
            m.mem[DG + 0x30894:DG + 0x30894 + 0x4C * n] = m.mem[rows:rows + 0x4C * n]
        m.store(DG + 0x3351C, 4, ed)
        m.mem[WM:WM + 0x4E0] = bytes(0x4E0)
        # The DUNGEON constructor's specialRoom -1; WORLD_MAN.dungeon[d].
        m.store(WM + 0x160, 4, M32)
        m.store(WM + 0x438 + 4 * code, 4, DG)
        for off, v in ((0x8, 2), (0x10, field_type), (0xC, weather), (0x120, event), (0x138, level_max),
                       (0x13C, room_max), (0x140, info["clut"]), (0x144, info["tex"]), (0x420, 0), (0x424, 0),
                       (0x428, fb(60000.0)), (0x42C, fb(60000.0)), (0xF0, hack)):
            m.store(WM + off, 4, v)
        for k, s in enumerate(seeds):
            m.store(WM + 0x48 + 4 * k, 4, s)
        m.mem[GAME:GAME + 0x88] = bytes(0x88)
        for off, v in ((0x14, 2), (0x18, 1), (0x1C, server), (0x24, event), (0x28, code)):
            m.store(GAME + off, 4, v)
        m.mem[SAVE:SAVE + 0x8530] = bytes(0x8530)
        # A new game's area bans: every entry -1.
        m.mem[SAVE + 0x6548:SAVE + 0x6548 + 128] = b"\xff" * 128
        m.mem[MISC:MISC + 0x1000] = bytes(0x1000)
        m.mem[PLAYER:PLAYER + 0x300] = bytes(0x300)
        # GO(2): seed = dungeonSeed[d], randcnt 0, then Generate.
        m.store(self.sym("seed"), 4, seeds[code])
        m.store(self.sym("randcnt"), 4, 0)
        self.fogs = []
        self.call("Generate__7DUNGEONFv", DG)
        self.call("GetStartPosition__7DUNGEONFPf", DG, WM + 0x20)
        self.hit_names()

    def hit_names(self):
        """Each ccModelHit of the decoded file's models (the index), then
        those of the ccObjs the room and door anms made: the controller's
        own object's name."""
        m = self.m
        self.names = {}
        anms = [m.load(DG + 0x2F230 + 4 * k, 4) for k in range(150)]
        anms += [m.load(DG + 0x3F4 + 4 * k, 4) for k in range(4)]
        # SetObject's anmobj[10] and SetAnmObject's anmobj2[10].
        anms += [m.load(DG + off + 4 * k, 4) for off in (0xD35B0, 0xD35D8) for k in range(10)]
        for anm in anms:
            if not anm:
                continue
            tbl, ch = m.load(anm + 0xAC, 4), m.load(anm + 0x94, 4)
            if not tbl or not ch:
                continue
            for i in range(m.load(ch + 0x20, 4)):
                e = tbl + 12 * i
                if m.load(e + 8, 2) != 0x100:
                    continue
                obj = m.load(e + 4, 4)
                idx, mdl = m.load(obj + 0x90, 4), m.load(obj + 0x94, 4)
                if mdl and m.load(mdl + 0x3C, 4):
                    # ccStream::Decode_ExtObj renames an ExtObj's index
                    # "EXT_...": the name table's is "OBJ_...".
                    name = self.cstr(idx + 8)
                    if name.startswith("EXT_"):
                        name = "OBJ_" + name[4:]
                    self.names[m.load(mdl + 0x3C, 4)] = name
        # SetDoor's ban block (DUNGEON+0xd3880) and SetObject's object[10]
        # (+0xd3588), ccClumps: their nodes' (+0x94, +0x98) models' hits
        # under the clump's name (its chunk's index, +0x90).
        for clump in [m.load(DG + 0xD3880, 4)] + [m.load(DG + 0xD3588 + 4 * k, 4) for k in range(10)]:
            if not clump:
                continue
            name = self.cstr(m.load(clump + 0x90, 4) + 8)
            for k in range(m.load(clump + 0x98, 2)):
                node = m.load(m.load(clump + 0x94, 4) + 4 * k, 4)
                mdl = m.load(node + 0x94, 4) if m.load(node + 0x8E, 2) == 0x100 else 0
                if mdl and m.load(mdl + 0x3C, 4):
                    self.names[m.load(mdl + 0x3C, 4)] = name

    def hit_list(self):
        m = self.m
        self.hit_names()
        out = []
        h = m.load(self.sym("ccModelHitTop"), 4)
        while h:
            out.append([self.names.get(h, hex(h)), m.load(h + 8, 4), self.rvec(h + 0x10, 16), self.rvec(h + 0x50, 16)])
            h = m.load(h + 4, 4)
        return out

    def chunk_name(self, chunk):
        """A chunk's name: its index (+0) holds it at +8."""
        if not chunk:
            return ""
        idx = self.m.load(chunk, 4)
        return self.cstr(idx + 8) if idx else "?"

    def own_clut(self, name):
        """The palette a fresh ccEff of the file's Eff `name` has (Init(chunk,
        1)'s +0x3c)."""
        m = self.m
        m.mem[ARGS:ARGS + len(name) + 1] = name.encode() + b"\0"
        chunk = self.call("GetChunkAdrsF__8ccStreamFPCci", self.ccs, ARGS, 0) & M32
        eff = self.malloc(m, 0x70)
        self.call("Init__5ccEffFP10ccEffChunki", eff, chunk, 1)
        return self.chunk_name(m.load(eff + 0x3C, 4))

    def dress(self):
        """What SetRoom stood in the room (docs/engine/dungeon.md, "The
        dressing"): water[0] (its anm and matrix; water 1 and 2 checked the
        same), fire[60] (base, pos, dir, life, cnt, the ccEff's place and
        transparency, and its palette), roomlight[32] (the glows' names,
        places and patterns, the omni light's place, colour and
        intensity), object[10], anmobj[10], anmobj2[10] (names and
        matrices), and fieldrand (seed, randcnt)."""
        m = self.m
        s32 = lambda v: v - (1 << 32) if v & 0x80000000 else v    # noqa: E731
        eff_name = lambda e: self.cstr(m.load(e + 0x44, 4) + 8) if e else ""    # noqa: E731
        water = None
        w = [m.load(DG + 0x364 + 4 * k, 4) for k in range(3)]
        if w[0]:
            water = [self.chunk_name(m.load(w[0] + 0x94, 4)), self.rvec(w[0] + 0x40, 16)]
            for x in w[1:]:
                assert [self.chunk_name(m.load(x + 0x94, 4)), self.rvec(x + 0x40, 16)] == water
        sparks, cluts = [], []
        for k in range(60):
            f = m.load(DG + 0x54 + 4 * k, 4)
            if not f:
                continue
            e = m.load(f + 0x14, 4)
            sparks.append([self.rvec(f + 0x20), self.rvec(f + 0x30), self.rvec(f + 0x40), s32(m.load(f, 4)),
                           s32(m.load(f + 8, 4)), self.rvec(e + 0x10), m.load(e + 0x34, 4)])
            cluts.append(self.chunk_name(m.load(e + 0x3C, 4)))
        lights = []
        for k in range(32):
            r = m.load(DG + 0x374 + 4 * k, 4)
            if not r:
                continue
            e, e2, light = m.load(r + 0x30, 4), m.load(r + 0x34, 4), m.load(r + 0x38, 4)
            lights.append([eff_name(e), self.rvec(r + 0x10), m.load(r, 4), eff_name(e2),
                           self.rvec(r + 0x20), m.load(r + 4, 4), self.rvec(light + 0x70, 3),
                           self.rvec(light + 0xB0, 3), m.load(light + 0xC0, 4)])
        objects = []
        for k in range(10):
            c = m.load(DG + 0xD3588 + 4 * k, 4)
            if c:
                objects.append([self.cstr(m.load(c + 0x90, 4) + 8), self.rvec(c + 0x40, 16)])
        anms = []
        for off in (0xD35B0, 0xD35D8):
            out = []
            for k in range(10):
                a = m.load(DG + off + 4 * k, 4)
                if a:
                    out.append([self.chunk_name(m.load(a + 0x94, 4)), self.rvec(a + 0x40, 16)])
            anms.append(out)
        # The lakes' FIREFLYs: base (+0x90; its w is what SetRoom left on
        # its stack) and pattern (+0x50).
        fireflies = []
        for k in range(5):
            f = m.load(DG + 0xD38A8 + 4 * k, 4)
            if f:
                fireflies.append([self.rvec(f + 0x90, 3), m.load(f + 0x50, 4)])
        return {"water": water, "sparks": sparks, "lights": lights, "objects": objects, "anmobj": anms[0],
                "anmobj2": anms[1], "fireflies": fireflies,
                "rng": [m.load(self.sym("seed"), 4), m.load(self.sym("randcnt"), 4)], "cluts": cluts}

    def draw_eff(self, n, player=None):
        """N frames of DUNGEON::DrawEff (0x005cdee0): each frame's ccEff::Draw
        calls recorded (the Eff's name, place, pattern, transparency), the
        room lights' intensities and fieldrand after it; `player` (x, y
        bits) is where the fireflies' ccTransPosW2P finds him (plw +0x40)."""
        m = self.m
        if player:
            for k, v in enumerate((player[0], player[1], 0, ONE)):
                m.store(PLAYER + 0x40 + 4 * k, 4, v)
        drawn = []
        name = lambda e: self.cstr(m.load(e + 0x44, 4) + 8)    # noqa: E731
        hooks = {
            "Draw__5ccEffFPfUs": lambda mm, e, pos, pat, *a: drawn.append(
                [name(e), self.rvec(pos), pat & 0xFFFF, m.load(e + 0x34, 4)]) or 0,
            "Draw__5ccEffFUs": lambda mm, e, pat, *a: drawn.append(
                [name(e), self.rvec(e + 0x10), pat & 0xFFFF, m.load(e + 0x34, 4)]) or 0,
        }
        for k, fn in hooks.items():
            m.hooks[self.sym(k)] = fn
        frames = []
        for _ in range(n):
            drawn.clear()
            self.call("DrawEff__7DUNGEONFv", DG)
            lights = []
            for k in range(32):
                r = m.load(DG + 0x374 + 4 * k, 4)
                if r:
                    lights.append(m.load(m.load(r + 0x38, 4) + 0xC0, 4))
            frames.append({"sprites": [list(x) for x in drawn], "lights": lights,
                           "rng": [m.load(self.sym("seed"), 4), m.load(self.sym("randcnt"), 4)]})
        for k in hooks:
            del m.hooks[self.sym(k)]
        return frames

    def draw_bg(self, here, n, names, mat):
        """N frames of DUNGEON::DrawBG(here) (0x005ce3d0) over the clumps the
        lakes' constructor makes (`new ccClump`, Init from the chunk,
        SetFogSw(0)) for `names`: each frame the halfword it leaves in
        `mat`'s substitute (+0x16) and each ccClump::Draw's matrix
        (+0x40, recorded; the draw itself stubbed)."""
        m = self.m
        for k, name in enumerate(names):
            m.mem[ARGS:ARGS + len(name) + 1] = name.encode() + b"\0"
            chunk = self.call("GetChunkAdrsF__8ccStreamFPCci", self.ccs, ARGS, 0) & M32
            c = self.malloc(m, 0xA0)
            self.call("__ct__7ccCoordFv", c)
            self.call("Init__7ccClumpFP12ccClumpChunk", c, chunk)
            self.call("SetFogSw__7ccClumpFi", c, 0)
            m.store(DG + 0x350 + 4 * k, 4, c)
        m.mem[ARGS:ARGS + len(mat) + 1] = mat.encode() + b"\0"
        subst = self.call("GetSubstAdrsF__8ccStreamFPCci", self.ccs, ARGS, 0) & M32
        drawn = []
        m.hooks[self.sym("Draw__7ccClumpFv")] = lambda mm, c, *a: drawn.append(self.rvec(c + 0x40, 16)) or 0
        frames = []
        for _ in range(n):
            drawn.clear()
            self.call("DrawBG__7DUNGEONFi", DG, here)
            frames.append([m.load(subst + 0x16, 2), [list(x) for x in drawn]])
        del m.hooks[self.sym("Draw__7ccClumpFv")]
        for k in range(5):
            m.store(DG + 0x350 + 4 * k, 4, 0)
        return frames

    def floors(self):
        """Each floor as the probe reports it: the model animIdx names, or
        for the Gott statue room (symFlag or lakeFlag, symFloor, symBlock) the one
        SetRoom's statue path plays (its table @4950, 0x00696aa0)."""
        m = self.m
        out = []
        # symFlag, or the lakes' lakeFlag (+0x44), picks the statue room.
        sym = (m.load(DG + 0x34, 4), m.load(DG + 0x38, 4)) if 1 in (m.load(DG + 0x30, 4), m.load(DG + 0x44, 4)) else None
        statue = self.cstr(m.load(inf_va(0x00696AA0) + 4 * m.load(DG + 0x10, 4), 4))
        for f in range(10):
            rooms = []
            for i in range(15):
                ai = DG + 0x30040 + f * 0x78 + i * 8
                k, r, tbl = m.load(ai, 1), m.load(ai + 1, 1), m.load(ai + 4, 4)
                name = ""
                if tbl:
                    row = tbl + 16 * k
                    name = self.cstr(m.load(m.load(row, 4) + 4 * r, 4))
                if (f, i) == sym:
                    name = statue
                pos = self.rvec(DG + 0x2F6E0 + f * 0xF0 + i * 16, 2)
                rot = m.load(DG + 0x2F488 + f * 60 + i * 4, 4)
                rooms.append([pos[0], pos[1], rot, name])
            start = [self.rvec(DG + 0x30590 + 0xA0 * s + 16 * f) for s in range(2)]
            out.append({"up": m.load(DG + 0xD3538 + 4 * f, 4), "down": m.load(DG + 0xD3560 + 4 * f, 4),
                        "start": start, "rooms": rooms})
        return out

    def room_type(self, f, i):
        """The type of the first story row of type 16 or more at (f, i), 0
        with none (the search MakeRoom, SetRoom and GotoNextRoom make)."""
        m = self.m
        for k in range(150):
            a = DG + 0x30894 + 0x4C * k
            if m.load(a, 4) == f and m.load(a + 4, 4) == i and m.load(a + 0x10, 4) >= 16:
                return m.load(a + 0x10, 4)
        return 0

    def set_room(self, f, i):
        self.call("ClearRoom__7DUNGEONFv", DG)
        # Nothing on the heap past the decoded file outlives ClearRoom (the
        # anms' destructors took their hits off the list).
        self.heap = self.base
        self.fogs = []
        self.call("SetRoom__7DUNGEONFii", DG, f, i)
        return self.m.load(DG + 0x20, 4)

    def land(self, x, y, z):
        m = self.m
        for k, v in enumerate((x, y, z, ONE)):
            m.store(ARGS + 4 * k, 4, v)
        self.call("ccLandHitCheck__FPfUi", ARGS, 0x20000001)
        zb = m.f[0] & M32
        near = self.sym("hitResultNearest")
        num = m.load(self.sym("hitResultNum"), 4)
        att = self.call("checkHitResultAttlibute__Fv") & M32
        # checkHitResultAttlibute takes the ground-type bits from the nearest
        # result that has them (not a shaded ground, not the "none" 0xe000e0)
        # and, with none such, from whatever $a2 holds: undefined here.
        res = self.sym("hitResult")
        own = any(m.load(res + 0x60 * k + 0x48, 4) & 0x40000 == 0
                  and m.load(res + 0x60 * k + 0x48, 4) & 0x00F0F0F0 != 0x00E000E0 for k in range(min(num, 32)))
        return {"z": zb, "num": num, "cp": self.rvec(near), "dist": m.load(near + 0x20, 4),
                "att": m.load(near + 0x48, 4), "attribute": att if own else None}

    def height(self, x, y):
        self.call("GetHeight__7DUNGEONFff", DG, fargs=(x, y))
        return self.m.f[0] & M32

    def draw(self, x, y):
        """DUNGEON::Draw (0x005ce930) with the player (plw) at (x, y): the
        cell's `here`, whether room[level][here] was drawn, and how many
        door anms MoveDoor drew. The minimap, the map, the layers, the
        door sounds and the dressing's DrawWater and DrawEff are stubbed;
        ccAnm::Draw records what it is given."""
        m = self.m
        drawn = []
        hooks = {
            "Draw__5ccAnmFv": lambda mm, anm, *a: drawn.append(anm) or 0,
            "Draw__7ccClumpFv": lambda mm, *a: 0,
            "SetActiveLayer__9WORLD_MANFi": lambda mm, *a: 0,
            "MakeMiniMap__7DUNGEONFi": lambda mm, *a: 0,
            "DrawMap__7DUNGEONFv": lambda mm, *a: 0,
            "SetPathFindingMap__Fv": lambda mm, *a: 0,
            "ccSeOn3D__FiPf": lambda mm, *a: 0,
            "Get2DPos__9WORLD_MANFPfPf": lambda mm, *a: 0,
            "GetRoom2DPos__7DUNGEONFPi": lambda mm, *a: 0,
            # The dressing's frame (test_draw_eff checks DrawEff).
            "DrawWater__7DUNGEONFv": lambda mm, *a: 0,
            "DrawEff__7DUNGEONFv": lambda mm, *a: 0,
        }
        for name, fn in hooks.items():
            m.hooks[self.sym(name)] = fn
        for k, v in enumerate((x, y, 0, ONE)):
            m.store(PLAYER + 0x40 + 4 * k, 4, v)
        self.call("Draw__7DUNGEONFv", DG)
        for name in hooks:
            del m.hooks[self.sym(name)]
        level = m.load(DG + 0x4, 4)
        cx, cy = int(bf(x) / 750), int(bf(y) / 750)
        here = m.load(DG + 0x430 + level * 0x4B00 + 3 * (80 * cx + cy) + 2, 1)
        room = m.load(DG + 0x2F230 + level * 60 + 4 * here, 4) if here < 15 else 0
        doors = [m.load(DG + 0x3F4 + 4 * k, 4) for k in range(4)]
        return {"here": here if here < 15 else -1, "room": int(bool(room) and room in drawn),
                "doors": sum(1 for d in doors if d and d in drawn)}

    # --- the doors (docs/engine/dungeon.md, "Doors") -------------------------
    def door_state(self):
        """CloseStart, lockNum, lockOff, doorAnm, doorFlag, stillOpenDoor;
        the door anms built; the hit list."""
        m = self.m
        words = [m.load(DG + off, 4) for off in (0x1C, 0x20, 0x24, 0x28, 0x2C, 0x48)]
        words = [w - (1 << 32) if w & 0x80000000 else w for w in words]
        doors = sum(1 for k in range(4) if m.load(DG + 0x3F4 + 4 * k, 4))
        return words, doors, self.hit_list()

    def active(self, clear_all, clear_here):
        """ccCheckActiveObject() and ccCheckActiveObject(f, i) answering as
        the case says (1 true: nothing of the entry control's there)."""
        m = self.m
        m.hooks[self.sym("ccCheckActiveObject__Fv")] = lambda mm, *a: int(clear_all)
        m.hooks[self.sym("ccCheckActiveObject__Fii")] = lambda mm, *a: int(clear_here)

    def room_c(self, f, i, clear):
        self.active(True, clear)
        self.call("ClearRoom__7DUNGEONFv", DG)
        self.heap = self.base
        self.fogs = []
        self.m.store(DG + 0x4, 4, f)
        self.call("SetRoom__7DUNGEONFii", DG, f, i)
        return self.door_state()

    def move_door(self, level, here, clear_all, clear_here):
        """DUNGEON::MoveDoor(here) (gcmn 0x005cd3d0) on floor `level`: the
        door sounds (ccSeOn3D, id and place) recorded, ccAnm::Draw
        stubbed."""
        m = self.m
        self.active(clear_all, clear_here)
        sounds = []
        hooks = {
            "ccSeOn3D__FiPf": lambda mm, se, v, *a: sounds.append((se, self.rvec(v, 3))) or 0,
            "Draw__5ccAnmFv": lambda mm, *a: 0,
        }
        for name, fn in hooks.items():
            m.hooks[self.sym(name)] = fn
        m.store(DG + 0x4, 4, level)
        self.call("MoveDoor__7DUNGEONFi", DG, here)
        for name in hooks:
            del m.hooks[self.sym(name)]
        return sounds, self.door_state()

    def room_2d_pos(self, level, block):
        """DUNGEON::GetRoom2DPos (gcmn 0x005cf080) for game.block `block` on
        floor `level`."""
        m = self.m
        m.store(DG + 0x4, 4, level)
        m.store(GAME + 0x30, 4, block)
        for k in range(3):
            m.store(ARGS + 4 * k, 4, 0xDEADBEEF)
        self.call("GetRoom2DPos__7DUNGEONFPi", DG, ARGS)
        return [m.load(ARGS + 4 * k, 4) - (1 << 32) if m.load(ARGS + 4 * k, 4) & 0x80000000
                else m.load(ARGS + 4 * k, 4) for k in range(3)]

    def room_select(self, f, i):
        """WORLD_MAN::RoomSelect's dungeon part: ClearRoom, SetRoom(f, i), then
        DUNGEON::RoomSelect(&WORLD_MAN.position, f, i). WORLD_MAN.position, level,
        roomEnterFlag, mapHideFlag and ccMenu+0x10 after it."""
        m = self.m
        m.store(DG + 0x4C, 4, 0)
        m.store(DG + 0x42C, 4, 0)
        m.store(MENU + 0x10, 2, 0)
        for k, v in enumerate((0xDEADBEEF,) * 4):
            m.store(WM + 0x20 + 4 * k, 4, v)
        self.set_room(f, i)
        self.call("RoomSelect__7DUNGEONFPfii", DG, WM + 0x20, f, i)
        return {"position": self.rvec(WM + 0x20), "level": m.load(DG + 0x4, 4), "enter": m.load(DG + 0x4C, 4),
                "hide": m.load(DG + 0x42C, 4), "menu": m.load(MENU + 0x10, 2)}

    def enter(self, x, y, z, level, floor, block):
        """WORLD_MAN::Enter (main 0x0019dda0) in area 2 from (x, y, z): the
        ChangeArea / ChangeScene calls it makes (ChangeRequest not run, so
        every call it makes runs)."""
        m = self.m
        m.store(DG + 0x4, 4, level)
        m.store(GAME + 0x2C, 4, floor)
        m.store(GAME + 0x30, 4, block)
        for k, v in enumerate((x, y, z, ONE)):
            m.store(ARGS + 4 * k, 4, v)
        self.changes = []
        self.call("Enter__9WORLD_MANFPf", WM, ARGS)
        return self.changes

    def goto(self, x, y, z, level, floor, block):
        m = self.m
        m.store(DG + 0x4, 4, level)
        m.store(GAME + 0x2C, 4, floor)
        m.store(GAME + 0x30, 4, block)
        for k, v in enumerate((x, y, z, ONE)):
            m.store(ARGS + 4 * k, 4, v)
        ret = self.call("GotoNextRoom__7DUNGEONFPfPf", DG, WM + 0x20, ARGS)
        ret = ret - (1 << 32) if ret & 0x80000000 else ret
        return ret, self.rvec(WM + 0x20)


def cases():
    """(seeds, field type, weather, levelMax, roomMax, hack, field, dungeon,
    server): story area 14 as the port's --mode dungeon:14 enters it, other
    story areas (four of them with flag 3, the "E" types 4-7), and random
    dungeons of the four plain types."""
    rng = random.Random(4)
    out = [((154874216, 3996388677, 1814669918), 10, 0, 4, 9, 2, 14, 0, 0)]
    for ev, ft, hack in ((17, 0, 2), (22, 7, 2), (29, 1, 2), (58, 2, 2), (83, 3, 2), (30, 0, 3), (31, 1, 3),
                         (32, 2, 3), (33, 7, 3)):
        out.append(((rng.getrandbits(32), rng.getrandbits(32), rng.getrandbits(32)), ft, 0, 3, 7, hack, ev, 0, 0))
    for ft in (7, 1, 0, 2, 10, 5, 8):
        for server in (0, 2):
            out.append(((rng.getrandbits(32), rng.getrandbits(32), rng.getrandbits(32)), ft, 0,
                        rng.choice((2, 3, 4)), rng.choice((5, 7, 9)), 2, 0, rng.choice((0, 1)), server))
    return out


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class DungeonAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()
        cls.probe = Probe()
        cls.game = Game()
        cls.counts = {}

    @classmethod
    def tearDownClass(cls):
        cls.probe.close()
        print("\n" + " ".join(f"{k} {v}" for k, v in sorted(cls.counts.items())), file=sys.stderr)

    def count(self, k, n=1):
        self.counts[k] = self.counts.get(k, 0) + n

    def same_hits(self, mine, game, what, tol=0.05):
        """The same owners in the same order, each with the same type, rm
        and im; a piece posed by a keyed rotation (a door's swinging leaf)
        may differ in the last bits (piney_data::anim interpolates keyed
        rotations in double precision, the game in its own arithmetic):
        within 1e-5 in the rotation and 0.05 in the translation, counted
        apart, with the largest difference in ULPs."""
        self.assertEqual([h[0] for h in mine], [h[0] for h in game], what + ": owners")
        for a, b in zip(mine, game):
            self.assertEqual(a[1], b[1], f"{what}: {a[0]} type")
            if a[2:] == b[2:]:
                self.count("hit models exact")
                continue
            for u, v in zip(a[2:], b[2:]):
                for k, (x, y) in enumerate(zip(u, v)):
                    tol_ = tol if k >= 12 else 1e-5
                    self.assertLessEqual(abs(bf(x) - bf(y)), tol_, f"{what}: {a[0]} word {k}")
            ulps = max(abs(ordered(x) - ordered(y)) for u, v in zip(a[2:], b[2:]) for x, y in zip(u, v))
            self.count("hit models near (keyed rotation)")
            self.counts["largest ULP difference"] = max(self.counts.get("largest ULP difference", 0), ulps)

    def same_dress(self, mine, game, what):
        """The dressing SetRoom stood in the room, piece by piece, to the bit,
        and fieldrand after it; each spark's palette as ccEff::ChangeClut
        left it (the port asks for (new, old); the game changes it only
        while it is old)."""
        g, mine = dict(game), dict(mine)
        cluts, want = g.pop("cluts"), mine.pop("clut")
        for key in ("water", "objects", "anmobj", "anmobj2", "sparks", "lights", "fireflies", "rng"):
            self.assertEqual(mine[key], g[key], f"{what}: dressing {key}")
        if cluts:
            own = self.game.own_clut("EFF_o_spark_s0_")
            expect = want[0] if want and own == want[1] else own
            self.assertEqual(cluts, [expect] * len(cluts), f"{what}: the sparks' palette")
            if expect != own:
                self.count("sparks with their palette changed", len(cluts))
        for key in ("sparks", "lights", "objects", "anmobj", "anmobj2", "fireflies"):
            self.count(f"dressing {key}", len(mine[key]))
        if mine["water"]:
            self.count("dressing water")

    def test_dungeons(self):
        g, p = self.game, self.probe
        rng = random.Random(7)
        for seeds, ft, weather, lmax, rmax, hack, field, code, server in cases():
            what = f"field {field} type {ft} dungeon {code} server {server} seeds {seeds}"
            info = p.ask(f"new {seeds[0]} {seeds[1]} {seeds[2]} {ft} {weather} {lmax} {rmax} {hack} {field} "
                         f"{code} {server}")
            if "error" in info:
                self.count("skipped (port error)")
                print(what, info["error"], file=sys.stderr)
                continue
            if info["dtype"] >= 8 or info["event"] in SPECIAL_EVENTS:
                self.count("skipped (lake or special area)")
                continue
            g.make(info, seeds, ft, weather, lmax, rmax, field, code, server)
            self.count("dungeons")
            self.count(f"dungeons of type {info['dtype']} ({'story' if info['event'] else 'random'})")
            # Generate: every floor's rooms, stairs and startpos.
            floors = g.floors()
            for f, (mine, game) in enumerate(zip(info["floors"], floors)):
                self.assertEqual((mine["up"], mine["down"]), (game["up"], game["down"]), f"{what} floor {f}")
                self.assertEqual(mine["start"], game["start"], f"{what} floor {f} startpos")
                for i, (a, b) in enumerate(zip(mine["rooms"], game["rooms"])):
                    if not a[4]:
                        continue
                    self.assertEqual(a[:4], b, f"{what} floor {f} room {i}")
                    self.count("rooms made")
            self.assertEqual(info["position"], g.rvec(WM + 0x20), what + ": GetStartPosition")
            fog, amb = g.fogs[-2], g.fogs[-1]
            self.assertEqual(fog, (info["near"], info["far"], 0, info["max"], info["fog"]), what + ": SetFog")
            self.assertEqual(list(amb), info["ambient"], what + ": SetAmbient")
            self.same_hits(info["hits"], g.hit_list(), what + ": SetRoom(0, 0)")
            if info["event"]:
                # A story dungeon's Generate draws for its gimmicks (SetAllGim),
                # which the port does not: its fieldrand starts from the game's.
                p.ask("rng {} {}".format(*g.dress()["rng"]))
                self.count("story dungeons' fieldrand taken from the game")
            else:
                self.same_dress(info["dress"], g.dress(), what + ": SetRoom(0, 0)")
            # Every room: SetRoom, its hits, the ground in it, GetHeight.
            story = {}
            if info["event"]:
                for i in range(150):
                    a = DG + 0x30894 + 0x4C * i
                    fl, idx, rt = g.m.load(a, 4), g.m.load(a + 4, 4), g.m.load(a + 0x10, 4)
                    if fl < 10:
                        story[(fl, idx)] = rt
            for f, fl in enumerate(info["floors"]):
                for i, room in enumerate(fl["rooms"]):
                    if not room[4] or story.get((f, i), 0) >= 15:
                        continue
                    mine = p.ask(f"room {f} {i}")
                    doors = g.set_room(f, i)
                    self.assertEqual(mine["doors"], doors, f"{what} room {f}/{i}: doors")
                    self.same_hits(mine["hits"], g.hit_list(), f"{what} room {f}/{i}")
                    self.same_dress(mine["dress"], g.dress(), f"{what} room {f}/{i}")
                    self.count("SetRoom")
                    self.assertEqual(p.ask(f"info {f} {i}")["info"], g.room_2d_pos(f, i),
                                     f"{what} room {f}/{i}: GetRoom2DPos")
                    self.count("GetRoom2DPos")
                    cx, cy = bf(room[0]), bf(room[1])
                    boxes = [(bf(h[2][12]), bf(h[2][13])) for h in mine["hits"]]
                    for k in range(40):
                        if k % 2 and boxes:
                            bx, by = rng.choice(boxes)
                            x, y = fb(bx + rng.uniform(-400, 400)), fb(by + rng.uniform(-400, 400))
                        else:
                            x = fb(cx + rng.uniform(-6200, 6200))
                            y = fb(cy + rng.uniform(-6200, 6200))
                        z = fb(rng.choice((0.0, rng.uniform(-300, 700))))
                        a = p.ask(f"land {x:x} {y:x} {z:x}")
                        b = g.land(x, y, z)
                        if b["attribute"] is None:
                            a["attribute"] = None
                            self.count("checkHitResultAttlibute without ground bits")
                        if b["num"] == 0:
                            # A miss leaves the last query's nearest result
                            # behind, and checkHitResultAttlibute with no
                            # result takes its ground bits from whatever $a2
                            # holds: only the height and the count mean
                            # anything.
                            for k in ("attribute", "cp", "dist", "att"):
                                del a[k], b[k]
                            self.count("ccLandHitCheck misses")
                        self.assertEqual(a, b, f"{what} room {f}/{i}: land at {bf(x)}, {bf(y)}, {bf(z)}")
                        self.count("ccLandHitCheck")
                    for _ in range(3):
                        x, y = fb(rng.uniform(0, 60000)), fb(rng.uniform(0, 60000))
                        self.assertEqual(p.ask(f"height {x:x} {y:x}")["h"], g.height(x, y), "GetHeight")
                        self.count("GetHeight")
            # GotoNextRoom from each door cell and from the stairs rooms.
            for f, fl in enumerate(floors):
                if f >= len(info["floors"]):
                    break
                cells = self.cells(g, f)
                for i, room in enumerate(info["floors"][f]["rooms"]):
                    if not room[4] or story.get((f, i), 0) >= 15:
                        continue
                    mine_cells = [c for c in cells if c[2] == i]
                    doors = [c for c in mine_cells if c[3]]
                    plain = [c for c in mine_cells if not c[3]]
                    picks = doors[:]
                    if i in (fl["up"], fl["down"]) and plain:
                        picks.append(rng.choice(plain))
                    for x, y, _, d, nxt in picks:
                        if not d and i not in (fl["up"], fl["down"]):
                            continue
                        if d and (nxt >= 15 or story.get((f, nxt), 0) >= 15 or not info["floors"][f]["rooms"][nxt][4]):
                            continue
                        if not d and i == fl["down"] and (f + 1 >= len(info["floors"])
                                                          or story.get((f + 1, info["floors"][f + 1]["up"]), 0) >= 15):
                            continue
                        if not d and i == fl["up"] and f > 0 and story.get((f - 1, fl["down"]), 0) >= 15:
                            continue
                        px = fb(x * 750 + rng.uniform(1, 749))
                        py = fb(y * 750 + rng.uniform(1, 749))
                        pz = fb(rng.choice((0.0, 12.5)))
                        p.ask(f"room {f} {i}")
                        g.set_room(f, i)
                        where = f"{what} floor {f} room {i} cell ({x}, {y})"
                        self.assertEqual(p.ask(f"drawn {px:x} {py:x}"), g.draw(px, py), where + ": Draw before")
                        mine = p.ask(f"goto {px:x} {py:x} {pz:x} {f} {f} {i}")
                        ret, pos = g.goto(px, py, pz, f, f, i)
                        self.assertEqual((mine["ret"], mine["position"]), (ret, pos), where)
                        self.same_hits(mine["hits"], g.hit_list(), where + ": the room built")
                        self.count("GotoNextRoom " + ("door" if d else "stairs"))
                        # DUNGEON::Draw after it, at the old spot (the new
                        # room not reached yet), at the new one, and at
                        # random points of the floor.
                        if ret not in (15, -1):
                            spots = [(px, py), (pos[0], pos[1])]
                            spots += [(fb(rng.uniform(0, 60000)), fb(rng.uniform(0, 60000))) for _ in range(2)]
                            for sx, sy in spots:
                                self.assertEqual(p.ask(f"drawn {sx:x} {sy:x}"), g.draw(sx, sy),
                                                 f"{where}: Draw at {bf(sx)}, {bf(sy)}")
                                self.count("DUNGEON::Draw")

    def test_lake_dressing(self):
        """Field type 4's lake dungeons (type 8, sd4), which test_dungeons
        leaves out: Generate's rooms and fieldrand, then every room's hit
        list and dressing - SetObject's statues and flowers (ccClumps at the
        OBJ_0ps0*-3* dummies of ANM_sd4l1n31-l4n31, their hits) and the water
        of ANM_sd4m0n80. No room of the file has an OBJ_0ps4* dummy (the
        fieldrand(4) pick); the fireflies are not made (GetTime is not 2
        here)."""
        g, p = self.game, self.probe
        for seeds, server in (((2898712818, 1158677336, 1446943725), 0), ((4210609577, 3044698653, 1706924747), 0),
                              ((2152778680, 1235723455, 47882419), 0), ((1249418419, 1660063002, 287017642), 1)):
            what = f"lake seeds {seeds} server {server}"
            info = p.ask(f"new {seeds[0]} {seeds[1]} {seeds[2]} 4 0 3 7 2 0 0 {server}")
            self.assertEqual(info["dtype"], 8, what)
            g.make(info, seeds, 4, 0, 3, 7, 0, 0, server)
            for f, (mine, game) in enumerate(zip(info["floors"], g.floors())):
                self.assertEqual((mine["up"], mine["down"]), (game["up"], game["down"]), f"{what} floor {f}")
                for i, (a, b) in enumerate(zip(mine["rooms"], game["rooms"])):
                    if a[4]:
                        self.assertEqual(a[:4], b, f"{what} floor {f} room {i}")
            self.same_hits(info["hits"], g.hit_list(), what + ": SetRoom(0, 0)")
            self.same_dress(info["dress"], g.dress(), what + ": SetRoom(0, 0)")
            for f, fl in enumerate(info["floors"]):
                for i, room in enumerate(fl["rooms"]):
                    if not room[4]:
                        continue
                    mine = p.ask(f"room {f} {i}")
                    g.set_room(f, i)
                    self.same_hits(mine["hits"], g.hit_list(), f"{what} room {f}/{i}")
                    self.same_dress(mine["dress"], g.dress(), f"{what} room {f}/{i}")
                    self.count("lake SetRoom")

    def test_lake_night(self):
        """The lakes by night (field type 4, bgnum 2: GetTime 2; type 8, sd4)
        and hacked (hackFlag 3, by day and by night; with WORLD_MAN's event
        14 type 9, sd9, which holds bgnum 0's clumps alone): in every
        room SetRoom's five FIREFLYs (base, pattern, fieldrand after them);
        in the first rooms DrawEff's frames with them (FIREFLY::Move and
        Draw, their sparks, the player at the room's centre and then far
        enough off for them to be put back about him) and DrawBG's scroll
        and matrices."""
        g, p = self.game, self.probe
        for seeds, weather, hack, field, server in (((2898712818, 1158677336, 1446943725), 2, 2, 0, 0),
                                                    ((4210609577, 3044698653, 1706924747), 2, 2, 0, 1),
                                                    ((2152778680, 1235723455, 47882419), 0, 3, 0, 0),
                                                    ((1249418419, 1660063002, 287017642), 2, 3, 0, 1),
                                                    ((2152778680, 1235723455, 47882419), 0, 3, 14, 0),
                                                    ((1249418419, 1660063002, 287017642), 0, 3, 14, 0)):
            # sd9 last: decoding it a second time after sd4 has DecodeSetup
            # read a bad pointer (0x7c7c7b84) in the interpreter.
            what = f"lake seeds {seeds} bg {weather} hack {hack} event {field}"
            info = p.ask(f"new {seeds[0]} {seeds[1]} {seeds[2]} 4 {weather} 3 7 {hack} {field} 0 {server}")
            dtype = info["dtype"]
            self.assertEqual(dtype, 9 if field == 14 else 8, what)
            g.make(info, seeds, 4, weather, 3, 7, field, 0, server, hack=hack)
            # DrawBG's scroll (v$8917) as the lake's first DrawBG finds it.
            g.m.store(g.sym("v$8917"), 4, 0)
            g.m.store(g.sym("init$8918"), 1, 1)
            self.same_hits(info["hits"], g.hit_list(), what + ": SetRoom(0, 0)")
            self.same_dress(info["dress"], g.dress(), what + ": SetRoom(0, 0)")
            if dtype == 8:
                names = [f"CMP_o_{n}{weather}_" for n in ("bac_l", "bac_m", "clo_l", "clo_m")]
                mat = f"MAT_sfp7bac{weather + 1}"
            else:
                names = [f"CMP_o_{n}{weather}_" for n in ("bac_l", "bac_m", "ero_l", "ero_m", "ero_s")]
                mat = f"MAT_sfp9dat{weather + 1}_3"
            drawn = 0
            for f, fl in enumerate(info["floors"]):
                for i, room in enumerate(fl["rooms"]):
                    if not room[4]:
                        continue
                    where = f"{what} room {f}/{i}"
                    mine = p.ask(f"room {f} {i}")
                    g.set_room(f, i)
                    self.same_dress(mine["dress"], g.dress(), where)
                    self.assertEqual(len(mine["dress"]["fireflies"]), 5, where)
                    self.count("lake SetRoom by night or hacked")
                    if drawn >= 2:
                        continue
                    drawn += 1
                    x, y = room[0], room[1]
                    far = fb(bf(x) + 4000.0)
                    for px, n in ((x, 60), (far, 20)):
                        game = g.draw_eff(n, (px, y))
                        for k, (a, b) in enumerate(zip(p.ask(f"draweff {n} {px:x} {y:x}")["frames"], game)):
                            self.assertEqual(a, b, f"{where}: DrawEff frame {k} (player {bf(px)}, {bf(y)})")
                            self.count("lake DrawEff frames")
                    mine = p.ask(f"drawbg {i} 40")
                    self.assertEqual(mine["clumps"], len(names), where)
                    zero = lambda v: 0 if v == 0x80000000 else v    # noqa: E731
                    for k, (a, b) in enumerate(zip(mine["frames"], g.draw_bg(i, 40, names, mat))):
                        self.assertEqual(a[0], b[0], f"{where}: DrawBG frame {k}: {mat}'s V")
                        self.assertEqual(len(b[1]), len(names), where)
                        for mm in b[1]:
                            self.assertEqual([zero(v) for v in a[1]], [zero(v) for v in mm],
                                             f"{where}: DrawBG frame {k}: the matrix")
                        self.count("lake DrawBG frames")

    def test_draw_eff(self):
        """DUNGEON::DrawEff over the rooms with sparks or glows of the random
        dungeons of types 0-3, 70 frames each: the sparks' moves (SNOW::Move,
        fieldrand), their fading, the glows' patterns and places, each room
        light's intensity, every frame."""
        g, p = self.game, self.probe
        for seeds, ft, weather, lmax, rmax, hack, field, code, server in cases():
            info = p.ask(f"new {seeds[0]} {seeds[1]} {seeds[2]} {ft} {weather} {lmax} {rmax} {hack} {field} "
                         f"{code} {server}")
            if "error" in info or info["dtype"] >= 8 or info["event"]:
                continue
            g.make(info, seeds, ft, weather, lmax, rmax, field, code, server)
            rooms = 0
            for f, fl in enumerate(info["floors"]):
                for i, room in enumerate(fl["rooms"]):
                    if not room[4] or rooms >= 4:
                        continue
                    mine = p.ask(f"room {f} {i}")
                    g.set_room(f, i)
                    if not mine["dress"]["sparks"] and not mine["dress"]["lights"]:
                        continue
                    rooms += 1
                    what = f"field {field} type {info['dtype']} room {f}/{i}"
                    game = g.draw_eff(70)
                    for k, (a, b) in enumerate(zip(p.ask("draweff 70")["frames"], game)):
                        self.assertEqual(a, b, f"{what}: DrawEff frame {k}")
                        self.count("DrawEff frames")
                    self.count("DrawEff rooms")

    def test_room_select(self):
        """The events' `room` (WORLD_MAN::RoomSelect): for rooms of every
        dungeon, the room built (its hit list), WORLD_MAN.position 200 in front
        of the first gate wall or door (or the centre), level, roomEnterFlag;
        and the game's mapHideFlag 2 and ccMenu map status 3, which the port
        sets as constants."""
        g, p = self.game, self.probe
        rng = random.Random(41)
        for seeds, ft, weather, lmax, rmax, hack, field, code, server in cases():
            info = p.ask(f"new {seeds[0]} {seeds[1]} {seeds[2]} {ft} {weather} {lmax} {rmax} {hack} {field} "
                         f"{code} {server}")
            if "error" in info or info["dtype"] >= 8 or info["event"] in SPECIAL_EVENTS:
                continue
            g.make(info, seeds, ft, weather, lmax, rmax, field, code, server)
            story = {}
            if info["event"]:
                for i in range(150):
                    a = DG + 0x30894 + 0x4C * i
                    fl_, idx, rt = g.m.load(a, 4), g.m.load(a + 4, 4), g.m.load(a + 0x10, 4)
                    if fl_ < 10:
                        story[(fl_, idx)] = rt
            rooms = [(f, i) for f, fl in enumerate(info["floors"]) for i, r in enumerate(fl["rooms"])
                     if r[4] and story.get((f, i), 0) < 15]
            rng.shuffle(rooms)
            picks = rooms[:8]
            if (0, 0) in rooms and (0, 0) not in picks:
                picks.append((0, 0))
            for f, i in picks:
                what = f"field {field} type {info['dtype']} room {f}/{i}"
                mine = p.ask(f"select {f} {i}")
                game = g.room_select(f, i)
                self.assertEqual((game["hide"], game["menu"]), (2, 3), what + ": the map's flags")
                self.assertEqual((mine["position"], mine["level"], mine["enter"]),
                                 (game["position"], game["level"], game["enter"]), what)
                self.same_hits(mine["hits"], g.hit_list(), what + ": the room built")
                self.count("RoomSelect")
                centre = info["floors"][f]["rooms"][i][:2] == game["position"][:2]
                self.count("RoomSelect at the centre" if centre else "RoomSelect in front of a way in")

    def test_special_rooms(self):
        """The story dungeons' event rooms: every floor's rooms as MakeRoom
        leaves them (the event rooms' own Anime chunks from spccs); then for
        each row of type 16 or more, GotoNextRoom from each door into it,
        with the room free and then banned (CheckAreaBan on the branch's
        area): the answer, WORLD_MAN.position, specialRoom, roomEnterFlag,
        the room built (its hit list), the event room's SetFog and
        SetAmbient, area 66's SetAreaBan; the same door through
        WORLD_MAN::Enter (the ChangeArea or ChangeScene it asks for, the
        first of them); and RoomSelect into each event room."""
        p = self.probe
        rng = random.Random(53)
        for field, code in SPECIAL_DUNGEONS:
            g = Game()
            seeds = (rng.getrandbits(32), rng.getrandbits(32), rng.getrandbits(32))
            what = f"area {field} dungeon {code}"
            info = p.ask(f"new {seeds[0]} {seeds[1]} {seeds[2]} 1 0 3 7 2 {field} {code} 0")
            if "error" in info:
                self.count("special dungeons skipped (port error)")
                print(what, info["error"], file=sys.stderr)
                continue
            g.make(info, seeds, 1, 0, 3, 7, field, code, 0)
            self.count("special dungeons")
            # MakeRoom: an event room's animIdx names the table row's model;
            # its own Anime chunk is SetRoom's (checked below), and spccs is
            # the file MakeRoom loaded.
            self.assertEqual(g.m.load(DG + 0xD3888, 4), g.spccs, what + ": spccs")
            event_rooms = []
            for f, (mine, game) in enumerate(zip(info["floors"], g.floors())):
                self.assertEqual((mine["up"], mine["down"]), (game["up"], game["down"]), f"{what} floor {f}")
                for i, (a, b) in enumerate(zip(mine["rooms"], game["rooms"])):
                    if a[4] and g.room_type(f, i) >= 25:
                        self.assertEqual(a[:3], b[:3], f"{what} floor {f} room {i}")
                        event_rooms.append((f, i, a[3]))
                    elif a[4]:
                        self.assertEqual(a[:4], b, f"{what} floor {f} room {i}")
            self.assertEqual(info["position"], g.rvec(WM + 0x20), what + ": GetStartPosition")
            for f, i, name in event_rooms:
                where = f"{what}: SetRoom({f}, {i})"
                mine = p.ask(f"room {f} {i}")
                g.set_room(f, i)
                anm = g.m.load(DG + 0x2F230 + f * 60 + 4 * i, 4)
                self.assertEqual(name, g.anm_chunks.get(g.m.load(anm + 0x94, 4)), where + ": the Anime chunk")
                self.same_hits(mine["hits"], g.hit_list(), where)
                fog = p.ask("fog")
                self.assertEqual(tuple(fog["fog"]), [x for x in g.fogs if len(x) == 5][-1], where + ": SetFog")
                amb = [x for x in g.fogs if len(x) == 3]
                if amb:
                    self.assertEqual(fog["ambient"], list(amb[-1]), where + ": SetAmbient")
                self.count(f"event rooms of type {g.room_type(f, i)} set")
            rows = sorted({(f, i) for f in range(10) for i in range(15) if g.room_type(f, i) >= 16})
            bans = SPECIAL_BANS.get(field, field)
            for f, s in rows:
                if f >= len(info["floors"]):
                    continue
                ways = [c for c in self.cells(g, f) if c[3] and c[4] == s]
                if not ways:
                    self.count("event rows no door leads to")
                # A row with no room (types 16-24 and 34) that the area's
                # branch does not take: the game's SetRoom reads through
                # its null anm (docs/engine/dungeon.md, "Event rooms").
                ty = g.room_type(f, s)
                if (16 <= ty <= 24 or ty == 34) and field not in LEAVE_AREAS:
                    self.count("roomless rows walked into by the door path (skipped)")
                    continue
                for banned in (False, True):
                    if banned:
                        p.ask(f"ban {bans} {code} {f} {s}")
                        g.call("SetAreaBan__10ccSaveDataFiiii", SAVE, bans, code, f, s)
                    for x, y, here, _, _ in ways[:3]:
                        where = f"{what} floor {f}: room {here} into {s} ({g.room_type(f, s)}) banned {banned}"
                        px = fb(x * 750 + rng.uniform(1, 749))
                        py = fb(y * 750 + rng.uniform(1, 749))
                        p.ask(f"room {f} {here}")
                        g.set_room(f, here)
                        mine = p.ask(f"goto {px:x} {py:x} 0 {f} {f} {here}")
                        ret, pos = g.goto(px, py, 0, f, f, here)
                        special = g.m.load(WM + 0x160, 4)
                        special = special - (1 << 32) if special & 0x80000000 else special
                        self.assertEqual((mine["ret"], mine["special"], mine["enter"]),
                                         (ret, special, g.m.load(DG + 0x4C, 4)), where)
                        if ret not in (-100, -255):
                            self.assertEqual(mine["position"], pos, where + ": the arrival")
                        self.same_hits(mine["hits"], g.hit_list(), where + ": the room built")
                        self.count(f"GotoNextRoom into an event room answering {ret}")
                        if any(h[0] == "CMP_o_block_m0_" for h in mine["hits"]):
                            self.count("rooms built with the ban block")
                        if field == 66:
                            game_bans = [g.m.load(SAVE + 0x6548 + k, 1) for k in range(128)]
                            self.assertEqual(p.ask("bans")["bans"], game_bans, where + ": the bans")
                        if ret == s:
                            self.count("event rooms entered")
                        # WORLD_MAN::Enter through the same door.
                        p.ask(f"room {f} {here}")
                        g.set_room(f, here)
                        mine = p.ask(f"enter {px:x} {py:x} 0 {f} {f} {here}")["exit"]
                        game = g.enter(px, py, 0, f, f, here)
                        self.assertEqual(mine, list(game[0]) if game else None, where + ": WORLD_MAN::Enter")
                        self.count("WORLD_MAN::Enter")
            for f, s in rows:
                if f >= len(info["floors"]) or not info["floors"][f]["rooms"][s][4]:
                    continue
                where = f"{what}: RoomSelect({f}, {s})"
                mine = p.ask(f"select {f} {s}")
                game = g.room_select(f, s)
                special = g.m.load(WM + 0x160, 4)
                special = special - (1 << 32) if special & 0x80000000 else special
                self.assertEqual((mine["position"], mine["level"], mine["enter"], mine["special"]),
                                 (game["position"], game["level"], game["enter"], special), where)
                self.same_hits(mine["hits"], g.hit_list(), where + ": the room built")
                self.count("RoomSelect into an event room")

    def same_doors(self, mine, game, what):
        words, doors, hits = game
        self.assertEqual(mine["state"], words, what + ": door words")
        self.assertEqual(mine["doors"], doors, what + ": doors")
        self.same_hits(mine["hits"], hits, what)

    def test_doors(self):
        """SetDoor with the room busy or clear, then frames of MoveDoor with
        the entry control empty or not, and the events' OpenDoor,
        CloseDoor and CloseDoor2 in between: the door words, the doors and
        the hit list (the leaves as they swing, sink and come off) after
        every step, and the doors' sounds (how many, where)."""
        g, p = self.game, self.probe
        rng = random.Random(31)
        for seeds, ft, weather, lmax, rmax, hack, field, code, server in cases():
            info = p.ask(f"new {seeds[0]} {seeds[1]} {seeds[2]} {ft} {weather} {lmax} {rmax} {hack} {field} "
                         f"{code} {server}")
            if "error" in info or info["dtype"] >= 8 or info["event"] in SPECIAL_EVENTS:
                continue
            g.make(info, seeds, ft, weather, lmax, rmax, field, code, server)
            self.count(f"door dungeons of type {info['dtype']}")
            rooms = [(f, i) for f, fl in enumerate(info["floors"]) for i, r in enumerate(fl["rooms"]) if r[4]]
            story = {}
            if info["event"]:
                for i in range(150):
                    a = DG + 0x30894 + 0x4C * i
                    fl_, idx, rt = g.m.load(a, 4), g.m.load(a + 4, 4), g.m.load(a + 0x10, 4)
                    if fl_ < 10:
                        story[(fl_, idx)] = rt
            rooms = [r for r in rooms if story.get(r, 0) < 15]
            rng.shuffle(rooms)
            for f, i in rooms[:6]:
                what = f"field {field} type {info['dtype']} room {f}/{i}"
                clear = rng.random() < 0.5
                self.same_doors(p.ask(f"roomc {f} {i} {int(clear)}"), g.room_c(f, i, clear), what + " SetRoom")
                if not g.door_state()[1]:
                    continue
                self.count("rooms with doors")
                # A script: busy or clear frames, the events' door calls.
                steps = []
                for _ in range(rng.choice((2, 3, 4))):
                    op = rng.choice(("frames", "frames", "open", "close", "close2", "close2"))
                    if op == "frames":
                        steps += [("move", rng.random() < 0.5, rng.random() < 0.5)] * rng.randrange(5, 70)
                    elif op == "open":
                        steps.append(("open",))
                    elif op == "close":
                        steps.append(("close",))
                    else:
                        steps.append(("close2",))
                        # The closing played out with the room busy, then
                        # cleared (the fight won).
                        steps += [("move", False, False)] * 70 + [("move", True, True)] * 70
                for n, st in enumerate(steps):
                    where = f"{what} step {n} {st}"
                    if st[0] == "move":
                        mine = p.ask(f"movedoor {f} {i} {int(st[1])} {int(st[2])}")
                        sounds, game = g.move_door(f, i, st[1], st[2])
                        self.assertEqual([s_[1:] for s_ in mine["sounds"]], [s_[1] for s_ in sounds],
                                         where + ": the doors' sounds")
                        self.count("MoveDoor")
                        self.count("door sounds", len(sounds))
                        if game[0][0]:
                            self.count("MoveDoor while a door closes")
                        if st[1] and not game[0][5]:
                            self.count("MoveDoor opening")
                    elif st[0] == "open":
                        mine = p.ask(f"opendoor {f} {i}")
                        g.call("OpenDoor__7DUNGEONFii", DG, f, i)
                        game = g.door_state()
                        self.count("OpenDoor")
                    elif st[0] == "close":
                        mine = p.ask(f"closedoor {f} {i}")
                        g.call("CloseDoor__7DUNGEONFii", DG, f, i)
                        game = g.door_state()
                        self.count("CloseDoor")
                    else:
                        mine = p.ask("closedoor2")
                        g.call("CloseDoor2__7DUNGEONFii", DG, f, i)
                        game = g.door_state()
                        self.count("CloseDoor2")
                    self.same_doors(mine, game, where)

    def test_near_door(self):
        """GetNearDoorPosition (the boss rooms' warnings' places) on rooms
        SetRoom builds, as SetItemBox builds each row's: each of the port's
        marks of a room given back as `in` must come back as `out` (every
        mark's place checked), and random places near the room's marks give
        the port's pick (none with no marks, `out` then `in`; one farther
        than 12,000 leaves the game's stack words)."""
        g, p = self.game, self.probe
        rng = random.Random(37)
        buf_in, buf_out = SCRATCH + 0xF000, SCRATCH + 0xF010
        fn = "GetNearDoorPosition__7DUNGEONFPfPfii"

        def game_near(f, i, at):
            for k in range(4):
                g.m.store(buf_in + 4 * k, 4, at[k])
            ret = g.call(fn, DG, buf_out, buf_in, f, i)
            return ret, g.rvec(buf_out)

        for seeds, ft, weather, lmax, rmax, hack, field, code, server in cases():
            info = p.ask(f"new {seeds[0]} {seeds[1]} {seeds[2]} {ft} {weather} {lmax} {rmax} {hack} {field} "
                         f"{code} {server}")
            if "error" in info or info["event"] in SPECIAL_EVENTS:
                continue
            g.make(info, seeds, ft, weather, lmax, rmax, field, code, server)
            rooms = [(f, i) for f, fl in enumerate(info["floors"]) for i, r in enumerate(fl["rooms"]) if r[4]]
            rng.shuffle(rooms)
            for f, i in rooms[:8]:
                what = f"field {field} type {info['dtype']} room {f}/{i}"
                if 16 <= g.room_type(f, i) < 24:
                    continue
                g.set_room(f, i)
                marks = p.ask(f"doormarks {f} {i}")["marks"]
                self.assertIsNotNone(marks, what + ": the port builds no room")
                if not marks:
                    at = [fb(rng.uniform(-30000, 30000)), fb(rng.uniform(-30000, 30000)), 0, ONE]
                    ret, out = game_near(f, i, at)
                    self.assertEqual((ret, out), (0, at), what + ": no marks")
                    self.count("rooms without marks")
                    continue
                self.count("rooms with marks")
                for mk in marks:
                    ret, out = game_near(f, i, mk)
                    self.assertEqual((ret, out[:3]), (1, mk[:3]), what + f": the mark {mk}")
                    self.count("marks")
                for _ in range(6):
                    mk = rng.choice(marks)
                    at = [fb(bf(mk[0]) + rng.uniform(-15000, 15000)), fb(bf(mk[1]) + rng.uniform(-15000, 15000)),
                          fb(bf(mk[2]) + rng.uniform(-500, 500)), ONE]
                    mine = p.ask(f"neardoor {f} {i} {at[0]} {at[1]} {at[2]} {at[3]}")["pos"]
                    ret, out = game_near(f, i, at)
                    self.assertEqual(ret, 1, what)
                    if mine is None:
                        self.count("places farther than 12000")
                        continue
                    self.assertEqual(out[:3], mine[:3], what + f": from {at}")
                    self.count("places near a mark")

    @staticmethod
    def cells(g, f):
        """(x, y, here, d, next) of every room cell of floor f."""
        m = g.m
        out = []
        base = DG + 0x430 + f * 0x4B00
        raw = bytes(m.mem[base:base + 0x4B00])
        for x in range(80):
            for y in range(80):
                d, nxt, here = raw[3 * (80 * x + y):3 * (80 * x + y) + 3]
                if here != 15:
                    out.append((x, y, here, d, nxt))
        return out



@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class EventDataAgainstGame(unittest.TestCase):
    def test_set_event_data(self):
        """WORLD_MAN::SetEventData (main 0x001a3c40) for every story area
        with an EditDungeon entry, others and none, in a dungeon and a
        field, with the caller's $f20 random: the ccEvent::SetEventPoint
        and SetEventPos calls in order (recorded) and WORLD_MAN's warp
        points (+0x170: the point's x, y, z and the room word at +0x10)."""
        build()
        p = Probe()
        g = Game()
        m = g.m
        rng = random.Random(17)
        calls = []
        m.hooks[g.sym("SetEventPoint__7ccEventFiii")] = lambda mm, ev, f, b, n: calls.append(
            ("point", [f - (1 << 32) if f & 0x80000000 else f, b, n])) or 0
        m.hooks[g.sym("SetEventPos__7ccEventFiiifPf")] = lambda mm, ev, f, b, n: calls.append(
            ("pos", [f, b, n, mm.f[12] & M32] + g.rvec(mm.r[8], 3))) or 0
        base = g.sym("EditDungeon")
        events = sorted({m.load(base + 24 * k, 4) for k in range(88)})
        cases = [(e, 2) for e in events] + [(e, 1) for e in events[:10]] + [(0, 2), (1, 2), (200, 2), (-5, 2)]
        counts = {"areas": 0, "points": 0, "positions": 0, "warps": 0}
        for event, area in cases:
            m.mem[WM:WM + 0x4E0] = bytes(0x4E0)
            m.store(WM + 0x120, 4, event & M32)
            m.mem[GAME:GAME + 0x88] = bytes(0x88)
            m.store(GAME + 0x14, 4, area)
            f20 = fb(rng.uniform(-3.2, 3.2))
            m.f[20] = f20
            calls.clear()
            g.call("SetEventData__9WORLD_MANFv", WM)
            mine = p.ask(f"eventdata {event} {area} {f20:x}")
            want_points = [c[1] for c in calls if c[0] == "point"]
            want_pos = [c[1] for c in calls if c[0] == "pos"]
            self.assertEqual([c[0] for c in calls], ["point"] * len(want_points) + ["pos"] * len(want_pos),
                             f"area {event}: points before positions")
            self.assertEqual(mine["points"], want_points, f"area {event} in {area}: points")
            self.assertEqual(mine["positions"], want_pos, f"area {event} in {area}: positions")
            warps = []
            for k in range(20):
                a = WM + 0x170 + 0x20 * k
                room = m.load(a + 0x10, 4)
                if room or any(m.load(a + 4 * j, 4) for j in range(3)):
                    warps.append([k] + g.rvec(a, 3) + [room])
            self.assertEqual(sorted(mine["warps"]), sorted(warps), f"area {event} in {area}: warps")
            counts["areas"] += 1
            counts["points"] += len(want_points)
            counts["positions"] += len(want_pos)
            counts["warps"] += len(warps)
        # ccEvent::SetEventPoint and SetEventPos themselves (run here, not
        # recorded) on random slot states: the first free slot takes each.
        del m.hooks[g.sym("SetEventPoint__7ccEventFiii")]
        del m.hooks[g.sym("SetEventPos__7ccEventFiiifPf")]
        ev = MISC
        for case in range(300):
            pts = [rng.choice((-1, -1, rng.randrange(0, 30))) for _ in range(16)]
            qs = [rng.choice((-1, -1, rng.randrange(0, 30))) for _ in range(16)]
            m.mem[ev:ev + 0x800] = bytes(0x800)
            for k in range(16):
                m.store(ev + 0x3C0 + 8 * k, 2, 0xFFFF)
                m.store(ev + 0x3C2 + 8 * k, 2, 0xFFFF)
                m.store(ev + 0x3C4 + 8 * k, 4, pts[k] & M32)
                m.store(ev + 0x1C0 + 32 * k, 2, 0xFFFF)
                m.store(ev + 0x1C2 + 32 * k, 2, 0xFFFF)
                m.store(ev + 0x1C4 + 32 * k, 4, qs[k] & M32)
            ops = []
            for _ in range(rng.randrange(1, 20)):
                f_, b_, n_ = rng.randrange(-2, 12), rng.randrange(-2, 20), rng.randrange(-1, 40)
                if rng.random() < 0.5:
                    ops.append(f"p:{f_}:{b_}:{n_}")
                    g.call("SetEventPoint__7ccEventFiii", ev, f_ & M32, b_ & M32, n_ & M32)
                else:
                    d, x, y, z = (fb(rng.uniform(-4, 4)), fb(rng.uniform(0, 60000)), fb(rng.uniform(0, 60000)),
                                  fb(rng.uniform(-300, 300)))
                    ops.append(f"q:{f_}:{b_}:{n_}:{d:x}:{x:x}:{y:x}:{z:x}")
                    for k, v in enumerate((x, y, z, 0)):
                        m.store(ARGS + 4 * k, 4, v)
                    g.call("SetEventPos__7ccEventFiiifPf", ev, f_ & M32, b_ & M32, n_ & M32, ARGS, fargs=(d,))
            mine = p.ask("slots " + " ".join(map(str, pts + qs)) + " " + " ".join(ops))
            s16 = lambda v: v - (1 << 16) if v & 0x8000 else v  # noqa: E731
            s32 = lambda v: v - (1 << 32) if v & 0x80000000 else v  # noqa: E731
            want_p = [[s16(m.load(ev + 0x3C0 + 8 * k, 2)), s16(m.load(ev + 0x3C2 + 8 * k, 2)),
                       s32(m.load(ev + 0x3C4 + 8 * k, 4))] for k in range(16)]
            want_q = [[s16(m.load(ev + 0x1C0 + 32 * k, 2)), s16(m.load(ev + 0x1C2 + 32 * k, 2)),
                       s32(m.load(ev + 0x1C4 + 32 * k, 4))] + g.rvec(ev + 0x1C8 + 32 * k, 1)
                      + g.rvec(ev + 0x1D0 + 32 * k, 4) for k in range(16)]
            self.assertEqual(mine["points"], want_p, f"slots case {case}: points")
            self.assertEqual(mine["positions"], want_q, f"slots case {case}: positions")
            counts["slot cases"] = counts.get("slot cases", 0) + 1
        p.close()
        print("\n" + " ".join(f"{k} {v}" for k, v in sorted(counts.items())), file=sys.stderr)


class Walk:
    """tools/test_world_rs.py's Field (Kite, the camera, their tasks) on the
    machine a Game built a room on: game.area 2, WORLD_MAN's dungeon."""

    def __init__(self, g, info, code):
        import anim
        import ccs
        import test_world_rs as tw
        self.tw = tw
        f = tw.Field.__new__(tw.Field)
        f.prog, f.m, f.sym = g.prog, g.m, g.sym
        f.malloc = g.malloc
        f.members, f.archive = g.members, g.data
        import gzarc
        kite = ccs.Ccs(gzarc.inflate(g.data, g.members["ctu1body.cmp"]))
        f.anims = {kite.objects[a.object][0]: a for _, a in anim.animations(kite)}
        f.setup_globals()
        self.f, self.g, self.info, self.code = f, g, info, code
        f.m.hooks[g.sym("Enter__9WORLD_MANFPf")] = lambda mm, *a: f.events.append("enter") or 0

    def start(self, pos, dircz, scheme, mode, seed, floor, block):
        f, tw, m = self.f, self.tw, self.f.m
        f.start(pos, dircz, scheme, mode, seed)
        # ccSetupGameCtrl's scene and GO(2)'s WORLD_MAN.
        for off, v in ((0x14, 2), (0x18, 2), (0x24, self.info["event"]), (0x28, self.code), (0x2C, floor),
                       (0x30, block)):
            m.store(tw.GAME + off, 4, v)
        for off, v in ((0x8, 2), (0x420, 0), (0x424, 0), (0x428, fb(60000.0)), (0x42C, fb(60000.0)),
                       (0x438 + 4 * self.code, DG)):
            m.store(tw.WM + off, 4, v)
        # ccPlayer::ccPlayer in a dungeon: standing (act 2) at once, his
        # body on the character list; WORLD_MAN::SetCenter at his feet.
        p = tw.PLAYER
        m.store(p + 0xEE, 2, 2)
        m.store(p + 0xF0, 2, 2)
        m.store(p + 0xE0, 1, m.load(p + 0xE0, 1) & ~0x04)
        m.store(p + 0x100, 2, 0)
        f.call("HitEnable__9ccCharHitFv", p + 0x1A0)
        f.call("SetCenter__7DUNGEONFff", DG, fargs=(pos[0], pos[1]))

    def frame(self, pad):
        f = self.f
        f.pad(*pad)
        s = f.frame()
        s["enter"] = int("enter" in f.events)
        return s


def walk_pads(kind, n, rng):
    """[(direct, push, powL, dircL, powR, dircR, pow[12])]: "door" pushes
    straight ahead (up) after a moment; "walls" leans a new way every 25-60
    frames, with the camera turned now and then."""
    import test_world_rs as tw
    pads = []
    lx = ly = rx = ry = 128
    for i in range(n):
        if kind == "door":
            lx, ly = (128, 0) if i >= 5 else (128, 128)
            rx = ry = 128
        elif i % rng.choice((25, 40, 60)) == 0:
            lx, ly = rng.choice(((128, 0), (255, 128), (0, 128), (128, 255), (40, 40), (220, 30), (128, 128),
                                  (128, 70)))
            rx, ry = rng.choice(((128, 128), (128, 128), (255, 128), (0, 128), (128, 40)))
        dl, pl = tw.stick(lx, ly)
        dr, pr = tw.stick(rx, ry)
        pads.append((0, 0, pl, dl, pr, dr, [0] * 12))
    return pads


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class KiteInDungeon(unittest.TestCase):
    def test_frames(self):
        import test_world_rs as tw
        build()
        counts = {}
        rng = random.Random(23)
        seeds, ft, weather, lmax, rmax, hack, field, code, server = cases()[0]
        # (floor, room, pads, scheme, cameraMode, heading): room 0's north
        # door from the start; room 1's south door (back to room 0) facing
        # it (pi faces +y).
        runs = [(0, 0, "door", 0, 3, None), (0, 1, "walls", 1, 3, None), (0, 3, "walls", 2, 3, None),
                (1, 1, "walls", 3, 3, None), (0, 0, "walls", 0, 3, None), (0, 4, "walls", 1, 3, None),
                (1, 2, "walls", 2, 1, None), (0, 1, "door", 3, 3, 0x40490FDB)]
        for f_, i, kind, scheme, mode, heading in runs:
            p = Probe()
            g = Game()
            info = p.ask(f"new {seeds[0]} {seeds[1]} {seeds[2]} {ft} {weather} {lmax} {rmax} {hack} {field} "
                         f"{code} {server}")
            g.make(info, seeds, ft, weather, lmax, rmax, field, code, server)
            p.ask(f"room {f_} {i}")
            g.set_room(f_, i)
            w = Walk(g, info, code)
            fl = info["floors"][f_]
            if (f_, i) == (0, 0):
                pos = info["position"][:3]
                dircz = info["position"][3]
            else:
                room = fl["rooms"][i]
                pos, dircz = [room[0], room[1], 0], fb(rng.uniform(-3.1, 3.1)) if heading is None else heading
            p.ask("start " + tw.hexs(*pos, dircz, scheme, mode, 1))
            w.start(pos, dircz, scheme, mode, 1, f_, i)
            enters = 0
            for n, pad in enumerate(walk_pads(kind, 240 if kind == "door" else 600, rng)):
                d, push, pl, dl, pr, dr, pw = pad
                mine = p.ask("pad " + tw.hexs(d, push, pl, dl, pr, dr, *pw))
                want = w.frame(pad)
                diff = [k for k in want if want[k] != mine[k]]
                self.assertFalse(diff, "floor %d room %d %s frame %d: %s\n game %s\n port %s" % (
                    f_, i, kind, n, diff, {k: want[k] for k in diff}, {k: mine[k] for k in diff}))
                counts["frames"] = counts.get("frames", 0) + 1
                counts["moving"] = counts.get("moving", 0) + want["move_flag"]
                enters += want["enter"]
                if want["enter"]:
                    break
            counts["WORLD_MAN::Enter"] = counts.get("WORLD_MAN::Enter", 0) + enters
            if kind == "door":
                self.assertEqual(enters, 1, "walked through the door")
            p.close()
        print("\n" + " ".join(f"{k} {v}" for k, v in sorted(counts.items())), file=sys.stderr)


if __name__ == "__main__":
    unittest.main()
