#!/usr/bin/env python3
"""A WAV writer, nothing else: 16-bit PCM, any rate and channel count, and an
optional `smpl` chunk carrying one forward loop.

    import wav; wav.write(path, pcm, rate, channels=1, loop=None)

`pcm` is little-endian signed 16-bit, interleaved when there is more than one
channel - bytes, or an array('h') / list of ints. `loop` is (start, end) in
sample frames, end exclusive; the smpl chunk stores the end inclusively, as
the format wants. The standard library's `wave` module cannot write a smpl
chunk, which is why the RIFF is put together by hand.
"""

import array
import struct
import sys


def _pcm_bytes(pcm):
    if isinstance(pcm, (bytes, bytearray, memoryview)):
        return bytes(pcm)
    a = pcm if isinstance(pcm, array.array) and pcm.typecode == "h" else array.array("h", pcm)
    if sys.byteorder != "little":
        a = array.array("h", a)
        a.byteswap()
    return a.tobytes()


def smpl_chunk(rate, loop):
    start, end = loop
    period = round(1e9 / rate)   # nanoseconds per sample
    body = struct.pack("<9I", 0, 0, period, 60, 0, 0, 0, 1, 0)
    body += struct.pack("<6I", 0, 0, start, end - 1, 0, 0)   # cue id, forward, inclusive end
    return b"smpl" + struct.pack("<I", len(body)) + body


def encode(pcm, rate, channels=1, loop=None):
    data = _pcm_bytes(pcm)
    if len(data) % (2 * channels):
        raise ValueError(f"{len(data)} bytes is not a whole number of {channels}-channel frames")
    fmt = struct.pack("<HHIIHH", 1, channels, rate, rate * 2 * channels, 2 * channels, 16)
    body = b"WAVE" + b"fmt " + struct.pack("<I", len(fmt)) + fmt
    if loop is not None:
        body += smpl_chunk(rate, loop)
    body += b"data" + struct.pack("<I", len(data)) + data
    if len(data) & 1:
        body += b"\0"
    return b"RIFF" + struct.pack("<I", len(body)) + body


def write(path, pcm, rate, channels=1, loop=None):
    with open(path, "wb") as f:
        f.write(encode(pcm, rate, channels, loop))


def read(path):
    """(rate, channels, pcm bytes, loop or None) - enough to check what write made."""
    with open(path, "rb") as f:
        d = f.read()
    if d[:4] != b"RIFF" or d[8:12] != b"WAVE":
        raise ValueError(f"{path}: not a RIFF WAVE file")
    p = 12
    rate = channels = None
    pcm = b""
    loop = None
    while p + 8 <= len(d):
        kind, n = d[p:p + 4], struct.unpack_from("<I", d, p + 4)[0]
        body = d[p + 8:p + 8 + n]
        if kind == b"fmt ":
            _, channels, rate = struct.unpack_from("<HHI", body)
        elif kind == b"data":
            pcm = body
        elif kind == b"smpl" and struct.unpack_from("<I", body, 28)[0]:
            s, e = struct.unpack_from("<II", body, 36 + 8)
            loop = (s, e + 1)
        p += 8 + n + (n & 1)
    return rate, channels, pcm, loop
