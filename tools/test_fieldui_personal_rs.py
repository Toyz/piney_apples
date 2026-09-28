#!/usr/bin/env python3
"""crates/piney-fieldui's PERSONAL pages against the game's own ccThMenu in
tools/eemu.py, frame by frame: Key Items (6), Discard Item (7), Status (8)
with a member's items (31), Equipment (63) and an item's status (64), Gate
Out (10), Log Out (11) and TransFieldMenu (86).

Built on tools/test_fieldui_rs.py (its hooks, scenario runner and
comparisons: the menu's state, every sprite packet, the kanji rows' text,
the message window, the sounds and every request); this file adds the world
these menus read (the party's records in the save, ccPgAdultCheck, rand,
checkPartyAnnihilation, the area) and the calls they make outside the menu
(ccGame::ChangeArea and ChangeRequest, WORLD_MAN::GoField,
ccSPC::DeleteNoPartyMember), and watches the save's bytes they write.

Skipped when the disc is not extracted or cargo is missing.
"""

import os
import random
import shutil
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc
import test_fieldui_rs as base  # noqa: E402

OK, CANCEL, TRIANGLE, SQUARE = base.OK, base.CANCEL, base.TRIANGLE, base.SQUARE
UP, RIGHT, DOWN, LEFT = base.UP, base.RIGHT, base.DOWN, base.LEFT
L1, R1 = 0x4, 0x8
SAVE, GAME = base.SAVE, base.GAME
SPC_PARAM, IMP_ITEM_LIST, PLCOL, PARODY = 0x7488, 0xCFC, 0x6771, 0x842B
SPC_MANAGER = inf_va(0x00730340)     # ccSpcManager: five {id, ..., +0x1c ccChar*} of 44 bytes
sym = base.sym


def sx(v):
    return v - (1 << 32) if v & 0x80000000 else v


class Scenario(base.Scenario):
    def __init__(self, frames):
        super().__init__(frames)
        self.grunty = {}          # slot -> ccPgAdultCheck's answer
        self.rand = 0
        self.wiped = 0
        self.field = 0
        self.town = 0
        self.registry = []        # (id, bootParam): ccSpcManager entries after the party's

    def spc_entries(self):
        party = [(c.id, 0) for c in self.party.values() if c.types & 7 and 0 <= c.id < 18]
        return (party + list(self.registry))[:5]

    def lines(self):
        out = [f"grunty {k} {v}" for k, v in sorted(self.grunty.items())]
        out += [f"rng {self.rand}", f"wiped {self.wiped}", f"field {self.field}", f"town {self.town}"]
        out += [f"spc {cid} {boot}" for cid, boot in self.spc_entries()]
        self.extra = out
        return super().lines()


