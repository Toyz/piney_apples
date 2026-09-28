#!/usr/bin/env python3
"""crates/piney-world's shop camera (Camera::set_merchant, camera.rs) against
the game's own ccMenuCtrl::SetMerchantCamera (gcmn 0x005269d0) and
changeCamera (main 0x001617f0), run in eemu over the field as
tools/test_evcam_rs.py's Game sets it up (test_world_rs's Field: Mac Anu's
collision mesh, Kite arriving, then ccThCamera itself for the camera's
set-up, tcam copied to ecam and bcam).

For each character spoken to, the game's cmndTarget is a ccChar built in
its memory (base->type, pos at +0x40, dirc at +0x60) and ccGame.server is
set; the merchcam_probe example gets the same through set_merchant. The
characters: Mac Anu's five merchants as ccSetMerchant(0) places them (the
probe's `merchants`), the Grunt Shops' breeders (type 0x8000000) on
servers 1-4 (breederCamPos / breederCamView), and random characters whose
headings take every branch of the wrap to -pi..pi. Kite stands before the
character; then, compared bit for bit:

  - after SetMerchantCamera: camID, tcam and ecam (all but cptr);
  - frames of cameraMain and ccPlayer::Main (the field's camera and player
    tasks) with camera 3 active, idle and with the sticks and zoom
    buttons: camID, tcam, ecam, and cameraSet's world_view and
    world_screen, now through ecam;
  - changeCamera(1) as the shop closes, then more frames: tcam, which kept
    following Kite, drawn again;
  - cameraGetRot(out, 3) on its own, out holding random words (its y and
    w are out's), over the ecams above and random ones.

SetMerchantCamera works in the ccSys scratch (+0x25c): for a breeder, rot's
y and w are what the scratch held at +0x24 and +0x2c. The scratch is
cleared before each call and the port takes zero.

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

import test_evcam_rs as tev  # noqa: E402
import test_world_rs as tw  # noqa: E402
from test_world_rs import ELF, ISO, ONE, ROOT, fb  # noqa: E402

EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "merchcam_probe")

TCAM, ECAM, CAMID = inf_va(0x00383CE0), inf_va(0x00383DC0), inf_va(0x0037897C)
BREEDER = 0x08000000


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                    "merchcam_probe"], cwd=ROOT, check=True)


class Probe:
    def __init__(self):
        self.p = subprocess.Popen([EXAMPLE, ISO], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
                                  cwd=ROOT)

    def ask(self, *words):
        self.p.stdin.write(" ".join("%x" % (w & 0xFFFFFFFF) if isinstance(w, int) else w for w in words) + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        self.p.wait()


class Game(tev.Game):
    def cameras(self):
        m = self.m

        def cam(a):
            return {"pos": self.rvec(a), "view": self.rvec(a + 16), "rot": self.rvec(a + 32)}
        st = self.state()
        return {"cam_id": m.load(CAMID, 2, True), "tcam": cam(TCAM), "ecam": cam(ECAM),
                "world_view": st["world_view"], "world_screen": st["world_screen"]}

    def talk_to(self, flags, pos, dirc, server):
        """cmndTarget a character of type `flags` at pos, heading dirc;
        ccGame.server; then SetMerchantCamera over a cleared scratch."""
        m = self.m
        c = self.malloc(m, 0x100)
        base = self.malloc(m, 0x20)
        m.store(c, 4, base)
        m.store(base + 8, 4, flags)
        self.vec(c + 0x40, pos)
        self.vec(c + 0x60, dirc)
        m.store(tw.CMNDTARGET, 4, c)
        m.store(tw.GAME + 0x1C, 4, server)
        m.mem[tw.SCRATCH:tw.SCRATCH + 0x100] = bytes(0x100)
        self.call("SetMerchantCamera__10ccMenuCtrlFv", 0)
        # cameraSet's view as the next frame's player task gives it.
        return self.cameras()


def front(pos, dircz, d=300.0):
    """A point `d` in front of a character at pos facing dircz (ccGetDirc's
    heading: 0 faces -y)."""
    x, y = struct.unpack("<ff", struct.pack("<II", pos[0], pos[1]))
    a = struct.unpack("<f", struct.pack("<I", dircz))[0]
    return fb(x + d * math.sin(a)), fb(y - d * math.cos(a)), pos[2]


def pad_words(pad):
    direct, push, pl, dl, pr, dr, pw = pad
    return [direct, push, pl, dl, pr, dr] + list(pw)


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class MerchantCameraAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()
        cls.g = Game()
        cls.probe = Probe()
        cls.merchants = cls.probe.ask("merchants")

    @classmethod
    def tearDownClass(cls):
        cls.probe.close()

    def same(self, want, got, what):
        for k in want:
            self.assertEqual(want[k], got[k], f"{what}: {k}")

    def begin(self, kite, dircz, scheme=0, mode=3, seed=1):
        self.g.begin(kite, dircz, scheme, mode, seed, {}, {})
        self.probe.ask("start", 0, kite[0], kite[1], kite[2], dircz, scheme, mode, seed)

    def frame(self, pad):
        self.g.pad(*pad)
        self.g.frame()
        return self.g.cameras(), self.probe.ask("frame", *pad_words(pad))

    def run_case(self, label, flags, pos, dirc, server, rng, frames=8):
        self.begin(front(pos, dirc[2]), fb(rng.uniform(-math.pi, math.pi)))
        idle = (0, 0, 0, 0, 0, 0, [0] * 12)
        for f in range(2):
            want, got = self.frame(idle)
            self.same(want, got, f"{label}: frame {f} before")
        want = self.g.talk_to(flags, pos, dirc, server)
        got = self.probe.ask("merchant", int(bool(flags & BREEDER)), server, *pos, *dirc)
        for k in ("cam_id", "tcam", "ecam"):
            self.assertEqual(want[k], got[k], f"{label}: SetMerchantCamera {k}")
        self.assertEqual(want["cam_id"], 3)
        pads = [idle] * 3 + [walk_pad(rng) for _ in range(frames)]
        for f, pad in enumerate(pads):
            want, got = self.frame(pad)
            self.same(want, got, f"{label}: frame {f} on camera 3")
        # cameraGetRot on its own, out holding random words.
        for _ in range(4):
            out = [rng.getrandbits(32) for _ in range(4)]
            self.g.vec(tw.ARGS, out)
            self.g.call("cameraGetRot__FPfi", tw.ARGS, 3)
            self.assertEqual(self.g.rvec(tw.ARGS), self.probe.ask("getrot", 3, *out), f"{label}: cameraGetRot")
        self.g.call("changeCamera__Fi", 1)
        got = self.probe.ask("change", 1)
        self.assertEqual(self.g.m.load(CAMID, 2, True), got["cam_id"])
        for f, pad in enumerate([idle] * 2 + [walk_pad(rng) for _ in range(frames)]):
            want, got = self.frame(pad)
            self.same(want, got, f"{label}: frame {f} back on camera 1")
        return want

    def test_mac_anu_merchants(self):
        rng = random.Random(1)
        self.assertEqual([m["id"] for m in self.merchants], [0, 1, 2, 3, 4])
        for m in self.merchants:
            self.assertFalse(m["flags"] & BREEDER)
            self.run_case(f"merchant {m['id']}", m["flags"], m["pos"], m["dirc"], 0, rng)

    def test_breeders(self):
        rng = random.Random(2)
        pos = [fb(-2400.0), fb(2450.0), 0, ONE]
        for server in (1, 2, 3, 4):
            self.run_case(f"breeder, server {server}", BREEDER, pos, [0x80000000, 0, 0, 0], server, rng, frames=4)

    def test_random_characters(self):
        rng = random.Random(3)
        for i in range(24):
            pos = [fb(rng.uniform(-3000, 3000)), fb(rng.uniform(-3000, 3000)), fb(rng.choice([0.0, 300.0])), ONE]
            # Headings on each side of the wrap: r.z = z - 1.3708 below -pi,
            # above pi, and between.
            z = rng.choice([rng.uniform(-3.2, -1.78), rng.uniform(4.52, 6.4), rng.uniform(-1.7, 4.4),
                            rng.uniform(-math.pi, math.pi)])
            dirc = [fb(rng.choice([0.0, rng.uniform(-0.3, 0.3)])), fb(rng.choice([0.0, rng.uniform(-0.5, 0.5)])),
                    fb(z), 0]
            self.run_case(f"character {i}", rng.choice([0x100, 0x800, 0x08, 0x10]), pos, dirc, 0, rng, frames=3)


def walk_pad(rng):
    """The sticks and the zoom and camera buttons (no L2: the eye view is
    tests/test_world_rs's)."""
    direct = rng.choice([0, 0, 0x8, 0x2, 0x4, 0x1000, 0x4000])
    dl, pl = tev.stick(rng.randrange(256), rng.randrange(256)) if rng.random() < 0.6 else (0, 0)
    dr, pr = tev.stick(rng.randrange(256), rng.randrange(256)) if rng.random() < 0.5 else (0, 0)
    pw = [0] * 12
    return (direct & ~1, 0, pl, dl, pr, dr, pw)


if __name__ == "__main__":
    unittest.main()
