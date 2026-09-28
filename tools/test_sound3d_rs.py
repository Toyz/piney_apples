#!/usr/bin/env python3
"""crates/piney-audio's positioned sound effects (src/se3d.rs) against the
game's own sndlib.cpp run in tools/eemu.py (the Rust eemu_rs when it is
built into tools/, as test_anim.machine_class picks).

The game side runs the functions natively with gcmn.prg loaded over the
executable (setbl.cpp's tables are there): seData, calcVel, calcPan,
sqrtf, atan2f, sinf, RAD2DEG, DEG2RAD, fptosi, checkCameraType, all the
game's; activeCamPtr points at a CAMERA written for the case (or is NULL),
ccSnd at a ccSound its constructor made. What leaves the EE is caught at
sceMSIn_PutMsg (2 bytes of a program change) and sceMSIn_PutHsMsg (5 bytes
of an F9 message, 7 of an FD one), and every message is checked to go to
port 0. The same cases go to the se3d_probe example, and everything is
compared exactly:

  vel      calcVel (0x0017a370): the return value
  pan      calcPan (0x0017a4c0): the return value (pan << 16 | cdeg)
  on       ccSeOn3D (0x00179d90): the bytes sent
  note     ccSeOn3DNote (0x00179f50), notes -128..127: the bytes sent
  loop     runs of ccSeOn3DLoop (0x0017a140) and ccSeOffLoop (0x0017a620)
           on one ccSound: the id returned, the bytes sent and
           ccSound.loopID[8] (+0x65) after each call
  spc      ccSeSetParamSPC (0x0017aa20), with ccSeOnPCStep and seHitAttr:
           the bytes sent, for a ccChar whose base's id, pos and
           hitAttribute the case sets
  pc       ccSeSetParamPC (0x0017aaf0), every ccstype
  enemy    ccSeSetParamEnemy (0x0017abd0), every category
  inu      ccSeSetParamInu (0x0017ac70)

Cases: every sound effect of seData; cameras at random places looking in
random directions (a view point at the eye's x and y too), of type 0, 1
and 2, or none; points near and far (past the rows' reach), in front,
beside and behind, on the eye's x line (the atan2f of 0 that the eye view
tests) and at the eye itself. A note's param reaches past its table's end
into the next table, the padding and inuSeData, as far as rows that name a
sound effect; hit attributes are the grounds seHitAttr knows with other
bits set, and random words.

    python3 tools/test_sound3d_rs.py              the tests, 1,500 cases a check
    python3 tools/test_sound3d_rs.py bulk N       N cases a check

Skipped when the disc is not extracted or cargo is missing. CARGO_TARGET_DIR
is honoured when locating the probe.
"""

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

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
GCMN = os.path.join(ROOT, "work", "infection", "disc", "DATA", "GCMN.PRG")
TARGET = os.environ.get("CARGO_TARGET_DIR") or os.path.join(ROOT, "target")
EXAMPLE = os.path.join(TARGET, "release", "examples", "se3d_probe")

ACTIVE_CAM = inf_va(0x0037896C)     # CAMERA *activeCamPtr
CCSND = inf_va(0x00378A08)          # ccSound *ccSnd
LOOPTEST = inf_va(0x003789DC)       # int looptest: TOBJ's hum's slot
SND = 0x01800000            # the ccSound (0x150 bytes)
CAM = 0x01810000            # the CAMERA (0x70 bytes): pos +0, view +0x10, type +0x5c
POS = 0x01820000            # the point
CH = 0x01830000             # a ccChar: base +0, pos +0x40, hitAttribute +0x80
BP = 0x01831000             # its ccCharBaseParam: id +0xc
LOOP_ID = SND + 0x65
N_SE = 237
DEFAULT_N = 1500

# The grounds seHitAttr (0x0017a7f0) gives a row, as hitAttribute & 0x00f0f0f0.
GROUNDS = (0x008080F0, 0x00B06010, 0x00C0D000, inf_va(0x00304050), inf_va(0x00405060), 0x20F0, 0x0090B0C0, 0x2040, 0x3060,
           0x5030, 0x6040, 0x4080, inf_va(0x0060B0D0), inf_va(0x0070C0E0), 0x00C0C0C0, 0x00D0D0D0, 0x00E0E0E0, inf_va(0x00404040),
           inf_va(0x00505050), inf_va(0x00606060), 0xC000)
