#!/usr/bin/env python3
"""piney-world's light group (`chara::kite::light_matrix`, which lights the
towns', fields', dungeons', event areas' and battles' characters) against
the game's own light code in tools/eemu.py, bit for bit.

Each case builds a light group as the game does: every light made by
`ccCreateLight` (main 0x001389b0) from a chunk that gives only its type,
then left as `ccAnm::SetAnmCtrlWork` (0x00150670) leaves one: its local
matrix (+0x40) with `matCalcSW` on and no parent, so `CheckRange` takes
it as its `lwMatrix` (and a direct or spot light's inverse) itself; its
colour, intensity and fall-off, the squares by the EE's `mul.s`. The
priorities are `ccCreateLight`'s, a boss's `ccLight(4, 1)` and the
dungeon's `ccLight(1, 0)`. Each is added by `ccLightGrp::AddGrp`
(0x00139060) into a `ccDrawEnv`'s group, with an ambient. Then, for a
model with a random `lwMatrix`, `ccModel::Draw`'s steps: `ccDrawEnv::
SetLightMatrix` (0x00105900) at its place, `ccSetMatrixPacket`
(0x0013e350), and VU1's `mc_SetMatrix` (tools/vu.py). VU1's light
registers (vf05-07 the directions, vf09-11 the colours, vf12 the ambient)
are compared word for word with what the world_probe example answers
through `light_matrix`, every slot, empty ones too.

    python3 tools/test_lights_rs.py           the unit tests
    python3 tools/test_lights_rs.py bulk N    N random groups
    python3 tools/test_lights_rs.py anims     every light animation of DATA.BIN

It runs on PINEY_VOLUME's disc, through tools/test_stream_rs.py's game
and tools/test_world_rs.py's probe.
"""

import math
import os
import random
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import eemu  # noqa: E402
import test_stream_rs as S  # noqa: E402
import test_world_rs as W  # noqa: E402
import vu  # noqa: E402

DISTANT, DIRECT, SPOT, OMNI = 1, 2, 3, 4
ONE = 0x3F800000


def fbits(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


class Light:
    """A light as its ccLight holds it, all floats as bits: the local matrix
    (stored rows, which CheckRange copies to the lwMatrix), the colour,
    intensity, fall-off (start, end, end squared), a direct light's radii
    (r0, r1, r1 squared) or a spot light's tangents."""

    def __init__(self, kind, pri, matrix, colour, inten, far=(0, 0, 0), radius=(0, 0, 0), tans=(0, 0)):
        self.kind, self.pri, self.matrix, self.colour, self.inten = kind, pri, matrix, colour, inten
        self.far, self.radius, self.tans = far, radius, tans

    def words(self):
        flat = [w for row in self.matrix for w in row]
        return [self.kind, self.pri & 0xFFFFFFFF] + flat + list(self.colour) + [self.inten] + list(self.far) + \
            list(self.radius) + list(self.tans)

    def __repr__(self):
        return "Light(%d, %d, %s)" % (self.kind, self.pri, " ".join("%08x" % w for w in self.words()[2:]))


def falloff(start, end):
    """(start, end, end squared) as SetAnmCtrlWork stores them."""
    s, e = fbits(start), fbits(end)
    return (s, e, eemu.f_mul(e, e))


def placed(axes, pos):
    """Stored rows: the three axes, then the place (w 1)."""
    return [[fbits(x) for x in a] + [0] for a in axes] + [[fbits(x) for x in pos] + [ONE]]


UNIT_AXES = ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0])


