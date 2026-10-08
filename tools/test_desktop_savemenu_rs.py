#!/usr/bin/env python3
"""crates/piney-desktop's save menus after the staff roll (dtmenu/save.rs:
menu 8 `ccDtMenu::SaveSelMenu`, menu 9 `SaveMenu` and `SaveMenuDisp`)
against the game's own run in tools/eemu.py, frame by frame.

The game's `ccThDtMenu` runs natively as in tools/test_desktop_menu_rs.py
(drawing, threads and the message window hooked and logged, a breath ends a
frame), with `ccSaveSys` running natively beside it: `StartReq`'s
`ccThSaveSys` is taken to run before the menu task each frame from the
second frame after it was made (its first run only breathes), its
`MainProccess` called between the menu's frames. The memory card is
tools/test_desktop_data_rs.py's (the sceMc* calls answered from a dict of
files per port, complete at once); desktop_probe's `savemenu` runs the port
over the same card rules.

`openReqNum` is set to 8 at a frame, as the ending's `staff_roll` does,
and the same pad script drives both. Compared each frame: the menu's state
(menu, menuNext, status, proccess, alpha), `waitCount`, `exceptionDisp`,
list 9's select, card slot, file, last result, disp, width and count, list
8's select, `ccMsg +0x24`; `ccSaveSys`'s result, proccess, port, fileNum
and whether its task runs; the sounds; `ccMsg->DispInfo` and `DispMsg`
lines; every packet of menuKanji, the four name kanji, menuWindow and
menuWindowA (positions to 0.01, colours); menuFont's strings with their
places and colours. At the end, the files each side's card holds.

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import functools
import os
import random
import shutil
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc
inf_va = functools.partial(va, overlay='desktop')  # this overlay's globals

import test_desktop_menu_rs as M  # noqa: E402
from test_desktop_data_rs import DIR, INDEX, SLOT_SIZE, VOL, Card, HMS, record  # noqa: E402
from eemu import ANNUL, RETURN_SENTINEL, Stop, _cstr  # noqa: E402

OK, CANCEL, UP, DOWN = M.OK, M.CANCEL, M.UP, M.DOWN
SAVESYS, MCOBJ = 0x01850000, 0x01852000
CCMSG = inf_va(0x00378a8c)
LIST8, LIST9 = 0x50 + 8 * 32, 0x50 + 9 * 32


def nested_call(m, addr, args, limit=50_000_000):
    """A call from inside a hook: the registers kept, a stack below the
    caller's, back to the hook when it returns."""
    saved = (m.r, m.f, m.hi, m.lo, m.hi1, m.lo1, m.fcr31, m.acc, m.steps)
    sp = m.r[29] & 0xFFFFFFFF
    m.r = [0] * 32
    m.f = list(saved[1])
    m.r[28] = m.p.gp
    m.r[29] = (sp - 0x4000) & ~0xF
    m.r[31] = RETURN_SENTINEL
    for i, a in enumerate(args[:8]):
        m.set32(4 + i, a)
    pc, npc, steps = addr, addr + 4, 0
    while pc != RETURN_SENTINEL:
        steps += 1
        if steps > limit:
            raise Stop(f"nested call gave up at 0x{pc:08x}")
        hook = m.hooks.get(pc)
        if hook is not None:
            m.set32(2, hook(m, *(m.r[4 + i] & 0xFFFFFFFF for i in range(4))))
            pc = m.r[31] & 0xFFFFFFFF
            npc = pc + 4
            continue
        target = m.exec(pc, m.load(pc, 4))
        if target is ANNUL:
            pc, npc = npc + 4, npc + 8
        else:
            pc, npc = npc, (target if target is not None else npc + 4)
    ret = m.r[2] & 0xFFFFFFFF
    m.r, m.f, m.hi, m.lo, m.hi1, m.lo1, m.fcr31, m.acc, m.steps = saved
    return ret


