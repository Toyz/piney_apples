#!/usr/bin/env python3
"""Party membership in crates/piney-world's party.rs against the game's own
functions run in eemu (the Rust eemu_rs when it is built into tools/, as
test_anim.machine_class picks).

  - ccParty::AddMember (gcmn 0x0059ce80) and inviteSpc (0x005a08e0) for a
    registered character, or for an unregistered one when the registry is
    full (inviteSpc's build at the Chaos Gate is not ported);
  - ccParty::DelMember (0x0059cf60) and disbandSpc (0x005a0f50);
  - expulsionSpc (0x005a1070) with ccParty::CheckMemberID.

Random states: ccSpcManager's five registry slots (the id, partyFlag, and
charPtr to a character or null), ccParty's memberChar, memberID and num,
and each character's +0xe0 word (partyFlag bits 14-16, recallFlag bit 12,
the other bits random), then one to five calls. Native, nothing hooked.
Compared after the calls: each call's return (AddMember's slot), the
registry's ids and partyFlags, memberChar (as the registry slot of the
character it points at), memberID, num, and each character's partyFlag and
recallFlag. The world_probe `party` request runs the same calls on
party.rs's `Spcs`.

Skipped when the disc is not extracted or cargo is missing.
"""

import os
import random
import shutil
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc
import test_world_rs as tw  # noqa: E402

PARTY_MANAGER = inf_va(0x00730310)   # ccParty: memberChar[3], memberID[3] (+0x0c), num (+0x18)
SPC_MANAGER = inf_va(0x00730340)     # ccSpcManager: 5 x 0x2c {id, partyFlag, bootParam, ..., +0x1c charPtr}, +0xdc num
CHAR, CHAR_SIZE = 0x01850000, 0x200


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v & 0x80000000 else v


def pf_of(word):
    v = (word >> 14) & 7
    return v - 8 if v & 4 else v


def state(r):
    """A consistent registry and party: Kite (0) in registry slot 0 and
    party slot 0, members registered, memberChar the member's character
    when it is built."""
    reg = [-1] * 5
    reg[0] = 0
    ids = r.sample(range(1, 24), 4)
    for k in range(1, 5):
        if r.random() < 0.8:
            reg[k] = ids[k - 1]
    pfs = [1] + [r.choice((0, 1)) for _ in range(4)]
    built = [r.random() < 0.85 if reg[k] != -1 else False for k in range(5)]
    words = [r.getrandbits(32) for _ in range(5)]
    for k in range(5):
        pf, recall = r.choice((-2, -1, 0, 1, 2)), r.random() < 0.5
        words[k] = (words[k] & ~((7 << 14) | (1 << 12))) | ((pf & 7) << 14) | (int(recall) << 12)
    mid = [0, -1, -1]
    registered = [reg[k] for k in range(1, 5) if reg[k] != -1]
    for s in (1, 2):
        if registered and r.random() < 0.6:
            mid[s] = r.choice(registered)
    mc = [-1] * 3
    for s in range(3):
        if mid[s] != -1:
            k = reg.index(mid[s])
            if built[k]:
                mc[s] = k
    num = sum(1 for m in mid if m != -1)
    full = all(x != -1 for x in reg)
    ops = []
    for _ in range(r.randint(1, 5)):
        op = r.choice((1, 1, 2, 2, 3))
        if op == 1:
            if full and r.random() < 0.3:
                arg = r.choice([i for i in range(24) if i not in reg])
            else:
                arg = r.choice([x for x in reg if x != -1])
        elif op == 2:
            arg = r.choice((-1, 0, 1, 2, 1, 2))
        else:
            arg = r.choice([k for k in range(5) if reg[k] != -1])
        ops.append((op, arg))
    return dict(reg=reg, pfs=pfs, built=built, words=words, mid=mid, mc=mc, num=num, ops=ops)


