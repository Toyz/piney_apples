#!/usr/bin/env python3
"""piney-game's announcements on the desktop and the board (desktop.rs's
Bridge::announce and story_area, story.rs's Announcements) against the
game's own ccEvent::Execute (INF SLUS_202.67:0x001a8d20) run in eemu (the
Rust eemu_rs when it is built into tools/, as test_anim.machine_class picks).

Each case is a one-instruction script run by Execute at level 2, with
ccGame.status 2 (the desktop) or 3 (the board) and eventMng.enablePhase 5
(play):

- gate_add_msg area (case 90) for every eventAreaInfo code;
- member_add_msg pc (case 80) for members 1-17;
- desktop_item type id (case 167) for types 0-3 and a few ids.

Native: Execute, ccEvent::DispInfo (0x001b27b0), WORLD_MAN::GetEventAreaInfo
and GetWordParamFromEvCode, ccSaveData::SetGateList, ccKanjiStrSeparate,
dec2sjis, strcpy and strcat, ccDtMenu::CheckMenuType. Hooked: ccSeOn
(recorded), ccBreathThread (the frame count), ccMessage::ChangeInfo (its
four line pointers read), ccMessage::Check (0 until the case's poll, then
1) and ccMessage::Close. The save is the game's own new game (the boot's
ccSaveData, NewGame(1), NewGame(0) with DEMO.PRG in, which also fills
spcNameList, where the members' name pointers point); desktop_item runs
with GCMN.PRG in, where bookItemAddMsg and the rest point. dtMenu's menu is
-1 or 7 (the fade menu), which DispInfo waits for.

Recorded per case, numbers only: the sounds; each ChangeInfo line's length
and FNV-1a hash (-1 for a null line); the frames of ChangeInfo, of each
Check, of Close and of Execute's return, counted from the first sound;
dtMenu +0x06 and +0x10 after; and every save byte that changed (offset,
value). The results go to crates/piney-game/tests/announce_fixture.txt;
desktop.rs's test replays each case through the port's event task, its
Bridge and Announcements.

    python3 tools/test_desktop_announce_rs.py            check the fixture is the game's
    python3 tools/test_desktop_announce_rs.py fixture    write it
"""

import functools
import os
import random
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va  # noqa: E402
inf_va = functools.partial(va, overlay='desktop')  # this overlay's globals

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
FIXTURE = os.path.join(ROOT, "crates", "piney-game", "tests", "announce_fixture.txt")

SIZE = 0x8530
# gp globals
CCSYS, GAME_P, DTMENU_P, SAVEDATA = inf_va(0x003788E0), inf_va(0x003789CC), inf_va(0x003789D0), inf_va(0x003789D8)
WORLDMAN_P, CCMSG_P, EVENTMNG_P = inf_va(0x00378A7C), inf_va(0x00378A8C), inf_va(0x00378A94)
# Scratch memory. The save sits where the port's NewGame says it does
# (piney_demo::newgame::SAVE_VA), for character 0's name pointer.
SAVE = 0x01810000
EV, GAMEOBJ, SYSOBJ, DTM, MSG, WM = 0x01840000, 0x01841000, 0x01842000, 0x01843000, 0x01844000, 0x01845000
SCRIPT, PTR = 0x01846000, 0x01846800
EVENT_AREA_INFO, EVENT_AREAS, RECORD = inf_va(0x00315120), 126, 0x54

OPS = {"gate": 90, "member": 80, "item": 167}


def fnv(b):
    h = 0x811C9DC5
    for c in b:
        h = ((h ^ c) * 0x01000193) & 0xFFFFFFFF
    return h


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


