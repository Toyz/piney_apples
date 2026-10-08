#!/usr/bin/env python3
"""crates/piney-world's Grunties (piney_world::grunty: ccSetChibiGuso,
ccPGuso) against the game's own code (gcmn pgbreed.cpp) run in
tools/eemu.py (the Rust eemu_rs.so when present), frame by frame.

The grunty_probe example builds a Grunty over Dun Loireag's collision and
steps it on the frames this sends; the game's side is ccPGuso::ccPGuso on
the entry setDog (gcmn 0x0050ef10) builds, then each frame the menu's
affect (the affectFunc called as ccChar::EntryAffect would: affectType,
its arguments, the caller) when there is one, ccEntryObj::routine and
ccPGuso::main (0x0050b3c0), over town02's Hit chunk decoded by the game's
ccStream::Decode_Hit and registered as a ccModelHit. setDog's and
ccSetChibiGuso's placing runs natively too (`set`).

Compared each frame: the act, its old value, proccess, count and anmNum;
position, posP, heading; the body's touch flag and count and the stuck
count; nextPos, oldPos and the route's place; the scale; growthNum,
msgNum, dmylevel, pgScale; the Grunty's growth record and the save's;
evolevel, evonum, evocnt, evoflag, exist, chatFlag, foodMode, chatcnt;
transrate; camFlag, camPos, camView; spdeg and speed; adultType, gpSub;
the ground attribute and transparency; each anm's animation and time;
inuCount; which affect function it has; and in order what it asked for -
ccSeSetParamInu (the note's param, the Grunty inuCheckNote reads through
pgPtr, its place and ground), ccSeOn3D, effSmoke (place, velocity, scale,
its two counts), effEvolvePG and effGrowPG, ccVoiceRequest and
ccEvVoiceStop, ccChatMsg::OpenChat (which line) and CloseChat,
changeCamera, cameraSetPos and cameraSetView, Kite put at his place,
deleteCmnd(1); the notes the anms handed on; the anms drawn (by ccChar::Draw
or ccAnm::Draw) with their alpha and whether the matrix had the scale.

What runs in Python in place of the game: the anm (SetAnm by name,
_AnimateForward by tools/anim.py's playback, NoteProcess handing the
passed notes to the game's own inuCheckNote), the draws and the requests
above (recorded), rand() (newlib's LCG seeded as the probe seeds it),
ccCheckVoiceGrp (its row, so the voices compare as rows). The ground
attribute is not compared once a grown one runs off (evoActAdult's
phases 6 on): runAway reads checkHitResultAttlibute without a ground
query of its own, so the results of whatever query ran last and, when none
is near, a register's leftover.

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
import volume  # noqa: E402  # PINEY_VOLUME's disc
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

import test_world_rs as tw  # noqa: E402
from test_world_rs import (CAMID, CCSYS, DATA, DRAWENV, DRAWENV_ACTIVE, ELF, GAME, GAME_P, HEAP, HEAP_END,  # noqa: E402
                           IDX, ISO, LAYER_ACTIVE, ONE, PI_BITS, K180, PLAYER, PLW_PW, ROOT, SAVE, SAVEDATA,
                           SCRATCH, STREAM, SYS, TCAM, VIEW, WM, WORLDMAN, Rand, fb, hexs)

EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "grunty_probe")
ACTIVE_CAM = inf_va(0x0037896C)
RT = 0x01032000
NOTE_LIST = 0x0104E000      # the notes for inuCheckNote, 0-terminated
NOTES = 0x0104E100          # their ccAnmNote records
FILL = 0x0104EF00           # the hook NoteProcess's stand-in calls first
INU_CHECK_NOTE = inf_va(0x0050FEC0)
if volume.NAME != "infection":
    # From Mutation on the dogs have an inuCheckNote of their own (on
    # inuPtr); the Grunties' (on pgPtr) is the function after
    # dogActionAdult (MUT gcmn 0x0052cb80).
    _adult = volume.program(volume.ELF).symbol_named("dogActionAdult__FP6ccChar")
    INU_CHECK_NOTE = (_adult.value + _adult.size + 15) & ~15
FILES = ["cdogboda", "cdogbodb", "cdogbod0"] + ["cdogbod%d" % k for k in range(2, 10)]


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                    "grunty_probe"], cwd=ROOT, check=True)


def ask(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True,
                       cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


def note_routine():
    """ccAnm::NoteProcess's stand-in: the hook at FILL puts the anm's notes
    in NOTE_LIST, then inuCheckNote(note) for each, as the game's hands
    each note of the list to the anm's note function (always inuCheckNote
    for a Grunty's anms)."""
    hi, lo = NOTE_LIST >> 16, NOTE_LIST & 0xFFFF
    return [
        0x27BDFFE0,                         # addiu sp, sp, -32
        0xFFBF0010,                         # sd    ra, 16(sp)
        0xFFB00000,                         # sd    s0, 0(sp)
        0x0C000000 | (FILL >> 2),           # jal   FILL (a0 the anm)
        0x00000000,                         # nop
        0x3C100000 | hi,                    # lui   s0, hi
        0x36100000 | lo,                    # ori   s0, s0, lo
        0x8E040000,                         # loop: lw a0, 0(s0)
        0x10800005,                         # beqz  a0, done
        0x00000000,                         # nop
        0x0C000000 | (INU_CHECK_NOTE >> 2),  # jal   inuCheckNote
        0x26100004,                         # addiu s0, s0, 4 (delay slot)
        0x1000FFFA,                         # b     loop
        0x00000000,                         # nop
        0xDFBF0010,                         # done: ld ra, 16(sp)
        0xDFB00000,                         # ld    s0, 0(sp)
        0x03E00008,                         # jr    ra
        0x27BD0020,                         # addiu sp, sp, 32
    ]