MASK = 0x00F0F0F0


def fbits(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v >> 31 else v


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-audio", "--example",
                    "se3d_probe"], cwd=ROOT, check=True)


def ask(lines):
    p = subprocess.run([EXAMPLE], input="\n".join(lines) + "\n", capture_output=True, text=True, cwd=ROOT)
    out = p.stdout.splitlines()
    if p.returncode:
        raise RuntimeError(f"se3d_probe failed on request {len(out)}: {lines[len(out)]}\n{p.stderr[-2000:]}")
    return out


# the game ---------------------------------------------------------------------------

class Game:
    def __init__(self):
        from image import Program
        from test_anim import machine_class
        self.p = Program(ELF, "gcmn")
        self.m = machine_class()(self.p)
        m = self.m
        self.sym = lambda n: self.p.symbol_named(n).value
        self.se_data = self.sym("seData")
        m.mem[SND:SND + 0x150] = bytes(0x150)
        m.store(CCSND, 4, SND)
        m.call(self.sym("__ct__7ccSoundFv"), [SND])
        self.sent = []
        self.ports = set()
        m.hooks[self.sym("sceMSIn_PutMsg")] = self._put_msg
        m.hooks[self.sym("sceMSIn_PutHsMsg")] = self._put_hs
        m.store(BP + 0xC, 2, 0)
        m.store(CH, 4, BP)

    def _put_msg(self, m, ctx, port, msg, *a):
        self.ports.add(port & 0xFFFFFFFF)
        n = 2 if msg & 0xF0 in (0x80, 0xC0, 0xD0) else 3
        self.sent += [(msg >> (8 * i)) & 0xFF for i in range(n)]
        return 0

    def _put_hs(self, m, ctx, port, ptr, *a):
        self.ports.add(port & 0xFFFFFFFF)
        n = {0xF9: 5, 0xFD: 7}.get(m.load(ptr, 1), 0)
        self.sent += list(m.mem[ptr:ptr + n])
        return 0

    def camera(self, cam):
        m = self.m
        if cam is None:
            m.store(ACTIVE_CAM, 4, 0)
            return
        (cx, cy, cz, vx, vy, vz, kind) = cam
        m.mem[CAM:CAM + 0x70] = bytes(0x70)
        for i, v in enumerate((cx, cy, cz, 0x3F800000, vx, vy, vz, 0x3F800000)):
            m.store(CAM + 4 * i, 4, v)
        m.store(CAM + 0x5C, 4, kind & 0xFFFFFFFF)
        m.store(ACTIVE_CAM, 4, CAM)

    def point(self, addr, pos):
        for i, v in enumerate(pos + (0x3F800000,)):
            self.m.store(addr + 4 * i, 4, v)

    def run(self, name, *args):
        self.sent = []
        r = self.m.call(self.sym(name), [a & 0xFFFFFFFF for a in args])
        if self.ports - {0}:
            raise AssertionError(f"{name} sent to port {self.ports}")
        return r

    def bytes_sent(self):
        return bytes(self.sent).hex() if self.sent else "-"

    def loop_ids(self):
        return ",".join(str(struct.unpack("b", bytes([self.m.load(LOOP_ID + i, 1)]))[0]) for i in range(8))


# the cases -------------------------------------------------------------------------

def cam_str(cam):
    if cam is None:
        return "-"
    return ",".join(f"{v:x}" for v in cam[:6]) + f",{cam[6]}"


def pos_str(pos):
    return " ".join(f"{v:x}" for v in pos)


def rnd_cam(rng):
    if rng.random() < 0.08:
        return None
    cx, cy, cz = rng.uniform(-4000, 4000), rng.uniform(-4000, 4000), rng.uniform(-300, 600)
    r = rng.choice((0.0, rng.uniform(0.5, 30), rng.uniform(30, 600)))
    a = rng.uniform(-3.2, 3.2)
    if rng.random() < 0.15:
        vx, vy = cx, cy                     # looking straight down: atan2f(0, 0)
    elif rng.random() < 0.1:
        vx, vy = cx + r, cy                 # along the x axis
    else:
        vx, vy = cx + r * math.cos(a), cy + r * math.sin(a)
    vz = cz + rng.uniform(-300, 300)
    kind = rng.choice((0, 1, 1, 2))
    return tuple(fbits(v) for v in (cx, cy, cz, vx, vy, vz)) + (kind,)


