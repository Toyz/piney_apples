#!/usr/bin/env python3
"""The event interpreter (crates/piney-event) against the game's own code.

The game's functions run in tools/eemu.py, extended here (EvMachine) with the
64-bit variable shifts dsllv, dsrlv and dsrav that the flag code uses. Main
alone is loaded; the few calls into overlays and into rendering are hooked
and answer from a world description both this file and the Rust test build
from the same seed. Everything the scripts read and write in ccSaveData is
the game's code at work: ccEventFlagSet, CheckOpen, SetCurrentOpen, Execute,
ccStartEvent, ccStartThEvent, NewMail, AddItem, SetGateList and the rest run
unhooked.

What is compared (in crates/piney-event/tests/vm.rs, against the fixture this
writes; the Rust side cannot run eemu):

- init: the boot's save (ccSaveData's constructor with ccSnd pointing at a
  constructed ccSound, then Init(1)) but for Init's text, against the port's
  SaveData::boot; the Rust side starts every case from SaveData::boot.
- flagset: ccEventFlagSet(n) for every script, from the boot save and from
  random saves: every byte of the 0x8530-byte
  saveData, and the event manager's registers (mng_digest).
- startth, startev: ccStartThEvent on random saves; ccStartEvent(vol, flag)
  for volumes 1-4 and both flags.
- open: the header's and every block's CheckOpen at lv 2, walked with the
  game's SetCurrentOpen, for every script under random saves and worlds.
- cond: every condition of every script alone, under the same.
- sub: eventSub(n) at lv 2 run to its end, frames and save.
- play: every block of every script played with Execute at lv 2 in turn,
  whatever its conditions: the frames each takes, the save, the registers.
- desktop: a new game reaching the desktop. The boot's ccStartEvent(1, 0),
  then the desktop setup's ccStartThEvent and phases 0, 2 and 4 with the
  game's own ccThEvent running frame by frame (its Breath is the frame
  boundary), then play: the desktop asks CheckOperate for two operations
  and the player reads two mails. The phase after each frame, every hooked
  call with its frame, and the save three times.

Calls outside the event and save code (message windows, streams, overlays,
name entry, menus, gcmn) are hooked and finish at once; the Rust test host
answers the same way.

    python3 tools/test_event_vm.py                 the unit tests
    python3 tools/test_event_vm.py fixture         write crates/piney-event/tests/vm_fixture.txt

The fixture holds numbers only: offsets and bytes, seeds, event numbers,
frame counts. No script, message or label text goes in it.
"""

import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402
import test_save_init_rs  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
FIXTURE = os.path.join(ROOT, "crates", "piney-event", "tests", "vm_fixture.txt")

M32 = 0xFFFFFFFF
M64 = (1 << 64) - 1
SAVE_SIZE = 0x8530
# ccSaveData::timeIdolRankStr: Init's text, kept out of the fixture.
IDOL_TEXT = (0x6775, 0x683D)

# Scratch memory (eemu has flat RAM up to 32 MB).
SAVE = 0x01800000
EVMNG = 0x01810000
GAME = 0x01811000
WORLDMAN = 0x01812000
SYS = 0x01813000
ENT = 0x01814000
EOT = 0x01815000
TSCB = 0x01816000
TARGET = 0x01817000
BOSSTSCB = 0x01818000
ENEMY = 0x01819000
CHUNK = 0x0181a000
MENU = 0x0181b000
DTMENU = 0x0181c000
STREAM = 0x0181d000
HEAP = 0x01900000

# ccSaveData offsets (Infection's DWARF, docs/formats/save.md).
EVENT_FLAG, EVENT_STATUS = 0x54f8, 0x64f8
MAIL, MAIL_ORDER, NEWS, BBS = 0x2264, 0x2464, 0x2864, 0x28e4
ITEMS, IMP, SKILLS, TALK = 0x30, 0xcfc, 0x1ec4, 0x220c
MEMBER = 0x2220   # flag, call, call store, exp, save: 5 ints
TOWN_MOVE, DT_LISTS = 0x2234, 0x2238    # wallpaper[3], bgm[3], str[5]
GATE_LIST, GATE_MARK, WORD_LIST, GATE_ORDER = 0x4fe4, 0x5048, 0x523c, 0x5278
AREA_BAN, PROTECT, PLCOL, CRISIS = 0x6548, 0x65c8, 0x6771, 0x6772
PARTY_TIME, SPC, SPC_SIZE = 0x73b8, 0x7488, 0xdc
LAST_TOWN, CLEAR_FLAG, PARODY = 0x8426, 0x842a, 0x842b

# gcmn data the event code reads directly (the overlay is not loaded).
PARTY_MANAGER = inf_va(0x00730310)       # ccParty: memberChar[3], memberID[3] (+0x0c), num (+0x18)
SPC_MANAGER = inf_va(0x00730340)         # 5 x 0x2c, +0 code
PLW_PTR = inf_va(0x00730300)


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def s32(v):
    v &= M32
    return v - (1 << 32) if v & 0x80000000 else v


class SplitMix:
    """splitmix64, the generator the Rust test uses for the same seeds."""

    def __init__(self, seed):
        self.s = seed & M64

    def next(self):
        self.s = (self.s + 0x9E3779B97F4A7C15) & M64
        z = self.s
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & M64
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & M64
        return z ^ (z >> 31)

    def below(self, n):
        return self.next() % n

    def pick(self, seq):
        return seq[self.below(len(seq))]