class Game(base.Game):
    """The base run with the calls these menus make outside the menu."""

    def make_char(self, c):
        a = super().make_char(c)
        if c.types & 7 and 0 <= c.id < 18:
            # A party member's personality is its record in the save.
            self.m.store(a + 0x04, 4, SAVE + SPC_PARAM + 0xDC * c.id)
        return a

    def setup(self):
        super().setup()
        m, sc = self.m, self.sc
        m.store(GAME + 0x20, 4, sc.town)
        m.store(GAME + 0x24, 4, sc.field)
        # ccSpcManager: the party's characters by member id (CheckSpc).
        chars = {c.id: a for a, c in self.chars.items() if c.types & 7 and c in self.sc.party.values()}
        entries = sc.spc_entries()
        for k in range(5):
            e = SPC_MANAGER + 44 * k
            if k < len(entries):
                cid, boot = entries[k]
                m.store(e, 4, cid)
                m.store(e + 8, 4, boot)
                m.store(e + 0x1C, 4, chars.get(cid, 0))
            else:
                m.store(e, 4, 0xFFFFFFFF)
        m.store(SPC_MANAGER + 0xDC, 4, len(entries))

    def hooks(self):
        super().hooks()
        m, sc = self.m, self.sc
        h = m.hooks

        def hook(name, fn):
            h[sym(name)] = fn

        hook("ccPgAdultCheck__Fii", lambda mm, server, k, *a: sc.grunty.get(k, -1) & 0xFFFFFFFF)
        hook("rand", lambda mm, *a: sc.rand)
        hook("checkPartyAnnihilation__Fv", lambda mm, *a: sc.wiped)

        def use_item(mm, u, t, code, arg, *a):
            code = code - (1 << 32) if code & 0x80000000 else code
            if arg:
                self.ev("use_item_arg", self.handle(t), code, arg - (1 << 32) if arg & 0x80000000 else arg)
            else:
                self.ev("use_item", self.handle(t), code)
            return 0
        hook("ccUseItemRequest__FP6ccCharP6ccCharii", use_item)
        hook("DeleteNoPartyMember__5ccSPCFv", lambda mm, *a: self.ev("delete_no_party") or 0)
        hook("ChangeArea__6ccGameFii", lambda mm, g, a, n, *x: self.ev("change_area", a, n) or 0)
        hook("ChangeRequest__6ccGameFii", lambda mm, g, a, n, *x: self.ev("change_mode", a, n) or 0)
        hook("GoField__9WORLD_MANFv", lambda mm, *a: self.ev("go_field") or 0)

        def del_member(mm, party, slot, *a):
            # ccParty::DelMember without disbandSpc.
            self.ev("del_member", slot)
            if 0 < slot < 3 and mm.load(party + 0x0C + 4 * slot, 4) != 0xFFFFFFFF:
                mm.store(party + 4 * slot, 4, 0)
                mm.store(party + 0x0C + 4 * slot, 4, 0xFFFFFFFF)
                mm.store(party + 0x18, 4, mm.load(party + 0x18, 4) - 1)
            return 0
        hook("DelMember__7ccPartyFi", del_member)

        # ccChar::CalcReal runs as the game has it, noted.
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

    def run_threads(self):
        # ccThEquipMenu: ccSPC::ChangeEquip(category, member, item) at once.
        for t, fn in self.threads:
            if fn == sym("ccThEquipMenu__FP6ccTscb") and self.m.load(t + 20, 4):
                c = self.m.load(SPC_MANAGER + 44 * self.m.load(t + 28, 4) + 0x1C, 4)
                self.ev("change_equip", self.handle(c), sx(self.m.load(t + 24, 4)), sx(self.m.load(t + 32, 4)))
                self.m.store(t + 20, 4, 0)
        super().run_threads()


