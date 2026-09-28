#!/usr/bin/env python3
"""The memory-card save, as each volume's own code writes and reads it.

    tools/save.py card  ELF [--out DIR] [--slot N]   make the save directory and
                                                     save one slot on a blank card
    tools/save.py carry ELF [--prev PREV_ELF] [--slot N]
                                                     load a previous volume's save
                                                     the way the title screen does

Nothing about the layout is taken from here: the executable's ccSaveSys and
ccMcard code runs in tools/eemu.py against a memory card kept in Python (the
sceMc* library calls are answered from a dict of files), so the paths, sizes,
checksums and copies are whatever that volume's code does. Only the protocol
around the code is supplied: the request functions (SaveDataReq,
LoadInfoPrevReq ...), MainProccess, NextProccess(1) to answer "yes" when it
asks, ccMcard::CheckPort answering "card present" (-1), and \\DATA\\ICON.BIN
read from the extracted disc for MakeDir.

`card` runs ccMcard::MakeDir, then SaveDataReq(slot), and lists every file it
wrote, with the icon.sys fields. `carry` asks for the previous volume's slot
index (LoadInfoPrevReq) and data (LoadDataPrevReq), learning from the card
calls which paths and sizes the code wants; the data file is filled with
three byte patterns, so every byte of ccSaveData (and, from Mutation on, of
the save extension) can be traced to the file offset it came from. It then
runs ccStartEventConvert and ccSaveData::ConvGame, as ccThDemo (demo.prg)
does when the title screen's load returns 4, and reports what they change.
With --prev, the previous volume's own code writes the save first (its
`card`), and this volume loads that file.
"""

import argparse
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mips  # noqa: E402
from image import Program  # noqa: E402

# Scratch memory for the objects the game allocates at start-up.
SYS = 0x01800000        # ccSaveSys (0x2b8 bytes)
SAVE = 0x01810000       # ccSaveData
EXT = 0x01820000        # the save extension (from Mutation on)
MC = 0x01830000         # ccMcard (0x42c bytes)
GAME = 0x01840000       # ccGame
SND = 0x01850000        # ccSnd
HEAP = 0x01900000       # ccMalloc / new
SLOT_INFO = 28          # ccSaveDataInfo; SaveSys/ReadSys move twelve of them
INFO = {"status": 0x00, "level": 0x01, "clearFlag": 0x02, "parodyFlag": 0x03,
        "name": 0x04, "sum": 0x16, "playtime": 0x18}
MC_CREAT = 0x200        # sceMcOpen mode bit
MC_NO_ENTRY = -4        # sceMcResNoEntry


class Card:
    """A memory card for the sceMc* calls: files by path, one pending result
    that sceMcSync reports, as the library's asynchronous calls do."""

    def __init__(self, files=None):
        self.files = {k: bytearray(v) for k, v in (files or {}).items()}
        self.dirs = {os.path.dirname(k) for k in self.files}
        self.fds = {}
        self.pending = 0
        self.log = []          # (call, path, size) in order

    def hooks(self, p):
        def name(m, va):
            b = bytes(m.mem[va:va + 256])
            return b[:b.index(b"\0")].decode("latin-1")

        def mc_open(m, port, slot, path, mode):
            path = name(m, path)
            if path not in self.files:
                if not mode & MC_CREAT:
                    self.log.append(("open-missing", path, 0))
                    self.pending = MC_NO_ENTRY
                    return 0
                self.files[path] = bytearray()
            fd = len(self.fds) + 3
            self.fds[fd] = [path, 0]
            self.pending = fd
            return 0

        def mc_read(m, fd, buf, size, *_):
            path, pos = self.fds[fd]
            data = self.files[path][pos:pos + size]
            m.mem[buf:buf + len(data)] = data
            self.fds[fd][1] = pos + len(data)
            self.log.append(("read", path, size))
            self.pending = len(data)
            return 0

        def mc_write(m, fd, buf, size, *_):
            path, pos = self.fds[fd]
            f = self.files[path]
            f[pos:pos + size] = bytes(m.mem[buf:buf + size])
            self.fds[fd][1] = pos + size
            self.log.append(("write", path, size))
            self.pending = size
            return 0

        def mc_close(m, fd, *_):
            self.fds.pop(fd, None)
            self.pending = 0
            return 0

        def mc_mkdir(m, port, slot, path, *_):
            path = name(m, path)
            self.pending = MC_NO_ENTRY if path in self.dirs else 0
            self.dirs.add(path)
            self.log.append(("mkdir", path, 0))
            return 0

        def mc_flush(m, *_):
            self.pending = 0
            return 0

        def mc_sync(m, mode, cmd, result, *_):
            if result:
                m.store(result, 4, self.pending)
            return 1

        return {"sceMcOpen": mc_open, "sceMcRead": mc_read, "sceMcWrite": mc_write,
                "sceMcClose": mc_close, "sceMcMkdir": mc_mkdir, "sceMcFlush": mc_flush,
                "sceMcSync": mc_sync}


