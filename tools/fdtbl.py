#!/usr/bin/env python3
"""The DATA.BIN index, read out of the executable - see docs/formats/data-bin.md.

The archive has no directory; the executable carries one. `categoryFDTbl`
points at one table per category, each a run of 44-byte records ending in an
all-zero record, and `cateCDOfsTbl` gives each category's byte offset in the
archive as a u64.

    tools/fdtbl.py ls     ELF [--category NAME]   every record, with its sector
    tools/fdtbl.py check  ELF DATA.BIN             every record against the archive

The executable's symbol table names all three tables, so nothing here is a
hard-coded address.
"""

import argparse
import pathlib
import struct
import sys
import zlib

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from image import Program  # noqa: E402

RECORD = 44
SECTOR = 2048


class Record:
    __slots__ = ("category", "index", "name", "offset", "csize", "usize")

    def __init__(self, category, index, name, offset, csize, usize):
        self.category = category
        self.index = index
        self.name = name
        self.offset = offset    # bytes, from the start of the category
        self.csize = csize      # the gzip member, without padding
        self.usize = usize      # inflated

    def sector(self, category_base):
        return category_base // SECTOR + (self.offset + SECTOR - 1) // SECTOR


# Infection's category names, for volumes whose tables carry no symbols.
CATEGORIES = ("cmn", "gcmn", "demo", "desktop", "toppage", "menu", "effect", "equip",
              "skill", "spc", "pc", "npc", "gimmick", "enemy", "boss", "town", "field",
              "dungeon", "event", "direct")


def load(path):
    """[(category name, base byte offset, [Record])] in category order.

    Works on a stripped volume too, through the names piney-gen syms carries
    over: only categoryFDTbl and cateCDOfsTbl need to be known. Each table is
    read up to its all-zero terminator, not by a symbol's size, since later
    volumes' tables are longer than Infection's."""
    p = Program(path)
    fd = p.symbol_named("categoryFDTbl")
    ofs = p.symbol_named("cateCDOfsTbl")
    if fd is None or ofs is None:
        raise SystemExit(f"{path}: categoryFDTbl / cateCDOfsTbl not named")
    out = []
    for k in range(fd.size // 4):
        va = p.u32(fd.value + 4 * k)
        hit = p.symbol_at(va)
        if hit and hit[1] == 0 and hit[0].name.endswith("CCSTbl"):
            cat = hit[0].name[:-len("CCSTbl")]
        else:
            cat = CATEGORIES[k] if k < len(CATEGORIES) else f"category{k}"
        base = struct.unpack("<Q", p.read(ofs.value + 8 * k, 8))[0]
        recs = []
        for i in range(1 << 16):
            raw = p.read(va + RECORD * i, RECORD)
            name = raw[:32].split(b"\0")[0].decode("latin-1")
            if not name:
                break   # the terminating all-zero record
            offset, csize, usize = struct.unpack_from("<III", raw, 32)
            recs.append(Record(k, i, name, offset, csize, usize))
        out.append((cat, base, recs))
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("ls")
    p.add_argument("elf")
    p.add_argument("--category")
    p = sub.add_parser("check")
    p.add_argument("elf")
    p.add_argument("archive")
    args = parser.parse_args()

    cats = load(args.elf)
    if args.cmd == "ls":
        print(f"{'cat':>3} {'category':10} {'sector':>7} {'csize':>9} {'usize':>9}  name")
        for cat, base, recs in cats:
            if args.category and cat != args.category:
                continue
            for r in recs:
                print(f"{r.category:3d} {cat:10} {r.sector(base):7d} {r.csize:9d} "
                      f"{r.usize:9d}  {r.name}")
        total = sum(len(r) for _, _, r in cats)
        print(f"# {total} records in {len(cats)} categories")
        return 0

    data = pathlib.Path(args.archive).read_bytes()
    bad = 0
    total = 0
    expect = 0
    for cat, base, recs in cats:
        if recs and base != expect:
            print(f"{cat}: base 0x{base:x}, but the previous category ended at 0x{expect:x}")
            bad += 1
        for r in recs:
            total += 1
            at = r.sector(base) * SECTOR
            member = data[at:at + r.csize]
            problem = None
            if member[:3] != b"\x1f\x8b\x08":
                problem = "no gzip member here"
            else:
                z = zlib.decompressobj(31)
                body = z.decompress(member)
                name = member[10:member.index(b"\0", 10)].decode("latin-1")
                if not z.eof:
                    problem = "csize cuts the member short"
                elif len(body) != r.usize:
                    problem = f"inflates to {len(body)}, record says {r.usize}"
                elif name.rsplit(".", 1)[0].upper() != r.name.rsplit(".", 1)[0].upper():
                    problem = f"member is named {name}"
            if problem:
                bad += 1
                print(f"{cat} {r.name}: {problem}")
            expect = at + (r.csize + SECTOR - 1) // SECTOR * SECTOR
    if expect != len(data):
        print(f"the last record ends at 0x{expect:x}; the archive is 0x{len(data):x} bytes")
        bad += 1
    print(f"{total} records, {bad} problems")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