class Lights:
    """The game's light code on one machine, and VU1's mc_SetMatrix."""

    def __init__(self):
        self.g = S.Game()
        self.m = self.g.m
        self.mc = vu.Microcode(S.ELF)
        g, m = self.g, self.m
        self.pkt = g.malloc(m, 0x400)
        m.hooks[g.sym("GetWork__16ccDrawPacketCtrlFi")] = lambda mm, *a: self.pkt
        self.mark = g.heap

    def put(self, a, words):
        for k, w in enumerate(words):
            self.m.store(a + 4 * k, 4, w & 0xFFFFFFFF)

    def run(self, lights, ambient, world):
        """VU1's registers for a lit model with lwMatrix `world` (stored
        rows, bits) under the group `lights` (in the order added) and
        `ambient` (4 words): ([dirs 3x3], [colours 3x3], [ambient 3])."""
        g, m = self.g, self.m
        g.heap = self.mark
        env, chunk = g.malloc(m, 0x200), g.malloc(m, 0x20)
        self.put(env + 0x30, ambient)
        for l in lights:
            m.mem[chunk + 12:chunk + 14] = struct.pack("<h", l.kind)
            lt = g.call("ccCreateLight__FP12ccLightChunk", (chunk,))
            m.store(lt + 0x96, 1, l.pri & 0xFF)
            self.put(lt + 0x40, [w for row in l.matrix for w in row])
            m.store(lt + 0x8D, 1, 1)
            if l.kind == DISTANT:
                self.put(lt + 0xC0, l.colour)
                self.put(lt + 0xB0, [l.inten])
            elif l.kind == OMNI:
                self.put(lt + 0xB0, l.colour)
                self.put(lt + 0xC0, [l.inten] + list(l.far))
            else:
                self.put(lt + 0x130, l.colour)
                self.put(lt + 0x140, [l.inten] + list(l.far[:2]))
                if l.kind == DIRECT:
                    self.put(lt + 0x14C, list(l.radius))
                else:
                    self.put(lt + 0x154, list(l.tans))
            g.call("AddGrp__10ccLightGrpFP7ccLight", (env + 0x80, lt))
        # ccModel::Draw's ccDrawModelParam: the lwMatrix, the environment,
        # a lit mtype; the packet control's view only feeds the other rows.
        param, lw, ctrl, view = (g.malloc(m, n) for n in (0x200, 64, 0x40, 0x300))
        self.put(lw, [w for row in world for w in row])
        for off, n, v in ((144, 4, lw), (340, 4, env), (348, 4, ctrl), (362, 2, 1), (364, 4, ONE), (292, 4, ONE)):
            m.store(param + off, n, v)
        m.store(ctrl + 44, 4, view)
        for off in (208, 228, 248, 268, 476, 492):
            m.store(view + off, 4, ONE)
        g.call("SetLightMatrix__9ccDrawEnvFPA4_fPA4_fPf", (env, param + 160, param + 224, lw + 48))
        g.call("ccSetMatrixPacket__FP16ccDrawModelParam", (param,))
        u = vu.Vu(self.mc.vu1)
        # The matrix packet's UNPACK V4-32 addr 8 num 17, MSCAL mc_SetMatrix.
        for q in range(17):
            u.mem[8 + q] = [m.load(self.pkt + 64 + 16 * q + 4 * k, 4) for k in range(4)]
        u.run(self.mc.vu1.labels["mc_SetMatrix"])
        dirs = [[u.vf[r][k] for r in (5, 6, 7)] for k in range(3)]
        colours = [list(u.vf[9 + k][:3]) for k in range(3)]
        return dirs, colours, list(u.vf[12][:3])


def request(lights, ambient, world):
    words = ["lights", "%x" % len(lights)]
    for l in lights:
        words += ["%x" % w for w in l.words()]
    words += ["%x" % (w & 0xFFFFFFFF) for w in list(ambient) + [w for row in world for w in row]]
    return " ".join(words)


def port_regs(answer):
    d, c = answer["dirs"], answer["colours"]
    return [d[3 * k:3 * k + 3] for k in range(3)], [c[3 * k:3 * k + 3] for k in range(3)], answer["ambient"]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def norm(v):
    n = sum(x * x for x in v) ** 0.5 or 1.0
    return [x / n for x in v]


def unit(rnd):
    return norm([rnd.uniform(-1, 1) for _ in range(3)])


def beam_axes(dirv):
    """A light's axes with its -z along `dirv`."""
    ez = [-x for x in dirv]
    ex = norm(cross([0.0, 1.0, 0.0] if abs(ez[1]) < 0.9 else [1.0, 0.0, 0.0], ez))
    return (ex, cross(ez, ex), ez)


def random_world(rnd, at):
    """A model's lwMatrix: turned about z (a character's heading), now and
    then about the other axes too and scaled, at `at`."""
    rz = rnd.uniform(-math.pi, math.pi)
    rx, ry = (rnd.uniform(-0.6, 0.6), rnd.uniform(-0.6, 0.6)) if rnd.random() < 0.3 else (0.0, 0.0)
    s = rnd.choice((1.0, 1.0, rnd.uniform(0.5, 2.0)))
    cz, sz, cy, sy, cx, sx = math.cos(rz), math.sin(rz), math.cos(ry), math.sin(ry), math.cos(rx), math.sin(rx)
    # Rz Ry Rx, columns as the stored rows.
    cols = [[cz * cy, sz * cy, -sy], [cz * sy * sx - sz * cx, sz * sy * sx + cz * cx, cy * sx],
            [cz * sy * cx + sz * sx, sz * sy * cx - cz * sx, cy * cx]]
    return placed([[s * x for x in c] for c in cols], at)