class PersonalAgainstGame(unittest.TestCase):
    game_class = Game
    compare = base.FieldUiAgainstGame.compare
    party = base.FieldUiAgainstGame.party
    battle = base.FieldUiAgainstGame.battle
    items = base.FieldUiAgainstGame.items

    @classmethod
    def setUpClass(cls):
        if not (os.path.exists(base.ELF) and os.path.exists(base.ISO) and shutil.which("cargo")):
            raise unittest.SkipTest("needs the extracted disc and cargo")
        subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-fieldui", "--features",
                        "trace", "--example", "fieldui_probe"], cwd=base.ROOT, check=True)

    def key_items(self, sc, ids):
        for k in range(291):
            sc.saves.append((IMP_ITEM_LIST + k, 1, ids.get(k, 0)))
        sc.watch.append((IMP_ITEM_LIST, 291))

    def personal(self, sc, area, row, at=10):
        """PERSONAL opened at frame `at` and row `row` chosen."""
        sc.opens[at] = area
        f = at + 12
        for _ in range(row):
            sc.pads[f] = (0, DOWN)
            f += 3
        sc.pads[f] = (OK, 0)
        return f

    # --- Key Items --------------------------------------------------------

    def test_key_item_pages(self):
        held = {0: 1, 2: 3, 27: 2, 30: 1, 42: 1, 44: 2, 48: 1, 49: 1, 50: 1, 60: 1, 61: 1, 70: 4, 71: 1,
                281: 1, 287: 1, 273: 1, 275: 2}
        for plcol, parody in ((0, 0), (1, 1), (1, 0)):
            sc = Scenario(160)
            self.party(sc, 2)
            sc.game = (1, 0, 0, 0)
            self.key_items(sc, held)
            sc.saves += [(PLCOL, 1, plcol), (PARODY, 1, parody)]
            f = self.personal(sc, 1, 2)
            for k, p in enumerate((DOWN, DOWN, UP, UP, UP, R1, DOWN, R1, R1, L1, R1, R1, R1, DOWN, DOWN, DOWN)):
                sc.pads[f + 12 + 5 * k] = (0, p)
            sc.pads[f + 100] = (CANCEL, 0)
            sc.pads[f + 115] = (CANCEL, 0)
            self.compare(sc, f"key item pages {plcol} {parody}")

    def test_key_item_use(self):
        held = {0: 1, 44: 2, 48: 1, 49: 1, 50: 1, 273: 1}
        # In a field: Virus Core (refused), Epitaph 02 (used), Imp's Pin
        # (refused), a Ryu Book (towns only), the Grunty Flute without a
        # Grunty, then with one.
        sc = Scenario(330)
        self.party(sc, 2)
        sc.game = (1, 0, 0, 0)
        sc.saves += [(PLCOL, 1, 1)]
        self.key_items(sc, held)
        f = self.personal(sc, 1, 2)
        sc.pads.update({f + 12: (OK, 0), f + 40: (OK, 0),          # Epitaph 02
                        f + 70: (0, DOWN), f + 75: (0, DOWN), f + 80: (OK, 0), f + 100: (OK, 0),  # Imp's Pin
                        f + 120: (0, UP), f + 125: (OK, 0), f + 150: (OK, 0),  # the flute: no Grunty
                        f + 175: (0, R1), f + 180: (0, R1), f + 185: (0, L1), f + 190: (0, L1),
                        f + 195: (0, R1), f + 200: (0, R1), f + 205: (0, R1), f + 212: (OK, 0),
                        f + 240: (OK, 0), f + 260: (0, L1), f + 265: (0, L1), f + 270: (0, L1),
                        f + 280: (OK, 0)})
        self.compare(sc, "key item use, field")
        # The flute with a Grunty in slot 2; rand picks slot 1 first.
        sc = Scenario(90)
        self.party(sc, 1)
        sc.game = (1, 0, 0, 0)
        sc.grunty = {2: 5}
        sc.rand = 7
        self.key_items(sc, {49: 1})
        f = self.personal(sc, 1, 2)
        sc.pads[f + 12] = (OK, 0)
        self.compare(sc, "grunty flute")
        # The flute refused: field 13, a battle, a dungeon, the special floors.
        for what, game, field in (("field 13", (1, 0, 0, 0), 13), ("battle", (1, 1, 0, 0), 5),
                                  ("dungeon", (2, 0, 0, 0), 0), ("floor 8", (2, 0, 0, 8), 0),
                                  ("town", (0, 0, 0, 0), 0)):
            sc = Scenario(100)
            self.party(sc, 1)
            sc.game = game
            sc.field = field
            self.key_items(sc, {49: 1})
            f = self.personal(sc, game[0], 2)
            sc.pads[f + 12] = (OK, 0)
            sc.pads[f + 40] = (OK, 0)
            self.compare(sc, f"grunty flute {what}")

    def test_ryu_book(self):
        # A Ryu Book read in town: the fade, a frame for its task, the
        # freeze again, the fade back.
        sc = Scenario(120)
        self.party(sc, 1)
        sc.game = (0, 0, 0, 0)
        sc.saves += [(PLCOL, 1, 1)]
        self.key_items(sc, {273: 1, 274: 2})
        f = self.personal(sc, 0, 2)
        sc.pads.update({f + 12: (0, L1), f + 20: (0, DOWN), f + 26: (OK, 0), f + 80: (CANCEL, 0)})
        self.compare(sc, "ryu book")

    # --- Discard Item -----------------------------------------------------

    def test_discard(self):
        sc = Scenario(330)
        self.battle(sc)
        sc.game = (1, 0, 0, 0)
        self.items(sc, [(10, 0, 30), (10, 18, 1), (13, 0, 1), (11, 0, 2), (12, 0, 1), (14, 3, 1), (6, 5, 1),
                        (0, 2, 1)])
        f = self.personal(sc, 1, 3)
        # Health Drink x30: the count up, down, by tens; cancel; again and
        # discard 12; Cancel on the dialog; then triangle on equipment.
        pads = [(OK, 0), (0, UP), (0, UP), (0, LEFT), (0, LEFT), (0, LEFT), (0, LEFT), (0, RIGHT), (0, DOWN),
                (0, RIGHT), (0, RIGHT), (0, RIGHT), (0, LEFT), (CANCEL, 0), (OK, 0), (0, LEFT), (0, UP), (OK, 0)]
        g = f + 12
        for p in pads:
            sc.pads[g] = p
            g += 4
        g += 30
        sc.pads.update({g: (0, UP), g + 5: (OK, 0), g + 30: (0, DOWN), g + 36: (OK, 0),
                        g + 70: (0, DOWN), g + 74: (OK, 0), g + 80: (OK, 0), g + 110: (0, DOWN), g + 116: (OK, 0),
                        g + 150: (0, R1), g + 158: (0, R1), g + 166: (0, R1), g + 174: (0, R1), g + 182: (TRIANGLE, 0)})
        self.compare(sc, "discard")

    # --- Status, a member's items, an equipment piece ---------------------

    def member(self, sc, cid, level=5, exp=320, gold=1234, job=0, equipment=(1, 2, 3, 4, 5, -1),
               real=None, skills=(), elm=None, temp=None, hpsp=(0, 0)):
        """spcParam[cid] and skillList[cid] in the save."""
        at = SPC_PARAM + 0xDC * cid
        sc.saves += [(at + 0x0C, 2, cid), (at + 0x0E, 2, level), (at + 0x10, 2, exp), (at + 0x14, 4, gold),
                     (at + 0xD8, 2, job), (at + 0x24, 2, hpsp[0]), (at + 0x26, 2, hpsp[1])]
        for off, vals in ((0x28, elm), (0x88, temp)):
            for k, v in enumerate(vals or ()):
                sc.saves.append((at + off + 2 * k, 2, v & 0xFFFF))
        for k, v in enumerate(equipment):
            sc.saves.append((at + 0xC8 + 2 * k, 2, v & 0xFFFF))
        real = real or [(37 * (k + 1) + 11 * cid) % 400 - 60 for k in range(16)]
        for k, v in enumerate(real):
            sc.saves.append((at + 0x48 + 2 * k, 2, v & 0xFFFF))
        for k in range(20):
            v = skills[k] if k < len(skills) else -1
            sc.saves.append((0x1EC4 + 40 * cid + 2 * k, 2, v & 0xFFFF))

    def status_party(self, sc):
        sc.party[0] = base.Char(0x100, 1, 0, 120, 60, 120, 60, b"Kite", cond=[0, 0, 5, 0, 7])
        sc.party[1] = base.Char(0x101, 2, 2, 30, 50, 150, 80, b"Orca", cond=[0, 0, 0, 3, 0, 1, 2])
        sc.party[2] = base.Char(0x102, 4, 3, 90, 9, 1500, 100, b"BlackRose")
        self.member(sc, 0, job=0, equipment=(3, 5, 7, 9, 11, -1), skills=(6, 7, 9, 170, 175, 2))
        self.member(sc, 2, level=12, exp=7, gold=0, job=3, equipment=(0, -1, 2, 60, 4, -1),
                    skills=(20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31))
        self.member(sc, 3, level=9, exp=99, gold=987654, job=5, equipment=(10, 12, -1, 1, 2, -1),
                    skills=(40, 41))
        # The records after their name pointers (the harness's own).
        for cid in (0, 2, 3):
            sc.watch.append((SPC_PARAM + 0xDC * cid + 4, 0xD8))

    def test_status(self):
        sc = Scenario(560)
        self.status_party(sc)
        sc.game = (1, 0, 0, 0)
        self.items(sc, [(10, 0, 3), (6, 5, 1), (0, 2, 1), (8, 3, 1), (13, 1, 2)])
        f = self.personal(sc, 1, 4)
        # The grid: down the equipment, right into the skills, round the
        # effects and the parameters.
        moves = [DOWN, DOWN, DOWN, DOWN, DOWN, RIGHT, RIGHT, RIGHT, LEFT, UP, UP, UP, RIGHT, DOWN, DOWN, DOWN,
                 DOWN, DOWN, DOWN, LEFT, UP, RIGHT, DOWN, DOWN, DOWN, DOWN, RIGHT, RIGHT, DOWN, DOWN, DOWN, DOWN,
                 LEFT, LEFT, LEFT, LEFT, UP, UP, RIGHT, UP, UP, UP, UP, UP, UP, DOWN, LEFT, UP, UP, UP, UP]
        g = f + 12
        for p in moves:
            sc.pads[g] = (0, p)
            g += 3
        # Through the members; triangle on the weapon; cancel; square.
        sc.pads.update({g + 5: (0, R1), g + 10: (0, R1), g + 15: (0, R1), g + 20: (0, L1), g + 25: (TRIANGLE, 0)})
        g += 60
        for k, p in enumerate((RIGHT, DOWN, DOWN, DOWN, DOWN, RIGHT, UP, LEFT, DOWN, DOWN, DOWN, DOWN, DOWN, RIGHT,
                               RIGHT, UP, UP, UP, UP, UP)):
            sc.pads[g + 3 * k] = (0, p)
        g += 70
        sc.pads.update({g: (CANCEL, 0), g + 30: (0, UP), g + 34: (0, UP), g + 38: (TRIANGLE, 0), g + 70: (CANCEL, 0),
                        g + 100: (SQUARE, 0), g + 130: (0, R1), g + 138: (0, R1), g + 146: (0, R1), g + 154: (0, R1),
                        g + 162: (TRIANGLE, 0), g + 200: (CANCEL, 0), g + 230: (CANCEL, 0), g + 260: (CANCEL, 0)})
        self.compare(sc, "status")

    # --- Equipment ----------------------------------------------------------

    def kite_equipment(self, sc, items, equipment=(5, 8, 10, 3, 3, -1), job=0):
        sc.party[0] = base.Char(0x100, 1, 0, 120, 60, 120, 60, b"Kite", cond=[0, 0, 0, 1, 0, 0, 3])
        sc.party[1] = base.Char(0x101, 2, 2, 30, 50, 150, 80, b"Orca")
        self.member(sc, 0, job=job, equipment=equipment, skills=(6, 9, 150, 153, 161, 170, 180, 217, 250, 278),
                    elm=[40, 35, 60, 20, 30, 25, 50, 15, 5, 10, 0, -5, 20, 30, 900, 950],
                    hpsp=(120, 60))
        self.member(sc, 2, job=1)
        self.items(sc, items)
        at = SPC_PARAM
        sc.watch += [(at + 4, 0xD8), (0x1EC4, 40)]

    def test_equipment(self):
        weapons = [(0, k, 1) for k in (0, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 22)]
        armour = [(6, 36, 1), (6, 42, 1), (6, 20, 3), (7, 37, 1), (8, 17, 1), (9, 37, 1), (9, 42, 1), (10, 0, 5)]
        sc = Scenario(700)
        self.kite_equipment(sc, weapons + armour)
        sc.game = (1, 0, 0, 0)
        f = self.personal(sc, 1, 5)
        g = f + 14
        # The worn piece, down the candidates past the eighth row, the grid
        # (skills, effects, parameters) and back up into the list.
        moves = [DOWN, DOWN, DOWN, DOWN, DOWN, DOWN, DOWN, DOWN, DOWN, DOWN, UP, UP, RIGHT, DOWN, DOWN, RIGHT, DOWN,
                 DOWN, DOWN, DOWN, DOWN, DOWN, DOWN, LEFT, UP, RIGHT, DOWN, DOWN, DOWN, DOWN, RIGHT, RIGHT, RIGHT,
                 DOWN, DOWN, DOWN, DOWN, LEFT, LEFT, LEFT, LEFT, UP, UP, UP, UP, UP, UP, UP, DOWN, DOWN, DOWN, DOWN,
                 DOWN, DOWN, DOWN, UP, UP, UP, UP, UP, UP, UP, UP, UP, UP, UP, UP, UP, UP, UP, UP, RIGHT, LEFT]
        for p in moves:
            sc.pads[g] = (0, p)
            g += 3
        # The pages round and back to the legs; the worn piece's status (64)
        # and a candidate's; the candidate on.
        for p in (R1, R1, R1, R1, R1, L1, L1, R1):
            sc.pads[g] = (p, 0)
            g += 6
        seq = [(TRIANGLE, 0), (CANCEL, 0), (0, DOWN), (TRIANGLE, 0), (CANCEL, 0), (OK, 0),
               # The hands: the candidate on.
               (L1, 0), (0, DOWN), (OK, 0),
               # The weapon: the fourth candidate on, then the second; the
               # grid after.
               (L1, 0), (L1, 0), (L1, 0), (0, DOWN), (0, DOWN), (0, DOWN), (0, DOWN), (OK, 0), (0, DOWN), (0, DOWN),
               (OK, 0), (0, RIGHT), (0, DOWN), (0, DOWN), (0, RIGHT), (0, DOWN), (0, LEFT), (0, LEFT),
               (CANCEL, 0), (CANCEL, 0)]
        for p in seq:
            sc.pads[g] = p
            g += 30 if p[0] in (TRIANGLE, CANCEL, OK) else 6
        sc.frames = g + 30
        self.compare(sc, "equipment")

    def test_equipment_refused(self):
        # 99 of the worn weapon carried: warning 1. A full list, none of
        # the worn one, two of the new: warning 0. Then a set's skill.
        for what, items, eq, n in (
                ("99", [(0, 3, 99), (0, 8, 1), (0, 12, 1)], (5, 8, 10, 3, 3, -1), 1),
                ("full", [(0, 8, 2)] + [(10, k, 1) for k in range(20)] + [(11, k, 1) for k in range(19)],
                 (5, 8, 10, 3, 3, -1), 1),
                ("set", [(6, 60, 1), (0, 8, 1)], (61, 60, 60, 60, 3, -1), 0)):
            sc = Scenario(220)
            self.kite_equipment(sc, items, equipment=eq)
            sc.game = (0, 0, 0, 0)
            f = self.personal(sc, 0, 5)
            g = f + 14
            if n == 0:
                sc.pads.update({g: (R1, 0)})
                g += 6
            sc.pads.update({g: (0, DOWN), g + 6: (OK, 0), g + 60: (OK, 0), g + 100: (0, RIGHT), g + 104: (0, DOWN),
                            g + 108: (0, DOWN), g + 112: (0, DOWN), g + 130: (CANCEL, 0)})
            self.compare(sc, f"equipment {what}")

    # --- PARTY: Add, Remove, Disband ---------------------------------------

    MEMBERS = {2: b"Orca", 3: b"Marlo", 15: b"BlackRose"}

    def town_party(self, sc, members=(), joiners=(2, 3, 15), call=(2, 15), talk=()):
        """Kite in Mac Anu with `members` in the party; `joiners` able to
        join (partyMemberFlag), `call` answering (partyMemberCall)."""
        sc.party[0] = base.Char(0x100, 1, 0, 120, 60, 120, 60, b"Kite")
        for slot, cid in enumerate(members, 1):
            sc.party[slot] = base.Char(0x100 + cid, 2, cid, 90, 40, 150, 80, self.MEMBERS[cid])
        for cid in joiners:
            sc.joiners[cid] = base.Char(0x100 + cid, 2, cid, 150, 80, 150, 80, self.MEMBERS[cid])
        for cid, name in self.MEMBERS.items():
            sc.spc[cid] = (name, 10 + cid, 100 * cid, 1000 + cid, 150, 80, cid % 6)
        flag = sum(1 << c for c in set(joiners) | set(members))
        sc.saves += [(0x2220, 4, flag), (0x2224, 4, sum(1 << c for c in call))]
        for cid in talk:
            sc.saves.append((0x220C + cid, 1, 1))
        sc.game = (0, 0, 0, 0)

    def party_pads(self, sc, f, pads):
        """(gap, pad) pairs from frame f."""
        for gap, p in pads:
            f += gap
            sc.pads[f] = p
        return f

    def test_party_refused(self):
        # PARTY outside a town; Add, Remove and Disband alone (Add with none
        # to call).
        sc = Scenario(80)
        self.party(sc, 1)
        sc.game = (1, 0, 0, 0)
        sc.opens[10] = 9
        sc.pads[40] = (OK, 0)
        self.compare(sc, "party in a field")
        sc = Scenario(260)
        self.town_party(sc, joiners=())
        f = self.personal(sc, 0, 6)
        self.party_pads(sc, f, [(16, (0, DOWN)), (4, (0, DOWN)), (4, (0, DOWN)), (4, (0, UP)), (4, (0, UP)),
                                (4, (0, UP)), (6, (OK, 0)), (30, (OK, 0)), (30, (0, DOWN)), (6, (OK, 0)),
                                (30, (OK, 0)), (30, (0, DOWN)), (6, (OK, 0)), (30, (OK, 0)), (30, (CANCEL, 0))])
        self.compare(sc, "party alone")

    def test_party_add(self):
        sc = Scenario(560)
        self.town_party(sc, talk=(15,))
        f = self.personal(sc, 0, 6)
        self.party_pads(sc, f, [
            (16, (OK, 0)),                          # Add
            (24, (0, DOWN)), (6, (OK, 0)),          # Marlo: no answer
            (24, (OK, 0)),                          # OK
            (70, (OK, 0)), (12, (OK, 0)),           # the refusal read
            (24, (0, UP)), (6, (OK, 0)),            # Orca
            (24, (0, DOWN)), (6, (OK, 0)),          # Cancel
            (24, (0, DOWN)), (6, (0, DOWN)), (6, (OK, 0)),  # BlackRose
            (24, (OK, 0)),                          # OK: the call
            (90, (OK, 0)), (12, (OK, 0)), (12, (OK, 0))])
        self.compare(sc, "party add")

    def test_party_add_loaded(self):
        # Orca's character loaded and idle: the other greeting; five loaded
        # and BlackRose not among them: no answer.
        for what, reg, row in (("idle", [(2, 0)], 0), ("busy", [(2, 1)], 0),
                                ("full", [(4, 0), (5, 0), (6, 0), (7, 0)], 1)):
            sc = Scenario(260)
            self.town_party(sc, talk=(2,))
            sc.registry = reg
            f = self.personal(sc, 0, 6)
            pads = [(16, (OK, 0))] + [(24, (0, DOWN))] * row + [(24, (OK, 0)), (24, (OK, 0)), (100, (OK, 0)),
                                                                 (12, (OK, 0)), (12, (OK, 0))]
            self.party_pads(sc, f, pads)
            self.compare(sc, f"party add {what}")

    def test_party_remove(self):
        # Add refused with three; Remove: Cancel, then BlackRose (slot 2).
        sc = Scenario(360)
        self.town_party(sc, members=(2, 15), talk=(15,))
        f = self.personal(sc, 0, 6)
        self.party_pads(sc, f, [(16, (OK, 0)), (30, (OK, 0)), (30, (0, DOWN)), (6, (OK, 0)),
                                (24, (0, DOWN)), (6, (OK, 0)), (24, (0, DOWN)), (6, (OK, 0)),
                                (24, (OK, 0)), (24, (OK, 0)), (30, (OK, 0)), (12, (OK, 0)), (12, (OK, 0))])
        self.compare(sc, "party remove")

    def test_party_disband(self):
        sc = Scenario(360)
        self.town_party(sc, members=(2, 15), talk=(2,))
        f = self.personal(sc, 0, 6)
        self.party_pads(sc, f, [(16, (0, DOWN)), (4, (0, DOWN)), (6, (OK, 0)), (24, (0, DOWN)), (6, (OK, 0)),
                                (30, (OK, 0)), (24, (OK, 0)), (30, (OK, 0)), (12, (OK, 0)), (30, (OK, 0)),
                                (12, (OK, 0)), (12, (OK, 0))])
        self.compare(sc, "party disband")

    # --- random pads through PERSONAL -----------------------------------------

    def test_random_personal(self):
        # PERSONAL in a town (the party's pages) or a field (Area
        # Information), reopened now and then; random pads through every
        # page: Key Items, Discard, Status and its pages, Equipment, Gate
        # Out, Log Out.
        weapons = [(0, k, 1) for k in (0, 1, 2, 8, 12, 22)]
        armour = [(6, 36, 1), (6, 20, 3), (7, 37, 1), (8, 17, 1), (9, 37, 1), (10, 0, 5), (10, 18, 1), (13, 0, 1)]
        for seed in range(1, 17):
            rnd = random.Random(seed)
            sc = Scenario(600)
            town = seed % 2 == 0
            self.kite_equipment(sc, weapons + armour)
            if town:
                self.town_party(sc, members=(2,) if seed % 4 == 0 else (), talk=(2,))
                sc.party[0] = base.Char(0x100, 1, 0, 120, 60, 120, 60, b"Kite")
            else:
                sc.game = (1, 0, 0, 0)
                sc.area_words = (3, 105, 208)
            sc.town = 2
            self.key_items(sc, {0: 1, 44: 2, 48: 1, 49: 1, 50: 1, 273: 1, 281: 1})
            for f in range(10, 600, 90):
                sc.opens[f] = 0 if town else 1
            start = 12
            if seed > 12:
                # Straight into Status or Equipment first.
                start = self.personal(sc, 0 if town else 1, 4 + seed % 2) + 14
            for f in range(start, 600):
                r = rnd.random()
                if r < 0.16:
                    sc.pads[f] = (0, rnd.choice((UP, DOWN, DOWN, LEFT, RIGHT)))
                elif r < 0.19:
                    sc.pads[f] = (rnd.choice((L1, R1)), rnd.choice((0, L1, R1)))
                elif r < 0.28:
                    sc.pads[f] = (OK, 0)
                elif r < 0.30:
                    sc.pads[f] = (CANCEL, 0)
                elif r < 0.31:
                    sc.pads[f] = (rnd.choice((TRIANGLE, SQUARE)), 0)
            self.compare(sc, f"random {seed}")

    # --- Gate Out, Log Out, 86 --------------------------------------------

    def test_gate_out(self):
        # In battle: refused.
        sc = Scenario(90)
        self.party(sc, 2)
        sc.game = (1, 1, 0, 0)
        f = self.personal(sc, 1, 7)
        sc.pads[f + 30] = (OK, 0)
        self.compare(sc, "gate out in battle")
        # Cancel on the dialog, then the cancel button, then OK: the fade and
        # back to town 3.
        sc = Scenario(200)
        self.party(sc, 2)
        sc.game = (1, 0, 0, 0)
        sc.town = 3
        f = self.personal(sc, 1, 7)
        sc.pads.update({f + 12: (OK, 0), f + 40: (OK, 0), f + 55: (CANCEL, 0), f + 80: (OK, 0),
                        f + 95: (0, UP), f + 100: (OK, 0)})
        self.compare(sc, "gate out")

    def test_log_out(self):
        sc = Scenario(160)
        self.party(sc, 1)
        sc.game = (0, 0, 0, 0)
        f = self.personal(sc, 0, 7)
        sc.pads.update({f + 12: (CANCEL, 0), f + 40: (OK, 0), f + 55: (0, DOWN), f + 60: (OK, 0)})
        self.compare(sc, "log out")

    def test_area_info(self):
        # Area Information in a field and a dungeon: the area's keywords on
        # the gate's page; cancel back.
        for what, area, words in (("field", 1, (3, 105, 208)), ("dungeon", 2, (10, 120, 230))):
            sc = Scenario(90)
            self.party(sc, 2)
            sc.game = (area, 0, 0, 0)
            sc.area_words = words
            f = self.personal(sc, area, 6)
            sc.pads[f + 40] = (CANCEL, 0)
            self.compare(sc, f"area info {what}")

    def test_trans_field(self):
        for what, wiped in (("go", 0), ("wiped", 1)):
            sc = Scenario(160)
            self.party(sc, 2)
            sc.game = (2, 0, 0, 0)
            sc.wiped = wiped
            sc.opens[10] = 86
            self.compare(sc, f"trans field {what}")


if __name__ == "__main__":
    unittest.main()
