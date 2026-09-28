#!/usr/bin/env python3
"""crates/piney-world's field weather and ambient pictures
(piney_world::field_ambient) against the game's own WORLD code run in
tools/eemu.py (the Rust eemu_rs VuMachine when built).

The game's side is a field made by its own WORLD::Init and WORLD::Generate
(gcmn 0x005a4cd0, 0x005a6da0) in the interpreter, then drawn frame after
frame by WORLD::Draw (0x005a97b0), which runs DrawEffect, DrawRain,
DrawSnow, DrawSteam, the SNOW, FIREFLY, FIREFLY2, TOBJ and BIRD classes and
the lens flare as they are. What only loads or draws is answered here: the
streams and chunks are named stand-ins, and the pictures are recorded as
they are asked for - ccEff::Draw (the sprite's name, place, pattern, scale,
turn, transparency and fog bit), ccClump::Draw and ccAnm::Draw (with their
matrices and TOBJ's shadow settings), ccSprite::MakePacket of the 2D
smoke's ccMask, effSmoke's arguments, the sounds, scFadeDef's flash and
the layer each is drawn on.
The ground, the objects, the minimap and the background are the other
checks' (tools/test_field.py, tools/test_field_rt.py); they are not drawn
here.

The ambient_probe example builds the port's field with the same arguments
and answers the same frames. Compared: fieldrand's count after Init and
after Generate, what Init and Generate made (every SNOW, the smoke, the
rain drops, the fireflies, TOBJ and BIRD), then each frame's pictures in
order and the counts of fieldrand and ccRand.

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
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "ambient_probe")
READY = os.path.exists(ELF) and shutil.which("cargo") is not None

# Scratch memory: WORLD_MAN, ccGame, the save, ccSys, cc3d, the player and
# the heap everything else comes from.
WM, GAME, SAVE, SYS, CC3D, PLAYER, WP = (0x01200000, 0x01201000, 0x01202000, 0x01210000, 0x01220000,
                                         0x01230000, 0x01231000)
FADE, SCRATCH, CAM, DRAWENV, EVMNG = 0x01232000, 0x01240000, 0x01238000, 0x01239000, 0x0123A000
HEAP, HEAP_END = 0x01400000, 0x01F00000
WORLD_SIZE = 0x61E0


def fb(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def bf(u):
    return struct.unpack("<f", struct.pack("<I", u & 0xFFFFFFFF))[0]


class Game:
    """The game's WORLD in the interpreter."""

    def __init__(self):
        from image import Program
        from test_anim import machine_class
        self.prog = Program(ELF, "gcmn")
        self.m = machine_class()(self.prog)
        self.sym = lambda n: self.prog.symbol_named(n).value      # noqa: E731
        self.names = {}         # fake chunk/stream address -> name
        self.by_name = {}
        self.ops = []
        self.layer = 0
        self.hooks()

    # --- memory ------------------------------------------------------------
    def new(self, m, n, *_):
        a = self.heap
        self.heap = (a + max(n, 16) + 15) & ~15
        if self.heap > HEAP_END:
            raise RuntimeError("scratch heap exhausted")
        m.mem[a:a + n] = bytes(n)
        return a

    def fake(self, name, size=0x400):
        """A stand-in stream or chunk for `name`, the same block each time."""
        if name not in self.by_name:
            a = self.new(self.m, size)
            self.by_name[name] = a
            self.names[a] = name
        return self.by_name[name]

    def cstr(self, a):
        mem = self.m.mem
        a &= 0xFFFFFFFF
        return bytes(mem[a:mem.index(0, a)]).decode("latin-1")

    def vec(self, a, n=4):
        return [self.m.load(a + 4 * i, 4) for i in range(n)]

    def put(self, a, v):
        for i, x in enumerate(v):
            self.m.store(a + 4 * i, 4, x)

    # --- the stand-ins -----------------------------------------------------
    def hooks(self):
        m, sym = self.m, self.sym
        nop = lambda mm, *a: 0                     # noqa: E731
        for name in ("__nw__FUi", "__nwa__FUi", "ccMalloc__FUi"):
            m.hooks[sym(name)] = self.new
        for name in ("__dl__FPv", "__dla__FPv", "ccFree__FPv"):
            m.hooks[sym(name)] = nop
        m.hooks[sym("GetCCSAdrs__8ccStreamFPCc")] = lambda mm, a0, *a: self.fake("ccs:" + self.cstr(a0))
        m.hooks[sym("GetChunkAdrsF__8ccStreamFPCci")] = self.chunk
        m.hooks[sym("GetSubstAdrsF__5ccAnmFPCcb")] = lambda mm, s, a1, *a: self.fake("subst:" + self.cstr(a1))
        m.hooks[sym("GetSubstAdrsF__8ccStreamFPCci")] = lambda mm, s, a1, *a: self.fake("subst:" + self.cstr(a1))
        m.hooks[sym("Init__5ccEffFP10ccEffChunki")] = self.eff_init
        m.hooks[sym("Draw__5ccEffFUs")] = self.eff_draw_own
        m.hooks[sym("Draw__5ccEffFPfUs")] = self.eff_draw_at
        m.hooks[sym("SetRenderState__5ccEffF20CC_RENDER_STATE_TYPEi")] = nop
        m.hooks[sym("SetActiveLayer__9WORLD_MANFi")] = self.set_layer
        m.hooks[sym("effSmoke__FPfPffiiUsUs")] = self.eff_smoke
        m.hooks[sym("ccSeOn__Fi")] = lambda mm, a0, *a: self.ops.append(["se", a0]) or 0
        m.hooks[sym("ccSeOn3D__FiPf")] = lambda mm, a0, a1, *a: self.ops.append(["se3d", a0, self.vec(a1)]) or 0
        m.hooks[sym("tobjSeLoopStart__FPf")] = self.tobj_se_start
        m.hooks[sym("tobjSeLoop__FPff")] = lambda mm, a0, *a: self.ops.append(
            ["seloop", self.vec(a0, 3), mm.f[12]]) or 0
        m.hooks[sym("EntryFlash__8ccScFadeFiiffff")] = self.flash
        m.hooks[sym("MakePacket__8ccSpriteFii")] = self.sprite
        m.hooks[sym("Draw__7ccClumpFv")] = lambda mm, a0, *a: self.ops.append(
            ["model", self.layer, "bird", self.vec(a0 + 0x40, 16), 0x3F800000, None]) or 0
        m.hooks[sym("Init__7ccClumpFP12ccClumpChunk")] = lambda mm, a0, a1, *a: self.m.store(a0 + 0x9c, 4, a1) or 0
        for name in ("__ct__16ccDrawPacketCtrlFv", "Init__7ccLayerFsP6ccView", "SetFrame__6ccViewFffffffff",
                     "SetTex__8ccSpriteFPcPci", "ccInitKanji__Fii", "SetCCS__5FIELDFP8ccStream",
                     "SetFogSw__7ccClumpFi", "SetFog__9ccDrawEnvFffffUi", "SetLightEnv__5ccAnmF11ccAnmOption",
                     "SetAnm__5ccAnmFP10ccAnmChunkUi", "_AnimateForward__5ccAnmFUi", "GetAmbient__5ccAnmFPf",
                     "AddGrp__10ccLightGrpFP7ccLight", "SetAmbient__9ccDrawEnvFPf",
                     "SetLightDirection__9WORLD_MANFP14ccDistantLight",
                     "CalcObjectVertexColor__FPfP8ccStreamP5ccObjP14ccDistantLight", "__dt__7ccClumpFv",
                     "SetClump__10FIELD_MESHFP8ccStreamPc", "SetFogSw__5ccAnmFi", "Duplicate__5ccObjFUi",
                     "Init__10ccTexChunkFP12ccChunkIndexP12ccChunkIndexP14ccTexChunkDesc",
                     "ChangeTex__7ccModelFP10ccTexChunkP10ccTexChunk", "Init__7ccModelFP12ccModelChunkPP7ccCoordUi",
                     "Duplicate__7ccModelFUi", "SetSmallMESH__5FIELDFiiP7ccModel",
                     "CalcQuadVertexNormal__5FIELDFv", "CalcVertexColor__5FIELDFPfP14ccDistantLight",
                     "GetAmbient__9ccDrawEnvFPf", "HitEnable__8FOBJECT2Fv", "SetPosition__6FCOVERFiiiP5FIELDP8ccStreamPcf",
                     "DrawMiniMap__5WORLDFv", "MakePacket__16ccBufferSamplingFP7ccLayer", "LockBlt__13ccBltGrpChunkFv",
                     "UnlockBlt__13ccBltGrpChunkFv", "MakePacketLoadData__13ccBltGrpChunkFP7ccLayer",
                     "DrawObject__5WORLDFv", "DrawMesh__5WORLDFv", "DrawBG__5WORLDFv", "Draw__8FOBJECT2Fv",
                     "MakePacketDrawBuffTrans__7ccLayerFP10ccTexChunk", "SetUV__7ccModelFiiP15ccMaterialChunki",
                     "waterUVModifi__FP5ccObj", "SendPacketS__6ccMaskFv"):
            m.hooks[sym(name)] = nop
        m.hooks[sym("Draw__5ccAnmFv")] = self.anm_draw
        # the camera, as each case sets it
        m.hooks[sym("cameraGetPos__FPfi")] = lambda mm, a0, a1, *a: self.put(a0, self.cam_pos[a1 & 0xFFFF]) or 0
        m.hooks[sym("cameraGetRot__FPfi")] = lambda mm, a0, a1, *a: self.put(a0, self.cam_rot) or 0
        m.hooks[sym("cameraGetRot2__FPfi")] = lambda mm, a0, a1, *a: self.put(a0, self.cam_rot2) or 0
        m.hooks[sym("checkCameraType__Fv")] = lambda mm, *a: self.cam_type
        m.hooks[sym("checkCameraID__Fv")] = lambda mm, *a: 1
        m.hooks[sym("cameraGetView__FPfi")] = lambda mm, a0, *a: self.put(a0, self.view) or 0
        m.hooks[sym("ccRand__Fv")] = self.cc_rand
        m.hooks[sym("Reset__19ccSubstSearchResultFUs")] = nop
        m.hooks[sym("GetSubstAdrs__5ccAnmFPCcP19ccSubstSearchResult")] = self.subst

    def tobj_se_start(self, m, *_):
        self.tobj_se += 1
        return 0

    def chunk(self, m, stream, name, *_):
        n = self.cstr(name)
        a = self.fake(n)
        if n.startswith("DMY_"):
            self.put(a + 0x10, self.sun)
        return a

    def subst(self, m, anm, name, res, *_):
        """GetSubstAdrs(anm, name, &result): one object, its local matrix
        at the runner's place and no parent."""
        n = self.cstr(name)
        k = 0 if n.endswith("0") else 1
        obj = self.fake("obj:" + n, 0x100)
        arr = self.fake("arr:" + n, 0x10)
        m.mem[obj:obj + 0x100] = bytes(0x100)
        self.put(obj + 0x70, self.runners[k])
        m.store(arr, 4, obj)
        m.store(res, 4, arr)
        return 0

    def cc_rand(self, m, *_):
        self.cc_count += 1
        return self.mt.next() & 0xFFFFFFFF

    def anm_draw(self, m, a, *_):
        w = self.w
        t = m.load(w + 0x60CC, 4)
        known = {m.load(w + 0x6160, 4): "haze"}
        if t:
            known.update({m.load(t + 0x40, 4): "tobj", m.load(t + 0x44, 4): "runner0", m.load(t + 0x48, 4): "runner1"})
        what = known.get(a)
        if what is None:
            return 0
        shadow = None
        if what != "haze":
            # ccDrawEnv +0xa0 and +0xa4: the shadow's length and transparency
            shadow = [m.load(DRAWENV + 0xA0, 4), m.load(DRAWENV + 0xA4, 1)]
        self.ops.append(["model", self.layer, what, self.vec(a + 0x40, 16), m.load(a + 0x88, 4), shadow])
        return 0

    def set_layer(self, m, wm, k, *_):
        self.layer = k
        return 0

    def eff_init(self, m, e, chunk, flag, *_):
        # ccEff::Init (main 0x0013ba20): what the drawing reads back.
        m.mem[e:e + 0x70] = bytes(0x70)
        m.store(e + 0x44, 4, chunk)
        for off in (0x1C, 0x20, 0x24, 0x34):
            m.store(e + off, 4, 0x3F800000)
        m.store(e + 0x2C, 4, 0x808080)
        m.store(e + 0x62, 2, ((flag & 0xFFFF) << 5) | 0x54)
        return 0

    def eff_rec(self, e, pos, pat):
        m = self.m
        return ["eff", self.layer, self.names.get(m.load(e + 0x44, 4), "?"), pos, pat & 0xFFFF,
                m.load(e + 0x20, 4), m.load(e + 0x24, 4), m.load(e + 0x28, 4), m.load(e + 0x34, 4),
                (m.load(e + 0x62, 2) >> 5) & 1]

    def eff_draw_own(self, m, e, pat, *_):
        self.ops.append(self.eff_rec(e, self.vec(e + 0x10), pat))
        return 0

    def eff_draw_at(self, m, e, pos, pat, *_):
        self.ops.append(self.eff_rec(e, self.vec(pos), pat))
        return 0

    def eff_smoke(self, m, pos, v, life, t, *_):
        # effSmoke(pos, v, s, life, t, in, out): s in f12, in and out in t0, t1
        self.ops.append(["smoke", self.vec(pos), self.vec(v), m.f[12], life, t, m.r[8] & 0xFFFF, m.r[9] & 0xFFFF])
        return 0

    def flash(self, m, fade, n, colour, *_):
        self.ops.append(["flash", n, colour & 0xFFFFFFFF, [m.f[12], m.f[13], m.f[14], m.f[15]]])
        return 0

    def sprite(self, m, s, a1, a2, *_):
        self.ops.append(["mask", self.layer] + [m.load(s + off, 4) for off in (0x30, 0x34, 0x38, 0x40, 0x44, 0x48,
                                                                                0x4C, 0x50, 0x54)])
        return 0

    # --- a field -----------------------------------------------------------
    def setup(self, c):
        m, sym = self.m, self.sym
        m.mem[:] = m.pristine if hasattr(m, "pristine") else m.mem
        self.heap = HEAP
        self.names, self.by_name, self.ops = {}, {}, []
        for a, n in ((WM, 0x500), (GAME, 0x100), (SAVE, 0x8000), (SYS, 0x400), (CC3D, 0x100), (PLAYER, 0x400),
                     (WP, 0x30), (FADE, 0x100)):
            m.mem[a:a + n] = bytes(n)
        world = self.new(m, WORLD_SIZE)
        self.w = world
        m.store(WM + 0x08, 4, 1)
        m.store(WM + 0x0C, 4, c["b"])
        m.store(WM + 0x10, 4, c["type"])
        m.store(WM + 0xF0, 4, c["hack"])
        m.store(WM + 0x120, 4, c["event"])
        m.store(WM + 0x158, 4, WP)
        m.store(WP + 0xC, 4, c["type"])
        m.store(WP + 0x14, 4, c["b"])
        m.store(WP + 0x18, 4, c["ground"])
        m.store(WP + 0x1C, 4, c["object"])
        self.put(WM + 0x80, c["centre"] + [0, 0x3F800000])
        for off, v in ((0x420, 0.0), (0x424, 0.0), (0x428, 48000.0), (0x42C, 48000.0)):
            m.store(WM + off, 4, fb(v))
        m.store(WM + 0x434, 4, world)
        m.store(GAME + 0x1C, 4, c["server"])
        m.store(GAME + 0x24, 4, c["event"])
        m.store(GAME + 0x58, 4, 1)
        m.store(world + 0x10, 4, c["type"])
        m.store(world + 0x14, 4, c["b"])
        for name, va in (("worldman", WM), ("game", GAME), ("saveData", SAVE), ("ccSys", SYS), ("cc3d", CC3D),
                         ("scFadeDef", FADE)):
            m.store(sym(name), 4, va)
        m.store(inf_va(0x00730300), 4, PLAYER)
        m.store(sym("seed"), 4, c["seed"])
        m.store(sym("randcnt"), 4, 0)
        self.mt = MT()
        self.cc_count = 0
        self.sun = [0, 0, 0, 0x3F800000]
        self.runners = [[0, 0, 0, 0x3F800000], [0, 0, 0, 0x3F800000]]
        self.view = [0, 0, 0, 0x3F800000]
        for a, n in ((SCRATCH, 0x1000), (CAM, 0x100), (DRAWENV, 0x100), (EVMNG, 0x800)):
            m.mem[a:a + n] = bytes(n)
        m.store(SYS + 604, 4, SCRATCH)
        m.store(sym("activeCamPtr"), 4, CAM)
        m.store(sym("active__9ccDrawEnv"), 4, DRAWENV)
        m.store(sym("eventMng"), 4, EVMNG)
        m.store(sym("camID"), 2, 0)
        self.cam_pos = {0: [0, 0, 0, 0x3F800000], 1: [0, 0, 0, 0x3F800000]}
        self.cam_rot = [0, 0, 0, 0]
        self.cam_rot2 = [0, 0, 0, 0]
        self.cam_type = 0

    def make(self, c):
        """WORLD::Init then WORLD::Generate (SetHackFlag first, as GO)."""
        m, sym = self.m, self.sym
        self.setup(c)
        self.tobj_se = 0
        m.call(sym("SetHackFlag__5WORLDFi"), (self.w, c["hack"]))
        m.call(sym("Init__5WORLDFv"), (self.w,), limit=200_000_000)
        init = m.load(sym("randcnt"), 4)
        m.call(sym("Generate__5WORLDFv"), (self.w,), limit=800_000_000)
        return init, m.load(sym("randcnt"), 4)


    # --- reading the game's state --------------------------------------
    def state(self, init, gen, c):
        m, w = self.m, self.w
        ld = lambda a: m.load(a, 4)                 # noqa: E731
        s32 = lambda a: m.load(a, 4, signed=True)   # noqa: E731
        snow = []
        for i in range(200):
            o = ld(w + 0x38 + 4 * i)
            if o:
                snow.append([ld(o + 4), self.names.get(ld(ld(o + 0x14) + 0x44), "?"), self.vec(o + 0x30, 3),
                             self.vec(o + 0x40, 3), s32(o), s32(o + 8)])
        smoke = None
        if c["type"] in (2, 3, 5, 6):
            smoke = {"sleep": s32(w + 0x61D0),
                     "puffs": [[s32(w + 0x6178 + 16 * i + 4 * k) for k in range(4)] for i in range(5)]}
        drops = []
        for i in range(50):
            e = ld(w + 0x5400 + 64 * i)
            if e:
                drops.append([self.names.get(ld(e + 0x44), "?"), s32(w + 0x5400 + 64 * i + 0x30)])
        t = ld(w + 0x60CC)
        tobj = None
        if t:
            tobj = [ld(t), s32(t + 4), self.vec(t + 0x10, 2), ld(t + 0x30), self.vec(t + 0x20, 3), self.tobj_se]
        flies = []
        for i in range(5):
            f = ld(w + 0x60D0 + 4 * i)
            if not f:
                continue
            keys = []
            for sp in (0x1C8, 0x1CC, 0x1D0):
                ns = ld(f + sp)
                data, n = ld(ns), s32(ns + 4)
                keys.append([ld(data + 28 * k + off) for k in range(n) for off in (0, 4, 8, 12, 16, 20)])
            flies.append([ld(f + 0x50), self.vec(f + 0x90, 3), keys])
        roamers = []
        for i in range(15):
            f = ld(w + 0x60E4 + 4 * i)
            if f:
                roamers.append([self.vec(f + 0xB0), self.vec(f + 0x70, 3), s32(f + 0xA4), s32(f + 0xA0), ld(f + 0x90)])
        b = ld(w + 0x6120)
        bird = self.vec(b + 0x20, 3) if b else None
        return {"init": init, "gen": gen, "snow": snow, "smoke": smoke, "drops": drops,
                "thunder": int(ld(w + 0x6080) != 0), "tobj": tobj, "fireflies": flies, "roamers": roamers,
                "bird": bird}


    def frame(self, f):
        m, sym = self.m, self.sym
        self.ops = []
        self.put(PLAYER + 0x40, f["player"])
        self.cam_pos = {0: f["eye"], 1: f["eye1"]}
        self.cam_rot, self.cam_rot2 = f["rot"], f["rot2"]
        self.cam_type = 1 if f["eye_view"] else 0
        self.view = f["view"]
        self.put(CAM, f["eye"])
        self.put(CAM + 0x10, f["view"])
        m.store(SYS + 0x358, 4, f["odd"])
        self.put(self.w + 0x6140, f["ofs"])
        self.put(WM + 0x80, f["centre"] + [0, 0x3F800000])
        self.sun = f["sun"]
        self.runners = f["runners"]
        for name, a in self.by_name.items():
            if name.startswith("DMY_"):
                self.put(a + 0x10, self.sun)
        m.call(sym("Draw__5WORLDFv"), (self.w,), limit=200_000_000)
        return {"ops": self.ops, "rand": m.load(sym("randcnt"), 4), "cc": self.cc_count}

    def set_fires(self, fires):
        self.m.store(self.w + 0x358, 4, len(fires))
        for i, p in enumerate(fires):
            self.put(self.w + 0x360 + 16 * i, p + [0])