class SaveEmu(M.Emu):
    """ccThDtMenu with ccSaveSys and a card."""

    def __init__(self, card, patches, sys_port=0, sys_file=0, rate=1, unplugged=False):
        super().__init__(status=2, save=M.FRESH, load_frames=1)
        m = self.m
        h = m.hooks
        h[M.sym("GetFrameRate__8ccSystemFv")] = lambda mm, *a: rate
        self.card = card
        for off, data in patches:
            m.mem[M.SAVE_BASE + off:M.SAVE_BASE + off + len(data)] = data
        sym = M.sym
        for n in ("SaveSelMenu__8ccDtMenuFv", "SaveMenu__8ccDtMenuFv"):
            del h[sym(n)]
        if not m.load(CCMSG, 4):
            m.store(CCMSG, 4, self.alloc(0x400))
        # A controller in the first port: ccSys.pad[0].status is its set-up
        # state, not 0 (from Mutation on SaveMenu holds its input after a
        # save only while the game is paused or a pad is there).
        m.store(M.SYS + 0x26c, 4, 0 if unplugged else 1)
        m.store(sym("saveSys"), 4, SAVESYS)
        m.store(sym("ccMc"), 4, MCOBJ)
        for n, f in card.hooks().items():
            h[sym(n)] = f
        for n in ("ccMalloc__FUi", "__nwa__FUi", "ccMallocB__FUi"):
            h[sym(n)] = lambda mm, n, *a: self.alloc(n)
        h[sym("ccFree__FPv")] = lambda mm, *a: 0
        h[sym("__dla__FPv")] = lambda mm, *a: 0
        h[sym("sprintf")] = self._sprintf
        h[sym("sceCdSearchFile")] = self._cd_search
        h[sym("Read2__6ccCdvdFUiUiPv")] = self._cd_read
        m.call(sym("__sinit_sdmng.cpp"), [])
        m.call(sym("__ct__9ccSaveSysFv"), [SAVESYS])
        m.store(SAVESYS + 0x2a0, 4, sys_port)
        m.store(SAVESYS + 0x2a4, 4, sys_file)
        self.task_frame = None
        parent_start = h[sym("ccStartThread__FPFPv_vii")]
        thsave = sym("ccThSaveSys__FP6ccTscb")

        def start_thread(mm, fn, pri, stack, *a):
            t = parent_start(mm, fn, pri, stack, *a)
            if fn == thsave:
                self.task_frame = self.frame
            return t
        h[sym("ccStartThread__FPFPv_vii")] = start_thread

        def disp_info(mm, this, s0, s1, s2, *a):
            s3 = mm.r[8] & 0xFFFFFFFF
            self.ev("disp_info", lines=[self.cstr(x) if x else None for x in (s0, s1, s2, s3)])
            return 0
        h[sym("DispInfo__9ccMessageFPcPcPcPc")] = disp_info

        def disp_msg(mm, this, data, name, *a):
            lines = [mm.load(data + o, 4) for o in (0xc, 0x10, 0x14)]
            self.ev("disp_msg", lines=[self.cstr(x) if x else None for x in lines] + [None])
            return 0
        h[sym("DispMsg__9ccMessageFP9ccMsgDataPc")] = disp_msg

    def _sprintf(self, m, dst, fmt, a2, a3):
        f = _cstr(m, fmt)
        args = [a2, a3, m.r[8] & 0xffffffff, m.r[9] & 0xffffffff, m.r[10] & 0xffffffff]
        out, k, j = b"", 0, 0
        while j < len(f):
            if f[j:j + 1] == b"%":
                t = f[j + 1:j + 2]
                if t == b"s":
                    out += _cstr(m, args[k])
                elif t == b"d":
                    out += str(struct.unpack("<i", struct.pack("<I", args[k]))[0]).encode()
                else:
                    raise RuntimeError(f)
                k += 1
                j += 2
            else:
                out += f[j:j + 1]
                j += 1
        m.mem[dst:dst + len(out) + 1] = out + b"\0"
        return len(out)

    def icon_bin(self):
        if not hasattr(self, "_icons"):
            with open(os.path.join(os.path.dirname(M.ELF), "DATA", "ICON.BIN"), "rb") as f:
                self._icons = f.read()
        return self._icons

    def _cd_search(self, m, fp, name, *_):
        m.store(fp, 4, 0)
        m.store(fp + 4, 4, len(self.icon_bin()))
        return 1

    def _cd_read(self, m, this, lsn, sectors, buf):
        data = self.icon_bin()[2048 * lsn:2048 * (lsn + sectors)]
        m.mem[buf:buf + len(data)] = data
        return 1

    def state(self):
        st = super().state()
        if st is None:
            return None
        d = self.dt()
        m = self.m
        g = lambda o: m.load(d + o, 2, True)  # noqa: E731
        msg = m.load(CCMSG, 4)
        st["save"] = [g(0xc), g(0xe), g(LIST9 + 0xa), g(LIST9 + 0x10), g(LIST9 + 0xc), g(LIST9 + 0x18),
                      g(LIST9 + 0x12), g(LIST9 + 0x14), g(LIST9 + 0x16), g(LIST8 + 0xa),
                      m.load(msg + 0x24, 2, True) if msg else 0]
        st["sys"] = [m.load(SAVESYS + 0x2a8, 4), m.load(SAVESYS + 0x2ac, 4, True), m.load(SAVESYS + 0x2a0, 4, True),
                     m.load(SAVESYS + 0x2a4, 4, True), int(m.load(SAVESYS + 0x2b4, 4) != 0)]
        return st


