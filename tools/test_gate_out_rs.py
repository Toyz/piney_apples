#!/usr/bin/env python3
"""The gate-hacked arrival in crates/piney-world (combat/gate_out.rs) against
the game's own functions run in eemu (the Rust eemu_rs when it is built into
tools/, as test_anim.machine_class picks).

  - ccClearGtHack (main 0x001b77c0) over game.area, areaPrev, game.dungeon
    and WORLD_MAN's dungeonType: whether gtHackFlag stays;
  - ccAddRequestFileListSpc (gcmn 0x005a10e0) with an empty registry, over
    game.area, areaPrev, the field type, game.dungeon, game.field and
    gtHackFlag: gateHackingOutID (and whether it was looked at) and the
    camera file it puts on the list (GateHackOutFileList);
  - ccPlayer::GateHackingOut (gcmn 0x0059bc00) frame by frame on a ccPlayer
    laid out in memory, from step 0 and from random later steps (the
    unreached 2-6 too), with changeCamera, cameraSetPos / cameraSetView,
    cameraParamChange, SetActNum, RAD2DEG / DEG2RAD, atan2f and sqrtf the
    game's. ghoCam's ccAnm is hooked: SetAnm sets it or not, _AnimateForward
    answers the script's "ended", GetSubstAdrsF hands out three markers
    under an identity parent at the script's positions each frame. Compared
    each frame: progCtrlFlag, +0x114, +0x224, speedRate, ghoCamArmsT, the
    blades' dispSW, ghoFlag, cameraFlag, the +0xe0 flags, actNum, pos, +0x250,
    posView, camID, and tcam's and ecam's pos, view, rot, dist and deg.

ccAI::ccAI's arrivalChatCnt is read off the disassembly (gate_out.rs's
arrival_chat); the world_probe's `ghoid` request prints it beside the ID.

Skipped when the disc is not extracted or cargo is missing.
"""

import os
import random
import shutil
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc
import test_world_rs as tw  # noqa: E402

M32 = 0xFFFFFFFF
GAME_P, WORLDMAN_P = inf_va(0x003789CC), inf_va(0x00378A7C)
GHO_FLAG, GHO_CAM, GHO_M, GHO_C, GHO_V, GHO_ARMS = (inf_va(0x00378CC0), inf_va(0x00378CC4), inf_va(0x00378CC8), inf_va(0x00378CCC), inf_va(0x00378CD0),
                                                  inf_va(0x00378CD4))
GT_HACK, GHO_ID = inf_va(0x00378A98), inf_va(0x00378CF8)
CAM_ID, ACTIVE_CAM, CAMERA_LIST = inf_va(0x0037897C), inf_va(0x0037896C), inf_va(0x002FB6F0)
TCAM, ECAM = inf_va(0x00383CE0), inf_va(0x00383DC0)
SPC_MANAGER = inf_va(0x00730340)
# Scratch.
GAME, WM, PL, W1, W2, MK, PARENT, HEAP = (0x01800000, 0x01801000, 0x01802000, 0x01803000, 0x01803100, 0x01803200,
                                          0x01803800, 0x01810000)
ANM = 0x01804000