class GruntyGame:
    """ccPGuso in the interpreter over town02's collision."""

    def __init__(self, town_no=1):
        import anim
        import ccs
        import gzarc
        from image import Program
        from test_anim import machine_class
        import test_save_init_rs
        self.prog = Program(ELF, "gcmn")
        self.m = machine_class()(self.prog)
        m = self.m
        self.sym = lambda n: self.prog.symbol_named(n).value     # noqa: E731
        sym = self.sym
        self.heap = HEAP
        for name in ("ccMalloc__FUi", "__nw__FUi"):
            m.hooks[sym(name)] = self.malloc
        for name in ("ccFree__FPv", "__dl__FPv"):
            m.hooks[sym(name)] = lambda mm, *a: 0
        data = gzarc.open_bytes(DATA)
        members = {mm.name.lower(): mm for mm in gzarc.members(data)}
        town = ccs.Ccs(gzarc.inflate(data, members["town02.cmp"]))
        self.town = town
        self.anims, self.in_file = {}, {}
        for f in FILES:
            c = ccs.Ccs(gzarc.inflate(data, members[f + ".cmp"]))
            for _, a in anim.animations(c):
                n = c.objects[a.object][0]
                self.anims.setdefault(n, a)
                self.in_file.setdefault(f, set()).add(n)
        self.dummies = {}
        for off, t, _n, _end in town.chunks():
            if t is None or t & 0xFFFF not in (0x1300, 0x1400):
                continue
            obj = struct.unpack_from("<I", town.data, off + 8)[0]
            pos = list(struct.unpack_from("<3I", town.data, off + 12))
            rot = list(struct.unpack_from("<3I", town.data, off + 24)) if t & 0xFFFF == 0x1400 else None
            self.dummies.setdefault(town.objects[obj][0], (pos, rot))
        self.fresh = test_save_init_rs.fresh_save(ELF)
        self.town_no = town_no
        self.globals()
        self.hooks()
        self.decode_hit()

    # --- memory ------------------------------------------------------------
    def malloc(self, m, n, *_):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        assert self.heap < HEAP_END
        m.mem[a:a + n] = bytes(n)
        return a

    def vec(self, a, v):
        for i, x in enumerate(v):
            self.m.store(a + 4 * i, 4, x)

    def rvec(self, a, n=4):
        return [self.m.load(a + 4 * i, 4) for i in range(n)]

    def call(self, name, *args, fargs=()):
        for i, v in enumerate(fargs):
            self.m.f[12 + i] = v
        return self.m.call(self.sym(name) if isinstance(name, str) else name, list(args), limit=50_000_000)

    def cstr(self, a):
        from eemu import _cstr
        return _cstr(self.m, a).decode("latin-1")

    # --- the world ---------------------------------------------------------
    def globals(self):
        m = self.m
        m.store(CCSYS, 4, SYS)
        m.store(SYS + 604, 4, SCRATCH)
        m.store(SYS + 12, 2, 512)
        m.store(SYS + 14, 2, 448)
        m.store(SYS + 20, 4, 0x3F955555)
        m.store(SAVEDATA, 4, SAVE)
        m.mem[SAVE:SAVE + 0x8530] = self.fresh
        m.store(GAME_P, 4, GAME)
        m.mem[GAME:GAME + 0x40] = bytes(0x40)
        m.store(GAME + 0x14, 4, 0)                 # area: a town
        m.store(GAME + 0x20, 4, self.town_no)      # town
        m.store(WORLDMAN, 4, WM)
        m.store(WM + 0x430, 4, RT)
        m.store(RT + 0x1A4, 4, STREAM)
        for off, v in ((0x420, -24000.0), (0x424, -24000.0), (0x428, 24000.0), (0x42C, 24000.0)):
            m.store(WM + off, 4, fb(v))
        m.store(DRAWENV_ACTIVE, 4, DRAWENV)
        m.store(PLW_PW, 4, PLAYER)
        m.mem[PLAYER:PLAYER + 0x300] = bytes(0x300)
        m.store(ACTIVE_CAM, 4, TCAM)
        m.store(CAMID, 2, 1)
        layer = self.malloc(m, 0x100)
        m.store(layer + 44, 4, VIEW)
        m.store(LAYER_ACTIVE, 4, layer)
        m.store(self.sym("ccChat"), 4, self.malloc(m, 0x100))
        m.store(self.sym("font"), 4, self.malloc(m, 0x100))

    def hooks(self):
        m, sym = self.m, self.sym
        nop = lambda mm, *a: 0                     # noqa: E731
        for name in ("SetFogBlend__9ccDrawEnvFfUi", "ResetFogBlend__9ccDrawEnvFv", "ccSpcConditionEffectSW__Fv",
                     "GetAmbient__9ccDrawEnvFPf", "SetAmbient__9ccDrawEnvFPf", "SleepDistantLight__9WORLD_MANFv",
                     "AwakeDistantLight__9WORLD_MANFv", "ccEntryCmnd__FP6ccChar", "SetActiveLayer__9WORLD_MANFi",
                     "Init__7ccClumpFP12ccClumpChunk", "__ct__7ccCoordFv", "SetFogSw__5ccAnmFi",
                     "SetFogSw__7ccClumpFi", "ApplyClump__5ccAnmFP7ccClumpP8ccStream"):
            m.hooks[sym(name)] = nop
        m.hooks[sym("rand")] = lambda mm, *a: self.rand.next()
        self.names, self.chunks, self.anm, self.streams = {}, {}, {}, {}
        self.events, self.notes_seen, self.drawn = [], [], []
        self.scaled = {}
        m.hooks[sym("GetChunkAdrsF__8ccStreamFPCci")] = self.chunk
        m.hooks[sym("GetCCSAdrs__8ccStreamFPCc")] = self.ccs_adrs
        def anm_ct(mm, a, *r):
            # ccCoord::ccCoord's +0x84 and +0x88 (the alpha) 1.
            mm.store(a + 0x84, 4, ONE)
            mm.store(a + 0x88, 4, ONE)
            return a
        m.hooks[sym("__ct__5ccAnmFv")] = anm_ct
        m.hooks[sym("SetAnm__5ccAnmFP10ccAnmChunkUi")] = self.set_anm
        m.hooks[sym("_AnimateForward__5ccAnmFUi")] = self.forward
        m.hooks[sym("GetObjAdrsF__7ccClumpFPCciP19ccSubstSearchResult")] = lambda mm, *a: 0
        np_ = sym("NoteProcess__5ccAnmFv")
        for i, wd in enumerate(note_routine()):
            m.store(np_ + 4 * i, 4, wd)
        m.hooks.pop(np_, None)
        m.hooks[FILL] = self.fill_notes
        m.hooks[sym("Draw__5ccAnmFv")] = self.anm_draw
        m.hooks[sym("SetMatrix_PosRotZYXScale__7ccCoordFPfPfPf")] = self.set_matrix(True)
        m.hooks[sym("SetMatrix_PosRotZYX__7ccCoordFPfPf")] = self.set_matrix(False)
        ev = self.events.append
        m.hooks[sym("ccSeSetParamInu__FUiP6ccChar")] = lambda mm, p, ch, *a: ev(
            ["note", p, self.rvec(ch + 0x40), mm.load(ch + 0x80, 4)]) or 0
        m.hooks[sym("ccSeOn3D__FiPf")] = lambda mm, n, pos, *a: ev(["se3d", n, self.rvec(pos)]) or 0

        def smoke(mm, pos, v, a2, a3, *rest):
            ev(["smoke", self.rvec(pos), self.rvec(v), mm.f[12], a2, a3])
            return 0
        m.hooks[sym("effSmoke__FPfPffiiUsUs")] = smoke
        # The effects read the character's place and its base's height.
        m.hooks[sym("effEvolvePG__FP6ccChar")] = lambda mm, ch, *a: ev(
            ["evolve", self.rvec(ch + 0x40), mm.load(mm.load(ch, 4) + 0x18, 4)]) or 0
        m.hooks[sym("effGrowPG__FP6ccChar")] = lambda mm, ch, *a: ev(
            ["grow", self.rvec(ch + 0x40), mm.load(mm.load(ch, 4) + 0x18, 4)]) or 0
        m.hooks[sym("ccCheckVoiceGrp__Fi")] = lambda mm, row, *a: row
        m.hooks[sym("ccVoiceRequest__Fii")] = lambda mm, g, n, *a: ev(
            ["voice", struct.unpack("<i", struct.pack("<I", g))[0], n]) or 0
        m.hooks[sym("ccEvVoiceStop__Fv")] = lambda mm, *a: ev(["voicestop"]) or 0
        chat_tbl = [m.load(inf_va(0x005EE650) + 4 * i, 4) for i in range(16)]
        m.hooks[sym("OpenChat__9ccChatMsgFP6ccCharPc")] = lambda mm, c, ch, text, *a: ev(
            ["chat", chat_tbl.index(text)]) or 0
        m.hooks[sym("CloseChat__9ccChatMsgFv")] = lambda mm, *a: ev(["chatclose"]) or 0

        def change(mm, n, *a):
            mm.store(CAMID, 2, n)
            ev(["camera", n])
            return 0
        m.hooks[sym("changeCamera__Fi")] = change
        m.hooks[sym("cameraSetPos__FPfi")] = lambda mm, v, *a: ev(["campos", self.rvec(v)]) or 0
        m.hooks[sym("cameraSetView__FPfi")] = lambda mm, v, *a: ev(["camview", self.rvec(v)]) or 0

        def delete_cmnd(mm, obj, flg, *a):
            if flg:
                ev(["dropped"])
            return 0
        m.hooks[sym("deleteCmnd__10ccEntryObjFi")] = delete_cmnd
        m.hooks[sym("MakePacketStr__8ccSpriteFPci")] = lambda mm, *a: ev(["debug"]) or 0
        m.hooks[sym("MakeNum__6ccFontFiiUl")] = lambda mm, *a: 0
        m.hooks[sym("SetType__6ccFontFi")] = lambda mm, *a: 0

        def set_dog(mm, n, *a):
            self.placed.append(n)
            return 0
        self.set_dog_hook = set_dog

    def chunk(self, mm, stream, name, *a):
        s = self.cstr(name)
        if s not in self.chunks:
            addr = self.malloc(mm, 0x40)
            if s in self.dummies:
                from eemu import f_div, f_mul
                pos, rot = self.dummies[s]
                self.vec(addr + 16, pos + [ONE])
                if rot is not None:
                    self.vec(addr + 32, [f_div(f_mul(PI_BITS, r), K180) for r in rot])
            self.chunks[s] = addr
            self.names[addr] = s
        if s.startswith("ANM_") and stream in self.streams:
            assert s in self.in_file.get(self.streams[stream], set()), (s, self.streams[stream])
        return self.chunks[s]

    def ccs_adrs(self, mm, name, *a):
        s = self.cstr(name).lower()
        for addr, f in self.streams.items():
            if f == s:
                return addr
        addr = self.malloc(mm, 0x100)
        self.streams[addr] = s
        return addr

    def set_anm(self, mm, anm, chunk, *a):
        self.anm[anm] = [self.names[chunk], 0, []]
        mm.store(anm + 0xAC, 4, IDX)
        mm.store(anm + 0x9C, 2, 256)
        mm.store(anm + 0x98, 4, 0)
        mm.store(anm + 0xA4, 4, INU_CHECK_NOTE)
        return 0

    def forward(self, mm, anm, spd, *a):
        st = self.anm[anm]
        f, notes = self.anims[st[0]].forward_notes(st[1], spd & 0xFFFF)
        st[1] = f.time
        st[2] = notes
        mm.store(anm + 0x98, 4, f.time >> 8)
        return int(f.ended)

    def fill_notes(self, mm, anm, *a):
        notes = self.anm.get(anm, [None, 0, []])[2]
        for i, (e, p) in enumerate(notes):
            n = NOTES + 0x20 * i
            mm.store(n + 4, 4, e)
            mm.store(n + 8, 4, p)
            mm.store(NOTE_LIST + 4 * i, 4, n)
            self.notes_seen.append([e, p])
        mm.store(NOTE_LIST + 4 * len(notes), 4, 0)
        return 0

    def set_matrix(self, scaled):
        def f(mm, anm, *a):
            self.scaled[anm] = scaled
            return 0
        return f

    def anm_draw(self, mm, anm, *a):
        which = self.which.get(anm)
        if which is not None:
            self.drawn.append([which, mm.load(anm + 0x88, 4), int(self.scaled.get(anm, False))])
        return 0

    # --- the collision mesh ------------------------------------------------
    def decode_hit(self):
        m, sym = self.m, self.sym
        c = self.town
        hit = next((off, end) for off, t, n, end in c.chunks() if t is not None and t & 0xFFFF == 0x0B00)
        self.data, self.cur_read = c.data, hit[0] + 8

        def read_struct(mm, rb, dst, n, *_):
            mm.mem[dst:dst + n] = self.data[self.cur_read:self.cur_read + n]
            self.cur_read += n
            return dst

        def read_u32(mm, *_):
            v = struct.unpack_from("<I", self.data, self.cur_read)[0]
            self.cur_read += 4
            return v

        def read_float(mm, *_):
            mm.f[0] = struct.unpack_from("<I", self.data, self.cur_read)[0]
            self.cur_read += 4
            return 0
        m.hooks[sym("ReadStruct__14ccRingBufferThFPvUi")] = read_struct
        m.hooks[sym("ReadU32__14ccRingBufferThFv")] = read_u32
        m.hooks[sym("ReadFloat__14ccRingBufferThFv")] = read_float
        m.store(STREAM + 0xA4, 4, STREAM + 0x100)
        m.store(STREAM + 0x30, 4, IDX)
        m.store(STREAM + 0x84, 4, 0)
        self.call("Decode_Hit__8ccStreamFv", STREAM)
        model = m.load(m.load(STREAM + 0x84, 4) + 16, 4)
        self.call("initHitCheck1__Fv")
        mh = self.malloc(m, 0xA0)
        self.call("__ct__10ccModelHitFv", mh)
        m.store(mh + 12, 4, model)
        self.call("HitEnable__10ccModelHitFi", mh, 0)

    # --- the save ------------------------------------------------------------
    def set_record(self, town, rec, flute):
        m = self.m
        a = SAVE + 0x2194 + 0x18 * town
        for k, v in enumerate(rec[:10]):
            m.store(a + 2 * k, 2, v)
        m.store(a + 0x14, 4, rec[10])
        m.store(SAVE + 0xCFC + 49, 1, flute)

    def record(self, town):
        m = self.m
        a = SAVE + 0x2194 + 0x18 * town
        return [m.load(a + 2 * k, 2, True) for k in range(10)] + [m.load(a + 0x14, 4, True)]

    def set_chibi(self, town):
        """ccSetChibiGuso with setDog recording the rows."""
        self.placed = []
        sd = self.sym("setDog__Fi")
        saved = self.m.hooks.get(sd)
        self.m.hooks[sd] = self.set_dog_hook
        self.m.store(GAME + 0x20, 4, town)
        self.call("ccSetChibiGuso__Fv")
        if saved is None:
            del self.m.hooks[sd]
        else:
            self.m.hooks[sd] = saved
        return {"rows": self.placed, "grow": self.record(town)}

    # --- the Grunty ----------------------------------------------------------
    def new(self, row, town, seed):
        """setDog(row), its ccEntryCtrl::entryObject answered by
        ccPGuso::ccPGuso on an entry with the param and the row's file."""
        m, sym = self.m, self.sym
        self.rand = Rand(seed)
        m.store(GAME + 0x20, 4, town)
        params = []

        def entry_object(mm, ctrl, param, *a):
            p = self.malloc(mm, 0x60)
            mm.mem[p:p + 0x60] = mm.mem[param:param + 0x60]
            params.append(p)
            return 0
        eo = sym("entryObject__11ccEntryCtrlFP12ccEntryParam")
        m.hooks[eo] = entry_object
        m.store(sym("g_entCtrl"), 4, self.malloc(m, 0x40))
        self.call("setDog__Fi", row)
        del m.hooks[eo]
        a = inf_va(0x00619460) + 0x70 * row
        stem = self.cstr(m.load(a + 0x6C, 4)).split(".")[0].lower()
        stream = self.ccs_adrs(m, self.put_str(stem))
        entry = self.malloc(m, 0x40)
        m.store(entry + 12, 4, params[0])
        m.store(entry + 16, 4, stream)
        g = self.g = self.malloc(m, 0x340)
        self.call("__ct__7ccPGusoFP7ccEntry", g, entry)
        self.affects = {m.load(g + 0x94, 4): 0}
        # dogAction, dogAction2, dogActionAdult. From Mutation on a dog's
        # dogAction carries the name too: the Grunty's is the function
        # before dogAction2.
        two = sym("dogAction2__FP6ccChar")
        one = self.prog.symbol_at(two - 4, 0x1000)[0].value
        self.fns = {one: 0, two: 1, sym("dogActionAdult__FP6ccChar"): 2}
        self.route = m.load(g + 0x2C8, 4)
        return self.state()

    def put_str(self, s):
        a = self.malloc(self.m, len(s) + 1)
        self.m.mem[a:a + len(s) + 1] = s.encode() + b"\0"
        return a

    @property
    def which(self):
        g, m = self.g, self.m
        return {m.load(g + 0xD4, 4): 0, m.load(g + 0x1E4, 4): 1, m.load(g + 0x1F0, 4): 2}

    def poke(self, field, value):
        if field == "race":
            self.set_race(value)
            return
        off = {"growth": 0x2AE, "foodmode": 0x307}[field]
        self.m.store(self.g + off, 1, value)

    def race_global(self):
        """From Mutation on adultMain asks 0x005ff790 (MUT) whether the
        race runs, then reads the race at the gp word its first `lw a1`
        loads: the kind raced +0x74, game.server +0x7c, the step +0x98."""
        a = self.sym("adultMain__7ccPGusoFv")
        for k in range(0, 0x40, 4):
            w = self.m.load(a + k, 4)
            if w >> 26 == 0x23 and (w >> 21) & 31 == 28 and (w >> 16) & 31 == 5:
                return (self.prog.gp + (w & 0xFFFF) - ((w & 0x8000) << 1)) & 0xFFFFFFFF
        raise AssertionError("adultMain reads no race")

    def set_race(self, value):
        """(kind, server, step) puts a race up (game.area is a town's 0),
        None takes it down."""
        m, at = self.m, self.race_global()
        if value is None:
            m.store(at, 4, 0)
            return
        r = m.load(at, 4) or self.malloc(m, 0x100)
        kind, server, step = value
        m.store(r + 0x74, 4, kind)
        m.store(r + 0x7C, 4, server)
        m.store(r + 0x98, 2, step)
        m.store(at, 4, r)

    def race_step(self):
        if volume.NAME == "infection":
            return None
        r = self.m.load(self.race_global(), 4)
        return self.m.load(r + 0x98, 2, True) if r else None

    def frame(self, player, pdirc, cam, deg1, eye, affect=None):
        m, g = self.m, self.g
        self.events.clear()
        self.notes_seen.clear()
        self.drawn.clear()
        self.vec(PLAYER + 0x40, list(player) + [ONE])
        self.vec(PLAYER + 0x60, [0, 0, pdirc, 0])
        self.vec(TCAM, list(cam) + [ONE])
        m.store(TCAM + 0x5A, 2, deg1)
        m.store(TCAM + 0x5C, 4, 1 if eye else 3)
        if affect is not None:
            cmd, a1, a2 = affect
            m.store(g + 0x98, 4, PLAYER)
            m.store(g + 0x9C, 2, cmd)
            m.store(g + 0x9E, 2, a1)
            m.store(g + 0xA0, 2, a2)
            before = self.rvec(PLAYER + 0x40), m.load(PLAYER + 0x68, 4)
            self.call(m.load(g + 0x94, 4), g)
            after = self.rvec(PLAYER + 0x40), m.load(PLAYER + 0x68, 4)
            if after != before:
                # Kite put at his place: after the camera's calls.
                self.events.append(["player", after[0], after[1]])
        self.call("routine__10ccEntryObjFv", g)
        self.call(m.load(m.load(g + 0x1CC, 4) + 8, 4), g)
        return {"s": self.state(), "ev": list(self.events), "notes": list(self.notes_seen),
                "drawn": list(self.drawn), "race": self.race_step()}

    def anm_state(self, off):
        a = self.m.load(self.g + off, 4)
        if not a or a not in self.anm:
            return None
        st = self.anm[a]
        return [st[0], st[1]]

    def state(self):
        m, g = self.m, self.g
        h = lambda o: m.load(g + o, 2, True)            # noqa: E731
        w = lambda o: m.load(g + o, 4, True)            # noqa: E731
        u = lambda o: m.load(g + o, 4)                  # noqa: E731
        b = lambda o: m.load(g + o, 1, True)            # noqa: E731
        town = m.load(GAME + 0x20, 4)
        grow = [h(0x2D0 + 2 * k) for k in range(10)] + [w(0x2E4)]
        mark = (u(0x2C4) - self.route) // 4
        # From Mutation on a grown one's +0x2c4 points at its dummy's name.
        if volume.NAME != "infection" and self.route == 0 and u(0x2C4):
            mark = self.cstr(m.load(u(0x2C4), 4))
        return {
            "id": w(0x270), "local": w(0x274),
            "act": [w(0x278), w(0x27C), w(0x284), w(0x288), w(0x28C)],
            "pos": self.rvec(g + 0x40), "posp": self.rvec(g + 0x50), "dirc": self.rvec(g + 0x60),
            "hit": [h(0x294), h(0x296), h(0x298)], "next": self.rvec(g + 0x200), "old": self.rvec(g + 0x210),
            "mark": mark, "scale": self.rvec(g + 0x220),
            "growth": [b(0x2AE), b(0x2AF), h(0x2AA), u(0x2B8)], "grow": grow,
            "evo": [w(0x2F4), w(0x2F8), w(0x2FC), m.load(g + 0x304, 1), m.load(g + 0x305, 1),
                    m.load(g + 0x306, 1), m.load(g + 0x307, 1), w(0x300)],
            "trans": self.rvec(g + 0x308, 2), "cam": [w(0x2EC), self.rvec(g + 0x250), self.rvec(g + 0x260)],
            "spdeg": m.load(g + 0x2AC, 2), "speed": u(0x2B4), "adult": h(0x2A8),
            "gpsub": [h(0x29E + 2 * k) for k in range(5)], "attr": u(0x80), "t": u(0x88),
            "a": self.anm_state(0xD4), "b": self.anm_state(0x1E4), "c": self.anm_state(0x1F0),
            "save": self.record(town),
            "inu": [m.load(SAVE + 0x7474 + 2 * k, 2, True) for k in range(9)],
            "fn": self.fns.get(u(0x94), -1),
        }