class Scenario:
    """A card, the save's patches, ccSaveSys's position, the frame
    openReqNum 8 comes, and the pads (push, repeat) by frame."""

    def __init__(self, name, frames=300, open_at=4, rate=1):
        self.name = name
        self.frames = frames
        self.open_at = open_at
        self.rate = rate
        self.setup = []
        self.script = {}
        self.sys_port, self.sys_file = 0, 0
        self.unplugged = False    # no controller in port 1 (ccSys.pad[0].status 0)
        # The name pointer and level ccSaveSys writes into the record, as
        # tools/test_desktop_data_rs.py sets them; the clear flag the
        # volume's number (the ending's clear_count).
        self.patches = [(0x7488, struct.pack("<I", M.SAVE_BASE)), (0x7496, bytes([7])), (0x842a, bytes([VOL]))]

    def card(self, port, present=1, ps2=1, formatted=1, full=0, failwrite=0, failsys=0, failfmt=0):
        self.setup.append(("card", port, present, ps2, formatted, full, failwrite, failsys, failfmt))
        return self

    def file(self, port, name, data):
        self.setup.append(("file", port, name, bytes(data)))
        return self

    def prepared(self, port, records=()):
        idx = bytearray(336)
        for slot, rec in records:
            idx[28 * slot:28 * slot + 28] = rec
        self.file(port, DIR.lstrip("/"), idx)
        for s in range(12):
            self.file(port, f"dhdata{s + 1:02d}", bytes(SLOT_SIZE))
        return self

    def save(self, off, data):
        self.patches.append((off, bytes(data)))
        return self

    def keys(self, text, start=20, gap=5):
        """The pads from `start`, one token each `gap` frames: o OK, x
        cancel (pushes), u d up and down (repeats), . nothing."""
        f = start
        for k in text.split():
            if k in "ud":
                self.script[f] = (0, UP if k == "u" else DOWN)
            elif k in "ox":
                self.script[f] = (OK if k == "o" else CANCEL, 0)
            f += gap
        self.frames = max(self.frames, f + 40)
        return self

    def probe_lines(self):
        out = ["savemenu", f"frames {self.frames}", f"open {self.open_at}", f"rate {self.rate}",
               f"sys {self.sys_port} {self.sys_file}"]
        if self.unplugged:
            out.append("unplugged")
        for s in self.setup:
            if s[0] == "file":
                out.append(f"file {s[1]} {s[2]} {s[3].hex()}")
            else:
                out.append(" ".join(str(v) for v in s))
        for off, data in self.patches:
            out.append(f"save {off} {data.hex()}")
        for f, (p, r) in sorted(self.script.items()):
            out.append(f"pad {f} {p} {r}")
        out.append("end")
        return out

    def run_game(self):
        card = Card()
        for s in self.setup:
            if s[0] == "card":
                keys = ("present", "ps2", "formatted", "full", "failwrite", "failsys", "failfmt")
                card.ports[s[1]].update(dict(zip(keys, s[2:])))
            else:
                card.ports[s[1]]["dir"] = 1
                card.ports[s[1]]["files"][f"{DIR}/{s[2]}"] = bytearray(s[3])
        e = SaveEmu(card, self.patches, self.sys_port, self.sys_file, self.rate, self.unplugged)

        def on_frame(emu):
            if emu.frame == self.open_at:
                emu.m.store(emu.dt() + 6, 2, 8)
            running = emu.m.load(SAVESYS + 0x2b4, 4)
            if running and emu.task_frame is not None and emu.frame >= emu.task_frame + 2:
                nested_call(emu.m, M.sym("MainProccess__9ccSaveSysFv"), [SAVESYS])
        log = e.run(self.frames, self.script, on_frame)
        names = e.objnames()
        out = {}
        for f, k, kw in log:
            o = out.setdefault(f, {"pk": [], "ev": [], "st": None})
            if k == "packet":
                n = names.get(kw["obj"], kw["obj"])
                if n in ("win", "winA", "kanji", "set0", "set1", "set2", "set3"):
                    o["pk"].append((n, kw["code"], kw["dx"], kw["dy"], kw["sx"], kw["sy"], list(kw["rgba"])))
            elif k == "str":
                o["ev"].append(("str", kw["text"], kw["dx"], kw["dy"], list(kw["rgba"])))
            elif k == "se":
                o["ev"].append(("se", kw["n"]))
            elif k in ("disp_info", "disp_msg"):
                o["ev"].append((k, kw["lines"]))
            elif k == "frame_end":
                s = kw["state"]
                o["st"] = [s["menu"], s["next"], s["status"], s["proc"], s["alpha"]] + s["save"] + s["sys"]
        return {"frames": out, "files": card.summary()}


