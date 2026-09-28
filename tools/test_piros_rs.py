#!/usr/bin/env python3
"""The event instruction piros_colour (ccEvent::Execute case 137, INF
SLUS_202.67:0x001b0c1c) run in eemu (the Rust eemu_rs when it is built into
tools/, as test_anim.machine_class picks), for the port's version to be
checked against.

When Piros (charTbl row 8) is in ccSpcManager's registry, the case switches
on saveData+0x64f9 (0-13): each value sets his ccChar's affectColorFix
(+0xa6), its rate (+0xaa) and affectColor (+0xac), with screen flashes
(scFadeDef's ccScFade::EntryFlash), SE 74, an information window
(ccEvent::DispInfo of the event's message saveData+0x6510, its first line
by ccKanjiStrSeparate) and breaths (ccBreathThread) between.

Run for event 22 (PIRO02) with a Piros in the registry's first slot, every
status 0-14 and messages 0-3, and once with no Piros. Native: Execute and
the case, ccKanjiStrSeparate, fptosi. Hooked, recorded in order: EntryFlash
(its count, colour and four floats), ccSeOn, DispInfo (each line's length
and FNV-1a hash, -1 for NULL), and every ccBreathThread (its count, then
Piros's +0xa6, +0xaa and +0xac after it), and the three fields at the end.

    python3 tools/test_piros_rs.py fixture OUT    write the records
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF

SAVEDATA, SCFADE_P = inf_va(0x003789D8), inf_va(0x00378968)
SPC_MANAGER = inf_va(0x00730340)
SAVE, EV, SCRIPT, PTR, CHAR, FADE = 0x01810000, 0x01840000, 0x01846000, 0x01846800, 0x01850000, 0x01851000
SIZE = 0x8530
EVENT = 22
OP = 137


def fnv(b):
    h = 0x811C9DC5
    for c in b:
        h = ((h ^ c) * 0x01000193) & 0xFFFFFFFF
    return h


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


class Piros:
    def __init__(self):
        from image import Program
        from test_anim import machine_class
        self.p = Program(ELF, "gcmn")
        self.m = m = machine_class()(self.p)
        sym = lambda n: self.p.symbol_named(n).value
        self.sym = sym
        self.log = []

        def flash(mm, this, n, col, *a):
            fl = " ".join(f"{mm.f[i] & 0xFFFFFFFF:08x}" for i in range(12, 16))
            self.log.append(f"flash {n} {col & 0xFFFFFFFF:08x} {fl}")
            return 0

        def breath(mm, n, *a):
            self.log.append(f"breath {n} {self.fields()}")
            return 0

        def se(mm, n, *a):
            self.log.append(f"se {n}")
            return 0

        def info(mm, this, a, b, c, *rest):
            lines = []
            for p in (a, b, c):
                if p == 0:
                    lines.append("-1")
                else:
                    s = self.cstr(p)
                    lines.append(f"{len(s)}:{fnv(s)}")
            self.log.append("info " + " ".join(lines))
            return 0

        m.hooks[sym("EntryFlash__8ccScFadeFiiffff")] = flash
        m.hooks[sym("ccBreathThread__Fi")] = breath
        m.hooks[sym("ccSeOn__Fi")] = se
        m.hooks[sym("DispInfo__7ccEventFPcPcPc")] = info

    def cstr(self, p):
        out = bytearray()
        while True:
            c = self.m.load(p + len(out), 1)
            if c == 0:
                return bytes(out)
            out.append(c)

    def fields(self):
        ld = self.m.load
        return f"{s16(ld(CHAR + 0xA6, 2))} {s16(ld(CHAR + 0xAA, 2))} {ld(CHAR + 0xAC, 4):08x}"

    def run(self, status, msg, piros=True, code=0):
        m = self.m
        m.mem[SAVE:SAVE + SIZE] = bytes(SIZE)
        m.store(SAVE + 0x64F9, 1, status & 0xFF)
        m.store(SAVE + 0x6510, 1, msg & 0xFF)
        m.store(SAVEDATA, 4, SAVE)
        m.mem[FADE:FADE + 0x100] = bytes(0x100)
        m.store(SCFADE_P, 4, FADE)
        m.mem[SPC_MANAGER:SPC_MANAGER + 224] = bytes(224)
        for k in range(5):
            m.store(SPC_MANAGER + 44 * k, 4, 0xFFFFFFFF)
        if piros:
            m.store(SPC_MANAGER, 4, 8)
            m.store(SPC_MANAGER + 0x1C, 4, CHAR)
        m.mem[CHAR:CHAR + 0x200] = bytes(0x200)
        m.mem[EV:EV + 0x800] = bytes(0x800)
        m.store(SCRIPT, 2, OP)
        m.store(SCRIPT + 2, 2, code & 0xFFFF)
        m.store(SCRIPT + 4, 2, 0)
        m.store(PTR, 4, SCRIPT)
        self.log = []
        m.call(self.sym("Execute__7ccEventFRPsiii"), [EV, PTR, EVENT, 0, 2])
        head = f"piros {status} {msg} {int(piros)}" + (f" {code}" if code else "")
        return head + " | " + " ; ".join(self.log) + f" | end {self.fields()}"


def records():
    g = Piros()
    out = []
    for status in range(15):
        for msg in range(4):
            out.append(g.run(status, msg))
    out.append(g.run(3, 0, piros=False))
    # Status 9's operands (the finale): 1-4 a colour's ramp, 5 the end.
    for code in range(1, 6):
        out.append(g.run(9, 0, code=code))
    return out


if __name__ == "__main__":
    if sys.argv[1:2] == ["fixture"]:
        lines = ["# tools/test_piros_rs.py: ccEvent::Execute's piros_colour (event 22) run in eemu",
                 "# piros STATUS MSG PIROS [CODE] | flash N COLOUR F12-F15 ; se N ; info LEN:HASH x3 ; "
                 "breath N FIX RATE COLOUR ... | end FIX RATE COLOUR"] + records()
        with open(sys.argv[2], "w") as f:
            f.write("\n".join(lines) + "\n")
        print(f"{len(lines) - 2} records -> {sys.argv[2]}")
    else:
        for line in records():
            print(line)
