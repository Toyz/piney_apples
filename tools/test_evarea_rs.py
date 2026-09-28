#!/usr/bin/env python3
"""crates/piney-world's story map of area 15 (piney_world::evarea,
EVENTAREA02) against the game's own code run in tools/eemu.py (the Rust
eemu_rs.so when present).

The evarea_probe example builds the map and answers the requests below; the
game's side is EVENTAREA02 built by its own constructor in the interpreter
(gcmn 0x00401e00), its STATICMODELs, STATICOBJECT and clumps made by their
own code over se1_2's objects as tools/test_world_rs.py's Pieces serves
them, and run through:

  - the set-up: the constructor, then ChangeBlock (gcmn 0x00402300) to the
    church and back: SetFog's arguments, each STATICMODEL's row, model and
    position, the STATICOBJECT's animation and root, the background clumps,
    the lights put in the group, the hit models put on the list,
    WORLD_MAN::SetStartPos's eventStartPos, revAnm, and the lens flare's six
    ccEffs (Init, SetRenderState);
  - the draw: EVENTAREA02::Draw (0x004033e0) frame after frame, eyes and
    players all over both blocks, with and without an event scene: every
    piece in order - ccClump::Draw with its layer and matrix, ccModel::Draw
    with its model, matrix, fog flags and layer (STATICMODEL::Draw and
    DrawWithOutFog), ccAnm::Draw with its time, layer and matrix (the
    STATICOBJECT and revAnm), ccEff::Draw's position (DrawLensFlare,
    0x00402dc0) - and MoveTexture's offsets in the scrolling materials;
  - the lens flare alone for random camera states (eye, view, rotations,
    the eye view): DrawLensFlare's six positions;
  - the ground: ccLandHitCheck (gcmn 0x00571e00) with game.area 1 and
    eventAreaFlag set at 1,500 points of each block, on and off the floors
    (se1_2's Hit chunks decoded by ccStream::Decode_Hit): z, the result
    count, the nearest result's attribute and point,
    checkHitResultAttlibute;
  - the door: WORLD_MAN::Enter (main 0x0019dda0) with game.field 15 and
    game.block -1, 0 and 1 over the constructed map (ChangeBlock run, the
    ChangeScene it asks recorded), then WORLD_MAN::SetCharPosition's event
    area branch (0x001a1414) for the leader and the three others;
  - the frame: cameraMain, ccPlayer::Main over each block's collision in
    an event area (WORLD_MAN::GetHeight 0, no height map, the -48000..48000
    bounds, EVENTAREA::AddCenter), frame by frame from Kite's arrival at
    DMY_marker01: up the bridge into the church's door (Enter), random
    pads, the church from DMY_marker01_2 out through its door; everything
    tools/test_world_rs.py compares for the town, plus Enter and the
    centre.

What runs in Python in place of the game: tools/test_world_rs.py's (the anm
playback by tools/anim.py, the AI, the effects), and the pieces' own
drawing (ccModel::Draw, ccClump::Draw, ccAnm::Draw, ccEff::Draw and Init
are recorded), the light group (AddGrp recorded) and the file's streams.

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
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

import test_world_rs as tw  # noqa: E402
from test_world_rs import GAME, ONE, PLAYER, SCRATCH, STREAM, TCAM, WM, fb, hexs  # noqa: E402

ROOT = tw.ROOT
ISO, ELF = tw.ISO, tw.ELF
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "evarea_probe")
EV = tw.EV
EVENTMNG, WORLDMAN = tw.EVENTMNG, tw.WORLDMAN
ACTIVE_CAM, CAMID, PLW_PW = inf_va(0x0037896C), inf_va(0x0037897C), inf_va(0x00730300)
LAYER_ACTIVE = tw.LAYER_ACTIVE
EA02_VTABLE = inf_va(0x00375B30)
# WORLD_MAN's layers (+0x494 bgLayer[10], objLayer, obj2Layer, floorLayer,
# effLayer, charLayer, refLayer) and their priorities as GO makes them.
LAYERS = [(0x494 + 4 * k, p) for k, p in enumerate((-100, -90, -80, -70, -60, -50, -45, -40, -35, -30))] + [
    (0x4BC, 0), (0x4C0, -10), (0x4C4, -20), (0x4C8, 20), (0x4CC, 10), (0x4D0, 30)]
BOUND = 48000.0
UNIT = [ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE]


def f32(bits):
    return struct.unpack("<f", struct.pack("<I", bits & 0xFFFFFFFF))[0]


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                    "evarea_probe"], cwd=ROOT, check=True)


def ask(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True,
                       cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


def set_area(m, obj=0):
    """game and WORLD_MAN as GO(1) leaves them for area 15's map."""
    m.store(GAME + 0x14, 4, 1)          # area: a field
    m.store(GAME + 0x24, 4, 15)         # field: the story area
    m.store(WM + 0x8, 4, 1)             # WORLD_MAN.flag: GO(1)
    m.store(WM + 0x120, 4, 15)          # eventAreaNumber
    m.store(WM + 0x124, 4, 1)           # eventAreaFlag
    for off, v in ((0x420, -BOUND), (0x424, -BOUND), (0x428, BOUND), (0x42C, BOUND)):
        m.store(WM + off, 4, fb(v))
    m.store(WM + 0x444, 4, obj)


