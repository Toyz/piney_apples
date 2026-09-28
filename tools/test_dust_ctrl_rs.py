#!/usr/bin/env python3
"""crates/piney-world's enemy dust controller (combat/dust.rs, `ctrl`)
against the game's own ccEnemyDustCtrl::ctrl (gcmn 0x0043aa90) run in eemu
(the Rust VU0 machine) over test_battle's GameBattle (GCMN.PRG loaded).

A controller is laid out in memory over a table of ccEnemyDustInfo rows -
three races' real tables (eac1DustInfo, e1h1DustInfo, ebl1DustInfo) and a
made-up one for the branches no real row takes (rows of anm 2 or 3, which
match either, and a row whose first and last frames are one) - with its
nodes' last frames 0, and an enemy object beside it. Then 3000 steps: the
enemy's anmNum now and then changed (its frameNum back to 0), frameNum on
by 0-3 (to 129 and round), dispSW mostly on, the flag (a note) one step in
four; ctrl(obj, flag) each step. ccEnemyEffDustRing(p, s, r, n, life, t)
is recorded; ccTransPosFW2LW is a copy.

The dust_probe example runs the port's `ctrl` on the same rows and steps;
each step's rings (n, life, s, r, in order) are compared. The rings'
places and puffs are the particle harness's (test_smoke_and_dust).

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

import eemu  # noqa: E402
import test_anim  # noqa: E402

eemu.Machine = test_anim.machine_class()

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "dust_probe")

# Scratch memory: ccSys and its scratch stack, the made-up rows, the
# controller and its nodes, the enemy.
BASE = 0x01E00000
SYS, SCR, SYN, CTRL, OBJ = BASE, BASE + 0x1000, BASE + 0x2000, BASE + 0x8000, BASE + 0x9000
# The races' tables: (address, rows).
TABLES = ((inf_va(0x005DD6D0), 4), (inf_va(0x005DB2C0), 5), (inf_va(0x005DE270), 3))
ONE = 0x3F800000


def row(anm, n, life, every, first, last, s, r, ofs):
    return bytes([anm, n, life, every]) + struct.pack("<hhII4I", first, last, s, r, *ofs)


# anm 2 and 3 (either matches), one on notes only, one whose first and last
# frames are one, one bucketed.
MADE_UP = [row(2, 4, 10, 3, 5, 40, 0x40400000, 0x42C80000, (0, 0x42480000, 0, ONE)),
           row(3, 5, 12, 0, 0, 60, 0x40000000, 0x41200000, (0, 0, 0, ONE)),
           row(5, 6, 14, 2, 12, 12, 0x40800000, 0x41A00000, (0x42480000, 0, 0, ONE)),
           row(10, 7, 16, 5, 10, 100, 0x40A00000, 0x41F00000, (0, 0, 0x42480000, ONE))]


def lcg(x):
    return (x * 1103515245 + 12345) & 0x7FFFFFFF


class Game:
    def __init__(self):
        import battle
        import test_battle as tb
        self.g = tb.GameBattle(battle.Data(ELF))
        m, sym = self.g.m, self.g.sym
        self.m, self.sym = m, sym
        m.mem[BASE:BASE + 0x10000] = bytes(0x10000)
        m.store(sym("ccSys"), 4, SYS)
        m.store(SYS + 604, 4, SCR)
        m.mem[SYN:SYN + 0x20 * len(MADE_UP)] = b"".join(MADE_UP)
        self.rings = []
        m.hooks[sym("ccEnemyEffDustRing__FPfffiii")] = self.ring
        m.hooks[sym("ccTransPosFW2LW__FPfPf")] = self.copy

    def ring(self, mm, p, n, life, t, *_):
        self.rings.append([n, life, mm.f[12] & 0xFFFFFFFF, mm.f[13] & 0xFFFFFFFF])
        return 0

    @staticmethod
    def copy(mm, o, i, *_):
        mm.mem[o:o + 16] = bytes(mm.mem[i:i + 16])
        return 0

    def rows(self, info, n):
        return bytes(self.m.mem[info:info + 0x20 * n])

    def lay_out(self, info, n):
        """ccEnemyDustCtrl's dsNum, dsTop and nodes (info, obj, last 0,
        next) over the rows."""
        m = self.m
        m.store(CTRL, 4, n)
        nodes = [CTRL + 0x100 + 0x10 * k for k in range(n)]
        m.store(CTRL + 4, 4, nodes[0])
        for k, a in enumerate(nodes):
            m.store(a, 4, info + 0x20 * k)
            m.store(a + 4, 4, OBJ)
            m.store(a + 8, 2, 0)
            m.store(a + 12, 4, nodes[k + 1] if k + 1 < n else 0)

    def ctrl(self, flag, disp, anm, frame):
        m = self.m
        m.store(OBJ + 0xE0, 1, 0x10 if disp else 0)
        m.store(OBJ + 0x256, 2, 116)
        m.store(OBJ + 0x2BC, 2, anm)
        m.store(OBJ + 0x2C0, 2, frame)
        self.rings = []
        m.call(self.sym("ctrl__15ccEnemyDustCtrlFP10ccEntryObji"), (CTRL, OBJ, flag))
        return self.rings


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


@unittest.skipUnless(os.path.exists(ELF) and shutil.which("cargo"), "needs the extracted disc and cargo")
class DustCtrlAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                        "dust_probe"], cwd=ROOT, check=True)
        cls.game = Game()
        cls.probe = Probe()

    @classmethod
    def tearDownClass(cls):
        cls.probe.close()

    def test_steps(self):
        fired = {}
        for info, n in TABLES + ((SYN, len(MADE_UP)),):
            self.game.lay_out(info, n)
            self.probe.ask("rows " + self.game.rows(info, n).hex())
            x, frame, anm = 1, 0, 0
            for step in range(3000):
                x = lcg(x)
                if step == 0 or (x >> 20) % 120 == 0:
                    anm, frame = [0, 2, 3, 4, 6, 7, 10, 5][(x >> 8) % 8], 0
                x = lcg(x)
                frame = (frame + (x >> 8) % 4) % 130
                x = lcg(x)
                flag = int((x >> 8) % 4 == 0)
                x = lcg(x)
                disp = (x >> 8) % 16 != 0
                want = self.game.ctrl(flag, disp, anm, frame)
                rings = self.probe.ask("ctrl %x %x %x" % (flag, anm, frame)) if disp else []
                got = [[r[1], r[2], r[3], r[4]] for r in rings]
                self.assertEqual(want, got, "table %x step %d (anm %d frame %d flag %d)" % (info, step, anm, frame,
                                                                                            flag))
                for r in rings:
                    fired[(info, r[0])] = fired.get((info, r[0]), 0) + 1
        # Every row of every table fired somewhere.
        self.assertEqual(len(fired), sum(n for _, n in TABLES) + len(MADE_UP), fired)


if __name__ == "__main__":
    unittest.main()
