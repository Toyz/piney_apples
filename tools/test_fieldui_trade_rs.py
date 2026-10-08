#!/usr/bin/env python3
"""crates/piney-fieldui's trade pages (gcmn.prg menu.cpp: TradeMenu 48,
TradeSubMenu 49, TradeMenuDisp, AddSpcItem) against the game's own
ccThMenu in eemu, frame by frame, over tools/test_fieldui_shop_rs.py's talk
base.

The trade is reached as in the game: the action button on a walking PC
opens its list (PcMenu, 22), Trade drops the target (cmndTargetPrev is the
PC) and goes into 48. The save holds the trade lists as a new game and a
town leave them: ccSaveData::InitTradeItem and SetTradeItemTown (main
0x00176610, 0x001767a0) are run natively in eemu first (rand() from a
seeded generator) and their bytes written into the scenario's save on both
sides. The state compared each frame grows drainItem[0..4] (+0x1ac, the
items traded) after temp[8]; the save runs watched are the bag, the trade
lists and their switches, the trade counts, and (for a party member) its
spcParam and skills.

A party member's trade is reached here through PcMenu opened without
firstTime on the member (SpcMenu's own way in is
tools/test_fieldui_talk_rs.py's): its base and personality are its spcParam
in the save (ccChar::personality points there), so AddSpcItem's equipment,
class and friendship are the save's. CalcReal runs as the game has it and
its thread (ccThEquipMenu) is noted at the menu's breath, as the shop
harness does.

Skipped when the disc is not extracted or cargo is missing.
"""

import os
import random
import shutil
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import test_fieldui_rs as base  # noqa: E402
import test_fieldui_shop_rs as shop  # noqa: E402

P, sym = base.P, base.sym
OK, CANCEL, TRIANGLE = base.OK, base.CANCEL, base.TRIANGLE
UP, RIGHT, DOWN, LEFT = base.UP, base.RIGHT, base.DOWN, base.LEFT

# Save runs.
ITEMS = shop.ITEMS                      # itemList[0]
GOLD = shop.GOLD
SPC_TRADE = (0xE3C, 17 * 64)            # spcTradeList[17][16]
NPC_TRADE = (0x127C, 48 * 64)           # npcTradeList[48][16]
TPC_TRADE_SW = shop.TPC_TRADE_SW        # tpcTradeListSW[24][3]
PC_TRADE_COUNT = shop.PC_TRADE_COUNT
SPC_PARAM, SPC_SIZE = 0x7488, 0xDC
SKILL_LIST = 0x1EC4


def s32(v):
    return v - (1 << 32) if v & 0x80000000 else v


_LISTS = {}


def trade_lists(server=0, seed=0):
    """The save's trade lists (+0xe3c to the end of tpcTradeListSW) after
    InitTradeItem and SetTradeItemTown on `server`, run natively."""
    key = (server, seed)
    if key in _LISTS:
        return _LISTS[key]
    from test_anim import machine_class
    m = machine_class()(P)
    rnd = random.Random(seed)
    m.hooks[sym("rand")] = lambda mm, *a: rnd.randrange(0x8000)
    m.store(base.SAVEDATA, 4, base.SAVE)
    m.store(base.GAME_P, 4, base.GAME)
    m.store(base.GAME + 0x1C, 4, server)
    m.call(sym("InitTradeItem__10ccSaveDataFv"), [base.SAVE], limit=50_000_000)
    m.call(sym("SetTradeItemTown__10ccSaveDataFv"), [base.SAVE], limit=50_000_000)
    lo, hi = SPC_TRADE[0], TPC_TRADE_SW[0] + TPC_TRADE_SW[1]
    b = bytes(m.mem[base.SAVE + lo:base.SAVE + hi])
    _LISTS[key] = b
    return b