class MapGame(tw.Pieces):
    """EVENTAREA02 as the game builds, changes and draws it."""

    FILES = ("se1_2", "town_z")

    def __init__(self):
        super().__init__(self.FILES)
        m, sym = self.m, self.sym
        c = self.ccs[self.FILES[0]]
        # The model each Hit chunk hangs on (ccModel +0x3c): a fake
        # ccModelHit per hit, recorded when STATICMODEL enables it.
        self.hit_of = {}
        for off, t, _n, _end in c.chunks():
            if t is not None and t & 0xFFFF == 0x0B00:
                obj, parent = struct.unpack_from("<II", c.data, off + 8)
                self.hit_of[c.objects[parent][0]] = c.objects[obj][0]
        self.mh_name = {}
        self.log = []
        rec = self.log.append

        def model_init(mm, model, chunk, *a):
            name = self.names[chunk]
            self.names[model] = name
            if name in self.hit_of:
                mh = self.malloc(mm, 0xA0)
                self.mh_name[mh] = self.hit_of[name]
                mm.store(model + 0x3C, 4, mh)
            return model
        m.hooks[sym("Init__7ccModelFP12ccModelChunkPP7ccCoordUi")] = model_init
        m.hooks[sym("HitEnable__10ccModelHitFi")] = lambda mm, mh, *a: rec(("hit", self.mh_name.get(mh))) or 0
        self.streams = {}

        def ccs_adrs(mm, name, *a):
            from eemu import _cstr
            s = _cstr(mm, name).decode()
            if s not in self.streams:
                self.streams[s] = self.malloc(mm, 0x100)
            rec(("ccs", s))
            return self.streams[s]
        m.hooks[sym("GetCCSAdrs__8ccStreamFPCc")] = ccs_adrs

        def fog(mm, env, color, *a):
            rec(("fog", [mm.f[12], mm.f[13], mm.f[14], mm.f[15], color & 0xFFFFFFFF]))
            return 0
        m.hooks[sym("SetFog__9ccDrawEnvFffffUi")] = fog

        def clump_init(mm, clump, chunk, *a):
            self.names[clump] = self.names[chunk]
            return 0
        m.hooks[sym("Init__7ccClumpFP12ccClumpChunk")] = clump_init
        for name in ("SetLightEnv__5ccAnmF11ccAnmOption", "GetAmbient__5ccAnmFPf", "SetAmbient__9ccDrawEnvFPf"):
            m.hooks[sym(name)] = lambda mm, *a: 0
        self.subst = {}

        def subst(mm, anm, name, *a):
            from eemu import _cstr
            s = _cstr(mm, name).decode()
            if s not in self.subst:
                self.subst[s] = self.malloc(mm, 0x100)
                self.names[self.subst[s]] = s
            return self.subst[s]
        m.hooks[sym("GetSubstAdrsF__5ccAnmFPCcb")] = subst
        m.hooks[sym("AddGrp__10ccLightGrpFP7ccLight")] = lambda mm, g, l, *a: rec(("light", self.names[l])) or 0
        m.hooks[sym("DelGrp__10ccLightGrpFP7ccLight")] = lambda mm, g, l, *a: 0
        m.hooks[sym("SetLightDirection__9WORLD_MANFP14ccDistantLight")] = (
            lambda mm, w, l, *a: rec(("dirlight", self.names[l])) or 0)

        def eff_init(mm, eff, chunk, fog_sw, *a):
            self.names[eff] = self.names[chunk]
            rec(("eff", self.names[chunk], fog_sw))
            return 0
        m.hooks[sym("Init__5ccEffFP10ccEffChunki")] = eff_init
        for name in ("__dt__11STATICMODELFv", "__dt__12STATICOBJECTFv", "__dt__7ccClumpFv", "__dt__5ccAnmFv"):
            m.hooks[sym(name)] = lambda mm, *a: 0
        # The draws.
        del m.hooks[sym("SetActiveLayer__9WORLD_MANFi")]
        self.layer_prio = {}
        for off, prio in LAYERS:
            la = self.malloc(m, 0x40)
            m.store(WM + off, 4, la)
            self.layer_prio[la] = prio
        self.mats = {}

        def mat_subst(mm, stream, name, *a):
            from eemu import _cstr
            s = _cstr(mm, name).decode()
            if s not in self.mats:
                mat = self.mats[s] = self.malloc(mm, 0x40)
                mm.store(mat + 12, 4, self.malloc(mm, 0x40))     # its chunk: no crop
            return self.mats[s]
        m.hooks[sym("GetSubstAdrsF__8ccStreamFPCci")] = mat_subst
        self.draws = []
        lay = lambda mm: self.layer_prio.get(mm.load(LAYER_ACTIVE, 4))          # noqa: E731
        m.hooks[sym("Draw__7ccModelFP16ccDrawModelParam")] = lambda mm, model, param, *a: self.draws.append(
            ("model", model, self.rvec(mm.load(param + 144, 4), 16), mm.load(param + 296, 4), lay(mm))) or 0
        m.hooks[sym("Draw__7ccClumpFv")] = lambda mm, cl, *a: self.draws.append(
            ("clump", cl, self.rvec(cl + 0x40, 16), lay(mm))) or 0
        m.hooks[sym("Draw__5ccAnmFv")] = lambda mm, anm, *a: self.draws.append(
            ("anm", anm, self.anm[anm][1], self.rvec(anm + 0x40, 16), lay(mm))) or 0
        m.hooks[sym("Draw__5ccEffFPfUs")] = lambda mm, eff, pos, *a: self.draws.append(
            ("eff", eff, self.rvec(pos), lay(mm))) or 0
        # The camera the flare reads, the player DrawWithOutFog measures from.
        m.store(ACTIVE_CAM, 4, TCAM)
        m.store(CAMID, 2, 1)
        m.store(PLW_PW, 4, PLAYER)
        m.store(EVENTMNG, 4, EV)
        m.store(tw.CCSYS, 4, tw.SYS)
        self.build()

    def build(self):
        """The map."""
        m = self.m
        self.obj = self.malloc(m, 0x200)
        set_area(m, self.obj)
        self.log.clear()
        self.call("__ct__11EVENTAREA02Fv", self.obj)
        assert m.load(self.obj + 0x1C0, 4) == EA02_VTABLE

    def state(self):
        """The map as the probe's `state` lists it."""
        m, o = self.m, self.obj
        models = []
        for k in range(m.load(o, 4)):
            sm = m.load(o + 0x78 + 4 * k, 4)
            models.append([k, m.load(sm, 4), self.names[m.load(sm + 4, 4)], self.rvec(sm + 0x10)])
        objects = []
        for k in range(m.load(o + 4, 4)):
            so = m.load(o + 0x140 + 4 * k, 4)
            anm = m.load(so + 8, 4)
            objects.append([k, self.anm[anm][0], self.rvec(anm + 0x40, 16), self.rvec(so + 0x10), m.load(so + 0x30, 4),
                            self.anm[anm][1]])
        bg = [self.names.get(m.load(o + 0x1A8 + 4 * k, 4)) for k in range(5)]
        rev = m.load(o + 0x1E4, 4)
        return {"block": m.load(o + 0x1E0, 4, True), "models": models, "objects": objects, "bg": bg,
                "start": self.rvec(WM + 0x70), "rev": self.anm[rev][1] if rev else -1}

    def logged(self, kind):
        return [e[1] if len(e) == 2 else list(e[1:]) for e in self.log if e[0] == kind]

    def change_block(self, b):
        self.log.clear()
        self.call("ChangeBlock__11EVENTAREA02Fi", self.obj, b)

    def camera(self, eye, view, rot, rot3, kind, puppet):
        m = self.m
        self.eye = list(eye)
        self.vec(TCAM, list(eye))
        self.vec(TCAM + 0x10, list(view))
        self.vec(TCAM + 0x20, list(rot))
        self.vec(TCAM + 0x40, list(rot3))
        m.store(TCAM + 0x5C, 4, kind)
        m.store(EV + 0x78C, 4, puppet)

    def draw(self, player):
        """One EVENTAREA02::Draw: its pieces as the probe's `draw` lists
        them (each with its matrix, to check apart), and the scrolls."""
        m, o = self.m, self.obj
        self.vec(PLAYER + 0x40, list(player) + [ONE])
        self.draws = []
        self.call("Draw__11EVENTAREA02Fv", o)
        bg = {m.load(o + 0x1A8 + 4 * k, 4): k for k in range(5)}
        models = {m.load(m.load(o + 0x78 + 4 * k, 4) + 4, 4): k for k in range(m.load(o, 4))}
        objs = {m.load(m.load(o + 0x140 + 4 * k, 4) + 8, 4): k for k in range(m.load(o + 4, 4))}
        flares = {m.load(o + 0x1C4 + 4 * k, 4): k for k in range(6)}
        rev = m.load(o + 0x1E4, 4)
        pieces, mats = [], []
        for d in self.draws:
            if d[0] == "clump":
                pieces.append(["bg", bg[d[1]], d[3]])
                mats.append(d[2])
            elif d[0] == "model":
                assert d[3] in (0x2C, 0x0C), d
                k = models[d[1]]
                pieces.append(["model", k, d[4], int(d[3] == 0x2C)])
                # sceVu0TransMatrix of the unit matrix: 0 + the position.
                from eemu import f_add
                pos = self.rvec(m.load(o + 0x78 + 4 * k, 4) + 0x10, 3)
                assert d[2][12:15] == [f_add(0, v) for v in pos], (k, d[2], pos)
                mats.append(d[2])
            elif d[0] == "anm" and d[1] == rev:
                pieces.append(["rev", d[4], d[2]])
                mats.append(d[3])
            elif d[0] == "anm":
                pieces.append(["obj", objs[d[1]], d[4], d[2]])
                mats.append(d[3])
            elif d[0] == "eff":
                pieces.append(["flare", flares[d[1]], d[3], d[2]])
                mats.append(None)
        tex = []
        for name in ("MAT_se1_2cl1", "MAT_se1_2wa1"):
            a = self.mats.get(name)
            tex.append([m.load(a + 20, 2), m.load(a + 22, 2)] if a else [0, 0])
        return {"pieces": pieces, "tex": tex}, mats


