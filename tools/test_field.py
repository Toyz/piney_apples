#!/usr/bin/env python3
"""tools/field.py against the game's own WORLD::Generate and WORLD::Init, and
against the code that turns a field into what the game draws.

WORLD::Generate (INF gcmn.prg 0x005a6da0) runs in tools/eemu.py over a
scratch WORLD, FIELD, WORLD_MAN and ccGame, from a given RNG state, and is
stopped when it reaches its last call of WORLD_MAN::SetStartPos, with the
start position in a1. What only loads or draws is stubbed (models, clumps,
animations, vertex colours and normals, cover meshes); FCOVER::SetPosition
records the cover tiles; the VU0 helpers the field code calls
(sceVu0Sub/Add/Scale/DivVector, InnerProduct, OuterProduct, Normalize, and
for lighting ApplyMatrix, UnitMatrix and RotMatrixX/Y/Z) are done with
field.py's model of them, the FPU's rules. Then the height map, the
check/check3/mnt grids, every placed object with its z (WORLD::GetHeight as
it was placed), the covers, the start chip and the RNG state are compared.
WORLD::Init is run the same way to count its fieldrand calls. Infection with
the most cases; the other three volumes, when extracted with a .syms sidecar,
each against its own code (through the names piney-gen syms carries).

The render checks run single functions over a FIELD holding field.py's
heights: WORLD::GetHeight at points across the field, on borders and on
hidden chips; InitQuad with CalcQuadVertexNormal (every vertex normal) and
CalcVertexColor under a background's light (every colour);
FIELD_MESH::RelocateMesh / SetMESH2 for ground tiles (visibility,
translation, every vertex z and colour); FCOVER::SetPosition with
FIELD::SetSmallMESH for covers of each quadrant; CalcObjectVertexColor on
random vertices. Skipped when an executable is not present.
"""

import os
import random
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF

OTHERS = [os.path.join(ROOT, "work", v, "disc", e) for v, e in
          (("mutation", "SLUS_205.62"), ("outbreak", "SLUS_205.63"),
           ("quarantine", "SLUS_205.64"))]

W, FLD, WM, WP, GAME, SAVE, SYS, CC3D = (0x01000000, 0x01010000, 0x01200000, 0x01201000,
                                         0x01202000, 0x01300000, 0x01310000, 0x01320000)
HEAP, HEAP_END = 0x01400000, 0x01c00000
# Scratch structures the render tests build: a FIELD_MESH and FCOVER, a
# model with its mmat and vertex, normal and colour arrays, a ccObj, a light
# and an ambient vector.
MESH, COVER, MODEL, MMAT, VERTS, NORMS, COLS = (0x01330000, 0x01330100, 0x01330200, 0x01330300,
                                                0x01331000, 0x01333000, 0x01335000)
OBJ, LIGHT, AMBIENT = 0x01330400, 0x01330500, 0x01330700
FVN, FCOLOR = 0x96000, 0xB5430
# The layout below is the same on all four volumes, read from each one's own
# code: FIELD's size from WORLD::Init's __nw, the grid offsets from
# FIELD::MakeField (map), CheckArea (check), Hide (check3) and CheckMount
# (mnt); WORLD's size from WORLD_MAN's __nw before __ct__5WORLDFv, its
# fields from WORLD::Generate, Init and SetKeyObject..SetTreeObject_B.
FIELD_SIZE, WORLD_SIZE = 0xCFD40, 0x61E0
MAP, CHECK, CHECK3, MNT = 0xAF02C, 0xCE430, 0xCF0B0, 0xCF6F0


