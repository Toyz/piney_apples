#!/usr/bin/env python3
"""Every store that can reach a ccDrawEnv's +0x88..+0x8b on a disc.

A ccDrawEnv pointer reaches code in these ways only: the globals cc3d and
active__9ccDrawEnv (loaded gp-relative or through a lui-built address), a
ccStream's own environment (+0x158), `this` in ccDrawEnv's methods, the
environment ccStream::InitScene is given, and the object __ct__9ccDrawEnv
builds. From each, the register is followed forward (moves copy it; any
other write or a call ends it) for 128 instructions, and every store whose
bytes cover +0x88 through it is listed. Code: main and the four overlays
with their static initialisers, as tools/refscan.py reads them.

    tools/envscan.py VOLUME...
"""
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
from image import Overlay, Program  # noqa: E402

EXE = {"infection": "SLUS_202.67", "mutation": "SLUS_205.62", "outbreak": "SLUS_205.63",
       "quarantine": "SLUS_205.64"}
STORES = {0x28: 1, 0x29: 2, 0x2B: 4, 0x3F: 8, 0x1F: 16, 0x39: 4, 0x3D: 8, 0x3E: 16, 0x2A: 4, 0x2E: 4, 0x2C: 8,
          0x2D: 8}
LOADS = {0x20, 0x21, 0x23, 0x24, 0x25, 0x27, 0x37, 0x1E, 0x31, 0x35, 0x22, 0x26, 0x1A, 0x1B}


def sext(v):
    return v - 0x10000 if v & 0x8000 else v


def dest(w):
    op, fn = w >> 26, w & 63
    if op in (0, 0x1C):
        if op == 0 and fn in (0x08, 0x0C, 0x0D, 0x0F, 0x18, 0x19, 0x1A, 0x1B, 0x11, 0x13):
            return None
        return (w >> 11) & 31
    if op in (0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x14, 0x15, 0x16, 0x17, 0x01, 0x10, 0x11, 0x12) or op in STORES:
        return None
    return (w >> 16) & 31


def scan(words, base, gp, globals_, starts, label, out):
    """`starts`: {address: [registers]} holding an environment there."""
    n = len(words)
    lui = {}
    for i, w in enumerate(words):
        pc = base + 4 * i
        op, rs, rt, imm = w >> 26, (w >> 21) & 31, (w >> 16) & 31, w & 0xFFFF
        seeds = list(starts.get(pc, []))
        if op == 0x0F:
            lui[rt] = (imm << 16, i)
        elif op in (0x09, 0x19, 0x0D) and rs in lui and i - lui[rs][1] < 8:
            a = lui[rs][0] + (sext(imm) if op != 0x0D else imm)
            lui[rt] = (a & 0xFFFFFFFF, i)
        if op in LOADS and op in (0x23, 0x37, 0x27):
            a = None
            if rs == 28:
                a = (gp + sext(imm)) & 0xFFFFFFFF
            elif rs in lui and i - lui[rs][1] < 8:
                a = (lui[rs][0] + sext(imm)) & 0xFFFFFFFF
            if a in globals_ or sext(imm) == 0x158:
                seeds.append((rt, i + 1))
        if op == 0x03:
            t = ((pc + 4) & 0xF0000000) | ((w & 0x3FFFFFF) << 2)
            if t == globals_.get("ctor"):
                seeds.append((4, i))
        for reg, at in [s if isinstance(s, tuple) else (s, i) for s in seeds]:
            follow(words, base, at, {reg}, label, out)


def follow(words, base, i0, regs, label, out):
    for j in range(i0, min(len(words), i0 + 128)):
        w = words[j]
        op, rs, rt, imm = w >> 26, (w >> 21) & 31, (w >> 16) & 31, w & 0xFFFF
        if op in STORES and rs in regs:
            lo, hi = sext(imm), sext(imm) + STORES[op]
            if lo <= 0x8B and hi > 0x88:
                out.append((label, base + 4 * j, w))
        if op == 0 and (w & 63) in (0x21, 0x2D, 0x25) and ((w >> 16) & 31) == 0 and rs in regs:
            regs = regs | {(w >> 11) & 31}
            continue
        if op == 0x03 or (op == 0 and (w & 63) == 0x09):
            regs = regs - set(range(1, 16)) - {24, 25, 31}
        d = dest(w)
        if d is not None and d in regs:
            regs = regs - {d}
        if not regs:
            return


def main(vol):
    disc = os.path.join(ROOT, "work", vol, "disc")
    p = Program(os.path.join(disc, EXE[vol]))
    sym = lambda n: p.symbol_named(n).value  # noqa: E731
    globals_ = {sym("cc3d"): 1, sym("active__9ccDrawEnv"): 1, "ctor": sym("__ct__9ccDrawEnvFv")}
    starts = {}
    for name in ("__ct__9ccDrawEnvFv", "Reset__9ccDrawEnvFv", "SetMipmap__9ccDrawEnvFii",
                 "SetMipmapMode__9ccDrawEnvFii", "SetLightNormal__9ccDrawEnvFPfPfPf",
                 "SetLightColor__9ccDrawEnvFPfPfPfPf", "SetAmbient__9ccDrawEnvFPf", "SetAmbient__9ccDrawEnvFUi",
                 "GetAmbient__9ccDrawEnvFv", "GetAmbient__9ccDrawEnvFPf", "SetFogBlend__9ccDrawEnvFfUi",
                 "ResetFogBlend__9ccDrawEnvFv", "SetFog__9ccDrawEnvFffffUi", "SetFog__9ccDrawEnvFffUi",
                 "SetLightMatrix__9ccDrawEnvFPA4_fPA4_fPf"):
        starts.setdefault(sym(name), []).append(4)
    starts.setdefault(sym("InitScene__8ccStreamFP7ccLayerP9ccDrawEnv"), []).append(6)
    out = []
    m = p.main_seg
    b = p.elf.read(m.vaddr, m.filesz)
    scan(list(struct.unpack_from("<%dI" % (len(b) // 4), b)), m.vaddr, p.gp, globals_, starts, "main", out)
    for name in ("GCMN", "DEMO", "DESKTOP", "TOPPAGE"):
        ov = Overlay(os.path.join(disc, "DATA", name + ".PRG"))
        t = ov.read(ov.text, ov.text_size)
        scan(list(struct.unpack_from("<%dI" % (len(t) // 4), t)), ov.text, p.gp, globals_, {}, name, out)
        ctors = [struct.unpack("<I", ov.read(a, 4))[0] for a in range(ov.ctor_start, ov.ctor_end, 4)]
        if ctors:
            init = ov.read(min(ctors), ov.ctor_start - min(ctors))
            scan(list(struct.unpack_from("<%dI" % (len(init) // 4), init)), min(ctors), p.gp, globals_, {},
                 name + " ctors", out)
    for label, at, w in sorted(set(out)):
        f = p.function_at(at) if label == "main" else None
        print(f"{vol} {label:13s} 0x{at:08x} {w:08x} {f.name if f else ''}")
    print(f"# {vol}: {len(set(out))} stores reach a ccDrawEnv's +0x88")


if __name__ == "__main__":
    for v in sys.argv[1:]:
        main(v)
