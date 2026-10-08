#!/usr/bin/env python3
"""crates/piney-stream against the game's own stream code run in tools/eemu.py.

The game's functions run in eemu's VuMachine (tools/test_anim.py: VU0
macro mode, lqc2/sqc2, the MMI the matrix code reaches), with operator new
and ccMalloc a bump allocator, ccRingBufferTh::OpenData pointing a reader
at a file already in memory, and the threads, CD and GS calls hooked:

  - the table lookup and file list: ccStreamInit (0x00198ee0) and
    ccStreamLoadPlay::RequestStrPlay (0x00198fc0) with searchPreLoad
    native, for all 134 streams (140 from Mutation on: `tables` writes a
    later volume's) in both languages: the STREAMDATA list,
    the ccsTbl the decoder plays and the files read whole first
    (ccLoadStreamOnMem's arguments, in order);
  - WaitEnd (0x00197f40): whether a push skips, over the file flags, the
    game mode, DESKTOP_FLG and the pushes;
  - the scenes of streams 0 (title1_st1), 106 (str6100), 2 (str0001,
    with str0001e loaded first), 15 (str0580, after str0580e; its
    texture uploads and their DMA stubbed) and 18 (str9102, after the
    desktop's resident str8000e, str8001e and str8800e from DATA.BIN, and
    str9102e): ccStream::DecodeSetup and InitScene run
    natively on the files; the draw list (every ccObj in the order
    ccStreamDrawLayerList::Draw walks, its layer and parent); then the
    frames as PlaySceneMain steps them - DecodeFrameSection, then
    SetMatrix_PosRotXYZDebug / ccView::SetView and
    ccStreamDrawLayerList::Draw, all native, ccModel::Draw hooked to record
    each object drawn (its name, transparency and lwMatrix, bit for bit),
    ccDrawEnv::SetLightMatrix run natively at the first object drawn; the
    frame number, the state, the notes and the view's world_screen each
    frame. Stream 2 runs its first 60 frames (the file is 38 MB), stream
    15 its first 420 (the file in the heap), stream 6 (str0130, after
    str0130e) its first 60, stream 18 the whole of it.

  - effects: stream 2's effect task, Func_str0001 (0x001885d0), run
    natively over the whole stream as the game's task runs it: one pass of
    its loop a game frame, ccTscb::Breath hooked to end the frame. The
    inputs are the scene's: the 0x8010 cues (str0001's F_Note records, each
    queued in eventExNoteTbl as ccSetStreamDemoNote queues it, on the step
    that draws the frame its Top starts), frameNow as PlaySceneMain leaves
    it when the task runs (the next frame, held at frameEnd), state 0x40
    until DelSceneObject (the first frame after the last gap frame); the
    scene's objects and LYR_jm (layer 8) stand in for GetSubstAdrsF. Every
    other function runs natively: ccRasterNoize, ccBufferSampling,
    ccScFade, ccMakePacketDrawBuffTrans, rand (seeded 1, newlib's), the
    ccLayer lists, with ccDrawPacket::SearchFreeWork a bump allocator. Each
    frame the DMA chains of every layer (ccLayer::root, drawn in ascending
    priority, each list as the DMA walks it) are decoded - VIF DIRECT, GIF
    PACKED and A+D - through a small GS register model into one line per
    primitive: its layer, kind, texture (a frame-buffer copy the context-2
    sprite made just before, the previous picture, ...), blend, tests,
    scissor and vertices in GS units. Step 0 is the task's first pass, in
    the frame before the scene's first.

  - effects str0300 / str0120: stream 10's Func_str0300 and stream 5's
    Func_str0120 the same way, each pass's lines hashed (FNV-1a 64;
    STREAM_FULL=1 writes them), with effHitMarkStr hooked (each cue's
    object a ccObj at its index on x, so a mark names it) and the view's
    divZ recorded.

  - subtitles: ccEventStream(num, 1) (0x001b5670) itself, over each
    stream's own notes (ccSetStreamDemoNote), with ccMessage's Change,
    Close and Disp, ccGetStreamDemoMsg and ccKanjiStrSeparate the game's;
    per step the draws Disp makes (cells, lines, the send), hashed.

  - the gate hack's stream: ccStreamLoadPlay::RequestStrPlayGH
    (0x0019a0e0) on stream 107 for every town, the fields str7000Out
    lists and others, crisis or not, both languages: the ccsTbl it builds
    from str7000Tbl* and the files it reads whole first.

  - music: ccSndStreamCtrl (0x0017d190), ccSndStreamSE / ccSndStreamBGM
    and strSeInit between tools/sound_ee.py's steps; what reaches the IOP.

    python3 tools/test_stream_rs.py fixture    write crates/piney-stream/tests/stream_fixture.txt
    PINEY_VOLUME=mutation python3 tools/test_stream_rs.py tables
                                               write .../tests/stream_tables_mut.txt (_out, _qua)
    python3 tools/test_stream_rs.py effects    write crates/piney-stream/tests/str0001_fixture.txt
    python3 tools/test_stream_rs.py effects str0300   (str0120)  write .../tests/str0300_fixture.txt
    python3 tools/test_stream_rs.py subtitles  write crates/piney-stream/tests/subtitle_fixture.txt
                                    subtitles --full NUM   print one stream's draws in full
    python3 tools/test_stream_rs.py cameras    write crates/piney-desktop/tests/camera_fixture.txt
    python3 tools/test_stream_rs.py music      write crates/piney-audio/tests/stream_fixture.txt
    python3 tools/test_stream_rs.py gatehack   write crates/piney-stream/tests/gate_hack_fixture.txt
    python3 tools/test_stream_rs.py            print this

The Rust tests (crates/piney-stream/tests/stream.rs, str0001.rs,
effects.rs, subtitles.rs; crates/piney-audio/tests/stream.rs) read the
fixtures and the disc and compare the port.
"""

import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ELF = volume.ELF
ISO = volume.ISO
FIXTURE = os.path.join(ROOT, "crates", "piney-stream", "tests", "stream_fixture.txt")
EFFECT_FIXTURE = os.path.join(ROOT, "crates", "piney-stream", "tests", "str0001_fixture.txt")
GATE_FIXTURE = os.path.join(ROOT, "crates", "piney-stream", "tests", "gate_hack_fixture.txt")

M32 = 0xFFFFFFFF
SYS, SCRATCH, PAD_SYS = 0x00F00000, 0x00F10000, 0x00F80000
DATA = inf_va(0x00400000)
HEAP0, HEAP_END = 0x01000000, 0x01F00000
PACKET_ARENA = 0x01D00000      # effects(): a pass's packet work, reused each pass
GAME, SAVE = 0x00F90000, 0x00FA0000

# (stream, the scene files in load order, frames to run: None for all).
# Stream 15's first scene, str0580, to frame 420: through the stretch where
# its ground is hidden (docs/engine/stream.md, "Stream 15's opening").
# Stream 6's first 60 frames: its models' three lights depend on the
# group's priority order (distant lights last).
# Stream 18 (str9102, the Audio screen's Movie 16) over the desktop's
# resident STR8000E, STR8001E and STR8800E (ccSetFileListDesktop), loaded
# first as ccStreams: its `#` objects (Kite, the backdrop, the bracelet's
# rings) are theirs.
SCENES = [(0, ["title1_st1"], None), (106, ["str6100"], None), (2, ["str0001e", "str0001"], 200),
          (15, ["str0580e", "str0580"], 420), (6, ["str0130e", "str0130"], 60),
          (18, ["str8000e", "str8001e", "str8800e", "str9102e", "str9102"], None)]
# The SCENES files that are DATA.BIN's, not the stream's.
RESIDENT = ("str8000e", "str8001e", "str8800e")
# The streams the volume's tables number: 134 on Infection, 140 from
# Mutation on (six new before str6100; docs/engine/stream.md).
STREAMS = 134 if volume.NAME == "infection" else 140
# Another volume's table lookup alone (`tables`): stream_tables_mut.txt,
# _out, _qua; Infection's is in stream_fixture.txt.
TABLES_FIXTURE = os.path.join(ROOT, "crates", "piney-stream", "tests",
                              f"stream_tables_{volume.CARRY_FILES.get(volume.NAME, 'inf').rstrip('_')}.txt")


def fnv(data, h=0xCBF29CE484222325):
    for b in data:
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def machine_class():
    """test_anim's VuMachine plus VU0's integer registers and data memory:
    viadd, viaddi, vlqi, vsqi, vlqd, vsqd and cfc2 / ctc2 of vi0-15, which
    ccCoord::_SetLWMatrix walks its parent chain with."""
    import test_anim
    base = test_anim.machine_class()

    class Vu0Machine(base):
        def __init__(self, program):
            super().__init__(program)
            self.vi = [0] * 16
            self.vumem = bytearray(4096)

        def vset_i(self, i, v):
            if i & 15:
                self.vi[i & 15] = v & 0xFFFF

        def qword(self, a):
            a = (a & 0xFF) * 16
            return [struct.unpack_from("<I", self.vumem, a + 4 * k)[0] for k in range(4)]

        def store_qword(self, a, mask, vals):
            a = (a & 0xFF) * 16
            for k in range(4):
                if mask & (8 >> k):
                    struct.pack_into("<I", self.vumem, a + 4 * k, vals[k] & M32)

        def cop2(self, pc, w):
            rs, rt, rd = (w >> 21) & 31, (w >> 16) & 31, (w >> 11) & 31
            if rs == 0x02:                                   # cfc2 rt, vi(rd)
                self.set(rt, self.vi[rd] if rd < 16 else 0)
                return None
            if rs == 0x06:                                   # ctc2 rt, vi(rd)
                if rd < 16:
                    self.vset_i(rd, self.r[rt])
                return None
            if rs & 0x10:
                funct, dest = w & 63, (w >> 21) & 0xF
                it, is_, id_ = rt & 15, rd & 15, (w >> 6) & 15
                if funct == 0x30:                            # viadd
                    self.vset_i(id_, self.vi[is_] + self.vi[it])
                    return None
                if funct == 0x32:                            # viaddi
                    imm = (w >> 6) & 31
                    self.vset_i(it, self.vi[is_] + (imm - 32 if imm & 16 else imm))
                    return None
                if funct >= 0x3C:
                    code = ((w >> 6) & 31) << 2 | (w & 3)
                    if code == 0x34:                         # vlqi vf(ft), (vi(fs)++)
                        self.vset(rt, dest, self.qword(self.vi[rd & 15]))
                        self.vset_i(rd, self.vi[rd & 15] + 1)
                        return None
                    if code == 0x35:                         # vsqi vf(fs), (vi(ft)++)
                        self.store_qword(self.vi[rt & 15], dest, self.vf[rd])
                        self.vset_i(rt, self.vi[rt & 15] + 1)
                        return None
                    if code == 0x36:                         # vlqd vf(ft), (--vi(fs))
                        self.vset_i(rd, self.vi[rd & 15] - 1)
                        self.vset(rt, dest, self.qword(self.vi[rd & 15]))
                        return None
                    if code == 0x37:                         # vsqd vf(fs), (--vi(ft))
                        self.vset_i(rt, self.vi[rt & 15] - 1)
                        self.store_qword(self.vi[rt & 15], dest, self.vf[rd])
                        return None
            return super().cop2(pc, w)

    return Vu0Machine


