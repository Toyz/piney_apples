#!/usr/bin/env python3
"""piney_battle::kite against the game's ccPlayer (player.cpp) run in
tools/eemu.py (the Rust machine, crates/piney-eemu, when built).

Each check builds a random world on tools/test_battle_party_ai_rs.py's:
party members (Kite in slot 0, always with his ccAI) and foes on the
command lists, the AIs on a ccAISystem with waiting and delivered messages,
the skill and item lists; then every ccPlayer member of Kite's (the flag
word, acts and counters, the skill state, the targets, movePos, the camera
points, diskOffset, bodyHit ...), the globals his code reads (the pad,
cmndTarget, the camera, WORLD_MAN's map bounds and warp flag, dneFlag, plw)
and a random script of what the world answers. It runs one of his
functions natively in the interpreter, once or for N frames in a row, and
the same case through the battle_probe example (`kite` requests), and
compares the return value, every field of every character, Kite's
ccPlayer, the AIs and the message bus, the command lists, the camera's
reset flag, plw, the rand() state, and every call in order.

The calls into code that is not rules are stubbed, recorded and answered
from the script, on both sides alike: ccSkillRequest (which, by the
script, ends a running attack: skillStatus 0), the animation player
(GetChunkAdrsF/SetAnm, _AnimateForward), ccCharHit::HitEnable/HitDisable,
ccSpcChar::HitCheck, ccLandHitCheck (and the hit result it leaves),
checkHitResultAttlibute, WORLD_MAN's GetTransMode, GetTransCenter,
AddCenter and Enter, the camera (cameraGetRot, cameraSetEyeLevel,
cameraSetManual, cameraSet), ccChar::Draw, GateHackingOut, the effects,
sounds and the weapon's trails. ccAnm::NoteProcess is replaced by a
routine that hands the script's notes for the frame to the game's own
ccPlayerCheckNote, so the normal attack's hit (CalcBattleDamage) runs
natively. ccChar::EntryAffect runs natively with every character's
real affectFunc (Influence for Kite, ccFellow::Influence for a member with
an AI, ccEnemyInfluence / affectEnemy for an enemy), only what they show
stubbed (as test_battle_rs.affect_hooks does; ccSkillCheck answers by the
case's rule), so the long runs check that an affect lands where the game
makes it (a poison tick downing Kite inside CalcReal is his fall in the
same frame's AnimCtrl). The AI (ccAI::Brains, levelCheck,
SelectAttackSkill, UseItem, the message bus), CalcReal, CheckLevelUp,
DamageActuate, ccEntryCmnd and checkPartyAnnihilation run natively too;
the party AI's own stubs are test_battle_party_ai_rs.AiGame's. Every
character's affect state, a member's and an enemy's flags are compared.

    python3 tools/test_battle_kite_rs.py            the unit tests
    python3 tools/test_battle_kite_rs.py bulk N     N cases of every check

Checks:
  anim_ctrl, anim_run      ccPlayer::AnimCtrl (0x005993c0), once and for
                           60-300 frames, with CheckNote (0x0059c300)
  main, main_run           ccPlayer::Main (0x00598310), once and for 60-300
                           frames
  control_move             ccPlayer::ControlMove (0x00598af0), every ctrlType
  attack                   ccPlayer::Attack (0x0059c580)
  check_note               ccPlayerCheckNote / ccPlayer::CheckNote
  small                    AttackCancel, BreakSomething, DamageActuate,
                           ResultOfConditions, ccPlayerMenuCheck,
                           CheckControlMode, SetTargetDist, SetTargetDirc,
                           ccAI::ReadSysMsg2
  frames                   W2MPos, W2PPos, P2WPos and ccTransPosW2M, W2P,
                           P2W, FW2LW on random maps and positions
  main_run_ai              ccPlayer::Main for 60-300 frames in a field with
                           ccAI::Brains driving Kite (charmed, confused, a
                           charmed ghost, or under an event's remote control):
                           FollowTarget, FollowTargetDirc, FollowPlayer,
                           LeavePlayer, ccPlayer::Attack, ManualControl and
                           ccSetDirc run natively (Natives), the port runs
                           kite::main with kite::Host (party_motion::Movement)
                           for its runtime; compared after every frame (the
                           calls each frame made), aiOpenDirc included
"""

import math
import os
import random
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from volume import va as inf_va  # noqa: E402  # Infection's addresses on PINEY_VOLUME's disc
import volume  # noqa: E402

import eemu                                     # noqa: E402
import test_anim                                # noqa: E402

eemu.Machine = test_anim.machine_class()

import test_battle_party_ai_rs as PA            # noqa: E402
import test_battle_rs as T                      # noqa: E402
from test_battle_rs import fbits                # noqa: E402

TB_MENU = 0x01020000    # test_battle.MENU: ccMenu

ANM = 0x01080000        # Kite's ccAnm stand-in: +0x9c frameSpd, +0xac a clip
NOTES = 0x01080400      # the frame's notes (ccAnmNote: +4 event, +8 param)
NOTE_LIST = 0x01080800  # their addresses, 0-terminated, for the routine at NoteProcess
CAM = 0x01081000        # activeCamPtr's CAMERA: +0x5c type, +0x60 resetFlag, +0x64 resetDirc
WM = 0x01082000         # WORLD_MAN: +0x168 warpFlag, +0x420 the bounds
OBJ = 0x01083000        # objHandL / objHandR stand-ins
OUTV = 0x01084000       # a scratch vector for the frame functions
# ccAI::CheckFrontObstacle and CheckFrontObstacleF: their ccHitCheckLM is the
# world's line; the decisions' (CheckEyeLineToTarget, the searches) stays theirs
FRONT = (inf_va(0x00589E90), inf_va(0x0058AE50))
F_ONE = 0x3F800000
F_M1 = 0xBF800000
PI = 0x40490FDB
KITE_TYPES = (1, 1, 1, 7)

s32, s16 = PA.s32, PA.s16


def vec(m, a):
    return [m.load(a + 4 * i, 4) for i in range(4)]


def put_vec(m, a, v):
    for i, x in enumerate(v):
        m.store(a + 4 * i, 4, x)


def fl(x):
    return fbits(x)


def fv(bits):
    return struct.unpack("<f", struct.pack("<I", bits))[0]


def rnd_f(rnd, lo, hi):
    return fl(rnd.uniform(lo, hi))


def rnd_vec(rnd, lo=-3000, hi=3000):
    return [rnd_f(rnd, lo, hi), rnd_f(rnd, lo, hi), rnd_f(rnd, -200, 200), F_ONE]


def nested_call(m, addr, args, out=()):
    """m.call from inside a hook (as tools/test_battle_fellow_rs.py does):
    the machine's call starts afresh at the top of the stack with the
    integer registers cleared, so the caller's frames and registers are
    kept aside and put back. The float registers carry through. `out`:
    the (address, size) the callee writes its results to, kept across
    the frames' restoring (a result in the caller's stack frame)."""
    sp = m.r[29] & 0xFFFFFFFF
    frames = bytes(m.mem[sp:eemu.STACK_TOP])
    regs = [m.r[i] for i in range(32)]
    hi, lo = m.hi, m.lo
    r = m.call(addr, args)
    kept = [(a, bytes(m.mem[a:a + n])) for a, n in out]
    m.mem[sp:eemu.STACK_TOP] = frames
    for a, b in kept:
        m.mem[a:a + len(b)] = b
    for i in range(32):
        m.r[i] = regs[i]
    m.hi, m.lo = hi, lo
    return r


def assemble_note_routine():
    """ccAnm::NoteProcess's stand-in: ccPlayerCheckNote(note) for each
    address in NOTE_LIST up to a 0, as the game's calls the anm's note
    function for each note the frame passed."""
    hi, lo = NOTE_LIST >> 16, NOTE_LIST & 0xFFFF
    check = inf_va(0x0059C550)
    return [
        0x27BDFFE0,                 # addiu sp, sp, -32
        0xFFBF0010,                 # sd    ra, 16(sp)
        0xFFB00000,                 # sd    s0, 0(sp)
        0x3C100000 | hi,            # lui   s0, hi
        0x36100000 | lo,            # ori   s0, s0, lo
        0x8E040000,                 # loop: lw a0, 0(s0)
        0x10800005,                 # beqz  a0, done
        0x00000000,                 # nop
        0x0C000000 | (check >> 2),  # jal   ccPlayerCheckNote
        0x26100004,                 # addiu s0, s0, 4 (delay slot)
        0x1000FFFA,                 # b     loop
        0x00000000,                 # nop
        0xDFBF0010,                 # done: ld ra, 16(sp)
        0xDFB00000,                 # ld    s0, 0(sp)
        0x03E00008,                 # jr    ra
        0x27BD0020,                 # addiu sp, sp, 32
    ]


# the game side ---------------------------------------------------------------------