def random_group(rnd):
    """A group as the areas hold them: distant lights among up to eight
    omni lights, now and then a boss's, a beam or a cone, some out of reach
    or dark; and the model's place and matrix."""
    lights = []
    at = [rnd.uniform(-2000, 2000), rnd.uniform(-2000, 2000), rnd.uniform(0, 300)]

    def col():
        return [fbits(rnd.choice((0.0, rnd.random()))) for _ in range(3)] + [0]

    def inten():
        return fbits(rnd.choice((1.0, rnd.random(), 0.005, 0.3)))

    def fade(hi):
        start = rnd.choice((0.0, rnd.uniform(0, hi)))
        end = 0.0 if start == 0.0 and rnd.random() < 0.5 else start + rnd.uniform(1, hi + 1000)
        return falloff(start, end)

    for _ in range(rnd.randint(1, 9)):
        r = rnd.random()
        if r < 0.2:
            # A beam or a cone from above near the point: straight down, as
            # the event rooms' are, or any way.
            down = rnd.random() < 0.5
            dirv = [0.0, 0.0, -1.0] if down else unit(rnd)
            pos = [at[0] + rnd.uniform(-1500, 1500), at[1] + rnd.uniform(-1500, 1500), rnd.uniform(500, 3000)]
            m = placed(beam_axes(dirv), pos)
            if rnd.random() < 0.7:
                r0 = rnd.uniform(0, 1500)
                r1 = r0 + (rnd.choice((2.0, rnd.uniform(1, 50))) if down else rnd.uniform(100, 1000))
                lights.append(Light(DIRECT, 0, m, col(), inten(), fade(3000), radius=falloff(r0, r1)))
            else:
                a0 = rnd.uniform(1, 40)
                a1 = a0 + rnd.uniform(0.5, 30)
                tans = tuple(fbits(math.tan(math.radians(a))) for a in (a0, a1))
                lights.append(Light(SPOT, 0, m, col(), inten(), fade(3000), tans=tans))
            continue
        if r < 0.4:
            pri = 0 if rnd.random() < 0.2 else -1
            lights.append(Light(DISTANT, pri, placed(beam_axes(unit(rnd)), [0.0] * 3), col(), inten()))
            continue
        pri = 1 if r < 0.5 else 0
        pos = [rnd.uniform(-3000, 3000), rnd.uniform(-3000, 3000), rnd.uniform(-200, 1500)]
        lights.append(Light(OMNI, pri, placed(UNIT_AXES, pos), col(), inten(), fade(2000)))
    ambient = [fbits(rnd.random()) for _ in range(3)] + [0]
    return lights, ambient, random_world(rnd, at)


def compare(game, port):
    """The first part that differs in any word, or None."""
    for part, a, b in zip(("dirs", "colours", "ambient"), game, port):
        if a != b:
            return f"{part}: game {a}, port {b}"
    return None


def run(n, seed):
    rnd = random.Random(seed)
    cases = [random_group(rnd) for _ in range(n)]
    lab = Lights()
    games = [lab.run(*c) for c in cases]
    ports = W.ask([request(*c) for c in cases])
    bad, first, lit = 0, None, 0
    for case, game, answer in zip(cases, games, ports):
        lit += sum(c != [0, 0, 0] for c in game[1])
        d = compare(game, port_regs(answer))
        if d:
            bad += 1
            if first is None:
                first = f"{d}\nlights {case[0]}\nambient {case[1]}\nworld {case[2]}"
    return bad, first, lit


# The lights as the animations leave them -----------------------------------------

STREAM_HOOKS = ("sceGsSetDefLoadImage", "sceGsExecLoadImage", "sceGsSyncPath", "FlushCache", "sceDmaGetChan",
                "sceDmaReset", "sceDmaPutEnv", "sceDmaSend", "sceDmaSendN", "sceDmaSendI", "sceDmaSync",
                "sceDmaWatch", "sceDmaPause", "sceDmaRestart")


_DATA = {}