class Game:
    def __init__(self):
        from image import Program
        self.prog = Program(ELF)
        self.m = machine_class()(self.prog)
        self.sym = lambda n: self.prog.symbol_named(n).value  # noqa: E731
        self.heap = HEAP0
        m = self.m
        for name in ("ccMalloc__FUi", "__nw__FUi", "ccMemalign__FUiUi", "__nwa__FUi"):
            m.hooks[self.sym(name)] = self.malloc
        m.hooks[self.sym("ccMemalignB__FUiUi")] = lambda mm, al, n, *a: self.malloc(mm, n)
        # _ccMalloc(align, size, ...)
        m.hooks[self.sym("_ccMalloc__FUiUiUii")] = lambda mm, al, n, *a: self.malloc(mm, n)
        for name in ("ccFree__FPv", "__dl__FPv", "__dla__FPv", "_ccFree__FPv"):
            m.hooks[self.sym(name)] = lambda mm, *a: 0
        m.hooks[self.sym("OpenData__14ccRingBufferThFPv")] = self.open_data
        for name in ("CreateSema", "DeleteSema", "SignalSema", "WaitSema", "iSignalSema", "DIntr", "EIntr"):
            m.hooks[self.sym(name)] = lambda mm, *a: 1
        m.store(self.sym("ccSys"), 4, SYS)
        m.store(SYS + 604, 4, SCRATCH)
        # ccSystem::SetScreenMode(512, 448, 0) and SetScreenModeMain's
        # screenAspect = 1.3333334 * H / W, which the views' matrices use.
        import eemu
        m.store(SYS + 0x0c, 2, 512)
        m.store(SYS + 0x0e, 2, 448)
        m.store(SYS + 0x14, 4, eemu.f_div(eemu.f_mul(0x3FAAAAAB, eemu.f_from_int(448)), eemu.f_from_int(512)))
        m.store(self.sym("game"), 4, GAME)
        m.store(self.sym("saveData"), 4, SAVE)

    def malloc(self, m, n, *_):
        a = self.heap
        self.heap = (self.heap + n + 15) & ~15
        if self.heap > HEAP_END:
            raise RuntimeError("heap exhausted")
        m.mem[a:a + n] = bytes(n)
        return a

    def open_data(self, m, reader, adrs, *_):
        m.store(reader + 36, 4, adrs)
        m.store(reader + 40, 4, 0x01FF0000)
        return 0

    def call(self, name, args, limit=2_000_000_000):
        return self.m.call(self.sym(name), args, limit=limit)

    def cstr(self, a):
        out = bytearray()
        while self.m.mem[a]:
            out.append(self.m.mem[a])
            a += 1
        return out.decode("latin-1")

    def f32(self, a):
        return struct.unpack_from("<f", self.m.mem, a)[0]


# The table lookup -------------------------------------------------------------

def tables(g):
    m = g.m
    out = []
    loads = []
    tscb = g.malloc(m, 0x54)
    m.store(tscb + 0x18, 4, 1)
    hooks = {
        "StInit__6ccCdvdFv": 0, "ccStartThread__FPFPv_vii": tscb, "WaitEnd__16ccStreamLoadPlayFv": 0,
        "ccDeleteThread__FP6ccTscb": 0, "__dt__8ccStreamFv": 0, "ccLoadStreamOnMemDelete__FP18ccStreamLoadOnmTbl": 0,
        "Init__7ccLayerFsP6ccView": 0, "__ct__16ccDrawPacketCtrlFv": 0,
        "__dt__7ccLayerFv": 0,
    }
    for name, v in hooks.items():
        m.hooks[g.sym(name)] = (lambda v: lambda mm, *a: v)(v)
    # RequestStrPlay clears the Read and Decode tasks' done words and waits
    # for them: a breath finishes them.
    m.hooks[g.sym("ccBreathThread__Fi")] = lambda mm, *a: m.store(tscb + 0x18, 4, 1) or 0
    m.hooks[g.sym("ccLoadStreamOnMem__16ccStreamLoadPlayFP10STREAMDATAPi")] = \
        lambda mm, this, sd, *a: loads.append(sd) or 0
    m.store(g.sym("ccCd"), 4, g.malloc(m, 0x200))
    m.store(g.sym("ccSnd"), 4, g.malloc(m, 0x200))
    mark = g.heap
    for english in (0, 1):
        m.store(SAVE + 0x842c, 1, english)
        for num in range(STREAMS):
            del loads[:]
            g.heap = mark
            slp = g.call("ccStreamInit__Fi", (num,))
            sd = m.load(slp + 0x1f8, 4)
            g.call("RequestStrPlay__16ccStreamLoadPlayFi", (slp, 0))
            rec = lambda a: (g.cstr(m.load(a, 4)), struct.unpack_from("<iihhI", m.mem, a + 4))  # noqa: E731
            hname, (hofs, hsize, htype, hflag, hgz) = rec(sd)
            scenes = []
            for i in range(struct.unpack_from("<h", m.mem, slp + 0x1c0)[0]):
                t = slp + 0x40 + 24 * i
                typ, flag = struct.unpack_from("<hh", m.mem, t + 8)
                gz, ofs, size = struct.unpack_from("<Iii", m.mem, t + 12)
                scenes.append(f"{g.cstr(m.load(t, 4))}:{typ}:{flag}:{gz}:{ofs}:{size}")
            pre = [g.cstr(m.load(a, 4)) for a in loads]
            out.append(f"table {num} {english} {hname} {hofs} {hsize} {htype} {hflag} {hgz} "
                       f"scenes {' '.join(scenes) or '-'} preload {' '.join(pre) or '-'}")
    for name in list(hooks) + ["ccBreathThread__Fi"]:
        del m.hooks[g.sym(name)]
    return out


# The gate hack's stream ---------------------------------------------------------

GATE_FIELDS = [19, 22, 23, 25, 27, 16, 44, 45, 46, 48, 50, 52, 66, 71, 73, 74, 76, 77, 91, 100, 101, 102, 103,
               104, 105, 106, 108, 14, 0, -1, 1, 200]


def gate_hack(g):
    """RequestStrPlayGH (0x0019a0e0) natively on stream 107's
    ccStreamLoadPlay (ccStreamInit(107)), for towns -1..4, the fields
    str7000Out lists and some it does not, in and out of the crisis, in both
    languages: the ccsTbl it builds and the files it reads whole
    (ccLoadStreamOnMem's STREAMDATA, in order). The threads, the stream
    buffers, WaitEnd and the teardown are hooked."""
    m = g.m
    out = []
    loads = []
    tscb = g.malloc(m, 0x54)
    m.store(tscb + 0x18, 4, 1)
    th = g.malloc(m, 0x40)
    hooks = {
        "ccStartThread__FPFPv_vii": tscb, "WaitEnd__16ccStreamLoadPlayFv": 0, "ccDeleteThread__FP6ccTscb": 0,
        "__dt__8ccStreamFv": 0, "ccLoadStreamOnMemDelete__FP18ccStreamLoadOnmTbl": 0,
        "SetStreamBuffer__14ccStreamReaderFPvisPFv_ss": 0, "GetThreadId": 1, "ccCheckThreadMoving__Fi": th,
        "__dt__8ccUngzipFv": 0, "__dt__14ccStreamReaderFv": 0, "StInit__6ccCdvdFv": 0,
        "Init__7ccLayerFsP6ccView": 0, "__ct__16ccDrawPacketCtrlFv": 0, "__dt__7ccLayerFv": 0,
        "PollSema": 0,
    }
    for name, v in hooks.items():
        m.hooks[g.sym(name)] = (lambda v: lambda mm, *a: v)(v)
    m.hooks[g.sym("ccBreathThread__Fi")] = lambda mm, *a: m.store(tscb + 0x18, 4, 1) or 0
    m.hooks[g.sym("ccLoadStreamOnMem__16ccStreamLoadPlayFP10STREAMDATAPi")] = \
        lambda mm, this, sd, *a: loads.append(sd) or 0
    m.store(g.sym("ccCd"), 4, g.malloc(m, 0x200))
    m.store(g.sym("ccSnd"), 4, g.malloc(m, 0x200))
    mark = g.heap
    rec = lambda a: f"{g.cstr(m.load(a, 4))}:{':'.join(map(str, struct.unpack_from('<hhIii', m.mem, a + 8)))}"  # noqa: E731
    for english in (0, 1):
        m.store(SAVE + 0x842c, 1, english)
        for crisis in (0, 1):
            m.store(SAVE + 0x6772, 1, crisis)
            for town in range(-1, 5):
                for field in GATE_FIELDS:
                    del loads[:]
                    g.heap = mark
                    slp = g.call("ccStreamInit__Fi", (107,))
                    g.call("RequestStrPlayGH__16ccStreamLoadPlayFii", (slp, town & M32, field & M32))
                    n = struct.unpack_from("<h", m.mem, slp + 0x1c0)[0]
                    scenes = [rec(slp + 0x40 + 24 * i) for i in range(n)]
                    pre = [g.cstr(m.load(a, 4)) for a in loads]
                    out.append(f"gate {english} {crisis} {town} {field} "
                               f"scenes {' '.join(scenes)} preload {' '.join(pre)}")
    for name in list(hooks) + ["ccBreathThread__Fi"]:
        del m.hooks[g.sym(name)]
    return out


def gate_fixture():
    lines = gate_hack(Game())
    with open(GATE_FIXTURE, "w") as f:
        f.write("# python3 tools/test_stream_rs.py gatehack: RequestStrPlayGH's ccsTbl and preloads\n")
        f.write("\n".join(lines) + "\n")
    print(f"{GATE_FIXTURE}: {len(lines)} lines")


# WaitEnd ------------------------------------------------------------------------

def skips(g):
    m = g.m
    out = []
    slp = g.malloc(m, 0x334)
    state = {}

    def breathe(mm, *a):
        m.store(slp + 0x1c2, 2, 2)
        m.store(slp + 0x1c6, 2, 2)
        return 0
    m.hooks[g.sym("ccBreathThread__Fi")] = breathe
    for mode in (2, 3, 5):
        for desk in (0, 1):
            for flag in (0, 0x40, 0x80, 0xc0, 0x10):
                for push in (0, 0x20, 0x40, 0x800, 0x820, 0x10):
                    m.mem[slp:slp + 0x334] = bytes(0x334)
                    m.store(slp + 0x1c0, 2, 2)                  # ccsNum
                    m.store(slp + 0x1c2, 2, 1)                  # readNum
                    m.store(slp + 0x1c6, 2, 0)                  # decodeNum
                    m.store(slp + 0x40 + 0x0a, 2, flag)         # ccsTbl[0].flag
                    m.store(slp + 0x258, 4, 1)                  # a scene is playing
                    m.store(GAME, 4, mode)
                    m.store(g.sym("DESKTOP_FLG"), 1, desk)
                    m.store(SYS + 0x2d0, 4, push)
                    m.store(SAVE + 0x8410, 2, 0x20)
                    g.call("WaitEnd__16ccStreamLoadPlayFv", (slp,))
                    state[(mode, desk, flag, push)] = m.mem[slp + 0x261]
                    out.append(f"skip {mode} {desk} {flag} {push} 32 {m.mem[slp + 0x261]}")
    del m.hooks[g.sym("ccBreathThread__Fi")]
    return out


# Scenes ------------------------------------------------------------------------

_ARCHIVES = {}


def member(name):
    """A STREAM archive member, inflated (the archives read once); else
    DATA.BIN's (the streams' common files, `str8000e` and the others)."""
    import gzarc
    import iso as isomod
    for arc in ("STREAM/STRCMN.BIN", "STREAM/STR1.BIN"):
        if arc not in _ARCHIVES:
            disc = isomod.Iso(ISO)
            e = disc.find(arc)
            data = disc.read(e.lba, e.size)
            _ARCHIVES[arc] = (data, {mb.name.lower(): mb for mb in gzarc.members(data)})
        data, byname = _ARCHIVES[arc]
        mb = byname.get(name + ".tmp")
        if mb:
            return gzarc.inflate(data, mb)
    if "DATA.BIN" not in _ARCHIVES:
        data = gzarc.open_bytes(volume.DATA)
        _ARCHIVES["DATA.BIN"] = (data, {mb.name.lower(): mb for mb in gzarc.members(data)})
    data, byname = _ARCHIVES["DATA.BIN"]
    if name + ".cmp" in byname:
        return gzarc.inflate(data, byname[name + ".cmp"])
    raise KeyError(name)


