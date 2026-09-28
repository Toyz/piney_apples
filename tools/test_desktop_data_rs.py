#!/usr/bin/env python3
"""crates/piney-desktop's Data screen (data.rs, savesys.rs, card.rs) against
the game's own SaveData_control (desktop.prg, savedata.cpp) and ccSaveSys
(main, sdmng.cpp) run in tools/eemu.py.

Both sides run the same scenarios frame by frame: the game's
MainSaveData_control and DrawBack after ccSaveSys::MainProccess (the card
task, while StartReq's task runs), the port's Data::main and draw_back
through the desktop_probe example. The memory card is kept in Python for the
game (the sceMc* calls answered from a dict of files per port, CheckPort and
Format answered from the scenario's flags) and in the probe for the port,
with the same rules. Compared each frame: SaveMode, Slot, listNO, dialog,
alpha, tempPN, saveSys result, proccess, fileNum and port, the return, and
every sound, ccKanji::Disp (layer, position and colour bits, text),
ccSprite::MakePacket (layer, dx dy sx sy cx cy bits, cell, alpha),
SendPacket and page draw in call order; at the end the index and slot files
each side's card holds (the game's MakeDir also writes icon.sys and the
icons, which the port's card does not).

Scenarios: saving to an empty slot and over used ones (clear data and parody
records), answering NO, no card, not a PS2 card, unformatted (formatted, the
directory created), full, no directory, write, index write, format and
directory failures, card slot 2, the positions ccSaveSys keeps from before,
frame rate 2, held keys, and random pads. Then SetSavePar's strings and
colours over many records, DrawCur, TimeAlphaCurDraw and DispButton (its
scale in double precision, bit exact) directly.

Skipped when the disc is not extracted or cargo is missing.
"""

import os
import random
import shutil
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import test_save_init_rs  # noqa: E402
import volume  # noqa: E402
from test_desktop_rs import ELF, ISO, ask, build  # noqa: E402

# Scratch memory for the interpreter.
SYS, SAVE, MC, GAME, CCSYS, THIS, HEAP = (0x01800000, 0x01810000, 0x01830000, 0x01840000, 0x01850000, 0x01860000,
                                          0x01900000)
OK, CANCEL = 0x40, 0x20
UP, DOWN = 0x1000, 0x4000
DIR = "/" + volume.card_dir()
INDEX = DIR + DIR
MC_CREAT = 0x200
SLOT_SIZE = volume.SLOT_SIZE
# The disc's volumeNum: clear data of it or later warns before an overwrite.
VOL = volume.number()


def f_bits(m, a):
    return m.load(a, 4)


class Card:
    """Memory cards in both ports: the scenario's flags and the files by
    path, answering the sceMc* calls as they complete at once."""

    def __init__(self):
        self.ports = [dict(present=0, ps2=1, formatted=1, full=0, failwrite=0, failsys=0, failfmt=0, dir=0,
                           files={}) for _ in range(2)]
        self.fds = {}
        self.pending = 0
        self.next_fd = 3

    def check_port(self, port):
        p = self.ports[port & 1]
        if not p["present"]:
            return 0
        if not p["ps2"]:
            return 1
        if not p["formatted"]:
            return 2
        if p["dir"]:
            return -1
        if p["full"]:
            return 3
        return 4

    def hooks(self):
        from eemu import _cstr

        def mc_open(m, port, slot, path, mode):
            path = _cstr(m, path).decode("latin-1")
            p = self.ports[port & 1]
            if path not in p["files"]:
                if not mode & MC_CREAT:
                    self.pending = -4
                    return 0
                p["files"][path] = bytearray()
            fd = self.next_fd
            self.next_fd += 1
            self.fds[fd] = [port & 1, path, 0]
            self.pending = fd
            return 0

        def mc_read(m, fd, buf, size, *_):
            port, path, pos = self.fds[fd]
            data = self.ports[port]["files"][path][pos:pos + size]
            m.mem[buf:buf + len(data)] = data
            self.fds[fd][2] = pos + len(data)
            self.pending = len(data)
            return 0

        def mc_write(m, fd, buf, size, *_):
            port, path, pos = self.fds[fd]
            p = self.ports[port]
            if (p["failwrite"] and "/dhdata" in path) or (p["failsys"] and path == INDEX):
                self.pending = -5
                return 0
            f = p["files"][path]
            f[pos:pos + size] = bytes(m.mem[buf:buf + size])
            self.fds[fd][2] = pos + size
            self.pending = size
            return 0

        def mc_close(m, fd, *_):
            self.fds.pop(fd, None)
            self.pending = 0
            return 0

        def mc_mkdir(m, port, slot, path, *_):
            p = self.ports[port & 1]
            self.pending = -4 if p["dir"] else 0
            p["dir"] = 1
            return 0

        def mc_flush(m, *_):
            # Write, Flush, then one Sync: a failed write is what it reports.
            self.pending = min(self.pending, 0)
            return 0

        def mc_sync(m, mode, cmd, result, *_):
            if result:
                m.store(result, 4, self.pending)
            return 1

        def check_port(m, this, port, *_):
            return self.check_port(port) & 0xFFFFFFFF

        def fmt(m, this, port, *_):
            p = self.ports[port & 1]
            if p["failfmt"]:
                return 8
            p.update(formatted=1, dir=0, files={})
            return 0

        return {"sceMcOpen": mc_open, "sceMcRead": mc_read, "sceMcWrite": mc_write, "sceMcClose": mc_close,
                "sceMcMkdir": mc_mkdir, "sceMcFlush": mc_flush, "sceMcSync": mc_sync,
                "CheckPort__7ccMcardFii": check_port, "Format__7ccMcardFii": fmt}

    def summary(self):
        """The index and slot files, as the probe prints them."""
        out = {}
        for port, p in enumerate(self.ports):
            for path, b in p["files"].items():
                if path != INDEX and not path.startswith(DIR + "/dhdata"):
                    continue
                out[f"{port}:{path}"] = f"zeros:{len(b)}" if not any(b) else bytes(b).hex()
        return out


