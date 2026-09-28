#!/usr/bin/env python3
"""crates/piney-world's event camera (evcam.rs) against the game's own code
run in tools/eemu.py (the Rust eemu_rs when it is built into tools/).

The evcam_probe example runs the port: the event's camera instructions
(EventCam::command), then each frame the event camera's task
(EventCam::frame), cameraMain and ccPlayer::Main (field_tasks). The same
goes through the game in eemu, over Mac Anu's collision mesh as
tools/test_world_rs.py's Field sets it up:

  - ccThCamera (main 0x00160610) itself for the camera's set-up (tcam, then
    ecam and bcam copied from it), stopped at its first Breath;
  - each instruction through ccEvent::Execute (main 0x001a8d20) on a
    one-instruction script at play level (2), its ccStartThread and
    ccDeleteThread noted as the task's start and stop; menu_ban and
    menu_clear through ccEvent::MenuBan / MenuClr (their party calls
    answered with nothing);
  - each frame, in the kernel's order (ccThGameCtrl, not run here, then the
    task started at priority 33 by the event task: see the docs), the
    task's first run through ccThCameraExecute (main 0x001b4ff0: its
    changeCamera(3), CamCtrl, then Breath, which stops it) and later ones
    through ccEvent::CamCtrl (0x001b3680); then cameraMain and
    ccPlayer::Main as Field.frame runs them.

The characters the instructions name are built in the game's memory and
found by the game's own GetSpc, GetNpc, GetEnemy and ccCheckTargetTypeId
(on ccSpcManager, the entry control's lists and the command lists, put in
place only while the event code runs); what they find is what the probe's
scene answers. Markers are served to GetChunkAdrsF from a table (in a
town, markerEvTbl's names).

Compared after every instruction and every frame, bit for bit: the whole
ccEvCamCtrl (eventMng+0x440, 0x310 bytes), whether camTscb is set,
normalCamID, puppetShow, camID, tcam, ecam and bcam (all but cptr),
camResetFlag, memDircZ, cameraMode, world_view and world_screen; after
every frame Kite's position, heading, move, speeds, flags, act, act
counter, cloak, transparency, whether he is drawn, angle, target count and
ground attribute.

CamCtrl works in the scratch pad (ccSys+0x25c): what it does not write
there (ecam.rot's y and w in most modes, cp in cpCtrl 4) is what the
frame's earlier code left. The scratch is cleared before each CamCtrl and
the port takes zero.

Skipped when the disc is not extracted or cargo is missing.
"""

import collections
import contextlib
import itertools
import json
import os
import random
import shutil
import struct
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc

import test_world_rs as tw  # noqa: E402
from test_world_rs import ELF, ISO, ONE, ROOT, fb, stick  # noqa: E402

EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "evcam_probe")

EV, SAVE, PLAYER, SCRATCH = tw.EV, tw.SAVE, tw.PLAYER, tw.SCRATCH
CAM = EV + 0x440
PUPPET, NORMAL, CAMTSCB = EV + 0x78C, EV + 0x7B4, EV + 0x7B8
CAMID, TCAM, BCAM, ECAM = inf_va(0x0037897C), inf_va(0x00383CE0), inf_va(0x00383D50), inf_va(0x00383DC0)
SPCMGR, PARTY = inf_va(0x00730340), inf_va(0x00730310)
START = (0, fb(5600.0), fb(600.0))

# Opcodes (docs/engine/events.md) and their operand counts.
OPS = {22: 3, 23: 3, 24: 3, 25: 1, 26: 3, 27: 3, 28: 1, 29: 4, 30: 4, 31: 4, 32: 2, 33: 3, 34: 4, 35: 4, 36: 0,
       40: 6, 41: 8, 42: 2, 43: 7, 44: 4, 48: 6, 49: 0, 50: 0}
# The instructions the tests ran, by code.
OP_COUNTS = collections.Counter()
# Kite's fields compared after a frame.
PLAYER_KEYS = ("pos", "dirc", "move", "now_speed", "speed_rate", "move_flag", "run_flag", "restraint", "act",
               "act_cnt", "cloak", "transparency", "drawn", "angle", "target_count", "attribute")


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                    "evcam_probe"], cwd=ROOT, check=True)


