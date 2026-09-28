#!/usr/bin/env python3
"""crates/piney-fieldui's talk and shop menus (gcmn.prg menu.cpp: the lists
the action button opens on a PC and the merchants, 22 and 24-26, and the
shops' pages) against the game's own ccThMenu in eemu, frame by frame.

It is tools/test_fieldui_rs.py's comparison with the character spoken to
added: cmndTarget is a ccChar whose base is the real npcTbl row (gcmn
0x00619460, in the loaded overlay), the port is told the same row through
FieldUi::talk_to (the probe's `npc` command), rand() returns the same
values on both sides, and the state compared each frame grows the talk
state (talkNum, talkTradeFlag, talkLoopCnt, dummyTarget, the open list's
index, page, size and scroll, temp[8]) and chosen runs of the save (gold,
the item lists, the trade counts). What the menus ask of the world is
logged on both sides: changeCamera and SetMerchantCamera (EntryAffect is
the base's). The pages not ported yet close at once on both sides.

The base classes here (Npc, TalkScenario, TalkGame, TalkCase) are what the
other talk harnesses build on (test_fieldui_talk_rs.py,
test_fieldui_trade_rs.py, test_fieldui_record_rs.py).

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
import volume  # noqa: E402
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc
import test_fieldui_rs as base  # noqa: E402

eemu = base.eemu
P, sym = base.P, base.sym
OK, CANCEL, TRIANGLE, SQUARE = base.OK, base.CANCEL, base.TRIANGLE, base.SQUARE
UP, RIGHT, DOWN, LEFT = base.UP, base.RIGHT, base.DOWN, base.LEFT
R1, L1 = 0x8, 0x4

NPC_TBL = inf_va(0x00619460)
# The pages each list leads to, and the handlers not ported yet: they close
# their menu at once on both sides (the port's Request::Unported).
PAGES = {21: "SpcMenu", 23: "NpcMenu", 27: "BreederMenu", 47: "TalkMenu", 48: "TradeMenu", 49: "TradeSubMenu",
         50: "PresentMenu", 51: "SellMenu", 52: "BuyMenu", 53: "RecordMenu", 54: "ItemDepositMenu",
         55: "ItemDrawMenu", 56: "BreedingMenu"}
UNPORTED = ()
# Save runs worth watching.
GOLD = (0x7488 + 0x14, 4)             # spcParam[0].base.gold
ITEMS = (0x30, 160)                   # itemList[0]
PL_ITEMS = (0xB70, 396)               # plItemList (Elf's Haven)
TPC_TRADE_SW = (0x1E7C, 72)           # tpcTradeListSW[24][3]
PC_TRADE_COUNT = (0x686A, 77)         # pcTradeCount


def s16(va):
    return struct.unpack("<h", P.read(va, 2))[0]


def npc_row(row):
    """npcTbl[row]'s name, type and id."""
    a = NPC_TBL + 0x70 * row
    name = P.cstr(P.u32(a)) if P.u32(a) else b""
    return name, P.u32(a + 8), s16(a + 0xC)


class Npc(base.Char):
    """A town NPC as cmndTarget: its base is npcTbl[row] itself."""

    def __init__(self, handle, row, server=0, tag=(256, 200)):
        name, types, cid = npc_row(row)
        super().__init__(handle, types, cid, 0, 0, 0, 0, name, tag=tag, width=100.0)
        self.row, self.server = row, server

    def target_line(self):
        tx, ty = self.tag if self.tag else (-9999, 0)
        return f"npc {self.handle} {self.row} {tx} {ty}"


class TalkScenario(base.Scenario):
    def __init__(self, frames):
        super().__init__(frames)
        self.rands = []           # rand()'s values in turn, 0 after
        self.modes = {}           # frame -> ccMenu.mode set before it
        self.watches = []         # (offset, length) of the save to compare

    def talk(self, npc, f, menu, mode=1):
        """ccThGameCtrl's action button on `npc` before frame `f`: cmndTarget,
        openReqNum `menu`, mode 1, firstTime 1."""
        self.target = npc
        self.server = (npc.server, 0, 0)
        self.opens[f] = menu
        self.firsts.add(f)
        self.modes[f] = mode

    def lines(self):
        out = super().lines()
        run = out.pop()
        out.append("tk")
        out.append(" ".join(["rand"] + [str(v) for v in self.rands]))
        for f, v in sorted(self.modes.items()):
            out.append(f"mode {f} {v}")
        for off, n in self.watches:
            out.append(f"watch {off} {n}")
        out.append(run)
        return out