class KiteGame:
    """AiGame (the party AI's world) plus Kite's ccPlayer, the globals and
    the world's stubs answering from the script."""

    def __init__(self, pc):
        self.pc = pc
        self.ai = pc.ai
        self.g = pc.c.game
        self.scene = pc.c.scene
        self.m = self.g.m
        m = self.m
        sym = self.g.sym
        self.sym = sym
        self.plw = sym("plw")
        self.fn = {n: sym(n) for n in (
            "AnimCtrl__8ccPlayerFv", "Main__8ccPlayerFv", "ControlMove__8ccPlayerFv",
            "Attack__8ccPlayerFP6ccChari", "AttackCancel__8ccPlayerFv", "BreakSomething__8ccPlayerFP6ccChar",
            "DamageActuate__8ccPlayerFi", "ResultOfConditions__8ccPlayerFv", "ccPlayerMenuCheck__Fv",
            "CheckControlMode__9ccSpcCharFv", "SetTargetDist__8ccPlayerFv", "SetTargetDirc__8ccPlayerFv",
            "ReadSysMsg2__4ccAIFv", "ccPlayerCheckNote__FP9ccAnmNote", "W2MPos__8ccPlayerFPfPf",
            "W2PPos__8ccPlayerFPfPf", "P2WPos__8ccPlayerFPfPf", "ccTransPosW2M__FPfPf", "ccTransPosW2P__FPfPf",
            "ccTransPosP2W__FPfPf", "ccTransPosFW2LW__FPfPf")}
        brains = sym("Brains__4ccAIFv")
        self.brains = (brains, brains + 764)
        m.store(sym("activeCamPtr"), 4, CAM)
        m.store(sym("worldman"), 4, WM)
        self.cam_id = sym("camID")
        self.cmnd_target = sym("cmndTarget")
        self.dne = sym("dneFlag")
        self.hit_num = sym("hitResultNum")
        self.hit_near = sym("hitResultNearest")
        self.lasts = [sym("cmndPcLast"), sym("cmndEneLast"), sym("cmndObjLast")]
        self.open = sym("aiOpenDirc")
        # NoteProcess hands the script's notes to ccPlayerCheckNote.
        np_ = sym("NoteProcess__5ccAnmFv")
        for i, wd in enumerate(assemble_note_routine()):
            m.store(np_ + 4 * i, 4, wd)
        m.hooks.pop(np_, None)
        m.store(NOTE_LIST, 4, 0)
        # Natives the other harnesses stub: EntryAffect runs the characters'
        # own affectFunc (Influence for Kite, ccFellow::Influence,
        # ccEnemyInfluence), only what shows is stubbed.
        for n in ("checkPartyAnnihilation__Fv", "CheckSpRegeneSpeed__9ccSpcCharFv",
                  "EntryAffect__6ccCharFP6ccCharssss"):
            m.hooks.pop(sym(n), None)
        self.affect_funcs = {1: sym("ccEnemyInfluence__FP6ccChar"), 2: sym("Influence__8ccFellowFv"),
                             3: sym("Influence__FP6ccChar")}
        self.script = {}
        self.skc = 0
        self.install()

    def rec(self, *a):
        self.ai.calls.append(list(a))
        return 0

    def pop(self, key, default):
        s = self.script.get(key)
        return s.pop(0) if s else default

    def name(self, v):
        return self.ai.name(v)

    def ai_body(self, ai):
        return self.name(self.m.load(ai + 0x18, 4)) if ai else 0

    def install(self):
        m, sym, rec = self.m, self.sym, self.rec

        def deg2rad(mm, a, *_):
            v = eemu.f_div(eemu.f_mul(PI, eemu.f_from_int(s16(a))), 0x47000000)
            mm.f[0] = v
            ra = mm.r[31] & 0xFFFFFFFF
            if self.brains[0] <= ra < self.brains[1]:
                rec("FaceTalk", self.ai.cur, s16(a))
            return v

        def skill_request(mm, a, b, c, *_):
            rec("ccSkillRequest", self.name(a), self.name(b), s32(c))
            if self.pop("req", 0):
                mm.store(a + 0x7E, 2, 0)
            return 0

        def chunk(mm, ccs, name, *_):
            return name

        def set_anm(mm, anm, chunk_, *_):
            b = bytes(mm.mem[chunk_:chunk_ + 32])
            rec("SetAnm", b.split(b"\0")[0].decode("latin-1"))
            return 0

        def forward(mm, anm, step, *_):
            rec("AnimateForward", step & 0xFFFF)
            notes = self.pop("notes", [])
            for i, (ev, par) in enumerate(notes):
                a = NOTES + 16 * i
                put_vec(mm, a, [0, ev, par, 0])
                mm.store(NOTE_LIST + 4 * i, 4, a)
            mm.store(NOTE_LIST + 4 * len(notes), 4, 0)
            return self.pop("fwd", 0)

        def hit_switch(on):
            def h(mm, hit, *_):
                rec("HitEnable" if on else "HitDisable", self.name(hit - 0x1A0))
                mm.store(hit, 4, 1 if on else 0)
                return 0
            return h

        def hit_check(mm, this, mp, *_):
            rec("HitCheck", self.name(this), vec(mm, mp))
            r, x, y = self.pop("hit", (0, 0, 0))
            mm.store(mp, 4, x)
            mm.store(mp + 4, 4, y)
            return r

        def land(mm, pos, mask, *_):
            rec("ccLandHitCheck", vec(mm, pos), mask)
            z = self.pop("land", 0)
            res = self.pop("result", -1)
            mm.store(self.hit_num, 4, 0 if res == -1 else 1)
            mm.store(self.hit_near + 0x48, 4, 0 if res == -1 else res)
            mm.f[0] = z
            return z

        def attr(mm, *_):
            rec("checkHitResultAttlibute")
            return self.pop("attr", 0)

        def trans_mode(mm, *_):
            rec("GetTransMode")
            return self.pop("trans", 0)

        def trans_center(mm, wm, v, *_):
            rec("GetTransCenter")
            put_vec(mm, v, self.pop("center", [0, 0, 0, F_ONE]))
            return 0

        def rot(mm, v, cid, *_):
            put_vec(mm, v, self.cam_rot)
            return 0

        def eye(mm, pe, ang, *_):
            rec("cameraSetEyeLevel", vec(mm, pe), vec(mm, ang))
            mm.store(ang + 8, 4, self.pop("eye", 0))
            return 0

        def draw(mm, this, *_):
            rec("Draw", self.name(this), mm.load(this + 0x8C, 4))
            d, t = self.pop("draw", (1, F_ONE))
            mm.store(this + 0x88, 4, t)
            return d

        def skill_check(mm, ch, *_):
            if self.skc == 0:
                return 0
            if self.skc == 1:
                sid, st = self.g.get(ch + 0x7C, "<hh")
                return 1 if sid == 1 and st != 0 else 0
            return 2

        def clear_effect(mm, a, *_):
            rec("ClearConditionEffect", self.name(a))
            mm.store(a + 0x30, 4, 0xFFFFFFFF)
            return 0

        def ptcl(mm, a, b, attr_, *_):
            base = mm.load(self.plw + 0x20, 4)
            return rec("startParticleEffect2", (a - base - 0x160) // 16, (b - base - 0x160) // 16, s32(attr_))

        hooks = {
            "DEG2RAD__Fs": deg2rad,
            "ccSkillRequest__FP6ccCharP6ccChari": skill_request,
            "GetChunkAdrsF__8ccStreamFPCci": chunk,
            "SetAnm__5ccAnmFP10ccAnmChunkUi": set_anm,
            "_AnimateForward__5ccAnmFUi": forward,
            "HitEnable__9ccCharHitFv": hit_switch(True),
            "HitDisable__9ccCharHitFv": hit_switch(False),
            "HitCheck__9ccSpcCharFPf": hit_check,
            "ccLandHitCheck__FPfUi": land,
            "checkHitResultAttlibute__Fv": attr,
            "GetTransMode__9WORLD_MANFv": trans_mode,
            "GetTransCenter__9WORLD_MANFPf": trans_center,
            "AddCenter__9WORLD_MANFff": lambda mm, *_: rec("AddCenter", mm.f[12], mm.f[13]),
            "Enter__9WORLD_MANFPf": lambda mm, wm, p, *_: rec("Enter", vec(mm, p)),
            "cameraGetRot__FPfi": rot,
            "cameraSetEyeLevel__FPfPf": eye,
            "cameraSetManual__FPf": lambda mm, p, *_: rec("cameraSetManual", vec(mm, p)),
            "cameraSet__Fv": lambda mm, *_: rec("cameraSet"),
            "SetMatrix_PosRotZYX__7ccCoordFPfPf": lambda mm, a, p, d, *_: rec("SetMatrix", vec(mm, p), vec(mm, d)),
            "Draw__6ccCharFv": draw,
            "GateHackingOut__8ccPlayerFv": lambda mm, *_: rec("GateHackingOut"),
            "ArmsEffect__9ccSpcCharFv": lambda mm, *_: rec("ArmsEffect"),
            "ClearArmsEffect__9ccSpcCharFv": lambda mm, *_: rec("ClearArmsEffect"),
            "SetArmsEffectColor__9ccSpcCharFi": lambda mm, a, sid, *_: rec("SetArmsEffectColor", s32(sid)),
            "startParticleEffect2__FPA4_fPA4_fiPi": ptcl,
            "effSkillStart__FP6ccChariii": lambda mm, a, b, c, d: rec("effSkillStart", s32(b), s32(c), s32(d)),
            "effTransfer__FP6ccChar": lambda mm, *_: rec("effTransfer"),
            "effWarpTransfer__FP6ccChar": lambda mm, *_: rec("effWarpTransfer"),
            "ccSeSetParamSPC__FUiP6ccChar": lambda mm, a, *_: rec("ccSeSetParamSPC", a),
            "ccEffPawSmoke__FP6ccCharf": lambda mm, *_: rec("ccEffPawSmoke", mm.f[12]),
            "effLevelUp__FP6ccChar": lambda mm, *_: rec("effLevelUp"),
            "effOpenBox__FPf": lambda mm, p, *_: rec("effOpenBox", vec(mm, p)),
            "EquipWeapon__9ccSpcCharFv": lambda mm, *_: rec("EquipWeapon"),
            "DeleteWeaponCCS__9ccSpcCharFv": lambda mm, *_: rec("DeleteWeaponCCS"),
            "SetActuater__5ccPadFiii": lambda mm, pad, a, b, c: rec("SetActuater", s32(a), s32(b), s32(c)),
            # what the affectFuncs show (test_battle_rs.affect_hooks)
            "ccHitMarkDisp__FP6ccCharP6ccChar": lambda mm, a, b, *_: rec("ccHitMarkDisp", self.name(a),
                                                                          self.name(b)),
            "SetPanelBure__10ccMenuCtrlFis": lambda mm, a, b, c, *_: rec("SetPanelBure", s32(b), s16(c)),
            "ccSkillCheck__FP6ccChar": skill_check,
            "AffectMessages__4ccAIFiP6ccChari": lambda mm, a, b, c, d: rec("AffectMessages", self.ai_body(a), s32(b),
                                                                           self.name(c), s32(d)),
            "Greeting__4ccAIFP6ccChari": lambda mm, a, b, c, *_: rec("Greeting", self.ai_body(a), self.name(b),
                                                                     s32(c)),
            "effAfterDrain__FP6ccChari": lambda mm, a, b, *_: rec("effAfterDrain", self.name(a), s32(b)),
            "selectTarget__7ccEnemyFv": lambda mm, a, *_: rec("selectTarget", self.name(a)),
            "effResistantShield__FP6ccCharii": lambda mm, a, b, c, *_: rec("effResistantShield", self.name(a),
                                                                            s32(b), s32(c)),
            "ClearConditionEffect__6ccCharFv": clear_effect,
        }
        for n, f in hooks.items():
            m.hooks[sym(n)] = f
        if volume.NAME != "infection":      # the AI's record of the hit (unnamed; Mutation 0x5c1a60)
            m.hooks[volume.callee("Influence__FP6ccChar", 0)] = lambda mm, a, b, *_: rec("NoteHit", self.ai_body(a),
                                                                                         s32(b))

    # Kite -------------------------------------------------------------------------------
    def put(self, w, k, script):
        """The world, then Kite's ccPlayer, the globals and the script."""
        ai, g, m = self.ai, self.g, self.m
        g.put_env(k["env"])
        ai.put_world(w)
        g.calls = ai.calls
        self.script = {key: list(v) if isinstance(v, list) else v for key, v in script.items()}
        me = w["me"]
        va = ai.va(me)
        ch = w["chars"][me]
        word = k["flags"] | (0x80 if ch.no_death else 0) | ((ch.party_flag & 7) << 14)
        m.store(va + 0xE0, 4, word)
        g.put(va + 0xE4, "<i", k["arms_effect_sw"])
        g.put(va + 0xEE, "<hhhh", k["act"], k["act_old"], k["atk_anm_cnt"], k["anm_flag"])
        g.put(va + 0xF8, "<hh", k["act_cnt"], k["react_cnt"])
        g.put(va + 0xFE, "<hh", k["attack"], k["transfer_lag"])
        m.store(va + 0x104, 4, k["speed"])
        m.store(va + 0x108, 4, k["speed_rate"])
        m.store(va + 0x110, 4, k["cloak"])
        g.put(va + 0x114, "<i", k["cnt"])
        m.store(va + 0x78, 4, ai.va(k["target"]) if k["target"] >= 0 else 0)
        put_vec(m, va + 0x60, k["dirc"])
        m.store(va + 0x80, 4, k["hit_attribute"])
        m.store(va + 0x88, 4, k["transparency"])
        m.store(va + 0x8C, 4, k["set_transparency"])
        m.store(va + 0xCC, 4, 0x01085000)
        m.store(va + 0xD4, 4, ANM)
        m.mem[ANM:ANM + 0x100] = bytes(0x100)
        g.put(ANM + 0x9C, "<H", k["frame_spd"])
        m.store(ANM + 0xAC, 4, 1)
        m.store(va + 0x12C, 4, OBJ)
        m.store(va + 0x130, 4, OBJ + 0x100)
        m.mem[va + 0x1A0:va + 0x1F0] = bytes(0x50)
        m.store(va + 0x1A0, 4, k["hit_sw"])
        m.store(va + 0x1B4, 4, k["hit_radius"])
        m.store(va + 0x1B8, 4, k["hit_height"])
        m.store(va + 0x200, 1, k["pflags"])
        g.put(va + 0x204, "<ii", k["ctrl_type"], k["prog_ctrl_flag"])
        g.put(va + 0x238, "<i", k["dist_tg"])
        m.store(va + 0x23C, 4, k["dirc_tg"])
        for off, key in ((0x240, "pos_view"), (0x260, "pos_eye"), (0x270, "angle"), (0x290, "move_pos"),
                         (0x2D0, "disk_offset")):
            put_vec(m, va + off, k[key])
        g.put(va + 0x2E4, "<hhh", k["target_count"], k["act_one_two_cnt"], k["stress"])
        # the globals
        pad = self.g.m.load(self.sym("ccSys"), 4)
        m.store(pad + 0x2B0, 1, k["pow_l"])
        m.store(pad + 0x2B4, 4, k["dirc_l"])
        m.store(self.cmnd_target, 4, 1 if k["cmnd_target"] else 0)
        m.mem[WM:WM + 0x500] = bytes(0x500)
        m.store(WM + 0x168, 4, k["warp"])
        for i, v in enumerate(k["bounds"]):
            m.store(WM + 0x420 + 4 * i, 4, v)
        m.store(self.dne, 4, k["dne"])
        m.mem[CAM:CAM + 0x70] = bytes(0x70)
        g.put(CAM + 0x5C, "<ii", k["cam_type"], k["cam_reset"])
        m.store(CAM + 0x64, 4, k["cam_reset_dirc"])
        put_vec(m, CAM, k["cam_pos"])
        g.put(self.cam_id, "<h", k["cam_id"])
        self.cam_rot = k["cam_rot"]
        m.mem[self.plw:self.plw + 0x20] = bytes(0x20)
        m.store(self.plw + 0x20, 4, va)
        m.store(NOTE_LIST, 4, 0)
        m.store(self.hit_num, 4, 0)
        m.store(self.open, 4, script["open"])
        self.skc = k["skc"]
        m.store(g.sym("ccMenu"), 4, TB_MENU if k["menu"] else 0)
        # every character's affectFunc, and what ccEnemyInfluence reads
        for i, c in enumerate(w["chars"]):
            a = ai.va(i)
            m.store(a + 0x94, 4, self.affect_funcs.get(getattr(c, "affect_func", 0), 0))
            g.put(a + 0x9C, "<h", getattr(c, "affect_type", 0))
            m.store(a + 0xB0, 4, getattr(c, "affect_mask", 0))
            if c.type & 7 and i != me:
                m.store(a + 0x200, 1, getattr(c, "fellow_flags", 0))
            elif c.type & 0xE0:
                # ccEnemy.eParam (+0x1d0) is the enemy's ccEnemyParam, which
                # affectEnemy reads directly (its real block); the words the
                # AI world keeps in that range (+0x1f4, +0x20c) stay
                keep = [(o, m.load(a + o, 4)) for o in (0x1F4, 0x20C)]
                g.put(a + 0x250, "<H", getattr(c, "enemy_flags", 0))
                m.store(a + 0x238, 4, a + 8)
                m.store(a + 0x244, 4, c.id)
                m.mem[a + 0x1D0:a + 0x1D0 + 0x64] = m.mem[a + 0x400:a + 0x400 + 0x64]
                for o, v in keep:
                    m.store(a + o, 4, v)
        # the command lists' ends, for ccEntryCmnd
        for root, lst, last in zip((g.cmnd_pc, g.cmnd_ene, g.cmnd_obj), (w["pcs"], w["enes"], []), self.lasts):
            m.store(last, 4, ai.va(lst[-1]) if lst else 0)

    def walk(self, root):
        out, a, n = [], self.m.load(root, 4), 0
        while a and n < 20:
            out.append(self.ai.index(a))
            a = self.m.load(a + 0xBC, 4)
            n += 1
        return out

    def read_kite(self, me):
        g, m, va = self.g, self.m, self.ai.va(me)
        k = [m.load(va + 0xE0, 4) & 0x3F7F]
        k += list(g.get(va + 0xEE, "<hhhh"))
        k += list(g.get(va + 0xF8, "<hh"))
        k += list(g.get(va + 0xFE, "<hh"))
        k += [m.load(va + 0x110, 4), g.get(va + 0x114, "<i")[0], g.get(va + 0xE4, "<i")[0]]
        k += list(g.get(va + 0x7C, "<hh"))
        k += [self.ai.index(m.load(va + 0x78, 4))]
        k += vec(m, va + 0x40) + vec(m, va + 0x50) + vec(m, va + 0x60)
        k += [m.load(va + 0x104, 4), m.load(va + 0x108, 4), m.load(va + 0x10C, 4)]
        k += vec(m, va + 0x290)
        k += [g.get(va + 0x11C, "<i")[0], g.get(va + 0xFC, "<h")[0], g.get(va + 0x124, "<i")[0]]
        k += [m.load(va + 0x1A0, 4), m.load(va + 0x80, 4), m.load(va + 0x88, 4), m.load(va + 0x8C, 4)]
        k += [m.load(va + 0x200, 1) & 7] + list(g.get(va + 0x204, "<ii")) + [g.get(va + 0x238, "<i")[0]]
        k += [m.load(va + 0x23C, 4)]
        for off in (0x240, 0x260, 0x270, 0x2D0):
            k += vec(m, va + off)
        k += list(g.get(va + 0x2E4, "<hhh")) + [g.get(ANM + 0x9C, "<H")[0]]
        return k

    def read_affect(self, w):
        """Each character's affect state (type, params, person, the flashes),
        a member's ccFellow flags and an enemy's."""
        g, m = self.g, self.m
        out = []
        for i, ch in enumerate(w["chars"]):
            a = self.ai.va(i)
            row = list(g.shorts(a + 0x9C, 4)) + [self.ai.index(m.load(a + 0x98, 4))]
            row += list(g.shorts(a + 0xA8, 2)) + [m.load(a + 0xAC, 4)] + list(g.shorts(a + 0xB4, 2))
            row += [m.load(a + 0xB8, 4)]
            row.append(m.load(a + 0x200, 1) if ch.type & 7 and i != w["me"] else 0)
            row.append(m.load(a + 0x250, 2) if ch.type & 0xE0 else 0)
            out.append(row)
        return out

    def read(self, w, ret):
        g, m = self.g, self.m
        out = self.ai.read_world(w, ret)
        out["chars"] = self.scene.read(w["chars"])
        out["lists"] = [self.walk(g.cmnd_pc), self.walk(g.cmnd_ene), self.walk(g.cmnd_obj)]
        out["affect"] = self.read_affect(w)
        out["kite"] = self.read_kite(w["me"])
        out["cam"] = [g.get(CAM + 0x60, "<i")[0]]
        out["plw"] = [g.get(self.plw + 8, "<i")[0], m.load(self.plw, 1) & 1]
        out["open"] = m.load(self.open, 4)
        return out

    def poke(self, me, kind, a, b):
        """What the rest of the game does to Kite between two frames (the
        probe's `poke`): a skill requested, a new target, a hit, going down,
        a revival, the stick, a condition, the camera, the flags."""
        g, m, va = self.g, self.m, self.ai.va(me)
        flags = m.load(va + 0xE0, 4)
        if kind == 0:
            g.put(va + 0x7C, "<hh", a, b)
        elif kind == 1:
            m.store(va + 0x78, 4, self.ai.va(a) if a >= 0 else 0)
        elif kind == 2:
            g.put(va + 0xEE, "<h", a)
            m.store(va + 0xE0, 4, (flags | 0x14) & ~0x808)
        elif kind == 3:
            g.put(va + 0xEE, "<h", 9)
            m.store(va + 0xE0, 4, (flags | 0x14) & ~0x808)
            g.put(va + 0x7C, "<hh", 0, 0)
            g.put(va + 0x08, "<h", 2)
            g.put(va + 0x114, "<i", 90)
        elif kind == 4:
            act = g.get(va + 0xEE, "<h")[0]
            if act in (9, 10):
                g.put(va + 0xEE, "<h", 2)
            g.put(va + 0x7C, "<hh", 0, 0)
            g.put(va + 0x08, "<h", 5)
            g.put(va + 0x114, "<i", 0)
            m.store(va + 0x110, 4, 0)
        elif kind == 5:
            pad = m.load(self.sym("ccSys"), 4)
            m.store(pad + 0x2B0, 1, a)
            m.store(pad + 0x2B4, 4, b)
        elif kind == 6:
            g.put(va + 0x08 + 2 * a, "<h", b)
        elif kind == 7:
            g.put(CAM + 0x5C, "<i", a)
        elif kind == 8:
            m.store(va + 0xE0, 4, flags ^ (a & 0x3F7F))

    def run(self, w, k, script, fn, args=(), frames=1, flt=None, pokes=()):
        self.put(w, k, script)
        self.ai.cur = self.name(self.ai.va(w["me"]))
        ret = 0
        for f in range(frames):
            for p in pokes:
                if p[0] == f:
                    self.poke(w["me"], *p[1:])
            if flt is not None:
                self.m.f[12] = flt
            ret = self.m.call(self.fn[fn], args)
        return ret


class Natives:
    """For the runs where ccAI::Brains drives Kite: the party's movement
    runs natively - FollowTarget, FollowPlayer, LeavePlayer,
    FollowTargetDirc, ccPlayer::Attack, ManualControl, and ccSetDirc
    (except Brains' own, the talking member's FaceTalk, which stays a
    stub). The following's ccHitCheckLM (from CheckFrontObstacle(F)) and
    ccHitCheckLM2 are the world's lines, recorded and answered from the
    script's `line`; the decisions' ccHitCheckLM stays the party AI's.
    TransferIn, resignParty and disbandSpc are recorded."""

    NATIVE = ("FollowTarget__4ccAIFP6ccChar", "FollowPlayer__4ccAIFv", "LeavePlayer__4ccAIFv",
              "FollowTargetDirc__4ccAIFP6ccChar", "Attack__8ccPlayerFP6ccChari", "ManualControl__4ccAIFv")

    def __init__(self, kg):
        self.kg = kg

    def __enter__(self):
        kg = self.kg
        m, sym, rec = kg.m, kg.sym, kg.rec
        self.saved = {}
        for n in self.NATIVE:
            self.saved[sym(n)] = m.hooks.pop(sym(n), None)
        ai_hit = m.hooks.get(sym("ccHitCheckLM__FPfPfUi"))
        set_dirc = sym("ccSetDirc__FPffi")

        def line(kind):
            def h(mm, a, b, mask, *_):
                ra = mm.r[31] & 0xFFFFFFFF
                if kind == 0 and not FRONT[0] <= ra < FRONT[1]:
                    return ai_hit(mm, a, b, mask)
                rec("line", vec(mm, a), vec(mm, b), mask & 0xFFFFFFFF, kind)
                r = kg.pop("line", F_M1)
                mm.f[0] = r
                return r
            return h

        def dirc(mm, p, sp, *_):
            ra = mm.r[31] & 0xFFFFFFFF
            if kg.brains[0] <= ra < kg.brains[1]:
                return 0
            h = mm.hooks.pop(set_dirc)
            try:
                nested_call(mm, set_dirc, (p, sp), out=((p, 4),))
            finally:
                mm.hooks[set_dirc] = h
            return 0

        hooks = {
            "ccHitCheckLM__FPfPfUi": line(0),
            "ccHitCheckLM2__FPfPfUi": line(1),
            "ccSetDirc__FPffi": dirc,
            "TransferIn__9ccSpcCharFv": lambda mm, a, *_: rec("TransferIn", kg.name(a)),
            "resignParty__Fi": lambda mm, a, *_: rec("resignParty", s32(a)),
            "disbandSpc__Fi": lambda mm, a, *_: rec("disbandSpc", s32(a)),
        }
        for n, f in hooks.items():
            a = sym(n)
            self.saved.setdefault(a, m.hooks.get(a))
            m.hooks[a] = f
        return self

    def __exit__(self, *exc):
        m = self.kg.m
        for a, h in self.saved.items():
            if h is None:
                m.hooks.pop(a, None)
            else:
                m.hooks[a] = h
        return False


# random cases -----------------------------------------------------------------------

def rnd_notes(rnd):
    n = rnd.choice((0, 0, 0, 0, 1, 1, 2, 3))
    ev = (1, 2, 3, 0x8005, 0x8005, 0x8005, 0x8004, 0x8003, 0x8002, 0x8001, 0, 7)
    return [(rnd.choice(ev), rnd.choice((0, 1, 5, rnd.getrandbits(32)))) for _ in range(n)]


def rnd_script(rnd, frames):
    n = 3 * frames + 8
    ft = lambda: rnd_f(rnd, -60, 60)        # noqa: E731
    return {
        "req": [rnd.choice((0, 0, 1)) for _ in range(n)],
        "fwd": [rnd.choice((0, 0, 0, 0, 1)) for _ in range(n)],
        "notes": [rnd_notes(rnd) for _ in range(n)],
        "land": [rnd.choice((0, ft(), ft())) for _ in range(n)],
        "result": [rnd.choice((-1, -1, 0, 0x80000, 0x80001, 0x20000001, rnd.getrandbits(32))) for _ in range(n)],
        "attr": [rnd.getrandbits(32) for _ in range(n)],
        "hit": [(rnd.choice((0, 1, 2, 3, 3, 3)), rnd.choice((0, ft())), rnd.choice((0, ft()))) for _ in range(n)],
        "trans": [rnd.choice((0, 0, 0, 1)) for _ in range(n)],
        "center": [rnd_vec(rnd, -500, 500) for _ in range(n)],
        "draw": [(rnd.choice((0, 1, 1)), rnd.choice((0, F_ONE, rnd_f(rnd, 0, 1)))) for _ in range(n)],
        "eye": [rnd_f(rnd, -3.2, 3.2) for _ in range(n)],
        "line": [rnd.choice((F_M1, F_M1, F_M1, F_M1, 0, rnd_f(rnd, 0, 1))) for _ in range(8 * n)],
        "open": rnd.choice((0, rnd_f(rnd, -3.2, 3.2))),
    }


SCRIPT_KEYS = ("req", "fwd", "notes", "land", "result", "attr", "hit", "trans", "center", "draw", "eye", "line")


def ser_script(s):
    out = []
    for key in SCRIPT_KEYS:
        v = s[key]
        out.append(str(len(v)))
        for x in v:
            if key == "notes":
                out.append(" ".join(str(y) for y in [len(x)] + [z for pair in x for z in pair]))
            elif isinstance(x, (list, tuple)):
                out.append(" ".join(str(y) for y in x))
            else:
                out.append(str(x))
    out.append(str(s["open"]))
    return " ".join(out)


KITE_KEYS = ("flags", "act", "act_old", "atk_anm_cnt", "anm_flag", "act_cnt", "react_cnt", "attack", "transfer_lag",
             "cloak", "cnt", "arms_effect_sw", "target", "dirc", "speed", "speed_rate", "move_pos", "hit_sw",
             "hit_attribute", "transparency", "set_transparency", "pflags", "ctrl_type", "prog_ctrl_flag", "dist_tg",
             "dirc_tg", "pos_view", "pos_eye", "angle", "disk_offset", "target_count", "act_one_two_cnt", "stress",
             "frame_spd", "pow_l", "dirc_l", "cmnd_target", "warp", "dne", "bounds", "cam_type", "cam_id", "cam_rot",
             "cam_reset", "cam_reset_dirc", "skc", "menu", "hit_radius", "hit_height")


def ser_kite(k):
    out = []
    for key in KITE_KEYS:
        v = k[key]
        out += [str(x) for x in v] if isinstance(v, (list, tuple)) else [str(v)]
    out.append(T.ser_env(k["env"]))
    return " ".join(out)


class KiteChecks:
    def __init__(self, checks):
        self.c = checks
        self.pc = PA.PartyChecks(checks)
        self.kg = KiteGame(self.pc)
        self.b = checks.b
        self.tb = checks.tb
        self.skills = [self.pc.pools.ty[i] for i in range(304)]
        self.anim_kinds = {}
        for i, t in enumerate(self.skills):
            kind = next((b for b in (0x100, 0x200, 0x400, 0x800, 0x1000) if t & b), 0)
            self.anim_kinds.setdefault(kind, []).append(i)

    # a world with Kite --------------------------------------------------------------
    def world(self, rnd, objects=True, **kw):
        pc = self.pc
        w = pc.world(rnd, **kw)
        me = 0
        w["me"] = me
        chars = w["chars"]
        if objects and rnd.random() < 0.15 and len(chars) < 8:
            # a gimmick on the lists: no side's type bits
            o = self.tb.rnd_other(self.b, rnd)
            o.pos_p = T.rnd_pos(rnd)
            o.pos = list(o.pos_p)
            o.width = rnd.choice((0, fl(rnd.uniform(0, 8))))
            o.height = fl(rnd.uniform(0, 30))
            o.skill_id = o.skill_status = o.anm_flag = 0
            o.spc = {"act_num": 0, "target_char": -1, "move_flag": 0, "now_speed": 0, "cycle": 0, "dist_tg": 0,
                     "spc_list_num": 0, "run_flag": 0, "ghost": 0, "stop_flag": 0, "stop_cnt": 0, "walk_run_cnt": 0}
            w["enes"].append(len(chars))
            chars.append(o)
        for ch in chars[1:]:
            if rnd.random() < 0.25:         # far off: beyond the arms' and skills' reach
                f = rnd.choice((5.0, 20.0, 60.0))
                ch.pos_p = [fl(fv(v) * f) if i < 2 else v for i, v in enumerate(ch.pos_p)]
                ch.pos = [fl(fv(v) * f) if i < 2 else v for i, v in enumerate(ch.pos)]
        for i, ch in enumerate(chars):
            # the Char's copies of what the Spc holds (one field in the game)
            sp = ch.spc
            ch.act_num = sp["act_num"]
            ch.target_char = sp["target_char"]
            ch.spc_flags = (sp["move_flag"] << 3) | (sp["stop_flag"] << 4) | (sp["run_flag"] << 5) | (sp["ghost"] << 6)
            if ch.type & 7:
                ch.affect_func = 2 if i in w["ais"] and i != me else 0
                ch.fellow_flags = rnd.choice((0, 0, 1, 2, 3))
            elif ch.type & 0xE0:
                ch.affect_func = 0 if ch.type & 0x80 else 1
                ch.enemy_flags = rnd.choice((0, 0, 0x40, rnd.getrandbits(9) & ~4))
                ch.affect_type = rnd.choice((0, 0, 0, 1, 13))
            else:
                ch.affect_func = 0
        kite = chars[me]
        if rnd.random() < 0.1 and len(chars) > 1:
            # a target exactly -1 away: on the same spot, widths summing to 1
            t = chars[rnd.randrange(1, len(chars))]
            t.pos_p = list(kite.pos_p)
            kite.width = t.width = fl(0.5)
        kite.type = rnd.choice(KITE_TYPES)
        kite.job = rnd.choice((0, 0, 0, 1, 2, 3, 4, 5))
        if kite.equip[4] >= len(self.c.data.weapons[kite.job]):
            kite.equip[4] = 0
        if me not in w["ais"]:
            ids = [a[PA.AI_AT["entry"]][0] for a in w["ais"].values() if a[PA.AI_AT["entry"]][0] >= 0]
            w["ais"][me] = pc.rnd_ai(rnd, me, chars, -1, ids, list(range(len(chars))), w["sys"]["time"])
            kite.ai = 1
        a = w["ais"][me]
        a[PA.AI_AT["param"]] = rnd.choice((0, 0, rnd.randrange(18)))
        if rnd.random() < 0.7:
            a[0] &= ~0x4                    # not talking, mostly
        if rnd.random() < 0.15:
            a[PA.AI_AT["entry"]][0] = -1    # not yet on the bus
        return w

    def kite(self, rnd, w):
        """Kite's ccPlayer: every act and state the code branches on."""
        chars = w["chars"]
        me = w["me"]
        ch = chars[me]
        n = len(chars)
        act = rnd.choice(tuple(range(26)) + (0, 2, 5, 6, 15, 16, 17, 10, 13, 12))
        k = {}
        f = 0
        for bit, p in ((0, 0.15), (1, 0.8), (2, 0.3), (3, 0.4), (4, 0.5), (5, 0.3), (6, 0.2), (10, 0.1), (11, 0.3),
                       (12, 0.1), (13, 0.2)):
            if rnd.random() < p:
                f |= 1 << bit
        f |= rnd.choice((0, 0, 0, 1, 2, 3)) << 8
        k["flags"] = f
        k["act"] = act
        k["act_old"] = rnd.choice((act, act, -1, rnd.randrange(26), 24))
        k["atk_anm_cnt"] = rnd.choice((0, 49, 50, 51, rnd.randrange(0, 200)))
        k["anm_flag"] = rnd.choice((0, 0, 1))
        k["act_cnt"] = rnd.choice((0, 1, 2, 20, 21, 22, 23, 29, 30, 31, 40, 41, 49, 50, 51, 69, 70, 71, 140, 141,
                                   rnd.randrange(-5, 300)))
        k["react_cnt"] = rnd.choice((0, 449, 450, 451, 32767, rnd.randrange(0, 460)))
        k["attack"] = rnd.choice((0, 1, 2, 3, 4, rnd.randrange(0, 6)))
        k["transfer_lag"] = rnd.choice((0, 0, 1, 2))
        k["cloak"] = rnd.choice((F_ONE, 0, 0x3F000000))
        k["cnt"] = rnd.choice((-1, 0, 1, 17, 18, 29, 30, 49, 50, 77, 78, 90, rnd.randrange(-3, 100)))
        k["arms_effect_sw"] = rnd.choice((0, 0, 1))
        k["target"] = rnd.choice((-1, me) + tuple(range(n)) * 3)
        k["dirc"] = [0, 0, rnd_f(rnd, -3.2, 3.2), F_ONE]
        k["speed"] = rnd.choice((fl(27.5), rnd_f(rnd, 0, 40)))
        k["speed_rate"] = rnd.choice((F_ONE, rnd_f(rnd, 0, 1.3)))
        k["move_pos"] = rnd.choice(([0, 0, 0, F_ONE], rnd_vec(rnd, -30, 30)))
        k["hit_sw"] = rnd.choice((0, 1))
        k["hit_attribute"] = rnd.getrandbits(32)
        k["transparency"] = rnd.choice((F_ONE, 0))
        k["set_transparency"] = rnd.choice((F_ONE, 0))
        k["pflags"] = rnd.choice((0, 0, 0, 1, 2, 3, 4))
        k["ctrl_type"] = rnd.choice((0, 0, 0, 0, 1, 1, 2))
        k["prog_ctrl_flag"] = rnd.choice((0, 0, 0, 0, 1))
        k["dist_tg"] = rnd.randrange(-50, 800)
        k["dirc_tg"] = rnd_f(rnd, -3.2, 3.2)
        k["pos_view"] = rnd_vec(rnd)
        k["pos_eye"] = rnd_vec(rnd)
        k["angle"] = [0, 0, rnd_f(rnd, -3.2, 3.2), F_ONE]
        k["disk_offset"] = rnd.choice(([0, 0, 0, F_ONE], rnd_vec(rnd, -100, 100)))
        k["target_count"] = rnd.choice((0, 3, 4))
        k["act_one_two_cnt"] = rnd.choice((0, 0, 1, 2, 30))
        k["stress"] = rnd.choice((0, 7, 8, 99, 100, rnd.randrange(0, 101)))
        k["frame_spd"] = 256
        k["pow_l"] = rnd.choice((0, 0, 63, 64, 100, 127, 128, 200, 230, 231, 255, rnd.randrange(256)))
        k["dirc_l"] = rnd.choice((0, PI, rnd_f(rnd, -3.2, 3.2), rnd_f(rnd, -0.5, 0.5)))
        k["cmnd_target"] = rnd.choice((0, 0, 1))
        k["warp"] = rnd.choice((0, 0, 1))
        k["dne"] = rnd.choice((0, 0, 0, 1))
        lo, hi = rnd.choice(((-24000.0, 24000.0), (-1000.0, 1000.0), (-3000.0, 5000.0), (0.0, 2000.0)))
        k["bounds"] = [fl(lo), fl(lo - rnd.uniform(0, 500)), fl(hi), fl(hi + rnd.uniform(0, 500))]
        k["cam_type"] = rnd.choice((3, 3, 1, 2, 3, 3, 1, 2, 0, 4))
        k["cam_id"] = rnd.choice((1, 1, 0))
        k["cam_rot"] = [0, 0, rnd_f(rnd, -3.2, 3.2), F_ONE]
        k["cam_reset"] = rnd.choice((0, 0, 1))
        k["cam_reset_dirc"] = rnd.choice((k["dirc_l"], rnd_f(rnd, -3.2, 3.2)))
        k["cam_pos"] = rnd_vec(rnd)
        k["skc"] = rnd.choice((0, 1, 1, 2))
        k["menu"] = 1
        k["hit_radius"] = rnd.choice((ch.width, rnd_f(rnd, 5, 30)))
        k["hit_height"] = rnd_f(rnd, 20, 60)
        # the skill: of each animation kind, the normal attack, a cure
        kind = rnd.choice((0, 0x100, 0x200, 0x400, 0x800, 0x1000))
        ch.skill_id = rnd.choice((0, 1, 1, 1, 180, rnd.choice(self.anim_kinds.get(kind, [2])), rnd.randrange(0, 304)))
        ch.skill_status = rnd.choice((0, 0, 1, 1, 2, 2, 3, 9, 8, rnd.randrange(0, 16)))
        # conditions: now and then each that holds or turns him
        ch.cond = [0] * 16 if rnd.random() < 0.5 else ch.cond
        ch.cond[0] = rnd.choice((0, 0, 0, 0, 1, 2, 3, 4, 5))
        for i in (1, 9, 10, 11, 14):
            if rnd.random() < 0.1:
                ch.cond[i] = rnd.randrange(1, 100)
        if rnd.random() < 0.1:              # about to fidget
            k["act"] = act = rnd.choice((0, 2, 0, 2, 1, 3))
            k["react_cnt"] = rnd.choice((449, 450, 450))
            k["cam_type"] = 3
            ch.cond = [0] * 16
            w["game"]["in_battle"] = 0
            w["ais"][me][0] &= ~1
        if rnd.random() < 0.05:             # leaving through the gate
            k["act"] = act = 12
            k["act_cnt"] = rnd.choice((0, 1, 1, 21, 22, 40, 41, 42, 140, 141))
            k["transfer_lag"] = 0
            k["anm_flag"] = 0
        env = self.tb.rnd_env(self.b, rnd)
        env.area = w["game"]["area"]
        env.in_battle = w["game"]["in_battle"]
        env.menu_type = w["game"]["menu_type"]
        k["env"] = env
        self.sync(w, k)
        return k

    @staticmethod
    def sync(w, k):
        """The copies of Kite's state the probe reads: the Char's and the
        Spc's, from `k` (after any steering)."""
        ch = w["chars"][w["me"]]
        f = k["flags"]
        ch.spc_flags = f
        ch.act_num = k["act"]
        ch.attack = k["attack"]
        ch.cnt = k["cnt"]
        ch.cloak = k["cloak"]
        ch.arms_effect_sw = k["arms_effect_sw"]
        ch.anm_flag = k["anm_flag"]
        ch.target_char = k["target"]
        ch.affect_func = 3
        s = ch.spc
        s["act_num"] = k["act"]
        s["target_char"] = k["target"]
        s["move_flag"] = (f >> 3) & 1
        s["run_flag"] = (f >> 5) & 1
        s["ghost"] = (f >> 6) & 1
        s["stop_flag"] = (f >> 4) & 1

    def request(self, fn, frames, args, w, k, script, pokes=()):
        p = " ".join(" ".join(str(x) for x in q) for q in pokes)
        return (f"kite {fn} {frames} {len(args)} {' '.join(str(a) for a in args)} "
                f"{PA.ser_world(w, self.b)} {ser_kite(k)} {ser_script(script)} {len(pokes)} {p}")

    def case(self, rnd, fn, frames=1, argf=None, steer=None, void=True, **kw):
        w = self.world(rnd, **kw)
        k = self.kite(rnd, w)
        if steer:
            steer(rnd, w, k)
            self.sync(w, k)
        script = rnd_script(rnd, frames)
        pokes = rnd_pokes(rnd, frames, len(w["chars"]), fn.startswith("Main")) if frames > 1 else []
        args, gargs, flt = argf(rnd, w, k) if argf else ([], [], None)
        me = w["me"]
        ret = self.kg.run(w, k, script, fn, (self.kg.ai.va(me),) + tuple(a & 0xFFFFFFFF for a in gargs), frames, flt,
                          pokes)
        out = self.kg.read(w, 0 if void else s32(ret))
        self.c.scene.restore()
        return self.request(fn, frames, args, w, k, script, pokes), out, fn


def frames_n(rnd):
    return rnd.randrange(60, 301)


def rnd_pokes(rnd, frames, n_chars, main):
    """Between frames, now and then: a normal attack or a skill asked for
    (as the action button or a menu does through ccSkillRequest), a new
    target, a hit, going down, a revival, the stick, a condition, the
    camera, some flags."""
    out = []
    for f in range(1, frames):
        if rnd.random() > 0.08:
            continue
        kind = rnd.choice((0, 0, 0, 1, 2, 2, 3, 4, 5, 5, 6, 7, 8) if main else (0, 0, 0, 1, 2, 2, 3, 4, 6, 7, 8, 8))
        if kind == 0:
            a, b = rnd.choice(((1, 1), (1, 1), (1, 9), (rnd.randrange(2, 304), 1), (rnd.randrange(0, 304), 9)))
        elif kind == 1:
            a, b = rnd.choice((-1,) + tuple(range(n_chars))), 0
        elif kind == 2:
            a, b = rnd.choice((7, 8)), 0
        elif kind == 5:
            a, b = rnd.choice((0, 0, 100, 200, 255)), rnd_f(rnd, -3.2, 3.2)
        elif kind == 6:
            a, b = rnd.choice((1, 9, 10, 11, 14)), rnd.choice((0, 0, rnd.randrange(1, 60)))
        elif kind == 7:
            a, b = rnd.choice((1, 3, 3)), 0
        elif kind == 8:
            a, b = rnd.choice((0x8, 0x20, 0x28, 0x10, 0x1, 0x4, 0x40, 0x2)), 0
        else:
            a, b = 0, 0
        out.append((f, kind, a, b))
    return out


def make_checks(kc):
    kg = kc.kg

    def anim(rnd, frames=1):
        def steer(rnd, w, k):
            me = w["me"]
            ch = w["chars"][me]
            r = rnd.random()
            if r < 0.3:
                # the normal attack's combo, often
                ch.skill_id = 1
                ch.skill_status = rnd.choice((1, 2, 2))
                k["attack"] = rnd.choice((0, 1, 2, 3, 4, 4))
                if rnd.random() < 0.5:
                    k["act"] = rnd.choice((0, 15, 16, 2))
                if k["attack"] == 4 and rnd.random() < 0.5:
                    # the third swing's reach, to a target exactly -1 away
                    t = rnd.randrange(len(w["chars"]))
                    k["target"] = t
                    w["chars"][t].pos_p = list(ch.pos_p)
                    if t != me:
                        ch.width = w["chars"][t].width = fl(0.5)
                    else:
                        ch.width = fl(0.5)
            elif r < 0.4:
                # lying down while the party is wiped out
                ch.cond[0] = rnd.choice((3, 4))
                k["act"] = 10
                k["cnt"] = rnd.choice((49, 50, 51))
                for i in w["party"]["members"]:
                    if i >= 0 and i != me and rnd.random() < 0.8:
                        w["chars"][i].cond[0] = rnd.choice((2, 3, 4))
            elif r < 0.5:
                # arriving, by warp or not, as a ghost
                k["act"] = 13
                k["act_cnt"] = rnd.choice((0, 0, 20, 21, 30, 39, 40, 41, 49, 50, 51, 60, 69, 70, 71))
                k["transfer_lag"] = 0
                k["anm_flag"] = 0
                if rnd.random() < 0.5:
                    k["flags"] |= 0x40
        return kc.case(rnd, "AnimCtrl__8ccPlayerFv", frames, steer=steer)

    def main(rnd, frames=1):
        def steer(rnd, w, k):
            ch = w["chars"][w["me"]]
            if rnd.random() < 0.3:          # charmed or confused: the AI drives him
                ch.cond[rnd.choice((10, 11))] = rnd.randrange(1, 300)
            if rnd.random() < 0.2:
                ch.cond[0] = 2
            if rnd.random() < 0.25:
                # the timers' affects: a poison tick that may down him inside
                # CalcReal, regeneration, curse
                ch.cond[0] = rnd.choice((0, 0, 0, 5))
                ch.cond[13] = rnd.choice((1, 91, 181, 2))
                ch.cond[12] = rnd.choice((0, 61, 1))
                ch.cond[7] = rnd.choice((0, 61, 1))
                ch.cond[8] = rnd.choice((0, 0, 91, 1))
                ch.HP = rnd.choice((1, 1, 2, ch.HP))
                ch.no_death = rnd.random() < 0.1
                ch.exp = rnd.choice((0, 0, 999, 1000))
                w["game"]["menu_type"] = k["env"].menu_type = rnd.choice((-1, -1, -1, 0))
        return kc.case(rnd, "Main__8ccPlayerFv", frames, steer=steer)

    def control(rnd):
        return kc.case(rnd, "ControlMove__8ccPlayerFv", void=False)

    def attack(rnd):
        def argf(rnd, w, k):
            t = rnd.randrange(len(w["chars"]))
            n = rnd.randrange(0, 6)
            return [t, n], [kg.ai.va(t), n], None

        def steer(rnd, w, k):
            ch = w["chars"][w["me"]]
            if rnd.random() < 0.5:
                ch.skill_id = rnd.choice((0, 1))
            if rnd.random() < 0.7:
                ch.cond[10] = ch.cond[11] = 0
            k["act"] = rnd.choice((0, 0, 0, 2, 5, 6, 7, 8, 9, 15, 16, k["act"]))
            ch.spc["cycle"] = rnd.choice((0, 180, 360, 0, 1, rnd.randrange(0, 1000)))
            for t in w["chars"]:
                if rnd.random() < 0.3:
                    t.cond[0] = 1
            PA.open_up(rnd, w, 3)
        hook = kg.sym("Attack__8ccPlayerFP6ccChari")
        f = kg.m.hooks.pop(hook, None)
        try:
            return kc.case(rnd, "Attack__8ccPlayerFP6ccChari", argf=argf, steer=steer, void=False, no_free=True)
        finally:
            if f is not None:
                kg.m.hooks[hook] = f

    def note(rnd):
        def argf(rnd, w, k):
            ev = rnd.choice((1, 2, 3, 0x8005, 0x8005, 0x8005, 0x8001, 0x8002, 0x8003, 0x8004, 0, 9))
            par = rnd.choice((0, 3, rnd.getrandbits(32)))
            put_vec(kg.m, NOTES + 0x200, [0, ev, par, 0])
            return [ev, par], [NOTES + 0x200], None
        w = kc.world(rnd)
        k = kc.kite(rnd, w)
        script = rnd_script(rnd, 1)
        args, gargs, _ = argf(rnd, w, k)
        kg.put(w, k, script)
        put_vec(kg.m, NOTES + 0x200, [0, args[0], args[1], 0])
        kg.ai.cur = kg.name(kg.ai.va(w["me"]))
        kg.m.call(kg.fn["ccPlayerCheckNote__FP9ccAnmNote"], (NOTES + 0x200,))
        out = kg.read(w, 0)
        kc.c.scene.restore()
        return kc.request("CheckNote", 1, args, w, k, script), out, "check_note"

    def small(rnd):
        which = rnd.choice(("AttackCancel", "BreakSomething", "DamageActuate", "ResultOfConditions",
                            "ccPlayerMenuCheck", "CheckControlMode", "SetTargetDist", "SetTargetDirc",
                            "ReadSysMsg2"))
        mangled = {"AttackCancel": "AttackCancel__8ccPlayerFv",
                   "BreakSomething": "BreakSomething__8ccPlayerFP6ccChar",
                   "DamageActuate": "DamageActuate__8ccPlayerFi",
                   "ResultOfConditions": "ResultOfConditions__8ccPlayerFv",
                   "ccPlayerMenuCheck": "ccPlayerMenuCheck__Fv",
                   "CheckControlMode": "CheckControlMode__9ccSpcCharFv",
                   "SetTargetDist": "SetTargetDist__8ccPlayerFv",
                   "SetTargetDirc": "SetTargetDirc__8ccPlayerFv",
                   "ReadSysMsg2": "ReadSysMsg2__4ccAIFv"}[which]
        w = kc.world(rnd)
        k = kc.kite(rnd, w)
        me = w["me"]
        if which == "AttackCancel":
            k["act"] = rnd.choice((15, 16, 25, 25, k["act"]))
        elif which in ("BreakSomething", "SetTargetDirc") and rnd.random() < 0.5:
            w["ais"][me][0] |= 1
        elif which == "ccPlayerMenuCheck":
            k["act"] = rnd.choice((12, 13, k["act"]))
        kc.sync(w, k)
        script = rnd_script(rnd, 1)
        args, gargs = [], []
        this = kg.ai.va(me)
        if which == "BreakSomething":
            t = rnd.choice((-1,) + tuple(range(len(w["chars"]))))
            args, gargs = [t], [kg.ai.va(t) if t >= 0 else 0]
        elif which == "DamageActuate":
            d = rnd.choice((0, -1, 1, 9, 10, 99, 100, 5000, rnd.randrange(-100, 1000)))
            args, gargs = [d], [d]
        elif which == "ccPlayerMenuCheck":
            sc = rnd.choice((0, 0, 1, 2, 3))
            args = [sc]
            saved_check = kg.m.hooks.get(kg.sym("ccSkillCheck__FP6ccChar"))
            kg.m.hooks[kg.sym("ccSkillCheck__FP6ccChar")] = lambda mm, *_: sc
        elif which == "ReadSysMsg2":
            this = PA.AIS + 0x400 * me
        kg.put(w, k, script)
        kg.ai.cur = kg.name(kg.ai.va(me))
        ret = kg.m.call(kg.fn[mangled], (this,) + tuple(a & 0xFFFFFFFF for a in gargs))
        if which == "ccPlayerMenuCheck":
            kg.m.hooks[kg.sym("ccSkillCheck__FP6ccChar")] = saved_check
        out = kg.read(w, s32(ret) if which in ("ccPlayerMenuCheck", "CheckControlMode") else 0)
        kc.c.scene.restore()
        return kc.request(which, 1, args, w, k, script), out, which

    def frames(rnd):
        """The frame conversions on random maps, Kite anywhere on them."""
        m = kg.m
        lo = [rnd.uniform(-30000, 5000) for _ in range(2)]
        hi = [x + rnd.choice((rnd.uniform(1, 60000), 2000.0, 48000.0)) for x in lo]
        bounds = [fl(lo[0]), fl(lo[1]), fl(hi[0]), fl(hi[1])]
        if rnd.random() < 0.2:
            bounds = [fl(-24000.0), fl(-24000.0), fl(24000.0), fl(24000.0)]

        def pt():
            k = rnd.random()
            if k < 0.5:
                return [fl(rnd.uniform(lo[0] - 3000, hi[0] + 3000)), fl(rnd.uniform(lo[1] - 3000, hi[1] + 3000)),
                        rnd_f(rnd, -500, 500), rnd.choice((F_ONE, 0))]
            if k < 0.8:
                return [fl(rnd.uniform(-60000, 60000)), fl(rnd.uniform(-60000, 60000)), rnd_f(rnd, -500, 500), F_ONE]
            return [rnd.choice((bounds[0], bounds[2])), rnd.choice((bounds[1], bounds[3])), 0, F_ONE]
        me_pos, p = pt(), pt()
        m.mem[WM:WM + 0x500] = bytes(0x500)
        for i, v in enumerate(bounds):
            m.store(WM + 0x420 + 4 * i, 4, v)
        this = T.SCN
        put_vec(m, this + 0x40, me_pos)
        m.store(kg.plw + 0x20, 4, this)
        w2p = kg.sym("ccTransPosW2P__FPfPf")
        hook = m.hooks.pop(w2p, None)
        out = {}
        for key, fn, args in (("W2MPos", "W2MPos__8ccPlayerFPfPf", (this, OUTV, OUTV + 0x20)),
                              ("W2PPos", "W2PPos__8ccPlayerFPfPf", (this, OUTV, OUTV + 0x20)),
                              ("P2WPos", "P2WPos__8ccPlayerFPfPf", (this, OUTV, OUTV + 0x20)),
                              ("ccTransPosW2M", "ccTransPosW2M__FPfPf", (OUTV, OUTV + 0x20)),
                              ("ccTransPosW2P", "ccTransPosW2P__FPfPf", (OUTV, OUTV + 0x20)),
                              ("ccTransPosP2W", "ccTransPosP2W__FPfPf", (OUTV, OUTV + 0x20)),
                              ("ccTransPosFW2LW", "ccTransPosFW2LW__FPfPf", (OUTV, OUTV + 0x20))):
            put_vec(m, OUTV, [0x12345678] * 4)
            put_vec(m, OUTV + 0x20, p)
            r = m.call(kg.fn[fn], args)
            out[key] = vec(m, OUTV) + ([s32(r)] if key[0] != "c" else [])
        if hook is not None:
            m.hooks[w2p] = hook
        req = "kite frames " + " ".join(str(x) for x in bounds + me_pos + p)
        return req, out, "frames"

    def main_ai(rnd):
        """Main for 60-300 frames with the AI driving Kite in a field:
        charmed, confused or under an event's remote control (manualSW).
        On the game's side the following, ccPlayer::Attack and
        ManualControl run natively (Natives); the port's runtime is
        kite::Host over party_motion::Movement. Compared after every
        frame."""
        frames = frames_n(rnd)
        w = kc.world(rnd, no_free=True)
        k = kc.kite(rnd, w)
        me = w["me"]
        ch = w["chars"][me]
        a = w["ais"][me]
        at = PA.AI_AT
        w["game"]["area"] = 1
        w["game"]["field_24"] = 0
        k["env"].area = 1
        a[at["mode"]] = rnd.choice((1, 1, 1, 2, 3, 4, 5, 6, a[at["mode"]]))
        # a field's navigation has always arrived (ccNavi's finishFlag 1;
        # only a dungeon's path finding clears it): AttackTarget follows
        # its target, never the dungeon's beacons
        a[at["naviFinish"]] = 1
        if rnd.random() < 0.85:
            a[0] &= ~0x4                            # not talking
        if rnd.random() < 0.8:
            ch.cond[0] = rnd.choice((0,) * 8 + (4, 5))
        for i in (1, 9, 14):
            if rnd.random() < 0.7:
                ch.cond[i] = 0
        if rnd.random() < 0.9:
            k["act"] = rnd.choice((0, 0, 2, 2, 5, 6, 15, 16, 7, 8, 13, 14, 12, 25))
        if rnd.random() < 0.9:
            k["prog_ctrl_flag"] = 0
        mode = rnd.choice(("charm", "confusion", "manual", "charm", "confusion", "manual", "ghost"))
        if mode == "ghost":
            # a charmed or confused ghost: Brains' mode 6, which follows
            # Kite (FollowPlayer) while followSW holds - the one way to
            # FollowPlayer from his own frame (every other call site in
            # ActInField sits behind the charm and confusion checks)
            ch.cond[0] = 4
            k["flags"] |= 0x40
            ch.party_flag = 1
            a[0] = (a[0] & ~1) | (2 if rnd.random() < 0.8 else 0)
            a[at["mode"]] = rnd.choice((6, 6, 6, 1))
            ch.cond[rnd.choice((10, 11))] = rnd.randrange(30, 400)
        elif mode == "manual":
            a[0] |= 1
            cmd = rnd.choice((0, 0, 1, 1, 2, 2, 3, 4, 6, 7, 5))
            a[at["remoteCmd"]] = cmd
            a[at["gPoint"]] = -1
            a[at["gRotSp"]] = rnd.choice((64, 64, 16, 1, 256))
            a[at["gDeg"]] = rnd.randrange(0, 65536)
            r = rnd.choice((20, 45, 60, 300, 900))
            h = rnd.uniform(-math.pi, math.pi)
            a[at["gPos"]] = [fl(fv(ch.pos[0]) + r * math.sin(h)), fl(fv(ch.pos[1]) - r * math.cos(h)),
                             ch.pos[2], F_ONE]
        else:
            a[0] &= ~1
            ch.cond[10 if mode == "confusion" else 11] = rnd.choice((rnd.randrange(30, 120),
                                                                   rnd.randrange(200, 400)))
            if rnd.random() < 0.25:
                # out of the party: wary (mode 2) he only turns to his
                # target (FollowTargetDirc)
                ch.party_flag = rnd.choice((0, 2))
                a[at["mode"]] = rnd.choice((2, 2, 1))
        kc.sync(w, k)
        script = rnd_script(rnd, frames)
        pokes = rnd_pokes(rnd, frames, len(w["chars"]), True)
        va = kg.ai.va(me)
        game = []
        with Natives(kg):
            kg.put(w, k, script)
            kg.ai.cur = kg.name(va)
            for f in range(frames):
                for q in pokes:
                    if q[0] == f:
                        kg.poke(me, *q[1:])
                start = len(kg.ai.calls)
                kg.m.call(kg.fn["Main__8ccPlayerFv"], (va,))
                out = kg.read(w, 0)
                out["calls"] = list(kg.ai.calls[start:])
                game.append(out)
        kc.c.scene.restore()
        return kc.request("MainRunAi", frames, [], w, k, script, pokes), {"frames": game}, "main_run_ai " + mode

    return [
        ("anim_ctrl", anim),
        ("anim_run", lambda rnd: anim(rnd, frames_n(rnd))),
        ("main", main),
        ("main_run", lambda rnd: main(rnd, frames_n(rnd))),
        ("control_move", control),
        ("attack", attack),
        ("check_note", note),
        ("small", small),
        ("frames", frames),
        ("main_run_ai", main_ai),
    ]


def diff_frames(port, game, what):
    """The first frame that differs, and where."""
    if not isinstance(port, dict) or "frames" not in port:
        return T.diff(port, game, what)
    pf, gf = port["frames"], T.norm(game)["frames"]
    for i, (a, g) in enumerate(zip(pf, gf)):
        d = T.diff(a, g, f"{what} frame {i}")
        if d:
            return d
    if len(pf) != len(gf):
        return f"{what}: {len(pf)} frames, game {len(gf)}"
    return None


def run(checks, name, n, seed):
    """test_battle_rs.run for these checks: a run's answers (a state per
    frame) go to the probe a few at a time and compare frame by frame."""
    if name not in RUNS:
        return T.run(checks, name, n, seed)
    rnd = random.Random(seed)
    fn = T.EXTRA[name]
    bad, first, first_req = 0, None, None
    done = 0
    while done < n:
        k = min(6, n - done)
        lines, answers = [], []
        for _ in range(k):
            checks.reset()
            req, game, label = fn(checks, rnd)
            lines.append(req)
            answers.append((game, label))
        got = T.ask(lines)
        for j, (game, label) in enumerate(answers):
            d = diff_frames(got[j], game, label)
            if d:
                bad += 1
                if first is None:
                    first, first_req = d, lines[j]
        done += k
    return bad, first, first_req


def bulk(n, only, names, seed0):
    """test_battle_rs.bulk with this module's runner."""
    T.EXTRA.update(KITE)
    T.build()
    checks = Kite()
    total = 0
    for k, name in enumerate(names):
        if only and name not in only:
            continue
        bad, first, req = run(checks, name, n, seed0 + k)
        total += bad
        print(f"{name:32} {n:6} cases, {bad} mismatches" + (f"; first: {first}" if first else ""))
        if first:
            print(f"  request: {req[:600]}")
    return total


class Kite(T.Checks):
    """The checks as test_battle_rs's runner takes them."""

    def __init__(self):
        super().__init__()
        self.kc = KiteChecks(self)
        self.fn = dict(make_checks(self.kc))


def _check(name):
    return lambda checks, rnd: checks.fn[name](rnd)


CHECK_NAMES = ["anim_ctrl", "anim_run", "main", "main_run", "control_move", "attack", "check_note", "small", "frames",
               "main_run_ai"]
KITE = {name: _check(name) for name in CHECK_NAMES}
RUNS = ("main_run_ai",)


@unittest.skipUnless(T.READY, "needs the extracted disc and cargo")
class KiteAgainstGame(T.Against):
    CASES = 150
    TABLE = KITE
    MAKE = Kite

    def test_anim_ctrl(self):
        self.check("anim_ctrl", 24000)
        self.check("anim_run", 24001)

    def test_main(self):
        self.check("main", 24002)
        self.check("main_run", 24003)

    def test_move_and_attack(self):
        self.check("control_move", 24004)
        self.check("attack", 24005)
        self.check("check_note", 24006)

    def test_small(self):
        self.check("small", 24007)
        self.check("frames", 24008)

    def test_main_run_ai(self):
        bad, first, req = run(self.checks, "main_run_ai", 24, 24009)
        self.assertEqual(bad, 0, f"{first}\nrequest: {(req or '')[:2000]}")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "bulk":
        n = int(sys.argv[2]) if len(sys.argv) > 2 else 500
        sys.exit(1 if bulk(n, sys.argv[3:], CHECK_NAMES, 24100) else 0)
    unittest.main()