FIELDRAND_SEED, RANDCNT = inf_va(0x00377D20), inf_va(0x00378A80)
EAB0_VTABLE = inf_va(0x00375BF0)


class ArenaGame(MapGame):
    """EVENTAREAB0 for field 1 (Skeith's arena, se1_5) as the game builds and
    draws it, its 54 FIREFLY2s and their sparks the game's own (each
    ccEff::Draw recorded with its position, pattern, transparency and
    scale); fieldrand runs natively from a seed."""

    FILES = ("se1_5",)

    def build(self):
        m, sym = self.m, self.sym
        lay = lambda mm: self.layer_prio.get(mm.load(LAYER_ACTIVE, 4))          # noqa: E731
        m.hooks[sym("Draw__5ccEffFPfUs")] = lambda mm, eff, pos, pat, *a: self.draws.append(
            ("eff", self.rvec(pos), pat & 0xFFFF, mm.load(eff + 0x34, 4), mm.load(eff + 0x20, 4), lay(mm))) or 0
        self.seed = 0

    def make(self, field, seed):
        m = self.m
        self.obj = self.malloc(m, 0x300)
        set_area(m, self.obj)
        m.store(GAME + 0x24, 4, field)
        m.store(WM + 0x120, 4, field)
        m.store(FIELDRAND_SEED, 4, seed)
        self.log.clear()
        self.call("__ct__11EVENTAREAB0Fv", self.obj)
        assert m.load(self.obj + 0x1C0, 4) == EAB0_VTABLE

    def state(self):
        m, o = self.m, self.obj
        models = []
        for k in range(m.load(o, 4)):
            sm = m.load(o + 0x78 + 4 * k, 4)
            models.append([k, m.load(sm, 4), self.names[m.load(sm + 4, 4)], self.rvec(sm + 0x10)])
        bg = [self.names.get(m.load(o + 0x1A8 + 4 * k, 4)) for k in range(3)]
        return {"models": models, "bg": bg, "start": self.rvec(WM + 0x70)}

    def draw(self, player):
        """One EVENTAREAB0::Draw with the player at `player`: the pieces,
        the bobbing models, the scroll's v offset, fieldrand's seed."""
        m, o = self.m, self.obj
        self.vec(PLAYER + 0x40, list(player) + [ONE])
        self.draws = []
        self.call("Draw__11EVENTAREAB0Fv", o)
        bg = {m.load(o + 0x1A8 + 4 * k, 4): k for k in range(3)}
        models = {m.load(m.load(o + 0x78 + 4 * k, 4) + 4, 4): k for k in range(m.load(o, 4))}
        pieces = []
        for d in self.draws:
            if d[0] == "clump":
                pieces.append(["bg", bg[d[1]], d[3]])
            elif d[0] == "model":
                pieces.append(["model", models[d[1]], d[4]])
            elif d[0] == "eff":
                pieces.append(list(d))
        a = self.mats.get("MAT_se1_5clo1")
        v = m.load(a + 22, 2) if a else 0
        bob = [self.rvec(m.load(o + 0x78 + 4 * k, 4) + 0x10) for k in (6, 7, 8)]
        return {"pieces": pieces, "bob": bob, "v": v, "seed": m.load(FIELDRAND_SEED, 4)}


