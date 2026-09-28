#!/usr/bin/env python3
"""crates/piney-effect against the game's own effect code run in eemu.

The effect_probe example runs the port's effects on the requests this sends
it; the same requests go through the game's functions in eemu (tools/
test_anim.py's machine: eemu_rs when built, else eemu.py), over the real
effect files' objects:

  - EffectMachine: main and GCMN.PRG with what the effect code reaches for -
    ccSys's scratch stack, the game's area (a town or a field), WORLD_MAN's
    bounds and layers, plw, the camera list and the active camera,
    characters (ccChar with a parameter row), rand() as newlib's LCG seeded
    as the probe seeds it - and ccEffectCtrl built by its own constructor
    over the effect files (ccStream::GetCCSAdrs / GetChunkAdrsF served from
    DATA.BIN: a real ccEffChunk for each EFF_ object, a named block for the
    rest). What it draws is recorded, not drawn: ccClump::Init / Draw /
    SetTransparency, ccAnm::SetAnm / Draw, ccEff::Draw with the ccEff as it
    stands, the active layer each is sent on; ccAnm::_AnimateForward is
    tools/anim.py's playback (itself checked against the game by
    tools/test_anim.py). Sounds (ccSeOn3D) and particle generators started
    (startParticleGenerator) are recorded too.
  - The arrival (ArrivalAgainstGame): effTransfer on a character, then
    ccEffectCtrl::Main frame by frame until every effect has ended, beside
    the port; each frame every live slot's whole state, the draws in order
    (clump, matrix, transparency, layer), the sounds, the generators and
    rand's state after the frame.

Other harnesses build on EffectMachine (tools/test_effect_*_rs.py).

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import os
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
TARGET = os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target"))
PROFILE = os.environ.get("PINEY_PROFILE", "debug")
EXAMPLE = os.path.join(TARGET, PROFILE, "examples", "effect_probe")

ONE = 0x3F800000
# gp globals and fixed objects (INF SLUS_202.67, gcmn.prg).
CCSYS, GAME_P, WORLDMAN, PLW = inf_va(0x003788E0), inf_va(0x003789CC), inf_va(0x00378A7C), inf_va(0x00730300)
CAMID, CAMERA_LIST, ACTIVE_CAM = inf_va(0x0037897C), inf_va(0x002FB6F0), inf_va(0x0037896C)
LAYER_ACTIVE, DRAWENV_ACTIVE = inf_va(0x003788D8), inf_va(0x003788C4)
EFFC, EFFWORK, EFFECT_SERIAL = inf_va(0x00378ABC), inf_va(0x003E4A60), inf_va(0x00378ADC)
EFFECT_CCS_TBL = inf_va(0x002FD4F0)
PARTICLE_GENERATOR_TBL = inf_va(0x003402F0)
# Scratch memory for the interpreter.
SYS, SCRATCH, GAME, WM, PLAYER, CAM, EYE, DRAWENV = (0x01000000, 0x01010000, 0x01020000, 0x01021000, 0x01022000,
                                                     0x01023000, 0x01023100, 0x01024000)
LAYERS, CHARS = 0x01025000, 0x01030000
HEAP, HEAP_END = 0x01100000, 0x01F00000
# WORLD_MAN's layers by SetActiveLayer's index: +0x4bc.. and their priorities.
WM_LAYERS = {0x4BC: 0, 0x4C0: -10, 0x4C4: -20, 0x4C8: 20, 0x4CC: 10, 0x4D0: 30}
SYSLAYER = inf_va(0x00379B10)


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def build(example="effect_probe"):
    args = [shutil.which("cargo"), "build", "-q", "-p", "piney-effect", "--example", example]
    if PROFILE == "release":
        args.append("--release")
    subprocess.run(args, cwd=ROOT, check=True)


def ask(lines, example=EXAMPLE):
    p = subprocess.run([example, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True,
                       check=True, cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


def hexs(*v):
    return " ".join("%x" % (x & 0xFFFFFFFF) for x in v)


class Rand:
    """newlib rand(): the probe's `Rand`."""

    def __init__(self, seed):
        self.s = seed

    def next(self, *_):
        self.s = (self.s * 6364136223846793005 + 1) & ((1 << 64) - 1)
        return (self.s >> 32) & 0x7FFFFFFF