class TalkGame(base.Game):
    """The game's ccThMenu with an NPC spoken to, and the game's ccSaveSys
    (its static initialiser and constructor run, left at the scenario's port
    and file) with the ccThSaveSys task's MainProccess run at the start of
    each frame while StartReq's task lives, the memory cards answering in
    Python (tools/test_desktop_data_rs.py's Card; none unless the scenario
    sets them)."""

    unported = UNPORTED

    def __init__(self, sc):
        import test_desktop_data_rs as dd
        self.card = dd.Card()
        for port, present, ps2, formatted, full, fw, fs, ff in getattr(sc, "cards", []):
            self.card.ports[port].update(present=present, ps2=ps2, formatted=formatted, full=full, failwrite=fw,
                                         failsys=fs, failfmt=ff)
        for p in getattr(sc, "card_dirs", []):
            self.card.ports[p]["dir"] = 1
        for p, name, b in getattr(sc, "card_files", []):
            self.card.ports[p]["files"][f"/{CARD_DIR}/{name}"] = bytearray(b)
        self.in_task = False
        self.sys = 0
        super().__init__(sc)

    def make_char(self, c):
        if not isinstance(c, Npc):
            a = super().make_char(c)
            if c.id == 0:
                # Kite's base is the save's spcParam[0].base: its gold is
                # the save's.
                m, sb = self.m, base.SAVE + 0x7488
                old = m.load(a, 4)
                for o in (0x08, 0x0C, 0x18, 0x1C):
                    m.store(sb + o, 4, m.load(old + o, 4)) if o != 0x0C else m.store(sb + o, 2, m.load(old + o, 2))
                m.store(a, 4, sb)
            return a
        m = self.m
        a = self.alloc(0x200)
        m.store(a + 0x00, 4, NPC_TBL + 0x70 * c.row)
        m.store(a + 0x04, 4, self.alloc(0x100))
        m.store(a + 0x40, 4, base.fb(float(c.handle)))
        m.store(a + 0x5C, 4, base.fb(1.0))
        self.chars[a] = c
        return a

    def setup(self):
        super().setup()
        m = self.m
        self.call(sym("__sinit_sdmng.cpp"), [])
        self.sys = self.alloc(0x300)
        self.call(sym("__ct__9ccSaveSysFv"), [self.sys])
        pos = getattr(self.sc, "card_pos", (0, 0))
        m.store(self.sys + 0x2A0, 4, pos[0])
        m.store(self.sys + 0x2A4, 4, pos[1])
        m.store(sym("saveSys"), 4, self.sys)
        self.sc.savebytes = bytes(m.mem[base.SAVE:base.SAVE + SLOT_SIZE])

    def breath(self, n):
        if self.in_task:
            return 0
        return super().breath(n)

    def icon_bin(self):
        if not hasattr(self, "_icons"):
            with open(os.path.join(os.path.dirname(base.ELF), "DATA", "ICON.BIN"), "rb") as f:
                self._icons = f.read()
        return self._icons

    def hooks(self):
        super().hooks()
        m, h = self.m, self.m.hooks
        rands = list(self.sc.rands)
        for n, f in self.card.hooks().items():
            h[sym(n)] = f
        # MakeDir copies the icons from DATA/ICON.BIN, read as the desktop
        # harness reads it.
        alloc = lambda mm, n, *a: self.alloc(n)  # noqa: E731
        for n in ("ccMalloc__FUi", "ccMallocB__FUi", "__nwa__FUi"):
            h[sym(n)] = alloc
        h[sym("ccFree__FPv")] = lambda mm, *a: 0
        h[sym("__dl__FPv")] = lambda mm, *a: 0

        def cd_search(mm, fp, name, *a):
            mm.store(fp, 4, 0)
            mm.store(fp + 4, 4, len(self.icon_bin()))
            return 1

        def cd_read(mm, this, lsn, sectors, buf, *a):
            data = self.icon_bin()[2048 * lsn:2048 * (lsn + sectors)]
            mm.mem[buf:buf + len(data)] = data
            return 1
        h[sym("sceCdSearchFile")] = cd_search
        h[sym("Read2__6ccCdvdFUiUiPv")] = cd_read

        def rand(mm, *a):
            return rands.pop(0) if rands else 0
        h[sym("rand")] = rand

        sx = lambda v: v - (1 << 32) if v & 0x80000000 else v  # noqa: E731
        h[sym("ccEvVoiceRequest__Fii")] = lambda mm, g, n, *a: self.ev("voice", sx(g), sx(n)) or 0
        h[sym("changeCamera__Fi")] = lambda mm, n, *a: self.ev("camera", n) or 0
        h[sym("SetMerchantCamera__10ccMenuCtrlFv")] = lambda mm, *a: self.ev("merchant_camera") or 0

        def unported(n):
            def fn(mm, this, *a):
                self.ev("unported", n)
                self.call(sym("CloseMenu__10ccMenuCtrlFv"), [this])
                return 0
            return fn
        for n in self.unported:
            h[sym(PAGES[n] + "__10ccMenuCtrlFv")] = unported(n)

        # A party member given something to wear (AddSpcItem, from Trade
        # and Gift): ccChar::CalcReal runs as the game has it, noted, and
        # ccSPC::CheckSpc's slot is the member's id (the runtime's to find).
        calc = sym("CalcReal__6ccCharFi")

        def calc_real(mm, c, flag, *a):
            self.ev("calc_real", self.handle(c))
            del h[calc]
            try:
                self.call(calc, [c, flag])
            finally:
                h[calc] = calc_real
            return 0
        h[calc] = calc_real
        h[sym("CheckSpc__5ccSPCFi")] = lambda mm, this, sid, *a: sid

    def run_threads(self):
        # ccThEquipMenu (gcmn 0x00538d80): ccSPC::ChangeEquip(cat,
        # CheckSpc(member), id) on the member spoken to (cmndTargetPrev: the
        # pages dropped it), then its flag down, at the menu's breath.
        m = self.m
        for t, fn in self.threads:
            if fn == sym("ccThEquipMenu__FP6ccTscb") and m.load(t + 20, 4):
                who = self.handle(m.load(base.CMNDTARGETPREV, 4))
                self.ev("change_equip", who, m.load(t + 24, 4, True), m.load(t + 32, 4, True))
                m.store(t + 20, 4, 0)
        super().run_threads()

    def start_frame(self):
        super().start_frame()
        c = self.menu()
        if c and self.frame in self.sc.modes:
            self.m.store(c + 0x16, 2, self.sc.modes[self.frame])
        if self.sys and self.m.load(self.sys + 0x2B4, 4):
            self.in_task = True
            try:
                self.call(sym("MainProccess__9ccSaveSysFv"), [self.sys])
            finally:
                self.in_task = False

    def state(self):
        st, ms = super().state()
        if st is None:
            return st, ms
        m, c = self.m, self.menu()
        g = lambda o: m.load(c + o, 2, True)  # noqa: E731
        menu = g(6)
        lst = inf_va(0x0072EFE0) + 32 * max(0, min(menu, 88))
        L = lambda o: m.load(lst + o, 2, True)  # noqa: E731
        st = st + [g(0xF6), g(0xFA), g(0xFC), self.handle(m.load(c + 0x234, 4)), L(0x10), L(0xC), L(0xE), L(0x14),
                   L(0x16), L(0x1C), L(0x1E), L(0x18), L(0x1A), L(0x12)]
        st += [m.load(c + 0x18C + 4 * k, 4, True) for k in range(8)]
        st += [m.mem[base.SAVE + off:base.SAVE + off + n].hex() for off, n in self.sc.watches]
        return st, ms