def word(name):
    """An object name as one word of a fixture line: '%' and ' ' escaped
    (Kite's bones in str0580 are "EXT_t0 r belt3" and the like)."""
    return name.replace("%", "%25").replace(" ", "%20")


def run_scene(g, num, stems, frames):
    m = g.m
    out = []
    # The texture uploads' GS packets and DMA (str0580e's textures; the
    # machine has no MMI por, no DMA registers): not part of the draw list.
    for name in ("sceGsSetDefLoadImage", "sceGsExecLoadImage", "sceGsSyncPath", "FlushCache", "sceDmaGetChan",
                 "sceDmaReset", "sceDmaPutEnv", "sceDmaSend", "sceDmaSendN", "sceDmaSendI", "sceDmaSync",
                 "sceDmaWatch", "sceDmaPause", "sceDmaRestart"):
        m.hooks[g.sym(name)] = lambda mm, *a: 0
    datas = [member(s) for s in stems]
    at = DATA
    streams = []
    for s, data in zip(stems, datas):
        if frames is not None and s == stems[-1]:
            data = truncate(data, frames)
        # The files lie below ccSys and the other fixed blocks (SYS on); one
        # that does not fit there goes in the heap.
        if at + len(data) > SYS:
            at = (g.malloc(m, len(data) + 0xFFF) + 0xFFF) & ~0xFFF
        m.mem[at:at + len(data)] = data
        ccs = g.malloc(m, 0x190)
        g.call("__ct__8ccStreamFP14ccStreamReader", (ccs, 0))
        m.store(ccs + 0x160, 4, at)
        g.call("DecodeSetup__8ccStreamFv", (ccs,))
        streams.append(ccs)
        at = (at + len(data) + 0xFFF) & ~0xFFF
    ccs = streams[-1]
    stem = stems[-1]
    layer = g.malloc(m, 0x40)
    g.call("Init__7ccLayerFsP6ccView", (layer, 0, 0))
    view = m.load(layer + 0x2c, 4)
    g.call("InitScene__8ccStreamFP7ccLayerP9ccDrawEnv", (ccs, layer, 0))
    # ccSetStreamDemoThread's divZ, as the port assumes.
    m.store(view + 0x25c, 4, 0x447A0000)
    idx, nidx = m.load(ccs + 0x30, 4), m.load(ccs + 0x38, 4)

    def name_of(obj):
        index = m.load(obj + 0x90, 4)
        i = (index - idx) // 64
        return g.cstr(index + 8) if 0 <= i < nidx else "?"

    dl = m.load(ccs + 0xac, 4)
    node = m.load(dl, 4)
    walk = []
    while node:
        lay = m.load(node + 4, 4)
        pri = struct.unpack_from("<h", m.mem, lay + 4)[0]
        o = m.load(node + 8, 4)
        while o:
            obj = m.load(o + 4, 4)
            if struct.unpack_from("<H", m.mem, obj + 0x8e)[0] == 0x100:
                par = m.load(obj + 0x80, 4)
                is_obj = par and struct.unpack_from("<H", m.mem, par + 0x8e)[0] == 0x100
                walk.append((pri, name_of(obj), name_of(par) if is_obj else "-"))
            o = m.load(o, 4)
        node = m.load(node, 4)
    out.append(f"scene {num} {stem} frames {m.load(ccs + 0x168, 4)} objects {len(walk)}")
    resident = [s for s in stems if s in RESIDENT]
    if resident:
        out.append(f"resident {num} {' '.join(resident)}")
    for pri, n, p in walk:
        out.append(f"draw {num} {stem} {pri} {word(n)} {word(p)}")
    drawn = []
    lights = []
    env = m.load(ccs + 0x158, 4)
    ln, lc = g.malloc(m, 64), g.malloc(m, 64)

    def model_draw(mm, model, param, *a):
        # No eemu call from inside a hook: a nested call resets the machine.
        obj = m.load(param + 0x90, 4)
        tp = m.load(param + 0x120, 4)
        drawn.append((name_of(obj), tp, bytes(m.mem[obj:obj + 64]), obj))
        return 0
    m.hooks[g.sym("Draw__7ccModelFP16ccDrawModelParam")] = model_draw
    m.hooks[g.sym("Draw__13ccShadowModelFP16ccDrawModelParam")] = lambda mm, *a: 0
    # The effect nodes' DrawNoAnm runs; its ccEff::Draw(pattern) is
    # recorded: the pattern, the place DrawNoAnm put at +0x10, the scale
    # and turn (+0x20..+0x28) and the transparency (+0x34).
    effs = []

    def eff_draw(mm, eff, pattern, *a):
        effs.append(struct.pack("<H", pattern & 0xFFFF) + bytes(m.mem[eff + 0x10:eff + 0x1c])
                    + bytes(m.mem[eff + 0x20:eff + 0x2c]) + bytes(m.mem[eff + 0x34:eff + 0x38]))
        return 0
    m.hooks[g.sym("Draw__5ccEffFUs")] = eff_draw
    # PlaySceneMain's first pass, then per frame: the draw, the next decode.
    g.call("DecodeFrameSection__8ccStreamFv", (ccs,))
    k = 0
    while True:
        k += 1
        del drawn[:]
        del lights[:]
        del effs[:]
        notes = []
        n = m.load(ccs + 0xb4, 4)
        while n:
            notes.append(f"{m.load(n + 12, 4)}:{m.load(n + 4, 4):x}:{m.load(n + 8, 4)}")
            n = m.load(n, 4)
        m.store(g.sym("active__9ccDrawEnv"), 4, env)
        m.store(g.sym("active__7ccLayer"), 4, m.load(ccs + 0x154, 4))
        cam = m.load(ccs + 0x130, 4)
        g.call("SetMatrix_PosRotXYZDebug__5ccCamFPfPfPA4_f", (cam, ccs + 0x120, ccs + 0x110, ccs + 0xc0))
        g.call("SetView__6ccViewFRC5ccCamPA4_f", (view, cam, 0))
        m.mem[env + 0x30:env + 0x40] = m.mem[ccs + 0x140:ccs + 0x150]
        g.call("Draw__21ccStreamDrawLayerListFv", (dl,))
        if drawn and m.load(env + 0x80, 4):
            nm, obj = drawn[0][0], drawn[0][3]
            m.mem[ln:ln + 64] = bytes(64)
            m.mem[lc:lc + 64] = bytes(64)
            g.call("SetLightMatrix__9ccDrawEnvFPA4_fPA4_fPf", (env, ln, lc, obj + 48))
            lights.append((nm, [g.f32(ln + 4 * k) for k in range(16)], [g.f32(lc + 4 * k) for k in range(16)]))
        now, state = m.load(ccs + 0x164, 4), struct.unpack_from("<H", m.mem, ccs + 0x17e)[0]
        h = 0xCBF29CE484222325
        for nm, tp, lw, _ in drawn:
            h = fnv(nm.encode() + struct.pack("<I", tp) + lw, h)
        out.append(f"frame {num} {stem} {k} {now} {state & 0x14:x} {len(drawn)} {h:016x} notes {' '.join(notes) or '-'}")
        ws = [g.f32(view + 0xd0 + 4 * i) for i in range(16)]
        out.append(f"camera {num} {stem} {k} " + " ".join(f"{x:.9g}" for x in ws))
        if effs:
            he = 0xCBF29CE484222325
            for e in sorted(effs):
                he = fnv(e, he)
            out.append(f"eff {num} {stem} {k} {len(effs)} {he:016x}")
        for nm, lnv, lcv in lights:
            out.append(f"light {num} {stem} {k} {word(nm)} " + " ".join(f"{x:.9g}" for x in lnv + lcv))
        if state & 0x10 or (frames is not None and k >= frames):
            break
        g.call("DecodeFrameSection__8ccStreamFv", (ccs,))
    for name in ("Draw__7ccModelFP16ccDrawModelParam", "Draw__13ccShadowModelFP16ccDrawModelParam",
                 "Draw__5ccEffFUs"):
        del m.hooks[g.sym(name)]
    return out


def truncate(data, frames):
    """The file up to the Top of frame frames + 2, then an end Top."""
    import ccs as ccsmod
    c = ccsmod.Ccs(data)
    for (p, t, n, end) in c.chunks():
        if t is not None and t & 0xFFFF == 0xFF01 and struct.unpack_from("<I", data, p + 8)[0] == frames + 2:
            return data[:p] + struct.pack("<III", 0xCCCCFF01, 1, 0xFFFFFFFF)
    return data


# Stream 2's effect task ------------------------------------------------------

# ccSystem's frame buffers as SetScreenModeMain lays them out (512 x 448
# PSMCT32, block addresses), the Z buffer after them; the page the packets
# are built on. A packet built on page P draws into fbuffAdrs[P ^ 1] once
# ccSystem::Ctrl has swapped, while fbuffAdrs[P] is displayed.
FBUFF = (0, 3584)
# effects(): the TBP a hooked ccSprite::SetTex gives its texture (the member
# drained stream's banner, xeffect's TEX_detadrain: 256 x 128, PSMT4), which
# the GS model names "ccs".
SKILL_TBP = 0x2A00
SKILL_TEX0 = SKILL_TBP | 4 << 14 | 0x14 << 20 | 8 << 26 | 7 << 30 | 1 << 34
ZBUFF = 7168
PAGE = 0
XYOFFSET = (0x7000, 0x7200)

GS_REG = {0x00: "PRIM", 0x01: "RGBAQ", 0x03: "UV", 0x05: "XYZ2", 0x06: "TEX0_1", 0x07: "TEX0_2",
          0x08: "CLAMP_1", 0x09: "CLAMP_2", 0x0d: "XYZ3", 0x14: "TEX1_1", 0x15: "TEX1_2",
          0x18: "XYOFFSET_1", 0x19: "XYOFFSET_2", 0x3b: "TEXA", 0x3f: "TEXFLUSH", 0x40: "SCISSOR_1",
          0x41: "SCISSOR_2", 0x42: "ALPHA_1", 0x43: "ALPHA_2", 0x47: "TEST_1", 0x48: "TEST_2",
          0x49: "PABE", 0x4a: "FBA_1", 0x4b: "FBA_2", 0x4c: "FRAME_1", 0x4d: "FRAME_2", 0x4e: "ZBUF_1",
          0x4f: "ZBUF_2"}


def str0001_schedule(stem="str0001", objs=None):
    """(frameEnd, {step: [0x8010 params]}) from a scene's own records: the
    Frame chunk's count - 1, and each F_Note under the Top of frame N,
    processed on the step that draws frame N. `objs`, a dict, gets each
    cue's object index (the note's first word) by (step, i)."""
    import ccs as ccsmod
    data = member(stem)
    frame_end, top, cues = None, None, {}
    for (p, t, n, end) in ccsmod.Ccs(data).chunks():
        if t is None:
            break
        kind = t & 0xFFFF
        if kind == 0x0005 and frame_end is None:
            frame_end = struct.unpack_from("<I", data, p + 8)[0] - 1
        elif kind == 0xFF01:
            top = struct.unpack_from("<i", data, p + 8)[0]
        elif kind == 0x0108:
            obj, event, param = struct.unpack_from("<III", data, p + 8)
            if event == 0x8010:
                cues.setdefault(top, []).append(param)
                if objs is not None:
                    objs[(top, len(cues[top]) - 1)] = obj
    return frame_end, cues