def randomise_save(img, seed):
    """A random save over `img` (bytearray), member by member. The Rust test
    (tests/vm.rs, randomise_save) does exactly the same."""
    r = SplitMix(seed)

    def put(at, n, v):
        img[at:at + n] = (v & ((1 << (8 * n)) - 1)).to_bytes(n, "little")

    for n in range(512):
        x = r.next()
        f = r.next() & ((1 << 62) - 1) if x & 3 else 0
        if (x >> 2) & 7 == 0:
            f |= 1 << 62
        if (x >> 5) & 7 == 0:
            f |= 1 << 63
        put(EVENT_FLAG + 8 * n, 8, f)
    for i in range(80):
        put(EVENT_STATUS + i, 1, r.below(16) - 4)
    for i in range(512):
        put(MAIL + i, 1, r.pick((0, 0, 0, 1, 4, 5, 6)))
    k = r.below(64)
    for i in range(512):
        put(MAIL_ORDER + 2 * i, 2, r.below(512) if i < k else -1)
    for i in range(128):
        put(NEWS + i, 1, r.pick((0, 0, 1, 3)))
    for i in range(128 * 48):
        put(BBS + i, 1, r.pick((0, 0, 0, 1, 3, 7)))
    for i in range(5):
        v = r.next() & M32
        if i == 1 and r.below(2):
            v |= 0x80000000
        put(MEMBER + 4 * i, 4, v)
    put(TOWN_MOVE, 2, r.next())
    for i in range(11):
        put(DT_LISTS + 4 * i, 4, r.next())
    for i in range(25):
        put(GATE_LIST + 4 * i, 4, r.next())
        put(GATE_MARK + 4 * i, 4, r.next())
    for i in range(15):
        put(WORD_LIST + 4 * i, 4, r.next())
    for i in range(5 * 64):
        put(GATE_ORDER + 2 * i, 2, r.below(151) - 1)
    for i in range(32 * 4):
        put(AREA_BAN + i, 1, r.below(5) - 1)
    for i in range(5):
        put(PROTECT + 4 * i, 4, r.next())
    for i in range(18):
        put(TALK + i, 1, r.below(8))
    put(PLCOL, 1, r.below(2))
    put(CRISIS, 1, r.below(2))
    for pc in range(18):
        for k in range(40):
            at = ITEMS + 160 * pc + 4 * k
            if r.below(3):
                put(at, 2, -1)
                put(at + 2, 1, -1)
                put(at + 3, 1, 0)
            else:
                put(at, 2, r.below(100))
                put(at + 2, 1, r.below(16))
                put(at + 3, 1, r.below(100))
        for k in range(20):
            put(SKILLS + 40 * pc + 2 * k, 2, r.below(312) - 1 if k < 4 else -1)
        put(SPC + SPC_SIZE * pc + 0x14, 4, r.below(10_000_500))
        put(SPC + SPC_SIZE * pc + 0xda, 2, r.below(1100) - 50)
    for i in range(320):
        put(IMP + i, 1, r.below(100) if r.below(4) == 0 else 0)
    for i in range(17):
        put(PARTY_TIME + 4 * i, 4, r.below(7560 * 3))
    put(LAST_TOWN, 1, r.below(6))
    put(CLEAR_FLAG, 1, r.below(3))
    put(PARODY, 1, 1 if r.below(4) == 0 else 0)


POSITION_NUMS = (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 31, 40, 50)


class World:
    """A random world for the conditions: the mode and scene, the event
    manager's registers, the party, what is present, menus, pad, markers.
    The Rust test (tests/vm.rs, World) draws the same values in the same
    order."""

    def __init__(self, seed, stories):
        r = SplitMix(seed ^ 0x5EED5EED)
        self.status = r.pick((1, 2, 3, 5, 5, 5))
        self.area = r.below(3)
        self.server = r.below(5)
        self.town = r.below(6)
        self.field = r.below(4)
        self.dungeon = r.below(4)
        self.floor = r.below(4)
        self.block = r.below(6)
        self.phase = r.below(7) - 1
        self.msg_num = r.below(13)
        self.msg_select = r.below(4)
        self.operate = r.next() & ((1 << 29) - 1)
        self.operate_set = r.below(22) - 1
        if r.below(2):
            code, server, words, _ = stories[r.below(len(stories))]
            self.area_code_set = [0 if w is None else w for w in words]
            if r.below(2):
                self.server = server
        else:
            self.area_code_set = [r.below(40) - 1 for _ in range(3)]
        self.points = []
        for _ in range(16):
            if r.below(3) == 0:
                self.points.append((r.below(4), r.below(6), r.below(11)))
            else:
                self.points.append((-1, -1, -1))
        self.positions = [(r.below(4), r.below(6), POSITION_NUMS[k], float(r.below(500))) for k in range(16)]
        self.party = [0, r.pick((-1, -1) + tuple(range(1, 18))), r.pick((-1, -1) + tuple(range(1, 18)))]
        self.party_num = sum(1 for x in self.party if x != -1)
        self.spc = [x for x in self.party if x != -1] + [r.below(20) for _ in range(5)]
        self.spc = self.spc[:5]
        if r.below(2):
            self.target = (1 << r.pick((2, 3)), r.below(140), r.below(2))
        else:
            self.target = None
        self.chars = [(r.next() & M32, 125 + r.below(40)) for _ in range(r.below(4))]
        self.gimmicks = [(r.next() & M32, r.below(40), r.below(256)) for _ in range(r.below(3))]
        self.mc_num = r.below(2)
        self.boss_entry = r.below(17) - 1
        self.boss_task = r.pick((None, 0, 1))
        self.active = r.below(2)
        self.menu = r.pick((-1, -1, 0, 62))
        self.pad = r.next() & M32
        self.enemy_pp = [r.below(2) for _ in range(64)]
        self.town_marker_x = [float(r.below(500)) for _ in range(33)]