def walk_frames(rng, n, near=None, affects=None, cam=None):
    """[(player, pdirc, cam, deg1, eye, affect)] as bits: Kite standing
    near `near` (or off by the pens), the camera about him."""
    out = []
    px, py, pz = near or (-2400.0, -5200.0, 0.0)
    for i in range(n):
        x = px + rng.uniform(-60, 60)
        y = py + rng.uniform(-60, 60)
        c = cam or (x + rng.uniform(-900, 900), y + rng.uniform(-900, 900), pz + rng.uniform(100, 600))
        out.append(((fb(x), fb(y), fb(pz)), fb(rng.uniform(-3.1, 3.1)), tuple(fb(v) for v in c),
                    rng.randrange(-4000, 4000) & 0xFFFF, rng.random() < 0.05, (affects or {}).get(i)))
    return out


class Scenario:
    """A Grunty of `row` over the record `rec` (and the flute), Kite about
    `near`, the menus' affects and writes (`pokes`: frame -> [(field,
    value)], growthNum and foodMode) by frame."""

    def __init__(self, name, row, rec, flute=0, frames=600, seed=7, affects=None, near=None, town=1, pokes=None):
        self.name, self.row, self.rec, self.flute = name, row, rec, flute
        self.frames, self.seed, self.affects, self.near, self.town = frames, seed, affects or {}, near, town
        self.pokes = pokes or {}