class TalkCase(unittest.TestCase):
    """compare() as test_fieldui_rs.py's, over TalkGame."""

    game = TalkGame

    @classmethod
    def setUpClass(cls):
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-fieldui", "--features",
                        "trace", "--example", "fieldui_probe"], cwd=base.ROOT, check=True)

    def compare(self, sc, what):
        g = self.game(sc).run()
        p = base.norm_port(base.port(sc))
        self.check(g, p, sc, what)

    def check(self, g, p, sc, what):
        """Every frame of the game's run `g` against the port's `p`."""
        close = base.close
        for f in range(1, sc.frames):
            a, b = g.get(f), p.get(f)
            self.assertIsNotNone(a, f"{what}: no game frame {f}")
            self.assertIsNotNone(b, f"{what}: no port frame {f}")
            self.assertEqual(a["st"], b["st"], f"{what} frame {f}: state")
            self.assertEqual(a["msg"], b["msg"], f"{what} frame {f}: message state")
            key = lambda e: json.dumps(e)  # noqa: E731
            ga = sorted((e for e in a["ev"] if e[0] != "map_alpha"), key=key)
            pa = sorted((e for e in b["ev"] if e[0] != "map_alpha"), key=key)
            self.assertEqual(ga, pa, f"{what} frame {f}: events")
            gm = [e for e in a["ev"] if e[0] == "map_alpha"]
            pm = [e for e in b["ev"] if e[0] == "map_alpha"]
            self.assertTrue(close(gm, pm), f"{what} frame {f}: map alpha {gm} vs {pm}")
            for obj in sorted({x[0] for x in a["pk"]} | {x[0] for x in b["pk"]}):
                ga = [x for x in a["pk"] if x[0] == obj]
                pa = [x for x in b["pk"] if x[0] == obj]
                self.assertEqual(len(ga), len(pa), f"{what} frame {f}: {obj} packets\n{ga}\n{pa}")
                for x, y in zip(ga, pa):
                    self.assertTrue(close(x, y), f"{what} frame {f}: {obj}\n{x}\n{y}")
            self.assertEqual(sorted(a["kanji"]), sorted(b["kanji"]), f"{what} frame {f}: kanji")
            self.assertTrue(close(a["mc"], b["mc"]), f"{what} frame {f}: message cells\n{a['mc']}\n{b['mc']}")
            self.assertTrue(close(a["mt"], b["mt"]), f"{what} frame {f}: message texts\n{a['mt']}\n{b['mt']}")

    def kite(self, sc, gold=0):
        """Kite alone in Mac Anu with `gold`."""
        sc.party[0] = base.Char(0x100, 1, 0, 120, 60, 120, 60, b"Kite")
        sc.game = (0, 0, 0, 0)
        sc.saves.append((GOLD[0], 4, gold))

    def items(self, sc, entries, pl=False):
        """Kite's bag (or, pl, the storage) holding (category, id, count)s."""
        at, n = (PL_ITEMS[0], 99) if pl else (ITEMS[0], 40)
        for k, (cat, iid, num) in enumerate(entries):
            sc.saves += [(at + 4 * k, 2, iid), (at + 4 * k + 2, 1, cat & 0xFF), (at + 4 * k + 3, 1, num)]
        for k in range(len(entries), n):
            sc.saves += [(at + 4 * k, 2, 0xFFFF), (at + 4 * k + 2, 1, 0xFF), (at + 4 * k + 3, 1, 0)]


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class MerchantLists(TalkCase):
    """The lists the action button opens: 22, 24, 25, 26."""

    def test_merchants(self):
        # Each Mac Anu merchant (weapons, Elf's Haven, items, magic, the
        # Recorder): greeted, the cursor through the rows, cancel.
        for row, menu in ((0, 24), (1, 26), (2, 24), (3, 24), (4, 25)):
            sc = TalkScenario(80)
            self.kite(sc)
            sc.talk(Npc(0x200, row), 10, menu)
            sc.pads.update({30: (0, DOWN), 38: (0, DOWN), 46: (0, UP), 60: (CANCEL, 0)})
            self.compare(sc, f"merchant {row}")

    def test_merchant_pages(self):
        # OK on each row: Talk keeps the target; the others put the tasks
        # to sleep, drop the target and (Vender) keep the shop's type.
        for row, menu, rows in ((0, 24, 3), (1, 26, 3), (4, 25, 2)):
            for sel in range(rows):
                sc = TalkScenario(70)
                self.kite(sc)
                sc.talk(Npc(0x200, row), 10, menu)
                for k in range(sel):
                    sc.pads[25 + 8 * k] = (0, DOWN)
                sc.pads[50] = (OK, 0)
                self.compare(sc, f"merchant {row} row {sel}")

    def test_merchant_server(self):
        # Another server's greeting and the list opened without firstTime.
        sc = TalkScenario(90)
        self.kite(sc)
        sc.talk(Npc(0x200, 2, server=3), 10, 24)
        sc.pads[40] = (CANCEL, 0)
        sc.opens[60] = 24
        sc.modes[60] = 1
        sc.pads[80] = (CANCEL, 0)
        self.compare(sc, "merchant server 3")

    def test_pcs(self):
        # A PC with random lines (30-65), the trading PCs (66-79) with their
        # open trades, and one past them (80 on: its table as the record).
        for row, rands, sw in ((30, [5], None), (45, [7], None), (66, [4], (0, 1, 1)), (70, [2], (0, 0, 0)),
                               (79, [0], (1, 0, 0)), (80, [], None), (100, [], None)):
            sc = TalkScenario(70)
            self.kite(sc)
            sc.rands = rands
            if sw is not None:
                for k, v in enumerate(sw):
                    sc.saves.append((TPC_TRADE_SW[0] + 3 * (row - 66) + k, 1, v))
            for k in range(77):
                sc.saves.append((PC_TRADE_COUNT[0] + k, 1, 0xFF))
            sc.watches = [TPC_TRADE_SW, PC_TRADE_COUNT]
            sc.talk(Npc(0x200, row), 10, 22)
            sc.pads.update({30: (0, DOWN), 40: (0, UP), 55: (CANCEL, 0)})
            self.compare(sc, f"pc {row}")

    def test_pc_pages(self):
        # Talk and Trade from a PC; a battle starting under the list.
        for row, sel in ((30, 0), (30, 1), (66, 1)):
            sc = TalkScenario(60)
            self.kite(sc)
            sc.rands = [1]
            sc.talk(Npc(0x200, row), 10, 22)
            if sel:
                sc.pads[25] = (0, DOWN)
            sc.pads[40] = (OK, 0)
            self.compare(sc, f"pc {row} row {sel}")
        sc = TalkScenario(50)
        self.kite(sc)
        sc.talk(Npc(0x200, 31), 10, 22)
        sc.gamefs[30] = (0, 1, 0)
        self.compare(sc, "pc battle")

    def test_random_lists(self):
        for seed in range(1, 7):
            rnd = random.Random(seed)
            sc = TalkScenario(300)
            self.kite(sc)
            row, menu = rnd.choice(((0, 24), (1, 26), (2, 24), (3, 24), (4, 25), (30, 22), (66, 22), (50, 22)))
            sc.rands = [rnd.randrange(100) for _ in range(4)]
            sc.talk(Npc(0x200, row), 10, menu)
            for f in range(60, 300, 60):
                sc.opens[f] = menu
                sc.modes[f] = 1
                if rnd.random() < 0.5:
                    sc.firsts.add(f)
            for f in range(12, 300):
                r = rnd.random()
                if r < 0.15:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN)))
                elif r < 0.2:
                    sc.pads[f] = (rnd.choice((OK, CANCEL)), 0)
            self.compare(sc, f"random list {seed}")


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class TalkPages(TalkCase):
    """Talk (47) from the lists, and on the characters the lists do not
    reach (opened directly)."""

    def talk_run(self, sc, start, oks, cancel=True):
        """Talk chosen at `start`, then OK every 12 frames `oks` times, then
        cancel out of the list."""
        sc.pads[start] = (OK, 0)
        f = start
        for _ in range(oks):
            f += 12
            sc.pads[f] = (OK, 0)
        if cancel:
            sc.pads[f + 25] = (CANCEL, 0)

    def test_merchant_talk(self):
        # Each merchant's Talk: its second record, chained (emode 1) to the
        # third; then the list again.
        for row, menu in ((0, 24), (1, 26), (2, 24), (3, 24), (4, 25)):
            sc = TalkScenario(160)
            self.kite(sc)
            sc.talk(Npc(0x200, row), 10, menu)
            self.talk_run(sc, 30, 8)
            self.compare(sc, f"merchant {row} talk")

    def test_pc_talk(self):
        # A PC's lines round the three talkNums, then msg[1] after three
        # talks; the story's talkNum[0] shifting the table.
        for row, story in ((30, 0), (31, 1), (47, 0), (65, 2)):
            sc = TalkScenario(420)
            self.kite(sc)
            sc.saves.append((0x220C, 1, story))
            sc.rands = [2]
            sc.talk(Npc(0x200, row), 10, 22)
            f = 30
            for _ in range(5):
                sc.pads[f] = (OK, 0)
                for k in range(1, 7):
                    sc.pads[f + 12 * k] = (OK, 0)
                f += 80
            sc.pads[f] = (CANCEL, 0)
            self.compare(sc, f"pc {row} talk")

    def test_trading_pc_talk(self):
        # The trading PCs' offers: one, two and three things offered, the
        # next open trade each talk, none left.
        for row, sw in ((66, (1, 1, 1)), (67, (0, 1, 0)), (72, (1, 0, 1)), (75, (0, 0, 0)), (79, (1, 1, 0))):
            sc = TalkScenario(330)
            self.kite(sc)
            sc.rands = [1]
            for k, v in enumerate(sw):
                sc.saves.append((TPC_TRADE_SW[0] + 3 * (row - 66) + k, 1, v))
            sc.talk(Npc(0x200, row), 10, 22)
            f = 30
            for _ in range(3):
                sc.pads[f] = (OK, 0)
                for k in range(1, 6):
                    sc.pads[f + 12 * k] = (OK, 0)
                f += 90
            sc.pads[f] = (CANCEL, 0)
            self.compare(sc, f"trading pc {row} talk")

    def test_other_talk(self):
        # Opened directly on the characters no list here leads from: the
        # administrator, a PC past the table's end, a Grunty (its voice
        # group), a breeder.
        for row, tn in ((29, 0), (80, 1), (100, 0), (141, 0), (150, 1), (157, 2), (10, 1)):
            sc = TalkScenario(120)
            self.kite(sc)
            sc.target = Npc(0x200, row)
            sc.opens[10] = 47
            sc.modes[10] = 1
            sc.firsts.add(10)
            for k in range(8):
                sc.pads[25 + 12 * k] = (OK, 0)
            self.compare(sc, f"talk row {row}")

    def test_talk_lost_target(self):
        sc = TalkScenario(80)
        self.kite(sc)
        sc.talk(Npc(0x200, 0), 10, 24)
        sc.pads[30] = (OK, 0)
        self.compare(sc, "talk")


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class BuyPages(TalkCase):
    """Buy (52) from the weapon, item and magic shops."""

    def shop(self, row, gold, frames, server=0):
        sc = TalkScenario(frames)
        self.kite(sc, gold)
        self.items(sc, [(10, 0, 3), (10, 3, 1), (6, 2, 1)])
        sc.watches = [GOLD, ITEMS]
        sc.talk(Npc(0x200, row, server=server), 10, 24)
        sc.pads.update({25: (0, DOWN), 35: (OK, 0)})
        return sc

    def test_buy(self):
        # Scroll the stock, a count up, left and right, OK, the question's
        # OK: bought; again to Cancel; then out.
        for row in (0, 2, 3):
            sc = self.shop(row, 5000, 330)
            sc.pads.update({60: (0, DOWN), 68: (0, DOWN), 76: (0, UP), 90: (OK, 0), 100: (0, UP), 106: (0, UP),
                            112: (0, LEFT), 118: (0, RIGHT), 124: (0, DOWN), 136: (OK, 0), 160: (OK, 0),
                            200: (0, DOWN), 210: (OK, 0), 222: (OK, 0), 245: (0, DOWN), 252: (OK, 0),
                            285: (CANCEL, 0), 310: (CANCEL, 0)})
            self.compare(sc, f"buy {row}")

    def test_buy_scroll(self):
        # Every row of the weapon shop on another server (22 items).
        sc = self.shop(0, 100, 300, server=2)
        for k in range(26):
            sc.pads[60 + 6 * k] = (0, DOWN)
        for k in range(10):
            sc.pads[220 + 6 * k] = (0, UP)
        sc.pads[285] = (CANCEL, 0)
        self.compare(sc, "buy scroll")

    def test_buy_refusals(self):
        # Short of gold; a full bag; 99 carried; each refusal's window.
        full = [(10, k, 1) for k in range(5, 45)]
        for what, gold, bag in (("gold", 10, [(10, 0, 3)]), ("full", 90000, full), ("99", 90000, [(10, 0, 99)])):
            sc = TalkScenario(200)
            self.kite(sc, gold)
            self.items(sc, bag)
            sc.watches = [GOLD, ITEMS]
            sc.talk(Npc(0x200, 2), 10, 24)
            sc.pads.update({25: (0, DOWN), 35: (OK, 0), 60: (OK, 0), 70: (0, UP), 80: (OK, 0), 110: (OK, 0),
                            125: (OK, 0), 170: (CANCEL, 0)})
            self.compare(sc, f"buy refusal {what}")

    def test_buy_cancels(self):
        # Cancel the count; the question's cancel; triangle on a weapon.
        sc = self.shop(0, 5000, 260)
        sc.pads.update({60: (OK, 0), 75: (CANCEL, 0), 90: (OK, 0), 100: (OK, 0), 125: (CANCEL, 0),
                        160: (0, DOWN), 170: (TRIANGLE, 0)})
        self.compare(sc, "buy cancels")

    def test_random_buy(self):
        for seed in range(1, 9):
            rnd = random.Random(seed)
            sc = self.shop(rnd.choice((0, 2, 3)), rnd.choice((0, 150, 3000, 9999999)), 420,
                           server=rnd.randrange(5))
            if rnd.random() < 0.3:
                self.items(sc, [(10, k, rnd.randrange(1, 100)) for k in range(rnd.randrange(30, 41))])
            for f in range(50, 420):
                r = rnd.random()
                if r < 0.2:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN, LEFT, RIGHT)))
                elif r < 0.27:
                    sc.pads[f] = (OK, 0)
                elif r < 0.29:
                    sc.pads[f] = (CANCEL, 0)
                elif r < 0.295:
                    sc.pads[f] = (TRIANGLE, 0)
            self.compare(sc, f"random buy {seed}")


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class SellPages(TalkCase):
    """Sell (51): the bag's pages sold at half price."""

    BAG = [(10, 0, 5), (10, 3, 2), (10, 18, 1), (13, 1, 3), (11, 0, 2), (12, 0, 1), (14, 3, 1), (0, 2, 1),
           (6, 5, 1), (7, 3, 2), (15, 44, 1)]

    def shop(self, gold, frames, bag=BAG):
        sc = TalkScenario(frames)
        self.kite(sc, gold)
        self.items(sc, bag)
        sc.watches = [GOLD, ITEMS]
        sc.talk(Npc(0x200, 2), 10, 24)
        sc.pads.update({25: (0, DOWN), 31: (0, DOWN), 40: (OK, 0)})
        return sc

    def test_sell(self):
        # Sell 3 of 5, count up, down, left, right; again all 2; the pages.
        sc = self.shop(100, 330)
        sc.pads.update({65: (OK, 0), 72: (0, UP), 78: (0, UP), 84: (0, UP), 90: (0, DOWN), 96: (0, LEFT),
                        102: (0, RIGHT), 108: (0, UP), 120: (OK, 0), 145: (OK, 0), 180: (0, DOWN), 188: (OK, 0),
                        195: (0, LEFT), 205: (OK, 0), 230: (OK, 0), 265: (0, R1), 272: (0, R1), 279: (0, R1),
                        286: (0, R1), 293: (0, L1), 310: (CANCEL, 0)})
        self.compare(sc, "sell")

    def test_sell_cancels(self):
        # Cancel the count; Cancel in the question; triangle on a weapon;
        # an empty page.
        sc = self.shop(9999990, 260)
        sc.pads.update({65: (OK, 0), 80: (CANCEL, 0), 95: (OK, 0), 105: (OK, 0), 125: (0, DOWN), 132: (OK, 0),
                        170: (0, R1), 180: (OK, 0), 190: (0, L1), 200: (0, L1), 215: (TRIANGLE, 0)})
        self.compare(sc, "sell cancels")

    def test_sell_all_gold(self):
        # Selling past the gold cap (9999999); the last of a stack.
        sc = self.shop(9999900, 200, bag=[(10, 18, 1), (0, 70, 1)])
        sc.pads.update({65: (OK, 0), 80: (OK, 0), 105: (OK, 0), 140: (0, L1), 150: (OK, 0), 160: (OK, 0),
                        185: (OK, 0)})
        self.compare(sc, "sell all")

    def test_random_sell(self):
        for seed in range(1, 9):
            rnd = random.Random(seed)
            bag = rnd.sample(self.BAG, rnd.randrange(0, len(self.BAG) + 1))
            sc = self.shop(rnd.choice((0, 500, 9999000)), 420, bag=bag)
            for f in range(50, 420):
                r = rnd.random()
                if r < 0.2:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN, LEFT, RIGHT, R1, L1)))
                elif r < 0.27:
                    sc.pads[f] = (OK, 0)
                elif r < 0.29:
                    sc.pads[f] = (CANCEL, 0)
                elif r < 0.295:
                    sc.pads[f] = (TRIANGLE, 0)
            self.compare(sc, f"random sell {seed}")


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class HavenPages(TalkCase):
    """Elf's Haven: Store Items (54) and Withdraw Items (55)."""

    BAG = [(10, 0, 5), (10, 3, 2), (13, 1, 3), (11, 0, 2), (12, 0, 1), (14, 3, 1), (0, 2, 1), (6, 5, 1)]
    BOX = [(10, 0, 97), (10, 5, 4), (11, 2, 1), (0, 7, 1), (7, 3, 2), (14, 1, 1)]

    def haven(self, row, frames, bag=BAG, box=BOX):
        sc = TalkScenario(frames)
        self.kite(sc)
        self.items(sc, bag)
        self.items(sc, box, pl=True)
        sc.watches = [ITEMS, PL_ITEMS]
        sc.talk(Npc(0x200, 1), 10, 26)
        for k in range(row):
            sc.pads[25 + 6 * k] = (0, DOWN)
        sc.pads[40] = (OK, 0)
        return sc

    def test_store(self):
        # Store 2 of the first (97 there: at most 2), then another whole;
        # the pages.
        sc = self.haven(1, 330)
        sc.pads.update({65: (OK, 0), 72: (0, UP), 78: (0, UP), 84: (0, DOWN), 90: (0, LEFT), 96: (0, RIGHT),
                        102: (0, UP), 110: (OK, 0), 135: (OK, 0), 170: (0, DOWN), 178: (OK, 0), 186: (0, LEFT),
                        195: (OK, 0), 220: (OK, 0), 255: (0, R1), 262: (0, R1), 269: (0, R1), 276: (0, R1),
                        283: (0, R1), 310: (CANCEL, 0)})
        self.compare(sc, "store")

    def test_withdraw(self):
        sc = self.haven(2, 330)
        sc.pads.update({65: (0, DOWN), 72: (OK, 0), 80: (0, UP), 86: (0, LEFT), 95: (OK, 0), 120: (OK, 0),
                        160: (OK, 0), 170: (0, UP), 180: (OK, 0), 205: (OK, 0), 245: (0, R1), 252: (0, R1),
                        260: (0, L1), 267: (0, L1), 274: (0, L1), 310: (CANCEL, 0)})
        self.compare(sc, "withdraw")

    def test_haven_refusals(self):
        # The storage full (99 kinds), 99 of the item there; the bag full,
        # 99 carried; each refusal's window; Cancel in the question.
        full_box = [(11, k, 1) for k in range(40)] + [(0, k, 1) for k in range(40)] + [(7, k, 1) for k in range(19)]
        for what, row, bag, box in (("box full", 1, self.BAG, full_box), ("99 stored", 1, [(10, 0, 5)], [(10, 0, 99)]),
                                    ("bag full", 2, [(10, k, 1) for k in range(40)], [(11, 2, 1)]),
                                    ("99 carried", 2, [(10, 0, 99)], [(10, 0, 5)])):
            sc = self.haven(row, 220, bag=bag, box=box)
            sc.pads.update({65: (OK, 0), 95: (OK, 0), 125: (OK, 0), 140: (OK, 0), 165: (0, DOWN), 172: (OK, 0),
                            200: (CANCEL, 0)})
            self.compare(sc, f"haven refusal {what}")

    def test_haven_cancels(self):
        for row in (1, 2):
            sc = self.haven(row, 220)
            sc.pads.update({65: (OK, 0), 75: (CANCEL, 0), 90: (OK, 0), 100: (OK, 0), 115: (0, DOWN), 122: (OK, 0),
                            150: (0, R1), 158: (0, R1), 166: (0, R1), 174: (0, R1), 182: (TRIANGLE, 0)})
            self.compare(sc, f"haven cancels {row}")

    def test_random_haven(self):
        for seed in range(1, 11):
            rnd = random.Random(seed)
            bag = rnd.sample(self.BAG, rnd.randrange(0, len(self.BAG) + 1))
            box = rnd.sample(self.BOX, rnd.randrange(0, len(self.BOX) + 1))
            sc = self.haven(rnd.choice((1, 2)), 420, bag=bag, box=box)
            for f in range(50, 420):
                r = rnd.random()
                if r < 0.2:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN, LEFT, RIGHT, R1, L1)))
                elif r < 0.27:
                    sc.pads[f] = (OK, 0)
                elif r < 0.29:
                    sc.pads[f] = (CANCEL, 0)
                elif r < 0.295:
                    sc.pads[f] = (TRIANGLE, 0)
            self.compare(sc, f"random haven {seed}")


