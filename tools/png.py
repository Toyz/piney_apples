#!/usr/bin/env python3
"""A PNG writer, nothing else: 8-bit RGBA, filter 0, one IDAT.

    import png; png.write_rgba(path, width, height, rgba_bytes)
"""

import struct
import zlib


def _chunk(kind, body):
    return (struct.pack(">I", len(body)) + kind + body
            + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF))


def encode_rgba(width, height, rgba):
    if len(rgba) != width * height * 4:
        raise ValueError(f"{len(rgba)} bytes for {width}x{height} RGBA")
    stride = width * 4
    raw = bytearray()
    for y in range(height):
        raw.append(0)   # filter type 0 on every row
        raw += rgba[y * stride:(y + 1) * stride]
    return (b"\x89PNG\r\n\x1a\n"
            + _chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + _chunk(b"IDAT", zlib.compress(bytes(raw), 9))
            + _chunk(b"IEND", b""))


def write_rgba(path, width, height, rgba):
    with open(path, "wb") as f:
        f.write(encode_rgba(width, height, rgba))
