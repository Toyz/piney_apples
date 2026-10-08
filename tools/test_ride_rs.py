#!/usr/bin/env python3
"""piney_battle::ride (the riding Grunty, ccPucciguso, gcmn pucciguso.cpp)
against the game's own code run in tools/eemu.py (the Rust machine,
crates/piney-eemu, when built) over test_battle's GameBattle (GCMN.PRG
loaded).

A ccPucciguso is laid out in scratch memory - its members at random (the
place about a field's map and past its edges, any heading, act and flags,
the idle count about the fidget's 451, the speeds, the eased move, the
body) - with plw (Kite's ccPlayer behind it), the pad, the camera
(activeCamPtr's type, reset flag and heading, camID, cameraGetRot's
answer), WORLD_MAN's bounds, dneFlag, game.area and a random rand() state;
then one of its functions runs natively, once or for 1-80 frames, and the
same case through the ride_probe example. Compared after every frame: the
object's members, pgR and pgDIN, plw's place, heading and pause, the
camera's reset flag, the rand() state, and every call in order.

The calls into code that is not the ride's are stubbed, recorded and
answered from a script, on both sides alike: ccLandHitCheck (with
hitResultNum and hitResultNearest's attribute), checkHitResultAttlibute,
ccCharHit::CollisionDetection (the offset) and SetHitSW, ccHitCheckLM2,
cameraGetRot, cameraSetEyeLevel (angle.z), cameraSetManual, cameraSet,
ccGetCameraTransparency (the fade and the hide flag), GetChunkAdrsF and
SetAnm (the clip's name), _AnimateForward, the Grunty clump's
GetObjAdrsF (a leg's coordinate, its local matrix the script's place),
ccChar::Draw (Kite's clump: setTransparency in, transparency out),
ccAnm::Draw of the Grunty with the draw environment's alpha and height and
the distant light's sleep (the shade), effSmoke, ccSeSetParamInu,
WORLD_MAN::AddCenter, Enter, SetMatrix_PosRotZYX. ccAnm::NoteProcess is
replaced by a routine that hands the script's notes for the frame to the
game's ccPuccigusoCheckNote. ControlMove, PadLeverPower, AnimCtrl,
CollisionTest, MapLoopAdjustPos (ccTransPosW2M, W2MPos), CameraPosCalc,
CameraPosSet, PawSmoke, DrawPG, the camera checks, RAD2DEG, DEG2RAD, the
maths and the VU0 routines run natively.

    python3 tools/test_ride_rs.py            the unit tests
    python3 tools/test_ride_rs.py bulk N     N cases of every check

Checks:
  tables        puccigusoAnimTbl, puccigusoAnimTblPG, puccigusoCharTbl,
                puccigusoAngleTbl as the probe read them
  ctor          ccPucciguso::ccPucciguso(kind) (0x00510f30) for every kind
  main          ccPucciguso::Main (0x005114e0) for 1-80 frames
  seek          from Mutation on, a field's search: the run (MUT 0x0052f250)
                and the search (0x0052f790) alone, ControlMove's and Main's
                parts of it, over g_entCtrl's gimmicks and circles,
                WORLD_MAN's event area and dungeonPos, the pad's push and
                powR; the balloon's line (+0xe1) and name (+0x24c) compared
  control_move  ControlMove (0x005119c0), anim_ctrl AnimCtrl (0x00511f80),
                draw_pg DrawPG (0x00512c80), note ccPuccigusoCheckNote
                (0x00512fb0), smoke PawSmoke (0x00512670), once each
  lever         PadLeverPower (0x00511ed0) over 0-255 and beyond
  adult         ccPgAdultCheck (0x00510650) over servers -1..6, slots -1..3
  exit          ccPuccigusoExit (0x00510bd0) natively (its fades done at
                once, Main stubbed): each member's place and heading against
                the probe's, and the order of its calls (the fades, the
                bodies, ccSpcWakeup, the menu's panels and ban, ccPgBgmEnd,
                the file, the flags) as the port's runtime follows them
  start         ccPuccigusoStart (0x005109c0) likewise: its refusals (not a
                field, pgR set), the flags, ccPgBgmInit, the fades, the
                party asleep and their bodies out, the file, the task

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import math
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

import eemu  # noqa: E402
import test_anim  # noqa: E402

eemu.Machine = test_anim.machine_class()

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
ISO = volume.ISO
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "ride_probe")

BASE = 0x01100000
OBJ = BASE                  # the ccPucciguso (0x1d0)
ANM_K, ANM_P = BASE + 0x400, BASE + 0x600     # its two ccAnm stand-ins
CAM = BASE + 0x1000         # activeCamPtr's CAMERA
WM = BASE + 0x2000          # WORLD_MAN
KITE = BASE + 0x3000        # plw.pw
COORD = BASE + 0x4000       # the legs' coordinates
DRAWENV = BASE + 0x5000     # ccDrawEnv::active
NOTES, NOTE_LIST = BASE + 0x6000, BASE + 0x6400
REG = BASE + 0x7000         # the registry's characters (Exit, Start)
ENTS = BASE + 0x9000        # the search's entries, their bases and names
SPAD = BASE + 0xC000        # ccSys spadWork (the search's scratch)
CHAT = BASE + 0xD000        # ccChat, and the party's first member for #a
KITE_CHAR = BASE + 0xD100
HEAP = BASE + 0x10000       # operator new
PCGS_TBL = inf_va(0x005EE810)
CHECK_NOTE = inf_va(0x00512FB0)
ONE = 0x3F800000
M_ONE = 0xBF800000
BOUND = 0x473B8000          # 48000.0

RIDE = [("pos", 0x40, "v"), ("pos_p", 0x50, "v"), ("rot", 0x60, "v"), ("hit_attribute", 0x80, "w"),
        ("transparency", 0x88, "w"), ("cset", 0x8C, "w"), ("flags", 0xE0, "b"), ("act", 0xE2, "h"),
        ("act_old", 0xE4, "h"), ("anm_end", 0xE6, "h"), ("idle", 0xE8, "h"), ("speed", 0xEC, "w"),
        ("speed_rate", 0xF0, "w"), ("now_speed", 0xF4, "w"), ("set_t", 0xF8, "w"), ("kind", 0x100, "w"),
        ("cycle", 0x104, "w"), ("pos_view", 0x110, "v"), ("pos_eye", 0x130, "v"), ("angle", 0x140, "v"),
        ("move_pos", 0x150, "v"), ("move_ease", 0x160, "v"), ("hit_sw", 0x170, "w"), ("hit_radius", 0x184, "w"),
        ("hit_height", 0x188, "w"), ("hit_pos", 0x190, "v"), ("hit_offset", 0x1A0, "v")]
# From Mutation on the object is 0x250 bytes: past the flags the members
# move 0x50 on, the eyes and the moves 0x60, the body 0x80 (MUT gcmn
# 0x0052dd10, the constructor; 0x00530360 CameraPosSet).
LATER = volume.NAME != "infection"
if LATER:
    _move = {"act": 0x132, "act_old": 0x134, "anm_end": 0x136, "idle": 0x138, "speed": 0x13C, "speed_rate": 0x140,
             "now_speed": 0x144, "set_t": 0x148, "kind": 0x150, "cycle": 0x154, "pos_view": 0x170,
             "pos_eye": 0x190, "angle": 0x1A0, "move_pos": 0x1B0, "move_ease": 0x1C0, "hit_sw": 0x1F0,
             "hit_radius": 0x204, "hit_height": 0x208, "hit_pos": 0x210, "hit_offset": 0x220}
    RIDE = [(n, _move.get(n, off), k) for n, off, k in RIDE]
# The body's kind words, the Grunty's file, clump and anm, the object's size.
HIT = 0x1F0 if LATER else 0x170
# From Mutation on: the search's lead, idle, hold, target and range, the
# balloon's line (+0xe1) and the name (+0x24c).
SEEK = [("seek_lead", 0x158, "w"), ("seek_idle", 0x15C, "w"), ("seek_hold", 0x160, "w"),
        ("seek_target", 0x1D0, "v"), ("seek_range", 0x1E0, "w")]
CHAT_TEXT, SEEK_NAME = 0xE1, 0x24C
PG_FILE, PG_ANM = (0x240, 0x248) if LATER else (0x1C0, 0x1C8)
SIZE = 0x260 if LATER else 0x200
RACE = BASE + 0x8000        # the Flag Race's object (Mutation on)


def race_ptr(g):
    """The Flag Race's object pointer (MUT 0x0038bd44): the $gp word
    ControlMove loads that has no name (the race's handling)."""
    import mips  # noqa: F401
    m, sym = g.m, g.sym
    lo, hi = volume.span("ControlMove__11ccPuccigusoFv")
    gp = volume.program(ELF).gp
    named = {sym(n) for n in ("game", "ccSys", "activeCamPtr", "camID")}
    for a in range(lo, hi, 4):
        w = m.load(a, 4)
        if w >> 16 == 0x8F82:
            t = (gp + (w & 0xFFFF) - (0x10000 if w & 0x8000 else 0)) & 0xFFFFFFFF
            if t not in named and volume.program(ELF).name_at(t) is None:
                return t
    raise AssertionError("ControlMove loads no race pointer")


def seek_fns():
    """The run and the search (MUT gcmn 0x0052f250, 0x0052f790): the two
    functions after ControlMove, none of them named."""
    lo, hi = volume.span("ControlMove__11ccPuccigusoFv")
    prog = volume.program(ELF)
    out, a = [], hi
    while len(out) < 2:
        w = prog.u32(a)
        if w >> 16 == 0x27BD and w & 0x8000:
            out.append(a)
            while prog.u32(a) != 0x03E00008:
                a += 4
        a += 4
    return out


def hexs(b):
    return bytes(b).hex()


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v >> 31 else v


def cstr_bytes(m, a):
    return bytes(m.mem[a:a + 0x60]).split(b"\0")[0]


def cstr(m, a):
    b = bytes(m.mem[a:a + 32])
    return b.split(b"\0")[0].decode("latin-1")


def note_routine():
    """ccAnm::NoteProcess's stand-in: ccPuccigusoCheckNote(note) for each
    address in NOTE_LIST up to a 0."""
    hi, lo = NOTE_LIST >> 16, NOTE_LIST & 0xFFFF
    return [
        0x27BDFFE0, 0xFFBF0010, 0xFFB00000, 0x3C100000 | hi, 0x36100000 | lo,
        0x8E040000, 0x10800005, 0x00000000, 0x0C000000 | (CHECK_NOTE >> 2), 0x26100004,
        0x1000FFFA, 0x00000000, 0xDFBF0010, 0xDFB00000, 0x03E00008, 0x27BD0020,
    ]


# the game side ---------------------------------------------------------------------

class RideGame:
    def __init__(self):
        import battle
        import test_battle as tb
        self.g = tb.GameBattle(battle.Data(ELF))
        g, m, sym = self.g, self.g.m, self.g.sym
        self.m, self.sym = m, sym
        m.hooks.pop(sym("sceVu0CopyVector"), None)
        m.mem[BASE:HEAP + 0x10000] = bytes(HEAP + 0x10000 - BASE)
        self.plw = sym("plw")
        self.glob = {n: sym(n) for n in ("pgR", "pgDIN", "pgRideFlag", "pcgs", "camTypeLock", "dneFlag",
                                         "hitResultNum", "camID", "activeCamPtr", "worldman", "ccMenu")}
        if LATER:
            self.glob.update({n: sym(n) for n in ("g_entCtrl", "ccChat")})
            self.seek_move_fn, self.seek_fn = seek_fns()
        self.hit_near = sym("hitResultNearest")
        self.race_ptr = race_ptr(g) if LATER else None
        m.store(self.glob["activeCamPtr"], 4, CAM)
        m.store(self.glob["worldman"], 4, WM)
        m.store(sym("active__9ccDrawEnv"), 4, DRAWENV)
        np_ = sym("NoteProcess__5ccAnmFv")
        for i, wd in enumerate(note_routine()):
            m.store(np_ + 4 * i, 4, wd)
        m.hooks.pop(np_, None)
        self.script = {}
        self.calls = []
        self.heap = HEAP
        self.streams = {}
        self.shaded = 0
        self.install()

    def pop(self, key, default):
        s = self.script.get(key)
        return s.pop(0) if s else default

    def vec(self, a):
        return [self.m.load(a + 4 * i, 4) for i in range(4)]

    def put_vec(self, a, v):
        for i, x in enumerate(v):
            self.m.store(a + 4 * i, 4, x)

    def which(self, anm):
        m = self.m
        if anm == m.load(OBJ + 0xD4, 4):
            return 0
        if anm == m.load(OBJ + PG_ANM, 4):
            return 1
        return -1

    def install(self):
        m, sym = self.m, self.sym
        rec = self.calls.append

        def land(mm, pos, mask, *_):
            rec(["ccLandHitCheck", self.vec(pos), mask])
            z = self.pop("land", 0)
            res = self.pop("result", -1)
            mm.store(self.glob["hitResultNum"], 4, 0 if res < 0 else 1)
            mm.store(self.hit_near + 0x48, 4, 0 if res < 0 else res)
            mm.f[0] = z
            return z

        def attr(mm, *_):
            rec(["checkHitResultAttlibute"])
            return self.pop("attr", 0)

        def collide(mm, hit, *_):
            rec(["CollisionDetection", self.vec(hit + 0x20), mm.load(hit + 0x14, 4)])
            r, off = self.pop("collide", (0, [0, 0, 0, 0]))
            self.put_vec(hit + 0x30, off)
            return r

        def line(mm, a, b, mask, *_):
            rec(["ccHitCheckLM2", self.vec(a), self.vec(b), mask])
            d = self.pop("line", M_ONE)
            mm.f[0] = d
            return d

        def hit_sw(mm, hit, on, *_):
            rec(["SetHitSW", on])
            mm.store(hit, 4, on)
            return 0

        def rot(mm, v, cid, *_):
            self.put_vec(v, self.cam_rot)
            return 0

        def eye(mm, pe, ang, *_):
            rec(["cameraSetEyeLevel", self.vec(pe), self.vec(ang)])
            mm.store(ang + 8, 4, self.pop("eye", 0))
            return 0

        def trans(mm, pos, hide, *_):
            rec(["ccGetCameraTransparency", self.vec(pos), mm.f[12], mm.f[13], mm.f[14], mm.f[15]])
            f, h = self.pop("trans", (ONE, 0))
            mm.store(hide, 4, h)
            mm.f[0] = f
            return f

        def chunk(mm, ccs, name, *_):
            return name

        def set_anm(mm, anm, ch, *_):
            rec(["SetAnm", self.which(anm), cstr(mm, ch)])
            return 0

        def forward(mm, anm, step, *_):
            w = self.which(anm)
            rec(["AnimateForward", w, step & 0xFFFF])
            if w == 1:
                notes = self.pop("notes", [])
                for i, (ev, par) in enumerate(notes):
                    a = NOTES + 16 * i
                    self.put_vec(a, [0, ev, par, 0])
                    mm.store(NOTE_LIST + 4 * i, 4, a)
                mm.store(NOTE_LIST + 4 * len(notes), 4, 0)
            return self.pop("fwd", 0)

        def obj(mm, clump, name, *_):
            rec(["GetObjAdrsF", cstr(mm, name)])
            c = COORD
            mm.mem[c:c + 0x100] = bytes(0x100)
            p = self.pop("leg", [0, 0, 0, 0])
            self.put_vec(c + 0x40, [ONE, 0, 0, 0])
            self.put_vec(c + 0x50, [0, ONE, 0, 0])
            self.put_vec(c + 0x60, [0, 0, ONE, 0])
            self.put_vec(c + 0x70, [p[0], p[1], p[2], ONE])
            return c

        def smoke(mm, pos, v, life, t, *_):
            rec(["effSmoke", self.vec(pos), self.vec(v), mm.f[12], s32(life), s32(t), mm.r[8] & 0xFFFF,
                 mm.r[9] & 0xFFFF])
            return 0

        def draw(mm, this, *_):
            rec(["Draw", mm.load(this + 0x8C, 4)])
            mm.store(this + 0x88, 4, self.pop("draw", ONE))
            return 0

        def anm_draw(mm, anm, *_):
            rec(["DrawPG", mm.load(anm + 0x88, 4), mm.load(DRAWENV + 0xA4, 1), mm.load(DRAWENV + 0xA0, 4),
                 self.shaded])
            self.shaded = 0
            return 0

        def sleep_light(mm, *_):
            self.shaded = 1
            return 0

        def get_ambient(mm, env, v, *_):
            self.put_vec(v, [ONE, ONE, ONE, ONE])
            return 0

        def new(mm, n, *_):
            a = self.heap
            self.heap += (n + 15) & ~15
            mm.mem[a:a + n] = bytes(n)
            return a

        def ccs(mm, name, *_):
            s = cstr(mm, name)
            rec(["GetCCSAdrs", s])
            return self.streams.setdefault(s, BASE + 0x8000 + 0x10 * len(self.streams))

        def open_chat(mm, chat, ch, text, *_):
            rec(["OpenChat", hexs(bytes(mm.mem[text:text + 0x60]).split(b"\0")[0])])
            return 0

        nop = lambda mm, *_: 0              # noqa: E731
        hooks = {
            "ccLandHitCheck__FPfUi": land,
            "checkHitResultAttlibute__Fv": attr,
            "CollisionDetection__9ccCharHitFv": collide,
            "ccHitCheckLM2__FPfPfUi": line,
            "SetHitSW__9ccCharHitFi": hit_sw,
            "cameraGetRot__FPfi": rot,
            "cameraSetEyeLevel__FPfPf": eye,
            "cameraSetManual__FPf": lambda mm, p, *_: rec(["cameraSetManual", self.vec(p)]) or 0,
            "cameraSet__Fv": lambda mm, *_: rec(["cameraSet"]) or 0,
            "ccGetCameraTransparency__FPfffffRi": trans,
            "GetChunkAdrsF__8ccStreamFPCci": chunk,
            "SetAnm__5ccAnmFP10ccAnmChunkUi": set_anm,
            "_AnimateForward__5ccAnmFUi": forward,
            "GetObjAdrsF__7ccClumpFPCciP19ccSubstSearchResult": obj,
            "effSmoke__FPfPffiiUsUs": smoke,
            "ccSeSetParamInu__FUiP6ccChar": lambda mm, p, ch, *_: rec(["ccSeSetParamInu", p,
                                                                       mm.load(ch + 0x80, 4)]) or 0,
            "AddCenter__9WORLD_MANFff": lambda mm, *_: rec(["AddCenter", mm.f[12], mm.f[13]]) or 0,
            "Enter__9WORLD_MANFPf": lambda mm, w, p, *_: rec(["Enter", self.vec(p)]) or 0,
            "SetMatrix_PosRotZYX__7ccCoordFPfPf": lambda mm, a, p, d, *_: rec(
                ["SetMatrix", self.which(a), self.vec(p), self.vec(d)]) or 0,
            "Draw__6ccCharFv": draw,
            "Draw__5ccAnmFv": anm_draw,
            "SetActiveLayer__9WORLD_MANFi": nop,
            "SetFogBlend__9ccDrawEnvFfUi": nop,
            "ResetFogBlend__9ccDrawEnvFv": nop,
            "GetAmbient__9ccDrawEnvFPf": get_ambient,
            "SetAmbient__9ccDrawEnvFPf": nop,
            "SleepDistantLight__9WORLD_MANFv": sleep_light,
            "AwakeDistantLight__9WORLD_MANFv": nop,
            "__nw__FUi": new,
            "GetCCSAdrs__8ccStreamFPCc": ccs,
            "Init__7ccClumpFP12ccClumpChunk": nop,
            "__ct__5ccAnmFv": lambda mm, a, *_: a,
            "ApplyClump__5ccAnmFP7ccClumpP8ccStream": nop,
        }
        if LATER:
            hooks.update({
                "OpenChat__9ccChatMsgFP6ccCharPc": open_chat,
                "ccEntryCmnd__FP6ccChar": lambda mm, *_: rec(["ccEntryCmnd"]) or 0,
                "getPartyMenberChar__Fi": lambda mm, *_: KITE_CHAR,
            })
        # Outbreak's carry leaves the light's wake-up unnamed: DrawPG's call.
        where = {"AwakeDistantLight__9WORLD_MANFv": lambda: volume.found(
            "AwakeDistantLight__9WORLD_MANFv", "DrawPG__11ccPuccigusoFv", 0)}
        for n, f in hooks.items():
            m.hooks[where[n]() if n in where else sym(n)] = f

    # the case ------------------------------------------------------------------------
    def put(self, c):
        """The ride, the globals, the camera and the script."""
        m, g = self.m, self.g
        m.mem[OBJ:OBJ + SIZE] = bytes(SIZE)
        r = c["ride"]
        for name, off, k in RIDE:
            v = r[name]
            if k == "v":
                self.put_vec(OBJ + off, v)
            elif k == "b":
                m.store(OBJ + off, 1, v)
            elif k == "h":
                m.store(OBJ + off, 2, v)
            else:
                m.store(OBJ + off, 4, v)
        m.store(OBJ, 4, PCGS_TBL)
        m.store(OBJ + 0x90, 4, 1)
        m.store(OBJ + HIT + 4, 4, 0xFFFFFFFF)
        m.store(OBJ + HIT + 8, 4, 0x40000001)
        m.store(OBJ + HIT + 0xC, 4, 0x01000000)
        m.store(OBJ + 0xD4, 4, ANM_K)
        m.store(OBJ + PG_ANM, 4, ANM_P)
        for a, fs in ((ANM_K, r["fs"][0]), (ANM_P, r["fs"][1])):
            m.mem[a:a + 0x110] = bytes(0x110)
            m.store(a + 0x9C, 2, fs)
            m.store(a + 0xAC, 4, 1)
        m.store(self.glob["pcgs"], 4, OBJ)
        # The Grunty's clips as this ride's constructor left the table.
        kind = r["kind"]
        tbl = self.sym("puccigusoAnimTblPG")
        for i in range(7):
            m.store(tbl + 21 * i + 7, 1, (kind + 1 if kind > 0 else kind) + 48)
        inp = c["input"]
        m.store(g.sym("ccSys"), 4, 0x01022000)
        sys_ = 0x01022000
        m.store(sys_ + 0x2B0, 1, inp["pow_l"])
        m.store(sys_ + 0x2B4, 4, inp["dirc_l"])
        m.mem[self.plw:self.plw + 0x20] = bytes(0x20)
        m.store(self.plw, 1, inp["pause"])
        m.store(self.plw + 0x20, 4, KITE)
        m.mem[KITE:KITE + 0x100] = bytes(0x100)
        m.store(self.glob["dneFlag"], 4, inp["dne"])
        m.mem[WM:WM + 0x500] = bytes(0x500)
        for i, v in enumerate(inp["bounds"]):
            m.store(WM + 0x420 + 4 * i, 4, v)
        m.store(0x01021000 + 0x14, 4, inp["area"])
        if LATER:
            # The race's handling, read by ControlMove in a town.
            m.mem[RACE:RACE + 0xB0] = bytes(0xB0)
            for k, v in enumerate(inp.get("race") or []):
                m.store(RACE + 0x64 + 4 * k, 4, v)
            m.store(self.race_ptr, 4, RACE if inp.get("race") else 0)
        if LATER:
            self.put_seek(c)
        m.store(self.glob["pgR"], 4, c["g"][0])
        m.store(self.glob["pgDIN"], 4, c["g"][1])
        cam = c["cam"]
        m.mem[CAM:CAM + 0x70] = bytes(0x70)
        m.store(CAM + 0x5C, 4, cam["type"])
        m.store(CAM + 0x60, 4, cam["reset"])
        m.store(CAM + 0x64, 4, cam["reset_dirc"])
        m.store(self.glob["camID"], 2, cam["id"])
        self.cam_rot = cam["rot"]
        m.store(self.glob["hitResultNum"], 4, 0)
        m.mem[DRAWENV:DRAWENV + 0x100] = bytes(0x100)
        m.store(DRAWENV + 0xA4, 1, 0x80)
        m.store(NOTE_LIST, 4, 0)
        self.g.set_rand(c["rand"])
        self.script = {k: [list(x) if isinstance(x, list) else x for x in v] for k, v in c["script"].items()}
        self.calls.clear()
        self.shaded = 0

    def put_seek(self, c):
        """The search's members, the pad's push and powR, the scratch, the
        entry control's gimmicks and circles (each a base with its name
        and type, its place, +0xe0 flags, `next`), WORLD_MAN's event area
        and dungeonPos, the player's place, ccChat and Kite's name."""
        m, r, inp, sw = self.m, c["ride"], c["input"], c["seek"]
        for name, off, k in SEEK:
            if k == "v":
                self.put_vec(OBJ + off, r[name])
            else:
                m.store(OBJ + off, 4, r[name] & 0xFFFFFFFF)
        m.mem[ENTS:ENTS + 0x3000] = bytes(0x3000)
        texts = [ENTS + 0x2000]

        def text(h):
            if h is None:
                return 0
            a = texts[0]
            b = bytes.fromhex(h)
            m.mem[a:a + len(b) + 1] = b + b"\0"
            texts[0] = (a + len(b) + 16) & ~15
            return a
        m.store(OBJ + SEEK_NAME, 4, text(r["seek_name"]))
        b = bytes.fromhex(r["chat"]) if r["chat"] else b""
        m.mem[OBJ + CHAT_TEXT:OBJ + CHAT_TEXT + 0x51] = bytes(0x51)
        m.mem[OBJ + CHAT_TEXT:OBJ + CHAT_TEXT + len(b)] = b
        sys_ = 0x01022000
        m.store(sys_ + 0x2D0, 4, inp["push"])
        m.store(sys_ + 0x2B1, 1, inp["pow_r"])
        m.store(sys_ + 0x25C, 4, SPAD)
        ctrl, at = ENTS, ENTS + 0x40
        m.store(self.glob["g_entCtrl"], 4, ctrl)
        for count_off, head_off, entries in ((0x28, 0x2C, sw["gims"]), (0x1C, 0x20, sw["circles"])):
            m.store(ctrl + count_off, 4, len(entries))
            prev = None
            for e in entries:
                obj, base = at, at + 0x200
                at += 0x240
                m.store(obj, 4, base)
                m.store(base, 4, text(e["name"]))
                m.store(base + 8, 4, e["ty"] & 0xFFFFFFFF)
                self.put_vec(obj + 0x40, e["pos"])
                m.store(obj + 0xE0, 1, e["active"] | e["going"] << 5)
                if prev is None:
                    m.store(ctrl + head_off, 4, obj)
                else:
                    m.store(prev + 0x1C4, 4, obj)
                prev = obj
        m.store(WM + 0x120, 4, sw["event_area"] & 0xFFFFFFFF)
        self.put_vec(WM + 0x460, sw["dungeon"])
        self.put_vec(KITE + 0x40, sw["player"])
        m.store(self.glob["ccChat"], 4, CHAT)
        m.store(KITE_CHAR, 4, KITE_CHAR + 0x40)
        m.store(KITE_CHAR + 0x40, 4, text(b"Kite".hex()))

    def read_seek(self):
        """The balloon's line to its NUL and the name, hex."""
        m = self.m
        if not LATER:
            return None
        name = m.load(OBJ + SEEK_NAME, 4)
        chat = bytes(m.mem[OBJ + CHAT_TEXT:OBJ + CHAT_TEXT + 0x51]).split(b"\0")[0]
        return {"chat": chat.hex(), "name": cstr_bytes(m, name).hex() if name else None}

    def read(self):
        m = self.m
        out = []
        for name, off, k in RIDE:
            if k == "v":
                out += self.vec(OBJ + off)
            elif k == "b":
                out.append(m.load(OBJ + off, 1) & (0xFF if LATER else 0x3F))
            elif k == "h":
                out.append(m.load(OBJ + off, 2, signed=True) & 0xFFFFFFFF)
            else:
                out.append(m.load(OBJ + off, 4))
        out += [m.load(ANM_K + 0x9C, 2), m.load(ANM_P + 0x9C, 2)]
        if LATER:
            for _, off, k in SEEK:
                out += self.vec(OBJ + off) if k == "v" else [m.load(OBJ + off, 4)]
        return out

    def frame(self, ret=None):
        m = self.m
        calls = list(self.calls)
        self.calls.clear()
        return {"ride": self.read(), "seek": self.read_seek(), "ret": ret,
                "g": [s32(m.load(self.glob["pgR"], 4)), s32(m.load(self.glob["pgDIN"], 4))],
                "rand": self.g.rand_now(),
                "plw": [self.vec(KITE + 0x40), self.vec(KITE + 0x60), m.load(self.plw, 1) & 1],
                "reset": 1 if m.load(CAM + 0x60, 4) else 0, "calls": calls}

    def run(self, c, fn, frames, args=()):
        self.put(c)
        out = []
        m = self.m
        for _ in range(frames):
            ret = None
            if fn == "main":
                m.call(self.sym("Main__11ccPuccigusoFv"), (OBJ,))
            elif fn == "control_move":
                m.call(self.sym("ControlMove__11ccPuccigusoFv"), (OBJ,))
                ret = s32(m.r[2]) if LATER else None
            elif fn == "seek_move":
                m.call(self.seek_move_fn, (OBJ,))
                ret = s32(m.r[2])
            elif fn == "seek":
                m.call(self.seek_fn, (OBJ, args[0]))
                ret = s32(m.r[2])
            elif fn == "anim_ctrl":
                m.call(self.sym("AnimCtrl__11ccPuccigusoFv"), (OBJ,))
            elif fn == "draw_pg":
                m.call(self.sym("DrawPG__11ccPuccigusoFv"), (OBJ,))
            elif fn == "note":
                self.put_vec(NOTES, [0, args[0], args[1], 0])
                m.call(CHECK_NOTE, (NOTES,))
            elif fn == "smoke":
                m.f[12] = args[0]
                m.call(self.sym("PawSmoke__11ccPuccigusoFfi"), (OBJ, args[1]))
            out.append(self.frame(ret))
        return out

    def ctor(self, kind, rand, pos, rot, c):
        m = self.m
        self.put(c)
        m.mem[OBJ:OBJ + SIZE] = bytes(SIZE)
        self.put_vec(KITE + 0x40, pos)
        self.put_vec(KITE + 0x60, rot)
        self.g.set_rand(rand)
        self.heap = HEAP
        self.streams = {}
        self.calls.clear()
        tbl = self.sym("puccigusoAnimTblPG")
        saved = bytes(m.mem[tbl:tbl + 21 * 7])
        m.call(self.sym("__ct__11ccPuccigusoFi"), (OBJ, kind))
        m.store(ANM_K + 0x9C, 2, 256)
        m.store(ANM_P + 0x9C, 2, 256)
        k, p = m.load(OBJ + 0xD4, 4), m.load(OBJ + PG_ANM, 4)
        names = [cstr(m, tbl + 21 * i) for i in range(7)]
        # the anms' frame speed as the probe starts them; their note function
        ride = self.read()
        k = len(ride) - 8 if LATER else len(ride)
        ride[k - 2:k] = [256, 256]
        out = {"ride": ride, "seek": self.read_seek(), "rand": self.g.rand_now(),
               "calls": [c for c in self.calls if c[0] != "GetCCSAdrs"],
               "files": [c[1] for c in self.calls if c[0] == "GetCCSAdrs"], "pg": names,
               "note_fn": m.load(p + 0xA4, 4), "anms": [k != 0, p != 0]}
        m.mem[tbl:tbl + len(saved)] = saved
        return out


# random cases ----------------------------------------------------------------------

def rf(rnd, lo, hi):
    return fb(rnd.uniform(lo, hi))


def rvec(rnd, lo, hi, w=ONE):
    return [rf(rnd, lo, hi), rf(rnd, lo, hi), rf(rnd, -300, 300), w]


def rnd_attr(rnd):
    return rnd.choice([0, 0, 0xB0C00F, 0xC0D000, 0x60B0D0, 0x70C0E3, 0x8080F0, 0x304050, 0x40404F, 0x606060,
                       0xE0E0E0, 0x40000, 0x40000 | 0x304050, 0x80000, rnd.getrandbits(32), rnd.getrandbits(24)])


def rnd_case(rnd, frames=1):
    near = rnd.random() < 0.25
    lo, hi = (-200, 400) if near else (500, 47500)
    pos = [rf(rnd, lo, hi) if not near or rnd.random() < 0.5 else rf(rnd, 47700, 48200) for _ in range(2)]
    pos += [rf(rnd, -200, 200), ONE]
    ride = {
        "pos": pos, "pos_p": rvec(rnd, -100, 100), "rot": [0, 0, rf(rnd, -math.pi, math.pi), 0],
        "hit_attribute": rnd_attr(rnd), "transparency": rf(rnd, 0, 1), "cset": rf(rnd, 0, 1),
        "flags": rnd.getrandbits(6), "act": rnd.randrange(7), "act_old": rnd.choice([-1, 0, 2, 5, 6,
                                                                                     rnd.randrange(7)]),
        "anm_end": rnd.choice([0, 0, 1]), "idle": rnd.choice([rnd.randrange(430, 460), rnd.randrange(0, 451),
                                                               440]),
        "speed": rnd.choice([0x42700000, rf(rnd, 10, 90)]), "speed_rate": rf(rnd, 0, 1.4),
        "now_speed": rf(rnd, 0, 60), "set_t": rnd.choice([ONE, rf(rnd, 0, 1)]), "kind": rnd.randrange(9),
        "cycle": rnd.getrandbits(28), "pos_view": rvec(rnd, 0, 48000), "pos_eye": rvec(rnd, 0, 48000),
        "angle": [0, 0, rf(rnd, -3, 3), ONE], "move_pos": rvec(rnd, -30, 30), "move_ease": [
            rf(rnd, -60, 60), rf(rnd, -60, 60), rf(rnd, -1, 1), rnd.choice([ONE, 0])],
        "hit_sw": rnd.choice([0, 1, 1]), "hit_radius": rf(rnd, 90, 170), "hit_height": 0x42C80000,
        "hit_pos": rvec(rnd, 0, 48000), "hit_offset": rvec(rnd, -20, 20, 0), "fs": [rnd.randrange(600),
                                                                                    rnd.randrange(600)],
    }
    pow_l = rnd.choice([0, 0, rnd.randrange(256), rnd.randrange(64, 128), rnd.randrange(200, 256), 255, 241])
    inp = {"pow_l": pow_l, "dirc_l": rf(rnd, -math.pi, math.pi), "pause": int(rnd.random() < 0.1),
           "dne": int(rnd.random() < 0.1), "bounds": [0, 0, BOUND, BOUND], "area": rnd.choice([1, 1, 1, 0, 2])}
    if LATER and inp["area"] == 0:
        # The Flag Race's handling (its tables' range: top 50-70, acceleration
        # 0.2-1, easings 0.03-0.125).
        inp["race"] = [rf(rnd, 50, 70), rf(rnd, 0.2, 1), rf(rnd, 0.03, 0.125), rf(rnd, 0.03, 0.125)]
    cam = {"type": rnd.choice([3, 3, 3, 1, 1, 2, 0]), "id": rnd.choice([1, 1, 0, 2]),
           "rot": [0, 0, rf(rnd, -math.pi, math.pi), ONE], "reset": int(rnd.random() < 0.25)}
    cam["reset_dirc"] = rnd.choice([rf(rnd, -math.pi, math.pi),
                                    fb(struct.unpack("<f", struct.pack("<I", inp["dirc_l"]))[0]
                                       + rnd.uniform(-0.2, 0.2))])
    n = max(frames, 1)
    script = {
        "land": [rf(rnd, -300, 300) for _ in range(14 * n)],
        "result": [rnd.choice([-1, -1, -1, rnd_attr(rnd) & 0x7FFFFFFF, 0x80000, 0x80000 | 0x1234])
                   for _ in range(14 * n)],
        "attr": [rnd_attr(rnd) for _ in range(8 * n)],
        "collide": [[rnd.choice([0, 0, 0, 1, 2, 3]), rvec(rnd, -40, 40, rnd.choice([0, ONE]))]
                    for _ in range(3 * n)],
        "line": [rnd.choice([M_ONE, M_ONE, M_ONE, rf(rnd, 0, 400), 0]) for _ in range(2 * n)],
        "eye": [rf(rnd, -math.pi, math.pi) for _ in range(n)],
        "trans": [[rnd.choice([ONE, rf(rnd, 0, 1), 0, fb(0.04)]), int(rnd.random() < 0.2)] for _ in range(2 * n)],
        "fwd": [int(rnd.random() < 0.15) for _ in range(3 * n)],
        "notes": [[[rnd.choice([1, 2, 3, 0x8005, 1, 2]), rnd.randrange(6)] for _ in range(rnd.choice([0, 0, 1, 2]))]
                  for _ in range(n)],
        "leg": [rvec(rnd, 0, 48000) for _ in range(12 * n)],
        "draw": [rnd.choice([ONE, rf(rnd, 0, 1), 0]) for _ in range(n)],
    }
    c = {"ride": ride, "input": inp, "g": [rnd.choice([0, 1]), 0], "rand": rnd.getrandbits(40), "cam": cam,
         "script": script}
    if LATER:
        rnd_seek(rnd, c)
    return c


SEEK_NAMES = [None, b"Golden Egg".hex(), b"Piney Apple".hex(), b"Magic Portal".hex(), b"Treasure".hex()]


def rnd_seek(rnd, c):
    """From Mutation on: the search's state (flags 6 and 7 too), the
    buttons, and its world: entries about the player, some within the range
    and some far, some foods, some on, some going, the event area (with
    and without a dungeon) and dungeonPos."""
    r, inp = c["ride"], c["input"]
    r["flags"] = rnd.getrandbits(8)
    player = [rf(rnd, 500, 47500), rf(rnd, 500, 47500), rf(rnd, -200, 200), ONE]
    near = lambda: [fb(struct.unpack("<f", struct.pack("<I", player[k]))[0] + rnd.uniform(-1500, 1500))  # noqa: E731
                    for k in range(2)] + [rf(rnd, -200, 200), rnd.choice([ONE, 0])]
    far = lambda: rvec(rnd, 0, 48000, rnd.choice([ONE, 0]))  # noqa: E731
    r.update({
        "seek_lead": rnd.choice([0, 0, 0, 1, 1, 1, 2, 40, rnd.randrange(-2, 41)]),
        "seek_idle": rnd.choice([0, 0, 1, -1, 40, 60, rnd.randrange(-3, 61)]),
        "seek_hold": rnd.choice([0, 0, 0, 1, 2, 10, 20, rnd.randrange(-1, 21)]),
        "seek_target": rnd.choice([near, far])(),
        "seek_range": rnd.choice([fb(1000.0), fb(2400.0), fb(1000.0), rf(rnd, 0, 3000)]),
        "seek_name": rnd.choice(SEEK_NAMES),
        "chat": rnd.choice(["", "", b"Old line".hex()]),
    })
    inp["push"] = rnd.choice([0, 0, 0, 0x10, 0x10, 0x0C, 0x04, 0x08, rnd.getrandbits(16)])
    inp["pow_r"] = rnd.choice([0, 0, 63, 64, 255, rnd.randrange(256)])

    def entry():
        return {"pos": rnd.choice([near, near, far])(), "ty": rnd.choice([0x800000, 0x800000, 0x4000, 0x810000, 0]),
                "name": rnd.choice(SEEK_NAMES[1:]), "active": int(rnd.random() < 0.8),
                "going": int(rnd.random() < 0.15)}
    c["seek"] = {
        "gims": [entry() for _ in range(rnd.choice([0, 1, 3, 6]))],
        "circles": [entry() for _ in range(rnd.choice([0, 1, 2, 4]))],
        "event_area": rnd.choice([0, 0, 0, 1, 28, 57, 110, 200, 39]),
        "dungeon": rnd.choice([near, far])()[:2] + [0, ONE],
        "player": player,
    }


# the probe side --------------------------------------------------------------------

def ser_ride(r):
    out = []
    for name, _, k in RIDE:
        v = r[name]
        out += v if k == "v" else [v]
    out += r["fs"]
    if LATER:
        out += [r["seek_lead"], r["seek_idle"], r["seek_hold"]] + r["seek_target"] + [r["seek_range"]]
        out += [r["seek_name"] or "-", r["chat"] or "-"]
    return out


def ser_seek(sw):
    out = [sw["event_area"]] + sw["dungeon"] + sw["player"]
    for es in (sw["gims"], sw["circles"]):
        out.append(len(es))
        for e in es:
            out += e["pos"] + [e["ty"], e["active"], e["going"], e["name"] or "-"]
    return out


def ser_script(s):
    out = [len(s["land"])] + s["land"]
    out += [len(s["result"])] + s["result"]
    out += [len(s["attr"])] + s["attr"]
    out += [len(s["collide"])] + [x for r, off in s["collide"] for x in [r] + off]
    out += [len(s["line"])] + s["line"]
    out += [len(s["eye"])] + s["eye"]
    out += [len(s["trans"])] + [x for f, h in s["trans"] for x in (f, h)]
    out += [len(s["fwd"])] + s["fwd"]
    out += [len(s["notes"])]
    for ns in s["notes"]:
        out += [len(ns)] + [x for ev, par in ns for x in (ev, par)]
    out += [len(s["leg"])] + [x for v in s["leg"] for x in v]
    out += [len(s["draw"])] + s["draw"]
    return out


def ser_cam(c):
    return [c["type"], c["id"]] + c["rot"] + [c["reset"], c["reset_dirc"]]


def ser_case(c):
    i = c["input"]
    race = [1] + i["race"] if i.get("race") else [0]
    later = [i["push"], i["pow_r"]] if LATER else []
    return (ser_ride(c["ride"]) + [i["pow_l"], i["dirc_l"], i["pause"], i["dne"]] + i["bounds"] + [i["area"]]
            + race + later + c["g"] + [c["rand"]] + ser_cam(c["cam"]) + ser_script(c["script"])
            + (ser_seek(c["seek"]) if LATER else []))


class Probe:
    def __init__(self):
        self.p = subprocess.Popen([EXAMPLE, ISO], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd=ROOT)

    def ask(self, words):
        self.p.stdin.write(" ".join(str(w) for w in words) + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        self.p.wait()


def plw_of(frame):
    return frame["plw"]


def normal(frames):
    """The probe's frames in the game's shapes."""
    return [{"ride": f["ride"], "seek": f["seek"], "ret": f["ret"], "g": f["g"], "rand": f["rand"], "plw": f["plw"],
             "reset": f["reset"], "calls": f["calls"]} for f in frames]


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class RideAgainstGame(unittest.TestCase):
    CASES = 60

    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-battle", "--example",
                        "ride_probe"], cwd=ROOT, check=True)
        cls.game = RideGame()
        cls.probe = Probe()

    @classmethod
    def tearDownClass(cls):
        cls.probe.close()

    def compare(self, want, got, what):
        self.assertEqual(len(want), len(got), what)
        for k, (w, g) in enumerate(zip(want, got)):
            for key in ("calls", "ride", "seek", "ret", "g", "rand", "plw", "reset"):
                self.assertEqual(w[key], g[key], f"{what} frame {k}: {key}")

    def run_fn(self, fn, rnd, frames=1, args=(), lead=None, types=None, field=False):
        c = rnd_case(rnd, frames)
        if field:
            c["input"]["area"] = 1
        if lead is not None:
            c["ride"]["seek_lead"] = rnd.choice([1, 1, 2, 40]) if lead else c["ride"]["seek_lead"]
        if types is not None:
            args = (types[c["ride"]["kind"]],)
        want = self.game.run(c, fn, frames, args)
        if fn == "main":
            got = self.probe.ask(["main", frames] + ser_case(c))["frames"]
        else:
            got = self.probe.ask(["fn", fn] + ser_case(c) + list(args))["frames"]
        # plw is written by Main only.
        if fn != "main":
            for w in want:
                w["plw"] = None
        self.compare(want, normal(got), f"{fn} {c}")

    def test_tables(self):
        got = self.probe.ask(["tables"])
        m, sym = self.game.m, self.game.sym
        self.assertEqual(got["anims"], [cstr(m, sym("puccigusoAnimTbl") + 21 * i) for i in range(7)])
        # As on the disc (the cases write the kind's digit into memory).
        prog = self.game.g.prog
        pg = sym("puccigusoAnimTblPG")
        self.assertEqual(got["anims_pg"], [prog.read(pg + 21 * i, 21).split(b"\0")[0].decode() for i in range(7)])
        tbl = sym("puccigusoCharTbl")
        self.assertEqual(got["files"], [cstr(m, m.load(tbl + 4 * i, 4)) for i in range(9)])
        a = sym("puccigusoAngleTbl")
        self.assertEqual(got["angles"], [m.load(a + 2 * i, 2, signed=True) for i in range(5)])

    def test_ctor(self):
        rnd = random.Random(1)
        for kind in list(range(9)) * 3:
            c = rnd_case(rnd)
            pos, rot, rand = rvec(rnd, 0, 48000), [0, 0, rf(rnd, -3, 3), 0], rnd.getrandbits(40)
            want = self.game.ctor(kind, rand, pos, rot, c)
            got = self.probe.ask(["new", kind, rand] + pos + rot + ser_cam(c["cam"]) + ser_script(c["script"]))
            self.assertEqual(want["ride"], got["ride"], f"kind {kind}")
            self.assertEqual(want["rand"], got["rand"], f"kind {kind}")
            self.assertEqual(want["calls"], got["calls"], f"kind {kind}")
            self.assertEqual(want["files"], ["ctu1body", got["file"]], f"kind {kind}")
            self.assertEqual(want["pg"], got["pg"], f"kind {kind}")
            self.assertEqual(want["note_fn"], CHECK_NOTE)

    def test_main(self):
        rnd = random.Random(2)
        for _ in range(self.CASES):
            self.run_fn("main", rnd, rnd.choice([1, 1, 2, 5, 20, 80]))

    def test_control_move(self):
        rnd = random.Random(3)
        for _ in range(self.CASES * 4):
            self.run_fn("control_move", rnd)

    def test_anim_ctrl(self):
        rnd = random.Random(4)
        for _ in range(self.CASES * 4):
            self.run_fn("anim_ctrl", rnd)

    def test_draw_pg(self):
        rnd = random.Random(5)
        for _ in range(self.CASES * 4):
            self.run_fn("draw_pg", rnd)

    @unittest.skipUnless(LATER, "the search is Mutation's and later")
    def test_seek_move(self):
        rnd = random.Random(11)
        for _ in range(self.CASES * 4):
            c_lead = rnd.random() < 0.8
            self.run_fn("seek_move", rnd, 1, lead=c_lead)

    @unittest.skipUnless(LATER, "the search is Mutation's and later")
    def test_seek(self):
        types = self.probe.ask(["tables"])["seek_types"]
        rnd = random.Random(12)
        for _ in range(self.CASES * 4):
            self.run_fn("seek", rnd, 1, types=types)

    @unittest.skipUnless(LATER, "the search is Mutation's and later")
    def test_seek_in_a_field(self):
        """ControlMove and Main in a field with the search on and off."""
        rnd = random.Random(13)
        for _ in range(self.CASES * 2):
            self.run_fn("control_move", rnd, 1, field=True)
            self.run_fn("main", rnd, rnd.choice([1, 2, 30, 80]), field=True)

    def test_note(self):
        rnd = random.Random(6)
        for _ in range(self.CASES * 4):
            self.run_fn("note", rnd, 1, (rnd.choice([0, 1, 2, 3, 0x8005]), rnd.randrange(4)))

    def test_smoke(self):
        rnd = random.Random(7)
        for _ in range(self.CASES * 4):
            self.run_fn("smoke", rnd, 1, (rf(rnd, -3.2, 3.2), rnd.choice([15, 3, 1, 2, 4, 8, 12, 15, 3])))

    def test_lever(self):
        m, sym = self.game.m, self.game.sym
        for v in list(range(0, 300)) + [-1, -64, 1000, 40000]:
            m.f[12] = fb(float(v))
            m.call(sym("PadLeverPower__11ccPuccigusoFf"), (OBJ,))
            want = m.f[0] & 0xFFFFFFFF
            self.assertEqual(want, self.probe.ask(["lever", fb(float(v))]), v)

    def test_adult(self):
        rnd = random.Random(8)
        m, g = self.game.m, self.game.g
        save = m.load(g.sym("saveData"), 4)
        for _ in range(200):
            words = [rnd.choice([0, 0, 1, -3, 7]) for _ in range(60)]
            for i, w in enumerate(words):
                m.store(save + 0x2194 + 2 * i, 2, w)
            for server in range(-1, 7):
                for slot in range(-1, 4):
                    want = s32(m.call(self.game.sym("ccPgAdultCheck__Fii"), (server, slot)))
                    got = self.probe.ask(["adult", server, slot] + words)
                    self.assertEqual(want, got, (server, slot, words))

    def test_exit(self):
        """ccPuccigusoExit's places and its order of calls."""
        rnd = random.Random(9)
        for _ in range(40):
            self.exit_case(rnd)

    def spc_sleep_wake(self):
        """ccSpcSleep and ccSpcWakeup: from Mutation on each takes the town
        flag (MUT gcmn 0x005cbb40, carried as ccSpcWakeup__Fv, and the
        wake 0x30 on)."""
        sym = self.game.sym
        if not LATER:
            return sym("ccSpcSleep__Fv"), sym("ccSpcWakeup__Fv")
        sleep = sym("ccSpcWakeup__Fv")
        return sleep, sleep + 0x30

    def exit_case(self, rnd):
        game = self.game
        m, sym = game.m, game.sym
        body = OBJ + (0x1F0 if LATER else 0x170)
        sleep, wake = self.spc_sleep_wake()
        c = rnd_case(rnd)
        game.put(c)
        kind = c["ride"]["kind"]
        m.store(OBJ + 0x100, 4, kind)
        pos, rot = rvec(rnd, 0, 48000), [rf(rnd, -1, 1), rf(rnd, -1, 1), rf(rnd, -math.pi, math.pi), ONE]
        game.put_vec(KITE + 0x40, pos)
        game.put_vec(KITE + 0x60, rot)
        mgr = sym("ccSpcManager")
        members = []
        for i in range(5):
            ch = REG + 0x400 * i
            m.mem[ch:ch + 0x400] = bytes(0x400)
            used = rnd.random() < 0.8
            m.store(mgr + 44 * i, 4, i if used else 0xFFFFFFFF)
            m.store(mgr + 44 * i + 0x1C, 4, ch if used else 0)
            party = rnd.choice([1, 1, 1, 0, 2])
            dead = rnd.choice([0, 0, 1])
            cid = 0 if i == 0 else rnd.choice([i, 3, 0])
            base = ch + 0x300
            m.store(ch, 4, base)
            m.store(base + 0xC, 2, cid)
            m.store(ch + 0xE0, 4, (party & 7) << 14)
            m.store(ch + 8, 2, dead)
            game.put_vec(ch + 0x40, rvec(rnd, 0, 1000))
            game.put_vec(ch + 0x60, rvec(rnd, -1, 1))
            members.append((used, party, dead, cid, ch))
        log = []
        rec = log.append
        hooks = {
            "ccDeleteCmnd__FP6ccChar": lambda mm, a, *_: rec(["ccDeleteCmnd", a]) or 0,
            "HitDisable__9ccCharHitFv": lambda mm, h, *_: rec(["HitDisable", h - 0x1A0 if h != body
                                                                else "pcgs"]) or 0,
            "HitEnable__9ccCharHitFv": lambda mm, h, *_: rec(["HitEnable", h - 0x1A0]) or 0,
            "EntryFade__8ccScFadeFiiiffff": lambda mm, f, a, b, cc, *_: rec(
                ["EntryFade", s32(a), s32(b), cc, mm.f[12], mm.f[13], mm.f[14], mm.f[15]]) or 7,
            "CheckFade__8ccScFadeFi": lambda mm, *_: 0,
            "ContinueFade__8ccScFadeFiii": lambda mm, f, i, n, cc: rec(["ContinueFade", s32(i), s32(n), cc]) or 0,
            "DeleteFade__8ccScFadeFi": lambda mm, f, i, *_: rec(["DeleteFade", s32(i)]) or 0,
            "Main__11ccPuccigusoFv": lambda mm, *_: rec(["Main"]) or 0,
            "ccBreathThread__Fi": lambda mm, *_: rec(["Breath"]) or 0,
            "ccPgBgmEnd__Fi": lambda mm, n, *_: rec(["ccPgBgmEnd", s32(n)]) or 0,
            "__dt__11ccPuccigusoFv": lambda mm, a, b, *_: rec(["~ccPucciguso"]) or 0,
            "ccFileListDeleteOne__FP10ccFileList": lambda mm, fl, *_: rec(
                ["ccFileListDeleteOne", m.load(fl, 4), cstr(mm, m.load(fl + 4, 4))]) or 0,
        }
        hooks = {sym(n): f for n, f in hooks.items()}
        hooks[wake] = (lambda mm, t, *_: rec(["ccSpcWakeup", s32(t)]) or 0) if LATER else (
            lambda mm, *_: rec(["ccSpcWakeup"]) or 0)
        saved = {a: m.hooks.get(a) for a in hooks}
        for a, f in hooks.items():
            m.hooks[a] = f
        menu = m.load(sym("ccMenu"), 4)
        m.store(menu + 0xFE, 2, 1)
        m.store(menu + 0xC, 2, 3)
        din = rnd.choice([0, 0, 0, 1])
        area = rnd.choice([1, 1, 1, 0, 2])
        m.store(game.glob["pgDIN"], 4, din)
        m.store(0x01021000 + 0x14, 4, area)
        for n in ("pgRideFlag", "pgR"):
            m.store(game.glob[n], 4, 1)
        m.store(game.glob["pcgs"], 4, OBJ)
        try:
            m.call(sym("ccPuccigusoExit__Fv"), ())
        finally:
            for a, f in saved.items():
                if f is None:
                    m.hooks.pop(a, None)
                else:
                    m.hooks[a] = f
        file = cstr(m, m.load(sym("puccigusoCharTbl") + 4 * kind, 4))
        tail = [["~ccPucciguso"], ["ccFileListDeleteOne", 11, file]]
        if din or (area != 1 and not (LATER and area == 0)):
            want = [["ccPgBgmEnd", 1]] + tail
        elif area == 0:
            # From Mutation on, in a town (the Flag Race): no fade nor
            # music, Kite's slot alone, the menu's panels and ban kept.
            want = [["ccDeleteCmnd", OBJ], ["HitDisable", "pcgs"]]
            used, party, dead, cid, ch = members[0]
            if used and party == 1 and not dead:
                want.append(["HitEnable", ch])
            want += [["ccSpcWakeup", 1]] + tail
            self.assertEqual(m.load(menu + 0xC, 2), 3)
            self.assertEqual(m.load(menu + 0xFE, 2), 1)
        else:
            want = [["ccDeleteCmnd", OBJ], ["HitDisable", "pcgs"],
                    ["EntryFade", 10, 0, 0x80000000, 0, 0, fb(512.0), fb(384.0)]]
            for used, party, dead, cid, ch in members:
                if used and party == 1 and not dead:
                    want.append(["HitEnable", ch])
            woke = ["ccSpcWakeup", 0] if LATER else ["ccSpcWakeup"]
            want += [woke, ["ContinueFade", 7, 15, 0], ["DeleteFade", 7], ["ccPgBgmEnd", 0]] + tail
            for i, (used, party, dead, cid, ch) in enumerate(members):
                if not (used and party == 1 and cid != 0):
                    continue
                self.assertEqual(game.vec(ch + 0x60), rot, i)
                self.assertEqual(game.vec(ch + 0x40), self.probe.ask(["place"] + pos + rot + [i]), i)
            self.assertEqual(m.load(menu + 0xC, 2), 1)
            self.assertEqual(m.load(menu + 0xFE, 2), 0)
        self.assertEqual(log, want)
        for n in ("pgRideFlag", "pgR", "pcgs"):
            self.assertEqual(m.load(game.glob[n], 4), 0, n)
        self.assertEqual(m.load(game.glob["camTypeLock"], 2), 0)

    def test_start(self):
        """ccPuccigusoStart's refusals, flags and order of calls."""
        rnd = random.Random(10)
        game = self.game
        m, sym = game.m, game.sym
        sleep, _ = self.spc_sleep_wake()
        for _ in range(30):
            mgr = sym("ccSpcManager")
            members = []
            for i in range(5):
                ch = REG + 0x400 * i
                m.mem[ch:ch + 0x400] = bytes(0x400)
                used = rnd.random() < 0.8
                m.store(mgr + 44 * i, 4, i if used else 0xFFFFFFFF)
                m.store(mgr + 44 * i + 0x1C, 4, ch if used else 0)
                party = rnd.choice([1, 1, 1, 0, 3])
                m.store(ch + 0xE0, 4, (party & 7) << 14)
                members.append((used, party, ch))
            log = []
            rec = log.append
            tcb = BASE + 0x9000
            hooks = {
                "ccPgBgmInit__Fv": lambda mm, *_: rec(["ccPgBgmInit"]) or 0,
                "EntryFade__8ccScFadeFiiiffff": lambda mm, f, a, b, cc, *_: rec(
                    ["EntryFade", s32(a), s32(b), cc, mm.f[12], mm.f[13], mm.f[14], mm.f[15]]) or 5,
                "CheckFade__8ccScFadeFi": lambda mm, *_: 0,
                "HitDisable__9ccCharHitFv": lambda mm, h, *_: rec(["HitDisable", h - 0x1A0]) or 0,
                "ClearConditionEffect__6ccCharFv": lambda mm, a, *_: rec(["ClearConditionEffect", a]) or 0,
                "ccLoadFLAddOne__FP10ccFileList": lambda mm, fl, *_: rec(
                    ["ccLoadFLAddOne", m.load(fl, 4), cstr(mm, m.load(fl + 4, 4))]) or 0,
                "ccStartThread__FPFPv_vii": lambda mm, fn, pri, stack, *_: rec(
                    ["ccStartThread", fn, pri, stack]) or tcb,
                "ContinueFade__8ccScFadeFiii": lambda mm, f, i, n, cc: rec(["ContinueFade", s32(i), s32(n), cc]) or 0,
                "DeleteFade__8ccScFadeFi": lambda mm, f, i, *_: rec(["DeleteFade", s32(i)]) or 0,
                "ccBgmPlay__Fi": lambda mm, n, *_: rec(["ccBgmPlay", s32(n), m.load(game.glob["camTypeLock"], 2)])
                or 0,
                "ccBreathThread__Fi": lambda mm, *_: rec(["Breath"]) or 0,
            }
            hooks = {sym(n): f for n, f in hooks.items()}
            hooks[sleep] = (lambda mm, t, *_: rec(["ccSpcSleep", s32(t)]) or 0) if LATER else (
                lambda mm, *_: rec(["ccSpcSleep"]) or 0)
            saved = {a: m.hooks.get(a) for a in hooks}
            for a, f in hooks.items():
                m.hooks[a] = f
            menu = m.load(sym("ccMenu"), 4)
            m.store(menu + 0xFE, 2, 0)
            m.store(menu + 0xC, 2, 1)
            area = rnd.choice([1, 1, 1, 0, 2])
            pg_r = rnd.choice([0, 0, 0, 1])
            kind = rnd.randrange(9)
            m.store(0x01021000 + 0x14, 4, area)
            m.store(game.glob["pgR"], 4, pg_r)
            for n in ("pgRideFlag", "pgDIN"):
                m.store(game.glob[n], 4, 7)
            m.store(tcb + 0x14, 4, 0)
            m.store(tcb + 0x18, 4, 0xFFFF)
            try:
                m.call(sym("ccPuccigusoStart__Fi"), (kind,))
            finally:
                for a, f in saved.items():
                    if f is None:
                        m.hooks.pop(a, None)
                    else:
                        m.hooks[a] = f
            town = LATER and area == 0
            if (area != 1 and not town) or pg_r:
                self.assertEqual(log, [])
                self.assertEqual(m.load(game.glob["pgRideFlag"], 4), 7)
                continue
            file = cstr(m, m.load(sym("puccigusoCharTbl") + 4 * kind, 4))
            thread = [["ccLoadFLAddOne", 11, file], ["ccStartThread", sym("ccThPucciguso__FP6ccTscb"), 49, 0x800]]
            if town:
                # From Mutation on, in a town: no music nor fade, Kite's slot
                # alone asleep (ccSpcSleep(1)), the task told it is a town's.
                want = [["ccSpcSleep", 1]]
                used, party, ch = members[0]
                if used and party == 1:
                    want += [["HitDisable", ch], ["ClearConditionEffect", ch]]
                want += thread
            else:
                want = [["ccPgBgmInit"], ["EntryFade", 10, 0, 0x80000000, 0, 0, fb(512.0), fb(384.0)],
                        ["ccSpcSleep", 0] if LATER else ["ccSpcSleep"]]
                for used, party, ch in members:
                    if used and party == 1:
                        want += [["HitDisable", ch], ["ClearConditionEffect", ch]]
                want += thread + [["ContinueFade", 5, 15, 0], ["DeleteFade", 5], ["ccBgmPlay", 0, 0]]
            self.assertEqual(log, want)
            self.assertEqual(m.load(tcb + 0x14, 4), kind)
            if LATER:
                self.assertEqual(m.load(tcb + 0x18, 4), int(town))
            self.assertEqual([m.load(game.glob[n], 4) for n in ("pgRideFlag", "pgR", "pgDIN")], [1, 1, 0])
            self.assertEqual([m.load(menu + 0xFE, 2), m.load(menu + 0xC, 2)], [1, 3])


def bulk(n):
    RideAgainstGame.CASES = n
    suite = unittest.TestLoader().loadTestsFromTestCase(RideAgainstGame)
    unittest.TextTestRunner(verbosity=2).run(suite)


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        bulk(int(sys.argv[2]))
    else:
        unittest.main()
