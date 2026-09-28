#!/usr/bin/env python3
"""Textures and palettes out of CCSF files, as PNG - see docs/formats/ccs.md.

    tools/ccstex.py ls     FILE                    every texture and its palette
    tools/ccstex.py png    FILE OUTDIR [--mips] [--matte]
                                                   one PNG per texture
    tools/ccstex.py dump   ARCHIVE OUTDIR          every texture in an archive

FILE is an inflated .ccs, or ARCHIVE::MEMBER (see tools/ccs.py). Layouts are
the ones ccStream::Decode_Texture (0x0014d9a0) and Decode_Clut (0x0014d660)
read. Palette entries are in logical order in the file - the game swizzles
256-entry palettes into the GS's CSM1 order itself, through clut256Tbl - so
they are used as they are. GS alpha runs 0..0x80; it is doubled for PNG.

Rows are stored bottom-up - the art was authored as .bmp, which is bottom-up,
and the exporter kept the row order - so the PNG is flipped to read the right
way up. --raw-rows keeps the stored order.
"""

import argparse
import pathlib
import struct
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import ccs  # noqa: E402
import png  # noqa: E402

PSMT8 = 0x13
PSMT4 = 0x14


class Clut:
    __slots__ = ("obj", "flag", "buf_x", "buf_y", "colours")


class Texture:
    __slots__ = ("obj", "clut", "blt", "flag", "psm", "mipmap", "aref", "tw", "th", "levels")

    @property
    def size(self):
        return 1 << self.tw, 1 << self.th


def read(c):
    """(textures, {clut object: Clut}) from a Ccs, as far as its walk goes."""
    d = c.data
    textures = []
    cluts = {}
    for off, t, n, end in c.chunks():
        if t is None:
            break
        kind = t & 0xFFFF
        if kind == 0x0400:
            q = off + 8
            cl = Clut()
            cl.obj = struct.unpack_from("<I", d, q)[0]
            q += 8 if c.version >= 0x92 else 4
            cl.flag = d[q]
            cl.buf_x, cl.buf_y, count = struct.unpack_from("<HHI", d, q + 4)
            q += 12
            cl.colours = [d[q + 4 * i:q + 4 * i + 4] for i in range(count)]
            cluts[cl.obj] = cl
        elif kind == 0x0300:
            q = off + 8
            tx = Texture()
            tx.obj, tx.clut = struct.unpack_from("<II", d, q)
            tx.blt = struct.unpack_from("<I", d, q + 8)[0] if c.version >= 0x92 else None
            q += 12 if c.version >= 0x92 else 8
            tx.flag, tx.psm, tx.mipmap, tx.aref, tx.tw, tx.th = d[q:q + 6]
            q += 8
            tx.levels = []
            for _ in range(tx.mipmap + 1):
                bx, by, count = struct.unpack_from("<HHI", d, q)
                tx.levels.append((bx, by, d[q + 8:q + 8 + 4 * count]))
                q += 8 + 4 * count
            textures.append(tx)
    return textures, cluts


def to_rgba(tx, clut, level=0, flip=True):
    w, h = tx.size
    w >>= level
    h >>= level
    pixels = tx.levels[level][2]
    palette = []
    for col in clut.colours:
        r, g, b, a = col
        palette.append(bytes((r, g, b, min(255, a * 2))))
    out = bytearray()
    if tx.psm == PSMT8:
        for i in range(w * h):
            out += palette[pixels[i]]
    elif tx.psm == PSMT4:
        for i in range(w * h):
            byte = pixels[i >> 1]
            out += palette[(byte >> 4) if i & 1 else (byte & 15)]
    else:
        raise ValueError(f"psm 0x{tx.psm:02x} not handled")
    if flip:
        stride = w * 4
        out = b"".join(out[y * stride:(y + 1) * stride] for y in range(h - 1, -1, -1))
    return w, h, bytes(out)


def matte(rgba, colour=(0x20, 0x20, 0x28)):
    """Composite over a solid colour: many textures are white with the shape in
    alpha only, which is invisible on a white background."""
    out = bytearray(len(rgba))
    for i in range(0, len(rgba), 4):
        a = rgba[i + 3]
        for ch in range(3):
            out[i + ch] = (rgba[i + ch] * a + colour[ch] * (255 - a)) // 255
        out[i + 3] = 255
    return bytes(out)


def export(c, outdir, mips=False, prefix="", flatten=False, flip=True):
    textures, cluts = read(c)
    outdir.mkdir(parents=True, exist_ok=True)
    written = 0
    for tx in textures:
        name = c.objects[tx.obj][0] if tx.obj < len(c.objects) else f"tex{tx.obj}"
        clut = cluts.get(tx.clut)
        if clut is None:
            print(f"{prefix}{name}: palette object {tx.clut} is not in this file", file=sys.stderr)
            continue
        for level in range(len(tx.levels) if mips else 1):
            w, h, rgba = to_rgba(tx, clut, level, flip)
            if flatten:
                rgba = matte(rgba)
            suffix = f"_mip{level}" if level else ""
            png.write_rgba(outdir / f"{prefix}{name}{suffix}.png", w, h, rgba)
            written += 1
    return written


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("ls").add_argument("file")
    p = sub.add_parser("png")
    p.add_argument("file")
    p.add_argument("out")
    p.add_argument("--mips", action="store_true")
    p.add_argument("--matte", action="store_true", help="composite over dark grey")
    p.add_argument("--raw-rows", action="store_true", help="keep the stored bottom-up row order")
    p = sub.add_parser("dump")
    p.add_argument("archive")
    p.add_argument("out")
    p.add_argument("--matte", action="store_true", help="composite over dark grey")
    args = parser.parse_args()

    if args.cmd == "dump":
        import gzarc
        data = gzarc.open_bytes(args.archive)
        total = 0
        out = pathlib.Path(args.out)
        for m in gzarc.members(data):
            c = ccs.Ccs(gzarc.inflate(data, m))
            total += export(c, out / c.name, flatten=args.matte)
        print(f"{total} PNGs -> {out}")
        return 0

    c = ccs.Ccs(ccs.load(args.file))
    if args.cmd == "ls":
        textures, cluts = read(c)
        for tx in textures:
            w, h = tx.size
            cl = cluts.get(tx.clut)
            print(f"{c.objects[tx.obj][0]:24} {w:4}x{h:<4} psm 0x{tx.psm:02x} "
                  f"mips {tx.mipmap} aref 0x{tx.aref:02x} flag 0x{tx.flag:02x}  "
                  f"clut {c.objects[tx.clut][0]} ({len(cl.colours) if cl else '?'} colours)")
    elif args.cmd == "png":
        n = export(c, pathlib.Path(args.out), mips=args.mips, flatten=args.matte,
                   flip=not args.raw_rows)
        print(f"{n} PNGs -> {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
