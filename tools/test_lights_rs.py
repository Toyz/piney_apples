#!/usr/bin/env python3
"""piney-world's light group (`chara::light_matrix`, which lights the
towns', fields', dungeons' and event areas' characters) against the game's
own light code in tools/eemu.py.

Each case builds a light group as the game does: every light made by
`ccCreateLight` (main 0x001389b0: priority -1 for a distant light, 0 for
the others; a boss's `ccLight(4, 1)` is an omni light of priority 1; the
event rooms' direct lights, `ccDirectLight::CheckRange` 0x00139330) from a
chunk that gives only its type, its fields then set as the port holds them,
and added in turn by `ccLightGrp::AddGrp` (0x00139060) into a
`ccDrawEnv`'s group. Then `ccDrawEnv::SetLightMatrix` (0x00105900) at a
point: the three slots' directions toward their lights and colours. The
world_probe example answers the same group, in the order the lights were
added, with `light_matrix`.

    python3 tools/test_lights_rs.py           the unit tests
    python3 tools/test_lights_rs.py bulk N    N random groups
"""

import os
import random
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import test_stream_rs as S  # noqa: E402
import test_world_rs as W  # noqa: E402

DISTANT, DIRECT, OMNI = 1, 2, 4


def fbits(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def bitsf(b):
    return struct.unpack("<f", struct.pack("<I", b & 0xFFFFFFFF))[0]


class Lights:
    """The game's light code on one machine."""

    def __init__(self):
        self.g = S.Game()
        self.m = self.g.m

    def put_vec(self, a, v):
        for k, x in enumerate(v):
            self.m.store(a + 4 * k, 4, fbits(x))

    def run(self, lights, at):
        """`lights`: (kind, priority, pos, dir, colour, intensity, start,
        end[, r0, r1]) in the order added. The slots as [dirs 3x3, colours
        3x3] (None for an empty slot)."""
        g, m = self.g, self.m
        env = g.malloc(m, 0x100)
        chunk = g.malloc(m, 0x20)
        for kind, pri, pos, dirv, col, inten, start, end, *rad in lights:
            m.mem[chunk + 12:chunk + 14] = struct.pack("<h", kind)
            lt = g.call("ccCreateLight__FP12ccLightChunk", (chunk,))
            # The priority ccCreateLight gave it, but a boss's own.
            m.store(lt + 0x96, 1, pri & 0xFF)
            # Its matrix as CheckRange last left it (matCalcSW off): an omni
            # light's position is its matrix's translation (+0x30), a
            # distant light's direction +0xd0.
            m.store(lt + 0x8D, 1, 0)
            if kind == DIRECT:
                # Its world matrix (+0x00: the beam's -z along dir, at pos)
                # and that matrix's inverse (+0xf0), as CheckRange keeps
                # them; +0x130 colour, +0x140 intensity, +0x144 and +0x148
                # the beam's start and end, +0x14c and +0x150 the radii,
                # +0x154 the second squared.
                ez = [-x for x in dirv]
                ex = cross([0.0, 1.0, 0.0] if abs(ez[1]) < 0.9 else [1.0, 0.0, 0.0], ez)
                ex = norm(ex)
                ey = cross(ez, ex)
                for c, v in enumerate((ex, ey, ez)):
                    self.put_vec(lt + 16 * c, v + [0.0])
                self.put_vec(lt + 0x30, list(pos) + [1.0])
                for c, v in enumerate((ex, ey, ez)):
                    self.put_vec(lt + 0xF0 + 16 * c, [ex[c], ey[c], ez[c], 0.0])
                self.put_vec(lt + 0x120, [-sum(v[k] * pos[k] for k in range(3)) for v in (ex, ey, ez)] + [1.0])
                self.put_vec(lt + 0x130, list(col) + [0.0])
                r0, r1 = rad
                for off, v in ((0x140, inten), (0x144, start), (0x148, end), (0x14C, r0), (0x150, r1),
                               (0x154, r1 * r1)):
                    m.store(lt + off, 4, fbits(v))
            elif kind == DISTANT:
                self.put_vec(lt + 0xD0, list(dirv) + [0.0])
                self.put_vec(lt + 0xC0, list(col) + [0.0])
                m.store(lt + 0xB0, 4, fbits(inten))
            else:
                self.put_vec(lt + 0x30, list(pos) + [1.0])
                self.put_vec(lt + 0xB0, list(col) + [0.0])
                m.store(lt + 0xC0, 4, fbits(inten))
                m.store(lt + 0xC4, 4, fbits(start))
                m.store(lt + 0xC8, 4, fbits(end))
                m.store(lt + 0xCC, 4, fbits(end * end))
            g.call("AddGrp__10ccLightGrpFP7ccLight", (env + 0x80, lt))
        ln, lc, p = g.malloc(m, 64), g.malloc(m, 64), g.malloc(m, 16)
        self.put_vec(p, list(at) + [1.0])
        g.call("SetLightMatrix__9ccDrawEnvFPA4_fPA4_fPf", (env, ln, lc, p))
        nv = [g.f32(ln + 4 * k) for k in range(16)]
        cv = [g.f32(lc + 4 * k) for k in range(16)]
        out = []
        for slot in range(3):
            col = cv[4 * slot:4 * slot + 3]
            toward = [nv[4 * c + slot] for c in range(3)]
            out.append(None if col == [0.0, 0.0, 0.0] else (toward, col))
        return out


def request(lights, at):
    words = ["lights", "%x" % len(lights)]
    for kind, pri, pos, dirv, col, inten, start, end, *rad in lights:
        words += ["%x" % kind, "%x" % (pri & 0xFFFFFFFF)]
        words += ["%x" % fbits(x) for x in list(pos) + list(dirv) + list(col) + [inten, start, end] + (rad or [0, 0])]
    words += ["%x" % fbits(x) for x in at]
    return " ".join(words)


def port_slots(answer):
    d = [bitsf(b) for b in answer["dirs"]]
    c = [bitsf(b) for b in answer["colours"]]
    out = []
    for slot in range(3):
        col = c[3 * slot:3 * slot + 3]
        out.append(None if col == [0.0, 0.0, 0.0] else (d[3 * slot:3 * slot + 3], col))
    return out


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def norm(v):
    n = sum(x * x for x in v) ** 0.5 or 1.0
    return [x / n for x in v]


def unit(rnd):
    v = [rnd.uniform(-1, 1) for _ in range(3)]
    n = sum(x * x for x in v) ** 0.5 or 1.0
    return [x / n for x in v]


def random_group(rnd):
    """A group as the areas hold them: a distant light or two among up to
    eight omni lights, now and then a boss's, some out of reach or dark."""
    lights = []
    at = [rnd.uniform(-2000, 2000), rnd.uniform(-2000, 2000), rnd.uniform(0, 300)]
    for _ in range(rnd.randint(1, 9)):
        r = rnd.random()
        if r < 0.2:
            # A beam from above near the point: straight down, as the event
            # rooms' are, or any way; wide or with the shrine's thin edge.
            down = rnd.random() < 0.5
            dirv = [0.0, 0.0, -1.0] if down else unit(rnd)
            pos = [at[0] + rnd.uniform(-1500, 1500), at[1] + rnd.uniform(-1500, 1500), rnd.uniform(500, 3000)]
            r0 = rnd.uniform(0, 1500)
            r1 = r0 + (rnd.choice((2.0, rnd.uniform(1, 50))) if down else rnd.uniform(100, 1000))
            col = [rnd.random() for _ in range(3)]
            inten = rnd.choice((1.0, rnd.random(), 0.005))
            start = rnd.choice((0.0, rnd.uniform(0, 3000)))
            end = 0.0 if start == 0.0 and rnd.random() < 0.5 else start + rnd.uniform(1, 3000)
            lights.append((DIRECT, 0, pos, dirv, col, inten, start, end, r0, r1))
            continue
        kind, pri = (DISTANT, -1) if r < 0.4 else (OMNI, 1) if r < 0.5 else (OMNI, 0)
        pos = [rnd.uniform(-3000, 3000), rnd.uniform(-3000, 3000), rnd.uniform(-200, 1500)]
        col = [rnd.choice((0.0, rnd.random())) for _ in range(3)]
        if col == [0.0, 0.0, 0.0]:
            col[0] = 0.5
        inten = rnd.choice((1.0, rnd.random(), 0.005))
        start = rnd.choice((0.0, rnd.uniform(0, 2000)))
        end = 0.0 if start == 0.0 and rnd.random() < 0.5 else start + rnd.uniform(1, 3000)
        lights.append((kind, pri, pos, unit(rnd), col, inten, start, end))
    return lights, at


def compare(game, port):
    """The first difference, or None: the same slots filled, each slot's
    direction within 1e-4 and colour within 1e-5."""
    for slot, (a, b) in enumerate(zip(game, port)):
        if (a is None) != (b is None):
            return f"slot {slot}: game {a}, port {b}"
        if a is None:
            continue
        if any(abs(x - y) > 1e-4 for x, y in zip(a[0], b[0])):
            return f"slot {slot} direction: game {a[0]}, port {b[0]}"
        if any(abs(x - y) > 1e-5 for x, y in zip(a[1], b[1])):
            return f"slot {slot} colour: game {a[1]}, port {b[1]}"
    return None


def run(n, seed):
    rnd = random.Random(seed)
    cases = [random_group(rnd) for _ in range(n)]
    lab = Lights()
    games = [lab.run(ls, at) for ls, at in cases]
    ports = W.ask([request(ls, at) for ls, at in cases])
    bad, first = 0, None
    for (ls, at), game, answer in zip(cases, games, ports):
        d = compare(game, port_slots(answer))
        if d:
            bad += 1
            if first is None:
                first = f"{d}\nlights {ls}\nat {at}"
    return bad, first


READY = os.path.exists(W.ISO) and os.path.exists(S.ELF)


@unittest.skipUnless(READY, "needs the extracted disc and cargo")
class LightGroupAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        W.build()

    def test_random_groups(self):
        bad, first = run(400, 1)
        self.assertEqual(bad, 0, first)

    def test_distant_goes_after_the_omni_lights(self):
        """A town's table order (the distant light first, then the omni
        lights) with every light in reach: the omni lights take slots 2 and
        1 and 0, the distant light none."""
        omni = [(OMNI, 0, [100.0 * k, 0.0, 100.0], [0.0, 0.0, 0.0], [0.2 * k, 0.1, 0.1], 1.0, 0.0, 0.0)
                for k in range(1, 4)]
        lights = [(DISTANT, -1, [0.0] * 3, [0.0, 0.0, -1.0], [1.0, 1.0, 1.0], 1.0, 0.0, 0.0)] + omni
        game = Lights().run(lights, [0.0, 0.0, 0.0])
        port = port_slots(W.ask([request(lights, [0.0, 0.0, 0.0])])[0])
        self.assertIsNone(compare(game, port))
        self.assertTrue(all(s is not None and s[1] != [1.0, 1.0, 1.0] for s in game), game)


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        W.build()
        bad, first = run(int(sys.argv[2]), 7)
        print(f"lights {int(sys.argv[2])} groups, {bad} mismatches" + (f"; first: {first}" if first else ""))
    else:
        unittest.main()
