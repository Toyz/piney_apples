#!/usr/bin/env python3
"""CCSF scene files - see docs/formats/ccs.md.

    tools/ccs.py info    FILE           header and name table
    tools/ccs.py objects FILE [--prefix TEX_]
    tools/ccs.py chunks  FILE           walk the chunks
    tools/ccs.py survey  ARCHIVE        walk every member, report where walks fail

FILE is an inflated .ccs on disk, or ARCHIVE::MEMBER to pull one out of a
gzip archive (`work/infection/disc/DATA/DATA.BIN::xasc00`).

The game never skips a chunk by its size field: ccStream::DecodeSetupSection
dispatches on the low 16 bits of the type and each decoder reads exactly what
it needs. The walk here uses the size field except where a decoder is known to
disagree with it - textures and models, whose lengths follow from their own
fields - and stops when it lands on something that is not a chunk header.

A file is the header and name table, then the setup section (KINDS) up to and
including the Frame chunk, then the frame section (FRAME_KINDS) that
ccStream::DecodeFrameChunk reads: per frame a Top chunk (0xccccff01, u32 frame
number) and that frame's chunks, closed by a Top numbered 0xffffffff (or
0xfffffffe, which also resets the scene). Every frame-section decoder reads
exactly its size field, checked over all four STREAM archives. Models are
decoded in full by tools/ccsmodel.py.
"""

import argparse
import pathlib
import struct
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))


# Low halves ccStream::DecodeSetupSection dispatches on, and its decoder.
KINDS = {
    0x0003: "Setup", 0x0005: "Frame", 0x0100: "Obj", 0x0200: "Material",
    0x0300: "Texture", 0x0400: "Clut", 0x0500: "Camera", 0x0600: "Light",
    0x0700: "Anime", 0x0800: "Model", 0x0900: "Clump", 0x0a00: "ExtObj",
    0x0b00: "Hit", 0x0c00: "Bbox", 0x0d00: "Particle", 0x0e00: "Eff",
    0x1000: "BltGrp", 0x1100: "FBRect", 0x1200: "FBPage", 0x1300: "DummyPos",
    0x1400: "DummyPosRot", 0x1700: "Layer", 0x1800: "Shadow", 0x1900: "Morpher",
    0x2000: "Obj2", 0x2200: "Pcm",
}
# Low halves ccStream::DecodeFrameChunk (0x0014e1b0) dispatches on. Frame ends
# the setup section and may repeat here; Top starts a frame.
FRAME_KINDS = {
    0xff01: "Top", 0x0005: "Frame", 0x0101: "F_Obj", 0x0108: "F_Note",
    0x0201: "F_Material", 0x0502: "F_Camera", 0x0601: "F_Ambient",
    0x0602: "F_DistantLight", 0x0604: "F_DirectLight", 0x0606: "F_SpotLight",
    0x0608: "F_OmniLight", 0x0802: "F_ModelVertex", 0x0803: "F_ModelNormal",
    0x1801: "F_Shadow", 0x1901: "F_Morpher", 0x2201: "F_Pcm",
}
HEAD_KINDS = {0x0001: "Header", 0x0002: "Index"}
# Neither dispatcher looks at the high half. Most chunks carry 0xcccc there;
# Setup is always written with 0, and stream files also write Frame, Pcm and
# F_Pcm with 0. Anything else is taken as the walk having left the rails.
HIGH_HALVES = (0xCCCC, 0x0000)
# Top frame numbers that close the frame section: DecodeFrameChunk returns the
# number, and DecodeFrameSection (0x0014ded0) treats -1 as the end and -2 as
# the end plus ccStream::ResetScene.
END_FRAMES = {0xFFFFFFFF: "end", 0xFFFFFFFE: "end, reset scene"}


def _align4(q):
    return (q + 3) & ~3