def eff_chunks(c):
    """Every 0x0e00 chunk of a Ccs: {object name: (fields, pats)}."""
    out = {}
    for off, t, _n, _end in c.chunks():
        if t is None or t & 0xFFFF != 0x0E00:
            continue
        p = off + 8
        obj, tex, flag, zoffs, unk, n = struct.unpack_from("<IIHhHH", c.data, p)
        corners = struct.unpack_from("<4I", c.data, p + 16)
        w, h = struct.unpack_from("<HH", c.data, p + 32)
        pats = [struct.unpack_from("<HHH", c.data, p + 36 + 8 * i) for i in range(n)]
        out[c.objects[obj][0]] = dict(tex=c.objects[tex][0], flag=flag, zoffs=zoffs, unk=unk, corners=corners,
                                      w=w, h=h, pats=pats)
    return out


class EffectMachine:
    """The game's effect system in eemu, as the module docstring says."""

    def __init__(self, town=True, seed=1):
        import anim
        import ccs
        import gzarc
        from image import Program
        from test_anim import machine_class
        self.prog = Program(ELF, "gcmn")
        self.m = machine_class()(self.prog)
        self.sym = lambda n: self.prog.symbol_named(n).value     # noqa: E731
        m, sym = self.m, self.sym
        self.heap = HEAP
        for name in ("ccMalloc__FUi", "__nw__FUi"):
            m.hooks[sym(name)] = self.malloc
        for name in ("ccFree__FPv", "__dl__FPv"):
            m.hooks[sym(name)] = lambda mm, *a: 0
        # The effect files, by effectCCSTbl.
        data = gzarc.open_bytes(DATA)
        members = {mm.name.lower(): mm for mm in gzarc.members(data)}
        self.files = []
        for i in range(13):
            name = self.prog.cstr(EFFECT_CCS_TBL + 44 * i).decode("latin-1")
            if not name:
                break
            self.files.append(name[:-4].lower())
        self.ccs = {n: ccs.Ccs(gzarc.inflate(data, members[n + ".cmp"])) for n in self.files}
        self.anims, self.effs = {}, {}
        for n, c in self.ccs.items():
            for _, a in anim.animations(c):
                self.anims.setdefault(c.objects[a.object][0], a)
            self.effs.update(eff_chunks(c))
        self.rand = Rand(seed)
        m.hooks[sym("rand")] = self.rand.next
        # ccSys, the game, WORLD_MAN, plw, the cameras, the draw environment.
        m.store(CCSYS, 4, SYS)
        m.store(SYS + 604, 4, SCRATCH)
        m.store(GAME_P, 4, GAME)
        m.store(GAME + 0x14, 4, 0 if town else 1)
        m.store(WORLDMAN, 4, WM)
        for off, v in ((0x420, -24000.0), (0x424, -24000.0), (0x428, 24000.0), (0x42C, 24000.0)):
            m.store(WM + off, 4, fb(v))
        self.layer_pri = {SYSLAYER: 0}
        for k, (off, pri) in enumerate(WM_LAYERS.items()):
            lay = LAYERS + 0x100 * k
            m.store(WM + off, 4, lay)
            self.layer_pri[lay] = pri
        m.store(PLW, 4, PLAYER)
        m.store(CAMID, 2, 0)
        m.store(CAMERA_LIST, 4, EYE)
        m.store(ACTIVE_CAM, 4, CAM)
        m.store(DRAWENV_ACTIVE, 4, DRAWENV)
        self.set_player((0.0, 0.0, 0.0))
        self.chars, self.char_of = {}, {}
        self.names, self.chunks = {}, {}
        self.anm, self.clump = {}, {}
        self.events = []
        nop = lambda mm, *a: 0                     # noqa: E731
        for name in ("SetFogSw__5ccAnmFi", "SetFogSw__7ccClumpFi", "__dt__5ccAnmFv", "__dt__7ccClumpFv",
                     "__ct__7ccCoordFv", "__ct__22ccEffectElementManagerFv", "Main__22ccEffectElementManagerFv",
                     "__ct__16ccDrawPacketCtrlFv", "Init__7ccLayerFsP6ccView", "SetFrame__6ccViewFffffffff"):
            m.hooks[sym(name)] = nop
        m.hooks[sym("GetCCSAdrs__8ccStreamFPCc")] = self.ccs_adrs
        m.hooks[sym("GetChunkAdrsF__8ccStreamFPCci")] = self.chunk_adrs
        m.hooks[sym("Init__7ccClumpFP12ccClumpChunk")] = self.clump_init
        m.hooks[sym("__ct__5ccAnmFv")] = lambda mm, a, *r: a
        m.hooks[sym("SetAnm__5ccAnmFP10ccAnmChunkUi")] = self.set_anm
        m.hooks[sym("_AnimateForward__5ccAnmFUi")] = self.forward
        m.hooks[sym("Draw__5ccAnmFv")] = self.anm_draw
        m.hooks[sym("Draw__7ccClumpFv")] = self.clump_draw
        m.hooks[sym("SetTransparency__7ccClumpFf")] = self.clump_transparency
        m.hooks[sym("Draw__5ccEffFUs")] = self.eff_draw
        m.hooks[sym("ccSeOn3D__FiPf")] = lambda mm, n, pos, *a: self.events.append(
            ["se3d", n, self.rvec(pos)]) or 0
        m.hooks[sym("startParticleGenerator__FP19ccParticleGenerator")] = self.start_generator
        effc = self.malloc(m, 0x2C0)
        self.call("__ct__12ccEffectCtrlFi", effc, 0)
        m.store(EFFC, 4, effc)
        self.effc = effc
        self.draws, self.gens = [], []

    # memory ------------------------------------------------------------------
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
        return self.m.call(self.sym(name) if isinstance(name, str) else name, list(args), limit=200_000_000)

    # the world ---------------------------------------------------------------
    def set_player(self, pos):
        self.vec(PLAYER + 0x40, [fb(v) for v in pos] + [ONE])

    def set_camera(self, eye, pos, view):
        self.vec(EYE, [fb(v) for v in eye] + [ONE])
        self.vec(CAM, [fb(v) for v in pos] + [ONE])
        self.vec(CAM + 0x10, [fb(v) for v in view] + [ONE])

    def add_char(self, cid, pos, height, width, dirc=(0.0, 0.0, 0.0)):
        """A ccChar (0xe0) with a parameter row (+0x18 height, +0x1c width)."""
        ch = CHARS + 0x200 * len(self.chars)
        param = ch + 0x100
        self.m.mem[ch:ch + 0x200] = bytes(0x200)
        self.m.store(ch, 4, param)
        self.vec(ch + 0x40, [fb(v) for v in pos] + [ONE])
        self.vec(ch + 0x60, [fb(v) for v in dirc] + [0])
        self.m.store(param + 0x18, 4, fb(height))
        self.m.store(param + 0x1C, 4, fb(width))
        self.chars[cid] = ch
        self.char_of[ch] = cid
        return ch

    def move_char(self, cid, pos):
        self.vec(self.chars[cid] + 0x40, [fb(v) for v in pos] + [ONE])

    # the stream --------------------------------------------------------------
    def ccs_adrs(self, m, name, *a):
        from eemu import _cstr
        return 0x10000 + self.files.index(_cstr(m, name).decode().lower())

    def chunk_adrs(self, m, stream, name, *a):
        from eemu import _cstr
        s = _cstr(m, name).decode()
        if s not in self.chunks:
            addr = self.malloc(m, 0x40)
            e = self.effs.get(s)
            if e is not None:
                # A ccEffChunk as Decode_Eff leaves it: +0x0c texIndex (a
                # ccChunkIndex with no texture: +0x2c 0), +0x18 flag, +0x1a
                # zoffs, +0x1c patNum, +0x20 x0 y0 x1 y1, +0x30 wh, +0x34 pat.
                index = self.malloc(m, 0x40)
                pats = self.malloc(m, 8 * max(1, len(e["pats"])))
                for i, (u, v, t) in enumerate(e["pats"]):
                    m.store(pats + 8 * i, 2, u)
                    m.store(pats + 8 * i + 2, 2, v)
                    m.store(pats + 8 * i + 4, 2, t)
                m.store(addr + 0x0C, 4, index)
                m.store(addr + 0x18, 2, e["flag"])
                m.store(addr + 0x1A, 2, e["zoffs"])
                m.store(addr + 0x1C, 2, len(e["pats"]))
                for i, c in enumerate(e["corners"]):
                    m.store(addr + 0x20 + 4 * i, 4, c)
                m.store(addr + 0x30, 4, e["w"] | e["h"] << 16)
                m.store(addr + 0x34, 4, pats)
            self.chunks[s] = addr
            self.names[addr] = s
        return self.chunks[s]

    # the draw objects --------------------------------------------------------
    def clump_init(self, m, clump, chunk, *a):
        self.clump[clump] = self.names.get(chunk, "?")
        return 0

    def set_anm(self, m, anm, chunk, *a):
        self.anm[anm] = [self.names[chunk], 0]
        m.store(anm + 172, 4, 1)
        m.store(anm + 156, 2, 256)
        return 0

    def forward(self, m, anm, spd, *a):
        st = self.anm[anm]
        f = self.anims[st[0]].forward(st[1], spd & 0xFFFF)
        st[1] = f.time
        return int(f.ended)

    def layer(self):
        return self.layer_pri.get(self.m.load(LAYER_ACTIVE, 4), None)

    def anm_draw(self, m, anm, *a):
        # ccAnm +0x88 transparency; the coordinate's matrix at +0x40 as
        # ccCoord::SetMatrix_* left it (the ccAnm is its own ccCoord).
        self.draws.append(["anm", self.anm[anm][0], self.anm[anm][1], self.rvec(anm + 0x40, 16),
                           m.load(anm + 0x88, 4), self.layer()])
        return 0

    def clump_transparency(self, m, clump, *a):
        self.clump_alpha = m.f[12]
        return 0

    def clump_draw(self, m, clump, *a):
        self.draws.append(["clump", self.clump[clump], self.rvec(clump + 0x40, 16), self.clump_alpha, self.layer()])
        return 0

    def eff_draw(self, m, eff, pat, *a):
        from test_effect_rs import eff_index
        self.draws.append(["eff", eff_index(self, eff), pat & 0xFFFF, self.rvec(eff + 0x10), m.load(eff + 0x20, 4),
                           m.load(eff + 0x24, 4), m.load(eff + 0x28, 4), m.load(eff + 0x2C, 4),
                           m.load(eff + 0x34, 4), self.layer()])
        return 0

    def start_generator(self, m, g, *a):
        row = (m.load(g, 4) - PARTICLE_GENERATOR_TBL) // 0x38
        sync = m.load(g + 0x18, 4)
        ref = None
        for cid, ch in self.chars.items():
            if sync == ch + 0x40:
                ref = ["pos", cid]
        self.gens.append([row, ref, (m.load(g + 4, 1) >> 4) & 1, self.rvec(g + 0x80)])
        return 0

    # the effects --------------------------------------------------------------
    def slot(self, i, base=EFFWORK, n=500):
        """Slot `i` of the work at `base` (`n` slots: effWork's 500, or
        effStrWork's 50)."""
        m = self.m
        a = base + 0xC0 * i
        bits = m.load(a + 0x60, 2)
        target = m.load(a + 0x88, 4)
        eff = m.load(a + 0x8C, 4)
        link = m.load(a + 0x98, 4)
        obj = None
        if eff:
            if eff in self.clump:
                obj = ["clump", self.clump[eff]]
            elif eff in self.anm:
                obj = ["anm", self.anm[eff][0], self.anm[eff][1]]
            else:
                obj = ["eff", m.load(eff + 0x20, 4), m.load(eff + 0x24, 4), m.load(eff + 0x28, 4),
                       m.load(eff + 0x2C, 4), m.load(eff + 0x34, 4), m.load(eff + 0x60, 2), self.rvec(eff + 0x10)]

        def ptr(p):
            if not p:
                return None
            for cid, ch in self.chars.items():
                if p == ch + 0x40:
                    return ["pos", cid]
                if p == ch + 0x60:
                    return ["dirc", cid]
            if base <= p < base + 0xC0 * n:
                k, off = divmod(p - base, 0xC0)
                return ["effpos" if off == 0 else "effrot", k]
            return ["?", p]
        return {
            "i": i, "id": m.load(a + 0x6C, 2, True), "status": m.load(a + 0x6E, 2, True),
            "life": m.load(a + 0x70, 2, True), "age": m.load(a + 0x72, 2, True), "cnt": m.load(a + 0x74, 2, True),
            "pat": m.load(a + 0x76, 2), "bits": bits & 0x1FF, "param": m.load(a + 0x64, 4, True),
            "flags": m.load(a + 0x68, 4, True), "pos": self.rvec(a), "offset": self.rvec(a + 0x10),
            "rot": self.rvec(a + 0x20), "speed": self.rvec(a + 0x30), "scale": self.rvec(a + 0x40),
            "posT": self.rvec(a + 0x50), "rotSpeed": [m.load(a + 0x78 + 2 * k, 2) for k in range(4)],
            "velocity": m.load(a + 0x80, 4), "transparency": m.load(a + 0x84, 4),
            "target": self.char_of.get(target, -1 if target == 0 else target), "posPtr": ptr(m.load(a + 0x90, 4)),
            "rotPtr": ptr(m.load(a + 0x94, 4)),
            "link": -1 if link == 0 else (link - base) // 0xC0, "temp": self.rvec(a + 0xA0),
            "sn": m.load(a + 0xB0, 4), "obj": obj,
        }

    def spawn(self, eid):
        """ccNewEffect(id): the slot, or -1."""
        p = self.call("ccNewEffect__Fi", eid & 0xFFFFFFFF)
        return -1 if p == 0 else (p - EFFWORK) // 0xC0

    def layer_of(self, pri):
        for lay, p in self.layer_pri.items():
            if p == pri and lay != SYSLAYER:
                return lay
        lay = self.malloc(self.m, 0x100)
        self.layer_pri[lay] = pri
        return lay

    def set_field(self, slot, key, *v):
        """A slot's field, as the probe's `set` takes it."""
        m, a = self.m, EFFWORK + 0xC0 * slot
        offs = {"pos": 0x00, "offset": 0x10, "rot": 0x20, "speed": 0x30, "scale": 0x40, "temp": 0xA0}
        if key in offs:
            self.vec(a + offs[key], list(v))
        elif key == "bits":
            m.store(a + 0x60, 2, (m.load(a + 0x60, 2) & ~0x1FF) | (v[0] & 0x1FF))
        elif key in ("life", "age", "cnt", "pat"):
            m.store(a + {"life": 0x70, "age": 0x72, "cnt": 0x74, "pat": 0x76}[key], 2, v[0])
        elif key == "transparency":
            m.store(a + 0x84, 4, v[0])
        elif key == "target":
            m.store(a + 0x88, 4, 0 if v[0] == 0xFFFFFFFF else self.chars[v[0]])
        elif key in ("posptr", "rotptr"):
            p = 0
            if v[0] != 0xFFFFFFFF:
                k, n = v
                p = [lambda: self.chars[n] + 0x40, lambda: self.chars[n] + 0x60, lambda: EFFWORK + 0xC0 * n,
                     lambda: EFFWORK + 0xC0 * n + 0x20][k]()
            m.store(a + (0x90 if key == "posptr" else 0x94), 4, p)
        elif key == "link":
            m.store(a + 0x98, 4, 0 if v[0] == 0xFFFFFFFF else EFFWORK + 0xC0 * v[0])
        elif key == "layer":
            m.store(a + 0x9C, 4, 0 if v[0] == 0xFFFFFFFF else self.layer_of(v[0] - (1 << 32) * (v[0] >> 31)))
        else:
            raise KeyError(key)

    def patch(self, eid, ccs_name, obj, kind):
        """effectTbl[id] names another object and type: its type in the
        table, and adrs[id] as the constructor would have looked it up."""
        self.m.store(inf_va(0x0033F240) + 12 * eid + 8, 4, kind)
        self.m.store(self.effc + 0x0C + 4 * eid, 4, self.chunk_named(obj))

    def chunk_named(self, s):
        m = self.m
        name = self.malloc(m, len(s) + 1)
        m.mem[name:name + len(s) + 1] = s.encode() + b"\0"
        return self.chunk_adrs(m, 0, name)

    def live(self, base=EFFWORK, n=500):
        return [self.slot(i, base, n) for i in range(n) if self.m.load(base + 0xC0 * i + 0x6E, 2)]

    def frame(self):
        """One ccEffectCtrl::Main, as the probe's `frame` answers it."""
        self.draws, self.events, self.gens = [], [], []
        self.call("Main__12ccEffectCtrlFv", self.effc)
        return {"slots": self.live(), "draws": self.draws, "events": self.events, "gens": self.gens,
                "rand": self.rand.s}