def hx(v):
    return "%x" % (v & 0xFFFFFFFF)


def s16(v):
    v &= 0xFFFF
    return v - 0x10000 if v & 0x8000 else v


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v & 0x80000000 else v


class Probe:
    """evcam_probe as a coprocess: requests in, JSON lines out."""

    def __init__(self):
        self.p = subprocess.Popen([EXAMPLE, ISO], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
                                  cwd=ROOT, bufsize=1)

    def tell(self, *words):
        self.p.stdin.write(" ".join(w if isinstance(w, str) else hx(w) for w in words) + "\n")

    def ask(self, *words):
        self.tell(*words)
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        self.p.wait()


class Game(tw.Field):
    """The game's event camera, camera and player in eemu."""

    SIDE_CALLS = ("ccStoreSpcCondition__Fv", "ManualModeAI__9ccSpcCharFi", "SetRemoteCmd__4ccAIFi",
                  "ccSpcConditionEffectOFF__Fv", "ccDeleteCmnd__FP6ccChar", "ClearCondition__6ccCharFP10ccSpcParam",
                  "ccRestoreSpcCondition__FP6ccChar", "ConditionAdjustment__9ccSpcCharFv",
                  "ccSpcConditionEffectON__Fv")

    def __init__(self):
        super().__init__("town01")
        from eemu import Stop
        self.Stop = Stop
        m, sym = self.m, self.sym
        for name in self.SIDE_CALLS:
            m.hooks[sym(name)] = lambda mm, *a: 0
        self.fn_exec = sym("ccThCameraExecute__FP6ccTscb")
        m.hooks[sym("ccStartThread__FPFPv_vii")] = self.start_thread
        m.hooks[sym("ccDeleteThread__FP6ccTscb")] = self.delete_thread
        m.hooks[sym("Breath__6ccTscbFi")] = self.breath
        tbl = sym("markerEvTbl")
        self.marker_names = [self.prog.u32(tbl + 4 * k) for k in range(33)]
        prev = m.hooks[sym("GetChunkAdrsF__8ccStreamFPCci")]

        def chunk(mm, stream, name, *a):
            if name in self.marker_names:
                self.vec(self.mchunk + 16, self.markers[self.marker_names.index(name)])
                return self.mchunk
            return prev(mm, stream, name, *a)
        m.hooks[sym("GetChunkAdrsF__8ccStreamFPCci")] = chunk
        self.globals = [sym(n) for n in ("g_entCtrl", "cmndPcRoot", "cmndPcLast", "cmndEneRoot", "cmndEneLast",
                                         "cmndObjRoot", "cmndObjLast")]

    # --- the kernel -------------------------------------------------------------------------------
    def start_thread(self, mm, fn, prio, size, *a):
        assert (fn, prio, size) == (self.fn_exec, 33, 0x800), (hex(fn), prio, size)
        self.task = "started"
        return self.tscb

    def delete_thread(self, mm, t, *a):
        assert t == self.tscb
        self.task = None
        return 0

    def breath(self, mm, *a):
        raise self.Stop("breath")

    def run_task(self, fn, *args):
        try:
            self.call(fn, *args)
        except self.Stop as e:
            if str(e) != "breath":
                raise

    # --- the scene ---------------------------------------------------------------------------------
    def begin(self, pos, dircz, scheme, mode, seed, markers, chars):
        """Kite arriving, ccThCamera's set-up, eventMng cleared, the
        characters built (chars: {(kind, code): (flags, pos)}, kind one of
        spc, npc, enemy, pc, obj)."""
        m = self.m
        self.start(pos, dircz, scheme, mode, seed)
        m.mem[EV:EV + 0x7E0] = bytes(0x7E0)
        self.run_task("ccThCamera__FPv", self.malloc(m, 0x60))
        self.tscb = self.malloc(m, 0x60)
        self.task = None
        self.mchunk = self.malloc(m, 0x40)
        self.code = self.malloc(m, 0x40)
        self.eotp = self.malloc(m, 0x10)
        m.store(SAVE + 0x8412, 2, 8)             # ccSaveData::Init's zoom buttons: R1, R2
        m.store(SAVE + 0x8414, 2, 2)
        self.markers = markers
        self.chars = {}
        spc = [(0, PLAYER)]
        lists = {"npc": [], "enemy": [], "pc": [], "obj": []}
        for (kind, code), (flags, p) in chars.items():
            c = self.malloc(m, 0x200)
            base = self.malloc(m, 0x20)
            m.store(c, 4, base)
            m.store(base + 8, 4, flags)
            m.store(base + 0xC, 2, code)
            self.vec(c + 0x40, p)
            self.chars[(kind, code)] = c
            if kind == "spc":
                spc.append((code, c))
            else:
                lists[kind].append(c)
        self.spc = spc
        self.lists = lists
        ent = self.malloc(m, 0x40)
        for off, key, link in ((0x14, "enemy", 0x1C4), (0x38, "npc", 0x1C4)):
            cs = lists[key]
            for a, b in itertools.pairwise(cs):
                m.store(a + link, 4, b)
            m.store(ent + off - 4, 4, len(cs))
            m.store(ent + off, 4, cs[0] if cs else 0)
        for key in ("pc", "obj"):
            for a, b in itertools.pairwise(lists[key]):
                m.store(a + 0xBC, 4, b)
        self.ent = ent

    @contextlib.contextmanager
    def scene(self):
        """ccSpcManager, ccPartyManager, the entry control and the command
        lists as the event code sees them, only while it runs."""
        m = self.m
        saved = [m.load(a, 4) for a in self.globals]
        spc_mem = bytes(m.mem[SPCMGR:SPCMGR + 5 * 0x2C])
        party_mem = bytes(m.mem[PARTY:PARTY + 0x20])
        m.mem[SPCMGR:SPCMGR + 5 * 0x2C] = bytes(5 * 0x2C)
        for i in range(5):
            m.store(SPCMGR + 0x2C * i, 4, 0xFFFFFFFF)
        for i, (code, c) in enumerate(self.spc):
            m.store(SPCMGR + 0x2C * i, 4, code)
            m.store(SPCMGR + 0x2C * i + 0x1C, 4, c)
        m.mem[PARTY:PARTY + 0x20] = bytes(0x20)
        m.store(PARTY, 4, PLAYER)                 # memberChar[0]: Kite, the leader
        m.store(PARTY + 0xC, 4, 0)                # memberID[0..2]: Kite, Orca, none
        m.store(PARTY + 0x10, 4, 2)
        m.store(PARTY + 0x14, 4, 0xFFFFFFFF)
        pcs, objs = self.lists["pc"], self.lists["obj"]
        for a, v in zip(self.globals, (self.ent, pcs[0] if pcs else 0, pcs[-1] if pcs else 0, 0, 0,
                                       objs[0] if objs else 0, objs[-1] if objs else 0)):
            m.store(a, 4, v)
        try:
            yield
        finally:
            for a, v in zip(self.globals, saved):
                m.store(a, 4, v)
            m.mem[SPCMGR:SPCMGR + 5 * 0x2C] = spc_mem
            m.mem[PARTY:PARTY + 0x20] = party_mem

    def find(self, ty, code):
        """What Execute's and CamCtrl's lookups find for (type, code): the
        character's address, 0 for none."""
        mask = 1 << (ty & 31)
        with self.scene():
            if mask & 4:
                return self.call("GetSpc__7ccEventFi", EV, code & 0xFFFFFFFF)
            if mask & 0x18:
                return self.call("GetNpc__7ccEventFi", EV, code & 0xFFFFFFFF)
            if mask & 0x60:
                return self.call("GetEnemy__7ccEventFi", EV, code & 0xFFFFFFFF)
            return self.call("ccCheckTargetTypeId__Fii", mask, code & 0xFFFFFFFF)

    # --- the event ---------------------------------------------------------------------------------
    def execute(self, op, args):
        m = self.m
        shorts = [op] + [s16(a) for a in args] + [0]
        raw = struct.pack(f"<{len(shorts)}h", *shorts)
        m.mem[self.code:self.code + len(raw)] = raw
        m.store(self.eotp, 4, self.code)
        with self.scene():
            self.call("Execute__7ccEventFRPsiii", EV, self.eotp, 2, 2, 2)

    def menu_ban(self, on):
        """MenuBan / MenuClr; Kite's +0x90, which MenuBan clears for the
        party (ccChar::Draw then skips the near fade), is put back: that is
        the party's side, not compared here."""
        m = self.m
        keep = m.load(PLAYER + 0x90, 4)
        with self.scene():
            self.call("MenuBan__7ccEventFv" if on else "MenuClr__7ccEventFv", EV)
        m.store(PLAYER + 0x90, 4, keep)

    def teach(self, part):
        """teach_camera1..3's camera part as its handler does it (0x001abb28
        and the like): cpCtrl 4 + part, the task started."""
        m = self.m
        m.store(CAM + 2, 2, 4 + part)
        if m.load(CAMTSCB, 4) == 0:
            with self.scene():
                m.store(CAMTSCB, 4, self.call("ccStartThread__FPFPv_vii", self.fn_exec, 33, 0x800))

    def ev_frame(self):
        """ccThCameraExecute's frame, if the task runs, then cameraMain and
        ccPlayer::Main."""
        if self.task is not None:
            self.m.mem[SCRATCH:SCRATCH + 0x100] = bytes(0x100)
            with self.scene():
                if self.task == "started":
                    self.task = "running"
                    self.run_task(self.fn_exec, self.tscb)
                else:
                    self.call("CamCtrl__7ccEventFv", EV)
        return self.frame()

    def cam_state(self):
        m = self.m
        words = lambda a, n: [m.load(a + 4 * i, 4) for i in range(n)]  # noqa: E731

        def cam(a):
            w = words(a, 26)
            w[20] = 0
            return w
        return {
            "ctrl": words(CAM, 0x310 // 4), "running": int(m.load(CAMTSCB, 4) != 0),
            "cam_id": s16(m.load(CAMID, 2)), "normal": s32(m.load(NORMAL, 4)), "puppet": int(m.load(PUPPET, 4) != 0),
            "tcam": cam(TCAM), "ecam": cam(ECAM), "bcam": cam(BCAM),
            "resetting": s16(m.load(tw.CAMRESET, 2)), "mem_dirc_z": s16(m.load(tw.MEMDIRCZ, 2)),
            "mode": m.load(SAVE + 0x8431, 1, True),
        }


def pad_words(pad):
    direct, push, pl, dl, pr, dr, pw = pad
    return [direct, push, pl, dl, pr, dr] + list(pw)


class Run:
    """One scenario: the game and the probe side by side, compared after
    every step."""

    def __init__(self, t, game, probe, label, pos=START, dircz=0, scheme=0, mode=3, seed=1, markers=None,
                 chars=None):
        self.t, self.g, self.p, self.label = t, game, probe, label
        self.markers = markers or [[fb(0.0), fb(5600.0), fb(600.0), ONE]] * 33
        self.chars = chars or {}
        self.frames = self.steps = 0
        self.log = [f"start scheme {scheme} mode {mode}"]
        # The eye view under a puppet show stores tcam.deg[0] from a stack
        # slot it never wrote (not modelled; the port leaves it): masked
        # until an eye-view frame outside the show or a soft reset rewrites
        # it, and L2 is kept off the first frame after the show so that
        # nothing reads it.
        self.eye_garbage = False
        self.g.begin(pos, dircz, scheme, mode, seed, self.markers, self.chars)
        got = probe.ask("start", *pos, dircz, scheme, mode, seed)
        for k, mk in enumerate(self.markers):
            probe.tell("mark", k, *mk)
        self.check(got, "start", with_player=False)

    def check(self, got, what, with_player=True):
        self.log.append(what)
        want = self.g.cam_state()
        if with_player:
            st = self.g.state()
            for k in PLAYER_KEYS:
                want[k] = st[k]
            want["world_view"], want["world_screen"] = st["world_view"], st["world_screen"]
        if self.eye_garbage:
            for c in (want, got):
                c["tcam"] = list(c["tcam"])
                c["tcam"][22] &= 0xFFFF0000
        for k, v in want.items():
            g = got[k]
            if g != v:
                diff = ""
                if isinstance(v, list):
                    diff = " at " + ", ".join(f"[{i}] {hx(a)} vs {hx(b)}" for i, (a, b) in enumerate(zip(g, v))
                                              if a != b)[:400]
                self.t.fail(f"{self.label}, {what} (frame {self.frames}): {k} port {g} game {v}{diff}; "
                            f"after {' / '.join(self.log[-12:])}")
        self.steps += 1

    def resolve(self, ty, code):
        """Tell the probe what the game's lookups find for (type, code)."""
        c = self.g.find(ty, code)
        if c == 0:
            self.p.tell("unchar", ty, code)
        elif c == PLAYER:
            self.p.tell("kite", ty, code)
        else:
            self.p.tell("char", ty, code, *self.g.rvec(c + 0x40))

    def cmd(self, op, *args):
        args = list(args) + [0] * (OPS[op] - len(args))
        OP_COUNTS[op] += 1
        if op in (23, 24, 27, 30, 31, 48):
            self.resolve(s16(args[0]), s16(args[1]))
        self.g.execute(op, args)
        if op == 50:
            self.eye_garbage = False
        self.check(self.p.ask("cmd", op, *[a & 0xFFFF for a in args]), f"cmd {op} {args}", with_player=False)

    def ban(self, on):
        self.g.menu_ban(on)
        self.check(self.p.ask("ban", int(on)), f"ban {on}", with_player=False)

    def teach(self, part):
        self.g.teach(part)
        self.check(self.p.ask("teach", part), f"teach {part}", with_player=False)

    def move(self, kind, code, pos, ty=None):
        """A character steps somewhere else."""
        c = self.g.chars[(kind, code)]
        self.g.vec(c + 0x40, pos)
        if ty is not None:
            self.resolve(ty, code)

    def frame(self, pad=None):
        pad = pad or (0, 0, 0, 0, 0, 0, [0] * 12)
        if self.eye_garbage and not self.g.cam_state()["puppet"]:
            pad = (pad[0] & ~1, pad[1] & ~1) + tuple(pad[2:])
        self.g.pad(*pad)
        self.g.ev_frame()
        got = self.p.ask("pad", *pad_words(pad), 8, 2)
        self.frames += 1
        st = self.g.cam_state()
        if st["cam_id"] == 1 and st["tcam"][23] == 1:
            self.eye_garbage = bool(st["puppet"])
        self.check(got, "frame")

    def frames_(self, n, pads=None):
        for i in range(n):
            self.frame(pads[i] if pads else None)


def walk_pads(rng, n, scheme_buttons=True):
    """Random pads: the left stick often, the right stick and the camera
    buttons (L1, R1, L2, R2, with and without pressure) now and then."""
    pads, held = [], 0
    for _ in range(n):
        lx = ly = rx = ry = 128
        if rng.random() < 0.6:
            lx, ly = rng.randrange(256), rng.randrange(256)
        if rng.random() < 0.4:
            rx, ry = rng.randrange(256), rng.randrange(256)
        direct = rng.choice([0, 0, 0, 1, 2, 4, 8, 4 | 8, 0x1000]) if scheme_buttons else 0
        push = direct & ~held
        held = direct
        dl, pl = stick(lx, ly)
        dr, pr = stick(rx, ry)
        pw = [rng.choice([0, 0, 5, 11, 60, 130, 255]) for _ in range(12)]
        pads.append((direct, push, pl, dl, pr, dr, pw))
    return pads


def tenths(rng, lo, hi):
    return rng.randrange(lo, hi)


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class EventCameraAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()
        cls.game = Game()
        cls.probe = Probe()

    @classmethod
    def tearDownClass(cls):
        cls.probe.close()
        print("\ninstructions by code:", dict(sorted(OP_COUNTS.items())))

    def test_event2(self):
        """Event 2's camera from the arrival: Orca at DMY_marker_ev03, the
        instructions with the script's values and the waits between them,
        then camera_end and Kite walking under the field camera again."""
        orca = self.probe.ask("markerpos", 3)
        run = Run(self, self.game, self.probe, "event 2", chars={("spc", 2): (7, orca)})
        rng = random.Random(2)
        run.ban(True)
        run.cmd(40, -2, 574, 102, -3, 537, 68)                    # camz_set
        run.frames_(2)                                           # wait 1
        run.cmd(42, 2, 2)                                        # camz_speed
        run.cmd(41, 0, 560, 70, 8, 521, 74, 120, 120)            # camz_move
        run.frames_(81)                                          # wait 80
        run.frames_(121)                                         # pc_act; wait 120
        run.cmd(48, 2, 2, 10, 1024, -30720, 40)                  # camera on Orca
        run.frames_(45)                                          # message 0, pc_face
        run.cmd(42, 2, 2)
        run.cmd(41, -12, 584, 69, 0, 536, 77, 20, 20)
        run.frames_(41)                                          # wait 40
        run.frames_(150)                                         # messages 1-6
        run.cmd(40, -33, 635, 71, -29, 585, 72)
        run.frames_(60)                                          # messages 7-11
        run.cmd(40, -11, 586, 72, 1, 534, 68)
        run.frames_(30)                                          # the party menu
        run.ban(True)                                            # menu_ban again: normalCamID 3
        run.frames_(90)                                          # messages 21-29, the fade
        run.cmd(40, -7, 577, 69, -13, 527, 64)
        run.frames_(45)
        run.cmd(49)                                              # camera_end
        run.ban(False)
        run.frames_(160, walk_pads(rng, 160))
        print(f"\nevent 2: {run.steps} comparisons over {run.frames} frames")

    def test_end_reset(self):
        """camera_end_reset back to the camera menu_ban found, tcam turned
        behind Kite at once, in both scheme types; a reset under way when
        the event camera takes over."""
        rng = random.Random(5)
        for scheme in range(4):
            run = Run(self, self.game, self.probe, f"end_reset scheme {scheme}", scheme=scheme, seed=scheme + 3)
            run.frames_(90, walk_pads(rng, 90))
            run.ban(True)
            run.cmd(40, 30, 600, 90, -40, 500, 120)
            reset = 2 if scheme in (0, 1) else 4
            run.frame((reset, reset, 200, fb(1.0), 0, 0, [0] * 12))
            run.frames_(60, walk_pads(rng, 60))
            run.cmd(50)
            run.ban(False)
            run.frames_(60, walk_pads(rng, 60))
            print(f"\nend_reset scheme {scheme}: {run.steps} comparisons over {run.frames} frames")

    def random_chars(self, rng):
        near = lambda: [fb(rng.uniform(-1500, 1500)), fb(rng.uniform(4000, 7000)),  # noqa: E731
                        fb(rng.uniform(300, 900)), ONE]
        chars = {("spc", 2): (7, near()), ("spc", 5): (7, near())}
        for code in (1, 29, 40, 158):
            chars[("npc", code)] = (0x10 if code in (29, 158) else 0x8, near())
        for code in (3, 7):
            chars[("enemy", code)] = (rng.choice([0x20, 0x40]), near())
        for code in (0, 4):
            chars[("pc", code)] = (7, near())
        for code in (0, 2):
            chars[("obj", code)] = (0x100 << code, near())
        return chars

    def random_command(self, run, rng):
        """One random camera instruction with operands a script could hold
        (and some it would not)."""
        v = lambda: [tenths(rng, -300, 300), tenths(rng, 400, 800), tenths(rng, 20, 200)]  # noqa: E731
        who = lambda: rng.choice([(2, 2), (2, 5), (2, 0), (2, 9), (3, 1), (3, 29), (4, 40), (4, 158),  # noqa: E731
                                  (5, 3), (6, 7), (0, 4), (1, 0), (8, 0), (10, 2), (9, 1), (3, 77)])
        op = rng.choice([40] * 6 + [41] * 8 + [42] * 6 + [43] * 3 + [44] * 2 + [48] * 5 +
                        [22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36] + [49, 50])
        rate = lambda: rng.choice([0, 1, 2, 5, 20, rng.randrange(0, 201)])  # noqa: E731
        if op == 40:
            run.cmd(40, *v(), *v())
        elif op == 41:
            run.cmd(41, *v(), *v(), rate(), rate())
        elif op == 42:
            run.cmd(42, *[rng.choice([0, 1, 2, 3, 4, 5, 6, -1]) for _ in range(2)])
        elif op == 43:
            run.cmd(43, rng.randrange(0, 16), *v(), *v())
        elif op == 44:
            run.cmd(44, rng.choice([1, 2, 3, 4, 4, 5, 8, 12, 15]), rate(), rate(), rng.randrange(-20, 21))
        elif op == 48:
            ty, code = who()
            run.cmd(48, ty, code, rng.randrange(0, 30), rng.randrange(-8192, 8192), rng.randrange(-32768, 32768),
                    rng.randrange(0, 200))
        elif op in (23, 24, 27):
            run.cmd(op, *who(), rng.randrange(0, 30))
        elif op in (30, 31):
            run.cmd(op, *who(), rng.randrange(0, 30), rate())
        elif op in (22, 26):
            run.cmd(op, *v())
        elif op == 29:
            run.cmd(op, *v(), rate())
        elif op in (25, 28):
            run.cmd(op, rng.randrange(0, 33))
        elif op == 32:
            run.cmd(op, rng.randrange(0, 33), rate())
        elif op == 33:
            run.cmd(op, rng.randrange(-8192, 8192), rng.randrange(-32768, 32768), rng.randrange(0, 200))
        elif op == 34:
            run.cmd(op, rng.randrange(-8192, 8192), rng.randrange(-32768, 32768), rng.randrange(0, 200), rate())
        elif op == 35:
            run.cmd(op, rng.randrange(-8192, 8192), rng.randrange(-32768, 32768), rng.randrange(0, 200),
                    rng.randrange(-600, 600))
        else:
            run.cmd(op)

    def test_random(self):
        """Random instruction sequences at random frames over random pads:
        every speed type and rate, camz_set in the middle of a move, paths,
        camera on characters of every kind (and on none), looks, pans and
        orbits, camera_end and camera_end_reset, menu_ban and menu_clear."""
        rng = random.Random(7)
        total = frames = 0
        for n in range(40):
            markers = [[fb(rng.uniform(-3000, 3000)), fb(rng.uniform(3000, 8000)), fb(rng.uniform(0, 1500)),
                        rng.choice([ONE, fb(0.5)])] for _ in range(33)]
            chars = self.random_chars(rng)
            run = Run(self, self.game, self.probe, f"random {n}", dircz=fb(rng.uniform(-3.1, 3.1)),
                      scheme=rng.randrange(4), mode=rng.choice([3, 3, 1]), seed=n + 11, markers=markers,
                      chars=chars)
            for _ in range(rng.randrange(20, 45)):
                r = rng.random()
                if r < 0.55:
                    self.random_command(run, rng)
                elif r < 0.6:
                    run.ban(rng.random() < 0.6)
                elif r < 0.65:
                    kind, code = rng.choice(list(chars))
                    ty = {"spc": 2, "npc": 3, "enemy": 5, "pc": 0, "obj": 8 + code}[kind]
                    run.move(kind, code, [fb(rng.uniform(-1500, 1500)), fb(rng.uniform(4000, 7000)),
                                          fb(rng.uniform(300, 900)), ONE], ty)
                # A few frames between instructions, sometimes in the same
                # frame (the task first runs after the instructions).
                k = rng.choice([0, 0, 1, 2, 3, 8, 20, 40])
                run.frames_(k, walk_pads(rng, k))
            total += run.steps
            frames += run.frames
        print(f"\nrandom: {total} comparisons over {frames} frames")

    def test_follow(self):
        """camera (and cam_follow_char / cam_pan_follow) on characters that
        move every frame, at random angles, distances and heights."""
        rng = random.Random(11)
        total = frames = 0
        for n in range(12):
            chars = self.random_chars(rng)
            run = Run(self, self.game, self.probe, f"follow {n}", scheme=n % 4, seed=n + 101, chars=chars)
            run.frames_(80, walk_pads(rng, 80))
            for _ in range(4):
                kind, code = rng.choice([("spc", 2), ("npc", 29), ("enemy", 7), ("pc", 4)])
                ty = {"spc": 2, "npc": 3, "enemy": 6, "pc": 1}[kind]
                op = rng.choice([48, 48, 24, 31])
                if op == 48:
                    run.cmd(48, ty, code, rng.randrange(0, 20), rng.randrange(-4096, 4096),
                            rng.randrange(-32768, 32768), rng.randrange(10, 120))
                else:
                    run.cmd(33, rng.randrange(-4096, 4096), rng.randrange(-32768, 32768), rng.randrange(10, 120))
                    run.cmd(op, ty, code, rng.randrange(0, 20), *([rng.randrange(0, 30)] if op == 31 else []))
                pads = walk_pads(rng, 50)
                for i in range(50):
                    p = [fb(rng.uniform(-1500, 1500)), fb(rng.uniform(4000, 7000)), fb(rng.uniform(300, 900)), ONE]
                    run.move(kind, code, p, ty)
                    run.frame(pads[i])
                # The follow onto Kite himself.
                run.cmd(48, 2, 0, 16, 1500, rng.randrange(-32768, 32768), 60)
                run.frames_(20, walk_pads(rng, 20))
            run.cmd(49)
            run.frames_(30, walk_pads(rng, 30))
            total += run.steps
            frames += run.frames
        print(f"\nfollow: {total} comparisons over {frames} frames")

    def test_curves(self):
        """camz_move over every speed type and rates 0-200, camz_set in the
        middle of a move, a second move from mid-way, and paths of 1-15
        points in both path curves."""
        rng = random.Random(13)
        total = frames = 0
        for n in range(24):
            run = Run(self, self.game, self.probe, f"curves {n}", scheme=n % 4, seed=n + 201)
            v = lambda: [tenths(rng, -300, 300), tenths(rng, 400, 800), tenths(rng, 20, 200)]  # noqa: E731
            run.cmd(40, *v(), *v())
            run.frame()
            for _ in range(6):
                run.cmd(42, rng.choice([0, 1, 2, 3, 5, -1]), rng.choice([0, 1, 2, 3, 5, -1]))
                vr, cr = rng.randrange(0, 201), rng.randrange(0, 201)
                run.cmd(41, *v(), *v(), vr, cr)
                k = rng.randrange(0, max(vr, cr) + 10)
                run.frames_(k, walk_pads(rng, k))
                if rng.random() < 0.3:
                    run.cmd(40, *v(), *v())
                    run.frames_(3)
            for num in (1, 2, 3, 4, 7, 12, 15):
                for i in range(num):
                    run.cmd(43, i, *v(), *v())
                run.cmd(42, rng.choice([4, 5, 0]), rng.choice([4, 5, 2]))
                vr, cr = rng.randrange(1, 201), rng.randrange(1, 201)
                run.cmd(44, num, vr, cr, rng.randrange(-20, 21))
                k = max(vr, cr) + 5
                run.frames_(k, walk_pads(rng, k))
            total += run.steps
            frames += run.frames
        print(f"\ncurves: {total} comparisons over {frames} frames")

    def test_teach(self):
        """The camera tutorial's modes (teach_camera1-3: cpCtrl 5, 6, 7) in
        every scheme under random pads, from an orbit round Kite."""
        rng = random.Random(17)
        total = frames = 0
        for n in range(16):
            scheme = n % 4
            run = Run(self, self.game, self.probe, f"teach {n}", scheme=scheme, seed=n + 301,
                      dircz=fb(rng.uniform(-3.1, 3.1)))
            run.frames_(80, walk_pads(rng, 80))
            run.cmd(48, 2, 0, 16, rng.randrange(0, 3000), rng.randrange(-32768, 32768), rng.randrange(30, 150))
            run.frame()
            for part in (1, 2, 3):
                run.teach(part)
                k = 60 if part < 3 else 25
                run.frames_(k, walk_pads(rng, k))
            run.cmd(49)
            run.frames_(20, walk_pads(rng, 20))
            total += run.steps
            frames += run.frames
        print(f"\nteach: {total} comparisons over {frames} frames")


if __name__ == "__main__":
    unittest.main()
