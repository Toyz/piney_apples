#!/usr/bin/env python3
"""crates/piney-fieldui's talk pages that are not a shop (gcmn.prg menu.cpp:
NpcMenu 23, an administrator; SpcMenu 21, a party member spoken to, with
Gift, PresentMenu 50; the Grunty breeders, BreederMenu 27 and Give Food,
BreedingMenu 56; Dun Loireag's dogs, NorainuMenu 44, and Grunties,
OtonainuMenu 45 and InuMenu 46) against the game's own
ccThMenu in eemu, frame by frame.

It is tools/test_fieldui_shop_rs.py's comparison (the NPC spoken to is its
real npcTbl row, rand() the same on both sides, the talk state compared)
with more characters to speak to:

- a party member (`Spc`): a ccChar whose base and personality are the
  save's spcParam[id] (as a ccSpcChar's are), named from charTbl; the save
  has ccSaveData::SetSpcBaseMsg run on it on both sides (the game's own,
  the port's menus::talk::set_spc_base_msg) so base->msg is spcMsgTbl[id];
- a Grunty in a breeder's pen (`Grunty`): a ccPGuso whose base is an
  npcTbl row, as cmndTargetPrev, with its growthNum, msgNum and exist set
  frame by frame as the scenario says (the Grunty's own code is the
  world's); the menu's write of growthNum is a request on the port's side.

Hooked here besides the shop harness's (which runs CalcReal as the game
has it and notes ccThEquipMenu's ChangeEquip at the menu's breath):
ccSpcMessagePresentOtherFellow (the other members' remarks on a gift) and
EntryAffect with its parameters (the Grunty fed). The menu's fade (menuFade) is the game's own, drawn through the
base's SendPacket hook; its four elements are compared each frame too.

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
import test_fieldui_shop_rs as shop  # noqa: E402

base = shop.base
P, sym = base.P, base.sym
OK, CANCEL, TRIANGLE = base.OK, base.CANCEL, base.TRIANGLE
UP, RIGHT, DOWN, LEFT = base.UP, base.RIGHT, base.DOWN, base.LEFT
R1, L1 = shop.R1, shop.L1
Npc = shop.Npc

# The pages this harness ports; the others stay closed at once.
MINE = (21, 23, 27, 44, 45, 46, 50, 56, 88, 90, 91)
SPC = 0x7488
SPC_SIZE = 0xDC
TALK_NUM = 0x220C
SPC_PRESENT = (0x73FC, 68)             # spcPresent[17]
IMP_ITEMS = (0xCFC, 64)                # impItemList (the Grunty foods, 26-41)
GROWTH = (0x2194, 120)                 # growth[5]
TRADE_COUNT = shop.PC_TRADE_COUNT
# charTbl's names (DEMO.PRG), as the port reads them: 21 from Mutation on.
NAMES = [n[:20] for n in base.char_tbl_names()]


def spc_row(i, off, n):
    """spcParam[i] + off, n bytes, as a watch."""
    return (base.spc_param(i) + off, n)


# From Mutation on, characters 18-20's records are the save's extension's
# (docs/formats/save.md, "The extension").
def talk_at(i):
    """talkNum[i]."""
    return TALK_NUM + i if i < 18 else base.EXT + 0x498 + i - 18


def items_at(i):
    """itemList[i]."""
    return 0x30 + 160 * i if i < 18 else base.EXT + 160 * (i - 18)


def skills_at(i):
    """skillList[i]."""
    return 0x1EC4 + 40 * i if i < 18 else base.EXT + 0x420 + 40 * (i - 18)


def present_watch(i):
    """spcPresent: the 17 of ccSaveData, or the extension's for 18-20."""
    return SPC_PRESENT if i < 18 else (base.EXT + 0x4A8, 12)


class Spc(base.Char):
    """A party member as cmndTarget: its base is saveData.spcParam[id].base
    (type 6, as NewGame copies it from charTbl), its name charTbl's."""

    def __init__(self, handle, cid, tag=(256, 200), server=0):
        super().__init__(handle, 6, cid, 100, 50, 100, 50, NAMES[cid], tag=tag, width=100.0)
        self.server = server



class Grunty(base.Char):
    """A Grunty (ccPGuso): its base is npcTbl[row]; its record's level,
    size and food line (what dogAction's line reads)."""

    def __init__(self, handle, row, exist=0, level=0, size=0, food=0, tag=None):
        name, types, cid = shop.npc_row(row)
        super().__init__(handle, types, cid, 0, 0, 0, 0, name, tag=tag, width=100.0)
        self.row, self.exist = row, exist
        self.level, self.size, self.food = level, size, food
        self.server = 1

    def target_line(self):
        # As shop.Npc's: the talk target named by its npcTbl row.
        tx, ty = self.tag if self.tag else (-9999, 0)
        return f"npc {self.handle} {self.row} {tx} {ty}"


def after_affect(pg, m, kind):
    """What the Grunty's affect function does to its msgNum (+0x2af):
    dogAction's 15 and 14 for a young one, dogAction2's (and
    dogActionAdult's) 14 for a grown one - the functions
    tools/test_grunty_rs.py runs natively."""
    row = m.load(m.load(pg, 4) + 0xC, 2, True)
    if row >= 154:
        if kind == 15:
            level, size = m.load(pg + 0x2D0, 2, True), m.load(pg + 0x2D2, 2, True)
            v = 7 if level == 4 else 2 if size == 0 else m.load(pg + 0x2E4, 1)
            m.store(pg + 0x2AF, 1, v)
        elif kind == 14 and m.load(pg + 0x2D0, 2, True) == 4:
            m.store(pg + 0x2AF, 1, 0)
    elif kind == 14:
        m.store(pg + 0x2AF, 1, 7)


