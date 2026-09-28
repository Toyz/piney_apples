#!/usr/bin/env python3
"""crates/piney-toppage against the game's own code run in tools/eemu.py.

The toppage_probe example (built with cargo, feature `trace`) runs the
port's top page task (`ccThToppageCtrl` with its board, without the system
menu) over a scenario and prints one JSON line a frame; the same scenario
runs through TOPPAGE.PRG's own code in eemu: the constructor
(0x004009c0), then `ccThToppageCtrl::Main` (0x00400f50) once a frame, with
everything in toppage.cpp and bbs.cpp native, and ccSaveData::CheckWriteBbs,
ccEvent::CheckOperate, ccGame::ChangeArea / ChangeScene, ccSprite's
constructor and SetPrim and ccMask's constructor native from the main
executable. The calls into the rest of the main executable are hooked and
recorded:

  - ccAnm (constructor, SetAnm, _AnimateForward, Draw): which member, which
    animation, the speed; _AnimateForward returns the port's own result for
    that call and sets frameNow to the port's frame (the playback itself is
    checked elsewhere, docs/engine/animation.md);
  - ccKanji::Disp: which kanji, its position, colour, ctrl and packet count
    (ccKanji::Init is modelled: ctrl 0x11, m packets), and the bytes drawn;
  - ccSprite::MakePacket and SendPacket on the board's mask: every cell's
    fields (wu, wv, wi, su, sv) and its size and position bit for bit;
  - ccLayer::Init / ccView::SetFrame and ccSprite::SetTex of the board;
  - ccView::SetView, ccSeOn, ccSoundFadeOut, ccScFade::EntryFlash and
    EntryFlash2, ccGame::ChangeRequest (with InitScene's -1s),
    ccDtMenu::CheckMenuType (answered from the scenario).

Every frame the calls, in order, the sounds and requests, and the state
agree: the control's mode, command, counters and NEW flag, the board's
page, thread and post cursors, scroll bars (all three, eight fields each),
the writing page's counters, the blinking arrow, every thread and post
object with its read state, the save's whole bbsList, g_TP_pushFlag /
g_TP_pushCntr, enableReset and operateSet; after Log in, the scene
ChangeArea leaves in ccGame. Scenarios: scripted runs (a fresh save, the
posts event 1 makes, the player's own posts typed out, the Time Idol
post with a ranking in the save, a long thread, a parody save, locked
commands, the system menu open under the board, a carried key-repeat
count) and random pad runs over random board states.

The EE's `div` by zero is modelled as PCSX2 has it (LO -1 for a
non-negative dividend, 1 for a negative one): the board's scroll bars
divide by zero on a list exactly a page long, which the tables hold.

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import functools
import os
import random
import shutil
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va  # noqa: E402
inf_va = functools.partial(va, overlay='toppage')  # this overlay's globals
import test_save_init_rs  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
ISO = volume.ISO
EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "toppage_probe")

# gp globals (INF SLUS_202.67 .sdata).
CCSYS, SAVEDATA, EVENTMNG, GAME, DTMENU, SCFADEDEF = (inf_va(0x003788e0), inf_va(0x003789d8), inf_va(0x00378a94), inf_va(0x003789cc),
                                                      inf_va(0x003789d0), inf_va(0x00378968))
LAYER_ACTIVE, CMNDTARGET, TP_FLAG, TP_CNTR = inf_va(0x003788d8), inf_va(0x00378c64), inf_va(0x00378e1c), inf_va(0x00378e20)
SERVER_TBL = inf_va(0x00306dc0)
# Scratch memory.
SYS, SAVE, EV, GAMEOBJ, DTM, FADE, LAYER, VIEW, HEAP = (0x01800000, 0x01810000, 0x01830000, 0x01831000,
                                                        0x01832000, 0x01833000, 0x01834000, 0x01835000,
                                                        0x01900000)
OK, CANCEL, UP, DOWN, START = 0x40, 0x20, 0x1000, 0x4000, 0x800
BBS_LIST, LAST_TOWN, PARODY, TIME_IDOL = 0x28e4, 0x8426, 0x842b, 0x6775

CTOR, MAIN = inf_va(0x004009c0), inf_va(0x00400f50)
HOOKED = ("__nw__FUi", "__nwa__FUi", "__dl__FPv", "__dla__FPv", "__ct__5ccAnmFv", "__dt__5ccAnmFv",
          "SetAnm__5ccAnmFP8ccStreamPcUi", "_AnimateForward__5ccAnmFUi", "Draw__5ccAnmFv",
          "GetCCSAdrs__8ccStreamFPCc", "SetView__6ccViewFRC5ccCamPA4_f", "SetFrame__6ccViewFffffffff",
          "Init__7ccLayerFsP6ccView", "__dt__7ccLayerFv", "__ct__16ccDrawPacketCtrlFv", "__dt__6ccMaskFv",
          "SetTex__8ccSpriteFPcPci", "Init__7ccKanjiFii", "__dt__7ccKanjiFv", "Disp__7ccKanjiFPciff",
          "MakePacket__8ccSpriteFii", "SendPacket__8ccSpriteFv", "ccSeOn__Fi", "ccSoundFadeOut__Fv",
          "EntryFlash__8ccScFadeFiiffff", "EntryFlash2__8ccScFadeFiiiffff", "ChangeRequest__6ccGameFii",
          "CheckMenuType__8ccDtMenuFv")


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-toppage", "--features",
                    "trace", "--example", "toppage_probe"], cwd=ROOT, check=True)


def probe(lines):
    p = subprocess.run([EXAMPLE, ISO], input="\n".join(lines) + "\n", capture_output=True, text=True,
                       check=True, cwd=ROOT)
    return [json.loads(line) for line in p.stdout.splitlines()]


def ee_machine(program):
    """A Machine whose `div` / `divu` by zero leave LO as the EE does."""
    from eemu import M64, Machine, sx

    class EE(Machine):
        def exec(self, pc, w):
            if w >> 26 == 0 and (w & 63) in (0x1a, 0x1b):
                rs, rt = (w >> 21) & 31, (w >> 16) & 31
                if self.r[rt] & 0xFFFFFFFF == 0:
                    n = sx(self.r[rs], 32)
                    q = (1 if n < 0 else -1) if (w & 63) == 0x1a else -1
                    self.lo = sx(q, 32) & M64
                    self.hi = sx(n, 32) & M64
                    return None
            return super().exec(pc, w)

    return EE(program)


class Game:
    """ccThToppageCtrl in eemu, the calls it makes into the main executable
    recorded."""

    def __init__(self, program, saves, operate, keys):
        from eemu import _cstr
        self.p = p = program
        self.cstr = _cstr
        m = self.m = ee_machine(p)
        for g, v in ((CCSYS, SYS), (SAVEDATA, SAVE), (EVENTMNG, EV), (GAME, GAMEOBJ), (DTMENU, DTM),
                     (SCFADEDEF, FADE), (LAYER_ACTIVE, LAYER), (CMNDTARGET, 0)):
            m.store(g, 4, v)
        m.store(LAYER + 0x2c, 4, VIEW)
        m.store(SYS + 0, 4, 1)
        # The port's fresh save, made by the game's own boot (ok cross,
        # cancel circle), then the scenario's.
        m.mem[SAVE:SAVE + 0x8530] = test_save_init_rs.fresh_save(ELF)
        for off, data in saves:
            m.mem[SAVE + off:SAVE + off + len(data)] = data
        m.store(EV + 0x770, 8, operate)
        m.store(EV + 0x778, 2, 0xffff)
        m.store(GAMEOBJ + 0x4c, 4, 1)
        m.store(TP_CNTR, 4, keys[0])
        m.store(TP_FLAG, 4, keys[1])
        self.heap = HEAP
        self.calls, self.req, self.odd = [], [], []
        self.fwd = []
        self.menu_type = -1
        self.labels = {}
        self.servers = [m.load(SERVER_TBL + 4 * i, 4, True) for i in range(8)]
        h = {}
        for name in HOOKED:
            sym = p.symbol_named(name)
            assert sym is not None, name
            h[name] = sym.value

        def alloc(mm, n, *a):
            at = self.heap
            self.heap = (self.heap + max(n, 1) + 15) & ~15
            mm.mem[at:at + n] = bytes(n)
            return at

        def rec(op, obj, arg=""):
            self.calls.append([op, obj, arg])

        def anm_ctor(mm, a0, *a):
            mm.mem[a0:a0 + 0x110] = bytes(0x110)
            mm.store(a0 + 0x9c, 2, 256)
            return a0

        def set_anm(mm, a0, a1, a2, a3):
            rec("set", a0, self.cstr(mm, a2).decode())
            mm.store(a0 + 0xac, 4, 1)
            mm.store(a0 + 0x98, 4, 0)
            return 0

        def forward(mm, a0, a1, *a):
            rec("fwd", a0, str(a1 & 0xffff))
            if not self.fwd:
                self.odd.append("a forward the port did not make")
                return 0
            ended, frame = self.fwd.pop(0)
            mm.store(a0 + 0x98, 4, frame)
            return ended

        def disp(mm, a0, a1, a2, a3):
            col = [mm.load(a0 + 0x68 + 4 * i, 4) for i in range(4)]
            ctrl, packets = mm.load(a0 + 4, 4), mm.load(a0 + 0x24, 4)
            if a2 != 0xffffffff:
                self.odd.append(f"Disp count {a2}")
            rec("disp", a0, f"{mm.load(a0 + 0x40, 4):08x} {mm.load(a0 + 0x44, 4):08x} "
                            f"{col[0]} {col[1]} {col[2]} {col[3]} {ctrl} {packets} "
                            f"{self.cstr(mm, a1).hex()}")
            return 0

        def packet(mm, a0, a1, a2, *a):
            if (a1, a2) != (0, 1):
                self.odd.append(f"MakePacket({a1}, {a2})")
            f = [mm.load(a0 + o, 4) for o in (0x58, 0x5c, 0x60, 0x50, 0x54)]
            g = [mm.load(a0 + o, 4) for o in (0x48, 0x4c, 0x40, 0x44)]
            rec("pkt", a0, " ".join(str(v) for v in f) + " " + " ".join(f"{v:08x}" for v in g))
            return 0

        def layer_init(mm, a0, a1, a2, *a):
            view = alloc(mm, 0x100)
            mm.store(a0 + 0x2c, 4, view)
            rec("layer", "bbs", str(a1 & 0xffff))
            return 0

        def set_frame(mm, a0, *a):
            self.calls[-1][2] += " " + " ".join(f"{mm.f[12 + i] & 0xffffffff:08x}" for i in range(8))
            return 0

        def set_tex(mm, a0, a1, a2, a3):
            rec("tex", a0, f"{self.cstr(mm, a1).decode()} {self.cstr(mm, a2).decode()} {a3}")
            return 0

        def kanji_init(mm, a0, a1, a2, *a):
            mm.store(a0 + 4, 4, 0x11)
            mm.store(a0 + 0x24, 4, a2)
            return 0

        def change_request(mm, a0, a1, a2, *a):
            self.req.append(["mode", a1, a2])
            if a1 != 6:
                for o in range(0x14, 0x4c, 4):
                    mm.store(a0 + o, 4, 0xffffffff)
            return 0

        def flash(mm, a0, a1, a2, *a):
            rec("flash", "fade", f"{a1} {a2:08x}")
            return 0

        def flash2(mm, a0, a1, a2, a3):
            rec("flash2", "fade", f"{a1} {a2} {a3:08x}")
            return 0

        def se(mm, a0, *a):
            self.req.append(["se", a0])
            return 0

        def fadeout(mm, *a):
            self.req.append(["fadeout"])
            return 0

        def view(mm, a0, a1, *a):
            rec("view", "camera")
            return 0

        def menu_type(mm, *a):
            return self.menu_type & 0xffffffff

        nop = lambda mm, a0, *a: a0  # noqa: E731
        for name, fn in (("__nw__FUi", alloc), ("__nwa__FUi", alloc), ("__dl__FPv", nop), ("__dla__FPv", nop),
                         ("__ct__5ccAnmFv", anm_ctor), ("__dt__5ccAnmFv", nop),
                         ("SetAnm__5ccAnmFP8ccStreamPcUi", set_anm), ("_AnimateForward__5ccAnmFUi", forward),
                         ("Draw__5ccAnmFv", lambda mm, a0, *a: rec("draw", a0) or 0),
                         ("GetCCSAdrs__8ccStreamFPCc", lambda mm, *a: 0x01836000),
                         ("SetView__6ccViewFRC5ccCamPA4_f", view), ("SetFrame__6ccViewFffffffff", set_frame),
                         ("Init__7ccLayerFsP6ccView", layer_init), ("__dt__7ccLayerFv", nop),
                         ("__ct__16ccDrawPacketCtrlFv", nop), ("__dt__6ccMaskFv", nop),
                         ("SetTex__8ccSpriteFPcPci", set_tex), ("Init__7ccKanjiFii", kanji_init),
                         ("__dt__7ccKanjiFv", nop), ("Disp__7ccKanjiFPciff", disp),
                         ("MakePacket__8ccSpriteFii", packet),
                         ("SendPacket__8ccSpriteFv", lambda mm, a0, *a: rec("send", a0) or 0),
                         ("ccSeOn__Fi", se), ("ccSoundFadeOut__Fv", fadeout),
                         ("EntryFlash__8ccScFadeFiiffff", flash), ("EntryFlash2__8ccScFadeFiiiffff", flash2),
                         ("ChangeRequest__6ccGameFii", change_request),
                         ("CheckMenuType__8ccDtMenuFv", menu_type)):
            m.hooks[h[name]] = fn
        self.alloc = alloc

    # the labels the port records by -------------------------------------------
    def update_labels(self):
        ld = lambda a: self.m.load(a, 4)  # noqa: E731
        t = self.this
        for off, name in ((4, "enter"), (8, "neutral"), (12, "login"), (16, "bbs"), (20, "quit"),
                          (24, "camera"), (32, "bbsnew"), (36, "new")):
            self.labels[ld(t + off)] = name
        bc = ld(t)
        self.labels[bc] = "thpage"
        self.labels[bc + 0x1c0] = "msgpage"
        for base, n, pre in ((0x130, 18, "th"), (0x4a0, 7, "title"), (0x4bc, 10, "msg")):
            for i in range(n):
                k = ld(bc + base + 4 * i)
                if k:
                    self.labels[k] = f"{pre}{i}"
        self.labels[bc + 0x2d0] = "nomsg"
        self.labels[bc + 0x3b8] = "thread"
        self.labels[bc + 0x508] = "newmsg"
        mask = ld(bc + 0x824)
        if mask:
            self.labels[mask] = "mask"

    def translate(self, calls):
        out = []
        for op, obj, arg in calls:
            if isinstance(obj, int):
                obj = self.labels.get(obj, hex(obj))
            out.append([op, obj, arg])
        return out

    # running ------------------------------------------------------------------
    def start(self, port_calls):
        self.fwd = [(int(a.split()[1]), int(a.split()[2])) for op, _, a in port_calls if op == "fwd"]
        self.this = self.alloc(self.m, 0x44)
        self.m.call(CTOR, [self.this])
        self.update_labels()
        calls, self.calls = self.translate(self.calls), []
        return {"calls": calls, "state": self.state()}

    def frame(self, pad, menu_type, port_calls):
        m = self.m
        for o, v in zip((0x2cc, 0x2d0, 0x2d4, 0x2d8), pad):
            m.store(SYS + o, 4, v)
        self.menu_type = menu_type
        self.fwd = [(int(a.split()[1]), int(a.split()[2])) for op, _, a in port_calls if op == "fwd"]
        self.req = []
        m.call(MAIN, [self.this])
        self.update_labels()
        calls, self.calls = self.translate(self.calls), []
        return {"calls": calls, "req": self.req, "state": self.state()}

    def scene(self):
        g = [self.m.load(GAMEOBJ + o, 4, True) for o in range(0x14, 0x4c, 4)]
        return {"area": g[0], "area_prev": g[1], "server": g[2], "town": g[3], "field": g[4], "dungeon": g[5],
                "floor": g[6], "block": g[7]}

    def state(self):
        m = self.m
        ld = lambda a, n=4: m.load(a, n, True)  # noqa: E731
        u = lambda a: m.load(a, 4)  # noqa: E731
        t = self.this
        bc = u(t)
        thnum = ld(bc + 0x838)
        tbl = [u(bc + 0x61c + 4 * i) for i in range(max(thnum, 0))]
        threads = []
        for th in tbl:
            n = ld(th + 0x208)
            msgs = [u(th + 8 + 4 * i) for i in range(n)]
            threads.append([ld(th), ld(th + 0x20c), [[ld(mo + 0x478), ld(mo + 0x270)] for mo in msgs]])
        wt, wm = u(bc + 0x500), u(bc + 0x504)
        ti = tbl.index(wt) if wt in tbl else -1
        mi = -1
        if ti >= 0 and wm:
            msgs = [u(wt + 8 + 4 * i) for i in range(ld(wt + 0x208))]
            mi = msgs.index(wm) if wm in msgs else -1
        sb = lambda a: [ld(a + 4 * i) for i in range(8)]  # noqa: E731
        command = u(t + 0x1c)
        bbs = {"draw": ld(bc + 0x618), "exit": ld(bc + 0x840), "init": ld(bc + 0x83c), "thnum": thnum,
               "tindex": ld(bc + 0x178), "tstart": ld(bc + 0x17c),
               "sb": [sb(bc + 0x110), sb(bc + 0x180), sb(bc + 0x1a0)],
               "m": [ld(bc + o) for o in (0x4e4, 0x4e8, 0x4ec, 0x4f0, 0x4f4, 0x4f8)],
               "w": [ti, mi] + [ld(bc + o) for o in range(0x5f0, 0x614, 4)],
               "mark": [ld(bc + 0x828), ld(bc + 0x82c), ld(bc + 0x830), int(u(bc + 0x834) != 0)],
               "threads": threads}
        posts = [[i, v] for i, v in enumerate(m.mem[SAVE + BBS_LIST:SAVE + BBS_LIST + 128 * 48]) if v]
        return {"mode": ld(t + 0x28), "cmd": ld(t + 0x34), "act": ld(t + 0x3c), "proc": ld(t + 0x38),
                "bbsnew": ld(t + 0x40), "command": self.labels.get(command, "none") if command else "none",
                "exit": ld(t + 0x2c), "bbs": bbs, "keys": [ld(TP_FLAG), ld(TP_CNTR)], "reset": ld(GAMEOBJ + 0x4c),
                "opset": ld(EV + 0x778, 2), "posts": posts, "lasttown": ld(SAVE + LAST_TOWN, 1)}


def seen(port):
    """What a run went through, from the port's frames."""
    out = set()
    for f in port[1:]:
        st, b = f["state"], f["state"]["bbs"]
        out.add(("mode", st["mode"]))
        if st["mode"] == 3:
            out.add(("page", b["draw"]))
            if b["draw"] == 1:
                out.add(("reading", b["m"][3]))
            if b["m"][2]:
                out.add("title scrolled away")
            if b["m"][4] > 0:
                out.add("post scrolled")
            if b["tstart"] > 0:
                out.add("threads scrolled")
            if b["mark"][0]:
                out.add("arrow")
            if b["w"][8]:
                out.add("written out")
        for r in f["req"]:
            out.add(tuple(r))
        for op, obj, arg in f["calls"]:
            out.add((op, obj))
            if op == "disp":
                out.add(("text", bytes.fromhex(arg.split()[-1]) if len(arg.split()) > 8 else b""))
    return out


