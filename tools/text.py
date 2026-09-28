#!/usr/bin/env python3
"""The in-game desktop's text: mail, bulletin board, news - see
docs/engine/text.md.

    tools/text.py mail ELF [--parody]      every mail and its replies
    tools/text.py bbs  ELF [--parody]      every thread and its posts
    tools/text.py news ELF                 news page titles and their images

Each table has a twin with a `p`/`P` suffix that the game uses when
ccSaveData.parodyFlag (+0x842b) is set; --parody reads that one. In the US
build the parody tables are Japanese.

Strings are ASCII plus Shift-JIS, with the escapes ccKanji::Disp
(0x0015e380) understands, shown here in braces:

    #0      {PC}       ccSaveData.plName, the player character's name
    #1      {PLAYER}   ccSaveData.plRealName, the player's own name
    #R #G #B #Y        {red} {green} {blue} {yellow}
    #W      {/}        back to the default colour
    %x      {%xx}      a two-byte extended font code, x in hex

Mail replies are filled in by the overlay's static initialisers, so mail is
read after running them (tools/eemu.py).
"""

import argparse
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from image import Program  # noqa: E402

COLOURS = {ord("R"): "{red}", ord("G"): "{green}", ord("B"): "{blue}",
           ord("Y"): "{yellow}", ord("W"): "{/}"}


def render(raw):
    """Game text bytes -> readable text with the escapes in braces."""
    out = []
    i = 0
    n = len(raw)
    while i < n:
        c = raw[i]
        if c == 0x23 and i + 1 < n:  # '#'
            code = raw[i + 1]
            if code == 0x30:
                out.append("{PC}")
            elif code == 0x31:
                out.append("{PLAYER}")
            else:
                out.append(COLOURS.get(code, "{#" + chr(code) + "}"))
            i += 2
        elif c == 0x25 and i + 1 < n:  # '%'
            out.append("{%%%02x}" % raw[i + 1])
            i += 2
        elif c < 0x80:
            out.append(chr(c))
            i += 1
        elif (0x81 <= c <= 0x9F or 0xE0 <= c <= 0xFC) and i + 1 < n:
            out.append(raw[i:i + 2].decode("shift_jis", "replace"))
            i += 2
        else:
            out.append(raw[i:i + 1].decode("shift_jis", "replace"))
            i += 1
    return "".join(out)


class Mem:
    """Reads through either a Program or a Machine that has run ctors."""

    def __init__(self, program, machine=None):
        self.p = program
        self.m = machine

    def read(self, va, n):
        if self.m is not None:
            return bytes(self.m.mem[va:va + n])
        return self.p.read(va, n)

    def cstr(self, va):
        if not va:
            return b""
        out = bytearray()
        while True:
            chunk = self.read(va, 64)
            k = chunk.find(b"\0")
            if k >= 0:
                out += chunk[:k]
                return bytes(out)
            out += chunk
            va += 64

    def lines(self, va, count):
        out = []
        for _ in range(count):
            s = self.cstr(va)
            out.append(s)
            va += len(s) + 1
        return out


def table(program, name):
    sym = program.symbol_named(name)
    if sym is None:
        raise SystemExit(f"no symbol {name}")
    return sym


def dump_mail(program, parody):
    import eemu
    mem = Mem(program, eemu.run_ctors(program))
    tbl = table(program, "MailTblp" if parody else "MailTbl")
    count = tbl.size // 0x48
    print(f"# {tbl.name}: {count} mails")
    for i in range(count):
        r = mem.read(tbl.value + 0x48 * i, 0x48)
        no, re_flg, mail_flg, title, frm, from_no, line, body = struct.unpack_from("<iiiIIiiI", r)
        print(f"\n## {no}. {render(mem.cstr(title))}")
        print(f"from: {render(mem.cstr(frm))} (sender {from_no})  flags: re {re_flg}, mail {mail_flg}")
        for ln in mem.lines(body, line):
            print(f"    {render(ln)}")
        for label, off in (("reply 1", 0x20), ("reply 2", 0x34)):
            flg, rtitle, rfrom, rline, rbody = struct.unpack_from("<iIIiI", r, off)
            if not rtitle and not rbody:
                continue
            print(f"  {label}: {render(mem.cstr(rtitle))} - from {render(mem.cstr(rfrom))} (flag {flg})")
            for ln in mem.lines(rbody, rline):
                print(f"      {render(ln)}")


def dump_bbs(program, parody):
    mem = Mem(program)
    tbl = table(program, "bbsThreadTblP" if parody else "bbsThreadTbl")
    va = tbl.value
    print(f"# {tbl.name}")
    t = 0
    while True:
        title, msgs, num = struct.unpack("<IIi", mem.read(va, 12))
        if not title or num <= 0 or num > 1000:
            break
        print(f"\n## thread {t}: {render(mem.cstr(title))} ({num} posts)")
        for k in range(num):
            date, ptitle, name, lines, body = struct.unpack("<iIIiI", mem.read(msgs + 0x14 * k, 0x14))
            print(f"\n### {render(mem.cstr(ptitle))}  -  {render(mem.cstr(name))}  (date {date})")
            for ln in mem.lines(body, lines):
                print(f"    {render(ln)}")
        va += 12
        t += 1


def dump_news(program):
    mem = Mem(program)
    tbl = table(program, "HtmlTbl")
    count = tbl.size // 0x1C
    print(f"# {tbl.name}: {count} pages")
    for i in range(count):
        no, flg, title, ccs, chunk, html, height = struct.unpack("<iiIIIIi", mem.read(tbl.value + 0x1C * i, 0x1C))
        print(f"{no:4d}  flg {flg:3d}  {render(mem.cstr(title)):48}  {render(mem.cstr(ccs))}::"
              f"{render(mem.cstr(chunk))}  height {height}")


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    for name in ("mail", "bbs", "news"):
        p = sub.add_parser(name)
        p.add_argument("elf")
        if name != "news":
            p.add_argument("--parody", action="store_true")
    args = parser.parse_args()

    if args.cmd == "mail":
        dump_mail(Program(args.elf, "desktop"), args.parody)
    elif args.cmd == "bbs":
        dump_bbs(Program(args.elf, "toppage"), args.parody)
    else:
        dump_news(Program(args.elf, "desktop"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
