#!/usr/bin/env python3
"""IOP modules run in tools/eemu.py: the sound driver's synthesizer
(MODHSYN.IRX) and sequencer (MODMIDI.IRX) driven the way SNDBASE.IRX drives
them, with the SPU2 register writes they make recorded.

    tools/iopemu.py se ELF N [--ticks T]          sound effect N on port 0: the
                                                  voice registers it sets
    tools/iopemu.py se-fixture ELF [--ticks T]    every sound effect's writes
                                                  (crates/piney-audio/tests/se_fixture.txt)
    tools/iopemu.py se3d-fixture ELF              the positioned sound effects'
                                                  messages (F9 pan and bend, FD note
                                                  on and off with an id): the writes
                                                  (crates/piney-audio/tests/se3d_fixture.txt)
    tools/iopemu.py song-fixture ELF NO... [--ticks T] [--block B]
                                                  jukebox rows played from the
                                                  desktop start, hashed per B ticks
                                                  (crates/piney-audio/tests/song_fixture.txt)
    tools/iopemu.py voice ISO PATH OFS SIZE       SEWORDS.IRX streaming one voice line:
                                                  how much of it is heard
    tools/iopemu.py voice-fixture ELF ISO         event voice lines streamed
                                                  (crates/piney-audio/tests/voice_fixture.txt)

The IOP's R3000 is a subset of the EE core eemu interprets; the modules are
relocated to a base in its flat memory (IRX relocations are R_MIPS_32,
R_MIPS_26 and HI16/LO16 pairs against module offsets; bases are 64 KB
aligned so a LO16 never changes). Imports are linked by export number:
to another loaded module's export (sceSdNote2Pitch and sceSdPitch2Note go
to LIBSD.IRX's own code, which does not touch the hardware), or to a Python
stand-in here - libsd's register functions record what they are given, the
kernel's return what the modules expect.

The CSL contexts are laid out as SNDBASE.IRX's setModuleContext (0x3564)
lays them out: synthesizer ports 0-3, each an input MIDI stream buffer
(u32 buffsize, u32 validsize, bytes) and a sceHSynEnv (0x558 bytes); port 0
is fed by the EE (sound effects), ports 1-3 by sequencers 0-2.

The stand-ins make the synthesizer see ENVX and ENDX as 0 (a get returns
the last value set), so every released voice is freed on the next tick and
every waiting voice keys on at once; the port's `spu::Recorder` does the
same for the comparison.

SEWORDS.IRX (the voice lines and BGM.BIN) runs with ioman stand-ins that
read the disc image and a scheduler in WaitSema that plays the SPU2's
auto-DMA buffer (`Seword`).

Also a library: `Iop()`, `Synth(iop)`, `Seword(iso)`, `stream_buffer`, `csl_ctx`.
"""

import argparse
import os
import struct
import sys
import types

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import eemu  # noqa: E402
import irx  # noqa: E402

M32 = 0xFFFFFFFF
MODULES = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                       "work", "infection", "disc", "MODULES")
TICK_US = 4167          # SNDBASE: sceHSyn_Init / sceMidi_Init / its hard timer
HEAP = 0x01000000       # where the contexts and banks go


class Module:
    """An IRX relocated into the machine at `base`."""

    def __init__(self, m, base):
        if base & 0xFFFF:
            raise ValueError("module bases must be 64 KB aligned")
        self.irx = m
        self.base = base
        img = bytearray(m.mem)
        for sec in (".text", ".rodata", ".data"):
            for off, t, _ in m.elf.relocs(sec):
                w = struct.unpack_from("<I", img, off)[0]
                if t == 2:
                    w = (w + base) & M32
                elif t == 4:
                    w = (w & 0xFC000000) | ((((w & 0x3FFFFFF) << 2) + base) >> 2 & 0x3FFFFFF)
                elif t == 5:
                    w = (w & 0xFFFF0000) | (((w & 0xFFFF) + (base >> 16)) & 0xFFFF)
                elif t != 6:
                    raise ValueError(f"relocation type {t} at 0x{off:x}")
                struct.pack_into("<I", img, off, w)
        self.image = bytes(img)
        mod = next(s for s in m.elf.sections if s.name == ".iopmod")
        self.gp = base + struct.unpack_from("<I", m.elf.data, mod.offset + 8)[0]
        self.lib = m.exports[0] if m.exports else None
        self.exports = [base + f if f else 0 for f in m.exports[1]] if m.exports else []

    def addr(self, name_or_offset):
        if isinstance(name_or_offset, int):
            return self.base + name_or_offset
        return self.base + self.irx.lookup(name_or_offset)