def race_globals(m):
    """From Mutation on, the Flag Race's object pointer (MUT 0x0038bd44)
    and its task's start (0x005ff820, which menu 88 calls), found from the
    task's name: the task stores the new object to $gp + n after naming
    itself PG_RACE; the start is the function that loads the task's
    address."""
    lo_, hi_ = 0x00400000, 0x00800000
    mem = bytes(m.mem[lo_:hi_])
    name = mem.find(b"PG_RACE\0") + lo_
    words = struct.unpack_from("<%dI" % ((hi_ - lo_) // 4), mem)

    def pair(va):
        """The first `lui r, hi(va); addiu r, r, lo(va)`."""
        hi = ((va + 0x8000) >> 16) & 0xFFFF
        for k in range(len(words) - 1):
            w, w2 = words[k], words[k + 1]
            r = (w >> 16) & 31
            if w >> 26 == 0x0F and w & 0xFFFF == hi and w2 >> 26 == 0x09 and (w2 >> 21) & 31 == r \
                    and w2 & 0xFFFF == va & 0xFFFF:
                return k
        raise AssertionError(hex(va))

    def start_of(k):
        while not (words[k] >> 16 == 0x27BD and words[k] & 0x8000):
            k -= 1
        return lo_ + 4 * k

    k = pair(name)
    store = next(w for w in words[k:k + 32] if w >> 26 == 0x2B and (w >> 21) & 31 == 28)
    imm = store & 0xFFFF
    ptr = (m.p.gp + (imm - 0x10000 if imm & 0x8000 else imm)) & 0xFFFFFFFF
    task = start_of(k)
    return ptr, start_of(pair(task))


class Scenario(shop.TalkScenario):
    def __init__(self, frames):
        super().__init__(frames)
        self.races = {}           # frame -> (state, rank, running, time), None gone
        self.spc_msg = False      # SetSpcBaseMsg on the save (both sides)
        self.drops = []           # frames before which cmndTarget is lost
        self.grunty = None        # the Grunty, cmndTargetPrev
        self.growth = {}          # frame -> the Grunty's growthNum from then
        self.msg_nums = {}        # frame -> its msgNum

    def lines(self):
        out = super().lines()
        run = out.pop()
        if isinstance(self.target, Spc):
            out.append(f"talkspc {self.target.handle} {self.target.id}")
        out.append("breed")
        for f, v in sorted(self.races.items()):
            out.append(f"norace {f}" if v is None else "race %d %d %d %d %d" % ((f,) + tuple(v)))
        if self.spc_msg:
            out.append("spcmsg")
        for f in self.drops:
            out.append(f"drop {f}")
        t = self.target
        if isinstance(t, Grunty):
            out.append(f"pgtarget {t.handle} {t.row} {t.exist} {t.level} {t.size} {t.food}")
            for f, v in sorted(self.growth.items()):
                out.append(f"growth {f} {v}")
            for f, v in sorted(self.msg_nums.items()):
                out.append(f"pgmsg {f} {v}")
        g = self.grunty
        if g is not None:
            out.append(f"pguso {g.handle} {g.row} {g.exist}")
            for f, v in sorted(self.growth.items()):
                out.append(f"growth {f} {v}")
            for f, v in sorted(self.msg_nums.items()):
                out.append(f"pgmsg {f} {v}")
        out.append(run)
        return out


class TalkGame(shop.TalkGame):
    """The game's ccThMenu with a party member, an administrator or a
    breeder spoken to."""

    unported = tuple(n for n in shop.UNPORTED if n not in MINE)

    def setup(self):
        super().setup()
        m, sc = self.m, self.sc
        self.race = None
        if sc.races:
            self.race_ptr, start = race_globals(m)
            m.hooks[start] = lambda mm, kind, *a: self.ev("race_start", base.sx32(kind)) or 0
        if sc.spc_msg:
            self.call(sym("SetSpcBaseMsg__10ccSaveDataFv"), [base.SAVE])
        self.pg = None
        if sc.grunty is not None:
            self.pg = self.make_char(sc.grunty)
            m.store(base.CMNDTARGETPREV, 4, self.pg)
        if isinstance(sc.target, Grunty):
            self.pg = next(a for a, c in self.chars.items() if c is sc.target)

    def make_char(self, c):
        m = self.m
        if isinstance(c, Spc):
            a = base.Game.make_char(self, c)
            sb = base.SAVE + base.spc_param(c.id)
            old = m.load(a, 4)
            for o in (0x18, 0x1C):
                m.store(sb + o, 4, m.load(old + o, 4))
            m.store(a, 4, sb)
            m.store(a + 4, 4, sb)
            return a
        if isinstance(c, Grunty):
            a = self.alloc(0x330)
            m.store(a + 0x00, 4, shop.NPC_TBL + 0x70 * c.row)
            m.store(a + 0x04, 4, self.alloc(0x100))
            m.store(a + 0x40, 4, base.fb(float(c.handle)))
            m.store(a + 0x5C, 4, base.fb(1.0))
            m.store(a + 0x305, 1, c.exist)
            m.store(a + 0x2D0, 2, c.level)
            m.store(a + 0x2D2, 2, c.size)
            m.store(a + 0x2E4, 4, c.food)
            self.chars[a] = c
            return a
        if isinstance(c, Npc):
            # As the base's, with room for the Grunty's fields (+0x2ae,
            # +0x305) that BreedingMenu reads of cmndTargetPrev whatever
            # it is: zeros here, an idle Grunty on the port's side.
            a = self.alloc(0x330)
            m.store(a + 0x00, 4, shop.NPC_TBL + 0x70 * c.row)
            m.store(a + 0x04, 4, self.alloc(0x100))
            m.store(a + 0x40, 4, base.fb(float(c.handle)))
            m.store(a + 0x5C, 4, base.fb(1.0))
            self.chars[a] = c
            return a
        return super().make_char(c)

    def hooks(self):
        super().hooks()
        h = self.m.hooks
        s16 = lambda v: v - 0x10000 if v & 0x8000 else v  # noqa: E731

        def affect(mm, c, who, kind, p0, *a):
            # Only a0-a3 come as arguments: the fourth short is t0.
            kind, p0, p1 = s16(kind & 0xFFFF), s16(p0 & 0xFFFF), s16(mm.r[8] & 0xFFFF)
            if kind == 19:
                self.ev("affect", self.handle(c), kind, p0, p1)
            else:
                self.ev("affect", self.handle(c), kind)
            if self.pg and c == self.pg:
                after_affect(c, mm, kind)
            return 0
        h[sym("EntryAffect__6ccCharFP6ccCharssss")] = affect
        # NorainuMenu's talkNum comes from ccRand (the town's Mersenne
        # Twister); the port's menu has rand() only, so both give the
        # scenario's values in turn.
        h[sym("ccRand__Fv")] = h[sym("rand")]
        h[sym("ccSpcMessagePresentOtherFellow__FP6ccChar")] = \
            lambda mm, c, *a: self.ev("present_other", self.handle(c)) or 0

    def start_frame(self):
        super().start_frame()
        m, sc, f = self.m, self.sc, self.frame
        if f in sc.drops and m.load(base.CMNDTARGET, 4):
            # ccThGameCtrl's ccChangeCmndTarget(0) for a target lost; the
            # character is still there (ccCheckTarget passes it as
            # cmndTargetPrev: with none, Disp would draw a target list
            # from registers it never set).
            m.store(base.CMNDTARGETPREV, 4, m.load(base.CMNDTARGET, 4))
            m.store(base.CMNDTARGET, 4, 0)
        if f in sc.races:
            v = sc.races[f]
            if v is None:
                m.store(self.race_ptr, 4, 0)
            else:
                if self.race is None:
                    self.race = self.alloc(0xB0)
                state, rank, running, time = v
                m.store(self.race + 0xA3, 1, state & 0xFF)
                m.store(self.race + 0xA2, 1, rank & 0xFF)
                m.store(self.race + 0xA4, 1, running)
                m.store(self.race + 0x90, 2, time & 0xFFFF)
                m.store(self.race_ptr, 4, self.race)
        if self.pg:
            if f in sc.growth:
                m.store(self.pg + 0x2AE, 1, sc.growth[f] & 0xFF)
            if f in sc.msg_nums:
                m.store(self.pg + 0x2AF, 1, sc.msg_nums[f] & 0xFF)

    def state(self):
        st, ms = super().state()
        if st is None:
            return st, ms
        m, c = self.m, self.menu()
        st = st + [m.load(c + 0x28, 2, True), m.load(c + 0x26, 2, True)]
        fade = m.load(c + 0xB8, 4)
        for k in range(4):
            e = fade + 4 + 0x24 * k
            st += [m.load(e, 2, True), m.load(e + 2, 2, True), m.load(e + 4, 2, True)]
        st.append(m.load(self.pg + 0x2AE, 1, True) if self.pg else 0)
        st += [m.load(self.pg + o, 1, True) if self.pg else 0 for o in (0x2AF, 0x307, 0x306)]
        if self.sc.races:
            r = m.load(self.race_ptr, 4)
            st += [m.load(r + 0xA3, 1, True), m.load(r + 0xA4, 1), m.load(r + 0xA5, 1)] if r else [-9, -9, -9]
        return st, ms


class Case(shop.TalkCase):
    game = TalkGame

    def spc(self, sc, cid, story=0, handle=0x300):
        """Party member `cid` spoken to (its spcParam base type 6, id)."""
        sc.spc_msg = True
        at = base.spc_param(cid)
        sc.saves += [(at + 8, 4, 6), (at + 0xC, 2, cid), (talk_at(cid), 1, story)]
        return Spc(handle, cid)


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class NpcPages(Case):
    """NpcMenu (23): the administrators."""

    def test_admin(self):
        # Rows 29 and 158 (sysopeMsg): the line, OK through it (emode 1
        # records chain), shut.
        for row in (29, 158):
            sc = Scenario(140)
            self.kite(sc)
            sc.talk(Npc(0x200, row), 10, 23)
            for k in range(10):
                sc.pads[25 + 10 * k] = (OK, 0)
            self.compare(sc, f"admin {row}")

    def test_admin_lost(self):
        # No target at all; a character with no lines; the target gone and
        # a battle while it speaks.
        sc = Scenario(40)
        self.kite(sc)
        sc.opens[10] = 23
        sc.modes[10] = 1
        sc.firsts.add(10)
        self.compare(sc, "admin no target")
        sc = Scenario(40)
        self.kite(sc)
        sc.target = base.Char(0x200, 0x10, 29, 0, 0, 0, 0, b"Nobody", tag=(256, 200))
        sc.opens[10] = 23
        sc.modes[10] = 1
        sc.firsts.add(10)
        self.compare(sc, "admin no lines")
        sc = Scenario(60)
        self.kite(sc)
        sc.talk(Npc(0x200, 29), 10, 23)
        sc.drops.append(30)
        self.compare(sc, "admin dropped")
        sc = Scenario(60)
        self.kite(sc)
        sc.talk(Npc(0x200, 29), 10, 23)
        sc.gamefs[30] = (0, 1, 0)
        self.compare(sc, "admin battle")


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class SpcPages(Case):
    """SpcMenu (21): a party member spoken to; Talk (47) from it."""

    def test_spc(self):
        # Several members and stories: the greeting (voice -31, 2 id), the
        # trade count started, the cursor through the rows, cancel.
        for cid, story in ((2, 0), (1, 1), (5, 2), (15, 0), (17, 1)):
            sc = Scenario(80)
            self.kite(sc)
            for k in range(77):
                sc.saves.append((TRADE_COUNT[0] + k, 1, 0xFF))
            sc.watches = [TRADE_COUNT, spc_row(cid, 0x20, 4)]
            sc.talk(self.spc(sc, cid, story), 10, 21)
            sc.pads.update({30: (0, DOWN), 38: (0, DOWN), 46: (0, DOWN), 54: (0, UP), 65: (CANCEL, 0)})
            self.compare(sc, f"spc {cid} story {story}")

    def test_spc_pages(self):
        # Talk (the member's line, voice 2 id + 1, chained) and back; Trade
        # (another harness's, closed at once); Gift.
        for cid, sel in ((2, 0), (7, 0), (2, 1), (3, 2)):
            sc = Scenario(140)
            self.kite(sc)
            sc.talk(self.spc(sc, cid, cid % 3), 10, 21)
            for k in range(sel):
                sc.pads[25 + 8 * k] = (0, DOWN)
            sc.pads[45] = (OK, 0)
            for k in range(6):
                sc.pads[60 + 10 * k] = (OK, 0)
            sc.pads[125] = (CANCEL, 0)
            self.compare(sc, f"spc {cid} row {sel}")

    def test_spc_no_lines(self):
        # A member whose base->msg is 0 (SetSpcBaseMsg not run): no
        # greeting; Talk then goes straight back to the list.
        sc = Scenario(90)
        self.kite(sc)
        spc = self.spc(sc, 5)
        sc.spc_msg = False
        sc.talk(spc, 10, 21)
        sc.pads.update({30: (OK, 0), 60: (CANCEL, 0)})
        self.compare(sc, "spc no lines")

    def test_spc_lost(self):
        # The member gone from under the list; a battle; opened with no
        # target; opened again without firstTime.
        sc = Scenario(60)
        self.kite(sc)
        sc.talk(self.spc(sc, 2), 10, 21)
        sc.drops.append(30)
        self.compare(sc, "spc dropped")
        sc = Scenario(60)
        self.kite(sc)
        sc.talk(self.spc(sc, 2), 10, 21)
        sc.gamefs[30] = (0, 1, 0)
        self.compare(sc, "spc battle")
        sc = Scenario(40)
        self.kite(sc)
        sc.opens[10] = 21
        sc.modes[10] = 1
        self.compare(sc, "spc no target")
        sc = Scenario(90)
        self.kite(sc)
        sc.talk(self.spc(sc, 4), 10, 21)
        sc.pads[40] = (CANCEL, 0)
        sc.opens[60] = 21
        sc.modes[60] = 1
        sc.pads[80] = (CANCEL, 0)
        self.compare(sc, "spc again")

    def test_random_spc(self):
        for seed in range(1, 7):
            rnd = random.Random(seed)
            sc = Scenario(300)
            self.kite(sc)
            cid = rnd.randrange(1, 18)
            sc.talk(self.spc(sc, cid, rnd.randrange(3)), 10, 21)
            for f in range(60, 300, 60):
                sc.opens[f] = 21
                sc.modes[f] = 1
                if rnd.random() < 0.5:
                    sc.firsts.add(f)
            for f in range(12, 300):
                r = rnd.random()
                if r < 0.15:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN)))
                elif r < 0.2:
                    sc.pads[f] = (rnd.choice((OK, CANCEL)), 0)
            self.compare(sc, f"random spc {seed}")


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class GiftPages(Case):
    """PresentMenu (50): Gift, from a member's list."""

    BAG = [(10, 0, 5), (10, 3, 2), (13, 1, 1), (12, 1, 2), (14, 2, 1), (14, 9, 1), (14, 6, 2), (1, 5, 2), (6, 22, 1),
           (7, 23, 1), (9, 60, 1), (3, 2, 1), (14, 7, 1)]

    def gift(self, cid, frames, bag=BAG, story=0, job=1, eq=(0, 0, 0, 0, 0, 0), member=(), gold=0, area=0,
             friend=0, mode=1, skills=(9, 33, 150, 172)):
        """Kite's `bag`; member `cid` (its job, level, equipment, skills,
        bag, gold and friendship) spoken to in town (or `area`), Gift chosen
        at 45. Its `real` and `tune` are watched too (CalcReal runs as the
        game has it). The random runs wear no -1 (no member does from a new
        game on): a slot still -1 when CalcReal runs is read by the game as
        the row before its table, which piney-battle's CalcReal skips."""
        sc = Scenario(frames)
        self.kite(sc, 500)
        if area:
            sc.game = (area, 0, 0, 0)
        self.items(sc, bag)
        spc = self.spc(sc, cid, story)
        at = base.spc_param(cid)
        sc.saves += [(at + 0xD8, 2, job), (at + 0x14, 4, gold), (at + 0xDA, 2, friend), (at + 0x0E, 2, 3 + cid)]
        sc.saves += [(at + 0xC8 + 2 * k, 2, v & 0xFFFF) for k, v in enumerate(eq)]
        sc.saves += [(skills_at(cid) + 2 * k, 2, (skills[k] if k < len(skills) else -1) & 0xFFFF) for k in range(20)]
        self.bag(sc, cid, member)
        sc.watches = [shop.ITEMS, (items_at(cid), 160), spc_row(cid, 0x14, 4), spc_row(cid, 0xC8, 12),
                      spc_row(cid, 0xDA, 2), present_watch(cid), (skills_at(cid), 40)]
        sc.watches.append(spc_row(cid, 0x48, 64))
        sc.talk(spc, 10, 21, mode)
        sc.pads.update({25: (0, DOWN), 33: (0, DOWN), 45: (OK, 0)})
        return sc

    def bag(self, sc, cid, entries):
        """Member `cid`'s bag (itemList[cid]) holding (category, id, count)s."""
        at = items_at(cid)
        for k in range(40):
            cat, iid, num = entries[k] if k < len(entries) else (-1, -1, 0)
            sc.saves += [(at + 4 * k, 2, iid & 0xFFFF), (at + 4 * k + 2, 1, cat & 0xFF), (at + 4 * k + 3, 1, num)]

    def give(self, sc, f, page, row, ups=0, answer=OK, oks=8):
        """From frame `f`: to `page` and `row`, OK, `ups` more, OK, the
        question's answer (OK on OK, or Cancel), then OK through the
        member's thanks and the result `oks` times. The frame after."""
        for _ in range(page):
            sc.pads[f] = (0, R1)
            f += 6
        for _ in range(row):
            sc.pads[f] = (0, DOWN)
            f += 6
        sc.pads[f] = (OK, 0)
        f += 8
        for _ in range(ups):
            sc.pads[f] = (0, UP)
            f += 6
        sc.pads[f] = (OK, 0)
        f += 25
        if answer == CANCEL:
            sc.pads[f] = (0, DOWN)
            f += 6
            answer = OK
        sc.pads[f] = (answer, 0)
        f += 20
        for _ in range(oks):
            sc.pads[f] = (OK, 0)
            f += 8
        return f

    def test_gift(self):
        # Items of each worth to Orca: the thanks by worth (and story), the
        # friendship, spcPresent; Kite's count down.
        for page, row, ups, story, friend in ((0, 0, 2, 0, 240), (0, 1, 0, 1, -200), (3, 0, 0, 0, 240),
                                              (3, 1, 0, 2, 240), (3, 2, 1, 0, 0), (3, 3, 0, 1, 0), (1, 0, 0, 0, 0)):
            sc = self.gift(2, 200, story=story, friend=friend)
            self.give(sc, 60, page, row, ups)
            self.compare(sc, f"gift page {page} row {row} story {story}")

    def test_gift_every_member(self):
        # Each member's thanks, members 1-17: their own records of the
        # volume's spcMsgPresent10 (issue #58: Mutation's read Infection's
        # address and showed errorData).
        for cid in range(1, 18):
            sc = self.gift(cid, 200)
            self.give(sc, 60, 0, 0)
            self.compare(sc, f"gift member {cid}")

    def test_gift_mia(self):
        # Mia's Aromatic Grass (14/10) is worth 50000; with a full bag she
        # keeps it over anything.
        sc = self.gift(1, 200, bag=[(14, 10, 3), (14, 0, 1)], member=[(10, 6, 1)] * 39 + [(14, 0, 1)])
        self.give(sc, 60, 3, 0, 1)
        self.compare(sc, "gift mia")
        sc = self.gift(1, 200, bag=[(14, 10, 3), (14, 0, 1)], member=[(14, 10, 1)] * 38 + [(13, 2, 1), (11, 0, 1)])
        self.give(sc, 60, 3, 1)
        self.compare(sc, "gift mia hers")

    @unittest.skipIf(base.volume.NAME == "infection", "Mutation on: Gift weighs against what the member has")
    def test_gift_wanted(self):
        # Mia wants weapon 1/66 and Aromatic Grass (14/10): each is worth
        # 50000, the top thanks. Equipment she carries or wears is weighed
        # as any other: its worth less her best of the kind.
        for what, member, eq, page in (("wanted", [], (0, 0, 0, 0, 0, 0), 4), ("carried", [(1, 66, 1)], (0, 0, 0, 0, 0, 0), 4),
                                       ("worn", [], (0, 0, 0, 0, 66, 0), 4), ("grass", [], (0, 0, 0, 0, 0, 0), 3)):
            sc = self.gift(1, 220, bag=[(1, 66, 1), (14, 10, 2)], job=1, eq=eq, member=member)
            self.give(sc, 60, page, 0)
            self.compare(sc, f"gift wanted {what}")

    @unittest.skipIf(base.volume.NAME == "infection", "Mutation on: members 18-20 in the save's extension")
    def test_gift_later_members(self):
        # Tsukasa, Subaru and Sora: records, bags, skills and spcPresent in
        # the extension. Their wants are all item 0/0, a weapon.
        for cid in (18, 19, 20):
            sc = self.gift(cid, 220, bag=[(0, 0, 1), (10, 0, 5)], job=0)
            sc.named = True
            self.give(sc, 60, 4, 0)
            self.compare(sc, f"gift member {cid}")

    def test_gift_book(self):
        # A book is read at once, once a copy (sound 92, "used").
        sc = self.gift(3, 200)
        self.give(sc, 60, 2, 0, 1)
        self.compare(sc, "gift book")

    def test_gift_equip(self):
        # A dearer weapon of the member's job is worn (its skills, CalcReal,
        # the thread's breath, the old one to its bag); two: one to the bag.
        # Armour of a class the job takes; one of a class it does not; a
        # weapon of another job; a cheaper one; nothing worn.
        for what, job, eq, row, ups in (("weapon", 1, (20, 20, 20, 20, 0, 0), 0, 0),
                                        ("weapon two", 1, (20, 20, 20, 20, 0, 0), 0, 1),
                                        ("head", 1, (0, 0, 0, 0, 0, 0), 1, 0),
                                        ("head wavemaster", 5, (0, 0, 0, 0, 0, 0), 1, 0),
                                        ("other weapon", 3, (0, 0, 0, 0, 0, 0), 0, 0),
                                        ("cheaper", 1, (0, 0, 0, 0, 30, 0), 0, 0),
                                        ("body bare", 2, (0, -1, 0, 0, 0, 0), 2, 0),
                                        ("leg set", 0, (60, 60, 60, 0, 0, 0), 3, 0)):
            sc = self.gift(7, 220, job=job, eq=eq, member=[(10, 0, 1)])
            self.give(sc, 60, 4, row, ups)
            self.compare(sc, f"gift equip {what}")

    def test_gift_sets(self):
        # The Cats set (68, 67, 66, 67 and weapon 8) and a Goblin set
        # broken.
        sc = self.gift(4, 220, bag=[(6, 68, 1)], job=2, eq=(0, 67, 66, 67, 8, 0))
        self.give(sc, 60, 4, 0)
        self.compare(sc, "gift cats set")
        sc = self.gift(4, 220, bag=[(7, 21, 1)], job=2, eq=(60, 60, 60, 60, 0, 0), skills=(158, 291, 5, 9, 162))
        self.give(sc, 60, 4, 0)
        self.compare(sc, "gift goblin set broken")

    def test_gift_full(self):
        # The member's bag full: its cheapest thing sold (not the first six
        # healing items); nothing cheaper: the gift itself sold; 99 of it:
        # the rest sold; in a field nothing is paid.
        full = [(10, k % 6, 1) for k in range(30)] + [(14, 3, 2), (11, 4, 1)] + [(10, 10 + k, 1) for k in range(8)]
        cheap = [(10, k % 6, 1) for k in range(40)]
        for what, member, area, page, row, ups in (("cheapest", full, 0, 3, 1, 0), ("itself", cheap, 0, 3, 1, 0),
                                                   ("99", [(10, 0, 98)], 0, 0, 0, 3), ("field", full, 1, 3, 1, 0),
                                                   ("field 99", [(10, 0, 98)], 1, 0, 0, 3)):
            sc = self.gift(6, 220, member=member, area=area, gold=9999000)
            self.give(sc, 60, page, row, ups)
            self.compare(sc, f"gift full {what}")

    def test_gift_cancels(self):
        # Cancel the count, the question's Cancel, triangle on equipment,
        # an empty page, cancel back to the list (the tasks woken), and the
        # same opened without mode (nothing to wake).
        sc = self.gift(2, 260)
        sc.pads.update({60: (OK, 0), 70: (CANCEL, 0), 80: (OK, 0), 90: (OK, 0), 115: (0, DOWN), 121: (OK, 0),
                        140: (0, R1), 146: (OK, 0), 155: (0, R1), 161: (0, R1), 167: (0, R1), 175: (OK, 0),
                        190: (CANCEL, 0), 230: (CANCEL, 0)})
        self.compare(sc, "gift cancels")
        for mode in (1, 0):
            sc = self.gift(2, 120, mode=mode)
            sc.pads.update({60: (CANCEL, 0), 90: (CANCEL, 0)})
            self.compare(sc, f"gift back mode {mode}")
        sc = self.gift(2, 120)
        sc.pads.update({60: (0, L1), 70: (TRIANGLE, 0)})
        self.compare(sc, "gift status")
        sc = self.gift(2, 120, bag=[])
        sc.pads.update({60: (OK, 0), 70: (0, R1), 76: (OK, 0), 90: (CANCEL, 0)})
        self.compare(sc, "gift empty")

    def test_random_gift(self):
        for seed in range(1, 9):
            rnd = random.Random(seed)
            bag = rnd.sample(self.BAG, rnd.randrange(0, len(self.BAG) + 1))
            member = [rnd.choice(self.BAG) for _ in range(rnd.choice((0, 5, 40)))]
            sc = self.gift(rnd.randrange(1, 18), 420, bag=bag, story=rnd.randrange(3), job=rnd.randrange(6),
                           eq=tuple(rnd.choice((0, 20, 45, 60)) for _ in range(4)) + (rnd.randrange(8), 0),
                           member=member, area=rnd.choice((0, 0, 1)), gold=rnd.choice((0, 9999900)))
            for f in range(55, 420):
                r = rnd.random()
                if r < 0.2:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN, LEFT, RIGHT, R1, L1)))
                elif r < 0.3:
                    sc.pads[f] = (OK, 0)
                elif r < 0.32:
                    sc.pads[f] = (CANCEL, 0)
                elif r < 0.325:
                    sc.pads[f] = (TRIANGLE, 0)
            self.compare(sc, f"random gift {seed}")


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class BreederPages(Case):
    """BreederMenu (27): the breeder's list, Talk (47) and the "About"
    pages."""

    def breeder(self, frames, server=0, town=0, gift=0):
        sc = Scenario(frames)
        self.kite(sc)
        sc.saves.append((0xCFC + 49, 1, gift))
        sc.talk(Npc(0x200, 10, server=server), 10, 27)
        sc.server = (server, town, 0)
        return sc

    def test_breeder(self):
        # The greeting by server, the rows, cancel.
        for server in (0, 2, 4):
            sc = self.breeder(80, server)
            sc.pads.update({30: (0, DOWN), 38: (0, DOWN), 46: (0, DOWN), 54: (0, DOWN), 60: (0, UP), 70: (CANCEL, 0)})
            self.compare(sc, f"breeder server {server}")

    def test_breeder_about(self):
        # The three pages, in town 1 and elsewhere: their lines chained,
        # then the list again (no greeting).
        for town in (1, 0):
            for row in (1, 2, 3):
                sc = self.breeder(260, town=town)
                for k in range(row):
                    sc.pads[25 + 8 * k] = (0, DOWN)
                sc.pads[60] = (OK, 0)
                for f in range(75, 230, 8):
                    sc.pads[f] = (OK, 0)
                sc.pads[245] = (CANCEL, 0)
                self.compare(sc, f"breeder about town {town} row {row}")

    def test_breeder_talk(self):
        # Talk: the breeder's line by talkNum (3 with key item 49, else 1).
        for gift in (0, 1):
            sc = self.breeder(160, server=1, gift=gift)
            sc.pads[30] = (OK, 0)
            for f in range(50, 130, 8):
                sc.pads[f] = (OK, 0)
            sc.pads[145] = (CANCEL, 0)
            self.compare(sc, f"breeder talk gift {gift}")

    def test_breeder_lost(self):
        # The target lost under the list: CloseMenu, then the keys that
        # frame (nothing, cancel, OK off Talk, OK on Talk); lost as a page
        # opens; opened with no target.
        for pad in (0, CANCEL, OK):
            for row in (0, 1):
                sc = self.breeder(80)
                if row:
                    sc.pads[25] = (0, DOWN)
                sc.drops.append(40)
                if pad:
                    sc.pads[41] = (pad, 0)
                self.compare(sc, f"breeder lost pad {pad} row {row}")
        sc = self.breeder(80)
        sc.pads.update({25: (0, DOWN), 40: (OK, 0)})
        sc.drops.append(41)
        self.compare(sc, "breeder lost page")
        sc = Scenario(40)
        self.kite(sc)
        sc.opens[10] = 27
        sc.modes[10] = 1
        sc.firsts.add(10)
        self.compare(sc, "breeder no target")

    @staticmethod
    def race_save(sc, server, pens=3, mail=4, records=()):
        """The town's pens holding grown Grunties (growth[server].type[k]),
        mail 324's state, and the player's ranks (rank, frames, row)."""
        for k in range(pens):
            sc.saves.append((GROWTH[0] + 24 * server + 0xE + 2 * k, 2, 1))
        sc.saves.append((0x2264 + 324, 1, mail))
        for rank, time, row in records:
            at = 0x8432 + 12 * (server - 1) + 4 * rank
            sc.saves += [(at, 2, time), (at + 2, 2, row)]

    @unittest.skipIf(base.volume.NAME == "infection", "Mutation on: the Flag Race")
    def test_breeder_race(self):
        # Talk, Flag Race and Rankings once the town's three pens hold grown
        # Grunties and mail 324 is read (4 and up); else the About rows.
        for pens, mail in ((3, 4), (3, 6), (3, 2), (2, 4), (3, 0)):
            sc = self.breeder(80, server=1)
            self.race_save(sc, 1, pens, mail)
            sc.pads.update({30: (0, DOWN), 38: (0, DOWN), 46: (0, DOWN), 60: (0, UP), 70: (CANCEL, 0)})
            self.compare(sc, f"breeder race pens {pens} mail {mail}")

    @unittest.skipIf(base.volume.NAME == "infection", "Mutation on: the Flag Race")
    def test_breeder_race_talk(self):
        # Talk: once the town's three pens hold grown Grunties and mail 324
        # is at 3 or more, the race's line (TalkMenu, MUT gcmn 0x0056d2a8);
        # else the line by server.
        for pens, mail in ((3, 3), (3, 4), (3, 2), (2, 4)):
            sc = self.breeder(160, server=2)
            self.race_save(sc, 2, pens, mail)
            sc.pads[30] = (OK, 0)
            for f in range(50, 130, 8):
                sc.pads[f] = (OK, 0)
            sc.pads[145] = (CANCEL, 0)
            self.compare(sc, f"breeder race talk pens {pens} mail {mail}")

    @unittest.skipIf(base.volume.NAME == "infection", "Mutation on: the Flag Race")
    def test_rankings(self):
        # Rankings (89): the town's three, the save's times where the player
        # holds a rank; cancel or OK back to the breeder's list.
        cases = ((1, (), CANCEL), (2, ((0, 1234, 148),), OK), (3, ((1, 601, 150), (2, 3599, 145)), CANCEL),
                 (4, ((0, 30, 152), (1, 1800, 153), (2, 32767, 145)), OK))
        for server, records, key in cases:
            sc = self.breeder(200, server=server)
            self.race_save(sc, server, records=records)
            sc.pads.update({25: (0, DOWN), 33: (0, DOWN), 45: (OK, 0), 120: (key, 0), 170: (CANCEL, 0)})
            self.compare(sc, f"rankings server {server}")

    def flag_race(self, frames, server=1, gold=1000, records=(), prizes=(), walls=0):
        """The breeder's list, Flag Race (menu 88) chosen at 40; Kite's
        gold, the town's ranks, the prizes given (server, rank, n) and the
        wallpapers held; the save's runs the race writes watched."""
        sc = self.breeder(frames, server=server)
        self.race_save(sc, server, records=records)
        sc.saves.append((shop.GOLD[0], 4, gold))
        for srv, rank, n in prizes:
            sc.saves.append((0x8462 + 3 * (srv - 1) + rank - 1, 1, n))
        sc.saves.append((0x2238, 4, walls))
        sc.watches = [shop.GOLD, (0x8432, 48), (0x8462, 12), (0x2238, 12)]
        sc.pads.update({25: (0, DOWN), 40: (OK, 0)})
        return sc

    @unittest.skipIf(base.volume.NAME == "infection", "Mutation on: the Flag Race")
    def test_flag_race_declined(self):
        # The greeting, "It costs 100GP": No, or Yes without the money;
        # the breeder's word, back to the list, cancel.
        for gold, sel in ((1000, 1), (50, 0), (1000, -1)):
            sc = self.flag_race(260, gold=gold)
            for f in range(55, 120, 10):
                sc.pads[f] = (OK, 0)
            if sel == 1:
                sc.pads[130] = (0, DOWN)
            sc.pads[140] = (CANCEL, 0) if sel == -1 else (OK, 0)
            for f in range(160, 230, 10):
                sc.pads[f] = (OK, 0)
            sc.pads[245] = (CANCEL, 0)
            sc.races[1] = None
            self.compare(sc, f"flag race declined gold {gold} sel {sel}")

    def to_race(self, sc, row=0):
        """Through the greeting (two records), Yes to the cost, Grunty `row`
        and Yes to the start: the race asked for at 201; its object there
        from the start, idle."""
        sc.pads.update({55: (OK, 0), 65: (OK, 0), 90: (OK, 0), 120: (OK, 0), 140: (OK, 0)})
        for k in range(row):
            sc.pads[160 + 6 * k] = (0, DOWN)
        sc.pads.update({180: (OK, 0), 200: (OK, 0)})
        sc.races[1] = (0, 0, 0, 0)

    @unittest.skipIf(base.volume.NAME == "infection", "Mutation on: the Flag Race")
    def test_flag_race_paused(self):
        # The race runs: paused and continued; paused, Quit, No (the pause
        # again on Quit), Quit, Yes: the timer stopped, the state over; the
        # result (none), the breeder's word, the prize through GetItemMenu.
        for row in (0, 1, 2):
            sc = self.flag_race(600)
            self.to_race(sc, row)
            sc.races[205] = (0, 0, 1, 150)
            sc.pads.update({215: (CANCEL, 0), 230: (OK, 0), 250: (CANCEL, 0), 262: (0, DOWN), 272: (OK, 0),
                            290: (OK, 0), 305: (OK, 0), 320: (0, UP), 330: (OK, 0)})
            sc.races[360] = (2, 0, 0, 999)
            for f in range(380, 580, 12):
                sc.pads[f] = (OK, 0)
            self.compare(sc, f"flag race paused row {row}")

    @unittest.skipIf(base.volume.NAME == "infection", "Mutation on: the Flag Race")
    def test_flag_race_results(self):
        # The result: the time, the Rankings page with the new rank
        # blinking (1-3), the breeder's word; a town's first win's
        # wallpaper (and with three other towns' first wins the Grand
        # Slam's); a rank's prizes given out; near the third; none.
        cases = (
            (1, 1, (), ((0, 1234, 146),)),
            (1, 1, ((2, 1, 1), (3, 1, 2), (4, 1, 1)), ((0, 1234, 146),)),
            (2, 2, ((2, 2, 3),), ((1, 2000, 148),)),
            (1, 3, ((1, 3, 1),), ((2, 3599, 145),)),
            (3, 4, (), ()),
            (4, 0, (), ()),
            (1, 1, ((1, 1, 3),), ((0, 600, 146),)),
        )
        for server, rank, prizes, records in cases:
            sc = self.flag_race(900, server=server, prizes=prizes, records=records)
            self.to_race(sc, rank % 3)
            sc.races[220] = (2, rank, 0, 1234 + 100 * rank)
            for f in range(240, 880, 12):
                sc.pads[f] = (OK, 0)
            self.compare(sc, f"flag race server {server} rank {rank} prizes {prizes}")

    def test_random_breeder(self):
        for seed in range(1, 7):
            rnd = random.Random(seed)
            sc = self.breeder(300, server=rnd.randrange(5), town=rnd.randrange(3), gift=rnd.randrange(2))
            for f in range(60, 300, 60):
                sc.opens[f] = 27
                sc.modes[f] = 1
                if rnd.random() < 0.5:
                    sc.firsts.add(f)
            for f in range(12, 300):
                r = rnd.random()
                if r < 0.15:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN)))
                elif r < 0.22:
                    sc.pads[f] = (rnd.choice((OK, OK, CANCEL)), 0)
            self.compare(sc, f"random breeder {seed}")


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class DogPages(Case):
    """NorainuMenu (44): a stray dog's list (Talk alone), Talk (47)."""

    def dog(self, frames, row=141, rands=(0,)):
        sc = Scenario(frames)
        self.kite(sc)
        sc.talk(Npc(0x200, row), 10, 44)
        sc.rands = list(rands)
        return sc

    def test_dog(self):
        # The greeting (its voice), then cancel.
        for row in (141, 144):
            sc = self.dog(60, row)
            sc.pads[40] = (CANCEL, 0)
            self.compare(sc, f"dog {row}")

    def test_dog_talk(self):
        # Talk: the line by talkNum (ccRand() & 3: 1, 3, 5, 1), back to
        # the list (no greeting, talkNum drawn again), cancel.
        for r in range(4):
            sc = self.dog(200, 142, rands=(r, 3 - r))
            sc.pads[30] = (OK, 0)
            for f in range(50, 150, 8):
                sc.pads[f] = (OK, 0)
            sc.pads[180] = (CANCEL, 0)
            self.compare(sc, f"dog talk {r}")

    def test_dog_lost(self):
        # The target lost under the list: CloseMenu, then the keys that
        # frame; opened with no target.
        for pad in (0, CANCEL, OK):
            sc = self.dog(80)
            sc.drops.append(40)
            if pad:
                sc.pads[41] = (pad, 0)
            self.compare(sc, f"dog lost pad {pad}")
        sc = Scenario(40)
        self.kite(sc)
        sc.opens[10] = 44
        sc.modes[10] = 1
        sc.firsts.add(10)
        self.compare(sc, "dog no target")


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class GruntyPages(Case):
    """InuMenu (46), a young Grunty's (Talk, Give Food, and its STATUS
    window), and OtonainuMenu (45), a grown one's (Talk, Trade); Talk (47)
    and Give Food (56) from them."""

    def grunty(self, frames, row=154, level=0, size=0, food=0, foods=(), town=1):
        sc = Scenario(frames)
        self.kite(sc)
        g = Grunty(0x280, row, 0, level, size, food, tag=(256, 200))
        sc.talk(g, 10, 45 if row < 154 else 46)
        sc.server = (1, town, 0)
        for k in range(26, 42):
            sc.saves.append((0xCFC + k, 1, dict(foods).get(k, 0)))
        growth = (level, size, 3, -2, 7, 0, 12)
        sc.saves += [(0x2194 + 24 * town + 2 * k, 2, v & 0xFFFF) for k, v in enumerate(growth)]
        sc.watches = [IMP_ITEMS, GROWTH]
        return sc

    def test_inu(self):
        # The fade in, the camera (11), its greeting (its line by msgNum),
        # the STATUS window; cancel: the fade out, the camera back, it walks
        # on (0).
        for row, level, size, food in ((154, 0, 0, -1), (155, 1, 6, 5), (156, 2, 12, 9), (157, 3, 24, 33)):
            sc = self.grunty(160, row, level, size, food)
            sc.msg_nums[5] = food & 0xFF if size else 2
            sc.pads[70] = (CANCEL, 0)
            self.compare(sc, f"inu {row}")

    def test_inu_talk(self):
        # Talk: EntryAffect 15, its line (talkNum its msgNum), back to the
        # list (no fade), Talk again, cancel.
        for row, level, size, food in ((154, 0, 0, -1), (156, 2, 12, 11), (157, 4, 30, 7)):
            sc = self.grunty(260, row, level, size, food)
            sc.pads[60] = (OK, 0)
            for f in range(80, 150, 8):
                sc.pads[f] = (OK, 0)
            sc.pads[160] = (OK, 0)
            for f in range(175, 220, 8):
                sc.pads[f] = (OK, 0)
            sc.pads[235] = (CANCEL, 0)
            self.compare(sc, f"inu talk {row}")

    def test_inu_food(self):
        # Give Food with none: "There is no food.", the list again; with
        # some: foodMode and chatFlag, Give Food (56) opens on the Grunty.
        sc = self.grunty(220, 155, 1, 6, 5)
        sc.pads.update({60: (0, DOWN), 70: (OK, 0), 110: (OK, 0), 140: (CANCEL, 0)})
        self.compare(sc, "inu no food")
        sc = self.grunty(200, 156, 2, 12, 9, foods=((27, 2), (40, 1)))
        sc.pads.update({60: (0, DOWN), 70: (OK, 0), 100: (CANCEL, 0), 140: (CANCEL, 0)})
        self.compare(sc, "inu give food")

    def test_otonainu(self):
        # A grown one: the greeting, Talk (line 7, talkNum from its msgNum
        # after EntryAffect 14), back, Trade (the target dropped, another
        # harness's page), cancel.
        for row in (145, 146, 147):
            sc = self.grunty(200, row, 4, 30, 9)
            sc.pads[30] = (OK, 0)
            for f in range(45, 110, 8):
                sc.pads[f] = (OK, 0)
            sc.pads[130] = (CANCEL, 0)
            self.compare(sc, f"otonainu {row}")
        sc = self.grunty(120, 146, 4, 30, 9)
        sc.pads.update({30: (0, DOWN), 40: (OK, 0)})
        self.compare(sc, "otonainu trade")

    def test_grunty_lost(self):
        # The target gone under OtonainuMenu's list, with each key; opened
        # with no target.
        for pad in (0, CANCEL, OK):
            sc = self.grunty(80, 145, 4, 30, 9)
            sc.drops.append(40)
            if pad:
                sc.pads[41] = (pad, 0)
            self.compare(sc, f"otonainu lost pad {pad}")
        for menu in (45, 46):
            sc = Scenario(40)
            self.kite(sc)
            sc.opens[10] = menu
            sc.modes[10] = 1
            sc.firsts.add(10)
            self.compare(sc, f"grunty menu {menu} no target")


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class BreedingPages(Case):
    """BreedingMenu (56): Give Food to the Grunty (cmndTargetPrev), opened
    as the Grunty's own menu (not ported) opens it, the Grunty's growth
    scripted frame by frame."""

    FOODS = ((26, 3), (30, 5), (41, 2))
    GROWTH = (3, 10, -5, 0, 7, 20, 1)

    def breeding(self, frames, foods=FOODS, server=0, growth=GROWTH, exist=0, gift=0, row=154):
        sc = Scenario(frames)
        self.kite(sc)
        sc.server = (server, 0, 0)
        sc.grunty = Grunty(0x280, row, exist)
        for k in range(26, 42):
            sc.saves.append((0xCFC + k, 1, dict(foods).get(k, 0)))
        sc.saves.append((0xCFC + 49, 1, gift))
        sc.saves += [(0x2194 + 24 * server + 2 * k, 2, v & 0xFFFF) for k, v in enumerate(growth)]
        sc.watches = [IMP_ITEMS, GROWTH]
        sc.opens[10] = 56
        sc.modes[10] = 1
        sc.firsts.add(10)
        return sc

    def feed(self, sc, f, row=0, ups=0, answer=OK):
        """From frame `f`: the food at `row`, OK, `ups` more, OK, the
        question's answer. The frame after."""
        for _ in range(row):
            sc.pads[f] = (0, DOWN)
            f += 8
        sc.pads[f] = (OK, 0)
        f += 8
        for _ in range(ups):
            sc.pads[f] = (0, UP)
            f += 6
        sc.pads[f] = (OK, 0)
        f += 25
        if answer == CANCEL:
            sc.pads[f] = (0, DOWN)
            f += 6
        sc.pads[f] = (OK, 0)
        return f + 1

    def test_feed(self):
        # Fed, it eats, grows up (the two records), speaks (its voice by
        # id), then: key item 49 given (sound 74, the tasks asleep and
        # woken), the fade to black and back, the window shut.
        for row, msg in ((154, 1), (156, 0), (155, 2)):
            sc = self.breeding(560, row=row)
            f = self.feed(sc, 30, 1, 2)
            sc.growth.update({f + 5: 1, f + 35: 2, f + 195: 3})
            sc.msg_nums[f + 195] = msg
            for g in range(f + 205, f + 330, 8):
                sc.pads[g] = (OK, 0)
            self.compare(sc, f"feed row {row} msg {msg}")

    def test_feed_ends(self):
        # It stays (exist): until it goes off (5); key item 49 held: the
        # fade at once; it goes off while eating; it eats and is idle
        # again: the foods again.
        for what, exist, gift, growth in (("exist", 1, 0, {5: 1, 40: 3, 120: 5}), ("gift", 0, 1, {5: 1, 40: 3}),
                                          ("off", 0, 0, {5: 1, 40: 5}), ("idle", 0, 0, {5: 1, 40: 0})):
            sc = self.breeding(300, exist=exist, gift=gift)
            f = self.feed(sc, 30)
            sc.growth.update({f + k: v for k, v in growth.items()})
            for g in range(f + 50, f + 200, 8):
                sc.pads[g] = (OK, 0)
            self.compare(sc, f"feed {what}")

    def test_feed_cancels(self):
        # The question's Cancel (the Grunty idle: the foods again), the
        # count's cancel, the list's cancel (the Grunty the target again).
        sc = self.breeding(200)
        f = self.feed(sc, 30, 2, 1, CANCEL)
        sc.pads.update({f + 20: (OK, 0), f + 30: (CANCEL, 0), f + 45: (CANCEL, 0)})
        self.compare(sc, "feed cancels")

    def test_foods(self):
        # No food: straight back. Every food (a scrolled list), the count
        # left, right, up and down; another server's stats.
        sc = self.breeding(60, foods=())
        self.compare(sc, "no food")
        allf = tuple((k, k - 20) for k in range(26, 42))
        sc = self.breeding(300, foods=allf, server=3, growth=(1, -100, 999, -999, 50, -7, 1234))
        for k in range(18):
            sc.pads[30 + 6 * k] = (0, DOWN)
        sc.pads.update({150: (0, UP), 160: (OK, 0), 170: (0, LEFT), 178: (0, LEFT), 186: (0, RIGHT), 194: (0, UP),
                        200: (0, DOWN), 206: (0, DOWN), 214: (0, RIGHT), 222: (0, RIGHT), 230: (CANCEL, 0),
                        240: (0, UP), 248: (OK, 0), 256: (0, UP), 270: (CANCEL, 0), 285: (CANCEL, 0)})
        self.compare(sc, "foods")

    def test_random_feed(self):
        for seed in range(1, 9):
            rnd = random.Random(seed)
            foods = tuple((k, rnd.randrange(1, 12)) for k in rnd.sample(range(26, 42), rnd.randrange(0, 17)))
            sc = self.breeding(420, foods=foods, server=rnd.randrange(5), exist=rnd.randrange(2),
                               gift=rnd.randrange(2), row=rnd.choice((154, 155, 156, 157)))
            g = 0
            for f in range(40, 420, 25):
                if rnd.random() < 0.5:
                    g = rnd.choice((0, 1, 1, 2, 3, 5))
                    sc.growth[f] = g
                    sc.msg_nums[f] = rnd.randrange(3)
            for f in range(15, 420):
                r = rnd.random()
                if r < 0.15:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN, LEFT, RIGHT)))
                elif r < 0.25:
                    sc.pads[f] = (OK, 0)
                elif r < 0.27:
                    sc.pads[f] = (CANCEL, 0)
            self.compare(sc, f"random feed {seed}")