class MT:
    """genrand (the executable's MT19937, sgenrand(4352) on first use), as
    ccRand draws from it."""

    def __init__(self):
        self.mt = [0] * 624
        self.mti = 625

    def seed(self, s):
        for i in range(624):
            self.mt[i] = s & 0xFFFF0000
            s = (s * 69069 + 1) & 0xFFFFFFFF
            self.mt[i] |= (s & 0xFFFF0000) >> 16
            s = (s * 69069 + 1) & 0xFFFFFFFF
        self.mti = 624

    def next(self):
        n, m_ = 624, 397
        mt = self.mt
        if self.mti >= n:
            if self.mti == 625:
                self.seed(4352)

            def mix(a, b, c):
                y = (a & 0x80000000) | (b & 0x7FFFFFFF)
                return c ^ (y >> 1) ^ (0x9908B0DF if y & 1 else 0)
            for k in range(n - m_):
                mt[k] = mix(mt[k], mt[k + 1], mt[k + m_])
            for k in range(n - m_, n - 1):
                mt[k] = mix(mt[k], mt[k + 1], mt[k + m_ - n])
            mt[n - 1] = mix(mt[n - 1], mt[0], mt[m_ - 1])
            self.mti = 0
        y = mt[self.mti]
        self.mti += 1
        y ^= y >> 11
        y ^= (y << 7) & 0x9D2C5680
        y ^= (y << 15) & 0xEFC60000
        y ^= y >> 18
        return y & 0xFFFFFFFF