def dma_chain(mem, head):
    """A layer's list as the DMA walks it: ('func', (fn, arg)) for a
    ccDmaFuncTag (a call marker, ID 7 ADDR 1, then a qwc-0 'next' tag
    holding the function and argument), else ('data', words) for a 'next'
    tag: two leading placeholders, the tag's two VIF words, its payload."""
    out = []
    tag = head
    while tag:
        lo = struct.unpack_from("<Q", mem, tag)[0]
        qwc, tid, addr = lo & 0xFFFF, (lo >> 28) & 7, (lo >> 32) & 0x7FFFFFFF
        if tid == 7 and addr == 1:
            out.append(("func", struct.unpack_from("<II", mem, tag + 24)))
            tag = (struct.unpack_from("<Q", mem, tag + 16)[0] >> 32) & 0x7FFFFFFF
            continue
        if tid != 2:
            raise RuntimeError(f"DMA tag id {tid} at {tag:#x}")
        words = [None, None] + list(struct.unpack_from(f"<{2 + 4 * qwc}I", mem, tag + 8))
        out.append(("data", words))
        tag = addr
    return out


def vif_direct(words):
    """The GIF qwords VIF DIRECT / DIRECTHL carry, as (lo, hi) pairs."""
    out = []
    i = 2
    while i < len(words):
        w = words[i]
        cmd = (w >> 24) & 0x7F
        i += 1
        if cmd in (0x00, 0x10, 0x11, 0x13):                 # NOP, FLUSHE, FLUSH, FLUSHA
            continue
        if cmd not in (0x50, 0x51) or i % 4:
            raise RuntimeError(f"VIF code {w:#x}")
        for q in range(w & 0xFFFF):
            a = words[i + 4 * q:i + 4 * q + 4]
            out.append((a[0] | a[1] << 32, a[2] | a[3] << 32))
        i += 4 * (w & 0xFFFF)
    return out


def gif_writes(qwords):
    """GS register writes, PACKED mode (A+D, RGBAQ, UV, TEX0, CLAMP, XYZ2,
    XYZ3)."""
    out = []
    i = 0
    while i < len(qwords):
        lo, hi = qwords[i]
        i += 1
        nreg = (lo >> 60) & 0xF or 16
        if (lo >> 58) & 3:
            raise RuntimeError("GIF mode other than PACKED")
        if (lo >> 46) & 1:
            out.append(("PRIM", (lo >> 47) & 0x7FF))
        for _ in range(lo & 0x7FFF):
            for k in range(nreg):
                r = (hi >> (4 * k)) & 0xF
                qlo, qhi = qwords[i]
                i += 1
                if r == 0xE:
                    out.append((GS_REG.get(qhi & 0xFF, f"{qhi & 0xFF:#x}"), qlo))
                elif r == 0x1:
                    out.append(("RGBAQ", (qlo & 0xFF) | (qlo >> 32 & 0xFF) << 8 | (qhi & 0xFF) << 16
                                | (qhi >> 32 & 0xFF) << 24))
                elif r == 0x3:
                    out.append(("UV", (qlo & 0x3FFF) | (qlo >> 32 & 0x3FFF) << 16))
                elif r in (0x6, 0x7, 0x8, 0x9):
                    # TEX0_1/2, CLAMP_1/2: the low 64 bits as they are.
                    out.append((("TEX0_1", "TEX0_2", "CLAMP_1", "CLAMP_2")[r - 6], qlo))
                elif r in (0x5, 0xD):
                    out.append(("XYZ2" if r == 5 else "XYZ3", (qlo & 0xFFFF) | (qlo >> 32 & 0xFFFF) << 16
                                | (qhi & M32) << 32))
                elif r != 0xF:
                    raise RuntimeError(f"PACKED register {r}")
    return out


class GsModel:
    """Enough of the GS to turn register writes into primitives: the
    registers, a vertex queue per PRIM, and the scratch copy a context-2
    sprite leaves for the next draw to texture from."""

    KINDS = {3: "tri", 4: "strip", 5: "fan", 6: "sprite"}

    def __init__(self):
        self.reg = {}
        self.copy = None       # (tbp, x, y, w, h)
        self.lines = []

    def source(self, tbp):
        if tbp == FBUFF[PAGE ^ 1]:
            return "cur"
        if tbp == FBUFF[PAGE]:
            return "prev"
        return None

    def layer(self, pri, scissor):
        self.pri = pri
        self.reg["SCISSOR_1"] = scissor
        self.prim, self.verts, self.state = None, [], None

    def write(self, name, value):
        if name == "PRIM":
            self.flush()
            self.prim, self.verts, self.state = value, [], None
        elif name in ("XYZ2", "XYZ3"):
            self.verts.append((value, self.reg.get("UV", 0), self.reg.get("RGBAQ", 0) & M32))
            if name == "XYZ2":
                state = self.snapshot()
                if self.state is not None and state != self.state:
                    raise RuntimeError("state changed inside a primitive")
                self.state = state
        else:
            self.reg[name] = value

    def snapshot(self):
        ctx = "2" if self.prim >> 9 & 1 else "1"
        keys = ("TEX0_", "TEX1_", "CLAMP_", "ALPHA_", "TEST_", "ZBUF_", "SCISSOR_", "FRAME_", "XYOFFSET_")
        return tuple(self.reg.get(k + ctx) for k in keys)

    def flush(self):
        if self.prim is None or not self.verts:
            return
        prim, verts = self.prim, self.verts
        tex0, tex1, clamp, alpha, test, zbuf, scissor, frame, xyoff = self.state
        self.verts = []
        if prim >> 9 & 1:
            self.copy_sprite(prim, verts, tex0, tex1, scissor, frame, xyoff)
            return
        if prim & 0x120 != 0x100:
            raise RuntimeError(f"PRIM {prim:#x}: fog, AA or ST coordinates")
        tex = "none"
        if prim >> 4 & 1:
            tbp, tbw, psm = tex0 & 0x3FFF, tex0 >> 14 & 0x3F, tex0 >> 20 & 0x3F
            tw, th = 1 << (tex0 >> 26 & 0xF), 1 << (tex0 >> 30 & 0xF)
            src = self.source(tbp)
            if src is not None and tbw == 8 and psm == 0:
                tex = src
            elif tbp == SKILL_TBP:
                tex = "ccs"
            elif self.copy is not None and tbp == self.copy[0] and (tw, th) == self.copy[3:]:
                tex = "frame:{},{},{}x{}".format(*self.copy[1:])
            else:
                raise RuntimeError(f"texture at {tbp:#x}")
            def wrap(mode, lo, hi):
                # REGION_REPEAT (3): the texel coordinate ANDed with lo,
                # ORed with hi (MINU / MAXU, MINV / MAXV).
                return ("repeat", "clamp", "rclamp", "region:{},{}".format(lo, hi))[mode]
            tex += "/{}/{}/{}/{}/{}".format("linear" if tex1 >> 5 & 1 else "nearest",
                                            wrap(clamp & 3, clamp >> 4 & 0x3FF, clamp >> 14 & 0x3FF),
                                            wrap(clamp >> 2 & 3, clamp >> 24 & 0x3FF, clamp >> 34 & 0x3FF),
                                            tex0 >> 35 & 3, tex0 >> 34 & 1)
        blend = f"{alpha:#x}" if prim >> 6 & 1 else "none"
        atest = f"{test >> 1 & 7}/{test >> 4 & 0xFF}/{test >> 12 & 3}" if test & 1 else "off"
        ztest = test >> 17 & 3 if test >> 16 & 1 else 1
        zwrite = 0 if zbuf >> 32 & 1 else 1
        sc = f"{scissor & 0x7FF},{scissor >> 16 & 0x7FF},{scissor >> 32 & 0x7FF},{scissor >> 48 & 0x7FF}"
        # Texture coordinates only where the primitive is textured.
        tme = prim >> 4 & 1
        vs = " ".join(f"{(v & 0xFFFF) - XYOFFSET[0]},{(v >> 16 & 0xFFFF) - XYOFFSET[1]},{v >> 32},"
                      f"{uv & 0x3FFF if tme else '-'},{uv >> 16 & 0x3FFF if tme else '-'},{c:08x}"
                      for v, uv, c in verts)
        self.lines.append(f"L{self.pri} {self.KINDS[prim & 7]} {'gouraud' if prim >> 3 & 1 else 'flat'} "
                          f"tex={tex} blend={blend} atest={atest} ztest={ztest} zwrite={zwrite} "
                          f"scissor={sc} v={vs}")

    def copy_sprite(self, prim, verts, tex0, tex1, scissor, frame, xyoff):
        """ccMakePacketDrawBuffTrans's context-2 sprite: the draw buffer,
        point-sampled at 1:1 through RGBA 0x80, into spare VRAM."""
        (a, uva, ca), (b, uvb, cb) = verts
        w, h = (b & 0xFFFF) >> 4, (b >> 16 & 0xFFFF) >> 4
        # A band above the frame starts at a negative row, which the 14-bit
        # V wraps past texel 768 (the source repeats at 512 anyway).
        wrapped = lambda v: (v & 0x3FFF) - (0x4000 if v & 0x3FFF >= 0x3000 else 0)  # noqa: E731
        x0, y0 = wrapped(uva) >> 4, wrapped(uva >> 16) >> 4
        x1, y1 = x0 + ((uvb - uva) & 0x3FFF) // 16, y0 + ((uvb >> 16) - (uva >> 16) & 0x3FFF) // 16
        ok = (prim & 7 == 6 and prim & 0x150 == 0x110 and a & 0xFFFFFFFF == 0 and xyoff == 0
              and self.source(tex0 & 0x3FFF) == "cur" and tex1 & 0x1E0 == 0 and tex0 >> 35 & 3 == 0
              and cb & 0xFFFFFF == 0x808080 and (x1 - x0, y1 - y0) == (w, h)
              and scissor == (w - 1) << 16 | (h - 1) << 48 and frame & 0x1FF)
        if not ok:
            raise RuntimeError(f"a context-2 draw that is not a frame-buffer copy: {prim:#x} {verts} {tex0:#x} {tex1:#x} {scissor:#x} {frame:#x} {xyoff} {(x0, y0, x1, y1, w, h)}")
        self.copy = ((frame & 0x1FF) * 32, x0, y0, w, h)