def fwd_arg(c):
    """A port call as the game's side records it (a forward's speed only)."""
    op, obj, arg = c
    return [op, obj, arg.split()[0] if op == "fwd" else arg]


def scenario_lines(saves, operate, keys, frames):
    lines = ["reset"]
    for off, data in saves:
        lines.append(f"save {off:x} {data.hex()}")
    lines.append(f"operate {operate:x}")
    lines.append(f"keys {keys[0]} {keys[1]}")
    lines.append("start")
    for raw, mt in frames:
        lines.append(f"frame {raw:x} {mt}")
    return lines


def posts_bytes(states):
    """bbsList bytes for {(thread, post): state}."""
    b = bytearray(128 * 48)
    for (t, p), v in states.items():
        b[48 * t + p] = v
    return [(BBS_LIST, bytes(b))]


def presses(n, script, holds=()):
    """n frames of raw buttons: {frame: buttons} pressed that frame, and
    (first, last, buttons) held."""
    out = []
    for f in range(n):
        b = script.get(f, 0)
        for a, z, h in holds:
            if a <= f <= z:
                b |= h
        out.append(b)
    return out


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class TopPageAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from image import Program
        build()
        cls.p = Program(ELF, "toppage")
        cls.tables = probe(["dump"])[0]["tables"]

    def run_scenario(self, name, saves=(), operate=0, keys=(0, 0), raws=(), menu=None, expect=None):
        """Runs the scenario through both and compares every frame; returns
        the port's frames."""
        menu = menu or {}
        frames = [(r, menu.get(f, -1)) for f, r in enumerate(raws)]
        port = probe(scenario_lines(list(saves), operate, keys, frames))
        g = Game(self.p, list(saves), operate, keys)
        bad = []
        first = port[0]
        got = g.start(first["calls"])
        want_calls = [fwd_arg(c) for c in first["calls"]]
        if got["calls"] != want_calls or got["state"] != first["state"]:
            bad.append(("start", got, first))
        for f, (pf, (raw, mt)) in enumerate(zip(port[1:], frames)):
            got = g.frame(pf["pad"], mt, pf["calls"])
            want = {"calls": [fwd_arg(c) for c in pf["calls"]],
                    "req": [r for r in pf["req"] if r[0] not in ("area", "reset")], "state": pf["state"]}
            got_cmp = {"calls": got["calls"], "req": got["req"], "state": got["state"]}
            if got_cmp != want:
                bad.append((f, got_cmp, want))
            for r in pf["req"]:
                if r[0] == "area":
                    area, town = r[1], r[2]
                    s = g.scene()
                    server = self.servers(g)[town] if 0 <= town < 8 else s["server"]
                    exp = {"area": area, "area_prev": -1, "server": server, "town": town, "field": -1,
                           "dungeon": -1, "floor": -1, "block": -1}
                    if s != exp:
                        bad.append((f, "scene", s, exp))
            if len(bad) > 3:
                break
        if g.odd:
            bad.append(("odd", g.odd[:5]))
        for b in bad[:3]:
            print(name, "frame", b[0])
            if len(b) == 3 and isinstance(b[1], dict):
                for k in b[2]:
                    if b[1].get(k) != b[2][k]:
                        print("  ", k, "game", json.dumps(b[1].get(k))[:3000])
                        print("  ", k, "port", json.dumps(b[2][k])[:3000])
            else:
                print("  ", b[1:])
        self.assertEqual(bad, [], name)
        if expect:
            expect(port)
        return port

    def servers(self, g):
        return g.servers

    # scripted -------------------------------------------------------------------
    def test_fresh_save(self):
        # The log-in animation to its end, the menu, the empty board, back,
        # and Log out.
        n = 520
        raws = presses(n, {170: DOWN, 190: DOWN, 200: UP, 215: OK, 300: DOWN, 320: CANCEL, 330: UP,
                           345: DOWN, 360: DOWN, 380: OK})

        def expect(port):
            s = seen(port)
            for want in (("se", 9), ("se", 10), ("mode", 3, 7), ("fadeout",), ("mode", 1), ("mode", 2),
                         ("mode", 3), ("page", 0), ("draw", "thpage"), ("flash", "fade"), ("flash2", "fade")):
                self.assertIn(want, s)
            self.assertEqual(port[-1]["state"]["bbs"]["thnum"], 0)

        self.run_scenario("fresh", raws=raws, expect=expect)

    def event1_posts(self):
        return {(62, p): 1 for p in range(7)}

    def test_board_reading(self):
        # Event 1's posts (thread 62, posts 0-6) and more, some read: the log-in
        # skipped, the board, a thread's posts, a post read and scrolled, back
        # out, then Log in on lastTown 2.
        st = self.event1_posts()
        st.update({(0, 0): 3, (0, 1): 1, (1, 0): 1, (1, 2): 3, (5, 1): 1, (13, 0): 3, (30, 4): 1})
        saves = posts_bytes(st) + [(LAST_TOWN, bytes([2]))]
        script = {5: CANCEL, 70: CANCEL, 90: DOWN, 100: OK, 150: DOWN, 160: DOWN, 170: UP, 180: OK,
                  200: DOWN, 210: DOWN, 220: OK, 240: DOWN, 250: DOWN, 260: DOWN, 270: UP, 280: CANCEL,
                  290: DOWN, 300: OK, 320: CANCEL, 330: CANCEL, 340: DOWN, 350: OK, 370: DOWN, 380: OK,
                  400: CANCEL, 410: CANCEL, 420: CANCEL, 470: UP, 480: OK}
        raws = presses(560, script, holds=[(430, 460, DOWN)])

        def expect(port):
            s = seen(port)
            for want in (("mode", 5, 8), ("area", 0, 2), ("mode", 6, 7), ("se", 13), ("reading", 0),
                         ("reading", 1), "post scrolled", ("disp", "msg0"), ("disp", "th3"), ("pkt", "mask"),
                         ("disp", "thread"), ("send", "mask")):
                self.assertIn(want, s)
            # Thread 1's post 0 and thread 5's post 1, read and closed.
            posts = dict(map(tuple, port[-1]["state"]["posts"]))
            self.assertEqual((posts[48 * 1 + 0], posts[48 * 5 + 1]), (3, 3))

        self.run_scenario("reading", saves=saves, raws=raws, expect=expect)

    def test_writing(self):
        # Two of the player's own posts (bbs_post7) typed out: waited, then
        # hurried with OK; each opens its thread on it; the second is found
        # on leaving the first's thread.
        st = self.event1_posts()
        st.update({(62, 2): 7, (3, 1): 7, (3, 0): 1})
        saves = posts_bytes(st)
        script = {5: CANCEL, 70: DOWN, 80: OK, 400: OK, 405: OK, 420: CANCEL, 430: OK, 432: OK,
                  440: OK, 460: CANCEL, 470: CANCEL, 480: CANCEL}
        raws = presses(520, script)

        def expect(port):
            s = seen(port)
            for want in (("page", 2), "written out", ("disp", "newmsg"), ("reading", 0)):
                self.assertIn(want, s)
            posts = dict(map(tuple, port[-1]["state"]["posts"]))
            self.assertEqual((posts[48 * 62 + 2], posts[48 * 3 + 1]), (3, 3))

        self.run_scenario("writing", saves=saves, raws=raws, expect=expect)

    def test_time_idol_and_long_threads(self):
        # The longest thread (26 posts, the cursor down its tree until the
        # thread's title scrolls away, and back), the six-post thread (its
        # scroll bar a page exactly: the EE's divide by zero), and the Time
        # Idol post (thread 29, post 1) over a ranking in the save, read and
        # scrolled.
        long_t = max(range(len(self.tables[0])), key=lambda t: len(self.tables[0][t]))
        six = [t for t, th in enumerate(self.tables[0]) if len(th) == 6]
        self.assertEqual((long_t, len(self.tables[0][long_t]), six), (7, 26, [11]))
        st = {(29, p): 1 for p in range(len(self.tables[0][29]))}
        st.update({(long_t, p): (1 if p % 3 else 3) for p in range(26)})
        st.update({(11, p): 1 for p in range(6)})
        rank = bytearray(200)
        for i in range(5):
            rank[40 * i:40 * i + 6] = f"Kite{i}".encode()
            rank[40 * i + 20:40 * i + 28] = f"0{i}:12:34".encode()
        saves = posts_bytes(st) + [(TIME_IDOL, bytes(rank))]
        script = {5: CANCEL, 70: DOWN, 80: OK, 150: OK, 265: OK, 335: CANCEL, 405: CANCEL, 410: DOWN, 420: OK,
                  475: OK, 480: CANCEL, 490: CANCEL, 500: DOWN, 510: OK, 520: DOWN, 530: OK, 610: CANCEL,
                  620: CANCEL, 630: CANCEL}
        holds = [(160, 260, DOWN), (270, 330, DOWN), (340, 400, UP), (430, 470, DOWN), (540, 600, DOWN)]
        raws = presses(660, script, holds=holds)

        def expect(port):
            s = seen(port)
            for want in ("title scrolled away", "arrow", "post scrolled", ("text", b"Player: Kite0"),
                         ("text", b"Time:   02:12:34"), ("reading", 1)):
                self.assertIn(want, s)
            self.assertEqual(max(p["state"]["bbs"]["m"][1] for p in port[1:]), 9)
            self.assertIn(-1, [p["state"]["bbs"]["sb"][2][2] for p in port[1:]])

        self.run_scenario("long", saves=saves, raws=raws, expect=expect)

    def test_parody_locks_and_menu(self):
        # A parody save with a post in every thread: Log in locked by the
        # events (operation 25; operateSet takes the 6 asked first), the
        # thread list scrolled past its 18 rows, OK and cancel pushed while
        # the system menu is open (CheckMenuType 0, then 12), Log out; a
        # key-repeat count carried in.
        st = {(t, 0): 1 for t in range(63)}
        st.update({(2, 1): 3})
        saves = posts_bytes(st) + [(PARODY, bytes([1]))]
        script = {5: CANCEL, 70: OK, 80: DOWN, 90: OK, 265: OK, 270: OK, 280: OK, 290: CANCEL, 295: CANCEL,
                  300: CANCEL, 340: CANCEL, 380: DOWN, 390: OK}
        holds = [(130, 260, DOWN), (305, 330, UP)]
        raws = presses(440, script, holds=holds)

        def expect(port):
            s = seen(port)
            for want in ("threads scrolled", ("mode", 3, 7), ("reading", 1)):
                self.assertIn(want, s)
            self.assertNotIn(("mode", 5, 8), s)
            self.assertEqual(port[-1]["state"]["opset"], 6)

        self.run_scenario("parody", saves=saves, operate=1 << 25, keys=(4, 1), raws=raws,
                          menu={265: 0, 290: 12}, expect=expect)

    # random --------------------------------------------------------------------
    def test_random_runs(self):
        choices = [0, 0, 0, 0, UP, DOWN, OK, CANCEL, UP | DOWN, OK | CANCEL, START, OK | DOWN]
        reached = set()
        for seed in range(6):
            rng = random.Random(seed)
            st = {}
            for t, th in enumerate(self.tables[seed % 2]):
                if rng.random() < 0.3:
                    for p in range(len(th)):
                        if rng.random() < 0.6:
                            st[(t, p)] = rng.choice([1, 1, 3, 3, 3, 7] if rng.random() < 0.1 else [1, 3])
            saves = posts_bytes(st) + [(PARODY, bytes([seed % 2])), (LAST_TOWN, bytes([rng.randrange(8)]))]
            raws, b = [], 0
            for f in range(900):
                if rng.random() < 0.08:
                    b = rng.choice(choices)
                raws.append(b)
            menu = {f: rng.choice([0, 12]) for f in range(900) if rng.random() < 0.03}
            port = self.run_scenario(f"random {seed}", saves=saves, operate=rng.choice([0, 0, 1 << 6, 1 << 27]),
                                     keys=(rng.randrange(6), 0), raws=raws, menu=menu)
            reached |= seen(port)
        for want in (("page", 0), ("page", 1), ("reading", 1), ("mode", 3, 7), ("mode", 5, 8)):
            self.assertIn(want, reached)


if __name__ == "__main__":
    unittest.main()