class Iop(eemu.Machine):
    """eemu's interpreter with IOP modules in memory instead of the EE
    program, and lwl/lwr/swl/swr, which R3000 code uses for unaligned words."""

    def __init__(self):
        self.p = types.SimpleNamespace(gp=0)
        self.mem = bytearray(eemu.RAM)
        self.hooks = {}
        self.r = [0] * 32
        self.f = [0] * 32
        self.hi = self.lo = self.hi1 = self.lo1 = 0
        self.fcr31 = 0
        self.acc = 0
        self.steps = 0
        self.modules = {}
        self.next_base = 0x00100000
        self.heap = HEAP
        self.log = []           # (kind, entry, value) from the libsd stand-ins
        self.params = {}        # entry -> last value set, for the Get functions
        self.hle = {"libsd": LIBSD_HLE, "intrman": KERNEL_HLE, "loadcore": KERNEL_HLE,
                    "stdio": KERNEL_HLE, "thbase": KERNEL_HLE, "sysclib": SYSCLIB_HLE}
        self.real = {("libsd", 13), ("libsd", 14)}      # pure functions, run for real

    # modules ------------------------------------------------------------------
    def load_module(self, name):
        m = Module(irx.load(os.path.join(MODULES, name)), self.next_base)
        self.next_base = (m.base + len(m.image) + 0xFFFF) & ~0xFFFF
        self.mem[m.base:m.base + len(m.image)] = m.image
        self.modules[name] = m
        return m

    def link(self):
        by_lib = {m.lib: m for m in self.modules.values() if m.lib}
        for m in self.modules.values():
            for lib, stubs in m.irx.imports:
                for idx, addr, _ in stubs:
                    at = m.base + addr
                    target = by_lib.get(lib)
                    if target is not None and (lib, idx) in self.real:
                        dest = target.exports[idx]
                        self.store(at, 4, 0x08000000 | ((dest >> 2) & 0x3FFFFFF))   # j dest
                        self.store(at + 4, 4, 0)
                        continue
                    fn = self.hle.get(lib, {}).get(idx)
                    if fn is None:
                        fn = _missing(lib, idx)
                    self.hooks[at] = fn

    def alloc(self, n, align=16):
        a = (self.heap + align - 1) & ~(align - 1)
        self.heap = a + n
        self.mem[a:a + n] = bytes(n)
        return a

    def put(self, data, align=16):
        a = self.alloc(len(data), align)
        self.mem[a:a + len(data)] = data
        return a

    def call_in(self, module, addr, args=(), limit=20_000_000):
        self.p.gp = module.gp
        return self.call(addr, args, limit)

    # R3000 extras ---------------------------------------------------------------
    def exec(self, pc, w):
        op = w >> 26
        if op in (0x22, 0x26, 0x2A, 0x2E):
            rs, rt = (w >> 21) & 31, (w >> 16) & 31
            ea = (eemu.sx(self.r[rs], 32) + eemu.sx(w & 0xFFFF, 16)) & M32
            al = ea & ~3
            word = self.load(al, 4)
            k = ea & 3
            reg = self.r[rt] & M32
            if op == 0x22:      # lwl
                sh = (3 - k) * 8
                v = (reg & ((1 << sh) - 1)) | ((word << sh) & M32)
                self.set32(rt, v)
            elif op == 0x26:    # lwr
                sh = k * 8
                keep = (~(M32 >> sh)) & M32
                v = (reg & keep) | (word >> sh)
                self.set32(rt, v)
            elif op == 0x2A:    # swl
                sh = (3 - k) * 8
                mask = M32 >> sh
                self.store(al, 4, (word & ~mask & M32) | (reg >> sh))
            else:               # swr
                sh = k * 8
                mask = (M32 << sh) & M32
                self.store(al, 4, (word & ~mask & M32) | ((reg << sh) & M32))
            return None
        if op == 0x10:          # COP0: mfc0 reads 0, mtc0 is ignored
            if (w >> 21) & 31 == 0:
                self.set32((w >> 16) & 31, 0)
            return None
        return super().exec(pc, w)


def _missing(lib, idx):
    def fn(m, *a):
        raise eemu.Stop(f"no stand-in for {lib} export {idx}")
    return fn


# kernel and C library stand-ins -------------------------------------------------

def _ret0(m, *a):
    return 0


def _suspend(m, a0, *a):
    if a0:
        m.store(a0, 4, 0)
    return 0


def _printf(m, fmt, *a):
    return 0


def _get_system_time(m, a0, *a):
    if a0:
        m.store(a0, 4, 0)
        m.store(a0 + 4, 4, 0)
    return 0


KERNEL_HLE = {17: _suspend, 18: _ret0, 6: _ret0, 4: _printf, 34: _get_system_time, 40: _ret0,
              20: lambda m, *a: 1}


def _strtol(m, s, endp, base, *a):
    text = eemu._cstr(m, s).decode("latin-1")
    t = text.lstrip()
    n = 0
    i = 0
    neg = t.startswith("-")
    if t[:1] in "+-":
        i = 1
    b = base or 10
    while i < len(t) and t[i].isalnum() and int(t[i], 36) < b:
        n = n * b + int(t[i], 36)
        i += 1
    if endp:
        m.store(endp, 4, s + (len(text) - len(t)) + i)
    return (-n if neg else n) & M32