class Member(base.Char):
    """A party member as cmndTarget (FieldUi::talk_to with Speaker::Spc):
    its base and personality are its spcParam in the save."""

    def __init__(self, handle, mid, name, types=4):
        super().__init__(handle, types, mid, 100, 50, 100, 50, name, tag=(256, 200), width=100.0)

    def target_line(self):
        return super().target_line() + f"\ntalkspc {self.handle} {self.id}"


class TradeScenario(shop.TalkScenario):
    def lines(self):
        out = super().lines()
        run = out.pop()
        out.append("trade")
        out.append(run)
        return out


class TradeGame(shop.TalkGame):
    """The talk base with 48 and 49 the game's own, AddSpcItem's thread run
    at the breath, and drainItem compared."""

    unported = tuple(n for n in shop.UNPORTED if n not in (48, 49))

    def make_char(self, c):
        a = super().make_char(c)
        if isinstance(c, Member):
            # base and personality: spcParam[id] (name, type, id as made).
            m, sb = self.m, base.SAVE + SPC_PARAM + SPC_SIZE * c.id
            old = m.load(a, 4)
            m.store(sb + 0x08, 4, m.load(old + 0x08, 4))
            m.store(sb + 0x0C, 2, m.load(old + 0x0C, 2))
            m.store(a, 4, sb)
            m.store(a + 4, 4, sb)
        return a

    def state(self):
        st, ms = super().state()
        if st is None:
            return st, ms
        c = self.menu()
        drain = [self.m.load(c + base.volume.menu_at(0x1AC) + 4 * k, 4, True) for k in range(4)]
        n = len(self.sc.watches)
        return st[:len(st) - n] + drain + st[len(st) - n:], ms


class TradeCase(shop.TalkCase):
    game = TradeGame

    def lists(self, sc, server=0, seed=0):
        """The trade lists as a new game and a town left them."""
        b = trade_lists(server, seed)
        lo = SPC_TRADE[0]
        for k in range(0, len(b), 4):
            sc.saves.append((lo + k, 4, int.from_bytes(b[k:k + 4], "little")))

    def pc(self, row, frames, bag, gold=1000, seed=0, sw=None, watches=None):
        """A trade with the PC of npcTbl row `row`: Kite's bag, the lists,
        the list opened (frame 10), Trade chosen (25 down, 35 OK)."""
        sc = TradeScenario(frames)
        self.kite(sc, gold)
        self.lists(sc, seed=seed)
        if sw is not None:
            for k, v in enumerate(sw):
                sc.saves.append((TPC_TRADE_SW[0] + 3 * (row - 66) + k, 1, v))
        self.items(sc, bag)
        sc.rands = [1]
        sc.watches = watches or [ITEMS, NPC_TRADE, TPC_TRADE_SW, PC_TRADE_COUNT]
        sc.talk(shop.Npc(0x200, row), 10, 22)
        sc.pads.update({25: (0, DOWN), 35: (OK, 0)})
        return sc


def run(sc, game=TradeGame, frames=None):
    """The game's frames of a scenario, for working one out: menu,
    menuNext, menuStatus, proccess, waitCount, the list's select, and
    the events."""
    g = game(sc).run()
    for f in sorted(g):
        if frames and f not in frames:
            continue
        o = g[f]
        st = o["st"]
        if st is None:
            continue
        ev = [e for e in o["ev"] if e[0] not in ("map_alpha",)]
        print(f, st[0], st[1], st[2], st[3], st[17], st[19], ev)
    return g