def f2b(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def s32(v):
    v &= M32
    return v - (1 << 32) if v & 0x80000000 else v


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


class Game:
    def __init__(self):
        from image import Program
        from test_anim import machine_class
        self.p = Program(tw.ELF, "gcmn")
        self.m = m = machine_class()(self.p)
        self.sym = lambda n: self.p.symbol_named(n).value
        self.heap = HEAP
        self.files = []
        self.set = True
        self.ended = False
        sym = self.sym

        def alloc(mm, n, *a):
            a0 = self.heap
            self.heap += (n + 15) & ~15
            m.mem[a0:a0 + n] = bytes(n)
            return a0

        def set_anm(mm, this, chunk, *a):
            m.store(this + 0xAC, 4, 1 if self.set else 0)
            return 0

        def subst(mm, this, name, *a):
            n = self.cstr(name)
            k = {"OBJ_marker_cam1": 0, "OBJ_marker_target0": 1, "OBJ_marker_man0": 2}[n]
            return MK + 0x100 * k

        def add_file(mm, fl, *a):
            self.files.append(self.cstr(m.load(fl + 4, 4)))
            return 0

        m.hooks[sym("__nw__FUi")] = alloc
        m.hooks[sym("__ct__5ccAnmFv")] = lambda mm, this, *a: this
        m.hooks[sym("__dt__5ccAnmFv")] = lambda mm, *a: 0
        m.hooks[sym("GetCCSAdrs__8ccStreamFPCc")] = lambda mm, name, *a: self.files.append(self.cstr(name)) or 1
        m.hooks[sym("GetChunkAdrsF__8ccStreamFPCci")] = lambda mm, s, name, *a: self.files.append(self.cstr(name)) or 1
        m.hooks[sym("SetAnm__5ccAnmFP10ccAnmChunkUi")] = set_anm
        m.hooks[sym("GetSubstAdrsF__5ccAnmFPCcb")] = subst
        m.hooks[sym("_AnimateForward__5ccAnmFUi")] = lambda mm, *a: 1 if self.ended else 0
        m.hooks[sym("ccAddFileListOne__FP10ccFileList")] = add_file
        m.hooks[sym("GetEventAreaInfo__9WORLD_MANFv")] = lambda mm, *a: 0
        m.hooks[sym("ccGetCharParam__Fi")] = lambda mm, *a: 0
        m.hooks[sym("ccAddRequestFileListEquip__Fi")] = lambda mm, *a: 0
        m.store(GAME_P, 4, GAME)
        m.store(WORLDMAN_P, 4, WM)
        # The markers' parent: an identity lwMatrix (+0x40).
        m.mem[PARENT:PARENT + 0x100] = bytes(0x100)
        for i in range(4):
            m.store(PARENT + 0x40 + 20 * i, 4, f2b(1.0))
        for k in range(3):
            m.mem[MK + 0x100 * k:MK + 0x100 * (k + 1)] = bytes(0x100)
            m.store(MK + 0x100 * k + 0x80, 4, PARENT)

    def cstr(self, p):
        out = bytearray()
        while self.m.mem[p + len(out)]:
            out.append(self.m.mem[p + len(out)])
        return out.decode("latin-1")

    # ccClearGtHack ----------------------------------------------------------

    def clear(self, area, prev, dungeon, dtype):
        m = self.m
        m.mem[GAME:GAME + 0x100] = bytes(0x100)
        m.store(GAME + 0x14, 4, area & M32)
        m.store(GAME + 0x18, 4, prev & M32)
        m.store(GAME + 0x28, 4, dungeon & M32)
        m.mem[WM:WM + 0x100] = bytes(0x100)
        m.store(WM + 0x34 + 4 * dungeon, 4, dtype & M32)
        m.store(GT_HACK, 4, 1)
        m.call(self.sym("ccClearGtHack__Fv"), [])
        return m.load(GT_HACK, 4) != 0

    # ccAddRequestFileListSpc --------------------------------------------------

    def file_list(self, area, prev, ftype, dungeon, field, gt):
        m = self.m
        m.mem[GAME:GAME + 0x100] = bytes(0x100)
        for k, v in ((0x14, area), (0x18, prev), (0x24, field), (0x28, dungeon)):
            m.store(GAME + k, 4, v & M32)
        m.mem[WM:WM + 0x100] = bytes(0x100)
        m.store(WM + 0x10, 4, ftype)
        for k in range(5):
            m.store(SPC_MANAGER + 0x2C * k, 4, M32)
        m.store(GT_HACK, 4, int(gt))
        m.store(GHO_ID, 4, 0x55)
        self.files = []
        m.call(self.sym("ccAddRequestFileListSpc__Fv"), [])
        v = s32(m.load(GHO_ID, 4))
        return (None if v == 0x55 else v), list(self.files)

    # GateHackingOut ------------------------------------------------------------

    def gho(self, st, frames):
        m = self.m
        m.mem[PL:PL + 0x300] = bytes(0x300)
        m.store(PL + 0x208, 4, st["prog"] & M32)
        m.store(PL + 0x114, 4, st["cnt"] & M32)
        m.store(PL + 0x224, 4, st["k224"])
        m.store(PL + 0x108, 4, st["speed"])
        m.store(GHO_ARMS, 4, st["arms"])
        m.store(W1 + 0xA2, 1, 3 if st["disp"] else 0)
        m.store(W2 + 0xA2, 1, 3 if st["disp"] else 0)
        m.store(PL + 0x12C, 4, W1)
        m.store(PL + 0x130, 4, W2)
        m.store(GHO_FLAG, 4, int(st["flag"]))
        m.store(PL + 0x200, 1, int(st["camflag"]))
        m.store(PL + 0xE0, 4, st["flags"])
        m.store(PL + 0xEE, 2, st["act"] & 0xFFFF)
        for off, key in ((0x40, "pos"), (0x250, "pcam"), (0x240, "pview")):
            for i in range(4):
                m.store(PL + off + 4 * i, 4, st[key][i])
        m.store(CAM_ID, 2, st["camid"])
        m.store(ACTIVE_CAM, 4, m.load(CAMERA_LIST + 4 * st["camid"], 4))
        for base, c in ((TCAM, st["tcam"]), (ECAM, st["ecam"])):
            m.mem[base:base + 0x70] = bytes(0x70)
            for i in range(4):
                m.store(base + 4 * i, 4, c["pos"][i])
                m.store(base + 0x10 + 4 * i, 4, c["view"][i])
                m.store(base + 0x20 + 4 * i, 4, c["rot"][i])
            m.store(base + 0x54, 4, c["dist"])
            m.store(base + 0x58, 2, c["deg"][0] & 0xFFFF)
            m.store(base + 0x5A, 2, c["deg"][1] & 0xFFFF)
        m.store(GHO_ID, 4, 0)
        self.set = st["set"]
        # A run that starts past step 0 finds ghoCam and its markers as
        # step 0 left them.
        m.mem[ANM:ANM + 0x110] = bytes(0x110)
        m.store(ANM + 0xAC, 4, int(self.set))
        m.store(GHO_CAM, 4, ANM)
        for k, a in enumerate((GHO_C, GHO_V, GHO_M)):
            m.store(a, 4, MK + 0x100 * k)
        out = []
        for fr in frames:
            self.ended = fr["ended"]
            if fr["act"] is not None:
                m.store(PL + 0xEE, 2, fr["act"] & 0xFFFF)
            for k in range(3):
                for i in range(3):
                    m.store(MK + 0x100 * k + 0x70 + 4 * i, 4, fr["mk"][k][i])
                m.store(MK + 0x100 * k + 0x7C, 4, f2b(1.0))
            m.call(self.sym("GateHackingOut__8ccPlayerFv"), [PL])
            ld = lambda a: m.load(a, 4)  # noqa: E731
            v4 = lambda a: [ld(a + 4 * i) for i in range(4)]  # noqa: E731

            def cam(b):
                return [v4(b), v4(b + 0x10), v4(b + 0x20), ld(b + 0x54), s16(m.load(b + 0x58, 2)),
                        s16(m.load(b + 0x5A, 2))]

            disp = m.load(W1 + 0xA2, 1)
            assert disp == m.load(W2 + 0xA2, 1)
            out.append([s32(ld(PL + 0x208)), s32(ld(PL + 0x114)), ld(PL + 0x224), ld(PL + 0x108), ld(GHO_ARMS),
                        int(disp == 3), ld(GHO_FLAG), m.load(PL + 0x200, 1) & 1, ld(PL + 0xE0), s16(m.load(PL + 0xEE, 2)),
                        v4(PL + 0x40), v4(PL + 0x250), v4(PL + 0x240), s16(m.load(CAM_ID, 2)), cam(TCAM), cam(ECAM)])
        return out


def v4(r, lo=-30000.0, hi=30000.0):
    return [f2b(r.uniform(lo, hi)), f2b(r.uniform(lo, hi)), f2b(r.uniform(0, 600)), f2b(1.0)]


def cam(r):
    return {"pos": v4(r), "view": v4(r), "rot": [f2b(r.uniform(-3, 3)) for _ in range(4)], "dist": f2b(r.uniform(100, 900)),
            "deg": [r.randint(-32768, 32767), r.randint(-32768, 32767)]}


def scenario(r):
    start = r.choice([0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 7, 8])
    st = {
        "prog": start,
        "cnt": r.choice([0, r.randint(0, 130), 40, 41, 110, 111]),
        "k224": r.choice([f2b(120.0), f2b(r.uniform(90, 130)), f2b(100.0)]),
        "speed": r.choice([f2b(1.0), f2b(r.uniform(0.2, 1.2)), f2b(0.99)]),
        "arms": r.choice([0, f2b(r.uniform(0, 1.2)), f2b(1.0), f2b(0.9)]),
        "disp": r.random() < 0.5,
        "flag": r.random() < 0.8,
        "camflag": r.random() < 0.5,
        "flags": r.getrandbits(32),
        "act": r.choice([24, 24, 23, 2]),
        "pos": v4(r), "pcam": v4(r), "pview": v4(r),
        "camid": r.choice([1, 3]),
        "tcam": cam(r), "ecam": cam(r),
        "set": r.random() < 0.9,
    }
    n = r.randint(1, 60)
    end_at = r.randint(0, 40)
    act_at = r.randint(0, 50)
    frames = []
    for f in range(n):
        frames.append({"ended": f >= end_at, "act": 23 if f == act_at else None,
                       "mk": [v4(r)[:3] for _ in range(3)]})
    return st, frames


def probe_gho(st, frames):
    v = [st["prog"], st["cnt"], st["k224"], st["speed"], st["arms"], int(st["disp"]), int(st["flag"]),
         int(st["camflag"]), st["flags"], st["act"]] + st["pos"] + st["pcam"] + st["pview"] + [st["camid"]]
    for c in (st["tcam"], st["ecam"]):
        v += c["pos"] + c["view"] + c["rot"] + [c["dist"]] + c["deg"]
    v += [int(st["set"]), len(frames)]
    for fr in frames:
        v += [int(fr["ended"]), M32 if fr["act"] is None else fr["act"]]
        for k in range(3):
            v += fr["mk"][k]
    return "gho " + tw.hexs(*v)


@unittest.skipUnless(os.path.exists(tw.ELF) and os.path.exists(tw.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class GateOutAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        tw.build()
        cls.g = Game()

    def test_clear_gt_hack(self):
        cases = [(a, p, d, t) for a in (0, 1, 2, 3) for p in (-1, 0, 1, 2) for d in (0, 1, 2) for t in (0, 7, 8, 9, 10)]
        got = tw.ask([f"gtclear {tw.hexs(a, p, t)}" for a, p, d, t in cases])
        bad = [(c, self.g.clear(*c), port["keep"]) for c, port in zip(cases, got) if self.g.clear(*c) != port["keep"]]
        self.assertEqual(bad, [])

    def test_file_list(self):
        fields = [19, 22, 23, 25, 27, 16, 44, 45, 46, 48, 50, 52, 66, 71, 73, 74, 76, 77, 91, 14, 100, 0]
        cases = [(a, p, ft, d, f, gt) for a in (0, 1, 2) for p in (0, 1, 2) for ft in (0, 4, 8) for d in (0, 1)
                 for f in fields for gt in (0, 1)]
        got = tw.ask([f"ghoid {tw.hexs(*c)}" for c in cases])
        bad = []
        for c, port in zip(cases, got):
            gid, files = self.g.file_list(*c)
            want_files = [f"x{port['file']}.ccs"] if port["file"] else []
            if gid != port["id"] or files != want_files:
                bad.append((c, gid, files, port))
        self.assertEqual(bad[:5], [])

    def test_gate_hacking_out(self):
        r = random.Random(0x6A7E)
        runs = [scenario(r) for _ in range(400)]
        got = tw.ask([probe_gho(st, fr) for st, fr in runs])
        bad = 0
        steps = set()
        for (st, fr), port in zip(runs, got):
            want = self.g.gho(st, fr)
            steps.update(w[0] for w in want)
            if want != port:
                bad += 1
                if bad <= 3:
                    k = next(i for i, (a, b) in enumerate(zip(want, port)) if a != b)
                    print("start", {x: st[x] for x in ("prog", "cnt", "act", "set", "camid")}, "frame", k,
                          "\n game", want[k], "\n port", port[k])
        self.assertEqual(bad, 0)
        self.assertEqual(steps, set(range(9)))


if __name__ == "__main__":
    unittest.main()