SYSCLIB_HLE = {12: eemu._memcpy, 14: eemu._memset, 23: eemu._strcpy, 20: eemu._strcat,
               27: eemu._strlen, 29: eemu._strncmp, 36: _strtol}


# libsd: record, don't emulate ---------------------------------------------------

def _set_param(m, entry, value, *a):
    entry &= 0xFFFF
    value &= 0xFFFF
    m.log.append(("param", entry, value))
    m.params[("param", entry)] = value
    return 0


def _get_param(m, entry, *a):
    return m.params.get(("param", entry & 0xFFFF), 0)


def _set_switch(m, entry, value, *a):
    entry &= 0xFFFF
    m.log.append(("switch", entry, value & M32))
    m.params[("switch", entry)] = value & M32
    return 0


def _get_switch(m, entry, *a):
    return m.params.get(("switch", entry & 0xFFFF), 0)


def _set_addr(m, entry, value, *a):
    entry &= 0xFFFF
    m.log.append(("addr", entry, value & M32))
    m.params[("addr", entry)] = value & M32
    return 0


def _get_addr(m, entry, *a):
    return m.params.get(("addr", entry & 0xFFFF), 0)


def _set_core(m, entry, value, *a):
    m.log.append(("core", entry & 0xFFFF, value & 0xFFFF))
    return 0


# sceSdProcBatch(sceSdBatch *batch, u32 *returns, u32 num): 8-byte commands
# {u16 func, u16 entry, u32 value}; func numbers from LIBSD.IRX's jump table
# at 0x3fa0 (func - 1): 1 SetParam, 2 SetSwitch, 3 SetAddr, 4 SetCoreAttr,
# 5 store entry at the IOP address value, 0x11 GetParam, 0x12 GetSwitch,
# 0x13 GetAddr, 0x14 GetCoreAttr.
BATCH = {1: _set_param, 2: _set_switch, 3: _set_addr, 4: _set_core}
BATCH_GET = {0x11: _get_param, 0x12: _get_switch, 0x13: _get_addr, 0x14: _ret0}


def _proc_batch(m, batch, returns, num, *a):
    done = 0
    for i in range(num):
        func, entry, value = struct.unpack("<HHI", m.mem[batch + 8 * i:batch + 8 * i + 8])
        if func in BATCH:
            BATCH[func](m, entry, value)
            r = 0
        elif func in BATCH_GET:
            r = BATCH_GET[func](m, entry)
        elif func == 5:
            m.store(value, 4, entry)
            r = 0
        else:
            m.log.append(("batch?", func, entry, value))
            r = 0
        if returns:
            m.store(returns + 4 * i, 4, r)
        done += 1
    return done


LIBSD_HLE = {4: _ret0, 5: _set_param, 6: _get_param, 7: _set_switch, 8: _get_switch,
             9: _set_addr, 10: _get_addr, 11: _set_core, 12: _ret0, 15: _proc_batch,
             16: _proc_batch, 17: _ret0, 18: _ret0, 19: _ret0, 20: _ret0, 23: _ret0,
             24: _ret0, 25: _ret0}


# the CSL contexts ----------------------------------------------------------------

def stream_buffer(iop, size):
    """A MIDI stream buffer: u32 buffsize, u32 validsize, then `size` bytes."""
    a = iop.alloc(8 + size + 8)
    iop.store(a, 4, size)
    iop.store(a + 4, 4, 0)
    return a


def csl_ctx(iop, groups):
    """sceCslCtx {buffGrpNum, buffGrp*, conf, callBack, extmod} with buffGrp
    [{buffNum, buffCtx*}] and buffCtx [{sema, buff}]."""
    grp = iop.alloc(8 * len(groups))
    for g, buffs in enumerate(groups):
        ctx = iop.alloc(8 * len(buffs))
        for i, b in enumerate(buffs):
            iop.store(ctx + 8 * i, 4, 0)
            iop.store(ctx + 8 * i + 4, 4, b)
        iop.store(grp + 8 * g, 4, len(buffs))
        iop.store(grp + 8 * g + 4, 4, ctx)
    ctx = iop.alloc(20)
    iop.store(ctx, 4, len(groups))
    iop.store(ctx + 4, 4, grp)
    return ctx