def probe_line(s):
    v = []
    for k in range(5):
        v += [s["reg"][k], s["pfs"][k] if s["reg"][k] != -1 else 0]
    v += s["mc"] + s["mid"] + [s["num"]]
    for k in range(5):
        w = s["words"][k]
        v += [int(s["built"][k]), pf_of(w), (w >> 12) & 1]
    v.append(len(s["ops"]))
    for op, arg in s["ops"]:
        v += [op, arg]
    return "party " + tw.hexs(*v)


class Game:
    def __init__(self):
        from image import Program
        from test_anim import machine_class
        self.p = Program(tw.ELF, "gcmn")
        self.m = machine_class()(self.p)
        self.sym = lambda n: self.p.symbol_named(n).value

    def run(self, s):
        m = self.m
        m.mem[SPC_MANAGER:SPC_MANAGER + 0xE0] = bytes(0xE0)
        m.mem[PARTY_MANAGER:PARTY_MANAGER + 0x1C] = bytes(0x1C)
        for k in range(5):
            b = SPC_MANAGER + 0x2C * k
            m.store(b, 4, s["reg"][k] & 0xFFFFFFFF)
            m.store(b + 4, 4, s["pfs"][k] if s["reg"][k] != -1 else 0)
            c = CHAR + CHAR_SIZE * k
            m.mem[c:c + CHAR_SIZE] = bytes(CHAR_SIZE)
            m.store(c + 0xE0, 4, s["words"][k])
            m.store(b + 0x1C, 4, c if s["built"][k] else 0)
        m.store(SPC_MANAGER + 0xDC, 4, sum(1 for x in s["reg"] if x != -1))
        for sl in range(3):
            k = s["mc"][sl]
            m.store(PARTY_MANAGER + 4 * sl, 4, CHAR + CHAR_SIZE * k if k >= 0 else 0)
            m.store(PARTY_MANAGER + 0xC + 4 * sl, 4, s["mid"][sl] & 0xFFFFFFFF)
        m.store(PARTY_MANAGER + 0x18, 4, s["num"])
        ret = []
        for op, arg in s["ops"]:
            if op == 1:
                ret.append(s32(m.call(self.sym("AddMember__7ccPartyFi"), [PARTY_MANAGER, arg & 0xFFFFFFFF])))
            elif op == 2:
                m.call(self.sym("DelMember__7ccPartyFi"), [PARTY_MANAGER, arg & 0xFFFFFFFF])
                ret.append(0)
            else:
                m.call(self.sym("expulsionSpc__Fi"), [arg])
                ret.append(0)
        ld = lambda a: s32(m.load(a, 4))
        reg = [[ld(SPC_MANAGER + 0x2C * k), ld(SPC_MANAGER + 0x2C * k + 4)] for k in range(5)]
        mc = []
        for sl in range(3):
            p = m.load(PARTY_MANAGER + 4 * sl, 4)
            mc.append((p - CHAR) // CHAR_SIZE if p else -1)
        chars = []
        for k in range(5):
            if s["built"][k]:
                w = m.load(CHAR + CHAR_SIZE * k + 0xE0, 4)
                chars.append([pf_of(w), (w >> 12) & 1])
            else:
                chars.append(None)
        return {"ret": ret, "reg": reg, "mc": mc, "mid": [ld(PARTY_MANAGER + 0xC + 4 * sl) for sl in range(3)],
                "num": ld(PARTY_MANAGER + 0x18), "chars": chars}


@unittest.skipUnless(os.path.exists(tw.ELF) and os.path.exists(tw.ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class PartyAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        tw.build()
        cls.g = Game()

    def test_membership(self):
        r = random.Random(0x9A27)
        states = [state(r) for _ in range(600)]
        got = tw.ask([probe_line(s) for s in states])
        bad = 0
        for s, port in zip(states, got):
            want = self.g.run(s)
            if port != want:
                bad += 1
                if bad <= 5:
                    print("state", s, "\n game", want, "\n port", port)
        self.assertEqual(bad, 0)


if __name__ == "__main__":
    unittest.main()