def machine_class():
    import eemu

    class EvMachine(eemu.Machine):
        """eemu.Machine plus dsllv, dsrlv and dsrav."""

        def exec(self, pc, w):
            if w >> 26 == 0 and (w & 63) in (0x14, 0x16, 0x17):
                rs, rt, rd = (w >> 21) & 31, (w >> 16) & 31, (w >> 11) & 31
                sh = self.r[rs] & 63
                v = self.r[rt] & M64
                fn = w & 63
                if fn == 0x14:
                    out = (v << sh) & M64
                elif fn == 0x16:
                    out = v >> sh
                else:
                    out = (v - (1 << 64) if v >> 63 else v) >> sh
                self.set(rd, out & M64)
                return None
            return super().exec(pc, w)

        def call_nested(self, addr, args=()):
            """call() from inside a hook: the caller's registers are kept and
            the callee gets a stack well below the running code's."""
            saved = (list(self.r), list(self.f), self.hi, self.lo, self.hi1, self.lo1, self.fcr31, self.acc,
                     self.steps)
            try:
                self.r = [0] * 32
                self.r[28] = self.p.gp
                self.r[29] = eemu.STACK_TOP - 0x80000
                self.r[31] = eemu.RETURN_SENTINEL
                for i, a in enumerate(args[:8]):
                    self.set32(4 + i, a)
                pc, npc = addr, addr + 4
                while pc != eemu.RETURN_SENTINEL:
                    hook = self.hooks.get(pc)
                    if hook is not None:
                        self.set32(2, hook(self, *(self.r[4 + i] & 0xFFFFFFFF for i in range(4))))
                        pc = self.r[31] & 0xFFFFFFFF
                        npc = pc + 4
                        continue
                    target = self.exec(pc, self.load(pc, 4))
                    if target is eemu.ANNUL:
                        pc, npc = npc + 4, npc + 8
                    else:
                        pc, npc = npc, (target if target is not None else npc + 4)
                return self.r[2] & 0xFFFFFFFF
            finally:
                (self.r, self.f, self.hi, self.lo, self.hi1, self.lo1, self.fcr31, self.acc, self.steps) = saved

    return EvMachine