def rnd_pos(rng, cam):
    """A point near or far from the camera, in any direction; on its x line
    now and then, or at the eye."""
    if cam is None:
        c = (rng.uniform(-4000, 4000), rng.uniform(-4000, 4000), 0.0)
    else:
        c = tuple(struct.unpack("<f", struct.pack("<I", v))[0] for v in cam[:3])
    k = rng.random()
    if k < 0.05:
        return tuple(fbits(v) for v in c)
    # The rows' reach is 1,375 to 5,500 (decay 64 to 256); calcVel falls
    # below 127 within 1,000 to 2,700 of it, ccSeOn3D below the row's
    # velocity within 1,000.
    d = rng.choice((rng.uniform(0, 60), rng.uniform(0, 1500), rng.uniform(0, 6000), rng.uniform(1000, 5600),
                    rng.uniform(1000, 5600), rng.uniform(4000, 9000), rng.uniform(0, 200000)))
    a = rng.uniform(-math.pi, math.pi)
    x, y = c[0] + d * math.cos(a), c[1] + d * math.sin(a)
    if k < 0.15:
        y = c[1]                            # dy exactly 0: atan2f of 0 or pi
    z = c[2] + rng.uniform(-400, 400)
    return (fbits(x), fbits(y), fbits(z))


def rnd_hit(rng):
    if rng.random() < 0.7:
        return rng.choice(GROUNDS) | (rng.getrandbits(32) & ~MASK & 0xFFFFFFFF)
    return rng.getrandbits(32)


class Setbl:
    """setbl.cpp's rows and tables, as the port keeps them
    (tools/portdata.py; piney-gen's placement::sound::setbl)."""

    def __init__(self):
        import portdata
        _, self.rows, self.spc, self.enemy, self.inu = portdata.setbl("INF")

    def params(self, start, rng):
        """A param whose row is in setbl.cpp's data and names a sound effect
        or none (-1) - not one of the pointer tables' words."""
        while True:
            span = len(self.rows) - start
            p = rng.randrange(min(span, 30)) if rng.random() < 0.8 else rng.randrange(span)
            if self.rows[start + p][0] < N_SE:
                return p


def cases(check, n, seed=1):
    """(probe request, game call) pairs for one check."""
    rng = random.Random(f"{check}-{seed}")
    tb = Setbl() if check in ("spc", "enemy", "inu") else None
    out = []
    for i in range(n):
        cam = rnd_cam(rng)
        pos = rnd_pos(rng, cam)
        se = i % N_SE if i < 2 * N_SE else rng.randrange(N_SE)
        c, p = cam_str(cam), pos_str(pos)
        if check == "vel":
            out.append((f"vel {se} {c} {p}", ("vel", cam, pos, se)))
        elif check == "pan":
            out.append((f"pan {c} {p}", ("pan", cam, pos)))
        elif check == "on":
            out.append((f"on {se} {c} {p}", ("on", cam, pos, se)))
        elif check == "note":
            k = rng.randrange(128) if rng.random() < 0.75 else rng.randrange(-128, 0)
            out.append((f"note {se} {k} {c} {p}", ("note", cam, pos, se, k)))
        elif check == "spc":
            cid = rng.randrange(19)
            start = tb.spc[cid]
            param = 0 if start is None or rng.random() < 0.3 else tb.params(start, rng)
            hit = rnd_hit(rng)
            out.append((f"spc {param} {cid} {hit} {c} {p}", ("spc", cam, pos, param, cid, hit)))
        elif check == "pc":
            param = rng.choice((0, 0, 0, 1, rng.getrandbits(32)))
            ccstype = rng.randrange(-2, 5)
            hit = rnd_hit(rng)
            out.append((f"pc {param} {ccstype} {hit} {c} {p}", ("pc", cam, pos, param, ccstype, hit)))
        elif check == "enemy":
            cat = rng.randrange(19)
            param = tb.params(tb.enemy[cat], rng)
            out.append((f"enemy {param} {cat} {c} {p}", ("enemy", cam, pos, param, cat)))
        elif check == "inu":
            param = rng.randrange(len(tb.rows) - tb.inu)
            hit = rnd_hit(rng)
            out.append((f"inu {param} {hit} {c} {p}", ("inu", cam, pos, param, hit)))
    return out


