#!/usr/bin/env python3
"""crates/piney-world's characters under the event scripts against the game's
own code run in tools/eemu.py.

The evchar_probe example runs the port (piney_world::ai, party, the
Player's frame, and the town's party members as the World runs them:
piney_world::town_party, the battle's characters under piney-battle's
ccFellow::Main) on the requests this sends it; the same requests go through
the game's functions in eemu over Mac Anu's real collision mesh
(test_world_rs.Field's machine):

  - Kite built as ccPlayer::ccPlayer leaves him (Field.start) with the
    registry's bootParam applied by the game's SetBootStatus (gcmn
    0x0059e950) the way the constructor calls it (before his AI exists), the
    registry and party made by ccSPC::Initialise and ccParty::InitParty, a
    real ccAISystem, and his ccAI made by its constructor, ChangeMode(1, 1)
    and (bit 2) ManualMode, as ccSPC::Reboot does;
  - Orca (charTbl row 2) registered by ccSPC::EntrySpc with his bootParam,
    built by the game's ccFellow02 constructor (ccFellow::Initialize) and
    given his ccAI the same way, then put at event 2's marker 3 as
    ccEntryEventMng does (the position, ccAI::SetDircZ, ccEntryCmnd);
  - the event instructions run through the game's own ccEvent::Execute
    (main 0x001a8d20) at level 2 on a short array each: pc_act, pc_mode,
    pc_turn, pc_face, menu_ban, menu_clear, party_add, party_remove;
  - each frame cameraMain, ccPlayer::Main (with the real ccAI::Brains,
    ManualControl, levelCheck, SysMsgEntry / ReadSysMsg, ChatCommand and
    ChatMessageSender), then Orca's task, ccThFellow02: ccFellow::Main
    (Brains, Move, HitCheck, Action), or, the frame after a Main set
    exitFlag, the task's end (expulsionSpc, the destructor, DelSpc).

Compared every frame: Kite's position, heading, move, speeds, flags, act and
act counters, transferLag, cloak, transparencies, animation and time,
frameSpd, ground attribute, whether he is drawn, dispSW, transDist,
noDeathFlag, partyFlag, command list and character list membership, his
AI's manualSW, talkFlag, remoteFlag, remoteCmd, gDeg, gRotSp, mode,
battleFlag, arrivalChatCnt, skillMask, inviteFlag, detourCnt; the camera's
position, target, angles and world_screen; the same for Orca (and posP,
his body's position); the registry, the party, the character list's order;
and no AI message sent.

What runs in Python in place of the game: the anms (tools/anim.py's
playback), clumps, weapons, condition effects and the arrival's effect
(recorded), the command lists (membership recorded).

Skipped when the disc is not extracted or cargo is missing.
"""

import json
import os
import random
import shutil
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

from test_world_rs import (ANM, EV, ISO, ONE, PLAYER, ROOT, SAVE, SCRATCH, STARTPOS, TCAM, VIEW, AI,  # noqa: E402
                           Field, fb, script_pads)

EXAMPLE = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "release", "examples",
                       "evchar_probe")
ELF = volume.ELF

SPCMNG, PARTYMNG, AISYS_P = inf_va(0x00730340), inf_va(0x00730310), inf_va(0x00378CA0)
HITTOP = inf_va(0x0037890C)
EVBUF, EVCELL = SCRATCH + 0xA000, SCRATCH + 0xA100
# ccEvent::Execute opcodes (docs/engine/events.md).
OPS = {"act": 0x3C, "mode": 0x41, "turn": 0x47, "face": 0x48, "add": 0x4D, "remove": 0x4E}
# Orca: charTbl row 2 (cbu4body), as the save's spcParam has him for these
# checks: velocity 27.5, height 180, width 45, type 6.
ORCA = 2
ORCA_VEL, ORCA_H, ORCA_W, ORCA_FLAGS = fb(27.5), fb(180.0), fb(45.0), 6
START = (0, fb(5600.0), fb(600.0))


def build():
    subprocess.run([shutil.which("cargo"), "build", "--release", "-q", "-p", "piney-world", "--example",
                    "evchar_probe"], cwd=ROOT, check=True)


def hexs(*v):
    return " ".join("%x" % (x & 0xFFFFFFFF) for x in v)


