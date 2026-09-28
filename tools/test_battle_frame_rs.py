#!/usr/bin/env python3
"""crates/piney-battle's frame.rs against the game's own ccThSpc run in the
EE interpreter (the Rust machine of tools/test_anim.py when it is built).

  spc_task      ccThSpc (gcmn 0x005a0530) from its start over 20-200
                frames: ccSpcSetOperation (0x005a1930),
                ccSpcShoutOperationName (0x005a17e0) and the battle
                condition, with ccGame.area, ccGame.inBattle, the event's
                puppetShow, the save's operation and the party's size
                changing at random between frames. Compared after every
                frame: spcBattleCondition, spcOpnCnt, partyStrategy,
                spcCheckInBattleOld, and the chat Kite opens (its text
                matched against ccKanjiStrSeparate's pieces).

ccSpcCheckLevelUp runs natively with no member away from the party (it is
checked by tools/test_battle_flow_rs.py); ccStartThread and
SetPathFindingMap are stubbed.

    python3 tools/test_battle_frame_rs.py            the unit tests
    python3 tools/test_battle_frame_rs.py bulk N     N cases of every check
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import eemu  # noqa: E402
import test_anim  # noqa: E402

eemu.Machine = test_anim.machine_class()

import test_battle_rs as rs  # noqa: E402
from test_battle import GAME, SAVE  # noqa: E402

EVT = 0x01033000        # the ccEvent eventMng points at (puppetShow at +0x78c)
TSCB = 0x01033800
CHAT = 0x01033C00
OPERATION = 0x6773


class Frame:
    """ccThSpc in the interpreter, a Breath a frame."""

    def __init__(self, checks):
        self.c = checks
        g = checks.game
        self.g, self.m = g, g.m
        m, sym = self.m, g.sym
        self.sym = sym
        m.store(sym("eventMng"), 4, EVT)
        m.store(sym("ccChat"), 4, CHAT)
        self.party = sym("ccPartyManager")
        self.g_opn, self.g_old = sym("spcOpnCnt"), sym("spcCheckInBattleOld")
        self.g_cond, self.g_strat = sym("spcBattleCondition"), sym("partyStrategy")
        # the texts a shout is made of, by operation
        sep = sym("ccKanjiStrSeparate__FPci")
        act, menu = m.load(sym("chatActionStr"), 4), m.load(sym("chatMenuStr"), 4)

        def piece(s, n):
            return self.cstr(m.call(sep, [s, n]))

        head, tail = piece(act, 5), piece(act, 6)
        self.shouts = {head + piece(menu, op + 1) + tail: op for op in range(-1, 12)}

    def cstr(self, a):
        mem = self.m.mem
        e = mem.index(0, a)
        return bytes(mem[a:e])

    def put_inputs(self, f):
        m = self.m
        area, in_battle, puppet, op, num = f
        m.store(GAME + 0x14, 4, area)
        m.store(GAME + 0x58, 4, in_battle)
        m.store(EVT + 0x78C, 4, puppet)
        m.store(SAVE + OPERATION, 1, op & 0xFF)
        m.store(self.party + 0x18, 4, num)

    def state(self):
        m = self.m
        return [m.load(self.g_cond, 2, True), m.load(self.g_opn, 2, True), m.load(self.g_strat, 4, True),
                m.load(self.g_old, 2, True)]

    def run(self, init, frames):
        m, sym = self.m, self.sym
        m.store(self.g_opn, 2, init[0])
        m.store(self.g_old, 2, init[1])
        m.store(self.g_cond, 2, init[2])
        m.store(self.g_strat, 4, init[3])
        m.store(SAVE + 0x2220, 4, 0)       # partyMemberFlag: nobody away from the party
        m.store(SAVE + 0x222C, 4, 0)
        for n in range(3):
            m.store(self.party + 4 * n, 4, rs.SCN + 0x1000 * n)
        self.put_inputs(frames[0])
        out, shout = [], [None]

        def breath(mm, *_):
            out.append(self.state() + [shout[0] if shout[0] is not None else -2])
            shout[0] = None
            if len(out) >= len(frames):
                raise eemu.Stop("done")
            self.put_inputs(frames[len(out)])
            return 0

        def chat(mm, this, ch, text, *_):
            s = self.cstr(text)
            shout[0] = self.shouts.get(s, 99) if ch == rs.SCN else 98
            return 0

        hooks = {"Breath__6ccTscbFi": breath, "OpenChat__9ccChatMsgFP6ccCharPc": chat,
                 "ccStartThread__FPFPv_vii": lambda mm, *_: 0, "SetPathFindingMap__Fv": lambda mm, *_: 0}
        saved = {}
        for n, h in hooks.items():
            a = sym(n)
            saved[a] = m.hooks.get(a)
            m.hooks[a] = h
        try:
            m.call(sym("ccThSpc__FPv"), [TSCB], limit=50_000_000)
        except eemu.Stop as e:
            if str(e) != "done":
                raise
        finally:
            for a, h in saved.items():
                if h is None:
                    del m.hooks[a]
                else:
                    m.hooks[a] = h
        return out


def check_spc_task(checks, rnd):
    fr = getattr(checks, "_frame", None) or Frame(checks)
    checks._frame = fr
    init = [rnd.choice((60, 60, 21, 20, 19, 1, 0, rnd.randrange(-5, 100))), rnd.choice((0, 1, 2, rnd.randrange(-2, 5))),
            rnd.randrange(-1, 7), rnd.randrange(-1, 5)]
    n = rnd.choice((20, 40, 80, 200))
    f = [rnd.choice((0, 1, 1, 2)), rnd.choice((0, 1, 2)), 0, rnd.choice((7, 8, 9, 10, 0, -1, 11)), rnd.choice((1, 2, 3))]
    frames = []
    for _ in range(n):
        if rnd.random() < 0.12:
            f = list(f)
            # inBattle changes most often; a fight lasts long enough for the shout
            k = rnd.choice((1, 1, 1, 0, 2, 3, 4))
            f[k] = [rnd.choice((0, 1, 1, 2)), rnd.choice((0, 1, 1, 2, 2, 3, -1)), rnd.choice((0, 0, 1)),
                    rnd.choice((7, 8, 9, 10, 0, -1, 11)), rnd.choice((1, 2, 2, 3, 3, 0))][k]
        frames.append(f)
    out = fr.run(init, frames)
    req = "spcthread " + " ".join(map(str, init)) + f" {n} " + " ".join(" ".join(map(str, x)) for x in frames)
    return req, {"frames": out}, "spc_task"


FRAME = {"spc_task": check_spc_task}


@unittest.skipUnless(rs.READY, "needs the extracted disc and cargo")
class FrameAgainstGame(rs.Against):
    TABLE = FRAME

    def test_spc_task(self):
        self.check("spc_task", 30000)


if __name__ == "__main__":
    rs.main(list(FRAME), 30000, FRAME)