def game_answer(g, call):
    kind = call[0]
    if kind == "reset":
        g.m.mem[LOOP_ID:LOOP_ID + 8] = b"\xff" * 8
        return g.loop_ids()
    if kind == "treset":
        g.m.mem[LOOP_ID:LOOP_ID + 8] = b"\xff" * 8
        g.m.store(SND + 0x133, 1, 0)
        g.m.store(LOOPTEST, 4, 0)
        return g.loop_ids()
    if kind == "tstart":
        g.run("tobjSeLoopStart__FPf", POS)
        return f"{g.bytes_sent()} {g.loop_ids()} {g.m.load(SND + 0x133, 1)} {s32(g.m.load(LOOPTEST, 4))}"
    if kind == "tloop":
        _, cam, pos, rate = call
        g.camera(cam)
        g.point(POS, pos)
        g.m.f[12] = rate
        g.run("tobjSeLoop__FPff", POS)
        return g.bytes_sent()
    if kind == "off":
        _, se, lid = call
        g.run("ccSeOffLoop__Fii", se, lid)
        return f"{g.bytes_sent()} {g.loop_ids()}"
    _, cam, pos = call[:3]
    g.camera(cam)
    g.point(POS, pos)
    if kind == "vel":
        return str(s32(g.run("calcVel__FPfP5SETBL", POS, g.se_data + 8 * call[3])))
    if kind == "pan":
        r = s32(g.run("calcPan__FPf", POS))
        return f"{r >> 16} {r & 0xFFFF}"
    if kind == "on":
        g.run("ccSeOn3D__FiPf", call[3], POS)
        return g.bytes_sent()
    if kind == "note":
        g.run("ccSeOn3DNote__FiPfc", call[3], POS, call[4])
        return g.bytes_sent()
    if kind == "loop":
        r = s32(g.run("ccSeOn3DLoop__FiPf", call[3], POS))
        return f"{r} {g.bytes_sent()} {g.loop_ids()}"
    g.point(CH + 0x40, pos)
    if kind == "spc":
        _, _, _, param, cid, hit = call
        g.m.store(BP + 0xC, 2, cid)
        g.m.store(CH + 0x80, 4, hit)
        g.run("ccSeSetParamSPC__FUiP6ccChar", param, CH)
    elif kind == "pc":
        _, _, _, param, ccstype, hit = call
        g.m.store(CH + 0x80, 4, hit)
        g.run("ccSeSetParamPC__FUiP6ccChari", param, CH, ccstype)
    elif kind == "enemy":
        _, _, _, param, cat = call
        g.run("ccSeSetParamEnemy__FUiP6ccChari", param, CH, cat)
    elif kind == "inu":
        _, _, _, param, hit = call
        g.m.store(CH + 0x80, 4, hit)
        g.run("ccSeSetParamInu__FUiP6ccChar", param, CH)
    return g.bytes_sent()


def compare(g, pairs, bytes_only=False):
    """Run the game on each case and the probe on all; the mismatches. For
    the notes, the probe's CODE and NOTE are dropped (the game shows only
    what it sends)."""
    got = ask([r for r, _ in pairs])
    bad = []
    for (req, call), port in zip(pairs, got):
        want = game_answer(g, call)
        if bytes_only:
            port = port.split()[-1]
        if want != port:
            bad.append(f"{req}\n    game {want}\n    port {port}")
    return bad


