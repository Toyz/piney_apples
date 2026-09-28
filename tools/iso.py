#!/usr/bin/env python3
"""ISO 9660 images, read directly - no mounting, no 7z.

PS2 DVDs are plain ISO 9660 (no Joliet, no Rock Ridge) with 2048-byte
sectors. Every entry records its starting LBA, which matters here: the game
code addresses some data by sector rather than by name.

    tools/iso.py ls      IMAGE [--lba]          every file, recursively
    tools/iso.py cat     IMAGE PATH             one file to stdout
    tools/iso.py extract IMAGE DIR [--only GLOB]
    tools/iso.py where   IMAGE LBA              which file holds a sector
    tools/iso.py sums    IMAGE                  every file's SHA-1, in disc order
"""

import argparse
import fnmatch
import hashlib
import pathlib
import struct
import sys

SECTOR = 2048


class Entry:
    __slots__ = ("path", "lba", "size", "is_dir")

    def __init__(self, path, lba, size, is_dir):
        self.path = path
        self.lba = lba
        self.size = size
        self.is_dir = is_dir


class Iso:
    def __init__(self, path):
        self.f = open(path, "rb")
        pvd = self.read_sectors(16, 1)
        if pvd[0] != 1 or pvd[1:6] != b"CD001":
            raise ValueError(f"{path}: no primary volume descriptor at sector 16")
        self.system_id = pvd[8:40].decode("ascii").strip()
        self.volume_id = pvd[40:72].decode("ascii").strip()
        self.volume_sectors = struct.unpack_from("<I", pvd, 80)[0]
        root = pvd[156:156 + 34]
        self.root_lba, self.root_size = struct.unpack_from("<I4xI", root, 2)

    def read_sectors(self, lba, count):
        self.f.seek(lba * SECTOR)
        return self.f.read(count * SECTOR)

    def read(self, lba, size):
        self.f.seek(lba * SECTOR)
        return self.f.read(size)

    def walk(self, lba=None, size=None, prefix=""):
        """Yield every Entry below a directory, depth first, in disc order."""
        if lba is None:
            lba, size = self.root_lba, self.root_size
        data = self.read(lba, size)
        pos = 0
        while pos < len(data):
            length = data[pos]
            if length == 0:
                # Records never straddle a sector; the rest is padding.
                pos = (pos // SECTOR + 1) * SECTOR
                continue
            rec = data[pos:pos + length]
            pos += length
            ext_lba, ext_size = struct.unpack_from("<I4xI", rec, 2)
            flags = rec[25]
            name_len = rec[32]
            raw = rec[33:33 + name_len]
            if raw in (b"\x00", b"\x01"):
                continue  # . and ..
            name = raw.decode("ascii")
            if ";" in name:
                name = name.split(";", 1)[0]
            path = prefix + name
            is_dir = bool(flags & 2)
            yield Entry(path, ext_lba, ext_size, is_dir)
            if is_dir:
                yield from self.walk(ext_lba, ext_size, path + "/")

    def files(self):
        return [e for e in self.walk() if not e.is_dir]

    def find(self, path):
        want = path.strip("/").upper()
        for e in self.walk():
            if e.path.upper() == want:
                return e
        raise KeyError(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("ls")
    p.add_argument("image")
    p.add_argument("--lba", action="store_true", help="sort by LBA, not name")
    p = sub.add_parser("cat")
    p.add_argument("image")
    p.add_argument("path")
    p = sub.add_parser("extract")
    p.add_argument("image")
    p.add_argument("out")
    p.add_argument("--only", default="*")
    p = sub.add_parser("where")
    p.add_argument("image")
    p.add_argument("lba", type=lambda s: int(s, 0))
    p = sub.add_parser("sums")
    p.add_argument("image")
    args = parser.parse_args()

    iso = Iso(args.image)
    if args.cmd == "ls":
        entries = iso.files()
        if args.lba:
            entries.sort(key=lambda e: e.lba)
        print(f"# system={iso.system_id!r} volume={iso.volume_id!r} "
              f"sectors={iso.volume_sectors}")
        print(f"{'lba':>8} {'sectors':>8} {'size':>12}  path")
        for e in entries:
            secs = (e.size + SECTOR - 1) // SECTOR
            print(f"{e.lba:8d} {secs:8d} {e.size:12d}  {e.path}")
    elif args.cmd == "sums":
        for e in sorted(iso.files(), key=lambda e: e.lba):
            h = hashlib.sha1()
            iso.f.seek(e.lba * SECTOR)
            left = e.size
            while left:
                chunk = iso.f.read(min(left, 1 << 22))
                h.update(chunk)
                left -= len(chunk)
            print(f"{h.hexdigest()}  {e.size:12d}  {e.path}")
    elif args.cmd == "cat":
        e = iso.find(args.path)
        sys.stdout.buffer.write(iso.read(e.lba, e.size))
    elif args.cmd == "extract":
        out = pathlib.Path(args.out)
        n = 0
        for e in iso.files():
            if not fnmatch.fnmatch(e.path.upper(), args.only.upper()):
                continue
            dest = out / e.path
            dest.parent.mkdir(parents=True, exist_ok=True)
            with open(dest, "wb") as w:
                iso.f.seek(e.lba * SECTOR)
                left = e.size
                while left:
                    chunk = iso.f.read(min(left, 1 << 22))
                    w.write(chunk)
                    left -= len(chunk)
            n += 1
        print(f"{n} files -> {out}")
    elif args.cmd == "where":
        for e in iso.files():
            secs = (e.size + SECTOR - 1) // SECTOR
            if e.lba <= args.lba < e.lba + max(secs, 1):
                print(f"{e.path} +0x{(args.lba - e.lba) * SECTOR:x}")
                return 0
        print("no file", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