class Ccs:
    def __init__(self, data):
        self.data = data
        t, n = struct.unpack_from("<II", data, 0)
        if t != 0xCCCC0001 or data[8:12] != b"CCSF":
            raise ValueError("not a CCSF file")
        self.name = data[12:44].split(b"\0")[0].decode("latin-1")
        # ccStream::DecodeHeaderSection: u16 version, pad to 4, u32 frames
        # (ccStream+0x16c: DecodeF_Top records where frame f starts only for
        # f below it), u32 count, u32[count].
        self.version = struct.unpack_from("<H", data, 44)[0]
        self.frames, count = struct.unpack_from("<II", data, 48)
        self.header_words = list(struct.unpack_from(f"<{count}I", data, 56))
        t, n = struct.unpack_from("<II", data, 60)
        if t != 0xCCCC0002:
            raise ValueError(f"second chunk is 0x{t:08x}, not the name table")
        fc, oc = struct.unpack_from("<II", data, 68)
        base = 76
        self.files = []
        for k in range(fc):
            raw = data[base + 32 * k:base + 32 * k + 32]
            self.files.append(raw.split(b"\0")[0].decode("latin-1"))
        self.objects = []
        for k in range(oc):
            raw = data[base + 32 * (fc + k):base + 32 * (fc + k) + 32]
            name = raw[:30].split(b"\0")[0].decode("latin-1")
            self.objects.append((name, struct.unpack_from("<H", raw, 30)[0]))
        self.body = 68 + n * 4

    def texture_end(self, p):
        """End of the texture chunk at p, as ccStream::Decode_Texture reads it."""
        q = p + 8 + (12 if self.version >= 0x92 else 8)
        mipmaps = 0
        if self.version >= 0x90:
            mipmaps = self.data[q + 2]
            q += 8      # six u8 fields, padded to 4
        else:
            q += 4      # two u8 fields, padded to 4
        for _ in range(mipmaps + 1):
            n = struct.unpack_from("<I", self.data, q + 4)[0]
            q += 8 + 4 * n
        return q

    def model_end(self, p):
        """End of the model chunk at p, as ccStream::Decode_Model (0x0014bce0)
        and the mmat decoders it calls read it. tools/ccsmodel.py decodes the
        same fields."""
        d = self.data
        mtype, count = struct.unpack_from("<HH", d, p + 16)
        if self.version < 0x100:
            mtype &= 0xFF       # 0x0014bd80: older files use the low byte only
        # object, vertexScale, mtype, mmat count, flag, zoffs, u8, pad to 4
        q = p + 8 + 20
        for _ in range(count):
            # Per-mmat header words, 0x0014be58..0x0014bf7c.
            if mtype & 2:
                vn, on = struct.unpack_from("<I4xI", d, q + 8)
                q += 20
            elif mtype & 4:
                vn, on = struct.unpack_from("<II", d, q + 4)
                q += 12
            elif mtype & 8:
                vn = on = 0
            else:
                vn = struct.unpack_from("<I", d, q + 8)[0]
                on = 0
                q += 12
            if mtype & 4:
                # Decode_Mmat02 (0x0014a600).
                if on:
                    # 8-byte offset entries until a vertex-end flag, one
                    # normal word per entry, one ST word per vertex.
                    ends = sum(d[q + 8 * k + 7] >> 1 & 1 for k in range(on))
                    q += 12 * on + 4 * ends
                else:
                    # Bone slot word, s16 x3 positions padded to 4, normal
                    # and ST words.
                    q = _align4(q + 4 + 6 * vn) + 8 * vn
            elif mtype & 8:
                # DecodeShadowModel (0x00140920): s32 vertex and index
                # counts, s16 x3 positions padded to 4, s32 x3 triangles.
                vnum, inum = struct.unpack_from("<ii", d, q)
                q += 8
                if vnum:
                    q = _align4(q + 6 * vnum) + 12 * int(inum / 3)
            else:
                # Decode_Mmat01 (0x0014b890): s16 x3 positions padded to 4,
                # then normal, colour and ST words unless 0x40, 0x200, 0x400.
                q = _align4(q + 6 * vn)
                for bit in (0x40, 0x200, 0x400):
                    if not mtype & bit:
                        q += 4 * vn
        return q

    def chunks(self):
        """Yield (offset, type, size_words, end) until the walk leaves the rails."""
        p = 0
        d = self.data
        allowed = KINDS.keys() | HEAD_KINDS.keys()
        frame_section = False
        while p + 8 <= len(d):
            t, n = struct.unpack_from("<II", d, p)
            kind = t & 0xFFFF
            if (t >> 16) not in HIGH_HALVES or kind not in allowed:
                yield (p, None, None, None)
                return
            if frame_section:
                end = p + 8 + n * 4
            elif kind == 0x0300:
                end = self.texture_end(p)
            elif kind == 0x0800:
                end = self.model_end(p)
            else:
                end = p + 8 + n * 4
            yield (p, t, n, end)
            if kind == 0x0005 and not frame_section:
                # Decode_Frame ends DecodeSetupSection; DecodeFrameSection
                # takes over with its own dispatcher.
                frame_section = True
                allowed = FRAME_KINDS.keys()
            p = end