def port(scenarios):
    lines = [line for s in scenarios for line in s.probe_lines()]
    p = subprocess.run([M.EXAMPLE, M.ISO], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True,
                       cwd=M.ROOT)
    runs, cur = [], {}
    for line in p.stdout.splitlines():
        j = json.loads(line)
        if "files" in j:
            runs.append({"frames": cur, "files": j["files"]})
            cur = {}
            continue
        o = {"pk": [], "ev": [], "st": j["st"][:5] + j["save"][:11] + j["sys"]}
        for e in j["ev"]:
            if e[0] == "pk":
                o["pk"].append((e[1], e[2], e[3], e[4], e[5], e[6], e[7]))
            elif e[0] == "str":
                o["ev"].append(("str", e[1], e[2], e[3], e[4]))
            elif e[0] in ("disp_info", "disp_msg"):
                o["ev"].append((e[0], e[1]))
        for r in j["req"]:
            o["ev"].append(tuple(r))
        cur[j["f"]] = o
    return runs


# Waits: after a push, the next message is up about ten frames later.
WAIT = ". . . . . ."
YES = "u o"


def scenarios():
    out = []
    recs = [(0, record(level=1, name=b"Kite", playtime=HMS(0, 0, 0))),
            (1, record(level=20, clear=VOL, name=b"Orca", playtime=HMS(999, 59, 59))),
            (2, record(level=99, parody=1, name=b"Balmung", playtime=HMS(12, 5, 9))),
            (3, record(level=5, clear=VOL + 1, name=b"ABCDEFGHIJKLMNOPQ", playtime=HMS(1, 0, 0)))]
    # Card slot 1 with no save directory: "Create new save data?" YES,
    # created, the file list; the second file, "Create new data?" YES,
    # "Data saved."; the same file again, "Data already exists." cancelled;
    # then back out.
    create = (f"o {WAIT} o {WAIT} o {WAIT} {YES} {WAIT} o {WAIT} . d o {WAIT} {YES} {WAIT} o {WAIT} "
              f"o {WAIT} x {WAIT} x {WAIT} x")
    out.append(Scenario("create and save").card(0).keys(create))
    # No controller in port 1: from Mutation on the hold after "Save data
    # created." and "Data saved." is dropped (MUT main 0x0016fb1c).
    s = Scenario("create and save, unplugged").card(0)
    s.unplugged = True
    out.append(s.keys(create.replace(WAIT, ". .") + f" {WAIT} x {WAIT} x {WAIT} x", gap=3))
    # As the game runs it after the staff roll: frame rate 2.
    out.append(Scenario("create and save, rate 2", rate=2).card(0).keys(create))
    # Cancel at the question; the Cancel row.
    out.append(Scenario("cancel").keys("x"))
    out.append(Scenario("the Cancel row").keys("d . o"))
    # A prepared card: the list of records (clear data, parody, a long
    # name); the fourth file (clear data past the volume's): the warning
    # and NO; the second
    # (the save's own clear flag): "Data already exists." YES, saved.
    s = Scenario("overwrite").card(0).prepared(0, recs).save(0x8400, struct.pack("<i", HMS(20, 1, 2, 3)))
    s.keys(f"o {WAIT} o {WAIT} . d . d . d o {WAIT} o {WAIT} o {WAIT} u o {WAIT} {YES} {WAIT} o {WAIT} x {WAIT} x")
    out.append(s)
    s = Scenario("overwrite, rate 2", rate=2).card(0).prepared(0, recs)
    s.keys(f"o {WAIT} o {WAIT} . d . d o {WAIT} o {WAIT} {YES} {WAIT} o {WAIT} x {WAIT} x")
    out.append(s)
    # Card slot 2, ccSaveSys left at slot 2 file 6.
    s = Scenario("card slot 2").card(0, present=0).card(1).prepared(1, recs[:2])
    s.sys_port, s.sys_file = 1, 5
    s.keys(f"o {WAIT} o {WAIT} . u . u o {WAIT} {YES} {WAIT} o {WAIT} x {WAIT} x")
    out.append(s)
    # No card in either slot; not a PS2 card.
    out.append(Scenario("no card").card(0, present=0).card(1, present=0)
               .keys(f"o {WAIT} o {WAIT} o {WAIT} d o {WAIT} o {WAIT} x"))
    out.append(Scenario("not a PS2 card").card(0, ps2=0).keys(f"o {WAIT} o {WAIT} o {WAIT} x {WAIT} x"))
    # Unformatted: NO to the format; then YES, formatted, the directory
    # made, saved to the first file.
    s = Scenario("unformatted").card(0, formatted=0)
    s.keys(f"o {WAIT} o {WAIT} o {WAIT} o {WAIT} o {WAIT} o {WAIT} o {WAIT} {YES} {WAIT} o {WAIT} {YES} {WAIT} "
           f"o {WAIT} {YES} {WAIT} o {WAIT} o {WAIT} {YES} {WAIT} o {WAIT} x {WAIT} x {WAIT} x")
    out.append(s)
    out.append(Scenario("format fails").card(0, formatted=0, failfmt=1)
               .keys(f"o {WAIT} o {WAIT} o {WAIT} {YES} {WAIT} o {WAIT} x {WAIT} x"))
    # No directory: NO to making it.
    out.append(Scenario("no directory, NO").card(0).keys(f"o {WAIT} o {WAIT} o {WAIT} o {WAIT} x {WAIT} x"))
    # Full; the writes failing.
    out.append(Scenario("full").card(0, full=1).keys(f"o {WAIT} o {WAIT} o {WAIT} o {WAIT} x {WAIT} x"))
    out.append(Scenario("write fails").card(0, failwrite=1).prepared(0)
               .keys(f"o {WAIT} o {WAIT} o {WAIT} {YES} {WAIT} o {WAIT} x {WAIT} x"))
    out.append(Scenario("index write fails").card(0, failsys=1).prepared(0, recs[:1])
               .keys(f"o {WAIT} o {WAIT} d o {WAIT} {YES} {WAIT} o {WAIT} x {WAIT} x"))
    # Random pads over a prepared card.
    rng = random.Random(9)
    s = Scenario("random pads").card(0).card(1).prepared(0, recs).prepared(1, recs[1:3])
    s.keys(" ".join(rng.choice("oouudd..x") for _ in range(160)), gap=3)
    out.append(s)
    return out