class Probe:
    def __init__(self):
        env = dict(os.environ)
        subprocess.run(["cargo", "build", "--release", "-q", "-p", "piney-world", "--example", "ambient_probe"],
                       cwd=ROOT, check=True, env=env)
        self.p = subprocess.Popen([EXAMPLE], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

    def ask(self, line):
        self.p.stdin.write(line + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        self.p.wait()
        self.p.stdout.close()


def hx(*v):
    return " ".join("%x" % (x & 0xFFFFFFFF) for x in v)


def first_diff(a, b, path=""):
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            d = first_diff(a.get(k), b.get(k), f"{path}.{k}")
            if d:
                return d
        return None
    if isinstance(a, list) and isinstance(b, list):
        for i, (x, y) in enumerate(zip(a, b)):
            d = first_diff(x, y, f"{path}[{i}]")
            if d:
                return d
        if len(a) != len(b):
            return f"{path}: length {len(a)} != {len(b)}"
        return None
    return None if a == b else f"{path}: {a!r} != {b!r}"


def case_line(c):
    return "field " + hx(c["seed"], c["type"], c["b"], c["ground"], c["object"], c["event"], c["protect"],
                         c["hack"], c["server"], *c["centre"])


def frame_line(f):
    return "frame " + hx(*f["player"], *f["eye"], *f["eye1"], *f["rot"], *f["rot2"], *f["view"],
                         int(f["eye_view"]), f["odd"] & 1, *f["ofs"], *f["centre"], *f["sun"],
                         *f["runners"][0], *f["runners"][1])


def frames(rnd, n, start):
    """A walk of n frames from `start`: the player's place, the camera
    behind and above, its turns and view."""
    import math
    x, y = bf(start[0]), bf(start[1])
    head = rnd.uniform(-math.pi, math.pi)
    out = []
    sun = [fb(rnd.uniform(-20000, 20000)), fb(rnd.uniform(-20000, 20000)), fb(rnd.uniform(0, 8000)), fb(1.0)]
    runners = [[fb(rnd.uniform(0, 48000)), fb(rnd.uniform(0, 48000)), fb(rnd.uniform(0, 3000)), fb(1.0)]
               for _ in range(2)]
    for i in range(n):
        if rnd.random() < 0.05:
            head += rnd.uniform(-1.0, 1.0)
        speed = rnd.choice((0.0, 8.0, 20.0, 40.0))
        x = (x + speed * math.sin(head)) % 48000.0
        y = (y - speed * math.cos(head)) % 48000.0
        z = rnd.uniform(0, 400)
        # The camera's turn is a 16-bit angle: -pi to pi.
        cz = math.remainder(head + rnd.uniform(-0.3, 0.3), 2 * math.pi)
        eye = [x - 600 * math.sin(cz), y + 600 * math.cos(cz), z + 300, 1.0]
        view = [x, y, z + 100, 1.0]
        rot = [rnd.uniform(-0.3, 0.3), 0.0, cz, 0.0]
        rot2 = [rnd.uniform(-0.3, 0.3), 0.0, math.remainder(cz + rnd.uniform(-0.5, 0.5), 2 * math.pi), 0.0]
        out.append({"player": [fb(x), fb(y), fb(z), fb(1.0)], "eye": [fb(v) for v in eye],
                    "eye1": [fb(v + rnd.uniform(-5, 5)) for v in eye[:3]] + [fb(1.0)],
                    "rot": [fb(v) for v in rot], "rot2": [fb(v) for v in rot2], "view": [fb(v) for v in view],
                    "eye_view": rnd.random() < 0.1, "odd": i, "ofs": [fb(x), fb(y)], "centre": [fb(x), fb(y)],
                    "sun": sun, "runners": runners})
    return out


SEEN = {}


def note(ops):
    for o in ops:
        k = o[0] + ":" + (o[2] if o[0] in ("eff", "model") else str(o[1]) if o[0] in ("se", "se3d") else "")
        SEEN[k] = SEEN.get(k, 0) + 1


def run_case(g, pr, c, n, rnd, fires=()):
    init, gen = g.make(c)
    game = g.state(init, gen, c)
    port = pr.ask(case_line(c))
    d = first_diff(game, port)
    if d:
        return f"state {d}"
    if fires:
        g.set_fires(list(fires))
        pr.ask("fires " + hx(len(fires), *[v for p in fires for v in p]))
    fr = frames(rnd, n, [fb(24000.0), fb(24000.0)])
    for i, f in enumerate(fr):
        a = g.frame(f)
        b = pr.ask(frame_line(f))
        note(a["ops"])
        d = first_diff(a, b)
        if d:
            return f"frame {i}: {d}"
    return None


ROWS = {0: 4, 1: 4, 2: 4, 3: 4, 5: 8, 6: 6, 7: 8, 8: 8, 9: 8, 10: 8}


def random_case(rnd, ty, b, hack):
    return {"seed": rnd.randrange(1, 0x7FFFFFFF), "type": ty, "b": b, "ground": rnd.randrange(3),
            "object": rnd.randrange(3), "event": 0, "protect": 0, "hack": hack, "server": rnd.randrange(5),
            "centre": [fb(rnd.uniform(0, 48000)), fb(rnd.uniform(0, 48000))]}


def fire_places(rnd):
    return [[fb(rnd.uniform(0, 48000)), fb(rnd.uniform(0, 48000)), fb(rnd.uniform(0, 500))]
            for _ in range(rnd.randrange(1, 6))]


def sweep(n_frames, seed, only=None):
    g = Game()
    pr = Probe()
    rnd = random.Random(seed)
    bad = 0
    for ty, rows in ROWS.items():
        if only is not None and ty not in only:
            continue
        for b in range(rows):
            for hack in (0, 3):
                c = random_case(rnd, ty, b, hack)
                fires = fire_places(rnd) if ty == 7 else ()
                d = run_case(g, pr, c, n_frames, rnd, fires)
                print(f"type {ty} b {b} hack {hack} server {c['server']}: {d or 'ok'}"[:400], flush=True)
                bad += d is not None
    pr.close()
    for k in sorted(SEEN):
        print(f"  {k}: {SEEN[k]}")
    return bad


@unittest.skipUnless(READY, "needs the extracted disc and cargo")
class AmbientAgainstGame(unittest.TestCase):
    def test_every_type_weather_and_hack(self):
        """Every field type and weather row, hacked and not, on a random
        server: Init, Generate and 1,000 frames of a walk; and every kind
        of picture among them."""
        self.assertEqual(sweep(1000, 1), 0)
        for k in ("flash:", "se:236", "se3d:43", "seloop:", "smoke:", "mask:", "model:bird", "model:haze", "model:tobj",
                  "model:runner0", "eff:EFF_sfp8thu1", "eff:EFF_sflenz_1", "eff:EFF_sfzdigi1", "eff:EFF_sfzfir1",
                  "eff:EFF_sfpfir_2", "eff:EFF_sfa8fir1", "eff:EFF_sfg8sno1", "eff:EFF_sfh8sno1"):
            self.assertIn(k, SEEN)


def main():
    """`test_field_ambient_rs.py FRAMES [TYPE ...]`: the sweep with that
    many frames a case (only those field types if named); with no
    arguments, the unittest."""
    if len(sys.argv) > 1 and sys.argv[1].isdigit():
        n = int(sys.argv[1])
        only = [int(x) for x in sys.argv[2:]] or None
        sys.exit(1 if sweep(n, 1, only) else 0)
    unittest.main()


if __name__ == "__main__":
    main()