# From Mutation on: the item registry (the extension's +0x754, 16 rows of
# four words), the desktop lists and ITEM COMPLETE's status.
REGISTRY = (base.EXT + 0x754, 256)
DESKTOP_LISTS = (0x2238, 36)
EVENT_NPC_STATUS = (0x64F8 + 52, 1)
# Each category's registry row and its listed ids (Item List's).
SHELVES = {0: (0, 60), 1: (1, 60), 2: (2, 80), 3: (3, 62), 4: (4, 60), 5: (5, 60), 6: (6, 60), 7: (7, 60),
           8: (8, 60), 9: (9, 60), 11: (10, 72), 14: (12, 10), -2: (13, 27), -3: (14, 16), -4: (15, 12)}


@unittest.skipUnless(os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
@unittest.skipIf(base.volume.NAME == "infection", "Mutation on: the Event NPC")
class EventNpcPages(Case):
    """The Event NPC (npcTbl 175 + town, flags 0x10000000): its list (90)
    and Item List (91), which registers what Kite holds, wears and keeps
    at Elf's Haven, and gives the desktop items once all is in."""

    def event_npc(self, frames, town=1, bag=(), haven=(), keys=(), worn=None, full=False, missing=()):
        """Spoken to in town `town`; the bag's and Elf's Haven's
        (category, id, count)s; key items held (index, count); Kite's
        weapon and four armours; the registry full but `missing` (category,
        id)s, or empty."""
        sc = Scenario(frames)
        self.kite(sc)
        sc.talk(Npc(0x200, 175 + town, server=town), 10, 90)
        sc.server = (town, town, 0)
        self.items(sc, bag)
        self.items(sc, haven, pl=True)
        for k, n in keys:
            sc.saves.append((0xCFC + k, 1, n))
        sp = base.spc_param(0)
        for k, v in enumerate(worn or (0xFFFF,) * 5):
            sc.saves.append((sp + (0xD0 if k == 0 else 0xC6 + 2 * k), 2, v & 0xFFFF))
        words = [0] * 64
        if full:
            for cat, (row, ids) in SHELVES.items():
                for i in range(ids):
                    if (cat, i) not in missing:
                        words[4 * row + i // 32] |= 1 << (i % 32)
        for k, w in enumerate(words):
            sc.saves.append((REGISTRY[0] + 4 * k, 4, w))
        sc.saves.append((EVENT_NPC_STATUS[0], 1, 1))
        sc.watches = [REGISTRY, DESKTOP_LISTS, EVENT_NPC_STATUS]
        return sc

    def test_event_npc_list(self):
        # The greeting by town, the two rows, cancel; and Talk.
        for town in (1, 2, 3):
            sc = self.event_npc(80, town)
            sc.pads.update({30: (0, DOWN), 38: (0, UP), 46: (0, DOWN), 60: (CANCEL, 0)})
            self.compare(sc, f"event npc town {town}")
        sc = self.event_npc(200, 1)
        sc.pads[30] = (OK, 0)
        for f in range(50, 180, 10):
            sc.pads[f] = (OK, 0)
        self.compare(sc, "event npc talk")

    def item_list(self, sc, at=30):
        """Item List chosen from the NPC's list at `at`."""
        sc.pads.update({at - 8: (0, DOWN), at: (OK, 0)})

    def test_item_list_registers(self):
        # Items in the bag, at Elf's Haven, worn and key items: the two
        # windows of what came in, then the groups with the count; each
        # group's pages, scrolled; back to the groups, back to the list.
        sc = self.event_npc(700, bag=((0, 3, 1), (10, 5, 2), (13, 1, 1), (11, 8, 1), (0, 70, 1), (10, 22, 1)),
                            haven=((6, 4, 1), (14, 2, 1), (14, 15, 1)), keys=((27, 1), (3, 2)),
                            worn=(12, 40, 41, 42, 43))
        self.item_list(sc)
        for f in (60, 90, 120, 150):
            sc.pads[f] = (OK, 0)
        sc.pads.update({200: (0, DOWN), 210: (0, DOWN), 220: (0, UP), 230: (OK, 0), 260: (0, DOWN),
                        268: (0, DOWN), 276: (0, RIGHT), 290: (0, RIGHT), 304: (0, LEFT), 320: (0, UP),
                        330: (CANCEL, 0), 360: (0, DOWN), 370: (0, DOWN), 380: (OK, 0)})
        for k in range(12):
            sc.pads[410 + 6 * k] = (0, DOWN)
        sc.pads.update({490: (0, RIGHT), 500: (0, RIGHT), 510: (0, RIGHT), 530: (CANCEL, 0), 560: (CANCEL, 0),
                        600: (CANCEL, 0)})
        self.compare(sc, "item list registers")

    def test_item_list_nothing_new(self):
        # Nothing new and one item missing: "There are no items you can
        # register", then the groups; the Key Items group's two pages.
        sc = self.event_npc(300, full=True, missing=((5, 10),))
        self.item_list(sc)
        for f in (60, 75):
            sc.pads[f] = (OK, 0)
        sc.pads.update({120: (0, UP), 130: (OK, 0), 170: (0, RIGHT), 185: (0, LEFT), 200: (CANCEL, 0),
                        230: (CANCEL, 0), 270: (CANCEL, 0)})
        self.compare(sc, "item list nothing new")

    def test_item_list_completes(self):
        # The last items come in from the bag and Elf's Haven: the NPC's
        # word, wallpaper 56, BGM 51 and movies 90-96, ITEM COMPLETE's
        # status cleared, the menu shut.
        sc = self.event_npc(1100, bag=((1, 7, 1),), haven=((10, 9, 1),), full=True,
                            missing=((1, 7), (-2, 9)))
        self.item_list(sc)
        for f in range(55, 1000, 12):
            sc.pads[f] = (OK, 0)
        self.compare(sc, "item list completes")

    def test_random_item_list(self):
        # Random holdings, a random registry and random keys through the
        # lists and pages.
        for seed in range(1, 7):
            rnd = random.Random(seed)
            cats = (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14)
            pick = lambda n: tuple((rnd.choice(cats), rnd.randrange(0, 90), 1) for _ in range(n))  # noqa: E731
            keys = tuple((k, rnd.randrange(1, 3)) for k in rnd.sample(range(0, 42), rnd.randrange(0, 8)))
            missing = tuple((c, rnd.randrange(0, 12)) for c in rnd.sample(sorted(SHELVES), 4))
            sc = self.event_npc(600, town=rnd.randrange(1, 5), bag=pick(rnd.randrange(0, 12)),
                                haven=pick(rnd.randrange(0, 12)), keys=keys, full=rnd.random() < 0.5,
                                missing=missing, worn=tuple(rnd.randrange(0, 70) for _ in range(5)))
            self.item_list(sc)
            for f in range(40, 600):
                r = rnd.random()
                if r < 0.15:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN, LEFT, RIGHT)))
                elif r < 0.22:
                    sc.pads[f] = (OK, 0)
                elif r < 0.25:
                    sc.pads[f] = (CANCEL, 0)
            self.compare(sc, f"random item list {seed}")

    def test_item_list_complete_already(self):
        # Every item in and none new: straight to the groups.
        sc = self.event_npc(160, full=True)
        self.item_list(sc)
        sc.pads.update({90: (OK, 0), 130: (CANCEL, 0), 145: (CANCEL, 0)})
        self.compare(sc, "item list complete already")


if __name__ == "__main__":
    unittest.main()