class Synth:
    """MODHSYN with four ports, as SNDBASE sets it up."""

    ENV = 0x558

    def __init__(self, iop, inputs=None):
        self.iop = iop
        self.mod = iop.modules.get("MODHSYN.IRX") or iop.load_module("MODHSYN.IRX")
        self.inputs = inputs or [stream_buffer(iop, 244)] + [stream_buffer(iop, 1032) for _ in range(3)]
        self.envs = [iop.alloc(self.ENV) for _ in range(4)]
        buffs = []
        for i in range(4):
            buffs += [self.inputs[i], self.envs[i]]
        self.ctx = csl_ctx(iop, [buffs])

    def init(self):
        self.iop.link()
        return self.call("sceHSyn_Init", self.ctx, TICK_US)

    def call(self, name, *args):
        return self.iop.call_in(self.mod, self.mod.addr(name), args)

    def attr(self, port, attr):
        """ccSetPortAttr (SNDBASE 0x2730)."""
        self.iop.store(self.envs[port], 1, attr & 0xFF)
        self.iop.store(self.envs[port] + 1, 1, (attr >> 8) & 0xFF)

    def load(self, port, hd, spu_addr):
        """ccSetHdSynth (SNDBASE 0x257c): clear the env's first words, then
        sceHSyn_Load(ctx, port, spuAddr, hdAddr)."""
        e = self.envs[port]
        self.iop.store(e, 1, 0)
        for off in (4, 8, 12, 16):
            self.iop.store(e + off, 4, 0)
        a = self.iop.put(hd)
        r = self.call("sceHSyn_Load", self.ctx, port, spu_addr, a)
        if r:
            raise eemu.Stop(f"sceHSyn_Load(port {port}) returned {r}")
        return a

    def volume(self, port, vol):
        return self.call("sceHSyn_SetVolume", self.ctx, port, vol)

    def send(self, port, data):
        """Append MIDI bytes to a port's input, as the EE's put_message
        (0x0012f240) and ATick's copy leave them."""
        b = self.inputs[port]
        n = self.iop.load(b + 4, 4)
        self.iop.mem[b + 8 + n:b + 8 + n + len(data)] = bytes(data)
        self.iop.store(b + 4, 4, n + len(data))

    def tick(self):
        return self.call("sceHSyn_ATick", self.ctx)


def se_bytes(se):
    """What ccSeOn (0x00179c10) sends for one seData row: program change,
    then note on."""
    prog, port, ch, note, vel = se[:5]
    return bytes((0xC0 | (ch & 0xFF), prog & 0x7F)) + bytes((0x90 | (ch & 0xFF), note & 0x7F, vel & 0x7F))


