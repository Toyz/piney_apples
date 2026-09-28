#!/usr/bin/env python3
"""crates/piney-battle's enemy weapon controller (weapon.rs, `WeaponCtrl`)
against the game's own ccEnemyWeaponCtrl (gcmn 0x0043e760-0x0043ece8) and
ccEnemyWeapon run in eemu (the Rust VU0 machine) over test_battle's
GameBattle (GCMN.PRG loaded).

A controller is built by the game's constructor (ccEnemyWeaponCtrl(n,
info, obj), the weapons' cells and flashes from the harness's heap) over
each weapon table the race constructors name (races.rs), or a made-up one for what no table has (rgb
colours, bezier cells with both packets, two and three edges, a missing
node), with an enemy object beside it whose clump and animation hand out
a coordinate (ccCoord) per node name - or none. Then 360 frames: the
nodes' matrices turned and moved; the enemy's act (attacking, act 6, now
and then, its attack 0-5 or out of range, actCnt counting from 0), its
dispSW mostly on, its skill's type (its element); some notes (0x8005,
0x8003 and others, params -1 to 7) through note(obj, note); then
ctrl(obj). ccTransPosFW2LW is (x + 0.5, y - 0.25, z, w), so that each
call shows; sceVu0RotTransPers copies its point, so what makePacket is
handed is the point in Kite's frame; makePacket and sendPacket are
recorded, not run; the camera's rotation is the harness's (cameraGetRot
or cameraGetRot2 by checkCameraType); the flash's light group calls and
disp (the points into Kite's frame in place) are recorded. A ccRand() is
an error.

The weapon_probe example runs the port on the same rows and steps; after
each step every weapon (flags, element, shares, the list, each cell and
its points, the flash with its parts and light) and what the step put
out (the makePacket calls in order, the lights, the rays) are compared.

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
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "weapon_probe")

# Scratch memory: the enemy, its skill, the controller, the nodes'
# coordinates, the made-up rows and their names, the layer and view, the
# packet work, the heap.
BASE = 0x01E00000
OBJ, SKP, CTRL, COORDS = BASE, BASE + 0x400, BASE + 0x500, BASE + 0x1000
ROWS, NAMES, LAYER, WORK = BASE + 0x3000, BASE + 0x4000, BASE + 0x5000, BASE + 0x6000
CHUNK = BASE + 0x7000
HEAP, HEAP_END = 0x01C00000, 0x01E00000

def race_tables():
    """Every (weapon table, rows) the race constructors name
    (crates/piney-battle/src/races.rs: a row's type, then its weapon)."""
    import re
    src = open(os.path.join(ROOT, "crates", "piney-battle", "src", "races.rs")).read()
    out = []
    for a, n in re.findall(r"\(\s*-?\d+,\s*Some\(\(0x([0-9a-f_]+),\s*(\d+)\)\)", src):
        t = (int(a.replace("_", ""), 16), int(n))
        if t not in out:
            out.append(t)
    return out


TABLES = race_tables()


def f32(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def s16(v):
    return v - 0x10000 if v & 0x8000 else v


def s32(v):
    return v - 0x100000000 if v & 0x80000000 else v


def row(name, flags, scale, tension, between, life, edges, pts):
    b = struct.pack("<III", 0, name, flags) + struct.pack("<II", f32(scale), f32(tension))
    b += struct.pack("<HHHH", between, life, edges, 0) + bytes(4)
    for k in range(4):
        p = pts[k] if k < len(pts) else (0, 0, 0, 1)
        b += struct.pack("<4I", *[f32(x) for x in p])
    return b


# name index -> name, rows over them (the names are laid out at NAMES).
MADE_NAMES = ["OBJ_mdl sword", "OBJ_mdl gone", "EXT_mdlbody"]
MADE_UP = [
    # rgb colours, both packets, bezier, two edges, every attack
    (0, 0x1F80 | 0x1 | 0x4 | 0x8 | 0x10 | 0x20, 1.0, 0.0, 3, 8, 2, [(0, 0, 0, 1), (0, 90, 10, 1)]),
    # hsv, strips, spline, three edges, attacks 0-2; a node that is missing
    (1, 0x380 | 0x2 | 0x4 | 0x10 | 0x40, 0.5, 0.25, 1, 6, 3, [(5, 0, 0, 1), (5, 40, 0, 1), (0, 80, 30, 1)]),
    # the animation's node, lines only, no between cells, attacks 3-5
    (2, 0x1C00 | 0x2 | 0x8 | 0x10 | 0x40, 1.0, 0.0, 0, 5, 4,
     [(0, 0, 0, 1), (0, 0, 40, 1), (0, 0, 80, 1), (0, 0, 120, 1)]),
]


class Game:
    def __init__(self):
        import battle
        import test_battle as tb
        self.g = tb.GameBattle(battle.Data(ELF))
        m, sym = self.g.m, self.g.sym
        self.m, self.sym = m, sym
        # GameBattle's no-op sceVu0CopyVector: the weapons copy with it.
        m.hooks.pop(sym("sceVu0CopyVector"), None)
        m.mem[BASE:BASE + 0x10000] = bytes(0x10000)
        self.heap = HEAP
        at = NAMES
        self.made_names = []
        for n in MADE_NAMES:
            self.made_names.append(at)
            m.mem[at:at + len(n) + 1] = n.encode() + b"\0"
            at += 0x20
        rows = b"".join(row(self.made_names[r[0]], *r[1:]) for r in MADE_UP)
        m.mem[ROWS:ROWS + len(rows)] = rows
        m.store(sym("active__7ccLayer"), 4, LAYER)
        m.store(LAYER + 0x2C, 4, LAYER + 0x100)
        m.store(sym("worldman"), 4, LAYER + 0x400)
        m.store(OBJ + 0xCC, 4, LAYER + 0x800)
        m.store(OBJ + 0xD0, 4, LAYER + 0x900)
        m.store(OBJ + 0xD4, 4, LAYER + 0xA00)
        m.store(OBJ + 0x2CC, 4, SKP)
        self.coords = {}      # node name -> ccCoord address, None for none
        self.chunk_rot = None
        self.outs = []
        self.cam_type, self.rot, self.rot2 = 0, [0] * 4, [0] * 4
        hooks = {
            "__nw__FUi": self.new, "__nwa__FUi": self.new,
            "__ct__7ccLightFsSc": lambda mm, a0, *_: a0, "Init__11ccOmniLightFP12ccLightChunk": lambda mm, *_: 0,
            "ccSetColor__FPfUif": self.set_color,
            "ccTransPosFW2LW__FPfPf": self.fw2lw,
            "sceVu0RotTransPers": self.rot_trans_pers,
            "makePacket__12ccPrimPacketF5cczQW5cczQW5cczQW5cczQWi": self.make_packet,
            "sendPacket__12ccPrimPacketFi": lambda mm, *_: 0,
            "GetWork__16ccDrawPacketCtrlFi": lambda mm, *_: WORK,
            "SetActiveLayer__9WORLD_MANFi": lambda mm, *_: 0,
            "checkCameraType__Fv": lambda mm, *_: self.cam_type,
            "cameraGetRot__FPfi": lambda mm, out, *_: self.put(out, "<4I", *self.rot) or 0,
            "cameraGetRot2__FPfi": lambda mm, out, *_: self.put(out, "<4I", *self.rot2) or 0,
            "AddGrp__10ccLightGrpFP7ccLight": self.add_grp,
            "DelGrp__10ccLightGrpFP7ccLight": lambda mm, *_: self.outs.append(["light_off"]) or 0,
            "disp__13ccPrimRadiateFP10ccPrimPart": self.rays,
            "GetObjAdrsF__7ccClumpFPCciP19ccSubstSearchResult": self.node,
            "GetSubstAdrsF__5ccAnmFPCcb": self.node,
            "GetChunkAdrsF__8ccStreamFPCci": self.chunk,
            "ccRand__Fv": self.no_rand, "strchr": self.strchr,
        }
        for n, h in hooks.items():
            m.hooks[sym(n)] = h

    # memory ---------------------------------------------------------------------------
    def put(self, va, fmt, *v):
        self.m.mem[va:va + struct.calcsize(fmt)] = struct.pack(fmt, *v)

    def get(self, va, fmt):
        return list(struct.unpack(fmt, bytes(self.m.mem[va:va + struct.calcsize(fmt)])))

    def cstr(self, a):
        b = bytes(self.m.mem[a:a + 64])
        return b.split(b"\0")[0].decode("latin1")

    # hooks ----------------------------------------------------------------------------
    def new(self, mm, n, *_):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        if self.heap > HEAP_END:
            raise eemu.Stop("the harness's heap ran out")
        mm.mem[a:a + n] = bytes(n)
        return a

    @staticmethod
    def set_color(mm, out, rgb, *_):
        mm.store(out, 4, rgb)
        for k in (1, 2, 3):
            mm.store(out + 4 * k, 4, 0)
        return 0

    @staticmethod
    def fw2lw(mm, o, i, *_):
        x, y, z, w = [mm.load(i + 4 * k, 4) for k in range(4)]
        for k, v in enumerate((eemu.f_add(x, 0x3F000000), eemu.f_add(y, 0xBE800000), z, w)):
            mm.store(o + 4 * k, 4, v)
        return 0

    @staticmethod
    def rot_trans_pers(mm, o, mat, v, *_):
        mm.mem[o:o + 16] = bytes(mm.mem[v:v + 16])
        return 0

    def make_packet(self, mm, pk, s0, c0, s1, *_):
        c1, cont = mm.r[8] & 0xFFFFFFFF, mm.r[9] & 0xFFFFFFFF

        def col(a):
            r, g, b, al = self.get(a, "<4I")
            return (r & 0xFF) | (g & 0xFF) << 8 | (b & 0xFF) << 16 | (al & 0xFF) << 24
        self.outs.append(["mp", pk, self.get(s0, "<4I") + self.get(s1, "<4I") + [col(c0), col(c1), cont]])
        return 0

    def add_grp(self, mm, g, lg, *_):
        self.outs.append(["light_on", self.get(lg + 0x40, "<16I") + [mm.load(lg + 0x8D, 1), mm.load(lg + 0xB0, 4),
                                                                      mm.load(lg + 0xC0, 4)]])
        return 0

    def rays(self, mm, rad, part, *_):
        mm.store(rad + 0x44, 4, (mm.load(rad + 0x44, 4) + 1) & 1)
        for k in range(s16(mm.load(rad + 4, 2))):
            for v in range(4):
                a = part + 0xD0 * k + 0x20 + 0x30 * v
                self.fw2lw(mm, a, a)
        self.outs.append(["rays"])
        return 0

    def chunk(self, mm, stream, name, *_):
        """A chunk of the enemy's file: a made-up one, its place (+0x10)
        and turn (+0x20) the harness's."""
        return CHUNK if self.chunk_rot is not None else 0

    def node(self, mm, owner, name, *_):
        return self.coords.get(self.cstr(name)) or 0

    @staticmethod
    def strchr(mm, a, c, *_):
        c &= 0xFF
        while True:
            b = mm.load(a, 1)
            if b == c:
                return a
            if b == 0:
                return 0
            a += 1

    @staticmethod
    def no_rand(mm, *_):
        raise eemu.Stop("the weapons drew ccRand()")

    # the controller -----------------------------------------------------------------------
    def build(self, info, n, names):
        """The enemy's nodes (a coordinate each, or none) and the game's
        constructor over the rows."""
        self.coords = {}
        for k, (nm, present) in enumerate(names.items()):
            self.coords[nm] = COORDS + 0x80 * k if present else None
        self.heap = HEAP
        self.m.mem[CTRL:CTRL + 8] = bytes(8)
        # setEdgeRate's 0 / 0 (a one-edge weapon): libgcc's dptofp packs the
        # NaN with an exponent __unpack_d never set, whatever the stack held;
        # zeros here, as the port takes it.
        self.m.mem[eemu.STACK_TOP - 0x4000:eemu.STACK_TOP] = bytes(0x4000)
        self.m.call(self.sym("__ct__17ccEnemyWeaponCtrlFiP13ccEnemyWpInfoP10ccEntryObj"), (CTRL, n, info, OBJ))

    def set_matrix(self, name, mat):
        a = self.coords.get(name)
        if a:
            self.put(a, "<16I", *mat)

    def enemy(self, at, ty):
        disp, act, cnt, atk = at[:4]
        m = self.m
        m.store(OBJ + 0xE0, 1, 0x10 if disp else 0)
        self.put(OBJ + 0x25A, "<hhh", act, cnt, atk)
        m.store(SKP + 0x2C, 4, ty)

    def note(self, at, ty, event, param):
        self.enemy(at, ty)
        self.outs = []
        self.put(LAYER + 0xC00, "<IIi", 0, event, param)
        self.m.call(self.sym("note__17ccEnemyWeaponCtrlFP10ccEntryObjP9ccAnmNote"), (CTRL, OBJ, LAYER + 0xC00))
        return self.outs

    def ctrl(self, at, ty):
        self.enemy(at, ty)
        self.outs = []
        self.m.call(self.sym("ctrl__17ccEnemyWeaponCtrlFP10ccEntryObj"), (CTRL, OBJ))
        return self.outs

    def weapons(self):
        m = self.m
        out, w = [], m.load(CTRL + 4, 4)
        for _ in range(s32(m.load(CTRL, 4))):
            out.append(w)
            w = m.load(w + 0x70, 4)
        return out

    def read(self):
        return [self.read_weapon(w) for w in self.weapons()]

    def read_weapon(self, w):
        m = self.m
        cells, n = m.load(w + 0x20, 4), s16(m.load(w + 0xC, 2))
        edges = s16(m.load(w + 0xE, 2))

        def ix(a):
            return -1 if a == 0 else (a - cells) // 0xD0
        head = [m.load(w, 1) & 7, m.load(w + 4, 4), s32(m.load(w + 8, 4)), edges] + self.get(w + 0x10, "<4I")
        head += [ix(m.load(w + 0x24, 4)), ix(m.load(w + 0x28, 4)), s32(m.load(w + 0x2C, 4)),
                 s32(m.load(w + 0x40, 4)), s32(m.load(w + 0x48, 4)), s32(m.load(w + 0x5C, 4))]
        cl = []
        for k in range(n):
            c = cells + 0xD0 * k
            v = [m.load(c, 1) & 1, s16(m.load(c + 2, 2)), m.load(c + 4, 4), ix(m.load(c + 8, 4)),
                 ix(m.load(c + 0xC, 4))]
            for e in range(min(edges, 4)):
                p = c + 0x10 + 0x30 * e
                v += [m.load(p, 1) & 1, m.load(p + 4, 4)] + self.get(p + 0x10, "<4I")
            cl.append(v)
        return [head, cl, self.read_rad(m.load(w + 0x44, 4))]

    def read_rad(self, r):
        """A ccPrimRadiate as the probe's rad_vec lists it."""
        m = self.m
        v = [m.load(r, 1) & 3, s16(m.load(r + 2, 2)), s16(m.load(r + 4, 2))]
        v += self.get(r + 8, "<11I")
        v += [s32(m.load(r + 0x44, 4))]
        v += self.get(r + 0x60, "<4I") + self.get(r + 0x70, "<4I") + self.get(r + 0x80, "<4h")
        v += self.get(r + 0x88, "<4I")
        lg = m.load(r + 0x58, 4)
        v += self.get(lg + 0x40, "<16I") + [m.load(lg + 0x8D, 1), m.load(lg + 0xB0, 4), m.load(lg + 0xC0, 4)]
        v += self.get(lg + 0xC4, "<3I")
        parts = m.load(r + 0x3C, 4)
        for k in range(s16(m.load(r + 4, 2))):
            a = parts + 0xD0 * k
            v += [m.load(a + 8, 4), m.load(a + 0xC, 4)]
            for q in range(4):
                b = a + 0x10 + 0x30 * q
                v += [m.load(b + 4, 4)] + self.get(b + 0x10, "<4I")
        return v

    def packet_of(self, pk):
        """makePacket's packet: (weapon, 0 the strips or 1 the lines)."""
        for i, w in enumerate(self.weapons()):
            if pk in (w + 0x48, w + 0x5C):
                return i, 0 if pk == w + 0x48 else 1
        return None


class Probe:
    def __init__(self):
        self.p = subprocess.Popen([EXAMPLE], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd=ROOT)

    def ask(self, line):
        self.p.stdin.write(line + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        self.p.wait()


def element(ty):
    """ccSkillCheckTypeAttribute(type): the first element bit (4 << i),
    2 + i, else 0."""
    return next((2 + i for i in range(6) if ty & (4 << i)), 0)


def matrix(rx, ry, rz, t):
    """A node's world matrix (rows x, y, z, translation) as float bits."""
    cx, sx, cy, sy, cz, sz = math.cos(rx), math.sin(rx), math.cos(ry), math.sin(ry), math.cos(rz), math.sin(rz)
    r = [[cy * cz, cy * sz, -sy], [sx * sy * cz - cx * sz, sx * sy * sz + cx * cz, sx * cy],
         [cx * sy * cz + sx * sz, cx * sy * sz - sx * cz, cx * cy]]
    rows = [r[0] + [0.0], r[1] + [0.0], r[2] + [0.0], list(t) + [1.0]]
    return [f32(x) for rr in rows for x in rr]


def diffs(a, b, path="", out=None):
    """Where two nested lists differ (paths and the two values), at most 12."""
    out = [] if out is None else out
    if isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            out.append("%s: lengths %d != %d" % (path, len(a), len(b)))
        for i, (x, y) in enumerate(zip(a, b)):
            if len(out) < 12:
                diffs(x, y, "%s[%d]" % (path, i), out)
    elif a != b:
        out.append("%s: %r != %r" % (path, a, b))
    return out


def game_outs(game, outs):
    """The game's step as the probe lists it."""
    o = []
    for x in outs:
        if x[0] == "mp":
            w, which = game.packet_of(x[1])
            o.append(["mp", w, which] + x[2])
        else:
            o.append(x)
    return o


def probe_outs(outs):
    """The probe's step: each trail's pairs as makePacketCell hands them
    (the strip's, then the line's, pair by pair), the rest as they are."""
    o = []
    for x in outs:
        if x[0] == "trail":
            w, polys, lines = x[1], x[2], x[3]
            for k in range(max(len(polys or []), len(lines or []))):
                if polys is not None:
                    o.append(["mp", w, 0] + polys[k])
                if lines is not None:
                    o.append(["mp", w, 1] + lines[k])
        else:
            o.append(x)
    return o


@unittest.skipUnless(os.path.exists(ELF) and shutil.which("cargo"), "needs the extracted disc and cargo")
class EnemyWeaponAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-battle", "--example",
                        "weapon_probe"], cwd=ROOT, check=True)
        cls.game = Game()
        cls.probe = Probe()

    @classmethod
    def tearDownClass(cls):
        cls.probe.close()

    def run_table(self, info, n, seed, frames=360):
        game, probe = self.game, self.probe
        rng = random.Random(seed)
        m = game.m
        names = {}
        game.chunk_rot = None
        for k in range(n):
            nm = game.cstr(m.load(info + 0x60 * k + 4, 4))
            names.setdefault(nm, nm != "OBJ_mdl gone")
        for k in range(n):
            a = m.load(info + 0x60 * k, 4)
            if a and game.cstr(a):
                game.chunk_rot = ([f32(rng.uniform(-50, 50)) for _ in range(3)] + [f32(1.0)]
                                  + [f32(rng.uniform(-3, 3)) for _ in range(3)] + [0])
                game.put(CHUNK + 0x10, "<8I", *game.chunk_rot)
        game.build(info, n, names)
        rows = []
        for k in range(n):
            a = info + 0x60 * k
            nm = game.cstr(m.load(a + 4, 4))
            ch = game.cstr(m.load(a, 4)) if m.load(a, 4) else ""
            if ch:
                probe.ask("chunk %s %s" % (ch.encode().hex(), " ".join("%x" % x for x in game.chunk_rot)))
            rows.append(bytes(m.mem[a:a + 0x60]).hex() + ":" + ch.encode().hex() + ":" + nm.encode().hex())
        got = probe.ask("new " + " ".join(rows))
        self.assertEqual(game.read(), got["w"], "table %x built" % info)
        pose = {nm: [rng.uniform(-3, 3) for _ in range(3)] + [rng.uniform(-500, 500) for _ in range(3)]
                for nm in names}
        act, cnt, atk, disp, ty, stats = 0, 0, 0, 1, 0, {"mp": 0, "rays": 0, "light_off": 0, "notes": 0}
        for frame in range(frames):
            for nm, present in names.items():
                p = pose[nm]
                for i in range(3):
                    p[i] += rng.uniform(-0.3, 0.3)
                    p[3 + i] += rng.uniform(-40, 40)
                mat = matrix(p[0], p[1], p[2], p[3:])
                game.set_matrix(nm, mat)
                probe.ask("mat %s %s" % (nm.encode().hex(), " ".join("%x" % x for x in mat) if present else "none"))
            # The enemy's act: an attack now and then, its count from 0.
            if act == 6 and rng.random() < 0.03:
                act, cnt = rng.choice([0, 1, 7]), 0
            elif act != 6 and rng.random() < 0.08:
                act, cnt, atk = 6, 0, rng.choice([0, 1, 2, 3, 4, 5, 0, 1, 2, 3, 4, 5, 6, -1])
                ty = rng.choice([0, 4, 8, 0x10, 0x20, 0x40, 0x80, 0x1C, 0x100])
            else:
                cnt += 1
            if rng.random() < 0.05:
                disp = 1 - disp if disp == 0 or rng.random() < 0.3 else disp
            at = [disp, act, cnt, atk]
            at_s = "%d %d %d %d %d" % (disp, act, cnt, atk, element(ty))
            while rng.random() < 0.15:
                ev = rng.choice([0x8005, 0x8005, 0x8003, 0x8003, 1, 2, 0x8002])
                param = rng.choice([-1, 0, 1, 2, 3, 4, 5, 7])
                want = game_outs(game, game.note(at, ty, ev, param))
                got = probe.ask("note %d %d %s" % (ev, param, at_s))
                self.assertEqual(game.read(), got["w"], "table %x frame %d note %x %d" % (info, frame, ev, param))
                self.assertEqual(want, probe_outs(got["out"]))
                stats["notes"] += 1
            game.cam_type = rng.choice([0, 1])
            game.rot = [f32(rng.uniform(-3, 3)) for _ in range(3)] + [0]
            game.rot2 = [f32(rng.uniform(-3, 3)) for _ in range(3)] + [0]
            cam = game.rot2 if game.cam_type == 1 else game.rot
            want = game_outs(game, game.ctrl(at, ty))
            got = probe.ask("ctrl %s %s" % (at_s, " ".join("%x" % x for x in cam)))
            have = game.read()
            self.assertEqual(have, got["w"], "table %x frame %d (act %d cnt %d atk %d disp %d): %s"
                             % (info, frame, act, cnt, atk, disp, diffs(have, got["w"])))
            po = probe_outs(got["out"])
            self.assertEqual(want, po, "table %x frame %d outs: %s" % (info, frame, diffs(want, po)))
            for x in want:
                if x[0] in stats:
                    stats[x[0]] += 1
        return stats

    def test_tables(self):
        seen = {"mp": 0, "rays": 0, "light_off": 0, "notes": 0}
        for i, (info, n) in enumerate(TABLES + [(ROWS, len(MADE_UP))]):
            with self.subTest(table=hex(info)):
                s = self.run_table(info, n, 1000 + i)
                for k in seen:
                    seen[k] += s[k]
        print("makePacket calls %(mp)d, flash frames %(rays)d, flashes out %(light_off)d, notes %(notes)d" % seen)
        # The trails were drawn and the flashes shone and went out.
        self.assertTrue(all(v > 0 for v in seen.values()), seen)


if __name__ == "__main__":
    unittest.main()