def run_loops(g, n, seed=1):
    """Runs of ccSeOn3DLoop and ccSeOffLoop on one ccSound, `n` calls in
    all, each run from free slots: loops at random points, the ends of
    loops the game started (by the id it returned) and of random slots. The
    game answers as the run goes; the probe replays the same calls."""
    rng = random.Random(f"loop-{seed}")
    reqs, wants = [], []
    while len(reqs) < n:
        reqs.append("reset")
        wants.append(game_answer(g, ("reset",)))
        taken = []
        for _ in range(rng.randrange(4, 24)):
            if taken and rng.random() < 0.35:
                se, lid = rng.choice(taken)
                if rng.random() < 0.8:
                    taken.remove((se, lid))
                call, req = ("off", se, lid), f"off {se} {lid}"
            elif rng.random() < 0.1:
                se, lid = rng.randrange(N_SE), rng.randrange(8)
                call, req = ("off", se, lid), f"off {se} {lid}"
            else:
                cam = rnd_cam(rng)
                pos = rnd_pos(rng, cam)
                se = rng.randrange(N_SE)
                call, req = ("loop", cam, pos, se), f"loop {se} {cam_str(cam)} {pos_str(pos)}"
            want = game_answer(g, call)
            if call[0] == "loop" and int(want.split()[0]) >= 0:
                taken.append((se, int(want.split()[0])))
            reqs.append(req)
            wants.append(want)
    got = ask(reqs)
    return [f"{r}\n    game {w}\n    port {p}" for r, w, p in zip(reqs, wants, got) if w != p], len(reqs)


def run_tobj(g, n, seed=1):
    """Runs of TOBJ's hum on one ccSound, `n` calls in all: each from free
    slots and the flag clear, the other loops taking slots now and then,
    tobjSeLoopStart (again, once started) and tobjSeLoop at random points
    and transparencies."""
    rng = random.Random(f"tobj-{seed}")
    reqs, wants = [], []
    while len(reqs) < n:
        reqs.append("treset")
        wants.append(game_answer(g, ("treset",)))
        for _ in range(rng.randrange(4, 24)):
            r = rng.random()
            cam = rnd_cam(rng) or (0, 0, 0, fbits(1.0), 0, 0, 0)
            pos = rnd_pos(rng, cam)
            if r < 0.2:
                se = rng.randrange(N_SE)
                call, req = ("loop", cam, pos, se), f"loop {se} {cam_str(cam)} {pos_str(pos)}"
            elif r < 0.35:
                call, req = ("tstart",), "tstart"
            else:
                rate = fbits(rng.choice((1.0, 0.8, rng.randrange(1, 101) / 100, rng.uniform(0, 2))))
                call, req = ("tloop", cam, pos, rate), f"tloop {cam_str(cam)} {pos_str(pos)} {rate:x}"
            reqs.append(req)
            wants.append(game_answer(g, call))
    got = ask(reqs)
    return [f"{r}\n    game {w}\n    port {p}" for r, w, p in zip(reqs, wants, got) if w != p], len(reqs)


HAVE = os.path.exists(ELF) and os.path.exists(GCMN)


@unittest.skipUnless(HAVE and shutil.which("cargo"), "needs the extracted disc and cargo")
class Sound3dAgainstGame(unittest.TestCase):
    n = DEFAULT_N

    @classmethod
    def setUpClass(cls):
        build()
        cls.g = Game()

    def check(self, name):
        pairs = cases(name, self.n)
        bad = compare(self.g, pairs, bytes_only=name in ("spc", "pc", "enemy", "inu"))
        print(f"{name}: {len(pairs)} cases, {len(bad)} mismatches", file=sys.stderr)
        self.assertEqual(bad[:5], [], f"{len(bad)} of {len(pairs)} differ")

    def test_vel(self):
        self.check("vel")

    def test_pan(self):
        self.check("pan")

    def test_on(self):
        self.check("on")

    def test_note(self):
        self.check("note")

    def test_spc(self):
        self.check("spc")

    def test_pc(self):
        self.check("pc")

    def test_enemy(self):
        self.check("enemy")

    def test_inu(self):
        self.check("inu")

    def test_loop(self):
        bad, n = run_loops(self.g, self.n)
        print(f"loop: {n} calls, {len(bad)} mismatches", file=sys.stderr)
        self.assertEqual(bad[:5], [], f"{len(bad)} of {n} differ")


    def test_tobj(self):
        bad, n = run_tobj(self.g, self.n)
        print(f"tobj: {n} calls, {len(bad)} mismatches", file=sys.stderr)
        self.assertEqual(bad[:5], [], f"{len(bad)} of {n} differ")

if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "bulk":
        Sound3dAgainstGame.n = int(sys.argv[2])
        sys.argv[1:] = sys.argv[3:]
    unittest.main()