SLOT_SIZE = volume.SLOT_SIZE
CARD_DIR = volume.card_dir()


class RecordScenario(TalkScenario):
    """A talk scenario with memory cards (tools/test_desktop_data_rs.py's
    Card rules on both sides) and ccSaveSys's position."""

    def __init__(self, frames):
        super().__init__(frames)
        self.cards = []           # (port, present, ps2, formatted, full, failwrite, failsys, failfmt)
        self.card_dirs = []
        self.card_files = []      # (port, name, bytes)
        self.card_pos = (0, 0)
        self.savebytes = None     # the game's ccSaveData after set-up, for the probe

    def card(self, port, present=1, ps2=1, formatted=1, full=0, failwrite=0, failsys=0, failfmt=0):
        self.cards.append((port, present, ps2, formatted, full, failwrite, failsys, failfmt))
        return self

    def prepared(self, port, records=()):
        """A card as MakeDir leaves it, the index holding `records`."""
        self.card_dirs.append(port)
        idx = bytearray(336)
        for slot, rec in records:
            idx[28 * slot:28 * slot + 28] = rec
        self.card_files.append((port, CARD_DIR, bytes(idx)))
        for k in range(12):
            self.card_files.append((port, f"dhdata{k + 1:02d}", bytes(SLOT_SIZE)))
        return self

    def lines(self):
        out = super().lines()
        run = out.pop()
        for c in self.cards:
            out.append("card " + " ".join(str(v) for v in c))
        for p in self.card_dirs:
            out.append(f"dir {p}")
        for p, name, b in self.card_files:
            out.append(f"file {p} {name} {base.hx(b)}")
        out.append("cardpos %d %d" % self.card_pos)
        if self.savebytes is not None:
            out.append("savebytes " + self.savebytes.hex())
        out.append(run)
        return out


