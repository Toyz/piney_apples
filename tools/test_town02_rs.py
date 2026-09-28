#!/usr/bin/env python3
"""crates/piney-world's Dun Loireag (piney_world::town02, ROOTTOWN02, with
its CLOUDs and LENSFLARE: piney_world::cloud, lensflare) against the game's
own code run in tools/eemu.py (the Rust eemu_rs.so when present).

The town02_probe example builds the town and answers the requests below;
the game's side is ROOTTOWN02 built by its own constructor in the
interpreter (gcmn 0x004240c0), its STATICMODELs, STATICOBJECTs, clumps,
water ccAnms, LENSFLARE and CLOUDs made by their own code over town02's
and town_z's objects as tools/test_world_rs.py's Pieces serves them, and
run through:

  - the set-up: the constructor: SetFog's arguments and ccSys.bgColor,
    the files it asks for, each STATICMODEL's row, type, model and
    position, each STATICOBJECT's root and animation time, the lights put
    in the group (the distant one also given to WORLD_MAN), the water's
    times after its first step, the lens flare's six ccEffs (Init(chunk,
    1), SetRenderState(ZENABLE, 0), PRIM's fog bit off);
  - the draw: ROOTTOWN02::Draw (0x004266a0) frame after frame, the player
    wandering the town (and beyond, so that the clouds are sent back
    about him), the camera looking every way, the eye view and event
    scenes now and then, in town02 and in town02d (crisis): every piece
    in order - ccAnm::Draw with its time and layer (the water, the
    STATICOBJECTs), ccClump::Draw with its layer and matrix (the sky, the
    sun at DMY_sr2lig_1point, the cloud layers, the crisis clumps),
    ccModel::Draw with its model, matrix and layer, the frame-buffer copy,
    DrawMap, ccEff::Draw with its position and pattern (LENSFLARE::Draw's
    six flares, each CLOUD::Draw) - and after it SetUV's value, DrawBG's
    offsets in MAT_sr2clo_1 and _2 (and the crisis materials), every
    CLOUD's angle, pattern, speed, position, turn and transparency,
    fieldrand's seed, and the three scrolls.

What runs in Python in place of the game: tools/test_world_rs.py's (the anm
playback by tools/anim.py), the pieces' own drawing (ccModel::Draw,
ccClump::Draw, ccAnm::Draw, ccEff::Draw and Init are recorded), the light
group (AddGrp recorded), waterUVModifi2 (recorded), the map's sprites and
the file's streams.

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
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

import test_world_rs as tw  # noqa: E402
from test_world_rs import ONE, PLAYER, TCAM, WM, fb, hexs  # noqa: E402

ROOT = tw.ROOT
ISO, ELF = tw.ISO, tw.ELF
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "town02_probe")
EV = tw.EV
EVENTMNG = tw.EVENTMNG
ACTIVE_CAM, CAMID, PLW_PW = inf_va(0x0037896C), inf_va(0x0037897C), inf_va(0x00730300)
LAYER_ACTIVE = tw.LAYER_ACTIVE
RT02_VTABLE = inf_va(0x00375FF0)
# WORLD_MAN's layers (+0x494 bgLayer[10], objLayer, obj2Layer, floorLayer,
# effLayer, charLayer, refLayer) and their priorities as GO makes them.
LAYERS = [(0x494 + 4 * k, p) for k, p in enumerate((-100, -90, -80, -70, -60, -50, -45, -40, -35, -30))] + [
    (0x4BC, 0), (0x4C0, -10), (0x4C4, -20), (0x4C8, 20), (0x4CC, 10), (0x4D0, 30)]
BOUND = 24000.0
# The area generator's seed and the ROOTTOWN02 statics.
SEED, RANDCNT = inf_va(0x00377D20), inf_va(0x00378A80)
U1777, INIT1778 = inf_va(0x00378B6C), inf_va(0x00378B70)
V1346, V2_1347, INIT1348 = inf_va(0x00378170), inf_va(0x00378B5C), inf_va(0x00378B60)


def f32(bits):
    return struct.unpack("<f", struct.pack("<I", bits & 0xFFFFFFFF))[0]


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                    "town02_probe"], cwd=ROOT, check=True)


def ask(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True,
                       cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


class TownGame(tw.Pieces):
    """ROOTTOWN02 as the game builds and draws it."""

    def __init__(self, crisis=False, seed=13):
        stem = "town02d" if crisis else "town02"
        super().__init__((stem, "town_z"))
        m, sym = self.m, self.sym
        # Each Eff chunk's patNum, which ccEff::Init copies to +0x30.
        z = self.ccs["town_z"]
        self.pat_num = {}
        for off, t, _n, _end in z.chunks():
            if t is not None and t & 0xFFFF == 0x0E00:
                obj = struct.unpack_from("<I", z.data, off + 8)[0]
                self.pat_num[z.objects[obj][0]] = struct.unpack_from("<H", z.data, off + 22)[0]
        self.log = []
        rec = self.log.append
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
        nop = lambda mm, *a: 0              # noqa: E731
        for name in ("SetLightEnv__5ccAnmF11ccAnmOption", "GetAmbient__5ccAnmFPf", "SetAmbient__9ccDrawEnvFPf",
                     "Init__7ccLayerFsP6ccView", "SetFrame__6ccViewFffffffff", "__ct__16ccDrawPacketCtrlFv",
                     "SetTex__8ccSpriteFPcPci", "Duplicate__5ccObjFUi",
                     "Init__10ccTexChunkFP12ccChunkIndexP12ccChunkIndexP14ccTexChunkDesc",
                     "ChangeTex__7ccModelFP10ccTexChunkP10ccTexChunk"):
            m.hooks[sym(name)] = nop
        m.hooks[sym("__ct__6ccMaskFii")] = lambda mm, a, *r: a
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
        m.hooks[sym("SetLightDirection__9WORLD_MANFP14ccDistantLight")] = (
            lambda mm, w, l, *a: rec(("dirlight", self.names[l])) or 0)

        def eff_init(mm, eff, chunk, fog_sw, *a):
            name = self.names[chunk]
            self.names[eff] = name
            mm.store(eff + 0x30, 2, self.pat_num.get(name, 0))
            rec(("eff", name, fog_sw))
            return 0
        m.hooks[sym("Init__5ccEffFP10ccEffChunki")] = eff_init
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
        d = self.draws.append
        m.hooks[sym("Draw__7ccModelFP16ccDrawModelParam")] = lambda mm, model, param, *a: d(
            ("model", model, self.rvec(mm.load(param + 144, 4), 16), lay(mm))) or 0
        m.hooks[sym("Draw__7ccClumpFv")] = lambda mm, cl, *a: d(("clump", cl, self.rvec(cl + 0x40, 16), lay(mm))) or 0
        m.hooks[sym("Draw__5ccAnmFv")] = lambda mm, anm, *a: d(
            ("anm", anm, self.anm[anm][1], self.rvec(anm + 0x40, 16), lay(mm))) or 0
        m.hooks[sym("Draw__5ccEffFPfUs")] = lambda mm, eff, pos, pat, *a: d(
            ("eff", eff, self.rvec(pos), pat & 0xFFFF, lay(mm))) or 0
        m.hooks[sym("waterUVModifi2__FP5ccObjPff")] = lambda mm, *a: d(("wateruv",)) or 0
        m.hooks[sym("SetUV__5ccAnmFiiP15ccMaterialChunki")] = lambda mm, anm, u, v, mat, *a: d(
            ("setuv", anm, u & 0xFFFF, v & 0xFFFF, mm.r[8] & 0xFFFFFFFF, lay(mm))) or 0
        m.hooks[sym("MakePacketDrawBuffTrans__7ccLayerFP10ccTexChunk")] = lambda mm, layer, tex, *a: d(
            ("copy", self.layer_prio.get(layer))) or 0
        m.hooks[sym("DrawMap__10ROOTTOWN02Fv")] = lambda mm, *a: d(("map",)) or 0
        # The camera the flare and the clouds read, the player they gather
        # about, the event manager's puppetShow, the town's bounds.
        m.store(ACTIVE_CAM, 4, TCAM)
        m.store(CAMID, 2, 1)
        m.store(PLW_PW, 4, PLAYER)
        m.store(EVENTMNG, 4, EV)
        m.store(tw.CCSYS, 4, tw.SYS)
        for off, v in ((0x420, -BOUND), (0x424, -BOUND), (0x428, BOUND), (0x42C, BOUND)):
            m.store(WM + off, 4, fb(v))
        m.store(tw.SAVE + 0x6772, 1, int(crisis))
        m.store(SEED, 4, seed)
        m.store(RANDCNT, 4, 0)
        for a in (U1777, V1346, V1346 + 4, V2_1347):
            m.store(a, 4, 0)
        for a in (INIT1778, INIT1348):
            m.store(a, 1, 0)
        self.obj = self.malloc(m, 0x260)
        self.log.clear()
        self.call("__ct__10ROOTTOWN02Fv", self.obj)
        assert m.load(self.obj + 0x1AC, 4) == RT02_VTABLE
        o = self.obj
        self.water = [m.load(o + 0x1B0 + 4 * k, 4) for k in range(3)]
        self.model_row = {m.load(m.load(o + 0xA0 + 4 * k, 4) + 4, 4): k for k in range(20)}
        self.obj_row = {m.load(m.load(o + 0x168 + 4 * k, 4) + 8, 4): k for k in range(9)}
        clumps = {0x244: ["sky"], 0x248: ["cloudsky", 0], 0x24C: ["cloudsky", 1], 0x250: ["sun"]}
        clumps.update({0x70 + 4 * k: ["crisis", k] for k in range(3)})
        self.clumps = {m.load(o + off, 4): v for off, v in clumps.items() if m.load(o + off, 4)}
        flare = m.load(o + 0x23C, 4)
        self.flares = {m.load(flare + 4 * k, 4): k for k in range(6)}
        self.cloud_objs = [m.load(o + 0x1D8 + 4 * k, 4) for k in range(25)]

    def state(self):
        """The constructor's work as the probe's `new` reports it."""
        m, o = self.m, self.obj
        models = []
        for k in range(20):
            sm = m.load(o + 0xA0 + 4 * k, 4)
            models.append([k, m.load(sm, 4), self.names[m.load(sm + 4, 4)], self.rvec(sm + 0x10, 3)])
        objects = []
        for k in range(9):
            so = m.load(o + 0x168 + 4 * k, 4)
            anm = m.load(so + 8, 4)
            objects.append([k, self.rvec(anm + 0x40, 16), self.anm[anm][1]])
        fog = [e[1] for e in self.log if e[0] == "fog"]
        return {"models": models, "objects": objects, "fog": fog[-1], "clear": m.load(tw.SYS + 0x18, 4),
                "water": [self.anm[a][1] for a in self.water]}

    def camera(self, eye, view, rot, rot2, kind, puppet):
        m = self.m
        self.eye = list(eye)
        self.vec(TCAM, list(eye))
        self.vec(TCAM + 0x10, list(view))
        self.vec(TCAM + 0x20, list(rot))
        self.vec(TCAM + 0x40, list(rot2))
        m.store(TCAM + 0x5C, 4, kind)
        m.store(EV + 0x78C, 4, puppet)

    def draw(self, player):
        """One ROOTTOWN02::Draw: its pieces as the probe's `draw` lists them,
        with the state after it; and each piece's layer and matrix to check
        apart."""
        m, o = self.m, self.obj
        self.vec(PLAYER + 0x40, list(player) + [ONE])
        self.draws.clear()
        self.call("Draw__10ROOTTOWN02Fv", o)
        pieces, extra, uv = [], [], None
        clouds = {m.load(c + 0x30, 4): k for k, c in enumerate(self.cloud_objs)}
        for e in self.draws:
            if e[0] == "setuv":
                k = self.water.index(e[1])
                if k == 1:
                    assert e[3] == 0 and e[4] == 2, e
                    uv = e[2]
                else:
                    assert k == 2 and e[2] == 0 and e[4] == 1 and e[3] == uv, e
            elif e[0] == "anm" and e[1] in self.water:
                k = self.water.index(e[1])
                pieces.append(["water", k, e[2]])
                extra.append((e[4], e[3]))
            elif e[0] == "anm":
                pieces.append(["obj", self.obj_row[e[1]], e[2]])
                extra.append((e[4], e[3]))
            elif e[0] == "copy":
                pieces.append(["copy"])
                extra.append((e[1], None))
            elif e[0] == "clump":
                pieces.append(list(self.clumps[e[1]]))
                extra.append((e[3], e[2]))
            elif e[0] == "model":
                row = self.model_row[e[1]]
                mat = e[2]
                assert mat[:12] == [ONE, 0, 0, 0, 0, ONE, 0, 0, 0, 0, ONE, 0] and mat[15] == ONE, mat
                pieces.append(["model", row, mat[12:15]])
                extra.append((e[3], None))
            elif e[0] == "map":
                pieces.append(["map"])
                extra.append((None, None))
            elif e[0] == "eff" and e[1] in self.flares:
                pieces.append(["flare", self.flares[e[1]], e[2]])
                extra.append((e[4], e[3]))
            elif e[0] == "eff":
                pieces.append(["cloud", clouds[e[1]], e[2], e[3]])
                extra.append((e[4], None))
        cloud_state = []
        for c in self.cloud_objs:
            eff = m.load(c + 0x30, 4)
            cloud_state.append([m.load(c, 4), m.load(c + 0xC, 4, True), m.load(c + 0x10, 4, True),
                                self.rvec(c + 0x20, 3), m.load(eff + 0x28, 4), m.load(eff + 0x34, 4)])
        clo = [m.load(self.mats[n] + 22, 2) if n in self.mats else 0 for n in ("MAT_sr2clo_1", "MAT_sr2clo_2")]
        bg = m.load(self.mats["MAT_sr2dat1_2"] + 20, 2) if "MAT_sr2dat1_2" in self.mats else 0
        if "MAT_sr2dat1_3" in self.mats:
            assert m.load(self.mats["MAT_sr2dat1_3"] + 20, 2) == bg == m.load(self.mats["MAT_sr2dat1_3"] + 22, 2)
        want = {"pieces": pieces, "uv": uv, "clo": clo, "bg": bg, "clouds": cloud_state, "seed": m.load(SEED, 4),
                "u": m.load(U1777, 4), "scroll": [m.load(V1346, 4), m.load(V1346 + 4, 4), m.load(V2_1347, 4)]}
        return want, extra


def look_at(eye, target, focal=450.0):
    """A world_screen for a camera at `eye` looking at `target`: a GS pixel
    (2048, 2048) centre, `focal` pixels a unit at depth 1, w the depth; its
    columns as float bits."""
    def sub(a, b):
        return [a[i] - b[i] for i in range(3)]

    def dot(a, b):
        return sum(a[i] * b[i] for i in range(3))

    def cross(a, b):
        return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]

    def norm(a):
        n = dot(a, a) ** 0.5
        return [v / n for v in a]
    d = norm(sub(target, eye))
    r = norm(cross(d, [0.0, 0.0, 1.0]))
    u = cross(r, d)
    rows = [[2048 * d[i] + focal * r[i] for i in range(3)], [2048 * d[i] - focal * u[i] for i in range(3)], d, d]
    rows = [row + [-dot(row, eye)] for row in rows]
    return [[fb(rows[i][k]) for i in range(4)] for k in range(4)]


def camera_state(rng, eye):
    """A camera at `eye` looking somewhere, with random turns (as the
    lens flare reads them)."""
    view = [fb(eye[0] + rng.uniform(-3000, 3000)), fb(eye[1] + rng.uniform(-3000, 3000)),
            fb(eye[2] + rng.uniform(-600, 600)), ONE]
    rot = [fb(rng.uniform(-1.4, 1.4)), fb(rng.choice([0.0, rng.uniform(-0.3, 0.3)])), fb(rng.uniform(-3.2, 3.2)),
           rng.choice([0, ONE])]
    rot2 = [fb(rng.uniform(-1.4, 1.4)), 0, fb(rng.uniform(-3.2, 3.2)), rng.choice([0, ONE])]
    return view, rot, rot2


def walk(rng, n):
    """Frames of a player wandering Dun Loireag from its start (0, 3500),
    now and then far off (so that clouds are sent back about him), with the
    camera near him looking every way: (player, eye, puppet, view, rot,
    rot2, kind)."""
    out = []
    x, y = 0.0, 3500.0
    for i in range(n):
        if i % 150 == 149:
            x, y = rng.uniform(-8000, 8000), rng.uniform(-8000, 8000)
        x = max(-12000.0, min(12000.0, x + rng.uniform(-60, 60)))
        y = max(-12000.0, min(12000.0, y + rng.uniform(-60, 60)))
        z = rng.choice([0.0, 0.0, 300.0, rng.uniform(-200, 800)])
        e = (x + rng.uniform(-900, 900), y + rng.uniform(-900, 900), z + rng.uniform(100, 700))
        view, rot, rot2 = camera_state(rng, e)
        # Now and then look straight at the sun (DMY_sr2lig_1point).
        if rng.random() < 0.3:
            view = [fb(-13600.0), fb(-16000.0), fb(1200.0), ONE]
        out.append(([fb(x), fb(y), fb(z)], [fb(v) for v in e] + [ONE], int(rng.random() < 0.1), view, rot, rot2,
                    1 if rng.random() < 0.15 else 3))
    return out


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class TownAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()

    def test_setup(self):
        """The constructor: its files, fog and background colour, the static
        models and objects, the lights, the water, the lens flare's
        ccEffs."""
        for crisis in (0, 1):
            g = TownGame(bool(crisis))
            port = ask(["new %x d" % crisis])[0]
            want = g.state()
            log = list(g.log)
            self.assertEqual([e[1] for e in log if e[0] == "ccs"], ["town02d" if crisis else "town02", "town_z"])
            self.assertEqual(want["fog"], [fb(1500.0), fb(10000.0), 0, fb(75.0), 0x00F0C080])
            self.assertEqual(want["fog"], port["fog"])
            self.assertEqual(want["clear"], port["clear"])
            self.assertEqual(want["models"], [[r[0], r[1], r[2].strip('"'), r[3]] for r in port["models"]])
            self.assertEqual(want["objects"], port["objects"])
            self.assertEqual(want["water"], port["water"])
            lights = [e[1] for e in log if e[0] == "light"]
            self.assertEqual(lights, ["LGT_sr2lig1"] + ["LGT_sr2omn0%d" % k for k in range(1, 6)])
            self.assertEqual([e[1] for e in log if e[0] == "dirlight"], ["LGT_sr2lig1"])
            self.assertEqual([k for k, *_ in port["lights"]], [1, 4, 4, 4, 4, 4])
            effs = [e[1:] for e in log if e[0] == "eff"]
            self.assertEqual(effs, [(n, 1) for n in ("EFF_sflenz_2", "EFF_sflenz_3", "EFF_sflenz_4", "EFF_sflenz_5",
                                                     "EFF_sflenz_6", "EFF_sflenz_1")])
            for eff in g.flares:
                self.assertEqual(g.m.load(eff + 0x58, 4) & 0x10000, 0)      # SetRenderState(ZENABLE, 0)
                self.assertEqual(g.m.load(eff + 0x62, 2) & 0x20, 0)         # PRIM's fog bit off
            sun = g.dummies["DMY_sr2lig_1point"][0]
            self.assertEqual(port["sun"], sun + [ONE])

    def run_draws(self, crisis, frames, seed):
        g = TownGame(bool(crisis), seed)
        lines = ["new %x %x" % (crisis, seed)]
        for player, eye, puppet, view, rot, rot2, kind in frames:
            lines.append("draw " + hexs(*eye[:3], *player, puppet, *view, *rot, *rot2, kind))
        got = ask(lines)[1:]
        self.assertEqual(len(got), len(frames))
        seen = {}
        seed = None
        for i, (fr, port) in enumerate(zip(frames, got)):
            player, eye, puppet, view, rot, rot2, kind = fr
            g.camera(eye, view, rot, rot2, kind, puppet)
            want, extra = g.draw(player)
            self.assertEqual(want, port, "crisis %d frame %d eye %s" % (crisis, i, [f32(v) for v in eye]))
            # A cloud sent back about the player draws from fieldrand again.
            seen["respawn"] = seen.get("respawn", 0) + (seed is not None and want["seed"] != seed)
            seed = want["seed"]
            for p, (layer, mat) in zip(want["pieces"], extra):
                seen[p[0]] = seen.get(p[0], 0) + 1
                expect = {"water": 20 if p[0] == "water" and p[1] != 0 else 0, "obj": 0, "copy": 0, "sky": -100,
                          "sun": -90, "cloudsky": 20, "model": 0, "flare": 20, "cloud": 20}
                if p[0] == "crisis":
                    self.assertEqual(layer, (-80, -70, -60)[p[1]], p)
                elif p[0] in expect:
                    self.assertEqual(layer, expect[p[0]], p)
                if p[0] == "flare":
                    self.assertEqual(mat, 0, p)                             # pattern 0
                if p[0] == "sun":
                    self.assertEqual(mat[12:15], g.dummies["DMY_sr2lig_1point"][0], p)
            if puppet:
                self.assertFalse(any(p[0] == "flare" for p in want["pieces"]))
        return seen

    def test_draw(self):
        """ROOTTOWN02::Draw frame after frame, the player wandering, the
        camera every way: the pieces in order, the clouds, the scrolls."""
        rng = random.Random(22)
        seen = self.run_draws(0, walk(rng, 700), 13)
        print("\nROOTTOWN02::Draw: 700 frames, %s" % seen, file=sys.stderr)
        for k in ("water", "obj", "copy", "sky", "sun", "cloudsky", "model", "map", "flare", "cloud", "respawn"):
            self.assertGreater(seen.get(k, 0), 0, k)

    def test_water_uv(self):
        """waterUVModifi2 (gcmn 0x005025d0) on each town's water 0 - wat1's
        MDL_wat00 in Mac Anu (reach 9000), town02's MDL_sr2wat00 in Dun
        Loireag (32000) - its model laid out in the interpreter (no Bbox, so
        ccObj::CheckBoundingBox passes; no parent, so its world matrix is its
        local one), for cameras all over the town looking every way: the
        texture coordinates it writes, or none when the eye is out of reach."""
        g = TownGame(False)
        m = g.m
        # The real one this time (the draw's test records it).
        del m.hooks[g.sym("waterUVModifi2__FP5ccObjPff")]
        rng = random.Random(24)
        # A ccView (its world_screen at +0xd0).
        view = g.malloc(m, 0x200)
        m.store(tw.SYSLAYER + 0x2C, 4, view)
        eye_at = g.malloc(m, 0x10)
        for town, root in ((0, (0.0, 0.0, 0.0)), (1, (0.0, 4200.0, 0.0))):
            info = ask(["watermodel %x" % town])[0]
            pos = info["positions"]
            n = len(pos) // 3
            obj, model, mm = g.malloc(m, 0x100), g.malloc(m, 0x40), g.malloc(m, 0x40)
            positions, uvs = g.malloc(m, 6 * n), g.malloc(m, 4 * n)
            for i, c in enumerate(pos):
                m.store(positions + 2 * i, 2, c & 0xFFFF)
            m.store(obj + 0x94, 4, model)
            m.store(model + 0x8, 4, mm)
            m.store(model + 0xC, 4, info["scale"])
            m.store(mm + 0x4, 4, n)
            m.store(mm + 0x10, 4, positions)
            m.store(mm + 0x1C, 4, uvs)
            reach = (fb(9000.0), fb(32000.0))[town]
            lines, calls = [], []
            for i in range(300):
                lw_t = [root[0] + rng.choice([0.0, rng.uniform(-50, 50)]), root[1], root[2], 1.0]
                lw = [fb(1.0), 0, 0, 0, 0, fb(1.0), 0, 0, 0, 0, fb(1.0), 0] + [fb(v) for v in lw_t]
                e = [rng.uniform(-12000, 12000), rng.uniform(-12000, 12000), rng.uniform(100, 2500)]
                if i % 10 == 0:
                    e[:2] = [rng.uniform(-40000, 40000), rng.uniform(-40000, 40000)]
                target = [e[0] + rng.uniform(-3000, 3000), e[1] + rng.uniform(-3000, 3000), rng.uniform(-300, 300)]
                ws = look_at(e, target)
                eye = [fb(v) for v in e]
                calls.append((eye, lw, ws))
                lines.append("wateruv %x " % town + hexs(*eye, *lw, *lw[12:16], *[v for col in ws for v in col]))
            got = ask(lines)
            written = 0
            for (eye, lw, ws), port in zip(calls, got):
                for k in range(16):
                    m.store(obj + 0x40 + 4 * k, 4, lw[k])
                m.store(obj + 0x80, 4, 0)
                for k in range(4):
                    for r in range(4):
                        m.store(view + 0xD0 + 16 * k + 4 * r, 4, ws[k][r])
                g.vec(eye_at, eye + [ONE])
                m.mem[uvs:uvs + 4 * n] = bytes(4 * n)
                done = g.call("waterUVModifi2__FP5ccObjPff", obj, eye_at, fargs=[reach])
                want = [[m.load(uvs + 4 * i, 2), m.load(uvs + 4 * i + 2, 2)] for i in range(n)] if done else None
                self.assertEqual(want, port, "town %d eye %s" % (town, [f32(v) for v in eye]))
                written += done != 0
            print("\nwaterUVModifi2, town %d: %d vertices, %d of %d calls within reach" % (town, n, written, len(calls)),
                  file=sys.stderr)
            self.assertTrue(0 < written < len(calls))

    def test_draw_crisis(self):
        """The same in town02d, whose DrawBG adds the three crisis clumps,
        from another fieldrand seed."""
        rng = random.Random(23)
        seen = self.run_draws(1, walk(rng, 200), 1_234_567)
        self.assertEqual(seen.get("crisis"), 3 * 200)


if __name__ == "__main__":
    unittest.main()