class Probe:
    """evchar_probe as a pipe: one request, one JSON line."""

    def __init__(self):
        self.p = subprocess.Popen([EXAMPLE, ISO], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
                                  cwd=ROOT, bufsize=1)

    def ask(self, line):
        self.p.stdin.write(line + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        self.p.wait()


class EvChar(Field):
    """Field's machine with what the party's characters ask of the game: every anm its own
    animation and time (Kite's from ctu1body, Orca's from cbu4body), the clump, hands and weapons
    recorded or skipped, the command lists' membership recorded, AI messages and chats recorded
    (none are expected), condition effects skipped; ccAI, ccAISystem, the registry, the party
    and ccEvent::Execute the game's own."""

    def __init__(self):
        import anim
        import ccs
        import gzarc
        super().__init__("town01")
        m, sym = self.m, self.sym
        c = ccs.Ccs(gzarc.inflate(self.archive, self.members["cbu4body.cmp"]))
        self.fel_anims = {}
        for _, a in anim.animations(c):
            self.fel_anims.setdefault(c.objects[a.object][0], a)
        self.chunk_at, self.anm, self.listed, self.sent, self.script = {}, {}, {}, [], []
        nop = lambda mm, *a: 0                     # noqa: E731
        for name in ("ApplyClump__5ccAnmFP7ccClumpP8ccStream", "Init__7ccClumpFP12ccClumpChunk", "__ct__7ccCoordFv",
                     "EquipWeapon__9ccSpcCharFv", "DeleteWeaponCCS__9ccSpcCharFv",
                     "ccStoreSpcConditionOne__FP6ccChar", "ccRestoreSpcCondition__FP6ccChar",
                     "ccSpcConditionEffectON__Fv", "ccSpcConditionEffectOFF__Fv"):
            m.hooks[sym(name)] = nop
        # The AI's own bookkeeping runs for real.
        for name in ("levelCheck__4ccAIFv", "SysMsgEntry__4ccAIFv", "checkPartyAnnihilation__Fv"):
            m.hooks.pop(sym(name), None)
        m.hooks[sym("GetChunkAdrsF__8ccStreamFPCci")] = self.ev_chunk
        m.hooks[sym("SetAnm__5ccAnmFP10ccAnmChunkUi")] = self.ev_set_anm
        m.hooks[sym("_AnimateForward__5ccAnmFUi")] = self.ev_forward
        m.hooks[sym("Draw__5ccAnmFv")] = lambda mm, a, *r: self.events.append("draw" if a == ANM else ("fdraw", a)) or 0

        def anm_ct(mm, a, *r):
            mm.store(a + 0x9C, 2, 256)
            mm.store(a + 0xAC, 4, 0)
            return a
        m.hooks[sym("__ct__5ccAnmFv")] = anm_ct
        m.hooks[sym("GetObjAdrsF__7ccClumpFPCciP19ccSubstSearchResult")] = lambda mm, *a: self.malloc(mm, 0x40)
        m.hooks[sym("ccEntryCmnd__FP6ccChar")] = lambda mm, ch, *a: self.listed.__setitem__(ch, 1) or 0
        m.hooks[sym("ccDeleteCmnd__FP6ccChar")] = lambda mm, ch, *a: self.listed.__setitem__(ch, 0) or 0
        for name in ("ccAISysMsgSend__FisUsUsUsUs", "ccAISysMsgSendP__FisUsUsUsUsiPv",
                     "OpenChat__9ccChatMsgFP6ccCharPc"):
            m.hooks[sym(name)] = lambda mm, *a, n=name: self.sent.append(n) or 0
        m.hooks[sym("effTransfer__FP6ccChar")] = (
            lambda mm, ch, *a: self.events.append("transfer" if ch == PLAYER else ("transfer", ch)) or 0)
        m.hooks[sym("rand")] = lambda mm, *a: self.script.pop(0) if self.script else self.rand.next()
        # A leaver's delete: its animation, clump, weapons and lattices are Python's.
        for name in ("__dt__5ccAnmFv", "__dt__7ccClumpFv", "__dt__7ccModelFv", "__dt__9ccLatticeFv"):
            m.hooks[sym(name)] = nop

    def ev_chunk(self, mm, stream, name, *a):
        from eemu import _cstr
        s = _cstr(mm, name).decode()
        addr = self.chunk_at.get(s)
        if addr is None:
            addr = self.malloc(mm, 0x40)
            self.chunk_at[s] = addr
        self.names[addr] = s
        return addr

    def ev_set_anm(self, mm, anm, ch, *a):
        if anm == ANM:
            self.cur, self.time = self.names[ch], 0
        else:
            self.anm[anm] = [self.names[ch], 0]
            mm.store(anm + 0xAC, 4, 1)
        return 0

    def ev_forward(self, mm, anm, spd, *a):
        if anm == ANM:
            f = self.anims[self.cur].forward(self.time, spd & 0xFFFFFFFF)
            self.time = f.time
        else:
            st = self.anm[anm]
            f = self.fel_anims[st[0]].forward(st[1], spd & 0xFFFF)
            st[1] = f.time
        return int(f.ended)

    # --- set-up -------------------------------------------------------------
    def make_ai(self, body, code, boot):
        """ccSPC::Reboot's AI for a character built (gcmn 0x005a01e8-0x005a027c)."""
        m = self.m
        ai = AI if body == PLAYER else self.malloc(m, 0x260)
        m.mem[ai:ai + 0x260] = bytes(0x260)
        self.call("__ct__4ccAIFi", ai, code)
        m.store(body + 0x128, 4, ai)
        m.store(ai + 0x18, 4, body)
        m.store(ai + 0xB6, 2, m.load(m.load(body, 4) + 0xE, 2))
        self.call("ChangeMode__4ccAIFii", ai, 1, 1)
        if boot & 4:
            self.call("ManualMode__4ccAIFv", ai)
        return ai

    def setup(self, pos, dircz, scheme, mode, seed, boot):
        """Kite as the game builds him in a town with bootParam `boot`, the camera as ccThCamera's
        set-up leaves it, a new game's registry and party."""
        m = self.m
        self.listed, self.anm, self.sent, self.fellows, self.gone = {}, {}, [], [], set()
        self.start(pos, dircz, scheme, mode, seed)
        # An empty character list and command lists.
        for a in (inf_va(0x00378908), HITTOP, inf_va(0x00378910)):
            m.store(a, 4, 0)
        p = PLAYER
        # ccSPC::Initialise and ccParty::InitParty (ccSetupNewGame), then the event's bootParam.
        m.mem[SPCMNG:SPCMNG + 0xE0] = bytes(0xE0)
        m.mem[PARTYMNG:PARTYMNG + 0x1C] = bytes(0x1C)
        self.call("Initialise__5ccSPCFv", SPCMNG)
        self.call("InitParty__7ccPartyFv", PARTYMNG)
        m.store(SPCMNG + 8, 4, boot)
        # ccPlayer::ccPlayer's partyFlag, SpcListNum and id (Field.start builds the rest).
        w = m.load(p + 0xE0, 4)
        m.store(p + 0xE0, 4, (w & ~0x1C000) | (1 << 14))
        m.store(p + 0xE8, 4, 0)
        m.store(p + 0x118, 4, 0)
        # The constructor: ccEntryCmnd unless boot bit 2, SetBootStatus before the AI exists,
        # restraintSW unless standing.
        if not boot & 4:
            self.listed[p] = 1
        m.store(p + 0x128, 4, 0)
        b = m.load(p + 0xE0, 1)
        m.store(p + 0xE0, 1, b & ~0x04)
        self.call("SetBootStatus__9ccSpcCharFi", p, boot)
        if m.load(p + 0xEE, 2) != 2:
            m.store(p + 0xE0, 1, m.load(p + 0xE0, 1) | 0x04)
        # The AI system and Kite's AI (the probe's rand only drives the fidget).
        aisys = self.malloc(m, 0x340)
        self.call("__ct__10ccAISystemFv", aisys)
        m.store(AISYS_P, 4, aisys)
        self.script = [0, 0]
        self.make_ai(p, 0, boot)
        self.script = []
        m.store(SPCMNG + 0x1C, 4, p)
        self.call("SetParty__5ccSPCFv", SPCMNG)

    def add_fellow(self, code, boot, lag, param, pos4, dd, vel, height, width, flags):
        """ccRegisterEventMng (EntrySpc, bootParam), Reboot's step (the row's constructor, its
        AI), SetParty, then ccEntryEventMng's step for the entry."""
        m = self.m
        spc = SAVE + 0x7488 + 0xDC * code
        m.mem[spc:spc + 0xDC] = bytes(0xDC)
        m.store(spc + 8, 4, flags)
        m.store(spc + 0xC, 2, code)
        m.store(spc + 0x18, 4, height)
        m.store(spc + 0x1C, 4, width)
        m.store(spc + 0x24, 2, 120)               # maxHP
        m.store(spc + 0x26, 2, 40)                # maxSP
        m.store(spc + 0xD4, 4, vel)
        i = self.call("EntrySpc__5ccSPCFi", SPCMNG, code)
        reg = SPCMNG + 0x2C * i
        m.store(reg + 8, 4, boot)
        m.store(reg + 0x20, 4, self.malloc(m, 0x40))
        # StartPos[i]: not in the party, the origin.
        self.vec(STARTPOS + 64 * i + 0x20, [0, 0, 0, ONE])
        self.vec(STARTPOS + 64 * i + 0x30, [0, 0, 0, 0])
        # rand: the ccSpcChar constructor's cycle, Initialize's atkDellay and transferLag
        # ((rand % 4) * 5), then the AI's two.
        self.script = [0, 0, lag // 5, 0, 0]
        obj = self.malloc(m, 0x250)
        m.mem[obj:obj + 0x250] = bytes(0x250)
        self.call("__ct__10ccFellow02Fi", obj, i)
        m.store(reg + 0x1C, 4, obj)
        self.make_ai(obj, code, boot)
        self.script = []
        self.call("SetParty__5ccSPCFv", SPCMNG)
        self.vec(obj + 0x40, pos4)
        self.call("SetDircZ__4ccAIFUs", m.load(obj + 0x128, 4), dd)
        if param == 5:
            self.listed[obj] = 1
        self.fellows.append(obj)
        return i

    # --- the event instructions and the frame -------------------------------
    def ev(self, op, *args):
        """One instruction through ccEvent::Execute at level 2 (`talk`: an AI's talkFlag and gDeg
        set as its affectFunc leaves them)."""
        m = self.m
        if op == "talk":
            body = PLAYER if args[0] == 0 else self.fellows[0]
            ai = m.load(body + 0x128, 4)
            m.store(ai, 1, (m.load(ai, 1) & ~4) | (4 if args[1] else 0))
            m.store(ai + 0x9A, 2, args[2])
            return
        code = ([0x12] if args[0] else [0x13]) if op == "ban" else [OPS[op], *args]
        for k, v in enumerate(code + [0]):
            m.store(EVBUF + 2 * k, 2, v & 0xFFFF)
        m.store(EVCELL, 4, EVBUF)
        self.call("Execute__7ccEventFRPsiii", EV, EVCELL, 2, 63, 2)

    def ev_frame(self):
        """ccThCamera, ccThPlayer, then each party member's task (ccThFellow02): its
        ccFellow::Main, or, once an earlier Main set exitFlag, the task's end."""
        self.events = []
        self.call("cameraMain__Fv")
        self.call("Main__8ccPlayerFv", PLAYER)
        for obj in self.fellows:
            if obj in self.gone:
                continue
            if m_exit(self.m, obj):
                self.task_end(obj)
            else:
                self.call("Main__8ccFellowFv", obj)
        return self.ev_state()

    def task_end(self, obj):
        """ccThFellow02 (gcmn 0x0041ed60) woken to exitFlag: expulsionSpc(listNum), then
        ccDeleteThread's ccThFellow02Delete: the character's destructor (~ccFellow: its
        registry and party pointers cleared, the AI, the body off the character list,
        ccDeleteCmnd), then DelSpc(listNum) unless partyFlag is 1."""
        m = self.m
        list_num = m.load(obj + 0xE8, 4)
        pf = m.load(obj + 0xE0, 4) >> 14 & 7
        self.call("expulsionSpc__Fi", list_num)
        self.call("__dt__8ccFellowFv", obj, 0)
        if pf != 1:
            self.call("DelSpc__5ccSPCFi", SPCMNG, list_num)
        self.gone.add(obj)

    # --- what is compared ----------------------------------------------------
    def ai_state(self, ai):
        m = self.m
        if ai == 0:
            return None
        b0, b1 = m.load(ai, 1), m.load(ai + 1, 1)
        bf = b1 & 3
        return {"manual": b0 & 1, "talk": b0 >> 2 & 1, "remote_flag": b0 >> 4 & 1,
                "remote_cmd": m.load(ai + 0x74, 2, True), "g_deg": m.load(ai + 0x9A, 2),
                "g_rot_sp": m.load(ai + 0x9C, 2, True), "mode": m.load(ai + 8, 4, True),
                "mode_old": m.load(ai + 0xC, 4, True), "battle_flag": bf - 4 if bf & 2 else bf,
                "arrival": m.load(ai + 0x80, 2, True), "skill_mask": m.load(ai + 3, 1),
                "invite": b0 >> 6 & 1, "detour": m.load(ai + 0xB4, 2, True)}

    def spc_state(self, o):
        m = self.m
        h = lambda a: m.load(a, 2, True)            # noqa: E731
        b = m.load(o + 0xE0, 1)
        pf = m.load(o + 0xE0, 4) >> 14 & 7
        return {"pos": self.rvec(o + 0x40), "dirc": self.rvec(o + 0x60),
                "move_flag": b >> 3 & 1, "run_flag": b >> 5 & 1, "stop_flag": b >> 4 & 1,
                "restraint": b >> 2 & 1, "pause": b & 1, "act": h(o + 0xEE), "act_old": h(o + 0xF0),
                "act_cnt": h(o + 0xF8), "anm_flag": h(o + 0xF4), "react_cnt": h(o + 0xFA),
                "transfer_lag": h(o + 0x100), "walk_run_cnt": m.load(o + 0x124, 4, True),
                "cloak": m.load(o + 0x110, 4), "transparency": m.load(o + 0x88, 4),
                "set_transparency": m.load(o + 0x8C, 4), "attribute": m.load(o + 0x80, 4),
                "stop_cnt": h(o + 0xFC), "disp": b >> 1 & 1, "trans_dist": int(m.load(o + 0x90, 4) != 0),
                "no_death": b >> 7 & 1, "party_flag": pf - 8 if pf & 4 else pf, "listed": self.listed.get(o, 0),
                "hitsw": int(m.load(o + 0x1A0, 4) != 0), "ai": self.ai_state(m.load(o + 0x128, 4)),
                "now_speed": m.load(o + 0x10C, 4), "speed_rate": m.load(o + 0x108, 4)}

    def ev_state(self):
        m = self.m
        k = self.spc_state(PLAYER)
        t = TCAM
        k.update({"move": self.rvec(PLAYER + 0x290), "frame_spd": m.load(ANM + 0x9C, 2), "time": self.time,
                  "drawn": int("draw" in self.events), "cam_pos": self.rvec(t), "cam_view": self.rvec(t + 16),
                  "cam_deg": [m.load(t + 0x58, 2, True), m.load(t + 0x5A, 2, True)],
                  "world_screen": self.rvec(VIEW + 0xD0, 16)})
        fellows = []
        for o in self.fellows:
            if o in self.gone:
                continue
            f = self.spc_state(o)
            anm = m.load(o + 0xD4, 4)
            b1 = m.load(o + 0xE1, 1)
            f.update({"code": m.load(m.load(o, 4) + 0xC, 2, True), "pos_p": self.rvec(o + 0x50),
                      "move": self.rvec(o + 0x230), "ghost": m.load(o + 0xE0, 1) >> 6 & 1,
                      "anim": self.anm[anm][0], "time": self.anm[anm][1], "frame_spd": m.load(anm + 0x9C, 2),
                      "drawn": int(("fdraw", anm) in self.events), "recall": b1 >> 4 & 1, "exit": b1 >> 2 & 1,
                      "hitpos": self.rvec(o + 0x1C0)})
            fellows.append(f)
        reg = []
        for i in range(5):
            r = SPCMNG + 0x2C * i
            reg.append([m.load(r, 4), m.load(r + 4, 4), m.load(r + 8, 4)])
        party = {"member": [m.load(PARTYMNG + 0xC + 4 * s, 4) for s in range(3)],
                 "num": m.load(PARTYMNG + 0x18, 4, True), "registry": reg,
                 "registry_num": m.load(SPCMNG + 0xDC, 4, True)}
        ids = {PLAYER + 0x1A0: 0}
        for o in self.fellows:
            if o in self.gone:
                continue
            ids[o + 0x1A0] = 0x40 + m.load(m.load(o, 4) + 0xC, 2)
        chars, c = [], m.load(HITTOP, 4)
        while c and len(chars) < 32:
            chars.append(ids.get(c, c))
            c = m.load(c + 0x10, 4)
        return {"kite": k, "fellows": fellows, "party": party, "chars": chars}


def m_exit(m, obj):
    return m.load(obj + 0xE1, 1) >> 2 & 1


def diff(want, got, path=""):
    """The paths where the two differ (dicts and lists walked; the port's extra keys ignored)."""
    out = []
    if isinstance(want, dict):
        if not isinstance(got, dict):
            return [path]
        for key, v in want.items():
            out += diff(v, got.get(key), path + "." + key)
    elif isinstance(want, list):
        if not isinstance(got, list) or len(got) != len(want):
            return [path]
        for i, (a, b) in enumerate(zip(want, got)):
            out += diff(a, b, "%s[%d]" % (path, i))
    elif want != got:
        return [path]
    return out


@unittest.skipUnless(os.path.exists(ELF) and os.path.exists(ISO) and shutil.which("cargo"),
                     "needs the extracted disc and cargo")
class EvCharAgainstGame(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build()
        cls.g = EvChar()
        cls.seen = set()
        cls.count = {"frames": 0, "instructions": 0, "scenarios": 0}

    @classmethod
    def tearDownClass(cls):
        print(cls.__name__, cls.count)
        # What the scenarios together reached (run as a whole).
        want = {("kite act", a) for a in (2, 12, 13, 14, 5, 6)}
        want |= {("orca act", a) for a in (2, 12, 13, 14)}
        want |= {("kite remote", c, 1) for c in (3, 4, 5)}
        want |= {("orca exit", 1), ("orca party", 1), ("orca party", -1), ("kite manual", 0),
                 "gDeg past 0x8000 never reached", "orca turned to gDeg", ("kite moved by the AI", 1, 5),
                 ("kite moved by the AI", 0, 6)}
        if len(cls.seen) > 30:
            missing = want - cls.seen
            assert not missing, "never reached: %r" % sorted(map(str, missing))

    def run_plan(self, label, start, boot, plan, frames, pads=None, fellow=None, seed=1):
        """start (pos, dircz, scheme, mode); `fellow` (boot, lag, param, marker) placed after Kite;
        plan: {frame: [(op, args...)]} applied before that frame; `frames` frames compared."""
        g, pr = self.g, Probe()
        try:
            pos, dircz, scheme, mode = start
            pr.ask("start " + hexs(*pos, dircz, scheme, mode, seed, boot))
            g.setup(pos, dircz, scheme, mode, seed, boot)
            if fellow is not None:
                fboot, lag, param, marker = fellow
                mk = pr.ask("marker %x" % marker)
                g.call("RAD2DEG__Ff", fargs=[mk["rot"]])
                dd = g.m.r[2] & 0xFFFF
                i = g.add_fellow(ORCA, fboot, lag, param, mk["pos"], dd, ORCA_VEL, ORCA_H, ORCA_W, ORCA_FLAGS)
                got = pr.ask("fellow " + hexs(ORCA, fboot, lag, param, *mk["pos"], dd, ORCA_VEL, ORCA_H, ORCA_W,
                                              ORCA_FLAGS))
                self.assertEqual(got["list"], i, label)
            bad, first = 0, None
            seen = self.seen
            for f in range(frames):
                for op in plan.get(f, []):
                    g.ev(op[0], *op[1:])
                    pr.ask(op[0] + " " + hexs(*op[1:]))
                    self.count["instructions"] += 1
                pad = pads[f] if pads else (0, 0, 0, 0, 0, 0, [0] * 12)
                d, pu, pl, dl, pwr, dr, pw = pad
                g.pad(d, pu, pl, dl, pwr, dr, pw)
                want = g.ev_frame()
                got = pr.ask("pad " + hexs(d, pu, pl, dl, pwr, dr, *pw))
                k = want["kite"]
                seen.add(("kite act", k["act"]))
                seen.add(("kite remote", k["ai"]["remote_cmd"], k["ai"]["remote_flag"]))
                seen.add(("kite manual", k["ai"]["manual"]))
                seen.add(("kite drawn", k["drawn"]))
                if k["ai"]["manual"] and k["move_flag"]:
                    seen.add(("kite moved by the AI", k["run_flag"], k["act"]))
                if k["ai"]["g_deg"] >= 0x8000 and k["ai"]["remote_flag"]:
                    seen.add("gDeg past 0x8000 never reached")
                for o in want["fellows"]:
                    seen.add(("orca act", o["act"]))
                    seen.add(("orca remote", o["ai"]["remote_cmd"]))
                    seen.add(("orca exit", o["exit"]))
                    seen.add(("orca party", o["party_flag"]))
                    if o["ai"]["remote_cmd"] == 0 and not o["ai"]["remote_flag"]:
                        seen.add("orca turned to gDeg")
                seen.add(("chars", tuple(want["chars"])))
                self.count["frames"] += 1
                dd = diff(want, got)
                if dd:
                    bad += 1
                    if first is None:
                        first = (f, dd[:8], {p: (eval_path(want, p), eval_path(got, p)) for p in dd[:4]})
            self.count["scenarios"] += 1
            self.assertEqual(g.sent, [], label + ": AI messages sent")
            self.assertEqual(bad, 0, "%s: %d of %d frames differ; first %r" % (label, bad, frames, first))
            return want
        finally:
            pr.close()

    def test_event_two(self):
        """Event 2's own sequence and timing: Kite out of sight (bootParam 6) and Orca standing at
        marker 3 (bootParam 5, entry param 5); menu_ban, Orca faces Kite at once; 80 frames later
        pc_act 0 3 (the arrival); 120 later Kite turns to Orca at 64; Orca joins (party_add) and the
        second menu_ban; both turned at once at the end."""
        start = (START, 0, 0, 3)
        plan = {0: [("ban", 1), ("face", 2, 2, 0, 0)], 81: [("act", 0, 3)],
                201: [("face", 0, 2, 2, 64)], 260: [("add", 2)], 262: [("ban", 1)],
                300: [("turn", 0, -32768 & 0xFFFF, 0), ("turn", 2, 28672, 0)]}
        want = self.run_plan("event 2", start, 6, plan, 340, fellow=(5, 10, 5, 3))
        k, o = want["kite"], want["fellows"][0]
        self.assertEqual((k["act"], k["ai"]["manual"], o["act"], o["ai"]["manual"]), (2, 1, 2, 1))
        self.assertEqual(want["party"]["member"][:2], [0, 2])

    def test_turns(self):
        """pc_turn and pc_face on Kite and Orca at random times, headings (below and above 0x8000)
        and rates 0, 1, 10, 64, 300, -5, from random places."""
        rng = random.Random(7)
        for case in range(6):
            pos = (fb(rng.uniform(-600, 600)), fb(5600.0 + rng.uniform(-300, 300)), fb(600.0))
            start = (pos, fb(rng.uniform(-3.1, 3.1)), rng.randrange(4), 3)
            plan = {0: [("act", 0, 7)], 2: [("act", 0, 2)]}
            f = 4
            while f < 280:
                pc = rng.choice([0, 2, -1])
                chg = rng.choice([0, 1, 10, 64, 300, -5 & 0xFFFF])
                if rng.random() < 0.5:
                    op = ("turn", pc & 0xFFFF, rng.randrange(0x10000), chg)
                else:
                    op = ("face", pc & 0xFFFF, 2, [0, 2][pc == 0], chg)
                plan.setdefault(f, []).append(op)
                if case == 0 and f == 4:
                    plan[f].append(("add", 2))
                f += rng.randrange(1, 60)
            plan.setdefault(3, []).append(("add", 2))
            if case == 1:
                plan.setdefault(120, []).append(("talk", 2, 1, 0x3000))
                plan.setdefault(150, []).append(("talk", 2, 0, 0x3000))
                plan.setdefault(160, []).append(("talk", 0, 1, 0xC000))
                plan.setdefault(200, []).append(("talk", 0, 0, 0xC000))
            self.run_plan("turns %d" % case, start, 6, plan, 300, fellow=(5, rng.choice([0, 5, 10, 15]), 5, 3),
                          seed=case + 3)

    def test_acts(self):
        """pc_act 0-9 on Kite (0, then -3 and -1 through the party) and on Orca (2, -1 once in the
        party), from Kite out of sight or standing; manual control off hands Kite back to the pad
        (random pads, the fidget's rand)."""
        rng = random.Random(11)
        cases = [(0, a) for a in range(10)] + [(-3, a) for a in (2, 3, 4, 6, 7, 8)] + \
                [(2, a) for a in (2, 3, 4, 5, 6, 7, 8)] + [(-1, a) for a in (3, 4, 5, 7)]
        for n, (pc, act) in enumerate(cases):
            boot = rng.choice([6, 5, 4, 0, 8, 12, 3, 1])
            plan = {1: [("add", 2)], 5: [("act", pc & 0xFFFF, act)], 90: [("act", pc & 0xFFFF, act)],
                    150: [("act", pc & 0xFFFF, 7)], 200: [("act", pc & 0xFFFF, 3)],
                    230: [("act", pc & 0xFFFF, 6)], 236: [("act", pc & 0xFFFF, 3)]}
            if act in (0, 1) and pc in (2, -1):
                continue
            pads = script_pads("random", 330, rng) if act in (0, 1) and pc == 0 else None
            self.run_plan("act %d %d boot %d" % (pc, act, boot), (START, 0, 0, 3), boot, plan, 330, pads=pads,
                          fellow=(5, 5, 5, 3), seed=n + 20)

    def test_modes_bans_and_party(self):
        """pc_mode on every branch (the registry), menu_ban then menu_clear (Kite back to the pad,
        Orca back to manual control as menu_ban found him), party_add, party_remove."""
        rng = random.Random(13)
        plan = {0: [("mode", 0xFFFD, 3), ("mode", 2, 7), ("mode", 0xFFFF, 9)], 2: [("add", 2)],
                3: [("mode", 0xFFFF, 1), ("mode", 0xFFFD, 12)], 10: [("ban", 1)], 80: [("act", 0, 3)],
                170: [("ban", 0)], 240: [("remove", 2)], 250: [("add", 2)], 255: [("ban", 1)],
                270: [("remove", 0xFFFD)]}
        pads = script_pads("random", 300, rng)
        self.run_plan("bans", (START, 0, 0, 3), 6, plan, 300, pads=pads, fellow=(5, 15, 5, 3))

    def test_caught_moving(self):
        """Manual control taken while he walks or runs under the pad: the AI's move keeps his
        moveFlag and runFlag (remote 3 waits for act 2), until remote 0 stops him; walls on the
        way."""
        rng = random.Random(19)
        for case, (lean, ly) in enumerate([(200, 0), (120, 70), (255, 0)]):
            pads = []
            for f in range(260):
                pads.append((0, 0, 0, 0, 0, 0, [0] * 12) if f < 80 or f >= 110 else
                            (0, 0, lean, fb(rng.uniform(-0.3, 0.3)), 0, 0, [0] * 12))
            plan = {100: [("act", 0, 3)], 170: [("act", 0, 2)], 200: [("act", 0, 0)], 230: [("act", 0, 8)]}
            self.run_plan("moving %d" % case, (START, fb(rng.uniform(-3, 3)), 0, 3), 0, plan, 260, pads=pads,
                          seed=case + 40)

    def test_no_event(self):
        """No bootParam: the arrival as it has always been (and his AI never runs)."""
        rng = random.Random(17)
        pads = script_pads("walk", 200, rng)
        self.run_plan("no event", (START, 0, 0, 3), 0, {}, 200, pads=pads)


def eval_path(v, path):
    import re
    for part in re.findall(r"\.([a-z_0-9]+)|\[(\d+)\]", path):
        key, idx = part
        try:
            v = v[key] if key else v[int(idx)]
        except (KeyError, IndexError, TypeError):
            return None
    return v


if __name__ == "__main__":
    unittest.main()