def scenarios():
    fresh = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1]
    feed = lambda f, n: (19, f, n)                  # noqa: E731
    out = [
        # A fresh one walking its route, resting, sitting to be spoken to.
        Scenario("walk", 154, fresh, frames=1500),
        Scenario("talk", 154, fresh, frames=500, affects={60: (14, 0, 0), 120: (15, 0, 0), 200: (0, 0, 0),
                                                           260: (14, 0, 0), 300: (15, 0, 0), 330: (0, 0, 0)}),
        # Fed: the fixed camera, eating, the food it asks for next.
        Scenario("eat", 154, fresh, frames=400, affects={20: (14, 0, 0), 40: (11, 0, 0), 60: feed(3, 1),
                                                         250: (15, 0, 0), 300: (11, 0, 0), 320: (0, 0, 0)}),
        # Growing: level 0 to 1, 1 to 2, 2 to 3, 1 to 3.
        Scenario("chibi2", 154, [0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 3], frames=520,
                 affects={10: (11, 0, 0), 20: feed(5, 1), 480: (11, 0, 0), 500: (0, 0, 0)}),
        Scenario("pon1", 155, [1, 9, 3, 1, 2, 0, 1, 0, 0, 0, 5], frames=560,
                 affects={10: (11, 0, 0), 20: feed(1, 1), 520: (11, 0, 0), 540: (0, 0, 0)}),
        Scenario("pon2", 156, [2, 19, 5, 4, 3, 2, 1, 0, 0, 0, 7], frames=560,
                 affects={10: (11, 0, 0), 20: feed(2, 1), 520: (11, 0, 0), 540: (0, 0, 0)}),
        Scenario("pon2 from 1", 155, [1, 19, 5, 4, 3, 2, 1, 0, 0, 0, 7], frames=600,
                 affects={10: (11, 0, 0), 20: feed(0, 1), 560: (11, 0, 0), 580: (0, 0, 0)}),
        # Grown: a new kind (the record's stats near the town's first grown
        # one's: kind town * 2 - 1), done speaking (BreedingMenu's growthNum
        # 4), walking the route as one; a kind the town has already, which
        # runs off and flattens away; a far one (kind 0).
        Scenario("adult new", 157, [3, 29, 10, 5, 20, 15, 5, 0, 0, 0, 9], frames=900,
                 affects={10: (11, 0, 0), 20: feed(4, 1), 560: (11, 0, 0), 700: (14, 0, 0), 720: (15, 0, 0),
                          760: (15, 0, 0), 800: (0, 0, 0)},
                 pokes={500: [("growth", 4)]}),
        Scenario("adult exists", 157, [3, 29, 30, 30, 30, 30, 30, 1, 0, 0, 9], frames=900, flute=1,
                 affects={10: (11, 0, 0), 20: feed(4, 1), 860: (11, 0, 0), 880: (0, 0, 0)},
                 pokes={480: [("growth", 4)]}),
        Scenario("adult mid", 157, [3, 29, 2, 0, 12, 8, 0, 0, 0, 0, 9], frames=700, flute=1,
                 affects={10: (11, 0, 0), 20: feed(0, 1), 660: (11, 0, 0), 680: (0, 0, 0)}),
        # A Kid fed and not growing: waiting for the menu, asking for food
        # in the chat bubble (foodMode, InuMenu's), every 211 frames.
        Scenario("chat", 156, [2, 11, 5, 4, 3, 2, 1, 0, 0, 0, 7], frames=800,
                 affects={10: (11, 0, 0), 20: feed(6, 1), 760: (11, 0, 0), 780: (0, 0, 0)},
                 pokes={200: [("foodmode", 1)]}),
        # Fed from nothing to grown in one visit, spoken to between meals.
        grow_up(),
        # The grown ones at the pens: lines by ccRand.
        Scenario("grown 145", 145, [4, 30, 0, 0, 0, 0, 0, 1, 1, 1, 9], frames=200,
                 affects={20: (14, 0, 0), 40: (15, 0, 0), 60: (15, 0, 0), 80: (15, 0, 0), 100: (0, 0, 0)}),
        Scenario("grown 146", 146, [4, 30, 0, 0, 0, 0, 0, 1, 1, 1, 9], frames=200,
                 affects={20: (14, 0, 0), 40: (15, 0, 0), 60: (15, 0, 0), 100: (0, 0, 0)}),
        Scenario("grown 147", 147, [4, 30, 0, 0, 0, 0, 0, 1, 1, 1, 9], frames=150,
                 affects={20: (14, 0, 0), 40: (15, 0, 0)}),
    ]
    if volume.NAME != "infection":
        # The Flag Race (from Mutation on): the one raced looked at, gone
        # from the lists, its clips, walking off to its goal (kind 0's by
        # the server), gone, back home; another kind's race leaves it be.
        for row, kind in ((145, 0), (146, 1), (147, 2)):
            race = lambda st: [("race", (kind, 1, st))]          # noqa: E731
            out.append(Scenario("raced %d" % row, row, [4, 30, 0, 0, 0, 0, 0, 1, 1, 1, 9], frames=300,
                                pokes={20: race(0), 30: race(1), 40: race(2), 230: race(6), 250: race(7),
                                       270: [("race", None)]}))
        out.append(Scenario("not raced", 145, [4, 30, 0, 0, 0, 0, 0, 1, 1, 1, 9], frames=120,
                            pokes={20: [("race", (2, 1, 1))], 40: [("race", (2, 1, 2))], 100: [("race", None)]}))
    return out