class Seq:
    """Pad presses in turn from frame `f`, `gap` frames apart."""

    def __init__(self, sc, f):
        self.sc, self.f = sc, f

    def press(self, push, rep=0, gap=4, n=1):
        for _ in range(n):
            self.sc.pads[self.f] = (push, rep)
            self.f += gap
        return self

    def wait(self, n):
        self.f += n
        return self

    def into_offers(self):
        """48's help dismissed; its list from 10 frames on."""
        return self.press(OK, gap=11)

    def into_bag(self, downs=0):
        """`downs` down 48's list, OK: 49; its help dismissed; its list
        from 12 frames after."""
        self.press(0, DOWN, n=downs)
        self.press(OK, gap=18)
        return self.press(OK, gap=12)

    def give(self, n=1, ok=True):
        """OK (push and repeat) `n` times on 49's row: one more offered
        while the trade does not balance."""
        return self.press(OK if ok else 0, OK, n=n)

    def take(self, n=1, push=True):
        """Cancel `n` times on 49's row: one fewer offered."""
        return self.press(CANCEL if push else 0, CANCEL, n=n)

    def again(self):
        """Trade chosen again on the list (after 20 frames)."""
        return self.wait(20).press(OK, gap=18)

    def approve(self, row=0):
        """OK pushed on a balanced trade: the question; its row (0 OK, 1
        Cancel) chosen."""
        self.press(OK, OK, gap=12)
        self.press(0, DOWN, n=row)
        return self.press(OK, gap=8)


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class TradePages(TradeCase):
    """Trade (48, 49) from a PC's list (22)."""

    # A bag of 13 (12 listed: the key item is left out), cheap to dear.
    BAG = [(10, 0, 5), (10, 3, 2), (10, 5, 4), (10, 7, 1), (11, 0, 3), (11, 2, 2), (13, 1, 2), (0, 2, 1),
           (6, 5, 1), (7, 3, 2), (14, 3, 1), (15, 44, 1), (12, 0, 1)]

    def test_trade(self):
        # Wing (30): the offers browsed; Miner's Gloves (13000 to him)
        # against Health Drinks, Antidotes, Knight Blood (not enough: up to
        # what is carried, three kinds at most, one taken back, one out);
        # out of the offer by a cancel on an item not in it (back to the
        # list); Trade again, Cougar Bandana for the Scarab Earring: OK.
        sc = self.pc(30, 560, self.BAG)
        q = Seq(sc, 53).into_offers()
        q.press(0, DOWN, n=7).press(0, UP, n=2)
        q.into_bag()
        q.give(6)                                  # 5 Health Drinks, then no more
        q.press(0, DOWN).give(3)                   # 2 Antidotes
        q.press(0, DOWN, n=2).give(2)              # Knight Blood; then no room
        q.press(0, DOWN).give(2)                   # Raining Rocks: no fourth kind
        q.press(0, UP, n=3).take(2)                # the Antidotes back
        q.press(0, DOWN, n=3).give(3)              # Raining Rocks x3
        q.take(1, push=False)                      # one back, by the repeat
        q.press(0, DOWN, n=8).press(0, UP, n=3)    # the bag scrolled
        q.take(1)                                  # not offered: out
        q.again()                                  # Trade again
        q.into_offers().press(0, DOWN, n=4)
        q.into_bag()
        q.press(0, DOWN, n=8).give(1)              # the Scarab Earring
        q.approve(0).wait(20).press(OK)            # traded; the message
        self.assertLess(q.f, 560)
        self.compare(sc, "trade")

    def test_many_offers(self):
        # Macky (31) with a longer list than a town leaves (a key item and
        # a hole among it, left out): the offers scrolled both ways, the
        # last traded for.
        offers = [(10, 0, 3), (11, 2, 1), (15, 44, 1), (0, 4, 1), (-1, -1, 0), (6, 2, 1), (7, 0, 1), (8, 22, 1),
                  (9, 2, 1), (10, 18, 2), (11, 54, 1), (12, 1, 1), (13, 1, 1), (14, 10, 4)]
        sc = self.pc(31, 300, self.BAG)
        at = NPC_TRADE[0] + 64 * 1
        for k in range(16):
            c, i, n = offers[k] if k < len(offers) else (-1, -1, 0)
            sc.saves += [(at + 4 * k, 2, i & 0xFFFF), (at + 4 * k + 2, 1, c & 0xFF), (at + 4 * k + 3, 1, n)]
        q = Seq(sc, 53).into_offers()
        q.press(0, DOWN, n=13).press(0, UP, n=4).press(0, DOWN, n=2)
        q.into_bag()
        q.press(0, DOWN, n=10).give(1)
        q.approve(0).wait(20).press(OK)
        self.assertLess(q.f, 300)
        self.compare(sc, "many offers")

    def test_trading_pcs(self):
        # Alicia (66): trade 0, 10 Well Water for Miner's Gloves+ (one not
        # wanted offered first). Tim (79): trade 2, 10 Golden Axes and 10
        # Silver Axes. Teria (70), trade 0 done: two left, the second.
        for row, sw, downs, bag, gives in (
                (66, None, 0, [(10, 0, 3), (10, 12, 12)], [(0, 1), (1, 10)]),
                (79, None, 2, [(3, 60, 10), (3, 61, 12), (10, 0, 1)], [(0, 10), (1, 10)]),
                (70, (0, 1, 1), 1, [(10, 16, 30)], [(0, 25)])):
            sc = self.pc(row, 400, bag, sw=sw)
            q = Seq(sc, 53).into_offers()
            q.into_bag(downs)
            for k, (r, n) in enumerate(gives):
                if k:
                    q.press(0, DOWN, n=r - gives[k - 1][0])
                q.give(n)
            q.approve(0).wait(20).press(OK)
            self.assertLess(q.f, 400)
            self.compare(sc, f"trading pc {row}")

    def test_nothing_to_trade(self):
        # An empty bag, one of key items only (20), a trading PC with its
        # trades all done (30): "There is no item to trade.", then back to
        # the list.
        for row, bag, sw in ((30, [], None), (45, [(15, 44, 1), (15, 2, 1)], None), (75, self.BAG, (0, 0, 0))):
            sc = self.pc(row, 120, bag, sw=sw)
            Seq(sc, 53).press(OK, gap=12).press(OK, gap=12).press(CANCEL)
            self.compare(sc, f"nothing to trade {row}")

    def test_refusals(self):
        # 99 Mage's Souls carried; a full bag (40 kinds) and the Silver
        # Scarab offered from a stack of two (kept); the same with the
        # stack given whole: the trade goes through.
        full = [(11, k, 1) for k in range(38)] + [(10, 0, 1)]
        for what, bag, downs, row in (("99", [(10, 18, 99), (14, 3, 1)], 1, 1),
                                      ("full", [(14, 3, 2)] + full, 4, 0),
                                      ("whole", [(14, 3, 1)] + full, 4, 0)):
            sc = self.pc(30, 360, bag)
            q = Seq(sc, 53).into_offers()
            q.into_bag(downs)
            q.press(0, DOWN, n=row).give(1)
            q.approve(0).wait(20).press(OK, gap=12).press(OK)
            self.assertLess(q.f, 360)
            self.compare(sc, f"refusal {what}")

    def test_cancels(self):
        # 48: cancel (back to the list, the tasks woken), Trade again; 49:
        # the question's Cancel row, then its cancel key; triangle on a
        # weapon offered (48) and carried (49): its status (64).
        sc = self.pc(30, 480, self.BAG)
        q = Seq(sc, 53).into_offers()
        q.press(0, DOWN).press(CANCEL).again()
        q.into_offers().into_bag(3)
        q.press(0, DOWN, n=8).give(1).approve(1)
        q.again()
        q.into_offers().into_bag(3)
        q.press(0, DOWN, n=8).give(1)
        q.press(OK, OK, gap=12).press(CANCEL)
        q.again().into_offers().press(0, DOWN, n=2).press(TRIANGLE, gap=20)
        self.assertLess(q.f, 480)
        self.compare(sc, "cancels")
        sc = self.pc(30, 200, self.BAG)
        q = Seq(sc, 53).into_offers().into_bag(0)
        q.press(0, DOWN, n=7).press(TRIANGLE)
        self.compare(sc, "triangle in 49")
        # The list opened with mode 0 (the tasks already asleep): 48 and 49
        # go back without their breath.
        sc = self.pc(30, 260, self.BAG)
        sc.modes[10] = 0
        q = Seq(sc, 53).into_offers().press(CANCEL).again()
        q.into_offers().into_bag(0).take(1)
        self.assertLess(q.f, 260)
        self.compare(sc, "mode 0")

    def test_random_trade(self):
        pool = self.BAG + [(10, 12, 20), (10, 16, 30), (3, 60, 12), (3, 61, 12), (10, 18, 3)]
        for seed in range(1, 9):
            rnd = random.Random(seed)
            row = rnd.choice((30, 31, 45, 50, 66, 70, 78, 79))
            bag = rnd.sample(pool, rnd.randrange(0, len(pool) + 1))
            sw = tuple(rnd.randrange(2) for _ in range(3)) if row >= 66 and rnd.random() < 0.5 else None
            sc = self.pc(row, 500, bag, gold=rnd.choice((0, 5000)), seed=seed, sw=sw)
            for f in range(120, 500, 120):
                sc.opens[f] = 22
                sc.modes[f] = 1
                if rnd.random() < 0.5:
                    sc.firsts.add(f)
            for f in range(40, 500):
                r = rnd.random()
                if r < 0.15:
                    d = rnd.choice((UP, DOWN))
                    sc.pads[f] = (d, d)
                elif r < 0.25:
                    sc.pads[f] = (OK, OK) if rnd.random() < 0.7 else (0, OK)
                elif r < 0.28:
                    sc.pads[f] = (CANCEL, CANCEL) if rnd.random() < 0.5 else (0, CANCEL)
                elif r < 0.285:
                    sc.pads[f] = (TRIANGLE, 0)
            self.compare(sc, f"random trade {seed}")


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class MemberTrades(TradeCase):
    """Trade with a party member (type 4): its spcTradeList, the rates
    and friendship, and AddSpcItem handing it what Kite gave (a book read,
    a piece of equipment put on, the rest into its bag)."""

    def member(self, mid, name, frames, bag, spc_bag, cls, equip, friendship, offers, skills=(), gold=100,
               level=None):
        """Member `mid` spoken to (PcMenu opened at frame 10 without
        firstTime), Trade chosen (25 down, 35 OK): its offers, bag, class,
        equipment (head, body, arm, leg, weapon), friendship, skills, and a
        level. CalcReal runs on the record as the game has it, so the member
        wears something in every slot and has a level, as every member does
        from a new game on (an empty slot, -1, is read by the game as the row
        before its table, which piney-battle's CalcReal skips)."""
        sc = TradeScenario(frames)
        self.kite(sc, 1000)
        self.lists(sc)
        at = SPC_TRADE[0] + 64 * (mid - 1)
        for k in range(16):
            c, i, n = offers[k] if k < len(offers) else (-1, -1, 0)
            sc.saves += [(at + 4 * k, 2, i & 0xFFFF), (at + 4 * k + 2, 1, c & 0xFF), (at + 4 * k + 3, 1, n)]
        self.items(sc, bag)
        at = ITEMS[0] + 160 * mid
        for k in range(40):
            c, i, n = spc_bag[k] if k < len(spc_bag) else (-1, -1, 0)
            sc.saves += [(at + 4 * k, 2, i & 0xFFFF), (at + 4 * k + 2, 1, c & 0xFF), (at + 4 * k + 3, 1, n)]
        sp = SPC_PARAM + SPC_SIZE * mid
        level = level or 3 + 4 * mid
        sc.saves += [(sp + 0x08, 4, 4), (sp + 0x0C, 2, mid), (sp + 0x0E, 2, level), (sp + 0x14, 4, gold),
                     (sp + 0xD8, 2, cls), (sp + 0xDA, 2, friendship)]
        for k, v in enumerate(equip):
            sc.saves.append((sp + 0xC8 + 2 * k, 2, v & 0xFFFF))
        for k in range(20):
            sc.saves.append((SKILL_LIST + 40 * mid + 2 * k, 2, (skills[k] if k < len(skills) else -1) & 0xFFFF))
        sc.watches = [ITEMS, (ITEMS[0] + 160 * mid, 160), (sp + 8, SPC_SIZE - 8), (SKILL_LIST + 40 * mid, 40),
                      SPC_TRADE, PC_TRADE_COUNT]
        sc.target = Member(0x300, mid, name)
        sc.opens[10] = 22
        sc.modes[10] = 1
        sc.pads.update({25: (0, DOWN), 35: (OK, 0)})
        return sc

    def test_member_equips(self):
        # Orca (2, a class that wears armour up to 3): Health Drinks, the
        # Scarab Earring, the Power Book for a Silver Scarab. She reads
        # the book, puts the earring on (the Bandana to her bag, its skill
        # dropped and the earring's learnt; ChangeEquip's breath), and the
        # drinks go to her bag, the one past 99 sold.
        sc = self.member(2, b"Orca", 420, [(10, 0, 5), (6, 5, 1), (12, 0, 1), (0, 2, 1), (14, 10, 2)],
                         [(10, 0, 98), (11, 3, 1)], 1, (0, 53, 55, 49, 61), 450,
                         [(14, 3, 1), (11, 56, 1)], skills=(150, 33))
        q = Seq(sc, 53).into_offers().into_bag(0)
        q.give(2).press(0, DOWN).give(1).press(0, DOWN).give(1)
        q.approve(0)
        q.press(OK, gap=8, n=30)
        self.assertLess(q.f, 420)
        self.compare(sc, "member equips")

    def test_member_full_bag(self):
        # Mia (1, member 1: Aromatic Grass counts 50000 a piece): Phantom
        # Blades (her class's weapon, dearer than hers: put on) and two
        # Aromatic Grass, her bag full: the old blades take the place of
        # her cheapest item (sold), the grass of the next.
        spc_bag = [(10, 0, 5)] + [(11, k, 1) for k in range(38)] + [(13, 1, 2)]
        sc = self.member(1, b"Mia", 420, [(0, 2, 1), (14, 10, 2), (10, 0, 3)], spc_bag, 0,
                         (1, 41, 41, 41, 0), 1000, [(14, 3, 1)], skills=(6,), gold=9_999_990)
        q = Seq(sc, 53).into_offers().into_bag(0)
        q.give(1).press(0, DOWN).give(1)
        q.approve(0)
        q.press(OK, gap=8, n=30)
        self.assertLess(q.f, 420)
        self.compare(sc, "member full bag")

    def test_random_member(self):
        pool = [(10, 0, 5), (6, 5, 1), (12, 0, 2), (12, 1, 1), (0, 2, 1), (1, 7, 1), (7, 3, 1), (14, 10, 3),
                (11, 2, 2), (9, 2, 1), (8, 22, 1)]
        for seed in range(1, 7):
            rnd = random.Random(seed)
            mid, name = rnd.choice(((1, b"Mia"), (2, b"Orca"), (15, b"BlackRose")))
            bag = rnd.sample(pool, rnd.randrange(1, len(pool) + 1))
            offers = rnd.sample([(14, 3, 1), (11, 56, 1), (6, 2, 1), (10, 18, 2), (12, 0, 1)], rnd.randrange(1, 4))
            sc = self.member(mid, name, 500, bag, [(10, 0, rnd.randrange(1, 99))], rnd.randrange(6),
                             tuple(rnd.choice((0, 1, 2)) for _ in range(5)), rnd.randrange(1200), offers,
                             skills=(150, 6, 33))
            for f in range(40, 500):
                r = rnd.random()
                if r < 0.12:
                    d = rnd.choice((UP, DOWN))
                    sc.pads[f] = (d, d)
                elif r < 0.3:
                    sc.pads[f] = (OK, OK) if rnd.random() < 0.7 else (0, OK)
                elif r < 0.32:
                    sc.pads[f] = (CANCEL, CANCEL) if rnd.random() < 0.5 else (0, CANCEL)
            self.compare(sc, f"random member {seed}")


if __name__ == "__main__":
    unittest.main()