def se_setup(elf):
    """An Iop with MODHSYN set up as ccSndCommSeLoad leaves it: bank 0 on
    port 0 at SPU 0x5010, priority 0x40, 16 voices, volume 256, one tick
    run. Returns (iop, synth, seData rows)."""
    import image
    prog = image.Program(elf)
    snd_path = os.path.join(os.path.dirname(os.path.abspath(elf)), "DATA", "SNDDATA.BIN")
    with open(snd_path, "rb") as f:
        snd = f.read()
    ofs, hd_size, _ = struct.unpack_from("<3I", prog.read(prog.symbol_named("commseTbl").value, 12))
    s = prog.symbol_named("seData")
    rows = [struct.unpack_from("<6bh", prog.read(s.value + 8 * i, 8)) for i in range(s.size // 8)]
    iop = Iop()
    iop.load_module("LIBSD.IRX")
    syn = Synth(iop)
    if syn.init():
        raise eemu.Stop("sceHSyn_Init failed")
    # ccSndCommSeLoad's order: the bank (ccSetHdSynth clears the priority),
    # then the attributes, then the volume.
    syn.load(0, snd[ofs:ofs + hd_size], 0x5010)
    syn.attr(0, 0x1040)
    syn.volume(0, 256)
    syn.tick()
    iop.log.clear()
    return iop, syn, rows


def se3d_bytes(se, pan, behind, vel, note=None, vid=0):
    """What ccSeOn3D (0x00179d90), ccSeOn3DNote and ccSeOn3DLoop send for a
    seData row (docs/engine/sound.md; the EE side is checked by
    tools/test_sound3d_rs.py): program change, F9 01 ch pan 00, behind the
    camera F9 02 ch 120 63, then FD 10 ch note id velocity 00."""
    prog, _, ch, row_note = se[:4]
    ch &= 0xFF
    out = bytes((0xC0 | ch, prog & 0x7F, 0xF9, 1, ch, pan & 0xFF, 0))
    if behind:
        out += bytes((0xF9, 2, ch, 120, 63))
    k = row_note if note is None else note
    return out + bytes((0xFD, 0x10, ch, k & 0xFF, vid, vel & 0xFF, 0))


def se_off_loop_bytes(se, vid):
    """ccSeOffLoop (0x0017a620): FD 10 ch note id 00 00."""
    return bytes((0xFD, 0x10, se[2] & 0xFF, se[3] & 0xFF, vid, 0, 0))


def se3d_cases(rows):
    """(name, port volume, ticks, [(tick, bytes)]) for se3d_fixture."""
    out = []
    for n, se in enumerate(rows):
        vel = max(1, min(se[4], (n * 29) % 128))
        out.append((f"se{n}", 256, 3, [(0, se3d_bytes(se, (n * 41) % 127, n % 3 == 0, vel))]))
    for n in (4, 21, 44, 100):
        for pan in (0, 1, 31, 62, 63, 64, 95, 125, 126):
            for behind in (0, 1):
                out.append((f"pan{n}-{pan}-{behind}", 256, 2, [(0, se3d_bytes(rows[n], pan, behind, rows[n][4]))]))
    for n in (4, 100):
        for pan in (0, 63, 126):
            out.append((f"vol{n}-{pan}", 128, 2, [(0, se3d_bytes(rows[n], pan, 0, 90))]))
    for n in (4, 44):
        for k in (36, 60, 90, 127):
            out.append((f"note{n}-{k}", 256, 2, [(0, se3d_bytes(rows[n], 40, 0, 100, note=k))]))
    # Loops: the note on with an id, its note off a tick later (and one with
    # another id, which ends nothing), and two loops of one row at once.
    for n in (230, 4, 21):
        se = rows[n]
        for vid in (0, 3, 7):
            on = se3d_bytes(se, 20 + vid, 0, 100, vid=vid)
            out.append((f"loop{n}-{vid}", 256, 4, [(0, on), (2, se_off_loop_bytes(se, vid))]))
            out.append((f"loopwrong{n}-{vid}", 256, 4, [(0, on), (2, se_off_loop_bytes(se, (vid + 1) & 7))]))
        both = se3d_bytes(se, 0, 0, 100, vid=0) + se3d_bytes(se, 126, 1, 80, vid=1)
        out.append((f"loops{n}", 256, 4, [(0, both), (2, se_off_loop_bytes(se, 1))]))
    # An override with no note on after it waits for the channel's next one
    # (a plain ccSeOn); two positioned effects in one tick.
    for n in (4, 44):
        se = rows[n]
        f9 = bytes((0xF9, 1, se[2] & 0xFF, 5, 0, 0xF9, 2, se[2] & 0xFF, 120, 63))
        out.append((f"carry{n}", 256, 3, [(0, f9), (1, se_bytes(se))]))
    for a, b in ((4, 21), (44, 42), (16, 17), (100, 4)):
        both = se3d_bytes(rows[a], 0, 1, 110) + se3d_bytes(rows[b], 126, 0, 70)
        out.append((f"two{a}-{b}", 256, 3, [(0, both)]))
    return out


def se3d_fixture(elf):
    iop, syn, rows = se_setup(elf)
    snap = snapshot(iop)
    print("# tools/iopemu.py se3d-fixture: MODHSYN.IRX run in eemu, port 0 with the common SE bank,")
    print("# given what the positioned sound effects send.")
    print("# case NAME PORTVOL TICKS TICK:BYTES,...   (the bytes sent before that tick, in hex)")
    print("# w NAME TICK WRITES...                    (kind:entry:value in hex; p param, a addr, s switch)")
    for name, vol, ticks, sends in se3d_cases(rows):
        restore(iop, snap)
        if vol != 256:
            syn.volume(0, vol)
            iop.log.clear()
        print(f"case {name} {vol} {ticks} " + ",".join(f"{t}:{b.hex()}" for t, b in sends))
        for t in range(ticks):
            for when, data in sends:
                if when == t:
                    syn.send(0, data)
            syn.tick()
            print(f"w {name} {t} " + " ".join(fmt_write(e) for e in iop.log))
            iop.log.clear()
    return 0


def snapshot(iop):
    return bytes(iop.mem), dict(iop.params)


def restore(iop, snap):
    iop.mem[:] = snap[0]
    iop.params = dict(snap[1])
    iop.log.clear()


def fmt_write(e):
    kind = {"param": "p", "addr": "a", "switch": "s", "core": "c"}.get(e[0], e[0])
    return f"{kind}:{e[1]:x}:{e[2]:x}"


def wave_bank(elf, no):
    """The SQ_LOAD row and SQTBLs `Wave[no]` names, from Infection's sound
    tables as the port keeps them (tools/portdata.py); `elf` is unused."""
    import portdata
    t = portdata.sound("INF")
    cat, ftype, bgnum, _ = t["wave"][no]
    names = {1: "desktop", 2: "town", 4: "dungeon", 5: "event", 6: "stream", 7: "title"}
    ctx = t["field"][ftype] if cat == 3 else t[names[cat]]
    return ctx["load"][bgnum], ctx["vol"][bgnum], t["commse"]


def song_fixture(elf, nos, ticks, block):
    """The desktop starting with `dtBgm = no`, the synthesizer side: the SE
    bank on port 0 (ccSndCommSeLoad's order), then as ccSQDataLoadCD and
    ccSndChangeData leave it - every port reset, notes and sound off,
    volume 0; the bank given to port i + 1 (attributes 0x2010) for each
    sequence; port volumes 256 and the SQTBL volumes; then sequence 0 (1 for
    rows 27 and 7) played by tools/midi.py - which matches MODMIDI.IRX byte
    for byte - into its port, and every tick's register writes hashed."""
    import midi
    with open(os.path.join(os.path.dirname(os.path.abspath(elf)), "DATA", "SNDDATA.BIN"), "rb") as f:
        snd = f.read()
    print(f"# tools/iopemu.py song-fixture: MODHSYN.IRX run in eemu playing a jukebox row "
          f"from the desktop start, {ticks} ticks.")
    print(f"# song NO FIRST_TICK WRITES FNV   (per {block} ticks; FNV-1a 64 over "
          f"'kind:entry:value;' in hex, a tick ending in '|')")
    for no in nos:
        row, vols, commse = wave_bank(elf, no)
        ofs, hd_size, s1, s2, s3, _ = row
        iop, syn, _ = se_setup(elf)
        for port in range(4):
            syn.call("sceHSyn_ResetAllControler", syn.ctx, port)
        for port in range(4):
            syn.call("sceHSyn_ResetAllControler", syn.ctx, port)
            syn.call("sceHSyn_AllNoteOff", syn.ctx, port)
            syn.call("sceHSyn_AllSoundOff", syn.ctx, port)
            syn.call("sceHSyn_SetVolume", syn.ctx, port, 0)
        for port in range(4):
            syn.call("sceHSyn_AllSoundOff", syn.ctx, port)
        hd = snd[ofs:ofs + hd_size]
        spu = 0x5020 + commse[2]
        sizes = (s1, s2, s3)
        seqs = [0] + [i for i in (1, 2) if sizes[i]]
        for i in seqs:
            syn.load(i + 1, hd, spu)
            syn.attr(i + 1, 0x2010)
        syn.volume(0, 256)
        for i in range(3):
            syn.volume(i + 1, vols[i][2])
        seq = 1 if no in (27, 7) else 0
        syn.volume(vols[seq][1], vols[seq][2])
        at = ofs + hd_size + sum(sizes[:seq])
        m = midi.Midi(snd[at:at + 0x20000], port=seq)
        iop.log.clear()
        h = FNV_OFFSET
        n = 0
        for t in range(ticks):
            for p, data in m.atick():
                syn.send(p + 1, data)
            syn.tick()
            for e in iop.log:
                h = fnv((fmt_write(e) + ";").encode(), h)
                n += 1
            h = fnv(b"|", h)
            iop.log.clear()
            if (t + 1) % block == 0:
                print(f"song {no} {t + 1 - block} {n} {h:016x}")
                h = FNV_OFFSET
                n = 0
    return 0


FNV_OFFSET = 0xCBF29CE484222325


def fnv(data, h=FNV_OFFSET):
    for b in data:
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


# SEWORDS.IRX: the voice lines ------------------------------------------------------

class _Done(Exception):
    pass


class Seword:
    """SEWORDS.IRX's channel 0 streaming a voice line, the module's own code
    run: bgmFunc takes the EE's commands (0x130 DVD, 0x140 ring buffer, then
    0x80e0 with evVoicePlay's vBank), and the streaming thread _BgmPlay is
    entered once and driven by the WaitSema stand-in, which plays the SPU
    buffer the way the SPU2's auto-DMA does: the half that is transferring
    plays out, then the transfer interrupt (gBgmIntr[0] = 1) and the thread
    refills it while the other half plays. sceSdBlockTransStatus reports the
    half transferring in bit 24. What is heard is each half as it starts to
    play, until the thread stops the transfer.

    The disc is the image, read through ioman stand-ins (open, lseek, read,
    close); a read of a negative size returns 0 - a guess, reached only by
    lines shorter than 16,384 bytes, of which the event tables have none.
    """

    ALLOC = 0xC000                  # BgmInit(ch, 0xc000) from wordPlay
    HALF = ALLOC // 3               # gSPacketSize, mono

    def __init__(self, iso_path):
        import iso
        self.img = iso.Iso(iso_path)
        self.iop = Iop()
        self.mod = self.iop.load_module("SEWORDS.IRX")
        sd = dict(LIBSD_HLE)
        sd.update({5: self._set_param, 18: self._block_trans, 20: self._trans_status, 26: _ret0})
        self.iop.hle = {
            "libsd": sd,
            "intrman": KERNEL_HLE,
            "stdio": {4: _printf},
            "sysclib": SYSCLIB_HLE,
            "sysmem": {4: self._alloc, 5: _ret0, 7: lambda m, *a: 0x100000, 8: lambda m, *a: 0x100000},
            "ioman": {4: self._open, 5: _ret0, 6: self._read, 8: self._lseek, 31: _ret0},
            "thbase": {4: lambda m, *a: 1, 5: _ret0, 6: _ret0, 10: _ret0, 20: lambda m, *a: 1, 24: _ret0},
            "thsemap": {4: lambda m, *a: 1, 5: _ret0, 7: _ret0, 8: self._wait_sema},
            "sifman": {5: _ret0, 29: lambda m, *a: 1},
            "sifcmd": {14: _ret0},
        }
        self.iop.link()
        # The module's start: gFd[] = -1, tryCount 16, the RPC threads made.
        self.iop.call_in(self.mod, self.mod.addr("start"), (0, 0))
        self.arg = self.iop.alloc(64)
        self.intr = self.mod.addr("gBgmIntr")
        self.snap = None
        self.file = b""
        self.fp = 0
        # The EE's start-up commands (ccSoundMain): the drive is a DVD, the
        # reads go through the ring buffer.
        self.command(0x130, (2,))
        self.command(0x140, (1,))
        self.snap = bytes(self.iop.mem)

    def command(self, cmd, words=(), name=b""):
        """bgmFunc(cmd, data, 64) as the RPC server calls it; the returned
        word (ret)."""
        m = self.iop
        m.mem[self.arg:self.arg + 64] = bytes(64)
        for i, w in enumerate(words):
            m.store(self.arg + 4 * i, 4, w & M32)
        m.mem[self.arg + 16:self.arg + 16 + len(name)] = name
        at = self.iop.call_in(self.mod, self.mod.addr("bgmFunc"), (cmd, self.arg, 64))
        return m.load(at, 4)

    # stand-ins ---------------------------------------------------------------------
    def _alloc(self, m, kind, size, *a):
        return m.alloc(size)

    def _open(self, m, name, mode, *a):
        path = eemu._cstr(m, name).decode("latin-1").split(":", 1)[-1]
        path = path.lstrip("\\").replace("\\", "/")
        try:
            e = self.img.find(path)
        except KeyError:
            return M32      # -1
        self.file = self.img.read(e.lba, e.size)
        self.fp = 0
        return 3

    def _lseek(self, m, fd, ofs, whence, *a):
        self.fp = eemu.sx(ofs, 32)
        return ofs

    def _read(self, m, fd, buf, n, *a):
        n = eemu.sx(n, 32)
        if n <= 0:
            return 0
        got = self.file[self.fp:self.fp + n]
        m.mem[buf:buf + len(got)] = got
        self.fp += len(got)
        return len(got)

    def _set_param(self, m, entry, value, *a):
        if entry & 0xFFFF in (0x0F80, 0x1080):          # BVOLL, BVOLR of core 0
            self.bvol.append(value & 0xFFFF)
        return 0

    def _block_trans(self, m, ch, mode, addr, size, *a):
        mode &= 0xFFFF
        if mode == 0x10 and ch == 0:        # write, loop: the stream starts
            self.spu = addr
            self.cur = 0
            self.started = True
        elif mode & 0xF == 2 and self.started:
            self.stopped = True
        return 0

    def _trans_status(self, m, ch, flag, *a):
        return (self.cur << 24) | ((self.spu + self.cur * self.HALF) & 0xFFFFFF)

    def _wait_sema(self, m, sema, *a):
        if self.stopped or not self.started or len(self.halves) >= self.limit:
            raise _Done()
        at = self.spu + self.cur * self.HALF
        self.halves.append(bytes(m.mem[at:at + self.HALF]))
        self.cur ^= 1
        m.store(self.intr, 4, 1)            # IntFunc(0, &gSem)
        return 0

    # a line ------------------------------------------------------------------------
    def play(self, path, ofs, size, vol=0x6FFF6FFF, limit=4000):
        """Stream one line; (samples heard as mono, BVOL writes). The halves
        are checked to hold the same samples left and right."""
        self.iop.mem[:] = self.snap
        self.iop.heap = HEAP + 0x100000
        self.started = self.stopped = False
        self.halves = []
        self.bvol = []
        self.limit = limit
        self.cur = 0
        self.spu = 0
        name = ("cdrom0:\\" + path.replace("/", "\\")).encode()
        ret = self.command(0x80E0, (ofs, size, 0, vol), name)
        if not self.started:
            return None, self.bvol, ret
        try:
            self.iop.call_in(self.mod, self.mod.addr("_BgmPlay"), (), limit=200_000_000)
        except _Done:
            pass
        out = bytearray()
        for h in self.halves:
            for b in range(0, self.HALF, 1024):
                left, right = h[b:b + 512], h[b + 512:b + 1024]
                if left != right:
                    raise ValueError(f"{path}+{ofs}: left and right differ")
                out += left
        return bytes(out), self.bvol, ret


def voice_lines(elf):
    """(path, ofs, size, label) for the event voice lines the fixture plays:
    every line of event 1 in both languages, every 12th line of the four
    volume 1 tables, the shortest and the longest, then made-up sizes at the
    offset of event 1's message 1 (Japanese) for the paths no event line
    takes - lines of 16,384 to 49,154 bytes."""
    import image
    p = image.Program(elf)
    files = {6: ("VOICE/EVVOL1.BIN", "VOICE_E/EVVOL1_E.BIN"), 7: ("VOICE/EVVOL1S.BIN", "VOICE_E/EVVOL1SE.BIN")}
    rows = []
    for table, fi, lang in (("evVoiceDataVol1M", 6, 0), ("evVoiceDataVol1S", 7, 0),
                            ("evVoiceDataVol1ME", 6, 1), ("evVoiceDataVol1SE", 7, 1)):
        s = p.symbol_named(table)
        for i in range(50):
            x = p.u32(s.value + 4 * i)
            if not x:
                continue
            n = p.symbol_named(p.name_at(x)).size // 8
            ev = i + (0 if fi == 6 else 50)
            for k in range(n):
                ofs, size = struct.unpack("<ii", p.read(x + 8 * k, 8))
                if ofs >= 0:
                    rows.append((files[fi][lang], ofs, size, f"{ev}/{k}/{'EJ'[lang == 0]}"))
    pick = [r for r in rows if r[3].startswith("1/")]
    pick += rows[::12]
    pick.append(min(rows, key=lambda r: r[2]))
    pick.append(max(rows, key=lambda r: r[2]))
    base = next(r for r in rows if r[3] == "1/1/J")
    for size in (16384, 16386, 20000, 24576, 24578, 30000, 32766, 32768, 32770, 40960, 40962, 49152, 49154):
        pick.append((base[0], base[1], size, "made-up"))
    seen = set()
    out = []
    for r in pick:
        if r[:3] not in seen:
            seen.add(r[:3])
            out.append(r)
    return out


def voice_fixture(elf, iso_path):
    sw = Seword(iso_path)
    print("# tools/iopemu.py voice-fixture: SEWORDS.IRX run in eemu streaming voice lines "
          "(evVoicePlay's 0x80e0, volume 0x6fff6fff).")
    print("# line PATH OFS SIZE LABEL | SAMPLES FNV BVOL...   (the samples heard, one channel "
          "- both are equal; FNV-1a 64 of them as 16-bit LE; each BVOLL/BVOLR write)")
    for path, ofs, size, label in voice_lines(elf):
        pcm, bvol, _ = sw.play(path, ofs, size)
        head = f"line {path} {ofs} {size} {label}"
        if pcm is None:
            print(f"{head} | none")
            continue
        print(f"{head} | {len(pcm) // 2} {fnv(pcm):016x} " + " ".join(f"{v:x}" for v in bvol))
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("se")
    p.add_argument("elf")
    p.add_argument("n", type=int)
    p.add_argument("--ticks", type=int, default=2)
    p = sub.add_parser("se-fixture")
    p.add_argument("elf")
    p.add_argument("--ticks", type=int, default=2)
    p = sub.add_parser("se3d-fixture")
    p.add_argument("elf")
    p = sub.add_parser("song-fixture")
    p.add_argument("elf")
    p.add_argument("nos", type=int, nargs="+")
    p.add_argument("--ticks", type=int, default=2400)
    p.add_argument("--block", type=int, default=100)
    p = sub.add_parser("voice")
    p.add_argument("iso")
    p.add_argument("path")
    p.add_argument("ofs", type=int)
    p.add_argument("size", type=int)
    p = sub.add_parser("voice-fixture")
    p.add_argument("elf")
    p.add_argument("iso")
    args = parser.parse_args()

    if args.cmd == "song-fixture":
        return song_fixture(args.elf, args.nos, args.ticks, args.block)
    if args.cmd == "se3d-fixture":
        return se3d_fixture(args.elf)
    if args.cmd == "voice-fixture":
        return voice_fixture(args.elf, args.iso)
    if args.cmd == "voice":
        pcm, bvol, ret = Seword(args.iso).play(args.path, args.ofs, args.size)
        if pcm is None:
            print(f"wordPlay returned 0x{ret:x}; nothing played")
            return 1
        n = len(pcm) // 2
        print(f"wordPlay returned {ret}; {n} samples heard ({n / 48000:.3f} s) of {args.size // 2}; "
              f"BVOL writes {' '.join(hex(v) for v in bvol)}")
        return 0
    iop, syn, rows = se_setup(args.elf)
    if args.cmd == "se":
        se = rows[args.n]
        print(f"seData[{args.n}] = {se}")
        syn.send(0, se_bytes(se))
        for t in range(args.ticks):
            syn.tick()
            print(f"tick {t}: {iop.steps} steps")
            for e in iop.log:
                print("   ", e)
            iop.log.clear()
        return 0
    # Every sound effect from a fresh synthesizer, then a few with the SE
    # port at other volumes, then two at once: the writes of each tick.
    snap = snapshot(iop)
    cases = [(n, 256, None) for n in range(len(rows))]
    cases += [(n, v, None) for n in (0, 4, 21, 44, 100, 200) for v in (0, 64, 128, 200)]
    cases += [(n, 256, m) for n, m in ((4, 21), (0, 0), (44, 42), (16, 17))]
    print("# tools/iopemu.py se-fixture: MODHSYN.IRX run in eemu, port 0 with the common SE bank.")
    print("# se N PORTVOL SECOND TICK WRITES...   (kind:entry:value in hex; p param, a addr, s switch)")
    for n, vol, second in cases:
        restore(iop, snap)
        if vol != 256:
            syn.volume(0, vol)
            iop.log.clear()
        syn.send(0, se_bytes(rows[n]))
        if second is not None:
            syn.send(0, se_bytes(rows[second]))
        for t in range(args.ticks):
            syn.tick()
            print(f"se {n} {vol} {-1 if second is None else second} {t} "
                  + " ".join(fmt_write(e) for e in iop.log))
            iop.log.clear()
    return 0


if __name__ == "__main__":
    sys.exit(main())