class GameEvents:
    """Infection's event code in eemu, with scratch objects for everything it
    dereferences."""

    def __init__(self):
        from image import Program
        self.p = Program(ELF)
        self.m = machine_class()(self.p)
        self.sym = lambda n: self.p.symbol_named(n).value
        m = self.m
        for name, va in (("saveData", SAVE), ("eventMng", EVMNG), ("game", GAME), ("worldman", WORLDMAN),
                         ("ccSys", SYS), ("g_entCtrl", ENT), ("ccMenu", MENU), ("dtMenu", DTMENU)):
            m.store(self.sym(name), 4, va)
        self.heap = HEAP
        self.calls = []            # (frame, name, args) of hooked calls, for the desktop trace
        self.frame = 0
        self.world = None
        for name in ("__nw__FUi", "__builtin_new"):
            s = self.p.symbol_named(name)
            if s is not None:
                m.hooks[s.value] = self.alloc
        self.hook("SimGenerateCode__9WORLD_MANFiii", lambda mm, this, a, b, c: self.log("generate_area", s32(a), s32(b), s32(c)))
        self.hook("ccClearGtHack__Fv", lambda mm, *a: self.log("clear_gate_hack"))
        self.fresh = self.boot_save()

    def hook(self, name, fn):
        self.m.hooks[self.sym(name)] = fn

    def alloc(self, m, n, *_):
        a = self.heap
        self.heap += (n + 15) & ~15
        m.mem[a:a + n] = bytes(n)
        return a

    # --- the save --------------------------------------------------------------------------
    def boot_save(self):
        """ccSaveData's constructor and Init(1), as ccThMother runs them,
        with ccSnd pointing at a constructed ccSound (test_save_init_rs)."""
        self.set_save(test_save_init_rs.boot_save(ELF))
        return self.save()

    def set_save(self, img):
        self.m.mem[SAVE:SAVE + SAVE_SIZE] = img

    def save(self):
        return bytes(self.m.mem[SAVE:SAVE + SAVE_SIZE])

    def init_mng(self):
        self.m.mem[EVMNG:EVMNG + 0x800] = bytes(0x800)
        self.m.call(self.sym("Init__7ccEventFv"), (EVMNG,))

    def mng_digest(self):
        """FNV-1a 64 of the event manager's registers, as mng_digest in
        tests/vm.rs builds them: enablePhase, msgSelect, msgNum,
        entryNpcNum, registNpcNum; target, entry, entryMc, areaCode; each
        evPos's floor, block and number; evPoint; currentOpen; operate,
        operateSet, areaCodeSet."""
        m = self.m
        e = EVMNG
        raw = bytearray()
        for off in (0xc, 0x10, 0x14, 0x18, 0x1c):
            raw += m.mem[e + off:e + off + 4]
        raw += m.mem[e + 0x40:e + 0x1c0]
        for k in range(16):
            raw += m.mem[e + 0x1c0 + 32 * k:e + 0x1c0 + 32 * k + 8]
        raw += m.mem[e + 0x3c0:e + 0x440]
        raw += m.mem[e + 0x750:e + 0x780]
        return fnv(raw)

    # --- bookkeeping -----------------------------------------------------------------------
    def flag_set(self, n, img):
        """ccEventFlagSet(n) on `img`; the save after it."""
        self.set_save(img)
        self.init_mng()
        self.m.call(self.sym("ccEventFlagSet__Fi"), (n,))
        return self.save()

    def events(self):
        """(event, script va) for every script in eventTbl."""
        tbl = self.sym("eventTbl")
        out = []
        for g in range(10):
            base = self.p.u32(tbl + 4 * g)
            if not base:
                continue
            for i in range(50):
                va = self.p.u32(base + 8 * i)
                if va:
                    out.append((50 * g + i, va))
        return out


    # --- conditions --------------------------------------------------------------------------
    def set_world(self, w):
        m = self.m
        st = lambda a, n, v: m.store(a, n, v & ((1 << (8 * n)) - 1))
        m.mem[GAME:GAME + 0x88] = bytes(0x88)
        for off, v in ((0, w.status), (0x14, w.area), (0x18, w.area), (0x1c, w.server), (0x20, w.town),
                       (0x24, w.field), (0x28, w.dungeon), (0x2c, w.floor), (0x30, w.block)):
            st(GAME + off, 4, v)
        e = EVMNG
        st(e + 0xc, 4, w.phase)
        st(e + 0x10, 4, w.msg_select)
        st(e + 0x14, 4, w.msg_num)
        st(e + 0x8, 4, w.boss_entry)
        st(e + 0x770, 8, w.operate)
        st(e + 0x778, 2, w.operate_set)
        for k in range(3):
            st(e + 0x77a + 2 * k, 2, w.area_code_set[k])
        for k, (fl, bl, num) in enumerate(w.points):
            st(e + 0x3c0 + 8 * k, 2, fl)
            st(e + 0x3c2 + 8 * k, 2, bl)
            st(e + 0x3c4 + 8 * k, 4, num)
        for k, (fl, bl, num, x) in enumerate(w.positions):
            a = e + 0x1c0 + 32 * k
            st(a, 2, fl)
            st(a + 2, 2, bl)
            st(a + 4, 4, num)
            m.mem[a + 0x10:a + 0x20] = struct.pack("<4f", x, 0.0, 0.0, 1.0)
        if w.target is not None:
            types, code, _ = w.target
            st(e + 0x780, 4, TARGET)
            st(TARGET, 4, TARGET + 0x100)
            st(TARGET + 0x108, 4, types)
            st(TARGET + 0x10c, 2, code)
        else:
            st(e + 0x780, 4, 0)
        if w.boss_task is None:
            st(e + 0x7bc, 4, 0)
        else:
            st(e + 0x7bc, 4, BOSSTSCB)
            st(BOSSTSCB + 0x14, 4, w.boss_task)
        for k in range(3):
            st(PARTY_MANAGER + 0xc + 4 * k, 4, w.party[k])
        st(PARTY_MANAGER + 0x18, 4, w.party_num)
        for k in range(5):
            st(SPC_MANAGER + 0x2c * k, 4, w.spc[k])
        m.mem[ENT:ENT + 0x2000] = bytes(0x2000)

        def chain(nodes, base):
            head = 0
            for i in reversed(range(len(nodes))):
                node = ENT + base + 0x200 * i
                data = node + 0x100
                types, code = nodes[i][0], nodes[i][1]
                st(node, 4, data)
                st(data + 8, 4, types)
                st(data + 0xc, 2, code)
                if len(nodes[i]) > 2:
                    st(node + 0xe0, 1, nodes[i][2])
                st(node + 0x1c4, 4, head)
                head = node
            return head
        st(ENT + 0x10, 4, len(w.chars))
        st(ENT + 0x14, 4, chain(w.chars, 0x200))
        st(ENT + 0x1c, 4, w.mc_num)
        st(ENT + 0x28, 4, len(w.gimmicks))
        st(ENT + 0x2c, 4, chain(w.gimmicks, 0x1000))
        st(SYS + 0x2d0, 4, w.pad)
        st(WORLDMAN + 0x430, 4, STREAM)
        st(PLW_PTR, 4, TARGET + 0x800)
        self.world = w

    def hook_world(self):
        """The overlay and rendering calls the conditions make, answered
        from self.world."""
        m = self.m
        marker_tbl = self.sym("markerEvTbl")
        names = [self.p.u32(marker_tbl + 4 * k) for k in range(33)]

        def check_target(mm, ptr, *_):
            return self.world.target[2] if self.world.target is not None and ptr == TARGET else 0

        def get_enemy(mm, this, code, *_):
            mm.store(ENEMY + 4, 4, ENEMY + 0x100)
            mm.store(ENEMY + 0x162, 2, self.world.enemy_pp[code & 63])
            return ENEMY

        def trans(mm, out, inp, *_):
            mm.mem[out:out + 16] = mm.mem[inp:inp + 16]
            return 0

        def dist(mm, a, b, *_):
            mm.f[0] = mm.load(a, 4)
            return 0

        def chunk(mm, stream, name, *_):
            x = self.world.town_marker_x[names.index(name)] if name in names else 0.0
            mm.mem[CHUNK + 0x10:CHUNK + 0x20] = struct.pack("<4f", x, 0.0, 0.0, 1.0)
            return CHUNK

        gcmn = {0x519920: check_target, 0x526150: lambda mm, *a: self.world.menu & M32,
                0x42df70: lambda mm, *a: self.world.active, 0x59b940: trans}
        for va, fn in gcmn.items():
            m.hooks[va] = fn
        # CheckMemberID and the gcmn data it reads: run the real function? It
        # lives in gcmn; answer as it does (memberID[i] == pc & 0xffff).
        m.hooks[0x59cfe0] = lambda mm, this, pc, *_: next(
            (i for i in range(3) if s32(mm.load(this + 0xc + 4 * i, 4)) == (pc & 0xFFFF)), -1) & M32
        self.hook("GetEnemy__7ccEventFi", get_enemy)
        self.hook("ccGetDist__FPfPf", dist)
        self.hook("GetChunkAdrsF__8ccStreamFPCci", chunk)

    def open_results(self, n, va):
        """The header's CheckOpen at lv 2, then each block's after its
        settings, stepping over the body with Execute at lv 0."""
        m = self.m
        co, ex = self.sym("CheckOpen__7ccEventFRPsiii"), self.sym("Execute__7ccEventFRPsiii")
        sc, clr = self.sym("SetCurrentOpen__7ccEventFPs"), self.sym("ClearCurrentOpen__7ccEventFv")
        m.store(EOT, 4, va)
        out = [m.call(co, (EVMNG, EOT, n, M32, 2))]
        m.call(clr, (EVMNG,))
        for b in range(62):
            eot = m.load(EOT, 4)
            if m.load(eot, 2, True) == -1:
                break
            while m.load(eot, 2, True) == -2:
                eot = m.call(sc, (EVMNG, eot))
            m.store(EOT, 4, eot)
            out.append(m.call(co, (EVMNG, EOT, n, b, 2)))
            m.call(ex, (EVMNG, EOT, n, b, 0))
        return out

    def condition_instances(self):
        """(event, [shorts]) of every condition in every script, in event
        order, the header's then each block's, as tools/evscript.py parses
        them."""
        import evscript
        ev = evscript.Events(ELF)
        out = []
        for n in sorted(ev.events):
            sc = ev.script(ev.events[n][0])
            for c in sc.header + [c for b in sc.blocks for c in b.conds]:
                out.append((n, [c.code] + list(c.args)))
        return out

    def condition(self, n, shorts):
        """One condition list [code, args..., 0] through CheckOpen at lv 2
        for event n, as an open condition (no settings)."""
        m = self.m
        raw = struct.pack(f"<{len(shorts) + 1}h", *shorts, 0)
        code = EOT + 0x100
        m.mem[code:code + len(raw)] = raw
        m.store(EOT, 4, code)
        return m.call(self.sym("CheckOpen__7ccEventFRPsiii"), (EVMNG, EOT, n, M32, 2))

    # --- the event task ------------------------------------------------------------------------
    def hook_play(self):
        """The calls a played block makes outside the event code: logged
        with the frame they happen in, and finished at once (windows close
        on the first Check, name entry's Main says done)."""
        m = self.m
        log = self.log
        noop = lambda mm, *a: 0
        for name in ("__ct__16ccDrawPacketCtrlFv", "Init__7ccLayerFsP6ccView", "SetFrame__6ccViewFffffffff",
                     "ccInitMenuWindow__FP7ccLayer", "Disp__9ccMessageFv", "Trans__8ccSpriteFv", "__dt__9ccMessageFv",
                     "__dt__8ccSpriteFv", "__dt__7ccLayerFv", "ccEvVoiceStop__Fv", "Close__9ccMessageFv"):
            self.hook(name, noop)
        self.hook("__ct__9ccMessageFv", lambda mm, this, *a: this)
        self.hook("SetFrameRate__8ccSystemFi", lambda mm, this, rate, *a: log("frame_rate", s32(rate)))
        paths = {"DEMO": 0, "DESKTOP": 1, "TOPPAGE": 2, "GCMN": 3}

        def overlay(mm, flag, path, *_):
            name = bytes(mm.mem[path:path + 64]).split(b"\0")[0].decode("latin-1")
            which = next(v for k, v in paths.items() if name.upper().endswith("\\" + k + ".PRG"))
            return log("overlay", which)
        self.hook("ccLoadOverlay__FiPc", overlay)
        self.hook("ccEventStream__Fii", lambda mm, num, flag, *a: log("stream", s32(num)))
        self.hook("Change__9ccMessageFP9ccMsgDataPcii",
                  lambda mm, this, data, voice, ev, *a: log("message", s32(ev), s16(mm.r[8])))
        self.hook("ChangeInfo__9ccMessageFPcPcPcPcii",
                  lambda mm, *a: log("info", s32(mm.r[9]), s16(mm.r[10])))
        self.hook("Check__9ccMessageFi", lambda mm, *a: 1)
        m.hooks[0x419520] = lambda mm, this, *a: (log("name_entry"), this)[1]
        m.hooks[0x414310] = lambda mm, *a: 1
        m.hooks[0x419550] = noop
        self.hook("ccSeOn__Fi", lambda mm, se, *a: log("se", s32(se)))
        self.hook("ccSndEvRequest__Fiiii", lambda mm, c, a0, a1, a2: log("sound", s32(c), s32(a0), s32(a1), s32(a2)))
        self.hook("MenuBan__7ccEventFv", lambda mm, *a: log("menu_ban", 1))
        self.hook("MenuClr__7ccEventFv", lambda mm, *a: log("menu_ban", 0))

        def change(mm, this, num, sf, *_):
            mm.store(EVMNG + 0xc, 4, M32)        # ccDisableThEvent, inside ChangeRequest
            return log("change_request", s32(num), s32(sf))
        self.hook("ChangeRequest__6ccGameFii", change)
        self.hook("CheckMenuType__8ccDtMenuFv", lambda mm, *a: M32)

    SUB_STUBS = (
        "__ct__16ccDrawPacketCtrlFv", "SetFrame__6ccViewFffffffff", "Init__7ccLayerFsP6ccView", "__dt__7ccLayerFv",
        "cosf", "sinf", "fptosi", "ccDeleteThread__FP6ccTscb", "ccSleepAllThread__Fv", "ccWakeAllThread__Fv",
        "__dt__8ccSpriteFv", "Trans__8ccSpriteFv", "EntryFlash__8ccScFadeFiiffff", "EntryFade__8ccScFadeFiiiffff",
        "ContinueFade__8ccScFadeFiii", "changeCamera__Fi", "cameraSoftReset__Fv", "ccFileExistCheck__Fi",
        "ChangeScene__6ccGameFiiiiii", "ChangeArea__6ccGameFii", "ccSetFileListTown__Fv", "ccSetFileListTownDel__Fv",
        "ccSetFileListField__Fv", "ccSetFileListFieldDel__Fv", "ccSetFileListDungeon__Fv",
        "ccSetFileListDungeonDel__Fv", "ccSndBgmCtrl__Fv", "Quit__9WORLD_MANFv", "RoomSelect__9WORLD_MANFii",
        "GoPrevRoom__9WORLD_MANFv", "fieldSel__Fv", "dungeonSel__Fv", "Open__9ccMessageFP9ccMsgDataPcii",
        "GetSpc__7ccEventFi", "GetNpc__7ccEventFi", "effRemoveTrap__FPfii", "ccGetDirc__FPfPf",
        "ccGetDircPL__FPf", "RAD2DEG__Ff", "DEG2RAD__Fs", "sceVu0AddVector", "sceVu0SubVector")

    def hook_sub(self):
        """For playing whole events: everything Execute calls outside the
        event manager and the save is answered at once (windows close on
        the first Check, menus are closed, ShowMap is done, a started task
        finishes after one frame); calls into gcmn return 0."""
        m = self.m
        for name in self.SUB_STUBS:
            self.hook(name, lambda mm, *a: 0)
        self.hook("ShowMap__9WORLD_MANFv", lambda mm, *a: 1)
        self.last_task = None

        def start_thread(mm, *a):
            t = self.alloc(mm, 0x60)
            self.last_task = t
            return t
        self.hook("ccStartThread__FPFPv_vii", start_thread)

        def breath(mm, n, *a):
            self.frame += s32(n) if s32(n) > 0 else 0
            if self.last_task is not None:
                mm.store(self.last_task + 0x14, 4, 1)
            # The desktop menu takes any request at once.
            mm.store(DTMENU + 6, 2, M32)
            return 0
        self.hook("ccBreathThread__Fi", breath)
        for va in (0x42e1d0, 0x4307f0, 0x430e70, 0x4312d0, 0x4316a0, 0x519630, 0x519700, 0x5199e0, 0x526940,
                   0x527950, 0x56b020, 0x5723e0, 0x572700, 0x572730, 0x581500, 0x5832e0, 0x5833a0, 0x59b980,
                   0x59cd60, 0x59ce80, 0x59cf60, 0x59eba0, 0x59f850, 0x5a0880, 0x5a0890, 0x5c8800, 0x5c8820):
            m.hooks[va] = lambda mm, *a: 0
        m.store(self.sym("cmndTarget"), 4, ENEMY + 0x800)
        m.store(ENEMY + 0x808, 2, 1)

    def sub_run(self, n):
        """eventSub(n) to its end: the frames its Breaths add up to."""
        self.frame = 0
        self.last_task = None
        self.m.call(self.sym("eventSub__Fi"), (n,), limit=50_000_000)
        return self.frame

    def has_messages(self, n):
        """Whether both message tables have an array for event n. Infection
        has none for the volume 2-4 story (events 100-149, 200-249,
        300-349): a message there reads a record at address 12 * msg, and
        is not compared."""
        for t in ("evMsgTbl", "evMsgTblp"):
            grp = self.p.u32(self.sym(t) + 4 * (n // 50))
            if not grp or not self.p.u32(grp + 4 * (n % 50)):
                return False
        return True

    def play_blocks(self, n, va):
        """Every block of event n played in turn with Execute at lv 2 (its
        settings applied, its conditions stepped over at lv 0): the frames
        each took."""
        m = self.m
        co, ex = self.sym("CheckOpen__7ccEventFRPsiii"), self.sym("Execute__7ccEventFRPsiii")
        sc = self.sym("SetCurrentOpen__7ccEventFPs")
        m.store(EOT, 4, va)
        m.call(co, (EVMNG, EOT, n, M32, 0))
        out = []
        for b in range(62):
            eot = m.load(EOT, 4)
            if m.load(eot, 2, True) == -1:
                break
            while m.load(eot, 2, True) == -2:
                eot = m.call(sc, (EVMNG, eot))
            m.store(EOT, 4, eot)
            m.call(co, (EVMNG, EOT, n, b, 0))
            self.frame = 0
            self.last_task = None
            # Null pointers read and written at low addresses start from zero
            # (the missing volume 2-4 message records are read from there).
            m.mem[0:0x1000] = bytes(0x1000)
            m.call(ex, (EVMNG, EOT, n, b, 2), limit=50_000_000)
            out.append(self.frame)
        return out

    def desktop_run(self, extra=90, actions=()):
        """A new game reaching the desktop: the boot's ccStartEvent(1, 0);
        then ccSetupDesktop's ccStartThEvent and ccEnableThEvent(0), (2), (4),
        each requested once the task has settled the one before, with
        ccThEvent running until `extra` frames into play. `actions` are
        (play frame, what, value) done after that frame of play, as the
        desktop would: ("operate", n) calls ccEvent::CheckOperate(n, 0),
        ("read", mail) sets mailList[mail] = 4. Returns [(stage, frame,
        save)], the phase after every frame, and the calls."""
        m = self.m
        self.set_save(self.fresh)
        self.init_mng()
        m.mem[GAME:GAME + 0x88] = bytes(0x88)
        m.store(GAME, 4, 2)
        m.mem[DTMENU:DTMENU + 0x40] = bytes(0x40)
        m.store(DTMENU + 6, 2, M32)
        m.mem[TSCB:TSCB + 0x60] = bytes(0x60)
        self.calls, self.frame = [], 0
        m.call(self.sym("ccStartEvent__Fii"), (1, 0))
        m.call(self.sym("ccStartThEvent__Fv"), ())
        m.store(EVMNG + 0xc, 4, 0)
        snaps, phases = [], []
        state = {"stage": 0, "left": extra}

        class EndRun(Exception):
            pass

        def end_of_frame():
            self.frame += 1
            phase = s32(m.load(EVMNG + 0xc, 4))
            phases.append(phase)
            if state["stage"] == 0 and phase == 1:
                snaps.append((0, self.frame, self.save()))
                m.store(EVMNG + 0xc, 4, 2)
                state["stage"] = 1
            elif state["stage"] == 1 and phase == 3:
                snaps.append((2, self.frame, self.save()))
                m.store(EVMNG + 0xc, 4, 4)
                state["stage"] = 2
            elif state["stage"] == 2 and phase == 5:
                state["left"] -= 1
                done = extra - state["left"]
                for at, what, v in actions:
                    if at != done:
                        continue
                    if what == "operate":
                        go = m.call_nested(self.sym("CheckOperate__7ccEventFii"), (EVMNG, v, 0))
                        self.calls.append((self.frame, "check_operate", (v, go)))
                    elif what == "read":
                        m.mem[SAVE + MAIL + v] = 4
                if state["left"] == 0:
                    snaps.append((4, self.frame, self.save()))
                    raise EndRun

        def breath(n):
            for _ in range(n):
                end_of_frame()
            return 0
        self.hook("Breath__6ccTscbFi", lambda mm, tscb, n, *a: breath(n))
        self.hook("ccBreathThread__Fi", lambda mm, n, *a: breath(n))
        try:
            m.call(self.sym("ccThEvent__FP6ccTscb"), (TSCB,), limit=200_000_000)
        except EndRun:
            pass
        return snaps, phases, list(self.calls)

    def log(self, name, *args):
        self.calls.append((self.frame + 1, name, tuple(args)))
        return 0

    # --- story areas -----------------------------------------------------------------------
    def story_areas(self):
        """Every eventAreaInfo record as the gate instructions see it:
        (code, server, [word id or None] * 3, [(item, count)] * 4), from the
        game's GetEventAreaInfo and GetWordParamFromEvCode."""
        m = self.m
        info_fn = self.sym("GetEventAreaInfo__9WORLD_MANFi")
        word_fn = self.sym("GetWordParamFromEvCode__9WORLD_MANFii")
        base = self.sym("eventAreaInfo")
        out = []
        for i in range(126):
            code = s32(self.p.u32(base + 0x54 * i))
            info = m.call(info_fn, (WORLDMAN, code & M32))
            server = s32(m.load(info + 0x10, 4))
            words = []
            for k in range(3):
                w = m.call(word_fn, (WORLDMAN, code & M32, k))
                words.append(s32(m.load(w + 4, 4)) if w else None)
            items = [(s32(m.load(info + 0x34 + 8 * j, 4)), s32(m.load(info + 0x38 + 8 * j, 4))) for j in range(4)]
            out.append((code, server, words, items))
        return out


def diff_runs(a, b, gap=8):
    """[(offset, bytes of b)] covering every byte where b differs from a;
    runs closer than `gap` bytes are merged (the bytes between are equal in
    both, so applying the runs to a still gives b)."""
    out = []
    i, n = 0, len(a)
    while i < n:
        if a[i] != b[i]:
            j = i + 1
            last = i
            while j < n and j - last <= gap:
                if a[j] != b[j]:
                    last = j
                j += 1
            out.append((i, b[i:last + 1]))
            i = last + 1
        else:
            i += 1
    return out


def fnv(data):
    h = 0xCBF29CE484222325
    for b in data:
        h = ((h ^ b) * 0x100000001B3) & M64
    return h


def run_text(off, bs):
    """OFFSET:HEX, or OFFSET:HEX*COUNT when the bytes are a short unit repeated."""
    for unit in range(1, 9):
        if len(bs) >= 3 * unit and len(bs) % unit == 0 and bs == bs[:unit] * (len(bs) // unit):
            return f"{off:x}:{bs[:unit].hex()}*{len(bs) // unit}"
    return f"{off:x}:{bs.hex()}"


def runs_text(runs):
    return " ".join(run_text(off, bs) for off, bs in runs)


# After the Nth frame of play: the player tries the address book (3) and the
# event answers; opens the mailer (1), which it allows; reads mails 4 and 5.
DESKTOP_ACTIONS = ((3, "operate", 3), (40, "operate", 1), (45, "read", 4), (46, "read", 5), (60, "operate", 3))
FLAGSET_SEEDS = (1, 2, 3)
OPEN_SEEDS = tuple(range(100, 108))
SUB_SEEDS = (100, 101, 102)
PLAY_SEEDS = (200, 201, 202)


def write_fixture(out):
    g = GameEvents()
    w = out.write
    w("# tools/test_event_vm.py fixture: the game's own event code in tools/eemu.py.\n")
    w("# Numbers only: offsets and bytes (hex), seeds, event numbers, frames.\n")
    w("# init OFFSET:BYTES...       the boot's save (constructor, Init(1)), its non-zero runs (BYTES*N: N times),\n")
    w("#                            but for timeIdolRankStr (+0x6775, 200 bytes: text, which the Rust side reads\n")
    w("#                            from the disc)\n")
    w("# area CODE SERVER W0 W1 W2 ITEM COUNT x4   eventAreaInfo as the gate code reads it (- no word)\n")
    w("# flagset 0 EVENT OFFSET:BYTES...  ccEventFlagSet(EVENT) on the boot save: every byte it changed\n")
    w("# flaghash SEED EVENT HASH MNG  the same on randomise_save(SEED): FNV-1a 64 of the save after,\n")
    w("#                            and of the event manager's registers (mng_digest)\n")
    textless = bytearray(g.fresh)
    textless[IDOL_TEXT[0]:IDOL_TEXT[1]] = bytes(IDOL_TEXT[1] - IDOL_TEXT[0])
    runs = diff_runs(bytes(SAVE_SIZE), bytes(textless))
    for k in range(0, len(runs), 8):
        w("init " + runs_text(runs[k:k + 8]) + "\n")
    for code, server, words, items in g.story_areas():
        ws = " ".join("-" if v is None else str(v) for v in words)
        its = " ".join(f"{a} {b}" for a, b in items)
        w(f"area {code} {server} {ws} {its}\n")
    w("# open SEED EVENT HEX        CheckOpen at lv 2 for the header (bit 0) and each block (bit b + 1),\n")
    w("#                            on randomise_save(SEED) in World(SEED)\n")
    w("# cond SEED COUNT HEX        every condition of every script alone (event order, header then\n")
    w("#                            blocks), bit i the i-th, same save and world\n")
    events = g.events()
    for seed in (0,) + FLAGSET_SEEDS:
        base = bytearray(g.fresh)
        if seed:
            randomise_save(base, seed)
        base = bytes(base)
        for n, _ in events:
            after = g.flag_set(n, base)
            if seed:
                w(f"flaghash {seed} {n} {fnv(after):016x} {g.mng_digest():016x}\n")
            else:
                w(f"flagset {seed} {n} {runs_text(diff_runs(base, after))}".rstrip() + "\n")
    w("# startth SEED HASH          ccStartThEvent on randomise_save(SEED): FNV-1a 64 of the save after\n")
    w("# startev VOL FLAG HASH      ccStartEvent(VOL, FLAG) on the boot save: FNV-1a 64 of the save after\n")
    for seed in FLAGSET_SEEDS:
        base = bytearray(g.fresh)
        randomise_save(base, seed)
        g.set_save(bytes(base))
        g.init_mng()
        g.m.call(g.sym("ccStartThEvent__Fv"), ())
        w(f"startth {seed} {fnv(g.save()):016x}\n")
    for vol in (1, 2, 3, 4):
        for flag in (0, 1):
            g.set_save(g.fresh)
            g.init_mng()
            g.m.call(g.sym("ccStartEvent__Fii"), (vol, flag))
            w(f"startev {vol} {flag} {fnv(g.save()):016x}\n")
    stories = g.story_areas()
    g.hook_world()
    for seed in OPEN_SEEDS:
        base = bytearray(g.fresh)
        randomise_save(base, seed)
        world = World(seed, stories)
        for n, va in events:
            g.set_save(bytes(base))
            g.init_mng()
            g.set_world(world)
            bits = g.open_results(n, va)
            v = sum(1 << i for i, x in enumerate(bits) if x)
            w(f"open {seed} {n} {v:x}\n")
    w("# desktop phases P...        a new game reaching the desktop (desktop_run): the phase after each frame\n")
    w("# desktop call FRAME NAME ARGS...   each call a played block made outside the event code, and\n")
    w("#                            the desktop's check_operate OPERATION RESULT (DESKTOP_ACTIONS)\n")
    w("# desktop save STAGE FRAME OFFSET:BYTES...  the save when phase STAGE settled (4: at the end),\n")
    w("#                            as runs over the boot save\n")
    g.hook_play()
    snaps, phases, calls = g.desktop_run(actions=DESKTOP_ACTIONS)
    w("desktop phases " + " ".join(map(str, phases)) + "\n")
    for frame, name, args in calls:
        w(f"desktop call {frame} {name} {' '.join(map(str, args))}".rstrip() + "\n")
    for stage, frame, img in snaps:
        w(f"desktop save {stage} {frame} {runs_text(diff_runs(g.fresh, img))}\n")
    w("# sub SEED EVENT FRAMES OFFSET:BYTES...  eventSub(EVENT) played to its end (hook_sub) on\n")
    w("#                            randomise_save(SEED) in World(SEED) with menus closed and input held:\n")
    w("#                            the frames it took and every byte of the save it changed\n")
    g.hook_sub()
    sub_blocks = 0
    for seed in SUB_SEEDS:
        base = bytearray(g.fresh)
        randomise_save(base, seed)
        base = bytes(base)
        world = World(seed, stories)
        world.menu = -1
        world.pad = M32
        for n, va in events:
            g.set_save(base)
            g.init_mng()
            g.set_world(world)
            m = g.m
            m.store(SYS + 0x2cc, 4, M32)
            m.store(SYS + 0x2b1, 1, 0xff)
            frames = g.sub_run(n)
            after = g.save()
            ran = struct.unpack_from("<Q", after, EVENT_FLAG + 8 * n)[0] & ~struct.unpack_from("<Q", base, EVENT_FLAG + 8 * n)[0]
            sub_blocks += bin(ran & ((1 << 62) - 1)).count("1")
            w(f"sub {seed} {n} {frames} {runs_text(diff_runs(base, after))}".rstrip() + "\n")
    print(f"sub: {sub_blocks} blocks marked as run", file=sys.stderr)
    w("# play SEED EVENT F,F,... HASH MNG  every block played in turn at lv 2 (play_blocks), same setup\n")
    w("#                            as sub: the frames each block took, the save's and mng_digest's hash\n")
    for seed in PLAY_SEEDS:
        base = bytearray(g.fresh)
        randomise_save(base, seed)
        base = bytes(base)
        world = World(seed, stories)
        world.menu = -1
        world.pad = M32
        for n, va in events:
            if not g.has_messages(n):
                continue
            g.set_save(base)
            g.init_mng()
            g.set_world(world)
            g.m.store(SYS + 0x2cc, 4, M32)
            g.m.store(SYS + 0x2b1, 1, 0xff)
            frames = g.play_blocks(n, va)
            w(f"play {seed} {n} {','.join(map(str, frames)) or '-'} {fnv(g.save()):016x} {g.mng_digest():016x}\n")
    conds = g.condition_instances()
    for seed in OPEN_SEEDS:
        base = bytearray(g.fresh)
        randomise_save(base, seed)
        g.set_save(bytes(base))
        g.init_mng()
        g.set_world(World(seed, stories))
        v = sum(1 << i for i, (n, sh) in enumerate(conds) if g.condition(n, sh))
        w(f"cond {seed} {len(conds)} {v:x}\n")
    return 0


class EventVmTest(unittest.TestCase):
    """The harness itself: the game's code runs and says what is expected.
    The comparison with the port is crates/piney-event/tests/vm.rs."""

    @classmethod
    def setUpClass(cls):
        if not os.path.exists(ELF):
            raise unittest.SkipTest("no extracted Infection disc")
        cls.g = GameEvents()

    def test_boot_save_is_initialised(self):
        img = self.g.fresh
        self.assertEqual(struct.unpack_from("<hbb", img, ITEMS), (-1, -1, 0))
        self.assertEqual(struct.unpack_from("<h", img, MAIL_ORDER)[0], -1)
        self.assertEqual(struct.unpack_from("<h", img, SKILLS)[0], -1)

    def test_flag_set_event_1(self):
        """The opening, replayed: mails 4 and 5 read, its blocks marked."""
        out = self.g.flag_set(1, self.g.fresh)
        self.assertEqual(out[MAIL + 4], 4)
        self.assertEqual(out[MAIL + 5], 4)
        flags = struct.unpack_from("<Q", out, EVENT_FLAG + 8)[0]
        self.assertTrue(flags & (1 << 63))
        # Block 1 is repeatable, so only blocks 0 and 2 mark themselves.
        self.assertEqual(flags & 7, 5)

    def test_new_game_desktop(self):
        """A new game on the desktop: the opening plays at phase 0 and
        delivers mails 4, 5 and 320; the address book is held back until
        mails 4 and 5 are read, and then the event closes."""
        g = GameEvents()
        g.hook_play()
        snaps, phases, calls = g.desktop_run(actions=DESKTOP_ACTIONS)
        first = snaps[0][2]
        self.assertEqual(struct.unpack_from("<3h", first, MAIL_ORDER), (4, 5, 320))
        self.assertEqual((first[MAIL + 4], first[MAIL + 5], first[MAIL + 320]), (1, 1, 1))
        self.assertEqual(phases[snaps[0][1] - 1], 1)
        ops = [(name, args) for _, name, args in calls if name == "check_operate"]
        self.assertEqual(ops, [("check_operate", (3, 0)), ("check_operate", (1, 1)), ("check_operate", (3, 1))])
        last = snaps[-1][2]
        self.assertTrue(struct.unpack_from("<Q", last, EVENT_FLAG + 8)[0] & (1 << 63))


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "fixture":
        with open(FIXTURE, "w") as f:
            sys.exit(write_fixture(f))
    unittest.main()