def data_member(stem):
    """DATA.BIN's member `stem` inflated (the archive read once)."""
    import gzarc
    if not _DATA:
        data = gzarc.open_bytes(W.DATA)
        _DATA.update({mb.name.lower().rsplit(".", 1)[0]: (data, mb) for mb in gzarc.members(data)})
    data, mb = _DATA[stem.lower()]
    return gzarc.inflate(data, mb)


class AnmLights:
    """A DATA.BIN file decoded by the game's ccStream, one of its animations
    played by a ccAnm (`SetAnm`, `_AnimateForward(frameSpd)` a step) and
    each light read back as `CheckRange` would take it: a distant light's
    direction (`GetDirc`), the others' `lwMatrix` (`_SetLWMatrix`, the anm
    its parent)."""

    def __init__(self, stem):
        self.g = g = S.Game()
        m = self.m = g.m
        for name in STREAM_HOOKS:
            m.hooks[g.sym(name)] = lambda mm, *a: 0
        data = data_member(stem)
        m.mem[S.DATA:S.DATA + len(data)] = data
        self.ccs = g.malloc(m, 0x190)
        g.call("__ct__8ccStreamFP14ccStreamReader", (self.ccs, 0))
        m.store(self.ccs + 0x160, 4, S.DATA)
        g.call("DecodeSetup__8ccStreamFv", (self.ccs,))
        m.store(g.sym("active__9ccDrawEnv"), 4, g.malloc(m, 0x200))
        self.name = g.malloc(m, 64)
        self.out = g.malloc(m, 64)
        self.mark = g.heap

    def cstr(self, s):
        self.m.mem[self.name:self.name + len(s) + 1] = s.encode() + b"\0"
        return self.name

    def words(self, a, n):
        return [self.m.load(a + 4 * k, 4) for k in range(n)]

    def run(self, anim, names, steps):
        """For each step count in `steps` (ascending): {name: words} as the
        probe's `animlights` gives them, and the ambient (GetAmbient)."""
        g, m = self.g, self.m
        g.heap = self.mark
        chunk = g.call("GetChunkAdrsF__8ccStreamFPCci", (self.ccs, self.cstr(anim), 0))
        anm = g.malloc(m, 0x200)
        g.call("__ct__5ccAnmFv", (anm,))
        g.call("SetAnm__5ccAnmFP10ccAnmChunkUi", (anm, chunk, 0))
        lights = {n: g.call("GetSubstAdrsF__5ccAnmFPCcb", (anm, self.cstr(n), 0)) for n in names}
        out, done = [], 0
        for k in steps:
            for _ in range(k - done):
                g.call("_AnimateForward__5ccAnmFUi", (anm, m.load(anm + 0x9C, 2)))
            done = k
            g.call("GetAmbient__5ccAnmFPf", (anm, self.out))
            ambient = self.words(self.out, 3)
            got = {}
            for n, lt in lights.items():
                kind = struct.unpack_from("<h", m.mem, lt + 0x94)[0]
                head = [kind, m.mem[lt + 0x96]]
                if kind == DISTANT:
                    g.call("GetDirc__14ccDistantLightFPf", (lt, self.out))
                    got[n] = head + self.words(lt + 0xC0, 3) + self.words(lt + 0xB0, 1) + self.words(self.out, 3)
                    continue
                m.store(lt + 0x8D, 1, 1)
                g.call("_SetLWMatrix__7ccCoordFv", (lt,))
                if kind == OMNI:
                    got[n] = head + self.words(lt + 0xB0, 3) + self.words(lt + 0xC0, 1) + self.words(lt + 0x30, 3) + \
                        self.words(lt + 0xC4, 3)
                elif kind == DIRECT:
                    got[n] = head + self.words(lt + 0x130, 3) + self.words(lt + 0x140, 1) + self.words(lt, 16) + \
                        self.words(lt + 0x144, 2) + self.words(lt + 0x14C, 3)
                else:
                    got[n] = head + self.words(lt + 0x130, 3) + self.words(lt + 0x140, 1) + self.words(lt, 16) + \
                        self.words(lt + 0x144, 2) + self.words(lt + 0x154, 2)
            out.append((got, ambient))
        return out


