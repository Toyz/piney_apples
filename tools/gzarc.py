#!/usr/bin/env python3
"""Sector-aligned gzip archives: DATA/DATA.BIN and STREAM/*.BIN.

Neither has a directory. Each is gzip members laid end to end, every member
starting on a 2048-byte boundary, named by its gzip FNAME field.

    tools/gzarc.py ls      ARCHIVE                  every member
    tools/gzarc.py cat     ARCHIVE NAME             one member, inflated
    tools/gzarc.py extract ARCHIVE DIR [--only GLOB] [--raw]

ARCHIVE is a host path, or IMAGE::PATH to read straight from a disc image
(`work/infection/infection.iso::DATA/DATA.BIN`). Members are written inflated
under their FNAME unless --raw.
"""

import argparse
import fnmatch
import pathlib
import struct
import sys
import zlib

SECTOR = 2048


class Member:
    __slots__ = ("index", "offset", "length", "name", "mtime")

    def __init__(self, index, offset, length, name, mtime):
        self.index = index
        self.offset = offset
        self.length = length    # up to the next member, padding included
        self.name = name
        self.mtime = mtime


def open_bytes(spec):
    if "::" in spec:
        image, inner = spec.split("::", 1)
        sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
        import iso
        disc = iso.Iso(image)
        entry = disc.find(inner)
        return disc.read(entry.lba, entry.size)
    return pathlib.Path(spec).read_bytes()


def members(data):
    starts = [o for o in range(0, len(data), SECTOR) if data[o:o + 3] == b"\x1f\x8b\x08"]
    out = []
    for i, o in enumerate(starts):
        end = starts[i + 1] if i + 1 < len(starts) else len(data)
        flags = data[o + 3]
        mtime = struct.unpack_from("<I", data, o + 4)[0]
        p = o + 10
        if flags & 4:
            p += 2 + struct.unpack_from("<H", data, p)[0]
        name = ""
        if flags & 8:
            nul = data.index(b"\0", p)
            name = data[p:nul].decode("latin-1")
        out.append(Member(i, o, end - o, name, mtime))
    return out


def inflate(data, member):
    return zlib.decompressobj(31).decompress(data[member.offset:member.offset + member.length])


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("ls")
    p.add_argument("archive")
    p = sub.add_parser("cat")
    p.add_argument("archive")
    p.add_argument("name")
    p = sub.add_parser("extract")
    p.add_argument("archive")
    p.add_argument("out")
    p.add_argument("--only", default="*")
    p.add_argument("--raw", action="store_true", help="write the gzip member as stored")
    args = parser.parse_args()

    data = open_bytes(args.archive)
    found = members(data)
    if args.cmd == "ls":
        print(f"{'#':>5} {'offset':>10} {'sector':>7} {'stored':>9} {'mtime':>10}  name")
        for m in found:
            print(f"{m.index:5d} 0x{m.offset:08x} {m.offset // SECTOR:7d} "
                  f"{m.length:9d} {m.mtime:10d}  {m.name}")
        print(f"# {len(found)} members")
    elif args.cmd == "cat":
        want = args.name.lower()
        for m in found:
            if m.name.lower() == want or m.name.lower().rsplit(".", 1)[0] == want:
                sys.stdout.buffer.write(inflate(data, m))
                return 0
        print(f"no member {args.name}", file=sys.stderr)
        return 1
    elif args.cmd == "extract":
        out = pathlib.Path(args.out)
        out.mkdir(parents=True, exist_ok=True)
        n = 0
        for m in found:
            if not fnmatch.fnmatch(m.name.lower(), args.only.lower()):
                continue
            body = data[m.offset:m.offset + m.length] if args.raw else inflate(data, m)
            (out / m.name).write_bytes(body)
            n += 1
        print(f"{n} members -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