def fresh_save():
    """The save both sides start from: the port's SaveState::fresh_with, made
    here by the game's own boot (ccSound's and ccSaveData's constructors,
    Init(1); test_save_init_rs.fresh_save) with the player named Kite."""
    return test_save_init_rs.fresh_slot(ELF)


class Game:
    """SaveData_control and ccSaveSys in eemu. `events` collects the frame's
    draws and sounds as the probe prints them."""

    NAMES = {0x78: "mask", 0x7c: "info", 0x80: "icon"}

    def __init__(self, p, card, rate=1):
        from eemu import Machine
        self.p = p
        self.m = m = Machine(p)
        self.card = card
        self.rate = rate
        self.heap = HEAP
        self.events = []
        self.layers = {}
        self.names = {}
        g = lambda n: p.symbol_named(n).value  # noqa: E731
        for n, v in (("saveSys", SYS), ("saveData", SAVE), ("ccMc", MC), ("game", GAME), ("ccSys", CCSYS)):
            m.store(g(n), 4, v)
        hooks = card.hooks()
        hooks.update({
            "ccSeOn__Fi": lambda mm, a0, *a: self.events.append(["se", a0]) or 0,
            "ccBreathThread__Fi": lambda mm, *a: 0,
            "ccStartThread__FPFPv_vii": lambda mm, *a: 0x1234,
            "ccDeleteThread__FP6ccTscb": lambda mm, *a: 0,
            "GetFrameRate__8ccSystemFv": lambda mm, *a: self.rate,
            "__nw__FUi": self._malloc, "__nwa__FUi": self._malloc, "ccMalloc__FUi": self._malloc,
            "ccMallocB__FUi": self._malloc, "ccFree__FPv": lambda mm, *a: 0,
            "__construct_new_array": lambda mm, blk, *a: blk + 16,
            "__ct__6ccMaskFii": lambda mm, a0, *a: a0,
            "__ct__16ccDrawPacketCtrlFv": lambda mm, a0, *a: a0,
            "__ct__12ccMenuWindowFv": lambda mm, a0, *a: a0,
            "Init__7ccLayerFsP6ccView": self._layer_init,
            "SetFrame__6ccViewFffffffff": self._set_frame,
            "SetPrim__8ccSpriteFii": lambda mm, *a: 0,
            "SetTex__8ccSpriteFPcPci": lambda mm, *a: 0,
            "SetAnm__5ccAnmFP8ccStreamPcUi": lambda mm, *a: 0,
            "_AnimateForward__5ccAnmFUi": lambda mm, *a: 0,
            "Draw__5ccAnmFv": self._anm_draw,
            "Disp__7ccKanjiFPciff": self._disp,
            "MakePacket__8ccSpriteFii": self._make_packet,
            "SendPacket__8ccSpriteFv": lambda mm, a0, *a: self.events.append(["send", self.name(a0)]) or 0,
            "Trans__8ccSpriteFv": lambda mm, a0, *a: self.events.append(["menu", "Trans"]) or 0,
            "SetType__12ccMenuWindowFi": lambda mm, a0, a1, *a: self.events.append(["menu", "SetType"]) or 0,
            "DispPageCursol__12ccMenuWindowFi": lambda mm, *a: self.events.append(["menu", "DispPageCursol"]) or 0,
            "sprintf": self._sprintf,
            "sceCdSearchFile": self._cd_search, "Read2__6ccCdvdFUiUiPv": self._cd_read,
            "__dt__6ccMaskFv": lambda mm, *a: 0, "__destroy_new_array": lambda mm, *a: 0,
            "__dt__7ccLayerFv": lambda mm, *a: 0, "__dt__8ccSpriteFv": lambda mm, *a: 0,
            "__dl__FPv": lambda mm, *a: 0,
        })
        for n, f in hooks.items():
            s = p.symbol_named(n)
            assert s is not None, n
            m.hooks[s.value] = f
        self.frames_log = []

    # -- hooks ------------------------------------------------------------
    def _malloc(self, m, n, *_):
        a = self.heap
        self.heap += (n + 63) & ~63
        m.mem[a:a + n] = bytes(n)
        return a

    def _layer_init(self, m, a0, a1, *_):
        self.layers[a0] = a1 & 0xffff
        return 0

    def _set_frame(self, m, a0, *a):
        self.frames_log.append([m.f[i] for i in range(12, 20)])
        return 0

    def _anm_draw(self, m, a0, *a):
        back = m.load(THIS + 0x84, 4)
        self.events.append(["anm", (a0 - back) // 0x110])
        return 0

    def name(self, a):
        return self.names.get(a, hex(a))

    def _layer_of(self, obj):
        lay = self.m.load(obj + 0xa8, 4)
        return self.layers.get(lay, -1)

    def _disp(self, m, a0, a1, *_):
        from eemu import _cstr
        rgba = [m.load(a0 + o, 1) for o in (0x68, 0x6c, 0x70, 0x74)]
        self.events.append(["disp", self.name(a0), self._layer_of(a0), f_bits(m, a0 + 0x40), f_bits(m, a0 + 0x44),
                            rgba, _cstr(m, a1).hex()])
        return 0

    def _make_packet(self, m, a0, *_):
        self.events.append(["pkt", self.name(a0), self._layer_of(a0),
                            [f_bits(m, a0 + o) for o in (0x40, 0x44, 0x48, 0x4c, 0x34, 0x38)],
                            m.load(a0 + 0x50, 4, True), m.load(a0 + 0x54, 4, True), m.load(a0 + 0x58, 4, True),
                            m.load(a0 + 0x5c, 4, True), m.load(a0 + 0x74, 1)])
        return 0

    def _sprintf(self, m, dst, fmt, a2, a3):
        from eemu import _cstr
        f = _cstr(m, fmt)
        args = [a2, a3, m.r[8] & 0xffffffff, m.r[9] & 0xffffffff, m.r[10] & 0xffffffff]
        out, k, j = b"", 0, 0
        while j < len(f):
            if f[j:j + 1] == b"%":
                t = f[j + 1:j + 2]
                if t == b"s":
                    out += _cstr(m, args[k])
                elif t == b"d":
                    out += str(struct.unpack("<i", struct.pack("<I", args[k]))[0]).encode()
                else:
                    raise RuntimeError(f)
                k += 1
                j += 2
            else:
                out += f[j:j + 1]
                j += 1
        m.mem[dst:dst + len(out) + 1] = out + b"\0"
        return len(out)

    def _cd_search(self, m, fp, name, *_):
        m.store(fp, 4, 0)
        m.store(fp + 4, 4, len(self.icon_bin()))
        return 1

    def _cd_read(self, m, this, lsn, sectors, buf):
        data = self.icon_bin()[2048 * lsn:2048 * (lsn + sectors)]
        m.mem[buf:buf + len(data)] = data
        return 1

    def icon_bin(self):
        if not hasattr(self, "_icons"):
            with open(os.path.join(os.path.dirname(ELF), "DATA", "ICON.BIN"), "rb") as f:
                self._icons = f.read()
        return self._icons

    # -- driving ------------------------------------------------------------
    def call(self, name, *args):
        va = name if isinstance(name, int) else self.p.symbol_named(name).value
        return self.m.call(va, args)

    def construct(self, save, port=0, file=0):
        """Boot: saveSysMsg, ccSaveSys (left at `port`, `file`), the save
        data, then SaveData_control's constructor."""
        m = self.m
        self.call("__sinit_sdmng.cpp")
        self.call("__ct__9ccSaveSysFv", SYS)
        m.store(SYS + 0x2a0, 4, port)
        m.store(SYS + 0x2a4, 4, file)
        test_save_init_rs.place_save(m, SAVE, save)
        self.call("__ct__16SaveData_controlFv", THIS)

    def enter(self):
        """ChooseMode's entry: StartReq(3), SetData(ccsc, kanjilayer); then
        what the constructors the hooks skip would set: every kanji's and
        mask's colour (128, 128, 128, 128) before SetData's."""
        m = self.m
        self.call("StartReq__9ccSaveSysFi", SYS, 3)
        kl = self._malloc(m, 0x40)
        self.layers[kl] = 127
        self.call("SetData__16SaveData_controlFP8ccStreamP7ccLayer", THIS, 0x0abc0000, kl)
        g = lambda o: m.load(THIS + o, 4)  # noqa: E731
        self.names = {g(o): n for o, n in self.NAMES.items()}
        for base, n, stem in ((0x68, 5, "kanji"), (0x70, 4, "infokanji"), (0x74, 2, "diakanji")):
            for k in range(n):
                a = g(base) + 0xe8 * k
                self.names[a] = f"{stem}[{k}]"
                m.store(a + 0x74, 4, 128)
        for o in self.NAMES:
            for c in (0x68, 0x6c, 0x70, 0x74):
                m.store(g(o) + c, 4, 128)
            # SetData set the cursor cell; keep what it wrote.

    def state(self):
        m = self.m
        return [m.load(THIS + 0x14, 4, True), m.load(THIS + 0x24, 4, True), m.load(THIS + 0xc, 4, True),
                m.load(THIS + 0x48, 4, True), m.load(THIS + 0, 4, True), m.load(THIS + 0x4c, 4, True),
                m.load(SYS + 0x2a8, 4), m.load(SYS + 0x2ac, 4, True), m.load(SYS + 0x2a4, 4, True),
                m.load(SYS + 0x2a0, 4, True)]

    def frame(self, direct, push, unpush, repeat, count):
        m = self.m
        for o, v in ((0x2cc, direct), (0x2d0, push), (0x2d4, unpush), (0x2d8, repeat), (0x358, count)):
            m.store(CCSYS + o, 4, v)
        self.events = []
        if m.load(SYS + 0x2b4, 4):
            self.call("MainProccess__9ccSaveSysFv", SYS)
        ret = struct.unpack("<i", struct.pack("<I", self.call("MainSaveData_control__16SaveData_controlFv", THIS)))[0]
        if ret == -1:
            self.call("EndReq__9ccSaveSysFv", SYS)
        else:
            self.call("DrawBack__16SaveData_controlFv", THIS)
        return {"state": self.state(), "ret": ret, "ev": self.events}


class Scenario:
    """One scenario for both sides: card flags and files, save data
    patches, ccSaveSys's position, frame rate; then the pads."""

    def __init__(self, name):
        self.name = name
        self.setup = []
        self.pads = []
        self.rate = 1
        self.sys_port, self.sys_file = 0, 0
        self.patches = [(0x7488, struct.pack("<I", SAVE)), (0x7496, bytes([7]))]

    def card(self, port, present=1, ps2=1, formatted=1, full=0, failwrite=0, failsys=0, failfmt=0):
        self.setup.append(("card", port, present, ps2, formatted, full, failwrite, failsys, failfmt))
        return self

    def dir(self, port):
        self.setup.append(("dir", port))
        return self

    def file(self, port, name, data):
        self.setup.append(("file", port, name, bytes(data)))
        return self

    def prepared(self, port, records=()):
        """A card as the game's MakeDir leaves it, the index holding
        `records` ((slot, bytes28) pairs)."""
        idx = bytearray(336)
        for slot, rec in records:
            idx[28 * slot:28 * slot + 28] = rec
        self.file(port, DIR.lstrip("/"), idx)
        for s in range(12):
            self.file(port, f"dhdata{s + 1:02d}", bytes(SLOT_SIZE))
        return self

    def save(self, off, data):
        self.patches.append((off, bytes(data)))
        return self

    def press(self, *pushes, count0=0):
        """One frame per push (pressed alone: direct = push)."""
        for p in pushes:
            self.pads.append((p, p, 0, 0, count0 + len(self.pads)))
        return self

    def raw(self, direct, push, unpush, repeat):
        self.pads.append((direct, push, unpush, repeat, len(self.pads)))
        return self

    def probe_lines(self):
        out = ["datascen", f"rate {self.rate}", f"sys {self.sys_port} {self.sys_file}"]
        for s in self.setup:
            if s[0] == "file":
                out.append(f"file {s[1]} {s[2]} {s[3].hex()}")
            else:
                out.append(" ".join(str(v) for v in s))
        for off, data in self.patches:
            out.append(f"save {off} {data.hex()}")
        for d, p, u, r, c in self.pads:
            out.append(f"frame {d} {p} {u} {r} {c}")
        out.append("end")
        return out

    def run_game(self, prog):
        card = Card()
        for s in self.setup:
            if s[0] == "card":
                keys = ("present", "ps2", "formatted", "full", "failwrite", "failsys", "failfmt")
                card.ports[s[1]].update(dict(zip(keys, s[2:])))
            elif s[0] == "dir":
                card.ports[s[1]]["dir"] = 1
            else:
                card.ports[s[1]]["dir"] = 1
                card.ports[s[1]]["files"][f"{DIR}/{s[2]}"] = bytearray(s[3])
        save = fresh_save()
        for off, data in self.patches:
            save[off:off + len(data)] = data
        g = Game(prog, card, self.rate)
        g.construct(save, self.sys_port, self.sys_file)
        g.enter()
        frames = []
        for pad in self.pads:
            if frames and frames[-1]["ret"] == -1:
                g.enter()           # the player opens Data again
            frames.append(g.frame(*pad))
        return {"frames": frames, "files": card.summary()}


def record(status=1, level=7, clear=0, parody=0, name=b"Kite", sum_=0x1234, playtime=0):
    r = bytearray(28)
    r[0:4] = bytes([status & 0xff, level & 0xff, clear & 0xff, parody & 0xff])
    r[4:4 + len(name)] = name
    struct.pack_into("<Hi", r, 0x16, sum_, playtime)
    return bytes(r)


HMS = lambda h, m, s, f=0: ((h * 60 + m) * 60 + s) * 60 + f  # noqa: E731


def scenarios():
    out = []
    # Save to an empty slot of a ready card, then back out.
    s = Scenario("save to an empty slot").card(0).prepared(0).save(0x8400, struct.pack("<i", HMS(3, 25, 7, 30)))
    s.press(0, 0, OK, 0, 0, DOWN, 0, 0, OK, 0, 0, UP, 0, OK, 0, 0, 0, OK, 0, 0, 0, 0, CANCEL, 0, CANCEL, 0, 0)
    out.append(s)
    # The same, answering NO, then saving to the first slot with YES.
    s = Scenario("answer NO, then YES").card(0).prepared(0)
    s.press(0, 0, OK, 0, 0, OK, 0, 0, OK, 0, 0, 0, OK, 0, 0, UP, OK, 0, 0, OK, 0, 0, 0, 0, CANCEL, 0, CANCEL, 0)
    out.append(s)
    # Used slots: a cleared save (warned), a parody one, a full play time.
    recs = [(0, record(level=1, name=b"Kite", playtime=HMS(0, 0, 0))),
            (1, record(level=20, clear=VOL, name=b"Orca", playtime=HMS(999, 59, 59))),
            (2, record(level=99, parody=1, name=b"Balmung", playtime=HMS(12, 5, 9))),
            (3, record(level=5, clear=4, parody=1, name=b"ABCDEFGHIJKLMNOPQ", playtime=HMS(1, 0, 0)))]
    s = Scenario("overwrite, clear data, parody").card(0).prepared(0, recs).save(0x842a, bytes([0]))
    s.press(0, 0, OK, 0, 0, 0, DOWN, 0, DOWN, 0, DOWN, 0, UP, 0, UP, 0, OK, 0, 0, OK, 0, 0, UP, OK, 0, 0, OK, 0, 0,
            0, 0, UP, 0, OK, 0, 0, UP, OK, 0, 0, OK, 0, 0, 0, CANCEL, 0, CANCEL, 0, 0)
    out.append(s)
    # The save's own clear flag matches the slot's: no warning.
    s = Scenario("same clear flag").card(0).prepared(0, recs[1:2]).save(0x842a, bytes([VOL]))
    s.press(0, 0, OK, 0, 0, DOWN, 0, OK, 0, 0, OK, 0, 0, OK, 0, 0, 0, OK, 0, 0, 0)
    out.append(s)
    # No card in either port.
    s = Scenario("no card").card(0, present=0).card(1, present=0)
    s.press(0, 0, OK, 0, 0, OK, 0, 0, DOWN, 0, OK, 0, 0, OK, 0, CANCEL, 0, 0)
    out.append(s)
    s = Scenario("not a PS2 card").card(0, ps2=0)
    s.press(0, 0, OK, 0, 0, OK, 0, 0, UP, UP, 0, CANCEL, 0)
    out.append(s)
    # Unformatted: format, "Formatted", no directory, create, the list.
    s = Scenario("unformatted, format and create").card(0, formatted=0)
    s.press(0, 0, OK, 0, 0, OK, 0, UP, OK, 0, 0, OK, 0, 0, OK, 0, UP, OK, 0, 0, OK, 0, 0, 0, OK, 0, UP, OK, 0, 0,
            OK, 0, 0, 0, CANCEL, 0, CANCEL, 0)
    out.append(s)
    s = Scenario("unformatted, answer NO").card(0, formatted=0)
    s.press(0, 0, OK, 0, 0, OK, 0, 0, OK, 0, 0, CANCEL, 0)
    out.append(s)
    s = Scenario("format fails").card(0, formatted=0, failfmt=1)
    s.press(0, 0, OK, 0, 0, OK, 0, UP, OK, 0, 0, OK, 0, 0, 0)
    out.append(s)
    s = Scenario("full").card(0, full=1)
    s.press(0, 0, OK, 0, 0, OK, 0, 0, OK, 0, 0, 0)
    out.append(s)
    s = Scenario("no directory, create").card(0)
    s.press(0, 0, OK, 0, 0, OK, 0, 0, UP, OK, 0, 0, OK, 0, 0, 0, 0, OK, 0, 0, OK, 0, 0, OK, 0, 0, 0)
    out.append(s)
    s = Scenario("no directory, answer NO").card(0)
    s.press(0, 0, OK, 0, 0, OK, 0, 0, OK, 0, 0, 0)
    out.append(s)
    s = Scenario("create fails").card(0, failwrite=1)
    s.press(0, 0, OK, 0, 0, OK, 0, 0, UP, OK, 0, 0, OK, 0, 0, 0)
    out.append(s)
    s = Scenario("write fails").card(0, failwrite=1).prepared(0)
    s.press(0, 0, OK, 0, 0, DOWN, 0, OK, 0, UP, OK, 0, 0, OK, 0, 0, 0)
    out.append(s)
    s = Scenario("index write fails").card(0, failsys=1).prepared(0)
    s.press(0, 0, OK, 0, 0, OK, 0, 0, UP, OK, 0, 0, OK, 0, 0)
    out.append(s)
    # Card slot 2, and ccSaveSys left on port 1, file 5 by the title screen.
    s = Scenario("card slot 2").card(0, present=0).card(1).prepared(1, recs[:2])
    s.sys_port, s.sys_file = 1, 5
    s.press(0, 0, OK, 0, 0, DOWN, 0, OK, 0, 0, UP, OK, 0, 0, OK, 0, 0, 0, OK, 0, 0, 0, CANCEL, 0, UP, 0, OK, 0, 0)
    out.append(s)
    # A bad index reads as empty.
    s = Scenario("bad index").card(0).prepared(0, [(0, record(level=100)), (1, record(playtime=HMS(999, 59, 59, 1)))])
    s.press(0, 0, OK, 0, 0, 0, OK, 0, 0)
    out.append(s)
    # Frame rate 2: the blink and the button's pulse.
    s = Scenario("frame rate 2").card(0, full=1)
    s.rate = 2
    s.press(*([0] * 3 + [OK] + [0] * 30), count0=1000)
    out.append(s)
    # Held keys: the list's repeat, the slot cursor's, the question's.
    s = Scenario("held keys").card(0).prepared(0, recs)
    s.press(0, 0, OK, 0, 0)
    for k in (DOWN, UP, CANCEL, UP):
        s.raw(k, k, 0, k)
        for _ in range(45):
            s.raw(k, 0, 0, k)
        s.raw(0, 0, k, 0)
    s.press(OK, 0, 0)
    for k in (UP, DOWN):
        for _ in range(35):
            s.raw(k, 0, 0, k)
        s.raw(0, 0, k, 0)
    s.press(CANCEL, 0, 0, CANCEL, 0)
    out.append(s)
    # Random pads over ready, empty and failing cards.
    for seed, setup in ((1, lambda s: s.card(0).prepared(0, recs).card(1).prepared(1)),
                        (2, lambda s: s.card(0, formatted=0).card(1, full=1)),
                        (3, lambda s: s.card(0).card(1, failwrite=1).prepared(1, recs)),
                        (4, lambda s: s.card(0).prepared(0, recs[1:]).save(0x842a, bytes([2])))):
        rng = random.Random(seed)
        s = setup(Scenario(f"random pads {seed}"))
        keys = [OK, OK, CANCEL, UP, DOWN, UP | DOWN, OK | CANCEL]
        for i in range(400):
            k = rng.choice(keys) if rng.random() < 0.3 else 0
            rep = rng.choice([0, 0, UP, DOWN]) if rng.random() < 0.2 else 0
            unp = rng.choice([0, UP, DOWN, CANCEL]) if rng.random() < 0.1 else 0
            s.raw(k | rep, k, unp, rep)
        out.append(s)
    return out


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class DataAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from image import Program
        build()
        cls.p = Program(ELF, "desktop")

    def compare(self, s, got):
        want = s.run_game(self.p)
        for i, (w, g) in enumerate(zip(want["frames"], got["frames"])):
            if w != g:
                print(f"{s.name}: frame {i} pad {s.pads[i]}")
                print("  game", w["state"], w["ret"])
                print("  port", g["state"], g["ret"])
                for a, b in zip(w["ev"] + [None] * 99, g["ev"] + [None] * 99):
                    if a is None and b is None:
                        break
                    print("   ", "  " if a == b else "!=", a, b if a != b else "")
                self.fail(f"{s.name}: frame {i} differs")
        self.assertEqual(len(want["frames"]), len(got["frames"]))
        self.assertEqual(want["files"], got["files"], f"{s.name}: the card's files")
        return want

    def test_scenarios(self):
        scen = scenarios()
        got = ask([line for s in scen for line in s.probe_lines()])
        self.assertEqual(len(got), len(scen))
        seen = set()
        for s, g in zip(scen, got):
            with self.subTest(s.name):
                w = self.compare(s, g)
                for f in w["frames"]:
                    seen.add(f["state"][6])
        # The results the scenarios went through: every message the screen
        # can show on the desktop (the volume's own for no directory and
        # clear data, two apart per volume).
        v = 2 * (VOL - 1)
        for r in (0, 1, 2, 4, 13, 25, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x100a, 0x800b, 0x1022, 0x2023,
                  0x1032 + v, 0x8033 + v, 0x1026, 0x2027, 0x801e, 0x801f, 0x102a + v, 0x802b + v, 0x101c, 0x201d):
            self.assertIn(r, seen, f"no scenario reached result {r:#x}")

    def test_saved_files(self):
        """The first scenario's save: slot 2's file is the save data, its
        index record the game's."""
        s = scenarios()[0]
        want = s.run_game(self.p)
        save = fresh_save()
        for off, data in s.patches:
            save[off:off + len(data)] = data
        self.assertEqual(want["files"][f"0:{DIR}/dhdata02"], bytes(save).hex())
        idx = bytes.fromhex(want["files"]["0:" + INDEX])
        rec = idx[28:56]
        self.assertEqual(rec[:4], bytes([1, 7, 0, 0]))
        self.assertEqual(rec[4:9], b"Kite\0")
        self.assertEqual(struct.unpack_from("<Hi", rec, 0x16), (sum(save) & 0xffff, HMS(3, 25, 7, 30)))

    # SetSavePar, the cursors, the button ------------------------------------
    def piece_game(self, rate=1):
        g = Game(self.p, Card(), rate)
        g.construct(fresh_save())
        g.enter()
        return g

    def test_set_save_par(self):
        rng = random.Random(5)
        cases = []
        for status in (0, 1):
            for clear in (0, 1, 2, 4, -1):
                for parody in (0, 1):
                    for pt in (0, 59, HMS(0, 1, 0), HMS(12, 5, 9), HMS(999, 59, 59, 59), -1, -216000 - 7):
                        cases.append(record(status, rng.choice([0, 1, 9, 10, 99, -3]), clear, parody,
                                            rng.choice([b"Kite", b"", b"#0", b"ABCDEFGHIJKLMNOPQ"]), 0, pt))
        cases.append(record(1, 7, 0, 0, b"ABCDEFGHIJKLMNOPQR", 0x4100, 60))
        lines = [f"datapar {r.hex()} {i % 12} {i % 13}" for i, r in enumerate(cases)]
        got = ask(lines)
        g = self.piece_game()
        bad = 0
        for i, (r, gv) in enumerate(zip(cases, got)):
            slot, no = i % 12, i % 13
            g.m.mem[SYS + 28 * slot:SYS + 28 * slot + 28] = r
            g.events = []
            g.call("SetSavePar__16SaveData_controlFP14ccSaveDataInfoi", THIS, SYS + 28 * slot, no)
            if g.events != gv:
                bad += 1
                if bad < 4:
                    print("SetSavePar", r.hex(), no, "game", g.events, "port", gv)
        self.assertEqual(bad, 0)

    def test_cursors(self):
        for rate in (1, 2):
            cases, lines = [], []
            for lst in range(-3, 16):
                for result in (0, 4, 0x1032, 0x8033, 0x10020):
                    count = (lst * 37 + result) % 400
                    cases.append(("cur", lst, result, count))
                    lines.append(f"datacur {lst} {result} {count} {rate}")
            for count in list(range(0, 330, 7)) + [0xfffffffe, 0xffffffff]:
                for small, alpha, result in ((0, 0, 13), (0, 0, 0x2005), (1, 0, 0x801e), (1, 64, 0x801e),
                                             (1, 128, 0x8033), (0, 128, 0x10020)):
                    cases.append(("cursor", 239, 278 + small, small, alpha, result, count))
                    lines.append(f"datacursor 239 {278 + small} {small} {alpha} {result} {count} {rate}")
            got = ask(lines)
            g = self.piece_game(rate)
            bad = 0
            for c, gv in zip(cases, got):
                g.events = []
                if c[0] == "cur":
                    _, lst, result, count = c
                    g.m.store(THIS + 0xc, 4, lst)
                    g.m.store(SYS + 0x2a8, 4, result)
                    g.m.store(CCSYS + 0x358, 4, count)
                    g.call("DrawCur__16SaveData_controlFv", THIS)
                else:
                    _, x, y, small, alpha, result, count = c
                    g.m.store(THIS, 4, alpha)
                    g.m.store(SYS + 0x2a8, 4, result)
                    g.m.store(CCSYS + 0x358, 4, count)
                    g.call("TimeAlphaCurDraw__16SaveData_controlFiii", THIS, x, y, small)
                if g.events != gv:
                    bad += 1
                    if bad < 4:
                        print("cursor", rate, c, "game", g.events, "port", gv)
            self.assertEqual(bad, 0)

    def test_disp_button(self):
        for rate in (1, 2):
            counts = list(range(0, 100)) + [0xffffffff - i for i in range(3)]
            got = ask([f"databtn 297 {256 + 16 * (c % 5)} {c} {rate}" for c in counts])
            g = self.piece_game(rate)
            bad = 0
            for c, gv in zip(counts, got):
                g.events = []
                g.m.store(CCSYS + 0x358, 4, c)
                g.call("DispButton__16SaveData_controlFii", THIS, 297, 256 + 16 * (c % 5))
                if g.events != gv:
                    bad += 1
                    if bad < 4:
                        print("DispButton", rate, c, "game", g.events, "port", gv)
            self.assertEqual(bad, 0)

    def test_set_data_frames(self):
        """SetData's layers and frames, which the port's views are built from."""
        g = self.piece_game()
        from eemu import f_to_py
        frames = [[f_to_py(v) for v in f] for f in g.frames_log]
        self.assertEqual(sorted(g.layers.values()), [127, 128, 129])
        self.assertEqual(frames[0], [0, 0, 512, 448, 256, 224, 1, 1])
        self.assertEqual(frames[1][:7], [0, 0, 512, 448, 256, 224, 1])
        self.assertEqual(g.frames_log[1][7], 0x3f5b6db7)
        self.assertEqual(frames[2], [0, 0, 512, 448, 256, 224, 1, 1])


if __name__ == "__main__":
    unittest.main()
