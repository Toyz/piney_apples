#!/usr/bin/env python3
"""PS-ADPCM, the SPU/SPU2 sample format ("VAG" data).

    tools/adpcm.py decode FILE OUT.wav --rate HZ [--offset N]   one sample to WAV
    tools/adpcm.py blocks FILE [--offset N] [--count N]         the frame headers

Data is 16-byte frames, each 28 samples:

    u8   shift | filter << 4    shift 0..12 (13..15 behave as 9), filter 0..4
    u8   flags                  bit 0 end, bit 1 repeat, bit 2 loop start
    u8[14] nibbles              low nibble first, signed 4-bit

    s = (nibble << 12 >> shift) + (s1 * F0[filter] + s2 * F1[filter] + 32) >> 6

clamped to 16 bits, with F0 = 0, 60, 115, 98, 122 and F1 = 0, 0, -52, -55,
-60 (sixty-fourths). A sample ends with the frame whose end bit is set; the
voice then jumps to the last loop-start frame if the repeat bit is also set,
and releases otherwise. The frame with the end bit is played, so it belongs
to the sample.

Also a library: `decode(data, offset)` gives a `Sample` (pcm, loop, frames,
flags seen); `decode_frames(data, offset, count)` decodes a fixed run.
"""

import argparse
import array
import pathlib
import sys

FRAME = 16
PER_FRAME = 28
F0 = (0, 60, 115, 98, 122)
F1 = (0, 0, -52, -55, -60)

END = 0x01
REPEAT = 0x02
LOOP_START = 0x04


class Sample:
    __slots__ = ("pcm", "loop", "frames", "flags", "reserved_filters", "clipped")

    def __init__(self, pcm, loop, frames, flags, reserved_filters, clipped):
        self.pcm = pcm              # array('h')
        self.loop = loop            # (start, end) in samples, end exclusive, or None
        self.frames = frames        # 16-byte frames consumed, the end frame included
        self.flags = flags          # set of flag bytes seen
        self.reserved_filters = reserved_filters   # frames with filter > 4
        self.clipped = clipped      # samples that hit the 16-bit clamp


def _frame(data, p, s1, s2, out):
    """Decode the 28 samples of the frame at p into out; return (s1, s2, clipped)."""
    head = data[p]
    shift = head & 15
    if shift > 12:
        shift = 9
    f = head >> 4
    if f > 4:
        f = 4
    f0 = F0[f]
    f1 = F1[f]
    clipped = 0
    for b in data[p + 2:p + 16]:
        for n in (b & 15, b >> 4):
            t = (n - 16 if n & 8 else n) << 12 >> shift
            s = t + ((s1 * f0 + s2 * f1 + 32) >> 6)
            if s > 32767:
                s = 32767
                clipped += 1
            elif s < -32768:
                s = -32768
                clipped += 1
            out.append(s)
            s2 = s1
            s1 = s
    return s1, s2, clipped


def decode(data, offset=0, limit=None):
    """Decode one sample starting at `offset`, up to and including the first
    frame with the end bit, or `limit` bytes, or the end of `data`."""
    stop = len(data) if limit is None else min(len(data), offset + limit)
    out = array.array("h")
    s1 = s2 = 0
    p = offset
    loop_start = None
    loop = None
    flags = set()
    reserved = 0
    clipped = 0
    while p + FRAME <= stop:
        fl = data[p + 1]
        flags.add(fl)
        if data[p] >> 4 > 4:
            reserved += 1
        if fl & LOOP_START:
            loop_start = len(out)
        s1, s2, c = _frame(data, p, s1, s2, out)
        clipped += c
        p += FRAME
        if fl & END:
            if fl & REPEAT and loop_start is not None:
                loop = (loop_start, len(out))
            break
    return Sample(out, loop, (p - offset) // FRAME, flags, reserved, clipped)


def decode_frames(data, offset=0, count=None):
    """Decode `count` frames (default: to the end of data) ignoring flags."""
    end = len(data) if count is None else offset + FRAME * count
    out = array.array("h")
    s1 = s2 = 0
    for p in range(offset, end - FRAME + 1, FRAME):
        s1, s2, _ = _frame(data, p, s1, s2, out)
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("decode")
    p.add_argument("file")
    p.add_argument("out")
    p.add_argument("--rate", type=int, required=True)
    p.add_argument("--offset", type=lambda s: int(s, 0), default=0)
    p = sub.add_parser("blocks")
    p.add_argument("file")
    p.add_argument("--offset", type=lambda s: int(s, 0), default=0)
    p.add_argument("--count", type=int, default=32)
    args = parser.parse_args()

    data = pathlib.Path(args.file).read_bytes()
    if args.cmd == "decode":
        sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
        import wav
        s = decode(data, args.offset)
        wav.write(args.out, s.pcm, args.rate, 1, s.loop)
        print(f"{len(s.pcm)} samples in {s.frames} frames, loop {s.loop}, "
              f"flags {sorted(s.flags)}")
        return 0
    for i in range(args.count):
        p = args.offset + FRAME * i
        if p + FRAME > len(data):
            break
        head, fl = data[p], data[p + 1]
        print(f"0x{p:08x}  shift {head & 15:2d}  filter {head >> 4}  flags 0x{fl:02x}  "
              f"{data[p + 2:p + 16].hex()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