# The effect tasks the port has, beyond str0001: (task, scene file, fixture).
TASKS = {
    "str0300": ("Func_str0300__FP6ccTscb", "str0300",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0300_fixture.txt")),
    "str0120": ("Func_str0120__FP6ccTscb", "str0120",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0120_fixture.txt")),
    "str0090": ("Func_str0090__FP6ccTscb", "str0090",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0090_fixture.txt")),
    "str0110": ("Func_str0110__FP6ccTscb", "str0110",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0110_fixture.txt")),
    "str0130": ("Func_str0130__FP6ccTscb", "str0130",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0130_fixture.txt")),
    "str0150": ("Func_str0150__FP6ccTscb", "str0150",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0150_fixture.txt")),
    "str0240": ("Func_str0240__FP6ccTscb", "str0240",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0240_fixture.txt")),
    "str0250": ("Func_str0250__FP6ccTscb", "str0250",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0250_fixture.txt")),
    "str0301": ("Func_str0301__FP6ccTscb", "str0301",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0301_fixture.txt")),
    "str0305": ("Func_str0305__FP6ccTscb", "str0305",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0305_fixture.txt")),
    "str0350": ("Func_str0350__FP6ccTscb", "str0350",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0350_fixture.txt")),
    "str0570": ("Func_str0570__FP6ccTscb", "str0570",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0570_fixture.txt")),
    "str0610": ("Func_str0610__FP6ccTscb", "str0610",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0610_fixture.txt")),
    "str8100": ("Func_str8000__FP6ccTscb", "str8100",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str8100_fixture.txt")),
    "str9102": ("Func_str8000__FP6ccTscb", "str9102",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str9102_fixture.txt")),
    "str9104": ("Func_str9000__FP6ccTscb", "str9104",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str9104_fixture.txt")),
    "str9101": ("Func_str9001__FP6ccTscb", "str9101",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str9101_fixture.txt")),
    "str7100": ("Func_str7100__FP6ccTscb", "str7100",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str7100_fixture.txt")),
    "str8801": ("Func_str8800__FP6ccTscb", "str8801",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str8801_fixture.txt")),
    "str0580": ("Func_str0580__FP6ccTscb", "str0580",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0580_fixture.txt")),
    "str0581": ("Func_str0581__FP6ccTscb", "str0581",
                os.path.join(ROOT, "crates", "piney-stream", "tests", "str0581_fixture.txt")),
}

# Stream 15's tasks: the objects their event tables and fog lists name
# (GetSubstAdrsF), each a ccObj standing at stand_in_pos(name), no parent;
# the four rocks `OBJ_se1_6ro*` and `EFF_x001` of str0580e.
STREAM15 = ("str0580", "str0581")
ROCKS = ("OBJ_se1_6roa", "OBJ_se1_6rob", "OBJ_se1_6roc", "OBJ_se1_6rod")


def stand_in_pos(name):
    """Where a named stand-in object stands: x the name's bytes summed
    times 10, y its length times 100, z 50."""
    return (float(sum(name.encode()) * 10), float(len(name) * 100), 50.0)


def effects(task="str0001"):
    """Stream 2's Func_str0001 (or a TASKS entry) run over its scene, every
    pass's packets decoded; see the module doc."""
    if task == "str0001":
        func, stem, out_path = "Func_str0001__FP6ccTscb", "str0001", EFFECT_FIXTURE
    else:
        func, stem, out_path = TASKS[task]
    cue_objs = {}
    frame_end, cues = str0001_schedule(stem, cue_objs)
    # The other tasks' fixtures hash each pass's lines (FNV-1a 64, each
    # line ended by a newline); STREAM_FULL=1 writes them out instead.
    hashed = task != "str0001" and not os.environ.get("STREAM_FULL")
    g = Game()
    m = g.m
    m.store(SYS + 0xBAC, 4, FBUFF[0])
    m.store(SYS + 0xBB0, 4, FBUFF[1])
    m.store(SYS + 0xBB4, 4, ZBUFF)
    m.store(SYS + 0xBC0, 1, 0)                                  # fbuffType PSMCT32
    m.store(SYS + 0xBC1, 1, 48)                                 # zbuffType PSMZ32
    m.store(SYS + 0xBC8, 8, (ZBUFF >> 5) | 48 << 24)            # zbuf_1
    m.store(SYS + 0xBD8, 1, PAGE)
    m.store(g.sym("checkSize__12ccDrawPacket"), 4, 0)
    # The packets' work: an arena of its own, emptied after each pass's
    # lists are decoded (the game's is double-buffered), so a long scene
    # does not run the heap out.
    arena = {"at": PACKET_ARENA}

    def packet_work(mm, n, *a):
        at = arena["at"]
        arena["at"] = (at + n + 15) & ~15
        if arena["at"] > HEAP_END or g.heap > PACKET_ARENA:
            raise RuntimeError("packet arena exhausted")
        mm.mem[at:at + n] = bytes(n)
        return at
    m.hooks[g.sym("SearchFreeWork__12ccDrawPacketFiRP15ccDrawPacketTagi")] = packet_work

    def new_layer(pri, view=0):
        lay = g.malloc(m, 0x40)
        g.call("__ct__16ccDrawPacketCtrlFv", (lay + 8,))
        g.call("Init__7ccLayerFsP6ccView", (lay, pri, view))
        return lay

    # sysLayer (the scene's), LYR_jm sharing its view, fontInit's layer.
    sys_layer = new_layer(0)
    view = m.load(sys_layer + 0x2C, 4)
    jm = new_layer(8, view)
    m.store(g.sym("fontLayer"), 4, new_layer(240))
    # PlaySceneMain's SetView: the feedback's Z takes view_screen.
    cam = g.malloc(m, 0x100)
    g.call("Init__5ccCamFP10ccCamChunk", (cam, 0))
    g.call("SetMatrix_PosRotXYZDebug__5ccCamFPfPfPA4_f", (cam, g.malloc(m, 16), g.malloc(m, 16), g.malloc(m, 64)))
    g.call("SetView__6ccViewFRC5ccCamPA4_f", (view, cam, 0))
    # The scene: GetSubstAdrsF's objects, its drawEnv, frameNow / frameEnd.
    names = {"LYR_jm": jm}
    for name in ("OBJ_se1_6flo1", "OBJ_se1_6bac1", "OBJ_se1_6clo1_1", "OBJ_se1_6clo1_2", "OBJ_se1_6moo1"):
        obj = g.malloc(m, 0xB0)
        model = g.malloc(m, 0x100)
        m.store(model + 0x30, 8, 0x5000D)                        # TEST_1 GEQUAL/0/FB_ONLY, GEQUAL
        m.store(obj + 0x94, 4, model)
        m.store(obj + 0xA0, 2, 0x8)                              # fogSW, as ccObj::Init leaves it
        names[name] = obj
    # A note's object (+16, the scene's ccObj for its index): one per
    # index, at the origin but for its index in x, with no parent, so
    # _SetLWMatrix leaves its lwMatrix the local one and a hit mark placed
    # at it names the index.
    note_objs = {}

    def note_obj(index):
        if index not in note_objs:
            o = g.malloc(m, 0x100)
            for r in range(4):
                m.store(o + 64 + 20 * r, 4, 0x3F800000)
            m.store(o + 64 + 48, 4, struct.unpack("<I", struct.pack("<f", float(index)))[0])
            note_objs[index] = o
        return note_objs[index]
    # The effect objects' layer (RequestStrPlay's, priority 20), which the
    # task hands effHitMarkStr; the hit marks it would start are recorded.
    marks = []
    if task != "str0001":
        str_eff = new_layer(20)
        m.store(g.sym("strEffLayer"), 4, str_eff)

        def hit_mark(mm, pos, rot, layer, *a):
            f = [mm.load(pos + 4 * i, 4) for i in range(3)] + [mm.load(rot + 4 * i, 4) for i in range(3)]
            marks.append("mark " + " ".join(f"{x:08x}" for x in f) + (" effLayer" if layer == str_eff else f" {layer}"))
            return 0
        m.hooks[g.sym("effHitMarkStr__FPfPfP7ccLayer")] = hit_mark

        def transfer(mm, pos, layer, *a):
            f = [mm.load(pos + 4 * i, 4) for i in range(3)] + [mm.f[12] & 0xFFFFFFFF]
            marks.append("transfer " + " ".join(f"{x:08x}" for x in f)
                         + (" effLayer" if layer == str_eff else f" {layer}"))
            return 0
        m.hooks[g.sym("effTransferStr__FPffP7ccLayer")] = transfer
    # Stream 15: the event objects as named stand-ins, str0580e's EFF_x001
    # and its four rocks; the parts' ccEff::Draw and ccObj::Draw recorded
    # (not drawn), in order, each pass.
    parts = []
    stand_ins = {}
    if task in STREAM15:
        f32b = lambda x: struct.unpack("<I", struct.pack("<f", x))[0]  # noqa: E731

        def stand_in(name):
            if name not in stand_ins:
                o = g.malloc(m, 0x100)
                for r in range(4):
                    m.store(o + 64 + 20 * r, 4, 0x3F800000)
                for k, v in enumerate(stand_in_pos(name)):
                    m.store(o + 64 + 48 + 4 * k, 4, f32b(v))
                m.store(o + 0xA0, 2, 0x8)                        # fogSW, as ccObj::Init leaves it
                stand_ins[name] = o
            return stand_ins[name]
        eff_stream, eff_chunk = g.malloc(m, 0x100), g.malloc(m, 0x100)
        rock_chunks = [g.malloc(m, 0x40) for _ in ROCKS]
        m.hooks[g.sym("GetCCSAdrsNE__8ccStreamFPCc")] = \
            lambda mm, name, *a: eff_stream if g.cstr(name) == "str0580e" else 0
        m.hooks[g.sym("GetChunkAdrsF__8ccStreamFPCci")] = \
            lambda mm, s, name, *a: eff_chunk if s == eff_stream and g.cstr(name) == "EFF_x001" else 0

        def chunk_search(mm, s, name, res, *a):
            if s != eff_stream or g.cstr(name) != "OBJ_se1_6ro*":
                return 0
            arr = mm.load(res, 4)
            for i, c in enumerate(rock_chunks):
                mm.store(arr + 8 * i, 4, c)
                mm.store(arr + 8 * i + 4, 4, 0)
            mm.store(res + 4, 2, len(rock_chunks))
            return 0
        m.hooks[g.sym("GetChunkAdrs__8ccStreamFPCcP19ccChunkSearchResult")] = chunk_search
        m.hooks[g.sym("Init__5ccEffFP10ccEffChunki")] = lambda mm, *a: 0
        rock_of = {}
        fake_model = g.malloc(m, 0x40)                           # no Bbox (+4 0): in view

        def obj_init(mm, obj, chunk, *a):
            mm.store(obj + 0x94, 4, fake_model)
            rock_of[obj] = rock_chunks.index(chunk)
            return 0
        m.hooks[g.sym("Init__5ccObjFP10ccObjChunkPP7ccCoordUi")] = obj_init
        m.hooks[g.sym("__dt__5ccObjFv")] = lambda mm, *a: 0

        def eff_draw(mm, eff, pos, pat, *a):
            v = [mm.load(pos + 4 * i, 4) for i in range(4)]
            f = [mm.load(eff + o, 4) for o in (0x20, 0x24, 0x34, 0x2C)]
            parts.append("eff " + " ".join(f"{x:08x}" for x in v) + f" {pat & 0xFFFF} "
                         + " ".join(f"{x:08x}" for x in f))
            return 0
        m.hooks[g.sym("Draw__5ccEffFPfUs")] = eff_draw

        def obj_draw(mm, obj, *a):
            lw = [mm.load(obj + 4 * i, 4) for i in range(16)]
            parts.append(f"rock {rock_of.get(obj, -1)} " + " ".join(f"{x:08x}" for x in lw)
                         + f" {mm.f[12] & 0xFFFFFFFF:08x}")
            return 0
        m.hooks[g.sym("Draw__5ccObjFf")] = obj_draw
    ccs = g.malloc(m, 0x190)
    env = g.malloc(m, 0x100)
    m.store(ccs + 0x154, 4, sys_layer)
    m.store(ccs + 0x158, 4, env)
    m.store(ccs + 0x164, 4, 1)
    m.store(ccs + 0x168, 4, frame_end)
    m.store(ccs + 0x17E, 2, 0x48)
    if task in STREAM15:
        m.hooks[g.sym("GetSubstAdrsF__8ccStreamFPCci")] = \
            lambda mm, this, name, *a: names.get(g.cstr(name)) or stand_in(g.cstr(name))
    else:
        m.hooks[g.sym("GetSubstAdrsF__8ccStreamFPCci")] = lambda mm, this, name, *a: names.get(g.cstr(name), 0)
    tscb = g.malloc(m, 0x54)
    m.store(tscb + 0x14, 4, ccs)
    m.store(g.sym("eventExTscb"), 4, tscb)
    m.store(g.sym("eventExNum"), 4, 0)
    for name in ("__dt__7ccLayerFv", "__dt__8ccScFadeFv"):
        m.hooks[g.sym(name)] = lambda mm, *a: 0

    # The task's end: ccDeleteThread does not come back (some tasks
    # breathe forever after it).
    class TaskEnd(Exception):
        pass

    def delete_thread(mm, *a):
        raise TaskEnd
    m.hooks[g.sym("ccDeleteThread__FP6ccTscb")] = delete_thread

    def set_tex(mm, sprite, *a):
        mm.store(sprite + 0xB0, 8, SKILL_TEX0)
        mm.store(sprite + 0xB8, 8, 0)
        return 0
    m.hooks[g.sym("SetTex__8ccSpriteFPcPci")] = set_tex
    # ccSprite::MakePacketStr draws nothing while fontTex is unset (the
    # game has set it long before any stream).
    m.store(g.sym("fontTex"), 4, 1)
    impure = m.load(g.sym("_impure_ptr"), 4)

    head = ("stream 2's Func_str0001" if task == "str0001" else f"{stem}'s {func.split('__')[0]}")
    lines = [f"# tools/test_stream_rs.py effects{'' if task == 'str0001' else ' ' + task}: {head} run in eemu, "
             "one pass a game frame.",
             f"# end FRAMEEND; cue STEP PARAM (0x8010 notes, {stem}'s records)",
             f"# step K rand STATE [same{'' if task == 'str0001' else ' | N FNV'}]: the pass before the task's Breath on "
             "step K (0: the frame before",
             "#   the scene's first), the libc rand state after it; 'same': the draws of step K - 1",
             "# L<layer> <kind> <flat|gouraud> tex=<none|prev|cur|frame:x,y,WxH>[/filter/wrap s/wrap t/tfx/tcc]",
             "#   blend=<none|ALPHA> atest=<off|method/ref/fail> ztest=N zwrite=N scissor=x0,x1,y0,y1",
             "#   v=x,y,z,u,v,rgba per vertex (x, y in 1/16 pixel off XYOFFSET, u, v in 1/16 texel)"]
    if task != "str0001":
        lines += ["# mark X Y Z RX RY RZ effLayer: effHitMarkStr(pos, rot, strEffLayer) in the pass (f32 bits;",
                  "#   X the cue's object index); transfer X Y Z H effLayer: effTransferStr(pos, height,",
                  "#   strEffLayer); divz BITS: the scene view's divZ changed in the pass"]
    if task in STREAM15:
        lines += ["# cue STEP PARAM OBJ: with the note's object index",
                  "# parts N FNV: the pass's part draws, each 'eff X Y Z W PAT SX SY TP COLOUR' (ccEff::Draw at",
                  "#   pos with pattern, the ccEff's scale, transparency and colour) or 'rock K M00 .. M33 TP'",
                  "#   (ccObj::Draw of rock K of OBJ_se1_6ro*, its lwMatrix, the transparency); hashed as the",
                  "#   primitives are (STREAM_FULL=1 writes them out). The objects the tables name stand at",
                  "#   stand_in_pos(name)."]
    lines.append(f"end {frame_end}")
    if task in STREAM15:
        # With each note's object (its first word), which a transfer names.
        lines += [f"cue {k} {p} {cue_objs[(k, i)]}" for k in sorted(cues) for i, p in enumerate(cues[k])]
    else:
        lines += [f"cue {k} {p}" for k in sorted(cues) for p in cues[k]]
    last = {"draws": None, "step": 0}
    layer_views = {}

    def breath(mm, *a):
        k = last["step"]
        gs = GsModel()
        arena["at"] = PACKET_ARENA
        lay = m.load(g.sym("root__7ccLayer"), 4)
        order = []
        while lay:
            order.append(lay)
            lay = m.load(lay, 4)
        for lay in reversed(order):                              # AddAll: ascending priority
            grp = m.load(lay + 8, 4)
            head = m.load(grp + 4, 4)
            m.store(grp + 4, 4, 0)
            m.store(grp + 8, 4, 0)
            if not head:
                continue
            v = m.load(lay + 0x2C, 4)
            gs.layer(struct.unpack_from("<h", m.mem, lay + 4)[0], m.load(v + 0x220, 8))
            layer_views[lay] = v
            for kind, payload in dma_chain(m.mem, head):
                if kind == "data":
                    for name, value in gif_writes(vif_direct(payload)):
                        gs.write(name, value)
            gs.flush()
        rn = m.load(impure + 168, 8)
        # What the pass did besides its packets: the hit marks it started,
        # and the view's divZ when it changed.
        extra = list(marks)
        del marks[:]
        dz = m.load(view + 0x25C, 4)
        if dz != last.get("divz"):
            # Stream 15's tasks set it before their first pass.
            if k > 0 or task in STREAM15:
                extra.append(f"divz {dz:08x}")
            last["divz"] = dz
        if gs.lines == last["draws"]:
            lines.append(f"step {k} rand {rn} same")
        elif hashed:
            text = "".join(x + "\n" for x in gs.lines).encode()
            lines.append(f"step {k} rand {rn} {len(gs.lines)} {fnv(text):016x}")
        else:
            lines.append(f"step {k} rand {rn}")
            lines.extend(gs.lines)
        lines.extend(extra)
        last["draws"] = gs.lines
        # Stream 15's parts: each ccEff::Draw and ccObj::Draw of the pass.
        if parts:
            if hashed:
                text = "".join(x + "\n" for x in parts).encode()
                lines.append(f"parts {len(parts)} {fnv(text):016x}")
            else:
                lines.extend(parts)
            del parts[:]
        # The next frame: PlaySceneMain has drawn frame k + 1 and read the
        # one after (held at frameEnd); the gap frames draw nothing, and the
        # second one's DelSceneObject ends the task.
        k += 1
        last["step"] = k
        if task == "str0580" and k > frame_end:
            # str0580 is followed: in the frame after the one it ends on
            # (as the port sets the next scene up, at once) the next scene's
            # ccSetStreamDemoThread sets param[1] 1, then 2 (eventExTscb and
            # eventExTscb2 are this task's), and the task passes four more
            # times before it ends.
            m.store(tscb + 0x18, 4, 2)
            return 0
        if k > frame_end + 1:
            # DelSceneObject's state, and the next scene's
            # ccSetStreamDemoThread ending the task (param[1]), which is
            # all some tasks (Func_str0150) look at.
            m.store(ccs + 0x17E, 2, 0x08)
            m.store(tscb + 0x18, 4, 1)
            return 0
        m.store(ccs + 0x164, 4, min(k + 1, frame_end))
        for i, p in enumerate(cues.get(k, [])):
            num = m.load(g.sym("eventExNum"), 4)
            if num < 8:
                note = g.sym("eventExNoteTbl") + 24 * num
                m.mem[note:note + 24] = bytes(24)
                m.store(note + 4, 4, 0x8010)
                m.store(note + 8, 4, p)
                m.store(note + 16, 4, note_obj(cue_objs[(k, i)]))
                m.store(g.sym("eventExNoteParamTbl") + 4 * num, 4, p)
                m.store(g.sym("eventExNum"), 4, num + 1)
        return 0

    m.hooks[g.sym("Breath__6ccTscbFi")] = breath
    try:
        g.call(func, (tscb,), limit=4_000_000_000)
    except TaskEnd:
        pass
    with open(out_path, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"{out_path}: {len(lines)} lines, {last['step']} steps")


# The subtitles: ccEventStream's window ------------------------------------------

SUB_FIXTURE = os.path.join(ROOT, "crates", "piney-stream", "tests", "subtitle_fixture.txt")
try:
    STREAM_TBL = inf_va(0x0030EF90)
except KeyError:
    # Outbreak on has streamTblE alone (the voice picks a PCM track).
    STREAM_TBL = inf_va(0x003102F0)
# (stream, saveData.strWinMode, parodyFlag, skip step or None): every stream
# on the disc with a table, Movie Text on; then Movie Text off (11 and 16
# have lines flagged 0x800, shown all the same), the parody table, and
# skips in the middle of a line.
SUB_RUNS = [(n, 1, 0, None) for n in (3, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)] + [
    (3, 0, 0, None), (11, 0, 0, None), (16, 0, 0, None), (3, 1, 1, None), (3, 1, 0, 860), (5, 1, 0, 1840)]
SUB_SAVE = 0x01A00000           # saveData points 0x8000 below it, as in test_desktop_rs
SUB_HEAP = 0x01B00000


def scenes_of(num, p):
    """A stream's scene files in play order (its records past the header
    that are not setup files; no stream with subtitles has alternatives or
    files read whole)."""
    head = p.u32(STREAM_TBL + 4 * num)
    va = head + 20
    scenes = []
    while True:
        name = p.u32(va)
        if not name:
            break
        typ, flag = struct.unpack("<hh", bytes(p.read(va + 12, 4)))
        if flag & 0x16 and typ != 0:
            raise RuntimeError(f"stream {num}: record flag {flag}")
        if typ != 0:
            scenes.append(p.cstr(name).decode())
        va += 20
    return scenes


def stream_schedule(num, p):
    """(frames, {step: [(event, param)]}, {steps where the next scene is set
    up}) over the stream's scenes as the port steps them: scene frame j of
    a scene that starts after `o` frames on step o + j; a following scene
    set up on its predecessor's last step (DecodeThread, priority 27, runs
    before the event task's pass; its ccSetStreamDemoThread sets eventMsg
    to -2)."""
    notes, resets, offset = {}, set(), 0
    scenes = scenes_of(num, p)
    for i, stem in enumerate(scenes):
        frame_end, ns = note_schedule(stem)
        for s, v in ns.items():
            notes.setdefault(offset + s, []).extend(v)
        offset += frame_end
        if i + 1 < len(scenes):
            resets.add(offset)
    return offset, notes, resets


def note_schedule(stem):
    """(frameEnd, {step: [(event, param)]}): each F_Note of the scene file
    under the Top of frame N, processed on the step that draws frame N (the
    Top of frame 0's with frame 1's, as the first pass reads both)."""
    import ccs as ccsmod
    data = member(stem)
    frame_end, top, notes = None, None, {}
    for (p, t, n, end) in ccsmod.Ccs(data).chunks():
        if t is None:
            break
        kind = t & 0xFFFF
        if kind == 0x0005 and frame_end is None:
            frame_end = struct.unpack_from("<I", data, p + 8)[0] - 1
        elif kind == 0xFF01:
            top = struct.unpack_from("<i", data, p + 8)[0]
        elif kind == 0x0108:
            obj, event, param = struct.unpack_from("<III", data, p + 8)
            notes.setdefault(max(top, 1), []).append((event, param))
    return frame_end, notes


def subtitles(full=None):
    """ccEventStream(num, 1) (0x001b5670) run in eemu over each stream's
    notes: the game's own loop, ccGetStreamDemoMsg, ccSetStreamDemoNote,
    ccKanjiStrSeparate and ccMessage's Change / Close / Disp, with the
    stream's task and the draws hooked. With `full` (a stream number) only
    that stream's first run, every draw written out, to stdout."""
    import eemu
    from image import Program
    import test_save_init_rs
    p = Program(ELF)
    sym = lambda n: p.symbol_named(n).value  # noqa: E731
    lines = ["# tools/test_stream_rs.py subtitles: ccEventStream(num, 1)'s window run in eemu over the",
             "# stream's own notes (0x8001 / 0x8011 through ccSetStreamDemoNote), one loop pass a game frame.",
             "# run NUM STRWINMODE PARODY SKIP FRAMES: step K is the pass of the frame the stream draws its",
             "#   K-th frame on (its scenes' frames one after another; FRAMES + 1, + 2 the two blank frames",
             "#   after the last scene); after the last, the call returns,",
             "#   ccEventStream closes the window and draws no more. SKIP: WaitEnd's skip on that step",
             "#   (the stream then returns after one blank frame).",
             "# change K EMODE NAME LINE0 LINE1 LINE2: the pass's ccMessage::Change (each string's FNV-1a 64 as h + hex;",
             "#   - empty, null none)",
             "# step K N FNV: the draws of that pass (ccMessage::Disp's calls), when they differ from the",
             "#   last step listed: N lines, FNV-1a 64 of them, each ended by a newline:",
             "#   pkt CODE DX DY SX SY SU SV WU WV WI ALPHA   ccSprite::MakePacket of a window cell",
             "#   kanji TEXT COUNT DX DY R G B A             ccKanji::Disp of a line (TEXT hex, - empty)",
             "#   send                                      the window's SendPacket",
             "#   (DX DY SX SY: f32 bits in hex; `subtitles --full NUM` prints the lines themselves)",
             "# end NUM STEPS"]
    save = test_save_init_rs.fresh_save(ELF)
    runs = SUB_RUNS if full is None else [r for r in SUB_RUNS if r[0] == full][:1]
    for num, winmode, parody, skip in runs:
        frame_end, notes, resets = stream_schedule(num, p)
        m = eemu.Machine(p)
        heap = [SUB_HEAP]

        def new(mm, size, *a):
            a0 = heap[0]
            heap[0] = (a0 + size + 15) & ~15
            mm.mem[a0:a0 + size] = bytes(size)
            return a0

        def nested(addr, *args):
            """A call from inside a hook: the caller's registers kept, the
            callee's stack below the caller's (Machine.call starts at
            STACK_TOP, where the caller's frame is)."""
            saved = (list(m.r), list(m.f), m.hi, m.lo, m.hi1, m.lo1, m.fcr31, m.acc, m.steps)
            top = eemu.STACK_TOP
            eemu.STACK_TOP = (m.r[29] - 0x1000) & ~0xF
            try:
                return m.call(addr, list(args))
            finally:
                eemu.STACK_TOP = top
                m.r, m.f, m.hi, m.lo, m.hi1, m.lo1, m.fcr31, m.acc, m.steps = saved

        base = SUB_SAVE - 0x8000
        m.mem[base:base + len(save)] = save
        m.store(base + 0x8430, 1, winmode)
        m.store(base + 0x842B, 1, parody)
        m.store(sym("saveData"), 4, base)
        sys_ = new(m, 0x1000)
        m.store(sym("ccSys"), 4, sys_)
        m.store(sys_, 4, 2)                                     # frameRate: The World's
        m.store(sym("ccSnd"), 4, new(m, 0x150))                 # no strSndTbl BGM: ccSndStreamSE idle
        log = []
        f2b = lambda mm, a: mm.load(a, 4)  # noqa: E731

        def pkt(mm, obj, code, *a):
            c = mm.load(obj + 0x74, 4) & 0xFF
            log.append("pkt {} {:x} {:x} {:x} {:x} {} {} {} {} {} {}".format(
                code, f2b(mm, obj + 0x40), f2b(mm, obj + 0x44), f2b(mm, obj + 0x48), f2b(mm, obj + 0x4C),
                *(mm.load(obj + o, 4, True) for o in (0x50, 0x54, 0x58, 0x5C, 0x60)), c))
            mm.store(obj + 0x40, 4, eemu.f_from_py(eemu.f_to_py(mm.load(obj + 0x40, 4))
                                                   + eemu.f_to_py(mm.load(obj + 0x48, 4))))
            return 0

        def cstr(mm, a):
            return bytes(mm.mem[a:mm.mem.index(0, a)])

        def kdisp(mm, obj, sp, c, *a):
            col = [mm.load(obj + 0x68 + 4 * i, 4) & 0xFF for i in range(4)]
            c = c - (1 << 32) if c & 0x80000000 else c
            log.append(f"kanji {cstr(mm, sp).hex() or '-'} {c} {f2b(mm, obj + 0x40):x} {f2b(mm, obj + 0x44):x} "
                       + " ".join(map(str, col)))
            return 0

        def message(mm, this, *a):
            """ccMessage's constructor, which stops on mfc0 Status in eemu:
            the object built as test_desktop_rs builds it."""
            win = new(mm, 0x1B0)
            mm.store(this, 4, win)
            nested(inf_va(0x0015A860), win)
            nested(inf_va(0x001B78B0), win, 0)
            for i in range(4):
                k = new(mm, 0xE8)
                nested(inf_va(0x0015A860), k)
                mm.store(this + 4 + 4 * i, 4, k)
                mm.store(k + 0xD4, 2, 2)
            mm.store(this + 0x20, 2, 7)
            return this

        changes = []
        change_fn = sym("Change__9ccMessageFP9ccMsgDataPcii")

        def change(mm, this, data, name, grp, *a):
            msg = mm.r[8] & 0xFFFFFFFF
            ls = [mm.load(data + 12 + 4 * i, 4) for i in range(3)]
            # The text itself stays out of the fixture: each string as its
            # FNV-1a 64 (`h` + 16 hex digits; - empty, null none).
            def text(p):
                return "null" if not p else ("h%016x" % fnv(cstr(mm, p)) if cstr(mm, p) else "-")
            changes.append("{:x} {} {}".format(mm.load(data, 4), text(name), " ".join(text(x) for x in ls)))
            # The game's own Change, run with this hook out of the way.
            del m.hooks[change_fn]
            try:
                nested(change_fn, this, data, name, grp, msg)
            finally:
                m.hooks[change_fn] = change
            return 0

        tscb, ccs = new(m, 0x54), new(m, 0x190)
        m.store(ccs + 0x17E, 2, 0x48)                           # InitScene's state
        hooks = {
            "ccStartThread__FPFPv_vii": lambda mm, *a: tscb,
            "ccGetStreamAdrs__Fv": lambda mm, *a: ccs,
            "__nw__FUi": new,
            "__ct__16ccDrawPacketCtrlFv": lambda mm, *a: 0,
            "Init__7ccLayerFsP6ccView": lambda mm, *a: 0,
            "SetFrame__6ccViewFffffffff": lambda mm, *a: 0,
            "ccInitMenuWindow__FP7ccLayer": lambda mm, *a: mm.store(sym("menuWin"), 4, new(mm, 0xD0)) or 0,
            "__ct__9ccMessageFv": message,
            "Change__9ccMessageFP9ccMsgDataPcii": change,
            "Trans__8ccSpriteFv": lambda mm, *a: 0,
            "__dt__9ccMessageFv": lambda mm, *a: 0,
            "__dt__8ccSpriteFv": lambda mm, *a: 0,
            "__dt__7ccLayerFv": lambda mm, *a: 0,
            "ccDeleteThread__FP6ccTscb": lambda mm, *a: 0,
            "MakePacket__8ccSpriteFii": pkt,
            "Disp__7ccKanjiFPciff": kdisp,
            "SendPacket__8ccSpriteFv": lambda mm, *a: log.append("send") or 0,
            "ccSeOn__Fi": lambda mm, *a: 0,
            "ccEvVoiceRequest__Fii": lambda mm, *a: 0,
            "ccEvVoiceStop__Fv": lambda mm, *a: 0,
        }
        for name, fn in hooks.items():
            m.hooks[sym(name)] = fn
        note = new(m, 0x18)
        state = {"step": 0, "last": None, "ended": False}
        out = [f"run {num} {winmode} {parody} {skip if skip is not None else '-'} {frame_end}"]
        # The pass after which the call returns: the last blank frame, or the
        # one blank frame after a skip.
        last = frame_end + 2 if skip is None else skip + 1

        def breath(mm, n, *a):
            if state["ended"]:
                return 0
            k = state["step"]
            if k == 0:
                if log != ["send"] or changes:
                    raise RuntimeError(f"stream {num}: the setup frame drew {log}")
            else:
                for c in changes:
                    out.append(f"change {k} {c}")
                if log != state["last"]:
                    if full:
                        out.append(f"step {k}")
                        out.extend(log)
                    else:
                        text = "".join(x + "\n" for x in log).encode()
                        out.append(f"step {k} {len(log)} {fnv(text):016x}")
                    state["last"] = list(log)
            del log[:]
            del changes[:]
            if k == skip:
                # WaitEnd, in ccThExecuteStream's task after this one:
                # canselFlag, ExitScene, eventMsg -1.
                m.store(sym("eventMsg"), 4, 0xFFFFFFFF)
            k += 1
            state["step"] = k
            if k > last:
                m.store(tscb + 20, 4, 0xFFFFFFFF)
                state["ended"] = True
                return 0
            if skip is None or k <= skip:
                for event, param in notes.get(k, []):
                    m.mem[note:note + 0x18] = bytes(0x18)
                    m.store(note + 4, 4, event)
                    m.store(note + 8, 4, param)
                    nested(sym("ccSetStreamDemoNote__FP9ccAnmNote"), note)
                if k in resets:
                    m.store(sym("eventMsg"), 4, 0xFFFFFFFE)
            return 0

        m.hooks[sym("ccBreathThread__Fi")] = breath
        # The scene's ccSetStreamDemoThread: nothing to show yet.
        m.store(sym("eventMsg"), 4, 0xFFFFFFFE)
        m.call(sym("ccEventStream__Fii"), [num, 1], limit=4_000_000_000)
        out.append(f"end {num} {state['step'] - 1}")
        lines += out
        print(f"stream {num} ({winmode} {parody} {skip}): {len(out)} lines", file=sys.stderr)
    if full is not None:
        print("\n".join(lines))
        return
    with open(SUB_FIXTURE, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"{SUB_FIXTURE}: {len(lines)} lines")


# The music around the streams: ccSndStreamCtrl and ccSndStreamBGM ---------------

# Infection's is stream_fixture.txt; another volume's (PINEY_VOLUME) has its
# tag: stream_fixture_mut.txt, _out, _qua.
MUSIC_FIXTURE = os.path.join(ROOT, "crates", "piney-audio", "tests",
                             "stream_fixture.txt" if volume.NAME == "infection"
                             else f"stream_fixture_{volume.CARRY_FILES[volume.NAME].rstrip('_')}.txt")
STR_SND_TBL = inf_va(0x0030B950)


def music_scenarios(p):
    """Step lists in tools/sound_ee.py's seq words, plus: strinit N
    (ccSound::strSeInit(N)), strctrl N W SIZE (ccSndStreamCtrl(N,
    streamTbl[N], W); SIZE is that header's size, for the port), strnote E
    (ccSndStreamSE on a note of event E), bgmrec SQ SQ2 TIME VOL CMD (a BGM
    record written and made the cursor's, then ccSndStreamSE on an event 4
    note), status S (game.status), movie V (ccSnd +0x62, the Audio screen's
    movie), loop I ID (ccSnd.loopID[I])."""
    def size(n):
        return struct.unpack("<i", bytes(p.read(p.u32(STREAM_TBL + 4 * n) + 8, 4)))[0]

    def ctrl(n, w):
        return f"strctrl {n} {w} {size(n)}"
    start = ["start 1", "bgm", "frames 2"]
    dungeon = ["set area 2", "set areaPrev 1", "set dungeon 1"]
    church = ["set area 1", "set areaPrev 1", "set field 15", "load 5"] + start
    town = ["set area 0", "set areaPrev 1", "set town 0", "load 2"] + start
    out = []
    # The arc: stream 3 in the dungeon of event 4 (each dungeon type's bank),
    # its three notes of event 4 walking strSndTbl[3]; streams 8-12 in the
    # church of area 15; 13 in Mac Anu, with looping effects on.
    for t in range(4):
        out.append(dungeon + [f"set dtype1 {t}", "load 4"] + start
                   + ["strinit 3", ctrl(3, 0), "frames 3", "strnote 4", "frames 3", "strnote 4", "strnote 4",
                      "frames 3", ctrl(3, 1), "frames 34"])
    out.append(church + ["strinit 8", ctrl(8, 0), "frames 34", "strnote 4", "strnote 2", "strnote 4", "frames 2",
                         "strnote 4", "strnote 4", "frames 2", ctrl(8, 1), "frames 2"]
               + [x for n in (9, 10, 11, 12) for x in (f"strinit {n}", ctrl(n, 0), "strnote 4", "frames 1",
                                                        ctrl(n, 1), "frames 1")])
    for loops in ([], [(2, 2)], [(0, 5), (3, 3)]):
        out.append(town + [f"loop {i} {v}" for i, v in loops] + ["strinit 13", ctrl(13, 0), ctrl(13, 1),
                                                                   "frames 1", ctrl(13, 1)])
    # 58's table: sequence 1 out over 120 frames.
    out.append(church + ["strinit 58", ctrl(58, 0), "strnote 4", "frames 125", "strnote 4", ctrl(58, 1)])
    # Every stream, before and after, over banks of one, two and three
    # sequences, playing (bgm) or not; the stream viewer (status 7); the
    # Audio screen's movie.
    banks = [["set area 1", "set fieldtype 0", "set bg 0", "load 3"], dungeon + ["set dtype1 0", "load 4"],
             church[:4], town[:4], ["load 7"]]
    for bank in banks:
        for playing in (True, False):
            for n in range(STREAMS):
                pre = bank + (start if playing else ["start 1", "frames 1"])
                out.append(pre + [f"strinit {n}", ctrl(n, 0), "frames 32", ctrl(n, 1), "frames 32"])
    # The Chaos Gate's own stream and the first Ryu Book's cover: 107 and
    # 112 on Infection, 113 and 118 from Mutation on.
    gate, cover = (107, 112) if volume.NAME == "infection" else (113, 118)
    for n in (3, 8, 12, 25, 56, gate, cover):
        out.append(church + ["status 7", ctrl(n, 0), ctrl(n, 1), "frames 31"])
        out.append(church + ["movie 1", "strinit 8", ctrl(n, 0), "strnote 4", "strnote 4", ctrl(n, 1), "frames 31"])
    for field in (16, 15):
        out.append(["set area 1", "set areaPrev 1", f"set field {field}", "load 5", "start 1", "frames 1",
                    ctrl(gate, 0), ctrl(gate, 1), "frames 1"])
    # Records of every command, the cursor on them one at a time.
    for rec in ((0, -1, -1, 0, 0), (1, -1, -1, 0, 0), (1, -1, 50, 64, 0), (0, -1, -1, 200, 1), (0, 1, -1, 0xffff, 1),
                (1, 0, -1, 128, 1), (0, -1, 1, 0, 2), (1, -1, 20, 128, 2), (2, -1, 8, 256, 2), (0, -1, 12, 64, 3),
                (1, -1, 30, 0, 3), (0, 1, 20, 0, 4), (1, 0, 10, 0, 4), (0, 2, 16, 0, 4), (0, -1, 5, 0, 6),
                (-1, -1, -1, 0, 1), (-1, -1, -1, 0, 5)):
        for bank in (church[:4], banks[0]):
            out.append(bank + start + ["bgmrec " + " ".join(map(str, rec)), "frames 40"])
    return out


def music():
    """ccSndStreamCtrl (0x0017d190), ccSndStreamSE (0x0017caa0) and
    ccSndStreamBGM (0x0017cb20), with strSeInit (0x00183f40), run in eemu
    by tools/sound_ee.py's Ee between its own steps: what they send, and the
    fades the sound task's frames then run."""
    import sound_ee
    ee = sound_ee.Ee(ELF)
    p, m = ee.p, ee.m
    note, rec = sound_ee.SND - 0x100, sound_ee.SND - 0x80
    out = ["# tools/test_stream_rs.py music: the game's ccSndStreamCtrl, ccSndStreamSE / ccSndStreamBGM and",
           "# strSeInit run in eemu between tools/sound_ee.py's seq steps; what reaches the IOP (its words).",
           "# seq STEP+STEP... | OUTPUT; OUTPUT; ...   (see music_scenarios for the stream steps)"]
    count = 0
    for steps in music_scenarios(p):
        ee.reset()
        ee.frames = True
        for step in steps:
            w = step.split()
            a = [int(x) for x in w[1:]] if w[0] != "set" else []
            if w[0] == "strinit":
                ee.call("strSeInit__7ccSoundFi", sound_ee.SND, a[0])
            elif w[0] == "strctrl":
                ee.call("ccSndStreamCtrl__FiP10STREAMDATAi", a[0], p.u32(STREAM_TBL + 4 * a[0]), a[1])
            elif w[0] == "strnote":
                m.mem[note:note + 0x18] = bytes(0x18)
                m.store(note + 4, 4, a[0])
                ee.call("ccSndStreamSE__FP9ccAnmNote", note)
            elif w[0] == "bgmrec":
                m.mem[rec:rec + 24] = struct.pack("<hhhHhh", a[0], a[1], a[2], a[3], 0, a[4]) + \
                    struct.pack("<hhhHhh", -1, -1, -1, 0xffff, -1, 5)
                m.store(sound_ee.SND + 0xE8, 4, rec)
                m.mem[note:note + 0x18] = bytes(0x18)
                m.store(note + 4, 4, 4)
                ee.call("ccSndStreamSE__FP9ccAnmNote", note)
            elif w[0] == "status":
                m.store(sound_ee.GAME, 4, a[0])
            elif w[0] == "movie":
                m.store(sound_ee.SND + 0x62, 1, a[0])
            elif w[0] == "loop":
                m.store(sound_ee.SND + 0x65 + a[0], 1, a[1] & 0xFF)
            else:
                ee.seq([step])
        out.append("seq " + "+".join(steps) + " | " + "; ".join(ee.log))
        count += 1
    with open(MUSIC_FIXTURE, "w") as f:
        f.write("\n".join(out) + "\n")
    print(f"{MUSIC_FIXTURE}: {count} scenarios")


CAMERA_FIXTURE = os.path.join(ROOT, "crates", "piney-desktop", "tests", "camera_fixture.txt")


def cameras():
    """ccCam::SetMatrix_PosRotXYZ (DecodeF_Camera's) and ccView::SetView
    natively, after SetFrame, for the desktop's camera (ANM_xddcamer) and
    seeded cameras: near-zero turns (libvu0's sine), whole turns, records
    without a fov (the camera keeps its own), the default, letterbox and
    mail frames. Degrees become radians as DecodeF_Camera's mul.s, div.s."""
    import random
    import eemu
    g = Game()
    m = g.m
    layer, cam, vec = g.malloc(m, 0x40), g.malloc(m, 0x60), g.malloc(m, 0x20)
    g.call("Init__7ccLayerFsP6ccView", (layer, 0, 0))
    view = m.load(layer + 0x2c, 4)
    g.call("Init__5ccCamFP10ccCamChunk", (cam, 0))
    fb = lambda v: struct.unpack("<I", struct.pack("<f", v))[0]  # noqa: E731
    frames = [(0, 0, 512, 384, 256, 192, 1, 1), (0, 48, 512, 288, 256, 192, 0.75, 0.75),
              (90, 270, 224, 80, 112, 40, 1, 6 / 7)]
    rng = random.Random(389)
    cases = [(0, (0, 0, 292609.3125), (0, 0, 0), 9.999999, frames[0])]
    for k in range(300):
        tiny = k % 3 == 0
        deg = tuple(rng.choice([0.0, rng.uniform(-0.05, 0.05)]) if tiny else rng.uniform(-360, 360) for _ in range(3))
        pos = tuple(rng.uniform(-1e5, 1e5) for _ in range(3))
        flag = 0x100 if k % 7 == 0 else 0
        fov = rng.choice([45.0, 9.999999, 30.0, 60.0, rng.uniform(5, 90)])
        cases.append((flag, pos, deg, fov, frames[k % len(frames)]))
    lines = ["# tools/test_stream_rs.py cameras: SetMatrix_PosRotXYZ and SetView run in eemu, float bits.",
             "# camera FLAG pos[3] degrees[3] fov frame[8] (SetFrame's x y w h cx cy ax ay)",
             "#   ccCam.matrix[16] ccView.world_screen[16]; FLAG 0x100: no fov, the camera keeps its own"]
    for flag, pos, deg, fov, frame in cases:
        for i, v in enumerate(frame):
            m.f[12 + i] = fb(v)
        g.call("SetFrame__6ccViewFffffffff", (view,))
        pos_b, deg_b = [fb(v) for v in pos], [fb(v) for v in deg]
        rad = [eemu.f_div(eemu.f_mul(0x40490FDB, d), 0x43340000) for d in deg_b]
        for i, v in enumerate(pos_b + [0x3F800000] + rad + [0]):
            m.store(vec + 4 * i, 4, v)
        if not flag & 0x100:
            m.store(cam + 12, 4, fb(fov))
        g.call("SetMatrix_PosRotXYZ__5ccCamFPfPf", (cam, vec, vec + 16))
        g.call("SetView__6ccViewFRC5ccCamPA4_f", (view, cam, 0))
        words = [flag] + pos_b + deg_b + [fb(fov)] + [fb(v) for v in frame]
        words += [m.load(cam + 16 + 4 * i, 4) for i in range(16)] + [m.load(view + 0xd0 + 4 * i, 4) for i in range(16)]
        lines.append("camera " + " ".join(f"{w:08x}" for w in words))
    with open(CAMERA_FIXTURE, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"{CAMERA_FIXTURE}: {len(cases)} cameras")


def fixture():
    g = Game()
    lines = ["# tools/test_stream_rs.py fixture: the game's stream code run in eemu.",
             "# table NUM ENGLISH HEADER ofs size type flag gzip scenes NAME:type:flag:gzip:ofs:size... preload NAME...",
             "# skip MODE DESKTOP_FLG FLAG PUSH CANCEL canselFlag",
             "# scene NUM STEM frames FRAMEEND objects N; resident NUM STEM... (DATA.BIN files loaded first);",
             "# draw NUM STEM LAYER OBJ PARENT (Draw's walk order;",
             "#   names with '%' and ' ' escaped as %25 and %20)",
             "# frame NUM STEM STEP frameNow state&0x14 DRAWN FNV1A-64(name, tp bits, lwMatrix bits) notes FRAME:EVENT:PARAM",
             "# camera NUM STEM STEP world_screen[16]; light NUM STEM STEP OBJ lightNormal[16] lightColor[16]",
             "# eff NUM STEM STEP N FNV1A-64 of the effect nodes' ccEff::Draw(pattern) records, sorted",
             "#   (u16 pattern, place xyz, scale xy, turn, transparency bits), when any draws"]
    lines += tables(g)
    lines += skips(g)
    for num, stems, frames in SCENES:
        # A fresh machine per stream, as each ccRequestLoadStream frees
        # everything: the last stream's ccStreams must not stay on ccscRoot
        # for CompleteIndexChunkAdrs to match names against.
        lines += run_scene(Game(), num, stems, frames)
        print(f"stream {num}: done", file=sys.stderr)
    with open(FIXTURE, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"{FIXTURE}: {len(lines)} lines")


def tables_fixture():
    """`tables` on a later volume (PINEY_VOLUME): ccStreamInit and
    RequestStrPlay for each of its streams, both voices."""
    if volume.NAME == "infection":
        raise SystemExit("Infection's tables are in stream_fixture.txt (`fixture`)")
    lines = ["# tools/test_stream_rs.py tables: the game's ccStreamInit and RequestStrPlay run in eemu.",
             "# table NUM ENGLISH HEADER ofs size type flag gzip scenes NAME:type:flag:gzip:ofs:size... preload NAME..."]
    lines += tables(Game())
    with open(TABLES_FIXTURE, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"{TABLES_FIXTURE}: {len(lines)} lines")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "fixture":
        fixture()
    elif len(sys.argv) > 1 and sys.argv[1] == "tables":
        tables_fixture()
    elif len(sys.argv) > 1 and sys.argv[1] == "effects":
        effects(sys.argv[2] if len(sys.argv) > 2 else "str0001")
    elif len(sys.argv) > 1 and sys.argv[1] == "cameras":
        cameras()
    elif len(sys.argv) > 1 and sys.argv[1] == "music":
        music()
    elif len(sys.argv) > 1 and sys.argv[1] == "gatehack":
        gate_fixture()
    elif len(sys.argv) > 1 and sys.argv[1] == "subtitles":
        subtitles(int(sys.argv[3]) if len(sys.argv) > 3 and sys.argv[2] == "--full" else None)
    else:
        print(__doc__)