class Game:
    """Execute in eemu over `overlay` (demo or gcmn), on `save`."""

    def __init__(self, overlay, save=None):
        from image import Program
        from test_anim import machine_class
        import test_save_init_rs
        self.p = Program(ELF, overlay)
        self.m = m = machine_class()(self.p)
        sym = self.sym
        m.mem[GAMEOBJ:GAMEOBJ + 0x100] = bytes(0x100)
        m.mem[SYSOBJ:SYSOBJ + 0x400] = bytes(0x400)
        for g, v in ((CCSYS, SYSOBJ), (GAME_P, GAMEOBJ), (DTMENU_P, DTM), (SAVEDATA, SAVE), (WORLDMAN_P, WM),
                     (CCMSG_P, MSG), (EVENTMNG_P, EV)):
            m.store(g, 4, v)
        for name in ("SetDisplayOffset__8ccSystemFii", "SetSoundEnv__10ccSaveDataFv", "setCameraCtrlType__Fi"):
            m.hooks[sym(name)] = lambda mm, *a: 0
        if save is None:
            # The game's new game: the boot's save, NewGame(1), NewGame(0).
            m.mem[SAVE:SAVE + SIZE] = test_save_init_rs.boot_save(ELF)
            m.call(sym("NewGame__10ccSaveDataFi"), [SAVE, 1])
            m.call(sym("NewGame__10ccSaveDataFi"), [SAVE, 0])
            save = bytes(m.mem[SAVE:SAVE + SIZE])
        self.save = save

        def breath(mm, n, *a):
            self.frame += n
            return 0

        def se(mm, n, *a):
            self.ev("se", n)
            return 0

        def change_info(mm, this, l0, l1, l2):
            self.lines = [self.cstr(p) for p in (l0, l1, l2, mm.r[8] & 0xFFFFFFFF)]
            self.ev("change")
            return 0

        def check(mm, this, *a):
            self.polls += 1
            self.ev("check")
            return 1 if self.polls >= self.poll else 0

        def close(mm, *a):
            self.ev("close")
            return 0

        m.hooks[sym("ccBreathThread__Fi")] = breath
        m.hooks[sym("ccSeOn__Fi")] = se
        m.hooks[sym("ChangeInfo__9ccMessageFPcPcPcPcii")] = change_info
        m.hooks[sym("Check__9ccMessageFi")] = check
        m.hooks[sym("Close__9ccMessageFv")] = close

    def sym(self, name):
        return self.p.symbol_named(name).value

    def cstr(self, p):
        if p == 0:
            return None
        out = bytearray()
        while True:
            c = self.m.load(p + len(out), 1)
            if c == 0:
                return bytes(out)
            out.append(c)

    def ev(self, what, n=None):
        if self.first is None:
            self.first = self.frame
        self.log.append((what, self.frame - self.first, n))

    def run(self, kind, status, menu, poll, a, b):
        """One instruction; the record the fixture keeps."""
        m = self.m
        m.mem[SAVE:SAVE + SIZE] = self.save
        m.mem[EV:EV + 0x800] = bytes(0x800)
        m.store(EV + 0x0C, 4, 5)
        m.store(GAMEOBJ, 4, status)
        m.mem[DTM:DTM + 0x40] = bytes(0x40)
        m.store(DTM + 0x00, 2, menu & 0xFFFF)
        m.store(DTM + 0x02, 2, menu & 0xFFFF)
        m.store(DTM + 0x06, 2, 0xFFFF)
        args = [a] if kind != "item" else [a, b]
        words = [OPS[kind]] + args + [0]
        for i, w in enumerate(words):
            m.store(SCRIPT + 2 * i, 2, w & 0xFFFF)
        m.store(PTR, 4, SCRIPT)
        self.frame, self.first, self.log, self.polls, self.poll, self.lines = 0, None, [], 0, poll, [None] * 4
        m.call(self.sym("Execute__7ccEventFRPsiii"), [EV, PTR, 1, 0, 2])
        end = self.frame - (self.first or 0)
        after = bytes(m.mem[SAVE:SAVE + SIZE])
        diff = [(i, after[i]) for i in range(SIZE) if after[i] != self.save[i]]
        return {
            "se": [n for w, _, n in self.log if w == "se"],
            "lines": [(-1, 0) if s is None else (len(s), fnv(s)) for s in self.lines],
            "change": [f for w, f, _ in self.log if w == "change"],
            "checks": [f for w, f, _ in self.log if w == "check"],
            "close": [f for w, f, _ in self.log if w == "close"],
            "end": end,
            "req": s16(m.load(DTM + 0x06, 2)),
            "ext": s16(m.load(DTM + 0x10, 2)),
            "save": diff,
        }


def area_codes():
    from image import Program
    p = Program(ELF, None)
    codes = []
    for i in range(EVENT_AREAS):
        c = p.read(EVENT_AREA_INFO + RECORD * i, 4)
        c = int.from_bytes(c, "little", signed=True)
        if c not in codes:
            codes.append(c)
    return codes


def cases():
    rng = random.Random(90)
    out = []
    for k, area in enumerate(area_codes()):
        out.append(("gate", 2 + k % 2, [-1, 7][rng.randrange(2)], rng.choice([1, 1, 3]), area, 0))
    for pc in range(1, 18):
        out.append(("member", 2 + pc % 2, -1, 1, pc, 0))
    for ty in range(4):
        for i, id_ in enumerate((1, 7, 50, 123)):
            out.append(("item", 2 + i % 2, -1, 2, ty, id_))
    return out


def fixture_lines():
    demo = Game("demo")
    gcmn = Game("gcmn", demo.save)
    out = [
        "# tools/test_desktop_announce_rs.py: ccEvent::Execute's announcements run in eemu (numbers only)",
        "# announce KIND STATUS MENU POLL A B | se N... | lines LEN HASH x4 | frames CHANGE CHECK... CLOSE END"
        " | dtmenu REQ EXT | save OFFSET VALUE ...",
    ]
    for kind, status, menu, poll, a, b in cases():
        g = gcmn if kind == "item" else demo
        r = g.run(kind, status, menu, poll, a, b)
        assert len(r["change"]) == 1 and len(r["close"]) == 1, (kind, a, r)
        lines = " ".join(f"{n} {h}" for n, h in r["lines"])
        frames = " ".join(str(f) for f in r["change"] + r["checks"] + r["close"] + [r["end"]])
        save = " ".join(f"{o} {v}" for o, v in r["save"])
        out.append(
            f"announce {kind} {status} {menu} {poll} {a} {b} | se {' '.join(map(str, r['se']))} | lines {lines}"
            f" | frames {frames} | dtmenu {r['req']} {r['ext']} | save {save}"
        )
    return out


@unittest.skipUnless(os.path.exists(ELF), "needs the extracted disc")
class AnnounceFixture(unittest.TestCase):
    def test_fixture_is_the_games(self):
        with open(FIXTURE) as f:
            have = f.read().splitlines()
        self.assertEqual(have, fixture_lines())


if __name__ == "__main__":
    if sys.argv[1:] == ["fixture"]:
        lines = fixture_lines()
        with open(FIXTURE, "w") as f:
            f.write("\n".join(lines) + "\n")
        print(f"{len(lines) - 2} cases -> {FIXTURE}")
    else:
        unittest.main()