def light_anims():
    """Every DATA.BIN file's animations with light records: (stem, anim,
    frames)."""
    import ccs
    import gzarc
    data = gzarc.open_bytes(W.DATA)
    found = []
    for mb in gzarc.members(data):
        try:
            c = ccs.Ccs(gzarc.inflate(data, mb))
        except Exception:
            continue
        for off, t, n, end in c.chunks():
            if t is None:
                break
            if t & 0xFFFF != 0x0700:
                continue
            obj, frames, words = struct.unpack_from("<III", c.data, off + 8)
            p, e = off + 20, off + 20 + 4 * words
            while p < e:
                rk, rn = struct.unpack_from("<II", c.data, p)
                if rk & 0xFFFF in (0x0603, 0x0605, 0x0607, 0x0609):
                    found.append((mb.name.rsplit(".", 1)[0], c.objects[obj][0], frames))
                    break
                p += 8 + 4 * rn
    return found


def run_anims(anims):
    """The port's `animlights` against the game's for each (stem, anim,
    frames): at frame 1, and through the first loop of a keyed one."""
    bad, first, lights = 0, None, 0
    for stem, anim, frames in anims:
        steps = sorted({1, 2, 10, frames // 3, frames // 2, frames - 2} & set(range(1, frames)))
        ports = W.ask(["animlights %s %s %d" % (stem, anim, 256 * k) for k in steps])
        names = [n for n, _ in ports[0]["lights"]]
        games = AnmLights(stem).run(anim, names, steps)
        for k, port, (game, ambient) in zip(steps, ports, games):
            want = {n: v for n, v in port["lights"]}
            lights += len(want)
            d = None
            if game != want:
                d = next(f"{n}: game {game[n]}, port {want[n]}" for n in names if game[n] != want[n])
            elif k == 1 and ambient != port["ambient"] and any(port["ambient"]):
                d = f"ambient: game {ambient}, port {port['ambient']}"
            if d:
                bad += 1
                first = first or f"{stem} {anim} step {k}: {d}"
    return bad, first, lights


# The town, gate, event-room and spot light animations, and a keyed colour.
ANIMS = [("town01", "ANM_sr1town1a", 101), ("town05", "ANM_sr5bac1a", 101), ("chgate", "ANM_xmgtcir1", 91),
         ("chgate", "ANM_xmgtcir3", 193), ("se1_3", "ANM_se1_3_1a", 101), ("se1_6", "ANM_se1_6ob1a", 601)]


READY = os.path.exists(W.ISO) and os.path.exists(S.ELF)


@unittest.skipUnless(READY, "needs the extracted disc and cargo")
class LightGroupAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        W.build()

    def test_random_groups(self):
        bad, first, lit = run(400, 1)
        self.assertEqual(bad, 0, first)
        self.assertGreater(lit, 400)

    def test_distant_goes_after_the_omni_lights(self):
        """A town's table order (the distant light first, then the omni
        lights) with every light in reach: the omni lights take slots 2 and
        1 and 0, the distant light none."""
        omni = [Light(OMNI, 0, placed(UNIT_AXES, [100.0 * k, 0.0, 100.0]),
                      [fbits(0.2 * k), fbits(0.1), fbits(0.1), 0], ONE) for k in range(1, 4)]
        sun = Light(DISTANT, -1, placed(beam_axes([0.0, 0.0, -1.0]), [0.0] * 3), [ONE, ONE, ONE, 0], ONE)
        world = placed(UNIT_AXES, [0.0] * 3)
        game = Lights().run([sun] + omni, [0, 0, 0, 0], world)
        port = port_regs(W.ask([request([sun] + omni, [0, 0, 0, 0], world)])[0])
        self.assertIsNone(compare(game, port))
        self.assertTrue(all(c not in ([0x40000000] * 3, [0, 0, 0]) for c in game[1]), game)

    def test_lights_as_the_animations_leave_them(self):
        """The towns' (a spot light among Lia Fail's), the Chaos Gate's
        keyed omni lights through their loops, an event room's direct
        light and a keyed colour: each light's state as SetAnmCtrlWork
        leaves it under its anm, word for word."""
        bad, first, lights = run_anims(ANIMS)
        self.assertEqual(bad, 0, first)
        self.assertGreater(lights, 50)


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "anims":
        W.build()
        found = light_anims()
        bad, first, lights = run_anims(found)
        print(f"anims {len(found)} animations ({lights} light states), {bad} mismatches"
              + (f"; first: {first}" if first else ""))
    elif len(sys.argv) > 2 and sys.argv[1] == "bulk":
        W.build()
        bad, first, lit = run(int(sys.argv[2]), 7)
        print(f"lights {int(sys.argv[2])} groups ({lit} lit slots), {bad} mismatches"
              + (f"; first: {first}" if first else ""))
    else:
        unittest.main()
