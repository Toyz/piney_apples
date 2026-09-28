#!/usr/bin/env python3
"""crates/piney-battle's enemy motion (src/enemy_motion.rs, the motion
helpers of src/geom.rs) against the game's own enemy.cpp and the races'
code run natively in the Rust VU0 machine (tools/eemu_rs.so).

Each check lays out tools/test_battle_enemy_ai_rs.py's scene of party
members and enemies (ccChar/ccEnemy at SCN + 0x1000 * i), with the acting
enemy built from its row's own constructor (ccEntryEnemyX, run once per row
with the model, CCS, weapon and dust set-up stubbed: its body hit, animation
table, eneType, controllers, offsets), then randomised; its players (ccAnm)
at +0x900 and +0xa20, the notes they pass at +0xb40. The world is a script:
the player's frame is a shift, and ccLandHitCheck, CollisionDetection,
ccHitCheckLM2, checkCameraShakeRange, ccGetCameraTransparency and
_AnimateForward (with the notes it passes, which the game's own NoteProcess
hands to ccEnemyCheckNote) answer with random values drawn when the game
asks and sent to battle_probe in that order. EntryAffect, the affect
functions (ccEnemyInfluence/affectEnemy, ccFellow::Influence, Influence)
and ccSkillDamage run natively, so an affect lands where the game makes it
(routineEnemy's poison and regeneration ticks, a special attack's hold, an
attack's hit on note 0x8005); every character carries its affectFunc and the
fields its influence reads. Presentation is stubbed and recorded: numbers,
hit marks, sounds, the camera shake, the weapon and dust controllers, the
draws (with the matrices the code built), a gold goblin's dust; a skill
request (_ccSkillRequest) is recorded.

Compared: the return, every character (with its affect fields, and a party
member's ccSpcChar/ccFellow words and target), every enemy's ccEnemy fields
and body hit, the command lists, both generators (newlib's rand() and
ccRand's Mersenne Twister), the enemy book, and the calls in order (the
world's queries with their arguments, the presentation, the rules'
outputs). EntryAffect and selectTarget run in both and are compared by what
they change.

    python3 tools/test_battle_enemy_motion_rs.py            the unit tests
    python3 tools/test_battle_enemy_motion_rs.py bulk N     N cases of every check

Checks (MOTION maps a name to fn(checks, rnd) -> (request, game, label)):
  helpers     ccSetRad, ccSetDist, ccGetDircChgF, ccSetDirc, ccGetDirc,
              ccGetDist, ccSetRadDisperse (main), ccPiLimit (both), and
              libvu0's RotMatrixZ/X/Y, RotMatrix, ApplyMatrix, TransMatrix,
              UnitMatrix
  act         actMove (0x004371e0), actFollow, actSlide
  escape      actEscape() (0x004374a0), actEscape(int, float), actEscapeX
  move        moveEnemy (0x00433e80)
  anim        animEnemy (0x00434560) with noteEnemy and ccEnemyCheckNote
  note        ccEnemyCheckNote (0x004328b0) and the race's note()
  disp        dispEnemy (0x004348b0)
  action      each race's action(): ccEnemyG, P, K, V, B, 1
  moveex      moveEG, moveEGG, moveEB
  excl        each race's exclusive() and note(), ccEnemyG::freeze
  main        ccEnemy::main (0x00432cd0) for 1-3 enemies of the races over
              60-300 frames, party members moving on scripted paths,
              events on the enemies between frames; compared every frame
"""

import copy
import math
import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import volume  # noqa: E402  # PINEY_VOLUME's disc (volume.py)
from volume import va as inf_va  # noqa: E402

import eemu  # noqa: E402
import test_anim  # noqa: E402

eemu.Machine = test_anim.machine_class()

import test_battle_enemy_ai_rs as eai  # noqa: E402
import test_battle_rs as rs  # noqa: E402
from test_battle_enemy_ai_rs import ENEMY_TBL, ROW, rangle, rfloat, rvec, s16, s32  # noqa: E402
from test_battle_rs import F_ONE, fbits, ser_env  # noqa: E402

MINUS_ONE = 0xBF800000
# the races ported, by ccEntryRaceTbl group
KINDS = {0: "E1", 1: "E2", 2: "E3", 3: "E4", 4: "A", 5: "B", 6: "C", 7: "D", 8: "E", 9: "F", 10: "G", 11: "H",
         12: "I", 13: "K", 14: "L", 15: "P", 16: "S", 17: "T", 18: "U", 19: "V", 20: "W", 21: "Z"}
# each race's vtable: 0x00376110 on, 0x20 apart by ccEntryRaceTbl index
VTABLES = {k: inf_va(0x00376110) + 0x20 * r for r, k in KINDS.items()}
# the kinds the random picks draw from (G the most: its rules are the most)
PICK = ("G", "G", "P", "K", "V", "B", "E1", "E2", "C", "F", "H", "I", "U", "E3", "E4", "A", "D", "E", "S", "T", "W",
        "Z", "L", "L")
FOOT_OBJ_TBL = inf_va(0x005E0640)   # ccEnemyE's feet (30-byte names)
# the first field and dungeon's rows (ccGame.field 14)
AREA_ROWS = {"G": (130, 151), "P": (219,), "K": (185, 189), "V": (268,), "B": (67,)}
# wc and dc (weapon and dust controllers) in the race's object
CTRL_OFS = {"G": (0x374, 0x378), "C": (0x348, 0x34C)}
# rows whose exclusive() does more than the controllers: ccEnemyH types 3-4,
# ccEnemyC type 3, ccEnemyF type 2
RACE_EXCL = (174, 175, 176, 86, 87, 120, 121, 97, 98, 99, 113, 114, 197, 199, 207, 210)
BR_OFS = 0xD20          # ccEnemyH's two breaths (+0x348, +0x34c) point here
ENT = 0x01034400        # a ccEntry for the constructors, its ccEntryParam at +0x100
CTOR = 0x01240000       # where the constructors' objects are allocated
CHUNK = 0x01037000      # what GetChunkAdrsF hands out
STREAM, STREAM2 = 0x01037100, 0x01037200
ANM_OFS, ANM2_OFS, NOTES_OFS, WC_OFS, DC_OFS = 0x900, 0xA20, 0xB40, 0xD00, 0xD10
MOVE_LO, MOVE_HI = volume.span("moveEnemy__7ccEnemyFv")   # moveEnemy: its W2P/P2W are logged
NOTE_EVENTS = (1, 2, 0x8005, 0x8003, 0x8002, 3, 0x8004, 0x8001, 0, 5)


def fl(v):
    return struct.unpack("<f", struct.pack("<I", v & 0xFFFFFFFF))[0]