@unittest.skipUnless(os.path.exists(M.ELF) and os.path.exists(M.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class SaveMenusAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-desktop", "--features",
                        "trace", "--example", "desktop_probe"], cwd=M.ROOT, check=True)

    def compare(self, s, got):
        want = s.run_game()
        g, p = want["frames"], got["frames"]
        for f in range(1, s.frames - 1):
            a, b = g.get(f), p.get(f)
            self.assertIsNotNone(a, f"{s.name}: no game frame {f}")
            self.assertIsNotNone(b, f"{s.name}: no port frame {f}")
            self.assertEqual(a["st"], b["st"], f"{s.name} frame {f}: state")
            key = lambda e: (e[0], json.dumps(e[1:]))  # noqa: E731
            self.assertEqual(sorted(a["ev"], key=key), sorted(b["ev"], key=key), f"{s.name} frame {f}: events")
            for obj in ("win", "winA", "kanji", "set0", "set1", "set2", "set3"):
                ga = [x for x in a["pk"] if x[0] == obj]
                pa = [x for x in b["pk"] if x[0] == obj]
                self.assertEqual(len(ga), len(pa), f"{s.name} frame {f}: {obj} packets {ga} vs {pa}")
                for x, y in zip(ga, pa):
                    self.assertTrue(M.same_pk(x, y), f"{s.name} frame {f}: {obj} {x} vs {y}")
        self.assertEqual(want["files"], got["files"], f"{s.name}: the card's files")
        return want

    def test_scenarios(self):
        scen = scenarios()
        got = port(scen)
        self.assertEqual(len(got), len(scen))
        seen = set()
        for s, g in zip(scen, got):
            with self.subTest(s.name):
                w = self.compare(s, g)
                st = [f["st"] for _, f in sorted(w["frames"].items()) if f["st"]]
                seen |= {x[16] for x in st}
                if s.name != "random pads":
                    self.assertEqual(st[-1][0], -1, f"{s.name}: the menu closes")
        v = 2 * (VOL - 1)
        for r in (0x1, 0x2, 0x4, 0xd, 0x19, 0x100a, 0x101c, 0x1022, 0x1026, 0x102a + v, 0x1032 + v, 0x2005, 0x2006,
                  0x2007, 0x2008, 0x2009, 0x201d, 0x2023, 0x800b, 0x801e, 0x801f, 0x802b + v, 0x8033 + v, 0x10020,
                  0x10024):
            self.assertIn(r, seen, f"no scenario reached result {r:#x}")

    def test_saved_clear_data(self):
        """The first scenario's save: the second file holds the save, its
        record the clear flag."""
        s = scenarios()[0]
        want = s.run_game()
        idx = bytes.fromhex(want["files"]["0:" + INDEX])
        rec = idx[28:56]
        self.assertEqual(rec[:4], bytes([1, 7, VOL, 0]), "used, level 7, the volume's clear flag")
        self.assertIn(f"0:{DIR}/dhdata02", want["files"])


if __name__ == "__main__":
    unittest.main()