def last_call(prog, fn, callee):
    """The address of the last `jal callee` in function `fn`."""
    f, t = prog.symbol_named(fn), prog.symbol_named(callee).value
    jal = 0x0C000000 | (t >> 2 & 0x3FFFFFF)
    words = struct.unpack(f"<{f.size // 4}I", prog.read(f.value, f.size // 4 * 4))
    return max(f.value + 4 * i for i, w in enumerate(words) if w == jal)


class Done(Exception):
    pass


class GameField:
    def __init__(self, data):
        import eemu
        import field
        self.eemu = eemu
        self.field = field
        self.prog = data.p
        self.m = eemu.Machine(self.prog)
        sym = self.prog.symbol_named
        self.sym = lambda n: sym(n).value        # noqa: E731
        # Generate's last jal WORLD_MAN::SetStartPos (INF 0x005a7620)
        self.start_call = last_call(self.prog, "Generate__5WORLDFv", "SetStartPos__9WORLD_MANFPf")
        nop = self.nop
        stubs = {
            "__nw__FUi": self.new, "__nwa__FUi": self.new, "__dl__FPv": nop, "__dla__FPv": nop,
            "GetCCSAdrs__8ccStreamFPCc": self.fake, "GetChunkAdrsF__8ccStreamFPCci": self.fake,
            "GetSubstAdrsF__5ccAnmFPCcb": self.fake,
            "SetPosition__6FCOVERFiiiP5FIELDP8ccStreamPcf": self.cover,
            # VU0 macro code, done with field.py's model of it (the FPU's
            # rules): what collisionLP, InitQuad and Lambert call.
            "sceVu0SubVector": self.vsub, "sceVu0AddVector": self.vadd,
            "sceVu0InnerProduct": self.vdot, "sceVu0Normalize": self.vnorm,
            "sceVu0OuterProduct": self.vouter, "sceVu0ScaleVector": self.vscale,
            "sceVu0DivVector": self.vdivv,
        }
        for name in ("__ct__7ccCoordFv", "Init__7ccClumpFP12ccClumpChunk", "SetFogSw__7ccClumpFi",
                     "SetFogSw__5ccAnmFi", "SetTex__8ccSpriteFPcPci", "ccInitKanji__Fii",
                     "SetFog__9ccDrawEnvFffffUi", "SetAmbient__9ccDrawEnvFPf", "GetAmbient__9ccDrawEnvFPf",
                     "Init__7ccLayerFsP6ccView", "SetFrame__6ccViewFffffffff",
                     "AddGrp__10ccLightGrpFP7ccLight", "SetLightDirection__9WORLD_MANFP14ccDistantLight",
                     "__dt__7ccClumpFv", "Init__5ccEffFP10ccEffChunki", "Duplicate__5ccObjFUi",
                     "Init__10ccTexChunkFP12ccChunkIndexP12ccChunkIndexP14ccTexChunkDesc",
                     "ChangeTex__7ccModelFP10ccTexChunkP10ccTexChunk", "GetAmbient__5ccAnmFPf",
                     "SetAnm__5ccAnmFP10ccAnmChunkUi", "_AnimateForward__5ccAnmFUi", "__ct__5ccAnmFv",
                     "SetClump__10FIELD_MESHFP8ccStreamPc", "sceVu0ApplyMatrix",
                     "sceVu0UnitMatrix", "sceVu0TransMatrix", "CalcQuadVertexNormal__5FIELDFv",
                     "CalcVertexColor__5FIELDFPfP14ccDistantLight", "SetSmallMESH__5FIELDFiiP7ccModel",
                     "HitEnable__8FOBJECT2Fv", "Init__7ccModelFP12ccModelChunkPP7ccCoordUi",
                     "Duplicate__7ccModelFUi"):
            stubs[name] = nop
        for name, fn in stubs.items():
            self.m.hooks[self.sym(name)] = fn
        self.m.hooks[self.start_call] = self.start

    # stubs ---------------------------------------------------------------
    @staticmethod
    def nop(m, *_):
        return 0

    def new(self, m, n, *_):
        a = self.heap
        self.heap = (a + max(n, 16) + 15) & ~15
        if self.heap > HEAP_END:
            raise self.eemu.Stop("scratch heap exhausted")
        m.mem[a:a + n] = bytes(n)
        return a

    def fake(self, m, *_):
        return self.new(m, 0x400)

    def vec(self, m, a):
        return tuple(m.load(a + 4 * i, 4) for i in range(4))

    def put(self, m, d, v):
        for i, c in enumerate(v):
            m.store(d + 4 * i, 4, c)

    def start(self, m, *_):
        raise Done()            # a1 holds the start position

    def cover(self, m, fc, x, y, t, *_):
        # a0-a3 are this, x, y, t; the model name is in t2 (FCOVER::SetPosition)
        name = m.r[10] & 0xFFFFFFFF
        self.covers.append((x, y, t, m.mem[name:m.mem.index(0, name)].decode()))
        return 0

    def vsub(self, m, d, a, b, *_):
        for i in range(4):
            m.store(d + 4 * i, 4, self.eemu.f_sub(m.load(a + 4 * i, 4), m.load(b + 4 * i, 4)))
        return 0

    def vadd(self, m, d, a, b, *_):
        for i in range(4):
            m.store(d + 4 * i, 4, self.eemu.f_add(m.load(a + 4 * i, 4), m.load(b + 4 * i, 4)))
        return 0

    def vdot(self, m, a, b, *_):
        m.f[0] = self.field.vu_inner(self.vec(m, a), self.vec(m, b))
        return 0

    def vnorm(self, m, d, a, *_):
        self.put(m, d, self.field.vu_normalize(self.vec(m, a)))
        return 0

    def vouter(self, m, d, a, b, *_):
        self.put(m, d, self.field.vu_outer(self.vec(m, a), self.vec(m, b)))
        return 0

    def vscale(self, m, d, a, *_):
        self.put(m, d, self.field.vu_scale(self.vec(m, a), m.f[12]))
        return 0

    def vdivv(self, m, d, a, *_):
        self.put(m, d, self.field.vu_div(self.vec(m, a), m.f[12]))
        return 0

    def vapply(self, m, d, mat, v, *_):
        cols = [self.vec(m, mat + 16 * i) for i in range(4)]
        self.put(m, d, self.field.vu_apply(cols, self.vec(m, v)))
        return 0

    def vunit(self, m, d, *_):
        for i, col in enumerate(self.field.UNIT_MATRIX):
            self.put(m, d + 16 * i, col)
        return 0

    def vrot(self, axis):
        def rot(m, d, src, *_):
            cols = [self.vec(m, src + 16 * i) for i in range(4)]
            for i, col in enumerate(self.field.vu_rot(cols, axis, m.f[12])):
                self.put(m, d + 16 * i, col)
            return 0
        return rot

    def light_vu(self):
        """The VU0 helpers Lambert and GetDirc also call, modelled likewise."""
        for name, fn in (("sceVu0ApplyMatrix", self.vapply), ("sceVu0UnitMatrix", self.vunit),
                         ("sceVu0RotMatrixX", self.vrot(0)), ("sceVu0RotMatrixY", self.vrot(1)),
                         ("sceVu0RotMatrixZ", self.vrot(2))):
            self.m.hooks[self.sym(name)] = fn

    # runs ----------------------------------------------------------------
    def setup(self, seed, ft, weather, ground, obj, event, randcnt=0):
        m = self.m
        self.heap = HEAP
        self.covers = []
        m.mem[W:W + WORLD_SIZE] = bytes(WORLD_SIZE)
        m.mem[FLD:FLD + FIELD_SIZE] = bytes(FIELD_SIZE)
        m.mem[WM:WM + 0x4e0] = bytes(0x4e0)
        m.mem[WP:WP + 0x30] = bytes(0x30)
        m.store(W + 0x10, 4, ft)
        m.store(W + 0x14, 4, weather)
        m.store(W + 0x6168, 4, FLD)
        m.store(W + 0x6148, 4, self.new(m, 0x400))
        m.store(WM + 0x8, 4, 1)
        m.store(WM + 0x10, 4, ft)
        m.store(WM + 0xc, 4, weather)
        m.store(WM + 0x120, 4, event)
        m.store(WM + 0x158, 4, WP)
        m.store(WP + 0xc, 4, ft)
        m.store(WP + 0x14, 4, weather)
        m.store(WP + 0x18, 4, ground)
        m.store(WP + 0x1c, 4, obj)
        m.store(GAME + 0x24, 4, event)
        for name, va in (("worldman", WM), ("game", GAME), ("saveData", SAVE), ("ccSys", SYS),
                         ("cc3d", CC3D)):
            m.store(self.sym(name), 4, va)
        m.store(self.sym("seed"), 4, seed)
        m.store(self.sym("randcnt"), 4, randcnt)

    def init_draws(self, ft, weather):
        """fieldrand calls WORLD::Init makes."""
        self.setup(12345, ft, weather, 0, 0, 0)
        self.m.call(self.sym("Init__5WORLDFv"), (W,), limit=100_000_000)
        return self.m.load(self.sym("randcnt"), 4)

    def generate(self, seed, ft, weather, ground, obj, event=0, randcnt=0):
        self.setup(seed, ft, weather, ground, obj, event, randcnt)
        try:
            self.m.call(self.sym("Generate__5WORLDFv"), (W,), limit=500_000_000)
        except Done:
            pass
        else:
            raise AssertionError("Generate returned without reaching the start position")
        return self.read()

    # render ----------------------------------------------------------------
    def load_field(self, fl):
        """A FIELD made by its constructor (every colour 128), holding
        field.py's heights and chip grids, and WORLD pointing at it."""
        m = self.m
        self.heap = HEAP
        m.mem[FLD:FLD + FIELD_SIZE] = bytes(FIELD_SIZE)
        m.call(self.sym("__ct__5FIELDFv"), (FLD,))
        for i, v in enumerate(fl.map):
            m.store(FLD + MAP + 4 * i, 4, v)
        m.mem[FLD + CHECK:FLD + CHECK + 3200] = bytes(fl.check)
        m.mem[FLD + CHECK3:FLD + CHECK3 + 1600] = bytes(fl.check3)
        m.mem[FLD + MNT:FLD + MNT + 1600] = bytes(fl.mnt)
        m.store(W + 0x6168, 4, FLD)

    def get_height(self, x, y):
        self.m.f[12], self.m.f[13] = x, y
        self.m.call(self.sym("GetHeight__5WORLDFff"), (W,), limit=10_000_000)
        return self.m.f[0]

    def model(self, scale, verts, normals=None, colours=None):
        """One rigid mmat of `verts` vertices (z sentinel 0x7777) at MODEL."""
        m = self.m
        m.mem[MODEL:MODEL + 0x100] = bytes(0x100)
        m.mem[MMAT:MMAT + 0x40] = bytes(0x40)
        m.store(MODEL + 8, 4, MMAT)
        m.store(MODEL + 12, 4, scale)
        m.store(MODEL + 34, 2, 1)                       # mmatNum; type 0: rigid
        m.store(MMAT + 4, 4, verts)
        m.store(MMAT + 16, 4, VERTS)
        m.store(MMAT + 20, 4, NORMS)
        m.store(MMAT + 24, 4, COLS)
        for k in range(verts):
            m.store(VERTS + 6 * k + 4, 2, 0x7777)
            m.store(NORMS + 4 * k, 4, normals[k] if normals else 0)
            m.store(COLS + 4 * k, 4, colours[k] if colours else 0x80808080)

    def model_z(self, verts):
        return {k: self.eemu.sx(self.m.load(VERTS + 6 * k + 4, 2), 16) for k in range(verts)
                if self.m.load(VERTS + 6 * k + 4, 2) != 0x7777}

    def model_colours(self, verts):
        return {k: tuple(self.m.load(COLS + 4 * k + i, 1) for i in range(3)) for k in range(verts)}

    def tile(self, mx, my, scale):
        """FIELD_MESH::RelocateMesh over the loaded FIELD."""
        m = self.m
        m.mem[MESH:MESH + 0x60] = bytes(0x60)
        m.store(MESH, 4, FLD)
        m.store(MESH + 76, 4, MODEL)
        self.model(scale, 24)
        m.call(self.sym("RelocateMesh__10FIELD_MESHFii"), (MESH, mx, my), limit=10_000_000)
        return {"visible": m.load(MESH + 12, 4) == 1, "pos": tuple(m.load(MESH + 16 + 4 * i, 4) for i in range(3)),
                "z": self.model_z(24), "colours": self.model_colours(24)}

    def cover_tile(self, i, j, t, scale):
        """FCOVER::SetPosition, then FIELD::SetSmallMESH as WORLD::SetCover
        calls them."""
        m = self.m
        addr = self.sym("SetPosition__6FCOVERFiiiP5FIELDP8ccStreamPcf")
        hook = m.hooks.pop(addr)
        try:
            m.mem[COVER:COVER + 0x40] = bytes(0x40)
            m.call(addr, (COVER, i, j, t, FLD, self.new(m, 0x400), self.new(m, 16), 0), limit=1_000_000)
        finally:
            m.hooks[addr] = hook
        pos = tuple(m.load(COVER + 16 + 4 * k, 4) for k in range(3))
        self.model(scale, 6)
        small = self.sym("SetSmallMESH__5FIELDFiiP7ccModel")
        stub = m.hooks.pop(small)           # stubbed for Generate runs
        try:
            m.call(small, (FLD, 2 * i + 1 + (t & 1), 2 * j + 1 + (t >> 1), MODEL), limit=1_000_000)
        finally:
            m.hooks[small] = stub
        return {"offset": pos, "z": self.model_z(6), "colours": self.model_colours(6)}

    def set_light(self, light):
        """A ccDistantLight as DecodeF_DistantLight leaves it: the local
        matrix sceVu0RotMatrix of the rotation (field.py's model), no parent,
        the dirty flag set, colour * 1/255; and the ambient vector."""
        import anim
        m, f = self.m, self.field
        m.mem[LIGHT:LIGHT + 0xE0] = bytes(0xE0)
        rad = [f.f_div(f.f_mul(0x40490FDB, d), f.F(180.0)) for d in light.rotation]
        for i, col in enumerate(anim.rot_bits(rad)):
            self.put(m, LIGHT + 64 + 16 * i, col)
        m.store(LIGHT + 141, 1, 1)
        self.put(m, LIGHT + 192, light.colour_rgb() + (0,))
        self.put(m, AMBIENT, light.ambient_rgb() + (0,))

    def colours(self, light):
        """InitQuad (with CalcQuadVertexNormal) then CalcVertexColor: the
        vertex normals and FIELD.color, as float bits."""
        m = self.m
        self.light_vu()
        self.set_light(light)
        # Generate runs these two stubbed; here they are what is checked.
        stubbed = {a: m.hooks.pop(a) for a in (self.sym("CalcQuadVertexNormal__5FIELDFv"),
                                               self.sym("CalcVertexColor__5FIELDFPfP14ccDistantLight"))}
        try:
            m.call(self.sym("InitQuad__5FIELDFv"), (FLD,), limit=100_000_000)
            vn = [self.vec(m, FLD + FVN + 16 * i) for i in range(6400)]
            m.call(self.sym("CalcVertexColor__5FIELDFPfP14ccDistantLight"), (FLD, AMBIENT, LIGHT),
                   limit=100_000_000)
        finally:
            m.hooks.update(stubbed)
        col = [self.vec(m, FLD + FCOLOR + 16 * i)[:3] for i in range(6400)]
        return vn, col

    def object_colours(self, light, normals, colours):
        """CalcObjectVertexColor over one rigid mmat."""
        m = self.m
        self.light_vu()
        self.set_light(light)
        self.model(0x3F800000, len(normals), normals, colours)
        m.mem[OBJ:OBJ + 0xA0] = bytes(0xA0)
        m.store(OBJ + 148, 4, MODEL)
        m.call(self.sym("CalcObjectVertexColor__FPfP8ccStreamP5ccObjP14ccDistantLight"),
               (AMBIENT, 0, OBJ, LIGHT), limit=10_000_000)
        return [tuple(m.load(COLS + 4 * k + i, 1) for i in range(4)) for k in range(len(normals))]

    def read(self):
        m = self.m
        mem = m.mem
        out = {"map": [m.load(FLD + MAP + 4 * i, 4) for i in range(6400)],
               "check": bytes(mem[FLD + CHECK:FLD + CHECK + 3200]),
               "check3": bytes(mem[FLD + CHECK3:FLD + CHECK3 + 1600]),
               "mnt": bytes(mem[FLD + MNT:FLD + MNT + 1600]),
               "seed": m.load(self.sym("seed"), 4), "randcnt": m.load(self.sym("randcnt"), 4),
               "covers": list(self.covers)}
        objs = []
        for i in range(m.load(W + 0x18, 4)):            # fobj2[KeyObjNum]: type, idx, mx, my, wp z
            o = m.load(W + 0x5090 + 4 * i, 4)
            objs.append(tuple(m.load(o + k, 4) for k in (0, 4, 8, 12, 72)))
        for i in range(1600):                           # fobj[40][40]
            o = m.load(W + 0x1e90 + 4 * i, 4)
            if o:
                objs.append((i // 40, i % 40) + tuple(m.load(o + k, 4) for k in (0, 4, 8, 12, 56)))
        out["objects"] = objs
        # the start chip: SetStartPos's argument, in world units
        pos = m.r[5] & 0xFFFFFFFF
        out["start"] = (m.load(pos, 4), m.load(pos + 4, 4))
        return out


def snapshot(fl):
    """field.Field in the shape GameField.read() returns."""
    import eemu
    check = bytes(fl.check)
    objs2, objs = [], []
    for (kind, idx, x, y, w, h, _model), z in zip(fl.objects, fl.object_z):
        chip = eemu.f_from_py(1200.0)
        wx = eemu.f_add(eemu.f_mul(chip, eemu.f_from_int(x)),
                        eemu.f_div(eemu.f_mul(chip, eemu.f_from_int(w)), eemu.f_from_py(2.0)))
        wy = eemu.f_add(eemu.f_mul(chip, eemu.f_from_int(y)),
                        eemu.f_div(eemu.f_mul(chip, eemu.f_from_int(h)), eemu.f_from_py(2.0)))
        mx = eemu.f_to_int(eemu.f_div(wx, eemu.f_from_py(600.0)))
        my = eemu.f_to_int(eemu.f_div(wy, eemu.f_from_py(600.0)))
        if kind in (3, 4):
            objs.append(((x, y), (kind, idx, mx, my, z)))
        else:
            objs2.append((kind, idx, mx, my, z))
    objs.sort()
    return {"map": list(fl.map), "check": check, "check3": bytes(fl.check3), "mnt": bytes(fl.mnt),
            "seed": fl.rng.seed, "randcnt": fl.rng.count, "covers": list(fl.covers),
            "objects": objs2 + [k + v for k, v in objs],
            "start": fl.start_world[:2]}


def compare(py, game):
    out = []
    for k in py:
        a, b = py[k], game[k]
        if a != b:
            if isinstance(a, (list, bytes)) and len(a) == len(b):
                i = next(i for i in range(len(a)) if a[i] != b[i])
                out.append((f"{k}[{i}]", a[i], b[i], sum(x != y for x, y in zip(a, b))))
            else:
                out.append((k, str(a)[:200], str(b)[:200]))
    return out


def check_field(test, field, data, game, seed, ft, weather, ground, obj, event=0, protect=0):
    """field.Field against the game's Generate from the same RNG state."""
    fl = field.Field(data, seed, ft, weather, ground, obj, event, protect,
                     skip_init=True).generate()
    got = game.generate(seed, ft, weather, ground, obj, event)
    test.assertEqual(compare(snapshot(fl), got), [],
                     f"{data.p.path}: seed {seed} fieldType {ft} weather {weather} "
                     f"ground {ground} object {obj}")


def check_story(test, field, data, game, code):
    """Story area `code` through field.from_words (the executable's own
    volume, its IsProtectArea record, WORLD::Init's draws) against the game's
    Generate started from the same RNG state and count."""
    e = next(e for e in data.areas.events if e["code"] == code)
    words = [data.areas.find(i, e[k]).ID for i, k in enumerate(("wordA", "wordB", "wordC"))]
    area, fl = field.from_words(data, *words, server=e["server"])
    test.assertEqual(area["event"], code)
    seed, draws = fl.rng.seed, fl.init_draws
    fl.generate()
    got = game.generate(seed, area["fieldType"], area["weather"], area["ground"], area["object"],
                        code, draws)
    test.assertEqual(compare(snapshot(fl), got), [], f"{data.p.path}: story area {code}")


@unittest.skipUnless(os.path.exists(ELF), "game executable not present")
class TestField(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import field
        cls.field = field
        cls.data = field.Data(ELF)
        cls.game = GameField(cls.data)

    def check(self, seed, ft, weather, ground, obj, event=0, protect=0):
        check_field(self, self.field, self.data, self.game, seed, ft, weather, ground, obj,
                    event, protect)

    def test_init_draws_match_game(self):
        for ft, weather in ((0, 0), (1, 2), (2, 0), (5, 0), (5, 4), (6, 5), (8, 3)):
            self.assertEqual(self.game.init_draws(ft, weather), self.field.init_draws(ft, weather),
                             (ft, weather))

    def test_random_fields_match_game(self):
        rnd = random.Random(5)
        for ft in (0, 1, 7, 8, 10):
            self.check(rnd.getrandbits(32), ft, rnd.randrange(4), rnd.randrange(3), rnd.randrange(3))

    def test_first_story_area(self):
        area, fl = self.field.from_words(self.data, "Bursting", "Passed Over", "Aqua Field")
        self.assertEqual(area["fieldSeed"], 1420855)
        self.assertEqual(fl.init_draws, 0)

    def test_newlib_sqrt(self):
        self.assertIs(self.data.sqrt, self.field.sqrtf)

    # render -------------------------------------------------------------------
    LIGHT = (0x8094786E, (0x420D0EBC, 0x41F00000, 0x420D0EBC), 0xDFF0F7)   # bg_p1's
    TILE_SCALE, COVER_SCALE = 0x44160000, 0x43960000                          # 600, 300

    def render_field(self):
        """The first story area: a lake, hidden chips, hills."""
        _area, fl = self.field.from_words(self.data, "Bursting", "Passed Over", "Aqua Field")
        return fl.generate()

    def test_heights_match_game(self):
        """WORLD::GetHeight at random points, on cell and chip borders and
        on hidden chips, against field.py's get_height."""
        fl = self.render_field()
        self.game.load_field(fl)
        rnd = random.Random(7)
        F = self.field.F
        points = [(rnd.uniform(0, 48000), rnd.uniform(0, 48000)) for _ in range(150)]
        points += [(600.0 * rnd.randrange(80), rnd.uniform(0, 48000)) for _ in range(20)]
        points += [(1200.0 * x + 1.0, 1200.0 * y + 599.5) for x, y in ((0, 0), (39, 39), (20, 20))]
        hidden = [(x, y) for x in range(40) for y in range(40) if fl.check3[x * 40 + y]]
        points += [(1200.0 * x + 300.0, 1200.0 * y + 700.0) for x, y in hidden[:5]]
        self.assertTrue(hidden)
        for x, y in points:
            with self.subTest(x=x, y=y):
                self.assertEqual(self.game.get_height(F(x), F(y)), fl.get_height(F(x), F(y)))

    def test_ground_matches_game(self):
        """InitQuad's vertex normals and CalcVertexColor's colours over the
        whole map, then FIELD_MESH::RelocateMesh (SetMESH2) for chips at the
        edges, on hidden chips and at random, and FCOVER::SetPosition with
        SetSmallMESH for covers of every quadrant - against field.py."""
        f = self.field
        fl = self.render_field()
        light = f.Light(*self.LIGHT)
        self.game.load_field(fl)
        vn, colours = self.game.colours(light)
        self.assertEqual(vn, f.vertex_normals(fl))
        want = f.vertex_colours(fl, light)
        self.assertEqual(colours, [tuple(c) for c in want])
        rnd = random.Random(8)
        hidden = [(x, y) for x in range(40) for y in range(40) if fl.check3[x * 40 + y]]
        chips = [(0, 0), (39, 39), (39, 0), (0, 39)] + hidden[:4] + [(rnd.randrange(40), rnd.randrange(40))
                                                                    for _ in range(12)]
        for mx, my in chips:
            with self.subTest(chip=(mx, my)):
                got = self.game.tile(mx, my, self.TILE_SCALE)
                py = f.tile(fl, mx, my, self.TILE_SCALE, want)
                self.assertEqual(got["visible"], py["visible"])
                self.assertEqual(got["pos"], py["pos"])
                self.assertEqual(got["z"], py["z"])
                self.assertEqual(got["colours"], py["colours"])
        covers = [c for c in fl.covers if c[2] == 0][:3] + [c for c in fl.covers if c[2] == 1][:3] + \
            [c for c in fl.covers if c[2] == 2][:3] + [c for c in fl.covers if c[2] == 3][:3] + \
            [c for c in fl.covers if c[0] in (0, 39) or c[1] in (0, 39)][:4]
        for cover in covers:
            with self.subTest(cover=cover):
                got = self.game.cover_tile(cover[0], cover[1], cover[2], self.COVER_SCALE)
                py = f.cover_mesh(fl, cover, self.COVER_SCALE, want)
                self.assertEqual(got["offset"], f.cover_offset(cover[2]))
                self.assertEqual(got["z"], py["z"])
                self.assertEqual(got["colours"], py["colours"])

    def test_object_colours_match_game(self):
        """CalcObjectVertexColor on random normals and colours."""
        rnd = random.Random(9)
        normals = [rnd.getrandbits(24) for _ in range(40)] + [0x400000, 0xC00000, 0]
        colours = [rnd.getrandbits(32) for _ in range(len(normals))]
        light = self.field.Light(*self.LIGHT)
        got = self.game.object_colours(light, normals, colours)
        want = self.field.object_colours([n.to_bytes(4, "little") for n in normals],
                                         [c.to_bytes(4, "little") for c in colours], light)
        self.assertEqual(got, [tuple(w) for w in want])


class TestOtherVolumes(unittest.TestCase):
    """Mutation, Outbreak and Quarantine, each against its own WORLD::Init
    and WORLD::Generate: a few Init runs, random fields of types 1 and 7, and
    one story area of a lake type - the first (event 14, dungeon near the
    start) on MUT, a protected one on OUT, one without a dungeon entrance on
    QUA. From OUT on, ccGetDist takes the FPU's sqrt.s."""

    STORY = {2: 14, 3: 23, 4: 42}       # volumeNum: story area (field type 10, 8, 9)

    def test_volumes(self):
        import eemu
        import field
        present = [p for p in OTHERS if os.path.exists(p) and os.path.exists(p + ".syms")]
        if not present:
            self.skipTest("no other volume extracted with a .syms sidecar")
        for path in present:
            with self.subTest(path=path):
                data = field.Data(path)
                volume = data.areas.volume
                self.assertEqual(volume, OTHERS.index(path) + 2)
                self.assertIs(data.sqrt, field.sqrtf if volume == 2 else eemu.f_sqrt)
                game = GameField(data)
                for ft, weather in ((0, 1), (2, 0), (5, 3), (6, 7)):
                    self.assertEqual(game.init_draws(ft, weather), field.init_draws(ft, weather),
                                     (path, ft, weather))
                rnd = random.Random(volume)
                for ft in (1, 7):
                    check_field(self, field, data, game, rnd.getrandbits(32), ft,
                                rnd.randrange(10), rnd.randrange(3), rnd.randrange(3))
                check_story(self, field, data, game, self.STORY[volume])


if __name__ == "__main__":
    unittest.main()