class Motion(eai.Harness):
    """The motion set-up on top of test_battle_enemy_ai_rs.Harness."""

    def __init__(self, checks):
        super().__init__(checks)
        g, m, sym = self.g, self.m, self.g.sym
        self.kinds = {}
        for eid in range(303):
            r = self.race(eid)
            # ccEnemyL's type 3 (203-206) is not ported
            if r and r[0] in KINDS and not 203 <= eid <= 206:
                self.kinds[eid] = KINDS[r[0]]
        self.rows = {k: [e for e, kk in self.kinds.items() if kk == k] for k in VTABLES}
        self.templates = {}
        self.note_char = sym("ccEnemyNoteChar")
        self.check_note_fn = sym("ccEnemyCheckNote__FP9ccAnmNote")
        self.script = None
        self.rnd = None
        self.ox = self.oy = 0
        self.collide_p = 0.35
        self.anm_owner = {}
        self.last_name = b""
        self.hooks = self.motion_hooks()
        self.running = 0
        self.entry_affect_fn = sym("EntryAffect__6ccCharFP6ccCharssss")
        self.affect_funcs = {1: sym("ccEnemyInfluence__FP6ccChar"), 2: sym("Influence__8ccFellowFv"),
                             3: sym("Influence__FP6ccChar")}
        rec, b = g.record, self.b
        # EntryAffect and the affect functions run; their presentation is
        # recorded (tools/test_battle_rs.py's affect_hooks, less selectTarget,
        # which runs, and ClearConditionEffect, which runs and shows nothing)
        self.affect_stubs = {
            "ccHitMarkDisp__FP6ccCharP6ccChar": lambda mm, a, b_, *_: rec("ccHitMarkDisp", a, b_),
            "DamageActuate__8ccPlayerFi": lambda mm, a, b_, *_: rec("DamageActuate", b_),
            "SetPanelBure__10ccMenuCtrlFis": lambda mm, a, b_, c, *_: rec("SetPanelBure", b_, b.s16(c)),
            "ccSkillCheck__FP6ccChar": lambda mm, *_: self.running,
            "HitDisable__9ccCharHitFv": lambda mm, a, *_: rec("HitDisable", a - 0x1A0),
            "HitEnable__9ccCharHitFv": lambda mm, a, *_: rec("HitEnable", a - 0x1A0),
            "ChatMessageDamage__4ccAIFi": lambda mm, a, b_, *_: rec("ChatMessageDamage", a, b_),
            "ChatMessageResurrectPlz__4ccAIFv": lambda mm, a, *_: rec("ChatMessageResurrectPlz", a),
            "AffectMessages__4ccAIFiP6ccChari": lambda mm, a, b_, c, d: rec("AffectMessages", a, b_, c, d),
            "Greeting__4ccAIFP6ccChari": lambda mm, a, b_, c, *_: rec("Greeting", a, b_, c),
            "ccAISysMsgSendP__FisUsUsUsUsiPv":
                lambda mm, a, b_, c, d: rec("ccAISysMsgSendP", a, b.s16(b_), c, d, mm.r[11]),
            "ccAISysMsgDeleteDelay__FUiUsUs": lambda mm, a, b_, c, *_: rec("ccAISysMsgDeleteDelay", a, b_, c),
            "effResistantShield__FP6ccCharii": lambda mm, a, b_, c, *_: rec("effResistantShield", a, b_, b.s32(c)),
        }

    # the world ------------------------------------------------------------------------
    def answer(self, kind):
        """The next answer of a world function: drawn now, kept for the probe."""
        rnd = self.rnd
        if kind == "land":
            v = rnd.choice((0, fbits(rnd.uniform(-30, 30)), fbits(rnd.uniform(-300, 300))))
        elif kind == "collide":
            r = 0 if rnd.random() > self.collide_p else rnd.choice((1, 1, 2, 3))
            off = [rfloat(rnd, -40, 40), rfloat(rnd, -40, 40), rfloat(rnd, -10, 10), rnd.choice((0, F_ONE))]
            v = (r, off, rnd.randrange(0, 8))
        elif kind == "line":
            v = rnd.choice((MINUS_ONE,) * 5 + (fbits(rnd.uniform(0, 1)), 0))
        elif kind == "shake":
            v = rnd.choice((0, 1, 1))
        elif kind == "trans":
            v = rnd.choice((F_ONE, fbits(rnd.uniform(0, 1)), 0))
        else:  # anim
            ret = rnd.choice((0,) * 5 + (1,))
            nxt = rnd.randrange(0, 0x10000) if rnd.random() < 0.3 else rnd.randrange(0, 200)
            notes = []
            for _ in range(rnd.choice((0, 0, 0, 1, 1, 2, 3))):
                ev = rnd.choice(NOTE_EVENTS + (rnd.randrange(0, 0x10000),))
                notes.append((ev, rnd.choice((0, 1, 2, 3, rnd.randrange(0, 300)))))
            v = (ret, nxt, notes)
        self.script[kind].append(v)
        return v

    def owner(self, anm):
        return self.anm_owner.get(anm, (anm, 0))

    def motion_hooks(self):
        g = self.g

        def vec(mm, a):
            return [mm.load(a + 4 * k, 4) for k in range(4)]

        def mat(mm, a):
            return [mm.load(a + 4 * k, 4) for k in range(16)]

        def w2p(mm, out, inp, *_):
            v = vec(mm, inp)
            r = [eemu.f_sub(v[0], self.ox), eemu.f_sub(v[1], self.oy), v[2], v[3]]
            for k in range(4):
                mm.store(out + 4 * k, 4, r[k])
            if MOVE_LO <= (mm.r[31] & 0xFFFFFFFF) < MOVE_HI:
                g.calls.append(("w2p",) + tuple(v))
            return 0

        def p2w(mm, out, inp, *_):
            v = vec(mm, inp)
            r = [eemu.f_add(v[0], self.ox), eemu.f_add(v[1], self.oy), v[2], v[3]]
            for k in range(4):
                mm.store(out + 4 * k, 4, r[k])
            if MOVE_LO <= (mm.r[31] & 0xFFFFFFFF) < MOVE_HI:
                g.calls.append(("p2w",) + tuple(v))
            return 0

        def land(mm, pos, mask, *_):
            g.calls.append(("land",) + tuple(vec(mm, pos)) + (mask,))
            v = self.answer("land")
            mm.f[0] = v
            return v

        def collide(mm, this, *_):
            g.calls.append(("collide", g.name(this - 0x160), mm.load(this + 0x14, 4), mm.load(this + 0x18, 4))
                           + tuple(vec(mm, this + 0x20)))
            r, off, attr = self.answer("collide")
            for k in range(4):
                mm.store(this + 0x30 + 4 * k, 4, off[k])
            mm.store(this + 0x40, 4, attr)
            return r

        def line(mm, a, b, mask, *_):
            g.calls.append(("line",) + tuple(vec(mm, a)) + tuple(vec(mm, b)) + (mask, 1))
            v = self.answer("line")
            mm.f[0] = v
            return v

        def chunk(mm, stream, name, *_):
            s = bytearray()
            while mm.load(name + len(s), 1) and len(s) < 64:
                s.append(mm.load(name + len(s), 1))
            self.last_name = bytes(s)
            return CHUNK

        def set_anm(mm, anm, *_):
            who, slot = self.owner(anm)
            g.calls.append(("anim_set", g.name(who), slot, self.last_name.decode("latin-1")))
            mm.store(anm + 0x98, 4, 0)
            return 0

        def forward(mm, anm, step, *_):
            who, slot = self.owner(anm)
            g.calls.append(("anim_forward", g.name(who), slot, step & 0xFFFF))
            ret, nxt, notes = self.answer("anim")
            mm.store(anm + 0x98, 4, nxt)
            if slot == 0:
                base = who + NOTES_OFS
                head = 0
                for k in reversed(range(len(notes))):
                    a = base + 0x18 * k
                    mm.mem[a:a + 0x18] = bytes(0x18)
                    mm.store(a, 4, head)
                    mm.store(a + 4, 4, notes[k][0])
                    mm.store(a + 8, 4, notes[k][1])
                    head = a
                mm.store(anm + 0xA0, 4, head)
            return ret

        def shake(mm, pos, *_):
            g.calls.append(("shake",) + tuple(vec(mm, pos)))
            return self.answer("shake")

        def trans(mm, pos, *_):
            g.calls.append(("transparency",) + tuple(vec(mm, pos)) + tuple(mm.f[12 + k] & 0xFFFFFFFF
                                                                            for k in range(4)))
            v = self.answer("trans")
            mm.f[0] = v
            return v

        def anm_draw(mm, anm, *_):
            who, slot = self.owner(anm)
            g.calls.append(("drawSecond", g.name(who)) + tuple(mat(mm, anm + 0x40)) + (mm.load(anm + 0x88, 4),))
            return 0

        def char_draw(mm, ch, *_):
            anm = mm.load(ch + 0xD4, 4)
            g.calls.append(("draw", g.name(ch)) + tuple(mat(mm, anm + 0x40)))
            # the nodes' _SetLWMatrix: the anm's world matrix is its own
            mm.mem[anm:anm + 0x40] = bytes(mm.mem[anm + 0x40:anm + 0x80])
            return 1

        def breath_slot(br):
            va = br - BR_OFS
            return (va, 0) if self.m.load(va + 0x348, 4) == br else (va - 0x10, 1)

        def dust(mm, p, kind, a2, a3):
            g.calls.append(("dust",) + tuple(vec(mm, p)) + (kind, mm.f[12] & 0xFFFFFFFF, a2, s32(s16(a3))))
            return 0

        return {
            "ccTransPosW2P__FPfPf": w2p,
            "ccTransPosP2W__FPfPf": p2w,
            "ccLandHitCheck__FPfUi": land,
            "CollisionDetection__9ccCharHitFv": collide,
            "ccHitCheckLM2__FPfPfUi": line,
            "GetChunkAdrsF__8ccStreamFPCci": chunk,
            "SetAnm__5ccAnmFP10ccAnmChunkUi": set_anm,
            "_AnimateForward__5ccAnmFUi": forward,
            "checkCameraShakeRange__FPf": shake,
            "cameraShake__Fiiii": lambda mm, a, b, c, d: g.calls.append(("cameraShake", s32(a), b, c, d)) or 0,
            "ccSeSetParamEnemy__FUiP6ccChari":
                lambda mm, p, ch, cat, *_: g.calls.append(("sound", p, g.name(ch), s32(cat))) or 0,
            "note__17ccEnemyWeaponCtrlFP10ccEntryObjP9ccAnmNote":
                lambda mm, this, obj, note, *_: g.calls.append(
                    ("weaponNote", g.name(obj), mm.load(note + 4, 4), mm.load(note + 8, 4))) or 0,
            "ctrl__17ccEnemyWeaponCtrlFP10ccEntryObj":
                lambda mm, this, obj, *_: g.calls.append(("weaponCtrl", g.name(obj))) or 0,
            "ctrl__15ccEnemyDustCtrlFP10ccEntryObji":
                lambda mm, this, obj, flag, *_: g.calls.append(("dustCtrl", g.name(obj), s32(flag))) or 0,
            "ccGetCameraTransparency__FPfffff": trans,
            "SetActiveLayer__9WORLD_MANFi": lambda mm, *_: 0,
            "Draw__5ccAnmFv": anm_draw,
            "Draw__6ccCharFv": char_draw,
            "ccSeOn3D__FiPf": lambda mm, i, p, *_: g.calls.append(("se3d", s32(i)) + tuple(vec(mm, p))) or 0,
            "ccTransPosFW2LW__FPfPf": lambda mm, o, i, *_: (mm.mem.__setitem__(slice(o, o + 16),
                                                                               bytes(mm.mem[i:i + 16])), 0)[1],
            "ccEnemyEffDust__FPfifii": dust,
            "GetObjAdrsF__7ccClumpFPCciP19ccSubstSearchResult":
                lambda mm, c, name, *_: (g.calls.append(("footDust", (name - FOOT_OBJ_TBL) // 30))
                                         if FOOT_OBJ_TBL <= name < FOOT_OBJ_TBL + 120 else None, 0)[1],
            "ctrlBreath__13ccEnemyBreathFv":
                lambda mm, br, *_: g.calls.append(("breathCtrl", g.name(breath_slot(br)[0]), breath_slot(br)[1])) or 0,
            "setBreath__13ccEnemyBreathFP14ccEnemyBrParamP10ccEntryObj":
                lambda mm, br, prm, obj, *_: g.calls.append(("breathSet", g.name(obj), breath_slot(br)[1], prm)) or 0,
        }

    # the constructors -----------------------------------------------------------------
    def template(self, eid):
        """The state ccEntryEnemyX leaves for row eid (constructed once)."""
        if eid in self.templates:
            return self.templates[eid]
        g, m = self.g, self.m
        kind = self.kinds[eid]
        row = ENEMY_TBL + ROW * eid
        m.mem[ENT:ENT + 0x200] = bytes(0x200)
        m.mem[ENT:ENT + 0x48] = m.mem[row + 0x68:row + 0xB0]
        ent = [fbits(100.0), fbits(-50.0), 0, F_ONE, 0, 0, fbits(0.5), F_ONE, 0, eid, 1, 2, 3, 4, 5, 6, 1, 0, 0, 0,
               0, 0]
        g.put(ENT + 0x100, "<8I14i", *ent)
        m.store(ENT + 0xC, 4, ENT + 0x100)
        m.store(ENT + 0x10, 4, STREAM)
        m.store(ENT + 0x14, 4, STREAM2)
        self.c.scene.new_at = CTOR
        m.mem[CTOR:CTOR + 0x4000] = bytes(0x4000)
        hooks = dict(self.init_hooks)
        hooks.update({
            "__ct__17ccEnemyWeaponCtrlFiP13ccEnemyWpInfoP10ccEntryObj": lambda mm, a0, *_: a0,
            "__ct__15ccEnemyDustCtrlFiP15ccEnemyDustInfoP10ccEntryObj": lambda mm, a0, *_: a0,
            "Duplicate__7ccClumpFUi": lambda mm, *_: 0,
            "ChangeClut__7ccClumpFP11ccClutChunkP15ccMaterialChunk": lambda mm, *_: 0,
            "checkGold__8ccEnemyGFv": lambda mm, *_: 0,
        })
        func = m.load(eai.RACE_TBL + 12 * self.race(eid)[0], 4)
        saved = {}
        for n, h in list(self.base_hooks.items()) + list(hooks.items()):
            a = g.sym(n)
            saved.setdefault(a, m.hooks.get(a))
            m.hooks[a] = h
        try:
            obj = m.call(func, (ENT,))
        finally:
            for a, h in saved.items():
                if h is None:
                    del m.hooks[a]
                else:
                    m.hooks[a] = h
        e = self.read_enemy(obj, eid)
        wo, do = CTRL_OFS.get(kind, (0x340, 0x344))
        t = {"e": e, "kind": kind, "hit": self.read_hit(obj), "anm_tbl": m.load(obj + 0x1B0, 4),
             "set_t": m.load(obj + 0x8C, 4), "wc": int(m.load(obj + wo, 4) != 0), "dc": int(m.load(obj + do, 4) != 0),
             "gold": m.load(obj + 0x340, 1) & 1 if kind == "G" else 0,
             "br": int(kind in ("H", "L") and m.load(obj + 0x348, 4) != 0)}
        self.templates[eid] = t
        return t

    # the motion members ---------------------------------------------------------------
    def read_hit(self, va):
        m = self.m
        h = va + 0x160
        return ([m.load(h, 4) & 1, m.load(h + 4, 4), m.load(h + 8, 4), m.load(h + 12, 4), m.load(h + 0x14, 4),
                 m.load(h + 0x18, 4)] + [m.load(h + 0x20 + 4 * k, 4) for k in range(4)]
                + [m.load(h + 0x30 + 4 * k, 4) for k in range(4)] + [m.load(h + 0x40, 4)])

    def put_mx(self, va, eid, e, mx):
        g, m = self.g, self.m
        kind = self.kinds.get(eid)
        if kind:
            m.store(va + 0x1CC, 4, VTABLES[kind])
        h = mx["hit"]
        g.put(va + 0x160, "<7I", h[0], h[1], h[2], h[3], 0, h[4], h[5])
        g.put(va + 0x180, "<9I", *h[6:15])
        m.store(va + 0x1B0, 4, mx["anm_tbl"])
        m.store(va + 0x8C, 4, mx["set_t"])
        m.store(va + 0x88, 4, mx["set_t"])
        anm, anm2 = va + ANM_OFS, va + ANM2_OFS
        m.mem[anm:anm + 0x110] = bytes(0x110)
        m.store(anm + 0x98, 4, mx["frame"][0])
        m.store(anm + 0xA4, 4, self.check_note_fn)
        m.store(anm + 0xAC, 4, 1)
        m.store(va + 0xD4, 4, anm)
        m.store(va + 0xCC, 4, STREAM)
        self.anm_owner[anm] = (va, 0)
        m.mem[anm2:anm2 + 0x110] = bytes(0x110)
        if e["eflags"] & 0x80:
            m.store(anm2 + 0x98, 4, mx["frame"][1])
            m.store(anm2 + 0xAC, 4, 1)
            m.store(va + 0x1BC, 4, anm2)
            m.store(va + 0x1B4, 4, STREAM2)
            self.anm_owner[anm2] = (va, 1)
        else:
            m.store(va + 0x1BC, 4, 0)
        if not kind:
            return
        wo, do = CTRL_OFS.get(kind, (0x340, 0x344))
        if kind == "G":
            m.store(va + 0x340, 2, mx["gold"])
        if kind == "C":
            m.store(va + 0x340, 4, 0)
        if kind in ("H", "L"):
            m.store(va + 0x348, 4, va + BR_OFS if mx["br"] else 0)
            m.store(va + 0x34C, 4, va + BR_OFS + 0x10 if kind == "H" else 0)
        m.store(va + wo, 4, va + WC_OFS if mx["wc"] else 0)
        m.store(va + do, 4, va + DC_OFS if mx["dc"] else 0)

    def anm_row(self, anm):
        """anmTbl as the port keeps it: the first enemyTbl row whose
        entry.anm it is, plus one (0 for none); rows sharing a table share
        its names."""
        if not hasattr(self, "anm_rows"):
            self.anm_rows = {}
            for r in range(303):
                self.anm_rows.setdefault(self.m.load(ENEMY_TBL + ROW * r + 0x80, 4), r + 1)
        return self.anm_rows.get(anm, 0) if anm else 0

    def ser_mx(self, mx):
        return " ".join(str(x) for x in mx["hit"] + [self.anm_row(mx["anm_tbl"]), mx["set_t"], mx["wc"], mx["dc"],
                                                     mx["gold"], mx["br"]]
                        + mx["frame"])

    def ser_script(self):
        s = self.script
        out = [self.ox, self.oy, len(s["land"])] + s["land"] + [len(s["collide"])]
        for r, off, attr in s["collide"]:
            out += [r] + off + [attr]
        out += [len(s["line"])] + s["line"] + [len(s["shake"])] + s["shake"] + [len(s["trans"])] + s["trans"]
        out += [len(s["anim"])]
        for ret, nxt, notes in s["anim"]:
            out += [ret, nxt, len(notes)]
            for ev, p in notes:
                out += [ev, p]
        return " ".join(str(x) for x in out)

    # random states --------------------------------------------------------------------
    def pick_row(self, rnd, kinds=None):
        kind = rnd.choice(kinds or PICK)
        if kind in AREA_ROWS and rnd.random() < 0.7:
            return rnd.choice(AREA_ROWS[kind])
        return rnd.choice(self.rows[kind])

    def rnd_foe_row(self, rnd, eid):
        b, tb, data = self.b, self.tb, self.data
        f = tb.rnd_foe(b, data, rnd, "enemy")
        while f.row["index"] != eid:
            row = dict(data.enemies[eid])
            f2 = b.Foe(row, "enemy")
            for k in ("elm", "beff", "maxPP", "pDefPP", "mDefPP", "Exdefense", "maxHP", "maxSP"):
                row[k] = copy.copy(f.row[k])
            for k in ("type", "real", "temp", "time", "PP", "PPcount", "PPrestore", "HP", "SP", "maxHP", "maxSP",
                      "cond", "speed_value", "ent_root"):
                setattr(f2, k, copy.copy(getattr(f, k)))
            f = f2
        return f

    def motion_state(self, rnd, e, eid, template_p=0.7):
        """A random ccEnemy state for row eid (enemy_ai's random fields, the
        constructor's where it set them), and its motion members."""
        t = self.template(eid)
        te = t["e"]
        if rnd.random() < template_p:
            e["move"][5], e["move"][6] = te["move"][5], te["move"][6]
            e["move"][3] = te["move"][3]
            e["shorts"][0] = te["shorts"][0]
            e["list"] = copy.deepcopy(te["list"])
            e["skill"][3] = te["skill"][3]
            e["eflags"] = e["eflags"] & ~0x80 | te["eflags"] & 0x80
        else:
            e["move"][5] = rnd.choice((0, rfloat(rnd, -20, 20)))
            e["move"][6] = rnd.choice((0, rfloat(rnd, -20, 20)))
            # ccEnemy1's escape reads an unset register past eneType 5,
            # ccEnemyA's past 4; their constructors never go there
            top = {"E1": 5, "A": 4}.get(self.kinds[eid], 6)
            types = (0, 1, 2, 4) if self.kinds[eid] == "L" else tuple(range(top + 1))
            e["shorts"][0] = rnd.choice(types + (te["shorts"][0],))
        e["dirc"] = [rnd.choice((0, rangle(rnd))), rnd.choice((0, rangle(rnd))), rangle(rnd), F_ONE]
        if rnd.random() < 0.3:
            e["move"][4] = rfloat(rnd, 0, 70)
        if rnd.random() < 0.5:
            e["anm"][1] = e["anm"][0]
        hit = list(t["hit"])
        if rnd.random() < 0.5:
            hit[4] = rfloat(rnd, 0, 400)
            hit[5] = rfloat(rnd, 0, 400)
        hit[6:10] = rvec(rnd, 1000)
        mx = {"hit": hit, "anm_tbl": t["anm_tbl"], "set_t": rnd.choice((t["set_t"], F_ONE, rfloat(rnd, 0, 1))),
              "wc": t["wc"] if rnd.random() < 0.8 else rnd.choice((0, 1)),
              "dc": t["dc"] if rnd.random() < 0.8 else rnd.choice((0, 1)), "gold": 0,
              "br": t["br"] if t["kind"] != "H" else 1,
              "frame": [rnd.randrange(0, 0x10000), rnd.randrange(0, 0x10000)]}
        return mx

    def rnd_scene_m(self, rnd, rows, npc=None):
        """Party members and the enemies of `rows` (the first acting), a few
        others; returns chars, foes, pcs, enes, me, mxs."""
        b, tb, data = self.b, self.tb, self.data
        chars, kinds = [], []
        npc = rnd.choice((0, 1, 2, 3, 3, 4)) if npc is None else npc
        span = rnd.choice((600, 1500, 3000))
        for _ in range(npc):
            pc = tb.rnd_pc(b, data, rnd)
            pc.type = rnd.choice((1, 2, 4, 6, 7, 2))
            pc.cond[0] = rnd.choice((0,) * 6 + (1, 2))
            chars.append(pc)
            kinds.append("pc")
        used = set(rows)
        foe_rows = list(rows)
        for _ in range(rnd.choice((0, 0, 1, 2))):
            f = tb.rnd_foe(b, data, rnd, "enemy")
            # ccEnemyL's type 3 (203-206) is not ported
            if f.row["index"] not in used and not 203 <= f.row["index"] <= 206:
                used.add(f.row["index"])
                foe_rows.append(f.row["index"])
        for eid in foe_rows:
            f = self.rnd_foe_row(rnd, eid)
            f.affect_func = 1
            f.cond[0] = rnd.choice((0,) * 8 + (1, 2, 3))
            if rnd.random() < 0.3:
                f.level = rnd.choice((1, 30, 31, 60))
            f.ai = 0
            f.party_flag = 0
            f.no_death = False
            chars.append(f)
            kinds.append("foe")
        for ch in chars:
            ch.pos = rvec(rnd, span)
            ch.pos_p = [eemu.f_sub(ch.pos[0], self.ox), eemu.f_sub(ch.pos[1], self.oy), ch.pos[2], ch.pos[3]] \
                if rnd.random() < 0.9 else rvec(rnd, span)
            ch.width = rnd.choice((0, fbits(rnd.uniform(0, 60))))
            ch.height = rnd.choice((0, fbits(rnd.uniform(0, 600)), fbits(rnd.uniform(0, 290))))
            ch.skill_id, ch.skill_status = rnd.randrange(0, 300), rnd.randrange(0, 10)
            ch.anm_flag = rnd.choice((0, 1))
            ch.HP = rnd.choice((ch.HP, ch.HP, 0, -5, 1))
            for c in (8, 9, 10, 11, 1, 14, 15):
                if rnd.random() < 0.08:
                    ch.cond[c] = rnd.choice((1, 30, 600))
            if ch.type & 7:
                ch.affect_func = rnd.choice((2, 2, 3, 0))
                ch.act_num = rnd.choice((0, 1, 5, 7, 9, 10, 14, rnd.randrange(0, 20)))
                ch.spc_flags = rnd.getrandbits(32) & ~(0x80 | 7 << 14)
                ch.fellow_flags = rnd.getrandbits(8)
                ch.arms_effect_sw = rnd.randrange(0, 3)
                ch.attack = rnd.randrange(0, 3)
                ch.cnt = rnd.randrange(0, 100)
                ch.cloak = rnd.choice((0, F_ONE))
                ch.no_death = rnd.random() < 0.1
                ch.affect_type = rnd.choice((0, 0, 0, 13, 1))
                ch.affect_mask = rnd.choice((0,) * 6 + (1 << rnd.randrange(0, 21),))
        order = list(range(len(chars)))
        rnd.shuffle(order)
        chars = [chars[i] for i in order]
        kinds = [kinds[i] for i in order]
        n = len(chars)
        # the acting enemy is the first of rows
        foe_idx = [i for i in range(n) if kinds[i] == "foe"]
        me = next(i for i in foe_idx if chars[i].row["index"] == rows[0])
        foes = [None] * n
        mxs = [None] * n
        for i in foe_idx:
            eid = chars[i].row["index"]
            foes[i] = self.rnd_enemy(rnd, chars[i], eid, n, me)
            chars[i].affect_type = foes[i]["affect"][0]
            if eid in self.kinds:
                mxs[i] = self.motion_state(rnd, foes[i], eid)
            else:
                mxs[i] = {"hit": [0, 0xFFFFFFFF, 0, 0, F_ONE, 0] + [0] * 9, "anm_tbl": 0, "set_t": 0, "wc": 0,
                          "dc": 0, "gold": 0, "br": 0, "frame": [0, 0]}
        pcs = [i for i in range(n) if kinds[i] == "pc" and rnd.random() > 0.05]
        enes = [i for i in range(n) if kinds[i] == "foe" and (rnd.random() > 0.05 or i == me)]
        return chars, foes, pcs, enes, me, mxs

    # one case ---------------------------------------------------------------------------
    def mcase(self, rnd, op, rows=None, args=(), tweak=None, main=None, patch_row=True):
        """One case: a scene with the enemies of `rows` (the first acting),
        the world, the generators, then `op` (or `main(...)` for the frames)."""
        g, m, b, tb = self.g, self.m, self.b, self.tb
        self.rnd = rnd
        self.script = {"land": [], "collide": [], "line": [], "shake": [], "trans": [], "anim": []}
        self.collide_p = rnd.choice((0.35, 0.35, 0.8, 0.97))
        self.ox = rnd.choice((0, 0, fbits(rnd.uniform(-2000, 2000))))
        self.oy = rnd.choice((0, 0, fbits(rnd.uniform(-2000, 2000))))
        rows = rows or [self.pick_row(rnd)]
        npc = None if main is None else rnd.choice((0, 1, 1, 2, 2, 3))
        chars, foes, pcs, enes, me, mxs = self.rnd_scene_m(rnd, rows, npc=npc)
        if tweak:
            tweak(chars, foes, pcs, enes, me, mxs)
        f = foes[me]
        eid = f["ids"][0]
        self.cur = eid
        row_patch = None
        spells = set()
        if patch_row and rnd.random() < 0.6:
            row_patch = self.rnd_patch(rnd, eid, chars[me].row["maxPP"])
            th = struct.unpack("<6f i 2f i", bytes(m.mem[ENEMY_TBL + ROW * eid + 0xB0:ENEMY_TBL + ROW * eid + 0xD8]))
            anm_spd = rnd.choice((th[8], 1.0, rnd.uniform(0, 3)))
            g.put(ENEMY_TBL + ROW * eid + 0xB0 + 0x20, "<f", anm_spd)
            parts = row_patch.split()
            parts[1 + 8] = str(fbits(anm_spd))
            row_patch = " ".join(parts)
            for k in range(2):
                spells.add(m.load(ENEMY_TBL + ROW * eid + 0xD8 + 0x70 + 4 * k, 4, True))
        for s in f["list"]:
            if s[4] != -1 and s[4] < 1000:
                spells.add(s[4])
        # the row's attacks' targetRange (an area attack hits a side)
        tr_line = None
        if rnd.random() < 0.3:
            row = ENEMY_TBL + ROW * eid
            trs = [rnd.choice((0, 0, fbits(rnd.uniform(50, 800)), MINUS_ONE)) for _ in range(4)]
            for o, v in zip(eai.SLOT_OFS.values(), trs):
                m.store(row + o + 0x20, 4, v)
            tr_line = f"1 {eid} {' '.join(map(str, trs))}"
        patched = [sid for sid in sorted(x for x in spells if 0 < x < 304) if rnd.random() < 0.5]
        spell_lines = [self.patch_spell(rnd, sid) for sid in patched]
        self.anm_owner = {}
        self.put_scene(chars, foes, pcs, enes)
        for i, c in enumerate(chars):
            base = m.load(self.ptr(i), 4)
            if not c.type & 7:
                m.store(base + 0x18, 4, c.height)
            if foes[i] is not None:
                self.put_mx(self.ptr(i), foes[i]["ids"][0], foes[i], mxs[i])
        # the affects: each character's affectFunc and the fields its
        # influence reads (as tools/test_battle_rs.py's affect check)
        for i, c in enumerate(chars):
            a = self.ptr(i)
            if c.type & 7:
                m.store(a + 0x94, 4, self.affect_funcs.get(c.affect_func, 0))
                g.put(a + 0x9C, "<h", c.affect_type)
                m.store(a + 0xB0, 4, c.affect_mask)
                m.store(a + 0x78, 4, 0)
                m.store(a + 0xE0, 4, m.load(a + 0xE0, 4) | c.spc_flags)
                g.put(a + 0xEE, "<h", c.act_num)
                m.store(a + 0x200, 1, c.fellow_flags)
                m.store(a + 0xE4, 4, c.arms_effect_sw)
                g.put(a + 0xFE, "<h", c.attack)
                m.store(a + 0x114, 4, c.cnt)
                m.store(a + 0x110, 4, c.cloak)
            else:
                m.store(a + 0x94, 4, self.affect_funcs[1] if foes[i] is not None else 0)
                m.store(a + 0xB0, 4, 0)
                m.store(a + 0x78, 4, 0)
        self.running = rnd.choice((0, 0, 1, 2))
        # ccMenu stays: CalcReal's poison asks it which menu is open
        menu = 1
        pc_ids = [c.id for c in chars if c.type & 7]
        ids = [rnd.choice((-1, rnd.randrange(0, 18)) + tuple(pc_ids)) for _ in range(3)]
        g.party = {x: k for k, x in enumerate(ids) if x >= 0 and x not in ids[:k]}
        m.store(g.sym("ccMenu"), 4, 0x01020000 if menu else 0)
        # the party's base is its ccSpcParam: height there is not the model's
        for i, c in enumerate(chars):
            if c.type & 7:
                c.height = m.load(m.load(self.ptr(i), 4) + 0x18, 4)
        puppet = int(rnd.random() < 0.2)
        ride = int(rnd.random() < 0.2)
        self.active = rnd.randrange(0, 6)
        player = rnd.choice((pcs[0] if pcs else -1, -1, rnd.randrange(len(chars))))
        if pcs and rnd.random() < 0.7:
            player = pcs[0]
        m.store(eai.EVT + 0x78C, 4, puppet)
        m.store(g.sym("pgRideFlag"), 4, ride)
        m.store(g.sym("ccPartyManager"), 4, self.ptr(player))
        state = rnd.getrandbits(64)
        g.set_rand(state)
        seed, mti = rnd.getrandbits(32), rnd.choice((rnd.randrange(0, 624), 623, 624, 625, 0))
        self.put_cc(seed, mti)
        env = tb.rnd_env(b, rnd)
        g.put_env(env)
        count = rnd.choice((0, 5, 98, 99, 127, rnd.randrange(0, 256)))
        server, area = rnd.randrange(-5, 40), [rnd.randrange(-100, 100) for _ in range(3)]
        rows_all = [ff["ids"][0] for ff in foes if ff]
        for r in rows_all:
            m.store(eai.SAVE + eai.KILL_COUNT + r, 1, count if r == eid else 0)
            m.mem[eai.SAVE + eai.KILL_AREA + 8 * r:eai.SAVE + eai.KILL_AREA + 8 * r + 8] = bytes(8)
        m.store(self.tb.GAME + 0x1C, 4, server & 0xFFFFFFFF)
        g.put(eai.WM + 0x14C, "<3i", *area)
        m.store(g.sym("cmndTarget"), 4, 0)
        g.calls = []
        self.new_made = False
        scene_line = self.ser_scene(chars, foes, pcs, enes).replace("escene", "mscene " + (tr_line or "0"), 1)
        head = (scene_line + f" {puppet} {ride} {self.active} {player} {state} {seed} {mti} {ser_env(env)}"
                + f" {count} {server} {' '.join(map(str, area))}"
                + " " + " ".join(str(c.height) for c in chars)
                + "".join(" " + self.ser_mx(mxs[i]) for i in range(len(chars)) if foes[i] is not None))
        if main is not None:
            out, tail = main(chars, foes, me)
        else:
            ret, tail = op(me)
            out = {"ret": ret, "rand": g.rand_now(), "cc": self.cc_state(), "calls": list(g.calls),
                   "state": self.read_state(chars, foes), "kill": self.kill(eid),
                   "new": [m.load(eai.NEWOBJ + 8, 2, True), m.load(eai.NEWOBJ + 0x264, 2, True)]
                   if self.new_made else None}
        for c in chars:
            if hasattr(c, "row"):
                self.g.restore(c.row["va"], 0x68)
        m.store(g.sym("ccMenu"), 4, 0x01020000)
        g.party = {}
        req = (head + " " + self.ser_script() + f" {self.running} {menu} {' '.join(map(str, ids))} " + tail)
        if row_patch:
            self.restore_row(eid)
        for sid in patched:
            self.g.restore(eai.SKILL_TBL + eai.SKILL_ROW * sid, eai.SKILL_ROW)
        prep = (f"eprep {int(bool(row_patch))}" + (f" {row_patch}" if row_patch else "")
                + f" {len(spell_lines)}" + "".join(" " + x for x in spell_lines))
        unprep = f"eunprep {int(bool(row_patch))} {eid} {len(patched)}" + "".join(f" {x}" for x in patched)
        if tr_line:
            self.restore_row(eid)
        return [prep, req, unprep], out

    def kill(self, eid):
        m = self.m
        return ([m.load(eai.SAVE + eai.KILL_COUNT + eid, 1, True)]
                + [m.load(eai.SAVE + eai.KILL_AREA + 8 * eid + 2 * k, 2, True) for k in range(4)])

    def read_affect(self, i, c, foe):
        g, m = self.g, self.m
        a = self.ptr(i)
        d = self.read_char(a, a + (0x1D0 if foe else 0x400), c, bool(c.type & 7))
        person = m.load(a + 0x98, 4)
        d["affect"] = (list(g.shorts(a + 0x9C, 4)) + [self.index(person) if person else -1]
                       + list(g.shorts(a + 0xA8, 2)) + [m.load(a + 0xAC, 4)] + list(g.shorts(a + 0xB4, 2))
                       + [m.load(a + 0xB8, 4)])
        d["spc"] = None if foe else [g.shorts(a + 0xEE, 1)[0], m.load(a + 0xE0, 4) & ~(0x80 | 7 << 14),
                                     m.load(a + 0x200, 1), 0, s32(m.load(a + 0xE4, 4)), g.shorts(a + 0xFE, 1)[0],
                                     s32(m.load(a + 0x114, 4)), m.load(a + 0x110, 4)]
        tc = m.load(a + 0x78, 4)
        d["target"] = self.index(tc) if tc else -1
        return d

    def read_state(self, chars, foes):
        return {"chars": [self.read_affect(i, c, foes[i] is not None) for i, c in enumerate(chars)],
                "foes": [self.read_enemy(self.ptr(i), f_["ids"][0]) if f_ else None for i, f_ in enumerate(foes)],
                "hits": [self.read_hit(self.ptr(i)) if f_ else None for i, f_ in enumerate(foes)],
                "lists": self.lists()}

    def run(self, fn, args=(), fargs=None):
        """A function on the acting enemy with the motion hooks; fargs sets
        f12.. first."""
        m, sym = self.m, self.g.sym
        hooks = dict(self.base_hooks)
        del hooks["ccSkillDamage__FP6ccCharP6ccCharP12ccSkillParami"]
        hooks.update(self.hooks)
        hooks.update(self.affect_stubs)
        saved = {}
        by_va = {sym(n): h for n, h in hooks.items()}
        if volume.NAME != "infection":      # the AI's record of the hit (unnamed; Mutation 0x5c1a60)
            rec = self.g.record
            by_va[volume.callee("Influence__FP6ccChar", 0)] = lambda mm, a, b_, *_: rec("NoteHit", a, b_)
        for a, h in by_va.items():
            saved[a] = m.hooks.get(a)
            m.hooks[a] = h
        saved[self.entry_affect_fn] = m.hooks.pop(self.entry_affect_fn, None)
        try:
            for k, v in enumerate(fargs or ()):
                m.f[12 + k] = v
            return m.call(sym(fn) if isinstance(fn, str) else fn, tuple(x & 0xFFFFFFFF for x in args))
        finally:
            for a, h in saved.items():
                if h is None:
                    del m.hooks[a]
                else:
                    m.hooks[a] = h


def harness(checks):
    h = getattr(checks, "_motion", None)
    if h is None:
        h = checks._motion = Motion(checks)
    return h


# the checks ---------------------------------------------------------------------------------

HELPERS = ("setrad", "setdist", "dircchgf", "setdirc", "getdirc", "getdist", "disperse", "pilimit", "pilimitp",
           "rotz", "rotx", "roty", "rot", "apply", "trans", "unit")


def rangle_wide(rnd):
    return rnd.choice((rangle(rnd), fbits(rnd.uniform(-7, 7)), 0, fbits(math.pi), fbits(-math.pi),
                       fbits(rnd.uniform(-20, 20))))


def rmat(rnd):
    return [rnd.choice((rfloat(rnd, -2, 2), 0, F_ONE, rfloat(rnd, -1000, 1000))) for _ in range(16)]


def check_helpers(checks, rnd):
    h = harness(checks)
    g, m = h.g, h.m
    fn = rnd.choice(HELPERS)
    A, B = 0x01038000, 0x01038100
    rate = rnd.choice((rfloat(rnd, -1.5, 1.5), rfloat(rnd, 0, 1), F_ONE, 0, MINUS_ONE, fbits(0.3), fbits(-0.3)))
    if fn in ("setrad", "setdist"):
        r = rangle_wide(rnd) if fn == "setrad" else rfloat(rnd, -100, 100)
        to = rangle_wide(rnd) if fn == "setrad" else rnd.choice((r, rfloat(rnd, -100, 100), 0))
        m.store(A, 4, r)
        h.run("ccSetRad__FPfff" if fn == "setrad" else "ccSetDist__FPfff", (A,), (to, rate))
        return f"mhelp {fn} {r} {to} {rate}", m.load(A, 4), fn
    if fn in ("dircchgf", "setdirc"):
        a, bb = rangle_wide(rnd), rangle_wide(rnd)
        mode = rnd.choice((128, 32, 64, 8, 2, 512, 1, 0x10000, 0x10020, rnd.randrange(1, 1000)))
        if fn == "dircchgf":
            h.run("ccGetDircChgF__Fffi", (mode,), (a, bb))
            return f"mhelp {fn} {a} {bb} {mode}", m.f[0] & 0xFFFFFFFF, fn
        m.store(A, 4, a)
        h.run("ccSetDirc__FPffi", (A, mode), (bb,))
        return f"mhelp {fn} {a} {bb} {mode}", m.load(A, 4), fn
    if fn in ("getdirc", "getdist"):
        va, vb = rvec(rnd, 3000), rnd.choice((rvec(rnd, 3000), None))
        vb = vb or list(va)
        g.put(A, "<4I", *va)
        g.put(B, "<4I", *vb)
        h.run("ccGetDirc__FPfPf" if fn == "getdirc" else "ccGetDist__FPfPf", (A, B))
        return f"mhelp {fn} {' '.join(map(str, va + vb))}", m.f[0] & 0xFFFFFFFF, fn
    if fn == "disperse":
        v, x = rangle_wide(rnd), rnd.choice((fbits(math.pi / 2), rfloat(rnd, 0, 4), fbits(0.3141593)))
        seed, mti = rnd.getrandbits(32), rnd.choice((rnd.randrange(0, 624), 624, 625))
        h.put_cc(seed, mti)
        m.store(A, 4, v)
        h.run("ccSetRadDisperse__FPff", (A,), (x,))
        return f"mhelp {fn} {v} {x} {seed} {mti}", [m.load(A, 4), h.cc_state()], fn
    if fn in ("pilimit", "pilimitp"):
        v = rnd.choice((rangle_wide(rnd), rfloat(rnd, -10, 10), fbits(math.pi), 0x40490FDC, 0xC0490FDC))
        if fn == "pilimit":
            h.run("ccPiLimit__Ff", (), (v,))
            return f"mhelp {fn} {v}", m.f[0] & 0xFFFFFFFF, fn
        m.store(A, 4, v)
        h.run("ccPiLimit__FPf", (A,))
        return f"mhelp {fn} {v}", m.load(A, 4), fn
    M = rmat(rnd) if rnd.random() < 0.7 else [F_ONE, 0, 0, 0, 0, F_ONE, 0, 0, 0, 0, F_ONE, 0, 0, 0, 0, F_ONE]
    g.put(A, "<16I", *M)
    if fn in ("rotz", "rotx", "roty"):
        r = rangle_wide(rnd)
        h.run({"rotz": "sceVu0RotMatrixZ", "rotx": "sceVu0RotMatrixX", "roty": "sceVu0RotMatrixY"}[fn], (B, A), (r,))
        return f"mhelp {fn} {' '.join(map(str, M))} {r}", list(g.get(B, "<16I")), fn
    if fn == "rot":
        v = [rangle_wide(rnd) for _ in range(3)] + [F_ONE]
        g.put(A + 0x40, "<4I", *v)
        h.run("sceVu0RotMatrix", (B, A, A + 0x40))
        return f"mhelp {fn} {' '.join(map(str, M + v))}", list(g.get(B, "<16I")), fn
    if fn in ("apply", "trans"):
        v = rvec(rnd, 1000) if rnd.random() < 0.8 else [rfloat(rnd, -5, 5) for _ in range(4)]
        g.put(A + 0x40, "<4I", *v)
        if fn == "apply":
            h.run("sceVu0ApplyMatrix", (B, A, A + 0x40))
            return f"mhelp {fn} {' '.join(map(str, M + v))}", list(g.get(B, "<4I")), fn
        h.run("sceVu0TransMatrix", (B, A, A + 0x40))
        return f"mhelp {fn} {' '.join(map(str, M + v))}", list(g.get(B, "<16I")), fn
    h.run("sceVu0UnitMatrix", (B,))
    return "mhelp unit", list(g.get(B, "<16I")), fn


def moving_tweak(rnd):
    def tweak(chars, foes, pcs, enes, me, mxs):
        f = foes[me]
        if rnd.random() < 0.5:
            f["move"][4] = rnd.choice((rfloat(rnd, 0, 70), rfloat(rnd, 25, 70), fbits(31.0)))
        if rnd.random() < 0.3:
            f["move"][2] = rnd.randrange(0, 40)
    return tweak


def check_act(checks, rnd):
    h = harness(checks)
    which = rnd.randrange(3)
    vals = [rangle_wide(rnd), rnd.choice((0, rfloat(rnd, -1.2, 1.2), fbits(0.3), fbits(-0.3))),
            rangle_wide(rnd), rnd.choice((0, rfloat(rnd, -1.2, 1.2))),
            rnd.choice((0, rfloat(rnd, 0, 60), rfloat(rnd, -5, 5))), rnd.choice((0, rfloat(rnd, 0, 1), F_ONE)),
            rnd.choice((0, 0, rfloat(rnd, 0, 3), F_ONE))]
    fn = ("actMove__7ccEnemyFfffffff", "actFollow__7ccEnemyFfffff", "actSlide__7ccEnemyFfffff")[which]
    fa = vals if which == 0 else [vals[0], vals[1], vals[4], vals[5], vals[6]]

    def op(me):
        h.run(fn, (h.ptr(me),), fa)
        return 0, f"act {me} {which} " + " ".join(map(str, fa + [0] * (7 - len(fa))))
    req, out = h.mcase(rnd, op, tweak=moving_tweak(rnd))
    return req, out, "act"


def check_escape(checks, rnd):
    h = harness(checks)
    kind = rnd.choice(("escape", "escape", "escapeby", "escapex"))

    def tweak(chars, foes, pcs, enes, me, mxs):
        f = foes[me]
        f["shorts"][5] = rnd.choice((0, 0, 1, 30, 60, 61, 62, 200, f["shorts"][5]))
        f["f5"][2] = rnd.choice((0, fbits(0.1), fbits(0.39), fbits(0.4), fbits(0.79), fbits(0.8), F_ONE,
                                 rfloat(rnd, 0, 1)))
        if rnd.random() < 0.3:
            f["ids"][2] = rnd.choice((0, 1))

    def op(me):
        if kind == "escapeby":
            ty = rnd.choice((0, 1, 2, 3, -1))
            red = rnd.choice((F_ONE, fbits(0.8), fbits(0.4), fbits(0.7), rfloat(rnd, 0, 2)))
            h.run("actEscape__7ccEnemyFif", (h.ptr(me), ty), (red,))
            return 0, f"escapeby {me} {ty} {red}"
        h.run("actEscape__7ccEnemyFv" if kind == "escape" else "actEscapeX__7ccEnemyFv", (h.ptr(me),))
        return 0, f"{kind} {me}"
    req, out = h.mcase(rnd, op, tweak=tweak)
    return req, out, kind


def check_move(checks, rnd):
    h = harness(checks)

    def op(me):
        h.run("moveEnemy__7ccEnemyFv", (h.ptr(me),))
        return 0, f"move {me}"
    req, out = h.mcase(rnd, op, tweak=moving_tweak(rnd))
    return req, out, "move"


def check_anim(checks, rnd):
    h = harness(checks)

    def tweak(chars, foes, pcs, enes, me, mxs):
        f = foes[me]
        if rnd.random() < 0.5:
            f["anm"][0] = rnd.randrange(0, 11)
        f["anm"][1] = rnd.choice((f["anm"][0], f["anm"][0], -1, rnd.randrange(0, 11)))
        f["flags"] = f["flags"] & ~0x18 | rnd.choice((0, 0, 1, 1, 2, 2, 3)) << 3
        if rnd.random() < 0.3:
            f["shorts"][4] = rnd.choice((6, 0, 1))

    def op(me):
        h.run("animEnemy__7ccEnemyFv", (h.ptr(me),))
        return 0, f"anim {me}"
    req, out = h.mcase(rnd, op, tweak=tweak)
    return req, out, "anim"


def hit_tweak(rnd):
    """Often an attack that lands: its target a living party member on the
    lists, close."""
    def tweak(chars, foes, pcs, enes, me, mxs):
        if rnd.random() < 0.5 or not pcs:
            return
        f = foes[me]
        tg = rnd.choice(pcs)
        chars[tg].cond[0] = 0
        f["skill"][2] = tg
        f["skill"][1] = rnd.choice((1000, 1001, 1004, 1005, f["skill"][1]))
        f["f5"][1] = rnd.choice((0, fbits(10.0), f["f5"][1]))
        if rnd.random() < 0.5:
            for c in pcs:
                chars[c].pos_p = list(chars[tg].pos_p)
    return tweak


def check_note(checks, rnd):
    h = harness(checks)
    notes = [(rnd.choice(NOTE_EVENTS + (0x8005, 0x8005)), rnd.choice((0, 1, 2, 3, rnd.randrange(0, 300))))
             for _ in range(rnd.randrange(1, 5))]

    def op(me):
        m = h.m
        va = h.ptr(me)
        m.store(h.note_char, 4, va)
        for ev, p in notes:
            a = va + NOTES_OFS
            m.mem[a:a + 0x18] = bytes(0x18)
            m.store(a + 4, 4, ev)
            m.store(a + 8, 4, p)
            h.run(h.check_note_fn, (a,))
        return 0, f"note {me} {len(notes)} " + " ".join(f"{e} {p}" for e, p in notes)
    req, out = h.mcase(rnd, op, tweak=hit_tweak(rnd))
    return req, out, "note"


def check_disp(checks, rnd):
    h = harness(checks)

    def tweak(chars, foes, pcs, enes, me, mxs):
        if rnd.random() < 0.4:
            foes[me]["eflags"] |= 0x80
        foes[me]["move"][5] = rnd.choice((0, rfloat(rnd, -50, 50)))

    def op(me):
        h.run("dispEnemy__7ccEnemyFv", (h.ptr(me),))
        return 0, f"disp {me}"
    req, out = h.mcase(rnd, op, tweak=tweak)
    return req, out, "disp"


def race_fn(h, eid, slot):
    return h.m.load(VTABLES[h.kinds[eid]] + slot, 4)


def check_action(checks, rnd):
    h = harness(checks)

    def tweak(chars, foes, pcs, enes, me, mxs):
        f = foes[me]
        f["shorts"][4] = rnd.choice((0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 1, 2, 6, 7, rnd.choice((-1, 10))))
        f["shorts"][5] = rnd.choice((0, 0, 0, 1, 16, 17, 32, 60, 61, rnd.randrange(0, 300)))
        f["shorts"][7] = rnd.choice((0, 5, 15, 16, 31, 32, 40))
        if rnd.random() < 0.3:
            f["anm"][0] = 7
        if rnd.random() < 0.3:
            f["f5"][1] = rnd.choice((fbits(150.0), fbits(250.0), 0))
        if rnd.random() < 0.3:
            f["ids"][0] = f["ids"][0]
        if h.kinds.get(f["ids"][0]) == "T" and rnd.random() < 0.6:
            f["anm"][1] = rnd.choice((6, 7, 11, 12))
            f["anm"][3] = rnd.choice((0, 1))

    rows = None
    if rnd.random() < 0.15:
        rows = [rnd.choice((218, 184, 267, rnd.choice((158, 159, 161, 165))))]

    def op(me):
        eid = h.cur
        h.run(race_fn(h, eid, 0x14), (h.ptr(me),))
        return 0, f"action {me}"
    if rows and rows[0] not in h.kinds:
        rows = None
    req, out = h.mcase(rnd, op, rows=rows, tweak=tweak)
    return req, out, "action_" + h.kinds[h.cur]


def check_moveex(checks, rnd):
    h = harness(checks)
    which = rnd.choice(("moveeg", "moveegg", "moveeb"))
    kinds = ("B",) if which == "moveeb" else ("G",)

    def tweak(chars, foes, pcs, enes, me, mxs):
        f = foes[me]
        f["shorts"][4] = rnd.randrange(-1, 11)
        f["shorts"][5] = rnd.choice((0, 0, 1, 16, 17, 32, rnd.randrange(0, 300)))
        if which == "moveegg":
            f["shorts"][0] = rnd.choice((5, 6))
            f["flags"] = f["flags"] & ~0x18 | rnd.choice((0, 1, 1, 2)) << 3

    fn = {"moveeg": "moveEG__8ccEnemyGFv", "moveegg": "moveEGG__8ccEnemyGFv", "moveeb": "moveEB__8ccEnemyBFv"}[which]

    def op(me):
        h.run(fn, (h.ptr(me),))
        return 0, f"{which} {me}"
    req, out = h.mcase(rnd, op, rows=[h.pick_row(rnd, kinds)], tweak=tweak)
    return req, out, which


def check_excl(checks, rnd):
    h = harness(checks)
    which = rnd.choice(("excl", "excl", "rnote", "rnote", "freeze"))
    kinds = ("G",) if which == "freeze" else None
    note = [rnd.choice(NOTE_EVENTS), rnd.randrange(0, 5)]

    def tweak(chars, foes, pcs, enes, me, mxs):
        f = foes[me]
        if h.kinds.get(f["ids"][0]) == "G" and rnd.random() < 0.5:
            mxs[me]["gold"] = 1
            f["anm"][0] = rnd.choice((7, 7, 6))
            f["move"][4] = rnd.choice((rfloat(rnd, 0, 60), 0))
            f["shorts"][5] = rnd.choice((0, 16, 32, rnd.randrange(0, 100)))
        if which == "freeze":
            f["eflags"] = f["eflags"] & ~0x44 | rnd.choice((0, 4, 4, 0x44))
        if f["ids"][0] in (113, 114) and rnd.random() < 0.7:
            f["anm"][0] = 7
            f["anm"][2] = rnd.choice((10, 45, 46, 60))
            if which == "rnote":
                note[0] = rnd.choice((1, 2))
        if f["ids"][0] in RACE_EXCL and rnd.random() < 0.7:
            # the races' own exclusive(): an H breathing (attack 4), a C
            # spinning (act 5), an F diving (attacks 2 and 3)
            f["shorts"][4] = rnd.choice((5, 5, 6, 6, 0))
            f["shorts"][5] = rnd.choice((0, 1, 2, 3, 49, 50, 51))
            f["shorts"][6] = rnd.choice((2, 3, 4, 4, 0))
            f["flags"] |= rnd.choice((1, 1, 0))
            f["eflags"] |= rnd.choice((0x10, 0x10, 0))

    def op(me):
        eid, va = h.cur, h.ptr(me)
        m = h.m
        if which == "excl":
            h.run(race_fn(h, eid, 0x18), (va,))
            return 0, f"excl {me}"
        if which == "rnote":
            a = va + NOTES_OFS
            m.mem[a:a + 0x18] = bytes(0x18)
            m.store(a + 4, 4, note[0])
            m.store(a + 8, 4, note[1])
            h.run(race_fn(h, eid, 0x1C), (va, a))
            return 0, f"rnote {me} {note[0]} {note[1]}"
        # ccEntryCmnd appends after the list's last: keep the Last pointers
        g = h.g
        for root, last in ((g.cmnd_pc, "cmndPcLast"), (g.cmnd_ene, "cmndEneLast"), (g.cmnd_obj, "cmndObjLast")):
            a, prev = m.load(root, 4), 0
            while a:
                prev, a = a, m.load(a + 0xBC, 4)
            m.store(g.sym(last), 4, prev)
        ret = h.run(race_fn(h, eid, 0x0C), (va,)) & 0xFF
        return ret, f"freeze {me}"
    row = rnd.choice(RACE_EXCL) if kinds is None and rnd.random() < 0.4 else h.pick_row(rnd, kinds)
    req, out = h.mcase(rnd, op, rows=[row], tweak=tweak)
    return req, out, which + "_" + h.kinds[h.cur]


def check_main(checks, rnd, frames=None):
    h = harness(checks)
    nene = rnd.choice((1, 1, 2, 3))
    rows = []
    while len(rows) < nene:
        r = h.pick_row(rnd, PICK)
        if r not in rows:
            rows.append(r)
    nframes = frames or rnd.choice((60, 60, 90, 120, 200, 300))

    def tweak(chars, foes, pcs, enes, me, mxs):
        if rnd.random() < 0.3:
            # the party out of sight: the enemies wander
            for c in chars:
                if c.type & 7:
                    c.pos = [fbits(fl(c.pos[0]) + 30000.0), c.pos[1], c.pos[2], c.pos[3]]
        for i, f in enumerate(foes):
            if f is None or i not in enes:
                continue
            f["eflags"] &= ~0x0C          # not frozen, no affect pending
            f["flags"] &= ~0x04           # no drain pending
            f["shorts"][9] = rnd.choice((0,) * 8 + (1, 2, 30))   # a drained form's grace, sometimes
            if rnd.random() < 0.6:
                f["shorts"][4] = 0        # waiting
                f["shorts"][5] = 0
            if rnd.random() < 0.2:
                f["bpos"] = rvec(rnd, 20000)   # far from home: it goes back
            chars[i].cond[0] = 0
            chars[i].HP = max(chars[i].HP, 1)

    def main(chars, foes, me):
        g, m = h.g, h.m
        live = [i for i, f in enumerate(foes) if f is not None and f["ids"][0] in h.kinds]
        pcs = [i for i, c in enumerate(chars) if c.type & 7]
        paths = {}
        for i in pcs:
            p0 = chars[i].pos
            ang = rnd.uniform(-math.pi, math.pi)
            spd = rnd.choice((0.0, rnd.uniform(0, 12), rnd.uniform(0, 40)))
            paths[i] = (fl(p0[0]), fl(p0[1]), fl(p0[2]), math.cos(ang) * spd, math.sin(ang) * spd,
                        rnd.choice((0, rnd.uniform(0, 0.05))))
        tail = [f"main {me} {nframes}"]
        frames_out = []
        for t in range(nframes):
            moves = []
            for i in pcs:
                x0, y0, z0, vx, vy, turn = paths[i]
                a = turn * t
                x = x0 + (vx * math.cos(a) - vy * math.sin(a)) * t
                y = y0 + (vx * math.sin(a) + vy * math.cos(a)) * t
                pos = [fbits(x), fbits(y), fbits(z0), F_ONE]
                pos_p = [eemu.f_sub(pos[0], h.ox), eemu.f_sub(pos[1], h.oy), pos[2], pos[3]]
                dead = m.load(h.ptr(i) + 8, 2, True)
                if rnd.random() < 0.004:
                    dead = 2 if dead == 0 else 0
                g.put(h.ptr(i) + 0x40, "<4I", *pos)
                g.put(h.ptr(i) + 0x50, "<4I", *pos_p)
                g.put(h.ptr(i) + 8, "<h", dead)
                moves.append(f"{i} {' '.join(map(str, pos + pos_p))} {dead}")
            events = []
            for i in live:
                if rnd.random() > 0.03:
                    continue
                va = h.ptr(i)
                kind = rnd.choice((0, 1, 1, 1, 2, 2, 3, 4))
                if kind == 0:
                    a, b_ = rnd.choice((0, 1, 5, -3, rnd.randrange(1, 200))), 0
                    g.put(va + 0x70, "<h", a)
                elif kind == 1:
                    a = rnd.choice((1, 1, 1, 3, 20, 4, 7))
                    b_ = rnd.choice((0, 1, 5, -1, rnd.randrange(1, 300)))
                    m.store(va + 0xE0, 1, m.load(va + 0xE0, 1) | 0x08)
                    g.put(va + 0x9C, "<hh", a, b_)
                elif kind == 2:
                    a = rnd.choice((1, 8, 9, 10, 11, 14, 15))
                    b_ = rnd.choice((0, 0, 1, 30, 120))
                    g.put(va + 8 + 2 * a, "<h", b_)
                elif kind == 3:
                    a = b_ = 0
                    m.store(va + 0x250, 1, m.load(va + 0x250, 1) | 0x04)
                else:
                    a, b_ = rnd.choice((0, 1)), 0
                    m.store(va + 0xE0, 1, m.load(va + 0xE0, 1) & ~0x04 | (4 if a else 0))
                events.append(f"{i} {kind} {a} {b_}")
            tail.append(f"{len(moves)}" + "".join(" " + x for x in moves))
            tail.append(f"{len(events)}" + "".join(" " + x for x in events))
            g.calls = []
            rets = []
            for i in list(live):
                h.cur = foes[i]["ids"][0]
                r = h.run("main__7ccEnemyFv", (h.ptr(i),)) & 0xFF
                rets.append([i, r])
                if r == 1:
                    live.remove(i)
            frames_out.append({"rets": rets, "calls": list(g.calls), "rand": g.rand_now(), "cc": h.cc_state(),
                               "state": h.read_state(chars, foes)})
        return {"frames": frames_out, "kill": h.kill(foes[me]["ids"][0])}, " ".join(tail)
    req, out = h.mcase(rnd, None, rows=rows, tweak=tweak, main=main, patch_row=rnd.random() < 0.5)
    return req, out, "main"


MOTION = {
    "helpers": check_helpers,
    "act": check_act,
    "escape": check_escape,
    "move": check_move,
    "anim": check_anim,
    "note": check_note,
    "disp": check_disp,
    "action": check_action,
    "moveex": check_moveex,
    "excl": check_excl,
    "main": check_main,
}


@unittest.skipUnless(rs.READY, "needs the extracted disc and cargo")
class EnemyMotionAgainstGame(rs.Against):
    TABLE = MOTION

    def test_helpers(self):
        self.check("helpers", 21000)

    def test_acts(self):
        self.check("act", 21001)
        self.check("escape", 21002)

    def test_move_anim(self):
        self.check("move", 21003)
        self.check("anim", 21004)
        self.check("note", 21005)
        self.check("disp", 21006)

    def test_races(self):
        self.check("action", 21007)
        self.check("moveex", 21008)
        self.check("excl", 21009)

    def test_main(self):
        self.check("main", 21010)


if __name__ == "__main__":
    rs.main(list(MOTION), 21100, MOTION)