def eff_index(em, eff):
    """The Eff chunk a ccEff was made from, as its index in its file (the
    probe's `eff.chunk`): found by its patterns' address."""
    pat = em.m.load(eff + 0x48, 4)
    for name, addr in em.chunks.items():
        if em.m.load(addr + 0x34, 4) == pat and name in em.effs:
            for f, c in em.ccs.items():
                names = list(eff_chunks(c))
                if name in names:
                    return names.index(name)
    return -1


def compare(test, want, got, what):
    """Frame dicts, key by key, with the first difference named."""
    for k in want:
        if want[k] != got.get(k):
            if k == "slots":
                for a, b in zip(want[k], got[k]):
                    if a != b:
                        diff = {kk: (a[kk], b.get(kk)) for kk in a if a[kk] != b.get(kk)}
                        test.fail("%s: slot %d differs: %s" % (what, a["i"], diff))
            test.assertEqual(want[k], got.get(k), "%s: %s" % (what, k))


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class ArrivalAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def run_arrival(self, town, seed, kite, height, player, camera, moves, warp=False):
        """effTransfer (effWarpTransfer when `warp`) on character 7 at
        `kite`, then frames until every slot is free; `moves` {frame: new
        position} walks him while the rings follow."""
        em = EffectMachine(town=town, seed=seed)
        em.set_player(player)
        em.set_camera(*camera)
        em.add_char(7, kite, height, 45.0)
        lines = ["reset %d %x" % (int(town), seed), "player " + hexs(*map(fb, player)),
                 "camera " + hexs(*map(fb, camera[0] + camera[1] + camera[2])),
                 "char 7 " + hexs(*map(fb, kite + (height, 45.0))), "warp 7" if warp else "transfer 7"]
        em.call("effWarpTransfer__FP6ccChar" if warp else "effTransfer__FP6ccChar", em.chars[7])
        slot0, events0 = em.live(), em.events
        frames = []
        pos = kite
        for f in range(80):
            if f in moves:
                pos = moves[f]
                em.move_char(7, pos)
                lines.append("char 7 " + hexs(*map(fb, pos + (height, 45.0))))
            lines.append("frame")
            frames.append(em.frame())
        got = [g for g in ask(lines) if g]
        self.assertEqual(got[0]["slot"], 0)
        self.assertEqual(len(slot0), 1)
        self.assertEqual(events0, [["se3d", 76, [fb(v) for v in kite] + [ONE]]])
        self.assertEqual(got[0]["events"], events0)
        rings = 0
        for f, (want, port) in enumerate(zip(frames, got[1:])):
            compare(self, want, port, "frame %d" % f)
            rings += sum(1 for d in want["draws"] if d[0] == "clump" and d[1] == "CMP_x032")
        self.assertEqual(frames[-1]["slots"], [])
        return frames, rings

    def test_kite_arrives_in_mac_anu(self):
        """Kite at Mac Anu's start (0, 5600, 600), height 160, the camera
        behind him: three rings, the generator at count 10 and the sound."""
        cam = ((0.0, 6100.0, 780.0), (0.0, 6100.0, 780.0), (0.0, 5600.0, 700.0))
        frames, rings = self.run_arrival(True, 0x1234, (0.0, 5600.0, 600.0), 160.0, (0.0, 5600.0, 600.0), cam, {})
        self.assertEqual([len(f["gens"]) for f in frames[:12]], [0] * 10 + [1, 0])
        self.assertEqual(frames[10]["gens"][0][0], 82)
        self.assertGreater(rings, 90)

    def test_moving_far_and_turned_away(self):
        """A character walking while the rings follow; the camera far enough
        for the distance fade, and turned away for a while (dispSW off)."""
        cam = ((1800.0, -2500.0, 900.0), (1800.0, -2500.0, 900.0), (1500.0, -1000.0, 700.0))
        moves = {5: (1520.0, -1010.0, 400.0), 12: (1560.0, -990.0, 410.0), 20: (1590.0, -900.0, 395.0)}
        for seed in (1, 77, 0xDEADBEEF):
            frames, _ = self.run_arrival(True, seed, (1500.0, -1000.0, 400.0), 120.0, (1400.0, -1100.0, 400.0),
                                         cam, moves)
            faded = [d[3] for f in frames for d in f["draws"] if d[0] == "clump"]
            self.assertTrue(any(a not in (0, ONE) for a in faded))
        away = ((0.0, 5000.0, 780.0), (0.0, 5000.0, 780.0), (0.0, 4000.0, 700.0))
        frames, rings = self.run_arrival(True, 5, (0.0, 5600.0, 600.0), 160.0, (0.0, 5600.0, 600.0), away, {})
        self.assertEqual(rings, 0)

    def test_warp(self):
        """effWarpTransfer: effect -25, the generator at once, no rings."""
        cam = ((0.0, 6100.0, 780.0), (0.0, 6100.0, 780.0), (0.0, 5600.0, 700.0))
        for town in (True, False):
            frames, rings = self.run_arrival(town, 3, (0.0, 5600.0, 600.0), 160.0, (0.0, 5600.0, 600.0), cam, {},
                                             warp=True)
            self.assertEqual(rings, 0)
            self.assertEqual([len(f["gens"]) for f in frames[:3]], [1, 0, 0])
            self.assertEqual(frames[0]["slots"][0]["id"], -25)

    def test_in_a_field(self):
        """In a field ccEffectCtrl resolves effectTbl (not effectTbl2); the
        arrival is the same."""
        cam = ((0.0, 500.0, 300.0), (0.0, 500.0, 300.0), (0.0, 0.0, 100.0))
        self.run_arrival(False, 99, (0.0, 0.0, 0.0), 160.0, (0.0, 0.0, 0.0), cam, {})


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class CoreAgainstGame(unittest.TestCase):
    """ccEffect::Main's own work - pointers, links, layers, pausing, the
    camera cone and the distance fade, the draw of each kind of object, the
    sprite's pattern steps, the ageing - through ids with no code of their
    own: 1 and 8 (clumps), 12-14 and 22-24 (drain's animations), 170
    (ANM_x071), and id 1 patched to each sprite kind (2-5) over PARTICLE's
    EFF_ objects."""

    POOL = (1, 8, 12, 13, 14, 22, 23, 24, 170)

    @classmethod
    def setUpClass(cls):
        build()

    def run_case(self, seed, patch=None, frames=70):
        import random
        rng = random.Random(seed)
        em = EffectMachine(town=False, seed=seed)
        lines = ["reset 0 %x" % seed]

        def both(line, fn, *args):
            lines.append(line)
            return fn(*args)

        if patch is not None:
            obj, kind = patch
            em.patch(1, "particle", obj, kind)
            lines.append("patch 1 particle %s %x" % (obj, kind))
        player = [rng.uniform(-3000, 3000), rng.uniform(-3000, 3000), rng.uniform(0, 500)]
        em.set_player(player)
        lines.append("player " + hexs(*map(fb, player)))
        chars = {}
        for c in range(1, 4):
            chars[c] = [player[0] + rng.uniform(-800, 800), player[1] + rng.uniform(-800, 800), player[2]]
            dirc = [rng.uniform(-3, 3) for _ in range(3)]
            em.add_char(c, chars[c], 160.0, 45.0, dirc)
            lines.append("char %d " % c + hexs(*map(fb, chars[c] + [160.0, 45.0] + dirc)))

        def camera():
            eye = [player[0] + rng.uniform(-6000, 6000), player[1] + rng.uniform(-6000, 6000), rng.uniform(100, 900)]
            if rng.random() < 0.6:
                eye = [player[0] + rng.uniform(-2500, 2500), player[1] + rng.uniform(-2500, 2500), 400.0]
            view = [player[0] + rng.uniform(-300, 300), player[1] + rng.uniform(-300, 300), 200.0]
            if rng.random() < 0.15:
                view = [2 * eye[0] - player[0], 2 * eye[1] - player[1], 200.0]
            em.set_camera(eye, eye, view)
            lines.append("camera " + hexs(*map(fb, eye + eye + view)))

        camera()
        pool = self.POOL if patch is None else (1,)
        slots = []

        def spawn():
            eid = rng.choice(pool)
            s = em.spawn(eid)
            lines.append("spawn %x" % eid)
            if s < 0:
                return
            slots.append(s)

            def put(key, *v):
                em.set_field(s, key, *v)
                lines.append("set %x %s %s" % (s, key, hexs(*v)))
            put("pos", *[fb(player[0] + rng.uniform(-4000, 4000)), fb(player[1] + rng.uniform(-4000, 4000)),
                         fb(rng.uniform(0, 600)), ONE])
            put("rot", *[fb(rng.uniform(-3.2, 3.2)) for _ in range(3)], 0)
            put("scale", *[fb(rng.uniform(0.2, 3)) for _ in range(3)], rng.choice([0, ONE]))
            put("transparency", fb(rng.random()))
            put("life", rng.choice([0xFFFF, rng.randrange(0, 60)]))
            put("bits", rng.randrange(0, 512) & ~8 | 2)
            if patch is not None:
                put("pat", rng.randrange(0, 60))
            r = rng.random()
            if r < 0.2:
                put("posptr", 0, rng.randrange(1, 4))
            elif r < 0.3 and len(slots) > 1:
                put("posptr", 2, rng.choice(slots[:-1]))
            elif r < 0.4 and len(slots) > 1:
                put("link", rng.choice(slots[:-1]))
            if rng.random() < 0.2:
                if rng.random() < 0.5:
                    put("rotptr", 1, rng.randrange(1, 4))
                else:
                    put("rotptr", 3, rng.choice(slots))
            if rng.random() < 0.3:
                put("target", rng.randrange(1, 4))
            if rng.random() < 0.2:
                put("layer", rng.choice([10, 30, 60, (-20) & 0xFFFFFFFF]))

        for _ in range(rng.randrange(6, 16)):
            spawn()
        want = []
        for f in range(frames):
            if rng.random() < 0.3:
                camera()
            for c in chars:
                if rng.random() < 0.5:
                    chars[c] = [v + rng.uniform(-40, 40) for v in chars[c][:2]] + [chars[c][2]]
                    em.move_char(c, chars[c])
                    lines.append("char %d " % c + hexs(*map(fb, chars[c] + [160.0, 45.0])))
                    em.vec(em.chars[c] + 0x60, [0, 0, 0, 0])
            if rng.random() < 0.15:
                spawn()
            live = [s["i"] for s in em.live()]
            if live and rng.random() < 0.2:
                s = rng.choice(live)
                b = em.m.load(EFFWORK + 0xC0 * s + 0x60, 2) ^ 4
                em.set_field(s, "bits", b & 0x1FF)
                lines.append("set %x bits %x" % (s, b & 0x1FF))
            lines.append("frame")
            want.append(em.frame())
        got = [g for g in ask(lines) if "slots" in g]
        seen = {"draws": 0, "paused": 0, "faded": 0}
        for f, (w, g) in enumerate(zip(want, got)):
            compare(self, w, g, "seed %x frame %d" % (seed, f))
            seen["draws"] += len(w["draws"])
            seen["paused"] += sum(1 for s in w["slots"] if s["bits"] & 4)
            # Not paused, yet not drawn: outside the camera's cone or past
            # the distance fade.
            seen["faded"] += sum(1 for s in w["slots"] if not s["bits"] & 6)
        return seen

    def test_objects_through_the_draw(self):
        total = {"draws": 0, "paused": 0, "faded": 0}
        for seed in range(1, 41):
            for k, v in self.run_case(seed).items():
                total[k] += v
        print("objects: 40 cases, 2800 frames, %(draws)d draws, %(paused)d paused slot-frames, %(faded)d hidden slot-frames"
              % total, file=sys.stderr)
        self.assertGreater(total["draws"], 3000)
        self.assertGreater(total["paused"], 100)
        self.assertGreater(total["faded"], 100)

    def test_sprites_through_the_draw(self):
        import random
        rng = random.Random(9)
        em = EffectMachine(town=False)
        names = sorted(n for n in em.effs if n in {x for c in [em.ccs["particle"]] for x in eff_chunks(c)})
        draws = 0
        for seed in range(100, 124):
            obj = rng.choice(names)
            kind = 2 + seed % 4
            draws += self.run_case(seed, (obj, kind), frames=50)["draws"]
        print("sprites: 24 cases, 1200 frames, %d draws" % draws, file=sys.stderr)
        self.assertGreater(draws, 1000)


if __name__ == "__main__":
    unittest.main()