def qualifies(att):
    """checkHitResultAttlibute takes a result's ground-type nibbles unless
    it is shaded (0x40000) or of type 0xe000e0."""
    return att & 0x40000 != 0x40000 and att & 0xF0F0F0 != 0xE000E0


def church_eyes(rng, n, block):
    """Camera eyes over a block: a grid over its floor's box and random
    points, each with the player near it."""
    x0, x1, y0, y1 = (-1150, 1150, -4150, 4150) if block == 0 else (-900, 900, -600, 6100)
    out = []
    for x in range(x0, x1 + 1, 575):
        for y in range(y0, y1 + 1, 830):
            out.append((float(x), float(y), 400.0))
    for _ in range(n):
        out.append((rng.uniform(x0 - 2000, x1 + 2000), rng.uniform(y0 - 2000, y1 + 2000), rng.uniform(-200, 2500)))
    rng.shuffle(out)
    return out


def flare_camera(rng, eye):
    """A camera at `eye` looking somewhere, with random turns."""
    view = [fb(eye[0] + rng.uniform(-3000, 3000)), fb(eye[1] + rng.uniform(-3000, 3000)),
            fb(eye[2] + rng.uniform(-600, 600)), ONE]
    rot = [fb(rng.uniform(-1.4, 1.4)), fb(rng.choice([0.0, rng.uniform(-0.3, 0.3)])), fb(rng.uniform(-3.2, 3.2)),
           rng.choice([0, ONE])]
    rot3 = [fb(rng.uniform(-1.4, 1.4)), 0, fb(rng.uniform(-3.2, 3.2)), rng.choice([0, ONE])]
    return view, rot, rot3