# Kept for the other harnesses: TalkGame has the Recorder's machinery.
RecordGame = TalkGame


def record(status=1, level=7, clear=0, parody=0, name=b"Kite", sum_=0x1234, playtime=0):
    """An index record (ccSaveDataInfo, 28 bytes)."""
    r = bytearray(28)
    r[0:4] = bytes([status & 0xFF, level & 0xFF, clear & 0xFF, parody & 0xFF])
    r[4:4 + len(name)] = name
    r[0x16:0x18] = struct.pack("<H", sum_)
    r[0x18:0x1C] = struct.pack("<i", playtime)
    return bytes(r)


def hms(h, m, s):
    return ((h * 60 + m) * 60 + s) * 60


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class RecordPages(TalkCase):
    """The Recorder's Save (53) over ccSaveSys and the memory cards; the
    cards' files compared at the end."""

    def compare(self, sc, what):
        game = self.game(sc)
        g = game.run()
        p = subprocess.run([base.EXAMPLE, base.ISO], input="\n".join(sc.lines()) + "\n", capture_output=True,
                           text=True, check=True, cwd=base.ROOT)
        rows = [json.loads(line) for line in p.stdout.splitlines()]
        files = next((r["files"] for r in rows if "files" in r), {})
        self.check(g, base.norm_port([r for r in rows if "f" in r]), sc, what)
        self.assertEqual(game.card.summary(), files, f"{what}: the cards' files")

    def recorder(self, frames, gold=1234):
        sc = RecordScenario(frames)
        self.kite(sc, gold)
        self.items(sc, [(10, 0, 3), (0, 2, 1)])
        sc.saves += [(0x7488 + 0x0E, 2, 12), (0x8400, 4, hms(3, 4, 5))]
        sc.talk(Npc(0x200, 4), 10, 25)
        sc.pads.update({25: (0, DOWN), 35: (OK, 0)})
        return sc

    def test_save_new(self):
        # Card slot 1, the fourth file (empty): saved, the index read again.
        sc = self.recorder(300)
        sc.card(0).prepared(0, [(0, record(name=b"Old", playtime=hms(1, 2, 3)))])
        sc.pads.update({60: (OK, 0), 90: (0, DOWN), 96: (0, DOWN), 102: (0, DOWN), 110: (OK, 0)})
        self.yes_ok(sc, 125, 250)
        sc.pads.update({260: (CANCEL, 0), 275: (CANCEL, 0)})
        self.compare(sc, "save new")

    @staticmethod
    def yes_ok(sc, start, end, step=20):
        """Up (to YES) every 4 frames, OK every `step`."""
        for f in range(start, end, 4):
            sc.pads[f] = (0, UP)
        for f in range(start + 2, end, step):
            sc.pads[f] = (OK, 0)

    def test_save_overwrite(self):
        # Over a used file: the question, NO then YES.
        recs = [(0, record(level=20, clear=1, name=b"Orca", playtime=hms(999, 59, 59))),
                (1, record(level=99, parody=1, name=b"Balmung", playtime=hms(12, 5, 9))),
                (2, record(level=5, name=b"ABCDEFGHIJKLMNOPQ", playtime=hms(1, 0, 0)))]
        sc = self.recorder(360)
        sc.card(0).prepared(0, recs)
        sc.card_pos = (0, 1)
        sc.pads.update({60: (OK, 0), 95: (OK, 0), 120: (0, DOWN), 130: (OK, 0), 160: (OK, 0), 180: (OK, 0)})
        self.yes_ok(sc, 200, 330)
        sc.pads.update({335: (CANCEL, 0), 348: (CANCEL, 0)})
        self.compare(sc, "save overwrite")

    def test_no_card(self):
        # No card; slot 2 not a PS2 card; OK through the errors; cancel.
        sc = self.recorder(220)
        sc.card(1, ps2=0)
        sc.pads.update({60: (OK, 0), 90: (OK, 0), 110: (OK, 0), 130: (0, DOWN), 140: (OK, 0), 170: (OK, 0),
                        190: (OK, 0), 205: (CANCEL, 0)})
        self.compare(sc, "no card")

    def test_new_directory(self):
        # A card without the save directory: made, then saved.
        sc = self.recorder(360)
        sc.card(0)
        sc.pads.update({60: (OK, 0), 75: (OK, 0), 92: (0, UP), 98: (OK, 0)})
        for f in range(110, 340, 6):
            sc.pads[f] = (OK, 0)
        sc.pads[345] = (CANCEL, 0)
        self.compare(sc, "new directory")

    def test_failures(self):
        # The slot write failing; the index write failing.
        for what, flags in (("write", dict(failwrite=1)), ("index", dict(failsys=1))):
            sc = self.recorder(300)
            sc.card(0, **flags).prepared(0)
            sc.pads[60] = (OK, 0)
            self.yes_ok(sc, 75, 280, 16)
            sc.pads[290] = (CANCEL, 0)
            self.compare(sc, f"failure {what}")

    def test_random_record(self):
        for seed in range(1, 7):
            rnd = random.Random(seed)
            sc = self.recorder(420)
            port = rnd.randrange(2)
            flags = rnd.choice((dict(), dict(failwrite=1), dict(formatted=0), dict(full=1), dict(present=0)))
            sc.card(port, **flags)
            if rnd.random() < 0.7:
                sc.prepared(port, [(k, record(name=b"Rnd%d" % k, level=k + 1)) for k in range(rnd.randrange(12))])
            sc.card_pos = (rnd.randrange(2), rnd.randrange(12))
            for f in range(50, 420):
                r = rnd.random()
                if r < 0.12:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN)))
                elif r < 0.22:
                    sc.pads[f] = (OK, 0)
                elif r < 0.25:
                    sc.pads[f] = (CANCEL, 0)
            self.compare(sc, f"random record {seed}")


if __name__ == "__main__":
    unittest.main()