def find_extension(p):
    """(global va, size) of the object ccSaveData's constructor allocates
    beside it - `li $a0, SIZE; jal new; ... sw $v0, OFS($gp)` - or None.
    Infection's constructor has no such allocation."""
    ct = p.symbol_named("__ct__10ccSaveDataFv")
    new = p.symbol_named("__nw__FUi")
    if ct is None or new is None:
        return None
    size = None
    for i in range(ct.size // 4 if ct.size else 64):
        va = ct.value + 4 * i
        ins = mips.decode(p.u32(va), va)
        w = ins.word
        if w >> 26 == 0x09 and ins.rs == 0 and ins.rt == 4:          # li $a0, SIZE
            size = w & 0xffff
        elif ins.call and ins.target == new.value and size:
            for j in range(i + 1, i + 12):
                w2 = p.u32(ct.value + 4 * j)
                if w2 >> 26 == 0x2b and (w2 >> 21) & 31 == 28 and (w2 >> 16) & 31 == 2:
                    return (p.gp + ((w2 & 0xffff) ^ 0x8000) - 0x8000) & 0xffffffff, size
        elif ins.mnemonic == "jr":
            break
    return None


class Game:
    """One executable's save code in tools/eemu.py, with the demo overlay
    loaded (ConvGame reads charTbl there in Infection and Mutation)."""

    def __init__(self, elf, card=None):
        import eemu
        self.eemu = eemu
        self.elf = elf
        self.p = p = Program(elf, "demo")
        self.m = m = eemu.Machine(p)
        self.card = card or Card()
        self.volume = p.u32(self.sym("volumeNum"))
        self.ext = find_extension(p)          # (global, size) or None
        self.save_size = self.object_size()
        for n, v in (("saveSys", SYS), ("saveData", SAVE), ("ccMc", MC), ("game", GAME),
                     ("ccSnd", SND)):
            if p.symbol_named(n):
                m.store(self.sym(n), 4, v)
        if self.ext:
            m.store(self.ext[0], 4, EXT)
        self.heap = HEAP
        hooks = self.card.hooks(p)
        hooks.update({
            "ccMalloc__FUi": self._malloc, "ccMallocB__FUi": self._malloc,
            "__nw__FUi": self._malloc, "ccFree__FPv": lambda m, *a: 0,
            "ccBreathThread__Fi": lambda m, *a: 0,
            "CheckPort__7ccMcardFii": lambda m, *a: 0xffffffff,
            "sceCdSearchFile": self._cd_search, "Read2__6ccCdvdFUiUiPv": self._cd_read,
        })
        for n, f in hooks.items():
            s = p.symbol_named(n)
            if s is not None:
                m.hooks[s.value] = f

    def sym(self, name):
        s = self.p.symbol_named(name)
        if s is None:
            raise SystemExit(f"{self.elf}: no {name}")
        return s.value

    def object_size(self):
        """sizeof(ccSaveData): the byte count its constructor clears (`li
        $v0, SIZE` bounding the loop); 0x8530 in all four volumes."""
        ct = self.p.symbol_named("__ct__10ccSaveDataFv")
        for i in range(64):
            va = ct.value + 4 * i
            ins = mips.decode(self.p.u32(va), va)
            if ins.word >> 26 == 0x0d and ins.rs == 0:                   # li $v0, 0x8530
                return ins.word & 0xffff
        raise SystemExit(f"{self.elf}: no size in ccSaveData's constructor")

    def _malloc(self, m, n, *_):
        a = self.heap
        self.heap += (n + 63) & ~63
        m.mem[a:a + n] = bytes(n)
        return a

    def _cd_search(self, m, fp, name, *_):
        m.store(fp, 4, 0)                                  # lsn 0: ICON.BIN's first sector
        m.store(fp + 4, 4, len(self.icon_bin()))
        return 1

    def _cd_read(self, m, this, lsn, sectors, buf):
        data = self.icon_bin()[2048 * lsn:2048 * (lsn + sectors)]
        m.mem[buf:buf + len(data)] = data
        return 1

    def icon_bin(self):
        if not hasattr(self, "_icons"):
            path = os.path.join(os.path.dirname(os.path.abspath(self.elf)), "DATA", "ICON.BIN")
            with open(path, "rb") as f:
                self._icons = f.read()
        return self._icons

    # -- the request protocol ---------------------------------------------
    def request(self, req, arg, yes=1):
        """req(saveSys, arg), then MainProccess until it settles, answering
        its confirmations (result bit 0x8000) with NextProccess(yes)."""
        m = self.m
        m.call(self.sym(req), (SYS, arg))
        m.call(self.sym("MainProccess__9ccSaveSysFv"), (SYS,))
        for _ in range(4):
            if not m.load(SYS + 0x2a8, 4) & 0x8000:
                break
            m.call(self.sym("NextProccess__9ccSaveSysFi"), (SYS, yes))
            m.call(self.sym("MainProccess__9ccSaveSysFv"), (SYS,))
        return m.load(SYS + 0x2a8, 4)

    def call(self, name, *args):
        return self.m.call(self.sym(name), args)

    def save_bytes(self):
        n = self.save_size
        data = bytes(self.m.mem[SAVE:SAVE + n])
        if self.ext:
            data += bytes(self.m.mem[EXT:EXT + self.ext[1]])
        return data


def minimal_save(g, level=1, name=b"Kite"):
    """A zeroed ccSaveData whose player has a name and a level, for SaveSys'
    slot record: spcParam[0].base.name points at plName, as LoadGame sets it."""
    m = g.m
    m.mem[SAVE:SAVE + g.save_size] = bytes(g.save_size)
    if g.ext:
        m.mem[EXT:EXT + g.ext[1]] = bytes(g.ext[1])
    m.mem[SAVE:SAVE + len(name) + 1] = name + b"\0"
    m.store(SAVE + 0x7488, 4, SAVE)             # spcParam[0].base.name -> plName
    m.store(SAVE + 0x7496, 1, level)            # spcParam[0].base.level


def run_card(elf, slot=0, card=None):
    """MakeDir and one save on a blank card, by the executable's own code."""
    g = Game(elf, card)
    g.call("InitInfo__9ccSaveSysFv", SYS)
    made = g.call("MakeDir__7ccMcardFii", MC, 0, 0)
    minimal_save(g)
    result = g.request("SaveDataReq__9ccSaveSysFi", slot)
    return g, made, result


def icon_sys(data):
    """The fields of an icon.sys ("PS2D" sceMcIconSys) that the game fills."""
    title = data[0xc0:0xc0 + 68].split(b"\0\0", 1)[0]
    names = [data[0x104 + 64 * k:0x104 + 64 * (k + 1)].split(b"\0", 1)[0].decode("latin-1")
             for k in range(3)]
    return {"magic": data[:4].decode("latin-1"), "title": title.decode("cp932", "replace"),
            "title_break": struct.unpack_from("<H", data, 6)[0], "list": names[0],
            "copy": names[1], "delete": names[2]}


def patterns(size):
    rnd = random.Random(99)
    return [bytes(i & 0xff for i in range(size)), bytes((i >> 8) & 0xff for i in range(size)),
            bytes(rnd.randrange(256) for _ in range(size))]


def trace(dest_runs, sources):
    """Group each destination byte by where it came from across the pattern
    runs: ('copy', delta) - source offset minus destination offset - a
    constant, or 'untouched' (still the 0xee fill), or 'other'."""
    segs = []
    n = len(dest_runs[0])
    for d in range(n):
        vals = [r[d] for r in dest_runs]
        x = vals[1] << 8 | vals[0]
        if x < len(sources[2]) and sources[2][x] == vals[2]:
            kind = ("copy", x - d)
        elif vals[0] == vals[1] == vals[2]:
            kind = ("untouched",) if vals[0] == 0xee else ("const", vals[0])
        else:
            kind = ("other",)
        if segs and segs[-1][0] == kind:
            segs[-1][2] = d + 1
        else:
            segs.append([kind, d, d + 1])
    return segs


def learn_previous(elf, slot):
    """The previous-volume paths and read size this executable asks for:
    run LoadInfoPrevReq and LoadDataPrevReq(slot) against cards that lack
    the files, and read the card calls. -> (index path, data path, size)."""
    g = Game(elf)
    g.request("LoadInfoPrevReq__9ccSaveSysFi", 0)
    index_path = next(p for c, p, _ in g.card.log if c == "open-missing")
    card = Card({index_path: bytes(SLOT_INFO * 12)})
    g = Game(elf, card)
    g.m.mem[SYS + 0x150 + SLOT_INFO * slot] = 1          # infoPrev[slot].status
    g.request("LoadDataPrevReq__9ccSaveSysFi", slot)
    data_path = next(p for c, p, _ in card.log if c == "open-missing")
    card.files[data_path] = bytearray(0x10000)
    g = Game(elf, card)
    g.request("LoadDataPrevReq__9ccSaveSysFi", slot)
    size = next(n for c, p, n in card.log if c == "read" and p == data_path)
    return index_path, data_path, size


def previous_card(elf, slot, data, clear=None):
    """A card holding one previous-volume save at the paths `elf` asks for:
    `data` as the slot file, and an index whose record for the slot is used,
    level 1, named Kite, cleared (clearFlag volumeNum - 1: the title screen
    offers only a save that finished the previous volume) and summed."""
    index_path, data_path, size = learn_previous(elf, slot)
    g = Game(elf)
    record = bytearray(SLOT_INFO)
    record[0:4] = bytes([1, 1, max(g.volume - 1, 1) if clear is None else clear, 0])
    record[4:8] = b"Kite"
    struct.pack_into("<H", record, INFO["sum"], sum(data) & 0xffff)
    index = bytes(SLOT_INFO * slot) + bytes(record) + bytes(SLOT_INFO * (11 - slot))
    return Card({index_path: index, data_path: data})


def load_previous(elf, slot, card):
    """LoadInfoPrevReq and LoadDataPrevReq(slot), as the title screen's
    Data_Control::Data_Select does, with ccSaveData (and the extension)
    first filled with 0xee. -> (Game, index result, data result)."""
    g = Game(elf, card)
    g.m.mem[SAVE:SAVE + g.save_size] = b"\xee" * g.save_size
    if g.ext:
        g.m.mem[EXT:EXT + g.ext[1]] = b"\xee" * g.ext[1]
    info = g.request("LoadInfoPrevReq__9ccSaveSysFi", 0)
    result = g.request("LoadDataPrevReq__9ccSaveSysFi", slot)
    return g, info, result


def describe_segments(segs, base_name):
    out = []
    for kind, a, b in segs:
        if kind[0] == "copy":
            out.append(f"  {base_name}+0x{a:04x}..0x{b:04x}  <- file 0x{a + kind[1]:04x}..0x"
                       f"{b + kind[1]:04x}")
        elif kind[0] == "untouched":
            out.append(f"  {base_name}+0x{a:04x}..0x{b:04x}  not written")
        else:
            out.append(f"  {base_name}+0x{a:04x}..0x{b:04x}  {kind}")
    return out


def convert(g):
    """ccStartEventConvert, then ccSaveData::ConvGame - ccThDemo's answer to
    a previous-volume load. -> (bytes StartEventConvert changed, bytes ConvGame
    changed), as offsets into ccSaveData followed by the extension."""
    before = g.save_bytes()
    g.call("ccStartEventConvert__Fv")
    mid = g.save_bytes()
    g.call("ConvGame__10ccSaveDataFv", SAVE)
    after = g.save_bytes()
    return ([i for i in range(len(mid)) if mid[i] != before[i]],
            [i for i in range(len(after)) if after[i] != mid[i]])


def carry(elf, slot=3, prev=None):
    lines = []
    index_path, data_path, size = learn_previous(elf, slot)
    if prev:
        # The previous volume's own code writes the card.
        pg, _, _ = run_card(prev, slot)
        card = Card(pg.card.files)
        lines.append(f"{prev} wrote {len(pg.card.files)} files in "
                     + ", ".join(sorted(pg.card.dirs)))
        missing = [p for p in (index_path, data_path) if p not in card.files]
        if missing:
            lines.append(f"this volume asks for {', '.join(missing)}, which that card lacks")
            return "\n".join(lines)
        written = pg.save_bytes()
        g, info, result = load_previous(elf, slot, card)
        loaded = g.save_bytes()
        n = min(len(written), len(loaded))
        lines.append(f"{data_path}: {len(card.files[data_path])} bytes on the card, {size} read; "
                     f"results 0x{info:x}, 0x{result:x}")
        lines.append(f"the first {n} bytes as loaded equal the ones saved (padding aside): "
                     f"{all(loaded[i] == written[i] or loaded[i] == 0xee for i in range(n))}")
        return "\n".join(lines)
    runs = []
    sources = patterns(size)
    for data in sources:
        g, info, result = load_previous(elf, slot, previous_card(elf, slot, data))
        runs.append(g)
    g = runs[0]
    lines.append(f"volume {g.volume}: sizeof(ccSaveData) 0x{g.save_size:x}"
                 + (f", extension 0x{g.ext[1]:x} bytes at *(0x{g.ext[0]:08x})" if g.ext else
                    ", no extension"))
    lines.append(f"previous volume's index: {index_path} ({SLOT_INFO * 12} bytes), "
                 f"result 0x{info:x}")
    lines.append(f"previous volume's slot {slot}: {data_path}, {size} bytes read, "
                 f"result 0x{result:x}")
    segs = trace([bytes(r.m.mem[SAVE:SAVE + r.save_size]) for r in runs], sources)
    lines.append("ccSaveData:")
    lines += describe_segments(segs, "saveData")
    if g.ext:
        segs = trace([bytes(r.m.mem[EXT:EXT + g.ext[1]]) for r in runs], sources)
        lines.append("extension:")
        lines += describe_segments(segs, "ext")
    # Then what ccThDemo does, on a zeroed save of the same size (so the
    # party flags are clear and every character is converted).
    g, _, _ = load_previous(elf, slot, previous_card(elf, slot, bytes(size)))
    flags, conv = convert(g)
    lines.append("ccStartEventConvert: " + ", ".join(
        f"+0x{i:x} (eventFlag[{(i - 0x54f8) // 8}] byte {(i - 0x54f8) % 8})" for i in flags))
    lines.append(f"ConvGame (partyMemberFlag 0): {len(conv)} bytes changed in "
                 + ", ".join(f"0x{a:x}-0x{b:x}" for a, b in ranges(conv)))
    return "\n".join(lines)


def ranges(xs, gap=16):
    """Sorted offsets -> [start, end) runs, joining runs less than `gap` apart."""
    out = []
    for x in xs:
        if out and x - out[-1][1] < gap:
            out[-1][1] = x + 1
        else:
            out.append([x, x + 1])
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("card")
    p.add_argument("elf")
    p.add_argument("--slot", type=int, default=0)
    p.add_argument("--out", help="write the card's files under this directory")
    p = sub.add_parser("carry")
    p.add_argument("elf")
    p.add_argument("--slot", type=int, default=3)
    p.add_argument("--prev", help="the previous volume's executable, to write the save")
    args = parser.parse_args()

    if args.cmd == "card":
        g, made, result = run_card(args.elf, args.slot)
        print(f"volume {g.volume}: MakeDir -> {made}, save -> result 0x{result:x}")
        for path in sorted(g.card.files):
            data = g.card.files[path]
            print(f"  {path:44} {len(data):7} bytes")
            if path.endswith("icon.sys"):
                for k, v in icon_sys(bytes(data)).items():
                    print(f"      {k:12} {v!r}")
        if args.out:
            for path, data in g.card.files.items():
                dest = os.path.join(args.out, path.lstrip("/"))
                os.makedirs(os.path.dirname(dest), exist_ok=True)
                with open(dest, "wb") as f:
                    f.write(data)
    else:
        print(carry(args.elf, args.slot, args.prev))
    return 0


if __name__ == "__main__":
    sys.exit(main())