class Frames(tw.Field):
    """The game's camera and player over the map's collision: se1_2's two
    Hit chunks decoded by ccStream::Decode_Hit, one ccModelHit each at the
    identity, the block's on the list."""

    def __init__(self):
        super().__init__("se1_2")
        m, sym = self.m, self.sym
        c = self.town
        self.mh = []
        for off, t, n, end in c.chunks():
            if t is None or t & 0xFFFF != 0x0B00:
                continue
            self.data, self.cur_read = c.data, off + 8
            m.store(STREAM + 0x84, 4, 0)
            self.call("Decode_Hit__8ccStreamFv", STREAM)
            mh = self.malloc(m, 0xA0)
            self.call("__ct__10ccModelHitFv", mh)
            m.store(mh + 12, 4, m.load(m.load(STREAM + 0x84, 4) + 16, 4))
            self.vec(mh + 0x10, UNIT)
            self.mh.append(mh)
        self.map = self.malloc(m, 0x200)
        self.entered = []
        m.hooks[sym("Enter__9WORLD_MANFPf")] = lambda mm, *a: self.entered.append(1) or 0

    def block(self, b):
        for mh in self.mh:
            self.m.store(mh, 4, 0)
            self.m.store(mh + 4, 4, 0)
        self.call("initHitCheck1__Fv")
        self.call("HitEnable__10ccModelHitFi", self.mh[b], 0)

    def area(self, center=None):
        set_area(self.m, self.map)
        self.m.store(GAME + 0x18, 4, 0)
        if center is not None:
            self.m.store(self.map + 8, 4, center[0])
            self.m.store(self.map + 12, 4, center[1])

    def start(self, pos, dircz, scheme, mode, seed):
        super().start(pos, dircz, scheme, mode, seed)
        self.area(center=pos[:2])
        self.entered = []

    def frame(self):
        self.events = []
        self.entered = []
        self.call("cameraMain__Fv")
        self.call("Main__8ccPlayerFv", PLAYER)
        s = self.state()
        s["enter"] = int(bool(self.entered))
        s["center"] = [self.m.load(self.map + 8, 4), self.m.load(self.map + 12, 4)]
        return s


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class ArenaAgainstGame(unittest.TestCase):
    """EVENTAREAB0 (field 1, Skeith's arena) against piney_world::evarea_b0."""

    @classmethod
    def setUpClass(cls):
        build()

    def test_arena(self):
        g = ArenaGame()
        seed = 0x12345
        g.make(1, seed)
        log = list(g.log)
        port = ask(["arena 1 %x" % seed])[0]
        want = g.state()
        self.assertEqual([[k, t, n, p[:3]] for k, t, n, p in want["models"]],
                         [[r, t, n, p[:3]] for r, t, n, p in port["models"]])
        self.assertEqual(want["bg"], port["bg"])
        self.assertEqual(want["start"], port["start"])
        self.assertEqual([e[1] for e in log if e[0] == "light"], port["lights"])
        self.assertEqual([e[1] for e in log if e[0] == "dirlight"], port["lights"][:1])
        self.assertEqual([e[1] for e in log if e[0] == "hit"], port["hits"])
        self.assertEqual([e[1] for e in log if e[0] == "fog"], [port["fog"]])
        self.assertEqual([e[1] for e in log if e[0] == "ccs"], ["se1_5"])
        self.assertEqual(g.m.load(tw.SYS + 0x18, 4), 0x1E1E1E)
        # Draw, with SwitchLayer now and then (Skeith's magic), the player
        # walking between the fireflies' markers and the world's origin
        # (where a firefly that lives out its life starts again).
        rng = random.Random(3)
        spots = [(0.0, 0.0)] + [tuple(f32(v) for v in g.dummies["DMY_marker%02d" % k][0][:2]) for k in range(1, 19)]
        lines, switches, players = [], [], []
        x, y = spots[1]
        goal = spots[0]
        for f in range(900):
            if f % 90 == 0:
                goal = rng.choice(spots)
            dx, dy = goal[0] - x, goal[1] - y
            d = max(math.hypot(dx, dy), 1.0)
            step = min(d, 40.0)
            x += dx / d * step + rng.uniform(-5, 5)
            y += dy / d * step + rng.uniform(-5, 5)
            player = [fb(x), fb(y), fb(rng.uniform(0, 50))]
            players.append(player)
            sw = rng.random() < 0.02
            switches.append(sw)
            if sw:
                lines.append("arenaswitch")
            lines.append("arenadraw %s" % hexs(*player))
        got = [r for r in ask(["arena 1 %x" % seed] + lines)[1:] if r]
        self.assertEqual(len(got), 900)
        drawn = 0
        for f, (sw, player, p) in enumerate(zip(switches, players, got)):
            if sw:
                g.call("SwitchLayer__11EVENTAREAB0Fv", g.obj)
            want = g.draw(player)
            self.assertEqual(want, p, "frame %d" % f)
            drawn += sum(1 for q in want["pieces"] if q[0] == "eff")
        # The walk passes the fireflies: some are drawn.
        self.assertGreater(drawn, 900)


class MapAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def compare_state(self, g, port, label):
        """The game's map beside the probe's state."""
        want = g.state()
        self.assertEqual(want["block"], port["block"], label)
        self.assertEqual([[k, t, n, p[:3]] for k, t, n, p in want["models"]],
                         [[r, t, n, p[:3]] for r, t, n, p in port["models"]], label)
        self.assertEqual(want["objects"], port["objects"], label)
        self.assertEqual(want["bg"], port["bg"], label)
        self.assertEqual(want["start"], port["start"], label)
        self.assertEqual(want["rev"], port["rev"], label)
        self.assertEqual(g.logged("light"), port["lights"], label)
        self.assertEqual(g.logged("hit"), port["hits"], label)
        fog = g.logged("fog")
        self.assertEqual(fog[-1], port["fog"], label)

    def test_setup(self):
        """The constructor, then ChangeBlock to the church and back."""
        g = MapGame()
        got = ask(["new", "block 1", "block 0", "block 1"])
        log = list(g.log)
        # The constructor: its own fog, se1_2 and town_z, the flares.
        fogs = [e[1] for e in log if e[0] == "fog"]
        self.assertEqual(fogs[0], [fb(1500.0), fb(12000.0), 0, fb(70.0), 0x1E1E1E])
        self.assertEqual([e[1] for e in log if e[0] == "ccs"], ["se1_2", "town_z"])
        self.assertEqual([e[1:] for e in log if e[0] == "eff"],
                         [(n, 1) for n in ("EFF_sflenz_2", "EFF_sflenz_3", "EFF_sflenz_4", "EFF_sflenz_5",
                                           "EFF_sflenz_6", "EFF_sflenz_1")])
        for k in range(6):
            eff = g.m.load(g.obj + 0x1C4 + 4 * k, 4)
            self.assertEqual(g.m.load(eff + 0x58, 4) & 0x10000, 0)      # SetRenderState(ZENABLE, 0)
            self.assertEqual(g.m.load(eff + 0x62, 2) & 0x20, 0)         # PRIM's fog bit off
        self.assertEqual(g.m.load(tw.SYS + 0x18, 4), 0x1E1E1E)          # ccSys.bgColor
        self.compare_state(g, got[0], "constructor")
        for b, port in zip((1, 0, 1), got[1:]):
            g.change_block(b)
            self.compare_state(g, port, "ChangeBlock(%d)" % b)
            self.assertEqual(g.logged("dirlight"), port["lights"][:1], b)

    def run_draws(self, g, frames, label):
        """Draw frame after frame on both sides: (block, player, eye, puppet
        show, flare view, rot, rot3, eye view) each."""
        lines = ["new"]
        for fr in frames:
            if fr[0] == "block":
                lines.append("block %x" % fr[1])
                continue
            _, player, eye, puppet, view, rot, rot3, kind = fr
            r = rot3 if kind == 1 else rot
            lines.append("draw " + hexs(*eye, *player, puppet, *view, *r))
        lines.append("revroot")
        got = ask(lines)[1:]
        rev_root = got.pop()
        self.assertEqual(len(got), len(frames), label)
        seen = {"flare": 0, "obj": 0, "rev": 0, "nofog": 0, "bg": 0}
        for i, (fr, port) in enumerate(zip(frames, got)):
            if fr[0] == "block":
                g.change_block(fr[1])
                continue
            _, player, eye, puppet, view, rot, rot3, kind = fr
            g.camera(eye, view, rot, rot3, kind, puppet)
            want, mats = g.draw(player)
            self.assertEqual(want, port, "%s frame %d eye %s" % (label, i, [f32(v) for v in eye]))
            for p, mt in zip(want["pieces"], mats):
                seen[p[0]] = seen.get(p[0], 0) + 1
                seen["nofog"] += p[0] == "model" and p[3] == 0
                if p[0] in ("bg", "obj"):
                    self.assertEqual(mt, UNIT, p)
                elif p[0] == "rev":
                    self.assertEqual(mt, rev_root, p)
                elif p[0] == "model":
                    self.assertEqual(mt[:12], UNIT[:12], p)
            if fr[3]:
                self.assertFalse(any(p[0] == "flare" for p in want["pieces"]))
        return seen

    def test_draw(self):
        """EVENTAREA02::Draw over both blocks, the eye and the player all
        over them, an event scene now and then: the pieces in order and the
        scrolls, stepped as the game steps them."""
        g = MapGame()
        rng = random.Random(5)
        frames = []
        for block in (0, 1, 0, 1):
            frames.append(("block", block))
            for e in church_eyes(rng, 120, block):
                eye = [fb(v) for v in e] + [ONE]
                player = [fb(e[0] + rng.uniform(-900, 900)), fb(e[1] + rng.uniform(-900, 900)), fb(rng.choice([0.0, 200.0]))]
                view, rot, rot3 = flare_camera(rng, e)
                frames.append(("draw", player, eye, int(rng.random() < 0.15), view, rot, rot3,
                               1 if rng.random() < 0.2 else 3))
        seen = self.run_draws(g, frames, "draw")
        print("\nEVENTAREA02::Draw: %d frames, %s" % (sum(1 for f in frames if f[0] == "draw"), seen), file=sys.stderr)
        for k in ("flare", "obj", "rev", "nofog", "bg"):
            self.assertGreater(seen[k], 0, k)

    def test_lens_flare(self):
        """DrawLensFlare's six positions for random camera states in the
        church."""
        g = MapGame()
        rng = random.Random(8)
        frames = [("block", 1)]
        for _ in range(400):
            e = (rng.uniform(-5000, 5000), rng.uniform(-3000, 12000), rng.uniform(-500, 3000))
            eye = [fb(v) for v in e] + [ONE]
            view, rot, rot3 = flare_camera(rng, e)
            frames.append(("draw", [fb(0.0), fb(900.0), 0], eye, 0, view, rot, rot3, rng.choice([1, 3, 3])))
        seen = self.run_draws(g, frames, "flare")
        self.assertEqual(seen["flare"], 6 * 400)

    def test_land(self):
        """ccLandHitCheck over each block's floor: points on it, beside it,
        above and below it, and far off."""
        f = Frames()
        m = f.m
        for block, (x0, x1, y0, y1) in enumerate(((-1150, 1150, -4150, 4150), (-900, 900, -600, 6100))):
            rng = random.Random(40 + block)
            pts = []
            for i in range(1500):
                x, y = rng.uniform(x0 - 300, x1 + 300), rng.uniform(y0 - 300, y1 + 300)
                if i % 5 == 0:
                    x, y = rng.uniform(-20000, 20000), rng.uniform(-20000, 20000)
                z = rng.choice([0.0, 200.0, 300.0, rng.uniform(-300, 2500)])
                pts.append((fb(x), fb(y), fb(z)))
            got = ask(["new"] + (["block 1"] if block else []) + ["land " + hexs(*p) for p in pts])[1 + block:]
            self.assertEqual(len(got), len(pts))
            f.area()
            f.block(block)
            bad = hits = door = 0
            for p, g in zip(pts, got):
                f.vec(tw.ARGS, list(p) + [ONE])
                f.call("ccLandHitCheck__FPfUi", tw.ARGS, 0x20000001)
                z = m.f[0]
                num = m.load(f.sym("hitResultNum"), 4)
                near = f.sym("hitResultNearest")
                att = m.load(near + 0x48, 4)
                cp = f.rvec(near)
                attribute = f.call("checkHitResultAttlibute__Fv") & 0xFFFFFFFF
                want = {"z": z, "num": num, "att": att if num else g["att"], "cp": cp if num else g["cp"],
                        "attribute": attribute if num else g["attribute"]}
                if not num or not qualifies(att):
                    # checkHitResultAttlibute's type nibbles from a stale
                    # register (see tools/test_field_rt.py).
                    want["attribute"] &= 0xFF0F0F0F
                    g["attribute"] &= 0xFF0F0F0F
                hits += num != 0
                door += num != 0 and att & 0x80000 != 0
                if want != g:
                    bad += 1
                    if bad < 5:
                        print("land", block, [f32(v) for v in p], want, g)
            self.assertEqual(bad, 0, "block %d" % block)
            self.assertGreater(hits, 300)
            print("\nccLandHitCheck, block %d: %d points, %d on the floor, %d on the door" %
                  (block, len(pts), hits, door), file=sys.stderr)

    def test_door(self):
        """WORLD_MAN::Enter on area 15's door from game.block -1, 0 and 1
        over the constructed map, then SetCharPosition for the party."""
        g = MapGame()
        m = g.m
        scenes = []
        def signed(x):
            x &= 0xFFFFFFFF
            return x - (1 << 32) if x >= 1 << 31 else x
        # ChangeScene(game, a, t, fd, d, f, b): $a1-$a3, $t0-$t2.
        m.hooks[g.sym("ChangeScene__6ccGameFiiiiii")] = lambda mm, *a: scenes.append(
            [signed(mm.r[k]) for k in (5, 6, 7, 8, 9, 10)]) or 0
        lines = ["new"]
        order = (-1, 1, 0, 1, 0)
        for b in order:
            lines += ["enter %x" % (b & 0xFFFFFFFF), "charpos"]
        got = ask(lines)[1:]
        self.assertEqual(len(got), 2 * len(order))
        for i, b in enumerate(order):
            port, pos = got[2 * i], got[2 * i + 1]
            m.store(GAME + 0x30, 4, b & 0xFFFFFFFF)
            scenes.clear()
            g.log.clear()
            g.call("Enter__9WORLD_MANFPf", WM, tw.ARGS)
            self.assertEqual(scenes, [[-2, -2, -2, -2, -2, port["to"]]], b)
            self.compare_state(g, port["state"], "Enter from block %d" % b)
            self.assertEqual(port["to"], port["state"]["block"])
            # SetCharPosition(p, f0, f1, f2): the leader and the others.
            members = [SCRATCH + 0xB000 + 0x40 * k for k in range(3)]
            for k, a in enumerate(members):
                m.store(a, 4, fb(k + 1.0))
            g.call("SetCharPosition__9WORLD_MANFPfPfPfPf", WM, tw.ARGS, *members)
            want = [m.load(tw.ARGS + 4 * k, 4) for k in (1, 2, 3, 4)]
            want.append([[m.load(a + 4 * k, 4) for k in (1, 2, 3, 4)] for a in members])
            self.assertEqual(want, pos, "SetCharPosition in block %d" % port["to"])

    def run_frames(self, f, block, pos, dircz, scheme, mode, seed, pads, label):
        lines = ["new"] + (["block 1"] if block else []) + ["start " + hexs(*pos, dircz, scheme, mode, seed)]
        for d, p, pl, dl, pr, dr, pw in pads:
            lines.append("pad " + hexs(d, p, pl, dl, pr, dr, *pw))
        got = ask(lines)[2 + block:]
        self.assertEqual(len(got), len(pads), label)
        f.block(block)
        f.start(pos, dircz, scheme, mode, seed)
        stats = {"moved": 0, "acts": set(), "enter": 0, "first_enter": None, "frames": 0}
        for i, ((d, p, pl, dl, pr, dr, pw), g) in enumerate(zip(pads, got)):
            f.pad(d, p, pl, dl, pr, dr, pw)
            want = f.frame()
            stats["frames"] += 1
            stats["acts"].add(want["act"])
            stats["moved"] += want["move_flag"]
            stats["enter"] += want["enter"]
            if want["enter"] and stats["first_enter"] is None:
                stats["first_enter"] = i
            if not g.pop("qualified"):
                want["attribute"] &= 0xFF0F0F0F
                g["attribute"] &= 0xFF0F0F0F
            diff = [k for k in want if want[k] != g[k]]
            if diff:
                self.fail("%s frame %d: %s\n game %s\n port %s" % (
                    label, i, diff, {k: want[k] for k in diff}, {k: g[k] for k in diff}))
        return stats

    def test_frames(self):
        """Kite arriving at DMY_marker01 and running up the bridge into the
        church's door; random pads on the holy ground; the church from
        DMY_marker01_2, out through its door."""
        f = Frames()
        rng = random.Random(15)
        # DMY_marker01 (0, -3600, 0), facing -pi: the camera behind him
        # looks along +y, so the stick up runs him up the bridge, and the
        # walls lead him onto the doorway's floor (x 0..190 from y 3575),
        # whose attribute has the Enter bit.
        marker01 = (0, fb(-3600.0), 0)
        facing = 0xC0490FDA
        pads = []
        for i in range(420):
            lx, ly = (128, 128) if i < 80 else (128, 0)
            dl, pl = tw.stick(lx, ly)
            pads.append((0, 0, pl, dl, 0, 0, [0] * 12))
        s = self.run_frames(f, 0, marker01, facing, 0, 3, 1, pads, "to the door")
        self.assertGreater(s["enter"], 0)
        total = s["frames"]
        print("\nthe door: Enter from frame %d" % s["first_enter"], file=sys.stderr)
        for k in range(3):
            pads = tw.script_pads("walk", 60, rng) + tw.script_pads("random", 360, rng)
            x, y = rng.uniform(-700, 700), rng.uniform(-3800, 3000)
            total += self.run_frames(f, 0, (fb(x), fb(y), fb(0.0)), fb(rng.uniform(-3, 3)), rng.randrange(4), 3,
                                     rng.randrange(1000), pads, "holy ground random %d" % k)["frames"]
        # The church: in from DMY_marker01_2, a walk up the nave, then back
        # out through the door (the stick down).
        marker01_2 = (0, fb(900.0), 0)
        pads = []
        for i in range(480):
            lx, ly = (128, 128) if i < 80 else ((128, 0) if i < 220 else (140, 255))
            dl, pl = tw.stick(lx, ly)
            pads.append((0, 0, pl, dl, 0, 0, [0] * 12))
        s = self.run_frames(f, 1, marker01_2, facing, 1, 3, 7, pads, "the church")
        self.assertGreater(s["enter"], 0)
        total += s["frames"]
        print("the church's door: Enter from frame %d" % s["first_enter"], file=sys.stderr)
        for k in range(2):
            pads = tw.script_pads("walk", 60, rng) + tw.script_pads("random", 360, rng)
            x, y = rng.uniform(-600, 600), rng.uniform(200, 5500)
            total += self.run_frames(f, 1, (fb(x), fb(y), fb(0.0)), fb(rng.uniform(-3, 3)), rng.randrange(4), 3,
                                     rng.randrange(1000), pads, "church random %d" % k)["frames"]
        print("%d frames, none differing" % total, file=sys.stderr)


if __name__ == "__main__":
    unittest.main()