def grow_up():
    """A fresh one fed three of food 0 (size +6) a meal, each meal as Give
    Food makes it: the camera (11), eating, the growing, the camera back
    (11), the menu shut (0); talked to between meals; grown at the fifth,
    done speaking (growthNum 4) and walking as a new kind."""
    affects, pokes = {}, {}
    f = 10
    for meal in range(5):
        affects[f] = (14, 0, 0)
        affects[f + 10] = (15, 0, 0)
        affects[f + 30] = (11, 0, 0)
        affects[f + 40] = (19, 0, 3)
        affects[f + 600] = (11, 0, 0)
        affects[f + 620] = (0, 0, 0)
        if meal == 4:
            pokes[f + 500] = [("growth", 4)]
        f += 700
    return Scenario("grow up", 154, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1], frames=f + 200, affects=affects,
                    pokes=pokes, seed=11)


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class GruntyAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def test_set_chibi_guso(self):
        """ccSetChibiGuso's rows and the record it leaves, over records of
        each level with kinds set and not, in each town."""
        g = GruntyGame()
        rng = random.Random(3)
        cases = []
        for town in (1, 2, 3, 4):
            for level in range(5):
                for ty in ((0, 0, 0), (1, 0, 0), (0, 1, 1), (1, 1, 1), (1, 0, 1)):
                    rec = [level, rng.randrange(40)] + [rng.randrange(-5, 20) for _ in range(5)] + list(ty) + [7]
                    cases.append((town, rec))
        lines = ["set %x %s %x" % (t, hexs(*r), 0) for t, r in cases]
        got = ask(lines)
        for (t, r), port in zip(cases, got):
            g.set_record(t, r, 0)
            self.assertEqual(g.set_chibi(t), port, (t, r))

    def run_scenario(self, s):
        g = GruntyGame(s.town)
        g.set_record(s.town, s.rec, s.flute)
        rng = random.Random(s.seed)
        frames = walk_frames(rng, s.frames, s.near, s.affects)
        lines = ["set %x %s %x" % (s.town, hexs(*s.rec), s.flute), "new %x %x %x" % (s.row, s.town, s.seed)]
        for i, (pl, pd, cam, deg1, eye, aff) in enumerate(frames):
            for f, v in s.pokes.get(i, []):
                if f != "race":
                    lines.append("poke %s %x" % (f, v))
                elif v is None:
                    lines.append("poke norace")
                else:
                    lines.append("poke race " + hexs(*v))
            args = list(pl) + [pd] + list(cam) + [deg1, int(eye)]
            if aff is not None:
                args += list(aff)
            lines.append("frame " + hexs(*args))
        got = ask(lines)
        want0 = g.new(s.row, s.town, s.seed)
        self.assertEqual(want0, got[1], "%s: the constructor" % s.name)
        seen = set()
        for i, ((pl, pd, cam, deg1, eye, aff), port) in enumerate(zip(frames, got[2:])):
            for f, v in s.pokes.get(i, []):
                g.poke(f, v)
            want = g.frame(pl, pd, cam, deg1, eye, aff)
            if want["s"]["act"][0] == 7 and want["s"]["evo"][1] >= 6:
                # runAway's checkHitResultAttlibute reads the last query's
                # results and, with none near, a register's leftover.
                want["s"]["attr"] = port["s"]["attr"]
            for e in want["ev"]:
                seen.add(e[0])
            seen.add("act%d" % want["s"]["act"][0])
            seen.add("evo%d.%d" % (want["s"]["evo"][0], want["s"]["evo"][1]))
            if want != port:
                for k in want:
                    if want[k] != port.get(k):
                        if k == "s":
                            diff = {kk: (want[k][kk], port[k].get(kk)) for kk in want[k]
                                    if want[k][kk] != port[k].get(kk)}
                            self.fail("%s frame %d: state %s" % (s.name, i, diff))
                        self.fail("%s frame %d: %s\n game %s\n port %s" % (s.name, i, k, want[k], port.get(k)))
        return seen

    def test_scenarios(self):
        seen = set()
        for s in scenarios():
            with self.subTest(s.name):
                seen |= self.run_scenario(s)
        for k in ("act0", "act1", "act2", "act3", "act4", "act6", "act7", "act9", "note", "smoke", "evolve",
                  "grow", "voice", "voicestop", "chat", "chatclose", "camera", "campos", "camview", "player",
                  "se3d", "dropped"):
            self.assertIn(k, seen, k)


if __name__ == "__main__":
    unittest.main()