def load(spec):
    if "::" in spec:
        import gzarc
        archive, member = spec.rsplit("::", 1)
        data = gzarc.open_bytes(archive)
        want = member.lower()
        for m in gzarc.members(data):
            if m.name.lower() == want or m.name.lower().rsplit(".", 1)[0] == want:
                return gzarc.inflate(data, m)
        raise KeyError(member)
    return pathlib.Path(spec).read_bytes()


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    for name in ("info", "chunks", "survey"):
        sub.add_parser(name).add_argument("file")
    p = sub.add_parser("objects")
    p.add_argument("file")
    p.add_argument("--prefix", default="")
    args = parser.parse_args()

    ccs = None if args.cmd == "survey" else Ccs(load(args.file))
    if args.cmd == "info":
        print(f"name {ccs.name}  version 0x{ccs.version:x}  frames {ccs.frames}  "
              f"header words {ccs.header_words}")
        print(f"{len(ccs.files)} files, {len(ccs.objects)} objects (both counting blank entry 0)")
        for i, f in enumerate(ccs.files):
            if i:
                print(f"  file {i:4d}  {f!r}")
    elif args.cmd == "objects":
        for i, (name, fidx) in enumerate(ccs.objects):
            if i and name.startswith(args.prefix):
                print(f"{i:6d}  {name:30}  {ccs.files[fidx].strip()}")
    elif args.cmd == "chunks":
        for off, t, n, end in ccs.chunks():
            if t is None:
                print(f"0x{off:08x}  walk lost - not a chunk header here")
                return 1
            kind = t & 0xFFFF
            obj = ""
            if kind == 0xFF01:
                f = struct.unpack_from("<I", ccs.data, off + 8)[0]
                obj = END_FRAMES.get(f, f"frame {f}")
            elif n and kind not in NO_OBJECT:
                oid = struct.unpack_from("<I", ccs.data, off + 8)[0]
                if 0 < oid < len(ccs.objects):
                    obj = ccs.objects[oid][0]
            real = (end - off - 8) // 4
            note = "" if real == n else f"  (reads {real})"
            print(f"0x{off:08x}  0x{t:08x}  {kind_name(kind):14} "
                  f"{n:8d} words{note}  {obj}")
    elif args.cmd == "survey":
        print_survey(survey(args.file))
    return 0


# Chunks whose first word is not an object index.
NO_OBJECT = {0x0001, 0x0002, 0x0003, 0x0005, 0x0601, 0x2200, 0x2201}


def kind_name(kind):
    return HEAD_KINDS.get(kind) or KINDS.get(kind) or FRAME_KINDS.get(kind, "?")


class Survey:
    """What walking every member of an archive found."""

    def __init__(self):
        import collections
        self.members = self.clean = 0
        self.lost = collections.Counter()           # kind walked before -> members
        self.example = {}
        self.counts = collections.Counter()         # kind -> chunks
        self.disagree = collections.Counter()       # kind -> chunks longer or shorter than size
        self.closes = collections.Counter()         # END_FRAMES name -> members
        self.animated = self.frames = self.unterminated = self.unordered = 0


def survey(spec):
    import gzarc
    data = gzarc.open_bytes(spec)
    s = Survey()
    for m in gzarc.members(data):
        s.members += 1
        c = Ccs(gzarc.inflate(data, m))
        prev = None
        ok = False
        tops = []
        for off, t, n, end in c.chunks():
            if t is None:
                kind = kind_name(prev & 0xFFFF) if prev is not None else "start"
                s.lost[kind] += 1
                s.example.setdefault(kind, m.name)
                break
            s.counts[t & 0xFFFF] += 1
            if end != off + 8 + 4 * n:
                s.disagree[t & 0xFFFF] += 1
            if t & 0xFFFF == 0xFF01:
                tops.append(struct.unpack_from("<I", c.data, off + 8)[0])
            prev = t
            if end == len(c.data):
                ok = True
        s.clean += ok
        if not tops or tops[-1] not in END_FRAMES:
            s.unterminated += 1
            continue
        s.closes[END_FRAMES[tops[-1]]] += 1
        if len(tops) > 1:
            s.animated += 1
            s.frames += len(tops) - 1
            s.unordered += tops[:-1] != list(range(len(tops) - 1))
    return s


def print_survey(s):
    print(f"{s.clean} of {s.members} members walk to their last byte")
    for kind, n in s.lost.most_common():
        print(f"  lost after {kind}: {n} (e.g. {s.example[kind]})")
    print(f"frame section: {s.animated} members hold frames, {s.frames} frames in all "
          f"({s.unordered} members not numbered 0, 1, 2, ...); closed by "
          f"{dict(s.closes)}, {s.unterminated} not closed")
    print("chunks seen before any walk was lost (and how many disagree with their size field):")
    for k, n in sorted(s.counts.items()):
        note = f"  ({s.disagree[k]} disagree)" if s.disagree[k] else ""
        print(f"  0x{k:04x} {kind_name(k):14} {n}{note}")


if __name__ == "__main__":
    sys.exit(main())
